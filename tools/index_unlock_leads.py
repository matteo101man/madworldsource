"""Index original room-unlock string references as leads, not inferred logic."""
from pathlib import Path
import sys,json,bisect
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'tools/upstream-native-core/port/recomp'))
from dol import Dol
from ppc_decode import decode
from recomp import load_functions

def main():
    dol=Dol(Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol'))
    targets={}
    prefix='アンロック'.encode('cp932')
    for section in dol.data_sections():
        offset=0
        while (offset:=section.data.find(prefix,offset))>=0:
            end=section.data.find(b'\0',offset,offset+100)
            if end>=0:targets[section.addr+offset]=section.data[offset:end].decode('cp932')
            offset+=len(prefix)
    parent=json.loads((ROOT/'research/unlock-parent-key-table.json').read_text(encoding='utf-8'))
    for record in parent['records']:targets[int(record['site'],16)]='pointer-table entry: '+record['label']
    functions=load_functions(ROOT/'tools/upstream-native-core/config/RZZE8P/symbols.txt')
    rows=[]
    descriptor_refs=[{'site':r['site'],'label':r['label']} for r in parent['records']]
    table_sites={int(row['site'],16):row['label'] for row in descriptor_refs}
    by_low={site&65535:site for site in table_sites}

    for function in functions:
        registers={2:0x80720720,13:0x8071f720};highs=set()
        for pc in range(function.addr,function.end,4):
            word=dol.read32(pc)
            if word is None:continue
            instruction=decode(word);op,F=instruction.op,instruction.f
            result=None
            if op=='addis' and F['ra']==0:highs.add(F['simm']&65535)
            if op=='lwz' and (site:=by_low.get(F['d']&65535)) is not None and ((site+32768)>>16) in highs:
                rows.append({'function':f'{function.addr:08x}','name':function.name,'site':f'{pc:08x}',
                             'tableSite':f'{site:08x}','string':table_sites[site],
                             'authority':'Indexed-table displacement lead; register/index flow not proven'})

            if op in ('addi','addis') and (F['ra']==0 or F['ra'] in registers):
                result=((registers.get(F['ra'],0) if F['ra'] else 0)+(F['simm']<<(16 if op=='addis' else 0)))&0xffffffff
                registers[F['rd']]=result
            elif op=='lwz' and F['ra'] in registers:
                address=(registers[F['ra']]+F['d'])&0xffffffff
                result=dol.read32(address)
                if result is not None:registers[F['rd']]=result
                else:registers.pop(F['rd'],None)
            elif op=='ori'  and F['rs'] in registers:
                result=registers[F['rs']]|F['uimm'];registers[F['ra']]=result
            elif op=='b' and F['lk']:
                registers={r:v for r,v in registers.items() if r>=14 or r in (2,13)}
            elif op in ('b','bc','bclr','bcctr'):
                # Preserve only ABI bases at an unknown flow join.
                registers={r:v for r,v in registers.items() if r in (2,13)}
            elif 'rd' in F:registers.pop(F['rd'],None)
            elif 'ra' in F:registers.pop(F['ra'],None)
            if result in targets:rows.append({'function':f'{function.addr:08x}','name':function.name,'site':f'{pc:08x}','stringAddress':f'{result:08x}','string':targets[result]})
    graph=[]
    for address,label in targets.items():graph.append({'n':{'id':f'string:{address:08x}','kind':'native-config-key','label':label}})
    for row in rows:
        graph.append({'n':{'id':'fn:'+row['function'],'kind':'native-function','label':row['name']}})
        graph.append({'e':{'src':'fn:'+row['function'],'dst':'string:'+row.get('stringAddress',row.get('tableSite','unknown')),'kind':'references-lead','attrs':row}})
    (ROOT/'research/unlock-leads.graph.jsonl').write_text('\n'.join(json.dumps(r,ensure_ascii=False) for r in graph)+'\n',encoding='utf-8')
    (ROOT/'research/unlock-leads.json').write_text(json.dumps({'authority':'Straight-line register constants with EABI nonvolatile preservation across direct calls; consumer/branch eligibility require focused Pcode/runtime proof','strings':{hex(k):v for k,v in targets.items()},'leads':rows},indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
    print(json.dumps({'strings':len(targets),'leads':rows},ensure_ascii=True))

if __name__=='__main__':main()
