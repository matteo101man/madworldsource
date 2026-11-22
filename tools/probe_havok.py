from pathlib import Path
import sys,json,hashlib,importlib.metadata
ROOT=Path('D:/Games/gamesfordecomp/ports/madworld-rinrin');sys.path.insert(0,'D:/Games/gamesfordecomp/exports/madworld/builders');from native import dat_entries
b=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files/case2/c201.dat').read_bytes();out=ROOT/'assets/native/collision';out.mkdir(exist_ok=True)
for e in dat_entries(b):
 if e['name'].endswith('.hkx'):(out/e['name']).write_bytes(b[e['offset']:e['offset']+e['size']])
try:
 from soulstruct.havok import HKX
 h=HKX.from_path(out/'col_pl.hkx');print(type(h),h.hk_version,type(h.root));print(h.root.get_type_name())
 (ROOT/'research/havok-tool-probe.json').write_text(json.dumps(dict(version=importlib.metadata.version('soulstruct-havok'),sample='col_pl.hkx',status='parsed',rootType=type(h.root).__name__),indent=2))
except Exception as ex:
 import traceback;traceback.print_exc()
 (ROOT/'research/havok-tool-probe.json').write_text(json.dumps(dict(sample='col_pl.hkx',status='failed',error=str(ex)),indent=2))
