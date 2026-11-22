//! The stage-start Shock TV broadcast behind the boss challenge (recomp captures evidence/native-look-capture*/auto-0005..
//! 0015: black, then a moving camera down Great Wall Street under the "LIVE ASIAN TOWN 53F" bug and the GREAT WALL STREET
//! title, with the TAKE THE CHALLENGE? window over it).
//! - Camera: c201's room clip cam00 (361 frames, 30 Hz; tools/stage_room_cameras.py), looped while the challenge is open.
//!   [inferred: which room clip the native broadcast plays; cam00 is the long street flight, the others 2-7 s shots]
//! - Overlay: idpac_c201_us common program 6 (ASIAN TOWN 84/4, GREAT WALL STREET 84/5, 53 in group-15 digits, the moon
//!   glyph, frame pieces and its TV line layers), played as the HUD plays 39/31.
//! - Street life: two kung-fu grunts (em0101, `kungfu-*.glb`, tools/stage_actor.py) along the camera's path, one spinning
//!   through em0101_0106 and one breathing fire through em0101_0110 with an added flame plume (the fire variant em0101B
//!   shares the model; its own flame emitter is not staged, so the plume uses core smoke sprites tinted fire-orange).
//! Off with MW_BROADCAST=0 (and whenever the challenge is skipped).
use bevy::prelude::*;
use mw_data::MotionBank;
use mw_sim::{character::{sample_retargeted, Retarget}, ui::UiScene};
use super::pause::native_ui::{self, UiSource};

const CLIP: &str = "cam00";
const OVERLAY: u32 = 6;
/// Performers: (clip, index along cam00 whose look-at point they stand near, sideways offset).
const PERFORMERS: [(&str, usize, f32); 2] = [("em0101_0106.mot", 150, -1.2), ("em0101_0110.mot", 280, 1.4)];

#[derive(Resource)]
pub struct Broadcast { pub on: bool, t: f32, cam: Vec<[f32; 8]>, overlay: Option<UiScene>, source: Option<UiSource>, bank: Option<MotionBank>,
                       spawned: bool, last: f32 }
#[derive(Component)]
struct Performer { clip: usize, joints: Option<Vec<Entity>>, colored: bool, fire: bool, at: Vec3, yaw: f32 }
#[derive(Component)]
struct BroadcastNode;
#[derive(Component)]
struct Flame { vel: Vec3, age: f32 }
#[derive(Resource)]
struct FlameAssets { mesh: Handle<Mesh>, material: Handle<StandardMaterial> }

pub struct BroadcastPlugin;
impl Plugin for BroadcastPlugin {
    fn build(&self, app: &mut App) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let on = std::env::var("MW_BROADCAST").as_deref() != Ok("0") && std::env::var("MW_CHALLENGE").as_deref() != Ok("0")
            && std::env::var("MW_DIRECT_FIGHT").as_deref() != Ok("1");
        let cam = std::fs::read_to_string(root.join("assets/native/world/cameras.json")).ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| serde_json::from_value::<Vec<[f32; 8]>>(v["clips"][CLIP].clone()).ok()).unwrap_or_default();
        let bank = std::fs::read_to_string(root.join("assets/native/kungfu-motion.json")).ok().and_then(|s| MotionBank::parse(&s).ok());
        let source = root.join("assets/native/ui/idpac_c201_us/animation.idt").exists().then(|| UiSource::load(&root, "idpac_c201_us"));
        // program 6's widgets 15 and 16 are full-screen tiled TV-noise layers (group 9/10 at 20 %, 9/7 at 10 %); the port's
        // tile mapping is coarser than GX's, so they bury the picture: hidden. The top/bottom bars stay.
        let overlay = source.as_ref().map(|s| { let mut o = s.scene(OVERLAY); for i in [15, 16] { if let Some(st) = o.states.get_mut(i) { st.forced_hidden = true; } } o });
        app.insert_resource(Broadcast { on: on && !cam.is_empty(), t: 0., cam, overlay, source, bank, spawned: false, last: -1. })
            .add_systems(Update, (advance, spawn, pose_performers, flames, present));
    }
}

impl Broadcast {
    /// The broadcast camera (eye, target, vertical FOV) while it runs.
    pub fn camera(&self) -> Option<(Vec3, Vec3, f32)> {
        if !self.on { return None; }
        let n = self.cam.len() as f32;
        let f = (self.t * 30.) % (n - 1.);
        let (i, k) = (f.floor() as usize, f.fract());
        let (a, b) = (self.cam[i], self.cam[(i + 1).min(self.cam.len() - 1)]);
        let l = |j: usize| a[j] + (b[j] - a[j]) * k;
        Some((Vec3::new(l(0), l(1), l(2)), Vec3::new(l(3), l(4), l(5)), l(7)))
    }
    fn sample(&self, i: usize) -> [f32; 8] { self.cam[i.min(self.cam.len() - 1)] }
}

/// Runs while the challenge window holds the entry; ends (for good) once it is answered.
fn advance(time: Res<Time>, intro: Res<super::intro::Intro>, mut b: ResMut<Broadcast>, mut commands: Commands, nodes: Query<Entity, Or<(With<Performer>, With<Flame>)>>) {
    if !b.on { return; }
    if !intro.challenge_blocked() && b.t > 0.5 {
        b.on = false;
        for e in &nodes { commands.entity(e).despawn(); }
        return;
    }
    b.t += time.delta_secs();
    let tick = (b.t * 29.97).floor();
    if tick != b.last {
        let steps = if b.last < 0. { 1. } else { tick - b.last };
        b.last = tick;
        if let Some(o) = &mut b.overlay { o.advance(steps); }
    }
}

fn spawn(mut b: ResMut<Broadcast>, assets: Res<AssetServer>, arena: Res<super::Arena>, mut meshes: ResMut<Assets<Mesh>>,
         mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    if !b.on || b.spawned || b.bank.is_none() { return; }
    b.spawned = true;
    let b = &*b;
    let bank = b.bank.as_ref().unwrap();
    for (clip, along, side) in PERFORMERS {
        let Some(ci) = bank.clips.iter().position(|c| c.name == clip) else { continue; };
        // stand on the street under the camera's look-at point, facing the camera
        let s = b.sample(along);
        let (eye, target) = (Vec3::new(s[0], s[1], s[2]), Vec3::new(s[3], s[4], s[5]));
        let flat = Vec3::new(eye.x - target.x, 0., eye.z - target.z).normalize_or(Vec3::Z);
        let across = Vec3::new(flat.z, 0., -flat.x);
        let mut at = target + across * side;
        if let Some(y) = super::floor(&arena.0, at + Vec3::Y * 4.) { at.y = y; }
        let yaw = flat.x.atan2(flat.z);
        for (colored, file) in [(false, "native/kungfu-source.glb"), (true, "native/kungfu-pack.glb")] {
            commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))), Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)),
                            Visibility::Hidden, Performer { clip: ci, joints: None, colored, fire: clip.ends_with("0110.mot"), at, yaw }));
        }
    }
    let flame = materials.add(StandardMaterial { base_color: Color::srgba(1., 0.55, 0.15, 0.85), emissive: LinearRgba::rgb(6., 2.2, 0.4), unlit: true,
                                                 alpha_mode: AlphaMode::Add, ..default() });
    commands.insert_resource(FlameAssets { mesh: meshes.add(Sphere::new(0.05)), material: flame });
}

fn pose_performers(b: Res<Broadcast>, play: Res<super::Play>, mut performers: Query<(Entity, &mut Performer, &mut Visibility)>, children: Query<&Children>,
                   names: Query<&Name>, mut joints: Query<&mut Transform, Without<Performer>>, flames: Option<Res<FlameAssets>>, mut commands: Commands, mut puff: Local<f32>) {
    let Some(bank) = &b.bank else { return; };
    for (root, mut p, mut vis) in &mut performers {
        if p.joints.is_none() {
            let found: std::collections::HashMap<String, Entity> = children.iter_descendants(root).filter_map(|d| names.get(d).ok().map(|n| (n.as_str().to_string(), d))).collect();
            let js: Option<Vec<Entity>> = bank.bones.iter().map(|b| b.id).chain(std::iter::once(-1)).map(|id| found.get(&format!("role_0_bone{id}")).copied()).collect();
            p.joints = js;
            if p.joints.is_none() { continue; }
        }
        *vis = if p.colored == play.colored { Visibility::Visible } else { Visibility::Hidden };
        let clip = &bank.clips[p.clip];
        let frame = (b.t * 30.) % (clip.frames.max(2) - 1) as f32;
        let mut poses = sample_retargeted(&bank.bones, clip, frame, Retarget::NATIVE);
        mw_sim::character::strip_root_xz(&bank.bones, &mut poses);
        for (e, pose) in p.joints.as_ref().unwrap().iter().zip(&poses) {
            if let Ok(mut t) = joints.get_mut(*e) {
                t.translation = Vec3::from_array(pose.translation); t.rotation = Quat::from_array(pose.rotation); t.scale = Vec3::from_array(pose.scale);
            }
        }
        // the fire breather's plume: from the mouth along his facing during the breath
        if p.fire && p.colored == play.colored {
            let breathing = (55. ..88.).contains(&frame);   // em0101_0110_00.seq: hit on at 55, off at 88 (the breath)
            *puff += 1.;
            if let (true, Some(f)) = (breathing, &flames) {
                if (*puff as u32) % 2 == 0 {
                    let fwd = Quat::from_rotation_y(p.yaw) * Vec3::Z;
                    let mouth = p.at + Vec3::Y * 1.62 + fwd * 0.25;
                    let jitter = Vec3::new((*puff * 1.7).sin(), (*puff * 2.3).cos() * 0.5, (*puff * 3.1).sin()) * 0.6;
                    commands.spawn((Mesh3d(f.mesh.clone()), MeshMaterial3d(f.material.clone()), Transform::from_translation(mouth),
                                    Flame { vel: fwd * 6. + Vec3::Y * 0.8 + jitter, age: 0. }, bevy::light::NotShadowCaster));
                }
            }
        }
    }
}

fn flames(time: Res<Time>, mut q: Query<(Entity, &mut Flame, &mut Transform)>, mut commands: Commands) {
    let dt = time.delta_secs();
    for (e, mut f, mut t) in &mut q {
        f.age += dt;
        let v = f.vel; t.translation += v * dt; f.vel *= 0.9; f.vel.y += 1.5 * dt;
        t.scale = Vec3::splat(0.5 + f.age * 4.);
        if f.age > 0.55 { commands.entity(e).despawn(); }
    }
}

fn present(b: Res<Broadcast>, hud: Res<super::hud_ui::Hud>, assets: Res<AssetServer>, windows: Query<&Window>, nodes: Query<Entity, With<BroadcastNode>>,
           mut commands: Commands) {
    for e in &nodes { commands.entity(e).despawn(); }
    if !b.on { return; }
    let (Some(scene), Some(source), Ok(window)) = (&b.overlay, &b.source, windows.single()) else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    for sprite in native_ui::sprites(scene, &source.layout, &hud.source().layout, &assets, viewport, 800, None) {
        commands.spawn((sprite, BroadcastNode));
    }

}
