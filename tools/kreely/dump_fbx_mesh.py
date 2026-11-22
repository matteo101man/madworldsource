"""Blender (headless): dump the Kreely FBX mesh as split-vertex arrays in the FBX's own Y-up space.

blender -b --factory-startup --python tools/kreely/dump_fbx_mesh.py -- <fbx> <out.npz>
Vertices are split per unique (vertex, uv) so each output vertex has one UV, like the explorer's MDB meshes.
"""
import bpy, sys
import numpy as np

fbx, out = sys.argv[sys.argv.index('--') + 1:][:2]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=fbx)
obj = next(o for o in bpy.data.objects if o.type == 'MESH')
me = obj.data
me.calc_loop_triangles()
co = np.array([v.co[:] for v in me.vertices], dtype=np.float64)   # object space: FBX Y-up
uv = me.uv_layers.active.data
keys, verts, uvs, src = {}, [], [], []
tris = []
for t in me.loop_triangles:
    tri = []
    for li in t.loops:
        vi = me.loops[li].vertex_index
        u = tuple(round(c, 6) for c in uv[li].uv)
        k = (vi, u)
        if k not in keys:
            keys[k] = len(src); src.append(vi); uvs.append((u[0], 1.0 - u[1]))
        tri.append(keys[k])
    tris.append(tri)
np.savez(out, positions=co[src], uvs=np.array(uvs), indices=np.array(tris, dtype=np.int32),
         source_vertex=np.array(src, dtype=np.int32), source_positions=co)
print('split vertices', len(src), 'triangles', len(tris))
