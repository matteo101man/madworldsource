"""Stage the character blob-shadow textures for mw-play (crates/mw-game/src/play_shadow.rs) from etc/core2.dat.

fn_8025BCE8 loads core2.dat\\jkShadow.tpl into texobj 0x806A1A98 (wrap clamp) and core2.dat\\jkNoise.tpl into
0x806A1B18 (wrap repeat); pass 4 of fn_8025BFCC draws every jkShadow quad with them. Both are IA8 64x64: intensity
0 (black) and the shape in alpha. Written as RGBA PNG with RGB = intensity, A = alpha, unchanged.
"""
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries, tpl  # noqa: E402

raw = (EXPLORER / 'extracted/disc/DATA/files/etc/core2.dat').read_bytes()
out = ROOT / 'assets/native/shadow'
out.mkdir(parents=True, exist_ok=True)
for e in dat_entries(raw):
    if e['name'] in ('jkShadow.tpl', 'jkNoise.tpl'):
        (im, info), = tpl(raw[e['offset']:e['offset'] + e['size']])
        im.convert('RGBA').save(out / e['name'].replace('.tpl', '.png'))
        print(e['name'], info)
