# Pure Rust port gaps - 2026-11-09

User request: after repairing fight timing, continue porting toward the recomp's
full UI, pause menu, intro cutscene and challenge menu. Kreely playtesting is
explicitly excluded. This tracks mw-play, not madworld-native (the oracle).
The table is current; continuation notes below preserve earlier checkpoints.

| Area | Present in pure Rust | Missing / next concrete work |
|---|---|---|
| Fight fidelity | Root movement, HP, reactions, fans, AI, clash, grabs, finisher, death/continue; recursive native contact-pose blending, retained gate contacts and corrected published target ordering for fan launch and AI selection, enemy pre-advance pose sampling and scene-scaled get-up protection expiry; automated replay regression gate | Canonical 1 HP frame; independent recording now has zero reported mismatches across1756 frames; catch/idle drift cleared in those two recordings; additional capture still has AI/hidden-state divergence; full proxy/filter/contact parity |
| Original UI assets | Newly staged 8 English sources, 18 PNG pages, 1067 exact TUV sprites; validated mw-data::ui loader | UiScene handles bounded transforms/envelopes/visibility and digits with native observation tests; remaining: other channel commands, draw compositing/clipping, special texture styles and native TEV/alpha/gamma behavior (current TV noise is too bright and dims slot legibility); shared original quad renderer and c201 pause map are now integrated |
| HUD | Original player39/boss31 health bars in normal play with live HP, front/trail progression, opening/flash and boss hold/fade; freeze/hide under pause and entry; debug state text opt-in;strict low-HP panel variant and program58 controller;400 original arithmetic calls,17 live geometry/RGBA/cropped-UV comparisons and340 original low-HP controller updates | Remaining counters/meters, prompts/subtitles, healing-completion/menu sounds, score/lives/weapon/status consumers, native per-update admission/teardown and pixel/material parity; replace temporary gameplay status hints with original prompt consumers |
| Pause | Escape pause/resume; fight/Virtual/BGM clocks and owned audio freeze; input release gate; original map/legend/arrow, Options, Wii Controls, Sound, Video calibration and ten-page Play Log, Title confirmation/return to original splash with navigation/back and save/discard; bounded freeze/resume smoke | Live Play Log statistics and full Title/main-menu flow, PC glyphs, remaining animation/render parity, original subpages, settings integration and safe quit/retry confirmation |
| Title/main menu | Original defaultNo confirmation, staged logo/Press, four native save rows, occupied START/COPY/DELETE panel; Rust-owned record/option persistence with interrupted-write recovery, confirmed copy/overwrite/delete and error/reload handling; fight audio removed; native empty START verified to persist records and reach B01 | Actual main menu, NewGame/Continue/Options admission, title BGM, native save/error widgets and precise transition timing; B01 opening cinematic/scene remains unported and Rust START cannot enter it yet |
| Intro ev21 | Default playable startup; source camera/actors/props/mesh cancellation/fade/audio; original comic lettering and WCD111; B/Menu Back skip; source cleanup wait, resource gate and15-update room fade with AI active/input blocked; BS04C retained or early-skip BS04 fallback; native normal/early/late-skip lifecycle checks and runtime smoke; source-checked absence of MESS (EV21 has no dialogue message track) | Other effect/render flags, first draw-time attachment/UI phase, SE DSP fidelity and general WCD/RNG side effects; complete world boot/boss-switch prerequisites, object messages bl015d/bl001c_N002, native actor/physics/RNG state and all scene transitions; original pixel/all-bone/PCM parity unproven. Explicit preview remains an inspection option |
| Stage challenge menu | Original boss-entry program42, stage message12010008/font, Yes/No selection and source animations; optional MW_CHALLENGE=1 joins intro; native accept/decline and48-glyph comparisons | Real roaming-world admission/return, native menu SE, saved ranking/unavailable path, abort/results and broader challenge systems. No exits opt-in inspection; default starts at intro. No general challenge list has been established by this controller |
| Event messages | Optional MESS loader, source glyph catalog, opcode18 show/clear and subtitle option admission; fixed first-line baseline and family3 spacing; EV00 inspection renders all4 original messages,209 native glyph geometry/UV checks; EV21 correctly has no MESS | Native configured subtitle-window overrides, other tokens/scrolling/controllers, wider event/voice synchronization and real world/cutscene admission; EV00 inspection is not a complete Man Darts scene |
| PC controls/settings | Recomp has rebindable KB/mouse/gamepad config and F10 menu | mw-play uses fixed keys and separate controls; share settings/bindings without core dependency; device glyphs, gamepad, window persistence; Sound volume/subtitle persistence is integrated |
| Death presentation | Native mode-4/ramp slowdown and default PC 0.35 motion rate | PC cinematic camera, original continue/game-over widgets, lives/save state, scene/retry flow; arbitrary continue input is currently simplified |
| Victory | Simulation reaches Victory | Full victory cutscene, result/score screen, unlock/save and return flow |
| Rendering | Native arena/rig meshes; added color default on characters and arena (recomp pack); wp0600 chainsaw arm at bone 8 (bind pose); native EST emitters for hits (etc/core2.dat core2.eff via tools/stage_effects.py, which names each field's native reader: lifetime, count/interval/duration, offset/velocity with U[-1,1] ranges, drag, accel, gravity, rotation/spin, size, RGBA, TUV sprites, pivots, floor stop); MW_TRACE_EFFECTS (core hle wrapper) traced triggers: player hits (1000+1005, 28 at HP<=50), motion op-16 cues for both actors, RinRin state code (111, 200/201, 140, 73/74, guard 12, 60/61) native character ink (captured white/black TEV programs, play_ink.rs) with the original's light state read from the recomp renderer: sun toward (0.389,0.819,0.422), a camera-position light for the facing/outline alpha, konst/registers; GX culling (clockwise fronts, back faces culled: the *_UR_ back meshes otherwise z-fight the cloth); x-ray *_HN_ and cut *_DA meshes hidden in normal play; image quality as the recomp renderer (play_look.rs): GX mip chains rebuilt (arena 4, characters 6 levels, trilinear), no tone curve, 4x MSAA with alpha to coverage on the arena cut-outs, 2x supersampling (MW_SSAA) as the recomp's EFB scale; electric arcs and flares (textures 33/34) additive; jkShadow per caster (RinRin 0.5/4.0 enemy table, Jack 0.5/2.0 player init fn_802705DC); added test looks MW_LOOK=modern|filmic / F3 (lit stage with sun shadow maps, SSAO, bloom, sharpening, screen-space ink outline from the depth/normal prepasses in play_outline.rs; flickering electric-fence point lights at the c201 arc draws, flaring on DM_ELECTRIC; night/day skyboxes (play_sky.rs, procedural cubemaps, night default, F2 with the grade) and night/day moon, sun, ambient; neon sign glow from lettering masks and sign point lights (tools/stage_sign_lights.py); RinRin spring bones (play_springs.rs: breast/butt jiggle after Punchout secondary.rs, skirt and hair-ornament Verlet ropes after Punchout whip.rs; butt bones 200/201 added by tools/stage_rin_jiggle.py); RinRin texture touch-ups (back of the head to hair navy, brown eyes; tools/stage_character_color.py); stage-start Shock TV broadcast behind the boss challenge (play_broadcast.rs: room clip cam00 from tools/stage_room_cameras.py, idpac_c201_us program 6 "LIVE ASIAN TOWN 53F / GREAT WALL STREET", kung-fu performers spinning em0101_0106 and fire-breathing em0101_0110 with an added flame plume); street stage MW_STAGE=street (play_street.rs: c201's streets, spawn set c20100 via tools/stage_street.py, em0101 kung-fu grunts in color via tools/stage_actor.py + stage_kungfu_shared.py, added grunt AI on native clips and SEQ hit windows, Fight::solo / player_strike / enemy_hits_player); Kreely pistol: authored reload (play_reload.rs, two-bone IK), tracers (play_tracer.rs), reticle-converging aim, RinRin fan deflects (Shot::Deflect, 30% + her native guard roll); filmic = AgX); the recomp's color grade (play_grade.rs); arena alpha-tested cut-outs (grating floor, fences) as the original's stage program (texture x vertex color, vertex colors are white); character shadows as the original's jkShadow blob decal (create/trans/draw fn_80263F3C/fn_80264078/fn_802644BC: ray down 4.0 from the actor, quad 1.5 x 0.5 x (1-(d/4)^4) on the hit normal, jkShadow warped by jkNoise at world xz x 1.5, alpha test > 64, solid black; tools/stage_shadow.py, play_shadow.rs); colored Jack/RinRin from the recomp color pack (tools/stage_character_color.py) | Character ink: shadow maps (TEV maps 0/1, taken as unoccluded), channel-0 vertex colors, the second konst state's trigger; blob shadow: the indirect-warp strength is fitted to captures (a quarter of the GX scale as computed), the ray is col.hkx rather than fn_800ABAA8's collision query, and in the intro the first native bone stands in for the object position; broadcast: which room clip the native broadcast plays is inferred (cam00), the native fire variant's flame emitter is not staged (an added plume stands in), program 6's tiled TV-noise layers are hidden (port tiling coarser than GX); street: grunt AI/tuning is added, not the cEmZako states, the camera does not collide with grunts, no kill counter or stage objectives; the additive blend of the arcs/flares is inferred (the GX blend was not dumped) and the arcs were not seen live yet; an intermittent all-black frame: one cause found and fixed (the supersample target was made before the window had a size, so SSAO's depth mips failed validation), but one more black capture followed and is unexplained; mips are 2x2 averages, not the disc's own levels; x-ray skeleton not shown during electrocution; fog is not drawn by the recomp renderer either; effect anchor bones beyond the root (setEffect part ids) and Jack's own seq effects' visual check; decode the effect draw path (size/aspect semantics inferred), over-life alpha/size curves and TUV animation; 131 is a 2D screen effect, not drawn; chainsaw blade motion; native bomb model (currently spheres) |
| Audio | Corrected native sound-ID definition lookup and restaged mappings; BGM native stream gains, fight sounds/announcer triggers; intro music/English voice, numeric2D dry effects and retained boss stream; Music/SFX/Cutscene/Announcer controls and pause/resume | Complete SE program interpreter, pitch/envelopes/effect sends and native spatial/output mix; full scene/menu/subtitle audio sync, PCM/routing parity and exact prioritization; other announcer lines need systems not in this encounter |
| World systems | c201 arena and RinRin encounter | Full stage enemies, scoring/object interactions, bloodbath/challenge gameplay, progression; required before claiming general game completeness |

## Work order

1. Keep replay reports and native evidence current; fix contact/catch boundaries without per-frame special cases.
2. Original UI resource boundary and renderer; translate IDT from its native consumers.
3. Pause flow with genuinely frozen clocks/audio; original UI widgets and input routing.
4. Intro ev21 actor/camera/audio/subtitle playback and skip-to-fight handoff.
5. Stage-201 challenge flow and original HUD widgets; broader systems only with native evidence.
6. Recomp PC settings/controls parity and remaining presentation/game flow.

## Completed in this continuation

- Fight regression reduced 132 -> 1 canonical HP frame; 39 simulation tests.
- tools/ui/stage_native_ui.py stages source pixels without editing them, including
  validated embedded IDD at offset 73728 in case2/idpac_c201_us.dat. Manifest
  records original container hashes and offsets; raw IDT is preserved, untranslated.
- mw-data::ui validates source-page references, contiguous group/index tables,
  exact normalized UV/pixel agreement, rectangles and safe asset-relative paths.
  Tests cover all 7 sources and malformed references/path traversal.

Evidence: research/FIGHT-REPLAY-FOLLOWUP.md; assets/native/ui/manifest.json;
research/UI-CORE-LAYOUT-API.md; research/EVENT-API.md. Neither atlas staging nor
an event clock alone counts as a completed original menu/cutscene.


2026-11-09 UI runtime continuation: UiTimeline now decodes132 screens/4397
widgets; native explicit-frame vector interpolation exists. Original pause XFB
and313 live widgets captured; source subscr000 screen0 fully bound.45 tests pass.
Preview remains visually incomplete; pause/HUD are not integrated into mw-play.
See UI-NATIVE-RUNTIME.md and NATIVE-UI-NEXT.md for exact current state.

2026-11-09 pause ownership: Escape pauses/resumes mw-play, with frozen combat,
Virtual/BGM time and owned audio; input release guard prevents menu actions from
leaking into combat. Original UI scene transforms/scale/color/digits gained a
45-node native observation regression; all 4397 widgets validate appearance.
46 library tests pass. Full original menu and intro/challenge remain outstanding.
See latest NATIVE-UI-NEXT.md checkpoint for diagnostic rendering limitations.

2026-11-09 original pause map integration: shared native_ui renderer fixes intrinsic
image sizing and supports native tiled atlas rectangles. Original c201.imt floor
maps, player marker and area legend are now visible in mw-play. Up/Down changes
the native Play Log/Options highlight; confirming those pages remains unported.
Lives2/score0 are initial values until underlying lives/score systems are ported.
47 tests pass; actual pause/freeze/resume smoke and screenshot inspected. See
evidence/rust-pause-original-map.png and latest NATIVE-UI-NEXT.md checkpoint.

2026-11-09 Options/Controls: real native menus now open/back in mw-play; shared TV,
confirmed-entry inversion and native alternate frame are translated and tested
against new captures. Controls still uses Wii artwork; recomp PC glyph/settings
parity remains. Sound/Video/Title confirmations and Play Log still no-op. Play Log
92 original messages/27 glyphs are staged but not rendered.50 tests pass; both
binaries check; real-window page/freeze/back/resume smoke runs pass. See latest
NATIVE-UI-NEXT.md checkpoint before continuing; goal still active.

2026-11-09 Sound: original screen70 and save dialog86 are interactive. Actual
Rust music/SFX/announcer gains preview edits; apply saves shared audio config,
discard restores previous gains. Cutscene voice/subtitles persist pending event
playback. Corrected native gauge anchoring and UV crop, including right-filled HP.
53 library tests plus2 audio and1 native-config test pass; live BGM gain/save/freeze/
resume smoke passed. See latest NATIVE-UI-NEXT.md; older checkpoints are historical.

2026-11-09 Video: original full-screen calibration program76 opens and returns
through Options/Map without resuming combat early. Native geometry/visibility and
navigation regressions pass; hidden-window freeze/resume and screenshot verified.
55 library tests pass. Original VI adjustment submode and PC prompt parity remain;
PlayLog and Title actions are next. See latest NATIVE-UI-NEXT.md checkpoint.

2026-11-09 Play Log: original fonts/metrics,92messages,10page wrap and native
layout bindings integrated. All75first-page glyph centers/sizes/UVs match original
message pool. Actual smoke verified first/last page captures, freeze/back/resume.
60 library tests pass.46 statistics remain initial zeros pending producer systems;
PC glyphs/general message styles/transition/pixel parity remain. Title is next.

2026-11-09 Title: original dialog85 and E00 logo/Press splash integrated. Choosing No
returns to Options; Yes removes fight voices/queues and stays at title without
resuming combat. Press Start and the main menu are not yet interactive. New E00
IDD stages25screens/967widgets.63 library tests and actual smoke checks pass;
axis scale-inheritance correction also matches previous UI observation tests.
See latest NATIVE-UI-NEXT.md for exact native consumers and next menu capture.

2026-11-09 save selector: Press reaches original four-slot selector and empty-slot
START panel, with clamped selection, Back and verified combat freeze. Geometry,
visibility and RGBA match native62+124+22records;66 library tests pass. The actual
main menu is later in the native flow. START activation and save handling remain
unported. Current TV/noise compositing is too bright: dimmed rows are hard to read;
matching widget RGBA alone does not establish native pixel/legibility parity.


Low-health HUD continuation: player39 now uses the original alternate root
appearance strictly below30%, preserves that binding on further damage, and
restores its original source on recovery. The source RGB loop resets at0 and
repeats every15 updates. Program58 admission/reversal clocks are translated;
its English border/noise geometry is disabled and its red quad stays alpha0
in all340 original controller observations, so no extra screen flash is drawn.
The controlled native fixture calls only a private UI controller's health API;
it never writes actor HP. Rust gameplay independently reaches20%/death and
Continue restores normal appearance.93 library tests pass.

Fan contact follow-up: the first throw's68 hand observations expose a pose
publication phase discrepancy. The previous Rust pose matches source rotation
within0.00000435; same-update error reaches0.08732. A pre-advance sampling
experiment matched original fan transforms closely but failed existing contact
regression limits in the independent recording; it was reverted. Runtime
retains the passing1+1 HP-frame baseline. Extended replay diagnostics and
FAN-HAND-PHASE-AUDIT.json preserve a concrete next lead without relaxing checks.


Fan transform correction retained: native cByMotion's pre-advance frame/blend
age now drives Rin's model pose, including new-motion transitions. Her throw
target includes Jack's newly published root delta as well as the lagged actor
translation and collision projection. This removes every reported mismatch
in the1756-frame independent fight recording (HP1->0); its regression gate
now permits zero. Canonical still has1 HP frame and its prior motion/position
counts; the frame398 gate check still passes.42 original checkpoints cover
first-throw fan geometry and eight launch targets/progress rates;93 library
tests pass. See FAN-PUBLISHED-TRANSFORM.md. The earlier rejected pose-only
experiment above is historical, not the current retained implementation.


Event message continuation: EV21 has no MESS chunk.800A4E74 creates a message
task only for MESS, so the previous recurring EV21 dialogue-subtitle gap was
incorrect. Source-hashed absence is now loaded/checked by playable entry even
with subtitles enabled. Broader subtitle systems remain required.

Added the optional MESS bank boundary and opcode18 command controller. EV00
(Man Darts introduction, not B01 opening) provides a positive4-message case:
original35-glyph font/metrics/tokens, native52-update UI component capture and
209 glyph geometry/UV comparisons. Event family3 uses a fixed first baseline
and0.3 trailing width; challenge family1 centers rows and uses0.2. Original
clear and subtitle-off inspection renders emit zero glyphs.96 library tests,
all game binaries check, fight gate1 canonical/0 independent, and EV21 enabled-
subtitles skip/handoff smoke pass. See EVENT-MESSAGES-API.md for exact scope.
