//! Badly hurt [added, not native]: at low HP Kreely keels over, limps on his right leg (the hips sink and the body leans
//! over it on its stance) and clutches his chest with his free hand; a face hit knocks teeth out of his mouth; the
//! screen pulses red, heartbeat-like, until the HP is back above the line. None of the pose while he aims or shoots.
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat}};
use mw_data::Bone;
use mw_sim::character::LocalPose;
use mw_sim::fight::{bone_frames, FightEvent};

/// At or under this HP (of 100) he counts as badly hurt.
pub const LOW_HP: i16 = 30;
/// Enemy attacks whose flinch snaps the head (80274DC4: RinRin's punches -> 100F / 100A / 100B, the returning fan).
const FACE_HITS: [u32; 5] = [0x172, 0x173, 0x174, 0x175, 0x18C];
/// Planar speed (m per tick) that counts as a full walk.
const WALK_STEP: f32 = 0.06;

#[derive(Resource, Default)]
pub struct Hurt {
    /// 0..1 eased in and out across the HP line (Kreely only), the clutch (only while walking or standing, no gun),
    /// how much he is moving, and the red pulse (whoever plays).
    pub weight: f32, pub clutch: f32, pub moving: f32, pulse: f32,
    prev_pos: Vec3,
    /// His mouth (world) and facing, written by the pose each frame for the teeth.
    pub mouth: Option<(Vec3, Vec3)>,
    seed: u32,
}

#[derive(Component)]
pub struct Tooth { vel: Vec3, spin: Vec3, floor: f32, age: f32, rest: bool }
#[derive(Component)]
pub struct Pulse;

#[derive(Resource)]
pub struct ToothAssets { mesh: Handle<Mesh>, material: Handle<StandardMaterial>, bloody: Handle<StandardMaterial> }

pub fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(Hurt::default());
    commands.insert_resource(ToothAssets {
        mesh: meshes.add(Capsule3d::new(0.016, 0.022)),
        material: materials.add(StandardMaterial { base_color: Color::srgb(0.93, 0.9, 0.8), perceptual_roughness: 0.35, ..default() }),
        bloody: materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.62, 0.55), perceptual_roughness: 0.3, ..default() }),
    });
    // an elliptical vignette (clear centre, opaque rim), tinted red on screen
    let n = 256u32;
    let mut px = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (x as f32 / (n - 1) as f32 * 2. - 1., y as f32 / (n - 1) as f32 * 2. - 1.);
            let r = (u * u + v * v).sqrt() / std::f32::consts::SQRT_2;
            let t = ((r - 0.32) / 0.68).clamp(0., 1.);
            px.extend([255, 255, 255, (t * t * (3. - 2. * t) * 255.) as u8]);
        }
    }
    let image = images.add(Image::new(Extent3d { width: n, height: n, depth_or_array_layers: 1 }, TextureDimension::D2, px,
                                      TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::all()));
    commands.spawn((Pulse, ImageNode { image_mode: NodeImageMode::Stretch, ..ImageNode::new(image).with_color(Color::NONE) }, GlobalZIndex(400), Visibility::Hidden,
                    Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default() }));
}

/// FixedUpdate, after the fight tick: ease the weights, knock teeth out on a low-HP face hit.
pub fn step(play: Res<super::Play>, mut hurt: ResMut<Hurt>, mut commands: Commands, assets: Res<ToothAssets>, arena: Res<super::Arena>) {
    let p = &play.fight.player;
    let low = p.hp > 0 && p.hp <= LOW_HP;
    let ease = |v: &mut f32, to: f32, rate: f32| *v += (to - *v).clamp(-rate, rate);
    // aiming or shooting he straightens up (the pistol's own pose takes over); it eases back once he lowers the gun
    let gun_up = play.gun.as_ref().is_some_and(|g| g.action != super::GunAction::PistolIdle);
    let to = if low && play.as_kreely && !gun_up { 1. } else { 0. };
    ease(&mut hurt.weight, to, if gun_up { 1. / 6. } else { 1. / 20. });
    ease(&mut hurt.pulse, if low { 1. } else { 0. }, 1. / 25.);
    ease(&mut hurt.clutch, if !p.loco.combat && play.gun.is_none() { 1. } else { 0. }, 1. / 6.);
    let pos = Vec3::from_array(p.pos);
    let speed = (pos - hurt.prev_pos).with_y(0.).length();
    hurt.prev_pos = pos;
    let moving = if p.loco.combat { 0. } else { (speed / WALK_STEP).min(1.) };
    let m = hurt.moving; hurt.moving = m + (moving - m) * 0.25;
    if !play.as_kreely { return; }
    for e in &play.fight.events {
        let FightEvent::PlayerHit { atk, hp, .. } = *e else { continue; };
        if hp < 1 || hp > LOW_HP || !FACE_HITS.contains(&atk) { continue; }
        let Some((mouth, fwd)) = hurt.mouth else { continue; };
        // out of the mouth, thrown away from her and up
        let away = (pos - Vec3::from_array(play.fight.rin.pos)).with_y(0.).normalize_or(-fwd);
        let floor = super::floor(&arena.0, pos).unwrap_or(pos.y);
        let count = 2 + (hurt.seed % 3) as usize;
        if std::env::var_os("MW_HURT_LOG").is_some() { eprintln!("[hurt] tick={} atk={atk:#x} hp={hp}: {count} teeth from {mouth:.2} (feet {pos:.2}, floor {floor:.2})", play.tick); }
        for k in 0..count {
            let mut rnd = || { hurt.seed = hurt.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223 + k as u32); (hurt.seed >> 8) as f32 / 16_777_216. };
            let side = fwd.cross(Vec3::Y).normalize_or_zero();
            let vel = (away * 0.6 + fwd * 0.5).normalize_or_zero() * (1.2 + rnd() * 1.2) + side * (rnd() - 0.5) * 1.6 + Vec3::Y * (0.9 + rnd() * 1.1);
            let spin = Vec3::new(rnd() - 0.5, rnd() - 0.5, rnd() - 0.5) * 40.;
            let material = if k == 0 { assets.bloody.clone() } else { assets.material.clone() };
            commands.spawn((Tooth { vel, spin, floor, age: 0., rest: false }, Mesh3d(assets.mesh.clone()), MeshMaterial3d(material),
                            Transform::from_translation(mouth).with_rotation(Quat::from_rotation_x(rnd() * 3.))));
        }
    }
}

/// Update: the teeth fall, bounce and settle on the floor, then shrink away.
pub fn teeth(time: Res<Time>, mut commands: Commands, mut teeth: Query<(Entity, &mut Tooth, &mut Transform)>) {
    let dt = time.delta_secs().min(1. / 20.);
    for (e, mut tooth, mut t) in &mut teeth {
        tooth.age += dt;
        if tooth.age > 14. { commands.entity(e).despawn(); continue; }
        if tooth.age > 12. { t.scale = Vec3::splat((14. - tooth.age) / 2.); }
        if tooth.rest { continue; }
        tooth.vel.y -= 9.8 * dt;
        t.translation += tooth.vel * dt;
        let spin = tooth.spin * dt;
        t.rotation = (Quat::from_scaled_axis(spin) * t.rotation).normalize();
        let bottom = tooth.floor + 0.008;
        if t.translation.y < bottom {
            t.translation.y = bottom;
            if tooth.vel.y.abs() < 0.6 { tooth.rest = true; t.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(t.rotation.to_euler(EulerRot::YXZ).0); continue; }
            tooth.vel = Vec3::new(tooth.vel.x * 0.55, -tooth.vel.y * 0.35, tooth.vel.z * 0.55);
            tooth.spin *= 0.5;
        }
    }
}

/// Update: the red heartbeat over the screen, faster the lower the HP.
pub fn pulse(time: Res<Time>, play: Res<super::Play>, hurt: Res<Hurt>, intro: Res<super::intro::Intro>, pause: Res<super::pause::Pause>,
             mut node: Query<(&mut ImageNode, &mut Visibility), With<Pulse>>) {
    let Ok((mut image, mut vis)) = node.single_mut() else { return; };
    let hp = play.fight.player.hp;
    if hurt.pulse <= 0.001 || hp < 1 || intro.controls_blocked() || pause.open { *vis = Visibility::Hidden; return; }
    *vis = Visibility::Visible;
    let bpm = 72. + (LOW_HP - hp.min(LOW_HP)) as f32 * 2.5;
    let phase = (time.elapsed_secs() * bpm / 60.).fract();
    let bump = |c: f32, w: f32| (-((phase - c) / w).powi(2)).exp();
    let beat = bump(0.06, 0.06) + 0.6 * bump(0.3, 0.07);
    image.color = Color::srgba(0.7, 0., 0., hurt.pulse * (0.22 + 0.5 * beat));
}

fn turn(bones: &[Bone], poses: &mut [LocalPose], i: usize, q: Quat) {
    let frames = bone_frames(bones, poses);
    let parent = if bones[i].parent >= 0 { Quat::from_array(frames[bones[i].parent as usize].0) } else { Quat::from_array(poses[bones.len()].rotation) };
    let world = q * Quat::from_array(frames[i].0);
    poses[i].rotation = (parent.inverse() * world).normalize().to_array();
}

/// The hurt pose over whatever he is playing (character space: +Z forward, +X his left, +Y up).
pub fn apply(bones: &[Bone], poses: &mut [LocalPose], hurt: &Hurt) {
    let w = hurt.weight;
    if w <= 0.001 { return; }
    let at = |id: i16| bones.iter().position(|b| b.id == id);
    let (Some(hips), Some(spine), Some(chest), Some(neck), Some(rfoot), Some(lfoot)) = (at(0), at(1), at(2), at(3), at(18), at(23)) else { return; };
    let frames = bone_frames(bones, poses);
    // the right (hurt) leg carries him while its foot is the lower one
    let right = ((frames[lfoot].1[1] - frames[rfoot].1[1]) / 0.04).clamp(0., 1.);
    let mv = hurt.moving;
    let on_hurt = right * mv;
    // the hips sink onto the hurt leg; the body leans over it there and off it otherwise (standing: off it)
    poses[hips].translation[1] -= w * (0.02 + 0.05 * on_hurt);
    let lean = w * (mv * (0.2 * right - 0.05 * (1. - right)) - (1. - mv) * 0.07);
    turn(bones, poses, spine, Quat::from_rotation_z(lean) * Quat::from_rotation_x(w * (0.3 * (1. - 0.4 * mv) + 0.06 * on_hurt)));
    turn(bones, poses, chest, Quat::from_rotation_x(w * 0.22));
    turn(bones, poses, neck, Quat::from_rotation_x(-w * 0.38));
    // the left hand clutching the middle of his chest (two-bone reach, elbow out and down)
    let c = w * hurt.clutch;
    let (Some(upper), Some(fore), Some(hand)) = (at(12), at(13), at(14)) else { return; };
    if c <= 0.001 { return; }
    let f = bone_frames(bones, poses);
    let v = |i: usize| Vec3::from_array(f[i].1);
    let target = v(chest) + Quat::from_array(f[chest].0) * Vec3::new(0.06, 0.15, 0.25);
    let (s, e, h) = (v(upper), v(fore), v(hand));
    let (a, b) = (s.distance(e), e.distance(h));
    let to = target - s;
    let d = to.length().clamp(0.05, (a + b) * 0.995);
    let dir = to.normalize_or_zero();
    let cos = ((a * a + d * d - b * b) / (2. * a * d)).clamp(-1., 1.);
    let pole = Vec3::new(1., -0.6, -0.2);
    let pole = (pole - dir * pole.dot(dir)).normalize_or_zero();
    let elbow = s + dir * a * cos + pole * a * (1. - cos * cos).sqrt();
    turn(bones, poses, upper, Quat::IDENTITY.slerp(Quat::from_rotation_arc((e - s).normalize(), (elbow - s).normalize()), c));
    let f = bone_frames(bones, poses);
    let (e, h) = (Vec3::from_array(f[fore].1), Vec3::from_array(f[hand].1));
    let reach = s + dir * d;
    turn(bones, poses, fore, Quat::IDENTITY.slerp(Quat::from_rotation_arc((h - e).normalize(), (reach - e).normalize_or(h - e)), c));
}
