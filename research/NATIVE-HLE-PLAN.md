# Native HLE plan: what still emulates hardware on the game thread

Goal (user): the game should run natively on the Rust/Bevy side, not as an emulation thread, with plenty of headroom on low-end CPUs. The recompiled PPC core is native x64 code and stays. This plan covers the hardware-model layers around it that still cost time.

## Measured cost (2026-11-08)

Release build, AX HLE on, intro cutscene, sampled 15 s at 38–53 s after start (~965 samples). The emulation thread runs at 0.95–0.99 CPU-s/s, so it is CPU-bound. Bench mode (`MW_BENCH_UNTHROTTLED=1`, which skips idle) presents frames no faster: 1,500 frames in 54 s against 55 s in normal mode. There is no idle left to remove; only throughput gains help.

Share of emulation-thread time:

| layer | share | notes |
|---|---|---|
| GX: C++ FIFO parse (`mwgx::Fifo::parse/vertex_size`) + Rust GX CPU work (`decode_vertex`, texgen, `build_vertex_format`, `drain_buf`, depth plane) | ~21% Rust + ~2.5% C++ | owned by gpu-renderer (GX SDK HLE / GPU vertex processing) |
| per-store dirty-page tracking (`native_memory_written`) | ~13% (bench ~24%) | coherence metadata for the GX texture cache; **fast path done**, see below |
| `VCRUNTIME140` memcpy/memset (Rust side) | ~11–15% | callers not yet resolved reliably (stale PDB in one run); suspected GX transport and vertex buffers |
| paired-single quantize helpers (`ppc_dequant`/`ppc_quant`/`psq_load`/`psq_store`) | ~5–6% | recompiler helpers, not hardware |
| guest game code (recompiled functions proper) | ~19% | the actual game; stays |
| hardware poll from C++ into Rust (`mw_dsp_tick` every 0.1 ms: AI DMA, VI half-lines, residual DSP) + Rust scheduler | ~3–5% (part of "other Rust") | |
| `ppc_poll` (every guest function entry) + `native_host_poll` | ~4–6% | **inline fast path done** (core-fast4) |
| timebase reads (`QueryPerformanceCounter` via `steady_clock`) | ~1–2% | |
| interrupt delivery by C++ exception (`OSInterruptReturn` throw/catch, `RtlUnwindEx`) | ~1% | |
| indirect-call lookup (`ppc_lookup` binary search, then the RSO search) | ~2–4% in bench | **cache done** |
| per-call `getenv` (trace wrappers, idle loop, input, DI, BT) | ~1.6% | **done** |

## Inventory and ranking

Ranked by cost × (1 / effort). "Exact" means the guest sees identical results; anything else must be verified the AX way: shadow-render, compare per frame, env fallback.

1. **GX** (largest): FIFO and register-level emulation, CPU vertex decode, texgen. Owner: gpu-renderer (GX SDK hooks, GPU vertex processing). Not in this plan's scope.
2. **Dirty-page tracking per guest store** (~13%):
   - Done: force-inlined single-page fast path with identical generations, plus force-inlined `st*`/`ld*` accessors (MSVC had left `native_memory_written`, then `st32`/`st64`, out of line).
   - Next, exact: bump the generation counter only when the texture cache observes it (`range_generation`), instead of on every store. Write-detection semantics are unchanged.
   - Long term: once GX textures are registered at the SDK level (`GXInitTexObj*`/`GXLoadTexObj` plus the game's own `DCFlushRange` calls), per-store tracking could go entirely. That needs gpu-renderer's GX HLE.
3. **memcpy/memset on the Rust side** (~11–15%): resolve the callers with a matching PDB, then remove copies (borrowed slices instead of `to_vec`, reusable buffers). Low effort once attributed.
4. **Paired-single quantize helpers** (~5–6%): recompiler work (specialize `psq_l`/`psq_st` on the GQR type at the call site, or cache the decoded GQR). Not a hardware layer, but cheap and exact. Needs a regen.
5. **Rust hardware poll at 10 kHz** (~3–5%): with AX HLE the DSP needs no polling. Poll at the next real deadline (AI DMA completion or interrupt) instead of every 0.1 ms; VI half-lines are already batch-safe (gecko test `long_native_poll_preserves_all_video_deadlines`). Medium effort, needs a Rust `next_deadline` export. Exact for AI/DSP interrupt timing.
6. **Interrupt polling and delivery** (`ppc_poll` ~3%, exception unwind ~1%): replace the C++ `throw OSInterruptReturn` with a non-exception return path. Medium effort, and touches the fiber/interrupt core (snapshot-sensitive).
7. **Timebase QPC** (~1–2%): read once per poll step (or via TSC) with identical deadline ordering. Low gain.
8. **VI**: retrace is a timebase event; presentation already goes to the Bevy renderer. A Bevy-vsync-driven retrace is a pacing change, not a CPU win. Defer.
9. **DVD/DI, IOS/NAND/FS/ES**: already host file reads behind IOS IPC (DI modelled at 32 MB/s + 0.6 ms per request, faster than a real drive). Boot-time only; no measurable steady-state cost. HLE at `DVDRead`/`DVDOpen` would mostly shorten loading. Low priority.
10. **EXI/SI**: negligible in samples.

## Compiler-level (native code quality, not HLE)

- **`/fp:precise` vs `/fp:strict`:** byte-identical on 14,645 synthetic FPU cases (both match the recorded Unicorn output). The integer difftest is unchanged. The game never changes the FP environment. Tests keep `/fp:strict`, because they exercise `fesetround`.
- **`/GL` + `/LTCG`:** built and tested, but the first measurement used an older source tree and looked slower. Being re-measured on the current tree (`core-fast2pl`).
- **`/arch:AVX2`:** not adopted, since low-end CPUs (Pentium/Celeron before Ice Lake) lack AVX2. The CRT `fma` already dispatches to FMA3 at runtime.
- **PGO:** pending.
- `build.py` now takes `--build-dir`, `--fp`, `--ltcg` and `--arch` for experiments. Default command lines are byte-identical to before, so there are no needless ninja rebuilds.

## Paused 2026-11-08: remaining performance work (user: speed and audio are good for now)

Status when paused:
- AX HLE is on by default. Frame-exact against the DSP interpreter; `MW_DSP_INTERPRETER=1` falls back.
- Core-thread overhead items 2, 6 (poll) and the lookup/getenv fixes are done and in the tree.
- The frame-rate-wait HLE (`runtime/game/frame_wait.cpp`) is in the tree but **opt-in** (`MW_FRAME_WAIT_HLE=1`) until its A/B shows unchanged frame cadence; see `research/PERF-AUDIO.md`.

Next, in priority order (all exact semantics, each behind an env fallback, measured as share of thread time plus time-to-N-frames, because per-second fps is ±13% noise):

1. **Frame-wait HLE → default-on.**
   - The first version slept through the whole wait. That freed ~0.5 core but cost 10–20% of frames, because the game adapts its frame rate to how late frames start, and the high-resolution timer oversleeps by up to 1.5 ms.
   - The current version sleeps only while the next VI retrace is more than 3 ms away and spins like the original near it. A/B result: CPU 0.99 → 0.57 CPU-s/s, percentiles mostly match, but it still presents fewer frames in most pairs, so it stays opt-in. Next idea: wake on an event signalled by the VI retrace handler (no timer oversleep), then A/B with more pairs on a quiet machine.
2. **Paired-single quantize helpers** (`ppc_dequant`/`ppc_quant`/`psq_*`, ~7.7%): specialize on the GQR type at recompile time, or cache the decoded GQR. Needs a regen; difftest plus `quant_scale` must pass.
3. **`fma`** (~2.6%): the UCRT call already dispatches to FMA3, so inline a runtime-dispatched `_mm_fmadd_sd`. Bit-identical. Needs an emitter change and a regen.
4. **Rust-side memcpy/memset** (~10%): build a profiling frontend with `/OPT:NOICF` (or symbols that survive identical-code folding) to attribute callers, then remove copies. Known: `mw_gx_command` copies every command packet into a `Vec` (~1%).
5. **Dirty tracking, step 2** (exact): bump the page generation only when the GX texture cache observes it (`RamView::range_generation`), not per store.
6. **1 ms AI clock event** (`hw/ai.cpp`): schedule at the exact AI interrupt (AIINT) or sample-counter deadline instead of a periodic tick; combine with the `mw_dsp_poll_hint` deadline poll (already in the tree), so the idle path wakes only for real deadlines.
7. **Interrupt return without C++ exceptions** (`throw OSInterruptReturn`, ~1%); snapshot-sensitive.
8. **Timebase QPC reads** (~1–2%): read once per poll step or via TSC, with identical deadline ordering.
9. **Remove the remaining emulated hardware** (user goal: native, not emulated; ~0 CPU today):
   - VI retrace and pacing driven from the Bevy frame or vsync;
   - DVD/DI and NAND/FS as direct host-file SDK HLE at `DVDOpen`/`DVDRead`, which also shortens loading;
   - EXI/SI stubs at the SDK level.
10. **Compiler:** `/fp:precise` is verified bit-identical but not adopted. LTCG showed no clear gain and takes 30 min to build. PGO trained on the boss intro is untried.

How to measure (already in the tree):
- `MW_PERF_LOG=<file>`: per-second counters, including frameP50/P99.
- `MW_PROFILE_CODE_MAP=<file>`: lets the sampler find the core thread.
- `MW_BENCH_UNTHROTTLED=1`: skip idle waits.
- `MW_CORE_DLL=<path>`: load an experimental core.
- `build.py --build-dir/--fp/--ltcg/--arch`: experimental core builds.
- `tests/difftest.py`: reports `hle.txt` replacements separately.
