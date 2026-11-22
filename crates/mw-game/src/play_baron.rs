//! The Black Baron stage (MW_STAGE=baron): room c503 (the DeathWatch heliport ring), cEm0623 the Black Baron, cEm0615
//! Mathilda at the west ropes and three cEm0965 oirans at the east ropes (research/BARON-LEVEL.md, addresses there).
//! MW_STAGE=mathilda is the added variant: Mathilda is the boss, the Baron watches from her ring side.
//!
//! The Baron's states as the native table (§3.1): wait (0000; beyond 4.5 m he steps in), threat (0001-0003), angry
//! (the phase-2 turn: 000c + camera em0623_c003, BGM bs03 -> bs05), steps (0011-0014), the attacks (combo 0011 +
//! 0100-0105 / 0200, mach punch 0018 -> 0106, screw drop kick 0018 -> 0202, shoryuken 0018 -> 0107, antiair 0107, BH
//! upper 0018 -> 010a / 010b, god hand 0018 -> 0108 pulling the player in, the Dempsey roll 0110 after blocked hits,
//! the rider kick 0203 + camera em0623_c002, the clash-opening grab 0018 -> 0003 -> 0300), avoid (0015-0017), guard
//! (0121), damage (the borrowed grunt reactions em0110_10a5 / 10a7 / 10a8), dizzy (000a -> 000b) and pl_dead (0002).
//! Damage intake (§3.2): avoid csv[1] / csv[3] %, guard csv[0] / csv[2] %, then the hit; before the first clash HP never
//! falls under max x csv[13] % - 10, so the clash comes first. Attack choice (§3.4): a weighted draw over the csv rows
//! that never repeats the last one. Clash mode 0x16 (§3.3): the grab 0301 / pl0030_0480, then a swing step per
//! exchange (0481 hat off, 0482 sunglasses off, 0483, 048a, 0484), 0485 / 0306 for -400 HP; the first win turns him
//! angry, a later one runs the finale chain 0487 / 0308, 0488 / 0309, the 048b meter, 0489 / 030a (slow motion), 030b /
//! 1051 (HP 1, camera em0623_c000) and 030c / 1052 (the helicopter, camera em0623_c001); a missed step is 0307 / 0486.
//! Mathilda (§5): ringside wait / walk along x = -8.2, and the bat combo when his shoryuken connects (em0610_0010 +
//! pl0030_1055 + camera pl0030_c493, hits at the 4213:403 cues). The oirans (§4): wait, cheer1 / cheer2, and the
//! item toss (em0960_0002 + camera em0960_c000) when the player's HP falls under 20 / 20 / 25 / 30 / 35 %: it0501 then
//! it0502, 900 frames apart.
//! [inferred] attack hit windows are placed on the clips' swing frames (the rig windows are not decoded); the kill after
//! the helicopter is the 030c chain ending in a dizzy wait that the next blow finishes (open in §9); heal amounts
//! (it0501 40, it0502 25) and the bat combo's damage (8 x 3) are picks; the swing steps are keyboard prompts (F / Q).
use bevy::prelude::*;
use mw_data::MotionBank;
use mw_sim::character::{sample_retargeted, strip_root_xz, Retarget};
use std::collections::HashMap;
use super::elise::{load_cams, CamClip};

const BARON_AT: Vec3 = Vec3::new(-0.003, 0.052, 5.096);
const MATHILDA_AT: Vec3 = Vec3::new(-8.2, -0.05, 0.);
const OIRAN_X: f32 = 7.9;
const RING: f32 = 7.6;
const BARON_HP: i32 = 2000;
const MATHILDA_HP: i32 = 1000;
/// em0623.csv (normal): guard / avoid % before the first clash, after it; phase-1 weights combo / mach / drop kick;
/// phase-2 weights combo / mach / drop / shoryuken / god hand / BH upper; clash HP %; clash damage.
const CSV: [i32; 15] = [60, 70, 30, 45, 40, 30, 30, 20, 20, 20, 20, 10, 10, 85, 400];
/// ボス攻撃1..8 (normal): combo, mach punch, drop kick, shoryuken, god hand, BH upper, Dempsey roll, rider kick.
const POWER: [i32; 8] = [5, 15, 15, 25, 40, 40, 10, 30];

pub fn on() -> bool { matches!(std::env::var("MW_STAGE").as_deref(), Ok("baron") | Ok("mathilda")) }
/// The added variant: Mathilda is the boss.
pub fn mathilda_boss() -> bool { std::env::var("MW_STAGE").as_deref() == Ok("mathilda") }

/// One skinned actor drawn in source and colour variants.
struct Actor {
    bank: Option<MotionBank>, clips: HashMap<String, usize>, roots: [Entity; 2], joints: [Option<Vec<Entity>>; 2],
    pos: Vec3, yaw: f32, clip: usize, frame: f32,
    /// a clip from another bank (Mathilda's borrowed RinRin moves): its name, sampled from `extra`
    borrowed: Option<usize>,
    /// an upper-body overlay from `extra` (clip, frame, weight): bones 1-14 (spine, head, arms) over the clip below
    /// (Mathilda's guard raised while she walks)
    upper: Option<(usize, f32, f32)>,
    /// a cutscene clip's facing correction: event motions (em0610_ev50_cut*) carry the event's own heading at frame 0,
    /// turned here to the gameplay idle's (her own em0610_0000)
    yaw_fix: f32,
}
/// Mathilda's bank for the variant: her own clips plus Jack's held-weapon swings (pl0030_14xx), which drive her rig by
/// bone id (bones 0-24 are the same skeleton; his 25 / 26 are his face, dropped) [added: she has no swing of her own].
fn mathilda_bank(own: Option<MotionBank>, jack: Option<MotionBank>) -> Option<MotionBank> {
    let mut own = own?;
    if let Some(jack) = jack {
        // (and his plain get-ups: RinRin's em0440_1004 is a handstand flip)
        for c in jack.clips.into_iter().filter(|c| c.name.starts_with("pl0030_14") || matches!(c.name.as_str(), "pl0030_1090.mot" | "pl0030_1091.mot" | "pl0030_1092.mot")) {
            let mut c = c; c.tracks.retain(|t| !(25..=26).contains(&t.bone)); own.clips.push(c);
        }
    }
    Some(own)
}
/// Her clips sample as rotations only (her own bone lengths: the cutscene motions and Jack's swings carry their own
/// translations, which stretched her), Jack's with his root height scaled to hers (1.24 / 1.08).
fn mathilda_retarget(name: &str) -> Retarget { Retarget { rotation_only: true, root_scale: if name.starts_with("pl0030_") { 1.24 / 1.08 } else { 1. } } }

/// Where she swings from (m between them at the hit frame).
const SWING_AT: f32 = 1.15;
/// How close her bat's head must come to his body's axis to land (m).
const BAT_REACH: f32 = 0.55;
/// Her bat's head in the world: the right hand (bone 9) carried on along the forearm (elbow bone 8 -> hand) by the bat's
/// length (wp0582, about 0.75 m), as she is posed this frame.
fn bat_head(b: &Baron) -> Vec3 {
    let Some(bank) = b.boss.bank.as_ref() else { return b.body(); };
    let clip = match b.boss.borrowed { Some(i) => match b.rin_bank.as_ref() { Some(r) => &r.clips[i], None => return b.body() }, None => &bank.clips[b.boss.clip] };
    let mut poses = sample_retargeted(&bank.bones, clip, b.boss.frame.min((clip.frames.max(2) - 1) as f32), mathilda_retarget(&clip.name));
    strip_root_xz(&bank.bones, &mut poses);
    let n = bank.bones.len(); poses[n].scale = [1.; 3];
    let f = mw_sim::fight::bone_frames(&bank.bones, &poses);
    let at = |id: i16| bank.bones.iter().position(|x| x.id == id).map(|i| Vec3::from_array(f[i].1));
    let (Some(elbow), Some(hand)) = (at(8), at(9)) else { return b.body(); };
    let local = hand + (hand - elbow).normalize_or(Vec3::Z) * 0.75;
    b.boss.pos + Quat::from_rotation_y(b.boss.yaw + b.boss.yaw_fix) * local
}

/// A clip's heading at frame 0 in model space: the thighs' right-to-left axis (bones 16 -> 20, +X when facing +Z) crossed
/// with up (the root's own rotation track carries the event heading, so the pelvis axis is read after it).
fn clip_heading(bank: &MotionBank, clip: &mw_data::Clip) -> f32 {
    // (sampled as it is drawn: her clips rotation-only)
    let retarget = if bank.bones.len() == 27 { mathilda_retarget(&clip.name) } else { Retarget::NATIVE };
    let p = sample_retargeted(&bank.bones, clip, 0., retarget);
    let f = mw_sim::fight::bone_frames(&bank.bones, &p);
    let at = |id: i16| bank.bones.iter().position(|b| b.id == id).map(|i| Vec3::from_array(f[i].1));
    let (Some(r), Some(l)) = (at(16), at(20)) else { return 0.; };
    let v = (l - r).cross(Vec3::Y);
    v.x.atan2(v.z)
}
impl Actor {
    fn new(bank: Option<MotionBank>, pos: Vec3, yaw: f32) -> Self {
        let clips = bank.as_ref().map(|b| b.clips.iter().enumerate().map(|(i, c)| (c.name.clone(), i)).collect()).unwrap_or_default();
        Self { bank, clips, roots: [Entity::PLACEHOLDER; 2], joints: [None, None], pos, yaw, clip: 0, frame: 0., borrowed: None, upper: None, yaw_fix: 0. }
    }
    fn play(&mut self, clip: &str) {
        self.frame = 0.; self.borrowed = None; self.yaw_fix = 0.;
        if let Some(&c) = self.clips.get(clip) {
            self.clip = c;
            if clip.contains("_ev") { if let Some(bank) = self.bank.as_ref() { self.yaw_fix = clip_heading(bank, &bank.clips[0]) - clip_heading(bank, &bank.clips[c]); } }
        }
    }
    fn len(&self, extra: Option<&MotionBank>) -> f32 {
        match self.borrowed { Some(b) => extra.map_or(1., |x| x.clips[b].frames as f32), None => self.bank.as_ref().map_or(1., |b| b.clips[self.clip].frames as f32) }
    }
    fn name(&self) -> &str { self.bank.as_ref().map_or("", |b| b.clips[self.clip].name.as_str()) }
    /// The clip drawn now, borrowed or own (for the log).
    fn shown_name<'a>(&'a self, extra: Option<&'a MotionBank>) -> &'a str { match self.borrowed { Some(i) => extra.map_or("?", |x| x.clips[i].name.as_str()), None => self.name() } }
    fn spawn(&mut self, files: [&str; 2], assets: &AssetServer, commands: &mut Commands) {
        for (k, f) in files.iter().enumerate() {
            self.roots[k] = commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(f.to_string()))), Transform::from_translation(self.pos), Visibility::Hidden)).id();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum St { Wait, Threat, Angry, Step, Combo, Mach, DropKick, Shoryuken, AntiAir, BhUpper, GodHand, Dempsey, RiderKick, Grab, Clash, Avoid, Guard, Damage, DmgPiyori, Piyori, PlDead, Hold, Finished,
              /// the actor editor plays a clip (play_editor.rs)
              Editor }
/// Ringside behaviour of Mathilda / an oiran.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Side { Wait, Walk, Cheer, Toss, BatCombo }
/// The recovery item an oiran threw: model (0 = it0501, 1 = it0502), where it is, its velocity, landed.
struct Item { kind: usize, roots: [Entity; 2], pos: Vec3, vel: Vec3, spin: f32, live: bool }
/// A detachable prop on the Baron's head (hat / sunglasses): attached, or flying free.
struct Prop { roots: [Entity; 2], free: Option<(Vec3, Vec3, f32)> }

#[derive(Resource)]
pub struct Baron {
    pub on: bool, pub mathilda_boss: bool, spawned: bool,
    boss: Actor, mathilda: Actor, oirans: Vec<(Actor, Side, f32)>, heli: Actor, heli_on: bool,
    /// RinRin's bank: Mathilda's borrowed moves in the variant
    rin_bank: Option<MotionBank>,
    hat: Prop, shades: Prop, items: Vec<Item>,
    /// Mathilda's bat (wp0582), on her right hand (bone 9) at (-0.12, -0.005, -0.07) (onPreMove 810F9E74)
    bat: Prop,
    pub st: St, sub: u8, timer: f32, pub hp: i32, max_hp: i32, rng: u32, landed: u32, hit_by: u32,
    atk_timer: f32, last_atk: u8, phase2: bool, clashes: u32, taken_since: i32, guard_hits: u32, defend_cd: f32,
    m_side: Side, m_timer: f32, m_target: Vec3,
    toss_step: usize, toss_cd: f32, toss_by: Option<usize>,
    /// the clash: step, its prompt and wanted swing, the meter, the player's start point
    qte: usize, want_v: bool, meter: f32, pub prompt: Option<String>, player_from: Vec3, pair_at: (Vec3, f32),
    /// a boss camera MOT running (name, frame, anchor, yaw)
    cam_run: Option<(String, f32, Vec3, f32)>,
    cams: HashMap<String, CamClip>, voices: Vec<u16>, pub bgm_switch: Option<&'static str>,
    /// the custom intro (variant): elapsed frames, the shot and line shown, done
    pub cine: Option<f32>, pub line: Option<(String, String)>,
    /// (variant) the player struck her heavy swing's wind-up: the power struggle starts next step; her SE this frame
    /// (block, id, where) for play.rs cue_sounds
    clash_req: bool, pub sfx: Vec<(u32, u32, Vec3)>,
    /// the last-life execution: 0 off, 1 the walk up, 2 the pair
    exec: u8,
    /// the hit being taken is a pistol round (its own wound effects, not the melee spray)
    shot: bool,
    /// the colour look is on (which of each actor's two models is drawn)
    colored: bool,
    /// (variant) frames her guard stays raised over her walk after a deflection
    guard_walk: f32,
    /// (variant) a flinch played over her walk after a round lands (RinRin's clip, its frame)
    hurt_walk: Option<(usize, f32)>,
    /// (variant) the pin's anchor for the player's half (his clip shifted onto where he stood)
    pin_at: Vec3,
    /// where her bat lands on him in the Baron's bat combo (the blood)
    bat_hit: Vec3,
    /// (variant intro) Enter held toward the skip (s), the frame the current shot began (its line's lip sync)
    skip_hold: f32, shot_at: f32,
    /// the actor editor: set up, its clip loops, the struggle's forced outcome
    ed_ready: bool, ed_loop: bool, auto: Option<bool>, ed_wait: u32,
    /// the editor's camera scene plays (his clip with its root travel from `pair_at`, the player's paired)
    ed_scene: bool,
    /// the clear sequence's line has played (the clear board may come)
    clear_said: bool,
}

pub struct BaronPlugin;
impl Plugin for BaronPlugin {
    fn build(&self, app: &mut App) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let on = on();
        let load = |f: &str| on.then(|| std::fs::read_to_string(root.join("assets/native").join(f)).ok().and_then(|s| MotionBank::parse(&s).ok())).flatten();
        let mb = mathilda_boss();
        let mut cams = HashMap::new();
        if on { for f in ["baron-camera.json", "oiran-camera.json"] { cams.extend(load_cams(f)); } }
        if on && mb { cams.extend(load_cams("rinrin-camera.json")); }
        let oirans = [0., -2.5, 2.5].iter().map(|z| (Actor::new(load("oiran-motion.json"), Vec3::new(OIRAN_X, -0.05, *z), -std::f32::consts::FRAC_PI_2), Side::Wait, 60. + 30. * z.abs())).collect();
        // the variant swaps them: Mathilda in the ring, the Baron at her ropes
        let (boss_at, boss_yaw, m_at, m_yaw) = if mb { (MATHILDA_AT, std::f32::consts::FRAC_PI_2, BARON_AT, std::f32::consts::PI) } else { (BARON_AT, std::f32::consts::PI, MATHILDA_AT, std::f32::consts::FRAC_PI_2) };
        app.insert_resource(Baron {
            on, mathilda_boss: mb, spawned: false,
            boss: Actor::new(if mb { mathilda_bank(load("mathilda-motion.json"), load("jack-player-motion.json")) } else { load("baron-motion.json") }, boss_at, boss_yaw),
            mathilda: Actor::new(load(if mb { "baron-motion.json" } else { "mathilda-motion.json" }), m_at, m_yaw),
            oirans, heli: Actor::new(load("heli-motion.json"), BARON_AT, 0.), heli_on: false,
            rin_bank: if mb { load("rinrin-motion.json") } else { None },
            hat: Prop { roots: [Entity::PLACEHOLDER; 2], free: None }, shades: Prop { roots: [Entity::PLACEHOLDER; 2], free: None }, items: vec![],
            bat: Prop { roots: [Entity::PLACEHOLDER; 2], free: None },
            st: St::Wait, sub: 0, timer: 0., hp: std::env::var("MW_BOSS_HP").ok().and_then(|v| v.parse().ok()).unwrap_or(if mb { MATHILDA_HP } else { BARON_HP }), max_hp: if mb { MATHILDA_HP } else { BARON_HP }, rng: 0x0b4e0,
            landed: 0, hit_by: 0, atk_timer: 60., last_atk: 255, phase2: false, clashes: 0, taken_since: 0, guard_hits: 0, defend_cd: 0.,
            m_side: Side::Wait, m_timer: 90., m_target: MATHILDA_AT, toss_step: 0, toss_cd: 0., toss_by: None,
            qte: 0, want_v: true, meter: 0., prompt: None, player_from: Vec3::ZERO, pair_at: (Vec3::ZERO, 0.),
            cam_run: None, cams, voices: vec![], bgm_switch: None,
            cine: (on && mb && std::env::var("MW_INTRO_PREVIEW").as_deref() != Ok("0") && !super::editor::on()).then_some(0.), line: None,
            ed_ready: false, ed_loop: false, auto: None, ed_wait: 0, ed_scene: false, clear_said: false,
            clash_req: false, exec: 0, shot: false, colored: true, guard_walk: 0., hurt_walk: None, sfx: vec![], pin_at: Vec3::ZERO, bat_hit: Vec3::ZERO, skip_hold: 0., shot_at: 0.,
        })
        .add_systems(Update, (spawn, bind, cine, pose, props, prompt, dialogue).chain());
    }
}

/// A pair clip's camera: the player clip's c-twin (pl0030_0483 runs camera c48c, 048a runs c483, 1051 c491 ...).
fn pair_cam(player_clip: &str) -> Option<&'static str> {
    Some(match player_clip {
        "pl0030_0480.mot" => "pl0030_c480.mot", "pl0030_0481.mot" => "pl0030_c481.mot", "pl0030_0482.mot" => "pl0030_c482.mot",
        "pl0030_0483.mot" => "pl0030_c48c.mot", "pl0030_048a.mot" => "pl0030_c483.mot", "pl0030_0484.mot" => "pl0030_c484.mot",
        "pl0030_0485.mot" => "pl0030_c485.mot", "pl0030_0486.mot" => "pl0030_c486.mot", "pl0030_0487.mot" => "pl0030_c487.mot",
        "pl0030_0488.mot" => "pl0030_c488.mot", "pl0030_0489.mot" => "pl0030_c489.mot", "pl0030_048b.mot" => "pl0030_c48d.mot",
        "pl0030_04aa.mot" => "pl0030_c4aa.mot", "pl0030_04ab.mot" => "pl0030_c4ab.mot", "pl0030_04ac.mot" => "pl0030_c4ac.mot", "pl0030_04ad.mot" => "pl0030_c4ad.mot",
        "pl0030_1050.mot" => "pl0030_c490.mot", "pl0030_1051.mot" => "pl0030_c491.mot", "pl0030_1052.mot" => "pl0030_c492.mot",
        "pl0030_1055.mot" => "pl0030_c493.mot", _ => return None,
    })
}

impl Baron {
    /// The last-life execution (play_execution.rs) [added; a stand-in on their struggles]: the boss walks up to the
    /// fallen player and plays the struggle through to its loss with no prompt. The Baron grabs him (0480 / 0301) and
    /// wins the exchange (0486 / 0307); Mathilda pins him under her bat (the beat-down pair) to its finishing blow.
    pub fn executing(&self) -> bool { self.exec > 0 }
    pub fn start_execution(&mut self, _play: &mut super::Play) -> bool {
        if !self.on || self.hp < 1 || matches!(self.st, St::Finished | St::Clash | St::Hold) { return false; }
        self.exec = 1; self.prompt = None; self.cam_run = None;
        true
    }
    fn roll(&mut self) -> f32 { self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (self.rng >> 8) as f32 / (1 << 24) as f32 }
    fn set(&mut self, st: St, clip: &str) { self.st = st; self.sub = 0; self.landed = 0; self.timer = 0.; self.boss.play(clip); }
    /// Mathilda (the variant) plays one of RinRin's clips on her own rig (same bone ids 0-26).
    fn set_rin(&mut self, st: St, clip: &str) {
        self.st = st; self.sub = 0; self.landed = 0; self.timer = 0.; self.boss.frame = 0.;
        self.boss.borrowed = self.rin_bank.as_ref().and_then(|b| b.clips.iter().position(|c| c.name == clip));
    }
    fn len(&self) -> f32 { self.boss.len(self.rin_bank.as_ref()) }
    pub fn body(&self) -> Vec3 { self.boss.pos + Vec3::Y * 1.1 }
    /// [added, VR] The boss model's animated joints (the shown model), for where the body really is in a paired clip.
    pub fn vr_joints(&self) -> Option<&Vec<Entity>> { self.boss.joints[self.colored as usize].as_ref() }
    /// [added, VR] The joint of bone `id` (3 the chest, 4 the neck) on the shown model.
    pub fn vr_joint(&self, id: i16) -> Option<Entity> { let i = self.boss.bank.as_ref()?.bones.iter().position(|b| b.id == id)?; self.vr_joints()?.get(i).copied() }
    /// A clip of the boss's bank by name (the chainsaw arm's wp0600 clips for the Baron's pairs live there).
    pub fn bank_clip(&self, name: &str) -> Option<&mw_data::Clip> { if !self.on { return None; } self.boss.bank.as_ref()?.clips.get(*self.boss.clips.get(name)?) }
    pub fn targetable(&self) -> bool { self.on && self.hp > 0 && !matches!(self.st, St::Clash | St::Hold | St::Finished) }
    fn paired(&self) -> bool { matches!(self.st, St::Clash | St::Hold) || self.m_side == Side::BatCombo || self.ed_scene }

    /// The camera: a pair clip's twin while the player is paired, else a boss / oiran camera MOT that is running.
    /// Beaten and his clear line said: the clear board, the results, then ev93 (play_clear.rs).
    pub fn cleared(&self) -> bool { self.on && self.clear_said }
    pub fn camera(&self, play: &super::Play) -> Option<(Vec3, Vec3, f32)> {
        if !self.on { return None; }
        // (variant) her finishing blow: low beside him as the bat goes up, then on the floor next to his head at the impact,
        // shaking hard and pushing in through the bounce
        if self.mathilda_boss && self.st == St::Clash && (self.sub == 2 || (self.sub == 0 && self.boss.frame >= FINISH_AT - 16.)) {
            // (where he lies: during the pin his object sits at the shifted clip anchor, 15 m off)
            let him = self.player_from;
            let her = self.boss.pos;
            let dir = Vec3::new(her.x - him.x, 0., her.z - him.z).normalize_or(Vec3::Z);
            let side = Vec3::new(dir.z, 0., -dir.x);
            if self.sub == 0 {
                // the bat going up for the last blow: low beside him, looking up at her
                let u = ((self.boss.frame - (FINISH_AT - 16.)) / 16.).clamp(0., 1.);
                let him = Vec3::new(him.x, her.y, him.z);
                return Some((him + side * (2.8 - 0.6 * u) - dir * 0.6 + Vec3::Y * 0.45, her + Vec3::Y * (1.2 + 0.3 * u), 0.85));
            }
            let t = self.timer;
            let shake = (1. - t / 20.).max(0.) * 0.09;
            let jolt = Vec3::new((t * 7.3).sin(), (t * 9.1).cos(), (t * 5.7).sin()) * shake;
            let push = (t / 24.).min(1.);
            let eye = him + side * (1.5 - 0.5 * push) - dir * 0.6 + Vec3::Y * (0.25 + 0.1 * push) + jolt;
            return Some((eye, Vec3::from_array(play.fight.player.pos) + Vec3::Y * 0.3 + jolt * 0.5, 0.95 - 0.15 * push));
        }
        // (variant) her pin: RinRin's keen-competition struggle view (em0440.dat pl0030_c4ab, its loop, in RinRin's frame:
        // the pinned player under her), anchored on him where he lies, facing her
        if self.mathilda_boss && self.st == St::Clash && play.fight.paired {
            if let Some(cam) = self.cams.get("pl0030_c4ab.mot") {
                let him = self.player_from; let her = self.boss.pos;
                let yaw = (her.x - him.x).atan2(her.z - him.z);
                let f = (self.boss.frame - PIN_FROM).max(0.) % (cam.frames() as f32 - 1.).max(1.);
                if std::env::var_os("MW_CAM_LOG").is_some() { eprintln!("[cam] tick {} mathilda pl0030_c4ab.mot f{f:.1}", play.tick); }
                return Some(cam.sample(f, Vec3::new(him.x, her.y, him.z), yaw));
            }
        }
        if self.paired() && play.fight.paired {
            let m = &play.fight.player.motion;
            let name = pair_cam(&play.fd.jack.clips.get(m.clip)?.name)?;
            if std::env::var_os("MW_CAM_LOG").is_some() { eprintln!("[cam] tick {} baron {name} f{:.1}", play.tick, m.frame); }
            let native = self.cams.get(name)?.sample(m.frame, self.pair_at.0, self.pair_at.1);
            // the last-life execution keeps both in the shot: the native camera while it frames them, else a side shot on
            // both [added; c486 looks across the ring early in his exchange win]
            if self.exec > 0 {
                let clip = &play.fd.jack.clips[m.clip];
                let r = mw_sim::character::root_xz(&play.fd.jack.bones, clip, m.frame.min((clip.frames.max(1) - 1) as f32));
                let him = self.pair_at.0 + Quat::from_rotation_y(self.pair_at.1) * Vec3::new(r[0], 0., r[1]) + Vec3::Y;
                let boss = self.boss.pos + Vec3::Y;
                let (eye, target, fov) = native;
                let fwd = (target - eye).normalize_or_zero();
                let seen = |p: Vec3| { let d = (p - eye).normalize_or_zero(); d.dot(fwd) > (fov * 0.5).cos() };
                if !(seen(him) && seen(boss)) {
                    let mid = (him + boss) * 0.5;
                    let along = Vec3::new(him.x - boss.x, 0., him.z - boss.z);
                    let side = Vec3::new(along.z, 0., -along.x).normalize_or(Vec3::X);
                    return Some((mid + side * (along.length() * 0.9 + 2.6) + Vec3::Y * 0.4, mid, 0.85));
                }
            }
            return Some(native);
        }
        let (name, frame, at, yaw) = self.cam_run.as_ref()?;
        Some(self.cams.get(name)?.sample(*frame, *at, *yaw))
    }
    /// Clips playing now with their motion cues (play.rs cue_sounds): key, clip, frame, where.
    pub fn motions(&self) -> Vec<(usize, String, f32, Vec3, f32)> {
        if !self.on || !self.spawned { return vec![]; }
        let mut v = vec![];
        if self.boss.borrowed.is_none() { v.push((0, self.boss.name().to_string(), self.boss.frame, self.boss.pos, self.boss.yaw)); }
        v.push((1, self.mathilda.name().to_string(), self.mathilda.frame, self.mathilda.pos, self.mathilda.yaw));
        v
    }

    /// A hit on the boss (onPreMove 810FAAA0): avoid, guard, then the damage and its reaction.
    fn hurt(&mut self, dmg: i32, atk: u32, from: Vec3, fx: &mut super::fx::Fx) -> bool {
        if !self.targetable() { return false; }
        let at = self.body();
        let to = Vec3::new(from.x - self.boss.pos.x, 0., from.z - self.boss.pos.z).normalize_or(Vec3::Z);
        // his attacks carry through hits once they are swinging (the armor flag +0x864 bit 0x10 and atk_keen) [the
        // per-attack armor windows are inferred: wind-ups stay interruptible]
        let armored = matches!(self.st, St::RiderKick | St::Grab | St::GodHand | St::Angry | St::Mach | St::DropKick | St::Shoryuken | St::AntiAir | St::BhUpper | St::Dempsey)
            && (self.sub >= 1 || matches!(self.st, St::RiderKick | St::Angry | St::Dempsey | St::AntiAir)) || (self.st == St::Combo && self.sub >= 1 && self.boss.frame > 4.);
        if self.st == St::Piyori && self.hp <= 1 {
            // the finishing blow on the dizzy wait
            fx.burst("core", 486, at, to.x.atan2(to.z), 1.2); fx.gore(at, to.x.atan2(to.z), 60, true, true);
            self.hp = 0; self.set(St::Finished, if self.mathilda_boss { "" } else { "em0110_10a8.mot" });
            if self.mathilda_boss { self.set_rin(St::Finished, "em0440_1003.mot"); }
            return true;
        }
        // (variant) walking or standing, she can deflect a round or a blow with her guard raised over her walk, without
        // stopping: a spark, no damage, the guard stays up a moment (and blocks what follows while it is up) [added]
        // (with the guard already up she still stops only some: 60 %)
        if self.mathilda_boss && matches!(self.st, St::Wait | St::Step) && self.roll() < if self.guard_walk > 0. { 0.6 } else if self.shot { 0.3 } else { 0.25 } {
            let to = Vec3::new(from.x - self.boss.pos.x, 0., from.z - self.boss.pos.z).normalize_or(Vec3::Z);
            // the deflection: the impact streaks off her guard (the guard spark, core 12, is a full-body starburst)
            fx.impact(self.body() + to * 0.35 + Vec3::Y * 0.3, to.x.atan2(to.z), 0.7, [1., 0.85, 0.45]);
            // raised for 48 frames from the first deflection; later ones keep it up a little longer, not afresh
            self.guard_walk = if self.guard_walk > 0. { (self.guard_walk + 12.).min(48.) } else { 48. };
            if std::env::var_os("MW_BARON_LOG").is_some() { eprintln!("[mathilda] deflects ({})", if self.shot { "round" } else { "blow" }); }
            return false;
        }
        let ready = self.defend_cd <= 0. && matches!(self.st, St::Wait | St::Step | St::Threat | St::Guard) && !(self.shot && self.mathilda_boss);
        let first = self.clashes == 0;
        if ready && self.st != St::Guard && !self.mathilda_boss && self.roll() * 100. < CSV[if first { 1 } else { 3 }] as f32 * 0.5 {
            // avoid: by hit type a side step, a back step or the dodge set (never the same dodge twice)
            self.defend_cd = 75.;
            let clip = match atk { 0x2A | 0x32 | 0x35 | 0x86 | 0x8C | 0xCC | 0xCE => if self.mathilda_boss { "em0440_0026.mot" } else { "em0623_0014.mot" },
                                   0x2E | 0x30 | 0x7D | 0x81 => if self.mathilda_boss { "em0440_0028.mot" } else { "em0623_0012.mot" },
                                   _ => if self.mathilda_boss { "em0440_0025.mot" } else { ["em0623_0015.mot", "em0623_0016.mot", "em0623_0017.mot"][(self.roll() * 3.) as usize % 3] } };
            if self.mathilda_boss { self.set_rin(St::Avoid, clip); } else { self.set(St::Avoid, clip); }
            return false;
        }
        if self.st == St::Guard || (ready && self.roll() * 100. < CSV[if first { 0 } else { 2 }] as f32 * 0.6) {
            // guard (0121, voice 0x112, guard spark 0xC); blocked hits feed the Dempsey-roll counter
            if self.st != St::Guard { if self.mathilda_boss { self.set_rin(St::Guard, "em0440_0003.mot"); } else { self.set(St::Guard, "em0623_0121.mot"); } self.voices.push(0x112); }
            self.guard_hits += 1; self.defend_cd = 20.;
            fx.burst("core", 12, at, to.x.atan2(to.z), 1.);
            return false;
        }
        let heavy = matches!(atk, 0x2A..=0x36 | 0x86 | 0x8C | 0xCC..=0xCE) && atk != 0x2D;
        self.voices.push(if heavy && !matches!(atk, 0x86 | 0x8C) { 0x113 } else { 0x114 });
        self.hp -= dmg; self.taken_since += dmg; self.atk_timer -= dmg as f32 * 0.5;
        // before the first clash HP stays above max x csv[13] % - 10 (the clash is forced first)
        if !self.mathilda_boss && self.clashes == 0 { self.hp = self.hp.max(self.max_hp * CSV[13] / 100 - 10); }
        // the hit: the native enemy-hit pair (core 1000 + the 1005 blood, as on RinRin and the grunts; core 200 is
        // an unrelated record); his own motion effects come from the op-16 cues (play.rs)
        let yaw = to.x.atan2(to.z);
        if !self.shot {
            fx.gore(at, yaw, dmg, heavy, false);
            // blood stays on her (the core stain, type 24, on both of her models)
            fx.burst_on("core", 535, at, yaw, self.boss.roots[self.colored as usize]);
        }
        if self.hp < 2 {
            self.hp = 1;
            if self.mathilda_boss { self.set_rin(St::Piyori, "em0440_1007.mot"); } else { self.set(St::DmgPiyori, "em0623_000a.mot"); }
            return true;
        }
        // (a pistol round does not stop her: she keeps walking and swinging through it; the wounds and blood still show)
        if armored || (self.shot && self.mathilda_boss) { return true; }
        if self.mathilda_boss {
            // her reactions are RinRin's (she has none of her own): light 1000 / 1001, heavy 1002, and a heavy blow under
            // half HP knocks her down (1003, then the get-up 1004)
            let down = heavy && self.hp * 2 < self.max_hp && self.roll() < 0.5;
            let clip = if down { "em0440_1003.mot" } else { ["em0440_1000.mot", "em0440_1001.mot", "em0440_1002.mot"][if heavy { 2 } else { (self.roll() * 2.) as usize }] };
            self.set_rin(St::Damage, clip);
        } else {
            let clip = if heavy { "em0110_10a8.mot" } else if self.roll() < 0.5 { "em0110_10a5.mot" } else { "em0110_10a7.mot" };
            self.set(St::Damage, clip);
        }
        true
    }

    /// A pistol round from `origin` along `dir`; returns the hit point.
    /// [added, VR] A fist at `at` thrown from `from`: within the boss's body he (or Mathilda) takes `damage` through
    /// the boss's own hurt (a heavy blow as the punch table's heavy type), with the punch's spray.
    pub fn vr_punch(&mut self, at: Vec3, from: Vec3, damage: i32, heavy: bool, fx: &mut super::fx::Fx) -> bool {
        if !self.targetable() { return false; }
        let c = self.body();
        if Vec2::new(at.x - c.x, at.z - c.z).length() > 0.62 || (at.y - c.y).abs() > 1.05 { return false; }
        self.hurt(damage, if heavy { 0x76 } else { 0x5e }, from, fx);
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
        // a round through the body: the wound (entry spray back toward the shooter, exit spray and mist, entry and exit
        // holes left on both of her models), a drop on the floor; the hit itself without the melee spray
        let hp = self.hp;
        self.shot = true;
        self.hurt(damage, 0, origin, fx);
        self.shot = false;
        if self.hp < hp {
            // walking or standing, the round shows on her upper body (RinRin's light flinch over her walk) and she keeps going
            if self.mathilda_boss && matches!(self.st, St::Wait | St::Step) {
                let k = if self.roll() < 0.5 { "em0440_1000.mot" } else { "em0440_1001.mot" };
                self.hurt_walk = self.rin_bank.as_ref().and_then(|b| b.clips.iter().position(|c| c.name == k)).map(|c| (c, 0.));
            }
            // the holes go on her torso, where the round crosses it: her body axis at the round's height (clamped to the
            // belly / chest, 0.95-1.5 m), the entry on the shooter's side of it, the exit behind (torso ~0.26 m deep)
            let flat = Vec3::new(dir.x, 0., dir.z).normalize_or(Vec3::Z);
            let axis = Vec3::new(self.boss.pos.x, self.boss.pos.y + (p.y - self.boss.pos.y).clamp(0.95, 1.5), self.boss.pos.z);
            let side = Vec3::new(flat.z, 0., -flat.x);
            let across = (p - axis).dot(side).clamp(-0.1, 0.1);
            let entry = axis - flat * 0.13 + side * across;
            // (on the model being drawn only: projecting onto both twins doubled the CPU posing, a hitch per round)
            fx.wound_holes(Some(self.boss.roots[self.colored as usize]), entry, flat, 0.26);
            // the spray where the round actually struck
            fx.burst("core", 1005, p, flat.x.atan2(flat.z) + std::f32::consts::PI, 0.35);
            fx.burst("core", 1005, p + flat * 0.26, flat.x.atan2(flat.z), 0.45);
            fx.splats.push((p + dir * 0.6, 0.45));
        }
        Some(p)
    }

    /// Phase-1 / phase-2 attack draw over the csv weights, never the last attack again (chkAtkStart / chkAtkStart2).
    fn pick_attack(&mut self, dist: f32, above: f32) -> St {
        if self.phase2 && above >= 0.8 { return St::AntiAir; }
        if self.phase2 && self.taken_since >= self.max_hp / 5 { self.taken_since = 0; return St::RiderKick; }
        let hp_pct = self.hp * 100 / self.max_hp;
        if (!self.phase2 && hp_pct <= CSV[13] && self.roll() < 0.75) || (self.phase2 && self.hp < CSV[14] - 10) { return St::Grab; }
        let table: &[(St, i32)] = if self.phase2 {
            &[(St::Combo, CSV[7]), (St::Mach, CSV[8]), (St::DropKick, CSV[9]), (St::Shoryuken, CSV[10]), (St::GodHand, CSV[11]), (St::BhUpper, CSV[12])]
        } else { &[(St::Combo, CSV[4]), (St::Mach, CSV[5]), (St::DropKick, CSV[6])] };
        let total: i32 = table.iter().enumerate().filter(|(i, _)| *i as u8 != self.last_atk).map(|(_, w)| w.1).sum();
        let mut r = self.roll() * total as f32;
        for (i, (st, w)) in table.iter().enumerate() {
            if i as u8 == self.last_atk { continue; }
            r -= *w as f32;
            if r <= 0. { self.last_atk = i as u8; let _ = dist; return *st; }
        }
        St::Combo
    }
}

fn spawn(mut bn: ResMut<Baron>, assets: Res<AssetServer>, mut commands: Commands) {
    if !bn.on || bn.spawned { return; }
    let b = &mut *bn;
    {
        b.spawned = true;
        if b.mathilda_boss { b.boss.spawn(["native/mathilda-source.glb", "native/mathilda-color.glb"], &assets, &mut commands); b.mathilda.spawn(["native/baron-source.glb", "native/baron-color.glb"], &assets, &mut commands); }
        else { b.boss.spawn(["native/baron-source.glb", "native/baron-color.glb"], &assets, &mut commands); b.mathilda.spawn(["native/mathilda-source.glb", "native/mathilda-color.glb"], &assets, &mut commands); }
        for o in b.oirans.iter_mut() { o.0.spawn(["native/oiran-source.glb", "native/oiran-color.glb"], &assets, &mut commands); o.0.play("em0960_0000.mot"); }
        b.heli.spawn(["native/heli-source.glb", "native/heli-color.glb"], &assets, &mut commands); b.heli.play("bl0522_030c.mot");
        for (prop, file) in [(&mut b.hat, "hat"), (&mut b.shades, "shades"), (&mut b.bat, "mbat")] {
            for (k, v) in ["source", "color"].iter().enumerate() {
                prop.roots[k] = commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(format!("native/{file}-{v}.glb")))), Transform::default(), Visibility::Hidden)).id();
            }
        }
        if b.mathilda_boss { b.set_rin(St::Wait, "em0440_0000.mot"); b.boss.play("em0610_0000.mot"); b.mathilda.play("em0623_0000.mot"); }
        else { b.set(St::Wait, "em0623_0000.mot"); b.mathilda.play("em0610_0000.mot"); }
    }
}

/// One 30 Hz game frame (FixedUpdate, after the fight tick).
pub fn step(mut bn: ResMut<Baron>, mut play: ResMut<super::Play>, mut fx: ResMut<super::fx::Fx>, assets: Res<AssetServer>, arena: Res<super::Arena>,
            mut editor: ResMut<super::editor::Editor>, intro: Res<super::intro::Intro>, mut commands: Commands) {
    if !bn.on || !bn.spawned { return; }
    let b = &mut *bn;
    b.sfx.clear();
    b.colored = play.colored;
    let pl = Vec3::from_array(play.fight.player.pos);
    let to = Vec3::new(pl.x - b.boss.pos.x, 0., pl.z - b.boss.pos.z);
    let dist = to.length();
    let above = pl.y - b.boss.pos.y;
    let player_alive = play.fight.player.hp > 0;
    let face = |b: &mut Baron, k: f32| { let want = to.x.atan2(to.z); b.boss.yaw = mw_sim::player::rad_lerp_range(b.boss.yaw, want, k, 0.4); };
    b.boss.frame += 1.;
    let len = b.len();
    let ended = b.boss.frame >= len - 1.;
    b.defend_cd -= 1.; b.toss_cd -= 1.;
    if let Some(c) = b.cam_run.as_mut() { c.1 += 1.; }

    // inspection: MW_BARON_TEST=clash | finale | bat | toss starts that piece next to the player at tick 40
    if play.tick == 40 {
        match std::env::var("MW_BARON_TEST").as_deref() {
            Ok("knock") if b.mathilda_boss => {
                b.boss.pos = pl + Vec3::new(0., 0., 1.8); b.last_atk = M_KNOCKDOWN as u8; b.st = St::Combo; b.sub = 0; b.landed = 0;
                b.boss.play(M_MOVES[M_KNOCKDOWN].clip);
            }
            Ok("clash") if b.mathilda_boss => { b.boss.pos = pl + Vec3::new(0., 0., 1.3); b.clash_req = true; }
            Ok("clash") => { b.boss.pos = pl + Vec3::new(0., 0., 1.3); b.set(St::Grab, "em0623_0300.mot"); b.sub = 1; }
            Ok("finale") => { b.clashes = 1; b.phase2 = true; b.hp = 500; b.boss.pos = pl + Vec3::new(0., 0., 1.3); b.set(St::Grab, "em0623_0300.mot"); b.sub = 1; }
            Ok("bat") => { b.boss.pos = pl + Vec3::new(0., 0., 1.3); b.phase2 = true; b.set(St::Shoryuken, "em0623_0107.mot"); b.sub = 1; }
            Ok("toss") => { play.fight.player.hp = 15; }
            _ => {}
        }
    }

    // the actor editor (em0623 the Baron, em0610 Mathilda as the boss): its commands, what it shows, the hold
    let wanted = if b.mathilda_boss { "em0610" } else { "em0623" };
    let frozen = editor.on && editor.id == wanted && !editor.ai;
    if editor.on && editor.id == wanted {
        if !b.ed_ready { b.ed_ready = true; let (s, ps) = (editor.spot, editor.player_spot); ed_reset(b, &mut play, &arena, s, ps); }
        if let Some(cmd) = editor.take() { let (s, ps) = (editor.spot, editor.player_spot); let r = ed_cmd(b, &cmd, &mut play, &arena, &mut fx, s, ps); editor.done(&cmd, r); }
        // a camera scene: his clock from the pair's origin; over, the pair ends
        if b.ed_scene {
            if b.st == St::Editor { editor.scene = Some((b.boss.frame, b.pair_at.0, b.pair_at.1)); }
            else { b.ed_scene = false; editor.scene = None; if play.fight.paired { end_pair(b, &mut play); } }
        }
        editor.anchor = (b.boss.pos, b.boss.yaw);
        editor.idle = matches!(b.st, St::Wait | St::Threat | St::Step) && !play.fight.paired;
        let id = if b.mathilda_boss { "em0610" } else { "em0623" };
        let name = super::editor::clip_label(id, b.boss.shown_name(b.rin_bank.as_ref()));
        let st = super::editor::phase_label(id, "state", format!("{:?}", b.st));
        let phase = if b.st == St::Clash { format!(": {}", super::editor::phase_label(id, "Clash", b.sub)) } else { String::new() };
        editor.info = format!("{name}   {:.0} / {:.0}   {st}{phase}   HP {}", b.boss.frame, b.len() - 1., b.hp);
        // (in the editor the finish plays out and resets: no defeat, no clear)
        if b.st == St::Finished { b.ed_wait += 1; if b.ed_wait as f32 > b.len() + 60. { b.ed_wait = 0; let (s, ps) = (editor.spot, editor.player_spot); ed_reset(b, &mut play, &arena, s, ps); } } else { b.ed_wait = 0; }
    }
    if frozen && matches!(b.st, St::Wait | St::Threat | St::Step) {
        let idle = if b.mathilda_boss { "em0610_0000.mot" } else { "em0623_0000.mot" };
        if b.st != St::Wait || b.boss.name() != idle || b.boss.borrowed.is_some() { b.set(St::Wait, idle); }
        if ended { b.boss.frame = 0.; }
        face(b, 0.1);
        b.atk_timer = b.atk_timer.max(60.);
    }
    if b.st == St::Editor && ended { if b.ed_loop { b.boss.frame = 0.; } else { let idle = if b.mathilda_boss { "em0610_0000.mot" } else { "em0623_0000.mot" }; b.set(St::Wait, idle); } }
    // the player's swing on the boss
    if let Some((hit, serial)) = play.fight.player_strike() {
        let c = Vec3::from_array(hit.center);
        let body = b.body();
        if serial != b.hit_by && Vec2::new(c.x - body.x, c.z - body.z).length() < hit.radius + 0.7 && (c.y - body.y).abs() < 1.4 {
            b.hit_by = serial;
            if b.hurt(hit.power, hit.atk, pl, &mut fx) { play.fight.hitstop = play.fight.hitstop.max(hit.hitstop); }
        }
    }

    // the boss's state
    let before = (b.boss.clip, b.boss.borrowed, b.boss.frame);
    // (held by the editor: no decisions from the wait; a pair it asked for still starts)
    let hold = frozen && matches!(b.st, St::Wait | St::Threat | St::Step) && !b.clash_req;
    if b.exec > 0 { step_execution(b, &mut play, &mut fx, &arena, dist, to, ended); }
    else if hold {}
    else if b.mathilda_boss { step_mathilda_boss(b, &mut play, &mut fx, &arena, dist, to, player_alive, ended, len); }
    else { step_baron(b, &mut play, &mut fx, &arena, dist, to, above, player_alive, ended, len); }
    // root motion (as the original's motion-driven movement): the clip's own travel this frame, turned by his yaw and slid
    // against the stage; not in the paired clips (grab, clash, hold: placed by the pair) or the homing rider kick
    if !b.mathilda_boss && before.0 == b.boss.clip && before.1.is_none() && b.boss.borrowed.is_none() && b.boss.frame >= 1.
        && !matches!(b.st, St::Grab | St::Clash | St::Hold | St::Finished | St::PlDead | St::RiderKick) {
        if let Some(bank) = b.boss.bank.as_ref() {
            let clip = &bank.clips[b.boss.clip];
            let (r0, r1) = (mw_sim::character::root_xz(&bank.bones, clip, b.boss.frame - 1.), mw_sim::character::root_xz(&bank.bones, clip, b.boss.frame));
            let d = Quat::from_rotation_y(b.boss.yaw) * Vec3::new(r1[0] - r0[0], 0., r1[1] - r0[1]);
            if d.length() < 0.5 {
                let (m, _) = mw_sim::collision::slide(&arena.0, b.boss.pos.to_array(), [d.x, d.z]);
                b.boss.pos.x += m[0]; b.boss.pos.z += m[1];
            }
        }
    }
    if !player_alive && b.exec == 0 && !matches!(b.st, St::PlDead | St::Finished | St::Clash | St::Hold) {
        // his win line: 0x11A when the player has no continues left, else 0x115 (BARON-LEVEL.md 1.4)
        b.voices.push(if play.continues == 0 { 0x11A } else { 0x115 });
        if b.mathilda_boss { b.set_rin(St::PlDead, "em0440_0002.mot"); } else { b.set(St::PlDead, "em0623_0002.mot"); }
    }
    let _ = face;
    if let Some(y) = super::floor(&arena.0, b.boss.pos + Vec3::Y * 0.5) { if !b.paired() { b.boss.pos.y = y; } }
    // bodies do not overlap (CharColRadius 0.8 for the Baron, 0.3 for Mathilda): each gives half the overlap
    if !b.paired() && !play.fight.paired && b.st != St::Finished {
        let min = if b.mathilda_boss { 0.65 } else { 0.95 };
        let d = Vec3::new(pl.x - b.boss.pos.x, 0., pl.z - b.boss.pos.z);
        if d.length() < min {
            let push = d.normalize_or(Vec3::Z) * (min - d.length()) * 0.5;
            b.boss.pos -= push;
            let (m, _) = mw_sim::collision::slide(&arena.0, play.fight.player.pos, [push.x, push.z]);
            play.fight.player.pos[0] += m[0]; play.fight.player.pos[2] += m[1];
        }
    }
    b.boss.pos.x = b.boss.pos.x.clamp(-RING, RING); b.boss.pos.z = b.boss.pos.z.clamp(-RING, RING);
    if b.mathilda_boss && !b.paired() && b.cine.is_none() { if let Some(y) = super::floor(&arena.0, b.boss.pos + Vec3::Y) { b.boss.pos.y = y; } }

    step_ringside(b, &mut play, &mut fx, &assets, &mut commands);
    step_items(b, &mut play, &mut fx);
    if std::env::var_os("MW_BARON_LOG").is_some() && play.tick % 30 == 0 {
        eprintln!("[baron] tick {} {:?}/{} {} f{:.0} hp {} phase2 {} clashes {} dist {:.1} player hp {} side {:?} toss {}",
                  play.tick, b.st, b.sub, b.boss.shown_name(b.rin_bank.as_ref()), b.boss.frame, b.hp, b.phase2, b.clashes, dist, play.fight.player.hp, b.m_side, b.toss_step);
    }
    play.boss = Some((b.hp.max(0) as i16, b.max_hp as i16, b.body()));
    for v in b.voices.drain(..) { play.fight.events.push(mw_sim::fight::FightEvent::Voice(v)); }
    if b.st == St::Finished && ended && !play.fight.victory { play.fight.victory = true; }
    // after ev51 the boss callback runs the clear sequence 80124258: its line 0x1027, the board, the results (play_clear.rs)
    if b.st == St::Finished && play.fight.victory && !b.mathilda_boss && !b.clear_said && !intro.outro_pending() && !intro.active() && !editor.on {
        b.clear_said = true; b.voices.push(0x1027);
    }
}

/// A melee window: inside frames [a, b) and within `reach`, once per `bit`.
fn swing(b: &mut Baron, play: &mut super::Play, (a, e): (f32, f32), reach: f32, atk: u32, power: i32, bit: u32, dist: f32) -> bool {
    if (a..e).contains(&b.boss.frame) && b.landed & bit == 0 && dist < reach {
        let p = &mut *play;
        if p.fight.enemy_hits_player(&p.fd, atk, power, b.boss.pos.to_array()) { b.landed |= bit; return true; }
    }
    false
}

/// Start a pair: the player plays `player` with the boss `boss` clip, both anchored at the boss facing the player.
fn pair(b: &mut Baron, play: &mut super::Play, player: &str, boss: &str) {
    b.boss.play(boss);
    let pp = &mut *play; pp.fight.external(&pp.fd, player, false, 4.);
    play.fight.paired = true;
    play.fight.player.pos = b.pair_at.0.to_array(); play.fight.player.loco.facing = b.pair_at.1;
}

/// The pair is over: the player keeps his clip's final root travel from the anchor.
fn end_pair(b: &Baron, play: &mut super::Play) {
    let fd = &play.fd;
    let clip = &fd.jack.clips[play.fight.player.motion.clip];
    let r = mw_sim::sample(&fd.jack.bones, clip, (clip.frames - 1) as f32)[fd.jack.bones.len()].translation;
    let at = b.pair_at.0 + Quat::from_rotation_y(b.pair_at.1) * Vec3::new(r[0], 0., r[2]);
    play.fight.player.pos = [at.x, b.player_from.y, at.z];
    play.fight.paired = false;
    play.fight.end_external();
}

#[allow(clippy::too_many_arguments)]
fn step_baron(b: &mut Baron, play: &mut super::Play, fx: &mut super::fx::Fx, _arena: &super::Arena, dist: f32, to: Vec3, above: f32, player_alive: bool, ended: bool, len: f32) {
    let face = |b: &mut Baron, k: f32| { let want = to.x.atan2(to.z); b.boss.yaw = mw_sim::player::rad_lerp_range(b.boss.yaw, want, k, 0.4); };
    let windup = |b: &mut Baron, next: &str| { if b.sub == 0 && b.boss.frame >= 16. { b.sub = 1; b.boss.play(next); } };
    match b.st {
        St::Wait => {
            face(b, 0.15);
            if ended { b.boss.frame = 0.; }
            b.atk_timer -= 1.;
            // chkItemInStart: the first oiran tosses a recovery item when the player is low
            if !b.mathilda_boss && b.toss_cd <= 0. && b.toss_step < 5 && player_alive && b.toss_by.is_none() {
                let pct = [20, 20, 25, 30, 35][b.toss_step];
                if (play.fight.player.hp as i32) < pct { b.toss_by = Some(0); }
            }
            if dist > 4.5 && b.atk_timer > 0. { b.set(St::Step, "em0623_0011.mot"); }
            else if b.atk_timer <= 0. && player_alive && b.toss_by.is_none() {
                let st = b.pick_attack(dist, above);
                b.atk_timer = 30. + b.roll() * 20. + if b.phase2 { 0. } else { 15. };
                match st {
                    St::Combo => b.set(St::Combo, "em0623_0011.mot"),
                    St::AntiAir => b.set(St::AntiAir, "em0623_0107.mot"),
                    St::RiderKick => { b.set(St::RiderKick, "em0623_0203.mot"); b.cam_run = Some(("em0623_c002.mot".into(), 0., b.boss.pos, b.boss.yaw)); }
                    St::Grab => b.set(St::Grab, "em0623_0018.mot"),
                    // (0018 backs him off 2.2 m: only the grab opens with it; the rest wind up from the stance)
                    other => { b.set(other, "em0623_0000.mot"); if other == St::Mach { b.voices.push(0x10E); } }
                }
            } else if b.guard_hits > 3 { b.guard_hits = 0; b.set(St::Dempsey, "em0623_0110.mot"); }
            else if b.roll() < 0.002 { b.voices.push(0x11B); let k = (b.roll() * 3.) as usize % 3; b.set(St::Threat, ["em0623_0001.mot", "em0623_0002.mot", "em0623_0003.mot"][k]); }
        }
        St::Threat => { face(b, 0.1); if ended { b.set(St::Wait, "em0623_0000.mot"); } }
        St::Step => {
            face(b, 0.25);
            if ended { if dist > 3. { b.boss.frame = 0.; } else { b.set(St::Wait, "em0623_0000.mot"); } }
        }
        // the combo: step in, then two or three of 0100-0105 / 0200
        St::Combo => {
            if b.sub == 0 { face(b, 0.3); if ended || dist < 1.4 { b.sub = 1; b.timer = 2. + (b.roll() * 2.).floor(); let k = ["em0623_0100.mot", "em0623_0101.mot", "em0623_0102.mot", "em0623_0103.mot", "em0623_0104.mot", "em0623_0105.mot", "em0623_0200.mot"][(b.roll() * 7.) as usize % 7]; b.boss.play(k); b.landed = 0; } }
            else {
                if b.boss.frame < 6. { face(b, 0.3); }
                swing(b, play, (len * 0.3, len * 0.55), 1.9, 0x172, POWER[0], 1, dist);
                if ended {
                    b.timer -= 1.;
                    if b.timer <= 0. || dist > 2.6 { b.set(St::Wait, "em0623_0000.mot"); }
                    else { let k = ["em0623_0100.mot", "em0623_0101.mot", "em0623_0102.mot", "em0623_0103.mot", "em0623_0104.mot", "em0623_0105.mot", "em0623_0200.mot"][(b.roll() * 7.) as usize % 7]; b.boss.play(k); b.landed = 0; }
                }
            }
        }
        // the mach punch: a flurry (0106, effect 0x1A)
        St::Mach => {
            windup(b, "em0623_0106.mot");
            if b.sub == 1 {
                if b.boss.frame < 20. { face(b, 0.2); }
                let k = ((b.boss.frame - 20.) / 8.).floor();
                if (0. ..5.).contains(&k) { swing(b, play, (20. + k * 8., 24. + k * 8.), 2.1, 0x18C, POWER[1] / 3, 1 << (k as u32), dist); }
                if ended { b.set(St::Wait, "em0623_0000.mot"); }
            } else { face(b, 0.3); }
        }
        // the screw drop kick (0202): he launches at the player
        St::DropKick => {
            windup(b, "em0623_0202.mot");
            if b.sub == 1 {
                if b.boss.frame < 14. { face(b, 0.3); }
                swing(b, play, (28., 52.), 1.7, 0x18E, POWER[2], 1, dist);
                if ended { b.set(St::Wait, "em0623_0000.mot"); }
            } else { face(b, 0.3); }
        }
        // the shoryuken (0107): when it catches with Mathilda at the ropes, her bat combo follows
        St::Shoryuken | St::AntiAir => {
            if b.st == St::Shoryuken { windup(b, "em0623_0107.mot"); } else if b.sub == 0 { b.sub = 1; }
            if b.sub == 1 {
                if b.boss.frame < 14. { face(b, 0.3); }
                if swing(b, play, (18., 34.), 1.6, if b.st == St::AntiAir { 0x161 } else { 0x17E }, POWER[3], 1, dist) && b.st == St::Shoryuken && player_alive && b.m_side != Side::BatCombo {
                    b.voices.push(0x11F);
                    // startCatch on 0x27 in shoryuken: Mathilda's atk_bat with the player paired to her
                    b.m_side = Side::BatCombo; b.mathilda.play("em0610_0010.mot");
                    b.player_from = Vec3::from_array(play.fight.player.pos);
                    let toward_ring = (-b.mathilda.pos.x).atan2(-b.mathilda.pos.z);
                    b.mathilda.yaw = toward_ring;
                    // his clip (pl0030_1055) starts at the launcher, 15.5 m off: placed as the Mathilda boss's pin places
                    // it, so that he comes down at her feet and her third blow lands on his hips; its camera (c493) rides
                    // the same origin
                    let (pin, hit) = bat_anchor(&b.mathilda, b.mathilda.pos, toward_ring, play);
                    b.pin_at = pin; b.bat_hit = hit;
                    b.pair_at = (pin, toward_ring);
                    let pp = &mut *play; pp.fight.external(&pp.fd, "pl0030_1055.mot", false, 4.);
                    play.fight.paired = true; play.fight.player.pos = pin.to_array(); play.fight.player.loco.facing = toward_ring;
                    b.set(St::Hold, "em0623_0000.mot");
                    return;
                }
                if ended { b.set(St::Wait, "em0623_0000.mot"); }
            } else { face(b, 0.3); }
        }
        St::BhUpper => {
            if b.sub == 0 && b.boss.frame >= 16. { b.sub = 1; let k = if b.roll() < 0.5 { "em0623_010a.mot" } else { "em0623_010b.mot" }; b.boss.play(k); }
            if b.sub == 1 {
                if b.boss.frame < 12. { face(b, 0.3); }
                swing(b, play, (16., 30.), 1.7, 0x17D, POWER[5], 1, dist);
                if ended { b.set(St::Wait, "em0623_0000.mot"); }
            } else { face(b, 0.3); }
        }
        // god hand (0108): pulls the player in (0.1 x (1 - d / 8) a frame inside 8 m), then the blow on contact
        St::GodHand => {
            windup(b, "em0623_0108.mot");
            if b.sub == 1 {
                face(b, 0.1);
                if (30. ..150.).contains(&b.boss.frame) && dist < 8. && dist > 1.0 {
                    let pull = -to.normalize_or_zero() * 0.1 * (1. - dist / 8.);
                    let pp = &mut play.fight.player.pos; pp[0] += pull.x; pp[2] += pull.z;
                }
                if swing(b, play, (140., 175.), 1.6, 0x18E, POWER[4], 1, dist) { fx.impact(b.body(), b.boss.yaw, 1.5, [1., 0.12, 0.08]); }
                if ended { b.set(St::Wait, "em0623_0000.mot"); }
            }
        }
        // the Dempsey roll (0110 looped four times): the guard counter
        St::Dempsey => {
            face(b, 0.3);
            swing(b, play, (10., 24.), 1.8, 0x172, POWER[6], 1 << (b.sub as u32), dist);
            if ended { b.sub += 1; b.boss.frame = 0.; if b.sub >= 4 { b.set(St::Wait, "em0623_0000.mot"); } }
        }
        // the rider kick (0203, camera em0623_c002): the leap lands around frame 120
        St::RiderKick => {
            if b.boss.frame < 90. { face(b, 0.2); }
            if (95. ..125.).contains(&b.boss.frame) { let d = to.normalize_or_zero() * (dist.min(6.) / 30.); b.boss.pos += d; }
            if swing(b, play, (118., 150.), 2.6, 0x18E, POWER[7], 1, dist) { fx.impact(Vec3::from_array(play.fight.player.pos) + Vec3::Y, 0., 1.5, [1., 0.12, 0.08]); }
            if ended { b.cam_run = None; b.set(St::Wait, "em0623_0000.mot"); }
        }
        // atk_keen: 0018 -> 0003 -> 0300, the grab; a catch opens the clash
        St::Grab => {
            match b.sub {
                0 => { face(b, 0.3); if ended { b.sub = 1; b.boss.play("em0623_0300.mot"); } }
                _ => {
                    if b.boss.frame < 16. { face(b, 0.3); }
                    if (14. ..40.).contains(&b.boss.frame) && dist < 1.5 && b.landed == 0 && player_alive && { let pp = &mut *play; pp.fight.enemy_hits_player(&pp.fd, 0x18E, 0, b.boss.pos.to_array()) } {
                        b.landed = 1;
                        b.voices.push(0x116);
                        b.player_from = Vec3::from_array(play.fight.player.pos);
                        let yaw = to.x.atan2(to.z);
                        b.boss.yaw = yaw; b.pair_at = (b.boss.pos, yaw);
                        b.st = St::Clash; b.sub = 0; b.qte = 0; b.prompt = None;
                        pair(b, play, "pl0030_0480.mot", "em0623_0301.mot");
                        // the clash cut-in (8029D350 phase 0)
                        super::cutin::request();
                        return;
                    }
                    if ended { b.set(St::Wait, "em0623_0000.mot"); }
                }
            }
        }
        St::Clash => step_clash(b, play, fx, ended),
        St::Hold => {
            if ended { b.boss.frame = 0.; }
            if b.m_side != Side::BatCombo { b.set(St::Wait, "em0623_0000.mot"); }
        }
        St::Angry => {
            if ended { b.cam_run = None; b.bgm_switch = Some("bs05"); b.set(St::Wait, "em0623_0000.mot"); }
        }
        St::Avoid | St::Damage => { if ended { b.set(St::Wait, "em0623_0000.mot"); } }
        St::Guard => { face(b, 0.2); if ended { if b.guard_hits > 3 { b.guard_hits = 0; b.set(St::Dempsey, "em0623_0110.mot"); } else { b.set(St::Wait, "em0623_0000.mot"); } } }
        St::DmgPiyori => { if ended { b.set(St::Piyori, "em0623_000b.mot"); } }
        // dizzy: with HP 1 he waits for the finishing blow; stars every 25-45 frames
        St::Piyori => {
            if ended { b.boss.frame = 0.; }
            b.timer += 1.;
            if b.timer as u32 % 35 == 0 { fx.burst("core", 0x4C, b.body() + Vec3::Y * 0.9, 0., 0.8); }
            if b.hp > 1 && b.timer > 90. { b.set(St::Wait, "em0623_0000.mot"); }
        }
        // the taunt over him; back to the fight once he is up again (a continue, or the restart after a game over)
        St::PlDead => { if player_alive { b.set(St::Wait, "em0623_0000.mot"); } else if ended { b.boss.play("em0623_0000.mot"); } }
        St::Finished => { if ended { b.boss.frame = len - 1.; } }
        // (the editor's clip: step() plays it and returns to the wait)
        St::Editor => {}
    }
}

/// Clash mode 0x16: the swing steps, the -400 exchange, the angry turn or the finale chain.
fn step_clash(b: &mut Baron, play: &mut super::Play, fx: &mut super::fx::Fx, ended: bool) {
    const STEPS: [(&str, &str); 4] = [("pl0030_0481.mot", "em0623_0302.mot"), ("pl0030_0482.mot", "em0623_0303.mot"), ("pl0030_0483.mot", "em0623_0304.mot"), ("pl0030_048a.mot", "em0623_030d.mot")];
    let (mut q_v, mut q_s) = play.swing_keys();
    let mut shake = play.latched_shake();
    // inspection: MW_BARON_AUTO=1 answers every prompt right (and the actor editor's forced outcome: right, or nothing)
    if std::env::var_os("MW_BARON_AUTO").is_some() || b.auto == Some(true) { shake = true; if b.sub == 1 && b.timer > 12. { q_v = b.want_v; q_s = !b.want_v; } }
    if b.auto == Some(false) { shake = false; q_v = false; q_s = false; }
    let fail = |b: &mut Baron, play: &mut super::Play| {
        // the Baron wins the exchange (0307 / 0486, voice 0x118 at frame 80): hat and sunglasses back on
        b.sub = 50; b.prompt = None; b.voices.push(0x118);
        pair(b, play, "pl0030_0486.mot", "em0623_0307.mot");
        let pp = &mut *play; pp.fight.enemy_hits_player(&pp.fd, 0x18E, 20, b.boss.pos.to_array());
        play.fight.paired = true;
    };
    match b.sub {
        // the grab plays out, then the first swing prompt
        0 => { if ended { b.sub = 1; b.qte = 0; b.timer = 0.; b.want_v = b.roll() < 0.5; } }
        // a swing step: the right swing within 45 frames plays its pair, a wrong one or none loses the exchange
        1 => {
            b.timer += 1.;
            b.prompt = Some(format!("SWING {} [{}]  {}/4", if b.want_v { "DOWN" } else { "SIDE" }, if b.want_v { "F" } else { "Q" }, b.qte));
            if (b.want_v && q_v) || (!b.want_v && q_s) {
                b.prompt = None; b.sub = 2;
                let (p, e) = STEPS[b.qte];
                pair(b, play, p, e);
            } else if q_v || q_s || b.timer > 45. { fail(b, play); }
        }
        2 => {
            // the hat flies at 0302 frame 28, the sunglasses at 0303 frame 29
            if b.qte == 0 && b.boss.frame >= 28. && b.hat.free.is_none() { b.hat.free = Some((b.boss.pos + Vec3::Y * 1.95, Vec3::new(b.roll() - 0.5, 1.6, -1.) * 0.06, 0.)); }
            if b.qte == 1 && b.boss.frame >= 29. && b.shades.free.is_none() { b.shades.free = Some((b.boss.pos + Vec3::Y * 1.85, Vec3::new(b.roll() - 0.5, 1.2, -1.) * 0.05, 0.)); }
            if ended {
                b.qte += 1;
                if b.qte < 4 { b.sub = 1; b.timer = 0.; b.want_v = b.roll() < 0.5; }
                else { b.sub = 3; pair(b, play, "pl0030_0484.mot", "em0623_0305.mot"); }
            }
        }
        3 => { if ended { b.sub = 4; b.landed = 0; pair(b, play, "pl0030_0485.mot", "em0623_0306.mot"); } }
        // 0306 / 0485: -csv[14] at frame 8
        4 => {
            if b.boss.frame >= 8. && b.landed == 0 {
                b.landed = 1; b.hp = (b.hp - CSV[14]).max(1); play.fight.hitstop = play.fight.hitstop.max(8);
                fx.impact(b.body(), b.boss.yaw, 1.6, [1., 0.12, 0.08]); fx.burst("core", 1005, b.body(), b.boss.yaw, 1.4);
            }
            if ended {
                b.clashes += 1;
                if b.clashes == 1 {
                    // the first win: voice 0x117, then the angry turn (000c + camera c003, BGM bs05 after it)
                    b.voices.push(0x117); end_pair(b, play); b.prompt = None;
                    b.phase2 = true;
                    b.set(St::Angry, "em0623_000c.mot"); b.cam_run = Some(("em0623_c003.mot".into(), 0., b.boss.pos, b.boss.yaw));
                } else if b.hp <= CSV[14] {
                    b.sub = 5; pair(b, play, "pl0030_0487.mot", "em0623_0308.mot");
                } else { end_pair(b, play); b.set(St::Wait, "em0623_0000.mot"); }
            }
        }
        5 => { if ended { b.sub = 6; pair(b, play, "pl0030_0488.mot", "em0623_0309.mot"); } }
        // the 048b meter: shake to fill it (0.005 x input a frame in the original's units)
        6 => {
            if ended && b.sub == 6 { b.sub = 7; b.meter = 0.3; pair(b, play, "pl0030_048b.mot", "em0623_0309.mot"); b.boss.frame = 30.; }
        }
        7 => {
            b.boss.frame = 30.;
            b.meter = (b.meter + if shake { 0.022 } else { -0.004 }).clamp(0., 1.);
            b.prompt = Some(format!("TAP A/D!  {}%", (b.meter * 100.) as i32));
            if b.meter >= 1. { b.prompt = None; b.sub = 8; pair(b, play, "pl0030_0489.mot", "em0623_030a.mot"); }
            else if play.fight.external_done() { fail(b, play); }
        }
        // 030a (slow motion in the original), then 030b (HP 1 at frame 26, camera em0623_c000)
        8 => { if ended { b.sub = 9; pair(b, play, "pl0030_1051.mot", "em0623_030b.mot"); b.landed = 0; } }
        9 => {
            if b.boss.frame >= 26. && b.landed == 0 { b.landed = 1; b.hp = 1; fx.impact(b.body(), b.boss.yaw, 2., [1., 0.12, 0.08]); }
            if ended { b.sub = 10; pair(b, play, "pl0030_1052.mot", "em0623_030c.mot"); b.heli_on = true; b.heli.play("bl0522_030c.mot"); b.heli.pos = b.boss.pos; b.heli.yaw = b.boss.yaw; }
        }
        // 030c: the helicopter; then the dizzy finisher wait with HP 1
        10 => { if ended { b.heli_on = false; end_pair(b, play); b.prompt = None; b.set(St::Piyori, "em0623_000b.mot"); } }
        // lost: back to waiting, hat and sunglasses restored
        50 => { if ended { end_pair(b, play); b.hat.free = None; b.shades.free = None; b.set(St::Wait, "em0623_0000.mot"); } }
        _ => {}
    }
}

/// The variant: Mathilda as the boss, with her own idle / walk / bat beat-down and RinRin's moves on her rig.
#[allow(clippy::too_many_arguments)]
/// One of her bat moves: Jack's own two-handed held-weapon swings (pl0030_14xx) on her rig (mathilda_bank, rotations
/// only); the hit window comes from each clip's own sequence (op 19); it lands when her bat's head reaches him
/// (bat_head), from 2 frames before the window to 8 after; power is a pick, `heavy` swings can pin him.
/// (Her cutscene motions are not attacks: they drifted, grew her and played scenes against people who are not there.)
struct MMove { clip: &'static str, hit: (f32, f32), atk: u32, power: i32, heavy: bool }
const M_MOVES: [MMove; 4] = [
    // (only Jack's clips whose own sequence opens a hit window, op 19 on the right hand: 1407, 1409 and 140d have none,
    // they are not swings; the explorer review, 2026-11-11)
    MMove { clip: "pl0030_140b.mot", hit: (8., 14.), atk: 0x172, power: 4, heavy: false },    // swing
    MMove { clip: "pl0030_140c.mot", hit: (0., 6.), atk: 0x172, power: 4, heavy: false },     // its follow-through
    MMove { clip: "pl0030_1414.mot", hit: (4., 9.), atk: 0x17D, power: 6, heavy: true },     // wound-up swing
    MMove { clip: "pl0030_140a.mot", hit: (16., 30.), atk: 0x161, power: 9, heavy: true },   // big smash: knocks down
];
/// The swing that knocks him down (an ATHIT in the player's knockdown table, 0x161).
const M_KNOCKDOWN: usize = 3;
/// He lies this long (frames) before she walks over to pin him.
const PIN_WAIT: u32 = 45;

/// Her taunt at range: the bat on her shoulder (an ev50 pose; grounded and rotation-only).
const M_THREATS: [&str; 1] = ["em0610_ev50_cut11.mot"];

/// Mathilda as the boss [added]: shaped on the original bosses' loops (RinRin's wait / step / attack decision on a timer
/// that shortens with lost HP, the Baron's combo strings, guard and avoid by roll, a grab, a power struggle): every
/// attack is her bat, from her own clips; RinRin's clips fill what she has none of (guard, dodges, damage, down, dizzy,
/// the clash pair).
fn step_mathilda_boss(b: &mut Baron, play: &mut super::Play, fx: &mut super::fx::Fx, arena: &super::Arena, dist: f32, to: Vec3, player_alive: bool, ended: bool, len: f32) {
    let face = |b: &mut Baron, k: f32| { let want = to.x.atan2(to.z); b.boss.yaw = mw_sim::player::rad_lerp_range(b.boss.yaw, want, k, 0.4); };
    let walk = |b: &mut Baron, speed: f32, arena: &super::Arena| {
        let step = to.normalize_or_zero() * speed;
        let (moved, _) = mw_sim::collision::slide(&arena.0, b.boss.pos.to_array(), [step.x, step.z]);
        b.boss.pos.x += moved[0]; b.boss.pos.z += moved[1];
    };
    if b.cine.is_some() { if ended { b.boss.frame = 0.; } return; }
    // inspection: MW_MATHILDA_HURT=1 plays each of her (RinRin) reaction clips for 90 ticks from tick 40
    if std::env::var_os("MW_MATHILDA_HURT").is_some() && play.tick >= 40 {
        const R: [&str; 8] = ["em0440_1000.mot", "em0440_1001.mot", "em0440_1002.mot", "em0440_1003.mot", "pl0030_1091.mot", "em0440_1007.mot", "em0440_0003.mot", "em0440_0021.mot"];
        let (k, t) = (((play.tick - 40) / 90) as usize, ((play.tick - 40) % 90) as f32 / 90.);
        if k < R.len() {
            if t < 0.012 { if R[k].starts_with("pl0030_") { b.set(St::Damage, R[k]); } else { b.set_rin(St::Damage, R[k]); } eprintln!("[mathilda] tick {} reaction {}", play.tick, R[k]); }
            b.boss.frame = t * (b.len() - 1.);
            let off = Vec3::new(-1.8, 0., 3.2);
            b.boss.pos = Vec3::from_array(play.fight.player.pos) + off;
            b.boss.yaw = (-off.x).atan2(-off.z);
        }
        return;
    }
    // inspection: MW_MATHILDA_CLIPS=1 plays each of her own clips for 90 ticks from tick 40 (frame scaled to the clip), AI off
    if std::env::var_os("MW_MATHILDA_CLIPS").is_some() && play.tick >= 40 {
        let n = b.boss.bank.as_ref().map_or(0, |k| k.clips.len());
        // per clip: upside down anywhere (a flip) = the head (bone 4) less than 0.2 m above the hips (bone 0)
        if play.tick == 40 { if let Some(bank) = b.boss.bank.as_ref() {
            let at = |c: &mw_data::Clip, id: i16, f: f32| { let i = bank.bones.iter().position(|x| x.id == id).unwrap_or(0); let p = sample_retargeted(&bank.bones, c, f, Retarget::NATIVE); mw_sim::fight::bone_frames(&bank.bones, &p)[i].1[1] };
            // (with the root's own rotation: the event clips carry their pitch / roll there)
            let up = |c: &mw_data::Clip, f: f32| { let p = sample_retargeted(&bank.bones, c, f, Retarget::NATIVE); (Quat::from_array(p[bank.bones.len()].rotation) * Vec3::Y).y };
            // the bat hand's fast span (over half its peak speed), with her retarget
            for c in bank.clips.iter().filter(|c| c.name.starts_with("pl0030_")) {
                let h = bank.bones.iter().position(|x| x.id == 9).unwrap_or(9);
                let at = |f: f32| { let mut p = sample_retargeted(&bank.bones, c, f, mathilda_retarget(&c.name)); strip_root_xz(&bank.bones, &mut p); Vec3::from_array(mw_sim::fight::bone_frames(&bank.bones, &p)[h].1) };
                let sp: Vec<f32> = (1..c.frames).map(|f| at(f as f32).distance(at(f as f32 - 1.))).collect();
                let peak = sp.iter().cloned().fold(0., f32::max);
                let fast: Vec<usize> = sp.iter().enumerate().filter(|(_, s)| **s > peak * 0.5).map(|(i, _)| i + 1).collect();
                eprintln!("[mathilda-swing] {} frames {} peak {peak:.3} fast {:?}..{:?}", c.name, c.frames, fast.first(), fast.last());
            }
            for c in &bank.clips { eprintln!("[mathilda-clip] {} head over hips min {:.2} root up min {:.2}", c.name, (0..c.frames).map(|f| at(c, 4, f as f32) - at(c, 0, f as f32)).fold(9., f32::min),
                                             (0..c.frames).map(|f| up(c, f as f32)).fold(9., f32::min)); }
        } }
        let (k, t) = (((play.tick - 40) / 90) as usize, ((play.tick - 40) % 90) as f32 / 90.);
        if k < n {
            let name = b.boss.bank.as_ref().unwrap().clips[k].name.clone();
            if b.boss.clip != k || b.boss.borrowed.is_some() { b.boss.play(&name); eprintln!("[mathilda] tick {} clip {k} {name} yaw fix {:.2}", play.tick, b.boss.yaw_fix); }
            b.boss.frame = t * (b.boss.len(None) - 1.);
            // beside his line of sight, facing him
            let off = Vec3::new(-1.8, 0., 3.2);
            b.boss.pos = Vec3::from_array(play.fight.player.pos) + off;
            b.boss.yaw = (-off.x).atan2(-off.z);
        }
        return;
    }
    // the power struggle: he is down under her bat (her own beat-down pair em0610_0010 + pl0030_1055, camera c493, as
    // when the Baron knocks him out of the ring to her)
    // (no knock-out of the ring: the pair starts at PIN_FROM, him already down, placed so that his hips lie where he
    // stood, and it ends at FINISH_AT, her last blow, before the clip throws him away)
    if b.clash_req && player_alive {
        b.clash_req = false;
        start_pin(b, play, to);
        return;
    }
    let hp_k = b.hp as f32 / b.max_hp as f32;
    // the walking guard: RinRin's guard pose (em0440_0003, held at frame 12) over her upper body, eased in and out
    if b.guard_walk > 0. { b.guard_walk -= 1.; }
    // inspection: MW_BARON_TEST=guardwalk keeps her guard raised as she walks
    if std::env::var("MW_BARON_TEST").as_deref() == Ok("guardwalk") { b.guard_walk = 24.; }
    // the walking flinch: the clip's frames over her upper body, in at full weight, out over its last 8 frames
    if let Some((c, f)) = b.hurt_walk {
        let n = b.rin_bank.as_ref().map_or(1., |k| (k.clips[c].frames.max(2) - 1) as f32);
        b.hurt_walk = (f + 1. < n && matches!(b.st, St::Wait | St::Step)).then_some((c, f + 1.));
    }
    b.boss.upper = if b.guard_walk > 0. && matches!(b.st, St::Wait | St::Step) {
        b.rin_bank.as_ref().and_then(|k| k.clips.iter().position(|c| c.name == "em0440_0003.mot")).map(|c| (c, 12., ((48. - b.guard_walk).min(b.guard_walk) / 6.).min(1.)))
    } else if let Some((c, f)) = b.hurt_walk {
        let n = b.rin_bank.as_ref().map_or(1., |k| (k.clips[c].frames.max(2) - 1) as f32);
        Some((c, f, ((n - f) / 8.).clamp(0., 1.) * 0.85))
    } else { None };
    // he is lying from her knockdown: past PIN_WAIT she walks over; reaching him she starts the pin
    if play.fight.hold_down && play.fight.player.state() == 92 && player_alive && !matches!(b.st, St::Clash | St::Damage | St::Piyori | St::Finished | St::Combo) {
        face(b, 0.3);
        // (watching him lie there first)
        if play.fight.down_for < PIN_WAIT {
            if b.st != St::Wait { b.set(St::Wait, "em0610_0000.mot"); }
            if ended { b.boss.frame = 0.; }
        } else if dist > 1.1 {
            if b.st != St::Step { b.st = St::Step; b.boss.play("em0610_0001.mot"); }
            if ended { b.boss.frame = 0.; }
            walk(b, 0.075, arena);
        } else { play.fight.hold_down = false; play.fight.down_for = 0; b.clash_req = true; }
        return;
    }
    match b.st {
        St::Wait | St::Step => {
            face(b, 0.15);
            b.atk_timer -= 1.;
            if dist > 2.4 {
                if b.st != St::Step { b.st = St::Step; b.boss.play("em0610_0001.mot"); }
                if ended { b.boss.frame = 0.; }
                // her walk's own pace (em0610_0001: 2.02 m over 33 frames)
                walk(b, 0.061 * if dist > 6. { 1.4 } else { 1. }, arena);
            } else {
                if b.st != St::Wait { b.st = St::Wait; b.boss.play("em0610_0000.mot"); }
                if ended { b.boss.frame = 0.; }
            }
            if b.atk_timer <= 0. && dist < 3.0 && player_alive {
                // the interval shortens as she loses HP (RinRin / Elise decideAttackInterval style)
                b.atk_timer = 18. + b.roll() * 40. * hp_k;
                // close: a light string; a step back: a heavy swing (more of them as she weakens)
                let heavy = dist > 2.0 || b.roll() < 0.25 + 0.35 * (1. - hp_k);
                let k = if heavy { 2 + (b.roll() * 2.) as usize % 2 } else { 0 };
                b.last_atk = k as u8; b.sub = 0; b.st = St::Combo; b.landed = 0; b.timer = 0.;
                b.boss.play(M_MOVES[k].clip);
            } else if dist > 3.2 && b.roll() < 0.004 {
                b.st = St::Threat; b.boss.play(M_THREATS[0]); b.voices.push(0x11B);
            }
        }
        St::Combo => {
            let m = &M_MOVES[b.last_atk as usize % M_MOVES.len()];
            if b.boss.frame < m.hit.0 {
                face(b, 0.3);
                // close to swinging distance (SWING_AT) by the hit frame: her bat's head reaches about 1.4 m ahead
                if dist > SWING_AT { walk(b, ((dist - SWING_AT) / (m.hit.0 - b.boss.frame).max(1.)).min(0.12), arena); }
            }
            // the bat lands only where it is: through the window (a few frames either side) her bat's head has to reach
            // his body (a capsule 0.2-1.9 m up, BAT_REACH around his axis)
            let tip = bat_head(b);
            let pl = Vec3::from_array(play.fight.player.pos);
            let off = Vec2::new(tip.x - pl.x, tip.z - pl.z).length();
            let touching = off < BAT_REACH && (pl.y + 0.2..pl.y + 1.9).contains(&tip.y);
            if std::env::var_os("MW_BAT_LOG").is_some() { eprintln!("[bat] {} f{:.0} dist {:.2} tip off {:.2} y {:.2}", M_MOVES[b.last_atk as usize % M_MOVES.len()].clip, b.boss.frame, dist, off, tip.y - pl.y); }
            if (m.hit.0 - 2. ..m.hit.1 + 8.).contains(&b.boss.frame) && b.landed & 1 == 0 && touching
                && { let pp = &mut *play; pp.fight.enemy_hits_player(&pp.fd, m.atk, m.power, b.boss.pos.to_array()) } {
                b.landed |= 1;
                // the bat lands: her bat-impact SE (em0610_0010_10's 4213:403) and the impact spark where it struck
                let at = tip;
                b.sfx.push((4213, 403, at)); fx.impact(at, b.boss.yaw, if m.heavy { 1.4 } else { 1. }, [1., 0.12, 0.08]);
                if m.heavy { play.fight.hitstop = play.fight.hitstop.max(6); }
                // her knockdown swing (the big smash) floors him; he has to get up himself (any press), and if he lies
                // there she walks over and pins him under the bat (the power struggle) [added]
                if b.last_atk as usize == M_KNOCKDOWN && player_alive { play.fight.hold_down = true; }
            }
            if ended {
                // her strings: swing -> follow-through -> wound-up swing -> (sometimes) the big smash, each link while he
                // is in reach; the knockdown smash ends a string
                let next = match b.last_atk { 0 => Some(1), 1 => Some(2), 2 => (b.roll() < 0.4).then_some(M_KNOCKDOWN), _ => None };
                if let Some(k) = next.filter(|_| dist < 2.8 && b.roll() < 0.75 && player_alive && !play.fight.hold_down) {
                    b.last_atk = k as u8; b.landed = 0; b.boss.play(M_MOVES[k].clip); return;
                }
                b.set(St::Wait, "em0610_0000.mot");
            }
        }
        // the bat beat-down: eight blows at the 4213:403 cues (73 .. 157), 3 each
        St::Hold => {
            for (k, f) in [73., 86., 98., 108., 121., 132., 145., 157.].iter().enumerate() {
                if b.boss.frame >= *f && b.landed & (1 << k) == 0 {
                    b.landed |= 1 << k;
                    let pp = &mut *play; pp.fight.player.hp = (pp.fight.player.hp - 3).max(1);
                    fx.burst("core", 1005, Vec3::from_array(play.fight.player.pos) + Vec3::Y * 1.2, 0., 1.);
                }
            }
            if ended { end_pair(b, play); b.set(St::Wait, "em0610_0000.mot"); }
        }
        // the power struggle (RinRin's pair on her rig, its c4aa..c4ad cameras): the shake meter, then win or lose
        St::Clash => step_mathilda_clash(b, play, fx, ended),
        St::Threat => { face(b, 0.1); if ended || dist < 2.2 { b.set(St::Wait, "em0610_0000.mot"); } }
        St::Avoid => { if ended { b.set(St::Wait, "em0610_0000.mot"); } }
        St::Damage => {
            // the knockdown (1003) gets up with 1004
            if ended {
                let name = b.boss.shown_name(b.rin_bank.as_ref()).to_string();
                if name == "em0440_1003.mot" { b.set(St::Damage, "pl0030_1091.mot"); } else { b.set(St::Wait, "em0610_0000.mot"); }
            }
        }
        St::Guard => { face(b, 0.2); if ended { b.set(St::Wait, "em0610_0000.mot"); } }
        St::Piyori => { if ended { b.boss.frame = 0.; } }
        // he is down for good: she laughs over him (RinRin's laughing taunt, her PL_DEAD clip), again and again with a beat
        // between; when he continues she is back at it
        St::PlDead => {
            if player_alive { b.atk_timer = 40.; b.set(St::Wait, "em0610_0000.mot"); }
            else if ended {
                if b.boss.borrowed.is_some() { b.boss.play("em0610_0000.mot"); b.timer = 45. + b.roll() * 45.; }
                else { b.timer -= len; if b.timer <= 0. { b.set_rin(St::PlDead, "em0440_0002.mot"); } else { b.boss.frame = 0.; } }
            }
        }
        St::Finished => { if ended { b.boss.frame = len - 1.; } }
        // (the editor's clip: step() plays it and returns to the wait)
        St::Editor => {}
        _ => { b.set(St::Wait, "em0610_0000.mot"); }
    }
}

/// Her power struggle [added, on her own beat-down]: the pair em0610_0010 / pl0030_1055 plays, the native two-arm
/// prompt asks for the shake (held X fills the meter, it drains otherwise). Filled before her last blow, he throws her
/// off: the pair breaks, she reels (RinRin's 1002, then the knockdown 1003) and takes 200. Otherwise every blow lands
/// (the eight 4213:403 cues, 3 each) and the pair plays out. Meter rates as the Baron's finale meter [inferred].
/// The pin starts with him already down (the flight over the ropes is frames 0-59); it ends at FINISH_AT, before the clip's
/// closing throw (186-216, 5.9 m).
const PIN_FROM: f32 = 60.;

/// The blow she is placed for (the third of the eight).
const PIN_AIM: f32 = 98.;
/// The last-life execution (Baron::start_execution): the walk up, then the struggle played to its loss.
fn step_execution(b: &mut Baron, play: &mut super::Play, fx: &mut super::fx::Fx, arena: &super::Arena, dist: f32, to: Vec3, ended: bool) {
    let want = to.x.atan2(to.z);
    if b.exec == 1 {
        b.boss.yaw = mw_sim::player::rad_lerp_range(b.boss.yaw, want, 0.25, 0.4);
        let reach = if b.mathilda_boss { 1.1 } else { 1.3 };
        if dist > reach {
            if b.mathilda_boss {
                if b.st != St::Step { b.st = St::Step; b.boss.play("em0610_0001.mot"); }
                let d = to.normalize_or_zero() * 0.075;
                let (m, _) = mw_sim::collision::slide(&arena.0, b.boss.pos.to_array(), [d.x, d.z]);
                b.boss.pos.x += m[0]; b.boss.pos.z += m[1];
            } else if b.st != St::Step { b.set(St::Step, "em0623_0011.mot"); }
            if ended { b.boss.frame = 0.; }
            return;
        }
        b.exec = 2;
        if b.mathilda_boss { start_pin(b, play, to); b.prompt = None; }
        else {
            b.player_from = Vec3::from_array(play.fight.player.pos);
            b.boss.yaw = want; b.pair_at = (b.boss.pos, want);
            b.st = St::Clash; b.sub = 60; b.qte = 0; b.prompt = None; b.voices.push(0x116);
            pair(b, play, "pl0030_0480.mot", "em0623_0301.mot");
        }
        return;
    }
    if b.mathilda_boss { step_mathilda_clash(b, play, fx, ended); return; }
    match b.sub {
        60 => if ended { b.sub = 61; b.landed = 0; b.voices.push(0x118); pair(b, play, "pl0030_0486.mot", "em0623_0307.mot"); },
        // his killing blow: 0307's big hit (its frame-80 effect cue 8127) takes the head off (play_execution.rs)
        61 if !ended => if b.boss.frame >= 80. && b.landed == 0 { b.landed = 1; play.fight.events.push(mw_sim::fight::FightEvent::ExecutionCut); },
        _ => if ended {
            end_pair(b, play);
            let pp = &mut *play; pp.fight.fall_dead(&pp.fd, "pl0030_1016.mot");
            b.exec = 0; b.voices.push(0x11A); b.set(St::PlDead, "em0623_0002.mot");
        },
    }
}

/// The pin pair starts (her beat-down from PIN_FROM, him lying under it).
/// The bat beat-down's placement from her side: Mathilda (`her`, her em0610_0010) stands at `at` facing `yaw`; returns
/// where his pl0030_1055 is anchored (his hips on the floor at PIN_FROM, under her bat at her third blow, PIN_AIM) and
/// the point her bat lands. `start_pin` solves the same placement for her from him.
fn bat_anchor(her: &Actor, at: Vec3, yaw: f32, play: &super::Play) -> (Vec3, Vec3) {
    let fd = &play.fd;
    let clip = &fd.jack.clips[fd.jack_clip("pl0030_1055.mot")];
    let hips_at = |f: f32| { let mut p = sample_retargeted(&fd.jack.bones, clip, f, Retarget::NATIVE); strip_root_xz(&fd.jack.bones, &mut p); Vec3::from_array(mw_sim::fight::bone_frames(&fd.jack.bones, &p)[0].1) };
    let q = Quat::from_rotation_y(yaw);
    let tip = her.bank.as_ref().and_then(|bank| {
        let c = &bank.clips[*her.clips.get("em0610_0010.mot")?];
        let hand = bank.bones.iter().position(|x| x.id == 9)?;
        let p = sample_retargeted(&bank.bones, c, PIN_AIM, Retarget::NATIVE);
        let h = Vec3::from_array(mw_sim::fight::bone_frames(&bank.bones, &p)[hand].1);
        let flat = Vec3::new(h.x, 0., h.z);
        Some(flat + flat.normalize_or_zero() * 0.7)
    }).unwrap_or(Vec3::Z * 1.2);
    let hit = at + q * tip;
    let h = hips_at(PIN_AIM);
    let mut pin = hit - q * Vec3::new(h.x, 0., h.z);
    pin.y = at.y + 0.15 - hips_at(PIN_FROM).y;
    (pin, Vec3::new(hit.x, at.y, hit.z))
}

fn start_pin(b: &mut Baron, play: &mut super::Play, to: Vec3) {
    let pl = Vec3::from_array(play.fight.player.pos);
    b.player_from = pl;
    let yaw = to.x.atan2(to.z); b.boss.yaw = yaw;
    let clip = &play.fd.jack.clips[play.fd.jack_clip("pl0030_1055.mot")];
    let mut poses = sample_retargeted(&play.fd.jack.bones, clip, PIN_FROM, Retarget::NATIVE);
    strip_root_xz(&play.fd.jack.bones, &mut poses);
    let hips = Vec3::from_array(mw_sim::fight::bone_frames(&play.fd.jack.bones, &poses)[0].1);
    // the two halves have their own origins (his is the launcher's, 15.5 m off; hers is her spot): she stands 0.9 m
    // from him facing him (the c493 camera's anchor, as at the ropes), his clip is shifted so that his hips lie on the
    // floor where he stood (they are 1.29 m below the clip's start: the drop off the ring)
    let q = Quat::from_rotation_y(yaw);
    let lift = 0.15 - hips.y;
    b.pin_at = pl - q * Vec3::new(hips.x, 0., hips.z) + Vec3::Y * lift;
    // she stands so that her bat lands on him: at her third blow (PIN_AIM) her bat hand (bone 9, with her clip's own
    // root travel), carried 0.7 m on along the swing, meets his hips at that frame
    let his = { let mut p = sample_retargeted(&play.fd.jack.bones, clip, PIN_AIM, Retarget::NATIVE); strip_root_xz(&play.fd.jack.bones, &mut p);
                let h = Vec3::from_array(mw_sim::fight::bone_frames(&play.fd.jack.bones, &p)[0].1); b.pin_at + q * Vec3::new(h.x, 0., h.z) };
    let tip = b.boss.bank.as_ref().and_then(|bank| {
        let c = &bank.clips[*b.boss.clips.get("em0610_0010.mot")?];
        let hand = bank.bones.iter().position(|x| x.id == 9)?;
        let p = sample_retargeted(&bank.bones, c, PIN_AIM, Retarget::NATIVE);
        let h = Vec3::from_array(mw_sim::fight::bone_frames(&bank.bones, &p)[hand].1);
        let flat = Vec3::new(h.x, 0., h.z);
        Some(flat + flat.normalize_or_zero() * 0.7)
    }).unwrap_or(Vec3::Z * 1.2);
    let at = his - q * tip;
    b.boss.pos = Vec3::new(at.x, b.boss.pos.y, at.z);
    b.pair_at = (b.boss.pos, yaw);
    if std::env::var_os("MW_BARON_LOG").is_some() { eprintln!("[mathilda] pin: player {pl:.2} hips {hips:.2} his anchor {:.2} her {:.2} yaw {yaw:.2}", b.pin_at, b.boss.pos); }
    b.st = St::Clash; b.sub = 0; b.landed = 0; b.meter = 0.25; b.boss.play("em0610_0010.mot"); b.boss.frame = PIN_FROM;
    let pp = &mut *play; pp.fight.external(&pp.fd, "pl0030_1055.mot", false, 4.); pp.fight.player.motion.frame = PIN_FROM;
    play.fight.paired = true; play.fight.player.pos = b.pin_at.to_array(); play.fight.player.loco.facing = yaw;
    b.voices.push(0x11D);
}

/// The pin is over: he stays where he lies (not the clip's final travel, end_pair's rule).
fn end_pin(b: &Baron, play: &mut super::Play) {
    let clip = &play.fd.jack.clips[play.fight.player.motion.clip];
    let mut poses = sample_retargeted(&play.fd.jack.bones, clip, play.fight.player.motion.frame, Retarget::NATIVE);
    strip_root_xz(&play.fd.jack.bones, &mut poses);
    let hips = Vec3::from_array(mw_sim::fight::bone_frames(&play.fd.jack.bones, &poses)[0].1);
    let at = b.pin_at + Quat::from_rotation_y(b.pair_at.1) * Vec3::new(hips.x, 0., hips.z);
    play.fight.player.pos = [at.x, b.player_from.y, at.z];
    play.fight.paired = false;
    play.fight.end_external();
}
fn step_mathilda_clash(b: &mut Baron, play: &mut super::Play, fx: &mut super::fx::Fx, _ended: bool) {
    let shake = b.exec == 0 && b.auto != Some(false) && (play.latched_shake() || std::env::var_os("MW_BARON_AUTO").is_some() || b.auto == Some(true));
    const BLOWS: [f32; 8] = [73., 86., 98., 108., 121., 132., 145., 157.];
    if b.sub == 0 {
        b.meter = (b.meter + if shake { 0.009 } else { -0.003 }).clamp(0., 1.);
        b.prompt = (b.exec == 0).then(|| format!("TAP A/D!  {}%", (b.meter * 100.) as i32));
        for (k, f) in BLOWS.iter().enumerate() {
            if b.boss.frame >= *f && b.landed & (1 << k) == 0 {
                b.landed |= 1 << k;
                if b.exec == 0 { let pp = &mut *play; pp.fight.player.hp = (pp.fight.player.hp - 1).max(1); }
                // where he actually lies (his shifted clip's hips), the bat's blood and the impact
                let at = b.player_from + Vec3::Y * 0.3;
                fx.gore(at, b.boss.yaw, 12, true, false); b.sfx.push((4213, 403, at));
            }
        }
        if b.meter >= 1. && b.boss.frame < BLOWS[7] {
            // thrown off: he gets up out of the pair, she reels back
            b.prompt = None; b.sub = 1;
            end_pin(b, play);
            b.hp = (b.hp - 200).max(1); b.clashes += 1; b.voices.push(0x114);
            play.fight.hitstop = play.fight.hitstop.max(8);
            fx.impact(b.body(), b.boss.yaw, 1.6, [1., 0.12, 0.08]); fx.burst("core", 1005, b.body(), b.boss.yaw, 1.4);
            b.sfx.push((4213, 404, b.body()));
            b.defend_cd = 60.; b.set_rin(St::Damage, "em0440_1002.mot");
        } else if b.boss.frame >= FINISH_AT {
            // he did not get free: her beat-down's last blow (frame 184, the heavy 4213:404) is the finishing blow, straight
            // down onto him; he lies where the pin left him and bounces off the floor
            b.prompt = None;
            end_pin(b, play);
            b.player_from = Vec3::from_array(play.fight.player.pos);
            b.sub = 2; b.landed = 1; b.timer = 0.;
            lie(play);
            let at = b.player_from + Vec3::Y * 0.3;
            let pp = &mut *play; pp.fight.player.hp = (pp.fight.player.hp - FINISH_DAMAGE).max(0);
            // (the last-life execution: the blow comes down on his head and takes it off)
            if b.exec > 0 { play.fight.events.push(mw_sim::fight::FightEvent::ExecutionCut); }
            // a long hit pause: the slow beat of the blow
            play.fight.hitstop = play.fight.hitstop.max(18);
            fx.gore(at, b.boss.yaw, 40, true, false); fx.burst("core", 532, at, b.boss.yaw, 1.);
            b.voices.push(0x114);
        }
    } else if b.sub == 2 {
        // (her beat-down plays on past the blow; he is held where he lies)
        if b.boss.frame >= b.len() - 1. { b.boss.frame = b.len() - 1.; }
        if b.landed == 1 {
            b.timer += 1.;
            let k = (b.timer / BOUNCE_FRAMES).min(1.);
            lie(play);
            play.fight.player.pos[0] = b.player_from.x; play.fight.player.pos[2] = b.player_from.z;
            play.fight.player.pos[1] = b.player_from.y + FINISH_BOUNCE * (std::f32::consts::PI * k).sin() * (1. - 0.35 * k);
            if k >= 1. && b.exec > 0 {
                // the execution: he stays down, dead, and she laughs over him
                b.landed = 2; play.fight.player.pos[1] = b.player_from.y;
                play.fight.hold_down = false; play.fight.down_for = 0; play.fight.lie_dead();
                b.exec = 0; b.voices.push(0x11A); b.set_rin(St::PlDead, "em0440_0002.mot");
            } else if k >= 1. {
                b.landed = 2; play.fight.player.pos[1] = b.player_from.y;
                // he gets up by himself now (no second pin)
                play.fight.hold_down = false; play.fight.down_for = 0;
                b.defend_cd = 30.; b.set(St::Wait, "em0610_0000.mot");
            }
        }
    }
}

/// The pin's finishing blow: the beat-down's last blow (its 4213:404 cue), damage, bounce height (m) and its length (frames).
const FINISH_AT: f32 = 184.;
const FINISH_DAMAGE: i16 = 10;
const FINISH_BOUNCE: f32 = 0.55;
const BOUNCE_FRAMES: f32 = 16.;
/// He lies on his back (the knockdown's last frame, state 92) while she finishes him.
fn lie(play: &mut super::Play) {
    let clip = play.fd.jack_clip("pl0030_1011.mot");
    let n = (play.fd.jack.clips[clip].frames.max(2) - 1) as f32;
    let p = &mut play.fight.player;
    if p.state() != 92 { p.loco.enter_combat(92); p.motion.play(clip, 0., false, true); }
    p.motion.frame = n;
    play.fight.hold_down = true; play.fight.down_for = 0;
}

/// Mathilda / the Baron at the ropes and the three oirans.
fn step_ringside(b: &mut Baron, play: &mut super::Play, fx: &mut super::fx::Fx, assets: &AssetServer, commands: &mut Commands) {
    let m = &mut b.mathilda;
    m.frame += 1.;
    let mlen = m.len(None);
    if b.mathilda_boss {
        // the Baron at her ropes: wait, now and then a provocation
        let toward = (Vec3::ZERO - m.pos).normalize_or(Vec3::Z); m.yaw = toward.x.atan2(toward.z);
        if m.frame >= mlen - 1. {
            let roll = { b.rng = b.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (b.rng >> 8) as f32 / (1 << 24) as f32 };
            b.mathilda.play(if roll < 0.25 { ["em0623_0001.mot", "em0623_0003.mot"][(roll * 8.) as usize % 2] } else { "em0623_0000.mot" });
        }
    } else {
        match b.m_side {
            Side::Wait => {
                m.yaw = mw_sim::player::rad_lerp_range(m.yaw, (-m.pos.x).atan2(-m.pos.z), 0.1, 0.4);
                if m.frame >= mlen - 1. { m.frame = 0.; }
                b.m_timer -= 1.;
                if b.m_timer <= 0. {
                    let r = b.roll();
                    b.m_target = Vec3::new(MATHILDA_AT.x, MATHILDA_AT.y, -6. + 12. * r); b.m_side = Side::Walk; b.mathilda.play("em0610_0001.mot");
                }
            }
            Side::Walk => {
                let d = b.m_target - m.pos;
                if Vec2::new(d.x, d.z).length() < 0.8 { b.m_side = Side::Wait; b.m_timer = 30. * (3. + 2. * b.roll()); b.mathilda.play("em0610_0000.mot"); }
                else { m.yaw = d.x.atan2(d.z); m.pos += d.normalize_or_zero() * 0.045; if m.frame >= mlen - 1. { m.frame = 0.; } }
            }
            Side::BatCombo => {
                for (k, f) in [73., 86., 98., 108., 121., 132., 145., 157.].iter().enumerate() {
                    if m.frame >= *f && b.landed & (1 << (8 + k)) == 0 {
                        b.landed |= 1 << (8 + k);
                        play.fight.player.hp = (play.fight.player.hp - 3).max(1);
                        let at = b.bat_hit + Vec3::Y * 0.3;
                        fx.gore(at, m.yaw, 8, k >= 6, false);
                    }
                }
                if m.frame >= 80. && b.landed & (1 << 20) == 0 { b.landed |= 1 << 20; b.voices.push(0x11D); }
                if m.frame >= mlen - 1. {
                    // (he stays where he lies, as after her pin)
                    end_pin(b, play);
                    let m = &mut b.mathilda;
                    m.pos.x = m.pos.x.clamp(-8.2, 8.2); m.pos.z = m.pos.z.clamp(-8.2, 8.2); m.pos.y = 0.05;
                    b.m_side = Side::Wait; b.m_timer = 60.; b.mathilda.play("em0610_0000.mot");
                    b.landed = 0;
                }
            }
            _ => {}
        }
    }
    // the oirans: wait, cheer1 / cheer2 every 30 x U[3, 5] frames, the toss
    for i in 0..b.oirans.len() {
        let roll = b.roll(); let roll2 = b.roll();
        let tossing = b.toss_by == Some(i);
        let toss_kind = if b.toss_step == 0 { 0 } else { 1 };
        let (o, side, timer) = &mut b.oirans[i];
        o.frame += 1.;
        let olen = o.len(None);
        if tossing && *side != Side::Toss {
            *side = Side::Toss; o.play("em0960_0002.mot"); o.yaw = (-o.pos.x).atan2(-o.pos.z);
            b.cam_run = Some(("em0960_c000.mot".into(), 0., o.pos, o.yaw));
        }
        match *side {
            Side::Toss => {
                if (o.frame - 16.).abs() < 0.5 { b.voices.push(0x11C); }
                if (o.frame - 67.).abs() < 0.5 {
                    // the throw: along her yaw at U[0.5, 0.8] (scaled to land inside the ring)
                    let dir = Vec3::new(o.yaw.sin(), 0., o.yaw.cos());
                    let at = o.pos + Vec3::Y * 1.4 + dir * 0.3;
                    let files = [["native/item-0501-source.glb", "native/item-0501-color.glb"], ["native/item-0502-source.glb", "native/item-0502-color.glb"]][toss_kind];
                    let roots = files.map(|f| commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(f))), Transform::from_translation(at), Visibility::Hidden)).id());
                    b.items.push(Item { kind: toss_kind, roots, pos: at, vel: dir * (0.16 + 0.08 * roll) + Vec3::Y * 0.14, spin: 0., live: true });
                }
                if o.frame >= olen - 1. {
                    *side = Side::Wait; *timer = 90.; o.play("em0960_0000.mot");
                    b.toss_by = None; b.toss_step += 1; b.toss_cd = 900.; b.cam_run = None;
                }
            }
            Side::Cheer => { if o.frame >= olen - 1. { *side = Side::Wait; *timer = 30. * (3. + 2. * roll); o.play("em0960_0000.mot"); } }
            _ => {
                if o.frame >= olen - 1. { o.frame = 0.; }
                *timer -= 1.;
                if *timer <= 0. { *side = Side::Cheer; o.play(if roll2 < 0.5 { "em0960_0003.mot" } else { "em0960_0004.mot" }); }
            }
        }
    }
}

/// Thrown items: fly, land, spin; the player picks one up by walking onto it.
fn step_items(b: &mut Baron, play: &mut super::Play, fx: &mut super::fx::Fx) {
    let pl = Vec3::from_array(play.fight.player.pos);
    for it in b.items.iter_mut().filter(|i| i.live) {
        it.spin += 0.05;
        if it.vel != Vec3::ZERO {
            it.vel.y -= 0.012; it.pos += it.vel;
            if it.pos.y <= 0.25 && it.vel.y < 0. { it.pos.y = 0.25; it.vel = Vec3::ZERO; }
        }
        if Vec2::new(it.pos.x - pl.x, it.pos.z - pl.z).length() < 0.9 && it.vel == Vec3::ZERO {
            it.live = false;
            let heal = if it.kind == 0 { 40 } else { 25 };
            play.fight.player.hp = (play.fight.player.hp + heal).min(100);
            fx.burst("core", 12, it.pos + Vec3::Y * 0.5, 0., 1.2);
        }
    }
}

fn bind(mut bn: ResMut<Baron>, children: Query<&Children>, names: Query<&Name>, mut commands: Commands) {
    if !bn.on || !bn.spawned { return; }
    let b = &mut *bn;
    let find = |root: Entity, bones: &[mw_data::Bone]| -> Option<Vec<Entity>> {
        let found: HashMap<String, Entity> = children.iter_descendants(root).filter_map(|d| names.get(d).ok().map(|n| (n.as_str().to_string(), d))).collect();
        bones.iter().map(|b| b.id).chain(std::iter::once(-1)).map(|id| found.get(&format!("role_0_bone{id}")).copied()).collect()
    };
    // (which springs each wears, play_springs.rs: the Baron's chest chain, Mathilda's and the oirans' breasts and hair)
    let (boss_id, side_id) = if b.mathilda_boss { ("em0610", "em0623") } else { ("em0623", "em0610") };
    let mut actors: Vec<(&mut Actor, Option<&'static str>)> = vec![(&mut b.boss, Some(boss_id)), (&mut b.mathilda, Some(side_id)), (&mut b.heli, None)];
    actors.extend(b.oirans.iter_mut().map(|o| (&mut o.0, Some("em0960"))));
    for (a, actor) in actors {
        let Some(bank) = &a.bank else { continue; };
        for k in 0..2 { if a.joints[k].is_none() { a.joints[k] = find(a.roots[k], &bank.bones);
            if let (Some(j), Some(actor)) = (&a.joints[k], actor) { commands.entity(a.roots[k]).insert(super::springs::SpringRequest { bones: bank.bones.clone(), joints: j.clone(), actor }); }
        } }
    }
}

/// Draw one actor: its clip (or a borrowed one), root travel kept while paired.
fn draw(a: &Actor, extra: Option<&MotionBank>, keep_root: bool, shown: bool, colored: bool, transforms: &mut Query<&mut Transform>, vis: &mut Query<&mut Visibility>) -> Option<Vec<([f32; 4], [f32; 3], [f32; 3])>> {
    let bank = a.bank.as_ref()?;
    let clip = match a.borrowed { Some(i) => &extra?.clips[i], None => &bank.clips[a.clip] };
    let retarget = if bank.bones.len() == 27 { mathilda_retarget(&clip.name) } else { Retarget::NATIVE };
    let mut poses = sample_retargeted(&bank.bones, clip, a.frame.min((clip.frames.max(2) - 1) as f32), retarget);
    // (her ev50 cutscene clips scale the whole rig by 1.2 on the root, the cutscene's framing: kept at her own size)
    if bank.bones.len() == 27 { let n = bank.bones.len(); poses[n].scale = [1.; 3]; }
    if !keep_root { strip_root_xz(&bank.bones, &mut poses); }
    if let (Some((c, f, w)), Some(extra)) = (a.upper, extra) {
        let top = sample_retargeted(&bank.bones, &extra.clips[c], f.min((extra.clips[c].frames.max(2) - 1) as f32), mathilda_retarget(&extra.clips[c].name));
        for (i, b) in bank.bones.iter().enumerate() {
            if (1..=14).contains(&b.id) {
                let r = Quat::from_array(poses[i].rotation).slerp(Quat::from_array(top[i].rotation), w.clamp(0., 1.));
                poses[i].rotation = r.normalize().to_array();
            }
        }
    }
    // a cutscene clip's root rides at its event's height, and its poses hold the pelvis higher than the gameplay idle:
    // her lowest foot (bones 19 / 23) is put where the idle's stands (her own em0610_0000, frame 0)
    if a.yaw_fix != 0. && a.borrowed.is_none() {
        let n = bank.bones.len(); poses[n].translation[1] = 0.;
        let feet = |p: &[mw_sim::character::LocalPose]| { let f = mw_sim::fight::bone_frames(&bank.bones, p);
            bank.bones.iter().enumerate().filter(|(_, b)| b.id == 19 || b.id == 23).map(|(i, _)| f[i].1[1]).fold(f32::MAX, f32::min) };
        let idle = feet(&sample_retargeted(&bank.bones, &bank.clips[0], 0., Retarget::NATIVE));
        let now = feet(&poses);
        if now.is_finite() && idle.is_finite() { poses[n].translation[1] -= now - idle; }
    }
    for k in 0..2 {
        let on = shown && (k == 1) == colored;
        if let Ok(mut v) = vis.get_mut(a.roots[k]) { *v = if on { Visibility::Visible } else { Visibility::Hidden }; }
        if let Ok(mut t) = transforms.get_mut(a.roots[k]) { *t = Transform::from_translation(a.pos).with_rotation(Quat::from_rotation_y(a.yaw + a.yaw_fix)); }
        let Some(js) = &a.joints[k] else { continue; };
        for (e, p) in js.iter().zip(&poses) {
            if let Ok(mut t) = transforms.get_mut(*e) { t.translation = Vec3::from_array(p.translation); t.rotation = Quat::from_array(p.rotation); t.scale = Vec3::from_array(p.scale); }
        }
    }
    Some(mw_sim::fight::bone_frames(&bank.bones, &poses))
}

pub fn pose(bn: Res<Baron>, play: Res<super::Play>, intro: Res<super::intro::Intro>, mut transforms: Query<&mut Transform>, mut vis: Query<&mut Visibility>) {
    if !bn.on || !bn.spawned { return; }
    // the event's own actors play ev50 / ev51
    let staged = !intro.active();
    let c = play.colored;
    let boss_frames = draw(&bn.boss, bn.rin_bank.as_ref(), bn.paired(), staged, c, &mut transforms, &mut vis);
    // her jaw (bone 24: it skins the chin) works with the intro's lines [added lip movement; no voice yet]
    let open = bn.mouth();
    if open > 0. { if let Some(j) = bn.boss.bank.as_ref().and_then(|bk| bk.bones.iter().position(|b| b.id == 24)) {
        for k in 0..2 { if let Some(e) = bn.boss.joints[k].as_ref().and_then(|js| js.get(j)) { if let Ok(mut t) = transforms.get_mut(*e) { t.rotation = t.rotation * Quat::from_rotation_x(0.32 * open); } } }
    } }
    // (the Mathilda variant has no Baron at the ropes)
    let m_frames = draw(&bn.mathilda, None, bn.m_side == Side::BatCombo, staged && !bn.mathilda_boss, c, &mut transforms, &mut vis);
    for o in &bn.oirans { draw(&o.0, None, false, staged, c, &mut transforms, &mut vis); }
    draw(&bn.heli, None, true, staged && bn.heli_on, c, &mut transforms, &mut vis);
    // the hat and sunglasses on the Baron's head (bone 4), or tumbling free
    let (baron, frames) = if bn.mathilda_boss { (&bn.mathilda, &m_frames) } else { (&bn.boss, &boss_frames) };
    let head = baron.bank.as_ref().and_then(|bk| bk.bones.iter().position(|b| b.id == 4));
    let base = Transform::from_translation(baron.pos).with_rotation(Quat::from_rotation_y(baron.yaw));
    for prop in [&bn.hat, &bn.shades] {
        let t = match (&prop.free, head, frames) {
            (Some((p, _, spin)), _, _) => Some(Transform::from_translation(*p).with_rotation(Quat::from_rotation_x(*spin))),
            (None, Some(h), Some(f)) => { let (r, p, s) = f[h]; Some(base * Transform { translation: Vec3::from_array(p), rotation: Quat::from_array(r), scale: Vec3::from_array(s) }) }
            _ => None,
        };
        for k in 0..2 {
            let on = staged && !bn.mathilda_boss && t.is_some() && (k == 1) == c;
            if let Ok(mut v) = vis.get_mut(prop.roots[k]) { *v = if on { Visibility::Visible } else { Visibility::Hidden }; }
            if let (Some(t), Ok(mut tr)) = (t, transforms.get_mut(prop.roots[k])) { *tr = t; }
        }
    }
    // her bat on her right hand
    let (her, her_frames) = if bn.mathilda_boss { (&bn.boss, &boss_frames) } else { (&bn.mathilda, &m_frames) };
    let hand = her.bank.as_ref().and_then(|bk| bk.bones.iter().position(|b| b.id == 9));
    let t = match (hand, her_frames) {
        (Some(h), Some(f)) => { let (r, p, s) = f[h]; Some(Transform::from_translation(her.pos).with_rotation(Quat::from_rotation_y(her.yaw + her.yaw_fix)) * Transform { translation: Vec3::from_array(p), rotation: Quat::from_array(r), scale: Vec3::from_array(s) } * Transform::from_xyz(-0.12, -0.005, -0.07)) }
        _ => None,
    };
    for k in 0..2 {
        if let Ok(mut v) = vis.get_mut(bn.bat.roots[k]) { *v = if staged && t.is_some() && (k == 1) == c { Visibility::Visible } else { Visibility::Hidden }; }
        if let (Some(t), Ok(mut tr)) = (t, transforms.get_mut(bn.bat.roots[k])) { *tr = t; }
    }
    for it in &bn.items {
        for k in 0..2 {
            if let Ok(mut v) = vis.get_mut(it.roots[k]) { *v = if it.live && (k == 1) == c { Visibility::Visible } else { Visibility::Hidden }; }
            if let Ok(mut t) = transforms.get_mut(it.roots[k]) { *t = Transform::from_translation(it.pos).with_rotation(Quat::from_rotation_y(it.spin)).with_scale(Vec3::splat(1.4)); }
        }
    }
}

/// The free hat / sunglasses fall to the canvas (FixedUpdate is not needed for a prop in flight: Update with dt).
pub fn props(mut bn: ResMut<Baron>, time: Res<Time>) {
    if !bn.on { return; }
    let k = (time.delta_secs() * 30.).min(3.);
    let b = &mut *bn;
    for prop in [&mut b.hat, &mut b.shades] {
        if let Some((p, v, spin)) = prop.free.as_mut() {
            if p.y > 0.08 { v.y -= 0.012 * k; *p += *v * k; *spin += 0.2 * k; } else { p.y = 0.08; *v = Vec3::ZERO; }
        }
    }
}

#[derive(Component)]
struct PromptText;
/// The clash prompts [added: the native cockpit art is not drawn].
fn prompt(bn: Res<Baron>, mut q: Query<(Entity, &mut Text), With<PromptText>>, mut commands: Commands) {
    if !bn.on { return; }
    // (the native swing navigation draws the clash prompts: play_prompts.rs; MW_TEXT_PROMPTS=1 adds this text)
    let shown = if std::env::var_os("MW_TEXT_PROMPTS").is_some() { &bn.prompt } else { &None };
    match (shown, q.single_mut()) {
        (Some(p), Ok((_, mut t))) => { if t.0 != *p { t.0 = p.clone(); } }
        (Some(p), Err(_)) => { commands.spawn((Text::new(p.clone()), TextFont { font_size: bevy::text::FontSize::Px(34.), ..default() }, TextColor(Color::srgb(1., 0.85, 0.2)), PromptText,
                                               Node { position_type: PositionType::Absolute, top: percent(18), left: percent(38), ..default() }, GlobalZIndex(2500))); }
        (None, Ok((e, _))) => { commands.entity(e).despawn(); }
        (None, Err(_)) => {}
    }
}

// ---- the variant's custom intro [added]: shots around the ring, Mathilda's lines as captions (no voice yet) ----

/// One shot: length (frames), camera eye and target relative to her (her yaw frame), her clip (her own, the cutscene poses
/// turned by Actor::yaw_fix) and the subtitle (speaker, line). The Baron is not in her fight.
struct Shot { frames: f32, eye: Vec3, target: Vec3, clip: &'static str, line: Option<(&'static str, &'static str)> }
const SHOTS: [Shot; 7] = [
    // she drops into the ring from above, lands, settles (RinRin's jump flight and scaffold landing on her rig)
    Shot { frames: 90., eye: Vec3::ZERO, target: Vec3::ZERO, clip: "em0610_0000.mot", line: None },
    // she walks straight up to him
    Shot { frames: 115., eye: Vec3::ZERO, target: Vec3::ZERO, clip: "em0610_0001.mot", line: None },
    Shot { frames: 130., eye: Vec3::new(0.35, 1.68, 1.5), target: Vec3::new(0., 1.62, 0.), clip: "em0610_ev50_cut11.mot",
           line: Some(("MATHILDA", "Well, well. The Baron's little contestant made it all the way to the top.")) },
    Shot { frames: 120., eye: Vec3::new(-1.4, 1.3, 2.4), target: Vec3::new(0., 1.2, 0.), clip: "em0610_0000.mot",
           line: Some(("MATHILDA", "He's busy, sugar. Tonight you dance with me.")) },
    Shot { frames: 90., eye: Vec3::ZERO, target: Vec3::ZERO, clip: "em0610_0000.mot",
           line: Some(("KREELY", "Lady, I didn't come here to dance.")) },
    Shot { frames: 120., eye: Vec3::new(0.9, 1.15, 2.2), target: Vec3::new(0., 1.35, 0.), clip: "em0610_ev50_cut11.mot",
           line: Some(("MATHILDA", "Then you'll just have to bleed for me instead.")) },
    Shot { frames: 70., eye: Vec3::new(0., 0.6, 3.4), target: Vec3::new(0., 1.4, 0.), clip: "em0610_ev50_cut11.mot", line: None },
];
/// Her drop: from DROP_HEIGHT onto DROP_AT (opposite his start), falling for DROP_FRAMES; she walks up to WALK_STOP m
/// from him at her walk's own pace (em0610_0001: 2.02 m in 33 frames).
const DROP_AT: Vec3 = Vec3::new(0., 0., 0.);
const DROP_HEIGHT: f32 = 9.;
const DROP_FRAMES: f32 = 32.;
const WALK_STOP: f32 = 2.6;
/// After the last shot, as the boss entry after an event (play_intro.rs BossEntry): black for 13 updates, then the 15-update
/// room fade into the fight; the intro opens from black over 15.
const CINE_WAIT: f32 = 13.;
const CINE_FADE: f32 = 15.;

/// The intro clock (Update, real time at 30 frames a second); holds the fight while it plays, Enter skips.
fn cine(mut bn: ResMut<Baron>, mut play: ResMut<super::Play>, time: Res<Time<Virtual>>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>,
        mut pause: ResMut<super::pause::Pause>) {
    let Some(t) = bn.cine else { return; };
    if !bn.spawned { return; }
    let total: f32 = SHOTS.iter().map(|s| s.frames).sum();
    // inspection: MW_CINE_AT=<frame> starts the intro there
    let t = if t == 0. { std::env::var("MW_CINE_AT").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.) } else { t };
    let dt = time.delta_secs() * 30.;
    // skipping as the event intros do (play_intro.rs): hold Enter SKIP_HOLD_SECS, or Backspace / right mouse (B / Back)
    if t < total {
        if keys.pressed(KeyCode::Enter) { bn.skip_hold += time.delta_secs(); } else { bn.skip_hold = 0.; }
        if bn.skip_hold >= super::intro::SKIP_HOLD_SECS || keys.just_pressed(KeyCode::Backspace) || mouse.just_pressed(MouseButton::Right) || std::env::var("MW_CINE_SKIP").is_ok() {
            bn.cine = Some(total); bn.line = None; bn.skip_hold = 0.; return;
        }
    }
    let nt = t + dt;
    // the room entry after the last shot: black, then the fade into the fight; control returns at its end
    if nt >= total {
        bn.line = None;
        if nt >= total + CINE_WAIT + CINE_FADE {
            bn.cine = None; pause.release_inputs = true;
            bn.set(St::Wait, "em0610_0000.mot");
        } else {
            if t < total { bn.set(St::Wait, "em0610_0000.mot"); let p = Vec3::from_array(play.fight.player.pos); let her = bn.boss.pos; bn.boss.yaw = (p.x - her.x).atan2(p.z - her.z); }
            bn.cine = Some(nt);
        }
        return;
    }
    let (mut acc, mut k) = (0., 0);
    while k + 1 < SHOTS.len() && nt >= acc + SHOTS[k].frames { acc += SHOTS[k].frames; k += 1; }
    let (start_prev, mut acc2, mut prev) = (t, 0., 0);
    while prev + 1 < SHOTS.len() && start_prev >= acc2 + SHOTS[prev].frames { acc2 += SHOTS[prev].frames; prev += 1; }
    let s = &SHOTS[k];
    if k != prev || t == 0. { bn.boss.play(s.clip); bn.shot_at = acc; }
    bn.cine = Some(nt);
    bn.line = s.line.map(|(a, b)| (a.to_string(), b.to_string()));
    let p = Vec3::from_array(play.fight.player.pos);
    let u = nt - acc;
    match k {
        // the drop: RinRin's jump flight (em0440_0021) while she falls, her scaffold landing (0022) on touching down
        0 => {
            let f = (u / DROP_FRAMES).min(1.);
            bn.boss.pos = DROP_AT + Vec3::Y * DROP_HEIGHT * (1. - f * f);
            let want = if u < DROP_FRAMES { "em0440_0021.mot" } else if u < DROP_FRAMES + 30. { "em0440_0022.mot" } else { "" };
            let shown = bn.boss.shown_name(bn.rin_bank.as_ref()).to_string();
            if !want.is_empty() && shown != want { let st = bn.st; bn.set_rin(st, want); }
            if want.is_empty() && bn.boss.borrowed.is_some() { bn.boss.play("em0610_0000.mot"); }
            if u < DROP_FRAMES && bn.boss.frame > 4. { bn.boss.frame = 4.; }   // the flight's held mid-air pose
        }
        // the walk up to him
        1 => {
            let to = Vec3::new(p.x - bn.boss.pos.x, 0., p.z - bn.boss.pos.z);
            if to.length() > WALK_STOP { bn.boss.pos += to.normalize_or_zero() * 0.061 * dt; }
            else if bn.boss.name() != "em0610_0000.mot" { bn.boss.play("em0610_0000.mot"); }
        }
        _ => {}
    }
    bn.boss.frame += dt;
    let l = bn.len(); if bn.boss.frame >= l - 1. { bn.boss.frame = if matches!(k, 1) || bn.boss.name() == "em0610_0000.mot" { 0. } else { l - 1. }; }
    // she faces him; he stands at his start in his idle, facing where she lands (set once, so he does not turn with her)
    let her = bn.boss.pos;
    bn.boss.yaw = (p.x - her.x).atan2(p.z - her.z);
    play.fight.player.loco.facing = (DROP_AT.x - p.x).atan2(DROP_AT.z - p.z);
    let idle = play.fd.jack_clip(mw_sim::player::IDLE);
    let n = (play.fd.jack.clips[idle].frames.max(2) - 1) as f32;
    let m = &mut play.fight.player.motion;
    if m.clip != idle { m.play(idle, 0., true, true); }
    m.frame = (m.frame + dt).rem_euclid(n);
}

impl Baron {
    /// The intro's skip hint (play_continue.rs draws it as for the event intros): the hold's progress.
    pub fn cine_skip_hint(&self) -> Option<f32> {
        let total: f32 = SHOTS.iter().map(|s| s.frames).sum();
        self.cine.filter(|t| *t < total).map(|_| (self.skip_hold / super::intro::SKIP_HOLD_SECS).min(1.))
    }
    /// The intro's fade (intro::fade draws it): from black over CINE_FADE at the start, black for CINE_WAIT after the
    /// last shot, then the CINE_FADE room fade.
    pub fn cine_fade(&self) -> Option<[u8; 4]> {
        let t = self.cine?;
        let total: f32 = SHOTS.iter().map(|s| s.frames).sum();
        let a = if t < CINE_FADE { 1. - t / CINE_FADE } else if t < total { 0. } else if t < total + CINE_WAIT { 1. } else { 1. - (t - total - CINE_WAIT) / CINE_FADE };
        (a > 0.).then(|| [0, 0, 0, (a.clamp(0., 1.) * 255.) as u8])
    }
    /// Her jaw while she speaks: 0 closed .. 1 open, from the line's length (about 2.4 frames a character) [added].
    fn mouth(&self) -> f32 {
        let (Some(t), Some((who, line))) = (self.cine, self.line.as_ref()) else { return 0.; };
        if who != "MATHILDA" { return 0.; }
        let u = t - self.shot_at - 8.;
        if u < 0. || u > line.len() as f32 * 2.4 { return 0.; }
        // syllables about 7 frames apart, a little uneven
        ((u * 0.9).sin() * 0.6 + (u * 0.37).sin() * 0.4).abs()
    }
    /// The custom intro's camera: the shot's eye / target around her (her yaw frame), a slow push-in.
    pub fn cine_camera(&self, play: &super::Play) -> Option<(Vec3, Vec3, f32)> {
        let t = self.cine?;
        let total: f32 = SHOTS.iter().map(|s| s.frames).sum();
        if t >= total { return None; }
        let (mut acc, mut k) = (0., 0);
        while k + 1 < SHOTS.len() && t >= acc + SHOTS[k].frames { acc += SHOTS[k].frames; k += 1; }
        let s = &SHOTS[k];
        let u = ((t - acc) / s.frames).clamp(0., 1.);
        let her = self.boss.pos;
        let q = Quat::from_rotation_y(self.boss.yaw);
        // shot 3 looks at the player (around his start)
        let (eye, target) = match k {
            4 => { let p = Vec3::from_array(play.fight.player.pos); let qp = Quat::from_rotation_y(play.fight.player.loco.facing); (p + qp * Vec3::new(0.5, 1.75, 2.3), p + Vec3::Y * 1.6) }
            // the drop: low behind his shoulder, looking up at her coming down, following her to the landing
            0 => { let p = Vec3::from_array(play.fight.player.pos); (p + Vec3::new(1.3, 1.0, -1.8), her + Vec3::Y * 1.0) }
            // the walk: tracking beside her
            1 => (her + Vec3::new(2.8, 1.5, 0.8), her + Vec3::Y * 1.2),
            _ => { let push = 1. - 0.15 * u; (her + q * (s.eye * push), her + q * s.target) }
        };
        Some((eye, target, 0.8))
    }
}

#[derive(Component)]
struct LineText;
/// The subtitle font: the game's message typeface (tools/ui/stage_subtitle_font.py), loaded once.
#[derive(serde::Deserialize)]
struct SubFont { pages: Vec<SubPage>, glyphs: Vec<SubGlyph>, chars: HashMap<String, u16>, #[serde(rename = "baseAdvance")] advance: f32 }
#[derive(serde::Deserialize)]
struct SubPage { image: String, width: f32, height: f32 }
#[derive(serde::Deserialize)]
struct SubGlyph { id: u16, page: usize, rect: [f32; 4] }
/// The intro's subtitles [added lines, no voice yet] in the game's message font, centred on the event message line
/// (mw_sim::ui_message::event_lines: centre x 320, 20-unit glyphs; the row is 312 here, above the skip hint), wrapped at 560.
fn dialogue(bn: Res<Baron>, assets: Res<AssetServer>, windows: Query<&Window>, q: Query<Entity, With<LineText>>, mut font: Local<Option<SubFont>>, mut commands: Commands) {
    for e in &q { commands.entity(e).despawn(); }
    if !bn.on { return; }
    let Some((_, line)) = bn.line.as_ref() else { return; };
    if font.is_none() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/ui/subtitle-font/catalog.json");
        *font = std::fs::read_to_string(root).ok().and_then(|t| serde_json::from_str(&t).ok());
    }
    let (Some(f), Ok(window)) = (font.as_ref(), windows.single()) else { return; };
    let viewport = Vec2::new(window.width(), window.height());
    let k = (viewport.x / 640.).min(viewport.y / 360.);
    let offset = (viewport - Vec2::new(640., 360.) * k) * 0.5;
    let scale = 20. / 27.;   // the cells are 27 px tall; the event line draws 20 units
    let glyph = |c: char| f.chars.get(&c.to_string()).and_then(|id| f.glyphs.iter().find(|g| g.id == *id));
    let width = |w: &str| w.chars().map(|c| glyph(c).map_or(f.advance * 0.4, |g| g.rect[2] - g.rect[0] - 2.) * scale).sum::<f32>();
    // wrap at 560 units
    let mut lines: Vec<String> = vec![String::new()];
    for word in line.split(' ') {
        let cur = lines.last().unwrap();
        let next = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if width(&next) > 560. && !cur.is_empty() { lines.push(word.to_string()); } else { *lines.last_mut().unwrap() = next; }
    }
    let n = lines.len() as f32;
    for (i, l) in lines.iter().enumerate() {
        let w = width(l);
        let mut x = 320. - w * 0.5;
        // (raised from the event line's 330 so the skip hint at 330 stays clear)
        let y = 312. - (n - 1. - i as f32) * 22.;
        for c in l.chars() {
            let Some(g) = glyph(c) else { x += f.advance * 0.4 * scale; continue; };
            let p = &f.pages[g.page];
            let (gw, gh) = (g.rect[2] - g.rect[0], g.rect[3] - g.rect[1]);
            let mut img = ImageNode::new(assets.load(p.image.clone()));
            img.rect = Some(Rect::new(g.rect[0], g.rect[1], g.rect[2], g.rect[3]));
            let _ = (p.width, p.height);
            // the font's cells hang from the line top (an apostrophe's is 16 px, g's 29)
            commands.spawn((img, Node { position_type: PositionType::Absolute, left: px(offset.x + x * k), top: px(offset.y + (y - 27. * scale) * k),
                                        width: px(gw * scale * k), height: px(gh * scale * k), ..default() }, GlobalZIndex(2600), LineText));
            x += (gw - 2.) * scale;
        }
    }
}

// ---- the actor editor (play_editor.rs): em0623 the Baron, em0610 Mathilda (the variant's boss) ----

fn ed_idle(b: &Baron) -> &'static str { if b.mathilda_boss { "em0610_0000.mot" } else { "em0623_0000.mot" } }

/// Both at their editor spots (default: the boss 3.5 m in front of the player), full HP, any pair ended.
fn ed_reset(b: &mut Baron, play: &mut super::Play, arena: &super::Arena, spot: Option<Vec3>, pspot: Option<Vec3>) {
    if play.fight.paired { end_pair(b, play); }
    if play.fight.player.state() == mw_sim::fight::STATE_EXTERNAL { play.fight.end_external(); }
    play.fight.paired = false;
    b.exec = 0; b.auto = None; b.ed_loop = false; b.prompt = None; b.cam_run = None; b.clash_req = false; b.hurt_walk = None; b.guard_walk = 0.;
    b.hp = b.max_hp; b.landed = 0; b.defend_cd = 0.; b.m_side = Side::Wait; b.meter = 0.;
    let fy = |v: Vec3| super::floor(&arena.0, v + Vec3::Y * 2.).unwrap_or(v.y);
    let pp = pspot.unwrap_or(Vec3::new(0., 0., -1.5));
    let at = spot.unwrap_or(pp + Vec3::Z * 3.5);
    play.fight.player.pos = [pp.x, fy(pp), pp.z];
    play.fight.player.loco.facing = (at.x - pp.x).atan2(at.z - pp.z);
    play.fight.player.hp = mw_sim::fight::PLAYER_MAX_HP;
    play.cam_yaw = play.fight.player.loco.facing;
    b.boss.pos = Vec3::new(at.x, fy(at), at.z);
    b.boss.yaw = (pp.x - at.x).atan2(pp.z - at.z);
    let idle = ed_idle(b);
    b.set(St::Wait, idle);
}

/// An editor command, played through their own states.
fn ed_cmd(b: &mut Baron, cmd: &super::editor::Cmd, play: &mut super::Play, arena: &super::Arena, fx: &mut super::fx::Fx,
          spot: Option<Vec3>, pspot: Option<Vec3>) -> Result<(), String> {
    use super::editor::Cmd;
    let pl = Vec3::from_array(play.fight.player.pos);
    let pf = play.fight.player.loco.facing;
    let pfwd = Vec3::new(pf.sin(), 0., pf.cos());
    let busy = matches!(b.st, St::Clash | St::Hold | St::Grab | St::Finished) || play.fight.paired || b.exec > 0;
    if *cmd != Cmd::Reset && busy { return Err("Still playing a pair or the finish: R resets".into()); }
    let close = |b: &mut Baron, d: f32| { b.boss.pos = Vec3::new(pl.x, b.boss.pos.y, pl.z) + pfwd * d; b.boss.yaw = pf + std::f32::consts::PI; };
    let who = if b.mathilda_boss { "Mathilda" } else { "the Baron" };
    match cmd {
        Cmd::Reset => ed_reset(b, play, arena, spot, pspot),
        Cmd::Clip { name, looped } => {
            if b.boss.clips.contains_key(name.as_str()) { b.set(St::Editor, name); }
            else if b.mathilda_boss && b.rin_bank.as_ref().is_some_and(|r| r.clips.iter().any(|c| &c.name == name)) { b.set_rin(St::Editor, name); }
            else { return Err(format!("{name} is not in {who}'s bank")); }
            b.ed_loop = *looped;
        }
        Cmd::Scene { actor, player } => {
            if !b.boss.clips.contains_key(actor.as_str()) { return Err(format!("{actor} is not in {who}'s bank")); }
            b.pair_at = (b.boss.pos, b.boss.yaw);
            b.set(St::Editor, actor); b.ed_loop = false; b.ed_scene = true;
            if let Some(pc) = player { let pp = &mut *play; pp.fight.external(&pp.fd, pc, false, 0.); pp.fight.paired = true; pp.fight.player.pos = b.pair_at.0.to_array(); pp.fight.player.loco.facing = b.pair_at.1; }
        }
        Cmd::Move(m) if b.mathilda_boss => {
            let k = match m.as_str() { "swing" => 0, "follow" => 1, "windup" => 2, "smash" => 3,
                "threat" => { b.st = St::Threat; b.boss.play(M_THREATS[0]); b.voices.push(0x11B); return Ok(()); }
                "guard" => { b.set_rin(St::Guard, "em0440_0003.mot"); return Ok(()); }
                "avoid" => { b.set_rin(St::Avoid, "em0440_0028.mot"); return Ok(()); }
                other => return Err(format!("Mathilda has no move {other:?} (swing, follow, windup, smash, threat, guard, avoid)")) };
            close(b, SWING_AT);
            b.last_atk = k as u8; b.sub = 0; b.st = St::Combo; b.landed = 0; b.timer = 0.;
            b.boss.play(M_MOVES[k].clip);
        }
        Cmd::Move(m) => {
            match m.as_str() {
                "combo" => { close(b, 1.6); b.set(St::Combo, "em0623_0011.mot"); }
                "mach" => { close(b, 2.); b.set(St::Mach, "em0623_0000.mot"); b.voices.push(0x10E); }
                "dropkick" => { close(b, 3.); b.set(St::DropKick, "em0623_0000.mot"); }
                "shoryuken" => { close(b, 1.4); b.phase2 = true; b.set(St::Shoryuken, "em0623_0000.mot"); }
                "antiair" => { close(b, 1.4); b.set(St::AntiAir, "em0623_0107.mot"); }
                "bhupper" => { close(b, 1.5); b.set(St::BhUpper, "em0623_0000.mot"); }
                "godhand" => { close(b, 5.); b.set(St::GodHand, "em0623_0000.mot"); }
                "dempsey" => { close(b, 1.6); b.set(St::Dempsey, "em0623_0110.mot"); }
                "riderkick" => { close(b, 5.); b.set(St::RiderKick, "em0623_0203.mot"); b.cam_run = Some(("em0623_c002.mot".into(), 0., b.boss.pos, b.boss.yaw)); }
                "threat" => { b.voices.push(0x11B); let k = (b.roll() * 3.) as usize % 3; b.set(St::Threat, ["em0623_0001.mot", "em0623_0002.mot", "em0623_0003.mot"][k]); }
                "guard" => { b.set(St::Guard, "em0623_0121.mot"); b.voices.push(0x112); }
                "avoid" => b.set(St::Avoid, "em0623_0015.mot"),
                // the phase-2 turn after his first lost clash: 000c with its camera em0623_c003 (and the BGM switch)
                // the first oiran throws the player a recovery item (em0960_0002 with its camera em0960_c000)
                "toss" => { if b.toss_by.is_some() { return Err("An oiran is already throwing".into()); } b.toss_step = b.toss_step.min(4); b.toss_by = Some(0); }
                "angry" => { b.phase2 = true; b.set(St::Angry, "em0623_000c.mot"); b.cam_run = Some(("em0623_c003.mot".into(), 0., b.boss.pos, b.boss.yaw)); }
                other => return Err(format!("The Baron has no move {other:?} (combo, mach, dropkick, shoryuken, antiair, bhupper, godhand, dempsey, riderkick, angry, threat, guard, avoid)")),
            }
        }
        Cmd::Hurt(k) => {
            let fwd = Vec3::new(b.boss.yaw.sin(), 0., b.boss.yaw.cos());
            let right = Vec3::new(fwd.z, 0., -fwd.x);
            let from = b.boss.pos + match k.as_str() { "back" => -fwd, "left" => -right, "right" => right, _ => fwd } * 2.;
            if k == "shot" {
                let c = b.body(); let o = c + fwd * 4.;
                if b.shoot(o, (c - o).normalize(), 20, f32::INFINITY, fx).is_none() { return Err("The round missed".into()); }
                return Ok(());
            }
            // (no guard, avoid or deflection roll: the hit lands)
            b.defend_cd = 999.; if matches!(b.st, St::Wait | St::Step | St::Editor) { b.st = St::Damage; }
            let (dmg, atk) = match k.as_str() { "light" => (10, 0x6D), _ => (30, 0x2A) };
            // (before his first clash his HP floor holds: the editor lets it go)
            if !b.mathilda_boss { b.clashes = b.clashes.max(1); }
            b.hurt(dmg, atk, from, fx);
            b.defend_cd = 0.;
        }
        Cmd::Kill => {
            // down to the dizzy wait (finish-ready): the "finisher" entry, or the player's blow, ends it
            let from = b.boss.pos + Vec3::new(b.boss.yaw.sin(), 0., b.boss.yaw.cos()) * 2.;
            b.defend_cd = 999.; b.st = St::Damage; if !b.mathilda_boss { b.clashes = b.clashes.max(1); }
            let hp = b.hp; b.hurt(hp, 0x2A, from, fx); b.defend_cd = 0.;
        }
        Cmd::Pair { name, win } => match (b.mathilda_boss, name.as_str()) {
            (false, "clash") => { close(b, 1.3); b.auto = *win; b.set(St::Grab, "em0623_0300.mot"); b.sub = 1; }
            (false, "finale") => { close(b, 1.3); b.auto = *win; b.clashes = 1; b.phase2 = true; b.hp = 500; b.set(St::Grab, "em0623_0300.mot"); b.sub = 1; }
            (false, "bat") => { close(b, 1.3); b.phase2 = true; b.set(St::Shoryuken, "em0623_0107.mot"); b.sub = 1; }
            (true, "pin") | (true, "clash") => { close(b, 1.3); b.auto = *win; b.clash_req = true; }
            (_, "finisher") => {
                // the dizzy wait, then the finishing blow
                close(b, 1.3); b.hp = 1;
                if b.mathilda_boss { b.set_rin(St::Piyori, "em0440_1007.mot"); } else { b.set(St::Piyori, "em0623_000b.mot"); }
                if win.is_some() { let from = b.boss.pos + Vec3::new(b.boss.yaw.sin(), 0., b.boss.yaw.cos()) * 2.; b.hurt(10, 0x2A, from, fx); }
                else { return Err("Finish-ready: F2, then strike".into()); }
            }
            (_, other) => return Err(format!("{who} has no pair {other:?} ({})", if b.mathilda_boss { "pin, finisher" } else { "clash, finale, bat, finisher" })),
        },
    }
    Ok(())
}
