"""Index source MOT content for bounded read-only live identity observations."""
import hashlib,json
from pathlib import Path
from collections import defaultdict
ROOT=Path(__file__).resolve().parents[1]
def main():
    export=Path('D:/Games/gamesfordecomp/exports/madworld')
    catalog=json.loads((export/'catalog.json').read_text(encoding='utf-8'));banks={r['id']:r for r in catalog['banks']};group=defaultdict(list);index={}
    for row in catalog['clips']:group[row['bank']].append(row)
    for bank,rows in group.items():
        with Path(banks[bank]['path']).open('rb') as source:
            for row in rows:
                source.seek(row['offset']);data=source.read(row['size'])
                if hashlib.sha256(data).hexdigest()!=row['sha256']:raise ValueError('Source resource changed: '+row['id'])
                index.setdefault(hashlib.sha256(data[:64]).hexdigest(),[]).append({k:row[k] for k in ('id','bank','name','offset','size','sha256','frames')})
    output={'authority':'All original catalog MOT member bytes rehashed. Header64 hash selects candidates only; complete live content SHA256 required for a resource identity match. No authored ownership inferred.','resources':sum(map(len,index.values())),'headers':index}
    (ROOT/'research/native-motion-live-index.json').write_text(json.dumps(output,indent=2)+'\n',encoding='utf-8');print(json.dumps({'resources':output['resources'],'headerKeys':len(index)}))
if __name__=='__main__':main()
