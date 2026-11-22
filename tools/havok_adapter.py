"""Extend the external HKX tool in-process using the owned file's reflected class layouts."""
from pathlib import Path
import sys,json,dataclasses,traceback,importlib.metadata
from soulstruct.havok import HKX
from soulstruct.havok.packfile.unpacker import PackFileUnpacker
from soulstruct.havok.enums import HavokModule
from soulstruct.utilities.binary import BinaryReader
from soulstruct.havok.types import hk550
from soulstruct.havok.types import hk as base_hk
ROOT=Path(__file__).resolve().parents[1]

def install_reflected_types(paths):
 info={}
 for path in paths:
  u=PackFileUnpacker(havok_module=HavokModule.hk550);u.unpack(BinaryReader(path),types_only=True)
  for t in u.hk_type_infos[1:]:
   if t and (not hasattr(hk550,t.py_name) or t.py_name in ('hkpSimpleMeshShape','hkpSimpleMeshShapeTriangle')):info.setdefault(t.py_name,t)
 namespace=vars(hk550)
 exec('from dataclasses import dataclass\nfrom soulstruct.havok.enums import *\nfrom soulstruct.havok.types.hk550.core import *',namespace)
 pending=dict(info);generated=[];errors={}
 while pending:
  progress=False
  for name,t in list(pending.items()):
   try:
    definition=t.get_class_py_def(list(namespace))
    exec('from __future__ import annotations\n'+definition,namespace)
    generated.append(dict(name=name,definition=definition));del pending[name];progress=True
   except Exception as ex:errors[name]=str(ex)
  if not progress:break
 (ROOT/'research/havok-generated-types.json').write_text(json.dumps(dict(authority='Soulstruct-havok class generation from original Havok reflection; in-process extension, upstream source not merged',generated=generated,unsupported={k:errors[k] for k in pending}),indent=2))
 print('Generated types',[x['name'] for x in generated],'unsupported',{k:errors[k] for k in pending})
 return generated

if __name__=='__main__':
 paths=list((ROOT/'assets/native/collision').glob('*.hkx'));install_reflected_types(paths);reports=[]
 for path in paths:
  try:
   hk=HKX.from_path(path);report=dict(file=path.name,status='parsed',rootType=type(hk.root).__name__,version=hk.hk_version)
  except Exception as ex:report=dict(file=path.name,status='unsupported',error=str(ex))
  reports.append(report);print(report)
 (ROOT/'research/havok-tool-probe.json').write_text(json.dumps(dict(tool='soulstruct-havok',version=importlib.metadata.version('soulstruct-havok'),results=reports),indent=2))
