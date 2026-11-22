//! Kreely's added melee moveset: God Hand motions retargeted onto the pl0030 rig (tools/godhand/retarget_godhand.py,
//! assets/native/kreely-moves.json): his clips join Jack's bank with native-style SEQ cues (hit windows, combo window,
//! sounds), the receivers' clips (the suplexed opponent, the sweep knockdown) join RinRin's bank, and the grunt copies
//! go to the street (mw-game). The move choice and the suplex are in fight.rs (`Fight::kreely`).
use crate::fight::{Cue, FightData};
use mw_data::Clip;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
pub struct MoveSpec { pub clip: String, pub atk: u32, pub power: i32, pub part: usize, pub rate: f32, pub frames: usize }

#[derive(Clone, Debug, Deserialize)]
pub struct SuplexSpec {
    /// The receiver's placement in the grabber's frame (x his left, z forward) and its yaw, at the grab.
    pub offset: [f32; 3], pub yaw: f32,
    /// The slam: grabber motion frames.
    pub slam: [f32; 2],
}

#[derive(Debug, Deserialize)]
pub struct KreelyMoves {
    pub clips: Vec<Clip>, pub cues: HashMap<String, Vec<Cue>>, pub moves: HashMap<String, MoveSpec>,
    pub rin_clips: Vec<Clip>, pub grunt_clips: Vec<Clip>, pub suplex: SuplexSpec,
}

impl KreelyMoves {
    pub fn parse(text: &str) -> Result<Self, String> { serde_json::from_str(text).map_err(|e| e.to_string()) }
}

/// What the fight keeps of the move set.
#[derive(Clone, Debug)]
pub struct KreelyKit { pub moves: HashMap<String, MoveSpec>, pub suplex: SuplexSpec }

impl FightData {
    /// Merge the move set: clips into the banks, cues into the combat data. Returns the grunt clips (the street's).
    pub fn add_kreely(&mut self, m: KreelyMoves) -> Vec<Clip> {
        for c in m.clips { if !self.jack.clips.iter().any(|j| j.name == c.name) { self.jack.clips.push(c); } }
        for c in m.rin_clips { if !self.rin.clips.iter().any(|j| j.name == c.name) { self.rin.clips.push(c); } }
        self.combat.cues.extend(m.cues);
        self.reindex();
        self.kreely = Some(KreelyKit { moves: m.moves, suplex: m.suplex });
        m.grunt_clips
    }
}
