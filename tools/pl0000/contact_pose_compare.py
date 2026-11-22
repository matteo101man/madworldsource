"""Compare source damage poses at current/pre-advance frames against native contacts.

Uses native actor positions/facings, so replay movement drift cannot mask pose errors.
This does not reconstruct blends, motion-start timing, part flags, or animation layers.
"""
import argparse
import importlib.util
import json
import os
import subprocess
from pathlib import Path

spec = importlib.util.spec_from_file_location('frame_compare', Path(__file__).with_name('frame_compare.py'))
compare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compare)
ROOT, load_record, motion_names, resolve = compare.ROOT, compare.load_record, compare.motion_names, compare.resolve


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('record', type=Path)
    parser.add_argument('--label', default='fight-contact-pose')
    args = parser.parse_args()
    _, headers, rows = load_record(args.record)
    resolve(rows, motion_names(headers))
    rows = [r for r in rows if 'contacts' in r]
    if not rows:
        raise SystemExit('record has no MW_FRAME_CONTACTS rows')
    resolved = ROOT / 'evidence/frame-record' / (args.label + '-resolved.jsonl')
    output = resolved.with_name(args.label + '-poses.jsonl')
    resolved.write_text(''.join(json.dumps(r) + '\n' for r in rows), encoding='utf-8')
    env = dict(os.environ)
    env.setdefault('CARGO_TARGET_DIR', 'D:/GDecomp/build-cache/bevy-0.20')
    subprocess.run(['cargo', 'run', '-q', '-p', 'mw-sim', '--example', 'contact_pose_compare', '--',
                    str(resolved), str(output)], cwd=ROOT, env=env, check=True)
    # Avoid motion-start/blend windows for this first timing check.
    stable = set()
    last = None
    age = 0
    for r in rows:
        clip = r['player']['clips'][0]
        age = age + 1 if clip == last else 0
        if age >= 30:
            stable.add(r['frame'])
        last = clip
    errors = {0: [], 1: []}
    transitions = {0: [], 2: []}
    for r in map(json.loads, output.read_text(encoding='utf-8').splitlines()):
        if r['frame'] in stable:
            for v in r['variants'][:2]:
                errors[v['delay']].extend(p['distance'] for p in v['parts'])
        if r['idleAge'] is not None and r['idleAge'] < 5:
            for v in r['variants'][::2]:
                transitions[v['delay']].extend(p['distance'] for p in v['parts'])
    report = {'record': str(args.record), 'stableFrames': len(stable), 'poseErrors': {
        str(k): {'count': len(v), 'mean': sum(v)/len(v), 'max': max(v)}
        for k, v in errors.items() if v},
        'getupToIdleErrors': {str(k): {'count':len(v), 'mean':sum(v)/len(v), 'max':max(v)}
                             for k, v in transitions.items() if v},
        'limits': 'Native positions/facings. Stable pose check excludes first 30 rows of each clip. Variant 2 filters only 1090-to-0000 with native idle blend 5. No layers/part flags reconstructed.'}
    dest = ROOT / 'research' / (args.label + '.json')
    dest.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
