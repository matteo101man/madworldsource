"""Stage the owned EV21 writing-character record and its original IDT bank."""
from pathlib import Path
import hashlib,json,struct,sys
ROOT=Path(__file__).resolve().parents[2]
EXPORT=Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0,str(EXPORT/'builders'))
from native import dat_entries


def main():
    path=EXPORT/'extracted/disc/DATA/files/etc/core2.dat';raw=path.read_bytes()
    member=next(e for e in dat_entries(raw)if e['name']=='core2.wcd')
    wcd=raw[member['offset']:member['offset']+member['size']]
    assert wcd[:4]==b'WCD\0'
    count,base=struct.unpack_from('>II',wcd,8);assert base==16 and base+count*100==len(wcd)
    event=json.loads((ROOT/'assets/native/events/ev21/event.json').read_text())
    commands=[c for r in event['resources']if r['tag']=='ID  'for c in r['commands']if c['opcode']==16]
    assert len(commands)==1 and commands[0]['operands'][0]==0
    cue=commands[0];id=cue['operands'][1];record=wcd[base+id*100:base+(id+1)*100]
    assert struct.unpack_from('>H',record)[0]==id
    idd_path=EXPORT/'extracted/disc/DATA/files/subscr/wc.idd';idd=idd_path.read_bytes()
    sections={struct.unpack_from('>I',idd,8+i*8)[0]:struct.unpack_from('>I',idd,12+i*8)[0]for i in range(4)}
    assert idd[:4]==b'IDD\0'and sections[0]==sections[1]==0 and sections[2]<sections[3]<=len(idd)
    idt=idd[sections[2]:sections[3]];assert idt[:4]==b'IDT\0'
    folder=ROOT/'assets/native/events/ev21/writing';folder.mkdir(parents=True,exist_ok=True)
    (folder/'animation.idt').write_bytes(idt);(folder/'record.wcd').write_bytes(record)
    manifest={'schema':'mw-event-writing/1','event':event['event'],'eventSourceSha256':event['sourceSha256'],
        'source':{'path':'etc/core2.dat','sha256':hashlib.sha256(raw).hexdigest(),'member':member,
                  'wcdSha256':hashlib.sha256(wcd).hexdigest(),'recordOffset':base+id*100,'recordSha256':hashlib.sha256(record).hexdigest()},
        'animationSource':{'path':'subscr/wc.idd','sha256':hashlib.sha256(idd).hexdigest(),'offset':sections[2],'bytes':len(idt),'idtSha256':hashlib.sha256(idt).hexdigest()},
        'frame':cue['frame'],'id':id,'record':'native/events/ev21/writing/record.wcd',
        'animation':'native/events/ev21/writing/animation.idt','sharedAtlas':'core_us',
        'scope':'Exact100-byte WCD record and original animation-only WC IDT; original shared TUV/TEX required.'}
    (folder/'writing.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(manifest,indent=2))


if __name__=='__main__':main()
