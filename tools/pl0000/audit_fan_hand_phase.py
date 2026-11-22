"""Bounded read-only phase audit of recorded fan hand matrices.

Run frame_compare with MW_REPLAY_CONTACTS=1 first. The native file is the
Rust replay despite its historical filename. This audit measures transforms,
not gameplay equivalence, and never changes replay acceptance thresholds.
"""
import hashlib
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def rows(path):
    return {r['frame']: r for line in path.read_text().splitlines()
            if (r := json.loads(line)).get('contacts')}


def main():
    original = ROOT / 'evidence/frame-record/fight-target-v1.jsonl'
    replay = original.with_name('fight-target-v1.native.jsonl')
    source, rust = rows(original), rows(replay)
    samples = []
    for frame in range(372, 406):
        for fan in range(2):
            observed = source[frame]['contacts']['fans'][fan]
            matrix = observed['handMatrix']
            errors = []
            for shift in (0, -1):
                other = rust[frame + shift]['contacts']['fans'][fan]['handMatrix']
                errors.append(math.sqrt(sum((matrix[k] - other[k]) ** 2
                                            for k in range(12) if k % 4 != 3)))
            local = source[frame - 1]['contacts']['fans'][fan]['local']
            predicted = [sum(matrix[axis * 4 + k] * local[k] for k in range(3))
                         + matrix[axis * 4 + 3] for axis in range(3)]
            samples.append({'frame': frame, 'fan': fan,
                            'samePoseRotationError': errors[0],
                            'previousPoseRotationError': errors[1],
                            'nativeWorldPublicationError': math.dist(predicted, observed['world'])})
    result = {'record': original.relative_to(ROOT).as_posix(),
              'sourceSha256': hashlib.sha256(original.read_bytes()).hexdigest(),
              'replaySha256': hashlib.sha256(replay.read_bytes()).hexdigest(),
              'scope': 'First throw, source/Rust motion clocks aligned; rotation matrices only for cross-implementation phase comparison. Source world publication uses source previous local and source current hand matrix.',
              'sampleCount': len(samples),
              'maxima': {key: max(s[key] for s in samples) for key in (
                  'samePoseRotationError', 'previousPoseRotationError', 'nativeWorldPublicationError')},
              'samples': samples}
    output = ROOT / 'research/FAN-HAND-PHASE-AUDIT.json'
    output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'sampleCount': len(samples), 'maxima': result['maxima']}))


if __name__ == '__main__':
    main()
