# The recompiled original core (`madworld-native`)

`madworld-native` is the **reference frontend**. It runs the game's own PowerPC code, statically recompiled to C++.
Rust and Bevy host the hardware around it: GX rendering, AX/DSP and remote-speaker audio, Wii remote input
(mapped from PC keys and pads), saves, and the PC prompt overlay. The pure-Rust `mw-play` port is checked against
it frame by frame.

## Version

| | |
|---|---|
| Upstream | [JaredMoss1996/MadWorldDecomp](https://github.com/JaredMoss1996/MadWorldDecomp) (public). It is a decomp-toolkit split of `main.dol` plus a static recompiler in `port/` |
| Base commit | `6e02a76a31dcd8c3147c92b223841bf52a4e7b23`: "first commit, phase 2 done" (2026-10-26). Upstream milestones 1–2 are done: the recompiler and booting to `main()` |
| Our changes | 22 modified upstream files (`upstream-modified.patch`, +747/−124) and 63 new files. All 85 are under `overlay/` at their upstream paths |
| Disc | MadWorld USA, **RZZE8P revision 0** (RVZ SHA-256 `cc78c7cc20f004f76d2d560f1b307e95f1ba2717aaec4577a69001395d304d17`) |
| Core ABI | checked at load (`mw_native_core_abi` in `crates/mw-game/src/native_core.rs`). Rebuild the DLL when the Rust side changes it |

What we added on top of upstream (in `overlay/port/`):
- `runtime/gx/`: a GX FIFO consumer and device. The Rust host draws the command stream (`native-platform/`).
- `runtime/hw/ai.cpp`, `dsp.cpp`, `ai_clock.h`: AI/DSP audio with an AX HLE path, plus the audio clock.
- `runtime/ios/bluetooth.cpp`, `platform_bluetooth.cpp`, `hci.h`: an emulated Bluetooth HCI with a Wii remote and Nunchuk, fed by
  host input. Also `input_file.h` for scripted input and `pc_ui_overlay.h` for the PC button prompts.
- `runtime/cpu/rso.cpp` plus `recomp/rso_recomp.py`: loads, relocates and recompiles `enemyRSO.rso`, so the enemies run.
- `runtime/cpu/embed_host.*`, `app/embedded.cpp`: builds the core as **`madworld_core.dll`** for the Rust host to load.
- `runtime/cpu/snapshot.*`: save-state snapshots. `runtime/os/host_fiber.*` and `fiber_x64.asm`: guest threads on host fibers.
- `runtime/game/`: PC camera, frame pacing, death cinematic, input HLE, and trace hooks (effects, sound, objects, boot).
- `tests/`: unit and difftests for each subsystem.

## Build it

Requirements: Windows x64, **Visual Studio 2022 Build Tools** (MSVC `cl` and `ml64`, static CRT), Python 3, `ninja`
(`pip install ninja`), and the extracted disc (see [../SETUP.md](../SETUP.md) step 1). The core must be built with
MSVC; clang/gcc are refused for the owned-stack runtime.

1. **Check out upstream at the base commit**, inside this repo at the path the Rust host expects:
   ```powershell
   git clone https://github.com/JaredMoss1996/MadWorldDecomp tools/upstream-native-core
   ```
   ```powershell
   git -C tools/upstream-native-core checkout 6e02a76a31dcd8c3147c92b223841bf52a4e7b23
   ```
2. **Lay our changes over it.** The overlay already contains the modified files, so the patch is for reference only:
   ```powershell
   Copy-Item -Recurse -Force recomp/overlay/* tools/upstream-native-core/
   ```
3. **Upstream config.** Put your disc in `tools/upstream-native-core/orig/RZZE8P/`, then run `python configure.py` and
   `python -m ninja` there. The build should print `build/RZZE8P/main.dol: OK`, which confirms `config/RZZE8P/symbols.txt` matches your disc.
4. **Link the enemy module for analysis.** This writes `tools/enemy-analysis.bin`:
   ```powershell
   python -I tools/prepare_rin_native.py
   ```
   It reads the explorer export (`D:/Games/gamesfordecomp/exports/madworld`: `extracted/disc/DATA/files/dll/Rel/enemyRSO.rso`,
   `evidence/resident-symbols.json`, `tools/upstream-research/symbols.txt`, which is upstream's `symbols.txt`).
5. **Recompile the enemy RSO** (once; `build.py` regenerates it when its inputs change):
   ```powershell
   python tools/upstream-native-core/port/recomp/rso_recomp.py --export D:/Games/gamesfordecomp/exports/madworld --analysis tools/enemy-analysis.bin
   ```
6. **Build the core.** `tools/build_native_msvc.cmd` opens a VS dev shell and runs
   `python port/build.py --jobs 4 --cxx <cl.exe>`. Edit the MSVC version path in it (`14.39.33519`) to match your
   install. The first build recompiles all of `main.dol` (`--dol` defaults to `orig/RZZE8P/sys/main.dol`, or use the
   extracted `DATA/sys/main.dol`). The result is `tools/upstream-native-core/port/build/madworld_core.dll` (~70 MB).
7. **Build and run the Rust frontend:**
   ```powershell
   cargo build --release --locked -p mw-game --features native-runtime --bin madworld-native
   ```
   ```powershell
   $env:MW_OWNED_PARTITION="D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA"; & "D:/GDecomp/build-cache/bevy-0.20/release/madworld-native.exe"
   ```

## Run-time switches

| Variable | Effect |
|---|---|
| `MW_OWNED_PARTITION` | Extracted disc `DATA` partition. Default: `D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA` |
| `MW_CORE_DLL` | Load another core build |
| `MW_NATIVE_ROOM=0x303` | Boot straight into a room (with fixtures such as `MW_TEST_ELISE_CLASH=1`) |
| `MW_NATIVE_DIAGNOSTICS=1` | Log original event and actor observations |
| `MW_TRACE_EFFECTS` | Trace effect triggers (`tools/play_traced_effects.cmd`) |
| `MW_TIMEOUT=<s>` | Dump the guest call stack if the game hangs (standalone exe) |

Saves and NAND live in `native-core-user/`, which is not committed.

## Controls

See the main [README](../README.md#native-frontend-madworld-native-and-gamepad). F1 in game lists every binding and
F10 opens the settings. Bindings are saved in `pc-controls.toml` next to the exe (or at `MW_PC_CONTROLS=<path>`).

## Status

The core boots, renders, plays audio and takes PC input. It has been checked headless on an RTX 3090 (DX12), and the
native RinRin clash completes under PC input. Full-speed play, every prompt replacement and audio-mix fidelity are
not fully verified. See `research/PC-CONTROLS.md`, `research/UI-PC-PROMPTS.md`, `research/REMOTE-SPEAKER-INDEX.md` and `ledger.json`.
