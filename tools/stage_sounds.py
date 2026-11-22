"""Bounded native PACK framing, delegated RedSpark decoding via upstream vgmstream."""
from pathlib import Path
import struct,json,hashlib,subprocess,sys,concurrent.futures
ROOT=Path(__file__).resolve().parents[1];SOURCE=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files');sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph

def members(data):
 if data[:4]!=b'PACK':raise ValueError('PACK magic')
 n=struct.unpack_from('>I',data,4)[0]
 if n>100000 or 8+n*8>len(data):raise ValueError('PACK directory')
 rows=[]
 for i in range(n):
  off,size=struct.unpack_from('>II',data,8+i*8)
  if size and (off<8+n*8 or off+size>len(data)):raise ValueError('PACK member bounds')
  rows.append((off,size))
 positive=sorted(set((o,o+s) for o,s in rows if s))
 if any(b[0]<a[1] for a,b in zip(positive,positive[1:])):raise ValueError('PACK overlapping members')
 return rows

def audit():
 allrows=[];failures=[]
 def walk(data,source,offset=0,path=(),depth=0):
  if depth>8:raise ValueError('PACK depth')
  for i,(off,size) in enumerate(members(data)):
   if not size:continue
   child=data[off:off+size];row=dict(source=source,path=list(path+(i,)),offset=offset+off,size=size,sha256=hashlib.sha256(child).hexdigest(),pack=child[:4]==b'PACK');allrows.append(row)
   if row['pack']:walk(child,source,offset+off,path+(i,),depth+1)
 for p in sorted((SOURCE/'se').rglob('*.rsp')):
  try:walk(p.read_bytes(),p.relative_to(SOURCE).as_posix())
  except Exception as ex:failures.append(dict(source=str(p),error=str(ex)))
 (ROOT/'research/sound-packs.json').write_text(json.dumps(dict(members=allrows,failures=failures),indent=2))
 print(dict(members=len(allrows),leaves=sum(not x['pack'] for x in allrows),failures=failures))
 return allrows

def stage(rows):
 out=ROOT/'assets/native/audio';out.mkdir(parents=True,exist_ok=True);g=Graph(dict(game='MadWorld',authority='Bounded PACK source ranges; vgmstream r2168 RedSpark decoder; bank IDs/timing not yet routed'))
 chosen=[x for x in rows if x['source'] in ('se/scene_mix0000.rsp','se/scene_mix0201.rsp') and not x['pack']]
 def probe(row):
  raw=(SOURCE/row['source']).read_bytes()[row['offset']:row['offset']+row['size']];name=Path(row['source']).stem+'-'+ '-'.join(map(str,row['path']));path=out/(name+'.rsd');path.write_bytes(raw)
  result=subprocess.run([str(ROOT/'tools/vgmstream/vgmstream-cli.exe'),'-m','-I',str(path)],capture_output=True,text=True)
  row=dict(row,staged=path.relative_to(ROOT/'assets').as_posix())
  try:
   row['metadata']=json.loads(result.stdout);total=max(1,row['metadata']['streamInfo']['total']);row['wav']=[]
   for sub in range(1,total+1):
    wav=out/(name+f'-{sub:04}.wav');cmd=[str(ROOT/'tools/vgmstream/vgmstream-cli.exe'),'-i','-L','-s',str(sub),'-o',str(wav),str(path)];decoded=subprocess.run(cmd,capture_output=True,text=True)
    if decoded.returncode:raise ValueError(decoded.stderr)
    row['wav'].append(wav.relative_to(ROOT/'assets').as_posix())
  except Exception as ex:row['decodeStatus']='not playable by tool (may be bus/config)';row['toolMessage']=result.stderr.strip() or str(ex)
  return row
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:staged=list(pool.map(probe,chosen))
 for x in staged:g.add_node('sound-member:'+x['source']+':'+'.'.join(map(str,x['path'])),'sound-member','/'.join(map(str,x['path'])),**x)
 g.save(ROOT/'research/arena-sounds.graph.jsonl');(ROOT/'research/arena-sounds.json').write_text(json.dumps(staged,indent=2));print(dict(staged=len(staged),playable=sum('metadata' in x for x in staged),wav=sum(len(x.get('wav',[])) for x in staged)))

if __name__=='__main__':stage(audit())
