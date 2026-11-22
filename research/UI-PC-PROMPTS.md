# Native PC prompt resources

The active owned resource is **files/etc/core_us.idd**, SHA256 b5d50f1b0ae656235fa7aee877faabf142b7e8a7e90f37606200016208d5f455, 3,050,208 bytes. Native DI tracing in evidence/bevy-native-pc-ui-disc.log establishes that this boot reads the standalone resource. The earlier core.idd inside core2_us.dat was a candidate containing similar art, but that archive was not read in this boot. The first candidate conversion therefore did not change live prompts.

Reproduce the active layout with tools/index_live_ui_layout.py: six texture pages, 42 groups, 546 exact source sprite rectangles. The active graph is ui-live-core-layout.graph.jsonl; UI-CORE-LAYOUT-API.md describes its vocabulary. TUV's second u16 selects a texture page in the multi-page resource (the single-page archived candidate had only zero). IDT command sequencing remains undecoded.

Run tools/stage_pc_ui.py to regenerate staged/pc-ui/core_us.idd and manifest.json. Eleven slots are converted: small/common A to LMB (1,47), B to RMB (49), C to Q (51), Z to SPC (54), Plus to TAB (42), Minus to ESC (43), and four device views to RIGHT/LEFT keyboard gesture cards (64–67). X is the held two-arm struggle binding. Colors/highlights remain controlled by the original palette/renderer. Conversion writes only source CI4 pixel nibbles in these exact rectangles. Every other sprite, every palette, all mip metadata and all other bytes are preserved; all modified pages have maxLOD zero. The original extracted resource and RVZ are untouched.

The native DI adapter accepts only a same-length override for /etc/core_us.idd through MW_PC_UI_FILE. The Bevy frontend selects the staged resource when present. An explicit MW_PC_UI_FILE can choose another same-length local file; invalid files fail the DI load. This is a sparse UI retarget, not a generic filesystem overlay.

Read-only evidence/native-pc-ui-live-memory.json matches all 524,288 bytes of each modified page (1,2,3) in guest RAM to the staged SHA256 values. This establishes loaded pixel content, not texture binding, frame composition, live legibility or animated orientation. Asset contact sheets in research/ui-source are data inspection, not game screenshots. The source's device animation transforms remain active and require visual assessment. Full contextual PC prompting and complete removal of every Wii-specific screen remain open; the F1 help panel is supplemental.

The archived candidate decoder/converter is retained as tools/index_ui_layout.py and tools/stage_archived_pc_ui.py for provenance. It is not the active frontend resource.

## 2026-11-08 — Runtime PC glyphs (keyboard/mouse and gamepad), host-side

The staged DI file (above) stays as it was. On top of it, the renderer now draws PC glyphs over the original prompt sprites at runtime. Glyphs follow the last device used and the current bindings. The feature is **on (auto) by default** (user decision, 2026-11-08): `MW_PC_PROMPTS=auto|kbm|pad|off` picks the start mode and **F4** cycles off → auto → keyboard → gamepad.

**Inventory (etc/core_us.idd, `research/ui-live-core-layout.json`).** Every controller sprite in the live UI resource. The other UI atlases checked (nl, subscr000/001/0fe, ex000_us) contain text and icons only, no controller art.

| sprite | page | size | Wii art | role → action (pc_config.rs) |
|---|---|---|---|---|
| 1, 47 | 2, 3 | 18, 28 | A | Attack, else Confirm |
| 2, 49 | 2, 3 | 18, 28 | B | Chainsaw, else Back (sprite 2 was missed by the DI conversion) |
| 51 | 3 | 28 | C | LockOn |
| 54 | 3 | 28 | Z | Jump |
| 42 / 43 | 3 | 20 | + / − | Plus / Pause |
| 44 | 3 | 28 | Nunchuk stick | Move (WASD cluster / left stick) |
| 50, 52, 53 | 3 | 28 | D-pad (plain, outlined) | Pickup (D-pad up; cIdActionButton2 icon 0xB) |
| 64, 65 | 1 | 46×154 | remote (front / angled) | Shake, or RightArm<dir> under a directional swing prompt |
| 66, 67 | 1 | 36×140 | Nunchuk | **Removed**: patched fully transparent in every glyph mode (user request, 2026-11-08); the remote's Shake binding shakes both arms on PC |
| 48 / 45 | 3 | 20 | 1 / 2 | Not bound on PC; left as the original |
| 46, 55 | 3, 1 | 28, 40 | warning triangle, glow | Not controls; unchanged |

The remote and Nunchuk pictures are what `cIdPlCockpit::setSwingNavigation` (8022F284) animates for every swing prompt: the two-arm struggle, the shake/finisher prompt (kind 0x11) and the directional swings. They are shown with a binding stack: the glyph(s) of the bound input, then a "SHAKE" label for shake prompts, or a modifier + direction for swings (gamepad `RB + R-stick ↑`, keyboard `↑`). The game keeps moving and rotating the sprite, so the motion still shows the direction.

**Prompt kind.** The core's input HLE reads the remote prompt kind (cockpit child +0x18) every pad sample (`input_hle.cpp remote_prompt`). `mw_pad_set_prompt` now also stores it in `host::remote_prompt()` (native-platform/src/host.rs). This is read-only presentation state. Mapping: 0x10 = shake (also the two-arm competition) and 0x11 = keep shaking, both established; 0x0C..0x0F = setSwingNavigation enum 0..3 (down/up/right/left) is **inferred** from 0x10/0x11 = enum 4/5. Only the remote's kind is read. The Nunchuk pictures are removed (transparent) whenever glyphs are on.

**Open: the live kind reads 0.** In every capture run, including during the finisher's shake prompt, `host::remote_prompt()` stayed 0. `remote_prompt` returns the kind only while the widget's active byte is set. The likely cause is that the game clears that flag after each drawn frame (MOTION_TO_BUTTONS.md: "show flag is cleared after every drawn frame"), so a pad read at frame start never sees it. Not verified: the cockpit object is outside MEM1, and the host's `peek_mem1_u32` cannot read it. Consequences: (a) the device pictures always show the **Shake** stack, which is right for shake/struggle/finisher prompts but not for directional swing prompts (those would need `RightArm<dir>`); (b) the A/D shake QTE in pad.rs, which keys on kind 0x11, probably never triggers live either. Fix (core, input HLE owner): report the stored kind regardless of the active byte, or sample it from the draw path. The frontend already handles 0x0C..0x11 once it arrives.

**Glyphs.** `tools/gen_pc_prompt_glyphs.py` writes `native-host-data/pc-prompts/` (`glyphs.rgba` + `glyphs.json`, 128 inputs × sizes 18/20/28/36/46; `pages.json` with the slot rects and the disc + staged texture names of pages 1–3). Preview: `research/ui-source/pc-prompt-glyphs.png`. Everything is drawn from plain shapes and the Windows Arial Bold face, at 8× supersampling. Key caps carry the key label (arrows and Space as drawn symbols). The mouse is a generic outline with the pressed button filled. Gamepad glyphs are generic: A/B/X/Y circles, LB/RB pills, LT/RT triggers, L/R sticks (with a flick arrow or rotation arc), a D-pad with the pressed arm, and Start/Select pills. No colors, no brand or game art. Like the originals, glyphs are white ink on a black face with a transparent outside, so the game's own tint, blink and animation still apply.

**Rebinding.** `crates/mw-game/src/native_prompts.rs` picks each slot's glyph from `PcConfig.bindings` (the pc-gamepad-settings session's `Action`/`Input` names) for the active device set, preferring a single-input binding. It rebuilds when bindings change. Input names come from `pc_config.rs` (`KEYS`/`MOUSE`/`PAD`/`STICK`), which the generator parses so the two stay in sync. A rebound Move shows its up key instead of the WASD cluster. If an action has no binding on a device, that slot keeps the original Wii glyph.

**Device switch.** Any key, mouse button or wheel selects keyboard/mouse. Any pad button, a stick past 0.5 or a trigger past 0.5 selects the gamepad. In auto mode the glyph set follows that.

**Renderer.** New `backend-wgpu/src/texture_patch.rs`, with `Renderer::set_texture_patches(name → full-size RGBA overlay + rects)`. When a draw binds a key whose texture name matches, the renderer builds a per-key copy of whatever the key currently shows: the guest decode, the color pack or the legacy pack. It does this with GPU texture copies in the frame encoder, after any pending decode, and then copies the glyph rectangles over it. Guest and pack textures are never written, and the color/legacy toggles keep working (the copy is rebuilt when its base changes). Hooks: `note_load` in LoadTexture, `apply_texture_patches` before each draw flush, and one view preference in the bind-group build. With no patches installed nothing is hashed or copied. Names cover both the disc page hashes and the staged-DI hashes, so this works with or without `MW_PC_UI_FILE`/staged/pc-ui.

Tests: `native_prompts::tests` (default bindings → MouseLeft/PadA/PadL3/PadRB+PadRStickUp, every role resolves in both sets for every gesture, 6 patched page names), `modes_cycle_and_parse`.

**Headless evidence (2026-11-08).** `MW_GPU_TEST_PROMPT_CAPTURE=<dir>` (native_gpu.rs + `native_prompts::CaptureProbe`) saves keyboard, gamepad and original frames, about 0.5 s apart, on each new prompt kind, when the PC test enters a shake phase, and periodically. `tools/pc_prompt_captures.py <dir>` writes PNGs, side-by-side sheets and `index.json`. Runs used `tools/run_owned_gpu_test.py --sustain --exe <copied test exe>` (new optional `--exe`) with `MW_TEST_RIN_FINISH_WAIT=1 MW_GPU_TEST_PC_FINISH_CONFIRM=1`.

- `evidence/pc-prompts/finisher/` (log `evidence/pc-prompts-finisher.log`) is the result. Jack's finisher shake prompt (phase finish-shake, t=77.7 s): `sheet-0077.7-kind00.png` and zoom `zoom-shake-prompt.png`. Keyboard shows an **X key cap + SHAKE**, gamepad an **L3 stick + SHAKE**, both legible at native 853×480. Everything outside the glyphs is transparent, so no remote/card outline remains. With the feature off, the staged-DI rectangular "RIGHT …" card is shown, unchanged.
- `evidence/pc-prompts/finisher-unarmed/`: the same prompt *before* the arming fix, with the card in all three frames. Cause: texture names were only recorded after the first non-empty patch set, and the UI pages load at boot. Now any `set_texture_patches` call arms naming, and the frontend and probe make that call before boot.
- `evidence/pc-prompts/combat/` and `finisher-try2/`: periodic frames in combat with no controller sprite on screen. The renderer path is otherwise unchanged (color pack and HUD intact).
- Composed tall-slot preview (not a game capture): `research/ui-source/pc-prompt-device-slots.png`.
- All runs ended with test exit 101. The finisher runs hit `[ppc] fatal: indirect call to 0x802E5878 has no recompiled function` after the finisher (RSO lr 0x80D225C4) with the core DLL rebuilt by other sessions at 16:51. One run stalled in finish-approach. Neither touches this feature.

**Not yet seen live:** the small button prompts (A/B/C/Z/+/−/D-pad/stick sprites; e.g. cIdActionButton2 "Pick up"/"Finish"), the two-arm struggle (kind 0x10) and directional swing prompts. Their slots go through the same page patch as the verified shake picture, but no capture route showed them.

**Color pack dependency (2026-11-08).** native-color baked the staged pages 1–3 (ffe7119e…, a0fa86ed…, 08589bd7…) into color-pack and color-pack-legacy with the HUD tint, so the prompt patches now sit on tinted pages. If `tools/stage_pc_ui.py` regenerates staged/pc-ui/core_us.idd, its page hashes change: rerun `python tools/bake_native_color.py --fx-hud` and `--legacy --fx-hud`, and `tools/gen_pc_prompt_glyphs.py` (pages.json names).
