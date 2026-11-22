# Look reference: Fist of the North Star: Lost Paradise

Goal: push the added color toward the *vibe* of Lost Paradise (2018, Ryu Ga Gotoku Studio): a warm, dusty,
sun-bleached post-apocalyptic city with cel-leaning characters. We match measurable palette and lighting traits,
not its assets.

## Context gathered
- Lost Paradise runs on the older Yakuza "Kiwami" engine (Yakuza 0/Kiwami), not the Dragon Engine. Nagoshi said the
  team knew it better and that it suits cel-shaded, less-realistic graphics
  ([SegaBits](https://segabits.com/blog/2017/08/30/segas-fist-of-the-north-star-confirmed-to-run-on-yakuza-0s-engine-not-yakuza-6/),
  [Gematsu](https://gematsu.com/2017/08/toshihiro-nagoshi-explains-hokuto-ga-gotoku-not-use-dragon-engine),
  [Siliconera](https://www.siliconera.com/yakuza-general-director-explains-hokuto-ga-gotoku-isnt-made-newer-dragon-engine/),
  [Wikipedia](https://en.wikipedia.org/wiki/Fist_of_the_North_Star:_Lost_Paradise)).
  So the target is toon/cel-leaning characters over a painterly, lit environment. It is not photoreal.
- Reviewers call the visuals mixed. The city of Eden is praised, the wasteland is called lifeless
  ([CGMagazine](https://www.cgmagonline.com/reviews/fist-of-the-north-star-lost-paradise-ps4-review),
  [Bonus Stage](https://www.bonusstage.co.uk/archives/134660)). The city streets are therefore the right reference,
  and they are also the closest match to RinRin's Asian Town stage.
- No published art-direction or grading breakdown was found. The palette below is measured from the user's
  screenshot (an Eden street, 1280x720), using k-means and luminance bands.

## Measured: Lost Paradise vs. our current output
| | Lost Paradise | Ours (color, boss intro) |
|---|---|---|
| Mean saturation | **0.39** | 0.09 |
| Pixels with color (sat > .15) | **95%** | 12% |
| Pure-black pixels (luma < 20) | **0%** | 32% |
| Luma p5 / p50 / p95 | 31 / 79 / **230** | 0 / 57 / 176 |
| Shadow mean | **#39271e** warm brown | #131311 black |
| Highlight mean | **#e8cfa8** sun-bleached sand | #b5b4a0 grey cream |
| Hue spread | **92% in 0–60°** (red→yellow, one warm family) | yellow→green; scenery palette adds jade/lilac/slate |

Dominant Lost Paradise clusters: #402b1d, #523b29, #6b4d38 (shadowed brick/wood, about 41%); #f3ddb0, #ddc09d (sunlit
ground and walls, about 33%); #2a1d1a (deepest shade, never black); #b19c8f (dusty stone); a cool accent #464257
(desaturated indigo: outfits, doors, about 4%); and #d07445 (orange-tan skin, about 1%).

**Diagnosis.** We are not just missing tints; three things differ structurally:
1. **Black ink.** MadWorld's flat black fills (32% of the frame) have no counterpart there. Their darkest tone is a
   warm brown. Our multiply tint keeps black black, so a third of the screen can never take color.
2. **Hue family.** They use one warm analogous family plus a single sparse cool accent. We spread hues over
   jade/lilac/slate, which reads as "tinted", not "lit".
3. **Light.** Their frame has a sun direction: bright warm ground (p95 230), shaded walls, haze toward the sky, and
   orange skin against an indigo outfit for figure–ground contrast. Ours is flat ink with no light.

Mock: `evidence/native-color-grade-mock.png`. Top is the current pack; bottom is a warm gradient map (ink → #2b1d15,
mid #8a5f40, paper → #f0d6a8) with the pack's existing chroma boosted ×1.8, top-of-frame sun haze and a soft
vignette. Result: saturation 0.60, black 0%. Alone it reads as sepia: layers 1 and 3 below supply the material
variety and light.

## Plan (three layers, cheapest first)
1. **Warm re-palette, texture-only** (bake, no renderer change; about 1–2 h)
   - Scenery: switch from multiply to a gradient map so ink becomes a warm brown instead of black. Materials stay
     inside 10–45°: sandstone/dust ground (#4a3222 → #e9cfa3), brick/rust walls (#3a2318 → #c98d5e / #b07a4f), adobe
     (#4a3426 → #d8b088), weathered wood (#2e2016 → #8a6446), rust metal (#2a1a12 → #9a5a3a). Cool accent on about
     10–15% of wall/sign sheets only: desaturated indigo (#232233 → #6a6a8c) or faded teal (#1f2d2b → #5f8a80).
   - Characters: same lift (outlines a warm near-black #20150f, still read as ink). Skin warm tan/orange (#e0905a
     family). Give each outfit a saturated cool or deep color so figures separate from the warm streets.
   - HUD/FX: unchanged (red-neutral). Blood keeps its red through the red guard.
2. **Screen grade in `blit_native`** (Bevy post pass on the native output; C bypasses it; about 2 h)
   - A warm split-tone that lifts blacks to brown and warms highlights, saturation ×1.2–1.4, highlights pushed toward
     p95 ≈ 220, a soft highlight bloom, sun haze toward the top of frame, and a light vignette. Values as in the mock,
     but on top of layer 1, so it needs far less strength.
   - This is a screen-space filter by design (overall mood). Material separation stays in layer 1.
3. **Lighting** (TEV/lighting shaders; coordinate with the gpu-renderer session; about 1 day, deferred)
   - Warm directional sun plus a cooler bounce ambient in the GX lighting path, and a 2–3-step cel ramp for
     characters (Kiwami-style toon). This is the real source of the depth that is still missing; layers 1–2 only
     approximate it.

**Targets to check after each layer** (same script, run on an `MW_COLOR_CAPTURE` frame): mean saturation 0.35–0.45,
pure black under 2%, at least 80% of colored pixels in 0–60°, highlight p95 at least 215, blood pixels unchanged.

## Implemented (layers 1–2, no ink look)
Controls in `madworld-native`: **C** turns color on/off (off = original game), **F2** switches night/day
(default night: the boss-intro HUD shows a moon, so Asian Town is a night stage; `MW_TIME=day` starts in
day), **F3** switches to the legacy v1 pack (the earlier look: explorer palette over ink, no grade).

- **Layer 1, v2 pack** (`python tools/bake_native_color.py`, output `native-host-data/color-pack`): a gradient map
  instead of multiply, so ink becomes each material's dark tone and nothing stays pure black. Asian Town
  palette: plaster and teak walls (most common), jade, imperial gold, rare vermilion lacquer and indigo accents;
  stone flags; jade or grey clay roof tiles; orange lantern trims. Signs, billboards and posters get neon
  (magenta, cyan, gold, green). Characters: ink lifted to a dark version of their region tint. HUD/FX:
  red-neutral, unchanged. Blood stays red.
- **Legacy v1 pack** (`--legacy`, output `native-host-data/color-pack-legacy`): the earlier look, kept as-is for F3.
- **Layer 2, grade** (`crates/mw-game/src/native_grade.rs`, a post pass after the native blit):
  1. White-balances out MadWorld's built-in cream cast (measured: neutrals have blue at 0.886 of red), so
     the presets alone set the mood. This is what removed the "everything sepia" look.
  2. Per-channel lift/gain (texture hues survive; no single-ramp duotone), then saturation.
  3. Sky: pure black in the upper frame (background, since the bake lifts all ink) becomes a gradient:
     day blue #4f86c6 → horizon #bcd3e6, night #0b0d22 → violet #2a1638. A vertical average keeps the
     game's TV scanlines from striping it.
  4. Glow from bright texels (lanterns, neon, sun glare), top-of-frame haze, vignette.
  5. Saturated reds (blood, red UI) pass through unchanged.

  Night: indigo shade #121426, warm lantern light #f0e4d4, violet neon haze, glow 0.6. Day: brown-grey shade
  #24201c, neutral warm-white sun #fff6e6, soft sky haze, gain 1.15.
- **Explorer lab:** `http://127.0.0.1:8781/lab/grade.html` runs the same shader math in WebGL on a game frame, with
  night/day (F2) and graded/source (C) toggles and the reference metrics. Frames: a v2-pack boss-intro capture
  (`evidence/native-color-v2-frame.ppm`, taken under `.bevy-instance.lock`), the original game, and the old v1 frame.

  | v2 frame | saturation | pure black | warm share | highlight p95 |
  |---|---|---|---|---|
  | night | 0.36 | 3% | 42% (cool by design) | 193 |
  | day | 0.33 | 0% | 71% | 233 |

  Before these fixes (single-ramp grade plus the game's own cream cast) day measured 98% warm, which is the sepia
  the user called out. The lab is served by `python -m http.server 8790` from `exports/madworld` when the explorer
  server isn't running.

## Ink fills and black-and-white characters (fixed in the TEV shader)
- **Cause:** MadWorld's character ink programs (`white`/`black` profiles in the explorer's
  `evidence/material-programs.json`) lerp the base texture against a flat alpha fill. In the white-base program,
  stage 6 does `mix(texture.rgb, prev.a, prev.rgb)`; the black-base program does the mirror. Lit or shadowed parts
  (side views especially) therefore show a flat white or black regardless of texture color. Characters drawn
  mostly that way stayed black and white even though every live character base texture is in the pack. The
  compare stages read shadow/depth maps (texmaps 0/1), not the pack, so ink thresholds are unaffected.
- **Fix** (`tools/gecko-reference/crates/backend-wgpu/src/shaders/main.wesl`, `hue_fill`): in any non-compare stage
  that lerps `texture.rgb` against an alpha broadcast (prev/reg0/reg1/reg2 alpha), the flat fill takes the
  texture's hue at its brightness, lifted to at least 0.2 so black becomes a dark tone. It is gated by the texel's
  saturation, so the game's grey textures (C = source mode) are bit-identical. Shaders compile from source at
  startup; the caches only hold keys, so no cache bump was needed.
- **Verified live** (under `.bevy-instance.lock`, frames 1200 and 2400): shaders compile, and the balcony enemy
  (fire-breather) and Jack render in color where source mode shows them black and white.
- **Palettes:** the Asian Town street enemies em0100/em0101 get strong v2 outfits (teal, plum, gold accents), and
  other unreviewed enemies use a deeper fallback (HSL l 0.45, s 0.6 instead of the explorer's pastel 0.62/0.42).
  Legacy (F3) keeps the original palette.

## HUD coverage (staged PC UI) and background pack decoding
- The live game reads the PC-staged HUD, `staged/pc-ui/core_us.idd` (built by `tools/stage_pc_ui.py`), not the disc
  copy. `--fx-hud` now tints both, plus `staged/pc-ui/core2_us.dat` and the per-stage UI containers
  `case*/idpac_*_us.dat` (e.g. the c201 challenge-prompt pages), with the same red-neutral rule. Latest live log:
  94 of 121 textures in the pack. The rest are `_AB` ink masks, shadow/noise, framebuffer copies, strap images and
  a constant, all untouched by design.
- Replacements decode on 3 background threads; the guest texture shows until the colored one is ready. Only the
  active pack decodes (legacy only once F3 selects it). A/B under the lock (`MW_COLOR_PACK=0` starts in source mode):
  render-queue time and frame times are the same with the pack on and off
  (`evidence/native-color-ab-pack-{on,off}-perf.log`). The single 0.9 s first-frame stall is gone.
