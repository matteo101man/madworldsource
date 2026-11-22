# Same-input frame comparison: recompiled core vs native `mw_sim::fight::Fight` (stage 201, RinRin)

2026-11-09. This closes the "same-input frame comparison" item left open in NATIVE-PORT-GOAL.md and ORACLE-COMPARE.md.
Labels: [native] = DOL/RSO evidence or the original's own values read frame-atomically from the recompiled core
(this recording; marked [observed] below), [inferred] = reasoning, [unknown] = not established.
Unknown: why the original Rin moves in `0102` and why its idle does not drift (items 1/2; the causes given are
inferred), and Rin's cooldown state at the seed frame.

## Tooling (all default-off)
- **Recorder**, in the recompiled core (`tools/upstream-native-core/port/runtime/game/frame_record.cpp`), enabled with
  `MW_FRAME_RECORD=<jsonl>`.
  - **Row boundary:** one row per game frame, i.e. per window between two calls of the frame wait 8026F324.
  - **Arming:** recording starts once the intro demo nesting 0x80604020 is 0 and both actors exist.
  - **Hooks:**
    - `cPl0000_frameUpdate` 802727B0. It is only reached through vtables, so `ppc_lookup` wraps it.
    - Rin's onMove 810CAC70, through the existing `trace_rin_move` wrapper.
  - **Fields per actor, read after its update:** state bytes +0x11A..+0x11D, motion resource (+0x1AC+0x54), motion
    frame and step, HP +0xEC, position +0x64, facing +0x74.
  - **Player pad view:** +0xEE0 holds, +0xEE4 triggers, +0xEF0/+0xEF4 accel, +0xF1C/+0xF20 stick, +0x113C stick yaw,
    +0xF64, +0x1088/+0x108C/+0x1090/+0x109C.
  - **Raw input:** every KPADReadEx sample of the frame.
  - **cRand:** the shared word 0x80609B90, at arming and at the end of every frame.
  - **Motion names:** each resource's 64-byte header is written once and resolved offline against
    `native-motion-live-index.json`.
  - **Read-only:** the recorder writes no guest memory.
- **Original-side injection:** `MW_INPUT_REPLAY=<recording>`. KPADReadEx (`input_hle.cpp`) answers with the recorded
  samples of the same frame and read order instead of the PC pad. When a replay starts from a snapshot taken after the
  recording armed, the base frame is found by matching the current cRand word against the recorded per-frame words
  (`replayBase` in the header).
- **Native-side injection:** `cargo run -p mw-sim --example frame_replay -- <rec> <out> [--start N ...]`.
  - Each recorded frame's pad view becomes one `FightInput`:
    - A/B/jump: bits 0x10/0x20/0x1000 of +0xEE4 (trigger) / +0xEE0 (hold);
    - swing: |accel| > 2;
    - shake / shake_remote / shake_nunchuk: +0x1088 / +0x1090 / +0x109C;
    - nunchuk_shake: +0xF64;
    - camera yaw: +0x113C minus the stick angle.
  - `--start N` seeds both actors from row N: state/phase (+0x11B/+0x11C), clip and frame, position, facing and HP.
    The seed is that row's cRand word.
  - Not seeded: Rin's cooldowns and scaffold timers, the player's input buffers. They keep their `Fight::new`
    values. Movement uses `step_free`, so there is no wall collision.
- **Comparator:** `python -I tools/pl0000/frame_compare.py <rec> [--start N | --against <rec2>]`.
  - Reports the first frame where state (+0x11B), motion (clip set), HP or position diverges.
  - Tolerances: position 0.05 (xz and y), motion frame 1.0.
  - Output: `research/FRAME-COMPARE-<label>.json`.
- **Runs:** `run_owned_gpu_test.py`, with the recorder DLL built into `port/build-record` and loaded via
  `MW_CORE_DLL`. The shared `port/build/madworld_core.dll` was not rebuilt. One Bevy instance at a time, under
  `.bevy-instance.lock`.

## Recordings (evidence/frame-record/, owned-run data, not for git)
| file | what |
|---|---|
| `live.jsonl` | Natural fight, PC steering in `--sustain` mode. 2867 frames, 1 KPAD read per frame, Rin's update in every row. The snapshot `control-ready-record.bin` was taken 488 frames after arming. |
| `replay.jsonl` | Restore of that snapshot, with `MW_INPUT_REPLAY=live.jsonl`. `replayBase` 488, 4468 rows (live pad after frame 2867). |
| `live-v1.jsonl` | An earlier natural fight with movement (stick in 1769 frames), recorded with the first recorder build. No replay: the rebuild invalidated its snapshot. |
| `*.native.jsonl` | `frame_replay` output for the two live recordings. |

The fight in `live.jsonl` [observed]:
- **Intro:** Rin starts on a scaffold (states 13 / 24 / 7 / 25 / 8, kamikaze down at frame 274) and lands in wait at
  frame 334.
- **Fan loop:** from then on she loops fan throws: state 14 with `0102` → `0103` → `0104`, one frame of state 0
  between throws.
- **Player:** each fan hit costs 10 HP and alternates the knockdown (92, `1011`/`1016`) with the light hit (85, `0600`)
  plus dizzy (91, `1097`).
- **End:** the player dies at frame 1354 (state 95). Rin enters pl_dead (31) at 1431.

## Result 1: the injection reproduces the original exactly
`frame_compare.py live.jsonl --against replay.jsonl`: **2379 of 2379 overlapping frames (488..2866) are bit-identical.**
That covers all state bytes, motion pointer and frame, HP, positions, facings, the player's pad view, and the cRand
word at the end of every frame.

The recompiled core is therefore deterministic from a snapshot under the injected pad stream. The recording is a
usable reference trajectory.

## Result 2: native sim vs original, same input, same seed
Segment `live.jsonl` frames 334..2866, seeded at 334 (Rin wait `002d` frame 60, player idle `0000` frame 19, cRand
ff447bb7): 2533 frames. `live-v1.jsonl` from frame 365 (2477 frames) shows the same pattern. Its frame numbers are in
brackets.

**First divergence: frame 337 (v1: 368), Rin's position.** Earliest frame per category:

| category | live (seed 334) | live-v1 (seed 365) |
|---|---|---|
| rin.position | 337 | 368 |
| player.position | 341 | 421 |
| player.state / motion / hp | 387 | 417 |
| rin.motionFrame | 389 | 419 |
| player.motionFrame | 390 | 861 |
| rin.motion | 407 | 436 |
| rin.state | 429 | 458 |

What matches:
- Frame 335 [366]: Rin's AI makes the same choice on the same frame, wait → throw (state 0 → 14, `em0440_0102`
  frame 1). The native sim chains run → throw within one step.
- The motion frame clock (+1 per frame) matches until the first hit.
- The throw cycle structure matches: `0102` → `0103` → `0104` → one frame of state 0 → `0102`.

Divergences, in order of appearance [observed unless marked]:
1. **Rin moves during the throw windup (frame 337).** In `em0440_0102` the original Rin turns and slides about 0.5
   units (2.15,-73.21 → 2.64,-73.06 over 33 frames, about 0.13/frame at the start). The native Rin stays at her
   spawn point. The cause is either root motion of `0102` not applied for state 14, or a turn/approach step
   in throw phase 0 [inferred].
2. **The native idle player drifts (frame 341).** The native player creeps about 0.06 units on `pl0030_0000`; the
   original holds exactly (2.50, -61.00) in idle. The original idle discards that clip's root xz [inferred].
3. **The fan hit lands one frame early (387 vs 388 [417 vs 419]), with twice the damage.**
   - Native: `PlayerHit atk 0x161 damage 20`.
   - Original: −10 for every fan hit, in both recordings.
   - So the native player dies after 5 hits (frame 846) instead of 10 (frame 1354).
   - The reaction also differs. The native always uses knockdown 92. The original alternates 92 (`1011`/`1013`/
     `1015`/`1016`) with light hit 85 (`pl0030_0600`) → dizzy 91 (`1097`). The 85/0600 path is missing from the native
     table for this attack.
4. **Hitstop freezes the whole native fight.** After the hit, the native holds both actors' motion frames for several
   frames: Rin's `0103` sticks at 19, and the player's reaction stays at frame 0. In the original, Rin keeps animating
   (20, 21, 22, …) and the player's reaction advances 1 per frame from the first frame. This is the source of the
   later `rin.motion` divergence: the native `0103` → `0104` comes at 413 instead of 407.
5. **Knockback distance.** Original reaction `1011` moves the player 1.41 units (z -61.00 → -59.59) and then stops.
   The native moves him 3.25 units (to -57.75).
6. **Input during reactions.** The steering held the stick forward while the original player was still in 85/91. The
   original ignored it; the native player had already got up and ran (state 4, `00ce`) for 1-2 frames before the next
   hit. This follows from 3/4 (different reaction lengths), not a separate rule.
7. **cRand is shared.** The original advances the 0x80609B90 LCG by a median of 20 draws per frame (p10 16, p90 34,
   max 676) [observed: LCG steps between consecutive recorded words]. Other systems draw from it too. The native
   `Fight::rng` only draws for its own decisions, so the sequences agree only on the seed frame. Matching random
   choices (taunt / guard / flinch picks) frame-exactly needs either the other draws modelled or the recorded word
   injected per frame. The recording holds every frame's word, so the replayer could reseed `Fight::rng` from it each
   frame [inferred next step].

Overall, over the whole segment (2533 frames), frames diverging per category: player.state 874, player.hp 964, rin.state
593, rin.motion 895. Positions diverge almost throughout, because of items 1/2/5 accumulating.

## What this enables / next steps
- Each item above is now a reproducible, frame-numbered test case. Re-run
  `frame_compare.py evidence/frame-record/live.jsonl --start 334` after a sim change, and the first-divergence frame
  should move later.
- Suggested order:
  1. Fan damage and the 85/0600 reaction (item 3).
  2. Hitstop scope (item 4).
  3. Rin's windup movement (item 1).
  4. Idle root motion (item 2).
  5. Knockback (item 5).
  6. Per-frame cRand reseed (item 7).
- Not covered by this segment: the clash, the finisher, player attacks landing on Rin (Rin HP stays 1600 in both
  recordings), scaffolds after the intro. A `--sustain` run that reaches a clash, recorded the same way, would cover
  them.
- Not seeded at `--start`: Rin's attack/slalom cooldowns and scaffold timers. That the first AI choice still matched
  here is luck of the wait → throw rule. Seeding from the start of the recording (row 0) instead puts Rin mid-scaffold
  sequence, which the sim cannot represent.

## Native fixes after the first comparison (session local_4b1b4493, 2026-11-09)
`frame_compare.py evidence/frame-record/live.jsonl --start 334`, first divergence per category, before -> after:

| category | before | after |
|---|---|---|
| rin.position | 337 | 411 |
| player.position | 341 | 390 |
| player.state / motion / hp | 387 | 487 |
| rin.motionFrame | 389 | 490 |
| rin.motion | 407 | 511 |
| rin.state | 429 | 533 |

`live-v1.jsonl --start 365`: player.state 417 -> 418 (one frame early, see below), rin.position 368 -> 440, rin.state 458 -> 561.

Changes (mw-sim `fight.rs`):
- Item 3: Rin's attack power is cEmBoss::getAtkPower(i) (81006300) = em_common.ini [em0440] "boss attack i"
  [native]: uraken 10, tetuzankou 10, kamikaze 10, fans 10, get-up / explosion / scaffold bomb 5, giant slalom 15
  (`RIN_ATK_POWER`; each attack's index from its 81005DA4 call). The fan hit is now 10 like the original.
- Item 4: no fight-wide hitstop when Rin hits the player [observed].
- Item 1: the throw applies 0102's root motion, turns from the wind-up's first frame, Rin's root step lands one frame
  late along her current facing and is weighted by the blend progress while blending in [observed]. The fan hit is
  resolved one frame after the volume is reserved (81005DA4 reserveAtkHit) [observed].
- Item 2: no root xz in idle [observed; the native code path is not located].
- One-shot motions report their end one frame after reaching the last frame (80079210 is the update itself)
  [observed on 1011 / 1090 / 0102 / 0104]; the throw phases update the motion before their end checks; the get-up's
  motion starts on the frame after changeState(0x5E); the catch 0104 starts at frame 0 (started from the fan update);
  the player's root step also lands one frame late [observed].
- Still open, with frame numbers:
  - player.position 390: knockdown 1011 moves the original only until its frame 9 (1.415 total), then holds; the port
    follows the whole root curve (3.25). Not a wall (he moves further later). 802C0720's +0x119C / +0x1240 push code is
    the lead [unknown].
  - player.state 487: the second fan hit lands one frame early in the port, and the original reacts with 85 (`100c`/
    `100f`) -> dizzy 91, so it is the returning fan (0x18C) from behind while the port's outbound fan hits. It follows
    from the knockback difference (different player position) [inferred].
  - live-v1 frame 418: the first hit is still one frame early with stick movement [unknown].
  - cRand per-frame reseeding (item 7) is not done.
- Knockdown displacement per occurrence [observed, both recordings]: 1011 moved 1.415 (stops at motion frame 9),
  0.185 and 1.146; 1016 four times 0.129; 1013 2.701; 1015 3.501 / 4.83 / 1.851. The same clip moves very different
  distances, so it is not the clip root curve the port applies; a hit-dependent push or collision decides it. The
  candidates are 802C0720's +0x12BC timer (0.5 x motion length x 805CBE24, -GameVSyncF per frame) and the
  stage collision the replay omits (`step_free`) [unknown].
- Collision replay: `MW_REPLAY_COLLISION=1 frame_compare.py ...` runs `frame_replay --collision`, which moves both
  actors with `Fight::step_world` (the presenter's wall slide and floor snap, moved to `mw_sim::collision`, keeping
  each actor's height offset above the floor). With it: rin.position 411, player.position 390, player.state 486.
- Knockdown stop [observed]: across both recordings the player's z never exceeds about -58.89 behind the spawn,
  and no static wall lies there (rays from (2.47, -61) toward +z hit nothing within 5 units in col.hkx / col_pl.hkx).
  So it is an area limit, not collision; the first 1011 stopping at -59.585 fits a limit applied to a body point
  ahead of the root (lying pose) rather than the root. Lead: the area check 80085894 on 0x80602D68 (also used by the
  clash / grab / finisher gates) [unknown].
- 80085894 [native]: returns 1 when the point is inside any of the 64 area volumes at 0x80602D68 + 0x260 (0x40 bytes
  each) whose word has bit 0x80 and type (& 0xF) 8, tested by 80085300; the areas are loaded from the room data by
  8008587C (802140E4). Its callers (8028BCB0 clash, 802C6020 grab, 802C44A8, 80281DD8, 8027621C clash prompt) all
  use it as a "not inside a no-action area" gate, not as a movement clamp. bl015d (the boss barrier the entry script
  spawns) has empty col*.hkx members, and the staged static collision has no surface behind the spawn. So the
  -58.89 limit is still unexplained: the room's area volumes of another type, or the character proxy's margins,
  are the remaining candidates [unknown].
- cRand reseed: `MW_REPLAY_RESEED=1` (`frame_replay --reseed`) starts every native frame from the original's word at the
  end of the previous frame. First divergences are unchanged (random draws are not what diverges first); over the
  whole `live` segment with collision + reseed, frames diverging: rin.state 71 / 2533, rin.motion 218, player.state 650.
- Ruled out for the -58.89 knockdown stop [observed]: c201.cdz (the 64 area volumes 8008587C loads: types 1 / 2 / 4 / 8,
  all in the main stage at y 0-15), AreaTbl.art (16 GameArea quads; GameArea09, x -9.1..15.79, z -69.33..-39.41,
  holds the boss arena but not a z = -58.9 edge), bl015d (empty collision), static col*.hkx. Remaining: the
  character controller's own collision (margins / proxy resolution, not decoded) [unknown].
- Root blending [observed, now ported]: while a motion blends in, the original weights the accumulated root offset
  from the motion start by k / n (`FightData::blend_root`), and reactions start at blend 0.5 x +0x1148 = 2.5
  (80274DC4 dVar13). With that, the 1011 knockback matches frame for frame through motion frame 9 (z -59.585 at
  recorded frame 397).
- Boss barrier [inferred]: ObjTbl.obt places bl015d_N001_sc at (2.593, 26.231, -58.344); the player's z never passes
  -58.89 in either recording. `mw_sim::collision::move_on_floor` (shared by mw-play and the collision replay) now
  holds bodies 0.55 short of z = -58.344 inside GameArea09. The barrier's own blocking code is not located.
- Still unexplained: the first 1011 stops at -59.585 (frame 9, body airborne up to 2.9, no part behind the root),
  1.24 short of the barrier, while the others reach -58.89. First divergence with collision + reseed: player
  position 398 (was 341 at the start of this work), player state / HP 485, Rin position 411, Rin state 530.
- Higher wall probe [observed]: from (2.47, -61) toward +z, no collision surface within 6 units at 1.5-4 m above the
  floor (the nearest, `bossk08_ST01`, starts 5 m up). The airborne 1011 stop at -59.585 is not static geometry.
- Boss arena area [native data + inferred margin]: the boundary is the room's cEmArea box in c20100.caa (corners
  (-8.207, -88.597) (13.817, -88.412) (13.316, -57.597) (-8.553, -57.583), y 17.549..24.749). cPl0000_frameUpdate
  pushes the player out of attribute-6 obstacle areas toward (0, 1.1, -76) at 0.015 / 0.02 / 0.025 per frame
  (801D8D94 -> cEmArea::ckHitAttr 801D7FD0, 80274420 push normals) and plays announcer 0x35 / 0x12F. The port holds
  bodies 1.3 inside the box (`mw_sim::collision::BOSS_ARENA`, margin fitted), replacing the bl015d plane. Over the
  `live` segment (collision + reseed) frames diverging: player.hp 47 / 2533 (was 894), player.state 452 (was 650),
  rin.state 91. First divergences unchanged (player position 398, state / HP 485): the first 1011 still stops 0.7
  earlier than the margin, so the push rule (which areas are attribute 6, and the push itself) is the next item.
- Character controller [native strings]: the DOL links hkpCharacterProxy / hkpCharacterMotion, so the player moves
  through a Havok character proxy. The staged collision has no parse failures; above the knockdown path only thin
  beams exist at about 4.8 m (bossk08_ST01 at z -60.25, cube3291_ST01 at z -58.25). Neither static geometry nor the
  area push (0.015-0.025 per frame, weaker than the 1011 root step) explains the dead stop at -59.585; the proxy's
  own shape and contact resolution (not decoded) remain the candidate [unknown].
- The boundary is the gate bl0510_N001_g01 [native data]: ObjTbl.obt places it at (2.5, 17.55, -58.24), yaw 0;
  bl0510's col.hkx (read from the packfile directly: its hkxScene part does not parse with the Havok adapter) is one
  keyframed rigid body, hkpConvexTranslateShape (0, 3.5, 0) over hkpBoxShape half extents (2.4, 3.5, 0.05), convex
  radius 0.05: a wall over x 0.1..4.9 with its arena face at z = -58.34. The staged static world never included
  placed objects. The player's proxy is cHavokCharacterCollision (hkpCharacterProxy) radius 0.4 / height 1.6
  (802705DC -> 800B2500, 80569F04 / 80569F68; radius reset to 0.4 on hits by 80275D28; height set each frame by
  8027621C before update 800B2D0C) [native]. Bodies stop 0.55 in front of the face in both recordings (0.4 + a fitted
  0.15 keep distance) [inferred]. `mw_sim::collision` now models this gate (replacing the fitted arena box); over the
  `live` segment: player.hp diverges on 45 / 2533 frames. Still unexplained: the airborne first 1011 stopping at
  -59.585 (1.25 from the face); 802C7338 (checkShapeHit) is the proxy's floor check, not a wall test.

- **Frame-398 stop resolved (2026-11-09):** the proxy timestep falls below the simplex solver's 0.0001-second
  advancement cutoff during opening-event hit slowdown; it is not an airborne collision margin. The Rust fight
  now keeps the proxy clock separate from animation and the replay conveys demo nesting. First player-position
  divergence moves 398 -> 493; first overall divergence is Rin position 411. Later HP/state discrepancies remain,
  and aggregate HP matching worsens (45 -> 132 differing frames), so this is not full replay equivalence.
  Native addresses, contact/clock observations, fixture scope and verification: [HAVOK-PROXY-CLOCK.md](HAVOK-PROXY-CLOCK.md).


2026-11-09 fight follow-up: repaired HP regression (132 -> 1 canonical mismatch frame; independent contact capture 8). Native root filtering, fan matrix timing, get-up protection, animation/death clocks and convex gate-side contacts translated. 39 simulation tests pass. Contact/catch/idle and independent-capture discrepancies remain open. Details: research/FIGHT-REPLAY-FOLLOWUP.md (repository relative). Continued full UI, pause, intro and challenge menu port tracked in research/NATIVE-PORT-GAPS.md. Kreely playtesting excluded.
