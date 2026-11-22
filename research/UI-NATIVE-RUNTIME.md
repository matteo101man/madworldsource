# Current integration status - 2026-11-09

mw-play now integrates the original pause map, Options, Controls, Sound, Video calibration and ten-page Play Log.
Pause freezes fight/Virtual/audio and guards held inputs on resume. Sound edits
actual Rust music/SFX/announcer gains, persists shared config and uses original
save/discard confirmation. Cutscene volume now drives the opt-in intro voice;
subtitles remain stored pending their native ID/message consumer.
Source-anchored gauge geometry/UV crop matches original0.7HP vertices.66 library,
2 audio persistence/gain and1 native-config roundtrip tests pass; actual Sound
page/dialog/freeze/resume and BGM sink gain verified. Controls remains Wii art;
Video geometry/navigation/freeze/resume match captured native initial behavior.
Play Log original text geometry/UVs are verified, statistics remain initial zeros.
Title confirmation and original splash now work; Press now opens four empty save slots and START panel/back. START activation,
save handling, actual main-menu, title music,
full HUD/intro/challenge flow remain unported. Native scale-axis inheritance is
corrected and title geometry is checked against original observations.
See [NATIVE-UI-NEXT.md](NATIVE-UI-NEXT.md) for authoritative latest evidence/next work.
The original decoding checkpoint below is historical, not current integration status.

---

# Original UI runtime translation - 2026-11-09

Goal remains the full original UI, pause, intro and challenge flow in mw-play.
This continuation implements data decoding and position interpolation, with a
visual diagnostic. It does NOT yet integrate a working pause screen into mw-play.

## Native source consumption recovered

- 8024BD90 resolves IDT at IDD+word(+1C), then screen ID/relative offset pairs.
- 8024B81C reads each screen's widget count and record-relative offsets. Records
  have a size at +0, eight channel offsets at +4..+20, native widget ID +2C,
  order +30, flags +34 and factory kind +3C.
- Six signed indices at +A0 are native parent, first child, next sibling,
  previous sibling, self/source index and auxiliary reference. Native traversal
  follows descending serialized siblings: 8024B81C maps A4 to widget+20,
  and A6 to widget+24. First child is commonly the last serialized child.
- Static block +B0 supplies six UV/group indices. 80184A1C copies RGBA at +BC,
  root position +CC, child-relative position +D8 and dimensions +E4.
- 80184B6C supplies absent channel defaults; these cannot be replaced by
  arbitrary pointers into a short record. 800CEB9C binds per-channel variants.
- 800CF768 / 800CECCC start/stop/reset channel flags and counters. 800D0180
  hides widgets recursively; source visibility alone does not reproduce menus.
- 800C80C0 combines parent transforms and the position channel. It clears final
  UI position Z. Eight owned position control points have NaNs only in this
  unused lane; the data reader validates X/Y and normalizes ignored Z to zero.
- 800CB358 position data: control-point count +8, key count +C, vectors +40,
  floating point-index values at +160 and frame integers at +164, stride 8.
  Bits 10/20 select linear/Hermite time mapping; 40/80 linear/Hermite space.
  80249D48 implements Hermite; 80249E24 uses half adjacent differences, including
  half a segment at endpoints. Generated PPC confirms r8 supplies point/key count
  at callers whose decompiled sixth argument is omitted.

## Rust implementation

mw-data::ui::UiTimeline parses all 7 staged English sources: 132 screens,
4397 widgets. It validates directories, contiguous bounded records, relative
channels, hierarchy consistency/cycles and finite static fields. UiWidget's
position_track validates animated vector/key arrays. mw-sim::ui evaluates
explicit frame seeks with native interpolation. Distance-driven bit 20000
requires widget clock state and returns an explicit untranslated error.
Channel start/stop/loop timing and other channels remain to be translated.

The mw-ui-preview binary loads original pages and source quads. Shared core TUV
fallback is necessary: 96 menu references in the first test used common groups.
MW_UI_RESOURCE selects a staged source; MW_UI_PROGRAM a native screen ID;
MW_UI_FRAME an explicit animation sample. Left/Right changes screen, Up/Down frame.
MW_UI_CAPTURE saves a hidden-window screenshot and exits after assets load.
This preview is diagnostic: incomplete visibility, rotation, scale, special
widgets, clipping and dynamic controllers cause overlapping/incorrect panels.
Do not present its output as the original menu or integrate it as a finished HUD.

## Original-core observation

Default-off MW_GPU_TEST_UI_PAUSE_CAPTURE fixture uses original Minus three
seconds after control-ready, then neutral packets, and captures the original XFB
at six seconds. It leaves normal PC test behavior unchanged when absent.
Native run ui-pause-oracle passed 100 seconds / 3962 GPU presentations, host blit
and cooperative shutdown. The XFB clearly shows the original paused arena map,
Play Log, Options, area-data legend and HP/lives panel (ui-pause-oracle.png).
No desktop input or desktop screenshots, and no Kreely playtest.

read_native_core.py --ui-widgets reads bounded list nodes, source record header
hashes, visibility and transforms from the owned process. Native iterator endpoints
are manager+14/+18 (8024CD44/CD3C); next is list node+8 (8024CB3C). Reads are
concurrent, not frame-atomic. Paused snapshot contains 313 widgets, list reached
end; 269 headers match staged source records. The entire subscr000_us screen 0
binds to 159 observed widgets using 158 exact header matches and a common source
base. This is strong source identity evidence, not Rust rendering equivalence.

Captured root matrix center is (320,180); the PC UI canvas is 640x360 for the
853x480 XFB. An earlier preview assumed 640x480 and was corrected. Do not infer
original-console aspect behavior from this PC capture.

Minimal forced-hidden subtrees in this observed map state include widget IDs
30,101,35,6,52..58,61,120,16. These are observations to trace to native controller
calls, not a hard-coded completion implementation. Native screen has 48 visible
screen-0 widgets and 111 hidden ones. Setting source flags alone is insufficient.

## Reproduction

```powershell
python -I tools/ui/stage_native_ui.py
python -I tools/ui/index_native_idt.py
python tools/read_native_core.py OWNED_PID --bevy-host --ui-widgets --label ui-oracle-paused
python -I tools/ui/index_native_idt.py --snapshot evidence/ui-oracle-paused.json --resource subscr000_us --program 0 --out research/ui-pause-program0-observed.json
cargo test -p mw-data -p mw-sim --lib
cargo check -p mw-game --bin mw-play --bin mw-ui-preview
```

45 combined tests pass (3 data, 42 simulation). Both binaries check; native test
fixture builds and runs. Current native fixture executable is
D:/GDecomp/build-cache/bevy-0.20/debug/deps/madworld_native-25e7df48b3ad44bb.exe.
Only the alternate build-record core DLL was used; production DLL unchanged.

## Next

Translate widget clocks, scale/color/visibility bindings and the subscreen
controller. Implement rendering for native geometry and special map/digit widgets,
then validate directly against the captured original. Pause must freeze fight,
interpolation, cue/announcer queues, BGM time and audio; UI remains interactive.
Full mw-play pause/HUD, intro and challenge flow are still required.


## Scene/runtime continuation (2026-11-09)

UiAppearance decodes scalar scale, RGB/alpha and rotation tracks and anchors.
RGB end-slot selection preserves the native FIRST strict maximum on time ties.
UiScene translates source-position bindings, recursive hide/show, digits, leading
zero suppression, gauge state, parent color and separate child-scale propagation.
Source record flags4/2 select scale X/Y. Group rectangle sizes and inherited scale
are distinct in 800C7204: visible quads do not resize descendants by their width.
Native 800C970C position anchor geometry now matches the captured world matrix.
Rotation pivots are decoded; full rotated-pivot geometry is pending.

The new regression compares 45 native-visible nodes' world position, signed size
and RGBA to ui-pause-program0-observed.json (settled frame20, tolerance0.0001).
The two animated scanning strips have unsynchronized clocks in the snapshot and
are excluded, as is fully transparent fade158. This is transform evidence, not
pixel equivalence. Native map target/compositing/clipping are still missing;
evidence/native-ui-scene.png remains visibly incomplete.

mw-play now owns pause via play_pause.rs, with Escape, Virtual time and fixed
fight gating, audio pause ownership and held-input release guard. A basic overlay
is shown while original rendering/page callbacks are unfinished. Full original
pause menu is not integrated. Bounded MW_PAUSE_SMOKE opens90/resumes180/exits300,
checking combat and Virtual time; optional MW_PAUSE_CAPTURE saves frame150.


## Integrated map / texture-style evidence

The large white/static panel failure was primarily ImageNode::Auto intrinsic
sizing, not proven native blend failure. Native dimensions require Stretch;
shared mw-game/native_ui.rs now uses this, tiled atlas quads, signed mirroring,
rotation and native source order. Native 8024BFF4 sorts layer then record+30,
confirming forward source order within a layer. Source+28 is not global Z order.
Texture-style block channel6 has kind at0, repeatXY+C/+10, scroll rate+14 and
signed scroll steps+18/+1A. 179kind1/2kind2 styles validate in staged data. Kind1
tiles translate, including fractional final UV crops. Kind2 distortions, scroll,
non-default blend modes and rotated pivot/clipping parity remain pending.
800D0A78 confirms blend mode comes from source channel0:0 alpha,1 additive,
2 subtract,3/4/5 custom factors,8 alpha. Current painter skips nondefault modes.

New native UI capture includes textureOverride and separate counters. ID100's
map is a texture override; ID71 remains a default source white primitive. All159
nodes bind (158 exact source hashes). After accounting for post-evaluation
position timer increment in 800CB358, the new regression compares all48 native
visible nodes, including strip65/66, using independently captured channel clocks.
UiScene position rebinding now preserves unrelated clocks.

c201.imt is an original three-page TPL in the stage archive at5438336,size117920.
New stage_pause_map.py preserves page pixels and records archive/member hashes.
IDT idpac_c201_us program50 widget30 supplies floors, map scale and origin.
Map floor selection and world X/Z projection follow8024803C/80247470. The original
map, legend (subscr000 program2), and arrow (core program18) now render in mw-play.
Evidence/rust-pause-original-map.png was inspected; no pixel-exact claim. Native
viewport compression/scan phase differ visibly. Menu pages still require confirm/
return controllers. The displayed initial lives2/score0 have no progression system.

Native ctor8024738C proves highlight binding table10,20. Native init/selection
logic drives Up/Down in the integrated pause map. Full PlayLog/Options remain next.
47 combined tests pass; bounded game smoke confirms frozen HP/position/tick and
Virtual time, followed by resumed combat. Both binaries check.


## Challenge-driven transform correction (2026-11-09)

The program42 opening/closing traces refine the earlier scale summary.
800C80C0 scales static child positions, then adds their animated offsets;
animated offsets are not multiplied by inherited scale. In800C7204, when
an active scalar channel exists, both inherited axes multiply the local
(axis-gated) factors. Without a scalar channel, source flags gate inherited
quad dimensions. Descendant scale propagation remains independent of visible
quad dimensions/axis gates. Scale-channel binding uses the bound source's
axis flags. These distinctions are visible during program42's opening and
X-only close, while settled title/save/map tests continue to pass. See
CHALLENGE-API.md and the two full controller trace comparisons.
