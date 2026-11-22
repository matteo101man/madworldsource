# GPU vertex pipeline and internal resolution (2026-11-08)

Goal: take vertex processing off the emulation (core) thread, make presentation
smooth, and raise resolution. The SDK-level replacement plan that follows from the
measurements below is in `GX-HLE.md`.

## What was built

**GPU vertex decode** (default on; `MW_GPU_VERTEX=0` restores the CPU decoder):
- On the core thread, gecko's `create_draw_batch` no longer decodes vertices. It
  copies the guest's raw FIFO vertex bytes into a word arena
  (`host::RawVertexStream`), along with each indexed attribute's `[min,max]` array
  range and the XF matrix memory. XF is re-snapshotted only after an XF write in
  0x000–0x13F, 0x400–0x46F or 0x500–0x60F.
- A draw becomes a 96-byte `GpuVertexJob`. It references a shared
  `GpuVertexFormat`: attribute modes, offsets and scales, texgen and dual-texgen
  setup, and default matrices. Formats are cached by their CP/XF register words,
  without array bases, because the game moves its arrays between frames.
- The render worker uploads the arena, the jobs and the formats. One WGSL compute
  pass (`backend-wgpu/src/shaders/vertex_decode.wgsl`) decodes, transforms, lights
  inputs and runs texgen into a shared `DrawVertex` buffer (164 B stride). Every
  pipeline binds that buffer directly, so the worker no longer repacks vertices on
  the CPU. TEV, lighting and the ubershader/async-specialization pipeline cache were
  already GPU-side and are unchanged.
- The depth plane for z-freeze still runs on the CPU. It decodes only the
  view-space positions of the triangles it inspects.
- Fallbacks: no position attribute, unmapped or >8 MB array ranges, or texture
  matrix indices beyond the snapshot are CPU-decoded and copied as predecoded
  words, which is exact.

**Correctness:** `MW_GPU_VERTEX_VALIDATE=1` also ships the CPU decode. The worker
reads back the GPU result and compares every word. Results: 0 mismatches over
8.3×10⁹ words (14,793 batches) and 6.9×10⁹ words (8,088 batches) with the first job
layout, and 0 over 5.96×10⁸ words with the compact layout. The worst relative
error is reported per batch. The XFB A/B at 1× shows the same scene content and
effects on both paths. The frames are not pixel-identical, because the guest clock
is wall-time and captures land on different moments.

**Internal resolution:** `MW_INTERNAL_RES=1..4|auto`. The default `auto` divides
window height by 480 and rounds, so a 720p window gets 2× and 1440p gets 3×. It
reuses backend-wgpu's existing EFB scale: viewport, scissor, EFB copies and
readback downsampling, the XFB composite, and 4× MSAA. The 3× capture
(`2560×1440`) has crisp line art. It is set at renderer creation; there is no live
toggle yet.

**Measurement:** `MW_PERF_LOG` now also reports guest frame-time p50/p99/max, the
core-thread GX command time, vertex-prep time, DSP time, draws/s, and the raw,
XF-snapshot and array bytes/s. The timers use `rdtsc`; the earlier
`Instant::now`/QPC per GX command cost about 15% of the core thread by itself.
Runs are 30 s, and accept the boss challenge with `MW_PC_DIAG_HOLD_ENTER`, which
now takes a list of windows (`10,0.2;11,0.2;…`).

## Numbers (release, RTX 3090 / Vulkan, i9-10900KF, 1×, 30 s runs)

Stage 201 at the boss-switch scene, core-thread ms per wall second, matched load:

| path | vertices/s | draws/s | vertex prep | all GX | DSP | guest fps |
|---|---|---|---|---|---|---|
| CPU decode | 2.8 M | 575 k | 337 | 482 | 24 | 25–27 |
| GPU decode | 2.8 M | 586 k | 286 | 438 | 27 | 25–26 |
| CPU decode | 3.7 M | 780 k | 430 | 610 | 23 | 37–39 |
| GPU decode | 3.8–4.0 M | 784–840 k | 384–396 | 574–583 | 26 | 34–39 |

Earlier interpreter-baseline runs, using QPC timers and so inflated, measured guest
fps of 18–29 with frame p50 33–67 ms and p99 50–120 ms in the same stretch. No run
showed any blocking on the GPU: `gxBlockingMsPerSec` was 0 throughout.

Leaf-RIP profile of the core thread, GPU path (1,156 samples):
- `madworld_core.dll` (the recompiled game): 42%.
- `build_raw_job`: 15% before the compact jobs.
- Depth plane: ~7%.
- `build_vertex_format`: ~4%.
- The rest is the FIFO parser and draw bookkeeping.

The CPU path's `decode_vertex` alone was 15.7%.

## Finding

MadWorld issues 550–840 thousand draw batches per second at about 5 vertices
each. The cost on the core thread is the fixed per-draw work of an emulated GX
command processor: FIFO framing, BP/CP/XF register parsing, the per-draw state
snapshot, batch bookkeeping and the depth plane. That outweighs the vertex math. So
moving vertex decode to the GPU on the FIFO path saves only about 15% of vertex
prep, or 5–8% of all GX time.

The rest of the gain needs GX work to stop happening per draw on the core thread.
`GX-HLE.md` covers that: SDK-level hooks, which are all identified in
`gx-sdk-functions.json`, and display lists parsed once and cached.

## Not done

- Display-list caching and GX HLE: planned, not started.
- Presentation pacing: Bevy presents at the display rate with AutoVsync and blits
  the latest XFB. Guest pacing follows the guest frame rate. No mailbox or uncapped
  option has been added yet.
- Vertex-JIT comparison: the measurement-only `vertex-jit` feature builds (cranelift
  0.134 `finalize` fix), but its binary was removed during a disk cleanup and has
  not been rebuilt in its own target directory.

## Display-list batching (2026-11-08, `MW_GX_DL_BATCH=1`, default off pending sign-off)

`MW_GX_DL_STATS=1` (C++ FIFO) showed:
- 99.8% of draws and vertices come from display lists. DL draws ran 300k–960k/s;
  top-level draws only 500–1,900/s.
- About 510 unique lists, with zero content rewrites.
- About 140 draws of about 5 vertices per call, and no state commands inside lists.

The FIFO forwarded each list's commands to the renderer one FFI call at a time, so gecko
never saw consecutive draws together and could not batch them. With `MW_GX_DL_BATCH=1`,
`gx/device.cpp` forwards a whole flattened list in one call.

Results, 30 s boss-switch runs, core-thread ms per second:

| mode | batches/s | all GX | vertex prep | guest fps |
|---|---|---|---|---|
| batching off, GPU decode | 615–940 k | 550–630 | 300–420 | 27–34 |
| batching on, CPU decode | 8–12 k | 420–560 | 340–460 | 25–33; 50–60 in the 17–23 s stretch |
| batching on, GPU decode | 8–20 k | 105–210 | 60–135 | 30 steady; 43–59 in the 16–21 s stretch |

Correctness and visuals:
- Vertex validation with batching on: 0 mismatches over 2.57×10⁹ words.
- XFB A/B, on vs off: both render the full scene with nothing missing.
- With batching, the guest runs at the game's own frame caps. Most gameplay logic is
  fixed at 30 (see SIXTY-FPS.md).
