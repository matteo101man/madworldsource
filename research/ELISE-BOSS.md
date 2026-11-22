# Elise (cEm0630, room c303): port map

Goal: port the Elise boss fight to mw-play the way RinRin (cEm0440, room c201) was ported. This file maps her
identity, assets, native code, mechanics and intro, and gives a staged plan.

Tags: **[obs]** = read from disc data or decompiled code. **[inf]** = inferred from that code or data; confirm
before relying on it. **[unres]** = still open.
Disc root: `D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files` (written `files/` below).
Enemy-module addresses are analysis addresses (base 0x81000000, `tools/enemy-analysis.bin`). DOL addresses are guest
addresses in `sys/main.dol`.
Decompiled output (outside the repo):
`C:/Users/Matteo/AppData/Local/Temp/claude/C--Users-Matteo--claude-spares-spare-SC961L/b18b4d75-495f-4d48-a047-7b3fa286cef3/scratchpad/elise/`
- `em0630.a.c` / `em0630.s.c`: all 75 cEm0630 rows, annotated / with boilerplate stripped
- `em0330.a.c`: bats, cEm0330 / cEm0331
- `room303.c`: DOL 8015EC00..80161000, room c303
- `plelise.a.c`: DOL 802A03A0, the player's Elise clash
- `coffin.a.c`: DOL 80073F00..80075800, the roaring / coffin controllers
- `bossentry.c`: the DOL boss-entry functions that call the challenge prompt
- `misc.a.c`: helpers
- `annot.py`, `summ.py`, `strref.py`, `xref2.py`, `ptr.py`: the scanners used here

## 0. Summary

| | RinRin (ported) | Elise |
|---|---|---|
| Class / object id | cEm0440 / 0x20440 | **cEm0630 / 0x20630** [obs: em0630 record in c30300.ems, family 0; family base 0x20000 per ARENA-API] |
| Room | c201 (case2), spawn set c20110 | **c303** (case3/c303.dat), spawn set **c30300.ems** [obs] |
| Minions | none | **100 bats cEm0330** (object 0x20330, parent 0x20331), created in onInit 81101894 [obs]; **cEm0331** bat "mother" / path spawner [obs] |
| State table | 32 slots × (branch, update, end) | **17 slots × (branch, update)**, no end table: 0x81179D58 / 0x81179E24 [obs] |
| HP (normal / hard) | 1600 | **1000 / 1500** (em_common.ini / em_common_hard.ini `[em0630]` ;女吸血鬼 "female vampire") [obs] |
| Tuning CSV | em0440.csv | **em/em0630.csv** (11 rows, normal/hard columns) [obs] |
| Intro event | ev21 (1080 frames) | **ev33** (2350 frames, 15 bats + Elise) at room entry; **ev34** (970 frames, Elise) is her **death** cutscene, after the finisher, §2 [obs] |
| Challenge card | 8012409C(0) before ev21 | **no 8012409C call in the c303 room code** [obs]. No TAKE THE CHALLENGE? prompt was found for her [inf] |
| Clash | startKeenCompetition(pl, 0xB) in the slalom window | **startKeenCompetition(pl, this, 0x18)** when the player hits her **in her coffin** (hide state, sub 5), in startAtk 81102B14, §6 [obs] |
| Finisher | state 26 `finish` + 27 `finish_wait`, pairs 0121–0125 / pl0030_1305/1308 | only **13 `dead` → 14 `finish_wait`** (em0630_1006 loop). The player's generic boss catch (type 8: pl0030_0000 + startCatch 0x2F) turns her into a corpse; ev34 shows the kill, §7 [obs] |
| BGM | bs04 / bs04c | ev33 SEQ: EV33 → **BS04C at frame 2022**; ev34 SEQ: EV34 → **JINGLE05 at 635**; the ev33 handler calls 80124690(0x18 = bs04) [obs] |

## 1. Identity and assets

### 1.1 Character banks
| File | Contents [obs] |
|---|---|
| `files/em/em0630.dat` (1,347,136 B, 147 members) | `em0630.mdb` (121,568 B, **61 bones**, flags 0x2000 = white ink program); `em0630.tpl` + `em0630_AB.tpl` (218,528 B each); `em0630_00.emh` (3,584 B, damage volumes); `ragdoll.hkx`; **`em0630.eff`** (114,112 B); 62 MOT; 77 SEQ (`_00` / `_10`); empty `em0630_0020_{00,10}.set` |
| `files/em/em0630.csv` | boss parameter file (cBossParamFile at this+0xA30), §8 |
| `files/em/em0330.dat` (61,760 B) | bat: `em0330.mdb` (6 bones, duplicated at 640 and 33376), tpl/_AB, `em0330_0000.mot` (6 frames, wing flap), `em0330.eff` (5,152 B), `em0330_00.emh`, empty `em0330.ini` / `_pick.bin` |
| `files/em/em0331.dat` (21,312 B) | bat mother: `em0331.mdb` (5 bones), `em0331_0000.mot` (6 frames), a copy of `em0330.eff` |

The paired clips ship inside em0630.dat (as RinRin's ship in em0440.dat) [obs]:
- player: `pl0030_0460..0466`, `pl0030_c460..c467`
- chainsaw: `wp0600_0460..0463`, `wp0600_0465`, `wp0600_0466`
- borrowed from RinRin: `em0440_1003.mot` (70 frames, knockdown) and `em0440_1005.mot` (20 frames)

#### Elise motions (frames) and their roles in the code [obs; the role names are inferred from the code paths]

| Clip | Frames | Role |
|---|---|---|
| 0000 | 51 | hover loop (flying idle / flight end) |
| 0001 | 51 | stand (start, grounded idle, victory) |
| 0002 | 46 | land / crouch (air→ground, hide entry) |
| 0003 | 101 | take-off (ground→air, EST 1 on) |
| 0010 | 33 | walk (grounded move, victory approach) |
| 0011 / 0012 / 0013 / 0014 | 30 each | flight: forward (dot > 0.5) / left or right by cross sign / back (dot < −0.5) [inf] |
| 000a / 000b / 000c / 000e | 10 each | coffin poses. Crypts 8–10 use 000a; otherwise 000b/000c/000e at random |
| 0020 | 41 | guard |
| 0100 / 0101 | 81 / 81 | air attacks (bat commands) |
| 0102 / 0103 / 0104 / 0105 | 79 / 10 / 31 / 41 | Black Dragon: cast / loop / aim / recall; 0104 also starts victory |
| 0110 | 91 | somersault |
| 0111 | 66 | wing strike |
| 0200 | 62 | clash start |
| 0201 (+ 0202 additive) | 40 | clash struggle |
| 0203 | 178 | clash won by player (explorer's "Elise falls" pair) |
| 0204 | 202 | bite (player lost the clash) |
| 0205 | 201 | clash mid-win |
| 0206 | 66 | pulled from the coffin |
| 1000 / 1001 / 1002 | 30 each | hit reactions: 1000/1001 for side hits (by cross sign), 1002 when \|dot\| > 0.7 |
| 1004 | 66 | get-up |
| 1005 | 60 | death |
| 1006 | 51 | finish_wait loop |
| em0440_1003 | 70 | knockdown / burn |

Explorer labels: `exports/madworld/builders/labels.py` and `evidence/elise-motion-*.png`.

Motion cues (`research/motion-cues.json` already holds 35 em0630 sequences) [obs]:

| Op | Count | Meaning |
|---|---|---|
| 16 | 22 | effects |
| 29 | 27 | SE |
| 42 | 16 | voice |
| 37 | 14 | footsteps |
| 35 | 6 | rig flag; on 0110 at frames 20/26 and on 0111 at 14/19/28/34. These are the attack windows that set the hit-check bytes +0x2CD / +0xB35 read by attack / somersault [inf] |
| 19, 20, 33 | — | other |

### 1.2 Room c303 (`files/case3/c303.dat`, 1,564,096 B, 25 members) [obs]

**World**
- `c303.scr` (1,311,264 B): 106 parts, e.g. church_base, brige×N, arch, roof, window, tree, table, waterarch, bl003c, bl005c
- **100 of the 106 SCR transforms are non-identity** (61 rotated, 7 scaled). All of them are Y rotation with uniform scale, which `tools/stage_arena.py` (MW_ROOM=c303) stages; the native order is in §9.3 item 1
- `c303.eff` (35,232 B), `c303.imt` (pause map, 32,896 B), `cam00_Clip.cam` (11,584 B, one room clip), `c30300.caa`, `c30300.lgt`, `searea_c303.bin`
- empty: `c303.sed`, `c303_000_00.nav`, `AreaTbl.art`, `c303.cdz`, `c30300.sca`

**Collision:** `col.hkx` 44,448 B, `col_obj.hkx` 52,784 B, `col_pl.hkx` 36,944 B; `col_cm.hkx` and `col_ef.hkx` are empty.

**Spawns**
- **c30300.ems** (v20, 5 records):
  - Elise 0x630 / family 0 at (0, 0.77, −9)
  - 4 × 0x132 / family 3 at (−11.9, 0, 24.98), (13.66, 0, 25.76), (10.56, 0, −15.31), (−10.54, 0, −15.28). These are cBl0132 objects (family 3, id 0x70132), pickable stage objects (§5.3) [obs]. They are not a fire source: a held bl0132 strikes with 0xF6 [obs]
- **c30301.ems** (7 records): record 0 is id 0x0 at (−0.19, 1.63, 119.88); then 6 × em0330 bats at z ≈ 101, 73, 45, x ≈ ±5, y = −4.97

**Player points:** PlayerTbl.plp has 2 points, `start` (0, −2.2, 115, 180) and `Boss` (0, 0, 23, 180). The field order is [unres]; the c201 parser in research/stage201-player-points.json applies.

**Objects (ObjTbl.obt):**
- **bl0015_N001..008: the 8 coffins** (x = ±13.5, y 1.518, z = −10 / 0 / 10 / 20, yaw ∓π/2), the coffin controller's crypts 0–7, §5.4 [obs]
- bl003c_N001..006 at x = 0, y = −3, z = 42.9 / 70 / 98 (yaw 0 and π), along the bridge. The room's onEmEvent breaks them (§2). They are not the coffins, as an earlier reading had it [obs positions; role inf]
- bl0001_N001..003 (crypts 9 / 10 are N001 / N002); bl0130_N001 / N002 at (±15, 0, 5) (getCoffinState), §5.4
- bl0132_N001..006, bl0049_N001, bl052c_N001, bl005f_N001
- bl0001 / bl0015 / bl005c / bl005d / bl0073 / bl015e / bl015f / bl0130 / bl015a / bl0074 / bl0075

**Announcer:** `an_c303_info_{eng,fre,ita,spa}.bin` (eng 6,880 B).

**Room UI:** `files/case3/idpac_c303_{us,fr,it,sp}.dat` (there is no plain `idpac_c303.dat`).

### 1.3 Audio [obs]
**BGM table (805CB2C8):**

| Index | Track |
|---|---|
| 0x18 | bs04 |
| 0x1E | bs04c |
| 0x25 | jingle04 |
| 0x26 | jingle05 |

- Strings `BGM/ev33.rsd` and `BGM/ev34.rsd` sit at 805663B1 / 805663BE (pointer table 805CB3E0)
- ev33 BGM SEQ: frame 0 `EV33_RSD`, frame 2022 `BS04C_RSD` (vol 100)
- ev34 BGM SEQ: frame 0 `EV34_RSD`, frame 635 `JINGLE05_RSD`
- RinRin's ev21 switches to BS04C at frame 748. Elise likely fights to the same bs04/bs04c [inf]

**Announcer / boss voice:** `files/announce/eng/an_c303_pack_eng.bin` + `an_c303_info_eng.bin`. 26 ids (id:clips):

```
9c:4 9d:5 9e:5 9f:7 a0:3 a1:5 a2:3 a3:3 a4:3 a5:7 a6:3 a7:3 a8:4 a9:4 aa:7 ab:4
1183:4 1018:12 1143:9 1144:7 1145:8 1146:8 1147:2 1148:3 1022:1 1027:6
```

Elise's code requests (PlayVoiceStream 8026B558):

| Id | When |
|---|---|
| 0x9C | torch put away (checkExtinguish) |
| 0x9D | wing strike |
| 0x9F | answerFromSlave |
| 0xA0 | landing |
| 0xA3 | Black Dragon evade hint |
| 0xA4 / 0xA5 | strong / normal hit |
| 0xA7 | clash start |
| 0xA8 | clash won by player |
| 0xA9 | bite |
| 0xAB | player dead |
| 0xAC | going into the coffin. **Not in the c303 table**, so it plays nothing [obs] |

The c201 player ids 0x1117, 0x100A and 0x100B (idle fidget, giant swing) are **absent** from the c303 pack; 0x1144 and 0x1018 are present [obs].

**Event voice:** `files/evvostr/evvo33*.rsd` and `evvo34*.rsd` (VCE `JKSNN_EVVOSTR_EV33` / `_EV34`).

**SE banks**
- `files/se/scene_mix0303.rsp`: blocks 303, 1001, 1012, 16, 1106, 7080, 7081, **4209 (41 sounds)**, **8122 (32)**, 6200, 6230
- Block 4209 is Elise's bat swarm: twitterControl 81109A90 loops **4209000** (more than 99 bats out) and **4209001** (more than 50) [obs]
- RinRin's analogs are scene_mix0221.rsp blocks 4203 and 8117 [inf]
- Event SE: `files/se/event/scene_mix7547.rsp` (ev33 cues 7547000..), `scene_mix7548.rsp` (ev34 cues 7548000..). ev34 also plays the named cue `eSeVoBooing` [obs]

### 1.4 Events (`files/event/ev33_us.dat`, `ev34_us.dat`) [obs]
**ev33** (1,795,168 B)
- CAM: 15 cuts, frames 157, 89, 95, 95, 120, 59, 16, 24, 109, 120, 117, 220, 538, 222, 369 (total 2350)
- OBJ:
  - **em0330 × 15 instances** (cuts 4–14)
  - **em0630** (cuts 10–14)
  - pl0030 / pl0040 / wp0600 (cuts 1–14)
- other chunks:
  - ID: IDD, op 5 ×5 and op 16 WCD 24 / 74
  - EFCT: banks `pl0030_00`, `em0630_00`, `em0330_00`
  - BGM, VCE, SE (11 cues)

**ev34** (595,328 B)
- CAM: 11 cuts, total 970
- OBJ: em0630 (cuts 0–7 and 9); pl0030 / pl0040 / wp0600 (cuts 0–3, 5–10)
- other chunks:
  - ID: op 16 WCD 43 at frame 64
  - EFCT: includes `NONE` 120 at frame 633
  - BGM, VCE, SE (6 cues)

`tools/events/stage_intro.py` parses both containers unchanged (EVNT → CAM / OBJ / ID / EFCT / BGM / VCE / SE).

## 2. Room flow and intro (DOL)

**Room constructor fn_8015FF7C** (class `room::c303::RoomMode_C303::cMainMode`, vtable 805A6F68, RTTI 805481DF) [obs]:
- registers room events: +0xE8 id −89, +0x110 id −88, +0x138 id 1, +0x160 id 0, plus a +0x8C handler
- sets 805A445C = 0xB14
- places bl0049 and loads **c30301.ems** (bats) through 80124E04
- creates **cBoss** (fn_801600AC → fn_80160108, RTTI 80548212 `...::cBoss`, vtable 805A6FA0), which registers callback **fn_80160238** through 80123470 (key 0x10)
- the event member-pointer table is at 805A6ED0..805A6F14

**Room event handlers** [obs]:

| Event | Handler | Action |
|---|---|---|
| 0 | **fn_80160870** | `80123820(0x33)` = **ev33**; 80124CE4(1, 0); places bl0049 / bl052c; kills effect 0; **80124690(0x18, 0)** (bs04) [inf: queues the room BGM]; **8007515C** creates the coffin controller (fn_800751BC, §5.4) |
| 1 | fn_80160838 | message `roaringOver` |
| −89 | fn_80160A2C | message `roaringStart`; 801246D8(0, 0); 802D3CB4(0, −1) |
| −88 | fn_801609C4 | 80123FB8 / 80124784 (event / BGM restore [inf]) |
| +0x8C | 80155C90 | generic |

The member `onEmEvent` (fn_80160A7C(this, "onEmEvent", fn_80160C44)) handles enemy messages 1..6. It sends `break` to `bl003c_N%03x`, with N picked from the table at 805480B8 = {3, 6, 2, 5, 1, 4} [obs].

**cBoss callback fn_80160238 → fn_80160278** [obs]:
1. Loads `c303.dat\c30300.ems` and spawns it through **80124DDC**. This is the only Elise spawn site in the DOL: c30300.ems has exactly one string reference, at 8016029C.
2. **80073FD8** creates the "roaring" controller fn_80074038 (messages `roaringStart` / `roaringOver` / `getRoaringOrder`). Its getRoaringOrder answers the bat em0330_r1_demofly.
3. 80133AA4.
4. **`80123820(0x34)` = ev34.**
5. 80126098("bossObj"); 800A7190(80607078, 1).

**Event ids:** 80123820 is the blocking event player [obs].

| Caller | Event | Room |
|---|---|---|
| 80147BBC | 0x21 = ev21 | RinRin |
| 80160870 | 0x33 = ev33 | c303 |
| 80160278 | 0x34 = ev34 | c303 |

The full id → caller list is in the call-site scan (xref2.py on 80123820).

**Challenge prompt:** 8012409C is called only by 80133898 (ev 0x10), 801437A4 (0x16), 80147BBC (0x21, RinRin), 8014F738 (0x22), 80158310 (0x31, c301 boss), 8015E594 (0x32, c302 boss) and 80164734 (0x41) [obs]. None of these is in the c303 cluster, so **Elise has no challenge prompt in her own room** [obs]. Whether c302 → c303 shows one is [unres].

**Order (resolved 2026-11-11, static) [obs].** ev33 is the room entry and ev34 is her death. fn_80160278 runs straight through:
1. 80124DDC spawns c30300.ems (Elise and the four bl0132)
2. 80073FD8 creates the roaring controller
3. **80133AA4(this)** is a frame-yield loop (8021EDF0) that runs until 80133BF0 is true. That happens when the boss object's +0x3A is set (by the `onBossDead` listener 80133F6C) or when no boss-class enemy is left (801E0C0C / 801EAA84 on the enemy list 80694A84). See PLAYER-DEATH-API.md "Victory chain".
4. BeginDemoMode 80086C9C(0), 80124190, fade 80123CB0, 80123B8C(0)
5. **80123820(0x34) = ev34**
6. 80126098("bossObj"), 800A7190(80607078, 1), 80123B8C(0xFF), end of demo, then `subMission` / `roomClear` (8005B674(805A4DC2 / 805A4DB8)), as in RinRin's fiber.

So ev34 cannot play before she is dead: it is the defeat cutscene, ending on JINGLE05 (the clear jingle). The Baron's fiber has the same shape: the wait, then ev51 (BARON-LEVEL §2). ev33 is room event 0 (fn_80160870): it plays ev33, then bs04 (80124690(0x18)), the room effects and the coffin controller 8007515C. No live trace was needed.

**Port:** play_intro.rs plays ev33 at entry. It plays ev34 as the outro once `fight.victory` is set, through the same path as the Baron's ev51.

**After ev34: roomClear (resolved 2026-11-11)** [obs]:
- cRoomEx's roomClear handler 8012896C sets stop flags 9 / 10 (8005A82C on 80697E64) and queues the clear task 801289DC through 80123470 (key 0x10).
- 801289DC runs **80124258**, then 800A7394(80607078, 0), then the scene change 80181BB8(80653EB4, 805A445C = **0xB14**, set by the room constructor).
- 80124258 runs in demo mode:
  - **80124708(0x1027)**: announcer line 0x1027 (8026C0C4; in the c303 pack, 6 clips).
  - **801246E8(0x26)** = PlayBGM_EX(jingle05). PlayBGM_EX 8026A108 returns at once for 0x26 when the current room (8069BCDC) is 0x107, **0x303** or 0x403; it plays only in 0x203, and stops the music (StopBGM 6) anywhere else. So in c303 the clear requests no new music: **ev34's own JINGLE05 (its BGM SEQ, frame 635) plays on**, and the room's bs04 / bs04c does not come back.
  - the clear board (80244D54 / 80244D70), then the stage-clear results (80124018), both decoded below.
**The room-clear windows (decoded 2026-11-11)** [obs unless marked]:

The HUD manager *0x8069E2C0 is set up by 80244A80. Its widget slots come from the UI factory 802233A8(8069D11C, id), which looks the id up in the table at 8059C598 (0x8B rows: id, then a creator member pointer).

| HUD slot | Layout widget id | Class |
|---|---|---|
| +0x14 | 1 | |
| +0x18 | **2** | **cIdClearBoard** (creator 800D7054, 0x20 bytes, vtable 805CA780) |
| +0x28 | **0xD** | **cIdStageClear** (creator 800D549C, 0x6C bytes, vtable 805CA6C0) |
| +0x2C / +0x30 | 0xB / 0xC | |
| +0x40 | 0x7A | |
| +0x48 | 0x13 | the TV-static wipe (PC-DEATH-CINEMATIC.md) |

Programs are created by 8024B81C(0x8069D49C, &handle, 0, resource, program, layer, flags), where resource 0 = core_us and resource 1 = the room's idpac (GRAB-THROW-MINIMAP.md).

**1. Clear board: 80244D54 / 80244D70**
- **Start (80244D54):** on slot +0x18 it sets byte 0 = 1, +0x10 = 0, +0x18 = 0 and +0x14 = −1.
- **Update (cIdClearBoard 80240AB4):** with byte 0 set and no handle, it creates **core_us (resource 0) program 10 on layer 0xD**. It then:
  - takes +0x1C = 8024BEBC(handle, 1), the program's layout widget id 1;
  - plays **SE 0x57** (80223150 → 80267AA0(…, 0x57, 0x7F));
    - 80267AA0 [obs]: unless flag 80697E64 bit 0x13 / 80697E8C bit 0 is set and id != 0x98968B, it hands **id % 10000000** to the RedSpark player 80503378 (volume 0x40 x 0x7F x mixer +0x1E40), on mixer 0x806A1D6C (80223150's constant). So the board is SE 87 and the results 10000036 is SE 36. [unres: which staged pack those ids resolve in: scene_mix0000 block 0 has 36 but no 87; the port plays neither yet]
  - shows the program (8024C184);
  - clears byte 0.
- **Program 10** has 28 widgets:
  - id 2 at (0, 0), id 21 / 20 at (±160, 0), id 11 / 10 at (0, ±90), id 1 at (0, 0);
  - under id 1: a 4-piece frame (UV group 9 #12, ±150 × ±100), a panel (9 #18, 180 × 115), and two copies (shadow and face) of six 48 × 64 letters from UV group 62: indices 22, 8, 13, 13, 4, 17 left to right.
  - [inf] Group 62 is an alphabet, so the letters read **"WINNER"**.
- **Wait (80244D70)** returns 1 while byte 0 ≠ 0, or while 80240B88 holds. 80240B88 holds while handle +4 ≥ 0, +0x1C ≠ 0 and `(*(+0x1C)->+0x1A0)->+0xC0` bit 3 (0x8) is set, meaning the program's widget-1 animation is still playing.
  - Once the bit clears, it calls 802228B4 (release the program, handle −1, byte 0 = 0), sets +0x18 = 1 and returns 0.
  - **No input is involved:** the board stays up exactly as long as program 10's widget-1 animation.

**2. "Fade" 80124018: actually the stage-clear results screen, not a fade**
- **Start:** 80124018 calls **8024523C**(ui, 1, *(u32 *)0x80697F04, (int)*(f32 *)0x8069BD00) on slot +0x28 (cIdStageClear). That sets byte 0 = 1, +0x19 = 1, **+0x10 = *0x80697F04**, **+0x14 = (int)*0x8069BD00**.
- **Wait:** it loops until **80245250** sees byte 0 = 0, then releases the slot (80245294 → 80223430).
- **Update (cIdStageClear 802416B0):**
  - **Program choice:** in rooms 0x107 / 0x203 / **0x303** / 0x403 / 0x503 it uses **program 31 (0x1F)**, else 30. Either way it is **resource 1 = the room's idpac** (case3/idpac_c303_us.dat has programs 4, 5, 6, 8, 11, 12, 30, 31, 50), on layer 0xD. It also calls 800A7190(80607078, 1).
  - **Sound and numbers:** it plays **SE 10000036 (0x9896A4)**, then fills the digit widgets (80222E40(widget, value, first id, digits)):

| Widget ids | Digits | Value |
|---|---|---|
| 0xB | 9 | the total (802426F8) |
| **0x15** | 7 | **+0x10 = *0x80697F04** |
| 0x1F | 5 | a tally counted up below |
| 0xC9 | 7 | a tally counted up below |
| 0x3D | 5 | *80697F04 / *80697F18 |
| 0x79 / 0x7E / 0x83 / 0x88 / 0x8D | 3 each | *80697F24 / F28 / F2C / F30 / F34 |
| **0x4B / 0x49 / 0x47** | | **+0x14** split (80222CA4) |
| 0x2E–0x35 | | rank / count digits from 8024258C, the stage table (80189B2C / 80189D00) and *8061688C |

  - [inf] *80697F04 is the stage score, *8069BD00 the play time, and *80697F24..F34 the kill-method counters. The globals' names are not read from code.
  - It also updates cSaveData: 8017FDC0 playtime, 80127970 copy, and a per-room flag in 80616C50 (none for 0x303).
- **Its steps** (byte +0x18):

| Step | Wait |
|---|---|
| 0 | layout id 4's animation ends ((+0x1A0)->+0xC0 bit 3) |
| 1 | id 0xB9's animation ends (+0x90 bit 3) |
| 2 | a 60-frame count-up of the tallies (8023356C), **skipped by cPad down bit 0x10** (0x80698B3C or 0x8069CF74) |
| 3 | shows id 0x14 if 8069D30A is set |
| 4..0x1E | one frame each (27 frames) |
| **0x1F** | **waits for cPad down bit 0x10**, then 802230F0 and shows layout id 2 (the 4-bar wipe, w196) |
| 0x20 | id 2's animation ends |
| 0x21 | the save system is idle (80213C6C(8069BC80) = 0; the screen requested a save through 80213D40), then byte 0 = 0 |

  - **The input:** cPad down bit 0x10. Bit 0x20 is Remote B (native_controls.rs, 8020EFC0), so [inf] 0x10 is Remote A, the confirm button.
- **No fade colour or length exists here** [obs]. The slot +0x28 fields are the score and time, not fade parameters.

**3. Scene 0xB14**
- The scene table at 805A3B84.. has 0x14-byte rows (flags, debug name, id, group, creator). **0xB14 = "Scene13"** (row 805A3EA4, group 0xA03, creator 80120BB0).
- Its event scene (ctor 80120BE4 → 80188054(this, **event 0x84**, **next 0xE01**); run by 801881C8 → 80123ADC) plays **ev84** (files/event/ev84_us.dat, 7.3 MB), then changes to **0xE01** ("Game Title" in the debug table).
- Neighbours, for context:
  - 0xB13 "Scene12" = ev83 → room 0x303 (the story scene before Elise's room, [inf]);
  - 0xB03 "Scene02" = ev73 → 0x105;
  - 0xB02 "Scene01" = ev72 → 0x104;
  - 0xB01 "Ending" = ev71 → 0xE01.
- The native c201 clear ran scenes e01 → b03 → ev73 → 105 (evidence/input-hle-finisher-x.log 1777–2463) [obs]. So the room change passes through 0xE01 first, and the story scene then plays its event and loads the next stage.
- **So Elise's clear leads to the story scene with ev84, then on to 0xE01** [obs table and ctor; that ev84 is the post-Elise story cutscene is inf].

- **Port:** the outro already drops the room track (`bgm.track = None`) and lets JINGLE05 play out; nothing restarts it. play_elise.rs Finisher pushes voice 0x1027 once the outro has ended (`Intro::outro_pending`), and tools/audio/stage_announce.py MW_ROOM=303 now stages 0x1027. The clear window and the scene change to 0xB14 are not ported.

## 3. Native state table (cEm0630, onMove 81102560 calls both tables through __ptmf_scall) [obs]

The state id is the rno byte at this+0x11B; the sub-state is this+0x11C. There are no end callbacks: changeState is the base class's.

| Id | State | Branch / update | Summary |
|---|---|---|---|
| 0 | start | 81103168 / 811031F4 | stand 0001; branch → air (2) when the target is within 12.5 m (sq 156.25) |
| 1 | idle | 811032B4 / 81103468 | ground: 0001. Air: 0000 hover with a random jkSpline wander, 3 m steps; arena bins x∈[−3, 3], z∈[−8, 23] steer the heading. Branch: chkAtkStart → attack; hide threshold → hide; grounded: rebirth timer < 0 → air, dist² > 25 → move; flying with < 31 bats → bat cmd 4 + air (land) |
| 2 | air | 81103CBC / 81103CC0 | toggles the flying flag +0x1314. Take-off 0003 + EST 1, or landing 0002 (kills EST 1). At landing frame 40: voice 0xA0, all 100 bats cmd 2, bat count +0x1A54 = 100, rebirth = decideRebirthTime, staggerForce(2) |
| 3 | move | 81103EEC / 81104014 | ground: walk 0010 (branch: dist² < 4 → idle; < 16 → chkAtkStart). Air: spline flight 3–6 m at random, eased by sin(2t − t³), 0011–0014 by direction |
| 4 | attack | 8110452C / 81104530 | §4 |
| 5 | somersault | 81104AF4 / 81104AF8 | §4 |
| 6 | annihilate | 81104EF0 / 81104EF4 | 0102 → 0103 → 0104 → 0105 → idle. **No setRno(…, 6) in em0630 code: unreachable** [obs] |
| 7 | blackdragon | 811050DC / 811050E0 | §5.2 (1,000+ lines) |
| 8 | guard | 81106B50 / 81106BFC | 0020; guard-cancel counter > decideGuardCancel → somersault (cancel variant); motion end → idle. Branch: hide threshold → hide |
| 9 | damage | 81106D44 / 81106DF0 | the reaction motion is set in onPreMove; end → idle |
| 10 | down | 81106E3C / 81106E40 | em0440_1003 → (lands if flying) 1004; at frame ≥ 20 in the air: re-summon the bats (cmd 2, count 100, rebirth, stagger) → idle |
| 11 | burn | 81107090 / 8110713C | on hit type 0x11A: em0440_1003; parts effects 0x104 / 0x105; bat cmd 4; **HP −1 per frame while burning (never below 1)**; rebirth −50; → down |
| 12 | hide | 81107310 / 81107314 | coffin, §5.4 |
| 13 | dead | 81107C88 / 81107C8C | HP < 1: 1005, bat cmd 4, unregistHit, HP pinned to 1, player hit-stop (0.001, 0x37, 0x14), pad vibration, camera quake (0.1, 0.9, 0.2, 1.5, 15) → finish_wait |
| 14 | finish_wait | 81107E70 / 81107E74 | condition flags 0 and 3 on (+0x3C0), unregistHit, HP = 1, 1006 loop |
| 15 | victory | 81107F58 / 81107FB8 | player HP < 1 (onMove; voice 0xAB): 0104 → walk 0010 to the player until dist² < 9 → 0001. Branch: the player alive again (Continue) → sub 5, 40-frame countdown → idle |
| 16 | keen_compe | 81108208 / 8110820C | clash, §6 |

**onPreMove 81101E88 (damage intake)** [obs]:
- skipped when HP is 0, guard-cancel is set, in hide, or with no hit
- ATHIT types 0x2A..0x36 count as "strong" (voice 0xA4, else 0xA5)
- damage × 1.2 when in attack (4) / somersault (5) or hit from the side the dot test selects (> 0.7); × 0.8 when dot < −0.7 and not strong
- HP (+0xEC, s16) −= dmg; the hide accumulator +0x1364 += raw damage; the attack timer +0xE48 −= dmg (taking damage brings her next attack sooner)
- in guard (8): chip = dmg × csv[2] %, at least 1, never lethal; parts effect 0xC
- type 0x11A → burn (11), accumulator += dmg × 10
- type 0xF6 → player SetDropObjPl (the held object breaks) [inf]
- reaction (state 9, 1000/1001/1002) unless in attack/somersault below frame 40/60 (type 0x119 always reacts)
- HP < 1 → dead (13)
- every damage calls LevelFlow(dmg), §8

**onMove 81102560** [obs]:
- runs both tables
- rebirth timer +0x1358 −= speed (× 0.1 while the player is within 5 m)
- bobbing height (+0x68) while flying, with take-off / landing / dragon ramps
- checkExtinguish
- player dead → bat cmd 11 if in the dragon, victory (15)
- twitterControl
- moveNeck, root motion, checkFloorLanding(0.05)
- `setBossHpGage(maxHP, HP)` every frame (the boss31 HUD the port already has)

## 4. Attacks [obs]

Damage comes from the boss attack table (vtable +0x470, cEmBoss::getAtkPower 81006300 [inf]) indexed 1..5, i.e. em_common.ini `ボス攻撃1..5`. ATHIT types are the `setAtkHitParam` / `setAtkType` arguments.

| Attack | ini idx (N / H) | Motion | Hit | Notes |
|---|---|---|---|---|
| Wing first strike 翼初撃 | 1 (10 / 20) | 0111 (ground), voice 0x9D | part 0x25, radius 0.5, ATHIT **0x172** (level ≤ 40) / **0x141** (level > 40); **0x151** if player HP ≤ power | frames < 25 |
| Wing second strike 翼次撃 | 2 (10 / 20) | 0111 frames ≥ 25 | part 0x1F, 0x151 | if 0111 ends and the player is not in flag 0x1000 (pl+0x1128) → somersault, else idle |
| Somersault サマーソルト | 3 (10 / 20) | 0110 | parts 0x12 + 0x16 (both feet), radius 0.7, ATHIT 0x155 | starts at frame 15 when guard-cancelling or at random when L³ > U |
| Somersault, guard cancel | 4 (10 / 20) | 0110 from guard | ATHIT **0x18E** if player HP > power(4), else 0x155 | guard-cancel flag +0x136C cleared at frame 40 |
| Air volley | — (bats) | 0100 or 0101 (50 / 50) | bat cmd 5, then cmd 6 at 0100 frame 40 / cmd 7 at 0101 frame 35 | bat dive: sphere 0.3, ATHIT 0x172 / 0x173 at random, bat power hard-coded **10** (5 in another command) (em0330 810B4B3C, commandFromMistress 810B5E3C) [inf mapping] |
| Black Dragon コウモリ集団 | 5 (20 / 30) | 0102 → 0103 → 0104 → 0105 | ATHIT **0x15B**, power(5) (scaled when twin-headed), sphere **0.6** (**0.3** while the player swings a torch) | §5.2 |

**Attack choice:**
- the attack timer +0xE48 counts down by scene speed; at ≤ 0, +0xE4C (attack count) += 1 and the timer resets (chkAtkStart 811030F0)
- `decideAttackInterval` 811095A8: U[20(1−L) + 30(1−L), 100(1.2−L)] frames, + 20L + L²·100(1.2−L) while flying, with L = level/100
- in the air, Black Dragon is chosen once the attack count ≥ `decideBlackDragonInterval` 81109550 (1 / 2 / 3 / 4 for L > 0.6 / 0.4 / 0.2 / else)

**Guard entry (startAtk 81102B14, when the player swings at her):**
- idle / move, player facing her (dot < −0.25, or random): → guard (8) + EST 2
- already in guard: stays when dot < −(L − 1); hit counter += 1..4 at random for strong types
- attack / somersault early (frame ≤ 15L²): → guard
- down (sub > 1): guard with chance L
- flying: → move (3), i.e. dodge
- in hide sub 5: the clash, §6
- `decideGuardCancel` 81109764: 4..8 (L < 0.2), 3..5 (< 0.5), 2 (< 0.8), 1

## 5. Special mechanics

### 5.1 Flight
- The flag +0x1314 flips on every air (2) state [obs].
- Height and paths use two `jkSpline` objects of 2 points each (+0x1318), created by 802547B4 and read by getPoint 80254BE4 / getTangent 80254DD0 [obs].
- `decideRebirthTime` 81109534 gives U[150, 300] frames until the next take-off ("rebirth") [obs].
- **The port has no spline type: jkSpline needs translating** (create / clear / getPoint / getTangent).

### 5.2 Bats and Black Dragon [obs]
- onInit creates **100** cEm0330 bats (CreateObj 8026FE6C (0x20330, …, 0x20331)), each through **setMistress** 810B57E0, stored at +0x13C4[100].
- commandToSlave 81108748 / commandFromMistress 810B5E3C use commands 1, 2, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14.
- Bat states (table 0x81172308 / 0x81172374):

| Id | State |
|---|---|
| 0 | wait |
| 1 | idle |
| 2 | path |
| 3 | dead |
| 4 | ready |
| 5 | attack |
| 6 | blackdragon |
| 7 | backstab |
| 8 | demofly |

- Command → bat state [obs]: 1 → idle; 4 → path (recall to the mother cEm0331); 5 / 8 / 12 → ready; 6 / 7 → attack; 10 → blackdragon (only from 5); 14 → backstab (hint bat) [inf naming].
- The bat answers through answerFromSlave 81108C20 (voice 0x9F).
- Bats are light objects: their own draw (@2608@draw 810B725C), a 6-frame flap, no skeleton fight logic.
- cEm0331 (810B6968..) spawns bats along its seq paths (getSeqList / getSeqInfo) for the c30301 entrance and the demo fly (setDemoFly, getRoaringOrder).

Black Dragon sub-states (811050E0) [obs]:

| Sub | Action |
|---|---|
| 0 | 0102 + bat cmd 8 + player SetLockOn_Trg2 |
| 1 | kill EST 0x2A |
| 2 | rise; at the end → 0103, twin head = `decideTwinHead` (U < 2L − 0.3), loops = `decideLoopNum` (min(5, 6(0.6 + 0.4·U[0, L]))) |
| 3 | 0104; aiming = `decideAimingRatio` (0.1(1−L) + 0.6(1−L²)); 1 or 2 heads get random spline speeds 2–4; staggerForce(2) |
| 4 | the heads chase the player along splines (+0x1A58 / +0x1B88, 4 points each). Each head is an attack sphere: setAtkType 0x15B power(5), setAtSphereVec, registAtWk, and a Havok hitAttackSphere. The aiming gain rises by 0.01 × player-flag and 0.02 × flamePhobia |
| 5 | recall 0105 + cmd 11 / 1 |
| 6 | → idle |

**At her death (resolved 2026-11-11)** [obs]: dead 81107C8C sub 0 sends **cmd 4** to all 100 bats. Hide sub 1 sends it too, so after a clash win straight into finish_wait the swarm is already gone.
- Cmd 4 (commandFromMistress 810B5E3C): mode +0xA44 = 4, setRno path (2), spline speed +0xA64 = 0.03.
  - The jkSpline runs from the bat's position to position + Rot(order 5; yaw U[−π, π], pitch −U[0, π/4]) · (0, 0, 20): 20 m out and climbing up to 45°. Its tangents are 0.5 × that offset.
- Path 810B4588: the progress advances 0.03 × the speed rate per frame, and the bat lerps 0.1 toward the spline point.
- At the end, PathOver 810B6690 (mode 4) sets y = −1 and setRno dead (3), so it sinks out of view. Only cmd 2 (landing / rebirth) brings bats back, and that never comes after her death.
- **Port:** play_elise.rs `scatter` / `BatSt::Scatter`. They fire on Dead, FinishWait or Finisher. Before, the swarm kept orbiting.

Abort the dragon when:
- fewer than 50 bats remain → cmd 11 / 4 → air
- or she has lost ≥ 70 + 30L HP since it started → she falls (down, flying cleared)

`getPlEvadeDir` 8110924C returns the evade hints 98/99 (+ voice 0xA3) when a head is within 3.75 m and its spline progress is in (0.7, 1).

### 5.3 Fire (torch) [obs; the weapon meaning is inferred]
- `flamePhobia` 811093F8 is true when the player's arm type (cPl0000::getArmType, inlined at 81109490) is **7** and isNowAttackFrm (8110945C) holds.
- It halves the dragon head radius (0.6 → 0.3) and adds to the aiming gain.
- `checkExtinguish` 811094A4 plays voice 0x9C when arm type 7 is dropped.
- Hit type **0x11A** sets her on fire (state burn).
- **Arm type 7 (resolved 2026-11-11)** [obs]:
- The arm table starts at +0x1504, with 0x18 bytes per slot. The slot index is the s16 at +0x1500. Each slot holds the type at +4 (+0x1508) and the weapon id at +8 (+0x150C). getArmType 81109490 = `*(pl + 0x1508 + 0x18 * pl[+0x1500])`.
- **fn_802725AC maps a weapon id to its arm type:**
  - 0x305xx → 2, except **0x30580 / 0x30582 → 7**
  - 0x306xx → 6, except **0x30690 → 7**, 0x30695 → 8, 0x30605 → 0xB, 0x30606 → 0xC, 0x30610 → 10
  - 0x30730 → 0 (fists, the default slot), 0x30740 → 2, 0x30760 → 3
  - **0x30801 → 7**
  - anything else, every bl object included → 2
- So type 7 is the weapon set **cWp0580, cWp0582, cWp0690, cWp0801** (wp/wp0580.dat … wp0801.dat). wp0580, wp0582 and wp0801 each carry an .eff; wp0690 is a wp0600 chainsaw variant with pl0030_1240..1243.
- The type-7 hit reaction (fn_802788F0) is rumble 0x2D and hit SE 0x14 (types 2 / 3 / 8 use 0x13).

**Fire hit types** [obs]. Row names are from the ATHIT table (main.dol 805588B4); the rows themselves are in plem_common.ini.

| ATHIT | Row | Power | Set by |
|---|---|---|---|
| 0x119 | FIRE_ATTACK | 60 | the state-27 throw 80282B38 (@80282D24) and the pl0030_050x attack 802AAB18 (@802AAF64) |
| **0x11A** | **FIRE_ATTACK2** | 60 | (1) the held-object strike 80283B70 when the held object (+0x1260) is **0x70042 (bl0042)**, @802840FC. Otherwise it sets 0xF6, or 0xF8 in sub 5 or with flag 0x100000. (2) The **wp0801 attack** 802BAA70 (pl0030_1427, @802BB18C): +0x1284 = 0x11A, +0x128C = 50, +0x1288 = 3 |
| 0x11B | FIRE_ATTACK3 | 60 | 802AAB18 |
| 0xF6 | OBJ_COVER | 10 | the held-object strike 80283B70, any other held object |

Elise reacts to 0x11A with burn and to 0xF6 by dropping the held object (SetDropObjPl). 0x119 only forces a reaction.

**bl0132** [obs]:
- Class **cBl0132**, a cCatchScrObj (base ctor 801CF070). Vtable 805AADB0, RTTI 805AAEA8. Object factory 805A1E50 → 801185CC (0x1BD0 bytes), ctor 80118664.
- bl/bl0132.dat holds an mdb, tpl, col.hkx (47,808 B), eff, sed and a `PICK` pick.bin (version 2, the same layout as bl0063's). It is a pickable stage object.
- Its vtable +0x30 hook 80198738 runs on its first pickup (+0x1652): it sets cSaveData flag 0x14 and, if the player holds it, plays announcer line **0x1148** (in the c303 pack).
- As a bl object it maps to arm type 2, not 7, and its strike is 0xF6, not 0x11A (802725AC / 80283B70).

**The fire source is absent from c303 (second pass, 2026-11-11)** [obs]:
- No member of case3/c303.dat contains the strings bl0042 / wp0580 / wp0582 / wp0690 / wp0801 or the ids 0x30801 / 0x70042.
- The spawn sets hold only Elise (0x630) and four 0x132 (c30300), and 0x0 plus six 0x330 bats (c30301).
- The c303 room code (8015EC00..80161000) loads none of the five ids (lis 3 / 7 + addi scan).
- Every DOL load of them (same scan), with what it is:

| Id | Where |
|---|---|
| 0x30580 | 80064D30 / 80065A90 / 80066D68 / 80067328: Bloodbath-challenge code (onBeginBbc / onEndBbc, c105.dat\c10504.ems) |
| 0x30690 | 80067EC0 / 80068F8C (a BBC with c301.dat\c3010a.ems and bl0541); 80156ABC = `room::c301::cTips`, an onEquipWeapon tip for 0x30690 |
| 0x30582 | 8012FB50 = `room::c103::cTips` (onEquipWeapon 0x30582); 8016985C / 80169BC0 |
| 0x30801 | only the arm table 802724B4 / 802725AC and its own attack 802BAA70 |
| 0x70042 | 801CE064 (generic catch-object code); the enemy module only in cEmZako::onInit 8100B958 (@8100CE14): grunts carry bl0042. bl0042 is a cRoomCatchScrObj (factory 805A1660 → 80119BB4, ctor 8017E5DC) |

- So Elise's flamePhobia / burn reactions have no fire source in her own room: no type-7 weapon, no bl0042, no grunts.
- The nearest one is c301, the same case3 stage: its BBC and its tip both use 0x30690 (wp0690, a wp0600 chainsaw variant). Whether that arm survives the room change into c303 is [unres]; a native c301 → c303 run would show it.
- **The port has no player arm type 7 and no burn, and needs none for c303 as it stands.**

### 5.4 Hide (coffins) [obs]
- **Trigger:** every time the accumulated raw damage reaches (n + 1) × **csv[0] = 250**, the idle / guard / damage / burn branches send her to hide (12), and n += 1.
- **Sub 0:**
  - 0002 (on the ground, jump to the last 5 frames)
  - bat cmd 13 + staggerForce, or in the air cmd 12 + player lock-on
  - kill EST 1 / 2
  - the cloth mesh flags +0xEF4 / +0xFB8 / +0x107C / +0x1140 |= 1 (hidden)
- **Sub 1 at motion end:**
  - bat cmd 4, collision off
  - `SendMsg("getCrypticPoint", pos, dir)` returns a crypt index 0..10 and her position / direction
  - coffin pose: 000a for 8–10, else 000b / 000c / 000e at random
  - voice 0xAC
- **Sub 5 (in the coffin):**
  - `getCoffinState` → bat cmd 14 (the hint bats fly to her coffin)
  - **heal:** +1 HP whenever the timer passes 30 × csv[4] frames, then it steps back by 30 × csv[5]: csv[4] (start) 5 s N / 1 s H, csv[5] (interval) 1 s. LevelFlow(−1) per HP
  - **hint bat** every 30 × csv[3] frames (5 s N / 10 s H)
- **Answered by the DOL coffin controller fn_800751BC** (allocated 0xA8 by 8007515C): it handles `getCrypticPoint`, `getCoffinState` and `hideOver`.
- **Coffin controller (resolved 2026-11-11)** [obs]:
  - fn_800751BC (vtable 80596964) registers four handlers through the member pointers at 805968C8...
  - It resolves 23 objects (801255C8) into +0x3C..+0x94: bl0015_N001..N008 (+0x3C..+0x58), bl006f_N001, bl0001_N001 / N002 (+0x60 / +0x64), bl005f_N001, bl0074_N001, bl005c_N001..N008 and bl0130_N001 / N002 (+0x90 / +0x94).
  - It then sets +0x98 (current object) = 0, +0x9C = 0, +0xA0 = 1 and +0xA4 (used bits) = 0.
- **getCrypticPoint 80075C0C** writes her position (+0x64) and rotation (+0x70) and returns the crypt index. The table is inline code (floats 80530028..80530068):

| Crypt | Object | Position | Yaw |
|---|---|---|---|
| 0–3 | bl0015_N001..N004 | (13.5, 1.5182, −10 / 0 / 10 / 20) | −π/2 |
| 4–7 | bl0015_N005..N008 | (−13.5, 1.5182, −10 / 0 / 10 / 20) | +π/2 |
| 8 | none | (0, 5.6, −20) | 0 |
| 9 | bl0001_N001 | (15.6857, 3.945, −15) | −π/2 |
| 10 | bl0001_N002 | (−15.6857, 3.945, −15) | +π/2 |

  - Crypts 0–7 match the bl0015 ObjTbl transforms exactly, so **bl0015 = the coffins**. In ObjTbl, bl0001_N001 / N002 sit at (±15.886, 5.775, −15.005).
  - **The pick:** r = getU16(0, 300).
    - If +0x9C < 2: r % 11 when +0xA0 = 0, else r & 7.
    - If +0x9C ≥ 2: r % 3 + 8.
    - After a coffin (< 8): +0xA0 = 0 and +0x9C += 1. After any other crypt: +0x9C = 0 and +0xA0 += 1.
    - So her first hide is always a coffin, and she never picks three coffins in a row.
  - The chosen object's +0x1A0 bit 2 is cleared and the object is stored in +0x98 (none for crypt 8) [obs; what the bit means is inf].
- **getStrugglePoint 80075884**(pos, rot, n) copies bl0015 #n's +0x64 / +0x70. Nothing in the DOL or in em0630 sends it [obs].
- **getCoffinState 800756B4** takes the first bl0130 (N001 or N002, bits 21 / 22 of +0xA4) that is not yet used and passes 801BE974. It marks that object, returns its world position and rotation, and returns 1; otherwise it returns 0.
  - Elise's hide sub 5 sends it every frame and, on 1, sends bat cmd 14 to that point.
  - Each bl0130 answers once per fight [obs; what bl0130 is, inf].
- **hideOver 800756A8** sets +0x98 = 0.
- **The coffins themselves (resolved 2026-11-11)** [obs]:
  - bl0015 (and bl0001) are created by the object factory entry 805A13C0 / 805A1280 → 80119F78: a plain **cBaseScrObj** (ctor 801B6E90, vtable 805ADBC8, 0x1A98 bytes), with no class of its own.
  - bl/bl0015.dat has no motions at all: bl0015.mdb (65,088 B), bl0015.tpl, col.hkx (43,440 B) and bl0015.eff (1,600 B). bl0015.ini, .sed and _pick.bin are empty. **So no coffin open / break / struck animation exists**. Her entry and exit are her own clips: 0002 down, then 000a / 000b / 000c / 000e, and 0206 when she is pulled out (with the player's pl0030_0466).
  - **+0x1A0 bit 2** is a generic cObj flag. Its setter 8017F574(obj, on) sets it when `on` and clears it otherwise; it sits in the cBaseScrObj-family vtables (cBl0132 slot 46). The event commands 80189094 / 80189264 set or clear it on an object found by name (event cScrObjNameTable::getObj 80188BDC), as does 801A3558. getCrypticPoint clears it on the chosen crypt object, and nothing in the controller sets it again. No reader of the bit was found in the recompiled code, so what it does is [unres]. [inf] It is a scripted hold or hide flag that events set on stage objects.
- **bl0130 (resolved 2026-11-11)** [obs]: class **cBl0130** (factory 805A1E30 → 8011878C, ctor 80118824 on cBaseScrObj, vtable 805AABC0, 0x1AC0 bytes).
  - On init (80196C88) it builds a Havok phantom (0x160 bytes, 8041EF18, stored at +0x1AB8) at an offset from itself, and adds it to the world (8043CC44).
  - Each update (80196D9C) it reads the phantom's overlaps. If one belongs to a live enemy-class object (category 0x2xxxx with HP ≠ 0), it sets its own motion rate +0x1ABC (80553A38; idle 80553A2C), then removes the phantom (8043DA14).
  - 80196FB8 then switches its EST 100 effect on or off on a vtable +0x48 test.
  - bl0130.dat: mdb 3,616 B, tpl, col.hkx (47,184 B), eff (5,536 B), no motions.
  - So bl0130_N001 / N002 at (±15, 0, 5) are sensors that light an effect when an enemy (a bat sent by getCoffinState → cmd 14) is inside them. [inf] They are the room's bat-roost / candle props that hint where she is.
- **Port:** bl0015 and bl0130 are not staged. No ObjTbl.obt object is: tools/ has no ObjTbl reader, assets/native/objects holds only bl0063, and the render-collision stand-in has no coffin bodies. To show the coffins:
  1. Export bl0015 as bl0063 was: `node tools/node-review/review_export.mjs D:/Games/gamesfordecomp/exports/madworld bl0015` (and with MW_COLOR=0). Place the GLBs as `assets/native/objects/bl0015-{source,color}.glb`.
  2. Spawn 8 static instances in play_elise.rs at the `CRYPTS[0..8]` transforms (ObjTbl: (±13.5, 1.518, z), yaw ∓π/2), with no animation.
  3. Optionally add bl0015.dat's col.hkx to the c303 collision set (tools/stage_collision.py, folder parameter, §9.3 item 2).
  - bl0130 would need the same export and an effect hook for EST 100 (bl0130.eff, tools/stage_effects.py BANKS).
- The room's `onEmEvent` breaks bl003c_N00x in the order at 805480B8. Those are the bridge objects, not the coffins.
- **Port:** play_elise.rs `CRYPTS` and its pick counters follow this table. The port had used the bl005c bases.

## 6. Clash (keen competition, mode 0x18)

**Entry** [obs]:
- startAtk 81102B14 checks state hide (12) and sub 5: `startKeenCompetition__7cPl0000FP8cGameObjs(player, this, 0x18)` (8028BDE8), then setRno keen_compe (16)
- so the clash starts when the player strikes the coffin she hides in
- RinRin's is 0xB, from a chainsaw swing in her slalom window
- Elise exports no chkEnableKeenCompetition / reqStartKeenCompetition (the base class is used) [obs]

**The player drives the sub-states** through vtable +0x98 = startCatchAtk 81102FC8 [obs]:

| Sub-state code | Elise | Player code (DOL fn_802A03A0, called from fn_8028C074) |
|---|---|---|
| 0 (state entry) | message `hideOver`, clears player flag 0x10000, **0206** (pulled from the coffin), cloth meshes visible again | phase 0: `pl0030_0466` (+ c466, wp0600_0466); the player code also loads em0630_0206 |
| 0x103 → sub 5 | **0200** + voice 0xA7 | phase 1: `pl0030_0460` / c460 / wp0600_0460 (start) |
| 0x102 → sub 10 | **0201 + additive 0202**, weight = player +0x27DC (the smoothed meter) | phase 2: `pl0030_0461` + blend 0462 (c461 / c462, wp0600_0461) |
| (player phase 3) | — | meter +0x1240 (`param_1[0x490]`): up 0.04 × (remote + nunchuk) when both > 0.5, down 0.05 × (+0x1244) per frame, capped at 30 |
| 0x100 → sub 70 | **0204** bite: from frame 40 to 100 she heals **csv[1] = 100 N / 200 H** HP, spread at csv[1]/60 per frame, plus voice 0xA9; at the end LevelFlow(−csv[1]), → idle (grounded) | the meter ran out: `pl0030_0464` / c464 (the player is drained, phase 10) |
| 0xFB → sub 15 | **0205** + `sexSymbol(1.5)` at frame 15 (jitters bones 0x19 / 0x1A, the breasts) | meter full: `pl0030_0465` / c465 / wp0600_0465 |
| (player phase 4) | — | after frame 110: a **swing-navigation QTE** (cIdPlCockpit::setSwingNavigation 8022F284; remote / nunchuk steps 0..3, accel > 1.5). Timeout (frame ≥ length − 1 − 20 × miss count) → 0x100 bite |
| 0xFA → sub 30 | **0203** (178 frames) + voice 0xA8 | QTE passed: `pl0030_0463` + `c463` / `c467` (the variant depends on a flag) + wp0600_0463 |
| 0x104 | **HP −= csv[6] = 300**, LevelFlow; HP < 1 → **finish_wait (14)**, else **down (10)**, sub 1 | phase 5 at the end of 0463; at frame 62: hit-stop + camera quake |

decideKeenPower1/2 (81109838 / 811098D0: 1 + 0.001·U + 0.25L capped at 1.65; 0.01·U + 0.25L capped at 3) and decideKeenCamera 8110995C have no caller in the em0630 code [unres: vtable or unused].

The explorer's existing "Jack strikes / Elise falls" pair (pl0030_0463 + em0630_0203, 178 frames) is the **0xFA clash win** [obs]. Port it the way `rin_keen` (fight.rs 1376) does, but with two stages (meter, then QTE) and the coffin entry.

## 7. Death and finisher
- **dead (13):** 1005 with hit-stop, vibration and quake, then **finish_wait (14):** 1006 loop, HP = 1, conditions 0 and 3 on [obs].
- Elise has **no `finish` state**, and no `pl0030_c3xx`-style finisher pair exists for her: the only em0630.dat\pl0030_* strings in the DOL are the clash set 0460..0466 / c460..c467, used by 802A03A0 and 802C6768 [obs].
- **The kill (resolved 2026-11-11)** [obs]:
  - The player's catch on the lock target (802C3C0C, the branch after the grab checks) calls **802A4830**. For ids ≥ 0x20400 it enters player state 0x44 sub 10 and sets the finisher type +0x11D by id:
    - 0x20400 → 0, 0x20410 → 1, 0x20430 → 2, 0x20440 (RinRin) → 3
    - 0x20450 → 4, 0x20460 → 5, 0x20475 → 6, 0x20480 → 7
    - **0x20600 / 0x20623 / 0x20630 → 8**
    - 0x20640 / 0x20645 → 10, 0x20650 → 0xB
  - The state-0x44 dispatcher 802A4A18 runs one body per type:
    - 0: 802A4A78 (em0410 pair 1300–1302)
    - 1: 802A53F4 (em0400, 0437–0439)
    - 2: 802A5F70 (em0430, 0440–0443)
    - 3: 802A688C (RinRin, 1305–1308)
    - 4: 802A6FBC (em0450, 04ea–04ed)
    - 5: 802A7B5C (em0460, 1303 / 1304 / 1309)
    - 6: 802A8158 (em0475, 04fa–04fd)
    - 7: 802A8BB0 (em0480, 205a–205d)
    - **8 / 0xB: the generic 802A9830 body**
    - 10: 802A99B4 (em0640, 0474–047b)
  - **Type 8** covers Elise, the Baron and 0x20600. Sub 10 plays `pl0030_0000` (blend 2 × the default blend) and `wp0600_0000`, sets +0x1260 = target and sends **startCatch(0x2F)**. Sub 11 waits for the clip to end. There is no pair clip.
  - Elise's startCatch 81102F24, for catch states 0x2F..0x32: setDeadTime, **setCorpse**, game-object flag 5, gravity and collision off, sub += 1.
  - With no boss-class enemy left, the room fiber's 80133AA4 wait ends and **ev34** plays (§2). The event shows the kill.
- **Port:** a swing next to her in finish_wait stands in for the catch input on PC. It plays pl0030_0000 and sets victory; play_intro.rs then plays ev34. The clash-win pair 0463 / 0203 is no longer reused as the finisher.

## 8. HP, difficulty and the em0630.csv parameters [obs]
**em_common.ini `[em0630]`** (normal / hard):

| Key | Normal | Hard |
|---|---|---|
| Hp | 1000 | 1500 |
| ボス攻撃1 (wing first strike) | 10 | 20 |
| ボス攻撃2 (wing second strike) | 10 | 20 |
| ボス攻撃3 (somersault) | 10 | 20 |
| ボス攻撃4 (somersault, guard cancel) | 10 | 20 |
| ボス攻撃5 (bat swarm) | 20 | 30 |

Both files also set CharColHeight 1.6, CharColRadius 0.6, 当たりサイズ (hit size) 0.6, Scale 1 and 頭回転 (head turn) (60, −45, 20). The ini sections `[em0330]` set Scale 1 only.

**em0630.csv** (cBossParamFile at +0xA30; the second column is hard [inf]):

| # | Name (translated) | N | H | Used by |
|---|---|---|---|---|
| 0 | hide every N accumulated raw damage | 250 | 250 | idle / guard / damage / burn branches |
| 1 | bite heal (player lost the clash) | 100 | 200 | keen_compe sub 70 |
| 2 | ground-guard chip % | 80 | 50 | onPreMove |
| 3 | hint-bat interval in hide (s) | 5 | 10 | hide sub 5 |
| 4 | time before healing starts in hide (s) | 5 | 1 | hide sub 5 |
| 5 | heal 1 HP every (s) | 1 | 1 | hide sub 5 |
| 6 | damage when the player wins the clash | 300 | 300 | startCatchAtk 0x104 |
| 7 | initial level (comment: N 0 / H 20) | 0 | 0 | onInit |
| 8 | level can go down (comment: N 1 / H 0) | 1 | 1 | onInit (+0x1384) |
| 9 | maximum level (comment: N 70 / H 100) | 70 | 70 | onInit (+0x1388) |
| 10 | level change rate (comment: N 1 / H 1.5) | 1 | 1 | onInit |

**Level (dynamic difficulty, LevelFlow 81109A30)** [obs]:
- the level (+0x1380, 0–100) gains Δ × rate, with rate = csv[10] × (max − init) / maxHP
- damage raises it; heals lower it, unless level-down is off (csv[8] = 0)
- L = level/100 drives every decide* function, the guard reactions and the attack types

## 9. Staged plan: tools

Done now (no edits needed):
- `research/motion-cues.json` already contains the em0630 sequences.
- `tools/audio/stage_bgm.py` already stages bs04c / bs04.

### 9.1 Usable as-is
```
# 1. explorer GLB exports first (stage_actor.py needs evidence/node-export-<stem>[-source].json; none exist for
#    em0630/em0330/em0331 yet). Explorer server on 127.0.0.1:8781, run from D:/Games/gamesfordecomp/exports/madworld:
node tools/node-review/review_export.mjs D:/Games/gamesfordecomp/exports/madworld em0630
MW_COLOR=0 node tools/node-review/review_export.mjs D:/Games/gamesfordecomp/exports/madworld em0630
#    (same for em0330, em0331)
# 2. rig, motions and GLBs (writes assets/native/elise-{source,color}.glb, elise-motion.json, research/elise-staging-manifest.json)
python -I tools/stage_actor.py em0630 elise
python -I tools/stage_actor.py em0330 bat        # em0330.dat has two identical em0330.mdb members; the first is taken
python -I tools/stage_actor.py em0331 batmother
```
- **stage_actor caveat:** elise-motion.json also carries the pl0030_046x / c46x and wp0600_046x clips.
- **Rust side of that caveat:** `FightData::new` moves `pl0030_*` clips into Jack's bank, but wp0600 clips stay in the enemy bank; play.rs plays wp0600_04aa..ad for RinRin the same way.
- **Event parser:** `tools/events/stage_intro.py`'s parser handles ev33 / ev34, but its paths and ids are fixed to ev21, so it needs a `--event 33` parameter (§9.2).

### 9.2 Needs a small change (hard-coded to RinRin / c201; add a parameter, not new logic)

| Tool | Hard-wired | For Elise |
|---|---|---|
| `tools/stage_character_color.py` | `MODELS` list (+ RinRin touch-ups) | add `('elise', 'em/em0630.dat', 'em0630')`, `('bat', 'em/em0330.dat', 'em0330')`; then `python -I tools/stage_character_color.py`. Whether the recomp color pack covers em0630 is [unres] |
| `tools/stage_ink.py` | `MODELS = {rinrin, jack}` | add elise / bat (em0630.mdb flag 0x2000, white program) |
| `tools/stage_effects.py` | `BANKS` | add `('em0630', 'em/em0630.dat', 'em0630.eff')`, `('em0330', 'em/em0330.dat', 'em0330.eff')`, and the room bank `c303.eff`. Her setEffect ids: EST 1 (flight aura), 2 (guard), 0x2A, parts effects 0xC / 0x104 / 0x105; trace them with MW_TRACE_EFFECTS |
| `tools/stage_room_cameras.py` | `case2/c201.dat` | `case3/c303.dat` (only cam00, 11,584 B) |
| `tools/stage_street.py` (pattern) | c20100 | new spawn meta from c30300 / c30301 + PlayerTbl (`start`, `Boss`) |
| `tools/audio/stage_announce.py` | `ROOM = 0x201`, `case2/`, `IDS` | ROOM 0x303, `case3/`, IDS = 0x9C..0xAB + 0x1144, 0x1018, 0x1143.., 0x1183 (0x1117 / 0x100A / 0x100B do not exist in c303) |
| `tools/audio/stage_se.py` | `PACKS` | add `scene_mix0303.rsp`, `event/scene_mix7547.rsp`, `event/scene_mix7548.rsp` |
| `tools/pl0000/stage_combat.py` | cues `('pl0030_', 'em0440_')`, `em0440.csv`, `em0440_00.emh` | add `em0630_` / `em0330_` cues, an `elise_tuning` from em0630.csv, damage volumes from em0630_00.emh; the ATHIT rows 0x141 / 0x151 / 0x155 / 0x15B / 0x172 / 0x173 / 0x18E must resolve in `atk_names_em` |
| `tools/index_rin_states.py` | 32 × 3 tables at 0x81174800.. | 17 × 2 tables at 0x81179D58 / 0x81179E24, name prefix `em0630_r1_`, no end table → `assets/native/elise-states.json` |
| `tools/events/stage_intro.py`, `stage_props.py`, `stage_audio.py`, `stage_effects.py`, `stage_ui.py`, `stage_writing.py`, `stage_messages.py` | ev21 paths; props pl0030 / pl0040 / wp0600; "only resolves the verified English EV21 voice path"; EV21 native SE evidence | `--event 33|34`; props add em0630 and **15 em0330 instances**; voice evvo33 / evvo34; SE 7547 / 7548. The SE comparison needs its own native capture |
| `tools/ui/stage_native_ui.py`, `stage_pause_map.py` | `case2/idpac_c201_us.dat`, c201.imt | `case3/idpac_c303_us.dat`, `c303.imt` |

### 9.3 New tooling
1. **Arena with transforms (resolved 2026-11-11)** [obs]: stage_arena.py (MW_ROOM=c303) accepts Y-only rotation with uniform scale. That covers all 106 c303 parts: rotation X / Z is 0 everywhere, 61 parts have a non-zero yaw (32 of them ±π) and 7 have a scale.
   - Native placement: 8020C1F4 walks the SCR directory (0x60-byte descriptors from +0x10). 8020C274 creates each part and copies descriptor **+0x34 → pos (+0x64), +0x40 → rot (+0x70), +0x4C → scale (+0x7C)**, then calls updatePartsMatrix 801B24D4.
   - cParts::updateLocalMatrix 8020F9F8: **local = PSMTXTrans(pos) · MtxInitRotVec(rot, order +0xA0) · PSMTXScale(scale)**. With a parent it adds an inverse parent-scale term.
   - MtxInitRotVec 800BA4B0 order 0 is Rx·Ry·Rz (orders 1–5 permute it); each PSMTXRotRad takes radians.
   - updateWorldMatrix 8020FBFC: world = parent · local, or local without a parent.
   - export_arena.mjs builds the same T·R·S with three.js Euler XYZ (= order 0) and yaw sign +1. The staged c303.glb / c303-color.glb node matrices were checked against c303.json: all 106 match, and the 29 parts whose yaw has |sin| > 0.001 (the ones that fix the sign) all use +1. tools/stage_render_collision.py uses the same rotation (x' = c·x + s·z, z' = −s·x + c·z).
   - Not checked: the order code at +0xA0 for SCR parts (moot while X / Z are 0), and that SCR parts have no parent [inf].
2. **Collision:** a small extractor for the c303 `col*.hkx` members into `assets/native/collision/c303/`, and a folder parameter for `tools/stage_collision.py` (it globs `assets/native/collision/*.hkx` and writes a single `world.json`).
3. **Coffin table (resolved 2026-11-11):** see §5.4. The coffins are bl0015 (8 of them), not bl003c. They have no animations. Still open: what reads +0x1A0 bit 2. The staging steps for bl0015 / bl0130 are in §5.4.
4. **bl0132 / arm type 7 (resolved 2026-11-11, except the c303 fire source):** see §5.3.
5. **Spline:** jkSpline 802547B4 / 80254A1C / 80254BE4 / 80254DD0 (a Rust translation with tests against native samples).
6. **Native evidence for the open items:** an oracle run in c303 (BOSS-UNLOCK-API method) for the BGM after ev34 and the clash phases. The event order and the finisher are settled statically (§2, §7). These need pair-content captures like `bevy-native-clash-*` but for mode 0x18.

## 10. Rust generalisation (crates; no code changed here)

**mw-sim/src/fight.rs** (2,577 lines) is RinRin-specific throughout:
- `Rin` struct
- `rin_state` ids
- `RIN_MAX_HP` 1600
- `RIN_ATK_POWER`
- `SCAFFOLDS`
- `Bomb` / `Fan`
- the electric fence (`fence_hit`, `rin_electric`)
- `rin_finish` / `rin_keen` / `rin_throw*` / `rin_scaffold`
- `FightData { rin, rin_idx, rin_clip }`
- `CombatData::tune()` = em0440.csv
- events `RinHit` / `RinState` / `RinDeflect` / `RinFinishReady`
- the `Autopilot` test driver

Split it into:
- (a) a shared fight core: player, hits, `resolve_hits`, `player_damaged_by`, the finisher meter and hit-stop, which already exist and are boss-neutral
- (b) a `Boss` enum or trait with per-boss data: bank, states, HP / attack table, tuning, emh volumes
- (c) `boss/rinrin.rs` (move the existing code)
- (d) `boss/elise.rs` with the 17 states above, LevelFlow and the decide* functions, plus:
  - a `Bats` swarm system (100 light entities with the em0330 states / commands)
  - the Black Dragon spline heads as attack spheres
  - flight (the flag + jkSpline)
  - hide / coffin (room messages)
  - the two-stage clash (meter, then swing QTE)
- Move the player's clash code (now RinRin-specific `clash*` fields, 802956B4) into a mode table: mode 0xB = RinRin, mode 0x18 = Elise (802A03A0).

**mw-game/src/play.rs** (1,536 lines): `Who::Rin`, `LockTarget::Rin`, and fixed assets: `native/world/c201*.glb`, `rinrin-*.glb`, `rinrin-motion.json`, ink model "rinrin", `ev21/wp0600.glb`, the status / HUD text ("RinRin HP {}/1600"), the window title. These need a stage / boss descriptor (room id, arena GLBs, collision, spawn meta, boss model, intro event, BGM, announce room).

**Other modules**
- `play_intro.rs` / `play_intro_audio.rs` / `play_intro_effects.rs` / `play_intro_ui.rs`: paths `assets/native/events/ev21/…`, the BS04 manifest key, and the attachment-target match `"pl0030" | "em0440"`. Parameterise by event id and add the em0630 / em0330 targets; ev33 has 15 bat actors.
- `play_challenge.rs`: the `challenge-c201-us` catalog. Elise has none, so make the challenge optional per boss.
- `play_broadcast.rs`: `idpac_c201_us` program 6 + cam00 → per room (`idpac_c303_us`; the program id is [unres]).
- `play_audio.rs` / `native_audio.rs`: announce / SE manifests per room (§9.2).
- `play_fx.rs`: banks em0630 / em0330 / c303.
- `play_springs.rs`: RinRin jiggle / skirt only. Elise uses **5 cByCloth** (+0xE50, +0xF14, +0xFD8, +0x109C, +0x1160; wind info 8059A32C..) and **2 cHair** (+0x1224 on bone 0x19, +0x129C on bone 0x1A) [obs, onInit]. They map to wings / hair; translate the cloth parameters (setClothParam 800788CC, info tables 8059A0E0..8059A2A0) or keep an added approximation, tagged as added.
- `play_hud.rs`: the boss31 gauge is generic (setBossHpGage). Use max HP 1000 / 1500.
- `play_continue.rs` / death: Elise's victory state 15 replaces RinRin's PL_DEAD taunt.

**mw-data:** collision / event / ui loaders are generic; the room-specific inputs are the paths.

## 11. Open questions (ordered)
1. ~~Event order and finisher~~, ~~BGM after ev34~~: resolved 2026-11-11 (§2, §7). Not ported: the clear window and the scene change to 0xB14.
2. ~~SCR transform basis~~: resolved 2026-11-11 (§9.3 item 1).
3. ~~Coffin controller fn_800751BC~~, ~~bl0015 animations~~ (none exist), ~~bl0130~~: resolved 2026-11-11 (§5.4). Still open: what reads the +0x1A0 bit 2 flag. Not staged: bl0015 / bl0130.
4. ~~Arm type 7 / bl0132 / hit types / the c303 fire source~~: resolved 2026-11-11 (§5.3). There is none in c303. Still open: whether c301's wp0690 carries over.
5. The cEm0331 path data and the bat command semantics (names here are [inf]).
6. CSV column 2 = hard [inf]; the em_common_hard.ini section is [obs].
7. decideKeenPower1/2 and decideKeenCamera callers; annihilate (6) reachability through the base class.
8. Whether c302 → c303 shows a challenge prompt.
