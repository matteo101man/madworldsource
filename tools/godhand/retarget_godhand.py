"""Retarget God Hand clips (exported by the God Hand explorer into assets/native/godhand/, see its index.json) onto the
MadWorld body rig shared by pl0030 (Jack, and Kreely through his rotation-only retarget), em0440 (RinRin) and em0101
(the kung-fu grunts): bones 0 hips .. 23 left toe, T-pose bind with identity rotations, Y up, facing +Z, the
character's right on -X -- the same conventions as God Hand's Gene rig (index.json `axes`), so no axis change.

Method: forward kinematics on the source; for each mapped target bone the world rotation is the source bone's world
rotation times the rest-pose alignment that turns the target's rest bone direction onto the source's (limbs only;
torso and head keep identity); local = parent world^-1 x world; then Euler X, Y, Z for the MadWorld channels 3..5
(composed Z*Y*X, mw_sim::character::quat_zyx), unwrapped frame to frame. Root motion: the source's actor travel goes
on the MOT root (-1, channels 0 / 2, from frame 0), its yaw into the hips; the hips' height on bone 0 channel 1, all
scaled by the target / source hip-height ratio.

Writes assets/native/kreely-moves.json: Kreely's clips (pl0030 bank names kreely_gh_*.mot) with their native-style
SEQ cues (op 1 rate, 0x13 / 0x14 hit window with the striking part and power, 0x0B combo window, 29 whoosh, 38 impact),
the receivers' clips for the RinRin and grunt banks (gh_*.mot), and the suplex placement.
Usage: python tools/godhand/retarget_godhand.py
"""
import json, math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / 'assets/native/godhand'
OUT = ROOT / 'assets/native/kreely-moves.json'

# source bone (God Hand: Gene / henchmen share 0..25) -> MadWorld bone id
MAP = {0: 0, 1: 1, 2: 2, 3: 3, 4: 4, 5: 5, 7: 7, 8: 8, 10: 9, 11: 10, 13: 12, 14: 13, 16: 14,
       17: 15, 18: 16, 19: 17, 20: 18, 21: 19, 22: 20, 23: 21, 24: 22, 25: 23}
# rest-direction child for the alignment (limbs)
CHILD = {5: 7, 7: 8, 8: 10, 11: 13, 13: 14, 14: 16, 18: 19, 19: 20, 20: 21, 22: 23, 23: 24, 24: 25}
MW_CHILD = {5: 7, 7: 8, 8: 9, 10: 12, 12: 13, 13: 14, 16: 17, 17: 18, 18: 19, 20: 21, 21: 22, 22: 23}
MIRROR = {5: 11, 6: 12, 7: 13, 8: 14, 9: 15, 10: 16, 18: 22, 19: 23, 20: 24, 21: 25}
MIRROR.update({v: k for k, v in list(MIRROR.items())})

# --- quaternions (x, y, z, w)
def qmul(a, b):
    ax, ay, az, aw = a; bx, by, bz, bw = b
    return (aw*bx + ax*bw + ay*bz - az*by, aw*by - ax*bz + ay*bw + az*bx, aw*bz + ax*by - ay*bx + az*bw, aw*bw - ax*bx - ay*by - az*bz)
def qinv(q): return (-q[0], -q[1], -q[2], q[3])
def qrot(q, v):
    p = qmul(qmul(q, (v[0], v[1], v[2], 0.)), qinv(q)); return (p[0], p[1], p[2])
def norm(v):
    l = math.sqrt(sum(x * x for x in v)) or 1.; return tuple(x / l for x in v)
def arc(a, b):
    a, b = norm(a), norm(b)
    d = sum(x * y for x, y in zip(a, b))
    if d < -0.9999:
        ax = norm((0., -a[2], a[1]) if abs(a[0]) < 0.9 else (-a[1], a[0], 0.)); return (ax[0], ax[1], ax[2], 0.)
    c = (a[1]*b[2] - a[2]*b[1], a[2]*b[0] - a[0]*b[2], a[0]*b[1] - a[1]*b[0])
    q = (c[0], c[1], c[2], 1. + d); l = math.sqrt(sum(x * x for x in q)); return tuple(x / l for x in q)
def euler_zyx(q):
    x, y, z, w = q
    m00 = 1 - 2*(y*y + z*z); m10 = 2*(x*y + w*z); m20 = 2*(x*z - w*y); m21 = 2*(y*z + w*x); m22 = 1 - 2*(x*x + y*y)
    return [math.atan2(m21, m22), math.asin(max(-1., min(1., -m20))), math.atan2(m10, m00)]
def yaw_of(q):
    f = qrot(q, (0., 0., 1.)); return math.atan2(f[0], f[2])

# God Hand's torso leans hard (the knee leans back ~67 deg, the powered punch winds up ~58 deg back); on MadWorld's
# stockier proportions that reads as the body tipping up. The upper body (spine and everything above it) is turned
# about the hips back within these limits of the facing; the legs keep the source pose [added].
LEAN_BACK = math.radians(20.)
LEAN_FORWARD = math.radians(35.)
UPPER = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 24, 25, 26}

# God Hand throws its punches side-on (Gene's pelvis turns 45-100 deg off his facing in the cross). Squared moves turn the
# hips, legs, torso, neck and shoulders back toward the facing by SQUARE of that turn while the arms keep their source
# world directions, so the fist still travels straight ahead [added].
SQUARE = 0.75
SQUARED = {15, 16, 17, 18, 19, 20, 21, 22, 23, 1, 2, 3, 4, 5, 6, 10, 11, 24, 25, 26}

# God Hand's straight punches climb: at contact Gene's fist is at or above his head (he hits taller targets / a high
# camera). Levelled moves pull an extending arm's shoulder-to-fist line down: elevation above LEVEL_FREE is kept at
# LEVEL_KEEP of its excess (none: level with the shoulder), the arm turning about the shoulder (upper arm, forearm and hand together) [added].
LEVEL_FREE = math.radians(0.)
LEVEL_KEEP = 0.
ARMS = [(7, 8, 9), (12, 13, 14)]

# God Hand's punches land with the elbow still bent (Gene's cross reaches 0.84 of his arm at contact, the jab 0.71); on
# Kreely that reads as punching from the chest. Extended moves open the striking arm's elbow toward EXTEND_TO of its
# length around each hit (from EXTEND_IN frames before the window, held through it, easing out over EXTEND_OUT), the
# fist staying on its shoulder-to-fist line. Squared, the arm keeps God Hand's side-on aim (the cross lands 50 deg across
# the chest, the powered punch 71 deg), so the same envelope also turns the striking arm about the shoulder until it
# points AIM_IN toward the centre line from straight ahead [added].
EXTEND_TO = 0.97
AIM_IN = 8.
# Squared, the other arm keeps God Hand's pulled-back line too and ends flung out to the side; through the same envelope
# the off hand comes up to a guard in front of its shoulder (GUARD: forward, up, inward from the shoulder, metres) [added].
GUARD = (0.24, 0.1, 0.14)

def two_bone(world, pos, arm, target, w, out_dir):
    """Turn the arm's upper arm and forearm (world rotations, about their joints) so the hand reaches `target`, the
    elbow bending down and toward `out_dir`; by weight w."""
    up_, fore_, hand_ = arm
    S, E, H = pos[up_], pos[fore_], pos[hand_]
    a, b = math.dist(S, E), math.dist(E, H)
    to = tuple(target[k] - S[k] for k in range(3)); d = math.sqrt(sum(x * x for x in to)) or 1.
    d = max(abs(a - b) + 1e-3, min(a + b - 1e-3, d)); dir_ = norm(to)
    cos_a = max(-1., min(1., (a * a + d * d - b * b) / (2 * a * d)))
    pole = norm((out_dir[0] * 0.6, -1., out_dir[2] * 0.6))
    dot = sum(pole[k] * dir_[k] for k in range(3)); perp = norm(tuple(pole[k] - dir_[k] * dot for k in range(3)))
    E2 = tuple(S[k] + dir_[k] * a * cos_a + perp[k] * a * math.sqrt(1 - cos_a * cos_a) for k in range(3))
    q1 = arc(tuple(E[k] - S[k] for k in range(3)), tuple(E2[k] - S[k] for k in range(3)))
    q1 = slerp((0., 0., 0., 1.), q1, w)
    for m in arm: world[m] = qmul(q1, world[m])
    E1 = tuple(S[k] + qrot(q1, tuple(E[k] - S[k] for k in range(3)))[k] for k in range(3))
    H1 = tuple(S[k] + qrot(q1, tuple(H[k] - S[k] for k in range(3)))[k] for k in range(3))
    T = tuple(S[k] + dir_[k] * d for k in range(3))
    q2 = slerp((0., 0., 0., 1.), arc(tuple(H1[k] - E1[k] for k in range(3)), tuple(T[k] - E1[k] for k in range(3))), w)
    for m in (fore_, hand_): world[m] = qmul(q2, world[m])

def slerp(a, b, t):
    d = sum(x * y for x, y in zip(a, b))
    if d < 0: b, d = tuple(-x for x in b), -d
    if d > 0.9995: q = tuple(a[k] + (b[k] - a[k]) * t for k in range(4)); l = math.sqrt(sum(x * x for x in q)); return tuple(x / l for x in q)
    th = math.acos(d); s0 = math.sin((1 - t) * th) / math.sin(th); s1 = math.sin(t * th) / math.sin(th)
    return tuple(a[k] * s0 + b[k] * s1 for k in range(4))
EXTEND_IN, EXTEND_OUT = 3, 5
STRIKING_ARM = {9: (7, 8, 9), 14: (12, 13, 14)}

def smooth(x): x = min(1., max(0., x)); return x * x * (3. - 2. * x)

def extend_weight(f, windows):
    """0..1 envelope of the extension at frame f over the hit windows [(from, to, arm)], per arm."""
    out = {}
    for a, b, arm in windows:
        w = smooth((f - (a - EXTEND_IN)) / EXTEND_IN) if f < a else 1. if f <= b else 1. - smooth((f - b) / EXTEND_OUT)
        out[arm] = max(out.get(arm, 0.), w)
    return out

def axis_angle(a, angle):
    a = norm(a); s_ = math.sin(angle / 2); return (a[0] * s_, a[1] * s_, a[2] * s_, math.cos(angle / 2))

def load(name): return json.loads((SRC / name).read_text(encoding='utf-8'))

def mirror_clip(c):
    """Mirror across the YZ plane: swap left / right, negate x translations and the quaternions' y, z."""
    out = dict(c); n = len(c['skeleton']['bones'])
    out['frames'] = []
    for fr in c['frames']:
        new = [None] * n
        for i in range(n):
            t = fr[MIRROR.get(i, i)] if MIRROR.get(i, i) < n else fr[i]
            new[i] = [-t[0], t[1], t[2], t[3], -t[4], -t[5], t[6], t[7], t[8], t[9]]
        out['frames'].append(new)
    out['root_motion'] = [dict(r, actor_translation=[-r['actor_translation'][0], r['actor_translation'][1], r['actor_translation'][2]],
                               actor_rotation=[r['actor_rotation'][0], -r['actor_rotation'][1], -r['actor_rotation'][2], r['actor_rotation'][3]])
                          for r in c['root_motion']]
    return out

def mw_rest(bones):
    """MadWorld bank bones (id, parent = index into the list, relative) -> ({id: world rest position}, {id: parent id})."""
    pos, par = {}, {}
    for b in bones:   # parents precede children
        pi = b['parent']
        par[b['id']] = bones[pi]['id'] if pi >= 0 else -1
        base = pos[par[b['id']]] if pi >= 0 else (0., 0., 0.)
        pos[b['id']] = tuple(base[k] + b['relative'][k] for k in range(3))
    return pos, par

def retarget(clip, target_bones, target_hip, name, upright=False, square=False, level=False, extend=()):
    """`level`: False, or the extension (fist distance / arm length) at which an arm starts levelling. `extend`: hit
    windows [(from, to, (upper arm, forearm, hand))] whose striking arm straightens into the hit."""
    sk = clip['skeleton']['bones']; n = len(sk)
    par = [b['parent'] for b in sk]
    rest_t = [b['rest_local']['translation'] for b in sk]
    # source rest world positions (identity rest rotations)
    sw = [None] * n
    for i in range(n):
        sw[i] = tuple(rest_t[i]) if par[i] < 0 else tuple(sw[par[i]][k] + rest_t[i][k] for k in range(3))
    src_hip = sw[0][1]
    tw, tpar = mw_rest(target_bones)
    align = {}
    for s, m in MAP.items():
        if s in CHILD and m in MW_CHILD:
            ds = tuple(sw[CHILD[s]][k] - sw[s][k] for k in range(3)); dt = tuple(tw[MW_CHILD[m]][k] - tw[m][k] for k in range(3))
            align[m] = arc(dt, ds)
        else: align[m] = (0., 0., 0., 1.)
    inv_map = {m: s for s, m in MAP.items()}
    order = [b['id'] for b in target_bones]
    scale = target_hip / src_hip
    seg12 = tuple(tw[2][k] - tw[1][k] for k in range(3)); seg23 = tuple(tw[3][k] - tw[2][k] for k in range(3))
    frames = len(clip['frames'])
    rot = {m: [] for m in order}
    root_x, root_z, hip_y, hip_x, hip_z = [], [], [], [], []
    t0 = clip['root_motion'][0]['actor_translation']
    for f in range(frames):
        fr = clip['frames'][f]; rm = clip['root_motion'][f]
        actor = tuple(rm['actor_rotation'])
        # source world rotations (actor yaw included at the hips)
        W = [None] * n
        for i in range(n):
            q = tuple(fr[i][3:7])
            W[i] = qmul(actor, q) if par[i] < 0 else qmul(W[par[i]], q)
        world = {}
        for m in order:
            if m in inv_map: world[m] = qmul(W[inv_map[m]], align[m])
            else: world[m] = world.get(tpar[m], (0., 0., 0., 1.)) if tpar[m] >= 0 else (0., 0., 0., 1.)
        if square:
            # the pelvis pivot's turn off the facing, taken back by SQUARE about the vertical
            fwd = qrot(actor, (0., 0., 1.)); pf = qrot(W[17], (0., 0., 1.))
            turn = math.atan2(pf[0], pf[2]) - math.atan2(fwd[0], fwd[2])
            turn = (turn + math.pi) % (2 * math.pi) - math.pi
            q = axis_angle((0., 1., 0.), -turn * SQUARE)
            for m in order:
                if m in SQUARED: world[m] = qmul(q, world[m])
        if level is not False:
            # target FK positions from the world rotations (bind offsets), then each arm's shoulder-to-fist elevation
            pos = {}
            for b in target_bones:
                pid = tpar[b['id']]
                pos[b['id']] = (0., 0., 0.) if pid < 0 else tuple(pos[pid][k] + qrot(world[pid], b['relative'])[k] for k in range(3))
            for up_, fore_, hand_ in ARMS:
                v = tuple(pos[hand_][k] - pos[up_][k] for k in range(3))
                l = math.sqrt(sum(x * x for x in v)) or 1.
                elev = math.asin(max(-1., min(1., v[1] / l)))
                # only as the arm straightens into the punch (the bent guard keeps its fists up)
                reach = sum(math.dist(pos[a_], pos[b_]) for a_, b_ in ((up_, fore_), (fore_, hand_)))
                ext = min(1., max(0., (l / (reach or 1.) - level) / 0.2))
                if elev > LEVEL_FREE and ext > 0.:
                    want = LEVEL_FREE + (elev - LEVEL_FREE) * LEVEL_KEEP
                    axis = norm((v[2], 0., -v[0]))   # horizontal, across the arm: turning by +angle lowers it
                    q = axis_angle(axis, (elev - want) * ext)
                    for m in (up_, fore_, hand_): world[m] = qmul(q, world[m])
        weights = extend_weight(f, extend)
        for (up_, fore_, hand_), w in weights.items():
            if w <= 0.: continue
            pos = {}
            for b in target_bones:
                pid = tpar[b['id']]
                pos[b['id']] = (0., 0., 0.) if pid < 0 else tuple(pos[pid][k] + qrot(world[pid], b['relative'])[k] for k in range(3))
            S, E, H = pos[up_], pos[fore_], pos[hand_]
            a, b = math.dist(S, E), math.dist(E, H); l = math.dist(S, H)
            want = l + (max(l, (a + b) * EXTEND_TO) - l) * w
            if want <= l + 1e-4 or a * b == 0.: continue
            se = tuple(E[k] - S[k] for k in range(3)); eh = tuple(H[k] - E[k] for k in range(3))
            hinge = (se[1] * eh[2] - se[2] * eh[1], se[2] * eh[0] - se[0] * eh[2], se[0] * eh[1] - se[1] * eh[0])
            if math.sqrt(sum(x * x for x in hinge)) < 1e-6: continue
            hinge = norm(hinge)
            # the elbow's interior angle now and for the wanted reach; the forearm opens about the hinge
            cur = math.acos(max(-1., min(1., (a * a + b * b - l * l) / (2 * a * b))))
            new = math.acos(max(-1., min(1., (a * a + b * b - want * want) / (2 * a * b))))
            best = None
            for sign in (1., -1.):
                q = axis_angle(hinge, sign * (new - cur))
                h2 = tuple(E[k] + qrot(q, eh)[k] for k in range(3))
                err = abs(math.dist(S, h2) - want)
                if best is None or err < best[0]: best = (err, q, h2)
            _, q, h2 = best
            for m in (fore_, hand_): world[m] = qmul(q, world[m])
            # then the whole arm turns about the shoulder so the fist keeps its line
            back = arc(tuple(h2[k] - S[k] for k in range(3)), tuple(H[k] - S[k] for k in range(3)))
            for m in (up_, fore_, hand_): world[m] = qmul(back, world[m])
        for (up_, fore_, hand_), w in weights.items():
            if w <= 0.: continue
            # aim: about the vertical through the shoulder, the shoulder-to-fist line toward the facing (a little inward)
            pos = {}
            for b in target_bones:
                pid = tpar[b['id']]
                pos[b['id']] = (0., 0., 0.) if pid < 0 else tuple(pos[pid][k] + qrot(world[pid], b['relative'])[k] for k in range(3))
            v = tuple(pos[hand_][k] - pos[up_][k] for k in range(3))
            if v[0] * v[0] + v[2] * v[2] < 1e-6: continue
            fwd = qrot(tuple(rm['actor_rotation']), (0., 0., 1.))
            side = qrot(tuple(rm['actor_rotation']), (1., 0., 0.))
            # inward is toward the centre line: away from the shoulder's side (+x is the left)
            left = sum(pos[up_][k] * side[k] for k in range(3)) > 0
            want = math.atan2(fwd[0], fwd[2]) + math.radians(-AIM_IN if left else AIM_IN)
            delta = (want - math.atan2(v[0], v[2]) + math.pi) % (2 * math.pi) - math.pi
            q = axis_angle((0., 1., 0.), delta * w)
            for m in (up_, fore_, hand_): world[m] = qmul(q, world[m])
        # the off hand up to its guard while the other arm strikes
        for arm in ARMS:
            if weights.get(arm, 0.) > 0.: continue
            w = max(weights.values(), default=0.)
            if w <= 0.: continue
            pos = {}
            for b in target_bones:
                pid = tpar[b['id']]
                pos[b['id']] = (0., 0., 0.) if pid < 0 else tuple(pos[pid][k] + qrot(world[pid], b['relative'])[k] for k in range(3))
            fwd = qrot(tuple(rm['actor_rotation']), (0., 0., 1.)); side = qrot(tuple(rm['actor_rotation']), (1., 0., 0.))
            S = pos[arm[0]]
            out = 1. if sum(S[k] * side[k] for k in range(3)) > 0 else -1.   # +side is the left
            target = tuple(S[k] + fwd[k] * GUARD[0] - side[k] * out * GUARD[2] + (0., GUARD[1], 0.)[k] for k in range(3))
            two_bone(world, pos, arm, target, w, tuple(side[k] * out for k in range(3)))
        if upright:
            # the spine's lean against the facing (the actor's yaw), clamped by turning the upper body about the hips
            fwd = qrot(actor, (0., 0., 1.)); fwd = norm((fwd[0], 0., fwd[2]))
            # the hips-to-neck line through the spine and the chest
            a_, b_ = qrot(world[1], seg12), qrot(world[2], seg23)
            spine = tuple(a_[k] + b_[k] for k in range(3))
            lean_back = math.atan2(-(spine[0] * fwd[0] + spine[2] * fwd[2]), spine[1])
            fix = (lean_back - LEAN_BACK) if lean_back > LEAN_BACK else (lean_back + LEAN_FORWARD) if lean_back < -LEAN_FORWARD else 0.
            if fix:
                q = axis_angle((fwd[2], 0., -fwd[0]), fix)   # up x fwd: turns the spine toward the facing
                for m in order:
                    if m in UPPER: world[m] = qmul(q, world[m])
        for m in order:
            pw = world[tpar[m]] if tpar[m] >= 0 else (0., 0., 0., 1.)
            rot[m].append(euler_zyx(qmul(qinv(pw), world[m])))
        at = rm['actor_translation']
        root_x.append((at[0] - t0[0]) * scale); root_z.append((at[2] - t0[2]) * scale)
        h = qrot(actor, fr[0][0:3])
        hip_x.append(h[0] * scale); hip_y.append(fr[0][1] * scale); hip_z.append(h[2] * scale)
    tracks = []
    for m in order:
        if m not in inv_map and m not in (15,): continue
        # every rotation has two Euler forms ((x, y, z) and (x + pi, pi - y, z + pi)); taking each frame's form nearest the
        # previous one (and unwrapping) keeps the per-channel interpolation from flipping through a gimbal swap
        series = []
        for e in rot[m]:
            if not series: series.append(e); continue
            prev = series[-1]; best = None
            for cand in (e, [e[0] + math.pi, math.pi - e[1], e[2] + math.pi]):
                c = [prev[k] + ((cand[k] - prev[k] + math.pi) % (2 * math.pi) - math.pi) for k in range(3)]
                dist = sum((c[k] - prev[k]) ** 2 for k in range(3))
                if best is None or dist < best[0]: best = (dist, c)
            series.append(best[1])
        for ch in range(3):
            tracks.append(dict(bone=m, channel=3 + ch, encoding=6, values=[round(series[f][ch], 6) for f in range(frames)]))
    tracks += [dict(bone=-1, channel=0, encoding=6, values=[round(v, 6) for v in root_x]),
               dict(bone=-1, channel=1, encoding=6, values=[0.] * frames),
               dict(bone=-1, channel=2, encoding=6, values=[round(v, 6) for v in root_z]),
               dict(bone=0, channel=0, encoding=6, values=[round(v, 6) for v in hip_x]),
               dict(bone=0, channel=1, encoding=6, values=[round(v, 6) for v in hip_y]),
               dict(bone=0, channel=2, encoding=6, values=[round(v, 6) for v in hip_z])]
    return dict(id=f"godhand:{clip['source']['motion_id']}", name=name, frames=frames, tracks=tracks)

def cue(op, frame, a=0, b=0, c=0, d=0, f=0.):
    return dict(type=op, frame=float(frame), a=a, b=b, c=c, d=d, f=f)

# Kreely's moves: (key, source file, mirrored, striking MadWorld part, attack id, power, hit windows (source frames),
#  rate, whoosh / impact sound ids). Parts: 9 right hand, 14 left hand, 17 right knee, 19 right toe, 22 left foot.
# Attack ids pick RinRin's reaction (0x6D dm_right, 0x6E dm_left, others dm_body; 0x77 / 0x78 = the chainsaw rows:
# dm_explosion, the knockdown) [added moveset; windows from each clip's SEQ, index.json].
MOVES = [
    ('cross', 'gene_150_cross_punch.json', False, 14, 0x6D, 10, [(9, 11)], 1.25, 0),
    ('cross_m', 'gene_150_cross_punch.json', True, 9, 0x6E, 10, [(9, 11)], 1.25, 1),
    ('knee', 'gene_119_knee.json', False, 17, 0x6F, 12, [(15, 17)], 1.2, 2),
    ('uppercuts', 'gene_105_uppercut_combo.json', False, 9, 0x70, 15, [(17, 19), (44, 46)], 1.15, 3),
    # the user's 256 / 351 / 373 / 535 are Gene's 255 / 350 / 372 / 534 now: renaming "candidate 166" to "Sweep kick"
    # moved every later label down by one (the first export took the henchman motions that still carry the old numbers)
    ('step_jab', 'gene_372_step_jab.json', False, 14, 0x6D, 10, [(6, 8)], 1.0, 4),
    ('turn_jab', 'gene_534_turn_jab.json', False, 9, 0x6E, 10, [(8, 10)], 1.0, 5),
    ('power_punch', 'gene_154_powered_punch.json', False, 9, 0x76, 25, [(56, 59)], 1.3, 6),
    ('spin_kick', 'gene_350_spin_kick.json', False, 23, 0x77, 20, [(16, 18)], 1.1, 7),
    ('sweep', 'gene_166_sweep_kick.json', False, 19, 0x78, 15, [(13, 15)], 1.1, 12),
    ('rush', 'gene_255_punch_combo.json', False, 9, 0x5E, 12, [(4, 6, 9), (11, 13, 14), (24, 26, 14)], 1.2, 13),
    ('whip', 'gene_063_pistol_whip.json', False, 9, 0x6D, 14, [(6, 9)], 1.2, 0),
    ('whip_alt', 'gene_069_pistol_whip_alt.json', False, 9, 0x6E, 14, [(13, 17)], 1.3, 1),
    # the grab pair's "receiver" (em00:325, a henchman) is the one doing the suplex -- it bridges back and stands up --
    # while Gene (em00:327) is lifted to 1.4 m, slammed by tick 36 and left lying: Kreely plays the henchman's half
    ('suplex', 'grab_0011c348_suplex_receiver.json', False, 9, 0x55, 0, [], 1.0, 0),
]
RECEIVERS = [('suplex_recv', 'grab_0011c348_suplex_gene.json'), ('sweep_down', 'smallfry_375_sweep_knockdown_reaction.json'),
             ('getup', 'smallfry_getup_candidate.json')]

def main():
    jack = json.loads((ROOT / 'assets/native/jack-player-motion.json').read_text())
    rin = json.loads((ROOT / 'assets/native/rinrin-motion.json').read_text())
    kung = json.loads((ROOT / 'assets/native/kungfu-motion.json').read_text())
    hip = lambda bank: next(b for b in bank['bones'] if b['id'] == 0)['relative'][1]
    out = dict(source='assets/native/godhand (God Hand explorer exports)', tool='tools/godhand/retarget_godhand.py', clips=[], cues={}, moves={},
               rin_clips=[], grunt_clips=[])
    for key, file, mirrored, part, atk, power, windows, rate, sound in MOVES:
        c = load(file)
        if mirrored: c = mirror_clip(c)
        name = f'kreely_gh_{key}.mot'
        clip = retarget(c, jack['bones'], hip(jack), name, upright=key != 'suplex',
                        square=key in ('cross', 'cross_m', 'step_jab', 'rush', 'whip', 'whip_alt'),
                        # straight punches level from a third of their extension so they travel flat into the hit; the pistol
                        # whips only once nearly straight (their overhead swing is the blow)
                        level={'cross': 0.35, 'cross_m': 0.35, 'step_jab': 0.35, 'turn_jab': 0.35, 'rush': 0.35, 'power_punch': 0.35,
                               'whip': 0.65, 'whip_alt': 0.65}.get(key, False),
                        # the straight punches reach out into the hit (the uppercuts and whips stay bent by design)
                        extend=[(w[0], w[1], STRIKING_ARM[w[2] if len(w) > 2 else part]) for w in windows]
                               if key in ('cross', 'cross_m', 'step_jab', 'turn_jab', 'rush', 'power_punch') else ())
        out['clips'].append(clip)
        seq = [cue(1, 0, f=rate)]
        for w in windows:   # (from, to[, striking part for this hit])
            a, b = w[0], w[1]
            seq += [cue(0x13, a, a=w[2] if len(w) > 2 else part, b=power), cue(0x14, b + 1)]
        if windows: seq.append(cue(0x0B, windows[-1][1] + 2))
        out['cues'][name.replace('.mot', '_00.seq')] = seq
        snd = [cue(29, max(0, w[0] - 2), a=1101, b=sound) for w in windows] + [cue(38, w[0], a=11, b=min(sound, 9)) for w in windows[:1]]
        out['cues'][name.replace('.mot', '_10.seq')] = snd
        out['moves'][key] = dict(clip=name, atk=atk, power=power, part=part, windows=windows, rate=rate, frames=clip['frames'], source=file, mirrored=mirrored)
    for key, file in RECEIVERS:
        c = load(file)
        out['rin_clips'].append(retarget(c, rin['bones'], hip(rin), f'gh_{key}.mot'))
        out['grunt_clips'].append(retarget(c, kung['bones'], hip(kung), f'gh_{key}.mot'))
    # suplex placement (grab_0011c348_suplex_pair.json): the receiver's container in Gene's container frame, scaled to Kreely
    pair = load('grab_0011c348_suplex_pair.json')
    k0 = pair['alignment_keys'][0]['actors']
    g, r = k0[0], k0[1]
    # the thrown actor (Gene's container) in the suplexer's (the receiver's) frame: local = RotY(-yaw_r) (g - r)
    wx, wz = g['position'][0] - r['position'][0], g['position'][2] - r['position'][2]
    cy, sy = math.cos(-r['yaw']), math.sin(-r['yaw'])
    lx, lz = cy * wx + sy * wz, -sy * wx + cy * wz
    s = hip(jack) / load('grab_0011c348_suplex_receiver.json')['frames'][0][0][1]
    # the slam: the thrown hips drop from 1.4 m (tick 28) to 0.33 m (tick 36); Gene's own SEQ hit window is 38-39
    out['suplex'] = dict(offset=[lx * s, 0., lz * s], yaw=g['yaw'] - r['yaw'], frames=pair['frame_count'],
                         slam=[37, 38], note='thrown actor container relative to the suplexer container at tick 0 (pair alignment key)')
    OUT.write_text(json.dumps(out, separators=(',', ':')))
    print('wrote', OUT, len(out['clips']), 'kreely clips,', len(out['rin_clips']), 'receiver clips; suplex', out['suplex'])

if __name__ == '__main__':
    main()
