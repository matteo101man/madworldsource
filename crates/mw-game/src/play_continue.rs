//! The native death flow and the intro's skip hint, in place of the old debug text:
//! - After a death (once RinRin's taunt has played, as before) the original continue window, cIdGameOver: subscr0ff
//!   program 0 ("JACK DIED / CONTINUE? / YES NO", the continues counter); its grain (program 3) is not drawn
//!   (mw_sim::ui_continue). The mouse is free while it is up (play.rs cursor). YES spends a continue and restarts (the fight's Continue); NO, or no continues left, plays
//!   cIdGameOverEnd (program 1, static and "YOU LOSE"). The stage-select return after a game over is not ported: the
//!   fight restarts with the continues refilled [PC].
//! - The original has no skip prompt (one B press skips, EVENT-API.md). The hold-to-skip hint is spelled in core_us's
//!   own small alphabet (group 38, the "YOU LOSE" letters) with a fill bar, bottom right [added, native art].
use bevy::prelude::*;
use mw_sim::ui_continue::{ContinueWindow, Input, Phase};
use super::pause::native_ui::{self, UiSource};

/// Continues at the start (the pause map's count; native getContinue() read 2 in the recomp's run).
pub const CONTINUES: u32 = 2;
/// Game frames after the death before the window may open, and the most it waits for RinRin's taunt.
const OPEN_AFTER: u64 = 45;
const OPEN_LATEST: u64 = 240;

#[derive(Resource)]
pub struct ContinueUi { source: UiSource, window: Option<ContinueWindow>, pending: Input, accumulator: f64,
                        /// The answer for the fight: Some(true) restarts the player (Continue).
                        pub answer: Option<bool> }
#[derive(Component)]
pub struct ContinueNode;

pub fn setup(mut commands: Commands) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    commands.insert_resource(ContinueUi { source: UiSource::load_idt(&root, "subscr0ff_us", "core_us"), window: None, pending: Input::default(),
                                          accumulator: 0., answer: None });
}

pub fn advance(mut ui: ResMut<ContinueUi>, mut play: ResMut<super::Play>, time: Res<Time<Real>>, keys: Res<ButtonInput<KeyCode>>,
               mouse: Res<ButtonInput<MouseButton>>, pause: Res<super::pause::Pause>, windows: Query<&Window>,
               execution: Res<super::execution::Execution>) {
    if pause.open || super::editor::on() { return; }
    let dead = play.fight.player.state() == 95;
    if !dead { ui.window = None; ui.accumulator = 0.; return; }
    // (the last-life execution plays first: play_execution.rs)
    if ui.window.is_none() && ui.answer.is_none() && !execution.holds_window() {
        let since = play.dead_since.unwrap_or(play.tick);
        let rin = &play.fight.rin;
        let laughed = rin.state == mw_sim::fight::rin_state::PL_DEAD && (rin.phase >= 2 || rin.motion.frame > 40.);
        if play.tick >= since + OPEN_AFTER && (laughed || play.tick >= since + OPEN_LATEST || play.fight.solo) {
            let s = &ui.source;
            ui.window = Some(ContinueWindow::new(s.scene(0), s.scene(3), s.scene(1), play.continues));
        }
        return;
    }
    let ui = &mut *ui;
    let Some(w) = ui.window.as_mut() else { return; };
    ui.pending.yes |= keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA);
    ui.pending.no |= keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD);
    ui.pending.back |= keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::Escape);
    // the mouse: pointing at YES / NO selects it (entries 11 / 12, about 110x34 on the layout), a click confirms
    let pointed = windows.single().ok().and_then(|win| {
        let p = win.cursor_position()?;
        let viewport = Vec2::new(win.width(), win.height());
        let factor = (viewport.x / 640.).min(viewport.y / 360.);
        let at = (p - (viewport - Vec2::new(640., 360.) * factor) * 0.5) / factor;
        w.scene.draw().iter().filter(|d| d.id == 11 || d.id == 12)
            .find(|d| (at.x - d.position[0]).abs() < 58. && (at.y - d.position[1]).abs() < 20.).map(|d| d.id == 11)
    });
    if let Some(yes) = pointed { if yes { ui.pending.yes = true; } else { ui.pending.no = true; } }
    ui.pending.confirm |= keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) || (mouse.just_pressed(MouseButton::Left) && pointed.is_some());
    // inspection: MW_CONTINUE_AUTO=yes|no answers once the window is up
    if let Ok(a) = std::env::var("MW_CONTINUE_AUTO") { if w.phase == Phase::Selecting && w.window_visible() { ui.pending = Input { no: a == "no", confirm: true, ..default() }; } }
    ui.accumulator += time.delta_secs_f64();
    while ui.accumulator >= 1. / mw_sim::event::EVENT_HZ {
        ui.accumulator -= 1. / mw_sim::event::EVENT_HZ;
        w.step(std::mem::take(&mut ui.pending));
        if w.phase == Phase::Done {
            play.continues = if w.result { w.continues } else { CONTINUES };
            ui.answer = Some(true);
            break;
        }
    }
    if ui.answer.is_some() { ui.window = None; }
}

pub fn present(ui: Res<ContinueUi>, hud: Res<super::hud_ui::Hud>, intro: Res<super::intro::Intro>, baron: Res<super::baron::Baron>, pause: Res<super::pause::Pause>, assets: Res<AssetServer>,
               windows: Query<&Window>, nodes: Query<Entity, With<ContinueNode>>, mut commands: Commands) {
    for e in &nodes { commands.entity(e).despawn(); }
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    let core = &hud.source().layout;
    if let Some(w) = ui.window.as_ref().filter(|_| !pause.open) {
        if w.window_visible() {
            // the window without its grain program (3): the user asked for no static over it [PC]
            for s in native_ui::sprites(&w.scene, core, core, &assets, viewport, 17500, None) { commands.spawn((s, ContinueNode)); }
        } else if w.phase == Phase::Ending {
            for s in native_ui::sprites(&w.end, core, core, &assets, viewport, 17600, None) { commands.spawn((s, ContinueNode)); }
        }
    }
    // the intro's skip hint: hold Enter; the bar fills with the hold
    if let Some(k) = intro.skip_hint().or(baron.cine_skip_hint()).filter(|_| !pause.open) {
        let factor = (viewport.x / 640.).min(viewport.y / 360.);
        let offset = (viewport - Vec2::new(640., 360.) * factor) * 0.5;
        let text = "HOLD ENTER TO SKIP";
        let (x, y, scale) = (604. - text.len() as f32 * 8.5, 330., 0.85);
        for s in native_ui::small_text(core, &assets, text, Vec2::new(x, y), scale, Color::WHITE, viewport, 17000) { commands.spawn((s, ContinueNode)); }
        let width = text.len() as f32 * 8.5 - 2.;
        let bar = |w: f32, color: Color, z: i32| (Node { position_type: PositionType::Absolute, left: px(offset.x + x * factor), top: px(offset.y + (y + 10.) * factor),
                                                          width: px(w * factor), height: px(2. * factor), ..default() }, BackgroundColor(color), GlobalZIndex(z), ContinueNode);
        commands.spawn(bar(width, Color::srgba(1., 1., 1., 0.25), 17000));
        if k > 0. { commands.spawn(bar(width * k, Color::WHITE, 17001)); }
    }
}
