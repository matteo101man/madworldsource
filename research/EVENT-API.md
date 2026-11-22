# Native event API and focused queries

Current message checkpoint: EV21 has no MESS chunk or message/font commands.
The earlier EV21 dialogue-subtitle gap below is historical and was incorrect.
Optional MESS support, source assets, opcode18 controller and fixed-baseline
glyph presentation are now verified using EV00 as a separate positive case.
See EVENT-MESSAGES-API.md for scope and original209-glyph comparisons.

2026-11-09 boss-entry lifecycle checkpoint:

- 80147BBC calls80123820(0x21) synchronously. The event wrapper loads/starts
  through800A4DC0/800A47A8, waits while manager state>4, finishes event audio,
  then yields13 world updates. Only after it returns does the room load
  c20110.ems, set player point5 and fade from black over15 updates. Rin's
  combat actor onInit is after the event-return log, not an actor running
  underneath event21's OBJ animation. AI does run during the room fade.
- 800A4864 applies task commands, advances the source clock, then checks end/
  skip. State6 forces black; the next update cleans up/reset through800A47DC/
  800A468C/800A4AD4. 800A49F0 tests manager skip-enabled and cPad down0x20.
  8020EFC0 maps WPAD B0x400 to that bit. 80123820's disabled-ID table contains
  18/34/43/51/93(hex); event21 is skippable. Backspace/right mouse supplies the
  corresponding B/Menu Back action in the Rust port.
- Added default-off MW_EVENT_LIFECYCLE bounded read-only world-edge sampling
  of state/clock, fade, player, first-observed Rin onMove and music slots.
  MW_TEST_EVENT_SKIP_FRAME supplies a one-frame WPAD B sample through the
  original KPAD/cPad path, without writing event/game state. The native runs
  complete normally and at skip300/820; endings are clock1079/301/821.
  Source cleanup/wait/loading order and all16 room-fade alpha endpoints match
  the Rust BossEntry model. Resource loading uses a readiness condition;
  the observed5-update native I/O latency is not baked into Rust.
- Normal completion retains currentMusic30/BGM/bs04c.rsd. An early skip starts
  currentMusic24/BGM/bs04.rsd through the room's801246D8(0x18) request. A skip
  after748 retains BS04C. Staged BS04's original93100-sample intro and loop
  alongside BS04C; native header/stream/channel mix metadata is preserved.
  EV21-ROOM-COMPARE.json audits these three music-admission cases.
- mw-play now uses this event/cleanup/resource/fade sequence by default.
  The existing Rust fight is held until the room's combat release, runs during
  the fade with demo_active/input blocking, then enables controls with a release
  gate. Pause and debug HUD remain blocked during entry. Source media/UI are
  removed on cleanup; persistent music stays, with the BS04 fallback only when
  the persistent event stream has not started. Normal/early/late-skip runtime
  smokes and post-entry pause/freeze/resume pass. Inspected fade/clear/pause
  captures provide application checks, not native pixel parity.

Limits: this is the selected boss-entry sequence using the existing Rust fight,
not the full native world boot or boss-switch prerequisites/object messages.
The native diagnostic boot has other nested startup tasks; their residual
demo depths and later fades are not copied as fixed delays. Native actor/RNG/
physics state, phase-perfect fixed/render scheduling, subtitle/effect/render
consumers and general room-clear/victory/save/challenge transitions remain open.
MW_DIRECT_FIGHT=1 or MW_INTRO_PREVIEW=0 bypasses entry for inspection;
MW_INTRO_PREVIEW=1 and MW_INTRO_FRAME retain source preview behavior.

Earlier checkpoints' opt-in/skip/handoff limits are superseded by this one
for the selected stage201 boss entry; their other fidelity limits remain.

2026-11-09 writing-character checkpoint:

- 802E293C loads subscr/wc.idd and resolves core2.dat/core2.wcd. The latter
  has256 records of100 bytes after its16-byte WCD header. 802E2C88 indexes
  the record by ID. tools/events/stage_writing.py preserves record111, its
  source hashes/range and the161472-byte animation-only WC IDT bank.
- Command16 at510 uses mode0/Print2D, ID111 and operands6..8 as a zero vector.
  The NONE actor field is unused in this mode. Record111 selects program0,
  common glyph group61 and glyphs19/7/20/3: THUD. It supplies position200/40,
  scale0.399999917, rotation-20 degrees, RGBA242/241/212/240 and4 letters.
  Its random ranges and exclusion rectangle are zero. Randomized, projected
  and actor-linked writing remain separate unported paths.
- 802E1DF0 applies source slot spacing and count-based32-unit offset, hides
  negative character indices and assigns the glyph group/index. Its source
  position binding references program100/widget3; 800CEB9C context3 retains
  both destination static axes. 802E1FE4 applies the record's transform/color.
- Corrected the earlier interpretation of position flag0x20000. 800C80C0
  invokes800CAF3C, which maintains a separate random offset for axis flags
  0x4000/0x8000. WC's0x20063 has neither axis flag; its ordinary frame-keyed
  Hermite/linear base curve supplies the bounce. The Rust sampler had rejected
  this flag too broadly. It now samples that curve and continues rejecting
  unported random-axis state. Native context3 binding is also implemented.
- 802E2104 sets a30-update deadline and decrements on creation; the expiry
  update frees before drawing. The native clock-edge capture exposes ages0..28,
  before511..539. 319 allocated-widget visibility observations and80 visible
  glyph UV/pose/color samples agree. Position/size/rotation errors are0;
  maximum color error2.98e-8. Invisible owner widgets are checked separately.
  EV21-WRITING-COMPARE.json records limits and thresholds; simulation tests
  also check allocation/removal across all1080 event rows.
- Default-off MW_EVENT_WRITING records bounded native WC widgets without guest
  calls/writes/RNG draws. The owned capture completes120.10s/5912 presentations,
  exit0. Only build-record DLL changed. Rust playback now presents this effect
  on the event clock and expires it at539. The full intro smoke passes all
  previous audio/cut/restoration checks plus4-glyph writing/expiry assertions.
  This is current fixed2D visual behavior, not a full WCD manager or shared RNG
  side-effect port. Native initialization consumes RNG even for zero ranges.

Dialogue subtitles, other effect/render consumers, native room overlap,
skip/end and normal startup are still required. First draw-time alignment,
compositing and pixel parity remain unproven.

2026-11-09 ID comic-lettering checkpoint:

- 800A4E74 dispatches ID to800D9FD0; its IDD child is aligned with
  `(childHeader+0x27)&~31`. EV21 contains an animation-only IDD bank, not its
  own TPL/TUV. tools/events/stage_ui.py preserves its21696-byte IDT bank and
  program IDs0/1/2/3/100; glyph group61 resolves in original core_us/page3.
- 800DA154 command5 calls8023235C with operation `((flags>>4)&1)^1`,
  low16 program ID, layer operand2 and bank `flags&15`. Operation0 creates,
  1 restarts an existing instance,2 removes. EV21 requests flags0x14,
  bank4/layer10, programs1/2/3 at260/350/392. 8023213C stores the request;
  80232064 creates/starts the UI cluster through8024B81C/8024C184.
- 80232178 initially creates program100. It is invisible owner scaffolding
  throughout the1080-row capture. Program0 is never requested. The requested
  programs spell WHOOSH/WOOSH using the original glyph quads; they are not
  dialogue subtitles. Native widgets stay allocated through event end while
  their source alpha envelopes fade to zero.
- Default-off MW_EVENT_UI samples bounded native widget fields at the event
  clock edge: record offset, visibility, UVs, channel clocks, RGBA, size and
  matrix. ev21-native-ui completes120s/5457 presentations/exit0. Edge N sees
  the UI pose applied by update N-1. UI animation advances once per event
  update, rather than twice or by wall-time60Hz. EV21-UI-COMPARE.json and the
  simulation test check14229 UV/visibility observations and400 visible glyph
  transforms/colors. Max position error0.000031, size0.0000039, color6e-8;
  rotation matches. This does not establish first draw-time/pixel parity.
- Rust intro playback now creates the three source programs at their cues,
  seeks by event-frame age, preloads the shared atlas, renders below the
  event fade and clears its quads on exit. Full intro smoke verifies source
  cue execution,6/5/5 glyph peaks, fade-out,4 cuts and restored meshes/props.
  Fixed-frame275/355/400 captures provide application checks, not native
  pixel/compositing equivalence. The intro remains explicit inspection mode.
- At this checkpoint command16 at510 was separate/unported: mode0 calls802E2BBC Print2D with writing
  character ID111 and zero vector; modes1/2 use802E2CE4/802E2E4C. Its WCD
  source/manager behavior were unported; the newer checkpoint above supersedes
  that limit for EV21's fixed2D record111. Dialogue subtitles remain unported.
  Native ID cleanup/reset operations800DA078/800DA0CC and general restart/
  remove behavior are not a complete room lifecycle implementation in Rust.

Earlier checkpoints below retain their scope at the time of writing.

2026-11-09 SE checkpoint:80189530's SE13 modes are0 stop,1 non-spatial,
2 actor-attached and3 positional; bit2 selects numeric ID instead of a name.
EV21 uses flags5, ID7522001/7522006, attributes1, options8, float volume127,
handle slot0 at frames180/866. Target actor fields are unused in this mode.
8026548C case21 resolves se/event/scene_mix7522.rsp via DOL string80566A22.

Native SDK definition lookup804F835C reads+0x40 offsets and resolves
bank+0x40+4*count+(offset&0x7FFFFFFF).804F6DBC uses wave-bank count at program+2
to locate banks/track lengths; the old shared stager's+0x44 lookup was wrong.
The fixed source decoder now resolves the event's last sound6 correctly.

Dispatch table805FDB48 has12-byte member-function descriptors per opcode.
For these exact11-byte linear programs:20 wave(804FF610),22 volume(804FF6EC),
60 effect sends(80500B0C),12 key-on(note)(804FF2AC),15 u16 wait(804FF474),
00 end/key-off(804FE96C).20 writes wave index at SDK voice+19C;1C8 is opcode
index.804FDA28 advances the sequencer by416/120 DSP callbacks per tick, each
96 samples/32000Hz. Runtime uses its mean hold time, not a60Hz delay assumption.

tools/events/stage_effects.py validates source programs and all576 active SDK
voice observations against ev21-native-se-v2.jsonl, stages the exact decoded
waves and observed default dry stereo scalar(23169/32768 per side), and writes
EV21-SE-COMPARE.json. Rust rejects unsupported program/admission shapes rather
than skipping unknown instructions. Native DSP effect sends, release/pitch/
resampling and PCM equivalence are still separate requirements. The full Rust
inspection verifies the two cues plus stream playback/handoff; normal startup,
original overlap, skip, ID/subtitles and actor/render/effect consumers remain open.

2026-11-09 audio/fade checkpoint supersedes the audio/fade gaps below for EV21.
Camera SEQ1 now evaluates the native packed RGBA hold/interpolate track and
matches all1080 rows of evidence/ev21-native-audio.jsonl. BGM/VCE command
consumers80073DA8/80189F04 produce source stream cues. Music mode2 uses the
master, ignoring command volume; bit4 preserves the final track on task end
(80073D70). DOL name tables8052F750/8054F430 resolve EV21_RSD/61,
BS04C_RSD/30 and JKSNN_EVVOSTR_EV21/0x11021. English voice filename is verified
against the native manager, not inferred from the event filename.

mw-play's opt-in inspection now applies fades and plays preloaded source audio
on its event time origin. The frame748 boss track survives handoff and reaches
its loop without a restart. Runtime log rust-intro-audio-smoke.log verifies
three source stream sinks, complete cuts and visibility restoration, then the
boss loop. EV21-AUDIO-COMPARE.json records native manager readiness/play states;
these are not audible sample timestamps. Native PCM/output-mixer equivalence,
event SE13, ID/subtitles/effects, actor render flags and real scene lifecycle,
overlap and skip remain unverified/unported. Normal startup is still unchanged.

2026-11-09 prop/actor checkpoint supersedes the attachment staging limits below.
8011E120 command14 resolves event actor identifier, finds a native bone ID with
801B2450(...,true), and calls8020FE24/FE38 attach/detach. Command15 uses
substring matching in800994F8/802EB90C and sets cancellation flag0x1. Command10's
Jack goggles mode uses80099558/802072E4 with exact mesh names. Other command10
render flags and model modulation remain separate render consumers.

tools/events/stage_props.py stages audited original pl0040/wp0600 GLBs plus
rig and44 mesh-name/node mappings. Rust intro inspection now applies the props
and commands. The read-only MW_EVENT_ACTORS recorder identifies tasks through
their8011DB08 update callback and reads their original names/objects/meshes.
`cargo run -p mw-sim --example event_compare -- evidence/ev21-native-actors.jsonl
research/EV21-ACTORS-COMPARE.json` compares1080 frames: visibility, attachment
pointer and98280 mesh cancellation checks have no mismatches.3802 root and
10372 selected part matrices agree within0.000035/0.00051 respectively.
Two initial prop matrices are observed before their first attached update;
their first draw-time placement is unverified and excluded from matrix checks.
The report states this limit. Full pixel/all-bone/scene lifecycle parity is not
established. ID/effect/render consumers and normal room overlap,
skip and handoff remain required; preview continues to freeze the fight.

2026-11-09 camera/runtime checkpoint supersedes the staging-only and cadence
limitations below for ev21 specifically. Native800838F4/80083988 read sample
eye/target;80082D58 reads roll/FOV and converts FOV radians to degrees with
805307C8;8008229C builds up with ROT_ORDER2: yaw from eye-minus-target, then
roll applied to world-up. Rust EventCameraPose matches1079 native applied
packets within2e-5 per component. The original clock hook reads a packet one
camera application behind its `before` clock; row0 belongs to the prior scene.
No final handoff/last-sample/pixel equivalence is claimed. See
EV21-CAMERA-COMPARE.json and tools/events/compare_camera.py.

The complete native capture reports2VI fields per update throughout; native
vi.cpp defines60000/1001fields/s. ev21 runs at30000/1001updates/s. EventPlayer
binds actor motions by cut and local frame and returns no clip for missing cuts
(8011DB08). Actor SEQ overrides, attachments and lifecycle remain separate.

`MW_INTRO_PREVIEW=1 cargo run -p mw-game --bin mw-play` renders original camera
and Jack/Rin body tracks for inspection. MW_INTRO_FRAME selects a held source
frame. This optional fixture freezes the fight and suppresses its audio/input,
then resumes it after all cuts; it does not reproduce the original overlap.
Normal startup has not been changed to this incomplete lifecycle. Props, audio,
subtitles/ID/fades/effects, native skip and final handoff are still required.

2026-11-09 staging checkpoint: `tools/events/stage_intro.py` now stages the
original English ev21 source plus decoded camera samples/actor motions into
`assets/native/events/ev21`. `mw-data::event::EventData` validates the boundary.
Four cameras have180/375/112/413frames; Rin has cuts1/2/3, the other three actors
have0/1/3. All21 staged chunk-range hashes and the full source hash are verified.
SEQ operands remain unexecuted data; no camera interpretation, cadence,
attachments, audio/subtitles/effects or actual playback is established by staging.

`event-pcode.graph.jsonl` records bounded original resident functions with normalized basic-block edges and switch cases. `event-vtables.graph.jsonl` records constructor-assigned task vtables and native slot targets. `event-resource-leads.graph.jsonl` is a conservative string-reference candidate index, not proof of a consumer. `python tools/query_event.py . ADDRESS` emits focused dependency pseudocode, explicit global RAM values and excluded instructions outside the declared source function body. Always inspect excludedOutsideFunction and unresolved_expression before translation.

Recovered loader:0x800a4dc0 requests event/ev%02x%s.dat using native language suffix handling.0x800a4e74 validates EVNT and dispatches CAM,OBJ,MESS,EFCT,BGM,VCE,SE,ID chunks. CAM initialization0x80085d7c, OBJ initialization0x8011dbf8, MESS initialization0x80112fec. OBJ actor name at chunk+8 and instance at+0x10 become native identifier %s_%02d; model factory mapping is0x8011d478/0x8011d6cc. Unknown sequence commands remain explicit.

Task wrappers contain their native vtable pointer at+0x404. Object table0x805a39e8 and camera table0x80596a98 have two-word headers followed by8 indexed callbacks. Slot meanings require their call sites, not position guesses. Event initialization already demonstrates source actor/cut bindings; physical attachments and scene command effects still require original consumer translation.

Clock0x800a6f60 advances global frame+0x10c and local frame+0x110 by one while global<totalFrames(+0x104)-1. When local reaches the current cut-length table entry(+4+cut*4), it resets local to0 and increments cut(+0x108). Clock reset0x800a6f44 clears cut/local/global/count fields. This is static native stepping evidence; the camera table construction and update frequency require separate proof before upgrading explorer timing claims. Saved event-clock-pseudocode.json is a focused query. Object callback0x8011db08 uses current cut at controller+0x108 to index its MOT table and passes current local frame+0x110 to motion evaluation; unavailable cuts change source visibility bit0x4. Do not infer attach command14 or mesh command15 effects from plausible operand strings.

Adapter functions can contain a tail jump into an uncreated native callee. The decompiler may follow it outside the source body. Such instructions are excluded by the focused query and must be recovered as their own function before translation. Example0x8011dae4 has a continuation at0x800990a4; it is not all one EventObj wrapper implementation.

Rust translation: mw_sim::event::EventClock implements original advance and global/cut seek field effects for validated nonempty positive cut tables(max64). It preserves the final frame and returns false there; unchanged global seek returns false. Rin ev21 boundaries179/180,554/555,666/667 and final1079 are tested. No wall-clock/native update frequency claim.
