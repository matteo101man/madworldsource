"""Native event task vtable index, bounded by known constructor assignments."""
from pathlib import Path
import struct,json,sys
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer');from gk.graph import Graph
raw=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol').read_bytes();names={int(a,16):n for a,s,n in (x.split('\t') for x in (ROOT/'tools/ghidra-functions.tsv').read_text().splitlines())}
def read(address):
 for i in range(18):
  j=i if i<7 else i-7
  o,a,z=(struct.unpack_from('>I',raw,k+j*4)[0] for k in ((0,0x48,0x90) if i<7 else (0x1c,0x64,0xac)))
  if a<=address< a+z:return struct.unpack_from('>I',raw,o+address-a)[0]
 raise ValueError('Unmapped vtable')
g=Graph(dict(game='MadWorld',authority='Original vtable pointers assigned by native event task constructors; slot semantics unresolved'));rows=[]
for name,base in [('event-object-task',0x805a39e8),('event-camera-task',0x80596a98)]:
 g.add_node(name,'native-vtable',name,address=base,thisOffset=0x404)
 for offset in range(8,0x60,4):
  target=read(base+offset)
  if target==0 and offset<8:continue
  if target not in names:break
  fid=f'fn:{target:08x}';g.add_node(fid,'native-function',names[target],address=target);g.add_edge(name,fid,'virtual-slot',offset=offset)
  row=dict(table=name,offset=offset,target=f'{target:08x}',function=names[target])
  first,second=read(target),read(target+4)
  if first>>26==14 and (first>>21)&31==3 and (first>>16)&31==3 and second&0xfc000003==0x48000000:
   delta=first&0xffff;delta=delta-0x10000 if delta&0x8000 else delta
   disp=second&0x3fffffc;disp=disp-0x4000000 if disp&0x2000000 else disp
   resolved=target+4+disp;rid=f'fn:{resolved:08x}'
   row.update(thisDelta=delta,adjustorTarget=f'{resolved:08x}',adjustorFunction=names.get(resolved,'unresolved'))
   g.add_node(rid,'native-function',names.get(resolved,'unresolved'),address=resolved);g.add_edge(fid,rid,'adjustor-tail-call',thisDelta=delta)
  rows.append(row)
g.save(ROOT/'research/event-vtables.graph.jsonl');(ROOT/'research/event-vtables.json').write_text(json.dumps(rows,indent=2));print(json.dumps(rows,indent=2))
