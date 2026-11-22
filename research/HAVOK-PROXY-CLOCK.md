# Character proxy clock: opening knockdown

2026-11-09. Scope: the Rust fight's frame-398 player-position mismatch. Kreely
playtesting and announcer systems were excluded by the user.

## Follow-up status (HP regression repaired)

The first pass below is historical. The current implementation includes the fight timing,
fan matrix, root filter and collision corrections in [FIGHT-REPLAY-FOLLOWUP.md](FIGHT-REPLAY-FOLLOWUP.md).
The final canonical replay has **1 HP mismatch frame**, down from 132 (and below the
pre-clock baseline of 45). A separate contact capture has 8. Neither is fully exact.

## Finding

The stop is a physics-clock effect, not an additional airborne gate margin.
Jack's rendered pelvis rises, but his actor origin stays at floor height and his
Havok capsule stays vertical, height 1.6 / radius 0.4. The previous gate geometry
and its ordinary 0.55 clearance remain applicable.

- **Native:** `802150F0` smooths scene field `+0x1C` toward `+0x18`, multiplying
  by 0.5 while slowing and by 2 while recovering. `80215160` passes that smoothed
  value to `800AB7D0`, which multiplies it by GameVpsInv and stores the physics
  timestep and its inverse at `80609A04` / `80609A08`.
- **Native:** `80279CA8` is `cPl0000::setHitStop`. Common damage requests use
  `80569FE8 = 0.0003`; the fallback duration is `6 >> 1 = 3` frames.
  `80279D10` processes the request. Animation timing can remain at one frame
  while the smoothed physics rate falls.
- **Native:** simplex solver `80466E20` advances its output position only when
  the next time interval is greater than `8058B0F0 = 0.0001` seconds. This is
  separate from the outer proxy integrator's smaller remaining-time epsilon.
- **Observed:** the optional proxy recorder captured 96 frames with a timestep
  at/below that cutoff, floor contacts only, and zero resolved velocity in all
  96. Clock samples include `0.00008` with zero displacement, followed during
  recovery by `0.00016` with nonzero displacement. See
  `havok-proxy-clock-evidence.json`.
- **Observed:** `proxy-seeded.jsonl` reproduces the distinctive first-1011 stop
  at motion frame 9, z -59.585831, without any wall/gate contact in the manifold.
  `havok-proxy-request.log` separately shows nonzero requested root steps after
  the proxy has stopped. Floor friction coefficients in the simplex trace are
  zero; a larger collision margin or a root-speed dead zone is not the cause.
- **Inferred implementation scope:** the opening event leaves the hit request
  latched until ordinary play resumes. `FightInput::demo_active` conveys that
  context to the Rust clock. This models this encounter's observed overlap;
  it is not a complete translation of all scene priorities, hitstop modes,
  or special death/finisher time effects.

DOL: owned RZZE8P revision 0, SHA-256
`8d7f3fe172468b334b6a4aee5cc8bc1479cfd137274100b072a4bfcfe76c5a06`.
Bounded decompilation was read from a scratch Ghidra project; generated C++
request/simplex probes were temporary and their source changes were restored.

## Rust change and verification

`mw_sim::collision::ProxyClock` preserves the smoothed physics clock separately
from the animation clocks. `Fight::step` consumes the preceding proxy result,
consistent with the existing delayed root step, and returns zero player movement
below the solver cutoff. This applies to the presenter and headless simulation.
The standalone presenter starts after the opening event; its demo flag is false.
`frame_replay` supplies the recording's actual demo nesting.

With collision + recorded per-frame random reseeding, starting `live.jsonl` at
334, the first player-position mismatch moves **398 -> 493**. The stopped Rust
z is -59.582352 versus original -59.585411, inside the existing 0.05 tolerance.
The first overall mismatch is now **Rin position at 411**. Player state/HP still
first differ at **485**. Reports: `FRAME-COMPARE-havok-before.json` and
`FRAME-COMPARE-havok-clock.json`.

This is a local fidelity fix, not a claim that the whole replay improves in every
metric: across 2533 compared frames, HP mismatches increase 45 -> 132 and player
state mismatches 462 -> 605; Rin state mismatches decrease 87 -> 50. Later
simulation paths still diverge. The v1 replay also retains earlier Rin/input
differences and is not an independent frame-exact confirmation of the opening.

All 36 mw-sim tests pass. The new regression checks the stop's position,
continued animation, clock recovery, and the same clip's ordinary-play travel.
`cargo check -p mw-game --bin mw-play` passes.

```powershell
$env:MW_REPLAY_COLLISION='1'
$env:MW_REPLAY_RESEED='1'
python -I tools/pl0000/frame_compare.py evidence/frame-record/live.jsonl --start 334 --label havok-clock
python tools/pl0000/proxy_report.py evidence/frame-record/proxy-clock.jsonl --out research/havok-proxy-clock-evidence.json
cargo test -p mw-sim --lib
```

## Native diagnostic capture

`MW_FRAME_PROXY=1` adds bounded post-update shape, contact, resolved-velocity,
physics-timestep, and raw player flag observations to `MW_FRAME_RECORD` rows.
It makes no guest calls and writes no guest memory. `MW_INPUT_REPLAY` injects
the recorded pad stream as before. Optional `MW_FRAME_REPLAY_RESEED=1` additionally
writes the recorded previous-frame cRand word; this is an explicit controlled
fixture, not a natural or snapshot-exact replay. New headers identify it with
`randomStateFixture`; the earlier captures named above predate that header field.
AI state initialized before recording is still not reconstructed by reseeding.

GPU runs passed startup, headless presentation, host blit, and shutdown. They
provide frame/contact evidence, not a visual playtest or full Havok equivalence.
Only the alternate `port/build-record` core was rebuilt; `port/build` was left
alone. Old snapshots tied to the previous alternate DLL identity are not
compatible with its rebuilt diagnostic DLL.
