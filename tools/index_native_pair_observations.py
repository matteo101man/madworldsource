"""Index simultaneous read-only owned native actor/motion observations as a graph."""
from pathlib import Path
import argparse,json,sys
from collections import Counter
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph
ROOT=Path(__file__).resolve().parents[1]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('label');args=ap.parse_args();assert args.label.replace('-','').isalnum()
    source=ROOT/'evidence'/f'{args.label}-feedback-observations.jsonl'
    rows=[json.loads(line) for line in source.read_text(encoding='utf-8').splitlines()]
    run_path=ROOT/'evidence'/f'{args.label}.json';run=json.loads(run_path.read_text(encoding='utf-8')) if run_path.exists() else {};fixture=run.get('clashContactFixtureRequested',False)
    graph=Graph(dict(game='MadWorld',authority='Concurrent read-only original native process; both complete source MOT contents matched by SHA256. Not frame-atomic or visual equivalence.',fixtureRequested=fixture))
    observed=[];counts=Counter()
    for row in rows:
        player=[m for m in row.get('playerNativeMotion',{}).get('resourceMatches',[]) if m['bank']=='em/em0440.dat']
        rin=row.get('rinNativeMotion',{}).get('resourceMatches',[])
        if not player or not rin:continue
        number=len(observed);nid=f'observation:{number}'
        info=dict(sampleSeconds=row['sampleSeconds'],playerMotionFrame=row['playerNativeMotion']['frame'],rinMotionFrame=row['rinNativeMotion']['frame'],actors=row['actors'],competition=row.get('playerCompetition'),navigation=row.get('nativeSwingNavigation'),metrics=row.get('playerMotionMetrics'),pcPhase=row.get('pcPhase'))
        graph.add_node(nid,'native-pair-observation',str(number),**info)
        for role,matches in (('player',player),('rin',rin)):
            actor=row['actors'][role];aid=role+':'+actor['object'];graph.add_node(aid,'native-actor',role,nativeId=actor['id'],object=actor['object']);graph.add_edge(nid,aid,'observes-role',role=role)
            for motion in matches:
                mid='motion:'+motion['id'];graph.add_node(mid,'native-motion',motion['name'],**motion);graph.add_edge(nid,mid,'observes-motion',role=role,fullContentMatched=True)
        for a in player:
            for b in rin:counts[(a['id'],b['id'])]+=1
        observed.append(info)
    name=args.label+'-paired'
    graph.save(ROOT/'research'/f'{name}.graph.jsonl')
    summary={'source':str(source),'fixtureRequested':fixture,'observations':len(observed),'pairs':[{'playerMotion':a,'rinMotion':b,'samples':count} for (a,b),count in counts.items()],'scope':'Simultaneous role/content/state/position observations. No success/failure semantic labels, complete authored phase chain, contact, visual timing or win/loss equivalence claim.'}
    (ROOT/'evidence'/f'{name}-summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
    (ROOT/'research'/f'{name}-INDEX.md').write_text('# Observed native Rin/player pair\n\nSource: '+source.name+'\n\n'+str(len(observed))+' concurrent observations. Query '+name+'.graph.jsonl.\n\n`native-pair-observation observes-role native-actor`; `native-pair-observation observes-motion native-motion`, with explicit role and complete content-hash match. Frame fields and native positions are sampled concurrently; do not claim frame-atomic synchronization or visual parity.\n',encoding='utf-8')
    print(json.dumps(summary))
if __name__=='__main__':main()
