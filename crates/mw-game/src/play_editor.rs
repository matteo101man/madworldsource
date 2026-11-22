//! The actor editor [added tooling, drawn in the game's own type]: MW_EDITOR=<actor id> (with that actor's MW_STAGE, or
//! from the stage select's ACTOR EDITOR list) opens the actor's room with the actor stood in front of the player, its AI
//! held, and a menu built from its definition (assets/defs/actors/<id>.json `editor`, play_actors.rs):
//! - the curated groups (attacks, hurts, deaths, taunts, the power struggle won and lost, grabs, finishers, ...): each
//!   entry is a command the actor's own module plays through its native code, so the clip's sound and effect cues, the
//!   hit blood and stains, the voice lines and the pair cameras all run as in a fight;
//! - every clip in its motion bank (by `clip_groups`), every voice line in `voices`, every camera clip in `cameras`.
//! The actor modules answer `Cmd`s (`Editor::take` / `Editor::done`); the editor itself plays voices, previews camera
//! clips, presses player buttons, runs the last-life execution and brings the player back after a death.
//! Keys: Up / Down pick, Left / Right change group, Enter plays, R resets, Space pauses, - / = speed, V camera (game,
//! orbit, shot), right mouse drag / wheel orbit, I the actor's AI on / off, F2 hands the keys to the player (and back),
//! F9 the stage select.
use bevy::{input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll}, prelude::*};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::Relaxed};

static FOCUS: AtomicBool = AtomicBool::new(false);
static REVIVE: AtomicBool = AtomicBool::new(false);
/// a player button the editor presses once (Press as u8; 0 none)
static PRESS: AtomicU8 = AtomicU8::new(0);

/// The actor being edited (MW_EDITOR).
pub fn actor_id() -> Option<String> { std::env::var("MW_EDITOR").ok().filter(|s| !s.trim().is_empty()) }
pub fn on() -> bool { actor_id().is_some() }
/// The menu holds the keys (the player stands still).
pub fn focus() -> bool { FOCUS.load(Relaxed) }
/// The editor brings the dead player back (play.rs: the continue answer).
pub fn take_revive() -> bool { REVIVE.swap(false, Relaxed) }
/// A player button the editor presses this tick (play.rs `tick`).
pub fn take_press() -> Option<Press> {
    match PRESS.swap(0, Relaxed) { 1 => Some(Press::Punch), 2 => Some(Press::Swing), 3 => Some(Press::Grab), 4 => Some(Press::Taunt), 5 => Some(Press::Jump), 6 => Some(Press::Dodge), _ => None }
}
/// Press a player button once (a module's pair that the player starts, e.g. the grab).
pub fn press(p: Press) { PRESS.store(p as u8, Relaxed); }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Press { Punch = 1, Swing, Grab, Taunt, Jump, Dodge }

/// What the actor's module is asked to play.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    /// a raw clip, once (back to idle) or looped
    Clip { name: String, looped: bool },
    /// a named native move (the module's own state: an attack, a taunt, the hide, ...)
    Move(String),
    /// struck: light, heavy, front, back, left, right, shot, down, launch
    Hurt(String),
    Kill,
    /// a pair with the player: clash, finisher, grab, pin ...; `win` forces the outcome (None: the player's input)
    Pair { name: String, win: Option<bool> },
    /// back to the editor's start: both stood at their spots, full HP, any pair ended
    Reset,
    /// a camera clip's scene: the actor's clip with its root travel, the player's paired clip from the same origin
    /// (the module reports the frame and origin in `Editor::scene` while it plays)
    Scene { actor: String, player: Option<String> },
}

#[derive(Clone, Debug)]
enum Action { Cmd(Cmd), Voice(u16), VoiceFile(String), Camera(String), Execution, Press(Press) }
#[derive(Clone, Debug)]
struct Item { label: String, note: String, action: Action }
struct Group { name: String, items: Vec<Item> }

#[derive(Clone, Copy, PartialEq, Debug)]
enum CamMode { Game, Orbit, Shot }

#[derive(Resource)]
pub struct Editor {
    pub on: bool,
    pub id: String,
    name: String,
    groups: Vec<Group>,
    g: usize, row: usize,
    /// the command waiting for the module, and the module's answer
    cmd: Option<Cmd>,
    status: (String, f64, bool),
    /// the actor's AI runs (off: it holds its idle between commands)
    pub ai: bool,
    /// the module's line (clip, frame, state) and where the actor stands (camera anchors)
    pub info: String,
    pub anchor: (Vec3, f32),
    /// the actor stands idle (nothing playing): the player's HP refills
    pub idle: bool,
    pub spot: Option<Vec3>, pub player_spot: Option<Vec3>,
    focus_h: f32,
    cam: CamMode, orbit: (f32, f32, f32),
    cams: Vec<(String, super::elise::CamClip)>,
    shot: Option<(usize, f32)>,
    /// a scene plays the previewed camera: the module's frame, origin and yaw (None: not playing)
    pub scene: Option<(f32, Vec3, f32)>,
    /// the previewed camera follows the scene's clock (else its own, on the anchor rule)
    shot_scene: bool, scene_seen: bool,
    /// the label of the menu entry whose command is waiting
    cmd_label: Option<String>,
    /// a camera clip with no scene rides the player (else the actor)
    cam_player: bool,
    speed: f32, paused: bool,
    voice: Option<u16>,
    /// a voice take to play as it is (one recording, not the announcer's pick)
    take: Option<String>,
    /// the execution entry is running (the player was laid down for it)
    executing: bool,
    dead_for: u32,
    /// the menu was built (the module's clips are in)
    built: bool,
    /// (RinRin, driven from here) set up; ticks the player's swing is held for her clash / finisher
    ready: bool, swing: u32, rin_dead: u32, down_for: u32,
    /// (RinRin) a camera scene's origin
    scene_origin: Option<(Vec3, f32)>,
    /// H: the menu hidden (a clean view)
    hidden: bool,
    /// ticks the waiting command has gone untaken (no module answers for this actor)
    waiting: u32,
}

impl Editor {
    /// The command for the module, if any.
    pub fn take(&mut self) -> Option<Cmd> { self.cmd.take() }
    /// The command, if it is one `f` accepts.
    pub fn take_if(&mut self, f: impl Fn(&Cmd) -> bool) -> Option<Cmd> { if self.cmd.as_ref().is_some_and(|c| f(c)) { self.cmd.take() } else { None } }
    /// The module's answer: Err shows in the status line.
    pub fn done(&mut self, cmd: &Cmd, r: Result<(), String>) {
        let ok = r.is_ok();
        // (said by the entry's own label when a menu entry sent it)
        let label = self.cmd_label.take();
        let msg = match r { Ok(()) => label.unwrap_or_else(|| describe(&self.id, cmd)), Err(e) => e };
        self.status = (msg, 0., !ok);
    }
    fn say(&mut self, msg: impl Into<String>, err: bool) { self.status = (msg.into(), 0., err); }
    fn item(&self) -> Option<&Item> { self.groups.get(self.g).and_then(|g| g.items.get(self.row)) }
    /// The editor's camera: a camera clip previewed (V shot), or the orbit (V orbit) when no pair camera plays.
    pub fn shot_camera(&self, play: &super::Play) -> Option<(Vec3, Vec3, f32)> {
        if !self.on || self.cam != CamMode::Shot { return None; }
        let (k, t) = self.shot?;
        let (_, cam) = self.cams.get(k)?;
        // in its scene: on the scene's clock and origin (the objects the camera was made around)
        if self.shot_scene { let (f, at, yaw) = self.scene?; return Some(cam.sample(f, at + Vec3::Y * 0.1, yaw)); }
        // alone: on the player (Elise's pair cameras ride him) or the actor
        let (at, yaw) = if self.cam_player { (Vec3::from_array(play.fight.player.pos), play.fight.player.loco.facing) } else { self.anchor };
        Some(cam.sample(t, at + Vec3::Y * 0.1, yaw))
    }
    pub fn orbit_camera(&self, play: &super::Play) -> Option<(Vec3, Vec3, f32)> {
        if !self.on || self.cam != CamMode::Orbit { return None; }
        let (a, facing) = self.anchor;
        let pl = Vec3::from_array(play.fight.player.pos);
        // around the actor, the yaw from its facing (0: in front of it); far out it takes in the player too
        let (yaw, pitch, dist) = self.orbit;
        let yaw = yaw + facing;
        let dir = Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos());
        // (the subject sits right of centre, clear of the menu on the left)
        let right = Vec3::new(-dir.z, 0., dir.x).normalize_or(Vec3::X);
        let target = a.lerp(pl, 0.3 * ((dist - 2.) / 3.).clamp(0., 1.)) + Vec3::Y * self.focus_h + right * dist * 0.17;
        Some((target + dir * dist, target, 0.85))
    }
}

fn describe(id: &str, cmd: &Cmd) -> String {
    match cmd {
        Cmd::Clip { name, looped } => format!("Playing {}{}", clip_label(id, name), if *looped { " (looped)" } else { "" }),
        Cmd::Move(m) => format!("Move {m}"),
        Cmd::Hurt(k) => format!("Hurt {k}"),
        Cmd::Kill => "Killed".into(),
        Cmd::Pair { name, win } => format!("Pair: {name}{}", match win { Some(true) => " (win)", Some(false) => " (loss)", None => "" }),
        Cmd::Reset => "Reset".into(),
        Cmd::Scene { actor, .. } => format!("Scene: {}", clip_label(id, actor)),
    }
}

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        let id = actor_id();
        let def = id.as_deref().and_then(super::actors::get);
        let ed = def.and_then(|d| d.editor.as_ref());
        let on = id.is_some();
        if on && def.is_none() { eprintln!("[editor] no definition assets/defs/actors/{}.json", id.as_deref().unwrap_or("")); }
        FOCUS.store(on, Relaxed);
        let mut cams = vec![];
        for f in ed.map(|e| e.cameras.clone()).unwrap_or_default() {
            let mut v: Vec<(String, super::elise::CamClip)> = super::elise::load_cams(&f).into_iter().collect();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            cams.extend(v);
        }
        app.insert_resource(Editor {
            on, id: id.clone().unwrap_or_default(), name: def.map_or(String::new(), |d| d.name.clone()), groups: vec![], g: 0, row: 0, cmd: None,
            status: (if on { "Ready".into() } else { String::new() }, 0., false), ai: false, info: String::new(), anchor: (Vec3::ZERO, 0.), idle: true,
            spot: ed.and_then(|e| e.spot).map(Vec3::from_array), player_spot: ed.and_then(|e| e.player_spot).map(Vec3::from_array),
            focus_h: ed.map_or(1.2, |e| if e.focus > 0. { e.focus } else { 1.2 }), cam: CamMode::Orbit, cams, shot: None, scene: None, shot_scene: false, scene_seen: false, cmd_label: None,
            cam_player: ed.is_some_and(|e| e.camera_anchor == "player"),
            // MW_EDITOR_ORBIT=yaw,pitch,dist starts the orbit there (tests)
            orbit: std::env::var("MW_EDITOR_ORBIT").ok().map(|v| v.split(',').filter_map(|x| x.trim().parse::<f32>().ok()).collect::<Vec<_>>())
                .filter(|v| v.len() == 3).map_or((0.75, 0.2, 5.0), |v| (v[0], v[1], v[2])),
            speed: 1., paused: false, voice: None, take: None, executing: false, dead_for: 0, built: false, ready: false, swing: 0, rin_dead: 0, down_for: 0, scene_origin: None, waiting: 0, hidden: false,
        })
        .add_systems(Update, (input, present, play_take));
    }
}

/// Build the menu from the definition (once).
fn build(ed: &mut Editor) {
    ed.built = true;
    let Some(def) = super::actors::get(&ed.id) else { return; };
    let page = def.editor.clone().unwrap_or_default();
    for g in &page.groups {
        let items = g.entries.iter().filter_map(|e| {
            let action = match e.action.as_str() {
                "clip" => Action::Cmd(Cmd::Clip { name: e.arg.clone(), looped: e.looped }),
                "move" => Action::Cmd(Cmd::Move(e.arg.clone())),
                "hurt" => Action::Cmd(Cmd::Hurt(if e.arg.is_empty() { "light".into() } else { e.arg.clone() })),
                "kill" => Action::Cmd(Cmd::Kill),
                "pair" => Action::Cmd(Cmd::Pair { name: e.arg.clone(), win: e.win }),
                "reset" => Action::Cmd(Cmd::Reset),
                "execution" => Action::Execution,
                "voice" => Action::Voice(def.voice(&e.arg).or_else(|| parse_id(&e.arg))?),
                "camera" => Action::Camera(e.arg.clone()),
                "input" => Action::Press(match e.arg.as_str() { "punch" => Press::Punch, "swing" => Press::Swing, "grab" => Press::Grab, "taunt" => Press::Taunt, "jump" => Press::Jump, _ => Press::Dodge }),
                other => { eprintln!("[editor] {}: unknown entry kind {other:?} ({})", def.id, e.label); return None; }
            };
            Some(Item { label: if e.label.is_empty() { e.arg.clone() } else { e.label.clone() }, note: e.note.clone(), action })
        }).collect();
        ed.groups.push(Group { name: g.name.clone(), items });
    }
    // the voice lines: every id in the stage's announcer table (named by the definition's `voices` where it names
    // them), played through the announcer's own pick; then every recording of each, one by one
    let room = match page.stage.as_str() { "elise" => Some("c303"), "baron" | "mathilda" => Some("c503"), _ => None };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let table: serde_json::Value = std::fs::read_to_string(root.join(room.map_or("native/audio/announce/announce.json".to_string(), |r| format!("native/rooms/{r}/announce/announce.json"))))
        .ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let mut ids: Vec<u16> = table.as_object().map(|o| o.keys().filter_map(|k| k.parse().ok()).collect()).unwrap_or_default();
    for id in def.voices.values() { if !ids.contains(id) { ids.push(*id); } }
    ids.sort();
    let name_of = |id: u16| def.names.voices.get(&id.to_string()).map(|v| v.name.clone()).filter(|n| !n.is_empty())
        .or_else(|| def.voices.iter().find(|v| *v.1 == id).map(|v| v.0.replace('_', " ")));
    let trigger_of = |id: u16| def.names.voices.get(&id.to_string()).map(|v| v.trigger.clone()).filter(|t| !t.is_empty());
    // (the definition's own lines first, then the room's others)
    ids.sort_by_key(|id| (name_of(*id).is_none(), *id));
    if !ids.is_empty() {
        ed.groups.push(Group { name: "Voice lines".into(), items: ids.iter().map(|&id| Item {
            label: name_of(id).unwrap_or_else(|| format!("Line {id:#X}")),
            note: format!("{:#X}: {}", id, trigger_of(id).unwrap_or_else(|| "no trigger named".into())), action: Action::Voice(id) }).collect() });
        let mut takes = vec![];
        for &id in &ids {
            let clips = table.get(id.to_string()).and_then(|e| e.get("clips")).and_then(|c| c.as_array()).cloned().unwrap_or_default();
            for (k, c) in clips.iter().enumerate() {
                let Some(file) = c.get("file").and_then(|f| f.as_str()) else { continue; };
                takes.push(Item { label: format!("{}, take {}", name_of(id).unwrap_or_else(|| format!("Line {id:#X}")), k + 1), note: file.rsplit('/').next().unwrap_or(file).into(), action: Action::VoiceFile(file.into()) });
            }
        }
        if !takes.is_empty() { ed.groups.push(Group { name: "Voice takes".into(), items: takes }); }
    }
    // the camera clips
    if !ed.cams.is_empty() {
        let items = ed.cams.iter().map(|(n, c)| {
            let named = def.names.cameras.get(n);
            let label = named.map(|c| c.name.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| n.trim_end_matches(".mot").to_string());
            let with = named.and_then(|c| c.entry.clone()).map(|e| format!("in {e}"))
                .or_else(|| named.and_then(|c| c.actor.clone()).map(|a| format!("with {}", def.clip_name(&a))))
                .unwrap_or_else(|| format!("{} frames", c.frames()));
            Item { label, note: format!("{}: {with}", n.trim_end_matches(".mot")), action: Action::Camera(n.clone()) }
        }).collect();
        ed.groups.push(Group { name: "Cameras".into(), items });
    }
    // every clip in the bank, sorted into the clip groups
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let names: Vec<String> = def.motions.as_ref().and_then(|m| std::fs::read_to_string(root.join(m)).ok())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("clips").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(String::from)).collect()))
        .unwrap_or_default();
    let mut rest: Vec<String> = names;
    rest.sort();
    for cg in &page.clip_groups {
        let (mine, others): (Vec<String>, Vec<String>) = rest.into_iter().partition(|n| cg.prefixes.iter().any(|p| n.starts_with(p.as_str())));
        rest = others;
        if !mine.is_empty() { ed.groups.push(clip_group(def, &format!("Clips: {}", cg.name), mine)); }
    }
    if !rest.is_empty() { ed.groups.push(clip_group(def, if page.clip_groups.is_empty() { "All clips" } else { "Clips: other" }, rest)); }
    ed.groups.retain(|g| !g.items.is_empty());
}
fn clip_group(def: &super::actors::ActorDef, name: &str, clips: Vec<String>) -> Group {
    Group { name: name.into(), items: clips.into_iter().map(|c| Item { label: def.clip_name(&c), note: c.trim_end_matches(".mot").into(), action: Action::Cmd(Cmd::Clip { name: c, looped: false }) }).collect() }
}

/// A clip of actor `id` in words (its definition's `names.clips`), else its file name.
pub fn clip_label(id: &str, clip: &str) -> String { super::actors::get(id).map_or_else(|| clip.trim_end_matches(".mot").to_string(), |d| d.clip_name(clip)) }
/// A numbered phase of one of `id`'s states in words.
pub fn phase_label(id: &str, state: &str, phase: impl std::fmt::Display) -> String {
    let k = phase.to_string();
    super::actors::get(id).map_or(k.clone(), |d| d.phase_name(state, k))
}
fn parse_id(s: &str) -> Option<u16> { if let Some(h) = s.strip_prefix("0x") { u16::from_str_radix(h, 16).ok() } else { s.parse().ok() } }

/// Update: a voice take, played as the announcer plays its lines (its bus and gain).
fn play_take(mut ed: ResMut<Editor>, assets: Res<AssetServer>, sound: Res<super::audio::PortAudio>, mut commands: Commands) {
    let Some(f) = ed.take.take() else { return; };
    commands.spawn((super::audio::Gain { bus: super::audio::Bus::Announcers, base: 1. }, AudioPlayer::new(assets.load::<AudioSource>(f)),
                    PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(sound.gain(super::audio::Bus::Announcers, 1.)))));
}

/// Update: the menu keys, the camera keys and mouse, time.
fn input(mut ed: ResMut<Editor>, keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, motion: Res<AccumulatedMouseMotion>,
         scroll: Res<AccumulatedMouseScroll>, real: Res<Time<Real>>, mut time: ResMut<Time<Virtual>>, pause: Res<super::pause::Pause>) {
    if !ed.on { return; }
    if !ed.built { build(&mut ed); }
    let now = real.elapsed_secs_f64();
    if ed.status.1 == 0. { ed.status.1 = now; }
    // the shot preview runs on its own clock (30 frames a second at the editor speed)
    // the scene played out: back to the orbit
    if ed.shot_scene { if ed.scene.is_some() { ed.scene_seen = true; } else if ed.scene_seen { ed.scene_seen = false; ed.shot_scene = false; ed.shot = None; if ed.cam == CamMode::Shot { ed.cam = CamMode::Orbit; } } }
    if let Some((k, t)) = ed.shot {
        let n = ed.cams.get(k).map_or(1., |c| (c.1.frames().max(2) - 1) as f32);
        let dt = if ed.paused { 0. } else { real.delta_secs() * 30. * ed.speed };
        ed.shot = Some((k, (t + dt) % n));
    }
    if pause.open { return; }
    if keys.just_pressed(KeyCode::F2) {
        let f = !focus(); FOCUS.store(f, Relaxed);
        ed.say(if f { "Editor has the keys (F2 gives them to the player)" } else { "Player has the keys (F2 for the editor)" }, false);
    }
    // the orbit: right mouse drag, the wheel
    if mouse.pressed(MouseButton::Right) && focus() {
        let d = motion.delta;
        ed.orbit.0 -= d.x * 0.006; ed.orbit.1 = (ed.orbit.1 + d.y * 0.004).clamp(-0.3, 1.2);
        if ed.cam == CamMode::Game { ed.cam = CamMode::Orbit; }
    }
    if scroll.delta.y != 0. && focus() { ed.orbit.2 = (ed.orbit.2 * (1. - scroll.delta.y * 0.08)).clamp(1.2, 14.); }
    if !focus() { return; }
    let n_groups = ed.groups.len().max(1);
    if keys.just_pressed(KeyCode::ArrowRight) { ed.g = (ed.g + 1) % n_groups; ed.row = 0; }
    if keys.just_pressed(KeyCode::ArrowLeft) { ed.g = (ed.g + n_groups - 1) % n_groups; ed.row = 0; }
    let n = ed.groups.get(ed.g).map_or(1, |g| g.items.len().max(1));
    if keys.just_pressed(KeyCode::ArrowDown) { ed.row = (ed.row + 1) % n; }
    if keys.just_pressed(KeyCode::ArrowUp) { ed.row = (ed.row + n - 1) % n; }
    if keys.just_pressed(KeyCode::PageDown) { ed.row = (ed.row + 10).min(n - 1); }
    if keys.just_pressed(KeyCode::PageUp) { ed.row = ed.row.saturating_sub(10); }
    if keys.just_pressed(KeyCode::KeyR) { ed.cmd = Some(Cmd::Reset); ed.shot = None; if ed.cam == CamMode::Shot { ed.cam = CamMode::Orbit; } }
    if keys.just_pressed(KeyCode::KeyI) { ed.ai = !ed.ai; let on = ed.ai; ed.say(if on { "AI on: the actor fights" } else { "AI off: the actor holds between commands" }, false); }
    if keys.just_pressed(KeyCode::KeyV) {
        ed.cam = match ed.cam { CamMode::Orbit => CamMode::Game, CamMode::Game => if ed.shot.is_some() { CamMode::Shot } else { CamMode::Orbit }, CamMode::Shot => CamMode::Orbit };
        let c = ed.cam; ed.say(format!("Camera: {}", match c { CamMode::Orbit => "orbit (pair cameras still play)", CamMode::Game => "game", CamMode::Shot => "camera clip" }), false);
    }
    if keys.just_pressed(KeyCode::Space) { ed.paused = !ed.paused; }
    if keys.just_pressed(KeyCode::KeyH) { ed.hidden = !ed.hidden; }
    if keys.just_pressed(KeyCode::Minus) { ed.speed = match ed.speed { s if s > 0.6 => 0.5, s if s > 0.3 => 0.25, _ => 0.1 }; }
    if keys.just_pressed(KeyCode::Equal) { ed.speed = match ed.speed { s if s < 0.2 => 0.25, s if s < 0.4 => 0.5, _ => 1. }; }
    if ed.paused { time.pause(); } else { time.unpause(); }
    // (the execution's slow cut sets its own speed; the editor's applies otherwise)
    if !ed.executing { time.set_relative_speed(ed.speed); }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        if let Some(item) = ed.item().cloned() { run(&mut ed, item); }
    }
}

/// Play a menu entry.
fn run(ed: &mut Editor, item: Item) {
        match item.action {
            Action::Cmd(c) => { ed.cmd = Some(c); ed.cmd_label = Some(item.label.clone()); }
            Action::Voice(id) => { ed.voice = Some(id); ed.say(format!("Voice {:#X}: {}", id, item.label), false); }
            Action::VoiceFile(f) => { ed.say(format!("Voice take {}", item.note), false); ed.take = Some(f); }
            Action::Camera(name) => {
                let Some(k) = ed.cams.iter().position(|c| c.0 == name) else { ed.say(format!("No camera clip {name}"), true); return; };
                let named = super::actors::get(&ed.id).and_then(|d| d.names.cameras.get(&name).cloned());
                // the entry whose scene it films: played as in a fight (the pair and boss cameras take the view)
                if let Some(path) = named.as_ref().and_then(|c| c.entry.clone()).filter(|e| !e.is_empty()) {
                    let (g, e) = path.split_once('/').unwrap_or(("", path.as_str()));
                    let found = ed.groups.iter().find(|gr| gr.name.eq_ignore_ascii_case(g)).and_then(|gr| gr.items.iter().find(|it| it.label.eq_ignore_ascii_case(e)).cloned());
                    match found {
                        Some(it) => { if ed.cam == CamMode::Shot { ed.cam = CamMode::Orbit; } ed.shot = None; let label = item.label.clone(); run(ed, it); ed.say(format!("{label}: plays in {path}"), false); }
                        None => ed.say(format!("{name}: its entry {path} is not on this page"), true),
                    }
                    return;
                }
                // a scene of its own (a camera the fight does not play): the actor's clip and the player's together
                if let Some(actor) = named.as_ref().and_then(|c| c.actor.clone()).filter(|a| !a.is_empty()) {
                    ed.cmd = Some(Cmd::Scene { actor, player: named.as_ref().and_then(|c| c.player.clone()).filter(|p| !p.is_empty()) });
                    ed.shot = Some((k, 0.)); ed.shot_scene = true; ed.scene = None; ed.cam = CamMode::Shot;
                    ed.say(format!("{} (V leaves it)", item.label), false);
                    return;
                }
                ed.shot = Some((k, 0.)); ed.shot_scene = false; ed.cam = CamMode::Shot;
                ed.say(format!("{} alone, on the {} (V leaves it)", item.label, if ed.cam_player { "player" } else { "actor" }), false);
            }
            Action::Execution => { ed.executing = true; ed.say("Execution: the player falls on his last life", false); }
            Action::Press(p) => { PRESS.store(p as u8, Relaxed); ed.say(format!("Player presses {p:?}"), false); }
        }
}

/// MW_EDITOR_RUN="tick:group/entry;..." plays entries by name at game ticks (the group and entry matched by their start,
/// any case; "reset" and "camera:orbit|game" also work) [for agents and tests].
fn script(ed: &mut Editor, tick: u64) {
    let Ok(list) = std::env::var("MW_EDITOR_RUN") else { return; };
    for step in list.split(';') {
        let Some((t, what)) = step.split_once(':') else { continue; };
        if t.trim().parse::<u64>().ok() != Some(tick) { continue; }
        let what = what.trim();
        match what.to_lowercase().as_str() {
            "reset" => { ed.cmd = Some(Cmd::Reset); continue; }
            "camera:orbit" | "camera" => { ed.cam = CamMode::Orbit; continue; }
            "camera:game" => { ed.cam = CamMode::Game; continue; }
            "hide" => { ed.hidden = !ed.hidden; continue; }
            _ => {}
        }
        let (g, e) = what.split_once('/').unwrap_or(("", what));
        let lc = |s: &str| s.to_lowercase();
        let found = ed.groups.iter().enumerate().filter(|(_, gr)| lc(&gr.name).starts_with(&lc(g)))
            .find_map(|(gi, gr)| gr.items.iter().position(|it| lc(&it.label).starts_with(&lc(e))).map(|ii| (gi, ii)));
        match found {
            Some((gi, ii)) => { ed.g = gi; ed.row = ii; let item = ed.groups[gi].items[ii].clone(); eprintln!("[editor] tick {tick}: {} / {}", ed.groups[gi].name, item.label); run(ed, item); }
            None => eprintln!("[editor] tick {tick}: no entry {what:?}"),
        }
    }
}

/// FixedUpdate, after `tick`: the editor's own pieces (voices, the execution, keeping the player alive).
pub fn step(mut ed: ResMut<Editor>, mut play: ResMut<super::Play>, ex: Res<super::execution::Execution>) {
    if !ed.on { return; }
    if !ed.built { build(&mut ed); }
    // MW_EDITOR_LOG=<n>: the module's line every n ticks (tests)
    if let Some(n) = std::env::var("MW_EDITOR_LOG").ok().and_then(|v| v.parse::<u64>().ok()) { if play.tick % n.max(1) == 0 { eprintln!("[editor] tick {} {}  | player hp {} state {}", play.tick, ed.info, play.fight.player.hp, play.fight.player.state()); } }
    // a command no module takes: this actor's module has no editor hooks (play_editor.rs says how to add them)
    if ed.cmd.is_some() { ed.waiting += 1; } else { ed.waiting = 0; }
    if ed.waiting > 15 { ed.cmd = None; ed.waiting = 0; let id = ed.id.clone(); ed.say(format!("No module answers for {id} here (its stage, or its editor hooks)"), true); }
    script(&mut ed, play.tick);
    if let Some(v) = ed.voice.take() { play.fight.events.push(mw_sim::fight::FightEvent::Voice(v)); }
    let p = &mut *play;
    let dead = p.fight.player.state() == 95;
    if ed.executing && !ex.started && !dead {
        // the last life: no continues left, laid down (play_execution.rs starts the boss's finisher 20 ticks on)
        p.continues = 0; p.fight.end_external(); p.fight.paired = false; p.fight.lie_dead(); p.dead_since = Some(p.tick);
    }
    if dead { ed.dead_for += 1; } else { ed.dead_for = 0; }
    // a death (or the execution played out): back up after a moment, both reset
    let holding = ex.started && !ex.done;
    if dead && !holding && ed.dead_for > if ex.done { 75 } else { 120 } {
        REVIVE.store(true, Relaxed); ed.executing = false; p.continues = super::continue_ui::CONTINUES;
        if ed.cmd.is_none() { ed.cmd = Some(Cmd::Reset); }
    }
    // knocked down, he gets up on any press (state 92): with the editor holding the keys it presses for him after 2 s
    if focus() && p.fight.player.state() == 92 { ed.down_for += 1; if ed.down_for > 60 { ed.down_for = 0; p.fight.get_up(); } } else { ed.down_for = 0; }
    // the player's HP refills while the actor stands idle
    if ed.idle && !dead && p.fight.player.state() != mw_sim::fight::STATE_EXTERNAL && p.fight.player.hp > 0 { p.fight.player.hp = mw_sim::fight::PLAYER_MAX_HP; }
}

// ---- the menu, in the game's own type ----

/// The message typeface (the subtitles' font, tools/ui/stage_subtitle_font.py) and the brush capitals (core_us page 2,
/// sprites 371-396 A-Z, 397 '!': the title lettering).
#[derive(serde::Deserialize)]
struct SubFont { pages: Vec<SubPage>, glyphs: Vec<SubGlyph>, chars: std::collections::HashMap<String, u16>, #[serde(rename = "baseAdvance")] advance: f32 }
#[derive(serde::Deserialize)]
struct SubPage { image: String }
#[derive(serde::Deserialize)]
struct SubGlyph { id: u16, page: usize, rect: [f32; 4] }
pub struct Fonts { msg: SubFont, glyph: std::collections::HashMap<u16, usize>, brush: std::collections::HashMap<char, [f32; 4]>, brush_image: String }

pub fn load_fonts() -> Option<Fonts> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/ui");
    let msg: SubFont = serde_json::from_str(&std::fs::read_to_string(root.join("subtitle-font/catalog.json")).ok()?).ok()?;
    let glyph = msg.glyphs.iter().enumerate().map(|(i, g)| (g.id, i)).collect();
    let layout: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("core_us/layout.json")).ok()?).ok()?;
    let mut brush = std::collections::HashMap::new();
    for s in layout.get("sprites")?.as_array()? {
        let id = s.get("id")?.as_u64()? as u32;
        let c = match id { 371..=396 => char::from(b'A' + (id - 371) as u8), 397 => '!', _ => continue };
        let r: Vec<f32> = s.get("rect")?.as_array()?.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();
        if r.len() == 4 { brush.insert(c, [r[0], r[1], r[2], r[3]]); }
    }
    Some(Fonts { msg, glyph, brush, brush_image: "native/ui/core_us/page-2.png".into() })
}

/// One glyph of a laid-out line (play_town.rs speech bubbles): its page image, its rect on the page, and where it sits
/// in the line (x, width, height, in the units of `size`).
pub struct Placed { pub image: String, pub rect: Rect, pub x: f32, pub w: f32, pub h: f32 }
impl Fonts {
    /// The message font's glyphs for `s` at line height `size` (characters it lacks advance as spaces), and the width.
    pub fn layout(&self, s: &str, size: f32) -> (Vec<Placed>, f32) {
        let f = &self.msg; let scale = size / 27.;
        let mut out = vec![]; let mut cx = 0.;
        for c in s.chars() {
            let Some(&i) = f.chars.get(&c.to_string()).and_then(|id| self.glyph.get(id)) else {
                cx += stroke_advance(c).map_or(f.advance * 0.4 * scale, |a| a * size); continue;
            };
            let g = &f.glyphs[i];
            let (gw, gh) = (g.rect[2] - g.rect[0], g.rect[3] - g.rect[1]);
            out.push(Placed { image: f.pages[g.page].image.clone(), rect: Rect::new(g.rect[0], g.rect[1], g.rect[2], g.rect[3]), x: cx, w: gw * scale, h: gh * scale });
            cx += (gw - 2.) * scale;
        }
        (out, cx)
    }
    /// The brush capitals for `s`, `h` tall: (rect on the brush page, x, width), and the width.
    pub fn brush_layout(&self, s: &str, h: f32) -> (Vec<(Rect, f32, f32)>, f32) {
        let mut out = vec![]; let mut cx = 0.;
        for c in s.to_uppercase().chars() {
            let Some(r) = self.brush.get(&c).copied() else { cx += h * 0.35; continue; };
            let w = (r[2] - r[0]) * h / (r[3] - r[1]);
            out.push((Rect::new(r[0], r[1], r[2], r[3]), cx, w));
            cx += w * 0.74;
        }
        (out, cx)
    }
    pub fn brush_image(&self) -> &str { &self.brush_image }
}

/// A node the painter drew: false the editor's menu, true the hold-controls panel (play_hold_ui.rs).
#[derive(Component)]
pub struct EdNode(pub bool);

/// Text in 640 x 360 screen units: the message font (`size`: the line height) or the brush capitals.
pub struct Painter<'a, 'w, 's> { pub commands: &'a mut Commands<'w, 's>, pub assets: &'a AssetServer, pub fonts: &'a Fonts, pub k: f32, pub offset: Vec2, pub layer: bool,
    /// [added] draw under this node instead of the screen (the VR menu's texture, play_vr_menu.rs)
    pub parent: Option<Entity> }
impl Painter<'_, '_, '_> {
    fn put(&mut self, b: impl Bundle) {
        match self.parent { Some(p) => { self.commands.spawn((b, ChildOf(p))); } None => { let layer = self.layer; self.commands.spawn((b, EdNode(layer))); } }
    }
    fn node(&self, x: f32, y: f32, w: f32, h: f32) -> Node {
        Node { position_type: PositionType::Absolute, left: px(self.offset.x + x * self.k), top: px(self.offset.y + y * self.k), width: px(w * self.k), height: px(h * self.k), ..default() }
    }
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color, z: i32) {
        let n = self.node(x, y, w, h);
        self.put((n, BackgroundColor(color), GlobalZIndex(z)));
    }
    pub fn width(&self, s: &str, size: f32) -> f32 {
        let f = &self.fonts.msg; let scale = size / 27.;
        s.chars().map(|c| match f.chars.get(&c.to_string()).and_then(|id| self.fonts.glyph.get(id)) {
            Some(&i) => (f.glyphs[i].rect[2] - f.glyphs[i].rect[0] - 2.) * scale,
            None => stroke_advance(c).map_or(f.advance * 0.4 * scale, |a| a * size),
        }).sum()
    }
    /// A straight stroke centred at (cx, cy), `len` long and `thick` wide, turned `deg` clockwise from upright.
    fn stroke(&mut self, cx: f32, cy: f32, len: f32, thick: f32, deg: f32, color: Color) {
        let n = self.node(cx - thick * 0.5, cy - len * 0.5, thick, len);
        self.put((n, BackgroundColor(color), UiTransform::from_rotation(Rot2::degrees(deg)), GlobalZIndex(9010)));
    }
    /// The characters the message font lacks, drawn as strokes in its weight: _ / < > | +
    fn stroked(&mut self, c: char, x: f32, y: f32, size: f32, color: Color) -> bool {
        let (s, t) = (size, size * 0.085);
        match c {
            '_' => { let n = self.node(x + s * 0.02, y + s * 0.8, s * 0.46, t); self.put((n, BackgroundColor(color), GlobalZIndex(9010))); }
            '/' => self.stroke(x + s * 0.2, y + s * 0.5, s * 0.72, t, 22., color),
            '|' => self.stroke(x + s * 0.12, y + s * 0.5, s * 0.8, t, 0., color),
            '<' => { self.stroke(x + s * 0.21, y + s * 0.4, s * 0.34, t, 58., color); self.stroke(x + s * 0.21, y + s * 0.6, s * 0.34, t, -58., color); }
            '>' => { self.stroke(x + s * 0.21, y + s * 0.4, s * 0.34, t, -58., color); self.stroke(x + s * 0.21, y + s * 0.6, s * 0.34, t, 58., color); }
            '+' => { self.stroke(x + s * 0.22, y + s * 0.52, s * 0.36, t, 0., color); self.stroke(x + s * 0.22, y + s * 0.52, s * 0.36, t, 90., color); }
            _ => return false,
        }
        true
    }
    /// The message font; clipped at `max` width with an ellipsis. Returns the width drawn.
    pub fn text(&mut self, s: &str, x: f32, y: f32, size: f32, color: Color, max: f32) -> f32 {
        let f = &self.fonts.msg; let scale = size / 27.;
        let mut cx = x;
        for c in s.chars() {
            let Some(&i) = f.chars.get(&c.to_string()).and_then(|id| self.fonts.glyph.get(id)) else {
                if let Some(a) = stroke_advance(c) { if cx + a * size - x > max { break; } self.stroked(c, cx, y, size, color); cx += a * size; }
                else { cx += f.advance * 0.4 * scale; }
                continue;
            };
            let g = &f.glyphs[i];
            let (gw, gh) = (g.rect[2] - g.rect[0], g.rect[3] - g.rect[1]);
            if cx + gw * scale - x > max { break; }
            let mut img = ImageNode::new(self.assets.load(f.pages[g.page].image.clone())).with_color(color);
            img.rect = Some(Rect::new(g.rect[0], g.rect[1], g.rect[2], g.rect[3]));
            let n = self.node(cx, y, gw * scale, gh * scale);
            self.put((img, n, GlobalZIndex(9010)));
            cx += (gw - 2.) * scale;
        }
        cx - x
    }
    /// The brush capitals (letters, '!' and spaces), `h` tall.
    pub fn brush(&mut self, s: &str, x: f32, y: f32, h: f32, color: Color) -> f32 {
        let mut cx = x;
        for c in s.to_uppercase().chars() {
            let Some(r) = self.fonts.brush.get(&c).copied() else { cx += h * 0.35; continue; };
            let scale = h / (r[3] - r[1]);
            let w = (r[2] - r[0]) * scale;
            let mut img = ImageNode::new(self.assets.load(self.fonts.brush_image.clone())).with_color(color);
            img.rect = Some(Rect::new(r[0], r[1], r[2], r[3]));
            let n = self.node(cx, y, w, h);
            self.put((img, n, GlobalZIndex(9010)));
            cx += w * 0.74;
        }
        cx - x
    }
}

/// How far a stroked character advances (in line heights).
fn stroke_advance(c: char) -> Option<f32> { match c { '_' => Some(0.5), '/' => Some(0.42), '|' => Some(0.26), '<' | '>' => Some(0.44), '+' => Some(0.46), _ => None } }

const ROWS: usize = 13;
const RED: Color = Color::srgb(0.78, 0.04, 0.04);
const INK: Color = Color::srgba(0., 0., 0., 0.78);

/// Update: the menu (rebuilt when what it shows changes).
fn present(ed: Res<Editor>, real: Res<Time<Real>>, windows: Query<&Window>, assets: Res<AssetServer>, nodes: Query<(Entity, &EdNode)>,
           mut fonts: Local<Option<Option<Fonts>>>, mut last: Local<String>, mut commands: Commands) {
    if !ed.on { return; }
    let Ok(window) = windows.single() else { return; };
    if fonts.is_none() { *fonts = Some(load_fonts()); }
    let Some(Some(fonts)) = fonts.as_ref() else { return; };
    let now = real.elapsed_secs_f64();
    let status_on = now - ed.status.1 < 4.;
    let key = format!("{}|{}|{}|{}|{}|{}|{}|{:?}|{}|{}|{}|{}x{}|{}", ed.hidden, ed.g, ed.row, ed.info, ed.status.0, status_on, ed.ai, ed.cam, ed.speed, ed.paused, focus(), window.width(), window.height(), ed.groups.len());
    if *last == key { return; }
    *last = key;
    for (e, n) in &nodes { if !n.0 { commands.entity(e).despawn(); } }
    if ed.hidden { return; }
    let viewport = Vec2::new(window.width(), window.height());
    let k = (viewport.x / 640.).min(viewport.y / 360.);
    let offset = (viewport - Vec2::new(640., 360.) * k) * 0.5;
    let mut p = Painter { commands: &mut commands, assets: &assets, fonts, k, offset, layer: false, parent: None };
    let dim = Color::srgb(0.62, 0.62, 0.62);
    // the panel: an ink block, a red rule under the title
    // (under the boss gauge, over where the radar was)
    let (px0, py0, pw, ph) = (10., 52., 214., 276.);
    p.rect(px0, py0, pw, ph, INK, 9000);
    p.rect(px0, py0, 3., ph, RED, 9001);
    p.brush("ACTOR EDITOR", px0 + 10., py0 + 6., 15., RED);
    // the name in the brush capitals (letters only: "Elise (c303 boss)" -> ELISE), the id and the rest under it
    let title: String = ed.name.split(" (").next().unwrap_or(&ed.id).to_string();
    let title = if title.chars().any(|c| c.is_ascii_alphabetic()) { title } else { ed.id.clone() };
    p.brush(&title, px0 + 10., py0 + 23., 22., Color::WHITE);
    let sub = format!("{}   {}", ed.id, ed.name.split_once(" (").map_or("", |x| x.1.trim_end_matches(')')));
    p.text(&sub, px0 + 10., py0 + 47., 9., dim, pw - 16.);
    p.rect(px0 + 10., py0 + 60., pw - 20., 1.5, RED, 9001);
    if ed.groups.is_empty() {
        p.text("No editor page for this actor.", px0 + 10., py0 + 70., 10., Color::WHITE, pw - 16.);
        p.text("Add \"editor\" to its definition.", px0 + 10., py0 + 84., 10., dim, pw - 16.);
    } else {
        let g = &ed.groups[ed.g];
        // the group: < name >  k / n
        p.text("<", px0 + 10., py0 + 65., 12., RED, 20.);
        let gw = p.width(&g.name, 12.).min(pw - 70.);
        p.text(&g.name, px0 + 22., py0 + 65., 12., Color::WHITE, pw - 70.);
        p.text(">", px0 + 30. + gw, py0 + 65., 12., RED, 20.);
        let count = format!("{} / {}", ed.g + 1, ed.groups.len());
        let cw = p.width(&count, 9.);
        p.text(&count, px0 + pw - 8. - cw, py0 + 67., 9., dim, 40.);
        let first = ed.row.saturating_sub(ROWS / 2).min(g.items.len().saturating_sub(ROWS));
        for (i, item) in g.items.iter().enumerate().skip(first).take(ROWS) {
            let y = py0 + 84. + (i - first) as f32 * 13.;
            let sel = i == ed.row;
            if sel { p.rect(px0 + 6., y - 1., pw - 12., 12.5, RED, 9002); }
            p.text(&item.label, px0 + 12., y, 10.5, if sel { Color::WHITE } else { Color::srgb(0.86, 0.86, 0.86) }, pw - 24.);
        }
        if g.items.len() > ROWS {
            let frac = ed.row as f32 / (g.items.len() - 1).max(1) as f32;
            p.rect(px0 + pw - 5., py0 + 84., 2., ROWS as f32 * 13., Color::srgba(1., 1., 1., 0.15), 9001);
            p.rect(px0 + pw - 5., py0 + 84. + frac * (ROWS as f32 * 13. - 12.), 2., 12., RED, 9002);
        }
        if let Some(item) = ed.item() {
            if !item.note.is_empty() { p.text(&item.note, px0 + 10., py0 + ph - 16., 9., dim, pw - 16.); }
        }
    }
    // the bottom strip: what plays, the clock, the camera, the AI; the status line over it
    // (left of the player's gauge)
    p.rect(0., 334., 452., 26., INK, 9000);
    p.rect(0., 334., 452., 1.5, RED, 9001);
    p.text(&ed.info, 12., 340., 11., Color::WHITE, 250.);
    let cam = match ed.cam { CamMode::Orbit => "ORBIT", CamMode::Game => "GAME", CamMode::Shot => "SHOT" };
    let right = format!("{}x{}   cam {}   AI {}   {}", ed.speed, if ed.paused { " PAUSED" } else { "" }, cam, if ed.ai { "on" } else { "off" }, if focus() { "EDITOR KEYS" } else { "PLAYER KEYS" });
    let rw = p.width(&right, 10.);
    p.text(&right, 444. - rw, 341., 10., dim, 200.);
    if status_on && !ed.status.0.is_empty() {
        let w = p.width(&ed.status.0, 11.).min(380.);
        p.rect(320. - w * 0.5 - 8., 312., w + 16., 17., INK, 9000);
        p.text(&ed.status.0, 320. - w * 0.5, 315., 11., if ed.status.2 { Color::srgb(1., 0.35, 0.3) } else { Color::WHITE }, 380.);
    }
    // the keys
    let help = ["UP DOWN pick   ENTER play", "LEFT RIGHT group   R reset", "SPACE pause   - = speed", "V camera   right drag orbit", "I AI  H hide  F2 keys  F9 stages"];
    p.rect(486., 10., 144., help.len() as f32 * 11. + 8., INK, 9000);
    for (i, h) in help.iter().enumerate() { p.text(h, 492., 14. + i as f32 * 11., 9., dim, 136.); }
}

// ---- RinRin (em0440): her module is the fight itself (mw_sim::fight, play.rs) ----

/// Her editor commands and hold, from play.rs `tick` (where her clash and finisher take the player's swing).
pub(super) fn rinrin(p: &mut super::Play, inp: &mut mw_sim::fight::FightInput, ed: &mut Editor, arena: &super::Arena) {
    use mw_sim::fight::rin_state as rs;
    if !ed.on || ed.id != "em0440" || p.fight.solo { return; }
    p.fight.rin_hold = !ed.ai; p.fight.editing = true;
    // a finish played out (she is dead): back up after a moment
    if p.fight.rin.state == rs::DEAD { ed.rin_dead += 1; if ed.rin_dead > 60 { ed.rin_dead = 0; rin_reset(p, ed, arena); ed.say("Reset", false); } } else { ed.rin_dead = 0; }
    if !ed.ready || ed.take_if(|c| *c == Cmd::Reset).is_some() { ed.ready = true; rin_reset(p, ed, arena); ed.done(&Cmd::Reset, Ok(())); }
    // the player's swing that opens her clash or finisher, held until it does
    // (on finish_wait only once her collapse has played out: the finisher opens on her lying)
    if ed.swing > 0 {
        ed.swing -= 1;
        if matches!(p.fight.rin.state, rs::KEEN_COMPE | rs::FINISH) { ed.swing = 0; }
        else if p.fight.rin.state != rs::FINISH_WAIT || p.fight.rin.motion.ended { inp.swing = 1; }
    }
    // (what her command raises, her voice lines, goes out with the coming fight step: the step clears `events`)
    let n0 = p.fight.events.len();
    if let Some(cmd) = ed.take() { let r = rin_cmd(p, &cmd, ed); ed.done(&cmd, r); }
    let raised: Vec<_> = p.fight.events.drain(n0..).collect();
    p.fight.pending.extend(raised);
    // a camera scene: her clip's clock from where it began; over, the pair ends
    match ed.scene_origin {
        Some((at, yaw)) if p.fight.rin.state == rs::EDITOR => ed.scene = Some((p.fight.rin.motion.frame, at, yaw)),
        Some(_) => { ed.scene_origin = None; ed.scene = None; if p.fight.player.state() == mw_sim::fight::STATE_EXTERNAL { p.fight.end_external(); } p.fight.paired = false; }
        None => {}
    }
    let r = &p.fight.rin;
    let name = clip_label("em0440", p.fd.rin.clips.get(r.motion.clip).map_or("", |c| c.name.as_str()));
    let n = p.fd.rin.clips.get(r.motion.clip).map_or(1, |c| c.frames.max(2) - 1);
    let st = rin_state_name(r.state);
    let phase = if matches!(r.state, rs::KEEN_COMPE | rs::FINISH | rs::EXECUTE) { format!(": {}", phase_label("em0440", st, r.phase)) } else { String::new() };
    ed.info = format!("{name}   {:.0} / {n}   {}{phase}   HP {}", r.motion.frame, phase_label("em0440", "state", st), r.hp);
    ed.anchor = (Vec3::from_array(r.pos), r.facing);
    ed.idle = r.state == rs::WAIT && !p.fight.paired;
}

fn rin_state_name(s: u8) -> &'static str {
    use mw_sim::fight::rin_state as rs;
    match s { rs::WAIT => "WAIT", rs::RUN => "RUN", rs::URAKEN => "URAKEN", rs::TETUZANKOU => "TETUZANKOU", rs::RIGHT_STEP => "RIGHT_STEP", rs::LEFT_STEP => "LEFT_STEP",
              rs::BACK_STEP => "BACK_STEP", rs::THROW => "THROW", rs::DM_EXPLOSION => "DM_EXPLOSION", rs::DM_RIGHT => "DM_RIGHT", rs::DM_LEFT => "DM_LEFT", rs::DM_BODY => "DM_BODY",
              rs::PROVOCATION => "PROVOCATION", rs::FINISH_WAIT => "FINISH_WAIT", rs::SLALOM => "SLALOM", rs::GUARD => "GUARD", rs::PL_DEAD => "PL_DEAD", rs::FINISH => "FINISH",
              rs::KEEN_COMPE => "KEEN_COMPE", rs::EXECUTE => "EXECUTE", rs::DM_DOWN => "DM_DOWN", rs::DM_ELECTRIC => "DM_ELECTRIC", rs::DM_GIANTSWING => "DM_GIANTSWING",
              rs::EDITOR => "EDITOR", _ => "?" }
}

fn rin_reset(p: &mut super::Play, ed: &Editor, arena: &super::Arena) {
    if p.fight.player.state() == mw_sim::fight::STATE_EXTERNAL { p.fight.end_external(); }
    let pp = ed.player_spot.unwrap_or(Vec3::from_array(p.fight.player.pos));
    let fy = |v: Vec3| super::floor(&arena.0, v + Vec3::Y * 2.).unwrap_or(v.y);
    let f = p.fight.player.loco.facing;
    let at = ed.spot.unwrap_or(pp + Vec3::new(f.sin(), 0., f.cos()) * 3.5);
    p.fight.player.pos = [pp.x, fy(pp), pp.z];
    p.fight.player.loco.facing = (at.x - pp.x).atan2(at.z - pp.z);
    p.fight.player.hp = mw_sim::fight::PLAYER_MAX_HP;
    p.cam_yaw = p.fight.player.loco.facing;
    p.fight.rin_reset([at.x, fy(at), at.z], (pp.x - at.x).atan2(pp.z - at.z));
}

fn rin_cmd(p: &mut super::Play, cmd: &Cmd, ed: &mut Editor) -> Result<(), String> {
    use mw_sim::fight::rin_state as rs;
    let busy = matches!(p.fight.rin.state, rs::KEEN_COMPE | rs::FINISH | rs::EXECUTE) || p.fight.paired;
    if busy { return Err("Still playing a pair: R resets".into()); }
    let pf = p.fight.player.loco.facing;
    let pp = p.fight.player.pos;
    // in front of him at `d` m, facing him
    let close = |p: &mut super::Play, d: f32| { p.fight.rin.pos = [pp[0] + pf.sin() * d, p.fight.rin.pos[1], pp[2] + pf.cos() * d]; p.fight.rin.facing = pf + std::f32::consts::PI; };
    match cmd {
        Cmd::Reset => {}
        Cmd::Clip { name, looped } => {
            let Some(i) = p.fd.rin.clips.iter().position(|c| &c.name == name) else { return Err(format!("{name} is not in her bank")); };
            p.fight.rin_play(i, *looped);
        }
        Cmd::Move(m) => {
            let st = match m.as_str() {
                "uraken" => { close(p, 1.6); rs::URAKEN }
                "tetuzankou" => { close(p, 1.6); rs::TETUZANKOU }
                "taunt" | "provocation" => rs::PROVOCATION,
                "guard" => rs::GUARD,
                "slalom" => { close(p, 2.); rs::SLALOM }
                "right_step" => rs::RIGHT_STEP,
                "left_step" => rs::LEFT_STEP,
                "back_step" => rs::BACK_STEP,
                "fans" | "throw" => rs::THROW,
                other => return Err(format!("RinRin has no move {other:?} (uraken, tetuzankou, slalom, fans, taunt, guard, right_step, left_step, back_step)")),
            };
            p.fight.rin_start(st);
        }
        Cmd::Hurt(k) => {
            let (atk, power) = match k.as_str() {
                "light" | "right" => (0x6D, 10), "left" => (0x6E, 10), "down" | "launch" => (0x76, 30),
                "shot" => return Err("Aim and fire yourself (F2, hold the right mouse button)".into()),
                _ => (0x6F, 25),
            };
            let pp = &mut *p; pp.fight.rin_hurt(&pp.fd, atk, power);
        }
        Cmd::Kill => { let hp = p.fight.rin.hp as i32; let pp = &mut *p; pp.fight.rin_hurt(&pp.fd, 0x6F, hp + 10); }
        Cmd::Scene { actor, player } => {
            let Some(i) = p.fd.rin.clips.iter().position(|c| &c.name == actor) else { return Err(format!("{actor} is not in her bank")); };
            p.fight.rin_play(i, false);
            if let Some(pc) = player { let pp = &mut *p; pp.fight.external(&pp.fd, pc, false, 0.); pp.fight.paired = true; pp.fight.player.pos = pp.fight.rin.pos; pp.fight.player.loco.facing = pp.fight.rin.facing; }
            ed.scene_origin = Some((Vec3::from_array(p.fight.rin.pos), p.fight.rin.facing));
        }
        Cmd::Pair { name, win } => match name.as_str() {
            // her giant slalom's keen window, his swing into it: the clash (won, lost, or the player's own)
            "clash" => { close(p, 1.2); p.fight.rin_start(rs::SLALOM); p.fight.rin.rig0 = true; p.fight.keen_auto = *win; ed.swing = 30; }
            // finish-ready, his swing: the finisher
            "finisher" => { close(p, 1.2); p.fight.rin.hp = 1; p.fight.rin_start(rs::FINISH_WAIT); ed.swing = 150; p.fight.keen_auto = Some(true); }
            other => return Err(format!("RinRin has no pair {other:?} (clash, finisher)")),
        },
    }
    Ok(())
}
