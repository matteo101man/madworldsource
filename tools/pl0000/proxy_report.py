"""Summarize optional MW_FRAME_PROXY observations without changing recordings.

python tools/pl0000/proxy_report.py evidence/frame-record/proxy-clock.jsonl --out research/havok-proxy-clock-evidence.json
"""
import argparse
import hashlib
import json
from pathlib import Path


def report(path):
    raw = path.read_bytes()
    header, rows = None, []
    for line in raw.splitlines():
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if 'recording' in row:
            header = row
        elif 'frame' in row and 'proxy' in row:
            rows.append(row)
    low = [r for r in rows if r['proxy'].get('physicsStep', 1) <= 0.0001]
    floor_only = [r for r in low if r['proxy']['manifold'] and
                  all(c['normal'][1] > 0.99 for c in r['proxy']['manifold'])]
    return {
        'record': path.as_posix(), 'sha256': hashlib.sha256(raw).hexdigest(),
        'header': header, 'proxyRows': len(rows), 'solverTimeCutoff': 0.0001,
        'belowCutoffRows': len(low), 'belowCutoffFloorOnlyRows': len(floor_only),
        'floorOnlyZeroVelocityRows': sum(all(abs(v) < 1e-8 for v in r['proxy']['velocity']) for r in floor_only),
        'window': [{k: r[k] for k in ('frame', 'demo', 'player', 'proxy')}
                   for r in rows if low and low[0]['frame'] - 3 <= r['frame'] <= low[0]['frame'] + 3],
        'scope': 'Owned core observations. Random-state injection is a fixture, not snapshot-exact playback. '
                 'Earlier proxy-seeded/request/simplex captures used this fixture before the header flag was added. '
                 'No wall contact at a stop does not by itself establish a clock cause; physicsStep supplies that evidence.'
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('record', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    result = report(args.record)
    if args.out:
        args.out.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: v for k, v in result.items() if k not in ('window', 'header')}, indent=2))
