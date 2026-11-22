"""Stage an event's IDD bank (MW_EVENT, default ev21) into assets/native/events/evNN/ui/.

Two IDD shapes exist (Native 800D9FD0 aligns the IDD-space chunk header+0x27 down to 32 bytes):

* Animation-only (ev21/ev33/ev34/ev50/ev51): sections 0 (TEX) and 1 (TUV) are empty; the programs draw with
  core_us pages/TUV. Writes ui/animation.idt and ui/ui.json:
      {"schema":"mw-event-ui/1","event":N,"eventSourceSha256":..,"sourceOffset":..,"sourceBytes":..,"sha256":..,
       "animation":"native/events/evNN/ui/animation.idt","sharedAtlas":"core_us","programs":[..],"scope":..}
* Embedded atlas (ev84/ev93): the IDD carries its own TEX (TPL) + TUV. Staged like tools/ui/stage_native_ui.py
  stages an idpac: ui/page-N.png, ui/page-N-inverted.png, ui/animation.idt and ui/layout.json
  (schema mw-ui-layout/1, read by mw_data::ui::UiLayout; source.offset is the IDD offset inside
  event/evNN_us.dat, sprite recordOffset is relative to that IDD). ui.json then has NO "sharedAtlas"; instead
      "atlas":"own", "layout":"native/events/evNN/ui/layout.json"
  and its "scope" says so. Every other field is as above.
"""
from pathlib import Path
import hashlib,json,os,struct,sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')


def stage_atlas(ev,raw,base,end,sections,idt,folder):
    """Decode the embedded TPL pages and TUV rectangles; returns the layout path (asset-relative)."""
    sys.path.insert(0,str(ROOT/'tools'))
    sys.path.insert(0,str(EXPORT/'builders'))
    from index_ui_layout import parse
    from native import tpl
    from PIL import Image, ImageOps
    idd=raw[base:end]
    start=sections[0]+16
    if idd[start:start+4]!=bytes.fromhex('0020af30'):raise ValueError('TPL framing')
    decoded=list(tpl(idd[start:]))
    layout=parse(idd,[(meta['width'],meta['height']) for _,meta in decoded])
    key=f'native/events/{ev}/ui'
    pages=[]
    for i,(image,meta) in enumerate(decoded):
        dest=folder/f'page-{i}.png';image.save(dest)
        # Native 800D07E4 selects the TEV RGB complement for white menu glyphs.
        r,g,b,a=image.convert('RGBA').split()
        inverted=folder/f'page-{i}-inverted.png'
        Image.merge('RGBA',(ImageOps.invert(r),ImageOps.invert(g),ImageOps.invert(b),a)).save(inverted)
        pages.append({'id':i,'width':meta['width'],'height':meta['height'],'image':f'{key}/page-{i}.png','sha256':hashlib.sha256(dest.read_bytes()).hexdigest(),
            'invertedImage':f'{key}/page-{i}-inverted.png','invertedSha256':hashlib.sha256(inverted.read_bytes()).hexdigest()})
    for sprite in layout['sprites']:sprite['texturePage']=sprite.pop('unknown16')
    source=(EXPORT/'extracted/disc/DATA/files/event'/f'{ev}_us.dat').read_bytes()
    if source!=raw:raise ValueError('source.dat differs from the disc event file')
    staged={'schema':'mw-ui-layout/1','source':{'path':f'event/{ev}_us.dat','containerSha256':hashlib.sha256(source).hexdigest(),'offset':base},
            'pages':pages,'groups':layout['groups'],'sprites':layout['sprites'],
            'animation':{'file':f'{key}/animation.idt','sha256':hashlib.sha256(idt).hexdigest(),'bytes':len(idt),'semantics':'untranslated'}}
    (folder/'layout.json').write_text(json.dumps(staged,indent=1)+'\n',encoding='utf-8')
    return f'{key}/layout.json',len(pages),len(layout['sprites'])


def main():
    ev=os.environ.get('MW_EVENT','ev21')
    event=json.loads((ROOT/'assets/native/events'/ev/'event.json').read_text())
    raw=(ROOT/'assets/native/events'/ev/'source.dat').read_bytes()
    source=next(r['source']for r in event['resources']if r['tag']=='ID  ')
    start=source['offset'];end=start+source['bytes'];block=raw[start:end]
    assert hashlib.sha256(block).hexdigest()==source['sha256']
    # Native800D9FD0 aligns the IDD-space chunk header+0x27 down to32 bytes.
    assert block[:4]==b'IDD '
    base=(start+0x27)&~31
    assert raw[base:base+4]==b'IDD\0'
    sections={struct.unpack_from('>I',raw,base+8+i*8)[0]:struct.unpack_from('>I',raw,base+12+i*8)[0]for i in range(4)}
    embedded=not(sections[0]==sections[1]==0)
    if embedded:assert 0<sections[0]<sections[1]<sections[2]
    assert sections[2]<sections[3] and base+sections[3]<=end
    idt=raw[base+sections[2]:base+sections[3]];assert idt[:4]==b'IDT\0'
    count=struct.unpack_from('>I',idt,4)[0]
    programs=[struct.unpack_from('>II',idt,8+i*8)for i in range(count)]
    if ev=='ev21':assert [p[0]for p in programs]==[0,1,2,3,100]
    folder=ROOT/'assets/native/events'/ev/'ui';folder.mkdir(parents=True,exist_ok=True)
    (folder/'animation.idt').write_bytes(idt)
    manifest={'schema':'mw-event-ui/1','event':event['event'],'eventSourceSha256':event['sourceSha256'],
        'sourceOffset':base+sections[2],'sourceBytes':len(idt),'sha256':hashlib.sha256(idt).hexdigest(),
        'animation':f'native/events/{ev}/ui/animation.idt','sharedAtlas':'core_us',
        'programs':[p[0]for p in programs],
        'scope':'Exact animation-only IDD bank using original core_us TUV/TEX; no embedded texture or dialogue data.'}
    if embedded:
        layout,pages,sprites=stage_atlas(ev,raw,base,end,sections,idt,folder)
        del manifest['sharedAtlas']
        manifest.update({'atlas':'own','layout':layout,
            'scope':f'Exact IDD bank with its own embedded TEX/TUV atlas ({pages} pages, {sprites} sprites, see layout); no dialogue data.'})
    (folder/'ui.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(manifest,indent=2))


if __name__=='__main__':main()
