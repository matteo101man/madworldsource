"""Render the authored Kreely bank through the user's approved local VoiceStudio profile.

Run with D:\\VoiceStudio\\.venv\\Scripts\\python.exe tools/audio/stage_kreely_voice.py
VoiceStudio must be running at http://127.0.0.1:3900. No external TTS service is used.
"""
import argparse
import hashlib
import io
import json
import math
from pathlib import Path
import re
import time

import numpy as np
import requests
import soundfile as sf
from scipy.signal import resample_poly

ROOT = Path(__file__).resolve().parents[2]
SETS = {'defeat': 10, 'knockdown': 8, 'hurt': 8, 'near_death': 8, 'teeth': 6}
TAGS = {'laughs': '[laughter]', 'groans': '[sigh]', 'grunts': '[dissatisfaction-hnn]'}
PRONUNCIATION_OVERRIDES = {
    'knockdown_03': ('Tim-ber!', 'Explicit syllable break for the isolated Timber bark.'),
    'teeth_06': ('Hey! I was using those!', 'Authored phonetic lisp produced garbled speech; use intended words because this engine has no lisp control.'),
}


def read_lines(path):
    current = None
    rows = []
    for line in path.read_text(encoding='utf-8').splitlines():
        header = re.match(r'^### `([^`]+)`', line)
        if header:
            current = header[1]
            continue
        entry = re.match(r'^(\d+)\. (.+)$', line)
        if not entry or current not in SETS:
            continue
        authored = entry[2]
        notes = re.findall(r'\[([^\]]+)\]', authored)
        spoken = re.sub(r'\[[^\]]+\]\s*', '', authored).strip()
        def cue(match):
            return ' '.join(TAGS[c.strip()] for c in match[1].split(',') if c.strip() in TAGS)
        synthesis = re.sub(r'\[([^\]]+)\]', cue, authored)
        synthesis = re.sub(r' +', ' ', synthesis).strip()
        unmapped = [c.strip() for note in notes for c in note.split(',') if c.strip() not in TAGS]
        words = len(re.findall(r"[\w]+(?:['’][\w]+)*", spoken))
        # Match the approved 9-word / 5.5-second sample's pace, not its duration.
        duration = max(1.2, min(12., words * 5.5 / 9.))
        duration += .55 * synthesis.count('[laughter]')
        duration += .35 * synthesis.count('[sigh]')
        duration += .25 * synthesis.count('[dissatisfaction-hnn]')
        key = f'{current}_{int(entry[1]):02}'
        pronunciation_note = None
        if words == 1:
            synthesis = synthesis.upper()
        if key in PRONUNCIATION_OVERRIDES:
            synthesis, pronunciation_note = PRONUNCIATION_OVERRIDES[key]
        rows.append({'set': current, 'number': int(entry[1]), 'key': key,
                     'authored': authored, 'text': spoken, 'synthesis_text': synthesis,
                     'delivery_notes': notes, 'unmapped_cues': unmapped, 'target_duration_s': round(duration,2),
                     'pronunciation_note': pronunciation_note})
    for name, count in SETS.items():
        actual = [r['number'] for r in rows if r['set'] == name]
        if actual != list(range(1,count+1)):
            raise ValueError(f'{name}: expected consecutive lines 1–{count}, got {actual}')
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', type=Path, default=Path(r'D:\VoiceStudio-runtime\kreely\voice-profile.json'))
    parser.add_argument('--base-url', default='http://127.0.0.1:3900')
    parser.add_argument('--only', nargs='*', help='Regenerate or inspect specific keys, e.g. hurt_02')
    parser.add_argument('--force', action='store_true')
    parser.add_argument('--plan-only', action='store_true')
    args = parser.parse_args()
    source = ROOT/'research/KREELY-VOICE.md'
    rows = read_lines(source)
    profile = json.loads(args.profile.read_text(encoding='utf-8'))
    if profile.get('engine') == 'voxcpm2':
        import subprocess
        import sys
        selected = [r for r in rows if not args.only or r['key'] in args.only]
        if args.only and set(args.only) != {r['key'] for r in selected}:
            raise ValueError('Unknown line key in --only')
        print(json.dumps({'version': profile['approved_version'], 'lines': len(rows), 'selected': len(selected)}), flush=True)
        if args.plan_only:
            return
        command = [sys.executable, str(Path(__file__).with_name('review_kreely_context.py')), '--all', '--base-url', args.base_url]
        if args.only:
            command += ['--only', *args.only]
        if args.force:
            command += ['--force']
        subprocess.run(command, check=True)
        subprocess.run([sys.executable, str(Path(__file__).with_name('validate_kreely_voice.py'))], check=True)
        from publish_kreely_voice import publish
        publish()
        return
    if profile.get('studio_profile_id') != 'c84a6501':
        raise ValueError('Expected the user-approved Kreely profile c84a6501')
    selected = [r for r in rows if not args.only or r['key'] in args.only]
    if args.only and set(args.only) != {r['key'] for r in selected}:
        raise ValueError('Unknown line key in --only')
    print(json.dumps({'sets': SETS, 'lines': len(rows), 'selected': len(selected),
                      'unsupported_cues': sorted({c for r in rows for c in r['unmapped_cues']})}), flush=True)
    if args.plan_only:
        return
    out = ROOT/'assets/native/audio/kreely'
    out.mkdir(parents=True, exist_ok=True)
    report_path = out/'generation.json'
    report = json.loads(report_path.read_text(encoding='utf-8')) if report_path.exists() else {'lines': {}}
    report.update({'source': 'research/KREELY-VOICE.md', 'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
                   'profile_id': profile['studio_profile_id'], 'profile_name': profile['name'],
                   'model_source': profile['model_source'], 'reference_audio': profile['reference_audio'],
                   'reference_sha256': hashlib.sha256(Path(profile['reference_audio']).read_bytes()).hexdigest(),
                   'settings': {'engine': 'omnivoice', 'seed': 21751, 'speed': 1.0, 'num_step': 32,
                                'guidance_scale': 2.0, 'effect_preset': 'broadcast', 'denoise': False},
                   'cue_policy': {'mapped': TAGS, 'unsupported': 'Preserved as metadata, omitted from spoken synthesis.'}})
    session = requests.Session()
    for row in selected:
        dest = out/(row['key']+'.wav')
        identity = {'synthesis_text': row['synthesis_text'], 'target_duration_s': row['target_duration_s']}
        fingerprint = hashlib.sha256(json.dumps([identity, report['settings'], report['reference_sha256']], sort_keys=True).encode()).hexdigest()
        prior = report['lines'].get(row['key'], {})
        if not args.force and dest.exists() and prior.get('fingerprint') == fingerprint and prior.get('sha256') == hashlib.sha256(dest.read_bytes()).hexdigest():
            print('Reusing', row['key'], flush=True)
            continue
        payload = {'text': row['synthesis_text'], 'profile_id': profile['studio_profile_id'], 'engine': 'omnivoice',
                   'language': 'English', 'seed': '21751', 'instruct': profile['direction'], 'num_step': '32',
                   'guidance_scale': '2.0', 'speed': '1.0', 'duration': str(row['target_duration_s']),
                   'denoise': 'false', 'effect_preset': 'broadcast', 'wav_bits': '16'}
        started = time.monotonic()
        response = session.post(args.base_url+'/generate', data=payload, timeout=600)
        if not response.ok:
            raise RuntimeError(f"{row['key']}: HTTP {response.status_code}: {response.text[:1800]}")
        audio, sr = sf.read(io.BytesIO(response.content), dtype='float32')
        if audio.ndim > 1:
            audio = audio.mean(axis=1)
        divisor = math.gcd(sr,44100)
        audio = resample_poly(audio,44100//divisor,sr//divisor)
        if not np.isfinite(audio).all() or len(audio) < 4410 or np.sqrt(np.mean(audio**2)) < .001:
            raise ValueError(f"{row['key']}: unusable audio")
        if np.max(np.abs(audio)) > .98:
            audio *= .98 / np.max(np.abs(audio))
        temporary = dest.with_suffix('.part.wav')
        sf.write(temporary, audio, 44100, subtype='PCM_16')
        temporary.replace(dest)
        report['lines'][row['key']] = {**row, 'file': f"native/audio/kreely/{dest.name}",
             'duration_s': round(len(audio)/44100,3), 'sample_rate': 44100, 'channels': 1, 'format': 'PCM_16',
             'sha256': hashlib.sha256(dest.read_bytes()).hexdigest(), 'fingerprint': fingerprint,
             'studio_history_id': response.headers.get('X-Audio-Id'), 'render_seconds': round(time.monotonic()-started,2)}
        report_path.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
        print(row['key'], f"{len(audio)/44100:.2f}s", row['text'], flush=True)
    missing = [r['key'] for r in rows if not (out/(r['key']+'.wav')).is_file()]
    if missing:
        print('Not yet generated:', ', '.join(missing), flush=True)
        return
    manifest = {name: [{'file': f"native/audio/kreely/{r['key']}.wav", 'text': r['text']}
                       for r in rows if r['set']==name] for name in SETS}
    temporary = out/'kreely.json.part'
    temporary.write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
    temporary.replace(out/'kreely.json')
    print('Complete:',len(rows),'WAV files and',out/'kreely.json',flush=True)


if __name__ == '__main__':
    main()
