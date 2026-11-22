//! Kreese Kreely's voice (assets/native/audio/kreely, the user's selected contextual-v4 bank: kreely.json) [added]:
//! when he plays, he talks. One line at a time on a stream of his own (it never waits behind the commentary), each set
//! with its own odds and rest time so he does not narrate every punch:
//! - hurt: a blow lands on him (55 %, then 6 s quiet);
//! - teeth: a heavy blow (25+ damage) lands on him (35 %, 25 s);
//! - near_death: his health falls under a quarter (each time it crosses, 20 s);
//! - knockdown: he puts RinRin, Elise or a street grunt on the floor (60 %, 7 s);
//! - defeat: RinRin or a boss falls for good (always), a street kill now and then (35 %, 10 s).
//! `MW_KREELY_VOICE=0` keeps him quiet; `MW_KREELY_VOICE_LOG=1` logs each line.
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(serde::Deserialize, Clone)]
struct Line { file: String }

#[derive(Resource, Default)]
pub struct KreelyVoice {
    sets: HashMap<String, Vec<Line>>, last: HashMap<String, usize>,
    /// game ticks until each set may speak again; the tick the current line ends (none plays before it)
    rest: HashMap<String, u64>, busy_until: u64,
    hp: Option<i32>, low: bool, rin_down: bool, elise_down: bool, knocks: u32, kills: u32, won: bool, rng: u32,
}

#[derive(Component)]
struct KreelyLine;

pub struct KreelyVoicePlugin;
impl Plugin for KreelyVoicePlugin {
    fn build(&self, app: &mut App) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/audio/kreely/kreely.json");
        let sets: HashMap<String, Vec<Line>> = std::fs::read_to_string(root).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0x5eed_1234, |d| d.subsec_nanos() | 1);
        app.insert_resource(KreelyVoice { sets, rng: seed, ..default() });
    }
}

impl KreelyVoice {
    fn roll(&mut self, n: u32) -> u32 { self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (self.rng >> 8) % n.max(1) }
    /// Say a line of `set` (`chance` in percent, then `rest` seconds before the set speaks again).
    fn say(&mut self, set: &str, chance: u32, rest: f32, tick: u64, sound: &super::audio::PortAudio, assets: &AssetServer, commands: &mut Commands) {
        if tick < self.busy_until || self.rest.get(set).is_some_and(|&t| tick < t) || self.roll(100) >= chance { return; }
        let Some(lines) = self.sets.get(set).cloned() else { return; };
        if lines.is_empty() { return; }
        // never the same line twice running
        let mut k = self.roll(lines.len() as u32) as usize;
        if lines.len() > 1 && self.last.get(set) == Some(&k) { k = (k + 1) % lines.len(); }
        self.last.insert(set.into(), k);
        self.rest.insert(set.into(), tick + (rest * 30.) as u64);
        // (a line runs up to ~5 s; nothing else of his until it is done)
        self.busy_until = tick + 90;
        if std::env::var_os("MW_KREELY_VOICE_LOG").is_some() { eprintln!("[kreely-voice] tick {tick} {set}: {}", lines[k].file); }
        let base = 1.6;
        commands.spawn((KreelyLine, super::audio::Gain { bus: super::audio::Bus::Announcers, base }, AudioPlayer::new(assets.load::<AudioSource>(lines[k].file.clone())),
                        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(sound.gain(super::audio::Bus::Announcers, base)))));
    }
}

/// FixedUpdate, after the fight and the stage modules have stepped.
pub fn step(mut v: ResMut<KreelyVoice>, play: Res<super::Play>, street: Res<super::street::Street>, elise: Res<super::elise::Elise>,
            baron: Res<super::baron::Baron>, sound: Res<super::audio::PortAudio>, assets: Res<AssetServer>, mut commands: Commands) {
    if !play.as_kreely || v.sets.is_empty() || std::env::var("MW_KREELY_VOICE").as_deref() == Ok("0") { return; }
    let tick = play.tick;
    let hp = play.fight.player.hp as i32;
    // his health: a blow (a heavy one may cost him teeth), and crossing under a quarter
    if let Some(prev) = v.hp {
        let lost = prev - hp;
        if lost > 0 && hp > 0 {
            let low = hp * 4 < mw_sim::fight::PLAYER_MAX_HP as i32;
            if low && !v.low { v.low = true; v.say("near_death", 100, 20., tick, &sound, &assets, &mut commands); }
            else if lost >= 25 { v.say("teeth", 35, 25., tick, &sound, &assets, &mut commands); v.say("hurt", 55, 6., tick, &sound, &assets, &mut commands); }
            else { v.say("hurt", 55, 6., tick, &sound, &assets, &mut commands); }
        }
        if hp * 4 >= mw_sim::fight::PLAYER_MAX_HP as i32 { v.low = false; }
    }
    v.hp = Some(hp);
    // knockdowns he caused
    let rin_down = play.fight.rin.down;
    let elise_down = elise.on && elise.st == super::elise::St::Down;
    let knocks = street.knocks;
    if (rin_down && !v.rin_down) || (elise_down && !v.elise_down) || knocks > v.knocks { v.say("knockdown", 60, 7., tick, &sound, &assets, &mut commands); }
    v.rin_down = rin_down; v.elise_down = elise_down; v.knocks = knocks;
    // wins: RinRin or a boss down for good, and now and then a street kill
    let won = play.fight.victory || elise.cleared() || baron.cleared();
    if won && !v.won { v.busy_until = 0; v.say("defeat", 100, 10., tick, &sound, &assets, &mut commands); }
    v.won = won;
    if street.kills > v.kills && !won { v.say("defeat", 35, 10., tick, &sound, &assets, &mut commands); }
    v.kills = street.kills;
}
