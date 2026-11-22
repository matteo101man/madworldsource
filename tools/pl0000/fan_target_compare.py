"""Check native target-world timing in an MW_FRAME_CONTACTS target capture."""
import argparse
import hashlib
import json
import math
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('record', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    rows = [r for line in args.record.read_text(encoding='utf-8').splitlines()
            if 'frame' in (r := json.loads(line))]
    errors = []
    moving = 0
    throws = []
    previous = None
    for row, following in zip(rows, rows[1:]):
        world = row['contacts']['playerWorld']
        if following['frame'] != row['frame'] + 1:
            raise ValueError('capture has a frame gap')
        errors.append(math.dist(world, following['player']['pos']))
        moving += math.dist(world, row['player']['pos']) > 1e-5
        fan = row['contacts']['fans'][0]
        if fan['phase'] == 2 and previous == 0:
            throws.append({'frame': row['frame'], 'target': fan['target'], 'dt': fan['dt']})
        previous = fan['phase']
    if not errors or not moving or not throws:
        raise ValueError('capture needs consecutive rows, root movement and fan throws')
    result = {'record': str(args.record), 'sha256': hashlib.sha256(args.record.read_bytes()).hexdigest(),
              'rows': len(rows), 'consecutiveWorldSamples': len(errors), 'movingSamples': moving,
              'maxWorldToNextActorError': max(errors), 'throws': throws,
              'passed': max(errors) < 1e-5,
              'scope': 'Read-only native world matrices compared with cached actor-update positions. 810658C8/8020FDEC select world translation for throws. This verifies transform phase, not full collision/AI or fan geometry parity.'}
    args.output.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: v for k, v in result.items() if k != 'throws'}, indent=2))
    return int(not result['passed'])


if __name__ == '__main__':
    raise SystemExit(main())
