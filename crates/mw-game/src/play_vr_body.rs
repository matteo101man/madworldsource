//! Kreely's whole body in VR [added]: one instance of his rigged model (kreely-source / kreely-color.glb) that only the
//! eyes see (the flat camera keeps his native, animated body), posed every frame from the headset and the controllers
//! so it feels like being him:
//! - his feet stand on the floor under the body and step after it when it moves or turns;
//! - his pelvis sinks as your head does (crouching), his knees bend forward over planted feet (two-bone leg IK);
//! - his spine bends from the pelvis toward your neck (leaning, bending over) and twists part way with your head;
//! - his arms reach from his shoulders to the controllers (two-bone IK, elbows down and out), the hands on the grips,
//!   the fingers between the rig's own open / fist / gun poses by the grip, the trigger and the thumb;
//! - his head folds away into his neck, so it never fills the view.
//!
//! The skeleton binds in a T pose with identity rotations (kreely-rig.json): pelvis 0 / 1 / 15 at 1.065 m, spine 2,
//! chest 3, neck 4 (head 24-26, eyes 40 / 41 at 1.72 m); right arm 5 > 7 > 8 > 9 along -X, left 10 > 12 > 13 > 14 along +X;
//! right leg 16 > 17 > 18 > 19 at x -0.12, left 20 > 21 > 22 > 23; he faces +Z.
use super::{interact::Interact, Rig, VrInput, VrSettings};
use bevy::{camera::visibility::RenderLayers, prelude::*};
use std::collections::HashMap;

#[derive(Component)] pub struct BodyModel { colored: bool, joints: Option<Vec<Entity>>, tagged: bool, headless: bool }

/// The folded bones' size (a point).
const FOLD: f32 = 1e-3;
/// A foot steps when it is this far (m) from where it should stand, over this many seconds, lifted this high.
const STEP_AT: f32 = 0.24;
const STEP_TIME: f32 = 0.2;
const STEP_LIFT: f32 = 0.11;

/// The model frame turned into the grip frame, per hand (0 left, 1 right): his fingers (right -X / left +X) forward
/// (-Z), his thumb (+Z) up (+Y), his palm (-Y) inward.
fn bind_to_grip(hand: usize) -> Quat {
    if hand == 1 { Quat::from_mat3(&Mat3::from_cols(Vec3::Z, Vec3::X, Vec3::Y)) } else { Quat::from_mat3(&Mat3::from_cols(Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y)) }
}

/// Per side (0 left, 1 right): clavicle, twist, shoulder, elbow, wrist; hip, knee, ankle, toe; fingers (thumb, index, rest).
struct Side { clav: i16, twist: i16, shoulder: i16, elbow: i16, wrist: i16, hip: i16, knee: i16, ankle: i16, toe: i16,
              thumb: [i16; 3], index: [i16; 3], rest: [i16; 9] }
const SIDES: [Side; 2] = [
    Side { clav: 10, twist: 11, shoulder: 12, elbow: 13, wrist: 14, hip: 20, knee: 21, ankle: 22, toe: 23,
           thumb: [1536, 1537, 1538], index: [1540, 1541, 1542], rest: [1544, 1545, 1546, 1548, 1549, 1550, 1552, 1553, 1554] },
    Side { clav: 5, twist: 6, shoulder: 7, elbow: 8, wrist: 9, hip: 16, knee: 17, ankle: 18, toe: 19,
           thumb: [1568, 1569, 1570], index: [1572, 1573, 1574], rest: [1576, 1577, 1578, 1580, 1581, 1582, 1584, 1585, 1586] },
];
const EYE: Vec3 = Vec3::new(0., 1.72, 0.064);
/// The body sits this far (m) behind the head (as VR bodies do), so looking down shows the belly, legs and feet
/// rather than the collar under the eyes.
const BACK: f32 = 0.10;

/// The rig in a form for posing: bone index by id, parents, bind positions (model space).
pub struct Skel { idx: HashMap<i16, usize>, parent: Vec<i16>, bind: Vec<Vec3>, rel: Vec<Vec3> }
impl Skel {
    fn new(bones: &[mw_data::Bone]) -> Self {
        let mut bind = vec![Vec3::ZERO; bones.len()];
        for i in 0..bones.len() {
            let p = bones[i].parent;
            bind[i] = Vec3::from_array(bones[i].relative) + if p >= 0 { bind[p as usize] } else { Vec3::ZERO };
        }
        Skel { idx: bones.iter().enumerate().map(|(i, b)| (b.id, i)).collect(), parent: bones.iter().map(|b| b.parent).collect(), bind,
               rel: bones.iter().map(|b| Vec3::from_array(b.relative)).collect() }
    }
    fn b(&self, id: i16) -> Vec3 { self.idx.get(&id).map_or(Vec3::ZERO, |&i| self.bind[i]) }
}

/// Two-bone reach from `a` to `c` with segment lengths `l1`, `l2`, the middle joint bending toward `pole`.
fn two_bone(a: Vec3, c: Vec3, l1: f32, l2: f32, pole: Vec3) -> Vec3 {
    let d = c - a;
    let len = d.length().clamp(0.02, l1 + l2 - 1e-3);
    let dn = d.normalize_or(Vec3::NEG_Y);
    let along = (l1 * l1 - l2 * l2 + len * len) / (2. * len);
    let h = (l1 * l1 - along * along).max(0.).sqrt();
    let perp = (pole - dn * pole.dot(dn)).normalize_or(Vec3::Z);
    a + dn * along + perp * h
}
/// The rotation that turns `base`'s copy of the bind direction `bind_dir` onto `want`, applied over `base`.
fn aim(base: Quat, bind_dir: Vec3, want: Vec3) -> Quat {
    (Quat::from_rotation_arc((base * bind_dir).normalize_or(Vec3::Y), want.normalize_or(base * bind_dir)) * base).normalize()
}

/// What the body is posed from this frame.
pub struct Drive<'a> {
    /// the floor point under the body, its yaw (game convention), the standing eye height over the floor
    feet: Vec3, yaw: f32, stand: f32,
    head: Transform, grip: [Transform; 2], curl: [[f32; 3]; 2], gun: Option<usize>, ankles: [Vec3; 2],
    poses: &'a [HashMap<i16, Quat>; 3],
}

/// Every bone's local transform (by bone index) for the drive.
fn body_pose(sk: &Skel, d: &Drive) -> Vec<Transform> {
    let n = sk.bind.len();
    let b = Quat::from_rotation_y(d.yaw);
    let mut wr: Vec<Quat> = vec![b; n];
    let mut wp: Vec<Vec3> = vec![Vec3::ZERO; n];
    let mut fold: Vec<bool> = vec![false; n];
    let mut local: Vec<Option<Transform>> = vec![None; n];
    let i = |id: i16| sk.idx[&id];
    // the head in the model's frame (the headset looks down -Z, the model faces +Z)
    let head_rot = d.head.rotation * Quat::from_rotation_y(std::f32::consts::PI);
    let eye = d.head.translation;
    let fwd0 = Quat::from_rotation_y(d.yaw) * Vec3::Z;
    let neck = eye + head_rot * (sk.b(4) - EYE) - fwd0 * BACK;
    // crouching: the head this much under its standing height sinks the pelvis (and sits it back a little)
    // (the first 6 cm are a slouch, not a crouch: standing tall keeps his legs straight)
    let drop = (d.stand - (eye.y - d.feet.y) - 0.06).clamp(0., 0.95);
    let fwd = b * Vec3::Z;
    let lean = Vec3::new(neck.x - d.feet.x, 0., neck.z - d.feet.z);
    let pelvis = d.feet + Vec3::Y * (sk.b(0).y - drop * 0.92) + lean * 0.3 - fwd * (drop * 0.28);
    for id in [0i16, 1, 15] { let k = i(id); wp[k] = pelvis + b * (sk.b(id) - sk.b(0)); wr[k] = b; }
    // the spine bends from bone 2 toward the neck, half in the spine, the rest in the chest, twisting with the head
    let twist = { let hy = super::game_yaw(d.head.rotation); ((hy - d.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).clamp(-0.8, 0.8) };
    let p2 = wp[i(1)] + b * (sk.b(2) - sk.b(1));
    let bend = Quat::from_rotation_arc((b * (sk.b(4) - sk.b(2))).normalize(), (neck - p2).normalize_or(Vec3::Y));
    let r2 = (Quat::IDENTITY.slerp(bend, 0.5) * Quat::from_rotation_y(twist * 0.3) * b).normalize();
    let r3 = (bend * Quat::from_rotation_y(twist * 0.7) * b).normalize();
    wp[i(2)] = p2; wr[i(2)] = r2;
    wp[i(3)] = p2 + r2 * (sk.b(3) - sk.b(2)); wr[i(3)] = r3;
    wp[i(4)] = wp[i(3)] + r3 * (sk.b(4) - sk.b(3)); wr[i(4)] = r3;
    // the chest's own bones (50-52) ride the chest
    for id in [50i16, 51, 52] { if let Some(&k) = sk.idx.get(&id) { wp[k] = wp[i(3)] + r3 * (sk.b(id) - sk.b(3)); wr[k] = r3; } }
    // the head folds into the neck
    for id in [4i16, 24, 25, 26, 40, 41] { if let Some(&k) = sk.idx.get(&id) { wp[k] = wp[i(4)]; wr[k] = r3; fold[k] = true; } }
    let [open, fist, gunpose] = d.poses;
    for (h, s) in SIDES.iter().enumerate() {
        // the arm: clavicle and shoulder on the chest, a two-bone reach to the wrist on the controller
        let c = i(s.clav);
        wp[c] = wp[i(3)] + r3 * (sk.b(s.clav) - sk.b(3)); wr[c] = r3;
        let sh = wp[c] + r3 * (sk.b(s.shoulder) - sk.b(s.clav));
        let grip = d.grip[h];
        let wrot = (grip.rotation * bind_to_grip(h)).normalize();
        let out = if h == 1 { 0.022 } else { -0.022 };
        let wpos = grip.translation + grip.rotation * Vec3::new(out, -0.005, 0.07);
        let (l1, l2) = ((sk.b(s.elbow) - sk.b(s.shoulder)).length(), (sk.b(s.wrist) - sk.b(s.elbow)).length());
        let side = if h == 1 { -1. } else { 1. };   // (model -X is his right)
        let elbow = two_bone(sh, wpos, l1, l2, r3 * Vec3::new(side * 0.6, -1., -0.35));
        let upper = aim(r3, sk.b(s.elbow) - sk.b(s.shoulder), elbow - sh);
        for id in [s.shoulder, s.twist] { let k = i(id); wp[k] = sh; wr[k] = upper; }
        let fdir = sk.b(s.wrist) - sk.b(s.elbow);
        let fore = (Quat::from_rotation_arc((wrot * fdir).normalize(), (wpos - elbow).normalize_or(wrot * fdir.normalize())) * wrot).normalize();
        wp[i(s.elbow)] = elbow; wr[i(s.elbow)] = fore;
        wp[i(s.wrist)] = wpos; wr[i(s.wrist)] = wrot;
        // the fingers: local, between the rig's poses
        let gun = d.gun == Some(h);
        let finger = |id: i16, k: f32| -> Quat {
            let a = open.get(&id).copied().unwrap_or(Quat::IDENTITY);
            let z = if gun { gunpose.get(&id).copied().unwrap_or(Quat::IDENTITY) } else { fist.get(&id).copied().unwrap_or(Quat::IDENTITY) };
            a.slerp(z, k.clamp(0., 1.))
        };
        let groups: [(&[i16], f32); 3] = [(&s.thumb, d.curl[h][0]), (&s.index, d.curl[h][1]), (&s.rest, d.curl[h][2])];
        for (ids, k) in groups { for id in ids { if let Some(&j) = sk.idx.get(id) { local[j] = Some(Transform { translation: sk.rel[j], rotation: finger(*id, k), scale: Vec3::ONE }); } } }
        // the leg: hip on the pelvis, a two-bone reach to the planted ankle, the knee forward (a little outward)
        let hip = wp[i(15)] + b * (sk.b(s.hip) - sk.b(15));
        let ankle = d.ankles[h];
        let (t1, t2) = ((sk.b(s.knee) - sk.b(s.hip)).length(), (sk.b(s.ankle) - sk.b(s.knee)).length());
        let knee = two_bone(hip, ankle, t1, t2, b * Vec3::new(side * 0.15, 0., 1.));
        let thigh = aim(b, sk.b(s.knee) - sk.b(s.hip), knee - hip);
        let shin = aim(b, sk.b(s.ankle) - sk.b(s.knee), ankle - knee);
        wp[i(s.hip)] = hip; wr[i(s.hip)] = thigh;
        wp[i(s.knee)] = knee; wr[i(s.knee)] = shin;
        wp[i(s.ankle)] = ankle; wr[i(s.ankle)] = b;
        wp[i(s.toe)] = ankle + b * (sk.b(s.toe) - sk.b(s.ankle)); wr[i(s.toe)] = b;
    }
    // everything not placed above follows its parent rigidly (the coat tails 30 / 31, the props); the pistol props fold
    let placed: Vec<bool> = (0..n).map(|k| wp[k] != Vec3::ZERO || local[k].is_some()).collect();
    for k in 0..n {
        let p = sk.parent[k];
        let prop = matches!(sk.parent.get(k).map(|_| ()), Some(_)) && sk.idx.iter().any(|(id, &j)| j == k && (*id == 1600 || *id == 1601));
        if prop { let q = p.max(0) as usize; wp[k] = wp[q]; wr[k] = wr[q]; fold[k] = true; continue; }
        if !placed[k] && local[k].is_none() && p >= 0 { let q = p as usize; wp[k] = wp[q] + wr[q] * sk.rel[k]; wr[k] = wr[q]; fold[k] = fold[q]; }
    }
    // to locals, parents first
    let world: Vec<Mat4> = (0..n).map(|k| Mat4::from_scale_rotation_translation(Vec3::splat(if fold[k] { FOLD } else { 1. }), wr[k], wp[k])).collect();
    let mut wm = world.clone();
    let mut out = vec![Transform::IDENTITY; n];
    for k in 0..n {
        let p = sk.parent[k];
        let pw = if p >= 0 { wm[p as usize] } else { Mat4::IDENTITY };
        match local[k] {
            Some(t) => { out[k] = t; wm[k] = pw * t.to_matrix(); }
            None => { let (s, r, t) = (pw.inverse() * world[k]).to_scale_rotation_translation(); out[k] = Transform { translation: t, rotation: r.normalize(), scale: s }; }
        }
    }
    out
}

pub fn build(app: &mut App) {
    app.add_systems(Startup, spawn.after(super::setup)).add_systems(Update, (bind, present).chain().in_set(super::VrActive).after(super::super::pose).after(super::rig));
}

fn spawn(mut commands: Commands, assets: Res<AssetServer>) {
    for (colored, file) in [(false, "native/kreely-source.glb"), (true, "native/kreely-color.glb")] {
        commands.spawn((BodyModel { colored, joints: None, tagged: false, headless: false }, super::super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))),
                        Transform::default(), Visibility::Hidden));
    }
}

/// Update: the body models' joints once their scenes are in; their meshes go to the eyes' layer and cast no second shadow
/// (his native body casts his).
#[allow(clippy::too_many_arguments)]
fn bind(mut bodies: Query<(Entity, &mut BodyModel)>, children: Query<&Children>, names: Query<&Name>, meshes: Query<(), With<Mesh3d>>,
        play: Res<super::super::Play>, mut commands: Commands, skinned: Query<(&Mesh3d, &bevy::mesh::skinning::SkinnedMesh)>,
        mut mesh_assets: ResMut<Assets<Mesh>>) {
    for (root, mut m) in &mut bodies {
        if m.joints.is_none() {
            let mut found = HashMap::new();
            for d in children.iter_descendants(root) { if let Ok(n) = names.get(d) { if n.as_str().starts_with("role_0_bone") { found.insert(n.as_str().to_string(), d); } } }
            let ids = play.kreely_bones.iter().map(|b| b.id).chain(std::iter::once(-1));
            m.joints = ids.map(|id| found.get(&format!("role_0_bone{id}")).copied()).collect();
        }
        if m.joints.is_some() && !m.tagged {
            let mut any = false;
            for d in children.iter_descendants(root) {
                if meshes.contains(d) { any = true; commands.entity(d).try_insert((RenderLayers::layer(super::L_SELF), bevy::light::NotShadowCaster)); }
            }
            m.tagged = any;
        }
        // his head and face cut out of the eyes' copy: every triangle on a vertex weighted a quarter or more to the neck,
        // head or eye bones (4, 24-26, 40, 41) goes
        if m.tagged && !m.headless {
            let mut done = true;
            for d in children.iter_descendants(root) {
                let Ok((mesh3d, skin)) = skinned.get(d) else { continue; };
                let head: Vec<u16> = skin.joints.iter().enumerate().filter(|(_, e)| names.get(**e).is_ok_and(|n| {
                    n.as_str().strip_prefix("role_0_bone").and_then(|v| v.parse::<i16>().ok()).is_some_and(|id| matches!(id, 4 | 24 | 25 | 26 | 40 | 41))
                })).map(|(k, _)| k as u16).collect();
                let Some(src) = mesh_assets.get(&mesh3d.0) else { done = false; continue; };
                let (Some(Ok(joints)), Some(Ok(weights))) = (src.try_attribute_option(Mesh::ATTRIBUTE_JOINT_INDEX).ok().map(|a| match a {
                        Some(bevy::mesh::VertexAttributeValues::Uint16x4(v)) => Ok(v.clone()), _ => Err(()) }),
                    src.try_attribute_option(Mesh::ATTRIBUTE_JOINT_WEIGHT).ok().map(|a| match a {
                        Some(bevy::mesh::VertexAttributeValues::Float32x4(v)) => Ok(v.clone()), _ => Err(()) })) else { continue; };
                let on_head: Vec<bool> = joints.iter().zip(&weights).map(|(j, w)| (0..4).filter(|k| head.contains(&j[*k])).map(|k| w[k]).sum::<f32>() >= 0.25).collect();
                let Some(idx) = src.try_indices_option().ok().flatten() else { continue; };
                let tris: Vec<u32> = idx.iter().map(|i| i as u32).collect::<Vec<_>>().chunks(3)
                    .filter(|t| t.len() == 3 && !t.iter().any(|v| on_head.get(*v as usize).copied().unwrap_or(false))).flatten().copied().collect();
                let cut = idx.len() - tris.len();
                let mut copy = src.clone();
                copy.insert_indices(bevy::mesh::Indices::U32(tris));
                let h = mesh_assets.add(copy);
                commands.entity(d).try_insert(Mesh3d(h));
                if std::env::var_os("MW_VR_LOG").is_some() { eprintln!("[vr] body: head and face cut from the eyes' model ({} triangles)", cut / 3); }
            }
            m.headless = done;
        }
    }
}

/// The feet: where each stands, and a step in flight (foot, from, to, seconds in).
#[derive(Default)]
pub struct Feet { at: Option<[Vec3; 2]>, step: Option<(usize, Vec3, Vec3, f32)>, yaw: Option<f32>, next: usize,
                  /// walking: the stride's phase (radians) and the body's last place
                  phase: f32, last: Option<Vec3>, walking: f32 }

#[allow(clippy::too_many_arguments)]
pub fn present(rig: Res<Rig>, inp: Res<VrInput>, act: Res<Interact>, set: Res<VrSettings>, view: Res<super::View>, play: Res<super::super::Play>,
               fixed: Res<Time<Fixed>>, time: Res<Time<Real>>, mut bodies: Query<(&BodyModel, &mut Transform, &mut Visibility)>,
               mut joints: Query<&mut Transform, Without<BodyModel>>, mut cache: Local<(Option<[[f32; 3]; 2]>, Feet, Option<[HashMap<i16, Quat>; 3]>, Option<Skel>)>, st: Res<super::struggle::Struggle>,
               chars: Query<&super::super::Character>, globals: Query<&GlobalTransform, Without<BodyModel>>) {
    if !rig.started { return; }
    let (curl_c, feet, poses, skel) = &mut *cache;
    let curl = curl_c.get_or_insert([[0.; 3]; 2]);
    let dt = time.delta_secs().min(0.1);
    if poses.is_none() {
        let pose = |name: &str| play.kreely.hand_pose(Some(name)).into_iter().map(|(id, q)| (id, Quat::from_array(q).normalize())).collect::<HashMap<_, _>>();
        *poses = Some([pose("open"), pose("fist"), pose("gun")]);
    }
    if skel.is_none() { *skel = Some(Skel::new(&play.kreely_bones)); }
    let (poses, sk) = (poses.as_ref().unwrap(), skel.as_ref().unwrap());
    let _g = set.main_hand();
    // the fingers: the grip closes the lower three, the trigger the index (resting on it: a little), the thumb closes on a
    // touch; holding something, a fist round it; the gun hand grips the pistol
    for h in 0..2 {
        let hand = &inp.hands[h];
        let holding = act.holding(h);
        let gun = h == act.gun_hand && act.gun_drawn;
        let grip = if holding || gun { 1. } else { hand.grip };
        let index = if gun { 0.15 + hand.trigger * 0.6 } else if holding { 1. } else { hand.trigger.max(if hand.trigger_touch { 0.2 } else { 0. }) };
        let thumb = if holding || gun || hand.grip > 0.5 { 1. } else if hand.thumb_touch { 0.55 } else { 0.05 };
        let want = [thumb, index, grip];
        for f in 0..3 { curl[h][f] += (want[f] - curl[h][f]) * (dt * 22.).min(1.); }
    }
    // the body's place: the sim player's, interpolated as his native body is
    let a = fixed.overstep_fraction();
    let body = play.prev_pos.lerp(Vec3::from_array(play.fight.player.pos), a);
    let floor = Vec3::new(rig.head.translation.x, body.y, rig.head.translation.z);
    let floor = floor - Quat::from_rotation_y(feet.yaw.unwrap_or(super::game_yaw(rig.head.rotation))) * Vec3::Z * BACK;
    // his facing eases after the head's when it turns far from it or the body moves
    let head_yaw = super::game_yaw(rig.head.rotation);
    let mut yaw = feet.yaw.unwrap_or(head_yaw);
    let off = (head_yaw - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let moving = (Vec3::from_array(play.fight.player.pos) - play.prev_pos).length() > 0.01;
    if off.abs() > 0.75 || moving { yaw += off * (dt * 5.).min(1.); }
    feet.yaw = Some(yaw);
    let b = Quat::from_rotation_y(yaw);
    // the feet stand under the hips; a foot left too far behind steps over (one at a time, alternating)
    let home = |h: usize| floor + b * Vec3::new(sk.b(SIDES[h].ankle).x, sk.b(SIDES[h].ankle).y, sk.b(SIDES[h].ankle).z + 0.2);
    let mut at = feet.at.unwrap_or([home(0), home(1)]);
    if (at[0] - home(0)).length() > 1.5 || (at[0].y - home(0).y).abs() > 0.3 { at = [home(0), home(1)]; feet.step = None; }
    let mut ankles = at;
    match feet.step {
        Some((h, from, to, t)) => {
            let t = t + dt;
            let k = (t / STEP_TIME).min(1.);
            let to = if (to - home(h)).length() > 0.05 { home(h) } else { to };
            ankles[h] = from.lerp(to, k * k * (3. - 2. * k)) + Vec3::Y * (k * std::f32::consts::PI).sin() * STEP_LIFT;
            if k >= 1. { at[h] = to; feet.step = None; feet.next = 1 - h; } else { feet.step = Some((h, from, to, t)); }
        }
        None => {
            let far: Vec<f32> = (0..2).map(|h| Vec2::new(at[h].x - home(h).x, at[h].z - home(h).z).length()).collect();
            let h = if far[feet.next] > STEP_AT { Some(feet.next) } else if far[1 - feet.next] > STEP_AT { Some(1 - feet.next) } else { None };
            if let Some(h) = h { feet.step = Some((h, at[h], home(h), 0.)); }
        }
    }
    // walking or running the feet swing in a stride cycle driven by the ground covered (a planted step cannot keep up)
    let travelled = feet.last.map_or(0., |l| Vec2::new(body.x - l.x, body.z - l.z).length());
    feet.last = Some(body);
    let speed = travelled / dt.max(1e-3);
    let target = if speed > 0.6 { 1. } else { 0. };
    feet.walking += (target - feet.walking) * (dt * 8.).min(1.);
    if feet.walking > 0.02 {
        let stride = 0.55 + (speed / 6.).min(1.) * 0.45;
        feet.phase = (feet.phase + travelled / stride * std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
        let dir = Vec3::new(body.x - floor.x, 0., body.z - floor.z);
        let mv = if travelled > 1e-4 { dir.normalize_or(b * Vec3::Z) } else { b * Vec3::Z };
        let mv = feet.last.map_or(mv, |_| mv);
        for h in 0..2 {
            let ph = feet.phase + h as f32 * std::f32::consts::PI;
            let swing = home(h) + (b * Vec3::Z).lerp(mv, 0.) * (ph.sin() * stride * 0.5) + Vec3::Y * ph.cos().max(0.) * STEP_LIFT * 1.3;
            ankles[h] = ankles[h].lerp(swing, feet.walking);
        }
        if feet.walking > 0.95 { at = [home(0), home(1)]; feet.step = None; }
    }
    feet.at = Some(at);
    // in a push his hands rest on her and go no further, however far yours do
    let mut grip = rig.grip;
    // a controller not tracked (set down, asleep, out of view): that arm hangs at his side, the hand down along the leg
    for h in 0..2 {
        if inp.hands[h].tracked { continue; }
        let side = if h == 1 { -0.24 } else { 0.24 };   // (his right is model -X)
        let turn = Quat::from_rotation_y(yaw);
        grip[h] = Transform { translation: floor + turn * Vec3::new(side, 0.86, 0.04),
                              rotation: Quat::from_rotation_y(yaw + std::f32::consts::PI) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2), scale: Vec3::ONE };
    }
    if let Some((at, dir)) = st.contact { for g in grip.iter_mut() { let over = (g.translation - at).dot(dir); if over > 0. { g.translation -= dir * over; } } }
    let pushing = st.contact.is_some();
    // pushing: open hands on her; holding her fans / hands: fists round them
    let curl = if st.gripping { [[1., 1., 1.]; 2] } else if pushing { [[0.15, 0.1, 0.1]; 2] } else { *curl };
    let drive = Drive { feet: floor, yaw, stand: super::EYE_HEIGHT, head: rig.head, grip, curl,
                        gun: (act.gun_drawn && !pushing).then_some(act.gun_hand), ankles, poses };
    // in a paired clash his body is the clip's (the native model's joints, as posed last frame), his arms yours
    let native: Option<Vec<Mat4>> = ((st.engaged || st.scene) && st.anchor.is_some()).then(|| {
        chars.iter().find(|c| c.who == super::super::Who::Kreely && c.colored == play.colored).and_then(|c| c.joints.as_ref())
            .and_then(|js| js.iter().take(sk.bind.len()).map(|e| globals.get(*e).ok().map(|g| g.to_matrix())).collect::<Option<Vec<_>>>())
    }).flatten().filter(|v| v.len() == sk.bind.len());
    let locals = match &native { Some(nw) => clash_pose(sk, nw, &drive, st.shift), None => body_pose(sk, &drive) };
    let show = !view.cinema && play.as_kreely;
    for (m, mut t, mut v) in &mut bodies {
        let want = if show && m.colored == play.colored { Visibility::Visible } else { Visibility::Hidden };
        if *v != want { *v = want; }
        if want == Visibility::Hidden { continue; }
        let Some(js) = &m.joints else { continue; };
        *t = Transform::IDENTITY;
        for (e, l) in js.iter().zip(&locals) { if let Ok(mut jt) = joints.get_mut(*e) { *jt = *l; } }
        if let Some(e) = js.last() { if let Ok(mut jt) = joints.get_mut(*e) { *jt = Transform::IDENTITY; } }
    }
}

/// In a paired clash: his body as the clip has it (`native`: every bone's world matrix from his animated model: on the
/// floor, kneeling, bent), his head folded away, his arms reaching from the clip's shoulders to your hands, the fingers
/// yours. Returns locals by bone index.
fn clash_pose(sk: &Skel, native: &[Mat4], d: &Drive, shift: Vec3) -> Vec<Transform> {
    let n = sk.bind.len();
    // (moved with his eyes when they were kept clear of her head)
    let mut world: Vec<Mat4> = native.iter().map(|m| Mat4::from_translation(shift) * *m).collect();
    let native: Vec<Mat4> = world.clone();
    let mut local: Vec<Option<Transform>> = vec![None; n];
    let i = |id: i16| sk.idx[&id];
    let neck = world[i(4)];
    for id in [4i16, 24, 25, 26, 40, 41, 1600, 1601] {
        if let Some(&k) = sk.idx.get(&id) { let (_, r, t) = neck.to_scale_rotation_translation(); world[k] = Mat4::from_scale_rotation_translation(Vec3::splat(FOLD), r, t); }
    }
    let chest_rot = native[i(3)].to_scale_rotation_translation().1;
    let [open, fist, gunpose] = d.poses;
    for (h, s) in SIDES.iter().enumerate() {
        let (_, sr, sh) = native[i(s.shoulder)].to_scale_rotation_translation();
        let grip = d.grip[h];
        let wrot = (grip.rotation * bind_to_grip(h)).normalize();
        let out = if h == 1 { 0.022 } else { -0.022 };
        let wpos = grip.translation + grip.rotation * Vec3::new(out, -0.005, 0.07);
        let (l1, l2) = ((sk.b(s.elbow) - sk.b(s.shoulder)).length(), (sk.b(s.wrist) - sk.b(s.elbow)).length());
        let side = if h == 1 { -1. } else { 1. };
        let elbow = two_bone(sh, wpos, l1, l2, chest_rot * Vec3::new(side * 0.6, -1., -0.35));
        let upper = aim(sr, sk.b(s.elbow) - sk.b(s.shoulder), elbow - sh);
        let fdir = sk.b(s.wrist) - sk.b(s.elbow);
        let fore = (Quat::from_rotation_arc((wrot * fdir).normalize(), (wpos - elbow).normalize_or(wrot * fdir.normalize())) * wrot).normalize();
        world[i(s.shoulder)] = Mat4::from_rotation_translation(upper, sh);
        world[i(s.twist)] = Mat4::from_rotation_translation(upper, sh);
        world[i(s.elbow)] = Mat4::from_rotation_translation(fore, elbow);
        world[i(s.wrist)] = Mat4::from_rotation_translation(wrot, wpos);
        let gun = d.gun == Some(h);
        let finger = |id: i16, k: f32| -> Quat {
            let a = open.get(&id).copied().unwrap_or(Quat::IDENTITY);
            let z = if gun { gunpose.get(&id).copied().unwrap_or(Quat::IDENTITY) } else { fist.get(&id).copied().unwrap_or(Quat::IDENTITY) };
            a.slerp(z, k.clamp(0., 1.))
        };
        let groups: [(&[i16], f32); 3] = [(&s.thumb, d.curl[h][0]), (&s.index, d.curl[h][1]), (&s.rest, d.curl[h][2])];
        for (ids, k) in groups { for id in ids { if let Some(&j) = sk.idx.get(id) { local[j] = Some(Transform { translation: sk.rel[j], rotation: finger(*id, k), scale: Vec3::ONE }); } } }
    }
    let mut wm = world.clone();
    let mut out = vec![Transform::IDENTITY; n];
    for k in 0..n {
        let p = sk.parent[k];
        let pw = if p >= 0 { wm[p as usize] } else { Mat4::IDENTITY };
        match local[k] {
            Some(t) => { out[k] = t; wm[k] = pw * t.to_matrix(); }
            None => { let (s, r, t) = (pw.inverse() * world[k]).to_scale_rotation_translation(); out[k] = Transform { translation: t, rotation: r.normalize(), scale: s }; }
        }
    }
    out
}
