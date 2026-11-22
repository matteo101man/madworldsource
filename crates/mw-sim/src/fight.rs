//! c201 fight simulation: Jack/Kreely (cPl0000) against RinRin (cEm0440) on the native 30 Hz clock.
//! Rules and constants are translated from the DOL / enemy RSO (research/PL0000-COMBAT.md, PL0000-LOCOMOTION.md);
//! data comes from assets/native/combat.json (tools/pl0000/stage_combat.py: motion cues, plem_common.ini damage
//! rows, em0440.csv tuning, em0440_00.emh damage volumes). No rendering here: mw-play presents the result.
use crate::character::{sample_retargeted, strip_root_xz, LocalPose, Retarget};
use crate::player::{rad_lerp_range, MotionCmd, Pad, Player};
use mw_data::{Bone, Clip, MotionBank};
use serde::Deserialize;
use std::collections::HashMap;

// ---------------------------------------------------------------- data

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Cue { #[serde(rename = "type")] pub op: u8, pub frame: f32, pub a: i16, pub b: i16, pub c: i16, pub d: i16, pub f: f32 }

#[derive(Clone, Debug, Deserialize)]
pub struct DamageVolume { pub bone: i16, pub offset: [f32; 3], pub radius: f32 }

#[derive(Debug, Deserialize)]
pub struct CombatData {
    pub cues: HashMap<String, Vec<Cue>>,
    pub damage_pl: HashMap<String, Vec<Option<f32>>>,
    pub damage_em: HashMap<String, Vec<Option<f32>>>,
    pub atk_names_pl: HashMap<String, String>,
    pub atk_names_em: HashMap<String, String>,
    pub rin_tuning: Vec<[f32; 2]>,
    pub rin_damage_volumes: Vec<DamageVolume>,
}

impl CombatData {
    pub fn parse(text: &str) -> Result<Self, String> { serde_json::from_str(text).map_err(|e| e.to_string()) }
    fn row<'a>(names: &HashMap<String, String>, rows: &'a HashMap<String, Vec<Option<f32>>>, atk: u32) -> Option<&'a [Option<f32>]> {
        names.get(&atk.to_string()).and_then(|n| rows.get(n)).map(|r| r.as_slice())
    }
    /// Attack power (column 1) and hit-stop frames (column 2) of a player attack (cAtHitInfo row [0]/[1]).
    pub fn player_attack(&self, atk: u32) -> (i32, i32) {
        let r = Self::row(&self.atk_names_pl, &self.damage_pl, atk);
        (r.and_then(|r| r.first().copied().flatten()).unwrap_or(0.) as i32, r.and_then(|r| r.get(1).copied().flatten()).unwrap_or(0.) as i32)
    }
    pub fn enemy_attack(&self, atk: u32) -> (i32, i32) {
        let r = Self::row(&self.atk_names_em, &self.damage_em, atk);
        (r.and_then(|r| r.first().copied().flatten()).unwrap_or(0.) as i32, r.and_then(|r| r.get(1).copied().flatten()).unwrap_or(0.) as i32)
    }
    /// em0440.csv value (normal difficulty column).
    pub fn tune(&self, i: usize) -> f32 { self.rin_tuning[i][0] }
    fn cues_for(&self, motion: &str) -> &[Cue] {
        let seq = motion.replace(".mot", "_00.seq");
        self.cues.get(&seq).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

// ---------------------------------------------------------------- motion player (cByMotion subset)

#[derive(Clone, Debug)]
pub struct Motion {
    pub clip: usize, pub frame: f32, pub rate: f32, pub looping: bool, pub ended: bool,
    /// Crossfade source for presentation: (clip, frame, k, n).
    pub blend: Option<(usize, f32, f32, f32)>,
    /// Number of motion starts (a change means the root-motion delta restarts from frame 0).
    pub starts: u32,
    fresh: bool,
    clock_rate: f32,
    root_value: [f32; 2], root_initial: [f32; 2], root_primed: bool,
    pose: Vec<LocalPose>,
    /// Enemy model sample (frame, remaining blend) before advance changes them.
    pose_sample: Option<(f32,f32)>,
}

impl Motion {
    /// Frames one game frame advances (rate x the scene clock: near 0 in the lethal-hit hold), for presenters
    /// interpolating between game frames; and the crossfade's per-frame step.
    pub fn step(&self) -> f32 { self.rate * self.clock_rate }
    pub fn blend_step(&self) -> f32 { self.rate * self.clock_rate * self.clock_rate }
    pub fn new(clip: usize, looping: bool) -> Self { Motion { clip, frame: 0., rate: 1., looping, ended: false, blend: None, starts: 0, fresh: true, clock_rate: 1., root_value: [0.; 2], root_initial: [0.; 2], root_primed: false, pose: vec![], pose_sample:None } }
    /// fn_80078CD0-style start: `restart` false keeps a motion that is already playing (fn_802716EC).
    pub fn play(&mut self, clip: usize, blend: f32, looping: bool, restart: bool) -> bool {
        if clip == self.clip && !restart && !self.ended { self.looping = looping; return false; }
        self.blend = if blend > 0. { Some((self.clip, self.frame, 0., blend)) } else { None };
        let mut pose = std::mem::take(&mut self.pose);
        // set__9cByMotion clears the synthetic root, retaining the model parts.
        if let Some(root) = pose.last_mut() {
            *root = LocalPose { translation: [0.; 3], rotation: [0., 0., 0., 1.], scale: [1.; 3] };
        }
        *self = Motion { clip, frame: 0., rate: self.rate, looping, ended: false, blend: self.blend, starts: self.starts + 1, fresh: true, clock_rate: self.clock_rate, root_value: [0.; 2], root_initial: [0.; 2], root_primed: false, pose, pose_sample:None };
        true
    }
    /// One game frame; returns the cues crossed (frame in (from, to], plus frame 0 on the first update).
    pub fn advance(&mut self, len: usize, cues: &[Cue]) -> Vec<Cue> {
        self.pose_sample=Some((self.frame,self.blend.map(|b|(b.3-b.2).max(0.)).unwrap_or(0.)));
        let last = (len.max(2) - 1) as f32;
        let from = if self.fresh { -1. } else { self.frame };
        self.fresh = false;
        let mut to = self.frame + self.rate * self.clock_rate;
        let mut hits: Vec<Cue> = cues.iter().filter(|c| c.frame > from && c.frame <= to.min(last)).copied().collect();
        if to >= last {
            // a one-shot motion reports its end one frame after reaching the last frame (80079210; the frame
            // recording holds 1011 / 1090 / 0104 on their last frame for one frame before the state changes) [observed]
            if self.looping {
                to -= last; hits.extend(cues.iter().filter(|c| c.frame <= to).copied());
            }
            else { if to > last { self.ended = true; } to = last; }
        }
        self.frame = to;
        // kept one frame past full weight so the root blend (blend_root) sees the step that completes it
        if let Some(b) = &mut self.blend { b.2 += self.rate * self.clock_rate * self.clock_rate; if b.2 >= b.3 + 1. { self.blend = None; } }
        hits
    }
}

// ---------------------------------------------------------------- pose helpers

fn qmul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1], a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
     a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3], a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2]]
}
fn qrot(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let u = [q[0], q[1], q[2]]; let s = q[3];
    let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2]; let uu = u[0] * u[0] + u[1] * u[1] + u[2] * u[2];
    let c = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    [0, 1, 2].map(|i| 2. * dot * u[i] + (s * s - uu) * v[i] + 2. * s * c[i])
}

/// Actor-space bone frames (rotation, position, scale) from local poses (bones in parent order, MOT root last).
pub fn bone_frames(bones: &[Bone], poses: &[LocalPose]) -> Vec<([f32; 4], [f32; 3], [f32; 3])> {
    let n = bones.len();
    let root = poses[n];
    let mut out: Vec<([f32; 4], [f32; 3], [f32; 3])> = Vec::with_capacity(n);
    for (i, b) in bones.iter().enumerate() {
        let (pr, pp, ps) = if b.parent >= 0 && (b.parent as usize) < i { out[b.parent as usize] } else { (root.rotation, root.translation, root.scale) };
        let p = poses[i];
        let t = qrot(pr, [p.translation[0] * ps[0], p.translation[1] * ps[1], p.translation[2] * ps[2]]);
        out.push((qmul(pr, p.rotation), [pp[0] + t[0], pp[1] + t[1], pp[2] + t[2]], [ps[0] * p.scale[0], ps[1] * p.scale[1], ps[2] * p.scale[2]]));
    }
    out
}

/// World point of an actor-space point for an actor at `pos` facing `yaw` (yaw 0 faces +Z).
pub fn to_world(pos: [f32; 3], yaw: f32, p: [f32; 3]) -> [f32; 3] {
    let (s, c) = yaw.sin_cos();
    [pos[0] + c * p[0] + s * p[2], pos[1] + p[1], pos[2] - s * p[0] + c * p[2]]
}

/// Inverse of `to_world`.
pub fn from_world(pos: [f32; 3], yaw: f32, w: [f32; 3]) -> [f32; 3] {
    let (s, c) = yaw.sin_cos(); let (x, z) = (w[0] - pos[0], w[2] - pos[2]);
    [c * x - s * z, w[1] - pos[1], s * x + c * z]
}

/// Move bone `idx` of a local pose so it sits at actor-space `target`, spun by `spin` about Y (thrown props).
pub fn place_bone(bones: &[Bone], poses: &mut [LocalPose], idx: usize, target: [f32; 3], spin: f32) {
    let frames = bone_frames(bones, poses);
    let b = &bones[idx];
    let (pr, pp, ps) = if b.parent >= 0 { frames[b.parent as usize] } else { (poses[bones.len()].rotation, poses[bones.len()].translation, poses[bones.len()].scale) };
    let inv = [-pr[0], -pr[1], -pr[2], pr[3]];
    let d = qrot(inv, [target[0] - pp[0], target[1] - pp[1], target[2] - pp[2]]);
    poses[idx].translation = [d[0] / ps[0].max(1e-6), d[1] / ps[1].max(1e-6), d[2] / ps[2].max(1e-6)];
    let (sh, ch) = (spin * 0.5).sin_cos();
    poses[idx].rotation = qmul(inv, [0., sh, 0., ch]);
}

fn dist2_xz(a: [f32; 3], b: [f32; 3]) -> f32 { (a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2) }
fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 { (0..3).map(|i| (a[i] - b[i]).powi(2)).sum() }
fn yaw_to(from: [f32; 3], to: [f32; 3]) -> f32 { (to[0] - from[0]).atan2(to[2] - from[2]) }

// ---------------------------------------------------------------- deterministic RNG

/// Hw::cRand (800BB89C / 800BB8C0): seed = seed * 1664525 + 1013904223. The shared instance at 0x80609B90 is seeded at
/// runtime (bss) and other game systems draw from it too, so sequences match the original only in algorithm [seed
/// inferred: `MW_SEED`].
#[derive(Clone, Debug)]
pub struct Rng(u32);
impl Rng {
    pub fn new(seed: u64) -> Self { Rng(seed as u32) }
    fn next(&mut self) -> u32 { self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); self.0 }
    /// getS16 (800BB79C): lo + ((seed >> 16) * (hi - lo + 1) >> 16).
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let r = (self.next() >> 16) & 0xFFFF;
        (lo as i16).wrapping_add(((r as i32 * (hi - lo + 1)) >> 16) as i16) as i32
    }
    /// getF32 (800BB7E8): a * t + b * (1 - t), t = (seed >> 8) / 16777215.
    pub fn rangef(&mut self, a: f32, b: f32) -> f32 {
        let t = (self.next() >> 8) as f32 / 16_777_215.;   // 80536600
        a * t + b * (1. - t)
    }
}

// ---------------------------------------------------------------- player combat (state 39 and damage)

/// One game frame of pad input in game terms (PC bindings map onto these).
#[derive(Clone, Copy, Default, Debug)]
pub struct FightInput {
    /// Original demo nesting is active (the opening event can overlap the fight).
    pub demo_active: bool,
    pub stick: [f32; 2], pub camera_yaw: f32,
    /// A (punch) trigger / held; B trigger / held.
    pub a_trig: bool, pub a_held: bool, pub b_trig: bool, pub b_held: bool,
    /// Wii-remote swing this frame: 1 vertical, 3 horizontal (chainsaw).
    pub swing: u8,
    /// 8-sample mean of |remote accel| - 1 g (`+0x1088`), and the two vector means of finisher phase 14
    /// (`+0x1090` remote, `+0x109C` nunchuk). PC held X = Gesture::Shake profile 9, measured 1.59 [observed].
    pub shake: f32, pub shake_remote: f32, pub shake_nunchuk: f32,
    /// Continue after death (the room-script layer's job; not translated) [inferred trigger].
    pub cont: bool,
    /// Jump button (pad bit 0x1000 -> `+0x1104` = 8).
    pub jump: bool,
    /// Nunchuk shake metric `+0xF64` this frame (dodge when > 1.2).
    pub nunchuk_shake: f32,
    /// Added (Kreely's moveset, PC): the heavy attack (Q), the grab (F; `grab_grunt` when the presenter has a street
    /// grunt in reach), the pistol whip (punch while aiming at point-blank range).
    pub heavy: bool, pub grab: bool, pub grab_grunt: bool, pub whip: bool,
    /// The native grab of a street grunt (research/GRAB-THROW-MINIMAP.md): the presenter has a grunt to catch (2.5 m,
    /// 80 deg, standing), the held grunt broke free or is gone, and the hold actions (PC: F overhead throw, Q side
    /// throw, X head-butt; native: remote swings and the nunchuk shake).
    pub carry_ok: bool, pub carry_lost: bool, pub throw_v: bool, pub throw_s: bool, pub headbutt: bool,
    /// The native pick-up of a placed object (the same A hold, alternating with the grunt grab): the presenter has one
    /// in reach (`pick_ok`), and a grunt stands close enough for the over-the-head slam (`cover_ok`, 2.5 m).
    pub pick_ok: bool, pub cover_ok: bool,
    /// Added (Kreely, PC Q while carrying a grunt): his suplex on the grunt in his hands.
    pub suplex_held: bool,
    /// The taunt (pad trigger bit 4, fn_802C68AC -> state 0x54; PC: T).
    pub taunt: bool,
    /// Kreely's run-and-gun (added): with the pistol out he moves at GUN_RUN instead of the aiming walk.
    pub gun_run: bool,
}

/// The native grab of a street grunt in progress (player states 0x14 grab, 0x16 hold, 0x17 release, 0x28 hold action):
/// the action (0 none, 1 WHANG_THROW, 2 WHANG_SIDE_THROW, 3 HANG_HEADBUT) and whether the head-butt has landed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Carry { pub act: u8, pub hit: bool,
                   /// an object, not a grunt (states 0x1A pick, 0x1B hold, 0x1D throw, 0x1E put down, 0x23 slam over a head)
                   pub object: bool }
/// The object states' key frames: taken (021a), let go in the throw (021d), set down (021c), on the head (0218)
/// [021a frame 6, 021d frame 6, 021c frame 7 obs; 0218's frame inferred].
const PICK_TAKE: f32 = 6.;
const OBJ_RELEASE: f32 = 6.;
const OBJ_DROP: f32 = 7.;
const OBJ_COVER: f32 = 10.;
/// Hold steering (80280244: +0x3B8 -= 0.069813 x stick X a frame) and the head-butt's hit frame (pl0030_0223_00.seq op 7).
const CARRY_STEER: f32 = 0.069813;
const HEADBUTT_HIT: f32 = 12.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackHit { pub atk: u32, pub center: [f32; 3], pub radius: f32, pub power: i32, pub hitstop: i32 }

/// Player damage volumes (802705DC): (part, radius).
pub const PLAYER_DAMAGE_PARTS: [(usize, f32); 7] = [(0x18, 0.14), (2, 0.2), (0xF, 0.18), (8, 0.14), (0xD, 0.14), (0x11, 0.16), (0x15, 0.16)];
pub const PLAYER_HIT_RADIUS: f32 = 0.6;   // 8056A008
pub const PLAYER_MAX_HP: i16 = 100;       // +0xEE [observed]
pub const RIN_MAX_HP: i16 = 1600;         // [observed]; em_common.ini [em0440] Hp = 1600
/// cEmBoss::getAtkPower(i) (81006300) reads em_common.ini [em0440] "boss attack i": 1 uraken, 2 tetuzankou,
/// 3 kamikaze, 4 fan throw, 5 get-up / explosion / scaffold bomb, 6 giant slalom.
pub const RIN_ATK_POWER: [i32; 6] = [10, 10, 10, 10, 5, 15];
pub const BODY_GAP: f32 = 0.85;
pub const JUMP_POWER: f32 = 0.151;        // +0x118C from 80569F3C
pub const GRAVITY: f32 = 0.0145;          // +0x1188 from 80569F38           // player 0.35 + RinRin 0.5 body radii [inferred]

#[derive(Clone, Debug)]
pub struct PlayerFight {
    /// Added: Kreely's moveset is on (set from `Fight::kreely` each frame); the next attack is the pistol whip.
    pub kreely: bool, whip: bool,
    /// The stick when the punch was pressed (a punch starts on its release; Kreely's directional openers read this).
    press_stick: [f32; 2],
    pub loco: Player, pub pos: [f32; 3], pub motion: Motion, pub hp: i16,
    buf_a: i32, buf_b: i32, combo: u32, flags: u32, swing: u8, death_timer: i32,
    /// Rush window `+0x12CC` (45 frames from A0) and whether the current attack is the rush (phase 0x32/0x33).
    rush_timer: i32, rushing: bool,
    /// Air attack (`+0x11A5`): 0 none, 1 air punch 1122 (phase 0x14/0x15), 2 landing slam 1123 (0x16/0x17).
    air_atk: u8,
    /// Grab: frames A has been held with the buffer spent (802C5DD8), giant-swing spin frames and strength.
    a_hold: i32, spin: f32, spin_power: f32, swing_timer: i32,
    /// Knockdown type `+0x1AB8` (1 after 1011 / 1016): picks the get-up motion.
    knock_type: u8,
    /// Stun counter `+0x1560`: set to 0x4B0 >> 1 by a 0x18C (returning fan) hit, else clamped to it, -1 per frame
    /// (frameUpdate); > 0x1C2 when a reaction ends -> dizzy state 91. Dizzy meter `+0x1240`.
    pub stun: i16, dizzy: f32,
    /// Root step computed last frame (player's local frame), applied this frame.
    root_lag: [f32; 2],
    /// Grab latch (flags1124 0x800, 802C5DD8): one grab attempt per A hold.
    grab_latch: bool,
    /// Turn target `+0x3B8` while reacting (B[85] turns with 802C71DC).
    turn_to: f32,
    /// Jump: buffer `+0x1104`, velocity `+0x117C..+0x1184`, airborne, and the floor height the presenter reports.
    buf_jump: i32, vel: [f32; 3], pub airborne: bool, pub floor_y: f32,
    atk: u32, power: i32, part: usize, hit_trigger: bool, hit_window: bool, pub hit_serial: u32, combo_window: bool, lunge: f32,
    presses: i32, invuln: f32, damage_guard: f32,
    pub last_hit: Option<AttackHit>,
}

const PF_100: u32 = 0x100; const PF_40: u32 = 0x40;

impl PlayerFight {
    pub fn new(pos: [f32; 3], facing: f32, idle: usize) -> Self {
        PlayerFight { loco: Player::new(facing), pos, motion: Motion::new(idle, true), hp: PLAYER_MAX_HP, buf_a: 0, buf_b: 0, combo: 0, death_timer: 0, knock_type: 0, stun: 0, dizzy: 0., grab_latch: false, root_lag: [0.; 2], turn_to: facing,
                      buf_jump: 0, vel: [0.; 3], airborne: false, floor_y: pos[1], rush_timer: 0, rushing: false, air_atk: 0, a_hold: 0, spin: 0., spin_power: 0., swing_timer: 0,
                      flags: 0, swing: 0, atk: 0, power: 0, part: 14, hit_trigger: false, hit_window: false, hit_serial: 0, combo_window: false, lunge: 0., presses: 0,
                      invuln: 0., damage_guard: 0., last_hit: None, kreely: false, whip: false, press_stick: [0.; 2] }
    }
    pub fn state(&self) -> u8 { self.loco.state }
    /// [added, VR] Held up by the hands (climbing): at `pos`, standing on nothing, no fall.
    pub fn vr_hang(&mut self, pos: [f32; 3]) { self.pos = pos; self.airborne = false; self.floor_y = pos[1]; self.vel = [0.; 3]; }
    /// [added, VR] Let go in the air: he falls (the standing jump's airborne state) to the floor under him.
    pub fn vr_drop(&mut self) {
        if matches!(self.loco.state, 0 | 1 | 2 | 4) { self.loco.state = 15; self.loco.phase = 1; }
        self.airborne = true; self.vel = [0.; 3];
    }

    /// 802C3B5C (default weapon, no grab): enter state 39 from a buffered tap or a swing.
    fn check_attack(&mut self, inp: &FightInput, chain: bool) -> bool {
        if !chain { self.combo = 0; self.flags &= !PF_40; }
        let tap = self.buf_a != 0 && !inp.a_held;
        if inp.swing != 0 { self.combo = inp.swing as u32; }
        let heavy = self.kreely && inp.heavy;
        if (self.flags & 0x800 == 0) && (tap || inp.swing != 0 || heavy || self.whip) {
            self.swing = if heavy { 2 } else { (inp.swing != 0) as u8 };
            self.buf_a = 0; self.buf_b = 0;
            self.loco.enter_combat(39); self.loco.phase = 0;
            return true;
        }
        false
    }

    /// State 39 phase 0 (80285744): pick the attack motion.
    fn start_attack(&mut self, d: &FightData, inp: &FightInput, rin_hp: i16) {
        if self.kreely { if let Some(kit) = &d.kreely { self.kreely_attack(d, kit, inp); return; } }
        self.motion.rate = 1.;
        self.flags &= !0x4100;
        // B[39] top: combo running, rush window open and more than 4 presses -> rush (phase 0x32): 1111 RUSH_PUNCH,
        // or 1112 FINISH_BLOW when the target has 10 HP or less
        if self.swing == 0 && self.combo != 0 && self.rush_timer > 0 && self.presses > 4 {
            let (name, atk) = if rin_hp <= 10 { ("1112", 0x5F) } else { ("1111", 0x5E) };
            self.combo = 0; self.presses = 0; self.rushing = true;
            self.atk = atk; self.power = 10; self.part = 14;
            self.hit_trigger = false; self.hit_window = false; self.combo_window = false; self.lunge = 0.;
            self.motion.play(d.jack_clip(&format!("pl0030_{name}.mot")), crate::player::BLEND_DEFAULT, false, true);
            self.loco.phase = 1;
            return;
        }
        self.rushing = false;
        let (name, atk) = if self.swing == 0 {
            let mask = if self.flags & PF_40 != 0 { 7 } else { 3 };
            let idx = self.combo & mask; self.combo = idx;
            if idx == 0 || inp.stick[1] <= 0.2 {
                let n = ["1100", "1101", "1102", "1103"][(idx & 3) as usize];
                if idx == 0 { self.presses = 0; self.rush_timer = 45; } else if idx == 4 { self.presses = 0; self.rush_timer = 60; }
                (n, 0x6D + idx)
            } else { self.flags |= PF_100; ("1105", 0x77) }
        } else {
            let idx = self.combo & 3;
            let r = match idx { 0 | 1 => { self.flags |= PF_100; ("1104", 0x76) } 2 => { self.flags |= PF_100; ("1105", 0x77) } _ => { self.flags |= PF_100; ("1106", 0x78) } };
            self.combo = if idx == 1 { 4 } else { 0 };
            r
        };
        if self.swing == 0 {
            if self.flags & PF_100 == 0 { self.power = 10; self.combo += 1; } else { self.power = 15; self.combo = 0; }
        } else { self.power = 15; }
        if self.combo == 0 { self.flags &= !PF_40; }
        self.atk = atk;
        self.part = 14;
        self.hit_trigger = false; self.hit_window = false; self.combo_window = false; self.lunge = 0.;
        let clip = d.jack_clip(&format!("pl0030_{name}.mot"));
        self.motion.play(clip, crate::player::BLEND_DEFAULT, false, true);
        self.loco.phase = 1;
    }
}

impl PlayerFight {
    /// Kreely's attack choice (added; God Hand motions, crate::kreely): punches chain cross, mirrored cross, knee; forward + punch opens with the step jab, back + punch with the turning jab; the heavy button is the
    /// powered punch, forward + heavy the spin kick (knockdown), back + heavy the sweep (knockdown); mashing in the
    /// string's rush window is the full combo; punch while aiming at point-blank range is the pistol whip.
    fn kreely_attack(&mut self, d: &FightData, kit: &crate::kreely::KreelyKit, inp: &FightInput) {
        // heavies read the stick now (they start on the press); punches the stick held at their press
        let y = if self.swing == 2 { inp.stick[1] } else if inp.stick[1].abs() > 0.2 { inp.stick[1] } else { self.press_stick[1] };
        let key = if self.whip {
            self.whip = false; self.combo = 0;
            if self.presses % 2 == 0 { "whip" } else { "whip_alt" }
        } else if self.swing == 2 {
            self.combo = 0;
            if y > 0.2 { "spin_kick" } else if y < -0.2 { "sweep" } else { "power_punch" }
        } else if self.combo != 0 && self.rush_timer > 0 && self.presses > 4 {
            self.combo = 0; self.presses = 0; "rush"
        } else {
            let idx = self.combo % 3;
            if idx == 0 { self.presses = 0; self.rush_timer = 45; }
            let k = if idx == 0 && y > 0.2 { "step_jab" } else if idx == 0 && y < -0.2 { "turn_jab" } else { ["cross", "cross_m", "knee"][idx as usize] };
            self.combo = (idx + 1) % 3;
            k
        };
        let Some(m) = kit.moves.get(key) else { return; };
        self.rushing = false;
        self.atk = m.atk; self.power = m.power; self.part = m.part;
        self.hit_trigger = false; self.hit_window = false; self.combo_window = false; self.lunge = 0.;
        self.motion.play(d.jack_clip(&m.clip), crate::player::BLEND_SHORT, false, true);
        self.loco.phase = 1;
    }
}

/// Added: Kreely's suplex in progress (state 0xCA): who is thrown, where both stand (the grab's frame), the receiver's
/// clip frame, whether the slam has landed.
#[derive(Clone, Debug)]
pub struct Suplex { pub on_rin: bool, pub origin: [f32; 3], pub facing: f32, pub target: [f32; 3], pub target_facing: f32, pub slammed: bool,
                   /// The grab reach first (the native grab 0226 up to its contact): frames left, and where RinRin started.
                   pub grab: Option<f32>, pub rin_from: [f32; 3] }
/// The native grab's contact frame (pl0030_0226_00.seq hit window 5-10): the suplex takes over there.
const GRAB_CONTACT: f32 = 10.;
pub const STATE_SUPLEX: u8 = 0xCA;
/// Added: the player's motion is set by the presenter (a boss's paired sequence such as Elise's clash); the clip plays
/// in place and the player takes no input until `end_external`.
pub const STATE_EXTERNAL: u8 = 0xCB;
/// RinRin being suplexed (added state, outside her native table).
pub const RIN_SUPLEXED: u8 = 40;
/// [added, VR] Held up in Kreely's hands (the VR hold: carried, thrown, or lifted overhead into his suplex).
pub const RIN_VR_HELD: u8 = 42;
/// [added, VR] How long she stays held before she breaks free (ticks: 5 s).
pub const VR_HOLD_TICKS: u32 = 150;
const SUPLEX_POWER: i32 = 150;
const SUPLEX_REACH: f32 = 1.6;

// ---------------------------------------------------------------- RinRin

#[derive(Clone, Debug)]
pub struct Rin {
    pub pos: [f32; 3], pub facing: f32, pub motion: Motion, pub hp: i16,
    pub state: u8, pub phase: u8, pub selector: u8,
    cd_atk: f32, cd_slalom: f32, last_atk: u32, hit_on: bool, hit_bone: i16, guard_timer: f32,
    /// Fan hit volumes reserved last frame, resolved this frame.
    pub fan_hits: Vec<(usize, u32, [f32; 3])>,
    /// Root step computed last frame (Rin's local frame), applied this frame.
    pub root_lag: [f32; 2],
    /// Rig flag 0 (`+0x864` bit 0): the keen-competition window, set / cleared by cue 35 (a = 0) in em0440_0105
    /// (frames 10 / 60), cleared by end_atk_giantslalom (810D08B0); chkEnableKeenCompetition (810CB518) reads it.
    pub rig0: bool,
    /// Last player attack serial taken (one cAtHit hits a target once).
    hit_by: u32,
    /// Additive finisher layer em0440_0123 (+0x834) and its weight.
    pub layer: Option<(usize, f32)>,
    pub fans: [Fan; 2],
    /// Scaffold work: current / next point (+0x10A4 / +0x10A6), timers (+0x10EC to-scaffold, +0x10E0 on-scaffold),
    /// jump start position, flag 0 (heading to a scaffold), whether she stands on one, bombs thrown this visit.
    pub on_scaffold: bool, sc_cur: usize, sc_next: usize, sc_timer: f32, sc_time: f32, sc_start: [f32; 3],
    sc_heading: bool, bombs_done: bool, pub bombs: Vec<Bomb>,
    /// Down after an explosion-class hit (`+0x10D1`) and the stun timer.
    pub down: bool, down_timer: f32,
    /// Thrown by the giant swing: velocity (+0x8BC..+0x8C4).
    pub thrown: Option<[f32; 3]>,
    /// [added, VR] Ticks she has been held (she breaks free after a while).
    pub vr_held: u32,
}

pub mod rin_state {
    pub const WAIT: u8 = 0; pub const RUN: u8 = 1; pub const URAKEN: u8 = 2; pub const TETUZANKOU: u8 = 3;
    pub const RIGHT_STEP: u8 = 9; pub const LEFT_STEP: u8 = 10; pub const BACK_STEP: u8 = 11; pub const THROW: u8 = 14;
    pub const DM_EXPLOSION: u8 = 16; pub const DM_RIGHT: u8 = 21; pub const DM_LEFT: u8 = 22; pub const DM_BODY: u8 = 23;
    pub const PROVOCATION: u8 = 24; pub const FINISH_WAIT: u8 = 27; pub const SLALOM: u8 = 28; pub const GUARD: u8 = 29;
    pub const PL_DEAD: u8 = 31; pub const FINISH: u8 = 26; pub const DEAD: u8 = 0xFF; pub const KEEN_COMPE: u8 = 17;
    /// Added: the last-life execution (play_execution.rs): she walks to the fallen player and wins the clash pair.
    pub const EXECUTE: u8 = 41;   // (40 is RIN_SUPLEXED)
    /// Added: the actor editor plays a clip on her (Fight::rin_play).
    pub const EDITOR: u8 = 42;
    pub const SCAFFOLD_JUMP: u8 = 4; pub const RIGHT_JUMP: u8 = 5; pub const LEFT_JUMP: u8 = 6; pub const FRONT_JUMP: u8 = 7;
    pub const KAMIKAZE: u8 = 8; pub const BOMB: u8 = 13; pub const SCAFFOLD_WAIT: u8 = 25; pub const DM_DOWN: u8 = 30; pub const DM_GIANTSWING: u8 = 19; pub const DM_ELECTRIC: u8 = 20;
}
use rin_state as rs;

/// RinRin's scaffold points (static initializer at 810D1950 -> 0x81174728, stride 0x18): position, facing, and the
/// jump table (kind 1 right / 2 left / 3 front, target point).
pub const SCAFFOLDS: [([f32; 3], f32, [(u8, usize); 2]); 9] = [
    ([10.34, 23.55, -83.65], -std::f32::consts::FRAC_PI_2, [(2, 1), (3, 8)]),
    ([10.34, 23.55, -79.82], -std::f32::consts::FRAC_PI_2, [(2, 2), (1, 0)]),
    ([10.34, 23.55, -75.95], -std::f32::consts::FRAC_PI_2, [(2, 3), (1, 1)]),
    ([12.17, 23.55, -70.80], -std::f32::consts::FRAC_PI_2, [(2, 4), (1, 2)]),
    ([12.17, 23.55, -64.11], -std::f32::consts::FRAC_PI_2, [(3, 5), (1, 3)]),
    ([-5.93, 23.55, -62.06], std::f32::consts::FRAC_PI_2, [(3, 6), (3, 4)]),
    ([-6.12, 23.55, -75.16], std::f32::consts::FRAC_PI_2, [(3, 5), (2, 7)]),
    ([-6.12, 23.55, -81.70], std::f32::consts::FRAC_PI_2, [(2, 8), (1, 6)]),
    ([-5.68, 23.55, -84.80], std::f32::consts::FRAC_PI_2, [(3, 0), (1, 7)]),
];

/// A dropped bomb, cBl0063 (eObjId 0x70063, bl/bl0063.dat). atk_bomb's objCreate (810CDBD4) creates it 20 above a
/// point within 8 of the arena centre and lights it: setFire(rand 150..300) (8018D32C). The fuse burns down at the
/// game speed (fn_8018D0B4) and explode (8018D3B8) starts EST 0 and a 100-frame blast timer; about the third frame
/// of it (timer 96-97) one attack sphere of radius 5 hits (type 0xF0, power 25). A bomb inside a blast goes off at
/// once (+5168) [inferred]. Fall, bounce and roll are Havok rigid-body physics on bl0063 col.hkx natively; here a
/// ball of radius BOMB_RADIUS under 9.8 m/s^2 with BOMB_BOUNCE restitution and rolling friction [inferred].
#[derive(Clone, Debug, Default)]
pub struct Bomb { pub pos: [f32; 3], vel: [f32; 3], pub live: bool,
                  /// Native frames of fuse left (300 at most; the fuse motion's frame is 300 - fuse) and of the blast (100 down).
                  pub fuse: f32, pub blast: f32,
                  /// Rolling: the ball's rotation (x, y, z, w) about its centre.
                  pub turn: [f32; 4], hit_done: bool }
pub const BOMB_RADIUS: f32 = 0.54;
const BOMB_BOUNCE: f32 = 0.35;
const BOMB_GRAVITY: f32 = 9.8 / (30. * 30.);
const BOMB_BLAST_RADIUS: f32 = 5.;
const BOMB_POWER: i32 = 25;
/// objCreate's drop centre (static vector 81500510 = (2.5, 17.65, -73.3), set by 810D1958; also the kamikaze target).
const BOMB_CENTRE: [f32; 3] = [2.5, 17.65, -73.3];

/// cEm0440::cIronfan: a thrown fan (startup 810CA20C: part 100 on hand 9, part 115 on hand 14).
#[derive(Clone, Debug, Default)]
pub struct Fan {
    /// 0 held, 1/2 outbound, 3/4 returning (cIronfan +0x39).
    pub phase: u8, pub t: f32, dt: f32, p0: [f32; 3], target: [f32; 3], t0: [f32; 3], t1: [f32; 3],
    pub pos: [f32; 3], pub spin: f32, hit: bool,
    local: [f32; 3],
    /// Rin part id carrying this fan and its hand.
    pub part: i16, pub hand: i16,
}
impl Fan {
    pub fn step_size(&self) -> f32 { self.dt }
    pub fn local_position(&self) -> [f32;3] { self.local }
    pub fn world_target(&self) -> [f32;3] { self.target }
}

/// getHermiteCurvePoint (8008C924): P0, T0, P1, T1.
pub fn hermite(t: f32, p0: [f32; 3], t0: [f32; 3], p1: [f32; 3], t1: [f32; 3]) -> [f32; 3] {
    let (t2, t3) = (t * t, t * t * t);
    let (h00, h10, h01, h11) = (1. + 2. * t3 - 3. * t2, t + t3 - 2. * t2, -2. * t3 + 3. * t2, t3 - t2);
    [0, 1, 2].map(|i| p0[i] * h00 + t0[i] * h10 + p1[i] * h01 + t1[i] * h11)
}

// ---------------------------------------------------------------- the fight

pub struct FightData {
    pub jack: MotionBank, pub rin: MotionBank, pub combat: CombatData,
    jack_idx: HashMap<String, usize>, rin_idx: HashMap<String, usize>,
    /// c201 collision for RinRin's fence checks (bodies named `*_ST10`: surface type 0x10, the electrified ring).
    pub world: Option<mw_data::collision::CollisionWorld>,
    /// Kreely's added moveset (crate::kreely), when staged.
    pub kreely: Option<crate::kreely::KreelyKit>,
}

impl FightData {
    pub(crate) fn reindex(&mut self) {
        self.jack_idx = self.jack.clips.iter().enumerate().map(|(i, c)| (c.name.clone(), i)).collect();
        self.rin_idx = self.rin.clips.iter().enumerate().map(|(i, c)| (c.name.clone(), i)).collect();
    }
    pub fn new(mut jack: MotionBank, rin: MotionBank, combat: CombatData) -> Self {
        // pl0030 motions shipped in em0440.dat (paired clash / finisher roles) join Jack's bank
        for c in &rin.clips {
            if c.name.starts_with("pl0030_") && !jack.clips.iter().any(|j| j.name == c.name) { jack.clips.push(c.clone()); }
        }
        let jack_idx = jack.clips.iter().enumerate().map(|(i, c)| (c.name.clone(), i)).collect();
        let rin_idx = rin.clips.iter().enumerate().map(|(i, c)| (c.name.clone(), i)).collect();
        FightData { jack, rin, combat, jack_idx, rin_idx, world: None, kreely: None }
    }
    /// Added: player clips from another bank (a boss's paired clips, e.g. em0630.dat's pl0030_046x clash set).
    pub fn add_player_clips(&mut self, clips: Vec<mw_data::Clip>) {
        for c in clips { if !self.jack.clips.iter().any(|j| j.name == c.name) { self.jack.clips.push(c); } }
        self.reindex();
    }
    pub fn jack_clip(&self, name: &str) -> usize { *self.jack_idx.get(name).unwrap_or_else(|| panic!("{name} not staged")) }
    pub fn rin_clip(&self, name: &str) -> usize { *self.rin_idx.get(name).unwrap_or_else(|| panic!("{name} not staged")) }
    pub fn jack_poses(&self, m: &Motion) -> Vec<LocalPose> {
        let mut p = if m.pose.is_empty() { sample_retargeted(&self.jack.bones, &self.jack.clips[m.clip], m.frame, Retarget::NATIVE) } else { m.pose.clone() };
        strip_root_xz(&self.jack.bones, &mut p); p
    }
    pub fn rin_poses(&self, m: &Motion) -> Vec<LocalPose> {
        let mut p = if m.pose.is_empty() { sample_retargeted(&self.rin.bones, &self.rin.clips[m.clip], m.frame, Retarget::NATIVE) } else { m.pose.clone() };
        strip_root_xz(&self.rin.bones, &mut p); p
    }
    /// Native 80078CD0 clears the root caches when setting a motion; 80079ED0 filters the
    /// sampled root toward its target with weight 1 / (1 + remaining blend frames).
    fn prime_root(bones: &[Bone], clip: &Clip, m: &mut Motion) {
        if m.root_primed { return; }
        let initial = crate::character::root_xz(bones, clip, 0.);
        let weight = 1. / (1. + m.blend.map(|b| b.3).unwrap_or(0.));
        m.root_initial = initial.map(|v| v * weight);
        m.root_value = m.root_initial;
        m.root_primed = true;
    }
    fn update_pose(bones: &[Bone], clip: &Clip, m: &mut Motion) {
        let remaining = m.blend.map(|b| (b.3 - b.2).max(0.)).unwrap_or(0.);
        crate::character::filter_native_pose(bones, clip, m.frame, remaining, &mut m.pose);
    }
    fn update_enemy_pose(bones: &[Bone], clip: &Clip, m: &mut Motion) {
        let(frame,remaining)=m.pose_sample.unwrap_or((m.frame,m.blend.map(|b|(b.3-b.2).max(0.)).unwrap_or(0.)));
        crate::character::filter_native_pose(bones,clip,frame,remaining,&mut m.pose);
    }
    fn blend_root(bones: &[Bone], clip: &Clip, m: &mut Motion, from: f32) -> [f32; 2] {
        Self::prime_root(bones, clip, m);
        if m.frame < from && m.looping {
            let travel = Self::root_step(bones, clip, 0., (clip.frames - 1) as f32, false);
            for i in 0..2 { m.root_value[i] -= travel[i]; }
        }
        let target = crate::character::root_xz(bones, clip, m.frame);
        let remaining = m.blend.map(|b| (b.3 - b.2).max(0.)).unwrap_or(0.);
        let weight = 1. / (1. + remaining);
        let next = [0, 1].map(|i| m.root_value[i] + (target[i] - m.root_value[i]) * weight);
        let step = [next[0] - m.root_value[0], next[1] - m.root_value[1]];
        m.root_value = next;
        step
    }

    fn root_step(bones: &[Bone], clip: &Clip, from: f32, to: f32, looping: bool) -> [f32; 2] {
        let last = (clip.frames.max(2) - 1) as f32;
        let at = |f: f32| crate::character::root_xz(bones, clip, f);
        if to >= from { let (a, b) = (at(from), at(to)); [b[0] - a[0], b[1] - a[1]] }
        else if looping { let (a, b, c, e) = (at(from), at(last), at(0.), at(to)); [b[0] - a[0] + e[0] - c[0], b[1] - a[1] + e[1] - c[1]] }
        else { [0., 0.] }
    }
}

/// Things that happened this frame (for logs, audio and effects).
#[derive(Clone, Debug, PartialEq)]
pub enum FightEvent {
    PlayerAttack { atk: u32 }, RinHit { atk: u32, damage: i32, hp: i16, reaction: u8 }, RinGuard { atk: u32 }, RinEvade { atk: u32 },
    PlayerHit { atk: u32, damage: i32, hp: i16 }, RinState { from: u8, to: u8 }, PlayerDead, RinFinishReady,
    FinishStart, FinishMeterFull, Victory, Continue, ClashStart, ClashWon, ClashLost, GiantSwing { damage: i32 },
    /// Announcer voice id (jkSound::PlayVoiceStream 8026B558) requested by the player code.
    Voice(u16),
    /// RinRin swatted a pistol round off a fan (added: the pistol is not native) at this world point.
    RinDeflect { at: [f32; 3] },
    /// A bomb went off here (cBl0063 explode: EST 0 of bl0063.eff).
    BombBlast { at: [f32; 3] },
    /// Kreely's suplex (added): the grab, and the slam at this point.
    SuplexStart { on_rin: bool }, SuplexSlam { at: [f32; 3], on_rin: bool },
    /// The native grunt grab: caught, let go (A released), thrown (`side`: WHANG_SIDE_THROW), head-butted (its hit).
    CarryStart, CarryRelease, CarryThrow { side: bool }, CarryHeadbutt,
    /// The object pick-up (added events for the presenter, the native player states 0x1A..0x23): reaching for it, in
    /// hand, thrown (let go now), set down, slammed over a grunt's head.
    PickStart, PickTake, ObjThrow, ObjDrop, ObjCover,
    /// Added: Kreely turned his carry into a suplex on the grunt he held.
    SuplexFromHold,
    /// Added: the last-life execution's killing cut (RinRin's 0113 fan, its second slash cue) and its end.
    ExecutionCut, ExecutionDone,
}

/// Where a pistol round ended (added; `Fight::player_shot`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shot { Miss, Hit([f32; 3]), Deflect([f32; 3]) }

/// Chance (in 100) that RinRin, facing the shooter and free to react, bats a round away with a fan on top of her native
/// guard roll [inferred: the pistol is not native].
const FAN_DEFLECT: u32 = 30;
/// Added: Kreely's walking speed with the pistol out, metres per game frame (2.4 m/s; Jack's walk is ~1.9) [added].
pub const GUN_WALK: f32 = 0.08;
/// Run-and-gun pace with the pistol out (added; Jack's run is about 0.17 m a frame).
pub const GUN_RUN: f32 = 0.165;

pub struct Fight {
    proxy_clock: crate::collision::ProxyClock,
    contacts: [crate::collision::ProxyContacts; 2],
    rin_proxy_hold: Option<[f32; 3]>, rin_last_step: [f32; 2],
    pub player: PlayerFight, pub rin: Rin, pub rng: Rng, pub tick: u64, pub hitstop: i32,
    pub events: Vec<FightEvent>,
    /// Added (mw-play's street stage): the player without RinRin. Her update, fans and body push are skipped and she
    /// waits out of the world; the player's moves still resolve, so `player.last_hit` serves other targets.
    pub solo: bool,
    /// Added: the player's attack sphere this frame, as `resolve_hits` used it (single-frame triggers included).
    strike: Option<(AttackHit, u32)>,
    /// Paired finisher motions: both actors share Rin's origin and keep their MOT root translation.
    pub paired: bool, pub victory: bool,
    /// Added: Kreely's moveset is on (mw-play sets it while he plays) and his suplex in progress.
    pub kreely: bool, pub suplex: Option<Suplex>,
    /// The native grunt grab in progress.
    pub carry: Option<Carry>,
    /// Inspection (`debug_passive`): RinRin starts no attack.
    passive: bool,
    /// Finisher meter: +0x1248 (a), +0x1160 (p), phase-14 wait/timer/latch.
    pub meter: (f32, f32), fin_wait: i32, fin_timer: i32, fin_set: bool,
    /// Clash (keen competition) meter `+0x1240` and its timeout.
    /// Keen competition (802956B4): raw meter +0x1240, smoothed +0x1248 / previous +0x27DC, cooldown +0x1230, timer
    /// +0x123C, end counter +0x124C, directional delay +0x1238.
    /// Added: after a knockdown he stays down (holding the fall's last frame) until a press gets him up (a boss's floor
    /// game: Mathilda); `down_for` counts the frames he has lain there.
    pub hold_down: bool, pub down_for: u32, up_pressed: bool,
    pub clash: f32, clash_timer: i32, pub clash_s: f32, clash_prev: f32, clash_cd: i32, clash_count: f32, clash_delay: i32,
    /// Added (the actor editor, mw-play play_editor.rs): her AI holds in WAIT; the clip EDITOR plays (looped or not);
    /// the clash's outcome forced (true: the meter fills and the swing lands; false: no input); her next hit lands.
    pub rin_hold: bool, rin_editor: Option<(usize, bool)>, pub keen_auto: Option<bool>, force_hit: bool, pub editing: bool,
    /// events raised between steps (by the actor editor's calls), sent out with the next step's
    pub pending: Vec<FightEvent>,
}

impl Fight {
    pub fn physics_seconds(&self) -> f32 { self.proxy_clock.seconds() }
    pub fn new(d: &FightData, player_pos: [f32; 3], player_yaw: f32, rin_pos: [f32; 3], seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let cd_slalom = rng.rangef(d.combat.tune(27), d.combat.tune(28));
        let sc_timer = d.combat.tune(23);
        let rin = Rin { pos: rin_pos, facing: yaw_to(rin_pos, player_pos), motion: Motion::new(d.rin_clip("em0440_0000.mot"), true),
                        hp: RIN_MAX_HP, state: rs::WAIT, phase: 0, selector: 0, cd_atk: 0., cd_slalom, last_atk: 0, hit_on: false, rig0: false, root_lag: [0.; 2], fan_hits: vec![],
                        hit_bone: 9, guard_timer: 0., hit_by: 0, layer: None,
                        fans: [Fan { part: 100, hand: 9, ..Default::default() }, Fan { part: 115, hand: 14, ..Default::default() }],
                        on_scaffold: false, sc_cur: 0, sc_next: 0, sc_timer: 0., sc_time: 0., sc_start: [0.; 3], sc_heading: false,
                        bombs_done: false, bombs: vec![], down: false, down_timer: 0., thrown: None, vr_held: 0 };
        let mut rin = rin; rin.sc_timer = sc_timer;
        Fight { proxy_clock: Default::default(), contacts: Default::default(), rin_proxy_hold: None, rin_last_step: [0.; 2], player: PlayerFight::new(player_pos, player_yaw, d.jack_clip(crate::player::IDLE)), rin, rng, tick: 0, hitstop: 0, events: vec![], solo: false, strike: None, kreely: false, suplex: None, carry: None, passive: false,
                paired: false, victory: false, meter: (0., 0.), fin_wait: 0, fin_timer: 0, fin_set: false, hold_down: false, down_for: 0, up_pressed: false, clash: 0., clash_timer: 0, clash_s: 0., clash_prev: 0., clash_cd: 0, clash_count: 0., clash_delay: 0,
                rin_hold: false, rin_editor: None, keen_auto: None, force_hit: false, editing: false, pending: vec![] }
    }

    /// One native game frame. `floor(x, y, z)` returns the ground height; movement deltas are returned for the
    /// caller's wall sliding: (player step, rin step) in world XZ.
    pub fn step(&mut self, d: &FightData, inp: &FightInput) -> ([f32; 2], [f32; 2]) {
        // (events raised between steps, the actor editor's rin_* commands, go out with this step's)
        self.events.clear();
        self.events.append(&mut self.pending);
        self.step_inner(d, inp)
    }
    fn step_inner(&mut self, d: &FightData, inp: &FightInput) -> ([f32; 2], [f32; 2]) {
        self.tick += 1;
        self.strike = None;
        if self.hitstop > 0 { self.hitstop -= 1; return ([0.; 2], [0.; 2]); }
        let damage_pose = bone_frames(&d.jack.bones, &d.jack_poses(&self.player.motion));
        let damage_spheres: Vec<_> = PLAYER_DAMAGE_PARTS.iter().filter_map(|&(part, radius)|
            damage_pose.get(part).map(|f| (to_world(self.player.pos, self.player.loco.facing, f.1), radius))).collect();
        // The actor consumes the preceding proxy result, just like its delayed root step.
        let proxy_seconds = self.proxy_clock.seconds();
        let proxy_advances = crate::collision::proxy_advances(proxy_seconds);
        let motion_rate = self.proxy_clock.animation_rate(inp.demo_active);
        self.player.motion.clock_rate = motion_rate;
        self.rin.motion.clock_rate = motion_rate;
        self.proxy_clock.advance(inp.demo_active);
        if !proxy_advances && !self.rin.on_scaffold && !self.paired {
            let anchor = [self.rin.pos[0] - self.rin_last_step[0], self.rin.pos[1], self.rin.pos[2] - self.rin_last_step[1]];
            self.rin.pos = *self.rin_proxy_hold.get_or_insert(anchor);
        } else { self.rin_proxy_hold = None; }
        self.player.kreely = self.kreely;
        if self.suplex.is_some() { self.step_suplex(d); }
        // a hit (or anything else) that takes him out of the grab states ends the hold
        if self.carry.is_some() && !matches!(self.player.state(), 20 | 22 | 23 | 40 | 26 | 27 | 29 | 30 | 35) { self.carry = None; }
        let pstarts = self.player.motion.starts;
        let pnow = self.step_player(d, inp);
        FightData::update_pose(&d.jack.bones, &d.jack.clips[self.player.motion.clip], &mut self.player.motion);
        // the player's root step also lands one frame late (frame recording: 1011 starts moving on its frame 2)
        let mut pstep = if self.paired || self.player.airborne { self.player.root_lag = [0.; 2]; pnow } else {
            let (s, c) = self.player.loco.facing.sin_cos();
            let local = [c * pnow[0] - s * pnow[1], s * pnow[0] + c * pnow[1]];
            let prev = std::mem::replace(&mut self.player.root_lag, local);
            let prev = if self.player.motion.starts != pstarts { self.player.motion.root_initial } else { prev };
            [c * prev[0] + s * prev[1], -s * prev[0] + c * prev[1]]
        };
        // Rin's root step lands one frame late: the original moves her by the previous update's motion delta
        // (frame recording, 0102 wind-up: its path trails the same-frame delta by one frame) [observed]
        let rstarts = self.rin.motion.starts;
        // 810658C8 -> 8020FDEC reads the target's world transform, after
        // its pending translation, rather than the actor-update position.
        // The native recorder's world transform equals the next actor row
        // throughout the 1982-row fight-target-v1 capture.
        let player_world = if proxy_advances {
            // Actor snapshots lag the published part transform by its newly
            // evaluated root delta as well as the translation committed now.
            let world_step=[pstep[0]+pnow[0],pstep[1]+pnow[1]];
            if let Some(world)=&d.world {
                // Predict the published proxy position without consuming its
                // retained contacts twice; move_world commits the real result.
                let mut contacts=self.contacts[0].clone();
                crate::collision::move_on_floor_contacts(world,self.player.pos,world_step,self.player.airborne,proxy_seconds,&mut contacts).0
            } else {[self.player.pos[0] + world_step[0], self.player.pos[1], self.player.pos[2] + world_step[1]]}
        } else { self.player.pos };
        let rnow = if self.solo { [0.; 2] } else { self.step_rin(d, player_world) };
        // cByMotion evaluates the pose before advancing its reported clock.
        // Retain that sample and blend age even when a phase starts a motion.
        FightData::update_enemy_pose(&d.rin.bones, &d.rin.clips[self.rin.motion.clip], &mut self.rin.motion);
        // stored in Rin's local frame and applied along her current facing
        let (s, c) = self.rin.facing.sin_cos();
        let local = [c * rnow[0] - s * rnow[1], s * rnow[0] + c * rnow[1]];
        let prev = std::mem::replace(&mut self.rin.root_lag, local);
        let prev = if self.rin.motion.starts != rstarts { self.rin.motion.root_initial } else { prev };
        let mut rstep = [c * prev[0] + s * prev[1], -s * prev[0] + c * prev[1]];
        let rin_before_step = self.rin.pos;
        self.rin.pos[0] += rstep[0]; self.rin.pos[2] += rstep[1];
        if !self.solo { self.move_fans(d, &damage_spheres); }
        self.rin.pos = rin_before_step;
        self.resolve_hits(d);
        // 80276E04 / 802786C8 subtract +1150 (GameVSyncF * scene
        // animation rate), rather than two unscaled units during hit slow.
        if self.player.state() != 94 { self.player.damage_guard = (self.player.damage_guard - 2. * motion_rate).max(0.); }
        // body separation (onHitChk keeps actors apart; distance from the bodies' radii [inferred])
        if !self.paired && self.rin.state != rs::DEAD && !self.solo && self.suplex.is_none() {
            let nx = [self.player.pos[0] + pstep[0] - self.rin.pos[0] - rstep[0], self.player.pos[2] + pstep[1] - self.rin.pos[2] - rstep[1]];
            let dist = (nx[0] * nx[0] + nx[1] * nx[1]).sqrt();
            if dist < BODY_GAP {
                let dir = if dist > 1e-4 { [nx[0] / dist, nx[1] / dist] } else { [self.player.loco.facing.sin() * -1., self.player.loco.facing.cos() * -1.] };
                let push = (BODY_GAP - dist) * 0.5;
                pstep[0] += dir[0] * push; pstep[1] += dir[1] * push; rstep[0] -= dir[0] * push; rstep[1] -= dir[1] * push;
            }
        }
        self.rin_last_step = rstep;
        (if proxy_advances { pstep } else { [0.; 2] }, rstep)
    }

    /// `step` plus applying the movement directly (no collision): headless runs and tests.
    /// One frame with mw-play's movement against `d.world` (falls back to `step_free` without collision).
    pub fn step_world(&mut self, d: &FightData, inp: &FightInput) {
        let Some(world) = d.world.as_ref() else { return self.step_free(d, inp); };
        let seconds = self.physics_seconds();
        let (p, r) = self.step(d, inp);
        self.move_world(world, p, r, seconds);
    }

    /// Shared by the playable port and replay, including retained proxy contacts.
    pub fn move_world(&mut self, world: &mw_data::collision::CollisionWorld, p: [f32;2], r: [f32;2], seconds: f32) -> u32 {
        let airborne = self.player.airborne;
        // a boss pair placed by the presenter (Elise's clash, the Baron's): no wall push, the pair keeps its layout (the
        // live recomp's Elise clash at a far crypt: he stays at x 13.0 by the wall)
        let (pos, mut hits) = if self.paired && self.player.state() == STATE_EXTERNAL {
            ([self.player.pos[0] + p[0], self.player.pos[1], self.player.pos[2] + p[1]], 0)
        } else { crate::collision::move_on_floor_contacts(world, self.player.pos, p, airborne, seconds, &mut self.contacts[0]) };
        self.player.pos = pos;
        if !self.rin.on_scaffold && !self.paired && !self.solo {
            let (pos, count) = crate::collision::move_on_floor_contacts(world, self.rin.pos, r, false, seconds, &mut self.contacts[1]);
            self.rin.pos = pos; hits += count;
        }
        hits
    }

    pub fn step_free(&mut self, d: &FightData, inp: &FightInput) {
        let (p, r) = self.step(d, inp);
        self.player.pos[0] += p[0]; self.player.pos[2] += p[1];
        if !self.rin.on_scaffold && !self.paired { self.rin.pos[0] += r[0]; self.rin.pos[2] += r[1]; }
    }

    fn step_player(&mut self, d: &FightData, inp: &FightInput) -> [f32; 2] {
        let p = &mut self.player;
        // fn_8027621C: input buffers
        // each buffer counts down one per frame (fn_8027621C)
        if inp.a_trig { p.buf_a = 8; p.press_stick = inp.stick; } else if p.buf_a > 0 { p.buf_a -= 1; }
        if inp.b_trig { p.buf_b = 8; } else if p.buf_b > 0 { p.buf_b -= 1; }
        if inp.jump { p.buf_jump = 8; } else if p.buf_jump > 0 { p.buf_jump -= 1; }
        let state0 = p.loco.state;
        if p.invuln > 0. { p.invuln -= 1.; }
        if p.stun > 0 { p.stun -= 1; }
        p.loco.set_saw(inp.b_held);
        // 8027621C: HP below 0.3 x max -> announcer 0x1018 every frame (its 0x1518 cooldown limits it; the
        // 8006A66C(10) gate is taken as clear [inferred])
        if p.hp > 0 && (p.hp as f32) < 0.3 * PLAYER_MAX_HP as f32 { self.events.push(FightEvent::Voice(0x1018)); }
        // 8028BCB0: a swing next to a finish-ready boss starts the paired finisher (boss vtbl 0x160 -> startCatch 0x2F)
        let near = dist2_xz(p.pos, self.rin.pos) < 3.0 * 3.0; // target search range [inferred]
        // the finisher waits until she is down: 810D061C raises condition flags 0 and 3 only when 1007 ends [inferred gate]
        if inp.swing != 0 && self.rin.state == rs::FINISH_WAIT && self.rin.motion.ended && near && matches!(state0, 0 | 1 | 2 | 4 | 39) {
            p.loco.enter_combat(68); p.loco.phase = 10;
            self.rin.state = rs::FINISH; self.rin.phase = 0;
            self.paired = true; self.meter = (0., 0.);
            self.events.push(FightEvent::FinishStart);
            return [0.; 2];
        }
        // clash (8028BCB0, called by the swing check 802C3C0C and the attack states before the swing itself): the target
        // is the lock target, its chkEnableKeenCompetition (vtbl 0x1D0: rig flag 0) holds, |dy| < 0.15 (not em0460),
        // no competition yet and |remote accel x| or |y| > 2.0 (the swing gesture) -> reqStartKeenCompetition (vtbl 0x160)
        // -> startKeenCompetition(player, 0xB). No distance test. +0x11A5 == 0, +0x396 != 0 and 80085894 on the
        // player position are taken as passing [inferred].
        if inp.swing != 0 && self.rin.rig0 && (self.rin.pos[1] - p.pos[1]).abs() < 0.15 && matches!(state0, 0 | 1 | 2 | 4 | 39) {
            p.loco.enter_combat(78); p.loco.phase = 1;
            self.rin.state = rs::KEEN_COMPE; self.rin.phase = 0; self.rin.rig0 = false;
            // phase 1 end: +0x1240 = +0x1248 = 0.75 (8056A084); phase 2: cooldown 0x14, timer 0xF0 [+0x27DC start inferred]
            self.paired = true; self.clash = 0.75; self.clash_s = 0.75; self.clash_prev = 0.75;
            self.clash_cd = 0x14; self.clash_timer = 0xF0; self.clash_count = 0.;
            self.events.push(FightEvent::ClashStart);
            return [0.; 2];
        }
        if inp.a_held && p.buf_a == 0 { p.a_hold += 1; } else if !inp.a_held { p.a_hold = 0; }
        if p.a_hold == 0 { p.grab_latch = false; }
        // the native grab of a street grunt (802C5DD8 hold -> 802C6020 -> state 0x14 grab, pl0030_0220), one per A hold
        // the object pick-up (state 0x1A, pl0030_021a): the same A hold when no grunt is in reach and an object is
        if !inp.carry_ok && inp.pick_ok && matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat && p.a_hold > 0 && !p.grab_latch && !p.airborne {
            p.grab_latch = true;
            p.loco.enter_combat(26); p.loco.phase = 0;
            p.hit_window = false; p.hit_trigger = false;
            p.motion.play(d.jack_clip("pl0030_021a.mot"), crate::player::BLEND_DEFAULT, false, true);
            self.carry = Some(Carry { object: true, ..Default::default() });
            self.events.push(FightEvent::PickStart);
            return [0.; 2];
        }
        if inp.carry_ok && matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat && p.a_hold > 0 && !p.grab_latch && !p.airborne {
            p.grab_latch = true;
            p.loco.enter_combat(20); p.loco.phase = 0;
            p.hit_window = false; p.hit_trigger = false;
            p.motion.play(d.jack_clip("pl0030_0220.mot"), crate::player::BLEND_DEFAULT, false, true);
            self.carry = Some(Carry::default());
            self.events.push(FightEvent::CarryStart);
            return [0.; 2];
        }
        // the taunt (fn_802C68AC, from idle / walk / run): state 0x54 plays pl0030_00b0 and provokes the grunts (no hit)
        if inp.taunt && matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat && !p.airborne {
            p.loco.enter_combat(84); p.loco.phase = 0;
            p.hit_window = false; p.hit_trigger = false;
            p.motion.play(d.jack_clip("pl0030_00b0.mot"), 2., false, true);
            return [0.; 2];
        }
        // Kreely's grab -> suplex: F, or A held like the native grab (802C5DD8, once per hold) when RinRin is not down
        // (down she still takes the giant swing below); with nobody in reach the hold falls through to the native whiff
        let hold_grab = p.kreely && matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat && p.a_hold > 0 && !p.grab_latch
            && !(self.rin.down && dist2_xz(p.pos, self.rin.pos) < 2.0 * 2.0);
        if p.kreely && (inp.grab || hold_grab) && !p.airborne && (matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat || state0 == 0xC8) {
            if let Some(kit) = d.kreely.as_ref() {
                let rin_ok = !self.solo && dist2_xz(p.pos, self.rin.pos) < SUPLEX_REACH * SUPLEX_REACH && !self.rin.on_scaffold && !self.rin.down
                    && (self.rin.pos[1] - p.pos[1]).abs() < 0.5
                    && matches!(self.rin.state, rs::WAIT | rs::RUN | rs::PROVOCATION | rs::GUARD | rs::DM_RIGHT | rs::DM_LEFT | rs::DM_BODY
                                | rs::RIGHT_STEP | rs::LEFT_STEP | rs::BACK_STEP | rs::URAKEN | rs::TETUZANKOU);
                // (a street grunt only with F: the A hold catches him natively, above)
                if rin_ok || (inp.grab_grunt && inp.grab) {
                    if hold_grab { p.grab_latch = true; }
                    let f = if rin_ok { yaw_to(p.pos, self.rin.pos) } else { p.loco.facing };
                    let (sn, cs) = f.sin_cos(); let o = kit.suplex.offset;
                    // the grab's reach carries him forward to its contact: the suplex starts from there
                    let grab_clip = d.jack_clip("pl0030_0226.mot");
                    let reach = crate::sample(&d.jack.bones, &d.jack.clips[grab_clip], GRAB_CONTACT)[d.jack.bones.len()].translation;
                    let origin = [p.pos[0] + cs * reach[0] + sn * reach[2], p.pos[1], p.pos[2] - sn * reach[0] + cs * reach[2]];
                    let target = [origin[0] + cs * o[0] + sn * o[2], origin[1], origin[2] - sn * o[0] + cs * o[2]];
                    let target_facing = f + kit.suplex.yaw;
                    p.loco.enter_combat(STATE_SUPLEX); p.loco.phase = 0; p.loco.facing = f;
                    p.hit_window = false; p.hit_trigger = false; p.invuln = 1000.;
                    p.motion.play(grab_clip, crate::player::BLEND_SHORT, false, true);
                    self.suplex = Some(Suplex { on_rin: rin_ok, origin, facing: f, target, target_facing, slammed: false,
                                                grab: Some(GRAB_CONTACT), rin_from: self.rin.pos });
                    if rin_ok {
                        let from = self.rin.state;
                        self.rin.state = RIN_SUPLEXED; self.rin.phase = 0; self.rin.hit_on = false; self.rin.rig0 = false;
                        self.rin.motion.play(d.rin_clip("gh_suplex_recv.mot"), 6., false, true);
                        self.events.push(FightEvent::RinState { from, to: RIN_SUPLEXED });
                    }
                    self.events.push(FightEvent::SuplexStart { on_rin: rin_ok });
                    return [0.; 2];
                }
            }
        }
        if matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat && p.a_hold > 0 && !p.grab_latch {
            // 802C5DD8: A held with the press buffer spent, once per hold (0x800 latch)
            p.grab_latch = true;
            if self.rin.down && dist2_xz(p.pos, self.rin.pos) < 2.0 * 2.0 {
                // 802C6020: grab; RinRin's catch state 6/0xC -> player 0x31 + startCatch(0xB) = dm_giantswing
                p.loco.enter_combat(49); p.loco.phase = 0; p.spin = 0.; p.spin_power = 0.;
                self.rin.down = false; self.rin.state = rs::DM_GIANTSWING; self.rin.phase = 0; self.paired = true;
            } else {
                // no catch started (+0x12D8 == 0): +0x14F0 = 2, state 0x15 = grab whiff (B 8027FDC4 plays table[2] = 0226)
                p.loco.enter_combat(21); p.loco.phase = 1;
                p.motion.play(d.jack_clip("pl0030_0226.mot"), crate::player::BLEND_DEFAULT, false, true);
            }
            return [0.; 2];
        }
        // Kreely (added): the pistol whip from the aim, and the grab -> suplex (RinRin in reach, or a street grunt the
        // presenter found)
        if p.kreely && inp.whip && state0 == 0xC8 && !p.airborne {
            p.whip = true; p.buf_a = 0; p.loco.enter_combat(39); p.loco.phase = 0;
        }
        // whips chain while the aim is held and the enemy is still in front (the attack's combo window)
        if p.kreely && inp.whip && state0 == 39 && p.loco.phase == 1 && p.combo_window { p.whip = true; }
        // 802C2610 (checked first in idle/walk/run and early state 39): nunchuk shake > 1.2 -> evade (state 7)
        let early39 = state0 == 39 && p.air_atk == 0 && p.loco.phase < 0x14 && !p.rushing;
        // PC: Kreely's pistol state (0xC8, added) dodges too; the presenter drops the aim when he leaves it
        if inp.nunchuk_shake > 1.2 && !p.airborne && (matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat || early39 || state0 == 0xC8) {
            // target attacking (vtbl 0x2FC) -> side step away from it (0031 / 0032), else 0033 [query simplified]
            let attacking = matches!(self.rin.state, rs::URAKEN | rs::TETUZANKOU | rs::SLALOM | rs::KAMIKAZE | rs::THROW);
            // PC: with a direction held the evade goes that way (relative to the camera: W forward, S back, A / D
            // the side steps): the clip whose root travels closest to it, the facing turned so it travels exactly
            // there [added; native picks from the attacker only]. Clip root directions (local yaw): 00ac forward
            // (1.91 m), 0032 his left (84 deg), 0031 his right (-88 deg), 0033 back (2.17 m).
            let held = inp.stick[0] * inp.stick[0] + inp.stick[1] * inp.stick[1] >= crate::player::STICK_MOVE;
            let m = if held {
                let want = inp.camera_yaw + (-inp.stick[0]).atan2(inp.stick[1]);
                let rel = crate::player::rad_lerp_range(0., want - inp.camera_yaw, 1., 10.);
                let wrap = |a: f32| (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                let (m, local) = [("pl0030_00ac.mot", 0f32), ("pl0030_0032.mot", 84f32.to_radians()), ("pl0030_0031.mot", -88f32.to_radians()),
                                  ("pl0030_0033.mot", std::f32::consts::PI)]
                    .into_iter().min_by(|a, b| wrap(rel - a.1).abs().total_cmp(&wrap(rel - b.1).abs())).unwrap();
                p.loco.facing = wrap(want - local);
                m
            } else if attacking {
                let to_rin = yaw_to(p.pos, self.rin.pos) - p.loco.facing;
                if to_rin.sin() >= 0. { "pl0030_0032.mot" } else { "pl0030_0031.mot" }
            } else { "pl0030_0033.mot" };
            p.loco.enter_combat(7); p.loco.phase = 1; p.hit_window = false; p.hit_trigger = false;
            p.motion.play(d.jack_clip(m), 2., false, true);
            p.invuln = (d.jack.clips[p.motion.clip].frames as f32).min(30.);   // [inferred i-frames]
            return [0.; 2];
        }
        // 802C2E68: jump from the ground -> 11 (moving, stick >= 0.2) or 14 (standing)
        if matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat && p.buf_jump > 0 && !p.airborne {
            let moving = inp.stick[0] * inp.stick[0] + inp.stick[1] * inp.stick[1] >= crate::player::STICK_MOVE;
            p.buf_jump = 0; p.loco.enter_combat(if moving { 11 } else { 14 }); p.loco.phase = 0;
        }
        // A halves: attack entry is checked first in every locomotion state (802C3B5C before walk/run checks)
        if matches!(state0, 0 | 1 | 2 | 4) && !p.loco.combat { p.check_attack(inp, false); }
        if p.loco.state == 39 && p.loco.combat {
            if inp.a_trig { p.presses += 1; }
            if p.loco.phase == 1 && p.combo_window && p.check_attack(inp, true) { p.loco.phase = 0; }
        }
        // B halves
        let mut out = [0.; 2];
        let (from, starts) = (p.motion.frame, p.motion.starts);
        match p.loco.state {
            39 if p.air_atk == 0 => {
                if p.rush_timer > 0 { p.rush_timer -= 1; }
                if p.loco.phase == 0 { p.start_attack(d, inp, self.rin.hp); self.events.push(FightEvent::PlayerAttack { atk: p.atk }); }
                let clip = &d.jack.clips[p.motion.clip];
                let cues = d.combat.cues_for(&clip.name);
                for c in p.motion.advance(clip.frames, cues) {
                    match c.op {
                        1 => p.motion.rate = c.f,
                        7 => { p.hit_trigger = true; p.hit_serial += 1; if c.a != 0 { p.part = c.a as usize; } if c.b != 0 { p.power = c.b as i32; } }
                        0xB => p.combo_window = true,
                        0x13 => { p.hit_window = true; p.hit_serial += 1; if c.a != 0 { p.part = c.a as usize; } if c.b != 0 { p.power = c.b as i32; } }
                        0x14 => p.hit_window = false,
                        _ => {}
                    }
                }
                // lunge toward Rin (802C72B4 with +0x12C4)
                let dr = dist2(self.rin.pos, p.pos).sqrt();
                if dr < 2. && !(2..=4).contains(&p.combo) {
                    let target = yaw_to(p.pos, self.rin.pos);
                    p.loco.facing = rad_lerp_range(p.loco.facing, target, crate::player::TURN_FRAC, crate::player::TURN_CLAMP);
                    if p.motion.frame <= 1. && !matches!(p.atk, 0x75 | 0x77 | 0x78) && dr > 1.6 { p.lunge = 0.2 * (dr - 1.6); }
                } else if p.motion.frame <= 1. { p.lunge = 0.; }
                out[0] += p.loco.facing.sin() * p.lunge; out[1] += p.loco.facing.cos() * p.lunge;
                p.lunge *= 0.6;
                if p.motion.ended && p.rushing && p.atk == 0x5E && p.presses >= 4 {
                    // phase 0x33: keep mashing to keep rushing [continue rule simplified]
                    p.presses = 0; p.motion.play(d.jack_clip("pl0030_1111.mot"), 0., false, true);
                } else if p.motion.ended { p.flags &= !PF_100; p.hit_window = false; p.rushing = false; p.loco.leave_combat(p.loco.facing); }
            }
            11 | 14 => {   // take-off (B 8027E54C / 8027EC50): 00A0 / 00A5, then airborne
                if p.loco.phase == 0 {
                    p.motion.play(d.jack_clip(if p.loco.state == 11 { "pl0030_00a0.mot" } else { "pl0030_00a5.mot" }), 2., false, true);
                    p.loco.phase = 1;
                }
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if p.motion.ended {
                    // B 8027E7EC / 8027EE54: vertical = jump power +0x118C (0.151) x GameVSyncF (x1.05 standing),
                    // horizontal 0.16 along the facing for the moving jump
                    let standing = p.loco.state == 14;
                    let v = JUMP_POWER * crate::player::VSYNC * if standing { 1.05 } else { 1. };
                    let h = if standing { 0. } else { 0.16 };
                    p.vel = [p.loco.facing.sin() * h, v, p.loco.facing.cos() * h];
                    p.airborne = true; p.loco.state = if standing { 15 } else { 12 }; p.loco.phase = 1;
                    p.motion.play(d.jack_clip(if standing { "pl0030_00a6.mot" } else { "pl0030_00a1.mot" }), 0., true, true);
                }
            }
            12 | 15 if inp.a_trig => {
                // 802C5C88: A in the air -> state 39, air flag; phase 0 resets the velocity to a 0.1 hop -> phase 0x14
                p.loco.state = 39; p.loco.phase = 1; p.air_atk = 1; p.vel = [0., 0.1, 0.]; p.buf_a = 0; p.buf_b = 0;
                p.atk = 0x7E; p.part = 9; p.power = 15; p.hit_trigger = false; p.hit_window = false;
                p.motion.play(d.jack_clip("pl0030_1122.mot"), crate::player::BLEND_DEFAULT, false, true);
                self.events.push(FightEvent::PlayerAttack { atk: 0x7E });
            }
            39 if p.air_atk != 0 => {
                let clip = &d.jack.clips[p.motion.clip];
                let cues = d.combat.cues_for(&clip.name);
                for c in p.motion.advance(clip.frames, cues) {
                    match c.op {
                        7 => { p.hit_trigger = true; p.hit_serial += 1; if c.a != 0 { p.part = c.a as usize; } }
                        0x13 => { p.hit_window = true; p.hit_serial += 1; if c.a != 0 { p.part = c.a as usize; } }
                        0x14 => p.hit_window = false,
                        _ => {}
                    }
                }
                if p.airborne {
                    p.pos[1] += p.vel[1]; p.vel[1] -= GRAVITY * crate::player::VSYNC;
                    if p.vel[1] < 0. && p.pos[1] <= p.floor_y { p.pos[1] = p.floor_y; p.airborne = false; }
                }
                // phase 0x15: motion ended on the ground -> phase 0x16 landing slam 1123 (PUNCH_JUMPATK1)
                if p.air_atk == 1 && p.motion.ended && !p.airborne {
                    p.air_atk = 2; p.atk = 0x7F; p.part = 9; p.hit_window = false;
                    p.motion.play(d.jack_clip("pl0030_1123.mot"), 0., false, true);
                    self.events.push(FightEvent::PlayerAttack { atk: 0x7F });
                } else if p.air_atk == 2 && p.motion.ended {
                    p.air_atk = 0; p.hit_window = false; p.loco.leave_combat(p.loco.facing);
                }
            }
            12 | 15 => {   // airborne: gravity +0x1188 (0.0145) x GameVSyncF per frame [integration inferred; apex ~1.57 m vs 1.48 observed]
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                p.pos[1] += p.vel[1]; p.vel[1] -= GRAVITY * crate::player::VSYNC;
                out[0] += p.vel[0]; out[1] += p.vel[2];
                if p.vel[1] < 0. && p.pos[1] <= p.floor_y {
                    p.pos[1] = p.floor_y; p.airborne = false; p.loco.state = 13; p.loco.phase = 1;
                    p.motion.play(d.jack_clip("pl0030_00a2.mot"), 0., false, true);
                }
            }
            7 => {   // evade (B 8027DDA8): play out the motion, then idle
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if p.motion.ended { p.loco.leave_combat(p.loco.facing); }
            }
            13 => {   // landing (B 8027EAB8): 00A2, then run / idle
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if p.motion.ended { p.loco.leave_combat(p.loco.facing); }
            }
            49 => {   // giant swing (B 802AF28C)
                let wrap = |a: f32| (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                match p.loco.phase {
                    // 0-1: grab 022C (the target plays its paired motion via vtbl 0x94), wait for its end
                    0 => { p.motion.play(d.jack_clip("pl0030_022c.mot"), 2., false, true); self.rin.motion.play(d.rin_clip("em0110_1050.mot"), 2., false, true); p.loco.phase = 1; }
                    1 => { if p.motion.ended { p.loco.phase = 2; } }
                    // 2: 023E at rate 0.5, power +0x1160 = 0.4, peak shake +0xF70 = 0, timer 300
                    2 => {
                        p.motion.play(d.jack_clip("pl0030_023e.mot"), 0., true, true); p.motion.rate = 0.5;
                        self.rin.motion.play(d.rin_clip("em0110_1051.mot"), 0., true, true);
                        p.spin_power = 0.4; p.spin = 0.; p.swing_timer = 300; p.loco.phase = 3;
                    }
                    // 3: rate += 0.01 (<= 1), peak = max(peak, shake), power = max(power, 0.2 shake); 022A when power >= 0.5;
                    //    facing += 0.15 + 0.1 peak; release on timeout or A released -> 4 with that angular speed
                    3 => {
                        p.motion.rate = (p.motion.rate + 0.01).min(1.);
                        p.spin = p.spin.max(inp.shake);
                        p.spin_power = p.spin_power.max(0.2 * inp.shake);
                        let fast = p.spin_power >= 0.5;
                        let want = d.jack_clip(if fast { "pl0030_022a.mot" } else { "pl0030_023e.mot" });
                        if p.motion.clip != want { p.motion.play(want, 15., true, true); }
                        self.rin.motion.rate = p.motion.rate;
                        p.loco.facing = wrap(p.loco.facing + 0.15 + 0.1 * p.spin);
                        // announcer 0x100A every spin frame while +0x11AC is set (taken as always set [inferred]);
                        // PlayVoiceStream's same-id check and the per-id cooldown keep it to one line
                        self.events.push(FightEvent::Voice(0x100A));
                        p.swing_timer -= 1;
                        if p.swing_timer <= 0 || !inp.a_held { p.loco.phase = 4; }
                    }
                    // 4: keep turning until facing the camera direction (+0x1134), then 5
                    4 => {
                        let w = 0.15 + 0.1 * p.spin;
                        p.loco.facing = wrap(p.loco.facing + w);
                        if wrap(inp.camera_yaw - p.loco.facing).abs() <= (0.008_726_646 + w).abs() { p.loco.facing = inp.camera_yaw; p.loco.phase = 5; self.events.push(FightEvent::Voice(0x100B)); }
                    }
                    // 5: throw 023F, release the target (vtbl 0x9C): RinRin flies along the facing
                    5 => {
                        p.motion.play(d.jack_clip("pl0030_023f.mot"), 2., false, true); p.motion.rate = 1.;
                        self.rin.motion.play(d.rin_clip("em0110_1052.mot"), 2., false, true); self.rin.motion.rate = 1.;
                        self.paired = false;
                        // dm_giantswing release velocity: facing x (0.1 + 0.7 k), up 0.05 + k [k = power, scale inferred]
                        let k = p.spin_power.clamp(0., 1.);
                        let (sn, cs) = p.loco.facing.sin_cos();
                        self.rin.thrown = Some([sn * (0.1 + 0.7 * k), 0.05 + k * 0.1, cs * (0.1 + 0.7 * k)]);
                        // GIANTSWING (0x55, 300; the plem_common.ini note: a weak swing does 50% [weak = power < 0.5 inferred])
                        let (power, _) = d.combat.player_attack(0x55);
                        let dmg = if p.spin_power < 0.5 { power / 2 } else { power };
                        self.rin.hp = (self.rin.hp as i32 - dmg).max(1) as i16;
                        self.events.push(FightEvent::GiantSwing { damage: dmg });
                        p.loco.phase = 6;
                    }
                    _ => { if p.motion.ended { p.loco.leave_combat(p.loco.facing); } }
                }
                if p.loco.phase <= 4 { self.rin.facing = p.loco.facing; }
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if self.paired { p.pos = self.rin.pos; }
                return [0.; 2];
            }
            // the native grunt grab (research/GRAB-THROW-MINIMAP.md §1.3-1.5)
            20 => {   // grab start (B 8027F898): 0220, then the hold
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if inp.carry_lost { self.carry = None; p.loco.leave_combat(p.loco.facing); }
                else if p.motion.ended { p.loco.state = 22; p.motion.play(d.jack_clip("pl0030_0221.mot"), crate::player::BLEND_DEFAULT, true, true); }
            }
            22 => {   // hold / carry (80280044 / 80280244)
                if inp.carry_lost || !inp.a_held {
                    // A released (or he broke free): 0x17, pl0030_0222
                    if !inp.carry_lost { self.events.push(FightEvent::CarryRelease); }
                    p.loco.state = 23; p.motion.play(d.jack_clip("pl0030_0222.mot"), crate::player::BLEND_DEFAULT, false, true);
                } else if p.kreely && inp.suplex_held && d.kreely.is_some() {
                    // [added] Kreely: the grunt in his hands goes straight into his suplex (the grab's reach is already
                    // done: the suplex starts where he stands)
                    let kit = d.kreely.as_ref().unwrap();
                    let f = p.loco.facing; let (sn, cs) = f.sin_cos(); let o = kit.suplex.offset;
                    let origin = p.pos;
                    let target = [origin[0] + cs * o[0] + sn * o[2], origin[1], origin[2] - sn * o[0] + cs * o[2]];
                    p.loco.enter_combat(STATE_SUPLEX); p.loco.phase = 0; p.invuln = 1000.;
                    p.hit_window = false; p.hit_trigger = false;
                    p.motion.play(d.jack_clip("kreely_gh_suplex.mot"), 6., false, true);
                    let rin_from = self.rin.pos;
                    self.suplex = Some(Suplex { on_rin: false, origin, facing: f, target, target_facing: f + kit.suplex.yaw, slammed: false, grab: None, rin_from });
                    self.carry = None;
                    self.events.push(FightEvent::SuplexFromHold);
                    return [0.; 2];
                } else if inp.headbutt || inp.throw_v || inp.throw_s {
                    // 802C44A8 -> 0x28: the head-butt keeps the hold; a throw lets go at once
                    let (act, clip) = if inp.headbutt { (3, "pl0030_0223.mot") } else if inp.throw_v { (1, "pl0030_0224.mot") } else { (2, "pl0030_022b.mot") };
                    if act != 3 { self.events.push(FightEvent::CarryThrow { side: act == 2 }); }
                    self.carry = Some(Carry { act, hit: false, object: false });
                    p.loco.state = 40; p.motion.play(d.jack_clip(clip), crate::player::BLEND_DEFAULT, false, true);
                } else {
                    // stick X steers 4 deg a frame; stick Y walks him forward / back with the grunt (0227 / 0228)
                    p.loco.facing = crate::player::rad_lerp_range(p.loco.facing, p.loco.facing - CARRY_STEER * inp.stick[0], 0.3, 1.);
                    let clip = if inp.stick[1] > 0.25 { "pl0030_0227.mot" } else if inp.stick[1] < -0.25 { "pl0030_0228.mot" } else { "pl0030_0221.mot" };
                    let ci = d.jack_clip(clip);
                    if p.motion.clip != ci { p.motion.play(ci, crate::player::BLEND_DEFAULT, true, true); }
                    let clip = &d.jack.clips[p.motion.clip];
                    p.motion.advance(clip.frames, &[]);
                }
            }
            // ---- the object in hand ----
            26 => {   // pick up (0x1A): 021a, taken at frame 6, then the hold
                let clip = &d.jack.clips[p.motion.clip];
                let before = p.motion.frame;
                p.motion.advance(clip.frames, &[]);
                if before < PICK_TAKE && p.motion.frame >= PICK_TAKE { self.events.push(FightEvent::PickTake); }
                if p.motion.ended { p.loco.state = 27; p.motion.play(d.jack_clip("pl0030_021b.mot"), crate::player::BLEND_DEFAULT, true, true); }
            }
            27 => {   // hold / carry (0x1B): A released sets it down; a throw, or the slam over a near grunt's head
                if !inp.a_held {
                    p.loco.state = 30; p.motion.play(d.jack_clip("pl0030_021c.mot"), crate::player::BLEND_DEFAULT, false, true);
                } else if inp.throw_v || inp.throw_s {
                    let (st, clip) = if inp.cover_ok && inp.throw_v { (35, "pl0030_0218.mot") } else { (29, "pl0030_021d.mot") };
                    p.loco.state = st; p.motion.play(d.jack_clip(clip), crate::player::BLEND_DEFAULT, false, true);
                } else {
                    // steered as the grunt hold (4 deg a frame); stick Y walks with it (021e)
                    p.loco.facing = crate::player::rad_lerp_range(p.loco.facing, p.loco.facing - CARRY_STEER * inp.stick[0], 0.3, 1.);
                    let clip = if inp.stick[1].abs() > 0.25 { "pl0030_021e.mot" } else { "pl0030_021b.mot" };
                    let ci = d.jack_clip(clip);
                    if p.motion.clip != ci { p.motion.play(ci, crate::player::BLEND_DEFAULT, true, true); }
                    let clip = &d.jack.clips[p.motion.clip];
                    p.motion.advance(clip.frames, &[]);
                }
            }
            29 | 30 | 35 => {   // throw (0x1D, 021d), put down (0x1E, 021c), slam over a head (0x23, 0218)
                let clip = &d.jack.clips[p.motion.clip];
                let before = p.motion.frame;
                p.motion.advance(clip.frames, &[]);
                let (at, ev) = match p.loco.state { 29 => (OBJ_RELEASE, FightEvent::ObjThrow), 30 => (OBJ_DROP, FightEvent::ObjDrop), _ => (OBJ_COVER, FightEvent::ObjCover) };
                if before < at && p.motion.frame >= at { self.events.push(ev); }
                if p.motion.ended { self.carry = None; p.loco.leave_combat(p.loco.facing); }
            }
            23 => {   // release (802805A0): 0222, then idle
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if p.motion.ended { self.carry = None; p.loco.leave_combat(p.loco.facing); }
            }
            40 => {   // hold action (80287DF0): throw (released at once) or head-butt (hit at 12, back to the hold)
                let clip = &d.jack.clips[p.motion.clip];
                let before = p.motion.frame;
                p.motion.advance(clip.frames, &[]);
                let mut c = self.carry.unwrap_or_default();
                if c.act == 3 && !c.hit && before < HEADBUTT_HIT && p.motion.frame >= HEADBUTT_HIT { c.hit = true; self.events.push(FightEvent::CarryHeadbutt); }
                self.carry = Some(c);
                if p.motion.ended {
                    if c.act == 3 && !inp.carry_lost && inp.a_held {
                        self.carry = Some(Carry::default());
                        p.loco.state = 22; p.motion.play(d.jack_clip("pl0030_0221.mot"), crate::player::BLEND_DEFAULT, true, true);
                    } else { self.carry = None; p.loco.leave_combat(p.loco.facing); }
                }
            }
            21 => {   // grab whiff (B 8027FDC4): play out, then idle [the 0x10D whiff hit volume is not ported]
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if p.motion.ended { p.loco.leave_combat(p.loco.facing); }
            }
            84 => {   // the taunt (A 802be3c4 / B 802be44c): to idle at the clip end; from the op-0xB window (frame 25) an
                      // attack, jump or dodge cancels it
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if p.motion.ended || (p.motion.frame >= 25. && (inp.a_trig || inp.jump || inp.nunchuk_shake > 1.2)) { p.loco.leave_combat(p.loco.facing); }
            }
            85 => {
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                // B 802BE694: motion end -> 0x5B when the stun counter is above 0x1C2, else idle
                if p.motion.ended { if p.stun > 0x1C2 { p.loco.state = 91; p.loco.phase = 0; } else { p.loco.leave_combat(p.loco.facing); } }
            }
            91 => {   // dizzy (B 802C03EC): 1097 at blend 2; meter -= 2.5 x (remote + nunchuk shake); < 0 -> idle
                if p.loco.phase == 0 { Self::enter_dizzy(d, p); }
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                p.dizzy -= 2.5 * (inp.shake_remote + inp.shake_nunchuk);
                if p.dizzy < 0. { p.loco.leave_combat(p.loco.facing); }   // 802C6768 taken as passing [inferred]
            }
            92 => {   // knockdown: fall, then get up (state 94, B 802C0D4C)
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                // changeState(0x5E); the get-up's B phase 0 starts its motion on the next frame [observed]
                // (added hold_down: he lies there until attack, jump, dodge or the stick gets him up; a press during the
                // fall counts)
                if inp.a_trig || inp.jump || inp.nunchuk_shake > 1.2 || inp.stick[0] * inp.stick[0] + inp.stick[1] * inp.stick[1] > 0.25 { self.up_pressed = true; }
                if p.motion.ended {
                    if self.hold_down && !self.up_pressed { self.down_for += 1; p.motion.frame = (d.jack.clips[p.motion.clip].frames.max(2) - 1) as f32; }
                    else { self.hold_down = false; self.down_for = 0; self.up_pressed = false; p.loco.state = 94; p.loco.phase = 0; }
                }
            }
            94 => {
                // B 802C0D4C refreshes +0x115C to 6, including its exit frame.
                // Damage dispatch consumes the preceding frame's protection.
                p.damage_guard = 6.;
                if p.loco.phase == 0 {
                    // get-up (B 802C0D4C): 1090 after a 1011/1016 knockdown, else 1092 at >= 40% HP, 1091 below
                    let m = if p.knock_type == 1 { "pl0030_1090.mot" } else if p.hp as f32 / PLAYER_MAX_HP as f32 >= 0.4 { "pl0030_1092.mot" } else { "pl0030_1091.mot" };
                    p.loco.phase = 1; p.motion.play(d.jack_clip(m), crate::player::BLEND_DEFAULT, false, true);
                }
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                // B 802C0D4C: end -> 0x5B above the stun threshold, else idle
                if p.motion.ended { if p.stun > 0x1C2 { p.loco.state = 91; p.loco.phase = 0; } else { p.loco.leave_combat(p.loco.facing); } }
            }
            95 => {   // death (B 802C0F80): 108A, countdown 0x78 >> 1, then hold; continue = restart 802C1264
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                if p.loco.phase == 0 { p.loco.phase = 1; p.death_timer = 0x78 >> crate::player::INTERVAL_SHIFT; }
                if p.death_timer > 0 { p.death_timer -= 1; }
                if p.loco.phase == 1 && p.motion.ended { p.loco.phase = 2; }
                if p.loco.phase == 2 && p.death_timer <= 0 && inp.cont {
                    p.hp = PLAYER_MAX_HP; p.loco.state = 94; p.loco.phase = 1;
                    p.motion.play(d.jack_clip("pl0030_1090.mot"), crate::player::BLEND_DEFAULT, false, true);
                    self.events.push(FightEvent::Continue);
                    if self.rin.state == rs::PL_DEAD { self.rin.state = rs::WAIT; self.rin.phase = 0; }
                }
            }
            78 => {   // keen competition (802956B4, PLAYER-COMPETITION-API): struggle meter, then the directional check
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                match p.loco.phase {
                    1 => { if self.rin.phase >= 2 { p.loco.phase = 3; } }
                    3 => {
                        // raw meter: -0.08 per frame, +0.04 x (1088 + 108C) when both > 0.5 once the cooldown is over
                        self.clash -= 0.08; self.clash_timer -= 1;
                        let (sh, nk) = match self.keen_auto { Some(true) => (2., 2.), Some(false) => (0., 0.), None => (inp.shake, inp.shake_nunchuk) };   // (2: the gain must beat the 0.08 decay)
                        if self.clash_cd == 0 { if sh > 0.5 && nk > 0.5 { self.clash += 0.04 * (sh + nk); } }
                        else { self.clash_cd -= 1; }
                        self.clash = self.clash.clamp(-1., 1.);
                        // smoothed: k (0.0375, 0.1 below zero) toward the raw meter
                        let k = if self.clash < 0. { 0.1 } else { 0.0375 };
                        self.clash_s = k * (self.clash - self.clash_prev) + self.clash_prev;
                        if self.clash_timer == 0 || self.clash_count > 30. {
                            self.clash_prev = self.clash_s;
                            if self.clash_prev <= 0. || self.clash_timer == 0 { p.loco.phase = 10; } else { p.loco.phase = 4; }
                        } else {
                            if self.clash_s <= 0. {
                                if self.clash_prev < 0. { if self.clash_s < -0.75 { self.clash_count += 1.; } } else { self.clash_count = 0.; }
                            } else if self.clash_prev > 0. { if self.clash_s > 0.95 { self.clash_count += 1.; } } else { self.clash_count = 0.; }
                            self.clash_prev = self.clash_s;
                        }
                    }
                    4 => { p.loco.phase = 5; self.clash_delay = 0x1E; self.clash_timer = 0x3C; }
                    5 => {
                        // directional stage: after 30 frames, 1090 > 1.4 and 109C > 1.1 within 60 frames
                        if self.clash_delay == 0 {
                            let (rm, nk) = match self.keen_auto { Some(true) => (2., 2.), Some(false) => (0., 0.), None => (inp.shake_remote, inp.shake_nunchuk) };
                            if rm > 1.4 && nk > 1.1 { p.loco.phase = 6; self.rin.phase = 10; self.events.push(FightEvent::ClashWon); }
                            else if self.clash_timer == 0 { p.loco.phase = 10; } else { self.clash_timer -= 1; }
                        } else { self.clash_delay -= 1; }
                    }
                    10 => {   // loss: 04AD / Rin 0113
                        p.loco.phase = 7; self.rin.phase = 0x14; self.events.push(FightEvent::ClashLost);
                        p.motion.play(d.jack_clip("pl0030_04ad.mot"), 1., false, true);
                    }
                    6 | 7 => { if p.motion.ended && !matches!(self.rin.phase, 10 | 0x14) { self.paired = false; let f = p.loco.facing; p.loco.leave_combat(f); } }
                    _ => {}
                }
            }
            68 => {   // finisher, selector 3 (802A688C): phases 10..17 paired with Rin's finish
                let clip = &d.jack.clips[p.motion.clip];
                let before = p.motion.frame;
                p.motion.advance(clip.frames, &[]);
                match p.loco.phase {
                    10 | 11 => { if p.loco.phase == 10 { p.loco.phase = 11; } else if p.motion.ended { p.loco.phase = 12; } }
                    12 => {
                        p.motion.play(d.jack_clip("pl0030_1306.mot"), 0., true, true);
                        self.meter = (0., 0.); p.loco.phase = 13;
                    }
                    13 => {
                        // (the actor editor's forced finish shakes all through)
                        let shake = if self.keen_auto == Some(true) { 2. } else { inp.shake };
                        let a = (self.meter.0 + 0.005 * shake).min(0.1);
                        let pr = (self.meter.1 + 0.1 * a).clamp(0., 1.);
                        self.meter = (a, pr); p.motion.rate = 1. + pr;
                        if pr >= 1. {
                            self.fin_wait = 20; self.fin_timer = 480 >> crate::player::INTERVAL_SHIFT; self.fin_set = false;
                            p.loco.phase = 14; self.events.push(FightEvent::FinishMeterFull);
                        }
                    }
                    14 => {
                        if self.fin_wait > 0 { self.fin_wait -= 1; }
                        else if !self.fin_set {
                            self.fin_timer -= 1;
                            if (inp.shake_remote > 1.4 && inp.shake_nunchuk > 1.2) || self.fin_timer <= 0 || self.keen_auto == Some(true) { self.fin_set = true; }
                        }
                        if self.fin_set && (p.motion.frame >= 1. && (before < 1. || p.motion.frame < before)) {
                            p.loco.phase = 15; self.rin.phase = 10;   // (+0x1260)->vtbl[0x90](0x30): Rin finish phase 10
                        }
                    }
                    _ => {}
                }
            }
            STATE_SUPLEX => return [0.; 2],   // added: step_suplex drives it
            STATE_EXTERNAL => {   // added: the presenter's clip plays in place
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                return [0.; 2];
            }
            0xC8 => {
                // added secondary weapon (Kreely's pistol), posed by the presenter: the stick walks him relative to
                // the camera at GUN_WALK while he aims or reloads (third-person shooter strafing) [added]
                let k = (inp.stick[0] * inp.stick[0] + inp.stick[1] * inp.stick[1]).sqrt().min(1.);
                if k * k >= crate::player::STICK_MOVE {
                    let a = inp.camera_yaw + (-inp.stick[0]).atan2(inp.stick[1]);
                    let v = if inp.gun_run { GUN_RUN } else { GUN_WALK };
                    return [a.sin() * v * k, a.cos() * v * k];
                }
                return [0.; 2];
            }
            _ => {
                let pad = Pad { stick: inp.stick, camera_yaw: inp.camera_yaw };
                let ended = p.motion.ended || p.motion.frame + p.motion.rate >= (d.jack.clips[p.motion.clip].frames.max(2) - 1) as f32;
                let rng = &mut self.rng;
                if let Some(MotionCmd { name, blend, restart }) = p.loco.step_rng(&pad, ended, &mut |lo, hi| rng.range(lo as i32, hi as i32) as i16) {
                    p.motion.play(d.jack_clip(name), blend, true, restart);
                }
                if let Some(v) = p.loco.voice.take() { self.events.push(FightEvent::Voice(v)); }
                // setMotion initializes the new clip's step to 1; the run-rate
                // request affects its following advance (recorded 00ce: 1, 2.025).
                p.motion.rate = if p.motion.starts != starts { 1. } else { p.loco.rate };
                let clip = &d.jack.clips[p.motion.clip];
                p.motion.advance(clip.frames, &[]);
                p.motion.rate = p.loco.rate;
            }
        }
        if matches!(state0, 85 | 92 | 94) {
            p.loco.facing = rad_lerp_range(p.loco.facing, p.turn_to,
                crate::player::TURN_FRAC * crate::player::VSYNC * p.motion.clock_rate, crate::player::TURN_CLAMP);
        }
        if self.paired { p.pos = self.rin.pos; p.loco.facing = self.rin.facing; return [0.; 2]; }
        if p.airborne { return out; }
        // root motion in the facing frame
        let clip = &d.jack.clips[p.motion.clip];
        let from = if p.motion.starts != starts { 0. } else { from };
        // the original holds the player exactly in idle (frame recording, FRAME-COMPARE item 2): no root xz in state 0
        // [inferred from the observation; the native idle code path that drops it is not located]
        let r = FightData::blend_root(&d.jack.bones, clip, &mut p.motion, from);
        let r = if p.loco.state == 0 && !p.loco.combat { [0.; 2] } else { r };
        let (s, c) = p.loco.facing.sin_cos();
        out[0] += c * r[0] + s * r[1]; out[1] += -s * r[0] + c * r[1];
        out
    }

    /// Added (the actor editor): her clip plays to its end (or loops), its hit and rig cues live, its root travel applied.
    fn rin_editor_step(&mut self, d: &FightData) -> [f32; 2] {
        let Some((clip, looped)) = self.rin_editor else { self.rin_change(rs::WAIT, 0); return [0.; 2]; };
        if self.rin.phase == 0 { self.rin.motion.play(clip, 2., looped, true); self.rin.phase = 1; }
        let (from, starts) = (self.rin.motion.frame, self.rin.motion.starts);
        let c = &d.rin.clips[self.rin.motion.clip];
        let cues = d.combat.cues_for(&c.name);
        for c in self.rin.motion.advance(c.frames, cues) {
            match c.op { 19 => { self.rin.hit_on = true; self.rin.hit_bone = c.a; } 20 => self.rin.hit_on = false, _ => {} }
        }
        if self.rin.motion.ended && !looped { self.rin_editor = None; self.rin_change(rs::WAIT, 0); }
        let c = &d.rin.clips[self.rin.motion.clip];
        let from = if self.rin.motion.starts != starts { 0. } else { from };
        let r = FightData::blend_root(&d.rin.bones, c, &mut self.rin.motion, from);
        let (s, c) = self.rin.facing.sin_cos();
        [c * r[0] + s * r[1], -s * r[0] + c * r[1]]
    }
    /// Added (the actor editor): play `clip` on her.
    pub fn rin_play(&mut self, clip: usize, looped: bool) { self.rin_editor = Some((clip, looped)); self.rin_change(rs::EDITOR, 0); }
    /// Added (the actor editor): start one of her native states from its top.
    pub fn rin_start(&mut self, state: u8) { self.rin_change(state, 0); }
    /// Added (the actor editor): a player blow that lands (no guard or evade roll), through her own damage path.
    pub fn rin_hurt(&mut self, d: &FightData, atk: u32, power: i32) {
        self.force_hit = true;
        let c = self.rin.pos;
        self.rin_damaged(d, AttackHit { atk, center: c, radius: 0., power, hitstop: 4 });
        self.force_hit = false;
    }
    /// [added, VR] Kreely holds her up in his hands: her feet at `pos`, facing `facing`. The first call takes her (not
    /// while she is down to her finisher, already paired, in a suplex or dead); each tick moves her. False: she is not
    /// (or no longer) held — after `VR_HOLD_TICKS` she breaks free and steps back.
    pub fn vr_hold_rin(&mut self, d: &FightData, pos: [f32; 3], facing: f32) -> bool {
        if self.rin.state != RIN_VR_HELD {
            if self.solo || self.rin.hp <= 0 || self.paired || self.suplex.is_some()
                || matches!(self.rin.state, rs::FINISH | rs::FINISH_WAIT | rs::DEAD | RIN_SUPLEXED | rs::EXECUTE | rs::KEEN_COMPE) { return false; }
            self.rin_change(RIN_VR_HELD, 0);
            self.rin.motion.play(d.rin_clip("em0110_1051.mot"), 4., true, true);
            self.rin.thrown = None; self.rin.down = false; self.rin.vr_held = 0;
        }
        self.rin.vr_held += 1;
        if self.rin.vr_held > VR_HOLD_TICKS {
            self.rin_change(rs::BACK_STEP, 0);
            return false;
        }
        self.rin.pos = pos; self.rin.facing = facing;
        true
    }
    /// [added, VR] Let go: she flies off along `vel` (m per tick, the giant swing's throw: she slides on, the fence
    /// shocks her) and lands; `damage` is taken from her (never below 1: the finisher stays the end).
    pub fn vr_throw_rin(&mut self, d: &FightData, vel: [f32; 3], damage: i32) {
        if self.rin.state != RIN_VR_HELD { return; }
        self.rin_change(rs::DM_GIANTSWING, 0);
        self.rin.motion.play(d.rin_clip("em0110_1052.mot"), 2., false, true); self.rin.motion.rate = 1.;
        self.rin.thrown = Some(vel);
        if damage > 0 { self.rin.hp = (self.rin.hp as i32 - damage).max(1) as i16; self.events.push(FightEvent::GiantSwing { damage }); }
    }
    /// [added, VR] Lifted overhead: she is set before Kreely, facing him, for his own grab (the suplex) this tick.
    pub fn vr_rin_to_grab(&mut self, at: [f32; 3], facing: f32) {
        if self.rin.state != RIN_VR_HELD { return; }
        self.rin.pos = at; self.rin.facing = facing;
        self.rin_change(rs::WAIT, 0);
    }

    /// Added (the actor editor): her at `pos` facing `facing`, waiting at full HP, every pair and timer cleared.
    pub fn rin_reset(&mut self, pos: [f32; 3], facing: f32) {
        let r = &mut self.rin;
        r.pos = pos; r.facing = facing; r.hp = RIN_MAX_HP; r.layer = None; r.down = false; r.down_timer = 0.; r.thrown = None;
        r.on_scaffold = false; r.sc_heading = false; r.rig0 = false; r.hit_on = false; r.motion.rate = 1.; r.cd_atk = 60.; r.cd_slalom = 60.;
        self.rin_editor = None; self.keen_auto = None; self.paired = false;
        self.rin_change(rs::WAIT, 0);
    }

    fn rin_change(&mut self, to: u8, sel: u8) {
        if self.rin.state != to { self.events.push(FightEvent::RinState { from: self.rin.state, to }); }
        // her own voice lines on entering a state (research/rinrin-voice-requests.json: PlayVoiceStream from
        // em0440_r1_atk_uraken / _tetuzankou / _giantslalom 0x53, _atk_throw 0x54, _atk_kamikaze 0x55, _finish_wait 0x59)
        if self.rin.state != to {
            let line = match to { rs::URAKEN | rs::TETUZANKOU | rs::SLALOM => Some(0x53), rs::THROW => Some(0x54), rs::KAMIKAZE => Some(0x55), rs::FINISH_WAIT => Some(0x59), _ => None };
            if let Some(v) = line { self.events.push(FightEvent::Voice(v)); }
        }
        if self.rin.state == rs::SLALOM && to != rs::SLALOM { self.rin.rig0 = false; }   // end_atk_giantslalom
        self.rin.state = to; self.rin.phase = 0; self.rin.selector = sel; self.rin.hit_on = false;
    }

    /// cEm0440 onMove: timers, branch (br_*) and update tables, motion.
    fn step_rin(&mut self, d: &FightData, player_world: [f32; 3]) -> [f32; 2] {
        let t = |i: usize| d.combat.tune(i);
        if self.rin.cd_atk > 0. { self.rin.cd_atk -= self.rin.motion.clock_rate; }
        if self.rin.cd_slalom > 0. { self.rin.cd_slalom -= self.rin.motion.clock_rate; }
        // br_wait/br_run and chkAtkStart use getTargetVec (810658C8),
        // the same published target transform as the fan launch.
        let ppos = player_world;
        let d2 = dist2_xz(self.rin.pos, ppos);
        let player_dead = self.player.state() == 95;
        if self.rin.state == RIN_SUPLEXED {
            let clip = &d.rin.clips[self.rin.motion.clip];
            if self.suplex.as_ref().is_none_or(|s| s.grab.is_none()) { self.rin.motion.advance(clip.frames, &[]); }
            return [0.; 2];
        }
        if self.rin.state == RIN_VR_HELD {
            // (her moves wait: she writhes in the giant swing's held loop where the hands put her)
            let clip = &d.rin.clips[self.rin.motion.clip];
            self.rin.motion.advance(clip.frames, &[]);
            return [0.; 2];
        }
        if self.rin.state == rs::FINISH { self.rin_finish(d); return [0.; 2]; }
        if self.rin.state == rs::KEEN_COMPE { self.rin_keen(d); return [0.; 2]; }
        if self.rin.state == rs::EXECUTE { return self.rin_execute(d); }
        self.move_bombs(d);
        if self.rin.state == rs::THROW { return self.rin_throw(d, player_world); }
        if self.rin.state == rs::DEAD { return [0.; 2]; }
        if self.rin.state == rs::EDITOR { return self.rin_editor_step(d); }
        if self.rin_hold && self.rin.state == rs::RUN { self.rin_change(rs::WAIT, 0); }
        // branch functions
        match self.rin.state {
            rs::WAIT if self.rin_hold => {}
            rs::WAIT => {
                if player_dead { self.rin_change(rs::PL_DEAD, 0); }
                else {
                    if t(9) * t(9) < d2 && (self.rng.range(0, 99) as f32) < t(10) { self.rin_change(rs::PROVOCATION, 0); }
                    if 25. < d2 && d2 < 225. { self.rin_change(rs::RUN, 0); }
                    self.chk_atk_start(d, d2); self.chk_scaffold();   // br_wait / br_run call both
                }
            }
            // a kill while she runs (or heads for a scaffold) stops her for the taunt at once: through WAIT the native
            // br_run only reaches PL_DEAD within 2 m, so a far kill never got its laugh before Continue [inferred]
            rs::RUN if player_dead => { self.rin.sc_heading = false; self.rin_change(rs::PL_DEAD, 0); }
            rs::RUN => {
                if d2 < 4. && !self.rin.sc_heading { self.rin_change(rs::WAIT, 0); }
                if t(9) * t(9) < d2 && self.rng.range(0, 99) < 1 { self.rin_change(rs::PROVOCATION, 0); }
                self.chk_atk_start(d, d2); self.chk_scaffold();   // br_wait / br_run call both
                if self.rin.sc_heading {
                    // searchNearScaffold + ckScaffoldPoint: close to the nearest point and on the ground -> scaffold_jump
                    let near = (0..9).min_by(|&a, &b| dist2(self.rin.pos, SCAFFOLDS[a].0).total_cmp(&dist2(self.rin.pos, SCAFFOLDS[b].0))).unwrap();
                    self.rin.sc_next = near;
                    if dist2_xz(self.rin.pos, SCAFFOLDS[near].0) < 15. && self.rin.pos[1] < 18. {
                        self.rin.sc_heading = false; self.rin.sc_cur = near; self.rin_change(rs::SCAFFOLD_JUMP, 0);
                    }
                }
            }
            _ => {}
        }
        if self.rin.state == rs::THROW { return self.rin_throw(d, player_world); }
        if matches!(self.rin.state, rs::SCAFFOLD_JUMP | rs::RIGHT_JUMP | rs::LEFT_JUMP | rs::FRONT_JUMP | rs::KAMIKAZE | rs::BOMB | rs::SCAFFOLD_WAIT)
            || (self.rin.on_scaffold && self.rin.state == rs::PROVOCATION) {
            return self.rin_scaffold(d);
        }
        if self.rin.state == rs::DM_ELECTRIC { return self.rin_electric(d); }
        if matches!(self.rin.state, rs::DM_EXPLOSION | rs::DM_DOWN) { return self.rin_down(d); }
        if self.rin.state == rs::DM_GIANTSWING {
            let clip = &d.rin.clips[self.rin.motion.clip];
            self.rin.motion.advance(clip.frames, &[]);
            if let Some(v) = self.rin.thrown {
                if let Some(at) = Self::fence_hit(d, self.rin.pos, v) {
                    self.rin.pos[0] = at[0]; self.rin.pos[2] = at[2]; self.rin.thrown = None;
                    self.rin_change(rs::DM_ELECTRIC, 0); return [0.; 2];
                }
                let step = [v[0], 0., v[2]];
                self.rin.thrown = Some([v[0] * 0.97, v[1] - GRAVITY * crate::player::VSYNC, v[2] * 0.97]);
                if self.rin.motion.ended { self.rin.thrown = None; }
                return [step[0], step[2]];
            }
            if self.player.state() != 49 || (self.player.loco.phase == 6 && self.rin.motion.ended) {
                if self.rin.hp <= 1 { self.rin_change(rs::FINISH_WAIT, 0); self.events.push(FightEvent::RinFinishReady); }
                else { self.rin_change(rs::BACK_STEP, 0); }
            }
            return [0.; 2];
        }
        // update functions
        let (from, starts) = (self.rin.motion.frame, self.rin.motion.starts);
        let target = if self.rin.sc_heading && self.rin.state == rs::RUN {
            yaw_to(self.rin.pos, SCAFFOLDS[self.rin.sc_next].0)   // em0440_r1_run heads for the scaffold
        } else { yaw_to(self.rin.pos, ppos) };
        let st = self.rin.state;
        let (mot, blend, looping): (&str, f32, bool) = match st {
            rs::WAIT => ("em0440_0000.mot", 15., true),
            rs::PL_DEAD => (if self.rin.phase < 2 { "em0440_0002.mot" } else { "em0440_0000.mot" }, 0., self.rin.phase >= 2),
            rs::RUN => ("em0440_0010.mot", 2., true),
            rs::URAKEN => ("em0440_0100.mot", 2., false),
            rs::TETUZANKOU => ("em0440_0101.mot", 2., false),
            rs::PROVOCATION => ("em0440_0002.mot", 2., false),
            rs::DM_RIGHT => ("em0440_1000.mot", 1., false),
            rs::DM_LEFT => ("em0440_1001.mot", 1., false),
            rs::DM_BODY => ("em0440_1002.mot", 1., false),
            // em0440_r1_finish_wait (810D061C): 1007 once (a stagger and collapse; its MOT header does not loop),
            // then she lies on its last frame; no turnDir
            rs::FINISH_WAIT => ("em0440_1007.mot", 2., false),
            rs::GUARD => (if self.rin.phase < 2 { "em0440_0003.mot" } else { "em0440_0004.mot" }, 0., false),
            rs::RIGHT_STEP => ("em0440_0025.mot", 2., false),
            rs::LEFT_STEP => ("em0440_0026.mot", 2., false),
            rs::BACK_STEP => ("em0440_0028.mot", 2., false),
            rs::SLALOM => ("em0440_0105.mot", 2., false),
            _ => ("em0440_0000.mot", 15., true),
        };
        if self.rin.phase == 0 || (st == rs::PL_DEAD && self.rin.phase == 2) {
            self.rin.motion.play(d.rin_clip(mot), blend, looping, st != rs::WAIT && st != rs::RUN);
            if matches!(st, rs::URAKEN | rs::TETUZANKOU) { self.rin.facing = target; }
            self.rin.phase = if st == rs::PL_DEAD && self.rin.phase == 2 { 3 } else { 1 };
        }
        let clip = &d.rin.clips[self.rin.motion.clip];
        let cues = d.combat.cues_for(&clip.name);
        for c in self.rin.motion.advance(clip.frames, cues) {
            match c.op { 19 => { self.rin.hit_on = true; self.rin.hit_bone = c.a; } 20 => self.rin.hit_on = false,
                         35 if c.a == 0 => self.rin.rig0 = c.b != 0, _ => {} }
        }
        // end_atk_giantslalom (810D08B0) runs on any exit from the slalom, including direct state sets (clash entry)
        if self.rin.state != rs::SLALOM { self.rin.rig0 = false; }
        let ended = self.rin.motion.ended;
        match st {
            // guard: r1_guard (810D0904) ends every frame with turnDir(getTargetDir(), k) (810D0AD8), so she keeps
            // squaring up to the player while blocking; k is a relocated constant, taken as the other turnDir sites' 1.0
            rs::WAIT | rs::RUN | rs::DM_RIGHT | rs::DM_LEFT | rs::DM_BODY | rs::PROVOCATION | rs::PL_DEAD | rs::GUARD => {
                self.rin.facing = rad_lerp_range(self.rin.facing, target, 0.2, std::f32::consts::FRAC_PI_6);
            }
            _ => {}
        }
        match st {
            rs::URAKEN | rs::TETUZANKOU | rs::PROVOCATION | rs::DM_RIGHT | rs::DM_LEFT | rs::DM_BODY
            | rs::RIGHT_STEP | rs::LEFT_STEP | rs::BACK_STEP | rs::SLALOM if ended => self.rin_change(rs::WAIT, 0),
            rs::GUARD if ended => {
                if self.rin.phase == 1 { self.rin.phase = 2; self.rin.guard_timer = t(31); let c = d.rin_clip("em0440_0004.mot"); self.rin.motion.play(c, 0., false, true); }
                else { self.rin_change(rs::WAIT, 0); }
            }
            rs::PL_DEAD if ended && self.rin.phase == 1 => {
                self.rin.phase = 2;
            }
            rs::GUARD if self.rin.phase == 2 => { self.rin.guard_timer -= 1.; if self.rin.guard_timer < 0. { self.rin_change(rs::WAIT, 0); } }
            _ => {}
        }
        let clip = &d.rin.clips[self.rin.motion.clip];
        let from = if self.rin.motion.starts != starts { 0. } else { from };
        let r = FightData::blend_root(&d.rin.bones, clip, &mut self.rin.motion, from);
        let (s, c) = self.rin.facing.sin_cos();
        [c * r[0] + s * r[1], -s * r[0] + c * r[1]]
    }

    /// em0440_r1_finish (810D022C): phases 0-3 with the meter layer, 10-13 the kill; drives the player's motions.
    fn rin_finish(&mut self, d: &FightData) {
        let r = &mut self.rin; let p = &mut self.player;
        let play_pl = |p: &mut PlayerFight, name: &str| { p.motion.play(d.jack_clip(name), 1., false, true); };
        match r.phase {
            0 => { r.motion.play(d.rin_clip("em0440_0121.mot"), 1., false, true); play_pl(p, "pl0030_1305.mot"); r.phase = 1; }
            1 => { if r.motion.ended { r.phase = 2; } }
            2 => {
                r.motion.play(d.rin_clip("em0440_0122.mot"), 1., true, true);
                r.layer = Some((d.rin_clip("em0440_0123.mot"), 0.));
                r.phase = 3;
            }
            3 => { let pr = self.meter.1; r.layer = r.layer.map(|(c, _)| (c, pr)); r.motion.rate = 1. + pr; }
            10 => {
                r.layer = None; r.motion.rate = 1.;
                r.motion.play(d.rin_clip("em0440_0124.mot"), 1., false, true); play_pl(p, "pl0030_1308.mot"); p.loco.phase = 16; r.phase = 11;
            }
            11 => { if r.motion.ended { r.phase = 12; } }
            12 => { r.motion.play(d.rin_clip("em0440_0125.mot"), 1., false, true); r.phase = 13; }
            13 => {
                if r.motion.ended {
                    r.state = rs::DEAD; r.hp = 0; self.paired = false;
                    // (in the actor editor the kill does not end the fight: the editor resets)
                    if !self.editing { self.victory = true; self.events.push(FightEvent::Victory); }
                    let f = p.loco.facing; p.loco.leave_combat(f);
                }
            }
            _ => {}
        }
        let clip = &d.rin.clips[r.motion.clip];
        r.motion.advance(clip.frames, &[]);
    }

    /// em0440_r1_keen_compe (810CEA04): 0110 / 0111 loop, win 0112 (+ tuning 16 damage at the cue) -> 1004 or
    /// finish_wait, loss 0113.
    fn rin_keen(&mut self, d: &FightData) {
        let dmg = d.combat.tune(16) as i32;
        let r = &mut self.rin;
        let cues: Vec<Cue>;
        // (em0440_r1_keen_compe's lines: 0x5B as it opens, 0x56 when he wins it (0112), 0x57 when she does (0113))
        match r.phase { 0 => self.events.push(FightEvent::Voice(0x5B)), 10 => self.events.push(FightEvent::Voice(0x56)), 0x14 => self.events.push(FightEvent::Voice(0x57)), _ => {} }
        let r = &mut self.rin; let p = &mut self.player;
        match r.phase {
            0 => { r.motion.play(d.rin_clip("em0440_0110.mot"), 1., false, true); p.motion.play(d.jack_clip("pl0030_04aa.mot"), 1., false, true); r.phase = 1; }
            1 => { if r.motion.ended { r.motion.play(d.rin_clip("em0440_0111.mot"), 1., true, true); p.motion.play(d.jack_clip("pl0030_04ab.mot"), 1., true, true); r.phase = 2; } }
            10 => { r.motion.play(d.rin_clip("em0440_0112.mot"), 1., false, true); p.motion.play(d.jack_clip("pl0030_04ac.mot"), 1., false, true); r.phase = 11; }
            12 => {
                if r.hp == 1 { r.state = rs::FINISH_WAIT; r.phase = 0; self.paired = false; self.events.push(FightEvent::RinFinishReady); return; }
                r.motion.play(d.rin_clip("em0440_1004.mot"), 1., false, true); r.phase = 13; self.paired = false;
            }
            0x14 => { r.motion.play(d.rin_clip("em0440_0113.mot"), 1., false, true); r.phase = 0x15; }
            _ => {}
        }
        let clip = &d.rin.clips[r.motion.clip];
        cues = d.combat.cues_for(&clip.name).to_vec();
        for c in r.motion.advance(clip.frames, &cues) {
            if r.phase == 11 && c.op == 7 { r.hp = (r.hp as i32 - dmg).max(1) as i16; self.events.push(FightEvent::RinHit { atk: 0, damage: dmg, hp: r.hp, reaction: rs::KEEN_COMPE }); }
        }
        match r.phase {
            11 if r.motion.ended => r.phase = 12,
            13 | 0x15 if r.motion.ended => { r.state = rs::WAIT; r.phase = 0; self.paired = false; }
            _ => {}
        }
    }

    /// Added: the last-life execution. The player lies dead (state 95) on his last continue; she runs to where the clash
    /// pair puts her (pl0030_04aa's throw offset, bone 1024: 0.10, 0, 1.98 in front of him), then plays the clash she
    /// wins with no prompt: 0110 / 04aa, the 0111 / 04ab hold for EXECUTE_HOLD frames, 0113 / 04ad (the cut at its
    /// second slash cue, frame 55); then back to her PL_DEAD taunt with the player dead again.
    pub fn start_execution(&mut self) {
        if self.player.state() != 95 || matches!(self.rin.state, rs::DEAD | rs::FINISH | rs::FINISH_WAIT) || self.solo { return; }
        self.rin.state = rs::EXECUTE; self.rin.phase = 0; self.rin.layer = None; self.rin.motion.rate = 1.;
    }
    pub fn executing(&self) -> bool { self.rin.state == rs::EXECUTE }
    /// Added: after an execution the player is dead again where his last clip left him (state 95, its hold phase).
    pub fn lie_dead(&mut self) {
        let p = &mut self.player;
        p.loco.enter_combat(95); p.loco.phase = 2; p.death_timer = 0; p.invuln = 1000.; p.hp = 0;
    }
    /// Added: a pair that leaves him on the floor hands him to the get-up (state 94: 1092 at >= 40% HP, else 1091).
    pub fn get_up(&mut self) {
        let p = &mut self.player;
        p.knock_type = 0; p.loco.enter_combat(94); p.loco.phase = 0; p.motion.rate = 1.;
    }
    /// Added: as `lie_dead`, falling with `clip` first (a pair that leaves him standing).
    pub fn fall_dead(&mut self, d: &FightData, clip: &str) {
        self.lie_dead();
        let p = &mut self.player;
        p.loco.phase = 1; p.motion.play(d.jack_clip(clip), 6., false, true);
    }
    fn rin_execute(&mut self, d: &FightData) -> [f32; 2] {
        const HOLD: f32 = 40.;
        const CUT: f32 = 55.;
        const SCISSORS_CUT: f32 = 204.;
        let pf = self.player.loco.facing;
        let (s, c) = pf.sin_cos();
        let spot = [self.player.pos[0] + s * 1.98 + c * 0.10, self.rin.pos[1], self.player.pos[2] + c * 1.98 - s * 0.10];
        match self.rin.phase {
            0 => { self.rin.motion.play(d.rin_clip("em0440_0010.mot"), 6., true, true); self.rin.phase = 1; }
            1 => {
                let (dx, dz) = (spot[0] - self.rin.pos[0], spot[2] - self.rin.pos[2]);
                let dist = (dx * dx + dz * dz).sqrt();
                let step = 0.13f32.min(dist);
                if dist > 1e-4 { self.rin.pos[0] += dx / dist * step; self.rin.pos[2] += dz / dist * step; self.rin.facing = dx.atan2(dz); }
                if dist < 0.05 {
                    // the pair shares one origin (the player stands on her root, facing her way: step_player while
                    // paired), so the origin goes to the fallen player and both take his facing
                    self.rin.pos = self.player.pos; self.rin.facing = pf;
                    self.paired = true;
                    self.rin.motion.play(d.rin_clip("em0440_0110.mot"), 4., false, true);
                    self.external(d, "pl0030_04aa.mot", false, 10.);
                    self.rin.phase = 2;
                }
            }
            2 => if self.rin.motion.ended {
                self.rin.motion.play(d.rin_clip("em0440_0111.mot"), 1., true, true);
                self.external(d, "pl0030_04ab.mot", true, 1.);
                self.rin.guard_timer = HOLD; self.rin.phase = 3;
            },
            3 => {
                self.rin.guard_timer -= 1.;
                if self.rin.guard_timer <= 0. {
                    // her alternate win: the fans closed on his neck like scissors (the explorer's added 0113sc /
                    // 04adsc, tools/stage_rin_scissors.py), else the fan cut 0113 / 04ad
                    let sc = d.rin.clips.iter().any(|c| c.name == "em0440_0113sc.mot") && d.jack.clips.iter().any(|c| c.name == "pl0030_04adsc.mot");
                    self.rin.motion.play(d.rin_clip(if sc { "em0440_0113sc.mot" } else { "em0440_0113.mot" }), 1., false, true);
                    self.external(d, if sc { "pl0030_04adsc.mot" } else { "pl0030_04ad.mot" }, false, 1.);
                    self.rin.phase = 4;
                }
            }
            4 => {
                // (the scissors shut on his neck at frame 204: the fans' tips meet there)
                let cut = if d.rin.clips[self.rin.motion.clip].name == "em0440_0113sc.mot" { SCISSORS_CUT } else { CUT };
                if self.rin.motion.frame >= cut { self.events.push(FightEvent::ExecutionCut); self.rin.phase = 5; }
            }
            5 => if self.rin.motion.ended {
                // he stays down where 04ad leaves him; she taunts over him
                self.lie_dead(); self.paired = false;
                self.rin.state = rs::PL_DEAD; self.rin.phase = 2;
                self.events.push(FightEvent::ExecutionDone);
                return [0.; 2];
            },
            _ => {}
        }
        let clip = &d.rin.clips[self.rin.motion.clip];
        let cues = d.combat.cues_for(&clip.name).to_vec();
        self.rin.motion.advance(clip.frames, &cues);
        [0.; 2]
    }

    /// The 800ABAA8 ray of dm_giantswing / dm_explosion: does `step` from `pos` (at spine height) hit the electrified
    /// fence (surface type 0x10, `_ST10` bodies)? Returns the hit point.
    fn fence_hit(d: &FightData, pos: [f32; 3], step: [f32; 3]) -> Option<[f32; 3]> {
        let w = d.world.as_ref()?;
        let len = (step[0] * step[0] + step[2] * step[2]).sqrt();
        if len < 1e-5 { return None; }
        let (dx, dz) = (step[0] as f64, step[2] as f64); let l = (dx * dx + dz * dz).sqrt();
        let dir = [dx / l, 0., dz / l];   // unit length in f64 (raw_surface_ray checks |d|^2 - 1 < 1e-8)
        let o = [pos[0] as f64, pos[1] as f64 + 1.0, pos[2] as f64];
        let hits = crate::collision::raw_surface_ray(w, o, dir, (len * 2.5) as f64, Some("col.hkx")).ok()?;   // fda8 = 2.5
        let h = hits.iter().filter(|h| h.body.name.ends_with("_ST10")).min_by(|a, b| a.distance.total_cmp(&b.distance))?;
        Some([pos[0] + dir[0] as f32 * h.distance as f32 * 0.8, pos[1], pos[2] + dir[2] as f32 * h.distance as f32 * 0.8])
    }

    /// em0440_r1_dm_electricshock (20): shock 1010 with tuning 17 (100) damage, landing 0022, back to wait.
    fn rin_electric(&mut self, d: &FightData) -> [f32; 2] {
        let dmg = d.combat.tune(17) as i32;
        let r = &mut self.rin;
        match r.phase {
            0 => {
                r.thrown = None; r.down = false;
                r.motion.play(d.rin_clip("em0440_1010.mot"), 1., false, true);
                r.hp = (r.hp as i32 - dmg).max(1) as i16;
                self.events.push(FightEvent::RinHit { atk: 0, damage: dmg, hp: r.hp, reaction: rs::DM_ELECTRIC });
                r.phase = 1;
            }
            1 => { if r.motion.ended { r.motion.play(d.rin_clip("em0440_0022.mot"), 1., false, true); r.phase = 2; } }
            _ => {
                if r.motion.ended {
                    if r.hp <= 1 { self.rin_change(rs::FINISH_WAIT, 0); self.events.push(FightEvent::RinFinishReady); } else { self.rin_change(rs::WAIT, 0); }
                    return [0.; 2];
                }
            }
        }
        let clip = &d.rin.clips[self.rin.motion.clip];
        self.rin.motion.advance(clip.frames, &[]);
        [0.; 2]
    }

    /// em0440_r1_dm_explosion (16) / dm_down_damage (30): 1003, down + stun (tuning 14 = 90 for chainsaw hits),
    /// get-up 1004 with its 0x161 cue window, back to wait; hits while down play 1006.
    fn rin_down(&mut self, d: &FightData) -> [f32; 2] {
        let stun = d.combat.tune(14);
        let r = &mut self.rin;
        match (r.state, r.phase) {
            (rs::DM_EXPLOSION, 0) => { r.motion.play(d.rin_clip(if r.selector == 2 { "gh_sweep_down.mot" } else { "em0440_1003.mot" }), 1., false, true); r.phase = 1; }
            (rs::DM_EXPLOSION, 1) if { let (s_, c_) = r.facing.sin_cos(); Self::fence_hit(d, r.pos, [-s_ * 0.3, 0., -c_ * 0.3]).is_some() } => {
                r.down = true; self.rin_change(rs::DM_ELECTRIC, 0); return [0.; 2];
            }
            (rs::DM_EXPLOSION, 1) => { if r.motion.ended { r.down = true; r.down_timer = if r.selector == 1 { stun } else { 30. }; r.phase = 2; } }
            (rs::DM_EXPLOSION, 2) => {
                r.down_timer -= 1.;
                if r.down_timer <= 0. { r.down = false; r.motion.play(d.rin_clip("em0440_1004.mot"), 1., false, true); r.phase = 3; }
            }
            (rs::DM_EXPLOSION, 3) => { if r.motion.ended { r.hit_on = false; self.rin_change(rs::WAIT, 0); return [0.; 2]; } }
            (rs::DM_DOWN, 0) => { r.motion.play(d.rin_clip("em0440_1006.mot"), 1., false, true); r.phase = 1; }
            (rs::DM_DOWN, _) => {
                if r.motion.ended { r.state = rs::DM_EXPLOSION; r.phase = 2; r.down_timer = r.down_timer.max(20.); }
            }
            _ => {}
        }
        let clip = &d.rin.clips[self.rin.motion.clip];
        let cues = d.combat.cues_for(&clip.name).to_vec();
        let from = self.rin.motion.frame;
        for c in self.rin.motion.advance(clip.frames, &cues) {
            match c.op { 19 | 7 if self.rin.phase == 3 => { self.rin.hit_on = true; self.rin.hit_bone = if c.a != 0 { c.a } else { 9 }; } 20 => self.rin.hit_on = false, _ => {} }
        }
        // The motions carry the knock-back: 1003 travels 9.1 back (into the electric fence when one is behind her,
        // checked above), get-up 1004 another 1.6. As rin_throw, the root step moves the actor.
        let r = FightData::blend_root(&d.rin.bones, clip, &mut self.rin.motion, from);
        let (s, c) = self.rin.facing.sin_cos();
        [c * r[0] + s * r[1], -s * r[0] + c * r[1]]
    }

    /// em0440_r1_atk_throw (810CDEA8): wind-up 0102, fling both fans at the player's chest (fling 810CE40C:
    /// dt = 1 / (0.5 * distance * tuning 13), tangents +-10 sideways), 0103 while they fly, catch 0104, back to wait.
    /// Returns the motion's root step: the original Rin turns and slides about 0.5 during 0102 (frame recording,
    /// research/FRAME-COMPARE.md item 1).
    fn rin_throw(&mut self, d: &FightData, player_world: [f32; 3]) -> [f32; 2] {
        let (starts, from) = (self.rin.motion.starts, self.rin.motion.frame);
        self.rin_throw_phase(d, player_world);
        let clip = &d.rin.clips[self.rin.motion.clip];
        let from = if self.rin.motion.starts != starts { 0. } else { from };
        let r = FightData::blend_root(&d.rin.bones, clip, &mut self.rin.motion, from);
        let (s, c) = self.rin.facing.sin_cos();
        [c * r[0] + s * r[1], -s * r[0] + c * r[1]]
    }

    fn rin_throw_phase(&mut self, d: &FightData, player_world: [f32; 3]) {
        let target_yaw = yaw_to(self.rin.pos, player_world);
        // as the original's phase code, the motion update (80079210) runs before the end checks; phases that start
        // a motion advance it after starting it
        let starts_motion = matches!(self.rin.phase, 0 | 2);
        if !starts_motion { let clip = &d.rin.clips[self.rin.motion.clip]; self.rin.motion.advance(clip.frames, &[]); }
        match self.rin.phase {
            // the turn starts on the wind-up's first frame (frame recording: facing 1.313 -> 1.056 at frame 1, x0.80 per frame)
            0 => {
                self.rin.motion.play(d.rin_clip("em0440_0102.mot"), 2., false, true); self.rin.phase = 1;
                self.rin.facing = rad_lerp_range(self.rin.facing, target_yaw, 0.2, std::f32::consts::FRAC_PI_6);
            }
            1 => {
                self.rin.facing = rad_lerp_range(self.rin.facing, target_yaw, 0.2, std::f32::consts::FRAC_PI_6);
                if self.rin.motion.ended { self.rin.phase = 2; }
            }
            2 => {
                let target = [player_world[0], player_world[1] + 1., player_world[2]];
                let rf = bone_frames(&d.rin.bones, &d.rin_poses(&self.rin.motion));
                let dist = dist2(target, self.rin.pos).sqrt() * d.combat.tune(13);
                for (i, fan) in self.rin.fans.iter_mut().enumerate() {
                    if fan.phase != 0 { continue; }
                    let hand = d.rin.bones.iter().position(|b| b.id == fan.hand).unwrap_or(0);
                    fan.p0 = to_world(self.rin.pos, self.rin.facing, rf[hand].1);
                    fan.target = target;
                    let (a, b) = if i == 0 { (10., -10.) } else { (-10., 10.) };
                    fan.t0 = [a, 0., 0.]; fan.t1 = [b, 0., 0.];
                    fan.dt = 1. / (0.5 * dist.max(0.1)); fan.t = 0.; fan.phase = 1; fan.hit = false; fan.pos = fan.p0; fan.local = [0.; 3];
                }
                // MOT header flags are 0: hold the final pose while the fans fly.
                self.rin.motion.play(d.rin_clip("em0440_0103.mot"), 2., false, true);
                self.rin.phase = 3;
            }
            3 => {
                // the catch is started by the returning fan (cIronfan::move runs after Rin's motion update): 0104 shows
                // frame 0 on its first frame [observed in the frame recording]
                if self.rin.fans.iter().any(|f| f.phase == 0) { self.rin.motion.play(d.rin_clip("em0440_0104.mot"), 2., false, true); self.rin.phase = 4; return; }
            }
            // phase 5: the state changes on the frame after the end is seen [observed]
            4 => { if self.rin.motion.ended { self.rin.phase = 5; } }
            5 => { self.rin_change(rs::WAIT, 0); return; }
            _ => {}
        }
        if starts_motion { let clip = &d.rin.clips[self.rin.motion.clip]; self.rin.motion.advance(clip.frames, &[]); }
    }

    /// cIronfan::move (810CAF54): outbound with JUMP_KBOMB 0x161, returning with 0x18C, radius 0.5, spin 0.5585/frame.
    fn move_fans(&mut self, d: &FightData, damage_spheres: &[([f32; 3], f32)]) {
        // Returning to the hand stops new volumes, but the preceding frame's
        // registered attack still has to reach damage dispatch (81005DA4).
        if self.rin.fans.iter().all(|f| f.phase == 0) && self.rin.fan_hits.is_empty() { return; }
        let rf = bone_frames(&d.rin.bones, &d.rin_poses(&self.rin.motion));
        let mut hits = vec![];
        for i in 0..2 {
            let hand = d.rin.bones.iter().position(|b| b.id == self.rin.fans[i].hand).unwrap_or(0);
            let (rotation, translation, scale) = rf[hand];
            let local_to_world = |v: [f32; 3]| {
                let r = qrot(rotation, [v[0] * scale[0], v[1] * scale[1], v[2] * scale[2]]);
                to_world(self.rin.pos, self.rin.facing, [translation[0] + r[0], translation[1] + r[1], translation[2] + r[2]])
            };
            let fan = &mut self.rin.fans[i];
            fan.pos = local_to_world(fan.local);
            let attack_pos = fan.pos;
            // cIronfan evaluates in the current hand's frame (810CAF54), then
            // its parent transform places the fan in world space. Both legs
            // retain the same stored tangents; returning does not swap them.
            let target_actor = from_world(self.rin.pos, self.rin.facing, fan.target);
            let target_hand = qrot([-rotation[0], -rotation[1], -rotation[2], rotation[3]],
                [target_actor[0] - translation[0], target_actor[1] - translation[1], target_actor[2] - translation[2]]);
            let target_hand = [target_hand[0] / scale[0], target_hand[1] / scale[1], target_hand[2] / scale[2]];
            match fan.phase {
                1 | 2 => {
                    if fan.phase == 1 { fan.t = 0.; fan.phase = 2; }
                    fan.local = hermite(fan.t, [0.; 3], fan.t0, target_hand, fan.t1);
                    if !fan.hit { hits.push((i, 0x161u32, attack_pos)); }
                    fan.t += fan.dt * self.rin.motion.clock_rate;
                    if fan.t > 1. { fan.phase = 3; fan.hit = false; }
                }
                3 | 4 => {
                    if fan.phase == 3 { fan.t = 0.; fan.phase = 4; }
                    fan.local = hermite(fan.t, target_hand, fan.t0, [0.; 3], fan.t1);
                    if !fan.hit { hits.push((i, 0x18Cu32, attack_pos)); }
                    fan.t += fan.dt * self.rin.motion.clock_rate;
                    if fan.t > 1. { fan.phase = 0; }
                }
                _ => {}
            }
            fan.spin += 0.558_505_4 * self.rin.motion.clock_rate;
        }
        // reserveAtkHit (81005DA4) registers the volume; the hit system resolves it on the next frame (frame
        // recording: the fan hit lands one frame after the same-frame test) [observed]
        let hits = std::mem::replace(&mut self.rin.fan_hits, hits);
        if hits.is_empty() || self.player.invuln > 0. || self.player.damage_guard > 0. || matches!(self.player.state(), 95 | 68 | 78 | 92 | 94) { return; }
        for (i, atk, c) in hits {
            let touched = damage_spheres.iter().any(|&(p, r)| dist2(p, c) < (0.5 + r).powi(2));
            if touched { self.rin.fans[i].hit = true; self.player_damaged(d, atk, 4); break; }
        }
    }

    /// chkScaffold (810D0DD8): countdown +0x10EC (tuning 23 = 600), then 30% per check -> flag 0 and run.
    fn chk_scaffold(&mut self) -> bool {
        if self.rin.sc_timer < 1. {
            if self.rng.range(0, 99) < 0x1E { self.rin.sc_heading = true; self.rin_change(rs::RUN, 0); return true; }
        }
        self.rin.sc_timer -= 1.;
        false
    }

    /// Scaffold states: scaffold_jump 4 (810CC1E0), right/left/front_jump 5-7 (linearTrans 810D1264), scaffold_wait 25
    /// (br 810D0128: bombs first, then hop or kamikaze), atk_bomb 13 (810CDAA0), atk_kamikaze 8 [simplified].
    fn rin_scaffold(&mut self, d: &FightData) -> [f32; 2] {
        let t = |i: usize| d.combat.tune(i);
        let st = self.rin.state; let ph = self.rin.phase;
        let r = &mut self.rin;
        let play = |r: &mut Rin, name: &str, looping: bool| { r.motion.play(d.rin_clip(name), 2., looping, true); };
        let progress = |r: &Rin| { let n = (d.rin.clips[r.motion.clip].frames.max(2) - 1) as f32; (r.motion.frame / n).clamp(0., 1.) };
        let mut next: Option<u8> = None;
        match (st, ph) {
            (rs::SCAFFOLD_JUMP, 0) => { play(r, "em0440_0020.mot", false); r.phase = 1; }
            (rs::SCAFFOLD_JUMP, 1) => {
                r.facing = rad_lerp_range(r.facing, std::f32::consts::PI + yaw_to(r.pos, SCAFFOLDS[r.sc_cur].0), 0.2, std::f32::consts::FRAC_PI_6);
                if r.motion.ended { play(r, "em0440_0021.mot", false); r.sc_start = r.pos; r.phase = 3; }
            }
            (rs::SCAFFOLD_JUMP, 3) => {
                let k = progress(r); let pt = SCAFFOLDS[r.sc_cur].0;
                r.pos[0] = r.sc_start[0] * (1. - k) + pt[0] * k; r.pos[2] = r.sc_start[2] * (1. - k) + pt[2] * k;
                r.pos[1] = r.sc_start[1] * (1. - k) + pt[1] * k; r.on_scaffold = true;
                if r.motion.ended { r.pos = pt; play(r, "em0440_0022.mot", false); r.phase = 5; }
            }
            (rs::SCAFFOLD_JUMP, 5) => { if r.motion.ended { r.sc_time = t(20); r.bombs_done = false; r.sc_timer = 0.; next = Some(rs::SCAFFOLD_WAIT); } }
            (rs::SCAFFOLD_WAIT, 0) => { play(r, "em0440_0000.mot", true); r.phase = 1; }
            (rs::SCAFFOLD_WAIT, _) => {
                r.facing = rad_lerp_range(r.facing, yaw_to(r.pos, self.player.pos), 0.2, std::f32::consts::FRAC_PI_6);
                r.sc_time -= 1.;
                if !r.bombs_done { next = Some(rs::BOMB); }
                else {
                    r.sc_timer -= 1.;
                    if r.sc_timer < 0. {
                        if r.sc_time > 0. {
                            // hop to an adjacent scaffold (not the one she came from)
                            let opts = SCAFFOLDS[r.sc_cur].2;
                            let pick = opts[(self.rng.range(0, 1)) as usize];
                            r.sc_next = pick.1;
                            r.sc_timer = 20. + 30. * self.rng.rangef(0., 1.);   // fe5c + fe88 * (..) [inferred ratio]
                            next = Some(match pick.0 { 1 => rs::RIGHT_JUMP, 2 => rs::LEFT_JUMP, _ => rs::FRONT_JUMP });
                        } else { next = Some(rs::KAMIKAZE); }
                    }
                }
            }
            (rs::RIGHT_JUMP | rs::LEFT_JUMP | rs::FRONT_JUMP, 0) => { r.facing = SCAFFOLDS[r.sc_cur].1; play(r, "em0440_0023.mot", false); r.phase = 1; }
            (rs::RIGHT_JUMP | rs::LEFT_JUMP | rs::FRONT_JUMP, 1) => {
                if r.motion.ended {
                    play(r, match st { rs::RIGHT_JUMP => "em0440_0025.mot", rs::LEFT_JUMP => "em0440_0026.mot", _ => "em0440_0027.mot" }, false);
                    r.phase = 3;
                }
            }
            (rs::RIGHT_JUMP | rs::LEFT_JUMP | rs::FRONT_JUMP, 3) => {
                let k = progress(r); let (a, b) = (SCAFFOLDS[r.sc_cur].0, SCAFFOLDS[r.sc_next].0);
                r.pos = [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * k);
                if r.motion.ended { r.sc_cur = r.sc_next; r.pos = b; play(r, "em0440_0024.mot", false); r.phase = 5; }
            }
            (rs::RIGHT_JUMP | rs::LEFT_JUMP | rs::FRONT_JUMP, 5) => { if r.motion.ended { next = Some(rs::SCAFFOLD_WAIT); } }
            (rs::BOMB, 0) => { play(r, "em0440_0001.mot", false); r.phase = 1; }
            (rs::BOMB, _) => {
                if r.motion.ended {
                    // objCreate x tuning 21 (3): each a bomb with probability tuning 12 (else the fuseless bl0163,
                    // not staged), dropped from 20 above the arena centre +- 8, fuse rand 150..300
                    for _ in 0..t(21) as usize {
                        if (self.rng.range(0, 99) as f32) >= t(12) { continue; }
                        let (a, b) = (self.rng.rangef(-1., 1.), self.rng.rangef(-1., 1.));
                        let fuse = self.rng.range(150, 300) as f32;
                        r.bombs.push(Bomb { pos: [BOMB_CENTRE[0] + 8. * a, BOMB_CENTRE[1] + 20., BOMB_CENTRE[2] + 8. * b], vel: [0.; 3], live: true,
                                            fuse, blast: 0., turn: [0., 0., 0., 1.], hit_done: false });
                    }
                    r.bombs_done = true; r.sc_timer = 30.;
                    next = Some(rs::PROVOCATION);
                }
            }
            (rs::PROVOCATION, 0) => { play(r, "em0440_0002.mot", false); r.phase = 1; }
            (rs::PROVOCATION, _) => { if r.motion.ended { next = Some(rs::SCAFFOLD_WAIT); } }
            (rs::KAMIKAZE, 0) => { r.facing = yaw_to(r.pos, self.player.pos); r.sc_start = r.pos; play(r, "em0440_002a.mot", false); r.phase = 1; }
            (rs::KAMIKAZE, 1) => {
                // leap from the scaffold to the player's spot; tuning 11 (accuracy) and 22 (speed) [simplified]
                let k = progress(r); let goal = self.player.pos;
                r.pos = [0, 1, 2].map(|i| r.sc_start[i] + (goal[i] - r.sc_start[i]) * k);
                if r.motion.ended { r.on_scaffold = false; play(r, "em0440_002b.mot", false); r.phase = 2; }
            }
            (rs::KAMIKAZE, 2) => { if r.motion.ended { play(r, "em0440_002d.mot", false); r.phase = 3; } }
            (rs::KAMIKAZE, _) => { if r.motion.ended { r.sc_timer = t(23); next = Some(rs::WAIT); } }
            _ => {}
        }
        let clip = &d.rin.clips[r.motion.clip];
        let cues = d.combat.cues_for(&clip.name).to_vec();
        for c in r.motion.advance(clip.frames, &cues) {
            match c.op { 19 => { r.hit_on = true; r.hit_bone = c.a; } 20 => r.hit_on = false, _ => {} }
        }
        if let Some(n) = next { self.rin_change(n, 0); }
        [0.; 2]
    }

    /// Bombs (cBl0063, see `Bomb`): fall, bounce and roll; the fuse burns; the blast hits once within 5 m.
    fn move_bombs(&mut self, d: &FightData) {
        let frames = crate::player::VSYNC * self.rin.motion.clock_rate;   // native frames this game frame
        let ground = |p: [f32; 3]| -> f32 {
            d.world.as_ref().and_then(|w| crate::collision::raw_surface_ray(w, [p[0] as f64, p[1] as f64 + 1., p[2] as f64], [0., -1., 0.], 30., Some("col.hkx")).ok()
                .and_then(|h| h.iter().map(|h| h.distance).min_by(f64::total_cmp)).map(|t| (p[1] as f64 + 1. - t) as f32)).unwrap_or(BOMB_CENTRE[1] - 0.1)
        };
        let mut blasts = vec![];
        for b in self.rin.bombs.iter_mut().filter(|b| b.live) {
            if b.blast > 0. {
                let before = b.blast;
                b.blast -= frames;
                if !b.hit_done && before >= 96. && b.blast < 97. { b.hit_done = true; blasts.push(b.pos); }
                if b.blast <= 0. { b.live = false; }
                continue;
            }
            // flight: gravity, then the floor (the ball's centre stays BOMB_RADIUS above it)
            b.vel[1] -= BOMB_GRAVITY;
            let next = [b.pos[0] + b.vel[0], b.pos[1] + b.vel[1], b.pos[2] + b.vel[2]];
            let floor = ground(b.pos) + BOMB_RADIUS;
            b.pos = next;
            if b.pos[1] <= floor {
                b.pos[1] = floor;
                if b.vel[1] < -0.03 {
                    // a bounce: lose most of the drop, kick off sideways a little (the ball and the uneven grating)
                    b.vel[1] = -b.vel[1] * BOMB_BOUNCE;
                    let (kx, kz) = (self.rng.rangef(-1., 1.), self.rng.rangef(-1., 1.));
                    b.vel[0] += kx * 0.03; b.vel[2] += kz * 0.03;
                } else { b.vel[1] = 0.; b.vel[0] *= 0.93; b.vel[2] *= 0.93; }
            }
            // stay inside the ring (the boss floor cube3219_ST0c spans x -8.3..13.3, z -88..-58.5)
            for (i, lo, hi) in [(0, -8.3 + BOMB_RADIUS, 13.3 - BOMB_RADIUS), (2, -88. + BOMB_RADIUS, -58.5 - BOMB_RADIUS)] {
                if b.pos[i] < lo { b.pos[i] = lo; b.vel[i] = b.vel[i].abs() * BOMB_BOUNCE; }
                if b.pos[i] > hi { b.pos[i] = hi; b.vel[i] = -b.vel[i].abs() * BOMB_BOUNCE; }
            }
            // rolling: turn about the axis across the motion by distance / radius
            let (vx, vz) = (b.vel[0], b.vel[2]);
            let speed = (vx * vx + vz * vz).sqrt();
            if speed > 1e-4 {
                let axis = [vz / speed, 0., -vx / speed];
                let half = speed / BOMB_RADIUS * 0.5;
                let q = [axis[0] * half.sin(), axis[1] * half.sin(), axis[2] * half.sin(), half.cos()];
                b.turn = qmul(q, b.turn);
            }
            b.fuse -= frames;
            if b.fuse <= 0. { b.blast = 100.; b.fuse = -1.; b.vel = [0.; 3]; self.events.push(FightEvent::BombBlast { at: b.pos }); }
        }
        // a blast sets off the other bombs it reaches [inferred: +5168]
        for c in &blasts {
            for b in self.rin.bombs.iter_mut().filter(|b| b.live && b.blast <= 0. && dist2(b.pos, *c) < BOMB_BLAST_RADIUS * BOMB_BLAST_RADIUS) {
                b.fuse = 0.; b.blast = 100.; b.vel = [0.; 3]; self.events.push(FightEvent::BombBlast { at: b.pos });
            }
        }
        self.rin.bombs.retain(|b| b.live);
        if self.player.invuln > 0. || self.player.damage_guard > 0. || matches!(self.player.state(), 95 | 68 | 78 | 92 | 94) { return; }
        for c in blasts {
            // attack type 0xF0 (OIL_DRUM) power 25; the player's knockdown reaction comes from the bomb row 0x161
            if dist2(c, self.player.pos) < BOMB_BLAST_RADIUS * BOMB_BLAST_RADIUS { self.player_damaged_by(d, 0x161, BOMB_POWER, c); break; }
        }
    }

    /// chkAtkStart (810CBEB8); the attack-area test is assumed true inside the c201 ring [inferred].
    fn chk_atk_start(&mut self, d: &FightData, d2: f32) -> bool {
        if self.passive { return false; }
        let t = |i: usize| d.combat.tune(i);
        if self.rin.pos[1] < 18. && self.rin.cd_slalom <= 0. && d2 < t(26) * t(26) && (self.rng.range(0, 99) as f32) < t(29) {
            self.rin.cd_slalom = self.rng.rangef(t(27), t(28)); self.rin_change(rs::SLALOM, 0); return true;
        }
        if self.rin.cd_atk <= 0. {
            if d2 < t(0) * t(0) { self.rin.cd_atk = self.rng.rangef(t(1), t(2)); self.rin_change(rs::URAKEN, 0); return true; }
            if d2 < t(3) * t(3) { self.rin.cd_atk = self.rng.rangef(t(4), t(5)); self.rin_change(rs::TETUZANKOU, 0); return true; }
            if t(6) * t(6) < d2 {
                self.rin.cd_atk = self.rng.rangef(t(7), t(8));
                if self.player.pos[1] < 19. { self.rin_change(rs::THROW, 0); return true; }
            }
        }
        false
    }

    /// Attack spheres against damage volumes, then the receivers' reactions (onPreMove / cPl0000_frameUpdate).
    fn resolve_hits(&mut self, d: &FightData) {
        // player -> Rin
        if (self.player.hit_trigger || self.player.hit_window) && self.player.state() == 39 {
            self.player.hit_trigger = false;
            let frames = bone_frames(&d.jack.bones, &d.jack_poses(&self.player.motion));
            let part = self.player.part.min(frames.len() - 1);
            let center = to_world(self.player.pos, self.player.loco.facing, frames[part].1);
            let (power, hitstop) = d.combat.player_attack(self.player.atk);
            let radius = if self.player.air_atk != 0 { 0.75 } else { PLAYER_HIT_RADIUS };   // 8056A084 for the air attack
            let hit = AttackHit { atk: self.player.atk, center, radius, power, hitstop };
            self.player.last_hit = Some(hit);
            self.strike = Some((hit, self.player.hit_serial));
            let rf = bone_frames(&d.rin.bones, &d.rin_poses(&self.rin.motion));
            let touched = d.combat.rin_damage_volumes.iter().skip(1).any(|v| {
                let Some(bi) = d.rin.bones.iter().position(|b| b.id == v.bone) else { return false; };
                let (q, p, _) = rf[bi];
                let o = qrot(q, v.offset);
                let w = to_world(self.rin.pos, self.rin.facing, [p[0] + o[0], p[1] + o[1], p[2] + o[2]]);
                dist2(w, center) < (hit.radius + v.radius).powi(2)
            });
            if touched && self.rin.hit_by != self.player.hit_serial { self.rin.hit_by = self.player.hit_serial; self.rin_damaged(d, hit); }
        }
        // Rin -> player
        if self.rin.hit_on && self.player.invuln <= 0. && self.player.damage_guard <= 0. && !matches!(self.player.state(), 95 | 68 | 92 | 94) {
            // setAtkHitParam calls: uraken 0x172 r1.5, tetuzankou / giant slalom 0x161 (JUMP_KBOMB) r1.0
            let (atk, radius, power) = match self.rin.state { rs::URAKEN => (0x172, 1.5, 1), rs::TETUZANKOU => (0x161, 1.0, 2), rs::SLALOM => (0x161, 1.0, 6),
                rs::KAMIKAZE => (0x17A, 1.5, 3), rs::DM_EXPLOSION => (0x161, 1.5, 5), _ => (0, 0., 0) };
            if atk != 0 {
                let rf = bone_frames(&d.rin.bones, &d.rin_poses(&self.rin.motion));
                if let Some(bi) = d.rin.bones.iter().position(|b| b.id == self.rin.hit_bone) {
                    let center = to_world(self.rin.pos, self.rin.facing, rf[bi].1);
                    let pf = bone_frames(&d.jack.bones, &d.jack_poses(&self.player.motion));
                    let touched = PLAYER_DAMAGE_PARTS.iter().any(|&(part, r)| {
                        part < pf.len() && dist2(to_world(self.player.pos, self.player.loco.facing, pf[part].1), center) < (radius + r).powi(2)
                    });
                    if touched { self.rin.hit_on = false; self.player_damaged(d, atk, power); }
                }
            }
        }
    }

    /// Kreely's pistol (added secondary): a shot along `dir` from `origin` (world). Hits RinRin's damage volumes with
    /// the native HANDGUN row (0x61, plem_common.ini power 20) through the normal intake [added weapon, native row].
    pub fn player_shot(&mut self, d: &FightData, origin: [f32; 3], dir: [f32; 3], range: f32) -> Shot {
        if matches!(self.rin.state, rs::FINISH | rs::DEAD) { return Shot::Miss; }
        let rf = bone_frames(&d.rin.bones, &d.rin_poses(&self.rin.motion));
        // the nearest damage volume the ray passes through, and where
        let hit = d.combat.rin_damage_volumes.iter().skip(1).filter_map(|v| {
            let bi = d.rin.bones.iter().position(|b| b.id == v.bone)?;
            let (q, p, _) = rf[bi]; let o = qrot(q, v.offset);
            let c = to_world(self.rin.pos, self.rin.facing, [p[0] + o[0], p[1] + o[1], p[2] + o[2]]);
            let oc = [c[0] - origin[0], c[1] - origin[1], c[2] - origin[2]];
            let t = (oc[0] * dir[0] + oc[1] * dir[1] + oc[2] * dir[2]).clamp(0., range);
            (dist2(c, [origin[0] + dir[0] * t, origin[1] + dir[1] * t, origin[2] + dir[2] * t]) < v.radius * v.radius).then_some(t)
        }).min_by(f32::total_cmp);
        let Some(t) = hit else { return Shot::Miss; };
        let at = [origin[0] + dir[0] * t, origin[1] + dir[1] * t, origin[2] + dir[2] * t];
        // a fan swat: she must face the shooter (the native guard's 50-degree rule) and be free to react
        let gap = crate::player::rad_lerp_range(0., self.player.loco.facing - self.rin.facing, 1., 10.).abs().to_degrees();
        let free = matches!(self.rin.state, rs::WAIT | rs::RUN | rs::GUARD | rs::PROVOCATION | rs::RIGHT_STEP | rs::LEFT_STEP | rs::BACK_STEP);
        if gap >= 50. && free && !self.rin.down && (self.rng.range(0, 99) as u32) < FAN_DEFLECT {
            // the fan in her right hand (bone 9) takes it
            let fan = d.rin.bones.iter().position(|b| b.id == 9).map(|i| to_world(self.rin.pos, self.rin.facing, rf[i].1)).unwrap_or(at);
            self.rin_change(rs::GUARD, 0);
            self.events.push(FightEvent::RinGuard { atk: 0x61 });
            self.events.push(FightEvent::RinDeflect { at: fan });
            return Shot::Deflect(fan);
        }
        let (power, hitstop) = d.combat.player_attack(0x61);
        self.player.hit_serial += 1; self.rin.hit_by = self.player.hit_serial;
        let guards_before = self.events.iter().filter(|e| matches!(e, FightEvent::RinGuard { .. })).count();
        self.rin_damaged(d, AttackHit { atk: 0x61, center: origin, radius: 0., power, hitstop });
        // her native guard roll also stops the round on a fan
        if self.events.iter().filter(|e| matches!(e, FightEvent::RinGuard { .. })).count() > guards_before {
            let fan = d.rin.bones.iter().position(|b| b.id == 9).map(|i| to_world(self.rin.pos, self.rin.facing, rf[i].1)).unwrap_or(at);
            self.events.push(FightEvent::RinDeflect { at: fan });
            return Shot::Deflect(fan);
        }
        Shot::Hit(at)
    }

    /// cEm0440::onPreMove damage intake.
    fn rin_damaged(&mut self, d: &FightData, hit: AttackHit) {
        let t = |i: usize| d.combat.tune(i);
        let prev = self.rin.state;
        let chainsaw = (0x75..=0x78).contains(&hit.atk);
        let (reaction, sel, mut evade) = if chainsaw {
            let r = self.rng.range(0, 99) as f32;
            (rs::DM_EXPLOSION, 1, prev != rs::PROVOCATION && (self.rin.last_atk == hit.atk || r < t(25)))
        } else {
            let r = (self.rng.range(0, 99) as f32) < t(24);
            (match hit.atk { 0x6D => rs::DM_RIGHT, 0x6E => rs::DM_LEFT, _ => rs::DM_BODY }, 0, r)
        };
        self.rin.last_atk = hit.atk;
        // facing gap between Rin and the player: behind her (< 50 deg) always hits
        let gap = crate::player::rad_lerp_range(0., self.player.loco.facing - self.rin.facing, 1., 10.).abs().to_degrees();
        let mut guard = false;
        if gap < 50. || matches!(prev, 8 | rs::PROVOCATION | rs::DM_EXPLOSION) { evade = false; }
        else { guard = (self.rng.range(0, 99) as f32) < t(30); }
        if prev == rs::FINISH_WAIT || self.force_hit { evade = false; guard = false; }
        // flag 5 (on in atk_giantslalom / scaffold_jump, cleared each onMove): no guard / evade, damage x1.1 (fdb4)
        let flag5 = matches!(prev, rs::SLALOM | rs::SCAFFOLD_JUMP);
        if flag5 { evade = false; guard = false; }
        let mult = if flag5 { 1.1f32 } else { 1. };
        if self.rin.down {
            // +0x10D1 set: HP -= power, dm_down_damage (30)
            self.rin.hp = (self.rin.hp as i32 - hit.power).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            self.hitstop = self.hitstop.max(hit.hitstop);
            self.rin_change(rs::DM_DOWN, 0);
            self.events.push(FightEvent::RinHit { atk: hit.atk, damage: hit.power, hp: self.rin.hp, reaction: rs::DM_DOWN });
            if self.rin.hp < 1 { self.rin.hp = 1; self.rin.down = false; self.rin_change(rs::FINISH_WAIT, 0); self.events.push(FightEvent::RinFinishReady); }
            return;
        }
        if prev == rs::GUARD || guard {
            self.rin_change(rs::GUARD, 0); self.events.push(FightEvent::RinGuard { atk: hit.atk });
        } else if evade {
            self.rin.cd_atk = 0.;
            let s = [rs::RIGHT_STEP, rs::LEFT_STEP, rs::BACK_STEP][self.rng.range(0, 2) as usize];
            self.rin_change(s, 0); self.events.push(FightEvent::RinEvade { atk: hit.atk });
        } else {
            self.rin.hp = (self.rin.hp as f32 - hit.power as f32 * mult) as i32 as i16;   // float math, truncated (onPreMove)
            // and her hurt line, as onPreMove plays it right after the HP is written (810CA748, voice 0x58)
            self.events.push(FightEvent::Voice(0x58));
            self.hitstop = self.hitstop.max(hit.hitstop);
            // Kreely's sweep (0x78 under his moveset): the God Hand sweep knockdown instead of the flying 1003 [added]
            let sel = if self.kreely && hit.atk == 0x78 && reaction == rs::DM_EXPLOSION && d.kreely.is_some() { 2 } else { sel };
            if prev != rs::FINISH_WAIT { self.rin_change(reaction, sel); }
            self.events.push(FightEvent::RinHit { atk: hit.atk, damage: hit.power, hp: self.rin.hp, reaction });
        }
        if self.rin.hp < 1 {
            self.rin.hp = 1;
            if self.rin.state != rs::FINISH_WAIT { self.rin_change(rs::FINISH_WAIT, 0); self.events.push(FightEvent::RinFinishReady); }
        }
    }

    /// cPl0000_frameUpdate damage + 80274DC4 reaction (Rin's punches: 0x172 -> 100F, 0x173 -> 100A, 0x174 -> 100B).
    /// changeState(0x5B) (B 802C03EC phase 0): 1097 at blend 2, meter = stun counter, counter cleared.
    fn enter_dizzy(d: &FightData, p: &mut PlayerFight) {
        p.loco.state = 91; p.loco.phase = 1;
        p.motion.play(d.jack_clip("pl0030_1097.mot"), 2., true, true);
        p.dizzy = p.stun as f32; p.stun = 0;
    }

    /// `boss_atk`: the getAtkPower index the attacker passes (RIN_ATK_POWER).
    /// Hit slowdown requests the native proxy and animation rates (the intro overrides animation speed).
    fn player_damaged(&mut self, d: &FightData, atk: u32, boss_atk: usize) {
        let from = self.rin.pos;
        self.player_damaged_by(d, atk, RIN_ATK_POWER[boss_atk.clamp(1, 6) - 1], from);
    }

    /// Added: the player's attack sphere this frame while a hit window is open (the same one RinRin is tested against),
    /// with the swing's serial so a target takes each swing once.
    pub fn player_strike(&self) -> Option<(AttackHit, u32)> { self.strike }

    /// Added: one game frame of Kreely's suplex. Both actors hold the grab's frame while their clips play (the
    /// presenter keeps the clips' root travel); the slam lands at the grabber's frame `slam`; at the end each actor takes
    /// its clip's final root travel and RinRin lies down (dm_explosion's down phase, then her get-up 1004).
    fn step_suplex(&mut self, d: &FightData) {
        let (Some(kit), Some(mut sx)) = (d.kreely.as_ref(), self.suplex.take()) else { return; };
        let release = |f: &mut Self| { if sx.on_rin && f.rin.state == RIN_SUPLEXED { f.rin_change(rs::WAIT, 0); } };
        if self.player.state() != STATE_SUPLEX { release(self); return; }
        if sx.on_rin && self.rin.state != RIN_SUPLEXED { sx.on_rin = false; }
        if let Some(left) = sx.grab {
            // the reach: the native grab plays from where he stood; RinRin is drawn into the hold as his hands close
            let p = &mut self.player; p.loco.facing = sx.facing;
            let clip = &d.jack.clips[p.motion.clip];
            p.motion.advance(clip.frames, &[]);
            let left = left - p.motion.step().max(0.5);
            if sx.on_rin {
                let k = (1. - left / GRAB_CONTACT).clamp(0., 1.); let k = k * k * (3. - 2. * k);
                self.rin.pos = [0, 1, 2].map(|i| sx.rin_from[i] + (sx.target[i] - sx.rin_from[i]) * k);
                self.rin.facing = crate::player::rad_lerp_range(self.rin.facing, sx.target_facing, 0.35, 1.);
            }
            if left <= 0. {
                // contact: the suplex takes over from the reach's end (its root travel is already in `origin`)
                sx.grab = None;
                p.pos = sx.origin;
                p.motion.play(d.jack_clip("kreely_gh_suplex.mot"), 6., false, true);
            } else { sx.grab = Some(left); }
            self.suplex = Some(sx);
            return;
        }
        let p = &mut self.player;
        p.pos = sx.origin; p.loco.facing = sx.facing;
        if sx.on_rin { self.rin.pos = sx.target; self.rin.facing = sx.target_facing; }
        let clip = &d.jack.clips[p.motion.clip];
        let before = p.motion.frame;
        p.motion.advance(clip.frames, &[]);
        let (frame, ended) = (p.motion.frame, p.motion.ended);
        if !sx.slammed && before < kit.suplex.slam[0] && frame >= kit.suplex.slam[0] {
            sx.slammed = true; self.hitstop = self.hitstop.max(6);
            self.events.push(FightEvent::SuplexSlam { at: sx.target, on_rin: sx.on_rin });
            if sx.on_rin {
                self.rin.hp = (self.rin.hp as i32 - SUPLEX_POWER).max(1) as i16;
                self.events.push(FightEvent::RinHit { atk: 0x55, damage: SUPLEX_POWER, hp: self.rin.hp, reaction: RIN_SUPLEXED });
            }
        }
        if !ended { self.suplex = Some(sx); return; }
        // the clips' final root travel becomes the actors' positions
        let travel = |bones: &[mw_data::Bone], clip: &mw_data::Clip, facing: f32| -> [f32; 2] {
            let r = crate::sample(bones, clip, (clip.frames - 1) as f32)[bones.len()].translation;
            let (sn, cs) = facing.sin_cos(); [cs * r[0] + sn * r[2], -sn * r[0] + cs * r[2]]
        };
        let t = travel(&d.jack.bones, clip, sx.facing);
        let p = &mut self.player;
        p.pos[0] += t[0]; p.pos[2] += t[1]; p.invuln = 10.;
        let f = p.loco.facing; p.loco.leave_combat(f);
        if sx.on_rin {
            let rt = travel(&d.rin.bones, &d.rin.clips[self.rin.motion.clip], sx.target_facing);
            self.rin.pos[0] = sx.target[0] + rt[0]; self.rin.pos[2] = sx.target[2] + rt[1];
            if self.rin.hp <= 1 { self.rin_change(rs::FINISH_WAIT, 0); self.events.push(FightEvent::RinFinishReady); }
            else {
                // she lies as the throw leaves her (its last frame), then her get-up 1004
                self.rin_change(rs::DM_EXPLOSION, 1);
                self.rin.phase = 2; self.rin.down = true; self.rin.down_timer = 45.;
            }
        }
    }

    /// Added: the presenter takes the player's motion (STATE_EXTERNAL) and plays `clip` (blend in frames).
    pub fn external(&mut self, d: &FightData, clip: &str, looping: bool, blend: f32) {
        let p = &mut self.player;
        if p.state() != STATE_EXTERNAL { p.loco.enter_combat(STATE_EXTERNAL); p.loco.phase = 0; }
        p.hit_window = false; p.hit_trigger = false; p.invuln = 1000.;
        p.motion.play(d.jack_clip(clip), blend, looping, true);
    }
    /// Added: whether the presenter's clip has played out.
    pub fn external_done(&self) -> bool { self.player.state() == STATE_EXTERNAL && self.player.motion.ended }
    /// Added: the presenter gives the player back.
    pub fn end_external(&mut self) {
        let p = &mut self.player;
        if p.state() == STATE_EXTERNAL { p.invuln = 20.; let f = p.loco.facing; p.loco.leave_combat(f); }
    }

    /// Inspection: RinRin never starts an attack (testing the player's moves).
    pub fn debug_passive(&mut self) { self.passive = true; self.rin.cd_slalom = 1e9; self.rin.sc_timer = 1e9; }

    /// Inspection: send RinRin up a scaffold now (she bombs from there), no other attacks meanwhile.
    pub fn debug_scaffold(&mut self) { self.rin.cd_atk = 1e6; self.rin.cd_slalom = 1e6; self.rin.sc_timer = 0.; self.rin.state = rs::RUN; self.rin.sc_heading = true; }

    /// Added: another enemy (mw-play's street grunts) lands `atk` for `power` from `from`, through the player's native
    /// damage path (reaction choice from the hit direction, knockdowns, death). False while the player cannot be hit.
    pub fn enemy_hits_player(&mut self, d: &FightData, atk: u32, power: i32, from: [f32; 3]) -> bool {
        if self.player.invuln > 0. || self.player.damage_guard > 0. || matches!(self.player.state(), 95 | 68 | 78 | 92 | 94) { return false; }
        self.player_damaged_by(d, atk, power, from);
        true
    }

    fn player_damaged_by(&mut self, d: &FightData, atk: u32, power: i32, from: [f32; 3]) {
        let hit_frames = d.combat.enemy_attack(atk).1;
        self.proxy_clock.hit(if hit_frames > 0 { hit_frames.min(255) as u8 } else { 3 });
        let p = &mut self.player;
        p.motion.rate = 1.;
        p.hp = (p.hp as i32 - power).max(0) as i16;
        self.events.push(FightEvent::PlayerHit { atk, damage: power, hp: p.hp });
        // 80274DC4 switch on the attack id: 0x161 / 0x17A (and the bombs, 0x161) knock down; 0x172 -> 100F, 0x173 -> 100A,
        // 0x174 -> 100B, 0x175 -> 100F; 0x18C -> the directional flinch table (LAB_80275130)
        let knockdown = matches!(atk, 0x15B | 0x161 | 0x162 | 0x17A | 0x17B);
        // stun counter (frameUpdate): 0x18C sets it, other hits clamp it
        let full = 0x4B0 >> crate::player::INTERVAL_SHIFT;
        p.stun = if atk == 0x18C { full } else { p.stun.min(full) };
        p.loco.enter_combat(if p.hp < 1 { 95 } else if knockdown { 92 } else { 85 });
        // hit direction in the player's frame (80274DC4: push vector from the attacker)
        let push = [p.pos[0] - from[0], p.pos[2] - from[2]];
        let (s, c) = p.loco.facing.sin_cos();
        let (lx, lz) = (c * push[0] - s * push[1], s * push[0] + c * push[1]);
        p.turn_to = yaw_to(p.pos, from);
        // 80274DC4 knockdown choice from the hit vector in the player's frame (x right, z forward): z >= 0 -> 1015;
        // acos(x) >= 0.61: |z| > cos(0.61) -> 1016 (HP <= 40%) / 1011, else 1018 / 1013 by side; else 1014 / 1013
        let n = (lx * lx + lz * lz).sqrt().max(1e-6); let (ux, uz) = (lx / n, lz / n);
        let (knock, ktype) = if uz >= 0. { ("pl0030_1015.mot", 0) }
            else if ux.acos() >= 0.610_865_2 {
                if uz.acos() < 0.610_865_2 || uz.acos() > 2.530_727_4 {
                    (if p.hp as f32 / PLAYER_MAX_HP as f32 <= 0.4 { "pl0030_1016.mot" } else { "pl0030_1011.mot" }, 1)
                } else if ux >= 0. { ("pl0030_1018.mot", 0) } else { ("pl0030_1013.mot", 0) }
            } else if ux >= 0. { ("pl0030_1014.mot", 0) } else { ("pl0030_1013.mot", 0) };
        if p.hp < 1 {
            // death keeps the knockdown motion (oracle: 1015 / 1016 play under state 95)
            p.motion.play(d.jack_clip(knock), 0.5 * crate::player::BLEND_DEFAULT, false, true);   // 80274DC4: blend 0.5 x +0x1148
            self.events.push(FightEvent::PlayerDead);
            self.events.push(FightEvent::Voice(0x1144));   // B 802C0F80 (when 8005F5B8(8) is clear [inferred])
            self.proxy_clock.death_hit();
            // The recompiled PC reference's default death presentation slows only Jack's motion.
            p.motion.rate = 0.35;
        } else if knockdown {
            p.knock_type = ktype;
            p.motion.play(d.jack_clip(knock), 0.5 * crate::player::BLEND_DEFAULT, false, true);   // 80274DC4: blend 0.5 x +0x1148
            p.invuln = 10.;
        } else {
            let m = match atk {
                0x173 => "pl0030_100a.mot", 0x174 => "pl0030_100b.mot",
                0x18C => if uz >= 0. { ["pl0030_1024.mot", "pl0030_1025.mot", "pl0030_1026.mot"][self.rng.range(0, 2) as usize] }
                    // acos(y) <= 2.53 always holds for the flat push here
                    else if ux.acos() >= 0.610_865_2 {
                        if uz.acos() < 0.610_865_2 || uz.acos() > 2.530_727_4 { "pl0030_100f.mot" }
                        else if ux >= 0. { "pl0030_100d.mot" } else { "pl0030_100c.mot" }
                    } else if ux >= 0. { "pl0030_100b.mot" } else { "pl0030_100a.mot" },
                _ => "pl0030_100f.mot",
            };
            let p = &mut self.player;
            p.motion.play(d.jack_clip(m), 0.5 * crate::player::BLEND_DEFAULT, false, true);   // 80274DC4: blend 0.5 x +0x1148
            p.invuln = 10.; // +0x1158 [inferred value]
        }
        let p = &mut self.player;
        FightData::prime_root(&d.jack.bones, &d.jack.clips[p.motion.clip], &mut p.motion);
        p.root_lag = p.motion.root_initial;
    }
}

/// A reactive test player (not native): closes in and punches, swings the chainsaw inside RinRin's clash window
/// (rig flag 0) and at finish_wait, holds shake through the clash / finisher / dizzy / giant swing, side-steps her
/// melee starts, grabs her when she is down, and continues after a death. Used by the full-fight test and
/// `MW_PLAY_BOT=1` so the Victory evidence does not depend on open-loop key timing.
#[derive(Clone, Debug, Default)]
pub struct Autopilot { tick: u64, rin_attack_seen: u8, grab_frames: u32 }

impl Autopilot {
    pub fn input(&mut self, f: &Fight, camera_yaw: f32) -> FightInput {
        self.tick += 1;
        if f.victory { return FightInput { camera_yaw, ..Default::default() }; }
        let p = &f.player; let r = &f.rin;
        let shake = FightInput { shake: 1.59, shake_remote: 1.59, shake_nunchuk: 1.59, camera_yaw, ..Default::default() };
        match p.state() {
            95 => return FightInput { cont: true, camera_yaw, ..Default::default() },
            78 | 68 | 91 => return shake,
            49 => { self.grab_frames += 1; return FightInput { a_held: self.grab_frames < 120, ..shake }; }
            _ => self.grab_frames = 0,
        }
        let dist = dist2_xz(p.pos, r.pos).sqrt();
        let want = yaw_to(p.pos, r.pos);
        let toward = |k: f32| [-(want - camera_yaw).sin() * k, (want - camera_yaw).cos() * k];
        let mut inp = FightInput { camera_yaw, ..Default::default() };
        if r.rig0 || (r.state == rs::FINISH_WAIT && dist < 2.5) {
            if self.tick % 4 == 0 { inp.swing = 3; }
            if r.state == rs::FINISH_WAIT && dist >= 1.5 { inp.stick = toward(1.); }
            return inp;
        }
        let attacking = matches!(r.state, rs::URAKEN | rs::TETUZANKOU | rs::KAMIKAZE);
        if attacking && self.rin_attack_seen != r.state && dist < 3. { self.rin_attack_seen = r.state; inp.nunchuk_shake = 1.59; return inp; }
        if !attacking { self.rin_attack_seen = 0; }
        if r.down && dist < 1.8 { inp.a_held = true; inp.a_trig = self.tick % 30 == 0; return inp; }
        // hold off while she guards (each blocked punch restarts her guard motion; constant jabs keep her in guard)
        if r.state == rs::GUARD { return inp; }
        if dist > 1.3 { inp.stick = toward(1.); }
        else if self.tick % 6 == 0 { inp.a_trig = true; inp.a_held = true; }
        inp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motion_start_retains_parts_but_clears_synthetic_root() {
        let mut motion = Motion::new(0, true);
        let part = LocalPose { translation:[1.,2.,3.], rotation:[0.,0.,0.,1.], scale:[2.;3] };
        motion.pose = vec![part, part];
        assert!(motion.play(1, 5., false, true));
        assert_eq!(motion.pose[0], part);
        assert_eq!(motion.pose[1], LocalPose { translation:[0.;3], rotation:[0.,0.,0.,1.], scale:[1.;3] });
        assert!(!motion.play(1, 0., false, false));
        assert_eq!(motion.pose[0], part);
    }
    fn data() -> Option<FightData> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
        let read = |p: &str| std::fs::read_to_string(root.join(p)).ok();
        Some(FightData::new(MotionBank::parse(&read("jack-player-motion.json")?).ok()?, MotionBank::parse(&read("rinrin-motion.json")?).ok()?,
                            CombatData::parse(&read("combat.json")?).ok()?))
    }

    #[test]
    fn solo_punch_reports_a_strike() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [-2., 0., 34.], std::f32::consts::PI, [0., -500., 0.], 1);
        f.solo = true;
        let mut seen = 0;
        let mut windows = 0;
        for t in 0..90 {
            let inp = FightInput { a_trig: t % 6 == 0, a_held: t % 6 == 0, ..Default::default() };
            f.step_free(&d, &inp);
            if f.player.hit_window { windows += 1; }
            if f.player_strike().is_some() { seen += 1; }
        }
        assert!(seen > 0, "no strike in 90 frames of punching (state {}, windows {windows})", f.player.state());
    }

    #[test]
    fn intro_hit_clock_stops_proxy_without_stopping_knockdown_animation() {
        let Some(d) = data() else { return; };
        let make = || {
            let mut f = Fight::new(&d, [2.5, 17.598827, -61.], std::f32::consts::PI,
                                   [2.5, 17.648827, -73.], 1);
            f.player_damaged(&d, 0x161, 4);
            // Isolate the existing hit from further AI/contact events.
            f.rin.state = rs::DEAD;
            f
        };
        let mut intro = make();
        let demo = FightInput { demo_active: true, ..Default::default() };
        for _ in 0..10 { intro.step_free(&d, &demo); }
        let stopped = intro.player.pos;
        assert!((stopped[2] - -59.585411).abs() < 0.02, "{:?}", stopped);
        let frame = intro.player.motion.frame;
        for _ in 0..10 { intro.step_free(&d, &demo); }
        assert_eq!(intro.player.pos, stopped);
        assert!(intro.player.motion.frame > frame + 9.);
        assert!(!crate::collision::proxy_advances(intro.proxy_clock.seconds()));
        // Recovery follows the clock, rather than permanently cutting clip 1011 at frame 9.
        for _ in 0..10 { intro.step_free(&d, &FightInput::default()); }
        assert!(intro.player.pos[2] > stopped[2] + 0.05);
        let mut ordinary = make();
        for _ in 0..20 { ordinary.step_free(&d, &FightInput::default()); }
        assert!((ordinary.player.motion.frame - 13.0021).abs() < 0.01);
        assert!(ordinary.player.pos[2] > stopped[2] + 0.25);
    }

    #[test]
    fn one_shot_end_distinguishes_exact_boundary_from_overshoot() {
        let mut m = Motion::new(0, false);
        m.frame = 39.;
        m.advance(41, &[]);
        assert_eq!(m.frame, 40.);
        assert!(!m.ended);
        m.advance(41, &[]);
        assert!(m.ended);
        let mut fractional = Motion::new(0, false);
        fractional.frame = 39.5;
        fractional.advance(41, &[]);
        assert!(fractional.ended);
    }

    #[test]
    fn a_string_chains_and_lands_on_rin() {
        let Some(d) = data() else { return; };
        // player 1.2 in front of Rin, facing her
        let mut f = Fight::new(&d, [0., 0., 1.2], std::f32::consts::PI, [0., 0., 0.], 7);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;  // keep Rin passive for this check
        let mut log = vec![];
        for t in 0..200u32 {
            let tap = t % 6 == 0 && t < 120;
            let inp = FightInput { a_trig: tap, camera_yaw: std::f32::consts::PI, ..Default::default() };
            f.step_free(&d, &inp);
            log.extend(f.events.clone());
        }
        let attacks: Vec<u32> = log.iter().filter_map(|e| if let FightEvent::PlayerAttack { atk } = e { Some(*atk) } else { None }).collect();
        assert!(attacks.starts_with(&[0x6D, 0x6E]), "{attacks:x?}");
        assert!(log.iter().any(|e| matches!(e, FightEvent::RinHit { .. } | FightEvent::RinGuard { .. } | FightEvent::RinEvade { .. })), "{log:?}");
    }

    #[test]
    fn rin_attacks_a_close_player() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 2.], std::f32::consts::PI, [0., 0., 0.], 3);
        let mut hit = false;
        for _ in 0..300 { f.step_free(&d, &FightInput::default()); hit |= f.events.iter().any(|e| matches!(e, FightEvent::PlayerHit { .. })); }
        assert!(hit, "Rin never hit the player; state {} hp {}", f.rin.state, f.player.hp);
        assert!(f.player.hp < PLAYER_MAX_HP);
    }

    #[test]
    fn finisher_with_held_shake_reaches_victory() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 1.5], std::f32::consts::PI, [0., 0., 0.], 5);
        f.rin.hp = 1; f.rin.state = rs::FINISH_WAIT; f.rin.phase = 0;
        let mut seen = vec![];
        for _ in 0..2000u32 {
            // the finisher opens once her collapse (1007) has ended and she lies down
            let down = f.rin.state == rs::FINISH_WAIT && f.rin.motion.ended;
            let inp = FightInput { swing: down as u8, shake: 1.59, ..Default::default() };
            f.step_free(&d, &inp);
            seen.extend(f.events.clone());
            if f.victory { break; }
        }
        assert!(seen.contains(&FightEvent::FinishStart) && seen.contains(&FightEvent::FinishMeterFull), "{seen:?}");
        assert!(f.victory, "player {} phase {} rin {} phase {}", f.player.state(), f.player.loco.phase, f.rin.state, f.rin.phase);
    }

    #[test]
    fn death_then_continue_restores_the_player() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 2.], std::f32::consts::PI, [0., 0., 0.], 9);
        f.player.hp = 1;
        let mut dead = false;
        for _ in 0..600 { f.step_free(&d, &FightInput::default()); dead |= f.player.state() == 95; if dead && f.player.loco.phase == 2 { break; } }
        assert!(dead && f.rin.state == rs::PL_DEAD);
        let mut cont = false;
        for _ in 0..200 {
            f.step_free(&d, &FightInput { cont: true, ..Default::default() });
            if f.events.contains(&FightEvent::Continue) { cont = true; assert_eq!(f.player.hp, PLAYER_MAX_HP); }
        }
        assert!(cont && f.player.hp > 0);
        assert!(matches!(f.player.state(), 0 | 1 | 2 | 4 | 39 | 85 | 92 | 94), "{}", f.player.state());
    }

    #[test]
    fn clash_during_slalom_with_held_shake_costs_rin_300() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 1.5], std::f32::consts::PI, [0., 0., 0.], 11);
        f.rin.cd_slalom = 1e6; f.rin_change(rs::SLALOM, 0);
        let mut seen = vec![];
        for t in 0..900u32 {
            if t == 2 { assert!(!f.rin.rig0); }
            let inp = FightInput { swing: (t >= 2 && t < 40 && t % 4 == 0) as u8, shake: 1.59, shake_remote: 1.59, shake_nunchuk: 1.59, ..Default::default() };
            f.step_free(&d, &inp);
            seen.extend(f.events.clone());
            if f.rin.state == rs::WAIT && seen.contains(&FightEvent::ClashWon) { break; }
        }
        assert!(seen.contains(&FightEvent::ClashStart) && seen.contains(&FightEvent::ClashWon), "{seen:?}");
        assert_eq!(f.rin.hp, RIN_MAX_HP - 300);
    }

    #[test]
    fn rin_throws_fans_that_reach_a_distant_player() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 12.], std::f32::consts::PI, [0., 0., 0.], 13);
        f.rin.cd_slalom = 1e6; f.rin.cd_atk = 0.;
        let mut thrown = false; let mut hit = false;
        for _ in 0..400 {
            f.step_free(&d, &FightInput::default());
            thrown |= f.rin.fans.iter().any(|x| x.phase != 0);
            hit |= f.events.iter().any(|e| matches!(e, FightEvent::PlayerHit { atk: 0x161 | 0x18C, .. }));
        }
        assert!(thrown, "Rin never threw (state {})", f.rin.state);
        assert!(hit, "fans never hit the player");
    }

    #[test]
    fn original_fan_geometry_and_launch_targets_match_published_motion_phase() {
        let mut d=data().expect("native fight data");
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        d.world=Some(mw_data::collision::CollisionWorld::parse(&std::fs::read_to_string(root.join("assets/native/collision/world.json")).unwrap()).unwrap());
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/fan-transform-observed.json")).unwrap()).unwrap();
        let scalar=|v:&serde_json::Value|v.as_f64().unwrap()as f32;
        let vector=|v:&serde_json::Value|[scalar(&v[0]),scalar(&v[1]),scalar(&v[2])];
        let seed=|v:&serde_json::Value|u32::from_str_radix(v.as_str().unwrap(),16).unwrap();
        let first=&observed["initial"];let pl=&first["player"];let rin=&first["rin"];
        let mut f=Fight::new(&d,vector(&pl["pos"]),scalar(&pl["facing"]),vector(&rin["pos"]),seed(&first["seed"])as u64);
        f.player.loco.phase=pl["state"][2].as_u64().unwrap()as u8;
        f.player.motion=Motion::new(d.jack_clip(&format!("{}.mot",pl["clips"][0].as_str().unwrap())),true);f.player.motion.frame=scalar(&pl["frame"]);
        f.rin.phase=rin["state"][2].as_u64().unwrap()as u8;f.rin.facing=scalar(&rin["facing"]);
        f.rin.motion=Motion::new(d.rin_clip(&format!("{}.mot",rin["clips"][0].as_str().unwrap())),false);f.rin.motion.frame=scalar(&rin["frame"]);
        let checks=observed["checks"].as_array().unwrap();let mut checked=0;let mut previous=seed(&first["seed"]);let mut demo=first["demo"].as_i64().unwrap();
        for step in observed["steps"].as_array().unwrap(){
            f.rng=Rng::new(previous as u64);f.step_world(&d,&FightInput{demo_active:demo<0,..FightInput::default()});previous=seed(&step["seed"]);demo=step["demo"].as_i64().unwrap();
            if let Some(check)=checks.iter().find(|c|c["frame"]==step["frame"]){
                let frame=step["frame"].as_u64().unwrap();let launch=check["launch"].as_bool().unwrap();
                for(i,fan)in f.rin.fans.iter().enumerate(){let expected=&check["fans"][i];
                    assert_eq!(fan.phase,expected["phase"].as_u64().unwrap()as u8,"frame{frame} fan{i}");
                    assert!((fan.dt-scalar(&expected["dt"])).abs()<0.000001,"frame{frame} fan{i} launch rate");
                    if launch{assert!(dist2(fan.target,vector(&expected["target"]))<0.01_f32.powi(2),"frame{frame} fan{i} target {:?} vs {}",fan.target,expected["target"]);}
                    else{
                        assert!(dist2(fan.pos,vector(&expected["world"]))<0.0002_f32.powi(2),"frame{frame} fan{i} published world");
                        assert!(dist2(fan.local,vector(&expected["local"]))<0.00005_f32.powi(2),"frame{frame} fan{i} evaluated local");
                    }
                }checked+=1;
            }
        }assert_eq!(checked,checks.len());assert_eq!(checked,42);
    }

    #[test]
    fn rin_attack_selection_reads_the_published_target() {
        let Some(d) = data() else { return; };
        for state in [rs::WAIT, rs::RUN] {
            let mut f = Fight::new(&d, [0., 0., 30.], 0., [0.; 3], 13);
            f.rin.state = state; f.rin.cd_atk = 0.; f.rin.cd_slalom = 1e6;
            // Actor position and published world transform can straddle an
            // attack range during root travel. Native 810CB5C4/810CB818 and
            // 810CBEB8 all obtain the latter through 810658C8.
            f.step_rin(&d, [0., 0., 1.]);
            assert_eq!(f.rin.state, rs::URAKEN, "branch state {state}");
            assert_eq!(f.rin.motion.clip, d.rin_clip("em0440_0100.mot"));
            assert_eq!(f.player.pos, [0., 0., 30.]);
        }
    }

    #[test]
    fn get_up_protection_uses_scene_time_during_hit_slow() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0.; 3], 0., [0., 0., 30.], 13);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;
        f.player.damage_guard = 4.;
        // 80276DB4 multiplies getSpeedRate by GameVSyncF (2), and
        // 80276E04 subtracts that value from +115C. A tiny animation
        // step must not expire protection by a full ordinary update.
        f.proxy_clock.hit(4);
        f.proxy_clock.advance(false);
        f.step_free(&d, &FightInput::default());
        assert!((f.player.damage_guard - (4. - 2. * 0.0003)).abs() < 1e-6);
        // The intro override returns scene rate 1 even with a hit request
        // latched. Protection then consumes the ordinary two units.
        f.step_free(&d, &FightInput { demo_active: true, ..Default::default() });
        assert!((f.player.damage_guard - (2. - 2. * 0.0003)).abs() < 1e-6);
        f.step_free(&d, &FightInput { demo_active: true, ..Default::default() });
        assert_eq!(f.player.damage_guard, 0.);
    }

    #[test]
    fn returning_fan_dispatches_its_final_registered_contact_once() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 12.], 0., [0.; 3], 13);
        // The last returning update registers an attack before marking the fan
        // held. Damage dispatch on the following frame must still see it.
        let center = [0., 1., 12.];
        f.rin.fan_hits.push((0, 0x18C, center));
        assert!(f.rin.fans.iter().all(|fan| fan.phase == 0));
        f.move_fans(&d, &[(center, 0.4)]);
        assert_eq!(f.player.hp, PLAYER_MAX_HP - 10);
        assert!(f.events.iter().any(|e| matches!(e, FightEvent::PlayerHit { atk: 0x18C, .. })));
        assert!(f.rin.fan_hits.is_empty());
        f.player.invuln = 0.; f.player.damage_guard = 0.;
        f.move_fans(&d, &[(center, 0.4)]);
        assert_eq!(f.player.hp, PLAYER_MAX_HP - 10);
    }

    #[test]
    fn rin_climbs_a_scaffold_bombs_and_dives_back() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [2.5, 17.55, -61.], std::f32::consts::PI, [2.3, 17.55, -68.6], 21);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6; f.rin.sc_timer = 0.;
        f.rin.state = rs::RUN; f.rin.sc_heading = true;
        let mut seen = std::collections::BTreeSet::new();
        for i in 0..3000 { f.step_free(&d, &FightInput::default()); let _ = i; seen.insert(f.rin.state); if seen.contains(&rs::KAMIKAZE) && f.rin.state == rs::WAIT { break; } }
        for s in [rs::SCAFFOLD_JUMP, rs::SCAFFOLD_WAIT, rs::BOMB, rs::KAMIKAZE] { assert!(seen.contains(&s), "missing {s}: {seen:?}"); }
        assert!(!f.rin.on_scaffold);
    }

    #[test]
    fn standing_jump_rises_and_lands() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 30.], 0., [0., 0., 0.], 2);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;
        let mut apex: f32 = 0.; let mut landed = false;
        for t in 0..200u32 {
            f.step_free(&d, &FightInput { jump: t == 1, ..Default::default() });
            apex = apex.max(f.player.pos[1]);
            landed |= f.player.state() == 13;
        }
        assert!(apex > 1.0 && apex < 2.5, "apex {apex}");
        assert!(landed && !f.player.airborne && f.player.pos[1] == 0.);
    }

    #[test]
    fn mashing_a_turns_the_string_into_a_rush() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 1.2], std::f32::consts::PI, [0., 0., 0.], 4);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;
        let mut atks = vec![];
        for t in 0..120u32 {
            f.step_free(&d, &FightInput { a_trig: t % 2 == 0, ..Default::default() });
            atks.extend(f.events.iter().filter_map(|e| if let FightEvent::PlayerAttack { atk } = e { Some(*atk) } else { None }));
        }
        assert!(atks.contains(&0x5E), "{atks:x?}");
    }

    #[test]
    fn jump_attack_punches_in_the_air_then_slams() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 30.], 0., [0., 0., 0.], 6);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;
        let mut atks = vec![];
        for t in 0..160u32 {
            f.step_free(&d, &FightInput { jump: t == 1, a_trig: t == 14, ..Default::default() });
            atks.extend(f.events.iter().filter_map(|e| if let FightEvent::PlayerAttack { atk } = e { Some(*atk) } else { None }));
        }
        assert_eq!(atks, vec![0x7E, 0x7F]);
        assert!(!f.player.airborne && f.player.pos[1] == 0.);
    }

    #[test]
    fn giant_swing_on_a_downed_rin() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 1.5], std::f32::consts::PI, [0., 0., 0.], 8);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6; f.rin.state = rs::DM_EXPLOSION; f.rin.phase = 2; f.rin.down = true; f.rin.down_timer = 200.;
        let mut dmg = 0;
        for t in 0..400u32 {
            f.step_free(&d, &FightInput { a_held: t < 120, shake: 2.6, ..Default::default() });
            for e in &f.events { if let FightEvent::GiantSwing { damage } = e { dmg = *damage; } }
        }
        assert_eq!(dmg, 300);
        assert_eq!(f.rin.hp, RIN_MAX_HP - 300);
    }

    #[test]
    fn thrown_into_the_electric_fence() {
        let Some(mut d) = data() else { return; };
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/collision/world.json");
        let Ok(t) = std::fs::read_to_string(root) else { return; };
        d.world = Some(mw_data::collision::CollisionWorld::parse(&t).unwrap());
        // the probe found bossk06_ST10 13.5 m from (2.4, 18.6, -72) along 45 degrees
        let start = [2.4 + 0.7071 * 10., 17.6, -72. + 0.7071 * 10.];
        let mut f = Fight::new(&d, [2.4, 17.6, -72.], 0.785, start, 3);
        f.rin.state = rs::DM_GIANTSWING; f.rin.motion.play(d.rin_clip("em0110_1052.mot"), 0., false, true);
        f.rin.thrown = Some([0.7071 * 0.6, 0.1, 0.7071 * 0.6]);
        let mut shocked = false;
        for _ in 0..60 { f.step_free(&d, &FightInput::default()); shocked |= f.rin.state == rs::DM_ELECTRIC; }
        assert!(shocked, "state {} pos {:?}", f.rin.state, f.rin.pos);
        assert_eq!(f.rin.hp, RIN_MAX_HP - 100);
    }

    #[test]
    fn nunchuk_shake_dodges_rins_attack() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 2.], std::f32::consts::PI, [0., 0., 0.], 3);
        f.rin.state = rs::URAKEN; f.rin.phase = 0;
        f.step_free(&d, &FightInput { nunchuk_shake: 1.59, ..Default::default() });
        assert_eq!(f.player.state(), 7);
        let mut hit = false;
        for _ in 0..25 { f.step_free(&d, &FightInput::default()); hit |= f.events.iter().any(|e| matches!(e, FightEvent::PlayerHit { .. })); }
        assert!(!hit);
    }

    #[test]
    fn hits_during_the_slalom_take_ten_percent_more() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 1.5], std::f32::consts::PI, [0., 0., 0.], 1);
        for (power, want) in [(50, 55), (10, 11), (20, 22), (5, 6)] {
            f.rin.hp = RIN_MAX_HP; f.rin.state = rs::SLALOM;
            f.rin_damaged(&d, AttackHit { atk: 0x76, center: [0.; 3], radius: 0., power, hitstop: 0 });
            assert_eq!(RIN_MAX_HP - f.rin.hp, want, "power {power}");
        }
    }

    #[test]
    fn returning_fan_hit_stuns_then_shaking_clears_the_dizzy() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 6.], std::f32::consts::PI, [0., 0., 0.], 21);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;
        f.player_damaged(&d, 0x18C, 4);
        assert_eq!(f.player.state(), 85);
        let name = &d.jack.clips[f.player.motion.clip].name;
        assert!(["pl0030_100a.mot", "pl0030_100b.mot", "pl0030_100c.mot", "pl0030_100d.mot", "pl0030_100f.mot",
                 "pl0030_1024.mot", "pl0030_1025.mot", "pl0030_1026.mot"].contains(&name.as_str()), "{name}");
        let mut t = 0;
        while f.player.state() == 85 && t < 200 { f.step_free(&d, &FightInput::default()); t += 1; }
        assert_eq!(f.player.state(), 91);
        for _ in 0..60 { f.step_free(&d, &FightInput::default()); }
        assert_eq!(f.player.state(), 91, "no shake keeps the player dizzy");
        let shake = FightInput { shake_remote: 1.59, shake_nunchuk: 1.45, ..Default::default() };
        for _ in 0..120 { f.step_free(&d, &shake); if f.player.state() != 91 { break; } }
        assert_eq!(f.player.state(), 0);
    }

    #[test]
    fn holding_a_away_from_a_downed_rin_whiffs_the_grab() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 6.], std::f32::consts::PI, [0., 0., 0.], 23);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;
        let mut seen21 = false;
        for t in 0..120u32 {
            let inp = FightInput { a_trig: t == 0, a_held: true, ..Default::default() };
            f.step_free(&d, &inp);
            if f.player.state() == 21 { seen21 = true; assert_eq!(d.jack.clips[f.player.motion.clip].name, "pl0030_0226.mot"); }
        }
        assert!(seen21);
        assert_ne!(f.player.state(), 21, "one whiff per hold");
    }

    #[test]
    fn idle_plays_the_fidgets_after_its_loop_countdown() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 30.], std::f32::consts::PI, [0., 0., 0.], 25);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..8000 {
            f.step_free(&d, &FightInput::default());
            if f.player.state() == 0 { seen.insert(d.jack.clips[f.player.motion.clip].name.clone()); }
        }
        assert!(seen.contains("pl0030_0008.mot") && seen.contains("pl0030_0007.mot"), "{seen:?}");
    }

    #[test]
    fn autopilot_wins_the_full_fight() {
        let Some(d) = data() else { return; };
        for seed in [1u64, 2, 3] {
            let mut f = Fight::new(&d, [2.5, 0., -61.], std::f32::consts::PI, [2.3, 0., -68.6], seed);
            let mut bot = Autopilot::default();
            let mut clash = false; let mut finish = false;
            for _ in 0..30000 {
                let inp = bot.input(&f, std::f32::consts::PI);
                f.step_free(&d, &inp);
                clash |= f.events.contains(&FightEvent::ClashStart);
                finish |= f.events.contains(&FightEvent::FinishStart);
                if f.victory { break; }
            }
            assert!(f.victory, "seed {seed}: rin hp {} state {} player hp {}", f.rin.hp, f.rin.state, f.player.hp);
            assert!(finish, "seed {seed}");
            eprintln!("seed {seed}: victory at tick {} clash {clash}", f.tick);
        }
    }

    #[test]
    fn holding_b_revs_the_chainsaw_and_releasing_stows_it() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 30.], std::f32::consts::PI, [0., 0., 0.], 27);
        f.rin.cd_atk = 1e6; f.rin.cd_slalom = 1e6;
        let name = |f: &Fight| d.jack.clips[f.player.motion.clip].name.clone();
        for _ in 0..5 { f.step_free(&d, &FightInput::default()); }
        for _ in 0..30 { f.step_free(&d, &FightInput { b_held: true, ..Default::default() }); }
        assert_eq!((f.player.state(), name(&f).as_str()), (0, "pl0030_0425.mot"));
        f.step_free(&d, &FightInput::default());
        f.step_free(&d, &FightInput::default());
        assert_eq!(name(&f), "pl0030_0424.mot");
        for _ in 0..200 { f.step_free(&d, &FightInput::default()); }
        assert_eq!(name(&f), "pl0030_0000.mot");
    }

    #[test]
    fn clash_without_shaking_is_lost() {
        let Some(d) = data() else { return; };
        let mut f = Fight::new(&d, [0., 0., 1.5], std::f32::consts::PI, [0., 0., 0.], 29);
        f.rin.cd_slalom = 1e6; f.rin_change(rs::SLALOM, 0);
        let mut seen = vec![];
        for t in 0..900u32 {
            let swing = (f.player.state() != 78 && t % 4 == 0) as u8;
            f.step_free(&d, &FightInput { swing: if seen.contains(&FightEvent::ClashStart) { 0 } else { swing }, ..Default::default() });
            seen.extend(f.events.clone());
            if seen.contains(&FightEvent::ClashLost) && f.player.state() != 78 { break; }
        }
        assert!(seen.contains(&FightEvent::ClashStart) && seen.contains(&FightEvent::ClashLost), "{seen:?}");
        assert!(!seen.contains(&FightEvent::ClashWon));
        assert_eq!(f.rin.hp, RIN_MAX_HP);
    }
}

