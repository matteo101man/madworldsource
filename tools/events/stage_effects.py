"""Stage EV21's two verified numeric2D effects and compare native SDK records.

Dry stereo mix only: native effect sends, DSP envelopes/resampling and audible
PCM equivalence remain separate requirements.
"""
import hashlib,importlib.util,json,struct,wave
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('stage_se',ROOT/'tools/audio/stage_se.py')
se=importlib.util.module_from_spec(spec);spec.loader.exec_module(se)


def main():
    event=json.loads((ROOT/'assets/native/events/ev21/event.json').read_text())
    native=[r for r in map(json.loads,(ROOT/'evidence/ev21-native-se-v2.jsonl').read_text().splitlines())if 'before'in r]
    assert len(native)==1080 and [r['before']for r in native]==list(range(1080))
    source=se.SE/'event/scene_mix7522.rsp';raw=source.read_bytes();offset,size=se.pack_members(raw)[1]
    member=raw[offset:offset+size];head=se.decrypt(member,0x40);base=struct.unpack_from('>I',head,0x18)[0]
    bank=se.decrypt(member,base)+member[base:];count=struct.unpack_from('>I',bank,0x30)[0]
    block,defs=se.definitions(member);assert block==7522 and count==7
    sounds={};comparisons=[]
    for resource in event['resources']:
        for cue in resource['commands']:
            if resource['tag']!='SE  ' or cue['opcode']!=13:continue
            args=cue['operands'];ident=args[1];sid=ident%1000
            assert args[0]&7==5 and ident//1000==7522 and struct.unpack('>f',args[21].to_bytes(4,'big'))[0]==127.
            off=struct.unpack_from('>I',bank,0x40+sid*4)[0];start=0x40+count*4+(off&0x7fffffff)
            program=bank[start:start+32];assert program[0:3]==bytes([1,127,1])
            length=struct.unpack_from('>H',program,18)[0]&0x7fff;code=list(program[20:20+length]);assert length==11
            assert code==[0x20,sid,0x22,127,0x60,0x12,60,0x15,code[8],code[9],0]
            hold=int.from_bytes(bytes(code[8:10]),'big');assert defs[sid]['plays'][0]['wave']==sid
            recorded=[(r['before'],slot,v)for r in native for slot in r['eventSE']if slot['id']==ident for v in slot['voices']]
            assert recorded and recorded[0][0]==cue['frame']+1
            for _,_,v in recorded:
                assert v['program']==program.hex() and (v['waveBlock'],v['waveIndex'],v['note'],v['seStep'])==(7522,sid,60,1)
            gains={(v['left'],v['right'])for _,_,v in recorded};assert len(gains)==1
            left,right=gains.pop();assert left==right and left>0
            remaining=recorded[0][2]['waitTicks']*416/120*96/32000
            observed=(recorded[-1][0]+1-recorded[0][0])*1001/30000
            assert abs(remaining-observed)<1001/30000
            file=ROOT/f'assets/native/audio/se/7522-{sid:03}.wav'
            with wave.open(str(file))as wav:
                assert wav.getnchannels()==1 and wav.getsampwidth()==2
                rate,samples=wav.getframerate(),wav.getnframes()
            sounds[str(ident)]={'file':file.relative_to(ROOT/'assets').as_posix(),'rate':rate,'samples':samples,
                'wavSha256':hashlib.sha256(file.read_bytes()).hexdigest(),'programOffset':start,'program':code,
                'dryMix':{'left':left,'right':right,'denominator':32768},'holdTicks':hold}
            comparisons.append({'id':ident,'cueFrame':cue['frame'],'wave':sid,'note':60,'voiceRows':len(recorded),
                'firstVoiceFrame':recorded[0][0],'lastVoiceFrame':recorded[-1][0],
                'remainingSecondsFromCounter':remaining,'observedRemainingSeconds':observed,'dryMix':[left,right]})
    assert len(sounds)==2
    scope='Verified default dry stereo path; effect sends, DSP release/resampling and PCM parity remain unported/unverified.'
    manifest={'schema':'mw-event-effects/1','event':event['event'],'eventSourceSha256':event['sourceSha256'],
        'source':'se/event/scene_mix7522.rsp','sourceSha256':hashlib.sha256(raw).hexdigest(),'scope':scope,'sounds':sounds}
    (ROOT/'assets/native/events/ev21/audio/effects.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    report={'nativeRows':1080,'comparisons':comparisons,'scope':scope}
    (ROOT/'research/EV21-SE-COMPARE.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(report,indent=2))


if __name__=='__main__':main()
