"""Stage the checked, selected Kreely voice bank at the game's existing asset paths."""
import hashlib
import json
from pathlib import Path
import shutil

import numpy as np
import soundfile as sf

from stage_kreely_voice import ROOT, SETS, read_lines

PILOTS = ['defeat_04', 'knockdown_04', 'hurt_03', 'near_death_03', 'teeth_02']


def publish():
    direction = json.loads((ROOT/'research/KREELY-VOICE-DIRECTION.json').read_text(encoding='utf-8'))
    out = ROOT/'assets/native/audio/kreely'
    selected = out/'review'/direction['revision']
    takes = json.loads((selected/'review.json').read_text(encoding='utf-8'))
    validation = json.loads((selected/'validation.json').read_text(encoding='utf-8'))
    rows = read_lines(ROOT/'research/KREELY-VOICE.md')
    for row in rows:
        key = row['key']
        source = selected/(key+'.wav')
        digest = hashlib.sha256(source.read_bytes()).hexdigest()
        if takes['takes'][key]['sha256'] != digest or validation[key].get('sha256') != digest or validation[key]['flags']:
            raise ValueError('Take must pass current content/format checks: '+key)
    receipt_path = ROOT/'research/KREELY-VOICE-CLEANUP.json'
    receipt = json.loads(receipt_path.read_text(encoding='utf-8-sig'))
    receipt.setdefault('replaced_bank_files', [])
    report = {'selected_version': direction['revision'], 'engine': 'voxcpm2',
              'selection': 'User selected the five pilots and the generation setup; remaining lines use that setup.',
              'source': 'research/KREELY-VOICE.md', 'lines': {}}
    for row in rows:
        key = row['key']
        source, dest = selected/(key+'.wav'), out/(key+'.wav')
        if dest.exists() and hashlib.sha256(dest.read_bytes()).hexdigest() != takes['takes'][key]['sha256']:
            receipt['replaced_bank_files'].append({'file': str(dest), 'previous_sha256': hashlib.sha256(dest.read_bytes()).hexdigest()})
        temporary = dest.with_suffix('.part.wav')
        shutil.copyfile(source, temporary)
        temporary.replace(dest)
        take = takes['takes'][key]
        take['review_status'] = 'User selected pilot' if key in PILOTS else 'Generated using user-selected setup; wording/format checked'
        report['lines'][key] = {**row, **take, 'file': f'native/audio/kreely/{key}.wav',
                               'source_take': str(source), 'target_duration_s': None, 'synthesis_text': take['text']}
    manifest = {name: [{'file': f"native/audio/kreely/{r['key']}.wav", 'text': r['text']}
                      for r in rows if r['set'] == name] for name in SETS}
    for name, value in [('kreely.json', manifest), ('generation.json', report), ('validation.json', validation)]:
        temporary = out/(name+'.part')
        temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False)+'\n', encoding='utf-8')
        temporary.replace(out/name)
    preview = []
    for key in PILOTS:
        audio, sr = sf.read(out/(key+'.wav'))
        preview += [audio, np.zeros(int(sr*.7))]
    reel = np.concatenate(preview[:-1])
    sf.write(out/'preview-five-sets.wav', reel, 44100, subtype='PCM_16')
    sf.write(selected/'contextual-review.wav', reel, 44100, subtype='PCM_16')
    takes['selection_status'] = 'User-selected voice setup; full bank staged'
    (selected/'review.json').write_text(json.dumps(takes, indent=2)+'\n', encoding='utf-8')
    notes = ['# Kreely selected voice bank', '', 'Selected version: '+direction['revision'], '',
             'Five pilot performances were selected by the user. All 40 use that setup and pass wording/format checks.', '']
    for row in rows:
        key = row['key']
        notes += [f"- [{key}: {row['text']}]({(out/(key+'.wav')).as_posix()})"]
    (out/'index.md').write_text('\n'.join(notes)+'\n', encoding='utf-8')
    receipt['selected_bank_files'] = len(rows)
    receipt_path.write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8')
    print('Staged', len(rows), 'selected-version WAVs; older bank replaced.', flush=True)


if __name__ == '__main__':
    publish()
