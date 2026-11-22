"""Stage a bank's camera MOTs (the pair cameras: pl0030_c46x in em0630.dat, pl0030_c48x / em0623_c00x in em0623.dat ...).

python -I tools/stage_camera_clips.py em/em0630.dat assets/native/elise-camera.json

A camera MOT keeps the actor root (bone -1, x y z) plus two pseudo-bones the model exporter skips (bone >= 0xF60):
0xF60 channels 0..2 the eye, 3 the field of view in radians (0.8 throughout the c46x set), 5 the roll; 0xF61
channels 0..2 the look target. Eye and target are in the pair's frame (origin at the pair's anchor, +Z its facing),
the same frame as the paired clips' root travel [inferred from the c46x data: target y ~1.35 = chest height].
Values are sampled per frame with the explorer's MOT reader (Hermite keys), unchanged.
"""
import json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
EXPORT = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPORT / 'builders'))
import native  # noqa: E402

# the explorer's reader, keeping the camera pseudo-bones it normally skips
_src = (EXPORT / 'builders/native.py').read_text(encoding='utf-8')
_code = _src[_src.index('def motion(data):'):_src.index('    return dict(frames=frames,tracks=tracks')] + '    return frames,tracks\n'
_code = _code.replace('def motion(data):', 'def cam_motion(data):').replace(
    'if bone==32767 or bone>=0xf60 or fmt==255: continue', 'if bone==32767 or fmt==255: continue')
_ns = dict(native.__dict__); exec(_code, _ns); cam_motion = _ns['cam_motion']


def main():
    dat, out = sys.argv[1], ROOT / sys.argv[2]
    raw = (EXPORT / 'extracted/disc/DATA/files' / dat).read_bytes()
    clips = {}
    for e in native.dat_entries(raw):
        if not e['name'].endswith('.mot') or not e['size']: continue
        frames, tracks = cam_motion(raw[e['offset']:e['offset'] + e['size']])
        ch = {(t['bone'], t['channel']): t['values'] for t in tracks}
        if (0xF60, 0) not in ch or (0xF61, 0) not in ch: continue
        get = lambda b, c, d=0.: ch.get((b, c), [d] * frames)
        eye = [list(v) for v in zip(get(0xF60, 0), get(0xF60, 1), get(0xF60, 2))]
        target = [list(v) for v in zip(get(0xF61, 0), get(0xF61, 1), get(0xF61, 2))]
        clips[e['name']] = dict(frames=frames, eye=eye, target=target, fov=get(0xF60, 3, 0.8), roll=get(0xF60, 5))
    out.write_text(json.dumps(dict(schema='mw-camera-clips/1', source=dat, clips=clips), separators=(',', ':')), encoding='utf-8')
    print(json.dumps({k: v['frames'] for k, v in clips.items()}))


if __name__ == '__main__':
    main()
