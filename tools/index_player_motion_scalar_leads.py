"""Bounded PPC store-offset leads for original player motion-filter producers."""
from pathlib import Path
import sys,json,re
import capstone
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools/upstream-native-core/port/recomp'))
from dol import Dol
sys.path.insert(0,'C:/Users/Matteo/Downloads/GDecomp/universalexplorer')
from gk.graph import Graph
def main():
    dol=Dol(Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol'))
    cs=capstone.Cs(capstone.CS_ARCH_PPC,capstone.CS_MODE_32|capstone.CS_MODE_BIG_ENDIAN);cs.skipdata=True
    graph=Graph(dict(game='MadWorld',authority='PPC immediate store offsets in bounded player code region; register identity and aliases unresolved, not proven field consumers'))
    wanted={0xf60,0xf64,0xf68,0xf6c};rows=[]
    for line in (ROOT/'tools/ghidra-functions.tsv').read_text(encoding='utf-8').splitlines():
        address,size,name=line.split('\t');address=int(address,16);size=int(size,16)
        if not 0x80270000<=address<0x802d0000:continue
        data=b''.join(dol.read32(p).to_bytes(4,'big') for p in range(address,address+size,4))
        for instruction in cs.disasm(data,address):
            if not instruction.mnemonic.startswith(('stf','stw','sth','stb')):continue
            found=re.search(r',\s*(-?(?:0x[0-9a-f]+|[0-9]+))\((r[0-9]+)\)$',instruction.op_str)
            if not found:continue
            offset=int(found[1],0)
            if offset not in wanted:continue
            row={'function':name,'address':hex(address),'site':hex(instruction.address),'offset':hex(offset),'baseRegister':found[2],'storeKind':instruction.mnemonic};rows.append(row)
            fid='fn:'+hex(address);oid='offset:'+hex(offset);graph.add_node(fid,'native-function',name,address=address);graph.add_node(oid,'field-offset-lead',hex(offset),offset=offset);graph.add_edge(fid,oid,'writes-offset-lead',site=instruction.address,baseRegister=found[2],storeKind=instruction.mnemonic)
    graph.save(ROOT/'research/player-motion-scalar-leads.graph.jsonl')
    (ROOT/'research/player-motion-scalar-leads.json').write_text(json.dumps(rows,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'leads':len(rows),'functions':sorted({r['address'] for r in rows})}))
if __name__=='__main__':main()
