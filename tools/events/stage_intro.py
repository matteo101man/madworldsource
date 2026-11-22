"""Stage the exact English event tracks (MW_EVENT, default ev21: RinRin's intro); preserve unresolved commands.

Does not assign meanings to camera sample components or execute SEQ commands.
"""
from pathlib import Path
import hashlib, json, os, struct, sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPORT/'builders'))
from event import chunks, sequence
from native import camera, motion

def main():
    # MW_EVENT=ev33 stages another event (default ev21, RinRin's intro)
    ev=os.environ.get('MW_EVENT','ev21')
    source=EXPORT/'extracted/disc/DATA/files/event'/f'{ev}_us.dat'
    raw=source.read_bytes();tree=chunks(raw)
    if len(tree)!=1 or tree[0]['tag']!='EVNT':raise ValueError('Expected one EVNT container')
    def provenance(node,skip=0):
        start=node['payload']+skip;size=node['end']-start
        return dict(offset=start,bytes=size,sha256=hashlib.sha256(raw[start:start+size]).hexdigest())
    def commands(node):
        seq=[c for c in node['children']if c['tag']=='SEQ ']
        if len(seq)>1:raise ValueError('Multiple sequence chunks')
        return sequence(raw,seq[0]['payload'],seq[0]['end']) if seq else []
    cameras=[];actors=[];resources=[];camera_commands=[]
    for node in tree[0]['children']:
        if node['tag']=='CAM ':
            if cameras:raise ValueError('Duplicate camera bank')
            camera_commands=commands(node)
            for child in node['children']:
                if child['tag']!='MOT ':continue
                info=camera(raw[child['payload']:child['end']])
                cameras.append(dict(frames=info['frames'],flags=info['headerField04'],samples=info['samples'],source=provenance(child)))
        elif node['tag']=='OBJ ':
            clips=[]
            for child in node['children']:
                if child['tag']!='MOT ':continue
                cut=struct.unpack_from('>I',raw,child['payload'])[0]
                info=motion(raw[child['payload']+4:child['end']])
                clips.append(dict(cut=cut,source=provenance(child,4),clip=dict(
                    id=f"{ev}/{node['object']}/{node['instance']}/{cut}",
                    name=f"{node['object']}_{ev}_cut{cut}.mot",frames=info['frames'],tracks=info['tracks'])))
            actors.append(dict(object=node['object'],instance=node['instance'],motions=clips,commands=commands(node)))
        else:
            resources.append(dict(tag=node['tag'],source=provenance(node),commands=commands(node)))
    folder=ROOT/'assets/native/events'/ev;folder.mkdir(parents=True,exist_ok=True)
    data=dict(schema='mw-native-event/1',event=int(ev[2:],16),sourceFile=f'event/{ev}_us.dat',sourceSha256=hashlib.sha256(raw).hexdigest(),
              sourceBytes=len(raw),cameras=cameras,cameraCommands=camera_commands,actors=actors,resources=resources,
              scope='Original camera samples, actor/cut MOTs and unexecuted SEQ commands. Playback cadence, attachments, audio and camera component semantics require native consumers.')
    (folder/'event.json').write_text(json.dumps(data,separators=(',',':'))+'\n',encoding='utf-8')
    (folder/'source.dat').write_bytes(raw)
    print(json.dumps(dict(cuts=len(cameras),frames=sum(c['frames']for c in cameras),actors=len(actors),motions=sum(len(a['motions'])for a in actors),bytes=(folder/'event.json').stat().st_size)))

if __name__=='__main__':main()
