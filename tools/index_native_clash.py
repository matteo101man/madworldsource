"""Audit native clash phases, source role contents and return after PC input."""
from pathlib import Path
import argparse,hashlib,json
from save_typed_graph import save_graph
from index_ui_layout import ROOT

def active(row):
    c=row.get('playerCompetition',{});flags=int(c.get('flags1124','0'),16)
    return c.get('peerMatchesRin',False) and (c.get('stateByte11b')==78 or flags&0x1000000!=0)
def names(row,role):return {m['name'].lower() for m in row.get(role+'NativeMotion',{}).get('resourceMatches',[]) if m['bank']=='em/em0440.dat'}
def main():
    ap=argparse.ArgumentParser();ap.add_argument('label');ap.add_argument('--require-success',action='store_true');a=ap.parse_args()
    if not a.label.replace('-','').isalnum():raise ValueError('Replay label')
    path=ROOT/'evidence'/f'{a.label}-feedback-observations.jsonl';rows=[json.loads(s) for s in path.read_text(encoding='utf-8').splitlines()];log=(ROOT/'evidence'/f'{a.label}.log').read_text(encoding='utf-8',errors='replace');run=json.loads((ROOT/'evidence'/f'{a.label}.json').read_text(encoding='utf-8'))
    fixture=[s for s in log.splitlines() if s.startswith('CLASH_CONTACT_FIXTURE ')];fixture_accepted=any('result accepted=1 ' in s for s in fixture)
    episodes=[];current=[]
    for index,row in enumerate(rows):
        if active(row):current.append(index)
        elif current:episodes.append(current);current=[]
    if current:episodes.append(current)
    results=[];graph=[dict(type='node',id='clash:run:'+a.label,kind='native-clash-run',fixtureRequested=run.get('clashContactFixtureRequested',False),fixtureAccepted=fixture_accepted,fixtureTrace=fixture,scope='Concurrent read-only original native state and full source motion matches; fixture setup is not natural collision proof')]
    for number,indices in enumerate(episodes):
        observed=[rows[i] for i in indices];success_rows=[v for v in observed if 'pl0030_04ac.mot' in names(v,'player') and 'em0440_0112.mot' in names(v,'rin')];failure_rows=[v for v in observed if 'pl0030_04ad.mot' in names(v,'player') or 'em0440_0113.mot' in names(v,'rin')]
        later=rows[indices[-1]+1:];returned=next((v for v in later if not active(v) and v.get('actors',{}).get('player',{}).get('stateBytes11a11d',[0,999])[1] in (0,4) and v.get('actors',{}).get('player',{}).get('hp',0)>0),None)
        phases=sorted({v.get('playerCompetition',{}).get('phaseByte11c') for v in observed if v.get('playerCompetition',{}).get('phaseByte11c') is not None})
        held=[v for v in observed if len((v.get('pcPhase')or{}).get('packet',[]))==8 and ((v['pcPhase']['packet'][6]&15)==9) and ((v['pcPhase']['packet'][7]&15)==9)]
        meter=[v['playerCompetition'] for v in observed if v['playerCompetition'].get('phaseByte11c')==3]
        meter_ranges={key:dict(minimum=min(v[key] for v in meter),maximum=max(v[key] for v in meter)) for key in ('float1240','float1248','float27dc')} if meter else {}

        info=dict(index=number,start=observed[0]['sampleSeconds'],last=observed[-1]['sampleSeconds'],samples=len(observed),heldBothArmPacketSamples=len(held),nativeMeterStageSamples=len(meter),nativeMeterRanges=meter_ranges,playerPhases=phases,successPairSamples=len(success_rows),failurePairSamples=len(failure_rows),playerSourceMotions=sorted({n for v in observed for n in names(v,'player')}),rinSourceMotions=sorted({n for v in observed for n in names(v,'rin')}),returnedOutsideCompetitionToIdleOrWalk=returned is not None,returnSeconds=returned['sampleSeconds'] if returned else None,playerHpAtStart=observed[0]['actors']['player']['hp'],playerHpAtReturn=returned['actors']['player']['hp'] if returned else None,rinHpAtStart=observed[0]['actors']['rin']['hp'],rinHpAtReturn=returned['actors']['rin']['hp'] if returned else None,observedSuccessfulCompletion=bool(held) and bool(success_rows) and bool({6,7}&set(phases)) and not failure_rows and returned is not None)
        results.append(info);nid=f'clash:{a.label}:{number}';graph.append(dict(type='node',id=nid,kind='native-clash-episode',**info));graph.append(dict(type='edge',source='clash:run:'+a.label,target=nid,kind='contains'))
    summary=dict(scope='Native phase/source-role progression and observed ordinary-state return; no visual/timing/full-fight or natural-contact proof when fixture used',source=str(path),sourceSha256=hashlib.sha256(path.read_bytes()).hexdigest(),fixtureRequested=run.get('clashContactFixtureRequested',False),fixtureAccepted=fixture_accepted,fixtureTrace=fixture,episodes=results,successfulEpisodes=sum(e['observedSuccessfulCompletion'] for e in results),gpu=run.get('gpu'),runtimeExitCode=run.get('exitCode'))
    (ROOT/'evidence'/f'{a.label}-clash-summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8');save_graph(ROOT/'research'/f'{a.label}-clash.graph.jsonl',graph,summary['scope'])
    print(json.dumps(dict(label=a.label,fixtureAccepted=fixture_accepted,episodes=len(results),successfulEpisodes=summary['successfulEpisodes'])))
    if a.require_success and (summary['successfulEpisodes']==0 or summary['runtimeExitCode']!=0 or (summary['fixtureRequested'] and not fixture_accepted)):raise SystemExit(1)
if __name__=='__main__':main()
