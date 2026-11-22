from pathlib import Path
import json,sys,collections
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph
r=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');g=Graph.load(r/'research/rinrin-pcode.graph.jsonl')
names={int(x.split('\t')[0],16):x.split('\t')[2] for x in (r/'tools/ghidra-functions.tsv').read_text().splitlines()}
out=[dict(gk_graph=1,meta=dict(game='MadWorld',authority='Derived from Gekko high Pcode; native resource loads are not gameplay eligibility'))];summaries=[]
for fn in [n for n in g.nodes.values() if n['kind']=='native-function']:
    out.append(fn);calls=[];resources=[]
    for _,op in g.out(fn['id'],('contains',)):
        if op['label']!='CALL':continue
        inputs=sorted(g.inn(op['id'],('feeds',)),key=lambda x:x[0].get('attrs',{}).get('seq',0))
        if not inputs:continue
        target=inputs[0][1]['attrs'].get('offset',0);name=names.get(target,hex(target));cid=f'api:{target:x}'
        out.extend([dict(id=cid,kind='native-api',label=name,attrs=dict(analysisAddress=target)),dict(source=fn['id'],target=cid,relation='calls',attrs=dict(address=op['attrs']['address']))])
        calls.append(dict(api=name,address=op['attrs']['address']))
        if name.startswith('GetData__'):
            for _,v in inputs[1:]:
                s=v.get('attrs',{}).get('string')
                if s:
                    rid='resource:'+s;resources.append(s)
                    out.extend([dict(id=rid,kind='native-resource',label=s,attrs=dict()),dict(source=fn['id'],target=rid,relation='loads',attrs=dict(address=op['attrs']['address']))])
    summaries.append(dict(function=fn['label'],address=fn['attrs']['address'],resources=resources,calls=calls))
# Graph wire format matches the source graph: node/edge wrappers.
nodes={};edges=[]
for x in out[1:]:
    if 'id' in x:nodes[x['id']]=x
    else:edges.append(x)
with (r/'research/rinrin-behavior.graph.jsonl').open('w',encoding='utf-8') as f:
    f.write(json.dumps(out[0])+'\n')
    for x in nodes.values():f.write(json.dumps(dict(n=x),ensure_ascii=False)+'\n')
    for x in edges:f.write(json.dumps(dict(e=dict(src=x['source'],dst=x['target'],kind=x['relation'],attrs=x['attrs'])),ensure_ascii=False)+'\n')
(r/'research/rinrin-state-resources.json').write_text(json.dumps(summaries,indent=2),encoding='utf-8')
(r/'research/RIN-BEHAVIOR-API.md').write_text('# Native behavior query\n\n`native-function --calls--> native-api` records direct calls and native instruction addresses. `native-function --loads--> native-resource` records literal resource arguments to ResourceManager::GetData. These prove resource requests in function bodies, not branch eligibility or runtime timing. Analysis addresses use the relocated base in enemy-link.json.\n\nQuery: `python -m gk graph stats research/rinrin-behavior.graph.jsonl`. Detailed dependency query: `python tools/query_rin.py . 810cb74c`.\n',encoding='utf-8')
print(json.dumps(dict(functions=len(summaries),resourceLoads=sum(len(x['resources']) for x in summaries),resources=len([x for x in nodes.values() if x['kind']=='native-resource']))))
for x in summaries:
    if x['resources']:print(x['function'].replace('enemy__',''),x['resources'])
