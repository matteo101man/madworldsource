# PC free camera

The Wii game has no manual camera. Tapping C recenters behind Jack, and holding C locks on. The PC port adds an optional orbit camera inside the game's own follow camera. It is **default off**; turn it on with `MW_PC_FREE_CAMERA=1`.

Evidence labels: **native** = read from the original code in `main.dol` (recompiled, unchanged) · **runtime** = observed in a live run · **inferred** = reasoned from the code, not observed.

## How to use it

| | |
|---|---|
| Turn on | `MW_PC_FREE_CAMERA=1` |
| Mouse | `MW_PC_FREE_CAMERA_MOUSE=capture` (default: the cursor is locked and hidden while the window is focused and the settings menu is closed), `hold` (look only while Left Alt is held), `off` (right stick only) |
| Gamepad | Right stick (from the settings session's `pc_config::CameraLook`; zero while the RB/LB gesture modifiers are held) |
| Sensitivity / invert | The saved `[camera]` settings in `pc-controls.toml` (settings menu). The env vars `MW_PC_FREE_CAMERA_SENS=<multiplier>` and `MW_PC_FREE_CAMERA_INVERT_Y=1|0` override them and are never saved |
| Auto-follow | Settings `camera.auto_follow` (default on), overridden by `MW_PC_CAMERA_AUTOFOLLOW=0|1`. After 1.5 s (45 follow-camera frames) without look input, *while Jack moves*, the yaw eases back behind him: step = 6 % of the error, at most 2°/frame, pitch offset decays. Within 5° the original chase takes over again. Off: the camera stays where you put it until Q |
| Lock-on target cycle | While the game is locked on: mouse wheel up/down, or a right-stick flick right/left (past 0.85 from rest, mostly horizontal), picks the next on-screen enemy to the right/left. Tab is not used (it is Wii Plus) |
| Q / middle mouse / pad LT | Unchanged: tap recenters, hold locks on. Both go back to the original camera and clear the orbit |
| Orbit while locked on | Settings `camera.lock_orbit` (default on, "Orbit while locked on" row on the Camera & gamepad page, saved in pc-controls.toml). `MW_PC_CAMERA_LOCK_ORBIT=0|1` overrides it for the session. F5 flips the env override if one is set, otherwise the setting. Sent to the core as flags bit1 |
| Trace | `MW_PC_FREE_CAMERA_TRACE=1` prints a `[pc-camera]` line every 30 follow-camera frames (and every 5 while input arrives) |

## Original camera (native)

The camera data lives in the player object `cPl0000`. Offsets are from `this`.

- **Camera array.** `g_Camera` is at 8069BD80, stride 0x324, index player+0x10F0. `cCamera` pos is at +0x1B0 (`getPos` 8007FCE4) and dest at +0x1BC (`getDest` 8007FD28). This is the camera that is shown.
- **Per-frame order.** In fn_8027621C (the player per-frame update, called from cPl0000_frameUpdate):
  - 802770E0 calls **fn_802C6EEC**, the movement basis.
  - Later, 8027785C / 80277890 (and 8027A1AC in fn_80279D10) call **fn_802C9F3C**, the follow camera. The arguments are r4 = centre vector, f1 = Jack facing (+0x74) and f2 = an extra distance.
- **Movement basis, fn_802C6EEC.** It sets `+0x1134 = atan2(dest.x-pos.x, dest.z-pos.z)` from g_Camera[+0x10F0], which is the shown camera, not the follow camera's internal values. For up to 90 frames after a camera cut (flag +0x27C4, set when fn_80080448 reports an area change), it RadLerps toward that value instead.
  - 20+ state handlers read +0x1134 as the stick reference, for example fn_8028DB68, fn_802910A4 and fn_80297C44.
  - So a camera change made *inside* the camera calc reaches movement on the next frame. A render-only view change would not.
- **Angle parameters, fn_802C9A98(this, mode=+0x2758).** It runs at the top of fn_802C9F3C. Every frame it reloads from the table at 805692A4 (+0x275C*0x40C + mode*0x1C), or from an active camera area (g_CameraArea 80602C5C):
  - dist → +0x1AE8
  - side → +0x1AF4
  - height → +0x1AF8
  - fwd → +0x1AFC
  - pitch → **+0x1ADC**
  - yaw offset → **+0x1AE0**
  - fov → +0x1AEC
  - area yaw limits: centre +0x1B24, half-range +0x1B28 (π = no limit)

  Default mode 0 is 2.25 / −0.2 / 1.5 / 0 / −0.026 rad / π / 50°. It **returns 0 when an area or event camera owns the view**, and fn_802C9F3C then does nothing.
- **Follow calc, fn_802C9F3C** (0xB74 bytes):
  - The pitch state +0x1AC4 RadLerps toward +0x1AD0 = RadAdjust(+0x1ADC).
  - The yaw state **+0x1AD4** chases `RadAdjust(facing + +0x1AE0)`, clamped to the area limits. In the unlocked main path it only chases when the error is between 25° and 135° (0x1488 / 0x148C), or when +0x276C is forced. The step is limited to 5°·+0x2764 per frame, times the time scale +0x1150.
  - Lock-on (+0x1254 target, +0x1251) chases the direction to the target. Non-default angle modes (+0x2758 ≠ 0) use a second chase branch (802CA3AC).
  - +0x1AC8 = +0x1AD4. Then `MtxInitRotVec(&R, this+0x1AC4, 5)` at **802CA618** (return address 802CA61C) builds the rotation.
  - The eye is dest + R·z·dist (+0x1AE4). Havok `checkLine` pulls the eye in front of walls. The result is stored in +0x2798 (eye) / +0x27A4 (dest), and fn_80082694 (80602CE4) submits a request to the camera manager.
- **Instant-cut path.** When +0x2764 == 1.0, fn_802CAB1C runs instead of the chase and snaps the yaw to facing + offset.
- **Recenter (C tap).** 80276C84 sets +0x27C2 = 8 while not locked. While it counts down, changeCameraAngle(this, 1) runs, +0x2764 = 0.32, and the main path sets +0x276C (forced chase). changeCameraAngle 802C9928 / LockCameraAngle 802C9A28 (+0x1128 bit 0x80) are the scripted angle API. For example, death uses mode 2.

## Hook (port code)

The hook lives in `runtime/game/pc_camera.cpp` and is listed in hle.txt. Both entries call the original body.

- **`0x802C9F3C PlayerFollowCamera`.**
  - It takes the host orbit since the last frame from `mw_pad_camera_take`, in native-platform `pad.rs` / `host.rs`. Input older than 250 ms is dropped, so mouse movement during events or menus does not land later as a jump.
  - It decides which camera is in charge:
    - **The original owns the camera when** +0x1254 ≠ 0, +0x1251 ≠ 0, +0x1128 & 0x80 is set, or +0x27C2 ≠ 0 (lock-on, LockCameraAngle, recenter). This clears the orbit.
    - **The orbit is applied only when** the user has looked, +0x11A ≠ 2, and this frame's yaw offset +0x1AE0 is the behind-Jack π. This covers angle modes 0, 2, 9–13, 16–24, 27, 28 and 33–36, plus behind-Jack camera areas.
    - **Designed shots keep the game's camera:** modes 3–8, 14, 15, 25, 26 and 29–32 (yaw offsets 0.87 / 3.42 / 3.55 / 3.97 / 0 rad, from the 805692A4 table), and any camera area with its own angle.
  - It saves the pre-chase yaw +0x1AD4, then runs the original.
- **`0x800BA4B0 MtxInitRotVec`.** It acts only while armed, with LR == 802CA61C and rot == player+0x1AC4:
  - It writes the yaw `+0x1AD4 = +0x1AC8 = previous yaw + user delta`, clamped to the area limits. This replaces only that frame's chase result, so the game no longer pulls the camera behind Jack.
  - It adds the pitch offset for the matrix only, with total pitch clamped to [−0.9, 0.45] rad, and restores +0x1AC4 afterwards.
  - Everything after this is original: distance, wall collision, camera manager, g_Camera, and then movement through fn_802C6EEC.
  - Area/event cameras (fn_802C9A98 returns 0) and the instant-cut path never reach that call site. A designed shot's yaw offset makes the hook call the original unchanged.
- **What it writes.** It writes only the follow camera's own yaw state (+0x1AD4/+0x1AC8), and only while the orbit is active. It writes no actor, motion, combat or event state. Deliberate PC feature hook.
- **Not saved.** Orbit state is not saved in snapshots (PC view state only). After a restore the camera starts in the original follow mode.

## Lock-on target cycle (native + port)

**Original lock (native, fn_8027621C 802765AC–80276A34).** The C/Q press toggles +0x1250:
- **While locked:** the press clears +0x1250 (unlock).
- **Otherwise:** the old +0x1254 is kept in r30, +0x1254 is cleared and +0x1250 is set to 1. Then fn_802C2204(this, cone, dist, exclude = r30) runs as a cascade: (π, 2), (π/2, 4), (π, 3), (π/2, 8), (π, 6). If nothing is found, it falls back to r30.

fn_802C2204 calls **fn_801DF43C(g_EmObjList 80694A84, facing, cone, player, dist, exclude)**:
- It walks the list (node *(list+0x14), object *(node+0), next *(node+8), end *(list+0x18)).
- It skips the player and the excluded object.
- It requires lockable (vfunc +0x198), bit 6 of obj+0x38C clear (fn_801DF2C0), distance² ≤ (dist + radius, vfunc +0x31C)², and the angle from Jack's facing within the cone.
- It picks the **highest priority (vfunc +0x2B4), then the smallest angle**.

The game also checks every frame:
- the target still exists (cScene::isExist);
- its vfunc +0x23C (if non-zero, the lock is dropped);
- the RC 0x220 class vfunc +0x194.

Because of the exclude argument, pressing C twice (unlock, relock) already switches to another target in the original.

**PC cycle (port).** It runs only while +0x1250 and +0x1254 are set:
- **Candidates:** the same list walk and the same filters (player, lockable, bit 6, reach 8 + radius = the widest original cascade). In addition, a candidate must be on screen: within 0.9 rad of the shown view yaw +0x1134.
- **Order:** by screen x = −wrap(atan2(dx, dz) − view), since game yaw grows to the left.
- **Choice:** the next one in the requested direction, wrapping around.
- **Switch:** it calls the original **SetLockOn_Trg 802CBDC8** (+0x1254 = target, +0x1250 = 1) at the start of the follow camera, so the original lock camera frames the new target in the same frame. The game's own per-frame lock checks still apply.
- **When it acts:** with no other candidate it does nothing. While unlocked, cycle events are ignored and a stick flick is just a look.

## Frontend (port code)

- `crates/mw-game/src/native_camera.rs` turns `CameraLook` into radians.
  - Mouse: 0.0025 rad/pixel × sensitivity.
  - Stick: 2.6 rad/s × sensitivity, with dt clamped to 0.1 s.
  - Sign conventions: mouse right / stick right → yaw −; mouse up / stick up → look up; invert flips the vertical.
  - It calls `host::add_camera_orbit` and manages the cursor grab.
- Unit tests:
  - `native_camera::tests` (4): default-off, env parsing (incl. `MW_PC_CAMERA_AUTOFOLLOW`), signs, sensitivity, invert, dt clamp, hold/off gating, wheel/flick cycle edges.
- `TargetCycle` (native_camera.rs) turns wheel / stick flicks into ±1 events → `host::add_camera_cycle`, carried in `mw_pad_camera_take` flags bits 8–15. If the settings session adds a bound cycle action, it can call `host::add_camera_cycle` directly.
  - `host::tests::camera_orbit_sums_until_taken_and_drops_stale_input`.

## Inferred / open
- **Death cinematic.** The death cinematic shares this hook and is on by default: player state 95, research/PC-DEATH-CINEMATIC.md, `MW_PC_DEATH_CINEMATIC=0` restores the original death camera. While it runs, the free camera is off and its input is drained. The death camera then plays for about 8.6 s before the continue window.

- **Yaw sign (inferred).** Mouse right = game yaw decreasing. GX is right-handed with Y up, so for a view along +Z the screen-right direction is −X, and `atan2(x,z)` decreases. Pitch sign `PITCH_UP` is checked against eye height in the runtime section below.
- **Action modes.** Attacks switch angle modes often (2 and 18 for seconds at a time in run 4). The first build allowed mode 0 only, so the orbit stopped during combat. The current rule lets every behind-Jack mode orbit. Designed-shot modes (3–8, 14, 15, 25, 26, 29–32) were seen only while lock-on owned the camera, so their exclusion is not separately observed.
- **Game-set lock in the Rin fight.** In run 7, +0x1254 = Rin (815ba840) was set by the game with no Q/LT press and stayed set for the rest of the run. The original camera runs its lock-on chase then, so the orbit is unavailable for long stretches of this fight. That follows the lock-on rule as specified. **Decided 2026-11-08 by the user: make it a toggle.** "Orbit while locked on" is on by default. Press F5 to switch it at runtime, or start it off with `MW_PC_CAMERA_LOCK_ORBIT=0`; off means the original lock camera.
  - **When it's on**, a lock (player or game set) no longer owns the camera:
    - Locking on clears the orbit, so the original lock framing comes first.
    - Look input then orbits from there.
    - With auto-follow on, 1.5 s after the last input the original lock chase reframes the target.
  - **Still original:** LockCameraAngle, the C recenter countdown and designed shots (for example lock mode 31) keep the game's camera.
  - **Evidence:** in run 11 the lock was on Rin and the stick turned yaw −1.77 → −2.94 with `applied=1`.
- **Pre-event prompt.** Before the first A press, the follow camera is the shown camera while Jack can't move, so the orbit turns the view behind that prompt (run 6: applied for 160 s).
- **Look-at point.** The look-at side offset (+0x1AF4, −0.2) is still relative to Jack's facing (original), not to the camera.
- **Untested.** +0x11A == 2 is treated as a separate vehicle-like mode and left original; its meaning is not identified.

## Runtime verification

**Setup.** Release `madworld-native` on core builds `evidence/pc-camera-core-build{,2}.log`. Env: `MW_PC_FREE_CAMERA=1 MW_PC_FREE_CAMERA_TRACE=1 MW_PC_FREE_CAMERA_MOUSE=off`. Synthetic input came only through the settings session's virtual pad `MW_PC_DIAG_PAD` (right stick → `CameraLook` → `native_camera` → core) and `MW_PC_DIAG_HOLD_ENTER`. No OS input, no Computer Use, no game-state fixtures. Logs: `evidence/pc-camera-run{1..7}.log`.

| Check | Run | Observed |
|---|---|---|
| Orbit from synthetic input | 3 | Right stick 0.6 s: yaw +0x1AC8 −0.119 → −1.681 with `applied=1`. The shown-camera yaw +0x1134 goes 3.022 → 1.461, the same −1.56 rad |
| Movement follows the shown camera | 3 | Before the orbit, stick up gives facing 3.02, and Jack walks −Z (z −60.1 → −78.8). After the orbit, stick up gives facing 1.45 = +0x1134, and Jack walks +X (x 2.70 → 11.34): heading atan2(8.64, 1.37) = 1.41 |
| Free camera holds still | 2, 3 | No input: yaw is constant (no pull behind Jack). With auto-follow, about 90 frames later `manual=0` and the original chase resumes from the user's yaw |
| Pitch sign | 2 | Stick up for +0.254: eye Y 19.15 → 17.90 under dest 19.28 (looks up). Clamped at total 0.45 |
| Lock-on (hold LT = Q) | 3 | +0x1254 = Rin and `owned=1`. Right-stick input during the lock (t=150, t=170) is ignored. The yaw follows the original target chase |
| Cutscene (event 21) | 7 | Stick held through the whole 36 s event (`[pc-skip]` true 20:37:59 → false 20:38:35). Zero `[pc-camera]` lines in that window: the player follow camera does not run during the event. The first frame afterwards gets a normal one-frame delta (−0.07 rad), not an accumulated jump |
| Behind-Jack action mode orbits | 9 | `applied=1 mode=18`: yaw −1.70 → −0.14 from the stick (the first build ignored mode 18) |
| Auto-follow eases back | 10 | Orbit to yaw −1.78 (behind = −0.22). Jack steps; 1.5 s after the last input the yaw goes −1.78 → −1.18 → −0.21, then `manual=0` (original chase) |
| Target cycle via SetLockOn_Trg | 8 | Locked on Rin, flick right/left: `cycle dir=±1 candidates=0` (Rin is the only lockable enemy), lock unchanged, no fault. Switching between two or more enemies is **not observed** (needs a stage with a group) |
| Game recenter / angle modes | 7 | Mode 1 + `owned=1` while +0x27C2 counts (f=3705–3770), then the orbit resumes. With the new rule, mode 2 orbits (f=3645 `applied=1 mode=2`) |

**Not verified:**
- A Q tap while unlocked mid-orbit. In run 4, Jack sat in mode 18 after attack taps and did not move.
- Mouse capture in a focused window by a person (only the stick path was driven).
- Screen-direction feel of the yaw sign (inferred, above).
- Snapshot interaction, other stages, finisher/grab cameras.
