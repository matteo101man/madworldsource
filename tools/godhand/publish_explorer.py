"""Publish Kreely's God Hand moveset (assets/native/kreely-moves.json, tools/godhand/retarget_godhand.py) to the MadWorld
explorer (D:/Games/gamesfordecomp/exports/madworld) for inspection: every clip as an explorer custom clip
(custom/clips/<name>.json + custom/clips/index.json, labelled "Kreely · <move> ..."), RinRin's receiver clips, and two
paired studies in app.js's PAIRS: the suplex (Kreely / RinRin, the grab's placement baked into her root so the pair
lines up as in mw-play) and the sweep (her knockdown starting on the kick's contact). Re-run after retargeting.
Usage: python tools/godhand/publish_explorer.py
"""
import hashlib, json, math, re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
CLIPS = EXPLORER / 'custom/clips'

LABELS = {
    'cross': ('Cross punch', 'Gene 150'), 'cross_m': ('Cross punch, mirrored', 'Gene 150 mirrored'), 'knee': ('Knee', 'Gene 119'),
    'uppercuts': ('Left / right uppercuts (not in the string)', 'Gene 105'), 'step_jab': ('Forward step jab (W + punch)', 'Elvis 373'),
    'turn_jab': ('Turning jab (S + punch)', 'Small Fry 535'), 'power_punch': ('Powered punch (Q)', 'Gene 154'),
    'spin_kick': ('Spin kick, knockdown (W + Q)', 'Gene-rig 351'), 'sweep': ('Sweep kick, knockdown (S + Q)', 'Gene 166'),
    'rush': ('Rush combo (mash punch)', 'Small Fry 256'), 'whip': ('Pistol whip', 'Gene 063'), 'whip_alt': ('Pistol whip, alternate', 'Gene 069'),
    'suplex': ('Suplex (hold punch / F)', 'grab 0011c348, the suplexer'),
}

def rot(yaw, x, z):
    c, s = math.cos(yaw), math.sin(yaw)
    return c * x + s * z, -s * x + c * z

def write(name, clip, label, derived, extra_authority=''):
    body = dict(frames=clip['frames'], tracks=clip['tracks'], headerFlag=0, fps=30,
                fpsEvidence='God Hand explorer clips are one source frame per 30 Hz tick; mw-play plays them at 30 Hz')
    path = CLIPS / f'{name}.json'
    data = json.dumps(body, separators=(',', ':'))
    path.write_text(data, encoding='utf-8')
    bones = sorted({t['bone'] for t in clip['tracks']})
    return dict(id=f'custom/clips#{name}', bank='custom/clips', name=name, frames=clip['frames'], bones=bones, reviewedLabel=label,
                url=f'custom/clips/{name}.json', sha256=hashlib.sha256(data.encode()).hexdigest(), derivedFrom=derived,
                authority='added animation: God Hand motion retargeted onto the pl0030 rig by tools/godhand/retarget_godhand.py' + extra_authority)

def placed(clip, offset, yaw, delay=0):
    """The clip's root carried to `offset` (x, z in the performer's frame) and turned by `yaw`; `delay` frames of its
    first pose first."""
    tr = {(t['bone'], t['channel']): t for t in clip['tracks']}
    rx, rz = tr[(-1, 0)]['values'], tr[(-1, 2)]['values']
    n = clip['frames']
    xs, zs = [], []
    for f in range(n):
        x, z = rot(yaw, rx[f], rz[f]); xs.append(round(offset[0] + x, 6)); zs.append(round(offset[1] + z, 6))
    tracks = [t for t in clip['tracks'] if not (t['bone'] == -1 and t['channel'] in (0, 2, 4))]
    tracks += [dict(bone=-1, channel=0, encoding=6, values=xs), dict(bone=-1, channel=2, encoding=6, values=zs),
               dict(bone=-1, channel=4, encoding=6, values=[yaw] * n)]
    if delay:
        tracks = [dict(t, values=[t['values'][0]] * delay + t['values']) for t in tracks]
        n += delay
    return dict(clip, frames=n, tracks=tracks)

def main():
    moves = json.loads((ROOT / 'assets/native/kreely-moves.json').read_text())
    index_path = CLIPS / 'index.json'
    index = json.loads(index_path.read_text(encoding='utf-8')) if index_path.exists() else dict(version=1, clips=[])
    keep = [c for c in index['clips'] if not c['name'].startswith(('kreely_gh_', 'rin_gh_', 'pl0030_kreely_gh_', 'em0440_gh_'))]
    entries = []
    for key, m in moves['moves'].items():
        clip = next(c for c in moves['clips'] if c['name'] == m['clip'])
        label, source = LABELS.get(key, (key, ''))
        # explorer names carry the family prefix its motion menu lists by (pl0030_ for Kreely, em0440_ for RinRin)
        entries.append(write('pl0030_' + m['clip'], clip, f'Kreely · {label} (God Hand {source}, retargeted)', f"godhand:{m['source']}"))
    rin = {c['name']: c for c in moves['rin_clips']}
    sx = moves['suplex']
    # the suplex pair: in mw-play the throw starts after the grab's reach, from where it ends; here both start at the
    # origin, RinRin placed as the grab places her
    entries.append(write('em0440_gh_suplex_recv_paired.mot', placed(rin['gh_suplex_recv.mot'], (sx['offset'][0], sx['offset'][2]), sx['yaw']),
                         'RinRin · suplexed, paired with Kreely (God Hand grab 0011c348, Gene thrown, retargeted)', 'godhand:grab_0011c348_suplex_gene'))
    # the sweep pair: she stands 1.25 m in front facing him (the explorer's sweep pair placement) and falls from the kick's
    # contact (Kreely's sweep hits at source frame 13)
    entries.append(write('em0440_gh_sweep_down_paired.mot', placed(rin['gh_sweep_down.mot'], (0., 1.25), math.pi, delay=13),
                         'RinRin · swept off her feet, paired with Kreely (God Hand Small Fry 375, retargeted)', 'godhand:smallfry_375_sweep_knockdown_reaction'))
    for name in ['gh_sweep_down.mot', 'gh_getup.mot', 'gh_suplex_recv.mot']:
        entries.append(write('em0440_' + name, rin[name], f'RinRin · {name[3:-4].replace("_", " ")} (God Hand, retargeted)', 'godhand'))
    index['clips'] = keep + entries
    index_path.write_text(json.dumps(index, indent=1), encoding='utf-8')
    # paired studies in app.js
    app = EXPLORER / 'app.js'
    src = app.read_text(encoding='utf-8')
    pairs = {
        'kreely-suplex': dict(bank='custom/clips', a=['custom/kreely', 'kreely.mdb'], b=['em/em0440.dat', 'em0440.mdb'], ca='pl0030_kreely_gh_suplex.mot',
                              cb='em0440_gh_suplex_recv_paired.mot', label='Kreely suplex / RinRin thrown (God Hand grab 0011c348, retargeted; added)', who=['Kreese Kreely', 'RinRin']),
        'kreely-sweep': dict(bank='custom/clips', a=['custom/kreely', 'kreely.mdb'], b=['em/em0440.dat', 'em0440.mdb'], ca='pl0030_kreely_gh_sweep.mot',
                             cb='em0440_gh_sweep_down_paired.mot', label='Kreely sweep / RinRin knocked down (God Hand 166 / 375, retargeted; added)', who=['Kreese Kreely', 'RinRin']),
    }
    for key, p in pairs.items():
        entry = f"'{key}':{json.dumps(p, separators=(',', ':'))}"
        if f"'{key}':" in src:
            src = re.sub(r"'" + re.escape(key) + r"':\{[^{}]*\}", lambda _: entry, src, count=1)
        else:
            src = src.replace("};let pairKey=", "," + entry + "};let pairKey=", 1)
    app.write_text(src, encoding='utf-8')
    print(f'published {len(entries)} clips to {CLIPS} and pairs {list(pairs)} to app.js')

if __name__ == '__main__':
    main()
