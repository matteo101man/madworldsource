"""Query bounded Pcode graphs as focused per-function pseudocode, never raw dumps."""
import argparse
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[2]))
from gk.graph import Graph


def summarize(root,function):
    root=Path(root);g=Graph.load(root/'evidence/render-pcode.graph.jsonl');names={int(p.split('\t')[0],16):p.split('\t')[2] for p in (root/'tools/ghidra-functions.tsv').read_text().splitlines()};nid='fn:'+function.removeprefix('fn:');memo={}
    def inputs(op):return [n for e,n in sorted(g.inn(op['id'],('feeds',)),key=lambda pair:pair[0].get('attrs',{}).get('seq',0))]
    def expr(value,depth=0):
        a=value.get('attrs',{});offset=a.get('offset',0)
        if a.get('space')=='ram' and offset in names:return names[offset]
        if 'string' in a:return repr(a['string'])
        if a.get('constant') or a.get('space')=='ram':return names.get(offset,hex(offset))
        if depth>7:return value['id'].split(':value:')[-1]+'?'
        definitions=g.inn(value['id'],('defines',))
        if not definitions:return ('r'+str(offset//4)) if a.get('space')=='register' and offset<128 else value['label']
        op=definitions[0][1];args=[expr(n,depth+1) for n in inputs(op)];kind=op['label']
        if kind in ('COPY','CAST','INT_ZEXT','INT_SEXT'):return args[0]
        if kind=='LOAD':return '*('+args[-1]+')'
        if kind in ('INT_ADD','PTRSUB'):return '('+' + '.join(args)+')'
        if kind=='PTRADD':return '('+args[0]+' + '+args[1]+' * '+args[2]+')'
        if kind in ('CALL','CALLIND'):return args[0]+'('+', '.join(args[1:])+')'
        return kind+'('+', '.join(args)+')'
    lines=[]
    for _,op in g.out(nid,('contains',)):
        if op['label'] not in ('CALL','CALLIND','CBRANCH','STORE','RETURN'):continue
        args=[expr(n) for n in inputs(op)];lines.append(dict(address=op['attrs']['address'],operation=op['label'],arguments=args))
    return dict(function=g.nodes[nid],pseudocode=lines,authority='Ghidra Pcode dependency query; unknown types, invalid Gekko instructions and runtime pointers remain explicit')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root');p.add_argument('function');a=p.parse_args();print(json.dumps(summarize(a.root,a.function),indent=2))
