"""Stage synthetic original HUD health-sequence evidence; no actor HP writes."""
from pathlib import Path
import hashlib,json
from index_native_idt import index
ROOT=Path(__file__).resolve().parents[2]

def main():
    p=ROOT/'evidence/hud-low-v3.jsonl';raw=p.read_bytes();rows=[json.loads(l)for l in raw.splitlines()];header=rows.pop(0)
    assert header['schema']=='mw-hud-low-fixture/1' and len(rows)==340
    resource=next(r for r in index()['resources']if r['id']=='core_us')
    programs={p['id']:p for p in resource['programs']if p['id']in [39,58]}
    offsets={w['offset']:(pid,w)for pid,p in programs.items()for w in p['widgets']}
    changes=[];previous=None
    for age,r in enumerate(rows):
        assert r['row']==age and len(r['widgets'])==16
        if r['low']!=previous:changes.append([age,r['low']]);previous=r['low']
        assert abs(r['target']-(rows[max(0,age-1)]['input']))<1e-6
        for w in r['widgets']:
            pid,source=offsets[w['offset']];assert (w['program'],w['id'])==(pid,source['id'])
            w['index']=source['index']
    assert changes==[[0,0],[72,1],[152,0],[202,1],[242,0],[262,1],[302,0]]
    observed=dict(schema='mw-hud-low-observed/1',source=p.relative_to(ROOT).as_posix(),sha256=hashlib.sha256(raw).hexdigest(),sourceIdtSha256=resource['sha256'],scope=header['scope'],transitions=changes,rows=rows)
    (ROOT/'research/ui-hud-low-observed.json').write_text(json.dumps(observed,separators=(',',':'))+'\n')
    print(json.dumps(dict(rows=len(rows),transitions=changes,scope=header['scope'])))

if __name__=='__main__':main()
