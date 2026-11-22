# Input HLE: replace the emulated Bluetooth stack with direct WPAD/KPAD hooks

Status: **implemented and verified** (2026-11-08) on core `ee4449dc` (approved plan below, kept for reference). The default is HLE; `MW_INPUT_HLE=legacy` runs the original Bluetooth path from the same DLL, and `MW_INPUT_HLE=compare` runs legacy and logs KPADStatus differences. The legacy sources are still present until removal (§8 step 6).

## Results

| check | evidence | result |
|---|---|---|
| boot without Bluetooth | `evidence/input-hle-pc-sequence.log` | `[input] HLE controller active`; `/dev/usb/oh1/57e/305` never opened; control-ready reached |
| movement / attacks | `input-hle-pc-sequence-feedback-observations.jsonl` | W moves the player 7.2 units; LMB plays motions `1100..1103`, RMB plays chainsaw `1104`; jump, dodge and strikes play their motions |
| gestures (KPAD axes) | same | the Right gesture appears in the game's slot `0x80698B48` as (−2, −1, 0), matching KPAD = (−X, −Z, Y) |
| held X (`+0x1088`) | `input-hle-motion-probe*` | mean 1.547 (1.11–2.21); `+0x1090` max 1.457, `+0x109C` max 1.451; 0 after release. The legacy path measured 1.586. |
| remote speaker | `input-hle-speaker-trace.log` vs `-legacy.log` | the same ControlSpeaker sequence, ON (cb 804F7FC8) → PLAY (804F7ED0) → MUTE, then IsSpeakerEnabled = 1 and streaming. The game keeps the speaker muted in this scenario on both paths, so 0 samples are mixed on either. |
| user playtest | coordinator report | "controls seemed to work too" |
| core-thread CPU | `evidence/input-hle-thread-cpu.jsonl` | No difference. The core thread is pegged at about 0.97 CPU-s/s on both paths because of frame pacing, so per-thread CPU is not a useful metric. The removed work is about 200 HID reports and about 150 speaker packets per second of IOS round trips, now replaced by one host call per read or packet. A sampling profile is still to do. |

Findings during implementation:
- **Connect event timing.** The sound library (`804F9B20`) replaces cPadSystem's KPAD connect callback with its own `804F8000`, and only that callback switches the speaker on. With Bluetooth, the remote connected after both registrations, so the game's `ConnectCallback(0)` never ran. The HLE now reports the connection once the callbacks have been stable for 30 reads.
- **Strap screen.** The 14 s boot stop seen during testing was skip-splash's empty strap-screen body ("truncated GX display list command"), not this change. They fixed it in core `0892d62d`.

Sources:
- the recompiled original code in `tools/upstream-native-core/port/generated/recomp_*.cpp`
- `tools/ghidra-functions.tsv`
- floats read from `exports/madworld/extracted/disc/DATA/sys/main.dol`
- `native-platform/src/{input,motion,speaker,snapshot}.rs`
- `runtime/ios/platform_bluetooth.cpp`
- gecko `hollywood/ipc/usb/wiimote.rs`

SDK names marked *(inferred)* come from behaviour and Revolution SDK layout, not from symbols. The only real symbols are `KPADReadEx`, `KPADiRead` and `cPadSystem_*`.

## 1. Recommendation in one paragraph

The game talks to the Wii remote through about 25 WPAD/KPAD SDK functions (§3). There is exactly **one controller read**: `cPadSystem_updateChannel` (8020EFC0) calls `KPADReadEx` (8036D950) once per channel per frame and uses only `status[0]`. There is exactly **one init chain**: `KPADInit` (8036E260) → `KPADiInit` → `WPADInit` (8032B15C) → `WUDInit` (80338F88) → BTE → IOS `/dev/usb/oh1/57e/305`.

Replace those boundary functions with host code using the existing `PPC_FUNC` + `hle.txt` mechanism:
- `KPADInit` no longer calls `WPADInit`, so the Bluetooth device is never opened. HCI, L2CAP, HID, the 200 Hz IOS polling event and the guest WPAD/KPAD libraries all go dead.
- `KPADReadEx` fills a 176-byte `KPADStatus` straight from the `host::set_controller` packet.
- Speaker audio is taken at `WPADSendStreamData` (8032EB8C): 20 ADPCM bytes become 40 PCM samples at 6 kHz, which go into the existing `push_remote_mono` lane.

The packet format, `PcControls`, gestures and `SpeakerMixer` stay as they are.

## 2. What happens today, and what it costs

```
Bevy PcControls -> host::set_controller (8-byte packet)
  -> OS event every 303750 tb (5 ms, 200 Hz): mw_bt_tick -> gecko Bluetooth/Wiimote -> HID input report
  -> pending USB bulk-in ioctlv replies -> guest BTE/WUD (HCI+L2CAP parse) -> WPAD HID parser + ring
  -> KPADiRead (calibration, axis remap, DPD maths) -> KPADStatus[16] -> cPadSystem_updateChannel
speaker: AXRmt alarm 804F7C74 (every ~6.7 ms) -> WENCGetEncodeData -> WPADSendStreamData
  -> WPAD queue -> HID 0x18 -> L2CAP/ACL -> USB ioctlv -> gecko speaker decode -> push_remote_mono
```

So each second costs about 200 input reports and about 150 speaker packets. Each one is a full IOS ioctlv round trip, and each runs guest BTE/WPAD code on the core thread. The new path costs one host call per `KPADReadEx` (4 channels × 60/s) and one per speaker packet.

## 3. The game-facing boundary (complete list)

I computed this by listing every function in the WPAD/KPAD library range (0x80329190..0x8036F118) that is called from outside the range. There are no other entry points. Internal WPAD functions (0x803291B4..0x8032AEFC, WUD 0x80338xxx, BTE 0x8033..0x8036) are reached only from inside the library.

Callers in 0x804F..0x8051 are the Home Button Menu (HBM) and its remote-speaker helpers.

| addr | SDK function | game callers | HLE behaviour |
|---|---|---|---|
| 8036E260 | **KPADInit** *(inferred, sole init)* | cPadSystem init 8020EE00 | Create host state. Do **not** call the original, so WPAD/BT never start. Queue a connect event for chan 0. |
| 8032B1D8 | WPADRegisterAllocator *(inferred; 4-byte thunk → WUD)* | 8020EE00 | no-op |
| 8032B1DC | **WPADGetStatus** *(thunk → WUDGetStatus)* | 8020EE00 spins `until ==3`, HBM 804F9B20 | return 3 (setup done). Without this, boot hangs. |
| 8036B07C / 8036B098 / 8036B0B4 / 8036B0D0 | KPADSet{Pos,Hori,Dist,Acc}Param *(inferred)* | 8020EE00 per channel | Store the values. The game passes (0.1, 1.0) for pos and (0.0, 1.0) for the other three (DOL 80561F38..40), which means no smoothing. |
| 8036E820 | **KPADSetConnectCallback** (stores at KPAD chan+0x550) | 8020EE00 → 8020ED28, HBM 804F9B20 | Store the callback per channel. Deliver pending connect events (§6). |
| 8036E888 / 8036E8A4 | KPADDisableDPD / KPADEnableDPD *(inferred; byte chan+0x556 = 0/1)* | 8020F704 | store the flag (used only by the optional pointer, §9) |
| **8036D950** | **KPADReadEx**(chan, KPADStatus\*, 16, s32\* err) | 8020EFC0 only | §4. Chan 0 returns 1 with err 0. Chans without a controller return 0 with err −1. |
| 8032C564 | **WPADProbe**(chan, u32\* type) → 0 / −1 / −2 | game 800B8AD0, HBM | chan 0: type = 1 (nunchuk), return 0. Others: −1. |
| 8032C7A0 | WPADSetDataFormat(chan, fmt) | connect cb 8020ED28, HBM | Record it and return 0. The HLE always supplies full data. |
| 8032C418 | WPADDisconnect(chan) | 8020ED28 (chans 2/3), 8020EFC0, HBM | Mark the channel disconnected. Chan 0 reconnects on the next read (queue disconnect then connect events). |
| 8032B1E0 | WPADGetRadioSensitivity *(inferred; u8 cb+0x983, 0..100, written by 803295E0)* | 8020EFC0, 8020F568 (0 means the "no signal" warning), 8026C1D8, 80236AD8/80236C18 | 100 if connected, otherwise 0 |
| 8032C8AC | WPADGetInfoAsync(chan, WPADInfo\*, cb) | 8020EFC0 every 360 frames (cb 8020F694), HBM | Fill `WPADInfo` with battery 4, no low-battery, attach = 1, speaker = state, then queue `cb(chan, 0)`. |
| 8032CACC | **WPADControlMotor**(chan, on) | cPad::startVibration 8020F7B0/8020F7DC, 8026F460, 800B8388, 80231290, HBM | Store, and forward through the optional host symbol `mw_pad_motor(pad,chan,on)` → `host::rumble(chan)`; the frontend drives gamepad rumble from it (research/PC-CONTROLS.md). |
| 8032C4D8 | WPADEnableMotor *(inferred; global byte r13−0x73CA)* | 8020F76C | store |
| 8032C50C | WPADResetAutoSleepTimeCount *(inferred; uses OSGetSystemTime)* | 8020F774 | no-op |
| 8032D2EC | **WPADIsSpeakerEnabled** (cb+0x83C = status-report bit 2, written by 8033219C) | AXRmt alarm 804F7C74, HBM | speaker ON flag (§5). Also delivers pending callbacks. |
| 8032D330 | **WPADControlSpeaker**(chan, cmd, cb): 0 off, 1 on, 2 mute, 3 unmute, 4 play | 8026C1D8 (mute 2 / unmute 3 for chans 0/1), HBM, speaker setup 804F6DBC/804F9C0C/805018A8 | Update speaker state and reset the decoder on on/play. Queue `cb(chan, 0)`. |
| 8032EB08 | WPADCanSendStreamData *(inferred)* | 804F7C74, HBM 80508B24 | 1 if speaker ON, otherwise 0 |
| **8032EB8C** | **WPADSendStreamData**(chan, adpcm, 20) | 804F7C74, HBM 80508B24 | Decode Yamaha ADPCM into 40 samples at 6 kHz. Push to `push_remote_mono` only when ON and unmuted. Return 0. |
| 8032B1CC/D0/D4 | WUD simple-sync thunks *(inferred)* | HBM only | no-op / 0 |
| 8032CB98, 8032CBCC, 8032CDE8, 8032E94C, 8032E980, 80330918, 8032C678, 8032C700 | HBM settings (speaker volume, motor enable, SC save, DPD sensitivity…) *(inferred)* | HBM only | Accept, store, return 0. HOME (wire 0x0080) is not bound on PC, so HBM is not reachable today. |

`WENCGetEncodeData` (8032851C) stays original. It is pure computation and HBM depends on its output buffer.

## 4. `KPADStatus` fill (0xB0 bytes, big-endian; only `status[0]` is read)

Fields that `cPadSystem_updateChannel` reads, where `slot` = 0x80698B38 + chan·0x80 and `kpad` = slot base + 0x4C0 + chan·0xB00:

| off | field | host source |
|---|---|---|
| 0x00 | hold (WPAD bits) | `swap16(BE packet[0..2])` & 0x9F1F, plus Z 0x2000 / C 0x4000 from `packet[2]` bit0/bit1. The packet carries remote **wire** order, and WPAD is that byte-swapped. The game remaps: A 0x800 → 0x10, B 0x400 → 0x20, and so on. |
| 0x04/0x08 | trig/release | Edges against the previous read. The game computes its own edges and ignores these. |
| 0x0C..0x14 | acc x,y,z (g) | remote gesture sample in KPAD axes (below) |
| 0x18/0x1C | acc_value/acc_speed | ‖acc‖ and Δ‖acc‖ |
| 0x20..0x50 | pos, vec, speed, horizon, hori_vec, dist… | DPD is invalid today. Copy the values the legacy KPAD reports with no IR (phase 0 capture). The game copies pos +0x20/+0x24, horizon +0x34/+0x38 and dist +0x48. |
| 0x54 | acc_vertical | from acc |
| 0x5C | dev_type | 1 (FREESTYLE = nunchuk), or 0xFF when not connected |
| 0x5D | wpad_err | 0 |
| 0x5E/0x5F | dpd_valid_fg / data_format | capture values (8020F568 tests +0x5F) |
| 0x60/0x64 | nunchuk stick x,y | (packet[3]−128)/127 and (packet[4]−128)/127, clamped to [−1, 1]. The game's direction thresholds are ±0.5 (DOL 80561F48/4C). |
| 0x68..0x70 | nunchuk acc x,y,z | nunchuk gesture sample in KPAD axes; rest is (0, −1, 0) |
| 0x74.. | nunchuk acc_value/speed | derived |

**Axes.** gecko encodes a gesture `[x, y, z]` in raw remote axes, where rest is z = +1 g. KPAD reports rest as (0, −1, 0). The expected mapping is KPAD = (−x, −z, y). Phase 0 checks this mapping and its signs for every gesture before it is fixed (§8).

**Motion timing is preserved.** `Pulse` was stepped at 200 Hz by the BT event. The HLE keeps a 200 Hz virtual clock: on each `KPADReadEx` it steps `Pulse` `floor((tb − last_tb)/303750)` times (capped at 64) and uses the last sample. Waveforms, the 20-tick chainsaw delay and the shake half-period of 8 ticks therefore keep their real-time shape. The finisher analysis (`victory-motion-input-handover`, +0x1088 ≈ 1.59 on held X) should still hold, and phase 0 checks it. `packet[5]` (legacy sine shake) maps to Gesture 9. `PcControls` never sets it.

## 5. Remote speaker

The state per channel is `{on, muted, decoder(ADPCM predictor 0, step 127), volume}`:
- ON/PLAY resets the decoder.
- MUTE and UNMUTE flip `muted`.
- OFF clears everything.
- Volume defaults to 88/127. This is the value WPAD wrote to register 5, and the legacy trace observed it. It is exposed as `MW_REMOTE_SPEAKER_VOLUME` so it can be changed freely.

`WPADSendStreamData` always advances the decoder, matching the hardware rule that data advances while muted. It pushes PCM only when the speaker is on and unmuted, through `audio_sink.push_remote_mono(samples, 6000, volume)`. `SpeakerMixer` is unchanged.

`REMOTE-SPEAKER-NEXT.md` found that in all observed play the game kept the speaker **muted** while it streamed. The HLE must keep that behaviour: no forced unmute. Phase 0 compares mute/unmute command traces between the legacy and HLE paths.

The Yamaha ADPCM decoder moves from gecko `usb/speaker.rs` into `native-platform/src/speaker.rs`, keeping the Dolphin attribution. The current packet → 0x18 report → register path is then deleted.

Optional later improvement (not in this change): take PCM before encoding, from `AXRmtGetSamples` (80317A70), so audio skips the lossy ADPCM round trip.

## 6. Callback delivery

Original WPAD callbacks run later, from BT-stack context with interrupts off. A synchronous call from inside `WPADControlSpeaker` could re-enter a caller's state machine before that caller has stored its own state. Callbacks therefore go into a host FIFO of `(guest fn, chan, result)`. The queue is drained at the **entry** of `KPADReadEx` and `WPADIsSpeakerEnabled`, whichever runs first:
- `WPADIsSpeakerEnabled` is polled every ~6.7 ms from the AXRmt alarm, which is already interrupt context.
- Each callback is called with `ppc_call_indirect(c, m, fn)`, with `r3 = chan`, `r4 = result` and MSR[EE] cleared. The caller's registers are saved and restored, following the `pc_skips.cpp` pattern.

The first connect event `(chan 0, 0)` is delivered this way, so the game's own `8020ED28` runs and calls `WPADSetDataFormat` exactly as it does today.

## 7. Files

New:
- `port/runtime/game/input_hle.cpp`: all `PPC_FUNC`s from §3, the `KPADStatus` writer, the callback FIFO, and snapshot of the FIFO and per-channel flags.
- `port/recomp/hle.txt`: about 30 entries. The build regenerates from the DOL recorded in `generation.json`.

Changed:
- `native-platform/src/input.rs`: replace `mw_bt_*` with a small C ABI:
  - `mw_input_create/destroy`
  - `mw_input_sample(h, chan, tb, *MwPadSample)`: a plain struct of buttons, f32 accel ×2 and stick
  - `mw_input_speaker(sys, h, chan, cmd)`
  - `mw_input_stream(sys, h, chan, ptr, len)`
  - `mw_input_rumble`
  - `mw_input_state_save/load`

  `MW_INPUT_FILE` reading moves here from `platform_bluetooth.cpp` (standalone `madworld.exe` tests). The guest memory layout stays in C++. Rust only produces values.
- `native-platform/src/{speaker,snapshot,host}.rs`: decoder, new blob, symbol table. `motion.rs` is unchanged.
- `port/runtime/cpu/snapshot.cpp`: replace `native_bluetooth_state` with `native_input_state` and bump the snapshot version.

Deleted after A/B (see below):
- `runtime/ios/platform_bluetooth.cpp`
- `runtime/ios/bluetooth.cpp`, the C++ HCI stub registered at `/dev/usb/oh1/57e/305`
- `hci.h`
- the gecko `usb::Bluetooth` dependency of native-platform (gecko itself is vendored and left alone)

Unchanged: `crates/mw-game` (packet format, `PcControls`, `CutsceneSkip`, HELP).

**A/B switch during the transition.** With `MW_LEGACY_BT=1`, every hook tail-calls its `f_X_recomp` original and the IOS device stays registered. Both paths then live in one DLL for parity tests. Once phase 3 passes, remove the switch and the deleted files in a separate step.

## 8. Phases and acceptance

0. **Golden capture on the current DLL (read-only, no rebuild).** Run the current build with `MW_INPUT_FILE` scripts covering:
   - neutral
   - each button
   - 8 stick directions
   - gestures 1..9 on the remote and the nunchuk
   - one speaker session

   Read `KPADStatus[0]` per sample and the speaker command sequence with `tools/read_native_core.py` (extended read-only), and write the results to `evidence/input-hle-golden.json`. This fixes the axis signs, stick scale, DPD-invalid defaults and +0x5F. It needs the coordinating session's OK, because it runs the game.
1. Rust unit tests:
   - packet → `MwPadSample` against the golden table, within 1 raw LSB (gecko quantised accel to 10-bit, and HLE does not)
   - 200 Hz stepping
   - ADPCM decode against gecko's codec vectors
   - mute gating
   - snapshot round trip
2. Build with hooks and `MW_LEGACY_BT` available. Boot to title in HLE mode. The log must show `[input] HLE controller active` and **no** open of `/dev/usb/oh1/57e/305`.
3. Parity: the same input script under both modes gives equal `KPADStatus[0]`, within tolerance, at the same game frames. Then the owned GPU action tests pass in HLE mode: menu confirm, move, attack, cutscene skip, held-X finisher (§4).
4. Speaker: `MW_TRACE_REMOTE_SPEAKER` shows the same ControlSpeaker and mute sequence, about 150 stream packets/s, and mixer counters consistent with the mute state.
5. Performance: compare `MW_PROFILE_NATIVE_IO` core-thread time before and after, and expect the Bluetooth IPC cost to disappear.
6. Remove `MW_LEGACY_BT`, the BT sources and the gecko `usb` usage. Update `INPUT-API.md` and `REMOTE-SPEAKER-API.md`.

## 9. Features that become easy afterwards (not in scope)

- Mouse pointer, by filling KPAD `pos` and setting `dpd_valid_fg` while DPD is enabled.
- Real gamepad rumble from `WPADControlMotor`.
- A second local player on chan 1. Today only chan 0 exists.
- A HOME-menu binding.
- Taking speaker PCM before encoding (§5).

## 10. Risks and unknowns

- **Snapshots.** Existing snapshots hold guest WPAD/BTE state and the gecko BT blob, so they will not restore under HLE. Proposal: bump the version and reject older snapshots with a clear message. Do this only after the coordinating session has finished the GPU snapshot tests tied to the current DLL hash.
- **Guessed defaults.** KPAD values computed with no DPD (pos, horizon, dist, acc_vertical) are guesses until phase 0. Some game code may depend on them, for example 800B8AD0 copies horizon.
- **Direct WPAD reads.** I found no game code that reads WPAD control blocks or `KPADStatus[1..]` directly. The count at slot+0x30C0 is only written. Callers reached through function pointers that I did not trace (vtable or alarm targets) could still exist. The boundary scan covers direct calls, plus the alarm targets 804F7C74 and 80508B24 found through their callees.
- **WPAD shutdown.** `OSRegisterShutdownFunction` is called from inside the library. With `WPADInit` skipped it is never registered, so reset and return-to-menu no longer wait on WPAD. That is fine on PC.

## 10a. Coordination constraints (from the main session, 2026-11-08)

- **Do not edit these; they belong to other sessions:**
  - `crates/mw-game/src/native_controls.rs`, `native_main.rs` and the remote-splash boot flow (skip-splash)
  - the gecko renderer (color-render)

  This plan needs none of them. The packet format and `PcControls` are unchanged. The only gecko change is the native-platform *dependency* on `usb::Bluetooth`, and the vendored gecko sources are not touched.
- **Message local_4942a8a3 before every `madworld_core.dll` rebuild**, so that only one session rebuilds at a time.
- **Shake input must keep working.** `+0x1088` shake input (`VICTORY-MOTION-INPUT.md`) is an acceptance gate in phases 0 and 3. Held X must give a mean `+0x1088` near 1.59, and phase 13 must advance to 15+.
- No commits, and never kill processes this session didn't start.
- **Core rebuilds:**
  - Take the lock with `mkdir .core-build.lock`. If that fails, wait until the directory is gone.
  - Build on top of skip-splash's `hle.txt` entry (`PcSkipStrap`) and `runtime/game/pc_skips.cpp`. Never revert them. Append the input entries to `hle.txt` instead of rewriting it.
- **Game instances.** Only one Rust/Bevy game instance may run at a time, across all sessions. This covers madworld-native, GPU tests, `run_owned_gpu_test.py` and mw-game. Pure-CPU unit tests are exempt.
  - Take the lock with `mkdir .bevy-instance.lock`.
  - Write `owner.txt` (session, PID, start time).
  - Check `tasklist` for madworld processes first.
  - Remove the lock on exit, even on failure.
  - Keep runs short.

  Phases 0, 3, 4 and 5 all need this lock.

## 11. Decisions for the user

1. Approve HLE at the WPAD/KPAD API boundary as above, rather than a lower-level fake WUD (which would still run guest WPAD/KPAD).
2. Snapshot compatibility: reject old snapshots after a version bump (recommended), or write a converter.
3. Keep `MW_LEGACY_BT` only until parity passes (recommended), or keep it permanently.
