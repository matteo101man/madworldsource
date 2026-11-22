"""Merge the shared grunt motions into the kung-fu bank (assets/native/kungfu-motion.json from tools/stage_actor.py).
em/em0000.dat holds the cEmZako family's shared em0110_* motions; their tracks use bones 0-24/100/101, which em0101 has,
so they play on it unretargeted. Merged: damage / down / get-up / death families 10xx-11xx (names kept; meanings are
read from the numbering, as for the player and RinRin banks), plus the AI motions in AI_CLIPS. Idempotent: clips already in the bank are skipped.
"""
import json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
EXPLORER = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPLORER / 'builders'))
from native import dat_entries, motion  # noqa: E402

# the native grunt AI's own motions outside those families (research/ZAKO-AI.md): spotted, threat waits, run, walk back,
# turn, run strafes, the dash attack, laughs
AI_CLIPS = {'em0110_0036', 'em0110_0042', 'em0110_0043', 'em0110_0044', 'em0110_0012', 'em0110_0011', 'em0110_0013', 'em0110_002b',
            'em0110_002c', 'em0110_010d', 'em0110_0045', 'em0110_0046', 'em0110_004c', 'em0110_004d', 'em0110_004e',
            # fear_to_escape: the startle, the run away, the stop
            'em0110_0016', 'em0110_0017', 'em0110_0018', 'em0110_0015', 'em0110_1512', 'em0110_0030'}
bank_path = ROOT / 'assets/native/kungfu-motion.json'
bank = json.loads(bank_path.read_text())
have = {c['name'] for c in bank['clips']}
raw = (EXPLORER / 'extracted/disc/DATA/files/em/em0000.dat').read_bytes()
added = []
for e in dat_entries(raw):
    n = e['name']
    if not (n.startswith('em0110_10') or n.startswith('em0110_11') or n[:-4] in AI_CLIPS) or not n.endswith('.mot') or n in have or not e['size']: continue
    clip = motion(raw[e['offset']:e['offset'] + e['size']])
    clip.update(id=f"em/em0000.dat#{n}@{e['offset']:x}", name=n)
    bank['clips'].append(clip); added.append((n, clip['frames']))
bank_path.write_text(json.dumps(bank, separators=(',', ':')))
print(len(added), 'merged:', added)
