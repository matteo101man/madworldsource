from pathlib import Path
import struct,json,sys
r=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer');from gk.graph import Graph
b=(r/'tools/enemy-analysis.bin').read_bytes();names={int(a,16):n for a,s,n in (x.split('\t') for x in (r/'tools/ghidra-functions.tsv').read_text().splitlines())};g=Graph(dict(game='MadWorld',authority='Bounded analysis-relocated cEm0440 primary vtable region; zero/unknown entries explicit; runtime address mapping separate'));base=0x81174d10;rows=[];g.add_node('rinrin-primary-vtable','native-vtable','cEm0440 primary',analysisAddress=base)
for offset in range(0,0x468,4):
 target=struct.unpack_from('>I',b,base-0x81000000+offset)[0];row=dict(offset=offset,value=target,function=names.get(target));rows.append(row)
 if target in names:
  fid=f'fn:{target:08x}';g.add_node(fid,'native-function',names[target],analysisAddress=target);g.add_edge('rinrin-primary-vtable',fid,'virtual-slot',offset=offset)
 else:g.add_node(f'vtable-unknown:{offset}','native-vtable-entry','null' if not target else 'unresolved',value=target,offset=offset);g.add_edge('rinrin-primary-vtable',f'vtable-unknown:{offset}','unknown-slot',offset=offset)
g.save(r/'research/rinrin-vtable.graph.jsonl');(r/'research/rinrin-vtable.json').write_text(json.dumps(rows,indent=2));print([(hex(x['offset']),hex(x['value']),x['function']) for x in rows if x['function'] and any(k in x['function'] for k in ('onInit__7cEm0440','onPreMove__7cEm0440','onMove__7cEm0440'))])
