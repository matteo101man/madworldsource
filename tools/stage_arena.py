"""Stage a room's native SCR and spawn set (default c201 / c20110). No invented arena geometry.
MW_ROOM=c303 MW_SPAWNSET=c30300 python -I tools/stage_arena.py stages another room into assets/native/rooms/<room>/world/
(the c201 outputs, docs and graph stay where mw-play's RinRin arena reads them). MW_SPAWNSET=none (or empty) stages a
room with no .ems spawn set, e.g. MW_ROOM=ca03 MW_SPAWNSET=none (ev84's event set caseA/ca03.dat): arena-meta.json then
has "spawns": [] and the .json "spawns" is empty. Part transforms: c201's are all identity;
other rooms may use translate x rotate-about-Y x uniform scale (c303: checked), which needs no Euler order; anything else
still fails staging. Native order (8020C274 copies descriptor +0x34/+0x40/+0x4C into the part's pos/rot/scale;
cParts::updateLocalMatrix 8020F9F8): local = PSMTXTrans(pos) . MtxInitRotVec(rot, order +0xA0; 0 = Rx.Ry.Rz,
PSMTXRotRad radians) . PSMTXScale(scale), world = parent . local (8020FBFC; SCR parts are
created without a parent [inferred]); three.js T.R.S with yaw sign +1."""
from pathlib import Path
import hashlib,json,struct,sys,math,os
ROOT=Path(__file__).resolve().parents[1];EXPLORER=Path('D:/Games/gamesfordecomp/exports/madworld');sys.path.insert(0,str(EXPLORER/'builders'))
from native import dat_entries,mdb,tpl,Reader
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph
ROOM=os.environ.get('MW_ROOM','c201');SPAWNSET=os.environ.get('MW_SPAWNSET','c20110');DEFAULT=ROOM=='c201'
source=EXPLORER/'extracted/disc/DATA/files'/f'case{ROOM[1]}'/f'{ROOM}.dat';raw=source.read_bytes();entries=dat_entries(raw)
def member(name):
 e=next(e for e in entries if e['name']==name);return raw[e['offset']:e['offset']+e['size']],e
scr,scrEntry=member(ROOM+'.scr');r=Reader(scr);count,textures=r.get('II',4)
if scr[:4]!=b'SCR\0' or not 0<count<10000 or 16+count*96>textures or textures>=len(scr):raise ValueError('SCR directory bounds')
rows=[]
for i in range(count):
 o=16+i*96;name=r.get('48s',o)[0].split(b'\0')[0].decode('cp932');start=o+r.u32(o+48);transform=list(r.get('9f',o+52))
 if start<16+count*96 or start>=textures or scr[start:start+2]!=b'JK':raise ValueError('SCR model range')
 if not all(math.isfinite(v) for v in transform):raise ValueError('SCR nonfinite transform')
 yaw_only=abs(transform[3])<1e-6 and abs(transform[5])<1e-6 and abs(transform[6]-transform[7])<1e-6 and abs(transform[7]-transform[8])<1e-6
 if transform!=[0.,0.,0.,0.,0.,0.,1.,1.,1.] and (DEFAULT or not yaw_only):raise ValueError('SCR transform outside the verified basis (Y rotation, uniform scale)')
 rows.append(dict(name=name,start=start,transform=transform,unknownWords=list(r.get('2I',o+88))))
starts=sorted(set(x['start'] for x in rows)|{textures});world=[];failures=[];g=Graph(dict(game='MadWorld',authority='Native SCR descriptors and bounded MDB/TPL reader; visual materials fallback; native collision indexed separately in collision.graph.jsonl'))
SCR=f'case{ROOM[1]}/{ROOM}.scr';g.add_node(SCR,'world-resource',ROOM+'.scr',sourceSha256=hashlib.sha256(scr).hexdigest())
for i,x in enumerate(rows):
 end=starts[starts.index(x['start'])+1]
 try:
  model=mdb(scr[x['start']:end]);model.update(x);world.append(model)
  g.add_node('world-model:'+str(i),'world-model',x['name'],offset=x['start'],size=end-x['start'],meshCount=len(model['meshes']),boneCount=len(model['bones']));g.add_edge(SCR,'world-model:'+str(i),'contains')
 except Exception as ex:failures.append(dict(model=x['name'],error=str(ex)))
out=ROOT/'assets/native/world' if DEFAULT else ROOT/'assets/native/rooms'/ROOM/'world';out.mkdir(parents=True,exist_ok=True)
images=tpl(scr[textures:])
for i,(im,*_) in enumerate(images):im.save(out/f'texture-{i:03}.png')
spawns=[];families=[0x20000,0x20000,0x10000,0x70000];n=0
if SPAWNSET.lower() not in ('','none'):
 ems,emsEntry=member(SPAWNSET+'.ems');r=Reader(ems);version,n=r.get('HH',4)
 if version!=20 or len(ems)<32+n*128:raise ValueError('Unsupported spawn set layout')
for i in range(n):
 o=32+i*128;nativeId=r.get('H',o)[0];family=r.get('B',o+0x4d)[0]
 if family>=len(families):raise ValueError('Unknown spawn family')
 spawns.append(dict(index=i,id=families[family]+nativeId,localId=nativeId,family=family,position=list(r.get('3f',o+0x10)),rawRecordHex=ems[o:o+128].hex()))
 g.add_node(f'spawn:{SPAWNSET}:{i}','native-spawn',hex(families[family]+nativeId),position=spawns[-1]['position'],sourceOffset=o);g.add_edge(f'spawn:{SPAWNSET}:{i}',SCR,'shares-case-bank')
result=dict(source=str(source),sourceSha256=hashlib.sha256(raw).hexdigest(),models=world,textures=len(images),spawns=spawns,failures=failures,authority='Native SCR mesh/TRS/TPL and EMS positions. Native collision, draw visibility and GX scene lighting remain unresolved.')
meta=dict(spawns=[{k:v for k,v in x.items() if k!='rawRecordHex'} for x in spawns],world=f'native/world/{ROOM}.glb' if DEFAULT else f'native/rooms/{ROOM}/world/{ROOM}.glb',sourceSha256=result['sourceSha256'])
(ROOT/'assets/native/arena-meta.json' if DEFAULT else out.parent/'arena-meta.json').write_text(json.dumps(meta,indent=2))
(out/f'{ROOM}.json').write_text(json.dumps(result,separators=(',',':')),encoding='utf-8');g.save(ROOT/('research/arena.graph.jsonl' if DEFAULT else f'research/arena-{ROOM}.graph.jsonl'))
if DEFAULT: (ROOT/'research/ARENA-API.md').write_text('# Native RinRin arena lead\n\nSCR: 16-byte header, 96-byte descriptors, relative MDB offsets and embedded TPL. Layout authority: MIT Kerilk/noesis_bayonetta_pc MadWorld.h, pinned in explorer THIRD-PARTY.txt. All c201 transforms are identity; nonidentity transforms fail staging pending basis verification.\n\nEMS v20: 32-byte header and 128-byte records. Native cEmSet helpers 0x801ebb74/0x801ebb48 give header/record sizes. Create wrapper 0x801ea684 passes record u16 at0 through 0x801ea61c family table indexed by byte0x4d; family bases at0x8055fba8 are 0x20000,0x20000,0x10000,0x70000. Position at0x10 is passed to CreateObj at0x801ea938/0x801ea980. c20110 record0 is0x20440 at(2.3005285,17.822876,-68.66516). Original ev21 camera is in the same world region. Native collision geometry is indexed separately in collision.graph.jsonl; native visibility/contact behavior remains unverified.\n',encoding='utf-8')
print(dict(models=len(world),textures=len(images),meshes=sum(len(x['meshes']) for x in world),vertices=sum(len(m['positions']) for x in world for m in x['meshes']),skinned=sum(bool(x['bones']) for x in world),spawns=[dict(id=hex(x['id']),position=x['position']) for x in spawns],failures=failures))

