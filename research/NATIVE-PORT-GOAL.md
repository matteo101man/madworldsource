# Goal: native Rust MadWorld (c201 RinRin fight), with Jack and Kreese Kreely playable

Set 2026-11-08 at the user's request. The target is the **pure native slice** (`crates/mw-data`, `mw-sim`,
`mw-game` main binary). It is not the recompiled-core frontend (`madworld-native`), which stays the **oracle**:
the reference the native code is compared against. The other sessions' runtime work (renderer, PC controls, 60 fps,
death camera) happens on `madworld-native` and is not touched by this goal.

**Done means:** in the native Rust build, a player can start the c201 arena, play **Jack or Kreely (Insert swaps
live)**, fight **RinRin driven by her translated native state machine**, use the real **combat rules** (attacks,
hit windows, damage, reactions, clash/power struggle, grabs where in scope), and reach **victory or death** at the
**native game clock**. Every rule is traced to native evidence (DOL/RSO pcode, tuning CSV, motion cues, or oracle
observations) and labelled [native] / [inferred] / [unknown]. Each phase has an **oracle diff**: the same scripted
input is run on the recompiled core and on the native build, and states, motions, positions and HP are compared.

## Method (every phase)
1. **Evidence:** existing research graphs and indexes (RIN-STATES, rinrin-behavior, TUNING, HIT-STATE,
   MOTION-CUES, MOTION-CLOCK, MOTION-STATE, PLAYER-COMPETITION, PLAYER-DEATH, VICTORY-MOTION-INPUT, COLLISION, AUDIO).
2. **Oracle capture:** scripted PC input on the owned headless core (`native_input.py`, `read_native_core.py`,
   `watch_owned_pc_motion.py`). This records motion ids, state indices, positions and HP per frame.
3. **Native translation** in Rust (`mw-sim` = rules/state machines, no rendering; `mw-game` = Bevy presentation).
4. **Diff test:** the same input script runs on both; tolerances are stated per quantity.

## Phases and pass criteria
| # | Phase | Pass criteria |
|---|---|---|
| 1 | **Native player foundation**: player entity in c201, PC input (same bindings as PC-CONTROLS), camera follow, collision grounding, motion playback + blending, **Jack/Kreely Insert swap**, Kreely's pistol set + transitions | Walk/run/turn/idle selected from **oracle-observed** clips; feet on collision floor; Insert swaps live with the pose kept; Kreely uses the rotation-only retarget (exact Jack bone rotations) |
| 2 | **Player state machine**: native `cPl0000` states for locomotion, jump, dodge, attack strings, lock-on (MOTION-STATE: `SetLockOn_Trg`, camera angle) | Per-input-script state/motion sequence matches the oracle (same motion ids in order, frame timing within 1 frame at the native clock) |
| 3 | **Combat rules**: hit windows from motion cues (op/frames), `reserveAtkHit` / `onHitChk` / damage reaction types, HP, invulnerability, chainsaw (Jack) / pistol (Kreely, secondary) | Oracle: same hits land on the same frames, same HP deltas (RinRin HP 1600 to 1300 clash reference already observed) |
| 4 | **RinRin AI**: 32-state table (wait ... pl_dead), branch/update/end callbacks, tuning rows | State transitions under the same scripted player input match the oracle for 60 s windows |
| 5 | **Clash / power struggle, finish, victory, player death** (PLAYER-COMPETITION, CLASH, VICTORY-MOTION-INPUT, PLAYER-DEATH) | Held-X clash completes with the same HP result; victory and death sequences reach the same end states |
| 6 | **Timing**: native clock (MOTION-CLOCK API), 30/60 handling, audio cues (AUDIO-API) on the motion events | Frame-count drift vs the oracle is 0 over 60 s scripted runs; audio cue ids fire on the same frames |
| 7 | **Kreely as a full fighter**: own secondary weapon (pistol) wired into the combat rules; his own clips replace Jack fallbacks as they are authored | Pistol shots register hits through the same hit pipeline; all Kreely states have a clip (own or retargeted) |

## Status
- Kreely's assets, rig, retarget, fingers, pistol and blended transitions exist (explorer stage, `research/KREELY.md`).
- **Phase 1, first slice running (2026-11-08):** `cargo build -p mw-game --bin mw-play` (target dir
  `D:/GDecomp/build-cache/bevy-0.20`).
  - **Scene:** pure Rust, no recompiled core. It loads the c201 arena, the player at PlayerTbl point 5 (2.5, 17.55,
    −61, yaw 180), and RinRin playing her wait clip at her spawn (no AI yet).
  - **Clock and movement:** a 30 Hz native tick, one motion frame per game frame (MOTION-CLOCK). Movement is each
    clip's own root motion; floor height comes from native collision rays (17.55 at the start point, matching
    PlayerTbl).
  - **Clips (from replay observations):** idle `0000`, move `00ce`, punch `1105`, dodge `1123`.
  - **Characters:** **Insert swaps Jack ↔ Kreely live**, and Kreely uses the rotation-only retarget plus fist hand
    pose (`mw-sim/character.rs`, unit-tested). C toggles the color palette.
  - **Evidence:** `evidence/native-play/{jack,kreely}-start.png`.
  - **Phase 1 additions (same day):**
    - Walls: horizontal rays against `col.hkx` + `col_pl.hkx` (player boxes; "player-only" [inferred] from names),
      slide along the hit normal, plus a ledge guard (no step without floor) [inferred].
    - Secondary weapon: Jack RMB -> chainsaw `pl0030_1104` [observed]; Kreely hold RMB -> raise (blend 20) -> aim
      loop `ev93_cut8_aim`, LMB -> fire `ev93_cut8_fire` (blend 10), release -> lower (blend 22) -> pistol idle
      `ev93_cut18_idle` -> idle. Overlays sampled in Rust (`mw_sim::character::sample_overlay`), golden-tested against
      the explorer (`assets/native/kreely-golden-poses.json`); source MOTs staged hash-checked in
      `assets/native/kreely-gun-motion.json`.
    - Scripted runs: `MW_PLAY_SCRIPT=<json> MW_PLAY_LOG=<jsonl> MW_PLAY_TICKS=<n> MW_PLAY_SHOTS="tick:png;..."`.
      Evidence: `evidence/native-play/script-phase1.{json,log.jsonl}` + `s*.png` (600 ticks: walk 10.7 units on the
      00ce root motion, punch, dodge, chainsaw, Insert swap, aim/fire/lower, color toggle, walk to the arena edge
      where the guard holds him at z = -88.0, floor 17.55 throughout).
  - **Still open in phase 1:** lock-on camera; the chainsaw prop is not shown on Jack's native mesh. Input-to-state
    choice, turn rate, crossfade lengths and camera stay [inferred] until phase 2.
- **Phase 2, part 1 (2026-11-08): locomotion translated** from the DOL (`research/PL0000-LOCOMOTION.md`,
  `crates/mw-sim/src/player.rs`): states 0/1/2/4 with native stick thresholds, walk `00c2` / run `00ce`, turn relay,
  RadLerpRange turning, playback rates, 5-frame blends and the double-tap dash with the `00cf` sprint layer. mw-play
  runs on it (Ctrl = walk tilt). Next: the combat entry checks (`802C3B5C` -> `802C3C0C`) and state 39, then the
  oracle script diff.
- **Phase 2 survey:** `research/PL0000-STATE-SURVEY.md` (dispatch, observed state->motion map, fields, gaps) and
  `research/pl0000-state-table.txt` (96 A/B handler pairs).
- Kreely asset staging: `tools/kreely/export_kreely_glb.py` writes `assets/native/kreely-{source,color}.glb` and
  `kreely-rig.json` (manifest `research/kreely-staging-manifest.json`).
- **Phases 3-5 core (2026-11-08):** the c201 fight runs natively end to end in `mw-play` (`crates/mw-sim/src/fight.rs`,
  research/PL0000-COMBAT.md): Jack's A string and chainsaw swings with cue-timed hits and plem_common.ini damage,
  RinRin's AI from em0440.csv (wait/run/taunt/uraken/tetuzankou/slalom, guard, evade steps, reactions, finish_wait),
  player light/knockdown reactions, death and continue, the paired finisher with held-X shake, Victory. Kreely plays
  the same fight; his pistol hits through the native HANDGUN row. Open: clash, Rin's ranged/scaffold attacks, player
  jump/dodge/grab/rush, lock-on, effects/audio, oracle diffs.
- **Later the same day:** clash (1600 -> 1300 like the oracle), fan throw (Hermite paths), scaffolds (9 native points,
  climb / hop / bombs / kamikaze), player jump, rush punches and FINISH_BLOW, body separation, lock-on (Tab, inferred).
  Kreely wins the full sequence with pistol + punches + finisher. 24 mw-sim tests. Remaining for "full": air attacks,
  grabs / giant swing, the B-chainsaw cut path (clash entry), Rin's electric shock / down damage / scaffold-bomb
  reactions, effects and audio cues, native camera, oracle diff runs.
- **Status at end of day (2026-11-08):** 26 mw-sim tests. Also added: jump attack, Rin's explosion down / down damage /
  get-up spin, giant swing (simplified). Full-sequence scripted runs end in Victory for Jack (tick 1950) and Kreely
  (tick 2139) on the final build. Open items are listed at the end of research/PL0000-COMBAT.md.
- **Continued:** electric fence (ST10 surfaces), native cRand, native follow camera, evade, wall-ray fix. 28 tests.
  Scripted full sequences: Jack wins via clash -> finisher (tick 1967), Kreely via pistol/punches/FINISH_BLOW ->
  finisher (tick 2157). Open: sound-event mapping (encoded sound scripts in se/*.rsp), B-chainsaw cut path, oracle diffs.
- **Sound + oracle diff (continued):** sound effects/voices/footsteps from the decoded RedSpark packs play on the
  native cue events; native giant swing; reaction motions and flag-5 damage fixed from the oracle diff
  (`research/ORACLE-COMPARE.md`: the native runs reproduce 31/66 player and 44/60 RinRin state-motion pairs seen in
  46 oracle runs; RinRin's HP steps now include the oracle's x1.1 values). 29 mw-sim tests.
- **Open items closed (2026-11-09):** the real clash trigger (8028BCB0: chainsaw swing inside RinRin's rig-flag-0
  window, frames 10-60 of the giant slalom, no distance test); boss BGM (`bs04c`, intro + loop); positioned sound
  effects; announcer voice lines (PlayVoiceStream: idle fidget, giant swing, death) from the stage-201 pack; the
  native hit-reaction table (kamikaze knockdown, 0x18C directional / random flinches), stun -> dizzy state 91, grab
  whiff state 21, idle fidgets, and the per-frame input-buffer countdown. 33 mw-sim tests. Victory evidence now
  comes from `fight::Autopilot` (`cargo test -p mw-sim autopilot`; `MW_PLAY_BOT=1`: Jack tick 4597 with 11 clashes,
  Kreely tick 14253, no deaths). Oracle diff: 40/66 player and 46/60 RinRin state-motion pairs reproduced.
  Still open: a same-input frame comparison (the oracle files are time-sampled snapshots without per-frame input;
  it needs a frame-atomic recorder in the recompiled core), the result jingle, track pan 0x12 / native 3D curve,
  ~40 further announcer call sites, the BGM c-variant requester, 0424/0425 (wp0600 chainsaw idle), clash loss
  04AD in a natural run.
- **Continued (2026-11-09):** native clash meter and loss (04AD), chainsaw slot (0425 / 0427 / 0424 stow on B
  hold / release), announcer 0x1018 (low HP), the BGM c-variant source (ev21's timeline), the stale rig-flag fix,
  key-driven evidence (`MW_PLAY_BOT=keys`, recorded to `script-keys-*.json` and replayed through the keyboard path),
  `MW_PLAYER_HP` debug start. 35 mw-sim tests. The same-input frame comparison runs in its own session
  ("Frame-exact oracle recorder for the Rin fight").
- **Havok follow-up (2026-11-09):** resolved the opening knockdown stop at recorded frame 398 through the native
  scene/proxy timestep cutoff, with separate Rust physics timing and demo-context replay. Player-position mismatch
  now begins at 493; Rin position still differs at 411 and player state/HP at 485. 36 mw-sim tests pass. Later HP
  matching regresses in the complete replay; full fidelity remains open. See `research/HAVOK-PROXY-CLOCK.md`.
  Kreely playtesting was explicitly excluded; no missing announcer systems were added.


2026-11-09 fight follow-up: repaired HP regression (132 -> 1 canonical mismatch frame; independent contact capture 8). Native root filtering, fan matrix timing, get-up protection, animation/death clocks and convex gate-side contacts translated. 39 simulation tests pass. Contact/catch/idle and independent-capture discrepancies remain open. Details: research/FIGHT-REPLAY-FOLLOWUP.md (repository relative). Continued full UI, pause, intro and challenge menu port tracked in research/NATIVE-PORT-GAPS.md. Kreely playtesting excluded.

2026-11-09 UI continuation: Rust IDT decoder now covers132 screens/4397 widgets; position interpolation translated, explicit distance-clock gap. Original owned pause reference captured;313 live widgets,269 source hash matches, full159-node subscreen0 bound. Diagnostic mw-ui-preview exists but remains visually incomplete and is not integrated into mw-play.45 combined tests and both bin checks pass;100s original-core UI capture/GPU/host-blit succeeds. Details UI-NATIVE-RUNTIME.md; full UI/pause/intro/challenge goal stays active.
