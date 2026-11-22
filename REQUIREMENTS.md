# MadWorld: RinRin boss fight port

Target: a Rust/Bevy reconstruction of the native boss encounter, including native player and boss mechanics, arena/collision, damage and interaction rules, cutscenes, HUD/UI, music/voice/effects, and appropriate PC controls/prompts replacing Wii remote UI. No DLSS. All port work/data/builds remain on D:.

## Completion evidence
- Source-linked typed graphs for boss/player dispatch, mechanics, timings, damage, hit/catch/finisher rules, RNG and every required engine API.
- Native rigs/weights/motions/mesh draw states and recovered material rules, with original-runtime comparisons. Optional added color mode clearly identified.
- Native arena geometry, placements, collision/navigation and lighting.
- All encounter cutscene actors, camera samples, sequence events and transitions on native clocks.
- Native HUD, voice/music/effects event bindings and timing; PC controls and prompts, configurable/rebindable where appropriate.
- Deterministic original-vs-port mechanic traces and outcomes, and rendered/live play verification. Export/unit checks alone do not establish 1:1 fidelity.

Current work is incomplete. A model viewer, generic AI, emulator wrapper, guessed arena or approximate gameplay does not satisfy this target.
