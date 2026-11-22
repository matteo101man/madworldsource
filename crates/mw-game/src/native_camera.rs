//! Optional PC free camera (MW_PC_FREE_CAMERA=1, default off). Mouse and right-stick look
//! from [`pc_config::CameraLook`] become orbit radians for the core hook in the game's own
//! follow camera (runtime/game/pc_camera.cpp; research/PC-CAMERA.md). Movement stays
//! relative to the shown camera because the game derives it from that camera.
//! Lock-on, Q recenter, events and scripted shots keep the original camera (decided in the core).
//! While the game is locked on, mouse wheel or a right-stick flick cycles the target (see
//! [`TargetCycle`]); the core switches through the original SetLockOn_Trg.
use crate::{pc_config, pc_menu};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use mw_native_platform::host;
/// Radians per mouse pixel at sensitivity 1.
const MOUSE_RAD_PER_PIXEL: f32 = 0.0025;
/// Radians per second at full right-stick deflection and sensitivity 1.
const STICK_RAD_PER_SEC: f32 = 2.6;
/// Game yaw grows from +Z toward +X; on screen (right-handed, Y up) that turns the view left.
const YAW_RIGHT: f32 = -1.;
/// Sign of the core's pitch offset that raises the view (eye lower, looking up).
const PITCH_UP: f32 = 1.;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseLook {
    /// Cursor locked and hidden while the window is focused; the mouse always looks.
    Capture,
    /// The mouse looks only while Left Alt is held; the cursor stays free.
    Hold,
    /// Right stick only.
    Off,
}
#[derive(Resource, Clone, Debug)]
pub struct FreeCamera {
    pub enabled: bool,
    pub mouse: MouseLook,
    /// Env overrides of the saved settings (MW_PC_FREE_CAMERA_SENS / _INVERT_Y); never saved.
    pub sensitivity: Option<f32>,
    pub invert_y: Option<bool>,
    /// MW_PC_CAMERA_AUTOFOLLOW=0|1 overrides the saved auto-follow setting.
    pub auto_follow: Option<bool>,
    /// MW_PC_CAMERA_LOCK_ORBIT=0|1 overrides the saved "orbit while locked on" setting
    /// (`camera.lock_orbit`, default on). F5 flips whichever of the two is in effect.
    pub lock_orbit: Option<bool>,
}
impl Default for FreeCamera {
    fn default() -> Self {
        Self::from_vars(|k| std::env::var(k).ok())
    }
}
impl FreeCamera {
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Self {
        let flag = |v: String| matches!(v.trim(), "1" | "true" | "on" | "yes");
        Self {
            enabled: get("MW_PC_FREE_CAMERA").is_some_and(flag),
            mouse: match get("MW_PC_FREE_CAMERA_MOUSE").as_deref().map(str::trim) {
                Some("hold") => MouseLook::Hold,
                Some("off") => MouseLook::Off,
                _ => MouseLook::Capture,
            },
            sensitivity: get("MW_PC_FREE_CAMERA_SENS").and_then(|v| v.trim().parse().ok()).filter(|s: &f32| s.is_finite() && *s > 0.),
            invert_y: get("MW_PC_FREE_CAMERA_INVERT_Y").map(flag),
            auto_follow: get("MW_PC_CAMERA_AUTOFOLLOW").map(flag),
            lock_orbit: get("MW_PC_CAMERA_LOCK_ORBIT").map(flag),
        }
    }
    /// (yaw, pitch) orbit radians for one host frame.
    pub fn orbit(&self, look: &pc_config::CameraLook, cfg: &pc_config::CameraSettings, look_held: bool, dt: f32) -> (f32, f32) {
        if !self.enabled {
            return (0., 0.);
        }
        let scale = self.sensitivity.unwrap_or(1.);
        let mouse_on = match self.mouse {
            MouseLook::Capture => true,
            MouseLook::Hold => look_held,
            MouseLook::Off => false,
        };
        let mouse = if mouse_on { look.mouse * MOUSE_RAD_PER_PIXEL * cfg.mouse_sensitivity * scale } else { Vec2::ZERO };
        let stick = look.stick.clamp_length_max(1.) * STICK_RAD_PER_SEC * cfg.stick_sensitivity * scale * dt.clamp(0., 0.1);
        // Bevy mouse +y is down; the stick is +y up.
        let right = mouse.x + stick.x;
        let up = -mouse.y + stick.y;
        let invert = self.invert_y.unwrap_or(cfg.invert_y);
        (YAW_RIGHT * right, PITCH_UP * if invert { -up } else { up })
    }
}
/// Lock-on target cycle events: +1 = next target to the right on screen, -1 = left.
/// Mouse wheel up/down, or a right-stick flick (past FLICK_ON from rest, mostly horizontal).
#[derive(Default, Debug)]
pub struct TargetCycle {
    flick_used: bool,
}
const FLICK_ON: f32 = 0.85;
const FLICK_REST: f32 = 0.3;
impl TargetCycle {
    pub fn update(&mut self, stick: Vec2, wheel: f32) -> i8 {
        let mut dir = 0;
        if stick.length() < FLICK_REST {
            self.flick_used = false;
        } else if !self.flick_used && stick.x.abs() > FLICK_ON && stick.y.abs() < 0.6 {
            self.flick_used = true;
            dir = if stick.x > 0. { 1 } else { -1 };
        }
        if wheel > 0. {
            dir = 1;
        } else if wheel < 0. {
            dir = -1;
        }
        dir
    }
}
/// Key that toggles orbiting while locked on.
pub const LOCK_ORBIT_TOGGLE: KeyCode = KeyCode::F5;
pub fn free_camera(
    mut cam: ResMut<FreeCamera>,
    look: Res<pc_config::CameraLook>,
    mut cfg: ResMut<pc_config::PcConfig>,
    menu: Res<pc_menu::SettingsMenu>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    mut cycle: Local<TargetCycle>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
) {
    if !cam.enabled {
        return;
    }
    if keys.just_pressed(LOCK_ORBIT_TOGGLE) && !menu.open {
        // An env override stays a session-only override; otherwise F5 changes the saved setting
        // and writes pc-controls.toml right away, like the C/F3 color hotkeys.
        match cam.lock_orbit.as_mut() {
            Some(on) => *on ^= true,
            None => {
                cfg.camera.lock_orbit ^= true;
                cfg.save();
            }
        }
        let on = cam.lock_orbit.unwrap_or(cfg.camera.lock_orbit);
        info!("[pc-camera] orbit while locked on: {}", if on { "on" } else { "off (original lock camera)" });
    }
    host::set_camera_lock_orbit(cam.lock_orbit.unwrap_or(cfg.camera.lock_orbit));
    host::set_camera_auto_follow(cam.auto_follow.unwrap_or(cfg.camera.auto_follow));
    let (yaw, pitch) = cam.orbit(&look, &cfg.camera, keys.pressed(KeyCode::AltLeft), time.delta_secs());
    host::add_camera_orbit(yaw, pitch);
    let focused = windows.single().is_ok_and(|(w, _)| w.focused);
    let wheel = if focused && !menu.open { scroll.delta.y } else { 0. };
    host::add_camera_cycle(cycle.update(look.stick, wheel));
    if let Ok((window, mut cursor)) = windows.single_mut() {
        let grab = cam.mouse == MouseLook::Capture && window.focused && !menu.open;
        let (mode, visible) = if grab { (CursorGrabMode::Locked, false) } else { (CursorGrabMode::None, true) };
        if cursor.grab_mode != mode || cursor.visible != visible {
            cursor.grab_mode = mode;
            cursor.visible = visible;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cam(vars: &[(&str, &str)]) -> FreeCamera {
        let vars: Vec<(String, String)> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        FreeCamera::from_vars(|k| vars.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()))
    }
    fn look(mouse: Vec2, stick: Vec2) -> pc_config::CameraLook {
        pc_config::CameraLook { stick, mouse, recenter: false }
    }
    #[test]
    fn free_camera_is_default_off_and_env_parses() {
        let off = cam(&[]);
        assert!(!off.enabled);
        assert_eq!(off.orbit(&look(Vec2::new(100., 50.), Vec2::X), &default(), true, 0.016), (0., 0.));
        let on = cam(&[("MW_PC_FREE_CAMERA", "1"), ("MW_PC_FREE_CAMERA_MOUSE", "hold"), ("MW_PC_FREE_CAMERA_SENS", "2"), ("MW_PC_FREE_CAMERA_INVERT_Y", "1")]);
        assert!(on.enabled && on.mouse == MouseLook::Hold && on.sensitivity == Some(2.) && on.invert_y == Some(true));
        assert_eq!(cam(&[("MW_PC_FREE_CAMERA", "1"), ("MW_PC_FREE_CAMERA_SENS", "-3")]).sensitivity, None);
        assert_eq!(cam(&[("MW_PC_FREE_CAMERA", "1")]).mouse, MouseLook::Capture);
        assert_eq!(cam(&[("MW_PC_CAMERA_AUTOFOLLOW", "0")]).auto_follow, Some(false));
        assert_eq!(cam(&[]).auto_follow, None);
        assert_eq!(cam(&[]).lock_orbit, None);
        assert_eq!(cam(&[("MW_PC_CAMERA_LOCK_ORBIT", "0")]).lock_orbit, Some(false));
        assert!(pc_config::CameraSettings::default().lock_orbit);
    }
    #[test]
    fn mouse_and_stick_orbit_with_sensitivity_and_invert() {
        let c = cam(&[("MW_PC_FREE_CAMERA", "1")]);
        let s = pc_config::CameraSettings::default();
        // Mouse right turns the view right (game yaw decreases); mouse up looks up.
        let (yaw, pitch) = c.orbit(&look(Vec2::new(100., -40.), Vec2::ZERO), &s, false, 0.016);
        assert!((yaw - YAW_RIGHT * 0.25).abs() < 1e-6 && (pitch - PITCH_UP * 0.1).abs() < 1e-6, "{yaw} {pitch}");
        // Full right stick for one second at sensitivity 1.
        let (yaw, pitch) = c.orbit(&look(Vec2::ZERO, Vec2::new(1., 0.)), &s, false, 0.1);
        assert!((yaw - YAW_RIGHT * STICK_RAD_PER_SEC * 0.1).abs() < 1e-6 && pitch == 0.);
        // Saved settings scale and invert; env overrides them.
        let saved = pc_config::CameraSettings { mouse_sensitivity: 2., invert_y: true, ..default() };
        let (yaw, pitch) = c.orbit(&look(Vec2::new(10., -10.), Vec2::ZERO), &saved, false, 0.016);
        assert!((yaw - YAW_RIGHT * 0.05).abs() < 1e-6 && (pitch + PITCH_UP * 0.05).abs() < 1e-6);
        let env = cam(&[("MW_PC_FREE_CAMERA", "1"), ("MW_PC_FREE_CAMERA_INVERT_Y", "0")]);
        assert!(env.orbit(&look(Vec2::new(0., -10.), Vec2::ZERO), &saved, false, 0.016).1 * PITCH_UP > 0.);
        // A long frame cannot turn the stick into a jump.
        assert_eq!(c.orbit(&look(Vec2::ZERO, Vec2::X), &s, false, 5.), c.orbit(&look(Vec2::ZERO, Vec2::X), &s, false, 0.1));
    }
    #[test]
    fn hold_and_off_modes_gate_only_the_mouse() {
        let s = pc_config::CameraSettings::default();
        let hold = cam(&[("MW_PC_FREE_CAMERA", "1"), ("MW_PC_FREE_CAMERA_MOUSE", "hold")]);
        let m = look(Vec2::new(50., 0.), Vec2::ZERO);
        assert_eq!(hold.orbit(&m, &s, false, 0.016), (0., 0.));
        assert!(hold.orbit(&m, &s, true, 0.016).0 != 0.);
        let off = cam(&[("MW_PC_FREE_CAMERA", "1"), ("MW_PC_FREE_CAMERA_MOUSE", "off")]);
        assert_eq!(off.orbit(&m, &s, true, 0.016), (0., 0.));
        assert!(off.orbit(&look(Vec2::ZERO, Vec2::new(0., 1.)), &s, false, 0.05).1 != 0.);
    }
    #[test]
    fn wheel_and_stick_flick_cycle_targets_once_per_flick() {
        let mut c = TargetCycle::default();
        assert_eq!(c.update(Vec2::ZERO, 1.), 1);
        assert_eq!(c.update(Vec2::ZERO, -2.), -1);
        assert_eq!(c.update(Vec2::new(0.5, 0.), 0.), 0); // partial tilt is not a flick
        assert_eq!(c.update(Vec2::new(1., 0.1), 0.), 1);
        assert_eq!(c.update(Vec2::new(1., 0.), 0.), 0); // held: one event only
        assert_eq!(c.update(Vec2::new(-1., 0.), 0.), 0); // no rest in between
        assert_eq!(c.update(Vec2::ZERO, 0.), 0);
        assert_eq!(c.update(Vec2::new(-0.9, 0.), 0.), -1);
        assert_eq!(c.update(Vec2::ZERO, 0.), 0);
        assert_eq!(c.update(Vec2::new(0.9, 0.9), 0.), 0); // diagonal: pitch/yaw look, not a flick
    }
}
