"""Stand-in collision for a room whose native Havok files cannot be staged here yet [added, not native]: every world part
of the decoded SCR (tools/stage_arena.py MW_ROOM=<room>, assets/native/rooms/<room>/world/<room>.json) becomes one
col.hkx body of its render triangles (hkpSimpleMeshShape), placed by its SCR transform (translate x rotate-about-Y x
uniform scale, as tools/export_arena.mjs draws it). Floors and walls come out where the render geometry is, but
decorations collide as solids and invisible native blockers are missing. Replace with tools/stage_collision.py
(MW_COLLISION_DIR) once the native .hkx can be read.
Usage: MW_ROOM=c303 python -I tools/stage_render_collision.py
"""
import json, math, os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ROOM = os.environ.get('MW_ROOM', 'c303')
src = ROOT / 'assets/native/rooms' / ROOM / 'world' / f'{ROOM}.json'
data = json.loads(src.read_text(encoding='utf-8'))
bodies, shapes = [], []
for i, model in enumerate(data['models']):
    t = model.get('transform') or [0, 0, 0, 0, 0, 0, 1, 1, 1]
    c, s = math.cos(t[4]), math.sin(t[4])
    verts, tris = [], []
    for m in model['meshes']:
        base = len(verts)
        for x, y, z in m['positions']:
            x, y, z = x * t[6], y * t[7], z * t[8]
            verts.append([t[0] + c * x + s * z, t[1] + y, t[2] - s * x + c * z])
        idx = m['indices']
        tris += [[base + idx[k], base + idx[k + 1], base + idx[k + 2]] for k in range(0, len(idx) - 2, 3)]
    if not tris: continue
    sid = f'render:{i}'
    shapes.append(dict(id=sid, kind='hkpSimpleMeshShape', radius=0.0, vertices=verts, triangles=tris, planes=[], originalTriangles=True))
    bodies.append(dict(id=f'body:{i}', source='col.hkx', name=model['name'], filterInfo=0, friction=0.5, restitution=0.0, shapes=[sid]))
out = ROOT / 'assets/native/rooms' / ROOM / 'collision'
out.mkdir(parents=True, exist_ok=True)
(out / 'world.json').write_text(json.dumps(dict(bodies=bodies, shapes=shapes, failures=[],
    authority=f'STAND-IN: render triangles of {ROOM}.scr as collision (tools/stage_render_collision.py), not the native Havok files'),
    separators=(',', ':')))
print(dict(room=ROOM, bodies=len(bodies), triangles=sum(len(s['triangles']) for s in shapes)))
