"""Check rendered speech for format, signal and obvious word errors; this does not grade acting."""
import argparse
import difflib
import hashlib
import json
from pathlib import Path
import re
import time

import numpy as np
import soundfile as sf
from faster_whisper import WhisperModel

from stage_kreely_voice import ROOT, read_lines

ALIASES = {
    'teeth_01': 'My teeth! Those were my good teeth!',
    'teeth_03': "That's... that's gonna cost ya, lady.",
    'teeth_06': 'Hey! I was using those!',
}


def words(text):
    text = text.lower().replace('’', "'")
    # Whisper alternates between contractions and their expanded forms.
    # Treat those as the same words when checking for missing content.
    for suffix, expanded in [("'ve", ' have'), ("'ll", ' will'), ("'re", ' are'), ("'m", ' am')]:
        text = text.replace(suffix, expanded)
    return re.findall(r'[a-z0-9]+', text.replace("'", ''))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', default='contextual-v4')
    parser.add_argument('--wait', action='store_true')
    args = parser.parse_args()
    folder = ROOT/'assets/native/audio/kreely/review'/args.revision
    rows = read_lines(ROOT/'research/KREELY-VOICE.md')
    report_path = folder/'validation.json'
    report = json.loads(report_path.read_text()) if report_path.exists() else {}
    model = WhisperModel(r'D:\VoiceStudio-runtime\models\asr\base.en', device='cpu', compute_type='int8', cpu_threads=4)
    for row in rows:
        key = row['key']
        path = folder/(key+'.wav')
        deadline = time.monotonic()+600
        while not path.exists():
            if not args.wait or time.monotonic() > deadline:
                raise FileNotFoundError(path)
            time.sleep(1)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if report.get(key, {}).get('sha256') == digest:
            cached = report[key]
            score = difflib.SequenceMatcher(None, words(cached['expected_text']), words(cached['transcript'])).ratio()
            cached['word_similarity'] = round(score, 3)
            cached['flags'] = [flag for flag in cached['flags'] if flag != 'wording']
            if score < .65:
                cached['flags'].append('wording')
            report_path.write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
            continue
        audio, sr = sf.read(path)
        info = sf.info(path)
        segments, _ = model.transcribe(str(path), language='en', beam_size=5, condition_on_previous_text=False)
        transcript = ' '.join(s.text.strip() for s in segments)
        expected = ALIASES.get(key, row['text'])
        score = difflib.SequenceMatcher(None, words(expected), words(transcript)).ratio()
        flags = []
        if sr != 44100 or info.channels != 1 or info.subtype != 'PCM_16':
            flags.append('format')
        if not np.isfinite(audio).all() or np.sqrt(np.mean(audio**2)) < .001:
            flags.append('signal')
        if score < .65:
            flags.append('wording')
        report[key] = {'sha256': digest, 'expected_text': expected, 'transcript': transcript,
            'word_similarity': round(score, 3), 'sample_rate': sr, 'channels': info.channels,
            'format': info.subtype, 'duration_s': len(audio)/sr, 'peak': float(np.max(np.abs(audio))), 'flags': flags}
        report_path.write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
        print(key, repr(transcript), flags, flush=True)
    print('Checked', len(rows), 'lines; flagged:', [r['key'] for r in rows if report[r['key']]['flags']], flush=True)


if __name__ == '__main__':
    main()
