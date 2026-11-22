"""Check ev21 stream name/ID/cue staging against the owned native manager capture.

Manager readiness/play states are not audible sample timestamps or PCM equivalence.
"""
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]


def main():
    event=json.loads((ROOT/'assets/native/events/ev21/event.json').read_text())
    manifest=json.loads((ROOT/'assets/native/events/ev21/audio/audio.json').read_text())
    rows=[r for r in map(json.loads,(ROOT/'evidence/ev21-native-audio.jsonl').read_text().splitlines()) if 'before' in r]
    assert len(rows)==1080 and [r['before'] for r in rows]==list(range(1080))
    assert manifest['eventSourceSha256']==event['sourceSha256']
    report=[]
    for resource in event['resources']:
        for cue in resource['commands']:
            if (resource['tag'],cue['opcode']) not in [('BGM ',8),('VCE ',9)]:continue
            args=cue['operands']
            name=b''.join(v.to_bytes(4,'big')for v in args[1:13]).split(b'\0',1)[0].decode('ascii')
            track=manifest['streams'][name]
            def states(row):
                sound=row['sound']
                return sound['musicSlots'] if track['bus']=='BGM' else [sound['voice']]
            ready=[r['before'] for r in rows if any(s['name']==track['source'] and s['state'] in [4,5] for s in states(r))]
            playing=[r['before'] for r in rows if any(s['name']==track['source'] and s['state']==5 for s in states(r))]
            assert ready and ready[0] in [cue['frame'],cue['frame']+1]
            assert playing and playing[0] in [cue['frame'],cue['frame']+1]
            native=rows[cue['frame']]['sound']
            assert (native['currentMusic'] if track['bus']=='BGM' else native['voice']['id'])==track['id']
            if 'file' in track:
                assert hashlib.sha256((ROOT/'assets'/track['file']).read_bytes()).hexdigest()==track['wavSha256']
            report.append({'name':name,'id':track['id'],'source':track['source'],'cueFrame':cue['frame'],
                           'firstReadyFrame':ready[0],'firstPlayingFrame':playing[0],
                           'lastPlayingFrame':playing[-1],'mix':track['mix']})
    assert len(report)==3
    result={'nativeRows':len(rows),'streams':report,'limits':['Readiness/play states do not establish audible sample onset or PCM parity.',
            'Preceding st06ab stage music, event SE13, subtitles/ID/effects and scene lifecycle are outside this check.']}
    (ROOT/'research/EV21-AUDIO-COMPARE.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,indent=2))


if __name__=='__main__':main()
