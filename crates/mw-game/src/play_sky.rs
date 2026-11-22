//! Added, not original: day and night skyboxes. The Wii clears the arena's open sky to black, and the recomp's color grade
//! paints a gradient into that black (native_grade.rs); here a real cubemap stands behind the stage instead, built
//! procedurally at startup. Night (the default, as the grade): an indigo zenith over a violet city glow, stars and a
//! haloed moon. Day: a blue gradient, soft clouds and a sun glow along the game's character light. It follows the
//! grade's night/day switch (F2, MW_TIME=day) and shows with the color mode, like the grade; C (color off) keeps the
//! native black.
use bevy::{
    asset::RenderAssetUsages, core_pipeline::Skybox, prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension},
};
use std::sync::atomic::Ordering;

/// Pixels per cube face.
const FACE: u32 = 384;
/// Skybox luminance (cd/m^2): with Bevy's default exposure (EV100 9.7) a texel shows at about its own value.
const BRIGHTNESS: f32 = 1000.;
/// Toward the game's character light (play_ink.rs WORLD_LIGHT): the day sun and, mirrored, the night moon.
const WORLD_LIGHT: Vec3 = Vec3::new(0.389, 0.819, 0.422);

#[derive(Resource)]
struct Skies { night: Handle<Image>, day: Handle<Image> }

pub struct SkyPlugin;
impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, build).add_systems(Update, show);
    }
}

fn build(mut images: ResMut<Assets<Image>>, mut commands: Commands) {
    commands.insert_resource(Skies { night: images.add(cubemap(night)), day: images.add(cubemap(day)) });
}

/// Put the sky matching the grade's night/day on the 3D camera while the color mode is on.
fn show(play: Res<super::Play>, skies: Option<Res<Skies>>, cams: Query<(Entity, Option<&Skybox>), With<Camera3d>>, mut commands: Commands) {
    let Some(skies) = skies else { return; };
    let night = super::grade::native_grade::NIGHT.load(Ordering::Relaxed);
    let want = play.colored.then(|| if night { skies.night.clone() } else { skies.day.clone() });
    for (e, have) in &cams {
        match (&want, have) {
            (Some(image), Some(sky)) if sky.image.as_ref() == Some(image) => {}
            (Some(image), _) => { commands.entity(e).insert(Skybox { image: Some(image.clone()), brightness: BRIGHTNESS, ..default() }); }
            (None, Some(_)) => { commands.entity(e).remove::<Skybox>(); }
            (None, None) => {}
        }
    }
}

/// Six faces (+X, -X, +Y, -Y, +Z, -Z) from a direction -> sRGB color function.
fn cubemap(sky: fn(Vec3) -> [f32; 3]) -> Image {
    let mut data = Vec::with_capacity((FACE * FACE * 6 * 4) as usize);
    for face in 0..6 {
        for y in 0..FACE {
            for x in 0..FACE {
                let (u, v) = ((x as f32 + 0.5) / FACE as f32 * 2. - 1., (y as f32 + 0.5) / FACE as f32 * 2. - 1.);
                let dir = match face {
                    0 => Vec3::new(1., -v, -u), 1 => Vec3::new(-1., -v, u), 2 => Vec3::new(u, 1., v),
                    3 => Vec3::new(u, -1., -v), 4 => Vec3::new(u, -v, 1.), _ => Vec3::new(-u, -v, -1.),
                }.normalize();
                let c = sky(dir);
                data.extend(c.iter().map(|c| (c.clamp(0., 1.) * 255. + 0.5) as u8));
                data.push(255);
            }
        }
    }
    let mut image = Image::new(Extent3d { width: FACE, height: FACE * 6, depth_or_array_layers: 1 }, TextureDimension::D2, data,
                               TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    let _ = image.reinterpret_stacked_2d_as_array(6);
    image.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::Cube), ..default() });
    image
}

fn hash(p: Vec3) -> f32 { ((p.dot(Vec3::new(127.1, 311.7, 74.7))).sin() * 43758.547).fract().abs() }
fn noise(p: Vec3) -> f32 {
    let (i, f) = (p.floor(), p.fract_gl());
    let f = f * f * (3. - 2. * f);
    let c = |dx: f32, dy: f32, dz: f32| hash(i + Vec3::new(dx, dy, dz));
    let x00 = c(0., 0., 0.) + (c(1., 0., 0.) - c(0., 0., 0.)) * f.x; let x10 = c(0., 1., 0.) + (c(1., 1., 0.) - c(0., 1., 0.)) * f.x;
    let x01 = c(0., 0., 1.) + (c(1., 0., 1.) - c(0., 0., 1.)) * f.x; let x11 = c(0., 1., 1.) + (c(1., 1., 1.) - c(0., 1., 1.)) * f.x;
    let y0 = x00 + (x10 - x00) * f.y; let y1 = x01 + (x11 - x01) * f.y;
    y0 + (y1 - y0) * f.z
}
fn fbm(p: Vec3) -> f32 { (0..5).map(|o| noise(p * 2f32.powi(o)) * 0.5f32.powi(o + 1)).sum() }
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] { [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t] }

fn night(d: Vec3) -> [f32; 3] {
    let h = d.y.max(0.);
    let mut c = mix([0.30, 0.13, 0.30], [0.05, 0.05, 0.16], h.powf(0.45));   // violet city glow -> indigo zenith
    if d.y < 0. { c = mix(c, [0.08, 0.04, 0.10], (-d.y * 3.).min(1.)); }
    // stars: sparse cells, fainter toward the glowing horizon
    let cell = (d * 260.).floor();
    let s = hash(cell);
    if s > 0.9965 && d.y > 0.08 { let b = (s - 0.9965) / 0.0035 * (0.4 + 0.6 * h); c = mix(c, [0.95, 0.95, 1.], b); }
    // haze bands
    c = mix(c, [0.22, 0.12, 0.28], (fbm(d * 3.) - 0.45).max(0.) * 0.6 * (1. - h));
    let moon = Vec3::new(-WORLD_LIGHT.x, WORLD_LIGHT.y * 0.8, -WORLD_LIGHT.z).normalize();
    let m = d.dot(moon);
    c = mix(c, [0.55, 0.6, 0.8], ((m - 0.95) / 0.05).clamp(0., 1.).powi(3) * 0.35);   // halo
    if m > 0.9994 { c = mix([0.93, 0.92, 0.85], [0.75, 0.75, 0.72], fbm(d * 400.)); }   // disc with maria
    c
}

fn day(d: Vec3) -> [f32; 3] {
    let h = d.y.max(0.);
    let mut c = mix([0.78, 0.86, 0.93], [0.25, 0.48, 0.85], h.powf(0.5));
    if d.y < 0. { c = mix(c, [0.55, 0.58, 0.62], (-d.y * 3.).min(1.)); }
    let sun = WORLD_LIGHT.normalize();
    let s = d.dot(sun);
    c = mix(c, [1., 0.97, 0.88], ((s - 0.9) / 0.1).clamp(0., 1.).powi(4) * 0.6);
    if s > 0.9992 { c = [1., 1., 0.96]; }
    // clouds on a dome
    if d.y > 0.02 {
        let p = Vec3::new(d.x / (d.y + 0.15), 0., d.z / (d.y + 0.15)) * 1.6;
        let n = ((fbm(p + Vec3::new(3., 0., 7.)) - 0.48) * 4.).clamp(0., 1.) * (h * 4.).min(1.);
        c = mix(c, [0.97, 0.97, 0.98], n * 0.85);
    }
    c
}
