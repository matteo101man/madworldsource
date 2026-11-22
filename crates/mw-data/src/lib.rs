//! Staged native data, independent of the renderer. Source provenance lives beside the assets.
use serde::Deserialize;
pub mod collision;
pub mod character;
pub mod event_message;

#[derive(Clone, Debug, Deserialize)]
pub struct Spawn { pub id: u32, pub position: [f32; 3] }

#[derive(Clone, Debug, Deserialize)]
pub struct Arena { pub spawns: Vec<Spawn>, pub world: String }

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RigEvent {
    #[serde(rename = "type")]
    pub kind: u8,
    pub subtype: u8,
    pub flags: u16,
    pub frame: f32,
    /// Original big-endian words at record offsets 4..48.
    pub words: [u32; 11],
}

impl RigEvent {
    pub fn signed_parameter_0c(&self) -> i16 { (self.words[2] >> 16) as i16 }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bone {
    pub id: i16,
    pub parent: i16,
    pub relative: [f32; 3],
}

#[derive(Clone, Debug, Deserialize)]
pub struct Track {
    pub bone: i16,
    pub channel: u8,
    pub encoding: u8,
    pub values: Vec<f32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Clip {
    pub id: String,
    pub name: String,
    pub frames: usize,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MotionBank {
    pub bones: Vec<Bone>,
    pub clips: Vec<Clip>,
}

impl MotionBank {
    pub fn parse(text: &str) -> Result<Self, String> {
        let bank: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let mut ids = std::collections::HashSet::new();
        for (i, bone) in bank.bones.iter().enumerate() {
            if bone.id < 0 || !ids.insert(bone.id) { return Err("duplicate/reserved native bone ID".into()); }
            if bone.parent < -1 || bone.parent as usize == i || bone.parent >= bank.bones.len() as i16 {
                return Err("invalid native parent".into());
            }
            if !bone.relative.iter().all(|v| v.is_finite()) { return Err("nonfinite bind translation".into()); }
        }
        for bone in &bank.bones {
            let mut cursor = bone.parent;
            for _ in 0..bank.bones.len() {
                if cursor < 0 { break; }
                cursor = bank.bones[cursor as usize].parent;
            }
            if cursor >= 0 { return Err("cyclic native hierarchy".into()); }
        }
        for clip in &bank.clips {
            if clip.frames == 0 { return Err("empty native motion".into()); }
            for track in &clip.tracks {
                if track.values.len() != clip.frames || !track.values.iter().all(|v| v.is_finite()) {
                    return Err(format!("{}: invalid sample count/value", clip.id));
                }
            }
        }
        Ok(bank)
    }
}
pub mod ui;
pub mod ui_writing;

pub mod ui_message;
pub mod save;
pub mod save_store;
pub mod event;
