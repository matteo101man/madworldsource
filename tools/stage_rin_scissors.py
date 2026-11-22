"""Stage RinRin's alternate clash win, the fan scissors closed on the neck, from the model explorer's added clips.

python -I tools/stage_rin_scissors.py
Inputs : <explorer>/custom/clips/em0440_0113sc.mot.json, pl0030_04adsc.mot.json (added animations the explorer
         authored from the original loss pair 0113 / 04ad: "Rin Power Struggle - ALT loss: fan scissors on neck")
Outputs: the clips appended to assets/native/rinrin-motion.json and assets/native/jack-player-motion.json (replacing an
         earlier copy of the same name). The port's last-life execution plays them (mw_sim Fight::rin_execute).
"""
import hashlib, json
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
CUSTOM = PORT.parents[1] / 'exports' / 'madworld' / 'custom' / 'clips'
INDEX = json.loads((CUSTOM / 'index.json').read_text(encoding='utf-8'))


def add(bank_path, name):
    src = CUSTOM / f'{name}.json'
    entry = next(c for c in INDEX['clips'] if c['name'] == name)
    data = src.read_bytes()
    assert hashlib.sha256(data).hexdigest() == entry['sha256'], f'{name} changed'
    clip = json.loads(data); clip['id'] = entry['id']; clip['name'] = name
    bank = json.loads(bank_path.read_text(encoding='utf-8'))
    bank['clips'] = [c for c in bank['clips'] if c['name'] != name] + [clip]
    bank_path.write_text(json.dumps(bank, separators=(',', ':')), encoding='utf-8')
    print(f'{bank_path.name}: + {name} ({clip["frames"]} frames, {len(clip["tracks"])} tracks)')


add(PORT / 'assets/native/rinrin-motion.json', 'em0440_0113sc.mot')
add(PORT / 'assets/native/jack-player-motion.json', 'pl0030_04adsc.mot')
