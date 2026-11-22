# Evidence ledger

Claims: 8 fact, 1 inferred, 1 unknown.

## Obligations

| id | needs | state | verifiers | description |
|---|---|---|---|---|
| mesh.structural | structural | **open** | - | Bounds finite, triangle/vertex counts match source, UVs and material slots resolved |
| mesh.rendered | rendered | **open** | - | Rendered view matches the original (shape, scale, orientation, textures) |
| texture.structural | structural | **open** | - | Decoded size/format matches header; mips and alpha handled |
| texture.rendered | rendered | **open** | - | Appears correct on a model, not just in an image viewer |
| skeleton.structural | structural | **open** | - | Bind pose sampled; weights unit-sum; bone hierarchy finite |
| skeleton.rendered | rendered | **open** | - | Posed character is not stretched or inverted |
| animation.structural | structural | **open** | - | Tracks bound by shared bone hashes/names; sampled poses bounded |
| animation.behavioral | behavioral | **open** | - | Clip selected by gameplay state plays correctly in play |
| world.structural | structural | **open** | - | Placement count and transforms match source records |
| world.rendered | rendered | **open** | - | Camera view agrees with the original's layout |
| world.behavioral | behavioral | **open** | - | Collision/grounding works when walked on |
| audio.structural | structural | **open** | - | Decoded length/sample rate match source |
| audio.behavioral | behavioral | **open** | - | The right sound plays for the right event, including delays and switches |
| hud.structural | structural | **open** | - | Layout, frame labels, nested transforms parsed |
| hud.behavioral | behavioral | **open** | - | States change with gameplay (ammo, health, weapon) |
| shader.structural | structural | **open** | - | Texture slots and constants identified from the original program |
| shader.rendered | rendered | **open** | - | Material look matches the original under comparable lighting |
| gameplay.behavioral | behavioral | **open** | - | Mechanic matches the original's data-defined numbers and feel |
| performance.rendered | rendered | **open** | - | Measured with real rendering enabled, warm and repeated |
| native.input-mapper | structural | **closed** | pc-mapper-contracts:structural | PC keys/buttons form the intended explicit native controller packet |
| native.hid-transport | structural | **closed** | native-hardware-contracts:structural | Native HID transport and held/directional motion contracts pass |
| native.source-role-content | structural | **closed** | native-source-role-observations:structural, native-pair-source-audit:structural | Recorded Rin/player role MOT identities agree with complete owned source members |
| native.audio-device-sink | structural | **closed** | native-audio-device:structural | Headless Bevy audio sink consumes captured original PCM |
| native.embedded-lifecycle | structural | **closed** | embedded-host-cases:structural | The owned embedded core stops safely and isolates startup failures |
| native.pc-damage-observed | behavioral | **closed** | native-health-delta-analysis:behavioral | Boss HP reduction is observed through shared PC inputs in native gameplay |
| native.struggle-completion | behavioral | **closed** | native-clash-fixture-trace:behavioral, native-natural-clash-trace:behavioral | PC held-struggle input completes the native competition and required follow-up |
| native.pc-ui-replacement | behavioral | **open** | - | Native controller graphics/prompts are replaced with correct PC UI in gameplay |
| native.natural-clash-contact | behavioral | **closed** | native-natural-clash-trace:behavioral | Normal PC attacks initiate Rin competition through unmodified AI and native contact resolution |

## Claims

### fact

- **C-001** Native PC replays observe Rin HP reduction, including1600 to1489, using explicit shared PC packets through the original game systems. _(runtime:evidence/native-held-struggle-summary.json)_
- **C-002** Both original actor-role motion contents match complete owned MOT resources during88 concurrent native paired observations (04AA/AB/AD with0110/0111/0113). _(runtime:evidence/native-rin-pair-content-summary.json)_
- **C-003** The Bevy AudioPlugin/default-device sink consumes captured original AID PCM through NativePcm. This does not establish human audibility or original mixing/timing equivalence. _(runtime:evidence/bevy-native-audio-device.json)_
- **C-004** The Explorer Color version checkbox remains default-off and its added palette applies across models/event roles; colors are an inspection treatment. _(runtime:D:/Games/gamesfordecomp/exports/madworld/evidence/color-toggle-http-regression.json)_
- **C-007** A separate same-length standalone core_us.idd changes 11 controller prompt slots while preserving palettes and every untargeted sprite. All three modified full CI4 texture pages match staged SHA256 values in the native guest RAM. _(bytes:staged/pc-ui/manifest.json; runtime:evidence/native-pc-ui-live-memory.json)_
- **C-008** The native USB speaker path decodes and mixes an unmuted synthetic ADPCM report fixture into the installed PC stereo sink. Native speaker initialization is observed as ADPCM at divisor2000 (6kHz), volume88, then muted; natural sound-event output remains unverified. _(runtime:evidence/native-speaker-mixer-tests.log; runtime:evidence/native-remote-audio-summary.json)_
- **C-009** In an explicit original-API contact fixture, shared held-X input raises the native meter, passes the directional follow-up, selects full source AA/AB/AC and0110/0111/0112 role contents, reduces Rin HP1600 to1300 and returns the player to ordinary play with HP80 unchanged. _(runtime:evidence/bevy-native-clash-fixture-clash-summary.json; runtime:evidence/bevy-native-clash-fixture-paired-summary.json)_
- **C-010** With the contact fixture disabled and no fixture execution, ordinary PC chainsaw press/release and strike inputs naturally initiate Rin competition. Held X passes the native meter and directional follow-up, selects original success roles, reduces Rin HP1600 to1300, and returns to ordinary play with player HP70 unchanged. _(runtime:evidence/bevy-native-pulsed-clash-clash-summary.json; runtime:evidence/bevy-native-pulsed-clash-paired-summary.json)_

### inferred

- **C-005** The common core.idd atlas contains the original handheld controller pictures and button glyphs; no native PC graphic replacement has been implemented. _(bytes:research/ui-common-and-core-source-index.json)_

### unknown

- **C-006** Full fight/competition/win-loss, native timing/shading/pixel equivalence and Bevy window behavior remain unverified. _(no evidence)_

