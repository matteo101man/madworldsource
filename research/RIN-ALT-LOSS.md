# Rin Power Struggle: alternate loss ("fan scissors"), an added animation

**Status:** an added PC animation, explorer only. It is not disc data and not wired into the game.

## Original loss pair [native]
- Rin `em/em0440.dat#em0440_0113.mot` and Jack `em/em0440.dat#pl0030_04ad.mot`, 130 frames each at 60 fps.
- Rin's handler `fn_802956B4` loads the AA/AB/AC/AD pairs (see PLAYER-COMPETITION-API.md). AC/0112 is the observed win; AD/0113 is the loss.
- Viewed together in the explorer:
  - **f0–30:** Rin sits on Jack.
  - **f35–60:** handstand on her fans (her spine is rotated about 180°).
  - **f85–95:** launch into the air.
  - **f100–110:** feet-first landing on Jack's torso.
  - **f110–130:** recovery.
- Jack lies on his back the whole time. He jolts at f40 and twists at f90–110.

## Alternate clips [added], v2
`tools/anim/rin_scissor_loss.py` (with `tools/anim/rig_fk.py`) builds two 166-frame clips. It uses the original pair plus
the struggle loop `em0440_0111` and writes them to `exports/madworld/custom/clips/`
(`em0440_0113sc.mot.json`, `pl0030_04adsc.mot.json`, `index.json` with sha256s).

The v1 problems the user reported:
- **She was still:** v1 held one frozen pose.
- **The fans only twisted:** v1 rotated them at the wrist instead of sweeping them through each other.

v2 builds her with FK/IK in the explorer's own three.js conventions. The Python FK matches the explorer's bone positions exactly.

Rin, body layer:
- **f0–24:** original start (f0–16).
- **f25–117:** the struggle loop `0111` (51 frames, seated at the same hip height), so she keeps moving.
- **f118–138:** hop-off blend to the original landing crouch (f121), with a 0.55 arc and a bone 0 offset of x +1.1 / z +0.35.
- **f139–152:** original recovery (f121–129).
- **Root:** the root (−1) X/Z is held constant, so the inferred "follows receiver root" option does not drag Jack.

Rin, procedural layers on top (`rin_scissor_loss.py` constants):
- **Upper-body lean (bone 1, world X):**
  - She rears back (+0.35, f25–40), then dives forward (−1.0 rad by f42).
  - An extra drive into the cut at f45–58.
  - While squeezing she bobs (±0.10) and grinds side to side (world Y ±0.18). Released by f116.
- **Head (bone 4):** sways, and tosses back after the cut (f53–90).
- **Arms:** two-bone IK (7→8→9, 12→13→14) puts her hands beside Jack's head at about y 0.26–0.34.
  - The hands are ±0.50 apart when open and ±0.30 when closed. The elbows point out and up.
- **Blades:** each blade runs along local −X on hand 9 and +X on hand 14, about 1.76 long.
  - They're aimed in world space, sloping down 0.22.
  - f29–45: they swing out to 80° from her forward axis, wide open to each side.
  - f46–52: an accelerating sweep inward. The blades cross each other at Jack's neck and continue all the way through to −72°, fully crossed, each blade to the opposite side.
  - Then a small squeeze oscillation, and release by f117.
- **Measured contact:** at f52 each blade passes x = −0.03 (his neck) at about y 0.20, z −0.56. His neck centre is (−0.03, 0.14, −0.52).
- **Floor:** the blade tips dip to about y −0.1. The original clip's tips already go to −0.3.

Jack:
- **f0–24:** original start.
- **f25–47:** blends to his pinned pose (f30).
- **f48–61:** the original jolt (f36–49), synced to the blades passing through. His head snaps back.
- **f63–107:** decaying twitches in his arms (7, 8, 12, 13) and shins (17, 21).
- **From f62:** his head rolls aside and he stays limp.
- **Neck stays put:** his neck stays at about (−0.03, 0.1–0.14, −0.52) the whole time.

## v3 changes (user feedback: she flew off right away, got up to the side; Jack went limp too early; detach the head)
Both clips are now 232 frames.

Rin:
- **Seated longer:** she stays on Jack until f175. The fans release at f104–120, then she idles on him, still running the struggle loop with her head swaying, from f120 to f175.
- **Get-up:** her own clip `em0440_0022` (deep fan crouch to standing), f10–29, played over f184–205 and yawed 180° in world space so she keeps facing Jack's head. Her bone 0 steps back +0.95 in z (behind her, over his legs) during f180–204, with a small 0.22 hop at f184–200. Then she holds her standing pose.
- **Removed:** the old sideways hop and the borrowed original recovery.

Jack:
- **Fights until the cut:** f25–45 now run his half of the struggle loop (`pl0030_04ab`, phase-locked with Rin's `0111`). Before, he was frozen, which read as limp before the blades closed.
- **Cut at f50:** the blades pass through each other. His body plays the original jolt (f48–61), then convulses: arms and legs flail through f60–105, fading so he is limp by about f100.
- **Decapitation:** from the cut, head bone 4 (with jaw, cigar, brows and eyes, bones 24/25/26/40/41) gets its own position track.
  - It pops 0.24 off the stump, lands, rolls about 0.55 away from Rin (toward −z, slightly +x) with a matching roll rotation, and settles by about f86.
  - His head meshes (`head_SB`, goggles, etc.) are skinned only to bones 4/24/26, so they detach cleanly. Only 17 `body_SB` seam vertices blend 3/4 and stretch slightly.
  - The game's own cut caps, `danmen_kubi_DA` (on bone 4) and `danmen_tate_DA`, are already visible in the explorer, because it shows all mesh parts.
  - Blood is not authored.

Fix: channels that exist in only some source clips now default to the bind offset (positions) or 0 (rotations) instead of 0. Before, the union with 0022's position tracks collapsed Rin's skeleton.

## v4: detach timing (user-chosen)
- **Detach moved:** Jack's head detach and convulsions are now at **f118.7** (`DETACH`), as Rin releases the fans. The blade cut through his neck is still at f50 (`CUT`).
- **f30–115:** Jack keeps struggling under the closed blades, on his struggle loop. The head stays attached (neck-to-head distance stays at the bind 0.063).
- **f116–119:** he blends into the original jolt at f36–49, which plays from 118.7.
- **f129–174:** the flailing, fading to limp by about f175.
- **f119:** the head pops off. It's 0.23 from the neck at f125 and 0.61 at f160, where it settles.

## v5: the attack is f0–36 (user-chosen)
The user found that the attack reads as complete at f30–38. So the kill now happens there, and the rest is linger and get-up. There is no second open and close.
- **Rin's attack layers stop advancing at `ATTACK_END = 36`:** the lean, the wind-up, the arm IK reach and the blade aim all hold their f36 pose (hands about y 0.5, blades down on his neck). The struggle-loop body keeps moving with a bob and grind (f42–106) and a head toss (f40–78).
- **Release:** she eases out of the attack pose into her seated idle over `RELEASE = f100–120`. The get-up at f176–205 is unchanged.
- **Jack:** `DETACH = 34`. The jolt starts at f34, the head pops off at f34–36 (0.12 from the neck at f36, 0.5 at f60, settled at 0.61), and the flailing at f44–89 fades to limp by about f90.
- **Unused:** the scissor-sweep constants (`OPEN_DEG`, `THROUGH_DEG`, `CUT`) stay in the script, but the frozen timeline no longer reaches the f46+ sweep.

## v6: smoother motion, clean decapitation
Smoothing (`rin_scissor_loss.py`):
- **Attack clock:** frames 20–38 now map onto attack time 20–36 through smootherstep, so the attack pose is reached with zero velocity. Before, it froze mid-motion at f36, which caused the snap.
- **Longer blends:**
  - Start → struggle loop: f18–34.
  - Release: f95–128.
  - Loop → get-up: f168–186.
  - The get-up itself is eased over f186–212, with a 0.95 step back over f178–210.
  - Jack: start → loop over f18–30, loop → jolt over 6 frames, jolt → pinned over 27 frames. The flail amplitude now ramps in.
- **Final curves:** every channel of both clips gets a Gaussian low-pass (`SMOOTH_SIGMA = 1.6` frames).
- **Measured:** after this, the fastest hand movement is in the get-up's own fan flourish (about 0.145 per frame, near f199), not in the attack.

Detachment (explorer, `severActor` in `app.js`; pair entry `sever: {main: [4]}`):
- **Re-skinning:** on a cloned copy of the geometry, every vertex that blends the severed subtree (bone 4 and its descendants: jaw, cigar, brows, eyes) with the rest of the body is assigned wholly to its dominant side and renormalized.
- **Dropped triangles:** triangles with vertices on both sides are removed. On Jack: 17 vertices reassigned and 35 triangles dropped, all in `body_SB` at the neck.
- **Result:** the head separates as its own piece, leaving an open cut at the neck, with no stretched skin.
- **Scope:** this is an explorer feature for added animations. The game will need the same split, or its own dismemberment path (`danmen_*_DA` cap meshes), when this is wired in.

## Geometry notes [measured in the explorer]
- **Fans:** seven `oogi_0x_KB` meshes, one rib of each fan per mesh. Each fan is skinned to rib bones 100–114 under hand 9 and 115–129 under hand 14. In bind pose a fan runs from the hand (x ±0.74) to x ±2.5. The ribs are not animated in the source clips, so the fans stay closed and rigid.
- **Facing:** in the paired frame Rin faces −z, toward Jack's head.
- **Debug hooks:** `__mwOffsets`, `__mwView`, `__mwBoneWorld`, `__mwPoseAt`.

## Explorer changes (`D:\Games\gamesfordecomp\exports\madworld`; backups are `*.bak-before-rin-pairs`)
- **Pair table:** paired playback is now a table (`PAIRS`) covering Elise and Rin's AA/AB/AC/AD, plus the alternate `rin-sc`.
- **Follow option:** an inferred checkbox, "Performer follows receiver root". It is based on cEmBoss::moveCatchedPos. The exact attach offset and facing are unverified.
- **Custom clips:** they load from `custom/clips/index.json`, bank `custom/clips`, fetched by `url` with a sha cache-buster.
- **Debug hooks:** `__mwOffsets`, `__mwView(pos, target)`, `__mwBoneWorld(role, boneId)`, `__mwPoseAt(t)`.

## Open
- **Contact precision:** this is a visual fit at the explorer's root alignment, which is inferred. In-game placement through moveCatchedPos is unverified.
- **Effects:** blood and the decapitation or other neck effects are not authored. The original loss has no blood effects in the clip data either.
- **Game wiring:** the clips are not in the game. That needs an MOT encoder (explorer JSON → .mot) plus a choice-of-loss hook in Rin's handler (`fn_802956B4` selects AD/0113).
