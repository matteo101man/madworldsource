"""Stage owned English UI pages/TUV rectangles for the pure Rust presenter.
IDT bytes are preserved with provenance; no animation/position semantics inferred.
"""
from pathlib import Path
import hashlib, json, struct, sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(ROOT/'tools'))
sys.path.insert(0,str(EXPORT/'builders'))
from index_ui_layout import parse
from native import tpl
from PIL import Image, ImageOps

# IDT-only containers (sections 0 and 1 empty): their programs draw with another resource's pages/TUV (subscr0ff:
# cIdGameOver's continue / game-over windows, all sprites from core_us), so only the IDT is staged, with provenance.
IDT_ONLY={'subscr/subscr0ff_us.idd':'core_us'}

def stage_idt_only(files,out):
    staged=[]
    for name,common in IDT_ONLY.items():
        source=(files/name).read_bytes();base=source.find(b'IDD\0')
        if base<0:raise ValueError(f'{name}: no IDD')
        raw=source[base:]
        sections={struct.unpack_from('>I',raw,8+i*8)[0]:struct.unpack_from('>I',raw,12+i*8)[0] for i in range(4)}
        if sections.get(0)!=0 or sections.get(1)!=0:raise ValueError(f'{name}: has TEX/TUV, stage it normally')
        idt=raw[sections[2]:sections[3]]
        if not idt.startswith(b'IDT\0'):raise ValueError('IDT framing')
        key=Path(name).stem;folder=out/key;folder.mkdir(parents=True,exist_ok=True)
        (folder/'animation.idt').write_bytes(idt)
        info={'schema':'mw-ui-idt-only/1','source':{'path':name,'containerSha256':hashlib.sha256(source).hexdigest(),'offset':base},
              'pagesFrom':common,'animation':{'file':f'native/ui/{key}/animation.idt','sha256':hashlib.sha256(idt).hexdigest(),'bytes':len(idt)}}
        (folder/'idt.json').write_text(json.dumps(info,indent=1)+'\n',encoding='utf-8')
        staged.append(info)
    return staged

def main():
    files=EXPORT/'extracted/disc/DATA/files'
    # --idt-only: stage just the IDT-only containers
    if '--idt-only' in sys.argv:
        print(json.dumps(stage_idt_only(files,ROOT/'assets/native/ui'),indent=1));return
    paths=['etc/core_us.idd','subscr/subscr000_us.idd','subscr/subscr001_us.idd',
           'subscr/subscr0fe_us.idd','subscr/nl_us.idd','subscr/ex000_us.idd','case2/idpac_c201_us.dat','caseE/idpac_ce00_us.dat',
           'case3/idpac_c303_us.dat',   # c303: the stage-clear results (program 31, ELISE-BOSS.md §2)
           'case5/idpac_c503_us.dat']   # c503: the Baron's clash cut-in (program 11)
    out=ROOT/'assets/native/ui';out.mkdir(parents=True,exist_ok=True)
    manifest={'schema':'mw-native-ui/1','scope':'Owned English source pages and exact TUV rectangles; IDT transforms/timing not translated','resources':[],'failures':[]}
    for name in paths:
        source=(files/name).read_bytes();candidates=[];cursor=0
        while (at:=source.find(b'IDD\0',cursor))>=0:
            candidates.append(at);cursor=at+4
        accepted=0
        for base in candidates:
            raw=source[base:]
            try:
                sections={struct.unpack_from('>I',raw,8+i*8)[0]:struct.unpack_from('>I',raw,12+i*8)[0] for i in range(4)}
                start=sections[0]+16
                if raw[start:start+4]!=bytes.fromhex('0020af30'):raise ValueError('TPL framing')
                decoded=list(tpl(raw[start:]))
                layout=parse(raw,[(meta['width'],meta['height']) for _,meta in decoded])
                if not all(0<=v<=len(raw) for v in sections.values()):raise ValueError('section bounds')
            except (ValueError,KeyError,IndexError,struct.error):continue
            key=Path(name).stem+(f'-{accepted}' if accepted else '')
            folder=out/key;folder.mkdir(exist_ok=True)
            pages=[]
            for i,(image,meta) in enumerate(decoded):
                dest=folder/f'page-{i}.png';image.save(dest)
                # Native 800D07E4 selects the TEV RGB complement for white menu glyphs.
                r,g,b,a=image.convert('RGBA').split()
                inverted=folder/f'page-{i}-inverted.png'
                Image.merge('RGBA',(ImageOps.invert(r),ImageOps.invert(g),ImageOps.invert(b),a)).save(inverted)
                pages.append({'id':i,'width':meta['width'],'height':meta['height'],'image':f'native/ui/{key}/page-{i}.png','sha256':hashlib.sha256(dest.read_bytes()).hexdigest(),
                    'invertedImage':f'native/ui/{key}/page-{i}-inverted.png','invertedSha256':hashlib.sha256(inverted.read_bytes()).hexdigest()})
            for sprite in layout['sprites']:sprite['texturePage']=sprite.pop('unknown16')
            provenance={'path':name,'containerSha256':hashlib.sha256(source).hexdigest(),'offset':base}
            idt=raw[sections[2]:sections[3]]
            if not idt.startswith(b'IDT\0'):raise ValueError('IDT framing')
            (folder/'animation.idt').write_bytes(idt)
            staged={'schema':'mw-ui-layout/1','source':provenance,'pages':pages,'groups':layout['groups'],'sprites':layout['sprites'],
                    'animation':{'file':f'native/ui/{key}/animation.idt','sha256':hashlib.sha256(idt).hexdigest(),'bytes':len(idt),'semantics':'untranslated'}}
            (folder/'layout.json').write_text(json.dumps(staged,indent=1)+'\n',encoding='utf-8')
            manifest['resources'].append({'id':key,**provenance,'pages':len(pages),'sprites':len(layout['sprites']),'layout':f'native/ui/{key}/layout.json'})
            accepted+=1
        if accepted==0:manifest['failures'].append({'source':name,'reason':'No validated IDD/TPL/TUV framing'})
    manifest['idtOnly']=stage_idt_only(files,out)
    (out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(manifest,indent=1))
    if manifest['failures']:raise SystemExit(1)
if __name__=='__main__':main()
