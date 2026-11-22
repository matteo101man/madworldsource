"""Stage the original stage201 boss-challenge prompt message and font."""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPORT/'builders'))
from native import tpl,message_tokens


def main():
    path=EXPORT/'extracted/disc/DATA/files/case2/idpac_c201_us.dat';raw=path.read_bytes()
    chunks={}
    for tag in [b'MSD\0',b'MTD\0']:
        at=raw.index(tag);size=struct.unpack_from('>I',raw,at+4)[0]
        assert at+32+size<=len(raw);chunks[tag]=(at+32,raw[at+32:at+32+size])
    message=message_tokens(chunks[b'MSD\0'][1])[8];assert message['id']==8
    font=chunks[b'MTD\0'][1];metrics,first,count=struct.unpack_from('>3I',font)
    assert metrics==12+count*20 and first==0x1000
    metric_count,advance,line_height=struct.unpack_from('>3I',font,metrics);assert metric_count==count
    at=font.find(bytes.fromhex('0020af30'),metrics+12+count*12);assert at>=metrics
    images=list(tpl(font[at:]));folder=ROOT/'assets/native/ui/challenge-c201-us';folder.mkdir(exist_ok=True)
    pages=[];glyphs=[]
    for i,(image,_)in enumerate(images):
        file=folder/f'font-{i}.png';image.save(file)
        pages.append(dict(image=file.relative_to(ROOT/'assets').as_posix(),width=image.width,height=image.height,sha256=hashlib.sha256(file.read_bytes()).hexdigest()))
    for i in range(count):
        index,page,*uv=struct.unpack_from('>HH4f',font,12+i*20)
        mid,mp,width,height=struct.unpack_from('>HH2f',font,metrics+12+i*12)
        assert index==mid==i and page==mp and page<len(images)
        size=images[page][0].size;rect=[round(v*s)for v,s in zip(uv,[size[0],size[1],size[0],size[1]])]
        glyphs.append(dict(id=0x1000+i,page=page,rect=rect,size=[width,height]))
    catalog=dict(schema='mw-native-message-catalog/2',pages=pages,glyphs=glyphs,messages=[dict(id=0,tokens=message['tokens'])],baseAdvance=advance,
                 source=dict(path='case2/idpac_c201_us.dat',sha256=hashlib.sha256(raw).hexdigest(),messageId='0x12010008',catalogMessage=0,
                             msdOffset=chunks[b'MSD\0'][0],mtdOffset=chunks[b'MTD\0'][0],lineHeight=line_height),
                 scope='Exact original message12010008 tokens at local catalog index0, metric glyphs and unmodified font pages; no replacement text.')
    (folder/'catalog.json').write_text(json.dumps(catalog,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(glyphs=count,pages=len(pages),advance=advance,lineHeight=line_height,tokens=message['tokens'])))


if __name__=='__main__':main()
