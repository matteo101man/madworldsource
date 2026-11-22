//! Kreely's pistol reload (added, authored here: no reload motion exists on the disc - Jude's revolvers never reload and
//! Leonardo's / the thugs' event clips only hold their guns). A procedural layer over the lowered pistol idle, keyed to
//! the 40-frame reload of play.rs (magazine seats at frame 28):
//!   0-6   the gun comes up in front of the chest and rolls inward, the left hand meets the grip (magazine out)
//!   6-15  the left hand drops to the belt for a fresh magazine
//!   15-26 it comes back and pushes the magazine up into the grip
//!   26-30 a firm palm slap seats it
//!   30-35 the left hand grabs the slide and racks it back
//!   35-40 both arms ease back into the idle
//! Each arm is a two-bone IK (upper arm, forearm) toward a hand target in the chest's frame, the elbow bending toward a
//! down-and-out pole; the hand keeps its rotation relative to the forearm (plus the gun roll).
use bevy::prelude::*;
use mw_data::Bone;
use mw_sim::character::LocalPose;

/// pl0030 bone ids: chest, right arm (upper, fore, hand), left arm.
const CHEST: i16 = 2;
const RIGHT: [i16; 3] = [7, 8, 9];
const LEFT: [i16; 3] = [12, 13, 14];

fn smooth(x: f32) -> f32 { let x = x.clamp(0., 1.); x * x * (3. - 2. * x) }
fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 { a + (b - a) * smooth(t) }

/// Hand targets in the chest's frame (x his left, y up, z forward) at reload frame `t`, and the gun's roll.
fn keys(t: f32) -> (Vec3, Vec3, f32) {
    let gun = Vec3::new(-0.06, -0.12, 0.34);                 // right hand: the gun up in front of the chest
    let grip = gun + Vec3::new(0.03, -0.11, -0.02);          // under the grip (magazine well)
    let belt = Vec3::new(0.17, -0.5, 0.08);                  // left hip
    let slide = gun + Vec3::new(0.02, 0.05, -0.02);
    let racked = slide + Vec3::new(0., 0.0, -0.1);
    let left = if t < 6. { lerp(grip + Vec3::new(0.12, -0.08, -0.1), grip, t / 6.) }
        else if t < 15. { lerp(grip, belt, (t - 6.) / 9.) }
        else if t < 26. { lerp(belt, grip - Vec3::Y * 0.03, (t - 15.) / 11.) }
        else if t < 30. { lerp(grip - Vec3::Y * 0.03, grip + Vec3::Y * 0.02, (t - 26.) / 2.) }
        else if t < 35. { if t < 32. { lerp(grip, slide, (t - 30.) / 2.) } else { lerp(slide, racked, (t - 32.) / 2.) } }
        else { racked };
    let roll = if t < 30. { smooth(t / 5.) * 0.55 } else { 0.55 * (1. - smooth((t - 30.) / 4.)) };
    (gun, left, roll)
}

/// Apply the reload at frame `t` (0..40) to Kreely's local pose (actor space, `bones` order + root).
pub fn apply(bones: &[Bone], poses: &mut [LocalPose], t: f32) {
    let weight = smooth(t / 5.) * (1. - smooth((t - 35.) / 5.));
    if weight <= 0. { return; }
    let index = |id: i16| bones.iter().position(|b| b.id == id);
    let Some(chest) = index(CHEST) else { return; };
    let frames = mw_sim::fight::bone_frames(bones, poses);
    let (cr, cp) = (Quat::from_array(frames[chest].0), Vec3::from_array(frames[chest].1));
    let (gun, left, roll) = keys(t);
    for (arm, target, pole, roll) in [(RIGHT, gun, Vec3::new(-0.7, -1., -0.4), roll), (LEFT, left, Vec3::new(0.7, -1., -0.4), 0.)] {
        let (Some(u), Some(f), Some(h)) = (index(arm[0]), index(arm[1]), index(arm[2])) else { continue; };
        let frames = mw_sim::fight::bone_frames(bones, poses);
        let p = |i: usize| Vec3::from_array(frames[i].1);
        let q = |i: usize| Quat::from_array(frames[i].0);
        let (s, e, w) = (p(u), p(f), p(h));
        let goal = cp + cr * target;
        let goal = w + (goal - w) * weight;                 // blend the target in and out
        let (a, b) = ((e - s).length(), (w - e).length());
        let d = (goal - s).length().clamp((a - b).abs() + 1e-3, a + b - 1e-3);
        let dir = (goal - s).normalize_or_zero();
        let pole = cr * pole.normalize();
        let side = (pole - dir * pole.dot(dir)).normalize_or(Vec3::NEG_Y);
        let alpha = ((a * a + d * d - b * b) / (2. * a * d)).clamp(-1., 1.).acos();
        let elbow = s + (dir * alpha.cos() + side * alpha.sin()) * a;
        let hand = s + dir * d;
        let du = Quat::from_rotation_arc((e - s).normalize_or_zero(), (elbow - s).normalize_or_zero());
        let upper = du * q(u);
        let df = Quat::from_rotation_arc((du * (w - e)).normalize_or_zero(), (hand - elbow).normalize_or_zero());
        let fore = df * du * q(f);
        let palm = df * du * q(h) * Quat::from_rotation_z(-roll * weight);
        let parent = |i: usize| if bones[i].parent >= 0 { Quat::from_array(frames[bones[i].parent as usize].0) } else { Quat::from_array(poses[bones.len()].rotation) };
        poses[u].rotation = (parent(u).inverse() * upper).normalize().to_array();
        poses[f].rotation = (upper.inverse() * fore).normalize().to_array();
        poses[h].rotation = (fore.inverse() * palm).normalize().to_array();
        if std::env::var_os("MW_RELOAD_LOG").is_some() {
            let after = mw_sim::fight::bone_frames(bones, poses);
            eprintln!("[reload] t {t:.1} arm {} hand {:?} -> {:?} goal {:?} chest {:?}", arm[0], w, Vec3::from_array(after[h].1), goal, cp);
        }
    }
}
