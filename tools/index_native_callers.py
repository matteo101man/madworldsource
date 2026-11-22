"""Bounded direct-call leads from owned DOL; no game instruction/text dump."""
from pathlib import Path
import argparse,bisect,struct,json,hashlib
from save_typed_graph import save_graph
ROOT=Path(__file__).resolve().parents[1]
def main():
 ap=argparse.ArgumentParser();ap.add_argument('targets',nargs='+',type=lambda s:int(s,16));ap.add_argument('--name',default='native-state-callers');a=ap.parse_args()
 if not(1<=len(a.targets)<=12):raise ValueError('Bounded target count')
 b=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol').read_bytes();symbols=[(int(x,16),int(z,16),n) for x,z,n in (s.split('\t') for s in (ROOT/'tools/ghidra-functions.tsv').read_text(encoding='utf-8').splitlines())];starts=[s[0] for s in symbols];rows=[]
 for i in range(7):
  off,addr,size=[struct.unpack_from('>I',b,t+i*4)[0] for t in (0,0x48,0x90)]
  for p in range(0,size,4):
   ins=struct.unpack_from('>I',b,off+p)[0]
   if ins&0xfc000002!=0x48000000:continue
   displacement=ins&0x03fffffc
   if displacement&0x02000000:displacement-=0x04000000
   site=addr+p;target=(site+displacement)&0xffffffff
   if target not in a.targets:continue
   ix=bisect.bisect_right(starts,site)-1
   if ix<0 or site>=symbols[ix][0]+symbols[ix][1]:continue
   fn=symbols[ix];rows.append(dict(site=hex(site),target=hex(target),link=bool(ins&1),caller=hex(fn[0]),callerName=fn[2],callerSize=fn[1]))
 nodes={};edges=[]
 for r in rows:
  for kind,pc in [('caller',int(r['caller'],16)),('target',int(r['target'],16))]:
   ix=bisect.bisect_left(starts,pc);name=symbols[ix][2] if ix<len(symbols) and starts[ix]==pc else 'unknown'
   nodes[hex(pc)]=dict(type='node',id='function:'+hex(pc),kind='native_function',label=name,address=hex(pc))
  edges.append(dict(type='edge',source='function:'+r['caller'],target='function:'+r['target'],kind='direct_call' if r['link'] else 'direct_branch_lead',site=r['site'],authority='Relative PPC branch encoding; control-flow reachability not proved'))
 save_graph(ROOT/f'research/{a.name}.graph.jsonl',list(nodes.values())+edges,'Bounded original relative-branch target index; no indirect-call coverage')
 out=dict(dolSha1=hashlib.sha1(b).hexdigest(),requestedTargets=[hex(x) for x in a.targets],scope='Direct relative call/tail-branch leads only; not complete dispatch proof',calls=rows)
 (ROOT/f'research/{a.name}.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8');print(json.dumps(rows,indent=2))
if __name__=='__main__':main()
