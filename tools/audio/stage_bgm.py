"""Stage the RinRin boss BGM for the native slice: intro + loop WAVs split at the RedSpark loop points.

python -I tools/audio/stage_bgm.py
Native facts (research/AUDIO-API.md):
- BGM name table at 805CB2C8 (index -> BGM/*.rsd); 801246D8(id) requests a track, 8026AC24 / 8026A4A4 stream it.
- The stage-201 boss entry (80147BBC) requests 0x18 (BGM/bs04.rsd); the recompiled-core disc log of the stage-201
  boss streams BGM/bs04c.rsd (index 0x1E). Both share the same loop (2420362 samples); bs04c has the longer intro.
  EV21 BGM SEQ requests BS04C at frame748; its mode2 uses music master100.
  Original stream/header/channel integer gain is translated in mw_sim::event::stream_gain.
Loop points come from the RedSpark header (vgmstream metadata). Output stays local (assets/ is gitignored).
"""
import importlib.util, json, re, subprocess, wave
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BGM = Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files/BGM')
VGM = ROOT / 'tools/vgmstream/vgmstream-cli.exe'
OUT = ROOT / 'assets/native/audio/bgm'
TRACKS = ['bs04c','bs04','bs03','bs05']   # bs03 / bs05: the Black Baron (c503) fight and its angry phase
spec=importlib.util.spec_from_file_location('redspark',Path(__file__).with_name('redspark.py'))
redspark=importlib.util.module_from_spec(spec);spec.loader.exec_module(redspark)


def stage(name):
    src = BGM / f'{name}.rsd'
    header=redspark.decrypt(src.read_bytes(),0x100)
    channels=[header[0x57+i*8]for i in range(header[0x4e])]
    if len(set(channels))!=1 or max(channels+[header[0x51]])>127:raise ValueError('Requires per-channel stream gain')
    meta = subprocess.run([str(VGM), '-m', str(src)], capture_output=True, text=True).stdout
    loop = int(re.search(r'loop start: (\d+)', meta).group(1)); end = int(re.search(r'loop end: (\d+)', meta).group(1))
    full = OUT / f'{name}-full.wav'
    subprocess.run([str(VGM), '-i', '-o', str(full), str(src)], capture_output=True, check=True)
    with wave.open(str(full)) as w:
        params = w.getparams(); frames = w.readframes(w.getnframes())
    step = params.sampwidth * params.nchannels
    for part, data in (('intro', frames[:loop * step]), ('loop', frames[loop * step:end * step])):
        with wave.open(str(OUT / f'{name}-{part}.wav'), 'wb') as o:
            o.setparams(params); o.writeframes(data)
    full.unlink()
    return dict(intro=f'native/audio/bgm/{name}-intro.wav', loop=f'native/audio/bgm/{name}-loop.wav',
                rate=params.framerate, loop_start=loop, loop_end=end,
                mix=dict(volume=100,header=header[0x51],channel=channels[0]))


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {name: stage(name) for name in TRACKS}
    manifest['boss'] = 'bs04c'
    manifest['boss_room'] = 'bs04'
    (OUT / 'bgm.json').write_text(json.dumps(manifest, indent=1), encoding='utf-8')
    print(json.dumps(manifest))


if __name__ == '__main__':
    main()
