//! Blood that sticks to the characters: EST particle type 24 (cEsp24, ctor 80097A54). Its first draw (801F89E8)
//! projects the effect's texture onto one or two meshes of its owner (801D5C5C / 801D6024): a box of the particle's
//! width x height x 2 depth along its local +Z, keeping whole triangles whose face normal points along +Z and that are
//! not wholly outside the box, each vertex given uv = 0.5 + dot(U|V, p - origin) / (w|h). The decal reuses the model's
//! own vertices (drawn with GX_EQUAL depth), so it rides the skinning; tinted with getBloodColor; it lasts the
//! particle's life, and life 0 never ends (em0440 201 / 120 stay on RinRin).
//! Here: the target primitive (GLB role_0_mesh_<i> = MDB mesh i; out of range -> mesh 0; the player's own mesh table is
//! not mapped, so his largest mesh) is posed on the CPU from its joints, the kept triangles get a fresh UV set, and the
//! decal is spawned as a skinned sibling sharing the joints, drawn over the body with a depth bias.
//! Inferred: the projection origin (the struck meshes' centre plus the emitter offset, the owner's facing as +Z, a
//! random roll and a small random shift so stains collect instead of stacking), the height rule (width x the emitter's
//! +0xC0 ratio, at least half the width), a cap of DECALS_PER_OWNER.
use bevy::{asset::RenderAssetUsages, image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
           mesh::{skinning::{SkinnedMesh, SkinnedMeshInverseBindposes}, Indices, VertexAttributeValues}, prelude::*};

/// A stain to project (play_fx.rs routes type-24 emitters here).
#[derive(Clone)]
pub struct DecalRequest { pub on_player: bool, /** a model root to stain instead of the player / RinRin (a street grunt) */ pub root: Option<Entity>,
                          /** projected at a world point, facing a direction, onto whichever visible parts are there (a bullet
                           wound) instead of the struck meshes' centre */ pub at: Option<(Vec3, Vec3)>, pub meshes: [i16; 2], pub depth: f32, pub size: [f32; 2], pub offset: Vec3,
                          pub image: String, pub tint: [f32; 4], pub life: u16, pub cull: bool,
                          /** no random roll: the image's top stays up in the world (drips run down) [added] */ pub upright: bool }

/// On a model root: its stains fade with it (1 opaque .. 0 gone; play_street.rs's fading corpses).
#[derive(Component)]
pub struct Fade(pub f32);

#[derive(Component)]
struct BloodDecal { on_player: bool, root: Option<Entity>, born: f32, life: Option<f32>, material: Handle<StandardMaterial>, alpha: f32 }

const DECALS_PER_OWNER: usize = 24;
/// The native limit of vertices per decal (0x600).
const MAX_VERTICES: usize = 0x600;

pub struct DecalPlugin;
impl Plugin for DecalPlugin {
    fn build(&self, app: &mut App) { app.add_systems(Update, (build, age)); }
}

fn build(mut fx: ResMut<super::fx::Fx>, play: Res<super::Play>, chars: Query<(Entity, &super::Character, &GlobalTransform)>, children: Query<&Children>,
         names: Query<&Name>, parents: Query<&ChildOf>, skinned: Query<(&Mesh3d, &SkinnedMesh, &Transform)>, (globals, visibility): (Query<&GlobalTransform>, Query<&Visibility>),
         mut meshes: ResMut<Assets<Mesh>>, bindposes: Res<Assets<SkinnedMeshInverseBindposes>>, mut materials: ResMut<Assets<StandardMaterial>>,
         assets: Res<AssetServer>, time: Res<Time>, existing: Query<(Entity, &BloodDecal)>, mut rng: Local<u32>, mut commands: Commands) {
    if fx.decals.is_empty() { return; }
    let t0 = std::time::Instant::now(); let n = fx.decals.len();
    let mut rand = || { *rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (*rng >> 8) as f32 / (1 << 23) as f32 - 1. };
    // (stains already sent away this frame: several requests in one frame would cap the same oldest ones again)
    let mut gone: Vec<Entity> = vec![];
    for req in std::mem::take(&mut fx.decals) {
        let want = if req.on_player { if play.as_kreely { super::Who::Kreely } else { super::Who::Jack } } else { super::Who::Rin };
        // per-owner cap: the oldest stains go first
        let mut mine: Vec<(Entity, f32)> = existing.iter().filter(|(e, d)| d.on_player == req.on_player && d.root == req.root && !gone.contains(e)).map(|(e, d)| (e, d.born)).collect();
        if mine.len() >= DECALS_PER_OWNER * 2 { mine.sort_by(|a, b| a.1.total_cmp(&b.1)); for (e, _) in mine.iter().take(2) { commands.entity(*e).despawn(); gone.push(*e); } }
        let (jitter, roll) = (Vec2::new(rand(), rand()) * 0.06, rand() * std::f32::consts::PI);
        let image: Handle<Image> = assets.load_builder().with_settings(|s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: ImageAddressMode::ClampToEdge, address_mode_v: ImageAddressMode::ClampToEdge,
                                                                          ..ImageSamplerDescriptor::linear() });
        }).load(req.image.clone());
        let owners: Vec<(Entity, GlobalTransform)> = match req.root {
            Some(r) => globals.get(r).ok().map(|g| (r, *g)).into_iter().collect(),
            None => chars.iter().filter(|(_, ch, _)| ch.who == want).map(|(e, _, g)| (e, *g)).collect(),
        };
        for (root, root_at) in owners {
            // the owner's skinned primitives, keyed by MDB mesh index
            let mut parts: Vec<(usize, Entity)> = vec![];
            for e in children.iter_descendants(root) {
                if !skinned.contains(e) || visibility.get(e).is_ok_and(|v| *v == Visibility::Hidden) { continue; }
                let index = parents.get(e).ok().and_then(|p| names.get(p.parent()).ok()).and_then(|n| n.as_str().strip_prefix("role_0_mesh_")).and_then(|i| i.parse().ok());
                parts.push((index.unwrap_or(usize::MAX), e));
            }
            if parts.is_empty() { continue; }
            let targets: Vec<Entity> = if req.at.is_some() {
                // every visible part near the point (the projection box keeps only what lies there)
                parts.iter().map(|p| p.1).collect()
            } else if req.on_player || req.root.is_some() {
                // largest primitive (the body; a grunt's mesh table is not mapped either)
                parts.iter().max_by_key(|(_, e)| skinned.get(*e).ok().and_then(|(m, ..)| meshes.get(&m.0)).map_or(0, |m| m.count_vertices())).map(|p| p.1).into_iter().collect()
            } else {
                let count = parts.iter().filter(|p| p.0 != usize::MAX).count();
                let mut out = vec![];
                for (k, m) in req.meshes.iter().enumerate() {
                    if k == 1 && *m <= 0 { continue; }   // no second mesh
                    let index = if *m < 0 || *m as usize >= count { 0 } else { *m as usize };
                    if let Some((_, e)) = parts.iter().find(|p| p.0 == index) { if !out.contains(e) { out.push(*e); } }
                }
                out
            };
            // pose each target on the CPU
            let mut posed: Vec<(Entity, Vec<Vec3>, Vec<Vec3>)> = vec![];
            for e in targets {
                let Ok((mesh, skin, _)) = skinned.get(e) else { continue; };
                let (Some(mesh), Some(ibp)) = (meshes.get(&mesh.0), bindposes.get(&skin.inverse_bindposes)) else { continue; };
                let mats: Vec<Mat4> = skin.joints.iter().zip(ibp.iter()).map(|(j, b)| globals.get(*j).map_or(Mat4::IDENTITY, |g| g.to_matrix()) * *b).collect();
                let (Some(VertexAttributeValues::Float32x3(pos)), Some(VertexAttributeValues::Uint16x4(ji)), Some(VertexAttributeValues::Float32x4(jw))) =
                    (mesh.attribute(Mesh::ATTRIBUTE_POSITION), mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX), mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)) else { continue; };
                let nrm = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) { Some(VertexAttributeValues::Float32x3(n)) => n.clone(), _ => vec![[0., 1., 0.]; pos.len()] };
                let mut p = Vec::with_capacity(pos.len()); let mut n = Vec::with_capacity(pos.len());
                for i in 0..pos.len() {
                    let m: Mat4 = (0..4).map(|k| mats.get(ji[i][k] as usize).copied().unwrap_or(Mat4::IDENTITY) * jw[i][k]).fold(Mat4::ZERO, |a, b| a + b);
                    p.push(m.transform_point3(Vec3::from_array(pos[i])));
                    n.push(m.transform_vector3(Vec3::from_array(nrm[i])).normalize_or_zero());
                }
                posed.push((e, p, n));
            }
            if posed.is_empty() { continue; }
            // the projection frame: +Z the owner's facing, centred on the struck meshes
            let count: usize = posed.iter().map(|p| p.1.len()).sum();
            let centre = posed.iter().flat_map(|p| p.1.iter()).copied().sum::<Vec3>() / count.max(1) as f32;
            let (_, facing, _) = root_at.to_scale_rotation_translation();
            let (n_axis, side) = match req.at {
                Some((_, face)) => { let n = face.normalize_or(Vec3::Z); (n, n.cross(Vec3::Y).normalize_or(Vec3::X)) }
                None => ((facing * Vec3::Z).normalize_or(Vec3::Z), (facing * Vec3::X).normalize_or(Vec3::X)),
            };
            let u0 = Quat::from_axis_angle(n_axis, if req.upright { std::f32::consts::PI } else { roll }) * side;
            let v_axis = n_axis.cross(u0).normalize_or(Vec3::Y);
            let u_axis = v_axis.cross(n_axis).normalize_or(Vec3::X);
            let origin = match req.at { Some((p, _)) => p, None => centre + side * (req.offset.x + jitter.x) + Vec3::Y * (req.offset.y + jitter.y) };
            let (w, h) = (req.size[0].max(0.02), (req.size[0] * req.size[1].max(0.5)).max(0.02));
            let depth = if req.at.is_some() { req.depth } else { req.depth.max(0.15) };
            for (e, p, n) in posed {
                let Ok((mesh_h, skin, local)) = skinned.get(e) else { continue; };
                let Some(src) = meshes.get(&mesh_h.0) else { continue; };
                let Some(indices) = src.indices() else { continue; };
                let idx: Vec<usize> = indices.iter().collect();
                let uv: Vec<[f32; 2]> = p.iter().map(|q| { let d = *q - origin; [0.5 + d.dot(u_axis) / w, 0.5 - d.dot(v_axis) / h] }).collect();
                let outside = |i: usize| { let d = p[i] - origin; let (a, b, c) = (d.dot(u_axis), d.dot(v_axis), d.dot(n_axis));
                    [a < -w * 0.5, a > w * 0.5, b < -h * 0.5, b > h * 0.5, c < -depth, c > depth] };
                let mut kept: Vec<u32> = vec![]; let mut used = std::collections::HashSet::new();
                for t in idx.chunks_exact(3) {
                    let facing = (n[t[0]] + n[t[1]] + n[t[2]]).dot(n_axis) > 0.;
                    let (o0, o1, o2) = (outside(t[0]), outside(t[1]), outside(t[2]));
                    let culled = (0..6).any(|k| o0[k] && o1[k] && o2[k]);
                    if !facing || culled { continue; }
                    for &i in t { used.insert(i); }
                    if used.len() > MAX_VERTICES { break; }
                    kept.extend(t.iter().map(|&i| i as u32));
                }
                if kept.is_empty() { continue; }
                let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::default());
                for attr in [Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_NORMAL, Mesh::ATTRIBUTE_JOINT_INDEX, Mesh::ATTRIBUTE_JOINT_WEIGHT] {
                    if let Some(values) = src.attribute(attr.clone()) { mesh.insert_attribute(attr, values.clone()); }
                }
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
                mesh.insert_indices(Indices::U32(kept));
                let [r, g, b, a] = req.tint;
                let material = materials.add(StandardMaterial { base_color: Color::srgba(r, g, b, a), base_color_texture: Some(image.clone()), unlit: true,
                    alpha_mode: AlphaMode::Blend, depth_bias: 200., cull_mode: if req.cull { Some(bevy::render::render_resource::Face::Back) } else { None },
                    double_sided: !req.cull, ..default() });
                let Ok(parent) = parents.get(e).map(|p| p.parent()) else { continue; };
                let decal = commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material.clone()), skin.clone(), *local, bevy::light::NotShadowCaster,
                    bevy::camera::visibility::NoFrustumCulling,
                    BloodDecal { on_player: req.on_player, root: req.root, born: time.elapsed_secs(), life: (req.life > 0).then(|| req.life as f32 / 60.), material, alpha: a })).id();
                commands.entity(parent).add_child(decal);
                if std::env::var_os("MW_DECAL_LOG").is_some() { eprintln!("[decal] {:?} {} vertices, {} x {} m, life {} at {:.2}", root, used.len(), w, h, req.life, origin); }
            }
        }
    }
    if std::env::var_os("MW_DECAL_LOG").is_some() { eprintln!("[decal] {n} requests built in {:.1} ms", t0.elapsed().as_secs_f32() * 1000.); }
}

/// Timed stains fade over their last half second and go; life-0 stains stay. A player stain also goes with a swap.
fn age(time: Res<Time>, mut materials: ResMut<Assets<StandardMaterial>>, decals: Query<(Entity, &BloodDecal)>, fades: Query<&Fade>, mut commands: Commands) {
    let now = time.elapsed_secs();
    for (e, d) in &decals {
        if let Some(f) = d.root.and_then(|r| fades.get(r).ok()) {
            if let Some(mut m) = materials.get_mut(&d.material) { m.base_color.set_alpha(d.alpha * f.0); }
            continue;
        }
        let Some(life) = d.life else { continue; };
        let left = life - (now - d.born);
        if left <= 0. { commands.entity(e).despawn(); continue; }
        if left < 0.5 { if let Some(mut m) = materials.get_mut(&d.material) { m.base_color.set_alpha(d.alpha * left / 0.5); } }
    }
}
