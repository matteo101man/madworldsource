"""Conservative DOL string/magic reference leads; no decompiled dumps."""
from pathlib import Path
import struct,json,re,sys,capstone
ROOT=Path(__file__).resolve().parents[1];EXPLORER=Path('D:/Games/gamesfordecomp/exports/madworld');raw=(EXPLORER/'extracted/disc/DATA/sys/main.dol').read_bytes();sections=[]
for i in range(18):
 j=i if i<7 else i-7;o,a,z=[struct.unpack_from('>I',raw,base+j*4)[0] for base in ((0,0x48,0x90) if i<7 else (0x1c,0x64,0xac))]
 if z:sections.append((a,a+z,o,i<7))
def read(a,n):
 for low,high,o,_ in sections:
  if low<=a and a+n<=high:return raw[o+a-low:o+a-low+n]
 return None
def string(a):
 for low,high,o,execute in sections:
  if not execute and low<=a<high:
   p=o+a-low;end=raw.find(b'\0',p,min(p+120,o+high-low))
   if end>p:
    s=raw[p:end]
    if all(32<=c<127 for c in s):return s.decode('ascii')
 return None
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer');from gk.graph import Graph
g=Graph(dict(game='MadWorld',authority='Conservative straight-line PPC constant references; leads, not proven resource consumers'));rows=[];seen=set();c=capstone.Cs(capstone.CS_ARCH_PPC,capstone.CS_MODE_32|capstone.CS_MODE_BIG_ENDIAN);c.skipdata=True
for line in (ROOT/'tools/ghidra-functions.tsv').read_text().splitlines():
 a,z,name=line.split('\t');a,z=int(a,16),int(z,16)
 if not 0x8028b000<=a<0x802a0000:continue
 data=read(a,z)
 if data is None:continue
 regs={'r2':0x80720720,'r13':0x8071f720}
 for ins in c.disasm(data,a):
  args=[x.strip() for x in ins.op_str.split(',')];m=ins.mnemonic
  try:
   if m in ('lis','li'):regs[args[0]]=(int(args[1],0)*(65536 if m=='lis' else 1))&0xffffffff
   elif m=='mr':regs[args[0]]=regs[args[1]]
   elif m in ('addi','addis','ori','oris'):
    v=int(args[2],0)*(65536 if m in ('addis','oris') else 1);regs[args[0]]=((regs[args[1]]|v) if m in ('ori','oris') else (regs[args[1]]+v))&0xffffffff
   elif m=='lwz':
    match=re.fullmatch(r'(-?(?:0x[0-9a-f]+|\d+))\((r\d+)\)',args[1]);address=(regs[match[2]]+int(match[1],0))&0xffffffff;v=read(address,4)
    if v is None:raise KeyError()
    regs[args[0]]=struct.unpack('>I',v)[0]
   elif m.startswith('b'):
    regs={k:v for k,v in regs.items() if k in ('r2','r13')}
   elif args and args[0].startswith('r') and not m.startswith(('st','cmp','mt')):regs.pop(args[0],None)
  except (KeyError,ValueError,TypeError):
   if args:regs.pop(args[0],None)
  for register,value in regs.items():
   s=string(value);magic={0x5041434b:'PACK',0x4c475400:'LGT',0x656d6800:'emh'}.get(value)
   if s and 'em0440.dat' in s.lower():
    key=(a,value)
    if key in seen:continue
    seen.add(key);row=dict(function=name,address=a,site=ins.address,register=register,pointer=value,string=s,magic=magic);rows.append(row);fid='fn:'+hex(a);sid='constant:'+hex(value);g.add_node(fid,'native-function',name,address=a);g.add_node(sid,'native-constant',s or magic,address=value);g.add_edge(fid,sid,'references-lead',site=ins.address)
g.save(ROOT/'research/player-competition-leads.graph.jsonl');(ROOT/'research/player-competition-leads.json').write_text(json.dumps(rows,indent=2));print(json.dumps({'leads':len(rows),'functions':sorted(set(hex(r['address']) for r in rows))}))
