//! The street's placed objects (MW_STAGE=street; case2/c201.dat ObjTbl, assets/native/world/street-props.json): what the
//! player picks up, carries, throws, sets down or slams over a grunt's head, and the set-pieces grunts are impaled on
//! (research in the scratchpad pickups-research, addresses there).
//! - Pickables: crates bl0019, baskets bl0013, Madam Yang heads bl0003, bins bl002d, garbage bags bl0104, turtles bl0122.
//!   The player's object states (mw_sim fight 0x1A pick 021a, taken at frame 6; 0x1B hold 021b / walk 021e; 0x1D throw
//!   021d, let go at 6; 0x1E put down 021c; 0x23 the slam over a head 0218) come from the same A hold as the grunt grab,
//!   when no grunt is in reach and an object is (`can_pick`: PICK_REACH, in front) [the per-object pick radius is open].
//!   Thrown (OBJ_THROW 0x107, power 20: speed 25 x 0.25, pitch -5 deg) it staggers the grunt it hits and breaks; landing
//!   slowly it stays. Over a head (OBJ_COVER 0xF6 within 2.5 m) the grunt is blinded, staggering under it, until it falls
//!   off [the covered grunt's own clips em0110_1513 / 1518 are not staged: the stun stands in].
//! - Impale set-pieces (collision attribute SF00004000 [inferred from the body names]): the bamboo spike wall bl0181 and
//!   the four meat hooks bl0106. A grunt flying (launched or thrown) whose head meets one sticks (play_street.rs Stuck:
//!   101c, slides off 101d, lands alive 101e or dead 101f; WALL_CRASH 20; blood core 0x1A5 / 0x1A9).
use bevy::prelude::*;

const PICK_REACH: f32 = 1.4;
const GRAVITY: f32 = 9.8 / 900.;
/// The throw: 25 x 0.25 m/s (a 30 Hz frame), pitched down 5 deg; faster than BREAK_SPEED on landing it breaks.
const THROW_SPEED: f32 = 25. * 0.25 / 30.;
const BREAK_SPEED: f32 = 0.08;

#[derive(serde::Deserialize)]
struct PropFile { props: Vec<PropRec> }
#[derive(serde::Deserialize, Clone)]
struct PropRec { object: String, kind: String, position: [f32; 3], rotation: [f32; 3] }

#[derive(Clone, Copy, PartialEq, Debug)]
enum PState { Placed, Reaching, Held, Flying, OnHead(u32), Gone,
              /// [added, VR] in a hand: the object's offset and turn in the hand's frame
              VrHeld(Vec3, Quat) }
struct Prop { kind: String, roots: [Entity; 2], pos: Vec3, rot: Quat, vel: Vec3, spin: Vec3, state: PState, pickable: bool, bound: bool }

#[derive(Resource, Default)]
pub struct Props { on: bool, spawned: bool, list: Vec<Prop>, held: Option<usize> }

/// The impale set-pieces' zones (local box: x, y, z ranges; the object's position and yaw), from street-props.json.
fn zones() -> &'static Vec<(Vec3, f32, [f32; 6])> {
    static Z: std::sync::OnceLock<Vec<(Vec3, f32, [f32; 6])>> = std::sync::OnceLock::new();
    Z.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/world/street-props.json");
        let Some(f) = std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str::<PropFile>(&t).ok()) else { return vec![]; };
        f.props.iter().filter_map(|p| {
            // (the bodies' spikes / hooks: the spike wall's face, 0.3-3 m up; a hook's hanging span, 1.35-2.6 m)
            let b = match p.kind.as_str() { "spike_wall" => [-2.5, 2.6, 0.3, 3.0, -0.6, 0.9], "meat_hook" => [-0.6, 0.65, 1.35, 2.6, -0.55, 0.55], _ => return None };
            Some((Vec3::from_array(p.position), p.rotation[1], b))
        }).collect()
    })
}
/// Where a grunt's head at `head` sticks, if it meets a spike wall or a hook.
pub fn impale_at(head: Vec3) -> Option<Vec3> {
    zones().iter().find_map(|(at, yaw, b)| {
        let l = Quat::from_rotation_y(-*yaw) * (head - *at);
        (l.x > b[0] && l.x < b[1] && l.y > b[2] && l.y < b[3] && l.z > b[4] && l.z < b[5]).then_some(head)
    })
}

pub struct PropsPlugin;
impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        let on = super::street::start().is_some() && std::env::var("MW_PROPS").as_deref() != Ok("0");
        app.insert_resource(Props { on, ..default() })
            .add_systems(Update, (spawn, look))
            .add_systems(FixedUpdate, step.after(super::tick).after(super::street::step).before(super::announce)
                .run_if(super::pause::running).run_if(super::intro::fight_running).run_if(super::cutin::world_running));
    }
}

impl Props {
    /// An object the player can pick up now: placed, pickable, within reach in front of him and at his level.
    pub fn can_pick(&self, pl: Vec3, facing: f32) -> bool { self.pick_target(pl, facing).is_some() }
    fn pick_target(&self, pl: Vec3, facing: f32) -> Option<usize> {
        if !self.on || self.held.is_some() { return None; }
        let fwd = Vec2::new(facing.sin(), facing.cos());
        self.list.iter().enumerate().filter(|(_, p)| p.pickable && p.state == PState::Placed && (p.pos.y - pl.y).abs() < 1.2)
            .map(|(i, p)| (i, Vec2::new(p.pos.x - pl.x, p.pos.z - pl.z)))
            .filter(|(_, d)| d.length() < PICK_REACH && d.normalize_or_zero().dot(fwd) > 0.3)
            .min_by(|a, b| a.1.length().total_cmp(&b.1.length())).map(|(i, _)| i)
    }
}

fn spawn(mut p: ResMut<Props>, assets: Res<AssetServer>, mut commands: Commands) {
    if !p.on || p.spawned { return; }
    p.spawned = true;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let Some(f) = std::fs::read_to_string(root.join("native/world/street-props.json")).ok().and_then(|t| serde_json::from_str::<PropFile>(&t).ok()) else { return; };
    for r in f.props {
        let file = format!("native/objects/{}.glb", r.object);
        if !root.join(&file).exists() { continue; }
        let color = file.replace(".glb", "-color.glb");
        let files = [file.clone(), if root.join(&color).exists() { color } else { file }];
        let rot = Quat::from_euler(EulerRot::YXZ, r.rotation[1], r.rotation[0], r.rotation[2]);
        let pos = Vec3::from_array(r.position);
        let roots = files.map(|f| commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(f))),
                                                  Transform::from_translation(pos).with_rotation(rot), Visibility::Hidden)).id());
        let pickable = !matches!(r.kind.as_str(), "spike_wall" | "meat_hook");
        p.list.push(Prop { kind: r.kind, roots, pos, rot, vel: Vec3::ZERO, spin: Vec3::ZERO, state: PState::Placed, pickable, bound: false });
    }
}

/// Update: the source or the colored model by the look; only intact objects on show (broken intact meshes: the D0 node).
fn look(mut p: ResMut<Props>, play: Res<super::Play>, mut vis: Query<&mut Visibility>, mut tf: Query<&mut Transform>,
        children: Query<&Children>, names: Query<&Name>) {
    if !p.on { return; }
    let shown = play.colored as usize;
    for o in p.list.iter_mut() {
        // (the broken shape, a D1 / D9 node, stays hidden: an object breaks by going)
        if !o.bound {
            let mut found = false;
            for r in o.roots { for e in children.iter_descendants(r) {
                if let Ok(n) = names.get(e) { found = true; let s = n.as_str(); if s.contains("_D1_") || s.contains("_D9_") || s.starts_with("D1_") || s.starts_with("D9_") { if let Ok(mut v) = vis.get_mut(e) { *v = Visibility::Hidden; } } }
            } }
            o.bound = found;
        }
        for (k, r) in o.roots.iter().enumerate() {
            if let Ok(mut v) = vis.get_mut(*r) { let want = if k == shown && o.state != PState::Gone { Visibility::Visible } else { Visibility::Hidden }; if *v != want { *v = want; } }
            if let Ok(mut t) = tf.get_mut(*r) { t.translation = o.pos; t.rotation = o.rot; }
        }
    }
}

/// FixedUpdate: the player's object events, the flight, the head it covers.
fn step(mut p: ResMut<Props>, mut play: ResMut<super::Play>, mut street: ResMut<super::street::Street>, arena: Res<super::Arena>,
        mut fx: ResMut<super::fx::Fx>) {
    if !p.on { return; }
    let pl = Vec3::from_array(play.fight.player.pos);
    let f = play.fight.player.loco.facing;
    let fwd = Vec3::new(f.sin(), 0., f.cos());
    // inspection: MW_PROPS_TEST=n stands the player 0.9 m from object n, facing it, at tick 50
    if play.tick == 50 { if let Some(n) = std::env::var("MW_PROPS_TEST").ok().and_then(|v| v.parse::<usize>().ok()).filter(|n| *n < p.list.len()) {
        let o = &p.list[n];
        let out = Vec3::new(0., 0., 0.9);
        let at = o.pos + out;
        play.fight.player.pos = [at.x, o.pos.y, at.z]; play.fight.player.loco.facing = std::f32::consts::PI; play.cam_yaw = std::f32::consts::PI;
        eprintln!("[props] test: {} {} at {:.2}", n, o.kind, o.pos);
    } }
    // inspection: MW_IMPALE_TEST=n launches a grunt at impale zone n (0 the spike wall, 1.. the hooks) from 3 m out, at tick 60
    if let Some(n) = std::env::var("MW_IMPALE_TEST").ok().and_then(|v| v.parse::<usize>().ok()) {
        let mut z: Vec<_> = zones().iter().collect(); z.sort_by_key(|(_, _, b)| (b[1] < 1.) as u8);
        if let Some((at, yaw, _)) = z.get(n) {
            let out = Quat::from_rotation_y(*yaw) * Vec3::Z;
            if play.tick == 50 { let pp = *at + out * 6. + Quat::from_rotation_y(*yaw) * Vec3::X * 2.; play.fight.player.pos = [pp.x, at.y, pp.z]; let d = *at - pp; play.fight.player.loco.facing = d.x.atan2(d.z); play.cam_yaw = play.fight.player.loco.facing; }
            if play.tick == 60 { street.test_launch(*at + out * 3., -out * 0.35); eprintln!("[props] impale test at {:.2} yaw {:.2}", at, yaw); }
        }
    }
    let events = play.fight.events.clone();
    let log = std::env::var_os("MW_PROPS_LOG").is_some();
    if log && play.tick % 10 == 0 && play.tick < 200 { eprintln!("[props] tick {} pl {:.2} facing {:.2} state {} pick {:?}", play.tick, pl, f, play.fight.player.loco.state, p.pick_target(pl, f)); }
    for e in &events {
        if log && matches!(e, mw_sim::fight::FightEvent::PickStart | mw_sim::fight::FightEvent::PickTake | mw_sim::fight::FightEvent::ObjThrow | mw_sim::fight::FightEvent::ObjDrop | mw_sim::fight::FightEvent::ObjCover) {
            eprintln!("[props] tick {} {:?} held {:?}", play.tick, e, p.held);
        }
        match e {
            mw_sim::fight::FightEvent::PickStart => {
                if let Some(i) = p.pick_target(pl, f) { p.list[i].state = PState::Reaching; p.held = Some(i); }
            }
            mw_sim::fight::FightEvent::PickTake => { if let Some(i) = p.held { p.list[i].state = PState::Held; } }
            mw_sim::fight::FightEvent::ObjThrow => {
                if let Some(i) = p.held.take() {
                    let dir = (fwd * (-5f32).to_radians().cos() + Vec3::Y * (-5f32).to_radians().sin()).normalize();
                    let o = &mut p.list[i]; o.state = PState::Flying; o.vel = dir * THROW_SPEED + Vec3::Y * 0.03; o.spin = Vec3::new(0.15, 0.05, 0.);
                }
            }
            mw_sim::fight::FightEvent::ObjDrop => {
                if let Some(i) = p.held.take() {
                    let at = pl + fwd * 0.8;
                    let y = super::floor(&arena.0, at + Vec3::Y).unwrap_or(pl.y);
                    let o = &mut p.list[i]; o.state = PState::Placed; o.pos = Vec3::new(at.x, y, at.z); o.rot = Quat::from_rotation_y(f);
                }
            }
            mw_sim::fight::FightEvent::ObjCover => {
                if let Some(i) = p.held.take() {
                    match street.cover_target(pl, f) {
                        Some(id) if street.cover(id, &mut fx) => { p.list[i].state = PState::OnHead(id); }
                        _ => { let o = &mut p.list[i]; o.state = PState::Flying; o.vel = fwd * 0.05; }
                    }
                }
            }
            _ => {}
        }
    }
    // the hold ended some other way (hit, death): it falls
    if let Some(i) = p.held { if play.fight.carry.is_none() { p.held = None; let o = &mut p.list[i]; o.state = PState::Flying; o.vel = Vec3::ZERO; } }
    for o in p.list.iter_mut() {
        match o.state {
            PState::Held | PState::Reaching => {
                // carried in both hands, chest high in front of him
                if o.state == PState::Held { o.pos = pl + Quat::from_rotation_y(f) * Vec3::new(0., 1.45, 0.3); o.rot = Quat::from_rotation_y(f); }
            }
            PState::Flying => {
                o.vel.y -= GRAVITY;
                o.pos += o.vel;
                o.rot = Quat::from_scaled_axis(o.spin) * o.rot;
                // OBJ_THROW on the first grunt it meets: he staggers, it breaks
                if o.vel.length() > BREAK_SPEED && street.obj_hit(o.pos, o.vel, &mut fx) {
                    fx.impact(o.pos + Vec3::Y * 0.3, o.vel.x.atan2(o.vel.z), 1.2, [0.9, 0.85, 0.7]);
                    o.state = PState::Gone; continue;
                }
                let floor = super::floor(&arena.0, o.pos + Vec3::Y * 0.5).unwrap_or(0.);
                if o.pos.y <= floor {
                    o.pos.y = floor;
                    if o.vel.length() > BREAK_SPEED { fx.impact(o.pos + Vec3::Y * 0.3, 0., 1.0, [0.9, 0.85, 0.7]); o.state = PState::Gone; }
                    else { o.state = PState::Placed; o.vel = Vec3::ZERO; o.rot = Quat::from_rotation_y(o.rot.to_euler(EulerRot::YXZ).0); }
                }
            }
            PState::OnHead(id) => {
                match street.covered_head(id) {
                    Some((head, yaw)) => { o.pos = head; o.rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(std::f32::consts::PI); }
                    // shaken off (or he died): it drops
                    None => { o.state = PState::Flying; o.vel = Vec3::new(0., 0.05, 0.); o.rot = Quat::from_rotation_y(o.rot.to_euler(EulerRot::YXZ).0); }
                }
            }
            _ => {}
        }
    }

}

/// [added] VR: objects picked up, carried and thrown by hand (play_vr_interact.rs).
impl Props {
    /// The nearest pickable object within reach of a hand (placed, or lying / tumbling slowly), taken in the hand where
    /// it was grabbed.
    pub fn vr_grab(&mut self, hand: &Transform) -> Option<usize> {
        if !self.on { return None; }
        let at = hand.translation;
        let i = self.list.iter().enumerate()
            .filter(|(_, p)| p.pickable && (matches!(p.state, PState::Placed) || p.state == PState::Flying && p.vel.length() < 0.05))
            .map(|(i, p)| (i, (p.pos + Vec3::Y * 0.28).distance(at)))
            .filter(|(_, d)| *d < 0.48).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)?;
        if self.held == Some(i) { return None; }
        let inv = hand.rotation.inverse();
        let o = &mut self.list[i];
        o.state = PState::VrHeld(inv * (o.pos - at), inv * o.rot);
        o.vel = Vec3::ZERO; o.spin = Vec3::ZERO;
        Some(i)
    }
    /// Carries object `i` in the hand; false if it is no longer held.
    pub fn vr_hold(&mut self, i: usize, hand: &Transform) -> bool {
        let Some(o) = self.list.get_mut(i) else { return false; };
        let PState::VrHeld(off, rot) = o.state else { return false; };
        o.pos = hand.translation + hand.rotation * off; o.rot = hand.rotation * rot;
        true
    }
    /// Lets object `i` go with the hand's velocity (m/s) and spin (rad/s): it flies (breaking on a grunt or the floor
    /// when fast) or drops.
    pub fn vr_release(&mut self, i: usize, vel: Vec3, spin: Vec3) {
        let Some(o) = self.list.get_mut(i) else { return; };
        if !matches!(o.state, PState::VrHeld(..)) { return; }
        o.state = PState::Flying; o.vel = vel / 30.; o.spin = (spin / 30.).clamp_length_max(0.4);
    }
    /// Flying objects near a point (for hits on RinRin and the bosses): index and velocity per frame.
    pub fn vr_flying(&self) -> Vec<(usize, Vec3, Vec3)> {
        self.list.iter().enumerate().filter(|(_, p)| p.state == PState::Flying && p.vel.length() > BREAK_SPEED).map(|(i, p)| (i, p.pos, p.vel)).collect()
    }
    /// Object `i` breaks (it struck someone).
    pub fn vr_break(&mut self, i: usize) { if let Some(o) = self.list.get_mut(i) { o.state = PState::Gone; } }
}
