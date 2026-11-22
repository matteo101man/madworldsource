"""Clipping metric (offline, same skinning math as the explorer): vertices not driven by the head/neck that end up inside the
head core, and arm vertices inside the torso core, over sampled frames of pl/pl0030.dat clips. Run for Kreely model
JSONs and for Jack (baseline) to compare.

python -I tools/kreely/clip_metrics.py <model.json|jack> [...] [--step 6]
"""
import argparse, json, sys
from pathlib import Path
import numpy as np
HERE = Path(__file__).resolve().parent; sys.path.insert(0, str(HERE))
EXPLORER = HERE.parents[3] / 'exports' / 'madworld'; sys.path.insert(0, str(EXPLORER / 'builders'))
import kreely_pose as kp
from native import motion, mdb

ARM = {5, 6, 7, 8, 9, 10, 11, 12, 13, 14}


def load(spec, catalog):
    if spec == 'jack':
        it = next(m for m in catalog['models'] if m['id'] == 'pl/pl0030.dat#pl0030.mdb@19552')
        b = next(x for x in catalog['banks'] if x['id'] == it['bank'])
        with open(b['path'], 'rb') as f: f.seek(it['offset']); m = mdb(f.read(it['size']))
        m['meshes'] = [x for x in m['meshes'] if not x['name'].startswith('danmen')]
        mesh = dict(positions=sum((x['positions'] for x in m['meshes']), []), joints=sum((x['joints'] for x in m['meshes']), []),
                    weights=sum((x['weights'] for x in m['meshes']), []))
        return m, mesh
    m = json.loads(Path(spec).read_text(encoding='utf-8')); return m, m['meshes'][0]


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('models', nargs='+'); ap.add_argument('--step', type=int, default=6)
    ap.add_argument('--clips', default='pl/pl0030.dat')
    ap.add_argument('--drivers', choices=['body', 'arm'], default='body'); a = ap.parse_args()
    cat = json.loads((EXPLORER / 'catalog.json').read_text(encoding='utf-8'))
    bank = next(x for x in cat['banks'] if x['id'] == a.clips)
    clips = []
    with open(bank['path'], 'rb') as f:
        for c in cat['clips']:
            if c['bank'] != a.clips: continue
            f.seek(c['offset']); clips.append((c['name'], motion(f.read(c['size']))))
    for spec in a.models:
        model, mesh = load(spec, cat)
        ids = [b['id'] for b in model['bones']]; J = np.array(mesh['joints']); Wt = np.array(mesh['weights'])
        armw = sum(Wt[:, k] * np.isin(np.array(ids)[J[:, k]], list(ARM)) for k in range(4))
        headw = sum(Wt[:, k] * np.isin(np.array(ids)[J[:, k]], [3, 4, 24, 26]) for k in range(4))
        P = np.array(mesh['positions']); B = kp.bind_worlds(model); i4 = ids.index(4); i2 = ids.index(2)
        # head core: bind-space box of head-weighted verts, shrunk to 75%, in bone-4 frame
        H = P[headw > 0.8] - B[i4][:3, 3]; hc = (H.max(0) + H.min(0)) / 2; hr = (H.max(0) - H.min(0)) / 2 * 0.75
        drivers = headw < 0.3 if a.drivers == 'body' else armw > 0.5   # body: everything not head/neck-driven; arm: arm/shoulder-driven
        total = 0; frames = 0; worst = []
        known = set(ids)
        for name, clip in clips:
            if any(t['bone'] not in known and t['bone'] != -1 and t['bone'] < 0xf60 and t['bone'] not in (model.get('ignoreMotionBones') or []) for t in clip['tracks']):
                continue
            for t in range(0, clip['frames'], a.step):
                W = kp.worlds(model, clip, t); V = kp.skin(model, dict(positions=P[drivers].tolist(), joints=J[drivers].tolist(), weights=Wt[drivers].tolist()), W, B)
                loc = (np.c_[V, np.ones(len(V))] @ np.linalg.inv(W[i4]).T)[:, :3] - hc
                n = int(((loc / hr) ** 2).sum(1).__lt__(1).sum()); total += n; frames += 1
                if n: worst.append((n, name, t))
        worst.sort(reverse=True)
        print(json.dumps(dict(model=spec, drivers=a.drivers, frames=frames, driverVerts=int(drivers.sum()), headCoreHits=total,
                              hitsPerFrame=round(total / max(frames, 1), 3), framesWithHits=len(worst), worst=worst[:6])))


if __name__ == '__main__':
    main()
