"""Index disc resources without guessing actor identities or action dispatch."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import sys
from native import dat_entries, Reader

sys.path.insert(0,str(Path(__file__).resolve().parents[2]))
from gk.graph import Graph


def build(root):
    root=Path(root).resolve(); source=root/'extracted/disc/DATA/files'
    (root/'evidence/scripts').mkdir(parents=True,exist_ok=True)
    graph=Graph(dict(game='MadWorld',authority='native DAT directory containment; no gameplay selection decoded'))
    banks=[]; models=[]; clips=[]; failures=[]; skipped=[]; kinds=Counter()
    for path in sorted(source.rglob('*.dat')):
        data=path.read_bytes(); rel=path.relative_to(source).as_posix()
        if not data:
            skipped.append(dict(path=rel,reason='empty disc file'));continue
        try: entries=dat_entries(data)
        except Exception as ex:
            failures.append(dict(path=rel,error=str(ex)));continue
        bank=dict(id=rel,path=str(path),sha256=hashlib.sha256(data).hexdigest(),entries=entries)
        banks.append(bank)
        graph.add_node('bank:'+rel,'bank',rel,sha256=bank['sha256'])
        for entry in entries:
            name=entry['name']; ext=Path(name).suffix.lower();kinds[ext]+=1
            if ext not in ('.mdb','.mot','.tpl'):continue
            if not entry['size']:
                skipped.append(dict(path=rel+'#'+name,reason='zero-size directory member'));continue
            b=data[entry['offset']:entry['offset']+entry['size']]; r=Reader(b)
            resource=dict(id=rel+'#'+name+'@'+str(entry['offset']),bank=rel,name=name,offset=entry['offset'],size=entry['size'],sha256=hashlib.sha256(b).hexdigest())
            try:
                if ext=='.mdb':
                    nb=r.u32(44)
                    resource.update(bones=nb,flags=r.get('H',2)[0],texture=None,identity='Unreviewed native model',review='unreviewed')
                    stem=Path(name).stem
                    textures=[x['name'] for x in entries if x['name'].endswith('.tpl')]
                    exact=stem+'.tpl'
                    if exact in textures:resource['texture']=exact
                    elif len(textures)==1:resource['texture']=textures[0]
                    resource['textureBinding']='same stem' if exact in textures else 'sole TPL in archive' if len(textures)==1 else 'unresolved'
                    models.append(resource)
                elif ext=='.mot':
                    flag,frames,off,n=r.get('hhII',4)
                    tracks=[r.get('hBBhhI',off+i*12) for i in range(n)]
                    resource.update(frames=frames,bones=sorted(set(t[0] for t in tracks if t[0]!=32767 and t[2]!=255)),encodings=sorted(set(t[2] for t in tracks)),channels=len(tracks),reviewedLabel=None)
                    clips.append(resource)
                graph.add_node('resource:'+resource['id'],ext[1:],name,**resource)
                graph.add_edge('bank:'+rel,'resource:'+resource['id'],'contains',offset=entry['offset'],size=entry['size'])
            except Exception as ex:failures.append(dict(path=resource['id'],error=str(ex)))
    index=dict(version=1,banks=banks,models=models,clips=clips,failures=failures,skipped=skipped,kinds=dict(kinds))
    (root/'catalog.json').write_text(json.dumps(index,separators=(',',':')),encoding='utf-8')
    graph.save(root/'evidence/resources.graph.jsonl')
    (root/'evidence/INDEX.md').write_text(f'# MadWorld native resources\n\n{len(banks)} DAT banks, {len(models)} MDB resources, {len(clips)} MOT resources.\n\nThis graph records source containment only. Identity, ownership, action selection and paired dispatch are unresolved.\n',encoding='utf-8')
    (root/'evidence/API.md').write_text('# Resource vocabulary\n\n`bank contains mdb/mot/tpl`: exact DAT table containment. Does not mean authored ownership or gameplay eligibility. No script API or paired dispatch is decoded yet.\n',encoding='utf-8')
    (root/'evidence/scripts/resource-loader.md').write_text('# Resource loader pseudocode\n\nAdapter inspection policy, not recovered game code:\n\n1. Read bounded DAT directory.\n2. Index each resource by archive, member, range and SHA-256.\n3. Associate exact-stem TPL; otherwise expose sole-archive TPL as a candidate.\n4. Keep other texture relationships unresolved.\n5. Same-bank MOT containment is browseable separately from rig compatibility.\n',encoding='utf-8')
    (root/'evidence/inventory.json').write_text(json.dumps(dict(banks=len(banks),models=len(models),clips=len(clips),failures=failures,kinds=dict(kinds)),indent=2),encoding='utf-8')
    from review import review
    from labels import apply
    review(root)
    apply(root)
    package(root)
    print(json.dumps(dict(banks=len(banks),models=len(models),clips=len(clips),failures=len(failures),kinds=dict(kinds)),indent=2))


def package(root):
    here=Path(__file__).parent
    for name in ('index.html','app.js','export.js','color.js','ink.js'):
        if (here/name).exists():shutil.copy2(here/name,root/name)
    if (here/'app.js').exists():
        exportVersion=hashlib.sha256((root/'export.js').read_bytes()).hexdigest()[:12]
        app=(root/'app.js').read_text(encoding='utf-8').replace("'./export.js'","'./export.js?v="+exportVersion+"'")
        colorVersion=hashlib.sha256((root/'color.js').read_bytes()).hexdigest()[:12]
        app=app.replace("'./color.js'","'./color.js?v="+colorVersion+"'")
        inkVersion=hashlib.sha256((root/'ink.js').read_bytes()).hexdigest()[:12]
        app=app.replace("'./ink.js'","'./ink.js?v="+inkVersion+"'")
        (root/'app.js').write_text(app,encoding='utf-8')
        appVersion=hashlib.sha256(app.encode()).hexdigest()[:12]
        html=(root/'index.html').read_text(encoding='utf-8').replace('src="./app.js"','src="./app.js?v='+appVersion+'"')
        (root/'index.html').write_text(html,encoding='utf-8')
    for name in ('README.md','THIRD-PARTY.txt'):
        if (here/name).exists():shutil.copy2(here/name,root/('ADAPTER.md' if name=='README.md' else name))
    builders=root/'builders'
    builders.mkdir(exist_ok=True)
    for module in here.glob('*.py'):
        if module.resolve()!=(builders/module.name).resolve():shutil.copy2(module,builders/module.name)
    graphdir=builders/'gk'
    graphdir.mkdir(exist_ok=True)
    (graphdir/'__init__.py').write_text('',encoding='utf-8')
    toolkit=here.parents[1]/'gk/graph.py'
    if toolkit.exists():shutil.copy2(toolkit,graphdir/'graph.py')
    (root/'Launch MadWorld.cmd').write_text('@echo off\ncd /d "%~dp0"\npython "%~dp0builders/serve.py" "%~dp0." --port 8781\npause\n',encoding='utf-8')
    vendor=root/'vendor/three'
    if not vendor.exists():
        shutil.copytree(root.parent/'asian-dynamite/vendor/three',vendor)
    if not (vendor/'addons/utils/SkeletonUtils.js').exists():
        import urllib.request
        urllib.request.urlretrieve('https://raw.githubusercontent.com/mrdoob/three.js/r175/examples/jsm/utils/SkeletonUtils.js',vendor/'addons/utils/SkeletonUtils.js')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root');a=p.parse_args();build(a.root)
