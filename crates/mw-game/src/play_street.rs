//! Street stage (MW_STAGE=street): Asia Town's Great Wall Street with its kung-fu grunts, in color like the boss arena.
//! c201.dat is the whole Asia Town level, so the world, collision and color pack are the arena's; the player starts at
//! the level's own start (PlayerTbl point 0) and the grunts come from the streets' spawn set c20100
//! (tools/stage_street.py), all em0101 kung fu (`kungfu-*.glb`, tools/stage_actor.py + stage_kungfu_shared.py).
//! The fight runs `solo` (no RinRin); the player's native moves resolve as ever and their attack sphere
//! (`Fight::player_strike`) and pistol rounds are tested against the grunts here.
//! Grunt behaviour follows the native cEmZako AI as em0101 uses it (research/ZAKO-AI.md, addresses there): wait until
//! the player is within 知覚距離, the "spotted" reaction (target_found, em0110_0036), then the besiege loop (siege_target
//! 0x7a): every grunt holds a slot on a ring around the player (9 angles in the camera frame, slot 0 between camera and
//! player), threatening (em0110_0042-44) or idling, side-walking back to its slot, backing off (walk-back em0110_0011 or
//! back-jump em0101_0121) when the player comes within 1.5 m. One grunt at a time holds the group's attack token (room
//! 攻撃者上限 = 1, the act number rotating after each attack): it approaches and rolls its per-frame attack chance (em0101
//! 攻撃頻度 2 % close, 遠攻撃頻度 5 % for the 3.5-5 m dash). Variant A (claws) throws 0104 (2.2 m) / 0105 (2.8 m) and
//! chains 0104 -> 0105 -> 0106 for 1-3 hits, or dashes (em0110_010d); variant B (gourd, the fire kung fu em0101B: HP 200,
//! 100 % rates, 1.5x power) breathes fire (0110, 3.5 m). Every hit is power 14 x 攻撃力. Struck: a 回避 dodge (back-jump)
//! on the 回避体力 roll while the 回避数 budget lasts, damage clips (em0110_1001/1000/1004/1008), launched only at HP <= 20 %
//! (吹っ飛び体力), stunned 5 s at 70 % and again at 30 % HP (ピヨリ, em0110_1078); down 0-1 s (em0110_1038), then up with
//! the neckspring above 60 % HP else em0110_103d. With the player dead they laugh (em0110_0045/46/4c/4d/4e, 3-4 s).
//! Launched (motion_blow): the flight em0110_1090 / 1091, the landing 1095 / 1096, then down. A gory kill (the head off)
//! frightens grunts within 9.5 m (msg 0x1011: 怖がり ~91 %, once each): after 3-12 frames they startle (em0110_0016 /
//! 0017), and on the 怖逃げ roll (80 %) turn and run 33-54 frames (0018 / 0015), stop (1512 / 0030) and come back. HP
//! regenerates 30 frames after the last hit, ramping to 0.24 HP a frame (moveMain). Only a grunt on the camera's side of
//! the player (no more than 0.3 m beyond him in camera depth) holds the attack token. The dead lie 60 s.
//! c201em.ini [em0101]: perception 12 m, CharColRadius 0.40; ring radius per grunt 2.5 + rand(0, 1) m (record ctor
//! 801ed124). [differs] the native street spawns only variant A; the outfit alternates here so the fire breather appears.
//! Guard (chkGuardDefault / face_guard, ZAKO-AI.md §4.1): when the evade roll fails, a 防御通常 5 % guard on the punch
//! types (RUSH_PUNCH, PUNCH_A0-A3 / B0, KUKRI) from the same once-per-life 回避数 budget: he turns to the player in
//! em0110_1063, two hits spark (core 12, the guard spark 0xC) without damage and the third lands; a chainsaw-class hit
//! (PUNCH_B1-B3, the counter, the bat charge) breaks it (GUARD_BREAK, em0110_10a4). After 45-60 frames he hops back
//! (catched_counter em0110_1075); 25 frames without a player attack, or beyond 3.5 m, he returns to the siege.
//! Dash dodges (msg 0x1006, §5.1): the siege attacker starting its dash or its run at the player warns the grunts in its
//! path (within 11 deg of its way); after 0-6 frames they step aside along the sender's side axis (run_one em0110_0012,
//! move_back 0011 / bk_jump em0101_0121, or the side runs 002b / 002c by how they face).
//! The taunt (player state 0x54, pl0030_00b0, PC T): every grunt within 15 m that is not angry is provoked after 3-9
//! frames: the spotted clip (em0110_0036), angry 390-480 frames (provoke_reaction: run in to 2.8-3.5 m, threat waits
//! while the player stays within 4.5-5 m), attacking without the token at +50 % a frame. [unresolved: the anger mark
//! parts effect 0x12f is not mapped to a staged emitter.]
//! The fire breath [added flame: em0101B's own flame emitter is not staged]: billboarded flame sprites from the grunt's
//! mouth (head bone 4) along his head's facing through the breath's window, white-yellow to orange, red and smoke; a
//! breath that lands sets the player burning for 66-90 frames (cEm0101::onAtkHit: msg 0x101c with that duration), flames
//! rising off his body and 1 HP every 10 frames (never the last). A clawed grunt's landed hit leaves claw marks on the
//! player where it struck (play_fx.rs `claw`).
//! The native grab (research/GRAB-THROW-MINIMAP.md): an A hold catches the nearest standing grunt within 2.5 m and 80
//! deg (catched_start em0110_1071, then the held loop 1072), kept 0.5535 m in front of the player and facing him while he
//! walks and steers. He struggles (1076 / 1077) once his hold time runs out (掴まれ体力: at once at full HP, 6 s from 99 %,
//! 8 s from 70 %) and breaks free after 5 s more (1075). The head-butt (25) plays 1079 and the hold goes on; letting go
//! plays 1073. The throws fly on their clips' own root travel (overhead em0110_100c -> 100d -> 100e, side 1013 -> 1014
//! -> 1015) along the player's facing [inferred: the throw offset in the player's .mot is not decoded]; the flying body
//! knocks down grunts it passes (BLOW) and lands for 30. While one is held the others attack at 他掴まれ攻撃頻度.
//! Waves keep coming: at most `ALIVE` at once.
//! Kreely's God Hand moves (mw_sim::kreely): his sweep drops a grunt with the God Hand sweep knockdown (gh_sweep_down,
//! retargeted onto em0101) before he lies and gets up; his grab suplexes the nearest grunt in reach (gh_suplex_recv,
//! placed in the grab's frame, SUPLEX_DAMAGE at the slam), who then lies and gets up (or dies).
//! Each grunt wears one of em0101's two outfits (A: the claws `tume_*_A_ON`; B: the straw hat, gourd and cord `*_B_ON`),
//! alternating as they spawn; the cut cross-sections (`*_DA`) stay hidden until a cut shows one.
//! Blood and gore: every hit bleeds with the native player-hit blood (core 1000 / 1005); heavy hits leave stains on the
//! body (core 535, type-24 decals, play_decal.rs); a kill sprays the waist (core 489) and soaks the clothes for good
//! (core 439). A heavy kill or a pistol headshot takes his head off [added gore, on the native pieces]: the head, pigtail
//! and hat come off (also every third kill, so the gore shows in a plain brawl) as one gib (posed from the skinned meshes at that moment) that flies and bounces, the neck's cut
//! cross-section (the top of `_00_DA`) caps the stump, the head bursts (core 486 + 543) and the neck spurts (core 1005)
//! for a few seconds.
use bevy::{asset::RenderAssetUsages, mesh::{skinning::{SkinnedMesh, SkinnedMeshInverseBindposes}, Indices, VertexAttributeValues}, prelude::*};
use mw_data::MotionBank;
use mw_sim::character::{sample_retargeted, strip_root_xz, Retarget};

const ALIVE: usize = 4;
/// 知覚距離 (case2/c201em.ini [em0101] overrides em_common.ini's 20).
const SENSE: f32 = 12.;
const WALK: f32 = 2.0 / 30.;
const RUN: f32 = 4.5 / 30.;
const SIDE: f32 = 1.4 / 30.;
/// The slot ring's 9 angles (table 0x81123104), degrees off the player-to-camera line (the radius is per grunt).
const SLOT_ANGLES: [f32; 9] = [0., 38., -38., 76., -76., 114., -114., 152., -152.];
/// 包囲戻り距離: farther than this from the slot it runs back.
const SIEGE_RETURN: f32 = 5.0;
/// The attacker's stop distance (siege 0x1c: max(1.2, (radius + 0.4) x 1.7) with the grunt's collision radius ~0.4).
const STOP: f32 = 1.36;
/// 9.8 m/s² per 30 Hz frame: the living and the dead fall to the floor under them.
const GRAVITY: f32 = 9.8 / 900.;
const SUPLEX_DAMAGE: i32 = 40;
/// Kreely's sweep (attack id under his moveset, mw_sim::kreely).
const SWEEP_ATK: u32 = 0x78;
/// em0101 MDB mesh indices (GLB role_0_mesh_<i>): what comes off with the head, the cut cross-sections, the outfits.
const HEAD_PARTS: [usize; 10] = [2, 6, 7, 8, 9, 10, 11, 12, 16, 17];
const CUTS: [usize; 3] = [13, 14, 15];
const NECK_CAP_SOURCE: usize = 14;
const OUTFIT_B: [usize; 4] = [16, 17, 18, 19];
const OUTFIT_A: [usize; 1] = [20];
/// Game frames the neck spurts after the head comes off.
const BLEED: u32 = 90;
/// 気絶体力 / 気絶回復 (em_common.ini [em0101]): faint at 40 % HP, for 4 s
const FAINT_PCT: i32 = 40;
const FAINT_FRAMES: f32 = 120.;
/// The split grunt's place in Jack's frame: pl0030_0403 / 0404's pseudo-bone 1024 (GetThrowOffset), facing him.
const SPLIT_AT: [Vec3; 2] = [Vec3::new(-0.1141, 0., 1.6141), Vec3::new(-0.0187, 0., 1.6134)];
/// The meshes: zubon (the legs, the waist split's lower half) and the cross-sections' bind spans.
const ZUBON: usize = 3;
/// The dead lie this many game frames (still taking hits: they jolt, slide and bleed), then fade and sink over FADE.
const CORPSE: f32 = 1800.;
const FADE: f32 = 75.;
/// em0101's attack records (0x81162e68 / 0x81162fd0): clip, SEQ hit windows (type 19/20), combo window (type 11), range,
/// yaw window (degrees). Power is 14 for all.
struct Atk { clip: &'static str, windows: &'static [(f32, f32)], combo: Option<f32>, range: f32, yaw: f32 }
const ATTACKS: [Atk; 5] = [
    Atk { clip: "em0101_0104.mot", windows: &[(20., 24.), (31., 35.)], combo: Some(37.), range: 2.2, yaw: 15. },   // 0x105
    Atk { clip: "em0101_0105.mot", windows: &[(16., 21.), (26., 31.)], combo: Some(33.), range: 2.8, yaw: 20. },   // 0x106
    Atk { clip: "em0101_0106.mot", windows: &[(7., 12.)], combo: None, range: 3.5, yaw: 30. },                     // 0x107, combo only
    Atk { clip: "em0101_0110.mot", windows: &[(55., 88.)], combo: None, range: 3.5, yaw: 30. },                    // 0x104, fire breath (B)
    Atk { clip: "em0110_010d.mot", windows: &[(26., 31.)], combo: None, range: 4.5, yaw: 15. },                    // 0x604, the dash
];
const POWER: i32 = 14;
const FIRE: usize = 3;
const DASH: usize = 4;
/// The live tables: assets/defs/actors/em0101.json (play_actors.rs) when present, else the built-in ones above.
/// (Attack order matters: a combo chains into the next attack in the list; `fire` / `dash` are found by name.)
struct Tables { attacks: Vec<super::actors::AttackDef>, props: Vec<Props>, power: i32, laughs: Vec<String>, threats: Vec<String>,
                fire: usize, dash: usize, alive: usize, sense: f32, pools: [Vec<usize>; 2] }
fn tables() -> &'static Tables {
    static T: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let def = super::actors::get("em0101");
        let attacks: Vec<super::actors::AttackDef> = match def.filter(|d| !d.attacks.is_empty()) {
            Some(d) => d.attacks.clone(),
            None => ATTACKS.iter().zip(["claw1", "claw2", "claw3", "fire", "dash"]).map(|(a, n)| super::actors::AttackDef {
                name: n.into(), clip: a.clip.into(), windows: a.windows.iter().map(|w| [w.0, w.1]).collect(), combo: a.combo, range: a.range, yaw: a.yaw,
                power: None, atk: 0x173, voice: None }).collect(),
        };
        let props: Vec<Props> = match def.filter(|d| !d.variants.is_empty()) {
            Some(d) => d.variants.iter().map(|v| Props { hp: v.hp.unwrap_or(d.hp), near: v.near, far: v.far, power: v.power, evades: v.evades, evade: v.evade }).collect(),
            None => PROPS.iter().map(|p| Props { ..*p }).collect(),
        };
        let pool = |name: &str, built: &[&str]| def.map(|d| d.pool(name).to_vec()).filter(|v| !v.is_empty()).unwrap_or_else(|| built.iter().map(|s| s.to_string()).collect());
        let find = |n: &str, k: usize| attacks.iter().position(|a| a.name == n).unwrap_or(k);
        let (fire, dash) = (find("fire", FIRE), find("dash", DASH));
        // each variant's attack pool (names in its `attacks`; the built-in: A claws 0 / 1, B the breath)
        let pools = [0, 1].map(|o| def.and_then(|d| d.variants.get(o)).map(|v| v.attacks.iter().filter_map(|n| attacks.iter().position(|a| &a.name == n)).collect::<Vec<_>>())
            .filter(|v| !v.is_empty()).unwrap_or(if o == 1 { vec![fire] } else { vec![0, 1] }));
        Tables { power: def.map_or(POWER, |d| if d.power > 0 { d.power } else { POWER }), laughs: pool("laughs", &LAUGHS), threats: pool("threats", &THREATS), fire, dash,
                 alive: def.map_or(ALIVE as f32, |d| d.tune("alive", ALIVE as f32)) as usize, sense: def.map_or(SENSE, |d| if d.sense > 0. { d.sense } else { SENSE }),
                 attacks, props, pools }
    })
}
fn atk(k: usize) -> &'static super::actors::AttackDef { &tables().attacks[k] }
const LAUGHS: [&str; 5] = ["em0110_0045.mot", "em0110_0046.mot", "em0110_004c.mot", "em0110_004d.mot", "em0110_004e.mot"];
const THREATS: [&str; 3] = ["em0110_0042.mot", "em0110_0043.mot", "em0110_0044.mot"];

/// em_common.ini [em0101] / [em0101B] by outfit: HP, 攻撃頻度 / 遠攻撃頻度 (% per frame), 攻撃力, 回避数, 回避体力 %.
#[derive(Clone, Copy)]
struct Props { hp: i32, near: u32, far: u32, power: f32, evades: u32, evade: u32 }
const PROPS: [Props; 2] = [Props { hp: 120, near: 2, far: 5, power: 0.5, evades: 1, evade: 10 },
                           Props { hp: 200, near: 100, far: 100, power: 1.5, evades: 255, evade: 30 }];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Sub { Hold, Return, Step, Back, Approach }
#[derive(Clone, Copy, PartialEq, Debug)]
enum State { /// face_guard (0x2a); a move on a fixed travel (the guard's hop back, the dash dodges)
             Guard, Sidestep,
             /// provoke_reaction (0x71): running in, or a threat wait
             Provoke(bool),
             Wait, Spotted, Siege(Sub), Attack(usize), Damage, Dodge, Launched, Land, Swept, Suplexed, Down, GetUp, Piyori, Laugh,
             /// fear_to_escape: 0 startle, 1 run away, 2 stop
             Fear(u8),
             /// the native grab: held (struggling or not), head-butted, let go, shaken free, thrown (flight 0, air 1, landing 2)
             Held(bool), Headbutted, Released, Escape, Thrown(u8),
             /// the actor editor plays a clip
             Editor,
             /// impaled on a spike wall / hook (motion_stuck 0x46): 0 stuck (101c), 1 sliding off (101d), 2 landing
             /// (101e alive / 101f dead) (play_props.rs)
             Stuck(u8),
             /// faint_away_stand (0x66): reeling at 気絶体力 40 %, 4 s (em0110_1120 loop); the chainsaw finisher's target
             Faint,
             /// separate_substance (0x8F): a half of a grunt cut in two (the `Cut` says which), until its clip ends
             Split,
             /// [added, MW_STAGE=town] a civilian going about the town (play_town.rs)
             Civ(CivAct),
             /// [added] a zako picking on a civilian in a substory, until someone steps in
             Bully,
             Dead }

/// [added] What a civilian is doing: walking a nav path, standing about, chatting (facing `Civ::face`), knocked aside by
/// the player, startled, running from danger, cowering, or (a substory's victim) cowering before the zako.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum CivAct { #[default] Walk, Idle, Chat, Bumped, Startle, Flee, Cower, Victim }

/// [added] How a queued townsperson starts (play_town.rs): walking about, standing, chatting facing a yaw, a substory's
/// victim cowering toward a yaw, or a zako bullying toward a yaw.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Role { #[default] Walker, Idler, Chatter(f32), Victim(f32), Bully(f32) }

/// [added] A civilian's own state: which of the four (1..4), the nav path ahead, progress checks, how scared and of
/// what, the bump cool-down, what he goes back to after a bump, the way he faces when chatting.
#[derive(Clone, Debug, Default)]
struct Civ { kind: u8, path: Vec<Vec3>, check: (f32, Vec3), scared: f32, threat: Vec3, bump: f32, resume: CivAct, face: f32 }

/// [added] What a civilian did this frame, for the speech bubbles (play_town.rs): bumped by the player, frightened.
#[derive(Clone, Copy, Debug)]
pub enum CivEvent { Bumped(u32), Scared(u32), Hurt(u32) }

/// [added] A tagged townsperson (play_town.rs substories): id, tag, position, yaw, alive, still scripted (bullying /
/// cowering), a civilian, his height.
#[derive(Clone, Copy, Debug)]
pub struct Tagged { pub id: u32, pub tag: u32, pub pos: Vec3, pub yaw: f32, pub alive: bool, pub scripted: bool, pub civ: bool, pub height: f32, pub fighting: bool }

/// A civilian's rig (assets/native/civ/civN-rig.json, tools/town/stage_civilians.py): his bind offsets per bank bone, the
/// root scale against the grunt's clips and against Jack's (his walk), his height, head and skin parts.
pub struct CivRig { rel: Vec<Option<Vec3>>, root_grunt: f32, root_jack: f32, height: f32, head: Vec<usize>, skin: Vec<usize> }
const CIV_HP: i32 = 40;
const CIV_WALK: f32 = 1.05 / 30.;
const CIV_RUN: f32 = 4.2 / 30.;
/// The townsfolk's stand, walk and run: assets/defs/actors/civ.json `anims` (idle / walk / run; any clip in their bank,
/// tools/town/stage_civ_motion.py: the thug's em0110 set and the NPC banks' own locomotion), these if left out.
fn civ_anim(role: &str, default: &'static str) -> &'static str {
    super::actors::get("civ").and_then(|d| d.anims.get(role)).map_or(default, |s| s.as_str())
}
static CIV_CLIPS: std::sync::OnceLock<[&'static str; 3]> = std::sync::OnceLock::new();
fn civ_clips() -> &'static [&'static str; 3] { CIV_CLIPS.get_or_init(|| [civ_anim("idle", "em0110_0000.mot"), civ_anim("walk", "em0131_0011.mot"), civ_anim("run", "em0110_0018.mot")]) }
const CIV_CHAT: [&str; 4] = ["em0110_0045.mot", "em0110_0046.mot", "em0110_0038.mot", "em0110_0000.mot"];
/// Clothing tints per civilian (colour look; the skin keeps its palette) [added].
const CIV_TINTS: [[f32; 3]; 10] = [[1., 1., 1.], [1.15, 0.55, 0.5], [0.55, 0.75, 1.2], [0.6, 1.05, 0.65], [1.2, 1.0, 0.5], [0.85, 0.6, 1.15],
                                   [1.25, 0.85, 0.6], [0.5, 0.95, 1.1], [0.95, 0.95, 0.95], [1.1, 0.7, 0.9]];

/// The chainsaw split (separate_substance): vertical (CHAINSAW_TATEGIRI 0xCA, pl0030_0403, em0110_1130 / 1131) or at
/// the waist (CHAINSAW_YOKOGIRI 0xCB, pl0030_0404, em0110_1132 upper / 1133 legs); half B is the copy dmSpine makes;
/// `done` once the cut frame has passed (vertical 15, waist > 17), `meshes` once the half's meshes are swapped.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cut { waist: bool, b: bool, done: bool, meshes: bool }

struct Grunt { roots: [Entity; 2], joints: [Option<Vec<Entity>>; 2], pos: Vec3, yaw: f32, state: State, clip: usize, frame: f32,
               hp: i32, hit_by: u32, cool: f32, landed: u32, knock: Vec3, timer: f32, id: u32,
               /// siege slot; hold timer (0x167) and idle-loop time (0x169) in frames; attacks made (0x4a0); combo
               /// (count, max); evades left (0x4da); stun stages spent (0x15e4)
               slot: usize, hold: f32, idle: f32, attacks: u32, combo: (u32, u32), evades: u32, piyori: u8,
               /// falling speed (m per frame)
               vy: f32,
               /// ring radius (prop+0x24); frames since the last hit and the regenerated HP fraction; fear: the delay
               /// before the startle, the way he runs, and whether he was frightened already (0x45e bit 0x11)
               radius: f32, since_hit: f32, regen: f32, max_hp: i32, fear_delay: u32, flee_yaw: f32, feared: bool,
               /// outfit (0 A, 1 B); each model's primitives by MDB mesh index
               outfit: usize, parts: [Vec<(usize, Entity)>; 2],
               /// head off: `decap` until the gib is thrown (Update), the neck joint, spurt frames left, which way it flies
               headless: bool, decap: bool, neck: Option<Entity>, bleed: u32, fling: Vec3,
               /// a corpse's jolt from a hit (1 .. 0) and which way; its opacity while it fades and the faded material copies
               jolt: f32, jolt_dir: Vec3, fade: f32, fade_mats: bool,
               /// guards left (0x143e); the angry timer (0x12b8), the provocation delay and whether the spotted clip ends
               /// in anger; a pending dash dodge: its delay and escape direction
               guards: u32, angry: f32, provoke: u32, provoked: bool, dodge_delay: u32, dodge_dir: Vec3,
               /// the current string of punch (P) / chainsaw (C) hits taken (the combo log +0x15e8), oldest first
               combo_log: Vec<bool>,
               /// an object over his head (OBJ_COVER): blinded while his stun lasts
               covered: bool,
               /// fainted once already (気絶); a half of a split grunt
               fainted: bool, split: Option<Cut>,
               /// whose body: 0 the kung fu grunt, 1..3 a vampire zako (Elise's women, tools/vampzako) on his skeleton,
               /// 10..13 a civilian (civ1..4, tools/town)
               body: u8,
               /// [added, town] a civilian's own state; the substory tag (0 none); the colour look's clothes tinted
               civ: Option<Civ>, tag: u32, tinted: bool }

/// A vampire zako's rig (assets/native/vampzakoN-rig.json): her bind offsets per bank bone (rotation-only retarget:
/// em0101's MOTs turn the body joints, so her own offsets replace his) and the hip-height ratio for the root.
pub struct VampRig { rel: Vec<Option<Vec3>>, root_scale: f32,
                    /// per bank bone, the turn from her limb's bind direction to the grunt's (identity off the limbs): his
                    /// clips' local rotations become C(parent)^-1 * clip * C(bone) on her
                    corr: Vec<Quat> }
/// The vampire voices [added: the women's hurt and death cries borrowed from the female voices on the disc]: (block,
/// hurt ids, death ids). 1 Elise's (8122: em0630 1000 / 1001 / 1002 hurt, 1005 death), 2 RinRin's (8117: 1000 / 1001 /
/// 1002 hurt, 1007 collapse), 3 both.
const VAMP_VOICES: [&[(u32, u32)]; 3] = [&[(8122, 20), (8122, 21), (8122, 24)], &[(8117, 20), (8117, 21), (8117, 22)], &[(8122, 21), (8117, 20), (8122, 24), (8117, 22)]];
const VAMP_DEATH: [&[(u32, u32)]; 3] = [&[(8122, 25)], &[(8117, 24)], &[(8122, 25), (8117, 24)]];
fn body_files(body: u8) -> [String; 2] {
    if body >= 10 { let k = body - 9; return [format!("native/civ/civ{k}-source.glb"), format!("native/civ/civ{k}-color.glb")]; }
    if body == 0 { ["native/kungfu-source.glb".into(), "native/kungfu-pack.glb".into()] } else { [format!("native/vampzako{body}-source.glb"), format!("native/vampzako{body}-color.glb")] }
}

#[derive(serde::Deserialize)]
struct Spawn { position: [f32; 3] }
#[derive(serde::Deserialize)]
struct Start { position: [f32; 3], yaw_deg: f32 }
#[derive(serde::Deserialize)]
struct Meta { start: Start, spawns: Vec<Spawn> }

#[derive(Resource)]
/// `landed`: a player swing struck a grunt this game frame (its impact sound plays, play.rs `cue_sounds`).
pub struct Street { pub on: bool, pub landed: bool,
                   /// the street's own waves (MW_STAGE=street); grunts other stages asked for (position, yaw, outfit), spawned next step
                   waves: bool, pub queue: Vec<(Vec3, f32, usize, u8)>,
                   /// the vampire zako rigs (1..3), when staged; the women's cries this frame (block, id, where), played by
                   /// play.rs cue_sounds
                   vamps: Vec<Option<VampRig>>, pub voices: Vec<(u32, u32, Vec3)>, pub brood: usize,
                   /// grunts the player has knocked off their feet so far (play_kreely_voice.rs)
                   pub knocks: u32,
                   /// the actor editor edits em0101 here (play_editor.rs): its grunt (id), set up, its clip loops
                   ed: bool, ed_grunt: Option<u32>, ed_ready: bool, ed_loop: bool, colored: bool, neck_cap: Option<Handle<Mesh>>, bank: Option<MotionBank>, spawns: Vec<Vec3>, next: usize, grunts: Vec<Grunt>, pub kills: u32, clips: std::collections::HashMap<String, usize>,
                   /// The grunt in Kreely's suplex.
                   suplexed: Option<usize>,
                   /// the group's attack token (act number: the slot whose grunt may attack) and its holder; the view yaw the
                   /// slots are measured from; the AI's random draws
                   act: usize, attacker: Option<u32>, cam_yaw: f32, rng: u32,
                   /// frames the player keeps burning after a fire breath
                   pub burn: u32,
                   /// a gory kill this frame (sendFearPos source)
                   fear_at: Option<Vec3>,
                   /// the grunt the player holds, and whether he got away (the fight input's carry_lost until the hold ends)
                   held: Option<u32>, pub carry_lost: bool,
                   /// the throw: overhead or sideways, and the grunts its flying body already knocked down
                   throw_side: bool, blown: Vec<u32>,
                   /// the player was taunting last frame; dash heads-ups sent this frame (sender id, position, way)
                   taunt_seen: bool, dashes: Vec<(u32, Vec3, Vec3)>,
                   /// the split running (half A's id) while Jack plays his paired clip; the halves' meshes, per (source
                   /// mesh, which cut)
                   sever: Option<u32>, ed_split: Option<bool>, cut_meshes: std::collections::HashMap<(AssetId<Mesh>, u8), Handle<Mesh>>,
                   /// [added, town] the civilians' rigs (civ1..4); townsfolk to spawn (position, yaw, body, outfit, role, tag);
                   /// the walk grid; this frame's civilian events; danger the civilians run from (where, how far it carries)
                   civs: Vec<Option<CivRig>>, pub civ_queue: Vec<(Vec3, f32, u8, usize, Role, u32)>, pub nav: Option<std::sync::Arc<super::town::Nav>>,
                   pub civ_events: Vec<CivEvent>, alarms: Vec<(Vec3, f32)>,
                   /// [added, VR] the grunts held in a hand (play_vr_interact.rs), and the ones thrown by hand: (id, the
                   /// flight's velocity per frame at the release, the grunts the body has knocked down)
                   vr_held: Vec<u32>, vr_thrown: Vec<(u32, Vec3, Vec<u32>)> }

/// The town free roam (MW_STAGE=town, play_town.rs) [added]: the same streets with townsfolk and substories instead of waves.
pub fn town() -> bool { std::env::var("MW_STAGE").as_deref() == Ok("town") }

/// The street start (position, yaw) when MW_STAGE=street (or town).
pub fn start() -> Option<([f32; 3], f32)> {
    if !matches!(std::env::var("MW_STAGE").as_deref(), Ok("street") | Ok("town")) { return None; }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let meta: Meta = serde_json::from_str(&std::fs::read_to_string(root.join("assets/native/world/street-meta.json")).ok()?).ok()?;
    Some((meta.start.position, meta.start.yaw_deg.to_radians()))
}

pub struct StreetPlugin;
impl Plugin for StreetPlugin {
    fn build(&self, app: &mut App) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        // the street, and the stages that borrow its grunts as placeholder zako (Elise's bat-born ones)
        // (the actor editor on em0101: one grunt at a time, no waves)
        // (the editor's em0101 is the kung fu grunt; its vampzako the vampire women, the same street module)
        // (the editor's civ: the townsfolk, hosted on the town stage with its crowd off)
        let ed = matches!(super::editor::actor_id().as_deref(), Some("em0101") | Some("vampzako") | Some("civ")) && start().is_some();
        let waves = start().is_some() && !ed && !town();
        let on = start().is_some() || super::elise::on();
        let mut bank = on.then(|| std::fs::read_to_string(root.join("assets/native/kungfu-motion.json")).ok().and_then(|s| MotionBank::parse(&s).ok())).flatten();
        // [added, town] the civilians stand, walk and run on the thug's own clips (the bank's other em0110 clips are his too)
        let jack_hip = 1.08;
        if town() {
            #[derive(serde::Deserialize)] struct Clips { clips: Vec<mw_data::Clip> }
            match std::fs::read_to_string(root.join("assets/native/civ/civ-motion.json")).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<Clips>(&s).map_err(|e| e.to_string())) {
                Ok(c) => if let Some(b) = bank.as_mut() { let have: Vec<String> = b.clips.iter().map(|c| c.name.clone()).collect(); b.clips.extend(c.clips.into_iter().filter(|c| !have.contains(&c.name))); },
                Err(e) => eprintln!("[town] no civilian clips ({e}): run python -I tools/town/stage_civ_motion.py"),
            }
            // and Jack's plain walk / run / stand (pl0030 shares the body bone ids)
            if let (Some(b), Some(j)) = (bank.as_mut(), std::fs::read_to_string(root.join("assets/native/jack-player-motion.json")).ok().and_then(|s| MotionBank::parse(&s).ok())) {
                b.clips.extend(j.clips.into_iter().filter(|c| ["pl0030_0000.mot", "pl0030_0031.mot", "pl0030_0032.mot", "pl0030_0033.mot"].contains(&c.name.as_str())));
            }
        }
        let civs: Vec<Option<CivRig>> = (1..=4).map(|n| {
            if !town() { return None; }
            let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join(format!("assets/native/civ/civ{n}-rig.json"))).ok()?).ok()?;
            let b = bank.as_ref()?;
            root.join(format!("assets/native/civ/civ{n}-source.glb")).is_file().then_some(())?;
            let rel = b.bones.iter().map(|bone| v["bones"].as_array()?.iter().find(|x| x["id"].as_i64() == Some(bone.id as i64))
                .and_then(|x| Some(Vec3::new(x["relative"][0].as_f64()? as f32, x["relative"][1].as_f64()? as f32, x["relative"][2].as_f64()? as f32)))).collect();
            let list = |k: &str| v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as usize)).collect()).unwrap_or_default();
            let hip = v["hipHeight"].as_f64()? as f32;
            Some(CivRig { rel, root_grunt: v["rootScale"].as_f64()? as f32, root_jack: hip / jack_hip, height: v["height"].as_f64()? as f32, head: list("head"), skin: list("skin") })
        }).collect();
        if town() { eprintln!("[town] civilians staged: {:?}", civs.iter().map(|c| c.is_some()).collect::<Vec<_>>()); }
        // the God Hand receiver clips retargeted onto em0101 (tools/godhand/retarget_godhand.py)
        if let (Some(b), Some(m)) = (bank.as_mut(), std::fs::read_to_string(root.join("assets/native/kreely-moves.json")).ok().and_then(|s| mw_sim::kreely::KreelyMoves::parse(&s).ok())) {
            b.clips.extend(m.grunt_clips);
        }
        let meta: Option<Meta> = std::fs::read_to_string(root.join("assets/native/world/street-meta.json")).ok().and_then(|s| serde_json::from_str(&s).ok());
        // street-level spawns, nearest the start first
        let mut spawns: Vec<Vec3> = meta.map(|m| m.spawns.iter().map(|s| Vec3::from_array(s.position)).filter(|p| p.y < 1.).collect()).unwrap_or_default();
        let s0 = start().map(|s| Vec3::from_array(s.0)).unwrap_or_default();
        spawns.sort_by(|a, b| a.distance(s0).total_cmp(&b.distance(s0)));
        let clips = bank.as_ref().map(|b| b.clips.iter().enumerate().map(|(i, c)| (c.name.clone(), i)).collect()).unwrap_or_default();
        let vamps = (1..=3).map(|n| {
            let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join(format!("assets/native/vampzako{n}-rig.json"))).ok()?).ok()?;
            let b = bank.as_ref()?;
            let rel = b.bones.iter().map(|bone| v["bones"].as_array()?.iter().find(|x| x["id"].as_i64() == Some(bone.id as i64))
                .and_then(|x| Some(Vec3::new(x["relative"][0].as_f64()? as f32, x["relative"][1].as_f64()? as f32, x["relative"][2].as_f64()? as f32)))).collect();
            root.join(format!("assets/native/vampzako{n}-source.glb")).is_file().then_some(())?;
            let corr = b.bones.iter().map(|bone| v["correction"].get(bone.id.to_string()).and_then(|q| Some(Quat::from_xyzw(q[0].as_f64()? as f32, q[1].as_f64()? as f32, q[2].as_f64()? as f32, q[3].as_f64()? as f32))).unwrap_or(Quat::IDENTITY)).collect();
            Some(VampRig { rel, root_scale: v["rootScale"].as_f64()? as f32, corr })
        }).collect();
        app.insert_resource(Street { on: on && bank.is_some(), waves, queue: vec![], vamps, voices: vec![], brood: 0, knocks: 0, ed, ed_grunt: None, ed_ready: false, ed_loop: false, landed: false, colored: true, neck_cap: None, bank, spawns, next: 0, grunts: vec![], kills: 0, clips, suplexed: None,
                                    act: 0, attacker: None, cam_yaw: 0., rng: 0x2b1d_7a33, burn: 0, fear_at: None, held: None, carry_lost: false, throw_side: false, blown: vec![],
                                    taunt_seen: false, dashes: vec![], sever: None, ed_split: None, cut_meshes: Default::default(),
                                    civs, civ_queue: vec![], nav: None, civ_events: vec![], alarms: vec![], vr_held: vec![], vr_thrown: vec![] })
            .add_systems(Update, (bind, tint, pose, gore, gibs, fade, fire).chain());
    }
}

impl Street {
    fn clip(&self, name: &str) -> usize { self.clips[name] }
    fn has(&self, name: &str) -> bool { self.clips.contains_key(name) }
    fn frames(&self, clip: usize) -> f32 { self.bank.as_ref().unwrap().clips[clip].frames as f32 }
    fn set(&self, g: &mut Grunt, state: State, clip: &str) { g.state = state; g.clip = self.clip(clip); g.frame = 0.; g.landed = 0; }
    /// rand(0..n)
    fn roll(&mut self, n: u32) -> u32 { self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (self.rng >> 8) % n.max(1) }
    /// The grunt gives the attack token back (attack over, down, stunned, dead): the act number moves to the next slot.
    /// A vampire's cry: hurt (one of her pool, by roll) or death.
    fn cry(&mut self, g: &Grunt, death: bool) {
        // (the townswomen borrow the same voices: civ2 RinRin's, civ3 Elise's; the men stay silent but for the flee line)
        let set = match g.body { 1..=3 => g.body as usize - 1, 11 => 1, 12 => 0, _ => return };
        let pool = if death { VAMP_DEATH[set] } else { VAMP_VOICES[set] };
        let (b, id) = pool[self.roll(pool.len() as u32) as usize];
        self.voices.push((b, id, g.pos + Vec3::Y * 1.6));
        if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] vampire {} (body {}) cries {b}:{id}{}", g.id, g.body, if death { " (death)" } else { "" }); }
    }
    fn release(&mut self, g: &Grunt) { if self.attacker == Some(g.id) { self.attacker = None; self.act = (self.act + 1) % SLOT_ANGLES.len(); } }

    /// A pistol round from `origin` along `dir`: the nearest grunt it passes through takes `damage`; returns the hit point
    /// and the grunt's shown model (for the wounds).
    /// How far along a ray the first standing grunt is (no harm done: where the screen's shot converges).
    pub fn ray_probe(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        self.grunts.iter().filter(|g| g.state != State::Dead).filter_map(|g| {
            let c = g.pos + Vec3::Y;
            let t = (c - origin).dot(dir);
            let p = origin + dir * t;
            (t > 0. && Vec2::new(p.x - g.pos.x, p.z - g.pos.z).length() < 0.35 && (g.pos.y + 0.3..g.pos.y + 2.1).contains(&p.y)).then_some(t)
        }).min_by(f32::total_cmp)
    }

    pub fn shoot(&mut self, origin: Vec3, dir: Vec3, damage: i32, range: f32, fx: &mut super::fx::Fx) -> Option<(Vec3, Entity)> {
        let mut best: Option<(usize, f32)> = None;
        for (i, g) in self.grunts.iter().enumerate() {
            let dead = g.state == State::Dead;
            if dead && g.fade < 1. { continue; }
            // standing: a vertical capsule 0.3..2.1 m (em0101 stands 2.1 m to the top of his head), radius 0.35; a corpse:
            // a low slab 0.9 m around his position, up to 0.7 m
            let c = g.pos + Vec3::Y * if dead { 0.3 } else { 1.0 };
            let t = (c - origin).dot(dir);
            if t < 0. || t > range { continue; }
            let p = origin + dir * t;
            let (r, lo, hi) = if dead { (0.9, -0.1, 0.7) } else { (0.35, 0.3, 2.1) };
            if Vec2::new(p.x - g.pos.x, p.z - g.pos.z).length() < r && (g.pos.y + lo..g.pos.y + hi).contains(&p.y)
                && best.is_none_or(|b| t < b.1) { best = Some((i, t)); }
        }
        let (i, t) = best?;
        let at = origin + dir * t;
        let push = Vec3::new(dir.x, 0., dir.z).normalize_or_zero();
        if self.grunts[i].state == State::Dead { self.corpse_hit(i, push, at, fx); return Some((at, self.grunts[i].roots[self.colored as usize])); }
        // a headshot (above the shoulders) hits three times as hard
        let head = at.y > self.grunts[i].pos.y + 1.8;
        self.damage(i, if head { damage * 3 } else { damage }, push, false, 0, at, fx);
        Some((at, self.grunts[i].roots[self.colored as usize]))
    }

    /// A hit on a corpse: it bleeds, jolts and slides a little.
    fn corpse_hit(&mut self, i: usize, push: Vec3, at: Vec3, fx: &mut super::fx::Fx) {
        let yaw = push.x.atan2(push.z);
        fx.impact(at, yaw, 0.8, [1., 0.12, 0.08]); fx.burst("core", 1005, at, yaw, 1.);
        fx.splats.push((at, 0.5));
        let g = &mut self.grunts[i];
        g.jolt = 1.; g.jolt_dir = push; g.knock += push * 0.05;
    }

    fn damage(&mut self, i: usize, power: i32, push: Vec3, heavy: bool, atk: u32, at: Vec3, fx: &mut super::fx::Fx) {
        // chkGuardDefault: a dodge on the 回避体力 roll while the 回避数 budget lasts (not while down or held)
        let standing = matches!(self.grunts[i].state, State::Wait | State::Spotted | State::Siege(_) | State::Attack(_) | State::Laugh | State::Provoke(_));
        let pr = &tables().props[self.grunts[i].outfit % tables().props.len()];
        // isGuardEnable (cEm0101 81071e88): the punch types are guardable; the chainsaw class breaks a guard
        let guardable = matches!(atk, 0x5e | 0x6d..=0x70 | 0x75 | 0x99..=0x9c | 0xa1..=0xa4);
        let breaks = matches!(atk, 0x76..=0x78 | 0x81 | 0x96);
        let guarding = self.grunts[i].state == State::Guard;
        if guarding && !breaks && self.grunts[i].guards > 0 {
            // refHitPos: a blocked hit (no HP loss), the guard spark, facing the attacker
            let g = &mut self.grunts[i];
            g.guards -= 1; g.idle = 0.; g.yaw = (-push.x).atan2(-push.z);
            fx.burst("core", 12, at, push.x.atan2(push.z), 1.);
            if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] grunt {} blocked 0x{atk:x} ({} guards left)", g.id, g.guards); }
            return;
        }
        if standing && atk != SWEEP_ATK && self.grunts[i].evades > 0 && self.roll(100) < pr.evade && self.has("em0101_0121.mot") {
            let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
            g.evades -= 1; g.knock = push * 0.12; g.yaw = (-push.x).atan2(-push.z);
            self.release(&g);
            self.set(&mut g, State::Dodge, "em0101_0121.mot");
            self.grunts[i] = g;
            return;
        }
        // chkGuard: 防御通常 5 % (MW_STREET_GUARD overrides it, for inspection) on a guardable type, from the same budget
        let rate = std::env::var("MW_STREET_GUARD").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
        if standing && guardable && self.grunts[i].evades > 0 && self.roll(100) < rate && self.has("em0110_1063.mot") {
            let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
            if g.evades != 255 { g.evades -= 1; }
            g.yaw = (-push.x).atan2(-push.z); g.knock = Vec3::ZERO;
            self.release(&g);
            self.set(&mut g, State::Guard, "em0110_1063.mot");
            // the guard time (getTime 0x112..0x116: 45-60 frames); this first hit is the first of the two blocks
            g.timer = 45. + self.roll(16) as f32; g.guards = 1; g.idle = 0.;
            fx.burst("core", 12, at, push.x.atan2(push.z), 1.);
            if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] grunt {} guards 0x{atk:x}", g.id); }
            self.grunts[i] = g;
            return;
        }
        let yaw = push.x.atan2(push.z);
        let kill = self.grunts[i].hp - power <= 0;
        fx.gore(at, yaw, power, heavy, kill);
        // [added, town] blood in the street sends the townsfolk running
        self.alarms.push((self.grunts[i].pos, if kill { 16. } else { 9. }));
        if self.grunts[i].civ.is_some() { self.civ_events.push(CivEvent::Hurt(self.grunts[i].id)); }
        let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
        let root = g.roots[self.colored as usize];
        let gap = g.since_hit;
        g.hp -= power; g.since_hit = 0.; g.regen = 0.; g.guards = 2;
        let max = g.max_hp.max(1);
        let pct = g.hp * 100 / max;
        if heavy || power >= 25 { fx.burst_on("core", 535, at, yaw, root); }
        if g.hp <= 0 { self.cry(&g, true); } else if gap > 8. { self.cry(&g, false); }
        if g.hp <= 0 {
            fx.burst("core", 28, at, yaw, 1.);
            // the kill: the waist sprays and the clothes soak for good
            fx.burst("core", 489, g.pos + Vec3::Y * 1.1, yaw, 1.);
            fx.burst_on("core", 439, at, yaw, root);
            if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] kill (power {power}, heavy {heavy})"); }
            if !g.headless && (heavy || power >= 25 || at.y > g.pos.y + 1.8 || (self.kills as usize + i) % 3 == 0) {
                let head = g.pos + Vec3::Y * 1.95;
                fx.burst("core", 486, head, yaw, 1.); fx.burst("core", 543, head, yaw, 1.);
                g.headless = true; g.decap = true; g.bleed = BLEED; g.fling = push;
                self.fear_at = Some(g.pos);
                if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] head off (power {power}, heavy {heavy}, at {:.2} m)", at.y - g.pos.y); }
            }
            g.knock = push * 0.12; g.timer = 0.;
            self.release(&g);
            self.set(&mut g, State::Dead, "em0110_1032.mot");
        } else if g.state == State::Down && self.has("em0110_105c.mot") {
            // damage_down_hit 0x5a (81043e80): a blow on him lying face up plays 105c, then he lies on
            g.knock = push * 0.03; g.clip = self.clip("em0110_105c.mot"); g.frame = 0.; g.timer = g.timer.max(25.);
        } else if atk == SWEEP_ATK && self.has("gh_sweep_down.mot") {
            // swept off his feet: the God Hand sweep knockdown, then he lies and gets up
            g.knock = push * 0.03;
            self.release(&g);
            self.knocks += 1;
            self.set(&mut g, State::Swept, "gh_sweep_down.mot");
        } else if !g.fainted && pct <= FAINT_PCT && matches!(g.state, State::Wait | State::Spotted | State::Siege(_) | State::Attack(_) | State::Damage | State::Laugh | State::Provoke(_) | State::Guard) && self.has("em0110_1120.mot") {
            // setFaintAway (calcDamage 81012c54): at 気絶体力 40 % he reels, open to the chainsaw finisher, 気絶回復 4 s
            g.fainted = true; g.timer = FAINT_FRAMES; g.knock = push * 0.03;
            self.release(&g);
            self.set(&mut g, State::Faint, "em0110_1120.mot");
            if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] grunt {} faints ({pct} %)", g.id); }
        } else if (heavy || power >= 25) && pct <= 20 {
            // blow-type hits launch only at 吹っ飛び体力 (20 %) or less
            g.knock = push * 0.16;
            self.release(&g);
            let flight = if self.roll(2) == 0 { "em0110_1090.mot" } else { "em0110_1091.mot" };
            self.knocks += 1;
            self.set(&mut g, State::Launched, flight);
        } else if (g.piyori == 0 && pct <= 70) || (g.piyori == 1 && pct <= 30) {
            // ピヨリ体力: stunned 5 s, once at 70 % and once at 30 %
            g.piyori += 1; g.timer = 150.; g.knock = push * 0.04;
            self.release(&g);
            self.set(&mut g, State::Piyori, "em0110_1078.mot");
        } else {
            // getDamageMotion: the light damage clips (0x803 default em0110_1001; 0x802 1000, 0x80b 1004, 0x800 1008)
            g.knock = push * if heavy { 0.08 } else { 0.04 };
            let f = ["em0110_1001.mot", "em0110_1000.mot", "em0110_1004.mot", "em0110_1008.mot"][self.roll(4) as usize];
            // the punch / chainsaw strings (getComboDamageMotion 810177c4): the clip follows the string of hits so far
            let f = match combo_clip(&mut g, atk, pct, gap) { Some(c) if self.has(c) => { if c == "em0110_104d.mot" { g.knock = push * 0.16; } c } _ => f };
            // stunned, the light reactions become 0x91f (em0110_10a4)
            let f = if g.piyori > 0 && g.timer > 0. && matches!(f, "em0110_1000.mot" | "em0110_1001.mot" | "em0110_1004.mot" | "em0110_1008.mot") { "em0110_10a4.mot" } else { f };
            // a broken guard: the type becomes 0xef GUARD_BREAK (damage clip 0x91f)
            let f = if guarding && breaks && self.has("em0110_10a4.mot") { "em0110_10a4.mot" } else { f };
            self.set(&mut g, State::Damage, f);
        }
        g.yaw = (-push.x).atan2(-push.z);   // face the hit
        self.grunts[i] = g;
    }
}

impl Street {
    /// Where the living grunts stand (for the pistol sight).
    /// The living grunts a lock can hold: (id, position, lying down).
    pub fn lock_points(&self) -> Vec<(u32, Vec3, bool)> {
        if !self.on { return vec![]; }
        self.grunts.iter().filter(|g| g.civ.is_none() && !matches!(g.state, State::Dead | State::Suplexed | State::Thrown(_)))
            .map(|g| (g.id, g.pos, matches!(g.state, State::Down | State::Swept | State::Launched | State::Land | State::GetUp | State::Piyori))).collect()
    }
    /// A thrown object (OBJ_THROW 0x107, power 20) at `at` moving `vel`: the first standing grunt it meets takes it.
    pub fn obj_hit(&mut self, at: Vec3, vel: Vec3, fx: &mut super::fx::Fx) -> bool {
        if !self.on { return false; }
        let Some(i) = self.grunts.iter().position(|g| !matches!(g.state, State::Dead | State::Down | State::Stuck(_) | State::Held(_) | State::Suplexed | State::Thrown(_))
            && Vec2::new(g.pos.x - at.x, g.pos.z - at.z).length() < 0.7 && at.y > g.pos.y - 0.2 && at.y < g.pos.y + 2.) else { return false; };
        let push = Vec3::new(vel.x, 0., vel.z).normalize_or_zero();
        let ev = self.grunts[i].evades; self.grunts[i].evades = 0;
        self.damage(i, 20, push, false, 0x107, at, fx);
        self.grunts[i].evades = ev;
        true
    }
    /// The grunt an object would go over (OBJ_COVER, 2.5 m in front of the player, standing).
    pub fn cover_target(&self, pl: Vec3, facing: f32) -> Option<u32> {
        if !self.on { return None; }
        let fwd = Vec2::new(facing.sin(), facing.cos());
        self.grunts.iter().filter(|g| matches!(g.state, State::Wait | State::Spotted | State::Siege(_) | State::Attack(_) | State::Damage | State::Laugh | State::Provoke(_) | State::Piyori | State::Civ(_) | State::Bully))
            .map(|g| (g.id, Vec2::new(g.pos.x - pl.x, g.pos.z - pl.z))).filter(|(_, d)| d.length() < 2.5 && d.normalize_or_zero().dot(fwd) > 0.5)
            .min_by(|a, b| a.1.length().total_cmp(&b.1.length())).map(|(id, _)| id)
    }
    /// Over his head: blinded, staggering under it (the stun stands in for em0110_1513 / 1518, not staged), 5 s.
    pub fn cover(&mut self, id: u32, fx: &mut super::fx::Fx) -> bool {
        let Some(i) = self.grunts.iter().position(|g| g.id == id) else { return false; };
        let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
        self.release(&g);
        g.covered = true; g.timer = 150.; g.knock = Vec3::ZERO;
        self.set(&mut g, State::Piyori, "em0110_1078.mot");
        fx.impact(g.pos + Vec3::Y * 1.8, g.yaw, 1., [0.9, 0.85, 0.7]);
        self.grunts[i] = g;
        true
    }
    /// The covered grunt's head (where the object sits) while it is still on him.
    pub fn covered_head(&self, id: u32) -> Option<(Vec3, f32)> {
        self.grunts.iter().find(|g| g.id == id && g.covered && g.state == State::Piyori).map(|g| (g.pos + Vec3::Y * 1.72, g.yaw))
    }
    /// Inspection (MW_IMPALE_TEST): the first standing grunt launched from `from` along `vel` (a frame).
    pub fn test_launch(&mut self, from: Vec3, vel: Vec3) {
        let Some(i) = self.grunts.iter().position(|g| !matches!(g.state, State::Dead)) else { return; };
        let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
        self.release(&g);
        g.pos = from; g.knock = vel; g.yaw = (-vel.x).atan2(-vel.z);
        self.set(&mut g, State::Launched, "em0110_1091.mot");
        self.grunts[i] = g;
    }
    /// The chainsaw finisher on a fainted grunt (fn_802C3C0C roll 5 -> L_802C3F58; dmSpine / onSeparate): Jack plays
    /// his paired clip, the grunt is placed on its 1024 offset as half A and a copy of him spawns as half B.
    fn split(&mut self, i: usize, waist: bool, play: &mut super::Play, assets: &AssetServer, commands: &mut Commands, fx: &mut super::fx::Fx) {
        let (jack, facing) = (Vec3::from_array(play.fight.player.pos), play.fight.player.loco.facing);
        let pp = &mut *play;
        pp.fight.external(&pp.fd, if waist { "pl0030_0404.mot" } else { "pl0030_0403.mot" }, false, 2.);
        pp.fight.paired = true;
        let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
        self.release(&g);
        if self.held == Some(g.id) { self.held = None; }
        g.pos = jack + Quat::from_rotation_y(facing) * SPLIT_AT[waist as usize];
        g.pos.y = jack.y;
        g.yaw = facing + std::f32::consts::PI;
        g.hp = 0; g.knock = Vec3::ZERO; g.vy = 0.; g.timer = 0.; g.bleed = 0;
        g.split = Some(Cut { waist, b: false, done: false, meshes: false });
        self.set(&mut g, State::Split, if waist { "em0110_1132.mot" } else { "em0110_1130.mot" });
        self.kills += 1;
        // sendFearPos 9.5 m
        self.fear_at = Some(g.pos);
        // the copy (half B): the same grunt, both models, its own clip
        self.next += 1;
        self.cry(&g, true);
        let roots = body_files(g.body).map(|file| commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))),
                                   Transform::from_translation(g.pos).with_rotation(Quat::from_rotation_y(g.yaw)), Visibility::Hidden)).id());
        let mut b = Grunt { roots, pos: g.pos, yaw: g.yaw, hp: 0, max_hp: g.max_hp, outfit: g.outfit, body: g.body, id: self.next as u32, headless: g.headless, ..Grunt::dummy() };
        b.split = Some(Cut { waist, b: true, done: false, meshes: false });
        self.set(&mut b, State::Split, if waist { "em0110_1133.mot" } else { "em0110_1131.mot" });
        // the saw bites (the seq's effects 195 / 472 / 475 at its first frames)
        fx.burst("core", 195, g.pos + Vec3::Y * 1.4, g.yaw, 1.);
        self.sever = Some(g.id);
        if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] grunt {} split {} (half B {})", g.id, if waist { "at the waist" } else { "vertically" }, b.id); }
        self.grunts[i] = g;
        self.grunts.push(b);
    }
    pub fn targets(&self) -> Vec<Vec3> { if !self.on { return vec![]; } self.grunts.iter().filter(|g| g.civ.is_none() && !matches!(g.state, State::Dead | State::Split)).map(|g| g.pos).collect() }
    /// The nearest grunt Kreely can grab (standing, within reach), if any.
    fn grab_target(&self, pl: Vec3) -> Option<usize> {
        if !self.on || !self.has("gh_suplex_recv.mot") { return None; }
        self.grunts.iter().enumerate().filter(|(_, g)| matches!(g.state, State::Wait | State::Spotted | State::Siege(_) | State::Attack(_) | State::Damage | State::Piyori | State::Laugh | State::Fear(_) | State::Civ(_) | State::Bully))
            .map(|(i, g)| (i, Vec2::new(g.pos.x - pl.x, g.pos.z - pl.z).length())).filter(|(_, d)| *d < 1.6)
            .min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
    }
    /// Whether a standing grunt is within `reach` and in front of `facing` (within about 35 degrees).
    pub fn in_front(&self, pl: Vec3, facing: f32, reach: f32) -> bool {
        if !self.on { return false; }
        let fwd = Vec2::new(facing.sin(), facing.cos());
        self.grunts.iter().filter(|g| !matches!(g.state, State::Dead | State::Down | State::Suplexed | State::Swept | State::Launched | State::Land | State::GetUp)).any(|g| {
            let d = Vec2::new(g.pos.x - pl.x, g.pos.z - pl.z); let l = d.length();
            l < reach && l > 1e-3 && (d / l).dot(fwd) > 0.82
        })
    }
    /// The grunt the native grab would catch (802C2304 / 802CC5B0): the nearest standing one within 2.5 m, within 80 deg of
    /// the facing and 0.3 m in height (catch type 3: not attacking, down, flying or already held).
    fn carry_target(&self, pl: Vec3, facing: f32) -> Option<usize> {
        if !self.on { return None; }
        let fwd = Vec2::new(facing.sin(), facing.cos());
        self.grunts.iter().enumerate()
            .filter(|(_, g)| matches!(g.state, State::Wait | State::Spotted | State::Siege(_) | State::Damage | State::Piyori | State::Laugh | State::GetUp | State::Civ(_) | State::Bully))
            .filter(|(_, g)| (g.pos.y - pl.y).abs() <= 0.3)
            .map(|(i, g)| { let d = Vec2::new(g.pos.x - pl.x, g.pos.z - pl.z); (i, d.length(), d.normalize_or_zero().dot(fwd)) })
            .filter(|(_, l, dot)| *l <= 2.5 && *dot >= 80f32.to_radians().cos())
            .min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, ..)| i)
    }
    pub fn can_carry(&self, pl: Vec3, facing: f32) -> bool { self.held.is_none() && self.carry_target(pl, facing).is_some() }
    /// Whether a grunt is in reach of Kreely's grab (the fight input asks before the frame).
    pub fn can_grab(&self, pl: Vec3) -> bool { self.grab_target(pl).is_some() }
}

/// [added, town] What play_town.rs reads and asks of the townsfolk.
impl Street {
    /// Every tagged actor (the substories' zako and victims).
    pub fn tagged(&self) -> Vec<Tagged> {
        self.grunts.iter().filter(|g| g.tag != 0).map(|g| Tagged { id: g.id, tag: g.tag, pos: g.pos, yaw: g.yaw, alive: g.state != State::Dead && g.hp > 0,
            scripted: matches!(g.state, State::Bully | State::Civ(CivAct::Victim)), civ: g.civ.is_some(), height: self.height(g),
            fighting: g.civ.is_none() && !matches!(g.state, State::Bully | State::Dead) }).collect()
    }
    fn height(&self, g: &Grunt) -> f32 {
        if g.body >= 10 { self.civs.get(g.body as usize - 10).and_then(|c| c.as_ref()).map_or(1.8, |c| c.height) } else { 2.05 }
    }
    /// Where a speaker's head is (for a bubble), and whether he still stands to speak.
    /// Where a speaker's head is (for a bubble), whether he still stands to speak, and his body (0 a zako, 10..13 civ1..4).
    pub fn speaker(&self, id: u32) -> Option<(Vec3, bool, u8)> {
        self.grunts.iter().find(|g| g.id == id).map(|g| (g.pos + Vec3::Y * self.height(g), g.state != State::Dead && !g.headless && g.hp > 0, g.body))
    }
    /// The living civilians (id, position).
    pub fn civilians(&self) -> Vec<(u32, Vec3)> {
        self.grunts.iter().filter(|g| g.civ.is_some() && g.state != State::Dead).map(|g| (g.id, g.pos)).collect()
    }
    /// Someone steps in: the substory's zako turn on the player (the spotted reaction, then the angry rush), its victim runs.
    pub fn provoke_tag(&mut self, tag: u32, pl: Vec3) {
        for k in 0..self.grunts.len() {
            if self.grunts[k].tag != tag || self.grunts[k].state == State::Dead { continue; }
            let mut g = std::mem::replace(&mut self.grunts[k], Grunt::dummy());
            if g.civ.is_some() {
                let c = g.civ.as_mut().unwrap(); c.scared = 300.; c.threat = pl; c.path.clear();
                self.set(&mut g, State::Civ(CivAct::Flee), civ_clips()[2]);
            } else if g.state == State::Bully {
                g.yaw = (pl.x - g.pos.x).atan2(pl.z - g.pos.z);
                g.provoked = true; g.feared = true;
                if self.has("em0110_0036.mot") { self.set(&mut g, State::Spotted, "em0110_0036.mot"); } else { self.set(&mut g, State::Wait, "em0101_0000.mot"); }
            }
            self.grunts[k] = g;
        }
    }
    /// Inspection (MW_TOWN_LINEUP): the first civilians stand in a row `at` + k * `step`, facing `yaw`, each looping a clip.
    pub fn lineup(&mut self, at: Vec3, step: Vec3, yaw: f32, clips: &[String]) {
        let mut k = 0;
        for i in 0..self.grunts.len() {
            if k >= clips.len() { break; }
            if self.grunts[i].civ.is_none() || self.grunts[i].tag != 0 || !self.has(&clips[k]) { continue; }
            let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
            g.pos = at + step * k as f32; g.yaw = yaw; g.knock = Vec3::ZERO;
            self.set(&mut g, State::Civ(CivAct::Idle), &clips[k]);
            g.timer = 1e9;
            self.grunts[i] = g;
            k += 1;
        }
    }
    /// Turn every scripted member of a tag to face `at` (the cut-scene's look at the player).
    pub fn face_tag(&mut self, tag: u32, at: Vec3) {
        for g in self.grunts.iter_mut().filter(|g| g.tag == tag && g.state == State::Bully) { g.yaw = (at.x - g.pos.x).atan2(at.z - g.pos.z); }
    }
}

impl Grunt {
    fn dummy() -> Self { Grunt { roots: [Entity::PLACEHOLDER; 2], joints: [None, None], pos: Vec3::ZERO, yaw: 0., state: State::Wait, clip: 0, frame: 0.,
                                 hp: 0, hit_by: 0, cool: 0., landed: 0, knock: Vec3::ZERO, timer: 0., id: 0,
                                 slot: 0, hold: 0., idle: 0., attacks: 0, combo: (0, 0), evades: 0, piyori: 0, vy: 0.,
                                 radius: 3., since_hit: 0., regen: 0., max_hp: 1, fear_delay: 0, flee_yaw: 0., feared: false, outfit: 0, parts: [vec![], vec![]],
                                 headless: false, decap: false, neck: None, bleed: 0, fling: Vec3::ZERO,
                                 jolt: 0., jolt_dir: Vec3::ZERO, fade: 1., fade_mats: false,
                                 guards: 2, angry: 0., provoke: 0, provoked: false, dodge_delay: 0, dodge_dir: Vec3::ZERO, combo_log: vec![], covered: false, fainted: false, split: None, body: 0,
                                 civ: None, tag: 0, tinted: false } }
}

/// One 30 Hz game frame of the street (FixedUpdate, after the fight tick): spawning, grunt AI, the player's hits.
pub fn step(mut street: ResMut<Street>, mut play: ResMut<super::Play>, arena: Res<super::Arena>, mut fx: ResMut<super::fx::Fx>,
            mut editor: ResMut<super::editor::Editor>, assets: Res<AssetServer>, globals: Query<&GlobalTransform>, mut commands: Commands) {
    street.landed = false;
    street.colored = play.colored;
    if !street.on || street.bank.is_none() { return; }
    let s = &mut *street;
    // waves: keep `ALIVE` grunts on the street, entering from the nearest spawns in turn
    if s.ed && (!s.ed_ready || editor.take_if(|c| *c == super::editor::Cmd::Reset).is_some()) {
        s.ed_ready = true;
        let outfit = s.grunts.last().map_or(0, |g| g.outfit);
        // (MW_VAMPZAKO_ED=1..3 starts the editor on a vampire; her "vampire_N" / "grunt" moves switch)
        // the vampire actor is always one of the women (vampire 1 first; MW_VAMPZAKO_ED=2 / 3 starts on another), the
        // grunt actor always the grunt
        let vamp = super::editor::actor_id().as_deref() == Some("vampzako");
        let first = std::env::var("MW_VAMPZAKO_ED").ok().and_then(|v| v.parse::<u8>().ok()).filter(|n| (1..=3).contains(n)).unwrap_or(1);
        let body = if vamp { s.grunts.last().map(|g| g.body).filter(|b| *b > 0).unwrap_or(first) } else { 0 };
        // the civ actor: one of the four townsfolk (MW_CIV_ED=1..4 starts on another; "civ_N" switches)
        let body = if super::editor::actor_id().as_deref() == Some("civ") {
            let first = std::env::var("MW_CIV_ED").ok().and_then(|v| v.parse::<u8>().ok()).filter(|n| (1..=4).contains(n)).unwrap_or(1);
            s.grunts.last().map(|g| g.body).filter(|b| *b >= 10).unwrap_or(9 + first)
        } else { body };
        ed_reset(s, &mut play, &arena, editor.spot, editor.player_spot, outfit, body);
        editor.done(&super::editor::Cmd::Reset, Ok(()));
    }
    let alive = s.grunts.iter().filter(|g| !matches!(g.state, State::Dead | State::Split)).count();
    // the split's paired clip played out: Jack is his own again
    if s.sever.is_some() && (play.fight.external_done() || play.fight.player.state() != mw_sim::fight::STATE_EXTERNAL) {
        s.sever = None; play.fight.paired = false; play.fight.end_external();
    }
    // inspection: MW_SEVER_TEST=vertical | waist faints the nearest grunt 1.6 m in front of the player at tick 60 and
    // splits him at tick 75
    if let Ok(how) = std::env::var("MW_SEVER_TEST") {
        let pl = Vec3::from_array(play.fight.player.pos); let f = play.fight.player.loco.facing;
        if let Some(i) = s.grunts.iter().enumerate().filter(|(_, g)| !matches!(g.state, State::Dead | State::Split)).min_by(|a, b| a.1.pos.distance(pl).total_cmp(&b.1.pos.distance(pl))).map(|(i, _)| i) {
            if play.tick == 60 {
                let mut g = std::mem::replace(&mut s.grunts[i], Grunt::dummy());
                s.release(&g);
                g.pos = pl + Quat::from_rotation_y(f) * Vec3::Z * 1.6; g.yaw = f + std::f32::consts::PI; g.fainted = true; g.timer = 1e6;
                s.set(&mut g, State::Faint, "em0110_1120.mot");
                s.grunts[i] = g;
            }
            if play.tick == 75 && s.grunts[i].state == State::Faint { s.split(i, how == "waist", &mut play, &assets, &mut commands, &mut fx); }
        }
    }
    s.voices.clear();
    s.civ_events.clear();
    let mut want: Vec<(Vec3, f32, Option<usize>, bool, u8, Option<Role>, u32)> = s.queue.drain(..).map(|(p, y, o, b)| (p, y, Some(o), true, b, None, 0)).collect();
    // [added, town] the townsfolk and the substories' zako
    want.extend(s.civ_queue.drain(..).map(|(p, y, b, o, r, t)| (p, y, Some(o), false, b, Some(r), t)));
    if s.waves && alive < tables().alive && !s.spawns.is_empty() && play.tick % 20 == 0 {
        let at = s.spawns[s.next % s.spawns.len()];
        want.push((at, (play.fight.player.pos[0] - at.x).atan2(play.fight.player.pos[2] - at.z), None, false, 0, None, 0));
    }
    for (mut at, yaw, outfit, alert, body, role, tag) in want {
        let role = role.or((body >= 10).then_some(Role::Idler));
        // (a civilian whose model is not staged is left out)
        if body >= 10 && !s.civs.get(body as usize - 10).is_some_and(|c| c.is_some()) { continue; }
        // (a woman whose model is not staged comes as a grunt)
        let body = if body >= 10 || (body > 0 && s.vamps.get(body as usize - 1).is_some_and(|v| v.is_some())) { body } else { 0 };
        s.next += 1;
        if let Some(y) = super::floor(&arena.0, at + Vec3::Y) { at.y = y; }
        let mut roots = [Entity::PLACEHOLDER; 2];
        for (k, file) in body_files(body).into_iter().enumerate() {
            roots[k] = commands.spawn((super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(file))),
                                       Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)), Visibility::Hidden)).id();
        }
        let idle = s.clip("em0101_0000.mot");
        // debug/test only: MW_STREET_HP starts the grunts with less HP
        let outfit = outfit.unwrap_or(s.next % 2) % tables().props.len();
        let hp = std::env::var("MW_STREET_HP").ok().and_then(|v| v.parse().ok()).unwrap_or(tables().props[outfit % tables().props.len()].hp);
        let radius = 2.5 + s.roll(1000) as f32 / 1000.;
        s.grunts.push(Grunt { roots, pos: at, yaw, clip: idle, hp, outfit, body, id: s.next as u32, evades: tables().props[outfit % tables().props.len()].evades, radius, max_hp: hp, tag, ..Grunt::dummy() });
        if let Some(role) = role {
            let mut g = s.grunts.pop().unwrap();
            if body >= 10 {
                // a civilian: weak, no dodges, guards, stuns or fainting
                g.hp = CIV_HP; g.max_hp = CIV_HP; g.evades = 0; g.guards = 0; g.piyori = 2; g.fainted = true;
                g.civ = Some(Civ { kind: body - 9, ..default() });
                let k = s.roll(1000) as f32;
                match role {
                    Role::Chatter(f) => { g.yaw = f; g.civ.as_mut().unwrap().face = f; s.set(&mut g, State::Civ(CivAct::Chat), civ_clips()[0]); g.frame = k % 60.; g.civ.as_mut().unwrap().resume = CivAct::Chat; }
                    Role::Victim(f) => { g.yaw = f; g.civ.as_mut().unwrap().face = f; s.set(&mut g, State::Civ(CivAct::Victim), "em0110_110b.mot"); g.civ.as_mut().unwrap().resume = CivAct::Victim; }
                    Role::Idler => { g.timer = 60. + k % 240.; s.set(&mut g, State::Civ(CivAct::Idle), civ_clips()[0]); g.frame = k % 60.; }
                    _ => { s.set(&mut g, State::Civ(CivAct::Walk), civ_clips()[1]); g.frame = k % 20.; }
                }
            } else if let Role::Bully(f) = role {
                g.yaw = f; g.timer = 0.;
                let k = s.roll(tables().threats.len() as u32) as usize;
                s.set(&mut g, State::Bully, tables().threats[k].as_str());
            }
            s.grunts.push(g);
        }
        // asked for by another stage: already alert (the spotted reaction, then the siege)
        if s.ed { s.ed_grunt = s.grunts.last().map(|g| g.id); }
        if alert && !s.ed && s.has("em0110_0036.mot") { let mut g = s.grunts.pop().unwrap(); s.set(&mut g, State::Spotted, "em0110_0036.mot"); s.grunts.push(g); }
    }
    let frozen = s.ed && !editor.ai;
    if s.ed {
        if let Some(i) = s.ed_grunt.and_then(|id| s.grunts.iter().position(|g| g.id == id)) {
            if let Some(cmd) = editor.take() { let r = ed_cmd(s, i, &cmd, &mut play, &mut fx); editor.done(&cmd, r); }
            if let Some(waist) = s.ed_split.take() { if s.sever.is_none() { s.split(i, waist, &mut play, &assets, &mut commands, &mut fx); } }
            let g = &s.grunts[i];
            editor.anchor = (g.pos, g.yaw);
            editor.idle = matches!(g.state, State::Wait | State::Siege(_)) && play.fight.carry.is_none() && play.fight.suplex.is_none();
            let who = super::editor::actor_id().unwrap_or_else(|| "em0101".into());
            let name = s.bank.as_ref().map_or(String::new(), |b| super::editor::clip_label(&who, &b.clips[g.clip].name));
            let st = super::editor::phase_label(&who, "state", format!("{:?}", g.state));
            editor.info = format!("{name}   {:.0} / {:.0}   {st}   HP {}   {}", g.frame, s.frames(g.clip) - 1., g.hp, if g.body >= 10 { format!("townsperson {}", g.body - 9) } else if g.body > 0 { format!("vampire {}", g.body) } else if g.outfit == 0 { "claws".into() } else { "gourd".into() });
        }
    }
    let pl = Vec3::from_array(play.fight.player.pos);
    s.cam_yaw = play.cam_yaw;
    // the attack token: the besieging grunt in the act slot (the next slot along when that one is empty)
    // (moveMain drops it from a grunt more than 0.3 m beyond the player in camera depth, unless mid-attack)
    let cam_fwd = Vec3::new(play.cam_yaw.sin(), 0., play.cam_yaw.cos());
    let near_side = |g: &Grunt| (g.pos - pl).dot(cam_fwd) <= 0.3;
    if s.attacker.is_none_or(|id| !s.grunts.iter().any(|g| g.id == id && (matches!(g.state, State::Attack(_)) || (matches!(g.state, State::Siege(_)) && near_side(g))))) {
        s.attacker = None;
        for k in 0..SLOT_ANGLES.len() {
            let slot = (s.act + k) % SLOT_ANGLES.len();
            if let Some(g) = s.grunts.iter().find(|g| g.slot == slot && matches!(g.state, State::Siege(_)) && near_side(g)) { s.attacker = Some(g.id); s.act = slot; break; }
        }
    }
    // a gory kill frightens the grunts around it (onRecvMsg 0x1011): once each, 怖がり ~91 %, not mid-attack, within 9.5 m
    let fear_at_civ = s.fear_at;
    if let Some(src) = s.fear_at.take() {
        if std::env::var_os("MW_STREET_LOG").is_some() {
            eprintln!("[street] gory kill at {:.1}: {:?}", src, s.grunts.iter().map(|g| (format!("{:?}", g.state), g.feared, (g.pos.distance(src) * 10.).round() / 10.)).collect::<Vec<_>>());
        }
        for k in 0..s.grunts.len() {
            let g = &s.grunts[k];
            if g.feared || g.headless || !matches!(g.state, State::Wait | State::Siege(_) | State::Laugh | State::Spotted) || g.pos.distance(src) >= 9.5 { continue; }
            if s.roll(100) > 90 { continue; }
            let delay = 3 + s.roll(10);
            let g = &mut s.grunts[k];
            g.feared = true; g.fear_delay = delay;
            if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] grunt {} frightened ({:.1} m from the gore)", g.id, g.pos.distance(src)); }
            let away = Vec3::new(g.pos.x - src.x, 0., g.pos.z - src.z).normalize_or(-Vec3::Z);
            g.flee_yaw = away.x.atan2(away.z);
        }
    }
    // [added, town] danger the townsfolk run from: a gunshot (14 m round the muzzle), blood spilt (play damage), a gory
    // kill (sendFearPos, 16 m), a swing that whiffs right by them (4 m)
    for e in &play.gun_events { if let super::GunEvent::Shot { muzzle, .. } = e { s.alarms.push((*muzzle, 14.)); } }
    if let Some(src) = fear_at_civ { s.alarms.push((src, 16.)); }
    if let Some((hit, _)) = play.fight.player_strike() { s.alarms.push((Vec3::from_array(hit.center), 3.)); }
    for (src, r) in std::mem::take(&mut s.alarms) {
        for k in 0..s.grunts.len() {
            let g = &s.grunts[k];
            let Some(c) = &g.civ else { continue; };
            if g.pos.distance(src) > r || matches!(g.state, State::Dead) { continue; }
            if matches!(g.state, State::Civ(CivAct::Walk | CivAct::Idle | CivAct::Chat | CivAct::Bumped | CivAct::Cower)) && c.scared <= 0. {
                let mut g = std::mem::replace(&mut s.grunts[k], Grunt::dummy());
                let c = g.civ.as_mut().unwrap(); c.scared = 330. + s.roll(150) as f32; c.threat = src; c.path.clear();
                g.yaw = (src.x - g.pos.x).atan2(src.z - g.pos.z);
                let clip = ["em0110_0016.mot", "em0110_0017.mot"][s.roll(2) as usize];
                s.set(&mut g, State::Civ(CivAct::Startle), clip);
                g.frame = s.roll(8) as f32;
                s.civ_events.push(CivEvent::Scared(g.id));
                // a scream (the women's cries; now and then a man's yell, the grunt's flee line)
                if matches!(g.body, 11 | 12) { if s.roll(2) == 0 { s.cry(&g, false); } }
                else if s.roll(4) == 0 { play.fight.events.push(mw_sim::fight::FightEvent::Voice(0x1002)); }
                s.grunts[k] = g;
            } else if let Some(c) = s.grunts[k].civ.as_mut() { if c.scared > 0. { c.scared = c.scared.max(200.); c.threat = src; } }
        }
    }
    // Kreely's suplex on a grunt: the grab picks the nearest one in reach; the slam hurts; at the end he lies (or dies)
    for e in play.fight.events.clone() {
        match e {
            mw_sim::fight::FightEvent::SuplexStart { on_rin: false } => {
                if let (Some(i), Some(sx)) = (s.grab_target(pl), play.fight.suplex.as_ref()) {
                    let mut g = std::mem::replace(&mut s.grunts[i], Grunt::dummy());
                    g.pos = Vec3::from_array(sx.target); g.yaw = sx.target_facing; g.knock = Vec3::ZERO;
                    s.set(&mut g, State::Suplexed, "gh_suplex_recv.mot");
                    s.grunts[i] = g; s.suplexed = Some(i);
                }
            }
            // Kreely's suplex on the grunt he was carrying (Q in the hold)
            mw_sim::fight::FightEvent::SuplexFromHold => {
                if let (Some(i), Some(sx)) = (s.held.and_then(|id| s.grunts.iter().position(|g| g.id == id)), play.fight.suplex.as_ref()) {
                    let mut g = std::mem::replace(&mut s.grunts[i], Grunt::dummy());
                    g.pos = Vec3::from_array(sx.target); g.yaw = sx.target_facing; g.knock = Vec3::ZERO;
                    s.set(&mut g, State::Suplexed, "gh_suplex_recv.mot");
                    s.grunts[i] = g; s.suplexed = Some(i);
                }
                s.held = None; s.carry_lost = false;
            }
            mw_sim::fight::FightEvent::CarryStart => {
                let facing = play.fight.player.loco.facing;
                if let Some(i) = s.carry_target(pl, facing) {
                    let mut g = std::mem::replace(&mut s.grunts[i], Grunt::dummy());
                    s.release(&g);
                    // 掴まれ体力: the hold time before he struggles (frames), by HP %; at full HP he struggles at once
                    let pct = g.hp * 100 / g.max_hp.max(1);
                    g.timer = if pct <= 70 { 240. } else if pct <= 99 { 180. } else { 0. };
                    g.knock = Vec3::ZERO;
                    s.set(&mut g, State::Held(false), "em0110_1071.mot");
                    s.held = Some(g.id); s.carry_lost = false;
                    if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] tick {} caught grunt {} (hp {}, struggles in {} frames)", play.tick, g.id, g.hp, g.timer); }
                    s.grunts[i] = g;
                } else { s.carry_lost = true; }
            }
            mw_sim::fight::FightEvent::CarryRelease => {
                if let Some(g) = s.held.take().and_then(|id| s.grunts.iter_mut().find(|g| g.id == id)) {
                    if matches!(g.state, State::Held(_) | State::Headbutted) { g.state = State::Released; g.clip = s.clips["em0110_1073.mot"]; g.frame = 0.; }
                }
            }
            mw_sim::fight::FightEvent::CarryHeadbutt => {
                if let Some(i) = s.held.and_then(|id| s.grunts.iter().position(|g| g.id == id)) {
                    let at = s.grunts[i].pos + Vec3::Y * 1.75;
                    let push = Vec3::new(s.grunts[i].pos.x - pl.x, 0., s.grunts[i].pos.z - pl.z).normalize_or(Vec3::Z);
                    s.landed = true;
                    // HANG_HEADBUT: 25 ([damage_pl]); held, he only reels (1079) and the hold goes on
                    s.grunts[i].since_hit = 0.;
                    s.damage(i, 25, push, false, 0x48, at, &mut fx);
                    let g = &mut s.grunts[i];
                    if g.state != State::Dead { g.state = State::Headbutted; g.clip = s.clips["em0110_1079.mot"]; g.frame = 0.; g.knock = Vec3::ZERO; }
                    else { s.held = None; }
                }
            }
            mw_sim::fight::FightEvent::CarryThrow { side } => {
                if let Some(g) = s.held.take().and_then(|id| s.grunts.iter_mut().find(|g| g.id == id)) {
                    // the overhead throw's flight runs backward in the thrown actor's frame, the side throw's forward: both
                    // are turned to carry him along the player's facing
                    let f = play.fight.player.loco.facing;
                    g.yaw = if side { f } else { f + std::f32::consts::PI };
                    g.state = State::Thrown(0); g.clip = s.clips[if side { "em0110_1013.mot" } else { "em0110_100c.mot" }]; g.frame = 0.;
                    g.knock = Vec3::ZERO; g.since_hit = 0.;
                    s.throw_side = side; s.blown.clear();
                    if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] tick {} threw grunt {} ({})", play.tick, g.id, if side { "side" } else { "overhead" }); }
                }
            }
            mw_sim::fight::FightEvent::SuplexSlam { on_rin: false, at } => {
                if let Some(g) = s.suplexed.and_then(|i| s.grunts.get_mut(i)) {
                    g.hp -= SUPLEX_DAMAGE;
                    let at = Vec3::from_array(at) + Vec3::Y * 0.3;
                    fx.impact(at, g.yaw, 1., [1., 0.12, 0.08]); fx.burst("core", 1005, at, g.yaw, 1.);
                    s.landed = true;
                }
            }
            _ => {}
        }
    }
    if let Some(i) = s.suplexed.filter(|_| play.fight.suplex.is_none()) {
        s.suplexed = None;
        if i < s.grunts.len() {
            let mut g = std::mem::replace(&mut s.grunts[i], Grunt::dummy());
            if g.state == State::Suplexed {
                // his clip's final root travel becomes his position
                let clip = &s.bank.as_ref().unwrap().clips[g.clip];
                let r = mw_sim::sample(&s.bank.as_ref().unwrap().bones, clip, (clip.frames - 1) as f32).last().map(|p| p.translation).unwrap_or([0.; 3]);
                g.pos += Quat::from_rotation_y(g.yaw) * Vec3::new(r[0], 0., r[2]);
                if g.hp <= 0 { g.knock = Vec3::ZERO; g.timer = 0.; s.set(&mut g, State::Dead, "em0110_1014.mot"); }
                else { g.timer = s.roll(31) as f32; s.set(&mut g, State::Down, "em0110_1038.mot"); }
            }
            s.grunts[i] = g;
        }
    }
    // the hold is over on the player's side (released, thrown, hit out of it): the held grunt is let go
    if play.fight.carry.is_none() {
        s.carry_lost = false;
        if let Some(g) = s.held.take().and_then(|id| s.grunts.iter_mut().find(|g| g.id == id)) {
            if matches!(g.state, State::Held(_) | State::Headbutted) { g.state = State::Released; g.clip = s.clips["em0110_1073.mot"]; g.frame = 0.; }
        }
    }
    // the taunt (chkEvade, provocation): on entering state 0x54 every grunt within 15 m (|dy| < 8 m) that is not angry,
    // stunned, held or down is provoked after 3-9 frames
    let taunting = play.fight.player.state() == 84;
    if taunting && !s.taunt_seen {
        for k in 0..s.grunts.len() {
            let g = &s.grunts[k];
            let near = Vec2::new(g.pos.x - pl.x, g.pos.z - pl.z).length() < 15. && (g.pos.y - pl.y).abs() < 8.;
            if !near || g.angry > 0. || g.provoke > 0 || g.hp * 10 < g.max_hp
                || !matches!(g.state, State::Wait | State::Spotted | State::Siege(_) | State::Laugh | State::Guard) { continue; }
            let delay = 3 + s.roll(7);
            s.grunts[k].provoke = delay;
            if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] tick {} grunt {} provoked (in {delay} frames)", play.tick, s.grunts[k].id); }
        }
    }
    s.taunt_seen = taunting;
    // inspection: MW_STREET_TEST=dash lines two grunts up in front of the player at tick 120 (8 m and 4 m) and has the far
    // one send its dash heads-up through the near one
    if play.tick == 120 && std::env::var("MW_STREET_TEST").as_deref() == Ok("dash") && s.grunts.len() >= 2 {
        let f = play.fight.player.loco.facing;
        let dir = Vec3::new(f.sin(), 0., f.cos());
        for (k, d) in [(0, 8.), (1, 4.)] {
            let mut g = std::mem::replace(&mut s.grunts[k], Grunt::dummy());
            g.pos = pl + dir * d; g.yaw = f + std::f32::consts::PI;
            s.set(&mut g, State::Siege(Sub::Hold), "em0101_0000.mot");
            s.grunts[k] = g;
        }
        s.dashes.push((s.grunts[0].id, s.grunts[0].pos, -dir));
    }
    // msg 0x1006: grunts in a dashing grunt's path (dot > 0.98) step out after 0-6 frames, to their side of its way
    for (from, at, way) in std::mem::take(&mut s.dashes) {
        let side = Vec3::new(way.z, 0., -way.x);
        for k in 0..s.grunts.len() {
            let g = &s.grunts[k];
            let rel = Vec3::new(g.pos.x - at.x, 0., g.pos.z - at.z);
            if g.id == from || g.dodge_delay > 0 || rel.length() > 10. || rel.normalize_or_zero().dot(way) <= 0.98
                || !matches!(g.state, State::Wait | State::Siege(_) | State::Laugh | State::Spotted) { continue; }
            let dir = if rel.dot(side) >= 0. { side } else { -side };
            let delay = 1 + s.roll(7);
            let g = &mut s.grunts[k];
            g.dodge_delay = delay; g.dodge_dir = dir;
            if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] tick {} grunt {} warned by grunt {from}'s dash", play.tick, g.id); }
        }
    }
    let strike = play.fight.player_strike();
    let player_alive = play.fight.player.hp > 0;
    let n = s.grunts.len();
    for i in 0..n {
        // the player's swing
        if let Some((hit, serial)) = strike {
            let (state, by, gpos) = (s.grunts[i].state, s.grunts[i].hit_by, s.grunts[i].pos);
            let dead = state == State::Dead;
            if state != State::Suplexed && state != State::Split && !(dead && s.grunts[i].fade < 1.) && by != serial {
                let c = Vec3::from_array(hit.center);
                let body = Vec2::new(c.x - gpos.x, c.z - gpos.z).length();
                // a corpse lies low and long: kicks and low swings reach it
                let (reach, lo, hi) = if dead { (0.9, -0.3, 0.9) } else { (0.45, 0., 2.) };
                if body < hit.radius + reach && (gpos.y + lo..gpos.y + hi).contains(&c.y) {
                    s.grunts[i].hit_by = serial;
                    let push = Vec3::new(gpos.x - pl.x, 0., gpos.z - pl.z).normalize_or(Vec3::Z);
                    let heavy = hit.power >= 20;
                    // the B (chainsaw) swing on a fainted grunt: one roll in 6 is the split (rand % 6 == 5), vertical or
                    // at the waist by rand(0,1); the other rolls are executions not ported, so the swing lands as a hit.
                    // MW_SEVER=1 (or vertical / waist) always splits.
                    let force = std::env::var("MW_SEVER").ok();
                    if state == State::Faint && matches!(hit.atk, 0x75..=0x78) && s.sever.is_none() && (force.is_some() || s.roll(101) % 6 == 5) && s.has("em0110_1130.mot") {
                        let waist = match force.as_deref() { Some("waist") => true, Some("vertical") => false, _ => s.roll(2) == 1 };
                        s.split(i, waist, &mut play, &assets, &mut commands, &mut fx);
                        s.landed = true;
                        continue;
                    }
                    if dead { s.corpse_hit(i, push * if heavy { 2. } else { 1. }, c, &mut fx); } else { s.damage(i, hit.power, push, heavy, hit.atk, c, &mut fx); }
                    s.landed = true;
                    play.fight.hitstop = play.fight.hitstop.max(hit.hitstop);
                }
            }
        }
        let mut g = std::mem::replace(&mut s.grunts[i], Grunt::dummy());
        let len = s.frames(g.clip);
        g.frame += 1.;
        let ended = g.frame >= len - 1.;
        let to = Vec3::new(pl.x - g.pos.x, 0., pl.z - g.pos.z);
        let dist = to.length();
        let face = |g: &mut Grunt, k: f32| { let want = to.x.atan2(to.z); g.yaw = mw_sim::player::rad_lerp_range(g.yaw, want, k, 0.3); };
        g.cool = (g.cool - 1.).max(0.);
        let mut step = Vec3::ZERO;
        let pr = tables().props[g.outfit % tables().props.len()];
        // the slot (calcSiegePos): SLOT_ANGLES off the player-to-camera line, SIEGE_RADIUS out
        let to_cam = -Vec3::new(s.cam_yaw.sin(), 0., s.cam_yaw.cos());
        let slot_dir = Quat::from_rotation_y(SLOT_ANGLES[g.slot].to_radians()) * to_cam;
        // pulled in front of walls and stalls (ckLineAttr_Obs: the line from the player, short of the hit by 0.1 + body radius)
        let o = pl + Vec3::Y * 0.8;
        let reach = mw_sim::collision::raw_surface_ray(&arena.0, [o.x as f64, o.y as f64, o.z as f64], [slot_dir.x as f64, 0., slot_dir.z as f64], g.radius as f64, Some("col.hkx"))
            .ok().and_then(|h| h.iter().map(|h| h.distance as f32).min_by(f32::total_cmp)).map_or(g.radius, |d| (d - 0.5).max(1.2));
        let slot_at = pl + slot_dir * reach;
        let to_slot = Vec3::new(slot_at.x - g.pos.x, 0., slot_at.z - g.pos.z);
        let attacker = s.attacker == Some(g.id);
        let fwd = Quat::from_rotation_y(g.yaw) * Vec3::Z;
        let off = to.normalize_or_zero().dot(fwd).clamp(-1., 1.).acos().to_degrees();   // degrees between facing and the player
        // back to holding the slot: a threat wait when the player is far, the plain wait close (siege 0x10); non-attackers
        // hold 15-30 frames before reshuffling (0x167)
        fn to_hold(s: &mut Street, g: &mut Grunt, dist: f32, attacker: bool) {
            // [added, town] a civilian gets up and runs (or, unhurt and calm, goes back to what he was doing)
            if g.civ.is_some() {
                let hurt = g.hp < g.max_hp;
                let c = g.civ.as_mut().unwrap();
                if hurt || c.scared > 0. { c.scared = c.scared.max(300.); c.path.clear(); s.set(g, State::Civ(CivAct::Flee), civ_clips()[2]); }
                else { let r = c.resume; civ_enter(s, g, r); }
                return;
            }
            let clip = if dist > 4.5 && s.roll(100) < 45 { tables().threats[s.roll(tables().threats.len() as u32) as usize].as_str() } else { "em0101_0000.mot" };
            s.set(g, State::Siege(Sub::Hold), clip);
            g.hold = if attacker { 0. } else { 15. + s.roll(16) as f32 }; g.idle = 0.;
        }
        // a free slot when joining the siege (getNoInGroup: the lowest unused number)
        fn join(s: &mut Street, g: &mut Grunt) {
            let used: Vec<usize> = s.grunts.iter().filter(|o| o.id != g.id && !matches!(o.state, State::Dead | State::Wait | State::Spotted)).map(|o| o.slot).collect();
            g.slot = (0..SLOT_ANGLES.len()).find(|k| !used.contains(k)).unwrap_or(0);
        }
        fn start_attack(s: &mut Street, g: &mut Grunt, k: usize) {
            g.combo = (1, match k { 0 => 1 + s.roll(3), 1 => 1 + s.roll(2), _ => 1 });
            s.set(g, State::Attack(k), &atk(k).clip);
        }
        // held: 0.5535 m in front of the player, facing him (moveCatchedPos 81050520: eased 0.3, then rigid)
        if matches!(g.state, State::Held(_) | State::Headbutted) && !s.vr_held.contains(&g.id) {
            let f = play.fight.player.loco.facing;
            let want = pl + Vec3::new(f.sin(), 0., f.cos()) * 0.5535;
            g.pos = g.pos.lerp(Vec3::new(want.x, g.pos.y, want.z), if g.frame < 4. { 0.3 } else { 1. });
            g.yaw = f + std::f32::consts::PI;
        }
        // the player down: laugh (笑い確率: em0101 100 %, em0101B 0 %)
        let laugh = !player_alive && g.outfit == 0;
        // HP regeneration (moveMain): 30 frames after the last hit, +0.0008 x min(frames past that, 300) a frame, to the max
        if !matches!(g.state, State::Dead | State::Split) {
            g.since_hit += 1.;
            if g.since_hit > 30. && g.hp < g.max_hp {
                g.regen += 0.0008 * (g.since_hit - 30.).min(300.);
                while g.regen >= 1. { g.regen -= 1.; g.hp = (g.hp + 1).min(g.max_hp); }
            }
        }
        // a pending fright: the startle after its delay (chkEvadeFear), facing the gore
        if g.fear_delay > 0 {
            g.fear_delay -= 1;
            if g.fear_delay == 0 && matches!(g.state, State::Wait | State::Siege(_) | State::Laugh | State::Spotted) {
                s.release(&g);
                g.yaw = g.flee_yaw + std::f32::consts::PI;
                let k = s.roll(2) as usize;
                s.set(&mut g, State::Fear(0), ["em0110_0016.mot", "em0110_0017.mot"][k]);
            }
        }
        if g.angry > 0. { g.angry -= 1.; }
        if g.provoke > 0 {
            g.provoke -= 1;
            if g.provoke == 0 && matches!(g.state, State::Wait | State::Spotted | State::Siege(_) | State::Laugh | State::Guard) && player_alive {
                // target_found (em0110_0036), which ends in setAngry
                s.release(&g);
                g.provoked = true;
                s.set(&mut g, State::Spotted, "em0110_0036.mot");
            }
        }
        if g.dodge_delay > 0 {
            g.dodge_delay -= 1;
            if g.dodge_delay == 0 && matches!(g.state, State::Wait | State::Siege(_) | State::Laugh | State::Spotted) {
                // chkEvade's dodge by dot(own forward, escape dir); travel from the clips' root tracks
                let d = g.dodge_dir;
                let ahead = to.normalize_or_zero().dot(fwd) > 0.17;
                let (clip, travel) = if d.dot(fwd) > 0.707 { ("em0110_0012.mot", d * (3.3 / 21.)) }
                    else if d.dot(fwd) < -0.707 { if ahead { ("em0101_0121.mot", d * (2.3 / 16.)) } else { ("em0110_0011.mot", d * (0.9 / 61.)) } }
                    else if (Quat::from_rotation_y(-g.yaw) * d).x > 0. { ("em0110_002c.mot", d * (2.4 / 21.)) }
                    else { ("em0110_002b.mot", d * (2.9 / 24.)) };
                s.release(&g);
                g.knock = travel;
                s.set(&mut g, State::Sidestep, clip);
            }
        }
        // flying (launched or thrown), his head meets a spike wall or a hook: he sticks (motion_stuck 0x46, WALL_CRASH 20)
        if matches!(g.state, State::Launched | State::Thrown(0) | State::Thrown(1)) && s.has("em0110_101c.mot") {
            if let Some(at) = super::props::impale_at(g.pos + Vec3::Y * 1.5) {
                s.release(&g);
                g.hp -= 20; g.since_hit = 0.; g.knock = Vec3::ZERO; g.vy = 0.; g.timer = 50.;
                fx.burst("core", 0x1A5, at, g.yaw, 1.); fx.burst("core", 0x1A9, at, g.yaw, 1.); fx.gore(at, g.yaw, 30, true, false);
                if s.held == Some(g.id) { s.held = None; }
                s.cry(&g, false);
                s.set(&mut g, State::Stuck(0), "em0110_101c.mot");
                if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] grunt {} impaled at {:.2} (hp {})", g.id, at, g.hp); }
            }
        }
        // [added, town] the player walking into a civilian going about his business knocks him aside ("watch it!")
        let moving = Vec2::new(pl.x - play.prev_pos.x, pl.z - play.prev_pos.z).length() > 0.02;
        let mut bumped = false;
        let state_now = g.state;
        let near_y = (pl.y - g.pos.y).abs() < 1.;
        if let Some(c) = g.civ.as_mut() {
            c.bump = (c.bump - 1.).max(0.);
            c.scared = (c.scared - 1.).max(0.);
            if let State::Civ(act @ (CivAct::Walk | CivAct::Idle | CivAct::Chat)) = state_now {
                if dist < 0.95 && moving && c.bump <= 0. && player_alive && near_y { c.bump = 150.; c.resume = act; bumped = true; }
            }
        }
        if bumped {
            g.knock = -to.normalize_or(Vec3::Z) * 0.09;
            g.yaw = to.x.atan2(to.z);
            s.set(&mut g, State::Civ(CivAct::Bumped), "em0110_1075.mot");
            s.civ_events.push(CivEvent::Bumped(g.id));
        }
        match g.state {
            // the editor holds a civilian standing, turned to the player
            State::Civ(_) if frozen => {
                if g.clip != s.clip(civ_clips()[0]) { s.set(&mut g, State::Civ(CivAct::Idle), civ_clips()[0]); }
                if ended { g.frame = 0.; }
                face(&mut g, 0.1);
            }
            State::Civ(act) => { step = civ_step(s, &mut g, act, ended, to, dist, &arena); }
            // [added, town] a substory's zako: threats and laughs at his victim, now and then a shove (her flinch)
            State::Bully => {
                if ended {
                    let r = s.roll(100);
                    if r < 18 && s.has("em0101_0104.mot") { s.set(&mut g, State::Bully, "em0101_0104.mot"); g.timer = 1.; }
                    else if r < 45 { let k = s.roll(tables().laughs.len() as u32) as usize; s.set(&mut g, State::Bully, &tables().laughs[k].clone()); g.timer = 0.; }
                    else { let k = s.roll(tables().threats.len() as u32) as usize; s.set(&mut g, State::Bully, &tables().threats[k].clone()); g.timer = 0.; }
                }
                // the shove lands: the victim of the same tag flinches
                if g.timer > 0. && g.frame >= 22. {
                    g.timer = 0.;
                    let (tag, at) = (g.tag, g.pos);
                    let flinch = s.clip("em0110_1001.mot");
                    if let Some(v) = s.grunts.iter_mut().find(|v| v.tag == tag && v.state == State::Civ(CivAct::Victim) && v.pos.distance(at) < 3.) {
                        v.clip = flinch; v.frame = 0.;
                    }
                }
            }
            // fainted: reeling on the spot until 気絶回復 runs out
            State::Faint => { step = g.knock; g.knock *= 0.8; if ended { g.frame = 0.; } g.timer -= 1.; if g.timer <= 0. { to_hold(s, &mut g, dist, attacker); } }
            // a half: the cut at its frame (vertical 15 / waist > 17), then it lies where its clip leaves it
            State::Split => {
                let mut c = g.split.unwrap();
                if !c.done && ((!c.waist && g.frame >= 15.) || (c.waist && g.frame > 17.)) {
                    c.done = true;
                    let fwd = Quat::from_rotation_y(g.yaw);
                    if !c.b {
                        if c.waist {
                            // cutWaist: 417 + 416 at the waist ring
                            let w = g.pos + Vec3::Y * 1.3;
                            fx.burst("core", 417, w, g.yaw, 1.); fx.burst("core", 416, w, g.yaw, 1.); fx.burst("core", 489, w, g.yaw, 1.);
                            fx.gore(w, g.yaw, 100, true, true);
                        } else {
                            // down the middle: blood along the seam, head to groin
                            for k in 0..4 { let at = g.pos + fwd * Vec3::new(0., 0.9 + 0.35 * k as f32, 0.05); fx.burst("core", 1005, at, g.yaw, 1.); }
                            fx.burst("core", 489, g.pos + Vec3::Y * 1.2, g.yaw, 1.); fx.burst("core", 543, g.pos + Vec3::Y * 1.7, g.yaw, 1.);
                            fx.gore(g.pos + Vec3::Y * 1.3, g.yaw, 100, true, true);
                        }
                    }
                    // the open cut spurts (as the neck stump)
                    g.bleed = BLEED;
                }
                g.split = Some(c);
                if ended { g.timer = 0.; g.knock = Vec3::ZERO; g.state = State::Dead; g.frame = len - 1.; }
            }
            // impaled: stuck, then he slides off and lands, alive or dead
            State::Stuck(0) => { if ended { g.frame = 0.; } g.timer -= 1.; if g.timer <= 0. { s.set(&mut g, State::Stuck(1), "em0110_101d.mot"); } }
            State::Stuck(1) => { if ended { let dead = g.hp <= 0; s.set(&mut g, State::Stuck(2), if dead { "em0110_101f.mot" } else { "em0110_101e.mot" }); } }
            State::Stuck(_) => {
                if ended {
                    if g.hp <= 0 { s.kills += 1; g.timer = 0.; s.cry(&g, true); s.set(&mut g, State::Dead, "em0110_101f.mot"); g.frame = s.frames(g.clip) - 1.; }
                    else { g.timer = s.roll(31) as f32; s.set(&mut g, State::Down, "em0110_1038.mot"); }
                }
            }
            // the editor holds him: his idle, turned to the player
            State::Wait | State::Spotted | State::Siege(_) | State::Laugh | State::Provoke(_) if frozen => {
                if g.state != State::Wait || g.clip != s.clip("em0101_0000.mot") { s.set(&mut g, State::Wait, "em0101_0000.mot"); }
                if ended { g.frame = 0.; }
                face(&mut g, 0.1);
            }
            State::Editor => { if ended { if s.ed_loop { g.frame = 0.; } else if g.civ.is_some() { g.timer = 1e9; s.set(&mut g, State::Civ(CivAct::Idle), civ_clips()[0]); } else { s.set(&mut g, State::Wait, "em0101_0000.mot"); } } }
            State::Wait => {
                if ended { g.frame = 0.; }
                if dist < tables().sense && player_alive {
                    if s.has("em0110_0036.mot") { s.set(&mut g, State::Spotted, "em0110_0036.mot"); } else { join(s, &mut g); to_hold(s, &mut g, dist, attacker); }
                }
            }
            // target_found: the "spotted" reaction, facing him, then into the siege
            State::Spotted => {
                face(&mut g, 0.15);
                if ended {
                    join(s, &mut g);
                    if g.provoked {
                        // setAngry(13-16 s), then provoke_reaction: run in to a stop radius of rand(2.8, 3.5)²
                        g.provoked = false; g.angry = 390. + s.roll(91) as f32;
                        let r = |s: &mut Street| 2.8 + s.roll(701) as f32 / 1000.;
                        g.hold = r(s) * r(s);
                        s.set(&mut g, State::Provoke(true), "em0110_0012.mot");
                    } else { to_hold(s, &mut g, dist, attacker); }
                }
            }
            State::Provoke(run) => {
                face(&mut g, 1.);
                if g.angry <= 0. || !player_alive { to_hold(s, &mut g, dist, false); }
                else if run {
                    if ended { g.frame = 0.; }
                    step = Quat::from_rotation_y(g.yaw) * Vec3::Z * RUN;
                    if dist * dist <= g.hold {
                        let r = |s: &mut Street| 4.5 + s.roll(501) as f32 / 1000.;
                        g.hold = r(s) * r(s);
                        let k = s.roll(3) as usize;
                        s.set(&mut g, State::Provoke(false), &tables().threats[k % tables().threats.len()]);
                    }
                } else {
                    // angry: the attack chance +50 % a frame (52 % for em0101), without the token
                    let pool: &[usize] = &tables().pools[g.outfit.min(1)];
                    let pick = pool.iter().copied().find(|&k| dist <= atk(k).range && off <= atk(k).yaw && s.has(&atk(k).clip));
                    if let Some(k) = pick.filter(|_| s.roll(100) < pr.near + 50) { start_attack(s, &mut g, k); }
                    else if ended {
                        if dist * dist <= g.hold { let k = s.roll(3) as usize; s.set(&mut g, State::Provoke(false), &tables().threats[k % tables().threats.len()]); }
                        else { let r = |s: &mut Street| 2.8 + s.roll(701) as f32 / 1000.; g.hold = r(s) * r(s); s.set(&mut g, State::Provoke(true), "em0110_0012.mot"); }
                    }
                }
            }
            // face_guard: hold the guard pose for the guard time; the player stopping (25 frames) or moving off (3.5 m)
            // ends it at once [inferred: the player's attack state stands in for the last startAtk]
            State::Guard => {
                face(&mut g, 0.3);
                if ended { g.frame = len - 1.; }
                g.timer -= 1.; g.idle += 1.;
                if play.fight.player.state() == 39 { g.idle = 0.; }
                if g.timer <= 0. {
                    // catched_counter: the hop back (em0110_1075, about 0.8 m back over 41 frames)
                    g.guards = 2; g.knock = -to.normalize_or_zero() * (0.8 / 41.);
                    s.set(&mut g, State::Sidestep, "em0110_1075.mot");
                } else if g.idle > 25. || dist > 3.5 { g.guards = 2; to_hold(s, &mut g, dist, false); }
            }
            State::Sidestep => { step = g.knock; if ended { g.knock = Vec3::ZERO; to_hold(s, &mut g, dist, false); } }
            State::Siege(sub) => {
                if laugh {
                    let k = s.roll(tables().laughs.len() as u32) as usize;
                    g.timer = 90. + s.roll(31) as f32;
                    s.set(&mut g, State::Laugh, &tables().laughs[k]);
                } else { match sub {
                    Sub::Hold if g.angry > 0. && player_alive => {
                        let r = |s: &mut Street| 2.8 + s.roll(701) as f32 / 1000.;
                        g.hold = r(s) * r(s);
                        s.set(&mut g, State::Provoke(true), "em0110_0012.mot");
                    }
                    Sub::Hold => {
                        face(&mut g, 0.1);
                        g.hold -= 1.; g.idle += 1.;
                        if ended { to_hold(s, &mut g, dist, attacker); }
                        if attacker && player_alive {
                            s.set(&mut g, State::Siege(Sub::Approach), "em0101_0010.mot");
                        } else if to_slot.length() > SIEGE_RETURN {
                            s.set(&mut g, State::Siege(Sub::Return), if to_slot.length() > 6. { "em0110_0012.mot" } else { "em0101_0010.mot" });
                        } else if dist < 1.5 {
                            // back off: a back-jump on a 1-in-3 roll, else the walk back
                            if s.roll(3) == 0 { g.knock = -to.normalize_or_zero() * 0.12; s.set(&mut g, State::Siege(Sub::Back), "em0101_0121.mot"); }
                            else { s.set(&mut g, State::Siege(Sub::Back), "em0110_0011.mot"); }
                        } else if g.hold <= 0. && g.idle > 90. && s.roll(16) == 0 && to_slot.length() > 1.5 {
                            // the idle reshuffle: side-walk (R em0101_0011 / L 0012), forward or back toward the slot
                            let local = Quat::from_rotation_y(-g.yaw) * to_slot;
                            let clip = if local.x.abs() > local.z.abs() { if local.x > 0. { "em0101_0012.mot" } else { "em0101_0011.mot" } }
                                       else if local.z > 0. { "em0101_0010.mot" } else { "em0110_0011.mot" };
                            g.timer = 0.;
                            s.set(&mut g, State::Siege(Sub::Step), clip);
                        }
                    }
                    Sub::Return => {
                        if ended { g.frame = 0.; }
                        if attacker { s.set(&mut g, State::Siege(Sub::Approach), "em0101_0010.mot"); }
                        else {
                            let want = to_slot.x.atan2(to_slot.z);
                            g.yaw = mw_sim::player::rad_lerp_range(g.yaw, want, 0.25, 0.3);
                            let run = to_slot.length() > 6.;
                            step = Quat::from_rotation_y(g.yaw) * Vec3::Z * if run { RUN } else { WALK };
                            if to_slot.length() < 2. { to_hold(s, &mut g, dist, attacker); }
                        }
                    }
                    Sub::Step => {
                        face(&mut g, 0.1);
                        if ended { g.frame = 0.; }
                        g.timer += 1.;
                        step = to_slot.normalize_or_zero() * SIDE;
                        if to_slot.length() < 0.6 || g.timer > 60. || attacker { to_hold(s, &mut g, dist, attacker); }
                    }
                    Sub::Back => {
                        face(&mut g, 0.1);
                        if s.clips.get("em0101_0121.mot") == Some(&g.clip) { step = g.knock; g.knock *= 0.85; } else { step = -to.normalize_or_zero() * SIDE; }
                        if ended { to_hold(s, &mut g, dist, attacker); }
                    }
                    // the attacker's approach (siege 0x1c): the far dash window, walk / run in, then the per-frame roll
                    Sub::Approach => {
                        // only the token holder, and only at a living target
                        if !attacker || !player_alive { to_hold(s, &mut g, dist, attacker); }
                        else {
                            face(&mut g, 0.25);
                            if ended { g.frame = 0.; }
                            if (3.5..5.).contains(&dist) && s.roll(100) < pr.far && off < 15. {
                                let k = if g.outfit == 1 { tables().fire } else { tables().dash };
                                if s.has(&atk(k).clip) { start_attack(s, &mut g, k); s.dashes.push((g.id, g.pos, to.normalize_or_zero())); }
                            } else if dist > STOP && !(g.clip == s.clip("em0101_0000.mot") && dist <= STOP + 0.6) {
                                let run = dist > 4.5 * STOP;
                                let clip = if run { "em0110_0012.mot" } else { "em0101_0010.mot" };
                                if s.clips.get(clip) != Some(&g.clip) {
                                    if run { s.dashes.push((g.id, g.pos, to.normalize_or_zero())); }
                                    s.set(&mut g, State::Siege(Sub::Approach), clip);
                                }
                                step = Quat::from_rotation_y(g.yaw) * Vec3::Z * if run { RUN } else { WALK };
                            } else {
                                if s.clips.get("em0101_0000.mot") != Some(&g.clip) { s.set(&mut g, State::Siege(Sub::Approach), "em0101_0000.mot"); }
                                if std::env::var_os("MW_STREET_AI_LOG").is_some() { eprintln!("[ai] tick {} grunt {} close: dist {:.2} off {:.0} clip {}", play.tick, g.id, dist, off, s.bank.as_ref().unwrap().clips[g.clip].name); }
                                let rate = if s.held.is_some() { if g.outfit == 1 { 100 } else { 1 } } else { pr.near };
                                if s.roll(100) < rate {
                                    // _decideAttackSub: in range and yaw, the best of 0.05 x power + 0.2 x dist² / range² + rand(0, 0.2)
                                    let pool: &[usize] = &tables().pools[g.outfit.min(1)];
                                    let mut best: Option<(usize, f32)> = None;
                                    for &k in pool {
                                        let a = atk(k);
                                        if dist > a.range || off > a.yaw { continue; }
                                        let score = 0.05 * tables().power as f32 + 0.2 * dist * dist / (a.range * a.range) + s.roll(1000) as f32 / 5000.;
                                        if best.is_none_or(|b| score > b.1) { best = Some((k, score)); }
                                    }
                                    if let Some((k, _)) = best { start_attack(s, &mut g, k); }
                                }
                            }
                        }
                    }
                } }
            }
            State::Attack(k) => {
                let a = atk(k);
                if g.frame < 8. { face(&mut g, 0.25); }
                let open = a.windows.iter().position(|w| (w[0]..w[1]).contains(&g.frame));
                if let Some(w) = open {
                    // the breath is forced from the far window (3.5-5 m, siege 0x1c) and carries that far
                    let reach = if k == tables().fire { 5.3 } else { a.range + 0.3 };
                    if g.landed & (1 << w) == 0 && dist < reach && to.normalize_or_zero().dot(fwd) > 0.5 {
                        // power 14 x 攻撃力 (em0101 0.5, em0101B 1.5)
                        let damage = (tables().attacks[k].power.unwrap_or(tables().power) as f32 * pr.power).round() as i32;
                        let p = &mut *play;
                        if p.fight.enemy_hits_player(&p.fd, if a.atk != 0 { a.atk } else { 0x173 }, damage, g.pos.to_array()) {
                            g.landed |= 1 << w;
                            if k == tables().fire { s.burn = s.burn.max(66 + s.roll(25)); if std::env::var_os("MW_STREET_LOG").is_some() { eprintln!("[street] tick {} burning {} frames (breath from {:.2}, yaw {:.2})", play.tick, s.burn, g.pos, g.yaw); } }
                            // the claws rake him where they land: his chest, on the side facing the grunt
                            else if g.outfit == 0 { let n = (-to).normalize_or(Vec3::Z); fx.claw(pl + Vec3::Y * (1.05 + 0.25 * (w as f32)) + n * 0.2, n); }
                        }
                    }
                }
                // chkComboContinue: at the combo window, 0104 -> 0105 -> 0106 while the count is under its max and he is in reach
                if let Some(cf) = a.combo {
                    if g.frame >= cf && g.frame - 1. < cf && g.combo.0 < g.combo.1 {
                        let next = k + 1;
                        if next < tables().attacks.len() && dist <= atk(next).range && off <= atk(next).yaw { g.combo.0 += 1; s.set(&mut g, State::Attack(next), &atk(next).clip); }
                    }
                }
                if ended && g.state == State::Attack(k) { g.attacks += 1; s.release(&g); to_hold(s, &mut g, dist, false); }
            }
            State::Damage => { step = g.knock; g.knock *= 0.8; if ended { to_hold(s, &mut g, dist, attacker); } }
            State::Dodge => { step = g.knock; g.knock *= 0.85; if ended { to_hold(s, &mut g, dist, attacker); } }
            State::Swept => { step = g.knock; g.knock *= 0.85; if ended { g.timer = s.roll(31) as f32; s.set(&mut g, State::Down, "em0110_1038.mot"); } }
            // held on his first frame while Kreely's grab reaches (mw_sim::fight::Suplex::grab), then thrown
            State::Suplexed => { if play.fight.suplex.as_ref().is_some_and(|x| x.grab.is_some()) { g.frame = 0.; } else if ended { g.frame = len - 1.; } }
            // motion_blow: the flight, then the landing (the down clip of the pair), then down
            State::Launched => {
                step = g.knock; g.knock *= 0.9;
                if ended { let land = if s.clips.get("em0110_1091.mot") == Some(&g.clip) { "em0110_1096.mot" } else { "em0110_1095.mot" }; s.set(&mut g, State::Land, land); }
            }
            State::Land => { step = g.knock; g.knock *= 0.85; if ended { g.timer = s.roll(31) as f32; s.set(&mut g, State::Down, "em0110_1038.mot"); } }
            // fear_to_escape: the startle; on 怖逃げ (80 %) turn and run away 33-54 frames, then the stop clip
            State::Fear(0) => {
                if ended {
                    if s.roll(100) < 80 {
                        g.timer = 33. + s.roll(22) as f32;
                        let k = s.roll(2) as usize;
                        s.set(&mut g, State::Fear(1), ["em0110_0018.mot", "em0110_0015.mot"][k]);
                        // his flee line (fear_to_escape 8102D454, cEnemyBase::playVoice 0x1002)
                        if g.body == 0 { play.fight.events.push(mw_sim::fight::FightEvent::Voice(0x1002)); } else { s.cry(&g, false); }
                    } else { to_hold(s, &mut g, dist, false); }
                }
            }
            State::Fear(1) => {
                if ended { g.frame = 0.; }
                g.yaw = mw_sim::player::rad_lerp_range(g.yaw, g.flee_yaw, 1., 1.);
                step = Quat::from_rotation_y(g.yaw) * Vec3::Z * RUN;
                g.timer -= 1.;
                // cornered (fear_to_escape subs 0x10-0x13): a wall within a few steps stops the run; he cowers (1109), shakes
                // there (110b) and stands up again (103a)
                let probe = step * 6.;
                let (moved, _) = mw_sim::collision::slide(&arena.0, g.pos.to_array(), [probe.x, probe.z]);
                if Vec2::new(moved[0], moved[1]).length() < probe.length() * 0.4 && s.has("em0110_1109.mot") { step = Vec3::ZERO; g.timer = 60.; s.set(&mut g, State::Fear(3), "em0110_1109.mot"); }
                else if g.timer <= 0. { let k = s.roll(2) as usize; s.set(&mut g, State::Fear(2), ["em0110_1512.mot", "em0110_0030.mot"][k]); }
            }
            State::Fear(3) => { if ended { let t = 60.; s.set(&mut g, State::Fear(4), "em0110_110b.mot"); g.timer = t; } }
            State::Fear(4) => { if ended { g.frame = 0.; } g.timer -= 1.; if g.timer <= 0. { s.set(&mut g, State::Fear(5), "em0110_103a.mot"); } }
            State::Fear(_) => { if ended { to_hold(s, &mut g, dist, false); } }
            // down 0-1 s (ダウン体力), then the neckspring above 60 % HP, else the stand-up
            State::Down => {
                // (back to the lying loop after a blow taken down there)
                if ended { if s.has("em0110_1038.mot") && g.clip != s.clip("em0110_1038.mot") { g.clip = s.clip("em0110_1038.mot"); } g.frame = 0.; }
                g.timer -= 1.;
                if g.timer <= 0. {
                    let up = if g.hp * 100 > g.max_hp * 60 { "em0110_1060.mot" } else { "em0110_103d.mot" };
                    s.set(&mut g, State::GetUp, up);
                }
            }
            State::GetUp => { if ended { to_hold(s, &mut g, dist, attacker); } }
            State::Piyori => { if ended { g.frame = 0.; } g.timer -= 1.; if g.timer <= 0. { to_hold(s, &mut g, dist, attacker); } }
            State::Laugh => { face(&mut g, 0.1); if ended { g.frame = 0.; } g.timer -= 1.; if g.timer <= 0. || player_alive { to_hold(s, &mut g, dist, attacker); } }
            // catched_start -> catched_idle; the hold time, then the struggle, then he shakes free
            State::Held(struggling) => {
                let loop_clip = if struggling { "em0110_1077.mot" } else { "em0110_1072.mot" };
                if ended { if s.clips.get(loop_clip) != Some(&g.clip) { s.set(&mut g, State::Held(struggling), loop_clip); } else { g.frame = 0.; } }
                g.timer -= 1.;
                if g.timer <= 0. {
                    if !struggling {
                        // 掴まれもがき体力: he struggles for 5 s before he breaks free
                        g.timer = 150.;
                        s.set(&mut g, State::Held(true), "em0110_1076.mot");
                    } else {
                        // catched_shake_off: free, 0.8 m back (1075)
                        s.held = None; s.carry_lost = true;
                        s.set(&mut g, State::Escape, "em0110_1075.mot");
                    }
                }
            }
            // reeling from the head-butt (catched_atk_start 1079), then held again (the hold timer runs on)
            State::Headbutted => { if ended { g.state = State::Held(false); g.clip = s.clips["em0110_1072.mot"]; g.frame = 0.; } }
            State::Released | State::Escape => { if ended { to_hold(s, &mut g, dist, false); } }
            // the throw: the start clip (the flight), the air loop, the landing; the flying body knocks others down
            State::Thrown(phase) => {
                if phase < 2 {
                    // where the body is now: his position plus the clip's root travel so far
                    let bank = s.bank.as_ref().unwrap();
                    let r = mw_sim::sample(&bank.bones, &bank.clips[g.clip], g.frame.min(len - 1.)).last().map(|p| p.translation).unwrap_or([0.; 3]);
                    let body = g.pos + Quat::from_rotation_y(g.yaw) * Vec3::new(r[0], 0., r[2]);
                    let dir = Vec3::new(play.fight.player.loco.facing.sin(), 0., play.fight.player.loco.facing.cos());
                    let hit: Vec<usize> = s.grunts.iter().enumerate()
                        .filter(|(_, o)| o.id != 0 && !s.blown.contains(&o.id) && matches!(o.state, State::Wait | State::Spotted | State::Siege(_) | State::Attack(_) | State::Damage | State::Laugh | State::Piyori | State::Civ(_) | State::Bully))
                        .filter(|(_, o)| Vec2::new(o.pos.x - body.x, o.pos.z - body.z).length() < 1.0).map(|(k, _)| k).collect();
                    for k in hit {
                        // BLOW: knocked down along the throw
                        s.blown.push(s.grunts[k].id);
                        let at = s.grunts[k].pos + Vec3::Y * 1.0;
                        s.damage(k, 10, dir, true, 0x15B, at, &mut fx);
                        let o = &mut s.grunts[k];
                        if o.state != State::Dead { o.knock = dir * 0.16; o.state = State::Launched; o.clip = s.clips["em0110_1090.mot"]; o.frame = 0.; }
                    }
                }
                if ended {
                    // the clip's final root travel becomes his position; then the next clip of the throw
                    let bank = s.bank.as_ref().unwrap();
                    let r = mw_sim::sample(&bank.bones, &bank.clips[g.clip], len - 1.).last().map(|p| p.translation).unwrap_or([0.; 3]);
                    g.pos += Quat::from_rotation_y(g.yaw) * Vec3::new(r[0], 0., r[2]);
                    let names = if s.throw_side { ["em0110_1014.mot", "em0110_1015.mot"] } else { ["em0110_100d.mot", "em0110_100e.mot"] };
                    match phase {
                        0 => { g.state = State::Thrown(1); g.clip = s.clips[names[0]]; g.frame = 0.; }
                        1 => {
                            g.state = State::Thrown(2); g.clip = s.clips[names[1]]; g.frame = 0.;
                            // the landing: WHANG_THROW / WHANG_SIDE_THROW 30 (setOwnDamage)
                            let at = g.pos + Vec3::Y * 0.5;
                            let push = Quat::from_rotation_y(g.yaw) * Vec3::Z;
                            g.hp -= 30; g.since_hit = 0.;
                            fx.impact(at, push.x.atan2(push.z), 1., [1., 0.12, 0.08]); fx.burst("core", 1005, at, push.x.atan2(push.z), 1.);
                            if g.hp <= 0 { g.timer = 0.; s.release(&g); s.set(&mut g, State::Dead, "em0110_1095.mot"); }
                        }
                        _ => { g.timer = s.roll(31) as f32; s.set(&mut g, State::Down, "em0110_1038.mot"); }
                    }
                }
            }
            State::Dead => {
                step = g.knock; g.knock *= 0.88;
                if ended { g.frame = len - 1.; }
                g.timer += 1.;
                g.jolt *= 0.8;
                g.fade = (1. - (g.timer - CORPSE) / FADE).clamp(0., 1.);
            }
        }
        // the stump spurts, weaker as it goes
        if g.bleed > 0 {
            g.bleed -= 1;
            if let Some(neck) = g.neck.and_then(|n| globals.get(n).ok()) {
                let every = if g.bleed > BLEED / 2 { 2 } else { 4 };
                if g.bleed % every == 0 { fx.burst("core", 1005, neck.translation() + Vec3::Y * 0.08, g.yaw, 1.); }
            }
        }
        // stay out of the player and the other grunts
        if !matches!(g.state, State::Dead | State::Suplexed | State::Held(_) | State::Headbutted | State::Thrown(_) | State::Split) {
            if dist < 0.9 && dist > 1e-3 { step -= to / dist * (0.9 - dist) * 0.5; }
            for (j, o) in s.grunts.iter().enumerate() {
                if j == i || o.state == State::Dead { continue; }
                let d = Vec3::new(g.pos.x - o.pos.x, 0., g.pos.z - o.pos.z); let l = d.length();
                if l < 0.8 && l > 1e-3 { step += d / l * (0.8 - l) * 0.3; }
            }
        }
        if step != Vec3::ZERO {
            let (moved, _) = mw_sim::collision::slide(&arena.0, g.pos.to_array(), [step.x, step.z]);
            g.pos.x += moved[0]; g.pos.z += moved[1];
        }
        // gravity: off a ledge or a step he falls to the floor under him (bodies too: no corpse hangs in the air); a rise of
        // under half a metre is stepped up
        if !matches!(g.state, State::Suplexed | State::Held(_) | State::Headbutted | State::Stuck(0) | State::Stuck(1) | State::Split) && g.split.is_none() {
            if let Some(y) = super::floor(&arena.0, g.pos + Vec3::Y * 0.5) {
                if g.pos.y > y + 0.01 { g.vy -= GRAVITY; g.pos.y = (g.pos.y + g.vy).max(y); if g.pos.y <= y { g.vy = 0.; } }
                else { g.pos.y = y; g.vy = 0.; }
            }
        }
        s.grunts[i] = g;
    }
    // burning: 1 HP every 10 frames, never the last (the native fire state on the player is not decoded)
    if s.burn > 0 {
        s.burn = if play.fight.player.hp > 0 { s.burn - 1 } else { 0 };
        if s.burn % 10 == 0 && play.fight.player.hp > 1 { play.fight.player.hp -= 1; }
    }
    // the dead are cleared after a while
    let mut kills = 0;
    s.grunts.retain(|g| {
        let gone = g.state == State::Dead && g.timer > CORPSE + FADE;
        if gone { kills += 1; for r in g.roots { commands.entity(r).despawn(); } }
        !gone
    });
    s.kills += kills;
    if std::env::var_os("MW_STREET_LOG").is_some() && play.tick % 30 == 0 {
        eprintln!("[street] tick {} player hp {} kills {} grunts {:?}", play.tick, play.fight.player.hp, s.kills,
                  s.grunts.iter().map(|g| (format!("{:?}", g.state), g.hp, (g.pos.distance(pl) * 10.).round() / 10.)).collect::<Vec<_>>());
    }
}

/// [added, town] Put a civilian into `act`: its clip and a fresh path or timer.
fn civ_enter(s: &mut Street, g: &mut Grunt, act: CivAct) {
    match act {
        CivAct::Chat => { let f = g.civ.as_ref().map_or(g.yaw, |c| c.face); g.yaw = f; s.set(g, State::Civ(CivAct::Chat), civ_clips()[0]); }
        CivAct::Idle => { g.timer = 90. + s.roll(220) as f32; s.set(g, State::Civ(CivAct::Idle), civ_clips()[0]); }
        CivAct::Victim => s.set(g, State::Civ(CivAct::Victim), "em0110_110b.mot"),
        _ => { if let Some(c) = g.civ.as_mut() { c.path.clear(); } s.set(g, State::Civ(CivAct::Walk), civ_clips()[1]); }
    }
}

/// [added, town] A path for a civilian: a stroll to somewhere 5-30 m off, or (fleeing) away from what scared him.
fn civ_plan(s: &mut Street, g: &Grunt, run: bool) -> Option<Vec<Vec3>> {
    let nav = s.nav.clone()?;
    let threat = g.civ.as_ref().map_or(g.pos, |c| c.threat);
    if run {
        let away = Vec3::new(g.pos.x - threat.x, 0., g.pos.z - threat.z).normalize_or(Quat::from_rotation_y(g.yaw) * Vec3::Z);
        let mut best: Option<(Vec3, f32)> = None;
        for _ in 0..14 {
            let p = nav.random(s.roll(1 << 24));
            let d = p.distance(g.pos);
            if !(6. ..40.).contains(&d) { continue; }
            let score = p.distance(threat) + (p - g.pos).normalize_or_zero().dot(away) * 8.;
            if best.is_none_or(|b| score > b.1) { best = Some((p, score)); }
        }
        return nav.path(g.pos, best?.0);
    }
    for _ in 0..6 {
        let p = nav.random(s.roll(1 << 24));
        if (5. ..30.).contains(&p.distance(g.pos)) { if let Some(path) = nav.path(g.pos, p) { return Some(path); } }
    }
    None
}

/// [added, town] One game frame of a civilian's own behaviour; returns his step.
fn civ_step(s: &mut Street, g: &mut Grunt, act: CivAct, ended: bool, to: Vec3, dist: f32, _arena: &super::Arena) -> Vec3 {
    let mut step = Vec3::ZERO;
    fn turn(g: &mut Grunt, d: Vec3, k: f32) { if d.length_squared() > 1e-6 { g.yaw = mw_sim::player::rad_lerp_range(g.yaw, d.x.atan2(d.z), k, 0.3); } }
    match act {
        CivAct::Walk | CivAct::Flee => {
            let run = act == CivAct::Flee;
            let want = if run { civ_clips()[2] } else { civ_clips()[1] };
            if s.clips.get(want) != Some(&g.clip) { let f = g.frame; s.set(g, State::Civ(act), want); g.frame = f % s.frames(g.clip).max(1.); }
            if ended { g.frame = 0.; }
            if g.civ.as_ref().unwrap().path.is_empty() {
                match civ_plan(s, g, run) {
                    Some(p) if !p.is_empty() => { let c = g.civ.as_mut().unwrap(); c.path = p; c.check = (0., g.pos); }
                    _ => {
                        if run { g.timer = 90.; s.set(g, State::Civ(CivAct::Cower), "em0110_1109.mot"); } else { civ_enter(s, g, CivAct::Idle); }
                        return step;
                    }
                }
            }
            let target = g.civ.as_ref().unwrap().path[0];
            let d = Vec3::new(target.x - g.pos.x, 0., target.z - g.pos.z);
            if d.length() < if run { 0.8 } else { 0.5 } {
                let c = g.civ.as_mut().unwrap();
                c.path.remove(0);
                if c.path.is_empty() {
                    let calm = c.scared <= 0.;
                    if run {
                        // out of danger: a moment's cowering if the player is still near, else back to the stroll
                        if calm { if dist < 7. { g.timer = 90.; s.set(g, State::Civ(CivAct::Cower), "em0110_1109.mot"); } else { civ_enter(s, g, CivAct::Walk); } }
                    } else if s.roll(100) < 45 { civ_enter(s, g, CivAct::Idle); }
                }
                return step;
            }
            turn(g, d, if run { 0.35 } else { 0.16 });
            let fwd = Quat::from_rotation_y(g.yaw) * Vec3::Z;
            let along = fwd.dot(d.normalize()).max(0.15);
            let scale = s.civs.get(g.body as usize - 10).and_then(|c| c.as_ref()).map_or(1., |c| c.root_grunt);
            step = fwd * if run { CIV_RUN } else { CIV_WALK } * along * scale;
            // stuck behind something: plan again
            let c = g.civ.as_mut().unwrap();
            c.check.0 += 1.;
            if c.check.0 >= 45. { let moved = c.check.1.distance(g.pos); c.check = (0., g.pos); if moved < 0.35 { c.path.clear(); } }
        }
        CivAct::Idle => {
            if ended { g.frame = 0.; }
            g.timer -= 1.;
            if g.timer <= 0. { civ_enter(s, g, CivAct::Walk); }
        }
        CivAct::Chat => {
            let f = g.civ.as_ref().unwrap().face;
            turn(g, Quat::from_rotation_y(f) * Vec3::Z, 0.1);
            if ended {
                let k = s.roll(100);
                let clip = if k < 40 { civ_clips()[0] } else { CIV_CHAT[s.roll(CIV_CHAT.len() as u32) as usize] };
                s.set(g, State::Civ(CivAct::Chat), clip);
            }
        }
        CivAct::Bumped => {
            step = g.knock; g.knock *= 0.82;
            turn(g, to, 0.2);
            if ended { let r = g.civ.as_ref().unwrap().resume; civ_enter(s, g, r); }
        }
        CivAct::Startle => {
            turn(g, to, 0.05);
            if ended { let c = g.civ.as_mut().unwrap(); c.path.clear(); s.set(g, State::Civ(CivAct::Flee), civ_clips()[2]); }
        }
        CivAct::Cower => {
            let (crouch, shake) = (s.clip("em0110_1109.mot"), s.clip("em0110_110b.mot"));
            if g.clip == crouch { if ended { g.timer = 60. + s.roll(90) as f32; s.set(g, State::Civ(CivAct::Cower), "em0110_110b.mot"); } }
            else if g.clip == shake {
                if ended { g.frame = 0.; }
                g.timer -= 1.;
                if g.timer <= 0. && dist > 3.5 { s.set(g, State::Civ(CivAct::Cower), "em0110_103a.mot"); }
            } else if ended {
                if g.civ.as_ref().unwrap().scared > 0. { g.civ.as_mut().unwrap().path.clear(); s.set(g, State::Civ(CivAct::Flee), civ_clips()[2]); }
                else { civ_enter(s, g, CivAct::Walk); }
            }
        }
        CivAct::Victim => {
            let f = g.civ.as_ref().unwrap().face;
            turn(g, Quat::from_rotation_y(f) * Vec3::Z, 0.2);
            if ended { if g.clip != s.clip("em0110_110b.mot") { s.set(g, State::Civ(CivAct::Victim), "em0110_110b.mot"); } else { g.frame = 0.; } }
        }
    }
    step
}

/// [added, town] The colour look's clothes per civilian: each one's own tint over the parts that are not skin.
fn tint(mut street: ResMut<Street>, parts: Query<&MeshMaterial3d<StandardMaterial>>, mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    if !street.on { return; }
    let s = &mut *street;
    for g in &mut s.grunts {
        if g.tinted || g.body < 10 || g.parts[1].is_empty() { continue; }
        let Some(Some(rig)) = s.civs.get(g.body as usize - 10) else { g.tinted = true; continue; };
        // (half strength: a shift in hue over the palette, not a new colour)
        let t = CIV_TINTS[(g.id as usize * 7 + g.body as usize) % CIV_TINTS.len()].map(|c| 1. + (c - 1.) * 0.5);
        for &(i, e) in &g.parts[1] {
            if rig.skin.contains(&i) { continue; }
            let Ok(m) = parts.get(e) else { continue; };
            let Some(mut copy) = materials.get(&m.0).cloned() else { continue; };
            let c = copy.base_color.to_linear();
            copy.base_color = Color::linear_rgba(c.red * t[0], c.green * t[1], c.blue * t[2], c.alpha);
            commands.entity(e).insert(MeshMaterial3d(materials.add(copy)));
        }
        g.tinted = true;
    }
}

fn bind(mut street: ResMut<Street>, children: Query<&Children>, names: Query<&Name>, prims: Query<&ChildOf, With<Mesh3d>>, mut commands: Commands) {
    if !street.on { return; }
    let s = &mut *street;
    let Some(bank) = &s.bank else { return; };
    for g in &mut s.grunts {
        for k in 0..2 {
            if g.joints[k].is_none() {
                let found: std::collections::HashMap<String, Entity> = children.iter_descendants(g.roots[k]).filter_map(|d| names.get(d).ok().map(|n| (n.as_str().to_string(), d))).collect();
                // (a civilian lacks the grunt's weapon bones: those stand in as placeholders nothing poses)
                let civ = g.body >= 10;
                g.joints[k] = bank.bones.iter().map(|b| b.id).chain(std::iter::once(-1)).map(|id| found.get(&format!("role_0_bone{id}")).copied().or((civ && !found.is_empty() && id != 0).then_some(Entity::PLACEHOLDER))).collect();
                // a woman's breasts, buttocks and hair spring (play_springs.rs VAMP)
                if let (Some(js), true) = (&g.joints[k], (1..=3).contains(&g.body)) {
                    commands.entity(g.roots[k]).insert(super::springs::SpringRequest { bones: bank.bones.clone(), joints: js.clone(), actor: "vampzako" });
                }
            }
            if !g.parts[k].is_empty() { continue; }
            g.parts[k] = children.iter_descendants(g.roots[k]).filter_map(|d| {
                let index = prims.get(d).ok().and_then(|p| names.get(p.parent()).ok())?.as_str().strip_prefix("role_0_mesh_")?.parse().ok()?;
                Some((index, d))
            }).collect();
            // the cross-sections hide until a cut; the other outfit's pieces stay off
            let off: &[usize] = if g.outfit == 0 { &OUTFIT_B } else { &OUTFIT_A };
            // (a civilian has no cuts or outfits: only his head goes, if it went before his model loaded)
            let civ_head: Vec<usize> = if g.body >= 10 { s.civs.get(g.body as usize - 10).and_then(|c| c.as_ref()).map(|c| c.head.clone()).unwrap_or_default() } else { vec![] };
            for &(i, e) in &g.parts[k] {
                let hide = if g.body >= 10 { g.headless && civ_head.contains(&i) } else { CUTS.contains(&i) || off.contains(&i) || (g.headless && HEAD_PARTS.contains(&i)) };
                if hide { commands.entity(e).insert(Visibility::Hidden); }
            }
        }
    }
}

/// The skinned primitive as posed now, in world space (positions, normals).
fn skinned_world(mesh: &Mesh, skin: &SkinnedMesh, ibp: &SkinnedMeshInverseBindposes, globals: &Query<&GlobalTransform>) -> Option<(Vec<Vec3>, Vec<Vec3>)> {
    let mats: Vec<Mat4> = skin.joints.iter().zip(ibp.iter()).map(|(j, b)| globals.get(*j).map_or(Mat4::IDENTITY, |g| g.to_matrix()) * *b).collect();
    let (Some(VertexAttributeValues::Float32x3(pos)), Some(VertexAttributeValues::Uint16x4(ji)), Some(VertexAttributeValues::Float32x4(jw))) =
        (mesh.attribute(Mesh::ATTRIBUTE_POSITION), mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX), mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)) else { return None; };
    let nrm = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) { Some(VertexAttributeValues::Float32x3(n)) => n.clone(), _ => vec![[0., 1., 0.]; pos.len()] };
    let (mut p, mut n) = (Vec::with_capacity(pos.len()), Vec::with_capacity(pos.len()));
    for i in 0..pos.len() {
        let m: Mat4 = (0..4).map(|k| mats.get(ji[i][k] as usize).copied().unwrap_or(Mat4::IDENTITY) * jw[i][k]).fold(Mat4::ZERO, |a, b| a + b);
        p.push(m.transform_point3(Vec3::from_array(pos[i])));
        n.push(m.transform_vector3(Vec3::from_array(nrm[i])).normalize_or_zero());
    }
    Some((p, n))
}

/// The neck's cross-section: the triangles of `_00_DA` at the top of the torso (bind y > 1.78, |x| < 0.2).
fn neck_cap(src: &Mesh) -> Option<Mesh> {
    let Some(VertexAttributeValues::Float32x3(pos)) = src.attribute(Mesh::ATTRIBUTE_POSITION) else { return None; };
    let keep = |i: usize| pos[i][1] > 1.78 && pos[i][0].abs() < 0.2;
    let all: Vec<usize> = src.indices()?.iter().collect();
    let kept: Vec<u32> = all.chunks_exact(3).filter(|t| t.iter().all(|&i| keep(i))).flatten().map(|&i| i as u32).collect();
    if kept.is_empty() { return None; }
    let mut cap = src.clone();
    cap.insert_indices(Indices::U32(kept));
    Some(cap)
}

/// A half's copy of a primitive (the bind pose decides; the model is built with its seams on x = 0 and at the waist):
/// 0 the triangles with every vertex at x <= 0, 1 at x >= 0; 2 / 3 the waist pieces of `_00_DA` (the upper half's
/// bottom face y 1.29-1.40, the lower half's top face y 1.20-1.28); 4 the whole mesh with `remap` (bone 1's slot to
/// bone 15's: setChildSetudan, so the waistband follows the legs); 5 the whole mesh, both windings (a cut face).
fn half_mesh(src: &Mesh, how: u8, remap: Option<(u16, u16)>) -> Option<Mesh> {
    let Some(VertexAttributeValues::Float32x3(pos)) = src.attribute(Mesh::ATTRIBUTE_POSITION) else { return None; };
    let all: Vec<usize> = src.indices()?.iter().collect();
    let keep = |i: usize| match how { 0 => pos[i][0] <= 1e-4, 1 => pos[i][0] >= -1e-4, 2 => pos[i][1] > 1.29 && pos[i][1] < 1.40 && pos[i][0].abs() < 0.2,
                                      3 => pos[i][1] > 1.20 && pos[i][1] < 1.28 && pos[i][0].abs() < 0.2, _ => true };
    let mut kept: Vec<u32> = all.chunks_exact(3).filter(|t| t.iter().all(|&i| keep(i))).flatten().map(|&i| i as u32).collect();
    if how == 5 { let back: Vec<u32> = kept.chunks_exact(3).flat_map(|t| [t[0], t[2], t[1]]).collect(); kept.extend(back); }
    let mut out = src.clone();
    out.insert_indices(Indices::U32(kept));
    if let (Some((from, to)), Some(VertexAttributeValues::Uint16x4(ji))) = (remap, out.attribute_mut(Mesh::ATTRIBUTE_JOINT_INDEX)) {
        for j in ji.iter_mut() { for v in j.iter_mut() { if *v == from { *v = to; } } }
    }
    Some(out)
}

#[derive(Component)]
struct Gib { vel: Vec3, spin: Vec3, floor: f32, age: f32, rest: bool }

/// Update, after the pose: a pending decapitation hides the head parts on both models, shows the neck cap, finds the
/// neck joint for the spurts and throws the head as a gib built from the shown model's posed meshes.
fn gore(mut street: ResMut<Street>, play: Res<super::Play>, prims: Query<(&Mesh3d, &SkinnedMesh, &MeshMaterial3d<StandardMaterial>)>,
        globals: Query<&GlobalTransform>, mut meshes: ResMut<Assets<Mesh>>, bindposes: Res<Assets<SkinnedMeshInverseBindposes>>, mut commands: Commands) {
    if !street.on { return; }
    let s = &mut *street;
    let bone_slot = |g: &Grunt, k: usize, skin: &SkinnedMesh, id: i16, bones: &[mw_data::Bone]| -> Option<u16> {
        let e = g.joints[k].as_ref()?.get(bones.iter().position(|b| b.id == id)?).copied()?;
        skin.joints.iter().position(|j| *j == e).map(|p| p as u16)
    };
    let bones = s.bank.as_ref().map(|b| b.bones.clone()).unwrap_or_default();
    for gi in 0..s.grunts.len() {
        let g = &s.grunts[gi];
        let Some(c) = g.split else { continue; };
        if !c.done || c.meshes || g.parts.iter().any(|p| p.is_empty()) { continue; }
        // which cut each part takes: 0 x <= 0, 1 x >= 0, 2 the waist ring under the upper half, 3 the lower half's top
        // ring, 4 the legs rebound below the waist, 5 a cross-section shown from both sides; None hides it
        let mut swaps: Vec<(Entity, Option<u8>)> = vec![];
        for k in 0..2 {
            for &(i, e) in &g.parts[k] {
                let off = if g.outfit == 0 { OUTFIT_B.contains(&i) } else { OUTFIT_A.contains(&i) };
                if off || (g.headless && HEAD_PARTS.contains(&i)) { continue; }
                let how = match (c.waist, c.b) {
                    (false, b) => if i == 13 || i == 15 { Some(5) } else if i == 14 { None } else { Some(b as u8) },
                    (true, false) => if i == ZUBON || i == 13 || i == 15 { None } else if i == 14 { Some(2) } else { Some(255) },
                    (true, true) => if i == ZUBON { Some(4) } else if i == 14 { Some(3) } else { None },
                };
                swaps.push((e, how));
            }
        }
        let mut bleed_joint = None;
        for (e, how) in swaps {
            let Some(how) = how else { commands.entity(e).insert(Visibility::Hidden); continue; };
            if how == 255 { continue; }
            let Ok((m, skin, _)) = prims.get(e) else { continue; };
            let key = (m.0.id(), how);
            let handle = if let Some(h) = s.cut_meshes.get(&key) { h.clone() } else {
                let Some(src) = meshes.get(&m.0) else { continue; };
                let remap = (how == 4).then(|| (bone_slot(&s.grunts[gi], 0, skin, 1, &bones).or(bone_slot(&s.grunts[gi], 1, skin, 1, &bones)), bone_slot(&s.grunts[gi], 0, skin, 15, &bones).or(bone_slot(&s.grunts[gi], 1, skin, 15, &bones))));
                let Some(cut) = half_mesh(src, how, remap.and_then(|(a, b)| Some((a?, b?)))) else { continue; };
                let h = meshes.add(cut);
                s.cut_meshes.insert(key, h.clone());
                h
            };
            if bleed_joint.is_none() { bleed_joint = bone_slot(&s.grunts[gi], play.colored as usize, skin, if c.waist { 1 } else { 3 }, &bones).and_then(|j| skin.joints.get(j as usize).copied()); }
            commands.entity(e).insert((Mesh3d(handle), Visibility::Inherited));
        }
        let g = &mut s.grunts[gi];
        if let Some(j) = bleed_joint { g.neck = Some(j); }
        g.split = Some(Cut { meshes: true, ..c });
    }
    for g in &mut s.grunts {
        if !g.decap || g.parts.iter().any(|p| p.is_empty()) { continue; }
        g.decap = false;
        let shown = play.colored as usize;
        // [added] a civilian's head parts come from his rig; his stump spurts from the neck bone (no cap mesh)
        let civ = g.body >= 10;
        let heads: Vec<usize> = if civ { s.civs.get(g.body as usize - 10).and_then(|c| c.as_ref()).map(|c| c.head.clone()).unwrap_or_default() } else { HEAD_PARTS.to_vec() };
        if civ { g.neck = bones.iter().position(|b| b.id == 3).and_then(|i| g.joints[shown].as_ref()?.get(i).copied()); }
        for k in 0..2 {
            for &(i, e) in &g.parts[k] {
                if heads.contains(&i) { commands.entity(e).insert(Visibility::Hidden); }
                if civ || i != NECK_CAP_SOURCE { continue; }
                let Ok((m, skin, _)) = prims.get(e) else { continue; };
                if s.neck_cap.is_none() { s.neck_cap = meshes.get(&m.0).and_then(neck_cap).map(|c| meshes.add(c)); }
                let Some(cap) = s.neck_cap.clone() else { continue; };
                // the neck joint: the one the cap leans on most
                if k == shown {
                    if let Some(VertexAttributeValues::Uint16x4(ji)) = meshes.get(&cap).and_then(|c| c.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)) {
                        let mut count = std::collections::HashMap::new();
                        for j in ji { *count.entry(j[0]).or_insert(0) += 1; }
                        g.neck = count.into_iter().max_by_key(|c| c.1).and_then(|(j, _)| skin.joints.get(j as usize).copied());
                    }
                }
                commands.entity(e).insert((Mesh3d(cap), Visibility::Inherited));
            }
        }
        // the gib: every visible head part of the shown model, posed as it is now, around their centre
        let mut pieces: Vec<(Mesh, Handle<StandardMaterial>)> = vec![];
        let mut points: Vec<Vec3> = vec![];
        for &(i, e) in &g.parts[shown] {
            if !heads.contains(&i) || (!civ && g.outfit == 0 && OUTFIT_B.contains(&i)) { continue; }
            let Ok((m, skin, mat)) = prims.get(e) else { continue; };
            let (Some(src), Some(ibp)) = (meshes.get(&m.0), bindposes.get(&skin.inverse_bindposes)) else { continue; };
            let Some((p, n)) = skinned_world(src, skin, ibp, &globals) else { continue; };
            let Some(indices) = src.indices().map(|x| x.iter().map(|i| i as u32).collect::<Vec<_>>()) else { continue; };
            let uv = src.attribute(Mesh::ATTRIBUTE_UV_0).cloned();
            let mut piece = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            points.extend(&p);
            piece.insert_attribute(Mesh::ATTRIBUTE_POSITION, p.iter().map(|v| v.to_array()).collect::<Vec<_>>());
            piece.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n.iter().map(|v| v.to_array()).collect::<Vec<_>>());
            if let Some(uv) = uv { piece.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv); }
            piece.insert_indices(Indices::U32(indices));
            pieces.push((piece, mat.0.clone()));
        }
        if points.is_empty() { continue; }
        let centre = points.iter().copied().sum::<Vec3>() / points.len() as f32;
        let turn = if g.outfit == 0 { 1. } else { -1. };
        let side = Vec3::new(-g.fling.z, 0., g.fling.x) * 0.6 * turn;
        let gib = commands.spawn((Gib { vel: g.fling * 2.4 + side + Vec3::Y * 3.4, spin: Vec3::new(9., 4., 6.) * turn, floor: g.pos.y, age: 0., rest: false },
                                  Transform::from_translation(centre), Visibility::Visible)).id();
        for (mut piece, material) in pieces {
            if let Some(VertexAttributeValues::Float32x3(p)) = piece.attribute_mut(Mesh::ATTRIBUTE_POSITION) { for v in p.iter_mut() { *v = (Vec3::from_array(*v) - centre).to_array(); } }
            let child = commands.spawn((Mesh3d(meshes.add(piece)), MeshMaterial3d(material), Transform::default())).id();
            commands.entity(gib).add_child(child);
        }
    }
}

/// The thrown heads: fall, tumble, bounce, come to rest; shrink away as their bodies fade.
fn gibs(time: Res<Time>, play: Res<super::Play>, mut q: Query<(Entity, &mut Gib, &mut Transform)>, mut commands: Commands) {
    if play.fight.hitstop > 0 { return; }
    let dt = time.delta_secs().min(1. / 20.);
    for (e, mut gib, mut t) in &mut q {
        gib.age += dt;
        // gone with its body: the corpse lies CORPSE and fades over FADE game frames
        let life = (CORPSE + FADE) / 30.;
        if gib.age > life { commands.entity(e).despawn(); continue; }
        if gib.age > life - FADE / 30. { t.scale = Vec3::splat((life - gib.age) / (FADE / 30.)); }
        if gib.rest { continue; }
        gib.vel.y -= 9.8 * dt;
        t.translation += gib.vel * dt;
        let spin = gib.spin * dt;
        t.rotation = (Quat::from_scaled_axis(spin) * t.rotation).normalize();
        let bottom = gib.floor + 0.12;
        if t.translation.y < bottom {
            t.translation.y = bottom;
            if gib.vel.y.abs() < 0.8 { gib.rest = true; continue; }
            gib.vel = Vec3::new(gib.vel.x * 0.5, -gib.vel.y * 0.3, gib.vel.z * 0.5);
            gib.spin *= 0.5;
        }
    }
}


pub fn pose(street: Res<Street>, play: Res<super::Play>, mut transforms: Query<&mut Transform>, mut vis: Query<&mut Visibility>, globals: Query<&GlobalTransform>, mut frames: Local<u32>) {
    if !street.on { return; }
    // inspection: MW_JOINT_LOG=n prints the shown model's leg joints (hip, knee, ankle, toe) in the grunt's frame at
    // rendered frame n
    *frames += 1;
    if std::env::var("MW_JOINT_LOG").ok().and_then(|v| v.parse::<u32>().ok()) == Some(*frames) {
        if let (Some(bank), Some(g)) = (&street.bank, street.grunts.first()) {
            let k = play.colored as usize;
            if let Some(js) = &g.joints[k] {
                let inv = Transform::from_translation(g.pos).with_rotation(Quat::from_rotation_y(g.yaw)).compute_affine().inverse();
                let at = |id: i16| bank.bones.iter().position(|b| b.id == id).and_then(|i| globals.get(js[i]).ok()).map(|t| inv.transform_point3(t.translation()));
                eprintln!("[joints] body {} clip {} frame {:.0}: {:?}", g.body, bank.clips[g.clip].name, g.frame, [15, 16, 17, 18, 19].map(|i| at(i).map(|v| (v * 100.).round() / 100.)));
            }
        }
    }
    let Some(bank) = &street.bank else { return; };
    for g in &street.grunts {
        let clip = &bank.clips[g.clip];
        let mut poses = sample_retargeted(&bank.bones, clip, g.frame.min((clip.frames.max(2) - 1) as f32), Retarget::NATIVE);
        if !matches!(g.state, State::Suplexed | State::Thrown(_)) && g.split.is_none() { strip_root_xz(&bank.bones, &mut poses); }
        // a vampire: her own bind offsets on the body joints (the bank's translations are the grunt's), the root and its
        // travel scaled by her hip height; the weapon bones keep theirs
        if let Some(Some(rig)) = (1..=3).contains(&g.body).then(|| street.vamps.get(g.body as usize - 1)).flatten() {
            for (i, b) in bank.bones.iter().enumerate() {
                let Some(p) = poses.get_mut(i) else { continue; };
                let cp = if b.parent >= 0 { rig.corr[b.parent as usize] } else { Quat::IDENTITY };
                if (cp != Quat::IDENTITY || rig.corr[i] != Quat::IDENTITY) && std::env::var_os("MW_VAMP_NOCORR").is_none() {
                    p.rotation = (cp.inverse() * Quat::from_array(p.rotation) * rig.corr[i]).normalize().to_array();
                }
                match b.id {
                    0 => { for v in p.translation.iter_mut() { *v *= rig.root_scale; } }
                    100 | 101 => {}
                    _ => if let Some(Some(r)) = rig.rel.get(i) { p.translation = r.to_array(); },
                }
            }
            if let Some(p) = poses.get_mut(bank.bones.len()) { for v in p.translation.iter_mut() { *v *= rig.root_scale; } }
        }
        // [added, town] a civilian: his own bind offsets; the root scaled by his hip height against the clip's owner
        // (Jack's walk / run / stand, the grunt's others)
        if let Some(Some(rig)) = (g.body >= 10).then(|| street.civs.get(g.body as usize - 10)).flatten() {
            let k = if clip.name.starts_with("pl0030_") { rig.root_jack } else { rig.root_grunt };
            for (i, b) in bank.bones.iter().enumerate() {
                let Some(p) = poses.get_mut(i) else { continue; };
                match b.id {
                    0 => { for v in p.translation.iter_mut() { *v *= k; } }
                    _ => if let Some(Some(r)) = rig.rel.get(i) { p.translation = r.to_array(); },
                }
            }
            if let Some(p) = poses.get_mut(bank.bones.len()) { for v in p.translation.iter_mut() { *v *= k; } }
        }
        // (setDrawEnable(0): the vertical copy shows at the cut)
        let unseen = g.split.is_some_and(|c| c.b && !c.waist && !c.meshes);
        // a corpse's jolt: the hips kick about the axis across the blow (in his own frame), easing out
        if g.jolt > 0.01 {
            let local = Quat::from_rotation_y(-g.yaw) * g.jolt_dir;
            let axis = Vec3::new(local.z, 0., -local.x).normalize_or(Vec3::X);
            let r = Quat::from_axis_angle(axis, 0.35 * g.jolt) * Quat::from_array(poses[0].rotation);
            poses[0].rotation = r.normalize().to_array();
            poses[0].translation[1] += 0.08 * g.jolt;
        }
        // fading, he sinks a little into the street
        let at = g.pos - Vec3::Y * 0.25 * (1. - g.fade);
        for k in 0..2 {
            let shown = (k == 1) == play.colored && !unseen;
            if let Ok(mut v) = vis.get_mut(g.roots[k]) { *v = if shown { Visibility::Visible } else { Visibility::Hidden }; }
            if let Ok(mut t) = transforms.get_mut(g.roots[k]) { *t = Transform::from_translation(at).with_rotation(Quat::from_rotation_y(g.yaw)); }
            let Some(js) = &g.joints[k] else { continue; };
            for (e, p) in js.iter().zip(&poses) {
                if let Ok(mut t) = transforms.get_mut(*e) {
                    t.translation = Vec3::from_array(p.translation); t.rotation = Quat::from_array(p.rotation); t.scale = Vec3::from_array(p.scale);
                }
            }
        }
    }
}

/// Update: a fading corpse gets its own copies of its materials (the grunts share theirs), blended out with its opacity;
/// its stains follow (play_decal.rs reads the root's Fade).
fn fade(mut street: ResMut<Street>, parts: Query<&MeshMaterial3d<StandardMaterial>>, mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    if !street.on { return; }
    for g in &mut street.grunts {
        if g.fade >= 1. { continue; }
        for k in 0..2 {
            commands.entity(g.roots[k]).insert(super::decal::Fade(g.fade));
            for &(_, e) in &g.parts[k] {
                let Ok(m) = parts.get(e) else { continue; };
                if !g.fade_mats {
                    let Some(mut copy) = materials.get(&m.0).cloned() else { continue; };
                    copy.alpha_mode = AlphaMode::Blend;
                    commands.entity(e).insert(MeshMaterial3d(materials.add(copy)));
                } else if let Some(mut mat) = materials.get_mut(&m.0) { mat.base_color.set_alpha(g.fade); }
            }
        }
        g.fade_mats = true;
    }
}

#[derive(Component)]
struct FirePart { vel: Vec3, age: f32, life: f32, size: (f32, f32), stage: usize }
#[derive(Resource)]
struct FireAssets { mesh: Handle<Mesh>, stages: Vec<Handle<StandardMaterial>> }
/// Flame colour over a sprite's life (hot core to smoke), HDR so the modern look's bloom catches it.
const FIRE_STAGES: [(f32, [f32; 4]); 5] = [(0.12, [6., 5., 2.4, 1.]), (0.32, [5., 2.6, 0.6, 1.]), (0.58, [4., 1.2, 0.2, 1.]), (0.8, [1.8, 0.3, 0.05, 1.]),
                                            (1.01, [0.07, 0.06, 0.06, 0.5])];

/// Update: the breath's flames from the mouth, the player's burning, and the sprites' flight (billboards).
fn fire(street: Res<Street>, play: Res<super::Play>, time: Res<Time>, globals: Query<&GlobalTransform>, cams: Query<&GlobalTransform, (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>,
        mut parts: Query<(Entity, &mut FirePart, &mut Transform, &mut MeshMaterial3d<StandardMaterial>)>, assets: Option<Res<FireAssets>>,
        mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands, mut seed: Local<u32>) {
    if !street.on { return; }
    let Some(fa) = assets else {
        // a soft round flame puff, licked at the edge
        let n = 64u32;
        let mut px = Vec::with_capacity((n * n * 4) as usize);
        for y in 0..n { for x in 0..n {
            let (u, v) = (x as f32 / (n - 1) as f32 * 2. - 1., y as f32 / (n - 1) as f32 * 2. - 1.);
            let r = (u * u + v * v).sqrt() * (1. + 0.12 * (v.atan2(u) * 5.).sin());
            let a = (1. - r).clamp(0., 1.).powf(1.6);
            px.extend([255, 255, 255, (a * 255.) as u8]);
        } }
        let image = images.add(Image::new(bevy::render::render_resource::Extent3d { width: n, height: n, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2, px, bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::all()));
        let stages = FIRE_STAGES.iter().enumerate().map(|(k, (_, c))| materials.add(StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::new(c[0], c[1], c[2], c[3])), base_color_texture: Some(image.clone()), unlit: true,
            alpha_mode: if k + 1 == FIRE_STAGES.len() { AlphaMode::Blend } else { AlphaMode::Add }, ..default() })).collect();
        commands.insert_resource(FireAssets { mesh: meshes.add(Rectangle::new(1., 1.)), stages });
        return;
    };
    let dt = time.delta_secs().min(1. / 20.);
    let mut rand = || { *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (*seed >> 8) as f32 / (1 << 24) as f32 };
    let spawn = |commands: &mut Commands, at: Vec3, vel: Vec3, life: f32, size: (f32, f32)| {
        commands.spawn((Mesh3d(fa.mesh.clone()), MeshMaterial3d(fa.stages[0].clone()), Transform::from_translation(at).with_scale(Vec3::splat(size.0)),
                        FirePart { vel, age: 0., life, size, stage: 0 }, bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
    };
    let per_frame = (dt * 90.).ceil() as usize;
    let shown = play.colored as usize;
    let head = street.bank.as_ref().and_then(|b| b.bones.iter().position(|b| b.id == 4));
    for g in &street.grunts {
        let State::Attack(k) = g.state else { continue; }; if k != tables().fire { continue; }
        if !(55. ..88.).contains(&g.frame) { continue; }
        let Some(h) = head.and_then(|h| g.joints[shown].as_ref().and_then(|js| js.get(h).copied())) else { continue; };
        let Ok(gt) = globals.get(h) else { continue; };
        // the mouth: 4.5 cm up and 18 cm out from the head joint; the breath leaves along the head's facing
        let mouth = gt.transform_point(Vec3::new(0., 0.045, 0.18));
        let mut dir = (gt.rotation() * Vec3::Z).normalize_or(Vec3::Z);
        dir.y = dir.y.clamp(-0.35, 0.35); let dir = dir.normalize();
        let side = dir.cross(Vec3::Y).normalize_or(Vec3::X); let up = side.cross(dir);
        for _ in 0..per_frame {
            let spread = side * (rand() - 0.5) * 1.1 + up * (rand() - 0.4) * 0.8;
            spawn(&mut commands, mouth + dir * rand() * 0.1, dir * (9. + rand() * 2.5) + spread * 1.4, 0.5 + rand() * 0.2, (0.07, 0.9 + rand() * 0.35));
        }
    }
    // the player on fire
    if street.burn > 0 && play.fight.player.hp > 0 {
        let at = Vec3::from_array(play.fight.player.pos);
        for _ in 0..(per_frame / 2).max(1) {
            let p = at + Vec3::new((rand() - 0.5) * 0.5, 0.3 + rand() * 1.4, (rand() - 0.5) * 0.5);
            spawn(&mut commands, p, Vec3::new((rand() - 0.5) * 0.4, 1.2 + rand() * 1.1, (rand() - 0.5) * 0.4), 0.35 + rand() * 0.2, (0.08, 0.4 + rand() * 0.2));
        }
    }
    let face = cams.single().map(|c| c.rotation()).unwrap_or_default();
    for (e, mut f, mut t, mut mat) in &mut parts {
        f.age += dt;
        let k = f.age / f.life;
        if k >= 1. { commands.entity(e).despawn(); continue; }
        let v = f.vel; t.translation += v * dt;
        f.vel *= 1. - 2.4 * dt; f.vel.y += 1.6 * dt;   // drag, and the heat lifts it
        t.scale = Vec3::splat(f.size.0 + (f.size.1 - f.size.0) * k.sqrt());
        t.rotation = face;
        let stage = FIRE_STAGES.iter().position(|(end, _)| k < *end).unwrap_or(FIRE_STAGES.len() - 1);
        if stage != f.stage { f.stage = stage; mat.0 = fa.stages[stage].clone(); }
    }
}

// ---- the actor editor (play_editor.rs) on em0101 ----

/// The player at his editor spot (default: where he stands), every grunt gone (faded out at once) and a fresh one of
/// `outfit` 3 m in front of him, facing him, waiting.
fn ed_reset(s: &mut Street, play: &mut super::Play, arena: &super::Arena, spot: Option<Vec3>, pspot: Option<Vec3>, outfit: usize, body: u8) {
    if play.fight.player.state() == mw_sim::fight::STATE_EXTERNAL { play.fight.end_external(); }
    play.fight.carry = None; s.held = None; s.carry_lost = false; s.suplexed = None; s.attacker = None; s.burn = 0;
    let p = pspot.unwrap_or(Vec3::from_array(play.fight.player.pos));
    let fy = |v: Vec3| super::floor(&arena.0, v + Vec3::Y * 2.).unwrap_or(v.y);
    let f = play.fight.player.loco.facing;
    let at = spot.unwrap_or(p + Vec3::new(f.sin(), 0., f.cos()) * 3.);
    play.fight.player.pos = [p.x, fy(p), p.z];
    play.fight.player.loco.facing = (at.x - p.x).atan2(at.z - p.z);
    play.fight.player.hp = mw_sim::fight::PLAYER_MAX_HP;
    play.cam_yaw = play.fight.player.loco.facing;
    for g in s.grunts.iter_mut() { g.state = State::Dead; g.timer = CORPSE + FADE; g.fade = 0.; }
    s.queue.push((at, (p.x - at.x).atan2(p.z - at.z), outfit, body));
    s.ed_grunt = None; s.ed_loop = false;
}

/// An editor command, played through his own states.
fn ed_cmd(s: &mut Street, i: usize, cmd: &super::editor::Cmd, play: &mut super::Play, fx: &mut super::fx::Fx) -> Result<(), String> {
    use super::editor::Cmd;
    let pl = Vec3::from_array(play.fight.player.pos);
    let pf = play.fight.player.loco.facing;
    let pfwd = Vec3::new(pf.sin(), 0., pf.cos());
    let dead = s.grunts[i].state == State::Dead;
    if dead && !matches!(cmd, Cmd::Reset | Cmd::Hurt(_)) { return Err("He is dead: R brings a fresh one".into()); }
    let mut g = std::mem::replace(&mut s.grunts[i], Grunt::dummy());
    let r = (|| {
        match cmd {
            Cmd::Reset => {}
            Cmd::Scene { .. } => return Err("em0101 has no camera scenes".into()),
            Cmd::Clip { name, looped } => {
                if !s.has(name) { return Err(format!("{name} is not in em0101's bank")); }
                s.set(&mut g, State::Editor, name); s.ed_loop = *looped;
            }
            Cmd::Move(m) => {
                if let Some(k) = tables().attacks.iter().position(|a| &a.name == m) {
                    // in reach, facing him; a combo runs to its full count
                    let reach = (atk(k).range * 0.8).min(2.);
                    g.pos = Vec3::new(pl.x, g.pos.y, pl.z) + pfwd * reach; g.yaw = pf + std::f32::consts::PI;
                    g.combo = (1, if k < 2 { 3 } else { 1 });
                    s.set(&mut g, State::Attack(k), &atk(k).clip.clone());
                    return Ok(());
                }
                match m.as_str() {
                    "laugh" => { let k = s.roll(tables().laughs.len() as u32) as usize; let c = tables().laughs[k].clone(); s.set(&mut g, State::Editor, &c); }
                    "threat" => { let k = s.roll(tables().threats.len() as u32) as usize; let c = tables().threats[k].clone(); s.set(&mut g, State::Editor, &c); }
                    "spotted" => s.set(&mut g, State::Editor, "em0110_0036.mot"),
                    "dodge" => { g.knock = -Vec3::new(g.yaw.sin(), 0., g.yaw.cos()) * 0.12; s.set(&mut g, State::Dodge, "em0101_0121.mot"); }
                    "guard" => { s.set(&mut g, State::Guard, "em0110_1063.mot"); g.timer = 60.; g.guards = 2; g.idle = 0.; }
                    "stun" => { s.set(&mut g, State::Piyori, "em0110_1078.mot"); g.timer = 150.; }
                    "faint" => { g.fainted = true; g.timer = FAINT_FRAMES; s.set(&mut g, State::Faint, "em0110_1120.mot"); }
                    // the chainsaw split: he faints in front of the player, who cuts him (step runs it: it spawns half B)
                    "split_vertical" | "split_waist" => {
                        g.pos = Vec3::new(pl.x, g.pos.y, pl.z) + pfwd * 1.6; g.yaw = pf + std::f32::consts::PI; g.fainted = true; g.timer = FAINT_FRAMES;
                        s.set(&mut g, State::Faint, "em0110_1120.mot");
                        s.ed_split = Some(m == "split_waist");
                    }
                    "fear" => { g.feared = true; g.fear_delay = 1; let away = Vec3::new(g.pos.x - pl.x, 0., g.pos.z - pl.z).normalize_or(-Vec3::Z); g.flee_yaw = away.x.atan2(away.z); }
                    // the townsfolk's own moves, looped where they loop in town
                    "walk" | "run" | "idle" | "chat" | "bump" | "startle" | "cower" | "victim" | "laugh_civ" if g.civ.is_some() => {
                        let (clip, looped) = match m.as_str() { "walk" => (civ_clips()[1], true), "run" => (civ_clips()[2], true), "idle" => (civ_clips()[0], true),
                            "chat" => ("em0110_0038.mot", true), "bump" => ("em0110_1075.mot", false), "startle" => ("em0110_0016.mot", false),
                            "cower" => ("em0110_1109.mot", false), "victim" => ("em0110_110b.mot", true), _ => ("em0110_0045.mot", false) };
                        s.set(&mut g, State::Editor, clip); s.ed_loop = looped;
                    }
                    "civ_1" | "civ_2" | "civ_3" | "civ_4" => { g.body = 9 + m[4..].parse::<u8>().unwrap_or(1); return Err("Body set: R brings them in".into()); }
                    "grunt" | "vampire_1" | "vampire_2" | "vampire_3" => { g.body = m.strip_prefix("vampire_").and_then(|n| n.parse().ok()).unwrap_or(0); return Err("Body set: R brings her in".into()); }
                    "outfit_a" | "outfit_b" => { g.outfit = if m == "outfit_a" { 0 } else { 1 }; return Err("Outfit set: R brings him in it".into()); }
                    other => return Err(format!("em0101 has no move {other:?} ({}, laugh, threat, spotted, dodge, guard, stun, faint, split_vertical, split_waist, fear, outfit_a, outfit_b)",
                                                tables().attacks.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", "))),
                }
            }
            _ => {}
        }
        Ok(())
    })();
    let outfit = g.outfit; let body = g.body;
    s.grunts[i] = g;
    if let Cmd::Move(m) = cmd { if m.starts_with("outfit_") { if let Some(last) = s.grunts.last_mut() { last.outfit = outfit; } } }
    if let Cmd::Move(m) = cmd { if m == "grunt" || m.starts_with("vampire_") || m.starts_with("civ_") { if let Some(last) = s.grunts.last_mut() { last.body = body; } } }
    r?;
    match cmd {
        Cmd::Hurt(k) => {
            let g = &s.grunts[i];
            let fwd = Vec3::new(g.yaw.sin(), 0., g.yaw.cos());
            let right = Vec3::new(fwd.z, 0., -fwd.x);
            // the blow comes from that side and pushes away from it
            let from = match k.as_str() { "back" => -fwd, "left" => -right, "right" => right, _ => fwd };
            let at = g.pos + Vec3::Y * 1.2 + from * 0.25;
            if k == "shot" {
                let o = g.pos + Vec3::Y * 1.3 + fwd * 4.;
                if s.shoot(o, (g.pos + Vec3::Y * 1.2 - o).normalize(), 20, f32::INFINITY, fx).is_none() { return Err("The round missed".into()); }
                return Ok(());
            }
            if dead { s.corpse_hit(i, -from, at, fx); return Ok(()); }
            // (no dodge or guard roll: the hit lands)
            let ev = s.grunts[i].evades; s.grunts[i].evades = 0;
            let (power, heavy, atk) = match k.as_str() { "light" => (10, false, 0x6d), "down" | "sweep" => (15, false, SWEEP_ATK), "launch" => (60, true, 0x76), _ => (25, true, 0x76) };
            s.damage(i, power, -from, heavy, atk, at, fx);
            s.grunts[i].evades = ev;
        }
        Cmd::Kill => {
            let (yaw, pos, hp) = (s.grunts[i].yaw, s.grunts[i].pos, s.grunts[i].hp);
            let fwd = Vec3::new(yaw.sin(), 0., yaw.cos());
            s.grunts[i].evades = 0;
            let at = pos + Vec3::Y * 1.2 + fwd * 0.25;
            s.damage(i, hp + 50, -fwd, true, 0x76, at, fx);
        }
        Cmd::Pair { name, .. } => match name.as_str() {
            // the player's grab: Kreely suplexes, Jack catches and carries (then F2: X head-butts, punch / swing throws)
            "grab" | "suplex" | "carry" => {
                let g = &mut s.grunts[i];
                g.pos = Vec3::new(pl.x, g.pos.y, pl.z) + pfwd * 0.9; g.yaw = pf + std::f32::consts::PI;
                super::editor::press(super::editor::Press::Grab);
            }
            other => return Err(format!("em0101 has no pair {other:?} (grab)")),
        },
        _ => {}
    }
    Ok(())
}

/// getComboDamageMotion 810177c4 (the +0xa78 tree, cEmZako::onInit 8100b958; research/ZAKO-AI.md 4.3): a punch (0x6d-0x70,
/// P) or chainsaw (0x75-0x78, C) hit picks its clip from the string so far, oldest first; other types None. The string
/// starts over after its fourth hit, or once 30 frames pass without a hit (clearComboDmgLog when the depth falls to 0).
fn combo_clip(g: &mut Grunt, atk: u32, pct: i32, gap: f32) -> Option<&'static str> {
    let chainsaw = match atk { 0x6d..=0x70 => false, 0x75..=0x78 => true, _ => return None };
    if g.combo_log.len() >= 4 || gap > 30. { g.combo_log.clear(); }
    g.combo_log.push(chainsaw);
    let key: String = g.combo_log.iter().map(|c| if *c { 'C' } else { 'P' }).collect();
    // PC launches with 10a0 only at HP <= 浮かし体力 90 %, else 10a4
    let launch = if pct > 90 { "em0110_10a4.mot" } else { "em0110_10a0.mot" };
    Some(match key.as_str() {
        "P" => "em0110_10a5.mot", "C" => "em0110_1026.mot",
        "PP" => "em0110_10a6.mot", "PC" => launch, "CP" => "em0110_1000.mot", "CC" => "em0110_1026.mot",
        "PPP" | "PCP" => "em0110_10a7.mot", "PPC" | "PCC" => "em0110_10a4.mot", "CPP" => "em0110_1000.mot", "CPC" | "CCP" => "em0110_1026.mot", "CCC" => "em0110_104d.mot",
        "PPPP" | "PCPP" => "em0110_10a8.mot", "PPPC" | "PCPC" => "em0110_10a4.mot", "PPCP" | "PCCP" | "CPCP" | "CCPP" => "em0110_1000.mot",
        "PPCC" | "PCCC" | "CPPC" | "CPCC" | "CCPC" => "em0110_1026.mot", "CPPP" => "em0110_1008.mot", "CCCC" => "em0110_1102.mot",
        _ => return None,
    })
}

/// [added] VR: the hands on the street's grunts and townsfolk (play_vr_interact.rs).
impl Street {
    /// The living, standing-or-reeling body nearest a hand within `reach` of its surface (a 0.3 m capsule 0.3-2.0 m
    /// over his feet): caught as the native grab catches (the hold timer by HP, struggling, shaking free), but held where
    /// the hand is. Returns his id and the hand's offset from his position.
    pub fn vr_grab(&mut self, hand: Vec3, reach: f32, player: Vec3) -> Option<(u32, Vec3)> {
        if !self.on { return None; }
        let i = self.grunts.iter().enumerate()
            .filter(|(_, g)| g.split.is_none() && !matches!(g.state, State::Dead | State::Split | State::Suplexed | State::Thrown(_) | State::Stuck(_) | State::Editor | State::Held(_) | State::Headbutted))
            .map(|(i, g)| { let y = hand.y.clamp(g.pos.y + 0.3, g.pos.y + 2.0); (i, hand.distance(Vec3::new(g.pos.x, y, g.pos.z)) - 0.3) })
            .filter(|(_, d)| *d < reach).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)?;
        let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
        self.release(&g);
        let pct = g.hp * 100 / g.max_hp.max(1);
        g.timer = if pct <= 70 { 240. } else if pct <= 99 { 180. } else { 90. };
        g.knock = Vec3::ZERO; g.vy = 0.;
        g.yaw = (player.x - g.pos.x).atan2(player.z - g.pos.z);
        let catch = if self.has("em0110_1071.mot") { "em0110_1071.mot" } else { "em0110_1072.mot" };
        if self.has(catch) { self.set(&mut g, State::Held(false), catch); } else { g.state = State::Held(false); }
        let id = g.id; let off = hand - g.pos;
        self.grunts[i] = g;
        self.vr_held.push(id);
        Some((id, off))
    }
    /// Holds grunt `id` with his position at `at` (lifted off the floor if the hand is high), facing the player. False
    /// once he is no longer held (he shook free, died, was knocked out of it).
    pub fn vr_hold(&mut self, id: u32, at: Vec3, player: Vec3) -> bool {
        let held = self.grunts.iter_mut().find(|g| g.id == id).filter(|g| matches!(g.state, State::Held(_) | State::Headbutted));
        let Some(g) = held else { self.vr_held.retain(|x| *x != id); return false; };
        // held out at arm's length: his body no nearer the player than 0.62 m
        let mut at = at;
        let d = Vec2::new(at.x - player.x, at.z - player.z);
        if d.length() < 0.62 { let n = d.normalize_or(Vec2::new(g.pos.x - player.x, g.pos.z - player.z).normalize_or(Vec2::Y)) * 0.62; at.x = player.x + n.x; at.z = player.z + n.y; }
        g.pos = g.pos.lerp(at, 0.6);
        g.yaw = (player.x - g.pos.x).atan2(player.z - g.pos.z);
        true
    }
    /// Lets grunt `id` go with the hand's velocity (m/s): thrown (flying, knocking down whoever he hits, a hard landing)
    /// above a toss's speed, else dropped.
    pub fn vr_release(&mut self, id: u32, vel: Vec3, _fx: &mut super::fx::Fx) {
        self.vr_held.retain(|x| *x != id);
        let Some(i) = self.grunts.iter().position(|g| g.id == id) else { return; };
        if !matches!(self.grunts[i].state, State::Held(_) | State::Headbutted) { return; }
        let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
        if vel.length() > 2.2 {
            let per = vel / 30.;
            g.knock = Vec3::new(per.x, 0., per.z); g.vy = per.y.max(0.02);
            g.yaw = (-per.x).atan2(-per.z); g.since_hit = 0.;
            let flight = if self.has("em0110_1091.mot") { "em0110_1091.mot" } else { "em0110_1090.mot" };
            self.set(&mut g, State::Launched, flight);
            self.knocks += 1;
            self.vr_thrown.push((id, per, vec![]));
        } else {
            let fall = if self.has("em0110_1073.mot") { "em0110_1073.mot" } else { "em0110_1001.mot" };
            self.set(&mut g, State::Released, fall);
        }
        self.grunts[i] = g;
    }
    /// A fist at `at` (radius `r`) moving along `dir`: the nearest body it meets takes the blow (`power`, heavy knocks
    /// down) through the native damage (guards, dodges, reactions, gore). A grunt held in the other hand (`held`) reels
    /// in the hold (the head-butt's catched_atk 1079). Returns the hit point and his id.
    #[allow(clippy::too_many_arguments)]
    pub fn vr_strike(&mut self, at: Vec3, r: f32, dir: Vec3, power: i32, heavy: bool, held: Option<u32>, fx: &mut super::fx::Fx) -> Option<(Vec3, u32)> {
        if !self.on { return None; }
        let i = self.grunts.iter().enumerate()
            .filter(|(_, g)| g.split.is_none() && !matches!(g.state, State::Split | State::Editor) && !(g.state == State::Dead && g.fade < 1.))
            .filter_map(|(i, g)| {
                let (lo, hi, rad) = if g.state == State::Dead || matches!(g.state, State::Down | State::Swept) { (-0.1, 0.7, 0.9) } else { (0.25, 2.1, 0.34) };
                let y = at.y.clamp(g.pos.y + lo, g.pos.y + hi);
                let d = Vec2::new(at.x - g.pos.x, at.z - g.pos.z).length();
                (d < rad + r && (at.y - y).abs() < r).then_some((i, d))
            }).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)?;
        let push = Vec3::new(dir.x, 0., dir.z).normalize_or(Vec3::Z);
        let id = self.grunts[i].id;
        if self.grunts[i].state == State::Dead { self.corpse_hit(i, push, at, fx); return Some((at, id)); }
        self.landed = true;
        if held == Some(id) {
            self.grunts[i].since_hit = 0.;
            self.damage(i, power, push, false, 0x48, at, fx);
            let reel = self.clips.get("em0110_1079.mot").copied();
            let g = &mut self.grunts[i];
            if g.state != State::Dead { g.state = State::Headbutted; if let Some(c) = reel { g.clip = c; } g.frame = 0.; g.knock = Vec3::ZERO; }
            return Some((at, id));
        }
        self.damage(i, power, push, heavy, if heavy { 0x99 } else { 0x5e }, at, fx);
        if heavy && self.grunts[i].state != State::Dead && !matches!(self.grunts[i].state, State::Launched | State::Guard | State::Dodge) {
            // a full swing knocks him off his feet
            let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
            g.knock = push * 0.12;
            self.release(&g);
            let flight = if self.has("em0110_1090.mot") { "em0110_1090.mot" } else { "em0110_1091.mot" };
            self.set(&mut g, State::Launched, flight);
            self.knocks += 1;
            self.grunts[i] = g;
        }
        Some((at, id))
    }
    /// The bodies thrown by hand: while he flies, whoever he meets is knocked down along the flight (the native
    /// throw's BLOW, 10); his landing hurts by the speed he was thrown at.
    pub fn vr_flights(&mut self, fx: &mut super::fx::Fx) {
        let mut thrown = std::mem::take(&mut self.vr_thrown);
        thrown.retain_mut(|(id, per, blown)| {
            let Some(i) = self.grunts.iter().position(|g| g.id == *id) else { return false; };
            let sliding = self.grunts[i].state == State::Land && self.grunts[i].knock.length() > 0.03;
            if self.grunts[i].state != State::Launched && !sliding {
                if std::env::var_os("MW_VR_LOG").is_some() {
                    let others: Vec<String> = self.grunts.iter().filter(|o| o.id != *id && o.state != State::Dead).map(|o| format!("{} at {:.2}", o.id, o.pos)).collect();
                    eprintln!("[vr] thrown grunt {} came down at {:.2} ({:?}); others: {}", id, self.grunts[i].pos, self.grunts[i].state, others.join(", "));
                }
                // landed: the hard landing (the native throw's 30, by how hard he was thrown)
                if matches!(self.grunts[i].state, State::Land | State::Down) {
                    let at = self.grunts[i].pos + Vec3::Y * 0.4;
                    let dmg = (per.length() * 30. * 5.).clamp(8., 45.) as i32;
                    let g = &mut self.grunts[i];
                    g.hp -= dmg; g.since_hit = 0.;
                    if std::env::var_os("MW_VR_LOG").is_some() { eprintln!("[vr] thrown grunt {} landed hard ({dmg}, hp {})", g.id, g.hp); }
                    fx.impact(at, per.x.atan2(per.z), 1., [1., 0.12, 0.08]); fx.burst("core", 1005, at, per.x.atan2(per.z), 1.);
                    if g.hp <= 0 { g.timer = 0.; let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy()); self.release(&g); self.set(&mut g, State::Dead, "em0110_1095.mot"); self.grunts[i] = g; }
                }
                return false;
            }
            let body = self.grunts[i].pos + Vec3::Y * 0.9;
            let dir = Vec3::new(per.x, 0., per.z).normalize_or(Vec3::Z);
            let hit: Vec<usize> = self.grunts.iter().enumerate()
                .filter(|(k, o)| *k != i && o.id != 0 && !blown.contains(&o.id) && matches!(o.state, State::Wait | State::Spotted | State::Siege(_) | State::Attack(_) | State::Damage | State::Laugh | State::Piyori | State::Guard))
                .filter(|(_, o)| Vec2::new(o.pos.x - body.x, o.pos.z - body.z).length() < 1.0 && (o.pos.y + 1.0 - body.y).abs() < 1.2).map(|(k, _)| k).collect();
            for k in hit {
                if std::env::var_os("MW_VR_LOG").is_some() { eprintln!("[vr] thrown grunt {} knocked down grunt {}", id, self.grunts[k].id); }
                blown.push(self.grunts[k].id);
                let at = self.grunts[k].pos + Vec3::Y * 1.0;
                self.damage(k, 10, dir, true, 0x15B, at, fx);
                let launch = self.clips.get("em0110_1090.mot").copied();
                let o = &mut self.grunts[k];
                if o.state != State::Dead { o.knock = dir * 0.16; o.state = State::Launched; if let Some(c) = launch { o.clip = c; } o.frame = 0.; }
                self.knocks += 1;
            }
            true
        });
        self.vr_thrown.append(&mut thrown);
    }
}

impl Street {
    /// [added, VR inspection: MW_VR_TEST=grunt] The nearest living grunt stands at `at`, facing `yaw`, waiting.
    pub fn vr_test_place(&mut self, at: Vec3, yaw: f32) -> Option<u32> { self.vr_test_place_except(at, yaw, &[]) }
    pub fn vr_test_place_except(&mut self, at: Vec3, yaw: f32, skip: &[u32]) -> Option<u32> {
        let i = self.grunts.iter().enumerate().filter(|(_, g)| !skip.contains(&g.id) && g.civ.is_none() && g.split.is_none() && !matches!(g.state, State::Dead | State::Split | State::Held(_) | State::Headbutted))
            .min_by(|a, b| a.1.pos.distance(at).total_cmp(&b.1.pos.distance(at))).map(|(i, _)| i)?;
        let mut g = std::mem::replace(&mut self.grunts[i], Grunt::dummy());
        self.release(&g);
        g.pos = at; g.yaw = yaw; g.knock = Vec3::ZERO; g.vy = 0.; g.evades = 0; g.timer = 0.;
        self.set(&mut g, State::Wait, "em0101_0000.mot");
        let id = g.id;
        self.grunts[i] = g;
        Some(id)
    }
    /// [added, VR inspection] A grunt's state, position and HP.
    #[allow(dead_code)]
    pub fn vr_probe(&self, id: u32) -> Option<(String, Vec3, i32)> { self.grunts.iter().find(|g| g.id == id).map(|g| (format!("{:?}", g.state), g.pos, g.hp)) }
}

impl Street {
    /// [added, VR inspection: MW_VR_TEST=civ] The nearest townsperson stands at `at`, facing `yaw`, idling.
    pub fn vr_test_place_civ(&mut self, at: Vec3, yaw: f32) -> Option<u32> {
        let i = self.grunts.iter().enumerate().filter(|(_, g)| g.civ.is_some() && !matches!(g.state, State::Dead | State::Held(_) | State::Headbutted))
            .min_by(|a, b| a.1.pos.distance(at).total_cmp(&b.1.pos.distance(at))).map(|(i, _)| i)?;
        let g = &mut self.grunts[i];
        g.pos = at; g.yaw = yaw; g.knock = Vec3::ZERO; g.vy = 0.;
        Some(g.id)
    }
}
