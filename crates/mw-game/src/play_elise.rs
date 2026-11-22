//! The Elise boss stage (MW_STAGE=elise): room c303 (the church), cEm0630 Elise and her 100 cEm0330 bats
//! (research/ELISE-BOSS.md, addresses there). Her world is tools/stage_arena.py MW_ROOM=c303 and its collision the
//! render-geometry stand-in (tools/stage_render_collision.py) until the native Havok files can be staged.
//! Her states as the native table (§3): start (stand 0001; up into the air when the player is within 12.5 m), idle /
//! move (ground walk 0010; air hover 0000 and flight 0011-0014 inside the arena bins x -3..3, z -8..23), air (take-off
//! 0003 / landing 0002; landing re-summons all 100 bats and rolls the next take-off, decideRebirthTime U[150, 300]),
//! attack (wing strike 0111: first strike frames < 25, second after; ATHIT 0x172 / 0x141 by level), somersault (0110),
//! the air volley (0100 / 0101: bats dive at the player), Black Dragon (0102 -> 0103 -> 0104 -> 0105: one or two heads
//! of bats chase the player, power 20, aborted under 50 bats or after losing 70 + 30L HP), guard (0020: chip damage at
//! 80 %, a guard-cancel somersault after decideGuardCancel blocked hits), damage (1000 / 1001 by side, 1002 front or
//! back), down (em0440_1003 -> 1004), hide (every 250 accumulated damage: 0002, then a coffin pose 000b / 000c / 000e at
//! a crypt; +1 HP a second after 5 s; hint bats circle it), the clash (striking her coffin: 0206 / pl0030_0466, 0200 /
//! 0460, the struggle 0201 / 0461 with a shake meter, then 0205 / 0465 and a swing sequence; won: 0203 / 0463 and -300
//! HP; lost: the bite 0204 / 0464, +100 HP), dead (1005) and finish_wait (1006).
//! Her voice (an_c303 lines, tools/audio/stage_announce.py MW_ROOM=303) through the announcer's voice events: 0x9D
//! wing strike, 0x9F the bats' answer, 0xA0 landing, 0xA3 the dragon-dodge hint, 0xA4 / 0xA5 hurt (strong / light),
//! 0xA7 clash start, 0xA8 clash won, 0xA9 the bite, 0xAB victory.
//! Level (LevelFlow): 0..70, rising with damage dealt to her (rate 70 / max HP), falling with her heals; L = level / 100
//! drives the attack interval (decideAttackInterval), the guard cancel, the dragon's heads and loops.
//! The crypts are the coffin controller's table (getCrypticPoint 80075C0C, CRYPTS). The finisher is the generic boss
//! catch (802A4830 type 8 -> 802A9830): no pair, she becomes a corpse and the room plays ev34 (play_intro.rs outro),
//! then the clear line 0x1027. Her death sends the swarm away (bat cmd 4: 20 m out, then under the floor).
//! [inferred] the bats fly simple steering paths in place of jkSpline; the clash's swing sequence is keyboard prompts
//! (PC: F down-swing, Q side-swing); burning (arm type 7 / held bl0042, ATHIT 0x11A) is not ported.
use bevy::prelude::*;
use mw_data::MotionBank;
use mw_sim::character::{mix, sample_retargeted, strip_root_xz, Retarget};

const SPAWN: Vec3 = Vec3::new(0., 0., -9.);
/// Built-in values; assets/defs/actors/em0630.json (play_actors.rs) overrides them: `hp`, `tune` keys csv0..csv6,
/// power0..power4, the summon's `summon_*`, and `voices` by event name.
const MAX_HP_NATIVE: i32 = 1000;
fn def() -> Option<&'static super::actors::ActorDef> { super::actors::get("em0630") }
fn max_hp() -> i32 { def().map_or(MAX_HP_NATIVE, |d| if d.hp > 0 { d.hp } else { MAX_HP_NATIVE }) }
fn csv(k: usize) -> i32 { def().map_or(CSV[k], |d| d.tune(&format!("csv{k}"), CSV[k] as f32) as i32) }
fn power(k: usize) -> i32 { def().map_or(POWER[k], |d| d.tune(&format!("power{k}"), POWER[k] as f32) as i32) }
fn tune(key: &str, v: f32) -> f32 { def().map_or(v, |d| d.tune(key, v)) }
fn vo(event: &str, native: u16) -> u16 { def().and_then(|d| d.voice(event)).unwrap_or(native) }
const BATS: usize = 100;
const FLY_HEIGHT: f32 = 2.6;
/// The circle she flies over the beaten player (m).
const VICTORY_RADIUS: f32 = 3.5;
/// em0630.csv: hide every N damage, bite heal, guard chip %, hint-bat interval s, heal start s, heal step s, clash damage.
const CSV: [i32; 7] = [250, 100, 80, 5, 5, 1, 300];
/// ボス攻撃1..5 (normal).
const POWER: [i32; 5] = [10, 10, 10, 10, 20];
const LEVEL_MAX: f32 = 70.;
/// getCrypticPoint 80075C0C (coffin controller fn_800751BC): position and yaw per crypt. 0..7 are the bl0015_N001..N008
/// coffins (same transforms as ObjTbl.obt), 8 the point over the apse (no object), 9 / 10 bl0001_N001 / N002.
const CRYPTS: [([f32; 3], f32); 11] = [
    ([13.5, 1.5182, -10.], -std::f32::consts::FRAC_PI_2), ([13.5, 1.5182, 0.], -std::f32::consts::FRAC_PI_2),
    ([13.5, 1.5182, 10.], -std::f32::consts::FRAC_PI_2), ([13.5, 1.5182, 20.], -std::f32::consts::FRAC_PI_2),
    ([-13.5, 1.5182, -10.], std::f32::consts::FRAC_PI_2), ([-13.5, 1.5182, 0.], std::f32::consts::FRAC_PI_2),
    ([-13.5, 1.5182, 10.], std::f32::consts::FRAC_PI_2), ([-13.5, 1.5182, 20.], std::f32::consts::FRAC_PI_2),
    ([0., 5.6, -20.], 0.), ([15.6857, 3.945, -15.], -std::f32::consts::FRAC_PI_2), ([-15.6857, 3.945, -15.], std::f32::consts::FRAC_PI_2),
];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum St { Start, Idle, Air, Move, Attack, Somersault, Volley, Dragon, Guard, Damage, Down, Hide, Keen, Dead, FinishWait, Finisher, Victory,
              /// the actor editor plays a clip (play_editor.rs)
              Editor }

#[derive(Clone, Copy, PartialEq, Debug)]
enum BatSt { Orbit, Dive, Head(usize), Hint, Dead, Scatter,
             /// [added] flying to drop group k's point, to fall there and become a zako
             Summon(usize) }
/// [added] a hide-phase zako drop: the floor point, frames since the summon, and whether it has landed.
#[derive(Clone, Copy)]
struct ZakoDrop { spot: Vec3, t: f32, done: bool }
/// `from` / `aim`: the scatter path (cmd 4), start and end.
struct Bat { roots: [Entity; 2], joints: [Option<Vec<Entity>>; 2], pos: Vec3, vel: Vec3, st: BatSt, phase: f32, frame: f32, t: f32, from: Vec3, aim: Vec3 }

#[derive(Resource)]
pub struct Elise {
    pub on: bool, bank: Option<MotionBank>, bat_bank: Option<MotionBank>, clips: std::collections::HashMap<String, usize>,
    roots: [Entity; 2], joints: [Option<Vec<Entity>>; 2], spawned: bool,
    pub pos: Vec3, pub yaw: f32, height: f32, pub st: St, sub: u8, clip: usize, frame: f32, timer: f32,
    pub hp: i32, level: f32, atk_timer: f32, atk_count: u32, hide_acc: i32, hide_n: i32, rebirth: f32, flying: bool,
    guard_hits: u32, guard_cancel: u32, landed: u32, hit_by: u32, dragon_hp: i32, crypt: usize,
    /// the controller's pick counters: coffins in a row (+0x9C), other crypts in a row (+0xA0, starts at 1)
    crypt_run: [u32; 2],
    /// the swarm was sent away at her death (dead 81107C8C: bat cmd 4)
    scattered: bool,
    heads: Vec<(Vec3, Vec3, u32, bool)>, loops: u32, bats: Vec<Bat>, rng: u32, volley_hit: bool,
    /// the clash: shake meter (0..30), swing-sequence step and its prompt, the player's last clip (for its root travel)
    meter: f32, qte: usize, qte_keys: [bool; 3], pub prompt: Option<String>, player_from: Vec3,
    /// her root travel scale in 0206 (the 10.93 m flight out of the crypt shortened to the crypt's real distance) [added]
    root_k: f32,
    /// the last-life execution: 0 off, 1 the approach, 2 the hold, 3 the bite
    exec: u8,
    /// the clash's placement case (fn_802A03A0 +0x1230: 0 in place, 1 / 2 the far side crypts, 3 the centre line)
    /// and whether frame 23's move to the floor spot has run
    keen_case: u8, keen_moved: bool, keen_tail: u32,
    /// his next pair clip, started on the next update
    his_next: Option<(&'static str, bool)>, her_next: Option<&'static str>,
    /// the bite's hit-stop ramp position (0 off) and her last frame step
    slow: usize, last_step: f32, his_slow: usize,
    /// the step his clip took in the fight's last update (his body trails by it)
    his_step: f32,
    /// a pistol round's flinch over her upper body (clip, frame), the model shown, the round in flight (hurt)
    hurt_walk: Option<(usize, f32)>, colored: bool, shot: bool,
    /// the bite's drain: points taken, the closing blow taken, the red pulse (1 on a drain, fading)
    drained: u32, bite_blow: bool, pub drain_pulse: f32,
    /// the one-update extra step on his drawn spot at the 0460 placement (world)
    his_blip: Vec3,
    /// the air flight hop: start and length in frames (81104014)
    fly_from: Vec3, fly_len: f32,
    /// the clash struggle's lean (0202 / pl0030_0462 weight, the smoothed meter)
    lean: f32,
    target: Vec3,
    /// voice lines requested this frame (flushed into the fight's events for the announcer)
    voices: Vec<u16>,
    /// the hide-phase drops in flight, and the zako they asked for (position, yaw), for play_street.rs
    drops: Vec<ZakoDrop>, zako: Vec<(Vec3, f32)>,
    /// the actor editor: the clash's forced outcome, its clip loops, it has stood her at her spot, a reset is due
    auto: Option<bool>, ed_loop: bool, ed_ready: bool, ed_reset: bool,
    /// she has drunk: his blood goes round her mouth when she next stands upright
    mouth_due: bool,
    /// the editor's camera scene plays (her clip with its root travel, his paired from her origin)
    ed_scene: bool,
    /// this clash's cut-in has been asked for
    cutin_done: bool,
    /// and his neck takes the fang marks (the side she bit: +1 his right; frames he has stood)
    neck_due: Option<(f32, u32)>,
    /// the pair cameras pl0030_c46x (tools/stage_camera_clips.py em/em0630.dat)
    cams: std::collections::HashMap<String, CamClip>,
}

/// A camera MOT: eye, look target and field of view per frame, in the pair's frame.
#[derive(serde::Deserialize)]
pub struct CamClip { frames: usize, eye: Vec<[f32; 3]>, target: Vec<[f32; 3]>, fov: Vec<f32> }
#[derive(serde::Deserialize)]
struct CamFile { clips: std::collections::HashMap<String, CamClip> }
impl CamClip {
    pub fn frames(&self) -> usize { self.frames }
    /// World eye, target and fov at a frame, the pair anchored at `origin` facing `yaw`.
    pub fn sample(&self, frame: f32, origin: Vec3, yaw: f32) -> (Vec3, Vec3, f32) {
        let k = (frame.max(0.) as usize).min(self.frames.max(1) - 1);
        let n = (k + 1).min(self.frames.max(1) - 1);
        let u = (frame - k as f32).clamp(0., 1.);
        let lerp = |v: &Vec<[f32; 3]>| Vec3::from_array(v[k]).lerp(Vec3::from_array(v[n]), u);
        let q = Quat::from_rotation_y(yaw);
        let fov = self.fov[k] + (self.fov[n] - self.fov[k]) * u;
        (origin + q * lerp(&self.eye), origin + q * lerp(&self.target), if (0.2..2.).contains(&fov) { fov } else { 0.8 })
    }
}
pub fn load_cams(file: &str) -> std::collections::HashMap<String, CamClip> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
    std::fs::read_to_string(root.join(file)).ok().and_then(|t| serde_json::from_str::<CamFile>(&t).ok()).map(|f| f.clips).unwrap_or_default()
}

pub fn on() -> bool { std::env::var("MW_STAGE").as_deref() == Ok("elise") }

pub struct ElisePlugin;
impl Plugin for ElisePlugin {
    fn build(&self, app: &mut App) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let on = on();
        let load = |f: &str| on.then(|| std::fs::read_to_string(root.join("assets/native").join(f)).ok().and_then(|s| MotionBank::parse(&s).ok())).flatten();
        let bank = load("elise-motion.json");
        let bat_bank = load("bat-motion.json");
        let clips = bank.as_ref().map(|b| b.clips.iter().enumerate().map(|(i, c)| (c.name.clone(), i)).collect()).unwrap_or_default();
        app.insert_resource(Elise { on: on && bank.is_some(), bank, bat_bank, clips, roots: [Entity::PLACEHOLDER; 2], joints: [None, None], spawned: false,
                                    pos: SPAWN, yaw: 0., height: 0., st: St::Start, sub: 0, clip: 0, frame: 0., timer: 0., hp: max_hp(), level: 0., atk_timer: 90.,
                                    atk_count: 0, hide_acc: 0, hide_n: 0, rebirth: 200., flying: false, guard_hits: 0, guard_cancel: 3, landed: 0, hit_by: 0,
                                    dragon_hp: 0, crypt: 0, crypt_run: [0, 1], scattered: false, heads: vec![], loops: 0, bats: vec![], rng: 0x0e15e, volley_hit: false,
                                    meter: 0., qte: 0, qte_keys: [false; 3], prompt: None, player_from: Vec3::ZERO, root_k: 1., exec: 0, keen_case: 0, keen_moved: false, keen_tail: 0, his_next: None, her_next: None, slow: 0, last_step: 1., his_slow: 0, his_step: 1., hurt_walk: None, colored: true, shot: false, drained: 0, bite_blow: false, drain_pulse: 0., his_blip: Vec3::ZERO, fly_from: Vec3::ZERO, fly_len: 1., lean: 0., target: Vec3::ZERO, voices: vec![], drops: vec![], zako: vec![], auto: None, ed_loop: false, ed_ready: false, ed_reset: false, mouth_due: false, neck_due: None, ed_scene: false, cutin_done: false, cams: if on { load_cams("elise-camera.json") } else { Default::default() } })
            .add_systems(Update, (bind, pose, prompt, drain_tint).chain());
    }
}

impl Elise {
    /// The last-life execution (play_execution.rs) [added; a stand-in on her clash]: she comes down to the fallen
    /// player, takes the struggle pair (0201 / pl0030_0461, her side 1.158 m in front of him: 0461's throw offset)
    /// with no prompt, then wins it with the bite (0204 / 0464), and laughs over him.
    pub fn executing(&self) -> bool { self.exec > 0 }
    pub fn start_execution(&mut self, _play: &mut super::Play) -> bool {
        if !self.on || self.hp < 1 || matches!(self.st, St::Dead | St::FinishWait | St::Finisher | St::Hide) { return false; }
        self.exec = 1; self.heads.clear(); self.prompt = None;
        self.set(St::Move, if self.height > 0.3 { "em0630_0011.mot" } else { "em0630_0010.mot" });
        true
    }
    /// The pair camera while Kreely plays a clash clip: its c46x twin (pl0030_0463 -> pl0030_c463).
    pub fn camera(&self, play: &super::Play) -> Option<(Vec3, Vec3, f32)> {
        if !self.on || self.st != St::Keen || !play.fight.paired { return None; }
        let m = &play.fight.player.motion;
        let name = play.fd.jack.clips.get(m.clip)?.name.replacen("pl0030_0", "pl0030_c", 1);
        // setObjMotCamera(.., player): the camera rides on Kreely as his root travels (c466's target ends 2.94 m behind
        // him, between him and her after she has flown past)
        let q = Quat::from_rotation_y(play.fight.player.loco.facing);
        // (on his drawn spot, a step behind his root; the pull-out's c466 on his root itself: both matched to the live
        // recomp's shown camera, update by update)
        let lag = if name == "pl0030_c466.mot" { 0. } else { self.his_step * PAIR_LAG };
        let at = Vec3::from_array(play.fight.player.pos) + q * root_at(&play.fd.jack, m.clip, (m.frame - lag).max(0.));
        let cam = self.cams.get(&name)?;
        // (the original's camera rides on his object, which stands 0.1 above the floor the port keeps him on)
        let at = at + Vec3::Y * 0.1;
        let (mut eye, mut target, fov) = cam.sample(m.frame, at, play.fight.player.loco.facing);
        // the struggle leans its camera too: c461 toward c462 by the meter, as 0461 toward 0462 (phase 2)
        if name == "pl0030_c461.mot" {
            if let Some(c2) = self.cams.get("pl0030_c462.mot") {
                let (e2, t2, _) = c2.sample(m.frame, at, play.fight.player.loco.facing);
                eye = eye.lerp(e2, self.lean); target = target.lerp(t2, self.lean);
            }
        }
        if std::env::var_os("MW_CAM_LOG").is_some() {
            eprintln!("[cam] tick {} elise {name} f{:.1}", play.tick, m.frame);
            eprintln!("PORT_ECAM_ROW {{\"tick\":{},\"clip\":\"{}\",\"pf\":{:.4},\"her\":\"{}\",\"hf\":{:.4},\"eye\":[{:.5},{:.5},{:.5}],\"dest\":[{:.5},{:.5},{:.5}]}}",
                      play.tick, play.fd.jack.clips[m.clip].name, m.frame, self.bank.as_ref().map_or("", |b| b.clips[self.clip].name.as_str()), self.frame, eye.x, eye.y, eye.z, target.x, target.y, target.z);
        }
        Some((eye, target, fov))
    }
    /// The fight is over and her clear line has been asked for: the room clear runs (play_clear.rs).
    pub fn cleared(&self) -> bool { self.on && self.st == St::Finisher && self.sub >= 2 }
    /// Her playing clip and frame, for its motion cues (play.rs cue_sounds).
    pub fn motion(&self) -> Option<(&str, f32)> { (self.on && self.spawned).then(|| self.bank.as_ref().map(|b| (b.clips[self.clip].name.as_str(), self.frame))).flatten() }
    fn roll(&mut self) -> f32 { self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (self.rng >> 8) as f32 / (1 << 24) as f32 }
    fn set(&mut self, st: St, clip: &str) { self.st = st; self.sub = 0; self.frame = 0.; self.landed = 0; if let Some(&c) = self.clips.get(clip) { self.clip = c; } }
    fn play(&mut self, clip: &str) { self.frame = 0.; if let Some(&c) = self.clips.get(clip) { self.clip = c; } }
    fn len(&self) -> f32 { self.bank.as_ref().map_or(1., |b| b.clips[self.clip].frames as f32) }
    fn l(&self) -> f32 { self.level / 100. }
    /// decideAttackInterval 811095A8 (frames), longer while flying.
    fn attack_interval(&mut self) -> f32 {
        let l = self.l();
        let lo = 20. * (1. - l) + 30. * (1. - l); let hi = 100. * (1.2 - l);
        let base = lo + (hi - lo).max(0.) * self.roll();
        if self.flying { base + 20. * l + l * l * 100. * (1.2 - l) } else { base }
    }
    /// LevelFlow 81109A30: damage raises the level, heals lower it.
    fn level_flow(&mut self, delta: f32) { self.level = (self.level + delta * LEVEL_MAX / max_hp() as f32).clamp(0., LEVEL_MAX); }
    /// Where her body is (for hits): her position plus the flight height.
    /// A clip of her bank by name (the chainsaw arm's wp0600 clips for her pairs live there).
    pub fn bank_clip(&self, name: &str) -> Option<&mw_data::Clip> { self.bank.as_ref()?.clips.get(*self.clips.get(name)?) }
    pub fn body(&self) -> Vec3 { self.pos + Vec3::Y * (self.height + 1.0) }
    /// [added, VR] Her model's animated joints (the shown model), for where she really is in a paired clip.
    pub fn vr_joints(&self) -> Option<&Vec<Entity>> { self.joints[self.colored as usize].as_ref() }
    /// [added, VR] The joint of bone `id` (3 the chest, 4 the neck) on the shown model.
    pub fn vr_joint(&self, id: i16) -> Option<Entity> { let i = self.bank.as_ref()?.bones.iter().position(|b| b.id == id)?; self.vr_joints()?.get(i).copied() }
    /// A bone's world position in her current pose (4 the head).
    fn bone_at(&self, id: i16) -> Option<Vec3> {
        let bank = self.bank.as_ref()?;
        let c = &bank.clips[self.clip];
        let p = sample_retargeted(&bank.bones, c, self.frame.min((c.frames.max(2) - 1) as f32), Retarget::NATIVE);
        let hf = mw_sim::fight::bone_frames(&bank.bones, &p);
        bank.bones.iter().position(|b| b.id == id).map(|i| self.pos + Vec3::Y * self.height + Quat::from_rotation_y(self.yaw) * Vec3::from_array(hf[i].1))
    }
    /// Blood that sticks to her where she is struck [added, as RinRin's em0440 stains]: on her body at the blow, and
    /// spattered over her face from the side it came from (play_fx.rs `stain`).
    fn bloody(&mut self, to: Vec3, dmg: i32, fx: &mut super::fx::Fx) {
        let root = Some(self.roots[self.colored as usize]);
        let r = self.roll() - 0.5;
        let side = Vec3::new(to.z, 0., -to.x);
        let r2 = self.roll() - 0.5;
        fx.stain(root, false, self.body() + Vec3::Y * (0.15 + r * 0.6) + side * r2 * 0.4, to, (0.18 + dmg as f32 / 150.).min(0.32), 0);
        // the face: a smaller spatter, every other blow (and every heavy one), scattered over it
        if dmg >= 25 || self.roll() < 0.5 {
            if let Some(head) = self.bone_at(4) {
                let fwd = Vec3::new(self.yaw.sin(), 0., self.yaw.cos());
                let face = (to + fwd).normalize_or(fwd);
                let r3 = self.roll() - 0.5;
                fx.stain(root, false, head + Vec3::Y * (0.05 + r3 * 0.12) + face * 0.08 + side * r2 * 0.14, face, 0.08 + dmg as f32 / 600., 0);
            }
        }
    }
    pub fn targetable(&self) -> bool { self.on && !matches!(self.st, St::Hide | St::Finisher) && self.hp > 0 || self.st == St::FinishWait }
    pub fn bats_alive(&self) -> usize { self.bats.iter().filter(|b| b.st != BatSt::Dead).count() }

    /// A hit on her (onPreMove 81101E88): guard chips, the damage multipliers, the hide accumulator, the reaction.
    fn hurt(&mut self, dmg: i32, from: Vec3, fx: &mut super::fx::Fx) {
        if self.hp <= 0 || matches!(self.st, St::Hide | St::Keen | St::Dead | St::FinishWait | St::Finisher | St::Victory) { return; }
        let at = self.body();
        let to = Vec3::new(from.x - self.pos.x, 0., from.z - self.pos.z).normalize_or(Vec3::Z);
        let fwd = Vec3::new(self.yaw.sin(), 0., self.yaw.cos());
        let dot = fwd.dot(to);
        if self.st == St::Guard {
            // the guard chips csv(2) %, at least 1, never lethal; the spark (parts effect 0xC)
            let chip = (dmg * csv(2) / 100).max(1).min(self.hp - 1);
            self.hp -= chip; self.hide_acc += dmg; self.guard_hits += 1;
            fx.burst("core", 12, at, to.x.atan2(to.z), 1.);
            self.level_flow(chip as f32);
            return;
        }
        let mult = if matches!(self.st, St::Attack | St::Somersault) { 1.2 } else if dot < -0.7 { 0.8 } else { 1. };
        let dmg = (dmg as f32 * mult) as i32;
        self.voices.push(if dmg >= 20 { vo("hurt_strong", 0xA4) } else { vo("hurt_light", 0xA5) });
        self.hp -= dmg; self.hide_acc += dmg; self.atk_timer -= dmg as f32;
        self.level_flow(dmg as f32);
        fx.gore(at, to.x.atan2(to.z), dmg, dmg >= 25, false);
        self.bloody(to, dmg, fx);
        if self.hp < 1 { self.hp = 1; self.set(St::Dead, "em0630_1005.mot"); return; }
        // attacking she keeps going under frame 40 (60 for the somersault)
        if (self.st == St::Attack && self.frame < 40.) || (self.st == St::Somersault && self.frame < 60.) { return; }
        // a pistol round [added, as Mathilda's]: no stop, the flinch rides over her upper body (shoot)
        if self.shot { return; }
        if self.flying { return; }
        let side = fwd.cross(to).y;
        let clip = if dot.abs() > 0.7 { "em0630_1002.mot" } else if side > 0. { "em0630_1000.mot" } else { "em0630_1001.mot" };
        self.set(St::Damage, clip);
    }

    /// A pistol round from `origin` along `dir`; returns the hit point.
    /// [added, VR] A fist at `at` thrown from `from`: within her body (0.6 m round, a metre up and down) she takes
    /// `damage` through her own hurt (the guard, the flinch, the hide), with the punch's spray.
    pub fn vr_punch(&mut self, at: Vec3, from: Vec3, damage: i32, fx: &mut super::fx::Fx) -> bool {
        if !self.targetable() || self.st == St::FinishWait { return false; }
        let c = self.body();
        if Vec2::new(at.x - c.x, at.z - c.z).length() > 0.62 || (at.y - c.y).abs() > 1.05 { return false; }
        self.hurt(damage, from, fx);
        let yaw = (at.x - from.x).atan2(at.z - from.z);
        fx.impact(at, yaw, 0.9, [1., 0.12, 0.08]); fx.burst("core", 1005, at, yaw, 0.6);
        true
    }
    pub fn shoot(&mut self, origin: Vec3, dir: Vec3, damage: i32, range: f32, fx: &mut super::fx::Fx) -> Option<Vec3> {
        if !self.targetable() { return None; }
        let c = self.body();
        let t = (c - origin).dot(dir);
        if t < 0. || t > range { return None; }
        let p = origin + dir * t;
        if Vec2::new(p.x - c.x, p.z - c.z).length() > 0.6 || (p.y - c.y).abs() > 1.0 { return None; }
        if self.st == St::FinishWait { return Some(p); }
        let hp = self.hp;
        self.shot = true;
        self.hurt(damage, origin, fx);
        self.shot = false;
        if self.hp < hp {
            // [added, as Mathilda's] she keeps moving: her own hit clip (1000 / 1001 by side) over her upper body
            let k = if self.roll() < 0.5 { "em0630_1000.mot" } else { "em0630_1001.mot" };
            self.hurt_walk = self.clips.get(k).map(|&c| (c, 0.));
            // entry and exit holes through her torso where the round crosses it, the spray where it struck, a floor drop
            let flat = Vec3::new(dir.x, 0., dir.z).normalize_or(Vec3::Z);
            let base = self.pos + Vec3::Y * self.height;
            let axis = Vec3::new(base.x, base.y + (p.y - base.y).clamp(0.95, 1.5), base.z);
            let side = Vec3::new(flat.z, 0., -flat.x);
            let across = (p - axis).dot(side).clamp(-0.1, 0.1);
            let entry = axis - flat * 0.12 + side * across;
            fx.wound_holes(Some(self.roots[self.colored as usize]), entry, flat, 0.24);
            fx.burst("core", 1005, p, flat.x.atan2(flat.z) + std::f32::consts::PI, 0.35);
            fx.burst("core", 1005, p + flat * 0.24, flat.x.atan2(flat.z), 0.45);
            if !self.flying { fx.splats.push((p + dir * 0.6, 0.45)); }
        }
        Some(p)
    }
}

/// One 30 Hz game frame (FixedUpdate, after the fight tick).
pub fn step(mut el: ResMut<Elise>, mut play: ResMut<super::Play>, mut fx: ResMut<super::fx::Fx>, assets: Res<AssetServer>, arena: Res<super::Arena>,
            intro: Res<super::intro::Intro>, mut street: ResMut<super::street::Street>, mut editor: ResMut<super::editor::Editor>, mut commands: Commands) {
    if !el.on { play.boss = None; return; }
    let e = &mut *el;
    if !e.spawned {
        e.spawned = true;
        // the eight coffins (bl0015_N001..N008, ObjTbl.obt = CRYPTS[0..8]): static ScrObjs with no motions
        // (tools/stage_static_object.py)
        for &(at, yaw) in &CRYPTS[..8] {
            commands.spawn((super::RoomProp, super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset("native/objects/bl0015.glb"))),
                            Transform::from_translation(Vec3::from_array(at)).with_rotation(Quat::from_rotation_y(yaw)), Visibility::Visible));
        }
        for (k, file) in ["native/elise-source.glb", "native/elise-color.glb"].iter().enumerate() {
            e.roots[k] = commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(*file))), Transform::from_translation(SPAWN), Visibility::Hidden)).id();
        }
        for i in 0..BATS {
            let mut roots = [Entity::PLACEHOLDER; 2];
            for (k, file) in ["native/bat-source.glb", "native/bat-color.glb"].iter().enumerate() {
                roots[k] = commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(*file))), Transform::from_translation(SPAWN), Visibility::Hidden)).id();
            }
            let a = i as f32 * 2.399;
            e.bats.push(Bat { roots, joints: [None, None], pos: SPAWN + Vec3::new(a.cos() * 2., 2. + (i % 7) as f32 * 0.3, a.sin() * 2.), vel: Vec3::ZERO,
                              st: BatSt::Orbit, phase: a, frame: (i % 6) as f32, t: 0., from: Vec3::ZERO, aim: Vec3::ZERO });
        }
        e.set(St::Start, "em0630_0001.mot");
    }
    // inspection: MW_ELISE_TEST=keen | finish starts the clash (her coffin) or the finisher next to the player at tick 40
    if play.tick == 40 {
        let test = std::env::var("MW_ELISE_TEST").unwrap_or_default();
        if test == "keen" || test == "finish" {
            e.pos = Vec3::from_array(play.fight.player.pos) + Vec3::new(0., 0., -1.2);
            e.flying = false; e.height = 0.;
            // MW_ELISE_CRYPT=n: in crypt n instead, the player on the floor 1.5 m in front of it
            if let Some(n) = std::env::var("MW_ELISE_CRYPT").ok().and_then(|v| v.parse::<usize>().ok()).filter(|n| *n < CRYPTS.len()) {
                let (at, yaw) = CRYPTS[n]; e.pos = Vec3::from_array(at); e.yaw = yaw; e.crypt = n;
                let p = e.pos + Vec3::new(yaw.sin(), 0., yaw.cos()) * 1.5;
                let y = super::floor(&arena.0, p + Vec3::Y * 2.).unwrap_or(0.);
                play.fight.player.pos = [p.x, y, p.z];
            }
            // MW_ELISE_AT=x,y,z,yaw[,px,pz]: her hide spot (and his), as a recomp run recorded it
            if let Some(v) = std::env::var("MW_ELISE_AT").ok().map(|v| v.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>()).filter(|v| v.len() >= 4) {
                e.pos = Vec3::new(v[0], v[1], v[2]); e.yaw = v[3];
                if v.len() >= 6 { play.fight.player.pos = [v[4], play.fight.player.pos[1], v[5]]; }
            }
            if test == "keen" { e.set(St::Hide, "em0630_000b.mot"); e.sub = 5; } else { e.hp = 1; e.set(St::FinishWait, "em0630_1006.mot"); }
            // MW_ELISE_STRIKE=1: the swing on her coffin lands now (the recomp fixture's startAtk)
            if test == "keen" && std::env::var_os("MW_ELISE_STRIKE").is_some() { let pl = Vec3::from_array(play.fight.player.pos); start_keen(e, &mut play, &arena, pl); }
        }
    }
    // inspection: MW_ELISE_ATK=attack|somersault|volley|dragon starts that attack 1.6 m in front of the player at tick 60 and
    // every 240 ticks after (MW_ELISE_LOG shows his HP)
    if let Ok(atk) = std::env::var("MW_ELISE_ATK") {
        if play.tick >= 60 && (play.tick - 60) % 240 == 0 && (atk != "hide" || play.tick == 60) {
            let pp = Vec3::from_array(play.fight.player.pos);
            let f = play.fight.player.loco.facing;
            e.pos = pp + Vec3::new(f.sin(), 0., f.cos()) * 1.6; e.yaw = f + std::f32::consts::PI;
            let fly = matches!(atk.as_str(), "volley" | "dragon");
            e.flying = fly; e.height = if fly { FLY_HEIGHT } else { 0. };
            match atk.as_str() {
                "attack" => e.set(St::Attack, "em0630_0111.mot"),
                "somersault" => e.set(St::Somersault, "em0630_0110.mot"),
                "volley" => { e.set(St::Volley, "em0630_0100.mot"); e.volley_hit = false; }
                "dragon" => { e.set(St::Dragon, "em0630_0102.mot"); e.dragon_hp = e.hp; }
                // the hide (and its zako summon), as after csv0 damage
                "hide" => { e.flying = false; e.height = 0.; e.hide_acc = (e.hide_n + 1) * csv(0); e.set(St::Idle, "em0630_0001.mot"); }
                _ => {}
            }
            eprintln!("[elise] test attack {atk} at tick {} player hp {}", play.tick, play.fight.player.hp);
        }
    }
    // inspection: MW_ELISE_SHOOT=1 fires a pistol round at her body from 4 m in front of her every 12 ticks (the shoot path)
    if std::env::var_os("MW_ELISE_SHOOT").is_some() && play.tick > 70 && play.tick % 12 == 0 && e.st != St::Start {
        let c = e.body();
        let from = c + Vec3::new(e.yaw.sin(), 0., e.yaw.cos()) * 4.;
        let hp = e.hp;
        let hit = e.shoot(from, (c - from).normalize(), 20, f32::INFINITY, &mut fx);
        eprintln!("[elise] test shot tick {} hit {} hp {} -> {} state {:?} flinch {}", play.tick, hit.is_some(), hp, e.hp, e.st, e.hurt_walk.is_some());
    }
    // inspection: MW_ELISE_TEST=hide sends her into a crypt at tick 40 and every 200 ticks after (MW_ELISE_LOG names it)
    if play.tick >= 40 && (play.tick - 40) % 200 == 0 && std::env::var("MW_ELISE_TEST").as_deref() == Ok("hide") {
        e.flying = false; e.height = 0.; e.set(St::Hide, "em0630_0002.mot");
    }
    if std::env::var_os("MW_ELISE_LOG").is_some() && e.st == St::Hide && e.sub == 5 && e.timer == 1. {
        eprintln!("[elise] tick {} hides in crypt {} at {:.2} yaw {:.2}", play.tick, e.crypt, e.pos, e.yaw);
    }
    // the actor editor: her spot on the first frame, its commands, what it shows
    let frozen = editor.on && !editor.ai;
    if editor.on {
        if !e.ed_ready || e.ed_reset { e.ed_ready = true; e.ed_reset = false; let (s, ps) = (editor.spot, editor.player_spot); editor_reset(e, &mut play, &arena, s, ps); }
        if let Some(cmd) = editor.take() { let (s, ps) = (editor.spot, editor.player_spot); let r = editor_cmd(e, &cmd, &mut play, &arena, &mut fx, s, ps); editor.done(&cmd, r); }
        // a camera scene: her clock and origin; over, his clip ends and both reset
        if e.ed_scene {
            if e.st == St::Editor { editor.scene = Some((e.frame, e.pos, e.yaw)); }
            else { e.ed_scene = false; editor.scene = None; e.ed_reset = true; }
        }
        editor.anchor = (e.pos, e.yaw);
        editor.idle = matches!(e.st, St::Idle | St::Move | St::Start) && !play.fight.paired && e.exec == 0;
        // (in words: the clip, her state, and the clash's phase)
        let name = e.bank.as_ref().map_or(String::new(), |b| super::editor::clip_label("em0630", &b.clips[e.clip].name));
        let st = super::editor::phase_label("em0630", "state", format!("{:?}", e.st));
        let phase = if e.st == St::Keen { format!(": {}", super::editor::phase_label("em0630", "Keen", e.sub)) } else { String::new() };
        editor.info = format!("{name}   {:.0} / {:.0}   {st}{phase}   HP {}", e.frame, e.len() - 1., e.hp);
        // held: no hide from damage, no attack clock
        if frozen { e.hide_acc = e.hide_acc.min((e.hide_n + 1) * csv(0) - 1); e.atk_timer = e.atk_timer.max(60.); }
    }
    // his blood round her mouth after a bite [added]: a smear and drips down her chin, upright again
    // (a few frames into her idle, so the pose has settled out of the bite)
    if e.mouth_due && matches!(e.st, St::Idle | St::Move | St::Editor) && !e.flying && e.frame >= 8. {
        if let Some(head) = e.bone_at(4) {
            e.mouth_due = false;
            let fwd = Vec3::new(e.yaw.sin(), 0., e.yaw.cos());
            // (bone 4 sits at her eyes; the smear's band is at 0.4 of the image, the drips under it)
            fx.mark(Some(e.roots[e.colored as usize]), false, head - Vec3::Y * 0.045 + fwd * 0.1, fwd, 0.1, "mouth-blood", 0);
        }
    }
    if let Some((side, n)) = e.neck_due {
        let st = play.fight.player.state();
        let up = !play.fight.paired && !matches!(st, 94 | 95) && st != mw_sim::fight::STATE_EXTERNAL;
        e.neck_due = Some((side, if up { n + 1 } else { 0 }));
        if up && n >= 8 {
            e.neck_due = None;
            let p = &play.fight.player;
            let clip = &play.fd.jack.clips[p.motion.clip];
            let poses = sample_retargeted(&play.fd.jack.bones, clip, p.motion.frame.min((clip.frames.max(2) - 1) as f32), Retarget::NATIVE);
            let jf = mw_sim::fight::bone_frames(&play.fd.jack.bones, &poses);
            let q = Quat::from_rotation_y(p.loco.facing);
            if let Some(i) = play.fd.jack.bones.iter().position(|b| b.id == 3) {
                let neck = Vec3::from_array(p.pos) + q * Vec3::from_array(jf[i].1);
                let out = q * Vec3::new(-side, 0., 0.3).normalize();
                fx.mark(None, true, neck + out * 0.05 + Vec3::Y * 0.01, out, 0.12, "bite-marks", 0);
            }
        }
    }
    let pl = Vec3::from_array(play.fight.player.pos);
    let chest = pl + Vec3::Y * 1.2;
    let to = Vec3::new(pl.x - e.pos.x, 0., pl.z - e.pos.z);
    let dist = to.length();
    let face = |e: &mut Elise, k: f32| { let want = to.x.atan2(to.z); e.yaw = mw_sim::player::rad_lerp_range(e.yaw, want, k, 0.4); };
    let player_alive = play.fight.player.hp > 0;
    start_hers(&mut play, e, &arena);
    // the bite's hit-stops (BITE_SLOW): her clip and his run at the ramp's rate
    let step = if e.slow > 0 { BITE_RAMP[e.slow - 1] } else { 1. };
    if e.slow > 0 { e.slow = if e.slow >= BITE_RAMP.len() { 0 } else { e.slow + 1 }; }
    e.frame += step;
    // (a clash clip that has played out holds its last frame while his catches up)
    if e.st == St::Keen && e.sub <= 1 && e.her_next.is_none() { e.frame = e.frame.min(e.len() - 1.); }
    e.last_step = step;
    e.colored = play.colored;
    if let Some((c, f)) = e.hurt_walk {
        let n = e.bank.as_ref().map_or(1., |b| (b.clips[c].frames.max(2) - 1) as f32);
        e.hurt_walk = (f + 1. < n && !matches!(e.st, St::Keen | St::Hide | St::Dead | St::FinishWait | St::Finisher)).then_some((c, f + 1.));
    }
    let len = e.len();
    let ended = e.frame >= len - 1.;
    if e.exec > 0 { step_execution(e, &mut play, &arena, ended); bite_drain(e, &mut play, &mut fx); bite_slow(e, &mut play); step_bats(e, &mut play, chest, &mut fx); return; }

    // the player's swing: on her (or her coffin), and on the bats in its reach
    if let Some((hit, serial)) = play.fight.player_strike() {
        let c = Vec3::from_array(hit.center);
        for b in e.bats.iter_mut().filter(|b| b.st != BatSt::Dead) {
            if b.pos.distance(c) < hit.radius + 0.35 { b.st = BatSt::Dead; b.vel = (b.pos - c).normalize_or(Vec3::Y) * 0.08; fx.burst("core", 1005, b.pos, 0., 0.5); }
        }
        if serial != e.hit_by {
            let body = e.body();
            let reach = hit.radius + 0.6;
            if e.st == St::Hide && e.sub >= 5 && Vec2::new(c.x - e.pos.x, c.z - e.pos.z).length() < reach + 0.6 {
                // startAtk on her coffin: the clash (startKeenCompetition mode 0x18). Phase 0 of the player's clash code
                // fn_802A03A0 (recomp 802A0414..802A0718): she stays in her crypt, turned to the room unless she is on the
                // centre line or past z -12; GetThrowOffset(em0630_0206.mot, bone 1024) = (0, 0, 1.618) puts him in front
                // of her, facing her (her yaw + pi), at the case's height
                e.hit_by = serial;
                start_keen(e, &mut play, &arena, pl);
            } else if e.st == St::FinishWait && Vec2::new(c.x - e.pos.x, c.z - e.pos.z).length() < reach + 0.5 {
                // the finisher: 802A4830 gives 0x20630 type 8 -> 802A9830: the player blends into pl0030_0000 (blend 2)
                // and startCatch(0x2F) makes her a corpse (81102F24: setDeadTime / setCorpse); the room's 80133AA4 wait
                // then ends and fn_80160278 plays ev34
                e.hit_by = serial;
                e.set(St::Finisher, "em0630_1006.mot");
                let pp = &mut *play; pp.fight.external(&pp.fd, "pl0030_0000.mot", false, 2.);
            } else if Vec2::new(c.x - body.x, c.z - body.z).length() < reach && (c.y - body.y).abs() < 1.3 {
                e.hit_by = serial;
                // the guard entry (startAtk 81102B14): idle / move facing the swing, or early in her own attack
                let fwd = Vec3::new(e.yaw.sin(), 0., e.yaw.cos());
                let facing = fwd.dot(to.normalize_or_zero()) > 0.25;
                let l = e.l();
                if !e.flying && ((matches!(e.st, St::Idle | St::Move) && (facing || e.roll() < 0.3))
                    || (matches!(e.st, St::Attack | St::Somersault) && e.frame <= 15. * l * l)) {
                    e.set(St::Guard, "em0630_0020.mot"); e.guard_hits = 0;
                    e.guard_cancel = if l < 0.2 { 4 + (e.roll() * 5.) as u32 } else if l < 0.5 { 3 + (e.roll() * 3.) as u32 } else if l < 0.8 { 2 } else { 1 };
                }
                e.hurt(hit.power, pl, &mut fx);
                play.fight.hitstop = play.fight.hitstop.max(hit.hitstop);
            }
        }
    }
    // hide every csv(0) accumulated damage
    if matches!(e.st, St::Idle | St::Guard | St::Damage) && e.hide_acc >= (e.hide_n + 1) * csv(0) {
        e.hide_n += 1;
        e.set(St::Hide, "em0630_0002.mot");
    }
    let l = e.l();
    match e.st {
        // the editor holds her: her idle (in the air or on the ground), turned to the player
        St::Idle | St::Move | St::Start if frozen => {
            let idle = if e.flying { "em0630_0000.mot" } else { "em0630_0001.mot" };
            if e.st != St::Idle || e.clips.get(idle) != Some(&e.clip) { e.set(St::Idle, idle); }
            if ended { e.frame = 0.; }
            face(e, 0.08);
        }
        St::Editor => { if ended { if e.ed_loop { e.frame = 0.; } else { e.set(St::Idle, if e.flying { "em0630_0000.mot" } else { "em0630_0001.mot" }); } } }
        St::Start => { face(e, 0.1); if ended { e.frame = 0.; } if dist < 12.5 { e.set(St::Air, "em0630_0003.mot"); } }
        St::Idle | St::Move => {
            e.atk_timer -= 1.;
            if e.flying {
                if ended { e.frame = 0.; }
                face(e, 0.1);
                if e.st == St::Move {
                    // the flight (81104014): a 3-6 m hop eased by sin(2t - t^3), still facing the player
                    e.timer += 1.;
                    let t = (e.timer / e.fly_len).min(1.);
                    let s = (2. * t - t * t * t).sin() / 1f32.sin();
                    e.pos = e.fly_from.lerp(e.target, s);
                    if t >= 1. { e.set(St::Idle, "em0630_0000.mot"); e.timer = 30. + e.roll() * 60.; }
                } else {
                    // the hover 0000; then the next hop inside the arena bins x -3..3, z -8..23, its clip by the hop's
                    // direction against her facing: 0011 ahead (dot > 0.5), 0014 back (dot < -0.5), 0012 / 0013 to a side
                    e.timer -= 1.;
                    if e.timer <= 0. {
                        let a = e.roll() * std::f32::consts::TAU;
                        let r = 3. + e.roll() * 3.;
                        let to_pt = Vec3::new((e.pos.x + a.sin() * r).clamp(-3., 3.), e.pos.y, (e.pos.z + a.cos() * r).clamp(-8., 23.));
                        let d = Vec3::new(to_pt.x - e.pos.x, 0., to_pt.z - e.pos.z);
                        if d.length() > 1. {
                            let (dir, fwd) = (d.normalize(), Vec3::new(e.yaw.sin(), 0., e.yaw.cos()));
                            let (dot, cross) = (fwd.dot(dir), fwd.x * dir.z - fwd.z * dir.x);
                            let clip = if dot > 0.5 { "em0630_0011.mot" } else if dot < -0.5 { "em0630_0014.mot" } else if cross > 0. { "em0630_0012.mot" } else { "em0630_0013.mot" };
                            e.set(St::Move, clip);
                            e.fly_from = e.pos; e.target = to_pt; e.fly_len = 20. + d.length() * 6.;
                        }
                        e.timer = 0.;
                    }
                }
                e.rebirth -= 1.;
                if e.bats_alive() < 31 || e.rebirth <= 0. { e.set(St::Air, "em0630_0002.mot"); }
                else if e.atk_timer <= 0. {
                    e.atk_count += 1; e.atk_timer = e.attack_interval();
                    let need = if l > 0.6 { 1 } else if l > 0.4 { 2 } else if l > 0.2 { 3 } else { 4 };
                    if e.atk_count >= need && e.bats_alive() >= 50 { e.atk_count = 0; e.set(St::Dragon, "em0630_0102.mot"); e.dragon_hp = e.hp; }
                    else { let k = if e.roll() < 0.5 { "em0630_0100.mot" } else { "em0630_0101.mot" }; e.set(St::Volley, k); e.volley_hit = false; }
                }
            } else {
                face(e, 0.2);
                e.rebirth -= if dist < 5. { 0.1 } else { 1. };
                // chkAtkStart runs in idle and in the walk (dist² < 16): with her attack due she walks in rather than
                // waiting between 4 and 5 m; the wing strike reaches 2.3 m (its 1.16 m lunge stays in place), so she closes to 1.9 m
                let ready = e.atk_timer <= 0. && player_alive;
                if e.rebirth <= 0. { e.set(St::Air, "em0630_0003.mot"); }
                else if ready && dist < 2.2 {
                    e.atk_timer = e.attack_interval();
                    if l * l * l > e.roll() { e.set(St::Somersault, "em0630_0110.mot"); } else { e.set(St::Attack, "em0630_0111.mot"); e.voices.push(vo("wing_strike", 0x9D)); }
                }
                else if dist > 5. || (e.st == St::Move && dist > 2.) || ready {
                    if e.st != St::Move { e.set(St::Move, "em0630_0010.mot"); }
                    if ended { e.frame = 0.; }
                    let step = to.normalize_or_zero() * (1.2 / 30.);
                    let (moved, _) = mw_sim::collision::slide(&arena.0, e.pos.to_array(), [step.x, step.z]);
                    e.pos.x += moved[0]; e.pos.z += moved[1];
                } else {
                    if e.st != St::Idle { e.set(St::Idle, "em0630_0001.mot"); }
                    if ended { e.frame = 0.; }
                }
            }
        }
        // take-off / landing: the flag flips; landing re-summons the bats and rolls the next take-off
        St::Air => {
            let landing = e.flying;
            e.height = if landing { FLY_HEIGHT * (1. - (e.frame / len).min(1.)) } else { FLY_HEIGHT * ((e.frame - 30.) / (len - 30.)).clamp(0., 1.) };
            if landing && e.frame >= 40. && e.sub == 0 {
                e.sub = 1; e.voices.push(vo("landing", 0xA0));
                for b in e.bats.iter_mut().filter(|b| !matches!(b.st, BatSt::Summon(_))) { b.st = BatSt::Orbit; }
                e.rebirth = 150. + e.roll() * 150.;
            }
            if ended { e.flying = !landing; e.height = if e.flying { FLY_HEIGHT } else { 0. }; e.timer = 0.; e.set(St::Idle, if e.flying { "em0630_0000.mot" } else { "em0630_0001.mot" }); }
        }
        // the wing strike (0111: first strike before frame 25, second after; rig windows 14-19 and 28-34)
        St::Attack => {
            if e.frame < 12. { face(e, 0.25); }
            for (w, (a, b)) in [(14., 19.), (28., 34.)].iter().enumerate() {
                // (the wing strike's cape-wings reach past her hands: WING_HIT)
                if (*a..*b).contains(&e.frame) && e.landed & (1 << w) == 0 && limb_reach(e, pl) < WING_HIT {
                    let atk = if w == 0 { if l > 0.4 { 0x141 } else { 0x172 } } else { 0x151 };
                    let p = &mut *play;
                    if p.fight.enemy_hits_player(&p.fd, atk, power(w), e.pos.to_array()) { e.landed |= 1 << w; }
                }
            }
            if ended { if e.roll() < 0.4 { e.set(St::Somersault, "em0630_0110.mot"); } else { e.set(St::Idle, "em0630_0001.mot"); } }
        }
        // the somersault (0110, both feet, rig window 20-26; ATHIT 0x18E as the guard cancel)
        St::Somersault => {
            if e.frame < 10. { face(e, 0.25); }
            if (20. ..26.).contains(&e.frame) && e.landed == 0 && limb_reach(e, pl) < LIMB_HIT {
                let cancel = e.sub == 1;
                let p = &mut *play;
                if p.fight.enemy_hits_player(&p.fd, if cancel { 0x18E } else { 0x155 }, power(if cancel { 3 } else { 2 }), e.pos.to_array()) { e.landed = 1; }
            }
            if ended { e.set(St::Idle, "em0630_0001.mot"); }
        }
        // the air volley: bats dive at the player (cmd 6 at 0100 frame 40 / cmd 7 at 0101 frame 35)
        St::Volley => {
            face(e, 0.1);
            let launch = if e.clips.get("em0630_0100.mot") == Some(&e.clip) { 40. } else { 35. };
            if e.frame >= launch && e.sub == 0 {
                e.sub = 1; e.voices.push(vo("bats_answer", 0x9F));
                let mut n = 0;
                for b in e.bats.iter_mut().filter(|b| b.st == BatSt::Orbit) {
                    if n >= 14 { break; }
                    b.st = BatSt::Dive; b.t = 0.; b.vel = (chest - b.pos).normalize_or(Vec3::Z) * 0.3; n += 1;
                }
            }
            if ended { e.set(St::Idle, "em0630_0000.mot"); }
        }
        // Black Dragon: cast 0102, rise into 0103, aim 0104 (the heads chase him), recall 0105
        St::Dragon => {
            face(e, 0.1);
            match e.sub {
                0 if ended => {
                    e.sub = 1; e.play("em0630_0103.mot");
                    let heads = if e.roll() < 2. * l - 0.3 { 2 } else { 1 };
                    e.loops = (6. * (0.6 + 0.4 * e.roll() * l)).min(5.) as u32;
                    e.heads = (0..heads).map(|k| (e.body() + Vec3::Y * 1.5 + Vec3::X * (k as f32 * 2. - 1.), Vec3::ZERO, 0, false)).collect();
                    let n = e.heads.len();
                    for (k, b) in e.bats.iter_mut().filter(|b| b.st == BatSt::Orbit).enumerate() { b.st = BatSt::Head(k % n); b.t = k as f32; }
                }
                1 => {
                    if ended { e.frame = 0.; }
                    // each head steers toward the player (the aiming gain), passes him, swings round for its loops
                    let aim = 0.1 * (1. - l) + 0.6 * (1. - l * l);
                    let mut done = true;
                    for h in e.heads.iter_mut() {
                        let want = (chest - h.0).normalize_or(Vec3::Z);
                        let speed = 0.12 + 0.04 * (h.2 as f32);
                        h.1 = (h.1 * (1. - aim * 0.12) + want * speed * aim * 0.12).clamp_length_max(speed);
                        if h.1.length() < 0.02 { h.1 = want * speed; }
                        h.0 += h.1;
                        if h.0.distance(chest) < 3.75 && !h.3 && play.tick % 45 == 0 { e.voices.push(vo("dragon_hint", 0xA3)); }
                        // (contact at the head's drawn size: a knot of bats about 1.2 m across)
                        if h.0.distance(chest) < 1.3 && !h.3 {
                            h.3 = true;
                            let p = &mut *play;
                            p.fight.enemy_hits_player(&p.fd, 0x15B, power(4), h.0.to_array());
                        }
                        if h.0.distance(chest) > 6. && h.3 { h.3 = false; h.2 += 1; }
                        if h.2 < e.loops { done = false; }
                    }
                    if done || e.bats_alive() < 50 || e.dragon_hp - e.hp >= (70. + 30. * l) as i32 {
                        let fall = e.dragon_hp - e.hp >= (70. + 30. * l) as i32;
                        e.heads.clear();
                        for b in e.bats.iter_mut() { if matches!(b.st, BatSt::Head(_)) { b.st = BatSt::Orbit; } }
                        if fall { e.flying = false; e.height = 0.; e.set(St::Down, "em0440_1003.mot"); } else { e.sub = 2; e.play("em0630_0105.mot"); }
                    }
                }
                2 if ended => { e.set(St::Idle, "em0630_0000.mot"); }
                _ => {}
            }
        }
        // guard (0020): enough blocked hits -> the guard-cancel somersault
        St::Guard => {
            face(e, 0.2);
            if e.guard_hits >= e.guard_cancel { e.set(St::Somersault, "em0630_0110.mot"); e.sub = 1; e.frame = 15.; }
            else if ended { e.set(St::Idle, "em0630_0001.mot"); }
        }
        St::Damage => { if ended { e.set(St::Idle, "em0630_0001.mot"); } }
        St::Down => { if ended { if e.clips.get("em0440_1003.mot") == Some(&e.clip) { e.play("em0630_1004.mot"); } else { e.set(St::Idle, "em0630_0001.mot"); } } }
        // hide: down into the crypt, the coffin pose; heal after csv(4) s by 1 HP a csv(5) s; hint bats on it
        St::Hide => {
            match e.sub {
                0 if ended => {
                    e.sub = 5; e.timer = 0.;
                    for b in e.bats.iter_mut().filter(|b| b.st != BatSt::Dead) { b.st = BatSt::Orbit; }
                    // getCrypticPoint 80075C0C: r = U[0, 300]; a coffin (r & 7) after another crypt, any of the 11
                    // (r % 11) after one coffin, never three coffins in a row (r % 3 + 8)
                    let r = (e.roll() * 301.) as usize;
                    e.crypt = if e.crypt_run[0] >= 2 { r % 3 + 8 } else if e.crypt_run[1] == 0 { r % 11 } else { r & 7 };
                    e.crypt_run = if e.crypt < 8 { [e.crypt_run[0] + 1, 0] } else { [0, e.crypt_run[1] + 1] };
                    let (at, yaw) = CRYPTS[e.crypt]; e.pos = Vec3::from_array(at); e.yaw = yaw; e.flying = false; e.height = 0.;
                    // the pose (81107314 sub 1): 000b / 000c / 000e at random in a coffin, 000a at crypts 8-10
                    let pose = if e.crypt >= 8 { "em0630_000a.mot" } else { ["em0630_000b.mot", "em0630_000c.mot", "em0630_000e.mot"][(e.roll() * 3.) as usize % 3] };
                    e.play(pose);
                    // em0630_r1_hide asks for 0xAC as she settles into the coffin (81107840); c303's pack has no take for it
                    e.voices.push(vo("hide", 0xAC));
                    let mut n = 0;
                    for b in e.bats.iter_mut().filter(|b| b.st == BatSt::Orbit) { if n >= 6 { break; } b.st = BatSt::Hint; n += 1; }
                    summon(e, Vec3::from_array(play.fight.player.pos));
                }
                5 => {
                    if ended { e.frame = len - 1.; }
                    e.timer += 1.;
                    if e.timer >= 30. * csv(4) as f32 { e.timer -= 30. * csv(5) as f32; e.hp = (e.hp + 1).min(max_hp()); e.level_flow(-1.); }
                }
                _ => {}
            }
        }
        // the clash: pulled out, start, the shake meter, the mid-win and the swing sequence, then won or bitten
        St::Keen => {
            start_his(&mut play, e, &arena);
            // (the editor can force the outcome: shaking all through and every swing right, or nothing)
            let shake = match e.auto { Some(w) => w, None => play.latched_shake() };
            let (q_v, q_s) = if e.auto.is_some() { (false, false) } else { play.swing_keys() };
            match e.sub {
                // pull-out frame 23 (802A0760): the far crypts' pairs move to a floor spot, his yaw set, her offset kept
                // (the live recomp moves them on the update his frame reaches 24)
                // (the player's clips drive the phases: her finished clip holds its last frame until his ends too; the live
                // recomp shows her 0206 / 0200 last frame twice, his one behind)
                0 if !(ended && his_last(&play)) => {
                    if !e.keen_moved && play.fight.player.motion.frame >= 24. { e.keen_moved = true; keen_move(&mut play, e); }
                    // the clash cut-in when his pull-out (pl0030_0466) reaches frame 25 (802A03A0 phase 1)
                    if !e.cutin_done && play.fight.player.motion.frame >= 25. && e.exec == 0 { e.cutin_done = true; super::cutin::request(); }
                }
                // pull-out over (802A0B60): he stands at (0, 0, 6) facing -z, she at his spot less
                // GetThrowOffset(pl0030_0460, bone 1024) unrotated, facing +z; both 0.1 up
                0 => {
                    e.sub = 1; e.voices.push(vo("clash_start", 0xA7));
                    pair_next(&mut play, e, "em0630_0200.mot", "pl0030_0460.mot", false);
                }
                // into the struggle (start_his places her from 0461's throw offset)
                1 if ended && his_last(&play) => { e.sub = 2; pair_next(&mut play, e, "em0630_0201.mot", "pl0030_0461.mot", true); }
                2 => {
                    if ended { e.frame = 0.; }
                    // the struggle leans with the smoothed meter: her 0201 toward 0202, his 0461 toward 0462
                    let w = (e.meter / 30.).clamp(0., 1.);
                    e.lean += (w - e.lean) * 0.15;
                    play.fight.player.loco.layer = Some(("pl0030_0462.mot", e.lean));
                    // the meter: up 0.04 x (remote + nunchuk) while shaking, down otherwise; full at 30, empty loses
                    e.meter = (e.meter + if shake { 0.04 * 2. * 1.59 } else { -0.05 }).clamp(0., 30.);
                    e.prompt = Some(format!("TAP A/D!  {}%", (e.meter / 30. * 100.) as i32));
                    if e.meter >= 30. { e.sub = 3; pair_next(&mut play, e, "em0630_0205.mot", "pl0030_0465.mot", false); e.qte = 0; e.timer = 0.; }
                    else if e.meter <= 0. { e.sub = 6; e.voices.push(vo("bite", 0xA9)); pair_next(&mut play, e, "em0630_0204.mot", "pl0030_0464.mot", false); e.prompt = None; }
                }
                3 => {
                    // after frame 110: the swing sequence (cIdPlCockpit::setSwingNavigation): three swings in time
                    if e.frame >= 110. {
                        if e.qte == 0 { e.qte_keys = [e.roll() < 0.5, e.roll() < 0.5, e.roll() < 0.5]; e.qte = 1; e.timer = 0.; }
                        let want_v = e.qte_keys[(e.qte - 1).min(2)];
                        let (q_v, q_s) = if e.auto == Some(true) && e.timer > 12. { (want_v, !want_v) } else { (q_v, q_s) };
                        e.prompt = Some(format!("SWING {} [{}]  {}/3", if want_v { "DOWN" } else { "SIDE" }, if want_v { "F" } else { "Q" }, e.qte - 1));
                        e.timer += 1.;
                        if (want_v && q_v) || (!want_v && q_s) { e.qte += 1; e.timer = 0.; }
                        else if q_v || q_s || e.timer > 45. { e.sub = 6; e.voices.push(vo("bite", 0xA9)); pair_next(&mut play, e, "em0630_0204.mot", "pl0030_0464.mot", false); e.prompt = None; }
                        if e.qte > 3 { e.sub = 4; e.voices.push(vo("clash_won", 0xA8)); pair_next(&mut play, e, "em0630_0203.mot", "pl0030_0463.mot", false); e.prompt = None; }
                    }
                    if ended && e.sub == 3 { e.frame = len - 1.; }
                }
                // won: at frame 62 the blow (hit-stop); at the end -csv(6) HP, down or finish_wait
                4 => {
                    if e.frame >= 62. && e.landed == 0 { e.landed = 1; play.fight.hitstop = play.fight.hitstop.max(8); fx.impact(e.body(), e.yaw, 1.5, [1., 0.12, 0.08]); }
                    if ended {
                        e.hp -= csv(6); e.level_flow(csv(6) as f32); end_pair(&mut play, e);
                        if e.hp < 1 { e.hp = 1; e.set(St::FinishWait, "em0630_1006.mot"); } else { e.set(St::Down, "em0440_1003.mot"); }
                    }
                }
                // bitten: she heals csv(1) over frames 40-100
                6 => {
                    // his 0464 ends first (190 frames to her 202): he leaves the pair into the get-up (the live recomp:
                    // state 94, pl0030_1092) while she finishes
                    if play.fight.paired && play.fight.external_done() && e.his_next.is_none() {
                        keep_his(&mut play); play.fight.paired = false; play.fight.get_up();
                    }
                    if (40. ..100.).contains(&e.frame) { e.hp = (e.hp + (csv(1) as f32 / 60.).ceil() as i32).min(max_hp()); e.level_flow(-(csv(1) as f32) / 60.); }
                    bite_drain(e, &mut play, &mut fx);
                    if ended { end_pair(&mut play, e); e.set(St::Idle, "em0630_0001.mot"); }
                }
                _ => {}
            }
        }
        St::Dead => { if ended { e.set(St::FinishWait, "em0630_1006.mot"); } }
        St::FinishWait => { if ended { e.frame = 0.; } }
        // the corpse: no boss-class enemy is left (80133BF0), so the room goes on to ev34; the player's 0000 plays out
        // after ev34 roomClear runs the clear sequence 80124258: announcer 0x1027 (its PlayBGM_EX(0x26) is skipped in
        // room 0x303: ev34's JINGLE05 plays on), the clear window, then scene 0xB14 (805A445C)
        // (in the editor the finisher plays its clips and resets: no outro, no room clear)
        St::Finisher if editor.on => {
            if play.fight.external_done() { play.fight.end_external(); }
            if ended { e.ed_reset = true; }
        }
        St::Finisher => {
            if ended { e.frame = 0.; }
            if e.sub == 0 { e.sub = 1; play.fight.victory = true; }
            if play.fight.external_done() { play.fight.end_external(); }
            if e.sub == 1 && !intro.outro_pending() && !intro.active() { e.sub = 2; e.voices.push(vo("clear", 0x1027)); }
        }
        // victory (state 15, voice 0xAB): she laughs (0104), takes off (0003) and circles over his body (the flight
        // clips 0011-0014 by her heading, VICTORY_RADIUS out), stopping in the air to laugh again every few seconds; when
        // he continues, 40 frames on she is back in the fight (sub 5) [the flight circle is added: natively she walks up]
        St::Victory => {
            let centre = Vec3::new(pl.x, e.pos.y, pl.z);
            if player_alive {
                if e.sub != 5 { e.sub = 5; e.timer = 40.; }
                e.timer -= 1.;
                if e.timer <= 0. { e.flying = e.height > 0.5; e.set(St::Idle, if e.flying { "em0630_0000.mot" } else { "em0630_0001.mot" }); e.timer = 0.; }
            } else {
                match e.sub {
                    // the laugh, then the take-off
                    0 => { if ended { e.sub = 1; e.play("em0630_0003.mot"); } }
                    1 => {
                        e.height = FLY_HEIGHT * ((e.frame - 30.) / (len - 30.)).clamp(0., 1.);
                        if ended { e.flying = true; e.sub = 2; e.timer = 90. + e.roll() * 60.; e.play("em0630_0011.mot"); }
                    }
                    // circling him
                    2 => {
                        let r = Vec3::new(e.pos.x - centre.x, 0., e.pos.z - centre.z);
                        let ang = r.z.atan2(r.x) + 0.035;
                        let want = centre + Vec3::new(ang.cos(), 0., ang.sin()) * VICTORY_RADIUS;
                        let step = Vec3::new(want.x - e.pos.x, 0., want.z - e.pos.z);
                        e.pos += step.clamp_length_max(0.14);
                        e.yaw = step.x.atan2(step.z);
                        if ended { e.frame = 0.; }
                        e.timer -= 1.;
                        if e.timer <= 0. { e.sub = 3; e.yaw = (pl.x - e.pos.x).atan2(pl.z - e.pos.z); e.play("em0630_0104.mot"); e.voices.push(vo("victory", 0xAB)); }
                    }
                    // hovering to laugh at him, then on around
                    _ => { if ended { e.sub = 2; e.timer = 90. + e.roll() * 60.; e.play("em0630_0011.mot"); } }
                }
            }
        }
    }
    bite_slow(e, &mut play);
    // her melee strikes travel as their clips do (the original moves the object by the root step; 0111 lunges 1.16 m, 0110
    // steps back and in): the frame's root step turned by her yaw, slid along the walls
    if matches!(e.st, St::Attack | St::Somersault) && !e.flying && e.frame >= 1. {
        if let Some(bank) = e.bank.as_ref() {
            let c = &bank.clips[e.clip];
            let n = (c.frames.max(2) - 1) as f32;
            let (r0, r1) = (mw_sim::character::root_xz(&bank.bones, c, (e.frame - 1.).min(n)), mw_sim::character::root_xz(&bank.bones, c, e.frame.min(n)));
            let d = Quat::from_rotation_y(e.yaw) * Vec3::new(r1[0] - r0[0], 0., r1[1] - r0[1]);
            if d.length() < 0.5 {
                let (m, _) = mw_sim::collision::slide(&arena.0, e.pos.to_array(), [d.x, d.z]);
                e.pos.x += m[0]; e.pos.z += m[1];
            }
        }
    }
    if !player_alive && !matches!(e.st, St::Victory | St::Dead | St::FinishWait | St::Finisher) {
        e.heads.clear();
        // (on the ground first: the laugh, then the take-off; already flying she laughs in the air and circles)
        if e.flying { e.set(St::Victory, "em0630_0104.mot"); e.sub = 3; } else { e.height = 0.; e.set(St::Victory, "em0630_0104.mot"); }
        e.voices.push(vo("victory", 0xAB));
    }
    // bodies do not overlap while she is on the ground (CharColRadius: the player 0.4, her ~0.35): each gives half [added,
    // as the Baron's]; not in her pairs, a crypt, or the finisher
    if e.height < 0.6 && !play.fight.paired && !matches!(e.st, St::Hide | St::Keen | St::Finisher | St::Dead | St::FinishWait) {
        let d = Vec3::new(pl.x - e.pos.x, 0., pl.z - e.pos.z);
        const MIN: f32 = 0.75;
        if d.length() < MIN {
            let push = d.normalize_or(Vec3::Z) * (MIN - d.length()) * 0.5;
            e.pos -= push;
            let (m, _) = mw_sim::collision::slide(&arena.0, play.fight.player.pos, [push.x, push.z]);
            play.fight.player.pos[0] += m[0]; play.fight.player.pos[2] += m[1];
        }
    }
    // flight bobbing; on the ground she stands on the floor
    if e.flying && e.st != St::Air { e.height = FLY_HEIGHT + 0.15 * (play.tick as f32 * 0.08).sin(); }
    // (in a crypt she keeps the crypt's height: getCrypticPoint sets her position, collision off)
    // (not in a crypt, nor through a raised crypt's pull-out until the pair is moved down: both stand at the platform)
    if e.st != St::Hide && !(e.st == St::Keen && e.keen_case != 0 && !e.keen_moved) { if let Some(y) = super::floor(&arena.0, e.pos + Vec3::Y * 0.5) { e.pos.y = y; } }
    // her death sends the swarm away (dead 81107C8C sub 0: bat cmd 4; via the clash the hide's cmd 4 already did)
    if matches!(e.st, St::Dead | St::FinishWait | St::Finisher) && !e.scattered { e.scattered = true; scatter(e); }
    step_bats(e, &mut play, chest, &mut fx);
    step_drops(e, &mut fx);
    // inspection: MW_ZAKO_TEST=1 drops three of her brood 3-4 m in front of the player at tick 40
    if play.tick == 40 && std::env::var_os("MW_ZAKO_TEST").is_some() {
        let pl = Vec3::from_array(play.fight.player.pos); let f = play.fight.player.loco.facing;
        for k in 0..3 { e.zako.push((pl + Quat::from_rotation_y(f + (k as f32 - 1.) * 0.5) * Vec3::Z * 3.5, 0.)); }
    }
    // the zako the drops asked for (play_street.rs's em0101: alternate outfits, facing the player)
    for (at, _) in e.zako.drain(..) {
        let pl = Vec3::from_array(play.fight.player.pos);
        let outfit = (street.queue.len() + street.kills as usize) % 2;
        // her brood: the three vampire women in turn (tools/vampzako; MW_VAMPZAKO=0 sends the kung fu grunts instead)
        street.brood += 1;
        let n = street.brood;
        let body = if std::env::var("MW_VAMPZAKO").as_deref() == Ok("0") { 0 } else { 1 + (n % 3) as u8 };
        street.queue.push((at, (pl.x - at.x).atan2(pl.z - at.z), outfit, body));
    }
    // inspection: MW_ELISE_ROWS=1 prints the clash per frame as the recomp fixture's ELISE_CLASH_ROW (world roots)
    if std::env::var_os("MW_ELISE_ROWS").is_some() && (e.st == St::Keen || e.keen_tail > 0) {
        if e.st == St::Keen { e.keen_tail = 90; } else { e.keen_tail -= 1; }
        let her = e.pos + Quat::from_rotation_y(e.yaw) * root_at(e.bank.as_ref().unwrap(), e.clip, (e.frame - e.last_step).max(0.)) * Vec3::new(e.root_k, 0., e.root_k);
        let m = &play.fight.player.motion;
        let his = Vec3::from_array(play.fight.player.pos) + if play.fight.paired { Quat::from_rotation_y(play.fight.player.loco.facing) * root_at(&play.fd.jack, m.clip, (m.frame - e.his_step * PAIR_LAG).max(0.)) + e.his_blip } else { Vec3::ZERO };
        eprintln!("PORT_CLASH_ROW {{\"tick\":{},\"elise\":{{\"st\":\"{:?}\",\"sub\":{},\"clip\":\"{}\",\"frame\":{:.1},\"pos\":[{:.4},{:.4},{:.4}],\"yaw\":{:.4}}},\"player\":{{\"clip\":\"{}\",\"frame\":{:.1},\"pos\":[{:.4},{:.4},{:.4}],\"facing\":{:.4}}}}}",
                  play.tick, e.st, e.sub, e.bank.as_ref().unwrap().clips[e.clip].name, e.frame, her.x, her.y, her.z, e.yaw,
                  play.fd.jack.clips[m.clip].name, m.frame, his.x, his.y, his.z, play.fight.player.loco.facing);
    }
    if std::env::var_os("MW_ELISE_LOG").is_some() && play.tick % std::env::var("MW_ELISE_LOG").ok().and_then(|v| v.parse().ok()).filter(|n: &u64| *n > 1).unwrap_or(30) == 0 {
        eprintln!("[elise] tick {} {:?}/{} hp {} level {:.0} flying {} bats {} dist {:.1} player hp {} at {:.2?}", play.tick, e.st, e.sub, e.hp, e.level, e.flying, e.bats_alive(), dist, play.fight.player.hp, play.fight.player.pos);
    }
    play.boss = Some((e.hp.max(0) as i16, max_hp() as i16, e.body()));
    for v in e.voices.drain(..) { play.fight.events.push(mw_sim::fight::FightEvent::Voice(v)); }
}

/// Bat cmd 4 (commandFromMistress 810B5E3C): every live bat flies a path 20 m out from where it is, yaw U[-pi, pi],
/// climbing U[0, 45] degrees (MtxInitRotVec order 5 on (0, 0, 20)).
fn scatter(e: &mut Elise) {
    for i in 0..e.bats.len() {
        if matches!(e.bats[i].st, BatSt::Dead | BatSt::Scatter) { continue; }
        let yaw = std::f32::consts::PI * (2. * e.roll() - 1.); let up = std::f32::consts::FRAC_PI_4 * e.roll();
        let b = &mut e.bats[i];
        b.st = BatSt::Scatter; b.t = 0.; b.from = b.pos;
        b.aim = b.pos + Vec3::new(yaw.sin() * up.cos(), up.sin(), yaw.cos() * up.cos()) * 20.;
    }
}

/// The clash meter's start: the live recomp run (MW_TEST_ELISE_CLASH, 2026-11-12) emptied it on the 241st update of
/// 0201 with no input, at -0.05 an update.
const KEEN_METER: f32 = 12.05;
/// The bite's hit-stops (the live recomp, 0204 / 0464 played to the end): when her frame reaches 42 and again 135, both
/// clips' rate drops to 0.001 for 9 updates, then eases back over 19 (about x1.25 an update); the 28 recorded steps.
const BITE_SLOW: [f32; 2] = [42., 135.];
const BITE_RAMP: [f32; 28] = [0.001, 0.001, 0.001, 0.001, 0.001, 0.001, 0.001, 0.001, 0.001, 0.003, 0.0037, 0.0047, 0.0059, 0.0073,
                              0.0092, 0.0114, 0.0143, 0.0179, 0.0224, 0.0279, 0.0349, 0.0437, 0.0545, 0.0683, 0.0852, 0.1066, 0.1332, 0.1333];

/// In her clash both bodies trail their clips' root travel by a frame: the live recomp's object positions match the
/// port's root one frame back (her 0206 / 0200 within 3 mm, his 0460 within 1 mm at two), as the fight's root step
/// lands one frame late (mw_sim fight.rs).
pub const PAIR_LAG: f32 = 1.;
impl Elise {
    /// The player's root lag in her clash (play.rs pose): the travel to take off his clip's root at `frame`.
    pub fn pair_lag(&self, play: &super::Play) -> Option<Vec3> {
        if !self.on || self.st != St::Keen || !play.fight.paired { return None; }
        let m = &play.fight.player.motion;
        let blip = Quat::from_rotation_y(-play.fight.player.loco.facing) * self.his_blip;
        Some(root_at(&play.fd.jack, m.clip, (m.frame - self.his_step * PAIR_LAG).max(0.)) - root_at(&play.fd.jack, m.clip, m.frame) + blip)
    }
}

/// startAtk on her coffin opens the clash (Phase 0 of fn_802A03A0; see the strike above).
fn start_keen(e: &mut Elise, play: &mut super::Play, arena: &super::Arena, pl: Vec3) {
    e.set(St::Keen, "em0630_0206.mot");
    e.cutin_done = false;
    e.keen_case = if e.pos.x.abs() < 0.2 { 3 } else if e.pos.z < -12. { if e.pos.x < 0. { 1 } else { 2 } } else { 0 };
    if e.keen_case != 3 && e.pos.z > -12. {
        e.yaw = (if e.pos.x > 0. { -1f32 } else { 1. }).atan2(if e.pos.z > 0. { -1. } else { 1. });
    }
    // (3.75 / 1.95 in fn_802A03A0; the live recomp stood both objects at 3.66 / 1.89 through the pull-out, the crypt
    // platforms' floor + 0.1, which the room collision staged here does not carry. The port keeps actors on the floor
    // itself (the original's objects stand 0.1 above it, their feet drawn on it): 3.56 / 1.79, and the floor for case 0)
    let y = match e.keen_case { 3 => 3.56, 1 | 2 => 1.79, _ => pl.y };
    let throw = throw_offset(e.bank.as_ref().unwrap(), e.clip);
    let at = e.pos + Quat::from_rotation_y(e.yaw) * throw;
    // both stand at that height (the live recomp: 3.66 / 1.89 for both, the crypt platform under him)
    let ground = super::floor(&arena.0, Vec3::new(at.x, y + 0.5, at.z));
    if std::env::var_os("MW_ELISE_LOG").is_some() { eprintln!("[elise] clash case {} height {y} floor {ground:?}", e.keen_case); }
    let y = ground.filter(|g| (y - 0.4..=y + 0.1).contains(g)).unwrap_or(y);
    if e.keen_case != 0 { e.pos.y = y; }
    e.root_k = 1.; e.keen_moved = false;
    e.meter = KEEN_METER; e.lean = 0.5; e.qte = 0; e.prompt = None;
    e.player_from = pl;
    play.fight.paired = true;
    let pp = &mut *play; pp.fight.external(&pp.fd, "pl0030_0466.mot", false, 4.);
    play.fight.player.pos = [at.x, y, at.z];
    play.fight.player.loco.facing = e.yaw + std::f32::consts::PI;
}

/// The bite's hit-stops: hers starts on the update after her frame reaches BITE_SLOW; his one update sooner, on the
/// update she reaches it (the live recomp: his ramp opens at his frame 40 while hers opens at 42, one behind at the
/// bite's start). His rate is set here for the fight's next step, which runs before her next update.
fn bite_slow(e: &mut Elise, play: &mut super::Play) {
    let biting = e.st == St::Keen && e.clips.get("em0630_0204.mot") == Some(&e.clip);
    if !biting { e.slow = 0; e.his_slow = 0; }
    if biting && e.slow == 0 && BITE_SLOW.iter().any(|t| e.frame >= *t && e.frame - e.last_step < *t) { e.slow = 1; }
    if biting && e.slow == 0 && e.his_slow == 0 && BITE_SLOW.iter().any(|t| e.frame < *t && e.frame + 1. >= *t) { e.his_slow = 1; }
    e.his_step = play.fight.player.motion.rate;
    if play.fight.paired && e.st == St::Keen {
        play.fight.player.motion.rate = if e.his_slow > 0 { BITE_RAMP[e.his_slow - 1] } else { 1. };
        if e.his_slow > 0 { e.his_slow = if e.his_slow >= BITE_RAMP.len() { 0 } else { e.his_slow + 1 }; }
    }
}

/// His pair clip is at its last frame (the update the original moves to the next phase; the fight's `ended` lands an
/// update later).
fn his_last(play: &super::Play) -> bool {
    let m = &play.fight.player.motion;
    m.frame >= (play.fd.jack.clips[m.clip].frames.max(1) - 1) as f32
}

/// The bite drains him (the live recomp, his HP over 0464: 1 every 6 updates from his frame 46.8 to 100.8, then 10 at
/// 133.8, the second hit-stop; 20 in all). Each point drawn shows as blood pulled from his neck to her mouth and a red
/// pulse on the screen [added FX].
fn bite_drain(e: &mut Elise, play: &mut super::Play, fx: &mut super::fx::Fx) {
    e.drain_pulse *= 0.85;
    if e.st != St::Keen || e.clips.get("em0630_0204.mot") != Some(&e.clip) || !play.fight.paired { return; }
    let m = &play.fight.player.motion;
    if play.fd.jack.clips[m.clip].name != "pl0030_0464.mot" { return; }
    let f = m.frame;
    let want = if f >= 46.8 { (((f.min(100.8) - 46.8) / 6.).floor() as u32 + 1).min(10) } else { 0 };
    let mut ticks = 0;
    while e.drained < want { e.drained += 1; ticks += 1; play.fight.player.hp = (play.fight.player.hp - 1).max(0); }
    if f >= 133.8 && !e.bite_blow { e.bite_blow = true; ticks += 4; play.fight.player.hp = (play.fight.player.hp - 10).max(0); }
    if ticks == 0 { return; }
    e.drain_pulse = 1.;
    // the stream: blood along the line from his neck to her mouth
    let jack = &play.fd.jack;
    let poses = sample_retargeted(&jack.bones, &jack.clips[m.clip], f.min((jack.clips[m.clip].frames.max(2) - 1) as f32), Retarget::NATIVE);
    let jf = mw_sim::fight::bone_frames(&jack.bones, &poses);
    let neck_local = jack.bones.iter().position(|b| b.id == 3).map(|i| Vec3::from_array(jf[i].1)).unwrap_or(Vec3::Y * 1.5);
    let neck = Vec3::from_array(play.fight.player.pos) + Quat::from_rotation_y(play.fight.player.loco.facing) * neck_local;
    let mouth = e.bank.as_ref().and_then(|bank| {
        let c = &bank.clips[e.clip];
        let p = sample_retargeted(&bank.bones, c, e.frame.min((c.frames.max(2) - 1) as f32), Retarget::NATIVE);
        let hf = mw_sim::fight::bone_frames(&bank.bones, &p);
        bank.bones.iter().position(|b| b.id == 4).map(|i| e.pos + Vec3::Y * e.height + Quat::from_rotation_y(e.yaw) * Vec3::from_array(hf[i].1))
    }).unwrap_or(e.body() + Vec3::Y * 0.5);
    let yaw = (mouth.x - neck.x).atan2(mouth.z - neck.z);
    // the first drink [added]: fang marks on his neck where she bit, his blood round her mouth
    if e.drained <= ticks {
        // (his marks go on once he is back on his feet, on the side she bit: his head is thrown back now)
        let to_her = Vec3::new(mouth.x - neck.x, 0., mouth.z - neck.z).normalize_or(Vec3::Z);
        let pf = play.fight.player.loco.facing;
        let his_right = Vec3::new(-pf.cos(), 0., pf.sin());
        e.neck_due = Some((if to_her.dot(his_right) >= 0. { 1. } else { -1. }, 0));
        // (her mouth is marked once she stands upright again: bent over his neck her face looks down)
        e.mouth_due = true;
    }
    for k in 0..(2 + ticks.min(4)) {
        let at = neck.lerp(mouth, k as f32 / (1 + ticks.min(4)) as f32);
        fx.gore(at, yaw, 6, false, false);
    }
}

#[derive(Component)]
struct DrainTint;
/// The red pulse while she drinks (bite_drain) [added FX].
fn drain_tint(el: Res<Elise>, q: Query<Entity, With<DrainTint>>, mut commands: Commands) {
    for x in &q { commands.entity(x).despawn(); }
    if !el.on || el.drain_pulse < 0.02 { return; }
    commands.spawn((Node { position_type: PositionType::Absolute, left: px(0.), top: px(0.), width: Val::Percent(100.), height: Val::Percent(100.), ..default() },
                    BackgroundColor(Color::srgba(0.6, 0., 0.02, 0.32 * el.drain_pulse)), GlobalZIndex(15500), DrainTint));
}

/// Her strikes land on contact [added, in place of a distance to her root]: a hand, elbow, knee or foot (the posed
/// bones, her yaw and height) within LIMB_HIT of his body (the segment from his shins to his head).
const LIMB_HIT: f32 = 0.55;
const WING_HIT: f32 = 0.75;
const LIMBS: [i16; 10] = [8, 9, 13, 14, 17, 18, 19, 21, 22, 23];
fn limb_reach(e: &Elise, pl: Vec3) -> f32 {
    let Some(bank) = e.bank.as_ref() else { return f32::MAX; };
    let c = &bank.clips[e.clip];
    let mut poses = sample_retargeted(&bank.bones, c, e.frame.min((c.frames.max(2) - 1) as f32), Retarget::NATIVE);
    strip_root_xz(&bank.bones, &mut poses);
    let frames = mw_sim::fight::bone_frames(&bank.bones, &poses);
    let (a, b) = (pl + Vec3::Y * 0.3, pl + Vec3::Y * 1.75);
    let q = Quat::from_rotation_y(e.yaw);
    let at = e.pos + Vec3::Y * e.height;
    bank.bones.iter().enumerate().filter(|(_, x)| LIMBS.contains(&x.id)).map(|(i, _)| {
        let p = at + q * Vec3::from_array(frames[i].1);
        let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0., 1.);
        p.distance(a + (b - a) * t)
    }).fold(f32::MAX, f32::min)
}

/// The last-life execution's steps (Elise::start_execution).
fn step_execution(e: &mut Elise, play: &mut super::Play, arena: &super::Arena, ended: bool) {
    start_his(play, e, arena);
    let pf = play.fight.player.loco.facing;
    let spot = Vec3::from_array(play.fight.player.pos) + Quat::from_rotation_y(pf) * Vec3::new(-0.011, 0., 1.158);
    match e.exec {
        1 => {
            if ended { e.frame = 0.; }
            e.height = (e.height - 0.06).max(0.);
            if e.height < 0.3 && e.clips.get("em0630_0010.mot") != Some(&e.clip) { e.play("em0630_0010.mot"); }
            let d = Vec3::new(spot.x - e.pos.x, 0., spot.z - e.pos.z);
            let step = d.length().min(0.07);
            if d.length() > 1e-4 { e.pos += d.normalize() * step; e.yaw = d.x.atan2(d.z); }
            if d.length() < 0.05 && e.height <= 0. {
                e.pos = Vec3::new(spot.x, play.fight.player.pos[1], spot.z); e.yaw = pf + std::f32::consts::PI; e.flying = false;
                e.set(St::Keen, "em0630_0201.mot"); e.sub = 20; e.root_k = 1.; e.lean = 0.;
                play.fight.paired = true;
                let pp = &mut *play; pp.fight.external(&pp.fd, "pl0030_0461.mot", true, 10.);
                e.exec = 2; e.timer = 50.;
            }
        }
        2 => {
            if ended { e.frame = 0.; }
            e.timer -= 1.;
            if e.timer <= 0. { e.exec = 3; e.landed = 0; e.voices.push(vo("bite", 0xA9)); pair_next(play, e, "em0630_0204.mot", "pl0030_0464.mot", false); }
        }
        // her bite's first hit-stop (frame 42) tears his head off (play_execution.rs)
        3 if !ended && e.clips.get("em0630_0204.mot") == Some(&e.clip) && e.frame >= 42. && e.landed == 0 => {
            e.landed = 1; play.fight.events.push(mw_sim::fight::FightEvent::ExecutionCut);
        }
        _ => {
            if ended {
                pair_keep(play, e); e.slow = 0; play.fight.player.motion.rate = 1.;
                play.fight.paired = false;
                let pp = &mut *play; pp.fight.fall_dead(&pp.fd, "pl0030_1016.mot");
                e.exec = 0; e.height = 0.; e.set(St::Victory, "em0630_0104.mot"); e.voices.push(vo("victory", 0xAB));
            }
        }
    }
}

/// A clip's throw offset (bone 1024 at frame 0, GetThrowOffset 8007F058): where the partner stands from the actor.
fn throw_offset(bank: &MotionBank, clip: usize) -> Vec3 {
    let c = &bank.clips[clip];
    let at = |ch: u8| c.tracks.iter().find(|t| t.bone == 1024 && t.channel == ch).and_then(|t| t.values.first().copied()).unwrap_or(0.);
    Vec3::new(at(0), at(1), at(2))
}

/// A clip's root travel (x, z) at a frame in the actor's frame.
fn root_at(bank: &MotionBank, clip: usize, frame: f32) -> Vec3 {
    let c = &bank.clips[clip];
    let r = mw_sim::sample(&bank.bones, c, frame.min((c.frames.max(1) - 1) as f32))[bank.bones.len()].translation;
    Vec3::new(r[0], 0., r[2])
}

/// Pull-out frame 23 (fn_802A03A0 802A0794..802A0B3C): cases 1 / 2 / 3 move him to (-11, 0.01, -10) facing
/// atan2(-2, -3), (13, 0.01, -11) facing atan2(2, -3), or (0, 0.01, -13) facing pi. Her offset from him is turned
/// by the dot of the old and new facings (as the original computes it), she faces him (his yaw + pi).
fn keen_move(play: &mut super::Play, e: &mut Elise) {
    let (to, yaw) = match e.keen_case {
        // (the atan2 arguments are doubles at r31+4744 / +4752 / +4768: -2.0, -3.0, 2.0; the live recomp's case 2 faced
        // 2.554 = atan2(2, -3))
        // (y 0.01 there: the floor, in the port's feet-on-the-floor heights)
        1 => (Vec3::new(-11., 0., -10.), (-2f64).atan2(-3.) as f32),
        2 => (Vec3::new(13., 0., -11.), 2f64.atan2(-3.) as f32),
        3 => (Vec3::new(0., 0., -13.), std::f32::consts::PI),
        _ => return,
    };
    // the move works on where both stood an update ago (their drawn spots trail the roots by an update, and the move
    // reads the objects before this update's step): two steps back on each clip
    let m = &play.fight.player.motion;
    let his_back = root_at(&play.fd.jack, m.clip, (m.frame - 2. * e.his_step).max(0.));
    let bank = e.bank.as_ref().unwrap();
    let her_back = root_at(bank, e.clip, (e.frame - 2. * e.last_step).max(0.));
    let his = Vec3::from_array(play.fight.player.pos) + Quat::from_rotation_y(play.fight.player.loco.facing) * his_back;
    let her = e.pos + Quat::from_rotation_y(e.yaw) * her_back;
    let fwd = |a: f32| Vec3::new(a.sin(), 0., a.cos());
    // (the live recomp turns it by +dot: its case-2 offset swung from yaw -1.56 to -0.99 for a dot of 0.555)
    let turn = if e.keen_case == 3 { 0. } else { fwd(play.fight.player.loco.facing).dot(fwd(yaw)) };
    let off = Quat::from_rotation_y(turn) * (her - his);
    // the new spots are the objects' positions; the update's root step then lands on them along the new facings (the
    // live recomp's case 2: her drawn spot after the move = his spot + the turned offset + her step, within 7 mm)
    let base = to - Quat::from_rotation_y(yaw) * his_back;
    play.fight.player.pos = [base.x, to.y, base.z]; play.fight.player.loco.facing = yaw;
    e.yaw = yaw + std::f32::consts::PI;
    e.pos = to + off - Quat::from_rotation_y(e.yaw) * her_back;
    e.pos.y = to.y;
}

/// Both halves of the pair keep the travel of the clips they played (their roots restart at 0 with each new clip).
fn pair_keep(play: &mut super::Play, e: &mut Elise) {
    keep_hers(e);
    keep_his(play);
}
fn keep_hers(e: &mut Elise) {
    // (where the clip is: a loop leaves mid-cycle)
    let her = root_at(e.bank.as_ref().unwrap(), e.clip, e.frame) * Vec3::new(e.root_k, 1., e.root_k);
    e.pos += Quat::from_rotation_y(e.yaw) * her;
    e.root_k = 1.;
}
fn keep_his(play: &mut super::Play) {
    play.fight.player.loco.layer = None;
    let m = &play.fight.player.motion;
    let his = root_at(&play.fd.jack, m.clip, m.frame);
    let at = Vec3::from_array(play.fight.player.pos) + Quat::from_rotation_y(play.fight.player.loco.facing) * his;
    play.fight.player.pos = [at.x, at.y, at.z];
}

/// The next clips of the clash pair [no blend: the pairs are authored back to back, and a crossfade would add the
/// last clip's root travel twice].
/// His clip starts on the next update (the live recomp: each of his pair clips is at frame 0 when hers is at 1).
/// Both switch on the update after the old clips' last frame (the live recomp shows her old clip at its last frame,
/// then her new one at frame 1 with his at 0): hers in `start_hers`, before her frame advances, his in `start_his`.
fn pair_next(play: &mut super::Play, e: &mut Elise, her: &'static str, his: &'static str, looping: bool) {
    let _ = play;
    e.her_next = Some(her);
    e.his_next = Some((his, looping));
}

/// Her deferred pair clip starts (0200 with her placement: his (0, 0, 6) less GetThrowOffset(pl0030_0460, bone 1024),
/// unrotated, facing +z, 0.1 up: 802A0B60).
fn start_hers(play: &mut super::Play, e: &mut Elise, arena: &super::Arena) {
    let Some(her) = e.her_next.take() else { return; };
    if her == "em0630_0204.mot" { e.drained = 0; e.bite_blow = false; }
    keep_hers(e);
    e.play(her);
    if her == "em0630_0200.mot" {
        let throw = throw_offset(&play.fd.jack, play.fd.jack_clip("pl0030_0460.mot"));
        let floor = super::floor(&arena.0, Vec3::new(0., 2., 6.)).unwrap_or(0.);
        e.pos = Vec3::new(-throw.x, floor, 6. - throw.z); e.yaw = 0.; e.height = 0.;
    }
}

/// His deferred pair clip starts (with his placement: 0460 at (0, 0, 6) facing -z, 0461 at its throw offset from her).
fn start_his(play: &mut super::Play, e: &mut Elise, arena: &super::Arena) {
    e.his_blip = Vec3::ZERO;
    let Some((his, looping)) = e.his_next.take() else { return; };
    // into 0460 his drawn spot takes the pull-out's last step once more on top of the new placement, for one update (the
    // live recomp: (0, 6) + (0.149, 0.159), his last 0466 step), then shows the placement
    if his == "pl0030_0460.mot" {
        let m = &play.fight.player.motion;
        let n = (play.fd.jack.clips[m.clip].frames.max(2) - 1) as f32;
        let step = root_at(&play.fd.jack, m.clip, n) - root_at(&play.fd.jack, m.clip, n - 1.);
        e.his_blip = Quat::from_rotation_y(play.fight.player.loco.facing) * step;
    }
    if his == "pl0030_0460.mot" { play.fight.player.loco.layer = None; } else { keep_his(play); }
    let pp = &mut *play; pp.fight.external(&pp.fd, his, looping, 0.);
    match his {
        "pl0030_0460.mot" => {
            let floor = super::floor(&arena.0, Vec3::new(0., 2., 6.)).unwrap_or(0.);
            // (the original's 0.1 lift is its objects' standing height: the port keeps him on the floor)
            play.fight.player.pos = [0., floor, 6.]; play.fight.player.loco.facing = std::f32::consts::PI;
        }
        // into the struggle she stands at his spot less 0461's throw offset, unrotated as for 0460 (the live recomp: him
        // (0.151, 1.205), her (0.162, 0.048) = his - (-0.011, 1.158)); he keeps where 0460 took him
        "pl0030_0461.mot" => {
            let throw = throw_offset(&play.fd.jack, play.fight.player.motion.clip);
            let her = e.pos + Quat::from_rotation_y(e.yaw) * root_at(e.bank.as_ref().unwrap(), e.clip, (e.frame - e.last_step).max(0.));
            let at = Vec3::from_array(play.fight.player.pos) - throw;
            e.pos += Vec3::new(at.x - her.x, 0., at.z - her.z);
        }
        _ => {}
    }
}

/// The clash pair is over: both keep their clips' final root travel.
fn end_pair(play: &mut super::Play, e: &mut Elise) {
    e.his_next = None; e.her_next = None; e.slow = 0; play.fight.player.motion.rate = 1.;
    // (he may have left the pair already: the bite's get-up)
    if play.fight.paired { pair_keep(play, e); } else { keep_hers(e); return; }
    play.fight.player.pos[1] = e.player_from.y;
    play.fight.paired = false;
    play.fight.end_external();
}

fn summon_frames() -> f32 { tune("summon_frames", 110.).max(30.) }

/// [added] the hide-phase summon: groups of bats from the swarm fly to points around the church's centre, drop and turn
/// into zako (play_street.rs's em0101 as the placeholder) — more with every hide.
fn summon(e: &mut Elise, pl: Vec3) {
    let first = tune("summon_zako", 3.);
    if first <= 0. { return; }
    let n = (first + tune("summon_zako_per_hide", 1.) * (e.hide_n - 1).max(0) as f32).min(tune("summon_zako_max", 5.)) as usize;
    let each = tune("summon_bats_each", 6.).max(1.) as usize;
    let centre = Vec3::new(tune("summon_center_x", 0.), 0., tune("summon_center_z", 4.));
    let radius = tune("summon_radius", 3.);
    let turn = e.roll() * std::f32::consts::TAU;
    for k in 0..n {
        let a = turn + k as f32 * std::f32::consts::TAU / n as f32;
        // (kept off the player: a point within 1.5 m of him moves out)
        let mut spot = centre + Vec3::new(a.cos(), 0., a.sin()) * radius;
        let off = Vec3::new(spot.x - pl.x, 0., spot.z - pl.z);
        if off.length() < 1.5 { spot = pl + off.normalize_or(Vec3::Z) * 1.5; }
        let g = e.drops.len();
        e.drops.push(ZakoDrop { spot, t: 0., done: false });
        let mut got = 0;
        for b in e.bats.iter_mut().filter(|b| matches!(b.st, BatSt::Orbit)) { if got >= each { break; } b.st = BatSt::Summon(g); got += 1; }
        if got == 0 { e.drops.pop(); break; }
    }
    if std::env::var_os("MW_ELISE_LOG").is_some() { eprintln!("[elise] hide {}: summoning {} zako from the bats", e.hide_n, e.drops.iter().filter(|d| !d.done).count()); }
}

/// The drops' clocks: at the end each group's bats burst into its zako.
fn step_drops(e: &mut Elise, fx: &mut super::fx::Fx) {
    let n = summon_frames();
    for k in 0..e.drops.len() {
        if e.drops[k].done { continue; }
        e.drops[k].t += 1.;
        let d = e.drops[k];
        if d.t < n { continue; }
        e.drops[k].done = true;
        for b in e.bats.iter_mut().filter(|b| b.st == BatSt::Summon(k)) { b.st = BatSt::Dead; b.pos.y = -3.; b.vel = Vec3::ZERO; }
        // the bats burst apart into the zako: blood mist, a splash on the floor
        fx.burst("core", 1005, d.spot + Vec3::Y * 1.2, 0., 1.4);
        fx.burst("core", 1000, d.spot + Vec3::Y * 0.6, 0., 1.0);
        fx.gore(d.spot + Vec3::Y * 1.0, 0., 24, false, false);
        e.zako.push((d.spot, 0.));
    }
    if e.drops.iter().all(|d| d.done) { e.drops.clear(); }
}

fn step_bats(e: &mut Elise, play: &mut super::Play, chest: Vec3, fx: &mut super::fx::Fx) {
    let center = e.body();
    let crypt = Some(Vec3::from_array(CRYPTS[e.crypt].0) + Vec3::Y * 1.0);
    let heads: Vec<Vec3> = e.heads.iter().map(|h| h.0).collect();
    let mut volley_hit = e.volley_hit;
    let drops: Vec<ZakoDrop> = e.drops.iter().map(|d| ZakoDrop { ..*d }).collect();
    for (i, b) in e.bats.iter_mut().enumerate() {
        b.frame = (b.frame + 1.) % 6.;
        match b.st {
            BatSt::Orbit => {
                b.phase += 0.05 + (i % 5) as f32 * 0.004;
                let r = 1.4 + (i % 9) as f32 * 0.18;
                let want = center + Vec3::new(b.phase.cos() * r, 0.6 + ((i * 7) % 11) as f32 * 0.12 + (b.phase * 2.).sin() * 0.2, b.phase.sin() * r);
                b.vel = b.vel * 0.85 + (want - b.pos) * 0.04;
            }
            BatSt::Dive => {
                b.t += 1.;
                b.vel = (b.vel + (chest - b.pos).normalize_or(Vec3::Z) * 0.03).clamp_length_max(0.32);
                if b.pos.distance(chest) < 0.65 && !volley_hit {
                    // the dive (sphere 0.3, ATHIT 0x172 / 0x173, power 10)
                    volley_hit = true;
                    if play.fight.enemy_hits_player(&play.fd, if i % 2 == 0 { 0x172 } else { 0x173 }, 10, b.pos.to_array()) { fx.burst("core", 1005, chest, 0., 0.6); }
                }
                if b.t > 50. || b.pos.distance(chest) < 0.3 { b.st = BatSt::Orbit; }
            }
            BatSt::Head(k) => {
                // the dragon's body: a trailing snake of bats behind its head
                if let Some(h) = heads.get(k) {
                    let lag = (b.t % 40.) * 0.09;
                    let want = *h - (*h - center).normalize_or(Vec3::Y) * lag + Vec3::new((b.t * 1.7).sin(), (b.t * 2.3).cos(), (b.t * 1.1).sin()) * 0.25;
                    b.vel = b.vel * 0.6 + (want - b.pos) * 0.25;
                } else { b.st = BatSt::Orbit; }
            }
            BatSt::Hint => {
                if let Some(c) = crypt { b.phase += 0.08; let want = c + Vec3::new(b.phase.cos() * 0.8, 0.6 + (b.phase * 3.).sin() * 0.2, b.phase.sin() * 0.8); b.vel = b.vel * 0.8 + (want - b.pos) * 0.06; }
                if e.st != St::Hide { b.st = BatSt::Orbit; }
            }
            BatSt::Summon(k) => {
                // gather in a swirl over the point, tighten and sink, then dive into it (the group lands together)
                let Some(d) = drops.get(k) else { b.st = BatSt::Orbit; continue; };
                let n = summon_frames();
                let u = (d.t / n).min(1.);
                b.phase += 0.12 + (i % 4) as f32 * 0.01;
                let (r, h) = if u < 0.55 { (1.1, 4.2) } else if u < 0.8 { (1.1 - (u - 0.55) * 3.2, 4.2 - (u - 0.55) * 6.) } else { (0.15, 2.7 - (u - 0.8) * 13.) };
                let want = d.spot + Vec3::new(b.phase.cos() * r, h.max(0.4) + ((i * 5) % 7) as f32 * 0.08, b.phase.sin() * r);
                let pull = if u < 0.55 { 0.03 } else { 0.12 };
                b.vel = (b.vel * 0.82 + (want - b.pos) * pull).clamp_length_max(0.45);
            }
            BatSt::Dead => { b.vel.y -= 0.01; b.vel *= 0.97; }
            // the scatter path (em0330 path 810B4588): spline progress +0.03 a frame, the bat lerps 0.1 toward it;
            // PathOver 810B6690 then parks it under the floor (y = -1, dead)
            BatSt::Scatter => {
                b.t = (b.t + 0.03).min(1.);
                let want = b.from.lerp(b.aim, b.t * b.t * (3. - 2. * b.t));
                b.vel = (want - b.pos) * 0.1;
                if b.t >= 1. { b.st = BatSt::Dead; b.pos.y = -3.; b.vel = Vec3::ZERO; }
            }
        }
        b.pos += b.vel;
    }
    e.volley_hit = volley_hit;
}

fn bind(mut el: ResMut<Elise>, children: Query<&Children>, names: Query<&Name>, mut commands: Commands) {
    if !el.on || !el.spawned { return; }
    let e = &mut *el;
    let find = |root: Entity, bones: &[mw_data::Bone]| -> Option<Vec<Entity>> {
        let found: std::collections::HashMap<String, Entity> = children.iter_descendants(root).filter_map(|d| names.get(d).ok().map(|n| (n.as_str().to_string(), d))).collect();
        bones.iter().map(|b| b.id).chain(std::iter::once(-1)).map(|id| found.get(&format!("role_0_bone{id}")).copied()).collect()
    };
    if let Some(bank) = &e.bank { for k in 0..2 { if e.joints[k].is_none() { e.joints[k] = find(e.roots[k], &bank.bones);
        // her cape, hair and breasts on springs (play_springs.rs)
        if let Some(j) = &e.joints[k] { commands.entity(e.roots[k]).insert(super::springs::SpringRequest { bones: bank.bones.clone(), joints: j.clone(), actor: "em0630" }); }
    } } }
    if let Some(bank) = &e.bat_bank { for b in e.bats.iter_mut() { for k in 0..2 { if b.joints[k].is_none() { b.joints[k] = find(b.roots[k], &bank.bones); } } } }
}

pub fn pose(el: Res<Elise>, play: Res<super::Play>, intro: Res<super::intro::Intro>, mut transforms: Query<&mut Transform>, mut vis: Query<&mut Visibility>) {
    if !el.on || !el.spawned { return; }
    // the events' own Elise and bats play the intro
    let staged = !intro.active();
    let Some(bank) = &el.bank else { return; };
    let clip = &bank.clips[el.clip];
    let mut poses = sample_retargeted(&bank.bones, clip, el.frame.min((clip.frames.max(2) - 1) as f32), Retarget::NATIVE);
    let paired = el.st == St::Keen || el.ed_scene;
    // the pistol flinch [added]: her hit clip over the spine, neck, head and arms (bones 1-14), in at full weight and out
    // over its last 8 frames
    if let Some((c, f)) = el.hurt_walk {
        let hc = &bank.clips[c];
        let n = (hc.frames.max(2) - 1) as f32;
        let w = ((n - f) / 8.).clamp(0., 1.) * 0.85;
        let hit = sample_retargeted(&bank.bones, hc, f.min(n), Retarget::NATIVE);
        for (i, b) in bank.bones.iter().enumerate() {
            if (1..=14).contains(&b.id) {
                let r = Quat::from_array(poses[i].rotation).slerp(Quat::from_array(hit[i].rotation), w);
                poses[i].rotation = r.to_array();
            }
        }
    }
    if !paired { strip_root_xz(&bank.bones, &mut poses); }
    else if let Some(r) = poses.get_mut(bank.bones.len()) {
        // her body trails the clip's root by PAIR_LAG frames, as the original object's position does
        let lag = root_at(bank, el.clip, (el.frame - el.last_step).max(0.)) - root_at(bank, el.clip, el.frame);
        r.translation[0] = (r.translation[0] + lag.x) * el.root_k; r.translation[2] = (r.translation[2] + lag.z) * el.root_k;
    }
    if paired && el.sub == 2 && el.lean > 0.01 {
        if let Some(&c) = el.clips.get("em0630_0202.mot") {
            let lean = sample_retargeted(&bank.bones, &bank.clips[c], el.frame.min((bank.clips[c].frames.max(2) - 1) as f32), Retarget::NATIVE);
            poses = mix(&poses, &lean, el.lean);
        }
    }
    let at = el.pos + Vec3::Y * el.height;
    for k in 0..2 {
        let shown = staged && (k == 1) == play.colored;
        if let Ok(mut v) = vis.get_mut(el.roots[k]) { *v = if shown { Visibility::Visible } else { Visibility::Hidden }; }
        if let Ok(mut t) = transforms.get_mut(el.roots[k]) { *t = Transform::from_translation(at).with_rotation(Quat::from_rotation_y(el.yaw)); }
        let Some(js) = &el.joints[k] else { continue; };
        for (e, p) in js.iter().zip(&poses) {
            if let Ok(mut t) = transforms.get_mut(*e) { t.translation = Vec3::from_array(p.translation); t.rotation = Quat::from_array(p.rotation); t.scale = Vec3::from_array(p.scale); }
        }
    }
    let Some(bb) = &el.bat_bank else { return; };
    let flap = &bb.clips[0];
    for b in &el.bats {
        let dead_gone = b.st == BatSt::Dead && b.pos.y < -2.;
        let bp = sample_retargeted(&bb.bones, flap, b.frame.min((flap.frames.max(2) - 1) as f32), Retarget::NATIVE);
        let yaw = b.vel.x.atan2(b.vel.z);
        for k in 0..2 {
            let shown = staged && (k == 1) == play.colored && !dead_gone;
            if let Ok(mut v) = vis.get_mut(b.roots[k]) { *v = if shown { Visibility::Visible } else { Visibility::Hidden }; }
            if let Ok(mut t) = transforms.get_mut(b.roots[k]) { *t = Transform::from_translation(b.pos).with_rotation(Quat::from_rotation_y(yaw)); }
            let Some(js) = &b.joints[k] else { continue; };
            for (e, p) in js.iter().zip(&bp) {
                if let Ok(mut t) = transforms.get_mut(*e) { t.translation = Vec3::from_array(p.translation); t.rotation = Quat::from_array(p.rotation); t.scale = Vec3::from_array(p.scale); }
            }
        }
    }
}

#[derive(Component)]
struct PromptText;
/// The clash prompts (meter, swing sequence) [added: the native cockpit art is not drawn].
fn prompt(el: Res<Elise>, mut q: Query<(Entity, &mut Text), With<PromptText>>, mut commands: Commands) {
    if !el.on { return; }
    // (the native swing navigation draws the clash prompts: play_prompts.rs; MW_TEXT_PROMPTS=1 adds this text)
    let shown = if std::env::var_os("MW_TEXT_PROMPTS").is_some() { &el.prompt } else { &None };
    match (shown, q.single_mut()) {
        (Some(p), Ok((_, mut t))) => { if t.0 != *p { t.0 = p.clone(); } }
        (Some(p), Err(_)) => { commands.spawn((Text::new(p.clone()), TextFont { font_size: bevy::text::FontSize::Px(34.), ..default() }, TextColor(Color::srgb(1., 0.85, 0.2)), PromptText,
                                               Node { position_type: PositionType::Absolute, top: percent(18), left: percent(38), ..default() }, GlobalZIndex(2500))); }
        (None, Ok((e, _))) => { commands.entity(e).despawn(); }
        (None, Err(_)) => {}
    }
}

// ---- the actor editor (play_editor.rs) ----

/// Both stood at their editor spots (her default 6 m into the nave, him 3.5 m in front), full HP, any pair ended.
fn editor_reset(e: &mut Elise, play: &mut super::Play, arena: &super::Arena, spot: Option<Vec3>, pspot: Option<Vec3>) {
    if play.fight.paired || e.st == St::Keen { end_pair(play, e); }
    if play.fight.player.state() == mw_sim::fight::STATE_EXTERNAL { play.fight.end_external(); }
    play.fight.paired = false;
    e.exec = 0; e.auto = None; e.ed_loop = false; e.hurt_walk = None; e.prompt = None; e.heads.clear(); e.slow = 0; e.drained = 0; e.bite_blow = false;
    e.hp = max_hp(); e.flying = false; e.height = 0.; e.hide_acc = 0; e.level = 0.; e.landed = 0;
    let s = spot.unwrap_or(Vec3::new(0., 0., 6.));
    let p = pspot.unwrap_or(s + Vec3::Z * 3.5);
    let fy = |v: Vec3| super::floor(&arena.0, v + Vec3::Y * 2.).unwrap_or(v.y);
    e.pos = Vec3::new(s.x, fy(s), s.z);
    e.yaw = (p.x - s.x).atan2(p.z - s.z);
    play.fight.player.pos = [p.x, fy(p), p.z];
    play.fight.player.loco.facing = (s.x - p.x).atan2(s.z - p.z);
    play.fight.player.hp = mw_sim::fight::PLAYER_MAX_HP;
    play.cam_yaw = play.fight.player.loco.facing;
    e.set(St::Idle, "em0630_0001.mot");
    let c = e.body();
    for (i, b) in e.bats.iter_mut().enumerate() {
        if matches!(b.st, BatSt::Dead | BatSt::Summon(_) | BatSt::Scatter) { let a = i as f32 * 2.399; b.pos = c + Vec3::new(a.cos() * 2., 1. + (i % 7) as f32 * 0.3, a.sin() * 2.); b.vel = Vec3::ZERO; }
        b.st = BatSt::Orbit;
    }
}

/// An editor command, played through her own states.
fn editor_cmd(e: &mut Elise, cmd: &super::editor::Cmd, play: &mut super::Play, arena: &super::Arena, fx: &mut super::fx::Fx,
              spot: Option<Vec3>, pspot: Option<Vec3>) -> Result<(), String> {
    use super::editor::Cmd;
    let pl = Vec3::from_array(play.fight.player.pos);
    let pf = play.fight.player.loco.facing;
    let pfwd = Vec3::new(pf.sin(), 0., pf.cos());
    // stood in front of him at `d` m, facing him
    let close = |e: &mut Elise, d: f32| { e.pos = Vec3::new(pl.x, e.pos.y, pl.z) + pfwd * d; e.yaw = pf + std::f32::consts::PI; };
    let busy = e.st == St::Keen || e.exec > 0 || play.fight.paired || matches!(e.st, St::Finisher | St::Hide);
    if *cmd != Cmd::Reset && busy { return Err("Still playing (a pair, the hide or the finisher): R resets".into()); }
    let ground = |e: &mut Elise| { e.flying = false; e.height = 0.; };
    match cmd {
        Cmd::Reset => editor_reset(e, play, arena, spot, pspot),
        Cmd::Clip { name, looped } => {
            if !e.clips.contains_key(name.as_str()) { return Err(format!("{name} is not in her bank")); }
            e.set(St::Editor, name); e.ed_loop = *looped;
        }
        Cmd::Move(m) => match m.as_str() {
            "attack" => { ground(e); close(e, 1.7); e.set(St::Attack, "em0630_0111.mot"); e.voices.push(vo("wing_strike", 0x9D)); }
            "somersault" => { ground(e); close(e, 1.7); e.set(St::Somersault, "em0630_0110.mot"); }
            "volley" | "volley_b" => { e.flying = true; e.height = FLY_HEIGHT; e.set(St::Volley, if m == "volley" { "em0630_0100.mot" } else { "em0630_0101.mot" }); e.volley_hit = false; }
            "dragon" => { e.flying = true; e.height = FLY_HEIGHT; e.set(St::Dragon, "em0630_0102.mot"); e.dragon_hp = e.hp; }
            "guard" => { ground(e); e.set(St::Guard, "em0630_0020.mot"); e.guard_hits = 0; e.guard_cancel = 3; }
            "takeoff" => { ground(e); e.set(St::Air, "em0630_0003.mot"); }
            "land" => { e.flying = true; e.height = FLY_HEIGHT; e.set(St::Air, "em0630_0002.mot"); }
            "hover" => { e.flying = true; e.height = FLY_HEIGHT; e.set(St::Idle, "em0630_0000.mot"); }
            "hide" => { ground(e); e.hide_n += 1; e.set(St::Hide, "em0630_0002.mot"); }
            "summon" => { summon(e, pl); if e.drops.is_empty() { return Err("No bats free to summon".into()); } }
            "laugh" => { e.set(St::Editor, "em0630_0104.mot"); e.ed_loop = false; e.voices.push(vo("victory", 0xAB)); }
            other => return Err(format!("Elise has no move {other:?} (attack, somersault, volley, volley_b, dragon, guard, takeoff, land, hover, hide, summon, laugh)")),
        },
        Cmd::Hurt(k) => {
            let fwd = Vec3::new(e.yaw.sin(), 0., e.yaw.cos());
            let right = Vec3::new(fwd.z, 0., -fwd.x);
            let from = e.pos + match k.as_str() { "back" => -fwd, "left" => -right, "right" => right, _ => fwd } * 2.;
            if e.st == St::Editor { e.set(St::Idle, if e.flying { "em0630_0000.mot" } else { "em0630_0001.mot" }); }
            match k.as_str() {
                "shot" => { let c = e.body(); let o = c + fwd * 4. + Vec3::Y * 0.3; if e.shoot(o, (c - o).normalize(), 20, f32::INFINITY, fx).is_none() { return Err("The round missed".into()); } }
                "down" | "launch" => { e.hurt(30, from, fx); if e.hp > 1 { ground(e); e.set(St::Down, "em0440_1003.mot"); } }
                "light" => e.hurt(10, from, fx),
                _ => e.hurt(30, from, fx),
            }
        }
        Cmd::Kill => { let from = e.pos + Vec3::new(e.yaw.sin(), 0., e.yaw.cos()) * 2.; ground(e); if e.st == St::Editor { e.set(St::Idle, "em0630_0001.mot"); } e.hurt(e.hp + 100, from, fx); }
        Cmd::Scene { actor, player } => {
            if !e.clips.contains_key(actor.as_str()) { return Err(format!("{actor} is not in her bank")); }
            ground(e); e.set(St::Editor, actor); e.ed_loop = false; e.ed_scene = true;
            if let Some(pc) = player { let pp = &mut *play; pp.fight.external(&pp.fd, pc, false, 0.); pp.fight.paired = true; pp.fight.player.pos = e.pos.to_array(); pp.fight.player.loco.facing = e.yaw; }
        }
        Cmd::Pair { name, win } => match name.as_str() {
            // her coffin struck: the clash, won (the meter filled, every swing right) or lost (the bite)
            "clash" => {
                ground(e); close(e, 1.2);
                e.set(St::Hide, "em0630_000b.mot"); e.sub = 5;
                start_keen(e, play, arena, pl);
                e.auto = *win;
            }
            // finish_wait: with an outcome the finishing blow lands now, without it waits for the player's strike
            "finisher" => {
                ground(e); close(e, 1.3); e.hp = 1;
                if win.is_some() { e.set(St::Finisher, "em0630_1006.mot"); let pp = &mut *play; pp.fight.external(&pp.fd, "pl0030_0000.mot", false, 2.); }
                else { e.set(St::FinishWait, "em0630_1006.mot"); return Err("Finish-ready: F2, then strike her".into()); }
            }
            other => return Err(format!("Elise has no pair {other:?} (clash, finisher; the execution is its own entry)")),
        },
    }
    Ok(())
}
