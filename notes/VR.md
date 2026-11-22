# VR (Quest 3) [added, not native]

Kreese Kreely in first person, on a Quest 3 over Link / Air Link (or any OpenXR headset), with his own rigged arms,
physical punches, grabs, throws, climbing, the pistol, and a VR menu in the pause menu's look.

## Playing

1. Start the Meta Quest Link app on the PC (it is already the active OpenXR runtime on this machine), connect the
   Quest 3 by cable or Air Link and enter Link. (If the game does not appear in the headset: Meta Quest Link app >
   Settings > General > Unknown Sources: on.)
2. Run `play-vr.cmd` (RinRin) or `play-vr.cmd street` / `elise` / `baron` / `mathilda` / `town`. It builds the
   release game if needed. By hand: `MW_VR=1 MW_STAGE=street D:/GDecomp/build-cache/bevy-0.20/release/mw-play.exe`.
3. Without a headset the runtime reports the device unavailable and the game falls back to the desktop view, driven
   by the simulator's keys (below). `MW_VR=sim` starts the simulator directly.

The desktop window mirrors the game (every third frame; the VR settings' Desktop mirror turns it off).

## Kreely's body

The eyes see all of Kreely (play_vr_body.rs): one instance of his rigged model posed from the headset and controllers.
His arms reach from his shoulders to your hands and bend at the elbows; his spine bends and twists toward your head;
crouch and his pelvis sinks, his knees bend over planted feet; his feet step after you when you move or turn, and swing
in a stride while the stick walks him. His head folds away so it never fills the view; the body stands 10 cm behind
your eyes so looking down shows his chest, legs and feet. Eye height is his: 1.72 m (his eye bones).

## Power struggles

Each struggle takes the gesture of its scene (play_vr_struggle.rs), not a shake:
- **The Baron**: a punch battle. Punch him again and again (each blow fills the meter); his steps want a hook across
  (SIDE) or a hammer down (DOWN), a straight jab counts as either; the wrong one fails the step, as natively.
- **RinRin**: grab her fans before her chest and wrench them back toward you, stroke after stroke.
- **Elise**: grip her hands and tear them off you (first half of the meter), then shove her away with open hands.
- **Mathilda's pin**: push her bat off, both hands into it, again and again.
- Dizzy: shake your head. The giant swing: whirl your arms.
In a clash the view rides Kreely's animated eyes, height included (the clips carry both fighters off the sim's origin;
his eyes keep 0.75 m from her head, his copied body moving with them), turned once to face her. Marks in the game's own lock-on art show where: the sight-ring on the spot to strike (it
jumps face / chest per blow, flashing red on a hit), rings where to grab with arrows sliding the way to pull or push. The
move's word floats over her in the core_us brush capitals with a white-ruled meter. The PC key pictures are hidden;
the native struggle widgets stay.

## Controls (Touch)

| | Left | Right |
|---|---|---|
| Stick | move (all the way: run) | snap / smooth turn |
| Stick click | run (when set to click) | |
| Grip | grab / climb / hold; at the left hip: a magazine; on the slide: rack it | grab / climb / hold; at the hip: draw the pistol |
| Trigger | index finger | fire the pistol |
| A / X | X: reload (simple pistol only) | A: jump (confirm in menus); holding the pistol: drop the magazine |
| B / Y | Y: taunt | B: dodge (back in menus); holding the pistol: slide release |
| Menu | the VR menu | |

- **Punch** by swinging: speed sets the power (6-36), a closed fist above 5.2 m/s is a heavy blow that knocks
  grunts down. RinRin, Elise, the Baron and Mathilda take the blows through their own hurt reactions.
- **Grab** a grunt or a townsperson by closing a hand on him: he is lifted by the collar (struggles, then breaks free
  as natively); punch him with the other hand (he reels in the hold); swing and let go to **throw** him (he flies,
  knocks down whoever he hits, lands hard). Close a hand on RinRin or a boss for Kreely's own grab.
- **Objects** (the street's crates, baskets, Madam Yang heads, bins...): grab, carry, throw; they break on grunts,
  RinRin and the bosses.
- **Climb**: grip a wall or a ledge and pull; over a ledge let go (or hold the left stick forward to pull up) to vault
  onto it (Elise's crypt tiers).
- **Pistol** (realistic handling, the default; play_vr_gun.rs): grip at the gun-hand hip to draw, aim down the barrel
  (it lies in Kreely's fist, the barrel along his hand, and the round goes down the barrel). Let go of it at the hip to holster it, anywhere
  else to drop it (a swing throws it); pick it up off the floor with either hand. It fires only with a round in the
  chamber; an empty chamber clicks. A (X left-handed) drops the magazine; grip at the other hip (the pouch) for a fresh
  one from the reserve and push its top into the bottom of the grip to seat it. Grip the rear of the slide with the
  free hand, pull it back and let go: it chambers a round (one already chambered flies out). Firing the last round
  locks the slide back; seat a fresh magazine and rack it or press B (Y) to send it home. The slide release drops
  the slide on an empty magazine too, so after it a fresh magazine still needs a rack. Loose magazines can be picked
  up; let go of at the pouch, their rounds go back in the reserve. The magazine plus the chamber is the HUD's count,
  the pouch is the reserve (punches refill it). Settings: "Pistol handling" SIMPLE brings back the holster and X
  reload; "Pistol angle" tilts the barrel if it does not line up with your hand on your controllers.
- **Aiming and rounds** (Pavlov-like): no crosshair or laser; aim with the three-dot iron sights (green dots on the
  front post and the rear notch), zeroed at 15 m: what the sights cover there is what the round strikes. Each shot is
  a real round (play.rs `fly_vr_rounds`): it leaves the bore at 360 m/s, drops under gravity, and is traced leg by
  leg each tick against grunts (headshots x3), the bosses, RinRin (her fan bats it back) and the stage; a grazing
  strike on a wall or the floor (under ~15 degrees) ricochets and flies on at half speed; a strike sparks where it
  lands. The recoil flips the muzzle up and the gun settles back over a few frames, so a fast string climbs (one-
  handed, four quick shots at 7 m go from the chest to over the head); close the other hand round the grip for a
  two-handed hold and the flip drops to about a third.
- **Tracers and brass** (play_tracer.rs; the screen game too): each round draws a bright streak with its copper tip
  riding the round's real legs at 360 m/s (ricochets included), and the path it ran keeps a smoke wake that holds a
  moment, spreads and drains away. Every shot throws a spent brass case out of the ejection port on the slide's
  right side (up, right and a little back, tumbling); cases bounce on the floor and settle, the newest 40 stay for
  12 s. Racking out a chambered round throws a live cartridge.
- **The screen (flat) game shares it**: its shots are the same rounds (`fly_vr_rounds`) with the same strikes,
  ricochets and fan deflections (a round batted back can still catch Kreely, the native ricochet's 45 % for 6), the
  same tracers, wakes and brass. A round leaves the pistol's muzzle for whatever lies under the reticle (the camera
  ray's first grunt, boss, RinRin or wall: the gun converges on the sight). The brass and streak are drawn somewhat
  larger there (the camera stands a few metres back).
- **Left-handed** ("Gun hand: LEFT"): the holster moves to the left hip, the magazine pouch to the right, X drops the
  magazine and Y is the slide release; the right hand racks and supports.
- **Mash meters** (dizzy, clashes, struggles): shake the controllers (or press the face buttons); clash swings are
  big down / sideways swings.
- **Everything is first person.** Event scenes, pair cameras, clashes, finishers and the defeat play from Kreely's
  animated eyes (height included: on the floor when he is), his body posed as the clip has it under you, your arms on
  your hands. (A theater screen can still be switched on in the VR settings; it is off by default.)
- The HUD (gauge, ammo, prompts, the native pause pages) floats on a panel that drifts after your view.

## VR menu and settings

Left menu button (F10 on the desktop): RESUME, VR SETTINGS, CONTROLS, GAME MENU (the native pause), RECENTER VIEW,
STAGE SELECT, QUIT GAME. Point with the gun hand's laser and pull the trigger, or use the sticks and A / B.

Settings (saved to `vr-settings.toml` beside the exe; `MW_VR_SETTINGS` names another): turning (snap / smooth), snap
angle, smooth speed, move direction (head / left hand), run (push / click / walk only), walk speed, comfort
vignette, gun hand (left-handed play), view height (Kreely's eyes / your own), seated play, grab hold / toggle,
haptics, punch strength, climbing, HUD distance / size / follow, theater, desktop mirror, and the hand fit (forward,
height, angle of Kreely's hands and the pistol on the controllers: tune these first on the real Quest).

## How it is built

- Every crate the VR work fetched lives on D: in `crates/vendor` (openxr, openxr-sys, libloading 0.9, cmake, mint via
  `[patch.crates-io]`; none are left in the C: cargo cache).
- `crates/vendor/bevy_mod_openxr`, `bevy_mod_xr`: bevy_oxr 0.6 ported to Bevy 0.20 / wgpu 30 (extract_app
  attributes, the wgpu wrappers, `create_texture_from_hal`'s initial state). Vulkan backend, STAGE space.
- `crates/mw-game/src/play_vr*.rs`: the rig (tracking space anchored to the sim player, body follows the head with
  collision, height calibration), views (the game camera renders a flat view for the mirror and the theater; all
  UI renders to a HUD texture), the OpenXR actions, the simulator, Kreely's arms (two instances of his model posed
  per hand: wrist on the controller, two-bone elbow, every other bone folded into the elbow, fingers from the rig's
  open / fist / gun poses), the interactions, the menu.
- Hooks in the game: `Play::vr_aim` (rounds along the barrel), `Street::vr_*`, `Props::vr_*`,
  `Elise / Baron::vr_punch`, `PlayerFight::vr_hang / vr_drop`, `Painter::parent`, XR cameras excluded from the
  game-camera queries.

Climbing grips any wall or ledge. A vault onto a top the native physics never stands on (the street's 0.96 m stall
counters) is held by a VR perch: he stays on it, the stick walks him along it, and walking over its edge steps him
down (xr_stall.json, MW_VR_TEST=place:0.2:25.5:90). Elise's crypt tiers are native floors (two vaults in a row).

## Headset on, headset off

- **Auto mode** (MW_VR unset or `auto`): at launch the game asks the OpenXR runtime for a head-mounted system and
  starts in VR with one, flat without. Started in VR, the switch is instant and in place: the session lost (unplugged,
  Link ended) flips to flat on the spot (the game camera and HUD go straight to the window, the VR systems pause, an
  open VR menu closes) and the game keeps running; it asks the runtime every 1.5 s, starts a new session when the
  headset answers, and flips back to VR the moment it runs (the tracking space re-anchors on Kreely). A headset that
  never runs a session within 25 s also plays flat until it does. Started flat, there is no headset renderer (the GPU
  device is made through OpenXR only when a headset is there at launch), so connecting one restarts the game on the
  same stage in VR. `MW_VR=1 / sim / 0` pins a mode. (play_vr_auto.rs, play_vr.rs `apply_mode`; vendored
  bevy_mod_openxr: a lost session now stops the frame loop, and a failed session creation leaves it Available.)
- **Meta's runtime (Quest Link / Air Link) is never probed** (found on the real Quest 3, 2026-11-15): every throwaway
  OpenXR instance makes Link launch the app and every closed one makes Link leave VR, which ends the headset's Link
  session. With Meta's runtime active (OVRServer_x64 running) the game starts through OpenXR straight away: its one real
  instance plays in VR if a headset is in Link, and with none (OpenXR unavailable) it plays on the monitor in place.
  While on the monitor it reads Meta's own service log (the newest %LOCALAPPDATA%\Oculus\Service_*.txt, the last
  `oculus_xrs_session_state_transition`: a `streaming` session `Connected`) and relaunches into VR once (a relaunch
  that still finds no headset waits for Link to go and come back: no loop). Other runtimes are still probed through
  OpenXR. Either trigger (or A / X) confirms in menus and prompts. A
  session the runtime stops (a Link USB drop: `ovrError_XRStreamingUSBIssue`, device_error) plays flat after 2 s and
  back in VR when the runtime brings it back; ending or beginning a session the runtime refuses is logged, not fatal
  (vendored bevy_mod_openxr). Test runtime: `MW_XRSIM_DROP_FILE=<path>` imitates such a drop while the file exists.
- **Hands on the controllers** (found on the real Quest 3, 2026-11-16): OpenXR's grip pose runs its -Z up the handle
  (through the fist toward the thumb), so a hand turned by it points at the sky. Kreely's hands and the pistol now sit
  at the grip (the palm) turned like the aim pose (the pointing ray: -Z forward, +Y up): play_vr.rs `Hand::hand_pose`.
  The test runtime reports Touch's poses that way too (the script's rotation is the aim; the grip is 55 degrees up).
- **Standing height** (found on the real Quest 3: crouched while standing tall): not the first tracked reading (which
  can come while the headset is still going on) but the 70th percentile of the head's height over the last 12 s
  (sampled at 10 Hz, after the first second): a moment held high (under ~3.5 s) or a crouch (under ~8 s) does not move
  it, a wrong start settles within seconds; the first 6 cm under it are a slouch, not a crouch (play_vr_body.rs).
  Recenter clears it. A controller that is not tracked hangs its arm at his side.
- **Holding RinRin**: closing a hand (or both) on her holds her up off the ground (fight.rs RIN_VR_HELD: her moves
  wait, she writhes in the giant swing's hold): carry her, walk with her; let go with a swing and she is thrown along
  the hand's speed (the giant swing's flight: she slides on, the fence shocks her; 6 damage per m/s, 10-70); lift
  both hands over your head while holding her and Kreely's own grab takes her: the suplex. She breaks free after 5 s.
  The bosses still take Kreely's grab directly. (MW_VR_TEST=rin:<m>:<every> sets her before the view; xr_rin_hold.json.)
- **Frame rate** (measured with the test runtime unpaced, MW_XRSIM_NOPACE): the desktop window no longer waits for the
  monitor's vsync in VR (it held the headset to the monitor's 75 Hz), the eyes render with 2x MSAA instead of the flat
  game's 4x (about 60 -> 95 fps in the fight at 2112 x 2304), and "Render scale" (VR settings, default 80 %, next
  launch; MW_VR_RENDER_SCALE) sizes the eye images. Each eye already draws only what its view frustum holds. No
  leak over several minutes (MW_VR_GROWTH_LOG prints fps and the asset counts every 10 s).
- **The challenge** ("take the challenge?") is accepted on its own after a moment (MW_CHALLENGE_AUTO=0: choose).
- **No head in VR**: the eyes' copy of Kreely drops every triangle weighted a quarter or more to the neck, head, jaw
  and eye bones (6461 triangles), so his face never fills the view in clashes, scenes or when looking down. That body (and the pistol) is on its own
  layer (L_SELF): the eyes see it, and so does the desktop mirror while it is the head; the flat game sees the whole
  native Kreely.
  (play_vr_body.rs `bind`)

## Testing without a headset

- `MW_VR=sim`: the simulator. Mouse looks (hold RMB: move the selected hand, wheel: depth); 1 / 2 / 3 select hands;
  LMB trigger, G grip, F jab, H hook, V chop, K shake; Q (held) reach for the holster; C crouch; WASD left stick,
  arrows right stick; Space A, Shift B, R X, T Y, Tab menu.
- `MW_VR_SCRIPT=tools/vr/scripts/*.json`: scripted controllers (`punch`, `throw_gun`, `gun`, `prop`, `climb`, `menu`,
  `intro`, `rin`); `MW_VR_SHOTS="clock:path;..."` screenshots on the script clock; `MW_VR_LOG=1` logs every grab,
  blow, throw, climb; `MW_VR_TEST=grunt:<m>:<every>` stands a grunt before you, `MW_VR_TEST=place:x:z:yaw` moves you.
- `tools/vr/xrsim`: a headless OpenXR runtime (Vulkan swapchains on the game's device, scripted Touch controllers,
  haptics logged, eye images dumped to PNG). Build: `CARGO_TARGET_DIR=D:/GDecomp/build-cache/xrsim cargo build
  --release` in that folder; run the game with `XR_RUNTIME_JSON=<repo>/tools/vr/xrsim/xrsim.json MW_VR=1`,
  `MW_XRSIM_SCRIPT=tools/vr/scripts/xr_fight.json`, `MW_XRSIM_DUMP="300,600:D|/out"`, `MW_XRSIM_VIEW=1832x1920`.
- `tools/vr/run_vr.sh KEY=VALUE...` runs the game under the one-instance lock.
- `MW_XRSIM_PRESENT_FILE=<path>`: xrsim's headset is plugged in only while that file exists (delete it to pull the
  cable: no system, no new session, and a running one goes LOSS_PENDING). Verified 2026-11-15 on the street stage, one
  process throughout: auto start in VR, unplug → flat at once, replug → VR again 1-2 s later; `MW_VR_SWITCH_SHOTS=
  <prefix>` saves the window after each switch. `MW_VR_PROBE_LOG=1` logs why a probe found no headset.

Verified this way (2026-11-14): the full OpenXR path (instance, Vulkan device, session states, a 2-layer swapchain,
12 actions on the Touch profile, action spaces, haptics, stereo frames) against xrsim; a full session through OpenXR
actions (tools/vr/scripts/xr_session.json: walking, snap turns, punches, a grab and throw, the pistol drawn and fired,
reload), the realistic pistol (tools/vr/scripts/xr_pistol.json: draw, fire, magazine out, a fresh one from the pouch
seated, racked (the live round ejected), emptied to slide lock, dry clicks, slide release, dropped, picked up off the
floor, holstered; tools/vr/scripts/xr_pistol_left.json is the same left-handed), live fire with ballistics
(xr_pistol_fire.json, MW_VR_TEST=grunt:8:100: single shots strike at 7 m, a one-handed string climbs off the head,
the two-handed string stays on him), tracers, wakes and brass in the headset and on the screen (xr_pistol_fire.json;
tools/vr/scripts/flat_fire.json with MW_PLAY_SCRIPT for the screen game), a live street brawl (tools/vr/scripts/xr_brawl.json with MW_VR_TEST=pair:1.0:140: punches, an attacking
grunt grabbed, lifted and thrown into a hard landing, 12 pistol rounds of which 7 struck flesh and killed two grunts),
a thrown grunt knocking down the one behind him (xr_throw.json, MW_VR_TEST=pair:1.0:140:1.2), climbing and vaulting
Elise's tiers twice in a row (1.79 m then 3.56 m), hands opening and closing, walking and smooth turning with the
vignette (xr_climb2.json), a townsperson grabbed by the collar and thrown (xr_town.json, MW_STAGE=town,
MW_VR_TEST=civ:1.0:20), the VR menu pages in stereo (xr_climb_menu.json), and room-scale movement (xr_roomscale.json / xr_wall.json: walking in
the play space moves Kreely 1:1, a wall stops the head, snap turns pivot on the head); ~71 fps at Quest 3's 1832 x 1920 per eye in the release build on the RTX 3090 during a street fight. Not yet verified on the real headset: comfort, scale and the controller offsets
(the Touch Plus grip pose may want small offset tweaks in play_vr_hands.rs).
