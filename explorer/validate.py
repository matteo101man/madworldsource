"""Validate native structure over local resources, never ship the source bytes."""
import argparse
from collections import Counter
import json
import math
from pathlib import Path
import time
from native import mdb, motion


def validate(root):
    root=Path(root); catalog=json.loads((root/'catalog.json').read_text(encoding='utf-8'))
    banks={b['id']:b for b in catalog['banks']};results=[];started=time.perf_counter()
    for bank in catalog['banks']:
        models=[m for m in catalog['models'] if m['bank']==bank['id']]
        clips=[m for m in catalog['clips'] if m['bank']==bank['id']]
        if not models and not clips:continue
        raw=Path(bank['path']).read_bytes()
        for item in models+clips:
            try:
                data=raw[item['offset']:item['offset']+item['size']]
                if item in models:
                    decoded=mdb(data); sums=[sum(w) for m in decoded['meshes'] for w in m['weights']]
                    if sums and max(abs(w-1) for w in sums)>1e-6:raise ValueError(f'weight sums {min(sums)}..{max(sums)}')
                    for i,b in enumerate(decoded['bones']):
                        seen={i};p=b['parent']
                        while p>=0:
                            if p in seen:raise ValueError('bone cycle')
                            seen.add(p);p=decoded['bones'][p]['parent']
                        expected=[b['relative'][k]+(decoded['bones'][b['parent']]['position'][k] if b['parent']>=0 else 0) for k in range(3)]
                        if max(abs(expected[k]-b['position'][k]) for k in range(3))>1e-4:raise ValueError('relative/absolute bind mismatch')
                    for m in decoded['meshes']:
                        if any(not math.isfinite(v) for p in m['positions']+m['normals']+m['uvs'] for v in p):raise ValueError('nonfinite vertex')
                    result=dict(id=item['id'],kind='model',ok=True,bones=len(decoded['bones']),meshes=len(decoded['meshes']))
                else:
                    decoded=motion(data);result=dict(id=item['id'],kind='motion',ok=True,frames=decoded['frames'],tracks=len(decoded['tracks']))
            except Exception as e:result=dict(id=item['id'],kind='model' if item in models else 'motion',ok=False,error=str(e))
            results.append(result)
    report=dict(seconds=time.perf_counter()-started,total=len(results),passed=sum(x['ok'] for x in results),failures=[x for x in results if not x['ok']],results=results)
    (root/'evidence/structural-validation.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps({k:v for k,v in report.items() if k not in ('results','failures')},indent=2));print(Counter(x['error'] for x in report['failures']))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root');a=p.parse_args();validate(a.root)
