"""Decode selected owned UI TPL resources, without altering their pixels."""
from pathlib import Path
import hashlib,json,sys
ROOT=Path(__file__).resolve().parents[1]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPORT/'builders'))
from native import tpl
def main():
    output=ROOT/'research/ui-source';output.mkdir(exist_ok=True);rows=[]
    def decode(raw,stem,source):
        offset=raw.find(bytes.fromhex('0020af30'))
        if offset<0:return
        for i,(image,meta) in enumerate(tpl(raw[offset:])):
            dest=output/f'{stem}-{i}.png';image.save(dest)
            rows.append(dict(source=source,sourceSha256=hashlib.sha256(raw).hexdigest(),tplOffset=offset,texture=i,metadata=meta,decodedImage=str(dest)))
    source=EXPORT/'extracted/disc/DATA/files/subscr'
    for name in ('nl.idd','subscr0fe.idd'):
        decode((source/name).read_bytes(),Path(name).stem,dict(path=str(source/name)))
    catalog=json.loads((EXPORT/'catalog.json').read_text(encoding='utf-8'))
    for bank in catalog['banks']:
        if bank['id']!='etc/core2_us.dat':continue
        with Path(bank['path']).open('rb') as f:
            for member in bank['entries']:
                if member['name']!='core.idd':continue
                f.seek(member['offset']);decode(f.read(member['size']),'core-us',dict(bank=bank['id'],member=member['name'],offset=member['offset'],size=member['size']))
    (ROOT/'research/ui-common-and-core-source-index.json').write_text(json.dumps({'scope':'Owned resource decoding only; no modified atlas or live UI replacement claim','resources':rows},indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'decodedTextures':len(rows),'output':str(output)}))
if __name__=='__main__':main()
