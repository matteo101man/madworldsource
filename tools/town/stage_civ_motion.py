"""Stage the thug's own locomotion for the town's civilians [added]: em0110's idle (em0110.dat) and the common zako
bank's walks, turns and runs (em0000.dat), on the bone ids the civilians share. Writes assets/native/civ/civ-motion.json
(clips only, the kungfu bank's format) and prints each clip's frames and root travel.
Run: python -I tools/town/stage_civ_motion.py"""
from pathlib import Path
import json, sys
ROOT = Path(__file__).resolve().parents[2]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries, motion  # noqa: E402

WANT = {'em0110': ['em0110_0000', 'em0110_0038', 'em0110_0039'],
        'em0000': ['em0110_00%02x' % k for k in [3, 4, 5, 7, 8, 0xa, 0xb, 0xc, 0x10, 0x14, 0x19, 0x1a, 0x1b, 0x1c, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x32, 0x37, 0x3a, 0x3b]]}
# other townsfolk-like banks whose rigs share the thug's body bone ids 0-24: their own locomotion (xxxx_00..)
from native import mdb  # noqa: E402
for bank in ['em0131', 'em0141', 'em0150', 'em0155', 'em0160', 'em0170', 'em0190', 'em0200', 'em0210']:
    raw = (EXPLORER / f'extracted/disc/DATA/files/em/{bank}.dat').read_bytes()
    ents = dat_entries(raw)
    model = next((e for e in ents if e['name'] == bank + '.mdb' and e['size']), None)
    if not model: continue
    ids = {b['id'] for b in mdb(raw[model['offset']:model['offset'] + model['size']])['bones']}
    if not set(range(25)) <= ids: print('skip', bank, 'rig'); continue
    WANT[bank] = [e['name'][:-4] for e in ents if e['size'] and e['name'].endswith('.mot') and e['name'][:-4].split('_')[1][:2] == '00'
                  and not e['name'].startswith('em0110')]
clips = []
for bank, names in WANT.items():
    raw = (EXPLORER / f'extracted/disc/DATA/files/em/{bank}.dat').read_bytes()
    for e in dat_entries(raw):
        if e['size'] and e['name'][:-4] in names and not any(c['name'] == e['name'] for c in clips):
            c = motion(raw[e['offset']:e['offset'] + e['size']])
            c.update(id=f'em/{bank}.dat#{e["name"]}@{e["offset"]:x}', name=e['name'])
            clips.append(c)
out = ROOT / 'assets/native/civ/civ-motion.json'
out.write_text(json.dumps(dict(clips=clips), separators=(',', ':')))
for c in clips:
    root = {t['channel']: t['values'] for t in c['tracks'] if t['bone'] in (0, -1) and t['channel'] in (0, 2)}
    tr = {k: round(v[-1] - v[0], 2) for k, v in root.items()}
    print(c['name'], c['frames'], 'root', tr, 'tracks', len(c['tracks']))

# the actor editor's list for the civ actor (names and lengths only: the clips themselves load from the two banks)
kf = json.loads((ROOT / 'assets/native/kungfu-motion.json').read_text())['clips']
seen, listing = set(), []
jack = [c for c in json.loads((ROOT / 'assets/native/jack-player-motion.json').read_text())['clips'] if c['name'] in ('pl0030_0000.mot', 'pl0030_0031.mot', 'pl0030_0032.mot', 'pl0030_0033.mot')]
for c in kf + clips + jack:
    if c['name'] in seen: continue
    seen.add(c['name']); listing.append(dict(name=c['name'], frames=c['frames']))
(ROOT / 'assets/native/civ/civ-editor-motion.json').write_text(json.dumps(dict(clips=listing), indent=0))
print('editor list', len(listing))
