"""Optional Blender background render of actual exported GLBs, without desktop input."""
import json
from pathlib import Path
import sys
import math
import struct
import bpy
from mathutils import Vector

root=Path(sys.argv[sys.argv.index('--')+1]);name=sys.argv[sys.argv.index('--')+2].replace(':','-')
report=json.loads((root/'evidence'/('node-export-'+name+'.json')).read_text())
bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
bpy.ops.import_scene.gltf(filepath=report['path'])
scene=bpy.context.scene;scene.frame_set(1);bpy.context.view_layer.update()
points=[obj.matrix_world@Vector(corner) for obj in scene.objects if obj.type=='MESH' for corner in obj.bound_box]
lo=Vector(tuple(min(p[i] for p in points) for i in range(3)));hi=Vector(tuple(max(p[i] for p in points) for i in range(3)));center=(lo+hi)/2;radius=max(hi-lo)
bpy.ops.object.camera_add();camera=bpy.context.object;camera.location=center+Vector((radius*.85,-radius*1.65,radius*.25));camera.rotation_euler=(center-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.lens=48;scene.camera=camera
scene.world.color=(.07,.085,.09);scene.render.engine='BLENDER_EEVEE_NEXT';scene.render.resolution_x=640;scene.render.resolution_y=760;scene.render.resolution_percentage=100;scene.render.image_settings.file_format='PNG';scene.render.filepath=str(root/'evidence'/('export-color-'+name+'.png'));scene.view_settings.view_transform='Standard';scene.render.film_transparent=False
bpy.ops.render.render(write_still=True)
if name.startswith('event-'):
    _,event,cut=name.split('-');descriptor=next(e for e in json.loads((root/'events.json').read_text())['events'] if e['id']==event);span=descriptor['cuts'][int(cut)];raw=Path(descriptor['path']).read_bytes();base=span['offset']+32
    scene.render.resolution_x=960;scene.render.resolution_y=540
    for frame in sorted({0,span['frames']//3,2*span['frames']//3,span['frames']-1}):
        values=struct.unpack_from('>8f',raw,base+frame*32)
        scene.frame_set(int(frame*scene.render.fps/60),subframe=frame*scene.render.fps/60%1)
        camera.location=(values[0],-values[2],values[1]);target=Vector((values[3],-values[5],values[4]));camera.rotation_euler=(target-camera.location).to_track_quat('-Z','Y').to_euler();camera.data.sensor_fit='VERTICAL';camera.data.sensor_height=24;camera.data.lens=12/math.tan(values[7]/2)
        scene.render.filepath=str(root/'evidence'/('export-'+name+'-frame-'+str(frame)+'.png'));bpy.ops.render.render(write_still=True)
print('VERIFIED_GLB_RENDER',report['path'])
