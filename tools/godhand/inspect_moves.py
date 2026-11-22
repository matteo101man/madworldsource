"""Checks for retargeted moves (assets/native/kreely-moves.json against the exported sources in assets/native/godhand/).
The poses are rebuilt exactly as the game samples them (per-channel linear Euler interpolation, composed Z*Y*X, Jack's
pl0030 bind offsets), so what these report is what mw-play and the MadWorld explorer draw.

  python tools/godhand/inspect_moves.py snaps                 every clip: worst joint deviation half-way between frames
                                                              (a gimbal flip shows as a large value; fast swings ~0.1-0.35 m)
  python tools/godhand/inspect_moves.py levels                punches: striking fist vs its shoulder at each hit and on
                                                              the 5 frames before it (positive = above the shoulder)
  python tools/godhand/inspect_moves.py compare cross         source vs retarget bone directions (spine, right upper
                                                              arm, right thigh) on a few frames: should match
  python tools/godhand/inspect_moves.py stick cross 0,6,9,14 out.png
                                                              side-view stick figures, source (top) vs retarget (bottom)
Keys are the `moves` keys in kreely-moves.json (cross, knee, spin_kick, ...).
"""
import json, math, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import retarget_godhand as R

MOVES = json.loads((R.ROOT / 'assets/native/kreely-moves.json').read_text())
BONES = json.loads((R.ROOT / 'assets/native/jack-player-motion.json').read_text())['bones']
SOURCE = {key: file for key, file, *_ in R.MOVES}

def quat_zyx(e):
    sx, cx = math.sin(e[0] / 2), math.cos(e[0] / 2); sy, cy = math.sin(e[1] / 2), math.cos(e[1] / 2); sz, cz = math.sin(e[2] / 2), math.cos(e[2] / 2)
    return (sx * cy * cz - cx * sy * sz, cx * sy * cz + sx * cy * sz, cx * cy * sz - sx * sy * cz, cx * cy * cz + sx * sy * sz)

def clip_of(key):
    return next(c for c in MOVES['clips'] if c['name'] == MOVES['moves'][key]['clip'])

def mw_fk(clip, t):
    """Retargeted clip -> {bone id: world position} at (fractional) frame t, as the game samples it."""
    a = int(t); b = min(a + 1, clip['frames'] - 1); f = t - a
    tr = {(x['bone'], x['channel']): x['values'][a] * (1 - f) + x['values'][b] * f for x in clip['tracks']}
    W, P = {}, {}
    for bn in BONES:
        pi = bn['parent']; pid = BONES[pi]['id'] if pi >= 0 else -1
        q = quat_zyx([tr.get((bn['id'], 3 + k), 0.) for k in range(3)])
        t_ = [tr.get((0, k), bn['relative'][k]) for k in range(3)] if bn['id'] == 0 else bn['relative']
        pq, pp = (W[pid], P[pid]) if pid >= 0 else ((0, 0, 0, 1), (0, 0, 0))
        W[bn['id']] = R.qmul(pq, q); P[bn['id']] = tuple(pp[k] + R.qrot(pq, t_)[k] for k in range(3))
    return P

def src_fk(c, f):
    """Exported God Hand clip -> list of world positions by source bone index at frame f."""
    sk = c['skeleton']['bones']; fr = c['frames'][f]; actor = tuple(c['root_motion'][f]['actor_rotation'])
    W, P = [], []
    for i, b in enumerate(sk):
        q = tuple(fr[i][3:7]); t = fr[i][0:3]
        if b['parent'] < 0: W.append(R.qmul(actor, q)); P.append(R.qrot(actor, t))
        else:
            pq = W[b['parent']]; W.append(R.qmul(pq, q)); P.append(tuple(P[b['parent']][k] + R.qrot(pq, t)[k] for k in range(3)))
    return P

def direction(a, b):
    v = [b[k] - a[k] for k in range(3)]; l = math.sqrt(sum(x * x for x in v)) or 1.; return [round(x / l, 2) for x in v]

def snaps():
    for c in MOVES['clips']:
        worst = (0, 0, 0)
        for f in range(c['frames'] - 1):
            A, M, B = mw_fk(c, f), mw_fk(c, f + .5), mw_fk(c, f + 1)
            for j in (4, 9, 14, 18, 22):
                d = math.dist([(A[j][k] + B[j][k]) / 2 for k in range(3)], M[j])
                if d > worst[0]: worst = (round(d, 3), f, j)
        print(c['name'], 'worst half-frame deviation (m, frame, bone id)', worst)

def levels():
    arm = {9: (7, 9), 14: (12, 14)}
    for key, m in MOVES['moves'].items():
        seq = MOVES['cues'].get(m['clip'].replace('.mot', '_00.seq'), [])
        hits = [c for c in seq if c['type'] == 0x13]
        if not hits or m['part'] not in arm and not any(c['a'] in arm for c in hits): continue
        clip = clip_of(key); rows = []
        for c in hits:
            sh, hd = arm.get(c['a'], (7, 9)); a = int(c['frame'])
            rows.append([round(mw_fk(clip, f)[hd][1] - mw_fk(clip, f)[sh][1], 2) for f in range(max(0, a - 5), a + 1)])
        print(f'{key:12s} fist above shoulder (m), 5 frames before each hit .. the hit:', rows)

def compare(key):
    src = R.load(SOURCE[key]); clip = clip_of(key)
    for f in [0, clip['frames'] // 3, 2 * clip['frames'] // 3]:
        m = mw_fk(clip, f); s = src_fk(src, f)
        print(key, f, 'spine src', direction(s[0], s[3]), 'mw', direction(m[0], m[3]), '| upper arm R src', direction(s[7], s[8]), 'mw', direction(m[7], m[8]),
              '| thigh R src', direction(s[18], s[19]), 'mw', direction(m[16], m[17]))

def stick(key, frames, out):
    from PIL import Image, ImageDraw
    src = R.load(SOURCE[key]); clip = clip_of(key)
    SCH = [[0, 1, 2, 3, 4], [2, 5, 7, 8, 10], [2, 11, 13, 14, 16], [0, 17, 18, 19, 20, 21], [17, 22, 23, 24, 25]]
    MCH = [[0, 1, 2, 3, 4], [2, 5, 7, 8, 9], [2, 10, 12, 13, 14], [0, 15, 16, 17, 18, 19], [15, 20, 21, 22, 23]]
    W, H = 220, 240
    img = Image.new('RGB', (W * len(frames), H * 2), 'white'); dr = ImageDraw.Draw(img)
    cols = [(200, 0, 0), (0, 120, 0), (0, 0, 200), (150, 0, 150), (200, 120, 0)]   # spine, right arm, left arm, right leg, left leg
    for j, f in enumerate(frames):
        for row, (P, CH, name) in enumerate([(src_fk(src, f), SCH, 'source'), (mw_fk(clip, f), MCH, 'retarget')]):
            ox, oy = j * W + W / 2, row * H + H - 15
            dr.text((j * W + 5, row * H + 3), f'{name} f{f} side (fwd ->)', fill=(0, 0, 0))
            dr.line([(j * W, oy), (j * W + W, oy)], fill=(180, 180, 180))
            for c, ch in enumerate(CH):
                dr.line([(ox + P[i][2] * 100, oy - P[i][1] * 100) for i in ch], fill=cols[c], width=3)
    img.save(out); print('wrote', out)

if __name__ == '__main__':
    cmd = sys.argv[1] if len(sys.argv) > 1 else 'snaps'
    if cmd == 'snaps': snaps()
    elif cmd == 'levels': levels()
    elif cmd == 'compare': compare(sys.argv[2])
    elif cmd == 'stick': stick(sys.argv[2], [int(x) for x in sys.argv[3].split(',')], sys.argv[4])
    else: print(__doc__)
