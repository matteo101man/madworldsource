"""Stage one original DAT rig/motion bank and independently audited GLB variants."""
from pathlib import Path
import argparse, hashlib, json, shutil, sys

ROOT=Path(__file__).resolve().parents[1]
EXPLORER=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPLORER/'builders'))
from native import dat_entries, mdb, motion

def stage(stem, name, bank=None):
    # bank: the gameplay DAT when the model lives in another stem's bank (em0615.dat holds Mathilda's em0610.mdb)
    bank=bank or stem
    source=EXPLORER/'extracted/disc/DATA/files'/bank[:2]/(bank+'.dat')
    raw=source.read_bytes(); entries=dat_entries(raw); clips=[]; resources=[]
    model=next(x for x in entries if x['name']==stem+'.mdb')
    decoded=mdb(raw[model['offset']:model['offset']+model['size']])
    for entry in entries:
        if not entry['size'] or not entry['name'].endswith('.mot'): continue
        data=raw[entry['offset']:entry['offset']+entry['size']]
        clip=motion(data)
        clip.update(id=f"{bank[:2]}/{bank}.dat#{entry['name']}@{entry['offset']:x}",name=entry['name'])
        clips.append(clip)
        resources.append(dict(name=entry['name'],offset=entry['offset'],size=entry['size'],sha256=hashlib.sha256(data).hexdigest()))
    geometry=[]
    for mode,suffix in [('source','-source'),('color','')]:
        report=json.loads((EXPLORER/f'evidence/node-export-{stem}{suffix}.json').read_text())
        src=Path(report['path']); assert hashlib.sha256(src.read_bytes()).hexdigest()==report['sha256']
        dst=ROOT/f'assets/native/{name}-{mode}.glb';shutil.copy2(src,dst)
        geometry.append(dict(mode=mode,path=str(dst),source=str(src),sha256=report['sha256'],validation=report['validation']))
    (ROOT/f'assets/native/{name}-motion.json').write_text(json.dumps(dict(bones=decoded['bones'],clips=clips),separators=(',',':')))
    result=dict(source=str(source),sourceSha256=hashlib.sha256(raw).hexdigest(),model=model,motions=resources,geometry=geometry,
                authority='Original rig and sampled MOT data; active-palette GLB export/reimport audits. No proof of original clock, native movement/controller behavior, materials or shared-bank eligibility.')
    (ROOT/f'research/{name}-staging-manifest.json').write_text(json.dumps(result,indent=2))
    print(dict(actor=name,bones=len(decoded['bones']),clips=len(clips),frames=sum(c['frames'] for c in clips)))

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('stem');p.add_argument('name');p.add_argument('--bank');a=p.parse_args();stage(a.stem,a.name,a.bank)
