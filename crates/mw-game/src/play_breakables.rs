//! Elise's church (c303): its breakable ScrObjs (cBaseScrObj; research in the scratchpad pews-research, addresses here).
//! - **The nave pillars, bl005c** (ObjTbl N001..N006 at x = -8 / +8, z = 0 / 10 / 20, and the two misnamed N7 / N8 at
//!   z = -10): the room's scenery draws only their remains (the stump and arch, D9_M00); the object adds the shaft (D0_M00).
//!   Mesh tokens (parser 801B97B0): HP 2 at D0 (HP0005 marks the D0 meshes, the part HP is the first D9 mesh's), so any
//!   player blow breaks one (hit handler 801BC02C, object damage from plem_common.ini column 3 >= 50). The break: the five
//!   shaft pieces (D9_M01..M05) fly off along the blow (801BFC60 [inferred speeds: the Havok masses are unread]), EST 1 at
//!   the blow, SE 4209:205; each piece's first landing EST 800 and SE 4209:206 [the EST / SE pairing inferred by id]; 120
//!   frames on the object fades (AD04: alpha -8 a frame) and goes; the stump and arch stay, and so do the walls.
//!   The coffin controller's update fn_80076054 plays **voice 0xA2** the first time each of N001..N006 breaks (its slots
//!   +0x70..+0x84; N007 / N008 are looked up by names ObjTbl does not use, so the z = -10 pair breaks silently).
//! - **The pews, bl005d** (ObjTbl records 26..35): HP 2 then 1 (chair_D0_M00_HP0002 / chair_D9_M00_HP0001): a blow breaks
//!   one into its broken shape (EST 1), which fades 3 s later (AD03). No voice, no wall.
//! Elise and her bats never break them (no enemy code reaches the ScrObj hit functions).
use bevy::prelude::*;

/// (position, yaw, voiced slot) of the pillars: N001..N006, then N7 / N8.
const PILLARS: [([f32; 3], f32, bool); 8] = [
    ([-8., 0., 0.], 0., true), ([-8., 0., 10.], 0., true), ([-8., 0., 20.], 0., true),
    ([8., 0., 0.], -std::f32::consts::PI, true), ([8., 0., 10.], -std::f32::consts::PI, true), ([8., 0., 20.], -std::f32::consts::PI, true),
    ([8., 0., -10.], -std::f32::consts::PI, false), ([-8., 0., -10.], 0., false),
];
/// The pews (ObjTbl records 26..35).
const PEWS: [([f32; 3], f32); 10] = [
    ([4., 0., 7.95], -3.1), ([-4.23, 0., 18.93], -3.1), ([4., 0., 17.95], -3.1), ([-4., 0., 10.78], -3.1), ([-3.86, 0., 11.95], -3.1),
    ([3.82, 0., 11.43], -3.1), ([-4., 0., 20.36], -3.1), ([3.82, 0., 19.95], -3.1), ([-3.82, 0., 21.57], -3.1), ([4., 0., 21.95], -3.1),
];
const GRAVITY: f32 = 9.8 / 900.;
/// AD04: frames after the break before the fade; the fade (255 / 8 frames).
const PILLAR_STAY: f32 = 120.;
const FADE: f32 = 31.;
/// AD03: the broken pew's 3 s.
const PEW_STAY: f32 = 90.;

#[derive(Clone, Copy, PartialEq)]
enum Kind { Pillar, Pew }
struct Piece { node: Entity, variant: usize, pos: Vec3, vel: Vec3, rot: Quat, spin: Vec3, landed: u8 }
struct Breakable {
    /// the source and the added-color model (tools/stage_object_color.py), shown by the look
    kind: Kind, roots: [Entity; 2], pos: Vec3, yaw: f32, voiced: bool, slot: usize,
    nodes: Vec<(String, Entity, usize)>, bound: bool,
    /// frames since it broke (None: whole)
    broken: Option<f32>, hit_by: u32, pieces: Vec<Piece>, faded: bool, gone: bool,
}

#[derive(Resource, Default)]
pub struct Breakables {
    on: bool, spawned: bool, list: Vec<Breakable>,
    /// the slots whose break already played 0xA2 (the controller's +0xA4 bits)
    spoken: u32,
    /// sounds this frame (block, id, where) for play.rs cue_sounds
    pub sfx: Vec<(u32, u32, Vec3)>,
}

pub struct BreakablesPlugin;
impl Plugin for BreakablesPlugin {
    fn build(&self, app: &mut App) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/objects");
        let on = super::elise::on() && root.join("bl005c.glb").exists() && std::env::var("MW_BREAKABLES").as_deref() != Ok("0");
        app.insert_resource(Breakables { on, ..default() })
            .add_systems(Update, (spawn, bind, look, fade))
            .add_systems(FixedUpdate, step.after(super::tick).before(super::announce).run_if(super::pause::running).run_if(super::intro::fight_running).run_if(super::cutin::world_running));
    }
}

fn spawn(mut b: ResMut<Breakables>, assets: Res<AssetServer>, mut commands: Commands) {
    if !b.on || b.spawned { return; }
    b.spawned = true;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let mut add = |file: &str, kind: Kind, pos: Vec3, yaw: f32, voiced: bool, slot: usize, list: &mut Vec<Breakable>| {
        let color = file.replace(".glb", "-color.glb");
        let files = [file.to_string(), if dir.join(&color).exists() { color } else { file.to_string() }];
        let roots = files.map(|f| commands.spawn((super::RoomProp, super::WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(f))),
                                   Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(yaw)), Visibility::Hidden)).id());
        list.push(Breakable { kind, roots, pos, yaw, voiced, slot, nodes: vec![], bound: false, broken: None, hit_by: 0, pieces: vec![], faded: false, gone: false });
    };
    let mut list = vec![];
    for (k, (p, yaw, voiced)) in PILLARS.iter().enumerate() { add("native/objects/bl005c.glb", Kind::Pillar, Vec3::from_array(*p), *yaw, *voiced, k, &mut list); }
    if std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/objects/bl005d.glb").exists() {
        for (k, (p, yaw)) in PEWS.iter().enumerate() { add("native/objects/bl005d.glb", Kind::Pew, Vec3::from_array(*p), *yaw, false, 100 + k, &mut list); }
    }
    b.list = list;
}

/// Update: find each object's part nodes once its scene is in; whole, only D0 shows (the remains are the scenery's).
fn bind(mut b: ResMut<Breakables>, children: Query<&Children>, names: Query<&Name>, meshes: Query<(), With<Mesh3d>>, mut vis: Query<&mut Visibility>) {
    if !b.on { return; }
    for o in b.list.iter_mut().filter(|o| !o.bound && !o.gone) {
        let mut found = vec![];
        let mut per = [0; 2];
        for k in 0..2 {
            for e in children.iter_descendants(o.roots[k]) {
                // (the part nodes only: "D0_M00", "D9_M01"; their primitives carry longer names)
                if meshes.contains(e) { continue; }
                if let Ok(n) = names.get(e) { let n = n.as_str(); if n.len() == 6 && (n.starts_with("D0_M") || n.starts_with("D9_M")) { found.push((n.to_string(), e, k)); per[k] += 1; } }
            }
        }
        if per[0] == 0 || per[1] == 0 { continue; }
        for (n, e, _) in &found { if let Ok(mut v) = vis.get_mut(*e) { *v = if n.starts_with("D0_") { Visibility::Visible } else { Visibility::Hidden }; } }
        o.nodes = found; o.bound = true;
    }
}

/// FixedUpdate: the player's blows break them; the pieces fly and land; then the fade.
fn step(mut b: ResMut<Breakables>, mut play: ResMut<super::Play>, arena: Res<super::Arena>, mut fx: ResMut<super::fx::Fx>,
        mut tf: Query<&mut Transform>, mut vis: Query<&mut Visibility>, globals: Query<&GlobalTransform>, mut commands: Commands, mut rng: Local<u32>) {
    b.sfx.clear();
    if !b.on { return; }
    let mut rand = || { *rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (*rng >> 8) as f32 / (1 << 24) as f32 };
    // inspection: MW_BREAK_TEST=n stands the player 1.3 m from object n (0..7 the pillars, 8.. the pews), facing it, at tick 50
    if play.tick == 50 { if let Some(n) = std::env::var("MW_BREAK_TEST").ok().and_then(|v| v.parse::<usize>().ok()).filter(|n| *n < b.list.len()) {
        let o = &b.list[n];
        let out = if o.kind == Kind::Pillar { Vec3::new(if o.pos.x < 0. { 1. } else { -1. }, 0., 0.) * 1.6 } else { Quat::from_rotation_y(o.yaw) * Vec3::NEG_Z * 1.1 };
        let at = o.pos + out;
        play.fight.player.pos = [at.x, 0., at.z]; play.fight.player.loco.facing = (-out.x).atan2(-out.z); play.cam_yaw = play.fight.player.loco.facing;
    } }
    let strike = play.fight.player_strike();
    let pl = Vec3::from_array(play.fight.player.pos);
    let bb = &mut *b;
    for o in bb.list.iter_mut() {
        if o.gone || !o.bound { continue; }
        if o.broken.is_none() {
            let Some((hit, serial)) = strike.as_ref() else { continue; };
            if *serial == o.hit_by { continue; }
            let c = Vec3::from_array(hit.center);
            // the blow in the object's own frame against its body (pillar shaft x +-1, z +-2, 0..6.5 m; pew 3.9 x 0.8 x 1.2)
            let local = Quat::from_rotation_y(-o.yaw) * (c - o.pos);
            let r = hit.radius;
            let inside = match o.kind {
                Kind::Pillar => local.x.abs() < 1. + r && local.z.abs() < 2. + r && local.y > -r && local.y < 6.5 + r,
                Kind::Pew => local.x.abs() < 1.94 + r && local.z > -0.52 - r && local.z < 0.3 + r && local.y > -r && local.y < 1.8 + r,
            };
            if std::env::var_os("MW_BREAK_LOG").is_some() && local.length() < 4. { eprintln!("[break] tick {} blow at local {:.2} r {:.2} on {} -> {}", play.tick, local, r, o.slot, inside); }
            if !inside { continue; }
            o.hit_by = *serial;
            o.broken = Some(0.);
            let away = Vec3::new(o.pos.x - pl.x, 0., o.pos.z - pl.z).normalize_or(Quat::from_rotation_y(o.yaw) * Vec3::Z);
            let bank = if o.kind == Kind::Pillar { "bl005c" } else { "bl005d" };
            fx.burst(bank, 1, c, away.x.atan2(away.z), 1.);
            for (n, e, _) in &o.nodes {
                if let Ok(mut v) = vis.get_mut(*e) {
                    // the pillar: the shaft goes, its pieces fly; the pew: the whole one swaps for the broken one
                    *v = match (o.kind, n.as_str()) { (_, s) if s.starts_with("D0_") => Visibility::Hidden, (Kind::Pillar, "D9_M00") => Visibility::Hidden, _ => Visibility::Visible };
                }
            }
            if o.kind == Kind::Pillar {
                bb.sfx.push((4209, 205, c));
                for (n, e, variant) in &o.nodes {
                    if !n.starts_with("D9_M0") || n == "D9_M00" { continue; }
                    let Ok(g) = globals.get(*e) else { continue; };
                    let (_, rot, at) = g.to_scale_rotation_translation();
                    let vel = away * (3. + 2. * rand()) / 30. + Vec3::Y * (1. + 2. * rand()) / 30. + Vec3::new(rand() - 0.5, 0., rand() - 0.5) * 2. / 30.;
                    let spin = Vec3::new(rand() - 0.5, rand() - 0.5, rand() - 0.5) * 0.4;
                    // (the piece leaves the object's hierarchy: it flies in the world)
                    commands.entity(*e).remove::<ChildOf>();
                    if let Ok(mut t) = tf.get_mut(*e) { t.translation = at; t.rotation = rot; }
                    o.pieces.push(Piece { node: *e, variant: *variant, pos: at, vel, rot, spin, landed: 0 });
                }
                // the coffin controller's line, once per voiced pillar
                if o.voiced && bb.spoken & (1 << o.slot) == 0 {
                    bb.spoken |= 1 << o.slot;
                    play.fight.events.push(mw_sim::fight::FightEvent::Voice(0xA2));
                }
            }
            if std::env::var_os("MW_BREAK_LOG").is_some() { eprintln!("[break] tick {} {} {} at {:.1}: {} pieces of {:?}", play.tick, bank, o.slot, o.pos, o.pieces.len(), o.nodes.iter().map(|n| n.0.as_str()).collect::<Vec<_>>()); }
            continue;
        }
        let t = o.broken.unwrap() + 1.;
        o.broken = Some(t);
        for p in o.pieces.iter_mut() {
            if p.landed >= 3 { continue; }
            p.vel.y -= GRAVITY;
            p.pos += p.vel;
            p.rot = Quat::from_scaled_axis(p.spin) * p.rot;
            let floor = super::floor(&arena.0, p.pos + Vec3::Y * 0.5).unwrap_or(0.);
            if p.pos.y < floor + 0.2 {
                p.pos.y = floor + 0.2;
                // (each piece is there twice, source and color: one lands audibly)
                if p.landed == 0 && p.variant == 0 { fx.burst("bl005c", 800, p.pos, 0., 1.); bb.sfx.push((4209, 206, p.pos)); }
                p.landed += 1;
                p.vel = Vec3::new(p.vel.x * 0.5, -p.vel.y * 0.3, p.vel.z * 0.5); p.spin *= 0.5;
                if p.landed >= 3 { p.vel = Vec3::ZERO; }
            }
            if let Ok(mut tr) = tf.get_mut(p.node) { tr.translation = p.pos; tr.rotation = p.rot; }
        }
        let stay = if o.kind == Kind::Pillar { PILLAR_STAY } else { PEW_STAY };
        if t > stay + FADE {
            o.gone = true;
            for p in &o.pieces { commands.entity(p.node).despawn(); }
            for r in o.roots { commands.entity(r).despawn(); }
        }
    }
}

/// Update: the source or the colored model by the look (C), pieces included.
pub fn look(b: Res<Breakables>, play: Res<super::Play>, mut vis: Query<&mut Visibility>) {
    if !b.on { return; }
    let shown = play.colored as usize;
    for o in b.list.iter().filter(|o| !o.gone) {
        for (k, r) in o.roots.iter().enumerate() { if let Ok(mut v) = vis.get_mut(*r) { let want = if k == shown { Visibility::Visible } else { Visibility::Hidden }; if *v != want { *v = want; } } }
        for p in &o.pieces { if let Ok(mut v) = vis.get_mut(p.node) { let want = if p.variant == shown { Visibility::Visible } else { Visibility::Hidden }; if *v != want { *v = want; } } }
    }
}

/// Update: the fade after the stay (AD03 / AD04): the parts' materials as blended copies, their alpha down to 0.
fn fade(mut b: ResMut<Breakables>, children: Query<&Children>, parts: Query<&MeshMaterial3d<StandardMaterial>>,
        mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    if !b.on { return; }
    for o in b.list.iter_mut().filter(|o| !o.gone) {
        let Some(t) = o.broken else { continue; };
        let stay = if o.kind == Kind::Pillar { PILLAR_STAY } else { PEW_STAY };
        if t < stay { continue; }
        let alpha = (1. - (t - stay) / FADE).clamp(0., 1.);
        let mut ents: Vec<Entity> = o.pieces.iter().map(|p| p.node).collect();
        ents.extend(o.roots);
        for root in ents {
            for e in std::iter::once(root).chain(children.iter_descendants(root)) {
                let Ok(m) = parts.get(e) else { continue; };
                if !o.faded {
                    let Some(mut copy) = materials.get(&m.0).cloned() else { continue; };
                    copy.alpha_mode = AlphaMode::Blend; copy.base_color.set_alpha(alpha);
                    commands.entity(e).insert(MeshMaterial3d(materials.add(copy)));
                } else if let Some(mut mat) = materials.get_mut(&m.0) { mat.base_color.set_alpha(alpha); }
            }
        }
        o.faded = true;
    }
}
