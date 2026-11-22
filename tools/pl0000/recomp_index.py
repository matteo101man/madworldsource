"""Index the recompiled C++ (tools/upstream-native-core/port/generated/recomp_*.cpp) by function.

python -I tools/pl0000/recomp_index.py  -> research/pl0000-recomp-index.json
Per function: address range, callees, and every changeState (802CBF50) / setRno (802096F8) call with the constant
register values (r4..r7) known at the call site (simple straight-line tracking of `c.r[N] = 0u + K` style loads).
"""
import json, re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GEN = ROOT / 'tools/upstream-native-core/port/generated'
HDR = re.compile(r'^// fn_([0-9A-F]{8})\s+0x([0-9A-F]{8})-0x([0-9A-F]{8})')
CALL = re.compile(r'f_([0-9A-F]{8})\(c, m\)')
SET = re.compile(r'c\.r\[(\d+)\] = 0u \+ \(uint32_t\)(-?\d+);')
SETHI = re.compile(r'c\.r\[(\d+)\] = 0u \+ \(\(uint32_t\)(-?\d+) << 16\);')
MOV = re.compile(r'c\.r\[(\d+)\] = c\.r\[(\d+)\] \| c\.r\[\2\];')
ANY = re.compile(r'c\.r\[(\d+)\] = ')
LABEL = re.compile(r'^L_([0-9A-F]{8}):')


def parse():
    funcs = {}
    for f in sorted(GEN.glob('recomp_*.cpp')):
        cur = None
        for line in f.read_text(encoding='utf-8', errors='replace').splitlines():
            m = HDR.match(line)
            if m:
                cur = dict(addr=m.group(1), end=m.group(3), file=f.name, lines=[]); funcs[m.group(1)] = cur; continue
            if cur is not None: cur['lines'].append(line)
    return funcs


def analyse(fn):
    regs, calls, states = {}, [], []
    for line in fn['lines']:
        s = line.strip()
        if LABEL.match(s): regs = {}; continue          # unknown at join points
        m = CALL.search(s)
        if m:
            callee = m.group(1); calls.append(callee)
            if callee in ('802CBF50', '802096F8'):
                states.append(dict(callee=callee, args={f'r{i}': regs.get(i) for i in (4, 5, 6, 7)}))
            regs = {k: v for k, v in regs.items() if k >= 14}  # volatile regs clobbered
            continue
        m = SET.match(s)
        if m: regs[int(m.group(1))] = int(m.group(2)); continue
        m = SETHI.match(s)
        if m: regs[int(m.group(1))] = int(m.group(2)) << 16; continue
        m = MOV.match(s)
        if m: v = regs.get(int(m.group(2))); regs.pop(int(m.group(1)), None) if v is None else regs.__setitem__(int(m.group(1)), v); continue
        m = ANY.match(s)
        if m: regs.pop(int(m.group(1)), None)
    return dict(file=fn['file'], end=fn['end'], calls=sorted(set(calls)), changeState=states)


def main():
    funcs = parse()
    out = {a: analyse(f) for a, f in funcs.items() if '802700' <= a[:6] <= '802D00'}
    (ROOT / 'research/pl0000-recomp-index.json').write_text(json.dumps(out, indent=1), encoding='utf-8')
    n = sum(1 for v in out.values() if v['changeState'])
    print(len(out), 'functions;', n, 'with changeState/setRno calls')


if __name__ == '__main__':
    main()
