from pathlib import Path
import struct,json,sys
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph
r=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');b=(r/'tools/enemy-analysis.bin').read_bytes();names={int(x.split('\t')[0],16):x.split('\t')[2] for x in (r/'tools/ghidra-functions.tsv').read_text().splitlines()};states=[];g=Graph.load(r/'research/rinrin-behavior.graph.jsonl')
for i in range(32):
 row=dict(id=i)
 for label,base in [('branch',0x81174800),('update',0x81174980),('end',0x81174b00)]:
  a=base+i*12;delta,index,fn=struct.unpack_from('>3I',b,a-0x81000000)
  assert (delta,index)==(0,0xffffffff)
  name=names[fn];assert 'em0440_r1_' in name
  row[label]=dict(function=name,address=fn,tableAddress=a)
  g.add_node('state:rinrin:'+str(i),'boss-state',str(i),index=i)
  g.add_edge('state:rinrin:'+str(i),'fn:'+format(fn,'x'),label+'-callback',tableAddress=a)
 row['name']=row['update']['function'].split('em0440_r1_')[1].split('__')[0];states.append(row)
g.save(r/'research/rinrin-behavior.graph.jsonl')
(r/'assets/native/rinrin-states.json').write_text(json.dumps(dict(authority='Native member-function tables; behavior predicates and dependencies require separate translation',states=states),indent=2))
(r/'research/RIN-STATES-INDEX.md').write_text('# RinRin native state tables\n\n32 slots, each with branch/update/end callbacks. Tables at analysis addresses 0x81174800, 0x81174980, 0x81174b00; records are CodeWarrior member pointers `(thisDelta=0, virtualIndex=-1, function)`. Native onMove calls the first two tables at 0x810cad90/0x810cadb0. changeState calls the end table at 0x810cb20c. Native source offsets/relocations are in enemy-link.json.\n\n'+ '\n'.join(f"- {x['id']}: `{x['name']}`" for x in states),encoding='utf-8')
print('Validated',len(states),'three-callback native state slots')
