"""Index bounded PPC displacement accesses; offsets are candidates, not pointer proof."""
from pathlib import Path
import struct,sys,json,bisect
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph
r=Path(__file__).resolve().parents[1];d=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol').read_bytes()
fns=sorted((int(a,16),int(s,16),name) for a,s,name in (x.split('\t') for x in (r/'tools/ghidra-functions.tsv').read_text().splitlines()));starts=[x[0] for x in fns]
g=Graph(dict(game='MadWorld',authority='Original PPC D-form instruction fields; matching displacement does not prove base pointer identity'));rows=[]
for section in range(7):
 off,addr,size=(struct.unpack_from('>I',d,k+section*4)[0] for k in (0,0x48,0x90))
 for p in range(0,size,4):
  word=struct.unpack_from('>I',d,off+p)[0];op=word>>26;imm=word&0xffff
  if op not in (32,34,36,38,40,42,44) or imm not in (0x3d0,0x3d2):continue
  pc=addr+p;i=bisect.bisect_right(starts,pc)-1;start,n,name=fns[i]
  if not start<=pc<start+n:continue
  fid=f'fn:{start:08x}';nid=f'access:{pc:08x}';kind={32:'load-u32',34:'load-u8',36:'store-u32',38:'store-u8',40:'load-u16',42:'load-i16',44:'store-u16'}[op]
  row=dict(address=f'{pc:08x}',function=f'{start:08x}',name=name,operation=kind,displacement=imm,baseRegister=(word>>16)&31,valueRegister=(word>>21)&31)
  rows.append(row);g.add_node(fid,'native-function',name,address=start);g.add_node(nid,'field-access-candidate',kind,**row);g.add_edge(fid,nid,'contains')
g.save(r/'research/save-access-candidates.graph.jsonl');(r/'research/save-access-candidates.json').write_text(json.dumps(rows,indent=2));print(json.dumps(rows,indent=2))
