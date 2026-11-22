# MadWorld native character explorer

Working local inspection build for the owner's USA Wii disc, **RZZE8P revision 0**.
The full requested roster/animation/choreography work is **not complete**.

Local output: `D:/Games/gamesfordecomp/exports/madworld`.
Viewer: `http://127.0.0.1:8781/`. Asian Dynamite's port 8778 is separate.
No game assets, executable dumps, keys or downloaded tools are stored here.

## Reproduce

From the toolkit repository, with the already installed official DolphinTool:

```powershell
C:/Tools/Dolphin-2609a/DolphinTool.exe extract -i 'D:/Games/gamesfordecomp/madworld/MadWorld (USA) (En,Fr,Es).rvz' -o D:/Games/gamesfordecomp/exports/madworld/extracted/disc -g -q
python case-studies/madworld/build.py D:/Games/gamesfordecomp/exports/madworld
python case-studies/madworld/validate.py D:/Games/gamesfordecomp/exports/madworld
python case-studies/madworld/native_graph.py D:/Games/gamesfordecomp/exports/madworld
python case-studies/madworld/serve.py D:/Games/gamesfordecomp/exports/madworld --port 8781
```

The existing Three.js r175 vendor directory is copied from the adjacent Asian
Dynamite output. Missing `SkeletonUtils.js` is fetched from the official pinned
Three.js source. Texture decoding uses Pillow; the optional native call graph
uses Capstone. The toolkit CLI itself remains stdlib-only.

`Launch MadWorld.cmd` in the output can restart the packaged viewer. A detached
hidden Python process currently serves it; logs are `server.stdout.log` and
`server.stderr.log` on D:. Stop only that MadWorld process before using the launcher.

## Current evidence and limits

- **Source:** original 2,427,771,660-byte RVZ remains intact. SHA-256 is
  `cc78c7cc20f004f76d2d560f1b307e95f1ba2717aaec4577a69001395d304d17`.
- **Inventory:** 784 top-level DAT archives; 714 MDB resource bindings and 4,082
  nonempty MOT resources. Duplicate member names retain distinct source offsets.
  Empty members and 512 non-DAT `.dat` resources remain explicit inventory records.
  The latter include event/message data; they are not all failed asset archives.
- **Structure:** all 4,082 nonempty MOT resources decode. 707/714 model resources
  pass the full geometry/weights/hierarchy/relative-bind validator. The seven
  exceptions are listed in `evidence/structural-validation.json`; five concern
  non-character resources and two concern male boss bind discrepancies.
- **Rendered roster audit:** contact sheets cover 70 unique model resources with
  more than 15 bones. The reviewed cast includes Jack's four body resources,
  RinRin, Mathilda, Elise, Naomi, Amala, an unidentified woman, and two distinct
  female silhouette bodies. Identity authority is stated per model and guarded
  by source mesh hashes. Naomi/Amala are appearance identifications; their native
  identity-string links are unresolved. The hat-wearing woman and silhouette
  identities remain unresolved. Small-rig/unrigged representations and
  pre-rendered-only characters are not closed coverage. **Do not claim every
  female character or costume is verified.**
- **Rig:** native hierarchy, absolute/relative bind positions and all four native
  influences are retained. Weights are percentages, not UNORM8. Bone IDs are
  unsigned bytes: RinRin needs 128 and 129. This corrects a signed-ID assumption
  in the reference reader. No generic humanoid rig or per-screenshot offset fits
  are used. Every native mesh is visible by default; `_SB` includes Jack's head.
- **Motion:** scalar constants, float/quantized samples, proprietary 6/9/47 half
  values and Hermite forms 4/6/7/8 are decoded. Local TRS uses the reference
  reader's ZYX convention; root channels are applied once. The adjustable 60 fps
  default comes from that reader, not a recovered runtime timebase. Event-driven
  animation and native actor/world scheduling remain unresolved.
- **Descriptions:** 28 concrete rendered clip descriptions currently exist.
  These are inspection descriptions, not recovered original action names.
  Other clips honestly retain source IDs. Previously generated
  `rinrin-motion-*` strips used an incorrect signed-ID compatibility filter and
  are invalid as authored-motion evidence; `rinrin-native-*` replaces them.
- **Ownership:** the default source-family filter finds clips across archives,
  including Mathilda's separate bank. Prefix association, same-bank containment,
  bone compatibility, native dispatch and gameplay eligibility remain distinct.
  The UI does not claim native ownership or full gameplay eligibility.
- **Pairing:** one Jack/Elise, 178-frame simultaneous inspection candidate is in
  the main selector, with independent native model controls, common playback,
  seek, speed, looping and skeleton/in-place controls. Same-bank containment and
  equal duration motivated this candidate. Native role dispatch, relative
  placement/facing, attachment/contact, release and recovery are unverified.
  It is **not** a recovered full finisher, phase chain or contact-verified pair.
- **Materials:** native mesh flags select recovered six-stage black-base and
  nine-stage white-base GX programs, with per-mesh thresholds and secondary AB
  ink atlases. GX intensity alpha and CMPR interpolation are decoded correctly.
  Native ink preview uses recovered stage operations; scene lighting, shadow
  depths, konst uniforms and visibility remain partial. Live-runtime rendering
  equivalence is not yet established. Source texture and lit previews remain available.
- **Color version:** the explorer checkbox applies an added palette to all models
  and event roles, including Jack and every female model. Default is off. Colors
  are an inspection treatment, not recovered canonical colors. Export records the
  active treatment; inactive vertex colors are omitted from monochrome GLBs.
- **Native events:** 73 base-language events expose 2,484 decoded actor motion
  chunks, native cut IDs and source cameras. Cast cuts appear beside individual
  clips, with optional continuous cut playback. Original code establishes base
  cut-step arithmetic; update cadence, SEQ timing/visibility/effects and gameplay
  dispatch remain incomplete.
- **Export:** rig plus sampled native clip (both roles when paired) saves directly
  to D:. Every export is reimported using GLTFLoader, compared at four integer
  frames against native bone world matrices, and checked for all skin influences, positions, UVs and triangle indices.
  The `.validation.json` beside each GLB records actual-file results. This is
  numerical/export evidence, not an original-runtime visual comparison.
- **Caching:** generated models/motions/textures are keyed by content hash and
  decoder version. Texture images decode separately; readers use private file
  handles and cache writes are serialized. Actual cold/warm UI observations are
  recorded separately; no broad FPS or memory claim is made.

## Graphs and validation

Generated native logic and resource evidence stay on D:. Read `INDEX.md`,
`API.md`, `NATIVE-INDEX.md`, `NATIVE-API.md`, `PAIRED-INDEX.md`, and the focused
`evidence/scripts/*.md` files before querying graphs:

```powershell
python -m gk graph D:/Games/gamesfordecomp/exports/madworld/evidence/resources.graph.jsonl digest
python -m gk graph D:/Games/gamesfordecomp/exports/madworld/evidence/native-actions.graph.jsonl search cEm0630
python -m gk graph D:/Games/gamesfordecomp/exports/madworld/evidence/paired.graph.jsonl digest
python -m unittest discover -s tests -p test_madworld.py
```

The RSO graph records exported symbols and resolved direct-call/relocation
relationships, plus conservative immediate-call candidates. It is partial;
branch conditions, virtual calls, resource-pointer dataflow and actor dispatch
are not reconstructed. Pseudocode records that limitation instead of emitting
raw decompiled dumps. Native function strings are never zipped by sorted index
to animation filenames.

Audit URLs are reproducible browser rendering tools: `?audit=0` through `3`,
`?motion-audit=em0440&count=12`, `?motion-audit=em0610&count=12`, and
`?pair-audit=1`. Screenshots and temporal sheets are saved on D:.

## Prior art

Format research and local source reference:
[Kerilk/noesis_bayonetta_pc](https://github.com/Kerilk/noesis_bayonetta_pc),
commit `bc030a37d3925fe378b14383680a08cb8762da09`, MIT. See `THIRD-PARTY.txt`.
Noesis's vendor download returned HTTP 406 three times; that route was stopped.
This adapter was implemented against the owned disc with bounded validation,
using the reader's MadWorld-specific layout knowledge rather than Bayonetta
format constants. RSO framing was checked against Dolphin's public
`Core/Debugger/RSO.h`; no Dolphin implementation was copied into this adapter.
