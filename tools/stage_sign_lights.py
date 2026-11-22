"""Sign light positions for mw-play's lit looks (crates/mw-game/src/play_look.rs): added art, not original.

The c201 neon and lamp signs are painted on a few sheets (SIGNS below) that the stage draws as parts of large atlas
meshes. Each triangle of those materials within RADIUS of the ring is binned on a GRID; a bin becomes one light at the
area-weighted centroid of its triangles, colored by the mean of the bright texels its UVs cover (color pack textures,
the colored arena). Writes assets/native/world/sign-lights.json: [{pos, color, area}], brightest first.
"""
import json, struct
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
ROOT = Path(__file__).resolve().parents[1]
SIGNS = {1, 7, 10, 19, 22, 23, 24, 25, 29}
RING = np.array([3.0, 18.0, -72.0]); RADIUS = 30.0; GRID = 5.0

glb = (ROOT / 'assets/native/world/c201.glb').read_bytes()
jl = struct.unpack_from('<I', glb, 12)[0]; doc = json.loads(glb[20:20 + jl]); body = glb[20 + jl + 8:]
def acc(i):
    a = doc['accessors'][i]; v = doc['bufferViews'][a['bufferView']]
    n = {'VEC2': 2, 'VEC3': 3, 'SCALAR': 1}[a['type']]
    dt = {5126: np.float32, 5123: np.uint16, 5125: np.uint32}[a['componentType']]; size = np.dtype(dt).itemsize
    o = v.get('byteOffset', 0) + a.get('byteOffset', 0); stride = v.get('byteStride', 0) or size * n
    raw = np.frombuffer(body[o:o + stride * a['count']], dtype=np.uint8).reshape(a['count'], stride)[:, :size * n].copy()
    return raw.view(dt).reshape(a['count'], n)

bins = {}
for node in doc['nodes']:
    if 'mesh' not in node: continue
    for prim in doc['meshes'][node['mesh']]['primitives']:
        tex = doc['materials'][prim['material']]['pbrMetallicRoughness'].get('baseColorTexture', {}).get('index')
        if tex not in SIGNS: continue
        image = np.asarray(Image.open(ROOT / f'assets/native/world/color/texture-{tex:03}.png').convert('RGB')).astype(float)
        h, w = image.shape[:2]
        pos = acc(prim['attributes']['POSITION']); uv = acc(prim['attributes']['TEXCOORD_0'])
        for t in acc(prim['indices']).reshape(-1, 3):
            p = pos[t]; c = p.mean(0)
            if np.linalg.norm(c - RING) > RADIUS: continue
            area = np.linalg.norm(np.cross(p[1] - p[0], p[2] - p[0])) / 2
            if area < 1e-4: continue
            u = uv[t].mean(0) % 1.0
            px = image[int(u[1] * (h - 1)), int(u[0] * (w - 1))]
            lum = px.mean()
            key = tuple((c // GRID).astype(int))
            b = bins.setdefault(key, [np.zeros(3), 0.0, np.zeros(3), 0.0])
            b[0] += c * area; b[1] += area
            if lum > 140: b[2] += px * area; b[3] += area   # bright (lit) texels set the color
lights = []
for b in bins.values():
    if b[3] < 0.05: continue
    color = b[2] / b[3] / 255
    lights.append(dict(pos=[round(float(x), 2) for x in b[0] / b[1]], color=[round(float(x), 3) for x in color], area=round(float(b[3]), 2)))
lights.sort(key=lambda l: -l['area'])
(ROOT / 'assets/native/world/sign-lights.json').write_text(json.dumps(lights, indent=1))
print(len(lights), 'sign lights'); [print(l) for l in lights[:12]]

# Emission masks: the sheets' beige paper must not glow, only the lettering and lamps - saturated texels and near-white
# ones. assets/native/world/emissive/texture-NNN.png (color pack colors, black elsewhere).
out = ROOT / 'assets/native/world/emissive'; out.mkdir(exist_ok=True)
for tex in sorted(SIGNS):
    a = np.asarray(Image.open(ROOT / f'assets/native/world/color/texture-{tex:03}.png').convert('RGB')).astype(float)
    mx, mn = a.max(-1), a.min(-1); sat = (mx - mn) / np.maximum(mx, 1)
    keep = ((sat > 0.5) & (mx > 150)) | (mn > 245)
    Image.fromarray((a * keep[..., None]).astype(np.uint8)).save(out / f'texture-{tex:03}.png')
    print(f'texture-{tex:03}: {keep.mean():.0%} emissive')
