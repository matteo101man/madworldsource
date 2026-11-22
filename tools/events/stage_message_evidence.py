"""Stage four valid original message component snapshots; reject empty captures."""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[2]
source=ROOT/'evidence/event-message-v5.jsonl'
rows=[json.loads(line)for line in source.read_text().splitlines()]
assert rows[0]['schema']=='mw-event-message-fixture/1'
assert len(rows)==53 and [r['row']for r in rows[1:]]==list(range(52))
selected=[]
for index,count in enumerate([65,62,41,41]):
    row=next(r for r in rows[1:]if r['row']==index*12+11)
    assert row['message']==index and len(row['glyphs'])==count
    assert all(g['hidden']==0 for g in row['glyphs'])
    selected.append(row)
assert all(not r['glyphs']for r in rows[1:]if r['row']>=48)
result=dict(schema='mw-event-message-observed/1',sourceSha256=hashlib.sha256(source.read_bytes()).hexdigest(),catalogSha256=hashlib.sha256((ROOT/'assets/native/events/ev00/messages/catalog.json').read_bytes()).hexdigest(),scope=rows[0]['scope'],captures=selected)
(ROOT/'research/event-message-observed.json').write_text(json.dumps(result,separators=(',',':'))+'\n')
print(json.dumps(dict(captures=len(selected),glyphs=sum(len(r['glyphs'])for r in selected))))
