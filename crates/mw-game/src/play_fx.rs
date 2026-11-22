//! Native EST emitters for hit and blood effects (tools/stage_effects.py documents every field and its native reader).
//! Native, observed with MW_TRACE_EFFECTS (evidence/native-effects-trace*.log, including a manual play with hits on
//! RinRin) and matched to the original code:
//! - every hit Jack takes: core 1000 + 1005 (CreatePartsEff from the player update, getBloodColor(0) = 800000FF), plus
//!   core 28 once his HP was 50 or below (fn_801FDF6C; its request for 25 above 50 finds no record and spawns nothing);
//! - motion sequence op 16 (a = EST id, c = 2 the actor's own bank, 0 core), e.g. em0440 1000/1001 -> 110, 1002 -> 112,
//!   1003 -> 115 then 112, 1004 -> 70, attacks -> 50/51/60/65/90;
//! - RinRin's state code (enemy RSO, base solved from the traced call sites): dm_right/dm_left -> em0440 111, dm_body ->
//!   em0440 200 + 201 (113 has no record), dm_explosion -> core 140 (killed at its end), dm_electricshock -> core 73 + 74,
//!   guard -> core 12 owned by Jack, atk_kamikaze -> em0440 61, atk_throw -> em0440 60 (killed at its end).
//! Sprites and sheets resolve through the core bank for both banks. Emitter count/interval/duration (0 = until killed),
//! lifetime, offsets, velocity/random ranges (U[-1,1]), drag, acceleration, gravity, rotation/spin, sizes, colors, TUV
//! rectangles, pivots and the floor stop (801FBAF8 motion) are native fields.
//! Inferred: the anchor (the owner's root bone, translation only), the 60 Hz effect frame (two steps per 30 Hz tick),
//! quads oriented by all three rotations when the x/y random is set and camera-facing otherwise, height = width x sprite
//! aspect x +0xC0, constant alpha (over-life curves not decoded), one static TUV frame, effects with no end stopped
//! after 600 frames. 131 (finisher) is a 2D screen effect and is not drawn.
use bevy::{asset::RenderAssetUsages, mesh::{Indices, PrimitiveTopology}, prelude::*};
use mw_sim::fight::{rin_state as rs, FightEvent};
use std::collections::HashMap;

#[derive(serde::Deserialize, Clone)]
struct Frame { page: usize, uv: [f32; 4] }
/// Type-24 extension (tools/stage_effects.py `decal`).
#[derive(serde::Deserialize, Clone)]
struct DecalSpec { depth: f32, meshes: [i16; 2], cull: i16 }
#[derive(serde::Deserialize, Clone)]
struct Emitter {
    /// EST particle type (byte +1); 24 = cEsp24, a decal projected onto the owner (play_decal.rs).
    #[serde(default)] kind: u8, #[serde(default)] decal: Option<DecalSpec>,
    life: u16, life_random: u16, flags: [u32; 2], floor: bool, count: u8, interval: u8, frames: u16, tuv: Option<u16>, tex: u16,
    frame_start: u8, offset: [f32; 3], offset_random: [f32; 3], velocity: [f32; 3], velocity_random: [f32; 3], speed_random: f32,
    drag: f32, accel: [f32; 3], rotation: [f32; 3], rotation_random: [f32; 3], spin: [f32; 3], spin_random: [f32; 3],
    size: [f32; 2], size_random: [f32; 2], rgba: [f32; 4], gravity: f32, pivot: [f32; 2],
}
#[derive(serde::Deserialize)]
struct EffectBank { effects: HashMap<String, Vec<Emitter>> }
#[derive(serde::Deserialize)]
struct Bank { blood: [f32; 4], pages: Vec<String>, sheets: HashMap<String, String>, #[serde(default)] sheet_frames: HashMap<String, Vec<String>>, sprites: HashMap<String, Vec<Frame>>, banks: HashMap<String, EffectBank> }

/// EST ids per hit [see module docs].
const PLAYER_HIT: [u32; 2] = [1000, 1005];
/// Extra player-hit effect once the HP before the hit is at most 50 [observed].
const PLAYER_HIT_LOW_HP: (u32, i16) = (28, 50);
const FRAMES_PER_TICK: u32 = 2;
const ENDLESS_CAP: u32 = 600;

#[derive(Clone, Copy, PartialEq)]
enum Owner { Jack, Rin, /** a world point, yaw and size scale (the pistol's muzzle and impacts) */ At(Vec3, f32, f32),
             /** a world point and yaw on a model (a street grunt): its type-24 stains land on that model root */ Body(Vec3, f32, Entity) }
/// Ends an emitter early: a state of RinRin's that killEffect follows, or the motion that started it.
#[derive(Clone, Copy, PartialEq)]
enum Until { RinState(u8), Clip(Owner, usize, u32) }

#[derive(Resource)]
pub struct Fx { bank: Option<Bank>, images: HashMap<String, Handle<Image>>, quads: HashMap<(String, [u32; 2]), Handle<Mesh>>, rng: u32,
                active: Vec<Active>, last: [(usize, u32, f32); 2],
                /// Type-24 stains waiting to be projected onto their owner (play_decal.rs).
                pub decals: Vec<super::decal::DecalRequest>,
                /// Floor blood pools waiting to be laid (play_gore.rs): where, how big (m).
                pub splats: Vec<(Vec3, f32)> }
/// A running emitter, attached to its owner: spawns `count` every `interval + 1` frames for `frames` frames (0: until killed).
struct Active { em: Emitter, age: u32, owner: Owner, tint: Option<[f32; 4]>, until: Option<Until> }

impl Fx {
    /// A blow that draws blood, the same everywhere [added default, on the core bank's own records]: the impact
    /// streaks (1000) and the blood spray (1005) at the point, both growing with the blow; a heavy blow adds a second
    /// spray; a kill adds the burst (532), the gush (489) and the red chunks (543); blood pools under it on the floor
    /// (play_gore.rs), more and bigger with the blow. `power` is the hit's damage.
    pub fn gore(&mut self, at: Vec3, yaw: f32, power: i32, heavy: bool, kill: bool) {
        // (scaled gently: at 1.8x the spray read as clouds)
        let k = (0.75 + power as f32 / 60.).clamp(0.75, 1.25);
        self.impact(at, yaw, k, [1., 0.12, 0.08]);
        self.burst("core", 1005, at, yaw, k);
        if heavy || power >= 20 { self.burst("core", 1005, at + Vec3::Y * 0.15, yaw + 0.6, k * 0.85); }
        if kill { self.burst("core", 532, at, yaw, 1.); self.burst("core", 489, at, yaw, 1.1); self.burst("core", 543, at, yaw, 1.1); }
        let n = 1 + (power / 15).min(3) as usize + if kill { 2 } else { 0 };
        for i in 0..n {
            let r = self.rand();
            let spread = Vec3::new(yaw.sin(), 0., yaw.cos()) * (0.3 + 0.5 * i as f32) + Vec3::new(r, 0., self.rand()) * 0.3;
            self.splats.push((Vec3::new(at.x, at.y, at.z) + spread, (0.6 + power as f32 / 50.).min(1.6) * if kill { 1.5 } else { 1. }));
        }
    }
    fn rand(&mut self) -> f32 { self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (self.rng >> 8) as f32 / (1 << 23) as f32 - 1. }   // U[-1,1]
    fn vec(&mut self, base: [f32; 3], range: [f32; 3]) -> Vec3 { Vec3::new(base[0] + range[0] * self.rand(), base[1] + range[1] * self.rand(), base[2] + range[2] * self.rand()) }
    /// Start an emitter at a world point (other modules: the street grunts' blood).
    pub fn burst(&mut self, bank: &str, id: u32, at: Vec3, yaw: f32, scale: f32) { self.start(bank, id, Owner::At(at, yaw, scale), None); }
    /// A pistol round through flesh [added]: blood sprays back out of the entry toward the shooter (core 1005) and
    /// forward out of the exit with a mist (core 1005 + 133); an entry hole and a larger exit wound stay on the body
    /// (`root`: a grunt's model; None: RinRin). `through`: the body's depth along the shot.
    pub fn wound(&mut self, root: Option<Entity>, at: Vec3, dir: Vec3, through: f32) {
        let dir = dir.normalize_or(Vec3::Z);
        let yaw = dir.x.atan2(dir.z);
        let exit = at + dir * through;
        // (small: 1005 opens with a 1 m white flash, a full-body burst on a round at this scale)
        self.burst("core", 1005, at, yaw + std::f32::consts::PI, 0.35);
        // the exit spray (the mist record, core 133, draws as a body-sized white burst whatever its scale: left out)
        self.burst("core", 1005, exit, yaw, 0.45);
        self.wound_holes(root, at, dir, through);
    }
    /// The wound's holes only (a model drawn twice, source and colour, takes them on both copies).
    pub fn wound_holes(&mut self, root: Option<Entity>, at: Vec3, dir: Vec3, through: f32) {
        let dir = dir.normalize_or(Vec3::Z);
        let exit = at + dir * through;
        for (point, face, image, size) in [(at, -dir, "native/effects/wound-entry.png", 0.15), (exit, dir, "native/effects/wound-exit.png", 0.28)] {
            self.decals.push(super::decal::DecalRequest { on_player: false, root, at: Some((point, face)), meshes: [-1, 0], depth: 0.2, size: [size, 1.],
                offset: Vec3::ZERO, image: image.into(), tint: [1., 1., 1., 1.], life: 0, cull: false, upright: false });
        }
        // (cull off: the GX winding flips the copied triangles' front on some models, which hid the holes; the body
        // itself hides the far side)
    }
    /// Claw marks [added]: three slashes projected onto the player's body at `at`, facing `face` (toward the clawing grunt).
    pub fn claw(&mut self, at: Vec3, face: Vec3) {
        self.burst("core", 1005, at, face.x.atan2(face.z), 0.8);
        self.decals.push(super::decal::DecalRequest { on_player: true, root: None, at: Some((at, face)), meshes: [-1, 0], depth: 0.18, size: [0.24, 1.],
            offset: Vec3::ZERO, image: "native/effects/claw-marks.png".into(), tint: [1., 1., 1., 1.], life: 0, cull: true, upright: false });
    }
    /// Blood that sticks where a blow lands [added placement, the native stain sheet]: one of the core blood-stain
    /// frames (tex 49, the type-24 stains' own image) projected at `at` facing `face` onto the model under it
    /// (`root`; None with `on_player` for the player). `life` in frames, 0 stays.
    pub fn stain(&mut self, root: Option<Entity>, on_player: bool, at: Vec3, face: Vec3, size: f32, life: u16) {
        let k = ((self.rand() * 0.5 + 0.5) * 4.) as usize;
        let Some(image) = self.bank.as_ref().and_then(|b| b.sheet_frames.get("49").and_then(|f| f.get(k.min(f.len().saturating_sub(1))).or(f.first()).cloned()).or(b.sheets.get("49").cloned())) else { return; };
        // (the sheet is a white mask: drawn in getBloodColor, as the native stains)
        let tint = self.bank.as_ref().map_or([0.55, 0.02, 0.02, 1.], |b| b.blood);
        self.decals.push(super::decal::DecalRequest { on_player, root, at: Some((at, face)), meshes: [-1, 0], depth: 0.16, size: [size, 1.],
            offset: Vec3::ZERO, image, tint, life, cull: false, upright: false });
    }
    /// A painted mark that keeps its top up (drips run down) [added art, tools/stage_wound_decals.py]: `bite-marks`,
    /// `mouth-blood`.
    pub fn mark(&mut self, root: Option<Entity>, on_player: bool, at: Vec3, face: Vec3, size: f32, image: &str, life: u16) {
        self.decals.push(super::decal::DecalRequest { on_player, root, at: Some((at, face)), meshes: [-1, 0], depth: 0.12, size: [size, 1.],
            offset: Vec3::ZERO, image: format!("native/effects/{image}.png"), tint: [1., 1., 1., 1.], life, cull: false, upright: true });
    }
    /// Start an emitter at a world point on a model (its stains project onto `root`).
    pub fn burst_on(&mut self, bank: &str, id: u32, at: Vec3, yaw: f32, root: Entity) { self.start(bank, id, Owner::Body(at, yaw, root), None); }
    /// The impact streaks (core 1000) at a hit point [added colour]: the record draws 0.2-0.4 m above its anchor (the
    /// owner's root in the original) in black and white; it is anchored that much lower so the streaks sit on the point,
    /// and tinted `rgb` (blood red on flesh, a warm spark off a guard).
    pub fn impact(&mut self, at: Vec3, yaw: f32, scale: f32, rgb: [f32; 3]) {
        let Some(b) = self.bank.as_ref() else { return; };
        let ems = b.banks.get("core").and_then(|k| k.effects.get("1000")).cloned().unwrap_or_default();
        let owner = Owner::At(at - Vec3::Y * 0.3 * scale, yaw, scale);
        for em in ems {
            // the black half stays dark (its outline), the white half takes the colour
            let dark = em.rgba[0] < 0.5;
            let tint = Some(if dark { [rgb[0] * 0.25, rgb[1] * 0.15, rgb[2] * 0.15, 1.] } else { [rgb[0], rgb[1], rgb[2], 1.] });
            self.active.push(Active { em, age: 0, owner, tint, until: None });
        }
    }
    fn start(&mut self, bank: &str, id: u32, owner: Owner, until: Option<Until>) {
        let Some(b) = self.bank.as_ref() else { return; };
        if std::env::var_os("MW_FX_LOG").is_some() { eprintln!("[fx] {bank}:{id}"); }
        let blood = b.blood;
        let ems = b.banks.get(bank).and_then(|k| k.effects.get(&id.to_string())).cloned().unwrap_or_default();
        // native work flag 0x20 (record +0x08): the draw color is the requester's, here getBloodColor
        for em in ems { let tint = (em.flags[1] & 0x20 != 0).then_some(blood); self.active.push(Active { em, age: 0, owner, tint, until }); }
    }
}

#[derive(Component)]
pub struct Particle { pos: Vec3, vel: Vec3, acc: Vec3, fall: f32, gravity: f32, drag: f32, age: u32, life: u32, floor: Option<f32>, rot: Vec3, spin: Vec3, oriented: bool }

/// The pistol's muzzle blast: Jude's single-shot flash (em0400 EST 31, started by em0400_0105; TUV 128 starburst and
/// smoke), and the spark of a round striking the stage (core EST 12, the guard spark). The flash emitters sit 0.4
/// down Jude's gun bone (0x29); the anchor is lifted by that so the flash lands on the muzzle.
const MUZZLE: [(&str, u32); 1] = [("em0400", 31)];
const MUZZLE_LIFT: f32 = 0.4;
/// Jude's flash is sized for his long revolvers seen from afar; on the pistol half a metre ahead of the camera it is
/// scaled down [inferred], and the impact spark a little.
const MUZZLE_SCALE: f32 = 0.18;
const IMPACT_SCALE: f32 = 0.6;
const IMPACT: u32 = 12;

pub fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/effects/effects.json");
    let bank: Option<Bank> = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok());
    if bank.is_none() { warn!("[fx] no native effects staged: run python -I tools/stage_effects.py"); }
    let images = bank.iter().flat_map(|b| b.pages.iter().chain(b.sheets.values())).map(|p| (p.clone(), assets.load(p.clone()))).collect();
    commands.insert_resource(Fx { bank, images, quads: default(), rng: 0x5eed_b100, active: vec![], last: [(usize::MAX, 0, 0.); 2], decals: vec![], splats: vec![] });
}

/// The owner's anchor (root bone over its position), yaw and floor height.
fn anchor(play: &super::Play, world: &mw_data::collision::CollisionWorld, owner: Owner) -> (Vec3, f32, f32) {
    let (pos, yaw, m, bones, clips) = match owner {
        Owner::At(p, yaw, _) | Owner::Body(p, yaw, _) => return (p, yaw, super::floor(world, p).unwrap_or(p.y - 50.)),
        Owner::Jack => (play.fight.player.pos, play.fight.player.loco.facing, &play.fight.player.motion, &play.fd.jack.bones, &play.fd.jack.clips),
        Owner::Rin => (play.fight.rin.pos, play.fight.rin.facing, &play.fight.rin.motion, &play.fd.rin.bones, &play.fd.rin.clips),
    };
    let root = mw_sim::character::sample_retargeted(bones, &clips[m.clip], m.frame, mw_sim::character::Retarget::NATIVE)[0].translation[1];
    let p = Vec3::from_array(pos);
    (p + Vec3::Y * root, yaw, super::floor(world, p).unwrap_or(p.y))
}

/// Fight events and motion cues of this game frame -> native effects (FixedUpdate, after the fight tick).
pub fn spawn(play: Res<super::Play>, mut fx: ResMut<Fx>) {
    if fx.bank.is_none() { return; }
    let (p, r) = (&play.fight.player, &play.fight.rin);
    // debug: MW_FX_PREVIEW=em0440:110,core:1000 plays those on RinRin every 45 ticks (inspection only)
    if play.tick % 45 == 10 {
        for item in std::env::var("MW_FX_PREVIEW").unwrap_or_default().split(',') {
            if let Some((bank, id)) = item.trim().split_once(':') { if let Ok(id) = id.parse() { fx.start(bank, id, Owner::Rin, None); } }
        }
    }
    // Kreely's pistol: the muzzle blast at the muzzle, a spark where a miss strikes the stage (a hit bleeds through the
    // RinHit reaction above)
    for g in &play.gun_events {
        // a VR round striking the stage (flesh bleeds through its own wound)
        if let super::GunEvent::Impact { at, dir, flesh: false } = *g { fx.start("core", IMPACT, Owner::At(at, dir.x.atan2(dir.z), IMPACT_SCALE), None); }
        if let super::GunEvent::Shot { muzzle, dir, end, kind } = *g {
            let yaw = dir.x.atan2(dir.z);
            for (bank, id) in MUZZLE { fx.start(bank, id, Owner::At(muzzle + Vec3::Y * MUZZLE_LIFT * MUZZLE_SCALE, yaw, MUZZLE_SCALE), None); }
            match (end, kind) {
                (Some(p), super::ShotEnd::Stage) => fx.start("core", IMPACT, Owner::At(p, yaw, IMPACT_SCALE), None),
                // the fan rings: the guard spark at the fan, a second where the ricochet lands
                (Some(p), super::ShotEnd::Fan { bounces, n, hurt }) => {
                    fx.start("core", IMPACT, Owner::At(p, yaw, 1.), None);
                    // a spark at every carom (not where a ricochet strikes the shooter: he bleeds through PlayerHit)
                    let sparks = if hurt { n.saturating_sub(1) } else { n };
                    for b in &bounces[..sparks] { fx.start("core", IMPACT, Owner::At(*b, yaw, IMPACT_SCALE), None); }
                }
                _ => {}
            }
        }
    }
    // RinRin's bombs (cBl0063): the fuse spark (EST 1) at the fuse tip while it burns, the blast (EST 0) on explode
    for (i, b) in play.fight.rin.bombs.iter().enumerate() {
        if b.blast <= 0. && (play.tick + i as u64) % 6 == 0 {
            let tip = Vec3::from_array(b.pos) + Quat::from_array(b.turn) * Vec3::new(0., 0.6, 0.);
            fx.start("bl0063", 1, Owner::At(tip, 0., 1.), None);
        }
    }
    for e in &play.fight.events {
        match *e {
            FightEvent::BombBlast { at } => fx.start("bl0063", 0, Owner::At(Vec3::from_array(at), 0., 1.), None),
            // Kreely's suplex slamming RinRin: her body-hit blood (the grunts bleed in play_street.rs)
            FightEvent::SuplexSlam { on_rin: true, .. } => { for id in [200, 201] { fx.start("em0440", id, Owner::Rin, None); } }
            FightEvent::PlayerHit { damage, hp, .. } => {
                for id in PLAYER_HIT { fx.start("core", id, Owner::Jack, None); }
                if damage > 0 { let p = Vec3::from_array(play.fight.player.pos); fx.splats.push((p, (0.35 + damage as f32 / 60.).min(1.1))); }
                if hp as i32 + damage <= PLAYER_HIT_LOW_HP.1 as i32 { fx.start("core", PLAYER_HIT_LOW_HP.0, Owner::Jack, None); }
            }
            FightEvent::RinHit { damage, hp, .. } if damage > 0 => {
                // her native reaction effects follow; this is the shared spray and the floor pool [added]
                let r = &play.fight.rin;
                let at = Vec3::from_array(r.pos) + Vec3::Y * 1.2;
                fx.gore(at, r.facing + std::f32::consts::PI, damage, damage >= 40, hp <= 0);
                match e { FightEvent::RinHit { reaction, .. } if matches!(*reaction, rs::DM_RIGHT | rs::DM_LEFT) => fx.start("em0440", 111, Owner::Rin, None),
                          FightEvent::RinHit { reaction, .. } if *reaction == rs::DM_BODY => { for id in [200, 201] { fx.start("em0440", id, Owner::Rin, None); } }
                          _ => {} }
            }
            FightEvent::RinHit { reaction, .. } if matches!(reaction, rs::DM_RIGHT | rs::DM_LEFT) => fx.start("em0440", 111, Owner::Rin, None),
            FightEvent::RinHit { reaction, .. } if reaction == rs::DM_BODY => { for id in [200, 201] { fx.start("em0440", id, Owner::Rin, None); } }
            FightEvent::RinGuard { atk } if atk != 0x61 => fx.start("core", 12, Owner::Jack, None),   // a pistol round sparks at her fan instead
            FightEvent::RinState { to, .. } => match to {
                rs::DM_EXPLOSION => fx.start("core", 140, Owner::Rin, Some(Until::RinState(to))),
                rs::DM_ELECTRIC => { for id in [73, 74] { fx.start("core", id, Owner::Rin, Some(Until::RinState(to))); } }
                rs::KAMIKAZE => fx.start("em0440", 61, Owner::Rin, Some(Until::RinState(to))),
                rs::THROW => fx.start("em0440", 60, Owner::Rin, Some(Until::RinState(to))),
                _ => {}
            },
            _ => {}
        }
    }
    // motion sequence op 16 crossed this frame (same crossing rule as the sound cues)
    let actors = [(Owner::Jack, &p.motion, &play.fd.jack.clips, None), (Owner::Rin, &r.motion, &play.fd.rin.clips, Some("em0440"))];
    for (k, (owner, m, clips, own)) in actors.into_iter().enumerate() {
        let (pclip, pstarts, pframe) = fx.last[k];
        let from = if pclip != m.clip || pstarts != m.starts { -1. } else { pframe };
        fx.last[k] = (m.clip, m.starts, m.frame);
        if m.frame <= from { continue; }
        let seq = clips[m.clip].name.replace(".mot", "_00.seq");
        let Some(cues) = play.fd.combat.cues.get(&seq) else { continue; };
        let crossed: Vec<(i16, i16)> = cues.iter().filter(|c| c.op == 16 && c.frame > from && c.frame <= m.frame).map(|c| (c.a, c.c)).collect();
        for (a, c) in crossed {
            let bank = match c { 2 => match own { Some(b) => b, None => continue }, 0 => "core", _ => continue };
            fx.start(bank, a as u16 as u32, owner, Some(Until::Clip(owner, m.clip, m.starts)));
        }
    }
}

/// Unit quad anchored at the pivot (sprite units, y up toward v0).
fn quad(frame: &Frame, pivot: [f32; 2]) -> Mesh {
    let [u0, v0, u1, v1] = frame.uv;
    let pos: Vec<[f32; 3]> = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]].iter().map(|[x, y]| [x - pivot[0], y - (1. - pivot[1]), 0.]).collect();
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; 4]);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]]);
    m.insert_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]));
    m
}

/// Emit and integrate on the 60 Hz effect frame (two frames per 30 Hz tick).
pub fn advance(play: Res<super::Play>, arena: Res<super::Arena>, mut commands: Commands, mut fx: ResMut<Fx>, mut meshes: ResMut<Assets<Mesh>>,
               mut materials: ResMut<Assets<StandardMaterial>>, mut parts: Query<(Entity, &mut Particle, &mut Transform)>) {
    let anchors = [anchor(&play, &arena.0, Owner::Jack), anchor(&play, &arena.0, Owner::Rin)];
    let alive = |u: &Until| match *u {
        Until::RinState(s) => play.fight.rin.state == s,
        Until::Clip(o, clip, starts) => { let m = if o == Owner::Jack { &play.fight.player.motion } else { &play.fight.rin.motion }; m.clip == clip && m.starts == starts }
    };
    for _ in 0..FRAMES_PER_TICK {
        let mut active = std::mem::take(&mut fx.active);
        // killEffect at the end of the starting state; an endless (frames 0) motion effect ends with its motion
        active.retain(|a| match &a.until { Some(u @ Until::RinState(_)) => alive(u), Some(u) if a.em.frames == 0 => alive(u), _ => true });
        for a in &mut active {
            if a.age % (a.em.interval as u32 + 1) == 0 {
                let em = a.em.clone();
                let at = match a.owner { Owner::Jack => anchors[0], Owner::Rin => anchors[1], o @ (Owner::At(..) | Owner::Body(..)) => anchor(&play, &arena.0, o) };
                for _ in 0..em.count { emit(&mut commands, &mut fx, &mut meshes, &mut materials, a, &em, at); }
            }
            a.age += 1;
        }
        active.retain(|a| a.age < if a.em.frames == 0 { ENDLESS_CAP } else { a.em.frames as u32 });
        fx.active.extend(active);
        for (e, mut p, mut t) in &mut parts {
            if p.age >= p.life { continue; }   // despawned in an earlier sub-step of this tick
            // 801FBAF8, dt = 1 frame
            let (v, acc, d, g) = (p.vel, p.acc, p.drag, p.gravity);
            p.pos += v; let f = p.fall; p.pos.y += f; p.fall += g;
            p.vel += acc; p.vel *= d; p.acc *= d;
            if let Some(y) = p.floor { if p.pos.y < y { p.pos.y = y; p.vel = Vec3::ZERO; p.acc = Vec3::ZERO; p.fall = 0.; p.gravity = 0.; } }
            let s = p.spin; p.rot += s;
            p.age += 1;
            if p.age >= p.life { commands.entity(e).despawn(); continue; }
            t.translation = p.pos;
        }
    }
}

fn emit(commands: &mut Commands, fx: &mut Fx, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, a: &Active, em: &Emitter,
        (at, yaw, floor): (Vec3, f32, f32)) {
    // not sprites, and not ported: 21 a full-screen 2D flash (640 x 360 screen units), 23 a ribbon trail that follows the
    // swinging limb, 26 a controller record with no image; drawn as world quads they were large white slabs
    if matches!(em.kind, 21 | 23 | 26) { return; }
    // the one-frame metre-wide white flash that opens the blood records (core 1005 / 1020: tex 6, life 1-2): drawn as a
    // world quad it whites out the whole body, black and white, above the hit
    if em.kind == 0 && em.tuv.is_none() && em.tex == 6 && em.life <= 2 && em.size[0] >= 0.9 { return; }
    // type 24: a stain on the owner's model, not a sprite (characters and grunts; a bare world point has no model)
    if em.kind == 24 {
        let (Some(d), Some(bank)) = (em.decal.as_ref(), fx.bank.as_ref()) else { return; };
        let (on_player, root) = match a.owner { Owner::Jack => (true, None), Owner::Rin => (false, None), Owner::Body(_, _, r) => (false, Some(r)), Owner::At(..) => return };
        let t = em.tex.to_string();
        let image = bank.sheet_frames.get(&t).and_then(|f| f.get(em.frame_start as usize).or(f.first())).or(bank.sheets.get(&t)).cloned();
        let Some(image) = image else { return; };
        let w = em.size[0] + em.size_random[0] * fx.rand();
        let ratio = if em.size[1] == 0. { 1. } else { em.size[1] };
        fx.decals.push(super::decal::DecalRequest { on_player, root, at: None, meshes: d.meshes, depth: d.depth, size: [w, ratio], offset: Vec3::from_array(em.offset),
            image, tint: a.tint.unwrap_or(em.rgba), life: em.life, cull: d.cull != 0, upright: false });
        return;
    }
    let pick = fx.rand();
    let Some(bank) = fx.bank.as_ref() else { return; };
    let (image, frame) = match em.tuv {
        Some(g) => {
            let Some(frames) = bank.sprites.get(&g.to_string()) else { return; };
            // 8008F1EC: work flag 0x200 (record +0x04) picks a random frame, otherwise the start frame
            let k = if em.flags[0] & 0x200 != 0 { ((pick * 0.5 + 0.5) * frames.len() as f32) as usize } else { em.frame_start as usize };
            let f = frames[k.min(frames.len() - 1)].clone();
            (bank.pages[f.page].clone(), f)
        }
        None => match bank.sheets.get(&em.tex.to_string()) { Some(p) => (p.clone(), Frame { page: 0, uv: [0., 0., 1., 1.] }), None => return },
    };
    let Some(img) = fx.images.get(&image).cloned() else { return; };
    let key = (format!("{image}{:?}", frame.uv), em.pivot.map(f32::to_bits));
    let mesh = match fx.quads.get(&key) { Some(m) => m.clone(), None => { let m = meshes.add(quad(&frame, em.pivot)); fx.quads.insert(key, m.clone()); m } };
    let local = Quat::from_rotation_y(yaw);
    let offset = fx.vec(em.offset, em.offset_random);
    // 801FAC58: per-axis random velocity, or a random direction of length +0x194 when that is set
    let vel = if em.speed_random != 0. { Vec3::from_array(em.velocity) + fx.vec([0.; 3], [1.; 3]).normalize_or_zero() * em.speed_random }
              else { fx.vec(em.velocity, em.velocity_random) };
    let rot = fx.vec(em.rotation, em.rotation_random).to_array().map(f32::to_radians);
    let spin = fx.vec(em.spin, em.spin_random).to_array().map(f32::to_radians);
    // +0xC4 width; +0xC0 scales the sprite's own aspect (0 = 1): 2 cm droplets, 10 cm spikes [inferred: the draw path
    // is not decoded; reading +0xC0 as a height gave 1 m droplets and zero-height 1005 sprites]
    let [u0, v0, u1, v1] = frame.uv;
    let aspect = if u1 != u0 { ((v1 - v0) / (u1 - u0)).abs() } else { 1. };
    let w = em.size[0] + em.size_random[0] * fx.rand();
    let ratio = if em.size[1] == 0. { 1. } else { em.size[1] + em.size_random[1] * fx.rand() };
    let h = w * aspect * ratio;
    let life = (em.life as f32 + em.life_random as f32 * fx.rand()).max(1.) as u32;
    let c = a.tint.unwrap_or(em.rgba);
    // a world-point emitter can be resized as a whole (Jude's gun flash on Kreely's smaller pistol)
    let k = if let Owner::At(_, _, k) = a.owner { k } else { 1. };
    let (w, h, offset, vel) = (w * k, h * k, offset * k, vel * k);
    let mut pos = at + local * offset;
    // a still, lasting quad laid flat (rotation x 90, no velocity, gravity or acceleration: em0440 200's 300-frame
    // pool) lies on the floor under its owner, not at the anchor (the root bone, hip high) [inferred: the native
    // anchor for these is not decoded]
    let still = em.velocity == [0.; 3] && em.velocity_random == [0.; 3] && em.gravity == 0. && em.accel == [0.; 3];
    if still && em.rotation[0] == 90. && em.life >= 100 { pos.y = floor + 0.01; }
    commands.spawn((Mesh3d(mesh), MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgba(c[0], c[1], c[2], c[3]), base_color_texture: Some(img),
            unlit: true, alpha_mode: AlphaMode::Blend, double_sided: true, cull_mode: None, ..default() })),
        Transform::from_translation(pos).with_scale(Vec3::new(w.max(1e-4), h.max(1e-4), 1.)), bevy::light::NotShadowCaster,
        Particle { pos, vel: local * vel, acc: local * Vec3::from_array(em.accel), fall: 0., gravity: em.gravity, drag: em.drag, age: 0, life,
                   floor: em.floor.then_some(floor), rot: Vec3::from_array(rot), spin: Vec3::from_array(spin),
                   oriented: em.rotation_random[0] != 0. || em.rotation_random[1] != 0. }));
}

/// Camera-facing sprites roll by their z rotation; oriented ones keep their full 3D rotation.
pub fn billboard(cam: Query<&Transform, (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>, Without<Particle>)>, mut parts: Query<(&Particle, &mut Transform)>) {
    let Ok(c) = cam.single() else { return; };
    for (p, mut t) in &mut parts {
        t.rotation = if p.oriented { Quat::from_euler(EulerRot::XYZ, p.rot.x, p.rot.y, p.rot.z) } else { c.rotation * Quat::from_rotation_z(p.rot.z) };
    }
}
