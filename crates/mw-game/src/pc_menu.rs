//! In-game PC settings overlay (F10, or Select+Start on a gamepad; both rebindable),
//! live application of window/color/volume settings, and gamepad rumble from the
//! game's own WPADControlMotor calls. Escape stays the game's pause while closed.
use crate::pc_config::{Action, Chord, ColorMode, Input, PcConfig, KEYS, PAD};
use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::input::gamepad::{Gamepad, GamepadButton, GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode};
use mw_native_platform::host;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Shared with the input system: while open (or until every input is released after
/// closing) the game receives a neutral controller.
#[derive(Resource, Default)]
pub struct SettingsMenu {
    pub open: bool,
    pub just_opened: bool,
    pub block_until_release: bool,
    page: usize,
    row: usize,
    capture: Option<Capture>,
    message: String,
    repeat: f32,
}
impl SettingsMenu {
    pub fn open(&mut self) {
        self.open = true;
        self.just_opened = true;
        self.capture = None;
        self.message.clear();
    }
}
struct Capture {
    action: Action,
    pad: bool,
    held: Vec<Input>,
    elapsed: f32,
    armed: bool,
}
#[derive(Component)]
struct MenuPanel;
#[derive(Component)]
struct MenuText;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Setting {
    MouseSens, StickSens, InvertY, AutoFollow, LockOrbit, Deadzone, GestureThreshold, Rumble, RumbleStrength,
    InternalRes, Vsync, Fullscreen, Color, Volume, ResetAll,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Row { Bind(Action), Set(Setting) }
const PAGES: [&str; 4] = ["Keyboard & mouse", "Gamepad", "Camera & gamepad", "Video, color & audio"];
fn rows(page: usize) -> Vec<Row> {
    use Setting::*;
    match page {
        0 | 1 => Action::ALL.into_iter().filter(|a| page == 0 || !matches!(a, Action::MoveUp | Action::MoveDown | Action::MoveLeft | Action::MoveRight)).map(Row::Bind).collect(),
        2 => [MouseSens, StickSens, InvertY, AutoFollow, LockOrbit, Deadzone, GestureThreshold, Rumble, RumbleStrength].map(Row::Set).to_vec(),
        _ => [InternalRes, Vsync, Fullscreen, Color, Volume, ResetAll].map(Row::Set).to_vec(),
    }
}
const RES: [&str; 5] = ["auto", "1", "2", "3", "4"];
fn step(v: &mut f32, d: i32, by: f32, lo: f32, hi: f32) {
    *v = ((*v + d as f32 * by) / by).round() * by;
    *v = v.clamp(lo, hi);
}
/// Change a setting by `d` (-1/+1; activate is +1). Returns a status message.
fn adjust(cfg: &mut PcConfig, s: Setting, d: i32) -> Option<String> {
    use Setting::*;
    match s {
        MouseSens => step(&mut cfg.camera.mouse_sensitivity, d, 0.1, 0.1, 5.),
        StickSens => step(&mut cfg.camera.stick_sensitivity, d, 0.1, 0.1, 5.),
        InvertY => cfg.camera.invert_y ^= true,
        AutoFollow => cfg.camera.auto_follow ^= true,
        LockOrbit => cfg.camera.lock_orbit ^= true,
        Deadzone => step(&mut cfg.gamepad.deadzone, d, 0.05, 0., 0.5),
        GestureThreshold => step(&mut cfg.gamepad.gesture_threshold, d, 0.05, 0.3, 0.9),
        Rumble => cfg.gamepad.rumble ^= true,
        RumbleStrength => step(&mut cfg.gamepad.rumble_strength, d, 0.1, 0., 1.),
        InternalRes => {
            let i = RES.iter().position(|r| *r == cfg.video.internal_res).unwrap_or(0) as i32;
            cfg.video.internal_res = RES[(i + d).rem_euclid(RES.len() as i32) as usize].into();
            return Some("Internal resolution applies at the next launch.".into());
        }
        Vsync => cfg.video.vsync ^= true,
        Fullscreen => cfg.video.fullscreen ^= true,
        Color => {
            let modes = [ColorMode::Original, ColorMode::Color, ColorMode::Legacy];
            let i = modes.iter().position(|m| *m == cfg.color).unwrap_or(1) as i32;
            cfg.color = modes[(i + d).rem_euclid(3) as usize];
        }
        Volume => step(&mut cfg.audio.master_volume, d, 0.05, 0., 1.),
        ResetAll => {
            if d <= 0 { return None; }
            let path = cfg.path.take();
            *cfg = PcConfig { path, ..default() };
            return Some("All settings and bindings restored to defaults.".into());
        }
    }
    None
}
fn env_note(var: &str) -> &'static str {
    if std::env::var_os(var).is_some() { "  [env override]" } else { "" }
}
fn setting_line(cfg: &PcConfig, s: Setting) -> String {
    use Setting::*;
    let on = |b: bool| if b { "on" } else { "off" };
    match s {
        MouseSens => format!("Mouse sensitivity        {:.1}", cfg.camera.mouse_sensitivity),
        StickSens => format!("Right-stick sensitivity  {:.1}", cfg.camera.stick_sensitivity),
        InvertY => format!("Invert camera Y          {}", on(cfg.camera.invert_y)),
        AutoFollow => format!("Camera auto-follow       {}", on(cfg.camera.auto_follow)),
        LockOrbit => format!("Orbit while locked on    {}", on(cfg.camera.lock_orbit)),
        Deadzone => format!("Stick deadzone           {:.2}", cfg.gamepad.deadzone),
        GestureThreshold => format!("Flick / circle threshold {:.2}", cfg.gamepad.gesture_threshold),
        Rumble => format!("Rumble                   {}", on(cfg.gamepad.rumble)),
        RumbleStrength => format!("Rumble strength          {:.0}%", cfg.gamepad.rumble_strength * 100.),
        InternalRes => format!("Internal resolution      {} (next launch){}", cfg.video.internal_res, env_note("MW_INTERNAL_RES")),
        Vsync => format!("VSync                    {}{}", on(cfg.video.vsync), env_note("MW_VSYNC")),
        Fullscreen => format!("Fullscreen (borderless)  {}{}", on(cfg.video.fullscreen), env_note("MW_FULLSCREEN")),
        Color => format!("Color mode               {}{}", cfg.color.label(), env_note("MW_COLOR")),
        Volume => format!("Master volume            {:.0}%{}", cfg.audio.master_volume * 100., env_note("MW_MASTER_VOLUME")),
        ResetAll => "Restore all defaults     (press Enter / A)".into(),
    }
}
fn bind_line(cfg: &PcConfig, a: Action, pad: bool) -> String {
    let list = cfg.device_bindings(a, pad);
    let names = if list.is_empty() { "-".into() } else { list.iter().map(Chord::name).collect::<Vec<_>>().join(", ") };
    format!("{:<40} {}", a.label(), names)
}
fn render(menu: &SettingsMenu, cfg: &PcConfig) -> String {
    let mut out = String::from("PC SETTINGS\n");
    for (i, p) in PAGES.iter().enumerate() {
        out += &if i == menu.page { format!("[{p}]  ") } else { format!(" {p}   ") };
    }
    out += "\n\n";
    let rows = rows(menu.page);
    // Scroll window so the cursor stays visible on the long binding pages.
    const VISIBLE: usize = 20;
    let first = menu.row.saturating_sub(VISIBLE / 2).min(rows.len().saturating_sub(VISIBLE));
    for (i, row) in rows.iter().enumerate().skip(first).take(VISIBLE) {
        let cursor = if i == menu.row { "> " } else { "  " };
        let line = match row {
            Row::Bind(a) => bind_line(cfg, *a, menu.page == 1),
            Row::Set(s) => setting_line(cfg, *s),
        };
        out += &format!("{cursor}{line}\n");
    }
    if rows.len() > VISIBLE { out += &format!("  ({}/{})\n", menu.row + 1, rows.len()); }
    out += "\n";
    if let Some(c) = &menu.capture {
        let what = if c.pad { "a gamepad button or combination (hold RB/LB + flick or rotate the right stick for gestures)" } else { "a key, combination or mouse button" };
        out += &format!("Binding \"{}\": press and release {what}.  Escape cancels.\n", c.action.label());
    } else if menu.page <= 1 {
        out += "Up/Down select   Enter/A add binding   Delete/X clear   Backspace/Y default\n";
    } else {
        out += "Up/Down select   Left/Right or Enter/A change\n";
    }
    out += "Tab/RB next page   Shift+Tab/LB previous   F10 / Esc / B / Select+Start close\n";
    out += &format!("Saved to {}", cfg.path.as_ref().map_or("(no path)".into(), |p| p.display().to_string()));
    if !menu.message.is_empty() { out += &format!("\n{}", menu.message); }
    out
}

pub struct PcMenuPlugin;
impl Plugin for PcMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsMenu>()
            .add_systems(Startup, spawn_panel)
            .add_systems(Update, (menu.after(crate::input), apply_live, rumble));
    }
}
fn spawn_panel(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(40),
                top: px(40),
                padding: UiRect::all(px(16)),
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.92)),
            GlobalZIndex(10),
            Visibility::Hidden,
            MenuPanel,
        ))
        .with_child((
            Text::new(""),
            TextFont { font_size: bevy::text::FontSize::Px(15.), ..default() },
            TextColor(Color::WHITE),
            MenuText,
        ));
}
/// Raw navigation edges for the menu (fixed, so a bad binding can never lock you out).
struct Nav { up: bool, down: bool, left: bool, right: bool, next: bool, prev: bool, ok: bool, clear: bool, default: bool, close: bool }
fn nav(keys: &ButtonInput<KeyCode>, pad: Option<&Gamepad>, stick_edge: (i32, i32)) -> Nav {
    let k = |c| keys.just_pressed(c);
    let p = |b| pad.is_some_and(|g| g.just_pressed(b));
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let select_start = pad.is_some_and(|g| g.pressed(GamepadButton::Select) && g.pressed(GamepadButton::Start)
        && (g.just_pressed(GamepadButton::Select) || g.just_pressed(GamepadButton::Start)));
    use GamepadButton as B;
    Nav {
        up: k(KeyCode::ArrowUp) || k(KeyCode::KeyW) || p(B::DPadUp) || stick_edge.1 > 0,
        down: k(KeyCode::ArrowDown) || k(KeyCode::KeyS) || p(B::DPadDown) || stick_edge.1 < 0,
        left: k(KeyCode::ArrowLeft) || k(KeyCode::KeyA) || p(B::DPadLeft) || stick_edge.0 < 0,
        right: k(KeyCode::ArrowRight) || k(KeyCode::KeyD) || p(B::DPadRight) || stick_edge.0 > 0,
        next: (k(KeyCode::Tab) && !shift) || k(KeyCode::PageDown) || p(B::RightTrigger),
        prev: (k(KeyCode::Tab) && shift) || k(KeyCode::PageUp) || p(B::LeftTrigger),
        ok: k(KeyCode::Enter) || k(KeyCode::NumpadEnter) || k(KeyCode::Space) || p(B::South),
        clear: k(KeyCode::Delete) || p(B::West),
        default: k(KeyCode::Backspace) || p(B::North),
        close: k(KeyCode::Escape) || k(KeyCode::F10) || p(B::East) || select_start,
    }
}
#[allow(clippy::too_many_arguments)]
fn menu(
    mut menu: ResMut<SettingsMenu>,
    mut cfg: ResMut<PcConfig>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    time: Res<Time>,
    mut panel: Query<&mut Visibility, With<MenuPanel>>,
    mut text: Query<&mut Text, With<MenuText>>,
    mut stick_dir: Local<(i32, i32)>,
    mut gestures: Local<crate::pc_config::StickGestures>,
) {
    let shown = menu.open;
    for mut v in &mut panel {
        v.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
    }
    if !menu.open { return; }
    let pad = pads.iter().next();
    // Left stick navigates like the D-pad, with a held repeat.
    let ls = pad.map_or(Vec2::ZERO, |g| g.left_stick());
    let dir = (if ls.x > 0.6 { 1 } else if ls.x < -0.6 { -1 } else { 0 }, if ls.y > 0.6 { 1 } else if ls.y < -0.6 { -1 } else { 0 });
    let mut edge = (0, 0);
    if dir != *stick_dir || (dir != (0, 0) && menu.repeat <= 0.) {
        edge = dir;
        menu.repeat = if dir != *stick_dir { 0.4 } else { 0.12 };
    }
    menu.repeat -= time.delta_secs();
    *stick_dir = dir;
    let stick_event = pad.and_then(|g| gestures.update(g.right_stick(), cfg.gamepad.gesture_threshold));
    if menu.just_opened {
        menu.just_opened = false;
    } else if let Some(mut c) = menu.capture.take() {
        c.elapsed += time.delta_secs();
        if keys.just_pressed(KeyCode::Escape) || c.elapsed > 8. {
            menu.message = "Binding cancelled.".into();
        } else {
            let mut pressed: Vec<Input> = Vec::new();
            if c.pad {
                if let Some(g) = pad {
                    pressed.extend(PAD.iter().filter(|(_, b)| g.pressed(*b)).map(|(_, b)| Input::Pad(*b)));
                }
            } else {
                pressed.extend(KEYS.iter().filter(|k| keys.pressed(**k)).map(|k| Input::Key(*k)));
                pressed.extend([MouseButton::Left, MouseButton::Right, MouseButton::Middle, MouseButton::Back, MouseButton::Forward]
                    .into_iter().filter(|m| mouse.pressed(*m)).map(Input::Mouse));
            }
            // The Enter/A that started the capture is still down: arm only once every key,
            // mouse button and pad button has been released.
            let anything = keys.get_pressed().next().is_some() || mouse.get_pressed().next().is_some()
                || pad.is_some_and(|g| PAD.iter().any(|(_, b)| g.pressed(*b)));
            if !c.armed {
                c.armed = !anything;
                pressed.clear();
            }
            for i in &pressed {
                if !c.held.contains(i) { c.held.push(*i); }
            }
            let stick = if c.pad && c.armed { stick_event } else { None };
            let done = stick.is_some() || (!c.held.is_empty() && pressed.is_empty());
            if done {
                let mut chord = c.held.clone();
                // Modifiers first so the name reads RB+Up, not Up+RB.
                chord.sort_by_key(|i| !matches!(i, Input::Pad(GamepadButton::LeftTrigger | GamepadButton::RightTrigger) | Input::Key(KeyCode::ShiftLeft | KeyCode::ShiftRight | KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::AltLeft | KeyCode::AltRight)));
                if let Some(s) = stick { chord.push(Input::Stick(s)); }
                let chord = Chord(chord);
                let mut moved = Vec::new();
                for other in Action::ALL {
                    if other == c.action { continue; }
                    let mut list = cfg.device_bindings(other, c.pad);
                    if list.contains(&chord) {
                        list.retain(|x| *x != chord);
                        cfg.set_device_bindings(other, c.pad, list);
                        moved.push(other.label());
                    }
                }
                let mut list = cfg.device_bindings(c.action, c.pad);
                if !list.contains(&chord) { list.push(chord.clone()); }
                if list.len() > 4 { list.remove(0); }
                cfg.set_device_bindings(c.action, c.pad, list);
                menu.message = if moved.is_empty() { format!("Bound {}.", chord.name()) } else { format!("Bound {} (removed from: {}).", chord.name(), moved.join(", ")) };
                cfg.save();
            } else {
                menu.capture = Some(c);
            }
        }
    } else {
        let n = nav(&keys, pad, edge);
        let rows = rows(menu.page);
        if n.close {
            menu.open = false;
            menu.block_until_release = true;
            menu.message.clear();
        } else if n.next || n.prev {
            menu.page = (menu.page + if n.next { 1 } else { PAGES.len() - 1 }) % PAGES.len();
            menu.row = 0;
            menu.message.clear();
        } else if n.up {
            menu.row = (menu.row + rows.len() - 1) % rows.len();
        } else if n.down {
            menu.row = (menu.row + 1) % rows.len();
        } else {
            match rows[menu.row.min(rows.len() - 1)] {
                Row::Bind(action) => {
                    let pad_page = menu.page == 1;
                    if n.ok {
                        menu.capture = Some(Capture { action, pad: pad_page, held: Vec::new(), elapsed: 0., armed: false });
                        menu.message.clear();
                    } else if n.clear {
                        cfg.set_device_bindings(action, pad_page, Vec::new());
                        menu.message = format!("Cleared {}.", action.label());
                        cfg.save();
                    } else if n.default {
                        let defaults = if pad_page { action.default_gamepad() } else { action.default_keyboard() };
                        cfg.set_device_bindings(action, pad_page, defaults.iter().filter_map(|s| Chord::parse(s)).collect());
                        menu.message = format!("Restored default for {}.", action.label());
                        cfg.save();
                    }
                }
                Row::Set(s) => {
                    let d = if n.left { -1 } else if n.right || n.ok { 1 } else { 0 };
                    if d != 0 {
                        if let Some(m) = adjust(&mut cfg, s, d) { menu.message = m; }
                        cfg.save();
                    }
                }
            }
        }
    }
    let value = render(&menu, &cfg);
    for mut t in &mut text {
        if t.as_str() != value { **t = value.clone(); }
    }
}
/// Window mode, vsync, color mode and master volume follow the config (env wins).
fn apply_live(
    cfg: Res<PcConfig>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut sinks: Query<&mut AudioSink>,
    mut global: ResMut<GlobalVolume>,
    mut last_volume: Local<Option<f32>>,
) {
    let volume = cfg.effective_volume();
    // A sink spawned after the last change (the native PCM player starts at Startup)
    // still gets the current volume: re-apply until every sink matches.
    let mut stale = *last_volume != Some(volume);
    if !sinks.is_empty() && stale {
        for mut s in &mut sinks { s.set_volume(Volume::Linear(volume)); }
        *last_volume = Some(volume);
        stale = false;
    }
    if stale || cfg.is_changed() {
        global.volume = Volume::Linear(volume);
    }
    if !cfg.is_changed() { return; }
    let video = cfg.effective_video();
    for mut w in &mut windows {
        let present = if video.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };
        if w.present_mode != present { w.present_mode = present; }
        let mode = if video.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
        if w.mode != mode { w.mode = mode; }
    }
    let (color, legacy) = match cfg.effective_color() {
        ColorMode::Original => (false, false),
        ColorMode::Color => (true, false),
        ColorMode::Legacy => (true, true),
    };
    crate::COLOR_PACK.store(color, Ordering::Relaxed);
    crate::LEGACY_PACK.store(legacy, Ordering::Relaxed);
}
/// The game's remote motor (WPADControlMotor via the input HLE) drives the first
/// gamepad's motors. Requests are renewed while on so a lost Stop cannot stick.
fn rumble(
    cfg: Res<PcConfig>,
    menu: Res<SettingsMenu>,
    pads: Query<Entity, With<Gamepad>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut requests: MessageWriter<GamepadRumbleRequest>,
    mut state: Local<(bool, f32)>,
) {
    let Some(gamepad) = pads.iter().next() else { return };
    let focused = windows.single().is_ok_and(|w| w.focused);
    let on = host::rumble(0) && cfg.gamepad.rumble && cfg.gamepad.rumble_strength > 0. && focused && !menu.open;
    let was = state.0;
    state.1 -= time.delta_secs();
    if on && (!was || state.1 <= 0.) {
        let s = cfg.gamepad.rumble_strength;
        requests.write(GamepadRumbleRequest::Add {
            duration: Duration::from_millis(150),
            intensity: GamepadRumbleIntensity { strong_motor: s, weak_motor: s * 0.6 },
            gamepad,
        });
        state.1 = 0.1;
    } else if !on && was {
        requests.write(GamepadRumbleRequest::Stop { gamepad });
    }
    state.0 = on;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_adjust_wraps_clamps_and_resets() {
        let mut cfg = PcConfig::default();
        for _ in 0..100 { adjust(&mut cfg, Setting::MouseSens, 1); }
        assert!((cfg.camera.mouse_sensitivity - 5.).abs() < 1e-4);
        adjust(&mut cfg, Setting::Volume, -1);
        assert!((cfg.audio.master_volume - 0.95).abs() < 1e-4);
        adjust(&mut cfg, Setting::InternalRes, -1);
        assert_eq!(cfg.video.internal_res, "4");
        adjust(&mut cfg, Setting::Color, 1);
        assert_eq!(cfg.color, ColorMode::Legacy);
        adjust(&mut cfg, Setting::InvertY, 1);
        assert!(cfg.camera.invert_y);
        cfg.path = Some("x.toml".into());
        adjust(&mut cfg, Setting::ResetAll, -1);
        assert!(cfg.camera.invert_y);
        adjust(&mut cfg, Setting::ResetAll, 1);
        assert!(!cfg.camera.invert_y && cfg.path.is_some() && cfg.color == ColorMode::Color);
    }
    #[test]
    fn every_page_renders_every_row() {
        let cfg = PcConfig::default();
        let mut m = SettingsMenu::default();
        for page in 0..PAGES.len() {
            m.page = page;
            for row in 0..rows(page).len() {
                m.row = row;
                assert!(render(&m, &cfg).lines().any(|l| l.starts_with("> ")));
            }
        }
        m.page = 1;
        m.row = rows(1).iter().position(|r| *r == Row::Bind(Action::RightArmCircle)).unwrap();
        assert!(render(&m, &cfg).contains("PadRB+PadRStickCW"));
    }
}
/// F1 help, generated from the current bindings (keyboard/mouse | gamepad).
pub fn help_text(cfg: &PcConfig) -> String {
    let names = |a: Action, pad: bool| {
        let list = cfg.device_bindings(a, pad);
        if list.is_empty() { "-".to_string() } else {
            list.iter().map(|c| c.name().replace("Key", "").replace("Digit", "").replace("Pad", "")).collect::<Vec<_>>().join(" / ")
        }
    };
    let mut out = String::from("PC controls (F1)                        keyboard & mouse | gamepad\n\n");
    let moves = [Action::MoveUp, Action::MoveLeft, Action::MoveDown, Action::MoveRight].map(|a| names(a, false)).join(" ");
    out += &format!("{:<28}{moves} | left stick\n", "Move (double-tap to run)");
    out += &format!("{:<28}mouse | right stick  (free camera: MW_PC_FREE_CAMERA=1)\n", "Camera");
    for a in [Action::Confirm, Action::Attack, Action::Chainsaw, Action::Jump, Action::Dodge, Action::LockOn, Action::Shake,
        Action::Pickup, Action::Taunt, Action::Weapon1, Action::Weapon2, Action::Back, Action::Pause, Action::Settings] {
        out += &format!("{:<28}{} | {}\n", short(a), names(a, false), names(a, true));
    }
    out += "Shake QTE: also wiggle left/right fast (A/D or left stick) when the game shows it\n";
    out += "\nGestures: right arm (remote) | left arm (Nunchuk)\n";
    let arms = [
        ("left/right/up/down", [Action::RightArmLeft, Action::RightArmRight, Action::RightArmUp, Action::RightArmDown], [Action::LeftArmLeft, Action::LeftArmRight, Action::LeftArmUp, Action::LeftArmDown]),
    ];
    for (label, r, l) in arms {
        let join = |v: [Action; 4], pad: bool| v.map(|a| names(a, pad)).join(" ");
        out += &format!("  {label}: {} | {}\n", join(r, false), join(l, false));
    }
    for (label, r, l) in [("push", Action::RightArmPush, Action::LeftArmPush), ("pull", Action::RightArmPull, Action::LeftArmPull),
        ("circle", Action::RightArmCircle, Action::LeftArmCircle), ("reverse circle", Action::RightArmReverseCircle, Action::LeftArmReverseCircle)] {
        out += &format!("  {label}: {} | {}\n", names(r, false), names(l, false));
    }
    out += "Gamepad: hold RB (right arm) or LB (left arm) and flick the right stick for a\n";
    out += "strike, or rotate it for a circle; RB/LB + D-pad up/down = push/pull.\n";
    out += "F10 or Select+Start: PC settings (rebinding, camera, video, color, volume)\n";
    out += "C: color pack   F2: night grade   F3: legacy pack";
    out
}
fn short(a: Action) -> &'static str {
    match a {
        Action::Confirm => "Confirm/attack (hold: skip)",
        Action::Attack => "Attack (hold: grab)",
        Action::LockOn => "Camera / hold lock-on",
        Action::Shake => "Shake (struggle, finisher)",
        Action::Settings => "PC settings",
        other => other.label(),
    }
}
