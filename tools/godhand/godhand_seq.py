"""Decode the SEQ records next to a God Hand motion (hit windows: flag bits 1/2; speed multipliers) via the explorer's own
builders (godhand_formats.pack_entries, native_clock.Sequence).
Usage: python tools/godhand/godhand_seq.py pl00:59 pl00:402
"""
import json,sys
from pathlib import Path
R=Path(r'D:/Games/gamesfordecomp/exports/godhand')
sys.path.insert(0,str(R/'builders'))
from godhand_formats import pack_entries
from native_clock import Sequence
E=json.load(open(R/'explorer.json'))
B={b['id']:b for b in E['motion_banks']};M={m['id']:m for m in E['motions']}
def seqinfo(mid,delta=1):
    m=M[mid];raw=Path(B[m['bank']]['source']).read_bytes();es={s:(t,o,z) for s,t,o,z in pack_entries(raw)}
    t,o,z=es[m['slot']];assert o==m['offset'],(o,m['offset'])
    out={}
    for d in (1,2):
        e=es.get(m['slot']+d)
        if e and e[0]=='SEQ':
            s=Sequence(raw[e[1]:e[1]+e[2]]);out[m['slot']+d]=dict(flags=s.flags,speed=s.speed)
    return out
if __name__=='__main__':
    for mid in sys.argv[1:]:print(mid,M[mid]['frames'],seqinfo(mid))
