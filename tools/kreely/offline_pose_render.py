"""Offline render (numpy poser, same math as the explorer) of skinned Kreely. python -I tools/kreely/offline_pose_render.py <out.png> <handpose> [clip frame region]"""
# render skinned Kreely (offline poser) for a hand pose / clip frame, cropped to a region; args: out pose [clipname frame] [region]
import sys, json, numpy as np
sys.path.insert(0, r'D:\Games\gamesfordecomp\ports\madworld-rinrin\tools\kreely'); sys.path.insert(0, r'D:\Games\gamesfordecomp\exports\madworld\builders')
import kreely_pose as kp
from PIL import Image, ImageDraw
import os
model = json.load(open(os.environ.get('KREELY_MODEL', r'D:\Games\gamesfordecomp\ports\madworld-rinrin\assets\characters\kreely\build\kreely.model.json')))
mesh = model['meshes'][0]
out, pose = sys.argv[1], sys.argv[2]
clip = None; t = 0
if len(sys.argv) > 4 and sys.argv[3] != '-':
    from native import motion
    c = json.load(open(r'D:\Games\gamesfordecomp\exports\madworld\catalog.json', encoding='utf-8'))
    it = next(x for x in c['clips'] if x['bank'] == 'pl/pl0030.dat' and x['name'] == sys.argv[3] + '.mot')
    b = next(x for x in c['banks'] if x['id'] == it['bank'])
    with open(b['path'], 'rb') as f: f.seek(it['offset']); clip = motion(f.read(it['size']))
    t = float(sys.argv[4])
region = sys.argv[5] if len(sys.argv) > 5 else 'hand'
W = kp.worlds(model, clip, t, pose); V = kp.skin(model, mesh, W)
I = np.array(mesh['indices']).reshape(-1, 3)
uv = np.array(mesh['uvs']); tex = np.asarray(Image.open(r'D:\Games\gamesfordecomp\ports\madworld-rinrin\assets\characters\kreely\build\kreely_color.png').convert('RGB'))
h, w, _ = tex.shape; col = tex[(np.clip(uv[:, 1], 0, 1) * (h - 1)).astype(int), (np.clip(uv[:, 0], 0, 1) * (w - 1)).astype(int)].astype(float)
idx = {b['id']: i for i, b in enumerate(model['bones'])}
if region == 'hand': c = W[idx[14]][:3, 3] + [0.08, 0, 0]; span = 0.32
elif region == 'handR': c = W[idx[9]][:3, 3] - [0.08, 0, 0]; span = 0.32
elif region == 'head': c = W[idx[4]][:3, 3] + [0, 0.05, 0]; span = 0.6
else: c = V.mean(0); span = np.ptp(V, 0).max() * 1.1
views = [('top', np.array([[1, 0, 0], [0, 0, -1]]), np.array([0, 1, 0])), ('front', np.array([[1, 0, 0], [0, 1, 0]]), np.array([0, 0, 1])),
         ('bottom', np.array([[1, 0, 0], [0, 0, 1]]), np.array([0, -1, 0]))]
S = 500; panels = []
for name, A, d in views:
    img = Image.new('RGB', (S, S), (40, 48, 52)); dr = ImageDraw.Draw(img)
    T = V[I]; n = np.cross(T[:, 1] - T[:, 0], T[:, 2] - T[:, 0]); n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-12
    depth = (T @ d).mean(1); sh = 0.35 + 0.65 * np.abs(n @ d)
    q = (T - c) @ A.T
    keep = (np.abs(q) < span / 2).all(2).any(1)
    for k in np.argsort(depth):
        if not keep[k]: continue
        pts = [((q[k, m, 0] / span + 0.5) * S, (0.5 - q[k, m, 1] / span) * S) for m in range(3)]
        cc = col[I[k]].mean(0) * sh[k]; dr.polygon(pts, fill=tuple(int(x) for x in cc))
    dr.text((5, 5), f'{name} {pose} {sys.argv[3] if len(sys.argv) > 3 else ""} {t}', fill=(255, 255, 0))
    panels.append(img)
o = Image.new('RGB', (S * 3, S)); [o.paste(p, (i * S, 0)) for i, p in enumerate(panels)]; o.save(out)
