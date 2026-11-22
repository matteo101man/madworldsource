"""Stage sparse PC prompt pixels in an owned archive; never alter source data."""
from pathlib import Path
import hashlib,json,struct
from index_ui_layout import ROOT,ARCHIVE,MEMBER_OFFSET,MEMBER_SIZE,load_member,parse

# Authored 3x5 bitmap font, independent of game artwork.
from pc_ui_font import FONT

def main():
    raw=load_member();layout=parse(raw);result=bytearray(raw)
    data_start=4096+608
    palette=struct.unpack_from('>256H',raw,4096+32)
    clear=palette.index(0);white=palette.index(0xffff);black=palette.index(0x8000);yellow=palette.index(0xffe0)
    rects={s['id']:s['rect'] for s in layout['sprites']};changed=[];allowed=set()
    def addr(x,y):return data_start+((y//4)*128+x//8)*32+(y%4)*8+x%8
    def paint(sid,lines,highlight=False):
        x0,y0,x1,y1=rects[sid];w=x1-x0;h=y1-y0;ink=yellow if highlight else white
        def put(x,y,c):
            if not(0<=x<w and 0<=y<h):raise ValueError('Authored glyph outside sprite')
            result[addr(x+x0,y+y0)]=c
        for y in range(h):
            for x in range(w):allowed.add(addr(x+x0,y+y0));put(x,y,clear)
        # A compact outlined keyboard card/keycap. Center each authored label.
        for y in range(1,h-1):
            for x in range(1,w-1):put(x,y,ink if x in (1,w-2) or y in (1,h-2) else black)
        step=(h-6)//len(lines)
        for row,label in enumerate(lines):
            scale=min(2,(w-6)//max(1,4*len(label)-1),max(1,(step-2)//5))
            if scale<1:raise ValueError('Label too wide')
            left=(w-(4*len(label)-1)*scale)//2;top=3+row*step+(step-5*scale)//2
            for k,ch in enumerate(label):
                for gy,bits in enumerate(FONT[ch]):
                    for gx,bit in enumerate(bits):
                        if bit=='1':
                            for dy in range(scale):
                                for dx in range(scale):put(left+(k*4+gx)*scale+dx,top+gy*scale+dy,ink)
        changed.append(dict(sprite=sid,rect=rects[sid],labels=lines,highlight=highlight))
    # Native A/B/C/Z glyph slots, including the smaller contextual A and B key.
    for sid,label in [(54,'LMB'),(55,'LMB'),(56,'RMB'),(57,'RMB'),(58,'Q'),(59,'Q'),(60,'SPC')]:paint(sid,[label])
    # Native right/left device slots become keyboard cards for the respective arm.
    # Native navigation animation remains intact; these are a retarget, not recovered art.
    right=['R','^','<v>','N M','V B','X']
    left=['L','I','JKL','U O','H Y','X']
    for sid,lines,highlight in [(63,right,False),(64,right,False),(65,left,False),(66,left,False),(137,right,True),(138,right,True),(139,left,True),(140,left,True)]:paint(sid,lines,highlight)
    actual={i for i,(a,b) in enumerate(zip(raw,result)) if a!=b}
    if not actual or not actual<=allowed:raise ValueError('Unexpected modified bytes')
    # Every sprite overlapping a changed region must be one of our intended slots.
    selected={s['sprite'] for s in changed}
    for s in layout['sprites']:
        if s['id'] in selected:continue
        x0,y0,x1,y1=s['rect']
        if any(addr(x,y) in actual for y in range(y0,y1) for x in range(x0,x1)):
            raise ValueError(f"Untargeted sprite overlap: {s['id']}")
    archive=bytearray(ARCHIVE.read_bytes());source_hash=hashlib.sha256(archive).hexdigest()
    archive[MEMBER_OFFSET:MEMBER_OFFSET+MEMBER_SIZE]=result
    output=ROOT/'staged/pc-ui-archived';output.mkdir(parents=True,exist_ok=True)
    dest=output/'core2_us.dat';dest.write_bytes(archive)
    manifest=dict(scope='PC prompt asset conversion; live native layout/animation remains unverified',source=str(ARCHIVE),sourceSha256=source_hash,output=str(dest),outputSha256=hashlib.sha256(archive).hexdigest(),archiveSize=len(archive),memberSourceSha256=hashlib.sha256(raw).hexdigest(),memberOutputSha256=hashlib.sha256(result).hexdigest(),changedPixelBytes=len(actual),sprites=changed,paletteUnchanged=True,mipLevels=1,untargetedSpritesUnchanged=True)
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(sprites=len(changed),changedPixelBytes=len(actual),output=str(dest))))
if __name__=='__main__':main()
