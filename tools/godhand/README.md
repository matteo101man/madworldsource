# Porting animations from another game's explorer onto MadWorld rigs

How Kreely's God Hand moveset was built, written so the same path works for clips from another explorer
(another game's character viewer). Everything here is reproducible from the scripts in this folder.

## The pipeline at a glance

```
other game's explorer ──export──▶ assets/native/godhand/*.json      (plain keyframes on the SOURCE skeleton)
                                   │
                         retarget_godhand.py
                                   ▼
                     assets/native/kreely-moves.json                  (MadWorld clips + SEQ-style cues)
                         │                 │
                 mw-play (game)    publish_explorer.py ──▶ MadWorld explorer (custom clips + paired studies)
```

| Step | Command | What it does |
|---|---|---|
| 1. Find the motions | God Hand explorer: `exports/godhand/Launch God Hand.cmd` → http://127.0.0.1:8786/ | Pick moves on the character you want them from |
| 2. Export | `python tools/godhand/export_godhand.py` | Samples each motion through the explorer's own solver, on the rig you picked it on |
| 3. Retarget | `python tools/godhand/retarget_godhand.py` | Bakes MadWorld clips, hit cues, sounds, receiver clips, suplex placement |
| 4. Inspect | `python tools/godhand/inspect_moves.py snaps / levels / compare <key> / stick <key> <frames> <out.png>` | Numeric and visual checks (see below) |
| 5. Explorer | `python tools/godhand/publish_explorer.py` → http://127.0.0.1:8781/ (reload) | Shows every move and the paired studies in the MadWorld explorer |
| 6. Game | `cargo build --release --bin mw-play`, then play as Kreely | The game loads `kreely-moves.json` at start (no rebuild needed for data-only changes) |

Steps 3–5 are all you rerun after tuning; the game picks the new data up on its next launch.

## 1. Picking motions: label traps

The God Hand explorer names motions it has not identified "Attack / action candidate NNN". These numbers are **not
stable**:

- **Renames shift the numbering.** When "candidate 166" was renamed "Sweep kick", every label above 166 moved down by
  one. The moves picked as 256 / 351 / 373 / 535 are now Gene's **255 / 350 / 372 / 534**. The first export matched
  the old numbers literally and grabbed henchman motions that still carry them: a crouch with arms up instead of a
  turning jab, a broken spin kick.
- **The same label exists on several characters.** Always note which character (and outfit) the motion was picked
  on. Record the explorer's motion id (`pl00:402`), not just the label.
- **Verify by what the motion does.** `godhand_server.py` + a scan like the one in the export notes (root travel, turn
  from the hips, foot height, number of SEQ hit windows) finds "the 38-frame three-hit standing combo" or "the
  360° spin with a high kick" reliably. Labels are a hint; motion shape is the proof.

The motion ids used now are listed in `export_godhand.py` (`singles`) and in `assets/native/godhand/index.json`.

## 2. Exporting

`export_godhand.py` loads the explorer's server in-process (`godhand_server.py` → `builders/serve_main.Server`) and
calls `raw_sample(rig, motion)`, the same path as the UI's `/api/clip?space=inspection-local`. That applies the decoded
curves **and the game's own leg IK** (two-bone chains 18-19-20 / 22-23-24 on a flat floor), so the export is the pose
the explorer shows. No HTTP server needs to be running.

- **Sample on the rig the motion was picked on** (`pl00-1` = Gene). A motion shared by several banks plays differently
  on another skeleton; the henchman rig made the hips float and the poses wrong.
- **Per clip, the exported JSON holds:**
  - the source skeleton: names, parents, rest translations; rest rotations are identity, a T-pose;
  - `frames[f][bone] = [tx,ty,tz, qx,qy,qz,qw, sx,sy,sz]`: local transforms relative to the parent; bones with parent
    -1 are relative to the actor root;
  - `root_motion[f]`: `actor_translation` / `actor_rotation` is the ground root; `hips_world_translation` is the hips;
  - the raw SEQ records and `hit_windows`, from flag bits 1/2 next to the motion (`godhand_seq.py` decodes them);
  - a guessed `striking_bone`, from the fastest end effector.
- **Paired grabs export both actors plus a `_pair.json`.** It holds each actor's container transform per tick and the
  receiver's root in the performer's frame.
- **Conventions (God Hand):**
  - Y up, characters face +Z, the character's right is −X;
  - 30 fps, one source frame per tick;
  - units treated as metres: Gene's hips are 1.14 m high.

  These are the same conventions as MadWorld, so no axis change is needed. **Check this first for any other game.**

## 3. Retargeting (`retarget_godhand.py`)

Target: the shared MadWorld body layout. pl0030 (Jack, and Kreely through his rotation-only rig), em0440 (RinRin) and
em0101 (the kung-fu grunts) all use bones **0–23** the same way:

| MadWorld | id | God Hand (Gene) | index |
|---|---|---|---|
| hips | 0 | hips | 0 |
| spine / chest / neck / head | 1 / 2 / 3 / 4 | spine / chest / neck / head | 1 / 2 / 3 / 4 |
| R clavicle / upper arm / forearm / hand | 5 / 7 / 8 / 9 | clavicle_r / upper_arm_r / forearm_r / hand_r | 5 / 7 / 8 / 10 |
| L clavicle / upper arm / forearm / hand | 10 / 12 / 13 / 14 | clavicle_l / upper_arm_l / forearm_l / hand_l | 11 / 13 / 14 / 16 |
| pelvis (leg pivot) | 15 | pelvis_lower | 17 |
| R thigh / shin / foot / toe | 16 / 17 / 18 / 19 | thigh_r / shin_r / foot_r / toe_r | 18 / 19 / 20 / 21 |
| L thigh / shin / foot / toe | 20 / 21 / 22 / 23 | thigh_l / shin_l / foot_l / toe_l | 22 / 23 / 24 / 25 |

Unmapped MadWorld bones get no tracks, so they follow their parent. Unmapped source bones (shoulder helpers, forearm
twist, locators, accessories) are dropped. Their effect is still inside the mapped bones' world rotations: the hand's
world rotation already includes the forearm twist.

### The math, per frame

1. **Forward kinematics on the source** gives each bone's world rotation `W_s`. The actor's yaw is applied at the
   hips.
2. **World rotations are copied across.** For each mapped target bone: `world_t = W_s · A`.
   - `A` is a rest-pose alignment that turns the target's rest bone direction onto the source's. It's used for limbs
     only; torso and head get identity.
   - It works because both rigs rest in a T-pose with identity rotations, so world rotations are directly comparable.
3. **Local rotations:** `local = parent_world⁻¹ · world_t`, walking the target hierarchy.
4. **Euler angles for MadWorld channels 3/4/5** (x, y, z), composed `Z·Y·X` (`mw_sim::character::quat_zyx`).
   - Every rotation has two Euler forms, `(x,y,z)` and `(x+π, π−y, z+π)`. Per frame, pick the form nearest the
     previous frame, then unwrap.
   - The game and the explorer interpolate each channel linearly between frames, so without this a gimbal swap flips
     the in-between frames. That was the upside-down frame in the power punch.
5. **Root motion:**
   - the actor's travel (from frame 0) goes on the MOT root `-1` channels 0 and 2;
   - the hips' height goes on bone 0 channel 1;
   - both are scaled by the target/source hip-height ratio (Jack 1.065 / Gene 1.14).
   - The receiver clips for RinRin and the grunts use their own hip height (1.24).

### Style corrections, added on top of the faithful retarget

God Hand's poses are exaggerated for its own camera. Three corrections, each a constant at the top of the script:

| Correction | Constant | Applies to | What it does |
|---|---|---|---|
| Upright limit | `LEAN_BACK` 20°, `LEAN_FORWARD` 35° | all moves except the suplex | Turns the upper body about the hips when the hips-to-neck line leans past the limits relative to the facing. The knee leaned back ~67° and the power-punch wind-up ~58°. |
| Square stance | `SQUARE` 0.75 | cross, cross_m, step jab, rush, whips | God Hand punches side-on (pelvis turned 45–100° off the facing). Turns hips, legs, torso, neck and shoulders 75% back toward the facing; the arms keep their source world directions, so the punch still goes straight ahead. |
| Level punches | `LEVEL_FREE` 0°, `LEVEL_KEEP` 0, per-move start extension | straight punches (start at 0.35 of extension), whips (0.65) | Gene's straight punches climb, ending at or above head height. As an arm straightens, the whole arm turns about the shoulder so the shoulder-to-fist line ends level. A bent guard below the shoulder is untouched; the whips start late so their overhead swing stays. |

Because of these, `inspect_moves.py compare` shows the corrected moves differing from the source on purpose. Compare
uncorrected moves (knee, sweep, spin kick) to check the plain retarget.

### Hit data and sounds (cues)

Each move becomes native-style SEQ cues in `kreely-moves.json` (`cues["<clip>_00.seq"]`):

| Cue | Meaning |
|---|---|
| op 1 | play rate (`f`) |
| op 0x13 / 0x14 | hit window open (`a` = striking MadWorld part, `b` = power) / close |
| op 0x0B | combo window (the next press chains) |

Sounds go in `_10.seq`:

| Cue | Meaning |
|---|---|
| op 29 (1101, n) | swing whoosh |
| op 38 (11, n) | stored impact; plays only when the hit lands |

The move table is `MOVES` in the script: key, source file, mirrored, striking part, attack id, power, windows (a window
may carry its own part), rate, sound.

- **Attack ids pick RinRin's native reaction:**
  - 0x6D → dm_right;
  - 0x6E → dm_left;
  - anything else → dm_body;
  - 0x76–0x78 (the heavy rows) → knockdown.
- Damage comes from the native row for that id.
- **Mirroring** (the "cross_m" variant) swaps left/right bones and negates x translation and the quaternion y/z.

### Receivers and pairs

- `RECEIVERS` retargets the victims' clips (suplexed, swept down, get-up) onto RinRin's rig (`rin_clips`) and the
  grunts' rig (`grunt_clips`).
- **Check who does what in a pair.** In grab 0011c348 the "receiver" (a henchman) is the one *doing* the suplex. Gene
  is lifted to 1.4 m and slammed. The roles were swapped at first. Look at the hip heights per tick to see which actor
  goes airborne and which ends lying down.
- **Pair placement:** the thrown actor's container relative to the thrower's container at tick 0
  (`suplex.offset`/`yaw`); each then plays its own root travel. The slam frame comes from the thrown hips' drop
  (1.4 m → 0.33 m by tick 36).

## 4. In the game (where the data is used)

- `crates/mw-sim/src/kreely.rs`: loads `kreely-moves.json` into the fight data. Clips go into Jack's bank, cues into
  the combat cues, receivers into RinRin's bank.
- `crates/mw-sim/src/fight.rs`:
  - `PlayerFight::kreely_attack`: the move table. Punch string cross → cross_m → knee; W/S + punch for the
    step/turn jab; Q heavy (W/S + Q for the spin kick / sweep); mash for the rush; point-blank punch from the aim for
    the whip.
  - The suplex is `STATE_SUPLEX` / `RIN_SUPLEXED` with `step_suplex`. The native grab reach (pl0030_0226 to its
    contact frame 10) plays first, then the suplex. The sweep sets RinRin's `selector` 2 so her knockdown plays
    `gh_sweep_down`.
- `crates/mw-game/src/play_street.rs`: the grunts' sweep (`Swept`) and suplex (`Suplexed`) states, using
  `grunt_clips`.
- **Inspection switches:**
  - `MW_RIN_PASSIVE=1`: RinRin never attacks;
  - `MW_POSE_LOG=1`: prints the player's posed joints, to compare against the data;
  - `MW_CAM_PL=x,y,z`: camera on the player.

## 5. Checking your work

| Check | How | Good result |
|---|---|---|
| Faithful retarget | `inspect_moves.py compare <key>`, `stick <key> 0,6,9 out.png` | Directions match the source (for uncorrected moves) |
| No flips | `inspect_moves.py snaps` | No big half-frame jumps except genuine fast swings. Confirm a spike by comparing hand speed with the source: the power punch's off hand really moves 1.3 m in a frame. |
| Punch height | `inspect_moves.py levels` | Fist within a few cm of the shoulder at the hit and on the approach |
| Game | scripted run with `MW_RIN_PASSIVE=1`, `MW_PLAY_LOG`, `MW_PLAY_SHOTS` | Each move shows RinHit / RinGuard / RinEvade |
| Explorer | `publish_explorer.py`, then reload :8781 | Moves listed under Kreely, pairs under RinRin. Set Preview fps to 30. The explorer only redraws while its tab is visible. |

## 6. Using a different explorer next time

The retarget only needs, per clip:
- a skeleton with names, parents and rest translations (rest rotations identity, ideally a T-pose);
- per-frame local TRS;
- root travel and yaw;
- fps.

To bring in another game:

1. **Write its exporter** (the equivalent of `export_godhand.py`) that writes the same JSON layout into a new
   `assets/native/<game>/`. Sample through that explorer's own solver so IK and curves are baked in.
2. **Confirm the conventions:**
   - up axis, forward axis, which side is the character's right;
   - units, fps;
   - whether rest rotations are identity.

   If they differ, convert in the exporter. If its rest pose isn't a T-pose with identity rotations, the alignment
   `A` has to include the rest rotations: `world_t = W_s · R_s,rest⁻¹ · R_t,rest`.
3. **Map its bones** to the MadWorld 0–23 layout (the `MAP`, `CHILD` and `MIRROR` tables).
4. **Add its moves** to `MOVES` and `RECEIVERS` with windows from its hit data, then rerun steps 3–5.
5. **Look before tuning.** Use the stick figures against the source, then the game. Adjust the style corrections per
   game; another source may need none.
