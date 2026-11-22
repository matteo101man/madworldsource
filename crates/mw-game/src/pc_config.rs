//! PC settings and rebindable controls, persisted as `pc-controls.toml` next to the exe
//! (or `MW_PC_CONTROLS=<path>`). Missing or unknown entries fall back to the defaults,
//! which reproduce the original fixed PC bindings. Env vars (MW_INTERNAL_RES, MW_VSYNC,
//! MW_FULLSCREEN, MW_COLOR, MW_MASTER_VOLUME) override the file and are never saved.
//! Only host input is produced here; the game still sees one Wii remote + Nunchuk.
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Every rebindable action. `name` is the config key and the binding name other code
/// (PC prompt glyphs, F1 help) refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    MoveUp, MoveDown, MoveLeft, MoveRight,
    Confirm, Attack, Chainsaw, Back, Jump, Dodge, LockOn, Shake,
    Pickup, Taunt, Weapon1, Weapon2, Pause, Plus,
    RightArmLeft, RightArmRight, RightArmUp, RightArmDown,
    RightArmPush, RightArmPull, RightArmCircle, RightArmReverseCircle,
    LeftArmLeft, LeftArmRight, LeftArmUp, LeftArmDown,
    LeftArmPush, LeftArmPull, LeftArmCircle, LeftArmReverseCircle,
    Settings,
}
use Action::*;
impl Action {
    pub const ALL: [Action; 35] = [
        MoveUp, MoveDown, MoveLeft, MoveRight, Confirm, Attack, Chainsaw, Back, Jump, Dodge, LockOn, Shake,
        Pickup, Taunt, Weapon1, Weapon2, Pause, Plus,
        RightArmLeft, RightArmRight, RightArmUp, RightArmDown, RightArmPush, RightArmPull, RightArmCircle, RightArmReverseCircle,
        LeftArmLeft, LeftArmRight, LeftArmUp, LeftArmDown, LeftArmPush, LeftArmPull, LeftArmCircle, LeftArmReverseCircle,
        Settings,
    ];
    pub fn name(self) -> &'static str {
        match self {
            MoveUp => "move_up", MoveDown => "move_down", MoveLeft => "move_left", MoveRight => "move_right",
            Confirm => "confirm", Attack => "attack", Chainsaw => "chainsaw", Back => "back", Jump => "jump",
            Dodge => "dodge", LockOn => "lock_on", Shake => "shake", Pickup => "pickup", Taunt => "taunt",
            Weapon1 => "weapon_1", Weapon2 => "weapon_2", Pause => "pause", Plus => "plus",
            RightArmLeft => "right_arm_left", RightArmRight => "right_arm_right", RightArmUp => "right_arm_up",
            RightArmDown => "right_arm_down", RightArmPush => "right_arm_push", RightArmPull => "right_arm_pull",
            RightArmCircle => "right_arm_circle", RightArmReverseCircle => "right_arm_reverse_circle",
            LeftArmLeft => "left_arm_left", LeftArmRight => "left_arm_right", LeftArmUp => "left_arm_up",
            LeftArmDown => "left_arm_down", LeftArmPush => "left_arm_push", LeftArmPull => "left_arm_pull",
            LeftArmCircle => "left_arm_circle", LeftArmReverseCircle => "left_arm_reverse_circle",
            Settings => "settings",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            MoveUp => "Move up", MoveDown => "Move down", MoveLeft => "Move left", MoveRight => "Move right",
            Confirm => "Attack / confirm (hold: skip cutscene)", Attack => "Attack (hold: grab)",
            Chainsaw => "Chainsaw (hold)", Back => "Menu back (Wii B)", Jump => "Jump",
            Dodge => "Dodge", LockOn => "Camera / hold lock-on", Shake => "Shake (struggle / finisher)",
            Pickup => "Pick up", Taunt => "Taunt", Weapon1 => "Weapon 1", Weapon2 => "Weapon 2",
            Pause => "Pause", Plus => "Wii Plus",
            RightArmLeft => "Right arm: left", RightArmRight => "Right arm: right (horizontal strike)",
            RightArmUp => "Right arm: up (uppercut)", RightArmDown => "Right arm: down",
            RightArmPush => "Right arm: push", RightArmPull => "Right arm: pull",
            RightArmCircle => "Right arm: circle", RightArmReverseCircle => "Right arm: reverse circle",
            LeftArmLeft => "Left arm: left", LeftArmRight => "Left arm: right", LeftArmUp => "Left arm: up",
            LeftArmDown => "Left arm: down", LeftArmPush => "Left arm: push", LeftArmPull => "Left arm: pull",
            LeftArmCircle => "Left arm: circle", LeftArmReverseCircle => "Left arm: reverse circle",
            Settings => "PC settings menu",
        }
    }
    pub fn from_name(name: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|a| a.name() == name)
    }
    /// Keyboard and mouse defaults: the original fixed PC bindings.
    pub fn default_keyboard(self) -> &'static [&'static str] {
        match self {
            MoveUp => &["KeyW"], MoveDown => &["KeyS"], MoveLeft => &["KeyA"], MoveRight => &["KeyD"],
            Confirm => &["Enter"], Attack => &["MouseLeft", "KeyE"], Chainsaw => &["MouseRight"],
            Back => &["Backspace"], Jump => &["Space"], Dodge => &["ShiftLeft", "ShiftRight"],
            LockOn => &["KeyQ", "MouseMiddle"], Shake => &["KeyX"], Pickup => &["KeyG"], Taunt => &["KeyT"],
            Weapon1 => &["Digit1"], Weapon2 => &["Digit2"], Pause => &["Escape"], Plus => &["Tab"],
            RightArmLeft => &["ArrowLeft"], RightArmRight => &["ArrowRight", "KeyF"],
            RightArmUp => &["ArrowUp", "KeyR"], RightArmDown => &["ArrowDown"],
            RightArmPush => &["KeyN"], RightArmPull => &["KeyM"], RightArmCircle => &["KeyV"],
            RightArmReverseCircle => &["KeyB"],
            LeftArmLeft => &["KeyJ"], LeftArmRight => &["KeyL"], LeftArmUp => &["KeyI"], LeftArmDown => &["KeyK"],
            LeftArmPush => &["KeyU"], LeftArmPull => &["KeyO"], LeftArmCircle => &["KeyH"],
            LeftArmReverseCircle => &["KeyY"],
            Settings => &["F10"],
        }
    }
    /// Xbox-layout defaults. The left stick always moves and the right stick is the
    /// camera; holding RB (right arm) or LB (left arm) turns right-stick flicks and
    /// rotations into the remote/Nunchuk gesture profiles instead.
    pub fn default_gamepad(self) -> &'static [&'static str] {
        match self {
            MoveUp | MoveDown | MoveLeft | MoveRight | Attack => &[],
            Confirm => &["PadA"], Back => &["PadB"], Jump => &["PadX"], Dodge => &["PadY"],
            Chainsaw => &["PadRT"], LockOn => &["PadLT", "PadR3"], Shake => &["PadL3", "PadLB+PadRB"],
            Pickup => &["PadUp"], Taunt => &["PadDown"], Weapon1 => &["PadLeft"], Weapon2 => &["PadRight"],
            Pause => &["PadStart"], Plus => &["PadSelect"],
            RightArmLeft => &["PadRB+PadRStickLeft"], RightArmRight => &["PadRB+PadRStickRight"],
            RightArmUp => &["PadRB+PadRStickUp"], RightArmDown => &["PadRB+PadRStickDown"],
            RightArmPush => &["PadRB+PadUp"], RightArmPull => &["PadRB+PadDown"],
            RightArmCircle => &["PadRB+PadRStickCW"], RightArmReverseCircle => &["PadRB+PadRStickCCW"],
            LeftArmLeft => &["PadLB+PadRStickLeft"], LeftArmRight => &["PadLB+PadRStickRight"],
            LeftArmUp => &["PadLB+PadRStickUp"], LeftArmDown => &["PadLB+PadRStickDown"],
            LeftArmPush => &["PadLB+PadUp"], LeftArmPull => &["PadLB+PadDown"],
            LeftArmCircle => &["PadLB+PadRStickCW"], LeftArmReverseCircle => &["PadLB+PadRStickCCW"],
            Settings => &["PadSelect+PadStart"],
        }
    }
}

/// Right-stick gesture events (see [`StickGestures`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StickDir { Up, Down, Left, Right, Cw, Ccw }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Input {
    Key(KeyCode),
    Mouse(MouseButton),
    Pad(GamepadButton),
    Stick(StickDir),
}
macro_rules! keys { ($($k:ident),* $(,)?) => { &[$(KeyCode::$k),*] }; }
/// Bindable keys; config names are the KeyCode variant names.
pub const KEYS: &[KeyCode] = keys![
    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ,
    KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Digit0, Digit1, Digit2, Digit3, Digit4, Digit5,
    Digit6, Digit7, Digit8, Digit9, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, ArrowUp, ArrowDown,
    ArrowLeft, ArrowRight, Space, Enter, Escape, Tab, Backspace, ShiftLeft, ShiftRight, ControlLeft,
    ControlRight, AltLeft, AltRight, CapsLock, Backquote, Minus, Equal, BracketLeft, BracketRight,
    Backslash, Semicolon, Quote, Comma, Period, Slash, Insert, Delete, Home, End, PageUp, PageDown,
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadEnter, NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide, NumpadDecimal,
];
const MOUSE: &[(&str, MouseButton)] = &[
    ("MouseLeft", MouseButton::Left), ("MouseRight", MouseButton::Right), ("MouseMiddle", MouseButton::Middle),
    ("MouseBack", MouseButton::Back), ("MouseForward", MouseButton::Forward),
];
pub const PAD: &[(&str, GamepadButton)] = &[
    ("PadA", GamepadButton::South), ("PadB", GamepadButton::East), ("PadX", GamepadButton::West),
    ("PadY", GamepadButton::North), ("PadLB", GamepadButton::LeftTrigger), ("PadRB", GamepadButton::RightTrigger),
    ("PadLT", GamepadButton::LeftTrigger2), ("PadRT", GamepadButton::RightTrigger2),
    ("PadL3", GamepadButton::LeftThumb), ("PadR3", GamepadButton::RightThumb),
    ("PadSelect", GamepadButton::Select), ("PadStart", GamepadButton::Start),
    ("PadUp", GamepadButton::DPadUp), ("PadDown", GamepadButton::DPadDown),
    ("PadLeft", GamepadButton::DPadLeft), ("PadRight", GamepadButton::DPadRight),
];
const STICK: &[(&str, StickDir)] = &[
    ("PadRStickUp", StickDir::Up), ("PadRStickDown", StickDir::Down), ("PadRStickLeft", StickDir::Left),
    ("PadRStickRight", StickDir::Right), ("PadRStickCW", StickDir::Cw), ("PadRStickCCW", StickDir::Ccw),
];
impl Input {
    pub fn parse(name: &str) -> Option<Input> {
        if let Some(k) = KEYS.iter().find(|k| format!("{k:?}") == name) {
            return Some(Input::Key(*k));
        }
        MOUSE.iter().find(|m| m.0 == name).map(|m| Input::Mouse(m.1))
            .or_else(|| PAD.iter().find(|p| p.0 == name).map(|p| Input::Pad(p.1)))
            .or_else(|| STICK.iter().find(|s| s.0 == name).map(|s| Input::Stick(s.1)))
    }
    pub fn name(self) -> String {
        match self {
            Input::Key(k) => format!("{k:?}"),
            Input::Mouse(m) => MOUSE.iter().find(|x| x.1 == m).map_or("Mouse?", |x| x.0).into(),
            Input::Pad(b) => PAD.iter().find(|x| x.1 == b).map_or("Pad?", |x| x.0).into(),
            Input::Stick(s) => STICK.iter().find(|x| x.1 == s).map_or("Stick?", |x| x.0).into(),
        }
    }
    pub fn is_gamepad(self) -> bool {
        matches!(self, Input::Pad(_) | Input::Stick(_))
    }
}
/// One binding: every input held, e.g. `PadRB+PadRStickUp`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord(pub Vec<Input>);
impl Chord {
    pub fn parse(text: &str) -> Option<Chord> {
        let inputs: Option<Vec<_>> = text.split('+').map(|s| Input::parse(s.trim())).collect();
        inputs.filter(|v| !v.is_empty()).map(Chord)
    }
    pub fn name(&self) -> String {
        self.0.iter().map(|i| i.name()).collect::<Vec<_>>().join("+")
    }
}

/// The gamepad as this frame saw it; built from Bevy's [`Gamepad`] so tests can fake one.
#[derive(Clone, Debug, Default)]
pub struct PadView {
    pub pressed: Vec<GamepadButton>,
    pub just_pressed: Vec<GamepadButton>,
    pub left: Vec2,
    pub right: Vec2,
}
impl PadView {
    pub fn from_gamepad(pad: &Gamepad) -> PadView {
        let all = PAD.iter().map(|p| p.1);
        PadView {
            pressed: all.clone().filter(|b| pad.pressed(*b)).collect(),
            just_pressed: all.filter(|b| pad.just_pressed(*b)).collect(),
            left: pad.left_stick(),
            right: pad.right_stick(),
        }
    }
}
/// Radial deadzone with rescale so motion starts at zero just outside it.
pub fn deadzone(v: Vec2, dz: f32) -> Vec2 {
    let m = v.length();
    if m <= dz || m < 1e-6 { Vec2::ZERO } else { v / m * ((m - dz) / (1. - dz).max(1e-3)).min(1.) }
}

/// Right-stick flicks and rotations. A flick is reported when the stick returns to
/// centre (so a rotation is not first read as a flick); a rotation as soon as it has
/// swept 270 degrees. Either way the stick must recentre before the next event.
#[derive(Clone, Debug, Default)]
pub struct StickGestures {
    active: bool,
    fired: bool,
    peak: Vec2,
    last_angle: f32,
    swept: f32,
}
pub const CIRCLE_SWEEP: f32 = std::f32::consts::PI * 1.5;
impl StickGestures {
    pub fn update(&mut self, v: Vec2, threshold: f32) -> Option<StickDir> {
        let m = v.length();
        if !self.active {
            if m < threshold { return None; }
            *self = StickGestures { active: true, fired: false, peak: v, last_angle: v.to_angle(), swept: 0. };
            return None;
        }
        if m < threshold * 0.5 {
            let flick = !self.fired && self.swept.abs() < std::f32::consts::FRAC_PI_2;
            self.active = false;
            if !flick { return None; }
            let p = self.peak;
            return Some(if p.x.abs() > p.y.abs() {
                if p.x > 0. { StickDir::Right } else { StickDir::Left }
            } else if p.y > 0. { StickDir::Up } else { StickDir::Down });
        }
        if m > self.peak.length() { self.peak = v; }
        if m >= threshold * 0.7 {
            let a = v.to_angle();
            let mut d = a - self.last_angle;
            if d > std::f32::consts::PI { d -= std::f32::consts::TAU; }
            if d < -std::f32::consts::PI { d += std::f32::consts::TAU; }
            self.swept += d;
            self.last_angle = a;
        }
        if !self.fired && self.swept.abs() >= CIRCLE_SWEEP {
            self.fired = true;
            // Screen-clockwise is a decreasing stick angle.
            return Some(if self.swept < 0. { StickDir::Cw } else { StickDir::Ccw });
        }
        None
    }
}

/// Raw device state for one frame.
pub struct Inputs<'a> {
    pub keys: &'a ButtonInput<KeyCode>,
    pub mouse: &'a ButtonInput<MouseButton>,
    pub pad: Option<&'a PadView>,
    pub stick: Option<StickDir>,
}
impl Inputs<'_> {
    fn pressed(&self, i: Input) -> bool {
        match i {
            Input::Key(k) => self.keys.pressed(k),
            Input::Mouse(m) => self.mouse.pressed(m),
            Input::Pad(b) => self.pad.is_some_and(|p| p.pressed.contains(&b)),
            Input::Stick(s) => self.stick == Some(s),
        }
    }
    fn just_pressed(&self, i: Input) -> bool {
        match i {
            Input::Key(k) => self.keys.just_pressed(k),
            Input::Mouse(m) => self.mouse.just_pressed(m),
            Input::Pad(b) => self.pad.is_some_and(|p| p.just_pressed.contains(&b)),
            Input::Stick(s) => self.stick == Some(s),
        }
    }
}

/// Resolved actions for one frame, the only thing `PcControls` reads.
#[derive(Clone, Debug, Default)]
pub struct ActionState {
    pub focused: bool,
    pressed: Vec<Action>,
    just: Vec<Action>,
    /// Left stick, deadzoned (analog movement, added to the digital move actions).
    pub stick: Vec2,
    /// Right stick for the camera; zero while a gesture modifier (RB/LB) is held.
    pub camera: Vec2,
}
impl ActionState {
    pub fn pressed(&self, a: Action) -> bool { self.pressed.contains(&a) }
    pub fn just_pressed(&self, a: Action) -> bool { self.just.contains(&a) }
}

#[derive(Clone, Debug)]
pub struct Bindings {
    pub map: BTreeMap<Action, Vec<Chord>>,
}
impl Default for Bindings {
    fn default() -> Self {
        let map = Action::ALL.into_iter().map(|a| {
            let all = a.default_keyboard().iter().chain(a.default_gamepad());
            (a, all.filter_map(|s| Chord::parse(s)).collect())
        }).collect();
        Bindings { map }
    }
}
impl Bindings {
    pub fn get(&self, a: Action) -> &[Chord] { self.map.get(&a).map_or(&[], |v| v.as_slice()) }
    /// Resolve actions. A binding is ignored while a longer binding that contains it
    /// is fully held, so RB+D-pad up pushes instead of also picking up.
    pub fn evaluate(&self, inputs: &Inputs, deadzone_radius: f32, focused: bool) -> ActionState {
        let mut s = ActionState { focused, ..default() };
        if !focused { return s; }
        let active: Vec<&Chord> = self.map.values().flatten()
            .filter(|c| c.0.iter().all(|i| inputs.pressed(*i))).collect();
        for (action, chords) in &self.map {
            for chord in chords {
                if !chord.0.iter().all(|i| inputs.pressed(*i)) { continue; }
                let shadowed = active.iter().any(|c| c.0.len() > chord.0.len() && chord.0.iter().all(|i| c.0.contains(i)));
                if shadowed { continue; }
                if !s.pressed.contains(action) { s.pressed.push(*action); }
                if chord.0.iter().any(|i| inputs.just_pressed(*i)) && !s.just.contains(action) { s.just.push(*action); }
            }
        }
        if let Some(pad) = inputs.pad {
            s.stick = deadzone(pad.left, deadzone_radius);
            let modifier = self.map.values().flatten().any(|c| {
                c.0.iter().any(|i| matches!(i, Input::Stick(_)))
                    && c.0.iter().filter(|i| !matches!(i, Input::Stick(_))).any(|i| inputs.pressed(*i))
            });
            if !modifier { s.camera = deadzone(pad.right, deadzone_radius); }
        }
        s
    }
}

/// Camera-look input for the free-camera code (pc-free-camera session), published
/// every frame. Raw values: sensitivity, invert-Y and auto-follow are applied by the
/// camera code from [`CameraSettings`].
#[derive(Resource, Default, Clone, Debug)]
pub struct CameraLook {
    /// Right stick, deadzoned, -1..1, +y up; zero while RB/LB gesture modifiers are held.
    pub stick: Vec2,
    /// Mouse motion this frame in pixels (+y down, as Bevy reports it).
    pub mouse: Vec2,
    /// Lock-on/camera action was just pressed (game C button). The core already
    /// recenters on C; this is for host-side camera code that wants the edge.
    #[allow(dead_code)]
    pub recenter: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode { Original, #[default] Color, Legacy }
impl ColorMode {
    pub fn parse(s: &str) -> Option<ColorMode> {
        match s.to_ascii_lowercase().as_str() { "original" | "0" | "off" => Some(Self::Original), "color" | "1" | "on" => Some(Self::Color), "legacy" | "2" => Some(Self::Legacy), _ => None }
    }
    pub fn label(self) -> &'static str { match self { Self::Original => "original (monochrome)", Self::Color => "color", Self::Legacy => "legacy color pack" } }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    pub mouse_sensitivity: f32,
    pub stick_sensitivity: f32,
    pub invert_y: bool,
    pub auto_follow: bool,
    /// Orbit the camera while locked on (free camera; F5 and MW_PC_CAMERA_LOCK_ORBIT override).
    pub lock_orbit: bool,
}
impl Default for CameraSettings {
    fn default() -> Self { Self { mouse_sensitivity: 1., stick_sensitivity: 1., invert_y: false, auto_follow: true, lock_orbit: true } }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GamepadSettings {
    pub deadzone: f32,
    /// Right-stick deflection that starts a flick or rotation gesture.
    pub gesture_threshold: f32,
    pub rumble: bool,
    pub rumble_strength: f32,
}
impl Default for GamepadSettings {
    fn default() -> Self { Self { deadzone: 0.2, gesture_threshold: 0.6, rumble: true, rumble_strength: 0.7 } }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoSettings {
    /// "auto" (match window height) or 1..4; applied at the next launch.
    pub internal_res: String,
    pub vsync: bool,
    pub fullscreen: bool,
}
impl Default for VideoSettings {
    fn default() -> Self { Self { internal_res: "auto".into(), vsync: true, fullscreen: false } }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings { pub master_volume: f32, #[serde(flatten)]pub sound:mw_sim::ui::SoundSettings }
impl Default for AudioSettings { fn default() -> Self { Self { master_volume: 1.,sound:default() } } }

/// On-disk form. Binding maps hold action name -> list of chord strings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct File {
    version: u32,
    keyboard: BTreeMap<String, Vec<String>>,
    gamepad: BTreeMap<String, Vec<String>>,
    gamepad_options: GamepadSettings,
    camera: CameraSettings,
    video: VideoSettings,
    color: ColorFile,
    audio: AudioSettings,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
struct ColorFile { mode: ColorMode }
impl Default for File {
    fn default() -> Self {
        File { version: 1, keyboard: BTreeMap::new(), gamepad: BTreeMap::new(), gamepad_options: default(), camera: default(), video: default(), color: default(), audio: default() }
    }
}

#[derive(Resource, Clone, Debug)]
pub struct PcConfig {
    pub bindings: Bindings,
    pub gamepad: GamepadSettings,
    pub camera: CameraSettings,
    pub video: VideoSettings,
    pub color: ColorMode,
    pub audio: AudioSettings,
    pub path: Option<PathBuf>,
}
impl Default for PcConfig {
    fn default() -> Self {
        PcConfig { bindings: default(), gamepad: default(), camera: default(), video: default(), color: default(), audio: default(), path: None }
    }
}
pub const FILE_NAME: &str = "pc-controls.toml";
pub fn default_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("MW_PC_CONTROLS") { return Some(p.into()); }
    Some(std::env::current_exe().ok()?.parent()?.join(FILE_NAME))
}
impl PcConfig {
    /// Parse a config; problems are reported and the affected entries keep defaults.
    pub fn from_toml(text: &str) -> (PcConfig, Vec<String>) {
        let mut warnings = Vec::new();
        let file: File = toml::from_str(text).unwrap_or_else(|e| {
            warnings.push(format!("{FILE_NAME} unreadable, using defaults: {e}"));
            File::default()
        });
        let mut cfg = PcConfig { gamepad: file.gamepad_options, camera: file.camera, video: file.video, color: file.color.mode, audio: file.audio, ..default() };
        for (table, entries, pad) in [("keyboard", &file.keyboard, false), ("gamepad", &file.gamepad, true)] {
            for (name, list) in entries {
                let Some(action) = Action::from_name(name) else {
                    warnings.push(format!("[{table}] unknown action {name:?}"));
                    continue;
                };
                let mut chords = Vec::new();
                for text in list {
                    match Chord::parse(text) {
                        Some(c) if c.0.iter().all(|i| i.is_gamepad() == pad) => chords.push(c),
                        _ => warnings.push(format!("[{table}] {name}: bad binding {text:?}")),
                    }
                }
                cfg.set_device_bindings(action, pad, chords);
            }
        }
        cfg.gamepad.deadzone = cfg.gamepad.deadzone.clamp(0., 0.9);
        cfg.gamepad.gesture_threshold = cfg.gamepad.gesture_threshold.clamp(0.2, 0.95);
        cfg.gamepad.rumble_strength = cfg.gamepad.rumble_strength.clamp(0., 1.);
        cfg.audio.master_volume = cfg.audio.master_volume.clamp(0., 1.);
        cfg.camera.mouse_sensitivity = cfg.camera.mouse_sensitivity.clamp(0.05, 10.);
        cfg.camera.stick_sensitivity = cfg.camera.stick_sensitivity.clamp(0.05, 10.);
        (cfg, warnings)
    }
    pub fn to_toml(&self) -> String {
        let table = |pad: bool| self.bindings.map.iter().map(|(a, chords)| {
            (a.name().to_string(), chords.iter().filter(|c| c.0[0].is_gamepad() == pad).map(Chord::name).collect())
        }).collect();
        let file = File {
            version: 1, keyboard: table(false), gamepad: table(true), gamepad_options: self.gamepad.clone(),
            camera: self.camera.clone(), video: self.video.clone(), color: ColorFile { mode: self.color }, audio: self.audio.clone(),
        };
        let body = toml::to_string_pretty(&file).unwrap_or_default();
        format!("# MadWorld native PC settings. Edit here or in-game (F10 / Select+Start).\n\
                 # Keys use Bevy KeyCode names (KeyE, Space, ShiftLeft, F1...), mouse MouseLeft/Right/Middle/Back/Forward,\n\
                 # pad PadA/B/X/Y/LB/RB/LT/RT/L3/R3/Select/Start/Up/Down/Left/Right and PadRStickUp/Down/Left/Right/CW/CCW;\n\
                 # join with + for a chord. Delete this file to restore defaults.\n\
                 # Env vars override: MW_INTERNAL_RES, MW_VSYNC, MW_FULLSCREEN, MW_COLOR, MW_MASTER_VOLUME.\n\n{body}")
    }
    /// Replace one device's bindings (keyboard+mouse, or gamepad) for an action.
    pub fn set_device_bindings(&mut self, a: Action, pad: bool, chords: Vec<Chord>) {
        let list = self.bindings.map.entry(a).or_default();
        list.retain(|c| c.0[0].is_gamepad() != pad);
        list.extend(chords);
    }
    pub fn device_bindings(&self, a: Action, pad: bool) -> Vec<Chord> {
        self.bindings.get(a).iter().filter(|c| c.0[0].is_gamepad() == pad).cloned().collect()
    }
    pub fn load() -> PcConfig {
        Self::load_from(default_path())
    }
    pub fn load_from(path: Option<PathBuf>) -> PcConfig {
        let mut cfg = match path.as_ref().map(std::fs::read_to_string) {
            Some(Ok(text)) => {
                let (cfg, warnings) = PcConfig::from_toml(&text);
                for w in warnings { eprintln!("[pc-config] {w}"); }
                cfg
            }
            _ => PcConfig::default(),
        };
        let v = cfg.effective_video();
        eprintln!(
            "[pc-config] {} ({}): res={} vsync={} fullscreen={} color={:?} volume={:.2} mouse_sens={:.1} invert_y={} rumble={}",
            path.as_ref().map_or("(no path)".into(), |p| p.display().to_string()),
            if path.as_ref().is_some_and(|p| p.exists()) { "loaded" } else { "defaults" },
            v.internal_res, v.vsync, v.fullscreen, cfg.effective_color(), cfg.effective_volume(),
            cfg.camera.mouse_sensitivity, cfg.camera.invert_y, cfg.gamepad.rumble,
        );
        cfg.path = path;
        cfg
    }
    pub fn save(&self) {
        let Some(path) = &self.path else { return };
        let tmp = path.with_extension("toml.tmp");
        let result = std::fs::write(&tmp, self.to_toml()).and_then(|_| std::fs::rename(&tmp, path));
        match result {
            Ok(()) => info!("[pc-config] saved {}", path.display()),
            Err(e) => warn!("[pc-config] could not save {}: {e}", path.display()),
        }
    }
    /// Values actually in effect: env vars win over the file.
    pub fn effective_video(&self) -> VideoSettings {
        let mut v = self.video.clone();
        if let Ok(r) = std::env::var("MW_INTERNAL_RES") { v.internal_res = r; }
        if let Some(b) = env_bool("MW_VSYNC") { v.vsync = b; }
        if let Some(b) = env_bool("MW_FULLSCREEN") { v.fullscreen = b; }
        v
    }
    pub fn effective_color(&self) -> ColorMode {
        std::env::var("MW_COLOR").ok().and_then(|s| ColorMode::parse(&s)).unwrap_or(self.color)
    }
    pub fn effective_volume(&self) -> f32 {
        std::env::var("MW_MASTER_VOLUME").ok().and_then(|s| s.parse::<f32>().ok()).map_or(self.audio.master_volume, |v| v.clamp(0., 1.))
    }
}
pub fn env_bool(name: &str) -> Option<bool> {
    match std::env::var(name).ok()?.to_ascii_lowercase().as_str() { "1" | "true" | "on" | "yes" => Some(true), "0" | "false" | "off" | "no" => Some(false), _ => None }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs<'a>(keys: &'a ButtonInput<KeyCode>, mouse: &'a ButtonInput<MouseButton>, pad: Option<&'a PadView>, stick: Option<StickDir>) -> Inputs<'a> {
        Inputs { keys, mouse, pad, stick }
    }
    #[test]
    fn rust_sound_levels_survive_native_config_round_trip(){
        let (config,warnings)=PcConfig::from_toml("[audio]\nmaster_volume=0.7\nmusic_level=9\nsfx_level=3\ncutscene_level=4\nannouncer_level=0\nsubtitles=true\n");
        assert!(warnings.is_empty());assert_eq!(config.audio.sound.levels(),[9,3,4,0]);assert!(config.audio.sound.subtitles);
        let (again,warnings)=PcConfig::from_toml(&config.to_toml());assert!(warnings.is_empty());assert_eq!(again.audio,config.audio);
    }
    #[test]
    fn every_default_binding_parses_and_names_round_trip() {
        for a in Action::ALL {
            assert_eq!(Action::from_name(a.name()), Some(a));
            for s in a.default_keyboard().iter().chain(a.default_gamepad()) {
                let c = Chord::parse(s).unwrap_or_else(|| panic!("{s}"));
                assert_eq!(&c.name(), s);
            }
            assert!(a.default_keyboard().iter().all(|s| !Chord::parse(s).unwrap().0[0].is_gamepad()));
            assert!(a.default_gamepad().iter().all(|s| Chord::parse(s).unwrap().0.iter().all(|i| i.is_gamepad())));
        }
        assert_eq!(Chord::parse("KeyNope"), None);
    }
    #[test]
    fn config_round_trips_and_defaults_fill_gaps() {
        let mut cfg = PcConfig::default();
        cfg.set_device_bindings(Jump, false, vec![Chord::parse("KeyZ").unwrap()]);
        cfg.set_device_bindings(Jump, true, vec![Chord::parse("PadA").unwrap()]);
        cfg.camera.invert_y = true;
        cfg.camera.mouse_sensitivity = 2.5;
        cfg.video.internal_res = "3".into();
        cfg.color = ColorMode::Legacy;
        cfg.audio.master_volume = 0.4;
        cfg.gamepad.rumble = false;
        let (back, warnings) = PcConfig::from_toml(&cfg.to_toml());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(back.device_bindings(Jump, false), vec![Chord::parse("KeyZ").unwrap()]);
        assert_eq!(back.device_bindings(Jump, true), vec![Chord::parse("PadA").unwrap()]);
        assert_eq!(back.bindings.get(Attack), PcConfig::default().bindings.get(Attack));
        assert_eq!((back.camera, back.video, back.color, back.audio, back.gamepad), (cfg.camera, cfg.video, cfg.color, cfg.audio, cfg.gamepad));
        // A partial file keeps defaults for everything it omits; an empty list unbinds.
        let (partial, w) = PcConfig::from_toml("[keyboard]\njump = []\nbogus = [\"KeyA\"]\ndodge = [\"PadA\", \"KeyZ\"]\n[camera]\ninvert_y = true\n");
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(partial.device_bindings(Jump, false).is_empty());
        assert_eq!(partial.device_bindings(Jump, true), vec![Chord::parse("PadX").unwrap()]);
        assert_eq!(partial.device_bindings(Dodge, false), vec![Chord::parse("KeyZ").unwrap()]);
        assert!(partial.camera.invert_y && partial.camera.auto_follow);
        let (broken, w) = PcConfig::from_toml("this is = = not toml");
        assert_eq!(w.len(), 1);
        assert_eq!(broken.bindings.get(Jump), PcConfig::default().bindings.get(Jump));
    }
    #[test]
    fn settings_persist_through_the_file() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evidence/pc-config-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{FILE_NAME}.{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let fresh = PcConfig::load_from(Some(path.clone()));
        assert!(!path.exists(), "loading never creates the file");
        assert_eq!(fresh.bindings.get(Jump), PcConfig::default().bindings.get(Jump));
        let mut cfg = fresh;
        cfg.audio.master_volume = 0.25;
        cfg.set_device_bindings(Pause, true, vec![Chord::parse("PadSelect").unwrap()]);
        cfg.save();
        let back = PcConfig::load_from(Some(path.clone()));
        assert_eq!(back.audio.master_volume, 0.25);
        assert_eq!(back.device_bindings(Pause, true), vec![Chord::parse("PadSelect").unwrap()]);
        assert_eq!(back.path.as_deref(), Some(path.as_path()));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
    #[test]
    fn defaults_match_current_behavior() {
        let cfg = PcConfig::default();
        assert_eq!(cfg.video, VideoSettings { internal_res: "auto".into(), vsync: true, fullscreen: false });
        assert_eq!(cfg.color, ColorMode::Color);
        assert_eq!(cfg.audio.master_volume, 1.);
        assert!(!cfg.camera.invert_y);
    }
    #[test]
    fn chord_shadows_its_parts_and_modifier_frees_the_camera() {
        let b = Bindings::default();
        let keys = ButtonInput::default();
        let mouse = ButtonInput::default();
        let mut pad = PadView { pressed: vec![GamepadButton::DPadUp], just_pressed: vec![GamepadButton::DPadUp], right: Vec2::new(0.9, 0.), ..default() };
        let s = b.evaluate(&inputs(&keys, &mouse, Some(&pad), None), 0.2, true);
        assert!(s.just_pressed(Pickup) && !s.pressed(RightArmPush));
        assert!(s.camera.x > 0.8);
        pad.pressed.push(GamepadButton::RightTrigger);
        let s = b.evaluate(&inputs(&keys, &mouse, Some(&pad), None), 0.2, true);
        assert!(s.just_pressed(RightArmPush) && !s.pressed(Pickup));
        assert_eq!(s.camera, Vec2::ZERO);
        let s = b.evaluate(&inputs(&keys, &mouse, Some(&pad), Some(StickDir::Cw)), 0.2, true);
        assert!(s.just_pressed(RightArmCircle) && !s.pressed(LeftArmCircle));
        // Select+Start opens settings without also pausing.
        let pad = PadView { pressed: vec![GamepadButton::Select, GamepadButton::Start], just_pressed: vec![GamepadButton::Start], ..default() };
        let s = b.evaluate(&inputs(&keys, &mouse, Some(&pad), None), 0.2, true);
        assert!(s.just_pressed(Settings) && !s.pressed(Pause) && !s.pressed(Plus));
        assert!(!b.evaluate(&inputs(&keys, &mouse, Some(&pad), None), 0.2, false).pressed(Settings));
    }
    #[test]
    fn stick_flicks_fire_on_return_and_rotations_once() {
        let mut g = StickGestures::default();
        assert_eq!(g.update(Vec2::new(0., 0.9), 0.6), None);
        assert_eq!(g.update(Vec2::new(0., 1.), 0.6), None);
        assert_eq!(g.update(Vec2::ZERO, 0.6), Some(StickDir::Up));
        for (v, d) in [(Vec2::X, StickDir::Right), (Vec2::NEG_X, StickDir::Left), (Vec2::NEG_Y, StickDir::Down)] {
            g.update(v, 0.6);
            assert_eq!(g.update(Vec2::ZERO, 0.6), Some(d));
        }
        // A small wobble below the threshold is nothing.
        assert_eq!(g.update(Vec2::new(0.5, 0.), 0.6), None);
        assert_eq!(g.update(Vec2::ZERO, 0.6), None);
        // A clockwise sweep (up -> right -> down -> left) is one circle, no flick after.
        let mut events = Vec::new();
        for i in 0..=40 {
            let a = std::f32::consts::FRAC_PI_2 - i as f32 / 40. * std::f32::consts::TAU;
            events.extend(g.update(Vec2::from_angle(a), 0.6));
        }
        events.extend(g.update(Vec2::ZERO, 0.6));
        assert_eq!(events, vec![StickDir::Cw]);
        for i in 0..=40 {
            events.extend(g.update(Vec2::from_angle(i as f32 / 40. * std::f32::consts::TAU), 0.6));
        }
        assert_eq!(events, vec![StickDir::Cw, StickDir::Ccw]);
    }
    #[test]
    fn deadzone_rescales() {
        assert_eq!(deadzone(Vec2::new(0.1, 0.1), 0.2), Vec2::ZERO);
        assert!((deadzone(Vec2::X, 0.2).x - 1.).abs() < 1e-5);
        assert!((deadzone(Vec2::new(0.6, 0.), 0.2).x - 0.5).abs() < 1e-5);
    }
}
