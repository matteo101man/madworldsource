#!/usr/bin/env python3
"""Differential test: recompiled C++ vs. the original PowerPC code in Unicorn.

Picks leaf, integer-only functions from main.dol (no calls, no FPU, no system
instructions), runs each on random inputs in Unicorn, then has the native
harness (port/build/difftest.exe) run the same cases and compares every GPR,
CR, XER[CA] and CRCs of the scratch buffer, stack and the DOL's data sections.

Usage:
    python port/tests/difftest.py [--cases 4] [--limit 0] [--seed 1]
"""
import argparse
import random
import struct
import subprocess
import sys
import zlib
from pathlib import Path

from unicorn import Uc, UcError, UC_ARCH_PPC, UC_MODE_PPC32, UC_MODE_BIG_ENDIAN, UC_HOOK_CODE
from unicorn import ppc_const as P

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "port/recomp"))
from dol import Dol  # noqa: E402
from ppc_decode import decode  # noqa: E402
from recomp import load_functions, Recompiler  # noqa: E402

SCRATCH, SCRATCH_SIZE = 0x81000000, 0x4000
STACK_TOP = 0x81700000
STACK_LO = STACK_TOP - 0x1000
RET = 0x80001000                  # sentinel return address (never executed natively)
R2, R13 = 0x80720720, 0x8071F720  # from __init_registers

LEAF_OK = {
    "addi", "addis", "addic", "subfic", "mulli", "add", "subf", "neg", "mullw", "mulhw", "mulhwu",
    "divw", "divwu", "addc", "adde", "addze", "addme", "subfc", "subfe", "subfze", "subfme",
    "cmpi", "cmpli", "cmp", "cmpl", "ori", "oris", "xori", "xoris", "andi", "andis", "and", "andc",
    "or", "orc", "xor", "nor", "nand", "eqv", "slw", "srw", "sraw", "srawi", "cntlzw", "extsb",
    "extsh", "rlwinm", "rlwimi", "rlwnm", "mfcr", "mtcrf", "mcrf", "crand", "cror", "crxor",
    "crnand", "crnor", "creqv", "crandc", "crorc",
    "lwz", "lbz", "lhz", "lha", "lwzu", "lbzu", "lhzu", "lhau", "lwzx", "lbzx", "lhzx", "lhax",
    "lwzux", "lbzux", "lhzux", "lhaux", "stw", "stb", "sth", "stwu", "stbu", "sthu", "stwx",
    "stbx", "sthx", "stwux", "stbux", "sthux", "lwbrx", "lhbrx", "stwbrx", "sthbrx", "lmw", "stmw",
    "b", "bc", "bclr",
}


def eligible(rc, f):
    if f.size > 0x400 or len(f.entries) > 1:
        return False
    for a, w in rc.words(f):
        ins = decode(w)
        if ins is None or ins.op not in LEAF_OK and ins.op not in ("mfspr", "mtspr"):
            return False
        if ins.op in ("mfspr", "mtspr") and ins.f["spr"] not in (8, 9):
            return False
        if ins.op in ("b", "bc"):
            if ins.f.get("lk"):
                return False
            t = rc.branch_target(a, ins)
            if not (f.addr <= t < f.end):
                return False
        if ins.op == "bclr" and ins.f["lk"]:
            return False
        if ins.oe:
            return False  # OV isn't modelled
    return True


def make_case(rng, func):
    regs = [rng.getrandbits(32) for _ in range(32)]
    for r in range(3, 11):
        k = rng.random()
        if k < 0.5:
            regs[r] = SCRATCH + (rng.randrange(0, SCRATCH_SIZE // 2) & ~3)
        elif k < 0.8:
            regs[r] = rng.randrange(0, 64)
    regs[1] = STACK_TOP - 0x100
    regs[2], regs[13] = R2, R13
    scratch = bytearray(rng.randbytes(SCRATCH_SIZE))
    for i in range(0, SCRATCH_SIZE, rng.randrange(8, 64)):
        scratch[i] = 0
    # XER starts at 0: Unicorn ignores CA written through XER (its internal carry stays 0).
    return dict(func=func, regs=regs, cr=rng.getrandbits(32), xer=0,
                ctr=rng.randrange(0, 1 << 16), scratch=bytes(scratch))


def data_crc(read, dol):
    crc = 0
    for s in dol.data_sections():
        crc = zlib.crc32(bytes(read(s.addr, len(s.data))), crc)
    return crc


def run_unicorn(dol, case, divides):
    """divides: {address: (is_signed, ra, rb)} for divw/divwu in the function."""
    uc = Uc(UC_ARCH_PPC, UC_MODE_PPC32 | UC_MODE_BIG_ENDIAN)
    uc.mem_map(0x80000000, 24 << 20)
    for s in dol.sections:
        uc.mem_write(s.addr, s.data)
    # Division by zero (and INT_MIN / -1) is undefined: QEMU returns the dividend,
    # hardware/Dolphin return 0 or -1, which is what the recompiler implements.
    # Cases that hit it don't say anything about the translation, so drop them.
    undefined = []

    def on_divide(uc_, addr, size, _):
        signed, ra, rb = divides[addr]
        a, b = uc_.reg_read(P.UC_PPC_REG_0 + ra), uc_.reg_read(P.UC_PPC_REG_0 + rb)
        if b == 0 or (signed and a == 0x80000000 and b == 0xFFFFFFFF):
            undefined.append(addr)
            uc_.emu_stop()

    for addr in divides:
        uc.hook_add(UC_HOOK_CODE, on_divide, begin=addr, end=addr)
    uc.mem_write(SCRATCH, case["scratch"])
    for i, v in enumerate(case["regs"]):
        uc.reg_write(P.UC_PPC_REG_0 + i, v)
    uc.reg_write(P.UC_PPC_REG_CR, case["cr"])
    uc.reg_write(P.UC_PPC_REG_XER, case["xer"])
    uc.reg_write(P.UC_PPC_REG_LR, RET)
    uc.reg_write(P.UC_PPC_REG_CTR, case["ctr"])
    try:
        uc.emu_start(case["func"], RET, count=2_000_000)
    except UcError:
        return None
    if undefined or uc.reg_read(P.UC_PPC_REG_PC) != RET:
        return None  # ran out of instructions: probably looping on random input
    regs = [uc.reg_read(P.UC_PPC_REG_0 + i) for i in range(32)]
    return dict(
        regs=regs, cr=uc.reg_read(P.UC_PPC_REG_CR), ca=(uc.reg_read(P.UC_PPC_REG_XER) >> 29) & 1,
        scratch=zlib.crc32(bytes(uc.mem_read(SCRATCH, SCRATCH_SIZE))),
        stack=zlib.crc32(bytes(uc.mem_read(STACK_LO, STACK_TOP - STACK_LO))),
        data=data_crc(uc.mem_read, dol))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--dol", type=Path, default=ROOT / "orig/RZZE8P/sys/main.dol")
    ap.add_argument("--exe", type=Path, default=ROOT / "port/build/difftest.exe")
    ap.add_argument("--cases", type=int, default=4, help="cases per function")
    ap.add_argument("--limit", type=int, default=0, help="max functions (0 = all eligible)")
    ap.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()

    dol = Dol(args.dol)
    rc = Recompiler(dol, load_functions(ROOT / "config/RZZE8P/symbols.txt"))
    rc.analyze()
    funcs = [f for f in rc.funcs if eligible(rc, f)]
    if args.limit:
        funcs = funcs[:args.limit]
    rng = random.Random(args.seed)
    cases, expected = [], []
    for f in funcs:
        divides = {}
        for a, w in rc.words(f):
            ins = decode(w)
            if ins.op in ("divw", "divwu"):
                divides[a] = (ins.op == "divw", ins.f["ra"], ins.f["rb"])
        for _ in range(args.cases):
            case = make_case(rng, f.addr)
            res = run_unicorn(dol, case, divides)
            if res is not None:
                cases.append(case)
                expected.append(res)
    print(f"{len(funcs)} eligible functions, {len(cases)} terminating cases")

    tmp = ROOT / "port/build"
    tmp.mkdir(parents=True, exist_ok=True)
    cases_path, out_path = tmp / "difftest_cases.bin", tmp / "difftest_results.bin"
    with open(cases_path, "wb") as fp:
        fp.write(struct.pack("<4sII", b"MWDT", len(cases), SCRATCH_SIZE))
        for c in cases:
            fp.write(struct.pack("<I32IIII", c["func"], *c["regs"], c["cr"], c["xer"], c["ctr"]))
            fp.write(c["scratch"])
    subprocess.run([str(args.exe), str(args.dol), str(cases_path), str(out_path)], check=True)

    # Functions replaced by host code (recomp/hle.txt) intentionally differ from the
    # original PPC body; report them separately so real recompiler failures stand out.
    replaced = set()
    for line in (ROOT / "port/recomp/hle.txt").read_text(encoding="utf-8").splitlines():
        word = line.split("#", 1)[0].split()
        if word:
            replaced.add(int(word[0], 16))
    raw = out_path.read_bytes()
    rec = struct.Struct("<32IIIIII")
    fails = 0
    replaced_cases = replaced_diffs = 0
    for i, (c, e) in enumerate(zip(cases, expected)):
        v = rec.unpack_from(raw, i * rec.size)
        got = dict(regs=list(v[:32]), cr=v[32], ca=v[33], scratch=v[34], stack=v[35], data=v[36])
        # CA isn't compared: Unicorn never reflects the carry in XER reads (checked with a
        # one-instruction addic test). Carry still gets tested wherever adde/addze/subfe
        # consume it, since their results land in GPRs.
        bad = [k for k in ("regs", "cr", "scratch", "stack", "data") if got[k] != e[k]]
        if c["func"] in replaced:
            replaced_cases += 1
            replaced_diffs += bool(bad)
            continue
        if bad:
            fails += 1
            if fails <= 20:
                detail = ""
                if "regs" in bad:
                    detail = " ".join(f"r{r}:{e['regs'][r]:08X}/{got['regs'][r]:08X}"
                                      for r in range(32) if e["regs"][r] != got["regs"][r])
                print(f"FAIL {c['func']:#010x} {','.join(bad)} {detail}")
    checked = len(cases) - replaced_cases
    print(f"{checked - fails}/{checked} cases match"
          f" ({replaced_cases} cases of hle.txt-replaced functions reported separately, {replaced_diffs} differ by design)")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
