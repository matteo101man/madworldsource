"""Stage stage-201 announcer voice clips for the native slice (jkSound::PlayVoiceStream 8026B558).

python -I tools/audio/stage_announce.py
Native facts (research/AUDIO-API.md):
- PlayVoiceStream(id < 0x10000) opens announce/<lang>/an_c<room>_pack_<lang>.bin ('%s%s/an_c%03x_pack_%s.bin') and looks
  the id up (8026BB14) in an_c<room>_info_<lang>.bin (DAT member of case2/c201.dat): 'apd' magic, u16 count at +6,
  0x108-byte entries from +0, u16 id at +0x14, u8 at +0x16, then groups of 8 u32 clip byte offsets into the pack at
  +0x18 + 0x20 * group (0xFFFFFFFF = empty). Each clip is a RedSpark stream; per-id cooldown +0x1BB8 = 0x708 frames
  (0x1005: 18000, 0x1018: 0x1518, 0x43: 0x546).
- The variant / group choice (8026BCE0, 8026C008) is not translated: the presenter picks a random clip [inferred].
Output stays local (assets/ is gitignored).
"""
import json, struct, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FILES = Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files')
VGM = ROOT / 'tools/vgmstream/vgmstream-cli.exe'
import os
# MW_ROOM=303 stages Elise's room: her own lines 0x9C-0xAB (research/ELISE-BOSS.md) into assets/native/rooms/c303/announce
ROOM, LANG = int(os.environ.get('MW_ROOM', '201'), 16), 'eng'
OUT = ROOT / ('assets/native/audio/announce' if ROOM == 0x201 else f'assets/native/rooms/c{ROOM:03x}/announce')
# ids requested by the ported player code: idle fidget, giant swing spin / throw, death, low HP (plus the boss's own;
# c303 and c503 add 0x1027, the room-clear line of 80124258; an_c503_info_eng.bin has no 0x1027 entry, so c503's
# manifest lists it with no clips: the request finds nothing there and stays silent [inferred])
IDS = [0x1117, 0x100A, 0x100B, 0x1144, 0x1018] + (list(range(0x9C, 0xAD)) + [0x1027] if ROOM == 0x303 else []) \
    + (list(range(0x10E, 0x11F)) + [0x1143, 0x1145, 0x1146, 0x1147, 0x1022, 0x1027] if ROOM == 0x503 else [])   # the Baron, BARON-LEVEL.md 1.4
# RinRin's own lines (research/rinrin-voice-requests.json: 0x53-0x5B)
IDS += list(range(0x53, 0x5C)) if ROOM == 0x201 else []
# the kung-fu grunts' own lines (research: cEnemyBase::playVoice 810669B0): 0x1002 the flee, 0x1007 killed by an explosion
IDS += [0x1002, 0x1007] if ROOM == 0x201 else []


def dat_member(path, want):
    b = path.read_bytes()
    n, ot, _et, nt, st = struct.unpack_from('>IIIII', b, 4)
    ln = struct.unpack_from('>I', b, nt)[0]
    for i in range(n):
        name = b[nt + 4 + i * ln:nt + 4 + (i + 1) * ln].split(b'\0')[0].decode()
        if name == want:
            o = struct.unpack_from('>I', b, ot + 4 * i)[0]; s = struct.unpack_from('>I', b, st + 4 * i)[0]
            return b[o:o + s]
    raise KeyError(want)


def main():
    info = dat_member(FILES / f'case{ROOM >> 8}/c{ROOM:03x}.dat', f'an_c{ROOM:03x}_info_{LANG}.bin')
    assert info[:3] == b'apd'
    pack = (FILES / f'announce/{LANG}/an_c{ROOM:03x}_pack_{LANG}.bin').read_bytes()
    count = struct.unpack_from('>H', info, 6)[0]
    entries, offsets = {}, set()
    for k in range(count):
        e = k * 0x108
        vid = struct.unpack_from('>H', info, e + 0x14)[0]
        groups = [[o for o in struct.unpack_from('>8I', info, e + 0x18 + 0x20 * g) if o != 0xFFFFFFFF] for g in range(8)]
        entries[vid] = (info[e + 0x16], groups)
        offsets.update(o for g in groups for o in g)
    ends = sorted(offsets) + [len(pack)]
    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for vid in IDS:
        clips = []
        mask, groups = entries.get(vid, (0, []))
        for g, group in enumerate(groups):
            for o in group:
                end = next(x for x in ends if x > o)
                rsd = OUT / f'{vid:04x}-{o:08x}.rsd'; wav = rsd.with_suffix('.wav')
                if not wav.exists():
                    rsd.write_bytes(pack[o:end])
                    subprocess.run([str(VGM), '-i', '-o', str(wav), str(rsd)], capture_output=True)
                    rsd.unlink()
                if wav.exists(): clips.append(dict(group=g, file=wav.relative_to(ROOT / 'assets').as_posix()))
        manifest[str(vid)] = dict(mask=mask, cooldown={0x1005: 18000, 0x1018: 0x1518, 0x43: 0x546}.get(vid, 0x708), clips=clips)
    (OUT / 'announce.json').write_text(json.dumps(manifest, indent=1), encoding='utf-8')
    print({hex(int(k)): len(v['clips']) for k, v in manifest.items()})


if __name__ == '__main__':
    main()
