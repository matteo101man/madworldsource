# Original HUD continuation - 2026-11-09

mw-play now renders the original player/boss health bars with live values,
front/trailing progression and boss hold/fade. Debug state/control text is
opt-in with MW_DEBUG_HUD=1; temporary gameplay status text remains. This is
not a complete HUD. Kreely playtesting remains excluded.

## Correct controller identities

CodeWarrior vtable[0] points to its own RTTI record; RTTI[0] points to the
class name. The nearby name at vtable-8 belongs to the preceding class and
is not a safe identity lookup. The capture reader now resolves the pointer
chain with bounded guest reads and exposes typeName plus typed HP fields.

| Type | Vtable | Native update | Role |
|---|---|---|---|
| cIdPlCockpit |805B0E14|8022F034|Owns HP and15 other HUD consumers; allocates child controllers |
| cIdSubMission |805B0E44|8022DF64|Six stage mission icons, not player HP/lives |
| cIdBossHp |805B14D4|802242D4|Boss HP, common program31 |
| cIdGageCluster |805B1504|80224158|Player HP admission/normalization, owns cIdHpGage |
| cIdHpGage |805B1534|80223D28|Player bar, common programs39 and low-HP effects58 |

The initial investigation's stage-clear classification was wrong. Those
observed types are cIdMission and cIdWeaponWindow. The original no-PC-sequence
captures stop startup confirmations on event21; residual startup demo/dialog
scopes remain after the event. They cannot establish fight HUD visibility.
Native80086DF8 tests the SIGN of80604020, not generic nonzero. Negative values
mean demo mode; demo0 and game-flags word0 are the selected live samples.

## Source value and animation paths

Player cockpit8022F25C submits max/current to the cIdGageCluster child.
80224290 is the equivalent cluster setter;802242AC/8022F270 reset damage
history and stop the child. Cluster fields14=max,18=current,1C=previous,
20=submitted.80224158 normalizes current/max (zero remains zero), marks the
underlying HpGage active, indicates fresh damage, and flags ratios<0.3.

Player HpGage creates program39 and secondary effect program58. Widgets10
and11 are front/trailing gauges;12 supplies the healing color preset. Damage
updates front immediately. The trailing bar waits until the byte counter
exceeds30, then approaches the target. Further damage while waiting does
not reset that counter. Healing approaches the front value and snaps the
trailing bar when done. The counter resets on equality. Basic damage/root flashes and the source
position/scale/alpha opening are connected. Low-HP effects, menu sounds,
complete admission and world/death transitions still need their consumers.

Boss work-manager8024A760 is a tail call to802247F0. It saves previous current
at28, writes max20/current24, and marks submitted.802242D4 normalizes into18,
creates program31, and shares the front/trailing arithmetic (committed1C,
delay34).80224644 reacts to raw HP changes with a100-update color/alpha hold,
then binds source widget2 to root1's RGB/alpha fade. The normal Rust presenter translates this hold/fade; complete native
control/event admission and boss teardown remain to verify.
The program has an alternative hidden compact layout as well. The initial
root alpha opens0->1 over5 updates; freezing it at0 incorrectly hides the
bar. Player damage RGB similarly runs to its white endpoint rather than
remaining frozen at its initial red value.

## Gauge translation

800C5120 initializes the horizontal increment as signed numerator(+42) /
denominator(+44) / original width(+E4). Source31 has320-unit bars, source39
100-unit bars. Front uses100/20; trail100/30. No guessed duration or pixel
speed is needed. UiWidget::gauge_step validates this supported data subset.

800C5508 performs one increment/decrement and then snaps when the remaining
distance is <= speed, including a small overshoot. mw-sim::ui_hud::Gauge
matches400 isolated calls to the original helper bit-for-bit. The explicit
MW_TEST_HUD_GAUGE fixture saves private scratch words below the current stack,
initializes only those words, calls the original helper, records output and
restores scratch/context. It never changes actors or real HUD widgets. It is
default-off and proves arithmetic only. Only build-record DLL was rebuilt.

HealthMeter translates front/trailing progression and applies ratios to the
shared UiScene. play_hud.rs connects programs39/31 to the live fight after
entry controls release, hides/freezes them during pause and hides the boss
bar on victory. Complete per-update controller timing/admission is not yet
verified against native; current component comparisons are stated below.
UiScene now binds immutable source envelopes, so healing template12 followed
by restoring template10 does not clone the already-rebound target. RGB/alpha
bindings preserve source defaults/inheritance flags. Per-channel stop masks
allow native hold states without freezing unrelated animations.

## Captures and comparisons

- hud-entry-v1(65s),hud-fresh-v1(70s),hud-normal-v1(90s): terminal exit0,
  original presentations and bounded reads. Their selected snapshots remain
  in demo mode and are not used as normal fight HUD evidence.
- hud-gauge-v1(25s): terminal exit0;400 original scratch calls, captured identity
  metadata, pointer-resolved types. Staged by tools/ui/stage_hud_evidence.py.
- hud-live-v1: ordinary existing PC-input sequence clears remaining dialogs.
  At55s/80s demo0/game-flags0; player HP ratios0.6/0.35, boss1600/1600 then
  1450/1600. Native UI lists reach their bounded endpoints. No actor/health
  state writes or desktop keyboard automation were used.
- IMPORTANT: hud-live-v1's overall test exits101 at its final harness assertion:
  observed phase sustained-combat, expected released. The stable test binary's
  110-second control sequence did not have enough wall time after startup. The
  read-only55s/80s component snapshots are preserved with hashes and narrower
  scope. Repeated as hud-live-v2 with190s total: exit0, PC sequence starts and
  reaches released,5867 presentations/host blit. This is native input/hardware
  acceptance, not a pixel/gameplay equivalence claim.
- Common source31 binds21 observed records; source39 binds8/9 (index8 absent).
  Unique shared bases and majority exact header hashes establish identity.
  Remaining header differences are only kind8's secondary UV bounds at74,
  78,80,84.800C5224 writes the current cropped TUV bounds into these fields
  each draw; the earlier PC-atlas relocation interpretation was wrong. They
  are recorded and directly compared to the Rust atlas crop.
-17 native-visible nondegenerate quads match source position, dimensions and
  RGBA at the selected clocks. Gauges compare against prepared vertex bounds,
  since native size remains the full width while prepared vertices crop toHP.
  Raw quad UVs are0..1, while the clipper writes atlas bounds into the source
  resource; all4 bounds agree with the Rust presenter crop. Generic secondary
  UV effects/sampling/compositing still require more proof. Boss bar is faded
  at the80s sample; pixel parity is not established.
- Restoring main RGB after the healing binding matches native settled gold.
  Full simulation suite83 tests, data9 tests and game bins check pass.

Raw decompiled C remains external in TEMP/mw-havok-proxy. Rust translations
and evidence contain no pasted decompiled function bodies.

## Required next work

1. Verify complete per-update player/boss controller traces across damage,
   healing, low-health, pause, death and admission; preserve native holds.
2. Finish remaining healing-completion sound, lifecycle and pixel/material
   parity; the low-health variant/program58 clocks are integrated below.
3. Port the remaining cockpit consumers: clocks, score/lives/status, weapons,
   navigation/clash/finisher prompts and feedback. Current world score/lives
   and many broader source systems remain absent; do not invent values.
4. Continue UI/title/save/world transitions, menu SE and intro subtitles from
   NATIVE-PORT-GAPS.md. This continuation does not shrink the full goal.


## Ordinary versus secondary-list widgets

8024B81C routes source flag80000 to manager+1C, while ordinary widgets are
inserted into the primary list.8024A99C updates/batches the primary list.
Program39 index8(flag19100E) is on the secondary list and absent from both
ordinary-list snapshots. Drawing it as an ordinary sprite put a red panel over
the bar. UiScene's ordinary draw visibility now excludes80000, while retaining
the source record for bindings. Other secondary-list consumers remain unported;
this is not evidence that no native consumer can ever use those records.

## Playable integration checks

- Inspected evidence/rust-hud-full.png and rust-hud-idle-damage.png: original
  source bar shapes/textures, gold fill and live player80% crop are visible.
- Full/direct and idle-to550 render smokes exit0; observed original player
  damage100->90 at359,80 at434,70 at518 updates front immediately and retains
  the previous trailing value for the source delay. No HP fixture used.
- Intro skip301, cleanup/fade and HUD admission at fight tick16: exit0. Pause
  hides/freezes bars, tick50/HP100/position/Virtual hold, then resumes to98.
  HUD signature assertions include front/trail, boss timer and source clock.
- Boss bot-damage run reaches source hold/fade; terminal renderer check and
  current source capture are recorded in evidence/rust-hud-boss-fade.log.
- Runtime entry, clock, low-health, sound and whole-screen parity remain
  incomplete as listed above. Passing component tests is not full HUD proof.


## Low-health controller and runtime verification

`MW_TEST_HUD_LOW_RECORD` is default-off and allocates a private original
cIdGageCluster through802233A8, then submits max/current values through
80224290. No actor health/state is written. Recording waits for its own
common39/common58 resources before starting340 updates. Source identity is
validated by tools/ui/stage_hud_low.py; staged rows and hashes are in
ui-hud-low-observed.json. Native hud-low-v3 completes the ordinary PC sequence
through released, runnerError null,6657 presentations; all bounded readers
exit0. Earlier v1/v2 captures predated UI admission and are not used.

80224158's threshold is strictly ratio<0.3.80223D28 binds common39's root1
to auxiliary record590292/ordinal0 on entry and restores original584308 on
exit. Damage while low restarts the current binding rather than restoring
normal RGB. The RGB first key is at1 and end at15, but the clock resets0;
using the first key as loop origin incorrectly produced a14-update cycle.

LowHealthEffects translates common58 root1 alpha opening/reversal and damage
restarts. In this English resource the border/noise leaves have4000 disabled
flags/zero native geometry and the red quad stays alpha0. They remain invisible
throughout all340 observations, including repeated damage,30%,29.9%, recovery
and re-entry. No hypothetical visible noise/flash was added. The replay checks
all visible player-panel geometry/RGBA, gauges, delays, committed HP, low state
and the invisible effect controller's alpha. All93 library tests pass.

play_hud.rs now integrates these controllers with actual player HP. Natural
combat smoke reaches30% at854 (normal),20% at1040 (low),5% at1133 and death
at1279. Ordinary Continue key pulses recover HP at1440 and restore normal
appearance; no actor HP fixtures. Renderer screenshots rust-hud-low-flash.png
and rust-hud-low-recovered.png were inspected. Pause signatures include every
HP/effect position/RGB/alpha clock and low-state/alpha-reversal state.

Remaining: native healing-completion SE10000004, complete admission/teardown
and whole-screen rendering parity, remaining cockpit counters/prompts/status
and their actual world systems. This closes the verified low-panel behavior,
not all original HUD consumers.

Low-health pause smoke also exits0: optional MW_PAUSE_SMOKE_WAIT_TICKS=1100
lets combat reach low HP naturally before the normal pause script. Fight
tick1135/HP5/position/Virtual and every HP/effect clock remain frozen, the HUD
is hidden, and resume reaches1183. No input/health manipulation is used beyond
the existing menu smoke actions. See evidence/rust-hud-low-pause.log.
