# The Black Baron fight (cEm0623, room c503), the oirans and Mathilda: port map

Goal: port the fight the user calls the "Mad Baron" fight to mw-play the way RinRin (cEm0440, room c201) was ported,
and catalogue every Mathilda asset for a custom variant where Mathilda is the boss. No Rust or game code was changed.
Companion map with the same layout: `research/ELISE-BOSS.md`.

Tags: **[obs]** = read from disc data or decompiled code. **[inf]** = inferred from that code or data; confirm before
relying on it. **[unres]** = still open.
Disc root: `D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files` (written `files/` below).
Enemy-module addresses are analysis addresses (base 0x81000000; `files/dll/Rel/enemyRSO.rso` file offset = address −
0x81000000). DOL addresses are guest addresses in `sys/main.dol`.

Decompiled output (outside the repo):
`C:/Users/Matteo/AppData/Local/Temp/claude/C--Users-Matteo--claude-spares-spare-SC961L/b18b4d75-495f-4d48-a047-7b3fa286cef3/scratchpad/baron/`
- `em0623a.c`: every cEm0623 / cEm0624 / cEm0625 / cEm0615 row, with RSO strings and floats resolved inline
- `em0965a.c`: all 23 cEm0965 rows (the oirans)
- `plbarona.c`: DOL 8029D350 / 8029F164 / 8029F80C / 8029FC24 (the player's side of the Baron clash and Mathilda's bat combo)
- `room503a.c`: DOL 8016B710..8016D21C (room c503 code)
- `misca.c`, `snd.c`: 80133AA4, 801246F8, 801246D8, PlayBGM_EX 8026A108, StopBGM 8026AEC0
- scripts: `annot.py` (string/float annotator), `catalog.py` (MOT frames + SEQ cues), `ems.py`, `poolscan.py` (DOL string-pool xrefs), `dolxref.py`; `dolstr.txt` (DOL strings)

## 0. Summary

**Who is who** [obs unless tagged]:
- **The Black Baron is MadWorld's final boss.** His fight is room **c503**, the DeathWatch heliport ring (`case5/c503.dat`). It ends in ev51 and then the game ending, ev93 (§2).
- Gameplay class **cEm0623** (`em_common.ini` comment `BBC男キャラ`, "BBC male character"; BBC = BloodBath Challenge, which the Baron hosts).
- Cutscene model of the Baron: **em0620** (with coat and hat). Its hand-ring props are **em0621 / em0622**.
- **Mathilda = em0610 model, gameplay class cEm0615** (`BBC女キャラ`, "BBC female character").
  - Mesh names: leather, spike, sunglasses, hair_front.
  - In the fight she stands at the ring side with a bat (wp0582) and beats Jack when the Baron's uppercut catches him.
  - She cannot be targeted or damaged.
- **The "geishas" are oirans (おいらん, high-ranking courtesans): class cEm0965, model em0960** (kimono, obi, geta, kanzashi meshes).
  - Three stand on the opposite ring side and cheer.
  - The first one tosses a recovery item into the ring when Jack's HP gets low.

| | RinRin (ported) | Black Baron |
|---|---|---|
| Class / object id | cEm0440 / 0x20440 | **cEm0623 / 0x20623** (c50300.ems record 0, id 0x623, family 0) [obs] |
| Room | c201, spawn set c20110 | **c503** (`case5/c503.dat`), spawn set **c50300.ems** (1 record) [obs] |
| Spawn | — | Baron at (−0.003, 0.052, 5.096), yaw 0xB4 = 180°. Jack's PLP `start` is at (0, 0, −6) [obs] |
| Children created in onInit 810FA710 | — | hat 0x20626 (cEm0624, bl0330); sunglasses 0x20627 (cEm0625, bl0331); **Mathilda 0x20615** at (−8.2, −0.05, 0), yaw +90°; **3 oirans 0x20965** at (7.9, −0.05, 0 / −2.5 / +2.5), yaw −90°, only when the save byte `80616CCD` bits 4–5 == 3 [obs; the flag's meaning is unres] |
| State table | 32 × (branch, update, end) | **24 × (branch, update)**, no end table: **0x81179368 / 0x81179488** [obs] |
| HP | 1600 | **2000** normal and hard (`[em0623]` in em_common.ini / em_common_hard.ini) [obs] |
| Tuning CSV | em0440.csv | **em/em0623.csv**, 15 rows, §3.6 [obs] |
| Intro event | ev21 (1080 frames) | **ev50** (3325 frames, 31 cuts), played by room event −89 (fn_8016CB58) [obs] |
| Outro | — | **ev51** (3200 frames, 26 cuts), then **ev93** (the ending, 10753 frames), from the cBoss callback once the boss is done (fn_8016C8F0) [obs/inf] |
| Clash | mode 0xB | **startKeenCompetition(player, this, 0x16)** (reqStartKeenCompetition 810FBC0C) [obs] |
| BGM | bs04 / bs04c | ev50 BGM SEQ → `BS03C_RSD`. Table index 0x1D maps to **bs03.rsd** (there is no bs03c file); the room requests 0x17 = bs03. Phase 2 (angry) does StopBGM(180) and then **PlayBGM_EX(0x19) = bs05** [obs] |
| SE / voice | scene_mix0221 (4203, 8117) | **se/scene_mix0503.rsp**: block 4213 (Baron SE), **8127 (Baron voice)**, 7100, 503, 1001 / 1002 / 1007 / 1029, 14 / 15 [obs] |
| Announcer | an_c201 | **an_c503_{info,pack}_eng** (23 ids, §1.4) [obs] |

## 1. Identity and files

### 1.1 Character banks [obs]

| File | Contents |
|---|---|
| `em/em0623.dat` (2,611,488 B, 230 members) | `em0623.mdb` (36 bones; meshes body / face / pants / hair / `pa_SB` open hand / `gu_SB` fist / acce / kutu); tpl + `_AB`; `em0623_00.emh` (3,712 B); **`em0623.eff`** (461,984 B); 51 own MOT + 3 borrowed `em0110_10a5/7/8`; 4 camera MOT `em0623_c000..c003`; paired player clips `pl0030_0480..048b, 047c, 1050..1052, 1055`, `wp0600_0480..048b`, 21 player camera MOT `pl0030_c480..c493`; SEQs |
| `em/em0624.dat`, `em/em0625.dat` | **event-only** models: em0624 = coat (`court_SB`, fur, belt) in ev50 cut 23 (the Baron takes his coat off); em0625 = head with hat and sunglasses (ev36) |
| `em/em0626.dat` (bl0330: `hat_SB`, + `bl0330_0000.mot` 648 B), `em/em0627.dat` (bl0331: `gulasan_SB` = sunglasses, + `bl0331_0000.mot`) | the gameplay hat / sunglasses. They are created as factory ids 0x20626 / 0x20627 but are **classes cEm0624 / cEm0625** (setParent on bone 4, startSeparate plays their own MOT, reset) [obs; that bone 4 is the head is inf] |
| `em/em0620.dat` | event Baron, 79 bones, model only (coat / court, fur, hat, belt, akuse) |
| `em/em0621.dat` / `em0622.dat` | `R_hand_SB + R_ring` / `L_hand_SB + L_ring`. In events they attach to em0620 by OBJ opcode 14 on bones 9 / 14 [obs] |
| `em/em0610.dat` | **Mathilda** model only (`em0610.mdb`, 27 bones), used by the events |
| `em/em0615.dat` (546,880 B) | **Mathilda gameplay bank**: `em0610.mdb` + tpl / _AB, `em0430_00.emh` (shared), `em0610.eff` (12,544 B), `em0610_0000 / 0001 / 0010.mot` + SEQ |
| `em/em0960.dat` | oiran model only (34 bones) |
| `em/em0965.dat` (1,106,496 B) | **oiran gameplay bank**: `em0960.mdb` + tpl, `em0430_00.emh`, `em0960_0000..0004.mot`, camera MOT `em0960_c000.mot` (no eff) |
| `wp/wp0582.dat` | Mathilda's bat (onPreMove 810F9E74 creates 0x30582 and attaches it at offset (−0.12, −0.005, −0.07)) [obs; that it is a bat is inf from atk_bat] |
| `bl/bl0522.dat` | **helicopter** (`heripro` = propeller, `hane` = rotor blade). `bl0522_030c.mot` (111 frames) pairs with the Baron's 030c finisher stage; `bl0522.eff` [obs; the name reading is inf] |
| `bl/bl0519.dat` | second helicopter model (hane / heripro, col.hkx), 2 instances in ev50 / ev51 [obs] |
| `bl/bl0557.dat`, `bl/bl0333.dat` | small props spawned by keen sub 0x2A (0x70557) and by God Hand (0x70333, `bl0333.eff`) [obs; roles unres] |
| `bl/bl0817.dat` / `bl0818.dat` | ring with `rope` (ev50) [obs] |

**Bones:** em0610 has ids 0–26; em0623 0–23, 30–33, 42, 50–52, 117–120; em0960 0–27, 30–35. pl0030 ids 0–24 are a subset of all three. Bind-direction compatibility with Jack's clips (the Kreely method) is [unres].

### 1.2 Room c503 (`case5/c503.dat`, 24 members) [obs]

**World**
- `c503.scr` (2,219,648 B): **249 parts**, 236 with non-identity transforms. **All 236 are yaw-only + uniform scale**, the basis stage_arena accepts for non-default rooms.
- Part names: base, gd, pipe, brige, brige_heli, frame, heliport, Mframe, ad, 27 × led, **rope**, spotlight, mob_AN01 (crowd), about 160 × flare_BB01, pole, flash, flag, bglight, spotflare, ELV.

**Collision:** col.hkx 45,424 B; col_ef.hkx and col_obj.hkx 36,896 B; col_pl.hkx 832 B; col_cm.hkx empty.

**Room data**
- `c503.eff` 132,064 B
- `c503.imt` 2,176 B (pause map)
- `c50300.lgt` 460 B
- `c50300.caa` 756 B (`CAA` camera areas [inf])
- `cam00_Clip.cam` (11,584 B, **361 frames**); the eye starts at (−35.4, 44.3, 13.5), FOV 1.134 rad

**PlayerTbl.plp** (the c201 layout: 36-byte records from +16, position at +20, yaw at +32):
- `start`: (0, 0, −6), yaw 0
- `heliport`: (0, −5, −113), yaw 0

**ObjTbl.obt:** bl051c_N001..8, bl051d_N001..4, **bl0522_N001 / N002** (the room ctor shows N001 and hides N002 through 8017F710), bl0551_N001..2, bl0552_N001..4, bl0559_N001, bl055c_N001.

**Spawns:** c50300.ems is v20 with 1 × 128-byte record: id 0x623, family 0. c503 has no `c503em.ini` (no grunts).

**Room UI:** `case5/idpac_c503_{us,fr,it,sp}.dat`.

### 1.3 Events [obs]

**ev50** (`event/ev50_us.dat`, 1,967,648 B)
- CAM: 31 cuts: 200, 65, 96, 51, 51, 109, 56, 103, 63, 117, 111, 53, 53, 50, 43, 77, 37, 29, 76, 39, 32, 50, 100, 83, 110, 450, 115, 106, 279, 340, 181.
- OBJ:
  - em0620 (the Baron, 20 clips, cuts 0–22) with em0621 / em0622
  - em0610 (Mathilda, cuts 9–18)
  - **em0623 (cuts 23–25, 30)** and em0624 (coat, cut 23)
  - **em0960 × 3 (cuts 26–29)**
  - bl0330 / bl0331 (hat / sunglasses, opcode 19 at 1854), bl0519 × 2, bl0817, bl0818
  - pl0030 / pl0040 / wp0600
- Other chunks:
  - ID (IDD + SEQ)
  - EFCT: banks `pl0030_00`, `bl0519_00/01`, `CAMERA`, `NONE`
  - BGM `EV50_RSD` → `BS03C_RSD`
  - VCE `JKSNN_EVVOSTR_EV50`
  - SE: 17 cues **7568000..7568014** (pack `se/event/scene_mix7568.rsp`), plus `eSeVoBooing` and `bl0330_00`

**ev51** (1,944,384 B)
- CAM: 26 cuts (160, 38, 65, 73, 409, …, 357).
- OBJ: em0623 (21 clips), em0610 (cuts 0–4), em0621 / em0622, bl0519 × 2, bl081c, wp0582 (the bat), pl0030 / pl0040 / wp0600.
- Other chunks:
  - EFCT: `em0623_00`, `wp0580_00`, `bl081c_00`, `bl0519_*`
  - BGM `EV51_RSD` / `EV51c_RSD`
  - VCE `EV51` / `EV51c`
  - SE **7569000..7569021** (`scene_mix7569.rsp`)

The `c` variants' condition is [unres].

**Voice streams:** `evvostr/evvo50{,_ENG,_FRE,_ITA,_SPA}.rsd` (315,392 B), `evvo51*` (577,536 B), `evvo51c*`.

`tools/events/stage_intro.py`'s parser handles these containers; its paths are fixed to ev21.

### 1.4 Audio [obs]

**BGM (table 805CB2C8):**

| Index | File |
|---|---|
| 0x17 | bs03 |
| 0x19 | bs05 |
| 0x1D | bs03 (also the "bs03c" slot) |
| 0x1F | bs05c |
| 0x53 | ev50 |
| 0x54 | ev51 |

ev51b / ev51c strings exist in the DOL. Only `ev51c.rsd` is on disc.

**SE pack** `se/scene_mix0503.rsp`: wave and definition blocks 503, 1001, 1002, 1007, 1029, **4213**, **7100**, **8127**, plus definitions 14 and 15.

Baron cue blocks: 4213 (SE), 8127 (voice), 7100 (in 0203 at frames 40/41).

Mathilda's only cues: **4213:403** (×8, the bat hits) and **4213:404**.

The oirans have no cues.

**Announcer** (jkSound::PlayVoiceStream 8026B558, `an_c503_info_eng.bin` member + `announce/eng/an_c503_pack_eng.bin`). 23 ids, given as id (group mask):

```
10e(7) 10f(f) 110(1f) 111(7) 112(3) 113(3) 114(ff) 115(f) 116(7) 117(7) 118(f) 119(7) 11a(7) 11b(3)
11c(7) 11d(f) 11e(3) 1018(ff) 1143(ff) 1145(ff) 1146(ff) 1147(3) 1022(3)
```

| Id | Requested by |
|---|---|
| 0x10E | mach punch |
| 0x112 | guard |
| 0x113 | strong hit taken |
| 0x114 | other hit taken |
| 0x115 / 0x11A | Baron wins (player dead; 0x11A when continues == 0) |
| 0x116 | clash start |
| 0x117 | first clash won |
| 0x118 | clash lost (sub 0x50, frame 80) |
| 0x11B | idle / dizzy taunt (cooldown 0xE10) |
| **0x11C** | **oiran item toss** (em0965 item_in, frame 16) |
| **0x11D** | **Mathilda's bat combo** (atk_bat, frame 80) |
| 0x11F | BH upper |
| 0x120 | shoryuken. **0x11F and 0x120 are not in the c503 table, so they play nothing** |

## 2. Room flow and intro (DOL) [obs]

**Room ctor fn_8016C508** (class `room::c503::RoomMode_C503::cMainMode`, vtable 805A785C):
- registers room event **−89 → fn_8016CB58** and −88 → 8013628C
- sets 805A445C = 0xE03
- creates cBoss (fn_8016C724 → fn_8016C780, vtable 805A7894)
- shows `bl0522_N001`, hides `bl0522_N002`
- fade 80086E60(3)
- 80123470(cb fn_8016CA64, key 0x10); 80123B7C(−89), 80123B7C(−88)

**fn_8016CB58 (room event −89, entrance):**
- **80123820(0x50) = ev50**
- room effects 1 / 2 (801FD578 on 80604148), fade 15
- **801246D8(0x17) = bs03**
- 802D3CB4(0, −1)

**cBoss callback fn_8016C8B0 → fn_8016C8F0:**
1. load `c503.dat\c50300.ems` and spawn it (80124DDC)
2. **80133AA4**: a frame-yield loop until its condition object reports done [inf: the boss is finished]
3. kill room effect 1
4. 801246F8(0x3C): BGM fade-out
5. fade 15
6. **80123820(0x51) = ev51**
7. 800A7190(80607078, 1)
8. **80123820(0x93) = ev93** (the ending)
9. 80181BB8(80653EB4, **0xE03**, 0): scene change [inf: credits / clear]

**Challenge card:** no 8012409C call in the c503 cluster [obs]. Like Elise, the Baron has no TAKE THE CHALLENGE prompt in his room [inf].

**Port flow** (mirrors the RinRin intro path in `crates/mw-game/src/play_intro*.rs` / ev21):

ev50 → spawn the Baron (Mathilda and the oirans come with him) → fight under bs03 (the ev50 SEQ names BS03C = bs03.rsd, so the 0x17 request is skipped as for RinRin's bs04c) → bs05 from the angry state on → ev51 → ev93.

## 3. The Baron's AI (cEm0623)

### 3.1 State table (branch 0x81179368, update 0x81179488; 12-byte member pointers; onMove 810FB634 runs both through 802E57C0) [obs]

| Id | State | Update | Motion(s) / summary |
|---|---|---|---|
| 0 | wait | 810FBCEC | 0000 (121 f) idle, open hands, announcer 0x11B taunt. Branch 810FBC54: `chkItemInStart` (§4.3); if the player is beyond 4.5 m (dist² > 20.25) → step_f; else chkAtkStart (phase 1) / chkAtkStart2 (phase 2) |
| 1 | threat | 810FBE24 | provocation: one of 0001 / 0002 / 0003 at random |
| 2 | angry | 810FBFEC | **phase-2 transition**: 000c (171 f) + **camera em0623_c003**, StopBGM(0xB4), at the end **PlayBGM_EX(0x19 = bs05)**, clears flag 80697E64 bit 10 |
| 3 / 4 / 5 / 6 | step_f / b / r / l | 810FC13C / 810FC224 / 810FC438 / 810FC520 | 0011 (21 f) / 0012 (38 f; can chain into 0001_01.seq) / 0013 / 0014 |
| 7 | atk_combo | 810FC608 | 0011 step-in, then a chain from 0100..0105 and 0200 (jabs / hooks / straight) |
| 8 | atk_mach_punch | 810FCC68 | 0018 (wind-up 17 f) → 0106 (76 f), effect 0x1A, voice 0x10E |
| 9 | atk_drop_kick | 810FD064 | 0018 → 0202 (114 f; "screw drop kick" in the csv) |
| 10 | atk_shoryuken | 810FD340 | 0018 → 0107 (95 f). **On catch: Mathilda's bat combo**, §5.2 |
| 11 | atk_antiair | 810FD8F0 | 0107. Chosen in phase 2 when the player is ≥ 0.8 m above him |
| 12 | atk_bh_upper | 810FDAA0 | 0018 → 010a (50) / 010b (51) / 010c (170), effect 0x28 |
| 13 | atk_godhand | 810FE054 | 0018 → **0108 (201 f)**. Sets 80697E8C bit 0x2A and 802CBE18(0.32, player, this, 9999): **pulls the player in** (step 0.1·(1 − d/8) per frame inside 8 m). On contact it spawns **bl0333 (0x70333)** at the hit part, launched forward at 0.45 |
| 14 | atk_dempseyroll | 810FE418 | 0110 (34 f) loop. **The guard counter**: chosen after he blocks > 3 hits (> 1 with flag 80697E50 bit 0xB) |
| 15 | atk_rider_kick | 810FE600 | **0203 (231 f) + camera em0623_c002** (イナヅマキック, "lightning kick"); effect 0x33 on the player; setLmtEffect(0x12). Phase 2, once damage taken since the last check ≥ 20 % of max HP |
| 16 | atk_keen | 810FEA38 | 0018 → 0003 → 0300: the **clash-opening grab**. Phase 1 when HP ≤ csv[13] % and U{0..3} ≠ 0; phase 2 when HP < csv[14] − 10 = 390 |
| 17 | keen_compe | 810FEDE8 | the clash and the finisher chain, §3.3 |
| 18 | avoid | 810FFF28 | 0015 / 0016 / 0017 (21 f each, never the same one twice) |
| 19 | guard | 8110000C | 0121 (20 f), voice 0x112, guard effect 0xC |
| 20 | dmg_normal | 81100128 | **borrowed grunt reactions em0110_10a5 / 10a7 / 10a8** (set in onPreMove) |
| 21 | dmg_piyori | 81100264 | 000a (61 f) going dizzy |
| 22 | piyori | 811003F0 | 000b (115 f) dizzy loop, stars effect 0x4C every U[25, 45] frames, 30 × U[3, 4] frames. **When HP < 2 the timer is 1e6 and condition flags 0 and 3 (+0x3C0) are set: he waits for the finisher** |
| 23 | pl_dead | 81100684 | player HP 0: 0002 (216 f) laugh, voice 0x11A (no continues) or 0x115, then 0000 |

Unused by code: **0120** (50 f) [obs: no string reference].

### 3.2 Damage intake (onPreMove 810FAAA0) [obs]

- HP s16 at +0xEC (max +0xEE). Damage is also added to the "taken since" counter +0xE4E.
- Order: avoid check → guard check → damage.
  - **Avoid** (chkAvoidStart 810FB54C): csv[1] = 70 % before the first clash, csv[3] = 45 % after. Never during guard.
  - **Guard** (chkGuardStart 810FB45C): csv[0] = 60 % / csv[2] = 30 %; always while already guarding. Both need the attack timers +0xE54 / +0xE5C ≤ 0.
- Avoid by hit type:

| Hit types | Response |
|---|---|
| 0x2A, 0x32, 0x35, 0x86, 0x8C, 0xCC, 0xCE | step_l (6) |
| 0x2E, 0x30, 0x7D, 0x81 | step_b (4) |
| others | avoid (18) |

- Damage types:

| Hit types | Response |
|---|---|
| 0x2A..0x36 (minus 0x2D), 0x86, 0x8C, 0xCC..0xCE | hit effect 200 + em0110_10a8, voice 0x113 (not on 0x86 / 0x8C) |
| 0x75..0x78, 0x7B..0x7D, 0x81, 0x8D | 10a8 + 0x114 |
| others | 10a5 / 10a7 alternating + 0x114 |
| 0xF6 | the player's held object breaks |
| 0x10D | ignored |

- While the armor flag (+0x864 bit 0x10) or atk_keen is active, damage is applied without a reaction.
- **HP floor before the first clash:** while +0xE58 (the "post-clash" flag) == 0, HP is clamped to **max × csv[13] % − 10** (85 % → 1690 normal; 70 % → 1390 hard). So the first clash is forced.
- HP < 2 → HP = 1, conditions 0 / 3 on, dmg_piyori (21).
- Every frame: `setBossHpGage(maxHP, HP)` 8024A760 (the boss HUD the port already has).

### 3.3 Clash (keen competition mode 0x16) and finisher chain [obs, sub names inf]

Entry is **catch 0x27**, i.e. atk_keen's grab connecting (startCatch 810FB9D8):
- if the Baron was in shoryuken → shoryuken sub 0x14 (Mathilda, §5.2)
- otherwise → keen_compe (17), except during rider kick

The player then drives the sub-states through vtable +0x98 = **startCatchAtk 810FBAC4**:

| Player code sent | Baron sub | Baron clip | Paired player clip (vtable +0x450, flags 0x400) | Notes |
|---|---|---|---|---|
| (entry) | 0 | **0301** (69 f) | **pl0030_0480** + camera pl0030_c480 + wp0600_0480 | effect 0xB, voice 0x116. Sub 1 then reads the player's struggle value +0x27DC |
| 0xFB | 0x0A | 0302 (88) | pl0030_0481 (+c481) | effects 0x47 / 0x48 / 0xB off, 0x4A on the player. **Hat flies off** (cEm0624 startSeparate, freed at frame 28) |
| 0xFC | 0x0C | 0303 (87) | pl0030_0482 (+c482) | effect 0x4C. **Sunglasses fly off** (cEm0625, frame 29) |
| 0xFD | 0x0E | 0304 (88) | pl0030_0483 (+c48c) | effect 0x4E |
| 0xFE | 0x10 | 030d (88) | pl0030_048a (+c483) | effect 0x50 |
| 0x104 | 0x14 | 0305 (71) | pl0030_0484 (+c484) | |
| 0x104 after sub 0x15 | 0x18 | 0306 (151) | pl0030_0485 (+c485) | HP −= **csv[14] = 400** at frame 8; at frame 120 → 0x1E |
| 0xFA after sub 0x15 | 0x16 | 0306 | pl0030_0485 | HP −400 at frame 8. **First win:** voice 0x117, flag 80697E64 bit 10, at the end **+0xE58 = 1 → angry (phase 2)**. Later wins → wait |
| (chain) | 0x1E → 0x20 | 0308 (53) → 0309 (31) | pl0030_0487 (c487 / c488) → pl0030_0488 | |
| **0xFA after sub 0x21** | **0x28** | **030a (93)** | **pl0030_0489** (+c489) | **slow motion** 8007F94C(…, 0.01), restored to 0.1 at the end |
| (chain) | **0x2A** | **030b (60) + camera em0623_c000** | (Jack: pl0030_1051 60 f + c491 [inf: matching length and SE 4213:222]) | spawns **bl0557 (0x70557)** + effect 0x61; **HP = 1 at frame 26** |
| (chain) | **0x2C** | **030c (202) + camera em0623_c001** | (Jack: pl0030_1052 202 f + c492 [inf: same SE 400 / 401 / 402]) | spawns **helicopter bl0522 (0x70522)** at his position, playing `bl0522_030c` |
| (end of 030c) | — | — | — | effects off → **piyori (22) with HP 1**: the finisher wait |
| 0x100 (player failed) | 0x50 | **0307 (141)** | **pl0030_0486** (+c486) | the Baron wins the exchange: effects 0x49..0x50, 0x56; hat and sunglasses reset; voice 0x118 at frame 80; → wait |

- From shoryuken, code 0x100 → shoryuken sub 0x1E (recover).
- **Player side** (DOL fn_8029D350, sub-states +0x11C):
  - a slow-motion exchange: 8021642C time scale 0.5..1
  - per step a swing-direction prompt through the cockpit widget 8022F284 (prompt 0 / 3 / 1 / 2)
  - thresholds on the remote / nunchuk swing values +0x3BC / +0x3BD / +0x424..+0x428 (1.2..1.9)
  - success calls fn_8029F164, which plays 0481 / 0482 / 0483 / 048a / 0484 and sends 0xFB / 0xFC / 0xFD / 0xFE / 0x104; failure → case 10 sends 0x100 with pl0030_0486
  - sub 7 is a fill meter (+0x492, 0.005 × input) with **pl0030_048b (261 f) + camera c48d + wp0600_048b**, followed by case 8 (0489, sends 0xFA)
  - rumble 80279CA8; camera quake 8007FB04 (0.1, 0.9, 0.2, 1.5, 15) [obs code; the input meaning is inf]
- **The kill** after the helicopter is [unres]. The Baron sits in piyori with HP 1 and conditions 0 / 3. Catch types 0x2F..0x32 send him to piyori sub 99 (the player-driven hold). The DOL's other Baron player clips:
  - `pl0030_1050` (120 f, camera c490), used by 80274DC4 and 802BEE14
  - `pl0030_047c` (151 f), with pl0030_0031 / 0032 / 1016 in 8029FC24

  These are the candidates for the kill / grab; confirm with a native trace. Then the room's 80133AA4 wait ends and ev51 plays.

### 3.4 Attack selection [obs]
- Timer +0xE48 (set after attacks to U[30, 40], U[40, 50], …) ≤ 0, then a weighted draw over csv rows that never repeats the last attack (+0xE4C).
  - **Phase 1** (chkAtkStart 811008AC): combo / mach / drop kick by csv[4..6] = 40 / 30 / 30.
  - **Phase 2** (chkAtkStart2 81100A90): combo / mach / drop / shoryuken / god hand / BH upper by csv[7..12] = 20 / 20 / 20 / 20 / 10 / 10. Rider kick when +0xE4E ≥ 20 % max HP; antiair when the player is ≥ 0.8 m higher; atk_keen when HP < 390.
- Flag 80697E50 bit 0xB doubles the attack-timer decay and lowers the counter threshold [obs; the flag's meaning is unres, possibly hard mode or a stage flag].

### 3.5 Attack hit parameters (setAtkHitParam 81005DA4(scale, this, part, ATHIT, bossAtkIndex)) [obs; index mapping inf]
Power comes from `ボス攻撃1..8` (normal / hard), read through vtable +0x470.

| Attack | ATHIT | Scale | ini (N / H) |
|---|---|---|---|
| combo | 0x172 | 1.0 | 1 コンボ 5 / 10 |
| mach punch | 0x18C | 0.7 | 2 マッハパンチ 15 / 20 |
| drop kick | 0x18E | 0.7 | 3 ドロップキック 15 / 20 |
| shoryuken | 0x17E | 0.8 | 4 昇竜拳 25 / 40 |
| god hand | contact | — | 5 ゴッドハンド 40 / 50 |
| BH upper | 0x17D | 2.0 | 6 BHアッパー 40 / 50 |
| dempsey roll | 0x172 | 0.7 | 7 デンプシーロール 10 / 20 |
| rider kick | 0x18E | 1.5 | 8 イナヅマキック 30 / 40 |
| antiair | 0x161 | 0.7 | — |
| atk_keen | 0x18E | 0.7 | — |

Other collision: `CharColHeight 1.6`, `CharColRadius 0.8`, `当たりサイズ` (hit size) 0.8; head-turn limits (60, −45, 20). The clash hit row is `EM0623_KEENCMP` (DOL string 80559D73, cAtHitInfo CSV loader 801B59A0).

### 3.6 em0623.csv (cBossParamFile +0xA30; columns normal / hard) [obs]

| # | Name | N | H |
|---|---|---|---|
| 0 / 1 | guard % / avoid % before the first clash | 60 / 70 | 60 / 70 |
| 2 / 3 | guard % / avoid % after it | 30 / 45 | 30 / 45 |
| 4..6 | phase-1 weights: combo / mach / screw drop kick | 40 / 30 / 30 | same |
| 7..12 | phase-2 weights: combo / mach / drop / shoryuken / god hand / BH upper | 20 / 20 / 20 / 20 / 10 / 10 | same |
| 13 | HP % at which he goes for the clash | **85** | **70** |
| 14 | clash damage | 400 | 400 |

### 3.7 Effects and parts [obs]

**Effect calls**
- 801FD578 / 801FCF64 (create / kill) on the effect manager 8069550C. Ids: 200 (hit), 0xC (guard), 0x1A, 0x23, 0x28, 0x33, 0xB, 0x47..0x50, 0x56, 0x61.
- `setLmtEffect` 81100FB8: effect 5, a rotating 8-slot trail on a part (0xE in shoryuken, 0x12 in rider kick).
- SEQ op-16 cues (e.g. 0100: id 15; 0106: 25; 0107: 30 / 10; 0108: 55; 0203: 50 / 131; 03xx: 70..98).

The ids index `em0623.eff` / core2 [inf]. Trace with MW_TRACE_EFFECTS.

**Hands:** `changeHandMesh` 81101080 swaps `pa_SB` (open) and `gu_SB` (fist).

**Hair:** onMoveHair 810FB944. The cloth setup 800788CC / 80077CBC (+0xE8C) is the coat or jacket cloth [inf].

## 4. The oirans (cEm0965, model em0960)

### 4.1 Spawn and setup [obs]
- **Created by the Baron's onInit**, not by an ems. There are 3, at x = 7.9, z = 0 / −2.5 / +2.5, yaw −90° (facing the ring), only when `80616CCD` bits 4–5 == 3. 80616880 is cSaveData [inf: a progress or option flag; **unres**].
- Only the first one (+0xE88) is ever told to toss items.
- `[em0965]` `;おいらん`: HP 1000, CharColHeight 1.6, radius 0.3, hit size 0.6.
- **isEnableTarget = 0** (8111D980): not targetable. Hits are swallowed in onPreMove.
- Two cHair (+0xE48: bones 0x1E–0x20; +0xE50: 0x21–0x23; kanzashi / ribbons [inf]) and two jiggle springs (802011E8 at +0xE58 on bone 0x19, +0xED0 on bone 0x1A, stiffness 0.4 / 0.9).
- States (branch 0x8117BD58, update 0x8117BD94): wait, walk, cheer1, cheer2, item_in.

### 4.2 Animations [obs]

| Clip | Frames | Use |
|---|---|---|
| em0960_0000 | 50 | wait (blend 5). Every 30 × U[3, 5] frames → cheer1 or cheer2 at random |
| em0960_0001 | 33 | walk. **Unreachable**: nothing sets its target +0xF58 [inf] |
| em0960_0003 | 197 | cheer1 → wait |
| em0960_0004 | 361 | cheer2 → wait |
| em0960_0002 | 80 | item_in (the toss) |
| em0960_c000 | 80 | **camera clip** of the toss (played through 800835D0(80602D24, mot, this, 3, …)) |

They have no SE, voice or effect cues. Event clips: ev05 / ev06 / ev07 (2 clips each, 241 f), ev09 (500 f), ev50 cuts 26–29 (840 f each, 3 instances).

### 4.3 Item toss [obs]
**Trigger:** cEm0623::chkItemInStart 81100D98, checked from the Baron's wait branch.
- The first call builds a 5-step threshold table (BSS 81500680) from the **player's** max HP: 20 %, 20 %, 25 %, 30 %, 35 %.
- A toss fires when all of these hold:
  - the player's HP (+0xEC) is below threshold[step]
  - the toss cooldown +0xE64 ≤ 0
  - the player is idle-ish: 81077C28(player), or player state 0x5D / 0x5E / 0, or 810F5470()
- Then `oiran[0].startItemIn(item[step])`, step += 1, cooldown = **900 frames**.
  - Item table 81159264: **it0501, it0502, it0502, it0502, it0502**.
  - With flag 80697E50 bit 0xB: item **it0500**, step += 2, cooldown 1350.

**item_in state** (8111E09C):
1. em0960_0002 + camera em0960_c000. Sets 80697E64 bit 10 and 80697E8C bit 1 (a scene / AI hold for the cut-in [inf]); turns to face the ring centre (atan2(−x, −z)).
2. Frame 16: creates the item object (8020B128) in her hand (bone 9, offset (−0.15, 0.05, 0)) at scale 0.5; random kicks on bones 0x19 / 0x1A; **announcer 0x11C**.
3. Frame 67: deletes the held copy and throws a free item (8020B128 mode 5) along her yaw at speed U[0.5, 0.8].
4. End: kills the camera, clears both flags → wait.

**Pickup:** the item is a normal world item; the player's generic pickup handles it [inf]. Which item heals how much (it0500 / it0501 / it0502; it0501 and it0503 are single `polymsh` models, it0500 / it0502 two-part) is **[unres]**. The same three items are resident in c201 / c202 (`常駐データ` it0500..it0502, it0510), so they are the standard recovery pickups [inf].

**During the fight** the oirans only idle and cheer. They do not move or attack [obs].

## 5. Mathilda (cEm0615, model em0610)

### 5.1 Gameplay class [obs]
- `[em0615]` `;BBC女キャラ`: **HP 1000**, CharColHeight 1.6, radius 0.3, hit size 0.6, same in hard.
- **isEnableTarget = 0**. onPreMove 810F9E74 discards any hit record, so **she is never damaged in the original**.
- Created by the Baron at (−8.2, −0.05, 0), yaw +90° (the ring side opposite the oirans). onPreMove creates **wp0582** once and attaches it.
- Springs: 802011E8 at +0xE48 on bone 0x19 and +0xEC0 on bone 0x1A (0.4 / 0.85..0.9).
- States (branch 0x81178E58, update 0x81178E7C):

| Id | State | Motion | Behaviour |
|---|---|---|---|
| 0 | wait | **em0610_0000** (50 f, blend 5) | faces the ring centre; every 30 × U[3, 5] frames picks a target (−8.2, 0.05, U[−6, 6]) (setMoveTargetPos 810FA644) → walk |
| 1 | walk | **em0610_0001** (33 f, blend 2) | walks along x = −8.2 until within 0.8 m (dist² < 0.64) → wait |
| 2 | atk_bat | **em0610_0010** (254 f) | started only by the Baron (startBatAtk 810FA0E8), §5.2 |

### 5.2 Bat combo (shoryuken catch) [obs]
1. Baron shoryuken (0107) connects with catch 0x27, so shoryuken sub 0x14: Jack is forced into **pl0030_1055** (254 f, flags 0x400); effect 0x23 on Jack and on Mathilda; she gets startBatAtk.
2. Mathilda plays **em0610_0010** with Jack paired again (pl0030_1055, flags 0x401).
3. Player side: fn_8029F80C plays pl0030_1055 + **camera pl0030_c493** (254 f).
4. Frames 74..160: every 16 frames the bat object's vfunc +0xA4(0x32) runs [inf: a hit / blood burst]. SEQ cues 4213:403 at 73 / 86 / 98 / 108 / 121 / 132 / 145 / 157, 4213:404 at 184; op-16 effect id 45 at frame 0.
5. Frame 80: announcer 0x11D.
6. At the end she is clamped back inside |x|, |z| ≤ 8.2, y = 0.05; then wait.
7. The Baron meanwhile holds 0000 and at frame 56 adds setLmtEffect(0xE).
8. Damage to Jack during the combo comes from the paired clip / player code [unres].

### 5.3 Complete Mathilda asset catalogue (for the custom Mathilda boss) [obs]

**Model:** `em0610.mdb` (27 bones, ids 0–26) in `em/em0610.dat` (event copy) and `em/em0615.dat`.
- Meshes: head, hair_front (+ _UR back face), side, leather, spike, sunglasses (+ _UR), skin, kuchi (mouth), hand_L / hand_R, ha_u / ha_d (teeth).
- Textures: tpl + `_AB` (ink).
- An explorer GLB already exists: `exports/madworld/evidence/node-export-em0610.json` (color; no `-source` export yet).

**Effects / hit data:** `em0610.eff` (12,544 B) in em0615.dat; damage volumes `em0430_00.emh` (shared).

**Prop:** `wp/wp0582.dat` (+ `wp0582.eff`). ev51 has a wp0582 actor with 21 clips (the bat in her ending scene).

**Gameplay motions (em0615.dat):**

| Clip | Frames | Cues | What it is |
|---|---|---|---|
| em0610_0000 | 50 | — | idle (wait) |
| em0610_0001 | 33 | — | walk cycle |
| em0610_0010 | 254 | op16 45 @0; SE 4213:403 ×8, 4213:404 | bat beat-down, paired with pl0030_1055 / camera pl0030_c493 |

**Event motions: 42 clips, 13,450 frames** (OBJ actor em0610 in `event/evNN.dat`; `exports/madworld/events.json`). In the table "@" gives the cut start frame on the event timeline.

| Event | Clips (frames @ cut start frame) | Context [inf] |
|---|---|---|
| ev02 | cut0 660, cut2 116 @755 | BBC (Man Darts props bl0504–0506) intro with the Baron |
| ev04 | cut0 542, cut2 155 @674 | BBC intro |
| ev0a | cut0 612, cut2 164 @677 | BBC intro |
| ev12 | cut0 880, cut2 152 @1048 | BBC intro |
| ev14 | cut0 1080, cut1 198 @1080 | BBC intro |
| ev26 | cut0 1337, cut2 350 @1414 | BBC intro |
| ev36 | cut1 480 @851, cut2 128 @1331, cut4 255 @1497 | BBC intro (em0625 head + it0107) |
| ev38 | cut0 786, cut1 49 @786, cut2 314 @835 | with em0461 |
| ev46 | cut0 684, cut1 260, cut2 42, cut3 110 | BBC intro |
| ev48 | cut0 1156, cut2 239 @1221 | BBC intro |
| **ev50** | cuts 9–18: 117, 111, 53, 53, 50, 43, 77, 37, 29, 76 (from @794) | the final-fight intro |
| **ev51** | cuts 0–4: 160, 38, 65, 73, 409 | the Baron's defeat |
| ev56 | cut0 867, cut1 32 @867, cut3 411 @981 | BBC intro |

All of these events also carry the em0620 Baron. Event clips are long, cut-local, world-placed performance takes. Use the Kreely method (trimmed `range`s, removed root origin) to turn them into gameplay clips (taunts, gestures, the ev51 bat swing).

**Voice / dialogue**
- **Mathilda has no gameplay voice block.** No 8xxx cue in her SEQs; scene_mix0503 has only the Baron's 8127.
- Her lines exist only inside the **event VCE streams**: `evvostr/evvo02 / 04 / 0a / 12 / 14 / 26 / 36 / 38 / 46 / 48 / 50 / 51 / 51c / 56.rsd`. English `_ENG` exists only for ev50 / ev51 / ev51c; the others are a single stream. Each stream mixes all speakers. Splitting out Mathilda needs the event MESS timing [unres].
- Subtitles: MESS in ev02..ev56 (`evidence/messages.json`, 7–12 lines each). The glyph atlases of these events are **not transcribed** yet (tokens only), so the text is [unres].
- Announcer lines about her: 0x11D (c503).

**Bones** are pl0030-compatible by id (0–24). Retargeting Jack / grunt / em0400 clips onto her, as Kreely does, is plausible [inf]; the bind directions are [unres].

## 6. Camera animations [obs]

| Clip | Frames | Played by | Format |
|---|---|---|---|
| `c503.dat cam00_Clip.cam` | 361 | room clip (stage-start broadcast / flyover) | `.cam` (the reader in `tools/stage_room_cameras.py`) |
| ev50 / ev51 CAM | 3325 / 3200 (31 / 26 cuts) | events | the event camera already staged for ev21 |
| em0623_c000 | 60 | keen sub 0x2A (030b) | **camera MOT** |
| em0623_c001 | 202 | keen sub 0x2C (030c, helicopter) | camera MOT |
| em0623_c002 | 171 | rider kick 0203 | camera MOT |
| em0623_c003 | 171 | angry 000c (phase 2) | camera MOT |
| em0960_c000 | 80 | oiran item toss | camera MOT |
| pl0030_c480..c48d, c490..c493 | 69–261 | player side of each clash step / bat combo / 1050–1052 | camera MOT |

**Camera MOTs** are normal `mot` files with 13 tracks:
- root (bone −1, channels 0–2)
- **bone 0xF60 (3936), channels 0–5**
- **bone 0xF61 (3937), channels 0–2**

All are fmt 6. The explorer's `native.motion()` **drops bones ≥ 0xF60**, so the current staging tools lose them. Channel semantics (eye / target / roll / FOV) are [unres].

They are played relative to the owner by **800835D0(80602D24, mot, owner, 3, 0, 1, 0, 0)** and stopped by 8008388C [obs]. RinRin's port did not need this path.

Player pairs [obs: DOL pools]:

| Camera | Player clip |
|---|---|
| c480 | 0480 |
| c481 / c482 / c48c / c483 / c484 | 0481 / 0482 / 0483 / 048a / 0484 |
| c485 / c486 / c489 | 0485 / 0486 / 0489 |
| c487 + c488 | 0487 |
| c48b | clash sub 1 |
| c48d | 048b |
| c493 | 1055 |

## 7. Staged plan

### 7.1 Usable as-is

**1. Arena world.** c503 transforms are yaw + uniform scale, which non-default rooms accept:
```
MW_ROOM=c503 MW_SPAWNSET=c50300 python -I tools/stage_arena.py     # -> assets/native/rooms/c503/world/
```
Then export the arena GLB with `tools/export_arena.mjs` (`MW_ARENA_OUT`) and run `tools/stage_arena_color.py` for c503, as for c303.

**2. Collision.**
- Copy `col.hkx`, `col_ef.hkx`, `col_obj.hkx`, `col_pl.hkx` from case5/c503.dat into `assets/native/rooms/c503/collision/`. The c303 folder was filled the same way; the extractor itself is a few lines, see §7.3.
- Then run:
```
MW_COLLISION_DIR=assets/native/rooms/c503/collision python -I tools/stage_collision.py
```

**3. Motion cues:** `research/motion-cues.json` already holds the em0615 / em0623 sequences.

**4. Baron rig, motions and GLBs.** First the explorer exports (server 127.0.0.1:8781, run from `D:/Games/gamesfordecomp/exports/madworld`):
```
node tools/node-review/review_export.mjs D:/Games/gamesfordecomp/exports/madworld em0623
MW_COLOR=0 node tools/node-review/review_export.mjs D:/Games/gamesfordecomp/exports/madworld em0623
python -I tools/stage_actor.py em0623 baron     # assets/native/baron-{source,color}.glb, baron-motion.json
```
The motion JSON also carries the pl0030_04xx / 105x and wp0600_048x clips. `FightData::new` moves the pl0030_* ones to Jack's bank.

**5. Bat, hat, sunglasses, helicopter:** `stage_actor.py` needs an mdb + motions per stem, so use it for wp0582 / bl0522 only after their GLB exports.

**6. Announcer, SE, BGM, effects:** `stage_announce.py`, `stage_se.py`, `stage_bgm.py` and `stage_effects.py` are usable once the parameter changes in §7.2 are in.

### 7.2 Needs a small change (hard-wired to RinRin / c201; add a parameter, not new logic)

| Tool | Hard-wired | For this fight |
|---|---|---|
| `tools/stage_actor.py` | assumes `<stem>.mdb` inside `<stem>.dat` | add an optional model stem: `em0615` + `em0610` (Mathilda), `em0965` + `em0960` (oiran), `em0626` + `bl0330`, `em0627` + `bl0331`. GLB exports of em0610 `-source` and em0960 first |
| `tools/stage_character_color.py`, `tools/stage_ink.py` | `MODELS` | add baron (em0623), mathilda (em0610), oiran (em0960). Recomp color-pack coverage is [unres] |
| `tools/stage_effects.py` | `BANKS` | add `em0623.eff`, `em0610.eff` (em0615.dat), `c503.eff`, `bl0522.eff`, `bl0519.eff`, `bl0333.eff`, `wp0582.eff` |
| `tools/stage_room_cameras.py` | `case2/c201.dat` | `case5/c503.dat` (cam00, 361 f) |
| `tools/audio/stage_announce.py` | `ROOM = 0x201`, `case2/`, `IDS` | ROOM 0x503, `case5/`, IDS 0x10E..0x11E + 0x1018, 0x1143, 0x1145..0x1147, 0x1022 |
| `tools/audio/stage_se.py` | `PACKS` | add `scene_mix0503.rsp`, `event/scene_mix7568.rsp`, `event/scene_mix7569.rsp` |
| `tools/audio/stage_bgm.py` | `TRACKS = ['bs04c','bs04']` | `bs03` (fight; 0x17 / 0x1D), `bs05` (phase 2, 0x19); maybe ev50 / ev51 |
| `tools/pl0000/stage_combat.py` | cues `('pl0030_','em0440_')`, em0440.csv, em0440_00.emh | add `em0623_`, `em0610_`, `em0960_` cues, a `baron_tuning` from em0623.csv, damage volumes from em0623_00.emh, ATHIT rows 0x161 / 0x172 / 0x17D / 0x17E / 0x18C / 0x18E + `EM0623_KEENCMP` |
| `tools/index_rin_states.py` | 32 × 3 at 0x81174800.., prefix `em0440_r1_` | Baron 24 × 2 at 0x81179368 / 0x81179488; Mathilda 3 × 2 at 0x81178E58 / 0x81178E7C; oiran 5 × 2 at 0x8117BD58 / 0x8117BD94 |
| `tools/events/stage_intro.py`, `stage_props.py`, `stage_audio.py`, `stage_effects.py`, `stage_ui.py`, `stage_writing.py`, `stage_messages.py` | ev21 paths, props pl0030 / pl0040 / wp0600, the EV21 voice path | `--event 50 / 51`; props listed below; voice `evvo50_ENG` / `evvo51(c)_ENG`; SE 7568 / 7569; BGM EV50 → BS03C(=bs03). The SE comparison needs its own capture |
| `tools/ui/stage_native_ui.py`, `stage_pause_map.py` | `case2/idpac_c201_us.dat`, c201.imt | `case5/idpac_c503_us.dat`, `c503.imt` |

Props for ev50 / ev51: em0620, em0621, em0622, em0610, em0623, em0624, 3 × em0960, bl0330, bl0331, bl0519 × 2, bl0817, bl0818, bl081c, wp0582.

### 7.3 New tooling
1. **Camera MOT reader:** decode the bone 0xF60 / 0xF61 tracks (the explorer `motion()` skips them), find their meaning in 800835D0's consumer, and play them relative to the owner. Needed for the item toss, angry, rider kick, the helicopter and every clash step. Validate against a native capture.
2. **Pair driver generalisation:** vtable +0x450(target, mot, 1, flags 0x400 / 0x401) forces Jack into a clip. This fight has a **3-actor chain** (Baron catch → Mathilda + Jack) and Baron ↔ Jack swaps per clash step. Extend rin_keen / the pair code (fight.rs) into a table keyed by sub-state.
3. **Clash mode 0x16 player port** (fn_8029D350 / fn_8029F164): slow-motion time scale, swing-direction prompts (8022F284), the 048b meter, the 0xFA / 0xFB..0xFE / 0x100 / 0x104 codes. Mode 0xB (RinRin) and 0x18 (Elise) are the precedents.
4. **Room objects:** bl0522_N001 / N002 visibility (8017F710), the helicopter spawn with `bl0522_030c`, bl0557 and bl0333 spawns, hat / sunglasses detach (bl0330_0000 / bl0331_0000).
5. **World items:** item object create / throw (8020B128 modes 0 / 5) and pickup + heal for it0500 / it0501 / it0502. The heal amounts and the item class are [unres]: trace or decompile the cObjItem path.
6. **Springs / hair:** cHair (80201B40.., the oiran kanzashi) and the 802011E8 jiggle springs on bones 0x19 / 0x1A (Mathilda, oiran). Map them onto `play_springs.rs`, tagged as added where not translated.
7. **Collision extractor:** a small DAT member copier for `col*.hkx` into `assets/native/rooms/<room>/collision/`.
8. **Native evidence:** an oracle run in c503, logging:
   - 80123820 arguments
   - CreateObj 0x20623 / 0x20615 / 0x20965
   - the 80616CCD byte and 80697E50 bit 0xB
   - 801246D8 / PlayBGM_EX
   - startCatchAtk codes
   - the finisher after the helicopter

### 7.4 Custom variant: Mathilda as the boss
1. **Stage:** stage_actor (with the model-stem change) for em0615 → `mathilda-motion.json`; color / ink as above; wp0582 as her weapon (attach offset (−0.12, −0.005, −0.07), §1.1).
2. **Own moves are few:** idle, walk, bat beat-down. The bat beat-down is a ready-made grab finisher / special: it already has a Jack pair, a camera and SE.
3. **Extra moves**, all tagged as added:
   - event-clip ranges (ev50 cuts 9–18, ev51 cuts 0–4 with the bat, the BBC intros' gestures) cut with the Kreely `range` / `rootOrigin` method
   - retargeted pl0030 / em0110 / em0400 clips (same bone ids)
   - the Baron's clips would need em0623 → em0610 retargeting (different bone set 30–33 / 42 / 117–120) [unres]
4. **Data:**
   - HP 1000 (em_common `[em0615]`). Collision h 1.6 / r 0.3, hit 0.6.
   - Make her targetable (the original returns isEnableTarget 0).
   - Sound: SE 4213:403 / 404 (scene_mix0503); announcer 0x11D.
   - No gameplay voice: event VCE excerpts are possible after the MESS timing is decoded.
5. **AI:** reuse the Baron's state machine shape (wait / step / attack / guard / clash) with her clip set. Put the oirans and the Baron at ringside the way the original puts her there [design choice].

## 8. Rust generalisation notes (no code changed)
`mw-sim/src/fight.rs` and `mw-game/src/play.rs` are RinRin-specific (see ELISE-BOSS.md §10 for the full list). For this fight also:
- a ringside-NPC system (non-targetable actors with their own small state machines: Mathilda, 3 oirans) owned by the boss
- per-boss child objects (hat / sunglasses parts)
- a phase flag (+0xE58) driving the BGM switch to bs05
- an HP floor before the first clash
- a player-HP-driven item director (§4.3)
- camera-MOT playback (§6)
- the c503 stage descriptor:
  - room c503, spawn set c50300
  - PLP start (0, 0, −6)
  - intro ev50, outro ev51 + ev93
  - BGM bs03 / bs05
  - announce room 0x503

## 9. Open questions (ordered)
1. The kill after the helicopter (pl0030_1050 / 047c / 1051 / 1052 roles; how piyori sub 99 ends), and which player inputs map to 0xFA vs 0x104.
2. `80616CCD` bits 4–5 (the oiran spawn condition) and `80697E50` bit 0xB (it0500 / faster timers).
3. The heal values of it0500 / it0501 / it0502 and the item pickup path.
4. Camera-MOT channel semantics (bones 0xF60 / 0xF61).
5. The ev51 vs ev51c (BGM / VCE) selection, and the ev51b string.
6. Mathilda's lines and subtitles in the event VCE / MESS (untranscribed glyph atlases).
7. Damage dealt to Jack during the bat combo (from the paired clip or the player code).
8. The bl0557 / bl0333 object roles; the 80697E64 bit 10 and 80697E8C bit 1 holds.
