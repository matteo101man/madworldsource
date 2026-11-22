"""Stage mw-play's street stage (crates/mw-game/src/play_street.rs): the kung-fu grunts of Asia Town's Great Wall Street.

c201.dat is the whole Asia Town level; the RinRin fight uses spawn set c20110 at the north end. c20100 is the streets'
set: 26 spawns, all family 0 id 0x101 (em0101, kung fu; case2/c201em.ini). The EMS layout is stage_arena.py's (version
20, 128-byte records from +32: native id u16 at +0, family u8 at +0x4D, position at +0x10). Writes
assets/native/world/street-meta.json: the player start (PlayerTbl point 0 "start", research/stage201-player-points.json)
and the grunt spawns, ground-level ones first (the rest stand on rooftops).
"""
import json, struct, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries  # noqa: E402

raw = (EXPLORER / 'extracted/disc/DATA/files/case2/c201.dat').read_bytes()
entry = next(e for e in dat_entries(raw) if e['name'] == 'c20100.ems')
ems = raw[entry['offset']:entry['offset'] + entry['size']]
version, n = struct.unpack_from('>HH', ems, 4)
assert version == 20 and len(ems) >= 32 + n * 128, 'unsupported spawn set layout'
spawns = []
for i in range(n):
    o = 32 + i * 128
    native_id = struct.unpack_from('>H', ems, o)[0]; family = ems[o + 0x4D]
    x, y, z = struct.unpack_from('>3f', ems, o + 0x10)
    spawns.append(dict(index=i, family=family, id=native_id, position=[round(x, 3), round(y, 3), round(z, 3)]))
spawns.sort(key=lambda s: (s['position'][1] > 1.0, s['index']))
start = dict(position=[-2.0, 0.0, 34.0], yaw_deg=180.0)
(ROOT / 'assets/native/world/street-meta.json').write_text(json.dumps(dict(schema='mw-street/1', source='case2/c201.dat#c20100.ems',
                                                                          start=start, spawns=spawns), indent=1))
print(len(spawns), 'spawns;', sum(s['position'][1] < 1. for s in spawns), 'at street level;', sorted(set((s['family'], hex(s['id'])) for s in spawns)))
