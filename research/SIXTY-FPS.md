# 60 fps for MadWorld: how the game paces itself, options, recommendation

Tags: **native** = read directly from recompiled original code, DOL/RSO bytes or runtime evidence; **inferred** = strong reading of code shape or naming, not proven; **unknown** = open.
Static evidence: `evidence/sixty-fps-static.txt`. Probe run: `evidence/sixty-fps-probe.jsonl`, `evidence/sixty-fps-perf.jsonl`.

## Summary

- The game has a built-in frame-interval setting. `fn_8026F3A8(n)` stores the VI-retrace interval, `GameVSyncF = n`, `fps = 60/n` and `GameVpsInv = 1/fps`. The DOL's static data holds the **60 fps** values (interval 1). Boot calls `fn_8026F3A8(2)` from one place only, and nothing changes it at runtime. **native**
- Logic is **fixed-step per game frame**. There is no delta-time from the timebase, and a late frame is never caught up (the game slows down instead). Only about 50 instruction sites read the frame-step globals: about 45 in the player and system code in the DOL, and about 5 in the 3828-export enemy RSO. Animation advances `step68 = 1.0` per game frame, independent of `GameVSyncF`. **native**
- So switching the interval to 1 would run most of the game, animation included, at double speed. An exact 60 Hz logic mode is not achievable (option a rejected).
- Recommendation: **render-side interpolation (option b)**. The gecko renderer already gives per-draw view-space vertices and the projection separately, so frame N+1's action list can be replayed with transforms interpolated against frame N. The probe below measures how well draws pair across frames. Prerequisite: the guest must hold a steady 30 first (the intro cutscene still runs at 12–17, see `PERF-AUDIO.md`).

## 1. Frame pacing in the original code

| Item | Address | What it does | Tag |
|---|---|---|---|
| Interval setter | `fn_8026F3A8(n)` | n>0: `[805CBE20]=n`, `[805CBE24]=(float)n`, `[805CBE28]=60/n`, `[805CBE2C]=60.0/n`, `[805CBE30]=1.0/(60.0/n)`. n<=0: the interval-1 set from the constants `80568FBC=60.0`, `80568FC0=1.0`, `80568FC4=1/60`. | native |
| Exported names | `Jack.sel` | `GameVSyncF` = 805CBE24 (frame step in 60 Hz ticks), `GameVpsInv` = 805CBE30 (seconds per game frame). Only these two are exported to RSOs. | native |
| DOL defaults | main.dol .data | `805CBE20..30` = 1, 1.0f, 60, 60.0f, 1/60. The shipped image is set up for 60 until boot overwrites it. | native |
| Boot call | `fn_8026F378` (called at `8026F00C`) | Registers retrace callback `0x8026F364` through `fn_80310C10`, a VI retrace-callback setter (pre/post not determined), then calls `fn_8026F3A8(2)`. This is the only call of the setter. | native (setter role inferred from shape) |
| Retrace callback | `fn_8026F364` | `[806A4174] += 1` on every VI retrace. | native |
| Frame wait | `fn_8026F324` | Calls `fn_802F7AD4` (unknown, likely GX draw sync), spins until `[806A4174] >= [805CBE20]`, then sets the counter to 0. No catch-up: if a frame takes 3 retraces, the next frame still gets a full interval. | native |
| Swap | `fn_8026F2F8` → `fn_8025BEB4` | `fn_80373274` / `fn_80372148` (GX copy/flush, inferred), then `VISetNextFrameBuffer` (`fn_80312590`), `VIFlush` (`fn_8031247C`), and toggles the XFB index. | native order; SDK names inferred |
| Main loop | `fn_8026F10C` | `while (fn_8026F160()) { swap; wait; }`. The loading loops `fn_8021F980` and `0x80264C3C` use the same swap+wait pair. | native |
| `VIWaitForRetrace` | `fn_803114C4` | Used only at video init (`fn_8025B5C8`, two waits) and on the reset/power path (`fn_8026F460`), not per frame. | native |
| Region / 50 Hz | — | The setter hard-codes 60. PAL 50 Hz handling was not found. | unknown |
| Built-in 60 fps or frame-skip mode | — | No second caller of the setter, no flag read before it, no debug menu reference found. The 60 values exist only as the pre-boot defaults. | native (absence within the static scan) |

## 2. Per-subsystem time step

| Subsystem | Step source | Evidence | Tag |
|---|---|---|---|
| Motion / animation (`cByMotion` update `fn_80079388`) | `frame58 += step`, where step = object float `+0x64` (or the `fn_8025AA04` override), and the countdown uses `cScene::getSpeedRate` | Does not read `GameVSyncF`. Runtime evidence files record `step68 = 1.0` (1.025 in one boss state) at 30 fps. Motion is authored per game frame. | native |
| Player `cPl0000` (DOL 0x8027–0x802D) | Partly scaled | About 45 `GameVSyncF` loads in about 25 functions (movement, charge and lock-on code). Two sites branch on `interval == 2` and pick a different tuned constant (`main` at `80270D3C`, `fn_802CD2B8` at `802CD4AC`). | native |
| Havok | `GameVpsInv` | `fn_800AB7D0` (called from `802151F0`) stores `GameVpsInv * k` and its inverse, then calls `fn_8043E920`/`fn_8043E934`. Most likely the Havok world step delta. | inferred |
| Pad / repeat | Interval | `fn_8020EE00` (`cPadSystem` update area) subtracts the interval from a repeat counter. | native code, semantics inferred |
| Long timer | `fps * 600` | `fn_80181CF8` sets a float to `fps * 600` frames (10 minutes). | native |
| Enemy AI (enemyRSO) | Mostly frames | Only 4 `GameVSyncF` and 1 `GameVpsInv` hi/lo relocation pairs in 3828 exports. Timers such as `getDownTime`, `getPiyoriTime` and `getBerserkTime` are frame-count based. | native (reloc count); per-timer units inferred |
| Camera, effects/particles (`EffCore`), UI layouts, event scripts | Per frame | No references to the globals found. | inferred (the scan cannot see values copied at init, e.g. `main` stores derived floats into the game object at `+4424`/`+4520`) |
| Audio | DSP/AI clock | AX mixing runs on the 32 kHz AI/DSP clock, independent of the frame. Sound triggers come from per-frame logic and motion cues. | inferred |
| Timebase (`OSGetTime`) | Not used for the logic step | The wait loop counts retraces, not time. Absence elsewhere is not proven. | inferred |

## 3. Options

### (a) True 60 Hz logic: rejected
- Mechanism: change the `2` in `fn_8026F378` to `1`. The `GameVSyncF`/`GameVpsInv` consumers (player, Havok, pad) would adapt.
- Why it is not exact: animation (`step68 = 1.0`/frame), enemy and event timers, effect lifetimes, UI tweens and per-frame RNG draws are not scaled. They would run at 2x speed. Making them exact means halving per-frame constants across the DOL, the RSO and the motion/event data. Half-steps also land motion cues between integer frames, and RNG call order would change. That amounts to the blanket multiplier the rules forbid, and it would still not be semantics-exact.
- Risk: high. Not prototyped.

### (b) Render-only interpolation: recommended
- Logic stays at 30. For each new guest frame N+1, the renderer first replays N+1's action list with transforms interpolated halfway against frame N. That frame is presented at t+16.7 ms, then N+1 itself.
- Why it fits gecko: draws already carry XF state, and vertices are transformed to view space per draw (`DrawVertex.pos_view` on the CPU path, `vertex_decode.wgsl` on the GPU path). Projection arrives separately as `SetProjection`.
  - Rigid draws: lerp the 3x4 position and normal matrices, then re-orthonormalise.
  - Skinned draws (per-vertex `PNMTXIDX`): lerp the view-space vertex positions, which works because the vertex counts match.
  - Projection: lerp it, or skip interpolation when it jumps (camera cut).
- Replay rules: draws that do not match use N+1 data unchanged. EFB-to-texture copies (MadWorld's post effects) are re-rendered in the replay, but **never written back to guest RAM** and never reported to the emulator.
- Latency: +1 guest frame (about 33 ms) with true interpolation. Extrapolation from N-1→N avoids that but overshoots on reversals.
- Risks:
  - Pairing errors when particles spawn or sorted transparent draws reorder.
  - 2D UI pairs trivially (static matrices).
  - Doubled GPU work (the GPU is not the bottleneck today).
  - It touches `backend-wgpu/src/sink.rs`/`render.rs`, which gpu-renderer currently owns.
- Game logic, audio and timing are untouched by construction.

### (c) Motion-based frame generation in the presenter: fallback only
- Optical flow between two presented XFBs, warped to the midpoint, in the Bevy blit path (`native_main.rs::blit_native`).
- It touches the least code, but gives the lowest quality: MadWorld's high-contrast black/white/red art and thin lines are a worst case for flow-based warping (halos, smearing on splatter). It also costs the same one-frame latency.
- Plain frame blending is not 60 fps motion and was dismissed.

## 4. Recommendation and prototype

1. Get the guest to a steady 30 first (perf-audio). Interpolating a 15 fps guest gives 30, not 60.
2. Build (b) behind `MW_60FPS=1` in the backend worker, together with gpu-renderer: keep the previous frame's draw fingerprints and matrices, replay with interpolation, and present the in-between frame at half the guest frame interval.
3. Keep (c) as a fallback only if (b)'s pairing turns out poor in combat scenes.

**Prototype in the tree:** `crates/mw-game/src/native_sixty.rs`, `MW_60FPS=probe` (optional `MW_60FPS_LOG=<file>`). It is observation only and wraps the render sink. Each draw is fingerprinted by position array base, VCD, primitive and vertex count, along with its default position matrix. Every guest frame is matched against the previous one, and the probe reports:
- draws per frame
- the matched draw and vertex share, and the in-order share
- EFB copies per frame
- for rigid draws, the share that moved and the view-space translation percentiles

This is the go/no-go data for (b)'s pairing. With `MW_60FPS` unset, nothing changes.

### With the GX SDK HLE renderer (direction from the main session)

The port is moving away from FIFO emulation: gpu-renderer is HLE-ing the GX SDK, so `GXLoadPosMtxImm/Indx`, `GXLoadNrmMtx*`, `GXSetProjection` and `GXCallDisplayList` reach native wgpu code directly. That makes (b) cleaner and cheaper than on the FIFO path:
- **Draw identity is exact.** The display-list address and length, plus the vertex array bases, identify a mesh across frames. The probe's fingerprint heuristic is only needed for immediate-mode `GXBegin` draws.
- **Matrices arrive as values per draw.** No XF-memory archaeology is needed: pos/nrm matrix slots, projection, and texture matrices are snapshotted at each draw call.
- **Interpolation becomes a per-draw uniform swap.** The renderer keeps frame N's recorded command list keyed by draw identity and repeat index. On N+1 it first submits the list with `lerp(M_N, M_N+1, 0.5)` and re-orthonormalised rotations, and `t = 0.5` projection (falling back to N+1 on a cut). Then it submits N+1 itself.
- **Skinning is free on the GPU.** Bone matrices are the `GXLoadPosMtxIndx` slots, so skinned meshes interpolate without touching vertices.
- **Guest-visible readback stays single.** `GXPeekZ`, EFB-to-RAM copies and draw-done/token callbacks are produced only by the real N+1 submission, never by the in-between frame.
- **Pacing:** each guest present becomes two host presents 16.7 ms apart, with one guest frame of latency (or extrapolation behind a second flag).

Proposed interface, sent to gpu-renderer:
- `begin_frame` / `end_frame(present)` markers at the game's swap (`fn_8025BEB4`).
- A per-draw record `{key: (dl_addr, dl_len, array_bases, seq), pos_mtx[], nrm_mtx[], tex_mtx[], projection, state_id}`.
- A renderer-side `MW_60FPS=1` mode that owns the replay and the presenter pacing.

**Agreed split with gpu-renderer (2026-11-08):**
- gpu-renderer owns the mechanism: a per-guest-frame `FrameRecord`, cut at the swap marker, holding each draw's key plus pos/nrm/tex matrices and projection as values. It also provides a `submit(record, matrix_override)` replay that renders to the present target only.
- This work owns the policy: matching, lerp/slerp with re-orthonormalisation, cut detection, and the 16.7 ms present schedule. That code lives in `native_sixty.rs`.
- FIFO-path shortcut before HLE stage 3: `RawVertexStream.xf_cache` already snapshots XF matrices per draw (0x000/0x400/0x500), so a "keep previous raw stream" hook in backend-wgpu would allow a prototype on today's path.
- Known limitation: in-frame EFB-copy textures (shadows, blur) are reused from frame N in the in-between frame.

### Probe results (native, two runs of boot → title/intro, FIFO path, GPU vertex decode on)

Medians over the seconds with more than 1000 draws per frame: run 1 = 60 s (`evidence/sixty-fps-probe-run1.jsonl`), run 2 = 56 s (`-run2.jsonl`). Per-second `guestFps` is in `sixty-fps-perf-run*.jsonl`.

| Metric | Run 1 | Run 2 |
|---|---|---|
| Guest presents/s | 28.5 | 24.7 |
| Draw batches per frame | 8.9 k | 21.4 k |
| Draws matched on the full key (array base + VCD + primitive + counts) | 17% | 20% |
| Vertices matched on the full key | 14% (max 35%) | 18% (max 58%) |
| Draws matched on the shape key (array base removed) | — | **63%** |
| EFB-to-texture copies per frame | 6.5 | 6.5 |
| Matched rigid draws whose matrix changed | 88% | 80% |

Reading:
- **The position-array base moves between frames for about two-thirds of draws.** The game rewrites vertex arrays into per-frame (double-buffered or CPU-written) memory, so `GXSetArray` state is not a stable identity. **native**
- **A shape-only key is ambiguous.** Many meshes share the same VCD and counts, so 63% is an upper bound, not usable pairings.
- **FIFO-state pairing fails the go bar (≥ 90% of vertices).** The FIFO-path shortcut (a "keep previous raw stream" hook) is therefore **not worth building**.
- **Most matched rigid draws move every frame.** That is consistent with a moving camera baked into the view-space matrices. Interpolation has real work to do, but it also means camera-cut detection must use the projection or view and not per-draw deltas.
- **About 6.5 EFB copies per frame** (post effects, shadows) confirm that the in-between frame must replay them renderer-side.

Consequences for (b) on the HLE path:
- Pairing must come from **game-side object identity**, not GX state. Use the model or mesh object and draw-call site that issues `GXCallDisplayList` / `GXSetArray` (caller LR plus object pointer, plus the repeat index), because array addresses change per frame.
- For CPU-written vertex arrays (positions change in RAM, not only through matrices), interpolate the decoded positions of the paired draw. Matrix lerp alone is not enough for them.
- gpu-renderer confirmed (2026-11-08): `FrameRecord` keys each draw by caller LR (`GXCallDisplayList`/`GXBegin`) plus the model object pointer where cheap plus a repeat index. Each draw also carries a "position array rewritten this frame" flag, taken from its dirty-page generations.
- Next step: when HLE stage 3 exposes `FrameRecord`, add the caller-identity key there and re-run this probe on it, with the same ≥ 90%-of-vertices go bar.

Not measured yet, because no in-between frames exist: presents per second in a 60 fps mode, game-speed and audio-sync invariance, and an A/B capture. With option (b), game logic is untouched by construction. Its own check is that guest `guestFps` and the AID rate stay unchanged with `MW_60FPS=1` on or off.
