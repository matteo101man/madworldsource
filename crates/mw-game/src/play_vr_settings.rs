//! VR settings [added]: comfort, movement, hands and HUD, edited in the VR menu (play_vr_menu.rs) and kept in
//! `vr-settings.toml` beside the executable (`MW_VR_SETTINGS` names another file).
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Turn { Snap, Smooth }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MoveDir { Head, Hand }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Run { Push, Click, Never }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Height { Character, Real }

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VrSettings {
    /// Right-stick turning: snap steps of `snap_deg`, or a smooth turn at `smooth_deg` per second.
    pub turn: Turn, pub snap_deg: f32, pub smooth_deg: f32,
    /// Left-stick movement follows the head or the left controller; how the run is reached; the top walk speed share.
    pub move_dir: MoveDir, pub run: Run, pub walk_speed: f32,
    /// Comfort vignette while moving or turning: 0 off, 1 light, 2 strong.
    pub vignette: u8,
    /// The gun hand and laser pointer on the left.
    pub left_handed: bool,
    /// The view at Kreely's eye height (your standing height is calibrated to his) or your own; seated play.
    pub height: Height, pub seated: bool,
    /// Grip toggles a grab instead of holding it.
    pub grab_toggle: bool,
    /// Controller rumble: 0 off, 1 light, 2 full.
    pub haptics: u8,
    /// The HUD panel: distance (m), size (width as a share of the distance) and whether it follows the view lazily.
    pub hud_dist: f32, pub hud_size: f32, pub hud_follow: bool,
    /// Punch strength sensitivity (the hand speed a full-power blow needs is divided by this).
    pub punch_power: f32,
    /// Climbing on walls and ledges by gripping them.
    pub climb: bool,
    /// Event scenes, finishers and pair cameras on a theater screen (off: watched from the body, headset view).
    pub cinema: bool,
    /// The desktop window shows the game camera (off saves a render).
    pub mirror: bool,
    /// Walking into a wall pushes the view back.
    pub wall_push: bool,
    /// Hand fit on the controllers: forward / up (m) and tilt (degrees, + raises the knuckles) of the grip.
    pub hand_fwd: f32, pub hand_up: f32, pub hand_tilt: f32,
    /// The pistol worked by hand (magazines, the slide, dropping it: play_vr_gun.rs) or the simple holster and X reload.
    pub gun_manual: bool,
    /// The pistol's barrel tilted against the controller's pointing ray (degrees, + up).
    pub gun_pitch: f32,
    /// The headset's render size as a share of the size the runtime recommends (takes effect on the next launch).
    pub render_scale: f32,
}

impl Default for VrSettings {
    fn default() -> Self {
        Self { turn: Turn::Snap, snap_deg: 45., smooth_deg: 120., move_dir: MoveDir::Head, run: Run::Push, walk_speed: 0.6, vignette: 1,
               left_handed: false, height: Height::Character, seated: false, grab_toggle: false, haptics: 2, hud_dist: 1.6, hud_size: 1.0,
               hud_follow: true, punch_power: 1.0, climb: true, cinema: false, mirror: true, wall_push: true, hand_fwd: 0., hand_up: 0., hand_tilt: 0., gun_manual: true, gun_pitch: 0., render_scale: 0.8 }
    }
}

pub fn path() -> std::path::PathBuf {
    if let Some(p) = std::env::var_os("MW_VR_SETTINGS") { return p.into(); }
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("vr-settings.toml"))).unwrap_or_else(|| "vr-settings.toml".into())
}

impl VrSettings {
    pub fn load() -> Self {
        std::fs::read_to_string(path()).ok().and_then(|t| toml::from_str(&t).map_err(|e| eprintln!("[vr] settings: {e}")).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        match toml::to_string(self) {
            Ok(t) => { if let Err(e) = std::fs::write(path(), t) { eprintln!("[vr] settings not saved: {e}"); } }
            Err(e) => eprintln!("[vr] settings not saved: {e}"),
        }
    }
    /// The gun / pointer hand index (0 left, 1 right).
    pub fn main_hand(&self) -> usize { if self.left_handed { 0 } else { 1 } }
    pub fn rumble(&self) -> f32 { [0., 0.45, 1.][self.haptics.min(2) as usize] }
}

/// One row of the settings page: its label, the value as shown, and how left / right (or a click) changes it.
pub struct Row { pub label: &'static str, pub value: String, pub help: &'static str }

pub const ROWS: usize = 25;
impl VrSettings {
    pub fn rows(&self) -> Vec<Row> {
        let on = |b: bool| if b { "ON" } else { "OFF" }.to_string();
        vec![
            Row { label: "Turning", value: match self.turn { Turn::Snap => "SNAP", Turn::Smooth => "SMOOTH" }.into(), help: "Right stick: snap steps or a smooth turn" },
            Row { label: "Snap angle", value: format!("{:.0} DEG", self.snap_deg), help: "How far one snap turns you" },
            Row { label: "Smooth turn speed", value: format!("{:.0} DEG/S", self.smooth_deg), help: "Degrees per second at full tilt" },
            Row { label: "Move direction", value: match self.move_dir { MoveDir::Head => "HEAD", MoveDir::Hand => "LEFT HAND" }.into(), help: "Forward on the left stick follows your head or your left controller" },
            Row { label: "Run", value: match self.run { Run::Push => "PUSH STICK", Run::Click => "CLICK STICK", Run::Never => "WALK ONLY" }.into(), help: "Run by pushing the stick all the way, by clicking it, or never" },
            Row { label: "Walk speed", value: format!("{:.0} %", self.walk_speed * 100.), help: "The top walking pace" },
            Row { label: "Comfort vignette", value: ["OFF", "LIGHT", "STRONG"][self.vignette.min(2) as usize].into(), help: "Darkens the edges of the view while you move or turn" },
            Row { label: "Gun hand", value: if self.left_handed { "LEFT" } else { "RIGHT" }.into(), help: "The pistol holster, the laser pointer and the dominant hand" },
            Row { label: "View height", value: match self.height { Height::Character => "KREELY", Height::Real => "YOUR OWN" }.into(), help: "Stand at Kreely's eye height (calibrated from yours) or at your own" },
            Row { label: "Seated play", value: on(self.seated), help: "Calibrate the height for playing seated (recenter after changing)" },
            Row { label: "Grab", value: if self.grab_toggle { "TOGGLE" } else { "HOLD" }.into(), help: "Hold the grip to keep hold, or press it once to grab and again to let go" },
            Row { label: "Haptics", value: ["OFF", "LIGHT", "FULL"][self.haptics.min(2) as usize].into(), help: "Controller rumble on hits, shots and grabs" },
            Row { label: "Punch strength", value: format!("{:.0} %", self.punch_power * 100.), help: "Higher lands full-power blows with slower swings" },
            Row { label: "Climbing", value: on(self.climb), help: "Grip walls and ledges to climb and vault" },
            Row { label: "HUD distance", value: format!("{:.1} M", self.hud_dist), help: "How far the HUD panel floats" },
            Row { label: "HUD size", value: format!("{:.0} %", self.hud_size * 100.), help: "The HUD panel's size" },
            Row { label: "HUD follows view", value: on(self.hud_follow), help: "The HUD drifts after your view (off: it stays where it was set)" },
            Row { label: "Theater for scenes", value: on(self.cinema), help: "Off (first person): scenes play from Kreely's own eyes. On: event scenes and finishers play on a screen" },
            Row { label: "Desktop mirror", value: on(self.mirror), help: "Show the game on the PC monitor too (off: a little more speed in the headset)" },
            Row { label: "Hand forward", value: format!("{:+.0} CM", self.hand_fwd * 100.), help: "Moves Kreely's hands and the pistol along the controllers" },
            Row { label: "Hand height", value: format!("{:+.0} CM", self.hand_up * 100.), help: "Raises or lowers Kreely's hands on the controllers" },
            Row { label: "Hand angle", value: format!("{:+.0} DEG", self.hand_tilt), help: "Tilts the hands and the pistol's barrel to match how you hold the controllers" },
            Row { label: "Pistol handling", value: if self.gun_manual { "REALISTIC" } else { "SIMPLE" }.into(), help: "Realistic: magazines, rack the slide, drop it. Simple: holster and X to reload" },
            Row { label: "Pistol angle", value: format!("{:+.0} DEG", self.gun_pitch), help: "Tilts the pistol's barrel up or down in your hand (shots follow the barrel)" },
            Row { label: "Render scale", value: format!("{:.0} %", self.render_scale * 100.), help: "Lower for a steadier frame rate, higher for a sharper image (next launch)" },
        ]
    }
    /// Left (-1) / right (+1) on row `i`.
    pub fn adjust(&mut self, i: usize, d: i32) {
        let step = |v: f32, by: f32, lo: f32, hi: f32| (v + by * d as f32).clamp(lo, hi);
        let cycle = |v: u8, n: u8| ((v as i32 + d).rem_euclid(n as i32)) as u8;
        match i {
            0 => self.turn = if self.turn == Turn::Snap { Turn::Smooth } else { Turn::Snap },
            1 => { let opts = [15., 30., 45., 60., 90.]; let k = opts.iter().position(|o| (*o - self.snap_deg).abs() < 1.).unwrap_or(2) as i32; self.snap_deg = opts[(k + d).clamp(0, 4) as usize]; }
            2 => self.smooth_deg = step(self.smooth_deg, 30., 30., 360.),
            3 => self.move_dir = if self.move_dir == MoveDir::Head { MoveDir::Hand } else { MoveDir::Head },
            4 => { let o = [Run::Push, Run::Click, Run::Never]; let k = o.iter().position(|r| *r == self.run).unwrap() as i32; self.run = o[(k + d).rem_euclid(3) as usize]; }
            5 => self.walk_speed = step(self.walk_speed, 0.05, 0.3, 0.9),
            6 => self.vignette = cycle(self.vignette, 3),
            7 => self.left_handed = !self.left_handed,
            8 => self.height = if self.height == Height::Character { Height::Real } else { Height::Character },
            9 => self.seated = !self.seated,
            10 => self.grab_toggle = !self.grab_toggle,
            11 => self.haptics = cycle(self.haptics, 3),
            12 => self.punch_power = step(self.punch_power, 0.1, 0.5, 2.),
            13 => self.climb = !self.climb,
            14 => self.hud_dist = step(self.hud_dist, 0.1, 0.8, 3.),
            15 => self.hud_size = step(self.hud_size, 0.05, 0.5, 1.5),
            16 => self.hud_follow = !self.hud_follow,
            17 => self.cinema = !self.cinema,
            18 => self.mirror = !self.mirror,
            19 => self.hand_fwd = step(self.hand_fwd, 0.01, -0.1, 0.1),
            20 => self.hand_up = step(self.hand_up, 0.01, -0.1, 0.1),
            21 => self.hand_tilt = step(self.hand_tilt, 5., -45., 45.),
            22 => self.gun_manual = !self.gun_manual,
            23 => self.gun_pitch = step(self.gun_pitch, 5., -45., 45.),
            24 => self.render_scale = step(self.render_scale, 0.05, 0.5, 1.3),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip() {
        let mut s = VrSettings::default();
        s.turn = Turn::Smooth; s.vignette = 2; s.left_handed = true;
        let t = toml::to_string(&s).unwrap();
        assert_eq!(toml::from_str::<VrSettings>(&t).unwrap(), s);
        assert_eq!(s.rows().len(), ROWS);
        for i in 0..ROWS { let mut c = s.clone(); c.adjust(i, 1); c.adjust(i, -1); }
    }
}
