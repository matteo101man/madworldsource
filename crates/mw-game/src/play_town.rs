//! [added, not original] MW_STAGE=town: a free-roam test of Asia Town by day. No waves: the streets fill with townsfolk
//! (the ending crowd em0915 / em0925 / em0935 / em0955, textured from their siblings by tools/town/stage_civilians.py and
//! run as street actors in play_street.rs) who walk the streets on a baked walk grid (tools/town/stage_nav.py, A* here),
//! stand about and chat in pairs; they can be hit, grabbed and killed, run from blood, gunfire and swings, and the player
//! walking into one knocks him aside ("watch it!").
//!
//! Substories (after Yakuza's): here and there a pair of the kung-fu zako picks on a civilian. Come within 10 m and their
//! exchange plays in speech bubbles (gone again as you walk off); within 4.5 m ENTER steps in: a short cut-scene (the
//! controls held, letterbox, its own camera shots and lines) and the zako turn on the player. Beat them and the victim
//! thanks him; a new substory starts elsewhere a while later.
//!
//! Speech bubbles are drawn in the game's own UI look (the pause menu's AREA DATA box: dark panel, thin white rules, a white
//! name bar) with the subtitle message font, typed out a letter at a time, held, then faded.
//! Inspection: `MW_TOWN_LOG=1` logs the town, `MW_TOWN_CIVS=n` sets the population, `MW_TOWN_SUB=n` puts the player by
//! substory n at the start.
use bevy::prelude::*;
use super::street::{CivEvent, Role, Street};
use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};

pub fn on() -> bool { super::street::town() }

/// The cut-scene holds the controls (play.rs input / tick) and the camera (play.rs camera); a taunt it asks of the player.
static LOCK: AtomicBool = AtomicBool::new(false);
static TAUNT: AtomicBool = AtomicBool::new(false);
static CAMERA: Mutex<Option<(Vec3, Vec3, f32)>> = Mutex::new(None);
pub fn locked() -> bool { LOCK.load(Ordering::Relaxed) }
pub fn take_taunt() -> bool { TAUNT.swap(false, Ordering::Relaxed) }
pub fn camera() -> Option<(Vec3, Vec3, f32)> { *CAMERA.lock().unwrap() }

// ---- the walk grid ----

/// The baked walk grid (assets/native/world/town-nav.json): each cell's floor height, NaN where nobody walks.
pub struct Nav { x0: f32, z0: f32, cell: f32, w: usize, h: usize, floor: Vec<f32>, open: Vec<u32> }

impl Nav {
    fn load() -> Option<Nav> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/world/town-nav.json");
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root).ok()?).ok()?;
        let floor: Vec<f32> = v["floor"].as_array()?.iter().map(|x| x.as_f64().map_or(f32::NAN, |f| f as f32)).collect();
        let open = floor.iter().enumerate().filter(|(_, f)| f.is_finite()).map(|(i, _)| i as u32).collect();
        Some(Nav { x0: v["origin"][0].as_f64()? as f32, z0: v["origin"][1].as_f64()? as f32, cell: v["cell"].as_f64()? as f32,
                   w: v["width"].as_u64()? as usize, h: v["height"].as_u64()? as usize, floor, open })
    }
    fn at(&self, i: usize) -> Vec3 { Vec3::new(self.x0 + ((i % self.w) as f32 + 0.5) * self.cell, self.floor[i], self.z0 + ((i / self.w) as f32 + 0.5) * self.cell) }
    fn raw(&self, p: Vec3) -> Option<(i32, i32)> {
        let (x, z) = (((p.x - self.x0) / self.cell).floor() as i32, ((p.z - self.z0) / self.cell).floor() as i32);
        (x >= 0 && z >= 0 && (x as usize) < self.w && (z as usize) < self.h).then_some((x, z))
    }
    fn ok(&self, x: i32, z: i32) -> bool { x >= 0 && z >= 0 && (x as usize) < self.w && (z as usize) < self.h && self.floor[z as usize * self.w + x as usize].is_finite() }
    /// The walkable cell at or nearest `p` (within 3 m).
    fn cell_of(&self, p: Vec3) -> Option<usize> {
        let (x, z) = self.raw(p).unwrap_or((((p.x - self.x0) / self.cell) as i32, ((p.z - self.z0) / self.cell) as i32));
        for r in 0..=6i32 {
            let mut best: Option<(usize, f32)> = None;
            for dz in -r..=r { for dx in -r..=r {
                if dx.abs() != r && dz.abs() != r { continue; }
                if self.ok(x + dx, z + dz) {
                    let i = (z + dz) as usize * self.w + (x + dx) as usize;
                    let d = self.at(i).distance(p);
                    if best.is_none_or(|b| d < b.1) { best = Some((i, d)); }
                }
            } }
            if let Some((i, _)) = best { return Some(i); }
        }
        None
    }
    pub fn walkable(&self, p: Vec3) -> bool { self.raw(p).is_some_and(|(x, z)| self.ok(x, z)) }
    /// A walkable point picked by `r`.
    pub fn random(&self, r: u32) -> Vec3 { if self.open.is_empty() { Vec3::ZERO } else { self.at(self.open[r as usize % self.open.len()] as usize) } }
    /// Whether a straight walk from a to b stays on walkable cells with no step over 0.4 m.
    fn clear(&self, a: Vec3, b: Vec3) -> bool {
        let n = (a.distance(b) / (self.cell * 0.4)).ceil().max(1.) as usize;
        let mut last = a.y;
        for k in 0..=n {
            let p = a.lerp(b, k as f32 / n as f32);
            // the body's width too: the cells a body radius to each side
            let side = Vec3::new(b.z - a.z, 0., a.x - b.x).normalize_or_zero() * 0.3;
            for q in [p, p + side, p - side] {
                let Some((x, z)) = self.raw(q) else { return false; };
                if !self.ok(x, z) { return false; }
            }
            let Some((x, z)) = self.raw(p) else { return false; };
            let y = self.floor[z as usize * self.w + x as usize];
            if (y - last).abs() > 0.4 { return false; }
            last = y;
        }
        true
    }
    /// An A* path (8 ways, no corner cutting) from `from` to `to`, smoothed to the waypoints a walker needs; None when
    /// there is no way.
    pub fn path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let (s, t) = (self.cell_of(from)?, self.cell_of(to)?);
        if s == t { return Some(vec![self.at(t)]); }
        let n = self.w * self.h;
        let mut cost = vec![f32::INFINITY; n];
        let mut came = vec![u32::MAX; n];
        let mut heap = std::collections::BinaryHeap::new();
        let (tx, tz) = ((t % self.w) as f32, (t / self.w) as f32);
        let hcost = |i: usize| { let (dx, dz) = (((i % self.w) as f32 - tx).abs(), ((i / self.w) as f32 - tz).abs()); dx.max(dz) + 0.414 * dx.min(dz) };
        cost[s] = 0.;
        heap.push((std::cmp::Reverse((hcost(s) * 1000.) as u32), s as u32));
        let mut steps = 0;
        while let Some((_, i)) = heap.pop() {
            let i = i as usize;
            if i == t { break; }
            steps += 1;
            if steps > 40_000 { return None; }
            let (x, z) = ((i % self.w) as i32, (i / self.w) as i32);
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                if !self.ok(x + dx, z + dz) { continue; }
                if dx != 0 && dz != 0 && (!self.ok(x + dx, z) || !self.ok(x, z + dz)) { continue; }
                let j = (z + dz) as usize * self.w + (x + dx) as usize;
                let dy = (self.floor[j] - self.floor[i]).abs();
                if dy > 0.4 { continue; }
                let c = cost[i] + if dx != 0 && dz != 0 { 1.414 } else { 1. } + dy * 4.;
                if c < cost[j] { cost[j] = c; came[j] = i as u32; heap.push((std::cmp::Reverse(((c + hcost(j)) * 1000.) as u32), j as u32)); }
            }
        }
        if came[t] == u32::MAX { return None; }
        let mut cells = vec![t];
        while *cells.last().unwrap() != s { let p = came[*cells.last().unwrap()] as usize; cells.push(p); }
        cells.reverse();
        let pts: Vec<Vec3> = cells.iter().map(|&i| self.at(i)).collect();
        // string pulling: keep a waypoint only where the straight line to the next one would leave the grid
        let mut out = vec![];
        let mut a = from;
        let mut k = 0;
        while k < pts.len() - 1 {
            let mut far = k + 1;
            for j in (k + 1..pts.len()).rev() { if self.clear(a, pts[j]) { far = j; break; } }
            out.push(pts[far]); a = pts[far]; k = far;
        }
        if out.is_empty() { out.push(*pts.last().unwrap()); }
        Some(out)
    }
}

// ---- the town ----

/// A speaker id for the player.
const PLAYER: u32 = u32::MAX;
const TALK_NEAR: f32 = 10.;
const TALK_GONE: f32 = 12.5;
const PROMPT_NEAR: f32 = 4.5;

/// A substory's script [added, original lines]: its title, the exchange (speaker 0 the victim, 1 / 2 the zako) and the
/// zako's line when the player steps in.
struct Script { title: &'static str, talk: &'static [(u8, &'static str)], answer: &'static str, thanks: &'static str }
const SCRIPTS: [Script; 4] = [
    Script { title: "Lunch Money", talk: &[(1, "Hand over the cash, old man."), (0, "P-please... it's all I have!"), (2, "Heh. Then you won't miss it."), (1, "Pockets. Now."), (0, "Somebody, help!")],
             answer: "Huh?! You want some too, tough guy?!", thanks: "Th-thank you, mister!" },
    Script { title: "Wrong Street", talk: &[(1, "You lost? This is our turf."), (0, "I just want to get home!"), (2, "Toll's fifty bucks. Pay up."), (0, "Leave me alone!"), (1, "Or what? Who's gonna stop us?")],
             answer: "Mind your own business, freak!", thanks: "You saved me... thank you!" },
    Script { title: "Late Rent", talk: &[(2, "Rent's late again, pal."), (0, "Business is slow, I swear!"), (1, "Maybe your knees can pay instead."), (0, "No, no, please!"), (2, "Hold him still.")],
             answer: "This ain't your fight. Walk away.", thanks: "I owe you one, friend." },
    Script { title: "Sore Losers", talk: &[(1, "You cheated us at mahjong."), (0, "I won fair and square!"), (2, "Then you'll lose fair and square."), (1, "Let's see those fingers."), (0, "Not the hands, please!")],
             answer: "Another hero, huh? Get him!", thanks: "My hands... still in one piece! Thanks!" },
];
const KREELY_LINES: [&str; 3] = ["Hey. Pick on somebody your own size.", "That's enough. Back off.", "Three on one? Let's even the odds."];
const BUMP_LINES: [&str; 7] = ["Hey! Watch it!", "Watch where you're going!", "Ow! Jerk!", "Excuse YOU!", "Do you mind?!", "Hey, I'm walking here!", "Look out, pal!"];
const SCARED_LINES: [&str; 6] = ["Aaah!", "Run!", "He's crazy!", "Help! Somebody!", "Call the cops!", "Get away from me!"];
const HURT_LINES: [&str; 4] = ["Ow! Stop!", "Please, no!", "What did I do?!", "Leave me alone!"];
const CHAT: [&str; 12] = ["Heard the city's sealed off.", "Nobody's getting out, they say.", "Nice day, at least.", "The noodle stand's open again.",
                          "Did you see the TV last night?", "Keep your head down, that's all.", "My cousin left last week.", "Prices are crazy here.",
                          "Those zako boys are trouble.", "I'm thinking of moving.", "Ha! You're kidding.", "Really? Huh."];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Sub { Waiting, Scene, Fighting, Cleared }
struct Substory { tag: u32, at: Vec3, script: usize, state: Sub, line: usize, wait: f32, victim: Option<u32>, zako: Vec<u32>, since: f32 }

/// A speech bubble over a speaker: its text, letters shown so far, its age after the last letter, the hold, how far it
/// carries, and its UI node once built (with the letters built into it).
struct Bubble { speaker: u32, name: &'static str, text: String, shown: f32, held: f32, hold: f32, far: f32, shout: bool,
                node: Option<Entity>, built: usize, gone: bool }

/// The cut-scene of stepping in: which substory, game frames into it, the anchor (the zako's middle), the stand point.
struct Scene { sub: usize, t: f32, centre: Vec3, stand: Vec3, side: Vec3, line: usize,
                /// the opening wide shot (eye, target): side-on to the victim and the zako, from the open side of the street
                wide: (Vec3, Vec3) }

#[derive(Resource)]
pub struct Town { ready: bool, rng: u32, subs: Vec<Substory>, bubbles: Vec<Bubble>, scene: Option<Scene>, next_tag: u32, population: usize,
                  banner: Option<(String, f32, Color)>, enter: bool, respawn: f32, chat_cool: f32, scripts_used: usize, bars: f32,
                  pairs: Vec<(u32, u32)>, prompt: Option<&'static str> }

pub struct TownPlugin;
impl Plugin for TownPlugin {
    fn build(&self, app: &mut App) {
        // (the actor editor hosts the civ actor on this stage: no crowd, no substories)
        if !on() || super::editor::on() { return; }
        let population = std::env::var("MW_TOWN_CIVS").ok().and_then(|v| v.parse().ok()).unwrap_or(26);
        app.insert_resource(Town { ready: false, rng: 0x5eed_7a11, subs: vec![], bubbles: vec![], scene: None, next_tag: 100, population, banner: None,
                                   enter: false, respawn: 0., chat_cool: 60., scripts_used: 0, bars: 0., pairs: vec![], prompt: None })
            .add_systems(Update, (keys, present.after(super::camera), bubble_letters.after(present)));
    }
}

fn log() -> bool { std::env::var_os("MW_TOWN_LOG").is_some() }

impl Town {
    fn roll(&mut self, n: u32) -> u32 { self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (self.rng >> 8) % n.max(1) }
    fn say(&mut self, speaker: u32, name: &'static str, text: &str, far: f32, shout: bool) {
        // one bubble per speaker: a new line replaces the old
        for b in self.bubbles.iter_mut().filter(|b| b.speaker == speaker) { b.gone = true; }
        let hold = 1.6 + text.len() as f32 * 0.035;
        self.bubbles.push(Bubble { speaker, name, text: text.to_string(), shown: 0., held: 0., hold, far, shout, node: None, built: usize::MAX, gone: false });
    }
    fn talking(&self, speaker: u32) -> bool { self.bubbles.iter().any(|b| b.speaker == speaker && !b.gone) }
}

fn civ_name(body: u8) -> &'static str { match body { 11 | 12 => "TOWNSWOMAN", 10 | 13 => "TOWNSMAN", _ => "ZAKO" } }

/// Update: ENTER for stepping in (read here, used on the next game frame).
fn keys(keys: Res<ButtonInput<KeyCode>>, mut town: ResMut<Town>) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) { town.enter = true; }
}

/// A substory site: a walkable spot 14-45 m from the start, 14 m clear of the others, with room for the three.
fn pick_site(town: &mut Town, nav: &Nav, start: Vec3, taken: &[Vec3]) -> Option<(Vec3, f32)> {
    for _ in 0..400 {
        let p = nav.random(town.roll(1 << 24));
        let d = p.distance(start);
        if !(14. ..45.).contains(&d) || taken.iter().any(|t| t.distance(p) < 14.) { continue; }
        let yaw = town.roll(628) as f32 / 100.;
        let f = Quat::from_rotation_y(yaw) * Vec3::Z;
        let side = Vec3::new(f.z, 0., -f.x);
        let spots = [p, p + f * 1.3 + side * 0.7, p + f * 1.3 - side * 0.7];
        if spots.iter().all(|q| nav.walkable(*q)) && nav.clear(p, spots[1]) && nav.clear(p, spots[2]) { return Some((p, yaw)); }
    }
    None
}

fn add_substory(town: &mut Town, street: &mut Street, nav: &Nav, start: Vec3) {
    let taken: Vec<Vec3> = town.subs.iter().filter(|s| s.state != Sub::Cleared).map(|s| s.at).collect();
    let Some((p, yaw)) = pick_site(town, nav, start, &taken) else { return; };
    let f = Quat::from_rotation_y(yaw) * Vec3::Z;
    let side = Vec3::new(f.z, 0., -f.x);
    let tag = town.next_tag; town.next_tag += 1;
    let script = town.scripts_used % SCRIPTS.len(); town.scripts_used += 1;
    let body = 10 + town.roll(4) as u8;
    // the victim faces the zako; the zako face him
    street.civ_queue.push((p, yaw, body, 0, Role::Victim(yaw), tag));
    for (k, q) in [p + f * 1.3 + side * 0.7, p + f * 1.3 - side * 0.7].into_iter().enumerate() {
        let toward = (p.x - q.x).atan2(p.z - q.z);
        street.civ_queue.push((q, toward, 0, (k + script) % 2, Role::Bully(toward), tag));
    }
    if log() { eprintln!("[town] substory {tag} \"{}\" at {p:.1}", SCRIPTS[script].title); }
    town.subs.push(Substory { tag, at: p, script, state: Sub::Waiting, line: 0, wait: 30., victim: None, zako: vec![], since: 0. });
}

/// FixedUpdate, after the street: the population, the substories, the cut-scene and what the townsfolk say.
pub fn step(mut town: ResMut<Town>, mut street: ResMut<Street>, mut play: ResMut<super::Play>) {
    let pl = Vec3::from_array(play.fight.player.pos);
    if !town.ready {
        let Some(nav) = Nav::load() else { if play.tick % 300 == 1 { eprintln!("[town] no walk grid: run python -I tools/town/stage_nav.py"); } return; };
        let nav = std::sync::Arc::new(nav);
        street.nav = Some(nav.clone());
        town.ready = true;
        // the townsfolk: walkers and idlers about the streets, a few pairs chatting
        let pairs = 4;
        let singles = town.population.saturating_sub(pairs * 2);
        for k in 0..singles {
            let p = loop { let p = nav.random(town.roll(1 << 24)); if p.distance(pl) > 4. { break p; } };
            let role = if k % 5 == 4 { Role::Idler } else { Role::Walker };
            let (yaw, body) = (town.roll(628) as f32 / 100., 10 + town.roll(4) as u8);
            street.civ_queue.push((p, yaw, body, 0, role, 0));
        }
        for _ in 0..pairs {
            for _ in 0..100 {
                let p = nav.random(town.roll(1 << 24));
                let yaw = town.roll(628) as f32 / 100.;
                let q = p + Quat::from_rotation_y(yaw) * Vec3::Z * 1.25;
                if p.distance(pl) < 5. || !nav.walkable(q) || !nav.clear(p, q) { continue; }
                let tag = town.next_tag; town.next_tag += 1;
                let (b1, b2) = (10 + town.roll(4) as u8, 10 + town.roll(4) as u8);
                street.civ_queue.push((p, yaw, b1, 0, Role::Chatter(yaw), tag));
                street.civ_queue.push((q, yaw + std::f32::consts::PI, b2, 0, Role::Chatter(yaw + std::f32::consts::PI), tag));
                town.pairs.push((tag, 0));
                break;
            }
        }
        for _ in 0..3 { add_substory(&mut town, &mut street, &nav, pl); }
        // inspection: MW_TOWN_SUB=n starts the player 3.5 m from substory n (in reach of its prompt)
        if let Some(n) = std::env::var("MW_TOWN_SUB").ok().and_then(|v| v.parse::<usize>().ok()) {
            if let Some(s) = town.subs.get(n) {
                let at = s.at;
                if let Some(p) = (0..32).map(|k| at + Quat::from_rotation_y(k as f32 * 0.4) * Vec3::Z * 3.5).find(|p| nav.walkable(*p)) {
                    play.fight.player.pos = p.to_array(); play.prev_pos = p;
                    play.fight.player.loco.facing = (at.x - p.x).atan2(at.z - p.z); play.cam_yaw = play.fight.player.loco.facing;
                }
            }
        }
        eprintln!("[town] {} townsfolk, {} pairs, {} substories", singles + pairs * 2, town.pairs.len(), town.subs.len());
        return;
    }
    let nav = street.nav.clone().unwrap();
    // inspection: MW_TOWN_LINEUP=clip,clip,... stands civilians in a row 4 m ahead of the player, each looping one clip
    if play.tick == 40 {
        if let Ok(list) = std::env::var("MW_TOWN_LINEUP") {
            let clips: Vec<String> = list.split(',').map(|c| if c.ends_with(".mot") { c.to_string() } else { format!("{c}.mot") }).collect();
            let f = play.fight.player.loco.facing;
            let fwd = Quat::from_rotation_y(f) * Vec3::Z; let side = Vec3::new(fwd.z, 0., -fwd.x);
            let at = pl + fwd * 4.5 - side * (clips.len() as f32 - 1.) * 0.6;
            street.lineup(at, side * 1.2, f + std::f32::consts::PI, &clips);
        }
    }
    // inspection: MW_TOWN_STEPIN=<tick> presses ENTER at that game tick
    if std::env::var("MW_TOWN_STEPIN").ok().and_then(|v| v.parse::<u64>().ok()) == Some(play.tick) { town.enter = true; }
    let tagged = street.tagged();
    // ---- the townsfolk's own lines ----
    for e in std::mem::take(&mut street.civ_events) {
        match e {
            CivEvent::Bumped(id) => {
                let k = town.roll(BUMP_LINES.len() as u32) as usize;
                let body = street.speaker(id).map_or(10, |s| s.2);
                town.say(id, civ_name(body), BUMP_LINES[k], 14., true);
            }
            CivEvent::Scared(id) => {
                let scared_now = town.bubbles.iter().filter(|b| !b.gone && b.shout).count();
                if scared_now < 3 && town.roll(100) < 35 && !town.talking(id) {
                    let k = town.roll(SCARED_LINES.len() as u32) as usize;
                    let body = street.speaker(id).map_or(10, |s| s.2);
                    town.say(id, civ_name(body), SCARED_LINES[k], 18., true);
                }
            }
            CivEvent::Hurt(id) => {
                if town.roll(100) < 50 {
                    let k = town.roll(HURT_LINES.len() as u32) as usize;
                    let body = street.speaker(id).map_or(10, |s| s.2);
                    town.say(id, civ_name(body), HURT_LINES[k], 14., true);
                }
            }
        }
    }
    // small talk between the chatting pairs, when the player passes close
    town.chat_cool -= 1.;
    if town.chat_cool <= 0. {
        town.chat_cool = 50.;
        let pairs = town.pairs.clone();
        for (k, (tag, turn)) in pairs.into_iter().enumerate() {
            let two: Vec<_> = tagged.iter().filter(|t| t.tag == tag && t.alive && t.civ).collect();
            if two.len() < 2 || two[0].pos.distance(pl) > 7. || two.iter().any(|t| town.talking(t.id)) { continue; }
            let who = two[turn as usize % 2];
            let line = CHAT[town.roll(CHAT.len() as u32) as usize];
            let body = street.speaker(who.id).map_or(10, |s| s.2);
            town.say(who.id, civ_name(body), line, 9., false);
            town.pairs[k].1 = turn + 1;
        }
    }
    // keep the streets busy: newcomers walk in out of sight
    let alive = street.civilians().len() - tagged.iter().filter(|t| t.civ && t.alive && t.scripted).count();
    town.respawn -= 1.;
    if alive < town.population.saturating_sub(4) && town.respawn <= 0. {
        town.respawn = 75.;
        for _ in 0..30 {
            let p = nav.random(town.roll(1 << 24));
            if p.distance(pl) < 22. { continue; }
            let (yaw, body) = (town.roll(628) as f32 / 100., 10 + town.roll(4) as u8);
            street.civ_queue.push((p, yaw, body, 0, Role::Walker, 0));
            break;
        }
    }
    // ---- the substories ----
    for i in 0..town.subs.len() {
        let tag = town.subs[i].tag;
        let members: Vec<_> = tagged.iter().filter(|t| t.tag == tag).copied().collect();
        if members.is_empty() { continue; }
        {
            let s = &mut town.subs[i];
            s.victim = members.iter().find(|t| t.civ).map(|t| t.id);
            s.zako = members.iter().filter(|t| !t.civ).map(|t| t.id).collect();
            s.since += 1.;
        }
        let state = town.subs[i].state;
        let script = &SCRIPTS[town.subs[i].script];
        let zako_alive = members.iter().filter(|t| !t.civ && t.alive).count();
        let near = town.subs[i].at.distance(pl);
        match state {
            Sub::Waiting => {
                // struck first: the fight is on without the scene
                if members.iter().any(|t| t.fighting) {
                    street.provoke_tag(tag, pl);
                    town.subs[i].state = Sub::Fighting;
                    if log() { eprintln!("[town] substory {tag}: the zako were struck first"); }
                    continue;
                }
                // the exchange plays while the player is near; the bubbles go when he walks off
                if near < TALK_NEAR {
                    town.subs[i].wait -= 1.;
                    let busy = members.iter().any(|t| town.talking(t.id));
                    if town.subs[i].wait <= 0. && !busy {
                        let (who, line) = script.talk[town.subs[i].line % script.talk.len()];
                        let id = if who == 0 { town.subs[i].victim } else { town.subs[i].zako.get(who as usize - 1).copied() };
                        if let Some(id) = id.filter(|id| members.iter().any(|t| t.id == *id && t.alive)) {
                            town.say(id, if who == 0 { civ_name(street.speaker(id).map_or(10, |s| s.2)) } else { "ZAKO" }, line, TALK_GONE, false);
                        }
                        town.subs[i].line += 1; town.subs[i].wait = 14.;
                    }
                } else if near > TALK_GONE {
                    for b in town.bubbles.iter_mut().filter(|b| members.iter().any(|t| t.id == b.speaker)) { b.gone = true; }
                    town.subs[i].wait = 20.;
                }
                // ENTER steps in
                if near < PROMPT_NEAR && town.scene.is_none() && play.fight.player.hp > 0 && zako_alive > 0 && town.enter {
                    town.enter = false;
                    start_scene(&mut town, &mut street, &mut play, &nav, i, &members);
                }
            }
            Sub::Scene => {}
            Sub::Fighting => {
                if zako_alive == 0 {
                    town.subs[i].state = Sub::Cleared; town.subs[i].since = 0.;
                    let title = script.title;
                    town.banner = Some((format!("SUBSTORY CLEAR"), 0., Color::srgb(0.95, 0.85, 0.2)));
                    if let Some(v) = town.subs[i].victim.filter(|v| members.iter().any(|t| t.id == *v && t.alive)) {
                        let body = street.speaker(v).map_or(10, |s| s.2);
                        town.say(v, civ_name(body), script.thanks, 20., false);
                    }
                    if log() { eprintln!("[town] substory {tag} \"{title}\" cleared"); }
                }
            }
            Sub::Cleared => {
                // a new one elsewhere, a while later
                if town.subs[i].since == 900. { add_substory(&mut town, &mut street, &nav, pl); }
            }
        }
    }
    town.enter = false;
    run_scene(&mut town, &mut street, &mut play);
    // bubbles age on the game clock
    for b in town.bubbles.iter_mut() {
        if b.gone { continue; }
        let n = b.text.chars().count() as f32;
        if b.shown < n { b.shown = (b.shown + 1.5).min(n); } else { b.held += 1. / 30.; if b.held > b.hold { b.gone = true; } }
    }
    if let Some((_, t, _)) = town.banner.as_mut() { *t += 1. / 30.; }
    if town.banner.as_ref().is_some_and(|b| b.1 > 2.6) { town.banner = None; }
    // the prompt
    town.prompt = None;
    if town.scene.is_none() && play.fight.player.hp > 0 {
        if let Some(s) = town.subs.iter().find(|s| s.state == Sub::Waiting && s.at.distance(pl) < PROMPT_NEAR) { town.prompt = Some(SCRIPTS[s.script].title); }
    }
}

fn start_scene(town: &mut Town, street: &mut Street, play: &mut super::Play, nav: &Nav, i: usize, members: &[super::street::Tagged]) {
    let zako: Vec<Vec3> = members.iter().filter(|t| !t.civ && t.alive).map(|t| t.pos).collect();
    let centre = zako.iter().copied().sum::<Vec3>() / zako.len().max(1) as f32;
    let pl = Vec3::from_array(play.fight.player.pos);
    // he stands 3.4 m from the zako, on his side of them (or the nearest walkable spot round them)
    let dir = Vec3::new(pl.x - centre.x, 0., pl.z - centre.z).normalize_or(Vec3::Z);
    let stand = (0..16).map(|k| centre + Quat::from_rotation_y((k as f32 * 0.4) * if k % 2 == 0 { 1. } else { -1. } * 0.5) * dir * 3.4)
        .find(|p| nav.walkable(*p) && nav.clear(*p, centre)).unwrap_or(pl);
    let stand = Vec3::new(stand.x, nav.cell_of(stand).map_or(pl.y, |c| nav.at(c).y), stand.z);
    play.fight.player.pos = stand.to_array(); play.prev_pos = stand;
    let face = (centre.x - stand.x).atan2(centre.z - stand.z);
    play.fight.player.loco.facing = face; play.cam_yaw = face;
    let fwd = Vec3::new(centre.x - stand.x, 0., centre.z - stand.z).normalize_or(Vec3::Z);
    let side = Vec3::new(fwd.z, 0., -fwd.x);
    for b in town.bubbles.iter_mut() { b.gone = true; }
    town.subs[i].state = Sub::Scene;
    LOCK.store(true, Ordering::Relaxed);
    if log() { eprintln!("[town] substory {}: stepping in (stand {stand:.1})", town.subs[i].tag); }
    let _ = street;
    // the opening shot: side-on to the line from the zako to the victim, from whichever side has the open street
    let victim = members.iter().find(|t| t.civ).map_or(centre, |t| t.pos);
    let mid = (victim + centre) * 0.5;
    let axis = Vec3::new(victim.x - centre.x, 0., victim.z - centre.z).normalize_or(fwd);
    let perp = Vec3::new(axis.z, 0., -axis.x);
    let open = |d: Vec3| (1..=12).take_while(|k| nav.walkable(mid + d * (*k as f32 * 0.5))).count();
    let away = if open(perp) >= open(-perp) { perp } else { -perp };
    let dist = (open(away) as f32 * 0.5).clamp(3.2, 5.5);
    let wide = (mid + away * dist + Vec3::Y * 1.6, mid + Vec3::Y * 1.0);
    town.scene = Some(Scene { sub: i, t: 0., centre, stand, side, line: 0, wide });
}

/// The cut-scene, shot by shot (game frames): the zako at it (0-55), him over the shoulder with his line and taunt
/// (55-125), the zako turning on him with theirs (125-190), the victim running and FIGHT! as play resumes (190-230).
fn run_scene(town: &mut Town, street: &mut Street, play: &mut super::Play) {
    let Some(sc) = town.scene.as_mut() else { town.bars = (town.bars - 0.08).max(0.); return; };
    sc.t += 1.;
    let (t, i, c, stand, side, wide) = (sc.t, sc.sub, sc.centre, sc.stand, sc.side, sc.wide);
    let tag = town.subs[i].tag;
    let script = &SCRIPTS[town.subs[i].script];
    let fwd = Vec3::new(c.x - stand.x, 0., c.z - stand.z).normalize_or(Vec3::Z);
    let up = Vec3::Y;
    town.bars = (town.bars + 0.08).min(1.);
    let cam = if t < 55. {
        // the scene as it is: low and wide from the side
        let k = t / 55.;
        (wide.0 + up * 0.2 * k, wide.1, 50f32.to_radians())
    } else if t < 125. {
        // over his shoulder
        (stand - fwd * 1.7 + side * -0.7 + up * 1.75, c + up * 1.25, 38f32.to_radians())
    } else if t < 190. {
        // on the zako, turning to him
        // (from his side of them, a little off his line, so both faces turn to the lens)
        (c - fwd * 3.0 + side * 1.6 + up * 1.7, c + up * 1.35, 46f32.to_radians())
    } else {
        // wide behind him as they come
        let k = ((t - 190.) / 40.).min(1.);
        (stand - fwd * (2.6 + k) + side * 0.9 + up * 2.0, c + up * 1.0, 50f32.to_radians())
    };
    *CAMERA.lock().unwrap() = Some(cam);
    let zako = town.subs[i].zako.clone();
    let victim = town.subs[i].victim;
    let line = sc.line;
    if t >= 8. && line == 0 {
        if let (Some(z), Some((_, true, _))) = (zako.first().copied(), zako.first().and_then(|z| street.speaker(*z))) { town.say(z, "ZAKO", script.talk[0].1, 99., false); }
        town.scene.as_mut().unwrap().line = 1;
    } else if t >= 62. && line == 1 {
        let k = town.roll(KREELY_LINES.len() as u32) as usize;
        town.say(PLAYER, "KREELY", KREELY_LINES[k], 99., false);
        TAUNT.store(true, Ordering::Relaxed);
        town.scene.as_mut().unwrap().line = 2;
    } else if t >= 128. && line == 2 {
        street.face_tag(tag, stand);
        if let Some(z) = zako.get(1).or(zako.first()).copied() { town.say(z, "ZAKO", script.answer, 99., true); }
        town.scene.as_mut().unwrap().line = 3;
    } else if t >= 190. && line == 3 {
        street.provoke_tag(tag, stand);
        if let Some(v) = victim { let body = street.speaker(v).map_or(10, |s| s.2); town.say(v, civ_name(body), "Eek!", 30., true); }
        town.banner = Some(("FIGHT!".into(), 0., Color::srgb(0.9, 0.06, 0.04)));
        town.scene.as_mut().unwrap().line = 4;
    }
    if t >= 230. {
        town.subs[i].state = Sub::Fighting;
        town.scene = None;
        LOCK.store(false, Ordering::Relaxed);
        *CAMERA.lock().unwrap() = None;
        play.cam_yaw = play.fight.player.loco.facing;
        if log() { eprintln!("[town] substory {tag}: fight"); }
    }
}

// ---- the speech bubbles, the prompt, the bars and the banner ----

#[derive(Component)]
struct TownUi;
#[derive(Component)]
struct BubbleText;

const LINE: Color = Color::srgb(0.96, 0.95, 0.98);
const TITLE: Color = Color::srgb(0.42, 0.40, 0.52);
const PANEL: Color = Color::srgba(0.06, 0.05, 0.12, 0.86);
const SHOUT: Color = Color::srgba(0.32, 0.03, 0.04, 0.9);
const TEXT: f32 = 11.;
const WRAP: f32 = 168.;

/// Split `text` into lines no wider than WRAP (message font units).
fn wrap(fonts: &super::editor::Fonts, text: &str) -> Vec<String> {
    let mut lines = vec![]; let mut cur = String::new();
    for w in text.split(' ') {
        let try_ = if cur.is_empty() { w.to_string() } else { format!("{cur} {w}") };
        if fonts.layout(&try_, TEXT).1 > WRAP && !cur.is_empty() { lines.push(std::mem::take(&mut cur)); cur = w.to_string(); } else { cur = try_; }
    }
    if !cur.is_empty() { lines.push(cur); }
    lines
}

/// Update, after the camera: place every bubble over its speaker's head, type its letters, fade it; the prompt, the
/// letterbox and the banner.
fn present(mut town: ResMut<Town>, street: Res<Street>, play: Res<super::Play>, pause: Res<super::pause::Pause>, windows: Query<&Window>,
           cams: Query<(&Camera, &Transform), (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>, assets: Res<AssetServer>,
           mut nodes: Query<&mut Node>, mut fonts: Local<Option<Option<super::editor::Fonts>>>, mut last: Local<String>, mut commands: Commands) {
    if fonts.is_none() { *fonts = Some(super::editor::load_fonts()); }
    let Some(Some(fonts)) = fonts.as_ref() else { return; };
    let Ok(window) = windows.single() else { return; };
    let Ok((camera, cam_t)) = cams.single() else { return; };
    // (this frame's camera placement: play.rs camera set its Transform; the global one lags a frame)
    let cam_t = GlobalTransform::from(*cam_t);
    let viewport = Vec2::new(window.width(), window.height());
    let k = (viewport.x / 640.).min(viewport.y / 360.);
    let offset = (viewport - Vec2::new(640., 360.) * k) * 0.5;
    let u = |v: f32| px(v * k);
    let pl = Vec3::from_array(play.fight.player.pos);
    let town = &mut *town;
    // drop the bubbles that are done (their nodes with them)
    town.bubbles.retain(|b| {
        let dead = b.speaker != PLAYER && street.speaker(b.speaker).is_none_or(|s| !s.1);
        let keep = !b.gone && !dead;
        if !keep { if let Some(n) = b.node { commands.entity(n).despawn(); } }
        keep
    });
    for b in town.bubbles.iter_mut() {
        let head = if b.speaker == PLAYER { Some(pl + Vec3::Y * 2.0) } else { street.speaker(b.speaker).map(|s| s.0) };
        let Some(head) = head else { continue; };
        let far = head.distance(pl) > b.far && !locked();
        // through NDC onto the window (the 3D camera may draw into a target of another size)
        let screen = camera.world_to_ndc(&cam_t, head + Vec3::Y * 0.25).filter(|n| n.z > 0. && n.x.abs() < 1.3 && n.y.abs() < 1.3)
            .map(|n| Vec2::new((n.x + 1.) * 0.5 * viewport.x, (1. - n.y) * 0.5 * viewport.y));
        let behind = (head - cam_t.translation()).dot(*cam_t.forward()) < 0.;
        let shown = !far && !behind && !pause.open && screen.is_some();
        let lines = wrap(fonts, &b.text);
        let width = lines.iter().map(|l| fonts.layout(l, TEXT).1).fold(0., f32::max).max(fonts.layout(b.name, 8.).1 + 10.);
        let (w, h) = (width + 14., lines.len() as f32 * (TEXT + 2.) + 8.);
        if b.node.is_none() {
            // the box (dark panel, white rules), the name bar above it, the pointer under it; the letters go in `BubbleText`
            let root = commands.spawn((TownUi, Node { position_type: PositionType::Absolute, width: u(w), height: u(h + 11.), ..default() }, GlobalZIndex(8800), Visibility::Hidden)).id();
            let panel = commands.spawn((Node { position_type: PositionType::Absolute, left: u(0.), top: u(9.), width: u(w), height: u(h), border: UiRect::all(u(1.2)), ..default() },
                                        BackgroundColor(if b.shout { SHOUT } else { PANEL }), BorderColor::all(LINE))).id();
            let tail = commands.spawn((Node { position_type: PositionType::Absolute, left: u(w * 0.5 - 4.5), top: u(h + 9. - 5.), width: u(9.), height: u(9.),
                                              border: UiRect { right: u(1.2), bottom: u(1.2), ..default() }, ..default() },
                                       BackgroundColor(if b.shout { SHOUT } else { PANEL }), BorderColor::all(LINE), UiTransform::from_rotation(Rot2::degrees(45.)))).id();
            let nw = fonts.layout(b.name, 8.).1 + 10.;
            let bar = commands.spawn((Node { position_type: PositionType::Absolute, left: u(5.), top: u(0.), width: u(nw), height: u(11.), ..default() }, BackgroundColor(LINE))).id();
            let mut name_glyphs = vec![];
            for g in fonts.layout(b.name, 8.).0 {
                let mut img = ImageNode::new(assets.load(g.image.clone())).with_color(TITLE);
                img.rect = Some(g.rect);
                name_glyphs.push(commands.spawn((img, Node { position_type: PositionType::Absolute, left: u(5. + g.x), top: u(1.5), width: u(g.w), height: u(g.h), ..default() })).id());
            }
            commands.entity(bar).add_children(&name_glyphs);
            let text = commands.spawn((BubbleText, Node { position_type: PositionType::Absolute, left: u(7.), top: u(5.), width: u(w - 14.), height: u(h - 8.), ..default() })).id();
            commands.entity(panel).add_child(text);
            commands.entity(root).add_children(&[tail, panel, bar]);
            b.node = Some(root);
        }
        let Some(root) = b.node else { continue; };
        let pos = screen.unwrap_or_default();
        let (bw, bh) = ((w) * k, (h + 11.) * k);
        let left = (pos.x - bw * 0.5).clamp(4., viewport.x - bw - 4.);
        let top = (pos.y - bh - 4. * k).clamp(4., viewport.y - bh - 4.);
        if let Ok(mut n) = nodes.get_mut(root) { n.left = px(left); n.top = px(top); }
        commands.entity(root).insert(if shown { Visibility::Visible } else { Visibility::Hidden });
        // the letters typed so far (rebuilt when another shows)
        let count = b.shown as usize;
        if count != b.built {
            b.built = count;
            commands.entity(root).insert(BubbleRebuild { lines: lines.clone(), count });
        }
    }
    // the prompt, the letterbox, the banner: rebuilt when they change
    let key = format!("{:?}|{:.2}|{:?}|{}x{}", town.prompt, town.bars, town.banner.as_ref().map(|b| (b.0.clone(), (b.1 * 20.) as i32)), viewport.x, viewport.y);
    if *last != key {
        *last = key;
        commands.run_system_cached_with(rebuild_fixed, (town.prompt.map(|s| s.to_string()), town.bars, town.banner.clone(), viewport, k, offset));
    }
}

/// Letters to (re)build in a bubble: its wrapped lines and how many letters show.
#[derive(Component)]
struct BubbleRebuild { lines: Vec<String>, count: usize }

/// The overlay's fixed parts (prompt, bars, banner), tagged so they can be cleared together.
#[derive(Component)]
struct FixedUi;

fn rebuild_fixed(In((prompt, bars, banner, viewport, k, offset)): In<(Option<String>, f32, Option<(String, f32, Color)>, Vec2, f32, Vec2)>,
                 old: Query<Entity, With<FixedUi>>, assets: Res<AssetServer>, mut fonts: Local<Option<Option<super::editor::Fonts>>>, mut commands: Commands) {
    for e in &old { commands.entity(e).despawn(); }
    if fonts.is_none() { *fonts = Some(super::editor::load_fonts()); }
    let Some(Some(fonts)) = fonts.as_ref() else { return; };
    let u = |v: f32| px(v * k);
    let at = |x: f32, y: f32| (px(offset.x + x * k), px(offset.y + y * k));
    // the letterbox
    if bars > 0.01 {
        let h = viewport.y * 0.11 * bars;
        commands.spawn((FixedUi, Node { position_type: PositionType::Absolute, left: px(0.), top: px(0.), width: px(viewport.x), height: px(h), ..default() }, BackgroundColor(Color::BLACK), GlobalZIndex(8700)));
        commands.spawn((FixedUi, Node { position_type: PositionType::Absolute, left: px(0.), top: px(viewport.y - h), width: px(viewport.x), height: px(h), ..default() }, BackgroundColor(Color::BLACK), GlobalZIndex(8700)));
    }
    // the prompt: the substory's title over a ruled key chip and "Step in"
    if let Some(title) = prompt {
        let tw = fonts.layout(&title, 10.).1;
        let chip = fonts.layout("ENTER", 9.).1 + 8.;
        let what = fonts.layout("Step in", 10.).1;
        let w = (chip + 8. + what).max(tw + 20.) + 16.;
        let (x, y) = (320. - w * 0.5, 292.);
        let (l, t) = at(x, y);
        let panel = commands.spawn((FixedUi, Node { position_type: PositionType::Absolute, left: l, top: t, width: u(w), height: u(34.), border: UiRect::all(u(1.2)), ..default() },
                                    BackgroundColor(PANEL), BorderColor::all(LINE), GlobalZIndex(8900))).id();
        let bar = commands.spawn((Node { position_type: PositionType::Absolute, left: u(0.), top: u(0.), width: u(w - 2.4), height: u(12.), ..default() }, BackgroundColor(LINE))).id();
        let mut kids = vec![bar];
        let mut glyphs = |s: &str, size: f32, x0: f32, y0: f32, color: Color, kids: &mut Vec<Entity>| {
            for g in fonts.layout(s, size).0 {
                let mut img = ImageNode::new(assets.load(g.image.clone())).with_color(color);
                img.rect = Some(g.rect);
                kids.push(commands.spawn((img, Node { position_type: PositionType::Absolute, left: u(x0 + g.x), top: u(y0), width: u(g.w), height: u(g.h), ..default() })).id());
            }
        };
        glyphs(&title.to_uppercase(), 10., (w - fonts.layout(&title.to_uppercase(), 10.).1) * 0.5, 0.5, TITLE, &mut kids);
        let cx = (w - (chip + 8. + what)) * 0.5;
        glyphs("ENTER", 9., cx + 4., 17.5, LINE, &mut kids);
        glyphs("Step in", 10., cx + chip + 8., 17., LINE, &mut kids);
        let chip_box = commands.spawn((Node { position_type: PositionType::Absolute, left: u(cx), top: u(16.), width: u(chip), height: u(13.), border: UiRect::all(u(0.8)), ..default() }, BorderColor::all(LINE))).id();
        kids.push(chip_box);
        commands.entity(panel).add_children(&kids);
    }
    // the banner: brush capitals across the middle, fading out
    if let Some((text, t, color)) = banner {
        let a = if t < 0.15 { t / 0.15 } else if t > 2.0 { (2.6 - t) / 0.6 } else { 1. }.clamp(0., 1.);
        let hgt = if text.len() < 8 { 54. } else { 30. };
        let (glyphs, w) = fonts.brush_layout(&text, hgt);
        let x0 = 320. - w * 0.5; let y0 = 150. - hgt * 0.5;
        let c = color.with_alpha(a);
        for (r, x, gw) in glyphs {
            let mut img = ImageNode::new(assets.load(fonts.brush_image().to_string())).with_color(c);
            img.rect = Some(r);
            let (l, t) = at(x0 + x, y0);
            commands.spawn((FixedUi, img, Node { position_type: PositionType::Absolute, left: l, top: t, width: u(gw), height: u(hgt), ..default() }, GlobalZIndex(8950)));
        }
    }
}

/// Update: rebuild the letters of the bubbles that asked for it.
fn bubble_letters(q: Query<(Entity, &BubbleRebuild, &Children)>, panels: Query<&Children>, texts: Query<(Entity, Option<&Children>), With<BubbleText>>,
                      windows: Query<&Window>, assets: Res<AssetServer>, mut fonts: Local<Option<Option<super::editor::Fonts>>>, mut commands: Commands) {
    if q.is_empty() { return; }
    if fonts.is_none() { *fonts = Some(super::editor::load_fonts()); }
    let Some(Some(fonts)) = fonts.as_ref() else { return; };
    let Ok(window) = windows.single() else { return; };
    let k = (window.width() / 640.).min(window.height() / 360.);
    let u = |v: f32| px(v * k);
    for (root, req, kids) in &q {
        commands.entity(root).remove::<BubbleRebuild>();
        // the text holder: the panel's child
        let holder = kids.iter().filter_map(|c| panels.get(c).ok()).flat_map(|cs| cs.iter()).find(|c| texts.get(*c).is_ok());
        let Some(holder) = holder else { continue; };
        if let Ok((_, Some(old))) = texts.get(holder) { for c in old.iter() { commands.entity(c).despawn(); } }
        let mut left = req.count;
        let mut glyphs = vec![];
        for (row, line) in req.lines.iter().enumerate() {
            if left == 0 { break; }
            let part: String = line.chars().take(left).collect();
            left = left.saturating_sub(line.chars().count() + 1);
            for g in fonts.layout(&part, TEXT).0 {
                let mut img = ImageNode::new(assets.load(g.image.clone())).with_color(LINE);
                img.rect = Some(g.rect);
                glyphs.push(commands.spawn((img, Node { position_type: PositionType::Absolute, left: u(g.x), top: u(row as f32 * (TEXT + 2.)), width: u(g.w), height: u(g.h), ..default() })).id());
            }
        }
        commands.entity(holder).add_children(&glyphs);
    }
}
