#![allow(dead_code)]   // (the per-hand arm models gave way to the full body, play_vr_body.rs)
//! Kreely's own arms in VR [added]: his rigged model (kreely-source / kreely-color.glb), one instance per hand, posed
//! so that only that forearm and hand show — the wrist on the controller, the elbow by a two-bone reach from his
//! shoulder, every other bone folded into the elbow — and his fingers between the rig's own hand poses (kreely-rig.json
//! `handPoses`: open, fist, gun) by the grip, the trigger and the thumb. Also the pistol (the ev93 wp0671 model) in
//! the gun hand or holstered at the hip, and the comfort vignette.
//!
//! The skeleton binds in a T pose with identity rotations: the right arm (shoulder 7, elbow 8, wrist 9, fingers
//! 1568-1586, the gun prop 1600) runs along -X, the left (12, 13, 14, fingers 1536-1554, prop 1601) along +X, palms down,
//! thumbs forward (+Z). The OpenXR grip frame has the fingers forward (-Z), the thumb up (+Y), the right palm facing -X.
use super::{interact::Interact, Rig, VrInput, VrSettings};
use bevy::{camera::visibility::RenderLayers, prelude::*};

/// One of Kreely's arm models: the controller it follows (0 left, 1 right), source or colour, its joints by bone index
/// (and the scene's -1 origin last).
#[derive(Component)] pub struct ArmModel { hand: usize, colored: bool, joints: Option<Vec<Entity>> }
#[derive(Component)] pub struct GunModel(bool);
impl GunModel { pub fn is_colored(&self) -> bool { self.0 } }
#[derive(Component)] pub struct Vignette;

/// Per hand (0 left, 1 right): Kreely's elbow and wrist bone ids (his shoulders 12 / 7 fold into the elbow), his fingers (thumb, index, the rest).
struct Side { elbow: i16, wrist: i16, thumb: [i16; 3], index: [i16; 3], rest: [i16; 9] }
const SIDES: [Side; 2] = [
    Side { elbow: 13, wrist: 14, thumb: [1536, 1537, 1538], index: [1540, 1541, 1542], rest: [1544, 1545, 1546, 1548, 1549, 1550, 1552, 1553, 1554] },
    Side { elbow: 8, wrist: 9, thumb: [1568, 1569, 1570], index: [1572, 1573, 1574], rest: [1576, 1577, 1578, 1580, 1581, 1582, 1584, 1585, 1586] },
];
/// His upper arm and forearm (m, bind: 7->8 0.332, 8->9 0.299).
const UPPER: f32 = 0.332;
const FORE: f32 = 0.299;
/// How small the folded bones are drawn (a point at the elbow).
const FOLD: f32 = 1e-3;

/// The model's frame turned into the grip frame, per hand: his fingers (right -X / left +X) forward (-Z), his thumb
/// (+Z) up (+Y), his palm (-Y) inward.
fn bind_to_grip(hand: usize) -> Quat {
    if hand == 1 { Quat::from_mat3(&Mat3::from_cols(Vec3::Z, Vec3::X, Vec3::Y)) } else { Quat::from_mat3(&Mat3::from_cols(Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y)) }
}
/// Along his fingers in his bind frame.
fn finger_dir(hand: usize) -> Vec3 { if hand == 1 { Vec3::NEG_X } else { Vec3::X } }

/// Where the gun sits on the hand: at the grip, turned along the aim; the barrel's muzzle in that frame.
const MUZZLE: Vec3 = Vec3::new(0., 0.105, -0.20);
/// The gun's origin in the hand frame: the middle of its grip (model x 0.11, z -0.04) in the palm.
const GUN_OFF: Vec3 = Vec3::new(0., 0.04, -0.11);
/// The holster at the gun-hand hip (body frame: side, height over the feet, forward).
pub const HOLSTER: Vec3 = Vec3::new(0.27, 1.0, -0.04);

/// The pistol model's own frame turned into the hand's: wp0671 lies along X (the muzzle at x -0.09, the grip at the
/// rear, x 0.07-0.14), its height along Z (the slide's top at z +0.09, the butt at z -0.09), its width along Y [read
/// from the GLB's vertices, side on]: model -X goes forward (-Z), +Z up (+Y), +Y to the right (+X).
/// `MW_VR_GUN_ROT=x,y,z` (degrees) overrides.
fn gun_rot() -> Quat {
    static R: std::sync::OnceLock<Quat> = std::sync::OnceLock::new();
    *R.get_or_init(|| {
        if let Some(v) = std::env::var("MW_VR_GUN_ROT").ok().map(|s| s.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 3) {
            return Quat::from_euler(EulerRot::YXZ, v[1].to_radians(), v[0].to_radians(), v[2].to_radians());
        }
        Quat::from_mat3(&Mat3::from_cols(Vec3::Z, Vec3::X, Vec3::Y))
    })
}

/// The gun's frame in the world for a hand: at the grip, along the hand.
/// (In line with Kreely's hand: at the palm, turned like the controller's pointing ray — play_vr.rs `Hand::hand_pose` —
/// so the barrel points where the controller points.)
pub fn gun_frame(grip: &Transform, _aim: &Transform) -> Transform {
    let pitch = f32::from_bits(GUN_PITCH.load(std::sync::atomic::Ordering::Relaxed));
    Transform { translation: grip.translation, rotation: grip.rotation * Quat::from_rotation_x(pitch), scale: Vec3::ONE }
}
/// The "Pistol angle" setting (radians, + raises the barrel), kept where gun_frame reads it.
static GUN_PITCH: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub fn set_gun_pitch(deg: f32) { GUN_PITCH.store(deg.to_radians().to_bits(), std::sync::atomic::Ordering::Relaxed); }
/// The muzzle and the barrel's direction for a gun frame.
pub fn muzzle_of(frame: &Transform) -> (Vec3, Vec3) { (frame.transform_point(MUZZLE), (frame.rotation * Vec3::NEG_Z).normalize()) }
/// The muzzle for the gun hand's grip pose (its aim taken as the grip's own when only the grip is at hand).
pub fn muzzle(grip: &Transform) -> (Vec3, Vec3) { muzzle_of(&gun_frame(grip, grip)) }
/// The holster's world position for a body at `feet` facing `yaw` (game convention), on the `hand` side.
pub fn holster(feet: Vec3, yaw: f32, hand: usize) -> Vec3 {
    let side = if hand == 0 { -1. } else { 1. };
    let r = Quat::from_rotation_y(yaw);
    // (+Z forward; the right side is -X in the game's frame)
    feet + r * Vec3::new(-side * HOLSTER.x, HOLSTER.y, HOLSTER.z)
}

/// The gun model's root for a hand's gun frame, the recoil lifting the barrel.
pub fn gun_root(frame: &Transform, recoil: f32) -> Transform {
    let kick = Quat::from_rotation_x(recoil * 0.35);
    Transform { translation: frame.translation + frame.rotation * GUN_OFF, rotation: frame.rotation * kick * gun_rot(), scale: Vec3::ONE }
}
/// The gun model's root holstered: barrel down along the thigh.
pub fn holster_root(body: Vec3, yaw: f32, hand: usize) -> Transform {
    let frame = Transform::from_translation(holster(body, yaw, hand)).with_rotation(Quat::from_rotation_y(yaw + std::f32::consts::PI) * Quat::from_rotation_x(-1.45));
    gun_root(&frame, 0.)
}

pub fn build(app: &mut App) {
    app.add_systems(Startup, spawn.after(super::setup));
    let _ = bind;
}

fn spawn(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>, assets: Res<AssetServer>) {
    // the pistol, source and colour
    for (colored, file) in [(false, "native/events/ev93/wp0671.glb"), (true, "native/events/ev93/wp0671-color.glb")] {
        commands.spawn((GunModel(colored), super::SelfView, super::super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))), Transform::default(), Visibility::Hidden));
    }
    // the vignette: clear in the middle, dark at the edge, a head-locked quad just before the eyes
    let n = 128u32;
    let mut px = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n { for x in 0..n {
        let d = Vec2::new(x as f32 + 0.5 - n as f32 * 0.5, y as f32 + 0.5 - n as f32 * 0.5).length() / (n as f32 * 0.5);
        let a = ((d - 0.42) / 0.5).clamp(0., 1.); let a = a * a * (3. - 2. * a);
        px.extend_from_slice(&[0, 0, 0, (a * 255.) as u8]);
    } }
    let img = images.add(Image::new(bevy::render::render_resource::Extent3d { width: n, height: n, depth_or_array_layers: 1 }, bevy::render::render_resource::TextureDimension::D2, px,
                                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::RENDER_WORLD));
    let mat = materials.add(StandardMaterial { base_color: Color::srgba(1., 1., 1., 0.), base_color_texture: Some(img), unlit: true, alpha_mode: AlphaMode::Blend,
                                               cull_mode: None, depth_bias: 1000., fog_enabled: false, ..default() });
    commands.spawn((Vignette, Mesh3d(meshes.add(Rectangle::new(1., 1.))), MeshMaterial3d(mat), Transform::default(), Visibility::Hidden,
                    RenderLayers::layer(super::L_EYES), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
}

/// Update: each arm model's joints (role_0_bone<id>, in the rig's bone order, then the -1 origin) once its scene is in.
fn bind(mut arms: Query<(Entity, &mut ArmModel)>, children: Query<&Children>, names: Query<&Name>, play: Res<super::super::Play>) {
    for (root, mut arm) in &mut arms {
        if arm.joints.is_some() { continue; }
        let mut found = std::collections::HashMap::new();
        for d in children.iter_descendants(root) { if let Ok(n) = names.get(d) { if n.as_str().starts_with("role_0_bone") { found.insert(n.as_str().to_string(), d); } } }
        let ids = play.kreely_bones.iter().map(|b| b.id).chain(std::iter::once(-1));
        let joints: Option<Vec<Entity>> = ids.map(|id| found.get(&format!("role_0_bone{id}")).copied()).collect();
        if joints.is_some() { arm.joints = joints; }
    }
}

/// The rig's finger rotations for pose `name` (bone id -> local rotation).
fn hand_pose(play: &super::super::Play, name: &str) -> std::collections::HashMap<i16, Quat> {
    play.kreely.hand_pose(Some(name)).into_iter().map(|(id, q)| (id, Quat::from_array(q).normalize())).collect()
}

/// The world pose of every bone of the arm model for `hand` (the controller's grip), and his fingers' curls
/// (thumb, index, the rest; 0 open .. 1 a fist), `gun` holding the pistol. Returns local transforms by bone index.
fn arm_pose(play: &super::super::Play, hand: usize, grip: &Transform, head: &Transform, curl: [f32; 3], gun: bool,
            poses: &[std::collections::HashMap<i16, Quat>; 3]) -> Vec<Transform> {
    let bones = &play.kreely_bones;
    let side = &SIDES[hand];
    let idx = |id: i16| bones.iter().position(|b| b.id == id);
    // the wrist on the controller: his palm round the handle, the back of the hand a little out
    let wr = (grip.rotation * bind_to_grip(hand)).normalize();
    let out = if hand == 1 { 0.022 } else { -0.022 };
    let wp = grip.translation + grip.rotation * Vec3::new(out, -0.005, 0.07);
    // the elbow: a two-bone reach from his shoulder (beside and below the head, a little back), pointing down and out
    let yaw = super::game_yaw(head.rotation);
    let body = Quat::from_rotation_y(yaw + std::f32::consts::PI);
    let sx = if hand == 1 { 1. } else { -1. };
    let shoulder = head.translation + body * Vec3::new(sx * 0.21, -0.3, 0.1);
    let d = wp - shoulder;
    let len = d.length().clamp(0.05, UPPER + FORE - 1e-3);
    let dn = d.normalize_or(Vec3::NEG_Y);
    let along = (UPPER * UPPER - FORE * FORE + len * len) / (2. * len);
    let h = (UPPER * UPPER - along * along).max(0.).sqrt();
    let pole = body * Vec3::new(sx * 0.5, -1., 0.35);
    let perp = (pole - dn * pole.dot(dn)).normalize_or(Vec3::NEG_Y);
    let ep = shoulder + dn * along + perp * h;
    // the forearm from the elbow to the wrist, keeping the wrist's twist
    let er = (Quat::from_rotation_arc((wr * finger_dir(hand)).normalize(), (wp - ep).normalize_or(wr * finger_dir(hand))) * wr).normalize();
    let fold = Mat4::from_scale_rotation_translation(Vec3::splat(FOLD), er, ep);
    // world matrices: the elbow and the wrist kept, the fingers by their poses (local), all else folded into the elbow
    let mut world: Vec<Mat4> = vec![fold; bones.len()];
    let mut local: Vec<Option<Transform>> = vec![None; bones.len()];
    let (Some(ei), Some(wi)) = (idx(side.elbow), idx(side.wrist)) else { return vec![Transform::IDENTITY; bones.len()]; };
    world[ei] = Mat4::from_rotation_translation(er, ep);
    world[wi] = Mat4::from_rotation_translation(wr, wp);
    let [open, fist, gunpose] = poses;
    let finger = |id: i16, k: f32| -> Quat {
        let a = open.get(&id).copied().unwrap_or(Quat::IDENTITY);
        let b = if gun { gunpose.get(&id).copied().unwrap_or(Quat::IDENTITY) } else { fist.get(&id).copied().unwrap_or(Quat::IDENTITY) };
        a.slerp(b, k.clamp(0., 1.))
    };
    let groups: [(&[i16], f32); 3] = [(&side.thumb, curl[0]), (&side.index, curl[1]), (&side.rest, curl[2])];
    for (ids, k) in groups {
        for id in ids { if let Some(i) = idx(*id) { local[i] = Some(Transform { translation: Vec3::from_array(bones[i].relative), rotation: finger(*id, k), scale: Vec3::ONE }); } }
    }
    // to locals, parents first (the rig lists parents before children)
    let mut out = vec![Transform::IDENTITY; bones.len()];
    for i in 0..bones.len() {
        let p = bones[i].parent;
        let parent_world = if p >= 0 { world[p as usize] } else { Mat4::IDENTITY };
        match local[i] {
            Some(t) => { out[i] = t; world[i] = parent_world * t.to_matrix(); }
            None => {
                let m = parent_world.inverse() * world[i];
                let (s, r, t) = m.to_scale_rotation_translation();
                out[i] = Transform { translation: t, rotation: r.normalize(), scale: s };
            }
        }
    }
    out
}

/// Update: Kreely's arms on the controllers, their fingers, the pistol and the vignette.
#[allow(clippy::too_many_arguments)]
pub fn present(rig: Res<Rig>, inp: Res<VrInput>, act: Res<Interact>, set: Res<VrSettings>, view: Res<super::View>, play: Res<super::super::Play>,
               mut arms: Query<(&ArmModel, &mut Transform, &mut Visibility), (Without<GunModel>, Without<Vignette>)>,
               mut joints: Query<&mut Transform, (Without<ArmModel>, Without<GunModel>, Without<Vignette>)>,
               mut guns: Query<(&GunModel, &mut Transform, &mut Visibility), (Without<ArmModel>, Without<Vignette>)>,
               vignette: Query<(&mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>), (With<Vignette>, Without<ArmModel>, Without<GunModel>)>,
               mut materials: ResMut<Assets<StandardMaterial>>, time: Res<Time<Real>>, fixed: Res<Time<Fixed>>, mut curl: Local<[[f32; 3]; 2]>,
               mut poses: Local<Option<[std::collections::HashMap<i16, Quat>; 3]>>) {
    if !rig.started { return; }
    let show = !view.cinema;
    let _g = set.main_hand();
    let dt = time.delta_secs();
    if poses.is_none() { *poses = Some([hand_pose(&play, "open"), hand_pose(&play, "fist"), hand_pose(&play, "gun")]); }
    let poses = poses.as_ref().unwrap();
    // what each hand's fingers want: the grip closes the lower three, the trigger the index (resting on it: a little), the
    // thumb closes on a touch; holding something, the hand is a fist round it; the gun hand grips the pistol
    for h in 0..2 {
        let hand = &inp.hands[h];
        let holding = act.holding(h);
        let gun = h == act.gun_hand && act.gun_drawn;
        let grip = if holding || gun { 1. } else { hand.grip };
        let index = if gun { 0.15 + hand.trigger * 0.6 } else if holding { 1. } else { hand.trigger.max(if hand.trigger_touch { 0.2 } else { 0. }) };
        let thumb = if holding || gun || hand.grip > 0.5 { 1. } else if hand.thumb_touch { 0.55 } else { 0.05 };
        let want = [thumb, index, grip];
        for f in 0..3 { let k = (dt * 22.).min(1.); curl[h][f] += (want[f] - curl[h][f]) * k; }
    }
    let _ = (&mut arms, &mut joints, &mut curl, &poses, &inp);
    // (the pistol itself: play_vr_gun.rs present)
    let _ = (&mut guns, &fixed, show);
    // the vignette
    for (mut t, mut v, m) in vignette {
        let a = rig.vignette;
        *v = if a > 0.02 { Visibility::Visible } else { Visibility::Hidden };
        let fwd = rig.head.rotation * Vec3::NEG_Z;
        *t = Transform { translation: rig.head.translation + fwd * 0.16, rotation: rig.head.rotation, scale: Vec3::splat(0.62) };
        let want = Color::srgba(1., 1., 1., a.min(1.));
        if materials.get(&m.0).is_some_and(|x| (x.base_color.alpha() - a.min(1.)).abs() > 0.01) { if let Some(mut mat) = materials.get_mut(&m.0) { mat.base_color = want; } }
    }
}
