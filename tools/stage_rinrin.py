from pathlib import Path
import json,sys,hashlib,shutil
r=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');e=Path('D:/Games/gamesfordecomp/exports/madworld');sys.path.insert(0,str(e/'builders'))
from native import dat_entries,motion,mdb
manifest=[];clips=[]
source=e/'extracted/disc/DATA/files/em/em0440.dat';raw=source.read_bytes();entries=dat_entries(raw)
for x in entries:
 if x['name'].endswith('.mot') and x['size']:
  data=raw[x['offset']:x['offset']+x['size']];clip=motion(data);clip.update(id=f"em/em0440.dat#{x['name']}@{x['offset']:x}",name=x['name']);clips.append(clip)
  manifest.append(dict(name=x['name'],offset=x['offset'],size=x['size'],sha256=hashlib.sha256(data).hexdigest()))
model=next(x for x in entries if x['name']=='em0440.mdb');decoded=mdb(raw[model['offset']:model['offset']+model['size']]);bones=decoded['bones']
(r/'assets/native/rinrin-motion.json').write_text(json.dumps(dict(bones=bones,clips=clips),separators=(',',':')),encoding='utf-8')
report=json.loads((e/'evidence/node-export-em0440.json').read_text());src=Path(report['path']);shutil.copy2(src,r/'assets/native/rinrin-color.glb')
sourceReport=json.loads((e/'evidence/node-export-em0440-source.json').read_text());shutil.copy2(sourceReport['path'],r/'assets/native/rinrin-source.glb')
(r/'research/staging-manifest.json').write_text(json.dumps(dict(source=str(source),sourceSha256=hashlib.sha256(raw).hexdigest(),motions=manifest,geometry=dict(source=str(src),sha256=hashlib.sha256(src.read_bytes()).hexdigest(),validation=report['validation']),authority='Native decoded samples; 60 fps is inspection timing, original game timing unresolved; GLB standard material fallback is not original GX rendering'),indent=2),encoding='utf-8')
print(dict(clips=len(clips),bones=len(bones),frames=sum(x['frames'] for x in clips)))
