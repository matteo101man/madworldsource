//! Character shadows: the original's jkShadow blob decal (core2 jkShadow class, fn_80263EAC..fn_8026461C), not a shadow
//! map. The arena receives nothing; the 5-stage map compare in the recomp GX dump is the characters' own ink program.
//! - create__8jkShadowFP4cObjffi(obj, size, length, bone -1 = the object's own position): RinRin through the enemy
//!   init fn_801B71C8 (table 0x80559DB0 +192/+196): size 0.5, length 4.0; Jack through the player-side init
//!   fn_802705DC (0x80569070 +3844/+3848, the same values as fn_8009890C / fn_802CCA00): size 0.5 x object scale
//!   (+0x7C, 1 here), length 2.0.
//! - trans__8jkShadowFv: ray from the object position down `length` (fn_800ABAA8 against the stage collision); no hit, no
//!   shadow. At the hit point a quad of half size 1.5 x size x (1 - (d / length)^4) (d = height above the hit), turned
//!   onto the surface normal and lifted 0.055 along it (fn_800BA4B0; the extra z turn is 0 on floors with n.y > 0.999).
//! - draw fn_802644BC, pass 4 of fn_8025BFCC: one TEV stage, REPLACE with texmap 1 = jkShadow (clamp) at texcoord
//!   (u, v - 0.14), indirect-warped by texmap 0 = jkNoise (repeat) at world (x, z) x 1.5; GXSetIndTexMtx(ITM_0,
//!   [[1.5, 0, *], [0, 1.5, -0.14]], exp -3), ITF_8, bias ST (-128), so S = noise alpha, T = noise intensity (0) and
//!   U = 0. Alpha test GREATER 64, no blend: a solid black, jagged patch over the grating's holes.
//! Inferred: the warp strength. 1.5 x 2^-3 texels per indirect step (the GX scale as Dolphin applies it) tears the blob
//! into streaks and moves it off the character; a quarter of that matches the captures (evidence/native-look-capture-2).
use bevy::{
    asset::{uuid_handle, RenderAssetUsages}, image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    mesh::{Indices, PrimitiveTopology}, prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
    transform::TransformSystems,
};

const SHADER: Handle<Shader> = uuid_handle!("8c3e2a17-5d4b-4f8e-9a61-2b7c0e4d9f35");
/// trans: quad half size factor (0x805CB250) and lift along the normal (0x80565688).
const SCALE: f32 = 1.5;
const LIFT: f32 = 0.055;

const WGSL: &str = r#"
// material bind group 3 (bevy_pbr MATERIAL_BIND_GROUP_INDEX); the #{} shader def is not expanded in a runtime WGSL string,
// and the inputs follow bevy_pbr forward_io::VertexOutput's locations (as play_ink.rs)
struct In { @builtin(position) position: vec4<f32>, @location(0) world_position: vec4<f32>, @location(1) world_normal: vec3<f32>, @location(2) uv: vec2<f32> }
@group(3) @binding(0) var shadow_tex: texture_2d<f32>;
@group(3) @binding(1) var shadow_samp: sampler;
@group(3) @binding(2) var noise_tex: texture_2d<f32>;
@group(3) @binding(3) var noise_samp: sampler;

// texels of jkShadow per indirect step [inferred, see play_shadow.rs]
const WARP: f32 = 1.5 * 0.125 * 0.25;

@fragment
fn fragment(in: In) -> @location(0) vec4<f32> {
  // indirect stage: ITF_8 with bias ST, S = alpha, T = blue (the IA8 intensity)
  let n = textureSample(noise_tex, noise_samp, in.world_position.xz * 1.5);
  let ind = vec2<f32>(n.a, n.b) * 255.0 - 128.0;
  let a = textureSample(shadow_tex, shadow_samp, in.uv + vec2<f32>(0.0, -0.14) + ind * WARP / 64.0).a;
  if a * 255.0 <= 64.0 { discard; }
  return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#;

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct BlobMaterial {
    #[texture(0)] #[sampler(1)] shadow: Handle<Image>,
    #[texture(2)] #[sampler(3)] noise: Handle<Image>,
}
impl Material for BlobMaterial {
    fn fragment_shader() -> ShaderRef { ShaderRef::Handle(SHADER) }
    /// Drawn after the opaque stage (pass 4 follows the world); every kept texel is opaque black.
    fn alpha_mode(&self) -> AlphaMode { AlphaMode::Blend }
    fn specialize(_: &bevy::pbr::MaterialPipeline, descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
                  _: &bevy::mesh::MeshVertexBufferLayoutRef, _: bevy::pbr::MaterialPipelineKey<Self>) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// A character root that drops a jkShadow (create__8jkShadowFP4cObjffi size and ray length).
#[derive(Component, Clone, Copy)]
pub struct Caster { size: f32, length: f32 }
impl Caster {
    pub const ENEMY: Self = Self { size: 0.5, length: 4.0 };
    pub const PLAYER: Self = Self { size: 0.5, length: 2.0 };
}
#[derive(Component)]
struct Blob(Entity);
#[derive(Resource)]
struct BlobAssets { mesh: Handle<Mesh>, material: Handle<BlobMaterial> }

pub struct ShadowPlugin;
impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        let _ = app.world_mut().resource_mut::<Assets<Shader>>().insert(SHADER.id(), Shader::from_wgsl(WGSL, "mw-game/play_shadow.wgsl"));
        app.add_plugins(MaterialPlugin::<BlobMaterial>::default()).add_systems(Startup, load)
            .add_systems(PostUpdate, (spawn, place).chain().before(TransformSystems::Propagate));
    }
}

fn load(assets: Res<AssetServer>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<BlobMaterial>>, mut commands: Commands) {
    let texture = |path: &'static str, wrap: ImageAddressMode| assets.load_builder().with_settings(move |s: &mut ImageLoaderSettings| {
        s.is_srgb = false;
        s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: wrap, address_mode_v: wrap, ..ImageSamplerDescriptor::linear() });
    }).load(path);
    let material = materials.add(BlobMaterial { shadow: texture("native/shadow/jkShadow.png", ImageAddressMode::ClampToEdge),
                                                noise: texture("native/shadow/jkNoise.png", ImageAddressMode::Repeat) });
    // fn_802644BC's corners in the surface frame: u along +x, v along +z (local y maps to world -z under the -90 deg
    // pitch), as a unit quad scaled per frame
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[-1., 0., -1.], [1., 0., -1.], [1., 0., 1.], [-1., 0., 1.]])
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]])
        .with_inserted_indices(Indices::U16(vec![0, 2, 1, 0, 3, 2]));
    commands.insert_resource(BlobAssets { mesh: meshes.add(mesh), material });
}

fn spawn(casters: Query<Entity, Added<Caster>>, blob: Option<Res<BlobAssets>>, mut commands: Commands) {
    let Some(blob) = blob else { return; };
    for e in &casters {
        commands.spawn((Mesh3d(blob.mesh.clone()), MeshMaterial3d(blob.material.clone()), Transform::default(), Visibility::Hidden, Blob(e)));
    }
}

/// trans__8jkShadowFv, every frame after the characters are posed. The intro leaves the actor roots at the scene origin
/// and places the actors through the skeleton, so there the first native bone stands in for the object position (its
/// height above the floor shrinks the blob by under 1 %).
fn place(mut blobs: Query<(&Blob, &mut Transform, &mut Visibility)>, casters: Query<(&Transform, &Visibility, &super::Character, &Caster), Without<Blob>>,
         bones: Query<&GlobalTransform>, arena: Res<super::Arena>, intro: Res<super::intro::Intro>, look: Res<super::look::Look>) {
    for (blob, mut t, mut vis) in &mut blobs {
        *vis = Visibility::Hidden;
        if look.lit() { continue; }   // the lit look casts real shadow maps (play_look.rs)
        let Ok((root, shown, ch, caster)) = casters.get(blob.0) else { continue; };
        if *shown == Visibility::Hidden { continue; }
        let at = if intro.active() {
            let Some(bone) = ch.joints.as_ref().and_then(|j| j.first()).and_then(|&j| bones.get(j).ok()) else { continue; };
            bone.translation()
        } else { root.translation };
        let Some((hit, normal)) = ground(&arena.0, at, caster.length) else { continue; };
        let d = (at - hit).length() / caster.length;
        let half = SCALE * caster.size * (1. - d.min(1.).powi(2).powi(2));
        if half <= 0. { continue; }
        *t = Transform::from_translation(hit + normal * LIFT).with_rotation(Quat::from_rotation_arc(Vec3::Y, normal)).with_scale(Vec3::splat(half));
        *vis = Visibility::Visible;
    }
}

/// fn_800ABAA8 stand-in: the nearest col.hkx surface within `length` below `pos` (cast from slightly above, since the
/// actors stand on the surface itself), with its upward normal.
fn ground(world: &mw_data::collision::CollisionWorld, pos: Vec3, length: f32) -> Option<(Vec3, Vec3)> {
    const ABOVE: f64 = 0.3;
    let o = [pos.x as f64, pos.y as f64 + ABOVE, pos.z as f64];
    let hits = mw_sim::collision::raw_surface_ray(world, o, [0., -1., 0.], length as f64 + ABOVE, Some("col.hkx")).ok()?;
    let h = hits.iter().min_by(|a, b| a.distance.total_cmp(&b.distance))?;
    let n = Vec3::new(h.normal[0] as f32, h.normal[1] as f32, h.normal[2] as f32).normalize_or(Vec3::Y);
    Some((Vec3::new(pos.x, (o[1] - h.distance) as f32, pos.z), if n.y < 0. { -n } else { n }))
}
