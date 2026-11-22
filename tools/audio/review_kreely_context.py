"""Make context-directed review takes without overwriting the existing voice bank."""
import argparse
import hashlib
import io
import json
import math
from pathlib import Path
import sys
import time

import numpy as np
import requests
import soundfile as sf
from scipy.signal import resample_poly

sys.path.insert(0, str(Path(__file__).parent))
from stage_kreely_voice import read_lines

ROOT = Path(__file__).resolve().parents[2]
PILOTS = ['defeat_04', 'knockdown_04', 'hurt_03', 'near_death_03', 'teeth_02']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--all', action='store_true', help='Render all 40 into the review folder')
    parser.add_argument('--only', nargs='*')
    parser.add_argument('--force', action='store_true')
    parser.add_argument('--direct', action='store_true', help='Use the isolated VoxCPM runtime with bounded CPU threads')
    parser.add_argument('--base-url', default='http://127.0.0.1:3900')
    args = parser.parse_args()
    direction = json.loads((ROOT/'research/KREELY-VOICE-DIRECTION.json').read_text(encoding='utf-8'))
    source = {r['key']: r for r in read_lines(ROOT/'research/KREELY-VOICE.md')}
    assert set(source) == set(direction['lines']), 'Every line needs its own direction'
    selected = args.only or (list(source) if args.all else PILOTS)
    out = ROOT/'assets/native/audio/kreely/review'/direction.get('revision','contextual-v2')
    out.mkdir(parents=True, exist_ok=True)
    manifest_path = out/'review.json'
    manifest = json.loads(manifest_path.read_text(encoding='utf-8')) if manifest_path.exists() else {'takes': {}}
    session = requests.Session()
    direct_model = None
    if args.direct:
        import os
        import torch
        from voxcpm import VoxCPM
        torch.set_num_threads(4)
        direct_model = VoxCPM.from_pretrained(os.environ['OMNIVOICE_VOXCPM_MODEL'], load_denoiser=False, optimize=False, device='cuda')
    for key in selected:
        row = source[key]
        reference = Path(direction.get('reference_per_line', {}).get(key,
            direction.get('reference_per_set', {}).get(row['set'], direction['voice_reference'])))
        reference_sha = hashlib.sha256(reference.read_bytes()).hexdigest()
        # Plain words here. Nonverbal actions belong in the delivery instruction,
        # not OmniVoice-specific tags or an imposed duration.
        text = 'Hey! I was using those!' if key == 'teeth_06' else row['text']
        instruction = direction.get('common_control','In-person brawler dialogue. ')+direction.get('pilot_controls',{}).get(key,direction['lines'][key])
        identity = {'engine': 'voxcpm2', 'text': text, 'instruct': instruction,
                    'reference_file': str(reference), 'reference_sha256': reference_sha, 'seed': 21751, 'steps': 32,
                    'guidance': 2.0, 'effect_preset': 'raw', 'duration': None}
        fingerprint = hashlib.sha256(json.dumps(identity,sort_keys=True).encode()).hexdigest()
        path = out/(key+'.wav')
        old = manifest['takes'].get(key,{})
        if not args.force and path.exists() and old.get('fingerprint')==fingerprint:
            print('Reusing',key,flush=True)
            continue
        data = {'text': text, 'engine': 'voxcpm2', 'language': 'English', 'instruct': instruction,
                'seed': '21751', 'num_step': '32', 'guidance_scale': '2.0', 'speed': '1.0',
                'denoise': 'false', 'effect_preset': 'raw', 'wav_bits': '16'}
        start = time.monotonic()
        history_id = None
        if direct_model is not None:
            torch.manual_seed(21751)
            audio = direct_model.generate(text=f'({instruction}){text}', reference_wav_path=str(reference),
                cfg_value=2.0, inference_timesteps=32, max_len=max(256,len(text)*6),
                retry_badcase=False, retry_badcase_max_times=1)
            if hasattr(audio,'detach'):
                audio=audio.detach().float().cpu().numpy()
            audio=np.asarray(audio,dtype=np.float32).squeeze()
            sr=48000
        else:
            with reference.open('rb') as f:
                response = session.post(args.base_url+'/generate',data=data,
                    files={'ref_audio': (reference.name,f,'audio/wav')}, timeout=1800)
            if not response.ok:
                raise RuntimeError(f'{key}: {response.status_code}: {response.text[:3000]}')
            audio,sr = sf.read(io.BytesIO(response.content),dtype='float32')
            history_id=response.headers.get('X-Audio-Id')
        if audio.ndim>1:
            audio=audio.mean(axis=1)
        d=math.gcd(sr,44100)
        audio=resample_poly(audio,44100//d,sr//d)
        if not np.isfinite(audio).all() or np.sqrt(np.mean(audio**2))<.001:
            raise ValueError('Unusable audio: '+key)
        # Constant headroom only, preserving the line's internal dynamics.
        if max(abs(audio))>.95:
            audio *= .95/max(abs(audio))
        sf.write(path,audio,44100,subtype='PCM_16')
        manifest['takes'][key] = {**identity, 'fingerprint': fingerprint,'file':str(path),
              'duration_s':len(audio)/44100,'render_seconds':time.monotonic()-start,
              'studio_history_id':history_id, 'execution':'isolated direct runtime' if args.direct else 'VoiceStudio API',
              'sha256':hashlib.sha256(path.read_bytes()).hexdigest(), 'review_status':'pending user listening review'}
        manifest_path.write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
        print(key,round(len(audio)/44100,2),'seconds',flush=True)
    preview=[]
    preview_keys = PILOTS if args.all else selected
    notes=['# Context-directed Kreely review','',
           'The existing bank is unchanged. Reference sources and individual delivery instructions are recorded in review.json. No fixed duration is imposed.','']
    for key in selected:
        path=out/(key+'.wav')
        audio,sr=sf.read(path)
        if key in preview_keys:
            preview.extend([audio,np.zeros(int(sr*.7))])
        notes += ['## '+key,'','['+source[key]['text']+']('+path.as_posix()+')','',direction['lines'][key],'']
    sf.write(out/'contextual-review.wav',np.concatenate(preview[:-1]),44100,subtype='PCM_16')
    (out/'index.md').write_text('\n'.join(notes)+'\n',encoding='utf-8')
    print('Review ready:',out,flush=True)


if __name__=='__main__':
    main()
