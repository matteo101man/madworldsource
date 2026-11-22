//! Blood on the floor [added]: every hit splashes (play_fx.rs `Fx::gore` queues them). The splat is the core bank's own
//! blood-splat texture (tex 49, the type-24 stain sheet: four splats), a flat quad on the floor under the hit, tinted
//! with getBloodColor(0) (0x80 0 0) as the game tints its blood, at a random turn and a size that grows with the blow.
//! Pools last SPLAT_LIFE seconds and fade, at most SPLATS at once (the oldest go first).
use bevy::prelude::*;

const SPLATS: usize = 96;
const SPLAT_LIFE: f32 = 45.;
const FADE: f32 = 5.;

#[derive(Component)]
pub struct Splat { born: f32, material: Handle<StandardMaterial>, alpha: f32 }

#[derive(Resource, Default)]
pub struct Gore { mesh: Option<Handle<Mesh>>, images: Vec<Handle<Image>>, rng: u32 }

pub fn spawn(mut gore: ResMut<Gore>, mut fx: ResMut<super::fx::Fx>, arena: Res<super::Arena>, time: Res<Time<Virtual>>, assets: Res<AssetServer>,
             mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, splats: Query<(Entity, &Splat)>, mut commands: Commands) {
    let queued = std::mem::take(&mut fx.splats);
    if queued.is_empty() { return; }
    if gore.mesh.is_none() {
        gore.mesh = Some(meshes.add(Rectangle::new(1., 1.)));
        gore.images = (0..4).map(|k| assets.load(format!("native/effects/tex-49-{k}.png"))).collect();
        gore.rng = 0x0b10_0d5e;
    }
    let mut live: Vec<(Entity, f32)> = splats.iter().map(|(e, s)| (e, s.born)).collect();
    let now = time.elapsed_secs();
    for (at, size) in queued {
        let Some(y) = super::floor(&arena.0, at + Vec3::Y * 0.5) else { continue; };
        let mut rng = gore.rng;
        let mut roll = || { rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (rng >> 8) as f32 / (1 << 24) as f32 };
        let k = (roll() * 4.) as usize % 4;
        let (turn, s, dx, dz) = (roll() * std::f32::consts::TAU, size * (0.7 + 0.6 * roll()), (roll() - 0.5) * 0.5, (roll() - 0.5) * 0.5);
        let bias = 40. + roll() * 10.;
        gore.rng = rng;
        let material = materials.add(StandardMaterial { base_color: Color::srgba(0.5, 0., 0., 0.92), base_color_texture: Some(gore.images[k].clone()),
                                                         alpha_mode: AlphaMode::Blend, unlit: true, depth_bias: bias, cull_mode: None, ..default() });
        let e = commands.spawn((Mesh3d(gore.mesh.clone().unwrap()), MeshMaterial3d(material.clone()),
                                Transform::from_translation(Vec3::new(at.x + dx, y + 0.012, at.z + dz))
                                    .with_rotation(Quat::from_rotation_y(turn) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)).with_scale(Vec3::splat(s)),
                                Splat { born: now, material, alpha: 0.92 })).id();
        live.push((e, now));
    }
    if live.len() > SPLATS {
        live.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (e, _) in &live[..live.len() - SPLATS] { commands.entity(*e).despawn(); }
    }
}

/// Old pools fade out.
pub fn fade(time: Res<Time<Virtual>>, mut splats: Query<(Entity, &mut Splat)>, mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    let now = time.elapsed_secs();
    for (e, mut s) in &mut splats {
        let age = now - s.born;
        if age < SPLAT_LIFE { continue; }
        let a = 0.92 * (1. - (age - SPLAT_LIFE) / FADE);
        if a <= 0. { commands.entity(e).despawn(); continue; }
        if (a - s.alpha).abs() > 0.02 { s.alpha = a; if let Some(mut m) = materials.get_mut(&s.material) { m.base_color = Color::srgba(0.5, 0., 0., a); } }
    }
}
