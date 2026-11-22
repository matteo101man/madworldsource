"""Native glyph messages with exact atlas matching; unmatched glyphs stay explicit."""
import hashlib
import json
from pathlib import Path
import struct
from native import Reader,tpl,message_tokens
from event import chunks
from gk.graph import Graph

# Visual transcription of the owner's nl.mtd atlas, in native glyph-index order.
# A glyph is reused only when its cropped bitmap matches exactly, including size.
NL_GLYPHS="ImportanNiceBfPlygMuThsd.WJ'kbxCw,EvjFD\"!AG/R-;OVHS"
NL_SHA256='7dcfa9a8c0941b31b092e2844a17560a6a9120e80bd98a33996810ee6ec04d4c'
FONT_TRANSCRIPTIONS={
    '183cba6657c465b83f4248b1063842da06dd9157d6b75030d842429dc6958c8f':
    ['S','o','.','t','h','e','a','r','g',"'",'s','i','n','c','l','u','?','C','m','p','w','f','d','v','I','y','M','b','B','q','H','A','T','-','D','z','N','E','"','k','Y','W','P','J','!','[','O',']',',','U','R','L','x','j','G','V'],
    '2dc7da9d75a49686b54f2a23d6eea8b91584f11e483aeacad9e77868b66dd0c1':
    ['L','o','k','s','l','i','e','y','u',"'",'r','n','.','G','d','w','T','h','t','m','B','p','g','a','c','Y','j','v','f','b',',','?','O','J','S','I','N','W','A','-']
}


def font(data):
    from PIL import ImageOps
    baseOffset=None
    for candidate in range(0,min(32,len(data)-12),2):
        metrics,base,count=Reader(data).get('3I',candidate)
        if base==0x1000 and 0<=count<2000 and metrics==12+count*20:
            baseOffset=candidate;break
    if baseOffset is None:raise ValueError('glyph directory size')
    data=data[baseOffset:];r=Reader(data)
    if count==0:return {}
    offset=data.find(bytes.fromhex('0020af30'),metrics)
    if offset<0:raise ValueError('missing font TPL')
    atlases=[im.convert('RGB') for im,_ in tpl(data[offset:])];out={}
    for i in range(count):
        index,flags,u,v,s,t=r.get('HH4f',12+i*20)
        if flags>=len(atlases):raise ValueError('font atlas page index')
        atlas=atlases[flags]
        box=tuple(round(x*y) for x,y in zip((u,v,s,t),(atlas.width,atlas.height,atlas.width,atlas.height)))
        if not 0<=box[0]<=box[2]<=atlas.width or not 0<=box[1]<=box[3]<=atlas.height:raise ValueError('glyph atlas bounds')
        image=ImageOps.grayscale(atlas.crop(box))
        digest=hashlib.sha256(str(image.size).encode()+image.tobytes()).hexdigest()
        out[base+index]=dict(image=image,digest=digest,flags=flags)
    return out


def messages(data):
    return message_tokens(data)


def build(root):
    root=Path(root);source=root/'extracted/disc/DATA/files';masterRaw=(source/'subscr/nl.mtd').read_bytes()
    if hashlib.sha256(masterRaw).hexdigest()!=NL_SHA256:raise ValueError('reference font hash changed')
    master=font(masterRaw)
    if len(master)!=len(NL_GLYPHS):raise ValueError('reference transcription count')
    glyphs={}
    def learn(f,transcription):
        if len(f)!=len(transcription):raise ValueError('transcribed glyph count')
        for (_,v),char in zip(sorted(f.items()),transcription):glyphs.setdefault(v['digest'],set()).add(char)
    learn(master,NL_GLYPHS)
    # Learn every hash-guarded transcription before decoding any event. Otherwise
    # earlier files silently have poorer coverage because of directory order.
    for path in sorted((source/'event').glob('*_us.dat')):
        raw=path.read_bytes()
        if not raw:continue
        mess=next((n for n in chunks(raw)[0]['children'] if n['tag']=='MESS'),None)
        if not mess:continue
        member=next((n for n in mess['children'] if n['tag']=='MTD '),None)
        if member is None:continue
        data=raw[member['payload']:member['end']];transcription=FONT_TRANSCRIPTIONS.get(hashlib.sha256(data).hexdigest())
        if transcription:learn(font(data),transcription)
    known={digest:next(iter(chars)) for digest,chars in glyphs.items() if len(chars)==1}
    g=Graph(dict(game='MadWorld',authority='native MSD token ranges; ASCII from exact glyph bitmap matches to hash-guarded visually transcribed atlases; ambiguous bitmap matches omitted'));rows=[];failures=[]
    for path in sorted((source/'event').glob('*_us.dat')):
        raw=path.read_bytes()
        if not raw:continue
        try:
            mess=next((n for n in chunks(raw)[0]['children'] if n['tag']=='MESS'),None)
            if not mess:continue
            members={n['tag']:raw[n['payload']:n['end']] for n in mess['children']}
            f=font(members['MTD ']);texts=messages(members['MSD ']);eid=path.stem.removesuffix('_us');g.add_node(eid,'event',eid)
            transcription=FONT_TRANSCRIPTIONS.get(hashlib.sha256(members['MTD ']).hexdigest())
            direct={}
            if transcription:
                if len(transcription)!=len(f):raise ValueError('transcribed glyph count')
                direct={code:char for (code,_),char in zip(sorted(f.items()),transcription)}
            for entry in texts:
                text=[];unknown=set();controls=[];i=0
                while i<len(entry['tokens']):
                    token=entry['tokens'][i];i+=1
                    if token in f:
                        ch=direct.get(token) or known.get(f[token]['digest'])
                        if ch is None:unknown.add(token);ch='⟦'+hex(token)+'⟧'
                        text.append(ch)
                    elif token==0x8001:text.append(' ')
                    elif token in (0x8000,0):text.append('\n')
                    elif token==0x8010 and i<len(entry['tokens']):controls.append([token,entry['tokens'][i]]);i+=1
                    else:controls.append([token])
                row=dict(event=eid,id=entry['id'],text=''.join(text),unknownGlyphs=sorted(unknown),controls=controls);rows.append(row)
                nid=eid+':message:'+str(entry['id']);g.add_node(nid,'message',str(entry['id']),**row);g.add_edge(eid,nid,'contains')
        except Exception as ex:failures.append(dict(file=path.name,error=str(ex)))
    (root/'evidence/messages.json').write_text(json.dumps(dict(messages=rows,failures=failures),indent=2),encoding='utf-8');g.save(root/'evidence/messages.graph.jsonl')
    (root/'evidence/MESSAGE-API.md').write_text('# Native message vocabulary\n\n`event contains message`: English locale MSD. Text glyphs are matched by exact bitmap and dimensions to a visually transcribed source atlas. Unknown glyphs remain hexadecimal tokens. 0x8001 is rendered as space and 0x8010 consumes one formatting operand; other controls remain explicit. Text alone does not prove actor identity.\n',encoding='utf-8')
    print(dict(messages=len(rows),unknown=sum(bool(r['unknownGlyphs']) for r in rows),failures=failures))


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('root');build(p.parse_args().root)
