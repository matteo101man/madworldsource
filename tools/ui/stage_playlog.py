"""Stage original Play Log and core message fonts, exact tokens and glyph metrics."""
from pathlib import Path
import hashlib,json,math,struct,sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPORT/'builders'))
from native import tpl,message_tokens

def main():
    files=EXPORT/'extracted/disc/DATA/files';folder=ROOT/'assets/native/ui/playlog-us';folder.mkdir(exist_ok=True)
    font=(files/'subscr/subscr100_us.mtd').read_bytes();text=(files/'subscr/MessSubscr100_us.msd').read_bytes()
    core=(files/'etc/idpac_core_us.dat').read_bytes();header=core.index(b'MTD\0');length=struct.unpack_from('>I',core,header+4)[0];entry=dict(name='MTD',offset=header,size=length+32,payload=header+32);shared=core[header+32:header+32+length]
    if header+32+length>len(core):raise ValueError('Core MTD bounds')
    sources=[dict(file=name,sha256=hashlib.sha256(data).hexdigest(),bytes=len(data))for name,data in [('subscr/subscr100_us.mtd',font),('subscr/MessSubscr100_us.msd',text)]]
    sources.append(dict(file='etc/idpac_core_us.dat',sha256=hashlib.sha256(core).hexdigest(),bytes=len(core),entry=entry))
    pages=[];glyphs=[];base_advance=None
    for key,data,code_base in [('font',font,0x1000),('core-font',shared,0)]:
        metrics,first,count=struct.unpack_from('>3I',data)
        if metrics!=12+count*20 or first!=0x1000 or count>2000:raise ValueError('Glyph directory')
        metric_count,advance,line_height=struct.unpack_from('>3I',data,metrics)
        if metric_count!=count or not 0<advance<256 or not 0<line_height<256:raise ValueError('Font metrics')
        if key=='core-font':base_advance=advance
        at=data.find(bytes.fromhex('0020af30'),metrics+12+count*12)
        if at<metrics:raise ValueError('Font atlas')
        images=list(tpl(data[at:]));first_page=len(pages)
        for i,(image,metadata)in enumerate(images):
            path=folder/f'{key}-{i}.png';image.save(path)
            pages.append(dict(image=path.relative_to(ROOT/'assets').as_posix(),width=image.width,height=image.height,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
        for i in range(count):
            index,page,*uv=struct.unpack_from('>HH4f',data,12+i*20)
            metric_id,metric_page,width,height=struct.unpack_from('>HH2f',data,metrics+12+i*12)
            if index!=i or metric_id!=i or page!=metric_page or page>=len(images):raise ValueError('Glyph indices')
            size=images[page][0].size;rect=[round(v*s)for v,s in zip(uv,[size[0],size[1],size[0],size[1]])]
            if rect[0]>=rect[2]or rect[1]>=rect[3]or not all(0<=v<=1 for v in uv)or not all(math.isfinite(v)and v>0 for v in [width,height]):raise ValueError('Glyph bounds')
            glyphs.append(dict(id=code_base+index,page=first_page+page,rect=rect,size=[width,height]))
    messages=message_tokens(text)
    if len(messages)%2:raise ValueError('Play Log paired messages')
    data=dict(schema='mw-native-message-catalog/2',sources=sources,pages=pages,glyphs=glyphs,messages=messages,baseAdvance=base_advance,
        scope='Original glyphs, metrics and exact message tokens. Live statistic systems are separate.')
    (folder/'catalog.json').write_text(json.dumps(data,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(glyphs=len(glyphs),messages=len(messages),pages=len(pages))))
if __name__=='__main__':main()
