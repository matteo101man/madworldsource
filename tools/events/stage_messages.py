"""Stage original optional MESS tracks, glyphs and counted commands."""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPORT/'builders'))
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from event import chunks,sequence
from native import tpl,message_tokens


def stage(key):
    source=EXPORT/'extracted/disc/DATA/files/event'/f'{key}_us.dat';raw=source.read_bytes()
    nodes=[n for n in chunks(raw)[0]['children']if n['tag']=='MESS'];assert len(nodes)<=1
    folder=ROOT/'assets/native/events'/key/'messages';folder.mkdir(parents=True,exist_ok=True)
    manifest=dict(schema='mw-event-messages/1',event=int(key[2:],16),sourceFile=f'event/{source.name}',sourceSha256=hashlib.sha256(raw).hexdigest(),sourceBytes=len(raw),track=None)
    if nodes:
        node=nodes[0];parts={n['tag']:n for n in node['children']};assert set(parts)=={'MSD ','MTD ','SEQ '}
        # Native80112FEC aligns (chunk-header+39) down to32, skipping padding.
        payload=lambda tag:raw[(parts[tag]['offset']+39)&~31:parts[tag]['end']]
        font=payload('MTD ');text=payload('MSD ')
        metrics,first,count=struct.unpack_from('>3I',font);assert first==0x1000 and metrics==12+count*20
        metric_count,advance,line_height=struct.unpack_from('>3I',font,metrics);assert metric_count==count
        at=font.find(bytes.fromhex('0020af30'),metrics+12+count*12);assert at>=0
        images=list(tpl(font[at:]));pages=[];glyphs=[]
        for i,(image,_)in enumerate(images):
            path=folder/f'font-{i}.png';image.save(path)
            pages.append(dict(image=path.relative_to(ROOT/'assets').as_posix(),width=image.width,height=image.height,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
        for i in range(count):
            index,page,*uv=struct.unpack_from('>HH4f',font,12+i*20)
            mid,mp,width,height=struct.unpack_from('>HH2f',font,metrics+12+i*12);assert index==mid==i and page==mp<len(pages)
            size=images[page][0].size;rect=[round(v*s)for v,s in zip(uv,[size[0],size[1],size[0],size[1]])]
            glyphs.append(dict(id=first+i,page=page,rect=rect,size=[width,height]))
        catalog=dict(schema='mw-native-message-catalog/2',pages=pages,glyphs=glyphs,messages=message_tokens(text),baseAdvance=advance)
        (folder/'catalog.json').write_text(json.dumps(catalog,indent=2)+'\n')
        (folder/'source.mess').write_bytes(raw[node['offset']:node['end']])
        seq=parts['SEQ '];commands=sequence(raw,seq['payload'],seq['end'])
        manifest['track']=dict(catalog=(folder/'catalog.json').relative_to(ROOT/'assets').as_posix(),source=dict(offset=node['offset'],bytes=node['end']-node['offset'],sha256=hashlib.sha256(raw[node['offset']:node['end']]).hexdigest()),commands=commands,lineHeight=line_height)
    (folder/'messages.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps(dict(event=key,track=bool(nodes),commands=len(manifest['track']['commands'])if nodes else 0)))


if __name__=='__main__':
    for key in (sys.argv[1:] or ['ev21','ev00']):stage(key)
