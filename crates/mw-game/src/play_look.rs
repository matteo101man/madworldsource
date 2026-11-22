//! Image quality as the recomp renderer gives it (tools/gecko-reference backend-wgpu), which is what makes its frames
//! clean next to mw-play's:
//! - Mip maps. Every c201 TPL page has 4 levels and every pl0030/em0440 page 6, sampled GX_LIN_MIP_LIN (TPL headers:
//!   min filter 5, LOD 0..3 / 0..5); the GLB exports keep only the base level, so the grating and the character
//!   textures minify without filtering (shimmer, broken lines). The chain is rebuilt here by 2x2 averaging [inferred:
//!   the disc's own levels are not decoded].
//! - No tone curve: GX writes texture x color straight to the EFB; Bevy's default TonyMcMapface bends only the
//!   StandardMaterial draws (arena), not the custom ink, so the two drifted apart.
//! - 4x MSAA (EFB_SAMPLE_COUNT 4); the alpha-tested arena uses alpha to coverage so cut-out edges get it too.
//! - Supersampling: the recomp draws the EFB at round(height / 480) times the Wii's 640x528 (2 at 720p) and scales the
//!   copy to the window. Here the 3D camera renders at MW_SSAA times the window (default 2; 1 turns it off) and a 2D
//!   camera draws that image at window size; linear filtering at exactly half size averages each 2x2 block. The HUD
//!   draws on the 2D camera at window resolution.
//!
//! Added, not original: a "modern" look for testing (MW_LOOK=modern|filmic, F3 cycles native -> modern -> filmic).
//! The Wii draws the stage unlit (texture x vertex color); here it is lit: a sun along the game's own character light
//! (play_ink.rs WORLD_LIGHT) with cascaded shadow maps cast by the characters (replacing the blob decals; the stage
//! only receives, since its roofing, signs and scaffolds would shade the whole floor), a cool ambient term (SSAO only on request, MW_LOOK_PARTS=ssao: its per-frame noise flickers without TAA; MSAA stays off, the supersampling stands in). On top (HDR):
//! contrast-adaptive sharpening and a screen-space ink outline; bloom on the bright strokes only on request
//! (MW_LOOK_PARTS=bloom: it flickers frame to frame with sub-pixel highlights)
//! (play_outline.rs); filmic adds AgX tone mapping. The electric fences on both sides of the ring light the stage with
//! cool flickering point lights (one per cluster of c201 arc draws, textures 33), which flare while RinRin is
//! electrocuted (DM_ELECTRIC). The neon and lamp sign sheets glow at night and the strongest sign clusters cast colored
//! light (tools/stage_sign_lights.py). Night (default) or day follows the grade's switch (F2): a cool moon and stronger
//! fences and signs at night, the warm sun by day.
//! Tried and left out: chromatic aberration (washes the frame out on the HDR supersampled target) and depth of field
//! (fine alone, MW_LOOK_PARTS=dof, but black/magenta frames combined with the other passes).
use bevy::{
    anti_alias::contrast_adaptive_sharpening::ContrastAdaptiveSharpening,
    camera::Hdr, light::{CascadeShadowConfigBuilder, GlobalAmbientLight, ShadowFilteringMethod}, pbr::ScreenSpaceAmbientOcclusion,
    post_process::{bloom::{Bloom, BloomPrefilter}, dof::{DepthOfField, DepthOfFieldMode}},
    asset::AssetEvent, camera::RenderTarget, core_pipeline::tonemapping::{DebandDither, Tonemapping},
    image::{ImageSampler, ImageSamplerDescriptor}, prelude::*,
    render::{render_resource::{Extent3d, TextureFormat}, view::Msaa}, ui::IsDefaultUiCamera, window::PrimaryWindow,
};

pub struct LookPlugin;
impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        let look = match std::env::var("MW_LOOK").as_deref() { Ok("modern") => Look::Modern, Ok("filmic") => Look::Filmic, _ => Look::Native };
        app.insert_resource(look).add_systems(Update, (camera, resize, mips, (cycle, apply, focus, arena_light).chain(), flicker, lit_characters));
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Look { Native, Modern, Filmic }
impl Look {
    /// The stage is lit with real lights and shadow maps.
    pub fn lit(self) -> bool { self != Look::Native }
}

/// The modern preset's sun.
#[derive(Component)]
struct Sun;
/// The game's character light direction (toward the light), play_ink.rs.
const WORLD_LIGHT: Vec3 = Vec3::new(0.389, 0.819, 0.422);
/// Lux for the sun and cd/m^2 for the ambient: at Bevy's default exposure (EV100 9.7) a surface facing the sun comes
/// out near its texture color and one in shadow near a third of it [tuned].
const SUN_LUX: f32 = 3000.;
const AMBIENT: f32 = 320.;
/// The electric fence lights: centers of the c201 arc draws (materials 182..204, texture 33) clustered every ~6 m
/// along each side of the ring.
const FENCE_LIGHTS: [[f32; 3]; 7] = [[-6.0, 20.6, -80.0], [-5.8, 21.4, -76.7], [-5.4, 20.1, -68.4], [-2.8, 19.6, -60.4],
                                     [11.7, 19.3, -77.6], [11.6, 20.7, -69.5], [11.6, 20.4, -63.8]];
/// Lumens of one fence light: about 1500 lux on the floor 3 m away at the default exposure [tuned].
const FENCE_LUMENS: f32 = 170_000.;
#[derive(Component)]
struct FenceLight(usize);
/// The c201 sheets the neon and lamp signs are painted on (tools/stage_sign_lights.py).
const SIGN_TEXTURES: [usize; 9] = [1, 7, 10, 19, 22, 23, 24, 25, 29];
/// Emissive gain of a white sign texel at night, in the HDR target's units (StandardMaterial emissive ignores the
/// camera exposure at the default emissive_exposure_weight 0, so 1.0 is already white): a little over white blooms
/// softly. At 160 the lettering was ~160x white and its sub-pixel shimmer pulsed the whole frame orange through the
/// bloom [tuned].
const SIGN_GLOW: f32 = 4.;
/// Lumens of a sign light at night (a quarter by day) and how many of the strongest sign clusters get one [tuned].
const SIGN_LUMENS: f32 = 25_000.;
const SIGN_LIGHTS: usize = 24;
#[derive(Component)]
struct SignLight;
/// tools/stage_sign_lights.py output: sign cluster centroids and their bright-texel colors.
#[derive(serde::Deserialize)]
struct SignSpot { pos: [f32; 3], color: [f32; 3] }
fn sign_spots() -> &'static [SignSpot] {
    static SPOTS: std::sync::OnceLock<Vec<SignSpot>> = std::sync::OnceLock::new();
    SPOTS.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/world/sign-lights.json");
        std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    })
}
/// The native look's character light in colour mode [added, tuned]: lux on the side facing it, cd/m^2 of fill.
const CHAR_LUX: f32 = 3600.;
const CHAR_AMBIENT: f32 = 1000.;

/// [added] Colour mode lights the characters: every skinned mesh's material (the colour models are exported unlit,
/// KHR_materials_unlit) is lit, matte (no metal, no specular), so the key and fill shape them; back in the original
/// black-and-white look they are unlit again, as natively. Kreely's and Jack's ink materials are not StandardMaterials
/// and stay as they are. MW_CHAR_LIGHT=0 keeps them unlit.
pub fn lit_characters(play: Res<super::Play>, q: Query<&MeshMaterial3d<StandardMaterial>, With<bevy::mesh::skinning::SkinnedMesh>>,
                      fresh: Query<(), (With<bevy::mesh::skinning::SkinnedMesh>, Changed<MeshMaterial3d<StandardMaterial>>)>,
                      mut mats: ResMut<Assets<StandardMaterial>>, mut last: Local<Option<bool>>) {
    let lit = play.colored && std::env::var("MW_CHAR_LIGHT").as_deref() != Ok("0");
    if *last == Some(lit) && fresh.is_empty() { return; }
    *last = Some(lit);
    for m in &q {
        let Some(mat) = mats.get(&m.0) else { continue; };
        if mat.unlit != lit { continue; }
        let Some(mut mat) = mats.get_mut(&m.0) else { continue; };
        mat.unlit = !lit;
        if lit { mat.metallic = 0.; mat.perceptual_roughness = 1.; mat.reflectance = 0.; }
    }
}

/// Bevy's default 0.02 leaves the whole stage self-shadowed (acne) at this scale [tuned].
const SUN_BIAS: f32 = 0.5;

fn cycle(keys: Res<ButtonInput<KeyCode>>, mut look: ResMut<Look>) {
    if !keys.just_pressed(KeyCode::F3) { return; }
    *look = match *look { Look::Native => Look::Modern, Look::Modern => Look::Filmic, Look::Filmic => Look::Native };
    info!("[look] {:?}", *look);
}

/// Put the preset's components on the 3D camera (on start and when F3 changes it).
fn night() -> bool { super::grade::native_grade::NIGHT.load(std::sync::atomic::Ordering::Relaxed) }

fn apply(look: Res<Look>, cams: Query<Entity, (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>, With<Ready>)>, fresh: Query<(), Added<Ready>>,
         lights: Query<Entity, Or<(With<Sun>, With<FenceLight>, With<SignLight>)>>, mut last_night: Local<Option<bool>>, mut commands: Commands) {
    let night = night();
    if !look.is_changed() && fresh.is_empty() && *last_night == Some(night) { return; }
    *last_night = Some(night);
    for e in &lights { commands.entity(e).despawn(); }
    if look.lit() {
        let parts = std::env::var("MW_LOOK_PARTS").unwrap_or_default();
        for (i, p) in FENCE_LIGHTS.iter().enumerate().filter(|_| !parts.contains("nofence")) {
            commands.spawn((FenceLight(i), PointLight { intensity: FENCE_LUMENS, range: 9., radius: 0.3, color: Color::srgb(0.55, 0.75, 1.), ..default() },
                            Transform::from_translation(Vec3::from_array(*p))));
        }
        for spot in sign_spots().iter().take(SIGN_LIGHTS).filter(|_| !parts.contains("nosign")) {
            let [r, g, b] = spot.color;
            commands.spawn((SignLight, PointLight { intensity: SIGN_LUMENS * if night { 1. } else { 0.25 }, range: 7., radius: 0.5, color: Color::srgb(r, g, b), ..default() },
                            Transform::from_translation(Vec3::from_array(spot.pos))));
        }
        // night: a cool moon from the side mirrored from the sun (as the night sky in play_sky.rs)
        let (lux, color, toward) = if night { (SUN_LUX * 0.3, Color::srgb(0.62, 0.7, 1.), Vec3::new(-WORLD_LIGHT.x, WORLD_LIGHT.y * 0.8, -WORLD_LIGHT.z)) }
                                   else { (SUN_LUX, Color::srgb(1., 0.96, 0.9), WORLD_LIGHT) };
        commands.spawn((Sun, DirectionalLight { illuminance: lux, shadow_maps_enabled: !std::env::var("MW_LOOK_PARTS").is_ok_and(|p| p.contains("noshadow")), shadow_depth_bias: SUN_BIAS, color, ..default() },
            Transform::default().looking_to(-toward.normalize(), Vec3::Y),
            CascadeShadowConfigBuilder { num_cascades: 3, minimum_distance: 0.1, maximum_distance: 45., first_cascade_far_bound: 8., overlap_proportion: 0.2 }.build()));
        let (ambient, brightness) = if night { (Color::srgb(0.55, 0.6, 1.), AMBIENT * 0.7) } else { (Color::srgb(0.75, 0.82, 1.), AMBIENT) };
        commands.insert_resource(GlobalAmbientLight { color: ambient, brightness, affects_lightmapped_meshes: true });
    } else {
        // [added] the native look leaves the stage unlit, but in colour mode the characters are lit (lit_characters):
        // a key light along the game's own character light (the B&W ink's light 0), no shadow maps (the stage takes
        // none; the blob shadows stay), and an ambient fill so the shade side keeps ~60 % of its colour
        let (lux, color, toward) = if night { (CHAR_LUX * 0.55, Color::srgb(0.72, 0.78, 1.), Vec3::new(-WORLD_LIGHT.x, WORLD_LIGHT.y * 0.8, -WORLD_LIGHT.z)) }
                                   else { (CHAR_LUX, Color::srgb(1., 0.96, 0.9), WORLD_LIGHT) };
        commands.spawn((Sun, DirectionalLight { illuminance: lux, shadow_maps_enabled: false, color, ..default() },
            Transform::default().looking_to(-toward.normalize(), Vec3::Y)));
        let (ambient, brightness) = if night { (Color::srgb(0.6, 0.62, 0.95), CHAR_AMBIENT * 0.75) } else { (Color::srgb(0.86, 0.9, 1.), CHAR_AMBIENT) };
        commands.insert_resource(GlobalAmbientLight { color: ambient, brightness, affects_lightmapped_meshes: true });
    }
    for e in &cams {
        let mut c = commands.entity(e);
        if *look == Look::Native {
            c.remove::<(Hdr, Bloom, DepthOfField, ContrastAdaptiveSharpening, ScreenSpaceAmbientOcclusion, super::outline::Outline, bevy::anti_alias::taa::TemporalAntiAliasing)>()
                .remove::<(bevy::core_pipeline::prepass::DepthPrepass, bevy::core_pipeline::prepass::NormalPrepass)>()
                .insert((Tonemapping::None, Msaa::Sample4));
            continue;
        }
        c.insert((Msaa::Off, ShadowFilteringMethod::Gaussian));
        if std::env::var("MW_LOOK_PARTS").is_ok_and(|p| p.contains("ssao")) { c.insert(ScreenSpaceAmbientOcclusion::default()); }
        // inspection: MW_LOOK_PARTS=bloom,sharp,outline,ssao,dof,taa picks the effects (default: sharp, outline). Bevy's
        // SSAO changes its noise every frame for TAA to resolve; without TAA it reads as flicker. Bloom is off by
        // default: sub-pixel highlights (sign lettering, the additive flares) popping in and out between frames dimmed
        // and lifted the whole frame for single frames every second or so (frame captures: 24 blips in 150 frames
        // with it, none without; TAA made it worse)
        let parts = std::env::var("MW_LOOK_PARTS").unwrap_or_else(|_| "sharp,outline".into());
        if parts.contains("outline") {
            // the ink pass reads the depth and normal prepasses (SSAO keeps them too)
            c.insert((super::outline::Outline::default(), bevy::core_pipeline::prepass::DepthPrepass, bevy::core_pipeline::prepass::NormalPrepass));
        }
        c.insert((Hdr, if *look == Look::Filmic { Tonemapping::AgX } else { Tonemapping::None }));
        if parts.contains("bloom") { c.insert(Bloom { intensity: 0.3, prefilter: BloomPrefilter { threshold: 0.8, threshold_softness: 0.4 }, ..Bloom::NATURAL }); }
        if parts.contains("dof") {
            c.insert(DepthOfField { mode: DepthOfFieldMode::Gaussian, focal_distance: 3.0, aperture_f_stops: 16.0, max_circle_of_confusion_diameter: 12.0, ..default() });
        }
        if parts.contains("sharp") { c.insert(ContrastAdaptiveSharpening { enabled: true, sharpening_strength: 0.45, denoise: false }); }
        if parts.contains("taa") { c.insert(bevy::anti_alias::taa::TemporalAntiAliasing::default()); }
    }
}

/// Depth of field focuses on Jack.
fn focus(play: Res<super::Play>, mut cams: Query<(&GlobalTransform, &mut DepthOfField)>) {
    for (t, mut dof) in &mut cams {
        let d = t.translation().distance(Vec3::from_array(play.fight.player.pos) + Vec3::Y);
        if (dof.focal_distance - d).abs() > 0.01 { dof.focal_distance = d; }
    }
}

/// The 3D camera's offscreen target and the sprite that shows it.
#[derive(Resource)]
struct Supersample { image: Handle<Image>, sprite: Entity }

fn ssaa() -> u32 { if super::vr::on() { return 1; } std::env::var("MW_SSAA").ok().and_then(|v| v.parse().ok()).unwrap_or(2).clamp(1, 4) }

/// The 3D camera once its look is set up.
#[derive(Component)]
struct Ready;

/// Set the camera up once the window has its real size: a target made from a not-yet-sized window is 2x2, and SSAO's
/// depth mip chain then fails validation and blacks out the frame for good.
fn camera(mut cams: Query<(Entity, &mut Camera), (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>, Without<Ready>)>, windows: Query<&Window, With<PrimaryWindow>>,
          mut images: ResMut<Assets<Image>>, mut commands: Commands) {
    for (e, mut cam) in &mut cams {
        let Ok(window) = windows.single() else { continue; };
        if window.physical_size().min_element() == 0 { continue; }
        commands.entity(e).insert((Ready, Tonemapping::None, DebandDither::Disabled));   // MSAA: `apply`
        let scale = ssaa();
        if scale == 1 { continue; }
        let size = window.physical_size() * scale;
        let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
        image.sampler = ImageSampler::linear();
        let image = images.add(image);
        cam.order = -1;
        // (the window's 2D camera below draws the UI: the 3D one gives it up, if a VR build had handed it over)
        commands.entity(e).insert(RenderTarget::from(image.clone())).remove::<IsDefaultUiCamera>();
        let sprite = commands.spawn(Sprite { image: image.clone(), custom_size: Some(window.size()), ..default() }).id();
        commands.spawn((Camera2d, Msaa::Off, Tonemapping::None, DebandDither::Disabled, IsDefaultUiCamera));
        commands.insert_resource(Supersample { image, sprite });
    }
}

/// Keep the offscreen target at MW_SSAA x the window and the sprite at window size.
fn resize(ss: Option<Res<Supersample>>, windows: Query<&Window, With<PrimaryWindow>>, mut images: ResMut<Assets<Image>>, mut sprites: Query<&mut Sprite>) {
    let (Some(ss), Ok(window)) = (ss, windows.single()) else { return; };
    if window.physical_size().min_element() == 0 { return; }
    let want = window.physical_size() * ssaa();
    if images.get(&ss.image).is_some_and(|i| i.size() != want) {
        if let Some(mut image) = images.get_mut(&ss.image) { image.resize(Extent3d { width: want.x, height: want.y, depth_or_array_layers: 1 }); }
    }
    if let Ok(mut sprite) = sprites.get_mut(ss.sprite) {
        if sprite.custom_size != Some(window.size()) { sprite.custom_size = Some(window.size()); }
    }
}

/// GX mip levels per model (TPL max LOD + 1).
fn levels(path: &str) -> Option<u32> {
    // the sign glow masks too: unfiltered, bright lettering shimmers as the camera moves
    if path.contains("world/emissive/") { return Some(6); }
    if !path.ends_with(".glb") { return None; }
    Some(if path.contains("world/") { 4 } else { 6 })
}

fn mips(mut events: MessageReader<AssetEvent<Image>>, assets: Res<AssetServer>, mut images: ResMut<Assets<Image>>) {
    for ev in events.read() {
        let AssetEvent::Added { id } = ev else { continue; };
        // glTF images are labeled sub-assets of the .glb
        let Some(want) = assets.get_path(*id).and_then(|p| levels(&p.path().to_string_lossy())) else { continue; };
        let Some(mut image) = images.get_mut(*id) else { continue; };
        if image.texture_descriptor.mip_level_count > 1 || !matches!(image.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm) { continue; }
        let (w, h) = (image.texture_descriptor.size.width, image.texture_descriptor.size.height);
        let Some(base) = image.data.clone() else { continue; };
        let (mut data, mut level, mut lw, mut lh, mut count) = (base.clone(), base, w, h, 1);
        while count < want && (lw > 1 || lh > 1) {
            let (nw, nh) = ((lw / 2).max(1), (lh / 2).max(1));
            let mut next = vec![0u8; (nw * nh * 4) as usize];
            for y in 0..nh {
                for x in 0..nw {
                    for c in 0..4 {
                        let px = |sx: u32, sy: u32| level[((sy.min(lh - 1) * lw + sx.min(lw - 1)) * 4 + c) as usize] as u32;
                        next[((y * nw + x) * 4 + c) as usize] = ((px(2 * x, 2 * y) + px(2 * x + 1, 2 * y) + px(2 * x, 2 * y + 1) + px(2 * x + 1, 2 * y + 1) + 2) / 4) as u8;
                    }
                }
            }
            data.extend_from_slice(&next);
            (level, lw, lh, count) = (next, nw, nh, count + 1);
        }
        image.data = Some(data);
        image.texture_descriptor.mip_level_count = count;
        // the GLB samplers already ask for LINEAR_MIPMAP_LINEAR; keep their wrap modes
        if let ImageSampler::Descriptor(d) = &mut image.sampler {
            d.mipmap_filter = bevy::image::ImageFilterMode::Linear;
        } else {
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: bevy::image::ImageAddressMode::Repeat,
                address_mode_v: bevy::image::ImageAddressMode::Repeat, ..ImageSamplerDescriptor::linear() });
        }
    }
}

/// Light the stage materials under the lit presets (back to unlit for native). The additive glows stay unlit.
fn arena_light(look: Res<Look>, arenas: Query<Entity, With<super::ArenaVariant>>, children: Query<&Children>,
               mats: Query<&MeshMaterial3d<StandardMaterial>>, mut materials: ResMut<Assets<StandardMaterial>>, assets: Res<AssetServer>,
               mut done: Local<Option<(Look, bool)>>, mut commands: Commands) {
    let night = night();
    if *done == Some((*look, night)) { return; }
    let found: Vec<_> = arenas.iter().flat_map(|a| children.iter_descendants(a)).filter_map(|e| mats.get(e).ok().map(|m| (e, m.0.clone()))).collect();
    if found.len() < 2 * 235 { return; }
    for (e, h) in found {
        commands.entity(e).insert(bevy::light::NotShadowCaster);
        let Some(mut m) = materials.get_mut(&h) else { return; };
        if m.alpha_mode == AlphaMode::Add { continue; }
        m.unlit = !look.lit();
        // GX winds fronts clockwise, so from above the floor is a glTF back face and a double-sided material would flip
        // its stored normal downward: keep the stored normals and draw both sides
        m.double_sided = false;
        m.cull_mode = None;
        m.perceptual_roughness = 0.85;
        m.reflectance = 0.25;
        // the sign sheets glow, backlit, at night under the lit looks
        let sign = m.base_color_texture.as_ref().and_then(|t| assets.get_path(t))
            .and_then(|p| p.label().and_then(|l| l.strip_prefix("Texture")).and_then(|i| i.parse::<usize>().ok()))
            .filter(|i| SIGN_TEXTURES.contains(i))
            // (c201's sheet numbers: other rooms' textures with those numbers are not its signs)
            .filter(|_| super::boss_room().is_none());
        if let Some(sheet) = sign {
            // lettering and lamps only: the sheets' paper stays dark (tools/stage_sign_lights.py masks)
            let on = look.lit() && night && !std::env::var("MW_LOOK_PARTS").is_ok_and(|p| p.contains("noglow"));
            m.emissive_texture = on.then(|| assets.load(format!("native/world/emissive/texture-{sheet:03}.png")));
            let glow = std::env::var("MW_SIGN_GLOW").ok().and_then(|v| v.parse().ok()).unwrap_or(SIGN_GLOW);
            m.emissive = if on { LinearRgba::rgb(glow, glow, glow) } else { LinearRgba::BLACK };
        }
    }
    *done = Some((*look, night));
}

/// The fence lights hum (each on its own phase) and flare while RinRin is electrocuted.
fn flicker(time: Res<Time>, play: Res<super::Play>, mut lights: Query<(&FenceLight, &mut PointLight)>) {
    let t = time.elapsed_secs();
    let shock = if play.fight.rin.state == mw_sim::fight::rin_state::DM_ELECTRIC { 3.5 } else { 1. } * if night() { 1.4 } else { 0.6 };
    for (f, mut light) in &mut lights {
        let k = f.0 as f32 * 1.7;
        // a slow hum: anything near the frame rate reads as frame-to-frame flicker
        let buzz = 0.9 + 0.07 * (t * 2.1 + k).sin() + 0.03 * (t * 5.3 + k * 2.).sin();
        light.intensity = FENCE_LUMENS * buzz * shock;
    }
}
