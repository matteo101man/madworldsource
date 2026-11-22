"""Compile owned enemy RSO using analysis CFG and original live relocations.

All emitted game code stays in the D: port. Analysis placeholders never execute.
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

from recomp import Recompiler, Function, write_if_changed
from dol import Section

BASE = 0x81000000
PORT = Path(__file__).resolve().parents[1]


class Image:
    def __init__(self, data, sections):
        self.data = data
        self.sections = [Section(7, BASE + off, data[off:off + size])
                         for i, (off, size) in enumerate(sections) if i > 1 and off and size]

    def read32(self, address):
        off = address - BASE
        return int.from_bytes(self.data[off:off + 4], 'big') if 0 <= off <= len(self.data) - 4 else None

    def data_sections(self):
        return self.sections


class RSORecompiler(Recompiler):
    def __init__(self, image, functions, relocations):
        super().__init__(image, functions)
        self.relocations = relocations

    def seed_relocated_entries(self, internal):
        # Constructors, vtables and callbacks include private, non-exported code.
        # ADDR32 text pointers are authoritative entries, even inside an export span.
        for relocation in internal.values():
            if relocation['type'] != 1 or relocation['section'] != 1:
                continue
            target = BASE + relocation['target']
            function = self.func_at(target)
            if not function or target & 3:
                raise ValueError('Relocated text pointer outside compiled text')
            function.entries.add(target)
            function.labels.add(target)

    def linked_pc(self, address):
        return f'mw_rso_runtime_pc(m, 0x{address:08X}u)'

    def immediate(self, address, value):
        relocation = self.relocations.get(address + 2 - BASE)
        if relocation:
            if relocation['type'] not in (4, 6):
                raise ValueError('Unexpected immediate relocation')
            return f'(int32_t)(int16_t)ld16(m, {self.linked_pc(address + 2)})'
        return super().immediate(address, value)

    def emit_insn(self, function, address, instruction):
        relocation = self.relocations.get(address - BASE)
        if relocation:
            if relocation['type'] != 10 or instruction.op != 'b':
                raise ValueError('Unexpected instruction relocation')
            target = self.branch_target(address, instruction)
            if not (function.addr <= target < function.end and not instruction.f['lk']):
                call = f'ppc_call_indirect(c, m, mw_rso_branch_target(m, 0x{address:08X}u));'
                return (f'c.lr = {self.linked_pc(address + 4)}; {call}' if instruction.f['lk']
                        else f'{{ {call} return; }}')
        half = self.relocations.get(address + 2 - BASE)
        if half and instruction.op not in ('addi', 'addis', 'lwzu', 'lfs', 'lwz', 'stw', 'lfd', 'lbzu', 'lbz', 'stb', 'sth', 'stfs'):
            raise ValueError(f'Unhandled relocated opcode {instruction.op}')
        result = super().emit_insn(function, address, instruction)
        if instruction.op == 'bcctr' and not instruction.f['lk']:
            result = result.replace('switch (c.ctr)', 'switch (mw_rso_canonical_pc(m, c.ctr))')
        if half and 'ld16(m, mw_rso_runtime_pc' not in result:
            raise ValueError('Relocation was not emitted')
        return result

    def write(self, out, per_file):
        # Base emitter is reused; namespacing is an explicit generated-output step.
        staging = out / 'staging'
        count = super().write(staging, per_file)
        for source in staging.glob('*'):
            if source.suffix not in ('.h', '.cpp', '.txt'):
                continue
            name = 'rso_' + source.name
            text = source.read_text(encoding='utf-8').replace('"funcs.h"', '"rso_funcs.h"')
            text = text.replace('g_ppc_func', 'g_rso_func')
            if source.name == 'funcs.h':
                text += '\n#include "rso.h"\n'
            write_if_changed(out / name, text)
        return count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--export', type=Path, required=True)
    parser.add_argument('--analysis', type=Path, required=True)
    parser.add_argument('--out', type=Path, default=PORT / 'generated-rso')
    args = parser.parse_args()
    sys.path.insert(0, str(args.export / 'builders'))
    from native_graph import rso
    raw = (args.export / 'extracted/disc/DATA/files/dll/Rel/enemyRSO.rso').read_bytes()
    sections, exports, _, external, internal = rso(raw)
    linked = args.analysis.read_bytes()
    expected = ('1fd690da94fbd35d84f0a8659d780a377cc970a7c25174e6e2280edb94e3bbe5',
                'b40967835e678db03d9ca27cf18172be3b5c0bd1645b7ab8aefa888dd334c1b5')
    if (hashlib.sha256(raw).hexdigest(), hashlib.sha256(linked).hexdigest()) != expected:
        raise ValueError('Unsupported original or analysis image; refuse stale CFG/relocations')
    if len(linked) != len(raw):
        raise ValueError('Analysis image length differs')
    relocations = {**external, **internal}
    text = sorted({e['offset']: e['name'] for e in exports if e['section'] == 1}.items())
    functions = [Function(name, BASE + offset, (text[i + 1][0] if i + 1 < len(text) else sum(sections[1])) - offset)
                 for i, (offset, name) in enumerate(text)]
    compiler = RSORecompiler(Image(linked, sections), functions, relocations)
    compiler.seed_relocated_entries(internal)
    compiler.analyze()
    compiler.write(args.out, 125)
    manifest = dict(export=str(args.export.resolve()), analysis=str(args.analysis.resolve()),
                    inputs={str(p.resolve()): hashlib.sha256(p.read_bytes()).hexdigest() for p in
                            [Path(__file__), Path(__file__).with_name('recomp.py'), Path(__file__).with_name('ppc_decode.py'),
                             Path(__file__).with_name('dol.py'), Path(__file__).with_name('native-bridges.json'), args.analysis, args.export / 'extracted/disc/DATA/files/dll/Rel/enemyRSO.rso']},
                    sourceSha256=hashlib.sha256(raw).hexdigest(), analysisSha256=hashlib.sha256(linked).hexdigest(),
                    functions=len(functions), entries=sum(len(f.entries) for f in functions), report=dict(compiler.report),
                    authority='Original RSO exports/relocations; live patched immediates/branch targets; no analysis placeholder execution')
    write_if_changed(args.out / 'generation.json', json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(manifest))
    if any(k.startswith('unhandled:') or k == 'invalid words inside functions' for k in compiler.report):
        raise SystemExit('RSO translation requires coverage review')


if __name__ == '__main__':
    main()
