from save_typed_graph import save_graph
"""Index the owned core IDD sprite table; preserve unknown field semantics."""
from pathlib import Path
import hashlib,json,math,struct
ROOT=Path(__file__).resolve().parents[1]
ARCHIVE=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files/etc/core2_us.dat')
MEMBER_OFFSET=5801440
MEMBER_SIZE=1403232
EXPECTED='901b367d33eaae1d2edc8434175a98e56b9006e8b84db87acea8add0bfe3abc6'

def load_member():
    with ARCHIVE.open('rb') as f:
        f.seek(MEMBER_OFFSET); raw=f.read(MEMBER_SIZE)
    if hashlib.sha256(raw).hexdigest()!=EXPECTED:raise ValueError('Unsupported core.idd revision')
    return raw

def parse(raw,dimensions=None):
    if raw[:4]!=b'IDD\0':raise ValueError('IDD magic')
    sections={struct.unpack_from('>I',raw,8+i*8)[0]:struct.unpack_from('>I',raw,12+i*8)[0] for i in range(4)}
    start,end=sections[1],sections[2]
    if raw[start:start+4]!=b'TUV\0' or raw[end:end+4]!=b'IDT\0':raise ValueError('Section framing')
    count=struct.unpack_from('>I',raw,start+16)[0];base=start+20
    if count>1024 or base+8*count>end:raise ValueError('Group count')
    groups=[];sprites=[];expected_first=0
    for i in range(count):
        group,first,n,offset=struct.unpack_from('>4H',raw,base+8*i)
        if first!=expected_first or offset!=count*8+first*20:raise ValueError('Noncontiguous sprite table')
        groups.append(dict(id=group,first=first,count=n,tableOffset=offset))
        for j in range(n):
            at=base+offset+20*j
            if at+20>end:raise ValueError('Sprite outside TUV')
            sid,unknown,*uv=struct.unpack_from('>HH4f',raw,at)
            if sid!=first+j or not all(math.isfinite(x) and 0<=x<=1 for x in uv):raise ValueError('Sprite UV/index')
            width,height=(dimensions[unknown] if dimensions is not None else (1024,1024))
            factors=[width,height,width,height]
            pixels=[round(x*f) for x,f in zip(uv,factors)]
            if any(abs(x*f-p)>1e-4 for x,f,p in zip(uv,factors,pixels)) or pixels[0]>=pixels[2] or pixels[1]>=pixels[3]:raise ValueError('Sprite pixel bounds')
            sprites.append(dict(id=sid,group=group,unknown16=unknown,recordOffset=at,uv=uv,rect=pixels))
        expected_first+=n
    if base+count*8+len(sprites)*20!=end:raise ValueError('Unconsumed TUV bytes')
    return dict(source=dict(path=str(ARCHIVE),member='core.idd',offset=MEMBER_OFFSET,size=MEMBER_SIZE,sha256=EXPECTED),width=1024,height=1024,sectionOffsets=sections,unknownHeaderWord=struct.unpack_from('>I',raw,4)[0],groups=groups,sprites=sprites)

def main():
    result=parse(load_member());out=ROOT/'research'
    (out/'ui-core-layout.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    nodes=[dict(type='node',id='ui:core',kind='ui-atlas',source=result['source'],width=1024,height=1024)]
    for g in result['groups']:
        gid=f"ui:core:group:{g['id']}";nodes.extend([dict(type='node',id=gid,kind='ui-uv-group',**{k:v for k,v in g.items() if k!='id'}),dict(type='edge',source='ui:core',target=gid,kind='contains')])
    for s in result['sprites']:
        sid=f"ui:core:sprite:{s['id']}";nodes.extend([dict(type='node',id=sid,kind='ui-sprite-rect',**{k:v for k,v in s.items() if k!='id'}),dict(type='edge',source=f"ui:core:group:{s['group']}",target=sid,kind='contains')])
    save_graph(out/'ui-core-layout.graph.jsonl',nodes,'Source resource/protocol index; see focused API/INDEX for evidence scope')
    (out/'UI-ARCHIVED-CORE-LAYOUT-INDEX.md').write_text('# Common HUD layout index\n\nReproduce with tools/index_ui_layout.py. ui-core-layout.json has 26 groups and 201 source sprite rectangles; ui-core-layout.graph.jsonl provides typed atlas/group/rectangle nodes. Source member SHA and byte ranges are included.\n\nTUV group descriptors are >4H (group, first, count, offset). Offset is relative to the descriptor-table beginning. Sprite records are >HH4f (global ID, unknown16, normalized UV bounds). All UV edges align to exact source pixels. IDT animation commands and unknown16 semantics remain undecoded; group IDs do not identify navigation kinds.\n',encoding='utf-8')
    (out/'UI-ARCHIVED-CORE-LAYOUT-API.md').write_text('# UI layout vocabulary\n\nui-atlas: owned texture with source provenance. ui-uv-group: contiguous source descriptor range. ui-sprite-rect: exact x0,y0,x1,y1 source pixel bounds and record offset. contains: atlas-to-group or group-to-rectangle relation. This table describes resource layout, not event timing or verified live prompts.\n',encoding='utf-8')
    print(json.dumps(dict(groups=len(result['groups']),sprites=len(result['sprites']),unknown16=sorted(set(s['unknown16'] for s in result['sprites'])))))
if __name__=='__main__':main()
