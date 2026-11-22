//! PC bindings shared by the game window and headless owned-core action tests.
use bevy::prelude::*;
use crate::pc_config::{Action, ActionState, Bindings, Inputs};
use mw_native_platform::motion::{self, Gesture};
pub const NEUTRAL: [u8; 8] = [0, 0, 0, 128, 128, 0, 0, 0];
pub const HELP:&str="PC controls (F1)\n\nWASD: move; double-tap direction to run\nLeft mouse / E: attack; hold to grab\nHold right mouse: chainsaw / vertical cut\nSpace: jump    Shift: dodge\nHold X: shake remote (struggle / finisher)
A/D back and forth: shake when the game asks to shake\nQ / middle mouse: camera; hold to lock on\nMouse / right stick: free camera (MW_PC_FREE_CAMERA=1); F5: orbit while locked on\nF: horizontal strike    R: uppercut\nG: pick up    T: taunt    1 / 2: weapon\nEnter: confirm; hold to skip cutscene
Escape: pause\nBackspace / right mouse: menu back\n\nDirectional actions\nArrows: right-arm left/right/up/down\nI J K L: left-arm up/left/down/right\nN / M: right-arm push / pull\nU / O: left-arm push / pull\nV / B: right-arm circle / reverse circle\nH / Y: left-arm circle / reverse circle";
/// Original event player singleton (guard 80606CE8, instance 80606CF8): word+4 is its
/// state (5 = playing) and byte+1 its skippable flag. While playing, original update
/// 800A4864 asks 800A49F0, which ends the event (fade, state6, 800A47DC) when the game's
/// pad-down has bit 0x20; cPadSystem_updateChannel 8020EFC0 maps Remote B to that bit.
pub const EVENT_PLAYER: u32 = 0x80606CF8;
pub const SKIP_HOLD_SECS: f32 = 1.2;
const REMOTE_A: u16 = 8;
const REMOTE_B: u16 = 4;
pub fn event_skippable(head: u32, state: u32) -> bool {
    state == 5 && (head >> 16) & 0xff != 0
}
/// Hold Enter during a skippable original event; when full, press Remote B once so the
/// game's own skip branch ends it. A short tap still sends Remote A (confirm).
#[derive(Resource, Default)]
pub struct CutsceneSkip {
    pub progress: f32,
    pulse: f32,
    latched: bool,
}
impl CutsceneSkip {
    pub fn update(&mut self, skippable: bool, enter: bool, dt: f32) {
        if !enter {
            self.latched = false;
        }
        self.pulse = if skippable { (self.pulse - dt).max(0.) } else { 0. };
        if skippable && enter && !self.latched {
            self.progress += dt / SKIP_HOLD_SECS;
            if self.progress >= 1. {
                self.progress = 0.;
                self.pulse = 0.5;
                self.latched = true;
            }
        } else {
            self.progress = 0.;
        }
    }
    /// The held Enter that triggered a skip stops reading as Remote A until released,
    /// so it does not carry into gameplay as a held attack. (Default bindings; tests.)
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn apply(&self, packet: &mut [u8; 8], keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) {
        self.apply_held(packet, mouse.pressed(MouseButton::Left) || keys.pressed(KeyCode::KeyE));
    }
    /// `attack_held`: the Attack action (not Confirm) is held, so A stays pressed.
    pub fn apply_held(&self, packet: &mut [u8; 8], attack_held: bool) {
        let mut buttons = u16::from_be_bytes([packet[0], packet[1]]);
        if self.latched && !attack_held {
            buttons &= !REMOTE_A;
        }
        if self.pulse > 0. {
            buttons |= REMOTE_B;
        }
        packet[..2].copy_from_slice(&buttons.to_be_bytes());
    }
}
#[derive(Resource, Default)]
pub struct PcControls {
    remote: u8,
    nunchuk: u8,
    chainsaw_motion: bool,
    #[cfg_attr(not(test), allow(dead_code))]
    default_bindings: Option<Bindings>,
}
/// Remote (right arm) and Nunchuk (left arm) gesture actions, in the original edge order.
const REMOTE_GESTURES: [(Action, Gesture); 9] = [
    (Action::Shake, Gesture::Shake),
    (Action::RightArmRight, Gesture::Right),
    (Action::RightArmLeft, Gesture::Left),
    (Action::RightArmUp, Gesture::Up),
    (Action::RightArmDown, Gesture::Down),
    (Action::RightArmPush, Gesture::Forward),
    (Action::RightArmPull, Gesture::Back),
    (Action::RightArmCircle, Gesture::Clockwise),
    (Action::RightArmReverseCircle, Gesture::CounterClockwise),
];
const NUNCHUK_GESTURES: [(Action, Gesture); 10] = [
    (Action::Shake, Gesture::Shake),
    (Action::LeftArmRight, Gesture::Right),
    (Action::LeftArmLeft, Gesture::Left),
    (Action::LeftArmUp, Gesture::Up),
    (Action::LeftArmDown, Gesture::Down),
    (Action::LeftArmPush, Gesture::Forward),
    (Action::LeftArmPull, Gesture::Back),
    (Action::Dodge, Gesture::Back),
    (Action::LeftArmCircle, Gesture::Clockwise),
    (Action::LeftArmReverseCircle, Gesture::CounterClockwise),
];
/// Wire (BE) button bits per action.
const BUTTONS: [(Action, u16); 10] = [
    (Action::Attack, REMOTE_A),
    (Action::Confirm, REMOTE_A),
    (Action::Chainsaw, REMOTE_B),
    (Action::Back, REMOTE_B),
    (Action::Pause, 0x10),
    (Action::Plus, 0x1000),
    (Action::Pickup, 0x800),
    (Action::Taunt, 0x400),
    (Action::Weapon1, 0x100),
    (Action::Weapon2, 0x200),
];
/// Analog stick scale: the pad HLE divides by 96 and clamps, so full speed starts at ~87%.
const STICK_SCALE: f32 = 110.;
impl PcControls {
    /// Default keyboard/mouse bindings only (headless tests and diagnostics).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn packet(
        &mut self,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
        focused: bool,
    ) -> [u8; 8] {
        let bindings = self.default_bindings.get_or_insert_with(Bindings::default);
        let state = bindings.evaluate(&Inputs { keys, mouse, pad: None, stick: None }, 0.2, focused);
        self.packet_from(&state)
    }
    pub fn packet_from(&mut self, a: &ActionState) -> [u8; 8] {
        if !a.focused {
            self.remote = 0;
            self.nunchuk = 0;
            self.chainsaw_motion = false;
            return NEUTRAL;
        }
        let mut p = NEUTRAL;
        if !a.pressed(Action::Shake) {
            if self.remote & 15 == Gesture::Shake as u8 { self.remote=0; }
            if self.nunchuk & 15 == Gesture::Shake as u8 { self.nunchuk=0; }
        }
        let buttons = BUTTONS.iter().filter(|(action, _)| a.pressed(*action)).fold(0u16, |b, (_, bit)| b | bit);
        p[..2].copy_from_slice(&buttons.to_be_bytes());
        p[2] = u8::from(a.pressed(Action::Jump)) | u8::from(a.pressed(Action::LockOn)) * 2;
        let x = i16::from(a.pressed(Action::MoveRight)) - i16::from(a.pressed(Action::MoveLeft));
        let y = i16::from(a.pressed(Action::MoveUp)) - i16::from(a.pressed(Action::MoveDown));
        if x != 0 || y != 0 {
            let scale = if x != 0 && y != 0 { 89 } else { 127 };
            p[3] = (128 + x * scale) as u8;
            p[4] = (128 + y * scale) as u8;
        } else if a.stick != Vec2::ZERO {
            let axis = |v: f32| (128. + v * STICK_SCALE).round().clamp(1., 255.) as u8;
            p[3] = axis(a.stick.x);
            p[4] = axis(a.stick.y);
        }
        if self.chainsaw_motion && !a.pressed(Action::Chainsaw) {
            self.remote = 0;
            self.chainsaw_motion = false;
        }
        if a.just_pressed(Action::Chainsaw) {
            self.remote = motion::token(self.remote, Gesture::Up);
            self.chainsaw_motion = true;
        }
        for (action, gesture) in REMOTE_GESTURES {
            if a.just_pressed(action) {
                self.remote = motion::token(self.remote, gesture);
                self.chainsaw_motion = false;
            }
        }
        for (action, gesture) in NUNCHUK_GESTURES {
            if a.just_pressed(action) {
                self.nunchuk = motion::token(self.nunchuk, gesture);
            }
        }
        p[6] = self.remote;
        p[7] = self.nunchuk;
        p
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pc_config::{PadView, StickDir};
    use bevy::input::gamepad::GamepadButton;
    #[test]
    fn pc_buttons_match_original_action_controls() {
        let mut controls = PcControls::default();
        let mut keys = ButtonInput::default();
        let mut mouse = ButtonInput::default();
        keys.press(KeyCode::Space);
        keys.press(KeyCode::KeyQ);
        keys.press(KeyCode::Escape);
        mouse.press(MouseButton::Left);
        let p = controls.packet(&keys, &mouse, true);
        assert_eq!(u16::from_be_bytes([p[0], p[1]]), 0x18);
        assert_eq!(p[2], 3);
        assert_eq!(p[5], 0);
        assert_eq!(controls.packet(&keys, &mouse, false), NEUTRAL);
    }
    #[test]
    fn independent_gesture_edges_survive_key_release_and_cancel_on_blur() {
        let mut c = PcControls::default();
        let mut keys = ButtonInput::default();
        let mouse = ButtonInput::default();
        keys.press(KeyCode::KeyR);
        keys.press(KeyCode::ShiftLeft);
        let first = c.packet(&keys, &mouse, true);
        assert_eq!(first[6] & 15, Gesture::Up as u8);
        assert_eq!(first[7] & 15, Gesture::Back as u8);
        keys.release(KeyCode::KeyR);
        keys.release(KeyCode::ShiftLeft);
        keys.clear();
        assert_eq!(c.packet(&keys, &mouse, true)[6..], first[6..]);
        keys.press(KeyCode::KeyR);
        let next = c.packet(&keys, &mouse, true);
        assert_ne!(next[6], first[6]);
        assert_eq!(c.packet(&keys, &mouse, false), NEUTRAL);
        keys.release_all();
        keys.clear();
        assert_eq!(c.packet(&keys, &mouse, true), NEUTRAL);
    }
    #[test]
    fn struggle_sends_both_arms_without_pressing_action_buttons() {
        let mut c = PcControls::default();
        let mut keys = ButtonInput::default();
        let mouse = ButtonInput::default();
        keys.press(KeyCode::KeyX);
        let first = c.packet(&keys, &mouse, true);
        assert_eq!(&first[..6], &NEUTRAL[..6]);
        assert_eq!(first[6] & 15, Gesture::Shake as u8);
        assert_eq!(first[7] & 15, Gesture::Shake as u8);
        keys.clear();
        assert_eq!(c.packet(&keys, &mouse, true), first);
        keys.release(KeyCode::KeyX);
        keys.clear();
        assert_eq!(c.packet(&keys, &mouse, true), NEUTRAL);
        keys.press(KeyCode::KeyX);
        let next = c.packet(&keys, &mouse, true);
        assert_eq!(next[6] & 15, Gesture::Shake as u8);
        assert_eq!(next[7] & 15, Gesture::Shake as u8);
        assert_eq!(c.packet(&keys, &mouse, false), NEUTRAL);
    }
    #[test]
    fn holding_enter_in_skippable_event_presses_b_once_and_taps_still_confirm() {
        let mut c = PcControls::default();
        let mut skip = CutsceneSkip::default();
        let mut keys = ButtonInput::default();
        let mouse = ButtonInput::default();
        keys.press(KeyCode::Enter);
        skip.update(true, true, 0.2);
        let mut p = c.packet(&keys, &mouse, true);
        skip.apply(&mut p, &keys, &mouse);
        assert_eq!(u16::from_be_bytes([p[0], p[1]]), REMOTE_A);
        skip.update(false, false, 0.1);
        assert_eq!(skip.progress, 0.);
        for _ in 0..13 {
            skip.update(true, true, 0.1);
        }
        let mut p = c.packet(&keys, &mouse, true);
        skip.apply(&mut p, &keys, &mouse);
        assert_eq!(u16::from_be_bytes([p[0], p[1]]), REMOTE_B);
        skip.update(false, true, 0.016);
        let mut p = c.packet(&keys, &mouse, true);
        skip.apply(&mut p, &keys, &mouse);
        assert_eq!(u16::from_be_bytes([p[0], p[1]]), 0);
        skip.update(true, true, 5.);
        assert_eq!(skip.progress, 0.);
        assert!(event_skippable(0x0101_0000, 5) && !event_skippable(0x0100_0000, 5) && !event_skippable(0x0101_0000, 6));
    }
    #[test]
    fn movement_cancels_opposites_and_normalizes_diagonal() {
        let mut c = PcControls::default();
        let mut keys = ButtonInput::default();
        let mouse = ButtonInput::default();
        keys.press(KeyCode::KeyW);
        keys.press(KeyCode::KeyD);
        let p = c.packet(&keys, &mouse, true);
        assert_eq!(&p[3..5], &[217, 217]);
        keys.press(KeyCode::KeyA);
        keys.press(KeyCode::KeyS);
        assert_eq!(c.packet(&keys, &mouse, true), NEUTRAL);
    }
    #[test]
    fn releasing_chainsaw_cancels_default_cut_instead_of_punching() {
        let mut controls = PcControls::default();
        let keys = ButtonInput::default();
        let mut mouse = ButtonInput::default();
        mouse.press(MouseButton::Right);
        let down = controls.packet(&keys, &mouse, true);
        assert_eq!(down[1], 4);
        assert_eq!(down[6] & 15, Gesture::Up as u8);
        mouse.release(MouseButton::Right);
        mouse.clear();
        assert_eq!(controls.packet(&keys, &mouse, true), NEUTRAL);
    }
    fn pad_state(pressed: &[GamepadButton], left: Vec2, stick: Option<StickDir>) -> ActionState {
        let keys = ButtonInput::default();
        let mouse = ButtonInput::default();
        let pad = PadView { pressed: pressed.to_vec(), just_pressed: pressed.to_vec(), left, right: Vec2::ZERO };
        Bindings::default().evaluate(&Inputs { keys: &keys, mouse: &mouse, pad: Some(&pad), stick }, 0.2, true)
    }
    #[test]
    fn gamepad_defaults_reach_the_same_wii_inputs() {
        use GamepadButton::*;
        let mut c = PcControls::default();
        let p = c.packet_from(&pad_state(&[South, West, LeftTrigger2, DPadUp], Vec2::ZERO, None));
        assert_eq!(u16::from_be_bytes([p[0], p[1]]), REMOTE_A | 0x800);
        assert_eq!(p[2], 3);
        // Right trigger is the chainsaw: B plus the default vertical gesture, cancelled on release.
        let p = c.packet_from(&pad_state(&[RightTrigger2], Vec2::ZERO, None));
        assert_eq!(u16::from_be_bytes([p[0], p[1]]), REMOTE_B);
        assert_eq!(p[6] & 15, Gesture::Up as u8);
        assert_eq!(c.packet_from(&pad_state(&[], Vec2::ZERO, None)), NEUTRAL);
        // Y dodges (Nunchuk back); L3 shakes both arms while held.
        assert_eq!(c.packet_from(&pad_state(&[North], Vec2::ZERO, None))[7] & 15, Gesture::Back as u8);
        let p = c.packet_from(&pad_state(&[LeftThumb], Vec2::ZERO, None));
        assert_eq!((p[6] & 15, p[7] & 15), (Gesture::Shake as u8, Gesture::Shake as u8));
        // RB + right-stick flick is a right-arm strike; LB + rotation a left-arm circle.
        let p = c.packet_from(&pad_state(&[RightTrigger], Vec2::ZERO, Some(StickDir::Right)));
        assert_eq!(p[6] & 15, Gesture::Right as u8);
        let p = c.packet_from(&pad_state(&[LeftTrigger], Vec2::ZERO, Some(StickDir::Ccw)));
        assert_eq!(p[7] & 15, Gesture::CounterClockwise as u8);
        // A flick without a modifier is only camera, never a gesture.
        let before = c.packet_from(&pad_state(&[], Vec2::ZERO, None));
        assert_eq!(c.packet_from(&pad_state(&[], Vec2::ZERO, Some(StickDir::Up))), before);
    }
    #[test]
    fn left_stick_is_analog_and_full_tilt_wiggles_past_the_shake_threshold() {
        let mut c = PcControls::default();
        let p = c.packet_from(&pad_state(&[], Vec2::new(0.6, 0.), None));
        assert!(p[3] > 160 && p[3] < 192 && p[4] == 128, "{p:?}");
        let p = c.packet_from(&pad_state(&[], Vec2::new(-1., 1.), None));
        assert!(p[3] < 64 && p[4] > 192);
        let p = c.packet_from(&pad_state(&[], Vec2::new(1., 0.), None));
        assert!(p[3] > 192);
    }
}
