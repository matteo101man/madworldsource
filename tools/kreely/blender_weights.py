"""Blender (headless) bone-heat skinning of the fitted Kreely mesh on Jack's pl0030 joints.

blender -b --factory-startup --python tools/kreely/blender_weights.py -- <in.npz> <out.npz>
in.npz : positions (N,3 Y-up, already fitted to Jack), triangles (T,3), bone_ids (B,), heads (B,3), tails (B,3)
out.npz: weights (N,B) raw automatic weights (rows may be all-zero where heat failed)
All coordinates are converted Y-up -> Blender Z-up as (x, -z, y), the same mapping Blender's FBX importer uses.
"""
import bpy, sys
import numpy as np

src, out = sys.argv[sys.argv.index('--') + 1:][:2]
d = np.load(src)
to_b = lambda p: np.stack([p[..., 0], -p[..., 2], p[..., 1]], -1)
P, T = to_b(d['positions']), d['triangles']
ids, heads, tails = d['bone_ids'], to_b(d['heads']), to_b(d['tails'])

bpy.ops.wm.read_factory_settings(use_empty=True)
me = bpy.data.meshes.new('kreely')
me.from_pydata(P.tolist(), [], T.tolist())
me.update()
obj = bpy.data.objects.new('kreely', me)
bpy.context.scene.collection.objects.link(obj)

arm_data = bpy.data.armatures.new('pl0030')
arm = bpy.data.objects.new('pl0030', arm_data)
bpy.context.scene.collection.objects.link(arm)
bpy.context.view_layer.objects.active = arm
bpy.ops.object.mode_set(mode='EDIT')
for i, h, t in zip(ids, heads, tails):
    b = arm_data.edit_bones.new('bone%d' % i)
    b.head, b.tail = h.tolist(), t.tolist()
bpy.ops.object.mode_set(mode='OBJECT')

bpy.ops.object.select_all(action='DESELECT')
obj.select_set(True); arm.select_set(True)
bpy.context.view_layer.objects.active = arm
bpy.ops.object.parent_set(type='ARMATURE_AUTO')

W = np.zeros((len(P), len(ids)), np.float32)
col = {g.index: list(ids).index(int(g.name[4:])) for g in obj.vertex_groups}
for v in me.vertices:
    for g in v.groups:
        W[v.index, col[g.group]] = g.weight
np.savez(out, weights=W)
print('weighted', int((W.sum(1) > 0).sum()), 'of', len(P))
