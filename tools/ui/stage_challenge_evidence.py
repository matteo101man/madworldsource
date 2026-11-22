"""Preserve bounded owned-core challenge traces as a Rust regression fixture.
Run cargo test -p mw-sim --lib ui_challenge for the actual native/Rust comparison.
"""
from pathlib import Path
import hashlib,json
from index_native_idt import index
ROOT=Path(__file__).resolve().parents[2]


def main():
    resource=next(r for r in index()['resources']if r['id']=='core_us')
    program=next(p for p in resource['programs']if p['id']==42)
    offsets={w['offset']:w['id']for w in program['widgets']};assert len(offsets)==40
    captures=[];summaries=[]
    for choice in ['accept','decline']:
        p=ROOT/f'evidence/challenge-{choice}-v1.jsonl';raw=p.read_bytes();rows=[json.loads(l)for l in raw.splitlines()]
        assert rows.pop(0)['schema']=='mw-challenge-record/1'
        assert rows[-1]['closed'] and rows[0]['row']==0
        changes=[];last=None;visible=0
        for r in rows:
            if 'phase'not in r:continue
            assert r['active']==1 and r['program']==42 and r['boss']==3 and r['ranking']==0
            if last!=r['phase']:changes.append([r['row'],r['phase'],r['yes'],r['result']]);last=r['phase']
            assert not r['widgets']or len(r['widgets'])==40
            for w in r['widgets']:
                assert offsets[w['offset']]==w['id']
                if not w['hidden'] and w['rgba'][3]>0 and all(w['size']):visible+=1
                for key in ['flags','bindings','clocks']:w.pop(key,None)
            r['glyphCount']=len(r['glyphs'])
            if r['row']!=25:r.pop('glyphs')
        assert rows[-2]['result']==(choice=='accept')
        assert rows[25]['glyphCount']==48
        digest=hashlib.sha256(raw).hexdigest()
        captures.append(dict(choice=choice,sha256=digest,rows=rows))
        summaries.append(dict(choice=choice,path=p.relative_to(ROOT).as_posix(),sha256=digest,rows=len(rows),phaseChanges=changes,closedRow=rows[-1]['row'],nondegenerateQuads=visible))
    p=ROOT/'evidence/challenge-window-v5.jsonl';raw=p.read_bytes()
    observation=next(r for r in map(json.loads,raw.splitlines())if r.get('phase')==1)
    window=observation['windows'][0]
    assert window['message']==0x12010008 and window['type']==5 and window['flags']&0x30==0x30
    assert window['singleRow']==0 and window['anchor']==[0,20]
    assert all(abs(f-20/24)<1e-6 for f in window['factors']) and abs(window['lineSpacing']-20*20/24)<1e-5
    observed_window=dict(schema='mw-challenge-window-observed/1',source=p.relative_to(ROOT).as_posix(),sha256=hashlib.sha256(raw).hexdigest(),window=window)
    (ROOT/'research/ui-challenge-window-observed.json').write_text(json.dumps(observed_window,indent=2)+'\n')
    fixture=dict(schema='mw-challenge-observed/1',captures=captures)
    (ROOT/'research/ui-challenge-observed.json').write_text(json.dumps(fixture,separators=(',',':'))+'\n')
    report=dict(schema='mw-challenge-evidence/1',source=dict(path=resource['path'],sha256=resource['sha256'],program=42,widgets=40),captures=summaries,
                messageWindow=observed_window,verification='cargo test -p mw-sim --lib ui_challenge; compares every controller edge and native-visible nondegenerate widget position/size/RGBA/UV, message lifetime, all48 metric glyphs and UV bounds. No pixel or whole-world equivalence claim.')
    (ROOT/'research/CHALLENGE-EVIDENCE.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))


if __name__=='__main__':main()
