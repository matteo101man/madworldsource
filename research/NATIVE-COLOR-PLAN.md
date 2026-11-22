# One-day plan: color everything (scenery, UI, effects)

Drafted by a planning agent (Fable) and trimmed to one working day. Not implemented yet.
Baseline: 36 of 107 live textures matched on the boss intro. The 16 `_AB` masks are left unchanged by design,
so the ceiling is about 91. The pack already passes through the TEV ink path, so no shader re-tint is needed.

1. **Hour 0–1: hash index and class report.** Write a new `tools/index_live_textures.py`. It hashes every TPL image
   using the same rule as `bake_native_color.names()`. Sources: catalog banks, the SCR texture block
   (`case2/c201.dat#c201.scr`, parsed as in `tools/stage_arena.py`), `etc/core2_us.dat` jk*.tpl,
   `etc/font.tpl`, `kanji*.tpl`, `envmap.tpl`, and `core_us.idd` pages (`tools/index_live_ui_layout.py`).
   Join the hashes against `live-textures-*.log` and classify each one as character, scenery, effect, ui, font, efb or unknown.
   *Check:* EFB re-reads are classed `efb` and no more than 5 are unknown.
2. **Hour 1–3: SCR scenery bake.** Reuse `rasterize`/`tint_map` with a generic stage palette
   chosen by mesh-name class: ground is warm grey-brown, walls desaturated olive, metal steel blue,
   wood ochre, signs a saturated accent. Meshes with no name match get a luminance-band hue map. Keep the red guard.
   *Check:* the hit count rises by about 25–35, and `MW_COLOR_CAPTURE` shows chroma on floors and walls.
3. **Hour 3–4: UI atlases.** Tint each sprite rect on the `core_us.idd` CI4/CI8 pages using the layout JSON:
   icons gold, panels cool grey-blue, red HUD kept as it is. Leave glyph-sized sprites and the fonts untouched.
   *Check:* the paletted names turn into hits, and text stays crisp.
4. **Hour 4–5: effects.** Leave jkShadow/jkNoise unchanged. Give jkFlesh/jkSmoke a flat multiply. Skip I4/IA effect textures,
   since their color already comes from konst/raster. *Check:* blood stays red.
5. **Hour 5–6 (optional): global saturation/gain.** Add two `FrameUniforms` tail floats
   (`shaders/common.wesl`, `render.rs::build_frame_uniform`, applied at the end of `run_tev`), driven by
   `MW_COLOR_SAT`/`MW_COLOR_GAIN` and reset to 1.0 when C is off. This needs no specialization-key bump.
   *Check:* the source capture is byte-identical to the one taken before the change.
6. **Hour 6–7: validation.** Add an `MW_COLOR_PACK=0` env read so the GPU goldens stay unchanged. Make one capture with the pack on and record
   the hit/miss counts by class.

## Plan vs. what exists now (boss intro, `live-textures-60780.log`, 107 live textures)

The step-1 classification was done ad hoc by hashing every TPL image on the disc (1530 images: all `.dat` members, SCR
texture blocks, loose `.tpl` files) and joining against the live misses:

| Plan step | Live textures it targets | Status now |
|---|---|---|
| Characters/props (done before the plan) | 36 | **Done.** All matched. Checked against the explorer's own `color.js` (below). |
| 1. Hash index and classes | all 71 misses | **Done ad hoc.** No `tools/index_live_textures.py` yet. |
| 2. SCR scenery bake | **34** (32 from `c201.scr`, one each from `c103`/`c105` SCR) | **Done.** `bake_native_color.py --scenery` colors every stage's SCR textures by geometry use (ground/wall/roof/cut-out). All 34 are now in the pack, so the boss intro is at 70 of 107 (checked offline against the live log). |
| 3. UI atlases | 11 CI4/CI8 | **Reclassified and done as far as textures allow.** Four 1024x1024 pages plus a 256x256 one are `etc/core2.dat` *effect* sheets (blood splatter, sparks, smoke, wipes). They are white masks the game colors per draw, so they are left untouched to keep blood red. The real HUD (`core_us.idd`) is also white glyph masks colored per draw, so it is not tinted either; HUD color would need per-draw changes in the shader. The two 128x128 pages are the 亀/龍 plaques in `caseF/cf02.dat:cf02.eff` and are now tinted. Four 1024x1024 pages are in no file on disc (compressed or built at runtime) and need an in-game dump. |
| 4. Effects + HUD | effect sheets, HUD pages | **Done, texture-only.** `--fx-hud` uses red-neutral hues (red channel = 1.0), so blood and red-konst draws are unchanged while white/grey masks take color; jkShadow/jkNoise are excluded. It was tested in the explorer lab (`exports/madworld/lab/fxhud.html`): blood-red output change is 0. The boss intro is at 78 of 107. |
| (none) | 2 strap/splash images (`strap/strap_16_9_us.tpl`) | Not in the plan. Easy to add as a flat tint, or skip because the splash is being removed. |
| (excluded) | 16 `_AB` ink masks; 853x480/640x448 framebuffer copies (3); 4x4 constant; 2 small unindexed (128x128 CMPR, 256x256 IA8) | Not replaceable or not color. |
| 5. Sat/gain uniform | n/a | Not started (optional). |
| 6. Validation | n/a | Partly done: `MW_COLOR_CAPTURE` A/B exists, but there is no `MW_COLOR_PACK=0` switch yet. |

**Result after steps 2–3: 72 of 107** live boss-intro textures are in the pack (checked offline against the live log).
The other 35 are 16 ink masks, 7 effect sheets/inputs colored per draw (left alone by design), 4 pages with no
file on disc, 3 framebuffer copies, 2 strap images, 1 constant and 2 small unindexed textures. The earlier
"81 of 91" estimate assumed the 1024x1024 pages were UI; they turned out to be effects.

**Deferred:**
- Luma-preserving shader re-tint (2–3 days). Not needed for the current hits.
- Hand-painted signage (20–30 min per texture).
- Per-draw konst table for particles (1 day).
- EFB re-reads. These can never be replaced and inherit scene color anyway.
