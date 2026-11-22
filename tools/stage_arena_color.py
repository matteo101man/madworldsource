"""Stage a room's arena textures (MW_ROOM, default c201) of the recomp's added color pack for mw-play.

The native frontend replaces textures by the renderer's xxhash64 key
(bake_native_color.names). This resolves each c201.scr TPL image to its key and
copies the pack PNG to assets/native/world/color/texture-NNN.png; images the pack
does not cover keep the source pixels, then writes c201-color.glb from c201.glb.
Added inspection color, not recovered artwork (same authority as the pack).
"""
import os, shutil, struct, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from bake_native_color import EXPLORER, names  # noqa: E402
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries  # noqa: E402

PACK = ROOT / 'native-host-data/color-pack/RZZE8P'
ROOM = os.environ.get('MW_ROOM', 'c201')
raw = (EXPLORER / f'extracted/disc/DATA/files/case{ROOM[1]}/{ROOM}.dat').read_bytes()
e = next(e for e in dat_entries(raw) if e['name'] == f'{ROOM}.scr')
scr = raw[e['offset']:e['offset'] + e['size']]
block = scr[struct.unpack_from('>I', scr, 8)[0]:]
count = struct.unpack_from('>I', block, 4)[0]
world = ROOT / 'assets/native/world' if ROOM == 'c201' else ROOT / 'assets/native/rooms' / ROOM / 'world'
out = world / 'color'
out.mkdir(exist_ok=True)
hit = 0
for i in range(count):
    src = next((PACK / f'{n}.png' for n in names(block, i) if (PACK / f'{n}.png').exists()), None)
    shutil.copyfile(src or world / f'texture-{i:03}.png', out / f'texture-{i:03}.png')
    hit += src is not None
print(f'{hit}/{count} {ROOM} textures from the color pack')

# c201-color.glb: the staged c201.glb with each embedded image (matched to its source
# texture by decoded pixels) replaced by the color texture. Geometry is untouched.
import io, json  # noqa: E402
from PIL import Image  # noqa: E402
glb = (world / f'{ROOM}.glb').read_bytes()
jlen = struct.unpack_from('<I', glb, 12)[0]
doc = json.loads(glb[20:20 + jlen])
bin_ = glb[20 + jlen + 8:]
def key(im):  # the exporter's canvas drops RGB under transparent texels: compare alpha + opaque texels
    import numpy as np
    a = np.asarray(im.convert('RGBA')).copy(); a[a[..., 3] < 255, :3] = 0
    return a.tobytes()
source = {key(Image.open(world / f'texture-{i:03}.png')): i for i in range(count)}
chunks, views, mapped = [], [], 0
for v in doc['bufferViews']:
    views.append(bin_[v.get('byteOffset', 0):v.get('byteOffset', 0) + v['byteLength']])
for img in doc.get('images', []):
    i = source.get(key(Image.open(io.BytesIO(views[img['bufferView']]))))
    if i is None: continue
    views[img['bufferView']] = (out / f'texture-{i:03}.png').read_bytes(); img['mimeType'] = 'image/png'; mapped += 1
body = b''
for v, data in zip(doc['bufferViews'], views):
    v['byteOffset'] = len(body); v['byteLength'] = len(data); body += data + b'\0' * (-len(data) % 4)
doc['buffers'][0]['byteLength'] = len(body)
js = json.dumps(doc, separators=(',', ':')).encode(); js += b' ' * (-len(js) % 4)
total = 12 + 8 + len(js) + 8 + len(body)
(world / f'{ROOM}-color.glb').write_bytes(struct.pack('<III', 0x46546C67, 2, total) + struct.pack('<II', len(js), 0x4E4F534A) + js
                                       + struct.pack('<II', len(body), 0x004E4942) + body)
print(f'{ROOM}-color.glb: {mapped}/{len(doc.get("images", []))} embedded images recolored')
