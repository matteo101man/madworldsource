"""Replay both owned fight recordings and fail if the repaired HP baseline regresses.

Uses recorded per-frame RNG seeds and the Rust collision path. This does not
reconstruct native hidden state or claim snapshot-exact replay.
"""
import importlib.util
import json
import os
from pathlib import Path

spec = importlib.util.spec_from_file_location('frame_compare', Path(__file__).with_name('frame_compare.py'))
fc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fc)


def main():
    os.environ.update(MW_REPLAY_COLLISION='1', MW_REPLAY_RESEED='1')
    reports = []
    failures = []
    cases = [
        ('live', 334, 2533, {'player.hp': 1, 'player.motion': 2, 'player.motionFrame': 13,
                             'player.position': 8, 'player.state': 2}),
        ('fight-contacts', 384, 1756, {'player.hp': 0}),
    ]
    for name, start, expected_frames, limits in cases:
        record = fc.ROOT / 'evidence/frame-record' / (name + '.jsonl')
        _, headers, rows = fc.load_record(record)
        fc.resolve(rows, fc.motion_names(headers))
        rows = [r for r in rows if r['frame'] >= start]
        out = record.with_name(name + '.regression.native.jsonl')
        native = fc.native_rows(record, out, start, tuple(rows[0][w]['clips'][0] for w in ('player', 'rin')))
        comparison = fc.compare(rows, native, 'original', 'rust')
        hp = sum(v for k, v in comparison['framesDivergingByCategory'].items() if k.endswith('.hp'))
        counts = comparison['framesDivergingByCategory']
        checks = {'completeRecording': comparison['framesCompared'] == expected_frames,
                  'hpBaseline': hp <= limits['player.hp'],
                  'otherCategories': all(v <= limits.get(k, 0) for k, v in counts.items())}
        report = {'record': name, 'start': start, 'framesCompared': comparison['framesCompared'],
                  'hpMismatchFrames': hp, 'maxHpMismatchFrames': limits['player.hp'],
                  'maxMismatchesByCategory': limits,
                  'firstByCategory': comparison['firstByCategory'],
                  'framesDivergingByCategory': comparison['framesDivergingByCategory']}
        if name == 'live':
            source = next(r for r in rows if r['frame'] == 398)
            replay = next(r for r in native if r['frame'] == 398)
            error = abs(source['player']['pos'][2] - replay['player']['pos'][2])
            report['frame398GateZError'] = error
            checks['airborneGateStop'] = error < 0.00005
        report['checks'] = checks
        reports.append(report)
        failures.extend(name + ': ' + k for k, passed in checks.items() if not passed)
    result = {'passed': not failures, 'recordings': reports, 'failures': failures,
              'scope': 'Owned source recordings, collision enabled, recorded RNG reseeding; remaining HP/contact and timing discrepancies are allowed only up to the documented repaired baseline.'}
    path = fc.ROOT / 'research/FIGHT-REGRESSION.json'
    path.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, indent=2))
    return int(bool(failures))


if __name__ == '__main__':
    raise SystemExit(main())
