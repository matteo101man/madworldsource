//! Two cockpit cues of the native fight HUD:
//! - The lock-on marker (cIdTargetCursor 802297A4, re-asserted every frame while the lock holds, player update
//!   8027621C): core EST 45 on the locked target (vanish EST 46). Drawn here as screen sprites from its own atlas
//!   frames (core-atlas-1, TUV 200 frames 245 ring, 246 filled arrow, 247 glowing outline): the arrow in the effect's
//!   yellow (1, 0.8, 0) above RinRin's head with a slow bob, and the two 2.5 m rings closing in over the effect's
//!   8 frames when the lock starts [the effect's size curves are not decoded: the close-in and bob are inferred].
//! - The dizzy prompt (state 91, B 802C03EC -> cIdPlCockpit::setSwingNavigation(4, 4) every frame): core_us program
//!   16, the "shake" swing navigation, both halves (Nunchuk widget 10 left, remote widget 1 right). As the recomp's PC
//!   prompts (native_prompts.rs) the controller pictures give way to keys: A on the left half and D on the right over
//!   "SHAKE" (the PC stun shake is mashing A / D, play.rs).
//! - The clash prompts (cIdPlCockpit::setSwingNavigation, PLAYER-COMPETITION-API.md): RinRin's keen competition calls
//!   (4, 4) through the struggle meter (state 78 phase 3: program 16, both arms) and (2, 3) for the directional stage
//!   (phase 5); Elise's and the Baron's clashes show (4, 4) while their meters fill and one remote swing per step.
//!   Navigation enum n is cockpit kind 0x0C + n and program 12 + n (4 / 5 = the shake programs 16 / 17); enums 0..3 as
//!   down / up / right / left are inferred from 4 / 5. On PC the struggle is held X (its key over both arms), a down
//!   swing F and a side swing Q.
//! - The giant-swing prompt (state 49, B 802AF28C: setSwingNavigation(5, -1) every frame of the grab, phase 1, and of
//!   the spin, phase 3): core_us program 17, "keep shaking", remote half only (widget 1); the remote pictures give way
//!   to the shake key (X) over "TAP".
use bevy::prelude::*;
use mw_sim::ui::UiScene;
use super::pause::native_ui;

/// Marker size in 640x360 layout pixels (its height over the target is play.rs's lock point: 2.15 m over RinRin, whose
/// head is ~1.6 m and the effect's +0.9 rides on her root bone).
const ARROW: f32 = 18.;
const RING: f32 = 30.;
/// The marker's ring closes in over EST 45's 8 effect frames (60 Hz).
const RING_SECS: f32 = 8. / 60.;
const YELLOW: Color = Color::srgb(1., 0.8, 0.);
/// Program 16 (shake) widgets: the remote half and the Nunchuk half; their pictures are UV group 8 sprites 0..3.
const SHAKE: u32 = 16;
const REMOTE: u32 = 1;
const NUNCHUK: u32 = 10;
const PICTURES: [[u16; 2]; 4] = [[8, 0], [8, 1], [8, 2], [8, 3]];
const DIZZY: u8 = 91;
/// A tapped key's swell and flash (s).
const TAP_PULSE: f32 = 0.18;
const KEEP_SHAKING: u32 = 17;
const MIDDLE: u32 = 5;
const GIANT_SWING: u8 = 49;

#[derive(Resource)]
pub struct Prompts { /// the clash's swing navigation (remote, Nunchuk enums; -1 none), its programs 12..15 (remote half), the F / Q keys
                     nav: Option<(i8, i8)>, dirs: Vec<UiScene>, struggle: UiScene, swing_keys: [Handle<Image>; 2],
                     shake: UiScene, shown: bool, spin: UiScene, spinning: bool, last_tick: u64, lock_since: Option<f32>, ring: Handle<Image>, arrow: Handle<Image>,
                     glow: Handle<Image>, keys: [Handle<Image>; 2], shake_key: Handle<Image>, word: Handle<Image> }
#[derive(Component)]
pub struct PromptNode;

pub fn setup(hud: Res<super::hud_ui::Hud>, assets: Res<AssetServer>, mut commands: Commands) {
    let mut shake = hud.source().scene(SHAKE);
    let pictures: Vec<usize> = shake.draw().iter().filter(|d| PICTURES.contains(&d.uv)).map(|d| d.index).collect();
    for i in pictures { shake.states[i].forced_hidden = true; }
    let mut spin = hud.source().scene(KEEP_SHAKING);
    for id in [NUNCHUK, MIDDLE] { spin.hide_tree(id, true); }
    let pictures: Vec<usize> = spin.draw().iter().filter(|d| PICTURES.contains(&d.uv)).map(|d| d.index).collect();
    for i in pictures { spin.states[i].forced_hidden = true; }
    let mut struggle = hud.source().scene(SHAKE);
    let pictures: Vec<usize> = struggle.draw().iter().filter(|d| PICTURES.contains(&d.uv)).map(|d| d.index).collect();
    for i in pictures { struggle.states[i].forced_hidden = true; }
    let dirs = (12..16).map(|program| {
        let mut s = hud.source().scene(program);
        s.hide_tree(NUNCHUK, true);
        let pictures: Vec<usize> = s.draw().iter().filter(|d| PICTURES.contains(&d.uv)).map(|d| d.index).collect();
        for i in pictures { s.states[i].forced_hidden = true; }
        s
    }).collect();
    let load = |n: &str| assets.load(format!("native/ui/{n}.png"));
    commands.insert_resource(Prompts { nav: None, dirs, struggle, swing_keys: [load("pc/KeyF-46"), load("pc/KeyQ-46")], shake, shown: false, spin, spinning: false, last_tick: 0, lock_since: None, ring: load("lock/ring"), arrow: load("lock/arrow"),
                                       glow: load("lock/arrow-outline"), keys: [load("pc/KeyA-46"), load("pc/KeyD-46")], shake_key: load("pc/KeyX-46"), word: load("pc/WordTap-46") });
}

/// One native frame of the shake program while the player is dizzy (restarted on each new dizzy spell).
pub fn advance(mut pr: ResMut<Prompts>, play: Res<super::Play>, elise: Res<super::elise::Elise>, baron: Res<super::baron::Baron>) {
    if pr.last_tick == play.tick { return; }
    pr.last_tick = play.tick;
    // the clash's swing navigation this frame
    let pl = &play.fight.player;
    let boss = elise.prompt.as_deref().or(baron.prompt.as_deref());
    let nav = match (pl.state(), pl.loco.phase, boss) {
        (78, 3, _) => Some((4, 4)),
        (78, 5, _) => Some((2, 3)),
        (_, _, Some(p)) if p.starts_with("TAP") || p.starts_with("SHAKE") => Some((4, 4)),
        (_, _, Some(p)) if p.contains("DOWN") => Some((0, -1)),
        (_, _, Some(p)) if p.contains("SIDE") => Some((2, -1)),
        _ => None,
    };
    if nav != pr.nav {
        if let Some((r, _)) = nav { if r == 4 { pr.struggle.restart_tree(REMOTE); pr.struggle.restart_tree(NUNCHUK); } else { pr.dirs[r as usize].restart_tree(REMOTE); } }
        pr.nav = nav;
    }
    match pr.nav { Some((4, _)) => pr.struggle.advance(1.), Some((r, _)) => pr.dirs[r as usize].advance(1.), None => {} }
    // inspection: MW_PROMPT_PREVIEW=1 shows the shake prompt without the dizzy state (=spin: the giant swing's)
    let dizzy = play.fight.player.state() == DIZZY || std::env::var("MW_PROMPT_PREVIEW").as_deref() == Ok("1");
    if dizzy && !pr.shown { pr.shake.restart_tree(REMOTE); pr.shake.restart_tree(NUNCHUK); }
    pr.shown = dizzy;
    if dizzy { pr.shake.advance(1.); }
    let pl = &play.fight.player;
    let spinning = (pl.state() == GIANT_SWING && matches!(pl.loco.phase, 1 | 3)) || std::env::var("MW_PROMPT_PREVIEW").as_deref() == Ok("spin");
    if spinning && !pr.spinning { pr.spin.restart_tree(REMOTE); }
    pr.spinning = spinning;
    if spinning { pr.spin.advance(1.); }
}

pub fn present(mut pr: ResMut<Prompts>, hud: Res<super::hud_ui::Hud>, play: Res<super::Play>, intro: Res<super::intro::Intro>,
               pause: Res<super::pause::Pause>, time: Res<Time<Real>>, assets: Res<AssetServer>, windows: Query<&Window>,
               cams: Query<(&Camera, &GlobalTransform), (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>, nodes: Query<Entity, With<PromptNode>>, keys_in: Res<ButtonInput<KeyCode>>,
               mut pressed: Local<[f32; 2]>, mut commands: Commands) {
    for e in &nodes { commands.entity(e).despawn(); }
    let now = time.elapsed_secs();
    // tap feedback [PC]: each A / D press swells its key cap and flashes it yellow for TAP_PULSE seconds
    if keys_in.just_pressed(KeyCode::KeyA) { pressed[0] = now; }
    if keys_in.just_pressed(KeyCode::KeyD) { pressed[1] = now; }
    if std::env::var_os("MW_TAP_PREVIEW").is_some() { pressed[((now * 6.) as usize) % 2] = now - 0.02; }
    let pulse = |k: usize| (1. - (now - pressed[k]) / TAP_PULSE).clamp(0., 1.);
    let tap_key = |k: usize, c: Vec2, factor: f32| {
        let p = pulse(k); let size = 30. * factor * (1. + 0.45 * p);
        (ImageNode::new(Handle::default()).with_color(Color::srgb(1., 1., 1. - 0.75 * p)), Node { position_type: PositionType::Absolute, left: px(c.x - size * 0.5), top: px(c.y - 8. * factor - size * 0.5), width: px(size), height: px(size), ..default() })
    };
    let locked = play.lock_on && play.lock_mark.is_some();
    if !locked { pr.lock_since = None; } else if pr.lock_since.is_none() { pr.lock_since = Some(now); }
    if intro.controls_blocked() || pause.open { return; }
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    let factor = (viewport.x / 640.).min(viewport.y / 360.);
    let offset = (viewport - Vec2::new(640., 360.) * factor) * 0.5;
    let node = |c: Vec2, w: f32, h: f32| Node { position_type: PositionType::Absolute, left: px(c.x - w * 0.5), top: px(c.y - h * 0.5),
                                                width: px(w), height: px(h), ..default() };
    // the lock marker over the target (play.rs keeps its point: over the head, lower once down, gliding to a new target)
    if let (Some(t0), Ok((cam, at)), Some(mark)) = (pr.lock_since, cams.single(), play.lock_mark) {
        let head = mark + Vec3::Y * 0.06 * (now * 3.).sin();
        if let Some(ndc) = cam.world_to_ndc(at, head).filter(|n| n.z > 0. && n.z < 1.) {
            let c = Vec2::new((ndc.x + 1.) * 0.5 * viewport.x, (1. - ndc.y) * 0.5 * viewport.y);
            let (a, g) = (ARROW * factor, ARROW * factor * 1.35);
            commands.spawn((ImageNode::new(pr.glow.clone()).with_color(YELLOW.with_alpha(0.55)), node(c, g, g), ZIndex(880), PromptNode));
            commands.spawn((ImageNode::new(pr.arrow.clone()).with_color(YELLOW), node(c, a, a), ZIndex(881), PromptNode));
            let k = ((now - t0) / RING_SECS).min(1.);
            if k < 1. {
                let r = RING * factor * (2.5 - 1.5 * k);
                commands.spawn((ImageNode::new(pr.ring.clone()).with_color(Color::WHITE.with_alpha(1. - k * 0.6)), node(c, r, r), ZIndex(879), PromptNode));
            }
        }
    }
    // the giant swing's keep-shaking prompt (remote half)
    if pr.spinning {
        let layout = &hud.source().layout;
        for sprite in native_ui::sprites(&pr.spin, layout, layout, &assets, viewport, 905, None) { commands.spawn((sprite, PromptNode)); }
        let c = offset + Vec2::new(460., 250.) * factor;
        let jig = Vec2::new(4. * (now * 18.).sin() * factor, 0.);
        // mashed A / D on PC
        for k in 0..2 {
            let (mut img, n) = tap_key(k, c + jig + Vec2::new(if k == 0 { -18. } else { 18. }, 8.) * factor, factor);
            img.image = pr.keys[k].clone();
            if !super::vr::on() { commands.spawn((img, n, GlobalZIndex(2000), PromptNode)); }
        }
        if !super::vr::on() { commands.spawn((ImageNode::new(pr.word.clone()), node(c + Vec2::Y * 16. * factor, 46. * factor, 15. * factor), GlobalZIndex(2000), PromptNode)); }
    }
    // the clash's swing navigation: the native programs, the PC key (and the word for the shake kinds) in the pictures' place
    if let Some((r, _)) = pr.nav {
        let layout = &hud.source().layout;
        let scene = if r == 4 { &pr.struggle } else { &pr.dirs[r as usize] };
        for sprite in native_ui::sprites(scene, layout, layout, &assets, viewport, 905, None) { commands.spawn((sprite, PromptNode)); }
        let draws = scene.draw();
        let halves: &[(u32, Vec2)] = if r == 4 { &[(NUNCHUK, Vec2::new(180., 220.)), (REMOTE, Vec2::new(460., 220.))] } else { &[(REMOTE, Vec2::new(460., 220.))] };
        let (key, word) = match r { 0 => (pr.swing_keys[0].clone(), false), 2 if pr.nav != Some((2, 3)) => (pr.swing_keys[1].clone(), false), _ => (pr.shake_key.clone(), true) };
        for (k, (id, fallback)) in halves.iter().enumerate() {
            // the shake kinds are mashed A / D on PC: A over the left half, D over the right (one half: D)
            let key = if word { pr.keys[if halves.len() == 2 { k } else { 1 }].clone() } else { key.clone() };
            let at = draws.iter().find(|d| d.id == *id).map(|d| Vec2::new(d.position[0], d.position[1])).unwrap_or(*fallback);
            let c = offset + at * factor;
            let jig = if word { Vec2::new(4. * (now * 18. + k as f32 * std::f32::consts::PI).sin() * factor, 0.) } else { Vec2::ZERO };
            if word {
                let kk = if halves.len() == 2 { k } else { 1 };
                let (mut img, n) = tap_key(kk, c + jig, factor); img.image = key.clone();
                if !super::vr::on() { commands.spawn((img, n, GlobalZIndex(2000), PromptNode)); }
            } else if !super::vr::on() { commands.spawn((ImageNode::new(key.clone()), node(c + jig - Vec2::Y * 8. * factor, 30. * factor, 30. * factor), GlobalZIndex(2000), PromptNode)); }
            if word { if !super::vr::on() { commands.spawn((ImageNode::new(pr.word.clone()), node(c + Vec2::Y * 16. * factor, 46. * factor, 15. * factor), GlobalZIndex(2000), PromptNode)); } }
        }
    }
    // the dizzy shake prompt: the native program's flash and glow, the key and word in the remote's place
    if pr.shown && !play.fight.victory {
        let layout = &hud.source().layout;
        for sprite in native_ui::sprites(&pr.shake, layout, layout, &assets, viewport, 905, None) { commands.spawn((sprite, PromptNode)); }
        let draws = pr.shake.draw();
        for (k, (id, fallback)) in [(NUNCHUK, Vec2::new(180., 220.)), (REMOTE, Vec2::new(460., 220.))].into_iter().enumerate() {
            let at = draws.iter().find(|d| d.id == id).map(|d| Vec2::new(d.position[0], d.position[1])).unwrap_or(fallback);
            let c = offset + at * factor;
            // the key caps jiggle as the controller pictures shake, the two halves in turn
            let jig = Vec2::new(4. * (now * 18. + k as f32 * std::f32::consts::PI).sin() * factor, 0.);
            let (mut img, n) = tap_key(k, c + jig, factor); img.image = pr.keys[k].clone();
            if !super::vr::on() { commands.spawn((img, n, GlobalZIndex(2000), PromptNode)); }
            if !super::vr::on() { commands.spawn((ImageNode::new(pr.word.clone()), node(c + Vec2::Y * 16. * factor, 46. * factor, 15. * factor), GlobalZIndex(2000), PromptNode)); }
        }
    }
}
