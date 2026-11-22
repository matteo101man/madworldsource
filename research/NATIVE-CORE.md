# Native RinRin port: current evidence

The Rust/Bevy madworld-native frontend hosts the original statically recompiled PowerPC game code in madworld_core.dll, with native MEM1/MEM2, original DOL and relocated enemy module. The Rust hardware adapter supplies GX, DSP/audio, input and presentation. This is an unfinished port: complete fight/victory, original timing, window/visual equivalence, complete PC UI and sound-event parity remain unverified. The separate mw-game rig/arena viewer is not the native gameplay frontend.

## Runtime and provenance

Core source: JaredMoss1996/MadWorldDecomp, CC0-1.0, pinned6e02a76a31dcd8c3147c92b223841bf52a4e7b23. Local runtime/generator changes are enumerated by native-core-local-files.json. Owned DOL SHA1b1a42fd6acb9dfe076b583b03b610ca44a65f532. Original generated instruction bodies and assets stay onD. Gecko-derived hardware code retains its licensing/attribution; see native-platform/README.md and THIRD-PARTY records.

Build core with tools/build_native_msvc.cmd (MSVC14.39, C++20, /O2 /fp:strict). Build Rust using the D-drive target directories in project instructions. Current host identities are evidence/native-host-binary-hashes.json. No DLSS. No second CPU interpreter/JIT runs original gameplay; the DSP interpreter is the audio processor.

The embedded ABI owns one native CPU worker and guest RAM, supports safe stop/join before unmapping, and rejects reuse. Native SDK threads run on host fibers. Detached state0 and joinable state8 retire correctly, including reused guest thread addresses; regressions and three lifecycle/error cases cover this previously reproduced stall. See TASK-API.md, tools/test_embedded_core.py and the current regression logs.

## Gameplay and choreography

Original scene201 entry requests event21 (1080 source frames/four cuts), then loads c20110.ems and initializes Rin native ID20440 and Jack10000/variant10030. Native actors execute original update, contact, damage and competition logic. PC bindings use shared PcControls packets; controls/profiles are described by PC-CONTROLS.md and PLAYER-COMPETITION-API.md.

The ordinary-input bevy-native-pulsed-clash replay has no contact fixture: one successful competition uses phases1/3/5/6/7, player04AA/04AB/04AC paired with Rin0110/0111/0112,144 complete source-content matches. Rin1600->1300HP; Jack70HP unchanged across that clash; ordinary play resumes. A separate explicitly requested original-API contact fixture also passes. These do not prove the complete fight, all mechanics, every event or console timing. audit_native_clash.py --require-natural distinguishes natural and fixture evidence.

## Graphics, UI and sound

Actual headless DX12 tests render original GX traffic on RTX3090 and complete the host blit. They create no window/surface/readback/screenshots, and do not prove visible pixels, material fidelity or usability. Recovered original TEV programs and Explorer inspection previews are separate evidence. Display lists are flattened once by C++; Rust rejects parent opcode40. Preserve command/memory/IRQ order in any transport change.

Native etc/core_us.idd has a staged same-length PC prompt overlay:11 selected slots, three modified CI4 pages matching full pixel-array hashes in live native RAM. Source resource/RVZ untouched. This establishes loaded bytes, not all Wii screens/contextual prompts, legibility or transform correctness. See native-pc-ui-summary.json.

Original AID PCM reaches the Bevy audio sink. Remote-speaker state/register routing, Yamaha ADPCM/s8/s16le decode, snapshot compatibility and mono-to-stereo mixing have fixture coverage. Observed game speaker packets are muted by native configuration and correctly produce no speaker mix; do not invent audible events. Complete sound-event selection, timing, mix and hearing equivalence remain open. See REMOTE-SPEAKER-INDEX.md and native-remote-audio-summary.json.

Explorer Color version remains default off across all models/event roles and uses an added inspection palette. It is independent of native original shading and does not recover canonical character colors. Some identities and event/visibility semantics remain unknown.

## Timing and performance

MOTION-CLOCK-API.md and motion-clock-api.graph.jsonl index bounded source timing dependencies and explicit ABI unknowns. Native clocks/animation steps are unchanged. The current quantized-memory helper forms valid exact power-of-two factors directly and retains CRT fallback outside the GQR domain; evidence/native-quant-scale-summary.json compares33,687,552 loads,20,480 stores and296 factor/environment cases across four rounding modes against the prior implementation. This is regression equivalence, not complete Broadway arithmetic proof.

PERFORMANCE-NEXT.md records wall-time attribution, local-symbol leaf sampling and limitations. Keep full draw behavior, dirty generations, DSP deadlines and native arithmetic. A helper benchmark or presentation count is not proof of full-game speed or original timing.

## Remaining acceptance

The full objective stays active. Required evidence still includes an entire native fight through victory/loss and ensuing cutscenes/HUD/audio, all relevant PC actions/prompts, comparable original-versus-port timing and rendering, and live visual review. gk.json/ledger.json hold scoped validators and obligations. A build or headless pass closes none of the broad visual/gameplay requirements by itself. Historical investigations and superseded states are retained in notes/MODLOG.md; use this page and current evidence for implementation status.
