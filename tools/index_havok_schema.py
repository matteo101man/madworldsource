from pathlib import Path
import sys,json,importlib.metadata
from soulstruct.havok.packfile.unpacker import PackFileUnpacker
from soulstruct.havok.enums import HavokModule
from soulstruct.utilities.binary import BinaryReader
from soulstruct.havok.types import hk550
ROOT=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer');from gk.graph import Graph
g=Graph(dict(game='MadWorld',authority='Original Havok5.5 BE32 reflection parsed by soulstruct-havok '+importlib.metadata.version('soulstruct-havok')));catalog={}
for p in (ROOT/'assets/native/collision').glob('*.hkx'):
 u=PackFileUnpacker(havok_module=HavokModule.hk550);u.unpack(BinaryReader(p),types_only=True);rows=[];missing=[]
 for t in u.hk_type_infos[1:]:
  if t is None:continue
  row=dict(name=t.name,pyName=t.py_name,parent=t.parent_type_info.name if t.parent_type_info else None,size=t.byte_size,alignment=t.alignment,flags=t.tag_type_flags,members=[dict(name=m.name,offset=m.offset,type=m.type_py_name,reflectedType=m.type_info.name if m.type_info else None) for m in t.members]);rows.append(row);tid='hk-type:'+t.name;g.add_node(tid,'native-type',t.name,size=t.byte_size,alignment=t.alignment)
  if not hasattr(hk550,t.py_name):missing.append(t.py_name)
  for m in row['members']:mid=tid+':'+m['name'];g.add_node(mid,'native-member',m['name'],**m);g.add_edge(tid,mid,'member')
 catalog[p.name]=dict(version=u.hk_version,headerPointerSize=u.header.pointer_size,classes=u.class_names,types=rows,missing=missing);print(p.name,'types',len(rows),'missing',missing)
(ROOT/'research/havok-schema.json').write_text(json.dumps(catalog,indent=2));g.save(ROOT/'research/havok-schema.graph.jsonl')
