"""Structural audit of recorded native roles against complete owned MOT bytes."""
import argparse,hashlib,json,math
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--label',default='bevy-native-pc-rin-pair');a=ap.parse_args()
    if not a.label.replace('-','').isalnum():raise ValueError('Replay label')
    export=Path('D:/Games/gamesfordecomp/exports/madworld')
    catalog=json.loads((export/'catalog.json').read_text(encoding='utf-8'));clips={r['id']:r for r in catalog['clips']};banks={r['id']:r for r in catalog['banks']}
    path=ROOT/'evidence'/f'{a.label}-feedback-observations.jsonl';count=0;checked=set()
    for row in map(json.loads,path.read_text(encoding='utf-8').splitlines()):
        player=[r for r in row.get('playerNativeMotion',{}).get('resourceMatches',[]) if r['bank']=='em/em0440.dat']
        rin=row.get('rinNativeMotion',{}).get('resourceMatches',[])
        if not player or not rin:continue
        assert row['actors']['rin']['id']=='0x20440' and row['actors']['player']['id'] in ('0x10000','0x10030')
        assert row['playerCompetition']['peerMatchesRin']
        for role in ('rin','player'):assert all(math.isfinite(x) for x in row['actors'][role]['position'])
        for motion in player+rin:
            source=clips[motion['id']]
            assert all(source[key]==motion[key] for key in ('bank','offset','size','sha256','frames'))
            if motion['id'] in checked:continue
            with Path(banks[source['bank']]['path']).open('rb') as f:f.seek(source['offset']);data=f.read(source['size'])
            assert hashlib.sha256(data).hexdigest()==source['sha256'];checked.add(motion['id'])
        count+=1
    assert count>0
    print('NATIVE_ROLE_CONTENT_AUDIT PASSED observations='+str(count)+' originalMembers='+str(len(checked))+'; recorded concurrent snapshots only, not a new live/rendered validation')
if __name__=='__main__':main()
