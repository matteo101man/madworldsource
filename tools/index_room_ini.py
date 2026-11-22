from pathlib import Path
import sys,json,hashlib,re
r=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');p=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files/case2/c201em.ini');raw=p.read_bytes()
for codec in ('utf-8-sig','cp932'):
 try:text=raw.decode(codec);break
 except UnicodeDecodeError:pass
else:raise ValueError('unrecognized room INI encoding')
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer');from gk.graph import Graph
g=Graph(dict(game='MadWorld',authority='Original room INI ordered keys/values; duplicate keys retained; consumer semantics not inferred'));g.add_node('c201em.ini','room-ini','c201em.ini',sha256=hashlib.sha256(raw).hexdigest(),encoding=codec);section='';rows=[];counts={}
for i,line in enumerate(text.splitlines(),1):
 line=line.strip()
 if not line or line.startswith((';','#','//')):continue
 if re.match(r'^\[[^\]]+\]',line):
  section=re.match(r'^\[([^\]]+)\]',line)[1];sid='section:'+section;g.add_node(sid,'room-ini-section',section);g.add_edge('c201em.ini',sid,'contains');continue
 if '=' not in line:continue
 key,value=(s.strip() for s in line.split('=',1));nid='entry:'+str(i);g.add_node(nid,'room-ini-entry',key,value=value,line=i);g.add_edge('section:'+section,nid,'contains',line=i);rows.append(dict(section=section,key=key,value=value,line=i));counts[section]=counts.get(section,0)+1
g.save(r/'research/room-ini.graph.jsonl');(r/'research/room-ini.json').write_text(json.dumps(dict(encoding=codec,entries=rows),ensure_ascii=False,indent=2),encoding='utf-8');print('encoding',codec,'sections',counts);print('point-related', [x for x in rows if any(k in (x['key']+x['value']).lower() for k in ('point','score','boss','得点','スコア','ポイント','ボス'))])
