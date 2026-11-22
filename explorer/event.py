"""Bounded tagged event chunks; unknown payloads remain source ranges."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import sys
from native import motion, camera
sys.path.insert(0,str(Path(__file__).resolve().parents[2]))
from gk.graph import Graph


def chunks(data,start=0,end=None):
    end=len(data) if end is None else end;out=[];cursor=start
    while cursor<end:
        if cursor+8>end:raise ValueError('truncated chunk header')
        tag=data[cursor:cursor+4]
        if not re.fullmatch(rb'[A-Z0-9_ ]{4}',tag):raise ValueError('not tagged chunk list')
        size=struct.unpack_from('>I',data,cursor+4)[0]
        if size>end-cursor-8:raise ValueError('chunk exceeds parent')
        node=dict(tag=tag.decode(),offset=cursor,size=size,payload=cursor+8,end=cursor+8+size)
        try:node['children']=chunks(data,cursor+8,cursor+8+size) if size else []
        except ValueError:node['children']=[]
        if tag==b'OBJ ' and size>=12:
            name=data[cursor+8:cursor+16].split(b'\0')[0].decode('ascii')
            if re.fullmatch(r'[A-Za-z][A-Za-z0-9_]{3,7}',name):
                node['object']=name;node['instance']=struct.unpack_from('>I',data,cursor+16)[0]
                node['children']=chunks(data,cursor+20,cursor+8+size)
        out.append(node);cursor+=8+size
    return out


def sequence(data,start,end):
    """Counted variable-size commands. Preserve unknown opcodes and operands."""
    if end-start<4:raise ValueError('short sequence')
    count=struct.unpack_from('>I',data,start)[0];cursor=start+4;out=[]
    if count>(end-cursor)//12:raise ValueError('sequence count exceeds range')
    for _ in range(count):
        if cursor+12>end:raise ValueError('short sequence command')
        op,size,frame=struct.unpack_from('>III',data,cursor)
        if size<12 or size%4 or cursor+size>end:raise ValueError('sequence command bounds')
        out.append(dict(offset=cursor,opcode=op,size=size,frame=frame,
                        operands=list(struct.unpack_from('>'+str((size-12)//4)+'I',data,cursor+12))))
        cursor+=size
    if any(data[cursor:end]):raise ValueError('nonzero sequence tail')
    return out


def index_motions(root):
    """Index native actor/cut bindings independently from gameplay DAT banks."""
    root=Path(root);source=root/'extracted/disc/DATA/files/event';catalog=json.loads((root/'catalog.json').read_text(encoding='utf-8'))
    models=catalog['models'];out=[];clips=[];failures=[];g=Graph(dict(game='MadWorld',authority='Native actor/cut bindings, SEQ framing and base cut-step arithmetic; cadence and command timing controls unverified'))
    g.add_node('native:event-clock','event-clock','Native base cut clock',capacity=64,cadence='unverified',commandTimingControls='unexecuted')
    for address,name in [('800a4ba8','build clock from camera cuts'),('800a4c1c','read camera frame count u16+6'),('800a6efc','append positive cut length'),('800a6f60','advance native cut clock')]:
        g.add_node('fn:'+address,'native-function',name,address=address);g.add_edge('native:event-clock','fn:'+address,'implemented-by')
    for path in sorted(source.glob('*.dat')):
        if re.search(r'_(?:fr|it|sp|us)\.dat$',path.name):continue
        raw=path.read_bytes()
        if not raw:continue
        try:
            tree=chunks(raw);nodes=tree[0]['children'];cams=[];actors=[];cursor=0
            eid=path.stem;g.add_node(eid,'event',eid,path=str(path),sha256=hashlib.sha256(raw).hexdigest())
            g.add_edge(eid,'native:event-clock','clocked-by')
            for n in nodes:
                if n['tag']=='CAM ':
                    for c in n['children']:
                        if c['tag']!='MOT ':continue
                        info=camera(raw[c['payload']:c['end']]);cut=len(cams)
                        cams.append(dict(index=cut,start=cursor,frames=info['frames'],headerField04=info['headerField04'],offset=c['payload'],size=c['size']))
                        g.add_node(eid+':cut:'+str(cut),'event-cut',str(cut),**cams[-1]);g.add_edge(eid,eid+':cut:'+str(cut),'contains');cursor+=info['frames']
            for n in nodes:
                if n['tag']!='OBJ ':continue
                name=n['object'];aid=eid+'@'+str(n['offset']);model=next((m for m in models if m['name']==name+'.mdb' and m['bank']==name[:2]+'/'+name+'.dat'),None) or next((m for m in models if m['name']==name+'.mdb'),None)
                a=dict(id=aid,object=name,instance=n['instance'],model=model['id'] if model else None,clips=[],sequence=[])
                g.add_node(aid,'event-actor',name,object=name,instance=n['instance'],model=a['model']);g.add_edge(eid,aid,'contains')
                for c in n['children']:
                    if c['tag']=='SEQ ':
                        a['sequence']=sequence(raw,c['payload'],c['end'])
                        for command in a['sequence']:
                            cid=eid+'@'+str(command['offset']);g.add_node(cid,'event-command','opcode '+str(command['opcode']),**command);g.add_edge(aid,cid,'commands')
                    if c['tag']!='MOT ':continue
                    cut=struct.unpack_from('>I',raw,c['payload'])[0];off=c['payload']+4;b=raw[off:c['end']];d=motion(b)
                    resource=dict(id=aid+':mot@'+str(off),bank='event/'+path.name,name=name+'_'+eid+'_cut'+str(cut)+'.mot',sourcePath=str(path),offset=off,size=len(b),sha256=hashlib.sha256(b).hexdigest(),frames=d['frames'],bones=sorted(set(t['bone'] for t in d['tracks'])),channels=len(d['tracks']),reviewedLabel=None,event=eid,actor=aid,object=name,cut=cut,ownership='native OBJ actor identifier',timelineStart=cams[cut]['start'] if cut<len(cams) else None)
                    clips.append(resource);a['clips'].append(resource['id']);g.add_node(resource['id'],'event-motion',resource['name'],**resource);g.add_edge(aid,resource['id'],'animates');
                    if cut<len(cams):g.add_edge(eid+':cut:'+str(cut),resource['id'],'selects',authority='native MOT cut index; camera correspondence structural')
                actors.append(a)
            out.append(dict(id=eid,path=str(path),sha256=hashlib.sha256(raw).hexdigest(),frames=cursor,cuts=cams,actors=actors))
        except Exception as ex:failures.append(dict(file=path.name,error=str(ex)))
    result=dict(events=out,clips=clips,failures=failures,authority='Native actor/cut bindings and base cut-step arithmetic recovered. Cadence and command timing effects unverified; unknown opcodes preserved, not executed.')
    (root/'events.json').write_text(json.dumps(result,separators=(',',':')),encoding='utf-8');g.save(root/'evidence/event-motions.graph.jsonl')
    (root/'evidence/EVENT-MOTIONS-API.md').write_text('# Event motion vocabulary\n\n`event contains event-actor/event-cut`; `event-actor animates event-motion`: original OBJ ID and MOT cut index. `event-cut selects event-motion`: matching native cut index. `commands`: bounded opcode, size, frame, operands; semantics unresolved. `event clocked-by event-clock`, `event-clock implemented-by native-function`: original camera count reader0x800a4c1c(u16 at+6), clock construction0x800a4ba8/0x800a6efc and advance0x800a6f60 establish base cut-step arithmetic. Update cadence, dynamic pacing/seek commands and physical attachments remain unverified. Missing actor cuts are hidden, not synthesized.\n',encoding='utf-8')
    (root/'evidence/EVENT-MOTIONS-INDEX.md').write_text('# Event motions\n\n'+f'{len(out)} base-language events, {len(clips)} decoded actor MOT chunks, {len(failures)} failures.\n\n'+'\n'.join('- '+e['id']+': '+str(len(e['cuts']))+' cuts; '+', '.join(a['object'] for a in e['actors']) for e in out),encoding='utf-8')
    print(dict(events=len(out),clips=len(clips),failures=failures))


def build(root):
    root=Path(root);source=root/'extracted/disc/DATA/files/event';g=Graph(dict(game='MadWorld',authority='exact tagged chunk framing; payload semantics unknown unless stated'));events=[];failures=[]
    def add(path,node,parent):
        nid=path+'@'+str(node['offset']);g.add_node(nid,'event-chunk',node['tag'].strip(),path=path,**{k:v for k,v in node.items() if k!='children'})
        if parent:g.add_edge(parent,nid,'contains')
        for child in node['children']:add(path,child,nid)
    for path in sorted(source.glob('*.dat')):
        raw=path.read_bytes()
        if not raw:continue
        try:tree=chunks(raw)
        except ValueError as ex:failures.append(dict(path=path.name,error=str(ex)));continue
        events.append(dict(path=path.name,size=len(raw),sha256=hashlib.sha256(raw).hexdigest(),chunks=tree))
        for node in tree:add(path.name,node,None)
    (root/'evidence/event-chunks.json').write_text(json.dumps(dict(events=events,failures=failures),indent=2),encoding='utf-8');g.save(root/'evidence/events.graph.jsonl')
    (root/'evidence/EVENT-INDEX.md').write_text('# Native event chunks\n\n'+f'{len(events)} files framed exactly; {len(failures)} failures.\n\nTagged ranges alone do not establish event timing, animation encoding or actor identity.\n\n'+'\n'.join('- '+e['path'] for e in events),encoding='utf-8')
    (root/'evidence/EVENT-API.md').write_text('# Event vocabulary\n\n`event-chunk`: four-byte native tag plus big-endian payload size. `contains`: exact byte-range nesting, accepted only when child chunks fully partition the payload. Unknown payloads remain bounded original ranges.\n',encoding='utf-8')
    print(dict(events=len(events),failures=failures,nodes=len(g.nodes)))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root');p.add_argument('--motions',action='store_true');a=p.parse_args();index_motions(a.root) if a.motions else build(a.root)
