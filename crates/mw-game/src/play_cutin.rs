//! The clash cut-in: the comic panels before a power struggle (the boss's portrait upper left, Jack's lower right, speed
//! lines, "VROROOOWW" in the brush capitals over the darkened frame). Native: the player's cockpit cut-in widget
//! (fn_8022B374 requests it, its update fn_8022B184 creates it) plays **program 11 of the room's idpac** (resource 1) on
//! layer 12 in a boss fight, and sets every bit of the global stop mask but 5 and 21 while it plays: the world stands
//! still until widget 1's animation ends (frame 40), then the program goes and the world runs on. Requested at:
//! RinRin's clash entry (802956B4 phase 0), Elise's pull-out when his pl0030_0466 reaches frame 25 (802A03A0 phase 1),
//! the Baron's clash grab (8029D350 phase 0, pl0030_0480); never in Mathilda's bat combo (8029F80C).
//! [inferred] the portrait panels' clip windows (kind-7 widgets, native_ui::sprites_ex); SE 35 (10000035) is requested
//! but the freeze it runs under mutes it (stop bit 19), so the port plays none.
use bevy::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use super::pause::native_ui::{self, UiSource};
use mw_sim::ui::UiScene;

const PROGRAM: u32 = 11;
/// The fixed 30 Hz clock it runs on while the fight is frozen.
const HZ: f32 = 30.;

static REQUEST: AtomicBool = AtomicBool::new(false);
/// Ask for the cut-in (the boss modules call this at their native trigger).
pub fn request() { REQUEST.store(true, Relaxed); }

#[derive(Resource, Default)]
pub struct Cutin { source: Option<UiSource>, scene: Option<(UiScene, f32, f32)> }

/// The FixedUpdate fight chain runs only while no cut-in plays (the native stop mask).
pub fn world_running(c: Res<Cutin>) -> bool { c.scene.is_none() }

#[derive(Component)]
struct CutinNode;

/// The room's idpac: RinRin c201, Elise c303, the Baron c503 (and its Mathilda variant, which has no cut-in).
fn pack() -> Option<&'static str> {
    if super::elise::on() { Some("idpac_c303_us") } else if super::baron::on() { Some("idpac_c503_us") }
    else if super::street::start().is_some() { None } else { Some("idpac_c201_us") }
}

pub struct CutinPlugin;
impl Plugin for CutinPlugin {
    fn build(&self, app: &mut App) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = pack().filter(|p| root.join("assets/native/ui").join(p).join("animation.idt").exists()).map(|p| UiSource::load(&root, p));
        app.insert_resource(Cutin { source, scene: None }).add_systems(Update, (advance, present).chain());
    }
}

/// Update: start a requested cut-in, run it on its own 30 Hz clock (real time), end it after widget 1's animation.
fn advance(mut c: ResMut<Cutin>, real: Res<Time<Real>>, pause: Res<super::pause::Pause>, mut commands: Commands) {
    if REQUEST.swap(false, Relaxed) && c.scene.is_none() {
        if std::env::var("MW_CUTIN").as_deref() != Ok("0") {
            if let Some(src) = c.source.as_ref() {
                let s = src.scene(PROGRAM);
                let end = s.track_end(1).max(1.);
                if std::env::var_os("MW_CUTIN_LOG").is_some() { eprintln!("[cutin] program {PROGRAM}: {} frames", end); }
                c.scene = Some((s, 0., end));
            }
        }
    }
    if pause.open { return; }
    let Some((scene, t, end)) = c.scene.as_mut() else { return; };
    let dt = real.delta_secs() * HZ;
    // inspection: MW_CUTIN_SHOTS="frame:png;..." screenshots the cut-in at its own frames (the fight's ticks stand still)
    if let Ok(list) = std::env::var("MW_CUTIN_SHOTS") {
        for item in list.split(';') {
            let Some((f, path)) = item.split_once(':') else { continue; };
            let Ok(f) = f.parse::<f32>() else { continue; };
            if *t < f && *t + dt >= f { commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(std::path::PathBuf::from(path))); }
        }
    }
    scene.advance(dt); *t += dt;
    if *t >= *end { c.scene = None; }
}

fn present(c: Res<Cutin>, hud: Res<super::hud_ui::Hud>, assets: Res<AssetServer>, windows: Query<&Window>, nodes: Query<Entity, With<CutinNode>>,
           mut commands: Commands) {
    for e in &nodes { commands.entity(e).despawn(); }
    let (Some((scene, _, _)), Some(source), Ok(window)) = (c.scene.as_ref(), c.source.as_ref(), windows.single()) else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    // (layer 12, over the HUD; the cut-in's own darkening quad w0 greys the frozen frame)
    for sprite in native_ui::sprites_ex(scene, &source.layout, &hud.source().layout, &assets, viewport, 9100, None, true) {
        commands.spawn((sprite, CutinNode));
    }
}
