"""Build the vampire zako (Elise's bat-born women) on the kung fu grunt's em0101 skeleton, as Kreely is built on Jack's.

python -I tools/vampzako/build_vampzako.py [1 2 3] [--blender PATH]

Inputs (local): C:/Users/Matteo/Desktop/newmadworldchars/vampirezakoN/vampirezakoN.fbx + .fbm/vampirezakoN_basecolor.jpg
                (user-provided Tripo meshes, unrigged, T-pose) and the owned em/em0101.dat#em0101.mdb (the explorer's
                MDB reader) for the grunt's joints and proportions.
Outputs: assets/native/vampzakoN-source.glb, assets/native/vampzakoN-color.glb, assets/native/vampzakoN-rig.json,
         assets/characters/vampzako/build/vampzakoN.build.json (landmarks, weights, hashes)

Skeleton: em0101's 34 bones with the same ids, parents and zero bind rotations, but the woman's own joint positions
(the grunt's joints carried into her proportions by the Kreely landmark warp). em0101's MOTs move body joints by
rotation only (translation tracks: the root, bone 0, the weapon bones), so the grunt bank drives this skeleton
directly; the port keeps her bind offsets and scales the root translation by the hip-height ratio (rig rootScale).
Added spring bones (play_springs.rs `VAMP`): breasts 250 (left, +x) / 251 under the chest (2); the buttocks
role_0_bone200 / 201 under the hips (15); two hair ropes 260-262 (left) / 263-265 (right) under the head (4).
Meshes, by the grunt's MDB mesh numbering so the street's gore treats them as his: 2 the head (and hair) - the
decapitation gib, 3 below the waist - the waist split's legs, 4 the rest of the body.
Colour: the source texture is her own (black and white); the colour texture is a gradient map per tone [added]:
pale skin, black-violet latex with a violet sheen, near-black hair with a wine tint.
"""
import argparse, hashlib, json, subprocess, sys
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.spatial import cKDTree

HERE = Path(__file__).resolve().parent
PORT = HERE.parents[1]
sys.path.insert(0, str(PORT / 'tools/kreely'))
import build_kreely as bk          # landmarks, warp, smooth (the Kreely fit)
import kreely_fingers as kfing     # finger chains on T-pose hands
import export_kreely_glb as eg     # the GLB writer (same conventions as the staged actors)

EXPLORER = PORT.parents[1] / 'exports' / 'madworld'
SRC = Path(r'C:\Users\Matteo\Desktop\newmadworldchars')
OUT = PORT / 'assets/characters/vampzako/build'
NATIVE = PORT / 'assets/native'
GRUNT_ID = 'em/em0101.dat#em0101.mdb@12992'
BLENDER = r'C:\Program Files\Blender Foundation\Blender 4.4\blender.exe'
HEIGHT = {1: 1.80, 2: 1.76, 3: 1.78}        # metres (the grunt stands 2.02) [added]
DEFORM = [1, 2, 3, 4, 5, 10, 7, 8, 9, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23]
SIDES = {'L': (1, 14), 'R': (-1, 9)}            # +x is her left; hand bone ids
FINGERS = ['thumb', 'index', 'middle', 'ring', 'pinky']
FINGER_ID0 = 0x600                              # as Kreely's: unused by every MOT
# the kung fu claw: fingers half curled, thumb tucked in front [added]
CLAW = dict(fingers=(35, 50, 35), pinky_extra=8, thumb=(30, 20, 15, 25))
LEGS = {-1: (16, 17, 18, 19), 1: (20, 21, 22, 23)}  # the grunt's 16-19 sit at -x, 20-23 at +x
BREAST = (250, 251); BUTT = (200, 201); HAIR = ((260, 261, 262), (263, 264, 265)); HAIR_TIP = (266, 267)   # unweighted rope ends
HAIR_ROPES = False   # her hair rides on her head (no hair ropes, by request); the rope code stays for later
# tone gradients (shadow -> light), sRGB [added]: black latex with a grey sheen, black hair
GRAD = dict(skin=('#120a0e', '#f3e4e6'), latex=('#050505', '#6a6a70'), hair=('#040404', '#8c8c94'), lips=('#3a0610', '#b0202e'))


def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def grunt():
    sys.path.insert(0, str(EXPLORER / 'builders'))
    from native import mdb
    catalog = json.loads((EXPLORER / 'catalog.json').read_text(encoding='utf-8'))
    item = next(m for m in catalog['models'] if m['id'] == GRUNT_ID)
    bank = next(b for b in catalog['banks'] if b['id'] == item['bank'])
    with open(bank['path'], 'rb') as f:
        f.seek(item['offset']); data = f.read(item['size'])
    assert hashlib.sha256(data).hexdigest() == item['sha256'], 'em0101.mdb changed'
    return item, mdb(data)


def dump(blender, fbx, tmp):
    out = tmp / 'raw.npz'
    subprocess.run([blender, '-b', '--factory-startup', '--python', str(PORT / 'tools/kreely/dump_fbx_mesh.py'), '--', str(fbx), str(out)],
                   check=True, capture_output=True)
    return dict(np.load(out))


def heat(blender, tmp, S, T, ids, heads, tails):
    """Bone heat (Kreely's script; then on a merged copy when that leaves most vertices unweighted; then segment
    distance weights [added fallback] for whatever heat still could not reach)."""
    np.savez(tmp / 'heat-in.npz', positions=S, triangles=T, bone_ids=np.array(ids), heads=np.array(heads), tails=np.array(tails))
    log = []
    # (bone heat is touchy on some Tripo meshes: the same mesh and bones turned 180 degrees about y can solve where the
    # upright one fails; the weights do not depend on the turn)
    turn = np.array([-1, 1, -1])
    np.savez(tmp / 'heat-in-turned.npz', positions=S * turn, triangles=T, bone_ids=np.array(ids), heads=np.array(heads) * turn, tails=np.array(tails) * turn)
    for script, inp in ((PORT / 'tools/kreely/blender_weights.py', 'heat-in.npz'), (HERE / 'blender_heat_merged.py', 'heat-in.npz'),
                        (HERE / 'blender_heat_merged.py', 'heat-in-turned.npz')):
        run = subprocess.run([blender, '-b', '--factory-startup', '--python', str(script), '--',
                              str(tmp / inp), str(tmp / 'heat-out.npz')], capture_output=True, text=True)
        if run.returncode: raise RuntimeError(run.stdout[-3000:] + run.stderr[-3000:])
        log += [f'{script.name} {inp}: ' + l for l in run.stdout.splitlines() if 'weighted' in l]
        with np.load(tmp / 'heat-out.npz') as z: W = z['weights'].copy()
        if (W.sum(1) > 0).mean() > 0.9: break
    miss = W.sum(1) <= 1e-6
    if miss.mean() > 0.5:
        # nearest bone segments, inverse distance^4 over the closest three [added]
        H, Tl = np.array(heads), np.array(tails); seg = Tl - H
        t = np.clip(np.einsum('vbk,bk->vb', S[:, None] - H[None], seg) / np.maximum((seg * seg).sum(1), 1e-9), 0, 1)
        dist = np.linalg.norm(S[:, None] - (H[None] + t[..., None] * seg[None]), axis=2)
        w = 1 / np.maximum(dist, 0.01) ** 4; keep = np.argsort(dist, 1)[:, :3]
        mask = np.zeros_like(w, bool); np.put_along_axis(mask, keep, True, 1); w = np.where(mask, w, 0)
        W = (w / w.sum(1, keepdims=True)).astype(np.float32); log.append('segment-distance fallback for every vertex')
    return W, log


def fix_landmarks(K, S, T, top):
    """Her armpit and crotch, measured on the torso only: the Kreely finder takes the widest point under the arm (her
    slightly lowered hands reach into that band) and the lowest slice crossing x = 0 (her knees touch)."""
    band = S[(S[:, 1] > K['armUnderside'] - 0.06 * top) & (S[:, 1] < K['armUnderside'] - 0.02 * top) & (S[:, 0] > 0) & (S[:, 0] < 0.2 * top)]
    if len(band): K['armpitX'] = float(band[:, 0].max())
    for y in np.linspace(0.6 * top, 0.3 * top, 300):           # down from the hips: the first slice with a gap at x = 0
        if not any(min(a[0], b[0]) <= 0 <= max(a[0], b[0]) for a, b in bk.slice_segments(S, T, y)):
            K['crotchY'] = float(y); break
    return K


def allowed_bones(S, cols, pos, K, fingers, u):
    """(V, B) bool: which bones may weight which vertex. Sides: a vertex off the midline takes no limb of the other
    side; arms only out past the armpit (the shoulder bones down to just under it), hands and fingers past the wrist;
    each leg bone only below its own top; the trunk not on the shins; neck and head only above the shoulders."""
    x, y = S[:, 0], S[:, 1]; ax = np.abs(x); V = len(S)
    side_l = x > 0.005 * u; side_r = x < -0.005 * u
    L = {10, 11, 12, 13, 14, 20, 21, 22, 23} | {FINGER_ID0 + i for i in range(0x20)}
    R = {5, 6, 7, 8, 9, 16, 17, 18, 19} | {FINGER_ID0 + 0x20 + i for i in range(0x20)}
    pit, wrist, under = K['armpitX'], K['wristX'], K['armUnderside']
    elbow = abs(pos[13][0])
    ok = np.ones((V, len(cols)), bool)
    for k, b in enumerate(cols):
        m = np.ones(V, bool)
        if b in L: m &= ~side_r
        if b in R: m &= ~side_l
        if b in (7, 12): m &= (ax > pit - 0.01 * u) | ((ax > 0.7 * pit) & (y > under - 0.01 * u))
        if b in (8, 13): m &= ax > elbow - 0.12 * u
        if b in (9, 14): m &= ax > wrist - 0.05 * u
        if b >= FINGER_ID0 and b < FINGER_ID0 + 0x40: m &= ax > wrist - 0.01 * u
        if b in (5, 6, 10, 11): m &= y > under - 0.09 * u
        if b in (16, 20): m &= y < pos[16][1] + 0.12 * u
        if b in (17, 21): m &= y < pos[17][1] + 0.1 * u
        if b in (18, 22): m &= y < pos[18][1] + 0.08 * u
        if b in (19, 23): m &= y < pos[18][1] + 0.02 * u
        if b in (1, 2, 15): m &= (y > pos[17][1] + 0.12 * u) & (ax < wrist - 0.08 * u)
        if b in (3, 4): m &= (y > under - 0.02 * u) & (ax < 0.2 * u)
        ok[:, k] = m
    # every vertex keeps at least the bone nearest its own region (never all-zero)
    none = ~ok.any(1)
    if none.any(): ok[none] = True
    return ok


def mirror_map(S, cols):
    """W -> the weights of each vertex's mirror image (x -> -x), left and right bones swapped."""
    swap = {5: 10, 6: 11, 7: 12, 8: 13, 9: 14, 16: 20, 17: 21, 18: 22, 19: 23, 250: 251, 200: 201, 260: 263, 261: 264, 262: 265}
    swap.update({v: k for k, v in list(swap.items())})
    for i in range(0x20): swap[FINGER_ID0 + i] = FINGER_ID0 + 0x20 + i; swap[FINGER_ID0 + 0x20 + i] = FINGER_ID0 + i
    perm = np.array([cols.index(swap.get(b, b)) if swap.get(b, b) in cols else k for k, b in enumerate(cols)])
    _, nn = cKDTree(S).query(S * [-1, 1, 1])
    return lambda W: W[nn][:, perm]


def adjacency(T, n):
    """Row-normalised mesh neighbour matrix (scipy sparse)."""
    from scipy import sparse
    i = np.r_[T[:, 0], T[:, 1], T[:, 2], T[:, 1], T[:, 2], T[:, 0]]; j = np.r_[T[:, 1], T[:, 2], T[:, 0], T[:, 0], T[:, 1], T[:, 2]]
    A = sparse.coo_matrix((np.ones(len(i), np.float32), (i, j)), shape=(n, n)).tocsr(); A.data[:] = 1
    deg = np.asarray(A.sum(1)).ravel(); deg[deg == 0] = 1
    return sparse.diags(1 / deg) @ A


def constrain(W, ok, S=None):
    W = np.where(ok, W, 0).astype(np.float32)
    s = W.sum(1, keepdims=True)
    # a vertex whose every weight was forbidden takes its nearest valid neighbour's weights (never an even share of
    # everything it may take: that scattered feet over fingers and hair)
    bad = s[:, 0] <= 1e-6
    if bad.any() and S is not None and (~bad).any():
        _, nn = cKDTree(S[~bad]).query(S[bad]); W[bad] = W[~bad][nn]; s = W.sum(1, keepdims=True)
    return W / np.maximum(s, 1e-9)


def rot_between(a, b):
    """Shortest-arc quaternion (xyzw) turning direction a onto direction b."""
    a = a / np.linalg.norm(a); b = b / np.linalg.norm(b); c = np.cross(a, b); w = 1 + a @ b
    if w < 1e-6: return np.array([1., 0, 0, 0])
    q = np.r_[c, w]; return q / np.linalg.norm(q)


def claw_rotations(fingers):
    """Local finger rotations for the claw, as Kreely's hand poses (bind frames are identity)."""
    q = {}
    for side_name in SIDES:
        det = fingers[side_name]; down = det['palmDown']
        fdir = np.mean([det[f][3] - det[f][0] for f in FINGERS[1:]], 0); fdir /= np.linalg.norm(fdir)
        across = det['pinky'][0] - det['index'][0]; across /= np.linalg.norm(across)
        for f, fname in enumerate(FINGERS):
            base = FINGER_ID0 + (0 if side_name == 'L' else 0x20) + f * 4
            jts = det[fname]; d = jts[3] - jts[0]; d /= np.linalg.norm(d)
            if fname == 'thumb':
                swing, cross, bend1, bend2 = CLAW['thumb']
                q[base] = bk.qmul(bk.axis_angle(np.cross(d, fdir), cross), bk.axis_angle(np.cross(d, down), swing))
                q[base + 1] = bk.axis_angle(np.cross(d, across), bend1); q[base + 2] = bk.axis_angle(np.cross(d, across), bend2)
            else:
                for k, deg in enumerate(CLAW['fingers']):
                    q[base + k] = bk.axis_angle(np.cross(d, down), deg + (CLAW['pinky_extra'] if fname == 'pinky' else 0))
    return q


def synthetic_fingers(S, sgn, K, hand_at):
    """Fingers placed across the hand when they cannot be told apart on the mesh (mitten hands): four along the hand's
    back half, the thumb in front."""
    x = S[:, 0] * sgn; H = S[x > K['wristX']]
    tip = float((H[:, 0] * sgn).max()); L = tip - K['wristX']
    z0, z1 = np.percentile(H[:, 2], 8), np.percentile(H[:, 2], 92); y = float(np.median(H[:, 1]))
    out = {}
    for k, name in enumerate(['index', 'middle', 'ring', 'pinky']):
        z = z1 - (k + 0.5) / 4 * (z1 - z0) * 0.8 - 0.1 * (z1 - z0)
        base = np.array([sgn * (K['wristX'] + 0.45 * L), y, z]); end = np.array([sgn * tip * (0.97 if k in (0, 3) else 1), y, z])
        out[name] = np.array([base, base + (end - base) * 0.45, base + (end - base) * 0.75, end])
    tb = np.array([sgn * (K['wristX'] + 0.15 * L), y - 0.1 * L, z1]); te = np.array([sgn * (K['wristX'] + 0.6 * L), y - 0.1 * L, z1 + 0.15 * L])
    out['thumb'] = np.array([tb, tb + (te - tb) * 0.38, tb + (te - tb) * 0.7, te])
    out['palmDown'] = np.array([0., -1., 0.])
    return out


def hexrgb(h): return np.array([int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)], np.float32)


def build(n, blender):
    fbx = SRC / f'vampirezako{n}/vampirezako{n}.fbx'; jpg = SRC / f'vampirezako{n}/vampirezako{n}.fbm/vampirezako{n}_basecolor.jpg'
    tmp = OUT / f'work{n}'; tmp.mkdir(parents=True, exist_ok=True)
    gitem, G0 = grunt()
    raw = dump(blender, fbx, tmp)
    S0, src, uv, Ts = raw['source_positions'], raw['source_vertex'], raw['uvs'], raw['indices']
    Tw = src[Ts]
    tex = Image.open(jpg).convert('RGB'); lum_img = tex.convert('L')
    L = np.asarray(lum_img, np.float32) / 255
    vlum = L[(np.clip(uv[:, 1], 0, 1) * (L.shape[0] - 1)).astype(int), (np.clip(uv[:, 0], 0, 1) * (L.shape[1] - 1)).astype(int)]
    wl = np.zeros(len(S0)); cnt = np.zeros(len(S0)); np.add.at(wl, src, vlum); np.add.at(cnt, src, 1); wlum = wl / np.maximum(cnt, 1)

    # grunt reference: the body without the cut faces and outfit B props; joints in MDB order
    keep = [m for m in G0['meshes'] if not any(k in m['name'] for k in ('DA', 'kasa', 'hyoutan', 'himo', 'ben', 'tume'))]
    GP = np.concatenate([np.array(m['positions']) for m in keep]); GT, off = [], 0
    for m in keep: GT.append(np.array(m['indices']).reshape(-1, 3) + off); off += len(m['positions'])
    gb = G0['bones']; gid = [b['id'] for b in gb]; gpos = np.array([b['position'] for b in gb])
    J = bk.landmarks(GP, np.concatenate(GT), side=1, wrist=gpos[gid.index(14)][0])

    # 1) her: scale to HEIGHT, feet on the floor, centred on x, facing +z (the breasts are in front)
    S = S0 * (HEIGHT[n] / np.ptp(S0[:, 1])); S[:, 1] -= S[:, 1].min(); S[:, 0] -= (S[:, 0].max() + S[:, 0].min()) / 2
    top = float(S[:, 1].max())
    # the toes point forward: the feet reach further out of the shin column on the front side
    feet = S[S[:, 1] < 0.03 * top]; shins = S[(S[:, 1] > 0.12 * top) & (S[:, 1] < 0.2 * top)]
    zc = np.median(shins[:, 2])
    flipped = (zc - feet[:, 2].min()) > (feet[:, 2].max() - zc)
    if flipped: S[:, 0] *= -1; S[:, 2] *= -1
    band = lambda P, t: P[(P[:, 1] > 0.6 * t) & (P[:, 1] < 0.75 * t) & (np.abs(P[:, 0]) < 0.08 * t)]
    gc = band(GP, J['top']); hc = band(S, top)
    dz = float((gc[:, 2].max() + gc[:, 2].min()) / 2 - (hc[:, 2].max() + hc[:, 2].min()) / 2); S[:, 2] += dz
    tip = float(S[:, 0].max())
    K = fix_landmarks(bk.landmarks(S, Tw, side=1, wrist=tip - 0.105 * top), S, Tw, top)

    # 2) joints: the grunt's carried into her proportions (midline kept, arms on her arm axis)
    kpos = bk.warp(gpos, J, K)
    kpos[[i for i, b in enumerate(gid) if b in (0, 1, 2, 3, 4, 15, 24, 30, 31, 32, 33, 34, 35, 36)], 0] = 0
    for i, b in enumerate(gid):
        if b in (5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 100, 101):
            kpos[i, 1] = K['armY'] + (gpos[i, 1] - J['armY']); kpos[i, 2] = K['armZ'] + (gpos[i, 2] - J['armZ'])
    # her legs: hip, knee and ankle on the centre of her own leg at those heights, the toe at the front of her foot
    # (the grunt's knee sits 11 cm behind his leg line; on her straight legs the knee would pivot behind the knee)
    for side, (hip, knee, ankle, toe) in ((1, LEGS[1]), (-1, LEGS[-1])):
        for b in (knee, ankle):
            i = gid.index(b); y = kpos[i, 1]
            q = S[(np.abs(S[:, 1] - y) < (0.015 if b == knee else 0.035) * top) & (S[:, 0] * side > 0.01 * top) & (S[:, 0] * side < 0.2 * top)]
            if len(q) >= 6: kpos[i, 0], kpos[i, 2] = (q[:, 0].max() + q[:, 0].min()) / 2, (q[:, 2].max() + q[:, 2].min()) / 2
        i = gid.index(hip); kpos[i, 2] = kpos[gid.index(knee), 2]
        # the hip in the top of her thigh, just under the crotch
        q = S[(np.abs(S[:, 1] - (K['crotchY'] - 0.03 * top)) < 0.015 * top) & (S[:, 0] * side > 0.005 * top) & (S[:, 0] * side < 0.2 * top)]
        if len(q) >= 6: kpos[i, 0], kpos[i, 2] = (q[:, 0].max() + q[:, 0].min()) / 2, (q[:, 2].max() + q[:, 2].min()) / 2
        foot = S[(S[:, 1] < 0.06 * top) & (S[:, 0] * side > 0.01 * top) & (S[:, 0] * side < 0.2 * top)]
        a = kpos[gid.index(ankle)]
        if len(foot): kpos[gid.index(toe)] = [a[0], max(float(foot[:, 1].min()) + 0.02 * top, 0.02), float(foot[:, 2].max()) - 0.03 * top]
    # her head and neck joints: the head bone at the base of her head (the chin line, read off the front profile of her
    # face and neck), the neck bone halfway down her neck; the bones under the head move with it [the warp put the head
    # bone near her mouth, so the chin bent with the neck]
    band = np.abs(S[:, 0]) < 0.03 * top
    ys = np.linspace(top * 0.80, top * 0.95, 61); prof = np.array([S[band & (np.abs(S[:, 1] - y) < 0.004 * top), 2].max(initial=-9) for y in ys])
    ok_p = prof > -8
    if ok_p.sum() > 20:
        neck_k = int(np.argmin(np.where(ok_p & (ys < top * 0.90), prof, 9)))
        neck_front = prof[neck_k]
        above = [k for k in range(neck_k, len(ys)) if ok_p[k] and prof[k] > neck_front + 0.02 * top / 1.8]
        if above:
            chin_y = float(ys[above[0]]); i4 = gid.index(4); i3 = gid.index(3)
            ring = S[(np.abs(S[:, 1] - chin_y) < 0.01 * top) & (np.abs(S[:, 0]) < 0.06 * top)]
            z4 = float((ring[:, 2].max() + ring[:, 2].min()) / 2) if len(ring) > 5 else kpos[i4, 2]
            d4 = np.array([0., chin_y - kpos[i4, 1], z4 - kpos[i4, 2]])
            under = {b for b in gid if b in (24, 30, 31, 32, 33, 34, 35, 36)}
            for i, b_ in enumerate(gid):
                if b_ == 4 or b_ in under: kpos[i] += d4
            kpos[i3, 1] = (kpos[i4, 1] + K['armY']) / 2
            ring3 = S[(np.abs(S[:, 1] - kpos[i3, 1]) < 0.01 * top) & (np.abs(S[:, 0]) < 0.06 * top)]
            if len(ring3) > 5: kpos[i3, 2] = float((ring3[:, 2].max() + ring3[:, 2].min()) / 2)
    pos = {b: kpos[i] for i, b in enumerate(gid)}
    fingers, finger_log = {}, {}
    for side_name, (sgn, hand_id) in SIDES.items():
        try:
            fingers[side_name] = kfing.detect(S, Tw, sgn, K['wristX']); finger_log[side_name] = 'detected'
        except Exception as e:
            fingers[side_name] = synthetic_fingers(S, sgn, K, pos[hand_id]); finger_log[side_name] = f'synthetic ({e})'

    # 3) heat weights on her body joints
    hand = lambda s: fingers['L' if s > 0 else 'R']['middle'][0]
    tail = {1: pos[2], 2: pos[3], 3: pos[4], 4: pos[4] + [0, 0.2 * top / 1.8, 0.02], 5: pos[7], 10: pos[12], 7: pos[8], 8: pos[9], 9: hand(-1),
            12: pos[13], 13: pos[14], 14: hand(1), 15: pos[15] - [0, 0.1, 0], 16: pos[17], 17: pos[18], 18: pos[19], 19: pos[19] + [0, 0, 0.12],
            20: pos[21], 21: pos[22], 22: pos[23], 23: pos[23] + [0, 0, 0.12]}
    fids, fheads, ftails, fparent = [], [], [], {}
    for side_name, (sgn, hand_id) in SIDES.items():
        for f, fname in enumerate(FINGERS):
            jts = fingers[side_name][fname]
            for k in range(3):
                bid = FINGER_ID0 + (0 if side_name == 'L' else 0x20) + f * 4 + k
                fids.append(bid); fheads.append(jts[k]); ftails.append(jts[k + 1]); fparent[bid] = (hand_id if k == 0 else bid - 1, jts[k])
    heat_ids = DEFORM + fids
    Wh, heat_log = heat(blender, tmp, S, Tw, heat_ids, [pos[i] for i in DEFORM] + fheads, [tail[i] for i in DEFORM] + ftails)
    cols = heat_ids + [6, 11] + list(BREAST) + list(BUTT) + [b for c in HAIR for b in c]
    col = {b: k for k, b in enumerate(cols)}
    W = np.c_[Wh, np.zeros((len(S), len(cols) - len(heat_ids)), np.float32)]
    empty = W.sum(1) <= 1e-6
    if empty.any():
        _, nn = cKDTree(S[~empty]).query(S[empty]); W[empty] = W[~empty][nn]
    for clav, helper, upper in ((5, 6, 7), (10, 11, 12)):          # the grunt's deltoid helpers, as Kreely's
        r = np.linalg.norm(S - pos[upper], axis=1); zone = 1 - bk.smooth(0.05, 0.13, r)
        m = W[:, col[clav]] + W[:, col[upper]]; give = 0.8 * zone * m
        for b in (clav, upper):
            W[:, col[b]] -= give * np.divide(W[:, col[b]], m, out=np.zeros_like(m), where=m > 0)
        W[:, col[helper]] += give
    W /= W.sum(1, keepdims=True)
    u = top / 1.8                                                  # metres per "unit woman"

    # 3b) clean-up: bone heat on these meshes bleeds across the body (one leg into the other, arms into the torso
    # sides, the neck and hair into the face); each bone may only hold the vertices it can own, then a few smoothing
    # passes over the mesh, the limits again each time
    allow = allowed_bones(S, cols, pos, K, fingers, u)
    adj = adjacency(Tw, len(S))
    W = constrain(W, allow, S)
    mirror = mirror_map(S, cols)
    for _ in range(4):
        W = constrain(0.5 * W + 0.5 * (adj @ W), allow, S)
        W = constrain(0.5 * (W + mirror(W)), allow, S)          # her mesh is symmetric: so are her weights
    # the clavicles keep only the shoulder tops: heat spread them over the upper chest, and the grunt's clips swing them
    # as the arms rise, dragging the chest and shoulders; inward of the shoulder and below the armpit line their weight
    # goes to the chest (2)
    ax_ = np.abs(S[:, 0])
    keep_c = bk.smooth(0.55 * K['armpitX'], 0.95 * K['armpitX'], ax_) * bk.smooth(K['armUnderside'] - 0.03 * u, K['armUnderside'] + 0.03 * u, S[:, 1])
    for clav in (5, 10):
        give = W[:, col[clav]] * (1 - keep_c)
        W[:, col[clav]] -= give; W[:, col[2]] += give
    # the shoulders take the grunt's own skinning: his weights were authored for these clips (chest, clavicle, deltoid
    # helper and upper arm sharing the joint), so his mesh, warped into her proportions as her joints are, lends each of
    # her shoulder vertices the weights of its nearest point; blended in from the top of the chest out to the elbow
    GJ = np.concatenate([np.array(m['joints']) for m in keep]); GWt = np.concatenate([np.array(m['weights'], np.float32) for m in keep])
    GPw = bk.warp(GP, J, K)
    arm_g = bk.smooth(J['armpitX'] * 0.6, J['armpitX'], np.abs(GP[:, 0])) * bk.smooth(J['armUnderside'] - 0.1 * J['top'], J['armUnderside'], GP[:, 1])
    GPw[:, 1] += arm_g * ((K['armY'] - J['armY']) - (bk.warp(np.array([[0, J['armY'], 0]]), J, K)[0, 1] - J['armY']))
    GPw[:, 2] += arm_g * (K['armZ'] - J['armZ'])
    _, near_g = cKDTree(GPw).query(S)
    Wg = np.zeros_like(W)
    for k in range(4):
        for gi, b_ in enumerate(gid):
            if b_ not in col: continue
            m_ = GJ[near_g, k] == gi
            Wg[m_, col[b_]] += GWt[near_g, k][m_]
    Wg = np.where(allow, Wg, 0); sg = Wg.sum(1, keepdims=True)
    ax_ = np.abs(S[:, 0]); elbow_x = abs(pos[13][0])
    zone = bk.smooth(0.35 * K['armpitX'], 0.6 * K['armpitX'], ax_) * (1 - bk.smooth(elbow_x - 0.06 * u, elbow_x + 0.02 * u, ax_))         * bk.smooth(K['armUnderside'] - 0.12 * u, K['armUnderside'] - 0.04 * u, S[:, 1]) * (sg[:, 0] > 0.5)
    Wg = np.divide(Wg, sg, out=np.zeros_like(Wg), where=sg > 0)
    W = W * (1 - zone[:, None]) + Wg * zone[:, None]
    weight_shoulder = int((zone > 0.5).sum())
    # the head is rigid above the neck (the face and skull all on the head bone); the neck blends neck -> head
    # the head's centre: the middle of her head mesh above the jaw line (not the head bone's depth: on her it sits at the
    # back of the neck, which left her mouth outside the head)
    hv = S[(S[:, 1] > pos[4][1] + 0.04 * u) & (np.abs(S[:, 0]) < 0.15 * u)]
    hc = np.array([0., (pos[4][1] + top) / 2, (np.percentile(hv[:, 2], 2) + np.percentile(hv[:, 2], 98)) / 2])
    skull = np.linalg.norm((S - hc) * [1, 0.85, 1], axis=1)
    y3, y4 = pos[3][1], pos[4][1]
    # (the grunt's head bone sits at about mouth height: her chin and jaw, in front of the neck down to 8 cm under it,
    # are head too, or they lag in the neck blend and the face twists when she turns her head)
    chin = (S[:, 1] > y4 - 0.03 * u) & (S[:, 2] > pos[4][2] + 0.03 * u) & (np.abs(S[:, 0]) < 0.075 * u)
    inhead = (skull < 0.17 * u) & ((S[:, 1] > y4 - 0.01 * u) | chin)
    W[inhead] = 0; W[inhead, col[4]] = 1
    neck = ~inhead & (S[:, 1] > y3 - 0.02 * u) & (S[:, 1] <= y4 + 0.01 * u) & (np.abs(S[:, 0]) < 0.085 * u)
    tn = np.clip((S[neck, 1] - (y3 - 0.02 * u)) / (y4 - y3 + 0.03 * u), 0, 1)
    W[neck] = 0; W[neck, col[3]] = 1 - tn; W[neck, col[4]] = tn
    weight_report = dict(head=int(inhead.sum()), neck=int(neck.sum()), shoulderTransfer=weight_shoulder)

    # 4) spring bones and their weights
    extra = []                                                     # (id, parent id, position)

    def blob(ids, side, ylo, yhi, front):
        pts = S[(S[:, 1] > ylo * top) & (S[:, 1] < yhi * top) & (S[:, 0] * side > 0.02 * u) & (S[:, 0] * side < 0.16 * u)]
        apex = pts[np.argmax(pts[:, 2] * front)]
        return apex

    reports = {}
    for bid, side in zip(BREAST, (1, -1)):
        apex = blob(bid, side, 0.66, 0.80, 1)
        piv = apex - [0, -0.01 * u, 0.09 * u]; extra.append((bid, 2, piv))
        c = apex - [0, 0, 0.035 * u]; d = np.linalg.norm(S - c, axis=1); R = 0.085 * u
        w = (1 - bk.smooth(0.45 * R, R, d)) * bk.smooth(piv[2] - 0.01 * u, piv[2] + 0.03 * u, S[:, 2]) * 0.85
        W *= (1 - w)[:, None]; W[:, col[bid]] += w; reports[f'breast{bid}'] = dict(apex=apex.round(3).tolist(), verts=int((w > 0.05).sum()))
    for bid, side in zip(BUTT, (1, -1)):
        apex = blob(bid, side, 0.46, 0.62, -1)
        piv = apex + [0, 0.01 * u, 0.1 * u]; extra.append((bid, 15, piv))
        c = apex + [0, 0, 0.04 * u]; d = np.linalg.norm(S - c, axis=1); R = 0.11 * u
        w = (1 - bk.smooth(0.4 * R, R, d)) * bk.smooth(piv[2] + 0.01 * u, piv[2] - 0.04 * u, S[:, 2]) * 0.7
        W *= (1 - w)[:, None]; W[:, col[bid]] += w; reports[f'butt{bid}'] = dict(apex=apex.round(3).tolist(), verts=int((w > 0.05).sum()))
    # hair: dark vertices standing off the skull (not the scalp, not the face), above the base of the neck; the
    # ropes take them more the further out they hang
    face = (S[:, 2] > hc[2] + 0.03 * u) & (S[:, 1] < hc[1] + 0.07 * u)
    hairm = (wlum < 0.42) & (S[:, 1] > y3 - 0.02 * u) & ~face & (skull > 0.11 * u) & (np.abs(S[:, 0]) < 0.25 * u)
    if hairm.sum() > 30:
        bottom = float(np.percentile(S[hairm, 1], 3)); back = float(np.percentile(S[hairm, 2], 10))
    else:
        bottom = hc[1] - 0.12 * u; back = hc[2] - 0.09 * u
    chains = {}
    for chain, side in zip(HAIR, (1, -1)):
        a = np.array([side * 0.05 * u, hc[1], hc[2] - 0.08 * u]); b = np.array([side * 0.075 * u, min(bottom, hc[1] - 0.08 * u), back])
        pts = [a + (b - a) * k / 3 for k in range(3)]
        for k, bid in enumerate(chain): extra.append((bid, 4 if k == 0 else chain[k - 1], pts[k]))
        extra.append((HAIR_TIP[0 if side == 1 else 1], chain[-1], b))
        chains[side] = (a, b)
    for side, chain in (zip((1, -1), HAIR) if HAIR_ROPES else []):
        a, b = chains[side]; ab = b - a; t = np.clip(((S - a) @ ab) / (ab @ ab), 0, 1)
        sw = bk.smooth(-0.02 * u, 0.02 * u, S[:, 0] * side)
        # (hair lying on the shoulders stays with the body: roped, it pulled away from the shoulder skin it touches)
        # (and only hair hanging below the rope's start: crown hair on a rope pivoting at the back of the skull swung
        # into spikes)
        reach = hairm * sw * bk.smooth(0.11 * u, 0.15 * u, skull) * bk.smooth(K['armY'] + 0.03 * u, K['armY'] + 0.1 * u, S[:, 1])             * (1 - bk.smooth(a[1] - 0.06 * u, a[1], S[:, 1])) * 0.95
        # split along the rope: bone k peaks at t = (k + 0.5) / 3
        f = t * 3
        tot = np.stack([np.clip(1 - np.abs(f - (k + 0.5)), 0, 1) for k in range(3)], 1); tot[:, 2] += np.clip(f - 2.5, 0, None)
        tot[:, 0] += np.clip(0.5 - f, 0, None); tot /= tot.sum(1, keepdims=True)
        W *= (1 - reach)[:, None]
        for k, bid in enumerate(chain): W[:, col[bid]] += reach * tot[:, k]
    reports['hair'] = dict(verts=int(hairm.sum()), bottom=round(bottom, 3), back=round(back, 3))
    reports['fingers'] = finger_log; reports['weights'] = weight_report
    for bid, (pid, at) in fparent.items(): extra.append((bid, pid, at))
    W /= W.sum(1, keepdims=True)

    # 5) skeleton: grunt bones (her positions) + the spring bones
    bones = []
    for i, b in enumerate(gb):
        p = b['parent']; rel = kpos[i] - (kpos[p] if p >= 0 else 0)
        bones.append(dict(id=b['id'], parent=p, position=np.round(kpos[i], 6).tolist(), relative=np.round(rel, 6).tolist()))
    idx = {b['id']: i for i, b in enumerate(bones)}
    for bid, pid, ppos in extra:
        p = idx[pid]; bones.append(dict(id=bid, parent=p, position=np.round(ppos, 6).tolist(), relative=np.round(ppos - np.array(bones[p]['position']), 6).tolist()))
        idx[bid] = len(bones) - 1
    order = np.argsort(-W, 1)[:, :4]; top4 = np.take_along_axis(W, order, 1)
    top4 = np.where(top4 > 0.01, top4, 0); top4 /= top4.sum(1, keepdims=True)
    joints = np.array([idx[c] for c in cols])[order]; joints[top4 == 0] = joints[:, :1].repeat(4, 1)[top4 == 0]
    # the final skin, for tools/vampzako/view_weights.py (bone ids per influence)
    np.savez(tmp / 'skin.npz', S=S, T=Tw, joints=np.array([b['id'] for b in bones])[joints], weights=top4,
             bone_ids=np.array([b['id'] for b in bones]), bone_pos=np.array([b['position'] for b in bones]))
    fn = np.cross(S[Tw[:, 1]] - S[Tw[:, 0]], S[Tw[:, 2]] - S[Tw[:, 0]]); vn = np.zeros_like(S)
    for k in range(3): np.add.at(vn, Tw[:, k], fn)
    vn /= np.linalg.norm(vn, axis=1, keepdims=True) + 1e-12

    # 6) meshes: head (2), legs below the waist (3), the rest (4), by triangle
    headish = {idx[b] for b in (3, 4) + HAIR[0] + HAIR[1]}
    dom = joints[:, 0]
    waist = (pos[15][1] + pos[2][1]) / 2 - 0.05 * u
    tri_head = np.array([all(dom[v] in headish for v in t) and S[t, 1].min() > pos[3][1] - 0.02 * u for t in Tw])
    tri_legs = ~tri_head & (S[Tw, 1].mean(1) < waist)
    # the Tripo meshes are fused where her legs touch in the T-pose (inner thighs, knees, heels): a triangle joining a
    # left-leg vertex to a right-leg one stretches into a sheet once the legs part, so those go
    leg_l = {idx[b] for b in (20, 21, 22, 23)}; leg_r = {idx[b] for b in (16, 17, 18, 19)}
    side = np.array([1 if d in leg_l else -1 if d in leg_r else 0 for d in dom])
    bridge = np.array([(side[t] == 1).any() and (side[t] == -1).any() for t in Tw])
    keep_tri = ~bridge
    tri_head &= keep_tri; tri_legs &= keep_tri
    weight_report['bridgesRemoved'] = int(bridge.sum())
    r6 = lambda v: np.round(v, 6).tolist()
    meshes = []
    for part, mask in ((2, tri_head), (3, tri_legs), (4, keep_tri & ~tri_head & ~tri_legs)):
        tri = Ts[mask]; used = np.unique(tri); remap = -np.ones(len(src), int); remap[used] = np.arange(len(used))
        vs = src[used]
        meshes.append(dict(name=f'vampzako{n}_part{part}', part=part, material=0, texture=0, renderFlags=8192, inkThreshold=140,
                           positions=r6(S[vs]), normals=r6(vn[vs]), uvs=r6(uv[used]), indices=remap[tri].reshape(-1).tolist(),
                           joints=joints[vs].tolist(), weights=np.round(top4[vs], 5).tolist()))

    # 7) textures: her own (source) and the gradient-map colour version
    (OUT).mkdir(parents=True, exist_ok=True)
    tex = crisp(tex, S, src, uv, Ts, hc, u)
    tex.save(OUT / f'vampzako{n}_source.png', optimize=True)
    color = color_texture(tex, S, src, uv, Ts, hairm, top, hc, u)
    color.save(OUT / f'vampzako{n}_color.png', optimize=True)

    root_scale = float(kpos[idx[0]][1] / gpos[gid.index(0)][1])
    # bind corrections: on each limb bone, the turn from her bone direction (to its child) to the grunt's, so his
    # rotation-only clips bend her limbs as they bend his (local = C(parent)^-1 * clip * C(bone)) [added]
    corr = {}
    # (not the feet: her heels stand on the toes as modelled, and her hip height already carries them)
    for b, c in ((16, 17), (17, 18), (20, 21), (21, 22), (7, 8), (8, 9), (12, 13), (13, 14)):
        hd = kpos[gid.index(c)] - kpos[gid.index(b)]; gd = gpos[gid.index(c)] - gpos[gid.index(b)]
        corr[b] = rot_between(hd, gd)
    claw = claw_rotations(fingers)
    model = dict(bones=bones, meshes=meshes, retarget=dict(skeleton=GRUNT_ID, rootScale=round(root_scale, 6), rotationOnly=True),
                 springs=dict(breasts=list(BREAST), butt=list(BUTT), hair=[list(c) for c in HAIR]),
                 provenance=dict(mesh=f'user-provided vampirezako{n}.fbx (Tripo; not disc data)',
                                 skeleton=GRUNT_ID + ' ids/parents/bind rotations; joint positions fitted to her; +10 spring bones',
                                 weights='Blender bone heat, island copy, grunt deltoid helpers; breast / butt / hair spring falloffs [added]'))
    for mode, png in (('source', OUT / f'vampzako{n}_source.png'), ('color', OUT / f'vampzako{n}_color.png')):
        eg.MESHES = [m['name'] for m in meshes]
        data = eg.glb(dict(bones=bones, meshes=[dict(m, name=m['name']) for m in meshes]), {0: png})
        # mesh nodes are named role_0_mesh_<i> in order; rename them to the grunt's part numbers
        data = rename_parts(data, [m['part'] for m in meshes], claw)
        (NATIVE / f'vampzako{n}-{mode}.glb').write_bytes(data)
    rig = dict(bones=[dict(id=b['id'], parent=b['parent'], relative=b['relative']) for b in bones], rootScale=round(root_scale, 6),
               correction={str(k): np.round(v, 6).tolist() for k, v in corr.items()},
               springs=model['springs'], height=HEIGHT[n])
    (NATIVE / f'vampzako{n}-rig.json').write_text(json.dumps(rig, separators=(',', ':')), encoding='utf-8')
    report = dict(source=dict(fbx=str(fbx), fbxSha256=sha(fbx), texture=str(jpg), textureSha256=sha(jpg)), flippedToFaceZ=bool(flipped),
                  height=HEIGHT[n], rootScale=root_scale, gruntLandmarks=J, landmarks=K, springs=reports, heatLog=heat_log,
                  heatFallbackVertices=int(empty.sum()), triangles=dict(head=int(tri_head.sum()), legs=int(tri_legs.sum()), body=int((~tri_head & ~tri_legs).sum())),
                  outputs={m: sha(NATIVE / f'vampzako{n}-{m}.glb') for m in ('source', 'color')})
    (OUT / f'vampzako{n}.build.json').write_text(json.dumps(report, indent=1), encoding='utf-8')
    print(n, json.dumps(dict(flipped=report['flippedToFaceZ'], rootScale=round(root_scale, 3), springs=reports, triangles=report['triangles'], heat=heat_log)))


def rename_parts(data, parts, rest=None):
    """Mesh nodes to the grunt's part numbers; finger nodes get their rest (claw) rotation (the pose never writes them,
    so the skin bends them by it)."""
    import struct
    jl = struct.unpack_from('<I', data, 12)[0]; js = json.loads(data[20:20 + jl])
    k = 0
    for nd in js['nodes']:
        if nd.get('name', '').startswith('role_0_mesh_'): nd['name'] = f'role_0_mesh_{parts[k]}'; k += 1
        bid = nd.get('name', '')[len('role_0_bone'):] if nd.get('name', '').startswith('role_0_bone') else None
        if rest and bid and bid.lstrip('-').isdigit() and int(bid) in rest: nd['rotation'] = [float(v) for v in rest[int(bid)]]
    j2 = json.dumps(js, separators=(',', ':')).encode(); j2 += b' ' * (-len(j2) % 4)
    rest = data[20 + jl:]
    return b'glTF' + struct.pack('<II', 2, 12 + 8 + len(j2) + len(rest)) + struct.pack('<I', len(j2)) + b'JSON' + j2 + rest


def crisp(tex, S, src, uv, Ts, hc, u):
    """Her ink, sharpened toward clean black and white lines [added]: the Tripo texture is soft (its faces smudged),
    so an unsharp mask and a contrast curve toward the ink, strongest on the head (within 17 cm of its centre),
    lighter on the body; only on the islands (the gutters keep their texels)."""
    from scipy import ndimage
    size = tex.size[0]
    P = bk.texel_positions(S, src, uv, Ts, size); cov = ~np.isnan(P[..., 0])
    head = np.zeros(cov.shape, np.float32)
    d = np.linalg.norm((P[cov] - hc) * [1, 0.85, 1], axis=1); head[cov] = 1 - bk.smooth(0.15 * u, 0.19 * u, d)
    L = np.asarray(tex.convert('L'), np.float32) / 255
    blur = ndimage.gaussian_filter(L, 1.4)
    amount = 0.7 + 1.1 * head
    sharp = np.clip(L + amount * (L - blur), 0, 1)
    ink = bk.smooth(0.28, 0.62, sharp)
    k = 0.3 + 0.45 * head
    out = sharp * (1 - k) + ink * k
    out = np.where(cov, out, L)
    g = (out * 255 + 0.5).astype(np.uint8)
    return Image.fromarray(np.dstack([g, g, g]))


def clothing_mask(P, cov, Lm, u, voxel=0.02, cut=0.56):
    """Skin weight per texel (1 skin, 0 clothing), from how dark the surface is around it in 3-D, not the texel itself:
    her latex is black with white highlight streaks, her skin white with thin ink lines, so the mean over ~6 cm of
    surface tells them apart where the texel alone cannot. Texel positions are binned in 2 cm voxels, averaged,
    box-blurred over the 3 x 3 x 3 neighbourhood, and looked up per texel [added]."""
    from scipy import ndimage
    v = voxel * u
    pts = P[cov]; lum = Lm[cov]
    lo = pts.min(0); ijk = np.floor((pts - lo) / v).astype(int); shape = ijk.max(0) + 3
    sums = np.zeros(shape); cnt = np.zeros(shape)
    np.add.at(sums, tuple((ijk + 1).T), lum); np.add.at(cnt, tuple((ijk + 1).T), 1)
    sums = ndimage.uniform_filter(sums, 3, mode='constant') * 27; cnt = ndimage.uniform_filter(cnt, 3, mode='constant') * 27
    mean = np.divide(sums, cnt, out=np.full(shape, 0.5), where=cnt > 0)
    # (trilinear between voxel centres: a nearest-voxel lookup left 2 cm blocks wherever the mean sat near the cut)
    from scipy.ndimage import map_coordinates
    q = ((P - lo) / v + 0.5).reshape(-1, 3).T
    m = map_coordinates(mean, q, order=1, mode='nearest').reshape(P.shape[:2])
    return bk.smooth(cut - 0.06, cut + 0.06, m)


def color_texture(tex, S, src, uv, Ts, hairm, top, hc, u):
    """Per texel: the tone (skin where bright, latex where dark; hair inside the hair mask; lips by the mouth) and a
    gradient map of the source luminance (shadow -> light)."""
    size = tex.size[0]
    pos = bk.texel_positions(S, src, uv, Ts, size)            # (H, W, 3), NaN off the islands
    from scipy import ndimage
    cov = ~np.isnan(pos[..., 0])
    near = ndimage.distance_transform_edt(~cov, return_distances=False, return_indices=True)
    P = pos[near[0], near[1]]
    # hair texels: nearest vertex in the hair mask
    tree = cKDTree(S); _, nv = tree.query(P.reshape(-1, 3)); hair = hairm[nv].reshape(P.shape[:2])
    Lm = np.asarray(tex.convert('L'), np.float32) / 255
    dec = lambda c: np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    enc = lambda c: np.where(c <= 0.0031308, c * 12.92, 1.055 * np.power(np.maximum(c, 0), 1 / 2.4) - 0.055)
    def grad(name, t):
        a, b = (dec(hexrgb(h)) for h in GRAD[name]); return a + (b - a) * t[..., None]
    t = np.clip(Lm, 0, 1)
    skin_w = clothing_mask(P, cov, Lm, u)
    # the face and neck are skin, whatever ink is on them (eyes, brows and hair shadow are dark enough to pass for latex)
    d = np.linalg.norm((P - hc) * [1, 0.85, 1], axis=-1)
    face = (1 - bk.smooth(0.15 * u, 0.19 * u, d)) * bk.smooth(hc[1] - 0.24 * u, hc[1] - 0.18 * u, P[..., 1])
    skin_w = np.maximum(skin_w, face)[..., None]
    out = grad('skin', t) * skin_w + grad('latex', np.clip(t * 1.6, 0, 1)) * (1 - skin_w)
    hw = (hair & (Lm < 0.55))[..., None].astype(np.float32)
    out = out * (1 - hw) + grad('hair', np.clip(t * 1.4, 0, 1)) * hw
    # (no lip tint: her lips are black ink, as drawn; a guessed mouth box missed and painted the eyes)
    return Image.fromarray(np.clip(enc(out) * 255 + 0.5, 0, 255).astype(np.uint8))


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('which', nargs='*', type=int, default=[1, 2, 3]); ap.add_argument('--blender', default=BLENDER)
    a = ap.parse_args()
    for n in a.which: build(n, a.blender)


if __name__ == '__main__':
    main()
