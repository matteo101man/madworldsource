# cPl0000 combat + RinRin fight rules (phase 2/3 working notes)

Sources: headless Ghidra decompiles (`tools/pl0000/decompile.sh`, output in scratch, never in the repo), constants via
`tools/pl0000/dol.py` (DOL) and `tools/pl0000/rso.py` (enemy RSO), staged data `assets/native/combat.json`
(`tools/pl0000/stage_combat.py`). Labels: [N] native, [O] observed, [I] inferred.

## Input buffers [N] (per frame in fn_8027621C)
Game pad bits after remap: A = 0x10, B = 0x20 (INPUT-HLE). Trigger edges `+0xEE4`, holds `+0xEE0`.
A trigger -> `+0x10F8` = 8, B trigger -> `+0x10FC` = 8, bit 0x800 -> `+0x1100` = 15, bit 0x1000 -> `+0x1104` = 8;
each counts down 1 per frame. Remote accel `+0xEF0` (x) / `+0xEF4` (y).

## Attack entry 802C3B5C [N] (default weapon, no target-specific branch)
- `param_2 == 0` (from locomotion): `+0x12A8` combo index = 0, clear flag 0x40.
- A buffered (`+0x10F8 != 0`) and A no longer held (`+0xEE0 & 0x10 == 0`) -> state 39. Holding A = grab path.
- Remote swing: |accel x| > 2 or |accel y| > 2 -> `+0x12A4` = 1 (swing), `+0x12A8` = 1 (|y| > 2, vertical) else 3
  (horizontal) -> state 39.
- Grab branches (A held near a target) -> states 0x3C / 0x26 / 0x31 / 0x19: not translated.

## State 39 (A 8028562C, B 80285744) [N]
Phase 0 picks the attack:
- A string (`+0x12A4 == 0`): idx = `+0x12A8` & (flag 0x40 ? 7 : 3). idx 0 or stick y <= 0.2:
  0 `1100` PUNCH_A0 (0x6D, timer `+0x12CC` = 45), 1 `1101` A1 (0x6E), 2 `1102` A2 (0x6F), 3 `1103` A3 (0x70),
  4-7 same motions as A4-A7 (0x71-0x74; idx 4 also `+0x11BC` = 10, timer 60). Otherwise (combo > 0, stick forward)
  `1105` PUNCH_B2 (0x77) with flag 0x100. `+0x128C` = 10 (A) / 15 (0x100); A path: `+0x12A8` += 1, else 0.
- Swing (`+0x12A4 != 0`): `+0x128C` = 15; idx = `+0x12A8` & 3: < 2 `1104` PUNCH_B1 (0x76, chainsaw vertical),
  2 `1105` B2 (0x77), 3 `1106` B3 (0x78); then `+0x12A8` = 4 if it was 1 else 0.
- Motions play with their `_00.seq` cue file (fn_8012730C clears `+0x1280/1/2`).
Phase 1: motion end -> state 0. Lunge toward the target (`+0x125C`) when it is closer than 2.0: speed `+0x12C4` =
0.2 x (dist - 1.6) (not for 0x75/0x77/0x78), decays x 0.6 per frame; face it with RadLerpRange 0.3.
A[39]: `+0xEE4 & 0x10` counts presses (`+0x12C8`); while the combo window `+0x1282` is open and phase < 0x32,
802C3B5C(this, 1) re-enters 39 with the next index. Rush (phase 0x32: >4 presses while `+0x12CC` runs) not yet read.

## Motion cues (player handler 802C1C7C) [N]
`_00.seq`: 48-byte events, type u8 @0, frame f32 @0x14, s16 a @0xC, b @0xE.
op 1 rate = f; op 7 hit trigger (`+0x1280` = 1, part `+0x1288` = a if != 0, `+0x128C` = b if != 0, serial++);
op 0xB combo window (`+0x1282` = 1); op 0x13 / 0x14 second hit on / off; op 0x24 sub 3 hit-stop (setHitStop
80279CA8); op 0x25 effect; op 0x26 sound. Enemy `_00.seq`: op 19 (0x13) hit on with bone id a, op 20 off.
`_10.seq` = sound / voice / footstep cues.

## Hit reservation and resolution [N]
802C81F4 reserveAtkHit(radius 0.6 = 8056A008, part, atk id, power): stores `+0x1A64..+0x1A78`; power overridden by
cAtHitInfo (0x80694064) row [0] when nonzero. 802C830C builds the cAtHit: default weapon = sphere
(setAtSphereVec 801B5440) at articular part `[+0x1288]` world position, radius `+0x1A68`; chainsaw 0x78 = capsule
from the weapon (wp0600 vtable +0xB4). Parts are the MDB node list in order (getPartsPtrImpl 801B2450 with b=0), so
part 14 = Jack bone 14 (left hand), 9 = right hand.
cAtHitInfo rows come from `files/plem_common.ini` `[damage_pl]` / `[damage_em]` (row names matched against
DOL tables 0x805588B4 x168 / 0x80558DF4 x94 by 801B59A0). `[damage_pl]` columns: 1 power, 2 hit-stop frames,
3-5 Havok forces, 6 enemy spacing, 7 sever attribute, 8 sever multiplier, 9 stun multiplier, 10 hit radius,
11/12 points on damage / kill, 13 hit-mark part. Example PUNCH_A0 = 5 power; PUNCH_B1 (chainsaw) = 50.
Player damage volumes (802705DC): parts 0x18 r0.14, 2 r0.2, 0xF r0.18, 8 r0.14, 0xD r0.14, 0x11 r0.16, 0x15 r0.16.
Enemy damage volumes: `em0440_00.emh` ("emh", v1, 27 records of 0x80 bytes from 0x10: u16 bone, u16 ?, vec4
offset+radius @+0x10, vec4 extent @+0x20, bind matrix @+0x50) [I layout].

## Player damage (cPl0000_frameUpdate 802727B0) [N]
For each damage work hit this frame: HP `+0xEC` -= damage (`+0x1690`) (except atk 0x17C); HP < 1 -> HP 0, state 95.
Else 80274DC4 picks the reaction motion by hit direction and attack (returns knockdown): false -> state 85 (0x17C ->
86, 0x17D -> 87), true -> state 92. HP max `+0xEE` = 100 [O]; heal/regen `+0x1506` added each frame.

## RinRin (cEm0440, enemy RSO) [N]
Tuning `+0xA30` = files/em/em0440.csv (index = row - 2, normal/hard columns), e.g. 0 uraken range 5, 1/2 cooldown
60-90, 3 tetuzankou range 8, 4/5 200-300, 6 throw range 10, 7/8 30-90, 9 taunt distance 8, 10 taunt % 20,
16 clash damage 300, 24 punch evade % 45/55, 25 chainsaw evade % 45/55, 26 giant-slalom range 15, 27/28 1000-1200,
29 slalom % 30, 30 guard % 30/40, 31 guard time 0.1. Distances are compared squared.
Frame step `+0x898` = 1.0 (enemy timers are frame counts, SIXTY-FPS). turnDir = RadLerpRange(0.2 k, pi/6).
- wait (0000, blend 15): player > 8 -> 20% per frame taunt (24); 5 < d < 15 -> run; chkAtkStart; chkScaffold.
- run (0010, blend 2): d < 2 -> wait; d > 8 -> 1% taunt; chkAtkStart; scaffold jump when flagged.
- chkAtkStart (in area 50<x<590, 0<y<1000, z<0 of 0x8069BD10's frame): on ground (y < 18) and `+0xE4C` <= 0 and
  d < 15 and rand < 30% -> giant slalom (28), cooldown rand(1000,1200). `+0xE48` <= 0: d < 5 -> uraken (2),
  cooldown rand(60,90); d < 8 -> tetuzankou (3), rand(200,300); d > 10 -> rand(30,90), throw (14) if player y < 19.
  Cooldowns tick down 1 per frame (onMove). Initial `+0xE4C` = rand(1000,1200).
- uraken (0100, blend 2): face the player at once; hit while cue op 19..20 (bone 9, frames 20-24), radius 1.5,
  type 0x172 = em PUNCH_A0 (10 power, 6 hit-stop). tetuzankou 0101: bone 7, frames 28-36.
- Being hit (onPreMove): reaction by type (0x6D dm_right 21, 0x6E dm_left 22, 0x6F/0x70/default dm_body 23,
  chainsaw 0x75-0x78 dm_explosion 16 sel 1) + evade roll (idx 24 punches; idx 25 chainsaw, or the same attack type
  as last time); no evade/guard when the player is behind her (facing gap < 50 deg) or she is in kamikaze/taunt/
  explosion; guard roll idx 30. Guard (29) > evade (random step 9/10/11, attack cooldown reset) > damage
  (HP `+0xEC` -= power, x1.1 with flag 5). HP < 1 -> HP 1, finish_wait (27). Start HP 1600 [O].
- Motions: wait 0000, run 0010, uraken 0100, tetuzankou 0101, throw 0102-0104, slalom 0105, bomb 0001, taunt 0002,
  guard 0003/0004, steps 0024+0025/0026/0028/0029, jumps 0023/0024+0025/0026/0027, scaffold 0020-0022,
  kamikaze 002a/002b/002d, clash 0110-0113, finish 0121-0125, dm_right 1000, dm_left 1001, dm_body 1002,
  explosion 1003/1004, down 1004/1006, finish_wait 1007, electric 0022/1010.

## Implemented in Rust (2026-11-08): `crates/mw-sim/src/fight.rs`, presented by `mw-play`
- Motion player with cue firing (`Motion::advance`), actor-space FK (`bone_frames`), attack spheres vs damage volumes,
  per-attack serials (one hit per cAtHit), hit-stop from column 2.
- Player: attack entry (A tap / swing), state 39 A-string 1100-1103 (+ A4-A7 after a vertical chainsaw hit flag),
  chainsaw 1104/1105/1106 with op 7 and op 0x13/0x14 windows, lunge, combo window chaining; damage: light 85
  (100F/100A/100B, turning to the attacker), knockdown 92 for types < 0x163 (1015 / 1010 / 1013 / 1014 by hit
  direction, simplified angle tests) then get-up 94 (1092), death 95 (108A, countdown 0x78>>1, hold),
  continue (restart 802C1264: HP max, get-up 1090) on Enter [continue trigger inferred: room-script layer].
- Finisher: swing next to a finish_wait RinRin (8028BCB0; range 3.0 [inferred]) -> player 68 / Rin finish 26 paired
  (shared origin, MOT roots kept): 1305 + 0121, 1306 + 0122 (+0123 layer = meter), meter a/p from held-X shake
  (802A688C phase 13), phase 14 wait 20 / timeout 480>>1, 1308 + 0124, 0125 -> Victory.
- RinRin: wait/run/taunt, chkAtkStart (uraken 0x172 r1.5, tetuzankou + giant slalom 0x161 r1.0 via cue windows),
  steps, guard, dm_right/left/body/explosion, finish_wait, pl_dead (0002 then 0000).
- Kreely pistol (added): ray from the gun hand at the overlay's shot frames, HANDGUN row 0x61 (power 20).
- RNG: xorshift stand-in for cRandom 800BB79C [inferred]; seed `MW_SEED`.
- Clash (keen competition): player 78 / Rin 17 (810CEA04) paired 04AA-AB / 0110-0111, meter -0.08 + 0.04 x (remote +
  nunchuk shake) when both > 0.5 (PLAYER-COMPETITION), directional check 1.4 / 1.1, win 04AC / 0112 with tuning 16
  (300) damage at the 0112 cue -> 1004 or finish_wait; loss 04AD / 0113. Entry: a chainsaw swing during Rin's giant
  slalom stands in for startAtk(TATEGIRI/YOKOGIRI 0x2A/0x2C/0x30/0x31) [inferred: the B-chainsaw cut path that
  issues those types is not located]; starting meter 0.5 and the loss timeout 480>>1 [inferred]. Result matches the
  oracle (Rin 1600 -> 1300 per clash).
- Fan throw (810CDEA8, cIronfan 810CAF54 / fling 810CE40C): 0102 wind-up, two fans on getHermiteCurvePoint (8008C924)
  paths from the hands to the player's chest (tangents +-10 sideways, dt = 1 / (0.5 x distance x tuning 13)), out
  with JUMP_KBOMB 0x161, back with 0x18C, radius 0.5, spin 0.5585/frame; 0103 while they fly, 0104 catch. Fans drawn
  by moving Rin's parts 100 / 115.
- Scaffolds: 9 points from the static initializer (810D1950 -> 0x81174728, values in `fight::SCAFFOLDS`) with their
  jump table; chkScaffold (tuning 23 = 600 countdown, 30%), run to the scaffold, scaffold_jump 0020-0021-0022 with the
  X/Z lerp, scaffold_wait, bombs (0001 + tuning 21 bombs), taunt, hops 5/6/7 (0023 + 0025/0026/0027 + 0024, linearTrans),
  kamikaze 002a/002b/002d (ROLLING 0x17A). Bomb flight/blast, kamikaze path and the hop timer ratio are [inferred].
- Player jump (802C2E68, B 8027E54C / 8027E7EC / 8027EAB8 / 8027EC50 / 8027EE54): 00A0 -> 00A1 / 00A5 -> 00A6 ->
  00A2, jump power 0.151 x GameVSyncF (x1.05 standing), horizontal 0.16, gravity 0.0145 x GameVSyncF per frame
  [integration inferred; apex ~1.6-1.9 m vs 1.48 m observed for the jump attack].
- Body separation 0.85 m [inferred], lock-on camera on Tab [inferred].
- Rush (B[39] phase 0x32): more than 4 A presses inside the 45-frame window from A0 -> 1111 RUSH_PUNCH (six cue hits,
  10 each), or 1112 FINISH_BLOW (150) when the target has <= 10 HP; keep mashing to keep rushing [phase 0x33 rule
  simplified].
- Jump attack (802C5C88 -> state 39 with the air flag): 0.1 hop, 1122 PUNCH_JUMPATK0 (cue window, radius 0.75), then on
  the ground 1123 PUNCH_JUMPATK1 (80).
- RinRin dm_explosion (16) / dm_down_damage (30): 1003, down with the tuning-14 stun (90) after chainsaw hits, hits
  while down -> 1006 without guard/evade, get-up 1004 with its 0x161 spin window.
- Giant swing [simplified]: hold A next to a downed RinRin (802C5DD8 -> 802C6020: her catch state 6/0xC gives player
  0x31 + startCatch(0xB); "down = catchable" is inferred) -> paired 022A/em0110_1050, spin 022C/1051 driven by shake,
  release on A after 60 frames or at 240, throw 023E/1052, GIANTSWING (0x55) 300, halved for a weak spin (the CSV note).
- Electric fence: bodies named `*_ST10` are surface type 0x10 (the dm_giantswing / dm_explosion 800ABAA8 ray checks
  for type 0x10): a thrown RinRin (release velocity facing x (0.1 + 0.7 k), up 0.05 + k [k scale inferred]) or an
  explosion knockback into them -> dm_electricshock (20): 1010, tuning 17 (100) damage, 0022, wait / finish_wait.
- RNG: Hw::cRand ported exactly (seed x 1664525 + 1013904223; getS16 / getF32 with 16777215); the shared instance's
  runtime seed is `MW_SEED` [inferred].
- Camera: mode-0 follow camera (PC-CAMERA.md: dist 2.25, side -0.2, height 1.5, pitch -0.026, fov 50, yaw chase only
  for 25..135 deg errors at <= 5 deg x GameVSyncF per frame, wall pull-in); mouse look pauses the chase for 45 frames
  (PC addition), Tab lock-on chases the target.
- Evade (802C2610, state 7): nunchuk shake > 1.2 (PC: Shift) -> 0033, or 0031 / 0032 away from an attacking RinRin
  [target query simplified; i-frames inferred].
- Fixed: ray directions are normalised in f64 (raw_surface_ray rejects |d|^2 - 1 > 1e-8); wall slides were silently
  failing before.
- Not yet: the B-chainsaw cut states, the native giant-swing spin/release logic,
  dodge, Rin's electric shock (20) / scaffold-bomb damage (18) / bad smell (15), sound and effect cues (event ids are
  not mapped to the staged streams yet, AUDIO-API), the native camera, oracle diffs.

## Evidence
- `cargo test -p mw-sim` (28): electric fence, evade; giant swing; rush, jump attack; clash -> 300 damage, fan throw hits, scaffold climb/bomb/kamikaze cycle, jump; A-string chain lands, Rin attacks, finisher with held shake reaches victory,
  death -> continue.
- `evidence/native-play/script-fight.*` (punch strings, chainsaw, guard/evade/hit reactions, Kreely swap),
  `script-finish.*` (MW_RIN_HP=120 debug start: chainsaw/punch mix -> finish_wait -> finisher -> Victory, tick 1944),
  `v*.png` captures, `script-kreely-pistol.*` (pistol: guard, HANDGUN hit 1600 -> 1580).

## Clash entry (resolved 2026-11-09)
- 8028BCB0 is the clash trigger. The swing check 802C3C0C and the attack states call it before the swing itself:
  target = lock target (+0x125C), its category 0x2xxxx, chkEnableKeenCompetition (vtbl 0x1D0) true, +0x11A5 == 0,
  +0x396 != 0, 80085894 on the player position == 0, |dy| < 0.15 unless the target is em0460, no competition yet
  (flags1124 0x1000000) and |remote accel x| or |y| > 2.0 (80569F78) -> reqStartKeenCompetition (vtbl 0x160) ->
  startKeenCompetition(player, 0xB). There is no distance test.
- cEm0440::chkEnableKeenCompetition (810CB518) reads rig flag 0 (+0x864 bit 0). Cue 35 (a = 0) in em0440_0105_00.seq
  sets it at frame 10 and clears it at frame 60; end_atk_giantslalom (810D08B0) clears it. So the clash window is a
  chainsaw swing during frames 10-60 of the giant slalom. The port follows this; +0x11A5 / +0x396 / 80085894 are
  taken as passing [inferred].

## Clash entry search (history)
- Player code never stores 0x2A/0x2C/0x30/0x31 into the attack id `+0x1284`; the only player-side uses of those values
  are catch states passed to the target's vtbl +0x90 (startCatch: 0x2A in B[63] 802B5198, 0x30/0x31 in the finisher
  802A4A78..802A99B4).
- 802CBD54 (called before the target's vtbl +0xAC startAtk in state 39) returns the motion's first cue frame
  (getFirstEventFrame 8025B0C0) as the time-to-hit; it does not remap the attack type.
- So the startAtk(TATEGIRI/YOKOGIRI) source is outside cPl0000's state 39 (weapon object or another player state);
  the port keeps the chainsaw-swing-during-slalom stand-in, which matches the natural clash replay.

## Later additions (2026-11-08, continued)
- Giant swing now follows B[49] 802AF28C: grab 022C; 023E at rate 0.5, power +0x1160 = 0.4, 300-frame timer; spin
  rate +0.01 to 1.0, peak shake +0xF70, power = max(power, 0.2 x shake), 022A when power >= 0.5; facing += 0.15 + 0.1
  x peak; release on timeout or A release; keep turning until facing the camera (+0x1134), throw 023F. GIANTSWING
  0x55 = 300, halved below power 0.5 [weak-swing threshold inferred]. PC held X (1.59) gives power 0.32 -> weak swing.
- Player reactions now follow 80274DC4 / 802C0D4C: knockdown 1015 (hit from behind), 1011 / 1016 (HP <= 40%) for
  head-on hits (+0x1AB8 = 1), 1018 / 1013 / 1014 by side; get-up 1090 after 1011/1016, else 1092 (HP >= 40%) / 1091;
  death keeps the knockdown motion (oracle: 1015 / 1016 under state 95).
- RinRin flag 5 (giant slalom, scaffold jump): no guard / evade, damage x1.1 truncated (onPreMove fdb4) -- the
  oracle's -55 / -22 / -11 / -6 HP steps.
- Sound: `research/AUDIO-API.md` (RedSpark packs decoded, cue ids -> waves, mw-play plays them).
- Oracle behavioural diff: `research/ORACLE-COMPARE.md` (`tools/pl0000/oracle_compare.py`).

## Additions (2026-11-09)
- Input buffers now count down one per frame (fn_8027621C); before, a stale A press could start a later attack.
- Hit reactions follow the 80274DC4 attack-id switch: knockdown (92) for 0x15B / 0x161 / 0x162 / 0x17A / 0x17B (the
  kamikaze was a flinch before); 0x172 -> 100F, 0x173 -> 100A, 0x174 -> 100B, 0x175 -> 100F; 0x18C (returning fans)
  -> from behind a random 1024 / 1025 / 1026 (cRand 0..2), else 100F / 100D / 100C / 100B / 100A by the hit angle
  (acos thresholds 0.611 / 2.531).
- Stun (frameUpdate): a 0x18C hit sets +0x1560 = 0x4B0 >> 1 (600), other hits clamp it, -1 per frame. When the
  flinch (B 802BE694) or the get-up (802C0D4C) ends with +0x1560 > 0x1C2 -> dizzy state 91 (B 802C03EC): 1097 at
  blend 2, meter +0x1240 = the counter, -2.5 x (remote + nunchuk shake) per frame, < 0 -> idle (802C6768 taken as
  passing). Without shaking the player stays dizzy.
- Grab whiff (802C5DD8): A held with the press buffer spent, once per hold (flags1124 0x800); no catch started ->
  +0x14F0 = 2, state 21 (B 8027FDC4) plays 0226, then idle [its 0x10D whiff hit volume is not ported].
- Idle fidgets (8027AFE4, PL0000-LOCOMOTION): countdown rand(0, 2) + 10 finished idle loops, then 0008 / 0000 /
  0007 / 0000 at blend 4 with announcer voice 0x1117 (+0x220 taken as "motion reached its end" [inferred]).
- Evidence runs: the open-loop key scripts (`evidence/native-play/script-*.json`) were timed against the old stand-in
  clash and the non-decaying buffers; with the native rules (and the idle fidget's extra cRand draws) they no longer
  reach Victory and stay as coverage runs. `fight::Autopilot` (a reactive test player, not native) drives
  `cargo test -p mw-sim autopilot` (Victory on seeds 1-3 at ticks 3321 / 5377 / 4705, natural clashes) and
  `MW_PLAY_BOT=1 mw-play` (`evidence/native-play/bot-*.log.jsonl`): Jack wins at tick 4597 with 11 clashes and no
  deaths in the full app (collision, camera, audio); Kreely (punches only, no clash) wins at tick 14253.
- Oracle diff after these changes: 40/66 player and 46/60 RinRin state-motion pairs (was 31 / 44); newly matched:
  21/0226, 91/1097, 92/1013/1015/1016, 94/1092, 95/1016.

## Additions (2026-11-09, continued)
- Clash meter now native (802956B4): phase 1 end sets raw +0x1240 = smoothed +0x1248 = 0.75 (8056A084); phase 2
  cooldown 0x14, timer 0xF0. Phase 3 per frame: raw -0.08, +0.04 x (1088 + 108C) when both > 0.5 after the cooldown,
  clamp [-1, 1]; smoothed += k x (raw - prev), k 0.0375 (0.1 while raw < 0); end counter +0x124C counts frames with
  smoothed > 0.95 (while the previous was > 0) or < -0.75 (while negative), reset otherwise; timer 0 or counter > 30
  -> loss (phase 10) if smoothed <= 0 or the timer ran out, else phase 4 (delay 0x1E, timer 0x3C) -> phase 5:
  1090 > 1.4 and 109C > 1.1 wins, timer 0 loses. Loss: 04AD / Rin 0113. Test `clash_without_shaking_is_lost`.
  The +0x27DC start (0.75) is [inferred].
- Rig flag 0 now clears on any exit from the slalom (the clash entry sets Rin's state directly); before, a stale
  flag let a clash start after Victory.
- Chainsaw slot (802C3338, 8027B6F0, 8027D2D8, 8027AFE4 phase 10): weapon slot 1 (type 6) while B is held (PC:
  Jack's Secondary / right mouse) [the 802C3338 toggle and buffer conditions are simplified to "while held"];
  idle 0425 at blend 4, run 0427, release in idle -> 0424 stow at blend 2 -> idle. The fade layers and the wp0600
  chainsaw motions are not drawn. Test `holding_b_revs_the_chainsaw_and_releasing_stows_it`.
- Announcer 0x1018: 8027621C requests it every frame while HP < 0.3 x max (8056A02C); cooldown 0x1518.
  Not ported (not part of this sequence): 0x1143 (chainsaw fuel +0x112C / +0x11AE), 0x1145-0x1147 (pickups
  0x405xx), 0x106A (environment kill on a caught object), 0x10EB / 0x106B / 0x10EC (weapon pickups), 0x1106 /
  0x1042 (mission-flag gated).
- Victory music: the stage-201 boss path requests no jingle. 80147BBC plays event 0x21 (ev21; its BGM timeline
  starts BS04C, which is why the later 801246D8(0x18) request is skipped) and then waits for the "subMission clear"
  / "roomClear" messages; the stage-201 code requests events 0x20 / 0x21 only (80123820 call scan). What happens
  after roomClear (result screen) is outside this sequence; mw-play stops the boss BGM at Victory [inferred].
- Key-driven evidence: `MW_PLAY_BOT=keys` / `keys-kreely` makes the autopilot press PC keys through the script
  key path (8-way WASD, Punch, SwingH, Secondary, Shake, Dodge, Continue, Swap) and records them with
  `MW_PLAY_RECORD` as `evidence/native-play/script-keys-{jack,kreely}.json`; replaying those with `MW_PLAY_SCRIPT`
  goes through the keyboard input path only and is tick-identical to the recording.
- Evidence (HP-override runs, `scratchpad runshort.sh` pattern): `MW_RIN_HP=400 MW_PLAY_BOT=keys` -> Jack wins at tick
  2329 (clash won, finisher); `MW_RIN_HP=200 MW_PLAY_BOT=keys-kreely` -> Kreely wins at tick 2541 (pistol, punches,
  finisher). Replaying `script-keys-{jack,kreely}.json` through the keyboard path is tick-identical (0 differing
  ticks). Loss paths are covered by `clash_without_shaking_is_lost` and `death_then_continue_restores_the_player`.
- Headless full fights (no Bevy, no collision): `cargo run --release -p mw-sim --example sim_fight -- evidence/native-play 12`
  writes `sim-{seed}.log.jsonl` (read by oracle_compare). All 12 seeds reach Victory; seeds 3 / 6 / 9 / 12 start the
  player at 1 HP without dodging and die 2-4 times, continue and still win; seeds 4 / 8 / 12 lose a clash (no shake)
  and win later ones.
- In-app death and continue (`MW_PLAYER_HP=1 MW_PLAY_SCRIPT=evidence/native-play/script-death.json`): Rin's tetuzankou
  0x161 kills at tick 28 (state 95, 1016), Continue at tick 300 -> state 94 with 1090 and HP 100, then uraken 0x172
  hits flinch (85 / 100F) for 10 each.
- In-app coverage (`script-coverage2.json`: jump attack, chainsaw rev in idle / run, release): 0425 / 0427 / 0424 and
  39 / 1122 / 1123 now appear in mw-play logs. Oracle diff with all current logs: 45/66 player, 49/60 RinRin pairs.
- Frame-exact comparison (research/FRAME-COMPARE.md, recorder by session local_59f64723): the fixes it drove are
  listed there; the first divergence moved from frame 337 to 390 (player position, knockdown root cut) and the
  state / HP divergence from 387 to 487.
