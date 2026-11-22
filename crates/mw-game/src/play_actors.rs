//! Actor definitions [added tooling, not native]: one JSON file per enemy or boss under `assets/defs/actors/<id>.json`,
//! read once at start-up, so their numbers are set in one place instead of across each module's constants.
//! A definition holds:
//! - `hp`, `power` (the base hit power; each attack's own `power` overrides it), `sense` (perception, m), `radius`;
//! - `variants`: per-outfit overrides (HP, attack rates, power scale, evades), as em_common.ini's em0101 / em0101B;
//! - `anims`: named roles -> clip names (`idle`, `walk`, `hurt_left`, ...), and `pools`: named lists of clips the AI picks
//!   from at random (laughs, threats, hide poses);
//! - `attacks`: name, clip, hit windows (clip frames), combo frame, range, yaw window, power, the native attack id
//!   (ATHIT, selects the player's reaction) and an optional voice event;
//! - `voices`: named events -> announcer voice ids (FightEvent::Voice);
//! - `tune`: free-form named numbers for whatever else a module reads (`tune("alive", 4.)`).
//! Missing files or fields fall back to the module's built-in values (what the native tables say), so a definition only
//! has to carry what it changes. Edit a file and restart the game; `MW_ACTOR_LOG=1` prints what was loaded.
//! The modules that read them: play_street.rs (em0101, the kung-fu grunts: also every placeholder zako, e.g. Elise's
//! bat-born ones), play_elise.rs (em0630).
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct AttackDef {
    pub name: String,
    pub clip: String,
    /// hit windows [from, to) in clip frames
    pub windows: Vec<[f32; 2]>,
    /// the frame a combo may chain into the next attack in the list
    pub combo: Option<f32>,
    /// m, and degrees either side of the facing
    pub range: f32, pub yaw: f32,
    /// None: the actor's `power`
    pub power: Option<i32>,
    /// the native attack id (ATHIT); 0: the module's default
    pub atk: u32,
    /// a voice event (a key of `voices`) when it starts
    pub voice: Option<String>,
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct VariantDef {
    pub name: String,
    pub hp: Option<i32>,
    /// attack chance (% a frame) close and far
    pub near: u32, pub far: u32,
    /// power scale (攻撃力)
    pub power: f32,
    /// dodges per life, and the dodge chance %
    pub evades: u32, pub evade: u32,
    /// attack names this variant picks from (empty: all)
    pub attacks: Vec<String>,
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct ActorDef {
    pub id: String,
    pub name: String,
    /// "zako" or "boss"
    pub kind: String,
    /// GLBs under assets/ (source + pack / color), and the motion bank
    pub models: Vec<String>,
    pub motions: Option<String>,
    pub hp: i32,
    pub power: i32,
    pub sense: f32,
    pub radius: f32,
    pub variants: Vec<VariantDef>,
    pub anims: HashMap<String, String>,
    pub pools: HashMap<String, Vec<String>>,
    pub attacks: Vec<AttackDef>,
    pub voices: HashMap<String, u16>,
    pub tune: HashMap<String, f32>,
    /// the in-game actor editor's page for this actor (play_editor.rs); None: not listed in the editor
    pub editor: Option<EditorDef>,
    /// what its clips, voice lines, cameras and numbered phases are, in words (the editor shows these)
    pub names: Names,
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Names {
    /// clip file -> what it is ("em0630_0111.mot": "Wing strike")
    pub clips: HashMap<String, String>,
    /// voice id (decimal) -> its name and the game event that plays it
    pub voices: HashMap<String, VoiceName>,
    /// camera clip -> the scene it films: the clips it plays with, and the editor entry that plays that scene
    pub cameras: HashMap<String, CameraName>,
    /// a state's numbered phases -> names ("Keen": {"2": "The struggle"})
    pub phases: HashMap<String, HashMap<String, String>>,
}
#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct VoiceName { pub name: String, pub trigger: String }
#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct CameraName { pub name: String, pub actor: Option<String>, pub player: Option<String>, pub entry: Option<String> }

/// The actor editor's page [added tooling]: where the actor is shown, its camera files, and the curated groups of
/// entries. The editor adds its own groups after these: every clip in `motions` (sorted into `clip_groups`), every voice
/// in `voices`, every camera clip in `cameras`.
#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct EditorDef {
    /// the MW_STAGE that hosts it (its room, banks and module)
    pub stage: String,
    /// where Reset stands the actor and the player (the actor faces the player)
    pub spot: Option<[f32; 3]>, pub player_spot: Option<[f32; 3]>,
    /// the orbit camera's look height over the actor (m)
    pub focus: f32,
    /// camera MOT files under assets/native (the pair and boss cameras, previewed from "Cameras")
    pub cameras: Vec<String>,
    /// what a camera clip with no scene is previewed on: "player" (Elise's pair cameras ride him) or "actor"
    pub camera_anchor: String,
    pub groups: Vec<EditorGroup>,
    /// "All clips" split by clip-name prefix, in order (a clip goes in the first group that matches)
    pub clip_groups: Vec<ClipGroup>,
}
#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct EditorGroup { pub name: String, pub entries: Vec<EditorEntry> }
/// One editor entry. `do`: `clip` (arg: clip name; `looped`), `move` (arg: the module's named move), `hurt` (arg: light,
/// heavy, front, back, left, right, shot, down, launch), `kill`, `pair` (arg: clash, finisher, grab, pin, ...; `win`
/// true / false picks the outcome, absent plays it as the player does), `execution` (the last-life finisher), `voice`
/// (arg: an event in `voices`, or a number), `camera` (arg: a camera clip), `input` (arg: a player button pressed
/// once: punch, swing, grab, taunt, jump, dodge), `reset`.
#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct EditorEntry {
    pub label: String,
    #[serde(rename = "do")] pub action: String,
    pub arg: String,
    pub win: Option<bool>,
    pub looped: bool,
    pub note: String,
}
#[derive(serde::Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct ClipGroup { pub name: String, pub prefixes: Vec<String> }

#[allow(dead_code)]
impl ActorDef {
    /// An animation role's clip, or `fallback`.
    pub fn anim<'a>(&'a self, role: &str, fallback: &'a str) -> &'a str { self.anims.get(role).map_or(fallback, |s| s.as_str()) }
    pub fn pool(&self, name: &str) -> &[String] { self.pools.get(name).map_or(&[], |v| v.as_slice()) }
    pub fn attack(&self, name: &str) -> Option<&AttackDef> { self.attacks.iter().find(|a| a.name == name) }
    pub fn attack_index(&self, name: &str) -> Option<usize> { self.attacks.iter().position(|a| a.name == name) }
    /// An attack's power: its own, else the actor's.
    pub fn attack_power(&self, a: &AttackDef) -> i32 { a.power.unwrap_or(self.power) }
    pub fn voice(&self, event: &str) -> Option<u16> { self.voices.get(event).copied() }
    pub fn tune(&self, key: &str, fallback: f32) -> f32 { self.tune.get(key).copied().unwrap_or(fallback) }
    /// A clip's name in words, else its file name without ".mot".
    pub fn clip_name(&self, clip: &str) -> String { self.names.clips.get(clip).cloned().unwrap_or_else(|| clip.trim_end_matches(".mot").to_string()) }
    /// A state's phase in words, else its number.
    pub fn phase_name(&self, state: &str, phase: impl std::fmt::Display) -> String {
        let k = phase.to_string();
        self.names.phases.get(state).and_then(|m| m.get(&k)).cloned().unwrap_or(k)
    }
}

static DEFS: OnceLock<HashMap<String, ActorDef>> = OnceLock::new();

fn load() -> HashMap<String, ActorDef> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/defs/actors");
    let mut out = HashMap::new();
    let Ok(rd) = std::fs::read_dir(&dir) else { return out; };
    for f in rd.flatten() {
        let p = f.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
        match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<ActorDef>(&s).map_err(|e| e.to_string())) {
            Ok(mut d) => {
                if d.id.is_empty() { d.id = p.file_stem().unwrap().to_string_lossy().into_owned(); }
                if std::env::var_os("MW_ACTOR_LOG").is_some() {
                    eprintln!("[actors] {} ({}): hp {} power {} {} attacks, {} anims, {} voices", d.id, d.kind, d.hp, d.power, d.attacks.len(), d.anims.len(), d.voices.len());
                }
                out.insert(d.id.clone(), d);
            }
            Err(e) => eprintln!("[actors] {} not loaded: {e}", p.display()),
        }
    }
    out
}

/// The definition for `id` (e.g. "em0101"); None when there is no file.
pub fn get(id: &str) -> Option<&'static ActorDef> { DEFS.get_or_init(load).get(id) }
/// Every definition with an editor page, by id.
pub fn with_editor() -> Vec<&'static ActorDef> {
    let mut v: Vec<&ActorDef> = DEFS.get_or_init(load).values().filter(|d| d.editor.is_some()).collect();
    v.sort_by(|a, b| a.id.cmp(&b.id));
    v
}
