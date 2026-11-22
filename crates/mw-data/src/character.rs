//! Added-character rig (tools/kreely/export_kreely_glb.py -> assets/native/kreely-rig.json).
//! Bone ids/parents follow pl0030; joint positions are the character's own. Not original disc data.
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
pub struct RigBone {
    pub id: i16,
    pub parent: i16,
    pub relative: [f32; 3],
    pub position: [f32; 3],
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Retarget {
    #[serde(default)]
    pub root_scale: f32,
    #[serde(default)]
    pub rotation_only: bool,
    #[serde(default)]
    pub root_scale_by_family: HashMap<String, f32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OverlayTrack { pub bone: i16, pub channel: u8, pub values: Vec<f32> }

#[derive(Clone, Debug, Deserialize)]
pub struct Shot { pub frame: f32, pub gun: String }

/// Floor-contact root lift: a constant or one value per frame.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Lift { Constant(f32), Frames(Vec<f32>) }
impl Default for Lift { fn default() -> Self { Lift::Constant(0.) } }
impl Lift {
    pub fn at(&self, frame: f32) -> f32 {
        match self {
            Lift::Constant(v) => *v,
            Lift::Frames(v) if v.is_empty() => 0.,
            Lift::Frames(v) => { let a = (frame.max(0.).floor() as usize).min(v.len() - 1); let b = (a + 1).min(v.len() - 1); let f = frame - a as f32; v[a] * (1. - f) + v[b] * f }
        }
    }
}

/// A clip played on this character with baked additions (explorer overlay format): trimmed range, optional
/// left/right mirror of the source tracks, event-world origin removal, prop/arm tracks, floor lift and facing.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overlay {
    pub clip: String,
    pub family: String,
    pub label: String,
    #[serde(default)]
    pub tracks: Vec<OverlayTrack>,
    #[serde(default)]
    pub shots: Vec<Shot>,
    #[serde(default)]
    pub hand_pose: Option<String>,
    #[serde(default)]
    pub range: Option<[usize; 2]>,
    #[serde(default)]
    pub root_origin: Option<[f32; 3]>,
    #[serde(default)]
    pub root_lift: Lift,
    #[serde(default)]
    pub root_yaw: f32,
    #[serde(default)]
    pub mirror: bool,
    #[serde(default)]
    pub style: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterRig {
    /// overlay key "<source clip id>[|variant]" -> overlay
    #[serde(default)]
    pub overlays: HashMap<String, Overlay>,
    pub bones: Vec<RigBone>,
    #[serde(default)]
    pub retarget: Retarget,
    /// pose name -> bone id (decimal string) -> local quaternion [x, y, z, w]
    #[serde(default)]
    pub hand_poses: HashMap<String, HashMap<String, [f32; 4]>>,
    #[serde(default)]
    pub default_hand_pose: Option<String>,
    /// Bones that rest at scale 0 unless an overlay shows them (e.g. the handgun).
    #[serde(default)]
    pub prop_bones: Vec<i16>,
}

impl CharacterRig {
    pub fn parse(text: &str) -> Result<Self, String> {
        let rig: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        for (i, b) in rig.bones.iter().enumerate() {
            if b.parent < -1 || (b.parent >= 0 && (b.parent as usize == i || b.parent as usize >= rig.bones.len())) {
                return Err(format!("bone {} has an invalid parent", b.id));
            }
            if !b.relative.iter().chain(&b.position).all(|v| v.is_finite()) { return Err(format!("bone {} is not finite", b.id)); }
        }
        if rig.retarget.rotation_only && !(rig.retarget.root_scale > 0.) { return Err("rotation-only retarget needs a root scale".into()); }
        Ok(rig)
    }

    /// Bones in the mw_data::Bone shape used by the samplers.
    pub fn sample_bones(&self) -> Vec<crate::Bone> {
        self.bones.iter().map(|b| crate::Bone { id: b.id, parent: b.parent, relative: b.relative }).collect()
    }

    /// Overlay key whose `clip` stem matches (e.g. "ev93_cut8_aim").
    pub fn overlay_key(&self, stem: &str) -> Option<&str> {
        self.overlays.iter().find(|(_, o)| o.clip == stem).map(|(k, _)| k.as_str())
    }

    pub fn hand_pose(&self, name: Option<&str>) -> Vec<(i16, [f32; 4])> {
        let key = name.or(self.default_hand_pose.as_deref());
        key.and_then(|k| self.hand_poses.get(k))
            .map(|m| m.iter().filter_map(|(id, q)| id.parse::<i16>().ok().map(|id| (id, *q))).collect())
            .unwrap_or_default()
    }
}
