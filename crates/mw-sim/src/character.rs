//! Playable-character pose evaluation (native goal phase 1): retargeted sampling, hand poses, pose crossfades and
//! root motion. Shared by Jack (native rig) and added characters (Kreely: pl0030 ids, own proportions).
use crate::{sample, Pose};
use mw_data::{Bone, Clip};

/// Local bone transform with a quaternion rotation [x, y, z, w].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalPose {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// three.js / native ZYX Euler (R = Rz * Ry * Rx) to a quaternion.
pub fn quat_zyx(e: [f32; 3]) -> [f32; 4] {
    let (sx, cx) = (e[0] * 0.5).sin_cos();
    let (sy, cy) = (e[1] * 0.5).sin_cos();
    let (sz, cz) = (e[2] * 0.5).sin_cos();
    [sx * cy * cz - cx * sy * sz, cx * sy * cz + sx * cy * sz, cx * cy * sz - sx * sy * cz, cx * cy * cz + sx * sy * sz]
}

impl From<Pose> for LocalPose {
    fn from(p: Pose) -> Self { Self { translation: p.translation, rotation: quat_zyx(p.rotation), scale: p.scale } }
}

#[derive(Clone, Copy, Debug)]
pub struct Retarget {
    /// Keep the character's own bind offsets for every bone except the root (-1, 0).
    pub rotation_only: bool,
    /// Root (-1, 0) translations are scaled by this (hip-height ratio).
    pub root_scale: f32,
}

impl Retarget {
    pub const NATIVE: Self = Self { rotation_only: false, root_scale: 1. };
}

/// Samples `clip` on `bones` (native order, plus the synthetic MOT root last) with a retarget: identical bone
/// rotations, the character's own offsets, scaled root translation. With `Retarget::NATIVE` equals `sample`.
pub fn sample_retargeted(bones: &[Bone], clip: &Clip, frame: f32, retarget: Retarget) -> Vec<LocalPose> {
    let mut poses = sample(bones, clip, frame);
    if retarget.rotation_only {
        for (pose, bone) in poses.iter_mut().zip(bones) {
            if bone.id != 0 { pose.translation = bone.relative; }
        }
    }
    if retarget.root_scale != 1. { scale_root_tracks(bones, clip, &mut poses, retarget.root_scale); }
    poses.into_iter().map(LocalPose::from).collect()
}

/// Root (-1, 0) translations driven by a track are scaled; untracked components keep the bind value (explorer rule).
fn scale_root_tracks(bones: &[Bone], clip: &Clip, poses: &mut [Pose], scale: f32) {
    let n = bones.len();
    for t in clip.tracks.iter().filter(|t| t.channel < 3 && (t.bone == -1 || t.bone == 0)) {
        let i = if t.bone == -1 { Some(n) } else { bones.iter().position(|b| b.id == 0) };
        if let Some(i) = i { poses[i].translation[t.channel as usize] *= scale; }
    }
}

/// Overrides local rotations with a hand pose (bone id, quaternion) and hides prop bones (scale 0).
pub fn apply_hand_pose(bones: &[Bone], poses: &mut [LocalPose], hand: &[(i16, [f32; 4])], hidden_props: &[i16]) {
    for (id, q) in hand {
        if let Some(i) = bones.iter().position(|b| b.id == *id) { poses[i].rotation = *q; }
    }
    for id in hidden_props {
        if let Some(i) = bones.iter().position(|b| b.id == *id) { poses[i].scale = [0.; 3]; }
    }
}

fn slerp(a: [f32; 4], mut b: [f32; 4], w: f32) -> [f32; 4] {
    let mut dot: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
    if dot < 0. { b = b.map(|v| -v); dot = -dot; }
    let (ka, kb) = if dot > 0.9995 { (1. - w, w) } else {
        let th = dot.clamp(-1., 1.).acos(); let s = th.sin();
        (((1. - w) * th).sin() / s, (w * th).sin() / s)
    };
    let q = [0, 1, 2, 3].map(|i| a[i] * ka + b[i] * kb);
    let n = q.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-9);
    q.map(|v| v / n)
}

/// Eased full-pose crossfade (same smoothstep as the explorer): `k` of `n` frames into the blend from `from` to `to`.
pub fn crossfade(from: &[LocalPose], to: &[LocalPose], k: f32, n: f32) -> Vec<LocalPose> {
    let w = ((k + 1.) / (n + 1.)).clamp(0., 1.); let w = w * w * (3. - 2. * w);
    from.iter().zip(to).map(|(a, b)| LocalPose {
        translation: [0, 1, 2].map(|i| a.translation[i] * (1. - w) + b.translation[i] * w),
        rotation: slerp(a.rotation, b.rotation, w),
        scale: [0, 1, 2].map(|i| a.scale[i] * (1. - w) + b.scale[i] * w),
    }).collect()
}

/// Linear pose mix (weight `w` of `b`), for motion layers such as the sprint layer.
pub fn mix(a: &[LocalPose], b: &[LocalPose], w: f32) -> Vec<LocalPose> {
    let w = w.clamp(0., 1.);
    a.iter().zip(b).map(|(a, b)| LocalPose {
        translation: [0, 1, 2].map(|i| a.translation[i] * (1. - w) + b.translation[i] * w),
        rotation: slerp(a.rotation, b.rotation, w),
        scale: [0, 1, 2].map(|i| a.scale[i] * (1. - w) + b.scale[i] * w),
    }).collect()
}

/// 80079ED0's normal motion path: filter the existing part transforms toward
/// this sample. Untracked part translations and rotations keep their values;
/// untracked scale tends to one. The synthetic root always has defaults.
/// This helper does not model part suppression/force flags or additive paths.
pub fn filter_native_pose(bones: &[Bone], clip: &Clip, frame: f32, remaining: f32, previous: &mut Vec<LocalPose>) {
    if previous.len() != bones.len() + 1 {
        *previous = bones.iter().map(|b| LocalPose::from(Pose::bind(b.relative)))
            .chain(std::iter::once(LocalPose::from(Pose::bind([0.; 3])))).collect();
    }
    let target = sample_retargeted(bones, clip, frame, Retarget::NATIVE);
    let weight = 1. / (1. + remaining.max(0.));
    for i in 0..previous.len() {
        let id = bones.get(i).map(|b| b.id).unwrap_or(-1);
        let translated = id == -1 || clip.tracks.iter().any(|t| t.bone == id && t.channel == 0);
        let rotated = id == -1 || clip.tracks.iter().any(|t| t.bone == id && t.channel == 3);
        let old = &mut previous[i];
        if translated {
            old.translation = [0, 1, 2].map(|j| old.translation[j] + (target[i].translation[j] - old.translation[j]) * weight);
        }
        if rotated { old.rotation = slerp(old.rotation, target[i].rotation, weight); }
        old.scale = [0, 1, 2].map(|j| old.scale[j] + (target[i].scale[j] - old.scale[j]) * weight);
    }
}

/// Root motion of the synthetic MOT root (-1) plus bone 0's horizontal offset, in clip space (x, z), between two
/// frames inside one playthrough (`to` >= `from`; callers split at the loop seam).
pub fn root_xz(bones: &[Bone], clip: &Clip, frame: f32) -> [f32; 2] {
    let poses = sample(bones, clip, frame);
    let origin = poses[bones.len()].translation;
    [origin[0], origin[2]]
}

/// In-place playback: zero the horizontal root travel that `root_xz` hands to the character controller.
pub fn strip_root_xz(bones: &[Bone], poses: &mut [LocalPose]) {
    let n = bones.len();
    poses[n].translation[0] = 0.; poses[n].translation[2] = 0.;
}

/// Left/right bone pairs for mirrored overlay clips (same table as app.js / kreely_pose.py).
pub const MIRROR_PAIRS: [(i16, i16); 12] = [(5, 10), (6, 11), (7, 12), (8, 13), (9, 14), (16, 20), (17, 21), (18, 22), (19, 23), (40, 41), (50, 51), (100, 101)];

fn mirror_bone(id: i16) -> i16 {
    MIRROR_PAIRS.iter().find_map(|&(a, b)| if a == id { Some(b) } else if b == id { Some(a) } else { None }).unwrap_or(id)
}

/// The source clip as an overlay plays it: trimmed to `range`, mirrored left/right when `mirror` is set
/// (paired bones swap; x positions and y/z ZYX angles change sign).
pub fn prepare_overlay_clip(src: &Clip, ov: &mw_data::character::Overlay) -> Clip {
    let (a, b) = ov.range.map(|[a, b]| (a, b.min(src.frames - 1))).unwrap_or((0, src.frames - 1));
    Clip { id: src.id.clone(), name: ov.clip.clone(), frames: b - a + 1, tracks: src.tracks.iter().map(|t| {
        let (bone, sign) = if ov.mirror { (mirror_bone(t.bone), if matches!(t.channel, 0 | 4 | 5) { -1. } else { 1. }) } else { (t.bone, 1.) };
        mw_data::Track { bone, channel: t.channel, encoding: t.encoding, values: t.values[a..=b].iter().map(|v| v * sign).collect() }
    }).collect() }
}

/// Samples a prepared overlay clip on a character (explorer sampleActor order): origin removal, rotation-only
/// retarget with the family's root scale, overlay prop/arm tracks, floor lift and facing yaw.
pub fn sample_overlay(bones: &[Bone], clip: &Clip, frame: f32, rotation_only: bool, root_scale: f32, ov: &mw_data::character::Overlay) -> Vec<LocalPose> {
    let mut poses = sample(bones, clip, frame);
    let n = bones.len();
    if let Some(o) = ov.root_origin {
        let has = |ch: u8| clip.tracks.iter().any(|t| t.bone == -1 && t.channel == ch);
        for ch in 0..3 { if has(ch as u8) { poses[n].translation[ch] -= o[ch] * if ov.mirror && ch == 0 { -1. } else { 1. }; } }
    }
    if rotation_only { for (pose, bone) in poses.iter_mut().zip(bones) { if bone.id != 0 { pose.translation = bone.relative; } } }
    scale_root_tracks(bones, clip, &mut poses, root_scale);
    let fr = frame.clamp(0., (clip.frames - 1) as f32);
    let (a, b) = (fr.floor() as usize, (fr.floor() as usize + 1).min(clip.frames - 1)); let f = fr - a as f32;
    for t in &ov.tracks {
        let Some(i) = bones.iter().position(|x| x.id == t.bone) else { continue; };
        let (Some(va), Some(vb)) = (t.values.get(a), t.values.get(b)) else { continue; };
        let v = va * (1. - f) + vb * f;
        match t.channel { 0..=2 => poses[i].translation[t.channel as usize] = v, 3..=5 => poses[i].rotation[(t.channel - 3) as usize] = v,
                          7..=9 => poses[i].scale[(t.channel - 7) as usize] = v, _ => {} }
    }
    poses[n].translation[1] += ov.root_lift.at(fr);
    poses[n].rotation[1] += ov.root_yaw;
    poses.into_iter().map(LocalPose::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mw_data::Track;
    fn bones() -> Vec<Bone> {
        vec![Bone { id: 0, parent: -1, relative: [0., 1., 0.] }, Bone { id: 7, parent: 0, relative: [0.3, 0.4, 0.] }]
    }
    fn clip() -> Clip {
        Clip { id: "c".into(), name: "c".into(), frames: 3, tracks: vec![
            Track { bone: 0, channel: 1, encoding: 0, values: vec![1.2, 1.2, 1.2] },
            Track { bone: 7, channel: 0, encoding: 0, values: vec![9., 9., 9.] },
            Track { bone: 7, channel: 5, encoding: 0, values: vec![0.5, 0.5, 0.5] },
            Track { bone: -1, channel: 2, encoding: 0, values: vec![0., 1., 2.] },
        ] }
    }
    #[test]
    fn native_pose_filter_uses_previous_result_and_preserves_untracked_parts() {
        let bones = bones();
        let mut clip = clip();
        // Native grouped channels are stored as complete triples.
        clip.tracks = (0..3).map(|channel| Track { bone:7, channel, encoding:0,
            values:vec![if channel == 0 { 12. } else { 0. };3] }).collect();
        let mut pose = sample_retargeted(&bones, &clip, 0., Retarget::NATIVE);
        pose[1].translation = [0.;3];
        pose[0].translation = [4.,5.,6.];
        pose[0].rotation = quat_zyx([0.,0.5,0.]);
        let held = pose[0];
        filter_native_pose(&bones, &clip, 0., 3., &mut pose);
        assert_eq!(pose[1].translation, [3.,0.,0.]);
        filter_native_pose(&bones, &clip, 0., 2., &mut pose);
        assert_eq!(pose[1].translation, [6.,0.,0.]);
        assert_eq!(pose[0], held);
        filter_native_pose(&bones, &clip, 0., 0., &mut pose);
        assert_eq!(pose[1].translation, [12.,0.,0.]);
    }
    #[test]
    fn native_pose_filter_interpolates_rotation_and_resets_absent_scale() {
        let bones = bones();
        let mut clip = clip();
        clip.tracks = (3..6).map(|channel| Track {bone:7, channel, encoding:0,
            values:vec![if channel == 4 { std::f32::consts::FRAC_PI_2 } else { 0. };3]}).collect();
        let mut pose: Vec<_> = bones.iter().map(|b|LocalPose::from(Pose::bind(b.relative)))
            .chain(std::iter::once(LocalPose::from(Pose::bind([0.;3])))).collect();
        pose[1].scale = [3.;3];
        filter_native_pose(&bones, &clip, 0., 1., &mut pose);
        let want = quat_zyx([0.,std::f32::consts::FRAC_PI_4,0.]);
        assert!(pose[1].rotation.iter().zip(want).all(|(a,b)| (a-b).abs() < 1e-6));
        assert_eq!(pose[1].scale,[2.;3]);
    }
    #[test]
    fn quaternion_matches_zyx_composition() {
        // Rz(0.5) alone: q = (0, 0, sin .25, cos .25)
        let q = quat_zyx([0., 0., 0.5]);
        assert!((q[2] - 0.25f32.sin()).abs() < 1e-6 && (q[3] - 0.25f32.cos()).abs() < 1e-6);
    }
    #[test]
    fn rotation_only_keeps_own_offsets_and_scales_the_root() {
        let p = sample_retargeted(&bones(), &clip(), 1., Retarget { rotation_only: true, root_scale: 0.5 });
        assert_eq!(p[1].translation, [0.3, 0.4, 0.]);           // own bind offset, track ignored
        assert_eq!(p[0].translation, [0., 0.6, 0.]);           // root height scaled
        assert_eq!(p[2].translation, [0., 0., 0.5]);           // synthetic root scaled
        assert_eq!(p[1].rotation, quat_zyx([0., 0., 0.5]));    // rotation identical to the source
    }
    #[test]
    fn crossfade_ends_on_target_and_root_motion_reads_the_origin() {
        let a = sample_retargeted(&bones(), &clip(), 0., Retarget::NATIVE);
        let b = sample_retargeted(&bones(), &clip(), 2., Retarget::NATIVE);
        let end = crossfade(&a, &b, 1e6, 8.);
        assert!((end[2].translation[2] - 2.).abs() < 1e-5);
        assert_eq!(root_xz(&bones(), &clip(), 2.), [0., 2.]);
    }
    #[test]
    fn overlay_sampling_matches_the_explorer_reference() {
        // Golden poses from tools/kreely/kreely_pose.py (same math as the explorer's sampleActor).
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
        let Ok(golden) = std::fs::read_to_string(root.join("kreely-golden-poses.json")) else { return; };
        let rig = mw_data::character::CharacterRig::parse(&std::fs::read_to_string(root.join("kreely-rig.json")).unwrap()).unwrap();
        #[derive(serde::Deserialize)] struct Bank { clips: Vec<Clip> }
        let bank: Bank = serde_json::from_str(&std::fs::read_to_string(root.join("kreely-gun-motion.json")).unwrap()).unwrap();
        let golden: Vec<serde_json::Value> = serde_json::from_str(&golden).unwrap();
        let bones = rig.sample_bones();
        for g in &golden {
            let stem = g["stem"].as_str().unwrap(); let frame = g["frame"].as_f64().unwrap() as f32;
            let key = rig.overlay_key(stem).unwrap(); let ov = &rig.overlays[key];
            let src = bank.clips.iter().find(|c| c.id == key.split('|').next().unwrap()).unwrap();
            let clip = prepare_overlay_clip(src, ov);
            let scale = rig.retarget.root_scale_by_family.get(&ov.family).copied().unwrap_or(rig.retarget.root_scale);
            let poses = sample_overlay(&bones, &clip, frame, rig.retarget.rotation_only, scale, ov);
            for (i, p) in poses.iter().enumerate() {
                let t: Vec<f32> = g["translation"][i].as_array().unwrap().iter().map(|v| v.as_f64().unwrap() as f32).collect();
                let q: Vec<f32> = g["rotation"][i].as_array().unwrap().iter().map(|v| v.as_f64().unwrap() as f32).collect();
                let dt = (0..3).map(|k| (p.translation[k] - t[k]).abs()).fold(0., f32::max);
                let dot = (0..4).map(|k| p.rotation[k] * q[k]).sum::<f32>().abs();
                assert!(dt < 2e-4, "{stem} f{frame} bone {i}: translation {:?} vs {:?}", p.translation, t);
                assert!(dot > 0.99995, "{stem} f{frame} bone {i}: rotation {:?} vs {:?}", p.rotation, q);
            }
        }
    }
}
