//! Native c201 fight (research/NATIVE-PORT-GOAL.md): Jack or Kreese Kreely against RinRin. Pure Rust, no
//! recompiled core. The rules live in mw_sim::fight (translated from the DOL / enemy RSO, research/PL0000-COMBAT.md,
//! PL0000-LOCOMOTION.md); this file is presentation, PC input, collision sliding and the added Kreely pistol.
//!
//! Native: start points (PlayerTbl point 5; Rin spawn 0 of the arena record), 30 Hz clock, locomotion states,
//! attack string / chainsaw swings with motion cues, plem_common.ini damage, em0440.csv AI tuning, damage volumes,
//! Rin's wait/run/attack/reaction/guard/evade/finish-wait states, HP. Floor and walls: c201 collision rays.
//! Inferred: PC key -> pad mapping, follow camera, wall/ledge sliding, the pistol flow (added character).
use bevy::{camera::visibility::NoFrustumCulling, input::mouse::AccumulatedMouseMotion, prelude::*};
use mw_data::{character::CharacterRig, Bone, Clip, MotionBank};
use mw_sim::character::{apply_hand_pose, crossfade, mix, prepare_overlay_clip, sample_overlay, sample_retargeted, strip_root_xz, LocalPose, Retarget};
use mw_sim::fight::{CombatData, Fight, FightData, FightInput, Motion};
use std::{collections::HashMap, io::Write};

#[path="play_pause.rs"]
mod pause;
#[path="play_audio.rs"]mod audio;
#[path="play_intro.rs"]mod intro;
#[path="play_hud.rs"]mod hud_ui;
#[path="play_fx.rs"]mod fx;
#[path="play_execution.rs"]mod execution;
#[path="play_ink.rs"]mod ink;
#[path="play_grade.rs"]mod grade;
#[path="play_shadow.rs"]mod shadow;
#[path="play_look.rs"]mod look;
#[path="play_outline.rs"]mod outline;
#[path="play_sky.rs"]mod sky;
#[path="play_springs.rs"]mod springs;
#[path="play_gun_hud.rs"]mod gun_hud;
#[path="play_reload.rs"]mod reload;
#[path="play_tracer.rs"]mod tracer;
#[path="play_broadcast.rs"]mod broadcast;
#[path="play_street.rs"]mod street;
#[path="play_actors.rs"]pub mod actors;
#[path="play_editor.rs"]mod editor;
#[path="play_cutin.rs"]mod cutin;
#[path="play_breakables.rs"]mod breakables;
#[path="play_props.rs"]mod props;
#[path="play_kreely_voice.rs"]mod kreely_voice;
#[path="play_hold_ui.rs"]mod hold_ui;
#[path="play_prompts.rs"]mod prompts;
#[path="play_continue.rs"]mod continue_ui;
#[path="play_decal.rs"]mod decal;
#[path="play_hurt.rs"]mod hurt;
#[path="play_radar.rs"]mod radar;
#[path="play_elise.rs"]mod elise;
#[path="play_baron.rs"]mod baron;
#[path="play_select.rs"]mod select;
#[path="play_clear.rs"]mod clear;
#[path="play_gore.rs"]mod gore;
#[path="play_town.rs"]mod town;
#[path="play_vr.rs"]mod vr;

const START: [f32; 3] = [2.5, 17.55, -61.];
const START_YAW_DEG: f32 = 180.;
const RIN_START: [f32; 3] = [2.300_528_5, 17.822_876, -68.665_16]; // arena spawn 0, kind 0x20440 (arena-meta.json)
const TICK_HZ: f64 = 30.;
/// Wall sources: the level mesh plus col_pl.hkx (player boxes; "player-only" [inferred] from names, COLLISION-API).
const STATE_GUN: u8 = 0xC8; // added: Kreely's pistol, outside the native state table

#[derive(Clone, Copy, PartialEq, Debug)]
enum GunAction { Aim, PistolIdle }

#[derive(Clone, PartialEq, Debug)]
enum Src { Jack(usize, f32), Gun(String, f32) }

struct Gun { action: GunAction, key: String, frame: f32, from: Option<(Src, f32, f32)> }
/// The legs while Kreely walks with the pistol out: Jack's walk cycle frame, its weight (eases in and out), and the
/// walking direction relative to his facing.
#[derive(Default, Clone, Copy)]
struct GunLegs { frame: f32, weight: f32, angle: f32 }

/// Kreely's pistol ammunition (added, not native: MadWorld's player has no firearm). A wp0671-style semi-automatic:
/// a 12-round magazine and a carried reserve; a spent magazine dry-fires and reloads. Ticks are the 30 Hz game frame.
const MAG: u8 = 12;
const RESERVE_START: u16 = 48;
const RESERVE_MAX: u16 = 96;
/// Ticks between shots (5 per second) and of a reload (the magazine seats at RELOAD_SEAT).
const FIRE_GAP: u8 = 6;
const RELOAD_TICKS: u8 = 40;
const RELOAD_SEAT: u8 = 28;
/// Rounds a landed melee hit earns back into the reserve (keeps the pistol fed through a long fight) [inferred].
const MELEE_REFILL: u16 = 2;
const MASH_TICKS: u8 = 6;
/// An unlocked attack turns to the nearest enemy within this many metres [added].
const AUTO_FACE: f32 = 5.;
/// Per-tick fade of the recoil kick.
const RECOIL_DECAY: f32 = 0.72;

/// [added] A VR round in flight: a 9 mm pistol round leaves at about 360 m/s, drops under gravity, is traced along
/// each tick's leg against the grunts, the bosses, RinRin and the stage; a shallow strike on the stage (under ~15
/// degrees) ricochets and flies on slower, a fan bats it away; past VR_ROUND_RANGE it is spent.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VrRound { pos: Vec3, vel: Vec3, travelled: f32, bounces: u8, id: u32, fan: bool }
const VR_ROUND_SPEED: f32 = 360.;
const VR_ROUND_RANGE: f32 = 200.;
const VR_ROUND_DAMAGE: i32 = 20;

/// The nearest stage surface along a leg, and its normal.
fn stage_hit(world: &mw_data::collision::CollisionWorld, from: Vec3, dir: Vec3, range: f32) -> Option<(Vec3, Vec3)> {
    let od = [from.x as f64, from.y as f64, from.z as f64];
    mw_sim::collision::raw_surface_ray(world, od, unit(dir), range as f64, None).ok()
        .and_then(|h| h.iter().filter(|h| h.distance > 1e-3).min_by(|a, b| a.distance.total_cmp(&b.distance))
            .map(|h| (from + dir * h.distance as f32, Vec3::new(h.normal[0] as f32, h.normal[1] as f32, h.normal[2] as f32).normalize_or(Vec3::Y))))
}

/// FixedUpdate (the fight tick): every VR round flies one tick, striking the first thing on its leg.
fn fly_vr_rounds(p: &mut Play, street: &mut street::Street, elise: &mut elise::Elise, baron: &mut baron::Baron, fx: &mut fx::Fx,
                 world: &mw_data::collision::CollisionWorld) -> Vec<&'static str> {
    let dt = 1. / 30.;
    let mut strikes = vec![];
    let log = std::env::var_os("MW_VR_LOG").is_some();
    let mut flying = vec![];
    for mut b in std::mem::take(&mut p.vr_rounds) {
        let mut left = dt;
        let mut alive = true;
        let chest = Vec3::from_array(p.fight.player.pos) + Vec3::Y * 1.15;
        // (a ricochet carries on within the same tick)
        for _ in 0..4 {
            let vel_end = b.vel + Vec3::NEG_Y * 9.81 * left;
            let leg = (b.vel + vel_end) * 0.5 * left;
            let len = leg.length();
            if len < 1e-4 { break; }
            let d = leg / len;
            let stage = stage_hit(world, b.pos, d, len);
            let reach = stage.map_or(len, |h| h.0.distance(b.pos));
            let from = b.pos;
            let leg_to = |to: Vec3, last: bool| GunEvent::Leg { id: b.id, from, to, last };
            // batted back off a fan, it may cross the shooter (the native ricochet's chance and power)
            if b.fan {
                let t = (chest - from).dot(d).clamp(0., reach);
                let near = from + d * t;
                let k = (b.id.wrapping_mul(0x9E37_79B9) ^ (b.travelled as u32).wrapping_mul(0x85EB_CA6B)).wrapping_mul(0xC2B2_AE35) >> 8;
                if near.distance(chest) < RICOCHET_REACH && (k % 100) < RICOCHET_CHANCE {
                    p.fight.enemy_hits_player(&p.fd, 0x172, RICOCHET_POWER, near.to_array());
                    p.gun_events.push(leg_to(near, true));
                    p.gun_events.push(GunEvent::Impact { at: near, dir: d, flesh: true });
                    if log { eprintln!("[vr] tick {} the ricochet strikes Kreely", p.tick); }
                    strikes.push("ricochet");
                    alive = false; break;
                }
            }
            let mut struck = |p: &mut Play, at: Vec3, what: &'static str| {
                p.gun_events.push(GunEvent::Leg { id: b.id, from, to: at, last: true });
                p.gun_events.push(GunEvent::Impact { at, dir: d, flesh: true });
                strikes.push("hit");
                if log { eprintln!("[vr] tick {} round struck {what} at {:.2} ({:.1} m out, {:.0} m/s, dropped {:.3} m)", p.tick, at, b.travelled + at.distance(b.pos), b.vel.length(), b.drop_at(b.travelled + at.distance(b.pos))); }
            };
            if street.on { if let Some((at, root)) = street.shoot(b.pos, d, VR_ROUND_DAMAGE, reach, fx) { fx.wound(Some(root), at, d, 0.32); struck(p, at, "a grunt"); alive = false; break; } }
            let boss = if elise.on { elise.shoot(b.pos, d, VR_ROUND_DAMAGE, reach, fx) } else if baron.on { baron.shoot(b.pos, d, VR_ROUND_DAMAGE, reach, fx) } else { None };
            if let Some(at) = boss { struck(p, at, "the boss"); alive = false; break; }
            match p.fight.player_shot(&p.fd, b.pos.to_array(), d.to_array(), reach) {
                mw_sim::fight::Shot::Hit(at) => { let at = Vec3::from_array(at); fx.wound(None, at, d, 0.24); struck(p, at, "RinRin"); alive = false; break; }
                mw_sim::fight::Shot::Deflect(at) => {
                    // batted off her fan: back the way it came, scattered and slowed
                    let at = Vec3::from_array(at);
                    let k = (b.travelled as u32).wrapping_mul(2_654_435_761);
                    let jig = Vec3::new(((k >> 8) & 0xFF) as f32 / 255. - 0.5, ((k >> 16) & 0xFF) as f32 / 400., ((k >> 4) & 0xFF) as f32 / 255. - 0.5);
                    let used = (at.distance(b.pos) / len).clamp(0., 1.);
                    left *= 1. - used;
                    b.travelled += at.distance(b.pos);
                    b.vel = (-d + jig * 0.8).normalize() * b.vel.length() * 0.5;
                    b.pos = at + b.vel.normalize() * 0.05;
                    b.bounces += 1; b.fan = true;
                    p.gun_events.push(leg_to(at, false));
                    p.gun_events.push(GunEvent::Impact { at, dir: d, flesh: false });
                    strikes.push("deflect");
                    if log { eprintln!("[vr] tick {} round batted off the fan at {:.2}", p.tick, at); }
                    continue;
                }
                mw_sim::fight::Shot::Miss => {}
            }
            match stage {
                Some((pt, n)) => {
                    p.gun_events.push(GunEvent::Impact { at: pt, dir: d, flesh: false });
                    let graze_now = -d.dot(n) < 0.26 && b.bounces < 2;
                    p.gun_events.push(leg_to(pt, !graze_now));
                    if !graze_now { strikes.push("miss"); }
                    let graze = -d.dot(n);
                    b.travelled += pt.distance(b.pos);
                    if graze < 0.26 && b.bounces < 2 {
                        let used = (pt.distance(b.pos) / len).clamp(0., 1.);
                        left *= 1. - used;
                        b.vel = (b.vel - 2. * b.vel.dot(n) * n) * 0.55;
                        b.pos = pt + n * 0.01;
                        b.bounces += 1;
                        if log { eprintln!("[vr] tick {} round ricochets off the stage at {:.2} ({:.0} deg)", p.tick, pt, graze.asin().to_degrees()); }
                        continue;
                    }
                    if log { eprintln!("[vr] tick {} round struck the stage at {:.2} ({:.1} m out)", p.tick, pt, b.travelled); }
                    alive = false; break;
                }
                None => {
                    b.pos += leg; b.vel = vel_end; b.travelled += len;
                    let spent = b.travelled >= VR_ROUND_RANGE || b.pos.y < -100.;
                    p.gun_events.push(leg_to(b.pos, spent));
                    if spent { strikes.push("miss"); }
                    break;
                }
            }
        }
        if alive && b.travelled < VR_ROUND_RANGE && b.pos.y > -100. { flying.push(b); }
    }
    p.vr_rounds = flying;
    strikes
}
impl VrRound {
    /// (for the log) how far gravity has pulled it under its line at `out` metres: ~g t^2 / 2
    fn drop_at(&self, out: f32) -> f32 { let t = out / VR_ROUND_SPEED; 4.905 * t * t }
}
#[derive(Clone, Copy, Debug)]
struct GunAmmo { mag: u8, reserve: u16, cool: u8, reload: Option<u8>, recoil: f32 }
impl Default for GunAmmo { fn default() -> Self { Self { mag: MAG, reserve: RESERVE_START, cool: 0, reload: None, recoil: 0. } } }
/// What the pistol did this game frame, for effects, sound and the HUD.
#[derive(Clone, Copy, Debug)]
enum GunEvent { Shot { muzzle: Vec3, dir: Vec3, end: Option<Vec3>, kind: ShotEnd }, DryFire, ReloadStart, ReloadDone,
    /// [added] a VR round (a projectile) struck something: flesh or the stage
    Impact { at: Vec3, dir: Vec3, flesh: bool },
    /// [added] one leg of a round's flight this tick (the tracer rides it); `last`: it stopped there
    Leg { id: u32, from: Vec3, to: Vec3, last: bool } }
/// What a round struck at its end point.
#[derive(Clone, Copy, Debug)]
enum ShotEnd { Stage, Flesh, Fan { bounces: [Vec3; RICOCHETS], n: usize, hurt: bool },
    /// [added] a VR round in flight: it strikes later (GunEvent::Impact)
    Launched }
/// A round batted off RinRin's fan flies back at the shooter's side and caroms off up to RICOCHETS stage surfaces;
/// crossing the shooter (within RICOCHET_REACH of his chest) it strikes him RICOCHET_CHANCE % of the time for
/// RICOCHET_POWER through the native light-flinch row (0x172) [added].
const RICOCHETS: usize = 3;
const RICOCHET_REACH: f32 = 0.5;
const RICOCHET_CHANCE: u32 = 45;
const RICOCHET_POWER: i32 = 6;
/// Ticks of the aim's blend in from the lowered pistol (the explorer's raise is 20, which reads sluggish) [added].
const AIM_BLEND: f32 = 6.;
/// How close an enemy right in front must be for a punch from the aim to be a pistol whip [added].
const WHIP_REACH: f32 = 1.8;
/// The aiming camera's sideways offset (over the right shoulder) [added].
const AIM_SIDE: f32 = -0.6;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Who { Jack, Kreely, Rin }

#[derive(Component)]
struct Character { who: Who, colored: bool, joints: Option<Vec<Entity>> }
/// ev84's backdrop world (ca03), source or color.
#[derive(Component)]
struct EndingWorld(bool);
/// A room object that is part of the room's own set (the coffins, the breakables): hidden with the room while an ending
/// plays elsewhere.
#[derive(Component)]
pub struct RoomProp;
/// Update, after the pose: while an ending with its own backdrop plays, the room and its objects give way to it.
fn ending_world(intro: Res<intro::Intro>, play: Res<Play>, mut worlds: Query<(&EndingWorld, &mut Visibility)>,
                mut room: Query<&mut Visibility, (Or<(With<ArenaVariant>, With<RoomProp>)>, Without<EndingWorld>)>) {
    let swap = intro.ending_active() && !worlds.is_empty();
    for (w, mut v) in &mut worlds { *v = if swap && w.0 == play.colored { Visibility::Visible } else { Visibility::Hidden }; }
    if swap { for mut v in &mut room { *v = Visibility::Hidden; } }
}
/// The c201 arena in source or added-color textures (tools/stage_arena_color.py: the recomp's color pack).
#[derive(Component)]
struct ArenaVariant(bool);
/// Jack's chainsaw arm wp0600, attached at native bone 8 as the intro's command 14 (research/EVENT-API.md). During the
/// RinRin clash it plays em0440.dat wp0600_04aa..04ad in step with Jack's pl0030_04aa..04ad (fn_802956B4 loads the four
/// together); otherwise bind pose [the ordinary chainsaw blade motions are not staged].
#[derive(Component)]
struct ChainsawArm(Option<Vec<Entity>>);
const ARM_BONE: i16 = 8;

/// Added color is on by default, as in the recomp frontend: MW_COLOR, then the shared pc-controls.toml
/// `[color] mode` ("original" turns it off; "color" and "legacy" both use the color pack here).
fn color_default() -> bool {
    if let Ok(v) = std::env::var("MW_COLOR") { return !matches!(v.to_ascii_lowercase().as_str(), "0" | "off" | "original"); }
    let path = std::env::var_os("MW_PC_CONTROLS").map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("pc-controls.toml"))));
    let mode = path.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| s.parse::<toml::Table>().ok())
        .and_then(|t| t.get("color")?.get("mode")?.as_str().map(str::to_ascii_lowercase));
    mode.as_deref() != Some("original")
}

/// RinRin's bombs: the bl0063 model (bl/bl0063.dat, exported by the explorer's review_export; color and source), one
/// pair per bomb slot. Its origin is the bottom of the ball, so it hangs BOMB_RADIUS below the rolling centre.
#[derive(Component)]
struct BombVisual(usize, bool);
#[derive(Component)]struct FightHud;

/// One game frame of PC input (keyboard/mouse or script).
#[derive(Default, Clone, Copy)]
struct Pad { stick: Vec2, punch: bool, punch_held: bool, secondary: bool, secondary_pressed: bool, swing_h: bool, shake: bool, cont: bool, jump: bool, dodge: bool, reload: bool,
           /// A or D pressed this frame: the stun shake (mashed left / right) [PC]
           mash: bool,
           /// F: Kreely's grab (suplex) [added]
           grab: bool,
           /// T: the taunt
           taunt: bool }

#[derive(Resource)]
struct Play {
    fd: FightData, fight: Fight, kreely: CharacterRig, kreely_bones: Vec<Bone>, gun_clips: HashMap<String, Clip>,
    gun: Option<Gun>, gun_out: Option<(Src, f32, f32)>, layer_frame: f32, ammo: GunAmmo, gun_events: Vec<GunEvent>, legs: GunLegs, prev_legs: GunLegs,
    prev_pos: Vec3, prev_heading: f32, prev_rin: (Vec3, f32), cam_yaw: f32, cam_pitch: f32, mouse_tick: u64,
    /// Camera (research/PC-CAMERA.md, as the recomp frontend): native follow camera; F4 toggles the PC free camera (a
    /// lock-on holds it on the target too); the lock key taps to recenter / unlock and holds to lock on.
    free_cam: bool, recenter: i32, lock_key: Option<(f64, bool)>, cam_notice: Option<(String, u64)>,
    /// Kreely's pistol as run-and-gun (one hand, full run pace; F6 back to the aiming walk) [added experiment]
    run_gun: bool,
    as_kreely: bool, colored: bool, lock_on: bool, latched: Pad, ground: Option<f32>, ground_attr: u8, tick: u64, wall_hits: u64,
    script: Option<Vec<ScriptStep>>, log: Option<std::fs::File>, shots_fired: Vec<u64>, last_events: Vec<String>,
    /// `MW_PLAY_BOT=1`: the mw-sim test autopilot drives the fight instead of the keys (evidence runs).
    bot: Option<mw_sim::fight::Autopilot>,
    /// `MW_PLAY_BOT=keys` / `keys-kreely`: the autopilot presses keys through the script key path, and the pressed
    /// keys are written as a replayable script to `MW_PLAY_RECORD` (key-driven evidence runs).
    key_bot: Option<KeyBot>,
    victory_tick: u64,
    /// Continues left (the continue window spends them; the pause map shows them).
    continues: u32,
    /// Game frames the last A / D press keeps the stun shake going.
    mash_hold: u8,
    /// Tick of the player's death, for the Continue hold below.
    dead_since: Option<u64>,
    /// RinRin's pair camera clock: the player clip it runs for and the motion steps since that clip started
    pair_cam: (usize, f32),
    /// Lock-on [added: any enemy, not only RinRin]: the held target (None: pick the best on the next game frame), a Q / E
    /// step to the next target left (-1) or right (+1) of the view, and the smoothed point the camera chases and the
    /// marker floats over (it glides to a new target).
    lock: Option<LockTarget>, lock_step: i8, lock_at: Option<Vec3>, lock_mark: Option<Vec3>,
    /// The shake key last frame (the hold's head-butt fires on its press).
    carry_shake: bool,
    /// A boss other than RinRin (Elise): HP, max HP and body point, for the HUD gauge, the lock, the radar and the sight.
    boss: Option<(i16, i16, Vec3)>,
    /// This frame's shake key and the down / side swing presses (F / Q), for a boss's clash prompts.
    pad_now: (bool, bool, bool),
    /// [added, VR] the drawn pistol's muzzle and barrel direction (play_vr.rs): rounds go along it, not the camera ray
    vr_aim: Option<(Vec3, Vec3)>,
    /// [added] VR rounds in flight (fly_vr_rounds)
    vr_rounds: Vec<VrRound>,
}

impl Play {
    fn latched_shake(&self) -> bool { self.pad_now.0 }
    fn swing_keys(&self) -> (bool, bool) { (self.pad_now.1, self.pad_now.2) }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum LockTarget { Rin, Grunt(u32), Boss }

struct KeyBot { pilot: mw_sim::fight::Autopilot, kreely: bool, prev: Vec<String>, rec: Vec<(u64, Vec<String>)>, gun_until: u64, gun_next: u64 }

impl KeyBot {
    /// The autopilot's game input expressed as PC keys (8-way WASD relative to the camera, Punch, SwingH, Shake,
    /// Dodge, Continue, Secondary for Kreely's pistol at range, Swap at tick 2 for Kreely).
    fn keys(&mut self, p: &Play) -> Vec<String> {
        let mut k: Vec<&str> = vec![];
        if self.kreely && p.tick == 2 { k.push("Swap"); }
        let inp = self.pilot.input(&p.fight, p.cam_yaw);
        let dist = (p.fight.rin.pos[0] - p.fight.player.pos[0]).hypot(p.fight.rin.pos[2] - p.fight.player.pos[2]);
        if self.kreely && p.as_kreely && p.tick >= self.gun_next && dist > 3. && matches!(p.fight.player.state(), 0 | 1 | 2 | 4)
            && p.fight.rin.state != mw_sim::fight::rin_state::FINISH_WAIT {
            self.gun_until = p.tick + 47; self.gun_next = p.tick + 400;
        }
        if p.tick < self.gun_until {
            k.push("Secondary");
            // squeeze off rounds once the aim has come up (every 4th frame: tap, release)
            if self.gun_until - p.tick < 32 && p.tick % 4 == 0 { k.push("Punch"); }
            return k.into_iter().map(String::from).collect();
        }
        let [x, y] = inp.stick;
        if y > 0.38 { k.push("W"); } if y < -0.38 { k.push("S"); } if x > 0.38 { k.push("D"); } if x < -0.38 { k.push("A"); }
        if inp.a_held || inp.a_trig { k.push("Punch"); }
        if inp.a_trig && self.prev.iter().any(|s| s == "Punch") { k.retain(|s| *s != "Punch"); }   // a new tap needs a release
        if inp.swing != 0 { k.push("SwingH"); }
        if inp.shake > 0. { k.push("Shake"); }
        if inp.nunchuk_shake > 0. { k.push("Dodge"); }
        if inp.cont && p.tick % 2 == 0 { k.push("Continue"); }
        k.into_iter().map(String::from).collect()
    }
}

#[derive(serde::Deserialize, Clone)]
struct ScriptStep { from: u64, to: u64, #[serde(default)] keys: Vec<String>, #[serde(default)] cam_yaw: Option<f32> }

#[derive(Resource)]
struct Arena(mw_data::collision::CollisionWorld);

/// Every defeat (each death, not only the last): the game drops into slow motion while the camera pans around the
/// fallen player, then eases back to speed for RinRin's taunt; the pan holds until Continue [PC presentation of the
/// recomp's death shot; timings inferred]. `since` is the real time of the death, `yaw` the camera yaw it began from.
#[derive(Resource, Default)]
struct DefeatCam { since: Option<f64>, yaw: f32 }
const DEFEAT_SLOW: f32 = 0.3;
const DEFEAT_SLOW_SECS: f64 = 2.2;
const DEFEAT_EASE_SECS: f64 = 0.8;
/// Pan speed (rad per real second) and radius of the orbit around the player.
const DEFEAT_PAN: f32 = 0.45;
const DEFEAT_DIST: f32 = 3.1;


/// Sound effects staged by tools/audio/stage_se.py: (block, sound) -> plays (research/AUDIO-API.md).
#[derive(serde::Deserialize, Clone)]
struct SePlay { file: String, delay: u32, volume: u8,
                /// key-on note and the wave's root key (tools/audio/stage_se.py): it sounds 2^((note - root) / 12) times its rate
                #[serde(default = "middle_c")] note: u8, #[serde(default = "middle_c")] root: u8 }
fn middle_c() -> u8 { 60 }
#[derive(serde::Deserialize, Clone)]
struct SeSound { volume: u8, plays: Vec<SePlay> }
#[derive(Resource, Default)]
struct Sounds { bank: HashMap<String, HashMap<String, SeSound>>, pending: Vec<(u64, String, f32, [f32; 3], f32)>,
                last: [(usize, u32, f32); 2],
                /// The player's pending impact (op 38 stores it at +0x110C; fn_802788F0 plays it when the swing lands).
                impact: Option<(u32, u32)> }

/// Boss BGM staged by tools/audio/stage_bgm.py: intro, then the loop from the RedSpark loop start.
#[derive(serde::Deserialize, Clone)]
struct BgmTrack { intro: String, #[serde(rename = "loop")] looped: String, rate: u32, loop_start: u32, mix:Option<StreamMix> }
#[derive(serde::Deserialize,Clone,Copy)]
struct StreamMix{volume:u8,header:u8,channel:u8}
impl StreamMix{fn gain(self)->f32{mw_sim::event::stream_gain(self.volume,self.header,self.channel).expect("native stream gain")}}
#[derive(Resource, Default)]
struct Bgm { track: Option<BgmTrack>, started: Option<f64>, looping: bool, stopped: bool }
#[derive(Component)]
struct BgmVoice;

/// Stage-201 announcer lines staged by tools/audio/stage_announce.py (jkSound::PlayVoiceStream, research/AUDIO-API.md).
#[derive(serde::Deserialize, Clone)]
struct AnClip { group: u8, file: String }
#[derive(serde::Deserialize, Clone)]
struct AnEntry { mask: u8, cooldown: i32, clips: Vec<AnClip> }
#[derive(Resource, Default)]
struct Announcer { table: HashMap<String, AnEntry>, cooldown: HashMap<u16, i32>, history: std::collections::VecDeque<(u16, u8)>,
                   queue: Vec<(u16, u64)>, rng: u32 }
#[derive(Component)]
struct VoiceClip(u16);

pub fn main() {
    // VR when a headset is connected (MW_VR pins a mode): decided before anything asks for the mode
    vr::auto::decide();
    // one mw-play for every stage: without MW_STAGE (and outside the scripted / inspection runs) the stage select opens
    let scripted = std::env::vars_os().any(|(k, _)| { let k = k.to_string_lossy(); k.starts_with("MW_PLAY_") || k == "MW_DIRECT_FIGHT" || k == "MW_INTRO_PREVIEW" });
    // (VR: no flat stage select in the headset; the VR menu's STAGE SELECT picks one)
    if std::env::var_os("MW_STAGE").is_none() && !scripted && !vr::on() && std::env::var("MW_SELECT").as_deref() != Ok("0") { select::run(); return; }
    // MW_STAGE=town starts by day (play_town.rs; F2 still switches)
    if town::on() && std::env::var_os("MW_TIME").is_none() { std::env::set_var("MW_TIME", "day"); }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read = |p: &str| std::fs::read_to_string(root.join("assets/native").join(p)).unwrap_or_else(|e| panic!("{p}: {e}"));
    let jack = MotionBank::parse(&read("jack-player-motion.json")).expect("jack-player bank");
    let rin = MotionBank::parse(&read("rinrin-motion.json")).expect("rinrin bank");
    let combat = CombatData::parse(&read("combat.json")).expect("run tools/pl0000/stage_combat.py");
    let kreely = CharacterRig::parse(&read("kreely-rig.json")).expect("run tools/kreely/export_kreely_glb.py");
    // MW_STAGE=elise: room c303 and Elise (play_elise.rs); MW_STAGE=baron / mathilda: room c503 (play_baron.rs)
    let room = boss_room();
    let mut collision = mw_data::collision::CollisionWorld::parse(&read(&room.map_or("collision/world.json".to_string(), |r| format!("rooms/{r}/collision/world.json")))).expect("native collision");
    // c503's ring walls (col_obj cube16-19, |x| / |z| 7.5-8 m, 10 m high) are 8-vertex boxes stored as hkpSimpleMeshShape,
    // which the convex wall contacts skip: they get their box planes here so nobody walks through the ropes
    if room == Some("c503") {
        let walls: Vec<String> = collision.bodies.iter().filter(|b| b.source == "col_obj.hkx").flat_map(|b| b.shapes.clone()).collect();
        for s in collision.shapes.iter_mut().filter(|s| walls.contains(&s.id) && s.planes.is_empty() && s.vertices.len() == 8) {
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for v in &s.vertices { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } }
            s.planes = vec![[1., 0., 0., -hi[0]], [-1., 0., 0., lo[0]], [0., 1., 0., -hi[1]], [0., -1., 0., lo[1]], [0., 0., 1., -hi[2]], [0., 0., -1., lo[2]]];
            s.original_triangles = false;
        }
    }
    #[derive(serde::Deserialize)] struct Bank { clips: Vec<Clip> }
    let bank: Bank = serde_json::from_str(&read("kreely-gun-motion.json")).expect("run tools/kreely/stage_kreely_native_clips.py");
    let mut gun_clips = HashMap::new();
    for (key, ov) in &kreely.overlays {
        let src = bank.clips.iter().find(|c| c.id == key.split('|').next().unwrap()).expect("staged overlay source clip");
        gun_clips.insert(key.clone(), prepare_overlay_clip(src, ov));
    }
    let script = std::env::var_os("MW_PLAY_SCRIPT").map(|p| serde_json::from_str::<Vec<ScriptStep>>(&std::fs::read_to_string(p).expect("script")).expect("script json"));
    let log = std::env::var_os("MW_PLAY_LOG").map(|p| std::fs::File::create(p).expect("log file"));
    // MW_STAGE=street: Asia Town's streets and their grunts (play_street.rs), without the boss entry or RinRin
    let street_start = street::start();
    if street_start.is_some() { std::env::set_var("MW_DIRECT_FIGHT", "1"); }
    // c303 PlayerTbl "Boss" (0, 0, 23), facing her down the nave
    // c503 PlayerTbl "start" (0, 0, -6), yaw 0 (facing the Baron across the ring)
    let (start_pos, yaw) = match room { Some("c303") => ([0., 0., 23.], std::f32::consts::PI), Some("c503") => ([0., 0., -6.], 0.), _ => street_start.unwrap_or((START, START_YAW_DEG.to_radians())) };
    let start = Vec3::from_array(start_pos);
    let kreely_bones = kreely.sample_bones();
    let mut fd = FightData::new(jack, rin, combat);
    // Kreely's God Hand moveset (tools/godhand/retarget_godhand.py; mw_sim::kreely)
    match std::fs::read_to_string(root.join("assets/native/kreely-moves.json")).map_err(|e| e.to_string()).and_then(|t| mw_sim::kreely::KreelyMoves::parse(&t)) {
        Ok(m) => { let _ = fd.add_kreely(m); }
        Err(e) => eprintln!("[kreely] no God Hand moveset: {e} (run python tools/godhand/retarget_godhand.py)"),
    }
    fd.world = Some(collision.clone());
    // Elise's clash pairs ship in em0630.dat: the player's half joins his bank
    // (the Baron's: em0623.dat's pl0030_048x clash steps, 105x and Mathilda's bat combo 1055)
    if let Some(bank) = match room { Some("c303") => Some("elise-motion.json"), Some("c503") => Some("baron-motion.json"), _ => None } {
        if let Ok(b) = std::fs::read_to_string(root.join("assets/native").join(bank)).map_err(|e| e.to_string()).and_then(|t| MotionBank::parse(&t).map_err(|e| e.to_string())) {
            fd.add_player_clips(b.clips.into_iter().filter(|c| c.name.starts_with("pl0030_")).collect());
        }
    }
    let seed = std::env::var("MW_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(0x4d57);
    let mut fight = Fight::new(&fd, start_pos, yaw, if street_start.is_some() || room.is_some() { [0., -500., 0.] } else { RIN_START }, seed);
    fight.solo = street_start.is_some() || room.is_some();
    // debug/test only: start RinRin with less HP to reach the finisher quickly (native start is 1600 [observed])
    if let Some(hp) = std::env::var("MW_RIN_HP").ok().and_then(|v| v.parse().ok()) { fight.rin.hp = hp; }
    if std::env::var_os("MW_RIN_SCAFFOLD").is_some() { fight.debug_scaffold(); }   // inspection: bombs
    if std::env::var_os("MW_RIN_PASSIVE").is_some() { fight.debug_passive(); }   // inspection: player moves
    if let Some(hp) = std::env::var("MW_PLAYER_HP").ok().and_then(|v| v.parse().ok()) { fight.player.hp = hp; }   // debug start
    let rin0 = (Vec3::from_array(RIN_START), fight.rin.facing);
    App::new()
        .insert_resource(intro::Intro::load(&root))
        .insert_resource(Play { fd, fight, kreely, kreely_bones, gun_clips, gun: None, gun_out: None, layer_frame: 0., ammo: GunAmmo::default(), gun_events: vec![], legs: default(), prev_legs: default(), prev_pos: start, prev_heading: yaw,
            prev_rin: rin0, cam_yaw: yaw, cam_pitch: 0., mouse_tick: 0,
            free_cam: std::env::var("MW_PC_FREE_CAMERA").is_ok_and(|v| matches!(v.trim(), "1" | "true" | "on" | "yes")),
            recenter: 0, lock_key: None, cam_notice: None,
            // Kreely is the default player (decided 2026-11-10); MW_PLAY_KREELY=0 plays Jack
            run_gun: std::env::var("MW_RUN_GUN").as_deref() != Ok("0"), as_kreely: std::env::var("MW_PLAY_KREELY").as_deref() != Ok("0"), colored: color_default(), lock_on: false,
            latched: Pad::default(), ground: None, ground_attr: 0, tick: 0, wall_hits: 0, script, log, shots_fired: vec![], last_events: vec![],
            victory_tick: 0, continues: std::env::var("MW_CONTINUES").ok().and_then(|v| v.parse().ok()).unwrap_or(continue_ui::CONTINUES), mash_hold: 0, dead_since: None, pair_cam: (usize::MAX, 0.), lock: None, lock_step: 0, lock_at: None, lock_mark: None, carry_shake: false, boss: None, pad_now: (false, false, false), vr_aim: None, vr_rounds: vec![],
            key_bot: std::env::var("MW_PLAY_BOT").ok().filter(|v| v.starts_with("keys")).map(|v| KeyBot { pilot: default(),
                kreely: v == "keys-kreely", prev: vec![], rec: vec![], gun_until: 0, gun_next: 0 }),
            bot: (std::env::var("MW_PLAY_BOT").as_deref() == Ok("1")).then(mw_sim::fight::Autopilot::default) })
        .insert_resource(Arena(collision)).init_resource::<DefeatCam>()
        .insert_resource(Sounds { bank: std::fs::read_to_string(root.join("assets/native/audio/se/se.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(), ..default() })
        .insert_resource(Announcer { table: std::fs::read_to_string(root.join(room.map_or("assets/native/audio/announce/announce.json".to_string(), |r| format!("assets/native/rooms/{r}/announce/announce.json")))).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(), rng: 0x1234_5678, ..default() })
        .insert_resource(Bgm { track: std::fs::read_to_string(root.join("assets/native/audio/bgm/bgm.json")).ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| { let name = v.get("boss")?.as_str()?.to_string(); serde_json::from_value(v.get(&name)?.clone()).ok() })
            .filter(|_| std::env::var_os("MW_NO_BGM").is_none()), ..default() })
        .insert_resource(Time::<Fixed>::from_hz(TICK_HZ))
        .insert_resource(ClearColor(Color::BLACK))   // native clears black (the grade paints black upper frame as sky)
        .add_game_plugins(DefaultPlugins.set(AssetPlugin { file_path: root.join("assets").to_string_lossy().into_owned(), ..default() }).set(WindowPlugin {
            primary_window: Some(Window { title: if vr::on() { "MadWorld native VR: Kreese Kreely".into() } else { "MadWorld native: Jack / Kreese Kreely vs RinRin".into() }, resolution: (1280, 720).into(), present_mode: if vr::built() { bevy::window::PresentMode::AutoNoVsync } else { bevy::window::PresentMode::AutoVsync }, visible:std::env::var_os("MW_PAUSE_SMOKE").is_none()&&std::env::var_os("MW_INTRO_SMOKE").is_none()&&!(std::env::var_os("MW_INTRO_PREVIEW").is_some()&&std::env::var_os("MW_PLAY_CAPTURE").is_some()), ..default() }), ..default()
        }))
        .add_plugins((vr::VrPlugin, vr::auto::AutoPlugin))
        .add_plugins((audio::PortAudioPlugin,pause::PausePlugin,ink::InkPlugin,grade::GradePlugin,shadow::ShadowPlugin,look::LookPlugin,outline::OutlinePlugin,sky::SkyPlugin,tracer::TracerPlugin,broadcast::BroadcastPlugin,street::StreetPlugin,decal::DecalPlugin,elise::ElisePlugin,baron::BaronPlugin,editor::EditorPlugin)).add_plugins((cutin::CutinPlugin, breakables::BreakablesPlugin, props::PropsPlugin, kreely_voice::KreelyVoicePlugin, town::TownPlugin))
        .add_systems(Startup, (setup,intro::setup,hud_ui::setup,fx::setup,gun_hud::setup.after(hud_ui::setup),prompts::setup.after(hud_ui::setup),continue_ui::setup,hurt::setup,radar::setup.after(hud_ui::setup)))
        .init_resource::<clear::Clear>().add_systems(Update, (clear::advance, clear::present).chain())
        .init_resource::<gore::Gore>().add_systems(Update, (gore::spawn, gore::fade).chain())
        .init_resource::<execution::Execution>().add_systems(Update, execution::present.after(defeat)).add_systems(Update, hold_ui::present.after(prompts::present))
        .add_systems(FixedUpdate, (tick, editor::step, street::step, town::step.run_if(resource_exists::<town::Town>), elise::step, baron::step, execution::step, cue_sounds, boss_effect_cues, announce, kreely_voice::step,hud_ui::advance, gun_hud::advance, prompts::advance, tracer::launch, fx::spawn, fx::advance, hurt::step).chain().run_if(pause::running).run_if(intro::fight_running).run_if(cutin::world_running))
        .add_systems(Update, ((input, bind_joints, springs::bind, bind_arm, arena_cutouts, intro::bind_models, intro::challenge::advance, intro::advance, intro::audio::play, intro::effects::play, intro::ui::present, intro::challenge::present, intro::fade).chain(), (cursor, defeat, pose, ending_world.after(breakables::look), springs::step.after(elise::pose).after(baron::pose).after(street::pose), intro::pose_props, fx::billboard, intro::mesh_commands, intro::verify_restored, bombs, camera, hud_ui::present, gun_hud::present, prompts::present, continue_ui::advance, continue_ui::present, hud, capture, title_audio, pause_smoke).chain()).chain()
            ).add_systems(Update,(hurt::teeth.run_if(pause::running),hurt::pulse,radar::present))
            .add_systems(Update,(play_sounds,bgm).chain().run_if(pause::running).run_if(intro::fight_running))
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    for i in 0..8 {
        for (colored, file) in [(false, "native/objects/bl0063-source.glb"), (true, "native/objects/bl0063-color.glb")] {
            commands.spawn((WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))), Transform::default(), Visibility::Hidden, BombVisual(i, colored)));
        }
    }
    commands.spawn((Camera3d::default(), Projection::Perspective(PerspectiveProjection { fov: 50f32.to_radians(), ..default() }), Transform::from_xyz(START[0], START[1] + 2., START[2] + 5.).looking_at(Vec3::from_array(START), Vec3::Y), SpatialListener::new(0.3)));
    let arena = match boss_room() { Some(r) => [(false, format!("native/rooms/{r}/world/{r}.glb")), (true, format!("native/rooms/{r}/world/{r}-color.glb"))],
                                    None => [(false, "native/world/c201.glb".to_string()), (true, "native/world/c201-color.glb".to_string())] };
    for (colored, file) in arena {
        commands.spawn((WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))), Transform::default(), Visibility::Hidden, ArenaVariant(colored)));
    }
    // ev84's backdrop (scene 0xB14 plays it in the event set ca03, not the church): shown only while the ending plays
    if elise::on() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (colored, file) in [(false, "native/rooms/ca03/world/ca03.glb"), (true, "native/rooms/ca03/world/ca03-color.glb")] {
            if root.join("assets").join(file).exists() {
                commands.spawn((WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))), Transform::default(), Visibility::Hidden, EndingWorld(colored)));
            }
        }
    }
    for (who, colored, file) in [(Who::Jack, false, "native/jack-player-source.glb"), (Who::Jack, true, "native/jack-player-pack.glb"),
                                 (Who::Kreely, false, "native/kreely-source.glb"), (Who::Kreely, true, "native/kreely-color.glb"),
                                 (Who::Rin, false, "native/rinrin-source.glb"), (Who::Rin, true, "native/rinrin-pack.glb")] {
        let mut e = commands.spawn((WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))), Transform::default(), Visibility::Hidden,
                        Character { who, colored, joints: None }, if who == Who::Rin { shadow::Caster::ENEMY } else { shadow::Caster::PLAYER }));
        match who { Who::Rin => { e.insert(ink::Inked { model: "rinrin", done: false }); } Who::Jack => { e.insert(ink::Inked { model: "jack", done: false }); } Who::Kreely => {} }
    }
    commands.spawn((WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("native/events/ev21/wp0600.glb"))), Transform::default(), Visibility::Hidden, ChainsawArm(None)));
    commands.spawn((Text::new("Loading native slice"), TextFont { font_size: bevy::text::FontSize::Px(17.), ..default() },
                    Node { position_type: PositionType::Absolute, top: px(12), left: px(12), ..default() }, FightHud));
}

/// Latch presses between fixed ticks; camera from mouse motion. Ignored while a script drives the game.
fn input(keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, motion: Res<AccumulatedMouseMotion>, time: Res<Time<Real>>, mut play: ResMut<Play>, mut pause:ResMut<pause::Pause>,intro:Res<intro::Intro>) {
    if intro.controls_blocked()||town::locked(){play.latched=Pad::default();return;}
    // color toggles in the pause menu too (its grade follows, play_grade.rs)
    if keys.just_pressed(KeyCode::KeyC) && (pause.open || !intro.controls_blocked()) { play.colored = !play.colored; }
    if pause.open {play.latched=Pad::default();return;}
    if pause.release_inputs {
        play.latched=Pad::default();
        // A menu confirmation or a held attack must not become an attack/continue on resume.
        if keys.get_pressed().next().is_some()||mouse.get_pressed().next().is_some(){return;}
        pause.release_inputs=false;return;
    }
    if play.script.is_some() { return; }
    // VR: the controllers make the pad (play_vr.rs)
    if vr::on() { return; }
    // the actor editor holds the keys (F2 hands them back): only the stage select passes
    if editor::focus() { play.latched = Pad::default(); if keys.just_pressed(KeyCode::F9) { select::relaunch(""); } return; }
    if keys.just_pressed(KeyCode::Insert) { swap(&mut play); }
    // locked on, Q / E step the lock to the next target left / right [added]; the mouse buttons still attack
    let (key_e, key_q) = if play.lock_on {
        if keys.just_pressed(KeyCode::KeyQ) { play.lock_step = -1; }
        if keys.just_pressed(KeyCode::KeyE) { play.lock_step = 1; }
        (false, false)
    } else { (keys.pressed(KeyCode::KeyE), keys.just_pressed(KeyCode::KeyQ)) };
    if (key_e && keys.just_pressed(KeyCode::KeyE)) || mouse.just_pressed(MouseButton::Left) { play.latched.punch = true; }
    play.latched.punch_held = key_e || mouse.pressed(MouseButton::Left);
    if mouse.just_pressed(MouseButton::Right) { play.latched.secondary_pressed = true; }
    if key_q || mouse.just_pressed(MouseButton::Middle) { play.latched.swing_h = true; }
    play.latched.secondary = mouse.pressed(MouseButton::Right);
    play.latched.shake = keys.pressed(KeyCode::KeyX);
    if keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::KeyD) { play.latched.mash = true; }
    if keys.just_pressed(KeyCode::KeyF) { play.latched.grab = true; }
    if keys.just_pressed(KeyCode::KeyT) { play.latched.taunt = true; }
    // F9: back to the stage select
    if keys.just_pressed(KeyCode::F9) { select::relaunch(""); }
    // F6: the pistol as run-and-gun or the aiming walk
    if keys.just_pressed(KeyCode::F6) { play.run_gun = !play.run_gun; play.cam_notice = Some((format!("Pistol: {}", if play.run_gun { "run and gun (one hand)" } else { "aim and walk" }), play.tick + 90)); }
    if keys.just_pressed(KeyCode::Space) { play.latched.jump = true; }
    if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight) { play.latched.dodge = true; }
    if keys.just_pressed(KeyCode::KeyR) { play.latched.reload = true; }
    // Lock key (the recomp's Q / Wii C, here Tab): a tap recenters behind Jack (80276C84: +0x27C2 = 8) or, while
    // locked, unlocks; holding it 0.25 s locks on [hold time inferred].
    let now = time.elapsed_secs_f64();
    // Z toggles the lock straight away (PC) [added]
    if keys.just_pressed(KeyCode::KeyZ) { play.lock_on = !play.lock_on; play.lock = None; if play.lock_on { play.cam_pitch = 0.; } }
    if keys.just_pressed(KeyCode::Tab) { play.lock_key = Some((now, false)); }
    if let Some((t0, done)) = play.lock_key {
        if keys.pressed(KeyCode::Tab) { if !done && now - t0 >= 0.25 { play.lock_on = true; play.cam_pitch = 0.; play.lock_key = Some((t0, true)); } }
        else {
            if !done { if play.lock_on { play.lock_on = false; } else { play.recenter = 8; } }
            play.lock_key = None;
        }
    }
    if keys.just_pressed(KeyCode::F4) {
        play.free_cam = !play.free_cam;
        let msg = format!("Free camera: {}", if play.free_cam { "on (mouse look; F4 for the native camera)" } else { "off (native camera)" });
        eprintln!("[camera] {msg}"); play.cam_notice = Some((msg, play.tick + 75));
    }
    if keys.just_pressed(KeyCode::Enter) { play.latched.cont = true; }
    let mut v = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) { v.y += 1.; }
    if keys.pressed(KeyCode::KeyS) { v.y -= 1.; }
    if keys.pressed(KeyCode::KeyA) { v.x -= 1.; }
    if keys.pressed(KeyCode::KeyD) { v.x += 1.; }
    play.latched.stick = stick(v, keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight));
    // Native camera: no manual look. Free camera: mouse orbit on top of the follow camera (recomp native_camera.rs:
    // 0.0025 rad per pixel; mouse up looks up), not while the original owns the view (recenter, or a lock without F5).
    // Kreely's free aim: holding the aim button the mouse steers the view (and so the sight) unless locked on
    let free_aim = play.as_kreely && mouse.pressed(MouseButton::Right) && !play.lock_on;
    let d = if free_aim || (play.free_cam && play.recenter == 0 && !play.lock_on) { motion.delta } else { Vec2::ZERO };
    if d != Vec2::ZERO { play.mouse_tick = play.tick; }
    play.cam_yaw -= d.x * 0.0025;
    // total pitch -0.026 + offset clamped to [-0.9, 0.45] (PC-CAMERA.md); positive raises the view
    play.cam_pitch = (play.cam_pitch - d.y * 0.0025).clamp(-0.45 + 0.026, 0.9 + 0.026);
}

/// While the fight takes mouse look the cursor is hidden, confined to the window and held at its centre (Windows has
/// no locked mode; the look reads raw mouse motion, so re-centring adds no motion); menus, the pause and the
/// challenge window and a death free it. Only while the window has focus.
fn cursor(mut windows: Query<(&mut Window, &mut bevy::window::CursorOptions), With<bevy::window::PrimaryWindow>>, pause: Res<pause::Pause>, intro: Res<intro::Intro>, play: Res<Play>) {
    use bevy::window::CursorGrabMode;
    let Ok((mut window, mut c)) = windows.single_mut() else { return; };
    // the death (and its continue window) frees it too
    let free = pause.open || intro.challenge_blocked() || !window.focused || play.fight.player.state() == 95;
    let grab = if free { CursorGrabMode::None } else { CursorGrabMode::Confined };
    if c.grab_mode != grab { c.grab_mode = grab; }
    if c.visible != free { c.visible = free; }
    if !free {
        let center = Vec2::new(window.width(), window.height()) * 0.5;
        if window.cursor_position().is_none_or(|p| p.distance(center) > 2.) { window.set_cursor_position(Some(center)); }
    }
}

/// Keys -> pad stick (x right, y forward): full tilt, or 0.6 tilt (0.36 < 0.9025: the native walk) with Ctrl.
fn stick(v: Vec2, walk: bool) -> Vec2 { v.normalize_or_zero() * if walk { 0.6 } else { 1. } }

fn swap(p: &mut Play) {
    p.as_kreely = !p.as_kreely;
    if p.gun.is_some() { end_gun(p); }  // Jack has no pistol states
}

fn gun_key(p: &Play, a: GunAction) -> String {
    let stem = match a { GunAction::Aim => "ev93_cut8_aim", GunAction::PistolIdle => "ev93_cut18_idle" };
    p.kreely.overlay_key(stem).expect(stem).to_string()
}

fn current_src(p: &Play) -> Src {
    match &p.gun { Some(g) => Src::Gun(g.key.clone(), g.frame), None => Src::Jack(p.fight.player.motion.clip, p.fight.player.motion.frame) }
}

fn set_gun(p: &mut Play, a: GunAction, blend: f32) {
    if p.gun.as_ref().map(|g| g.action) == Some(a) { return; }
    let from = Some((current_src(p), 0., blend));
    if p.gun.is_none() { p.fight.player.loco.enter_combat(STATE_GUN); }
    p.gun = Some(Gun { action: a, key: gun_key(p, a), frame: 0., from });
}

fn end_gun(p: &mut Play) {
    let src = current_src(p);
    p.gun = None;
    if p.fight.player.state() == STATE_GUN { let f = p.fight.player.loco.facing; p.fight.player.loco.leave_combat(f); }
    p.gun_out = Some((src, 0., 22.));
}

/// One native game frame (30 Hz).
fn tick(mut play: ResMut<Play>, arena: Res<Arena>, mut exit: MessageWriter<AppExit>,intro:Res<intro::Intro>, cams: Query<&GlobalTransform, (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>,
        mut street: ResMut<street::Street>, mut elise: ResMut<elise::Elise>, mut baron: ResMut<baron::Baron>, mut fx: ResMut<fx::Fx>, gun_muzzle: Res<tracer::Muzzle>, mut cont: ResMut<continue_ui::ContinueUi>,
        mut editor: ResMut<editor::Editor>, props: Res<props::Props>) {
    let p = &mut *play;
    p.tick += 1;
    let bot_keys = p.key_bot.take().map(|mut kb| {
        let keys = kb.keys(p); let prev = std::mem::replace(&mut kb.prev, keys.clone());
        if kb.rec.last().map(|r| &r.1) != Some(&keys) { kb.rec.push((p.tick, keys.clone())); }
        if let Some(path) = std::env::var_os("MW_PLAY_RECORD") {
            // run-length script: each entry holds its keys from its tick until the next entry
            let mut steps = vec![serde_json::json!({"from": 1, "to": p.tick + 1, "keys": []})];
            for (i, (from, ks)) in kb.rec.iter().enumerate() {
                let to = kb.rec.get(i + 1).map(|r| r.0).unwrap_or(p.tick + 1);
                if !ks.is_empty() { steps.push(serde_json::json!({"from": from, "to": to, "keys": ks})); }
            }
            if p.tick % 100 == 0 || p.fight.victory { let _ = std::fs::write(path, serde_json::to_string(&steps).unwrap()); }
        }
        p.key_bot = Some(kb);
        (keys, prev)
    });
    let pad = if p.script.is_some() || bot_keys.is_some() {
        let (keys, prev) = if let Some(kp) = bot_keys { kp } else {
            let script = p.script.as_ref().unwrap();
            let keys: Vec<String> = script.iter().filter(|s| s.from <= p.tick && p.tick < s.to).flat_map(|s| s.keys.clone()).collect();
            let prev: Vec<String> = script.iter().filter(|s| s.from + 1 <= p.tick && p.tick - 1 < s.to).flat_map(|s| s.keys.clone()).collect();
            if !p.lock_on { if let Some(y) = script.iter().filter(|s| s.from <= p.tick && p.tick < s.to).find_map(|s| s.cam_yaw) { p.cam_yaw = y; } }
            (keys, prev)
        };
        let has = |k: &str| keys.iter().any(|x| x == k); let edge = |k: &str| has(k) && !prev.iter().any(|x| x == k);
        if edge("Swap") { swap(p); }
        if edge("Color") { p.colored = !p.colored; }
        if edge("LockOn") { p.lock_on = !p.lock_on; p.lock = None; }
        if edge("LockLeft") { p.lock_step = -1; }
        if edge("LockRight") { p.lock_step = 1; }
        let v = Vec2::new(has("D") as i32 as f32 - has("A") as i32 as f32, has("W") as i32 as f32 - has("S") as i32 as f32);
        // test driver aid: "Seek" pushes the stick toward RinRin relative to the camera
        let v = if has("Seek") {
            let (pp, rp) = (p.fight.player.pos, p.fight.rin.pos);
            let yaw = (rp[0] - pp[0]).atan2(rp[2] - pp[2]) - p.cam_yaw;
            if (rp[0] - pp[0]).hypot(rp[2] - pp[2]) > 1.3 { Vec2::new(-yaw.sin(), yaw.cos()) } else { Vec2::ZERO }
        } else { v };
        Pad { stick: stick(v, has("Walk")), punch: edge("Punch"), punch_held: has("Punch"), secondary: has("Secondary"),
              secondary_pressed: edge("Secondary"), swing_h: edge("SwingH"), shake: has("Shake"), cont: edge("Continue"), jump: edge("Jump"), dodge: edge("Dodge"),
              reload: edge("Reload"), mash: edge("A") || edge("D") || has("Shake"), grab: edge("Grab"), taunt: edge("Taunt") }
    } else {
        let l = p.latched; p.latched.punch = false; p.latched.secondary_pressed = false; p.latched.swing_h = false; p.latched.cont = false; p.latched.jump = false; p.latched.dodge = false; p.latched.mash = false; p.latched.grab = false; p.latched.taunt = false; l
    };
    let mut pad=if intro.controls_blocked()||town::locked(){Pad::default()}else{pad};
    if town::take_taunt() { pad.taunt = true; }
    // a button the actor editor presses (its "input" entries)
    match editor::take_press() {
        Some(editor::Press::Punch) => pad.punch = true, Some(editor::Press::Swing) => pad.swing_h = true, Some(editor::Press::Grab) => pad.grab = true,
        Some(editor::Press::Taunt) => pad.taunt = true, Some(editor::Press::Jump) => pad.jump = true, Some(editor::Press::Dodge) => pad.dodge = true, None => {}
    }
    // every shake meter (the dizzy shake, RinRin's clash, the finisher, the giant swing, the bosses' struggles) is mashing
    // A / D on PC, as the native prompt's two halves: each press shakes for MASH_TICKS game frames [PC mapping, added].
    // X stays the head-butt while holding a grunt.
    let x_held = pad.shake;
    if pad.mash { p.mash_hold = MASH_TICKS; }
    pad.shake = p.mash_hold > 0;
    p.mash_hold = p.mash_hold.saturating_sub(1);
    // Continue comes from the native continue window (play_continue.rs), which opens after RinRin's PL_DEAD taunt
    if p.fight.player.state() == 95 {
        p.dead_since.get_or_insert(p.tick);
        pad.cont = cont.answer.take().unwrap_or(false) || editor::take_revive();
    } else { p.dead_since = None; cont.answer = None; }
    p.prev_pos = Vec3::from_array(p.fight.player.pos); p.prev_heading = p.fight.player.loco.facing;
    p.prev_rin = (Vec3::from_array(p.fight.rin.pos), p.fight.rin.facing);
    if let Some(o) = &mut p.gun_out { o.1 += 1.; if o.1 >= o.2 { p.gun_out = None; } }

    // Kreely's pistol (added; the native B button / chainsaw slot) [inferred flow, explorer clips]. Hold the secondary
    // to aim (Leonardo's aim loop), punch fires a round from the aim (5 a second; the explorer's recoil kick on the arm),
    // an empty magazine dry-fires and reloads, R reloads (the lowered idle while the magazine is swapped).
    let mut gun_input = false;
    let mut whip = false;
    // the pistol whip: aiming with an enemy directly in front (within WHIP_REACH and about 35 degrees of the facing)
    let point_blank = p.fight.kreely && {
        let pl = Vec3::from_array(p.fight.player.pos);
        let f = p.fight.player.loco.facing;
        let to_rin = Vec2::new(p.fight.rin.pos[0] - pl.x, p.fight.rin.pos[2] - pl.z);
        let rin_front = !p.fight.solo && to_rin.length() < WHIP_REACH && to_rin.normalize_or_zero().dot(Vec2::new(f.sin(), f.cos())) > 0.82;
        (rin_front || street.in_front(pl, f, WHIP_REACH)) && !vr::on()
    };
    let mut shot: Option<([f32; 3], [f32; 3])> = None;
    p.gun_events.clear();
    p.ammo.cool = p.ammo.cool.saturating_sub(1);
    p.ammo.recoil *= RECOIL_DECAY;
    if let Some(t) = p.ammo.reload {
        let t = t + 1;
        if t == RELOAD_SEAT {
            let take = (MAG - p.ammo.mag).min(p.ammo.reserve.min(255) as u8);
            p.ammo.mag += take; p.ammo.reserve -= take as u16;
        }
        if t >= RELOAD_TICKS { p.ammo.reload = None; p.gun_events.push(GunEvent::ReloadDone); } else { p.ammo.reload = Some(t); }
    }
    let can_reload = p.ammo.reload.is_none() && p.ammo.mag < MAG && p.ammo.reserve > 0;
    if p.as_kreely && (p.fight.player.state() == STATE_GUN || matches!(p.fight.player.state(), 0 | 1 | 2 | 4)) {
        let reloading = p.ammo.reload.is_some();
        if pad.reload && can_reload {
            p.ammo.reload = Some(0); p.gun_events.push(GunEvent::ReloadStart);
            set_gun(p, GunAction::PistolIdle, 10.);
        }
        if reloading || p.ammo.reload.is_some() {
            gun_input = pad.secondary || pad.punch;
            if p.gun.is_none() { set_gun(p, GunAction::PistolIdle, 10.); }
        } else if pad.secondary {
            gun_input = true;
            if !matches!(p.gun.as_ref().map(|g| g.action), Some(GunAction::Aim)) { set_gun(p, GunAction::Aim, AIM_BLEND); }
            else if pad.punch && point_blank {
                // an enemy right in front: a pistol whip instead of a shot (the fight takes it from the aim, mw_sim::kreely)
                whip = true;
            }
            else if pad.punch && p.ammo.cool == 0 {
                p.ammo.cool = FIRE_GAP;
                if p.ammo.mag == 0 {
                    p.gun_events.push(GunEvent::DryFire);
                    if can_reload { p.ammo.reload = Some(0); p.gun_events.push(GunEvent::ReloadStart); set_gun(p, GunAction::PistolIdle, 10.); }
                } else {
                    p.ammo.mag -= 1; p.ammo.recoil = 1.;
                    p.shots_fired.push(p.tick);
                    let pl = &p.fight.player;
                    let chest = Vec3::new(pl.pos[0], pl.pos[1] + 1.45, pl.pos[2]);
                    // the round goes where the sight is: it is traced along the camera ray through the screen centre,
                    // from level with the shooter (nothing behind him counts), so whatever it strikes lies under the
                    // reticle; the tracer is drawn from the gun to that point, as third-person shooters converge the
                    // gun on the reticle [added]
                    let fallback = { let (yaw, pitch) = (pl.loco.facing, (-p.cam_pitch).clamp(-0.5, 0.5) - 0.02);
                                     Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()) };
                    let (origin, dir) = if let Some((m, d)) = p.vr_aim { (m, d) } else { match cams.single() {
                        Ok(cam) => {
                            let (eye, look) = (cam.translation(), cam.forward().as_vec3());
                            // never shoot behind the shooter
                            if look.dot(fallback) > 0.3 { (eye + look * (chest - eye).dot(look).max(0.), look) } else { (chest, fallback) }
                        }
                        Err(_) => (chest, fallback),
                    } };
                    // run-and-gun: a moving shot strays up to ~2 degrees [added]
                    let dir = if p.run_gun && p.legs.weight > 0.3 && p.vr_aim.is_none() {
                        let h = (p.tick.wrapping_mul(2_654_435_761) >> 7) as u32;
                        let (u, v) = ((h & 0xFF) as f32 / 255. - 0.5, ((h >> 8) & 0xFF) as f32 / 255. - 0.5);
                        let right = Vec3::new(dir.z, 0., -dir.x).normalize_or_zero();
                        (dir + right * u * 0.07 + Vec3::Y * v * 0.05).normalize()
                    } else { dir };
                    // a real round leaves the muzzle and flies (fly_vr_rounds). In VR it goes down the barrel; on the
                    // screen from the gun's muzzle to what lies under the reticle (the camera ray's first target or
                    // wall), as third-person shooters converge the gun on the sight
                    let (muzzle, dir) = if p.vr_aim.is_some() { (origin, dir) } else {
                        let right = Vec3::new(dir.z, 0., -dir.x).normalize_or_zero();
                        let m = gun_muzzle.0.filter(|m| m.distance(origin) < 2.).unwrap_or(origin + dir * 0.55 - right * 0.06 - Vec3::Y * 0.04);
                        let mut reach = stage_hit(&arena.0, origin, dir, 60.).map_or(60., |h| h.0.distance(origin));
                        if street.on { if let Some(t) = street.ray_probe(origin, dir) { reach = reach.min(t); } }
                        let rin = Vec3::from_array(p.fight.rin.pos);
                        let mut body = |c: Vec3, r: f32, lo: f32, hi: f32| {
                            let t = (c - origin).dot(dir);
                            let q = origin + dir * t;
                            if t > 0. && Vec2::new(q.x - c.x, q.z - c.z).length() < r && (c.y + lo..c.y + hi).contains(&q.y) { reach = reach.min(t); }
                        };
                        if !p.fight.solo && p.fight.rin.hp > 0 { body(rin, 0.55, 0.3, 2.3); }
                        if elise.targetable() { body(elise.body(), 0.6, -1., 1.); }
                        if baron.targetable() { body(baron.body(), 0.6, -1., 1.); }
                        let aim = origin + dir * reach;
                        (m, (aim - m).normalize_or(dir))
                    };
                    let id = p.shots_fired.len() as u32;
                    p.vr_rounds.push(VrRound { pos: muzzle, vel: dir * VR_ROUND_SPEED, travelled: 0., bounces: 0, id, fan: false });
                    p.gun_events.push(GunEvent::Shot { muzzle, dir, end: None, kind: ShotEnd::Launched });
                    let _ = &mut shot;
                }
            }
        } else if matches!(p.gun.as_ref().map(|g| g.action), Some(GunAction::Aim)) {
            set_gun(p, GunAction::PistolIdle, 22.);
        } else if p.gun.as_ref().is_some_and(|g| g.action == GunAction::PistolIdle && g.frame >= p.gun_clips[&g.key].frames as f32 - 1.)
                  || (p.gun.is_some() && (pad.stick != Vec2::ZERO || pad.punch)) { end_gun(p); }
    }
    if p.as_kreely && p.fight.player.state() == 39 && pad.secondary && pad.punch && point_blank { whip = true; }
    if p.gun.is_some() && p.fight.player.state() != STATE_GUN { p.gun = None; p.ammo.reload = None; }   // knocked out of the pistol
    if let Some(g) = &mut p.gun {
        let n = p.gun_clips[&g.key].frames as f32;
        g.frame = if g.action == GunAction::Aim || p.ammo.reload.is_some() { (g.frame + 1.) % (n - 1.).max(1.) } else { (g.frame + 1.).min(n - 1.) };
        if let Some(f) = &mut g.from { f.1 += 1.; if f.1 >= f.2 { g.from = None; } }
        // aiming he squares up to the view; walking with the lowered pistol (a reload) he turns toward his way
        let stick = pad.stick;
        let want = if g.action != GunAction::PistolIdle { Some(p.cam_yaw) }
            else if stick.length_squared() >= mw_sim::player::STICK_MOVE { Some(p.cam_yaw + (-stick.x).atan2(stick.y)) } else { None };
        if let Some(want) = want {
            let f = &mut p.fight.player.loco.facing;
            *f = mw_sim::player::rad_lerp_range(*f, want, mw_sim::player::TURN_FRAC * mw_sim::player::VSYNC, mw_sim::player::TURN_CLAMP);
        }
    }

    // the native fight frame
    let keys_inp = FightInput {
        demo_active: false, // this standalone fight starts after the opening event
        stick: pad.stick.to_array(), camera_yaw: p.cam_yaw,
        a_trig: pad.punch && !gun_input, a_held: pad.punch_held && !gun_input,
        // Jack: holding Secondary (right mouse: chainsaw) holds B = weapon slot 1 (802C3338)
        b_trig: false, b_held: pad.secondary && !p.as_kreely,
        // the swing: RMB / Q (Jack's chainsaw). The clash (8028BCB0) answers only the remote's swing gesture during
        // RinRin's slalom window (rig0, frames 10-60 of 0105); on PC a punch press then counts as that swing too, and
        // Kreely (no chainsaw, RMB aims) swings with Q or a punch there [PC mapping, added]
        swing: if p.as_kreely && !(p.fight.rin.state == mw_sim::fight::rin_state::FINISH_WAIT || p.fight.rin.rig0) { 0 }
            else if pad.secondary_pressed && !(p.as_kreely && p.fight.rin.rig0) { 1 } else if pad.swing_h { 3 } else if pad.punch && p.fight.rin.rig0 { 1 } else { 0 },
        // held X = Gesture::Shake profile 9: +0x1088 mean 1.59 [observed]; the phase-14 vector means are assumed to follow it
        shake: if pad.shake { 1.59 } else { 0. }, shake_remote: if pad.shake { 1.59 } else { 0. }, shake_nunchuk: if pad.shake { 1.59 } else { 0. },
        cont: pad.cont, jump: pad.jump, nunchuk_shake: if pad.dodge { 1.59 } else { 0. },   // Shift = nunchuk shake
        // Kreely's moveset (added): Q is his heavy attack outside the clash / finisher windows, F grabs
        heavy: p.as_kreely && pad.swing_h && !(p.fight.rin.state == mw_sim::fight::rin_state::FINISH_WAIT || p.fight.rin.rig0),
        grab: p.as_kreely && pad.grab && p.fight.carry.is_none(), grab_grunt: street.can_grab(Vec3::from_array(p.fight.player.pos)), whip,
        // the native grab: a grunt to catch; while holding, F throws overhead, Q sideways, X head-butts [PC mapping]
        carry_ok: street.can_carry(Vec3::from_array(p.fight.player.pos), p.fight.player.loco.facing),
        // (the street's objects, play_props.rs: the same A hold picks one up when no grunt is in reach)
        pick_ok: props.can_pick(Vec3::from_array(p.fight.player.pos), p.fight.player.loco.facing),
        cover_ok: street.cover_target(Vec3::from_array(p.fight.player.pos), p.fight.player.loco.facing).is_some(),
        carry_lost: p.fight.carry.is_some() && street.carry_lost,
        // (Kreely: Q while carrying is his suplex on the grunt he holds; F still throws)
        throw_v: p.fight.carry.is_some() && pad.grab, throw_s: p.fight.carry.is_some() && pad.swing_h && !p.as_kreely,
        suplex_held: p.as_kreely && p.fight.carry.is_some_and(|c| !c.object) && pad.swing_h,
        headbutt: p.fight.carry.is_some() && x_held && !p.carry_shake,
        taunt: pad.taunt,
        gun_run: p.as_kreely && p.run_gun,
    };
    p.carry_shake = x_held;
    p.pad_now = (pad.shake, pad.grab, pad.swing_h);
    let cam_yaw = p.cam_yaw;
    p.fight.kreely = p.as_kreely && p.fd.kreely.is_some();
    let mut inp = match &mut p.bot { Some(bot) => { let fight = &p.fight; bot.input(fight, cam_yaw) } None => keys_inp };
    if intro.controls_blocked(){inp=FightInput{demo_active:true,..Default::default()};}
    // Kreely has no chainsaw: his swing only exists as the finisher entry (same rule as the keys above)
    if p.as_kreely && !(p.fight.rin.state == mw_sim::fight::rin_state::FINISH_WAIT || p.fight.rin.rig0) { inp.swing = 0; }
    if let Some(y) = p.ground { if !p.fight.player.airborne { p.fight.player.floor_y = y; } }
    // up onto a ledge (c303's crypt tiers): falling past a floor higher than the one he left lands him on it; within
    // LEDGE_ASSIST below its top he is lifted onto it [added assist: the native jump apex, ~1.6-1.9 m, meets the 1.79 m
    // steps at the edge of its reach]
    if let Some(y) = p.ground {
        const LEDGE_ASSIST: f32 = 0.35;
        let pl = &mut p.fight.player;
        if pl.airborne && y > pl.floor_y + 0.2 && pl.pos[1] >= y - LEDGE_ASSIST && pl.pos[1] <= y + 0.05 && pl.pos[1] < p.prev_pos.y {
            pl.pos[1] = y; pl.floor_y = y;
        } else if pl.airborne && y > pl.floor_y + 0.2 && pl.pos[1] > y { pl.floor_y = y; }
    }
    // the mantle [added, c303]: jumping forward into a ledge whose top is within MANTLE of his feet lifts him onto it
    // (the crypt tiers: 1.79 m steps against a ~1.73 m jump apex; no stairs lead up to the centre crypt)
    if boss_room() == Some("c303") && p.fight.player.airborne && inp.stick != [0.; 2] {
        const MANTLE: f32 = 0.6;
        let pl = &p.fight.player;
        let ahead = Vec3::from_array(pl.pos) + Vec3::new(pl.loco.facing.sin(), 0., pl.loco.facing.cos()) * 0.6;
        // (never through the room's own walls: the step must be clear of col.hkx / col_pl.hkx)
        let step = [ahead.x - pl.pos[0], ahead.z - pl.pos[2]];
        let (clear, _) = mw_sim::collision::slide(&arena.0, pl.pos, step);
        let open = (clear[0] - step[0]).abs() + (clear[1] - step[1]).abs() < 0.02;
        if let Some(top) = floor(&arena.0, ahead + Vec3::Y * MANTLE).filter(|_| open) {
            if top > pl.floor_y + 0.5 && top <= pl.pos[1] + MANTLE {
                let pl = &mut p.fight.player;
                pl.pos = [ahead.x, top, ahead.z]; pl.floor_y = top;
            }
        }
    }
    // follow camera fn_802C9F3C (PC-CAMERA.md): yaw chases behind the player only while the error is 25..135 deg,
    // at most 5 deg x +0x2764 (1.0 [inferred]) x GameVSyncF per frame; lock-on (+0x1254) chases the target direction;
    // a recenter (+0x27C2 countdown of 8) forces the chase with +0x2764 = 0.32 [step as a 0.32 RadLerp: inferred].
    // Free camera (PC, as the recomp hook): look input holds the view for 45 frames; then, with auto-follow, while Jack
    // moves the yaw eases back 6 % of the error, at most 2 deg/frame, until within 5 deg the original chase resumes.
    let free_aim = p.gun.as_ref().is_some_and(|g| g.action == GunAction::Aim) && !p.lock_on;
    // the lock target: RinRin or any street grunt; Z picks the one nearest the view's centre, Q / E step left / right
    // of the view, a fallen target hands over to the next [added]
    {
        let rin = &p.fight.rin;
        let mut cands: Vec<(LockTarget, Vec3)> = vec![];
        if !p.fight.solo && !p.fight.victory && rin.state != mw_sim::fight::rin_state::DEAD {
            let lying = rin.down || (rin.state == mw_sim::fight::rin_state::FINISH_WAIT && rin.motion.frame > 35.);
            cands.push((LockTarget::Rin, Vec3::from_array(rin.pos) + Vec3::Y * if lying { 1.1 } else { 2.15 }));
        }
        if let Some((hp, _, at)) = p.boss { if hp > 0 { cands.push((LockTarget::Boss, at + Vec3::Y * 1.1)); } }
        cands.extend(street.lock_points().into_iter().map(|(id, at, lying)| (LockTarget::Grunt(id), at + Vec3::Y * if lying { 0.9 } else { 2.3 })));
        let pl = Vec3::from_array(p.fight.player.pos);
        let bearing = |at: Vec3| { let b = (at.x - pl.x).atan2(at.z - pl.z) - p.cam_yaw; (b + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI };
        cands.retain(|c| c.1.distance(pl) < 15.);
        if p.fight.player.hp < 1 { p.lock_on = false; }   // the lock lets go when he dies
        if !p.lock_on || cands.is_empty() { p.lock = None; p.lock_at = None; p.lock_mark = None; p.lock_step = 0; if cands.is_empty() { p.lock_on = false; } }
        else {
            let held = p.lock.and_then(|t| cands.iter().find(|c| c.0 == t).copied());
            let pick = match held {
                Some(cur) if p.lock_step != 0 => {
                    // the next target past the current one in the stepped direction (bearing grows to the left), wrapping
                    let b0 = bearing(cur.1);
                    let mut others: Vec<(f32, (LockTarget, Vec3))> = cands.iter().filter(|c| c.0 != cur.0).map(|c| (bearing(c.1), *c)).collect();
                    others.sort_by(|a, b| a.0.total_cmp(&b.0));
                    let left = p.lock_step < 0;
                    let next = if left { others.iter().find(|o| o.0 > b0).or(others.first()) } else { others.iter().rev().find(|o| o.0 < b0).or(others.last()) };
                    next.map(|o| o.1).unwrap_or(cur)
                }
                Some(cur) => cur,
                // the nearest threat first, toward the view: a metre counts as ~9 degrees off centre
                None => *cands.iter().min_by(|a, b| (bearing(a.1).abs() + a.1.distance(pl) * 0.15).total_cmp(&(bearing(b.1).abs() + b.1.distance(pl) * 0.15))).unwrap(),
            };
            p.lock = Some(pick.0); p.lock_step = 0;
            p.lock_at = Some(pick.1);
            p.lock_mark = Some(p.lock_mark.map_or(pick.1, |m| m.lerp(pick.1, 0.25)));
        }
    }
    let manual = free_aim || (p.free_cam && p.tick <= p.mouse_tick + 45 && !p.lock_on && p.recenter == 0);
    if !manual {
        let pl = &p.fight.player;
        // aiming, the view sits off the shoulder (camera: side -0.6), so the lock turns it to put RinRin under the
        // sight from there, not from the shooter [added]
        let aiming = p.gun.as_ref().is_some_and(|g| g.action == GunAction::Aim);
        let want = if let (true, Some(m)) = (p.lock_on, p.lock_mark) {
            let mut pp = pl.pos;
            if aiming { let (s, c) = p.cam_yaw.sin_cos(); pp[0] += -c * AIM_SIDE; pp[2] += s * AIM_SIDE; }
            let toward = (m.x - pp[0]).atan2(m.z - pp[2]);
            if aiming { toward } else {
                // the fight camera [added, after Way of the Samurai's lock]: side-on to the line between the two, on the side it
                // is already on (so the stick's left / right runs along the line), see `camera`
                let (a, b) = (toward + std::f32::consts::FRAC_PI_2, toward - std::f32::consts::FRAC_PI_2);
                let off = |y: f32| ((y - p.cam_yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).abs();
                if off(a) <= off(b) { a } else { b }
            }
        } else { pl.loco.facing };
        let err = (want - p.cam_yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        let deg = err.abs().to_degrees();
        let moving = pad.stick != Vec2::ZERO;
        if p.recenter > 0 { p.cam_yaw += err * 0.32; p.recenter -= 1; p.cam_pitch = 0.; }
        else if p.free_cam && !p.lock_on && deg > 5. && p.mouse_tick > 0 && p.tick <= p.mouse_tick + 45 + 600 {
            if moving { let max = 2f32.to_radians(); p.cam_yaw += (err * 0.06).clamp(-max, max); }
        } else if p.lock_on || (25. ..=135.).contains(&deg) {
            let max = 5f32.to_radians() * mw_sim::player::VSYNC;
            p.cam_yaw += err.clamp(-max, max);
        }
        p.cam_pitch *= 0.9;   // pitch state RadLerps back to the mode's pitch
    }
    // the actor editor on RinRin (em0440): her commands, her hold
    editor::rinrin(p, &mut inp, &mut editor, &arena);
    // inspection: MW_RIN_TEST=keen opens RinRin's clash from tick 40 (her slalom window open 1.2 m in front, a swing)
    if (40..70).contains(&p.tick) && std::env::var("MW_RIN_TEST").as_deref() == Ok("keen") && !p.fight.solo
        && p.fight.rin.state != mw_sim::fight::rin_state::KEEN_COMPE {
        let f = p.fight.player.loco.facing;
        let pp = p.fight.player.pos;
        p.fight.rin.pos = [pp[0] + f.sin() * 1.2, pp[1], pp[2] + f.cos() * 1.2];
        p.fight.rin.facing = f + std::f32::consts::PI;
        p.fight.rin.state = mw_sim::fight::rin_state::SLALOM; p.fight.rin.rig0 = true; inp.swing = 1;
    }
    // inspection: MW_RIN_TEST=finish puts RinRin finish-ready (HP 1, finish_wait played out) 1.2 m in front at tick 40,
    // and swings from tick 45: the player's finisher on her (to compare with the last-life execution's presentation)
    if std::env::var("MW_RIN_TEST").as_deref() == Ok("finish") && !p.fight.solo {
        if p.tick == 40 {
            let f = p.fight.player.loco.facing;
            let pp = p.fight.player.pos;
            p.fight.rin.pos = [pp[0] + f.sin() * 1.2, pp[1], pp[2] + f.cos() * 1.2];
            p.fight.rin.facing = f + std::f32::consts::PI;
            p.fight.rin.hp = 1; p.fight.rin.state = mw_sim::fight::rin_state::FINISH_WAIT; p.fight.rin.motion.ended = true;
        }
        if (45..60).contains(&p.tick) && p.fight.rin.state == mw_sim::fight::rin_state::FINISH_WAIT { inp.swing = 1; }
    }
    let physics_seconds = p.fight.physics_seconds();
    let (mut pstep, rstep) = p.fight.step(&p.fd, &inp);
    // RinRin's clash cut-in (802956B4 phase 0: the competition's entry)
    if !p.fight.executing() && p.fight.events.iter().any(|e| matches!(e, mw_sim::fight::FightEvent::ClashStart)) { cutin::request(); }
    // the pair camera runs its own clock from its clip's start, on the player's motion steps (the live recomp: c4aa at the
    // player frame less one, c4ac at his frame through its hit-stops, the c4ab orbit on through his 51-frame hold loop)
    {
        let m = &p.fight.player.motion;
        if !p.fight.paired { p.pair_cam = (usize::MAX, 0.); }
        else if p.pair_cam.0 != m.clip { p.pair_cam = (m.clip, 0.); }
        else { p.pair_cam.1 += m.step(); }
    }
    // locked on, the walk / run strafes: the step goes along the stick (camera-relative), while he keeps facing the target
    // (the facing block below) [added]
    if p.lock_on && p.lock_at.is_some() && matches!(p.fight.player.state(), 1 | 2 | 4) && !p.fight.paired {
        let k = Vec2::new(pstep[0], pstep[1]).length();
        let stick = Vec2::from_array(inp.stick);
        if k > 1e-4 && stick.length_squared() >= mw_sim::player::STICK_MOVE {
            let a = inp.camera_yaw + (-stick.x).atan2(stick.y);
            pstep = [a.sin() * k, a.cos() * k];
        }
    }
    // facing the fight [added, PC]: locked on, he keeps facing the target (standing, walking, running, taunting and through
    // every attack's wind-up); not locked on, an attack turns him to the nearest enemy within AUTO_FACE m. After the
    // fight step, so its own stick facing does not undo it.
    if !p.fight.paired && p.fight.carry.is_none() && p.gun.is_none() {
        let pl = &p.fight.player;
        let pos = Vec3::from_array(pl.pos);
        let st = pl.state();
        // his attacks are state 39 (the punch strings, Kreely's God Hand moves); hit reactions and holds are not
        let attacking = st == 39 && pl.motion.frame < 10.;
        let target = if p.lock_on { p.lock_at.filter(|_| attacking || matches!(st, 0 | 1 | 2 | 4 | 84)) } else if attacking {
            let mut enemies: Vec<Vec3> = street.targets();
            if let Some((hp, _, at)) = p.boss { if hp > 0 { enemies.push(at); } }
            if !p.fight.solo && p.fight.rin.hp > 0 { enemies.push(Vec3::from_array(p.fight.rin.pos)); }
            enemies.into_iter().filter(|e| e.distance(pos) < AUTO_FACE).min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)))
        } else { None };
        if let Some(t) = target {
            let want = (t.x - pos.x).atan2(t.z - pos.z);
            let pl = &mut p.fight.player;
            let before = pl.loco.facing;
            // locked on he faces the target outright (the walk would turn him back to the stick every frame); an
            // unlocked attack turns him quickly through its wind-up
            pl.loco.facing = if p.lock_on { want } else { mw_sim::player::rad_lerp_range(pl.loco.facing, want, 0.6, 0.6) };
            if std::env::var_os("MW_FACE_LOG").is_some() && (attacking || p.tick % 15 == 0) {
                eprintln!("[face] tick {} state {st} lock {} attacking {attacking} target {:.1} m off {:.0} -> {:.0} deg", p.tick, p.lock_on, t.distance(pos),
                          ((want - before + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).to_degrees(),
                          ((want - pl.loco.facing + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).to_degrees());
            }
        }
    }
    if let Some((name, _)) = p.fight.player.loco.layer {
        let ln = p.fd.jack.clips[p.fd.jack_clip(name)].frames as f32;
        p.layer_frame = (p.layer_frame + p.fight.player.motion.rate) % (ln - 1.).max(1.);
    }
    // movement against the c201 collision
    p.wall_hits += p.fight.move_world(&arena.0, pstep, rstep, physics_seconds) as u64;
    // c303: inside the hall (its walls stand at |x| 15.3 and z -19.8; the gap behind the top tier let him slip through the
    // back wall once the tiers became climbable) [added bound]
    if boss_room() == Some("c303") {
        let pl = &mut p.fight.player;
        pl.pos[0] = pl.pos[0].clamp(-14.9, 14.9); pl.pos[2] = pl.pos[2].clamp(-19.3, 29.);
    }
    p.ground = floor(&arena.0, Vec3::from_array(p.fight.player.pos));
    p.ground_attr = ground_attr(&arena.0, Vec3::from_array(p.fight.player.pos));
    // the pistol walk's legs: the walk cycle at the pace he actually covers, played backward when he backs up
    p.prev_legs = p.legs;
    let moved = Vec2::new(p.fight.player.pos[0] - p.prev_pos.x, p.fight.player.pos[2] - p.prev_pos.z);
    let walking = p.gun.is_some() && moved.length() > 0.01;
    p.legs.weight = (p.legs.weight + if walking { 0.25 } else { -0.2 }).clamp(0., 1.);
    if walking {
        let rel = moved.x.atan2(moved.y) - p.fight.player.loco.facing;
        let rel = (rel + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        p.legs.angle = rel;
        let cycle = if p.run_gun { mw_sim::player::RUN } else { mw_sim::player::WALK };
        let n = (p.fd.jack.clips[p.fd.jack_clip(cycle)].frames.max(2) - 1) as f32;
        let pace = moved.length() / if p.run_gun { mw_sim::fight::GUN_RUN } else { mw_sim::fight::GUN_WALK };
        let step = if rel.abs() > 1.75 { -pace } else { pace };
        p.legs.frame = (p.legs.frame + step).rem_euclid(n);
    }
    let shot_hit = shot.map(|(o, d)| p.fight.player_shot(&p.fd, o, d, 60.));
    if let (Some((o, d)), Some(hit)) = (shot, shot_hit) {
        let (origin, dir) = (Vec3::from_array(o), Vec3::from_array(d));
        let right = Vec3::new(dir.z, 0., -dir.x).normalize_or_zero();
        // the gun's muzzle as last posed (else an estimate beside the aim line)
        let muzzle = gun_muzzle.0.filter(|m| m.distance(origin) < 2.).unwrap_or(origin + dir * 0.55 - right * 0.06 - Vec3::Y * 0.04);
        // the nearest stage surface along a ray (where a miss or a ricochet lands), and its normal
        let surface_hit = |from: Vec3, dir: Vec3, range: f64| -> Option<(Vec3, Vec3)> {
            let od = [from.x as f64, from.y as f64, from.z as f64]; let d = dir.normalize_or_zero();
            mw_sim::collision::raw_surface_ray(&arena.0, od, unit(d), range, None).ok()
                .and_then(|h| h.iter().filter(|h| h.distance > 0.05).min_by(|a, b| a.distance.total_cmp(&b.distance))
                    .map(|h| (from + d * h.distance as f32, Vec3::new(h.normal[0] as f32, h.normal[1] as f32, h.normal[2] as f32).normalize_or(Vec3::Y))))
        };
        let surface = |from: Vec3, dir: Vec3| surface_hit(from, dir, 60.).map(|h| h.0);
        // a per-shot pseudo-random draw in [-1, 1]
        let seed = p.shots_fired.len() as u32;
        let draw = |k: u32| { let x = (seed.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B)).wrapping_mul(0xC2B2_AE35); (x >> 8) as f32 / (1u32 << 23) as f32 - 1. };
        // the street's grunts stand in the line of fire first
        let grunt = if street.on { street.shoot(origin, dir, 20, f32::INFINITY, &mut fx) } else { None };
        let boss_hit = if grunt.is_none() && elise.on { elise.shoot(origin, dir, 20, f32::INFINITY, &mut fx) } else if grunt.is_none() && baron.on { baron.shoot(origin, dir, 20, f32::INFINITY, &mut fx) } else { None };
        let hit = if let Some(at) = boss_hit { mw_sim::fight::Shot::Hit(at.to_array()) } else { hit };
        // the round goes in and comes out: flesh hit sprays and wounds on whoever it struck
        match (grunt, hit) {
            (Some((at, root)), _) => fx.wound(Some(root), at, dir, 0.32),
            // (a boss's round already left its own wounds: baron / elise shoot)
            (None, mw_sim::fight::Shot::Hit(at)) if boss_hit.is_none() => fx.wound(None, Vec3::from_array(at), dir, 0.24),
            _ => {}
        }
        let hit = if let Some((at, _)) = grunt { mw_sim::fight::Shot::Hit(at.to_array()) } else { hit };
        let (end, kind) = match hit {
            mw_sim::fight::Shot::Hit(at) => (Some(Vec3::from_array(at)), ShotEnd::Flesh),
            mw_sim::fight::Shot::Miss => (surface(origin, dir), ShotEnd::Stage),
            mw_sim::fight::Shot::Deflect(at) => {
                // off the fan: back toward the shooter's side, sprayed sideways and up, then caroming off the stage
                let at = Vec3::from_array(at);
                let mut d = (-dir + right * draw(1) * 0.7 + Vec3::Y * (0.05 + 0.3 * draw(2).abs())).normalize();
                let chest = Vec3::from_array(p.fight.player.pos) + Vec3::Y * 1.15;
                let (mut from, mut bounces, mut n, mut hurt) = (at, [at; RICOCHETS], 0, false);
                while n < RICOCHETS {
                    let hit = surface_hit(from, d, 30.);
                    let to = hit.map_or(from + d * 30., |h| h.0);
                    // past the shooter: the closest approach along this leg
                    let t = (chest - from).dot(d).clamp(0., from.distance(to));
                    let near = from + d * t;
                    if near.distance(chest) < RICOCHET_REACH && ((draw(10 + n as u32) + 1.) * 50.) < RICOCHET_CHANCE as f32 {
                        bounces[n] = near; n += 1; hurt = true;
                        p.fight.enemy_hits_player(&p.fd, 0x172, RICOCHET_POWER, at.to_array());
                        break;
                    }
                    bounces[n] = to; n += 1;
                    let Some((pt, normal)) = hit else { break; };
                    // reflect, losing a little true: each carom scatters the round
                    d = (d - 2. * d.dot(normal) * normal + Vec3::new(draw(20 + n as u32), draw(30 + n as u32), draw(40 + n as u32)) * 0.25).normalize();
                    from = pt + normal * 0.02;
                }
                (Some(at), ShotEnd::Fan { bounces, n, hurt })
            }
        };
        p.gun_events.push(GunEvent::Shot { muzzle, dir, end, kind });
    }
    let strikes = fly_vr_rounds(p, &mut street, &mut elise, &mut baron, &mut fx, &arena.0);
    // a landed melee hit feeds the pistol's reserve
    if p.as_kreely && p.fight.events.iter().any(|e| matches!(e, mw_sim::fight::FightEvent::RinHit { .. })) && shot.is_none() {
        p.ammo.reserve = (p.ammo.reserve + MELEE_REFILL).min(RESERVE_MAX);
    }
    p.last_events = p.fight.events.iter().map(|e| format!("{e:?}")).collect();
    for s in strikes { p.last_events.push(format!("PistolShot {{ hit: {s} }}")); }
    if let Some(hit) = shot_hit { p.last_events.push(format!("PistolShot {{ hit: {} }}", match hit { mw_sim::fight::Shot::Miss => "miss", mw_sim::fight::Shot::Hit(_) => "hit", mw_sim::fight::Shot::Deflect(_) => "deflect" })); }
    for g in &p.gun_events { if !matches!(g, GunEvent::Shot { .. }) { p.last_events.push(format!("Pistol{g:?}")); } }
    if p.as_kreely { p.last_events.push(format!("Ammo {{ mag: {}, reserve: {} }}", p.ammo.mag, p.ammo.reserve)); }
    if let Some(f) = &mut p.log {
        let pl = &p.fight.player; let rin = &p.fight.rin;
        let _ = writeln!(f, "{}", serde_json::json!({"tick": p.tick, "who": if p.as_kreely {"kreely"} else {"jack"},
            "state": pl.state(), "phase": pl.loco.phase, "clip": p.fd.jack.clips[pl.motion.clip].name, "frame": pl.motion.frame, "rate": pl.motion.rate,
            "gun": p.gun.as_ref().map(|g| format!("{:?}", g.action)), "pos": pl.pos, "heading": pl.loco.facing, "hp": pl.hp,
            "rin": {"state": rin.state, "clip": p.fd.rin.clips[rin.motion.clip].name, "frame": rin.motion.frame, "pos": rin.pos, "hp": rin.hp},
            "events": p.last_events, "ground": p.ground, "wallHits": p.wall_hits, "shotsFired": p.shots_fired.len()}));
    }
    if let Some(end) = std::env::var("MW_PLAY_TICKS").ok().and_then(|v| v.parse::<u64>().ok()) { if p.tick >= end { exit.write(AppExit::Success); } }
    // evidence runs: MW_PLAY_EXIT_AFTER_VICTORY=n quits n ticks after Victory
    if let Some(n) = std::env::var("MW_PLAY_EXIT_AFTER_VICTORY").ok().and_then(|v| v.parse::<u64>().ok()) {
        if p.fight.victory { if p.victory_tick == 0 { p.victory_tick = p.tick; } if p.tick >= p.victory_tick + n { exit.write(AppExit::Success); } }
    }
}

/// Motion sound cues (`_10.seq`) crossed this game frame by Jack/Kreely and RinRin. fn_80259854 plays op 29 (SE, e.g.
/// the punch whooshes 1101:n) and op 42 (voice) itself; the rest go to the owner's jkRigEvent. The player's
/// (fn_802C1C7C): op 37 footsteps play block 1000 + the ground attribute (table 8056A3E8, attribute 0 -> 1001; the c201
/// floor is 0x0C -> 1012), id operand b; op 38 only stores the impact sound (a*1000+b at +0x110C), which fn_802788F0
/// plays when the swing lands. RinRin's op 37: blocks 7030 / 7031 (onSeqFootStepCall); her op 38 plays directly [her
/// cEnemyBase::jkRigEvent is not traced]. Track delays are scheduled in game frames [unit inferred].
/// The bosses' motion sequence op-16 effect cues (a = EST id; c = 2 their own bank, 0 core), as RinRin's in play_fx.rs:
/// Elise em0630, the Baron em0623, Mathilda em0610 (the bank follows the clip's model prefix).
fn boss_effect_cues(play: Res<Play>, elise: Res<elise::Elise>, baron: Res<baron::Baron>, mut fx: ResMut<fx::Fx>, mut last: Local<HashMap<usize, (String, f32)>>) {
    let mut actors = baron.motions();
    if let Some((clip, frame)) = elise.motion() { actors.push((9, clip.to_string(), frame, elise.pos, elise.yaw)); }
    for (key, clip, frame, at, yaw) in actors {
        let prev = last.entry(key).or_default();
        let from = if prev.0 != clip || frame < prev.1 { -1. } else { prev.1 };
        *prev = (clip.clone(), frame);
        // (Jack's swings on Mathilda carry his own weapon-flash cues: not hers)
        if clip.starts_with("pl0030_") { continue; }
        let own = ["em0623", "em0630", "em0610"].into_iter().find(|b| clip.starts_with(b));
        let Some(cues) = play.fd.combat.cues.get(&clip.replace(".mot", "_00.seq")) else { continue; };
        for c in cues.iter().filter(|c| c.op == 16 && c.frame > from && c.frame <= frame) {
            let bank = match c.c { 2 => match own { Some(b) => b, None => continue }, 0 => "core", _ => continue };
            fx.burst(bank, c.a as u16 as u32, at, yaw, 1.);
        }
    }
}

fn cue_sounds(play: Res<Play>, street: Res<street::Street>, elise: Res<elise::Elise>, mut last_elise: Local<(String, f32)>, baron: Res<baron::Baron>, breakables: Res<breakables::Breakables>,
              mut last_baron: Local<HashMap<usize, (String, f32)>>, mut boss_hp: Local<Option<i16>>, mut snd: ResMut<Sounds>) {
    if snd.bank.is_empty() { return; }
    // Kreely's pistol speaks with Jude's gun (em0400, block 4201 of se/scene_mix0125.rsp): 4201:0 the shot, 4201:201
    // the double click as the magazine seats and the slide drops, 4201:7 the hammer on an empty chamber [picked from
    // the waves; Jude never reloads or dry-fires]
    let pl = play.fight.player.pos;
    for g in &play.gun_events {
        let id = match g { GunEvent::Shot { .. } => GUN_SHOT, GunEvent::DryFire => GUN_DRY, GunEvent::ReloadDone => GUN_SEAT, GunEvent::ReloadStart => GUN_RELEASE, GunEvent::Impact { .. } | GunEvent::Leg { .. } => continue };
        queue_sound(&mut snd, play.tick, GUN_BLOCK, id, pl);
    }
    // the vampire zako's cries (play_street.rs)
    for &(block, id, at) in &street.voices { queue_sound(&mut snd, play.tick, block, id, at.to_array()); }
    let step_block = { let b = 1000 + play.ground_attr.clamp(1, 29) as u32; if snd.bank.contains_key(&b.to_string()) { b } else { 1001 } };
    let actors = [(&play.fight.player.motion, &play.fd.jack.clips, false, play.fight.player.pos),
                  (&play.fight.rin.motion, &play.fd.rin.clips, true, play.fight.rin.pos)];
    for (k, (m, clips, rin, pos)) in actors.into_iter().enumerate() {
        let (pclip, pstarts, pframe) = snd.last[k];
        let from = if pclip != m.clip || pstarts != m.starts { -1. } else { pframe };
        snd.last[k] = (m.clip, m.starts, m.frame);
        if m.frame <= from { continue; }
        let seq = clips[m.clip].name.replace(".mot", "_10.seq");
        let Some(cues) = play.fd.combat.cues.get(&seq) else { continue; };
        for c in cues.iter().filter(|c| c.frame > from && c.frame <= m.frame) {
            let (block, id) = match c.op {
                29 | 42 => (c.a as u16 as u32, c.b as u16 as u32),
                38 if rin => (c.a as u16 as u32, c.b as u16 as u32),
                38 => { snd.impact = (c.a >= 0).then(|| (c.a as u32, c.b as u16 as u32)); continue; }
                // the foot/variant is operand b; operand a is 0 in every staged op-37 cue
                37 => if rin { (if play.fight.rin.state == mw_sim::fight::rin_state::RUN { 7030 } else { 7031 }, c.b.max(0) as u32) } else { (step_block, c.b.max(0) as u32) },
                _ => continue,
            };
            queue_sound(&mut snd, play.tick, block, id, pos);
        }
    }
    // Elise's own motion cues (em0630_*_10.seq: her wing, landing and bat SE from se/scene_mix0303.rsp)
    if let Some((clip, frame)) = elise.motion() {
        let from = if last_elise.0 != clip || frame < last_elise.1 { -1. } else { last_elise.1 };
        *last_elise = (clip.to_string(), frame);
        // her sequences are the _00 variant (em0630_0110_00.seq); 4209 is her SE block, 8122 her voiced SE
        let seq = |suffix: &str| play.fd.combat.cues.get(&clip.replace(".mot", suffix));
        if let Some(cues) = seq("_10.seq").or_else(|| seq("_00.seq")) {
            for c in cues.iter().filter(|c| c.frame > from && c.frame <= frame && matches!(c.op, 29 | 42)) { queue_sound(&mut snd, play.tick, c.a as u16 as u32, c.b as u16 as u32, elise.pos.to_array()); }
        }
    }
    // the Baron's and Mathilda's motion cues (em0623 / em0610 _00 sequences: 4213 SE, 8127 the Baron's voice, 7100)
    for (key, clip, frame, at, _) in baron.motions() {
        let last = last_baron.entry(key).or_default();
        let from = if last.0 != clip || frame < last.1 { -1. } else { last.1 };
        *last = (clip.clone(), frame);
        let seq = |suffix: &str| play.fd.combat.cues.get(&clip.replace(".mot", suffix));
        if let Some(cues) = seq("_10.seq").or_else(|| seq("_00.seq")) {
            for c in cues.iter().filter(|c| c.frame > from && c.frame <= frame && matches!(c.op, 29 | 42)) { queue_sound(&mut snd, play.tick, c.a as u16 as u32, c.b as u16 as u32, at.to_array()); }
        }
    }
    for (block, id, at) in &baron.sfx { queue_sound(&mut snd, play.tick, *block, *id, at.to_array()); }
    // the church's pillars breaking and their pieces landing (play_breakables.rs)
    for (block, id, at) in &breakables.sfx { queue_sound(&mut snd, play.tick, *block, *id, at.to_array()); }
    // Kreely's suplex slam: the heaviest punch impact (block 11, 9)
    for e in &play.fight.events { if let mw_sim::fight::FightEvent::SuplexSlam { at, .. } = e { queue_sound(&mut snd, play.tick, 11, 9, *at); } }
    // the swing landed (on RinRin or a street grunt): its stored impact plays once
    // (Elise / the Baron / Mathilda: their gauge's HP dropped this frame)
    let hp = play.boss.map(|b| b.0);
    let boss_struck = matches!((*boss_hp, hp), (Some(a), Some(b)) if b < a);
    *boss_hp = hp;
    let landed = street.landed || boss_struck || play.fight.events.iter().any(|e| matches!(e, mw_sim::fight::FightEvent::RinHit { atk, .. } if *atk != 0x61));
    if landed { if let Some((block, id)) = snd.impact.take() { queue_sound(&mut snd, play.tick, block, id, pl); } }
}

/// PlayVoiceStream (8026B558): one announcer stream at a time; the same id while it plays is dropped, another id waits
/// in a 4-slot queue for 0x5A frames; an id on cooldown (+0x1BB8, set when it plays, -1 per frame [unit inferred]) is
/// dropped. Group choice follows 8026BCE0 (groups in the entry mask, least used in a 256-entry history, never the last
/// one again when all were used), clip choice 8026C008 (random in the group). Draws use a separate cRand instance
/// [native: the shared one]. Not ported: the room-specific queue gate (+0x15EE > 2) and the 8008BCDC(0x14) override.
/// The voice mix: the bosses' lines and the commentators' (linear gain on the announcer bus).
const BOSS_VOICE_GAIN: f32 = 2.0;
const BOSS_SE_VOICE_GAIN: f32 = 1.8;
const COMMENTARY_GAIN: f32 = 0.8;
fn announce(play: Res<Play>, sound:Res<audio::PortAudio>, mut an: ResMut<Announcer>, assets: Res<AssetServer>, playing: Query<&VoiceClip>, mut commands: Commands) {
    if an.table.is_empty() { return; }
    for v in an.cooldown.values_mut() { if *v > 0 { *v -= 1; } }
    let now = play.tick;
    for e in &play.fight.events {
        if let mw_sim::fight::FightEvent::Voice(id) = *e {
            if std::env::var_os("MW_ANNOUNCE_LOG").is_some() { eprintln!("[announce] tick {now} asked {id:#x} (queue {:?})", an.queue); }
            if !an.queue.iter().any(|q| q.0 == id) && an.queue.len() < 4 { an.queue.push((id, now + 0x5A)); }
        }
    }
    an.queue.retain(|q| q.1 >= now);
    let busy: Vec<u16> = playing.iter().map(|v| v.0).collect();
    an.queue.retain(|q| !busy.contains(&q.0));
    // the bosses' own lines (ids under 0x1000: RinRin 0x53.., Elise 0x9C.., the Baron 0x10E..) have a stream of their
    // own beside the commentators' (0x1000 up), so a boss is heard over the commentary [added; native: one stream]
    // (RinRin's room's 0x53..0x5B are the commentators on her moves, not her: commentary)
    let boss = |id: u16| id < 0x1000 && !(0x53..=0x5B).contains(&id);
    let (boss_busy, com_busy) = (busy.iter().any(|&i| boss(i)), busy.iter().any(|&i| !boss(i)));
    if !sound.commentary_on { an.queue.retain(|q| boss(q.0)); }
    let Some(k) = an.queue.iter().position(|q| if boss(q.0) { !boss_busy } else { !com_busy }) else { return; };
    let (id, _) = an.queue.remove(k);
    let Some(entry) = an.table.get(&id.to_string()).cloned() else { return; };
    // (the actor editor replays a line as often as it is asked)
    if an.cooldown.get(&id).copied().unwrap_or(0) > 0 && !editor::on() { return; }
    let groups: Vec<u8> = (0..8u8).filter(|g| entry.mask & (1 << g) != 0 && entry.clips.iter().any(|c| c.group == *g)).collect();
    if groups.is_empty() { return; }
    if std::env::var_os("MW_ANNOUNCE_LOG").is_some() { eprintln!("[announce] tick {now} plays {id:#x}"); }
    let used: Vec<usize> = groups.iter().map(|g| an.history.iter().filter(|h| h.0 == id && h.1 == *g).count()).collect();
    let last = an.history.iter().rev().find(|h| h.0 == id).map(|h| h.1);
    let mut seed = an.rng;
    let mut rand = |n: usize| -> usize {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (((seed >> 16) & 0xFFFF) as usize * n) >> 16
    };
    let group = if groups.len() == 1 { groups[0] }
        else if used.iter().all(|&u| u == 0) { groups[rand(groups.len())] }
        else if used.iter().all(|&u| u > 0) {
            let min = *used.iter().min().unwrap();
            let pool: Vec<u8> = groups.iter().zip(&used).filter(|(_, u)| **u <= min).map(|(g, _)| *g).collect();
            let mut g = pool[rand(pool.len())];
            for _ in 0..7 { if Some(g) != last { break; } g = pool[rand(pool.len())]; }
            g
        } else {
            let pool: Vec<u8> = groups.iter().zip(&used).filter(|(_, u)| **u == 0).map(|(g, _)| *g).collect();
            pool[rand(pool.len())]
        };
    let clips: Vec<&AnClip> = entry.clips.iter().filter(|c| c.group == group).collect();
    let clip = clips[rand(clips.len())].file.clone();
    an.rng = seed;
    an.history.push_back((id, group)); if an.history.len() > 255 { an.history.pop_front(); }
    an.cooldown.insert(id, entry.cooldown);
    if std::env::var_os("MW_SOUND_LOG").is_some() { println!("[voice] tick {now} {id:#06x} group {group} {clip}"); }
    // boss lines louder, the commentary a little under them [added mix]
    let (bus, base) = if boss(id) { (audio::Bus::Announcers, BOSS_VOICE_GAIN) } else { (audio::Bus::Commentary, COMMENTARY_GAIN) };
    commands.spawn((VoiceClip(id),audio::Gain{bus,base}, AudioPlayer::new(assets.load::<AudioSource>(clip)), PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(sound.gain(bus,base)))));
}

/// Jude's gun sounds (block 4201) for Kreely's pistol, see `cue_sounds`.
const GUN_BLOCK: u32 = 4201;
const GUN_SHOT: u32 = 0;
const GUN_DRY: u32 = 7;
const GUN_RELEASE: u32 = 200;
const GUN_SEAT: u32 = 201;

fn queue_sound(snd: &mut Sounds, tick: u64, block: u32, id: u32, pos: [f32; 3]) {
    let Some(sound) = snd.bank.get(&block.to_string()).and_then(|b| b.get(&id.to_string())).cloned() else { return; };
    for p in &sound.plays {
        // the bosses' voiced SE (RinRin 8117, Elise 8122, the Baron 8127: their cries and shouts in the fight) louder, so
        // they carry over the commentary [added mix]
        let voice = if matches!(block, 8117 | 8122 | 8127) { BOSS_SE_VOICE_GAIN } else { 1. };
        let vol = sound.volume as f32 / 127. * p.volume as f32 / 127. * voice;
        // staged track "delays" carry a flag byte (0x0A/0x0B) above the 60 Hz wait: only the low byte is time
        // [inferred]; reading the whole word scheduled layers 40+ s late as stray hits.
        let at = tick + (p.delay as u64 & 0xFF) / 2;
        let speed = 2f32.powf((p.note as f32 - p.root as f32) / 12.);
        snd.pending.push((at, p.file.clone(), vol, [pos[0], pos[1] + 1., pos[2]], speed));
    }
}

fn play_sounds(play: Res<Play>, sound:Res<audio::PortAudio>, mut snd: ResMut<Sounds>, assets: Res<AssetServer>, mut commands: Commands) {
    let now = play.tick;
    let due: Vec<(String, f32, [f32; 3], f32)> = snd.pending.iter().filter(|p| p.0 <= now).map(|p| (p.1.clone(), p.2, p.3, p.4)).collect();
    snd.pending.retain(|p| p.0 > now);
    for (file, vol, at, speed) in due.into_iter().take(12) {
        if std::env::var_os("MW_SOUND_LOG").is_some() { println!("[sound] tick {now} {file} vol {vol:.2}"); }
        // positioned at the emitting actor, heard from the camera (PC: rodio stereo panning with 1/d^2 falloff, distances
        // scaled by 0.4 so the follow distance plays at full volume) [native 3D curve/pitch/envelopes are not applied]
        commands.spawn((audio::Gain{bus:audio::Bus::Sfx,base:vol},AudioPlayer::new(assets.load::<AudioSource>(file)), Transform::from_translation(Vec3::from_array(at)),
                        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(sound.gain(audio::Bus::Sfx,vol))).with_speed(speed)
                            .with_spatial(true).with_spatial_scale(bevy::audio::SpatialScale::new(0.4))));
    }
}

/// Boss BGM: the intro, then the loop section on repeat once the intro's length has elapsed (a gap of up to one frame
/// at the seam); stopped at Victory [inferred: the result-screen jingle is not ported].
/// Staged native mixer metadata supplies the gain; old assets retain their inferred0.6 fallback.
fn bgm(play: Res<Play>, sound:Res<audio::PortAudio>, time: Res<Time<Virtual>>, mut bgm: ResMut<Bgm>, assets: Res<AssetServer>, voices: Query<Entity, With<BgmVoice>>,
       mut baron: ResMut<baron::Baron>, mut commands: Commands) {
    if let Some(name) = baron.bgm_switch.take() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let track = std::fs::read_to_string(root.join("assets/native/audio/bgm/bgm.json")).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| serde_json::from_value::<BgmTrack>(v.get(name)?.clone()).ok());
        if track.is_some() && std::env::var_os("MW_NO_BGM").is_none() {
            for e in &voices { commands.entity(e).despawn(); }
            bgm.track = track; bgm.started = None; bgm.looping = false; bgm.stopped = false;
        }
    }
    let Some(track) = bgm.track.clone() else { return; };
    if bgm.stopped { return; }
    if play.fight.victory {
        for e in &voices { commands.entity(e).despawn(); }
        bgm.stopped = true; return;
    }
    let now = time.elapsed_secs_f64();
    let base=track.mix.map(StreamMix::gain).unwrap_or(0.6);
    let vol = bevy::audio::Volume::Linear(sound.gain(audio::Bus::Music,base));
    match bgm.started {
        None => {
            commands.spawn((BgmVoice,audio::Gain{bus:audio::Bus::Music,base}, AudioPlayer::new(assets.load::<AudioSource>(track.intro.clone())), PlaybackSettings::DESPAWN.with_volume(vol)));
            bgm.started = Some(now);
        }
        Some(t0) if !bgm.looping && now - t0 >= track.loop_start as f64 / track.rate as f64 => {
            commands.spawn((BgmVoice,audio::Gain{bus:audio::Bus::Music,base}, AudioPlayer::new(assets.load::<AudioSource>(track.looped.clone())), PlaybackSettings::LOOP.with_volume(vol)));
            bgm.looping = true;
            if std::env::var_os("MW_INTRO_SMOKE").is_some(){eprintln!("[intro-audio-smoke] boss loop after{}s from event cue; gain={base}",now-t0);}
        }
        _ => {}
    }
}

/// Floor height under `pos` (nearest col.hkx surface below a point 1.5 above it).
/// RinRin's pair cameras (em0440.dat pl0030_c4aa..c4ad for the keen competition, c305..c309; tools/stage_camera_clips.py):
/// while Jack plays a paired clip with a camera twin (pl0030_0xxx -> pl0030_cxxx), the view follows it, anchored at
/// RinRin (cEmBoss::moveCatchedPos places Jack in her frame).
fn rin_pair_camera(play: &Play) -> Option<(Vec3, Vec3, f32)> {
    static CAMS: std::sync::OnceLock<HashMap<String, elise::CamClip>> = std::sync::OnceLock::new();
    if play.fight.solo || !(play.fight.paired || play.fight.player.state() == 78) { return None; }
    let cams = CAMS.get_or_init(|| elise::load_cams("rinrin-camera.json"));
    let m = &play.fight.player.motion;
    // (her clash pl0030_04xx -> c4xx; her finisher pl0030_13xx -> c3xx: 1305 -> c305 ... 1309 -> c309)
    let clip = &play.fd.jack.clips.get(m.clip)?.name;
    // (her scissors win, 04adsc, rides the fan cut's camera c4ad stretched over its longer clip)
    let scissors = clip == "pl0030_04adsc.mot";
    let name = if scissors { "pl0030_c4ad.mot".to_string() } else if clip.starts_with("pl0030_13") { clip.replacen("pl0030_13", "pl0030_c3", 1) } else { clip.replacen("pl0030_0", "pl0030_c", 1) };
    let r = &play.fight.rin;
    let cam = cams.get(&name)?;
    // its own clock (tick's pair_cam), looping over the MOT's length (c4ab: a 150-frame orbit under his 51-frame hold)
    let n = (cam.frames().max(2) - 1) as f32;
    let f = if scissors { m.frame * n / (play.fd.jack.clips[m.clip].frames.max(2) - 1) as f32 }
        else if play.pair_cam.0 == m.clip { if m.looping { play.pair_cam.1 % n } else { play.pair_cam.1.min(n) } } else { m.frame };
    // inspection: MW_CAM_LOG=1 names the camera MOT each pair frame plays
    let (eye, target, fov) = cam.sample(f, Vec3::from_array(r.pos), r.facing);
    if std::env::var_os("MW_CAM_LOG").is_some() {
        eprintln!("[cam] tick {} rinrin {name} f{:.1}", play.tick, f);
        eprintln!("PORT_CAM_ROW {{\"clip\":\"{}\",\"pf\":{:.4},\"rin\":[{:.5},{:.5},{:.5}],\"facing\":{:.5},\"eye\":[{:.5},{:.5},{:.5}],\"dest\":[{:.5},{:.5},{:.5}]}}",
                  play.fd.jack.clips[m.clip].name, m.frame, r.pos[0], r.pos[1], r.pos[2], r.facing, eye.x, eye.y, eye.z, target.x, target.y, target.z);
    }
    Some((eye, target, fov))
}

/// The boss room this run plays (MW_STAGE): None is c201 (RinRin or the street), c303 Elise, c503 the Black Baron (or the
/// Mathilda variant).
fn boss_room() -> Option<&'static str> { if elise::on() { Some("c303") } else if baron::on() { Some("c503") } else { None } }

/// [added] The nearest walkable surface under a point (stage or objects: crates, steps), for small things that fall.
fn floor_any(world: &mw_data::collision::CollisionWorld, pos: Vec3) -> Option<f32> {
    let o = [pos.x as f64, pos.y as f64 + 0.1, pos.z as f64];
    mw_sim::collision::raw_surface_ray(world, o, [0., -1., 0.], 30., None).ok()
        .and_then(|hits| hits.iter().filter(|h| h.normal[1] > 0.5).map(|h| h.distance).min_by(f64::total_cmp).map(|d| (o[1] - d) as f32))
}

fn floor(world: &mw_data::collision::CollisionWorld, pos: Vec3) -> Option<f32> {
    let o = [pos.x as f64, pos.y as f64 + 1.5, pos.z as f64];
    // c303's crypt tiers (1.79 / 3.56 m, the steps to the centre crypt) and coffin ledges are col_obj.hkx floors: the
    // original's player stands on them (the live recomp's clash at a far crypt stood him at 1.79 + 0.1)
    let objects = boss_room() == Some("c303");
    mw_sim::collision::raw_surface_ray(world, o, [0., -1., 0.], 6., None).ok()
        .and_then(|hits| hits.iter().filter(|h| h.body.source == "col.hkx" || (objects && h.body.source == "col_obj.hkx"))
            .map(|h| h.distance).min_by(f64::total_cmp).map(|d| (o[1] - d) as f32))
}

/// cHavok::getRigidAttribute (800B0B94) for the floor under `pos`: the two hex digits after "_ST" in the col.hkx body's
/// name (32 and up, or none, read as 0). The c201 boss floor cube3219_ST0c is 0x0C.
fn ground_attr(world: &mw_data::collision::CollisionWorld, pos: Vec3) -> u8 {
    let o = [pos.x as f64, pos.y as f64 + 1.5, pos.z as f64];
    mw_sim::collision::raw_surface_ray(world, o, [0., -1., 0.], 6., Some("col.hkx")).ok()
        .and_then(|hits| hits.into_iter().min_by(|a, b| a.distance.total_cmp(&b.distance)))
        .and_then(|h| { let n = &h.body.name; let i = n.find("_ST")?; n.get(i + 3..i + 5).and_then(|x| u8::from_str_radix(x, 16).ok()) })
        .filter(|v| *v < 32).unwrap_or(0)
}

/// The arena's grating floor and chain-link fences are alpha cut-outs: the original alpha-tests those draws (the stage
/// program is texture x vertex color, recomp GX dump evidence/native-look-capture.log), while the exported materials are
/// opaque. Switch the arena materials to an alpha test once both scenes are in (as alpha to coverage under the 4x MSAA,
/// play_look.rs).
/// The electric fence arcs (texture 33: white strokes on opaque black) and the lamp flares (texture 34) are glows: black
/// adds nothing, so they blend additively [inferred: the GX blend of those draws was not dumped; drawn opaque they show
/// black panels].
fn arena_cutouts(arenas: Query<Entity, With<ArenaVariant>>, children: Query<&Children>, mats: Query<&MeshMaterial3d<StandardMaterial>>,
                 mut materials: ResMut<Assets<StandardMaterial>>, assets: Res<AssetServer>, mut done: Local<bool>) {
    if *done { return; }
    let found: Vec<_> = arenas.iter().flat_map(|a| children.iter_descendants(a)).filter_map(|e| mats.get(e).ok().map(|m| (e, m.0.clone()))).collect();
    if found.len() < 2 * 235 { return; }   // c201 has 235 materials per variant
    let mut glows = 0;
    for (_, h) in found {
        let Some(mut m) = materials.get_mut(&h) else { return; };
        let glow = m.base_color_texture.as_ref().and_then(|t| assets.get_path(t)).is_some_and(|p| matches!(p.label(), Some("Texture33" | "Texture34")));
        m.alpha_mode = if glow { AlphaMode::Add } else { AlphaMode::AlphaToCoverage };
        glows += glow as u32;
    }
    info!("[arena] {glows} additive glow draws (electric arcs, flares)");
    *done = true;
}

/// Collect each character's joints (role_0_bone<id>, native order + origin) once its scene is spawned.
fn bind_joints(mut chars: Query<(Entity, &mut Character)>, children: Query<&Children>, names: Query<&Name>, meshes: Query<Entity, Added<Mesh3d>>,
               play: Res<Play>, mut commands: Commands) {
    for e in &meshes { commands.entity(e).insert(NoFrustumCulling); }
    for (root, mut ch) in &mut chars {
        if ch.joints.is_some() { continue; }
        let ids: Vec<i16> = match ch.who {
            Who::Jack => play.fd.jack.bones.iter().map(|b| b.id).collect(),
            Who::Kreely => play.kreely_bones.iter().map(|b| b.id).collect(),
            Who::Rin => play.fd.rin.bones.iter().map(|b| b.id).collect(),
        };
        let mut found: HashMap<String, Entity> = HashMap::new();
        for d in children.iter_descendants(root) {
            if let Ok(n) = names.get(d) { if n.as_str().starts_with("role_0_bone") { found.insert(n.as_str().to_string(), d); } }
        }
        let mut joints = Vec::new();
        for id in ids.iter().copied().chain(std::iter::once(-1)) {
            match found.get(&format!("role_0_bone{id}")) { Some(e) => joints.push(*e), None => { joints.clear(); break; } }
        }
        if !joints.is_empty() { ch.joints = Some(joints); }
    }
}

/// Collect the chainsaw arm's joints (role_0_bone<id> in wp0600 order + origin) once its scene is spawned.
fn bind_arm(mut arm: Query<(Entity, &mut ChainsawArm)>, children: Query<&Children>, names: Query<&Name>, intro: Res<intro::Intro>) {
    let Some(bones) = intro.model_bones("wp0600") else { return; };
    for (root, mut ch) in &mut arm {
        if ch.0.is_some() { continue; }
        let found: HashMap<String, Entity> = children.iter_descendants(root).filter_map(|d| names.get(d).ok().map(|n| (n.as_str().to_string(), d))).collect();
        let joints: Option<Vec<Entity>> = bones.iter().map(|b| b.id).chain(std::iter::once(-1)).map(|id| found.get(&format!("role_0_bone{id}")).copied()).collect();
        ch.0 = joints;
    }
}

/// Local pose of the player character for a source (Jack bank clip or Kreely's pistol overlay).
fn sample_src(p: &Play, kreely: bool, src: &Src) -> Vec<LocalPose> {
    match (kreely, src) {
        (true, Src::Gun(k, f)) => {
            let ov = &p.kreely.overlays[k];
            let scale = p.kreely.retarget.root_scale_by_family.get(&ov.family).copied().unwrap_or(p.kreely.retarget.root_scale);
            sample_overlay(&p.kreely_bones, &p.gun_clips[k], *f, p.kreely.retarget.rotation_only, scale, ov)
        }
        (true, Src::Jack(i, f)) => sample_retargeted(&p.kreely_bones, &p.fd.jack.clips[*i], *f,
            Retarget { rotation_only: p.kreely.retarget.rotation_only, root_scale: p.kreely.retarget.root_scale }),
        (false, Src::Jack(i, f)) => sample_retargeted(&p.fd.jack.bones, &p.fd.jack.clips[*i], *f, Retarget::NATIVE),
        (false, Src::Gun(..)) => sample_retargeted(&p.fd.jack.bones, &p.fd.jack.clips[p.fd.jack_clip("pl0030_0000.mot")], 0., Retarget::NATIVE),
    }
}

fn set_joints<F:bevy::ecs::query::QueryFilter>(joints: &[Entity], poses: &[LocalPose], q: &mut Query<&mut Transform,F>) {
    for (e, p) in joints.iter().zip(poses) {
        if let Ok(mut t) = q.get_mut(*e) {
            t.translation = Vec3::from_array(p.translation);
            t.rotation = Quat::from_array(p.rotation);
            t.scale = Vec3::from_array(p.scale);
        }
    }
}

fn motion_pose(bones: &[Bone], clips: &[Clip], m: &Motion, a: f32) -> Vec<LocalPose> {
    let last = (clips[m.clip].frames.max(2) - 1) as f32;
    let f = if m.ended { m.frame } else { (m.frame + a * m.step()).min(last) };
    let mut poses = sample_retargeted(bones, &clips[m.clip], f, Retarget::NATIVE);
    if let Some((c, bf, k, n)) = m.blend {
        let from = sample_retargeted(bones, &clips[c], bf, Retarget::NATIVE);
        poses = crossfade(&from, &poses, k + a * m.blend_step(), n);
    }
    poses
}

fn pose(play: Res<Play>, fixed: Res<Time<Fixed>>, mut chars: Query<(&Character, &mut Transform, &mut Visibility), (Without<ArenaVariant>, Without<ChainsawArm>)>,
        mut joints: Query<&mut Transform, (Without<Character>, Without<ArenaVariant>, Without<ChainsawArm>)>,
        mut arenas: Query<(&ArenaVariant, &mut Visibility), (Without<Character>, Without<ChainsawArm>)>,
        mut arm: Query<(&ChainsawArm, &mut Transform, &mut Visibility), (Without<Character>, Without<ArenaVariant>)>, arena: Res<Arena>, intro:Res<intro::Intro>,
        mut muzzle: ResMut<tracer::Muzzle>, mut hurt: ResMut<hurt::Hurt>, execution: Res<execution::Execution>, elise: Res<elise::Elise>, baron: Res<baron::Baron>) {
    for (v, mut vis) in &mut arenas { *vis = if v.0 == play.colored { Visibility::Visible } else { Visibility::Hidden }; }
    for (_, _, mut vis) in &mut arm { *vis = Visibility::Hidden; }
    let a = fixed.overstep_fraction();
    let lerp_yaw = |a0: f32, a1: f32| a0 + ((a1 - a0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI) * a;
    let pl = &play.fight.player;
    let pos = play.prev_pos.lerp(Vec3::from_array(pl.pos), a);
    let heading = lerp_yaw(play.prev_heading, pl.loco.facing);
    for (ch, mut t, mut vis) in &mut chars {
        let Some(js) = &ch.joints else { continue; };
        if intro.active(){
            // the intro's player actor (pl0030) is whoever plays: Kreely takes Jack's motion on his own rig
            let name=match ch.who{Who::Jack=>if play.as_kreely{""}else{"pl0030"},Who::Rin=>"em0440",Who::Kreely=>if play.as_kreely{"pl0030"}else{""}};
            let sample=intro.actor(name).filter(|_|ch.colored==play.colored);
            *vis=if sample.is_some(){Visibility::Visible}else{Visibility::Hidden};
            if let Some((clip,frame))=sample{
                *t=Transform::default();
                let poses=match ch.who{
                    Who::Rin=>sample_retargeted(&play.fd.rin.bones,clip,frame,Retarget::NATIVE),
                    Who::Kreely=>sample_retargeted(&play.kreely_bones,clip,frame,Retarget{rotation_only:play.kreely.retarget.rotation_only,root_scale:play.kreely.retarget.root_scale}),
                    Who::Jack=>sample_retargeted(&play.fd.jack.bones,clip,frame,Retarget::NATIVE),
                };
                set_joints(js,&poses,&mut joints);
            }
            continue;
        }
        let shown = match ch.who { Who::Rin => ch.colored == play.colored && !play.fight.solo, Who::Jack => !play.as_kreely && ch.colored == play.colored, Who::Kreely => play.as_kreely && ch.colored == play.colored };
        *vis = if shown { Visibility::Visible } else { Visibility::Hidden };
        if !shown { continue; }
        if ch.who == Who::Rin {
            let rin = &play.fight.rin;
            let mut at = play.prev_rin.0.lerp(Vec3::from_array(rin.pos), a);
            // The actor stands ~0.1 above col.hkx [observed]; her feet are drawn on the floor itself. Larger gaps are
            // jumps/scaffolds and keep the actor height.
            if let Some(y) = floor(&arena.0, at) { if (0. ..0.35).contains(&(at.y - y)) { at.y = y; } }
            *t = Transform::from_translation(at).with_rotation(Quat::from_rotation_y(lerp_yaw(play.prev_rin.1, rin.facing)));
            let mut poses = motion_pose(&play.fd.rin.bones, &play.fd.rin.clips, &rin.motion, a);
            if let Some((c, w)) = rin.layer {
                let n = (play.fd.rin.clips[c].frames.max(2) - 1) as f32;
                let layer = sample_retargeted(&play.fd.rin.bones, &play.fd.rin.clips[c], rin.motion.frame.min(n), Retarget::NATIVE);
                poses = mix(&poses, &layer, w);
            }
            if !play.fight.paired && rin.state != mw_sim::fight::RIN_SUPLEXED { strip_root_xz(&play.fd.rin.bones, &mut poses); }
            for fan in rin.fans.iter().filter(|f| f.phase != 0) {
                if let Some(i) = play.fd.rin.bones.iter().position(|b| b.id == fan.part) {
                    let local = mw_sim::fight::from_world(rin.pos, rin.facing, fan.pos);
                    mw_sim::fight::place_bone(&play.fd.rin.bones, &mut poses, i, local, fan.spin);
                }
            }
            set_joints(js, &poses, &mut joints);
            continue;
        }
        *t = Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(heading));
        let kreely = ch.who == Who::Kreely;
        let bones = if kreely { &play.kreely_bones } else { &play.fd.jack.bones };
        let mut poses = match &play.gun {
            Some(g) => {
                let mut cur = sample_src(&play, kreely, &Src::Gun(g.key.clone(), g.frame));
                if let Some((src, k, n)) = &g.from { cur = crossfade(&sample_src(&play, kreely, src), &cur, k + a, *n); }
                cur
            }
            None => {
                let m = &pl.motion;
                let last = (play.fd.jack.clips[m.clip].frames.max(2) - 1) as f32;
                let f = if m.ended { m.frame } else { (m.frame + a * m.step()).min(last) };
                let mut cur = sample_src(&play, kreely, &Src::Jack(m.clip, f));
                if let Some((c, bf, k, n)) = m.blend { cur = crossfade(&sample_src(&play, kreely, &Src::Jack(c, bf)), &cur, k + a * m.blend_step(), n); }
                if let Some((name, w)) = pl.loco.layer {
                    let layer = sample_src(&play, kreely, &Src::Jack(play.fd.jack_clip(name), play.layer_frame));
                    cur = mix(&cur, &layer, w);
                }
                if let Some((src, k, n)) = &play.gun_out { cur = crossfade(&sample_src(&play, kreely, src), &cur, k + a, *n); }
                cur
            }
        };
        if !play.fight.paired && pl.state() != mw_sim::fight::STATE_SUPLEX { strip_root_xz(bones, &mut poses); }
        // Elise's clash: his body trails his clip's root a frame, as the original's (play_elise.rs PAIR_LAG)
        if let Some(lag) = elise.pair_lag(&play) { if let Some(r) = poses.get_mut(bones.len()) { r.translation[0] += lag.x; r.translation[2] += lag.z; } }
        if kreely && play.gun.is_some() { gun_legs(&play, bones, &mut poses, a); }
        // run-and-gun [added experiment, after Max Payne 3's one-handed pistol]: only the gun arm and the torso hold the
        // aim; the free arm (bones 10-14) swings with the run while he moves and hangs from his idle when he stands
        if kreely && play.run_gun && play.gun.as_ref().is_some_and(|g| g.action == GunAction::Aim) { one_hand(&play, bones, &mut poses, a); }
        if kreely { hurt::apply(bones, &mut poses, &hurt); }
        // aiming, the torso follows the view up and down (spine and chest share the camera's pitch) [added]
        if kreely && play.gun.as_ref().is_some_and(|g| g.action == GunAction::Aim) {
            let pitch = play.cam_pitch.clamp(-0.6, 0.8);
            for (id, share) in [(1i16, 0.45f32), (2, 0.55)] {
                let Some(i) = bones.iter().position(|b| b.id == id) else { continue; };
                let frames = mw_sim::fight::bone_frames(bones, &poses);
                let parent = if bones[i].parent >= 0 { Quat::from_array(frames[bones[i].parent as usize].0) } else { Quat::from_array(poses[bones.len()].rotation) };
                let world = Quat::from_rotation_x(-pitch * share) * Quat::from_array(frames[i].0);
                poses[i].rotation = (parent.inverse() * world).normalize().to_array();
            }
        }
        if kreely { if let Some(r) = play.ammo.reload { reload::apply(bones, &mut poses, r as f32 + a); } }
        // the pistol's muzzle in the world, for the tracers: the gun prop bone (1600) plus whichever end of the gun mesh
        // points furthest along the facing (the mesh's long axis runs grip to barrel)
        if kreely {
            if let Some(i) = bones.iter().position(|b| b.id == 1600) {
                let (r, p, sc) = mw_sim::fight::bone_frames(bones, &poses)[i];
                let (r, p, sc) = (Quat::from_array(r), Vec3::from_array(p), Vec3::from_array(sc));
                let world = |o: Vec3| *t * (p + r * (o * sc));
                let fwd = t.rotation * Vec3::Z;
                let ends = [world(Vec3::new(0.01, 0.168, -0.101)), world(Vec3::new(-0.005, -0.052, 0.065))];
                let base = world(Vec3::ZERO);
                let front = (ends[0] - base).dot(fwd) > (ends[1] - base).dot(fwd);
                muzzle.0 = Some(if front { ends[0] } else { ends[1] });
                muzzle.1 = Some(if front { ends[1] } else { ends[0] });
            }
        }
        // his mouth, for the teeth a low-HP face hit knocks out
        if kreely {
            if let Some(i) = bones.iter().position(|b| b.id == 4) {
                let (r, p, _) = mw_sim::fight::bone_frames(bones, &poses)[i];
                hurt.mouth = Some((*t * (Vec3::from_array(p) + Quat::from_array(r) * Vec3::new(0., 0.05, 0.1)), t.rotation * Vec3::Z));
            }
        }
        // pistol recoil: the muzzle climbs through the whole arm (shoulder, upper arm, forearm, wrist flick) and the
        // chest rocks back, easing out over a few frames (explorer: 9 deg at the wrist; the arm share is added)
        let recoil = play.ammo.recoil * (1. - a * (1. - RECOIL_DECAY));
        if kreely && recoil > 0.01 {
            let kick = recoil * 16f32.to_radians();
            for (id, share) in [(2i16, 0.12f32), (5, 0.15), (7, 0.35), (8, 0.3), (9, 0.45), (12, 0.2), (13, 0.15)] {
                let Some(i) = bones.iter().position(|b| b.id == id) else { continue; };
                let frames = mw_sim::fight::bone_frames(bones, &poses);
                let parent = if bones[i].parent >= 0 { Quat::from_array(frames[bones[i].parent as usize].0) } else { Quat::from_array(poses[bones.len()].rotation) };
                let world = Quat::from_rotation_x(-kick * share) * Quat::from_array(frames[i].0);
                poses[i].rotation = (parent.inverse() * world).normalize().to_array();
            }
        }
        if !kreely {
            if let (Some(i), Ok((ch, mut at, mut vis))) = (bones.iter().position(|b| b.id == ARM_BONE), arm.single_mut()) {
                let (r, p, s) = mw_sim::fight::bone_frames(bones, &poses)[i];
                *at = *t * Transform { translation: Vec3::from_array(p), rotation: Quat::from_array(r), scale: Vec3::from_array(s) };
                *vis = Visibility::Visible;
                let m = &pl.motion;
                // the chainsaw arm's own clip beside his paired one (pl0030_NNNN -> wp0600_NNNN), from the boss's bank: RinRin's
                // clash (04aa-04ad), Elise's (0460-0466), the Baron's clash and finale (0480-048b)
                let clip = play.fd.jack.clips[m.clip].name.strip_prefix("pl0030_").map(|k| format!("wp0600_{k}"));
                let found = clip.as_deref().and_then(|n| play.fd.rin.clips.iter().find(|c| c.name == n).or_else(|| elise.bank_clip(n)).or_else(|| baron.bank_clip(n)));
                if let (Some(js), Some(wp_bones), Some(c)) = (&ch.0, intro.model_bones("wp0600"), found) {
                    let wp = mw_sim::character::sample_retargeted(wp_bones, c, m.frame.min((c.frames.max(2) - 1) as f32), Retarget::NATIVE);
                    set_joints(js, &wp, &mut joints);
                }
            }
        }
        if kreely {
            // the pistol stays in his hand (with its grip) through a pistol whip
            let whipping = play.gun.is_none() && play.fd.jack.clips[pl.motion.clip].name.starts_with("kreely_gh_whip");
            let gun = play.gun.as_ref().map(|g| &play.kreely.overlays[&g.key]).or_else(|| whipping.then(|| &play.kreely.overlays[&gun_key(&play, GunAction::PistolIdle)]));
            let hand = play.kreely.hand_pose(gun.and_then(|o| o.hand_pose.as_deref()));
            let hidden: Vec<i16> = if gun.is_some() { vec![0x641] } else { play.kreely.prop_bones.clone() };
            apply_hand_pose(bones, &mut poses, &hand, &hidden);
        }
        // inspection: MW_POSE_LOG=1 prints the player's posed joint positions (actor space: hips, chest, neck, head,
        // feet) and the clip, to compare retargets against their source
        if std::env::var_os("MW_POSE_LOG").is_some() && ch.colored == play.colored {
            let fr = mw_sim::fight::bone_frames(bones, &poses);
            let at = |id: i16| bones.iter().position(|b| b.id == id).map(|i| fr[i].1.map(|v| (v * 100.).round() / 100.));
            eprintln!("[pose] {:?} {} f{:.1} hips {:?} chest {:?} neck {:?} head {:?} footR {:?} footL {:?} armR {:?} {:?} {:?} armL {:?} {:?} {:?}", ch.who,
                      play.fd.jack.clips[pl.motion.clip].name, pl.motion.frame, at(0), at(2), at(3), at(4), at(18), at(22), at(7), at(8), at(9), at(12), at(13), at(14));
        }
        // the last-life execution's cut (play_execution.rs): the head joint scaled away [added]
        if execution.head_off { if let Some(i) = bones.iter().position(|b| b.id == 4) { poses[i].scale = [0.001; 3]; } }
        set_joints(js, &poses, &mut joints);
    }
}

/// The free arm in run-and-gun: bones 10-14 (the left clavicle to hand) from Jack's run at the legs' cycle frame, weighted
/// by how much he is moving, else from his idle (pl0030_0000).
fn one_hand(play: &Play, bones: &[Bone], poses: &mut [LocalPose], a: f32) {
    let (l0, l1) = (play.prev_legs, play.legs);
    let weight = l0.weight + (l1.weight - l0.weight) * a;
    let run = play.fd.jack_clip(mw_sim::player::RUN);
    let n = (play.fd.jack.clips[run].frames.max(2) - 1) as f32;
    let moving = sample_src(play, true, &Src::Jack(run, l1.frame.rem_euclid(n)));
    let idle = sample_src(play, true, &Src::Jack(play.fd.jack_clip("pl0030_0000.mot"), 0.));
    for id in 10i16..=14 {
        let Some(i) = bones.iter().position(|b| b.id == id) else { continue; };
        let free = mix(&idle[i..=i], &moving[i..=i], weight)[0];
        poses[i] = mix(&poses[i..=i], &[free], 0.9)[0];
    }
}

/// Kreely's legs (bone 15's chain) from Jack's walk while he walks with the pistol out, the hips turned toward the
/// walking direction (up to 60 degrees off the facing; backward walks run the cycle in reverse).
fn gun_legs(play: &Play, bones: &[Bone], poses: &mut [LocalPose], a: f32) {
    let (l0, l1) = (play.prev_legs, play.legs);
    let weight = l0.weight + (l1.weight - l0.weight) * a;
    if weight <= 0.01 { return; }
    let clip = play.fd.jack_clip(if play.run_gun { mw_sim::player::RUN } else { mw_sim::player::WALK });
    let n = (play.fd.jack.clips[clip].frames.max(2) - 1) as f32;
    let mut df = l1.frame - l0.frame;
    if df.abs() > n * 0.5 { df -= df.signum() * n; }
    let walk = sample_src(play, true, &Src::Jack(clip, (l0.frame + df * a).rem_euclid(n)));
    let Some(hips) = bones.iter().position(|b| b.id == 15) else { return; };
    let legs: Vec<usize> = (0..bones.len()).filter(|&i| {
        let mut j = i;
        loop { if j == hips { return true; } if bones[j].parent < 0 { return false; } j = bones[j].parent as usize; }
    }).collect();
    for &i in &legs { poses[i] = mix(&poses[i..=i], &walk[i..=i], weight)[0]; }
    let back = l1.angle.abs() > 1.75;
    let off = if back { (l1.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI } else { l1.angle };
    let off = off.clamp(-1.05, 1.05) * weight;
    let frames = mw_sim::fight::bone_frames(bones, poses);
    let parent = if bones[hips].parent >= 0 { Quat::from_array(frames[bones[hips].parent as usize].0) } else { Quat::from_array(poses[bones.len()].rotation) };
    let world = Quat::from_rotation_y(off) * Quat::from_array(frames[hips].0);
    poses[hips].rotation = (parent.inverse() * world).normalize().to_array();
}

fn bombs(play: Res<Play>, mut q: Query<(&BombVisual, &mut Transform, &mut Visibility)>) {
    for (b, mut t, mut v) in &mut q {
        // gone once it has blown (its blast is the bl0063 EST 0 effect, play_fx.rs)
        match play.fight.rin.bombs.get(b.0).filter(|bomb| bomb.blast <= 0. && b.1 == play.colored) {
            Some(bomb) => {
                let turn = Quat::from_array(bomb.turn);
                *t = Transform::from_translation(Vec3::from_array(bomb.pos) + turn * Vec3::new(0., -mw_sim::fight::BOMB_RADIUS, 0.)).with_rotation(turn);
                *v = Visibility::Visible;
            }
            None => *v = Visibility::Hidden,
        }
    }
}

/// Mode-0 follow camera (fn_802C9A98 table 805692A4: dist 2.25, side -0.2, height 1.5, pitch -0.026 rad, fov 50 deg;
/// fn_802C9F3C eye = dest + R z dist, pulled in front of walls by checkLine).
/// A direction for the collision rays, normalized in f64 (they reject anything off unit length by 1e-8, which an f32
/// normalization does not meet).
fn unit(d: Vec3) -> [f64; 3] {
    let v = [d.x as f64, d.y as f64, d.z as f64];
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0. { v.map(|x| x / l) } else { [0., 0., 1.] }
}

/// How far a camera can sit from `from` along `dir` (up to `max`) before the stage, its props (col_obj) or its camera
/// blockers (col_cm) get in the way: a small sweep (the centre ray and four rays 0.25 m around it), held 0.3 m clear.
fn camera_clear(world: &mw_data::collision::CollisionWorld, from: Vec3, dir: Vec3, max: f32) -> f32 {
    let dir = dir.normalize_or(Vec3::Z);
    let side = dir.cross(Vec3::Y).normalize_or(Vec3::X);
    let up = side.cross(dir);
    let mut clear = max;
    for o in [Vec3::ZERO, side * 0.25, -side * 0.25, up * 0.25, -up * 0.25] {
        let p = from + o;
        for source in ["col.hkx", "col_obj.hkx", "col_cm.hkx"] {
            if let Ok(hits) = mw_sim::collision::raw_surface_ray(world, [p.x as f64, p.y as f64, p.z as f64], unit(dir), max as f64, Some(source)) {
                if let Some(h) = hits.iter().map(|h| h.distance).min_by(f64::total_cmp) { clear = clear.min(h as f32 - 0.3); }
            }
        }
    }
    clear.max(0.6)
}

/// Starts and ends the defeat shot and runs its slow motion on the virtual clock (the fixed game frame follows it).
fn defeat(play: Res<Play>, real: Res<Time<Real>>, mut time: ResMut<Time<Virtual>>, mut cam: ResMut<DefeatCam>) {
    let dead = play.fight.player.state() == 95;
    let now = real.elapsed_secs_f64();
    match (dead, cam.since) {
        (true, None) => { cam.since = Some(now); cam.yaw = play.cam_yaw; }
        (false, Some(_)) => { cam.since = None; time.set_relative_speed(1.); return; }
        _ => {}
    }
    let Some(t0) = cam.since else { return; };
    let t = now - t0;
    let speed = if t < DEFEAT_SLOW_SECS { DEFEAT_SLOW }
        else { DEFEAT_SLOW + (1. - DEFEAT_SLOW) * ((t - DEFEAT_SLOW_SECS) / DEFEAT_EASE_SECS).min(1.) as f32 };
    if (time.relative_speed() - speed).abs() > 1e-4 { time.set_relative_speed(speed); }
}

fn camera(play: Res<Play>, arena: Res<Arena>, fixed: Res<Time<Fixed>>, mut cam: Query<(&mut Transform,&mut Projection), (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>,intro:Res<intro::Intro>,
          tv: Res<broadcast::Broadcast>, defeat: Res<DefeatCam>, real: Res<Time<Real>>, elise: Res<elise::Elise>, baron: Res<baron::Baron>, ed: Res<editor::Editor>,
          execution: Res<execution::Execution>) {
    // a boss pair's own camera MOT (Elise's clash and finisher: pl0030_c46x; the Baron's c48x / em0623_c00x / em0960_c000)
    // (MW_CAM_AT, the fixed inspection camera, also overrides the pair / boss cameras)
    let cam_fixed = std::env::var_os("MW_CAM_AT").is_some();
    // the town's cut-scene shots (play_town.rs)
    if let Some((eye, target, fov)) = town::camera() {
        for (mut t, mut projection) in &mut cam {
            *t = Transform::from_translation(eye).looking_at(target, Vec3::Y);
            if let Projection::Perspective(p) = &mut *projection { p.fov = fov; }
        }
        return;
    }
    // the actor editor: a camera clip it previews over everything, its orbit under the pair cameras
    let pair_cam = rin_pair_camera(&play).is_some() || elise.camera(&play).is_some() || baron.camera(&play).is_some();
    // (the orbit held clear of walls, as the defeat pan)
    let orbit = if pair_cam { None } else { ed.orbit_camera(&play).map(|(eye, target, fov)| {
        let d = eye - target; let n = d.normalize_or(Vec3::Z);
        (target + n * camera_clear(&arena.0, target, n, d.length()), target, fov)
    }) };
    if let Some((eye, target, fov)) = ed.shot_camera(&play).or(orbit) {
        for (mut t, mut projection) in &mut cam {
            *t = Transform::from_translation(eye).looking_at(target, Vec3::Y);
            if let Projection::Perspective(p) = &mut *projection { p.fov = fov; }
        }
        return;
    }
    if let Some((eye, target, fov)) = tv.camera().filter(|_| !cam_fixed).or_else(|| if cam_fixed { None } else { rin_pair_camera(&play) }).or_else(|| if cam_fixed { None } else { elise.camera(&play).or_else(|| baron.cine_camera(&play)).or_else(|| baron.camera(&play)) }) {
        for (mut t, mut projection) in &mut cam {
            *t = Transform::from_translation(eye).looking_at(target, Vec3::Y);
            if let Projection::Perspective(p) = &mut *projection { p.fov = fov; }
        }
        return;
    }
    if let Some(pose)=intro.camera(){
        for(mut transform,mut projection)in &mut cam{
            *transform=Transform::from_translation(Vec3::from_array(pose.eye)).looking_at(Vec3::from_array(pose.target),Vec3::from_array(pose.up));
            if let Projection::Perspective(perspective)=&mut *projection{perspective.fov=pose.fov_radians;}
        }return;
    }
    let a = fixed.overstep_fraction();
    if let Some(t0) = defeat.since {
        // the defeat pan: around the fallen player, held clear of walls
        let now = real.elapsed_secs_f64();
        // after a finisher: the pan stops where it was and the camera turns once round him
        let (turn, t) = match execution.orbit(now) { Some((a, t1)) => (a, (t1 - t0) as f32), None => (0., (now - t0) as f32) };
        let yaw = defeat.yaw + std::f32::consts::PI * 0.25 + t * DEFEAT_PAN + turn;
        let center = play.prev_pos.lerp(Vec3::from_array(play.fight.player.pos), a) + Vec3::Y * 0.6;
        let out = Vec3::new(-yaw.sin(), 0.35, -yaw.cos()).normalize();
        let mut dist = camera_clear(&arena.0, center, out, DEFEAT_DIST);
        // keep RinRin (taunting over the body) from filling the shot: pull in in front of her
        let rin = Vec3::from_array(play.fight.rin.pos) + Vec3::Y * 0.9;
        let along = (rin - center).dot(out);
        if along > 0. && along < dist + 0.5 && (center + out * along).distance(rin) < 0.7 { dist = dist.min((along - 0.7).max(1.)); }
        let eye = center + out * dist;
        for (mut tr, mut projection) in &mut cam {
            *tr = Transform::from_translation(eye).looking_at(center, Vec3::Y);
            if let Projection::Perspective(p) = &mut *projection { p.fov = 50f32.to_radians(); }
        }
        return;
    }
    let aiming = play.gun.as_ref().is_some_and(|g| g.action != GunAction::PistolIdle);
    let fwd = Vec3::new(play.cam_yaw.sin(), 0., play.cam_yaw.cos());
    let right = Vec3::new(-fwd.z, 0., fwd.x);
    let side = if aiming { AIM_SIDE } else { -0.2 };   // aiming shoulder offset is the added pistol's [inferred]
    let dest = play.prev_pos.lerp(Vec3::from_array(play.fight.player.pos), a) + Vec3::Y * 1.5 + right * side;
    // inspection: MW_CAM_PITCH=<rad> tilts the follow camera (negative looks down from above)
    let pitch = -0.026 + play.cam_pitch + std::env::var("MW_CAM_PITCH").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.);
    let dist = if aiming { 2.0 } else { 2.25 };
    let back = Vec3::new(-fwd.x * pitch.cos(), -pitch.sin(), -fwd.z * pitch.cos());
    let mut eye_dist = dist;
    let o = [dest.x as f64, dest.y as f64, dest.z as f64];
    let (bx, by, bz) = (back.x as f64, back.y as f64, back.z as f64); let l = (bx * bx + by * by + bz * bz).sqrt();
    if let Ok(hits) = mw_sim::collision::raw_surface_ray(&arena.0, o, [bx / l, by / l, bz / l], dist as f64, Some("col.hkx")) {
        if let Some(h) = hits.iter().map(|h| h.distance).min_by(f64::total_cmp) { eye_dist = (h as f32 - 0.2).max(0.5); }
    }
    let eye = dest + back * eye_dist + Vec3::Y * 0.25;
    // locked on (not aiming): the side-on fight camera over the middle of the two, back as far as their distance needs
    let aiming_now = play.gun.as_ref().is_some_and(|g| g.action == GunAction::Aim);
    let (eye, dest) = match (play.lock_on && !aiming_now, play.lock_mark) {
        (true, Some(m)) => {
            let pl = play.prev_pos.lerp(Vec3::from_array(play.fight.player.pos), a);
            let target = Vec3::new(m.x, pl.y, m.z);
            let mid = (pl + target) * 0.5 + Vec3::Y * 1.1;
            let gap = Vec2::new(target.x - pl.x, target.z - pl.z).length();
            let fwd = Vec3::new(play.cam_yaw.sin(), 0., play.cam_yaw.cos());
            (mid - fwd * (3.6 + gap * 0.75) + Vec3::Y * 0.55, mid)
        }
        _ => (eye, dest),
    };
    // inspection: MW_CAM_AT=ex,ey,ez,tx,ty,tz places the camera at a fixed eye looking at a target
    let (eye, dest) = match std::env::var("MW_CAM_AT").ok().map(|v| v.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>()) {
        Some(v) if v.len() == 6 => (Vec3::new(v[0], v[1], v[2]), Vec3::new(v[3], v[4], v[5])),
        _ => (eye, dest),
    };
    // inspection: MW_CAM_RIN / MW_CAM_PL=right,up,forward frames RinRin's / the player's head from that offset in their
    // own facing frame
    let cam_on = |key: &str| std::env::var(key).ok().map(|v| v.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 3);
    // inspection: MW_CAM_BOSS=right,up,forward frames the boss's chest from that offset (world axes)
    let (eye, dest) = match (cam_on("MW_CAM_BOSS"), play.boss) {
        (Some(v), Some((_, _, at))) => (at + Vec3::new(v[0], v[1], v[2]), at),
        _ => (eye, dest),
    };
    let (eye, dest) = match cam_on("MW_CAM_PL") {
        Some(v) => {
            let pl = &play.fight.player;
            let head = Vec3::from_array(pl.pos) + Vec3::Y * 1.45;
            (head + Quat::from_rotation_y(pl.loco.facing) * Vec3::new(v[0], v[1], v[2]), head)
        }
        None => (eye, dest),
    };
    let (eye, dest) = match std::env::var("MW_CAM_RIN").ok().map(|v| v.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>()) {
        Some(v) if v.len() == 3 => {
            let rin = &play.fight.rin;
            let head = Vec3::from_array(rin.pos) + Vec3::Y * 1.62;
            let turn = Quat::from_rotation_y(rin.facing);
            (head + turn * Vec3::new(v[0], v[1], v[2]), head)
        }
        _ => (eye, dest),
    };
    for (mut t,mut projection) in &mut cam { *t = Transform::from_translation(eye).looking_at(dest, Vec3::Y);if let Projection::Perspective(perspective)=&mut *projection{perspective.fov=50f32.to_radians();} }
}

fn hud(play: Res<Play>, mut text: Query<&mut Text,With<FightHud>>,intro:Res<intro::Intro>,pause:Res<pause::Pause>) {
    let notice = play.cam_notice.as_ref().filter(|(_, until)| play.tick < *until).map(|(m, _)| m.clone());
    if intro.controls_blocked()||pause.open{
        // the skip hint is native art now (play_continue.rs)
        for mut text in &mut text{if !text.0.is_empty(){text.0.clear();}}return;
    }
    let pl = &play.fight.player; let rin = &play.fight.rin;
    let status = if play.fight.victory { if play.boss.is_some() || play.fight.solo { String::new() } else { "VICTORY - RinRin defeated".to_string() } }
        else if pl.state() == 95 { String::new() }   // the native continue window (play_continue.rs)
        else if rin.state == mw_sim::fight::rin_state::FINISH_WAIT { if play.as_kreely { "RinRin is down: get close and press Q (or RMB) to start the finisher".to_string() } else { "RinRin is down: get close and swing (RMB / Q) to start the finisher".to_string() } }
        else if rin.state == mw_sim::fight::rin_state::FINISH { format!("FINISHER: hold X to shake  meter {:.0}%", play.fight.meter.1 * 100.) }
        else if rin.down { "RinRin is down: hold E / LMB next to her to grab and giant-swing (X shakes the spin)".to_string() }
        // (the clash shows the native swing navigation: play_prompts.rs)
        else { String::new() };
    let status = match notice { Some(n) if status.is_empty() => n, Some(n) => format!("{status}\n{n}"), None => status };
    let debug=std::env::var("MW_DEBUG_HUD").as_deref()==Ok("1");
    if !debug{for mut t in &mut text{if t.0!=status{t.0=status.clone();}}return;}
    let label = format!(
        "MadWorld native (pure Rust)  |  {} HP {}/100  state {} {}  |  RinRin HP {}/1600  state {}  | tick {}\n\
         WASD run  Ctrl walk  double-tap dash  Space jump  Shift dodge  E/LMB punch string  RMB: {}  X shake  Tab tap recenter / hold lock-on  F4 free camera  F5 lock orbit  Enter continue  Insert swap  C color  Esc pause\n\
         {}  {}",
        if play.as_kreely { "Kreese Kreely" } else { "Jack" }, pl.hp, pl.state(),
        play.gun.as_ref().map(|g| format!("{:?}", g.action)).unwrap_or_default(), rin.hp, rin_name(rin.state), play.tick,
        if play.as_kreely { "hold to aim pistol, LMB fires" } else { "chainsaw (Q horizontal)" },
        status, play.last_events.join(" "));
    for mut t in &mut text { if t.as_str() != label { **t = label.clone(); } }
}

fn rin_name(s: u8) -> &'static str {
    ["wait", "run", "atk_uraken", "atk_tetuzankou", "scaffold_jump", "right_jump", "left_jump", "front_jump", "atk_kamikaze", "right_step",
     "left_step", "back_step", "front_step", "atk_bomb", "atk_throw", "bad_smell", "dm_explosion", "keen_compe", "dm_scaffold_bomb",
     "dm_giantswing", "dm_electricshock", "dm_right", "dm_left", "dm_body", "provocation", "scaffold_wait", "finish", "finish_wait",
     "atk_giantslalom", "guard", "dm_down_damage", "pl_dead"].get(s as usize).copied().unwrap_or("?")
}

/// MW_PLAY_CAPTURE=<png> (single shot after binding) or MW_PLAY_SHOTS="tick:png;tick:png" (game ticks).
fn capture(mut frames_seen: Local<u32>, mut rendered: Local<u64>, mut taken: Local<Vec<u64>>, play: Res<Play>, chars: Query<&Character>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    if chars.iter().any(|c| c.joints.is_none()) { return; }
    let shot = |c: &mut Commands, file: String| { c.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(file)); };
    *rendered += 1;
    let t_frame = |t: &str| t.starts_with('f').then_some(*rendered);
    if let Some(list) = std::env::var_os("MW_PLAY_SHOTS") {
        for part in list.to_string_lossy().split(';') {
            let Some((t, file)) = part.split_once(':') else { continue; };
            // ("fN:png" counts rendered frames instead of game ticks: shots while an event plays and the ticks stand still)
            let now = if let Some(f) = t_frame(t) { f } else { play.tick };
            let Ok(t) = t.trim_start_matches('f').parse::<u64>() else { continue; };
            if now >= t && !taken.contains(&t) { taken.push(t); shot(&mut commands, file.replace('|', ":")); }
        }
        // the run quits 20 frames after its last shot (MW_SHOTS_EXIT=0 keeps it going)
        let want = list.to_string_lossy().split(';').filter(|p| p.contains(':')).count();
        if want > 0 && taken.len() >= want && std::env::var("MW_SHOTS_EXIT").as_deref() != Ok("0") {
            *frames_seen += 1; if *frames_seen > 20 { exit.write(AppExit::Success); }
        }
    }
    // inspection: MW_PLAY_STATE_SHOTS=<player state>:<path prefix> saves a frame every 15 ticks while the player is in
    // that state (at most 12), e.g. 78 for a clash
    if let Some((st, prefix)) = std::env::var("MW_PLAY_STATE_SHOTS").ok().and_then(|v| v.split_once(':').map(|(a, b)| (a.parse::<u8>().ok(), b.to_string()))) {
        if st == Some(play.fight.player.state()) && play.tick % 15 == 0 && !taken.contains(&play.tick) && taken.len() < 12 {
            taken.push(play.tick); shot(&mut commands, format!("{prefix}{:05}.png", play.tick));
        }
    }
    // inspection: MW_PLAY_FRAMES=<first frame>,<count>,<path prefix> saves consecutive rendered frames
    if let Some(spec) = std::env::var("MW_PLAY_FRAMES").ok() {
        let mut it = spec.splitn(3, ',');
        if let (Some(Ok(first)), Some(Ok(count)), Some(prefix)) = (it.next().map(str::parse::<u32>), it.next().map(str::parse::<u32>), it.next()) {
            *frames_seen += 1;
            let k = *frames_seen;
            if (first..first + count).contains(&k) { shot(&mut commands, format!("{prefix}{k:04}.png")); }
            if k == first + count + 30 { exit.write(AppExit::Success); }
            return;
        }
    }
    let Some(file) = std::env::var_os("MW_PLAY_CAPTURE") else { return; };
    *frames_seen += 1;
    if *frames_seen == 90 { shot(&mut commands, file.into_string().unwrap()); }
    if *frames_seen == 150 { exit.write(AppExit::Success); }
}

/// Bounded real-window smoke run; no keyboard automation or original runtime dependency.
fn pause_smoke(play:Res<Play>,pause:Res<pause::Pause>,time:Res<Time<Virtual>>,sound:Res<audio::PortAudio>,sinks:Query<(&audio::Gain,&AudioSink)>,
    mut saved:Local<Option<(u64,[f32;3],i16,f64)>>,mut commands:Commands,mut exit:MessageWriter<AppExit>){
    if !pause.smoke{return;}
    if pause.frame==91 {
        *saved=Some((play.tick,play.fight.player.pos,play.fight.player.hp,time.elapsed_secs_f64()));
        eprintln!("[pause-smoke] frozen tick {}",play.tick);
    }
    if pause.frame==150 {
        if let Some(path)=std::env::var_os("MW_PAUSE_CAPTURE") {
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                .observe(bevy::render::view::screenshot::save_to_disk(std::path::PathBuf::from(path)));
        }
    }
    if pause.frame==160&&std::env::var("MW_PAUSE_SMOKE_PAGE").as_deref()==Ok("playlog"){
        if let Some(path)=std::env::var_os("MW_PAUSE_CAPTURE_LAST"){
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(std::path::PathBuf::from(path)));
        }
    }
    if pause.frame==260{
        if let Some(path)=std::env::var_os("MW_PAUSE_CAPTURE_TITLE"){
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(std::path::PathBuf::from(path)));
        }
    }
    if pause.frame==200 {
        if let Some(path)=std::env::var_os("MW_PAUSE_CAPTURE_DIALOG"){
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(std::path::PathBuf::from(path)));
        }
    }
    if pause.frame==155&&std::env::var("MW_PAUSE_SMOKE_PAGE").as_deref()==Ok("sound"){
        use bevy::audio::AudioSinkPlayback;
        let mut count=0;
        for (gain,sink)in &sinks{if matches!(gain.bus,audio::Bus::Music){
            let expected=sound.gain(audio::Bus::Music,gain.base);assert!((sink.volume().to_linear()-expected).abs()<0.0001,"BGM gain did not follow Sound settings");
            assert!(sink.is_paused(),"BGM resumed inside Sound menu");count+=1;
            eprintln!("[pause-smoke] paused BGM gain {}",sink.volume().to_linear());
        }}
        assert!(count>0,"No BGM sink available to validate Sound preview");
    }
    if pause.frame==179 {
        assert_eq!(*saved,Some((play.tick,play.fight.player.pos,play.fight.player.hp,time.elapsed_secs_f64())),"fight or virtual clock advanced while paused");
        assert!(time.is_paused());eprintln!("[pause-smoke] held tick {}, HP {}, position {:?}, virtual time {}",play.tick,play.fight.player.hp,play.fight.player.pos,time.elapsed_secs_f64());
    }
    if std::env::var("MW_PAUSE_SMOKE_PAGE").as_deref()==Ok("title-save"){
        if matches!(pause.frame,365|450|550|695|745){if let Some(dir)=std::env::var_os("MW_SAVE_SMOKE_CAPTURES"){let path=std::path::PathBuf::from(dir).join(format!("save-{}.png",pause.frame));commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(path));}}
        if pause.frame==780{assert!(pause.open&&time.is_paused());assert_eq!(*saved,Some((play.tick,play.fight.player.pos,play.fight.player.hp,time.elapsed_secs_f64())));eprintln!("[save-smoke] title copy/cancel/erase completed with fight frozen");exit.write(AppExit::Success);}
        return;
    }
    if std::env::var("MW_PAUSE_SMOKE_PAGE").as_deref()==Ok("title-slots"){
        if pause.frame==315||pause.frame==365{
            let name=if pause.frame==315{"MW_PAUSE_CAPTURE_SLOTS"}else{"MW_PAUSE_CAPTURE_ACTIONS"};
            if let Some(path)=std::env::var_os(name){commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(std::path::PathBuf::from(path)));}
        }
        if pause.frame==500{
            assert_eq!(pause.menu.page,mw_sim::ui::PausePage::SaveSlots);assert_eq!(pause.slots.as_ref().unwrap().selected,2);assert!(pause.open&&time.is_paused());assert_eq!(*saved,Some((play.tick,play.fight.player.pos,play.fight.player.hp,time.elapsed_secs_f64())));assert_eq!(sinks.iter().count(),0);
            eprintln!("[pause-smoke] save slots clamp/select/actions/back; selected slot3; fight frozen");exit.write(AppExit::Success);
        }
        return;
    }
    if pause.frame==300&&std::env::var("MW_PAUSE_SMOKE_PAGE").as_deref()==Ok("title-return"){
        assert_eq!(pause.menu.page,mw_sim::ui::PausePage::TitleSplash);assert!(pause.open&&time.is_paused());
        assert_eq!(*saved,Some((play.tick,play.fight.player.pos,play.fight.player.hp,time.elapsed_secs_f64())));assert_eq!(sinks.iter().count(),0,"Fight voices retained at title");
        eprintln!("[pause-smoke] returned to Title splash; fight voices removed");exit.write(AppExit::Success);return;
    }
    if pause.frame==300 {
        assert!(play.tick>saved.unwrap().0,"fight failed to resume");assert!(!time.is_paused());
        eprintln!("[pause-smoke] resumed tick {}",play.tick);exit.write(AppExit::Success);
    }
}

/// E00 return removes fight audio/queues; paused sinks must never restart at title.
fn title_audio(pause:Res<pause::Pause>,mut commands:Commands,voices:Query<Entity,With<audio::Gain>>,mut sounds:ResMut<Sounds>,mut announcer:ResMut<Announcer>,mut bgm:ResMut<Bgm>){
    if pause.menu.page!=mw_sim::ui::PausePage::TitleSplash{return;}
    for entity in &voices{commands.entity(entity).despawn();}
    sounds.pending.clear();announcer.queue.clear();bgm.stopped=true;
}

/// [added] The base plugins, or OpenXR's in their place for VR (play_vr.rs).
trait GamePlugins { fn add_game_plugins(&mut self, base: bevy::app::PluginGroupBuilder) -> &mut Self; }
impl GamePlugins for App { fn add_game_plugins(&mut self, base: bevy::app::PluginGroupBuilder) -> &mut Self { vr::plugins(self, base); self } }
