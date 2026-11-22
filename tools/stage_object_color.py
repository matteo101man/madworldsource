"""The added color look for a staged ScrObj: python -I tools/stage_object_color.py bl/bl005c.dat

As tools/stage_arena_color.py does for a room: each of the object's TPL images is looked up in the recomp's added color
pack by the renderer's xxhash64 key (bake_native_color.names), and assets/native/objects/<stem>-color.glb is written from
<stem>.glb (tools/stage_static_object.py) with the covered images replaced; the rest keep their source pixels. The
c303 pillars (bl005c) and pews (bl005d) use it so they match the colored room. Added inspection color, not recovered
artwork (the pack's authority).
"""
import io, json, struct, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from bake_native_color import EXPLORER, names  # noqa: E402
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries  # noqa: E402

PACK = ROOT / 'native-host-data/color-pack/RZZE8P'


def main():
    dat = sys.argv[1]
    stem = Path(dat).stem
    raw = (EXPLORER / 'extracted/disc/DATA/files' / dat).read_bytes()
    e = next(e for e in dat_entries(raw) if e['name'] == f'{stem}.tpl')
    tplb = raw[e['offset']:e['offset'] + e['size']]
    count = struct.unpack_from('>I', tplb, 4)[0]
    found = [next((PACK / f'{n}.png' for n in names(tplb, i) if (PACK / f'{n}.png').exists()), None) for i in range(count)]
    src = ROOT / f'assets/native/objects/{stem}.glb'
    glb = src.read_bytes()
    jlen = struct.unpack_from('<I', glb, 12)[0]
    doc = json.loads(glb[20:20 + jlen])
    body = bytearray(glb[20 + jlen + 8:])
    # (stage_static_object writes the images in TPL page order, each its own buffer view: rebuild the binary chunk with
    # the covered pages swapped)
    views = doc['bufferViews']
    swap = {}
    for k, im in enumerate(doc['images']):
        if k < len(found) and found[k] is not None:
            buf = io.BytesIO()
            from PIL import Image
            Image.open(found[k]).convert('RGBA').save(buf, 'PNG')
            swap[im['bufferView']] = buf.getvalue()
    out = bytearray()
    for i, v in enumerate(views):
        data = swap.get(i, bytes(body[v.get('byteOffset', 0):v.get('byteOffset', 0) + v['byteLength']]))
        while len(out) % 4: out.append(0)
        v['byteOffset'] = len(out); v['byteLength'] = len(data)
        out.extend(data)
    while len(out) % 4: out.append(0)
    doc['buffers'] = [dict(byteLength=len(out))]
    js = json.dumps(doc, separators=(',', ':')).encode(); js += b' ' * (-len(js) % 4)
    dst = ROOT / f'assets/native/objects/{stem}-color.glb'
    dst.write_bytes(struct.pack('<III', 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(out)) + struct.pack('<II', len(js), 0x4E4F534A) + js
                    + struct.pack('<II', len(out), 0x004E4942) + bytes(out))
    print(dst, f'{sum(f is not None for f in found)}/{count} images from the color pack')


if __name__ == '__main__':
    main()
