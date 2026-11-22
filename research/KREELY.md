# Kreese Kreely: added character, explorer stage (rig v2)

Kreely is an **added character**. His mesh is a user-provided model, not a disc resource. For now he exists only
in the model explorer (`exports/madworld`), rigged on Jack's `pl0030` skeleton (plus finger chains) and playing
Jack's MOTs. The game runtime (`crates/`, native core) is untouched.

Fact labels used below: **[native]** comes from disc data or the explorer's native readers. **[user]** comes from
the user's files. **[inferred]** is our own choice or estimate. **[measured]** comes from our own scripts. **[unknown]** is open.

## Reference note (character)
Source: https://madworld.fandom.com/wiki/Kreese_Kreely (read 2026-11-08 through the MediaWiki API; the HTML page returned 402).
- **Role:** one of the two DeathWatch commentators on Shock TV, with Howard "Buckshot" Holmes. He is a former
  contestant and champion who says he fought most of the bosses Jack faces. The only loss he names is to Elise.
  Voiced by John DiMaggio (NA).
- **Personality:** crude, macho, anti-intellectual (calls himself illiterate, mocks Howard's vocabulary), drinks
  (Scotch) and uses drugs. He enjoys the gore and tells war stories about his own kills and injuries, but says he
  doesn't want to go back to DeathWatch. He insults Howard constantly, though they sometimes bond over drinks.
  He is implied to be physically worn down by those injuries.
- **Look, outfit, colors:** the article has **no physical description** and no infobox image. Everything visual
  here comes from the user's model **[user]**: a burly man with slicked-back dark hair, heavy brows, a mustache and
  a goatee. He wears a dark suit jacket with wide lapels over a white shirt that is buttoned below mid-chest and
  open above it, showing his neck and a hairy chest. He also wears a belt with a large square buckle, dark trousers
  and dress shoes. The texture is MadWorld-style black/white ink. The source face has only squint slits for eyes;
  the painted eyes are our addition (see Textures).
- **Color version palette [inferred]:** an added inspection palette in the explorer's existing style, not
  recovered artwork. It is: charcoal-blue suit `#4a5368`, off-white shirt `#ece6d6`, warm skin `#d69c76` (face,
  neck, bare chest, hands), dark brown hair/beard/chest hair `#3a2a20`, **brown eyes** (iris `#8a5a32`, sclera
  `#f4f1ea`), brass buckle `#d0a646`, brown-black belt `#3b2a22` and shoes `#2c2422`. The earlier red tie/vest
  accent was removed at the user's request: it read as a blood streak, and the source has no tie there, only bare
  chest and chest hair.

## Source files
| File | What | Provenance |
|---|---|---|
| `assets/characters/kreely/source/kreely.fbx` | FBX 7.4 binary, one mesh: 5,627 verts, 10,608 tris, one UV layer, **no armature, no weights, no animation**. Fingers and thumb are separate geometry. | [user] sha256 `eef7342f…87600b` |
| `assets/characters/kreely/source/kreely.fbm/kreely_basecolor.jpg` | 2048² grayscale ink atlas | [user] sha256 `4ed144d7…e419a9` |
| `pl/pl0030.dat#pl0030.mdb@19552` | Jack, 36 bones, T-pose, Y-up, facing +Z | [native] sha256 `3578b095…c90561` |

As delivered, the model is in a T-pose, Y-up and facing +Z (the same basis as Jack), 0.842 units tall, palms down.

## Pipeline (reproducible, CLI only; Blender 4.4 headless is called twice)
```
python -I tools/kreely/build_kreely.py                       # build + explorer hook
python -I tools/kreely/regress_kreely.py --base http://127.0.0.1:<explorer port>
python -I tools/kreely/clip_metrics.py <model.json> [jack] --step 8
python -I tools/kreely/offline_pose_render.py <out.png> fist|relaxed|open [clip frame hand|handR|head|all]
python -I tools/kreely/offline_jaw_render.py <out.png>
python -I tools/kreely/ortho_texture_render.py <out.png> <texture> <x0> <x1> <y0> <y1> [width] [--grid G]   # per-pixel textured front view
```
- `dump_fbx_mesh.py`: splits vertices per unique (vertex, UV), one UV per vertex as in MDB.
- `kreely_fingers.py`: detects the finger chains.
- `blender_weights.py`: bone heat.
- `kreely_pose.py`: numpy copy of the explorer's `sampleActor` plus linear blend skinning, used by the metrics,
  the regression and the offline renders.
- `build_kreely.py` options: `--deltoid` (default 0.8), `--clavicle-chest` (default 0), `--jack-clavicle-height`,
  and `--variant NAME`, which writes to `build/variants/NAME` and leaves the explorer hook alone. The variants are
  for measuring with `clip_metrics.py`.

Outputs (all gitignored, local on D:) in `assets/characters/kreely/build/`:
- `kreely.model.json`
- `kreely_basecolor.png` (the source atlas plus painted eyes; the explorer's primary texture)
- `kreely_color.png` (Color-version texture)
- `kreely_AB.png` (shadow-ink texture)
- `kreely.build.json` (landmarks, joints, finger chains, bind-direction report, weights, hashes)

## Rig
**pl0030 part (36 bones) [native ids, measured positions]:** the same bone ids, parents (array-index convention)
and zero bind rotations (ZYX) as Jack. The joint positions are Kreely's own:
1. Kreely is uniformly scaled to Jack's height (×2.1940) and centered, and his chest depth is aligned (Δz +0.016).
   The mesh is not warped, so he keeps his own silhouette.
2. Jack's joints are carried into Kreely's proportions by a landmark map (sole / crotch / sleeve underside / top
   on y; armpit / wrist / fingertip on |x|). Kreely's landmarks are measured on his own mesh with plane slices.
3. The whole arm chain (clavicle, deltoid helper, shoulder, elbow, wrist, weapon attach) keeps Jack's offset from
   the arm axis. **Bind bone directions therefore match Jack's within 0.5° on every deform chain** (spine,
   clavicle→hand, hips→toes). The one exception is the chest→clavicle pivot offset (8.5°). See
   `bindDirectionDegFromJack`.

**Finger part (30 bones, ids 0x600–0x60e left / 0x620–0x62e right) [measured]:** three bones per digit
(thumb, index, middle, ring, pinky), parented to hand bones 14 (+x, his left) and 9 (−x, his right). Names are in
`boneNames`, such as `L_index_2`. These ids are unused by every MOT in the catalog, whose ids are ≤0x81,
0x400–0x403 and ≥0xf60. Chains are detected on the mesh: the web line between the fingers comes from plane
slices, the fingers from 1-D k-means of the finger vertices, the thumb from the largest z gap, and each axis from
PCA. Knuckles sit 0.13 of hand length inside the palm, and joints are placed at 45% / 75% of the finger length.

**Hand poses [inferred]:** pl0030 MOTs carry no finger tracks, because Jack's hands are modeled as fists. Kreely's
fingers are posed by named local-quaternion sets in the model (`handPoses`), applied before the MOT tracks:
- `fist` (default, matching Jack's look): about 80/100/65° curls; the thumb swings 55° to the palm side and 30° across, then bends 30°/45°.
- `relaxed`: a slight curl.
- `open`: the bind pose.

There is a **Hands** selector in the explorer (shown only for models with poses) and a `&hands=` deep-link
parameter.

**Retarget [measured]:** across all 248 `pl0030.dat` clips, MOT position tracks exist only on root −1/0, face
helpers 25/26/40/41, weapon attach 100/101 and the prop bone 1024. Every body joint is rotation-only. Kreely's
model therefore declares `retarget: {rotationOnly: true, rootScale: 0.986}`:
- non-root position tracks are ignored, so his own bind offsets stay;
- root translation is scaled by the hip-height ratio;
- rotations are applied unchanged.

**Result:** on 1,040 sampled frames covering every Jack-compatible `pl0030.dat` clip, all 36 pl0030 bones have
**identical world rotations** to Jack's (max abs diff 0.0). The regression checks this.

## Skinning
1. **Bone heat** (Blender `ARMATURE_AUTO`) on the unwarped mesh with Kreely's joints: 21 body bones + 30 finger
   bones. 5,500 of 5,627 welded vertices are solved.
2. **Islands** that bone heat can't solve (127 verts: buttons, buckle, small shells) copy the weights of the nearest
   solved vertex. This replaces v1's distance-to-bone fallback.
3. **Shoulders (Jack's own pattern) [native pattern, inferred amount]:** Jack weights his deltoid helpers 6/11
   (≈0.5 average) blended with clavicle 5/10 and upper arm 7/12. Within a 5.5–15 cm falloff of Kreely's shoulder
   joint, 80% of the clavicle + upper-arm weight moves to 6/11. Jack also blends his clavicles with chest bone 2.
   That was tried (`--clavicle-chest`) and **left off** because it measurably increased head-core hits
   (0.392 → 0.427/frame).
4. **Jaw [inferred]:** chin and beard below the mouth line, on the front of the face, weight to bone 24, capped at 0.65.
   Jack's MOTs open the jaw up to 1.05 rad in 40 clips. Kreely's mesh has no mouth interior, so the cap keeps
   the chin from tearing (see `offline-jaw-open.png`).
5. Top 4 influences ≥0.01, renormalized, stored as bone **array indices** (the MDB/explorer convention).

## Textures and color
- **Normal:** `kreely_basecolor.png`, which is the source ink atlas plus **painted eyes [inferred]**. The FBX face
  has only squint slits with no eyeball. The build paints almond eyes into every texel whose 3-D position falls in
  each eye opening: white sclera, a light iris with a dark ring, a black pupil, an ink line under the upper lid and
  a catch-light. Eye centers are at (±0.0185, 0.9135) of height, 0.021 wide, with ~0.0056 opening, a 0.0034 iris
  radius and outer corners tilted up 0.0012 for the scowl; they were hand-placed from ortho renders (`EYES` in
  `build_kreely.py`). Painting is supersampled 4×4 per texel for anti-aliased edges, and seam texels near the
  eyes take their nearest surface point so no unpainted specks show. The face has only ~40 texels across each
  eye, so close-ups stay soft. Evidence: `ortho-eyes-{normal,color}.png`.
- **Native ink:** now runs on Kreely through the explorer's recovered white-base TEV program (renderFlags 0x2000,
  threshold 140), using a **synthesized** `kreely_AB.png` **[inferred]**. Jack's `pl0030_AB` turned out to be an
  artist-painted shadow pass: same UVs, intensity in RGB+A, most whites kept, midtones darkened (L 96–224 →
  about 0.8 L), with some painted black fill. Ours is an approximation: 0.5·L^1.5 plus 0.5·min3(L)^1.5. The painted
  fill is not reproduced.
- **Color version:** a per-texel bake (`kreely_color.png`). Every UV triangle is rasterized at 2048², its 3-D
  position is interpolated per texel and classified into regions, gutters are filled from the nearest texel, and
  the palette is multiplied in linear space, the same math as the explorer's map × vertex color. Region edges now
  follow the texture instead of the large low-poly triangles. All region rules are **[inferred]**, measured on
  ortho renders of the source:
  - **Beard/mustache:** only inked texels (L < 0.6) in the zone below the nose, from face landmarks measured on the
    front profile (nose 0.891, mouth 0.876, chin 0.854 of height; hairline nose + 0.06). White skin around the
    goatee stays skin.
  - **Head vs. collar:** the jaw/beard forms a V from (0, 0.835) to (±0.045, 0.875). Outside it, below 0.875, is
    the standing shirt collar.
  - **Bare neck and chest:** the open-shirt V from the apex (0, 0.745), with edges at ±0.011 / ±0.032 / ±0.04 at
    y 0.76 / 0.80 / 0.835, then the neck up to the jaw (±0.04 → ±0.046). It is skin, and the inked strokes on the
    chest are chest hair (hair color).
  - **Shirt:** bright texels between the lapels (|x| < 0.045 + 0.2·(y − 0.62), front half) and the collar.
  - **Belt:** only in the visible gap between the jacket fronts, so the jacket hem is not tinted.
  - **Eyes:** sclera/iris regions come from the eye painting.

  v3 fixed the issues reported in the user's color screenshot. The red tie strip is gone. The neck/collar band had
  been tinted suit-blue: it failed a chest depth test and part of it fell in a beard zone, which read as
  "random shadows". The shirt also had blue side stripes. The explorer swaps this texture into the color
  materials, and per-vertex `regions` (same rules) are kept as a fallback. Evidence:
  `ortho-upper-body-{source-jpg,normal,color}.png`, `ortho-neck-chest-color.png` and
  `kreely-bind-{color,normal}-lit-face.png` (the user's Lit normals + Color view).

## Explorer integration
Changes in `exports/madworld` (pre-Kreely originals are backed up in `assets/characters/kreely/explorer-backup/`):
- `custom/characters.json`: catalog hook. It holds metadata plus local paths and sha256 hashes for the model,
  texture, colorTexture and abTexture, and no asset data. `motionFamily` holds the swappable animation binding.
- `builders/serve.py`: custom characters in `/api/catalog`, with `textureAB` set when an AB texture exists.
  `/api/model` serves the model, and `/api/texture` serves `bank=primary|ab|color`, all verified by hash.
- `app.js` (shared with other sessions, which also edit it; changes are additive):
  - `motionFamily()` / `motionBank()`.
  - `sampleActor` applies `retarget` (rotation-only, `rootScale`) and `handPoses`.
  - `compatible()` honors a model's `ignoreMotionBones`.
  - The color texture is swapped in, and there is a Hands selector.
  - Deep link `?model=&clip=&t=&color=1&material=&hands=&orbit=&focus=<bone id>&dist=`.
  - `?motion-audit=` takes `&clips=` / `&color=1` / `&material=`.
- `color.js`: `kreely` palette; models with per-vertex `regions` are tinted by region.
- `index.html`: Hands selector (hidden unless the model has hand poses).

Disc models behave as before, and Jack's own menu is unchanged. **Color version** stays default-off.

## Verification (evidence in `evidence/kreely/`)
- `kreely-http-regression.json`: **ok: true**, 18 checks:
  - pl0030 ids/parents kept; 30 finger bones on unused ids; hand poses; rotation-only retarget;
  - all three textures served; skin arrays aligned; weights normalized; joint indices valid;
  - Kreely's menu contains every clip in Jack's menu (651), plus all 248 `pl0030.dat` clips (911 `pl0030_*` clips in total);
  - all 248 clips decode; **bone world rotations equal Jack's** (1,040 frames, diff 0.0);
  - Color checkbox present and default-off.
- In-explorer captures (headless Chrome against my explorer instance on port 8795):
  - `kreely-clips-{normal,color,ink,color-ink}.png` next to `jack-clips-{normal,color}.png`: the same 5 clips, 4 frames each.
  - `kreely-previously-hidden-clips-{normal,color}.png`: 0404/0403/040c, which were hidden in v1.
  - `kreely-bind-{normal,color,ink}.png`, `kreely-0000-f40-{normal,color}-{orbit45,fists}.png`, `kreely-0505-f25-{color,ink}-front.png`.
  - `kreely-bind-{color,normal}-lit-face.png`: face, neck and chest in Lit normals mode (eyes, collar, bare chest).
- Offline (numpy poser, same math): `offline-hand-L-{open,relaxed,fist}.png`, `offline-hand-R-fist.png`,
  `offline-jaw-open.png` (0 / 0.4 / 1.0 rad), `offline-120a-f32-head.png` (the worst clipping frame), and
  `offline-jack-vs-kreely-0505-f25.png` (Jack's own mesh at the v1 "shoulder over face" frame, flat-shaded with
  head-driven verts in tan).

## Known-issue fixes from v1, and what is left
| v1 issue | Status |
|---|---|
| 40 `pl0030.dat` clips hidden (bone 1024) | **Fixed for Kreely.** The model lists 1024/1025 in `ignoreMotionBones`, so all 248 play. Jack's own menu still hides them, by the explorer's unchanged policy for disc models. What 1024 is (a prop or held object, from its position-only tracks) is **[unknown]**. |
| Flat hands | **Fixed.** 30 finger bones, heat-weighted, with a default `fist` pose plus `relaxed`/`open`. Left: Jack's clips have no finger data, so grabs and open-hand moments still show the chosen preset. A per-clip hand-pose table is future work. |
| No jaw / eyes / brows | **Jaw fixed** (bone 24 weighted, follows Jack's jaw tracks, capped at 0.65). Eyes 40/41 and brows 26: **not applicable**, because Kreely's eyes and brows are painted, with no eyeball or lid geometry. Cigar bone 25 is Jack-only. |
| Clipping (raised-arm shoulder over the face) | **Explained and partly addressed.** At `0505` f25, **Jack's own mesh covers his chin the same way** (`offline-jack-vs-kreely-0505-f25.png`): the head is tucked into the raised arm in the authored pose. v2's own-proportion arms (bind directions equal to Jack's) removed v1's extra shoulder swell. Measured over 1,455 frames (`clip_metrics.py`): arm-driven verts inside the head core are 0.063/frame against Jack's 0.021, and all non-head verts 0.392/frame. Jack's all-body number is not comparable, because his collar and goggle straps sit inside his own head box. Helpers did not lower the metric: the deltoid helper was neutral, and clavicle→chest made it worse, so it is off. The worst frames left (`120a` f32, `1105` f8, `110b` f8, `063a` f32) are Jack's hand-to-head and guard motions meeting Kreely's larger head, hair and sleeves. That is a contact, not a skinning tear, and fixing it needs Kreely-specific animation or arm IK. |
| Proportions warped to Jack | **Fixed.** The mesh keeps Kreely's proportions, and the skeleton is fitted to him with rotation-only retarget. Left: legs are ~1.5% shorter (root scale 0.986), so slight foot sliding is possible in locomotion. There is no foot IK. |
| Heat fallback (127 verts) | **Fixed.** Nearest solved vertex instead of distance to bone. |
| Heuristic, blurry color regions | **Improved (v3).** Texel-accurate bake with measured beard, jaw-V, bare-chest and collar zones. The red strip and neck/collar "shadows" are fixed. Still **[inferred]**: the regions are rules, not a painted mask. |
| No eyes (squint slits only) | **Added (v3).** Painted almond eyes in the primary texture, brown irises in color. They are hand-placed and limited by the face's texel density. |
| No AB texture, so no native ink | **Fixed approximately.** A synthesized AB lets the ink TEV path run. It is not artist-painted. |

## Secondary weapon: handgun (explorer only)
Kreely's secondary weapon is a handgun in his right hand, where Jack carries his chainsaw arm. Jack's moves are
unchanged.

### Research: guns in MadWorld [native data + wiki]
- **Wiki** (MediaWiki API, 2026-11-08):
  - **Jude the Dude** (boss, Varrigan City Downtown) carries two revolvers. He fires while skating the walls in his
    ranged phase and shoots bullet volleys in his Power Struggle.
  - **The Greys of Area 66** (Grunt Enemy page) use ray guns as a last resort.
  - In cutscenes, **Leonardo Fallmont** and Jack point guns at each other, and Leonardo shoots Noa in the ending.
  - The Magnet Gun is a pickup tool, not a firearm.
- **Disc:**
  - A mesh-name scan of every model finds `L_gun`/`R_gun` plus holsters on **em0400/em0401 (Jude)**, holsters on
    em0150/em0151, and `gun` props **wp0670** (ray-gun shape), **wp0671** (semi-automatic pistol, 23 cm) and
    **wp0672** (submachine gun).
  - Event bindings: **wp0671 is held by em0940 (Leonardo)** in ev75 c8/9/11/12 (seated), ev78 c2 and ev93
    c8/18/19 (standing). **wp0672 is held by em0110 thugs** across ev93 (handling/holding only, with no clean
    fire bursts).
  - No gameplay Grey ray-gun clips were identified [unknown which model].
- **Skeletons:** em0400 (Jude), em0940 (Leonardo) and em0110 (thug) all use pl0030's bone ids 0–24 with the same
  bind directions (0–2° off), so their MOTs drive Kreely with the same rotation-only retarget, root scaled per
  family (`rootScaleByFamily`: em0400 0.791, em0940 0.955).

### Gun model and grip
- **wp0671 pistol** (mesh, texture page and `_AB` page, served as Kreely texture pages 1 and 2) rides prop bone
  **0x640** on hand 9. **Jude's revolver** (texture page 2) and left-hand copies (prop bone 0x641) are included
  but hidden by default; they're in the mesh checkboxes. Prop bones rest at scale 0, so no gun shows in Jack's
  pl0030 clips.
- **Canonical grip [inferred, revised after user feedback]:** the barrel runs along the index finger (hand −x),
  the grip runs across the knuckle line toward the pinky (hand −z), the slide sits on the index/thumb side, and the
  grip centre sits 1.2 cm past the knuckle line, 3.2 cm to the palm side. Grip finger poses: trigger finger
  22/60/30° into the guard, the other fingers about 72–80/95/50–55° wrapping a ~3 cm grip, thumb forward along the
  frame. Measured result: the grip centre stays 6.2 cm from the wrist at every sampled aim frame.

### Clip sets ("Secondary weapon: handgun" group in Kreely's animation menu)
Revised after user review. These are trimmed gameplay clips: the standoff (ev78 c2) and turning (ev93 c19) clips,
Jude's 6-shot burst (0102) and his two 2-shot clips (0032, 0053) were dropped.

- **Pistol (realistic)**, cut from Leonardo's standing ending clips (event MOTs, stored as overlay
  `range`s; the explorer plays only the range):

  | Clip | Source range | Notes |
  |---|---|---|
  | Aim hold (loop) | ev93 c8 frames 126–179 | Loop points picked where the end pose best matches the start |
  | Aim & fire | ev93 c8 frames 236–296 | Shot at clip frame 14; ends on the pose closest to its start |
  | Idle, lowered (loop) | ev93 c18 frames 2–57 | Before he turns (yaw holds at 155° until about frame 90); the pre-shooting idle |

  Leonardo aims **left-handed**, so the c8 clips are **mirrored** (`mirror: true`: L/R bone swap, x positions and
  y/z ZYX angles negated) and Kreely shoots right-handed. Event world placement is removed (`rootOrigin`, taken at
  the range start).
  **Aim layer [inferred]:** in the aim and fire clips, two-bone IK on both arms puts the pistol at **shooting
  height**: arms extended, rear sight about 3 cm under the eye line, 0.5 m forward, barrel level along Leonardo's
  own aim direction. The right hand holds the canonical grip and the left hand cups it (support grip). The shot
  gets a short recoil kick (9° muzzle climb plus 2 cm back, decaying over 5 frames).
- **Gunslinger**, from Jude's gameplay clips with the right gun drawn: `0002`, `0031`, `0033`, `0050`, `0051`,
  `0052`, `0057`, `0058`, `0105` (1 shot), `0201` (1 shot). **One-handed:** only the right pistol is shown. The
  **left arm** is solved every frame to hang relaxed at his side: the wrist target sits in the chest's frame, so
  it follows the torso through crouches, bends and falls, with the palm toward the thigh and a relaxed hand pose.
  **Exception: `0051` (the kneeling crouch)** keeps both hands on the single pistol: the left arm IKs to the
  support grip while the gun is in hand (`TWO_HAND_CLIPS` in `kreely_gun.py`; 0052 is the other kneeling crouch if
  that's the one wanted). Per frame, the gun goes in the hand (canonical grip) or at the hip (Jude's thigh→gun
  transform, scaled), blended through the draw. Jude's twirls are not reproduced in the hand.

### Floor contact (all pistol clips)
Checked after a user report. Before the fix, Leonardo's clips put Kreely about **12 cm through the floor** (event
root heights vs. Kreely's legs), and Jude's floated 6–14 cm (Jude's hip height includes his roller skates).
`rootLift` is now a per-frame root offset:
- On every frame where the **source** character touches the floor (Jude: his own skinned mesh, skates included,
  within 3 cm of its clip minimum; Leonardo: every frame, all standing), Kreely's lowest body point is placed at
  y = 0.
- Airborne frames interpolate the lift, so jumps keep their arc.
- The lift never lets any point go below the floor.

Result (regression `noClipBelowFloor`, every 3rd frame of every clip): the lowest body point is 0.000 m in all 13
clips, and contact frames sit 0–2.4 cm off the floor. 0105 (fall and get-up) is no longer lifted while standing.

### Shots and muzzle FX
- **Jude [native cue]:** `em0400_*_00.seq` records are 48 bytes (`"seq\0"`, count, then opcode, operands and a float
  frame). **Op 0x07** marks a shot, followed 1–3 frames later by a `0x1069:00xx` gunshot sound in `*_10.seq`. For
  example, 0102 fires at frames 11/15/21/27/33/39. The meaning of op 0x21 (paired with 0x07 in 0102) is [unknown].
- **Leonardo [inferred]:** event SEQs only toggle visibility (op 0x14), so shots are **recoil spikes** of the event
  gun along its barrel: 8–40 mm/frame local maxima, barrel level, gun steady before. That gives ev93 c8 frame 250
  (12.7 mm/frame against about 3.5 mm/frame of handling noise). The ev78 c2 fall is rejected.
- **FX:** `em0400.eff` is `EFF\0` + `TEX` (one 128² TPL at 0x60: two **muzzle-blast streaks** and a **shot ring**) +
  `EST` (33 particle emitters of 672-byte records: offsets, cone angles, 0/0.25/0.5/0.75/1 keyframe curves; **not
  decoded**) + empty `MOD`/`TUV`/`MOT`. The explorer draws the native sheet as a mask (alpha = 1 − luminance) on two
  crossed blast planes along the barrel plus an expanding ring at the muzzle, for 4 frames from each shot, white
  (warm `#ffd890` in Color mode). The emitter shape, timing and color are [inferred] until EST is decoded.
- No sound playback. The cue ids are recorded above.

### Verification
`kreely-http-regression.json` has **34 checks, all passing**. The handgun ones:
- the exact clip set (13 clips)
- every overlay clip exists and is compatible
- texture pages 1/2 + AB + FX served
- gun hidden in Jack clips
- aim and fire at eye height (gun pivot −0.11 to −0.13 m from the eye joint)
- grip centre in the right hand (0.062 m from the wrist)
- the fire clip has its shot
- **Jude clips one-handed** (left hand at least 12 cm from the grip; 0.2–1.07 m measured)
- **0051 both hands on the pistol** (median 5 cm)
- **no clip below the floor**

Captures are in `evidence/kreely/handgun/`: explorer views plus offline strips of the one-handed clips, the
two-handed crouch, the idle loop, the fire clip and a grip close-up.

### Known issues (handgun)
- The realistic set has no aim-raise or aim-lower transition: idle, aim hold and fire are separate loops. Blending
  between them at runtime (or authoring transitions) is still to do.
- The support hand is placed by a fixed offset under the grip, so on some frames its fingers clip the right hand
  or the frame.
- Jude's gunslinger clips are cowboy style (hip draws, spins). Their spins are replaced by a steady grip, and the
  relaxed left arm is a fixed hang relative to the chest, with no secondary motion.
- The muzzle FX is an approximation of the EST emitter, with no smoke, casings or sound.

## Plan to make him playable
1. **Skeleton:** keep pl0030 + fingers. Every player system that assumes pl0030 ids keeps working, finger bones are
   ignored where they're unknown, and the retarget block is per-character data.
2. **Own clips:** author `kreely_*` MOTs (idle, walk, run, fight set, plus finger tracks and jaw talk) on this
   skeleton, then switch `motionFamily`. States without a Kreely clip fall back to `pl0030_*` with rotation-only
   retarget.
3. **Game asset path:** convert `kreely.model.json` plus the textures into the native renderer's player-body path
   (GX display lists/TPL, including `_AB`, or the Bevy mesh path). Port `retarget` (rotation-only, root scale) and
   the hand-pose layer into the runtime's pose sampler. Coordinate with the renderer sessions first.
4. **Selection:** a character-select or debug toggle that swaps only the body. Gameplay, hitboxes and timing stay
   pl0030's until Kreely has his own move data.
5. **Polish:** paint a real AB shadow pass and a region mask, add a per-clip hand-pose table (open hand for grabs),
   foot IK for the 1.5% leg difference, and contact-aware arm IK for the hand-to-head clips.

## Native Rust slice (2026-11-08)
- Playable in `mw-play` (pure Rust, `crates/mw-game/src/play.rs`): Insert swaps Jack <-> Kreely live, C toggles color.
- Kreely's pistol works there: hold RMB to aim (eye-level Leonardo aim loop), LMB fires, release lowers into the pistol
  idle; transitions use the explorer's blend lengths (20 / 10 / 22 frames). Overlay sampling is ported to
  `mw_sim::character` and matches the explorer reference poses (golden test).
- Not yet in Rust: muzzle FX/audio, shot hits (phase 3/7 of `research/NATIVE-PORT-GOAL.md`), the Jude one-handed set
  (staged, not yet bound to inputs).

## Native fight (2026-11-08, later)
Kreely plays the whole c201 RinRin fight in `mw-play` (pure Rust, `crates/mw-sim/src/fight.rs`):
- Moves: Jack's translated cPl0000 locomotion, jump, A string, rush / FINISH_BLOW, finisher (paired motions retargeted
  rotation-only), damage reactions, death and continue. His secondary is the pistol instead of the chainsaw.
- Pistol: hold RMB to aim (camera direction; Tab lock-on keeps RinRin ahead), LMB fires; each overlay shot frame casts
  a ray against RinRin's em0440_00.emh damage volumes and goes through the native intake with the HANDGUN row
  (0x61, power 20): guard / evade rolls and the dm_body reaction apply as for any attack.
- On a finish-ready RinRin RMB swings (no chainsaw needed) to start the finisher.
- Evidence: `evidence/native-play/script-kreely-finish.*` and `kf*.png` (MW_RIN_HP=60 debug start: pistol hits,
  punch string, FINISH_BLOW at <= 10 HP, finisher, Victory at tick 2139); `script-kreely-pistol.*`.
- Open: Kreely's own clash entry (the clash stands in on chainsaw swings; Kreely reaches it only via the finisher
  path), pistol muzzle FX / shot audio, his own grab / throw set.

## Pistol system (2026-11-10)
Added, not native: MadWorld's player has no firearm, so the gun's rules are ours, built to sit in the game's own
systems (state slot, damage intake, EST effects, SE, HUD). `crates/mw-game/src/play.rs` (`GunAmmo`, `GunEvent`):
- **Aim / fire:** hold RMB (the secondary / chainsaw slot) to raise Leonardo's aim loop; LMB fires a round from the aim
  instead of playing the 2 s fire cut, at most 5 a second (`FIRE_GAP` 6 frames). Each round casts the existing ray into
  RinRin's em0440_00.emh volumes through the native intake (HANDGUN row 0x61, power 20, guard/evade rolls), pitched with
  the camera. Recoil: the explorer's 9 deg muzzle climb, split over the upper arm (bone 7) and wrist (9), decaying x0.6
  a frame.
- **Ammo:** 12-round magazine (a wp0671-style semi-automatic), 48 carried, at most 96. An empty magazine dry-fires and
  reloads by itself; R reloads early. A reload is 40 frames on the lowered idle; the magazine seats at frame 28 and the
  pistol cannot fire until it ends. Landed melee hits put 2 rounds back in the reserve [inferred economy].
- **Events:** each game frame publishes `GunEvent`s (shot with muzzle, direction, stage impact or hit; dry fire; reload
  start / done) for effects, sound and the HUD. `MW_PLAY_LOG` lines carry `Pistol*` and `Ammo { mag, reserve }`.
- **Effects:** the muzzle blast and a stage-impact spark spawn at world points (`Owner::At` in play_fx.rs); a hit bleeds
  through RinRin's normal reaction effects.
- **Native FX / SE / HUD (same day):**
  - Muzzle: Jude's single-shot flash em0400 EST 31 (em0400_0105's op-16 cue; TUV 128 starburst + smoke), staged by
    `tools/stage_effects.py` (bank em0400), lifted by its 0.4 gun-bone offset and scaled x0.18 for the pistol [inferred].
    Missed rounds spark on the stage (core EST 12 at the ray's stage hit).
  - Sound: Jude's gun block 4201 (`se/scene_mix0125.rsp`, staged by `tools/audio/stage_se.py`): 4201:0 shot, 4201:7
    dry fire, 4201:200 magazine out, 4201:201 seat/slide [picked from waveforms; Jude never reloads].
  - HUD (`crates/mw-game/src/play_gun_hud.rs`): common program 37 (the native weapon window: cIdWeaponGimmick) with the
    pistol in the chainsaw picture slot (widget 22; 21 hidden) and its gauge as the magazine; common program 57 (the
    native "x N" counter) with the group-15 digits as rounds in the magazine and the pistol in place of the face.
    cIdTargetSight's art (UV group 21, absent from core_us; cut from the archived core page by `tools/stage_gun_ui.py`):
    ring sight while aiming, locked sight with lock-on, and the native RELOAD! banner while reloading or empty.
  - Not native: no reload motion exists on disc (the lowered idle stands in).
- **Reload, tracers, aim, deflects (2026-11-10):**
  - Reload: authored (no disc reload exists), `crates/mw-game/src/play_reload.rs`: two-bone IK on both arms in the
    chest frame over the 40-frame reload (gun up and rolled in, magazine out, left hand to the belt and back, seat at
    28, slide rack, ease out).
  - Tracers: `play_tracer.rs`, a copper bullet with a generated smoky trail from the posed pistol muzzle (prop bone 1600 +
    the gun mesh's forward end) to where the round ended; 75 m/s visual speed; deflected rounds continue as a ricochet.
  - Aim: rounds converge on the reticle (camera ray through the screen centre picks the aim point: RinRin, the stage, or
    40 m out).
  - Hits: RinRin takes pistol rounds through her native intake (dm_body / native guard / evade); added fan deflect
    (`Shot::Deflect`): when she faces the shooter and is free, 30 % of rounds are batted off the fan in her right hand
    (guard motion, spark at the fan, ricochet tracer); her native guard roll also deflects. Street grunts flinch / are
    launched / die.
  - HUD: the pistol is drawn above the weapon window, muzzle clear of the cog.

## Melee moveset (God Hand retarget)

Kreely's punches, kicks, pistol whips and suplex are God Hand motions (Gene and the grab pair) retargeted onto the
pl0030 rig. How they were picked, exported, retargeted, corrected (upright / squared / levelled) and checked, and how
to repeat it with another explorer: [tools/godhand/README.md](../tools/godhand/README.md).
