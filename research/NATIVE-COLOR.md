# Native color pack (madworld-native)

> Current look: v2 pack + night/day grade (C color on/off, F2 night/day, F3 legacy v1). See `research/LOST-PARADISE-LOOK.md`. The sections below describe the v1 pipeline, which `--legacy` still produces.

Added inspection color, not recovered artwork. It brings the explorer's palette
(`exports/madworld/color.js`) to the native GX renderer.

## How it works
- The explorer tints each mesh by per-vertex region colors: a per-model palette for
  em0440/0610/0630/0920/0925/0930/0935/0960/pl, a hashed fallback for every other model,
  regions chosen by mesh-name regex plus a bind-space height split. The tint is multiplied
  over the source texture.
- `tools/bake_native_color.py` rasterizes each mesh's tint into its texture's UV space
  (averaged where meshes overlap, then dilated into the gutters), multiplies it over the decoded
  TPL texture and keeps alpha. Skeleton/x-ray parts (`*_HN_*` meshes) share UV space with the
  visible body, so they only fill texels that no visible mesh covers. Strongly red texels (`r - max(g,b) > 60`) are left
  unchanged, so blood and red accents stay as they are. The output is a Dolphin-format pack in
  `native-host-data/color-pack/RZZE8P`, named by the renderer's xxhash64 key, with
  both the plain and `_m` mip names. `index.json` maps each name to its bank, TPL and models.
- Renderer (`tools/gecko-reference/crates/backend-wgpu`): `action.rs` LoadTexture now
  keeps the guest decode next to every pack replacement (`PackVariant`).
  `GxRenderer::sync_pack_toggle` swaps the shown view live, so no guest reload is needed.
  `Renderer::set_texture_pack_enabled` sets the toggle. `texture_pack.rs` writes
  `color-pack/live-textures-<pid>.log`, which records each live texture once as hit or miss.
- Frontend (`crates/mw-game/src/native_main.rs`): `blit_native` loads the pack, and **C**
  switches between color (the default) and source monochrome. C was unused in `native_controls.rs`.
  F1 help does not list C yet, because `HELP` lives in another session's file.
  `MW_COLOR_CAPTURE=<dir>` (plus `MW_COLOR_CAPTURE_FRAME`, default 2400) writes
  `color.ppm`/`source.ppm` as an XFB A/B check. Evidence: `evidence/native-color-toggle.png`
  (top: color, bottom: source, boss-intro screen).

This is per-texture recoloring at upload time, not a screen-space filter.
The GX/TEV ink programs pass the tinted texture color through: the green turtle and the
tinted props show up in the XFB capture.

## HUD and effects (`--fx-hud`)
HUD pages (`etc/core_us.idd`) and effect sheets (`etc/core2*.dat` effect banks, stage `.eff`/`.idd` members) are
white masks that the game colors per draw (konst/vertex color × texture). That is also how blood and the red UI
get their red. The pass multiplies each mask by a hue whose **red channel is exactly 1.0**, so:
- a pure-red draw (blood) is bit-identical: red × 1 = red, and green/blue were already 0;
- white and grey draws take the hue: gold, amber, cream, peach or soft rose for the HUD (one per sprite group from
  `research/ui-live-core-layout.json`), and gold, amber, peach or warm cream for effects (one per sheet);
- textures that already have color (logos, icons), `_AB` ink masks, and the shadow/noise sheets are left as they are.

This is texture-only; there is no renderer change. 199 textures, indexed in `fxhud-index.json`.

**Explorer test:** `exports/madworld/lab/fxhud.html`, served by the explorer at `http://127.0.0.1:8781/lab/fxhud.html`.
It draws HUD pages and FX sheets with a GX-style modulate shader (8-bit `konst × texture`) under several konst
colors, with a source/color toggle (C) and a per-konst numeric check (maximum 8-bit output change, color vs source):

| konst | HUD page 1 | FX sheet 0 |
|---|---|---|
| blood red (.75, 0, 0) | **0** | **0** |
| UI red (.9, .1, .1) | 17 | 12 (only in its small green/blue parts) |
| white / grey .5 | 164 / 82 | 117 / 59 (these take the hue) |

Not verified: that the game makes blood red by *multiplying* (konst/vertex × mask). If some blood draws are white with
red added another way, they would come out peach. A short in-game look at a kill would settle it.

## Check against the explorer (no game launch)
The explorer's own `color.js` `colorize()` runs in node, using its `tools/node-review` three loader, on
the explorer's `mdb()` output. At each triangle's UV centroid, the expected value is source texel ×
`colorize` vertex color, which is what the explorer renders. That is compared with the baked pack texel
(max channel error on a 0–255 scale):

| Model | Triangles | Median error | Within 16 |
|---|---|---|---|
| bl0003 (live prop) | 336 | 0.2 | 100% |
| bl0182 (live prop) | 542 | 0.3 | 98% |
| em0101 | 2384 | 0.4 | 100% |
| em0440 RinRin (visible meshes) | 5236 | 0.0 | 94.5% (was ~73% before the skeleton-part fix) |
| pl0030 Jack | 3052 | 0.3 | 92% |

The remaining mismatches are texels shared by several meshes with different tints (mirrored or overlapping
UVs). A single texture cannot reproduce per-vertex colors there.

## Stage scenery (plan step 2)
SCR scenery mesh names are generic (`polymsh`, `cube3329`), and each texture is a tiling sheet shared by up to
32 meshes, so the explorer's name-based regions cannot apply. `--scenery` colors each SCR texture by how the
world uses it instead:
- **Ground:** mostly upward-facing and low (normal y > 0.7, in the lowest 20% of heights + 1). Warm flagstone.
- **Roof:** upward-facing and high. Jade tile.
- **Walls/signs:** vertical. One of six muted hues per texture: ochre, jade, lilac, slate, sand, wood.
- **Alpha cut-outs (IA4/IA8):** gold accent.

Each texel is multiplied by a shadow-to-highlight gradient of its hue, the same texture × tint rule the
characters use, so ink stays black and paper takes the hue. There are no blood-like reds, and source red
texels pass through. 1088 SCR textures from every `case*/*.dat` stage give 1018 unique names. Textures that the
character/prop pass also produces keep the explorer palette. The index is `scenery-index.json`, and the before/after
sheet for `c201` is `evidence/native-color-scenery-sheet.png`.

The same pass also tints TPLs embedded in stage-bank members (`.tpl`/`.eff`/`.imt`/`.mtd`, never `_AB` or
`.idd`), but only ink-style art: at least 15% near-black and 15% near-white opaque texels. That takes in signs,
plaques, billboard/TV animation frames, WANTED posters and map pieces (140 textures), and skips soft particles and
white silhouette masks. Hue is fixed per bank member, so animation frames don't flicker.

## Coverage
- Baked: 920 texture images from 714 catalog models (bl/em/wp/cs/it/pl/caseF). 5 models
  fail the explorer's MDB parser (bl00ff, three caseF sky models, cf04 cube) and get no color.
- Live, on the boss-intro screen after about 90 s (`live-textures-60780.log`): 107 distinct textures,
  **36 matched**, 71 unmatched. Every live catalog base texture matched.
  - 16 are `_AB` ink-threshold textures. They are left unchanged on purpose (they are masks, not color).
  - 2 are `etc/core2_us.dat` jkShadow/jkNoise (effect inputs).
  - 53 are not in any catalog bank: SCR stage scenery, UI CI4/CI8 atlases, and
    853x480/640x448 EFB re-reads (these cannot be replaced; they inherit scene color).
- With the HUD/FX pass on top, the boss intro is at **78 of 107** (the 6 core2 effect sheets/wipe are now in).
- After the scenery and embedded-art passes, the boss intro was at **72 of 107** (checked offline against that log;
  no game run, per the one-instance rule). Left as they are: 16 ink masks; 7 effect sheets/inputs (blood, sparks,
  smoke, wipes) that the game colors per draw; the HUD/fonts (`core_us.idd`), which are white masks colored per draw;
  4 pages with no file on disc; framebuffer copies, strap images and constants.
- Before the scenery pass, characters and catalog props were in color while stage geometry, HUD and fonts were
  source monochrome. The explorer never had a palette for those. A one-day plan to cover them is in
  `research/NATIVE-COLOR-PLAN.md`.

## Rebuild
```
python tools/bake_native_color.py      # every pass; or --models (~20 min), --scenery (~3 min), --fx-hud (~2 min)
CARGO_TARGET_DIR=D:/GDecomp/build-cache/bevy-0.20 cargo build --locked -p mw-game --features native-runtime --bin madworld-native
```
The pack and its logs are derived from the owned disc and stay local.
