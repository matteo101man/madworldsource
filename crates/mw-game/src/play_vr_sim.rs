//! The VR simulator (MW_VR=sim, and MW_VR=1 until a headset session runs): a standing player with two Touch
//! controllers, driven by the mouse and keys, or by a script (`MW_VR_SCRIPT`, JSON, on the game's 30 Hz ticks).
//!
//! Keys (see the VR menu's CONTROLS page for the headset's):
//! - mouse: look; hold the right mouse button to move the selected hand instead (wheel: nearer / further)
//! - 1 / 2 / 3: select the left / right / both hands; LMB: trigger; G (held): grip; F: punch (a jab); H: hook; V: chop
//! - Q (held): reach for the holster (G there draws the pistol); C (held): crouch
//! - WASD: left stick; arrow keys: right stick (turning); X / Z: left / right stick click
//! - Space: A, Shift: B, R: X, T: Y, Tab: the menu button; K: shake both controllers
//!
//! Script steps: `{"from": tick, "to": tick, "buttons": ["A", "RTrigger", "LGrip", ...], "lstick": [x, y], "rstick": [x, y],
//! "look": [yaw_deg, pitch_deg], "walk": [x, z], "l": [x, y, z], "r": [x, y, z], "l_to": [..], "r_to": [..], "crouch": 0.5}`:
//! hand positions are offsets (m, head frame: x right, y up, z forward) from the rest pose, eased from `l` / `r` to
//! `l_to` / `r_to` over the step; `walk` moves the head in the room (m per tick, tracking space).
use super::{Hand, VrInput};
use bevy::{input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll}, prelude::*};

const STAND: f32 = 1.72;

#[derive(serde::Deserialize, Clone, Default)]
struct Step {
    from: u64, to: u64, #[serde(default)] buttons: Vec<String>,
    lstick: Option<[f32; 2]>, rstick: Option<[f32; 2]>, look: Option<[f32; 2]>, walk: Option<[f32; 2]>,
    l: Option<[f32; 3]>, r: Option<[f32; 3]>, l_to: Option<[f32; 3]>, r_to: Option<[f32; 3]>, crouch: Option<f32>,
    /// hand rotations: pitch / yaw / roll offsets (degrees) from pointing forward
    l_rot: Option<[f32; 3]>, r_rot: Option<[f32; 3]>,
}

#[derive(Resource, Default)]
pub struct Sim {
    /// Head in the room (tracking space): position, yaw, pitch.
    pos: Vec3, yaw: f32, pitch: f32,
    /// Selected hands (left, right); each hand's offset from its rest pose (head frame) and its rotation offset.
    sel: [bool; 2], off: [Vec3; 2], rot: [Vec3; 2],
    /// A gesture running per hand: (kind, seconds since it started).
    gesture: [Option<(u8, f32)>; 2],
    last: [Vec3; 2], started: bool,
    /// Script time spent while the game is held (menus): 30 Hz units added to the game's tick.
    held: f32,
    /// The script's clock (game ticks plus held time), for MW_VR_SHOTS.
    pub clock: u64,
    /// The game tick last frame and how long it has stood still (an event, a prompt).
    last_tick: u64, stall: f32,
    script: Option<Vec<Step>>,
}

pub fn build(app: &mut App) {
    let script = std::env::var_os("MW_VR_SCRIPT").map(|p| {
        let t = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("MW_VR_SCRIPT {}: {e}", p.to_string_lossy()));
        serde_json::from_str::<Vec<Step>>(&t).unwrap_or_else(|e| panic!("MW_VR_SCRIPT: {e}"))
    });
    app.insert_resource(Sim { pos: Vec3::new(0., STAND, 0.), sel: [false, true], script, ..default() })
        .add_systems(Update, shots);
}

/// The rest pose of hand `h` in the head's yaw frame: chest high, a little forward, pointing ahead.
fn rest(h: usize) -> Vec3 { Vec3::new(if h == 0 { -0.19 } else { 0.19 }, -0.27, 0.36) }

/// A gesture's offset (head frame) and the time it lasts: 0 jab, 1 hook (across), 2 chop (down), 3 shake.
fn gesture(kind: u8, t: f32, h: usize) -> (Vec3, f32) {
    let side = if h == 0 { 1. } else { -1. };
    match kind {
        0 => { let d = 0.22; let k = if t < 0.09 { t / 0.09 } else { (1. - (t - 0.09) / (d - 0.09)).max(0.) }; (Vec3::new(0., 0.18 * k, 0.5 * k), d) }
        1 => { let d = 0.3; let k = (t / 0.13).min(1.); let back = if t > 0.13 { ((t - 0.13) / 0.17).min(1.) } else { 0. };
               (Vec3::new(side * (-0.45 + 0.9 * k) * (1. - back) - side * 0.45 * (1. - k) * 0., 0.2, 0.25 * (1. - back)), d) }
        2 => { let d = 0.3; let k = (t / 0.12).min(1.); let back = if t > 0.12 { ((t - 0.12) / 0.18).min(1.) } else { 0. };
               (Vec3::new(0., (0.45 - 0.75 * k) * (1. - back), 0.3 * (1. - back)), d) }
        _ => { let d = 0.6; (Vec3::new(0., ((t * 22.).sin()) * 0.12, 0.), d) }
    }
}

pub fn read(mut sim: ResMut<Sim>, mut inp: ResMut<VrInput>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, motion: Res<AccumulatedMouseMotion>,
            scroll: Res<AccumulatedMouseScroll>, time: Res<Time<Real>>, play: Res<super::super::Play>, fixed: Res<Time<Fixed>>,
            windows: Query<&Window, With<bevy::window::PrimaryWindow>>, pause: Res<super::super::pause::Pause>) {
    let dt = time.delta_secs().max(1e-4);
    let focused = windows.single().is_ok_and(|w| w.focused);
    let script = sim.script.clone();
    let s = &mut *sim;
    let mut hands = [Hand::default(); 2];
    let mut crouch = 0.;
    if let Some(steps) = &script {
        // the scripted session: every step active at this tick contributes
        if play.tick == s.last_tick { s.stall += dt; } else { s.stall = 0.; s.last_tick = play.tick; }
        let stalled = super::menu_open() || pause.open || s.stall > 0.1;
        if stalled { s.held += dt * 30.; }
        let tick = play.tick + s.held as u64;
        if std::env::var_os("MW_VR_LOG").is_some() && tick / 10 != s.clock / 10 { eprintln!("[vr] script clock {tick} (game tick {}, held {:.1})", play.tick, s.held); }
        s.clock = tick;
        for st in steps.iter().filter(|st| st.from <= tick && tick < st.to) {
            // (between ticks too, so a scripted swing moves every rendered frame)
            let sub = if stalled { s.held.fract() } else { fixed.overstep_fraction() };
            let k = (((tick - st.from) as f32 + sub) / (st.to - st.from).max(1) as f32).clamp(0., 1.);
            if let Some([y, p]) = st.look { s.yaw = y.to_radians(); s.pitch = p.to_radians(); }
            if let Some([x, z]) = st.walk { s.pos += Quat::from_rotation_y(s.yaw) * Vec3::new(x, 0., z) * 30. * dt; }
            if let Some(v) = st.lstick { hands[0].stick = Vec2::from_array(v); }
            if let Some(v) = st.rstick { hands[1].stick = Vec2::from_array(v); }
            if let Some(c) = st.crouch { crouch = c; }
            for (h, (a, b, r)) in [(st.l, st.l_to, st.l_rot), (st.r, st.r_to, st.r_rot)].into_iter().enumerate() {
                if let Some(a) = a { let a = Vec3::from_array(a); s.off[h] = b.map_or(a, |b| a.lerp(Vec3::from_array(b), k * k * (3. - 2. * k))); }
                if let Some(r) = r { s.rot[h] = Vec3::from_array(r); }
            }
            for b in &st.buttons {
                match b.as_str() {
                    "A" => hands[1].primary = true, "B" => hands[1].secondary = true, "X" => hands[0].primary = true, "Y" => hands[0].secondary = true,
                    "Menu" => hands[0].menu = true, "LTrigger" => hands[0].trigger = 1., "RTrigger" => hands[1].trigger = 1.,
                    "LGrip" => hands[0].grip = 1., "RGrip" => hands[1].grip = 1., "LClick" => hands[0].stick_click = true, "RClick" => hands[1].stick_click = true,
                    _ => {}
                }
            }
        }
    } else {
        // live: mouse look (or the selected hand), the keys
        if keys.just_pressed(KeyCode::Digit1) { s.sel = [true, false]; }
        if keys.just_pressed(KeyCode::Digit2) { s.sel = [false, true]; }
        if keys.just_pressed(KeyCode::Digit3) { s.sel = [true, true]; }
        let d = if focused { motion.delta } else { Vec2::ZERO };
        if mouse.pressed(MouseButton::Right) {
            for h in 0..2 { if s.sel[h] { s.off[h] += Vec3::new(d.x * 0.0015, -d.y * 0.0015, scroll.delta.y * 0.04); s.off[h] = s.off[h].clamp(Vec3::new(-0.6, -0.5, -0.3), Vec3::new(0.6, 0.75, 0.45)); } }
        } else {
            s.yaw -= d.x * 0.0025;
            s.pitch = (s.pitch - d.y * 0.0025).clamp(-1.4, 1.4);
            if scroll.delta.y != 0. { for h in 0..2 { if s.sel[h] { s.off[h].z = (s.off[h].z + scroll.delta.y * 0.04).clamp(-0.3, 0.45); } } }
        }
        if keys.just_pressed(KeyCode::Backquote) { s.off = [Vec3::ZERO; 2]; }
        let mut v = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) { v.y += 1.; } if keys.pressed(KeyCode::KeyS) { v.y -= 1.; }
        if keys.pressed(KeyCode::KeyA) { v.x -= 1.; } if keys.pressed(KeyCode::KeyD) { v.x += 1.; }
        hands[0].stick = v.normalize_or_zero() * if keys.pressed(KeyCode::ControlLeft) { 0.6 } else { 1. };
        let mut r = Vec2::ZERO;
        if keys.pressed(KeyCode::ArrowLeft) { r.x -= 1.; } if keys.pressed(KeyCode::ArrowRight) { r.x += 1.; }
        if keys.pressed(KeyCode::ArrowUp) { r.y += 1.; } if keys.pressed(KeyCode::ArrowDown) { r.y -= 1.; }
        hands[1].stick = r;
        hands[1].primary = keys.pressed(KeyCode::Space);
        hands[1].secondary = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        hands[0].primary = keys.pressed(KeyCode::KeyR);
        hands[0].secondary = keys.pressed(KeyCode::KeyT);
        hands[0].menu = keys.pressed(KeyCode::Tab);
        hands[0].stick_click = keys.pressed(KeyCode::KeyX);
        hands[1].stick_click = keys.pressed(KeyCode::KeyZ);
        if keys.pressed(KeyCode::KeyC) { crouch = 0.55; }
        for h in 0..2 {
            if !s.sel[h] { continue; }
            if mouse.pressed(MouseButton::Left) { hands[h].trigger = 1.; }
            if keys.pressed(KeyCode::KeyG) { hands[h].grip = 1.; }
            if s.gesture[h].is_none() {
                if keys.just_pressed(KeyCode::KeyF) { s.gesture[h] = Some((0, 0.)); }
                if keys.just_pressed(KeyCode::KeyH) { s.gesture[h] = Some((1, 0.)); }
                if keys.just_pressed(KeyCode::KeyV) { s.gesture[h] = Some((2, 0.)); }
            }
        }
        if keys.just_pressed(KeyCode::KeyK) { s.gesture = [Some((3, 0.)), Some((3, 0.))]; }
    }
    let head_rot = Quat::from_euler(EulerRot::YXZ, s.yaw, s.pitch, 0.);
    let body_rot = Quat::from_rotation_y(s.yaw);
    let head = Vec3::new(s.pos.x, STAND - crouch, s.pos.z);
    inp.head = Transform { translation: head, rotation: head_rot, scale: Vec3::ONE };
    inp.head_tracked = true;
    let holster = keys.pressed(KeyCode::KeyQ) && script.is_none();
    for h in 0..2 {
        let mut off = s.off[h];
        if let Some((kind, t)) = s.gesture[h] {
            let (g, d) = gesture(kind, t, h);
            off += g;
            s.gesture[h] = if t + dt > d { None } else { Some((kind, t + dt)) };
        }
        // the hands follow the head's yaw, and its pitch half way (as arms do when you look down at them)
        let frame = Quat::from_euler(EulerRot::YXZ, s.yaw, s.pitch * 0.5, 0.);
        // (offsets are written with +z forward; the view looks down -Z)
        let fwd = |v: Vec3| Vec3::new(v.x, v.y, -v.z);
        let mut pos = head + frame * fwd(rest(h) + off);
        if holster && s.sel[h] { pos = Vec3::new(s.pos.x, 0., s.pos.z) + body_rot * Vec3::new(if h == 0 { -0.24 } else { 0.24 }, 0.98, -0.04); }
        let r = s.rot[h];
        let rot = frame * Quat::from_euler(EulerRot::YXZ, r.y.to_radians(), r.x.to_radians(), r.z.to_radians());
        let hand = &mut hands[h];
        hand.grip_pose = Transform { translation: pos, rotation: rot, scale: Vec3::ONE };
        // the aim pose points along the controller, a few centimetres ahead of the grip
        hand.aim_pose = Transform { translation: pos + rot * Vec3::new(0., 0.02, -0.05), rotation: rot, scale: Vec3::ONE };
        hand.tracked = true;
        hand.vel = if s.started { (pos - s.last[h]) / dt } else { Vec3::ZERO };
        s.last[h] = pos;
        hand.trigger_touch = hand.trigger > 0.;
        hand.thumb_touch = hand.stick != Vec2::ZERO || hand.primary || hand.secondary;
    }
    s.started = true;
    inp.hands = hands;
}

/// Inspection: MW_VR_SHOTS="clock:path;..." saves the window at those script-clock ticks (the clock runs on while a
/// menu holds the game), and quits 20 frames after the last.
fn shots(sim: Res<Sim>, mut taken: Local<Vec<u64>>, mut after: Local<u32>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    let Ok(list) = std::env::var("MW_VR_SHOTS") else { return; };
    let parts: Vec<(u64, String)> = list.split(';').filter_map(|p| { let (t, f) = p.split_once(':')?; Some((t.parse().ok()?, f.replace('|', ":"))) }).collect();
    for (t, f) in &parts {
        if sim.clock >= *t && !taken.contains(t) {
            taken.push(*t);
            commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(f.clone()));
        }
    }
    if !parts.is_empty() && taken.len() >= parts.len() { *after += 1; if *after > 20 { exit.write(AppExit::Success); } }
}
