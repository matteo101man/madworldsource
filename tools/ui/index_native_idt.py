"""Index native IDT widget provenance and match bounded own-core UI observations.
Header hashes disambiguate source records; a hash may legitimately occur in several screens.
Native directory/record consumption: 8024BD90 / 8024B81C / 80184A1C.
"""
from pathlib import Path
import argparse,collections,hashlib,json,struct
ROOT=Path(__file__).resolve().parents[2]

def index():
    resources=[]
    for path in sorted((ROOT/'assets/native/ui').glob('*/animation.idt')):
        raw=path.read_bytes()
        def u32(at):
            if at<0 or at+4>len(raw):raise ValueError('IDT index bounds')
            return struct.unpack_from('>I',raw,at)[0]
        if raw[:4]!=b'IDT\0':raise ValueError('IDT magic')
        count=u32(4)
        if count>1024 or 8+count*8>len(raw):raise ValueError('IDT program count')
        programs=[]
        for i in range(count):
            ident=u32(8+8*i);base=u32(12+8*i);n=u32(base);end=u32(20+8*i) if i+1<count else len(raw)
            if n>256 or base+4+4*n>end:raise ValueError('IDT widget count')
            widgets=[];previous=base+4+4*n
            for j in range(n):
                at=base+u32(base+4+4*j);size=u32(at)
                if at<previous or size<0x220 or at+size>end:raise ValueError('IDT widget record')
                widgets.append(dict(index=j,id=u32(at+0x2c),kind=u32(at+0x3c),flags=hex(u32(at+0x34)),
                    offset=at,bytes=size,headerSha256=hashlib.sha256(raw[at:at+0xb0]).hexdigest(),
                    recordSha256=hashlib.sha256(raw[at:at+size]).hexdigest(),sourceIndices=list(struct.unpack_from('>6h',raw,at+0xa0))))
                previous=at+size
            programs.append(dict(id=ident,offset=base,widgets=widgets))
        resources.append(dict(id=path.parent.name,path=path.relative_to(ROOT).as_posix(),sha256=hashlib.sha256(raw).hexdigest(),programs=programs))
    return dict(schema='mw-ui-idt-index/1',scope='Native source widget records; hashes are not live layout/animation equivalence proof',resources=resources)

def main():
    p=argparse.ArgumentParser();p.add_argument('--snapshot',type=Path);p.add_argument('--out',type=Path);p.add_argument('--resource');p.add_argument('--program',type=int);a=p.parse_args()
    data=index();out=a.out or ROOT/'research/ui-idt-source-index.json'
    if a.snapshot:
        snapshot=json.loads(a.snapshot.read_text(encoding='utf-8'));by_hash=collections.defaultdict(list)
        for resource in data['resources']:
            for program in resource['programs']:
                for widget in program['widgets']:
                    by_hash[widget['headerSha256']].append(dict(resource=resource['id'],program=program['id'],index=widget['index'],id=widget['id']))
        source=snapshot['uiWidgets'];matches=[]
        for widget in source['widgets']:
            matches.append(dict(pointer=widget['pointer'],id=widget['id'],cluster=widget['cluster'],hidden=widget['hidden'],
                candidates=by_hash[widget['sourceHeaderSha256']]))
        if a.resource is not None or a.program is not None:
            resource=next(r for r in data['resources'] if r['id']==a.resource)
            program=next(p for p in resource['programs'] if p['id']==a.program)
            candidates=collections.defaultdict(list)
            for widget in program['widgets']:candidates[widget['headerSha256']].append(widget)
            bases=collections.Counter(int(w['sourceRecord'],16)-c['offset'] for w in source['widgets'] for c in candidates[w['sourceHeaderSha256']])
            ranked=bases.most_common()
            if not ranked or ranked[0][1]<min(20,len(program['widgets'])) or (len(ranked)>1 and ranked[0][1]==ranked[1][1]):
                raise ValueError('Ambiguous or insufficient live/source record-base evidence')
            base=ranked[0][0];by_offset={w['offset']:w for w in program['widgets']};bindings=[]
            for widget in source['widgets']:
                original=by_offset.get(int(widget['sourceRecord'],16)-base)
                if original is not None:
                    if widget['id']!=original['id'] or widget['kind']!=original['kind']:raise ValueError('Source-base binding inconsistent')
                    bindings.append(dict(index=original['index'],sourceId=original['id'],observed=widget))
            data=dict(schema='mw-ui-program-observation/1',snapshot=str(a.snapshot),resource=a.resource,program=a.program,
                      recordBase=hex(base),exactHeaderMatches=ranked[0][1],bindings=bindings,listReachedEnd=source['reachedEnd'],
                      scope='Program binding supported by exact header matches and shared source base; geometry remains a concurrent observation')
        else:
            data=dict(schema='mw-ui-observation-match/1',snapshot=str(a.snapshot),listReachedEnd=source['reachedEnd'],widgets=matches,
                      scope='Concurrent source header matches; candidates may be ambiguous, transforms are not compared here')
    out.write_text(json.dumps(data,indent=1)+'\n',encoding='utf-8')
    print(json.dumps(dict(output=str(out),resources=len(data.get('resources',[])),widgets=len(data.get('widgets',[])))))
if __name__=='__main__':main()
