#!/usr/bin/env python3
"""Static recompiler: PowerPC (Gekko) main.dol -> C++.

Usage:
    python port/recomp/recomp.py [--dol orig/RZZE8P/sys/main.dol]
                                 [--symbols config/RZZE8P/symbols.txt]
                                 [--out port/generated] [--per-file 250]

Every function in symbols.txt becomes `PPC_FUNC(f_XXXXXXXX)` operating on a
PPCContext (see port/runtime/cpu/ppc_context.h). Output:
    funcs.h          declarations
    func_table.cpp   address -> function table for indirect calls
Functions listed in port/recomp/hle.txt are emitted as `f_XXXXXXXX_recomp`;
the runtime provides `f_XXXXXXXX` itself (high-level emulation of SDK code).
    recomp_NNN.cpp   function bodies
    report.txt       stats, unhandled instructions, odd control flow
"""
import argparse
import hashlib
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from dol import Dol  # noqa: E402
from ppc_decode import decode  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]

SYM_RE = re.compile(r"^(\S+) = (\.init|\.text):0x([0-9A-Fa-f]+); // type:function size:0x([0-9A-Fa-f]+)")


class Function:
    def __init__(self, name, addr, size):
        self.name, self.addr, self.size = name, addr, size
        self.end = addr + size
        self.entries = {addr}       # extra entry points get added by other functions' branches
        self.labels = set()
        self.has_bcctr = False

    @property
    def cname(self):
        return f"f_{self.addr:08X}"


def load_functions(path: Path):
    funcs = []
    for line in path.read_text().splitlines():
        m = SYM_RE.match(line)
        if m and int(m.group(4), 16):
            funcs.append(Function(m.group(1), int(m.group(3), 16), int(m.group(4), 16)))
    funcs.sort(key=lambda f: f.addr)
    # drop overlaps (keep the first)
    out = []
    for f in funcs:
        if out and f.addr < out[-1].end:
            continue
        out.append(f)
    return out


def load_overrides(path: Path):
    """hle.txt: one `0xADDRESS  name  # comment` per line."""
    out = {}
    if path.exists():
        for line in path.read_text().splitlines():
            line = line.split("#", 1)[0].strip()
            if line:
                parts = line.split()
                out[int(parts[0], 16)] = parts[1] if len(parts) > 1 else ""
    return out


class Recompiler:
    def __init__(self, dol: Dol, funcs, overrides=None):
        self.dol = dol
        self.funcs = funcs
        self.overrides = overrides or {}
        self.starts = {f.addr: f for f in funcs}
        self._addrs = [f.addr for f in funcs]
        self.report = Counter()
        self.notes = []
        bridge_path = Path(__file__).with_name("native-bridges.json")
        self.bridges = json.loads(bridge_path.read_text()) if bridge_path.exists() else {}

    # --- lookups ---------------------------------------------------------
    def func_at(self, addr):
        import bisect
        i = bisect.bisect_right(self._addrs, addr) - 1
        if i >= 0 and addr < self.funcs[i].end:
            return self.funcs[i]
        return None

    def words(self, f):
        for a in range(f.addr, f.end, 4):
            yield a, self.dol.read32(a)

    # --- analysis --------------------------------------------------------
    def analyze(self):
        # Data words pointing inside functions: jump-table / computed targets.
        data_targets = defaultdict(set)
        for sec in self.dol.data_sections():
            d = sec.data
            for o in range(0, len(d) - 3, 4):
                v = int.from_bytes(d[o:o + 4], "big")
                if v & 3 or not (0x80000000 <= v < 0x81800000):
                    continue
                f = self.func_at(v)
                if f:
                    data_targets[f].add(v)
        for f in self.funcs:
            for a, w in self.words(f):
                ins = decode(w) if w is not None else None
                if ins is None:
                    continue
                if ins.op == "bcctr":
                    f.has_bcctr = True
                tgt = self.branch_target(a, ins)
                if tgt is None:
                    continue
                if f.addr <= tgt < f.end and not ins.f.get("lk"):
                    f.labels.add(tgt)
                else:
                    g = self.func_at(tgt)
                    if g and tgt != g.addr:
                        g.entries.add(tgt)
                        g.labels.add(tgt)
        for f in self.funcs:
            # A bctr can only land on addresses some data word points at (jump tables).
            f.jump_targets = sorted(data_targets.get(f, ())) if f.has_bcctr else []
            f.labels.update(f.jump_targets)
        for entry in self.bridges.get("continuations", []):
            address = int(entry, 16)
            function = self.func_at(address)
            if function:
                function.entries.add(address)
                function.labels.add(address)
        self.report["functions"] = len(self.funcs)
        self.report["extra entry points"] = sum(len(f.entries) - 1 for f in self.funcs)

    @staticmethod
    def branch_target(addr, ins):
        if ins.op == "b":
            return (ins.f["li"] if ins.f["aa"] else addr + ins.f["li"]) & 0xFFFFFFFF
        if ins.op == "bc":
            return (ins.f["bd"] if ins.f["aa"] else addr + ins.f["bd"]) & 0xFFFFFFFF
        return None

    def immediate(self, addr, value):
        return str(value)

    def linked_pc(self, addr):
        return f"0x{addr:08X}u"

    # --- emission helpers ------------------------------------------------
    def call_expr(self, tgt):
        g = self.func_at(tgt)
        if g is None:
            self.report["calls to unknown code"] += 1
            return f"ppc_call_indirect(c, m, 0x{tgt:08X}u);"
        return f"{self.entry_name(g, tgt)}(c, m);"

    @staticmethod
    def entry_name(g, tgt):
        return g.cname if tgt == g.addr else f"f_{tgt:08X}"

    def jump(self, f, addr, tgt):
        """Unconditional transfer to tgt without linking."""
        if f.addr <= tgt < f.end and tgt in f.labels:
            return f"goto L_{tgt:08X};"
        return f"{{ {self.call_expr(tgt)} return; }}"

    @staticmethod
    def cond_expr(bo, bi):
        pre, parts = "", []
        if not bo & 4:
            pre = "c.ctr--; "
            parts.append("c.ctr == 0" if bo & 2 else "c.ctr != 0")
        if not bo & 0x10:
            want = (bo >> 3) & 1
            parts.append(f"{'' if want else '!'}cr_bit(c, {bi})")
        return pre, " && ".join(parts) if parts else "true"

    # --- per instruction -------------------------------------------------
    def emit_insn(self, f, addr, ins):  # noqa: C901 - one big table
        o, F = ins.op, ins.f
        nxt = addr + 4
        rc = (lambda v: f" cr0_rec(c, {v});") if ins.rc else (lambda v: "")
        R = lambda n: f"c.r[{n}]"  # noqa: E731
        RA0 = lambda n: "0u" if n == 0 else f"c.r[{n}]"  # noqa: E731
        FR = lambda n: f"c.f[{n}]"  # noqa: E731

        def ea_d():
            return f"({RA0(F['ra'])} + (uint32_t){self.immediate(addr, F['d'])})"

        def ea_x():
            return f"({RA0(F['ra'])} + c.r[{F['rb']}])"

        if f"{addr:08x}" in self.bridges.get("nonlocal_returns", []):
            if o != "bclr" or F["lk"]:
                raise ValueError("Native unwind boundary differs from verified return")
            return "extern void ppc_native_unwind(PPCContext&, uint8_t*, uint32_t); ppc_native_unwind(c, m, c.lr); return;"

        # ---- branches
        if o == "b":
            tgt = self.branch_target(addr, ins)
            if F["lk"]:
                if tgt == nxt:
                    return f"c.lr = {self.linked_pc(nxt)};"
                return f"c.lr = {self.linked_pc(nxt)}; {self.call_expr(tgt)}"
            return self.jump(f, addr, tgt)
        if o == "bc":
            tgt = self.branch_target(addr, ins)
            pre, cond = self.cond_expr(F["bo"], F["bi"])
            if F["lk"]:
                return f"{pre}if ({cond}) {{ c.lr = {self.linked_pc(nxt)}; {self.call_expr(tgt)} }}"
            return f"{pre}if ({cond}) {self.jump(f, addr, tgt)}"
        if o == "bclr":
            pre, cond = self.cond_expr(F["bo"], F["bi"])
            if F["lk"]:
                return (f"{pre}if ({cond}) {{ uint32_t t = c.lr; c.lr = {self.linked_pc(nxt)}; "
                        f"ppc_call_indirect(c, m, t); }}")
            return f"{pre}if ({cond}) return;" if cond != "true" else f"{pre}return;"
        if o == "bcctr":
            pre, cond = self.cond_expr(F["bo"], F["bi"])
            if F["lk"]:
                return f"{pre}if ({cond}) {{ c.lr = {self.linked_pc(nxt)}; ppc_call_indirect(c, m, c.ctr); }}"
            cases = "".join(f" case 0x{t:08X}u: goto L_{t:08X};" for t in f.jump_targets)
            body = f"switch (c.ctr) {{{cases} default: ppc_call_indirect(c, m, c.ctr); return; }}"
            return f"{pre}if ({cond}) {{ {body} }}" if cond != "true" else f"{pre}{body}"

        # ---- integer arithmetic
        if o == "addi":
            return f"{R(F['rd'])} = {RA0(F['ra'])} + (uint32_t){self.immediate(addr, F['simm'])};"
        if o == "addis":
            return f"{R(F['rd'])} = {RA0(F['ra'])} + ((uint32_t){self.immediate(addr, F['simm'])} << 16);"
        if o == "addic":
            return (f"{{ uint32_t a = {R(F['ra'])}; uint32_t r = a + (uint32_t){F['simm']}; "
                    f"c.ca = r < a; {R(F['rd'])} = r;{rc('r')} }}")
        if o == "subfic":
            return (f"{{ uint64_t r = (uint64_t)(uint32_t)~{R(F['ra'])} + (uint32_t){F['simm']} + 1; "
                    f"c.ca = (uint32_t)(r >> 32); {R(F['rd'])} = (uint32_t)r; }}")
        if o == "mulli":
            return f"{R(F['rd'])} = (uint32_t)((int32_t){R(F['ra'])} * {F['simm']});"
        if o in ("add", "subf", "neg", "mullw", "mulhw", "mulhwu", "divw", "divwu",
                 "addc", "adde", "addze", "addme", "subfc", "subfe", "subfze", "subfme"):
            a, b, d = R(F["ra"]), R(F["rb"]), R(F["rd"])
            if ins.oe:
                self.report["OE (overflow) forms, OV not modelled"] += 1
            expr = {
                "add": f"{d} = {a} + {b};",
                "subf": f"{d} = {b} - {a};",
                "neg": f"{d} = (uint32_t)-(int32_t){a};",
                "mullw": f"{d} = (uint32_t)((int32_t){a} * (int32_t){b});",
                "mulhw": f"{d} = (uint32_t)(((int64_t)(int32_t){a} * (int32_t){b}) >> 32);",
                "mulhwu": f"{d} = (uint32_t)(((uint64_t){a} * {b}) >> 32);",
                "divw": f"{d} = ppc_divw({a}, {b});",
                "divwu": f"{d} = ppc_divwu({a}, {b});",
                "addc": f"{{ uint64_t r = (uint64_t){a} + {b}; c.ca = (uint32_t)(r >> 32); {d} = (uint32_t)r; }}",
                "adde": f"{{ uint64_t r = (uint64_t){a} + {b} + c.ca; c.ca = (uint32_t)(r >> 32); {d} = (uint32_t)r; }}",
                "addze": f"{{ uint64_t r = (uint64_t){a} + c.ca; c.ca = (uint32_t)(r >> 32); {d} = (uint32_t)r; }}",
                "addme": f"{{ uint64_t r = (uint64_t){a} + c.ca + 0xFFFFFFFFull; c.ca = (uint32_t)(r >> 32); {d} = (uint32_t)r; }}",
                "subfc": f"{{ uint64_t r = (uint64_t)(uint32_t)~{a} + {b} + 1; c.ca = (uint32_t)(r >> 32); {d} = (uint32_t)r; }}",
                "subfe": f"{{ uint64_t r = (uint64_t)(uint32_t)~{a} + {b} + c.ca; c.ca = (uint32_t)(r >> 32); {d} = (uint32_t)r; }}",
                "subfze": f"{{ uint64_t r = (uint64_t)(uint32_t)~{a} + c.ca; c.ca = (uint32_t)(r >> 32); {d} = (uint32_t)r; }}",
                "subfme": f"{{ uint64_t r = (uint64_t)(uint32_t)~{a} + c.ca + 0xFFFFFFFFull; c.ca = (uint32_t)(r >> 32); {d} = (uint32_t)r; }}",
            }[o]
            return expr + rc(d)

        # ---- compare
        if o == "cmpi":
            return f"cr_cmp_s(c, {F['crf']}, (int32_t){R(F['ra'])}, {F['simm']});"
        if o == "cmpli":
            return f"cr_cmp_u(c, {F['crf']}, {R(F['ra'])}, {F['uimm']}u);"
        if o == "cmp":
            return f"cr_cmp_s(c, {F['crf']}, (int32_t){R(F['ra'])}, (int32_t){R(F['rb'])});"
        if o == "cmpl":
            return f"cr_cmp_u(c, {F['crf']}, {R(F['ra'])}, {R(F['rb'])});"

        # ---- logical
        logic_imm = {"ori": "|", "oris": "|", "xori": "^", "xoris": "^", "andi": "&", "andis": "&"}
        if o in logic_imm:
            imm = F["uimm"] << 16 if o.endswith("s") else F["uimm"]
            if o in ("ori",) and F["rs"] == 0 and F["ra"] == 0 and imm == 0:
                return "/* nop */"
            return f"{R(F['ra'])} = {R(F['rs'])} {logic_imm[o]} 0x{imm:X}u;{rc(R(F['ra']))}"
        logic = {"and": "{s} & {b}", "andc": "{s} & ~{b}", "or": "{s} | {b}", "orc": "{s} | ~{b}",
                 "xor": "{s} ^ {b}", "nor": "~({s} | {b})", "nand": "~({s} & {b})",
                 "eqv": "~({s} ^ {b})",
                 "slw": "ppc_slw({s}, {b})", "srw": "ppc_srw({s}, {b})",
                 "cntlzw": "ppc_cntlzw({s})", "extsb": "(uint32_t)(int32_t)(int8_t){s}",
                 "extsh": "(uint32_t)(int32_t)(int16_t){s}"}
        if o in logic:
            e = logic[o].format(s=R(F["rs"]), b=R(F["rb"]))
            return f"{R(F['ra'])} = {e};{rc(R(F['ra']))}"
        if o == "sraw":
            return f"{R(F['ra'])} = ppc_sraw(c, {R(F['rs'])}, {R(F['rb'])});{rc(R(F['ra']))}"
        if o == "srawi":
            return f"{R(F['ra'])} = ppc_srawi(c, {R(F['rs'])}, {F['sh']});{rc(R(F['ra']))}"
        if o in ("rlwinm", "rlwimi", "rlwnm"):
            mask = rot_mask(F["mb"], F["me"])
            s, a = R(F["rs"]), R(F["ra"])
            if o == "rlwinm":
                rot = s if F["sh"] == 0 else f"ppc_rotl({s}, {F['sh']})"
                return f"{a} = {rot} & 0x{mask:08X}u;{rc(a)}"
            if o == "rlwnm":
                return f"{a} = ppc_rotl({s}, {R(F['rb'])} & 31) & 0x{mask:08X}u;{rc(a)}"
            return (f"{a} = (ppc_rotl({s}, {F['sh']}) & 0x{mask:08X}u) | ({a} & 0x{(~mask) & 0xFFFFFFFF:08X}u);"
                    f"{rc(a)}")

        # ---- condition register
        if o == "mfcr":
            return f"{R(F['rd'])} = c.cr;"
        if o == "mtcrf":
            crm = F["crm"]
            mask = 0
            for i in range(8):
                if crm & (0x80 >> i):
                    mask |= 0xF0000000 >> (4 * i)
            return f"c.cr = (c.cr & 0x{(~mask) & 0xFFFFFFFF:08X}u) | ({R(F['rs'])} & 0x{mask:08X}u);"
        if o == "mcrf":
            return f"cr_set_field(c, {F['crfd']}, cr_get_field(c, {F['crfs']}));"
        if o == "mcrxr":
            return f"cr_set_field(c, {F['crf']}, ppc_get_xer(c) >> 28); ppc_set_xer(c, ppc_get_xer(c) & 0x0FFFFFFFu);"
        crops = {"crand": "a & b", "cror": "a | b", "crxor": "a ^ b", "crnand": "!(a & b)",
                 "crnor": "!(a | b)", "creqv": "!(a ^ b)", "crandc": "a & !b", "crorc": "a | !b"}
        if o in crops:
            return (f"{{ uint32_t a = cr_bit(c, {F['crba']}), b = cr_bit(c, {F['crbb']}); "
                    f"cr_set_bit(c, {F['crbd']}, ({crops[o]}) & 1); }}")

        # ---- SPRs / system
        if o == "mfspr":
            spr = F["spr"]
            fast = {8: "c.lr", 9: "c.ctr", 1: "ppc_get_xer(c)"}
            if spr in fast:
                return f"{R(F['rd'])} = {fast[spr]};"
            if 912 <= spr <= 919:
                return f"{R(F['rd'])} = c.gqr[{spr - 912}];"
            return f"{R(F['rd'])} = ppc_mfspr(c, {spr});"
        if o == "mtspr":
            spr, s = F["spr"], R(F["rd"])
            if spr == 8:
                return f"c.lr = {s};"
            if spr == 9:
                return f"c.ctr = {s};"
            if spr == 1:
                return f"ppc_set_xer(c, {s});"
            if 912 <= spr <= 919:
                return f"c.gqr[{spr - 912}] = {s};"
            return f"ppc_mtspr(c, m, {spr}, {s});"
        if o == "mftb":
            return f"{R(F['rd'])} = ppc_mftb(c, {F['spr']});"
        if o == "mfmsr":
            return f"{R(F['rd'])} = c.msr;"
        if o == "mtmsr":
            return f"ppc_mtmsr(c, m, {R(F['rd'])});"
        if o == "mfsr":
            return f"{R(F['rd'])} = c.sr[{F['sr']}];"
        if o == "mtsr":
            return f"c.sr[{F['sr']}] = {R(F['rd'])};"
        if o in ("mfsrin", "mtsrin"):
            if o == "mfsrin":
                return f"{R(F['rd'])} = c.sr[{R(F['rb'])} >> 28];"
            return f"c.sr[{R(F['rb'])} >> 28] = {R(F['rd'])};"
        if o in ("sync", "isync", "eieio", "dcbf", "dcbst", "dcbt", "dcbtst", "icbi", "tlbie",
                 "tlbsync", "dcbi"):
            return f"/* {o} */"
        if o == "dcbz":
            return f"ppc_dcbz(m, {ea_x()});"
        if o == "dcbz_l":
            return f"ppc_dcbz(m, {ea_x()});"
        if o == "sc":
            return "ppc_syscall(c, m);"
        if o == "rfi":
            return "ppc_rfi(c, m); return;"
        if o in ("tw", "twi"):
            return f"ppc_trap(c, 0x{addr:08X}u);"
        if o in ("eciwx", "ecowx"):
            return f"ppc_unimplemented(c, 0x{addr:08X}u, \"{o}\");"

        # ---- integer loads / stores
        int_ld = {"lwz": "ld32", "lbz": "ld8", "lhz": "ld16", "lha": "lds16"}
        int_st = {"stw": "st32", "stb": "st8", "sth": "st16"}
        if o in ("lwz", "lbz", "lhz", "lha", "lwzu", "lbzu", "lhzu", "lhau",
                 "lwzx", "lbzx", "lhzx", "lhax", "lwzux", "lbzux", "lhzux", "lhaux"):
            fn = int_ld[o[:3]]
            indexed = o.endswith("x")
            upd = "u" in o[3:]
            ea = ea_x() if indexed else ea_d()
            if upd:
                return f"{{ uint32_t ea = {ea}; {R(F['rd'])} = {fn}(m, ea); c.r[{F['ra']}] = ea; }}"
            return f"{R(F['rd'])} = {fn}(m, {ea});"
        if o in ("stw", "stb", "sth", "stwu", "stbu", "sthu", "stwx", "stbx", "sthx",
                 "stwux", "stbux", "sthux"):
            fn = int_st[o[:3]]
            indexed = o.endswith("x")
            upd = "u" in o[3:]
            ea = ea_x() if indexed else ea_d()
            if upd:
                return f"{{ uint32_t ea = {ea}; {fn}(m, ea, {R(F['rd'])}); c.r[{F['ra']}] = ea; }}"
            return f"{fn}(m, {ea}, {R(F['rd'])});"
        if o == "lwbrx":
            return f"{R(F['rd'])} = ld32le(m, {ea_x()});"
        if o == "lhbrx":
            return f"{R(F['rd'])} = ld16le(m, {ea_x()});"
        if o == "stwbrx":
            return f"st32le(m, {ea_x()}, {R(F['rd'])});"
        if o == "sthbrx":
            return f"st16le(m, {ea_x()}, {R(F['rd'])});"
        if o == "lmw":
            return f"{{ uint32_t ea = {ea_d()}; for (int i = {F['rd']}; i < 32; i++, ea += 4) c.r[i] = ld32(m, ea); }}"
        if o == "stmw":
            return f"{{ uint32_t ea = {ea_d()}; for (int i = {F['rd']}; i < 32; i++, ea += 4) st32(m, ea, c.r[i]); }}"
        if o == "lwarx":
            return f"{R(F['rd'])} = ld32(m, {ea_x()});"
        if o == "stwcx":
            return f"st32(m, {ea_x()}, {R(F['rs'])}); cr_set_field(c, 0, 0x2 | (c.so & 1));"
        if o in ("lswi", "stswi"):
            nb = F["nb"] or 32
            return f"ppc_{o}(c, m, {F['rd']}, {RA0(F['ra'])}, {nb});"
        if o in ("lswx", "stswx"):
            return f"ppc_unimplemented(c, 0x{addr:08X}u, \"{o}\");"

        # ---- floating point loads / stores
        if o in ("lfs", "lfsu", "lfsx", "lfsux", "lfd", "lfdu", "lfdx", "lfdux"):
            indexed = o.endswith("x")
            upd = "u" in o[3:]
            ea = ea_x() if indexed else ea_d()
            fd = FR(F["rd"])
            load = (f"{fd}.ps0 = {fd}.ps1 = (double)ldf32(m, ea);" if o.startswith("lfs")
                    else f"{fd}.u0 = ld64(m, ea);")
            update = f" c.r[{F['ra']}] = ea;" if upd else ""
            return f"{{ uint32_t ea = {ea}; {load}{update} }}"
        if o in ("stfs", "stfsu", "stfsx", "stfsux", "stfd", "stfdu", "stfdx", "stfdux", "stfiwx"):
            indexed = o.endswith("x")
            upd = "u" in o[4:]
            ea = ea_x() if indexed else ea_d()
            fs = FR(F["rd"])
            if o == "stfiwx":
                store = f"st32(m, ea, (uint32_t){fs}.u0);"
            elif o.startswith("stfs"):
                store = f"stf32(m, ea, (float){fs}.ps0);"
            else:
                store = f"st64(m, ea, {fs}.u0);"
            update = f" c.r[{F['ra']}] = ea;" if upd else ""
            return f"{{ uint32_t ea = {ea}; {store}{update} }}"

        # ---- floating point arithmetic
        frc = " cr1_rec_fp(c);" if ins.rc else ""
        if o in ("fadds", "fsubs", "fmuls", "fdivs", "fmadds", "fmsubs", "fnmadds", "fnmsubs",
                 "fadd", "fsub", "fmul", "fdiv", "fmadd", "fmsub", "fnmadd", "fnmsub"):
            a, b, cc = f"{FR(F['fa'])}.ps0", f"{FR(F['fb'])}.ps0", f"{FR(F['fc'])}.ps0"
            core = o.rstrip("s") if o.endswith("s") and o not in ("fabs",) else o
            # PowerPC multiply-add is fused (one rounding), hence std::fma.
            expr = {
                "fadd": f"{a} + {b}", "fsub": f"{a} - {b}", "fmul": f"{a} * {cc}",
                "fdiv": f"{a} / {b}", "fmadd": f"std::fma({a}, {cc}, {b})",
                "fmsub": f"std::fma({a}, {cc}, -{b})",
                "fnmadd": f"-std::fma({a}, {cc}, {b})", "fnmsub": f"-std::fma({a}, {cc}, -{b})",
            }[core]
            fd = FR(F["fd"])
            if o.endswith("s"):
                return f"{{ double r = ppc_round_single({expr}); {fd}.ps0 = r; {fd}.ps1 = r; }}{frc}"
            return f"{fd}.ps0 = {expr};{frc}"
        if o in ("fsqrt", "fsqrts", "fres", "frsqrte"):
            b, fd = f"{FR(F['fb'])}.ps0", FR(F["fd"])
            e = {"fsqrt": f"std::sqrt({b})", "fsqrts": f"ppc_round_single(std::sqrt({b}))",
                 "fres": f"ppc_round_single(1.0 / {b})", "frsqrte": f"1.0 / std::sqrt({b})"}[o]
            if o in ("fsqrts", "fres"):
                return f"{{ double r = {e}; {fd}.ps0 = r; {fd}.ps1 = r; }}{frc}"
            return f"{fd}.ps0 = {e};{frc}"
        if o == "fsel":
            a, b, cc = (f"{FR(F[k])}.ps0" for k in ("fa", "fb", "fc"))
            return f"{FR(F['fd'])}.ps0 = ({a} >= 0.0) ? {cc} : {b};{frc}"
        if o == "frsp":
            fd = FR(F["fd"])
            return f"{{ double r = ppc_round_single({FR(F['fb'])}.ps0); {fd}.ps0 = r; {fd}.ps1 = r; }}{frc}"
        if o in ("fctiw", "fctiwz"):
            return f"{FR(F['fd'])}.u0 = ppc_{o}(c, {FR(F['fb'])}.ps0);{frc}"
        if o == "fmr":
            return f"{FR(F['fd'])}.u0 = {FR(F['fb'])}.u0;{frc}"
        if o in ("fneg", "fabs", "fnabs"):
            op = {"fneg": "^ 0x8000000000000000ull", "fabs": "& 0x7FFFFFFFFFFFFFFFull",
                  "fnabs": "| 0x8000000000000000ull"}[o]
            return f"{FR(F['fd'])}.u0 = {FR(F['fb'])}.u0 {op};{frc}"
        if o in ("fcmpu", "fcmpo"):
            return f"cr_cmp_f(c, {F['crf']}, {FR(F['fa'])}.ps0, {FR(F['fb'])}.ps0);"
        if o == "mffs":
            return f"{FR(F['fd'])}.u0 = 0xFFF8000000000000ull | c.fpscr;{frc}"
        if o == "mtfsf":
            return f"ppc_mtfsf(c, {F['fm']}, (uint32_t){FR(F['fb'])}.u0);{frc}"
        if o == "mtfsfi":
            return f"ppc_mtfsf(c, 0x{0x80 >> F['crfd']:02X}, {F['imm']}u << {28 - 4 * F['crfd']});{frc}"
        if o == "mtfsb0":
            return f"c.fpscr &= ~(0x80000000u >> {F['crbd']});{frc}"
        if o == "mtfsb1":
            return f"c.fpscr |= (0x80000000u >> {F['crbd']});{frc}"
        if o == "mcrfs":
            return f"cr_set_field(c, {F['crfd']}, (c.fpscr >> {28 - 4 * F['crfs']}) & 0xF);"

        # ---- paired singles
        if o.startswith("ps_") or o.startswith("psq_"):
            return self.emit_ps(ins, addr)

        self.report[f"unhandled: {o}"] += 1
        return f"ppc_unimplemented(c, 0x{addr:08X}u, \"{o}\");"

    def emit_ps(self, ins, addr):
        o, F = ins.op, ins.f
        FR = lambda n: f"c.f[{n}]"  # noqa: E731
        frc = " cr1_rec_fp(c);" if ins.rc else ""
        if o.startswith("psq_"):
            store = "st" in o
            indexed = o.endswith("x")
            upd = "u" in o[4:]
            ra = F["ra"]
            base = "0u" if ra == 0 and not upd else f"c.r[{ra}]"
            ea = f"({base} + c.r[{F['rb']}])" if indexed else f"({base} + (uint32_t){self.immediate(addr, F['d'])})"
            gqr = f"c.gqr[{F['i']}]"
            fn = "psq_store" if store else "psq_load"
            body = f"{fn}(c, m, ea, {F['fd']}, {F['w']}, {gqr});"
            return f"{{ uint32_t ea = {ea}; {body}{f' c.r[{ra}] = ea;' if upd else ''} }}"
        d, a, b, cc = FR(F.get("fd", 0)), FR(F.get("fa", 0)), FR(F.get("fb", 0)), FR(F.get("fc", 0))
        two = {
            "ps_add": ("{a}.ps0 + {b}.ps0", "{a}.ps1 + {b}.ps1"),
            "ps_sub": ("{a}.ps0 - {b}.ps0", "{a}.ps1 - {b}.ps1"),
            "ps_mul": ("{a}.ps0 * {c}.ps0", "{a}.ps1 * {c}.ps1"),
            "ps_div": ("{a}.ps0 / {b}.ps0", "{a}.ps1 / {b}.ps1"),
            "ps_madd": ("std::fma({a}.ps0, {c}.ps0, {b}.ps0)", "std::fma({a}.ps1, {c}.ps1, {b}.ps1)"),
            "ps_msub": ("std::fma({a}.ps0, {c}.ps0, -{b}.ps0)", "std::fma({a}.ps1, {c}.ps1, -{b}.ps1)"),
            "ps_nmadd": ("-std::fma({a}.ps0, {c}.ps0, {b}.ps0)", "-std::fma({a}.ps1, {c}.ps1, {b}.ps1)"),
            "ps_nmsub": ("-std::fma({a}.ps0, {c}.ps0, -{b}.ps0)", "-std::fma({a}.ps1, {c}.ps1, -{b}.ps1)"),
            "ps_muls0": ("{a}.ps0 * {c}.ps0", "{a}.ps1 * {c}.ps0"),
            "ps_muls1": ("{a}.ps0 * {c}.ps1", "{a}.ps1 * {c}.ps1"),
            "ps_madds0": ("std::fma({a}.ps0, {c}.ps0, {b}.ps0)", "std::fma({a}.ps1, {c}.ps0, {b}.ps1)"),
            "ps_madds1": ("std::fma({a}.ps0, {c}.ps1, {b}.ps0)", "std::fma({a}.ps1, {c}.ps1, {b}.ps1)"),
            "ps_sum0": ("{a}.ps0 + {b}.ps1", None),
            "ps_sum1": (None, "{a}.ps0 + {b}.ps1"),
            "ps_res": ("1.0 / {b}.ps0", "1.0 / {b}.ps1"),
            "ps_rsqrte": ("1.0 / std::sqrt({b}.ps0)", "1.0 / std::sqrt({b}.ps1)"),
        }
        if o in two:
            e0, e1 = two[o]
            fmt = dict(a=a, b=b, c=cc)
            p0 = f"ppc_round_single({e0.format(**fmt)})" if e0 else f"{cc}.ps0"
            p1 = f"ppc_round_single({e1.format(**fmt)})" if e1 else f"{cc}.ps1"
            if o == "ps_sum0":
                p1 = f"{cc}.ps1"
            if o == "ps_sum1":
                p0 = f"{cc}.ps0"
            return f"{{ double r0 = {p0}, r1 = {p1}; {d}.ps0 = r0; {d}.ps1 = r1; }}{frc}"
        if o == "ps_sel":
            return (f"{{ double r0 = ({a}.ps0 >= 0.0) ? {cc}.ps0 : {b}.ps0, "
                    f"r1 = ({a}.ps1 >= 0.0) ? {cc}.ps1 : {b}.ps1; {d}.ps0 = r0; {d}.ps1 = r1; }}{frc}")
        if o in ("ps_neg", "ps_abs", "ps_nabs", "ps_mr"):
            fn = {"ps_neg": "-{x}", "ps_abs": "std::fabs({x})", "ps_nabs": "-std::fabs({x})",
                  "ps_mr": "{x}"}[o]
            return (f"{{ double r0 = {fn.format(x=b + '.ps0')}, r1 = {fn.format(x=b + '.ps1')}; "
                    f"{d}.ps0 = r0; {d}.ps1 = r1; }}{frc}")
        merges = {"ps_merge00": ("ps0", "ps0"), "ps_merge01": ("ps0", "ps1"),
                  "ps_merge10": ("ps1", "ps0"), "ps_merge11": ("ps1", "ps1")}
        if o in merges:
            x, y = merges[o]
            return f"{{ double r0 = {a}.{x}, r1 = {b}.{y}; {d}.ps0 = r0; {d}.ps1 = r1; }}{frc}"
        if o in ("ps_cmpu0", "ps_cmpo0", "ps_cmpu1", "ps_cmpo1"):
            lane = "ps0" if o.endswith("0") else "ps1"
            return f"cr_cmp_f(c, {F['crf']}, {a}.{lane}, {b}.{lane});"
        self.report[f"unhandled: {o}"] += 1
        return f"ppc_unimplemented(c, 0x{addr:08X}u, \"{o}\");"

    # --- per function ----------------------------------------------------
    def emit_function(self, f):
        lines = []
        backedges = set()
        for address, word in self.words(f):
            instruction = decode(word) if word is not None else None
            if instruction and instruction.op in ("b", "bc") and not instruction.f.get("lk"):
                target = self.branch_target(address, instruction)
                if f.addr <= target <= address:
                    backedges.add(target)
        multi = len(f.entries) > 1
        hdr = f"// {f.name}  0x{f.addr:08X}-0x{f.end:08X}"
        if multi:
            lines.append(hdr)
            lines.append(f"static void body_{f.addr:08X}(PPCContext& c, uint8_t* m, uint32_t entry) {{")
            cases = " ".join(f"case 0x{e:08X}u: goto L_{e:08X};" for e in sorted(f.entries) if e != f.addr)
            lines.append("\tPPC_ENTER(entry);")
            lines.append(f"\tswitch (entry) {{ {cases} default: break; }}")
        else:
            lines.append(hdr)
            name = f.cname + ("_recomp" if f.addr in self.overrides else "")
            lines.append(f"PPC_FUNC({name}) {{")
            lines.append(f"\tPPC_ENTER(0x{f.addr:08X}u);")
        last = None
        for addr, w in self.words(f):
            if addr in f.labels:
                lines.append(f"L_{addr:08X}:")
                if addr in backedges:
                    lines.append("\tppc_poll(c, m); // cooperative IRQ safe point on original loop")
            ins = decode(w) if w is not None else None
            if ins is None:
                self.report["invalid words inside functions"] += 1
                lines.append(f"\tppc_illegal(c, 0x{addr:08X}u); return; // .long 0x{(w or 0):08X}")
                last = None
                continue
            lines.append(f"\t{self.emit_insn(f, addr, ins)}")
            last = ins
        # falling off the end continues into the next function
        if not self.is_terminator(last):
            nxt = self.starts.get(f.end)
            if nxt:
                lines.append(f"\t{nxt.cname}(c, m); return;")
                self.report["fall-through into next function"] += 1
            else:
                lines.append(f"\tppc_fell_off(c, 0x{f.end:08X}u);")
        lines.append("}")
        if multi:
            for e in sorted(f.entries):
                name = f"f_{e:08X}" + ("_recomp" if e in self.overrides else "")
                lines.append(f"PPC_FUNC({name}) {{ body_{f.addr:08X}(c, m, 0x{e:08X}u); }}")
        return "\n".join(lines)

    @staticmethod
    def is_terminator(ins):
        if ins is None:
            return True
        if ins.op == "b" and not ins.f["lk"]:
            return True
        if ins.op in ("bclr", "bcctr") and not ins.f["lk"] and (ins.f["bo"] & 0x14) == 0x14:
            return True
        if ins.op == "rfi":
            return True
        return False

    # --- output ----------------------------------------------------------
    def write(self, out: Path, per_file: int):
        """Writes the output, touching only files whose contents changed so the
        build recompiles just those."""
        out.mkdir(parents=True, exist_ok=True)
        hdr = "// Generated by port/recomp/recomp.py. Do not edit.\n"
        entries = sorted({e for f in self.funcs for e in f.entries})

        h = [hdr, '#pragma once\n#include "ppc_context.h"\n\n']
        h += [f"PPC_FUNC(f_{e:08X});\n" for e in entries]
        write_if_changed(out / "funcs.h", "".join(h))

        # Kept out of funcs.h so editing hle.txt only rebuilds the files it touches.
        h = [hdr, '#pragma once\n#include "funcs.h"\n\n',
             "// Original code of functions the runtime replaces (hle.txt)\n"]
        h += [f"PPC_FUNC(f_{a:08X}_recomp);  // {self.overrides[a]}\n" for a in sorted(self.overrides)]
        write_if_changed(out / "hle_funcs.h", "".join(h))

        t = [hdr, '#include "funcs.h"\n\nconst PPCFuncEntry g_ppc_funcs[] = {\n']
        t += [f"\t{{0x{e:08X}u, f_{e:08X}}},\n" for e in entries]
        t.append("};\nconst size_t g_ppc_func_count = sizeof(g_ppc_funcs) / sizeof(g_ppc_funcs[0]);\n\n")
        # Names for logs and crash reports (sorted by address)
        t.append("const PPCFuncName g_ppc_func_names[] = {\n")
        t += [f'\t{{0x{f.addr:08X}u, 0x{f.end:08X}u, "{f.name}"}},\n' for f in self.funcs]
        t.append("};\nconst size_t g_ppc_func_name_count = "
                 "sizeof(g_ppc_func_names) / sizeof(g_ppc_func_names[0]);\n")
        write_if_changed(out / "func_table.cpp", "".join(t))

        n = 0
        for i in range(0, len(self.funcs), per_file):
            body = [hdr, '#include "funcs.h"\n\n']
            for f in self.funcs[i:i + per_file]:
                body += [self.emit_function(f), "\n\n"]
            write_if_changed(out / f"recomp_{n:03d}.cpp", "".join(body))
            n += 1
        for stale in out.glob("recomp_*.cpp"):
            if int(stale.stem.split("_")[1]) >= n:
                stale.unlink()
        self.report["output files"] = n
        write_if_changed(out / "report.txt", "".join(f"{k}: {v}\n" for k, v in sorted(self.report.items())))
        return n


def write_if_changed(path: Path, text: str):
    if path.exists() and path.read_text() == text:
        return
    path.write_text(text)


def rot_mask(mb, me):
    if mb <= me:
        m = ((0xFFFFFFFF >> mb) & (0xFFFFFFFF << (31 - me))) & 0xFFFFFFFF
    else:
        m = ((0xFFFFFFFF >> mb) | (0xFFFFFFFF << (31 - me))) & 0xFFFFFFFF
    return m


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dol", type=Path, default=ROOT / "orig/RZZE8P/sys/main.dol")
    ap.add_argument("--symbols", type=Path, default=ROOT / "config/RZZE8P/symbols.txt")
    ap.add_argument("--out", type=Path, default=ROOT / "port/generated")
    ap.add_argument("--per-file", type=int, default=250)
    args = ap.parse_args()

    overrides = load_overrides(Path(__file__).parent / "hle.txt")
    rc = Recompiler(Dol(args.dol), load_functions(args.symbols), overrides)
    rc.analyze()
    bad = [a for a in overrides if a not in rc.starts]
    if bad:
        sys.exit("hle.txt: not a function start: " + ", ".join(f"0x{a:08X}" for a in bad))
    n = rc.write(args.out, args.per_file)
    inputs = [Path(__file__), Path(__file__).with_name("ppc_decode.py"), Path(__file__).with_name("native-bridges.json"),
              Path(__file__).with_name("dol.py"), Path(__file__).with_name("hle.txt"),
              args.symbols, args.dol]
    (args.out / "generation.json").write_text(json.dumps({
        "dol": str(args.dol.resolve()), "perFile": args.per_file,
        "inputs": {str(p.resolve()): hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}
    }, indent=2) + "\n", encoding="utf-8")
    for k, v in sorted(rc.report.items()):
        print(f"{k}: {v}")
    print(f"wrote {n} files to {args.out}")


if __name__ == "__main__":
    main()
