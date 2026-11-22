"""Stage the native ink inputs for mw-play (crates/mw-game/src/play_ink.rs).

Native: MDB render flags 0x2000 / 0x1000 select the white / black GX TEV ink programs (draw 80257BE0/80257C48,
shader 802612C4/80261840), MDB +0x5F is the ink threshold (shader argument 4), and each mesh samples its TPL texture
together with the same index in the _AB (shadow ink) TPL. The two programs were captured from the native runtime by the
explorer (exports/madworld/evidence/material-programs.json). Writes assets/native/ink/: ink.json (programs, per-model
mesh table in GLB order role_0_mesh_<i>) and <model>-ab-<i>.png (unchanged pixels).
"""
import json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries, mdb, tpl  # noqa: E402

MODELS = {'rinrin': ('em/em0440.dat', 'em0440'), 'jack': ('pl/pl0030.dat', 'pl0030')}
OUT = ROOT / 'assets/native/ink'
OUT.mkdir(parents=True, exist_ok=True)
programs = json.loads((EXPLORER / 'evidence/material-programs.json').read_text(encoding='utf-8'))
models = {}
for name, (dat, stem) in MODELS.items():
    raw = (EXPLORER / 'extracted/disc/DATA/files' / dat).read_bytes()
    es = {e['name']: e for e in dat_entries(raw)}
    member = lambda n: raw[es[n]['offset']:es[n]['offset'] + es[n]['size']]
    meshes = mdb(member(f'{stem}.mdb'))['meshes']
    ab = tpl(member(f'{stem}_AB.tpl'))
    for i, (im, _) in enumerate(ab):
        im.convert('RGBA').save(OUT / f'{name}-ab-{i}.png')
    models[name] = dict(source=f'{dat}:{stem}.mdb', ab=[f'native/ink/{name}-ab-{i}.png' for i in range(len(ab))], meshes=[
        dict(name=m['name'], texture=m['texture'], program='white' if m['renderFlags'] & 0x2000 else 'black' if m['renderFlags'] & 0x1000 else None,
             threshold=m['inkThreshold']) for m in meshes])
(OUT / 'ink.json').write_text(json.dumps(dict(schema='mw-native-ink/1', programs=programs['profiles'], evidence=programs['evidence'], models=models), indent=1))
print({k: (len(v['meshes']), len(v['ab']), sum(1 for m in v['meshes'] if m['program'])) for k, v in models.items()})
