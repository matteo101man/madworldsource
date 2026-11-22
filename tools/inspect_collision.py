from pathlib import Path
import sys,dataclasses,collections
ROOT=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');sys.path.insert(0,str(ROOT/'tools'));from havok_adapter import install_reflected_types
from soulstruct.havok import HKX
paths=list((ROOT/'assets/native/collision').glob('*.hkx'));install_reflected_types(paths)
for p in paths:
 h=HKX.from_path(p);print(p.name,'variants',[(v.name,type(v.variant).__name__) for v in h.root.namedVariants])
 for v in h.root.namedVariants:
  if hasattr(v.variant,'systems'):
   for system in v.variant.systems:
    print('bodies',len(system.rigidBodies),'shapes',collections.Counter(type(body.collidable.shape).__name__ for body in system.rigidBodies))
    if system.rigidBodies:
     body=system.rigidBodies[0];shape=body.collidable.shape;print('first',body.name,'shape fields',[(f.name,type(getattr(shape,f.name)).__name__) for f in dataclasses.fields(shape)]);print('transform',type(body.motion.motionState.transform).__name__,str(body.motion.motionState.transform)[:180])
