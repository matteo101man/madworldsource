//! cPl0000 (Jack) locomotion, translated from the DOL (research/PL0000-LOCOMOTION.md).
//!
//! Per game frame `fn_8027621C` counts the dash window down, then runs A[state] (checks / transitions) and
//! B[state] (motion + turning) from the member-pointer table at 0x805CBFCC. Translated here:
//! state 0 idle (A 8027AD20, B 8027AEC4 -> 8027AFE4), 1 turn relay (A 8027C504, B stub), 2 walk (A 8027C600,
//! B 8027C744), 4 run / dash (A 8027D164, B 8027D2D8), with the shared checks 802C2434 / 802C2528 (walk / run
//! choice), 802C2B0C (double-tap dash), 802C7144 + 802C71DC (turn toward the stick, RadLerpRange 800BB634).
//! Only the default-weapon branches are translated (weapon types 3/6/8/9/10-12 pick other motions).
//! Combat entries (802C2610 shake, 802C2E68 jump, 802C3B5C attacks, ...) are not translated yet: the caller asks
//! for them through `Player::enter_combat`, labelled [observed] in mw-play.

/// Stick thresholds are squared magnitudes (`+0x1144`).
pub const STICK_MOVE: f32 = 0.04;       // 8056A098 (0.2^2): idle -> move
pub const STICK_KEEP: f32 = 0.0256;     // 8056A09C (0.16^2): walk/run -> idle
pub const STICK_RUN: f32 = 0.9025;      // 8056A0A0 (0.95^2): walk <-> run
pub const STICK_FLICK: f32 = 0.64;      // 8056A46C (0.8^2): dash double-tap
pub const TURN_MIN: f32 = 0.523_598_8;  // 8056A150 (30 deg)
pub const TURN_MAX: f32 = 2.617_993_8;  // 8056A464 (150 deg)
pub const TURN_FRAC: f32 = 0.3;         // 8056A02C, scaled by +0x1150
pub const TURN_CLAMP: f32 = std::f32::consts::FRAC_PI_2; // 80569FB0
/// `+0x1150` = GameVSyncF = 2.0 at the shipped frame interval 2 (fn_8026F3A8(2), SIXTY-FPS.md).
pub const VSYNC: f32 = 2.0;
/// `+0x11A8` = 1 at interval 2: 60 Hz-unit timers are shifted right by it.
pub const INTERVAL_SHIFT: u32 = 1;
/// `+0x1148` default blend = 80569F6C (5.0) at interval 2 (main 80270D3C); 4.0 = 8056A00C.
pub const BLEND_DEFAULT: f32 = 5.0;
pub const BLEND_SHORT: f32 = 4.0;
pub const RUN_RATE: f32 = 1.025;        // 8056A0A8
pub const DASH_SPEED: f32 = 1.1;        // 80569F94
pub const DASH_STEP: f32 = 0.03;        // 80569FA4
pub const DASH_MAX: f32 = 1.3;          // 8056A0AC
pub const DASH_RATE_K: f32 = 1.4;       // 8056A08C
pub const SPRINT_LAYER_K: f32 = 3.33;   // 8056A0B0
pub const DASH_WINDOW: i16 = 13;        // 802C2B0C
pub const RUN_TIMER: i32 = 0x96 >> INTERVAL_SHIFT;

pub const IDLE: &str = "pl0030_0000.mot";
pub const WALK: &str = "pl0030_00c2.mot";
pub const RUN: &str = "pl0030_00ce.mot";
pub const SPRINT_LAYER: &str = "pl0030_00cf.mot";
pub const FIDGET_A: &str = "pl0030_0008.mot";
/// Chainsaw slot (weapon slot `+0x1500` = 1, type 6): idle 0425 (8027B6F0), run 0427 (8027D2D8), stow 0424
/// (8027AFE4 phase 10).
pub const SAW_IDLE: &str = "pl0030_0425.mot";
pub const SAW_RUN: &str = "pl0030_0427.mot";
pub const SAW_STOW: &str = "pl0030_0424.mot";
pub const FIDGET_B: &str = "pl0030_0007.mot";
/// Idle fidget announcer line (jkSound::PlayVoiceStream 8026B558(0x1117)).
pub const FIDGET_VOICE: u16 = 0x1117;

/// One game frame of pad input.
#[derive(Clone, Copy, Default, Debug)]
pub struct Pad {
    /// Stick in pad space (x right, y up), |stick| <= 1 (`+0xF1C/+0xF20`).
    pub stick: [f32; 2],
    /// Camera yaw (`+0x1134`, written by the movement basis 802C6EEC).
    pub camera_yaw: f32,
}
impl Pad {
    fn mag2(&self) -> f32 { self.stick[0] * self.stick[0] + self.stick[1] * self.stick[1] }
    /// World yaw of the stick (`+0x113C`); yaw 0 faces +Z, matching mw-play's heading.
    fn yaw(&self) -> f32 { self.camera_yaw + (-self.stick[0]).atan2(self.stick[1]) }
}

/// A motion change requested this frame (fn_80271424 / fn_802716EC: same motion keeps playing).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MotionCmd { pub name: &'static str, pub blend: f32, pub restart: bool }

#[derive(Clone, Debug)]
pub struct Player {
    pub mode: u8, pub state: u8, pub phase: u8, pub selector: u8,
    pub prev_state: u8,
    /// `+0x74` facing and `+0x3B8` turn target.
    pub facing: f32, target: f32,
    /// `+0x11B8` motion playback rate, `+0x1164` run speed, `+0x1230` timer.
    pub rate: f32, pub speed: f32, timer: i32,
    flags: u32, window: i16, snap: [f32; 2],
    /// Sprint layer (`+0x14E8`, weight `+0x1240`), drawn over the run.
    pub layer: Option<(&'static str, f32)>,
    pub motion: &'static str,
    /// Set by the caller while a combat state from outside this module owns the player.
    pub combat: bool,
    /// Idle fidget slot `+0x1234` (0..3) and loop countdown `+0x1238`.
    fidget: u8, fidget_count: i32,
    /// Announcer voice requested this frame (drained by the caller).
    pub voice: Option<u16>,
    /// Weapon slot 1 (chainsaw revved) this frame and last frame (`+0x1500` / `+0x1502`).
    saw: bool, saw_prev: bool,
}

const F_RELEASED: u32 = 2; const F_PRESSED: u32 = 4; const F_STAND: u32 = 0x8000;

fn rad_adjust(mut a: f32) -> f32 {
    while a < -std::f32::consts::PI { a += std::f32::consts::TAU; }
    while a > std::f32::consts::PI { a -= std::f32::consts::TAU; }
    a
}
/// RadLerpRange (800BB634): cur + clamp(frac * wrap(target - cur), +-max), wrapped.
pub fn rad_lerp_range(cur: f32, target: f32, frac: f32, max: f32) -> f32 {
    rad_adjust(cur + (frac * rad_adjust(target - cur)).clamp(-max, max))
}

impl Player {
    pub fn new(facing: f32) -> Self {
        Player { mode: 1, state: 0, phase: 0, selector: 0, prev_state: 0, facing, target: facing, rate: 1., speed: RUN_RATE,
                 timer: 0, flags: 0, window: 0, snap: [0.; 2], layer: None, motion: IDLE, combat: false,
                 fidget: 0, fidget_count: 10, voice: None, saw: false, saw_prev: false }
    }

    /// changeState (802CBF50).
    fn change(&mut self, state: u8, phase: u8) { self.prev_state = self.state; self.state = state; self.phase = phase; self.selector = 0; }

    fn play(&mut self, name: &'static str, blend: f32, out: &mut Option<MotionCmd>) {
        let restart = self.motion != name;
        self.motion = name; *out = Some(MotionCmd { name, blend, restart });
    }

    /// One native game frame. `motion_end`: the current motion reached its end this frame.
    pub fn step(&mut self, pad: &Pad, motion_end: bool) -> Option<MotionCmd> { self.step_rng(pad, motion_end, &mut |a, _| a) }

    /// 802C3338 weapon switch, simplified to "slot 1 while B is held" [the toggle / buffer conditions are not
    /// translated]. Leaving slot 1 in idle, walk or run sets phase 10 (idle: the 0424 stow); entering it restarts
    /// the locomotion motion with the type-6 variant.
    pub fn set_saw(&mut self, on: bool) {
        self.saw_prev = self.saw; self.saw = on;
        if self.combat || self.saw == self.saw_prev { return; }
        if matches!(self.state, 0 | 2 | 4) { self.phase = if !on && self.state == 0 { 10 } else { 0 }; }
    }
    pub fn saw(&self) -> bool { self.saw }

    /// `step` with the shared cRand (`getS16(lo, hi)`, 800BB79C) for the idle fidget countdown.
    pub fn step_rng(&mut self, pad: &Pad, motion_end: bool, rand: &mut dyn FnMut(i16, i16) -> i16) -> Option<MotionCmd> {
        // fn_8027621C: dash window countdown
        if self.window != 0 { self.window -= 1; } else { self.flags &= !(F_RELEASED | F_PRESSED); }
        if self.combat { return None; }
        let mut out = None;
        match self.state { 0 => self.a_idle(pad, &mut out), 1 => self.a_turn(pad, &mut out), 2 => self.a_walk(pad, &mut out), 4 => self.a_run(pad, &mut out), _ => {} }
        match self.state {
            0 => self.b_idle(pad, motion_end, rand, &mut out),
            2 => self.b_walk(pad, motion_end, &mut out),
            4 => self.b_run(pad, motion_end, &mut out),
            _ => {}
        }
        out
    }

    /// Leave locomotion for a combat state handled elsewhere (until 802C3B5C etc. are translated).
    pub fn enter_combat(&mut self, state: u8) { self.change(state, 0); self.combat = true; self.layer = None; self.motion = ""; }
    /// Combat finished: back to idle (the observed 39 -> 0 path), keeping the facing the combat state left.
    pub fn leave_combat(&mut self, facing: f32) { self.combat = false; self.change(0, 0); self.facing = facing; self.target = facing; }

    // ---- shared checks ----
    fn angle_off(&self, pad: &Pad) -> f32 { rad_adjust(pad.yaw() - self.facing).abs() }

    /// 802C2434: walk (2) or turn relay (1).
    fn check_walk(&mut self, pad: &Pad, allow_turn: bool) -> bool {
        let m = pad.mag2();
        if m < STICK_MOVE || (self.flags & F_STAND == 0 && STICK_RUN <= m) { return false; }
        let d = self.angle_off(pad);
        if !allow_turn || d <= TURN_MIN || TURN_MAX <= d { self.change(2, 0); } else { self.change(1, 0); }
        true
    }
    /// 802C2528: run (4) or turn relay (1).
    fn check_run(&mut self, pad: &Pad, allow_turn: bool) -> bool {
        if !(self.flags & F_STAND == 0 && STICK_RUN <= pad.mag2()) { return false; }
        let d = self.angle_off(pad);
        if !allow_turn || d <= TURN_MIN || TURN_MAX <= d { self.change(4, 0); } else { self.change(1, 0); }
        true
    }
    /// 802C2B0C: double-tap past 0.8 on the same axis and sign within 13 frames -> dash (run phase 3).
    fn check_dash(&mut self, pad: &Pad, out: &mut Option<MotionCmd>) -> bool {
        let m = pad.mag2(); let [x, y] = pad.stick; let [sx, sy] = self.snap;
        if m < STICK_FLICK {
            if self.window != 0 { self.flags = (self.flags & !F_PRESSED) | F_RELEASED; }
            return false;
        }
        if STICK_FLICK < m {
            if self.window == 0 && self.flags & F_PRESSED == 0 {
                self.window = DASH_WINDOW; self.flags = (self.flags & !F_RELEASED) | F_PRESSED; self.snap = [x, y];
            } else if self.flags & F_RELEASED != 0
                && ((sy.abs() < sx.abs() && y.abs() < x.abs() && 0. < sx * x) || (sx.abs() < sy.abs() && x.abs() < y.abs() && 0. < sy * y)) {
                self.flags &= !(F_RELEASED | F_PRESSED);
                self.change(4, 0);
                self.play(RUN, BLEND_DEFAULT, out);
                if let Some(o) = out.as_mut() { o.restart = true; }   // fn_80271424 always restarts
                if self.layer.is_none() { self.layer = Some((SPRINT_LAYER, 0.)); }
                self.window = 0; self.speed = DASH_SPEED; self.phase = 3; self.timer = 0;
                return true;
            }
        }
        false
    }

    // ---- A halves ----
    /// 8027AD20 (locomotion part; the combat checks before it are the caller's).
    fn a_idle(&mut self, pad: &Pad, out: &mut Option<MotionCmd>) {
        if self.phase >= 10 { return; }
        if self.check_dash(pad, out) { return; }
        let turn = self.phase & 1 != 0;
        if self.check_walk(pad, turn) { return; }
        self.check_run(pad, turn);
    }
    /// 8027C504.
    fn a_turn(&mut self, pad: &Pad, out: &mut Option<MotionCmd>) {
        if pad.mag2() < STICK_MOVE { self.change(0, 0); return; }
        if self.timer == 0 && !self.check_dash(pad, out) && !self.check_walk(pad, false) { self.check_run(pad, false); }
    }
    /// 8027C600 (the 1,3 branch needs weapon type 9).
    fn a_walk(&mut self, pad: &Pad, out: &mut Option<MotionCmd>) {
        if self.check_dash(pad, out) { return; }
        if STICK_KEEP <= pad.mag2() { self.check_run(pad, false); } else { self.change(0, 0); }
    }
    /// 8027D164 (802C5B8C dash attack and the type-9 branch are not translated).
    fn a_run(&mut self, pad: &Pad, out: &mut Option<MotionCmd>) {
        if self.check_dash(pad, out) { return; }
        if STICK_KEEP <= pad.mag2() { self.check_walk(pad, false); } else { self.change(0, 0); }
    }

    // ---- B halves ----
    /// 802C7144 + 802C71DC(0.3).
    fn turn_toward_stick(&mut self, pad: &Pad) {
        if STICK_MOVE < pad.mag2() { self.target = pad.yaw(); }
        self.facing = rad_lerp_range(self.facing, self.target, TURN_FRAC * VSYNC, TURN_CLAMP);
    }
    /// Layer weight w <- k * -w, below 0.1 -> 0 and the layer is dropped (8027D2D8 phase 1, literal: a positive
    /// weight vanishes on the first non-dash frame).
    fn fade_layer(&mut self, k: f32) {
        if let Some((_, w)) = &mut self.layer {
            if *w == 0. { return; }
            let nw = k * -*w;
            *w = if nw < 0.1 { 0. } else { nw };
            if *w == 0. { self.layer = None; }
        }
    }
    /// 8027AEC4 -> 8027AFE4 (default weapon; idle fidgets need weapon type 0 and are left out).
    /// 8027AFE4 (weapon type 0): phase 0 plays 0000 and seeds the countdown with rand(0, 2) + 10; phase 1 counts
    /// finished idle loops (`+0x220` taken as "the motion reached its end this frame" [inferred]) and at zero steps the
    /// slot: 1 -> 0008, 3 -> 0007, 0 / 2 -> 0000 with a new countdown, each at blend 4.0 with announcer voice 0x1117.
    fn b_idle(&mut self, _pad: &Pad, motion_end: bool, rand: &mut dyn FnMut(i16, i16) -> i16, out: &mut Option<MotionCmd>) {
        if self.saw {
            // 8027B6F0 (type 6) phase 0: 0425 at blend 4 (the 0000 fade layer and the reversed 0424 / wp0600 draw are
            // not drawn [simplified])
            if self.phase == 0 { self.rate = 1.; self.layer = None; self.play(SAW_IDLE, BLEND_SHORT, out); self.phase = 1; }
        } else if self.phase == 10 {
            // 8027AFE4 phase 10: stow, 0424 at blend 2 (+ wp0600_0424), then 11 -> idle at the motion end
            self.rate = 1.; self.layer = None; self.play(SAW_STOW, 2., out);
            if let Some(o) = out.as_mut() { o.restart = true; }
            self.phase = 11;
        } else if self.phase == 11 {
            if motion_end { self.phase = 0; self.b_idle(_pad, false, rand, out); }
        } else if self.phase == 0 {
            self.rate = 1.;
            self.layer = None;
            self.play(IDLE, BLEND_DEFAULT, out);
            self.timer = 0;
            self.fidget = 0; self.fidget_count = rand(0, 2) as i32 + 10;
            self.phase = 1;
        } else if self.phase == 1 && motion_end {
            if self.fidget_count == 0 {
                self.fidget = (self.fidget + 1) & 3;
                self.voice = Some(FIDGET_VOICE);
                match self.fidget {
                    1 => self.play(FIDGET_A, BLEND_SHORT, out),
                    3 => self.play(FIDGET_B, BLEND_SHORT, out),
                    _ => { self.play(IDLE, BLEND_SHORT, out); self.fidget_count = rand(0, 2) as i32 + 10; }
                }
            } else { self.fidget_count -= 1; }
        }
        // facing keeps easing toward the last target (802C71DC)
        self.facing = rad_lerp_range(self.facing, self.target, TURN_FRAC * VSYNC, TURN_CLAMP);
    }
    /// 8027C744 (default weapon).
    fn b_walk(&mut self, pad: &Pad, _end: bool, out: &mut Option<MotionCmd>) {
        if self.phase == 0 { self.layer = None; self.play(WALK, BLEND_DEFAULT, out); self.phase = 1; }
        self.rate = 1.;
        self.turn_toward_stick(pad);
    }
    /// 8027D2D8 (default weapon).
    fn b_run(&mut self, pad: &Pad, _end: bool, out: &mut Option<MotionCmd>) {
        self.selector = self.selector.wrapping_add(1);
        if self.phase == 0 {
            self.rate = 1.;
            self.play(if self.saw { SAW_RUN } else { RUN }, BLEND_DEFAULT, out);
            self.speed = RUN_RATE;
            self.timer = RUN_TIMER;
            self.phase = 1; self.selector = 0;
        }
        if matches!(self.phase, 1 | 3) {
            self.rate = self.speed;
            if self.phase != 3 { self.fade_layer(0.4); }       // 80569F04
            self.timer -= 1;
            if self.timer < 0 && self.phase == 3 {
                self.speed += DASH_STEP;
                if self.speed <= DASH_MAX { self.rate = DASH_RATE_K * self.speed; } else { self.speed = DASH_MAX; }
                let w = SPRINT_LAYER_K * (self.speed - 1.);
                if let Some((_, lw)) = &mut self.layer { *lw = w; }
            }
        }
        self.turn_toward_stick(pad);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pad(x: f32, y: f32) -> Pad { Pad { stick: [x, y], camera_yaw: 0. } }

    #[test]
    fn full_stick_runs_half_stick_walks_and_release_idles() {
        let mut p = Player::new(0.);
        p.step(&pad(0., 0.), false);
        assert_eq!((p.state, p.motion), (0, IDLE));
        let cmd = p.step(&pad(0., 1.), false).unwrap();
        assert_eq!((p.state, cmd.name, cmd.blend), (4, RUN, BLEND_DEFAULT));
        assert_eq!(p.rate, RUN_RATE);
        p.step(&pad(0., 0.6), false);       // 0.36 < 0.9025 -> walk
        assert_eq!((p.state, p.motion), (2, WALK));
        p.step(&pad(0., 0.18), false);      // 0.0324 >= 0.0256 keeps walking (hysteresis)
        assert_eq!(p.state, 2);
        p.step(&pad(0., 0.1), false);
        assert_eq!((p.state, p.motion), (0, IDLE));
    }

    #[test]
    fn idle_turns_through_the_relay_state_and_faces_the_stick() {
        let mut p = Player::new(0.);
        p.step(&pad(0., 0.), false);        // idle phase 1 -> turn checks allowed
        p.step(&pad(1., 0.), false);        // stick 90 deg off the facing
        assert_eq!(p.state, 1);
        p.step(&pad(1., 0.), false);
        assert_eq!(p.state, 4);
        for _ in 0..20 { p.step(&pad(1., 0.), false); }
        assert!((p.facing - -std::f32::consts::FRAC_PI_2).abs() < 1e-3, "{}", p.facing);
    }

    #[test]
    fn double_tap_dashes_and_ramps_to_the_sprint_cap() {
        let mut p = Player::new(0.);
        p.step(&pad(0., 0.), false);
        p.step(&pad(0., 1.), false);        // press: window opens, run starts
        p.step(&pad(0., 0.), false);        // release inside the window (run -> idle)
        p.step(&pad(0., 1.), false);        // second tap -> dash
        assert_eq!((p.state, p.phase, p.motion), (4, 3, RUN));
        assert!(p.layer.is_some());
        for _ in 0..12 { p.step(&pad(0., 1.), false); }
        assert_eq!(p.speed, DASH_MAX);
        assert!((p.layer.unwrap().1 - SPRINT_LAYER_K * (DASH_MAX - 1.)).abs() < 1e-4);
    }

    #[test]
    fn rad_lerp_range_wraps_and_clamps() {
        assert!((rad_lerp_range(3.0, -3.0, 1., 10.) - -3.0).abs() < 1e-5);
        assert!((rad_lerp_range(0., 3.0, 1., 0.5) - 0.5).abs() < 1e-6);
    }
}
