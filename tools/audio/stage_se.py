"""Stage MadWorld sound effects for the native slice: cue sound id -> wave files.

python -I tools/audio/stage_se.py
Native facts (see research/AUDIO-API.md):
- Sound ids are decimal: block = id / 1000, sound = id % 1000 (jkSound 80268E18). Motion cues `_10.seq` carry them as
  (block, sound) s16 pairs (op 29 SE, op 42 voice, op 38/37 footstep families).
- se/scene_mixNNNN.rsp is a PACK: member 0 = PACK of RedSpark wave banks (one per block, header +0x14 = block),
  other members = RedSpark sound-definition banks (header +0x14 = block). RedSpark headers are XOR-encrypted
  (vgmstream redspark.c key schedule, tools/audio/redspark.py); the data after the header is plain.
- Definition bank: +0x30 entry count, +0x40 u32 offset table (0xFFFFFFFF = unused).
  804F835C resolves programs at bank+0x40+4*count+(offset&0x7FFFFFFF).
  Program: u8 track count, u8 volume, ..., at +0x10 u16 block, then per-track u16 byte lengths, then the track
  command lists.
Native handlers: 0x20=wave,0x22=volume,0x12=key-on(note),0x15=u16 wait,
0x60=effect-send flags. Other track commands remain a partial inspection parser.
- Wave bank: u16 wave count at +0x3E, u32 wave record offsets from +0x50; each record's s8.24 tune (semitones) at +0x1C
  and root key at +0x22. A wave's rate is 32000 * 2^(tune / 12) (the DSP's 32 kHz output: tunes -12, -6.448, -6.487,
  -14.312 give 16000, 22050, 22000, 14000 Hz); vgmstream labels every bank 24000 Hz, so each staged wave's header is
  rewritten to its own rate, and a play's key-on note sounds 2^((note - root) / 12) times that (play.rs).
"""
import json, struct, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SE = Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files/se')
VGM = ROOT / 'tools/vgmstream/vgmstream-cli.exe'
OUT = ROOT / 'assets/native/audio/se'
sys.path.insert(0, str(ROOT / 'tools/audio'))
from redspark import decrypt, pack_members

PACKS = ['scene_mix0000.rsp', 'scene_mix0201.rsp', 'scene_mix0221.rsp', 'event/scene_mix7522.rsp',
         'scene_mix0125.rsp',   # 0125: Jude (block 4201 gunshots and gun clicks) for Kreely's pistol (added use)
         'scene_mix0303.rsp', 'event/scene_mix7547.rsp', 'event/scene_mix7548.rsp',   # c303 Elise, ev33 / ev34
         'scene_mix0503.rsp', 'event/scene_mix7568.rsp', 'event/scene_mix7569.rsp',   # c503 the Black Baron, ev50 / ev51
         'event/scene_mix7549.rsp', 'event/scene_mix7570.rsp']   # ev84 (after Elise) / ev93 (after the Baron)
OPLEN = {0x20: 1, 0x22: 1, 0x12: 1, 0x14: 1, 0x15:2, 0x1F: 3, 0x60: 0, 0x00: 0, 0x36: 1, 0x35: 1, 0x78: 2, 0x18: 0}


def parse_track(cmds):
    """Partial one-shot inspection. Native0x12 is note/key-on, not pan."""
    wave, delay, vol, pan, note, i, seen = 0, 0, 0x7F, 0x40, 60, 0, False
    while i < len(cmds):
        op = cmds[i]
        n = OPLEN.get(op, 0)
        arg = cmds[i + 1:i + 1 + n]
        if op == 0x20 and arg: wave = arg[0]; seen = True
        elif op == 0x1F and len(arg) == 3: delay = (arg[0] << 16) | (arg[1] << 8) | arg[2]
        elif op == 0x22 and arg: vol = arg[0]
        elif op == 0x12 and arg: note = arg[0]; seen = True
        i += 1 + n
    return [dict(wave=wave, delay=delay, volume=vol, pan=pan,note=note)] if seen else []


def definitions(member):
    """Program start = 0x40 + 4 * count + offset (fits every bank checked); +0 tracks, +1 volume, +0x10 wave block,
    +0x12 per-track lengths, then the tracks."""
    head = decrypt(member, 0x40)
    base = struct.unpack_from('>I', head, 0x18)[0]
    d = decrypt(member, base) + member[base:]
    block = struct.unpack_from('>I', d, 0x14)[0]
    count = struct.unpack_from('>I', d, 0x30)[0]
    if count > 4096: return block, {}
    if 0x40+4*count>len(d):raise ValueError('Truncated sound definition table')
    offs = [struct.unpack_from('>I', d, 0x40 + 4 * i)[0] for i in range(count)]
    sounds = {}
    for sid, off in enumerate(offs):
        if off == 0xFFFFFFFF: continue
        a = 0x40 + 4 * count + (off&0x7fffffff)
        if a + 0x10 > len(d): continue
        tracks = d[a]&0x7f
        banks = d[a+2]
        lengths = a+0x10+2*banks
        if tracks == 0 or tracks > 16 or not banks or lengths + 2 * tracks > len(d): continue
        wave_blocks = [struct.unpack_from('>H', d, a+0x10+2*i)[0]for i in range(banks)]
        lens = [struct.unpack_from('>H', d, lengths + 2 * t)[0]&0x7fff for t in range(tracks)]
        p = lengths + 2 * tracks
        plays = []
        for n in lens:
            plays += parse_track(d[p:p + n]); p += n
        if plays: sounds[sid] = dict(volume=d[a + 1], wave_block=wave_blocks[0], wave_blocks=wave_blocks, plays=plays)
    return block, sounds


def wave_info(bank):
    """[(rate, root key)] per wave of a RedSpark wave bank."""
    h = decrypt(bank, min(len(bank), 0x8000))
    count = struct.unpack_from('>H', h, 0x3E)[0]
    out = []
    for i in range(count):
        o = struct.unpack_from('>I', h, 0x50 + 4 * i)[0]
        if o + 0x24 > len(h): out.append((24000, 60)); continue
        tune = struct.unpack_from('>i', h, o + 0x1C)[0] / 2 ** 24
        out.append((round(32000 * 2 ** (tune / 12)), h[o + 0x22]))
    return out


def set_rate(wav, rate):
    b = bytearray(wav.read_bytes())
    if b[:4] != b'RIFF' or b[8:16] != b'WAVEfmt ': return
    ch, bits = struct.unpack_from('<H', b, 22)[0], struct.unpack_from('<H', b, 34)[0]
    if struct.unpack_from('<I', b, 24)[0] == rate: return
    struct.pack_into('<I', b, 24, rate); struct.pack_into('<I', b, 28, rate * ch * bits // 8)
    wav.write_bytes(bytes(b))


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    bank_files, defs = {}, {}
    for pk in PACKS:
        data = (SE / pk).read_bytes()
        rows = pack_members(data)
        o0, s0 = rows[0]; waves = data[o0:o0 + s0]
        for o, s in pack_members(waves):
            w = waves[o:o + s]
            blk = struct.unpack_from('>I', decrypt(w, 0x40), 0x14)[0]
            if blk in bank_files: continue
            f = OUT / f'{Path(pk).stem}-{blk}.rsd'; f.write_bytes(w); bank_files[blk] = f
        for o, s in rows[1:]:
            m = data[o:o + s]
            if s < 0x48 or decrypt(m, 8)[:8] != b'RedSpark': continue
            blk, sounds = definitions(m)
            if blk not in defs and sounds: defs[blk] = sounds
    infos = {blk: wave_info(f.read_bytes()) for blk, f in bank_files.items()}
    manifest = {}
    for blk, sounds in defs.items():
        entry = {}
        for sid, snd in sounds.items():
            wb = next((b for b in snd['wave_blocks']if b in bank_files),None)
            if wb is None: continue
            plays = []
            for pl in snd['plays']:
                wav = OUT / f'{wb}-{pl["wave"]:03}.wav'
                if not wav.exists():
                    subprocess.run([str(VGM), '-i', '-s', str(pl['wave'] + 1), '-o', str(wav), str(bank_files[wb])], capture_output=True, text=True)
                    if not wav.exists(): continue
                rate, root = infos[wb][pl['wave']] if pl['wave'] < len(infos[wb]) else (24000, 60)
                set_rate(wav, rate)
                plays.append(dict(pl, file=f'native/audio/se/{wav.name}', rate=rate, root=root))
            if plays: entry[str(sid)] = dict(volume=snd['volume'], plays=plays)
        if entry: manifest[str(blk)] = entry
    (OUT / 'se.json').write_text(json.dumps(manifest, separators=(',', ':')), encoding='utf-8')
    print(json.dumps({k: len(v) for k, v in manifest.items()}))


if __name__ == '__main__':
    main()
