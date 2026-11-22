"""Stage isolated original800C5508 arithmetic observations, not game/HUD parity."""
from pathlib import Path
import hashlib,json,collections,struct
from index_native_idt import index
ROOT=Path(__file__).resolve().parents[2]

def main():
    p=ROOT/'evidence/hud-gauge-v1.jsonl';raw=p.read_bytes();rows=[json.loads(l)for l in raw.splitlines()];header=rows.pop(0)
    assert header['schema']=='mw-hud-gauge-fixture/1' and len(rows)==400
    for c in range(4):assert [r['step']for r in rows if r['case']==c]==list(range(100))
    observed=dict(schema='mw-hud-gauge-observed/1',source=p.relative_to(ROOT).as_posix(),sha256=hashlib.sha256(raw).hexdigest(),scope=header['scope'],rows=rows)
    (ROOT/'research/ui-hud-gauge-observed.json').write_text(json.dumps(observed,separators=(',',':'))+'\n')
    resource=next(r for r in index()['resources']if r['id']=='core_us');source=(ROOT/'assets/native/ui/core_us/animation.idt').read_bytes();captures=[]
    for sample in [55,80]:
        p=ROOT/f'evidence/hud-live-v1-{sample}.json';raw=p.read_bytes();snapshot=json.loads(raw)
        assert snapshot['demoCounter']==0 and snapshot['gameFlags']['word0']=='0x0' and snapshot['uiWidgets']['reachedEnd']
        for pid,name in [(31,'cIdBossHp'),(39,'cIdHpGage')]:
            program=next(p for p in resource['programs']if p['id']==pid)
            candidates=collections.Counter(int(w['sourceRecord'],16)-o['offset']for w in snapshot['uiWidgets']['widgets']for o in program['widgets']if w['sourceHeaderSha256']==o['headerSha256'])
            ranked=candidates.most_common();base,hits=ranked[0];assert hits>len(program['widgets'])//2 and (len(ranked)==1 or hits>ranked[1][1])
            by_offset={w['offset']:w for w in program['widgets']};bindings=[]
            for w in snapshot['uiWidgets']['widgets']:
                original=by_offset.get(int(w['sourceRecord'],16)-base)
                if not original:continue
                assert (w['id'],w['kind'])==(original['id'],original['kind'])
                header=bytes.fromhex(w['sourceHeaderHex']);at=original['offset']
                different=[i for i in range(0,176,4)if source[at+i:at+i+4]!=header[i:i+4]]
                #800C5224 writes cropped primary UV bounds back to the kind8 source.
                assert not different or (w['kind']==8 and set(different)<={0x74,0x78,0x80,0x84})
                fields={k:w[k]for k in ['id','kind','hidden','rgba','size','matrix1f8','preparedVertices','channelFrames']}
                if 'gauge'in w:
                    fields['gauge']=w['gauge'];fields['gaugeUvBounds']=[struct.unpack_from('>f',header,o)[0]for o in (0x74,0x78,0x80,0x84)]
                bindings.append(dict(index=original['index'],observed=fields))
            controller=next(c for c in snapshot['uiControllers']['controllers']if c['typeName']==name)
            captures.append(dict(sample=sample,snapshot=p.relative_to(ROOT).as_posix(),sha256=hashlib.sha256(raw).hexdigest(),program=pid,recordBase=hex(base),exactHeaders=hits,missingIndices=sorted(set(range(len(program['widgets'])))-{b['index']for b in bindings}),controller=controller,bindings=bindings))
    (ROOT/'research/ui-hud-observed.json').write_text(json.dumps(dict(schema='mw-hud-observed/1',sourceSha256=resource['sha256'],captures=captures),indent=1)+'\n')
    print(json.dumps(dict(calls=len(rows),scope=observed['scope'],sha256=observed['sha256'])))

if __name__=='__main__':main()
