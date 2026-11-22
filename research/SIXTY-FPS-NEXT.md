# 60 fps presentation: state and remaining plan (2026-11-08)

The design and probe results are in [SIXTY-FPS.md](SIXTY-FPS.md). This file covers what
was built after that and what is still open. Work stopped at the user's request.

## What is in the tree

Everything is opt-in. When `MW_60FPS` is unset, nothing changes except one null check
per present and one per blit.

- **`crates/mw-game/src/native_sixty.rs`**
  - Fingerprints every guest draw and matches it against the previous guest frame.
  - Estimates the camera delta from the consensus of
    `MV_cur · MV_prev⁻¹` over rigid draws whose position array is unchanged (static
    world geometry). It uses the 32 largest pairs as candidates and keeps the one that
    agrees with the most vertices.
  - A cut is declared when the inlier share is below 0.3, the turn exceeds 20°, or the
    projection changed for that pair.
  - `MW_60FPS=probe` only logs `MW_60FPS_PROBE` lines, which now include the camera
    fields `camInlierP10/P50`, `camRotDeg*`, `camTrans*` and `camCuts`.
  - `MW_60FPS=1` also sends `GxAction::SetInterpolation { delta, projection }` just
    before each present.
- **gecko `host.rs`**: one new variant, `GxAction::SetInterpolation`.
- **backend-wgpu `interp.rs`** (new):
  - The render worker records each guest frame's actions plus a copy of every batch's
    vertex and raw streams. It also snapshots the renderer state the frame starts from.
  - After the real present it replays the frame once into a second XFB texture, with
    `mvp = P · freelook · E` on perspective draws that use the measured projection.
    `E = (delta^½)⁻¹`, the camera midpoint.
  - Texture uploads and cache drops are not replayed. EFB RAM writebacks from the replay
    are discarded.
  - `Presenter` shows the midpoint image and swaps in the real frame half a guest
    interval later.
  - No midpoint is rendered when the guest already presents faster than 40 fps, or at a
    cut.
- **Hooks**, written with gpu-renderer's OK:
  - `sink.rs`: `Shared.presenter`, a `RenderWorker.interp` field, `begin_batch` in the
    three batch paths, and recording in `exec_action`.
  - `blit_into_encoder`: calls `presenter.before_blit`.
  - `lib.rs`: the `interp_view` field.
  - `action.rs`: one multiply on the draw MVP, and an ignore arm for the new action.
- **Measurement**:
  - `MW_60FPS_PACE_LOG=<file>` (or any `MW_60FPS`) writes per-second
    `MW_60FPS_PACE` lines: blits/s, distinct images/s, midpoints/s, and image interval
    p50/p90/p99/max.
  - `MW_60FPS_CAPTURE=<dir>` with `MW_60FPS_CAPTURE_AT=N,...` saves the real frame
    N−1, the midpoint of N and the real frame N as PPM.

## Status of the last run

- The first `MW_60FPS=1` run (`evidence/sixty-fps-interp-*`) crashed about 20 s in, on
  the renderer assert "pending EFB texture missing from renderer cache"
  (`action.rs:296`). The cause: the replay re-referenced an EFB copy made in an earlier
  frame and evicted later in the same frame.
- **Fixed, not yet run:** the replay now skips reference loads whose EFB copy is no
  longer cached. A rebuild with this fix was the last step started.
- Before the crash, the pace log showed the display blitting at **75 Hz** and the guest
  at 30 images/s. Midpoints were 0/s in those first seconds, which was menu or loading
  with no camera consensus.
- The baseline run (`MW_60FPS=probe`) never ran: it timed out after 30 min waiting for
  the instance lock.
- **No frame-pacing numbers or XFB A/B exist yet for the interpolated path.**

## Remaining plan

1. **Re-run the A/B** with `scratchpad/sixty-run.sh`. Run `probe base 70`, then
   `1 interp 70`, each under the lock. Do the runs at a quiet time; the lock was busy
   for an hour today.
   - Check that guestFps and the AID/audio rate in the perf log match between the two
     runs.
   - Report image interval p50/p99 and midpoints per second.
   - Look at the f900/f1500 prev/mid/real captures. The midpoint should sit visibly
     between the other two and show no corruption.
2. **75 Hz displays.** Thirty real frames plus thirty midpoints quantised to 75 Hz vsync
   are uneven: intervals of 13.3 and 26.7 ms. Options:
   - Ask for a 60 Hz present mode.
   - Pick t per vsync (t = phase within the guest interval) instead of a fixed 0.5. This
     costs one replay per display refresh, so it needs a GPU-time check.
3. **Camera consensus quality.** Read `camInlierP10/P50` and `camCuts` from the probe
   runs.
   - If the share of static draws is low in arenas with lots of moving geometry, add
     the projection-matched skybox and terrain draws as a prior.
   - Alternatively, read the camera matrix from the game's camera object (the HLE route).
4. **Shadow and light passes.** E applies only to draws whose projection equals the
   consensus projection. Check the captures for shadow or blur offsets. If they show,
   limit E to draws in the batch that ends in `CopyXfb`.
5. **Moving objects.** Characters still step at 30 Hz under a smooth camera.
   - Per-draw matrix lerp needs stable draw identity. gpu-renderer will expose
     `GXCallDisplayList` (addr, size) per draw when its DL cache lands. Key by DL address
     + caller + repeat index, and lerp or slerp rigid matrices. Skinned draws need
     per-matrix-slot lerp from the XF snapshot. Arrays rewritten by the CPU need a
     per-vertex position lerp.
   - Go bar: at least 90% of vertices matched (SIXTY-FPS.md).
6. **Cost.** Measure the worker's GPU and CPU time per replay (`gx-stats`, or timestamps
   around `Interp::replay`). The replay copies every batch's raw stream once per frame;
   if that shows up, move the streams instead of copying them. Also confirm that CPU
   decode (`MW_GPU_VERTEX=0`) works or gate it off.
7. **Default.** Propose making it the default only after 1–6, with p50/p99 pacing and an
   XFB A/B in hand. Until then it stays behind `MW_60FPS=1`.

## Rules kept

- Guest logic, clocks and the frame divider are untouched. Interpolation is
  presentation only and adds half a guest frame of latency.
- One game instance at a time via `.bevy-instance.lock`. The probe exe is a private
  copy, `release/madworld-native-sixty.exe`.
