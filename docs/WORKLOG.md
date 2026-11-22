# MadWorld RinRin native reconstruction

Owner disc: RZZE8P revision 0. Work in progress; no claim of a completed playable port.

Original extraction and explorer: D:/Games/gamesfordecomp/exports/madworld. Runtime, staging tools and generated evidence live here on D:. Research logic is maintained as typed graphs with focused indexes/vocabulary/pseudocode. Native-vs-inferred-vs-unknown evidence stays explicit.

No DLSS dependency is planned. The existing Bevy 0.20 cache is reused.

The playable pure Rust fight is `cargo run -p mw-game --bin mw-play`. It now
starts with the original RinRin intro, then performs the boss-entry cleanup
wait and fade into combat. Hold Enter (1.2 s, as the recomp), Backspace or right
mouse skips the intro (the original B/Menu Back action); Escape opens pause after control is released.
Normal completion retains the event's BS04C music; an early skip starts the
original shorter BS04 room track. Original pause pages are integrated, while
the full HUD, challenge menu, save/main-menu flow and remaining fidelity work
are tracked in [the port gap list](research/NATIVE-PORT-GAPS.md).

The original TAKE THE CHALLENGE? confirmation now opens on launch
(`MW_CHALLENGE=0` skips it). Left/Right or A/D selects Yes/No; Enter or left
mouse confirms. Backspace, Escape or right mouse selects No. Yes starts the
intro. No reopens the prompt because the roaming-world return is still unported.

One mw-play runs every stage: started without `MW_STAGE` it opens a stage select (Up / Down or 1-5, Enter), which
relaunches it on the pick; F9 in a fight returns to the select. Scripted and inspection runs (`MW_PLAY_*`,
`MW_DIRECT_FIGHT`) skip the select and default to RinRin, and `MW_SELECT=0` skips it too.

Kreese Kreely is the default player (`MW_PLAY_KREELY=0` plays Jack). T taunts (state 0x54): street grunts within 15 m
get angry and rush in. Locked on, the camera goes side-on to the two fighters (after Way of the Samurai's lock) and he faces the target,
strafing around it; unlocked, an attack turns him to the
nearest enemy within 5 m. Every shake meter (dizzy, clashes, finishers, the bosses' struggles) is mashing A / D; X is
the head-butt while holding a grunt. Kreely's grab in every fight: holding A on a zako is the carry (walk him about,
F throws him, X head-butts), and Q while carrying turns it into his suplex on the grunt in his hands; RinRin still
takes the suplex straight from the A hold. While he holds someone or something, a panel in the bottom-right corner (above his gauge, drawn as the
pause menu's AREA DATA box: play_hold_ui.rs) lists the keys for it
(`MW_HOLD_UI=0` hides it). The pistol is run-and-gun by default (one-handed, full run pace, the free arm
swinging with the run, moving shots stray a little; after Max Payne 3's one-handed pistol); F6 switches back to the
aiming walk (`MW_RUN_GUN=0`). Every hit bleeds the same way (impact streaks, spray, a burst on heavy blows, chunks and
a gush on kills) and leaves blood pools on the floor (play_gore.rs). `MW_STAGE` picks a stage directly (`rinrin` is
the default):
- `elise`: room c303. ev33 then ev34, then Elise with her bats, the coffin clash (X shakes, F / Q swing) and her
  finisher (research/ELISE-BOSS.md). Her wing strike and somersault move on their clips' root travel and hit with her
  posed limbs (wings 0.75 m, limbs 0.55 m), so they land as they look. Pistol rounds leave holes and blood on her and she
  keeps going (a flinch blended over her move); a lost clash's bite drains 20 HP with blood at his neck and a red screen
  pulse per drain. Her hide crypts are reached as natively: by jumping up the crypt tiers (col_obj: 1.79 m, then 3.56 m;
  the port now stands on col_obj in c303 and helps a jump that reaches a ledge's lip onto it); the side coffins can be
  struck from the floor. [added] Each hide, bats leave the swarm in groups, fly to points around the centre, swirl, dive
  and burst into zako (the em0101 grunts for now): 3 the first hide, one more each hide after, up to 5 (`summon_*` in
  em0630.json). `MW_ELISE_ATK=attack|somersault|volley|dragon|hide` runs one of her moves in front of the player.
- `baron`: room c503. ev50, then the Black Baron with Mathilda and the three oirans at the ropes. His clash chain
  (F / Q swings, X fills the last meter) runs on the original pair cameras; ev51 plays after his defeat
  (research/BARON-LEVEL.md).
- `mathilda`: the added variant: Mathilda is the boss (no Baron). Her intro skips, fades and enters the room as the event
  intros do, with subtitles in the game's own message font (`python -I tools/ui/stage_subtitle_font.py`) and her jaw
  moving with her lines (no voice yet); she drops into the ring from above, lands and walks up to him. Her attacks are
  Jack's own two-handed bat swings (pl0030_14xx) on her rig, sampled rotation-only so her proportions hold; RinRin's
  clips fill only her guard, damage, down, dizzy and the drop. Pistol rounds leave entry / exit wounds on her. A
  heavy swing that lands pins the player under her bat (her beat-down pair, in place, RinRin's clash camera): held X
  breaks free. Inspection: `MW_MATHILDA_CLIPS=1` plays each of her clips, `MW_BARON_TEST=clash` opens the pin.
- `street`: Asia Town's grunts.
- `town` [added, test]: Asia Town free roam by day (play_town.rs). No waves: 26 townsfolk walk a baked walk grid (A*),
  stand about and chat; they can be hit, grabbed and killed, run from blood, gunfire and swings, and walking into one
  knocks him aside with a typed-out speech bubble in the game's UI look. Substories: zako picking on a civilian; their
  exchange shows in bubbles within 10 m, ENTER within 4.5 m plays a letterboxed step-in scene and starts the fight; a
  clear thanks you and a new one starts elsewhere. The townsfolk are the ending crowd (em0915 / em0925 / em0935 /
  em0955), textured from their siblings (`python -I tools/town/stage_civilians.py`); walk grid `tools/town/stage_nav.py`;
  their clips `tools/town/stage_civ_motion.py`. Their stand / walk / run are `anims` in assets/defs/actors/civ.json:
  audition clips in the actor editor's `civ` page (Edit civ, or `MW_STAGE=town MW_EDITOR=civ`). Inspection:
  `MW_TOWN_LOG=1`, `MW_TOWN_CIVS=n`, `MW_TOWN_SUB=n` (start by substory n), `MW_TOWN_STEPIN=tick`,
  `MW_TOWN_LINEUP=clip,clip,...`.

VR (added, play_vr*.rs, [notes/VR.md](notes/VR.md)): `play-vr.cmd [stage]` (or `MW_VR=1`) plays Kreely in first
person on a Quest 3 over Link / Air Link (any OpenXR headset): his own rigged arms on the controllers, punches by swing
speed, grabbing and throwing grunts, townsfolk and objects, Kreely's grab on RinRin and the bosses, climbing and
vaulting ledges, the pistol drawn from the hip, a floating HUD, a theater screen for event scenes, and a VR menu with
settings and controls in the pause menu's look (left menu button). `MW_VR=sim` is a desktop VR simulator;
tools/vr/xrsim is a headless OpenXR runtime for testing the headset path.

The actor editor (added, play_editor.rs): the stage select's ACTOR EDITOR list, or `MW_STAGE=<stage> MW_EDITOR=<id>`
(em0630 Elise, em0440 RinRin, em0623 the Baron, em0610 Mathilda, em0101 the kung-fu grunt). The actor stands in front
of the player with its AI held, and a menu in the game's own lettering (the brush capitals of core_us, the subtitle
message font) plays its attacks, hurts, deaths, taunts, defences, power struggles won or lost, grabs, finishers and the
last-life execution through its own module (so the clip's sounds and effects, the blood and stains, the voices and the
pair cameras are the fight's), any clip in its bank, every voice line of its room (the announcer's pick, or each
recording), and its camera clips, each played in its own scene (the entry that films it, or the clips it was made
around). Everything is named in words from each definition's `names` (clips, voice lines and what triggers them, cameras,
states and their phases). Orbit / game / camera-clip views (V, right drag), pause and slow motion, the AI on
(I), the keys handed to the player (F2) to fight it or play a pair by hand. Entries are data: each definition's
`editor` page (assets/defs/actors/README.md says how to add entries, drive it from `MW_EDITOR_RUN`, and wire a new
actor in).

Voice lines traced to their native callers (cEnemyBase::playVoice 810669B0 / PlayVoiceStream 8026B558):
- RinRin's room lines 0x53-0x5A are the announcers' commentary on her moves, now staged (`tools/audio/stage_announce.py`,
  c201) and requested where em0440 asks for them: 0x53 as a backfist, shoulder ram or giant slalom starts, 0x54 the fan
  throw, 0x55 the kamikaze dive, 0x58 when she takes damage, 0x59 finish-ready, 0x56 / 0x57 when Jack wins / loses the
  clash (0x5B, the clash opening, has no take in the pack; 0x5A is not hers). One line plays at a time and a waiting
  one drops after 90 frames, as natively.
- The kung-fu grunt's flee line 0x1002 plays as a frightened grunt turns to run (fear_to_escape 8102D454). His death
  by explosion (0x1007) is staged, waiting for oil drums and dynamite; 0x10A0 belongs to another enemy.
- Elise: 0xAC is asked for as she settles into her coffin (no take in c303's pack); 0xA2 is the church room's line
  for a nave pillar's first break (below); 0x9E, 0xA1, 0xA6 and 0xAA are recorded but nothing in the game asks for
  them.

The clash cut-in (play_cutin.rs): before each power struggle the comic panels play (the boss's portrait upper left,
Jack's lower right, speed lines and "VROROOOWW" in the brush capitals over the darkened frame): program 11 of the room's
idpac on layer 12 (the cockpit's cut-in widget, fn_8022B374 / fn_8022B184), and the fight stands still until its widget 1
animation ends (40 frames), as the native stop mask holds the world. RinRin's at her clash entry, Elise's when his
pull-out reaches frame 25, the Baron's at his clash grab; none in Mathilda's bat combo (as natively). The Baron's room
pack (idpac_c503_us) is staged now (`tools/ui/stage_native_ui.py`). The portrait panels clip their portraits (the kind-7
widgets as windows, `native_ui::sprites_ex` [inferred]). `MW_CUTIN=0` turns it off, `MW_CUTIN_SHOTS="frame:png"`
screenshots it.

Elise's church breaks (play_breakables.rs): the eight nave pillars (bl005c) stand whole now (the room's scenery holds only
their stumps and arches) and one blow brings one down: its five shaft pieces fly and land (bl005c EST 1 / 800, SE
4209:205 / 206), and 120 frames later fade. The first break of each of N001..N006 plays the room's line 0xA2 (the coffin
controller fn_80076054; the two z = -10 pillars are misnamed in ObjTbl and break silently). The ten pews (bl005d) break
into their broken shape and fade 3 s later. Objects: `tools/stage_static_object.py bl/bl005c.dat` (rigid models: one node
per break part) and `tools/stage_object_color.py` (their added-color variants from the color pack); `MW_BREAKABLES=0`
turns them off, `MW_BREAK_TEST=n` stands the player at object n.

Other native pieces ported here: the street grunts' damage clips follow the punch / chainsaw string (getComboDamageMotion
810177c4: 10a5 -> 10a6 -> 10a7 -> 10a8, chainsaw 1026, the PC launch 10a0, CCC 104d, CCCC 1102); a blow on a downed grunt
plays 105c; a fleeing grunt that runs into a wall cowers (1109, 110b) and stands up (103a); the chainsaw arm plays its own
wp0600 clips in Elise's and the Baron's pairs (it stayed in its bind pose); the Baron's win line is 0x11A on the player's
last life (0x115 otherwise).

The Baron's shoryuken into Mathilda's bat: his clip (pl0030_1055) starts at the launcher, 15.5 m off, so it is now
placed as the Mathilda boss's pin places it (`bat_anchor`): he comes down at her feet and her third blow lands on his
hips, its camera (c493) riding the same origin; he stays where he lies afterwards.

Cloth, hair and breasts on the other bosses (play_springs.rs): the springs built for RinRin now ride every boss through a
per-actor `Spec`, asked for with a `SpringRequest` once a module binds its joints. Elise (em0630): breasts 25 / 26, the
cape down her back (42-46), her two long hair tails (50-54, 55-59) and two side locks (60-62, 63-65). The oirans
(em0960): breasts and their two hair tails (30-32, 33-35). Mathilda (em0610): breasts. The Baron (em0623): the chain off
his chest (50-52); his coat is mesh cloth in the original, with no bones to swing here. [added, as RinRin's: the
original's own cloth (cByCloth, setClothParam 800788CC) and hair (cHair) parameters are not used.]

Picking things up (play_props.rs, `MW_STAGE=street`): the street's placed objects (case2/c201.dat ObjTbl, 37 of them in
`assets/native/world/street-props.json`: crates, baskets, bins, garbage bags, the madam heads, the turtles) stand in the
street. The same A hold that grabs a grunt picks one up when no grunt is in reach (the player states 0x1A pick 021a, 0x1B
hold / carry 021b / 021e, steered as the grunt hold): let go of A to set it down (021c), a throw sends it flying (021d)
and it knocks down the first grunt it meets (OBJ_THROW 0x107, power 20) and breaks, and the vertical throw next to a
grunt slams it over his head (0218): blinded, he reels under it for 5 s (his covered clips 1513 / 1518 are not staged,
so the stun stands in) and it drops when he recovers. There are no oil drums, tyres or dynamite on these stages.
Environmental kills: a grunt launched or thrown into the spike wall (bl0181) or a meat hook (bl0106) sticks on it (motion
stuck 0x46: 101c), slides off (101d) and lands alive (101e) or dead (101f), with the wall crash's 20 damage and the
impale blood (core 0x1A5 / 0x1A9). `MW_PROPS=0` turns the objects off; `MW_PROPS_TEST=n` stands the player at object n,
`MW_IMPALE_TEST=n` launches a grunt into impale zone n (0 the spike wall), `MW_PROPS_LOG=1` logs the object events.

The chainsaw split (play_street.rs): a grunt at 気絶体力 40 % HP faints once (faint_away_stand: em0110_1120, 気絶回復
4 s); Jack's chainsaw swing on a fainted grunt is the finisher check (fn_802C3C0C), and one roll in six (rand % 6 == 5)
is the split; rand(0, 1) picks the cut. Jack plays the paired clip (pl0030_0403 vertical / 0404 waist), the grunt stands
on the clip's 1024 offset facing him and becomes half A, and a copy of him spawns as half B (dmSpine / onSeparate):
- vertical (CHAINSAW_TATEGIRI 0xCA): halves 1130 / 1131; at frame 15 half A keeps every triangle at bind x <= 0 and half
  B (unseen until then) x >= 0; both show the x = 0 cross-sections (`_01_DA`, `_02_DA`) from both sides;
- waist (CHAINSAW_YOKOGIRI 0xCB): half A 1132 is the upper body flung off (everything but the trousers, and the waist
  face under it), half B 1133 the legs (the trousers, rebound from bone 1 to bone 15 so the waistband follows them, and
  the face on top); the cut lands after frame 17.
The halves keep their clips' root travel, the cut sprays (cutWaist 417 / 416, the seam's 1005s) and spurts for 3 s, the
grunts around are frightened (9.5 m) and the halves lie as corpses. The other five rolls are chainsaw executions not
ported yet, so the swing lands as an ordinary hit. `MW_SEVER=1` (or `vertical` / `waist`) always splits;
`MW_SEVER_TEST=vertical | waist` faints the nearest grunt in front of the player and splits him; the actor editor's
em0101 has "Fainted" and both splits under Hurt.

The endings: after the room clear's results are confirmed, the story event that follows plays (play_intro.rs, asked for
by play_clear.rs): Elise's ev84 (in the scene 0xB14 backdrop, room ca03) and the Baron's ev93. ev84 can be skipped (hold
Enter, Backspace, right mouse); ev93 cannot, and shows no skip hint (the event player's no-skip list: ev18 / 34 / 43 /
51 / 93; the entry events in it no longer show the hint either). Both events carry their own comic-panel atlas
(`"atlas":"own"` in their ui.json); one ev93 program is left out (its key order is one this IDT reader refuses), and
ev84's numerals (message tokens 0-9) are not drawn. Staging: `tools/events/stage_intro.py`, `stage_audio.py`,
`stage_ui.py`, `stage_props.py`, `stage_messages.py` for ev84 / ev93, the ca03 backdrop with `MW_ROOM=ca03
MW_SPAWNSET=none python -I tools/stage_arena.py`, then `tools/export_arena.mjs` and `tools/stage_arena_color.py`.
`MW_ENDING=0` turns them off; `MW_ENDING_TEST=1` plays the room's ending straight away (with `MW_INTRO_FRAME=n` from
frame n). The Baron's clear line 0x1027 has no take in c503's pack (c303 has one), so it is silent, as it is natively.

Elise's brood, the vampire zako (tools/vampzako/build_vampzako.py, as Kreely is built on Jack's rig): three
user-provided women (C:/Users/Matteo/Desktop/newmadworldchars/vampirezako1-3, unrigged Tripo meshes) on the kung fu
grunt's em0101 skeleton. The grunt's joints are carried into each woman's proportions (the Kreely landmark warp, with
her armpit and crotch measured on the torso), she is bone-heat skinned in Blender (on a merged and, if need be, turned
copy where Tripo meshes defeat the solver), and the port keeps her own bind offsets under the grunt's rotation-only
clips, scaling the root by her hip height (`vampzakoN-rig.json`). Her mesh is cut into the grunt's part numbers (2 the
head, 3 below the waist, 4 the rest), so a head can come off and both chainsaw splits work on her. The source look is
her own black-and-white texture; the colour look is a gradient map per tone (warm skin, black latex with a grey sheen,
black hair, red lips) [added]; skin and clothing are told apart by how dark her surface is over ~6 cm around each
texel (her latex is black with white highlights, her skin white with ink lines), not by the texel alone. Her hip, knee, ankle and toe joints sit on her own leg line (the grunt's knee is 11 cm behind
his), and each limb bone carries a bind correction (`correction` in the rig: the turn from her bone to the grunt's) so
his rotation-only clips bend her legs as they bend his: local = C(parent)^-1 * clip * C(bone) (`MW_VAMP_NOCORR=1`
shows her without). Thirty finger bones (0x600+, Kreely's finger detection) are skinned in the heat pass and rest
curled in a kung fu claw (their GLB rest rotation; the grunt's clips never move them). Added bones spring (play_springs.rs `VAMP`): breasts 250 / 251 and buttocks 200 / 201 (the hair
rope bones 260-267 are built but unweighted). They move, walk, run, fear, siege and attack with the grunt's clips and AI for now (no
moveset of their own yet), and cry with the female voices on the disc [added]: vampire 1 with Elise's (8122: hurt 20 /
21 / 24, death 25), vampire 2 with RinRin's (8117: hurt 20 / 21 / 22, death 24, her collapse), vampire 3 with both;
the grunt's flee line (0x1002, a man's) becomes a hurt cry. Elise's drops bring them in turn (`MW_VAMPZAKO=0` sends the kung fu grunts again); `MW_ZAKO_TEST=1`
drops three in front of the player at once. In the actor editor they are an actor of their own, `vampzako`
(assets/defs/actors/vampzako.json: "Edit vampzako" in the stage select, or `MW_STAGE=street MW_EDITOR=vampzako`), its
Bodies group switching between the three women (`MW_VAMPZAKO_ED=2 / 3` starts on another); em0101 is the kung fu grunt
alone again. Their hair rides on their heads (no hair ropes: `HAIR_ROPES` in the build); breasts and buttocks spring. Rebuild: `python -I tools/vampzako/build_vampzako.py [1 2 3]`.

The vampire zako rig, redone: the heat weights are now limited to the bones that can own each vertex (no limb takes
the other side, the arms stay off the torso sides, the shins stay off the thighs), smoothed over the mesh and mirrored
left to right; the head is rigid above the neck (the face no longer rides the neck and hair bones); the hair ropes
start at their first hair bone and end in an unweighted tip (a rope turned its first bone, the head, which made the
heads bob); triangles joining the two legs (her legs are fused where they touch in the T-pose) are dropped, so the
legs part cleanly; no foot correction (her heels stand as modelled). Her ink is sharpened toward clean lines (strongest
on the face: the Tripo faces hold only ~150 texels across, so they stay soft up close). `tools/vampzako/view_weights.py
N [--bone ID]` renders her weights (front, back, side). Skin pale again, clothes black. Her face and neck are always
skin in the colour look (the clothing test, now trilinear, had flipped inky parts of the faces to latex in 2 cm
blocks), her ink is black (the skin gradient's shadow end), her lips stay black ink as drawn (no guessed lip tint), and
her hair's highlights go to a grey sheen. `tools/vampzako/view_face.py N` renders her head close up, textured, beside the
concept sheet's face. Her head bone sits on her jaw line (read off the front profile of her face and neck; the grunt's warped
joint landed near her mouth, so her chin bent with the neck and the face twisted when she turned her head), her neck
bone halfway down her neck, and everything within her head (centred on the head mesh, not on the bone) is rigid on it. Her shoulders take the grunt's own skinning (his mesh warped into her proportions lends each shoulder vertex its
nearest point's weights, blended from the top of the chest to the elbow; the clavicles keep only the shoulder tops);
only hair hanging below the ropes' start and above the shoulder line is roped (crown hair swung into spikes, hair on
the shoulders pulled away from the skin), and the ropes stray at most 12 cm.

The voice mix: the bosses' lines play on a stream of their own beside the commentators' (they no longer wait behind
the commentary) at twice the gain, the commentary at 0.8. Esc's menu shows the port's own settings along the bottom:
[M] music and [C] commentary on / off (the commentators only; the bosses still speak), saved with the audio settings
(pc-controls.toml [audio] music_on / commentary_on). Commentary starts off when Kreely plays and on for Jack, until it is
set. Elise's (and the oirans' and Mathilda's) breasts use a stronger, looser chest spring (`BOUNCY`).

Kreely's voice (play_kreely_voice.rs; assets/native/audio/kreely, kreely.json: 40 lines in five sets) [added]: hurt
when a blow lands (55 %), teeth on a heavy one, near_death when his health crosses under a quarter, knockdown when he
floors RinRin, Elise or a grunt (60 %), defeat on a win or a boss clear (and 35 % of street kills); one line at a
time, each set resting a while after it speaks. `MW_KREELY_VOICE=0` silences him, `MW_KREELY_VOICE_LOG=1` logs.

The last-life finisher now waits: his fall and its slow motion play out, he lies still 2.5 s, then the boss walks up
and the finisher starts (the continue window holds back meanwhile). RinRin's is her alternate win, the fans closed on
his neck like scissors (the explorer's added em0440_0113sc / pl0030_04adsc, `tools/stage_rin_scissors.py`; the cut at
frame 204, framed by her fan cut's camera c4ad stretched over it). After it the camera turns once round his body
(6 s), holds 10 s (Enter skips), then fades to black and the continue screen. Commentary off now mutes RinRin's room's
0x53-0x5B too (the commentators on her moves, not her); the bosses' voiced cries (SE blocks 8117 / 8122 / 8127) play
1.8x louder.

Quick inspection runs: `MW_PLAY_SHOTS` now ends the run 20 frames after its last screenshot (`MW_SHOTS_EXIT=0` keeps it
running), and `fN:path` takes a shot at rendered frame N instead of game tick N, for shots while an event plays.

Blood that sticks (added): Elise's blows leave the core blood-stain sheet (tex 49, in getBloodColor) on her body where
they land and spattered over her face; a lost clash's bite leaves fang marks on the player's neck, on the side she bit,
and his blood round her mouth with drips down her chin once she stands up (bite-marks.png / mouth-blood.png,
`tools/stage_wound_decals.py`; `Fx::stain` / `Fx::mark`, projected upright).

Enemy and boss numbers live in actor definitions, `assets/defs/actors/<id>.json` (play_actors.rs, read at start;
`MW_ACTOR_LOG=1` lists them): HP, power, perception, outfit variants (HP, attack rates, power scale, dodges, the attacks
each picks from), named animations and random clip pools (laughs, threats), attacks (clip, hit windows, combo frame,
range, yaw window, power, ATHIT id), voice ids by event, and free `tune` numbers. em0101 (the kung-fu grunts, also the
placeholder zako elsewhere) and em0630 (Elise: HP, the em0630.csv values as `csv0`..`csv6`, attack powers `power0`..
`power4`, the summon, her voice lines) read theirs; anything left out keeps the native value. Other stages can drop grunts
in through `Street::queue` (position, yaw, outfit). A new actor is a new file plus the code that reads it; see
`assets/defs/actors/README.md`.

Stage sound effects keep each wave's own sample rate: the SE banks give every wave a tune (s8.24 semitones, record
+0x1C) and root key (+0x22), rate = 32000 x 2^(tune / 12) (`tools/audio/stage_se.py`), and a play's note against its
root sets its playback speed. They were all staged at 24 kHz before, which pitched Elise's voice up.

Clashes draw the original cockpit swing navigation (the two-arm struggle while the meter fills, then the swing
prompts; F down, Q side, held X shakes) and play the pair cameras: RinRin's from em0440.dat
(`python -I tools/stage_camera_clips.py em/em0440.dat assets/native/rinrin-camera.json`), Elise's and the Baron's from
their banks. Boss effects come from their own banks (em0630 / em0623 / em0610 in `tools/stage_effects.py`) through
their motion op-16 cues. `python -I tools/stage_boss_skin.py` fixes the Baron's, the oirans' and Elise's skin tones after
`tools/stage_actor.py`. Inspection: `MW_RIN_TEST=keen` opens RinRin's clash at tick 40,
`MW_PLAY_STATE_SHOTS=<state>:<prefix>` screenshots a player state.

Last-life execution (added, play_execution.rs; `MW_EXECUTION=0` turns it off): when the player dies with no continues
left, the boss finishes him before the game-over static, presented as the event intros are (in from black, the HUD
hidden, out to black) under letterbox bars, on the pairs' own camera MOTs as the boss defeats are (all four bosses; `MW_CAM_LOG=1` names them), the killing blow in
slow motion. (RinRin's own finisher now runs its cameras too: pl0030_13xx plays c3xx; `MW_RIN_TEST=finish` opens it.) RinRin's pair
camera was checked against the live recomp's shown camera (g_Camera, `MW_RIN_CLASH_ROWS=1` with
`tools/run_owned_gpu_test.py --clash-contact-fixture`, evidence/rin-clash-camera/): it runs its own clock from its clip's
start on the player's motion steps (c4aa at his frame less one, the c4ab orbit through his 51-frame hold loop over its
150 frames); 04aa and 04ab match update by update, 0.000 m eye and target. RinRin walks up and wins her clash pair with no prompt (0110 / 04aa, the 0111 / 04ab hold, 0113 / 04ad);
her second fan slash takes his head off. Elise (her struggle, then the bite 0204 / 0464: her bite at its first
hit-stop tears the head off), the Baron (the grab, then his exchange win 0486 / 0307: 0307's frame-80 blow knocks it
off) and Mathilda (the bat pin: her finishing blow crushes it) play their struggles the same way, each ending on its own
kill.
Elise's coffin clash places the pair as the player's clash code fn_802A03A0 does (GetThrowOffset bone 1024, the move to
a floor spot for the far / centre crypts, then (0, 0, 6) facing -z; she is placed from 0461's throw offset into the
struggle), and was checked frame by frame against the live recomp: `MW_NATIVE_ROOM=0x303 MW_TEST_ELISE_CLASH=1
[MW_TEST_ELISE_CLASH_AT=x,y,z,yaw]` on madworld-native boots room 0x303, hides her through her own accumulator, calls the
original startAtk on her coffin and prints ELISE_CLASH_ROW per frame; `MW_ELISE_TEST=keen MW_ELISE_STRIKE=1
MW_ELISE_AT=x,y,z,yaw,px,pz MW_ELISE_ROWS=1` runs the same in mw-play, and `python -I tools/compare_elise_clash.py`
compares them (recordings in evidence/elise-clash/, an in-place, the centre and a far crypt). All 623 updates of each
clash, pull-out to the bite's end, match within 1.1 cm in 3D (the port keeps feet on the floor; the original's objects
stand 0.1 above it). What it took: both bodies trail their roots by one update; his clips start an update after hers and
the phases move on his clip's last frame; the frame-23 move turns her offset by +dot and lands that update's step; the
bite's two hit-stop ramps; the get-up after 0464; no wall push on the pair; the meter starts at 12.05. Her pair cameras were checked against the recomp's shown camera too
(evidence/elise-clash/recomp-camera.jsonl): they ride his object (0.1 above the floor; c466 on his root itself) and c461
leans toward c462 with the meter; c466 matches exactly, c460 / c461 / c464 to a few millimetres apart from their cut and
hand-over frames. Inspection: `MW_CONTINUES=<n>` starts
with n continues, `MW_EXEC_LOG=1` logs the execution, `MW_ELISE_CRYPT=<n>` puts `MW_ELISE_TEST=keen` in crypt n.

mw-play uses the original follow camera by default, as the recomp does. F4 toggles the PC free
camera (mouse look; MW_PC_FREE_CAMERA=1 starts with it on), and the lock key (Tab) taps to recenter or
unlock and holds to lock on; Z toggles the lock. The lock holds any enemy (RinRin or a street grunt), Q / E
step it to the next target left / right while locked, and it holds the free camera on the target too
(decided 2026-11-10, replacing the F5 orbit-while-locked toggle). The cursor is hidden during play,
never locked.

Hit and blood effects run the original EST emitters from core2.dat and RinRin's em0440.eff
(`python -I tools/stage_effects.py`); their triggers were traced in the original with
MW_TRACE_EFFECTS (tools/play_traced_effects.cmd). See research/NATIVE-PORT-GAPS.md for limits.

mw-play starts in added color like the recomp (`MW_COLOR=0` or `[color] mode =
"original"` in pc-controls.toml for monochrome; C toggles). Characters and the
c201 arena both switch; `python -I tools/stage_arena_color.py` stages the arena
from the recomp color pack. The recomp's screen grade is not ported to mw-play. Details and native comparison evidence
are in [the challenge checkpoint](research/CHALLENGE-API.md).

Normal play now shows the original player and boss health bars, with live HP
and trailing damage fills. The remaining HUD consumers are still being ported.
Set `MW_DEBUG_HUD=1` for the previous state/control diagnostics.

EV21 contains no original dialogue-message track. Optional event-message
support is verified using the separate EV00 Man Darts introduction. For font/
message inspection, run `cargo run -p mw-game --bin mw-message-preview` with
`MW_MESSAGE_EVENT=00` and `MW_MESSAGE_FRAME=20` (or170/293/355). This displays
the original messages and glyphs; full Man Darts scene/world flow is separate.
Details are in [the message checkpoint](research/EVENT-MESSAGES-API.md).

For direct fight inspection, set `MW_DIRECT_FIGHT=1` (or
`MW_INTRO_PREVIEW=0`). `MW_INTRO_PREVIEW=1` retains source inspection with the
fight frozen; `MW_INTRO_FRAME` holds a selected event frame. These are debug
paths. The normal playable startup requires the locally staged intro assets
and both boss tracks from `python -I tools/audio/stage_bgm.py`.

The Rust workspace now separates native data (`mw-data`), sampled rig evaluation (`mw-sim`) and Bevy rendering (`mw-game`). The first runtime slice inspects all 70 motions from the RinRin bank on her 103-bone native rig. Arrow keys change clips, Space pauses, and C switches between source and added color palettes. This is an animation validation slice; it does not yet implement the boss encounter.

The native c201 arena is rendered at the original EMS spawn. Run the native rig/arena staging tools and `tools/stage_collision.py` (with the local Havok environment), then `cargo run -p mw-game`. Staging requires the local source extraction and decoder tools; assets are not distributed. The 60 fps inspection clock is not verified native game timing. GLB materials currently use the source atlas fallback, not the recovered GX program.

Native behavior evidence: `research/RIN-STATES-INDEX.md` indexes 32 original branch/update/end callback slots. `research/rinrin-behavior.graph.jsonl` links callbacks to native APIs and literal resource requests. `research/AUDIO-API.md` describes the 275 successfully probed source audio streams. `research/COLLISION-API.md` describes 353 native static bodies, original loader categories and Rust raw-surface diagnostics. State predicates, shared enemy/player mechanics, native contact/filter behavior, original audio routing and UI still require translation and runtime comparison.

The additional original-core frontend is built with `cargo build --locked -p mw-game --features native-runtime --bin madworld-native`, using `CARGO_TARGET_DIR=D:/GDecomp/build-cache/bevy-0.20`. It requires the locally built `tools/upstream-native-core/port/build/madworld_core.dll` and the owned extracted DATA partition. `MW_OWNED_PARTITION` can select another local partition directory. The executable and test target compile; all three PCM tests and the owned-core headless startup/close test pass. This frontend is undergoing further integration checks; its displayed graphics, audible output, gameplay speed, PC controls and original HUD prompt replacement are not yet verified. Original logic remains in the statically recompiled CPU core, while Rust/Bevy hosts hardware rendering, PCM playback and explicit window input.

Native frontend PC bindings now use WASD, left mouse/E attack/grab, hold right mouse chainsaw, Space jump, Shift dodge, hold X struggle, Q/middle mouse camera/lock, F/R directional strikes, Enter confirm, Escape pause and Backspace menu back. F1 shows the full optional controls panel. Mapping/report tests pass. Native movement, Rin lock-on, upward movement during jump and boss HP reduction are observed in headless replays; complete advanced actions and original Wii prompt replacement remain unverified. See research/PC-CONTROLS.md for exact scope. The monochrome source rendering remains the default; Explorer/rig color views remain separate.

Headless native GPU validation now passes on the RTX3090/DX12, including a240s run and the host blit. Existing original event/actor observations can be enabled with MW_NATIVE_DIAGNOSTICS=1; this changes logging only. Device/window display, audio device output, action validation, PC prompt replacement and fidelity/performance remain unfinished. tools/run_owned_gpu_test.py runs the bounded owned-process diagnostic and records exact scope in evidence.


Current verification obligations and scoped facts are in ledger.json; notes/LEDGER.md is the generated report. The full-port check intentionally remains open. Current summaries include native-held-struggle-summary.json, native-rin-pair-content-summary.json and bevy-native-audio-device.json in evidence.

PC prompt staging: run `python tools/index_live_ui_layout.py` and `python tools/stage_pc_ui.py`. The native frontend loads `staged/pc-ui/core_us.idd` when present. Eleven common controller slots become PC mouse/key prompts; all three modified texture pages are hash-matched in native RAM. Live animated legibility and full contextual PC UI remain open. See `research/UI-PC-PROMPTS.md`.

The native remote-speaker route now supports enable/status/mute/registers and ADPCM/PCM into the PC stereo sink. Synthetic USB-to-stereo tests pass; actual game setup is observed at6kHz/volume88 and muted. Game-triggered unmuted sound, full audio events and original mix/timing remain open. See `research/REMOTE-SPEAKER-INDEX.md`.

Held X now completes the native Rin clash and directional follow-up in a fixture-disabled PC replay, with source AA/AB/AC role contents verified, Rin HP1600->1300 and ordinary play resumed. `research/PC-CONTROLS.md` documents the tested press/release/strike cycle and scope. Complete fight victory, native speed and visual/audio/UI fidelity remain unfinished.
