"""Prepare scene-matched references from Matteo's original impression recordings."""
import hashlib
import json
import math
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly

ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(r'C:\Users\Matteo\Desktop\My impressions - Voice Clips\Individual Clips')
OUT = ROOT/'assets/native/audio/kreely/review/contextual-v4/references'
GROUPS = {
    'taunt': [2, 7, 9, 13, 16],
    'challenge': [3, 5, 7, 8, 16],
    'pain': [16, 18, 19, 20, 21, 23],
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {'source': 'Original user-recorded Bender impressions',
                'processing': 'Mono, 48 kHz, edge silence trim with breathing room, constant gain per clip, short gaps. No pitch/formant shift.',
                'references': {}}
    for name, numbers in GROUPS.items():
        parts, sources = [], []
        for number in numbers:
            path = SOURCE/f'Bender - {number:03}.wav'
            audio, sr = sf.read(path, dtype='float32')
            if audio.ndim > 1:
                audio = audio.mean(axis=1)
            active = np.flatnonzero(np.abs(audio) > max(.002, np.max(np.abs(audio))*.01))
            if not len(active):
                raise ValueError(f'Silent reference: {path}')
            pad = int(sr*.08)
            audio = audio[max(0, active[0]-pad):min(len(audio), active[-1]+pad+1)]
            divisor = math.gcd(sr, 48000)
            audio = resample_poly(audio, 48000//divisor, sr//divisor)
            rms = np.sqrt(np.mean(audio**2))
            audio *= min(.10/max(rms, 1e-6), .90/max(np.max(np.abs(audio)), 1e-6))
            fade = min(240, len(audio)//2)
            audio[:fade] *= np.linspace(0, 1, fade)
            audio[-fade:] *= np.linspace(1, 0, fade)
            parts += [audio, np.zeros(5760, dtype='float32')]
            sources.append({'file': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                            'used_seconds': len(audio)/48000})
        dest = OUT/(name+'.wav')
        combined = np.concatenate(parts[:-1])
        sf.write(dest, combined, 48000, subtype='PCM_16')
        manifest['references'][name] = {'file': str(dest), 'duration_s': len(combined)/48000,
                                      'sha256': hashlib.sha256(dest.read_bytes()).hexdigest(), 'sources': sources}
        print(name, round(len(combined)/48000, 2), 'seconds', flush=True)
    (OUT/'provenance.json').write_text(json.dumps(manifest, indent=2)+'\n', encoding='utf-8')


if __name__ == '__main__':
    main()
