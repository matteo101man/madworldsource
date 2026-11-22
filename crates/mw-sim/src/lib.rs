//! Native sampled rig evaluation. Gameplay dispatch is not substituted with generic AI.
use mw_data::{Bone, Clip};
pub mod collision;
pub mod event;
pub mod event_room;
pub mod event_message;
pub mod ui_writing;
pub mod ui_challenge;
pub mod ui_continue;
pub mod ui_hud;
pub mod ui_save;
pub mod character;
pub mod player;
pub mod fight;
pub mod kreely;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub translation: [f32; 3],
    /// Native local Euler components; compose Z*Y*X in the renderer.
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl Pose {
    pub fn bind(translation: [f32; 3]) -> Self {
        Self { translation, rotation: [0.; 3], scale: [1.; 3] }
    }
}

/// Returns bones in native MDB order plus the synthetic MOT root (native ID -1).
/// Fractional sampling matches the explorer. It is an inspection interpolation,
/// not a claim about the original game's update cadence.
pub fn sample(bones: &[Bone], clip: &Clip, frame: f32) -> Vec<Pose> {
    let mut poses: Vec<_> = bones.iter().map(|b| Pose::bind(b.relative)).collect();
    poses.push(Pose::bind([0.; 3]));
    let frame = if frame.is_finite() { frame.clamp(0., (clip.frames - 1) as f32) } else { 0. };
    let a = frame.floor() as usize;
    let b = (a + 1).min(clip.frames - 1);
    let fraction = frame - a as f32;
    for track in &clip.tracks {
        let index = if track.bone == -1 { Some(bones.len()) } else { bones.iter().position(|b| b.id == track.bone) };
        let Some(index) = index else { continue; };
        let value = track.values[a] * (1. - fraction) + track.values[b] * fraction;
        let pose = &mut poses[index];
        match track.channel {
            0..=2 => pose.translation[track.channel as usize] = value,
            3..=5 => pose.rotation[(track.channel - 3) as usize] = value,
            7..=9 => pose.scale[(track.channel - 7) as usize] = value,
            _ => {} // channel 6 and unknown auxiliary channels are not interpreted.
        }
    }
    poses
}

/// Translation of original function 0x8025adb0. Argument labels follow f1..f4
/// and r4 until caller semantics are fully established. The reverse wrap test
/// deliberately retains the original subtraction and absence of a loop gate.
/// Authority: motion-pcode normalized CFG plus source constant 0x80565200=0.
pub fn native_frame_test(f1: f32, f2: f32, f3: f32, f4: f32, r4: bool) -> bool {
    if f2 > 0. {
        if f1 <= f4 && f4 < f1 + f2 { return true; }
        if f3 <= f1 + f2 && r4 && 0. <= f4 && f4 < (f1 + f2) - f3 { return true; }
    } else {
        if f4 <= f1 && f1 + f2 < f4 { return true; }
        if f4 < f3 && (f3 + f1) - f2 <= f4 { return true; }
    }
    false
}

/// Partial, exact field effects of cEmBoss::onRigEvent (analysis 0x81005aac).
/// Offsets retain neutral names until the consumers establish their semantics.
/// Unknown event types return false; they are never treated as implemented.
#[derive(Default, Debug)]
pub struct BossRigState {
    pub flags_b34: [u8; 3],
    pub counter_b38: u32,
    pub selector_b3c: i16,
    pub field_f0: u32,
    pub field_b44: u32,
    pub event_b4c: Option<mw_data::RigEvent>,
}

impl BossRigState {
    pub fn apply(&mut self, event: &mw_data::RigEvent) -> bool {
        match event.kind {
            18 => { self.flags_b34 = [0; 3]; self.field_f0 = 0; self.field_b44 = 0; }
            11 => self.flags_b34[2] = 1,
            7 | 19 => {
                self.flags_b34[usize::from(event.kind == 19)] = 1;
                let parameter = event.signed_parameter_0c();
                if parameter != 0 { self.selector_b3c = parameter; }
                self.event_b4c = Some(event.clone());
                self.counter_b38 = self.counter_b38.wrapping_add(1);
            }
            20 => self.flags_b34[1] = 0,
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mw_data::Track;
    #[test]
    fn native_boss_event_preserves_selector_and_counter_on_reset() {
        let mut state = BossRigState { selector_b3c: 5, counter_b38: u32::MAX, ..Default::default() };
        let mut event = mw_data::RigEvent { kind: 19, subtype: 0, flags: 0, frame: 20., words: [0; 11] };
        event.words[2] = 9 << 16;
        assert!(state.apply(&event));
        assert_eq!(state.flags_b34, [0, 1, 0]);
        assert_eq!((state.selector_b3c, state.counter_b38), (9, 0));
        event.kind = 20; event.words[2] = 0;
        assert!(state.apply(&event));
        assert_eq!(state.flags_b34, [0; 3]);
        assert_eq!(state.selector_b3c, 9);
        event.kind = 7; state.apply(&event);
        event.kind = 18; state.apply(&event);
        assert_eq!(state.counter_b38, 1);
        assert_eq!(state.selector_b3c, 9);
        assert_eq!(state.event_b4c.as_ref().unwrap().kind, 7);
        event.kind = 42; assert!(!state.apply(&event));
    }
    #[test]
    fn native_event_window_boundaries_and_loop_gate() {
        assert!(native_frame_test(0., 1., 50., 0., false));
        assert!(!native_frame_test(0., 1., 50., 1., false));
        assert!(native_frame_test(49., 2., 50., 0., true));
        assert!(!native_frame_test(49., 2., 50., 0., false));
        assert!(native_frame_test(10., -2., 50., 10., false));
        assert!(native_frame_test(10., -2., 50., 9., false));
        assert!(!native_frame_test(10., -2., 50., 8., false));
        assert!(native_frame_test(-2., -1., 50., 49., false));
    }
    #[test]
    fn native_ids_root_and_channels_are_independent_of_array_order() {
        let bones = vec![Bone { id: 129, parent: -1, relative: [1., 2., 3.] }];
        let clip = Clip { id: "fixture".into(), name: "fixture".into(), frames: 2, tracks: vec![
            Track { bone: 129, channel: 0, encoding: 1, values: vec![10., 20.] },
            Track { bone: -1, channel: 2, encoding: 1, values: vec![2., 8.] },
            Track { bone: 129, channel: 5, encoding: 0, values: vec![0.5; 2] },
            Track { bone: 128, channel: 1, encoding: 0, values: vec![99.; 2] },
        ] };
        let poses = sample(&bones, &clip, 0.5);
        assert_eq!(poses[0].translation, [15., 2., 3.]);
        assert_eq!(poses[0].rotation, [0., 0., 0.5]);
        assert_eq!(poses[1].translation, [0., 0., 5.]);
        assert_eq!(sample(&bones, &clip, 100.)[0].translation[0], 20.);
        assert_eq!(sample(&bones, &clip, f32::NAN)[0].translation[0], 10.);
    }
    #[test]
    fn every_staged_motion_has_finite_poses_at_boundaries() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
        for (name, count) in [("rinrin", 71), ("jack", 363), ("jack-player", 249)] {   // (+1 each: the scissors win, tools/stage_rin_scissors.py)
          let path = root.join(format!("{name}-motion.json"));
          let bank = mw_data::MotionBank::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
          assert_eq!(bank.clips.len(), count);
          for clip in &bank.clips {
            for frame in [0., 0.5, (clip.frames - 1) as f32] {
                for pose in sample(&bank.bones, clip, frame) {
                    assert!(pose.translation.iter().chain(&pose.rotation).chain(&pose.scale).all(|v| v.is_finite()), "{}", clip.id);
                }
            }
          }
        }
    }
}
pub mod ui;

pub mod ui_message;
