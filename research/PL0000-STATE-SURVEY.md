# cPl0000 (Jack) state machine: survey for phase 2

Written 2026-11-08 from a read-only pass over this repository's existing research (no new disassembly). Labels:
**[N]** native (DOL/RSO/disc), **[O]** observed in oracle feedback captures, **[I]** inferred.

## Dispatch [N]
- `802727b0` `cPl0000_frameUpdate` (vtable `0x805CDFD8`) calls `fn_8027621C`, the per-frame player update.
- `fn_8027621C` clears bits in `+0x1124` (`&0xFFFDFFEF`) and `+0x1128`, then calls **A[state] and B[state] every
  frame** through `__ptmf_scall` (`802E57C0`); table at `0x805CBFCC` (A, 96 entries) and `0x805CC44C` (B). After the
  dispatch it sets byte `+0x1113 = (state != 72)` (recomp_055.cpp:10190-10220). A is therefore a per-frame pre-update,
  not an on-enter hook [I from the code]. Full table: `research/pl0000-state-table.txt` (96 rows, address/size).
- `802CBF50` changeState(this, mode, state, phase, selector): saves the old `+0x11B` to `+0x1562`, calls
  `cObj::setRno` (`802096f8`), which writes `+0x11A` mode, `+0x11B` state, `+0x11C` phase, `+0x11D` selector
  (single bytes; never read `+0x11C` as 32-bit).

## Observed states [O] (7204 feedback records, 33 `evidence/*feedback-observations.jsonl`; not frame-atomic)
| State | Motions | Meaning |
|---|---|---|
| 0 | pl0030_0000 | idle |
| 4 | 00ce, 0427 | locomotion |
| 7 | 0033 | |
| 11-15 | 00a0, 00a1, 00a2, 00a5, 00a6 | jump family (00a1 = jump attack) [I] |
| 21 | 0226 | |
| 39 | 1100-1105, 1122, 1123 | combat: LMB 1100-1103 string, RMB chainsaw 1104 (INPUT-HLE) |
| 68 | 1305, 1306, 1308 | finisher (VICTORY-MOTION-INPUT: selector 3 -> 802A688C, phases 10-17) |
| 78 | 04aa-04ad | keen competition / clash (`802956B4`, PLAYER-COMPETITION) |
| 85 | 100f, 1024-1026 | |
| 91 | 1097 | |
| 92 | 1011, 1013-1018 | |
| 94 | 1090-1092 | respawn (restart `802C1264`: HP = max, changeState(1,0x5E)) |
| 95 | 1015, 1016 | death (PLAYER-DEATH: phases 0-2, `+0x1230` countdown) |

Observed transitions: 39 -> 68 / 11 / 3; 95 -> 94 -> 4 -> 39.

## Fields [N]
cByMotion at `+0x1AC` (resource `+0x54`, frame `+0x58`, previous frame `+0x5C`); pad copy `+0xEE0`, remote accel
`+0xEF0`; facing `+0x74`; movement basis `802C6EEC` writes camera yaw to `+0x1134` (read by 20+ handlers); lock-on
target `+0x1254` (`SetLockOn_Trg` 802cbdc8 family); HP `+0xEC` / max `+0xEE` (s16, observed max 100).
Motion names are literal DOL strings, so each handler's motion choice resolves statically from its string-pool refs
(`research/section11-pool-refs.json`).

## Sources for translation
- Recompiled C++ of every function: `tools/upstream-native-core/port/generated/recomp_*.cpp` (+ `func_table.cpp`).
- Pcode graphs: `player-input-pcode`, `player-continue-pcode`, `player-death-pcode`, `player-state95-pcode`,
  `player-state-pcode` (8028C074), `player-competition-pcode`, `motion-state-pcode`; query with `tools/query_boot.py`.
- Oracle: `tools/read_native_core.py`, `watch_owned_pc_motion.py`.

## Gaps (phase 2 work list)
1. Translate handlers for states 0, 4, 39 first (idle / locomotion / combat): B[0] `8027AEC4`, B[4] `8027D2D8`,
   B[39] `80285744` (0x2380 bytes, largest), with their A halves; resolve each one's motion strings.
2. Index the callers of `802CBF50` to build the transition graph (input/condition -> changeState).
3. Recover walk speed / turn rate (currently root motion only + an inferred turn rate), lock-on and combo
   (`ComboCoutUp` 802cba00).
4. Name the jump and dodge states (11-15 look like jump; dodge 1123 was observed inside state 39).
5. Build the oracle input-script diff: `mw-play` already logs per tick (MW_PLAY_SCRIPT / MW_PLAY_LOG); the
   recompiled-core side needs the same script fed through `native_input.py` and a frame-atomic state/motion read.
