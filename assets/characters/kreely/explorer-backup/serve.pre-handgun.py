"""Local, content-keyed MadWorld resource service. No shared seek handles."""
import argparse
from functools import partial
import hashlib
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import io
import json
from pathlib import Path
import threading
from urllib.parse import parse_qs, urlparse
from native import mdb, motion, tpl, dat_entries
from build import package
from event import camera as event_camera

VERSION='mw-reader-5-native-ink-fields'


def serve(root,port):
    root=Path(root).resolve();package(root)
    catalog=json.loads((root/'catalog.json').read_text(encoding='utf-8'))
    events=json.loads((root/'events.json').read_text(encoding='utf-8')) if (root/'events.json').exists() else dict(events=[],clips=[],failures=[])
    materialPath=root/'evidence/material-programs.json'
    materialPrograms=json.loads(materialPath.read_text(encoding='utf-8')) if materialPath.exists() else {}
    models={x['id']:x for x in catalog['models']}; clips={x['id']:x for x in catalog['clips']+events['clips']}
    banks={x['id']:x for x in catalog['banks']}
    lock=threading.RLock(); cache=root/'cache';cache.mkdir(exist_ok=True)

    def custom_models():
        # Added non-disc characters (custom/characters.json): local model JSON + texture, verified by sha256.
        path=root/'custom/characters.json'
        return json.loads(path.read_text(encoding='utf-8'))['characters'] if path.exists() else []

    def custom_file(item,kind):
        data=Path(item['custom'][kind]).read_bytes()
        if hashlib.sha256(data).hexdigest()!=item['custom'][kind+'Sha256']:raise ValueError('custom '+kind+' changed; rebuild '+item['id'])
        return data

    def lookup(id):
        return models.get(id) or next(m for m in custom_models() if m['id']==id)

    def member(item):
        with open(item.get('sourcePath') or banks[item['bank']]['path'],'rb') as f:
            f.seek(item['offset']);data=f.read(item['size'])
        if hashlib.sha256(data).hexdigest()!=item['sha256']:raise ValueError('source changed; rebuild catalog')
        return data

    class Handler(SimpleHTTPRequestHandler):
        def __init__(self,*a,**kw):super().__init__(*a,directory=str(root),**kw)
        def log_message(self,*a):pass
        def end_headers(self):
            self.send_header('Cache-Control','no-store')
            super().end_headers()
        def do_POST(self):
            if self.path!='/api/export':
                self.send_error(404);return
            size=int(self.headers.get('Content-Length','0'))
            if not 12<=size<=256*1024*1024:
                self.send_error(413);return
            data=self.rfile.read(size)
            if data[:4]!=b'glTF' or int.from_bytes(data[8:12],'little')!=size:
                self.send_error(422);return
            exports=root/'exports';exports.mkdir(exist_ok=True)
            name=hashlib.sha256(data).hexdigest()[:20]+'.glb';dest=exports/name
            dest.write_bytes(data)
            validation=json.loads(self.headers.get('X-MadWorld-Validation','{}'))
            validation.update(sha256=hashlib.sha256(data).hexdigest(),file=str(dest))
            dest.with_suffix('.validation.json').write_text(json.dumps(validation,indent=2),encoding='utf-8')
            response=json.dumps(dict(path=str(dest),url='exports/'+name,sha256=hashlib.sha256(data).hexdigest())).encode()
            self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(response)));self.end_headers();self.wfile.write(response)
        def do_GET(self):
            u=urlparse(self.path);q=parse_qs(u.query)
            if not u.path.startswith('/api/'):
                return super().do_GET()
            try:
                if u.path=='/api/catalog':
                    fresh=json.loads((root/'catalog.json').read_text(encoding='utf-8'))
                    validationPath=root/'evidence/structural-validation.json'
                    validation=json.loads(validationPath.read_text(encoding='utf-8')) if validationPath.exists() else {}
                    coverage=dict(indexedModels=len(fresh['models']),nonemptyMotions=len(fresh['clips']),structuralFailures=validation.get('failures',[]),reviewedDescriptions=sum(bool(c.get('reviewedLabel')) for c in fresh['clips']),verifiedNativePairs=0,rosterAudit='70 unique >15-bone resources reviewed; small/unrigged/pre-rendered coverage open',eventAnimations=dict(events=len(events['events']),decodedMotions=len(events['clips']),failures=events['failures'],authority=events.get('authority')),nativeIdentityLinks='Naomi/Amala linked by native subtitle and cut context; unnamed models remain explicit',shader='native source-flag TEV programs recovered; preview lighting, shadow and konst inputs still incomplete',colorVersion='optional added palette for every model and event role; original source palette remains default')
                    for model in fresh['models']:
                        if model.get('custom'):continue
                        candidate=(model['texture'] or '').removesuffix('.tpl')+'_AB.tpl'
                        model['textureAB']=candidate if any(e['name']==candidate and e['size'] for e in banks[model['bank']]['entries']) else None
                    for model in custom_models():model['textureAB']=Path(model['custom']['abTexture']).name if model['custom'].get('abTexture') else None;fresh['models'].append(model)
                    data=json.dumps(dict(models=fresh['models'],clips=fresh['clips'],events=events,materialPrograms=materialPrograms,failures=fresh['failures'],kinds=fresh['kinds'],coverage=coverage),separators=(',',':')).encode();mime='application/json'
                elif u.path=='/api/event-camera':
                    event=next(e for e in events['events'] if e['id']==q['id'][0]);cut=event['cuts'][int(q['cut'][0])]
                    raw=Path(event['path']).read_bytes()
                    if hashlib.sha256(raw).hexdigest()!=event['sha256']:raise ValueError('event source changed')
                    data=json.dumps(event_camera(raw[cut['offset']:cut['offset']+cut['size']]),allow_nan=False,separators=(',',':')).encode();mime='application/json'
                elif u.path=='/api/model' and lookup(q['id'][0]).get('custom'):
                    data=custom_file(lookup(q['id'][0]),'model');mime='application/json'
                elif u.path=='/api/model':
                    item=models[q['id'][0]];key=item['sha256']+'-'+VERSION+'.json';dest=cache/key
                    with lock:
                        if not dest.exists():dest.write_text(json.dumps(mdb(member(item)),separators=(',',':'),allow_nan=False),encoding='utf-8')
                    data=dest.read_bytes();mime='application/json'
                elif u.path=='/api/clip':
                    item=clips[q['id'][0]];key=item['sha256']+'-'+VERSION+'.motion.json';dest=cache/key
                    with lock:
                        if not dest.exists():dest.write_text(json.dumps(motion(member(item)),separators=(',',':'),allow_nan=False),encoding='utf-8')
                    data=dest.read_bytes();mime='application/json'
                elif u.path=='/api/texture' and lookup(q['id'][0]).get('custom'):
                    item=lookup(q['id'][0]);kind={'primary':'texture','ab':'abTexture','color':'colorTexture'}[q.get('bank',['primary'])[0]]
                    if kind not in item['custom']:raise ValueError('custom character has no '+kind)
                    data=custom_file(item,kind);mime='image/png' if item['custom'][kind].lower().endswith('.png') else 'image/jpeg'
                elif u.path=='/api/texture':
                    item=models[q['id'][0]]
                    if not item['texture']:raise ValueError('unresolved texture binding')
                    entries=banks[item['bank']]['entries'];name=item['texture']
                    if q.get('bank',['primary'])[0]=='ab':name=name.removesuffix('.tpl')+'_AB.tpl'
                    entry=next(e for e in entries if e['name']==name)
                    with open(banks[item['bank']]['path'],'rb') as f:f.seek(entry['offset']);raw=f.read(entry['size'])
                    key=hashlib.sha256(raw).hexdigest()+'-'+VERSION+'-'+q.get('index',['0'])[0]+'.png';dest=cache/key
                    with lock:
                        if not dest.exists():
                            images=tpl(raw,int(q.get('index',['0'])[0]));images[0][0].save(dest)
                    data=dest.read_bytes();mime='image/png'
                else:raise ValueError('unknown API')
                self.send_response(200);self.send_header('Content-Type',mime);self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
            except Exception as ex:
                data=json.dumps(dict(error=str(ex))).encode();self.send_response(422);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
    server=ThreadingHTTPServer(('127.0.0.1',port),Handler)
    print(f'MadWorld explorer http://127.0.0.1:{port}/',flush=True);server.serve_forever()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root');p.add_argument('--port',type=int,default=8781);a=p.parse_args();serve(a.root,a.port)
