//! Added, not native: visible rounds for Kreely's pistol. Each shot (play.rs `GunEvent::Shot`) launches a small copper
//! bullet with a smoky white trail from the muzzle to where the round ended (the stage, RinRin, or her fan), at a speed
//! the eye can follow; the trail drains into the impact and fades. A round batted off a fan carries on as a second
//! tracer along each carom of its ricochet.
//! The trail is a camera-facing ribbon along the flight line with a generated wispy texture (bright at the head, thin
//! smoke behind).
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat}};

/// Visual speed of a round (m/s; a real one is ~5x faster and would be invisible), trail length and width. Seen from
/// behind the shooter the trail runs nearly along the view, so it is long and wide enough to read foreshortened.
const SPEED: f32 = 75.;
const TRAIL: f32 = 5.5;
const WIDTH: f32 = 0.11;
/// Seconds the trail takes to drain and fade after the round stops.
const FADE: f32 = 0.35;

#[derive(Resource)]
pub struct TracerAssets { ribbon: Handle<Mesh>, bullet: Handle<Mesh>, smoke: Handle<StandardMaterial>, copper: Handle<StandardMaterial>,
                          casing: Handle<Mesh>, cartridge: Handle<Mesh>, brass: Handle<StandardMaterial> }

/// The pistol muzzle's world position as last posed (play.rs `pose`).
#[derive(Resource, Default)]
pub struct Muzzle(pub Option<Vec3>, pub Option<Vec3>);

#[derive(Component)]
struct Tracer { from: Vec3, to: Vec3, travelled: f32, after: f32, then: Vec<Vec3>, trail: Entity, bullet: Entity }

pub struct TracerPlugin;
impl Plugin for TracerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Muzzle>().init_resource::<Ejector>().add_systems(Startup, setup).add_systems(Update, (fly, fly_rounds, fly_casings));
    }
}

fn setup(mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>, mut commands: Commands) {
    // trail texture: u along the trail (0 tail .. 1 head), v across; brightest core at the head, wisps behind
    let (w, h) = (256u32, 32u32);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    let wisp = |x: f32, y: f32| ((x * 31.).sin() * 0.5 + (x * 13. + y * 4.).sin() * 0.3 + (x * 67. - y * 9.).sin() * 0.2) * 0.5 + 0.5;
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / (w - 1) as f32; let v = y as f32 / (h - 1) as f32 * 2. - 1.;
            let spread = 0.25 + 0.75 * (1. - u);                         // the smoke widens behind the round
            let core = (-(v / (0.18 + 0.5 * (1. - u).powi(2) * spread)).powi(2) * 2.).exp();
            let head = u.powf(2.2);
            let smoke = core * (0.35 + 0.65 * wisp(u, v)) * u.powf(0.7);
            let a = (smoke * 0.9 + core * head * 0.8).clamp(0., 1.);
            let white = 0.82 + 0.18 * head;
            data.extend([(white * 255.) as u8, (white * 252.) as u8, (white * 245.) as u8, (a * 255.) as u8]);
        }
    }
    let image = images.add(Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data,
                                      TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD));
    let ribbon = Mesh::from(Rectangle::new(1., 1.)).translated_by(Vec3::new(-0.5, 0., 0.));   // x in [-1, 0]: tail .. head
    commands.insert_resource(TracerAssets {
        ribbon: meshes.add(ribbon),
        bullet: meshes.add(Capsule3d::new(0.018, 0.05)),
        smoke: materials.add(StandardMaterial { base_color_texture: Some(image), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None,
                                                double_sided: true, ..default() }),
        copper: materials.add(StandardMaterial { base_color: Color::srgb(0.78, 0.5, 0.28), emissive: LinearRgba::rgb(9., 5., 2.2), ..default() }),
        casing: meshes.add(Cylinder::new(0.0048, 0.019)),
        cartridge: meshes.add(Capsule3d::new(0.0048, 0.02)),
        brass: materials.add(StandardMaterial { base_color: Color::srgb(1., 0.78, 0.25), metallic: 0.6, perceptual_roughness: 0.25, emissive: LinearRgba::rgb(2.2, 1.5, 0.35), ..default() }),
    });
}

/// A live round's tracer: it rides the round's legs (GunEvent::Leg) at the round's own speed, a bright streak with
/// the smoky trail behind it, and fades where the round stopped.
#[derive(Component)]
pub struct RoundTracer { id: u32, pts: Vec<Vec3>, total: f32, head: f32, done: bool, after: f32, trail: Entity, bullet: Entity,
                         /// seconds since it left the muzzle; one smoke wake per leg (spawned as the head enters it)
                         clock: f32, wakes: Vec<Entity>,
                         /// the streak's length and width (the screen's camera stands back: longer and wider than in the headset)
                         len: f32, width: f32 }
/// The round tracer's speed (a 9 mm round's, m/s), its streak's length and width.
const ROUND_SPEED: f32 = 360.;
const ROUND_TRAIL: f32 = 4.;
const ROUND_WIDTH: f32 = 0.06;

/// A spent case (or a live round racked out) flying from the ejection port: tumbling, bouncing on the floor, settling.
#[derive(Component)]
pub struct Casing { vel: Vec3, spin: Vec3, ground: Option<f32>, ground_at: Vec3, rest: f32, bounces: u8 }
const CASINGS_MAX: usize = 40;
const CASING_LIFE: f32 = 12.;

/// Where the pistol throws its cases: the VR gun's frame this frame (model: -x forward, +z up, +y its right side), and
/// cases waiting to be thrown (a racked-out live round: port, velocity, live).
#[derive(Resource, Default)]
pub struct Ejector { pub vr: Option<Transform>, pub extra: Vec<(Vec3, Vec3, bool)> }

/// The ejection port and the gun's right / up / back, for a shot: the VR gun's own frame, or the screen pistol's two ends.
fn port_frame(ej: &Ejector, gun: &Muzzle, muzzle: Vec3, dir: Vec3) -> (Vec3, Vec3, Vec3, Vec3) {
    if let Some(root) = ej.vr {
        return (root.transform_point(Vec3::new(0.03, 0.012, 0.088)), root.rotation * Vec3::Y, root.rotation * Vec3::Z, root.rotation * Vec3::X);
    }
    let (m, b) = match (gun.0, gun.1) { (Some(m), Some(b)) if m.distance(muzzle) < 1.5 => (m, b), _ => (muzzle, muzzle - dir * 0.18) };
    let along = (m - b).normalize_or(dir);
    let right = along.cross(Vec3::Y).normalize_or(Vec3::X);
    let up = right.cross(along).normalize_or(Vec3::Y);
    (b + (m - b) * 0.55 + up * 0.02 + right * 0.012, right, up, -along)
}

fn throw_case(commands: &mut Commands, a: &TracerAssets, port: Vec3, vel: Vec3, live: bool, seed: u32, size: f32) {
    let r = |k: u32| { let x = (seed.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B)).wrapping_mul(0xC2B2_AE35); (x >> 8) as f32 / (1u32 << 24) as f32 };
    let spin = Vec3::new(r(1) - 0.5, r(2) - 0.5, r(3) - 0.5) * 60.;
    commands.spawn((Casing { vel, spin, ground: None, ground_at: Vec3::splat(1e9), rest: 0., bounces: 0 },
                    Mesh3d(if live { a.cartridge.clone() } else { a.casing.clone() }), MeshMaterial3d(a.brass.clone()),
                    Transform::from_translation(port).with_rotation(Quat::from_rotation_z(1.2 * r(4))).with_scale(Vec3::splat(size)), bevy::light::NotShadowCaster));
}

/// Launch a tracer from each of this game frame's shots (FixedUpdate, after the fight tick); the round tracers' legs;
/// the cases out of the port.
#[allow(clippy::too_many_arguments)]
pub fn launch(play: Res<super::Play>, assets: Option<Res<TracerAssets>>, gun: Res<Muzzle>, cams: Query<(&Camera, &GlobalTransform), (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>, mut commands: Commands,
              mut ej: ResMut<Ejector>, mut rounds: Query<&mut RoundTracer>, cases: Query<Entity, With<Casing>>) {
    let Some(a) = assets else { return; };
    // (seen from the screen's camera, a few metres back, the cases and the streak are drawn larger to read)
    let headset = super::vr::on();
    let case_size = if headset { 1. } else { 1.8 };
    let (len, width) = if headset { (ROUND_TRAIL, ROUND_WIDTH) } else { (TRAIL, WIDTH) };
    // spent cases out of the port: to the gun's right, up and a little back, with a tumble
    let mut thrown = 0;
    for g in &play.gun_events {
        let super::GunEvent::Shot { muzzle, dir, .. } = *g else { continue; };
        let (port, right, up, back) = port_frame(&ej, &gun, muzzle, dir);
        let seed = play.shots_fired.len() as u32 * 7 + thrown;
        let j = |k: u32| ((seed.wrapping_mul(2_654_435_761) >> (k * 5)) & 0xFF) as f32 / 255.;
        let vel = right * (1.9 + 0.8 * j(1)) + up * (1.3 + 0.7 * j(2)) + back * (0.2 + 0.4 * j(3));
        throw_case(&mut commands, &a, port, vel, false, seed, case_size);
        if std::env::var_os("MW_TRACER_LOG").is_some() { eprintln!("[tracer] case out at {port:.2} vel {vel:.2} (vr frame {})", ej.vr.is_some()); }
        thrown += 1;
    }
    for (port, vel, live) in std::mem::take(&mut ej.extra) { throw_case(&mut commands, &a, port, vel, live, play.tick as u32 * 13 + thrown, case_size); thrown += 1; }
    // (only the newest cases stay)
    let n = cases.iter().count() + thrown as usize;
    for e in cases.iter().take(n.saturating_sub(CASINGS_MAX)) { commands.entity(e).despawn(); }
    // the rounds' legs: a new round starts its tracer at the muzzle, its legs extend it (a round's several legs in one
    // tick, a ricochet's, extend the tracer started this tick)
    let extend = |t: &mut RoundTracer, from: Vec3, to: Vec3, last: bool| {
        if t.pts.last().is_none_or(|p| p.distance(from) > 0.05) { t.total += t.pts.last().map_or(0., |p| p.distance(from)); t.pts.push(from); }
        t.total += from.distance(to); t.pts.push(to); t.done |= last;
    };
    let mut fresh: Vec<RoundTracer> = vec![];
    for g in &play.gun_events {
        let super::GunEvent::Leg { id, from, to, last } = *g else { continue; };
        if let Some(mut t) = rounds.iter_mut().find(|t| t.id == id && !t.done) { extend(&mut t, from, to, last); }
        else if let Some(t) = fresh.iter_mut().find(|t| t.id == id) { extend(t, from, to, last); }
        else {
            let trail = commands.spawn((Mesh3d(a.ribbon.clone()), MeshMaterial3d(a.smoke.clone()), Transform::from_translation(from).with_scale(Vec3::ZERO),
                                        bevy::light::NotShadowCaster)).id();
            let bullet = commands.spawn((Mesh3d(a.bullet.clone()), MeshMaterial3d(a.copper.clone()), Transform::from_translation(from).with_scale(Vec3::splat(0.6)),
                                         bevy::light::NotShadowCaster)).id();
            fresh.push(RoundTracer { id, pts: vec![from, to], total: from.distance(to), head: 0., done: last, after: 0., trail, bullet, len, width, clock: 0., wakes: vec![] });
            if std::env::var_os("MW_TRACER_LOG").is_some() { eprintln!("[tracer] round {id} from {from:.2} to {to:.2} last {last}"); }
        }
    }
    for t in fresh { commands.spawn(t); }
    for g in &play.gun_events {
        let super::GunEvent::Shot { muzzle, dir, end, kind } = *g else { continue; };
        // (a VR round is a real projectile: no drawn tracer, its strike shows where it went)
        if matches!(kind, super::ShotEnd::Launched) { continue; }
        let to = end.unwrap_or(muzzle + dir * 60.);
        // from the gun itself (off to the side of the aim line), converging on where the round ended
        let muzzle = gun.0.filter(|m| m.distance(muzzle) < 1.5).unwrap_or(muzzle);
        let then = if let super::ShotEnd::Fan { bounces, n, .. } = kind { bounces[..n].to_vec() } else { vec![] };
        if std::env::var_os("MW_TRACER_LOG").is_some() {
            let ndc = cams.single().ok().and_then(|(c, t)| c.world_to_ndc(t, to));
            eprintln!("[tracer] {muzzle:?} -> {to:?} ({:.1} m) {kind:?} end on screen (ndc) {ndc:?}", muzzle.distance(to));
        }
        spawn(&mut commands, &a, muzzle, to, then);
    }
}

fn spawn(commands: &mut Commands, a: &TracerAssets, from: Vec3, to: Vec3, then: Vec<Vec3>) {
    let trail = commands.spawn((Mesh3d(a.ribbon.clone()), MeshMaterial3d(a.smoke.clone()), Transform::from_translation(from).with_scale(Vec3::ZERO),
                                bevy::light::NotShadowCaster)).id();
    let bullet = commands.spawn((Mesh3d(a.bullet.clone()), MeshMaterial3d(a.copper.clone()), Transform::from_translation(from),
                                 bevy::light::NotShadowCaster)).id();
    commands.spawn(Tracer { from, to, travelled: 0., after: 0., then, trail, bullet });
}

/// The smoke a round leaves along its path: it holds a moment after the head passes, then thins and drains from the
/// tail over WAKE_FADE (a round at 360 m/s crosses a street in a frame or two; the wake is what the eye catches).
const WAKE_HOLD: f32 = 0.15;
const WAKE_FADE: f32 = 0.5;

/// Update: each round tracer's head runs along its legs at the round's speed, a bright streak with the copper round
/// at its tip; every leg it has run keeps a smoke wake that drains away behind it.
fn fly_rounds(time: Res<Time>, assets: Option<Res<TracerAssets>>, cam: Query<&GlobalTransform, (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>,
              mut tracers: Query<(Entity, &mut RoundTracer)>, mut transforms: Query<&mut Transform>, mut commands: Commands) {
    let Some(a) = assets else { return; };
    let dt = time.delta_secs();
    let eye = cam.iter().next().map(|c| c.translation());
    let face = |from: Vec3, dir: Vec3| -> Quat {
        let to_cam = eye.map_or(Vec3::Y, |c| (c - from).normalize_or(Vec3::Y));
        let across = dir.cross(to_cam).normalize_or(Vec3::Y);
        Quat::from_mat3(&Mat3::from_cols(dir, across, dir.cross(across)))
    };
    for (e, mut t) in &mut tracers {
        t.clock += dt;
        if t.head < t.total { t.head = (t.head + ROUND_SPEED * dt).min(t.total); } else if t.done { t.after += dt; }
        // the legs: where each starts along the path, the head's leg
        let mut offset = 0.;
        let (mut head_pos, mut head_dir) = (t.pts[0], Vec3::Z);
        let legs: Vec<(Vec3, Vec3, f32, f32)> = t.pts.windows(2).map(|w| { let l = w[0].distance(w[1]); let o = offset; offset += l; (w[0], w[1], o, l) }).collect();
        for (k, &(p0, p1, o, l)) in legs.iter().enumerate() {
            if l <= 1e-4 || t.head < o { continue; }
            let dir = (p1 - p0) / l;
            let reach = (t.head - o).min(l);
            if t.head <= o + l || k == legs.len() - 1 { head_pos = p0 + dir * reach; head_dir = dir; }
            // this leg's wake: from its drained tail to how far the head has run along it
            while t.wakes.len() <= k {
                let w = commands.spawn((Mesh3d(a.ribbon.clone()), MeshMaterial3d(a.smoke.clone()), Transform::from_translation(p0).with_scale(Vec3::ZERO), bevy::light::NotShadowCaster)).id();
                t.wakes.push(w);
            }
            let entered = o / ROUND_SPEED;
            let age = (t.clock - entered).max(0.);
            let tail = l * ((age - WAKE_HOLD) / WAKE_FADE).clamp(0., 1.);
            let seg = (reach - tail).max(0.);
            if let Ok(mut tr) = transforms.get_mut(t.wakes[k]) {
                let end = p0 + dir * reach;
                tr.translation = end;
                tr.rotation = face(end, dir);
                tr.scale = Vec3::new(seg.max(1e-4), t.width * 1.3 * (1. + age * 4.), 1.);
            }
        }
        // the bright streak at the head
        let running = !(t.done && t.head >= t.total);
        if let Ok(mut tr) = transforms.get_mut(t.trail) {
            tr.translation = head_pos; tr.rotation = face(head_pos, head_dir);
            tr.scale = if running { Vec3::new(t.len.min(t.head).max(1e-4), t.width, 1.) } else { Vec3::ZERO };
        }
        if let Ok(mut b) = transforms.get_mut(t.bullet) {
            b.translation = head_pos; b.rotation = Quat::from_rotation_arc(Vec3::Y, head_dir);
            b.scale = Vec3::splat(if running { 0.6 } else { 0. });
        }
        let gone = t.done && t.head >= t.total && t.clock > t.total / ROUND_SPEED + WAKE_HOLD + WAKE_FADE;
        if gone || t.clock > 10. {
            commands.entity(t.trail).despawn(); commands.entity(t.bullet).despawn();
            for w in &t.wakes { commands.entity(*w).despawn(); }
            commands.entity(e).despawn();
        }
    }
}

/// Update: the cases tumble out, bounce on the floor (losing most of their speed each time) and settle; they go after
/// a while.
fn fly_casings(time: Res<Time>, arena: Option<Res<super::Arena>>, mut cases: Query<(Entity, &mut Casing, &mut Transform)>, mut commands: Commands) {
    let dt = time.delta_secs().min(0.05);
    for (e, mut c, mut t) in &mut cases {
        c.rest += dt;
        if c.rest > CASING_LIFE { commands.entity(e).despawn(); continue; }
        if c.bounces >= 3 && c.vel.length_squared() < 1e-4 { continue; }
        // the floor under it (looked up again once it has moved off)
        if Vec2::new(t.translation.x - c.ground_at.x, t.translation.z - c.ground_at.z).length() > 0.3 {
            c.ground = arena.as_ref().and_then(|a| super::floor_any(&a.0, t.translation));
            c.ground_at = t.translation;
        }
        c.vel.y -= 9.81 * dt;
        t.translation += c.vel * dt;
        t.rotation = (Quat::from_scaled_axis(c.spin * dt) * t.rotation).normalize();
        if let Some(gy) = c.ground {
            if t.translation.y < gy + 0.005 {
                t.translation.y = gy + 0.005;
                c.bounces += 1;
                if c.bounces >= 3 || c.vel.y.abs() < 0.4 {
                    // settled: on its side, rolled a little
                    c.vel = Vec3::ZERO; c.spin = Vec3::ZERO; c.bounces = 3;
                    let yaw = t.rotation.to_euler(EulerRot::YXZ).0;
                    t.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
                } else {
                    c.vel = Vec3::new(c.vel.x * 0.45, -c.vel.y * 0.35, c.vel.z * 0.45);
                    c.spin *= 0.5;
                }
            }
        }
    }
}

fn fly(time: Res<Time>, assets: Option<Res<TracerAssets>>, cam: Query<&GlobalTransform, (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>, mut tracers: Query<(Entity, &mut Tracer)>,
       mut transforms: Query<&mut Transform>, mut commands: Commands) {
    if std::env::var_os("MW_TRACER_LOG").is_some() && !tracers.is_empty() {
        eprintln!("[tracer] fly: {} live, assets {}, cameras {}", tracers.iter().count(), assets.is_some(), cam.iter().count());
    }
    let (Some(a), Ok(cam)) = (assets, cam.single()) else { return; };
    let dt = time.delta_secs();
    for (e, mut t) in &mut tracers {
        let length = t.from.distance(t.to);
        let dir = (t.to - t.from).normalize_or(Vec3::Z);
        // inspection: MW_TRACER_SLOW=k divides the speed by k and widens the trail k times
        let k = std::env::var("MW_TRACER_SLOW").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.).max(1.);
        if t.travelled < length { t.travelled = (t.travelled + SPEED / k * dt).min(length); } else { t.after += dt; }
        let head = t.from + dir * t.travelled;
        // the trail drains into the impact as it fades
        let drain = (t.after / FADE).clamp(0., 1.);
        let tail_len = (t.travelled.min(TRAIL)) * (1. - drain);
        if let Ok(mut tr) = transforms.get_mut(t.trail) {
            let to_cam = (cam.translation() - head).normalize_or(Vec3::Y);
            let across = dir.cross(to_cam).normalize_or(Vec3::Y);
            let normal = dir.cross(across);
            tr.translation = head;
            tr.rotation = Quat::from_mat3(&Mat3::from_cols(dir, across, normal));
            tr.scale = Vec3::new(tail_len.max(1e-4), WIDTH * k * (1. + drain * 1.5), 1.);
        }
        if let Ok(mut b) = transforms.get_mut(t.bullet) {
            b.translation = head;
            b.rotation = Quat::from_rotation_arc(Vec3::Y, dir);
            b.scale = Vec3::splat(if t.travelled >= length { 0. } else { 1. });
        }
        if t.travelled >= length && t.after == 0. {
            if !t.then.is_empty() { let mut rest = std::mem::take(&mut t.then); let next = rest.remove(0); let at = t.to; spawn(&mut commands, &a, at, next, rest); }
        }
        if t.after >= FADE {
            commands.entity(t.trail).despawn(); commands.entity(t.bullet).despawn(); commands.entity(e).despawn();
        }
    }
}
