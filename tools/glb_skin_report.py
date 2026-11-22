"""Which bones weight the vertices in a region of a skinned GLB (bind pose).

python -I tools/glb_skin_report.py <glb> [--ymin Y --ymax Y --zmin Z]  -> per bone: vertex count and summed weight
"""
import argparse, json, struct
from collections import Counter
import numpy as np

COMP = {5126: np.float32, 5123: np.uint16, 5121: np.uint8, 5125: np.uint32}
SIZE = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}


def load(path):
    d = open(path, 'rb').read(); jl = struct.unpack_from('<I', d, 12)[0]; js = json.loads(d[20:20 + jl])
    bl = struct.unpack_from('<I', d, 20 + jl)[0]; bin_ = d[28 + jl:28 + jl + bl]
    def acc(i):
        a = js['accessors'][i]; v = js['bufferViews'][a['bufferView']]
        off = v.get('byteOffset', 0) + a.get('byteOffset', 0); n = a['count'] * SIZE[a['type']]
        return np.frombuffer(bin_, COMP[a['componentType']], n, off).reshape(a['count'], SIZE[a['type']])
    return js, acc


def main():
    ap = argparse.ArgumentParser(); ap.add_argument('glb'); ap.add_argument('--ymin', type=float, default=-1e9)
    ap.add_argument('--ymax', type=float, default=1e9); ap.add_argument('--zmin', type=float, default=-1e9)
    a = ap.parse_args(); js, acc = load(a.glb)
    names = [n.get('name', '') for n in js['nodes']]
    tally = Counter(); wsum = Counter()
    for nd in js['nodes']:
        if 'mesh' not in nd or 'skin' not in nd: continue
        joints = js['skins'][nd['skin']]['joints']
        for pr in js['meshes'][nd['mesh']]['primitives']:
            at = pr['attributes']; P = acc(at['POSITION']); J = acc(at['JOINTS_0']); W = acc(at['WEIGHTS_0']).astype(np.float32)
            m = (P[:, 1] > a.ymin) & (P[:, 1] < a.ymax) & (P[:, 2] > a.zmin)
            for j, w in zip(J[m], W[m]):
                for k in range(4):
                    if w[k] > 0.05: tally[names[joints[j[k]]]] += 1; wsum[names[joints[j[k]]]] += float(w[k])
    for n, c in tally.most_common(20): print(f'{n:24s} {c:6d} {wsum[n]:8.1f}')


if __name__ == '__main__':
    main()
