"""Retarget the standalone UI resource actually read by the native game."""
import hashlib,json,struct
from index_ui_layout import ROOT
from index_live_ui_layout import load,SOURCE
from pc_ui_font import FONT

def address(page,x,y):
    # CI4: 8x8 tiles, two source texels per byte (high nibble first).
    return page['dataOffset']+((y//8)*(page['width']//8)+x//8)*32+(y%8)*4+(x%8)//2

def main():
    raw,layout=load();output=bytearray(raw);rects={s['id']:s for s in layout['sprites']};changed=[];allowed=set()
    def paint(sid,lines):
        sprite=rects[sid];page=layout['pages'][sprite['texturePage']]
        if page['format']!=8 or page['maxLod']!=0:raise ValueError('Unsupported prompt texture layout')
        pal=struct.unpack_from('>16H',raw,page['paletteOffset']);clear=pal.index(0);ink=pal.index(0xffff);black=pal.index(0x8000)
        x0,y0,x1,y1=sprite['rect'];w=x1-x0;h=y1-y0
        def put(x,y,value):
            if not(0<=x<w and 0<=y<h):raise ValueError('PC glyph out of bounds')
            px=x0+x;at=address(page,px,y0+y);shift=4 if px%2==0 else 0
            allowed.add((at,shift));output[at]=(output[at]&~(15<<shift))|(value<<shift)
        for y in range(h):
            for x in range(w):put(x,y,clear)
        for y in range(1,h-1):
            for x in range(1,w-1):put(x,y,ink if x in (1,w-2) or y in (1,h-2) else black)
        step=(h-6)//len(lines)
        for row,label in enumerate(lines):
            scale=min(3,(w-6)//max(1,4*len(label)-1),max(1,(step-2)//5))
            if scale<1:raise ValueError('PC label too wide')
            left=(w-(4*len(label)-1)*scale)//2;top=3+row*step+(step-5*scale)//2
            for k,ch in enumerate(label):
                for gy,bits in enumerate(FONT[ch]):
                    for gx,bit in enumerate(bits):
                        if bit=='1':
                            for dy in range(scale):
                                for dx in range(scale):put(left+(4*k+gx)*scale+dx,top+gy*scale+dy,ink)
        changed.append(dict(sprite=sid,page=sprite['texturePage'],rect=sprite['rect'],labels=lines))
    for sid,label in [(1,'LMB'),(47,'LMB'),(49,'RMB'),(51,'Q'),(54,'SPC'),(42,'TAB'),(43,'ESC')]:paint(sid,[label])
    for sid,lines in [(64,['RIGHT','^','<v>','N M','V B','X']),(65,['RIGHT','^','<v>','N M','V B','X']),(66,['LEFT','I','JKL','U O','H Y','X']),(67,['LEFT','I','JKL','U O','H Y','X'])]:paint(sid,lines)
    actual={(at,shift) for at,(a,b) in enumerate(zip(raw,output)) if a!=b for shift in (0,4) if ((a^b)>>shift)&15}
    if not actual or not actual<=allowed:raise ValueError('Untargeted nibble changed')
    selected={s['sprite'] for s in changed}
    for sprite in layout['sprites']:
        if sprite['id'] in selected:continue
        page=layout['pages'][sprite['texturePage']]
        if page['format']!=8:continue
        x0,y0,x1,y1=sprite['rect']
        if any((address(page,x,y),4 if x%2==0 else 0) in actual for y in range(y0,y1) for x in range(x0,x1)):
            raise ValueError(f"Untargeted sprite overlap: {sprite['id']}")
    folder=ROOT/'staged/pc-ui';folder.mkdir(exist_ok=True,parents=True);dest=folder/'core_us.idd';dest.write_bytes(output)
    manifest=dict(scope='Standalone common PC UI conversion; native transforms and live legibility unverified',source=str(SOURCE),sourceSha256=hashlib.sha256(raw).hexdigest(),output=str(dest),outputSha256=hashlib.sha256(output).hexdigest(),size=len(output),changedPixelNibbles=len(actual),changedBytes=sum(a!=b for a,b in zip(raw,output)),sprites=changed,palettesUnchanged=True,untargetedSpritesUnchanged=True,mipLevels=1,pages=[])
    for page in layout['pages']:
        if page['id'] not in {s['page'] for s in changed}:continue
        at=page['dataOffset'];n=page['width']*page['height']//2
        manifest['pages'].append(dict(id=page['id'],offset=at,size=n,sha256=hashlib.sha256(output[at:at+n]).hexdigest()))
    (folder/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(sprites=len(changed),changedPixelNibbles=len(actual),output=str(dest))))
if __name__=='__main__':main()
