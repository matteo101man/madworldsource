"""Blender (headless) inspection of the user-provided Kreely FBX.

blender -b --factory-startup --python tools/kreely/inspect_fbx.py -- <fbx> <out.json>
"""
import bpy, json, sys
from mathutils import Vector

fbx, out = sys.argv[sys.argv.index('--') + 1:][:2]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=fbx)
report = dict(source=fbx, objects=[])
for o in bpy.data.objects:
    e = dict(name=o.name, type=o.type, parent=o.parent.name if o.parent else None,
             location=list(o.location), rotation=list(o.rotation_euler), scale=list(o.scale),
             modifiers=[(m.type, getattr(m, 'object', None) and m.object.name) for m in o.modifiers])
    if o.type == 'MESH':
        me = o.data
        ws = [o.matrix_world @ v.co for v in me.vertices]
        lo = Vector((min(v.x for v in ws), min(v.y for v in ws), min(v.z for v in ws)))
        hi = Vector((max(v.x for v in ws), max(v.y for v in ws), max(v.z for v in ws)))
        e.update(vertices=len(me.vertices), polygons=len(me.polygons), loops=len(me.loops),
                 uv_layers=[u.name for u in me.uv_layers], materials=[m.name if m else None for m in me.materials],
                 vertex_groups=[g.name for g in o.vertex_groups], world_min=list(lo), world_max=list(hi),
                 color_attributes=[c.name for c in me.color_attributes])
    if o.type == 'ARMATURE':
        e['bones'] = [dict(name=b.name, parent=b.parent.name if b.parent else None,
                           head=list(o.matrix_world @ b.head_local), tail=list(o.matrix_world @ b.tail_local))
                      for b in o.data.bones]
    report['objects'].append(e)
report['materials'] = [dict(name=m.name, images=[n.image.filepath for n in m.node_tree.nodes if n.type == 'TEX_IMAGE' and n.image] if m.use_nodes else [])
                       for m in bpy.data.materials]
report['actions'] = [a.name for a in bpy.data.actions]
json.dump(report, open(out, 'w'), indent=1)
