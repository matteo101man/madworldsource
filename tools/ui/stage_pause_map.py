"""Stage original <room>.imt map pages and IDT-50 projection parameters for Rust pause and the radar.

python -I tools/ui/stage_pause_map.py [room]   (default c201; c303 Elise, c503 the Black Baron)"""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPORT/'builders'))
from native import tpl

def idt_from_disc(room):
    # the room's idpac IDD carries IDT program 50 (the map widget); the first IDD whose sections frame an IDT
    raw=(EXPORT/'extracted/disc/DATA/files'/f'case{room[1]}'/f'idpac_{room}_us.dat').read_bytes();cursor=0
    while (at:=raw.find(b'IDD\0',cursor))>=0:
        cursor=at+4;r=raw[at:]
        try:sec={struct.unpack_from('>I',r,8+i*8)[0]:struct.unpack_from('>I',r,12+i*8)[0] for i in range(4)}
        except struct.error:continue
        if 2 in sec and 3 in sec and r[sec[2]:sec[2]+4]==b'IDT\0':return r[sec[2]:sec[3]]
    raise ValueError('No IDT in idpac_'+room)

def main():
    room=sys.argv[1] if len(sys.argv)>1 else 'c201'
    catalog=json.loads((EXPORT/'catalog.json').read_text(encoding='utf-8'))
    bank=next(b for b in catalog['banks'] if b['id']==f'case{room[1]}/{room}.dat')
    source=Path(bank['path']).read_bytes()
    if hashlib.sha256(source).hexdigest()!=bank['sha256']:raise ValueError('Stage archive identity')
    member=next(e for e in bank['entries'] if e['name']==f'{room}.imt')
    raw=source[member['offset']:member['offset']+member['size']]
    if raw[:4]!=bytes.fromhex('0020af30'):raise ValueError('IMT TPL header')
    folder=ROOT/'assets/native/ui'/f'{room}-map';folder.mkdir(exist_ok=True)
    pages=[]
    for i,(image,meta) in enumerate(tpl(raw)):
        path=folder/f'page-{i}.png';image.save(path)
        pages.append(dict(image=path.relative_to(ROOT/'assets').as_posix(),width=image.width,height=image.height,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    staged=ROOT/'assets/native/ui'/f'idpac_{room}_us/animation.idt'
    idt=staged.read_bytes() if staged.is_file() else idt_from_disc(room)
    count=struct.unpack_from('>I',idt,4)[0]
    base=next(struct.unpack_from('>I',idt,12+i*8)[0]for i in range(count)if struct.unpack_from('>I',idt,8+i*8)[0]==50)
    at=next(base+struct.unpack_from('>I',idt,base+4+i*4)[0]for i in range(struct.unpack_from('>I',idt,base)[0])if struct.unpack_from('>I',idt,base+struct.unpack_from('>I',idt,base+4+i*4)[0]+0x2c)[0]==30)
    low,high=struct.unpack_from('>2h',idt,at+0x42);floor_count=high-low
    if floor_count==0:floor_count=1   # a one-floor room (c303) leaves the height range empty
    if floor_count!=len(pages):raise ValueError('Map layers / IDT height count mismatch')
    span,pixels,origin_x,origin_z=struct.unpack_from('>4h',idt,at+0x46)
    thresholds=list(struct.unpack_from('>'+str(floor_count-1)+'f',idt,at+0x54))
    data=dict(schema='mw-stage-map/1',source=dict(bank=bank['id'],bankSha256=bank['sha256'],member=f'{room}.imt',offset=member['offset'],bytes=member['size'],sha256=hashlib.sha256(raw).hexdigest()),
        projectionSource=dict(resource=f'idpac_{room}_us',program=50,widget=30,recordOffset=at,idtSha256=hashlib.sha256(idt).hexdigest()),
        pages=pages,heightThresholds=thresholds,pixelsPerWorldUnit=span*0.1,texturePixels=pixels,origin=[-origin_x,origin_z],
        authority='8024803C floor selection and 80247470 world X/Z projection; source IMT bytes unmodified')
    (folder/'map.json').write_text(json.dumps(data,indent=2)+'\n',encoding='utf-8');print(json.dumps(data))
if __name__=='__main__':main()
