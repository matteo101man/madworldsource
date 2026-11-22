# Fight replay follow-up - 2026-11-09

Scope: repair HP regression and continue discrepancies; no Kreely playtest.

## Follow-up: protection time and AI target position

Rechecked the current HP repair:2533 canonical rows retain one HP mismatch
(Rust487/native488), while the1756-row independent recording has no reported
mismatches. Frame398 gate Z error remains0.0000304894. Acceptance limits are
unchanged; this continuation does not claim to clear the final canonical hit.

Two further native behaviors are now translated in fight.rs:

- 80276DA4..80276E1C computes player +1150 as GameVSyncF times
  cScene::getSpeedRate (802163C8), then subtracts it from positive +115C
  protection time. The alternate actor branch repeats that subtraction at
  802786C8. Rust previously subtracted an unconditional2; protection now uses
  twice the current animation/scene rate, including the intro's rate1 override.
  This preserves the native slow-time protection duration without adding an
  arbitrary update of protection at the get-up boundary.
- Rin's br_wait810CB5C4, br_run810CB818 and chkAtkStart810CBEB8 obtain distance
  through getTargetVec810658C8, which reads the published target transform.
  They now use the same predicted world position as the corrected fan launch,
  instead of Jack's earlier actor-update position. The actor movement commit
  remains once per update.

New tests cover protection under slow time and the intro override, plus both
AI branches with actor and published positions on opposite sides of an attack
range.88 simulation and14 data tests pass; all game binaries check. Both
fight regression recordings pass at their existing limits. Logs:
evidence/fight-timer-library-tests.log, fight-timer-game-check.log and
fight-ai-world-regression.log.

The rebuilt mw-play also exits0 after an original intro skip at300, source
cleanup and15-update room fade, followed by700 Jack key-bot fight ticks.
Runtime assertions cover actor restoration, blocked entry controls and HUD
admission. Log:evidence/fight-timer-runtime.log. This is a bounded smoke test;
the simulation's full-fight autopilot victory test also passes.

The third fight-target-v1 recording still first diverges in Rin's AI at1138
and has200 HP mismatch rows (FRAME-COMPARE-fight-ai-world-third.json). Its
initial attack cooldowns are absent from the recording and retain Fight::new
defaults; that is a known replay limitation, not a proven cause of this
divergence. Neither correction improved its category counts. Raw native
decompilation remains external in TEMP/mw-havok-proxy.

## Latest: fan target world-transform ordering

The HP repair remains in place. Fixed a further cause of fan travel/catch drift:
810CDEA8 obtains the player target through 810658C8 -> 8020FDEC, which reads
the cached world matrix translation, then adds one unit in Y. Rust was using
the earlier actor-update position, before its pending root translation. The
throw now sees that translated target; actor position is still committed by
the existing movement path. A stopped proxy contributes no translation.

Added bounded, default-off target/hand-matrix/player-world fields to
MW_FRAME_CONTACTS. The owned fight-target-v1 capture runs 120.11 seconds,
5911 presentations, exit 0, with 1982 fight rows. All 1981 consecutive world
translations equal the next actor-update position exactly, including 120
moving samples. FAN-TARGET-WORLD-COMPARE.json records this phase evidence;
tools/pl0000/fan_target_compare.py reproduces it. Only the alternate
build-record core DLL was rebuilt.

Final comparison (collision and recorded per-frame RNG reseeding):

| Recording | Before this correction | After |
|---|---|---|
| Canonical, 2533 rows | 1 HP frame; Rin state 2, motion 3, motion-frame 27, position 6 | 1 HP frame; all Rin categories 0 |
| Independent, 1756 rows | 8 HP frames; player position 39, motion-frame 301; Rin position 111 | 1 HP frame; both actors' position/motion-frame and all Rin categories 0 |

Canonical player counts remain HP 1, state/motion 2 each, motion-frame 13,
position 8. The early hit remains 487 versus native 488. Independent now only
differs at 1418: lethal hit arrives at 1419 instead. Frame-398 gate Z error
remains 0.00003049. Reports FRAME-COMPARE-fight-target-world{,-contacts}.json
and FIGHT-REGRESSION.json contain the complete category counts.

tools/pl0000/fight_regression.py fails on incomplete recordings, worsening
HP/category counts, or the frame-398 gate stop. It now guards the improved
one-HP-frame limit in each recording. A new simulation test covers a throw
during pending knockback translation. All 74 simulation tests, both Rust bin
checks and the mw-play build pass. The hidden Jack autopilot smoke exits 0
after 300 ticks, with HP 100 / Rin HP 1560. No Kreely playtest.

Limits: the target translation uses the pending Rust movement, before final
wall/body resolution; complete Havok ordering/filter parity remains open.
The additional capture is a transform-phase check, not a third passing fight
replay. Its independently seeded replay first diverges in Rin state at 1138
and later has 200 HP mismatch frames; FRAME-COMPARE-fight-target-world-fresh.json
preserves that result. Hidden state/RNG draw ordering and full contact poses
still prevent a general frame-exact claim.

```powershell
python -I tools/pl0000/fight_regression.py
python -I tools/pl0000/fan_target_compare.py evidence/frame-record/fight-target-v1.jsonl research/FAN-TARGET-WORLD-COMPARE.json
```

The sections below retain earlier investigation results.

## Follow-up: retain the gate contact

Fixed the independent replay's first position discrepancy at frame 641. The
gate approximation forgot an existing plane as soon as the actor was about
0.001 outside its keep distance and treated the next movement as a new cast,
allowing roughly 0.1 extra travel. It now stores bounded gate contact state
per actor. Existing contacts permit the positive plane gap or recover penetration;
moving outside the query shell or gate bounds releases them. New unmatched
casts preserve their separate margin.

Native evidence: cinfo constructor 800B28F0 sets +0x28 to 0.1 from 80535E60;
80444F64 copies it to proxy +0x5C. Integrator 80446914 forms the query tolerance
from proxy +0x58 + +0x5C (keep distance plus contact tolerance). Manifold update
804489B4 matches existing planes before adding the nearest new query contacts
and cast hit. Constraint builder 80447FA0 subtracts keep distance, preserving
positive gaps and recovering negative gaps by depth * timestep. The Rust state
models this gate distinction only, not the complete manifold matcher/simplex.

Both mw-play and the headless replay now apply movement through Fight::move_world,
so retained contacts cannot be omitted from the playable path. A test covers
new cast entry, the small-gap retained plane and release while moving away.

Final reports: `FRAME-COMPARE-fight-gate-retained.json` and
`FRAME-COMPARE-fight-gate-retained-contacts.json`. Canonical counts are unchanged
(including 1 HP frame and 8 player-position frames). Independent player-position
mismatches drop **70 -> 39**, first mismatch **641 -> 1044**; Rin position drops
112 -> 111. Its 8 HP frames and other timing counts remain. The main frame-398
stop and late catch/idle boundary remain at their prior accuracy. 70 simulation
tests and the pure Rust mw-play check pass (`evidence/fight-retained-*.log`).

## Follow-up: native contact-pose blending

Reconfirmed the HP repair on the current tree: 1 mismatch frame in the 2533-row
canonical comparison, versus the reported 132-frame regression; 8 in the
independent 1756-row comparison. The early canonical contact remains at 487
versus native 488. This is not a zero-mismatch result.

Translated the normal part-transform path of 80079ED0 into
`character::filter_native_pose`. Contact geometry now keeps each motion's
previous evaluated parts and filters toward its current sample with weight
`1/(1+remainingBlend)`. Missing translation/rotation tracks retain the part's
values; missing scales tend to one. Motion starts retain parts and reset the
synthetic root, matching 80078CD0. This affects simulation contact/hand frames;
presentation's separate smoothstep crossfade is unchanged. Part suppression/
force flags, additive paths, animation layers and full quaternion/Euler
round-trip parity are still outside this helper.

Added `tools/pl0000/contact_pose_compare.py` and the matching Rust example.
They use native positions/facings instead of replayed movement, separating
pose error from controller drift. In the existing independent recording,
245 damage-bone samples in the first five rows of seven 1090-to-idle transitions
have mean position error 0.02318 -> 0.00538 and maximum 0.08155 -> 0.01702 world
units. This check reconstructs only that known idle blend (5), not every native
transition. Across 10073 samples whose clip is unchanged for 30 rows, sampling
the current frame has mean error 0.00351, versus 0.01472 for frame-minus-step;
there is no support for adding a blanket one-frame pose delay.

Reports: `fight-contact-pose.json`, `FRAME-COMPARE-fight-pose-filter.json` and
`FRAME-COMPARE-fight-pose-filter-contacts.json`; logs `evidence/fight-pose-*.log`.
The pose change leaves every replay mismatch category at its prior count.
69 simulation tests and the pure Rust mw-play check pass.

That comparison exposed the frame-641 gate contact issue addressed above.

## Follow-up: terminal fan contact

Re-ran both recordings against the current UI-inclusive tree. The repaired HP
baseline remains 1 canonical mismatch frame and 8 independent-capture mismatch
frames; the UI work has not reintroduced the 132-frame regression.

Fixed a separate damage-dispatch edge case: `move_fans` returned immediately
when both fans were held, even if the previous returning update had queued an
attack. It now drains that registration before becoming idle. The native
`810CAF54` registers the returning volume through `81005DA4` before changing
the fan to held; held state stops new registrations. The new test covers a
final returning contact with both fans held and verifies one damage event and
no duplicate damage on the next update. This is a synthetic boundary test,
not a newly observed hit in either existing recording.

63 simulation tests pass and `cargo check -p mw-game --bin mw-play` passes.
Reports: `FRAME-COMPARE-fight-terminal-fan.json` and
`FRAME-COMPARE-fight-terminal-fan-contacts.json`. Their category counts match
the previous baseline. Logs are in `evidence/fight-terminal-fan-*.log`.

Further investigation: native `80079ED0` filters bone translations/scales
recursively and interpolates rotations from the existing pose with weight
`1/(1+remainingBlend)`. Fight damage/fan geometry currently samples the target
clip directly; presentation has a separate smoothstep crossfade. This is an
untranslated native behavior, not yet established as the cause of frame 487.
The separate capture also has small position/throw-step differences before
its first HP mismatch at 969; by 968 the Rust fans have entered their return
leg while the native fans are still outbound. Do not fix these boundaries by
extending get-up protection or adding frame-specific delays.

## Current result

Canonical live.jsonl, start 334, collision + per-frame recorded RNG reseeding:
2533 frames. HP mismatch count 132 -> **1** (pre-clock baseline 45).
Player state/motion: 2 frames each; motion-frame: 13; position: 8.
Rin state: 2; motion: 3; motion-frame: 27; position: 6.
Report: FRAME-COMPARE-fight-fixed2.json. Frame 398 z is within 0.00004 of
original -59.585411. Remaining canonical HP difference: hit 487 instead of 488;
totals recover at 488. Final catch has a one-frame shift near 1408.

An intermediate run (fight-proxy-anchor) scored zero canonical HP differences,
but retained an extra frame of get-up protection. Fresh native contact evidence
shows 4 on the get-up exit row, then 2,0. The final implementation keeps the
evidence-backed expiration rather than optimizing a single-record score.

Independent fresh-core fight-contacts.jsonl, start 384: 1756 frames, 8 HP
mismatch frames, first HP mismatch 969 and position mismatch 641. Report:
FRAME-COMPARE-fight-fixed2-contacts.json. This is not snapshot-exact; hidden
state and RNG draw order are not reconstructed. live-v1 from 334 retains
immediate Rin/AI divergence and is not a passing validation (fight-fixed-v1).

## Native evidence translated

- 80279D10: intro does not pause hit timers. getSpeedRate's intro override
  returns 1 and masks the reset comparison, leaving the actual request latched.
  Requested animation and smoothed physics rates are separate.
- 802727B0 lethal hit / 80279CA8: mode 4 requests 0.001 for 70 frames, holding
  for 10. Mode 1 starts at 0.003 and multiplies by 1.25 below 0.3, 1.35 above,
  capped at 1. Rin finishes the attack before noticing death in her wait branch.
  Jack's 0.35 death motion rate matches the recomp's default PC presentation
  hook, not a Wii constant. Its cinematic camera is not yet ported.
- 80078CD0 clears root caches; 80079ED0 filters sampled root with weight
  1/(1+remainingBlend), then subtracts the previous filtered root. 80079388
  advances blend by the scaled motion step times requested rate. Root filtering
  continues on slow hit frames even when motion advance is almost zero.
- Only synthetic MOT origin supplies actor XZ travel. Hip XZ stays in the rig
  and damage spheres. Removing it displaced native contact geometry.
- 810CAF54: hand-local Hermite paths toward a world target transformed into
  the current hand frame. Both legs keep the same tangents. Current world/attack
  position uses previous local position and current hand matrix, before local
  coordinates are overwritten. Catch accepts either fan.
- 802C0D4C refreshes get-up protection to 6. Flinch/get-up defer dizzy clip entry.
- 80447FA0: recover penetration by depth * timestep, speed 1. Original cube3292
  side plane in col_obj.hkx explains frame-752 lateral stop. Capsule support uses
  radius 0.4, keep 0.15 and native shape radius. Full filter/simplex parity is open.

MW_FRAME_CONTACTS adds bounded read-only timers, player part worlds and fan
phase/local/world positions; no guest calls/writes. Only alternate build-record
DLL rebuilt, not the production core. No Kreely visual playtest.

## Verification and remaining work

39 simulation tests pass, including source-recorded gate-side recovery,
exponential death-rate recovery and fractional motion-end boundaries.
Final mw-play cargo check passes, including the locomotion first-frame correction.

Open: 487 contact, catch/idle boundary, independent capture drift, native contact
filtering and hidden-state reconstruction. No full-fidelity claim.

```powershell
$env:MW_REPLAY_COLLISION='1'
$env:MW_REPLAY_RESEED='1'
python -I tools/pl0000/frame_compare.py evidence/frame-record/live.jsonl --start 334 --label fight-fixed2
python -I tools/pl0000/frame_compare.py evidence/frame-record/fight-contacts.jsonl --start 384 --label fight-fixed2-contacts
cargo test -p mw-sim --lib
cargo check -p mw-game --bin mw-play
```


## Fan hand publication phase investigation

Extended frame_replay --contacts now emits handMatrix, local position and
world target alongside the existing fan positions. The native hand matrices
already exist in fight-target-v1, a1982-row owned recording. Start335 is its
WAIT/phase0 boundary; start334 incorrectly seeds the preceding selector state
and is not the aligned first-throw audit. This extra recording still develops
hidden-state drift and is not added as a passing gameplay regression.

Reproduce the bounded baseline phase audit:

```powershell
$env:MW_REPLAY_CONTACTS='1'
$env:MW_REPLAY_COLLISION='1'
$env:MW_REPLAY_RESEED='1'
python -I tools/pl0000/frame_compare.py evidence/frame-record/fight-target-v1.jsonl --start 335 --label fight-hand-baseline-audit
python -I tools/pl0000/audit_fan_hand_phase.py
```

FAN-HAND-PHASE-AUDIT.json records68 samples over frames372..405. Same-frame
Rust hand rotation error reaches0.08732038 in Frobenius norm; the preceding
Rust pose stays within0.00000435. Native current handMatrix times previous
local reproduces native fan world within0.00000388. Translations are excluded
from the cross-implementation rotation metric, so actor/root drift cannot
masquerade as a sampling-phase result. Original80079210 evaluates80079ED0
before80079388 increments the motion clock, agreeing with the observations.

A pre-advance pose sample experiment (including the pre-advance blend weight)
matched selected first-throw hand matrices/local/world positions to about
0.00002. It restored the independent lethal hit at1418 but moved its previous
hit to969 instead of970: HP count stayed1, while motion/state/position categories
exceeded the existing gate. Source fan world at968 is just outside Jack's
damage sphere (minimum distance margin+0.033 against preceding body geometry);
the experimental Rust fan differs by up to0.043 in X and0.022 in Z there, with
throw-progress error0.00208. The incomplete runtime change was reverted;
no acceptance limits were loosened. Both retained regressions pass at1 HP
frame each and frame398 gateZ error0.0000304894.

Next: reconcile launch distance/target and actor world/root drift, then apply
the evidence-backed pose phase together with those fixes. Avoid arbitrary
contact delays, radius changes or renewed get-up invulnerability. Logs:
evidence/fight-pre-advance-pose-regression.log (rejected experiment),
evidence/hud-low-fight-regression.log (retained passing runtime). Raw native
decompilation stays external in TEMP; no function bodies were copied to Rust.


## Retained published-transform correction

The missing companion change was the newly evaluated player root translation
in the published world transform, in addition to the lagged translation that
lands in the actor snapshot. Before correction, the predicted throw target
equalled the current replay actor row; the original target is the next actor
row. Source946 launches during get-up: native player actorZ=-58.9029617,
next actorZ=-58.8652306. The previous Rust targetZ=-58.9029579, so it omitted
the published root delta. Post-collision prediction uses a copy of retained
proxy contacts and leaves the actual movement commit unchanged.

Together with enemy pre-advance pose/blend sampling this clears every reported
category in the1756-frame independent recording: HP1->0, motion/state1->0.
The existing regression now permits zero independent mismatches. Canonical
2533-frame replay remains at1 HP frame and prior category counts; frame398
gateZ error is still0.0000304894. No arbitrary delay, extra protection or
contact-radius adjustment was introduced. Jack retains his separately observed
pose publication path; applying the enemy phase to him worsened idle body
geometry by about0.0114 and was reverted.

Original first-throw hand rotation error now stays within0.00000435; ordinary
fan world error over frames372..405 peaks at0.000027014. The42-check source
fixture in fan-transform-observed.json covers those34 rows plus eight launches
through1071, driven by737 original RNG words and idle inputs. Native demo
state=-2 is preserved for its separate hit-clock behavior. No later actor, HP
or fan states are injected.84 simulation+9 data tests pass; game bins check.
Details, scopes and remaining third-record AI divergence: FAN-PUBLISHED-TRANSFORM.md.
