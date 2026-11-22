"""Same-input, frame-by-frame comparison: recompiled-core recording vs the native Rust fight sim.

python -I tools/pl0000/frame_compare.py evidence/frame-record/<label>.jsonl [--against <other recording>] [--label NAME]

The recording comes from the recompiled core with MW_FRAME_RECORD (runtime/game/frame_record.cpp): one frame-atomic
row per game frame with player/Rin state bytes +0x11A..+0x11D, motion resource + frame, HP, position, the player's
pad view (+0xEE0/+0xEE4/+0xEF0/+0xEF4/+0xF1C/+0xF20/+0x113C/+0xF64/+0x1088/+0x108C/+0x1090/+0x109C), every
KPADReadEx sample, and the shared cRand word 0x80609B90.
Default: replays the recording into mw_sim::fight::Fight (cargo example frame_replay, seeded with the recorded cRand
word) and reports the first frame where state, motion, HP or position diverge.
--against: compares two recordings of the original (e.g. a live run and its MW_INPUT_REPLAY rerun from the same
snapshot) with the same rules, to show what the injection reproduces.
Writes research/FRAME-COMPARE-<label>.json and prints the summary used in research/FRAME-COMPARE.md.
"""
import argparse, collections, hashlib, json, math, os, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POS_TOL = 0.05     # world units (xz), per frame
Y_TOL = 0.05
FRAME_TOL = 1.0    # motion frames


def load_record(path):
    head, headers, rows = None, {}, []
    for line in open(path, encoding='utf-8'):
        line = line.strip()
        if not line: continue
        try: v = json.loads(line)
        except ValueError: continue   # a torn last line from a killed process
        if 'recording' in v: head = v
        elif 'motionHeader' in v: headers[v['motionHeader']] = bytes.fromhex(v['bytes'])
        elif 'frame' in v: rows.append(v)
    return head, headers, rows


def motion_names(headers):
    """Pointer -> candidate clip names (header SHA-256 against the owned DAT catalog, as read_native_core.py)."""
    index = json.loads((ROOT / 'research/native-motion-live-index.json').read_text(encoding='utf-8'))['headers']
    out = {}
    for ptr, data in headers.items():
        names = sorted({r['name'].removesuffix('.mot') for r in index.get(hashlib.sha256(data).hexdigest(), [])})
        out[ptr] = names
    return out


def resolve(rows, names):
    for r in rows:
        for who, prefix in (('player', 'pl0030'), ('rin', 'em0440')):
            a = r[who]
            cands = [n for n in names.get(a['motion'], []) if n.startswith(prefix)] or names.get(a['motion'], [])
            a['clips'] = cands or ['?' + a['motion']]


def native_rows(record, out, start=0, clips=()):
    env = dict(os.environ)
    env.setdefault('CARGO_TARGET_DIR', 'D:/GDecomp/build-cache/bevy-0.20')
    extra = []
    if start:
        _, headers, source = load_record(record)
        first = next(r for r in source if r['frame'] >= start)
        for who in ('player', 'rin'):
            header = headers[first[who]['motion']]
            extra += [f'--{who}-looping', str(int.from_bytes(header[4:6], 'big') & 1)]
    subprocess.run(['cargo', 'run', '-q', '-p', 'mw-sim', '--example', 'frame_replay', '--', str(record), str(out)] + (['--start', str(start), '--player-clip', clips[0], '--rin-clip', clips[1]] if start else [])
                   + extra + (['--contacts'] if os.environ.get('MW_REPLAY_CONTACTS') else [])
                   + (['--collision'] if os.environ.get('MW_REPLAY_COLLISION') else [])
                   + (['--reseed'] if os.environ.get('MW_REPLAY_RESEED') else []),
                   cwd=ROOT, env=env, check=True)
    rows = [json.loads(l) for l in open(out, encoding='utf-8') if l.strip()]
    for r in rows:
        for who in ('player', 'rin'):
            a = r[who]
            a['clips'] = [a['clip'].removesuffix('.mot')]
            a['state'] = [None, a['state'], a.get('phase'), None]
    return rows


def view(a):
    return {'state': a['state'][1], 'clip': '|'.join(a['clips']), 'frame': round(a['frame'], 3), 'hp': a['hp'],
            'pos': [round(x, 3) for x in a['pos']]}


def diverges(a, b):
    """Categories in which actor views a (reference) and b differ this frame."""
    out = []
    if a['state'][1] != b['state'][1]: out.append('state')
    if not set(a['clips']) & set(b['clips']): out.append('motion')
    elif abs(a['frame'] - b['frame']) > FRAME_TOL: out.append('motionFrame')
    if a['hp'] != b['hp']: out.append('hp')
    dx = math.hypot(a['pos'][0] - b['pos'][0], a['pos'][2] - b['pos'][2])
    if dx > POS_TOL or abs(a['pos'][1] - b['pos'][1]) > Y_TOL: out.append('position')
    return out


def lcg_steps(a, b, limit=20000):
    s = a
    for n in range(limit + 1):
        if s == b: return n
        s = (s * 1664525 + 1013904223) & 0xFFFFFFFF
    return None


def compare(ref, other, ref_name, other_name):
    by_frame = {r['frame']: r for r in other}
    first = {}          # (actor, category) -> frame
    first_any = None
    counts = collections.Counter()
    frames = 0
    for r in ref:
        o = by_frame.get(r['frame'])
        if o is None: continue
        frames += 1
        for who in ('player', 'rin'):
            for cat in diverges(r[who], o[who]):
                counts[(who, cat)] += 1
                first.setdefault((who, cat), r['frame'])
                if first_any is None: first_any = r['frame']
    window = []
    if first_any is not None:
        for f in range(first_any - 3, first_any + 6):
            if f in by_frame and any(x['frame'] == f for x in ref):
                r = next(x for x in ref if x['frame'] == f)
                window.append({'frame': f, ref_name: {w: view(r[w]) for w in ('player', 'rin')},
                               other_name: {w: view(by_frame[f][w]) for w in ('player', 'rin')},
                               'input': r.get('input'), 'nativeEvents': by_frame[f].get('events')})
    return {'framesCompared': frames, 'firstDivergence': first_any,
            'firstByCategory': {f'{w}.{c}': f for (w, c), f in sorted(first.items(), key=lambda x: x[1])},
            'framesDivergingByCategory': {f'{w}.{c}': n for (w, c), n in sorted(counts.items())},
            'window': window}


def summary(rows):
    seq = collections.Counter()
    draws = collections.Counter()
    for a, b in zip(rows, rows[1:]):
        n = lcg_steps(int(a['seed'], 16), int(b['seed'], 16))
        draws[n if n is not None else 'unreached'] += 1
    for who in ('player', 'rin'):
        last = None
        for r in rows:
            k = (r[who]['state'][1], r[who]['clips'][0])
            if k != last: seq[who] += 1; last = k
    kpad = collections.Counter(len(r.get('kpad', [])) for r in rows)
    stale = sum(1 for r in rows if not r['rin']['fresh'])
    states = {w: sorted({r[w]['state'][1] for r in rows}) for w in ('player', 'rin')}
    hp = {w: [rows[0][w]['hp'], rows[-1][w]['hp']] for w in ('player', 'rin')}
    return {'frames': len(rows), 'firstFrame': rows[0]['frame'], 'lastFrame': rows[-1]['frame'],
            'cRandDrawsPerFrame': {str(k): v for k, v in sorted(draws.items(), key=lambda x: str(x[0]))},
            'kpadReadsPerFrame': dict(sorted(kpad.items())), 'rinStaleRows': stale,
            'stateMotionSegments': dict(seq), 'states': states, 'hpFirstLast': hp}


def main():
    p = argparse.ArgumentParser()
    p.add_argument('record', type=Path)
    p.add_argument('--against', type=Path, help='a second recording of the original instead of the native sim')
    p.add_argument('--label')
    p.add_argument('--start', type=int, default=0, help='native replay begins at this recorded frame (state seeded from it)')
    a = p.parse_args()
    head, headers, rows = load_record(a.record)
    names = motion_names(headers)
    resolve(rows, names)
    label = a.label or a.record.stem
    result = {'record': a.record.resolve().relative_to(ROOT).as_posix() if a.record.resolve().is_relative_to(ROOT) else str(a.record),
              'header': head, 'original': summary(rows),
              'unresolvedMotions': sorted(p for p, n in names.items() if not n),
              'tolerances': {'positionXZ': POS_TOL, 'positionY': Y_TOL, 'motionFrame': FRAME_TOL}}
    if a.against:
        h2, hd2, rows2 = load_record(a.against)
        names.update(motion_names(hd2)); resolve(rows2, names)
        base = (h2 or {}).get('replayBase', 0)   # replay frame 0 is recorded frame `base`
        for r in rows2: r['frame'] += base
        result['replayBase'] = base
        result['against'] = str(a.against); result['againstSummary'] = summary(rows2)
        result['compare'] = compare(rows, rows2, 'original', 'replay')
        result['inputIdentical'] = sum(1 for x, y in zip(rows, rows2) if x.get('kpad') == y.get('kpad') and x['frame'] == y['frame'])
    else:
        out = a.record.with_name(a.record.stem + '.native.jsonl')
        s = next(r for r in rows if r['frame'] >= a.start)
        nat = native_rows(a.record, out, a.start, (s['player']['clips'][0], s['rin']['clips'][0]))
        rows = [r for r in rows if r['frame'] >= a.start]
        result['nativeStart'] = {'frame': s['frame'], 'seed': s['seed'], 'player': view(s['player']), 'rin': view(s['rin'])}
        result['native'] = str(out)
        result['compare'] = compare(rows, nat, 'original', 'native')
        nseq = collections.Counter()
        for who in ('player', 'rin'):
            last = None
            for r in nat:
                k = (r[who]['state'][1], r[who]['clips'][0])
                if k != last: nseq[who] += 1; last = k
        result['nativeSummary'] = {'frames': len(nat), 'stateMotionSegments': dict(nseq),
                                   'states': {w: sorted({r[w]['state'][1] for r in nat}) for w in ('player', 'rin')},
                                   'hpFirstLast': {w: [nat[0][w]['hp'], nat[-1][w]['hp']] for w in ('player', 'rin')}}
    (ROOT / 'research' / f'FRAME-COMPARE-{label}.json').write_text(json.dumps(result, indent=1) + '\n', encoding='utf-8')
    c = result['compare']
    print(json.dumps({'firstDivergence': c['firstDivergence'], 'firstByCategory': c['firstByCategory'],
                      'framesCompared': c['framesCompared'], 'window': c['window'][:6]}, indent=1))


if __name__ == '__main__':
    sys.exit(main())
