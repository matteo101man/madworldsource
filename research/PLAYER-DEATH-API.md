# Player death (state 95) API

Source authority: original DOL `b1a42fd6acb9dfe076b583b03b610ca44a65f532`, Ghidra high P-code graphs listed below. Bounded pseudocode only; no raw dumps.

## Dispatch

- Player state table: 192 CodeWarrior member pointers `(0,-1,fn)` at `0x805CBFCC`, two 96-slot halves indexed by byte `+0x11B` (rno1). Half B is the per-state update. Cross-check: B[78] = `0x8028C074` (known competition dispatcher). The earlier unwind-metadata lead (`0x8005724C`) is unrelated and stays rejected.
- State 95: A[95] = `0x802C0F7C` (4-byte stub), **B[95] = `0x802C0F80` update (740 bytes)**.
- Entry: `cPl0000_frameUpdate` (`0x802727B0`) after a lethal hit stores HP `+0xEC`=0, clears `+0x1124` bit 19, calls `changeState` `0x802CBF50(this,1,0x5F,0,0)` (saves previous `+0x11B` to `+0x1562`, then `cObj::setRno`), pad vibration, `setHitStop`. `0x802745F0` is a second entry with the same sequence.

## State 95 update (`0x802C0F80`), phase byte `+0x11C`

Every frame: `+0x1124 |= 0x20`, `+0x1128 |= 0x8`.

| Phase | Behaviour |
|---|---|
| 0 | Death motion `pl0030_108a.mot` / `pl0030_108b.mot` (selected by `+0x1AB8`), `changeCameraAngle(…,2)`, conditional voice streams 0x1144 / 0x12C via flags `0x80697E8C` bits 8/3, camera-motion release, phase→1 |
| 1 | `+0x1230` countdown = 0xF0 or 0x78 >> (`+0x11A8`&0x3F) by flag 3; set flag object `0x80697E78` bit 7; **`rpn send(this,"onDead")`; `send(cMessageTarget("subMission"),"onPlayerDead")`**; advance motion; phase→2 when motion ends |
| 2 | Decrement `+0x1230` to 0; set `+0x1110`=1, `+0x398`=0; nothing else |

The player handler never leaves state 95 by itself. Game over / retry must be driven by the listener of the `onDead` / `onPlayerDead` messages.

## rpn named messages

- `0x8005B6A4 send(target, name)`: builds a tag, `cListenerBase::find` in the global `cMessageHandler<void()>` tree, invokes `0x8005B724` on `listener+0x1C` only when found. A missing listener is silent.
- `0x801133DC cMessageTarget(const char*)` interns the name through `0x80059520(0x8060258C, …)`.
- The `onPlayerDead` string occurs 11 times and `subMission` 27 times in the DOL. No direct `lis/addi` references exist (pooled), so the listener registrations are not yet located.

## Flag objects (bit arrays: word0 = bits pointer, word1 = word count)

- `0x80697E78`: clear `0x8006F44C`, set `0x8006F55C`, check `0x8008B5E4`.
- `0x80697E64` (bits at `0x80697EC4`): clear `0x8005A7A8`, set `0x8005A82C`, check `0x8006A66C`. Bits 10 and 18 become set at death (observed word 0x40400). `0x80086F34` clears both; `0x80086D18`/`0x80086DBC` toggle a stop byte and counter `0x80604088`. `0x80197314` (thrown-object physics) only uses 10/18/24 as stop guards and is not the game-over consumer.

## Runtime observation (bevy-native-sustained-battle, ended early with no final JSON)

Death at control-ready ≈124.6s: state (1,95,1), `0x80604020` demo counter 0→−1, flags 0x0→0x40400. Phase 1 countdown runs 50→0 while `pl0030_1015.mot` plays to frame 68. Phase 2 holds for ≥258s with scene 201 still updating. No game-over/retry transition was observed.

## Runtime listener trace (bevy-native-rpn-death, MW_TRACE_RPN=1, 330s sustained)

The default-off hooks `0x8005B6A4`/`0x8005B724` (`runtime/game/trace_boot.cpp`) log each send and whether a listener was invoked. 67 sends were logged. Only `cameraFix` and `lock` reached a listener, which confirms the tracer works. **No listener** was found for `onStartupScene`, `onBoss`, `beginDemo`, `onBeginEvent`/`onEndEvent`, `onUpdateScene` (28), `onTransScene` (28), `onDead` (player), `onPlayerDead` to `subMission` (caller `802C11DC`), or `onPlayerDead` from `8006A3C4` (tag = Rin object). The test exits FAILED only on the "PC action sequence did not complete" assertion, which cannot pass after player death; no crash occurred (334.98s).

String pools place `onPlayerDead`/`onContinueRestart`/`BossSw`/`invoke`/`roomClear`/`execRoomEvent` in room scripts (`rpn::cRoomCommon`, `cRoomEx`, `room::cRoomMode_Boss`); stage 201's class is `room::RoomMode_C201Boss::cBoss`. Continue/retry therefore belongs to the room-script layer, whose scene-message listeners are absent in this boot. Registration goes through `cListenerBase::set` `0x80113900` (190 direct callers, all generic template wrappers; names are passed through stack buffers, so static name resolution needs one more caller level). Leads: `rpn-listener-set-callers.json`.

## Listener registration trace (bevy-native-rpn-set, 90s, MW_TRACE_RPN=1)

Hook `0x80113900 cListenerBase::set(this, tree&, const cMessageTarget&, const char*)` logged 286 registrations. Names seen: accept? blood cameraFix clear cover get getRemainTime hasRemainTime hideAll invoke isOtama isSignpost lock on onCatchObj onCatched onDrop onEmEvent onEndEvent onHitCol onHitWall onPreCatched onRelease onShake remain? reset throw unlock. **None** of onPlayerDead / onDead / onContinueRestart / roomClear / onUpdateScene / onTransScene / onStartupScene / onBoss / beginDemo registered. The stage-201 scenario objects did run: `cC201` (ctor `0x801447EC`, vtable `0x805a5b00`) and its boss child (`0x80145050`, vtable `0x805a5b38`) registered exactly the names the trace shows for them (`onCatchObj` ×2 with tags 0x7006c/0x7002c, `onEndEvent`, `invoke` via `0x80145934`). So the diagnostic boot does not skip stage-201 room setup; the earlier hypothesis is withdrawn.

## String-pool addressing (why names are not `lis/addi`-visible)

CodeWarrior loads a per-translation-unit pool base once (`lis/addi` to the pool start) and addresses each literal as `addi rX, rPool, off`. `research/section11-pool-refs.json` indexes every pool base in data section 11 to its referencing functions; `research/room-pool-strings-by-function.json` lists the strings each room-class function uses. Key pools: scene/message helpers `0x80534be0` (`msg::` senders only: onStartupScene `0x800A821C`, onUpdateScene `0x800A8254`, onTransScene `0x800A828C`, onPlayerDead `0x800A8E04`, roomClear = `msg::BossResult` `0x800A9E20`), room-event framework `0x805417e0` (`execRoomEvent`, `onContinueRestart`, `roomClear`), stage-201 pools `0x80544760` (cC201 boss child) and `0x80544f00` (C201 sub-mission), sub-mission base `0x8054bc7a` (registers `subMission`/`onPlayerDead` only while a sub-mission is active, `0x80176498`).

## Original death → continue flow (static, Ghidra C of the listed functions)

1. State 95 phase 2 sets player `+0x1110 = 1` once `+0x1230` reaches 0 (`0x802C0F80`).
2. Predicate `0x801248A4` ("player dead"): player (from list `0x8069bca8`) has HP `+0xEC <= 0` or `+0x1110 != 0`; also true when flag `0x80697e8c` bit 9 is set.
3. Scenario-base update `0x801286FC` (vtable slot shared by every room scenario, e.g. `cC201` `0x805ae0b0`): `if dead && !IsDemoMode() && !flag(0x80697e8c,3) && !flag(0x80697e8c,9)` → allocate 0x40 from `this+0x28` and construct the dead-event task `0x80127BC4(obj, this, -1)` (vtable `0x805a44b4`, member-pointer callback `(0,-1,0x80127DFC)`).
4. `0x80127DFC` is the continue sequence on the room-event fiber: wait flag 0x22 clear; `BeginDemoMode(0)` under a scoped guard (dtor `0x8005A634` → `EndDemoMode`); `0x801241A0`; player `+0x1A0 |= 4`; if continues `0x8017FA10(0x80616880) == 0` → game-over window (`0x80244DBC(ui,1)`, wait `0x80244E1C`) then `0x80127900`; else continue window `0x80244DBC(ui,0)`, wait, if `0x80244E44` (yes) → `0x8017FA40(…,1)` consume a continue, `0x801241F4`, reset children (`0x80128B28(0x65)` → virtual +0x10), **player restart `0x802C1264`** (HP `+0xEC = +0xEE` max, `+0x1110 = 0`, clear flag object `0x80697e78` bit 7, `changeState(1,0x5E)` or `(2,1)`), respawn point (`+0x1158` default or `setPlayerPoint(0x8069bca8, event&0xff)`), **send `onContinueRestart`** (`0x8005B674(0x80541830)`), fade `0x8012365C`, `0x8024482C`, `0x80086E60(8)`. GameOver UI: `0x80234808`/`0x8023491C` (`GameOver` pool), `0x80233690` (`cIdGameOver`).
5. `IsDemoMode()` (`0x80086DF8`) is `(int)0x80604020 < 0`. `BeginDemoMode(bool)` `0x80086C9C` decrements the counter (first entry also stops `0x8060401c` via `0x80086D18` and sends `beginDemo`); `EndDemoMode()` `0x80086D58` increments (last exit sends `endDemo`). Player-side guard pair: `0x80280F84` begin (arg 1, callers `0x80280D00`, `0x80280FCC`, `0x802AF28C`, `0x802CB778`) / `0x802788B0` end (callers `0x8027621C`, `0x80280FCC`, `0x802AF28C`, `0x802CB8A8`).

Demo-mode trace (bevy-native-rpn-demo, 330s, player survived, no death): boot demo nesting reached −6 (callers `801473A8`, `802D3CF0`, `8016CE4C`, `801470E0`, `80147BE4`, `801240C0`; Rin intro), all unwound by scoped-guard dtors (`8005A664`) and `800648A8` to 0 at control-ready; during the fight the player guard pair `80280F84`/`802788B0` (callers `80280FAC`/`802788D4`, arg 1) toggled 0↔−1 in matched pairs. The earlier death (bevy-native-rpn-death) happened from state 39 at ~170s with `competition=false`, so the death-time begin is not the clash guard (`802CB778`).

**Resolved by trace (bevy-native-rpn-demo2, 600s, death at ~330s from state 39):** the death-time `BeginDemoMode(0)` (`counter 0 → −1`, caller `80127E50`) is the one *inside* the continue sequence `0x80127DFC`. So steps 1–4 do run natively: the dead-event task is created and the continue sequence starts, then parks in its window wait. The earlier reading of the −1 counter as a stuck demo mode was wrong. What is still missing is the window result: the sequence waits on `0x80244E1C` (widget byte0 set when the window closes), and the headless harness never produces a fresh confirm press after death (`pad-down` stays 0 in sustain mode; it keeps holding the camera key). `0x80244DBC` resolves layout widget id 3 via `0x802233A8(0x8069d11c, 3)`, which returns 0 when the layout manager has nothing (`0x80223774() == 0`) — then the wait would never end; `[continue-ui]` hooks log the resolved widget and `getContinue()` (`0x80616880+4`) to tell the two cases apart.

## Verified native defeat → continue → restart (bevy-native-death-fixture, 150s)

Fixture: `MW_TEST_PLAYER_HP1=1` (core, `trace_boot.cpp` scene-update hook) sets player HP `+0xEC` to 1 once the intro demo nesting is 0; the next original hit runs the lethal path (explicit, test-only, never natural). Harness knob `MW_GPU_TEST_PC_CONTINUE_CONFIRM=1` (`native_gpu.rs`) stops battle inputs when the read-only feedback reports HP 0 and pulses the PC confirm binding (Enter → core button bit 8 = Wii Remote A) so the window gets fresh press edges.

Observed sequence (control-ready seconds): 30.4 HP 1 → 32.4 state 95 phase 1, `onDead`/`onPlayerDead` sent → 36.5 phase 2 → continue sequence `0x80127DFC` begins demo (`80127E50`), **`[continue-ui] window mode=0 widget=90ec7480 continues=2`** (`getContinue()` = 2, so the continue window, not game over) → confirm accepted → fade `0x80086E60(8)` (`begin 80086EA0` / `end 80086F04`, counter back to 0) → 40.5 **HP 100, state 94 (0x5E respawn)** → 46.6 state 4 → 48.7 state 39, fight resumed. One full cycle (the only death in the run). The runner's "final-state" snapshot is taken 10s before the test deadline, which here fell inside the death window (state 95/2, `demoCounter −1`), i.e. before the restart; it is not a second death.

Conclusion for acceptance item 1 (defeat/retry): the original room-event continue flow is intact in the native core and completes with PC input. No gameplay code was changed; the earlier "stuck" observations were a missing confirm press in headless runs. Still open: the game-over branch (`getContinue()==0` → `0x80244DBC(ui,1)` GameOver window → `0x80127900` → scene `0xE01`), which needs a save-data fixture to exercise; victory (`msg::BossResult`/`roomClear`) is a separate path; and the PC-facing prompt text/UI replacement for the Wii "press A" is UI work, not state logic.

## Game-over branch (bevy-native-gameover-fixture, continues forced to 0)

Companion fixture `MW_TEST_PLAYER_CONTINUES=0` writes `cSaveData` `0x80616880+4` once (test-only). With `getContinue()==0` the continue sequence took the other branch: `[continue-ui] gameover banner arg=0` (`0x80245370`, layout widget 0x11, caller `80127F0C`), then waits for the banner (`0x802453D4`), shows the GameOver window (`0x80244DBC(ui,1)`), waits, and runs `0x80127900` → `0x80127970` (save-data reload from the stage-select template `0x805a8938`), `0x8017FDE8`, `0x80181BB8(0x80653eb4, 0xE01, 0)` → scene-change request `0x80200AA4` (next scene `0xE01`, flags `0x80697e50` bits 0xc/0xe). Result: the branch completed natively. `[continue-ui] window mode=1 widget=90ec7340 continues=0` (GameOver window), confirm accepted, continue-sequence demo guard released (`end counter −1 → 0`), then the original scene change ran: `[world] begin scene=e01` → `end`, then `begin scene=b02` → `end`, with the scene update continuing and no fault (only pre-existing unhandled MMIO reads/writes at `0xCC00401C`/`0xCD0060xx`, which are not new). So defeat with no continues leaves stage 201 through the original post-fight transition.

## Victory chain (static)

Stage-201 boss fiber `0x80147BBC` (`TraceNativeBossEntry`): `BeginDemoMode`; intro camera `0x8012409C(0)` (aborts if the player dies); `0x80133CF8` sends `BossSw/on` and `onBeginBoss`; `0x80133C58` waits for the boss list (`0x801E3868(0x80694a84,0)`); event `0x21` (Rin entry demo); `0x801265E8`/`0x80124DDC` spawn; `setPlayerPoint(…,5)`; `0x801246D8(0x18)`; fade; `0x80064884` ends the intro demo guard; then **`0x80133AA4` waits until `0x80133BF0`**: boss object `+0x3a` (set by the `onBossDead` listener `0x80133F6C`, registered by the "boss" target object `0x80133D54`) or no boss-class (`0x20000`) enemies remain; then sends `subMission clear` and `roomClear` (`0x8005B674`) and calls its own finish virtual. `cRoomEx` handles `roomClear` in `0x8012896C`: sets stop flags 9 and 10 in `0x80697e64` and queues the clear task (callbacks `0x80127A78`/`0x801289DC`). Fixture for this path: `MW_TEST_RIN_HP1=1` (`rso.cpp`, Rin `+0xEC` → 1 once the intro demo nesting is 0; the original player hit decides the kill).

## Victory path runs

- bevy-native-victory-fixture (200s, `MW_TEST_RIN_HP1=1`): fixture set Rin `+0xEC` 1600 → 1 at control-ready. Ordinary hits did not reduce her further: at low HP Rin enters state 17 `keen_compe` and the player state 78 (competition; `modeWord27e4 = 0xFFFF`, phase 1 → 3 by +91s). The boss kill is therefore the keen-competition finish (earlier sustained runs completed phases 1/3/5/6/7 and applied 300 HP), not an HP-0 hit; the 200s budget ended inside phase 3. No fault.
- bevy-native-victory-fixture2: same recipe at 330s, stopped by hand at the user's request (owned PID verified) before the competition finished; its evidence files are partial.
- bevy-native-bossdead-fixture: `MW_TEST_BOSS_DEAD=1` issues the original `send(cMessageTarget("boss"), "onBossDead")` (DOL strings `0x805a4dfb`/`0x805a4de9`) 300 scene updates after the intro nesting unwinds — the same message Rin's death path sends — to exercise the result/`roomClear` transition directly. Result recorded below.

## Boot-path caveat

The production frontend (`crates/mw-game/src/native_core.rs`) always sets `MW_BOOT_SCENE=0x201`, `MW_BOOT_POINT=5`, `MW_BOOT_BOSS=1`: it rewrites the first world argument E00→201 and invokes the boss switch callback. The original mission/menu progression that may register `subMission` listeners is therefore not followed.

## Unknowns

- Which object registers (`subMission`, `onPlayerDead`) and what it does (continue menu, retry, scene change).
- Whether a menu would wait for a fresh pad edge (the sustained harness only holds buttons; pad-down stays 0).

Graphs: `player-state95-pcode.graph.jsonl`, `player-death-pcode.graph.jsonl`, `rpn-message-pcode.graph.jsonl`, `game-flag-pcode.graph.jsonl`, `game-flag2-pcode.graph.jsonl`, `player-continue-pcode.graph.jsonl`; caller leads `native-continue-callers.json`, `native-game-stop-callers*.json`.

## Test acceleration (MW_TEST_TIME_SCALE)

`ppc_timebase()` is the host `steady_clock` scaled to 60.75 MHz, and every device model (decrementer, AI, DSP, DI latency, OS events) keys off it; the only host sleep is the idle wait in `os_wait_for_event`. `MW_TEST_TIME_SCALE=N` (integer 1..16, default 1 = original rate, test-only) multiplies the guest time base and divides the idle sleep by the same N, so boot, loading and the intro choreography run N× faster with game logic untouched. Wall-clock-driven host parts (test harness `seconds`, feedback sampling, VI scanout) are not scaled, so harness phase timings cover N× more guest time; feedback-driven modes are unaffected.

Calibration: 2× boots to the Rin intro event in 60s wall (≈2× speed-up, 756 log lines, no fault). 4× loads the world but scene updates then crawl (no BossSw callback within 75s); 8× never reaches the world loader in 60s. Cause: `mw_dsp_tick` must resolve every elapsed hardware-scheduler deadline (DSP batches, ARAM/AI DMA, VI half-lines) and the DSP interpreter runs only ≈2–3× real time, so above 2× the main thread starves in catch-up. Truly instant tests would need a guest snapshot/restore at control-ready, which the runtime does not have; 2× is the safe setting (boot ≈45s instead of ≈95s).

## Normal-speed finish fixture (continuation)

`bevy-native-finish-normal`: 200 s at 1x, MW_TEST_RIN_FINISH_WAIT=1,
MW_GPU_TEST_PC_FINISH_CONFIRM=1, RPN/world/object tracing, sustained PC inputs.
Enter initiated the original finisher: Rin reached state 26/phase 3; Jack reached
state 68/phase 13/selector 3. No boss death, roomClear or post-fight transition.
Exit 101 is the harness's final PC-phase==released assertion (last phase
finish-confirm), not a game crash. This is not a passing victory/integration test.

Read-only samples at wall 161.4 through 206.6 s show Jack+0x1088=0 and +0x1248=0.
Enter alone does not drive the finisher's motion gate. The correct shared PC
gesture still needs runtime verification.

Proven table/branch chain: B[68] at 0x805CC77C is (0,-1,0x802A4A18).
That dispatcher uses byte+0x11D; selector 3 tail branch at 0x802A4A54 targets
0x802A688C (size 0x730). Phase 13 starts at 0x802A6BF4: accumulates +0x1248
using +0x1088, updates/clamps +0x1160, drives motion/camera blend, advances
phase byte+0x11C after its threshold, and requests remote swing navigation kind 5.

Typed graph: player-rin-finish-pcode.graph.jsonl; bounded query:
player-rin-finish-phase13-pseudocode.json. The preliminary dispatcher graph
player-finish-gate-pcode includes followed tail targets outside its tiny bounds;
only its in-bound operations support dispatcher claims. The dedicated
0x802A688C graph has no excluded outside-function operations.

Follow-up (2026-11-08): phase-13 predicate, input chain and PC proposal resolved read-only in VICTORY-MOTION-INPUT.md / victory-motion-input.graph.jsonl. Phase 13 reads only +0x1088 = mean8|remote accel magnitude - 1g|; the finish-confirm harness clears X.
