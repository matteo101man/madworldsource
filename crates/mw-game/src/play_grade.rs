//! The recomp frontend's added color grade (native_grade.rs: same WGSL and night/day presets) as a mw-play post-process.
//! As in the recomp it is part of the color mode: C (color off) bypasses it, F2 switches night/day (MW_TIME=day starts
//! in day). Added art direction, not original lighting (research/LOST-PARADISE-LOOK.md).
use bevy::{
    asset::uuid_handle, core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin}, prelude::*,
    render::{extract_component::ExtractComponent, render_resource::ShaderType, RenderApp}, shader::ShaderRef,
};
use std::sync::atomic::Ordering;

#[path = "native_grade.rs"]
#[allow(dead_code)]
pub(super) mod native_grade;

const SHADER: Handle<Shader> = uuid_handle!("2d8b6f91-41c7-4b0a-a3e5-7c19d4e2f6a1");

/// The grade uniform (shadow+gain, light, haze+strength, saturation/glow/vignette/linear, sky top, sky horizon).
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
#[extract_app(RenderApp)]
pub struct Grade { shadow: Vec4, light: Vec4, haze: Vec4, p: Vec4, sky_top: Vec4, sky_low: Vec4 }
impl FullscreenMaterial for Grade {
    fn fragment_shader() -> ShaderRef { ShaderRef::Handle(SHADER) }
}
/// The same grade over the UI layer (the supersampling's 2D camera draws the HUD and menus after the 3D grade has
/// run): put on that camera while the pause menu is open in color mode, so the pause pages and the black backdrop
/// are graded as the recomp grades its whole frame, menus included (native_main.rs blit_native) [recomp].
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
#[extract_app(RenderApp)]
pub struct UiGrade { shadow: Vec4, light: Vec4, haze: Vec4, p: Vec4, sky_top: Vec4, sky_low: Vec4 }
impl FullscreenMaterial for UiGrade {
    fn fragment_shader() -> ShaderRef { ShaderRef::Handle(SHADER) }
    fn schedule() -> impl bevy::ecs::schedule::ScheduleLabel + Clone { bevy::core_pipeline::Core2d }
    fn schedule_configs(system: bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::BoxedSystem>) -> bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::BoxedSystem> {
        system.after(bevy::ui_render::render_pass::ui_pass).before(bevy::core_pipeline::upscaling::upscaling)
    }
}
impl From<Grade> for UiGrade {
    fn from(g: Grade) -> Self { Self { shadow: g.shadow, light: g.light, haze: g.haze, p: g.p, sky_top: g.sky_top, sky_low: g.sky_low } }
}

impl Grade {
    /// The target is sRGB (FullscreenMaterial), so samples arrive linear: the shader's linear flag is set.
    fn new(night: bool) -> Self {
        let mut v = native_grade::preset(night, true);
        // mw-play draws a real sky (play_sky.rs): no painting of the black upper frame, which here is ink (black hair,
        // black clothes) turned sky blue
        v[19] = 0.;
        if !night {
            // daylight retuned for the lit, skyboxed frame [added]: true blacks (the lift made every shadow a brown
            // grey), no extra gain (it blew the sunlit street out), gentler saturation and glare, and almost no haze:
            // the haze is a wash over the top of the screen, which tinted every head and shoulders pale blue
            let shadow = [0.035, 0.032, 0.03];
            v[..3].copy_from_slice(&shadow);
            v[3] = 1.0;
            v[4..7].copy_from_slice(&[1.0, 0.975, 0.94]);
            v[11] = 0.02;
            v[12] = 1.18; v[13] = 0.12; v[14] = 0.22;
        }
        let q = |i: usize| Vec4::new(v[i], v[i + 1], v[i + 2], v[i + 3]);
        Self { shadow: q(0), light: q(4), haze: q(8), p: q(12), sky_top: q(16), sky_low: q(20) }
    }
}

pub struct GradePlugin;
impl Plugin for GradePlugin {
    fn build(&self, app: &mut App) {
        native_grade::init_from_env();
        let _ = app.world_mut().resource_mut::<Assets<Shader>>().insert(SHADER.id(), Shader::from_wgsl(native_grade::SHADER, "mw-game/native_grade.wgsl"));
        app.add_plugins((FullscreenMaterialPlugin::<Grade>::default(), FullscreenMaterialPlugin::<UiGrade>::default())).add_systems(Update, toggle);
    }
}

/// Grade on the 3D camera while the color mode is on (and on the UI camera while paused); F2 flips night/day.
fn toggle(play: Res<super::Play>, pause: Res<super::pause::Pause>, keys: Res<ButtonInput<KeyCode>>, cams: Query<(Entity, Option<&Grade>), (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>,
          ui: Query<(Entity, Option<&UiGrade>), (With<Camera2d>, With<bevy::ui::IsDefaultUiCamera>)>, mut commands: Commands) {
    if keys.just_pressed(KeyCode::F2) {
        let night = !native_grade::NIGHT.load(Ordering::Relaxed);
        native_grade::NIGHT.store(night, Ordering::Relaxed);
        info!("[grade] {}", if night { "night" } else { "day" });
    }
    let want = play.colored.then(|| Grade::new(native_grade::NIGHT.load(Ordering::Relaxed)));
    for (e, have) in &cams {
        match (want, have) {
            (Some(g), None) => { commands.entity(e).insert(g); }
            (Some(g), Some(h)) if h.p != g.p || h.sky_top != g.sky_top => { commands.entity(e).insert(g); }
            (None, Some(_)) => { commands.entity(e).remove::<Grade>(); }
            _ => {}
        }
    }
    let want = want.filter(|_| pause.open).map(UiGrade::from);
    for (e, have) in &ui {
        match (want, have) {
            (Some(g), None) => { commands.entity(e).insert(g); }
            (Some(g), Some(h)) if h.p != g.p || h.sky_top != g.sky_top => { commands.entity(e).insert(g); }
            (None, Some(_)) => { commands.entity(e).remove::<UiGrade>(); }
            _ => {}
        }
    }
}
