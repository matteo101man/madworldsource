from pathlib import Path
import json,subprocess,concurrent.futures,hashlib,sys
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph
r=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');src=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files');tool=r/'tools/vgmstream/vgmstream-cli.exe'
def probe(p):
 out=subprocess.run([str(tool),'-m','-I',str(p)],capture_output=True,text=True)
 try:meta=json.loads(out.stdout);return dict(source=p.relative_to(src).as_posix(),sha256=hashlib.sha256(p.read_bytes()).hexdigest(),metadata=meta)
 except Exception:return dict(source=p.relative_to(src).as_posix(),error=out.stderr.strip() or out.stdout.strip())
paths=sorted(src.rglob('*.rsd'))
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(probe,paths))
g=Graph(dict(game='MadWorld',authority='vgmstream r2168 metadata; event selection and mixing not established'))
for x in rows:g.add_node('audio:'+x['source'],'audio-stream',x['source'],**x)
g.save(r/'research/audio.graph.jsonl');(r/'research/audio.json').write_text(json.dumps(rows,indent=2))
(r/'research/AUDIO-API.md').write_text('# Audio stream catalog\n\n`audio-stream` contains original relative path, source SHA256 and unmodified vgmstream r2168 JSON metadata. Sample rate, channels, sample count and loops are decoder observations. Names are source names, not verified gameplay roles. Runtime event IDs and mixing remain separate requirements.\n',encoding='utf-8')
print(dict(streams=len(rows),decoded=sum('metadata' in x for x in rows),errors=[x for x in rows if 'error' in x]))
