"""Stage native static Havok shape geometry without replacing collision by render meshes."""
from pathlib import Path
import sys,json,hashlib,collections
import numpy as np
from scipy.spatial import ConvexHull,QhullError
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT/'tools'));from havok_adapter import install_reflected_types
from soulstruct.havok import HKX
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer');from gk.graph import Graph
import os;CDIR=ROOT/os.environ.get('MW_COLLISION_DIR','assets/native/collision')   # another room: assets/native/rooms/<room>/collision
paths=list(CDIR.glob('*.hkx'));install_reflected_types(paths);g=Graph(dict(game='MadWorld',authority='Original Havok5.5 reflected layouts via Soulstruct adapter; source transforms/geometry/filter/radius retained; native runtime query equivalence not yet verified'));bodies=[];shapes=[];failures=[]
def mat(values):
 m=np.asarray(values,dtype=float).reshape(4,4).T.copy();m[3]=[0,0,0,1];return m  # hkTransform has an implicit affine last row; vector w lanes are not used.

def walk(shape,transform,sid):
 kind=type(shape).__name__;g.add_node(sid,'collision-shape',kind)
 if kind=='hkpMoppBvTreeShape':
  code=bytes(shape.code.data);g.add_node(sid+':mopp','mopp-code','native acceleration',bytes=len(code),sha256=hashlib.sha256(code).hexdigest());g.add_edge(sid,sid+':mopp','accelerated-by');walk(shape.child.childShape,transform,sid+':child');g.add_edge(sid,sid+':child','contains');return
 if kind in ('hkpConvexTransformShape','hkpConvexTranslateShape'):
  child=np.eye(4)
  if kind=='hkpConvexTransformShape':child=mat(shape.transform)
  else:child[:3,3]=list(shape.translation)[:3]
  walk(shape.childShape.childShape,transform@child,sid+':child');g.add_edge(sid,sid+':child','contains');return
 if kind=='hkpListShape':
  for i,child in enumerate(shape.childInfo):
   if not (shape.enabledChildren[i//32]&(1<<(i%32))):continue
   walk(child.shape,transform,sid+':'+str(i));g.add_edge(sid,sid+':'+str(i),'contains',filterInfo=child.collisionFilterInfo)
  return
 radius=float(shape.radius);planes=[];originalTriangles=True
 if kind=='hkpConvexVerticesShape':
  vertices=[]
  for four in shape.rotatedVertices:
   for i in range(4):vertices.append([four.x[i],four.y[i],four.z[i]])
  vertices=np.asarray(vertices[:shape.numVertices],dtype=float);p=np.asarray(shape.planeEquations,dtype=float).reshape(-1,4);worldPlanes=(np.linalg.inv(transform).T@p.T).T
  for plane in worldPlanes:
   length=np.linalg.norm(plane[:3]);assert length>0;planes.append((plane/length).tolist())
  try:triangles=ConvexHull(vertices).simplices.tolist()
  except QhullError:triangles=[]
  originalTriangles=False
 elif kind=='hkpSimpleMeshShape':
  vertices=np.asarray(shape.vertices,dtype=float)[:,:3];triangles=[[t.a,t.b,t.c] for t in shape.triangles]
 elif kind=='hkpBoxShape':
  # a box is the convex hull of its 8 corners: half extents -> corners and the 6 face planes (n.x + w <= 0 inside, as
  # hkpConvexVerticesShape's planeEquations), staged as a convex-vertices shape the runtime already queries
  h=[float(v) for v in list(shape.halfExtents)[:3]]
  vertices=np.asarray([[sx*h[0],sy*h[1],sz*h[2]] for sx in (-1,1) for sy in (-1,1) for sz in (-1,1)],dtype=float)
  p=np.asarray([[1,0,0,-h[0]],[-1,0,0,-h[0]],[0,1,0,-h[1]],[0,-1,0,-h[1]],[0,0,1,-h[2]],[0,0,-1,-h[2]]],dtype=float);worldPlanes=(np.linalg.inv(transform).T@p.T).T
  for plane in worldPlanes:
   length=np.linalg.norm(plane[:3]);planes.append((plane/length).tolist())
  triangles=ConvexHull(vertices).simplices.tolist();originalTriangles=False;kind='hkpConvexVerticesShape'
 else:raise ValueError('Unsupported native shape '+kind)
 world=(transform@np.column_stack([vertices,np.ones(len(vertices))]).T).T[:,:3]
 if not np.isfinite(world).all():raise ValueError('Nonfinite collision vertices')
 if any(any(v<0 or v>=len(world) for v in tri) for tri in triangles):raise ValueError('Native collision index bounds')
 row=dict(id=sid,kind=kind,radius=radius,vertices=world.tolist(),triangles=triangles,planes=planes,originalTriangles=originalTriangles,trianglePolicy='native mesh topology' if originalTriangles else 'derived convex hull for visualization only; planes/radius define native convex shape')
 if kind=='hkpSimpleMeshShape':row.update(materialIndices=list(shape.materialIndices),weldingType=shape.weldingType,weldingInfo=[t.weldingInfo for t in shape.triangles])
 shapes.append(row);g.nodes[sid]['attrs']=dict(vertexCount=len(world),triangleCount=len(triangles),planeCount=len(planes),radius=radius,originalTriangles=originalTriangles)

for path in paths:
 h=HKX.from_path(path);g.add_node(path.name,'collision-resource',path.name,sha256=hashlib.sha256(path.read_bytes()).hexdigest())
 for v in h.root.namedVariants:
  if not hasattr(v.variant,'systems'):continue
  for si,system in enumerate(v.variant.systems):
   for i,body in enumerate(system.rigidBodies):
    bid=f'body:{path.name}:{si}:{i}';sid=bid+':shape';before=len(shapes)
    try:
     transform=mat(body.motion.motionState.transform);walk(body.collidable.shape,transform,sid)
     row=dict(id=bid,source=path.name,name=body.name,transform=list(body.motion.motionState.transform),filterInfo=body.collidable.broadPhaseHandle.collisionFilterInfo,friction=body.material.friction,restitution=body.material.restitution,shapes=[s['id'] for s in shapes[before:]])
     bodies.append(row);g.add_node(bid,'collision-body',body.name,source=path.name,filterInfo=row['filterInfo'],friction=row['friction'],restitution=row['restitution']);g.add_edge(path.name,bid,'contains');g.add_edge(bid,sid,'shape')
    except Exception as ex:failures.append(dict(body=bid,name=body.name,error=str(ex)))
result=dict(bodies=bodies,shapes=shapes,failures=failures,authority='Native static body transforms, shape planes/vertices/radius and mesh indices; MOPP omitted as acceleration, not geometry; native filtering/margins/query behavior still require runtime comparison')
(CDIR/'world.json').write_text(json.dumps(result,separators=(',',':')));g.save(ROOT/('research/collision.graph.jsonl' if CDIR==ROOT/'assets/native/collision' else f'research/collision-{CDIR.parent.name}.graph.jsonl'));print(dict(bodies=len(bodies),shapes=len(shapes),vertices=sum(len(s['vertices']) for s in shapes),triangles=sum(len(s['triangles']) for s in shapes),failures=failures))
