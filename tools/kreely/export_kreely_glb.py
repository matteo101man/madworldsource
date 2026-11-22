"""Stage Kreely for the native Bevy slice: GLB (source + color variants) and a rig JSON.

python -I tools/kreely/export_kreely_glb.py
Inputs : assets/characters/kreely/build/kreely.model.json + textures (tools/kreely/build_kreely.py)
Outputs: assets/native/kreely-source.glb, assets/native/kreely-color.glb, assets/native/kreely-rig.json,
         research/kreely-staging-manifest.json
Same conventions as the staged Jack/RinRin GLBs (tools/stage_actor.py via the explorer exporter): one node per
bone named role_0_bone<id> under the origin role_0_bone-1, skinned meshes role_0_mesh_N, unlit materials, PNG
textures, inverse binds = -bind translation (bind rotations are zero). Only the visible meshes are exported: the
body and the right-hand pistol (alternate guns stay explorer-only).
"""
import hashlib, json, struct
from pathlib import Path
import numpy as np

PORT = Path(__file__).resolve().parents[2]
BUILD = PORT / 'assets/characters/kreely/build'
OUT = PORT / 'assets/native'
MESHES = ['kreely_body_SB', 'kreely_pistol_R_KB']


def glb(model, textures):
    bones = model['bones']; n = len(bones)
    buf = bytearray(); views, accessors = [], []

    def add(arr, comp, typ, target=None, minmax=False):
        arr = np.ascontiguousarray(arr); off = len(buf); buf.extend(arr.tobytes())
        while len(buf) % 4: buf.append(0)
        views.append(dict(buffer=0, byteOffset=off, byteLength=arr.nbytes, **({'target': target} if target else {})))
        acc = dict(bufferView=len(views) - 1, componentType=comp, count=len(arr), type=typ)
        if minmax: acc.update(min=arr.min(0).tolist(), max=arr.max(0).tolist())
        accessors.append(acc); return len(accessors) - 1

    def add_image(path):
        data = Path(path).read_bytes(); off = len(buf); buf.extend(data)
        while len(buf) % 4: buf.append(0)
        views.append(dict(buffer=0, byteOffset=off, byteLength=len(data))); return len(views) - 1

    # nodes: bones (array order) + origin last; children from parent indices
    nodes = [dict(name=f"role_0_bone{b['id']}", translation=[float(v) for v in b['relative']]) for b in bones]
    nodes.append(dict(name='role_0_bone-1'))
    for i, b in enumerate(bones):
        p = b['parent'] if b['parent'] >= 0 else n
        nodes[p].setdefault('children', []).append(i)
    bindpos = np.array([b['position'] for b in bones] + [[0, 0, 0]], np.float32)
    ibm = np.tile(np.eye(4, dtype=np.float32), (n + 1, 1, 1)); ibm[:, :3, 3] = -bindpos
    ibm_acc = add(ibm.transpose(0, 2, 1).reshape(-1, 16), 5126, 'MAT4')       # glTF matrices are column-major
    images, textures_j, materials, meshes, mesh_nodes = [], [], [], [], []
    for mi, m in enumerate([mm for mm in model['meshes'] if mm['name'] in MESHES]):
        P = np.array(m['positions'], np.float32); N = np.array(m['normals'], np.float32)
        UV = np.array(m['uvs'], np.float32); J = np.array(m['joints'], np.uint16); Wt = np.array(m['weights'], np.float32)
        Wt /= Wt.sum(1, keepdims=True); I = np.array(m['indices'], np.uint32)
        attrs = dict(POSITION=add(P, 5126, 'VEC3', 34962, True), NORMAL=add(N, 5126, 'VEC3', 34962),
                     TEXCOORD_0=add(UV, 5126, 'VEC2', 34962), JOINTS_0=add(J, 5123, 'VEC4', 34962), WEIGHTS_0=add(Wt, 5126, 'VEC4', 34962))
        images.append(dict(bufferView=add_image(textures[m['texture']]), mimeType='image/png'))
        textures_j.append(dict(source=len(images) - 1, sampler=0))
        materials.append(dict(name=m['name'], pbrMetallicRoughness=dict(baseColorTexture=dict(index=len(textures_j) - 1), metallicFactor=0, roughnessFactor=0.9),
                              doubleSided=True, extensions=dict(KHR_materials_unlit={}),
                              extras=dict(renderFlags=m['renderFlags'], threshold=m['inkThreshold'], source='Kreely (user model) / wp0671 pistol')))
        meshes.append(dict(name=m['name'], primitives=[dict(attributes=attrs, indices=add(I, 5125, 'SCALAR', 34963), material=len(materials) - 1)]))
        nodes.append(dict(name=f'role_0_mesh_{mi}', mesh=len(meshes) - 1, skin=0, extras=dict(enabled=True))); mesh_nodes.append(len(nodes) - 1)
    root = len(nodes); nodes.append(dict(name='AuxScene', children=[n] + mesh_nodes))
    j = dict(asset=dict(version='2.0', generator='madworld-rinrin tools/kreely/export_kreely_glb.py'), scene=0,
             scenes=[dict(name='AuxScene', nodes=[root])], nodes=nodes, meshes=meshes, materials=materials,
             textures=textures_j, images=images, samplers=[dict(magFilter=9729, minFilter=9987, wrapS=10497, wrapT=10497)],
             skins=[dict(joints=list(range(n + 1)), inverseBindMatrices=ibm_acc, skeleton=n)],
             accessors=accessors, bufferViews=views, buffers=[dict(byteLength=len(buf))], extensionsUsed=['KHR_materials_unlit'])
    js = json.dumps(j, separators=(',', ':')).encode()
    js += b' ' * (-len(js) % 4)
    return b'glTF' + struct.pack('<II', 2, 12 + 8 + len(js) + 8 + len(buf)) + struct.pack('<I', len(js)) + b'JSON' + js + \
        struct.pack('<I', len(buf)) + b'BIN\0' + bytes(buf)


def main():
    model = json.loads((BUILD / 'kreely.model.json').read_text(encoding='utf-8'))
    out = {}
    for mode, body in (('source', 'kreely_basecolor.png'), ('color', 'kreely_color.png')):
        data = glb(model, {0: BUILD / body, 1: BUILD / 'kreely_tex1.png', 2: BUILD / 'kreely_tex2.png'})
        path = OUT / f'kreely-{mode}.glb'; path.write_bytes(data); out[mode] = dict(path=str(path), sha256=hashlib.sha256(data).hexdigest(), bytes=len(data))
    rig = {k: v for k, v in model.items() if k != 'meshes'}
    rig['meshes'] = MESHES
    (OUT / 'kreely-rig.json').write_text(json.dumps(rig, separators=(',', ':')), encoding='utf-8')
    out['rig'] = dict(path=str(OUT / 'kreely-rig.json'), sha256=hashlib.sha256((OUT / 'kreely-rig.json').read_bytes()).hexdigest())
    out['input'] = dict(model=str(BUILD / 'kreely.model.json'), sha256=hashlib.sha256((BUILD / 'kreely.model.json').read_bytes()).hexdigest())
    out['authority'] = 'User-provided Kreely mesh on pl0030 bone ids; rotation-only retarget data, hand poses, gun overlays and blended transitions as built for the explorer.'
    (PORT / 'research/kreely-staging-manifest.json').write_text(json.dumps(out, indent=2), encoding='utf-8')
    print(json.dumps({k: (v.get('bytes') if isinstance(v, dict) else v) for k, v in out.items() if k != 'authority'}))


if __name__ == '__main__':
    main()
