//! Added, not original: RinRin's secondary motion. The em0440 motions never move these bones (her breast bones keep
//! constant keys; the skirt and hair-ornament chains are untracked), so they ride rigidly in the original.
//! - Jiggle: breasts (bones 25, 26) and buttocks (bones 200, 201, added by tools/stage_rin_jiggle.py) use the KOF2006
//!   "mode 0" inertia controller as Punchout_Source runtime/src/secondary.rs ports it: the bone's animated motion, in
//!   its own frame, drives an angular spring clamped to a cone; the angles post-multiply the animated rotation.
//! - Cloth: the four skirt chains (31-36, 37-42, 43-49, 50-56, under the hips) and both hair tails with their dangling
//!   ornaments (60-69, 70-79, under the head) are Verlet ropes as Punchout_Source runtime/src/whip.rs: each point is
//!   pulled toward its animated position, keeps its segment lengths, and is pushed out of body capsules; each bone is
//!   then turned from its animated direction to the simulated one. The scarf's back loop rides two short ropes
//!   (210-212 under the right clavicle, 213-215 under the left, added by tools/stage_rin_jiggle.py).
//! Runs after the pose and before transform propagation, in world space, so RinRin's own movement swings everything.
//! The same springs ride the other bosses (`Spec` per actor; their modules ask for them with `SpringRequest` once their
//! joints are bound) [added, as RinRin's; the original's own cloth (cByCloth, setClothParam 800788CC) and hair (cHair)
//! parameters are not used]:
//! - Elise (em0630): breasts 25 / 26; the cape down her back (42-46, under the hips); her two long hair tails (50-54,
//!   55-59) and two side locks (60-62, 63-65) under the head.
//! - The oirans (em0960): breasts 25 / 26; their two hair tails (30-32, 33-35).
//! - Mathilda (em0610): breasts 25 / 26.
//! - The Baron (em0623): the chain off his chest (50-52); his coat is mesh cloth in the original, with no bones here.
use bevy::prelude::*;
use std::collections::HashMap;

/// Jiggle response (secondary.rs Response): angular gains per driver axis, cone limit (deg), damping, spring.
#[derive(Clone, Copy)]
struct Response { gains: [Vec3; 3], limit: Vec3, damping: f32, spring: f32 }
const CHEST: Response = Response { gains: [Vec3::new(0., 0., -0.2625), Vec3::new(0.9, 0., 0.), Vec3::new(-0.6, 0.1875, 0.)],
                                   limit: Vec3::new(11., 6., 5.), damping: 0.84, spring: 0.16 };
/// The boss women's (Elise, the oirans, Mathilda): their breast bones sit close behind the surface and they mostly hover
/// or stand, so the same chest response barely showed; a stronger, looser one [added].
const BOUNCY: Response = Response { gains: [Vec3::new(0., 0., -0.6), Vec3::new(2.3, 0., 0.), Vec3::new(-1.5, 0.45, 0.)],
                                    limit: Vec3::new(20., 10., 8.), damping: 0.88, spring: 0.11 };
const BUTT: Response = Response { gains: [Vec3::new(0., 0., -0.2), Vec3::new(0.75, 0., 0.), Vec3::new(-0.5, 0.15, 0.)],
                                  limit: Vec3::new(8., 5., 5.), damping: 0.86, spring: 0.14 };

/// One rope: bone ids from the attached root to the tip, pull stiffness at the root and at the tip, and how far a point
/// may stray from its animated position.
struct ChainSpec { bones: &'static [i16], stiff_root: f32, stiff_tip: f32, max_offset: f32 }
const CHAINS: [ChainSpec; 8] = [
    ChainSpec { bones: &[31, 32, 33, 34, 35, 36], stiff_root: 420., stiff_tip: 160., max_offset: 0.22 },
    ChainSpec { bones: &[37, 38, 39, 40, 41, 42], stiff_root: 420., stiff_tip: 160., max_offset: 0.22 },
    ChainSpec { bones: &[43, 44, 45, 46, 47, 48, 49], stiff_root: 420., stiff_tip: 140., max_offset: 0.25 },
    ChainSpec { bones: &[50, 51, 52, 53, 54, 55, 56], stiff_root: 420., stiff_tip: 140., max_offset: 0.25 },
    ChainSpec { bones: &[60, 61, 62, 63, 64, 65, 66, 67, 68, 69], stiff_root: 900., stiff_tip: 45., max_offset: 0.4 },
    ChainSpec { bones: &[70, 71, 72, 73, 74, 75, 76, 77, 78, 79], stiff_root: 900., stiff_tip: 45., max_offset: 0.4 },
    ChainSpec { bones: &[210, 211, 212], stiff_root: 300., stiff_tip: 90., max_offset: 0.15 },
    ChainSpec { bones: &[213, 214, 215], stiff_root: 300., stiff_tip: 90., max_offset: 0.15 },
];
/// Bones added to the GLB (tools/stage_rin_jiggle.py) with their parent bone ids: scarf chains. Found by name; the pose
/// never writes them, so they are restored to their bind transform each frame before the springs act.
const EXTRAS: [(i16, i16); 6] = [(210, 5), (211, 210), (212, 211), (213, 10), (214, 213), (215, 214)];
/// Body capsules (bone id to bone id, radius): torso, head, thighs, shins, upper arms.
const CAPSULES: [(i16, i16, f32); 9] = [(15, 2, 0.13), (2, 3, 0.11), (4, 4, 0.12), (16, 17, 0.085), (20, 21, 0.085),
                                        (17, 18, 0.06), (21, 22, 0.06), (7, 8, 0.055), (12, 13, 0.055)];
const GRAVITY: f32 = 9.81;
const RETAIN: f32 = 0.94;   // velocity kept per substep (whip.rs)
const SUBSTEP: f32 = 1. / 120.;

struct Jiggle { bone: Entity, response: Response, angles: Vec3, velocity: Vec3, prev: Option<Vec3>, base: Option<Quat> }
struct Rope { joints: Vec<usize>, points: Vec<Vec3>, previous: Vec<Vec3>, stiff: Vec<f32>, max_offset: f32 }

/// What springs an actor carries: jiggling bones, ropes, extra bones (added to its GLB) and body capsules.
pub struct Spec { jiggles: &'static [(i16, Response)], chains: &'static [ChainSpec], extras: &'static [(i16, i16)], capsules: &'static [(i16, i16, f32)],
                  /// named jiggles (bones the GLB adds, e.g. RinRin's buttocks) and the bone they hang from
                  named: &'static [&'static str], named_parent: i16 }
const RIN: Spec = Spec { jiggles: &[(25, CHEST), (26, CHEST)], chains: &CHAINS, extras: &EXTRAS, capsules: &CAPSULES,
                         named: &["role_0_bone200", "role_0_bone201"], named_parent: 15 };
const ELISE: Spec = Spec { jiggles: &[(25, BOUNCY), (26, BOUNCY)], extras: &[], capsules: &CAPSULES, named: &[], named_parent: 15, chains: &[
    ChainSpec { bones: &[42, 43, 44, 45, 46], stiff_root: 380., stiff_tip: 110., max_offset: 0.3 },
    ChainSpec { bones: &[50, 51, 52, 53, 54], stiff_root: 900., stiff_tip: 50., max_offset: 0.3 },
    ChainSpec { bones: &[55, 56, 57, 58, 59], stiff_root: 900., stiff_tip: 50., max_offset: 0.3 },
    ChainSpec { bones: &[60, 61, 62], stiff_root: 700., stiff_tip: 120., max_offset: 0.15 },
    ChainSpec { bones: &[63, 64, 65], stiff_root: 700., stiff_tip: 120., max_offset: 0.15 },
] };
const OIRAN: Spec = Spec { jiggles: &[(25, BOUNCY), (26, BOUNCY)], extras: &[], capsules: &CAPSULES, named: &[], named_parent: 15, chains: &[
    ChainSpec { bones: &[30, 31, 32], stiff_root: 700., stiff_tip: 90., max_offset: 0.15 },
    ChainSpec { bones: &[33, 34, 35], stiff_root: 700., stiff_tip: 90., max_offset: 0.15 },
] };
const MATHILDA: Spec = Spec { jiggles: &[(25, BOUNCY), (26, BOUNCY)], extras: &[], capsules: &CAPSULES, named: &[], named_parent: 15, chains: &[] };
const BARON: Spec = Spec { jiggles: &[], extras: &[], capsules: &CAPSULES, named: &[], named_parent: 15, chains: &[
    ChainSpec { bones: &[50, 51, 52], stiff_root: 600., stiff_tip: 150., max_offset: 0.12 },
] };
/// The vampire zako (tools/vampzako): breasts 250 / 251 on the chest, buttocks role_0_bone200 / 201 on the hips, two
/// hair ropes 260-262 / 263-265 from the head; all bones the build adds to her GLB.
const VAMP: Spec = Spec { jiggles: &[(250, CHEST), (251, CHEST)], capsules: &CAPSULES, named: &["role_0_bone200", "role_0_bone201"], named_parent: 15,
    // (her hair rides on her head: no hair ropes, by request)
    extras: &[(250, 2), (251, 2)], chains: &[] };
fn spec(actor: &str) -> Option<&'static Spec> {
    Some(match actor { "vampzako" => &VAMP, "em0440" => &RIN, "em0630" => &ELISE, "em0960" => &OIRAN, "em0610" => &MATHILDA, "em0623" => &BARON, _ => return None })
}

/// A module's request for springs on an actor root it poses itself: its skeleton and joints (bones order, then the
/// root node, as its pose writes them) and which actor's `Spec`.
#[derive(Component)]
pub struct SpringRequest { pub bones: Vec<mw_data::Bone>, pub joints: Vec<Entity>, pub actor: &'static str }

/// Spring state on a character root.
#[derive(Component)]
pub struct Springs { actor: &'static str, bones: Vec<mw_data::Bone>, joints: Vec<Entity>, named_parent: i16,
                     jiggles: Vec<Jiggle>, ropes: Vec<Rope>, capsules: Vec<(usize, usize, f32)>, carry: f32, last_root: Option<Vec3>,
                     /// every simulated node: the skeleton (bones order) then the extras, with parents in the same numbering
                     nodes: Vec<Entity>, parent: Vec<Option<usize>>, extras: Vec<(usize, Transform)> }

/// Build the spring state once RinRin's joints are bound (the butt bones are found by name), and for every actor a
/// module asked springs for.
pub fn bind(play: Res<super::Play>, chars: Query<(Entity, &super::Character), Without<Springs>>, requests: Query<(Entity, &SpringRequest), Without<Springs>>,
            children: Query<&Children>, names: Query<&Name>, mut commands: Commands) {
    for (root, ch) in &chars {
        if ch.who != super::Who::Rin { continue; }
        let Some(joints) = &ch.joints else { continue; };
        let mut rig = build(root, &play.fd.rin.bones, joints, &RIN, &children, &names); rig.actor = "em0440";
        commands.entity(root).insert(rig);
    }
    for (root, req) in &requests {
        let Some(sp) = spec(req.actor) else { continue; };
        let mut rig = build(root, &req.bones, &req.joints, sp, &children, &names); rig.actor = req.actor;
        if std::env::var_os("MW_SPRINGS_LOG").is_some() { eprintln!("[springs] {}: {} jiggles, {} ropes, {} extras", req.actor, rig.jiggles.len(), rig.ropes.len(), rig.extras.len()); }
        commands.entity(root).insert(rig).remove::<SpringRequest>();
    }
}

fn build(root: Entity, bones: &[mw_data::Bone], joints: &[Entity], sp: &Spec, children: &Query<&Children>, names: &Query<&Name>) -> Springs {
    {
        let mut index: HashMap<i16, usize> = bones.iter().enumerate().map(|(i, b)| (b.id, i)).collect();
        let mut nodes: Vec<Entity> = joints[..bones.len()].to_vec();
        let mut parent: Vec<Option<usize>> = bones.iter().map(|b| (b.parent >= 0).then_some(b.parent as usize)).collect();
        let mut extras = vec![];
        let named: HashMap<String, Entity> = children.iter_descendants(root).filter_map(|d| names.get(d).ok().map(|n| (n.as_str().to_string(), d))).collect();
        for &(id, parent_id) in sp.extras {
            let (Some(&e), Some(&p)) = (named.get(&format!("role_0_bone{id}")), index.get(&parent_id)) else { continue; };
            index.insert(id, nodes.len());
            extras.push((nodes.len(), Transform::IDENTITY));
            nodes.push(e); parent.push(Some(p));
        }
        let mut jiggles: Vec<Jiggle> = sp.jiggles.iter()
            .filter_map(|(id, r)| index.get(id).map(|&i| Jiggle { bone: nodes[i], response: *r, angles: Vec3::ZERO, velocity: Vec3::ZERO, prev: None, base: None }))
            .collect();
        for d in children.iter_descendants(root) {
            if let Ok(n) = names.get(d) {
                if sp.named.contains(&n.as_str()) {
                    jiggles.push(Jiggle { bone: d, response: BUTT, angles: Vec3::ZERO, velocity: Vec3::ZERO, prev: None, base: None });
                }
            }
        }
        let ropes = sp.chains.iter().filter_map(|c| {
            let js: Option<Vec<usize>> = c.bones.iter().map(|id| index.get(id).copied()).collect();
            let js = js?;
            let n = js.len();
            let stiff = (0..n).map(|i| c.stiff_root + (c.stiff_tip - c.stiff_root) * i as f32 / (n - 1).max(1) as f32).collect();
            Some(Rope { joints: js, points: vec![], previous: vec![], stiff, max_offset: c.max_offset })
        }).collect();
        let capsules = sp.capsules.iter().filter_map(|(a, b, r)| Some((*index.get(a)?, *index.get(b)?, *r))).collect();
        let _ = root;
        Springs { actor: "", bones: bones.to_vec(), joints: joints.to_vec(), named_parent: sp.named_parent, jiggles, ropes, capsules, carry: 0., last_root: None, nodes, parent, extras }
    }
}

/// World transform of every node (skeleton in bones order, then the extras) from the local transforms the pose wrote.
fn world_pose(root: Entity, joints: &[Entity], rig: &Springs, transforms: &Query<&mut Transform>, parents: &Query<&ChildOf>) -> Option<Vec<Transform>> {
    let origin = *joints.last()?;
    // origin: compose its ancestors up to the character root
    let mut chain = vec![origin];
    let mut e = origin;
    while e != root { e = parents.get(e).ok()?.parent(); chain.push(e); }
    let mut origin_world = Transform::IDENTITY;
    for &a in chain.iter().rev() { origin_world = origin_world.mul_transform(*transforms.get(a).ok()?); }
    let mut world: Vec<Transform> = Vec::with_capacity(rig.nodes.len());
    for (i, &e) in rig.nodes.iter().enumerate() {
        let parent = rig.parent[i].map_or(origin_world, |p| world[p]);
        world.push(parent.mul_transform(*transforms.get(e).ok()?));
    }
    Some(world)
}

fn push_out(p: Vec3, a: Vec3, b: Vec3, r: f32) -> Vec3 {
    let ab = b - a;
    let t = if ab.length_squared() < 1e-8 { 0. } else { ((p - a).dot(ab) / ab.length_squared()).clamp(0., 1.) };
    let c = a + ab * t; let d = p - c; let l = d.length();
    if l >= r || l < 1e-6 { p } else { c + d / l * r }
}

/// Step the springs and write the bones' local rotations (after `pose`, before propagation).
pub fn step(time: Res<Time>, mut rigs: Query<(Entity, &mut Springs, &Visibility)>,
            mut transforms: Query<&mut Transform>, parents: Query<&ChildOf>) {
    let dt = time.delta_secs().min(0.1);
    for (root, mut rig, vis) in &mut rigs {
        if *vis == Visibility::Hidden { rig.last_root = None; continue; }
        let joints = rig.joints.clone();
        let bones = rig.bones.clone();
        let named_parent = rig.named_parent;
        // the extras are not posed: back to their bind transform before acting on them
        let extras: Vec<(Entity, Transform)> = rig.extras.iter().map(|&(i, base)| (rig.nodes[i], base)).collect();
        for (e, base) in &extras {
            if let Ok(mut t) = transforms.get_mut(*e) { t.rotation = base.rotation; }
        }
        let Some(world) = world_pose(root, &joints, &*rig, &transforms, &parents) else { continue; };
        let (nodes, parent) = (rig.nodes.clone(), rig.parent.clone());
        let root_pos = transforms.get(root).map(|t| t.translation).unwrap_or_default();
        // a teleport (round start, intro cut) resets everything to the animated pose
        let reset = rig.last_root.is_none_or(|p| p.distance(root_pos) > 1.5) || dt <= 0.;
        rig.last_root = Some(root_pos);
        let goal = |j: usize| world[j].translation;

        // --- jiggle (60 Hz steps, secondary.rs) ---
        let steps60 = if reset { 0 } else { ((dt * 60.).round() as u32).max(1) };
        for jg in rig.jiggles.iter_mut() {
            // world position and rotation of this bone: animated joints are in `joints`; the butt bones hang off bone 15
            let local = *transforms.get(jg.bone).unwrap();
            let base = *jg.base.get_or_insert(local.rotation);
            let animated = joints.contains(&jg.bone);
            let parent_world = if animated {
                let i = joints.iter().position(|&e| e == jg.bone).unwrap();
                if bones[i].parent < 0 { continue; } else { world[bones[i].parent as usize] }
            } else {
                match bones.iter().position(|b| b.id == named_parent) { Some(h) => world[h], None => continue }
            };
            let rest = if animated { local.rotation } else { base };
            let w = parent_world.mul_transform(Transform { rotation: rest, ..local });
            if reset || jg.prev.is_none() { jg.prev = Some(w.translation); jg.angles = Vec3::ZERO; jg.velocity = Vec3::ZERO; }
            let delta = w.translation - jg.prev.unwrap();
            jg.prev = Some(w.translation);
            if steps60 > 0 {
                let driver = (w.rotation.inverse() * delta * 100. / steps60 as f32).clamp(Vec3::splat(-3.), Vec3::splat(3.));
                let p = jg.response;
                for _ in 0..steps60 {
                    for axis in 0..3 { jg.velocity += p.gains[axis] * driver[axis]; }
                    jg.velocity = (jg.velocity - jg.angles * p.spring) * p.damping;
                    let wanted = jg.angles + jg.velocity;
                    jg.angles = wanted.clamp(-p.limit, p.limit);
                    for axis in 0..3 { if wanted[axis] != jg.angles[axis] { jg.velocity[axis] = 0.; } }
                }
            }
            let a = jg.angles * std::f32::consts::PI / 180.;
            if let Ok(mut t) = transforms.get_mut(jg.bone) { t.rotation = rest * Quat::from_euler(EulerRot::XYZ, a.x, a.y, a.z); }
        }

        // inspection: MW_SPRINGS_LOG=1 prints the largest jiggle angle and rope offset about once a second
        if std::env::var_os("MW_SPRINGS_LOG").is_some() && (time.elapsed_secs() % 1.) < dt {
            let jiggle = rig.jiggles.iter().map(|j| j.angles.abs().max_element()).fold(0., f32::max);
            let rope = rig.ropes.iter().flat_map(|r| r.points.iter().zip(&r.joints).map(|(p, &j)| p.distance(goal(j)))).fold(0., f32::max);
            if *vis != Visibility::Hidden { info!("[springs] {} jiggle {jiggle:.1} deg, rope offset {rope:.3} m", rig.actor); }
        }
        // --- ropes (Verlet at 120 Hz, whip.rs) ---
        let carry = if reset { 0. } else { rig.carry + dt };
        let substeps = (carry / SUBSTEP).floor() as u32;
        rig.carry = carry - substeps as f32 * SUBSTEP;
        let capsules: Vec<(Vec3, Vec3, f32)> = rig.capsules.iter().map(|&(a, b, r)| (goal(a), goal(b), r)).collect();
        for rope in rig.ropes.iter_mut() {
            let goals: Vec<Vec3> = rope.joints.iter().map(|&j| goal(j)).collect();
            if reset || rope.points.len() != goals.len() { rope.points = goals.clone(); rope.previous = goals.clone(); }
            let lengths: Vec<f32> = goals.windows(2).map(|w| w[0].distance(w[1])).collect();
            for _ in 0..substeps {
                let h2 = SUBSTEP * SUBSTEP;
                rope.points[0] = goals[0]; rope.previous[0] = goals[0];
                for i in 1..goals.len() {
                    let old = rope.points[i];
                    let velocity = (old - rope.previous[i]) * RETAIN;
                    rope.previous[i] = old;
                    let k = rope.stiff[i] * h2;
                    let p = (old + velocity + Vec3::NEG_Y * GRAVITY * h2 + goals[i] * k) / (1. + k);
                    rope.points[i] = goals[i] + (p - goals[i]).clamp_length_max(rope.max_offset);
                }
                for _ in 0..6 {
                    for i in 0..lengths.len() {
                        let (a, b) = (rope.points[i], rope.points[i + 1]);
                        let d = b - a; let l = d.length().max(1e-6);
                        let fix = d * ((l - lengths[i]) / l);
                        if i == 0 { rope.points[1] -= fix; } else { rope.points[i] += fix * 0.5; rope.points[i + 1] -= fix * 0.5; }
                    }
                    for i in 1..goals.len() {
                        for &(a, b, r) in &capsules { rope.points[i] = push_out(rope.points[i], a, b, r); }
                    }
                }
            }
            // turn each bone from its animated direction to the simulated one, root first
            let Some(first_parent) = parent[rope.joints[0]] else { continue; };
            let mut parent_world = world[first_parent];
            // the tip keeps its animated rotation relative to its (turned) parent
            for (n, &j) in rope.joints[..rope.joints.len() - 1].iter().enumerate() {
                let animated = world[j];
                let from = goals[n + 1] - goals[n]; let to = rope.points[n + 1] - rope.points[n];
                let rotation = if from.length_squared() > 1e-8 && to.length_squared() > 1e-8 { Quat::from_rotation_arc(from.normalize(), to.normalize()) * animated.rotation } else { animated.rotation };
                let local = parent_world.rotation.inverse() * rotation;
                if let Ok(mut t) = transforms.get_mut(nodes[j]) { t.rotation = local; }
                let local_t = transforms.get(nodes[j]).map(|t| *t).unwrap_or_default();
                parent_world = parent_world.mul_transform(local_t);
            }
        }
    }
}
