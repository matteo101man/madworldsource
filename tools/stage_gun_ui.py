"""Stage the HUD art for Kreely's pistol (crates/mw-game/src/play_gun_hud.rs): added use of native artwork.

- Sights and the RELOAD! banner are cIdTargetSight's UV group 21 (program 19), sprites 165-168: absent from the US
  core_us.idd the port stages, present in the archived core page (research/ui-core-layout.json rects on
  research/ui-source/core-us-0.png). Cut out unchanged: sight-ring (165), sight-lock (166), sight-none (167), reload (168).
- pistol.png / pistol-color.png: a side view of Kreely's wp0671 pistol (kreely_pistol_R_KB in kreely-color.glb), flat
  texture color per triangle with a 2 px ink outline, for the weapon window's picture slot (83x30 in program 37) and the
  counter's icon. Black/white version from kreely-source.glb.
Writes assets/native/ui/gun/.
"""
import io, json, struct
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/native/ui/gun'; OUT.mkdir(parents=True, exist_ok=True)

page = Image.open(ROOT / 'research/ui-source/core-us-0.png').convert('RGBA')
for name, rect in {'sight-ring': (721, 703, 753, 735), 'sight-lock': (41, 559, 73, 591), 'sight-none': (41, 601, 73, 633),
                   'reload': (536, 511, 646, 543)}.items():
    page.crop(rect).save(OUT / f'{name}.png')


def pistol(glb_path, out, size=(166, 60)):
    glb = glb_path.read_bytes(); jl = struct.unpack_from('<I', glb, 12)[0]; doc = json.loads(glb[20:20 + jl]); body = glb[20 + jl + 8:]
    def acc(i):
        a = doc['accessors'][i]; v = doc['bufferViews'][a['bufferView']]; n = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3}[a['type']]
        dt = {5126: np.float32, 5123: np.uint16, 5125: np.uint32}[a['componentType']]; sz = np.dtype(dt).itemsize
        o = v.get('byteOffset', 0) + a.get('byteOffset', 0); st = v.get('byteStride', 0) or sz * n
        raw = np.frombuffer(body[o:o + st * a['count']], dtype=np.uint8).reshape(a['count'], st)[:, :sz * n].copy()
        return raw.view(dt).reshape(a['count'], n)
    mesh = next(m for m in doc['meshes'] if m.get('name') == 'kreely_pistol_R_KB')
    prim = mesh['primitives'][0]
    pos = acc(prim['attributes']['POSITION']); uv = acc(prim['attributes']['TEXCOORD_0']); tri = acc(prim['indices']).reshape(-1, 3)
    tex = doc['materials'][prim['material']]['pbrMetallicRoughness']['baseColorTexture']['index']
    v = doc['bufferViews'][doc['images'][doc['textures'][tex]['source']]['bufferView']]
    image = np.asarray(Image.open(io.BytesIO(body[v.get('byteOffset', 0):v.get('byteOffset', 0) + v['byteLength']])).convert('RGB')).astype(float)
    # side view: the longest extent is the barrel (horizontal), the second the grip (vertical), the third is depth
    c = pos - pos.mean(0); axes = np.linalg.svd(c, full_matrices=False)[2]
    p = c @ axes.T
    if p[:, 0].max() < -p[:, 0].min(): p[:, 0] *= -1          # muzzle to the right
    if p[:, 1].max() > -p[:, 1].min(): p[:, 1] *= -1           # the grip is the long excursion from the barrel: down
    W, H = size; pad = 6
    scale = min((W - 2 * pad) / np.ptp(p[:, 0]), (H - 2 * pad) / np.ptp(p[:, 1]))
    xy = np.c_[(p[:, 0] - p[:, 0].min()) * scale + pad, H - ((p[:, 1] - p[:, 1].min()) * scale + pad)]
    canvas = Image.new('RGBA', size, (0, 0, 0, 0)); draw = ImageDraw.Draw(canvas)
    h, w = image.shape[:2]
    for t in sorted(tri, key=lambda t: p[t, 2].mean()):      # far to near
        u = uv[t].mean(0) % 1.
        col = image[int(u[1] * (h - 1)), int(u[0] * (w - 1))]
        draw.polygon([tuple(xy[k]) for k in t], fill=tuple(int(x) for x in col) + (255,))
    alpha = canvas.getchannel('A')
    ring = alpha.filter(ImageFilter.MaxFilter(5))
    out_img = Image.new('RGBA', size, (0, 0, 0, 0)); out_img.paste((10, 8, 12, 255), mask=ring); out_img.alpha_composite(canvas)
    out_img.save(OUT / out)


pistol(ROOT / 'assets/native/kreely-color.glb', 'pistol-color.png')
pistol(ROOT / 'assets/native/kreely-source.glb', 'pistol.png')
print(sorted(x.name for x in OUT.iterdir()))
