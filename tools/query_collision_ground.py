from pathlib import Path
import json,collections,math
r=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');d=json.loads((r/'assets/native/collision/world.json').read_text());smap={x['id']:x for x in d['shapes']};x,y,z=next(q for q in json.loads((r/'assets/native/arena-meta.json').read_text())['spawns'] if q['id']==0x20440)['position'];hits=[]
for body in d['bodies']:
 for sid in body['shapes']:
  shape=smap[sid];verts=shape['vertices']
  if not(min(v[0] for v in verts)-1e-4<=x<=max(v[0] for v in verts)+1e-4 and min(v[2] for v in verts)-1e-4<=z<=max(v[2] for v in verts)+1e-4):continue
  if shape['planes']:
   lo,hi=-math.inf,math.inf
   for nx,ny,nz,w in shape['planes']:
    b=nx*x+nz*z+w
    if abs(ny)<1e-7:
     if b>1e-4:lo,hi=1,0;break
    elif ny>0:hi=min(hi,-b/ny)
    else:lo=max(lo,-b/ny)
   if lo<=hi and hi<=y+.1:hits.append(dict(source=body['source'],body=body['name'],height=hi,delta=y-hi,radius=shape['radius']))
  else:
   for tri in shape['triangles']:
    a,b,c=[verts[i] for i in tri];dx,dz=b[0]-a[0],b[2]-a[2];ex,ez=c[0]-a[0],c[2]-a[2];den=dx*ez-ex*dz
    if abs(den)<1e-9:continue
    u=((x-a[0])*ez-ex*(z-a[2]))/den;v=(dx*(z-a[2])-(x-a[0])*dz)/den
    if u>=0 and v>=0 and u+v<=1:
     hi=a[1]+u*(b[1]-a[1])+v*(c[1]-a[1])
     if hi<=y+.1:hits.append(dict(source=body['source'],body=body['name'],height=hi,delta=y-hi,radius=shape['radius']))
hits=sorted(hits,key=lambda q:q['height'],reverse=True);print('native roof contacts',hits[:12]);(r/'evidence/native-spawn-ground-query.json').write_text(json.dumps(dict(position=[x,y,z],hits=hits,authority='Geometric diagnostic without native filter/radius application; not Havok runtime equivalence'),indent=2));print('filter summaries',{f:dict(collections.Counter(hex(b['filterInfo']) for b in d['bodies'] if b['source']==f)) for f in sorted(set(b['source'] for b in d['bodies']))})
