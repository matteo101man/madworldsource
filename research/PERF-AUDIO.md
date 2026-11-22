# Performance and audio (2026-11-08)

User report: windowed `madworld-native.exe` had laggy, crackly, wrongly pitched sound; the game was laggy; the intro cutscene played very slowly.

## How it was measured

- `MW_PERF_LOG=1` (stderr) or `MW_PERF_LOG=<file>` turns on `crates/mw-game/src/native_perf.rs`. It prints one JSON line per second:
  - `guestFps`: guest `PresentXfb`/`PresentRawXfb` actions per wall second.
  - `gxSinkMsPerSec`: time the core thread spends in the GX render sink.
  - `gxBlockingMsPerSec`: time spent in synchronous EFB peeks or copies.
  - `aidProducedHz`: AID frames the core produces.
  - `sourceConsumedHz`: AID frames the resampler consumes.
  - `deviceOutputHz`: frames handed to the audio device.
  - `underrunFrames`, `droppedFrames`, `queuedFrames`.
- With the option unset, nothing is wrapped and only a few relaxed atomics remain.
- CPU breakdown: core-thread RIP sampling by module, with the TID taken from `MW_PROFILE_CODE_MAP`. Rust frames were resolved with `llvm-symbolizer` (the release profile now emits line tables), ntdll frames from its export table.
- The guest time base is wall-clock (`ppc_timebase`), so AID output stays real-time even when frames drop. When the CPU falls behind, the game runs fewer frames per wall second, which looks like slow motion.

## Audio: root cause and fix

**Root cause:** `NativePcm`'s rodio `Source` returned `current_span_len()` = 1 or 2 as a "remaining samples" countdown while switching `sample_rate()` per sample. In rodio 0.22, `current_span_len` is the *total* span size. Rodio therefore never converted the 32 kHz AID stream: the device pulled 48,000 frames/s from a source producing 32,000. Measured baseline:

| | aidProducedHz | device pull | underrun (zero) frames/s |
|---|---|---|---|
| before | 32 000 | 47 950–47 990 | ~16 000 |

The result was playback 1.5× too fast (wrong pitch), with a third of the frames being inserted zeros (crackle).

**Fix** (`crates/mw-game/src/native_audio.rs`):
- The source declares one constant 48 kHz stereo stream (`current_span_len` = `None`).
- AID frames, each tagged 32 or 48 kHz, are linearly resampled inside the source, in 256-frame chunks per lock.
- A 60 ms jitter buffer is primed before playback and re-primed after an underrun, which outputs silence rather than interleaved zeros.
- A bounded ±0.5% step trim, proportional to the smoothed queue fill, absorbs drift between the device clock and the host clock. That is well under any audible pitch change, and guest timing is untouched.
- The queue bound dropped from 2 s to 250 ms, so latency can't creep.

After the fix (release build, 60 s from boot through the cutscene):

| | aidProducedHz | sourceConsumedHz | deviceOutputHz | underruns | queue |
|---|---|---|---|---|---|
| after | 31 990–32 010 | 31 880–32 230 | 47 820–48 330 | 0 | 1 650–2 070 frames (~55 ms), stable |

The ±0.3% swing between seconds is the 1 s window aliasing against 256-frame chunks. The averages match: consumed equals produced, and 32 k × 1.5 = 48 k. A single underrun burst appeared only while the sampling profiler was suspending the core thread.

Unit tests (pure CPU), all passing:
- a 1 kHz tone at 32 kHz comes out at 1000 ± 5 Hz at 48 kHz, with stereo kept separate and amplitude preserved;
- 48 kHz passes through 1:1;
- priming and underrun produce silence;
- the 250 ms latency bound holds.

## Frame rate: findings

The guest frame rate is the game's own (30 fps during boot/loading). It falls to 12–17 fps in the intro cutscene: the emulation thread is saturated, not waiting on the GPU (`gxBlockingMsPerSec` = 0, GX sink ≈ 1 ms per frame).

| build | boot/loading | intro cutscene |
|---|---|---|
| dev profile (opt 3 + overflow checks/debug asserts), before | 30 | 12–14, dips to 4 |
| release profile | 30 | 14–17, dips to 9–10 |

Release-build core-thread samples during the cutscene (975–1 161 samples):
- 29–33% `madworld_core.dll`, the recompiled game.
- ~20% gecko GX CPU-side work: `decode_vertex`, `compute_texgen`/`apply_*texgens`, `build_vertex_format`, `drain_buf`, `DepthPlane::update_depth`. This is software vertex decoding and texgen running on the emulation thread.
- ~12% DSP interpreter (`drain_dsp_synchronous_interp`, `step_dsp_instruction`).
- ~12% stderr I/O (`ZwWriteFile` 6.5% plus `ZwDeviceIoControlFile` 5%). It came from 13 000–20 000 `gecko::gx::unimplemented` WARN lines per second, formatted and written synchronously from the emulation thread (525 000 lines in 60 s). In a real console window, the user's case, this costs far more than with a file.
- ~2% `RtlQueryPerformanceCounter` (wall-clock time base), plus a small amount of C++ unwind for interrupt returns.
- The dev-build profile seemed to put Bluetooth IPC near the top. That was symbol aliasing in the dev build; it does not show up in the release profile.

## Changes made

1. Audio source rewritten (above).
2. `crates/mw-game/src/native_main.rs`: the default `LogPlugin` filter is now `DEFAULT_FILTER,gecko::gx::unimplemented=error`. `RUST_LOG` still overrides it, e.g. `RUST_LOG=gecko::gx::unimplemented=warn` brings the warnings back.
3. `native-platform/src/lib.rs`: `mw_dsp_tick` cached its `MW_TRACE_DSP` environment lookup. Before, it ran on every DSP sync (each 0.1 ms event and each DSP register access).
4. Workspace `Cargo.toml`: `[profile.release] debug = "line-tables-only"`, for symbolized profiles; no codegen change. Play the `--release` build: it is faster than dev, which keeps overflow checks in the gecko DSP/GX loops.

No animation multiplier, clock scale, draw dropping or change to core logic.

## DSP: high-level AX mixer (replaces the interpreter)

`tools/gecko-reference/crates/gecko/src/flipper/dsp/hle.rs` is a Rust port of Dolphin's AXWii HLE. The reference is pinned at Dolphin commit `257a6d0e`, in `tools/dolphin-ax-reference/`.

How it works:
- **Hand-over:** the interpreter still runs the IROM boot, the microcode upload and the AX init handshake. The mixer takes over at the first command-list mail (`0xBABExxxx`), while the microcode sits in its mailbox wait.
- **Mixing:** from then on it renders each command list directly: setup, add/sub to LR, voice processing, aux A/B/C mix and upload, compressor, output, and the Wii Remote buses. Voice processing covers ADPCM/PCM decoding through an accelerator model, polyphase/linear sample-rate conversion using the DSP coefficient ROM, the volume envelope, low-pass and biquad filters, and mixer ramps.
- **Protocol:** it answers the same mailbox/interrupt sequence the game sees (`DSP_SYNC`, `DSP_YIELD`, resume/continue).
- **Reset and snapshots:** a DSP reset hands control back to the interpreter. Mixer state is stored as a tagged trailer after the hardware snapshot; older snapshots without it load and continue on the interpreter.
- **Microcode:** `HashEctor` = `4cc52064`, Dolphin's newest AXWii layout (no updates field, full biquad).
- **Opt-out:** `MW_DSP_INTERPRETER=1` or `MW_AX_HLE=0`.

**Verification** (`MW_AX_HLE_VERIFY=1`): the interpreter stays authoritative, and the mixer renders every command list into a shadow write log. When the interpreter posts `DSP_YIELD`, each shadow write is compared byte for byte with guest memory. Over a release run from boot through the boss intro:

| AX frames compared | stereo output frames byte-identical | mismatched output samples | aux/surround/remote mismatches | voice-block words differing |
|---|---|---|---|---|
| 19 500 (~65 s) | 19 500 | 0 (max diff 0) | 0 | 2 voice-frames, aux-B anti-pop words 43/47 only (no output effect) |

**Core thread, release build, intro cutscene** (sampled, about 960 samples each, same build and core):

| | DSP interpreter | AX HLE | all Rust code | guest fps, same cutscene window |
|---|---|---|---|---|
| interpreter (verify run) | 18.7% | 2.2% (shadow) | 48% | 12–20 |
| AX HLE | 0.2% | 0.7% | 33% | 21–33, late scene 30–35 |

Evidence:
- `evidence/ax-hle-verify.log`: the per-500-frame verification lines.
- `evidence/ax-hle-verify.perf.jsonl` and `evidence/ax-hle-run.perf.jsonl`: per-second counters for the two runs.
- `evidence/ax-hle-profile.json`: module and category split plus fps series.
- `evidence/ax-hle-profile-{interpreter,hle}.rips.json`: raw samples.

Audio with the mixer: AID 32 000 Hz, device output 48 000 Hz, 0 underruns in steady state. The only underruns are the startup priming and a ~0.1 s scene-load stall, both also present with the interpreter.

Unit tests (`native-platform/src/ax_hle_tests.rs`):
- `HashEctor`;
- linear resampler passthrough with the 3-sample window delay;
- polyphase DC gain;
- mixer-control ramp bits;
- layout span sizes.

All 34 native-platform tests pass.

## Emulation-thread CPU optimisation (exact semantics)

Measured on the release build with AX HLE, intro cutscene, core-thread RIP samples (15 s at 38–53 s). The thread runs at 0.95–0.99 CPU-s/s, so it is purely CPU-bound. `MW_BENCH_UNTHROTTLED=1` (diagnostic: idle waits skip ahead in guest time) presents frames no faster: 1,500 frames in 54 s against 55 s in normal mode.

Per-second fps varies ±13% between identical runs, because cutscene content and timing shift. The robust measures are the share of thread time per item, and the wall time to present N frames.

| change | file | before | after |
|---|---|---|---|
| dirty-page tracking: force-inlined single-page fast path, identical generations | `cpu/native_dirty.h` | `native_memory_written` 13% of thread (24% in bench) | 0% |
| guest load/store accessors force-inlined (MSVC left `st32`/`st64` out of line in the huge generated functions) | `cpu/ppc_context.h` | `st32`+`st64` ~5% | 0% |
| indirect-call cache keyed by address + RSO base | `cpu/ppc_runtime.cpp`, `rso.cpp` | `ppc_lookup`+`mw_rso_lookup` ~4% (bench) | 0.6% |
| `ppc_poll` (every guest function entry) inline fast path; interrupt-delivery cadence unchanged | `cpu/ppc_context.h`, `ppc_runtime.cpp` | `ppc_poll`+`native_host_poll` 4–6% | 0% |
| frozen options cached per call site (`NATIVE_OPTION`): trace wrappers, idle loop, input, DI, BT | `cpu/native_config.h` + callers | `getenv` ~1.6% | 0% |
| idle wait: high-resolution waitable timer instead of MSVC `sleep_for` (which rounds up to ≥1 ms, or 15.6 ms at default timer resolution) | `os/interrupts.cpp` | — | no regression measured; first frame, fps and boot timeline unchanged |
| deadline-driven hardware poll with AX HLE (`mw_dsp_poll_hint`: poll at the next AI/ARAM DMA interrupt, bounded to 0.1–1 ms) | `hw/dsp.cpp`, native-platform | — | built and tested; in the current tree |

Result:
- Recompiled game code went from **19% to 30%** of the thread; the rest of the gain is removed overhead.
- Time to present 1,200 frames: 42 s on the 14:16 shared core against 38–40 s on core-fast4.
- Correctness for every variant: all 15 core test exes pass, and the Unicorn integer difftest gives 11,701/11,701 eligible cases. `tests/difftest.py` now reports `hle.txt`-replaced functions separately.
- `/fp:precise` is bit-identical on 14,645 synthetic FPU cases but not adopted yet. `/GL`+`/LTCG` (core-fast2pl) showed no clear gain within the noise and takes 30 min to build, so it isn't the default.

Evidence:
- `evidence/cpu-opt-*.rips.json`: raw samples.
- `evidence/cpu-opt-*.perf.jsonl`: per-second counters.
- The ranked inventory and next items are in `research/NATIVE-HLE-PLAN.md`.

Next measured costs: paired-single quantize helpers ~7.7%, CRT `fma` call ~2.6% (UCRT already dispatches to FMA3), GX FIFO parse ~2% (gpu-renderer), Rust-side `memcpy` ~10% (attribution still unreliable under identical-code folding).

## Frame-rate wait HLE (fn_8026F324), opt-in

With `MW_GX_DL_BATCH=1` the game reaches its frame cap, and its frame wait spins on `ppc_poll` until the VI retrace counter (`0x806A4174`) reaches the frames-per-update target (`0x805CBE20`). That spin was ~33% of the core thread.

`runtime/game/frame_wait.cpp` keeps the original's prologue, call, per-iteration poll, compare, CR0, registers and counter reset. When the count is short and interrupts are deliverable, it delivers due events and sleeps until the next hardware deadline. Variants were A/B-tested in the same DLL, interleaved, over a 45–50 s window:

| variant | core CPU-s/s | frames | p50 | p90 | p99 | underruns (steady) |
|---|---|---|---|---|---|---|
| original spin | 0.98–1.00 | 2,253 / 2,380 / 2,494 / 1,760* | 17.0 ms | 33 ms | 50 ms | 0 |
| sleep through the wait | 0.47–0.59 | 2,024 / 2,212* | 17.0–17.1 ms | 34 ms | 50.7 ms | ~0 |
| hybrid: sleep only while the next retrace is >3 ms away | 0.57 | 1,878 / 1,730 | 17.0–17.5 ms | 34–49 ms | 50.6 ms | 0 |

\* Build activity on the machine during that run.

- It frees ~0.4–0.5 core.
- The percentiles mostly match, but it presents fewer frames in most pairs. The game adapts its frame rate to how late frames start, so wake-up latency matters: the high-resolution waitable timer has ~0.5 ms resolution and oversleeps 0.5–1.5 ms.
- It therefore stays **opt-in** (`MW_FRAME_WAIT_HLE=1`), and the original spin is the default.
- Next: wake on an event signalled by the VI retrace handler instead of a timed sleep, and A/B on a quiet machine with more pairs.
- Evidence: `evidence/frame-wait-*.perf.jsonl`.

## Remaining gap to 30 fps in the cutscene (not done)

The cutscene needs about 2× more throughput on the emulation thread. The measured candidates, largest first:
- **GX CPU work (~20%):** move vertex decode and texgen off the emulation thread (pipeline the decoder behind the command FIFO), or onto the GPU. See the cautions in `PERFORMANCE-NEXT.md` on ordering and shared-memory barriers.
- **DSP:** done; see the AX mixer section above.
- **Recompiled core (~30%):** unchanged here.

## Console-window measurement

To be added once the core DLL builds and boots again: the DLL from 10:04 (sha `ac66410a…`) exits about 12 s into boot with "truncated GX display list command" (a parallel core change, under investigation by its sessions).
