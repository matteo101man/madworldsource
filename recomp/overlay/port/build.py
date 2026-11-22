#!/usr/bin/env python3
"""Build the PC port: recompiled code + runtime -> madworld.exe and difftest.exe.

Usage:
    python port/build.py [--cxx PATH] [--recomp] [--debug]

Compiler, in order of preference:
    --cxx, $CXX, build/tools/llvm-mingw/bin/clang++.exe, clang++/g++ on PATH, cl.exe on PATH.
Owned-stack runtime currently requires MSVC-style cl/clang-cl with the static
MSVC CRT and ml64. GNU-style flag generation below is retained for reference.
Writes port/build/build.ninja and runs ninja (pip install ninja).
"""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PORT = ROOT / "port"
BUILD = PORT / "build"
GEN = PORT / "generated"
RSOGEN = PORT / "generated-rso"

INCLUDES = [PORT / "runtime/cpu", PORT / "runtime/os", PORT / "runtime/hw", GEN, RSOGEN]
# executable -> its own main source; everything in runtime/ is shared
EXES = {"madworld.exe": "app/main.cpp", "difftest.exe": "tests/difftest.cpp"}


def find_cxx(explicit):
    for cand in (explicit, os.environ.get("CXX"),
                 ROOT / "build/tools/llvm-mingw/bin/clang++.exe",
                 shutil.which("clang++"), shutil.which("g++"), shutil.which("cl")):
        if cand and Path(cand).exists():
            return Path(cand)
    sys.exit("No C++ compiler found. Install LLVM/clang, llvm-mingw or Visual Studio Build Tools, "
             "or pass --cxx.")


def ninja_escape(p):
    # Paths relative to the build dir: the repo path contains spaces, and ninja
    # doesn't quote $in/$out when it builds command lines.
    rel = os.path.relpath(p, BUILD).replace("\\", "/")
    if " " in rel:
        sys.exit(f"path with spaces inside the repo is not supported: {rel}")
    return rel.replace("$", "$$")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--cxx")
    ap.add_argument("--recomp", action="store_true", help="re-run the recompiler first")
    ap.add_argument("--debug", action="store_true")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--build-dir", type=Path, help="alternate output directory (optimization experiments); default port/build")
    ap.add_argument("--fp", choices=("strict", "precise"), default="strict",
                    help="game/runtime FP model; tests always use /fp:strict (they exercise fesetround)")
    ap.add_argument("--ltcg", action="store_true", help="MSVC whole-program optimization (/GL + /LTCG)")
    ap.add_argument("--arch", choices=("none", "AVX2"), default="none", help="MSVC /arch for game/runtime code")
    ap.add_argument("--dol", type=Path, help="owned DOL path for generation; otherwise reuse generation manifest")
    args = ap.parse_args()
    global BUILD
    if args.build_dir:
        BUILD = args.build_dir.resolve()

    recomp_inputs = [PORT / "recomp/recomp.py", PORT / "recomp/ppc_decode.py", PORT / "recomp/hle.txt",
                     ROOT / "config/RZZE8P/symbols.txt"]
    stamp_path = GEN / "generation.json"
    stamp = json.loads(stamp_path.read_text(encoding="utf-8")) if stamp_path.exists() else {}
    dol_path = args.dol or Path(stamp.get("dol", ROOT / "orig/RZZE8P/sys/main.dol"))
    recorded = stamp.get("inputs", {})
    stale = not (GEN / "funcs.h").exists() or not recorded or any(
        not Path(p).exists() or hashlib.sha256(Path(p).read_bytes()).hexdigest() != digest
        for p, digest in recorded.items()) or str(dol_path.resolve()) not in recorded
    if args.recomp or stale:
        subprocess.run([sys.executable, str(PORT / "recomp/recomp.py"), "--dol", str(dol_path)], check=True)

    rso_stamp_path = RSOGEN / "generation.json"
    if not rso_stamp_path.exists():
        sys.exit("Generate the owned enemy module first with port/recomp/rso_recomp.py --export PATH --analysis PATH")
    rso_stamp = json.loads(rso_stamp_path.read_text(encoding="utf-8"))
    if not rso_stamp.get("inputs"):
        sys.exit("Regenerate RSO once to record generation inputs")
    if any(not Path(p).exists() or hashlib.sha256(Path(p).read_bytes()).hexdigest() != digest
           for p, digest in rso_stamp["inputs"].items()):
        subprocess.run([sys.executable, str(PORT / "recomp/rso_recomp.py"), "--export", rso_stamp["export"],
                        "--analysis", rso_stamp["analysis"]], check=True)

    subprocess.run([sys.executable, str(PORT / "recomp/rso_fixture.py")], check=True)

    cxx = find_cxx(args.cxx)
    msvc = cxx.stem.lower() in ("cl", "clang-cl")
    if not msvc:
        sys.exit("Owned-stack runtime requires cl/clang-cl with the static MSVC CRT; use tools/build_native_msvc.cmd")
    inc = " ".join(("/I" if msvc else "-I") + f'"{p}"' for p in INCLUDES)
    if msvc:
        opt = "/Od /Zi" if args.debug else "/O2"
        # Defaults reproduce the historical command lines exactly (no needless ninja rebuilds).
        common = f"/nologo /std:c++20 /EHsc /fp:{args.fp} {inc} " + opt             + (" /GL" if args.ltcg else "") + (f" /arch:{args.arch}" if args.arch != "none" else "")
        gen_flags = common + " /w /bigobj"
        rt_flags = common + " /W3"
        test_flags = f"/nologo /std:c++20 /EHsc /fp:strict {inc} " + opt + " /W3"
        obj, exe_flags = ".obj", ""
        compile_cmd = f'"{cxx}" $flags /showIncludes /c $in /Fo$out'
        dependencies = ["  deps = msvc"]
        link_cmd = f'"{cxx}" /nologo $in /Fe$out'
    else:
        # -ffp-contract=off: only PowerPC's own fused ops (emitted as std::fma) may fuse.
        common = (f"-std=c++20 -march=x86-64-v3 -ffp-contract=off -fno-strict-aliasing {inc} "
                  + ("-O0 -g" if args.debug else "-O2"))
        gen_flags = common + " -w"
        rt_flags = common + " -Wall"
        test_flags = rt_flags
        obj, exe_flags = ".o", " -static"
        compile_cmd = f'"{cxx}" $flags -MMD -MF $out.d -c $in -o $out'
        dependencies = ["  deps = gcc", "  depfile = $out.d"]
        link_cmd = f'"{cxx}" $in -o $out{exe_flags}'

    gen_srcs = sorted(GEN.glob("recomp_*.cpp")) + [GEN / "func_table.cpp"]
    gen_srcs += sorted(RSOGEN.glob("rso_recomp_*.cpp")) + [RSOGEN / "rso_func_table.cpp"]
    rt_srcs = sorted((PORT / "runtime").rglob("*.cpp"))
    assembler = shutil.which("ml64") or str(cxx.parent / "ml64.exe")
    if not Path(assembler).exists():
        sys.exit("Owned x64 guest stacks require ml64.exe (run from the x64 MSVC developer environment)")
    BUILD.mkdir(parents=True, exist_ok=True)
    lines = [
        "rule cxx_gen", f"  command = {compile_cmd.replace('$flags', gen_flags)}", "  description = CXX $in", *dependencies,
        "rule cxx_rt", f"  command = {compile_cmd.replace('$flags', rt_flags)}", "  description = CXX $in", *dependencies,
        "rule cxx_test", f"  command = {compile_cmd.replace('$flags', test_flags)}", "  description = CXX $in", *dependencies,
        "rule link", f"  command = {link_cmd}", "  description = LINK $out",
        "rule asm", f'  command = "{assembler}" /nologo /c /Fo$out $in', "  description = ASM $in",
    ]
    if msvc:
        lines += ["rule link_dll",f'  command = "{cxx}" /nologo /LD $in /Fe$out /link /INCREMENTAL:NO /DYNAMICBASE:NO /BASE:0x1A0000000 /ENTRY:mw_native_dll_entry /MAP:madworld_core.map' + (" /LTCG" if args.ltcg else ""),"  description = DLL $out"]
    funcs_h = ninja_escape(GEN / "funcs.h")
    shared = []
    fiber_obj = BUILD / "obj/runtime/os" / ("fiber_x64" + obj)
    lines.append(f"build {ninja_escape(fiber_obj)}: asm {ninja_escape(PORT / 'runtime/os/fiber_x64.asm')}")
    shared.append(fiber_obj)
    for s in gen_srcs:
        o = BUILD / "obj/generated" / (s.stem + obj)
        dependency = ninja_escape((RSOGEN if s.parent == RSOGEN else GEN) / ("rso_funcs.h" if s.parent == RSOGEN else "funcs.h"))
        lines.append(f"build {ninja_escape(o)}: cxx_gen {ninja_escape(s)} | {dependency}")
        shared.append(o)
    for s in rt_srcs:
        o = BUILD / "obj" / s.relative_to(PORT).with_suffix(obj)
        lines.append(f"build {ninja_escape(o)}: cxx_rt {ninja_escape(s)} | {funcs_h}")
        shared.append(o)
    for exe, main_src in EXES.items():
        s = PORT / main_src
        o = BUILD / "obj" / Path(main_src).with_suffix(obj)
        lines.append(f"build {ninja_escape(o)}: cxx_rt {ninja_escape(s)} | {funcs_h}")
        lines.append(f"build {ninja_escape(BUILD / exe)}: link {ninja_escape(o)} "
                     + " ".join(ninja_escape(x) for x in shared))
    if msvc:
        embedded_src = PORT / "app/embedded.cpp"
        embedded_obj = BUILD / "obj/app/embedded.obj"
        lines.append(f"build {ninja_escape(embedded_obj)}: cxx_rt {ninja_escape(embedded_src)}")
        lines.append(f"build madworld_core.dll: link_dll {ninja_escape(embedded_obj)} {ninja_escape(BUILD / 'obj/app/main.obj')} " + " ".join(ninja_escape(x) for x in shared))
    for test in ("gx_transport", "hci_protocol", "ai_clock", "irq_snapshot", "hardware_events", "ipc_transport", "rso_relocations", "native_unwind", "di_async", "nand_fs", "input_file", "dirty_tracking", "quant_scale", "thread_reuse", "host_fiber", "pc_ui_overlay"):
        transport_src = PORT / ("tests/" + test + ".cpp")
        transport_obj = BUILD / "obj/tests" / (test + obj)
        lines.append(f"build {ninja_escape(transport_obj)}: cxx_test {ninja_escape(transport_src)}")
        extra = []
        if test in ("irq_snapshot", "hardware_events"):
            extra = [BUILD / "obj/runtime" / Path(p).with_suffix(obj) for p in
                     ("os/interrupts.cpp", "hw/mmio.cpp", "cpu/ppc_memory.cpp")]
        if test == "dirty_tracking":
            extra = [BUILD / "obj/runtime/cpu" / ("ppc_memory" + obj)]
        if test in ("di_async", "nand_fs"):
            extra = [BUILD / "obj/runtime/cpu" / ("ppc_memory" + obj)]
        if test in ("native_unwind", "thread_reuse"):
            extra = [BUILD / "obj/runtime/cpu" / ("ppc_memory" + obj)]
        if test in ("thread_reuse", "host_fiber"):
            extra += [fiber_obj, BUILD / "obj/runtime/os" / ("host_fiber" + obj)]
        if test == "rso_relocations":
            extra = [BUILD / "obj/runtime" / Path(p).with_suffix(obj) for p in ("cpu/rso.cpp", "cpu/ppc_memory.cpp")]
        if test == "ipc_transport":
            extra = [BUILD / "obj/runtime" / Path(p).with_suffix(obj) for p in
                     ("ios/ipc.cpp", "hw/mmio.cpp")]
        lines.append(f"build {test}.exe: link {ninja_escape(transport_obj)} " + " ".join(ninja_escape(p) for p in extra))
    (BUILD / "build.ninja").write_text("\n".join(lines) + "\n")
    print(f"compiler: {cxx}")
    subprocess.run([sys.executable, "-m", "ninja", "-C", str(BUILD), "-j", str(args.jobs)], check=True)


if __name__ == "__main__":
    main()
