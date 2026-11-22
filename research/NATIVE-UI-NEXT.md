Latest event-message checkpoint: EV21 has no original MESS track; the earlier
dialogue-subtitle gap for that selected event was incorrect. Playable entry
now checks its source-hashed empty message manifest. Generic MESS admission/
clear/option handling and original font presentation are implemented and
verified using EV00's4 Man Darts intro messages,209 native glyph observations.
The EV00 inspector is not full cutscene/world integration.96 library tests pass.
See EVENT-MESSAGES-API.md. Remaining: configured subtitle-window overrides,
other message tokens/controllers, menu SE and full save/world/title flow.

Current fight checkpoint: independent1756-frame recording now has zero
reported mismatches (gate tightened to zero); canonical retains one HP timing
frame. Rin pre-advance pose phase and Jack's fully published root/collision
target are integrated together.42 source checkpoints and93 library tests pass.
See FAN-PUBLISHED-TRANSFORM.md. Earlier pose-only rejection below is historical.

Latest checkpoint: common39 low-HP variant and common58 controller clocks
are integrated and verified against340 private original UI-controller updates.
Threshold strictly<30%, RGB loop15 updates, further damage retains the variant,
recovery restores normal source. English program58's geometry/alpha remain
invisible as observed. Actual damage/death/Continue renderer runs pass; see
HUD-API.md and ui-hud-low-observed.json.93 library tests pass.

Fight replay gate remains1 HP timing frame per owned recording and the
frame398 gate stop error0.00003049. Extended hand-matrix/local/target replay
diagnostics expose a pose-phase lead; the incomplete pre-advance correction
was reverted after the independent regression failed. Do not loosen limits.
Next: settle fan transform/target drift with source evidence, remaining cockpit
consumers/prompts and original UI/save/world flow. Kreely playtest excluded.

# Latest checkpoint: original health bars in normal play

2026-11-09: mw-play now renders original player HP(program39) and boss HP
(program31), with live fight values, immediate damage/front and delayed trail,
healing, source opening animations and boss100-update hold/fade. Debug state/
control text is opt-in with MW_DEBUG_HUD=1; temporary gameplay status/continue
hints remain. This is partial HUD integration, not full HUD completion.

Native gauge800C5508 matches400 isolated scratch calls bit-for-bit.17 live
visible quads at player60%/35%, boss1600/1450 match prepared geometry/RGBA and
all cropped primary atlas bounds.800C5224 writes those UV bounds into the
source each draw. The UV fields are not evidence of PC-atlas relocation.

Source flag80000 routes a record to the manager's secondary list in8024B81C.
The ordinary8024A99C draw pass does not consume it. Excluding those records
from the shared ordinary presenter removes an extra red player-bar overlay;
this is supported by the source and both native snapshots (index8 absent).
Reference records remain available for channel binding. Generic secondary-list
consumers remain separate work. All prior native UI comparisons still pass.

The190s native ordinary-PC run exits0: sequence starts/reaches released and
5867 presentations/host blit. The earlier110s run was too short for a110s
control sequence after startup; its final assertion failed, with valid bounded
component snapshots preserved separately. Rust full/idle-damage captures and
intro-skip-to-HUD/pause smoke exit0; inspected original full and80% bars. HUD
waits for controls, hides/freezes under pause and resumes with the fight.
83 simulation/9 data tests and bin check/build pass. See HUD-API.md and
research/ui-hud-observed.json; runtime logs/captures: evidence/rust-hud-*.

Next: remaining cockpit counters/weapon/status/clash/finisher prompts, full
low-health program58 effects/sounds and exact admission/teardown; native
per-update timing and pixel/material checks. Keep full UI/pause/intro/challenge/
save/world scope intact in NATIVE-PORT-GAPS.md. Kreely playtest stays excluded.

# Previous checkpoint: HUD gauge foundations and live evidence

2026-11-09: traced actual cIdPlCockpit/GageCluster/HpGage and BossHp paths.
Native player HP uses program39; boss31. Translated normalized gauge rates,
800C5508 approach/snap and shared front/trailing progression.400 isolated
original helper calls match Rust bit-for-bit.17 native-visible prepared
quads match source geometry/RGBA at HP0.6/0.35 and boss1450/1600. Source
bindings now restore immutable envelopes and support stopped RGB/alpha clocks.

The native type lookup is corrected to vtable[0]->RTTI[0]->name; adjacent
names misidentify the prior class. The reader now records typed HP fields
and raw gauge increments/current values. HUD-API.md records the corrected
classes and native source/visibility/hold/fade boundaries.

Normal mw-play still displays debug HUD. Kind8's material/secondary UVs,
controller admission/timing, low-HP effects and remaining cockpit consumers
must be ported before full HUD integration. The original PC input capture
reached live demo0 with valid HUD snapshots, but its final harness assertion
failed (sustained-combat vs released). Only the stated component snapshots
are used as evidence; it is not an overall native acceptance pass. Other
bounded native runs and gauge fixture exit0.83 sim/9 data tests and bins pass.

Next: kind8 rendering and full controller traces, then normal HUD integration.
Keep the full UI/pause/intro/challenge/save/world scope in NATIVE-PORT-GAPS.md.

# Latest checkpoint: original boss challenge confirmation

2026-11-09: program42 and source stage message12010008 are translated and
available with MW_CHALLENGE=1 before the intro. Yes proceeds through the
existing intro/room handoff; No closes and exits the opt-in inspection.
Actual roaming-world return is unported, so default startup stays at intro.
This is the original boss-entry confirmation, not a general challenge list.

Native accept/decline captures finish exit0. Source timing, selected choice,
result, visible quad geometry/RGBA/UV and all48 centered original glyphs are
verified against both traces. Shared UI fixes separate animated offsets from
parent scale, propagate child envelopes correctly, and bind scale/RGB source
channels. Existing UI geometry tests stay green:80 simulation/9 data tests.
Rust bins check/build and runtime prompt/accept/decline smokes pass. Original
font/atlas pixels are retained; inspected screenshot still shows known TV
noise/compositing/render differences. Menu SE and world/ranking return remain.

See CHALLENGE-API.md, CHALLENGE-EVIDENCE.json, ui-challenge-observed.json.
The fight regression remains repaired:1 HP mismatch each in2533/1756-frame
captures, frame398 Z error3.04894e-5; all category ceilings pass. Next work:
original HUD, prompt/menu audio and real world/scene routing, subtitles and
remaining event/render/SE consumers. Kreely visual playtest stays excluded.

# Latest checkpoint: default intro and boss-entry handoff

2026-11-09: mw-play now starts with the original intro by default. Backspace
or right mouse invokes the native B/Menu Back skip. After end/skip it holds
black for cleanup and the source13-update wait, waits for resources, releases
the combat actor/AI during the15-update room fade, then releases player input.
Pause/debug HUD are gated during entry, and the input-release gate prevents
the skip press from becoming a fight action. Inspection bypass is explicit:
MW_DIRECT_FIGHT=1 or MW_INTRO_PREVIEW=0; preview1/fixed-frame inspection remain.

Source correction:80147BBC waits for80123820(event21) before creating combat
Rin. Existing references to combat overlapping the OBJ cutscene were too broad.
AI overlaps the post-event room fade instead. The diagnostic native boot also
has independent startup tasks; their residual demo nesting is not a boss-entry
delay to hardcode. EVENT-API.md records the original functions and remaining
boot/object-message/actor-state boundaries.

Native bounded read-only lifecycle captures and original WPAD B fixtures
verify normal ending1079, early-skip ending301 and late-skip ending821.
All room fade alpha endpoints agree with the new simulation model. Normal/
late skip retains BS04C; early skip starts original BS04. The latter's93100-
sample intro and native gain metadata are now staged. Source resource latency
remains asynchronous/readiness-based. EV21-ROOM-COMPARE.json audits all three
native captures and music admission. Only build-record DLL was rebuilt.

Verification:78 simulation tests and Rust bin check/build pass. Full default
startup and early/late-skip smokes exit0, including source media/UI cleanup,
mesh/prop restoration, tick0 before fight release and AI advancing during
fade before controls. Post-skip pause smoke exits0: tick51 stays frozen,
HP/position/Virtual/audio hold, then resumes to99. Inspected fade5, clear15
and original pause-map captures. Logs: evidence/rust-intro-room-*.

REQUIRED NEXT: original HUD and challenge-menu flow, dialogue subtitles,
remaining effect/render consumers and SE DSP fidelity; actual main-menu/save
integration and world/victory/scene transitions. Full native boot/boss-switch
prerequisites, bl015d/bl001c_N002 object messages, exact actor/physics/RNG state
and render/fixed-clock phase parity remain open. NATIVE-PORT-GAPS.md retains
the full scope. Goal remains active; Kreely playtesting stays excluded.

---

# Previous checkpoint: original THUD writing effect

2026-11-09: ported command16's fixed2D WCD111 effect at510. It is THUD,
not a dialogue subtitle. Recovered802E293C's subscr/wc.idd and core2.dat/
core2.wcd bindings; staged the exact100-byte record and161472-byte IDT bank
with source hashes/ranges. Rust preserves original shared group61 glyphs,
slot spacing,200/40 placement,0.399999917 scale,-20-degree rotation and
RGBA242/241/212/240. Program100/widget3 supplies its bounce while binding
context3 retains both static destination axes. The native30-update timeout
removes it before drawing age29, at event539.

Corrected an earlier sampler classification: position flag0x20000 invokes
the separate random-offset helper800CAF3C, not a different time parameter.
WC0x20063 has no0x4000/0x8000 random axis, so its ordinary frame-keyed base
curve is now evaluated. Random-axis state remains explicitly unsupported.
This change also implements native context3's preservation of both axes.

Evidence: default-off MW_EVENT_WRITING bounded reads, no guest writes/calls/
RNG draws. Capture completes120.10s/5912 presentations/exit0 with1080 event
rows. All319 allocated-widget visibility/lifetime checks and80 visible glyph
UV/pose samples pass; position/size/rotation errors0, color error2.98e-8.
Allocation is observed before511..539, with all owner widgets invisible.
EV21-WRITING-COMPARE.json and the native-fixture simulation test enforce
these bounds. Only the alternate recording core DLL was rebuilt.

Verification:9 data and77 simulation tests, both Rust bin checks and playable
build pass. Full intro smoke exits0, verifying WCD111's4-glyph peak/removal
alongside all previous lettering/audio/cut/restoration checks. Rendered
impact/bounce and post-removal stills512/539 were inspected; source glyph
placement/tint/rotation appear and clear. See evidence/rust-intro-writing-*.

REQUIRED NEXT: original room-script fight overlap, normal startup, skip/end
and real handoff; dialogue subtitles and other effect/render consumers;
first draw-time phase; SE effect sends/DSP release/resampling. Intro remains
explicit MW_INTRO_PREVIEW=1 with frozen fight. This is one fixed2D record,
not a general WCD manager or its shared RNG side-effect implementation
(native init still draws random numbers even when ranges are zero).
Full UI/HUD/challenge/save/PC-settings and broader fight parity remain open
in NATIVE-PORT-GAPS.md. Complete goal stays active; Kreely playtest excluded.

---

# Previous checkpoint: original intro comic-lettering playback

2026-11-09: ported EV21 ID command5's three original comic programs into the
Rust intro, using the staged animation-only IDD bank and shared core_us atlas.
Source cues260/350/392 create programs1/2/3, bank4/layer10. The event clock
drives their transforms, rotation and alpha once per event frame. Atlas pages
are ready before playback; the event fade remains above the glyphs. Source
program100 is invisible owner scaffolding and0 is never requested.

Native evidence: MW_EVENT_UI's1080-row capture completes120s,5457 presentations,
exit0. All14229 requested-widget UV/visibility samples and400 visible glyph
poses agree; max position error0.000031 logical pixels, size0.0000039,
color6e-8 and rotation0. Clock edgeN observes the preceding updateN-1.
EV21-UI-COMPARE.json preserves this phase and first draw/pixel limits.

Verification:76 simulation tests, Rust bin checks/build pass. Full Rust intro
smoke exits0: programs execute at260/350/392, peak6/5/5 glyphs, clear after
source alpha fades; all4 cuts complete with fight tick0/HP100,80 mesh nodes
restore,2 props hide, source streams/effects and retained boss loop succeed.
Inspected rendered stills275/355/400: the original WHOOSH/WOOSH glyphs are
placed and rotated over the scene. Logs/captures are evidence/rust-intro-ui-*.

REQUIRED NEXT: dialogue subtitles and command16 writing-character111 (separate
from comic lettering), other effect/render consumers, first draw-time phase,
SE effect sends/DSP release/resampling, and the original room-script overlap,
skip/end and normal startup. Intro still uses explicit MW_INTRO_PREVIEW=1 and
freezes the fight. Full UI/HUD/challenge/save/PC-settings and broader fight
parity remain open in NATIVE-PORT-GAPS.md. The complete goal remains active;
Kreely playtesting stays excluded.

---

# Previous checkpoint: fight HP guard and fan world-target correction

2026-11-09: returned to the requested fight discrepancies; Kreely excluded.
The repaired canonical HP baseline remains 1 frame, versus the old132-frame
regression. Corrected throws to use the player's translated world target
(810658C8/8020FDEC), supported by1981 consecutive native matrix samples.
Canonical Rin catch/idle timing and position mismatches clear; independent
capture HP mismatches8 ->1 and position/motion-frame mismatches clear.
74 simulation tests, Rust bin checks/build and300-tick Jack smoke pass.
tools/pl0000/fight_regression.py guards both improved baselines and frame398.
See FIGHT-REPLAY-FOLLOWUP.md for complete counts and the additional capture's
AI/hidden-state divergence; this is not general frame-exact fight parity.

Remaining fight work: canonical487/488 contact, independent1418/1419 lethal
contact, proxy/filter/body-resolution ordering and hidden state/RNG draws.
At this checkpoint, intro ID1/2/3 WHOOSH/WOOSH source programs were staged and
compared against native widget state, but not integrated into mw-play. The
latest checkpoint above supersedes that integration limit. Dialogue subtitles
and writing-character111 are separate unported consumers. The full UI/intro/
challenge goal remains active; the previous checkpoint below retains scope.

---

# Previous checkpoint: intro effects and corrected shared sound lookup

2026-11-09: concrete progress; full goal remains active. Kreely playtest excluded.

- Found and fixed a shared SE staging bug:804F835C reads the sound-definition
  table at+0x40, not+0x44. The old parser shifted sound IDs by one and dropped
  final entries. Programs use0x40+4*count+(offset&0x7FFFFFFF);804F6DBC handles
  the variable wave-bank list/track lengths. The corrected stager preserves
  those layouts, and the owned EV21 bank regression checks IDs0,1,6 plus all7
  entries. Restaged1143 bindings in66 banks:867 existing mappings corrected,
  254 added,57 old invalid/shifted mappings removed.304 referenced WAVs validate
  as complete16-bit mono/stereo PCM. Reports SE-DEFINITION-TABLE-FIX.json and
  SE-STAGED-WAV-VALIDATION.json document scope; the general interpreter is partial.
- Recovered SE13 consumer80189530: EV21 flags5 select numeric, non-spatial
  playback through80268DBC. Its actor-name fields do not attach these sounds.
  DOL8026548C resolves event21 to se/event/scene_mix7522.rsp, which is now staged.
  The original cue IDs7522001/7522006 run at180/866, using waves1/6, volume127,
  note60 and sequencer waits1500/350.0x12 is note/key-on(804FF2AC), not pan;
  0x60 sets effect sends(80500B0C), not key-on. Prior labels are corrected.
- Added default-off MW_EVENT_SE read-only SDK diagnostics. The authoritative
  ev21-native-se-v2 capture completes120s/5783 presentations/exit0 with1080 rows.
  Source program bytes, wave/block, note and sequencer step match all576 active
  voice rows. Default dry left/right mixer words are both23169(/32768). Native
  804FDA28 phase +=416/-=120 per96-sample32kHz callback gives mean0.0104s/tick;
  remaining counter times agree within one event frame. EV21-SE-COMPARE.json
  records the limits. The first probe's+0x1C8 field was relabeled opcodeIndex;
  v2 correctly reads waveIndex at+0x19C. Only v2 supports the wave comparison.
- Rust intro now preloads/plays both verified numeric2D effects on its shared
  event clock, with crossed-cue seek and source hold duration. It uses the
  observed default dry stereo gain, without the fight's spatial approximation.
  Full rust-intro-se-smoke exits0: both source effect sinks, all3 music/voice
  sinks,4 cuts at tick0/HP100/START, restored80 meshes/2 props, retained boss
  music and loop handoff. General fight audio smoke runs300 ticks/exit0 and
  consumes the corrected mappings.73 simulation tests and both Rust bin checks pass.

REQUIRED NEXT: native effect-send/reverb and DSP release/resampling fidelity
  (these two effects currently play the verified dry path), event ID/title/
  subtitles and other effect/render consumers; creation-frame attachment draw;
  original room-script fight overlap, skip/end and normal startup. The intro is
  still opt-in, with fight frozen. Full UI/pause/main-menu/HUD/challenge/save/
  PC-controls/render/fight requirements remain in NATIVE-PORT-GAPS.md.

---

# Previous checkpoint: original intro fades and stream audio

2026-11-09: concrete progress; full goal remains active. Kreely playtest excluded.

- Camera SEQ1 now drives the Rust full-screen fade. 800A7930 selects the key;
  800A73A4 interpolates normalized float RGBA and truncates on conversion back
  to bytes. All1080 native recorder values match, including alpha169 at frame5.
  Inspected real Rust captures at0(black),7(partial opening),15(clear) and1072
  (partial closing). This verifies application, not native pixel/gamma parity.
- tools/events/stage_audio.py resolves DOL BGM/VCE names/IDs and stages original
  EV21 music plus English evvo21 voice, retaining source/WAV hashes and sample
  metadata. Existing BS04C intro/loop WAVs are reused and validated. Native
  resource commands start EV21_RSD/61 and JKSNN_EVVOSTR_EV21/0x11021 at0, then
  BS04C_RSD/30 at748. compare_audio.py checks the1080-row manager capture;
  EV21-AUDIO-COMPARE.json explicitly distinguishes manager state from audible onset.
- Translated stream integer mix from80501E64/805018A8/80502320. Mode2 uses
  music master100 and ignores the declared event command volume60. RedSpark
  header volume70/60/127 gives base gain0.4376526(EV21),0.37600708(BS04C),
  0.9999695(voice), before Rust PC master/bus controls. Normal boss BGM now uses
  staged native mix metadata instead of its old inferred0.6; old unstaged
  manifests retain that fallback. Cutscene Sound volume now controls event voice.
- Inspection preloads all stream assets before starting its frame0 clock;
  source commands use that common time origin, compensate a crossed cue with
  a source seek, and preserve bit4 persistent music (80073D70) at the end.
  Full runtime smoke exits0, plays all3 source streams/sinks, completes4 cuts
  with tick0/HP100/START held and restores80 meshes/2 props. At handoff the
  retained boss sink was11.075s versus11.083s event elapsed; its loop starts
  at17.459s from cue, without restarting the intro. No PCM/device parity claim.
- 72 simulation and2 audio settings tests pass. Rendered fade captures and the
  full audio handoff/loop smoke pass. Pause smoke timing is now a deterministic
  60Hz fixture, matching its frame-indexed input packets, while production
  clocks still use real time; opening-gated save-slot packets are scheduled
  after their actual30-frame gate rather than depending on renderer speed.
  Final title/save-slot and Sound freeze/save/resume runs both exit0;
  evidence/event-audio-pause-{title-slots,sound}-final.log. Both pure Rust
  bins check in event-audio-final-check.log. The failed earlier fixture runs
  remain evidence of the timing flaw; they were not gameplay parity failures.

NEXT: event SE13(7522001/7522006), ID/title/subtitles and remaining effect/model
  render-state consumers; creation-frame draw placement; original room-script
  fight overlap, end/skip and real startup lifecycle. MW_INTRO_PREVIEW remains
  opt-in and freezes the fight. Normal mw-play still starts directly in combat.
  The complete UI/pause/main-menu/HUD/challenge/save/PC-controls/rendering and
  fight gaps remain required in NATIVE-PORT-GAPS.md; this is not a completion claim.

---

# Previous checkpoint: original intro props, attachments and mesh cancellation

2026-11-09: concrete progress; full goal remains active. Kreely playtest excluded.

- Recovered8011E120 actor commands14(attach/detach),15(substring mesh
  cancellation through800994F8/802EB90C), and command10's Jack goggles mode
  (80099558/802072E4 exact names). Attach IDs are native bone IDs, not array
  indices: pl0040->pl0030_00/bone14 and wp0600->pl0030_00/bone8.
- Exported original pl0040 and wp0600 via the existing source exporter with
  actual GLTFLoader reimport checks: attributes/weights unchanged, matrix error
 0 and<4e-7 respectively. tools/events/stage_props.py stages both GLBs and
  binds44 original mesh names/node ordinals across Jack and these two props.
  No original source pixels or meshes were edited to simulate cancellation.
- mw-play intro inspection now loads, binds, poses and attaches both props,
  uses original mesh cancellation rules, and restores original visibility after
  inspection. Parent transforms come from the current full world-space MOT
  pose. Source cut2 hides Jack and both props; no held-pose stand-ins.
- Added optional MW_EVENT_ACTORS to the read-only clock recorder. Owned native
  run ev21-native-actors completed120s/5756presentations/exit0. Original task
  identity, actor visibility, attachment target pointers, mesh cancellation,
  roots and selected bone matrices are recorded. No guest calls/writes.
- cargo example event_compare verifies1080 frames: zero visibility/attachment
  mismatches and zero cancelled-mesh mismatches over98280 comparisons. 3802
  root matrices match within0.000035;10372 selected bone matrices within0.00051.
  IMPORTANT LIMIT: two creation-frame prop roots are still identity in this
  callback because command14 binds after the first parts update. They are
  explicitly excluded ONLY from transform checks; their first draw-time
  placement is not verified. This is not all-bone/pixel/lifecycle equivalence.
  Report research/EV21-ACTORS-COMPARE.json records the exclusion.
- Inspected Rust screenshots rust-intro-props-0.png and rust-intro-props-240.png.
  Full four-cut smoke completes with fight tick0/HP100/player[2.5,17.55,-61]
  held, then verifies restoration of80 mesh nodes and hiding of both prop roots.
 74 library tests pass(8data66sim), native comparison passes, both Rust bins
  check. Original native user-file hashes are unchanged. The temporary owned
  export server was stopped after staging.

NEXT: source audio/BGM/VCE, ID/fade/effect commands and actor render-state
  flags; verify creation-frame draw placement, original room-script overlap,
  end/skip/handoff, then integrate the real intro into normal startup. Current
  MW_INTRO_PREVIEW remains an inspection fixture with fight frozen. Normal
  mw-play still starts directly in the fight. Save/main-menu/B01/HUD/challenges/
  PC controls/render/fight parity gaps remain in NATIVE-PORT-GAPS.md.

---

# Previous checkpoint: verified Rin camera application and Rust track inspection

2026-11-09: concrete progress; full goal remains active. Kreely playtest excluded.

- Recovered native camera application:800838F4 eye,80083988 target,80082D58
  sample roll/FOV and8008229C up-vector construction. Source FOV is radians;
  packet FOV is degrees through805307C8=57.29578. Roll rotates world-up using
  ROT_ORDER2 with the horizontal eye-minus-target yaw, not an arbitrary view-axis
  quaternion. mw-sim::event now translates this and selects actor cuts/local frames.
- Added default-off MW_EVENT_RECORD to the existing original clock wrapper.
  It reads native clock edges, two-field cadence, camera cursor/sample and final
  camera fields, without guest calls/writes. Owned native ev21-native-camera ran
 120s/6053presentations/exit0 with neutral input after event21 was requested.
- tools/events/compare_camera.py verifies the complete1080-row source clock and
 1079 observed applied packets against source frames0..1078. Eye/target error
  <5e-8,up<3.6e-8,FOVdegrees<2.6e-6. The callback observes the preceding camera
  application: row0 is the old scene camera. This does not verify final scene
  handoff or the last source camera sample's presentation. Native cadence stays
 2VI fields/update; vi.cpp uses60000/1001fields/s, so ev21 is30000/1001updates/s.
- Actual mw-play source-track inspection now exists, enabled ONLY with
  MW_INTRO_PREVIEW=1. It plays camera and Jack/Rin body MOTs at native cadence,
  keeps world-space motion roots and hides missing actor cuts. MW_INTRO_FRAME=N
  holds a chosen source frame. Optional screenshot runs use hidden windows.
  This is a validation fixture, not a replacement for the normal intro lifecycle.
- Captured and inspected evidence/rust-intro-0.png and rust-intro-600.png.
  Jack's opening shot and Rin's cut2 body pose/camera render. Native pixel/actor
  bone parity is not yet established. Props/pl0040/wp0600, mesh/actor commands,
  voice/BGM/subtitles/ID/effects/fades and native room-script integration remain.
- Full inspection playthrough traverses all four cuts then resumes the inspection
  fight. Its test verifies tick0/HP100/player[2.5,17.55,-61] throughout. FREEZING
  FIGHT IS INSPECTION-ONLY: original fight and intro overlap; this cannot become
  the production handoff without translating the original lifecycle. Normal
  mw-play startup remains direct fight; no normal intro completion claim.
-73 library tests pass(8data65sim), both Rust binaries check. New regression
  compares all1079 native packets and actor cut presence. Existing title/slots
  pause smoke still passes with tick118/HP100/time3.9926242 held. Data loader now
  rejects invalid FOV and degenerate camera directions before rendering.

NEXT: actor sequence dispatcher8011E120 and attachment/mesh commands14/15,
  original event-object transforms/parts observations, pl0040/wp0600 meshes and
  audio/ID/fade consumers. Then reproduce room-script overlap, end/skip behavior
  and connect the real intro to normal startup. Rust save persistence/menu/B01,
  HUD/challenges/PC/render/fight gaps remain in the inventory.

---

# Previous checkpoint: START reaches native B01; Rin intro tracks staged

2026-11-09: previous turn made concrete progress. Full goal remains active;
Kreely playtest excluded. This checkpoint supersedes the START-wait speculation below.

- New bounded diagnostics traverse107 native controllers to the actual list end,
  identify OptLoadGame by its8023C290 vtable method, and expose its state/record/
  callback fields. START reached state12 with callback8023D560: the wait was
  save completion, not the selection animation. Resource animation-wait flags
  and four record summaries/hashes are now available in UI snapshots.
- The isolated test NAND used Rust's canonical Windows `\\?\` path. Native
  host_path appended Wii slash-separated components without normalizing them.
  `fopen` rejected the resulting mixed extended path. Fixed with make_preferred
  after joining. New native nand_fs test reproduces failure with this prefix
  before the fix and passes create/open/write/seek/read afterward. Only the
  alternate build-record DLL was rebuilt; original native user files are unchanged.
- Native ui-save-fs-fixed-oracle180s/6626presentations/exit0 now writes option.dat
  (32bytes) and save.dat(201024bytes), then enters B01. Capture55 shows its city
  opening cinematic. Empty START does NOT show program84 first in this flow.
  B01 is not the arena/title main menu and must not be replaced with an invented
  Rin-fight destination. The record persisted on disk matches the Rust reset+
  flag3 record exactly, SHA4528f6695021ead5cdaa59d78d4f74766d0f2b9d0faf711871ddb860eecd0894.
- mw-data::save::start_new_game now models that verified record initialization,
  including occupancy/new-game-pending bits. Occupied slots cannot be silently
  overwritten by this operation. Filesystem writes and menu/scene integration
  are still absent in mw-play; START remains a no-op there.
- Staged English Rin intro ev21: source.dat and event.json under
  assets/native/events/ev21. Four cameras/1080frames, four actor identities,
  twelve MOTs, original sequence commands and21 hashed chunk ranges. New
  mw-data::event parser checks bounds/counts/finite samples, actor identity and
  cut/sample consistency. Rin is missing cut0; Jack/pl0040/wp0600 are missing
  cut2. Do not synthesize those clips. Raw unexecuted commands, effect/ID/audio
  resources and source provenance are retained. No intro playback is claimed.
-71 library tests pass(8data63sim), both pure Rust binary checks pass; Python
  staging/read adapters compile. Native prefix regression and original user-file
  hash checks pass. See SAVE-FLOW-NATIVE.md and EVENT-API.md.

NEXT: integrate Rust-owned record persistence and occupied-slot UI using the
  verified format; keep B01's missing scene explicit. For the requested Rin intro,
  translate native camera/actor/attachment/audio command consumers, verify
  camera samples against native observations, then connect playback and skip to
  arena handoff. Full HUD/challenge/PC/render/fight gaps remain in the inventory.

---

# Previous checkpoint: native save records and START transition evidence

2026-11-09: concrete progress; full goal remains active. Kreely playtest excluded.

- Added `mw-data::save`: exact-size four-record parser, read-only occupancy/
  score/stage/time views, byte-preserving record copy and reset. No unknown fields
  are normalized. Invalid length/index operations fail without modifying data.
- `tools/ui/stage_save_defaults.py` derives the empty record from native reset
  8017FD14 and DOL table8054F3F0 (entry setter80189B24/getter80189CBC). Its
  50256bytes match slots2,3,4 of the existing native save exactly, SHA256
  256998695e6c3c8738b32a78e5909620d30f2c6af9095f1ea2b8f22c95dc2301.
  No player save is used as the asset source. These table values are reset-entry
  defaults; the earlier scratch name 'weapon-defaults' is NOT an established meaning.
- CORRECTION: 8017FDC0 copies/clamps playtime. It does not schedule saving.
  Actual full-record/options writes are requested by8023D5E8 through8021248C,
  80213D6C and80213D40. The separately managed32-byte options area80653E20 is
  outside the201024-byte save.dat records; its full persistence remains unported.
- Native runs use a test-only validated MW_GPU_TEST_USER_TAG under
  evidence/native-users, copied from the original NAND. Production ignores it.
  ui-newgame-confirm-v2-oracle completed180s/5694presentations/exit0. Actual
  packet logging proves A48.100..49.008 was delivered. Capture55 still shows
  START and no program84 headers appear in the final snapshot. The old0.2s
  press run also did not reach84. Do not count either as a confirmation capture.
- STATIC CORRECTION: empty START does not necessarily open program84. Native
  8023C290 state4/selection0 checks8017FCB8 first; in the empty-slot branch it
  calls8023D5E8 with callbacks8023D3F4(reset/create) then8023D560(load/B01).
  Program84 is another branch. Need inspect the action confirmation animation
  wait (8024C73C/80226F00), controller state and actual next scene. No modal or
  destination may be invented from its class name. The expected START->84 step
  in the older checkpoint below is superseded by this evidence.
- 69 library tests pass (6data63sim), including independent sparse native reset
  bytes and preservation of unknown fields through parse/copy/reset. The codec
  is a foundation, not an integrated save/load feature: Rust START remains a
  no-op, occupied-slot presentation and filesystem persistence remain unported.

NEXT: expose bounded controller/animation-running state in native diagnostics,
observe why START waits and reach its real successor. Then integrate the codec
with original save setup, safe Rust-owned persistence, occupied rows and menu
transitions. Keep full UI/HUD/ev21/challenge work in NATIVE-PORT-GAPS.md.

---

# Previous checkpoint: Press -> original save-slot selector and Start panel

2026-11-09: previous turn was concrete progress. Goal remains active; Kreely
playtest excluded. Full UI/intro/challenge/menu flow remains incomplete.

IMPORTANT CORRECTION to the previous Title checkpoint: Press does NOT open
TitleMenuEx/program4 directly. In the native no-save state it opens the four-slot
save selector(program80 + four instances81). Confirming a slot opens program83
with START only on an empty slot. Actual main menu was not reached in these runs.
Do not treat fixture names ui-title-menu/ui-title-newgame as proof of those pages.

- Rust Title splash Enter now opens the original save selector after the opening
  gate. Four empty NEW GAME rows, native numbering/selection frame and0.3RGB preset
  dimming. Up/Down CLAMP0..3(nativeD994), not wrap. Initial Back has no action,
  matching original controller and the lack of a Return prompt on this screen.
- Enter a slot opens its original START panel. Back returns to slot selection,
  preserving selected slot. Empty slots hide COPY/DELETE. Corrected visibility
  ordering: showing parent10 after hiding12/13 accidentally re-enabled those rows;
  they are now hidden after the parent is shown, with native visibility regression.
- START ACTIVATION IS STILL A NO-OP. No new-game confirmation, save creation/load,
  copy/delete, occupied-slot rendering, main menu or title music yet. Pure Rust
  currently represents4initial empty rows; do not claim a save/load subsystem or
  use fake progress. Native/recomp save files are not imported or written here.
- UiScene can bind position across screens sharing one IDT. Slot root1 binds to
  master-frame80 anchors5..8, including original position tracks. Each root is
 260/80,140,200,260; slot-number subtree40 follows it. External widgets are cloned
  with same-IDT guard, not hard-coded screen coordinates.
- Added802249B4 RGB preset application from source channel5's4-entry table. Native
  preset0=white,preset2=.3gray. Explicit local RGBA overrides now take precedence
  over color envelopes before parent multiplication. Existing UI tests still pass.
-66 library tests pass(4data62sim); both Rust binaries check. New tests compare
  master frame and ALL4row instances against62+124 native bindings: geometry,
  visibility and RGBA. Action panel comparison22records verifies START only.
  Position scanline phases align native postincrement counters for the comparison.
- Actual Rust smoke: pause/title Yes/Press, slot0Upclamp,Down/Up,actions/Back,
  slot3Downclamp,finalslot2. Tick120/HP100/position[2.5,17.55,-61]/Virtual4.0452191
  held; fight voices absent. Screenshots rust-save-slots.png and
  rust-save-slot-actions.png inspected; evidence/ui-save-slots-smoke.log.
- VISUAL GAP: Rust TV/noise compositing is too bright and dimmed slot numbers/text
  have poor contrast relative to original capture. Widget RGBA matches native,
  so do not fake brighter row presets to hide it. Need native texture/TEV/alpha/
  gamma blend-space investigation. PC glyphs and viewport also still differ.
  Geometry/color-component tests do not prove pixel equivalence or legibility.
- Native fixtures both exited cleanly: ui-title-menu-oracle180s/6388presentations
  captured selector; ui-title-newgame-oracle180s/6883 captured empty START panel.
  research/ui-save-slots-frame-observed.json(62),ui-save-slots-rows-observed.json
 (124across4clusters1/4/5/6),ui-save-actions-observed.json(22).

NEXT: activate START on an empty slot, capture the resulting cIdNewGameChkWindow
(program84), translate its actual choice text/input and save-record lifecycle.
Native fixture title-newgame currently sendsA28(Press),A41(slot) then captures45
(START panel). Add a distinct mode sendingA48(START), capture55, neutral afterward;
existing180s budget should cover it. Do not change old fixture semantics silently.
Native tests currently use native-core-user; isolate user directory BEFORE sending
Yes on a dialog that writes original records. No Yes/new record writes occurred
in the two native fixtures above.

Native consumers:
- cIdOptLoadGame ctor8023C140,update8023C290,initial slot input8023D994,
  action input8023D6A8,rootdim init8023E08C,selection8023DF8C. Header80/layer9,
  rows81/layer8,actions83/layer11. In this title context header40 hidden, header2
  stays visible. Slot root1 binds external position to header5..8, slot IDs41/42
  display one-based number. Empty-row setup8023CFD4 hides15/14/13/11/10/20,
  shows16; occupied rows use record-specific contents not ported.
- Enter slot animates action root10 forward15frames; empty hides12/13, START11
  remains. Highlight5 binds11 position context1; confirmation6 hidden. Back runs
  root10 reverse before restoring selector(native15-frame close not fully modeled).
- START eventually reaches cIdNewGameChkWindow(string80563FC3,vtable805C9F58),
  update8023E254,input8023E688,init/selectE8B0,acceptE998. Program84/layer16.
  Mode0 shows widgets21/23; mode1 shows22/24. Choice index0 default; actual texts
  must be captured, don't assume they are Yes/No or mode1's difficulty labels.
  Confirm updates slot flags and calls original save manager; Back sends parent
 0xFFFFFF01 and closes (unlike Title confirmation's Back-selectNo behavior).
- Native initial record pool80622CE0,4slots,strideC450,slot index80697F48;
  8017FCA0(...,0) tests occupancy;8017FD14 resets;80127970 copies records;
  8017FC6C changes flags;8017FDC0 updates playtime (corrected above).
- Actual TitleMenuEx/program4 remains after save-selection handling; its enabled
  list/scene transitions and source factory mapping still need confirmation.

Read-only scratch: ui-save-slots.c,ui-save-slot-input.c,ui-newgame-confirm.c,
ui-selection-opacity.c. UI main-menu research: ui-title-menu-ex.c,
ui-title-enabled-entries.c(DAF0 is a splash confirmation handler, not entry-list setup).
Do not repeat the unsupported5E==TitleMenuEx assumption from old checkpoint.
After this: native UI compositing/PC prompts, HUD, intro ev21, challenge flow and
outstanding fight parity. See NATIVE-PORT-GAPS.md for complete inventory.

---

# Latest checkpoint: Title confirmation and original splash integrated

2026-11-09: previous turn was concrete progress. Goal remains active; Kreely
playtest excluded. Full UI/intro/challenge flow is NOT complete. Older checkpoints
below describe previously unported actions; this one is authoritative.

- Options Title now opens original dialog85. DefaultNo; LeftYes/RightNo; Back
  selectsNo (does not dismiss); Enter confirms after26source opening frames.
  Native RGB confirmation track13 lasts4frames repeated5times (20); No closes
  the15frame root scale then returns Options, preserving Title selection.
  Yes reaches original E00-style title splash, not process exit or fight resume.
- Title splash currently STOPS AT THE PRESS PROMPT. Press Start/main menu/save-slot
  flows and title BGM are not ported yet. Do not claim full Title/menu functionality.
  Fight remains paused behind black/title presentation, cannot be resumed by Back.
  Fight voices are despawned, SFX/announcer queues cleared and BGM stopped on Yes.
- New owned English title source: caseE/idpac_ce00_us.dat embedded IDD at12608.
  Stager now8sources/18original PNG pages/1067TUVsprites/157screens/5364widgets.
  Original/inverted provenance kept. Resource idpac_ce00_us:25screens/967widgets.
  Program0 is original logo/copyright; program2 Press controls; program4 TitleMenuEx.
  Native title-logo63/63 and Press27/27 source headers match E00 snapshot.
- Investigated core2_us.dat core.idd at5801440 but it is NOT this Title source;
  its generic parent cycle fails UI loader and is not staged. The original cycle
  guard stays strict. Do not confuse the E00 ROOM resource1 with core2_us.
- Fixed general UiScene scale inheritance: flags4(X)/2(Y) gate BOTH local envelope
  and inherited scale. Title Press label has X disabled and stays56units wide,
  while its A glyph inherits1.2scale. Prior code incorrectly made label67.2wide.
  Existing Map/Sound/Options/Controls/Video/PlayLog observation tests all pass.
- Title dialog border is6source corner pieces, not a solid rectangle. Its original
  positions/sizes match Rust once scanline phase is aligned (widget7 position
  counter18 postincrements ->evaluate17). Do not replace this border with invented
  UI. Native channel bind0x10 only marks an extension flag; it does not map one of
  the8 standard source channel pointers in800CEB9C.
-63 library tests pass(4data59sim). Both pure Rust binaries check. Actual return
  smoke: tick116/HP100/position[2.5,17.55,-61]/Virtual3.9115692 held; original
  splash reached, all fight sinks removed. Screenshots rust-title-confirm.png /
  rust-title-splash.png inspected. Cancel smoke also opens/no/confirms/returns/resumes: tick119/Virtual4.0290456
  held, resumed190; evidence/ui-title-cancel-smoke.log.
- Original title-return oracle180s/9480presentations/exit0: actual Yes returns
  native stage201 ->E00, screenshot ui-title-return-oracle.png. Stable defaultNo
  title-dialog oracle120s/6392presentations/exit0,32/32records bound in
  research/ui-title-dialog-observed.json. Owned processes ended cleanly.

NEXT: Press Start -> TitleMenuEx and real NewGame/Continue/Options/save flow.
Native splash cIdTitleMenu8024D6EC loads resource1/program2/layer12; PressA bit10
starts its RGB fade, then creates type5E(cIdTitleMenuEx). This loads resource1/
program4/layer9 via8024DD68. ctorDD54, activate8024E518, input8024E720. Program4
contains133widgets. Its selected0or3 requests E01 orE02 after fade (do not guess
those scene meanings until consumers/save setup are traced). Options selection4
enters shared Options controller. Menus use enabled entry list on+7D/count+83,
selected+7C, runtime state+78; source binding table805648E0(18widget/slot pairs),
selection tables80564970/805649A0. Need next original capture after secondA at
E00 to bind settled TitleMenuEx widgets and verify enabled list/save-state behavior.

RTTI recovered: OptTitle string8056400C ->vtable805CA124; ctor80239E68,
input80239F08,update8023A214; selectA018, confirmA100. Source dialog85/layer10,
outline9 binds position to11Yes/12No context2. Update waits root1/11/12 before input.
Native Yes does5color pulses then requests E00 through original cleanup/fade.
TitleMenuEx string805649CE ->vtable805CAAF4; cIdTitleMenu string805649DD ->
vtable805CAB50. cIdTitleBG8024EC80 loadsprogram7/layer7 (not main logo).
Read-only scratch: ui-title-flow.c,ui-title-dialog-menu.c,ui-title-menu-ex.c,
ui-title-art.c,ui-channel-bind.c. No raw original C copied into Rust.

After Title: PC controls/settings/glyph parity, original HUD, intro ev21 and c201
challenge flow. Play Log statistics remain initial zeros; fight parity still has
canonical1HPframe/fresh8 and catch/idle discrepancies. See NATIVE-PORT-GAPS.md.

---

# Latest checkpoint: Play Log text and ten-page navigation integrated

2026-11-09: previous turn made concrete progress. Goal remains active; Kreely
playtest excluded. Original Map/Options/Controls/Sound/Video/PlayLog now open/back
in mw-play. Options Title is still a no-op; full original UI/intro/challenges remain
incomplete. Checkpoints below are historical where they list PlayLog as unported.

- Map PlayLog + Enter opens source-selected Play Log, hiding map/arrow and showing
  source page/Return prompts, counter and original label/value glyphs. Left/Right
  wraps10pages. Back restores map and removes selected-entry inversion/fill.
  All46 statistic values currently remain initial zeros: world/scoring/statistic
  producers are NOT ported. Do not report a functioning live statistics system.
- Native text path translated:80184490 glyph placement,80185BF8 command subset,
  80186DAC widget metrics,801877A8 vertical centering. Catalog validates native
  glyph bounds/page paths, exact message termination, supported commands/operands.
  Core/PlayLog labels use original pixels, not system-font replacements.
- Important font correction: numbers use MTD chunk at128 in
  etc/idpac_core_us.dat, payload160. Header length12512 excludes32-byte chunk
  header. This has10digit glyphs,24-unit nominal advance and2 native atlas pages.
  The core.mtd inside core2_us.dat is a DIFFERENT112glyph font; do not use it here.
  PlayLog subscr100_us.mtd adds27glyphs/1page. Catalogv2:37glyphs,3pages,92messages;
  sources/entry offsets/hashes preserved by tools/ui/stage_playlog.py.
- Row bindings in Map:91/92,96/97,106/107,111/112,116/117. Header+40 dimensions
  [18,18,16] drive X/Y glyph scale and line height. Root positions are native
  logical anchors110/370 and Y100/130/160/190/220. Scale80% yields glyphfactor.6.
  Vertical centering offsets line half-height; native first glyph center114.8,94.2.
  Native advance is half glyph width BEFORE draw,2+.3width AFTER, space half nominal
  width. Dynamic decimal insertion caps999999 and left-shifts half nominal advance
  per extra digit. Full command system/typewriter/multiline not translated.
- read_native_core.py --ui-widgets now also reads separate256-slot message pool
  widgets80653F58+i*318/resources80685758+i*E4. Original IDT manager does NOT
  include these glyphs. Bounded read-only own-process gates preserved.
- Original ui-playlog-text-oracle passed120s/4700presentations/exit0, captured75
  first-page glyphs. New regression compares ALL75 glyph centers/sizes AND UV
  bounds against native pool; another checks all92messages and six-digit cap.
  research/ui-playlog-glyphs-observed.json retains condensed original observations.
-60 library tests pass(4data56sim). Both Rust binaries check. Live hidden-window
  PlayLog smoke opens, visits2->1->10, captures first/last pages, returns to Map
  and resumes. Tick116/HP100/position[2.5,17.55,-61]/Virtual3.9235477 held,
  resumed257. evidence/ui-playlog-smoke.log and rust-playlog-page.png/
  rust-playlog-last-page.png inspected. Native Map visibility/positions/sizes,
  page counters and selected-entry inversion also have observation regression.
- Remaining visual differences: PC glyphs still Wii, viewport/noise/TEV/compositing
  and source transition fidelity. Only initial controller state/static rows verified,
  not general multiline/event message rendering, GPU pixel parity or live stats.

Next: Options Title confirmation plus real title/menu return flow (do not map it
onto process exit). Then PC prompt/input/config parity, original HUD, intro ev21,
challenge selection/setup/results and outstanding fight parity. Statistics need
native producer mapping and event tracking; current zeros are deliberately flagged.
See PLAYLOG-NATIVE-RUNTIME.md for controller details and current implementation.

---

# Latest checkpoint: original Video calibration page integrated

2026-11-09: previous turn was concrete progress; goal remains active. Kreely
playtest excluded. Sound checkpoint below is historical where it lists Video
as a no-op. Options Video now opens original full-screen program76 and Back
returns Video -> Options (selection2 retained) -> Map -> gameplay. Native initial
mode8023BD18 accepts Return and does not expose adjustment selection; source
calibration frame, central skull and original text are rendered. Rust uses one
instance because the native two instances have identical visible static quads.
No resolution/vsync options were invented. Separate VI-origin adjustment submode
is not integrated; PC bindings/prompt artwork still need sharing with recomp.

- UiScene initialize_video hides native confirmation/highlight5/6. Full-screen
  Video suppresses map/Options/background presentation while retaining their
  paused navigation state. Original B/Return artwork remains, PC glyphs pending.
- New regression compares visible geometry/visibility against first60 exact
  native source bindings in research/ui-video-observed.json; source4000 excludes
  unprepared records whose native matrices are zero. New navigation regression
  verifies Video return selection and keeps arrows/confirm inert in initial mode.
-55 library tests pass (3data52sim). Actual hidden-window smoke passed entry,
  freeze, Back and resume: tick216/HP100/position[2.5,17.55,-61]/Virtual7.2664393
  held, resumed364. evidence/rust-video-page.png inspected against original
  ui-video-oracle.png. Source geometry parity is verified, pixel parity is not;
  native viewport and PC prompts still differ. evidence/ui-video-smoke.log and
  ui-video-library-tests.log preserve verification.
- Original Video oracle passed120s/6093presentations/exit0 with host blit/capture.
- Remaining Map PlayLog and Options Title confirmations are still no-ops. Full
  original HUD, intro, challenge, lives/scoring and PC settings/input remain.

Current next task: Play Log. Original fixture ui-playlog-oracle passed120s/4563presentations/exit0;
process ended cleanly. Capture inspected;159 Map records bound in
research/ui-playlog-map-observed.json. See PLAYLOG-NATIVE-RUNTIME.md for exact
row anchors, selected-page visibility and next message parser/layout functions. Native ctor8024974C/update80249778,
input80249A30, page renderer80249B74, page counter80249CB8.46 statistic pairs,
5pairs/page =>10pages. Left/Right wraps; Back removes messages before return.
Each message emits native ID0x21000000+index; labels/values alternate, widget
bindings supply ten layout pointers from parent map+6C. Need trace binding setup
and original message glyph/number layout. Staged catalog92 exact messages/27glyphs
is data only; no PlayLog renderer or live stats yet. Scratch ui-playlog-layout.c
has parent helpers:80248A64 hides map100/71/arrow;80248B28(param3=3) hides16/30/35,
restores map and resumes parent page state when PlayLog exits.

---

# Latest checkpoint: Sound settings and native gauge fill integrated

2026-11-09: Goal remains active. Kreely playtesting excluded. The previous
checkpoint below is historical; Sound is now functional, not a no-op.

- mw-play opens original Sound screen70 from Options. Four levels0..10 are MUSIC,
  SFX+GAME VOICE, CUTSCENE VOICE and ANNOUNCERS. Row5 is SUBTITLES OFF/ON;
  row6 DEFAULT resets levels10/subtitlesfalse. Earlier stereo/mono inference was
  incorrect; original capture and native helper8023B5E0 establish subtitles.
- Volume edits preview immediately through Rust audio buses. BGM, SFX and fight
  announcer spawns retain source gains and multiply master*level/10. Existing
  AudioSink/SpatialAudioSink gains update while paused. Cutscene voice level and
  subtitles persist, but playback awaits the unported event/cutscene system.
- Dirty Back opens original save dialog86, title20UV4, defaultNo. LeftYes,
  RightNo, Enter accepts; Back selectsNo. Input waits26 native source frames for
  visible buttons. Apply saves draft; discard restores previous settings and exits
  to Options. No-change Back exits directly. Fight/Virtual/audio stay paused.
- play_audio.rs merges Sound keys into shared pc-controls.toml [audio], preserving
  master_volume, keyboard/video and unknown tables. MW_PC_CONTROLS overrides path;
  MW_MASTER_VOLUME overrides master. Invalid/unreadable files are not overwritten.
  Shared recomp PcConfig flattens SoundSettings so its saves preserve these keys;
  this does not rewrite the original guest's audio settings.
- Correct native kind8 gauge update is800C5224 (not UV-scroll800C560C). It crops
  geometry AND UV around source anchor; +40 axis1X/2Y/0both. Pause HP fills from
  right, Sound gauges from left. New0.7HP vertex regression matches original
  preparedVertices. Shared renderer now honors UV crop fractions.
- Verification:53 library tests (3data50sim),2 Rust audio/persistence tests and1
  native-config roundtrip test pass. Native frontend cargo check passes. Actual
  Sound smoke opened, changed music10->9, captured Sound/dialog, saved and resumed;
  tick47/HP100/position/Virtual time held. Paused BGM sink gain0.37800002 equals
  source0.6*master0.7*music0.9. Save preserved keyboard/video/master fixture fields.
- Evidence: evidence/rust-sound-page.png, rust-sound-save-dialog.png,
  ui-sound-smoke.log, ui-sound-persistence-tests.log,
  ui-sound-native-config-tests.log, ui-sound-tests.log.
  Original Sound oracle120s/4922presentations/exit0; ui-sound-oracle.png/json,
  research/ui-sound-observed.json and ui-sound-map-observed.json.
- Options Video/Title and Map PlayLog remain no-ops. Controls still Wii artwork.
  Full UI, PC prompts/bindings, live score/lives, intro, challenges remain missing.
  General TEV, clipping, scan/noise/aspect and some widget/transition parity remain.

Next: finish Video page76 research and capture, then implement its real behavior.
Native update8023B8A0/input8023BD18/highlight8023C0A4. Scratch
%TEMP%/mw-havok-proxy/ui-video-control.c is read-only original analysis.
It includes screen-position adjustment in4-unit increments via80263E1C/E4C;
this is not evidence for invented resolution/vsync settings. Native page76 loads
layers8/11; actual capture is needed to identify initial visibility/selection.
Then Title/controller flow and PlayLog message/stat rendering, PC control parity,
original HUD, intro ev21 and c201 challenge flow.

Video research addendum (same turn): original capture evidence/ui-video-oracle.png
shows the full-screen CHECK YOUR TV SCREEN SETTINGS calibration frame, central
skull and Return. The native observed state has two program76 instances in
clusters17/18 (120 exact header matches,60 unique records each), not an ordinary
resolution menu. research/ui-video-observed.json preserves both. Initial primary
controls5/6/10 and settings groups11/12/13 are hidden; full-screen bounds/text and
Return are active. Input8023BD18 in initial mode accepts Back; the adjustment
submode changes native VI origins via80263E1C/E4C: X0..80, Y0..160, step4.
Do not expose an adjustment UI before tracing its entry/selection and translating
its effect. Video oracle passed120s/6093presentations/exit0 with host blit and capture;
owned process exited cleanly. Video entry/back integration is still pending. Lifetime functions
B824 initializes handles, B84C destroys; B778 was Sound gauge helper, not Video
constructor. Scratch ui-video-control.c/ui-video-lifetime.c/ui-video-constructor.c
contain read-only decompilation; despite filename, B778 is not Video construction.

---

# Latest checkpoint: Options and Controls entry/back integrated

Previous turn made concrete progress. Goal remains active; full original UI,
pause, intro and challenge flow are not complete. Kreely playtest excluded.

Latest state (2026-11-09):
- mw-play: Map selection OPTIONS + Enter opens original Options screen61.
  Up/Down wraps Controls/Sound/Video/Title; Enter on Controls opens original
  screen65. Escape/Backspace returns Controls -> Options -> Map -> gameplay.
  Other confirmations are STILL NO-OPS: Map PlayLog, Options Sound/Video/Title.
  Continue implementing those, do not mistake visible menus for completion.
- Pure mw-sim PauseNavigation holds native 2-entry/4-entry selections and back
  routing. Frontend keeps Virtual time/audio paused through all subpages and
  clears held fight inputs on resume. Back wins over confirm in the same packet.
- Original native Options ctor80238DF4/update802391A0/input80239AEC; enabled type
  list[0,1,2,5] -> widget IDs[11,12,13,15]. Highlight ID5 binds position channel2
  with context1 (keep own X, take source static Y). New UiScene axis binding.
- Corrected earlier component identification: top SubscrBase ctor80249274 gets
  0x4E Map, 0x69 Options, 0x51 background; PlayLog is0x50 via802493FC. Slot2 of
  container is Options, slot1 PlayLog. Native RTTI identifies cIdOption and
  cIdSubscrPlayLog. Do not reuse old swapped 0x69/0x51 assumptions.
- Source layers: background program7 layer6, Controls65 layer8, shared map0
  layer9, Options61 layer11. Renderer now preserves that ordering. Initial wrong
  capture hid the shared TV/counters because the background was above them.
- Controls uses alternate frame Map ID6: hide5/show6, restart scale FORWARD,
  endpoint scale1.2 * static0.8=.96, position[240,176]. Regression checks all12
  alternate frame nodes against new original capture. Reverse-scale support is
  also available but not used for Controls entry (wrong earlier assumption).
- Controls keeps just selected Controls label and yellow confirmation fill;
  selected text/icon invert via native800D07E4 setting instance flag200000.
  Native800D0A78 uses RGB complement in TEV. Staging adds RGB-inverted PNG variants
  preserving alpha/source pages; shared renderer uses them for white menu glyphs.
  General colored-vertex TEV complement equivalence remains unimplemented.
- IMPORTANT Controls still shows ORIGINAL WII DIAGRAM, not recomp PC glyphs.
  PC bindings/config sharing is outstanding. Do not call it correct PC help.
  Existing recomp prompts: tools/stage_pc_ui.py + gen_pc_prompt_glyphs.py,
  native_prompts.rs; PcConfig pc_config.rs. Do NOT load the native DLL into Rust.
- Assets: original17 PNG pages remain unchanged;17 derived inverted variants
  recorded in each layout.json as invertedImage/invertedSha256. stage_native_ui.py
  reproduces both. Optional inverted paths validated by mw-data.
- PlayLog sources staged with provenance in assets/native/ui/playlog-us/catalog.json:
  tools/ui/stage_playlog.py,92 MSD messages,27 indexed glyphs,1 original font page.
  Message commands, numeric insertions, layout and real statistics not translated.
  First tokens32883,0,32784,80,...; dynamic integer uses32915(0x8093).
- 50 library tests pass (3data47sim); both binaries check. Actual Options and
  Controls hidden-window smoke runs pass open/page/back/freeze/resume. Latest
  Controls tick47/HP100/position[2.5,17.55,-61]/Virtual1.6046835 held, resumed95.
  Evidence/rust-controls-page.png and rust-options-menu.png inspected. These are
  original-style functional pages; GPU viewport/noise/sampling parity remains.
- Native fixture options120s/6438 presentations; controls120s/5295; both exit0.
  New MW_GPU_TEST_UI_PAGE=map|options|controls|sound|video|playlog extends the
  bounded fixture. Captures at6/8/11 seconds after control-ready, sends actual
  D-pad/A packets. No state injection; only alternate build-record DLL used.
  Native executable madworld_native-25e7df48b3ad44bb.exe rebuilt.
- New observations: research/ui-options-observed.json (38/38 exact records),
  ui-controls-observed.json(107/107),ui-controls-options-observed.json,
  ui-controls-map-observed.json. Unprepared source flag4000 leaves native zero
  matrices despite hidden269=0; regression excludes these from coordinate checks
  and asserts Rust not-visible, rather than misreading hidden269 as draw proof.

Next concrete work:
1. Sound page70: cIdOptSound update8023A9AC/input8023B11C; initialize helper binds
   in scratch ui-option-pages.c/ui-sound-control.c. Native four volume levels0..10,
   row selections, subtitles flag and reset (corrected by Sound capture); integration must affect actual Rust
   BGM, SFX, announcer audio and persist settings (PcConfig only has master now).
   Source IDs11,21,31,41 gauges;10,20,30,40 row anchors;5 highlight;33/61 confirm
   animations hidden. Four numeric IDs15,25,35,45. Page resources already staged.
2. Video page76 (NOT75): cIdOptVideo update8023B8A0; native creates two layers8/11.
   Native implementations in ui-option-pages.c, remaining input/helpers decode.
3. Title confirmation/controller and actual title/menu return flow; no fake quit.
4. Play Log page10 rows/pagination already in Map source; cIdSubscrPlayLog
   init8024974C/update80249778/input80249A30/helpers80249B74/80249CB8.
   Loads MessSubscr100_us.msd + subscr100_us.mtd, registers EM; 5 stat pairs/page,
   native count=(messageCount/2)/5+1. Layout/stat IDs via parent Map +6C tables.
5. PC glyph/config parity; original HUD, intro ev21, challenge flow remain.
6. Fight reports still canonical1HP frame/contacts8/live-v1 divergence; no Kreely.

New scratch decomp: ui-page-lookup.c,ui-options-playlog.c,ui-page-bindings.c,
ui-option-pages.c,ui-sound-control.c,ui-options-back.c,ui-pause-page-transforms.c,
ui-selected-color.c. Original C stays in %TEMP%/mw-havok-proxy, not repository.

--- Historical checkpoints; latest above supersedes old menus-only state ---

# Latest checkpoint: original map is integrated in mw-play

Previous goal turn made concrete progress; goal stays active. Full original UI,
pause subpages, intro and challenge flow remain incomplete. Kreely playtest excluded.

Authoritative latest state (2026-11-09):
- mw-play Escape opens the original IDT/TUV pause map, replacing the basic overlay.
  Screenshot evidence/rust-pause-original-map.png inspected: native map frame,
  PLAY LOG/OPTIONS highlight, area-data legend, HP/lives/score, c201 floor map and
  player arrow visible. It is qualitatively close; native GPU viewport/scan timing,
  texture sampling and animation parity are NOT proven pixel-exact.
- Shared renderer crates/mw-game/src/native_ui.rs used by mw-play + mw-ui-preview.
  The major geometry corruption came from ImageNode::Auto intrinsic sizing.
  Native quad dimensions require NodeImageMode::Stretch. This is fixed.
- Source texture-style channel6 has native repeat counts at+0C/+10. 179kind1 and
  2kind2 styles in staged English data. Kind1 tiled sprites are translated with
  partial last-tile cropping; kind2 special distortion is skipped explicitly.
  Scroll rate/step decoded but not yet applied. Non-default blend modes skipped.
- Native 8024738C proves selection source table IDs10/20. Up/Down wraps the two
  entries and rebinds only highlight position. ENTER/CONFIRM DOES NOT OPEN SUBPAGES
  YET. Implement Play Log and Options controller callbacks next; do not call full
  pause complete because the map is visible.
- Native map data is c201.imt inside case2/c201.dat offset5438336,size117920.
  tool tools/ui/stage_pause_map.py verifies archive identity, stages all3 original
  280x280 CI4 pages, and translates IDT idpac_c201_us program50 widget30 metadata.
  assets/native/ui/c201-map/map.json has source hashes/projection provenance.
  8024803C selects floor with strict Y>3.3000011/9.4999981. 80247470 projects
  world X/Z using pixelsPerWorldUnit2,280pixels, source origin[0,20],quad size250.
  Core program18 is the player arrow; subscr000 program2 is the area legend.
- UI counter display still starts lives2/score0: real lives/score/progression is
  NOT ported. HP bar takes actual fight HP. Game pause ownership unchanged.
- Widget channels now have independent UiClocks. Position rebinding resets only
  position, preserving scale/color clocks. UiScene also supports runtime local
  position/rotation for the player map marker.
- New owned oracle evidence/ui-pause-material2-final-state.json has403 widgets;
  all159 map nodes bind with158 exact hashes in research/ui-pause-material-observed.json.
  New read-only fields: texture overrides, UV index/corners, separate channel
  timers. Map ID100 overrides a real texture; ID71 is default white primitive.
  800CB358 increments position counter200 AFTER evaluation; a running timer86
  renders frame85. Stopped endpoint counters remain at their last key. Regression
  accounts for this and now compares ALL48 native-visible nodes, including the
  scanning strips excluded by the earlier single-frame test.
- 47 library tests pass (3data44sim). Checks for both binaries pass. Bounded
  smoke log evidence/ui-map-pause-smoke.log: tick46,HP100,position[2.5,17.55,-61]
  and Virtual time1.5772508 held across pause; resumed tick94. Screenshot above.
- Native oracle fixture100s/4414 presentations passed, shutdown exit0;
  exact counts in evidence/ui-pause-material2.json. No owned process remains.
  tools/run_owned_gpu_test.py --ui-widgets now includes bounded UI state in
  scheduled samples, avoiding manual timing. Reader remains read-only/own-process.

Next work, preserve user scope:
1. Implement native Play Log and Options page entry/exit/confirm and settings.
   Map update80247624 calls workcomponent slot1 PlayLog and slot2 Options via
   80250C58/80250BE0; callbacks80248B28; source initializer8024738C. Need locate
   actual component implementations/assets, not invent UI choices.
2. Persistent UiScene channel start/stop/end and scroll/pivot/clipping semantics,
   live score/lives state and native menu sound/counters. Partial-HP gauge fill
   anchoring/cropping still needs a native comparison (full-HP snapshot only).
3. Original HUD, intro ev21 actor/camera/audio/subtitles + skip, challenge flow.
4. Continue fight discrepancies (canonical1HP frame, contacts8, hidden-state
   live-v1 divergence); existing fight fixes remain verified, no Kreely playtest.

New scratch native decomp (not copied to repo): ui-render.c,ui-submit.c,
ui-material.c,ui-order.c,ui-gpu.c,ui-uv-clock.c,ui-map-load.c,ui-map-resource.c,
ui-map-project.c. All in %TEMP%/mw-havok-proxy, readOnly MotionNative project.

--- Historical checkpoints; latest above supersedes old incomplete rendering ---

# Current continuation checkpoint: pause ownership and UI scene

Goal remains active. Kreely playtesting is excluded. Full original UI, menu,
intro and challenge flow remain unfinished; continue autonomously.

Latest verified changes (2026-11-09):
- mw-data appearance decoding: scalar scale/alpha, RGB four-slot envelopes,
  rotation channels, position/rotation anchor kinds. All 4397 original widgets
  across 132 programs validate appearance as well as hierarchy/position tracks.
- mw-sim UiScene: recursive visibility, source position binding, digit overrides,
  leading-zero numbers, HP ratio, scalar interpolation and inherited color/scale.
  Native map initialization translated from 80247624/80248E9C/80248F78/80249030/
  80249054. Selection source-table values and full page flow remain unresolved.
- IMPORTANT native geometry correction: parent quad dimensions are separate from
  inherited child scale. Source self-hidden groups propagate dimensions; visible
  quads propagate scale envelopes. Axis flags come from source record +34 (4 X,
  2 Y), not the envelope block. Position anchors: 800C970C; rotation pivots:
  800C9640 (rotation pivots decoded but not fully applied yet).
- Regression asserts position, size and RGBA against 45 native-visible captured
  nodes at settled frame20. Excludes asynchronously sampled scanning strips65/66
  and fully transparent fade158. Geometry Y sign is converted to top-down UI.
- 46 combined library tests pass (3 data,43 simulation). Diagnostic viewer now
  exercises UiScene and forward source order, handles mirrored atlas sprites.
  evidence/native-ui-scene.png is STILL WRONG: map render target and native
  compositing/clipping missing; large white/static panels. Do not call it parity.
- mw-play Escape now pauses/resumes instead of exiting. PreUpdate owns pause,
  FixedUpdate fight/cue/announcer chain is gated, Virtual time freezes BGM clock,
  and audio ownership pauses only previously playing sinks, including late loads.
  Resume clears and waits for held keys/buttons to release. HUD query isolated.
  Overlay is deliberately a basic PAUSED / Esc to resume panel. Original map UI
  is NOT integrated while its rendering is wrong; full menu is still required.
- MW_PAUSE_SMOKE=1 is a bounded hidden-window run: opens frame90, resumes180,
  exits300; asserts frozen tick/HP/position/Virtual time and resumed ticks.
  MW_PAUSE_CAPTURE=<absolute.png> captures frame150. See evidence/ui-pause-smoke.log.

Next useful work:
1. Resolve native UI draw compositing / shared TUV primitives / map render target;
   original noise/map frame still opaque white in Rust diagnostic capture.
2. Finish independent per-channel clocks/restart/stop, rotation pivots, native
   special-kind geometry, draw sorting, UV animation and clipping.
3. Translate Play Log and Options callbacks and integrate original pause pages
   into working freeze/audio/input ownership. Confirm source IDs from native
   selection table before claiming navigation parity.
4. Original HUD, intro ev21 and challenge/menu flow remain on the inventory.

Native scratch now also contains ui-control.c, ui-map-init.c, ui-map-fields.c,
ui-channels.c, ui-cluster-stop.c, ui-pivots.c. No generated native C copied here.
Library scene test uses research/ui-pause-program0-observed.json as owned oracle.

--- Earlier evidence / reproduction ---

# Current continuation state - original UI

The previous turn made verified progress; goal remains active. Full UI, pause,
intro and challenge menu in mw-play are NOT complete. Kreely playtest excluded.

Read UI-NATIVE-RUNTIME.md first. Added UiTimeline/UiWidget in mw-data/ui.rs,
position_track decoding and mw-sim/ui.rs native explicit-frame vector interpolation.
132 screens /4397 widgets parse;45 combined tests pass; mw-play and mw-ui-preview
check. Distance-driven flag20000 returns explicit untranslated error.

Critical new evidence:
- Original pause screenshot evidence/ui-pause-oracle.png (from native XFB, not desktop).
  It shows arena MAP + PLAY LOG + OPTIONS + area legend and HP/lives/score panel.
- Read-only snapshot evidence/ui-oracle-paused.json has313 widgets,listReachedEnd=true.
 269 headers match staged sources. research/ui-pause-program0-observed.json binds all
 159 subscr000_us screen0 nodes using158 exact header hashes and a uniform raw base.
- UI logical canvas is640x360 (root320,180) for the PC853x480 XFB.
- Native hierarchy A0 parent,A2 firstchild,A4 NEXT,A6 previous. Traversal follows
 descending serialized siblings. E.g.screen0 root0 child149,next148,... Source
 flags hide only own geometry; native callbacks recursively hide subtrees.
- First static preview was WRONG: omitted shared core TUV groups. Fallback now added.
 Second/third previews still have overlapping/incorrect panels: animation-driven
 visibility, rotations/scales, clipping and special widgets are pending. They are
 DIAGNOSTICS, not a finished pause screen. No UI integration into mw-play yet.
- UiWidget static offsets: +BC RGBA,+CC root-position,+D8 child-position,+E4 size.
 Position channel+C: flags+4,pointCount+8,keyCount+C,vectors+40,index/time keys+160/164.
 800C80C0 discards positionZ;8 source control points have unused NaN Z lanes.
- 80249D48 Hermite,80249E24 half-difference tangents. Generated PPC confirms
 omitted sixth count argument in Ghidra is r8 (key/point count), so it is recovered.

Owned process79652 and preview processes have completed; no task-owned run remains.
Native UI fixture test built and passed100s/3962 presentations/host blit/shutdown.
Default-off MW_GPU_TEST_UI_PAUSE_CAPTURE opens original Minus3s aftercontrol-ready,
then neutral controls,capturesXFB at6s. Requires --pc-sequence. New test executable:
D:/GDecomp/build-cache/bevy-0.20/debug/deps/madworld_native-25e7df48b3ad44bb.exe.
Run tools/run_owned_gpu_test.py --seconds100 --pc-sequence --exe <path> --label...
with MW_CORE_DLL=tools/upstream-native-core/port/build-record/madworld_core.dll
and MW_GPU_TEST_UI_PAUSE_CAPTURE=<absolute output.ppm>. Production DLL untouched.

Native diagnostics: tools/read_native_core.py PID --bevy-host --ui-widgets
--label LABEL --trace-log evidence/RUN.log. Strict owned-process path/mapping
checks unchanged. UI list endpoints manager8069D49C+14/+18; node+8 next;
node+0 widgetptr. Output includes source header hashes/bytes, parent pointers,
visibility, RGBA,size,translation,matrix1c8/matrix1f8. Concurrent,notframe-atomic.
To bind source:
python -I tools/ui/index_native_idt.py --snapshot evidence/ui-oracle-paused.json
--resource subscr000_us --program0 --out research/ui-pause-program0-observed.json
(Actual CLI syntax is --program 0, separated.)

Primary scratch decomp project %TEMP%/mw-havok-proxy/MotionNative (readOnly).
New files: ui-create.c (8024B81C and directory),ui-bind.c(80184A1C),
ui-defaults.c(80184B6C,800C80C0),ui-curves.c,ui-draw.c,ui-widget.c,
ui-subscreen.c(80249274/80249440),ui-pause.c. Generated native C stays in scratch.
80249274 loads subscr/subscr000%s.idd (8056468A) and acquires components4E,69,51.
80240720 initializes cIdPause references55..59; do NOT mistake nearby
80240000/802401C0/80240270/8024040C HUD handlers for the pause master.
Subscr constructor RTTI80564430,descriptor805CA8A8; constructor stub800D6420.
Native UI controller logic still needs tracing to original visibility/init calls;
observed map hidden subtree IDs30,101,35,6,52..58,61,120,16 are evidence,not a
replacement for translating the original controller.

NEXT: implement original widget clock/color/scale/visibility and controller,
then renderer shared with mw-play and validate against original pause capture.
Avoid more static-preview-only work as a substitute for working menus.
Alternative independent concrete work: full ev21 intro actor/camera/audio playback;
existing source events.json/native.py and EventClock are ready, props/attachments
still require staging. All explicit user deliverables remain in scope.

--- Earlier locator notes (historical) ---

# UI continuation checkpoint

Active durable goal remains full UI/pause/intro/challenge parity, not just assets.
No subagents or approvals needed. Kreely playtest excluded. Workspace has no root git repo.

Completed: fight follow-up documented in FIGHT-REPLAY-FOLLOWUP.md (canonical HP 132->1,
fresh contact capture 8), 40 combined tests pass, mw-play check passes. Latest reports
fight-fixed2, fight-fixed2-contacts; earlier zero-HP report retained only as history.

UI assets now staged by tools/ui/stage_native_ui.py: 7 English resources, 17 source
pages, 916 sprites, raw untranslated IDT, provenance in assets/native/ui/manifest.json.
Validated Rust loader is crates/mw-data/src/ui.rs. No UI renderer integration yet.
Current mw-play HUD still debug text and Escape exits. Pause has NOT been implemented.
Gap list and acceptance sequence in NATIVE-PORT-GAPS.md.

Read-only Ghidra decomp scratch %TEMP%/mw-havok-proxy/ui-next.c covers
80237960,80237DC0,80237E0C,80237EE0,802380F4,802416B0. First family is BloodBath
Challenge selection (native strings 80563EFC Unlock BBC, 80563F08 Random BBC);
802416B0 is results/score processing. These are not the IDT decoder. Original
widget APIs called include 800CEB9C (resource/animation binding), 800CF768,
800D0180 and manager 8024BEBC/8024C2EC. Decode their native consumers next to
recover IDT commands/transforms rather than inventing widget positions.

Ghidra scratch MotionNative on owned DOL+RSO, readOnly headless invocation:
C:/Tools/ghidra_12.0.2_PUBLIC/support/analyzeHeadless.bat <scratch> MotionNative
-process -noanalysis -readOnly -scriptPath <repo>/tools/ghidra-scripts
-postScript MadWorldDecompileText.java <repo> ADDRESS+ADDRESS <scratch>/output.c
Function sizes/names in tools/ghidra-functions.tsv (RSO names available, DOL mostly fn).

Source English pause artwork differs from older research/ui-source/subscr000 images,
which are not English. Always use newly staged *_us pages. Challenge c201 idpac isn't
DAT format: validate IDD/TPL/TUV embedded at 73728; preceding IDD at 73696 is a wrapper.
The staging script verifies framing and rejects unsupported candidates.

Intro: source event/ev21{,_us}.dat; native EventClock already translated/tested in
mw-sim/src/event.rs, but no actor/camera/audio runtime in mw-play. Explorer
D:/Games/gamesfordecomp/exports/madworld/events.json and builders/native.py contain
source clip/camera readers; hash-check source bytes. Existing EVENT-API.md documents
native consumer entrypoints and unknown sequence-command semantics.

Pause must freeze simulation, interpolation, queued sounds, announcer/cue stepping,
BGM intro->loop timing and audio playback; input/menu navigation remains active.
play.rs BGM currently uses Time<Real>, so simply stopping fight.tick is insufficient.
Core PC settings module depends on mw_native_platform; refactor only as required for
pure Rust sharing, not by introducing recompiled core as a dependency.
