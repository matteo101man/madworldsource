"""Stage the native combat data for the pure-Rust slice -> assets/native/combat.json (gitignored, disc-derived).

python -I tools/pl0000/stage_combat.py
- cues: every pl0030_* / em0440_* motion-cue sequence (research/motion-cues.json, read from the owned disc) as
  {type, frame, a (s16 @+0xC), b (s16 @+0xE), c (s16 @+0x10), d (s16 @+0x12), f (f32 @+0x4)}
  (event layout: 48-byte records, type u8 @0, frame f32 @0x14; field offsets from the player cue handler 802C1C7C)
- damage_pl / damage_em: rows of files/plem_common.ini (column meanings in research/PL0000-COMBAT.md)
- atk_names_pl / atk_names_em: ATHIT_ATK_TYPE -> row name tables in main.dol (0x805588B4 x168, 0x80558DF4 x94),
  the tables cAtHitInfo's CSV loader (801B59A0) matches row names against
- rin_tuning: files/em/em0440.csv values (normal, hard) by parameter index
"""
import json, struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DISC = Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files')
sys.path.insert(0, str(ROOT / 'tools/pl0000'))
from dol import Dol


def s16(w, hi): return struct.unpack('>h', struct.pack('>H', (w >> 16) & 0xffff if hi else w & 0xffff))[0]


def cue(e):
    w = e['words']
    return dict(type=e['type'], frame=e['frame'], a=s16(w[2], True), b=s16(w[2], False), c=s16(w[3], True),
                d=s16(w[3], False), f=struct.unpack('>f', struct.pack('>I', w[0]))[0])


def ini_rows(text, section):
    rows, on = {}, False
    for line in text.splitlines():
        line = line.split(';')[0].strip()
        if line.startswith('['): on = line == f'[{section}]'; continue
        if not on or '=' not in line: continue
        name, vals = line.split('=', 1)
        cols = [v.strip() for v in vals.replace('\t', ' ').split(',')]
        rows[name.strip()] = [float(v) if v else None for v in cols]
    return rows


def main():
    seqs = json.loads((ROOT / 'research/motion-cues.json').read_text(encoding='utf-8'))['sequences']
    cues = {s['name']: [cue(e) for e in s['events']] for s in seqs if s['name'].startswith(('pl0030_', 'em0440_', 'em0630_', 'em0330_', 'em0623_', 'em0610_', 'em0615_', 'em0960_', 'em0965_'))}
    ini = (DISC / 'plem_common.ini').read_bytes().decode('cp932')
    d = Dol()
    csv = (DISC / 'em/em0440.csv').read_bytes().decode('cp932').splitlines()[1:]
    tuning = []
    for line in csv:
        parts = line.rsplit(',', 2)
        if len(parts) == 3 and parts[1].strip():
            tuning.append([float(parts[1]), float(parts[2])])
    sys.path.insert(0, str(Path('D:/Games/gamesfordecomp/exports/madworld/builders')))
    from native import dat_entries
    dat = (DISC / 'em/em0440.dat').read_bytes()
    emh = next(dat[e['offset']:e['offset'] + e['size']] for e in dat_entries(dat) if e['name'] == 'em0440_00.emh' and e['size'])
    assert emh[:4] == b'emh\0'
    count = struct.unpack_from('>H', emh, 6)[0]
    vols = []
    for i in range(count):
        o = 0x10 + i * 0x80
        bone, aux = struct.unpack_from('>Hh', emh, o)
        off = struct.unpack_from('>4f', emh, o + 0x10); ext = struct.unpack_from('>3f', emh, o + 0x20)
        vols.append(dict(bone=bone, aux=aux, offset=off[:3], radius=off[3], extent=ext))
    out = dict(cues=cues, damage_pl=ini_rows(ini, 'damage_pl'), damage_em=ini_rows(ini, 'damage_em'), rin_damage_volumes=vols,
               atk_names_pl={str(d.u32(0x805588B8 + i * 8)): d.cstr(d.u32(0x805588B4 + i * 8)) for i in range(0xA8)},
               atk_names_em={str(d.u32(0x80558DF8 + i * 8)): d.cstr(d.u32(0x80558DF4 + i * 8)) for i in range(0x5E)},
               rin_tuning=tuning[:32])
    dst = ROOT / 'assets/native/combat.json'
    dst.write_text(json.dumps(out, separators=(',', ':')), encoding='utf-8')
    print(json.dumps(dict(cues=len(cues), damage_pl=len(out['damage_pl']), damage_em=len(out['damage_em']),
                          tuning=len(out['rin_tuning']), bytes=dst.stat().st_size)))


if __name__ == '__main__':
    main()
