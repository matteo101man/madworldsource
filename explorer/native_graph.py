"""Bounded RSO symbol/call graph and conservative immediate-motion call evidence.

File layouts checked against Dolphin's public Core/Debugger/RSO.h.
No disassembly dumps are sent to the explorer or prompts.
"""
import argparse
from bisect import bisect_right
import hashlib
import json
from pathlib import Path
import re
import sys
import capstone
from native import Reader
sys.path.insert(0,str(Path(__file__).resolve().parents[2]))
from gk.graph import Graph


def rso(data):
    r=Reader(data);count,table=r.get('II',8)
    sections=[r.get('II',table+i*8) for i in range(count)]
    def string(p):
        end=data.find(b'\0',p)
        if end<0:raise ValueError('unterminated RSO name')
        return data[p:end].decode('ascii')
    ex,sz,names=r.get('III',64); exports=[]
    if sz%16:raise ValueError('RSO export stride')
    for p in range(ex,ex+sz,16):
        name,value,section,h=r.get('4I',p)
        if section>=count:raise ValueError('RSO export section')
        exports.append(dict(name=string(names+name),value=value,section=section,offset=sections[section][0]+value))
    im,sz,names=r.get('III',76);imports=[]
    if sz%12:raise ValueError('RSO import stride')
    for p in range(im,im+sz,12):
        name,_,_=r.get('3I',p);imports.append(string(names+name))
    ext,size=r.get('II',56);relocs={}
    for p in range(ext,ext+size,12):
        offset,info,addend=r.get('III',p);symbol=info>>8
        if symbol>=len(imports):raise ValueError('RSO import relocation index')
        relocs[offset]=dict(name=imports[symbol],type=info&255,addend=addend)
    internal,size=r.get('II',48);internals={}
    for p in range(internal,internal+size,12):
        offset,info,addend=r.get('III',p);section=info>>8
        if section>=len(sections):raise ValueError('RSO internal section')
        internals[offset]=dict(section=section,type=info&255,addend=addend,target=sections[section][0]+addend)
    return sections,exports,imports,relocs,internals


def build(root):
    root=Path(root);path=root/'extracted/disc/DATA/files/dll/Rel/enemyRSO.rso';raw=path.read_bytes()
    sections,exports,imports,relocs,internals=rso(raw)
    graph=Graph(dict(game='MadWorld',module=str(path),sha256=hashlib.sha256(raw).hexdigest(),authority='RSO exports, direct branch targets, external relocations; immediate call arguments only in straight-line blocks'))
    functions=sorted([s for s in exports if s['section']==1],key=lambda s:s['offset']);at={s['offset']:s['name'] for s in functions}
    for s in functions:graph.add_node('fn:'+s['name'],'function',s['name'],offset=s['offset'])
    cs=capstone.Cs(capstone.CS_ARCH_PPC,capstone.CS_MODE_32|capstone.CS_MODE_BIG_ENDIAN);cs.detail=True
    cs.skipdata=True
    rows=[];calls=[];descriptions=[]
    for i,s in enumerate(functions):
        end=functions[i+1]['offset'] if i+1<len(functions) else sections[1][0]+sections[1][1]
        regs={};body=[]
        for ins in cs.disasm(raw[s['offset']:end],s['offset']):
            parts=[p.strip() for p in ins.op_str.split(',')];m=ins.mnemonic
            if m=='.byte':
                regs.clear()
            elif m in ('li','lis') and len(parts)==2:
                try:regs[parts[0]]=int(parts[1],0)*(65536 if m=='lis' else 1)
                except ValueError:regs.pop(parts[0],None)
            elif m=='mr' and len(parts)==2:
                if parts[1] in regs:regs[parts[0]]=regs[parts[1]]
                else:regs.pop(parts[0],None)
            elif m in ('addi','ori') and len(parts)==3:
                try:
                    v=int(parts[2],0)
                    relocation=internals.get(ins.address+2)
                    if relocation and relocation['type']==4:regs[parts[0]]=relocation['target']
                    elif parts[1] in regs:regs[parts[0]]=(regs[parts[1]]+v if m=='addi' else regs[parts[1]]|v)&0xffffffff
                    else:regs.pop(parts[0],None)
                except ValueError:regs.pop(parts[0],None)
            elif m in ('bl','bla'):
                target=relocs.get(ins.address,{}).get('name')
                relocation=internals.get(ins.address)
                if relocation and relocation['type']==10:target=at.get(relocation['target'])
                if target is None:
                    try:target=at.get(int(ins.op_str,0))
                    except ValueError:pass
                if target:
                    graph.add_node('fn:'+target,'function',target)
                    graph.add_edge('fn:'+s['name'],'fn:'+target,'calls',offset=ins.address)
                    call=dict(caller=s['name'],callee=target,offset=ins.address,immediates={k:v for k,v in regs.items() if k in ('r3','r4','r5','r6','r7')})
                    calls.append(call)
                    if re.search(r'(?:setMotion|setSceMotion|changeMotion).*FPC?v',target) and 'r4' in regs:
                        pointer=regs['r4']
                        if 0<=pointer<len(raw):
                            value=raw[pointer:raw.find(b'\0',pointer)].decode('ascii','replace')
                            row=dict(call,resourcePointer=pointer,resourceString=value if re.fullmatch(r'[\w/.-]{1,100}',value) else None)
                            rows.append(row);body.append(f"call {target}(this, resource_at={pointer:#x}); // call site {ins.address:#x}")
                    if re.search(r'(?:setMotion|setSceMotion|changeMotion).*FUl',target) and 'r4' in regs:
                        row=dict(call,motionId=regs['r4']);rows.append(row);body.append(f"call {target}(this, motion_id={regs['r4']:#x}); // byte offset {ins.address:#x}")
                for reg in range(0,13):regs.pop('r'+str(reg),None)
            elif m.startswith('b'):
                # No flow join proof in this small extractor. Drop constants.
                regs.clear()
            else:
                # Conservatively invalidate explicitly written registers.
                if parts and parts[0].startswith('r') and not m.startswith(('st','cmp','mt')):regs.pop(parts[0],None)
        if body:descriptions.append((s['name'],body))
    graph.save(root/'evidence/native-actions.graph.jsonl')
    (root/'evidence/native-motion-calls.json').write_text(json.dumps(rows,indent=2),encoding='utf-8')
    (root/'evidence/native-calls.json').write_text(json.dumps(calls,separators=(',',':')),encoding='utf-8')
    (root/'evidence/NATIVE-INDEX.md').write_text('# Native enemy module\n\n'+f'{len(functions)} exported text symbols; {len(calls)} resolved direct calls; {len(rows)} immediate motion-call candidates.\n\nThese are individual call sites. Branch selection, dynamic arguments, receiver dispatch and gameplay eligibility are not reconstructed.\n\n'+'\n'.join(f'- [{name}](scripts/native-{i}.md)' for i,(name,_) in enumerate(descriptions)),encoding='utf-8')
    for i,(name,body) in enumerate(descriptions):
        (root/f'evidence/scripts/native-{i}.md').write_text('# '+name+'\n\nPartial pseudocode: only proven immediate argument call sites; control flow omitted.\n\n```text\n'+'\n'.join(body)+'\n```\n',encoding='utf-8')
    (root/'evidence/NATIVE-API.md').write_text('# Native vocabulary\n\n`calls`: original direct branch or RSO external relocation.\n`motionId`: constant r4 at a setMotion/setSceMotion call with unsigned-long signature. Caller remains native symbol text; semantic move labels require review. Full control flow and receiver bindings are unresolved.\n',encoding='utf-8')
    print(json.dumps(dict(functions=len(functions),calls=len(calls),motionCalls=len(rows),motionCallees=sorted(set(r['callee'] for r in rows))),indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root');a=p.parse_args();build(a.root)
