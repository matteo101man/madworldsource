# Street grunt (em0101 kung fu, cEm0101 : cEmZako) native AI

Behaviour spec for porting the native street-grunt AI to `crates/mw-game/src/play_street.rs`. Every claim cites the enemy-RSO analysis address it comes from (relocated base, see `enemy-link.json`; bodies via `tools/pl0000/decompile.sh`, kept outside the repo) and is tagged **[obs]** = observed in decompiled code / data, **[inf]** = inferred from names, call shape or data layout, **[unres]** = not resolved. Distances are metres, angles radians unless noted, times are frames at 30 fps (`x * 30` in code), "per frame" means the check runs every branch tick.

Offset notation: `this+0x…` is a byte offset. Bare flag words (`0x45d`, `0x45e`, `0x468`, `0x5cb`) and the state-local timers `0x167`/`0x168`/`0x169` are **4-byte word indices** as the decompiler prints them (`param_1[n]`, byte offset = 4n: `0x45d` = `this+0x1174`, `0x45e` = `+0x1178`, `0x468` = `+0x11a0`, `0x5cb` = `+0x172c`, `0x167` = `+0x59c`). Every timer below counts by the frame step `this+0x898`, which is 1.0 per game frame (fixed step, see `PL0000-COMBAT.md`, `SIXTY-FPS.md`) **[obs]**. The global frame clock used for "N frames ago" checks is the float at `0x8069bcf4` **[inf]**.

## 0. Sources and tables

| What | Where | Notes |
|---|---|---|
| State table (branch / update) | `0x811614a0` (156 `br_*` fn ptrs), `0x81161710` (156 update fn ptrs) in `tools/enemy-analysis.bin` | Plain 4-byte pointer arrays; index = state id passed to `_chgState` (`8105b730`). Each subclass has its own copy (cEm0101 copy at `0x81163540`). Slots 0x3e/0x3f are swapped between the two arrays (br: angry, blow_ragdoll; update: blow_ragdoll, angry). Both `br_` entries are 4-byte stubs, so it changes nothing **[obs]**. moveMain `81015f60` dispatches them: `br_` is the per-frame transition check, the update fn runs the state **[inf]**. |
| Sub-state | byte `this+0x11c`. siege_target uses it as a sub-state machine via `_chgSiegeState` `81033cd4`: the low 2 bits are the phase (0 = enter, 1 = run) | **[obs]** |
| Property block `stEmProperty` (0x148 bytes) | `this+0x860` = `param_1[0x218]`. Loaded by `cEmObjList::getProperty` `801e6678` from records parsed out of `DATA/files/em_common.ini` (`em_common_hard.ini` on Hard) by the DOL key parser `fn_801E4090` (key table `0x8055ea7c`, 114 Shift-JIS keys) | Key→offset map below was obtained from the parser's write offsets minus the 0xc record header. It was cross-checked against the consumers (`+0x96` attack rate in `chkChgAttack`, `+0xbf` laugh rate in `br_wait`, `+0x84/85` combo in `getRandCombo` `8102af48`) **[obs]** |
| Motion id → clip | `cEm0101::getModelData` `8106eb98` registers 3 tables via `cEmMotTblList::setMotInfoTbl` `801dab90`: T0 = 0xc9 entries, T1 = 0x92, T2 = 0x14c | `cEmMot::getMotionData` `8100b00c`: table = `(id>>10)&7`, slot = `id&0x3ff`. So ids 0x000–0x3ff → T0, 0x400–0x7ff → T1, 0x800–0xbff → T2 **[obs]**. Unused slots point at `em0000.dat\em0110_1000.mot` **[obs]** |
| Attack info table `stEmAttackInfo` (0x24 bytes/record) | `cEm0101::onInitialize` `81071694`: `setAttackInfoTbl(group 1, 0x81162e68, 10)` → attack ids 0x100–0x109; `setAttackInfoTbl(group 6, 0x81162fd0, 6)` → 0x600–0x605. Flags word +0x20 is patched at runtime by the same function | Record layout from `_decideAttackSub` `8102a7f0`, `chkEnableAttackDirLen` `8102a6e4`, `chkEnableAttackHp` `8102a664`, `registAttack` `8103cd80`: +0 ATHIT_ATK_TYPE, +4 u16 motion id, +8 lost-part mask vs `this+0x1738`, +0xc s16 power, +0x10 range² (× `prop+0x2c`), +0x14/+0x18 yaw window min/max, +0x1c same-attack repeat limit, +0x1d weight byte, +0x1e/+0x1f HP% window, +0x20 cFlag (bit n = `1<<n`, `8100f740`) **[obs]** |

### 0.1 em0101 property values (em_common.ini `[em0101]`, normal difficulty)

| Key (offset) | em0101 | em0101B (torch) | Hard em0101 | Consumer |
|---|---|---|---|---|
| Hp (`+0x9e`) | 120 | 200 | | |
| 攻撃頻度 near-attack rate %/frame (`+0x96`) | 2 | 100 | | `chkChgAttack` `810289e4`, siege 0x1c |
| 遠攻撃頻度 far-attack rate %/frame (`+0x97`) | 5 | 100 | | siege 0x1c |
| 他掴まれ攻撃頻度 rate while a fellow is grabbed (`+0x98`) | 1 | 100 | | when `sendMsgEm(8)` ≠ 0 |
| 攻撃力 attack multiplier (`+0x74`) | 0.5 | 1.5 | | `stEmProperty::getAtkPower` `81008450` (`power*mult>>8`, so the value is stored ×256 **[inf]**) |
| コンボ数 combo min,max (`+0x84/85`) | 1 | | | `getRandCombo` (used only if >1) |
| 回避数 evade budget (`+0xa2` → `this+0x4da`) | 1 | 255 | 2 | `chkGuardDefault` `8101ced8`. Any evade **or guard** spends one (255 = unlimited). With 0 left, `chkGuardDefault` returns 10 at once, so em0101 dodges or guards **once per life** (§4.1) **[obs]** |
| 回避回復 refill after n hits taken (`+0x90`) | none (`;回避回復 = 2` is commented out → default 255 = never) | | | `onPreMove` `8100d95c`: counter `this+0x1366` +1 per hit taken, budget +1 when it reaches `+0x90` **[obs]** |
| 回避体力 (hp%,chance%)×2 (`+0xc6..`) | 100,10, 50,10 | 100,30, 50,30 | 100,50, 50,50 | `chkGuardDefault` |
| チェーンソー回避 % (`+0xb0`) | 30 | 100 | 40 | `chkGuardDefault` |
| チェーンソー防御 (`+0xb1`) | 0 | | | `isGuardEnable` |
| 防御通常 / 防御ピヨリ guard % (`+0x112/113`) | 5 / 0 | 10 | | `chkGuard` `81028e9c` |
| 防御時間通常 guard time s (`+0x114/116`) | 1.5–2.0 | | | `stEmProperty_ActInfo::getTime` `81029260` = `rand(u16 +0x114, u16 +0x116)`, read once on entering face_guard into `this+0x5ac` and counted down by the frame step, so the values are frames: **45–60** **[obs code; inf: the parser stores s × 30]** |
| 反撃通常 / ピヨリ counter % (`+0x10c/10d`), 反撃ウェイト (`+0x72`) | 100 / 0, 0.1 s | | | `chkCounter` `81028f44` is called only from `moveMain` reserve bits 0xb/0xf, and only for chainsaw-armed 0x20220/0x20141. 反撃ウェイト (s16) is read only by the counter state 0x2d (`81045c1c`), which needs `chkGuardDefault` code 9 (never for em0101). **Neither affects em0101** **[obs: load scan of `+0x72`]** |
| ダウン体力 (hp%, s)×2 (`+0xe6..`) | 100,0, 40,0 | | | `getDownTime` `81028fdc` |
| ピヨリ体力 (hp%, s)×2 (`+0xce..`) | 70,5.0, 30,5.0 | 70,5, 40,5 | | `getPiyoriTime` `8102937c` |
| 吹っ飛び体力 blow-away hp% (`+0xda`) | 20 | | | `decideDamageProc` `81022dec` |
| 浮かし体力 launch hp% (`+0xde`) | 90 | | 80 | `getComboDamageMotion` `810177c4`: the punch→chainsaw combo launch 0x919 becomes 0x91f while HP% > 90 (§4.3) **[obs]** |
| 気絶体力 faint hp% (`+0xd6`) | 40 | | | **[unres]** |
| 怖がり確率 / 怖逃げ確率 fear / fear-flee % (`+0xbe/+0xbd`) | 90 / 80 | 0 | | `onRecvMsg` `810595dc` (0x1007/0x1011, `rand(0..99) <= 90`) / `fear_to_escape` `8102d21c` sub 1 (`rand(0..99) < 80`), §5.1 |
| ラッシュ回数 rush count (`+0xa3`) | default 5 (ctor) | | | attack `8103cf8c`: `this+0x17a2 = rand(1..+0xa3)` for 0x109 **[obs]**, loop use **[inf]** |
| 回復 HP regeneration (prop disable flag 3) | default on (no key in `[em0101]`) | | | `moveMain` `81015f60`, §5.2 **[inf: key→flag]** |
| 笑い確率 laugh % (`+0xbf`) | 100 | 0 | | `br_wait`, siege |
| 攻撃補助人数 / 攻撃補助確率 (`+0x99/+0xc0`) | 0 / 0 | 2 / 100 | | siege 0x10 |
| 回避率 (`+0x92`) | default 100 **[inf]** | | | siege back-off rolls |
| 包囲戻り距離 (`+0x54`) | default 5.0 (ini header) | | | siege 0x24 |
| 知覚距離 / 見失距離 (`+0x18`, `+0x1c/20`) | 20 / 40 (**12 / 30 in c201**, c201em.ini) | | | 0x1009 "come here" range, sensing |
| 同ガード回数 / ガード回数 (`+0xa1/+0xa0`) | 2 / 2 (ini note: "hits until the guard is knocked away") | | 1 / – | `refHitPos` `8105870c` (same-direction count `this+0x143a`, guards left `this+0x143e`), refilled from `+0xa0` by `onInit`, every hit taken (`onPreMove`) and face_guard's exits (§4.1.2) **[obs]** |
| ガード回数後 action after the guard count (`+0x8`) | 威嚇 (threaten; ini note "現在無効" = disabled) | | | face_guard writes `prop+8 = 10` before every exit, so the exit is always code 10 → catched_counter 0x61 **[obs]** |
| 掴まれ体力 / 掴まれもがき体力 | 99,6, 70,8, 0,10 / 0,5 | | | grab (player side) |

Room level: `攻撃者上限 = 1` ("enemies attacking the player at once", default 1) in `[room]` of both `em_common.ini` and `case2/c201em.ini` **[obs data]**. Its consumer is not traced, but the group token in §2 behaves that way **[inf]**.

**Property defaults without an ini key** **[obs]**: new records are built by the list-node ctor `801e8548` → `801ed124` (DOL, constants at `0x8055fcc0`). It sets `prop+0x24` (siege ring radius) = **2.5 + rand(0, 1.0) m**, rolled once per record (so all em0101 grunts share one value per boot), and `prop+0x2c` (attack-range scale) = **1.0**. Same ctor: `+0x18/+0x1c` = 20, `+0x20` = 9999, `+0x28` = 0.5, `+0x30` = 225 (15 m squared, 挑発反応遠距離), `+0x54` = 5.0 (包囲戻り距離), `+0x68` = 1.0 (MotBlowRate), `+0xa3` = 5. `fn_801E5FE8` is a different parser (key table `0x8055ecdc`) and `fn_801E3CAC` only inserts/looks up records **[obs]**.

**Street variant** **[obs data, field offsets inf]**: `cEmZako::onInit` `8100b958` reads the spawn record: Type = byte `+0x2f` (→ `this+0x17a0`, the `_01` ini suffix) and B = byte `+0x4f` bit 0, and calls `getProperty(prop, id, Type, B)` (`8100be8c`). B is also passed to `cEnemyBase::createModel` `810621e4`, which stores it in `this+0x915`. In `c201.dat#c20100.ems` (the streets' set) all 26 spawns are id 0x101 with Type 0 and B 0, so the street uses **plain `[em0101]`, variant A** (claws, no fire breath). The other c201 sets mix Types 1/2/4/5 and B=1 (for example `c20107`/`c20108`), which matches the `_01`…`_05` and `em0101B` sections. `0x915 != 0` marks variant B: it gates the fire-breath attacks (§3) and `cEm0101::onInitialize` raises rank `0x928/0x92c` to 0x10 for it **[obs]**.

**Room overrides** `case2/c201em.ini [em0101]` **[obs data]**: `CharColRadius = 0.40`, `知覚距離 = 12` (perception, replaces 20) and `見失距離 = 30` (lose sight, replaces 40). Nothing else changes for Type 0.

## 1. States a street grunt uses

State ids (decimal/hex) from the table; only the combat-relevant ones:

| id | state | br / update |
|---|---|---|
| 0x04 | wait | `8102b818` / `8102be2c` |
| 0x1c | attack | `8103cd60` / `8103cf8c` |
| 0x3e | angry | `81047058` / `8104705c` |
| 0x41 | damage | `81047204` / `81047208` |
| 0x45 | motion_blow (launched / blown away) | `81049dac` / `81049db0` |
| 0x47 | damage_chgatk | `8104c4b0` / `8104c4f0` |
| 0x48 / 0x49 | damage_stand_up / _neckspring | `8104c8d8`,`8104c928` / `8104cac8`,`8104cb18` |
| 0x4d / 0x4f | damage_down_idle / down_back_idle | `8104d97c`,`8104d99c` / `8104de1c`,`8104de3c` |
| 0x4e | damage_down_pursue (hit while down) | `8104dd10` / `8104dd14` |
| 0x5a | damage_down_hit | `81043e7c` / `81043e80` |
| 0x64 | piyori (stun) | `81056334` / `8105637c` |
| 0x71 | provoke_reaction | `8105d290` / `8105d36c` |
| 0x72 | wander | `8102defc` / `8102e058` |
| 0x74 | target_found | `8102e7b8` / `8102e814` |
| 0x77 | threat_aggressive | `8102eb60` / `8102ebe4` |
| 0x7a | siege_target (the main combat loop) | `8102ff38` / `81030300` |
| 0x7c | laugh | `81034138` / `810341e0` |
| 0x7d | dead | `81034404` / `81034408` |
| 0x85 | tired | `810365c8` / `81036608` |
| 0x97 | fear | `8103a9fc` / `8103aa1c` |
| 0x0a | fear_to_escape | `8102d1dc` / `8102d21c` |
| 0x0f / 0x19 | walk_one / run_one (forward dodge) | – / `8103c694`,`8103c6e0` |
| 0x15 / 0x16 | r / l_side_walk (lateral dodge, walk variant) | `8103be50` / `8103becc`,`8103bfbc` |
| 0x1a / 0x1b | r / l_side_run (lateral dodge) | `8103c7fc` / `8103c848`,`8103c9ec` |
| 0x20 / 0x23 | move_back / bk_jump (backward dodge; bk_jump is also the evade jump) | `8104052c` / `810405a8`; stub `81040b38` / `81040b3c` |
| 0x2a / 0x2b | face_guard (the guard) / body_guard | stub `81045000` / `81045020`; body_guard is two 4-byte stubs `81044ff8`/`81044ffc` |
| 0x2d | counter (unused by em0101, §4.1.3) | stub `81045c18` / `81045c1c` |
| 0x35 / 0x36 | r / l_rolljump (side evade jump) | stub `81041364` / `81041368`; stub `810414b4` / `810414b8` |
| 0x61 | catched_counter (the guard's exit move) | stub `810556ec` / `810556f0` |
| 0x76 | threat_battle (after an evade without a counter) | – / `8102ea30` |
| 0x9b | formation | stub `8102ff30`/`8102ff34` (empty for cEmZako) **[obs]** |

### 1.1 Normal-combat flow

```
spawn → wait(4) ──SeekTarget──▶ chgStateTargetFound ──▶ target_found(0x74) ──anim end──▶ [br_first_sense forced attack?] ──▶ wait(4)
wait(4) ── in a group / besieging ──▶ siege_target(0x7a)  ◀──── loop: approach slot, threaten, side-step, back off,
                                          │  (attacker token)          approach-to-attack, attack
                                          └──▶ attack(0x1c) ──motion end──▶ wait(4) ──▶ siege_target …
any ── hit ──▶ decideDamageProc ──▶ damage(0x41) / motion_blow(0x45) / piyori(0x64) / dead(0x7d) …
damage/blow ──▶ down_idle(0x4d|0x4f) ──timer──▶ stand_up(0x48) | neckspring(0x49) ──▶ wait(4)
target HP == 0 ──▶ laugh(0x7c) ──▶ wait(4)
```

### 1.2 wait branch `br_wait` `8102b818` (evaluated top-down each frame, first hit wins) [obs]

1. `chkKill` `81040148`, `checkFall` `8101f224` → handled there.
2. Faint (bell) / faint away → 0x14 / 0x66 / 0x68 / 0x4d.
3. Tired (`isTired` or reserve flag) → `tired` 0x85.
4. Piyori → 0x64. `chkForceWait` → demo_wait 0x90. Reserve flag 7 → angry 0x3e.
5. `chkEscape(1.0)` `8103e9f4`, `chkEventAction`. `isAngry` → provoke_reaction 0x71. This is the taunt path: taunt → target_found → `setAngry` → here (§5.1) **[obs]**.
6. Scripted/object targets (bills, food, cover, explode, move_destination 0x9a) – not street.
7. **In a group or besieging → siege_target 0x7a** (line `0x6579`).
8. Otherwise, with a target: `chkJoinGroup(0,1)` `81027e98` (joins a group and goes to 0x7a), `chkEvade` (vfunc 0x490 = `810296dc`), `chkRest` `8103e858`, `chkCommand` `8103ff50`. If target HP == 0 and `笑い確率 != 0` → **laugh** 0x7c. Else `chkChgAttack(this,1,1,0,1)`.
9. No target: wander 0x72 (if allowed) or `SeekTarget` → `chgStateTargetFound` `81026374` → target_found 0x74 for em0101.

`wait` update `8102be2c` picks an idle loop: `getWaitMotion` `81017ad8` → unarmed returns 0 = `em0101_0000`, or random [0x183/0x184 table] when variant **[obs]**. It turns toward the target at 1 rad/frame max when armed with a target (`_turnDir` `8101e128`) **[obs]**.

### 1.3 target_found `8102e814` [obs]
It plays motion 0x25 = `em0110_0036` (the "spotted" reaction) and keeps facing the target. On motion end (or seq flag 0x45d bit5) it calls `br_first_sense` `81026138`. With sense type 7/8/9 (how the target was noticed) em0101 forces attack 0x104 if it is a fire kung-fu (`isFireKungfu` `8101db38`), else 0x604 or 0x605. A sense type of 10 goes to state 0x25 (fw_v_jump). The state then optionally starts `setAngry` (13–16 s) and returns to wait.

### 1.4 threat_aggressive / threat_battle [obs]
`br_threat_aggressive` `8102eb60`: becomes siege_target as soon as `chkGroupAttacker(0,0xff)` succeeds. `threat_battle` `8102ea30` plays one of 3–4 threat motions (table `0x81123454`, plus 0x39 = `em0110_0042` when no lost-part bits `0x180` are set) and returns to wait.

## 2. Positioning around the player (siege_target `81030300`)

### 2.1 Groups, slots and the attacker token [obs]
- Grunts join a group (`chkJoinGroup` `81027e98`, `cEmObjList::addGroupAuto` `801e1e28` / `addGroupInLeader` `801e1f98`). If that fails they join the free besiege ring (`chkJoinBesiege` `810280e8` → `addBesiegePlayer` `801e1760` and `refBesiegeAngle` `801e18f8`; the angle is stored at `this+0x964`).
- **Slot position** `calcSiegePos` `81013a90`:
  - In a group: slot index = `getNoInGroup` (the current attacker uses its act number). Angle = table `0x81123104` = {0°, +38°, −38°, +76°, −76°, +114°, −114°, +152°, −152°} indexed `slot % 9`, measured in the **target's camera frame**: slot 0 is between the player and the camera. Radius = `this+0x1790`, eased each frame toward `prop+0x24` (2.5–3.5 m, §0.1) at 3 % per frame (`81030300`). Height offset 0.25. The slot is pulled in front of walls by `ckLineAttr_Obs`/`cHavok::checkLine` (offset `0.1 + CharColRadius`).
  - Besieging without a group: a fixed 8.0 m ring at the besiege angle, slerped 0.08 per frame.
- **Who may attack**: `chkGroupAttacker` `810282e8`. A grunt can take the token only when the target is a player. The group's act number (`getActNoInGroup` `801e2690`) must equal its own slot, or it must be top rank in the group. When it isn't already in a group, it must be the enemy nearest the target inside 10 m in camera space (`getNearEm` with the camera matrix) and in front of the camera (`chkTargetPosZinCamera(0.3)`). It then calls `setActNoInGroup` `801e263c`. The token rotates by `incActNoInGroup` `801e2824` (act number +1, wraps at group size). It is released by `resetGroupAttacker` `810285ec` on piyori, down, blow and death (`chkPiyori`, `down_idle`, `motion_blow`, `moveMain`). `moveMain` also drops it every frame when the holder is off screen (`isBoundingBoxIn`/`isDisp`, vfuncs 0x214/0x218) or `chkTargetPosZinCamera(0.3)` `810281f4` is true, i.e. the grunt's camera-space depth exceeds the target's by more than 0.3 m (it stands beyond the player as seen from the camera) **[obs]**. In `chkChgAttack` only `isAttacker` / `isAttackSupport` grunts may start an attack on a player. With room `攻撃者上限=1` the result is **one attacker at a time, the others hold the ring** **[inf for the cap value; obs for the gate]**.
- Attack-support: a non-attacker may become a support attacker (flag `this+0x3c0 |= 0x100000`) when the group has fewer than `攻撃補助人数` supporters, an attacker exists and `rand<攻撃補助確率`. For em0101 both are 0, so this never happens **[obs data]**.

### 2.2 Siege sub-states (byte `0x11c & 0xfc`) [obs unless marked]

| sub | meaning | motion(s) | exits |
|---|---|---|---|
| 0x00 | enter: compute the slot, reset timers. If it must turn first → 0x20 | turn R/L 0x14/0x15 (`em0110_0013`) | → 0x10 |
| 0x10 | init idle-at-slot. With dist² > 25 and `rand<45%` → threat wait (`getThreatWaitMotion` `81018320`: 0x39/0x3a/0x3b = `em0110_0042/0043/0044`), else `getWaitMotion`. Non-attackers get a hold timer `0x167` = 15–30 frames (0.5–1 s); the attacker gets 0 | as left | → 0x11 |
| 0x11 | **hold / threaten at slot**. Target HP 0 and laugh% → laugh 0x7c. **Attacker**: with attack count `0x4a0` < 9999 → **0x1c**. **Non-attacker**: with `0x4a0` < 1 → 0x1c once, else → **0xb8** (back off), and on a 1-in-3 roll with `rand<回避率` while facing (dot>0.17) → **0xc4**. If more than `包囲戻り距離` from the slot (or the slot is invalid) → **0x24**. Within 1.5 m of the target and `rand<回避率` → 0xb8 (+0xc4 roll as above). Idle reshuffle: once the hold timer is over, the loop motion exceeds 3 s (`0x169 > 90`) and a 1/16 roll passes: more than 1.5 m from the slot → side-walk 0x88 (R) / 0x8c (L) or forward 0xb0 / back 0xb4 toward the slot (by its local x/z). Otherwise it re-picks an idle: target farther than 4.5 m → threat waits 0x3a/0x3b (90 % if besieging), else unarmed waits 0x59/0x5d (= `em0101_0000`). Near the slot (<0.6 m) side-walk clips are added. A side-walk has a 24 % chance instead of a wait | waits/threats | see left |
| 0x1c | **approach to attack** (attacker). Stop distance `d = max(1.2, (radius+0.4)×1.7)` for the attacker (2.2 for support); em0101 variant with vfunc 0x2c4 true uses 3.5. **Far window** dist² in (12.25, 25), i.e. 3.5–5 m: per frame `rand<遠攻撃頻度` (5 %), facing within 15°, clear line (`ckLineAttr_Obs`) → forced dash attack: variant A **0x604** (or 0x109 when flag `0x462` bit0 is set), variant B **0x104** (fire breath). Inside 4.5·d it walks (`getWalkMotion` 0x0e = `em0101_0010`), beyond that it runs (0x13 = `em0110_0012`). **Near** (dist < d): per frame `rand<攻撃頻度` (2 %; 他掴まれ 1 % while someone is grabbed) → `chkChgAttack(1,!support,1,0)` → attack 0x1c. Otherwise it keeps a threat-wait loop | walk/run/threat | attack / 0x10 |
| 0x20 | turn in place (1.3 rad/s blend) | turn R/L | → saved sub |
| 0x24 | return to slot: run when >6 m (dist² > 36), walk otherwise. Strafes R/L (run R/L 0x16/0x17 = `em0110_002b/002c`) when the slot is 1.5–6 m off to the side. Re-picks every 0.9–1.1 s | run/walk | → 0x10 when <2 m (dist² < 4) |
| 0x30 | support-attacker lineup | | → 0x10 |
| 0x88/0x8c | side-walk R/L toward the slot (`getWalkR/LMotion` 0x10/0x11 = `em0101_0011/0012`) | | → 0x10 at the slot |
| 0x90/0x94 | side-jump R/L (ninja types only) | | |
| 0xb0 | walk forward to the slot | 0x0e | → 0x10 |
| 0xb4 / 0xb8 | **walk backward** (`getWalkBkMotion` 0x0f = `em0110_0011`) | | → 0x10 |
| 0xc0/0xc4 | **back-jump** (`getBkJumpMotion` 0x1a = `em0101_0121`) | | → 0x10 |
| 0xc8/0xcc | step/run variants | | → 0x1c / 0x10 |
| 0xff | re-init; with another attacker or interesting enemy within 6 m and a "laughing-stock" enemy found (`getNearLaughingstockEm(6)` `801e022c`) → laugh 0x7c | | |

Each frame: target gone → leave besiege, remove from the group, back to wait. `chkGroupAttacker(0,0x7a)` is re-run (`br_siege_target` `8102ff38`). `chkJump(30)` `8103ece0`, `chkEscape`, `chkEvade` (vfunc 0x490) can preempt the sub-states **[obs]**.

## 3. Attack choice and hit

### 3.1 em0101 attack records (`0x81162e68`, `0x81162fd0`; flags after `onInitialize`) [obs data]

| id | motion | clip | atk type | power | range (√) | yaw | in random pool? |
|---|---|---|---|---|---|---|---|
| 0x100–0x102 | 0x401/0x403/0x407 | (em0110_1000 fill) | 0x15e/0x160/0x168 | 14/14/10 | 1.5/1.5/1.1 | ±15° | no (flag bit0) |
| 0x103, 0x104 | 0x404 | **em0101_0110** (fire breath) | 0x193 | 14 | 3.5 | ±30° | yes, **variant B only** (bit11 ⇒ requires `0x915==1`) |
| 0x105 | 0x47c | **em0101_0104** | 0x184 | 14 | 2.2 | ±15° | yes, **variant A only** (bit10 ⇒ `0x915==0`) |
| 0x106 | 0x47d | **em0101_0105** | 0x185 | 14 | 2.8 | ±20° | yes, variant A only |
| 0x107 | 0x47e | **em0101_0106** | 0x186 | 14 | 3.5 | ±30° | no: combo follow-up only |
| 0x108 | 0x47f | (em0110_1000 fill) | 0x175 | 14 | 2.2 | ±15° | no |
| 0x109 | 0x488 | **em0101_0102** | 0x15e | 14 | 4.5 | ±15° | no: forced far dash (siege 0x1c) |
| 0x600–0x603, 0x605 | 0x40d/0x411/0x413/0x40f/0x405 | fill / em0101_0000 | 0x12d.. | 10–15 | 1–3.5 | ±15° | no |
| 0x604 | 0x414 | **em0110_010d** (jump/dash attack) | 0x134 | 14 | 4.5 | ±15° | yes in principle (flags bits 1,2,9), but bit 2 limits it to while running (`0x45d` bit 3, set by run_one / side_run), so in practice it comes from the forced siege dash |

All records have HP window 0–100 %, repeat limit 1 (byte +0x1c) and lost-part mask 0x78 (punches/kicks: disabled when arm/leg-loss bits 3–6 of `this+0x1738` are set) or 0xa80 (fire breath, the 0x10x fillers: head bits). The mask meaning comes from `getWaitMotion` using bits 3/4 for lost-arm waits **[inf]**. The motions 0x487 (`em0101_0101`) and 0x489 (`em0101_0103`) belong to attack 0x109. The attack state `8103cf8c` plays 0x487 as a lead-in, then the record motion 0x488 (`em0101_0102`), then 0x489 as the follow-through. It also sets the rush count `this+0x17a2 = rand(1..ラッシュ回数)` (default 5) **[obs]**. The count looks like the number of lead-in loops **[inf]**.

**Attack-info flag bits** (record `+0x20`, read through `cFlag::ck` `8102af80`; all readers are in `_decideAttackSub` `8102a7f0` and attack `8103cf8c`) **[obs unless marked]**:

| bit | meaning |
|---|---|
| 0 | not in the random pool (forced or combo use only) |
| 1 | no reader in the enemy RSO: no effect for cEmZako **[obs: scan of all `8102af80` call sites]** |
| 2 | needs `0x45d` bit 3 = "currently running" |
| 3 | adds `2·|approach|` to the score when the approach term `param_4 < −0.2` |
| 4 | selects the `param_7` pool: with `param_7 == 0` the record must have bit 4 clear, otherwise bit 4 set and prop flag 2 clear **[inf: long-range/alternate pool]** |
| 7 | counter-only: needs `param_9` or `0x45e` bit 0xc (armed by `chkGuardDefault` after an evade, cleared in `chkCounterAttack` `81040850` / counter) |
| 8 | the attacker keeps turning toward the target during the attack (same as `0x468` bit 0xf) |
| 9 | the attack motion is started with `_setMotion` option 2 (6th arg). The blow clips use the same option for mirrored versions **[inf: mirror/alt]** |
| 10 / 11 | variant A only (`0x915 == 0`) / variant B only (`0x915 == 1`) |
| 12 | must match the actor's cfg flag `this+0x868` bit 0x19 (set ⇔ set) **[unres: meaning of 0x868 bit 0x19]** |
| 13 / 14 | 13 = usable by a support attacker too, 14 = support only. A non-support grunt skips bit-14 records, and a support grunt needs 13 or 14 |
| 15 | must match "holding an object" (`this+0x1274 != 0`) |
| 16 | skipped when vfunc 0x1d4 returns true **[unres]** |

### 3.2 Decision [obs]
- `chkChgAttack` `810289e4` (from wait, and from siege 0x1c with its own gate): the per-frame chance applies only when `param_4==0`. It is `攻撃頻度` (or `他掴まれ…` while a fellow is grabbed) +50 if angry, +50 if the player's state byte is 'H' (player state 0x48 = 72, the charge attack, §5.1), compared with `rand(1..99)`. The function rejects: player mid-move (`+0x1158/115c` > 0), grunt behind the target's local z<0 (unless angry or in support), and grunts that are neither attacker nor support. A catch attempt (`chkStartCatch`) is tried first. Then `_decideAttack` `8102a3c0` → `_decideAttackSub` `8102a7f0`, and the result goes to state attack 0x1c (or catch_target_attack 0x6f for 0xb0x ids).
- `_decideAttackSub` scans the groups in the order {0xa00, 0x700, 0x100, 0x000, 0x800, 0x900, 0xc00, 0x600, 0x500, 0xb00} (list `0x811233d4`). For each record it skips: flag bit0, lost-part mask, the HP window, a same-as-last attack already used `+0x1c` times (`0x4a7`/`0x57e`), range/yaw out (`chkEnableAttackDirLen`: dist² ≤ range²·`prop+0x2c`, yaw in [min,max]), and the variant bits 10/11. Score = `0.05·power + 0.2·dist²/range² + rand(0,0.2)` (+`3·weight` with `param_8`, +`2·|v|` with flag bit3 when the approach term < −0.2). It picks the max. With the em0101 table this is effectively a random pick between 0x105 and 0x106 (variant A) when both are in range, weighted toward the farther-reaching one when the player sits near the edge **[inf from formula]**.
- **Combo** `chkComboContinue` `8103cb5c` (runs as attack's branch): starting 0x105 sets combo max `rand(1..3)`; starting 0x106 sets `rand(1..2)` (`setComboMax` `81022410`, 0x20101 branch). While the seq combo-window flag (`0x45d` bit5) is set and count `0x17a1` < max, 0x105 → **0x106** → **0x107** (0104 → 0105 → 0106), each re-checked for range and yaw and started with `forceAttack` `8102275c`.
- **Evade counter**: `chkGuardDefault` evades (code 3) against chainsaw-type hits and arms counter attack `0x57f` = 0x105, or 0x104 for fire kung-fu (§4) **[obs]**. It is executed by `chkCounterAttack` `81040850` at the end of bk_jump / rolljump (§4.1.3), not by `_counterAttack` `8102afac` (whose only caller is `appear` `81026a08`) **[obs: bl scan]**.

### 3.3 Attack state `8103cf8c` and hit application [obs]
Phase 2 plays the record's motion (`+4`) at speed 1, stores `0x4a5` = atk type, bumps `0x4a0` (attack count used by siege) and `0x17a1` (combo count). It then tells the target the first SEQ event frame (target vfunc 0xac with `jkMotionRig::getFirstEventFrame`). That is the player's "incoming attack" warning **[inf]**. Every frame until the hit is registered, `registAttack` `8103cd80` → `setAtkHitParam` `8102114c` (atk type, power = record +0xc). The hit itself is applied when the clip's SEQ attack window is open, i.e. the type 19/20 events already in `motion-cues.json` **[inf]**. Damage to the player = `power × 攻撃力` = 14 × 0.5 = **7** for every em0101 attack (`getAtkPower` `81008450`) **[inf: ×256 fixed point]**. On motion end → wait (`0x94c3`), and siege resumes. `chkTired` runs after each attack; em0101 sets no 疲れ攻撃回数, so it never tires **[obs data]**.

Fire breath extra: `cEm0101::onAtkHit` `81072058`. When atk type 0x193 hits a player it sends msg 0x101c with a random 66–90 frame duration (2.2–3.0 s × 30) **[obs]**, read as "set the player on fire" **[inf]**.

## 4. Guarding, evasion and damage reactions

### 4.1 Pre-damage reaction: `startAtk` → `chkGuardDefault` (evade / guard) [obs unless marked]

**Trigger.** The player announces each attack to **its current target only**. In state 39 (punch/chainsaw string), state 72 (charge) and the other attack states it calls the target's vfunc 0xac = `cEmZako::startAtk(type, attacker, t)` `8101c134`, where `t` is the first event frame of the attack clip (`802CBD54` → `getFirstEventFrame`, PL0000-COMBAT.md). Other grunts never react to that swing. A scan for `lwz rX, 0xac(rY)` + `mtctr` in the player code finds such calls in B39 (6), B41–B48 (weapons), B72–B77 (charges), B28, B35, B43, B57, B80, B90 and the swing check `802c3c0c`, but none in the throw states B24/B25/B32/B38 or the taunt B84 **[obs scan; that every hit is a startAtk call, inf]**. `startAtk`:
1. stores the type in `this+0x13f8` and sets `this+0x13f4 = 0` ("an attack was just announced"; `moveMain` counts it up and resets it to −1 after 60 frames). It sets `0x468` bit 0x1a and sends msg 0x100c to all zako.
2. bails out if dead, if `isGuardEnableCondition(0)` `8101af00` fails (faint, piyori, tired, berserk, grabbed…), or if a reaction is already pending (`this+0x1438 ≥ 0` or rsv flag 1).
3. skips the types 0x11e–0x121 (bike), 0xd2, 0x5f FINISH_BLOW, 0x5b SENPUKYAKU, 0x55 GIANTSWING and 0x169. Those can never be dodged or guarded.
4. needs the attacker within **6.16 m** (12.25 m when the player holds arm object 0x30760).
5. needs the grunt in one of these states (byte `+0x11b`): wait 4 (or 0 / 6), r/l_side_walk 0x15/0x16, move_back 0x20, damage 0x41, 0x42 (rig flag 0x1a), damage_sever 0x43, target_found 0x74, threat_battle 0x76 or siege 0x7a. Attack 0x1c is allowed only outside the hit phase: not while `0x45d` bit 1 is set, and with bit 0 set only outside 20–75 % of the clip. While `0x468` bit 10 (airborne after a hit) is set, the result is always 10.
6. calls `chkGuardDefault(type, attacker)` and turns its code into a reserve bit of `0x5cb` (§5.2): 3 → bit 4 (bk_jump), 5 → bit 7 or 8 at 50/50 (r/l_rolljump), 4 → `initGuardInfo` `81045004` + bit 5 (face_guard; not if already in 0x2a/0x2b), 1/2 → bit 2 or 3 at random, 7 → 0xb, 8 → 0xc, 9 → 0xe (counter). The reserve delay is `this+0x1730 = max(t − 3, 0)` frames. The reaction therefore fires 3 frames before the announced hit frame, or at once for 1100–1102, whose hit cues are at frames 3–5.

#### 4.1.1 Decision `chkGuardDefault` `8101ced8` [obs]
It returns 10 = take the hit, 3/5 = evade, 4 = guard:
- The evade budget `this+0x4da` (= 回避数, 1 for em0101) must be non-zero, otherwise it returns 10 at once. Any result other than 10 spends one (unless the budget is 255), the guard included. em0101 has no 回避回復, so a street grunt **evades or guards once per life** **[obs]**.
- Normal hits (not the chainsaw-cut types 0x2a,0x2c,0x2e,0x30–0x36,0xcc–0xce): evade on the 回避体力 rolls (HP ≤ 100 % → 10 %, the 2nd pair is checked first). em0101 (0x20101) picks 3 or 5 at 50/50. It sets `0x45e` bit 0xc but leaves the counter id `0x57f` at −1, so the evade ends in threat_battle, not in an attack (§4.1.3).
- Chainsaw-cut types: `rand < チェーンソー回避` (30 %) → evade 3 with counter `0x57f` = 0x105 (0x104 for fire kung-fu). These types are never guarded.
- Guard (4) is tried when no evade was chosen, or over an evade 3. `chkGuard` `81028e9c` must pass (`rand(0..99) < 防御通常` = **5 %**, never with `0x1738 & 0x600 == 0x600`) and `isGuardEnable(type)` (vfunc 0x9d0) must return true. When an evade 3 was already chosen, チェーンソー防御 = 0 together with `0x461` bit 0x1c cancels the guard. On a guard the grunt faces the attacker (`calcDir`).
- `cEm0101::isGuardEnable` `81071e88` returns "guardable" and sets `*out = 1` for the chainsaw class:
  - **Always guardable**: 0x5e RUSH_PUNCH, 0x6d–0x70 PUNCH_A0–A3, 0x75 PUNCH_B0, 0x99–0x9c KUKRI_A0–A3, 0xa1–0xa4 KUKRI_B0–B3.
  - **Chainsaw class** 0x76–0x78 PUNCH_B1–B3, 0x81 PUNCH_COUNTERATK, 0x96 BAT_CHGATK: guardable only when チェーンソー防御 ≠ 0 (0 for em0101). Otherwise `*out = 1`, which is a guard break (§4.1.2).
  - Variant A only: 0x30, 0x35, 0x150–0x157, 0x15a.
  - Variant A with rank `0x928 > 0x1f` only (never on the street): 0x2a–0x2f, 0x32, 0x3b–0x44, 0xca–0xcb.
  - Never: 0x31, 0x33, 0x34, 0x36, 0x5f–0x6c (FINISH_BLOW, guns), 0x71–0x74 (PUNCH_A4–A7) and every other type **[obs]**.
  - The base `cEmZako::isGuardEnable` `8101d970` instead requires an arm guard (`isArmGuard` `8101db88`) for chainsaw types.

#### 4.1.2 Guard state face_guard 0x2a (`81045020`) [obs unless marked]
- **Blocking starts with the reservation.** `isReserveGuard` `810594d8` is true while reserve bit 5/6 is pending, while `0x1184` bit 8 is set, or in state 0x2a/0x2b. `refHitPos` `8105870c` runs per incoming hit (1 = damage). It blocks a hit when `isReserveGuard` holds, `0x461` bit 0x1c is clear and `isGuardEnable` passes.
- **Guard limits** (`refHitPos`): "vertical" = type in {0x2a, 0x30, 0x32, 0x35, 0xca, 0xcc}, so every punch is "horizontal". `0x143a` = same-direction count, `0x143b` = last direction, `0x1436` = hits blocked in this guard, `0x143e` = guards left (= ガード回数 2). `0x143e` is refilled on `onInit`, on every hit taken (`onPreMove`) and on the guard's exits. A hit is blocked only while `0x143e > 0`. A direction change before 同ガード回数 (2) same-direction blocks also fails. So against Jack's punch string the grunt **blocks two hits and the third lands** (the ini's ガード弾き).
- **A blocked hit** adds its damage to `this+0x1434` (no HP loss) and flags the cAtHit as guarded (`+2 = 1`). The grunt faces the attacker, sets `0x468` bits 0x13/0x14, calls `setEfcBloodColor` and spawns **parts effect 0xc** (`CreatePartsEff`, the guard spark) on itself. Then `0x1436 += 1`, `0x143e −= 1`, and the hit does no damage.
- **Guard break by type**: a chainsaw-class type (the `*out = 1` case above) on a guarding grunt sets `0x45e` bit 0xb (and `0x461` bit 0x1c). It **rewrites the incoming type to 0xef GUARD_BREAK** (damage clip 0x91f = `em0110_10a4`, §4.3) and the hit lands. An armed grunt with a breakable weapon loses it (`breakWeapon` `8101f844`).
- **Clips** (unarmed, not holding an object): sub 0 plays 0x2a (vertical type) or 0x2b, both `em0110_1063` (12 frames), at blend 1. It sets the guard timer `this+0x5ac = getTime(prop+0x112)` = 45–60 frames (§0.1). Sub 0x10: once the clip passes frame 2, `setNullSpeedRate(0, 1)` turns root motion off and the state goes to sub 0x11. Sub 0x11 counts the timer down. A blocked hit plays no new clip for an unarmed grunt (the guard-hit clips 0x5e–0x6d are for armed types).
- **Exits**, checked every frame:
  - timer out → code 10.
  - the player stops attacking or moves off: `0x13f4` < 0 or > 25 frames since the last `startAtk`, or dist² to the target `this+0x894` > 12.25 (3.5 m). If no guards are left (`0x143e < 1`): refill and code 10. Otherwise **→ wait 4** (`0xa413`).
  - code 10 → reserve bit 0xd → **catched_counter 0x61** (`810556f0`); `0x1184` bit 0 is set and the hit mask cleared (`801d9bdc`). For em0101, catched_counter faces the target, sets `0x461` bit 0 and turns its hit volume off (no damage during the move). It plays **0x835 = `em0110_1075`** (41 frames, root ≈0.8 m backwards) at blend 3. It picks no attack (`0x15f` stays −1 for 0x20101) and → wait at the clip end. The guard therefore ends in a hop back, not a strike **[obs; "hop back" from the root track, inf]**.
- ガード回数後 (`prop+8`) is forced to 10 before every exit, so the ini's 威嚇 is never used.
- **What the player sees**: the grunt turns to Jack in the `em0110_1063` guard pose 0–3 frames before the punch lands. Up to two punches spark (parts effect 0xc) without a flinch, and the third hits normally. After 1.5–2 s, or when Jack stops for 25 frames or steps beyond 3.5 m, the grunt hops back (`em0110_1075`) or returns to wait. The enemy side requests no guard sound **[unres: the player-side consumer of the guarded cAtHit (+2) was not traced]**.

#### 4.1.3 Evade jumps and the evade counter [obs]
- Code 3 → reserve bit 4 → `chkEvadeJumpBack(0)` `8103e734` → **bk_jump 0x23** (`81040b3c`). It plays `getBkJumpMotion` 0x1a = `em0101_0121` (16 frames, root **2.3 m backwards**) at blend 5. On entry it sets `0x1184` bit 0, clears the hit mask (`801d9bdc`) and sets game-obj flag 2. `refHitPos` returns 0 (no damage) for any hit in states 0x23/0x26/0x27 and sets `0x468` bit 0x13. The jump ends at the clip end or at the combo window (`0x45d` bit 5).
- Code 5 → reserve bit 7 or 8 → `chkEvadeJumpRight/Left(1, 0)` (`8103e494`/`8103e5e4`) → **r_rolljump 0x35** if `isEnableMoveR(1.0, 0.4)` passes, else l_rolljump 0x36 (mirrored for Left). For bits 7/8 `moveMain` sets `0x461` bit 0 and clears the hit mask. The state (`81041368`) plays `getRotJumpR/LMotion` 0x2e = `em0110_0022` / 0x2f = `em0110_0023` at blend 1 and turns toward the target at 0.2 rad/frame. Its exits are the same as bk_jump's.
- Both end in `chkCounterAttack` `81040850`. If `0x45e` bit 0xc is set, it clears the bit and (when not faint or piyori) runs `forceAttack(0x57f, 1, 1, 1)` when a counter id is armed: **0x105 = the claw `em0101_0104`, after a chainsaw-cut evade**. Without a counter id it goes to **threat_battle 0x76** (unarmed, prop flag 2 clear), which plays a threat motion and returns to wait (§1.4). If it returns 0, the grunt goes to wait.
- The counter state 0x2d (`81045c1c`, reads 反撃ウェイト) and `chkCounter` (反撃通常) are not reached by em0101 (§0.1).

### 4.2 Damage routing `decideDamageProc` `81022dec` (by incoming type `this+0x141c`) [obs]
- HP ≤ 0 or killing types → `setDead` `8101dc98` + dead 0x7d (various dismember paths: dmNeckCut, deadWaistCut, partsSeparate…). Gory kills call `sendFearPos(9.5 m)` `81015184` (msg 0x1011 to nearby grunts).
- **Blow-type hits** (0x77, 0x78, 0x7d, 0x8a, 0x125 …): blown away (motion_blow 0x45) only if HP% ≤ 吹っ飛び体力 (20 %), otherwise **plain damage 0x41**. So a full-HP em0101 is not launched by these **[obs]**.
- The HP gate covers only 0x77, 0x78, 0x7d, 0x8a and 0x125 (`81024f14`/`81024f34`; 0x78/0x7d/0x8a/0x125 also face the attacker). 0x7b/0x7c PUNCH_CHGATK/CHGATK2 skip it and **always launch** (motion_blow 0x45), as do 0x97 BAT_DASHATK (facing the attacker), 0x6b GRENADE and 0xf0 OIL_DRUM. A chainsaw B hit (0x75–0x78) with `0x45e` bit 0xb set (guard broken, §4.1.2) never launches. 0x5a → damage_down_pursue 0x4e. 0xa7 → blow_ragdoll. 0xa8 → damage_chgatk 0x47 (or damage_crouch 0x58). 0x68 = stun type, piyori 4–5 s for robot kung-fu only. Remaining normal types → damage 0x41 or damage_down_hit 0x5a when already down.

### 4.3 damage state `81047208` → motion [obs]
Motion = `getDamageMotion(type)` `810171c4`:
- Combo hits 0x8e–0x95 use `getComboDamageMotion` on combo log `+0xaf0`, 0x6d–0x70/0x75–0x78 on `+0xa78`/`+0xab4`, and 0x99–0x9c/0xa1–0xa4 on `+0xb2c`.
  - **Combo tree** (`getComboDamageMotion` `810177c4`, tables filled with constants by `cEmZako::onInit` `8100b958` at `8100bb38…`) **[obs]**. Each hit is classed P (punch 0x6d–0x70) or C (chainsaw 0x75–0x78) by `getComboDamageType` `81017988`. Its depth n = `getComboDamageNo` `8100f5f4` (0–3). The earlier hits of the current string come from the log `this+0x15e8` (written by `onPreMove`, cleared by `clearComboDmgLog` `8100d2a8` when the depth is 0). The tree is indexed by the history, oldest hit first. The `+0xa78` tree (rig flag 4 clear, the normal case):

    | depth | history → motion |
    |---|---|
    | 1st hit | P 0x915 `em0110_10a5` (19 f), C 0x857 `em0110_1026` (44 f) |
    | 2nd | PP 0x916 `10a6` (20 f), PC 0x919 `10a0` (77 f, launch; replaced by 0x91f `10a4` while HP > 浮かし体力 90 %), CP 0x802 `1000`, CC 0x857 `1026` |
    | 3rd | PPP 0x917 `10a7` (31 f), PPC 0x91f `10a4`, PCP 0x917, PCC 0x91f, CPP 0x802, CPC 0x857, CCP 0x857, CCC 0x8d9 `104d` (43 f, root 3.4 m back) |
    | 4th | PPPP 0x918 `10a8` (32 f), PPPC 0x91f, PPCP 0x802, PPCC 0x857, PCPP 0x918, PCPC 0x91f, PCCP 0x802, PCCC 0x857, CPPP 0x808, CPPC 0x857, CPCP 0x802, CPCC 0x857, CCPP 0x802, CCPC 0x857, CCCP none (0x1fff), CCCC 0x86f `em0110_1102` |

    With rig flag 4 set, the `+0xab4` tree gives 0x8f9 `em0110_1083` / 0x818 `em0110_1036` instead. Bat (`+0xaf0`) starts 0x80b `em0110_1004` / 0x916. The clip names are the em0101 table entries (`em0000.dat`).
- Otherwise a direct switch: 0→0x800 (`em0110_1008`), 1→0x801 (`1008`), 2→0x802 (`em0110_1000`), 3→0x803 (`em0110_1001`), 5→0x805 (`1001`), 7→0x807 (`1002`), 0xb→0x80b (`em0110_1004`), 0xd→0x80d (`1005`). Unknown types default to 0x803 (`em0110_1001`). 0x6d–0x70 → 0x915–0x918 or 0x8f9 (`em0110_1083`). 0x7d/0x8a/0x125 → 0x8f4 (`em0110_101b`). 0x7f → 0x800 (`em0110_1008`) at HP% ≤ 吹っ飛び体力 (20 %), else 0x91f (`em0110_10a4`). Many slots resolve to the fill clip `em0110_1000` in em0101's table.
- While stunned (flag `0xf0` bit 0x11) the light motions 0x802–0x80d are replaced by 0x91f.

The clip plays at speed 1. Root pushback comes from `setNullSpeedRate`, ×2.1 for types 0x13/0x15. The exit to down idle goes through `getDownProc` `810586b8`: 0x4d (face up) or 0x4f (face down, by a part's axis) **[obs]**.

### 4.4 Down and get-up [obs]
- `chgDownIdle` `8104d600` sets the down timer `this+0x1154 = getDownTime()` = `rand(t·30, t·30+30)` frames, where t comes from ダウン体力. For em0101 t=0 at every HP, so **0–30 frames (0–1 s)** before it may rise **[obs data]**.
- `down_idle` `8104d99c` loops motion 0x867 = `em0110_1038`, releases the attacker token, and blocks rising while faint or while flag `0x11a0` bit 0x1c is set. Then: HP > 60 % and not nearly dead → **neckspring 0x49**, else **stand_up 0x48**, after recovering piyori. A grunt whose lower body is lost bleeds out: dead after a 150–180 frame (5–6 s) timer.
- `damage_stand_up` `8104c928` plays 0x81a = `em0110_103d` (0x869 = `em0110_103a` when flag `0x864` bit6 is set) and → wait on end.
- `damage_stand_up_neckspring` `8104cb18` plays **0x8a2 = `em0110_1060`** (blend 5, speed = frame step) and → wait on motion end **[obs]**. If the actor has ragdoll physics enabled (cfg `this+0x870` bit 3) it first starts a ragdoll-to-animation mix (`startRagdollMix`, `addRagdoll`). The mix weight `this+0x59c`/`+0x64c` starts at 0 and is eased 5 % per frame toward 1 **[obs]**, read as "blend out of the ragdoll pose" **[inf]**. A port without ragdolls just plays the clip.

### 4.5 Piyori (stun) [obs]
`chkPiyori` `8101269c` (on damage). If not nearly dead and `getPiyoriTime` passes: stage i (counter `0x15e4`, max 2) fires once when HP% ≤ ピヨリ体力[i].hp, so **at ≤70 % and again at ≤30 %, 5.0 s each** (stored ×256). It sets the stun timer `0x888`, adds time if already stunned, releases the token and spawns the stun effect. The piyori state `8105637c` loops motion 0x83d = `em0110_1078`. `recoverPiyori` `81015d18` counts the timer down by the frame step and the state returns to wait when it expires (`br_piyori` `81056334`). Guard rate in piyori 0 %, counter 0 %, grab 100 %.

### 4.6 Death [obs]
- **Entry** (`decideDamageProc` `81022dec`, label `81024144`, the plain HP ≤ 0 case): `setDead(-1, ragdoll=1)` `8101dc98`, then `isDeadBlowMot` → **motion_blow 0x45** (death launch, §4.7, ending in dead) or **dead 0x7d**. `setDead(time, …, p4, p5, …)` zeroes HP, stores the corpse time in `this+0x1794` (−1 = default), sets `0x468` bit 3 = p4 and bit 9 = p5 (**ragdoll death**), releases besiege/group/attacker, adds kill points and spawns blood effect 0x18e. Gory special kills skip this path: neck cut, waist cut (`deadWaistCut`, impulse 30–80) and spine (`dmSpine`). They also send `sendFearPos(9.5 m)` (§5.1).
- **Clip deaths**: for ATHIT types 0xab and 0x197 (and the 0x7f head crush) it plays `getDamageMotion(type)` and clears bit 9, so there is no ragdoll and the corpse timer is 15 frames when unset. Type 0x3a also clears bit 9 and uses timer `0x8112338c`. Everything else ragdolls.
- **dead state** `81034408`: sub 0 drops items and sets the dead event. Corpse time = 60 s (1800 frames) when unset. Global flag2 0xd (`0x80697e8c`) caps it at 5 s (1.5 s on stage id 0x501). The target is dropped after 150 frames. With bit 9: `startRagdollDieMode(1.0)`, `setNullSpeedRate(0,1)`, `initRagdollDown`. When prop disable-flag 10 is clear, the body is released at once (`setCorpse`, blood parts 0x17c, → release 0x99).
- Sub 2 waits 6 frames (ragdoll) and sub 3 waits 3–5 frames more, then `setCorpse` (`cEnemyBase::setCorpse` `810646c0` → `cEmObjList::addCorpse` `801e122c`). Every frame without a ragdoll the death clip keeps updating, and with a ragdoll `updateRagdollDown` runs (blood 0x1eb once on landing).
- The corpse time counts down by the frame step. At 0 (or when the ragdoll ends without physics, cfg `0x870` bit 3 clear): vanish puff effect 0x36 + `reqRelease`, or blood parts + release state 0x99 when `this+0x118c` bit 4 (area ragdoll delete) is set. Subs 4–7 are the shrink-out disposal (scale ×(1 − 0.15)/frame until < 0.33).
- **Ragdoll physics** comes from Havok (`setRagdollG` `81063afc` stores the gravity scale at `this+0x678`). Ini 死体重力 1.6, 死体減衰XZ 0.93, 死体減衰XZMAX 0.945, 死体衝撃 52.0 are in `[em0110]` and `[em0110B]`, not `[em0101]`. em0101 therefore keeps the defaults (1.0 / 0.9 / 0.93 / 55) unless records inherit **[inf]**. For the port: a ragdoll-free stand-in is the blow/down clip, then lying still for 60 s, then the puff.

### 4.7 motion_blow `81049db0` (launched / blown away) [obs]
Sub 0 picks three clips per incoming type `this+0x141c`: **flight** (played now), **fall/land** (`this+0x57c`) and **down** (`this+0x580`). Defaults (types without a case, label `8104a5cc`): a 50/50 pick from table `0x811236f0` of {`em0110_1090`, `em0110_101a`, `em0110_1095`} or {`em0110_1091`, `em0110_101a`, `em0110_1096`}. The blow types 0x7b, 0x8a, 0x125 and 0x113+ use {0x90e `em0110_1090`, 0x8f3 `em0110_101a`, 0x90f `em0110_1095`}, with a root-motion scale of 2.0 for 0x79–0x7a and 2.5 for 0x113. 0x96 (bat-type) uses 0x8f1 `em0110_1018` (or 0x863 `em0110_1097` when the player's `+0x129c` ≥ 2.49), then `em0110_101a`, then 0x8f2 `em0110_1019`. 0xcc–0xce use {0x912/0x913/0x914 = `em0110_1092/1093/1094`}.
Two kinds of flight:
- **Root motion** (most types): gravity off (`setGravity(0)` sets cfg `0x868` bit 3), the flight clip moves the body with `setNullSpeedRate(scale × MotBlowRate prop+0x68 = 1.0)`. Motion speed is 1.
- **Physics "fly away"** (0x5f, 0x7c, 0x96 types and the 0x116/0x11e–0x121 group; clip 0x933 = `em0110_10a3` with the fly-away voice): `0x45d` bit 6, gravity on, initial velocity `this+0x8bc = (0, 0.22, 0)` (0.28 for 0x121 group), clip speed 1.6 (1.4 for the 0x6b/0x77/0xf0/0x1ab group). Per frame `pos += dir × v` with `v = this+0x1254` = 1.3 (0.7 / 1.6), decaying 2 % per frame. `dir` = `getBlowMotDir` `810499f8` = the grunt's local −Z, flattened (it is faced toward the attacker first, so it flies away from the hit). Gravity = `this+0x8a4` = **0.029 m/frame²** (`cEnemyBase::initialize` `8105e930`, constant `0x8112559c`), integrated in `cEnemyBase::moveFall` `81062aa8`. The floor is not checked for 20 frames.
After the flight: when the flight clip ends (root motion) or the body is on the floor (`this+0x396`, `checkFloorLanding(0.05)`), gravity comes back on. If airborne, sub 2 plays the fall/land clip until the floor is reached. Then sub 4 plays the down clip (blend 3, yaw += `this+0x12d4`), applies the stored landing damage (`setOwnDamage`), and on its end → `getDownProc` → `chgDownIdle` (down_idle 0x4d / 0x4f, §4.4), wait, or dead when HP is 0. Wall hits during sub 5 go to `msg::OnHitWall` **[obs]**.

### 4.8 Player attacks → ATHIT_ATK_TYPE → grunt reaction

**Sources** **[obs]**. Type names come from the DOL tables `0x805588B4` (×168) and `0x80558DF4` (×94), staged in `assets/native/combat.json` (`atk_names_pl`/`_em`). Power is column 1 of `plem_common.ini [damage_pl]` (same file). The per-state assignments come from a scan of the DOL for immediates stored into the player's attack id `+0x1284` (the value passed to `reserveAtkHit` `802C81F4` and to the target's `startAtk`), mapped to states with `pl0000-state-table.txt`:
B39 0x5e/0x5f/0x6d–0x74/0x76–0x78/0x7d–0x7f/0x81; B72 0x7b/0x7c (and 0x75); B84 0x6e; B21 0x10d; B25 0x4e; B38 0x53; B43 0x54/0x55; B44 bat; B41/B42 kukri/naginata; B45 blade; B46 double chainsaw 0xcc/0xcd; B61 0xea; B73–B77 the weapon charge attacks.
Grab and throw types reach the target through vtable calls in the grab states (B24 0x4f, B25 0x4e/0x51/0x52, B32 0x4d), and the finisher states (`802a4a78`–`802a99b4`) pass 0x30/0x31 as catch types (PL0000-COMBAT.md) **[obs scan; per-call meaning inf]**.

"Not announced" = no vfunc-0xac call was found in that player state (§4.1), so no evade or guard roll **[inf]**. The chainsaw-cut row's 30 % applies only if those types reach `startAtk` (they appear as catch types in the finishers) **[unres]**. "Guard" = `isGuardEnable` for street em0101 (variant A, rank < 0x20, チェーンソー防御 0) **and** not on `startAtk`'s skip list. Every guard or evade also needs the 5 % / 10 % rolls and the one-per-life budget (§4.1). "Routing" = `decideDamageProc` `81022dec` for a grunt that survives the hit. "Motion" = `getDamageMotion` `810171c4` for damage 0x41.

| Player move (state, clip) | type | power | guard? | evade? | routing (alive) | damage motion / notes |
|---|---|---|---|---|---|---|
| A string 1100–1103 (39) | 0x6d–0x70 PUNCH_A0–A3 | 5/10/10/20 | **yes** | 10 % | damage 0x41 (down → 0x5a) | combo tree §4.3: 10a5 → 10a6 → 10a7 → 10a8 |
| A4–A7 (39, same clips after flag 0x40) | 0x71–0x74 | 30 | no | 10 % | damage 0x41 | direct 0x915–0x918 = `10a5`–`10a8` |
| stick-forward finisher 1105 (39, flag 0x100) / swing B2 1105 | 0x77 PUNCH_B2 | 50 | no (chainsaw class; breaks a guard) | 10 % | **launch only at HP ≤ 20 %**, else damage 0x41 | combo tree (C) |
| swing B1 1104 (vertical) | 0x76 PUNCH_B1 | 50 | no (breaks a guard) | 10 % | damage 0x41 (no HP gate) | combo tree (C): 1st hit `em0110_1026` |
| swing B3 1106 | 0x78 PUNCH_B3 | 50 | no (breaks a guard) | 10 % | launch at HP ≤ 20 % (faces the attacker), else damage | combo tree (C) |
| short charge (72, released before loop frame 10) | 0x75 PUNCH_B0 | 20 | **yes** | 10 % | damage 0x41 | combo tree (C) |
| charge attack (72, `wp0600_1115`) | 0x7b PUNCH_CHGATK / 0x7c CHGATK2 (full charge) | 20 / 150 | no | 10 % | **always motion_blow 0x45** (0x7c: physics fly-away, §4.7) | – |
| rush 1111 (39, >4 presses) | 0x5e RUSH_PUNCH | 10 ×6 | **yes** | 10 % | damage 0x41; `onPreMove` sets rush mode (`0x468` bit 0) | default 0x803 `em0110_1001` |
| finish blow 1112 (39, target HP ≤ 10) | 0x5f FINISH_BLOW | 150 | no, and `startAtk` skips it | none | lands only while the grunt plays 0x929 = `em0110_1228` (`refHitPos`); a kill → `dmCrushHeadLight` + motion_blow | – |
| jump attack 1122 / landing 1123 | 0x7e / 0x7f PUNCH_JUMPATK0/1 | 20 / 80 | no | 10 % | damage 0x41 | 0x7e default `1001`; 0x7f: 0x800 `1008` at HP ≤ 20 %, else 0x91f `10a4`; a 0x7f kill crushes the head (`dmCrushHeadLight`) |
| dash attack (39) | 0x7d PUNCH_DASHATK | 20 | no | 10 % | launch at HP ≤ 20 % (faces the attacker), else damage | 0x8f4 `em0110_101b` |
| counter attack (39) | 0x81 PUNCH_COUNTERATK | 5 | no (breaks a guard) | 10 % | damage 0x41 | 0x8f4 `101b` (0x91f with `0x45e` bit 0xb) |
| taunt 00b0 (84) | 0x6e PUNCH_A1 set up, part 9, power 10 | – | – | – | **no hit**: `pl0030_00b0_00.seq` has no op-7 trigger, only the op-0xB window at frame 25 **[obs data]** | provokes instead (§5.1) |
| giant swing (43) | 0x55 GIANTSWING | 300 | no, skipped by `startAtk` | none | no state change: the grab states move the grunt **[inf]** | a kill takes the catch-death path |
| wall crash (43), Mad Buster (38), bound / down throw (25 / 24), backbreaker, hang throws (32) | 0x54, 0x53, 0x4e, 0x4f, 0x56, 0x4b–0x4d, 0x52 | 20, 60, 40, 40, 40 | no | not announced | no state change (caught); 0x56 spawns blood parts effect 0x19d | a kill takes the catch-death path (`81024310` switch); a 0x56 kill cuts all parts (`partsCutOnly`) |
| stomp | 0x5a STOMPING | – | no | not announced | **damage_down_pursue 0x4e** | – |
| spin kick | 0x5b SENPUKYAKU | – | no, skipped by `startAtk` | none | damage **[inf]** | a kill → blow_ragdoll 0x3f straight up |
| guns (Kreely's pistol = HANDGUN) | 0x60–0x65 (0x61 HANDGUN 20) | 20 | no | not announced | no `decideDamageProc` state change (the 0x60–0x65 cases break out) **[obs]**; the bullet path (`onHitEmBullet` `8105d6f0`) is not traced **[unres]** | – |
| bat string / dash / charge (44, 75) | 0x8e–0x95 / 0x97 / 0x96 | 30 / 30 / 100 | no | 10 % | damage 0x41 / motion_blow facing the attacker / `chkProcBatChgAttack` | bat combo tree `+0xaf0` |
| chainsaw cuts (finishers, B-chainsaw) | 0x2a TATEGIRI, 0x2c, 0x30/0x31 S_TATEGIRI/S_YOKOGIRI | 100 | no (excluded in `chkGuardDefault`) | **30 %** (チェーンソー回避) + claw counter 0x105 | sever / kill paths **[unres: `isDamageSever` `810128d4`, `calcDamageDead` `8101353c`]** | default 0x803 |
| oil drum | 0xf0 OIL_DRUM | 200 | no | – | motion_blow (voice 0x10a0); a kill: voice 0x1007 | – |
| object cover (tyre-style) | 0xf6 OBJ_COVER | 10 | no | – | sends warning 0x1007 (5.5 m, §5.1) | 0x8f4 `101b` / 0x91d `em0110_1513` |

**The port's ids** (`crates/mw-sim/src/fight.rs`, `kreely.rs` + `assets/native/kreely-moves.json`, `crates/mw-game/src/play_street.rs`): Jack uses the native ids above (0x6d–0x74, 0x76–0x78, 0x5e/0x5f, 0x7e/0x7f, 0x55, 0x61 for the pistol). Kreely's added moves reuse them: cross / step_jab / whip 0x6d, cross_m / turn_jab / whip_alt 0x6e, knee 0x6f, uppercuts 0x70, power_punch **0x76**, spin_kick **0x77**, sweep **0x78** (the street's `SWEEP_ATK`), rush 0x5e, suplex 0x55. Natively 0x76–0x78 are chainsaw swings. A grunt never guards them (and they break a guard), dodges them at the normal 10 %, and launches on 0x77/0x78 only at HP ≤ 20 %. The port's sweep knockdown (`gh_sweep_down`) is a port addition with no native counterpart for these ids **[obs port code vs native tables]**.

## 5. Other gameplay behaviour

- **Laugh / taunt** [obs]: when the target's HP is 0 and 笑い確率 ≠ 0, from wait (`8102b818`) or siege 0x11. Also when a nearby enemy is a "laughing-stock" (`getNearLaughingstockEm(6 m)`). `laugh` `810341e0` picks a random laugh other than the current one from {0x0c, 0x07, 0x08, 0x09, 0x0a} = `em0110_0045, 0046, 004c, 004d, 004e` (`getLaughMotion` `810185f8`). It lasts 3–4 s (90–120 frames); with a 1/6 chance it lasts until a flag instead. It faces the laughed-at object, then → wait. It is cancelled when the object becomes "safe" (`br_laugh` `81034138`).
- **Angry** [obs]: `setAngry` `81020010` (13–16 s, from target_found when flag `0x468` bit 0xe is set). It is the only setter of the angry timer `this+0x12b8` and the `this+0x3c0` bit 5 "angry" flag in the RSO. Being angry adds +50 % per-frame attack rate in `chkChgAttack` and lets it attack while not holding the token. `moveMain` counts the timer down and calls `resetAngry` at 0.

### 5.1 Messages (`cEmZako::onRecvMsg` `810595dc`) [obs unless marked]
Senders use `cEmObjList::sendMsgEm(list, from, id, p3, p4, mask, …)`. Mask 0x20000 = all zako, and the return value is the sum or OR of the receivers' answers. Unhandled ids return 1 (label `8105add4`).

**Fear 0x1011 / warning 0x1007** (shared body `8105a190`). `sendFearPos(r)` `81015184` sends 0x1011 with {pos, r²}. Senders: gory kills in `decideDamageProc` (`810235d4`, `8102480c`), `dmSpine`, `dmNeckCut`, `deadWaistCut`, and rig event 0x1e (`onRigEvent` `81011250`), all with r = 9.5 m. `sendWarningPos(r)` `81015120` (0x1007) is sent only from `onPreMove` `8100d95c` on damage type 0xf6 with r = 5.5 m. A receiver reacts only if all of these hold:
- rank (`this+0x928`) < 0x20, no lost limb (`this+0x1738 & 0x78`, flag `this+0x1740` bit 0xf), not in attack 0x1c;
- evade flags `0x45e` bits 0x10/0x11 are clear;
- `rand(0..99) <= 怖がり確率` (90 → ≈91 %);
- |self − src|² < r².

Then: escape dir `this+0x1600 = normalize(self − src)` (−forward if zero), set `0x45e` bit 0x10 (evade pending), stamp the frame clock into `this+0x17b8`, clear bit 0x12, set bit 0x13 = (id == 0x1011), and set the delay `this+0x1640 = rand(3, 12)` frames (0.1–0.4 s).
The reaction runs in `chkEvade` `810296dc` (vfunc 0x490, called from `br_wait`, siege, provoke) and in `chkEvadeFear` `810295ac`. Each counts the delay down. When it hits 0 within 36 frames of the stamp: with bit 0x13 (fear) the grunt faces `atan2(−dir)` (toward the source), → **fear_to_escape 0x0a**, and bit 0x11 is set. A warning (0x13 clear) dodges like the 0x1006 case below. After 36 frames all bits are dropped.

**fear_to_escape** `8102d21c`:
- sub 0: plays a random startle clip from {0xc5 `em0110_0016`, 0xc6 `em0110_0017`} (0x8c0 `em0170_0200` when flag `0x461` bit 0x1e), blend 5.
- sub 1, at clip end: rolls `rand(0..99) < 怖逃げ確率` (80 %). On a fail → wait. On a pass: run clip 0xc8 `em0110_0018` or 0xc4 `em0110_0015` (50/50), target yaw = facing + π, voice 0x1002.
- sub 2: turns (`_turnDir`, 1 rad/frame cap) until within 8° of the target yaw, then sets the run timer `0x168 = rand(33, 54)` frames (1.1–1.8 s).
- sub 3: runs away. Every 4 frames while the timer is < 24 it probes 1 m ahead (`checkShapeHit`). A wall hit → sub 0x10 with timer 30–45 frames. When the timer ends → sub 0x30.
- sub 0x30: plays 0x923 `em0110_1512` or 0x485 `em0110_0030` (50/50), then → wait.
- sub 0x10–0x13 (cornered): `em0110_1109`, then loop `em0110_110b` until the timer ends, then `em0110_103a`, then → wait.

**fear 0x97** `8103aa1c` is a separate scripted state. It is entered only through reserve bit 0x12 of `moveMain`'s reserve table (below), and no cEmZako code sets that bit with a literal **[obs: scan]**. It sets no motion itself (it plays what the caller set): sub 1/0x10 → wait at clip end or timer `0x167`, sub 0x30 → dead. Street grunts reach fear only via fear_to_escape **[inf]**.

**Dash heads-up 0x1006**: the only senders are the two `sendMsgEm(…, 0x1006, …, mask 0x20000)` calls in siege_target `81030300` sub 0x1c (`81032d74`, `81032e80`). They fire when the siege attacker starts its forced dash attack (after `forceAttack` succeeds) or starts running at the target (beyond 4.5·d). No player move and no DOL code sends 0x1006 or 0x1007 **[obs: immediate scan of the RSO and the DOL]**. So a dash dodge is always a grunt stepping out of a fellow grunt's dash. Payload: p3 = sender matrix, p4 = unit travel direction. A receiver in the path reacts when the bearing from the sender has dot(·, dir) > 0.98 (≈11°) and the same lost-limb/evade gates as fear pass. It sets bit 0x10, delay `rand(0, 6)` frames, and escape dir = ± the sender's local X, on whichever side the receiver already is.

**The dodge** (`chkEvade` `810296dc`, for 0x1006, a warning 0x1007, and the camera self-trigger below). The delay `this+0x1640` counts down by the frame step. The dodge runs at 0 if it is still within 36 frames of the stamp; after 36 frames bits 0x10–0x13 are dropped. With bit 0x13 (fear) it is fear_to_escape. Otherwise it picks by dot(own forward, escape dir) **[obs]**:
- \> 0.707: forward **run_one 0x19** if `chkObstacleSphere(4.0, 0.8)` finds nothing ahead (walk_one 0x0f when bit 0x12 is set).
- < −0.707 and `isEnableMoveB(1.0, 0.4)`: **move_back 0x20**, or **bk_jump 0x23** when the target lies ahead (dot > 0.17) and bit 0x12 is clear.
- otherwise, and as the fallback when the forward or back move is blocked: lateral by x·dir ≤ 0 → **r_side_run 0x1a** (r_side_walk 0x15), else **l_side_run 0x1b** (l_side_walk 0x16).
- Then bits 0x10/0x12/0x13 are cleared and bit 0x11 is set to "not the walk variant". `_chgState` `8105b730` clears bit 0x11 again on every state change (`8105c148`). So bit 0x11 only locks out a second dodge while the dodge state runs; it is not a once-per-life lockout **[obs]**.

| dodge | state (br / update) | clip (frames, root displacement from `kungfu-motion.json`) | play | end |
|---|---|---|---|---|
| run_one 0x19 | `8103c694` / `8103c6e0` | `getRunMotion` 0x13 = `em0110_0012` (21 f, **+3.3 m forward**) | blend 5, once; sets `0x45d` bit 3 (running) | clip end → `chkRunEnd` `81046cfc` (may append 0x29, the fill clip) → wait 4 |
| r/l_side_run 0x1a/0x1b | `8103c7fc` / `8103c848`, `8103c9a0` / `8103c9ec` | `getRunR/LMotion` 0x16 = `em0110_002b` (24 f, **2.9 m right**), 0x17 = `em0110_002c` (21 f, **2.4 m left**) | blend 10; extra loops = byte `this+0x1674`, which `_chgState` zeroes, so one cycle; `0x45d` bit 3; turns toward the target at 1 rad/frame | → wait 4 |
| move_back 0x20 | `8104052c` / `810405a8` | `getWalkBkMotion` 0x0f = `em0110_0011` (61 f, **0.9 m back**) | blend 5, one loop (`0x57c` = 1), faces the target | → wait 4; its `br_` re-enters siege 0x7a when `chkGroupAttacker` passes |
| bk_jump 0x23 | `81040b38` / `81040b3c` | 0x1a = `em0101_0121` (16 f, **2.3 m back**) | blend 5, no damage (§4.1.3) | `chkCounterAttack` or wait |

Each dodge's `br_` runs `checkFall` and `chkEvade` again (bit 0x11 blocks a nested dodge). Distances come from the clips' root motion; the code adds none **[obs]**.

`chkEvade` also self-triggers the same dodge when the grunt blocks the camera: camera-local |x| < 2.7, z < 2, and the player is not nearer, with delay 4·min(0.4·dist, 1) frames.
**Finisher back-off**: with no dodge this frame, bit 0x11 clear, no held object (`this+0x1290` = 0) and the player in a finish move (`cPl0000::isFinish(0xc0000)`), a grunt within 4 m goes to move_back 0x20 and one farther away to threat_aggressive 0x77 **[obs]**.

**Provocation** (player taunt; in `chkEvade`, not a message). The player's state byte `+0x11b` is the cPl0000 state id (`802CBF50` changeState → `setRno`, PL0000-STATE-SURVEY.md), so **'T' = state 0x54 = 84, the taunt** **[obs]**:
- **Input**: `fn_802C68AC` enters state 0x54 when the pad trigger bit 4 (`+0xEE4 & 4`) is set. It is called from the A (pre-update) halves of idle 0 (odd idle phase only), 29, 39–42, 44–48 and the weapon charge states 75–77 **[obs: bl scan]**. The port maps Taunt to wire bit 0x400 = D-pad Down (`crates/mw-game/src/native_controls.rs`, `pc_config.rs`: T / PadDown) **[inf: bit 4 = D-pad Down after the game's remap]**.
- **State 0x54** (B `802be44c`, A `802be3c4`) **[obs]**: it plays `pl0030_00b0.mot` (blend 2, speed `+0x114c`; ≈74 frames in the `pl0000_00b0` copy) and, on the chainsaw, `wp0600_00b0.mot`. It sets `+0x1168 = 900 + frame count` and returns to idle 0 at the clip end. From the op-0xB window (frame 25) A[84] lets evade, jump, attack and grab cancel it. Its PUNCH_A1 setup never hits (§4.8).
- **Grunt side**: if the grunt is not angry, nearly dead, piyori/faint or grabbed, has no reserve bit 0x10 pending, holds no object (`this+0x1290` = 0), has `0x5d0` (`this+0x1740`) bit 0xf clear, and dist² < prop+0x30 (225 = 15 m) and |Δy| < 8 m. Every grunt in range reacts, not only the target:
1. retarget the player, set `0x468` bit 0xe, clear the reserve word and set reserve bit 0x10, `0x1438 = −1`, delay `rand(0.1·30, 0.3·30)` = 3–9 frames;
2. `moveMain` fires the reserve (rank < 0x20, not the attacker, not already there): **target_found 0x74** (`em0110_0036`, 64 frames, root +1.0 m);
3. target_found sees bit 0xe and calls **`setAngry(rand(13·30, 16·30))`** = 390–480 frames → wait;
4. `br_wait`'s `isAngry` → **provoke_reaction 0x71** `8105d36c`. Sub 0x10: run (`getRunMotion` 0x13 `em0110_0012`, blend 5) at the player while dist² (`this+0x894`) > `0x167 = r1·r2`, with r1, r2 = rand(2.8, 3.5), i.e. a stop radius of ≈2.8–3.5 m. Sub 0x20: a threat wait (`getThreatWaitMotion` vfunc 0x464 → 0x39/0x3a/0x3b = `em0110_0042/43/44`, 99/67/134 frames, blend 5), with a new threshold `rand(4.5, 5)·rand(4.5, 5)`. At each clip end: dist² ≤ threshold → another threat wait (sub 0x20); farther → run again (sub 0x10). It faces the target at 1 rad/frame throughout.
- **setAngry** `81020010`: angry timer `this+0x12b8` = t, `0x12bc` = 0, `setEfcBloodColor`, **parts effect 0x12f** on the grunt (the anger mark), `this+0x1365` +1, `cEnemyBase::setAngry(1)`. `moveMain` counts the timer down; at 0, `resetAngry` `810200bc` clears `0x468` bit 0xe and the counter and kills effects 0x12f/0x130. While angry, `chkChgAttack` adds +50 % per frame (52 % for em0101) and lets the grunt attack without the token or from behind the target (§3.2), and a new taunt is ignored.

`br_provoke_reaction` `8105d290` exits to wait when the angry timer runs out or the target is lost. It also runs `checkFall`, `chkKill`, `chkEscape(1.0)`, `chkEvade` and `chkJump(30)`. 挑発反応角度 (`+0x34`) is not used on this path **[obs]**. Messages 0x1012/0x101d hold the alternative message-driven provocation, but for obj id 0x20170 only.
**Second trigger** (same function): when the player's `+0x398` = 0x2e (set only by player states 89/90, a grab-carry pair whose B90 throws BOUND_THROW 0x4e **[inf]**), grunts in the same 15 m / 8 m range with rank < 0x20 get `0x468` bit 0x18 and reserve bit 0x11 after 9–30 frames. `moveMain` turns that into **cover_target 0x38** **[obs; meaning unres]**.

**'H' = state 0x48 = 72, the charge attack** (B `802b9118`) **[obs]**. Phase 0/1: Jack winds up (`pl0030_0000` + chainsaw `wp0600_1113`, parts effect 300, `changeCameraAngle`) while A is held with the remote raised (|accel y| ≥ 1). Letting go earlier turns it into a state-39 swing (`changeState(…, 0x27)`). At the clip end he holds the charged loop (`wp0600_1114`, phase 10/11) until B is released or the loop ends. Phase 12 fires `wp0600_1115` with PUNCH_B0 0x75 (released before loop frame 10), PUNCH_CHGATK 0x7b (frame ≥ 10) or CHGATK2 0x7c (in the last 6 frames of the loop or after it ended) and calls the target's `startAtk`. The code passes 500 (1000 for 0x7c) as power, and the `[damage_pl]` rows override it with 20 / 20 / 150. While Jack is in this state, `chkChgAttack` gives every grunt +50 % attack rate. **[inf: the button roles from the `+0xEE0` bits 0x10/0x20]**; the entry transition was not found as a literal `changeState(…, 0x48)` **[unres]**.

**Other ids**:

| id | meaning |
|---|---|
| 0x1005 | falls through: returns 1, no action |
| 0x1009 | "come here": in wander/0x73 without a target and within 知覚距離 → `chgTarget(p4)` |
| 0x100a | returns `this+0x870` flag 3 |
| 0x100b | group-slot hand-over |
| 0x100c | clears `0x468` bit 0x1a and sets `this+0x13f8 = 0x127` |
| 0x100f | "are you an attack supporter" (sender counts supporters against 攻撃補助人数) |
| 0x1010 | "is your `this+0x178c` == p4" |
| 0x1013 | clears the provocation reserve |
| 0x1014/0x1015/0x1022 | catch-object queries |
| 0x1016 | evacuation for 0x20150 |
| 0x1018 | "are you in entrance (state 2)" |
| 0x1019 | "are you in attach_parts 0x54" |
| **0x101b** | "are you attacking target p4": yes when in attack 0x1c or siege sub 0x1c. The siege sender (non-player target, type 4) becomes the attacker only when nobody answers and the target has been idle > rand(2.5, 4) s |
| **0x101c** | laughing-stock offer: p4 = object, p3 = frames. Accepted when 笑い確率 ≠ 0, guard conditions allow, no current laugh object and dist < 12 m. Sets `this+0x174c = p4`, `this+0x1754 = p3` |
| 0x1020/0x101e/0x101a | species-specific (0x20160 / 0x20210) |
| 7 | "are you holding obj p4" (`this+0x127c`, returns `this+0x1290 != 0`) |
| **8** | "are you being grabbed by the player": not nearly dead, `isCatched` `810279ec` and grab phase `this+0x1650 == 3`. A non-zero sum switches the attack rate to 他掴まれ攻撃頻度 |
| 9 | "are you attacking p4" (attack 0x1c or catch attack 0x6f) |
| **0xb** | "are you mid-catch on the player within 3 m": `this+0x1658` is a player, dist < 3, `this+0x1464` < 30. The siege attacker (rank & 0xf0 == 0) holds back while anyone answers |
| 0xc | clears object refs |
| 0xd | clears `this+0x12e0` when it equals p4 |
| 0xf | `8105955c` |
| 1 | hit-volume test |
| 6 | blown/killed by an object: → release 0x99 with an effect |
| 0x18/0x16..0x25 | stuck/attach and object queries |

### 5.2 Per-frame bookkeeping `moveMain` `81015f60` [obs]
In order, each frame. "−step" means minus the frame step `this+0x898` (= 1):
1. `this+0x1170` = global flag2 1, unless the grunt is in a wind area.
2. **Reserve reactions**: the delay `this+0x1730` (≥ 0) gets −step. When it reaches 0 (and `isReserveGuardReaction`), the lowest set bit of the reserve word `0x5cb` (`cFlag<eGuardFlag>`, set via `8101c608`) picks a state from table `0x81123158` (unless `0x468` bit 7):

   | bit | → | bit | → |
   |---|---|---|---|
   | 0/1 | r_jump 0x26 / l_jump 0x27 | 0x0c | threat_battle 0x76 |
   | 2/3/4 | `chkEvadeJumpRight/Left(0,1)` (r/l_jump, with fallbacks) / `chkEvadeJumpBack(0)` → **bk_jump 0x23** | 0x0d | **catched_counter 0x61** (the guard's exit, §4.1.2) |
   | 5/6 | **face_guard 0x2a** / body_guard 0x2b (skipped when already in it) | 0x0e | counter 0x2d |
   | 7/8 | `chkEvadeJumpRight/Left(1,0)` → **r/l_rolljump 0x35/0x36**; `0x461` bit 0 set and the hit mask cleared | **0x10** | **target_found 0x74** (rank < 0x20, non-attacker) |
   | 9/0xa | 0x2e / 0x2f | 0x11/0x13/0x14/0x15 | cover / bills / food / bite |
   | 0x0b/0x0f | `_decideAttack` → attack 0x1c or wait | **0x12** | **fear 0x97** |
   | | | 0x16 | sp_attack 0x02 |

   The byte table is `26 27 26 27 23 2a 2b 35 36 2e 2f 1c 76 2c 2d 1c 74 74 97 74 74 74 02` **[obs]**; bits 4–8 and 0xb–0x15 are special-cased as listed. Then the word is cleared. Setters seen: provocation (bit 0x10/0x11, `chkEvade`), `chkEmEvent` (scripted), `startAtk` (§4.1: codes 3/4/5 → bits 4/5/7–8), `face_guard` (exits → 0xd), `catched_counter`.
3. If alive (vfunc 0x23c `isDeadHp`):
   - **HP regen**: `onPreMove` sets the delay `this+0x1474 = 30` and syncs the float HP `this+0x1440` on every hit (`8100f248`). Once the delay is ≤ 0 it keeps counting negative. Unless prop disable-flag 3 is set (`81016bf0`; ini 回復 = 0 **[inf]**), or while `0x468` bit 0x1c is set, HP gains 0.0008·min(|t|, 300) per frame, capped at max HP. That is a linear ramp up to 0.24 HP/frame (7.2 HP/s) 10 s after the last hit, about 36 HP over the first 10 s. Bit 0x1c drops once HP > max/3. em0101 sets no 回復 key, so **street grunts regenerate** **[inf on the key mapping, obs on the code]**.
   - `recoverPiyori` / `recoverFaintAway` (stun timer `this+0x888` −step), unless both hands are caught.
   - Part durability: after the delay `this+0x1454` (= prop+0x82, 耐久力回復Delay, set on hit) runs out, `this+0x1448/0x144c/0x1450` refill toward prop+0x7a/7c/7e (頭/腕/足耐久力) at step × max / prop+0x80 (耐久力回復).
   - `this+0x1444` −step while > 0.
   - tired timer `this+0x17b0` −step (not while attacking) → `resetTired`;
   - `this+0x17b4` −step while byte `0x17ad` is set;
   - short `0x17c2` −1;
   - `updateTargetInfo`, `chkRoute`;
   - **angry** `this+0x12b8` −step → `resetAngry` at 0;
   - **berserk** `this+0x15e0` (< 10000) −step → `setBerserk(0)`;
   - **burning** (`isFireDamage`, vfunc 0x184): `this+0x1168` −step → `dmFire(108000)` ends it.
4. Always:
   - `this+0x146c` −step (free-running);
   - `chkEmArea`, `chkEmTerritory`;
   - the attacker-token drop of §2.1;
   - **state dispatch**: state < 0x9c → `chkAreaEvent`, then the `br_` and update member-pointer calls (`0x8115dfd4`, `0x8115e724`); else the subclass's vfunc 0x99c (`cEm0101::onMove` `81071b84`);
   - frame counter `this+0x1418` +1, state clock `this+0x1414` +step, `chkTimeEvent`;
   - int `0x469` (`this+0x11a4`) −1;
   - `moveNeck`;
   - velocity `this+0x1264`: `pos += vel·rate·step`, `vel *= 0.9` per frame (water: drag 0.035/0.027);
   - `moveFall` + `checkFloorLanding(0.05)` while gravity is on (§4.7). `0x468` bit 10 (airborne after a hit) clears on landing;
   - **down timer `this+0x1154` −step while > 0** (set by `chgDownIdle`, §4.4);
   - `this+0x13f4` counts **up** by step from 0 and resets to −1 after 60.
State-local timers (`0x167` hold, `0x169` idle loop, `0x168`, `0x16a`…) are not touched here. Each state counts its own.

### 5.3 Flag words [obs for set/clear sites, inf for names]

| word.bit | set / cleared by | read by → meaning |
|---|---|---|
| `0x45d`.0/1 | attack phase 0/2 on (bit 1 off at the end) | `startAtk` → attack phase markers |
| `0x45d`.2 | r/l_side_walk on; `_chgState`, siege off | walking sideways |
| `0x45d`.3 | run_one, r/l_side_run on | attack flag bit 2 (`_decideAttackSub`), `moveMain` → "running" |
| `0x45d`.4 | down_idle, down_pursue on | has been down |
| `0x45d`.5 | rig event 0xb on, rig event 0x12 off (`onRigEvent` `81011250`) | `chkComboContinue`, attack, target_found, laugh → **combo/cancel window** |
| `0x45d`.6 | motion_blow physics flight | physics blow in progress |
| `0x45e`.0x10–0x13 | §5.1; bit 0x11 is set after a dodge starts and cleared by every `_chgState` (`8105c148`) | evade pending / dodge in progress / walk variant / fear |
| `0x45e`.0xb | `refHitPos` on a chainsaw-class hit while guarding | guard broken: type rewritten to 0xef, no launch for 0x75–0x78 |
| `0x45e`.0x17 / 0x18 / 0x1c | rig events 7 / 0x13 (primary / secondary attack window open), cleared by 0x12 / 0x14 | attack window open. These are the type 7/0x12 and 0x13/0x14 events of `motion-cues.json` |
| `0x45e`.0x1d | rig event 0x26 | sound/effect cue id in `this+0x179c` |
| `0x45e`.0xc | `chkGuardDefault` on (every evade/guard), `catched_counter`, `chkCounterAttack`/counter off | counter armed (attack flag bit 7); id in `0x57f` (−1 = threat_battle instead) |
| `0x45e`.0x1e | | alternate fear clip (`em0170_0200`) |
| `0x468`.0 | `onPreMove` | rush mode (`chkRushModeEnd`) |
| `0x468`.1/2 | `chkEmEvent` | scripted event flags (also gate the 0x1f sense messages) |
| `0x468`.3 / .9 | `setDead` p4 / p5 | dead flag / **ragdoll death** (cleared for clip deaths) |
| `0x468`.7 | | frozen in place: `moveMain` restores the position from `this+0x1408`, reserve reactions are blocked |
| `0x468`.10 | damage on, landing off (`moveMain`) | airborne after a hit |
| `0x468`.0xc | `_decideAttackSub`/attack/`forceAttack` off | |
| `0x468`.0xd | msg 0x1012 on / 0x1013, `_chgState` off | message provocation pending (0x20170 only) |
| `0x468`.0xe | provocation in `chkEvade` | target_found → `setAngry` |
| `0x468`.0xf | | attack keeps tracking the target; attack end clears it |
| `0x468`.0x13 / 0x14 | `refHitPos` on a blocked hit (and 0x13 in evade jumps); `_chgState` off | face_guard turns to the attacker on 0x13 |
| `0x468`.0x19 | `registAttack` on, rig event 7 / attack off | hit already registered for this window |
| `0x468`.0x1a | `startAtk` on; `_chgState`, damage, msg 0x100c off | attack started |
| `0x468`.0x1c | **[unres setter]** | regen-while-down; cleared at HP > max/3 |
| `0x468`.0x1d | `_chgSiegeState` on, `_chgState` off | siege sub-state active (blocks `chkEvade`'s main body) |
| `0x468`.0x1e | damage, blow_ragdoll on; `onPreMove` off | read by `decideDamageProc` |

## 6. Mapping to the port (`play_street.rs`)

The current port: per-grunt `State {Idle, Walk, Attack, Flinch, Launched, Swept, Suplexed, Down, GetUp, Dead}`, HP 60, aggro 14 m, reach 1.6 m, 0104/0105/0106/0110 picked freely, Down 40 frames.

What the native AI needs, all expressible with per-grunt state, clip, frame, position, yaw, HP, the player position and the other grunts:
1. **Group state** shared by all street grunts: member list with slot numbers and one act number (the token). Slot yaw = table above, relative to the camera→player direction (slot 0 between camera and player). Radius `prop+0x24` **[unres]**: pick the value that matches captures.
2. Per grunt: siege sub-state, hold timer (`0x167`), idle-loop timer (`0x169`), attack count (`0x4a0`), combo count/max, evade budget, piyori stage, down timer.
3. Only the token holder runs the 0x1c approach with the 2 %/5 % per-frame attack rolls. Everyone else holds the slot with threat/idle clips, side-walks to the slot and backs off (walk-back / back-jump) when the player is within 1.5 m. That is the native "circle and wait" look.
4. Variant A (claws) attacks: 0104 (2.2 m) and 0105 (2.8 m), chaining 0104→0105→0106 up to 1–3 hits, plus the 3.5–5 m dash 0x604 = `em0110_010d`. Variant B: fire breath 0110 (3.5 m, ±30°) and 0x604. All hits do 7 to the player.
5. HP 120. Light hits play damage clips `em0110_1001/1000/1004/1008`. Blow-type hits launch only at HP ≤ 20 %. Down lasts 0–1 s, then neckspring (`em0110_1060`) if HP > 60 % else `em0110_103d` stand up. Stun at 70 % and 30 % HP for 5 s each (`em0110_1078`). 10 % dodge with one evade per life.
6. Street spawns are variant A, Type 0, with perception 12 m / lose sight 30 m and CharColRadius 0.40 (c201em.ini). Ring radius 2.5–3.5 m (one roll), range scale 1.0. HP regenerates after 1 s without damage (ramp to 7.2 HP/s, §5.2).
7. Launches (§4.7): root-motion flight `em0110_1090`/`1091` → fall `em0110_101a` → down `em0110_1095`/`1096` → down idle. The physics "fly away" `em0110_10a3` uses vy 0.22, h 1.3 m/frame decaying 2 %/frame and gravity 0.029. Death = launch or ragdoll, corpse kept 60 s, then a puff.
8. Reactions (§5.1): a gory kill within 9.5 m makes 91 % of the grunts startle (`em0110_0016/0017`) after 3–12 frames. 80 % of those turn and flee (`em0110_0018`/`0015`) for 1.1–1.8 s, then play `em0110_1512`/`em0110_0030`. A dashing attacker makes grunts in its path sidestep (side run/back/run_one). A taunt (player state 'T') within 15 m → spotted clip, angry 13–16 s → walk up to ~3 m and threaten.

9. Guard and evade (§4.1): only the target of Jack's swing reacts, 3 frames before the announced hit frame, within 6.16 m and from wait/siege/side-walk/move-back/threat/target_found/damage. A full-HP em0101 has one reaction per life: 10 % evade (50/50 bk_jump `em0101_0121` 2.3 m back, or a side rolljump `em0110_0022/0023`), else a 5 % guard on PUNCH_A0–A3 / PUNCH_B0 / RUSH_PUNCH. No damage during either jump. After a normal evade comes threat_battle; after a 30 % chainsaw-cut evade comes the claw `em0101_0104`. The guard (`em0110_1063`) blocks two punches with spark effect 0xc, the third lands. It ends after 45–60 frames, or 25 frames after Jack's last swing, or beyond 3.5 m, with the hop back `em0110_1075` (no damage during it) or a plain return to wait. Chainsaw swings 0x76–0x78, counter 0x81 and bat charge 0x96 are never guarded and break a guard (type → 0xef, clip `em0110_10a4`).
10. Damage clips per hit (§4.3 combo tree): punch string `10a5 → 10a6 → 10a7 → 10a8`; a chainsaw hit first `1026`; a punch→chainsaw pair launches with `10a0` only at HP ≤ 90 %. 0x7b/0x7c charge hits always launch; 0x77/0x78/0x7d launch only at HP ≤ 20 %.
11. Taunt (§5.1): Jack's state 0x54 (`pl0030_00b0`, D-pad Down in the port) provokes every grunt within 15 m that is not angry yet: 3–9 frames, then `em0110_0036`, the anger effect 0x12f for 390–480 frames, a run to 2.8–3.5 m and threat waits while Jack stays within 4.5–5 m. Angry grunts attack at +50 % per frame without the token. The taunt does no damage.

## 7. Unresolved / next

- Setter of reserve bit 0x12 (fear 0x97) and of `0x468` bit 0x1c (regen while down). Tried: literal-operand scans of the RSO for `on(0x172c, 0x12)`.
- Mapping of ini 回復 to prop disable-flag 3 (the key parser `fn_801E4090` case for 回復 wrote no offset in the earlier key dump), so regen-on for em0101 is inferred. Likewise the ×30 scale of 防御時間通常 (§0.1).
- Attack flag bit 12 (cfg `0x868` bit 0x19 meaning), bit 16 (vfunc 0x1d4) and the exact `param_7` pool of bit 4. Whether the 0x109 rush count loops the lead-in clip.
- Per-type branches of `motion_blow` sub 0 not listed in §4.7 (grab throws 0x4b–0x55, fan/stuck exits), and the meaning of `this+0x396` beyond "on floor".
- Guard/hit feedback on the player side: what Jack plays or sounds on a guarded cAtHit (`+2` set by `refHitPos`), and which effect ids `CreatePartsEff` 0xc / 0x12f map to in the port's effect packs.
- Which event `getFirstEventFrame` returns for clips whose first cue is not the hit (1104/1105 start with non-hit events at frame 0, so the reaction may fire at once).
- Chainsaw-cut kill and sever routing (`isDamageSever` `810128d4`, `calcDamageDead` `8101353c`, `damage_sever` 0x43) and the gun-bullet path (`onHitEmBullet` `8105d6f0`), which `decideDamageProc` leaves untouched for 0x60–0x65.
- Entry of player state 72 (charge) and the meaning of player `+0x398 == 0x2e` (states 89/90) → grunt cover_target 0x38.
