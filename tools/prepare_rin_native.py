"""Relocate the owned enemy module into an analysis-only DOL memory image.

The source executable is never written. All generated code/data stays on D:.
Relocation layouts use the already validated local MadWorld RSO adapter.
"""
from pathlib import Path
import collections
import hashlib
import json
import re
import struct
import sys

ROOT=Path(__file__).resolve().parents[1]
EXPLORER=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPLORER/'builders'))
from native_graph import rso


def prepare():
    source=EXPLORER/'extracted/disc/DATA/files/dll/Rel/enemyRSO.rso';raw=source.read_bytes()
    sections,exports,imports,external,internal=rso(raw)
    resident=json.loads((EXPLORER/'evidence/resident-symbols.json').read_text(encoding='utf-8'))
    symbolText=(EXPLORER/'tools/upstream-research/symbols.txt').read_text(encoding='utf-8')
    absolute={};functions=[]
    for line in symbolText.splitlines():
        m=re.match(r'(\S+) = \.(\w+):0x([0-9A-Fa-f]+);.*size:0x([0-9A-Fa-f]+)',line)
        if not m:continue
        name,section,address,size=m.groups();absolute[name]=int(address,16)
        if section in ('init','text'):functions.append((int(address,16),int(size,16),name))
    baseCandidates=collections.defaultdict(collections.Counter)
    for item in resident:
        if item['name'] in absolute:baseCandidates[item['section']][absolute[item['name']]-item['address']]+=1
    bases={section:counts.most_common(1)[0][0] for section,counts in baseCandidates.items()}
    bases[65521]=0
    symbols={item['name']:bases[item['section']]+item['address'] for item in resident if item['section'] in bases}
    imageBase=0x81000000;linked=bytearray(raw);relocations=[];unresolved=[]
    sectionBases={};bssCursor=0x81500000;memorySections=[]
    for i,(offset,size) in enumerate(sections):
        if offset:base=imageBase+offset
        elif size:base=bssCursor;bssCursor+=(size+31)&~31
        else:base=0
        sectionBases[i]=base
        if size:memorySections.append(dict(index=i,offset=offset,size=size,address=base,zero=not bool(offset)))
    def patch(offset,kind,target,authority):
        location=imageBase+offset
        if kind==1:struct.pack_into('>I',linked,offset,target)
        elif kind==4:struct.pack_into('>H',linked,offset,target&65535)
        elif kind==6:struct.pack_into('>H',linked,offset,((target+32768)>>16)&65535)
        elif kind==10:
            distance=target-location
            if not -0x2000000<=distance<0x2000000 or distance%4:raise ValueError('BR24 analysis range')
            word=struct.unpack_from('>I',linked,offset)[0];struct.pack_into('>I',linked,offset,(word&0xfc000003)|(distance&0x03fffffc))
        else:raise ValueError('unsupported relocation '+str(kind))
        relocations.append(dict(offset=offset,type=kind,target=target,authority=authority))
    for offset,entry in internal.items():patch(offset,entry['type'],sectionBases[entry['section']]+entry['addend'],'original RSO internal relocation')
    unresolvedNames={}
    for offset,entry in external.items():
        name=entry['name'];target=symbols.get(name)
        if target is None:
            target=unresolvedNames.setdefault(name,0x81700000+4*len(unresolvedNames));unresolved.append(dict(offset=offset,name=name))
        patch(offset,entry['type'],target+entry['addend'],'resident SEL export' if name in symbols else 'unresolved analysis placeholder')
    text=sorted([e for e in exports if e['section']==1],key=lambda e:e['offset'])
    rin=[]
    for i,item in enumerate(text):
        end=text[i+1]['offset'] if i+1<len(text) else sections[1][0]+sections[1][1]
        if end<=item['offset']:continue
        address=imageBase+item['offset'];name='enemy__'+item['name'];functions.append((address,end-item['offset'],name))
        if 'cEm0440' in item['name']:rin.append(dict(name=item['name'],address=address,offset=item['offset'],size=end-item['offset']))
    for name,address in unresolvedNames.items():functions.append((address,4,'unresolved__'+name))
    (ROOT/'tools/enemy-analysis.bin').write_bytes(linked)
    (ROOT/'tools/ghidra-functions.tsv').write_text('\n'.join(f'{a:x}\t{s:x}\t{n}' for a,s,n in functions),encoding='utf-8')
    result=dict(source=str(source),sourceSha256=hashlib.sha256(raw).hexdigest(),linkedSha256=hashlib.sha256(linked).hexdigest(),analysisBase=imageBase,sections=memorySections,residentSectionBases=bases,relocations=len(relocations),unresolved=unresolved,unresolvedSymbols=unresolvedNames,rinFunctions=rin,authority='Original RSO relocations + resident SEL; virtual analysis base is not a runtime placement')
    (ROOT/'research/enemy-link.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    (ROOT/'research/RIN-NATIVE-INDEX.md').write_text('# RinRin native module\n\n'+f'{len(rin)} RinRin/iron-fan exports; {len(relocations)} relocated sites. Unresolved imports: '+', '.join(unresolvedNames)+'\n\n'+'\n'.join(f"- `{e['name']}`: analysis address {e['address']:#x}, source offset {e['offset']:#x}" for e in rin),encoding='utf-8')
    print(dict(rinFunctions=len(rin),relocations=len(relocations),residentBases={s:hex(v) for s,v in bases.items()},unresolvedSymbols=unresolvedNames))


if __name__=='__main__':prepare()
