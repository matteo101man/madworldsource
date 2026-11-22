"""Index observed native room bindings and the bounded original boss-entry call path."""
from pathlib import Path
import hashlib,json,re,sys
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
sys.path.insert(0,str(ROOT/'tools/upstream-native-core/port/recomp'))
from gk.graph import Graph
from dol import Dol
from recomp import load_functions

def main():
    original=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol')
    dol=Dol(original)
    functions=load_functions(ROOT/'tools/upstream-native-core/config/RZZE8P/symbols.txt')
    by_addr={f.addr:f for f in functions}
    graph=Graph(dict(game='MadWorld',authority='Native runtime record fields plus original DOL direct calls; task scheduling/timing remains unverified',dolSha256=hashlib.sha256(original.read_bytes()).hexdigest()))
    rows=[]
    pattern=re.compile(r'\[room-binding\] record=([0-9a-f]+) key=(-?\d+) flags=([0-9a-f]+) fields08to18=([0-9a-f/]+)')
    for match in pattern.finditer((ROOT/'evidence/native-core-room-bindings.log').read_text(encoding='utf-8')):
        if len(rows)==19:break  # Original constructor801447EC initializes19, not20.
        record,key,flags,words=match.groups();values=[int(v,16) for v in words.split('/')]
        assert len(values)==5 and values[1:3]==[0,0xffffffff]
        row=dict(key=int(key),record=record,flags=flags,owner=hex(values[0]),callback=hex(values[3]),invoker=hex(values[4]))
        rows.append(row)
        nid='room-event:'+key;target='fn:'+f'{values[3]:08x}'
        graph.add_node(nid,'native-room-event','Stage201 event '+key,**row)
        graph.add_node(target,'native-function',by_addr[values[3]].name)
        graph.add_edge(nid,target,'native-binding',authority='Observed record offsets00/04/08..18; direct member descriptor0/-1/callback')
    assert len(rows)==19 and next(x for x in rows if x['key']==-97)['callback']=='0x801470c4'
    sources=(0x801470c4,0x801479e0,0x80147a4c,0x80147b7c,0x80147bbc)
    for address in sources:
        f=by_addr[address];src='fn:'+f'{address:08x}';graph.add_node(src,'native-function',f.name)
        for pc in range(f.addr,f.end,4):
            word=dol.read32(pc)
            if word is None or word>>26!=18:continue
            off=word&0x3fffffc
            if off&0x2000000:off-=0x4000000
            target=(off if word&2 else pc+off)&0xffffffff
            dst='fn:'+f'{target:08x}';name=by_addr[target].name if target in by_addr else 'unknown native entry'
            graph.add_node(dst,'native-function',name,address=hex(target))
            graph.add_edge(src,dst,'native-call' if word&1 else 'native-tail-transfer',site=hex(pc))
    graph.add_node('event:21','native-event-resource','Original event21',frames=1080,authority='Prior EVNT resource index; live cadence not inferred')
    graph.add_node('resource:c20110.ems','native-enemy-set','c201.dat/c20110.ems')
    graph.add_node('player-point:5','native-player-point','PLP boss point5')
    for target,kind,site in [('event:21','requests-event','80147c68'),('resource:c20110.ems','loads-resource','80147c94'),('player-point:5','sets-player-point','80147cb8')]:
        graph.add_edge('fn:80147bbc',target,kind,site=site,authority='Focused boss Pcode; order in API, blocking timing not yet proven')
    graph.save(ROOT/'research/stage201-native-logic.graph.jsonl')
    (ROOT/'research/stage201-native-logic.json').write_text(json.dumps(dict(bindings=rows,sourceLog='evidence/native-core-room-bindings.log',bossEntry='0x80147bbc',unknown=['Native task scheduling and waits','Intro completion and actual Rin actor creation','Gameplay/render/audio equivalence']),indent=2)+'\n',encoding='utf-8')
    check=Graph.load(ROOT/'research/stage201-native-logic.graph.jsonl')
    print(json.dumps(dict(bindings=len(rows),nodes=len(check.nodes),edges=len(check.edges))))
if __name__=='__main__':main()
