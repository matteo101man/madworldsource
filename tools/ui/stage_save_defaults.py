"""Derive the native empty record from 8017FD14 and its original reset table.

Never reads or stages a player's save. Optional --verify-save compares empty
records read-only, without using them as the source of generated assets.
"""
from pathlib import Path
import argparse, hashlib, json, struct, sys
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/pl0000'))
from dol import Dol, DOL

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--verify-save', type=Path)
    args = parser.parse_args()
    dol = Dol()
    words = [dol.u32(0x8054F3F0 + i * 4) for i in range(15)]
    record = bytearray(0xC450)
    struct.pack_into('>I', record, 4, 3)
    struct.pack_into('>H', record, 0xC, 0x100)
    struct.pack_into('>H', record, 0x3D0, 1)
    for i, word in enumerate(words):
        struct.pack_into('>I', record, 0x3D8 + i * 8, word)
    folder = ROOT / 'assets/native/save'
    folder.mkdir(parents=True, exist_ok=True)
    (folder / 'reset-record.bin').write_bytes(record)
    data = dict(schema='mw-native-save-reset/1', source=str(DOL),
                sourceSha256=hashlib.sha256(dol.b).hexdigest(),
                resetFunction='8017FD14', entrySetter='80189B24',
                tableGetter='80189CBC', tableAddress='8054F3F0',
                tableWords=words, recordBytes=len(record),
                sha256=hashlib.sha256(record).hexdigest(),
                scope='Exact empty record reset; does not define new-game setup or companion options serialization.')
    (folder / 'reset-record.json').write_text(json.dumps(data, indent=2)+'\n', encoding='utf-8')
    if args.verify_save:
        raw = args.verify_save.read_bytes()
        if len(raw) != 4 * len(record):
            raise ValueError('Native save size differs from four records')
        matches = [i for i in range(4) if raw[i*len(record):(i+1)*len(record)] == record]
        if not matches:
            raise ValueError('No independently recorded empty slot matches reset')
        print(json.dumps(dict(exactResetMatches=matches, sha256=data['sha256'])))
    else:
        print(json.dumps(dict(recordBytes=len(record), sha256=data['sha256'])))

if __name__ == '__main__':
    main()
