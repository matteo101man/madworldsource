//! Kreely's pistol on the HUD (added use of native UI; tools/stage_gun_ui.py stages the art):
//! - the native weapon window, common program 37 (cIdWeaponGimmick under cIdWeaponWindow, bottom right), with the
//!   pistol in its picture slot and its gold gauge (widgets 10/11) as the magazine;
//! - the native "x N" counter, common program 57 (cIdContinueItem: face icon, x, two group-15 digits at widgets 10/11),
//!   its face icon hidden, the rounds left in the magazine as the number;
//! - while aiming, cIdTargetSight's art (UV group 21, absent from core_us): the ring sight, the locked sight when the
//!   lock-on holds RinRin, and the RELOAD! banner during a reload or on an empty magazine.
//! Shown only for Kreely, with the fight HUD (not in the intro or the pause).
use bevy::prelude::*;
use mw_sim::ui::UiScene;
use super::pause::native_ui;

/// Program 37's chainsaw pictures (widgets 21: the 42x20 icon and 22: the 83x30 arm; MW_GUN_HUD_DUMP=1 lists every
/// widget). Both are hidden: the arm's slot runs under the window's cog (widget 20, 22 px at (577, 324)), which suits
/// the chainsaw but buries a pistol, so the pistol is drawn above the window, muzzle out to the right of the cog.
const WINDOW_HIDDEN: [u32; 2] = [21, 22];
const WINDOW_PISTOL_AT: [f32; 2] = [611., 336.];
const WINDOW_PISTOL_SIZE: [f32; 2] = [44., 16.];
/// Program 57's face icon: UV group 22 sprite 17 (its widget id is the shared 0, so it is hidden by index).
const COUNTER_FACE: [u16; 2] = [22, 17];
const GAUGE: [u32; 2] = [10, 11];
/// Frames the weapon window animates as it opens before it is held still.
const WINDOW_SETTLE: u32 = 30;
const DIGITS: u32 = 10;

#[derive(Resource)]
pub struct GunHud { window: UiScene, counter: UiScene, pistol: Handle<Image>, pistol_color: Handle<Image>, ring: Handle<Image>,
                    lock: Handle<Image>, reload: Handle<Image>, last_tick: u64, window_frames: u32 }
#[derive(Component)]
pub struct GunHudNode;

pub fn setup(hud: Res<super::hud_ui::Hud>, assets: Res<AssetServer>, mut commands: Commands) {
    let source = hud.source();
    let (mut window, mut counter) = (source.scene(37), source.scene(57));
    for id in WINDOW_HIDDEN { if let Some(i) = window.index(id) { window.states[i].forced_hidden = true; } }
    let face: Vec<usize> = counter.draw().iter().filter(|d| d.uv == COUNTER_FACE).map(|d| d.index).collect();
    for i in face { counter.states[i].forced_hidden = true; }
    if std::env::var_os("MW_GUN_HUD_DUMP").is_some() {
        for (n, s) in [(37, &window), (57, &counter)] {
            for d in s.draw() { eprintln!("[gun-hud] program {n} widget {} kind {} uv {:?} scale {:?} pos {:?}", d.id, d.kind, d.uv, d.scale, d.position); }
        }
    }
    let load = |n: &str| assets.load(format!("native/ui/gun/{n}.png"));
    commands.insert_resource(GunHud { window, counter, pistol: load("pistol"), pistol_color: load("pistol-color"), ring: load("sight-ring"),
                                      lock: load("sight-lock"), reload: load("reload"), last_tick: 0, window_frames: 0 });
}

/// One native frame of the two programs, with the magazine on the gauge and in the digits.
pub fn advance(mut gh: ResMut<GunHud>, play: Res<super::Play>) {
    if gh.last_tick == play.tick { return; }
    gh.last_tick = play.tick;
    let ratio = play.ammo.mag as f32 / super::MAG as f32;
    let gh = &mut *gh;
    // the chainsaw window's own animation is held still once it has opened (its first WINDOW_SETTLE frames) [decided
    // 2026-11-10]; the gauge below still follows the magazine
    if gh.window_frames < WINDOW_SETTLE { gh.window.advance(1.); gh.window_frames += 1; }
    gh.counter.advance(1.);
    for id in GAUGE { if let Some(i) = gh.window.index(id) { gh.window.states[i].gauge = Some(ratio); } }
    gh.counter.number(DIGITS, 2, play.ammo.mag as u32);
}

/// Red: the sight is over an enemy, white otherwise [added: the native sight art is white].
const ON_TARGET: Color = Color::srgb(1., 0.12, 0.08);

/// Whether the camera's centre ray passes an enemy's body (a vertical stack of spheres from shin to head).
fn over_enemy(play: &super::Play, street: &super::street::Street, eye: Vec3, look: Vec3) -> bool {
    let mut bodies = street.targets();
    let rin = &play.fight.rin;
    if !play.fight.solo && !play.fight.victory && rin.state != mw_sim::fight::rin_state::DEAD { bodies.push(Vec3::from_array(rin.pos)); }
    if let Some((hp, _, at)) = play.boss { if hp > 0 { bodies.push(at - Vec3::Y * 1.0); } }
    bodies.iter().any(|b| [0.35f32, 0.8, 1.2, 1.55].iter().any(|h| {
        let c = *b + Vec3::Y * *h;
        let t = (c - eye).dot(look);
        t > 0. && t < 60. && (eye + look * t).distance(c) < 0.38
    }))
}

pub fn present(gh: Res<GunHud>, hud: Res<super::hud_ui::Hud>, play: Res<super::Play>, intro: Res<super::intro::Intro>, baron: Res<super::baron::Baron>, execution: Res<super::execution::Execution>, pause: Res<super::pause::Pause>,
               street: Res<super::street::Street>, cams: Query<&GlobalTransform, (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>,
               assets: Res<AssetServer>, windows: Query<&Window>, nodes: Query<Entity, With<GunHudNode>>, mut commands: Commands) {
    for e in &nodes { commands.entity(e).despawn(); }
    // (not over the Mathilda variant's own intro either)
    if !play.as_kreely || intro.controls_blocked() || baron.cine.is_some() || execution.hides_hud() || pause.open || play.fight.victory { return; }
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    let layout = &hud.source().layout;
    let picture = if play.colored { gh.pistol_color.clone() } else { gh.pistol.clone() };
    for sprite in native_ui::sprites(&gh.window, layout, layout, &assets, viewport, 900, None) {
        commands.spawn((sprite, GunHudNode));
    }
    for sprite in native_ui::sprites(&gh.counter, layout, layout, &assets, viewport, 901, None) {
        commands.spawn((sprite, GunHudNode));
    }
    // the native layout's 640x360 frame, centered as `sprites`
    let factor = (viewport.x / 640.).min(viewport.y / 360.);
    let offset = (viewport - Vec2::new(640., 360.) * factor) * 0.5;
    let at = |p: [f32; 2], size: [f32; 2]| {
        let (w, h) = (size[0] * factor, size[1] * factor);
        let c = offset + Vec2::from_array(p) * factor;
        Node { position_type: PositionType::Absolute, left: px(c.x - w * 0.5), top: px(c.y - h * 0.5), width: px(w), height: px(h), ..default() }
    };
    // one pistol only: the counter's face slot stays empty (a second pistol there read as two guns equipped)
    commands.spawn((ImageNode::new(picture), at(WINDOW_PISTOL_AT, WINDOW_PISTOL_SIZE), ZIndex(903), GunHudNode));
    // the sight and the reload banner, centered (native sight art is 32 px on the 640x360 layout)
    let aiming = play.gun.as_ref().is_some_and(|g| g.action == super::GunAction::Aim);
    let reloading = play.ammo.reload.is_some();
    let center = |w: f32, h: f32, dy: f32| Node { position_type: PositionType::Absolute, left: px(viewport.x * 0.5 - w * factor * 0.5),
                                                  top: px(viewport.y * 0.5 - h * factor * 0.5 + dy * factor), width: px(w * factor), height: px(h * factor), ..default() };
    // (VR: the pistol is aimed by hand, down its own sights; no screen-centre ring)
    if aiming && !reloading && !super::vr::on() {
        let image = if play.lock_on { gh.lock.clone() } else { gh.ring.clone() };
        let hot = cams.single().is_ok_and(|c| over_enemy(&play, &street, c.translation(), c.forward().as_vec3()));
        let color = if hot { ON_TARGET } else { Color::WHITE };
        commands.spawn((ImageNode::new(image).with_color(color), center(32., 32., 0.), ZIndex(902), GunHudNode));
    }
    if reloading || (play.ammo.mag == 0 && play.gun.is_some()) {
        commands.spawn((ImageNode::new(gh.reload.clone()), center(110., 32., 40.), ZIndex(902), GunHudNode));
    }
}
