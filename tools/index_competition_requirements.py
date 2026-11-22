"""Index bounded original clash rules and the live held-PC-input probe."""
from pathlib import Path
import hashlib,json,statistics,sys
from save_typed_graph import save_graph
from index_ui_layout import ROOT
from query_boot import summarize
sys.path.insert(0,str(ROOT/'tools/upstream-native-core/port/recomp'))
from dol import Dol

def main():
    functions=[]
    for address,window in [('81006400',None),('810cb518',None),('810cb548',None),('810cb1b4',None),('81064e54',None),('8021639c',None),('802727b0',(0x802727f0,0x80272860))]:
        functions.append(summarize(ROOT,address,window,'competition-eligibility-pcode.graph.jsonl'))
    (ROOT/'research/competition-eligibility-pseudocode.json').write_text(json.dumps(functions,indent=2)+'\n',encoding='utf-8')
    rows=[dict(type='node',id='clash:rin-eligibility',kind='native-predicate',function='810cb518',flagHelper='81006400',wordOffset=0x864,bitIndex=0,predicate='(Rin.word864 & 1) != 0'),dict(type='node',id='clash:request',kind='native-api',function='810cb548',target='8028bde8',mode=11,effect='original startKeenCompetition(player,Rin,11); no direct Rin state assignment here'),dict(type='node',id='clash:scalar-input',kind='native-input-transform',function='802727b0',remoteVectorOffset=0xef0,nunchukVectorOffset=0xefc,outputOffsets=[0xf60,0xf64],expression='abs(length(accelerationVector)-1)',authority='PSVECDotProduct/sqrt call sequence and original stores; decompiler omits implicit FP call arguments'),dict(type='node',id='clash:ring',kind='native-input-filter',function='802c6c0c',length=8,scalarScale=0.125,outputOffsets=[0x1088,0x108c],cadence='not independently established'),dict(type='node',id='clash:meter',kind='native-meter-rule',function='802956b4',phaseByte11c=3,cooldownWord=0x1230,gain='-0.08 + 0.04*(remote1088+nunchuk108C) when cooldown==0 and both>0.5; otherwise -0.08',limits='original clamp/smoothing/phase-dependent goal logic retained'),dict(type='node',id='clash:direction',kind='native-input-predicate',function='802956b4',phaseByte11c=5,remote1090GreaterThan=1.4,nunchuk109cGreaterThan=1.1,delayCounterWord=0x1238,timeoutCounterWord=0x123c),dict(type='edge',source='clash:scalar-input',target='clash:ring',kind='feeds'),dict(type='edge',source='clash:ring',target='clash:meter',kind='feeds'),dict(type='edge',source='clash:rin-eligibility',target='clash:request',kind='gates')]
    path=ROOT/'evidence/bevy-native-motion-probe-feedback-observations.jsonl';observations=[json.loads(s) for s in path.read_text(encoding='utf-8').splitlines()]
    held=[v for v in observations if (v.get('pcPhase')or{}).get('phase')=='held-struggle-probe'];stats={}
    for offset in ('0xf60','0xf64','0x1088','0x108c','0x1090','0x109c'):
        values=[v['playerMotionMetrics'][offset] for v in held if offset in v.get('playerMotionMetrics',{})]
        if not values:raise ValueError('Missing held motion observations '+offset)
        stats[offset]=dict(count=len(values),minimum=min(values),mean=statistics.mean(values),median=statistics.median(values),maximum=max(values))
    gains=[.04*(v['playerMotionMetrics']['0x1088']+v['playerMotionMetrics']['0x108c'])-.08 if v['playerMotionMetrics']['0x1088']>.5 and v['playerMotionMetrics']['0x108c']>.5 else -.08 for v in held]
    directional=sum(v['playerMotionMetrics']['0x1090']>1.4 and v['playerMotionMetrics']['0x109c']>1.1 for v in held)
    released=[v for v in observations if (v.get('pcPhase')or{}).get('phase')=='released']
    settled={offset:max(abs(v.get('playerMotionMetrics',{}).get(offset,0)) for v in released) for offset in ('0x1088','0x108c','0x1090','0x109c')} if released else {}
    result=dict(scope='Native PC input/filtered-value observations plus bounded source comparisons; not a competition-completion or frame-atomic meter trace',observationsSha256=hashlib.sha256(path.read_bytes()).hexdigest(),heldSamples=len(held),releasedSamples=len(released),settledReleasedAbsoluteMaxima=settled,metrics=stats,conditionalStage3GainEstimate=dict(mean=statistics.mean(gains),positiveSnapshots=sum(v>0 for v in gains),assumptions='stage3/cooldown0, before clamp/smoothing; free-control probe did not execute this meter'),directionalThresholdSnapshots=directional,requiredDirectionalThresholds=dict(remote1090=1.4,nunchuk109c=1.1),runtime='evidence/bevy-native-motion-probe.json')
    (ROOT/'evidence/native-pc-motion-probe-summary.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    rows.append(dict(type='node',id='clash:held-input-probe',kind='runtime-observation',artifact='evidence/native-pc-motion-probe-summary.json',heldSamples=len(held),scope=result['scope']));rows.append(dict(type='edge',source='clash:held-input-probe',target='clash:meter',kind='compared-with-source-rule'))
    save_graph(ROOT/'research/clash-input-requirements.graph.jsonl',rows,'Bounded original predicates/input math and concurrent PC input probe; no completion claim')
    print(json.dumps(dict(heldSamples=len(held),conditionalGainMean=statistics.mean(gains),directionalThresholdSnapshots=directional)))
if __name__=='__main__':main()
