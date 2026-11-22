"""Summarize numeric speaker diagnostics; never expose packet audio/key bytes."""
from pathlib import Path
import argparse,collections,hashlib,json
ROOT=Path(__file__).resolve().parents[1]
def summarize(label):
    if not label.replace('-','').isalnum():raise ValueError('Diagnostic label')
    path=ROOT/'evidence'/f'{label}.log';lines=path.read_text(encoding='utf-8',errors='replace').splitlines()
    def rows(prefix):return [json.loads(line.removeprefix(prefix)) for line in lines if line.startswith(prefix)]
    reports=rows('REMOTE_AUDIO ');decoded=rows('REMOTE_AUDIO_DECODE ');states=rows('REMOTE_SPEAKER_STATE ');registers=rows('REMOTE_REGISTER ');mixes=rows('REMOTE_AUDIO_MIX ')
    speaker_registers=[r for r in registers if r['space']!=0 and r['address']&0xfe0000==0xa20000]
    data=[r for r in reports if r['report']==24]
    run=json.loads(path.with_suffix('.json').read_text(encoding='utf-8'))
    result=dict(label=label,logSha256=hashlib.sha256(path.read_bytes()).hexdigest(),traceEnabled='REMOTE_AUDIO_TRACE enabled=1' in lines,reportCounts={hex(k):v for k,v in collections.Counter(r['report'] for r in reports).items()},speakerDataReports=len(data),declaredSpeakerDataBytes=sum(r['declaredDataBytes'] for r in data),malformedSpeakerDataReports=sum(r['declaredDataBytes']>20 or r['declaredDataBytes']>r['availableDataBytes'] for r in data),decodedOutputBlocks=len(decoded),decodedOutputSamples=sum(r['samples'] for r in decoded),decodedRates=dict(collections.Counter(r['rate'] for r in decoded)),decodedVolumes=dict(collections.Counter(r['volume'] for r in decoded)),speakerRegisterOperations=speaker_registers,speakerStateObservations=states,mixerSummary=mixes[-1] if mixes else None,gpu=run['gpu'],exitCode=run['exitCode'],scope='Observed protocol traffic and produced speaker PCM only; no human listening, original mix, complete sound/event coverage or timing parity claim')
    return result

def main():
    ap=argparse.ArgumentParser();ap.add_argument('labels',nargs='+');a=ap.parse_args();results=[summarize(label) for label in a.labels]
    (ROOT/'evidence/native-remote-audio-summary.json').write_text(json.dumps(dict(replays=results),indent=2)+'\n',encoding='utf-8')
    print(json.dumps([dict(label=r['label'],speakerDataReports=r['speakerDataReports'],decodedOutputBlocks=r['decodedOutputBlocks'],exitCode=r['exitCode']) for r in results]))
if __name__=='__main__':main()
