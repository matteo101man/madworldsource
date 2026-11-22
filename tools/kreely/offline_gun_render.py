"""Offline render (numpy poser, same math as the explorer) of Kreely playing one of Jude's handgun clips.

python -I tools/kreely/offline_gun_render.py <out.png> <overlay clip, e.g. em0400_0102 | ev93_cut8> <frame> [frame ...] [--zoom hand|body] [--mesh kreely_pistol_R_KB]
Gun meshes are flat grey, Kreely uses his color texture; the muzzle point is marked red on shot frames.
"""
import argparse, json, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
HERE = Path(__file__).resolve().parent; sys.path.insert(0, str(HERE))
EXPLORER = HERE.parents[3] / 'exports' / 'madworld'; sys.path.insert(0, str(EXPLORER / 'builders'))
import kreely_pose as kp
from native import motion
BUILD = HERE.parents[1] / 'assets/characters/kreely/build'


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('out'); ap.add_argument('clip'); ap.add_argument('frames', type=float, nargs='+')
    ap.add_argument('--zoom', default='body'); ap.add_argument('--view', default='front'); ap.add_argument('--mesh', default='kreely_pistol_R_KB'); a = ap.parse_args()
    model = json.loads((BUILD / 'kreely.model.json').read_text(encoding='utf-8'))
    cat = json.loads((EXPLORER / 'catalog.json').read_text(encoding='utf-8'))
    ev = json.loads((EXPLORER / 'events.json').read_text(encoding='utf-8'))
    def load(cid):
        it = next((x for x in cat['clips'] + ev['clips'] if x['id'] == cid.split('|')[0]))
        path = it.get('sourcePath') or next(x for x in cat['banks'] if x['id'] == it['bank'])['path']
        with open(path, 'rb') as f: f.seek(it['offset']); return kp.prepare(motion(f.read(it['size'])), cid, model['overlays'][cid])
    if a.clip.startswith('synth:'):
        clip = kp.make_synth(model, a.clip, load); ov = dict(handPose=None, shots=[])
    else:
        cid, ov = next((k, v) for k, v in model['overlays'].items() if v['clip'] == a.clip); clip = load(cid)
    shots = {s['frame'] for s in ov['shots']}
    tex = np.asarray(Image.open(BUILD / 'kreely_color.png').convert('RGB'))
    ids = [x['id'] for x in model['bones']]; B = kp.bind_worlds(model)
    meshes = [m for m in model['meshes'] if m['name'] == 'kreely_body_SB' or m['name'] == a.mesh]
    S = 420; out = Image.new('RGB', (S * len(a.frames), S + 20), (30, 34, 38))
    for k, t in enumerate(a.frames):
        W = kp.worlds(model, clip, t, ov['handPose'])  # synth: per-segment hand poses come from each overlay
        tris, cols = [], []
        for m in meshes:
            V = kp.skin(model, m, W, B); I = np.array(m['indices']).reshape(-1, 3)
            if m['name'] == 'kreely_body_SB':
                uv = np.array(m['uvs']); h, w, _ = tex.shape
                c = tex[(np.clip(uv[:, 1], 0, 1) * (h - 1)).astype(int), (np.clip(uv[:, 0], 0, 1) * (w - 1)).astype(int)].astype(float)
                cols.append(c[I].mean(1))
            else: cols.append(np.tile([150, 155, 165], (len(I), 1)))
            tris.append(V[I])
        T = np.concatenate(tris); C = np.concatenate(cols)
        Rv = {'front': np.eye(3), 'side': np.array([[0, 0, -1], [0, 1, 0], [1, 0, 0]]), 'top': np.array([[1, 0, 0], [0, 0, -1], [0, 1, 0]]),
              'right': np.array([[0, 0, 1], [0, 1, 0], [-1, 0, 0]])}[a.view]
        T = T @ Rv.T
        gi = ids.index(0x640); muzzle = Rv @ (W[gi] @ np.linalg.inv(B[gi]) @ np.r_[np.array(model['bones'][gi]['position']) + model['muzzles']['points'][a.mesh], 1])[:3]
        if a.zoom == 'hand': c0 = Rv @ W[ids.index(9)][:3, 3]; span = 0.5
        else: c0 = T.reshape(-1, 3).mean(0); span = np.ptp(T.reshape(-1, 3), 0).max() * 1.1
        img = Image.new('RGB', (S, S), (40, 48, 52)); dr = ImageDraw.Draw(img)
        n = np.cross(T[:, 1] - T[:, 0], T[:, 2] - T[:, 0]); n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-12
        sh = 0.35 + 0.65 * np.abs(n[:, 2])
        for i in np.argsort(T[:, :, 2].mean(1)):
            q = T[i] - c0
            dr.polygon([((p[0] / span + .5) * S, (.5 - p[1] / span) * S) for p in q], fill=tuple(int(x) for x in C[i] * sh[i]))
        if round(t) in shots:
            mx, my = (muzzle[0] - c0[0]) / span + .5, .5 - (muzzle[1] - c0[1]) / span
            dr.ellipse([mx * S - 6, my * S - 6, mx * S + 6, my * S + 6], outline=(255, 40, 40), width=3)
        out.paste(img, (k * S, 0)); ImageDraw.Draw(out).text((k * S + 5, S + 4), f'{a.clip} f{t:g}{" SHOT" if round(t) in shots else ""}', fill=(255, 255, 0))
    out.save(a.out)


if __name__ == '__main__':
    main()
