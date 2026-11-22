//! Native rig validation slice. The boss simulation, arena and original GX lighting are unfinished.
use bevy::{camera::visibility::NoFrustumCulling, prelude::*, render::view::screenshot::{save_to_disk, Screenshot}};
use mw_data::MotionBank;

#[derive(Resource)]
struct NativeRig { bank: MotionBank, clip: usize, frame: f32, paused: bool, colored: bool, ticks: u32 }
#[derive(Component)]
struct PaletteRoot(bool);
#[derive(Resource)]
struct NativeArena(mw_data::Arena);
#[derive(Resource)]
struct NativeCollision(mw_data::collision::CollisionWorld);

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bank = MotionBank::parse(&std::fs::read_to_string(root.join("assets/native/rinrin-motion.json")).expect("run tools/stage_rinrin.py")).expect("valid native motion bank");
    let requested = std::env::var("MW_CLIP").unwrap_or_else(|_| "em0440_0000.mot".into());
    let clip = bank.clips.iter().position(|c| c.name == requested).expect("requested native resource");
    let frame = std::env::var("MW_FRAME").ok().map(|v| v.parse::<f32>().expect("numeric inspection frame"));
    let arena: mw_data::Arena = serde_json::from_str(&std::fs::read_to_string(root.join("assets/native/arena-meta.json")).expect("run tools/stage_arena.py")).expect("native arena metadata");
    let collision = mw_data::collision::CollisionWorld::parse(&std::fs::read_to_string(root.join("assets/native/collision/world.json")).expect("run tools/stage_collision.py with the Havok adapter")).expect("validated native collision geometry");
    App::new()
        .insert_resource(NativeRig { bank, clip, frame: frame.unwrap_or(0.), paused: frame.is_some(), colored: std::env::var("MW_COLOR").as_deref()==Ok("1"), ticks: 0 })
        .insert_resource(NativeArena(arena))
        .insert_resource(NativeCollision(collision))
        .insert_resource(ClearColor(Color::srgb(0.12, 0.12, 0.12)))
        .add_plugins(DefaultPlugins.set(AssetPlugin { file_path: root.join("assets").to_string_lossy().into_owned(), ..default() }).set(WindowPlugin {
            primary_window: Some(Window { title: "MadWorld RinRin — native rig validation".into(), resolution: (1280, 720).into(), ..default() }), ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (controls, pose, audit_capture).chain())
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>, arena: Res<NativeArena>, collision: Res<NativeCollision>) {
    let position = Vec3::from_array(arena.0.spawns.iter().find(|s| s.id==0x20440).expect("native RinRin spawn").position);
    let hits = mw_sim::collision::raw_surface_ray(&collision.0, position.to_array().map(f64::from), [0., -1., 0.], 100., Some("col.hkx")).expect("valid spawn probe");
    if let Some(hit) = hits.first() {
        info!("Native raw roof probe: {} distance={} category={:?}; margins/contact resolution not yet applied", hit.body.name, hit.distance, hit.body.loader_category());
    }
    commands.spawn((Camera3d::default(), Transform::from_translation(position+Vec3::new(3., 1.7, 5.5)).looking_at(position+Vec3::Y, Vec3::Y)));
    commands.spawn(WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(arena.0.world.clone()))));
    commands.spawn((WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("native/rinrin-color.glb"))), Transform::from_translation(position), PaletteRoot(true), Visibility::Hidden));
    commands.spawn((WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("native/rinrin-source.glb"))), Transform::from_translation(position), PaletteRoot(false), Visibility::Visible));
    commands.spawn((Text::new("Loading native RinRin rig"), TextFont { font_size: bevy::text::FontSize::Px(18.), ..default() }, Node { position_type: PositionType::Absolute, top: px(14), left: px(14), ..default() }));
}

fn controls(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut rig: ResMut<NativeRig>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) { exit.write(AppExit::Success); }
    if keys.just_pressed(KeyCode::Space) { rig.paused = !rig.paused; }
    if keys.just_pressed(KeyCode::KeyC) { rig.colored = !rig.colored; }
    let step = if keys.just_pressed(KeyCode::ArrowRight) { 1 } else if keys.just_pressed(KeyCode::ArrowLeft) { -1 } else { 0 };
    if step != 0 { rig.clip = (rig.clip as isize + step).rem_euclid(rig.bank.clips.len() as isize) as usize; rig.frame = 0.; }
    if !rig.paused { rig.frame = (rig.frame + time.delta_secs() * 60.) % rig.bank.clips[rig.clip].frames as f32; }
}

fn pose(rig: Res<NativeRig>, mut joints: Query<(&Name, &mut Transform)>, mut text: Query<&mut Text>, meshes: Query<Entity, Added<Mesh3d>>, mut roots: Query<(&PaletteRoot, &mut Visibility)>, mut commands: Commands) {
    let clip = &rig.bank.clips[rig.clip];
    let poses = mw_sim::sample(&rig.bank.bones, clip, rig.frame);
    let mut bound = 0;
    for (name, mut transform) in &mut joints {
        let Some(id) = name.as_str().strip_prefix("role_0_bone").and_then(|s| s.parse::<i16>().ok()) else { continue; };
        let index = if id == -1 { Some(rig.bank.bones.len()) } else { rig.bank.bones.iter().position(|b| b.id == id) };
        if let Some(i) = index {
            let p = poses[i];
            transform.translation = Vec3::from_array(p.translation);
            transform.rotation = Quat::from_euler(EulerRot::ZYX, p.rotation[2], p.rotation[1], p.rotation[0]);
            transform.scale = Vec3::from_array(p.scale);
            bound += 1;
        }
    }
    for entity in &meshes { commands.entity(entity).insert(NoFrustumCulling); }
    for (palette, mut visibility) in &mut roots { *visibility = if palette.0 == rig.colored { Visibility::Visible } else { Visibility::Hidden }; }
    for mut value in &mut text {
        let label = format!("Native rig validation | {} | {:.1}/{} | {bound}/{} joints\nLeft/Right motion   Space pause   C color   Esc exit\n60 fps inspection clock | {} | source atlas fallback\nNative c201 scenery | mechanics, collision, audio and GX lighting unfinished", clip.name, rig.frame, clip.frames, poses.len()*2, if rig.colored { "added color palette" } else { "original source palette" });
        if value.as_str()!=label { **value=label; }
    }
}

fn audit_capture(mut rig: ResMut<NativeRig>, mut commands: Commands, mut exit: MessageWriter<AppExit>, joints: Query<&Name>) {
    if std::env::var_os("MW_CAPTURE").is_none() { return; }
    if joints.iter().filter(|n| n.as_str().starts_with("role_0_bone")).count() != (rig.bank.bones.len() + 1)*2 { return; }
    rig.ticks += 1;
    if rig.ticks == 120 {
        let file = std::env::var("MW_CAPTURE").unwrap();
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(file));
    }
    if rig.ticks == 180 { exit.write(AppExit::Success); }
}

