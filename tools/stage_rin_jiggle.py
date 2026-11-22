"""Add butt jiggle bones to RinRin's GLBs for mw-play's spring bones (crates/mw-game/src/play_springs.rs): added, not
original. The em0440 rig has breast bones (25, 26) but nothing for the buttocks, whose skin rides on the hips (15).

Two nodes role_0_bone200 (right, -x) and role_0_bone201 (left, +x) are added as children of bone 15 at the centroid of
each cheek (body vertices behind the hips, weighted to 15, between the waist and the thighs), axes aligned with the
hips. Every skin gets the two joints (inverse bind = the new bind pose). In hada_SB, huku_KB and obi_SB a smooth falloff
around each cheek moves up to MAX of a vertex's bone-15 weight onto its cheek bone (taking a free influence slot, or
folding the smallest one into bone 15 first). Idempotent: a GLB that already has role_0_bone200 is left alone.

Scarf (hagoromo): the band runs along both arms and dips in a loop behind the back, skinned only to the arm and spine
bones. Two 3-bone chains, role_0_bone210..212 under the right clavicle (5) and 213..215 under the left (10), run from
the shoulder blades down to the middle of the loop; the back-of-loop vertices (behind the spine, |x| < 0.27) blend onto
them (tent weights along the chain, full share inside |x| < 0.20), so the loop can swing. Idempotent on role_0_bone210.
"""
import json, struct
from pathlib import Path
import numpy as np
ROOT = Path(__file__).resolve().parents[1]
MESHES = ('hada_SB', 'huku_KB', 'obi_SB')
RADIUS = 0.14; MAX = 0.75

names = [m['name'] for m in json.loads((ROOT / 'assets/native/ink/ink.json').read_text())['models']['rinrin']['meshes']]


def load(path):
    glb = path.read_bytes(); jl = struct.unpack_from('<I', glb, 12)[0]
    return json.loads(glb[20:20 + jl]), bytearray(glb[20 + jl + 8:])


def save(path, doc, body):
    body += b'\0' * (-len(body) % 4); doc['buffers'][0]['byteLength'] = len(body)
    js = json.dumps(doc, separators=(',', ':')).encode(); js += b' ' * (-len(js) % 4)
    path.write_bytes(struct.pack('<III', 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(body))
                     + struct.pack('<II', len(js), 0x4E4F534A) + js + struct.pack('<II', len(body), 0x004E4942) + bytes(body))


def read(doc, body, i):
    a = doc['accessors'][i]; v = doc['bufferViews'][a['bufferView']]
    n = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[a['type']]
    dt = {5126: np.float32, 5123: np.uint16, 5125: np.uint32, 5121: np.uint8}[a['componentType']]; size = np.dtype(dt).itemsize
    o = v.get('byteOffset', 0) + a.get('byteOffset', 0); stride = v.get('byteStride', 0) or size * n
    raw = np.frombuffer(bytes(body[o:o + stride * a['count']]), dtype=np.uint8).reshape(a['count'], stride)[:, :size * n].copy()
    return raw.view(dt).reshape(a['count'], n)


def append(doc, body, array, like=None, type_=None):
    """Append an array as a new bufferView + accessor; returns the accessor index."""
    body += b'\0' * (-len(body) % 4)
    data = np.ascontiguousarray(array).tobytes()
    doc['bufferViews'].append(dict(buffer=0, byteOffset=len(body), byteLength=len(data))); body += data
    acc = dict(bufferView=len(doc['bufferViews']) - 1, count=len(array),
               componentType={np.dtype(np.float32): 5126, np.dtype(np.uint16): 5123, np.dtype(np.uint8): 5121}[array.dtype],
               type=type_ or {1: 'SCALAR', 2: 'VEC2', 3: 'VEC3', 4: 'VEC4', 16: 'MAT4'}[array.shape[1]])
    if like is not None and doc['accessors'][like].get('normalized'): acc['normalized'] = True
    doc['accessors'].append(acc)
    return len(doc['accessors']) - 1


def stage(path):
    doc, body = load(path)
    node_names = [n.get('name') for n in doc['nodes']]
    if 'role_0_bone200' in node_names: print(path.name, 'already has the jiggle bones'); return
    hips = node_names.index('role_0_bone15')
    skin0 = doc['skins'][0]
    ibm0 = read(doc, body, skin0['inverseBindMatrices']).reshape(-1, 4, 4).transpose(0, 2, 1)   # column-major -> rows
    hips_bind = np.linalg.inv(ibm0[skin0['joints'].index(hips)])
    # cheek centroids from the body skin: behind the hips, weighted to 15, between waist and thighs
    mesh_nodes = {n['mesh']: i for i, n in enumerate(doc['nodes']) if 'mesh' in n}
    body_prim = doc['meshes'][names.index('hada_SB')]['primitives'][0]
    pos = read(doc, body, body_prim['attributes']['POSITION'])
    joints = read(doc, body, body_prim['attributes']['JOINTS_0']).astype(int)
    weights = read(doc, body, body_prim['attributes']['WEIGHTS_0']).astype(float)
    if weights.max() > 1.5: weights /= weights.sum(1, keepdims=True)
    skin = doc['skins'][doc['nodes'][mesh_nodes[names.index('hada_SB')]]['skin']]
    w15 = (weights * (np.array(skin['joints'])[joints] == hips)).sum(1)
    back = (pos[:, 2] < hips_bind[2, 3] - 0.03) & (pos[:, 1] > hips_bind[1, 3] - 0.32) & (pos[:, 1] < hips_bind[1, 3] - 0.04) & (w15 > 0.5)
    cheeks = []
    for side in (-1, 1):
        sel = back & (pos[:, 0] * side > 0.01)
        c = pos[sel].mean(0); c[2] = pos[sel][:, 2].min() + 0.06   # under the skin surface
        cheeks.append(c)
    print(path.name, 'cheeks', [c.round(3) for c in cheeks], 'from', int(back.sum()), 'vertices')
    # nodes: local = inv(hips_bind) * (hips rotation, cheek translation)
    new = []
    for side, c in zip(('200', '201'), cheeks):
        world = hips_bind.copy(); world[:3, 3] = c
        local = np.linalg.inv(hips_bind) @ world
        doc['nodes'].append(dict(name=f'role_0_bone{side}', translation=[float(x) for x in local[:3, 3]]))
        new.append((len(doc['nodes']) - 1, world))
    doc['nodes'][hips].setdefault('children', []).extend(i for i, _ in new)
    # skins: two more joints with their inverse binds (one new accessor per distinct original)
    made = {}
    for s in doc['skins']:
        key = s['inverseBindMatrices']
        if key not in made:
            ibm = read(doc, body, key).reshape(-1, 16)
            extra = np.stack([np.linalg.inv(w).T.reshape(16) for _, w in new]).astype(np.float32)
            made[key] = append(doc, body, np.vstack([ibm, extra]).astype(np.float32), type_='MAT4')
        s['inverseBindMatrices'] = made[key]; s['joints'] = s['joints'] + [i for i, _ in new]
    # weights: falloff around each cheek moves part of the bone-15 share onto the cheek bone
    for name in MESHES:
        mesh = names.index(name)
        skin = doc['skins'][doc['nodes'][mesh_nodes[mesh]]['skin']]
        for prim in doc['meshes'][mesh]['primitives']:
            pos = read(doc, body, prim['attributes']['POSITION'])
            j = read(doc, body, prim['attributes']['JOINTS_0']).astype(int)
            wa = prim['attributes']['WEIGHTS_0']; w = read(doc, body, wa).astype(float)
            scale = {5126: 1., 5121: 255., 5123: 65535.}[doc['accessors'][wa]['componentType']]
            w /= scale
            hips_slot = np.array(skin['joints'])[j] == hips
            moved = 0
            for v in range(len(pos)):
                if not hips_slot[v].any(): continue
                for k, (c, (node, _)) in enumerate(zip(cheeks, new)):
                    d = np.linalg.norm((pos[v] - c) * np.array([1., 1.3, 1.]))
                    if d > RADIUS or pos[v][2] > c[2] + 0.05: continue
                    f = (1 - d / RADIUS) ** 2 * MAX
                    s15 = int(np.argmax(hips_slot[v])); take = w[v, s15] * f
                    if take < 0.02: continue
                    free = [s for s in range(4) if w[v, s] == 0 and s != s15]
                    if not free:
                        small = min((s for s in range(4) if s != s15), key=lambda s: w[v, s])
                        w[v, s15] += w[v, small]; w[v, small] = 0; free = [small]; take = w[v, s15] * f
                    w[v, s15] -= take; w[v, free[0]] = take; j[v, free[0]] = skin['joints'].index(node); moved += 1
                    hips_slot[v] = np.array(skin['joints'])[j[v]] == hips
            out_w = (w * scale).round().astype({5126: np.float32, 5121: np.uint8, 5123: np.uint16}[doc['accessors'][wa]['componentType']]) \
                if scale != 1. else w.astype(np.float32)
            prim['attributes']['WEIGHTS_0'] = append(doc, body, out_w, like=wa)
            prim['attributes']['JOINTS_0'] = append(doc, body, j.astype(np.uint16))
            print(f'  {name}: {moved} vertex weights moved')
    save(path, doc, body)


SCARF = {5: [(-0.22, 1.59, -0.13), (-0.14, 1.48, -0.145), (-0.05, 1.41, -0.145)],
         10: [(0.22, 1.59, -0.13), (0.14, 1.48, -0.145), (0.05, 1.41, -0.145)]}


def stage_scarf(path):
    doc, body = load(path)
    node_names = [n.get('name') for n in doc['nodes']]
    if 'role_0_bone210' in node_names: print(path.name, 'already has the scarf bones'); return
    skin0 = doc['skins'][0]
    ibm0 = read(doc, body, skin0['inverseBindMatrices']).reshape(-1, 4, 4).transpose(0, 2, 1)
    chains = {}   # parent bone id -> [(node index, bind world)]
    next_id = 210
    for parent_id, points in SCARF.items():
        parent = node_names.index(f'role_0_bone{parent_id}')
        parent_world = np.linalg.inv(ibm0[skin0['joints'].index(parent)])
        chain = []
        for p in points:
            world = parent_world.copy(); world[:3, 3] = p
            local = np.linalg.inv(parent_world) @ world
            doc['nodes'].append(dict(name=f'role_0_bone{next_id}', translation=[float(x) for x in local[:3, 3]]))
            index = len(doc['nodes']) - 1
            doc['nodes'][parent].setdefault('children', []).append(index)
            chain.append((index, world)); parent, parent_world = index, world; next_id += 1
        chains[parent_id] = chain
    new = [c for chain in chains.values() for c in chain]
    made = {}
    for s in doc['skins']:
        key = s['inverseBindMatrices']
        if key not in made:
            ibm = read(doc, body, key).reshape(-1, 16)
            extra = np.stack([np.linalg.inv(w).T.reshape(16) for _, w in new]).astype(np.float32)
            made[key] = append(doc, body, np.vstack([ibm, extra]).astype(np.float32), type_='MAT4')
        s['inverseBindMatrices'] = made[key]; s['joints'] = s['joints'] + [i for i, _ in new]
    mesh_nodes = {n['mesh']: i for i, n in enumerate(doc['nodes']) if 'mesh' in n}
    for name in ('hagoromo_KB', 'hagoromo_UR_KB'):
        mesh = names.index(name)
        skin = doc['skins'][doc['nodes'][mesh_nodes[mesh]]['skin']]
        for prim in doc['meshes'][mesh]['primitives']:
            pos = read(doc, body, prim['attributes']['POSITION'])
            j = read(doc, body, prim['attributes']['JOINTS_0']).astype(int)
            wa = prim['attributes']['WEIGHTS_0']; w = read(doc, body, wa).astype(float)
            scale = {5126: 1., 5121: 255., 5123: 65535.}[doc['accessors'][wa]['componentType']]; w /= scale
            moved = 0
            for v in range(len(pos)):
                x, y, z = pos[v]
                if z > -0.07 or abs(x) > 0.27: continue
                share = min(1., (0.27 - abs(x)) / 0.07)
                target = {}
                for parent_id, chain in chains.items():
                    side = 1. if (x < 0) == (parent_id == 5) else 0.
                    if abs(x) < 0.03: side = 0.5
                    if side == 0: continue
                    xs = [abs(SCARF[parent_id][k][0]) for k in range(3)]
                    for k, (node, _) in enumerate(chain):   # tent weights along |x|
                        lo = xs[k + 1] if k + 1 < 3 else 0.; hi = xs[k - 1] if k > 0 else 1.
                        ax = abs(x)
                        t = (ax - lo) / (xs[k] - lo) if ax <= xs[k] else (hi - ax) / (hi - xs[k])
                        if t > 0: target[skin['joints'].index(node)] = target.get(skin['joints'].index(node), 0.) + side * min(t, 1.)
                total = sum(target.values())
                if total <= 0: continue
                mixed = {}
                for s_ in range(4):
                    if w[v, s_] > 0: mixed[j[v, s_]] = mixed.get(j[v, s_], 0.) + w[v, s_] * (1 - share)
                for k, t in target.items(): mixed[k] = mixed.get(k, 0.) + share * t / total
                top = sorted(mixed.items(), key=lambda kv: -kv[1])[:4]
                norm = sum(t for _, t in top)
                j[v] = [k for k, _ in top] + [0] * (4 - len(top))
                w[v] = [t / norm for _, t in top] + [0.] * (4 - len(top))
                moved += 1
            out_w = (w * scale).round().astype({5126: np.float32, 5121: np.uint8, 5123: np.uint16}[doc['accessors'][wa]['componentType']])                 if scale != 1. else w.astype(np.float32)
            prim['attributes']['WEIGHTS_0'] = append(doc, body, out_w, like=wa)
            prim['attributes']['JOINTS_0'] = append(doc, body, j.astype(np.uint16))
            print(f'  {name}: {moved} vertices onto the scarf chains')
    save(path, doc, body)


for model in ('rinrin-source.glb', 'rinrin-pack.glb'):
    stage(ROOT / 'assets/native' / model)
    stage_scarf(ROOT / 'assets/native' / model)
