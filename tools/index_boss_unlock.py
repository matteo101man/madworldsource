"""Index native Unlock5 storage/getter provenance; runtime gating is not inferred."""
from pathlib import Path
import hashlib,json,sys
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools/upstream-native-core/port/recomp'))
from dol import Dol

def main():
    source=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol')
    dol=Dol(source)
    # Original parser copies120 key pointers from8055ECE0 into a local array.
    # Its ordinary point branch stores this+0x38+4*keyIndex. The owning list
    # parses room data at this+0xB8, then copies100 values at+0x120 to+0x7A0.
    records=[]
    for index in range(120):
        site=0x8055ece0+4*index;address=dol.read32(site);raw=bytearray()
        for offset in range(100):
            word=dol.read32((address+offset)&~3)
            if word is None:raise ValueError('key pointer outside original data')
            value=word.to_bytes(4,'big')[(address+offset)&3]
            if not value:break
            raw.append(value)
        key=raw.decode('cp932')
        if key=='アンロック5':
            room_offset=0x38+4*index;list_offset=0xb8+room_offset
            enum=(list_offset-0x120)//4
            records.append(dict(key=key,keyIndex=index,keyPointerSite=hex(site),
                parserOffset=hex(room_offset),listParsedOffset=hex(list_offset),
                pointEnum=enum,listFinalOffset=hex(0x7a0+4*enum)))
    assert len(records)==1 and records[0]['pointEnum']==24
    calls=json.loads((ROOT/'research/point-value-call-leads.json').read_text(encoding='utf-8'))
    candidates=[row for row in calls if row['enumLead']==24]
    graph=[{'gk_graph':1,'meta':{'game':'MadWorld','authority':'Original DOL data plus focused native Pcode dependency evidence; final eligibility unknown'}},
        {'n':{'id':'key:unlock5','kind':'native-config-key','label':'Unlock5 boss threshold','attrs':records[0]}},
        {'n':{'id':'fn:801e5fe8','kind':'native-function','label':'Room INI parser'}},
        {'n':{'id':'fn:801dbb14','kind':'native-function','label':'Enemy-list room data initialization'}},
        {'n':{'id':'fn:801e3aac','kind':'native-function','label':'getPointValue'}},
        {'e':{'src':'fn:801e5fe8','dst':'key:unlock5','kind':'parses-storage','attrs':{'site':'801e6240','ordinaryBranch':'atof then native integer truncation; branch eligibility must be retained'}}},
        {'e':{'src':'fn:801dbb14','dst':'key:unlock5','kind':'copies-parameter','attrs':{'site':'801dc1ac','count':100,'sourceOffset':'0x120','destinationOffset':'0x7a0','zeroHandling':'conditional original branch; no synthetic substitution'}}},
        {'e':{'src':'fn:801e3aac','dst':'key:unlock5','kind':'reads-parameter','attrs':{'enum':24,'objectOffset':'0x800'}}}]
    for row in candidates:
        nid='fn:'+row['function'].removeprefix('0x')
        graph.extend([{'n':{'id':nid,'kind':'native-function','label':row['name']}},
            {'e':{'src':nid,'dst':'fn:801e3aac','kind':'calls-lead','attrs':row}}])
    graph.append({'e':{'src':'fn:80145934','dst':'key:unlock5','kind':'reads-parameter','attrs':{'site':'8014633c','enum':24,'authority':'Focused Pcode direct call, stage201 EMS resources referenced in same function; downstream condition unknown'}}})
    (ROOT/'research/boss-unlock.graph.jsonl').write_text(''.join(json.dumps(x,ensure_ascii=False)+'\n' for x in graph),encoding='utf-8')
    result=dict(dolSha256=hashlib.sha256(source.read_bytes()).hexdigest(),parameter=records[0],candidateCalls=candidates,
        known=['Stage201 setup function80145934 registers BossSw/invoke and reads enum24 at8014633c','Original room INI Unlock5 value is2000000'],
        unknown=['Live threshold after all room merges','Exact score/state comparison and switch transition','Native Rin entrance and encounter closure'])
    (ROOT/'research/boss-unlock.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'parameter':records[0],'candidateCalls':len(candidates)},ensure_ascii=True))
if __name__=='__main__':main()
