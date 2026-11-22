"""Stage source-only fan observations and RNG words for a bounded idle replay."""
import hashlib
import importlib.util
import json
from pathlib import Path
spec = importlib.util.spec_from_file_location('frame_compare', Path(__file__).with_name('frame_compare.py'))
fc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fc)


def main():
    path = fc.ROOT / 'evidence/frame-record/fight-target-v1.jsonl'
    _, headers, source = fc.load_record(path)
    fc.resolve(source, fc.motion_names(headers))
    rows = [r for r in source if 335 <= r['frame'] <= 1072]
    assert len(rows) == 738
    for r in rows:
        i = r['input']
        assert i['hold'] == i['trig'] == '00000000' and i['stick'] == [0, 0]
        assert abs(i['accX']) <= 2 and abs(i['accY']) <= 2
        assert i['f64'] == i['m1088'] == i['m1090'] == i['m109c'] == 0
    first = rows[0]
    checks = []
    for prev, r in zip(rows, rows[1:]):
        launch = r['rin']['state'][2] == 3 and prev['rin']['state'][2] == 2
        if 372 <= r['frame'] <= 405 or launch:
            checks.append({'frame': r['frame'], 'launch': launch,
                           'fans': r['contacts']['fans']})
    result = {'schema': 'mw-fan-transform-observed/1',
              'source': path.relative_to(fc.ROOT).as_posix(),
              'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
              'scope': 'Original idle input through1072. Initial actor state and per-update RNG words seed Rust; subsequent actor states, HP and fan states are never injected. Checks cover first throw geometry and later launch targets/progress.',
              'initial': {k: first[k] for k in ('frame', 'seed', 'demo', 'player', 'rin')},
              'steps': [{'frame': r['frame'], 'seed': r['seed'], 'demo': r['demo']} for r in rows[1:]],
              'checks': checks}
    output = fc.ROOT / 'research/fan-transform-observed.json'
    output.write_text(json.dumps(result, separators=(',', ':')) + '\n')
    print(json.dumps({'steps': len(result['steps']), 'checks': len(checks)}))


if __name__ == '__main__':
    main()
