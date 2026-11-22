"""Alternate Rin Power Struggle loss: fan scissors on Jack's neck (added animation, not disc data).

Builds two explorer motion clips from the original loss pair (em0440_0113 / pl0030_04ad) and the struggle loop
(em0440_0111):
  em0440_0113sc  Rin: original start, keeps moving on Jack (struggle loop body), leans in with her hands beside his
                 head, swings both fans wide open, then sweeps them inward so the blades cross through each other at
                 his neck and all the way past, stays seated squeezing, releases, hops off, original recovery tail.
  pl0030_04adsc  Jack: original start, pinned, convulses as the blades pass through his neck, then goes limp.

Source clips are read from the running explorer (/api/clip, decoded per-frame tracks). FK/IK follows the explorer's
three.js conventions (tools/anim/rig_fk.py). Output goes to <explorer>/custom/clips/ with an index.json.

    python -I tools/anim/rin_scissor_loss.py --base http://127.0.0.1:8795 --explorer D:/Games/gamesfordecomp/exports/madworld
"""
import argparse, hashlib, json, math, sys, urllib.parse, urllib.request
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rig_fk import Pose, axis_angle, align, two_bone_ik  # noqa: E402

RIN_MODEL = 'em/em0440.dat#em0440.mdb@5792'
RIN_LOSS = 'em/em0440.dat#em0440_0113.mot@1719232'
RIN_LOOP = 'em/em0440.dat#em0440_0111.mot@1678336'
RIN_GETUP = 'em/em0440.dat#em0440_0022.mot@1507904'   # deep fan crouch -> stand; f10-29 used as her get-up
JACK_MODEL = 'pl/pl0030.dat#pl0030.mdb@19552'
JACK_LOOP = 'em/em0440.dat#pl0030_04ab.mot@1957088'    # Jack's half of the struggle loop (pairs with 0111)
JACK_LOSS = 'em/em0440.dat#pl0030_04ad.mot@1976384'
FRAMES = 232
CUT = 50                        # blades pass through each other at Jack's neck (mid snap)
ATTACK_START, ATTACK_END, ATTACK_ARRIVE = 20, 36, 38  # attack clock: frame 20-38 maps to attack time 20-36, eased in/out
SMOOTH_SIGMA = 1.6              # gaussian smoothing (frames) over all final curves
DETACH = 34                     # Jack's head comes off and he convulses, inside the attack window
RELEASE = (95, 128)             # she eases out of the attack pose into her seated idle

# Jack's neck in the explorer's paired frame (pl0030 bone 3; measured, stays within 0.03 through his clip).
NECK = np.array([-0.03, 0.14, -0.52])
# Rin faces -z here. Her +x hand is bone 9 (blade runs along its local -X), the -x hand is bone 14 (local +X).
FANS = ((9, 7, 8, np.array([-1.0, 0, 0]), +1), (14, 12, 13, np.array([1.0, 0, 0]), -1))
HAND_OPEN_X, HAND_CLOSED_X, HAND_Y, HAND_Z = 0.50, 0.30, 0.24, -0.48
LEAN = -1.0                     # upper-body pitch about world X (radians, forward toward Jack's head)
OPEN_DEG, THROUGH_DEG = 80, -72  # blade yaw from Rin's forward (-z): + = out to her hand's side, - = across
DROP = 0.22                     # blade slope down so the tips stay just above the floor
BACK_OFF = 0.95                 # how far she steps back (+z, behind her) when she gets up; root -1 stays put


def get(base, path, ident):
    with urllib.request.urlopen(f'{base}/api/{path}?id=' + urllib.parse.quote(ident)) as r:
        return json.load(r)


def ease(t):
    t = min(1.0, max(0.0, t))
    return t * t * (3 - 2 * t)


def smoother(t):
    t = min(1.0, max(0.0, t))
    return t * t * t * (t * (6 * t - 15) + 10)


def attack_time(f):
    """Attack clock: runs the attack layers up to ATTACK_END but arrives there with zero velocity (no hard stop)."""
    if f <= ATTACK_START:
        return float(f)
    span = ATTACK_ARRIVE - ATTACK_START
    return ATTACK_START + (ATTACK_END - ATTACK_START) * smoother((f - ATTACK_START) / span)


def smooth_tracks(tracks, sigma=SMOOTH_SIGMA):
    """Gaussian low-pass over every channel (edge-clamped) to take the snap out of IK/blend corners."""
    if sigma <= 0:
        return
    r = int(math.ceil(sigma * 3))
    kern = [math.exp(-0.5 * (i / sigma) ** 2) for i in range(-r, r + 1)]
    tot = sum(kern)
    kern = [k / tot for k in kern]
    for t in tracks:
        v = t['values']
        n = len(v)
        t['values'] = [sum(kern[j + r] * v[min(n - 1, max(0, i + j))] for j in range(-r, r + 1)) for i in range(n)]


def window(f, a, b, c, d):
    """0 before a, eases to 1 by b, holds to c, eases back to 0 by d."""
    if f <= a or f >= d:
        return 0.0
    if f < b:
        return ease((f - a) / (b - a))
    if f <= c:
        return 1.0
    return 1 - ease((f - c) / (d - c))


def default(model, key):
    """Value a missing channel has in the explorer: bind offset for positions, zero rotation."""
    bone, ch = key
    if ch < 3:
        rel = next((b['relative'] for b in model['bones'] if b['id'] == bone), None)
        return rel[ch] if rel else 0.0
    return 1.0 if 7 <= ch <= 9 else 0.0


def wrap_near(a, b):
    return b + round((a - b) / (2 * math.pi)) * 2 * math.pi


class Clip:
    def __init__(self, src):
        self.src = src
        self.tracks = {(t['bone'], t['channel']): t['values'] for t in src['tracks']}

    def sample(self, f):
        out = {}
        for key, v in self.tracks.items():
            f2 = min(max(f, 0.0), len(v) - 1)
            a = int(math.floor(f2)); b = min(a + 1, len(v) - 1); w = f2 - a
            vb = wrap_near(v[a], v[b]) if key[1] in (3, 4, 5) else v[b]
            out[key] = v[a] * (1 - w) + vb * w
        return out


def blend(pa, pb, w):
    out = dict(pa)
    for key, vb in pb.items():
        va = pa.get(key, vb)
        if key[1] in (3, 4, 5):
            vb = wrap_near(va, vb)
        out[key] = va + (vb - va) * w
    return out


def getup_pose(model, getup, g):
    """em0440_0022 at frame g, yawed 180 deg so she keeps facing Jack's head (-z) while rising."""
    pose = Pose(model['bones'], getup.sample(g))
    turn = axis_angle([0, 1, 0], math.pi)
    for bone in (1, 15):
        pose.rotate_world(bone, turn)
    return pose.local


def rin_base(model, loss, loop, getup, f):
    """Body: original start, struggle loop while seated (never frozen), sits on after the kill, gets up backwards."""
    def looped(g):
        return loop.sample((g - 25) % 50)
    start = loss.sample(min(f, 24) * 16 / 24)
    if f <= 18:
        return start
    if f <= 34:
        return blend(start, looped(f), smoother((f - 18) / 16))
    if f <= 168:
        return looped(f)
    if f <= 186:
        return blend(looped(f), getup_pose(model, getup, 10), smoother((f - 168) / 18))
    return getup_pose(model, getup, 10 + 19 * smoother((f - 186) / 26))


def blade_dir(deg, side):
    a = math.radians(deg)
    d = np.array([side * math.sin(a), -DROP, -math.cos(a)])
    return d / np.linalg.norm(d)


def scissor_deg(f):
    if f < 46:
        return OPEN_DEG
    if f < 52:
        return OPEN_DEG + (THROUGH_DEG - OPEN_DEG) * ((f - 46) / 6) ** 1.6   # accelerating snap through
    return THROUGH_DEG + 3.0 * math.sin((f - 52) * 0.8) * max(0.0, 1 - (f - 52) / 50)  # squeeze


def build_rin(model, loss, loop, getup):
    out = {key: [] for key in set(loss.tracks) | set(loop.tracks) | set(getup.tracks)}
    root0 = (loss.tracks[(-1, 0)][0], loss.tracks[(-1, 2)][0])
    for f in range(FRAMES):
        local = rin_base(model, loss, loop, getup, f)
        # Head toss / sway on top of the loop (local, before FK) so she keeps performing while seated.
        live = window(f, 22, 36, 166, 184)
        local[(4, 3)] = local.get((4, 3), 0.0) + live * (0.10 * math.sin(f * 0.21)) - 0.45 * window(f, 40, 47, 62, 78)
        local[(4, 5)] = local.get((4, 5), 0.0) + live * 0.18 * math.sin(f * 0.13 + 1.0)
        pose = Pose(model['bones'], local)
        # The attack runs f0-ATTACK_END; after that its procedural layers hold that pose (no second close) while the
        # body loop keeps her moving, then ease out over RELEASE.
        g = attack_time(f)
        hold = 1 - smoother((f - RELEASE[0]) / (RELEASE[1] - RELEASE[0]))
        lean = window(g, 27, 42, 1e9, 2e9) * hold
        if lean:
            windup = 0.35 * window(g, 25, 31, 33, 40) * hold              # rears back before diving in
            bob = 0.10 * math.sin((f - 40) * 2 * math.pi / 22) * window(f, 42, 50, 96, 106)
            pose.rotate_world(1, axis_angle([1, 0, 0], (LEAN + bob) * lean + windup))
            twist = 0.18 * math.sin((f - 40) * 2 * math.pi / 36) * window(f, 44, 54, 94, 106)
            if twist:
                pose.rotate_world(1, axis_angle([0, 1, 0], twist))       # grinds side to side while lingering
        reach = window(g, 29, 42, 1e9, 2e9) * hold
        if reach:
            closing = ease((g - 46) / 6) if g >= 46 else 0.0
            hx = HAND_OPEN_X + (HAND_CLOSED_X - HAND_OPEN_X) * closing
            for hand, upper, lower, blade, side in FANS:
                target = NECK + np.array([side * hx, HAND_Y - NECK[1], HAND_Z - NECK[2]])
                cur, _ = pose.world(hand)
                shoulder, _ = pose.world(upper)
                two_bone_ik(pose, upper, lower, hand, cur + (target - cur) * reach,
                            shoulder + np.array([side * 0.6, 0.45, 0.25]))
                _, hr = pose.world(hand)
                now = hr @ blade
                want = now + (blade_dir(scissor_deg(g), side) - now) * reach
                pose.rotate_world(hand, align(now, want))
        for key in out:
            v = pose.local.get(key, default(model, key))
            if key[1] in (3, 4, 5) and out[key]:
                v = wrap_near(out[key][-1], v)
            out[key].append(v)
        if f >= 178:
            out[(0, 2)][-1] += BACK_OFF * smoother((f - 178) / 32)
            if 186 <= f <= 206:
                out[(0, 1)][-1] += 0.18 * math.sin(math.pi * (f - 186) / 20)
    out[(-1, 0)] = [root0[0]] * FRAMES
    out[(-1, 2)] = [root0[1]] * FRAMES
    tracks = [dict(bone=b, channel=c, encoding=6, values=v) for (b, c), v in sorted(out.items())]
    smooth_tracks(tracks)
    return dict(loss.src, frames=FRAMES, tracks=tracks)


JACK_FLAIL = {7: 0.45, 8: 0.6, 12: 0.45, 13: 0.6, 16: 0.3, 17: 0.4, 20: 0.3, 21: 0.4}  # arms, legs (pl0030 ids)


def jack_offset(key, f):
    """Convulsions after the cut, strongest just after it, fading so he goes limp around f95-105."""
    bone, ch = key
    if bone in JACK_FLAIL and ch == 3 and DETACH + 10 <= f < DETACH + 55:
        k = (f - DETACH - 10) / 45
        amp = (1 - k) ** 1.5 * smoother((f - DETACH - 10) / 6)
        return JACK_FLAIL[bone] * amp * math.sin((f - DETACH) * (1.1 + 0.25 * (bone % 4)))
    return 0.0


def head_path(f, start_pos, start_rot):
    """Severed head: pops up off the stump, lands, rolls away from Rin (-z) and settles."""
    u = (f - DETACH) / 36
    d = np.array([0.35, 0.0, -1.0]); d /= np.linalg.norm(d)
    dist = 0.55 * ease(min(1.0, u))
    if u < 0.4:
        lift = 0.24 * math.sin(math.pi * u / 0.4)
    elif u < 0.65:
        lift = 0.05 * math.sin(math.pi * (u - 0.4) / 0.25)
    else:
        lift = 0.0
    pos = start_pos + d * dist + np.array([0, lift, 0])
    roll = axis_angle(np.cross([0, 1, 0], d), dist / 0.13 + 0.6 * min(1.0, u / 0.4))
    return pos, roll @ start_rot


def build_jack(model, jack, loop):
    def base(f):
        def looped(g):
            return loop.sample((g - 25) % 50)
        if f <= 18: return jack.sample(f)
        if f <= 30: return blend(jack.sample(f), looped(f), smoother((f - 18) / 12))
        if f <= DETACH - 6: return looped(f)                                 # still struggling under the blades
        if f < DETACH: return blend(looped(f), jack.sample(36), smoother((f - DETACH + 6) / 6))
        if f <= DETACH + 13: return jack.sample(36 + (f - DETACH))            # original jolt as the head comes off
        if f <= DETACH + 40: return blend(jack.sample(49), jack.sample(30), smoother((f - DETACH - 13) / 27))
        return jack.sample(30)
    keys = set(jack.tracks) | set(loop.tracks) | {(4, 0), (4, 1), (4, 2)}
    out = {key: [] for key in keys}
    head0 = None
    for f in range(FRAMES):
        local = base(f)
        for key in list(local):
            local[key] += jack_offset(key, f)
        pose = Pose(model['bones'], local)
        if f >= DETACH:
            if head0 is None:
                head0 = pose.world(4)
            pos, rot = head_path(f, *head0)
            parent_pos, parent_rot = pose.world(3)
            lp = parent_rot.T @ (pos - parent_pos)
            pose.local[(4, 0)], pose.local[(4, 1)], pose.local[(4, 2)] = (float(v) for v in lp)
            pose.refresh()
            pose.set_world_rot(4, rot)
        for key in out:
            v = pose.local.get(key, default(model, key))
            if key[1] in (3, 4, 5) and out[key]:
                v = wrap_near(out[key][-1], v)
            out[key].append(v)
    tracks = [dict(bone=b, channel=c, encoding=6, values=v) for (b, c), v in sorted(out.items())]
    smooth_tracks(tracks)
    return dict(jack.src, frames=FRAMES, tracks=tracks)


def build(base, explorer):
    model = get(base, 'model', RIN_MODEL)
    rin = build_rin(model, Clip(get(base, 'clip', RIN_LOSS)), Clip(get(base, 'clip', RIN_LOOP)),
                    Clip(get(base, 'clip', RIN_GETUP)))
    jack = build_jack(get(base, 'model', JACK_MODEL), Clip(get(base, 'clip', JACK_LOSS)),
                      Clip(get(base, 'clip', JACK_LOOP)))
    out = Path(explorer) / 'custom' / 'clips'
    out.mkdir(parents=True, exist_ok=True)
    index = []
    for name, data, src, label in (
        ('em0440_0113sc.mot', rin, RIN_LOSS, 'Rin · alt loss: fan scissors (added)'),
        ('pl0030_04adsc.mot', jack, JACK_LOSS, 'Jack · alt loss: fan scissors (added)'),
    ):
        raw = json.dumps(data, separators=(',', ':'), allow_nan=False).encode()
        (out / (name + '.json')).write_bytes(raw)
        index.append(dict(id='custom/clips#' + name, bank='custom/clips', name=name, frames=data['frames'],
                          bones=sorted({t['bone'] for t in data['tracks']}), reviewedLabel=label,
                          url='custom/clips/' + name + '.json', sha256=hashlib.sha256(raw).hexdigest(),
                          derivedFrom=src, authority='added animation authored from the original loss pair'))
    (out / 'index.json').write_text(json.dumps(dict(version=1, clips=index), indent=1), encoding='utf-8')
    # Diagnostics: hand reach and blade crossing at the neck.
    m = Pose(model['bones'], {})
    rt = {(t['bone'], t['channel']): t['values'] for t in rin['tracks']}
    for f in (20, 44, 52, 80, 150, 231):
        m.local = {k: v[f] for k, v in rt.items()}; m.refresh()
        h9, r9 = m.world(9); h14, r14 = m.world(14)
        print(f'f{f}: hand9 {h9.round(2)} hand14 {h14.round(2)} '
              f'tip9 {(h9 + r9 @ np.array([-1.76, 0, 0])).round(2)} tip14 {(h14 + r14 @ np.array([1.76, 0, 0])).round(2)}')
    print('\n'.join(f"{c['name']}: {c['frames']} frames {c['sha256'][:12]}" for c in index))


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--base', default='http://127.0.0.1:8795')
    ap.add_argument('--explorer', default='D:/Games/gamesfordecomp/exports/madworld')
    a = ap.parse_args()
    build(a.base.rstrip('/'), a.explorer)
