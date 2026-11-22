# madworldsource: MadWorld native Rust port

A native PC reconstruction of **MadWorld** (Wii, disc RZZE8P rev 0), written in Rust on the **Bevy 0.20** engine.
It replaces the emulated Wii hardware with SDK-level high-level emulation (HLE) and reimplements game logic in Rust.
It started from the RinRin boss fight and grew into a multi-stage game with optional VR.

> **No original game data is in this repository.** Everything under `assets/native/`, `extracted/`, `staged/` and the
> research dumps comes from your own disc through the staging tools in `tools/`. The only committed assets are the
> owner's own: **Kreese Kreely** (`assets/characters/kreely`, `assets/native/kreely-*`, `assets/native/audio/kreely`),
> **the three vampire zako** (`assets/characters/vampzako`, `assets/native/vampzako*`) and the port's actor
> definitions (`assets/defs`).

**Start here:**
- **[SETUP.md](SETUP.md)**: from your disc to a playable `mw-play`. Covers disc extraction, the explorer export, every staging step, then build and run.
- **[recomp/README.md](recomp/README.md)**: the recompiled original core (`madworld-native`). Covers the upstream version
  ([JaredMoss1996/MadWorldDecomp](https://github.com/JaredMoss1996/MadWorldDecomp) @ `6e02a76`), our overlay of
  changes, and the MSVC build.

Status: work in progress. It plays, but full fidelity, the HUD, menus and save flow are still being ported
(see `research/NATIVE-PORT-GAPS.md`).

---

## How it was built

1. **Extraction.** The original disc was extracted from an owned copy (DOL, RSO modules, `.dat` packs, audio banks).
2. **Reverse engineering.** Ghidra (`tools/ghidra-scripts/`, `tools/ghidra-functions.tsv`) decompiled the game code.
   Each subsystem's logic was recorded as typed graphs with indexes, vocabulary and pseudocode (`research/*.md`).
   Every fact is tagged native, inferred, or `[added]`.
3. **Asset staging.** Python tools (`tools/stage_*.py`, `tools/audio/`, `tools/events/`, `tools/ui/` …) decode the
   original formats into engine-ready files: rigs and motions, collision (Havok via a local adapter), EST effect
   emitters, SE banks with their original sample rates and tunings, IDT/IDD UI widgets, event scripts, camera clips,
   fonts and messages.
4. **Two runtimes, compared against each other.**
   - `madworld-native` (`native-platform/`, feature `native-runtime`) hosts a statically recompiled original CPU
     core. Rust/Bevy supplies rendering, PCM/AX audio, remote-speaker audio and PC input. This is the reference.
   - `mw-play` is the **pure Rust** port. Game logic is translated function by function (native addresses are
     cited in comments, e.g. `fn_802A03A0`).
5. **Frame-by-frame verification.** Fixtures run the same scenario in the recomp and in the port and compare state
   per update (`tools/run_owned_gpu_test.py`, `tools/compare_*.py`, `FRAME-COMPARE-*` reports). One example: all 623 updates of
   Elise's coffin clash match the original within 1.1 cm, and her pair cameras match to millimetres. Open verification
   obligations are tracked in `ledger.json` (report: `notes/LEDGER.md`).
6. **Additions.** Some features are new, marked `[added]` in code and notes: Kreese Kreely as a player, the town
   free roam, the vampire zako, VR, the actor editor, modern looks, run-and-gun and others.

The full engineering log (every subsystem, env var and verification) is in **[docs/WORKLOG.md](docs/WORKLOG.md)**.
VR design notes are in [notes/VR.md](notes/VR.md).

## Repository layout

| Path | What |
|---|---|
| `crates/mw-data` | Native data formats and loaders |
| `crates/mw-sim` | Sampled rig and motion evaluation |
| `crates/mw-game` | Bevy game: `mw-play` (the port), `madworld-native` (recomp host), previews; `play_*.rs` gameplay modules |
| `crates/vendor` | Vendored crates (bevy_mod_openxr/xr ported to Bevy 0.20, openxr, …) |
| `native-platform` | HLE platform layer for the recompiled core (GX render, AX/DSP audio, input) |
| `recomp/` | Our changes to the upstream recompiled core (`overlay/`, `upstream-modified.patch`) and its build guide |
| `explorer/` | The MadWorld explorer builder (disc decoder, GLB exporter, viewer) that the staging tools read from |
| `assets/characters/kreely`, `assets/native/kreely-*` | Kreese Kreely: source model, build, rig, moves, voice bank |
| `assets/characters/vampzako`, `assets/native/vampzako*` | The vampire zako: source meshes, concept sheets, builds, rigs |
| `tools/` | Extraction, staging, indexing, comparison, VR test runtime (`tools/vr/xrsim`), Ghidra scripts |
| `research/*.md` | Reverse-engineering write-ups per subsystem (boss behavior, audio, collision, events, UI, controls) |
| `notes/` | VR notes, ledger report, mod log |
| `assets/defs/actors` | Actor definitions (HP, attacks, voices, editor pages). Data-driven tuning lives here |

## Features

**Stages** (stage select on launch, or `MW_STAGE=<name>`):
- `rinrin`: the RinRin boss fight. Original intro event, the TAKE THE CHALLENGE? prompt, clash and finisher, BS04/BS04C music.
- `elise`: Elise in the church (c303). Her bats, the summoned zako, the coffin clash, crypt tiers you jump up, breakable nave pillars and pews, the ev84 ending.
- `baron`: the Black Baron with Mathilda and the oirans. Clash chain on the original pair cameras, the ev51 and ev93 events.
- `mathilda` [added]: Mathilda as the boss. Bat combos, the pin struggle, subtitled intro.
- `street`: Asia Town grunts. Pick up and throw props, impale on the spike wall and meat hooks, chainsaw splits (vertical and waist).
- `town` [added]: daytime free roam. 26 townsfolk on an A* walk grid, speech bubbles, substory fights.

**Combat and presentation:**
- Original follow camera, lock-on and target stepping, plus an optional free camera.
- Clash cut-ins (comic panels), the cockpit swing navigation, and pair cameras from the original banks.
- Original EST hit and blood effects. Added gore: blood pools, stains, bite marks, wounds.
- Last-life executions for every boss, slow-motion kills, the continue screen.
- Kreese Kreely as the default player [added]: grab and carry, suplex, head-butt, taunt that aggroes grunts, voiced lines.
- Pistol with real ballistic rounds (gravity, ricochets, fan deflection), tracers, ejected brass, run-and-gun or aim-walk.
- Springs for cloth, hair and breasts on bosses. Original voice routing and announcer commentary (toggleable).
- Original HUD health bars, pause pages, hold-key panel, original fonts and messages.
- Added colour mode (as the recomp) or original monochrome. F3 cycles Native / Modern / Filmic looks.
- Actor editor: plays any actor's clips, attacks, voices and cameras with orbit view and slow motion.
- Save/load snapshots, rebindable PC controls and gamepad support (`pc-controls.toml`).

**VR** [added]: Quest 3 over Link or Air Link (any OpenXR runtime). Always first person, with Kreely's own arms on the controllers.
You punch by swing speed, grab and throw grunts and objects, climb ledges, and handle a realistic pistol (draw, mag drop,
rack the slide, iron sights). `MW_VR=sim` gives a desktop simulator, and `tools/vr/xrsim` is a headless OpenXR test runtime.

## Controls

### Keyboard and mouse (`mw-play`)

| Input | Action |
|---|---|
| W A S D | Move (Ctrl: walk) |
| Left mouse / E | Attack. Hold: grab (grunt or prop) |
| Right mouse (hold) | Chainsaw / pistol aim (Kreely: mouse steers aim) |
| Middle mouse / Q | Horizontal swing |
| F | Grab / throw while carrying |
| X (hold) | Shake: struggles, head-butt while holding a grunt |
| A / D (mash) | Fill shake meters (dizzy, clashes, boss struggles) |
| Space | Jump |
| Shift | Dodge |
| R | Reload |
| T | Taunt |
| Tab | Tap: recenter camera or unlock. Hold: lock on |
| Z | Toggle lock-on |
| Q / E (while locked) | Step lock to next target left / right |
| Enter | Confirm / continue. Hold 1.2 s: skip intro |
| Backspace / right mouse | Skip intro, menu back |
| Escape | Pause menu (M music, C commentary toggles) |
| C | Toggle colour / original monochrome |
| F3 | Cycle look (Native, Modern, Filmic) |
| F4 | Toggle free camera (mouse look) |
| F6 | Pistol: run-and-gun or aim-walk |
| F9 | Back to stage select |
| Insert | Swap player: Kreely or Jack |
| F2 | Actor editor: hand keys back to the player |
| Stage select | Up / Down or 1-6, Enter |

### Native frontend (`madworld-native`) and gamepad

Defaults are in `crates/mw-game/src/pc_config.rs` and can be rebound in `pc-controls.toml`. F1 shows the full panel,
F10 opens the settings menu.
- **Keyboard:** WASD move, LMB/E attack, RMB chainsaw, Space jump, Shift dodge, Q/MMB lock, X shake, G pick up,
  T taunt, 1/2 weapons, F/R or the arrow keys for right-arm Wii gestures, I J K L U O H Y for left-arm (Nunchuk) gestures,
  Enter confirm, Backspace back, Escape pause, Tab Wii Plus.
- **Xbox pad:** left stick moves, right stick is the camera. A confirm, B back, X jump, Y dodge, RT chainsaw,
  LT/R3 lock, L3 or LB+RB shake, d-pad pick up / taunt / weapons. **Hold RB (right arm) or LB (left arm)** and flick or
  rotate the right stick to perform the Wii remote/Nunchuk gestures. Start pauses, Select+Start opens settings.

### VR (Touch controllers)

| | Left | Right |
|---|---|---|
| Stick | Move (full tilt: run) | Snap / smooth turn |
| Grip | Grab / climb / hold. At hip: magazine. On slide: rack | Grab / climb / hold. At hip: draw pistol |
| Trigger | Index finger | Fire |
| A / X | X: reload (simple pistol) | A: jump / confirm. With pistol: drop magazine |
| B / Y | Y: taunt | B: dodge / back. With pistol: slide release |
| Menu | VR menu (settings, controls, recenter, stage select) | |

Swing to punch (speed sets the power), shake the controllers for mash meters, and make big down or sideways swings for clash strikes.
Desktop simulator keys: see `notes/VR.md`.

## Building and running

Requirements: Rust (stable), Windows with DX12 or Vulkan, Python 3 and Node for staging, your own extracted disc.
Follow **[SETUP.md](SETUP.md)** to stage the game files first.

```powershell
# after staging:
cargo run --release -p mw-game --bin mw-play
```

```powershell
# a specific stage, skipping the select
$env:MW_STAGE="elise"; cargo run --release -p mw-game --bin mw-play
```

```powershell
# VR (Quest 3 over Link / Air Link)
.\play-vr.cmd street
```

```powershell
# recompiled-core reference frontend (build madworld_core.dll first: recomp/README.md)
cargo build --locked -p mw-game --features native-runtime --bin madworld-native
```

Run only one Bevy instance at a time. Many `MW_*` environment variables expose inspection and test paths;
they are documented in [docs/WORKLOG.md](docs/WORKLOG.md).

## Legal

This is a private preservation and research project. MadWorld is © SEGA / PlatinumGames. No original game code
binaries, assets, audio or disc images are included. You need your own legally obtained copy.
