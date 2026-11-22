"""Stage validated event prop models, rigs and original mesh-name bindings (MW_EVENT, default ev21)."""
from pathlib import Path
import hashlib,json,os,shutil,struct,sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPORT/'builders'))
from native import dat_entries,mdb

def glb_names(path):
    raw=path.read_bytes()
    if raw[:4]!=b'glTF':raise ValueError('GLB magic')
    size,kind=struct.unpack_from('<II',raw,12)
    if kind!=0x4e4f534a:raise ValueError('GLB JSON chunk')
    data=json.loads(raw[20:20+size])
    return [n.get('name','')for n in data['nodes']]

def main():
    ev=os.environ.get('MW_EVENT','ev21')
    folder=ROOT/'assets/native/events'/ev;folder.mkdir(parents=True,exist_ok=True)
    actors=[]
    event=json.loads((folder/'event.json').read_text())
    stems=['pl0030','pl0040','wp0600']+sorted({a['object'] for a in event['actors']}-{'pl0030','pl0040','wp0600'})
    for stem in stems:
        path=EXPORT/'extracted/disc/DATA/files'/stem[:2]/(stem+'.dat');raw=path.read_bytes()
        model=next(e for e in dat_entries(raw)if e['name']==stem+'.mdb')
        decoded=mdb(raw[model['offset']:model['offset']+model['size']])
        if stem=='pl0030':
            glb=ROOT/'assets/native/jack-player-source.glb';validation=json.load(open(ROOT/'research/jack-player-staging-manifest.json'))['geometry'][0]['validation']
        else:
            exported=EXPORT/f'evidence/node-export-{stem}-source.json'
            if not exported.is_file():print(f'skipping {stem}: no explorer export');continue
            report=json.load(open(exported));glb=Path(report['path']);validation=report['validation']
            if hashlib.sha256(glb.read_bytes()).hexdigest()!=report['sha256']:raise ValueError('Prop GLB hash')
            destination=folder/(stem+'.glb');shutil.copy2(glb,destination);glb=destination
            # the colour look's copy beside it (the explorer's added inspection palette), picked by play_intro.rs
            color=EXPORT/f'evidence/node-export-{stem}.json'
            if color.is_file():
                c=json.load(open(color));cp=Path(c['path'])
                if hashlib.sha256(cp.read_bytes()).hexdigest()==c['sha256']:shutil.copy2(cp,folder/(stem+'-color.glb'))
        names=glb_names(glb)
        expected=[f'role_0_mesh_{i}'for i in range(len(decoded['meshes']))]
        if sorted(n for n in names if n.startswith('role_0_mesh_'))!=sorted(expected):raise ValueError('Export mesh ordinal mapping')
        if any(f'role_0_bone{b["id"]}'not in names for b in decoded['bones']):raise ValueError('Export bone mapping')
        actors.append(dict(object=stem,glb=glb.relative_to(ROOT/'assets').as_posix(),bones=decoded['bones'],
                           meshes=[dict(node=expected[i],name=m['name'])for i,m in enumerate(decoded['meshes'])],
                           source=dict(path=str(path),sha256=hashlib.sha256(raw).hexdigest(),model=model),
                           glbSha256=hashlib.sha256(glb.read_bytes()).hexdigest(),validation=validation))
    (folder/'models.json').write_text(json.dumps(dict(schema='mw-event-models/1',actors=actors,
        scope='Original rigs/mesh names and audited GLB mappings. Native attachment and command consumers are translated separately.'),indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(actors=len(actors),meshes=sum(len(a['meshes'])for a in actors))))

if __name__=='__main__':main()
