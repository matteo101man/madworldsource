# cPl0000 locomotion: translated (phase 2, part 1)

Translated 2026-11-08 into `crates/mw-sim/src/player.rs` (unit-tested) and driven by `mw-play`. Sources: headless
Ghidra decompiles of the owned `main.dol` (`tools/pl0000/decompile.sh`, output kept out of the repo), constants read
with `tools/pl0000/dol.py`, and the recompiled C++ (`tools/upstream-native-core/port/generated/recomp_055.cpp`) to
check single instructions. Labels: **[N]** native, **[O]** oracle-observed, **[I]** inferred.

## Frame structure [N]
`fn_8027621C` (per player frame): counts the dash window `+0x1504` down (at 0 it clears flag bits 0x2|0x4 of
`+0x1124`), then calls A[state] and B[state] from the tables at `0x805CBFCC` / `0x805CC44C`.
A = transition checks, B = motion selection + turning. changeState (`802CBF50`) only writes the 4 state bytes
(`+0x11A..+0x11D`), saves the previous state at `+0x1562`, clears `+0x1282`; no enter hook.
The recompiled-function index with every changeState call site and its constant arguments:
`research/pl0000-recomp-index.json` (`tools/pl0000/recomp_index.py`; 173 functions in 0x8027-0x802D call it).

## States
| State | A | B | Translated behavior [N] |
|---|---|---|---|
| 0 idle | `8027AD20` | `8027AEC4` -> `8027AFE4` | phase 0 plays `pl0030_0000` (blend `+0x1148`, rate 1); turn toward the last target |
| 1 turn relay | `8027C504` | stub | one frame, then walk/run (no turn allowed from here) |
| 2 walk | `8027C600` | `8027C744` | plays `pl0030_00c2` (blend `+0x1148`), rate 1, turns toward the stick |
| 4 run | `8027D164` | `8027D2D8` | phase 0 plays `pl0030_00ce`, speed `+0x1164` = 1.025, timer 150>>1; phases 1/3 rate = speed |
| 4 phase 3 dash | (`802C2B0C`) | `8027D2D8` | restart `00ce`, layer `pl0030_00cf` (weight 0), speed 1.1; each frame speed += 0.03, while <= 1.3 rate = 1.4 x speed, layer weight = 3.33 x (speed - 1) |

## Checks [N]
- Stick `+0x1144` is a **squared** magnitude: idle -> move at >= 0.04 (0.2^2), walk/run -> idle below 0.0256
  (0.16^2), walk/run split at 0.9025 (0.95^2), flick at 0.64 (0.8^2). `+0x113C` stick world yaw, `+0x74` facing.
- `802C2434` walk / `802C2528` run: when the second argument is set (idle phase 1 only) and the stick is 30-150 deg off
  the facing, go to state 1 instead; lock-on (`+0x1251`, `+0x1254`) suppresses that.
- `802C2B0C` dash: first push past 0.8 opens a 13-frame window and snapshots the stick; release inside the window
  marks "released"; a second push past 0.8 on the same dominant axis with the same sign dashes.
- Turning (`802C7144`, `802C71DC`, `RadLerpRange 800BB634`): target = stick yaw when the stick is past 0.2;
  facing += clamp(0.3 x `+0x1150` x wrap(target - facing), +-pi/2). `+0x1150` = GameVSyncF = 2.0 at the shipped
  interval 2, so 60% of the remaining angle per frame.
- Default blend `+0x1148` = 5.0 at interval 2 (`80569F6C`, chosen in `main` at `80270D3C`; 10.0 at interval 1).
- Sprint-layer fade when not dashing: w <- 0.4 x (-w), below 0.1 -> 0 (literal; the layer vanishes on the first
  non-dash run frame).

## Not translated yet
- Weapon-type branches (`+0x1508[+0x1500]` types 3/6/8/9/10-12: other idle/run motions, wp0600/wp0760 props).
- Idle fidgets: translated (2026-11-09, `player.rs` b_idle, 8027AFE4 phase 1): weapon type 0, not paused
  (`80086DF8`), and `+0x220` (taken as "the motion reached its end" [I]); the countdown `+0x1238` = rand(0, 2) + 10
  idle loops, then slot `+0x1234` steps 1 -> 0008, 3 -> 0007, 0 / 2 -> 0000 + new countdown, at blend 4.0, with
  announcer voice 0x1117 (PlayVoiceStream). The oracle shows 0007 / 0008 under state 0 [O].
- Combat entries checked before locomotion in every A half: `802C32CC`, `802C2610` (Wii-remote shake -> state 7),
  `802C2E68` (-> 34/14/11, jump family), `80284B34` (-> 31), `802C3B5C` -> `802C3C0C` (attack dispatcher: 39, 25,
  38, 41-49, 51-62, 65-77), `802C3338`, `802C68AC` (-> 84), `802C5B8C` (dash follow-up). mw-play still enters
  punch/dodge/chainsaw from keys with the clips observed in state 39 [O].
- Root motion of the `00cf` layer (mw-play moves by the base `00ce` root at the native rate) [I].

## Evidence
- `cargo test -p mw-sim player` (4 tests: run/walk/idle hysteresis, turn relay + facing, dash ramp, RadLerpRange).
- `evidence/native-play/script-phase2-loco.{json,log.jsonl}` + `p2-*.png`: run (rate 1.025), Ctrl-walk (`00c2`),
  turn relay (state 1 for one frame), double-tap dash (rate 1.58 -> 1.79 -> 1.3, layer 0.43 -> 1.0), punch and
  return, Kreely dash and walk.
- Not yet diffed against the oracle: the same script still has to be fed to the recompiled core.
