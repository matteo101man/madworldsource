"""Stage original event music/English voice and native RedSpark mix metadata (MW_EVENT, default ev21).

Voice: localised event streams (id 0x11000+N) read evvostr/evvoN_ENG.rsd; story-scene streams
(id 0x10000+N, e.g. ev84/ev93) have no language variant and read evvostr/evvoN.rsd.

Name/ID tables: 8052F750 BGM, 8054F430 event voice; BGM filenames 805CB2C8.
The event's mode2 uses music master100, not its nominal command volume60.
Decodes source PCM without applying gain; Rust translates the integer mixer.
"""
import hashlib
import importlib.util
import json
import os
import subprocess
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DISC = Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files')
VGM = ROOT / 'tools/vgmstream/vgmstream-cli.exe'


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


dol = module('dol', 'tools/pl0000/dol.py').Dol()
decrypt = module('redspark', 'tools/audio/redspark.py').decrypt


def table_id(address, count, name):
    matches = [dol.u32(address+i*8+4) for i in range(count)
               if dol.cstr(dol.u32(address+i*8)) == name]
    if len(matches) != 1:
        raise ValueError(f'Native stream name not unique: {name}')
    return matches[0]


def mix(source, volume):
    header = decrypt(source.read_bytes(), 0x100)
    if header[:8] != b'RedSpark':
        raise ValueError('RedSpark header')
    channels = header[0x4e]
    gains = [header[0x57+i*8] for i in range(channels)]
    if not gains or len(set(gains)) != 1 or max(gains+[header[0x51]]) > 127:
        raise ValueError('Stream requires per-channel gain handling')
    return {'volume': volume, 'header': header[0x51], 'channel': gains[0]}


def info(path):
    with wave.open(str(path)) as wav:
        if wav.getsampwidth() != 2 or wav.getcomptype() != 'NONE':
            raise ValueError('Expected original decoded16-bit PCM')
        return {'rate': wav.getframerate(), 'channels': wav.getnchannels(), 'samples': wav.getnframes()}


def main():
    ev = os.environ.get('MW_EVENT', 'ev21')
    event = json.loads((ROOT/'assets/native/events'/ev/'event.json').read_text())
    names = {}
    for resource in event['resources']:
        for command in resource['commands']:
            if (resource['tag'],command['opcode']) not in [('BGM ',8),('VCE ',9)]:
                continue
            args = command['operands']
            if args[0]&3:
                name = b''.join(v.to_bytes(4,'big') for v in args[1:13]).split(b'\0',1)[0].decode('ascii')
                names[name] = resource['tag'].strip()
    out = ROOT/'assets/native/events'/ev/'audio'
    out.mkdir(parents=True,exist_ok=True)
    assets = {}
    bgm_path = ROOT/'assets/native/audio/bgm/bgm.json'
    bgm = json.loads(bgm_path.read_text())
    for name, bus in names.items():
        if bus == 'BGM':
            ident = table_id(0x8052f750,81,name)
            filename = dol.cstr(dol.u32(0x805cb2c8+ident*4))
        else:
            ident = table_id(0x8054f430,74,name)
            # the event's own stream. Localised: id 0x11000 + the event number (ev21: 0x11021, evvo21_ENG;
            # ev51 also carries its 'c' take: 0x1151c, evvo51c_ENG). Story scenes: id 0x10000 + the event
            # number with no language file (ev84: 0x10084, evvo84; ev93c: 0x1093c, evvo93c).
            localised = ident >= 0x11000
            tag = format(ident - (0x11000 if localised else 0x10000), 'x')
            if ident < 0x10000 or not tag.startswith(ev[2:]):
                raise ValueError(f'Voice id {ident:#x} is not {ev}\'s own stream')
            filename = f'evvostr/evvo{tag}_ENG.rsd' if localised else f'evvostr/evvo{tag}.rsd'
        source = DISC/filename
        data = {'id':ident,'bus':bus,'source':filename,'sourceSha256':hashlib.sha256(source.read_bytes()).hexdigest(),
                'mix':mix(source,100 if bus=='BGM' else 127)}
        if name == 'BS04C_RSD':
            track = bgm['bs04c']
            intro = info(ROOT/'assets'/track['intro'])
            loop = info(ROOT/'assets'/track['loop'])
            if intro['samples']!=track['loop_start'] or loop['samples']!=track['loop_end']-track['loop_start'] or intro['rate']!=loop['rate']:
                raise ValueError('Existing boss loop assets differ from native loop metadata')
            data.update({'intro':track['intro'],'loop':track['loop'],'rate':track['rate'],
                         'loopStart':track['loop_start'],'loopEnd':track['loop_end'],'channels':intro['channels']})
            track['mix'] = data['mix']
        else:
            destination = out/(Path(filename).stem+'.wav')
            subprocess.run([str(VGM),'-i','-o',str(destination),str(source)],capture_output=True,check=True)
            data.update(info(destination))
            data['file'] = destination.relative_to(ROOT/'assets').as_posix()
            data['wavSha256'] = hashlib.sha256(destination.read_bytes()).hexdigest()
        assets[name] = data
    manifest = {'schema':'mw-event-audio/1','event':event['event'],'eventSourceSha256':event['sourceSha256'],
                'dolSha256':hashlib.sha256(dol.b).hexdigest(),'language':'ENG','streams':assets}
    (out/'audio.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    bgm_path.write_text(json.dumps(bgm,indent=1)+'\n',encoding='utf-8')
    print(json.dumps(manifest,indent=2))


if __name__ == '__main__':
    main()
