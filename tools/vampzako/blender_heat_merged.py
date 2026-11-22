"""Blender (headless) bone heat, as tools/kreely/blender_weights.py, on a cleaned copy of the mesh: duplicate vertices
merged and loose bits dropped first (bone heat fails outright on some Tripo meshes otherwise); every input vertex then
takes the weights of its nearest cleaned vertex.

blender -b --factory-startup --python tools/vampzako/blender_heat_merged.py -- <in.npz> <out.npz>
"""
import bpy, bmesh, sys
import numpy as np
from mathutils.kdtree import KDTree

src, out = sys.argv[sys.argv.index('--') + 1:][:2]
d = np.load(src)
to_b = lambda p: np.stack([p[..., 0], -p[..., 2], p[..., 1]], -1)
P, T = to_b(d['positions']), d['triangles']
ids, heads, tails = d['bone_ids'], to_b(d['heads']), to_b(d['tails'])

bpy.ops.wm.read_factory_settings(use_empty=True)
me = bpy.data.meshes.new('body'); me.from_pydata(P.tolist(), [], T.tolist()); me.update()
bm = bmesh.new(); bm.from_mesh(me)
bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=0.002)
bmesh.ops.dissolve_degenerate(bm, edges=bm.edges, dist=1e-5)
loose = [v for v in bm.verts if not v.link_faces]
bmesh.ops.delete(bm, geom=loose, context='VERTS')
bm.to_mesh(me); bm.free(); me.update()
obj = bpy.data.objects.new('body', me); bpy.context.scene.collection.objects.link(obj)
arm_data = bpy.data.armatures.new('rig'); arm = bpy.data.objects.new('rig', arm_data)
bpy.context.scene.collection.objects.link(arm); bpy.context.view_layer.objects.active = arm
bpy.ops.object.mode_set(mode='EDIT')
for i, h, t in zip(ids, heads, tails):
    b = arm_data.edit_bones.new('bone%d' % i); b.head, b.tail = h.tolist(), t.tolist()
bpy.ops.object.mode_set(mode='OBJECT')
bpy.ops.object.select_all(action='DESELECT'); obj.select_set(True); arm.select_set(True)
bpy.context.view_layer.objects.active = arm
bpy.ops.object.parent_set(type='ARMATURE_AUTO')
Wc = np.zeros((len(me.vertices), len(ids)), np.float32)
col = {g.index: list(ids).index(int(g.name[4:])) for g in obj.vertex_groups}
for v in me.vertices:
    for g in v.groups: Wc[v.index, col[g.group]] = g.weight
kd = KDTree(len(me.vertices))
for v in me.vertices: kd.insert(v.co, v.index)
kd.balance()
W = np.zeros((len(P), len(ids)), np.float32)
for i, p in enumerate(P): W[i] = Wc[kd.find(p.tolist())[1]]
np.savez(out, weights=W)
print('weighted', int((W.sum(1) > 0).sum()), 'of', len(P), '(cleaned', len(me.vertices), ')')
