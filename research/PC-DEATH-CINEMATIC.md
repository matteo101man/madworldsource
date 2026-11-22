# PC death cinematic

**PC feature hook.** On the Wii, when Jack dies:
1. The death motion plays with the angle-mode camera.
2. About 1 s later, a full-screen TV-static wipe runs.
3. The continue / game-over window opens over the rest of the fall, with rolling noise bars.

The PC port can replace that with a short cinematic:
1. **Push-in.** The camera pushes in on Jack and swings to an open side near his front three-quarter.
2. **Slow motion.** The death motion plays in slow motion. Only Jack's own motion is slowed.
3. **Orbit.** The camera pulls back and slowly orbits the body.
4. **Original flow.** Then the original continue / game-over flow runs: window, confirm, DeathWatch shutter and logo, respawn. Optionally, the TV-static wipe is removed.

Both parts are **default on** (user-confirmed, 2026-11-08):
- `MW_PC_DEATH_CINEMATIC=0` restores the original death presentation.
- `MW_PC_DEATH_NO_STATIC=0` keeps the original TV static (separate toggle).

Nothing runs outside player state 95. No scene, global, VI or motion clock other than Jack's death motion step is touched.

Code: `tools/upstream-native-core/port/runtime/game/pc_death_cinematic.cpp` (+ `.h`), a three-line call-out in `pc_camera.cpp`, and the hle.txt block "PC death cinematic".

Evidence labels: **native** = read from the original code in `main.dol` (recompiled, unchanged) · **runtime** = observed in a live run · **inferred** = reasoned from the code, not observed.

## Toggles

| Env | Default | Effect |
|---|---|---|
| `MW_PC_DEATH_CINEMATIC=0` | on | Cinematic camera, slow-motion death motion, continue flow held until the shot is done |
| `MW_PC_DEATH_CINEMATIC_SLOWMO=<0.05..1>` | 0.35 | Death-motion speed |
| `MW_PC_DEATH_NO_STATIC=0` | on | `=0` keeps the static. Default: skip the continue sequence's TV-static wipe (independent of the cinematic) |
| `MW_PC_DEATH_TRACE=1` | off | `[pc-death]` lines: begin / plan / camera every 15 frames / motion end / release / static skip / end |

Both were default off during development and switched to default on after the user confirmed. They are not wired to the PC settings menu (F10) yet.

## Original death sequence (native)

Details are in `research/PLAYER-DEATH-API.md`.

**State 95 update `0x802C0F80`** (phase byte `+0x11C`):
- Phase 0 starts the death motion and `changeCameraAngle(…,2)`.
- Phase 1 sends `onDead`/`onPlayerDead`, then calls `cByMotion::move` (`0x80079210`) on **player+0x1AC** (`addi r3,r30,428`, `bl` at `802C11E0`). A nonzero return means phase 2.
- Phases 1 and 2 share the countdown tail at `802C120C`: `+0x1230` decrements, and at 0 `+0x1110 = 1` is set.

**Continue gate:**
- "Player dead" predicate `0x801248A4`: true when HP `+0xEC <= 0`, when `+0x1110 != 0`, or when game flag `0x80697E8C` bit 9 is set.
- HP is already 0 on the lethal hit, so the room scenario update `0x801286FC` (its call at `80128710`, LR **`0x80128714`**) creates the dead-event task `0x80127BC4` on the next scene update.
- That task runs the continue sequence `0x80127DFC`. **runtime:** the window opened about 1 s after the hit while the fall was still playing (`pc-death-off`, `pc-death-run1`). The countdown does not gate it.

**Continue sequence `0x80127DFC`** (native):
- `BeginDemoMode(0)` at `80127E50`.
- **`0x801241A0` at LR `0x80127E54`**.
- Then the continue or game-over window (`0x80244DBC`), the confirm, and restart `0x802C1264`.
- `onContinueRestart`.
- `0x8012365C`, `0x8024482C`, `0x80086E60(8)`.

**`0x801241A0` is the TV-static helper** (native): `0x802452DC(ui)` sets byte 0 of HUD-manager (`*0x8069E2C0`) widget `+0x48`, which is layout widget id **19** resolved in `0x80244A80`. It then waits, one frame at a time, until `0x802452F4` reports byte `+0x10`.
- The same helper (and its duplicate `0x802D3F70`) is called from about 12 scene scripts (`0x8005AEE8`, `0x8006C950`, `0x8016F3F8`, …) as the game's generic static transition.
- **runtime:** with the helper running, frames show a full-screen static wipe about 1.1–1.7 s after the hit, then horizontal noise bars and grain over the continue window (`pc-death-run1-first12.png`, `pc-death-off-sheet.png`).
- With only that call skipped, neither appears (`pc-death-run3/4`).

**Not the static** (runtime + native):
- `core2.dat/jkNoise.tpl` (texobj `0x806A1B18`, loaded by `0x8025BCE8`) is bound every frame by render pass 4 of `0x8025BFCC` (jump table `0x805CB254`, call `8025D58C`) at a constant rate before, during and after death. That is the base look, not the death effect.
- jkNoise2 (`0x806A1B38`) was never bound through the cached loader `0x80256900` during the runs.
- The continue→respawn "TV switch-off" shutter and DW logo come from widget 61 (`0x801241F4` → `0x802447D0`/`0x80244A64`). They are the original DeathWatch transition and are kept.

**Motion step** (native):
- `cByMotion` frame advance `0x80079388` loads `f1 = motion+0x64` and calls `0x8025AA04` (LR `800793F4`, its only caller). That returns `motion+0x34` when `motion+0x38 == 1`, otherwise the incoming f1.
- The result is stored at `+0x68` and added to frame `+0x58`. The end test uses the advanced frame.

## Hooks (port code)

All are hle.txt entries. Each one runs the original body, except StaticWipe for its one caller under its toggle.

| Address | Name | Change while active |
|---|---|---|
| `0x802C0F80` | PlayerDeadUpdate | None. Follows the phase (motion end = phase ≥ 2) and decides when the shot is done |
| `0x8025AA04` | MotionStep | Only LR `800793F4` with r3 == player+0x1AC, until the motion ends: step × slow-mo |
| `0x801248A4` | PlayerDeadQuery | Only LR `80128714`: answers 0 until the shot is done (motion ended and 105 orbit frames, or at most 900 state-95 frames) |
| `0x801241A0` | StaticWipe | `MW_PC_DEATH_NO_STATIC` (default on) and LR `80127E54` only: returns at once (widget 19 never started, nothing to wait for) |
| `0x802C9F3C` / `0x800BA4B0` | free-camera hook | `pc_death_camera_begin/end/rot` (below) |

**Camera.** There is one camera hook, shared with the free camera (pc-free-camera session informed).
- While the cinematic is active, `pc_camera.cpp` calls `pc_death_camera_begin` first. It clears the free-camera orbit (**free camera disabled for the whole cinematic**) and runs the original follow camera.
- **Centre.** The follow camera receives a guest-stack copy of its centre vector, lowered from the original look height (+1.7–1.95, runtime) to 1.0 during the push-in and to 0.45 above the feet as the fall lands. The caller's vector is untouched.
- **Rotation.** At the follow camera's own `MtxInitRotVec` (LR `802CA61C`), the cinematic writes the follow camera's state: yaw `+0x1AD4`/`+0x1AC8`, pitch `+0x1AC4` and distance `+0x1AE4` (read for the eye at `802CA648`).
- **Original after that.** Eye = dest + R·z·dist, the Havok wall check, the camera manager and g_Camera are all original. The respawn camera is reset by the original after the DW blackout.
- **Shot planning.** It happens once, at the first cinematic frame. 24 yaws around the lowered look point are tested at the far distance with the follow camera's own read-only line query `fn_800ABAA8(0x80609964, &hit, 0, &from, &to, 27, 0, 0)`.
  - The push-in takes the clear yaw (≥ 0.85) nearest facing ± 0.95 rad.
  - The orbit stays inside the clear arc (≥ 0.7) around it, easing and reversing at its ends. If every probe is clear, it is a full orbit.
  - Distances shrink if even the best direction is partly blocked.
- **Shot timing** (camera frames, about 30/s):
  - Push-in: 40 frames, to distance 1.45 and pitch −0.22.
  - Hold: drifts with the slowed fall.
  - After the motion ends: pull back over 75 frames to 3.2 and pitch −0.62 (above, looking down), orbiting at up to 0.0105 rad/frame.
  - The orbit continues behind the continue window until the confirm.
- **For sixty-fps.** The cinematic adds no camera cut: it starts from the current yaw/pitch/distance and moves smoothly. The only cut is the original respawn after the DW blackout.

## Runtime verification

**Setup:**
- Core rebuilt 4× under `.core-build.lock` + `.bevy-instance.lock`; logs in `evidence/pc-death-core-build{,2,3,4}.log`.
- Test exe: a private copy, `D:/GDecomp/build-cache/bevy-0.20/debug/deps/madworld-native-deathcine.exe`.
- Runner command: `tools/run_owned_gpu_test.py --pc-steering --exe …`, with:
  - the explicit test fixture `MW_TEST_PLAYER_HP1=1` (HP → 1; the original hit kills);
  - `MW_GPU_TEST_PC_CONTINUE_CONFIRM=1` (Enter pulses while HP 0);
  - `MW_TRACE_RPN=1`.
- XFB captures come from the new harness probe `MW_GPU_TEST_DEATH_CAPTURE=<dir>` (`native_gpu.rs` `DeathCapture`). It saves the XFB every `MW_GPU_TEST_DEATH_EVERY` s for `MW_GPU_TEST_DEATH_SPAN` s from the first `[pc-death] begin` / `onDead` log line.
- Each run's exit 1 is the pre-existing harness assertion "PC action sequence did not complete" (left `dead-confirm`). The harness is still in the dead-confirm phase at its deadline; this is not a game fault.

| Run | Env | Result |
|---|---|---|
| `pc-death-off` | none (defaults) | Original: static wipe at about 1 s, window at once, 2× continue → `onContinueRestart` → respawn. The hooks are inert when off |
| `pc-death-run1` | cinematic, trace | Slow-mo works (death motion 124 frames vs about 43). The window still opened early (countdown pacing alone does not gate it), so the `0x801248A4` gate was added. Static visible: `pc-death-run1-first12.png` |
| `pc-death-run2` | + gate, no-static | 3 deaths: shot → release at frame 228/259 → static skipped → window → confirm → restart. The camera looked over the body (look height) and was pulled in by walls, so a lowered centre and wall probes were added |
| `pc-death-run3` | + framing | Body framed; wall probe limits the arc in the walled spot (`clear` 0.2–0.4 on 14 of 24 yaws). Continue frame clean: `pc-death-run3-xfb/death-033-t010.6.png` |
| `pc-death-run4` | final | 2 deaths, both: `plan` → motion end frame 154 (slow-mo) → release frame 259 (105 orbit frames) → `static wipe skipped` → `[continue-ui] window mode=0` → `onContinueRestart` → state left 95 → fight resumes. Sheet `pc-death-run4-sheet.png`, shutter/logo `pc-death-run4-transition.png` |
| `pc-death-default` | no `MW_PC_DEATH_*` vars (default on), core build 5 | Death 1: plan, motion end at frame 123, release at 228, static skipped, continue window, `onContinueRestart`, respawn. Death 2 reached motion end; the test deadline ended the run. Sheet `pc-death-default-sheet.png` |

The first frames of some captures show the Rin VS card. That is the capture probe's stale XFB at start, not game state.

## Inferred / open

- **Game-over branch (inferred).** Not run with the cinematic. The `getContinue()==0` branch goes through the same `0x80127DFC` entry, so the gate and the static skip apply to it too (fixture `MW_TEST_PLAYER_CONTINUES=0`).
- **Rin during the shot (runtime, not judged).** The shot holds the original continue flow for about 8.6 s after the hit, versus about 1 s. During that time the world keeps its normal speed (no world slow-down, by design: no global speed hack). Rin is free to act; the runs showed no extra hits in state 95, which has no damage handling.
- **Root position (inferred).** The look point uses Jack's root position `+0x64`. The fallen body appears centred in the runs, but a death motion that moves the body far from the root would need a joint position.
- **HUD.** The original HUD stays visible during the shot.
- **Not tested.** Free camera enabled together with the cinematic (code path: `pc_death_camera_begin` runs before the free camera's `enabled()` check), other stages, and a death inside a camera area or event camera (`fn_802C9A98` returns 0; the trace would log `ran=0` and the shot keeps the original camera).
