//! What the hands do in the fight [added] (FixedUpdate, the 30 Hz game frame, after everything else in it):
//! - the body follows the headset (sliding along the collision; walking into a wall pushes the view back) and the
//!   tracking space moves with the body's own movement (the stick, a lunge, a knock-back);
//! - fists strike by their speed: grunts and townsfolk (Street::vr_strike), RinRin (her own hurt), Elise and the
//!   Baron / Mathilda (their hurt); a held grunt takes the blow in the hold (the native head-butt reel);
//! - a grip closing: the holster (the pistol), a street object (Props::vr_grab), a grunt or a townsperson
//!   (Street::vr_grab: lifted by the collar), RinRin / a boss (Kreely's grab), a ledge or a wall (climbing);
//!   opening it: the pistol goes back to the holster, what was held is thrown with the hand's velocity (or dropped);
//! - climbing: an anchored hand stays where it gripped and the body moves instead; letting go over a ledge stands on
//!   it (the vault), pushing the stick forward while hanging from a ledge pulls up over it.
use super::{Rig, VrInput, VrSettings};
use bevy::prelude::*;

/// Fist speeds (m/s) for the lightest and a full-power blow; a heavy blow knocks down.
const PUNCH_MIN: f32 = 1.9;
const PUNCH_FULL: f32 = 6.0;
const PUNCH_HEAVY: f32 = 5.2;
const PUNCH_COOL: f32 = 0.28;
/// Reach of a grab (m from the hand to a body's surface) and a throw's least speed and boost.
const GRAB_REACH: f32 = 0.3;
const THROW_MIN: f32 = 2.2;
const THROW_BOOST: f32 = 1.35;
const THROW_MAX: f32 = 11.;
/// The holster's grab radius.
pub const HOLSTER_REACH: f32 = 0.24;
/// Pull-up speed (m/s) when the stick is pushed while hanging from a ledge.
const PULL_UP: f32 = 1.6;
const DT: f32 = 1. / 30.;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Hold {
    #[default] None,
    /// a closed fist on nothing
    Fist,
    Gun, Prop(usize), Grunt(u32, Vec3), Boss,
    /// a magazine (its rounds); the pistol's slide (the gun in the other hand); a support hand round its grip
    Mag(u8), Slide, Support,
    /// RinRin held up (play_vr_interact: carried, thrown, or lifted overhead into the suplex)
    Rin,
    /// climbing: where the hand gripped (world), and the ledge top if it is one
    Climb(Vec3, Option<f32>),
}

#[derive(Resource, Default)]
pub struct Interact {
    pub gun_drawn: bool, pub gun_was_drawn: bool,
    /// the hand holding the pistol (when drawn)
    pub gun_hand: usize,
    pub shake_dir: [Vec3; 2], pub shake_at: [f32; 2], pub swing_at: [f32; 2],
    pub hold: [Hold; 2], closed: [bool; 2], toggle: [bool; 2],
    cool: [f32; 2], hand_prev: [Option<Vec3>; 2], vel_hist: [[Vec3; 3]; 2],
    /// the body was hanging (climbing) last frame
    pub hanging: bool,
    /// the stick's pull-up so far (m): the anchored hands are taken as that much lower
    pull: f32,
    /// a ledge vaulted onto (its top): the body stays on it unless it steps off
    perch: Option<f32>,
    /// [stats for tests] blows landed, throws, grabs, climbs
    pub stats: [u32; 4],
}
impl Interact {
    pub fn holding(&self, h: usize) -> bool { !matches!(self.hold[h], Hold::None) }
    pub fn climbing(&self) -> bool { self.hold.iter().any(|h| matches!(h, Hold::Climb(..))) }
}

/// FixedUpdate, before the fight tick: where the body starts the step.
pub fn before_tick(play: Res<super::super::Play>, mut rig: ResMut<Rig>) { rig.body_before = Vec3::from_array(play.fight.player.pos); }

#[allow(clippy::too_many_arguments)]
pub fn step(mut play: ResMut<super::super::Play>, mut rig: ResMut<Rig>, mut inp: ResMut<VrInput>, set: Res<VrSettings>, mut act: ResMut<Interact>,
            mut street: ResMut<super::super::street::Street>, mut props: ResMut<super::super::props::Props>, mut elise: ResMut<super::super::elise::Elise>,
            mut baron: ResMut<super::super::baron::Baron>, mut fx: ResMut<super::super::fx::Fx>, arena: Res<super::super::Arena>,
            mut snd: ResMut<super::super::Sounds>, view: Res<super::View>, mut pistol: ResMut<super::gun::Pistol>) {
    if !rig.started { return; }
    let p = &mut *play;
    let log = std::env::var_os("MW_VR_LOG").is_some();
    // the space moves with the body's own movement this step
    let body0 = Vec3::from_array(p.fight.player.pos);
    let loco = body0 - rig.body_before;
    rig.prev = rig.cur;
    let climbing = act.climbing();
    // (perched on a vaulted ledge the space follows the body once its place is settled, below)
    if !climbing && act.perch.is_none() { rig.cur.x += loco.x; rig.cur.z += loco.z; rig.cur.y = body0.y; }
    let root = rig.cur;
    let head = rig.world_at(root, &inp.head);
    let fit = Transform { translation: Vec3::new(0., set.hand_up, -set.hand_fwd), rotation: Quat::from_rotation_x(set.hand_tilt.to_radians()), scale: Vec3::ONE };
    let hands: [Transform; 2] = [rig.world_at(root, &inp.hands[0].hand_pose(&fit)), rig.world_at(root, &inp.hands[1].hand_pose(&fit))];
    let aims: [Transform; 2] = [rig.world_at(root, &(inp.hands[0].aim_pose * fit)), rig.world_at(root, &(inp.hands[1].aim_pose * fit))];
    // hand velocities: the tracked velocity (turned into the world) plus the body's own movement
    let body_vel = loco / DT;
    let mut vel = [Vec3::ZERO; 2];
    for h in 0..2 {
        // the fastest the hand moved since the last game frame (render frames come faster than 30 Hz)
        let peak = if rig.peak[h].length() > inp.hands[h].vel.length() { rig.peak[h] } else { inp.hands[h].vel };
        rig.peak[h] = Vec3::ZERO;
        vel[h] = rig.rot() * peak;
        act.vel_hist[h] = [act.vel_hist[h][1], act.vel_hist[h][2], vel[h] + body_vel];
        act.hand_prev[h] = Some(hands[h].translation);
        act.cool[h] = (act.cool[h] - DT).max(0.);
    }
    let st = p.fight.player.state();
    let free = matches!(st, 0 | 1 | 2 | 4 | 0xC8 | 12 | 13 | 15) && !p.fight.paired;

    // climbing: the anchored hands hold still in the world; the body (and the space) move instead
    if climbing && !view.cinema {
        let mut d = Vec3::ZERO; let mut n = 0f32;
        for h in 0..2 { if let Hold::Climb(at, _) = act.hold[h] { d += hands[h].translation - at; n += 1.; } }
        let mut d = d / n.max(1.);
        // pulling up over a ledge with the stick: the hands count as lower and lower
        let ledge = act.hold.iter().find_map(|h| if let Hold::Climb(_, Some(top)) = h { Some(*top) } else { None });
        if let Some(top) = ledge {
            if inp.hands[0].stick.y > 0.6 && rig.cur.y < top + 0.1 { act.pull += PULL_UP * DT; }
        }
        d.y -= act.pull;
        let d = d.clamp_length_max(0.35);
        rig.cur -= d;
        // never below the floor under the body
        let pos = Vec3::from_array(p.fight.player.pos);
        if let Some(f) = super::super::floor(&arena.0, Vec3::new(pos.x, rig.cur.y + 0.3, pos.z)) { if rig.cur.y < f { rig.cur.y = f; } }
        let head = rig.world_at(rig.cur, &inp.head);
        let step = Vec2::new(head.translation.x - pos.x, head.translation.z - pos.z).clamp_length_max(0.35);
        let (clear, _) = mw_sim::collision::slide(&arena.0, [pos.x, rig.cur.y + 0.4, pos.z], [step.x, step.y]);
        let to = [pos.x + clear[0], rig.cur.y, pos.z + clear[1]];
        let left = step - Vec2::from_array(clear);
        if left.length() > 0.01 { rig.cur.x -= left.x; rig.cur.z -= left.y; }
        p.fight.player.vr_hang(to);
        act.hanging = true;
        // over a ledge with the hips: up onto it
        if let Some(top) = ledge {
            if rig.cur.y >= top - 0.05 {
                let fwd = Vec3::new(head.translation.x - pos.x, 0., head.translation.z - pos.z).normalize_or(Vec3::new(p.fight.player.loco.facing.sin(), 0., p.fight.player.loco.facing.cos()));
                // the first footing on the ledge's top ahead of him (up to a metre: over its edge)
                let onto = [0.3f32, 0.45, 0.6, 0.75, 0.9, 1.05].iter().map(|k| Vec3::new(to[0], top, to[2]) + fwd * *k)
                    .find(|o| [0f32, 0.2, 0.4].iter().all(|k| top_near(&arena.0, *o + fwd * *k, 0.5, 1.0).is_some_and(|f| (f - top).abs() < 0.25)))
                    // (firm footing: well past the edge, so the body is not pushed back off it)
                    .map(|o| o + fwd * 0.15);
                if let Some(onto) = onto {
                    let off = onto - Vec3::from_array(to);
                    let dy = top - rig.cur.y; rig.cur += Vec3::new(off.x, dy, off.z);
                    p.fight.player.vr_hang(onto.to_array());
                    for h in 0..2 { if matches!(act.hold[h], Hold::Climb(..)) { act.hold[h] = Hold::Fist; } }
                    act.pull = 0.;
                    act.perch = Some(top);
                    act.stats[3] += 1;
                    if log { eprintln!("[vr] tick {} vaulted onto the ledge at {:.2}", p.tick, top); }
                }
            }
        }
    } else if act.hanging {
        act.pull = 0.;
        // let go: standing if a floor is right under him, else he falls
        act.hanging = false;
        let pos = Vec3::from_array(p.fight.player.pos);
        match super::super::floor(&arena.0, pos + Vec3::Y * 0.4) {
            Some(f) if pos.y - f < 0.3 => { p.fight.player.vr_hang([pos.x, f, pos.z]); rig.cur.y = f; }
            _ => { p.fight.player.vr_drop(); if log { eprintln!("[vr] tick {} let go: falling", p.tick); } }
        }
    } else if let Some(top) = act.perch.filter(|_| !view.cinema) {
        // standing on a vaulted ledge: the stage's own contacts may shove the body off one the native game never stands
        // on (the street's counters); back on top unless the stick walked him off the edge
        let pos = Vec3::from_array(p.fight.player.pos);
        let on = |q: Vec3| top_near(&arena.0, Vec3::new(q.x, top, q.z), 0.5, 1.0).is_some_and(|f| (f - top).abs() < 0.25);
        // where the stick sends him (the pad's camera-relative stick)
        let stick = p.latched.stick;
        let a = p.cam_yaw + (-stick.x).atan2(stick.y);
        let intent = if stick.length() > 0.3 { Vec3::new(a.sin(), 0., a.cos()) } else { Vec3::ZERO };
        let moved = Vec3::new(pos.x - rig.body_before.x, 0., pos.z - rig.body_before.z);
        let _ = moved;
        // the stick walks him along the top (the native contacts would not): over the edge, he steps down
        let walk = rig.body_before + intent * stick.length().min(1.) * 2.6 / 30.;
        if intent != Vec3::ZERO && on(walk) { p.fight.player.vr_hang([walk.x, top, walk.z]); }
        else if intent != Vec3::ZERO { act.perch = None; p.fight.player.vr_hang([walk.x, top, walk.z]); p.fight.player.vr_drop(); if log { eprintln!("[vr] tick {} stepped off the ledge", p.tick); } }
        else if on(rig.body_before) { let b = rig.body_before; p.fight.player.vr_hang([b.x, top, b.z]); }
        else { act.perch = None; }
        // the space moves with where he ended up
        let fin = Vec3::from_array(p.fight.player.pos);
        rig.cur.x += fin.x - rig.body_before.x; rig.cur.z += fin.z - rig.body_before.z;
        rig.cur.y = if act.perch.is_some() { top } else { fin.y };
        // the body follows the head on top too
        let pos = Vec3::from_array(p.fight.player.pos);
        let want = Vec2::new(head.translation.x - pos.x, head.translation.z - pos.z).clamp_length_max(0.4);
        if want.length() > 0.015 && on(pos + Vec3::new(want.x, 0., want.y)) { p.fight.player.pos[0] += want.x; p.fight.player.pos[2] += want.y; }
    } else if free && !view.cinema && !p.fight.player.airborne {
        // the body follows the headset around the room
        let pos = Vec3::from_array(p.fight.player.pos);
        let want = Vec2::new(head.translation.x - pos.x, head.translation.z - pos.z);
        if want.length() > 0.015 {
            let step = want.clamp_length_max(0.4);
            let (clear, _) = mw_sim::collision::slide(&arena.0, p.fight.player.pos, [step.x, step.y]);
            p.fight.player.pos[0] += clear[0]; p.fight.player.pos[2] += clear[1];
            let left = step - Vec2::from_array(clear);
            if set.wall_push && left.length() > 0.01 { rig.cur.x -= left.x; rig.cur.z -= left.y; }
        }
        // standing still he faces where you look
        if matches!(st, 0) { p.fight.player.loco.facing = super::game_yaw(head.rotation); }
    }

    // inspection: MW_VR_TEST=grunt stands the nearest grunt 0.95 m before the view every 150 game frames from 100
    // (MW_VR_TEST=grunt:<metres>:<every n frames>)
    let test = std::env::var("MW_VR_TEST").unwrap_or_default();
    let mut it = test.split(':');
    let (dist, every) = (it.nth(1).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.95), it.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(150));
    // (MW_VR_TEST=pair:<metres>:<every>: a second grunt 2.5 m behind the first, to be hit by a throw)
    if test.starts_with("pair") && p.tick >= 100 && (p.tick - 100) % every == 0 {
        let yaw = super::game_yaw(head.rotation);
        let fwd = Vec3::new(yaw.sin(), 0., yaw.cos());
        let base = Vec3::from_array(p.fight.player.pos);
        let a = street.vr_test_place(base + fwd * dist, yaw + std::f32::consts::PI);
        let gap = test.split(':').nth(3).and_then(|v| v.parse::<f32>().ok()).unwrap_or(2.5);
        let b = street.vr_test_place_except(base + fwd * (dist + gap), yaw + std::f32::consts::PI, &a.into_iter().collect::<Vec<_>>());
        eprintln!("[vr] test: grunts {a:?} at {dist} m and {b:?} behind him");
    }
    // (MW_VR_TEST=rin:<metres>:<every>: RinRin set before the view, waiting, every n frames from 100)
    if test.starts_with("rin") && p.tick >= 100 && (p.tick - 100) % every == 0 && p.fight.rin.state != mw_sim::fight::RIN_VR_HELD {
        let yaw = super::game_yaw(head.rotation);
        let at = Vec3::from_array(p.fight.player.pos) + Vec3::new(yaw.sin(), 0., yaw.cos()) * dist;
        p.fight.rin_reset(at.to_array(), yaw + std::f32::consts::PI);
        eprintln!("[vr] test: RinRin set at {at:.2}");
    }
    if test.starts_with("civ") && p.tick >= 100 && (p.tick - 100) % every == 0 {
        let yaw = super::game_yaw(head.rotation);
        let at = Vec3::from_array(p.fight.player.pos) + Vec3::new(yaw.sin(), 0., yaw.cos()) * dist;
        let id = street.vr_test_place_civ(at, yaw + std::f32::consts::PI);
        eprintln!("[vr] test: townsperson {id:?} placed at {at:.2}");
    }
    if test.starts_with("grunt") && p.tick >= 100 && (p.tick - 100) % every == 0 {
        let yaw = super::game_yaw(head.rotation);
        let at = Vec3::from_array(p.fight.player.pos) + Vec3::new(yaw.sin(), 0., yaw.cos()) * dist;
        let id = street.vr_test_place(at, yaw + std::f32::consts::PI);
        eprintln!("[vr] test: grunt {id:?} placed at {at:.2}");
    }
    // inspection: MW_VR_TEST=place:<x>:<z>:<yaw deg> stands the player there at game frame 60
    if test.starts_with("place") && p.tick == 60 {
        let v: Vec<f32> = test.split(':').skip(1).filter_map(|x| x.parse().ok()).collect();
        if v.len() >= 3 {
            let pl = &mut p.fight.player;
            let from = Vec3::from_array(pl.pos);
            pl.pos = [v[0], pl.pos[1], v[1]]; pl.loco.facing = v[2].to_radians();
            let d = Vec3::from_array(pl.pos) - from;
            // (the space turns with him, keeping the head over the body)
            rig.cur += Vec3::new(d.x, 0., d.z); rig.prev = rig.cur;
            let turn = v[2].to_radians() - super::game_yaw(head.rotation);
            rig.head.translation = rig.world_at(rig.cur, &inp.head).translation;
            rig.turn(turn); rig.prev = rig.cur;
            eprintln!("[vr] test: placed at {:.2} facing {:.0} deg", Vec3::from_array(p.fight.player.pos), v[2]);
        }
    }
    // inspection: MW_VR_PROBE=1 lists standable tops 0.8-1.9 m high within 12 m (at game frame 30)
    if p.tick == 30 && std::env::var_os("MW_VR_PROBE").is_some() {
        let c = Vec3::from_array(p.fight.player.pos);
        let mut found = vec![];
        for i in -24..=24 { for k in -24..=24 {
            let q = c + Vec3::new(i as f32 * 0.5, 0., k as f32 * 0.5);
            if let Some(t) = top_near(&arena.0, Vec3::new(q.x, c.y + 2.0, q.z), 0.1, 2.2) { if t > c.y + 0.8 && t < c.y + 1.9 { found.push(format!("({:.1},{:.1})={:.2}", q.x, q.z, t)); } }
        } }
        eprintln!("[vr] probe: {}", found.join(" "));
    }
    if log && (p.tick % 30 == 0 || std::env::var_os("MW_VR_TRACE").is_some() && p.tick % 3 == 0) {
        eprintln!("[vr] tick {} yaw {:.0} body {:.2} head {:.2} hands {:.2} / {:.2} hold {:?} stats {:?} rin {:.2} hp {} solo {}", p.tick, rig.yaw.to_degrees(), Vec3::from_array(p.fight.player.pos), head.translation, hands[0].translation, hands[1].translation, act.hold, act.stats, Vec3::from_array(p.fight.rin.pos), p.fight.rin.hp, p.fight.solo);
    }
    if log && p.fight.suplex.is_some() && p.tick % 10 == 0 { eprintln!("[vr] tick {} suplex under way (Kreely state {})", p.tick, p.fight.player.state()); }
    // the grips
    let g = set.main_hand();
    let body = Vec3::from_array(p.fight.player.pos);
    let body_yaw = super::game_yaw(head.rotation);
    let tick = p.tick;
    let manual = set.gun_manual && p.as_kreely;
    let holster_at = super::hands::holster(body, body_yaw, g);
    let pouch_at = super::hands::holster(body, body_yaw, 1 - g);
    if !manual && matches!(pistol.at, super::gun::GunAt::Loose { .. }) { pistol.at = super::gun::GunAt::Holster; }
    super::gun::update_root(&mut pistol, &hands, &aims, body, body_yaw, g, p.ammo.recoil);
    for h in 0..2 {
        let grip = inp.hands[h].grip;
        let raw = if act.closed[h] { grip > 0.35 } else { grip > 0.6 };
        let closed = if set.grab_toggle {
            if raw && !act.toggle[h] { act.toggle[h] = true; !act.closed[h] } else { if !raw { act.toggle[h] = false; } act.closed[h] }
        } else { raw };
        let was = act.closed[h];
        act.closed[h] = closed;
        let at = hands[h].translation;
        if closed && !was && !view.cinema {
            // what is in reach, nearest kind first
            let holster = holster_at;
            let picked = if manual { super::gun::grab(&mut pistol, &mut act, h, at, holster_at, pouch_at, &mut p.ammo.reserve, log, tick) } else { None };
            act.hold[h] = if let Some(x) = picked {
                inp.rumble(h, 0.35, 0.05);
                x
            } else if !manual && h == g && p.as_kreely && at.distance(holster) < HOLSTER_REACH && !act.gun_drawn {
                act.gun_drawn = true; act.gun_hand = h; pistol.at = super::gun::GunAt::Hand(h); inp.rumble(h, 0.35, 0.05);
                if log { eprintln!("[vr] tick {tick} drew the pistol"); }
                Hold::Gun
            } else if let Some(i) = props.vr_grab(&hands[h]) {
                act.stats[2] += 1; inp.rumble(h, 0.4, 0.05);
                if log { eprintln!("[vr] tick {tick} hand {h} picked up object {i}"); }
                Hold::Prop(i)
            } else if let Some((id, off)) = street.vr_grab(at, GRAB_REACH, body) {
                act.stats[2] += 1; inp.rumble(h, 0.6, 0.08);
                if log { eprintln!("[vr] tick {tick} hand {h} grabbed grunt {id}"); }
                Hold::Grunt(id, off)
            } else if near_rin(p, at) && (act.hold[1 - h] == Hold::Rin || { let mask = [h == 0 || act.hold[0] == Hold::Rin, h == 1 || act.hold[1] == Hold::Rin]; let at = rin_hold_at(&hands, mask, h, body); let f = rin_facing(p, body); p.fight.vr_hold_rin(&p.fd, at.to_array(), f) }) {
                // RinRin: held up in his hands (carry her, throw her; both hands over the head: his suplex)
                inp.rumble(h, 0.6, 0.08);
                if log { eprintln!("[vr] tick {tick} hand {h} holds RinRin"); }
                Hold::Rin
            } else if near_boss(p, &elise, &baron, at) {
                p.latched.grab = true; inp.rumble(h, 0.5, 0.06);
                if log { eprintln!("[vr] tick {tick} hand {h} grabs the boss (Kreely's grab)"); }
                Hold::Boss
            } else if let Some(k) = climb_hold(&arena.0, &hands[h], body).filter(|_| set.climb) {
                act.stats[3] += 1; inp.rumble(h, 0.3, 0.04);
                if log { eprintln!("[vr] tick {tick} hand {h} climbs ({})", if k.1.is_some() { "ledge" } else { "wall" }); }
                Hold::Climb(k.0, k.1)
            } else { Hold::Fist };
        }
        if !closed && was {
            let v = (act.vel_hist[h][0] + act.vel_hist[h][1] + act.vel_hist[h][2]) / 3.;
            let v = if v.length() > THROW_MIN { (v * THROW_BOOST).clamp_length_max(THROW_MAX) } else { v };
            match act.hold[h] {
                Hold::Gun => {
                    act.gun_drawn = false;
                    if manual { super::gun::release(&mut pistol, Hold::Gun, h, &hands[h], v, rig.rot() * inp.hands[h].ang_vel, holster_at, pouch_at, &mut p.ammo.reserve, log, tick); }
                    else { pistol.at = super::gun::GunAt::Holster; }
                }
                Hold::Rin if act.hold[1 - h] != Hold::Rin => {
                    let ms = v.length();
                    let damage = if ms > THROW_MIN { (ms * 6.).clamp(10., 70.) as i32 } else { 0 };
                    let per_tick = (v * DT).to_array();
                    p.fight.vr_throw_rin(&p.fd, per_tick, damage);
                    if ms > THROW_MIN { act.stats[1] += 1; }
                    if log { eprintln!("[vr] tick {tick} let go of RinRin at {ms:.1} m/s ({damage} damage)"); }
                }
                Hold::Mag(_) => { super::gun::release(&mut pistol, act.hold[h], h, &hands[h], v, rig.rot() * inp.hands[h].ang_vel, holster_at, pouch_at, &mut p.ammo.reserve, log, tick); }
                Hold::Prop(i) => { props.vr_release(i, v, rig.rot() * inp.hands[h].ang_vel); if v.length() > THROW_MIN { act.stats[1] += 1; } if log { eprintln!("[vr] tick {tick} released object {i} at {:.1} m/s", v.length()); } }
                Hold::Grunt(id, _) => { street.vr_release(id, v, &mut fx); if v.length() > THROW_MIN { act.stats[1] += 1; } if log { eprintln!("[vr] tick {tick} released grunt {id} at {:.1} m/s", v.length()); } }
                _ => {}
            }
            act.hold[h] = Hold::None;
            act.cool[h] = act.cool[h].max(0.2);
        }
        // keep holding
        match act.hold[h] {
            Hold::Prop(i) => { if !props.vr_hold(i, &hands[h]) { act.hold[h] = Hold::Fist; } }
            Hold::Grunt(id, off) => {
                if !street.vr_hold(id, at - off, body) { act.hold[h] = Hold::Fist; inp.rumble(h, 0.5, 0.1); if log { eprintln!("[vr] tick {tick} grunt {id} got away"); } }
            }
            Hold::Gun if !act.gun_drawn => act.hold[h] = Hold::Fist,
            Hold::Mag(n) => {
                if super::gun::try_seat(&pistol, h, &hands[h]) {
                    super::gun::seat(&mut pistol, n);
                    act.hold[h] = Hold::Fist;
                    inp.rumble(h, 0.6, 0.05); inp.rumble(1 - h, 0.6, 0.05);
                    super::super::queue_sound(&mut snd, tick, super::super::GUN_BLOCK, super::super::GUN_SEAT, at.to_array());
                    if log { eprintln!("[vr] tick {tick} magazine seated ({n} rounds)"); }
                }
            }
            Hold::Slide | Hold::Support if pistol.in_hand() != Some(1 - h) => act.hold[h] = Hold::Fist,
            Hold::Rin if act.hold[1 - h] != Hold::Rin || h == 0 => {
                // (once a tick: the first hand that holds her moves her)
                let holding: Vec<usize> = (0..2).filter(|k| act.hold[*k] == Hold::Rin).collect();
                let overhead = holding.len() == 2 && holding.iter().all(|k| hands[*k].translation.y > head.translation.y + 0.05);
                if overhead {
                    // lifted over his head: she is set before him, facing him, and his own grab (the suplex) takes her
                    let fwd = Vec3::new(head.rotation.mul_vec3(Vec3::NEG_Z).x, 0., head.rotation.mul_vec3(Vec3::NEG_Z).z).normalize_or(Vec3::Z);
                    let at = body + fwd * 0.8;
                    p.fight.vr_rin_to_grab(at.to_array(), (-fwd).x.atan2(-fwd.z));
                    p.fight.player.loco.facing = fwd.x.atan2(fwd.z);
                    p.latched.grab = true;
                    for k in &holding { act.hold[*k] = Hold::Fist; inp.rumble(*k, 0.9, 0.12); }
                    if log { eprintln!("[vr] tick {tick} RinRin lifted overhead: the suplex (Kreely state {} combat {} airborne {}; her at {:.2}, him at {:.2})",
                                       p.fight.player.state(), p.fight.player.loco.combat, p.fight.player.airborne, Vec3::from_array(p.fight.rin.pos), body); }
                } else if !{ let mask = [act.hold[0] == Hold::Rin, act.hold[1] == Hold::Rin]; let at = rin_hold_at(&hands, mask, h, body); let f = rin_facing(p, body); p.fight.vr_hold_rin(&p.fd, at.to_array(), f) } {
                    for k in &holding { act.hold[*k] = Hold::Fist; inp.rumble(*k, 0.6, 0.12); }
                    if log { eprintln!("[vr] tick {tick} RinRin broke free"); }
                }
            }
            _ => {}
        }
    }
    if !p.as_kreely || view.cinema && act.gun_drawn && false { act.gun_drawn = false; }
    // the gun hand let go some other way (a knock-down): dropped (realistic) or back in the holster
    if act.gun_drawn && !matches!(act.hold[act.gun_hand], Hold::Gun) {
        act.gun_drawn = false;
        pistol.at = if manual { super::gun::GunAt::Loose { tf: pistol.root, vel: Vec3::ZERO, spin: Vec3::ZERO, rest: false } } else { super::gun::GunAt::Holster };
        if log { eprintln!("[vr] tick {tick} the pistol left the hand"); }
    }
    if !act.gun_drawn && pistol.in_hand().is_some() { pistol.at = super::gun::GunAt::Holster; }
    pistol.support = act.hold.contains(&Hold::Support);
    if manual { super::gun::step(&mut pistol, &act, &hands, p, &mut inp, &mut snd, &arena.0, body, log); }

    // fists
    for h in 0..2 {
        if matches!(act.hold[h], Hold::Gun | Hold::Prop(_) | Hold::Climb(..) | Hold::Grunt(..) | Hold::Mag(_) | Hold::Slide | Hold::Support | Hold::Rin) || act.cool[h] > 0. || view.cinema { continue; }
        let v = vel[h];
        let speed = v.length() * set.punch_power;
        if speed < PUNCH_MIN { continue; }
        let k = ((speed - PUNCH_MIN) / (PUNCH_FULL - PUNCH_MIN)).clamp(0., 1.);
        let closed = act.closed[h] || inp.hands[h].grip > 0.4;
        let power = ((6. + 30. * k) * if closed { 1. } else { 0.6 }) as i32;
        let heavy = speed > PUNCH_HEAVY && closed;
        let dir = v.normalize();
        // the knuckles, a little ahead of the grip along the swing
        let fist = hands[h].translation + aims[h].rotation * Vec3::new(0., 0., -0.06) + dir * 0.03;
        let mut landed: Option<Vec3> = None;
        let held = act.hold.iter().find_map(|x| if let Hold::Grunt(id, _) = x { Some(*id) } else { None });
        if let Some((at, id)) = street.vr_strike(fist, 0.12, dir, power, heavy, held, &mut fx) {
            landed = Some(at);
            if log { eprintln!("[vr] tick {tick} hand {h} punched grunt {id} ({power}{}) at {:.1} m/s", if heavy { " heavy" } else { "" }, speed); }
        } else if !p.fight.solo && p.fight.rin.hp > 0 && in_capsule(Vec3::from_array(p.fight.rin.pos), fist, 0.55, 0.3, 2.3) {
            // RinRin: her own reaction rows, light or heavy (the punch table's light 0x5E / heavy 0x76)
            p.fight.rin_hurt(&p.fd, if heavy { 0x76 } else { 0x5E }, power);
            fx.impact(fist, dir.x.atan2(dir.z), 0.9, [1., 0.12, 0.08]); fx.burst("core", 1005, fist, dir.x.atan2(dir.z), 0.6);
            landed = Some(fist);
            if log { eprintln!("[vr] tick {tick} hand {h} punched RinRin ({power}) hp {}", p.fight.rin.hp); }
        } else if elise.vr_punch(fist, body, power, &mut fx) || baron.vr_punch(fist, body, power, heavy, &mut fx) {
            landed = Some(fist);
            if log { eprintln!("[vr] tick {tick} hand {h} punched the boss ({power})"); }
        }
        if let Some(at) = landed {
            act.cool[h] = PUNCH_COOL; act.stats[0] += 1;
            inp.rumble(h, 0.45 + 0.55 * k, 0.07);
            super::super::queue_sound(&mut snd, tick, 1101, if heavy { 3 } else { (tick % 3) as u32 }, at.to_array());
            if p.as_kreely { p.ammo.reserve = (p.ammo.reserve + 1).min(super::super::RESERVE_MAX); }
        }
    }
    // the pistol's kick in the hand
    for e in &p.gun_events {
        if let super::super::GunEvent::Shot { dir, end, kind, .. } = e {
            inp.rumble(g, 0.9, 0.06);
            if log { eprintln!("[vr] tick {tick} shot #{} along {:.2} -> {} (magazine {} / reserve {})", p.shots_fired.len(), dir, match kind { super::super::ShotEnd::Flesh => format!("flesh at {:.2}", end.unwrap_or_default()), super::super::ShotEnd::Stage => "the stage".into(), super::super::ShotEnd::Launched => "in flight".into(), _ => "the fan".into() }, p.ammo.mag, p.ammo.reserve); }
        }
        if log && matches!(e, super::super::GunEvent::ReloadStart) { eprintln!("[vr] tick {tick} reloading"); }
    }
    if p.gun_events.iter().any(|e| matches!(e, super::super::GunEvent::DryFire)) { inp.rumble(g, 0.2, 0.03); }
    // a thrown object strikes RinRin or a boss it flies into (the grunts: Props' own flight)
    for (i, at, v) in props.vr_flying() {
        let dir = v.normalize_or_zero();
        let power = (v.length() * 30. * 3.).clamp(10., 40.) as i32;
        let hit = if !p.fight.solo && p.fight.rin.hp > 0 && in_capsule(Vec3::from_array(p.fight.rin.pos), at + Vec3::Y * 0.2, 0.6, 0.0, 2.4) {
            p.fight.rin_hurt(&p.fd, 0x107, power); true
        } else { elise.vr_punch(at, at - dir, power, &mut fx) || baron.vr_punch(at, at - dir, power, true, &mut fx) };
        if hit {
            fx.impact(at + Vec3::Y * 0.2, dir.x.atan2(dir.z), 1.2, [0.9, 0.85, 0.7]);
            props.vr_break(i);
            if log { eprintln!("[vr] tick {tick} thrown object {i} struck the boss ({power})"); }
        }
    }
    // a body thrown in VR knocks down whoever it flies into, and lands hard
    street.vr_flights(&mut fx);
    let _ = (aims, head);
}

/// A point within a standing capsule at `base` (radius `r`, from `lo` to `hi` over its base), plus a little reach.
fn in_capsule(base: Vec3, p: Vec3, r: f32, lo: f32, hi: f32) -> bool {
    Vec2::new(p.x - base.x, p.z - base.z).length() < r && p.y > base.y + lo && p.y < base.y + hi
}

/// RinRin is within a hand's reach (her body, as a grab sees it).
fn near_rin(p: &super::super::Play, at: Vec3) -> bool {
    p.as_kreely && !p.fight.solo && p.fight.rin.hp > 0 && in_capsule(Vec3::from_array(p.fight.rin.pos), at, 0.7, 0.2, 2.3)
}
/// Where her feet go while held: under the holding hands (both: between them), she hanging by the collar a body's
/// length below them, never through the floor under her.
fn rin_hold_at(hands: &[Transform; 2], mask: [bool; 2], h: usize, body: Vec3) -> Vec3 {
    let holding: Vec<Vec3> = (0..2).filter(|k| mask[*k]).map(|k| hands[k].translation).collect();
    let grip = if holding.is_empty() { hands[h].translation } else { holding.iter().sum::<Vec3>() / holding.len() as f32 };
    // a little out from his chest so she does not sit inside him
    let out = Vec3::new(grip.x - body.x, 0., grip.z - body.z);
    let at = grip + out.normalize_or_zero() * 0.15 - Vec3::Y * 1.25;
    Vec3::new(at.x, at.y.max(body.y - 0.2), at.z)
}
/// She faces him while held.
fn rin_facing(p: &super::super::Play, body: Vec3) -> f32 {
    let r = Vec3::from_array(p.fight.rin.pos);
    (body.x - r.x).atan2(body.z - r.z)
}

fn near_boss(p: &super::super::Play, elise: &super::super::elise::Elise, baron: &super::super::baron::Baron, at: Vec3) -> bool {
    if !p.as_kreely { return false; }
    let rin = !p.fight.solo && p.fight.rin.hp > 0 && in_capsule(Vec3::from_array(p.fight.rin.pos), at, 0.7, 0.2, 2.3);
    let el = elise.targetable() && elise.body().distance(at) < 0.9;
    let bn = baron.targetable() && baron.body().distance(at) < 0.9;
    rin || el || bn
}

/// A hand gripping the stage: the top of a ledge just under the palm, or a wall the hand touches. Returns where it holds
/// and the ledge top.
fn climb_hold(world: &mw_data::collision::CollisionWorld, hand: &Transform, body: Vec3) -> Option<(Vec3, Option<f32>)> {
    let at = hand.translation;
    // a ledge: a floor top a hand's height under / at the palm (or just beyond it, reaching over an edge), above the feet
    let reach = Vec3::new(at.x - body.x, 0., at.z - body.z).normalize_or_zero();
    for k in [0., 0.08, 0.16] {
        let p = at + reach * k;
        if let Some(top) = top_near(world, Vec3::new(p.x, at.y, p.z), 0.3, 0.6) {
            if top <= at.y + 0.14 && top >= at.y - 0.25 && top > body.y + 0.45 { return Some((Vec3::new(at.x, at.y, at.z), Some(top))); }
        }
    }
    // a wall: any stage surface within 10 cm of the palm, along the hand's axes
    let dirs = [hand.rotation * Vec3::NEG_Z, hand.rotation * Vec3::X, hand.rotation * Vec3::NEG_X, hand.rotation * Vec3::NEG_Y, hand.rotation * Vec3::Y];
    for d in dirs {
        let d = d.normalize_or_zero();
        let o = [at.x as f64 - d.x as f64 * 0.04, at.y as f64 - d.y as f64 * 0.04, at.z as f64 - d.z as f64 * 0.04];
        if let Ok(hits) = mw_sim::collision::raw_surface_ray(world, o, super::super::unit(d), 0.14, None) {
            if hits.iter().any(|h| h.distance > 0. && matches!(h.body.source.as_str(), "col.hkx" | "col_obj.hkx" | "col_pl.hkx")) { return Some((at, None)); }
        }
    }
    None
}

/// The highest stage surface top within `above` over / `depth` under `at` (a short probe: not the roof over it).
pub fn top_near(world: &mw_data::collision::CollisionWorld, at: Vec3, above: f32, depth: f32) -> Option<f32> {
    let o = [at.x as f64, (at.y + above) as f64, at.z as f64];
    mw_sim::collision::raw_surface_ray(world, o, [0., -1., 0.], (above + depth) as f64, None).ok()?
        .iter().filter(|h| h.distance > 1e-3 && h.normal[1] > 0.5 && matches!(h.body.source.as_str(), "col.hkx" | "col_obj.hkx"))
        .map(|h| h.distance).min_by(f64::total_cmp).map(|d| (o[1] - d) as f32)
}
