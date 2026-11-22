//! The last-life execution [added feature, not native]: when the player dies with no continues left, the boss finishes
//! him off in a short cinematic before the game-over static (cIdGameOverEnd, play_continue.rs).
//! - RinRin walks up to the fallen player and plays her clash pair through to her own win with no prompt: 0110 / 04aa,
//!   the 0111 / 04ab hold, then 0113 / 04ad, whose second fan slash takes his head off (mw_sim Fight::start_execution).
//! - Elise and the Baron / Mathilda play their own clash or pin through to the loss the same way (a stand-in until
//!   each has its own finisher): play_elise.rs / play_baron.rs `start_execution`.
//! The pair's own camera MOTs (pl0030_c4aa..c4ad, c46x, c48x) frame it as in the boss defeats; letterbox bars close
//! in, the cut slows the clock, and the death orbit (play.rs defeat) follows over the body.
use bevy::prelude::*;

/// Game ticks he lies still (his fall played out: death state 95 phase 2) before the boss walks up to finish him.
const LIE_WAIT: u64 = 75;
/// After the finisher: one full turn of the camera round his body (real seconds), then it waits this long (or for
/// Enter) before the fade to black and the continue window.
const ORBIT_SECS: f64 = 6.;
const HOLD_SECS: f64 = 10.;
/// Real seconds the clock runs slow after the killing blow, and how slow.
const CUT_SLOW_SECS: f64 = 1.4;
const CUT_SLOW: f32 = 0.3;
/// Letterbox bar height (fraction of the screen, each) and how fast they close in / open (per real second).
const BARS: f32 = 0.11;
const BARS_RATE: f32 = 1.6;

#[derive(Resource, Default)]
pub struct Execution {
    /// Started for this death (once per death), and finished (the game-over window may open).
    pub started: bool, pub done: bool,
    /// The player's head is off (play.rs pose scales the head joint away).
    pub head_off: bool,
    /// Real time of the killing blow (the slow clock), the bars' current height (0..1).
    cut_at: Option<f64>, bars: f32,
    /// Real times it started and finished (the fades, as the event intros': in from black, out to black).
    started_at: Option<f64>, done_at: Option<f64>,
    /// Ticks of neck spurts left after the cut.
    spurts: u32,
    /// The tick his fall ended (he lies still); the real time the finisher ended (the turn round his body starts) and
    /// whether the fade has finished (the continue window may open).
    lie_since: Option<u64>, ended_at: Option<f64>, faded: bool,
    /// He is down on his last life and the boss is about to come (the wait before the walk-up).
    pending: bool,
}

impl Execution {
    /// The execution holds the continue window back until it is done.
    pub fn holds_window(&self) -> bool { (self.started && !self.faded) || self.pending }
    /// The camera's turn round his body after the finisher (radians, 0 .. 2 pi), and the real time it began.
    pub fn orbit(&self, now: f64) -> Option<(f32, f64)> {
        let t0 = self.ended_at?; Some(((((now - t0) / ORBIT_SECS).min(1.)) as f32 * std::f32::consts::TAU, t0))
    }
    /// The HUD is hidden while it plays and until the game-over window (as in the event intros).
    pub fn hides_hud(&self) -> bool { self.started && self.started_at.is_some() }
}

#[derive(Component)]
pub struct BarNode;
/// Fade in from black at the start, and out to black once it is done (beneath the game-over window).
const FADE_IN: f64 = 0.45;
const FADE_OUT: f64 = 0.8;

/// FixedUpdate, after the bosses: starts the execution once the player is down on his last life, and follows it.
pub fn step(mut ex: ResMut<Execution>, mut play: ResMut<super::Play>, mut elise: ResMut<super::elise::Elise>, mut baron: ResMut<super::baron::Baron>,
            mut fx: ResMut<super::fx::Fx>, real: Res<Time<Real>>) {
    let pl = &play.fight.player;
    let down = pl.state() == 95;
    let executing = play.fight.executing() || elise.executing() || baron.executing();
    // alive again (a continue or the game-over restart): reset for the next death
    if !down && !executing && pl.state() != mw_sim::fight::STATE_EXTERNAL {
        if ex.started || ex.head_off { *ex = Execution { bars: ex.bars, ..default() }; }
        return;
    }
    let off = std::env::var("MW_EXECUTION").as_deref() == Ok("0");
    if std::env::var_os("MW_EXEC_LOG").is_some() && play.tick % 15 == 0 {
        eprintln!("[exec] tick {} down {down} phase {} continues {} started {} done {} rin {} {} executing {executing} head {} at {:?}", play.tick, pl.loco.phase, play.continues, ex.started, ex.done, play.fight.rin.state, play.fight.rin.phase, ex.head_off, neck(&play));
    }
    // the boss waits for his fall to play out (and the slow motion with it), then a little longer, before walking up
    if down && pl.loco.phase == 2 { ex.lie_since.get_or_insert(play.tick); }
    ex.pending = down && !ex.started && !off && play.continues == 0;
    if down && !ex.started && !off && play.continues == 0 && ex.lie_since.is_some_and(|t| play.tick >= t + LIE_WAIT) {
        let started = if super::elise::on() { elise.start_execution(&mut play) }
            else if super::baron::on() { baron.start_execution(&mut play) }
            else { play.fight.start_execution(); play.fight.executing() };
        ex.started = true; ex.done = !started;
    }
    if !ex.started || ex.done { return; }
    for e in &play.fight.events {
        match e {
            mw_sim::fight::FightEvent::ExecutionCut => {
                ex.head_off = true; ex.cut_at = Some(real.elapsed_secs_f64()); ex.spurts = 40;
                let at = neck(&play);
                fx.gore(at, play.fight.rin.facing, 60, true, true);
                fx.burst("core", 486, at, play.fight.rin.facing, 1.2);
            }
            mw_sim::fight::FightEvent::ExecutionDone => ex.done = true,
            _ => {}
        }
    }
    // the boss modules end theirs by leaving the player dead again
    let executing = play.fight.executing() || elise.executing() || baron.executing();
    if !ex.done && play.fight.player.state() == 95 && !executing && (super::elise::on() || super::baron::on()) {
        // (the stand-ins' fall plays slow, as RinRin's cut)
        ex.done = true; ex.cut_at = Some(real.elapsed_secs_f64());
    }
    if ex.spurts > 0 {
        ex.spurts -= 1;
        if ex.spurts % 6 == 0 { let at = neck(&play); fx.gore(at, play.fight.player.loco.facing, 12, false, false); }
    }
}

/// Where his neck is: the root as it travels (paired clips keep their root) at shoulder height, lower as he falls.
fn neck(play: &super::Play) -> Vec3 {
    let p = &play.fight.player;
    let clip = &play.fd.jack.clips[p.motion.clip];
    let poses = mw_sim::character::sample_retargeted(&play.fd.jack.bones, clip, p.motion.frame.min((clip.frames.max(1) - 1) as f32), mw_sim::character::Retarget::NATIVE);
    let frames = mw_sim::fight::bone_frames(&play.fd.jack.bones, &poses);
    let local = play.fd.jack.bones.iter().position(|b| b.id == 3).map(|i| Vec3::from_array(frames[i].1)).unwrap_or(Vec3::Y * 1.5);
    Vec3::from_array(p.pos) + Quat::from_rotation_y(p.loco.facing) * local
}

/// Update: the killing blow's slow clock, and the letterbox bars while the execution runs.
pub fn present(mut ex: ResMut<Execution>, real: Res<Time<Real>>, mut time: ResMut<Time<Virtual>>, windows: Query<&Window>,
               nodes: Query<Entity, With<BarNode>>, keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    let now = real.elapsed_secs_f64();
    if let Some(t) = ex.cut_at {
        let u = now - t;
        if u < CUT_SLOW_SECS { time.set_relative_speed(CUT_SLOW); }
        else { time.set_relative_speed(1.); ex.cut_at = None; }
    }
    if ex.started && ex.started_at.is_none() { ex.started_at = Some(now); }
    // the finisher over: the camera turns once round his body, holds (10 s, or until Enter), then the fade to black
    if ex.done && ex.started_at.is_some() && ex.ended_at.is_none() { ex.ended_at = Some(now); }
    if let (Some(t0), None) = (ex.ended_at, ex.done_at) {
        if now - t0 >= ORBIT_SECS + HOLD_SECS || keys.just_pressed(KeyCode::Enter) { ex.done_at = Some(now); }
    }
    if ex.done_at.is_some_and(|d| now - d >= FADE_OUT) { ex.faded = true; }
    let fade = match (ex.started_at, ex.done_at) {
        (Some(_), Some(d)) => ((now - d) / FADE_OUT).min(1.) as f32,
        (Some(s), None) => (1. - (now - s) / FADE_IN).max(0.) as f32,
        _ => 0.,
    };
    let want = if ex.started && ex.done_at.is_none() { 1. } else { 0. };
    let dt = real.delta_secs() * BARS_RATE;
    ex.bars = if want > ex.bars { (ex.bars + dt).min(1.) } else { (ex.bars - dt).max(0.) };
    for e in &nodes { commands.entity(e).despawn(); }
    let Ok(window) = windows.single() else { return; };
    if fade > 0. {
        commands.spawn((Node { position_type: PositionType::Absolute, left: px(0.), top: px(0.), width: Val::Percent(100.), height: Val::Percent(100.), ..default() },
                        BackgroundColor(Color::srgba(0., 0., 0., fade)), GlobalZIndex(16500), BarNode));
    }
    if ex.bars <= 0. { return; }
    let h = window.height() * BARS * ex.bars;
    for top in [true, false] {
        let mut node = Node { position_type: PositionType::Absolute, left: px(0.), width: Val::Percent(100.), height: px(h), ..default() };
        if top { node.top = px(0.); } else { node.bottom = px(0.); }
        commands.spawn((node, BackgroundColor(Color::BLACK), GlobalZIndex(16000), BarNode));
    }
}
