//! Kreely's pistol worked by hand in VR [added] (the "Pistol handling: REALISTIC" setting; SIMPLE keeps the old
//! holster-and-X-reload pistol). Everything a real pistol asks of your hands:
//! - **Draw / holster / drop**: grip it at the gun-hand hip to draw; let go of it there to holster it, anywhere else to
//!   drop it (it falls and lies where it lands; a swing throws it). Pick it up with either hand.
//! - **Magazine**: the gun hand's A (left-handed: X) drops the magazine out of the grip. Grip at the other hip (the
//!   pouch) to take a fresh one (from the reserve), push its top into the bottom of the grip to seat it. Magazines let
//!   go of elsewhere fall and can be picked up again; let go of at the pouch, their rounds go back to the reserve.
//! - **Slide**: grip the rear of the slide with the free hand and pull it back, then let go (or push it home): a round
//!   from the magazine is chambered (a round already in the chamber flies out). Fired dry, the slide locks back on the
//!   empty magazine; a fresh magazine and a rack (or the gun hand's B / Y, the slide release) send it home.
//! - **Trigger**: a chambered round fires (the game's own shot along the barrel, play.rs); an empty chamber clicks.
//! The magazine and chamber are the game's `ammo.mag` (chamber + magazine), the pouch its `ammo.reserve`.
//!
//! The pistol model (ev93 wp0671) is one mesh: its slide (everything above the frame) is cut out into a part of its
//! own that slides back; the magazine is a dark box sized to the grip.
use super::interact::{Hold, Interact};
use super::{Rig, VrSettings};
use bevy::{camera::visibility::RenderLayers, prelude::*};

/// The model's frame (x back, the muzzle at -x; y across; z up, the slide's top at +0.09, the butt at -0.09): the
/// slide's travel, where its rear is gripped, the bottom of the magazine well and the magazine's way in (up the grip).
pub const TRAVEL: f32 = 0.04;
const SLIDE_ABOVE: f32 = 0.045;
const SLIDE_GRAB: Vec3 = Vec3::new(0.115, 0., 0.07);
const MAGWELL: Vec3 = Vec3::new(0.103, 0., -0.088);
fn mag_axis() -> Vec3 { Vec3::new(0.15, 0., 1.).normalize() }
/// The bore's mouth, the sights' dots (front post, the rear notch's two), the range the sights are zeroed at (the
/// round meets the sight line there), and the middle of the grip (where a support hand closes).
const BORE: Vec3 = Vec3::new(-0.09, 0., 0.065);
const FRONT_DOT: Vec3 = Vec3::new(-0.074, 0., 0.101);
const REAR_DOT: Vec3 = Vec3::new(0.098, 0., 0.101);
const REAR_GAP: f32 = 0.0045;
const ZERO: f32 = 15.;
const GRIP_MID: Vec3 = Vec3::new(0.11, 0., -0.04);
const SUPPORT_REACH: f32 = 0.12;
/// Recoil: the muzzle flip (radians at full kick) one-handed and with a support hand.
const FLIP: f32 = 0.30;
const FLIP_SUPPORTED: f32 = 0.11;

/// The magazine box: depth (front to back), height (along its axis), width.
const MAG_SIZE: Vec3 = Vec3::new(0.03, 0.11, 0.02);
/// Reaches (m): the slide's grip, a loose gun, a loose magazine, the pouch, a magazine's top to the well.
const SLIDE_REACH: f32 = 0.09;
const GUN_REACH: f32 = 0.16;
const MAG_REACH: f32 = 0.13;
pub const POUCH_REACH: f32 = 0.22;
const SEAT_REACH: f32 = 0.06;
const LOOSE_MAX: usize = 6;

#[derive(Clone, Copy, Debug)]
pub enum GunAt { Holster, Hand(usize), Loose { tf: Transform, vel: Vec3, spin: Vec3, rest: bool } }

#[derive(Clone, Copy, Debug)]
pub struct LooseMag { pub tf: Transform, vel: Vec3, spin: Vec3, rest: bool, pub rounds: u8, age: f32 }

#[derive(Resource, Debug)]
pub struct Pistol {
    pub at: GunAt,
    /// the magazine in the grip (its rounds), a round in the chamber, the slide held back on an empty magazine
    pub mag: Option<u8>, pub chamber: bool, pub locked: bool,
    /// the slide pulled back by hand (0..1 of its travel), the stroke went all the way back, where the hand took it
    pub pull: f32, primed: bool, rack_from: Option<Vec3>,
    /// the slide's kick after a shot (1 → 0)
    pub kick: f32,
    /// requests from the buttons (Update) for the next fixed step: drop the magazine, the slide release, a dry click
    pub eject_req: bool, pub release_req: bool, pub dry_req: bool,
    pub loose: Vec<LooseMag>,
    /// what the game's ammo.mag was set to last step (another change: the flat game, a refill, a fresh start)
    synced: Option<u8>,
    /// this step's gun root (the model's frame in the world)
    pub root: Transform,
    /// the other hand closed round the grip (two-handed: the recoil flips less)
    pub support: bool,
    /// rounds racked out of the chamber this step (port, velocity), for the ejector
    pub racked_out: Vec<(Vec3, Vec3)>,
}
impl Default for Pistol {
    fn default() -> Self {
        Self { at: GunAt::Holster, mag: None, chamber: false, locked: false, pull: 0., primed: false, rack_from: None, kick: 0.,
               eject_req: false, release_req: false, dry_req: false, loose: vec![], synced: None, root: Transform::IDENTITY, support: false, racked_out: vec![] }
    }
}
impl Pistol {
    pub fn in_hand(&self) -> Option<usize> { if let GunAt::Hand(h) = self.at { Some(h) } else { None } }
    /// It fires on the trigger now.
    pub fn ready(&self) -> bool { self.chamber && !self.locked && self.pull < 0.15 }
    fn rounds(&self) -> u8 { self.chamber as u8 + self.mag.unwrap_or(0) }
    /// The slide goes home: it strips a round from the magazine, or stays back on an empty one.
    fn slide_home(&mut self) -> bool {
        self.pull = 0.; self.primed = false;
        match self.mag {
            Some(n) if n > 0 => { self.mag = Some(n - 1); self.chamber = true; self.locked = false; true }
            Some(_) if !self.chamber => { self.locked = true; false }
            _ => { self.locked = false; false }
        }
    }
}

/// This step's gun root from the hands (or the holster / where it lies).
pub fn update_root(ps: &mut Pistol, grips: &[Transform; 2], aims: &[Transform; 2], body: Vec3, yaw: f32, g: usize, recoil: f32) {
    ps.root = match ps.at {
        GunAt::Hand(h) => root_in_hand(&super::hands::gun_frame(&grips[h], &aims[h]), flip(ps, recoil)),
        GunAt::Holster => super::hands::holster_root(body, yaw, g),
        GunAt::Loose { tf, .. } => tf,
    };
}

/// The muzzle and the round's way out: along the bore, tipped so the round crosses the sight line at ZERO (the sights
/// sit 3.6 cm over the bore) — what the sights cover at that range is what it strikes, as with a zeroed pistol.
pub fn barrel(root: &Transform) -> (Vec3, Vec3) {
    let muzzle = root.transform_point(BORE);
    let along = root.rotation * Vec3::NEG_X;
    let sight = root.transform_point(REAR_DOT);
    (muzzle, (sight + along * ZERO - muzzle).normalize())
}
/// The recoil's flip for the game's recoil (1 at the shot, decaying), one-handed or supported.
pub fn flip(ps: &Pistol, recoil: f32) -> f32 { recoil * if ps.support { FLIP_SUPPORTED } else { FLIP } / 0.35 }

/// The gun's root (the model's frame) for a hand frame, with the recoil's lift.
pub fn root_in_hand(frame: &Transform, recoil: f32) -> Transform { super::hands::gun_root(frame, recoil) }
pub fn slide_grab(root: &Transform) -> Vec3 { root.transform_point(SLIDE_GRAB) }
/// The bottom of the well and the way in (up the grip), in the world.
pub fn magwell(root: &Transform) -> (Vec3, Vec3) { (root.transform_point(MAGWELL), root.rotation * mag_axis()) }
/// Back along the slide, in the world.
pub fn rearward(root: &Transform) -> Vec3 { root.rotation * Vec3::X }
/// A magazine's box rotation for its axis (box +Y) and its width (box +Z).
fn mag_rot(up: Vec3, across: Vec3) -> Quat {
    let up = up.normalize();
    let across = (across - up * across.dot(up)).normalize_or(up.any_orthonormal_vector());
    Quat::from_mat3(&Mat3::from_cols(up.cross(across), up, across))
}
/// A magazine in a hand: in the fist, its top up past the thumb, tipped forward.
pub fn mag_in_hand(grip: &Transform) -> Transform {
    let up = grip.rotation * Vec3::new(0., 1., -0.35).normalize();
    Transform { translation: grip.translation + grip.rotation * Vec3::new(0., 0.02, -0.02), rotation: mag_rot(up, grip.rotation * Vec3::X), scale: Vec3::ONE }
}
/// The top of a magazine's box and its axis.
fn mag_top(tf: &Transform) -> (Vec3, Vec3) { let up = tf.rotation * Vec3::Y; (tf.translation + up * MAG_SIZE.y * 0.5, up) }
/// A magazine seated in the grip (its base a centimetre below it).
pub fn mag_in_gun(root: &Transform) -> Transform {
    let c = MAGWELL + mag_axis() * (MAG_SIZE.y * 0.5 - 0.01);
    Transform { translation: root.transform_point(c), rotation: root.rotation * mag_rot(mag_axis(), Vec3::Y), scale: Vec3::ONE }
}

/// Falling under gravity onto the stage's floors; at rest it lies flat (`flat`: the rotation lying down at yaw 0,
/// `half`: half its thickness).
fn fall(tf: &mut Transform, vel: &mut Vec3, spin: &mut Vec3, rest: &mut bool, world: &mw_data::collision::CollisionWorld, flat: Quat, half: f32, dt: f32) {
    if *rest { return; }
    vel.y -= 9.8 * dt;
    let next = tf.translation + *vel * dt;
    let ground = super::interact::top_near(world, tf.translation, 0.3, 4.);
    if let Some(gy) = ground.filter(|gy| next.y - half <= *gy) {
        let f = tf.rotation * Vec3::NEG_Z;
        let yaw = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::new(f.x, 0., f.z).normalize_or(Vec3::NEG_Z));
        *tf = Transform { translation: Vec3::new(next.x, gy + half, next.z), rotation: yaw * flat, scale: Vec3::ONE };
        *vel = Vec3::ZERO; *spin = Vec3::ZERO; *rest = true;
        return;
    }
    tf.translation = next;
    tf.rotation = (Quat::from_scaled_axis(*spin * dt) * tf.rotation).normalize();
}

/// The fixed step's grips on the pistol and the magazines (from play_vr_interact.rs `step`, a grip closing): the
/// slide (the gun in the other hand), a loose gun, a loose magazine, the holster, the pouch. Side effects done.
#[allow(clippy::too_many_arguments)]
pub fn grab(ps: &mut Pistol, act: &mut Interact, h: usize, at: Vec3, holster: Vec3, pouch: Vec3, reserve: &mut u16, log: bool, tick: u64) -> Option<Hold> {
    let other = 1 - h;
    if ps.in_hand() == Some(other) {
        let (ds, dg) = (at.distance(slide_grab(&ps.root)), at.distance(ps.root.transform_point(GRIP_MID)));
        if ds < SLIDE_REACH && ds <= dg {
            if log { eprintln!("[vr] tick {tick} hand {h} takes the slide"); }
            return Some(Hold::Slide);
        }
        if dg < SUPPORT_REACH {
            if log { eprintln!("[vr] tick {tick} hand {h} supports the pistol (two hands)"); }
            return Some(Hold::Support);
        }
    }
    if matches!(ps.at, GunAt::Loose { .. }) && at.distance(ps.root.translation) < GUN_REACH {
        ps.at = GunAt::Hand(h); act.gun_drawn = true; act.gun_hand = h;
        if log { eprintln!("[vr] tick {tick} hand {h} picked up the pistol"); }
        return Some(Hold::Gun);
    }
    if let Some(k) = ps.loose.iter().enumerate().filter(|(_, m)| m.tf.translation.distance(at) < MAG_REACH)
        .min_by(|a, b| a.1.tf.translation.distance(at).total_cmp(&b.1.tf.translation.distance(at))).map(|(k, _)| k) {
        let m = ps.loose.remove(k);
        if log { eprintln!("[vr] tick {tick} hand {h} picked up a magazine ({} rounds)", m.rounds); }
        return Some(Hold::Mag(m.rounds));
    }
    if matches!(ps.at, GunAt::Holster) && at.distance(holster) < super::interact::HOLSTER_REACH {
        ps.at = GunAt::Hand(h); act.gun_drawn = true; act.gun_hand = h;
        if log { eprintln!("[vr] tick {tick} drew the pistol (hand {h}; chamber {}, magazine {:?})", ps.chamber, ps.mag); }
        return Some(Hold::Gun);
    }
    if at.distance(pouch) < POUCH_REACH && *reserve > 0 {
        let n = (super::super::MAG as u16).min(*reserve) as u8;
        *reserve -= n as u16;
        if log { eprintln!("[vr] tick {tick} hand {h} takes a magazine from the pouch ({n} rounds, {} left)", reserve); }
        return Some(Hold::Mag(n));
    }
    None
}

/// A grip opening on the pistol or a magazine: holstered / dropped / thrown, into the pouch / dropped.
#[allow(clippy::too_many_arguments)]
pub fn release(ps: &mut Pistol, hold: Hold, h: usize, grip: &Transform, v: Vec3, ang: Vec3, holster: Vec3, pouch: Vec3, reserve: &mut u16, log: bool, tick: u64) {
    match hold {
        Hold::Gun => {
            if grip.translation.distance(holster) < super::interact::HOLSTER_REACH { ps.at = GunAt::Holster; if log { eprintln!("[vr] tick {tick} holstered the pistol"); } }
            else { ps.at = GunAt::Loose { tf: ps.root, vel: v, spin: ang, rest: false }; if log { eprintln!("[vr] tick {tick} dropped the pistol at {:.1} m/s", v.length()); } }
        }
        Hold::Mag(n) => {
            if grip.translation.distance(pouch) < POUCH_REACH {
                *reserve = (*reserve + n as u16).min(super::super::RESERVE_MAX);
                if log { eprintln!("[vr] tick {tick} put a magazine back in the pouch ({n} rounds)"); }
            } else {
                drop_mag(ps, mag_in_hand(grip), v, ang, n);
                if log { eprintln!("[vr] tick {tick} hand {h} dropped a magazine ({n} rounds)"); }
            }
        }
        _ => {}
    }
}

fn drop_mag(ps: &mut Pistol, tf: Transform, vel: Vec3, spin: Vec3, rounds: u8) {
    if ps.loose.len() >= LOOSE_MAX { ps.loose.remove(0); }
    ps.loose.push(LooseMag { tf, vel, spin, rest: false, rounds, age: 0. });
}

/// A magazine held by hand `h` meets the bottom of the grip of the gun in the other hand: it seats.
pub fn try_seat(ps: &Pistol, h: usize, grip: &Transform) -> bool {
    if ps.in_hand() != Some(1 - h) || ps.mag.is_some() { return false; }
    let (top, up) = mag_top(&mag_in_hand(grip));
    let (well, way) = magwell(&ps.root);
    top.distance(well) < SEAT_REACH && up.dot(way) > 0.3
}
pub fn seat(ps: &mut Pistol, n: u8) { ps.mag = Some(n); }

/// The fixed step, after the grips: the slide by hand, the buttons, the shots fired this tick, loose things falling,
/// and the game's magazine count kept as chamber + magazine.
#[allow(clippy::too_many_arguments)]
pub fn step(ps: &mut Pistol, act: &Interact, hands: &[Transform; 2], p: &mut super::super::Play, inp: &mut super::VrInput, snd: &mut super::super::Sounds,
            world: &mw_data::collision::CollisionWorld, body: Vec3, log: bool) {
    let tick = p.tick;
    let dt = 1. / 30.;
    let sound = |snd: &mut super::super::Sounds, id: u32, at: Vec3| super::super::queue_sound(snd, tick, super::super::GUN_BLOCK, id, at.to_array());
    // the game's count changed by something else (the flat game, a fresh start): a round in the chamber, the rest in
    // the magazine
    let shots = p.gun_events.iter().filter(|e| matches!(e, super::super::GunEvent::Shot { .. })).count() as u8;
    if ps.synced.is_none_or(|s| s != p.ammo.mag.saturating_add(shots)) {
        ps.chamber = p.ammo.mag > 0; ps.mag = Some(p.ammo.mag.saturating_sub(1)); ps.locked = false;
        if log { eprintln!("[vr] tick {tick} pistol: {} in the chamber, {:?} in the magazine", ps.chamber as u8, ps.mag); }
    } else {
        // the shots fired: each takes the chambered round; the slide strips the next or locks back on an empty magazine
        for _ in 0..shots {
            ps.chamber = false; ps.kick = 1.;
            match ps.mag { Some(n) if n > 0 => { ps.mag = Some(n - 1); ps.chamber = true; } Some(_) => { ps.locked = true; } None => {} }
            if log { eprintln!("[vr] tick {tick} pistol fired: chamber {}, magazine {:?}{}", ps.chamber, ps.mag, if ps.locked { ", slide locked back" } else { "" }); }
        }
    }
    let gh = ps.in_hand();
    let gun_at = ps.root.translation;
    // [test] where the magazine well and the slide sit in the gun hand's own frame (scripted hands aim at them)
    if let Some(g) = gh.filter(|_| log && std::env::var_os("MW_VR_GUN_GEOM").is_some() && tick % 30 == 0) {
        let inv = hands[g].rotation.inverse();
        let local = |w: Vec3| inv * (w - hands[g].translation);
        let (well, way) = magwell(&ps.root);
        eprintln!("[vr] gun world axes: muzzle {:.2} top {:.2} side {:.2}", ps.root.rotation * Vec3::NEG_X, ps.root.rotation * Vec3::Z, ps.root.rotation * Vec3::Y);
        eprintln!("[vr] gun geometry (hand {g} frame): well {:.3} way {:.3} slide {:.3} rear {:.3}", local(well), inv * way, local(slide_grab(&ps.root)), inv * rearward(&ps.root));
    }
    // the buttons
    if std::mem::take(&mut ps.dry_req) {
        sound(snd, super::super::GUN_DRY, gun_at);
        if let Some(g) = gh { inp.rumble(g, 0.2, 0.03); }
        if log { eprintln!("[vr] tick {tick} click (empty chamber{})", if ps.locked { ", slide back" } else { "" }); }
    }
    if std::mem::take(&mut ps.eject_req) && gh.is_some() {
        if let Some(n) = ps.mag.take() {
            let tf = mag_in_gun(&ps.root);
            drop_mag(ps, tf, -(ps.root.rotation * mag_axis()) * 0.8, Vec3::ZERO, n);
            sound(snd, super::super::GUN_RELEASE, gun_at);
            if let Some(g) = gh { inp.rumble(g, 0.35, 0.04); }
            if log { eprintln!("[vr] tick {tick} magazine out ({n} rounds)"); }
        }
    }
    if std::mem::take(&mut ps.release_req) && ps.locked && gh.is_some() {
        // the lever drops the slide even on an empty magazine (racking by hand locks it back on one)
        let fed = ps.slide_home();
        if !fed { ps.locked = false; }
        sound(snd, super::super::GUN_SEAT, gun_at);
        if log { eprintln!("[vr] tick {tick} slide release: {}", if fed { "chambered" } else if ps.locked { "still locked (empty magazine)" } else { "home, empty" }); }
    }
    // the slide by hand
    let racking = (0..2).find(|h| act.hold[*h] == Hold::Slide && gh == Some(1 - *h));
    match (racking, ps.rack_from) {
        (Some(h), None) => { ps.rack_from = Some(hands[h].translation); }
        (Some(h), Some(from)) => {
            let back = rearward(&ps.root);
            let pull = ((hands[h].translation - from).dot(back) / TRAVEL).clamp(0., 1.);
            ps.pull = pull;
            let full = if ps.locked { 0.3 } else { 0.85 };
            if pull >= full && !ps.primed {
                ps.primed = true;
                if ps.chamber && !ps.locked {
                    ps.chamber = false;
                    let r = ps.root;
                    ps.racked_out.push((r.transform_point(Vec3::new(0.03, 0.012, 0.088)), r.rotation * Vec3::new(0.3, 1.6, 0.9)));
                    if log { eprintln!("[vr] tick {tick} a live round flies out"); }
                }
                inp.rumble(h, 0.4, 0.04);
                sound(snd, super::super::GUN_RELEASE, gun_at);
                if log { eprintln!("[vr] tick {tick} slide back"); }
            }
            // pushed home by hand
            if ps.primed && pull < 0.2 {
                let fed = ps.slide_home();
                inp.rumble(h, 0.6, 0.05); sound(snd, super::super::GUN_SEAT, gun_at);
                if log { eprintln!("[vr] tick {tick} slide home: {}", if fed { "chambered" } else if ps.locked { "locked back (empty magazine)" } else { "empty" }); }
                ps.rack_from = Some(hands[h].translation);
            }
        }
        (None, Some(_)) => {
            // let go: the spring sends it home
            if ps.primed {
                let fed = ps.slide_home();
                sound(snd, super::super::GUN_SEAT, gun_at);
                if let Some(g) = gh { inp.rumble(g, 0.5, 0.05); }
                if log { eprintln!("[vr] tick {tick} slide let go: {}", if fed { "chambered" } else if ps.locked { "locked back (empty magazine)" } else { "empty" }); }
            }
            ps.pull = 0.; ps.primed = false; ps.rack_from = None;
        }
        (None, None) => { ps.pull = 0.; }
    }
    // loose things fall and settle; a pistol lost far away (or under the stage) is back in the holster
    if let GunAt::Loose { tf, vel, spin, rest } = &mut ps.at {
        fall(tf, vel, spin, rest, world, Quat::IDENTITY, 0.018, dt);
        if tf.translation.distance(body) > 40. || tf.translation.y < body.y - 20. { ps.at = GunAt::Holster; if log { eprintln!("[vr] tick {tick} the pistol is back in the holster"); } }
        else { ps.root = *tf; }
    }
    let flat_mag = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    for m in &mut ps.loose { m.age += dt; fall(&mut m.tf, &mut m.vel, &mut m.spin, &mut m.rest, world, flat_mag, MAG_SIZE.x * 0.5, dt); }
    ps.loose.retain(|m| m.age < 120. && m.tf.translation.y > body.y - 20. && m.tf.translation.distance(body) < 60.);
    // the game's count: what is in the gun
    p.ammo.mag = ps.rounds().min(super::super::MAG);
    p.ammo.reload = None;
    ps.synced = Some(p.ammo.mag);
}

// ---- the models ----

#[derive(Component)] pub struct GunSplit;
#[derive(Component)] pub struct GunSlide;
/// 0: in the gun, 1 / 2: in the left / right hand, 3..: loose.
#[derive(Component)] pub struct MagModel(usize);

pub fn build(app: &mut App) {
    app.init_resource::<Pistol>()
        .add_systems(Startup, spawn_mags.after(super::setup))
        .add_systems(Update, (split, present.after(super::hands::present)).in_set(super::VrActive));
}

fn spawn_mags(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mesh = meshes.add(Cuboid::new(MAG_SIZE.x, MAG_SIZE.y, MAG_SIZE.z));
    let mat = materials.add(StandardMaterial { base_color: Color::srgb(0.07, 0.07, 0.085), perceptual_roughness: 0.55, metallic: 0.3, ..default() });
    for k in 0..3 + LOOSE_MAX {
        commands.spawn((MagModel(k), Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), Transform::default(), Visibility::Hidden,
                        RenderLayers::layer(super::L_SELF), bevy::light::NotShadowCaster));
    }
}

/// Once the pistol model is in: its slide (every triangle wholly above the frame) becomes a part of its own.
fn split(mut commands: Commands, guns: Query<Entity, (With<super::hands::GunModel>, Without<GunSplit>)>, children: Query<&Children>,
         parts: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    for root in &guns {
        let mut done = false;
        for d in children.iter_descendants(root) {
            let Ok((m, mat)) = parts.get(d) else { continue; };
            let Some(src) = meshes.get(&m.0) else { continue; };
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = src.attribute(Mesh::ATTRIBUTE_POSITION).cloned() else { continue; };
            let Some(idx) = src.indices() else { continue; };
            let idx: Vec<u32> = idx.iter().map(|i| i as u32).collect();
            let (mut frame, mut slide) = (vec![], vec![]);
            for t in idx.chunks(3).filter(|t| t.len() == 3) {
                if t.iter().all(|v| pos.get(*v as usize).is_some_and(|p| p[2] > SLIDE_ABOVE)) { slide.extend_from_slice(t); } else { frame.extend_from_slice(t); }
            }
            let mut f = src.clone(); f.insert_indices(bevy::mesh::Indices::U32(frame));
            let mut s = src.clone(); s.insert_indices(bevy::mesh::Indices::U32(slide.clone()));
            s.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX); s.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
            let (fh, sh) = (meshes.add(f), meshes.add(s));
            commands.entity(d).try_insert(Mesh3d(fh));
            let slide_e = commands.spawn((GunSlide, Mesh3d(sh), MeshMaterial3d(mat.0.clone()), Transform::default(), Visibility::Inherited,
                            RenderLayers::layer(super::L_SELF), bevy::light::NotShadowCaster, ChildOf(root))).id();
            // iron sights on the slide: a front post and a notched rear, each with a glowing dot (three-dot sights)
            let black = materials.add(StandardMaterial { base_color: Color::srgb(0.02, 0.02, 0.025), perceptual_roughness: 0.8, ..default() });
            let glow = materials.add(StandardMaterial { base_color: Color::srgb(0.6, 1., 0.55), emissive: LinearRgba::rgb(2.5, 6., 2.2), unlit: true, ..default() });
            let post = meshes.add(Cuboid::new(0.004, 0.0035, 0.013));
            let dot = meshes.add(Sphere::new(0.0016).mesh().uv(8, 6));
            let mut part = |m: &Handle<Mesh>, mat: &Handle<StandardMaterial>, at: Vec3| {
                commands.spawn((Mesh3d(m.clone()), MeshMaterial3d(mat.clone()), Transform::from_translation(at), Visibility::Inherited,
                                RenderLayers::layer(super::L_SELF), bevy::light::NotShadowCaster, ChildOf(slide_e)));
            };
            part(&post, &black, Vec3::new(FRONT_DOT.x - 0.002, 0., 0.095));
            part(&dot, &glow, FRONT_DOT + Vec3::X * 0.0003);
            for side in [-1., 1.] {
                part(&post, &black, Vec3::new(REAR_DOT.x + 0.002, side * (REAR_GAP + 0.0012), 0.095));
                part(&dot, &glow, Vec3::new(REAR_DOT.x, side * (REAR_GAP + 0.0012), REAR_DOT.z));
            }
            if std::env::var_os("MW_VR_LOG").is_some() { eprintln!("[vr] pistol: slide cut out ({} triangles)", slide.len() / 3); }
            done = true;
        }
        if done { commands.entity(root).try_insert(GunSplit); }
    }
}

/// Update: the pistol where it is (hand, holster, the floor), its slide, the magazines.
#[allow(clippy::too_many_arguments)]
pub fn present(rig: Res<Rig>, act: Res<Interact>, set: Res<VrSettings>, view: Res<super::View>, play: Res<super::super::Play>, fixed: Res<Time<Fixed>>,
               time: Res<Time<Real>>, mut ps: ResMut<Pistol>, mut ej: ResMut<super::super::tracer::Ejector>, mut guns: Query<(&super::hands::GunModel, &mut Transform, &mut Visibility), (Without<GunSlide>, Without<MagModel>)>,
               mut slides: Query<&mut Transform, (With<GunSlide>, Without<super::hands::GunModel>, Without<MagModel>)>,
               mut mags: Query<(&MagModel, &mut Transform, &mut Visibility), (Without<GunSlide>, Without<super::hands::GunModel>)>) {
    if !rig.started { return; }
    let g = set.main_hand();
    let body_yaw = super::game_yaw(rig.head.rotation);
    let body = play.prev_pos.lerp(Vec3::from_array(play.fight.player.pos), fixed.overstep_fraction());
    let root = match ps.at {
        GunAt::Hand(h) => root_in_hand(&super::hands::gun_frame(&rig.grip[h], &rig.aim[h]), flip(&ps, play.ammo.recoil)),
        GunAt::Holster => super::hands::holster_root(body, body_yaw, g),
        GunAt::Loose { tf, .. } => tf,
    };
    let show = !view.cinema && play.as_kreely;
    // the cases fly from this gun's port while it is in a hand
    ej.vr = matches!(ps.at, GunAt::Hand(_)).then_some(root);
    for (port, vel) in std::mem::take(&mut ps.racked_out) { ej.extra.push((port, vel, true)); }
    for (gm, mut t, mut v) in &mut guns {
        *v = if gm.is_colored() == play.colored && show { Visibility::Visible } else { Visibility::Hidden };
        *t = root;
    }
    ps.kick = (ps.kick - time.delta_secs() * 14.).max(0.);
    let back = if ps.locked { 1. } else { ps.pull.max(ps.kick) };
    for mut t in &mut slides { t.translation = Vec3::X * TRAVEL * back; }
    let manual = set.gun_manual;
    for (m, mut t, mut v) in &mut mags {
        let (tf, on) = match m.0 {
            0 => (mag_in_gun(&root), ps.mag.is_some() && manual),
            1 | 2 => (mag_in_hand(&rig.grip[m.0 - 1]), matches!(act.hold[m.0 - 1], Hold::Mag(_))),
            k => ps.loose.get(k - 3).map_or((Transform::default(), false), |l| (l.tf, true)),
        };
        *v = if on && show { Visibility::Visible } else { Visibility::Hidden };
        if on { *t = tf; }
    }
}
