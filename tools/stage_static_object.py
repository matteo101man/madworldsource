"""Stage a static (boneless) ScrObj model as a GLB for mw-play: python -I tools/stage_static_object.py bl/bl0015.dat

The explorer's Node exporter needs three.js; for a model with no bones this writes the GLB directly from the explorer's
MDB / TPL readers: every mesh is one primitive (positions, normals, UVs, triangle-list indices) with its TPL page as
the base colour texture, unlit as the Wii draws the stage (texture x vertex colour; these are white). Output:
assets/native/objects/<stem>.glb. Used for the c303 coffins (bl0015, research/ELISE-BOSS.md §5.4).
Breakable ScrObjs (cBaseScrObj, mesh-name parser 801B97B0): a rigid model whose bones all sit at the origin (bl005c, the
c303 pillars) or none (bl005d, the pews) is written with one named node per break part, from the mesh name's `_Dn_Mnn`
token (D0_M00 intact, D9_M00 the remains, D9_M01.. the debris), the empty 4-vertex placeholders left out, and each
debris part (D9_M01 on) centred on its own centroid with the node placed there, so the game can throw it.
"""
import io, json, struct, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries, mdb, tpl  # noqa: E402


def main():
    dat = sys.argv[1]
    stem = Path(dat).stem
    raw = (EXPLORER / 'extracted/disc/DATA/files' / dat).read_bytes()
    es = {e['name']: e for e in dat_entries(raw)}
    member = lambda n: raw[es[n]['offset']:es[n]['offset'] + es[n]['size']]
    model = mdb(member(f'{stem}.mdb'))
    import re
    rigid = all(abs(c) < 1e-4 for b in model['bones'] for c in (b.get('position') or b.get('translation') or [0, 0, 0])) if model['bones'] else True
    assert rigid or not model['bones'], 'skinned model: use tools/stage_actor.py'
    part = lambda name: (lambda m: f'D{m.group(1)}_M{m.group(2)}' if m else None)(re.search(r'_D(\d)_M(\d\d)', name))
    pages = tpl(member(f'{stem}.tpl'))
    body, views, accessors, meshes = bytearray(), [], [], []

    def add(data, target=None):
        while len(body) % 4: body.append(0)
        views.append(dict(buffer=0, byteOffset=len(body), byteLength=len(data), **({'target': target} if target else {})))
        body.extend(data)
        return len(views) - 1

    def acc(values, kind, comp, fmt, target):
        flat = [x for v in values for x in (v if isinstance(v, (list, tuple)) else [v])]
        view = add(struct.pack(f'<{len(flat)}{fmt}', *flat), target)
        a = dict(bufferView=view, componentType=comp, count=len(values), type=kind)
        if kind == 'VEC3':
            a['min'] = [min(v[i] for v in values) for i in range(3)]; a['max'] = [max(v[i] for v in values) for i in range(3)]
        accessors.append(a)
        return len(accessors) - 1

    images = []
    for im, _ in pages:
        buf = io.BytesIO(); im.convert('RGBA').save(buf, 'PNG')
        images.append(dict(bufferView=add(buf.getvalue()), mimeType='image/png'))
    materials = [dict(pbrMetallicRoughness=dict(baseColorTexture=dict(index=i), metallicFactor=0., roughnessFactor=1.), alphaMode='MASK', doubleSided=True,
                      extensions={'KHR_materials_unlit': {}}) for i in range(len(images))]
    groups = {}   # node name -> mesh indices (in order)
    for m in model['meshes']:
        key = part(m['name'])
        if key and len(m['positions']) <= 4: continue   # the empty placeholders (D0_M01..M05)
        groups.setdefault(key or m['name'], []).append(len(meshes))
        prim = dict(attributes=dict(POSITION=acc([list(map(float, p)) for p in m['positions']], 'VEC3', 5126, 'f', 34962),
                                    NORMAL=acc([list(map(float, n)) for n in m['normals']], 'VEC3', 5126, 'f', 34962),
                                    TEXCOORD_0=acc([list(map(float, t)) for t in m['uvs']], 'VEC2', 5126, 'f', 34962)),
                    indices=acc(list(m['indices']), 'SCALAR', 5125, 'I', 34963), material=m['texture'])
        meshes.append(dict(name=m['name'], primitives=[prim], _pos=[list(map(float, p)) for p in m['positions']]))
    # one node per break part (its meshes as one multi-primitive mesh); debris parts centred on their centroid
    nodes, gmeshes = [], []
    for name, idx in groups.items():
        centre = [0., 0., 0.]
        if re.fullmatch(r'D9_M0[1-9]', name):
            pts = [p for i in idx for p in meshes[i]['_pos']]
            centre = [sum(p[k] for p in pts) / len(pts) for k in range(3)]
            for i in idx:
                a = accessors[meshes[i]['primitives'][0]['attributes']['POSITION']]
                moved = [[p[k] - centre[k] for k in range(3)] for p in meshes[i]['_pos']]
                meshes[i]['primitives'][0]['attributes']['POSITION'] = acc(moved, 'VEC3', 5126, 'f', 34962)
        gmeshes.append(dict(name=name, primitives=[meshes[i]['primitives'][0] for i in idx]))
        nodes.append(dict(name=name, mesh=len(gmeshes) - 1, **({'translation': centre} if any(centre) else {})))
    meshes = gmeshes
    doc = dict(asset=dict(version='2.0', generator='tools/stage_static_object.py'), scene=0, scenes=[dict(nodes=list(range(len(nodes))))],
               nodes=nodes, meshes=meshes, materials=materials, accessors=accessors,
               bufferViews=views, buffers=[dict(byteLength=len(body))], images=images,
               textures=[dict(source=i) for i in range(len(images))], extensionsUsed=['KHR_materials_unlit'])
    js = json.dumps(doc, separators=(',', ':')).encode(); js += b' ' * (-len(js) % 4)
    while len(body) % 4: body.append(0)
    out = ROOT / f'assets/native/objects/{stem}.glb'
    out.write_bytes(struct.pack('<III', 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(body)) + struct.pack('<II', len(js), 0x4E4F534A) + js
                    + struct.pack('<II', len(body), 0x004E4942) + bytes(body))
    print(out, len(meshes), 'meshes', len(images), 'textures')


if __name__ == '__main__':
    main()
