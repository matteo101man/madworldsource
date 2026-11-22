"""Audit owned native normal/skip boss-entry order, fade and music admission."""
from pathlib import Path
import hashlib,json,struct
ROOT=Path(__file__).resolve().parents[2]
def f32(v):return struct.unpack('>f',struct.pack('>f',v))[0]


def main():
    reports=[]
    for name,skip_frame,track in [('ev21-lifecycle-audio',None,30),('ev21-skip-audio',300,24),('ev21-skip-late-audio',820,30)]:
        skip=skip_frame is not None
        path=ROOT/'evidence'/f'{name}.jsonl'
        rows=[r for line in path.read_text(encoding='utf-8').splitlines()if 'row'in(r:=json.loads(line))]
        end=next(i for i,r in enumerate(rows)if r['state']==6);cleanup=end+1
        first=next(i for i,r in enumerate(rows)if r['rinMoved'] is not None)
        assert rows[cleanup]['state']==0 and rows[cleanup]['frame']==0 and first>cleanup+14
        playing=[r for r in rows[:end]if r['state']==5]
        assert [r['frame']for r in playing]==list(range(skip_frame+1 if skip else 1080))
        assert rows[end]['frame']==(skip_frame+1 if skip else 1079)
        assert all(r['skipEnabled']==1 for r in playing)
        if skip:assert rows[end]['padDown']&0x20
        alphas=[r['fade']['rgba']for r in rows[first:first+16]]
        expected=[int(f32(f32(1.-f32(age/15.))*255.))for age in range(16)]
        assert alphas==expected
        assert all(r['sound']['currentMusic']==track for r in rows[first:first+60])
        filename='BGM/bs04.rsd'if track==24 else'BGM/bs04c.rsd'
        assert any(s['name']==filename for s in rows[first]['sound']['slots'])
        log=(ROOT/'evidence'/f'{name}.log').read_text(encoding='utf-8',errors='replace')
        assert log.index('[event] request=21 returned')<log.index('[rin-native] onInit begin')
        if skip:assert f'[event-skip-fixture] original WPAD B packet at event frame{skip_frame}'in log
        reports.append({'capture':name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'rows':len(rows),
            'playingRows':len(playing),'skipInputFrame':skip_frame,'ending':{'row':rows[end]['row'],'clock':rows[end]['frame']},
            'cleanupRow':rows[cleanup]['row'],'roomResumeRow':rows[cleanup+14]['row'],
            'firstCombatMoveRow':rows[first]['row'],'variableResourceWait':first-cleanup-14,
            'roomFadeAlpha':alphas,'roomMusic':track,'roomMusicFile':filename})
    result={'passed':True,'captures':reports,'authority':'80123820 waits for event cleanup, then yields13 updates; scheduler resume is the following edge. 80147BBC creates combat Rin only after this call returns and fades15 updates. 800A49F0 uses B/Menu Back; 8020EFC0 maps WPAD B to0x20.',
        'limits':'World-update observations, not pixel/audio onset proof. rinMoved begins after first onMove, not allocation. The diagnostic boot has other nested startup tasks; their remaining demo counters and later fades are not the boss-entry sequence. Raw event field in capture is manager+0x130 and is not used as an event identifier.'}
    (ROOT/'research/EV21-ROOM-COMPARE.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,indent=2))


if __name__=='__main__':main()
