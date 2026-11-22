"""Colored characters for mw-play from the recomp's added color pack (native-host-data/color-pack).

The recomp frontend recolors by texture: every TPL page is replaced by the pack PNG named by the renderer's xxhash64
key (bake_native_color.names). This hashes each page of the character's TPL, and writes <model>-pack.glb: the source
GLB with each embedded image (matched to its TPL page by decoded pixels) replaced by the pack PNG. Geometry, skins
and the _AB ink pages are unchanged. Added inspection color, as the pack.

RinRin touch-ups (added art, not the pack): every GLB material owns a copy of its atlas, so single meshes can be
repainted without touching the rest. head_KB (back of the head) samples a wine-red dress patch of the clothes atlas;
its copy is recolored to the hair's navy, keeping the stroke detail. The face's eye (one almond, mirrored for both
eyes) is a solid dark shape in the pack; its copy gets an off-white eye with a brown iris, pupil and catch-light
under the dark lash line.

Kung-fu grunt touch-up (added art, not the pack): the pack paints em0101's bare torso and arms the same violet as his
trousers (the lit looks push it to magenta); per mesh the violet texels (hue 240-345 deg) take that part's color at their
own shading (recolor_kungfu). Ink and trim keep.
"""
import io, json, struct, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from bake_native_color import EXPLORER, names  # noqa: E402
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries, tpl  # noqa: E402

PACK = ROOT / 'native-host-data/color-pack/RZZE8P'
MODELS = [('jack-player', 'pl/pl0030.dat', 'pl0030'), ('rinrin', 'em/em0440.dat', 'em0440'), ('kungfu', 'em/em0101.dat', 'em0101')]


def key(im):  # exporters drop RGB under transparent texels: compare alpha + opaque texels
    a = np.asarray(im.convert('RGBA')).copy(); a[a[..., 3] < 255, :3] = 0
    return a.tobytes()


def accessor(doc, body, i):
    a = doc['accessors'][i]; v = doc['bufferViews'][a['bufferView']]
    n = {'VEC2': 2, 'VEC3': 3, 'SCALAR': 1}[a['type']]
    dt = {5126: np.float32, 5123: np.uint16, 5125: np.uint32, 5121: np.uint8}[a['componentType']]; size = np.dtype(dt).itemsize
    o = v.get('byteOffset', 0) + a.get('byteOffset', 0); stride = v.get('byteStride', 0) or size * n
    raw = np.frombuffer(body[o:o + stride * a['count']], dtype=np.uint8).reshape(a['count'], stride)[:, :size * n].copy()
    return raw.view(dt).reshape(a['count'], n)


def touch_up(doc, views, body):
    names = [m['name'] for m in json.loads((ROOT / 'assets/native/ink/ink.json').read_text())['models']['rinrin']['meshes']]
    def image(mesh):   # (bufferView index, PIL image) of the mesh's own atlas copy (GLB mesh i uses material i)
        prim = doc['meshes'][mesh]['primitives'][0]
        tex = doc['materials'][prim['material']]['pbrMetallicRoughness']['baseColorTexture']['index']
        view = doc['images'][doc['textures'][tex]['source']]['bufferView']
        return prim, view, Image.open(io.BytesIO(views[view])).convert('RGBA')
    def save(view, im):
        buf = io.BytesIO(); im.save(buf, 'PNG'); views[view] = buf.getvalue()
    # back of the head: the UV footprint (dilated over the seams) takes the hair's navy at the patch's own shading
    prim, view, im = image(names.index('head_KB'))
    uv = accessor(doc, body, prim['attributes']['TEXCOORD_0']); tris = accessor(doc, body, prim['indices']).reshape(-1, 3)
    mask = Image.new('L', im.size, 0); draw = ImageDraw.Draw(mask)
    for t in tris: draw.polygon([(uv[k][0] * im.width, uv[k][1] * im.height) for k in t], fill=255)
    m = np.asarray(mask.filter(ImageFilter.MaxFilter(7))) > 0
    a = np.asarray(im).astype(float); lum = a[..., :3].mean(-1)
    shade = (lum / max(lum[m].mean(), 1.)).clip(0.4, 1.8)
    hair = np.array([15., 20., 28.])   # hair_KB / hair_front_KB mean in the pack
    a[m, :3] = (hair[None, :] * shade[m][:, None]).clip(0, 255)
    save(view, Image.fromarray(a.astype(np.uint8)))
    # eye: the dark almond near texels (80..166, 215..250) of the face page
    prim, view, im = image(names.index('face_SB'))
    a = np.asarray(im).astype(float); h, w = a.shape[:2]
    x0, x1, y0, y1 = int(78 / 512 * w), int(167 / 512 * w), int(214 / 512 * h), int(252 / 512 * h)
    box = a[y0:y1, x0:x1]; dark = box[..., :3].mean(-1) < 90
    ys, xs = np.nonzero(dark)
    for x in np.unique(xs):
        col = ys[xs == x]; top, bot = col.min(), col.max(); span = bot - top + 1
        if span < 4: continue
        for y in range(top + max(2, int(span * 0.32)), bot):   # keep the upper lash line and the lower edge dark
            box[y, x, :3] = (236, 228, 218)
    cx, cy = (xs.min() + xs.max()) / 2 + 2, (ys.min() + ys.max()) / 2 + 3
    r = (ys.max() - ys.min()) * 0.42
    yy, xx = np.mgrid[0:box.shape[0], 0:box.shape[1]]; d = np.hypot(xx - cx, yy - cy)
    inner = (box[..., :3].mean(-1) > 200) & dark
    iris = inner & (d < r); box[iris, :3] = (128, 76, 38)
    box[iris & (d > r * 0.72), :3] = (78, 44, 22)
    box[inner & (d < r * 0.38), :3] = (22, 14, 10)
    box[inner & (np.hypot(xx - (cx - r * 0.35), yy - (cy - r * 0.35)) < r * 0.2), :3] = (250, 248, 244)
    a[y0:y1, x0:x1] = box
    save(view, Image.fromarray(a.astype(np.uint8)))
    print('rinrin touch-ups: head_KB recolored, face eye painted')


# em0101 is a bare-chested brute; the pack paints his torso, arms and trousers the same violet. Per MDB mesh (each GLB
# material owns its atlas copy, mesh i -> material i) the violet texels become: skin on the body (the pack's own skin
# tone), black trousers, near-black pigtail, a straw hat, a tan gourd and cord, steel claws, and flesh red on the cut
# cross-sections. Shading follows the violet's own value (its bright parts sit at 0.54).
SKIN = (211., 154., 114.)
KUNGFU_PARTS = {0: SKIN, 1: SKIN, 2: SKIN, 4: SKIN, 5: SKIN, 3: (44., 42., 48.), 6: (30., 28., 32.), 7: (30., 28., 32.), 8: (30., 28., 32.),
                9: (30., 28., 32.), 10: (30., 28., 32.), 11: (30., 28., 32.), 12: (30., 28., 32.), 13: (150., 26., 24.), 14: (150., 26., 24.),
                15: (150., 26., 24.), 16: (196., 166., 104.), 17: (196., 166., 104.), 18: (172., 118., 58.), 19: (120., 82., 46.), 20: (156., 158., 166.)}


def recolor_kungfu(doc, views):
    """The pack's violet -> each mesh's own color, in that mesh's atlas copy."""
    done = set()
    for i, mesh in enumerate(doc['meshes']):
        mat = mesh['primitives'][0].get('material')
        if mat is None or i not in KUNGFU_PARTS: continue
        tex = doc['materials'][mat]['pbrMetallicRoughness']['baseColorTexture']['index']
        view = doc['images'][doc['textures'][tex]['source']]['bufferView']
        if view in done: continue
        done.add(view)
        im = Image.open(io.BytesIO(views[view])).convert('RGBA')
        hsv = np.asarray(im.convert('RGB').convert('HSV')).astype(float)
        a = np.asarray(im).astype(float)
        hue = hsv[..., 0] * 360 / 255; sat = hsv[..., 1] / 255; val = hsv[..., 2] / 255
        m = (hue > 240) & (hue < 345) & (sat > 0.1)
        shade = np.clip(0.45 + val[m] / 0.54 * 0.55, 0.45, 1.05)
        a[m, :3] = (np.array(KUNGFU_PARTS[i])[None, :] * shade[:, None]).clip(0, 255)
        buf = io.BytesIO(); Image.fromarray(a.astype(np.uint8)).save(buf, 'PNG'); views[view] = buf.getvalue()
    print(f'kungfu touch-up: {len(done)} atlas copies, violet -> skin / black trousers / hat, gourd, claws')


for model, dat, stem in MODELS:
    raw = (EXPLORER / 'extracted/disc/DATA/files' / dat).read_bytes()
    es = {e['name']: e for e in dat_entries(raw)}
    block = raw[es[f'{stem}.tpl']['offset']:es[f'{stem}.tpl']['offset'] + es[f'{stem}.tpl']['size']]
    pages = tpl(block)
    pack = {}
    for i, (im, _) in enumerate(pages):
        hit = next((PACK / f'{n}.png' for n in names(block, i) if (PACK / f'{n}.png').exists()), None)
        if hit: pack[key(im)] = hit.read_bytes()
    glb = (ROOT / f'assets/native/{model}-source.glb').read_bytes()
    jl = struct.unpack_from('<I', glb, 12)[0]
    doc = json.loads(glb[20:20 + jl]); body = glb[20 + jl + 8:]
    views = [body[v.get('byteOffset', 0):v.get('byteOffset', 0) + v['byteLength']] for v in doc['bufferViews']]
    mapped = 0
    for img in doc.get('images', []):
        new = pack.get(key(Image.open(io.BytesIO(views[img['bufferView']]))))
        if new is None: continue
        views[img['bufferView']] = new; img['mimeType'] = 'image/png'; mapped += 1
    if model == 'rinrin': touch_up(doc, views, body)
    if model == 'kungfu': recolor_kungfu(doc, views)
    out = b''
    for v, data in zip(doc['bufferViews'], views):
        v['byteOffset'] = len(out); v['byteLength'] = len(data); out += data + b'\0' * (-len(data) % 4)
    doc['buffers'][0]['byteLength'] = len(out)
    js = json.dumps(doc, separators=(',', ':')).encode(); js += b' ' * (-len(js) % 4)
    (ROOT / f'assets/native/{model}-pack.glb').write_bytes(struct.pack('<III', 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(out))
        + struct.pack('<II', len(js), 0x4E4F534A) + js + struct.pack('<II', len(out), 0x004E4942) + out)
    print(f'{model}-pack.glb: {mapped}/{len(doc.get("images", []))} images from the pack ({len(pack)}/{len(pages)} pages)')
