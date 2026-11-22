"""Stage c201's room camera clips (case2/c201.dat cam00_Clip.cam .. cam23_Clip.cam) for mw-play's stage-start broadcast
(crates/mw-game/src/play_broadcast.rs). Samples per 30 Hz frame: eye xyz, target xyz, roll, vertical FOV (radians), via the
explorer's native.camera reader (the same reader the ev21 event cameras use). Writes assets/native/world/cameras.json.
"""
import json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
import native  # noqa: E402

raw = (EXPLORER / 'extracted/disc/DATA/files/case2/c201.dat').read_bytes()
clips = {}
for e in native.dat_entries(raw):
    if e['name'].endswith('.cam'):
        c = native.camera(raw[e['offset']:e['offset'] + e['size']])
        clips[e['name'].removesuffix('_Clip.cam')] = [[round(float(x), 5) for x in s[:8]] for s in c['samples']]
(ROOT / 'assets/native/world/cameras.json').write_text(json.dumps(dict(schema='mw-room-cameras/1', source='case2/c201.dat', clips=clips)))
print({k: len(v) for k, v in clips.items()})
