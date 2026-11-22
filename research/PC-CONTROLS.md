# PC input boundary and validation

## Current PC controls (2026-11-08): gamepad, rebinding, settings menu

Code: `crates/mw-game/src/pc_config.rs` (actions, bindings, config file, stick gestures), `native_controls.rs` (actions → the unchanged 8-byte remote/Nunchuk packet), `pc_menu.rs` (settings overlay, live window/color/volume, rumble, F1 help), `native_main.rs::input`. The game still sees one Wii remote + Nunchuk through the input HLE; nothing writes game state.

**Config file.** `pc-controls.toml` next to `madworld-native.exe` (override the path with `MW_PC_CONTROLS=<file>`). It is only written when you change something (settings menu, or the C/F3 color hotkeys); with no file every default below applies, so behavior is unchanged from the fixed bindings. Missing entries keep their defaults, unknown actions/inputs are reported on stderr as `[pc-config] …` and skipped, an unreadable file falls back to defaults. Writes are atomic (temp file + rename). Env vars win over the file and are never saved: `MW_INTERNAL_RES`, `MW_VSYNC`, `MW_FULLSCREEN`, `MW_COLOR` (original|color|legacy), `MW_MASTER_VOLUME` (0..1). The menu marks overridden rows `[env override]`.

Input names: keys are Bevy `KeyCode` names (`KeyE`, `Space`, `ShiftLeft`, `F10`, `ArrowUp`…); mouse `MouseLeft/Right/Middle/Back/Forward`; pad (Xbox layout) `PadA/B/X/Y/LB/RB/LT/RT/L3/R3/Select/Start/Up/Down/Left/Right`; right-stick gestures `PadRStickUp/Down/Left/Right/CW/CCW`. `+` makes a chord. While a chord is fully held, shorter bindings it contains are ignored (RB+Up pushes instead of also picking up; Select+Start opens settings instead of also pausing).

**Actions** (config key → Wii input → default keyboard/mouse | gamepad). These names are stable; PC prompt glyphs should use them.

| key | Wii input | keyboard / mouse | gamepad |
|---|---|---|---|
| move_up/down/left/right | Nunchuk stick | W S A D | left stick (analog) |
| confirm | A (hold in a skippable event: skip) | Enter | A |
| attack | A (hold: grab) | MouseLeft, E | – |
| chainsaw | B + default vertical motion, cancelled on release | MouseRight | RT |
| back | B (menus) | Backspace | B |
| jump | Z | Space | X |
| dodge | Nunchuk back motion | ShiftLeft, ShiftRight | Y |
| lock_on | C (tap camera, hold lock-on) | Q, MouseMiddle | LT, R3 |
| shake | both arms shake while held | X | L3, LB+RB |
| pickup / taunt | D-pad up / down | G / T | D-pad up / down |
| weapon_1 / weapon_2 | D-pad left / right | 1 / 2 | D-pad left / right |
| pause | Minus | Escape | Start |
| plus | Plus | Tab | Select |
| right_arm_left/right/up/down | remote motion | ←, → or F, ↑ or R, ↓ | RB + right-stick flick |
| right_arm_push / pull | remote forward / back | N / M | RB + D-pad up / down |
| right_arm_circle / reverse_circle | remote circle | V / B | RB + right-stick rotation CW / CCW |
| left_arm_* (same eight) | Nunchuk motion | J L I K, U / O, H / Y | LB + same as right arm |
| settings | – (PC menu) | F10 | Select+Start |

The right stick is the camera (`CameraLook`, consumed by the free-camera code in `native_camera.rs`, `MW_PC_FREE_CAMERA=1`). While RB or LB is held it stops moving the camera and becomes the gesture stick: a **flick** (out and back to centre, under 90° of sweep) is reported on return to centre as the direction of its peak; a **rotation** is reported as soon as it has swept 270°, once, and the stick must recentre before the next gesture. Flicks and rotations feed the same gesture profiles (motion.rs tokens) as the keys, so the game's own recognizers decide. The left stick also drives the existing shake QTE: fast left/right reversals under the game's shake prompt (pad.rs `Wiggle`). Analog movement is scaled ×110 into the stick byte, so full speed starts at ~87% tilt; keyboard movement wins when both are used.

**Settings menu** (F10 or Select+Start; Escape stays the game's pause). Four pages: keyboard & mouse bindings, gamepad bindings, camera & gamepad (mouse/stick sensitivity, invert Y, auto-follow, deadzone, flick/circle threshold, rumble on/off and strength), video/color/audio (internal resolution auto/1–4 — next launch, vsync, borderless fullscreen, color mode, master volume, restore all defaults). Navigation is fixed so a bad binding cannot lock you out: arrows/WASD/D-pad/left stick, Tab/PageDown/RB and Shift+Tab/PageUp/LB for pages, Enter/Space/A to add a binding or change a value, Delete/X to clear, Backspace/Y to restore a row's default, Esc/F10/B/Select+Start to close. Adding a binding: press and release a key, mouse button or combination; on the gamepad page hold RB/LB and flick/rotate the right stick for gesture chords. A chord taken from another action is removed there (the message says from where). While the menu is open, and until every input is released after closing, the game receives a neutral controller and camera look is zero. Every change saves immediately. F1 help is generated from the current bindings.

**Rumble.** `WPADControlMotor` (8032CACC) in `runtime/game/input_hle.cpp` now also calls the optional host symbol `mw_pad_motor(pad, chan, on)` (native-platform `pad.rs` → `host::rumble(chan)`); the frontend renews a 150 ms `GamepadRumbleRequest` every 100 ms while channel 0's motor is on (strength from settings, off when unfocused or in the menu) and stops it when the game turns the motor off. A core built before this change never calls it, so rumble simply stays off until the next core rebuild.

**Diagnostics (default off).** `MW_PC_DIAG_PAD=start,secs,PadA+PadRB;…` scripts a virtual gamepad through the same bindings (tokens are pad names plus `LStickUp/Down/Left/Right`, `RStickUp/Down/Left/Right`) and logs `[pc-pad]` packets; `MW_TRACE_PAD=1` logs every change of what the pad HLE hands `KPADReadEx` (`[pad-hle] hold=… stick=… remote=… nunchuk=…`).

**Verification (2026-11-08).** 22 unit tests pass: mapping, chords and shadowing, stick flick/rotation, analog movement, config round trip, partial and broken files, file save→load, defaults equal to the old fixed bindings, menu adjust/render. The 10 native-platform pad/motion tests pass too, plus a new one that checks the motor flag. In a 100 s windowed run (`evidence/pc-controls-diag/run.log`) the game read the custom config `evidence/pc-controls-diag/pc-controls.toml` (jump↔dodge swapped on the pad, volume 0.5, invert Y), with that file reported as `loaded`. The scripted virtual pad reached the game's KPAD read as:
- PadA → WPAD A `0800` (eleven presses through boot and the skippable intro event)
- left stick up → stick y 1.0
- PadY → Z `2000` (the remapped jump)
- PadRT → B `0400` + remote Up gesture
- RB + right-stick flick → remote Right gesture
- L3 → Shake on both arms
- PadX → Nunchuk Back (the remapped dodge)

Not verified: a physical controller, the settings overlay driven by hand, rumble (it needs a core build that includes the `mw_pad_motor` hook), and a full fight played on a controller.

The native window and headless action tests share crates/mw-game/src/native_controls.rs. F1 toggles an optional PC controls panel; it is not a replacement for every original Wii glyph/diagram. Current bindings: WASD movement, left mouse/E attack or hold to grab, hold right mouse chainsaw with a default vertical gesture, Space jump, Shift dodge, hold X to struggle, Q/middle mouse camera or hold lock-on, F/R directional strikes, G pickup, T taunt,1/2 weapon selection, Enter confirm, Escape pause, Backspace/right mouse menu back. Escape no longer closes the application. The normal window-close path remains available.

Arrows produce right-arm directional events; IJKL produce left-arm events. N/M and U/O provide push/pull, while V/B and H/Y provide circles in either direction. These are input profiles, not replacement combat functions or automatic finishers. Action recognition, animation, collision, damage, energy and timing remain original game code. All advanced gesture/action outcomes still need native verification.

Sources for original roles: SEGA instruction booklet (publisher-authored scan at https://manualzilla.com/doc/6982051/sega-madworld-10086650198-user-s-manual), and the developer's explanation at https://www.platinumgames.com/official-blog/article/1210. PC key choices and generated sensor profiles are port adaptations.

Wire API remains eight bytes: buttonsBE16; Nunchuk Z/C bits; stickX/Y; legacy shake flag; remote gesture token; Nunchuk gesture token. Tokens use a four-bit event sequence and four-bit profile number. Profiles1..6 are signed X/Y/Z impulses;7/8 are circles. Profile9 is a repeated three-axis shake while X is held; release cancels only active shake channels. Directional/circle tokens stay stable after key-up so taps survive host/native poll differences; those profiles do not automatically repeat. Zero cancels. The default right-mouse gesture cancels when that button is released, so it cannot become an unintended unarmed strike. Unfocused windows publish neutral state and cancel both motion channels.

The hardware adapter samples each event over64 nominal200Hz HID ticks, with20 initial ticks when chainsaw activation begins. Pulse timings are adaptations requiring play testing. An opaque Controller handle owns the existing Bluetooth transport plus two pulse states; the existing hardware C ABI pointer signatures remain unchanged. The first six packet bytes retain their previous meanings. Nunchuk sensor input now packs all10 accelerometer bits before the existing extension encryption, preserving stick and active-low buttons. Calibration reference: https://github.com/dolphin-emu/dolphin/blob/409881c34357d0f105fd473167d15ab0bd9c1628/Source/Core/Core/HW/WiimoteEmu/Extension/Nunchuk.h . The live Nunchuk override clears on attachment/load and does not change the existing serialized state layout; the native hardware instance does not use save-state APIs.

Verification:14 hardware tests pass;2 focused native Nunchuk report-packing tests pass;7 frontend tests pass (four PC-control contracts and three PCM tests). The headless control-ready replay finishes all scripted phases with source input bits arriving and Jack moving from the original spawn toward the arena side. Native controls have an additional lock phase after Rin initialization; the test waits for observed demo-counter0/game-flags0 after exercising normal mapped inputs. This is a diagnostic readiness heuristic, not a production state patch. No game flags, actor coordinates, health or animation state are written.

The five-minute sustained replay completes, but sampled Rin HP remains1600. Jack reaches an arena boundary while Rin stays out of range. This does not establish attack, dodge, jump, lock-on, finisher or win/loss correctness. Next combat validation must steer relative to the native camera/enemy, and observe native action states. UI prompt removal, device audio, Bevy window behavior, real-time speed and pixel parity remain open.

Original gesture vocabulary: player-input-pcode.graph.jsonl / player-input-pcode-report.json contain four bounded functions, including cPl0000_frameUpdate and checkSwing/checkRemoteAcc/checkNunchukAcc. player-gesture-pseudocode.json exposes the original selector/threshold checks. Remote selectors0/1 compare signed channel80698B4C,2/3 channel80698B48, and4 self+F68; Nunchuk uses80698B58/54 and self+F6C. Runtime slot stride is80hex; controller-index derivation is not fully resolved here. Idle first-slot samples are0,-1,0 for each device, so these post-SDK channels must not be equated blindly with raw sensor axes. The original recognizers remain executable native code.

UI source reconnaissance: ui-source-index.json records locally decoded owned TPL pages onD. The examined subscr000/001 and ex000 pages contain menu/decorative artwork; controller sprite identities and IDD layout are not yet established. wc.idd has no embedded TPL magic found by the current probe. No asset override or original UI replacement is claimed from this inventory.


## 2026-11-08 — Native PC feedback and motion identity

Added test-only read-only actor feedback to the headless PC sequence. It learns native W/D world displacement and drives the shared PcControls mapper toward Rin, without writing game flags, actors, HP, camera or motion state. First replay was interrupted by a Windows feedback-file sharing error; preserved as failed/partial evidence. Separated reader output and atomic feedback publication, added bounded sharing retries and terminal error reporting. The second300s replay exits0, completes all action phases and6,118 native XFB presentations/host blit. These are presentation counts, not rendered FPS or parity proof.

Recovered13 bounded original motion/camera/attack-observer functions into motion-state-pcode.graph.jsonl with INDEX/API/pseudocode. The source catalog index includes248 pl0030 MOTs. All275 concurrent motion observations match a complete original resource by SHA256, after header-only lookup.233 observations establish native lock-on state1 targeting Rin's exact object. During jump-attack phase Jack Y rises from17.598827 to19.082829; original movement/jump recognition evidence, not timing/trajectory equivalence. Sustained combat reaches1.00125 units3D separation, but Rin stays1600HP. Jack reaches0HP after inputs are released; loss UI/flow not verified. Native hit/contact handling remains open. Three attack-observer bytes are now exposed by the reader for the next action-phase trace; this replay did not sample those fields during actions.

Seven frontend tests and all three embedded lifecycle/error cases pass. No Computer Use, window, screenshot or audible-device test. Original shading/event timing and original Wii UI replacement remain open. Evidence: native-pc-motion-summary.json; source hashes native-pc-feedback-local-files.json. Next bounded investigation: native attack-window bytes and hit/contact registration, rather than repeating fixed-direction combat. Full goal remains active.


X now emits a right-arm/right and left-arm/left impulse together without pressing native action buttons. This is an explicit PC input adaptation for two-arm struggle prompts, not an automatic competition win. Eight frontend tests pass, including both-channel event-edge/no-button/blur behavior. Complete native competition handling is still under test.


## 2026-11-08 — Boss damage, paired role contents and two-arm input

Approach-first headless PC strategy records Rin HP1600->1550 in a clean300s/6,201-presentation replay. Follow-up native-pc-rin-pair replay records minimum1534HP and88 simultaneous native paired-role observations. Every observed paired MOT matches complete source contents: player04AA/AB/AD with Rin0110/0111/0113. The source camera and moveCatchedPos role dispatch are recovered, and native state/peer/position observations are indexed as a typed graph. This verifies those source role selections, not a complete authored phase chain, placement/visual/timing equivalence or successful competition. Expanded live index from248 to all4,082 original catalog MOTs, rehashing their source bytes.

Added X two-arm input. Initial directional impulses were sampled below the original0.5 metric thresholds (~0.36), and the test also released inputs before the graded AB phase. New bounded Shake profile9 alternates a three-axis waveform within Nunchuk calibration; normal directional profiles unchanged.15 hardware tests and8 frontend tests pass. Hardware DLL rebuild passed; frontend rebuild is in progress/completing. Current bounded replay bevy-native-pc-rin-shake tests the new profile and continues X during an observed competition after the normal action timer, returning to neutral once it ends. No native state is written. Outcome is not yet recorded.

Original HUD controller atlas found/decoded in etc/core2_us.dat#core.idd (1024x1024); reproducible tools/index_ui_sources.py and UI-PC-PROMPTS.md. No UI texture override implemented. Explorer HTTP regression confirms existing Color version checkbox and default-off behavior across all models/event roles. All generated source assets/evidence remain onD; no Computer Use. Native boss damage is now observed; full combat, PC UI replacement, device/window/audio output and original rendering/timing equivalence remain open.


## Latest verification

The held-input battle replay completes300s and6,169 XFB presentations, with Rin HP reaching1489 from1600. It does not enter a competition, so complete held-X handling remains open. Earlier native observations establish three paired role-resource phases by complete hashes, and finite shake clears the original two-arm scalar thresholds. The original eight-entry motion-filter producers are now recovered for targeted follow-up.

The Bevy AudioPlugin/default-device sink test passes using one second of captured native AID PCM at3% test volume. It confirms that NativePcm reaches a consuming output sink; human audibility, integrated battle timing/mix and all sound routes remain unverified. Three current embedded lifecycle/error cases and15 hardware/8 frontend tests pass. Hardware DLL and frontend executable have been rebuilt. Original Wii controller graphics are located in the common HUD atlas but have not been replaced. The normal window/render schedule, full clash/win flow, original shading/timing and performance remain open. No Computer Use was used.


## 2026-11-08 — Active PC prompt resource

DI tracing corrected the initial atlas lead: this boot reads standalone etc/core_us.idd, not core2_us.dat. The new converter stages 11 PC prompt slots in the active resource, retaining exact file length, all palettes, other sprites and native timing/layout commands. The frontend now selects that staged file through a single-file DI override. All three modified full CI4 pages match in live guest RAM. Device-card transforms, live legibility and complete contextual/Wii UI removal remain unverified. See UI-PC-PROMPTS.md and native-pc-ui-live-memory.json. Eight frontend tests and the native overlay size-check/lifecycle cases pass.


## 2026-11-08 — Complete native clash and follow-up observed

The natural bevy-native-pulsed-clash replay (fixture disabled, no fixture calls) uses the shared PC mapper with chainsaw press/release and right-strike cycles, then held X during the original competition. It observes player phases1/3/5/6/7, native raw meter rising to1 and smoothed meter~0.985, original paired AA/AB/AC with0110/0111/0112, and ordinary-state return at274.719s. Rin HP1600->1300; player HP70 remains70 across the clash. All six original role MOT contents rehash against owned source bytes;144 paired observations, including60 success-pair snapshots. No host writes to gameplay fields. No input amplitude/profile change was needed. Existing held-X profile9 passed actual native checks.

The separate bevy-native-clash-fixture replay first established the same success path through an explicit default-off original-API attack/contact fixture. It is labelled synthetic and remains useful for isolated regressions; natural contact is independently proved by the pulsed replay. tools/audit_native_clash.py --label bevy-native-pulsed-clash --require-natural validates the recorded phase/input/source-content/return evidence. Full-fight win, other advanced actions, original animation speed, window/UI legibility and visual/mix/timing fidelity remain open. See the labelled clash summaries, paired graphs and CLASH-INPUT-REQUIREMENTS-INDEX.md.


## 2026-11-08 — Hold Enter to skip cutscenes; strap screen skip

**Cutscene skip (frontend, works on the current core).** Hold Enter while an original event is playing. A white ring (bottom-right, "SKIP") fills over 1.2 s. If you let go early it resets, and a short tap is still Remote A (confirm). When the ring is full, the frontend presses Remote B once (for up to 0.5 s, only while the event is still playing). That is the game's own skip input: the original event update 800A4864 (state 5) calls 800A49F0, which returns true when the event's skippable byte (+1) is set and the game's pad-down has bit 0x20. cPadSystem_updateChannel 8020EFC0 maps WPAD B 0x0400 (wire 0x0004) to that bit. The game then starts its own fade (800A7394), enters state 6 and ends the event through 800A47DC, exactly as a console B press would. "Cutscene playing" is read-only. `native_platform::host::peek_mem1_u32` reads the event-player singleton 80606CF8 (guard 80606CE8; word+4 state == 5, byte+1 skippable). Nothing is written to guest memory. After a skip, the held Enter stops reading as A until it is released, so it cannot carry into gameplay as a held attack. The default-off diagnostic `MW_PC_DIAG_HOLD_ENTER=start,seconds` simulates a held Enter, and the log prints `[pc-skip]` lines. Unit test `holding_enter_in_skippable_event_presses_b_once_and_taps_still_confirm` passes. Confirmed by the user in a playtest of the release build on 2026-11-08 ("hold enter worked").

**Strap screen removed (core).** The boot "put on the wrist strap / allow adequate room" screen is fn 8026F538. Boot fn 8026EF0C calls it exactly once and ignores its result. It loads strap/strap_%s.tpl and runs its own frame loop: fade in, up to 600 frames or a press, fade out. hle.txt `0x8026F538 RemovedStrapScreen` with `runtime/game/pc_removed.cpp` replaces it with only the original's first frame, drawn invisibly: load the strap texture (8021FB84), GX setup (80371A04/80374474), its two quads (8026F838) with the original white colors at alpha 0, end of frame (8025BEB4/8026F324), then free the texture (8021FD18, twice, as the original does). There are no timers, fade, press wait or option.
Why not an empty body: the 70 s headless test with an empty body (and with 1 or 150 blank frames that skip the texture and quads) ends about 14–18 s into boot with `Core exited -1: truncated GX display list command`. Later boot depends on that screen's GX/texture work. The exact dependency (vertex/TEV state vs. heap layout) is not identified. Evidence: `evidence/bevy-native-strap-removed{,2,3}.log` fail; `-strap-isolate.log` (original screen) and `-strap-removed4.log` (current) pass: BossSw, event 21 requested, 70 s, host blit complete. The earlier cIdStrapManager hook (80246CC8) was the wrong screen and is deleted.

**Remote shake and the finisher QTE (core input HLE).** Controller input now goes straight to the game's KPAD reads (research/INPUT-HLE.md), so a PC shake produces a real remote/Nunchuk accelerometer sample. The game's own detectors measure it; nothing writes game state.

- **Hold X: shake anywhere.** This drives both the two-arm struggle and Jack's finisher. In Jack state 68 / selector 3, phase 13 reads only `+0x1088`, the mean shake magnitude (research/VICTORY-MOTION-INPUT.md).
- **A/D back and forth: the shake QTE.** It is active only while the game itself shows its shake prompt, meaning the cockpit's remote prompt kind is 0x11 (setSwingNavigation enum 5). The core reads that kind from cPlayer +0x11C8 → +0x24 → +0x18. Two or more strong left/right stick reversals within 0.4 s count as shaking, so a gamepad stick works too. Tapping faster keeps the shake going, and the original integrator decides how fast the meter fills. Outside the prompt, A/D is plain movement, and it does not shake under the two-arm competition prompt (0x10). F1 help lists both bindings.
- **Evidence.** `evidence/input-hle-finisher-x.*` is a headless held-X replay on core `48864ab0`. Route: the test-only `MW_TEST_RIN_FINISH_WAIT=1` fixture (Rin's original changeState to finish_wait), then the harness walks to Rin and starts the finisher with right mouse (chainsaw), which takes player state 39 → 68/11/3. The rest is natural, through the X binding only:
  - phase 13: `+0x1088` = 1.80, and `+0x1248` reaches its 0.1 cap
  - phases 14 → 16 → 17
  - Rin 26/3 → 26/14
  - stop flags 9/10 (`0x40600`)
  - `subMission clear` and `roomClear`, then scenes e01 → b03 → event 73 → 105
- **A/D status.** The A/D QTE is unit-tested (`pad::tests::stick_wiggle_shakes_only_under_the_shake_prompt`). Its headless replay (`MW_GPU_TEST_PC_FINISH_WIGGLE=1`) was cut short: `evidence/input-hle-finisher-ad.*` is incomplete because the machine was loaded by a parallel build. The user will test it by hand.
