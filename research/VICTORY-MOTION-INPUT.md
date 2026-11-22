# Victory finisher motion input (Jack state 68 / selector 3)

Handoff step 5 of `HANDOFF-2026-11-08-SNAPSHOT-SAVE-LOAD.md`. This is read-only analysis. No build, no live test.
Typed graph: `victory-motion-input.graph.jsonl`. Regenerate it with `python research/victory-motion-input-graph.py`.
Each node carries `evidence` = native / runtime / port / inferred / unknown / proposal.

Sources: the recompiled original code `tools/upstream-native-core/port/generated/recomp_056.cpp:12086` (`fn_802A688C`) and `recomp_055.cpp:5024` (`cPl0000_frameUpdate`), plus constants, strings and jump tables read from `main.dol`.

## Answer in one paragraph

Phase 13 never reads buttons. Its only input is `Jack+0x1088`, the 8-sample mean of `|‖remote accel‖ − 1 g|`. A remote at rest gives exactly 0, so Enter alone can never move it.

The port already has a key for this: **held X** sends the remote `Gesture::Shake` profile 9. In free control it measured `+0x1088` with a mean of 1.59. The stall comes from the test harness: its `finish-confirm` phase calls `keys.clear()` and only pulses Enter.

The fix is a test-driver change, plus optional help text. **No `native-platform` change is needed.** It needs no speed multiplier and no game-state writes.

## Native predicate (evidence: native)

**Dispatch.** `B[68]` at `0x805CC77C` leads to `0x802A4A18`. Selector byte `+0x11D` = 3 leads to `fn_802A688C`. Inside it, phase byte `+0x11C` 10..17 indexes the jump table at `0x805CD29C`:

| phase | entry | behaviour |
|---|---|---|
| 10 | 802A68D4 | start `em0440.dat\pl0030_1305.mot` (+ `_00/_10.seq`, camera `pl0030_c307.mot`), then phase 11 |
| 11 | 802A6AC0 | advance motion; phase 12 at motion end |
| 12 | 802A6AE0 | start `pl0030_1306.mot`; `+0x123C=10`, `+0x1160=0`, `+0xF70=0`, `+0x1248=0`, `+0x27DC=0`; falls into 13 |
| **13** | **802A6BF4** | **motion integrator (below)** |
| 14 | 802A6D18 | directional check, or a timeout |
| 15 | 802A6E10 | start `pl0030_1308.mot`, camera `pl0030_c308.mot` |
| 16 | 802A6EDC | wait for `(+0x1260)->vtbl[0x24C]() != 0`, then camera `pl0030_c309.mot` |
| 17 | 802A6F60 | motion only |

**Phase 13, once per player update.** Constants come from the DOL: `8056A2E0`=0.005, `80569F44`=0.1, `80569F0C`=1.0, `80569F08`=0.0.

```
a      = min(+0x1248 + 0.005 * (+0x1088), 0.1);   +0x1248 = a
p      = clamp(+0x1160 + 0.1 * a, 0, 1);          +0x1160 = p
*(+0x14E8)+4 = p;   +0x11B8 = 1 + p      // meter / motion rate scalar (original)
setObjMotCameraBlendrate(0x80602D24, +0x27B8, ...); cByMotion::move(+0x1AC)
if p >= 1: +0x1238=0; +0x123C=20; +0x1230 = 480 >> (+0x11A8 & 63); phase=14; fn_802C6E20()  // ring reset
if !flag(0x80697E78, 7): setSwingNavigation(cockpit +0x11C8, remote=5, nunchuk=-1)        // prompt only
```

- `+0x1248` is never reduced inside phase 13. Once it is raised, progress keeps going after the shake stops.
- `fn_802A688C` touches no pad or button field (`+0xEE0..+0xEE8`). It reads only `+0x1088`, `+0x1090` and `+0x109C`.
- The game's own `+0x11B8 = 1 + progress` is its authored motion-rate coupling. It is not a host speed multiplier.

**Phase 14.**
1. Wait out `+0x123C` (20 updates).
2. While `+0x1238 == 0`, decrement `+0x1230`. Set `+0x1238` once `+0x1090 > 1.4` and `+0x109C > 1.2`, **or** once `+0x1230` reaches 0. With `+0x11A8 & 63 = 0`, the timeout is 480 updates.
3. Show prompt `setSwingNavigation(3,3)`.
4. After that, when the motion frame `+0x204` crosses 1.0, go to phase 15 and call `(+0x1260)->vtbl[0x90](…, 0x30)`.

Constants: `8056A08C`=1.4, `80569FA8`=1.2.

**Input chain.** It runs in `cPl0000_frameUpdate` (a virtual, vtable `0x805CDFD8`), before its first `+0x11A` branch at `0x80272940`, so it runs regardless of state:
1. `0x802727FC`: copy pad slot `0x80698B38 + (+0x10EC)*0x80` to `+0xEE0`. Remote accel lands at `+0xEF0` (slot `0x80698B48..50`), nunchuk accel at `+0xEFC`.
2. `0x8027282C`: `+0xF60 = +0xF68 = |sqrt(dot(+0xEF0)) − 1.0|`. Nunchuk goes to `+0xF64/+0xF6C` the same way.
3. Gate at `0x80272860..0x802728A4`: if `flag(0x80697E64, 10)`, or controller 0 with flag `0x20`, or controller 1 with flag `0x21`, the pad copy and `+0xF60/+0xF64` are zeroed.
4. `0x80272904`: `fn_802C6C0C` is called **exactly once per frameUpdate** (this settles the "cadence" question in CLASH-INPUT-REQUIREMENTS-INDEX). It keeps 8-entry rings and computes `+0x1088 = 0.125·Σ F60`, `+0x108C` for the nunchuk, and the vector means `+0x1090` (remote, rotated by `+0xF14`, abs) and `+0x109C` (nunchuk abs).

## Runtime evidence (evidence: runtime)

- `bevy-native-finish-normal` reached Jack 68/13/3 with `+0x1088 = 0` and `+0x1248 = 0` from 161.4 s to 206.6 s, while the harness was in `finish-confirm`. This matches a remote at rest: the idle slot is (0,−1,0), so the magnitude is 1 and `F60 = 0`.
- `evidence/native-pc-motion-probe-summary.json` (held X, free control) measured:
  - `+0x1088`: mean 1.586, min 0.696
  - `+0x1090`: max 1.4615
  - `+0x109C`: max 1.451
  - all four metrics exactly 0 after release.

## Port path (evidence: port)

- `crates/mw-game/src/native_controls.rs`: held X puts `Gesture::Shake` on both the remote and nunchuk tokens, with no buttons.
- `native-platform/src/motion.rs`: the Shake waveform is `[w, w, 1+w]`, with w = ±1.45 alternating every 8 ticks at 200 Hz. Unclipped `|‖a‖−1|` is 2.195 or 1.099, so neither half-period is ever near 0 and sampling phase cannot null it.
- `crates/mw-game/src/native_gpu.rs:333`: `finish-confirm` does `keys.clear()` and pulses Enter. **This is the cause of the stall.**

## Inferred (not yet observed)

- With held X in phase 13: `+0x1248` reaches 0.1 within about 13–29 updates, and `+0x1160` reaches 1.0 after about **105–115 player updates**. That is about 1.8 s at 60 Hz, or 3.7 s at 30 Hz.
- Phase 14 should pass on the gesture, because the probe maxima are above 1.4 and 1.2. Its timeout path also guarantees it advances.

## Unknown

- Whether stop flag `0x80697E64` bit 10, or `0x20`/`0x21`, is set during the finisher. If it is, `F60` is forced to 0 whatever the input. Check: `F60 == 0` while `nativeGestureSlots` is not (0,−1,0).
- The player update rate during the finisher.
- What Rin's `vtbl+0x90(0x30)` and `vtbl+0x24C` mean, and how phases 16/17 lead to Rin's death, `onBossDead` and `roomClear`.

## Implementation proposal (main session)

1. **`crates/mw-game/src/native_gpu.rs`** (test driver only):
   - In `Steering::apply`, also keep the player's `stateBytes11a11d` (`[1]` state, `[2]` phase, `[3]` selector), read-only from the existing feedback JSON.
   - In the `MW_GPU_TEST_PC_FINISH_CONFIRM` block: if the player is state 68, selector 3 and phase ≥ 12, push `KeyCode::KeyX` and no Enter. Name this phase `finish-shake`. Otherwise keep the Enter pulses.
   - Hold X until the phase is ≥ 15 or the state leaves 68. Because `+0x1248` does not decay, keep holding through phase 14 so it can pass on the gesture instead of the 480-update timeout.
   - Go through the shared `PcControls` mapper as today, so this tests the real binding.
2. **`tools/read_native_core.py`**:
   - Add `float1160` (progress), `float11b8`, and the `0x80697E64` flag word to `playerCompetition`, or to a new `playerFinish` block. These are read-only.
   - `F60/1088/1090/109C`, `float1248`, `1230/1238/123C` and `nativeSwingNavigation.remote.kindByte18` (expected to be 5 in phase 13 and 3 in phase 14) are already exported.
3. **`tools/run_owned_gpu_test.py`**:
   - The existing `MW_GPU_TEST_PC_FINISH_CONFIRM` is enough. Report the `finish-shake` phase.
   - The final `phase == released` assertion will still fail if the run ends inside finish-confirm or finish-shake. Allow those as final phases, or use a non-sustained run.
4. **`crates/mw-game/src/native_controls.rs`**: optional HELP text change only, from "Hold X: two-arm struggle" to "Hold X: shake (struggle / finisher)". Optionally show a PC prompt for cockpit remote kind 5 (see UI-PC-PROMPTS.md).
5. **No change** to `native-platform/src/motion.rs` or `input.rs`. Profile 9 already produces the needed signal. Do not add amplitude changes or speed multipliers.

**Acceptance trace:**
1. Phase 13 with `F60 > 0` and `1088 > 0`, then `1248` rising to 0.1 and `1160` rising to 1.0.
2. Phase 14, then 15, 16 and 17.
3. Rin's death, `onBossDead`, `subMission clear`/`roomClear`, and stop flags 9/10.

A GPU pass alone does not count.

## Result (2026-11-08, input HLE core `48864ab0`)

The proposal is confirmed. With KPAD fed directly (research/INPUT-HLE.md), the harness walks to Rin in finish_wait, starts the finisher with right mouse, and holds X in phases 12..14 (`finish-shake`). The finisher then completes naturally: `+0x1088` = 1.80, `+0x1248` capped at 0.1, phases 13 → 14 → 16 → 17, Rin 26/14, stop flags 9/10, `subMission clear` and `roomClear`, and the next scenes load (`evidence/input-hle-finisher-x.*`).

Two harness changes were needed in `crates/mw-game/src/native_gpu.rs`:
1. **Approach before confirming.** Finish-confirm used to clear the walk keys, so the player never reached Rin.
2. **Shake instead of Enter.** Finish-confirm used to pulse Enter during the shake phases; it now holds X there.

The prompt-gated A/D shake QTE is documented in PC-CONTROLS.md. The remote prompt kind observed in phases 13–14 is 0x11 and in phase 16 it is 0x0D.
