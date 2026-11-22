//! The room clear after a boss (research/ELISE-BOSS.md §2, the clear sequence 80124258), drawn once Elise's ev34 and her
//! clear line 0x1027 are done:
//! - the clear board (cIdClearBoard, HUD slot +0x18, started by 80244D54): core_us program 10 on layer 0xD, "WINNER";
//!   80244D70 holds until its widget 1 animation ends: the program's keys end at frame 10 (UiScene::track_end), so the
//!   hold is at least BOARD_MIN [inferred];
//! - the stage-clear results (cIdStageClear, slot +0x28, 80124018 -> 802416B0): the room idpac's program 31 (c303), its
//!   tally counted up over 60 frames (confirm skips it), 27 frames more, then it waits for confirm (cPad down 0x10, PC
//!   Enter / left mouse) [the score / time / kill digits are not filled: their globals 80697F04.. are not tracked, and the program shows
//!   its portrait and burst only];
//! - then the story scene: Elise's 0xB14 plays ev84 (then the world map), the Baron's results go to ev93 (then 0xE03, the
//!   ending); the port plays the event when it is staged (play_intro.rs `request_ending`), then returns to the stage select
//!   (play_select.rs) in place of the world map [added]. The Baron's clear runs after ev51 (the boss callback 8016C8F0
//!   calls 80124258 too), its results from idpac_c503_us program 31.
//! Not ported: the board's SE 0x57 and the results' SE 10000036, the autosave.
use bevy::prelude::*;
use mw_sim::ui::UiScene;
use super::pause::native_ui::{self, UiSource};

const BOARD: u32 = 10;
/// The board's shortest hold (frames).
const BOARD_MIN: f32 = 90.;
const RESULTS: u32 = 31;
const TALLY: f32 = 60. + 27.;

#[derive(Resource, Default)]
pub struct Clear { phase: u8, t: f32, board_end: f32, board: Option<UiScene>, results: Option<(UiSource, UiScene)> }
#[derive(Component)]
pub struct ClearNode;

pub fn advance(mut c: ResMut<Clear>, elise: Res<super::elise::Elise>, baron: Res<super::baron::Baron>, mut intro: ResMut<super::intro::Intro>,
               hud: Res<super::hud_ui::Hud>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, time: Res<Time<Real>>) {
    let frames = time.delta_secs() * 30.;
    let confirm = keys.just_pressed(KeyCode::Enter) || mouse.just_pressed(MouseButton::Left)
        || std::env::var_os("MW_CLEAR_AUTO").is_some();
    match c.phase {
        0 => if elise.cleared() || baron.cleared() || std::env::var("MW_ENDING_TEST").as_deref() == Ok("results") {
            let board = hud.source().scene(BOARD);
            // (widget 1 carries no tracks of its own: its tree animates, so the board lasts to the program's last key)
            // [inferred floor: the tracks end at frame 10, but the native play flag +0xC0 bit 3 outlasts them]
            c.board_end = board.program.widgets.iter().map(|w| board.track_end(w.id)).fold(BOARD_MIN, f32::max);
            c.board = Some(board); c.phase = 1; c.t = 0.;
            eprintln!("[clear] board (core_us program {BOARD}, its animation ends at frame {})", c.board_end);
        },
        1 => {
            c.t += frames;
            if let Some(b) = c.board.as_mut() { b.advance(frames); }
            if c.t >= c.board_end {
                let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
                let pack = if super::elise::on() { "idpac_c303_us" } else { "idpac_c503_us" };
                c.results = root.join("assets/native/ui").join(pack).join("animation.idt").exists().then(|| {
                    let source = UiSource::load(&root, pack); let scene = source.scene(RESULTS); (source, scene)
                });
                c.board = None; c.phase = 2; c.t = 0.;
                eprintln!("[clear] results ({pack} program {RESULTS})");
            }
        }
        2 => {
            c.t += frames;
            if let Some((_, s)) = c.results.as_mut() { s.advance(frames); }
            if c.t < 60. && confirm && std::env::var_os("MW_CLEAR_AUTO").is_none() { c.t = 60.; }
            else if c.t >= TALLY && confirm {
                c.phase = 3; c.results = None;
                if intro.has_ending() { eprintln!("[clear] confirmed: the ending event"); intro.request_ending(); }
                else { eprintln!("[clear] confirmed: no ending staged -> the stage select"); if std::env::var_os("MW_CLEAR_AUTO").is_none() { super::select::relaunch(""); } }
            }
        }
        // the ending played out (or was skipped): the stage select stands in for the world map / the ending credits
        3 => if intro.ending_done() {
            c.phase = 4;
            eprintln!("[clear] ending over -> the stage select");
            if std::env::var_os("MW_CLEAR_AUTO").is_none() { super::select::relaunch(""); }
        },
        _ => {}
    }
}

pub fn present(c: Res<Clear>, hud: Res<super::hud_ui::Hud>, assets: Res<AssetServer>, windows: Query<&Window>, nodes: Query<Entity, With<ClearNode>>,
               mut commands: Commands) {
    for e in &nodes { commands.entity(e).despawn(); }
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    if let Some(b) = &c.board {
        let layout = &hud.source().layout;
        for sprite in native_ui::sprites(b, layout, layout, &assets, viewport, 1500, None) { commands.spawn((sprite, ClearNode)); }
    }
    // (the idpac's programs draw from their own pages: with core_us as the common resource they draw wrong pages)
    if let Some((source, s)) = &c.results {
        for sprite in native_ui::sprites(s, &source.layout, &source.layout, &assets, viewport, 1500, None) { commands.spawn((sprite, ClearNode)); }
    }
}
