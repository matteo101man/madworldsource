# Grab / carry / throw and the in-game radar: port spec

Scope: two native features for `crates/mw-sim` (fight rules) and `crates/mw-game` (street grunts in `play_street.rs`, HUD in `play_hud.rs`). Part 1 covers Jack (cPl0000, DOL) grabbing a regular grunt (cEmZako / cEm0101), holding him, walking with him, head-butting him and throwing him. Part 2 covers the bottom-left radar HUD (cIdRadar).

Tags follow `ZAKO-AI.md`: **[obs]** = read in decompiled code or data, **[inf]** = inferred from names, call shape or data, **[unres]** = not resolved. Offsets are `this+0x…` byte offsets. `param_1[n]` in an enemy body is the 4-byte word n, so the byte offset is 4n. Times are frames at 30 fps. Distances are metres. Angles are radians unless marked.

Sources: headless Ghidra through `tools/pl0000/decompile.sh`. The decompiled text is kept outside the repo in `C:/Users/Matteo/AppData/Local/Temp/claude/C--Users-Matteo--claude-spares-spare-SC961L/b18b4d75-495f-4d48-a047-7b3fa286cef3/scratchpad/grab/` (`pl_*.c`, `em_*.c`, `radar*.c`). Constants come from `tools/pl0000/dol.py` (DOL) and `tools/pl0000/rso.py` (enemy RSO). Enemy motion ids were resolved from `cEm0101::getModelData` `8106eb98` with the scratch script `motres.py`. It checks out against the ids already known: 0x83d = `em0110_1078`, 0x25 = `em0110_0036`, 0x13 = `em0110_0012`.

---

## Part 1: grab, hold, carry, throw

### 1.1 Overview

```
player idle/walk/attack ── A held ≥ press-buffer, target in cone ──▶ 0x14 grab start (pl0030_0220)
        │                                                              │ enemy: startCatch(3) → catched_start 0x5c (em0110_1071)
        │  (no target: 0x15 whiff, pl0030_0226)                        ▼
        │                                         0x16 HOLD (A still held): idle pl0030_0221 / walk fwd 0227 / walk back 0228
        │                                            │  enemy: catched_idle 0x5d (em0110_1072, struggle loop em0110_1077)
        │          A released ──────────────────────┼──▶ 0x17 release (pl0030_0222) → idle; enemy catched_end 0x5f (em0110_1073)
        │          nunchuk shake > 1.2 ─────────────┼──▶ 0x28 HANG_HEADBUT (pl0030_0223) → back to 0x16
        │          remote swing vertical ───────────┼──▶ 0x28 WHANG_THROW (pl0030_0224)      → enemy motion_blow 0x45 (em0110_100c/100d/100e)
        │          remote swing horizontal ─────────┴──▶ 0x28 WHANG_SIDE_THROW (pl0030_022b) → enemy motion_blow 0x45 (em0110_1013/1014/1015)
        │  grunt struggle timer runs out ───────────────▶ enemy catched_shake_off 0x60 (em0110_1075) → wait; player loses the target
```

Every clip named here is already staged: the `pl0030_02xx` clips are in `assets/native/jack-player-motion.json`, the `em0110_10xx` clips in `assets/native/kungfu-motion.json`, and the cue files in `research/motion-cues.json` **[obs]**.

### 1.2 Input and target choice (player)

- **Button.** A is bit 0x10 (`+0xEE0` holds, `+0xEE4` edges, `+0x10F8` the 8-frame press buffer, see `PL0000-COMBAT.md`). The attack-entry dispatcher `fn_802C3B5C` sends a press-and-release inside the buffer to state 39 (punch). The grab path is `fn_802C5DD8`. It runs when **A is still held and the press buffer has expired** (`+0xEE0 & 0x10` and `+0x10F8 == 0`), the player is on the floor (`+0x396 != 0`), `+0x11A5 == 0`, and flag `+0x1124 & 0x800` is clear. That flag is set on the first attempt, so there is one attempt per hold. It is cleared again when A is not held **[obs]**. So a grab is "hold A for about 8 frames" **[inf from the buffer length]**. PC: hold LMB/E (`PC-CONTROLS.md`).
- `fn_802CC28C` runs first. It is a stage-weapon special case (`+0x12CE`, wp0760 `pl0030_0879`), not a grab **[obs]**.
- **Target search.** `fn_802C6020` → `fn_802C2304(1.3963, 2.5)`:
  - It calls `801df940(facing +0x74, 1.3963 rad (80°), 2.5 m, enemy list 0x80694a84)`, or `801dfaf0` when `+0x11A5` is set. Range is 2.5 m. Whether 80° is the half-angle or the full cone is **[unres]**; use ±80° until a capture says otherwise.
  - When stage flag 3 is set, catchable objects from list `0x80698854` (`8020b7c0`) also compete **[obs]**.
  - `fn_802CC5B0` casts a line of sight from the player to the target, both points raised 0.9 m, with collision filter 0x1e. A hit means no grab **[obs]**.
- **Branches by target category** (`target+0xF4`):
  - `0x1xxxx`: catch object (tire, sign …). It goes through the same height test.
  - Enemies first ask the target `vfunc 0x1AC` = `cEmZako::chkEnableCatch` `8101b084`. It rejects the grab when the grunt has flag `0x1180` bit 2, flag `0x1214` bit 0xd, cfg `0x868` bits 0x18 or 0xc, or (`8105e284` and not `0x8de` bit 3). It also tests its Havok proxy against the player's position (proxy shrunk to 0.25/0.5, raised 1.6 m; `800b3804`) **[obs; bit names unres]**.
  - Height test: |target.y − player.y| ≤ **0.3 m**, or 1.5 m when the target's catch type is 0x25. Otherwise `vfunc 0x250` must allow it **[obs]**.
  - **Catch type** = `target+0x398`, published by the enemy **[obs read; writer unres]**:

    | `+0x398` | player state | enemy `startCatch` state | meaning |
    |---|---|---|---|
    | **3** | **0x14** | **3** | **standard standing grab (this spec)** |
    | 0x1c | 0x19 | 0x25 | chainsaw skewer hold: `pl0030_110a/110b/110c` with `c030–c032` chainsaw layers, repeated hit `+0x1284 = 0x4e` **[inf]** |
    | 0x25 | 0x26 | 0x29 | grab out of a jump: `pl0030_110d` + `c033`, sends MADBUSTER 0x53 **[inf]** |
    | 6 / 0xc | 0x18 (or 0x31 for RinRin) | 0x37 (0xb for RinRin) | down / stunned grab: `pl0030_0240` + `c034`, sends DOWN_THROW 0x4f; RinRin gets the giant swing |
    | other | – | – | no grab |

    In `802C6020`, when `80085894(0x80602d68, pos)` is non-zero, the player calls `startCatch(0x1b)` and goes to state 0x25 instead **[obs; area meaning unres]**.
- **No target or refused** → state **0x15** (whiff, `pl0030_0226`), `+0x14F0 = 2` **[obs]** (already ported, `PL0000-COMBAT.md`).
- The grab start sets `+0x1260` = held object, `+0x12D8 = 1` (holding), and `+0x14F0 = 2` (hold mode "bare hands"; 5/6/7 are other modes) **[obs]**.

### 1.3 Player states while grabbing (state table `research/pl0000-state-table.txt`)

| id | A / B handler | what it does |
|---|---|---|
| 0x14 (20) | `8027F894` / `8027F898` | **Grab start.** Phase 0: hit part `+0x1288 = 9` (right hand). Camera mode 10 via `802c9928(-1, 10)`, or 0x1b/0x1c under stage flags 0x12/0x14, or 3 in modes 6/7. Announcer 0x122 under flag 3. Clears `+0x11CC` (can attack). Phase 1: plays `motion[+0x14F0]` = **`pl0030_0220`** (mode 2; others `pl0030_0000`) at rate 1.0, blend `+0x1148`, plus **`wp0600_0220`** on the chainsaw-arm rig `+0x1530`. Phase 2: on motion end → **0x16**. Phase 3 is a scripted stage variant (stage flag 0xE/0x10: turn toward fixed points (19, 1.1, −32.5) / (0, 1.1, −76) within 14 m, then 0x16) **[obs]**. In modes 6/7 the player is snapped to the target with offset (−0.0212, 0.0849, 0.0281) and yaw = target yaw + 0.0494 **[obs]**. |
| 0x15 (21) | – / `8027FDC4` | whiff `pl0030_0226` (ported) |
| 0x16 (22) | `80280044` / `80280244` | **Hold / carry.** A side: exit to idle when the target is gone (`80209718`) or dead (`vfunc 0x23C`) (`+0x12D8 = 0`, `+0x11CC = 1`). Not on the floor → `802C32CC` (fall states 0xF/0x23). Mode 2: **A released → 0x17** and the target gets `vfunc 0x9C` (endCatch). Otherwise `802C44A8` handles hold actions (§1.5). B side: phase 0 plays `pl0030_0221` (hold idle), keeps the yaw target `+0x3B8 = facing`, sets the camera as in 0x14. Every frame in mode 2: **stick not pushed (`+0x11AC == 0`) → loop `pl0030_0221`; pushed with stick-Y `+0xF20 > 0` → `pl0030_0227` (walk forward); `+0xF20 ≤ 0` → `pl0030_0228` (walk backward)**, all through `802716EC` at blend `+0x1148` and rate `+0x11B8 × +0x114C`. Global flag 0x1A is set while walking. **Steering**: `+0x3B8 = wrap(+0x3B8 − 0.069813 × stickX(+0xF1C))` per frame (4°/frame at full deflection), then `802C71DC(0.3)` turns the facing toward it. Translation is the clips' root motion **[obs; `+0x11AC` = stick pushed past threshold, written by `802C704C`, inf]**. |
| 0x17 (23) | – / `802805A0` | **Release.** `pl0030_0222` (mode 2), camera mode 0, hit part 9, toggles `+0x1128 ^= 0x40` (alternates enemy-first / object-first search on the next grab). On motion end `+0x11CC = 1` → idle **[obs]**. |
| 0x28 (40) | `80287AC4` / `80287DF0` | **Attack while holding** (throws, head-butt), §1.5 |
| 0x18 / 0x19 / 0x26 / 0x1a–0x1b | `80280D00` / `80280FCC` / `80280ABC` / `80281794` | down-throw, chainsaw skewer, mad-buster, object pick-up / carry (follow-ups, not detailed) |

How long he can hold: there is **no player-side timer**. Jack holds while A stays held. The hold ends when A is released, the target dies or vanishes, Jack leaves the floor, or the grunt struggles free (§1.7) **[obs]**.

### 1.4 Where the held grunt sits

The grunt is **not attached to a bone** for catch types 1–6/0xc. `cEmZako::moveCatchedPos(rate)` `81050520` places him every frame from `catched_start` / `catched_idle` **[obs]**:

- Target point = Jack's world matrix (`owner+4`) × local offset **(0, 0, z) × 0.01**, with z = **55.35 → 0.5535 m in front of Jack** for type 3. Types 1/2/0xc use 60 → 0.60 m, types 4–6 use 120 → 1.20 m.
- `pos += rate × (target − pos)` on x, y, z.
- Yaw: face Jack (`81065EC0`). When `rate < 1` the turn is limited (`800BB634`, 0.4 × `+0x89C` per frame, cap π/2) **[obs]**.
- Rate: `catched_start` uses 0.3. `catched_idle` (type 3) eases `+0x59C` from 0.1 toward 1.1 by 20 % per frame, capped at 1.0. So the grunt snaps onto the offset within a few frames and then follows rigidly **[obs]**.
- `catched_idle` type 3 also shrinks the grunt's Havok proxy toward `0.5·CharColHeight − 0.1`, lerped at 0.2 × (1.3·CharColRadius − current) per frame (`800B319C`) **[obs; field meaning inf]**.
- Other catch types (0xe–0x3b: finishers, object combos) use paired offsets read from the player's `.mot` through `GetThrowOffset` `8007F058(mot, pos, rot, 0x400)`. The grunt is placed at Jack × offset with yaw = Jack yaw − rot.y (+π for most) **[obs]**.

Port: during the hold, set `grunt.pos = jack.pos + rot(jack.yaw) · (0, 0, 0.5535)` and `grunt.yaw = yaw toward Jack`, eased as above. Paired clips: Jack `pl0030_0221/0227/0228` with grunt `em0110_1072` (or `em0110_1077` while struggling).

### 1.5 Hold actions (`fn_802C44A8`, state 0x16, mode 2)

These are accepted only on the floor (`+0x396`) with `+0x1198 == 0` **[obs]**:

| input | internal id `+0x1284` | ATHIT sent to the grunt (table `0x805690F8`) | Jack clip | hit part | notes |
|---|---|---|---|---|---|
| remote swing \|accelY\| > 2.0, or holding an object | 0x36 | **0x4B WHANG_THROW** (背負い投げ, shoulder throw) | `pl0030_0224` | 9 | release |
| remote swing \|accelX\| > 2.0 only | 0x37 | **0x4C WHANG_SIDE_THROW** (sideways) | `pl0030_022b` | 9 | release |
| nunchuk shake `+0xF64` > 1.2 | 0x32 | **0x48 HANG_HEADBUT** (掴みからの頭突き) | `pl0030_0223` (+`_00.seq`) | 0x18 | keeps holding |
| (unused) | 0x38 | 0x4D HHANG_THROW | `pl0030_0000` | 9 | – |

Throw global flag 0x19 and head-butt flag 0x25 are set (tutorial/stat **[inf]**).

State 0x28 (`80287DF0`):
- Phase 0: `target->vfunc 0x98(ATHIT)` = `cEmZako::startCatchAtk`. Motion via `802C15B0(id)`. Hit param `+0x128C` = 50 for ids that `802C6BB4` lists as odd (0x36/0x37), else 10.
- For **0x36–0x38 (throws)** the player drops the target pointer at once: `+0x12D8 = 0`, camera mode 0 blend 0.3, `+0x1240 = 0`, `+0x115C = 180`.
- Cue `op 7` triggers `reserveAtkHit(radius 0.7, part, ATHIT, +0x128C)`. On motion end: throws → idle; head-butt with the target still held → back to **0x16**.
- A side (`80287AC4`): at the cue combo window (`+0x1282`, cue type 11) a throw may cancel into dodge / jump / attack / walk / run (`802C2610`, `802C2E68`, `802C3B5C`, `802C2434`, `802C2528`, `802C68AC`). Releasing A during a head-butt → release 0x17 **[obs]**.

Cue frames **[obs, `motion-cues.json`]**:

| clip | events |
|---|---|
| `pl0030_0224_00.seq` | type 3 at frame 15, combo window (11) at **41** |
| `pl0030_0223_00.seq` | hit (7) at **12**, combo window at 15 |
| `pl0030_0224_10.seq` / `022b_10.seq` / `0223_10.seq` | sound/effect cues only |

Damage rows, `plem_common.ini [damage_pl]` (power, hit-stop, Havok…, points on damage/kill) **[obs]**:

| row | power | points |
|---|---|---|
| WHANG_THROW / WHANG_SIDE_THROW | 30 | 1000 / 10000 |
| HANG_HEADBUT | 25 | 500 / 10000, hit-mark part 3 |
| DOWN_THROW | 40 | |
| WALL_CRASH | 20 | |
| BOUND_THROW | 40 | |
| MADBUSTER | 60 | |
| OBJ_THROW | 20 | |

Points on kill also come from `em_common.ini [room]`: トゲ壁 5000, トゲ壁投げ 10000 …

PC suggestion (port choice, not native): vertical throw = the existing vertical-swing key, side throw = the horizontal-swing key (F/R directional strikes in `native_controls.rs`), head-butt = the shake key (X).

### 1.6 Enemy side: catch states (state table `0x811614a0` / `0x81161710`)

| id | state | br / update |
|---|---|---|
| 0x5c | catched_start | `810517b0` / `810517b4` |
| 0x5d | catched_idle | `81052910` / `81052970` |
| 0x5e | catched_atk_start (in-hold hits) | `8105310c` / `81053110` |
| 0x5f | catched_end | `81055094` / `81055098` |
| 0x60 | catched_shake_off | `810555cc` / `810555d0` |
| 0x61 | catched_counter | `810556ec` / `810556f0` |
| 0x45 | motion_blow (thrown) | `81049dac` / `81049db0` |
| 0x46 | motion_stuck (impaled on a BG object) | `8104bbe8` / `8104bc08` |
| 0x09 | stuck_spike_wall | `8102cdd8` / `8102cddc` |

`isCatchedNow` `810279ec` = state byte `+0x11B` in 0x5c..0x60.

**`startCatch(type, owner)` `8101b2b8`** **[obs]**:
1. Returns at once if the grunt is dead, or already in 0x5c..0x5e with the same type.
2. Sets `setCatched` `8101b23c`: `+0x165C` = owner, `+0x1650` = type, `+0x395 = 1`, flag `0x11A0` bit 0x10.
3. For type 3: `chkEnableCatched` `81028dec` must pass. It reads the 掴まれ通常 / 掴まれピヨリ chance at `prop+0x106` (100 % for em0101). If it fails, the grunt **counters** (state 0x61 `catched_counter`: `em0110_1075`, releases, may attack).
4. Otherwise `resetGroupAttacker`, state **0x5c**, and the hold timer `+0x1158 = getCatchedCancelTime()`, or 30000 when flag `0x1214` bit 2 / cfg 0xd is set.

**catched_start** (type 3): clip **0x888 = `em0110_1071`**, or **0x83b = `em0110_1076`** when already struggling (flag `0x1214` bit 6). Blend 3, rate `+0x898`. `moveCatchedPos(0.3)` each frame. On motion end: `+0x1154 = +0x1158`, `+0x1158 = −1`, → **0x5d**.

**catched_idle** (type 3) **[obs]**:
- Clip **0x889 = `em0110_1072`** (held loop), or **0x83c = `em0110_1077`** (struggle loop), swapped when flag `0x1214` bit 6 changes.
- Each frame it checks that the owner is still holding it (`isNowCatchingEnm` `810530e4` and `owner+0x1260 == this`). If not → `setCatched(0)`, clear the struggle flag, → wait 4.
- `br_catched_idle` exits to wait when flag `0x11A0` bit 0x10 drops. Otherwise it runs `chkCatchedCancelTime` (§1.7).

**catched_end** (player released, `endCatch` `8101bdc4` → 0x5f): clip **0x88a = `em0110_1073`**. Then, if the grunt can take the group attacker token → siege 0x7a, else wait 4. It goes to `chkKill`/`chkEnd` if dead **[obs]**.

**HANG_HEADBUT** (`startCatchAtk(0x48)` → 0x5e): the grunt plays **0x88f = `em0110_1079`**, or 0x88e = `em0110_1074` if it was struggling. Damage comes from Jack's own cue hit (§1.5). On motion end, if not dead and `vfunc 0x1CC` (still caught) → **back to 0x5c**, i.e. the hold continues **[obs]**.

**Throws** (`startCatchAtk` `8101b9e4`): it sends msg 0x100C to the group and stores `+0x1654` = ATHIT. If `chkFlickThrow` `810226b8` is true → counter 0x61. That needs 投げ抵抗 (prop flag 5) and a struggling grunt for 0x4b/0x4c/0x53; em0101 has 投げ抵抗 commented out, so throws always go through. For 0x4b/0x4c: `+0x141C` = blow type = ATHIT, `+0x177C = 1` → **motion_blow 0x45**. 0x4f (DOWN_THROW) only clears flag 0x10 (motion_blow is entered from the down-grab path). Anything else → 0x5e **[obs]**.

### 1.7 How long the grunt can be held (掴まれ体力 / 掴まれもがき体力)

`em_common.ini [em0101]` **[obs data]**:

```
掴まれ体力       = 99,6, 70,8, 0,10   ; HP≤x% → can be held y s (3 stages; last = fainted; stage-1 HP=0 → grab only when stunned)
掴まれもがき体力 = 0, 5               ; HP≥x% → breaks free after y s (struggle-on-grab setting)
掴まれ通常/ピヨリ = 100 / 100         ; grab chance %
掴まれ時間通常   = 0.5, 0.6            ; [unres consumer]
他掴まれ攻撃頻度 = 1                    ; attack rate of the others while one is held
```

Code **[obs]**:
- `getCatchedCancelTime` `81029198`: tests the u16 pairs at `prop+0xF2`, then `prop+0xEE`. If HP/HPmax ≤ 0.01·hp → time = value × 30 frames. If no pair matches → −1, clamped to **0**.
- `getCatchedStruggleTime` `810290b4`: if not `81027d80`/`81061888` and HP/HPmax ≥ 0.01·`prop+0xE2` → `prop+0xE4`, else −1.
- `chkCatchedCancelTime` `810527c8`, every frame in catched_idle for types 1–6, only when not `8100d768`, lost-part bits `0x1738 & 0x600 != 0x600`, `& 0x18 == 0`, and `isGuardEnableCondition(1)`. Once the timer `+0x1154` (counted down by the generic timer code) is ≤ 0:
  - already struggling (flag `0x1214` bit 6) → clear it, **state 0x60 catched_shake_off** (type 3 clip **0x835 = `em0110_1075`**, then wait 4), and `sendReleaseCatch` tells the owner (`vfunc 0xA4`). The grunt **escapes**.
  - not struggling and struggle time ≥ 0 → `+0x1154 = +0x1158 =` struggle time, set the struggle flag, recover piyori / faint, **re-enter 0x5c** (plays `em0110_1076`, then loop `em0110_1077`).

The pair layout is **[inf]**: the three 掴まれ体力 pairs sit at `prop+0xEE/0xF2/0xF6`. This follows the key packing (ダウン体力 ends at `+0xED`, 掴まれもがき at `+0xE2`). The code reads only the first two pairs, checking (70 %, 8 s) first and then (99 %, 6 s). The third (fainted) pair is not read here.

Result for an em0101 street grunt:

| HP | before he starts struggling | struggle, then breaks free |
|---|---|---|
| 100 % (untouched) | 0 frames: struggles at once | 5 → escape |
| ≤ 99 % | **6 s = 180 frames** | 5 → escape |
| ≤ 70 % | **8 s = 240 frames** | 5 → escape |

The struggle "5" is read **without** the ×30 the cancel time gets. Whether the ini parser already stores it in frames is **[unres]**. The ini comment says seconds, so use **5 s = 150 frames** until a capture says otherwise. While the grunt struggles, Jack keeps holding and can still throw or head-butt. A head-butt sends him back into catched_start but does not reset `+0x1154` **[obs]**.

### 1.8 Thrown: flight, landing, impacts (`motion_blow` `81049db0`)

Phase 0, set-up per blow type `+0x141C` (word `0x507`) **[obs]**:

| blow | placement | clips (start → air loop `+0x57C` → landing `+0x580`) | extra |
|---|---|---|---|
| **0x4B WHANG_THROW** | `setThrowOffset(π, Jack, "pl0030_0224.mot")` `8101b8cc`: the grunt is placed at Jack × the offset stored in Jack's throw clip (`GetThrowOffset(…, 0x400)`), yaw = Jack yaw − rot.y + π | **0x8fd `em0110_100c`** → **0x8fe `em0110_100d`** → **0x8ff `em0110_100e`** (start becomes 0x8fc `em0110_1016` in rooms 0x302/0xd08 with flag 0xD, or 0x900 `em0110_100f` with flags 0xE/0x10 or room 0x103) | `+0x5A4 = 17` **[unres use]** |
| **0x4C WHANG_SIDE_THROW** | catch type set to 0x33, `moveCatchedPos(0.1)` with the `GetThrowOffset("pl0030_022b.mot")` offset | **0x901 `em0110_1013`** → **0x902 `em0110_1014`** → **0x903 `em0110_1015`** | |
| 0x4F DOWN_THROW | `setThrowOffset(0, Jack, "pl0030_0240.mot")`, re-applied while `+0x12C9` > 0 | 0x8fb `em0110_10ae` → 0x8fe → 0x8ff | |
| 0x4E BOUND_THROW | `setThrowOffset(π, "pl0030_063a.mot")` | 0x904 `em0110_1085` → 0x8f3 `em0110_101a` → 0x90f `em0110_1095` | |

Common set-up for all blow types: `+0x598 = 1` (owns damage), `+0x584++` (land-damage pending), drop the attacker token, motion speed `81065FF0(rate × prop+0x68)`.

Flight (phases 1–3):
- The arc is the **authored root motion** of the start clip (`getBlowMotDir` `810499f8`, applied by the motion system), plus an optional extra velocity `+0x1254` that decays by `fRam81123728` per frame. That velocity is 0 for these throws **[inf]**. No ballistic constants are applied to the throw itself.
- When the start clip ends or the grunt leaves the floor → air-loop clip (phase 2/3) until `+0x396` (on floor) → landing clip (phase 4/5) **[obs]**.
- **Background finishers during flight** (`chkBgFinish` `810153a4`, radius **0.7** around the root, skipped for ids with `prop` flag 2 or `+0x58C != 0`):
  - rigid body attribute **0x40000** → `setCatched(0)`, `setAttachRigidBody` `81026478`, state **0x80** (dead_fan path). Bodies 0x70535/0x70514/0x70527/0x70503/0x70547 are excluded.
  - body kind 7 or attribute **0x4000** (spikes / sign) → impale: record the stab point and normal, `cancelCatchedAttack`, state **0x46 motion_stuck**. Room-gated bodies 0x70145–0x70147 add points/messages.
  - Blow types 0x4e/0x4f/0x52/0x55 with `+0x598 = 1` add game point 0xC (`addPoint`).
- **Hitting other grunts** **[inf]**: when prop flag 3 is set and the body speed |`+0x8B0`| > 0.05/frame, `vfunc 0x430(1.0, dir, …, 0x15B BLOW, 10, 1)` registers a BLOW attack along the flight direction, re-armed every 15 frames (`+0x5AC`). `[damage_em] BLOW = 60 power, radius 3.0`. Treat it as "a flying grunt knocks down grunts it passes through".
- **Walls** (phase 4/5, `getHitWallRb` `81063db0`, radii 0.7/0.2/1.2): on a wall hit it sends `msg::OnHitWall` `800a8ad4` to the wall object, sets `+0x86C |= 0x20`, and adds points for the bound-type throws **[obs]**.

Landing (phase 4) **[obs]**:
- Plays the landing clip, yaw += `+0x12D4`, `vfunc 0x158(1)`.
- If land damage is pending: `+0x584 = 2`, flag `0x1184` bit 0x18. When that count reaches 0: **`setOwnDamage(blow type, …)` `8104e6ac`**, i.e. the grunt takes the throw's own `[damage_pl]` row (WHANG_THROW 30) **[inf: damage value]**.
- On motion end: dead → dead 0x7d; else `8104d600(down state)` (down idle → stand-up, as in `ZAKO-AI.md` §4) or wait.

Effect on the other grunts **[obs]**:
- msg **8**: a grunt held in phase 3 makes every grunt in the group use 他掴まれ攻撃頻度 (**1 %/frame**) instead of 攻撃頻度 2 % (`chkChgAttack`, `ZAKO-AI.md` §5.1).
- msg **0xb** (the siege attacker holds back) is about a grunt catching the player, not this feature.
- `startCatchAtk` sends msg 0x100C, which clears `0x468` bit 0x1a and sets `+0x13F8 = 0x127` on the receivers **[obs; effect unres]**.

### 1.9 Related follow-ups (noted, not specified)

- **Spike wall push** (`stuck_spike_wall` 0x09, clips 0x944–0x948, which em0101 does not have, so they fall back to `em0110_1000`). Timer `rand(5, 5.5) × 30`, then pull-off. It is reached from WALL_CRASH 0x54 or object hits **[inf]**.
- **Object carry / pick-up** (player 0x1A/0x1B, `cCatchScrObj::takeObj` `801d0488`): signs, tires, `ev0012 bl0109_0000`.
- **Down throw** (player 0x18): `pl0030_0240`, DOWN_THROW 40, enemy `em0110_10ae`.
- **Mad Buster** (player 0x26): `pl0030_110d`, MADBUSTER 60.
- **Chainsaw skewer** (player 0x19): `pl0030_110a/b/c`, repeated 0x4e hits, released when the target dies, `+0x1230` (2 loops) runs out, or `+0x1088` < 1.
- **Giant swing**: RinRin only in the port today; zako type 0xb = `em0110_1050/1051/1053/1052` (already in `PL0000-COMBAT.md`).

### 1.10 Port checklist (mw-sim / play_street)

1. Input: an A hold that outlives the 8-frame press buffer, with no punch fired, triggers one grab attempt per hold.
2. Target: nearest grunt within 2.5 m, ±80° of facing, |Δy| ≤ 0.3, clear line at 0.9 m height. The grunt must be standing (catch type 3: not attacking, jumping, down or stunned).
3. Jack: 0x14 `pl0030_0220` → 0x16 hold (0221/0227/0228 by stick Y, steer 4°/frame × stick X) → 0x17 `pl0030_0222` on A release.
4. Grunt: 0x5c `em0110_1071` → 0x5d `em0110_1072`, kept 0.5535 m in front of Jack, facing him. Struggle timer per §1.7 (`em0110_1076/1077`), escape via `em0110_1075`.
5. Actions: shake → head-butt (`0223` / `em0110_1079`, 25 dmg at frame 12, hold continues). Vertical swing → `0224` + grunt `em0110_100c/d/e` placed by the `0224` throw offset. Horizontal swing → `022b` + `em0110_1013/14/15`. 30 damage on landing. Cancel window at frame 41.
6. Flight: root motion only. Impale on spike / sign bodies within 0.7 m, BLOW knock-down to grunts on the path. Other grunts drop to a 1 % attack rate while one is held.
7. Open: the `GetThrowOffset` binary layout inside `.mot` (track 0x400) has to be decoded to place the thrown grunt exactly (`8007f058`) **[unres]**.

---

## Part 2: in-game radar (cIdRadar)

### 2.1 Classes and programs

| class | vtable | update | role |
|---|---|---|---|
| cIdRadar | `805B1444` | `80224E80` (stop/free `80224E0C`) | the radar |
| cIdRadarPoint | `805B1474` | `8022480C` (set `8022487C`) | one dot (20 pooled at `+0x7C`) |
| cIdRadarPointNew | `805B14A4` | `80224B1C` (set `802249E4`) | "new marker" pulse |

RTTI strings are at `805638C6` / `805638CF` / `805638DD` **[obs]**. Programs are created through `8024B81C(0x8069d49c, &handle, 0, resource, program, layer, flags)`. Resource 0 = **core_us** (`etc/core_us.idd`, staged `assets/native/ui/core_us/`). Resource 1 = **the room's idpac** (for the streets `case2/idpac_c201_us.dat`, staged `assets/native/ui/idpac_c201_us/`) **[obs calls; resource meaning inf, consistent with the staged data]**.

| handle | program | layer | widgets (`tools`: scratch `idtdump.py`) |
|---|---|---|---|
| `+0x4` root | **core_us 36** | 9 | w0 id1 root at layout **(−238, 110)**, so the centre is (82, 290) on 640×360. w1 id0 82×82 (UV group 25 #1). w2 **id51** 82×82 (25 #2; handle `+0x4C`, shown/hidden). w3 **id50 kind 9, 80×80, UV 250 = the map-texture viewport** (handle `+0x48`). w4 id0 82×82 (25 #0, frame) |
| `+0x1C` | core_us 35 | 0xC | 82×82 overlay at the same place (25 #0, alpha 0 at rest) **[inf: flash/frame]** |
| `+0x10` | **core_us 18** | 0xC | **player arrow** 14×14 (25 #6), at the radar centre (handle `+0x40` = child 1) |
| `+0x18` | **core_us 56** | 0xB | **objective pointer** 30×30 (UV group 9 #24), child 2 |
| dots | **core_us 4** | 0xB (PointNew: 0x11, flags 3) | 10×10 dot (25 #3) |
| `+0x14` | **room program 50** | 0xA | only if the room resource has it (`802259E8`). Widget **30 = floor/map data**. Widgets 50–59 = per-floor overlay icons (none in c201) |

Sprite rectangles (core_us `layout.json`, groups 25 and 9):
- 25 #0 page 4 (97,5)-(173,81)
- 25 #1 page 4 (5,5)-(87,87)
- 25 #2 page 2 8×8
- 25 #3 page 2 (1009,281) 10×10 (dot; #7–#11 are dot variants)
- 25 #6 page 1 (185,851) 14×14 (arrow)
- 9 #24 page 3 (89,55)-(121,87)

### 2.2 What is drawn (`80224E80`, every frame) **[obs]**

**Room map texture** (widget 50):
- The floor index comes from room widget 30: current / base / count bytes at `+0x40/+0x42/+0x44`, height thresholds at `+0x50…`. Floors step down when player.y < threshold[floor] and up when player.y > threshold[floor+1].
- The texture bound is `room IMT set (iRam8069e260)[floor]` (`800D00C8`). In room 0x103 with flag 0xC, floor 1 shows as 2.
- For c201: span 20, pixels 280, origin (0, 20), thresholds 3.3 / 9.5, 3 floors. This is exactly `assets/native/ui/c201-map/map.json` with pages `page-0..2.png` from `case2/c201.dat#c201.imt`, already used by the pause map.
- **Translation**: (player.x − origin_x, ·, player.z + origin_z) / (10·pixels/span = 140), i.e. texture-fraction units at 2 px per metre (`800C5878`).
- **Scale**: `10·(+0x2C · w30[+0x88]) / span`, with +0x2C = 1.5 and +0x88 = 350 for c201.
- **Rotation** `+0xA8` = **57.29578 × camera yaw** (degrees). The map turns with the camera, so the radar's up is the camera's forward.
- Visibility: `800CECCC(…, 4|0, 1)` from byte 0. Byte 0 is set by `disp(bool)` `80225F68` and cleared while the game-state test `8008B5E4(0x80697e78, 7)`, stage flags 5/6 or demo mode is true.

**Player arrow** (program 18): fixed at the radar centre, rotation **180 − 57.29578·playerYaw + 57.29578·cameraYaw** degrees. It is visible with byte 0.

**Dots** — `addPoint(radar, pos, colour, type)` `80225A48`, called each frame by the owners. The radar holds up to 20 dots per frame (`+0x1D8`). Skipped in room 0x106 or when hidden.
- d = horizontal distance from player to point. The offset is (player.x − p.x, p.z − player.z).
- **type 3 (enemy)**, except in room 0x404:
  - culled when d·1.5 > **37 px**.
  - alpha = 1 − (d·1.5 − 30)/5 above **30 px**.
- **other types**: clamped to **27 m** (normalised, alpha 0.75). They stay on the rim as direction hints.
- px = offset × **1.5** (`+0x2C`; `80225990` can override radius / scale per frame).
- Rotated by camera yaw (`802F12A4` cos / `802F16B0` sin of `8007FD74` camera yaw), then added to the arrow's position.
- Colour / variant index: the caller's `colour`; type 8 forces 2. A point outside the current floor's height band → 1. In room 0x202 a different-floor point is hidden, and under flag 8 in other rooms too (except rooms 0x301/0x403/0x503).
- The variant picks 16 bytes from the dot widget's table (`802249B4`, `widget+0x34`). Index 0 with type ≠ 3 overrides it with RGB (0.9, 0.7, 0) **[obs; table contents unres]**.
- **Types 7/9/10/11** are also stored as markers (`+0xCC` positions, `+0x1BC` types). A new type 7/10 marker spawns a pulse (`80225FF4` → cIdRadarPointNew, program 56). The first type 9/11 marker drives the **objective pointer** (program 56 at the clamped 27 m rim position, shown only while such a marker exists).

### 2.3 Who feeds it **[obs]**

| caller | function | type / colour | what |
|---|---|---|---|
| player update `fn_8027621C` → `801DEBE0` (`cEmObjList`, calls at `80277B2C` / `80278460`) | every enemy with `vfunc 0x194` true | type **3**, colour **0** if `vfunc 0x2B8` true else **3** | **enemy dots**, position from `8020FE08` (actor centre) **[inf]** |
| `8020A89C` (item cObj update) | `+0x1D6` set | type **10** for categories 0x40500/0x40501/0x40510, else **7** | pickups / weapons **[inf]** |
| `80170F50` | – | type 8 | gimmick object **[inf]** |
| `8012E714` | event object with `+0x78` | type 9 | objective (pointer) |
| `8012B980` | stage flag 0x1E | type 11 | fixed objective point (data at `0x80541CF0`) |
| player update `80277198`, `801A4940` | `disp(radar, on)` | – | visibility |

Player yaw is `player+0x74` and position `player+0x64` (`802163AC(0x8069bca8, 0)` = player 0). Camera yaw is `8007FD74(0x8069bd80)`.

### 2.4 Port notes (mw-game `play_hud.rs`)

- Use the existing `UiSource::load(root, "core_us")` scenes for programs **36, 18, 4, 56** (35 optional). Draw widget 50 of program 36 with an override image, the way `play_pause.rs` overrides widget 100 with `StageMap` pages: same `StageMap::layer(y)` floor choice, UV window centred on the player. Rotate the map quad by camera yaw.
- Feed: the player position and yaw from `play.fight.player`, the camera yaw from `native_camera`, and every live street grunt from `play_street` (type 3, colour 3, cull above 37 px, fade from 30 px, 1.5 px/m). There are no items yet.
- The kind-9 widget (UV 250) clip/stencil behaviour is not translated in `native_ui::sprites` (only kinds 0/8 draw today). Clip the map to the 80×80 circle in the port **[unres native clip shape]**.
- Open:
  - The exact map scale (w30 `+0x88` = 350 gives 262.5 against the 210 px the 1.5 px/m dot scale implies) **[unres]**. Match the dot scale until a capture of the original radar is taken.
  - The dot colour table **[unres]**.
