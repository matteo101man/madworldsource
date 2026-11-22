//! The hold controls [added]: while the player holds someone (or something), a panel in the bottom-right corner, above
//! his gauge, lists what he can do with them, drawn as the pause menu's AREA DATA box (a white title bar,
//! thin white rules, white rows in the subtitle font; play_editor.rs Painter).
//! - a grunt in his hands (states 0x14 grab, 0x16 hold): walk him, throw, head-butt, put down, and as Kreely his suplex;
//! - an object (0x1A pick, 0x1B hold): walk, throw, over a grunt's head, put down.
use bevy::prelude::*;
use super::editor::{EdNode, Fonts, Painter};

/// The pause menu's colours: the white rules and title bar, its grey-violet title letters, the dark panel.
const LINE: Color = Color::srgb(0.96, 0.95, 0.98);
const TITLE: Color = Color::srgb(0.42, 0.40, 0.52);
const PANEL: Color = Color::srgba(0.06, 0.05, 0.12, 0.82);

/// Update: the panel, rebuilt when what it shows changes.
pub fn present(play: Res<super::Play>, pause: Res<super::pause::Pause>, intro: Res<super::intro::Intro>, execution: Res<super::execution::Execution>,
               windows: Query<&Window>, assets: Res<AssetServer>, nodes: Query<(Entity, &EdNode)>,
               mut fonts: Local<Option<Option<Fonts>>>, mut last: Local<String>, mut commands: Commands) {
    let st = play.fight.player.state();
    let object = play.fight.carry.is_some_and(|c| c.object);
    let holding = play.fight.carry.is_some() && matches!(st, 20 | 22 | 26 | 27);
    let shown = holding && !pause.open && !intro.controls_blocked() && !execution.hides_hud() && std::env::var("MW_HOLD_UI").as_deref() != Ok("0");
    let Ok(window) = windows.single() else { return; };
    let key = format!("{shown}|{object}|{}|{}x{}", play.as_kreely, window.width(), window.height());
    if *last == key { return; }
    *last = key;
    for (e, n) in &nodes { if n.0 { commands.entity(e).despawn(); } }
    if !shown { return; }
    if fonts.is_none() { *fonts = Some(super::editor::load_fonts()); }
    let Some(Some(fonts)) = fonts.as_ref() else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    let k = (viewport.x / 640.).min(viewport.y / 360.);
    let offset = (viewport - Vec2::new(640., 360.) * k) * 0.5;
    let mut p = Painter { commands: &mut commands, assets: &assets, fonts, k, offset, layer: true, parent: None };
    // (key, what it does)
    let rows: Vec<(&str, &str)> = if object {
        vec![("WASD", "Move"), ("F", "Throw"), ("F", "Cover head (close)"), ("LMB", "Release: drop")]
    } else if play.as_kreely {
        vec![("WASD", "Move"), ("Q", "Suplex"), ("F", "Throw"), ("X", "Head-butt"), ("LMB", "Release: drop")]
    } else {
        vec![("WASD", "Move"), ("F", "Throw"), ("Q", "Side throw"), ("X", "Head-butt"), ("LMB", "Release: drop")]
    };
    let title = if object { "ITEM" } else { "GRAB" };
    // the pause menu's AREA DATA box: a white title bar with dark letters over a box ruled in thin white, its rows in
    // white; each key in a small ruled box of its own
    let (w, row_h, bar) = (150., 14., 13.);
    let h = bar + 6. + rows.len() as f32 * row_h + 4.;
    let (x0, y0) = (640. - 14. - w, 288. - h);
    let line = |p: &mut Painter, x: f32, y: f32, w: f32, h: f32, t: f32| {
        p.rect(x, y, w, t, LINE, 9002); p.rect(x, y + h - t, w, t, LINE, 9002);
        p.rect(x, y, t, h, LINE, 9002); p.rect(x + w - t, y, t, h, LINE, 9002);
    };
    p.rect(x0, y0, w, h, PANEL, 9000);
    line(&mut p, x0, y0, w, h, 1.2);
    p.rect(x0, y0, w, bar, LINE, 9001);
    let tw = p.width(title, 10.);
    p.text(title, x0 + (w - tw) * 0.5, y0 + 1.5, 10., TITLE, w);
    let chip_w = rows.iter().map(|(k, _)| p.width(k, 9.)).fold(0., f32::max) + 8.;
    for (i, (key, what)) in rows.iter().enumerate() {
        let y = y0 + bar + 6. + i as f32 * row_h;
        line(&mut p, x0 + 7., y - 1., chip_w, 12., 0.8);
        let kw = p.width(key, 9.);
        p.text(key, x0 + 7. + (chip_w - kw) * 0.5, y + 0.5, 9., Color::WHITE, chip_w);
        p.text(what, x0 + 13. + chip_w, y, 10., Color::WHITE, w - chip_w - 20.);
    }
}
