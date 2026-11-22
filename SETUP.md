# Setup: from your own disc to a playable game

This repository has no game data. You build it from **your own copy of MadWorld (USA, RZZE8P rev 0)**.
The pipeline has four parts:

1. Extract the disc.
2. Build the **explorer** export, which decodes models, textures, motions and catalogs.
3. Run the **staging tools**, which write engine-ready files into `assets/native/` and `staged/`.
4. Build and run `mw-play`. The [recompiled core](recomp/README.md) is optional and built separately.

> **Paths are hardcoded.** The tools were written for one machine. Most scripts expect exactly:
> - explorer export: `D:/Games/gamesfordecomp/exports/madworld`
> - this repo: `D:/Games/gamesfordecomp/ports/madworld-rinrin`
> - cargo target: `D:/GDecomp/build-cache/bevy-0.20` (`.cargo/config.toml`, `play-vr.cmd`)
>
> **The simplest route is to use the same layout.** Otherwise, search and replace those strings in `tools/`
> (`grep -rn "exports/madworld" tools`). `MW_OWNED_PARTITION` overrides only the native frontend's partition path.

## 0. Prerequisites

| Tool | Used for |
|---|---|
| Rust stable (MSVC toolchain) | the game (`Cargo.lock` pins Bevy 0.20.0, wgpu 30.0.1) |
| Python 3.11+ with `Pillow`, `numpy`, `capstone`, `ninja` | explorer and staging tools (always run them with `python -I`) |
| Node.js 20+, `three` (r175), `@napi-rs/canvas` | GLB export (`explorer/review_export.mjs`, `tools/export_arena.mjs`) |
| [DolphinTool](https://dolphin-emu.org) | disc extraction |
| [vgmstream](https://github.com/vgmstream/vgmstream) r2168 win64 → `tools/vgmstream/` | audio decoding (RedSpark `.rsd`) |
| [soulstruct-havok](https://github.com/Grimrukh/soulstruct-havok) in a venv at `tools/havok-venv/` (clone into `tools/soulstruct-havok/`) | static collision (`tools/stage_collision.py`) |
| Blender 4.4 (`C:/Program Files/Blender Foundation/Blender 4.4/`) | only to rebuild Kreely or the vampire zako skinning |
| Visual Studio 2022 Build Tools | only for the recompiled core |
| Windows 10/11, a DX12 or Vulkan GPU | developed on an RTX 3090 |

## 1. Extract the disc

```powershell
DolphinTool.exe extract -i "<your MadWorld (USA).rvz or .iso>" -o D:/Games/gamesfordecomp/exports/madworld/extracted/disc -g -q
```

You should get `extracted/disc/DATA/sys/main.dol` and `extracted/disc/DATA/files/{em,pl,bl,it,wp,etc,case2,…}`.
A good rev-0 RVZ has SHA-256 `cc78c7cc20f004f76d2d560f1b307e95f1ba2717aaec4577a69001395d304d17`.

## 2. Build the explorer export

`explorer/` holds the MadWorld builder from the game-reconstruction-kit (`case-studies/madworld`), with its `gk` graph package.
Copy it in as the export's `builders/` folder, then build:

```powershell
Copy-Item -Recurse explorer D:/Games/gamesfordecomp/exports/madworld/builders
```
```powershell
python D:/Games/gamesfordecomp/exports/madworld/builders/build.py D:/Games/gamesfordecomp/exports/madworld
```
```powershell
python D:/Games/gamesfordecomp/exports/madworld/builders/validate.py D:/Games/gamesfordecomp/exports/madworld
```
```powershell
python D:/Games/gamesfordecomp/exports/madworld/builders/native_graph.py D:/Games/gamesfordecomp/exports/madworld
```

Copy `app.js`, `index.html`, `export.js`, `color.js` and `ink.js` from `explorer/` to the export root, and put
three.js r175 under the export's `vendor/`. Then **leave the viewer server running**, because the GLB exporter fetches from it:

```powershell
python D:/Games/gamesfordecomp/exports/madworld/builders/serve.py D:/Games/gamesfordecomp/exports/madworld --port 8781
```

Install the exporter's Node dependencies in `<export>/tools/node-review/` (`npm i three@0.175.0 @napi-rs/canvas`).

## 3. Stage the assets

Run every command from the repo root. Each actor needs its GLBs exported first. The export writes
`<export>/evidence/node-export-<stem>[-source].json`:

```powershell
node explorer/review_export.mjs D:/Games/gamesfordecomp/exports/madworld <stem>
```

### 3a. Required for the default RinRin fight

| Step | Command | Writes |
|---|---|---|
| Enemy module image | `python -I tools/prepare_rin_native.py` | `tools/enemy-analysis.bin` |
| RinRin | export `em0440`, then `python -I tools/stage_rinrin.py` | `rinrin-{source,color}.glb`, `rinrin-motion.json` |
| RinRin extras | `python -I tools/stage_rin_jiggle.py` and `python -I tools/stage_rin_scissors.py` | spring bones, alternate finisher clips |
| Jack (player) | export `pl0030`, `pl0000`, then `python -I tools/stage_actor.py pl0030 jack-player` and `python -I tools/stage_actor.py pl0000 jack` | `jack-player-*`, `jack-*` |
| Grunts | export `em0101`, then `python -I tools/stage_actor.py em0101 kungfu` and `python -I tools/stage_kungfu_shared.py` | `kungfu-*` |
| Arena (c201) | `python -I tools/stage_arena.py`, `node tools/export_arena.mjs`, `python -I tools/stage_arena_color.py` | `assets/native/world/`, `rooms/` |
| Collision | `tools/havok-venv/Scripts/python.exe -I tools/stage_collision.py` | `assets/native/collision/` |
| Pair cameras | `python -I tools/stage_camera_clips.py em/em0440.dat assets/native/rinrin-camera.json` and `python -I tools/stage_room_cameras.py` | camera JSONs |
| Combat data | `python -I tools/pl0000/stage_combat.py` | `assets/native/combat.json` |
| Effects, shadow, ink | `python -I tools/stage_effects.py`, `tools/stage_shadow.py`, `tools/stage_ink.py` | `effects/`, `shadow/`, `ink/` |
| Audio | `python -I tools/audio/stage_bgm.py`, `tools/audio/stage_se.py`, `tools/audio/stage_announce.py`, `tools/stage_sounds.py` | `assets/native/audio/` |
| Intro event (ev21) | `python -I tools/events/stage_intro.py`, then `stage_audio.py`, `stage_ui.py`, `stage_props.py`, `stage_messages.py`, `stage_effects.py`, `stage_writing.py` (all in `tools/events/`, `MW_EVENT` defaults to ev21) | `assets/native/events/ev21/` |
| UI | `python -I tools/ui/stage_native_ui.py`, `stage_challenge.py`, `stage_playlog.py`, `stage_pause_map.py`, `stage_save_defaults.py`, `stage_subtitle_font.py`, plus `python -I tools/stage_gun_ui.py` | `assets/native/ui/`, `save/` |
| Colour mode | `python tools/bake_native_color.py`, then `python -I tools/stage_character_color.py` | `native-host-data/color-pack`, coloured GLBs |

### 3b. Optional, per stage

- **elise** (c303): export and stage `em0630 elise`, `em0330 bat` and `em0331 batmother`. Then:
  `MW_ROOM=c303 MW_SPAWNSET=c30300 python -I tools/stage_arena.py` plus `export_arena.mjs` and `stage_arena_color.py`,
  `python -I tools/stage_camera_clips.py em/em0630.dat assets/native/elise-camera.json`, and
  `python -I tools/stage_static_object.py bl/bl005c.dat` plus `stage_object_color.py` (and the same for `bl005d`) for the breakables.
  Run the events with `MW_EVENT=ev33` / `ev34` / `ev84` through the `tools/events/` scripts. The ending backdrop needs
  `MW_ROOM=ca03 MW_SPAWNSET=none python -I tools/stage_arena.py`. Run `tools/ui/stage_pause_map.py c303`.
- **baron / mathilda** (c503): stage `em0623 baron`, `em0610 mathilda --bank em0615` and `em0960 oiran --bank em0965`, plus `bl0331 shades`.
  Then `python -I tools/stage_boss_skin.py`, the c503 arena, the camera clips from `em/em0623.dat`, `tools/ui/stage_native_ui.py`
  (the c503 pack) and events `ev50` / `ev51` / `ev93`.
- **street**: `python -I tools/stage_street.py`, the props via `tools/stage_static_object.py`, and the effect props
  `it0501`, `it0502`, `bl0330 hat`, `bl0522 heli` and `wp0582 mbat` through `stage_actor.py`. Also `python -I tools/stage_wound_decals.py`.
- **town**: `python -I tools/town/stage_civilians.py`, `tools/town/stage_civ_motion.py` and `tools/town/stage_nav.py` (needs the collision step first).
- **Lit looks (F3)**: `python -I tools/stage_sign_lights.py`.
- **Vampire zako**: already in the repo (see 3c).

### 3c. Already in the repo: colour, Mathilda, civilians, Kreely, vampire zako

These steps from 3a/3b can be skipped because their outputs are committed:
- **Colour:** `bake_native_color.py`, `stage_character_color.py`, `stage_arena_color.py`, `stage_object_color.py`,
  `stage_boss_skin.py`. Every `*-color.glb` and colour texture under `assets/native/` and the pack in `native-host-data/color-pack/`
  are committed. The superseded `color-pack-legacy` is not.
- **Mathilda:** `stage_actor.py em0610 mathilda --bank em0615`, her camera clips and the ev50/ev51 `em0610` models.
- **Town civilians:** `tools/town/stage_civilians.py` and `stage_civ_motion.py` (`assets/native/civ/`) and the ev84 crowd models.
  The walk grid (`assets/native/world/town-nav.json`, from `tools/town/stage_nav.py`) is committed too.

Their non-colour partners (for example `rinrin-source.glb` and the motion files) still come from staging.

**The vampire zako** (Elise's brood, the owner's own models) are committed and need no staging:
- `assets/characters/vampzako/source/`: the three original meshes (`vampirezakoN/vampirezakoN.fbx` and `.fbm` textures)
  and the concept sheets (`vampire_purple_*.png`).
- `assets/characters/vampzako/build/`: build reports and source and colour textures.
- `assets/native/vampzako{1,2,3}-{source,color}.glb` and `vampzakoN-rig.json`. The actor definition is `assets/defs/actors/vampzako.json`.

To rebuild them: `python -I tools/vampzako/build_vampzako.py 1 2 3`. This needs Blender 4.4 and the staged grunt
(`em0101`, step 3a), because they ride his skeleton and clips. `tools/vampzako/view_weights.py` and `view_face.py` render checks.

**Kreese Kreely**, the custom player character, is committed and needs no staging:
- `assets/characters/kreely/`: source FBX and textures, the build model and textures, skinning work files.
- `assets/native/kreely-{source,color}.glb`, `kreely-rig.json`, `kreely-moves.json`, `kreely-gun-motion.json`, `kreely-golden-poses.json`.
- `assets/native/audio/kreely/`: 40 voice lines (`kreely.json`), with review takes.

To rebuild him: `python -I tools/kreely/build_kreely.py` (needs Blender 4.4 and Jack's staged rig), then
`tools/kreely/stage_kreely_native_clips.py`. The voice bank is rendered with a local VoiceStudio profile
(`tools/audio/stage_kreely_voice.py`, see its docstring); that tool is not part of this repo. `kreely-moves.json`
holds clips retargeted from a God Hand export (`tools/godhand/retarget_godhand.py`), which needs your own God Hand extraction to rebuild.

## 4. Build and play

```powershell
cargo run --release -p mw-game --bin mw-play
```

The stage select opens: Up/Down or 1–6, then Enter. Or skip it:

```powershell
$env:MW_STAGE="elise"; cargo run --release -p mw-game --bin mw-play
```

**VR:** start Quest Link or Air Link (or any OpenXR runtime), then run `.\play-vr.cmd [stage]`.
`$env:MW_VR="sim"` gives a desktop VR simulator.

Settings: Escape opens the pause menu (music and commentary toggles). C switches colour/monochrome and F3 cycles the looks.
Bindings and display settings are in `pc-controls.toml` next to the exe. Controls are listed in the [README](README.md#controls).

### If something is missing

- `mw-play` logs the asset path it failed to open. Find the tool that writes that file (`grep -rln "<file name>" tools`) and run it.
- `MW_DIRECT_FIGHT=1` skips the intro if the event assets are not staged yet. `MW_CHALLENGE=0` skips the challenge prompt.
- Run only **one** Bevy instance at a time.
- The full list of `MW_*` switches is in [docs/WORKLOG.md](docs/WORKLOG.md).

## Caveats

- This order was rebuilt from the tool sources after the fact. It has not been tested from a clean machine, so a step
  may be missing or need a different order. Fix this file when you find one.
- Two things are not on the disc and not in the repo: the God Hand source for Kreely's clips and Kreely's voice
  tooling. Their outputs are committed, so play does not need them.
- `research/` holds only the written `.md` notes. The generated graph, JSONL and compare files are rebuilt by the `tools/index_*.py`
  scripts. They are needed for analysis, not for play.
