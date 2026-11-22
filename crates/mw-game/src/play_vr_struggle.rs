//! Power struggles in VR [added]: every struggle meter of the fight is fed by the fight's shake input, which the keyboard
//! mashes. In VR each struggle takes the gesture its own scene shows:
//! - **The Baron** (the clash chain, his arm lock): a punch battle. Throw real punches into him, again and again: each
//!   one counts; a swing step wants its punch (a hook across for SIDE, a hammer down for DOWN; a jab counts as either).
//! - **RinRin** (the keen competition): she drives her fans at you. Grab them (close a hand where they are, before her
//!   chest) and wrench them back toward you, again and again; her directional stage wants a sweep across.
//! - **Elise** (the bite clash): she holds you by the arms and leans in to bite. First grip her hands and tear them off
//!   (pull back and apart), then shove her away with open hands; her swing steps want a chop / sweep.
//! - **Mathilda's pin**: you lie under her bat. Push it off (both hands into it, again and again).
//! - **Dizzy**: shake your head. **The giant swing**: whirl your arms round.
//! Kreely's hands meet the enemy (they stop on her, close on what they hold); the view is your own while a struggle
//! engages you (not the theater), turned once to face her; the controllers rumble as she fights back; a cue on the HUD
//! names the move and shows its meter.
use super::{Rig, VrInput, VrSettings};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind { Punch { step: Option<bool> }, PullFans, Elise, Push, Swing { down: bool }, Dizzy, Spin }

#[derive(Resource, Default)]
pub struct Struggle {
    pub kind: Option<Kind>,
    /// the enemy's chest (world) and the surface the hands meet, along the push direction from the chest
    pub target: Vec3, pub contact: Option<(Vec3, Vec3)>,
    /// the shove's strength this frame (0..1), smoothed for the cue; the struggle's own meter (0..1) when it shows one
    pub effort: f32, pub meter: Option<f32>,
    /// a push or swing step engaged this paired scene: the view stays in your eyes until it ends
    pub engaged: bool,
    faced: bool, head_yaw: Option<f32>, head_dir: f32, head_at: f32, swing_at: f32, pulse: f32,
    pub log: Vec<String>,
    /// per hand: the last blow / pull (s), and a pull stroke under way (where it starts, along the pull axis)
    hit_at: [f32; 2], pull_from: [Option<f32>; 2],
    /// the hands close on what they hold (fans, her hands)
    pub gripping: bool,
    /// in a paired scene: where Kreely's animated eyes are and which way his head faces (the view rides there)
    pub anchor: Option<(Vec3, Vec3)>,
    /// the Baron's spot to hit (face / chest), moving on with each blow
    pub spot: u32,
    /// her neck (animated), for the face mark and the word over her
    pub neck: Option<Vec3>,
    /// how far his eyes were moved off the clip's (kept clear of her head): his copied body moves with them
    pub shift: Vec3,
    /// a native-camera scene (an event, a pair, a finisher, the defeat): played from his eyes too
    pub scene: bool,
}

#[derive(Component)] pub struct Cue;
#[derive(Component)] pub struct CueText;
#[derive(Component)] pub struct CueBar;

pub fn build(app: &mut App) {
    app.init_resource::<Struggle>().add_systems(Startup, setup);
    build_marks(app);
    build_label(app);
}

fn setup(mut commands: Commands) {
    let line = Color::srgb(0.96, 0.95, 0.98);
    commands.spawn((Cue, Node { position_type: PositionType::Absolute, left: percent(28), width: percent(44), top: percent(62), padding: UiRect::all(px(8)),
                               flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(6), border: UiRect::all(px(2)), ..default() },
                    BackgroundColor(Color::srgba(0.06, 0.05, 0.12, 0.82)), BorderColor::all(line), GlobalZIndex(2500), Visibility::Hidden))
        .with_children(|p| {
            p.spawn((CueText, Text::new(""), TextFont { font_size: bevy::text::FontSize::Px(30.), ..default() }, TextColor(Color::srgb(0.95, 0.12, 0.08))));
            p.spawn((Node { width: percent(90), height: px(14), border: UiRect::all(px(1)), ..default() }, BorderColor::all(line)))
                .with_children(|b| { b.spawn((CueBar, Node { width: percent(0), height: percent(100), ..default() }, BackgroundColor(Color::srgb(0.85, 0.85, 0.9)))); });
        });
}

/// The meter in a boss prompt ("... 45%").
fn meter_of(p: &str) -> Option<f32> { p.split_whitespace().last()?.strip_suffix('%')?.parse::<f32>().ok().map(|v| v / 100.) }

/// Update, before the view: which struggle runs now, and against whom.
#[allow(clippy::too_many_arguments)]
pub fn detect(mut st: ResMut<Struggle>, play: Res<super::super::Play>, elise: Res<super::super::elise::Elise>, baron: Res<super::super::baron::Baron>,
              chars: Query<(&super::super::Character, &Visibility)>, globals: Query<&GlobalTransform>, intro: Res<super::super::intro::Intro>,
              tv: Res<super::super::broadcast::Broadcast>, defeat: Res<super::super::DefeatCam>, execution: Res<super::super::execution::Execution>) {
    st.scene = super::native_scene(&play, &intro, &tv, &elise, &baron, &defeat, &execution);
    let pl = &play.fight.player;
    // a paired scene carries both fighters off their origin by the clips' root motion: his animated eyes are the truth
    st.anchor = None;
    if play.fight.paired || st.scene {
        let eye = play.kreely_bones.iter().position(|b| b.id == 40);
        let head = play.kreely_bones.iter().position(|b| b.id == 4);
        for (ch, vis) in &chars {
            if ch.who != super::super::Who::Kreely || ch.colored != play.colored || *vis == Visibility::Hidden { continue; }
            let (Some(js), Some(e), Some(hd)) = (&ch.joints, eye, head) else { continue; };
            if let (Ok(eg), Ok(hg)) = (globals.get(js[e]), globals.get(js[hd])) {
                let fwd = hg.rotation() * Vec3::Z;
                st.anchor = Some((eg.translation(), Vec3::new(fwd.x, 0., fwd.z).normalize_or(Vec3::Z)));
            }
        }
    }
    // who: Elise (0), the Baron (1), Mathilda (2)
    let boss = elise.prompt.clone().map(|p| (p, elise.body(), 0)).or(baron.prompt.clone().map(|p| (p, baron.body(), if super::super::baron::mathilda_boss() { 2 } else { 1 })));
    let rin = Vec3::from_array(play.fight.rin.pos) + Vec3::Y * 1.35;
    let tap = |p: &str| p.starts_with("TAP") || p.starts_with("SHAKE");
    let (kind, target, meter) = match (pl.state(), pl.loco.phase, &boss) {
        (78, 3, _) => (Some(Kind::PullFans), rin, None),
        (78, 5, _) => (Some(Kind::Swing { down: false }), rin, None),
        (_, _, Some((p, at, 1))) if tap(p) => (Some(Kind::Punch { step: None }), *at, meter_of(p)),
        (_, _, Some((p, at, 1))) if p.contains("DOWN") || p.contains("SIDE") => (Some(Kind::Punch { step: Some(p.contains("DOWN")) }), *at, None),
        (_, _, Some((p, at, 0))) if tap(p) => (Some(Kind::Elise), *at, meter_of(p)),
        (_, _, Some((p, at, _))) if tap(p) => (Some(Kind::Push), *at, meter_of(p)),
        (_, _, Some((p, at, _))) if p.contains("DOWN") => (Some(Kind::Swing { down: true }), *at, None),
        (_, _, Some((p, at, _))) if p.contains("SIDE") => (Some(Kind::Swing { down: false }), *at, None),
        (91, _, _) => (Some(Kind::Dizzy), Vec3::ZERO, None),
        (49, 1 | 3, _) => (Some(Kind::Spin), Vec3::ZERO, None),
        _ => (None, Vec3::ZERO, None),
    };
    if kind != st.kind {
        if std::env::var_os("MW_VR_LOG").is_some() { eprintln!("[vr] tick {} struggle: {:?}", play.tick, kind); }
        st.kind = kind; st.faced = false; st.effort = 0.;
    }
    if kind.is_some_and(|k| !matches!(k, Kind::Dizzy | Kind::Spin)) { st.engaged = true; }
    if kind.is_none() && !play.fight.paired { st.engaged = false; }
    // (paired, the enemy is where her animated body is: the mean of her joints, chest high)
    // her chest and neck bones, as animated (3, 4 in every native rig)
    let at = |e: Option<Entity>| e.and_then(|e| globals.get(e).ok()).map(|g| g.translation()).filter(|p| p.is_finite() && p.length() > 1e-3);
    let (chest, neck) = if elise.on { (at(elise.vr_joint(3)), at(elise.vr_joint(4))) } else if baron.on { (at(baron.vr_joint(3)), at(baron.vr_joint(4))) } else { (None, None) };
    let rin_joint = |id: i16| chars.iter().find(|(c, _)| c.who == super::super::Who::Rin && c.colored == play.colored).and_then(|(c, _)| c.joints.as_ref())
        .and_then(|js| play.fd.rin.bones.iter().position(|b| b.id == id).and_then(|i| js.get(i).copied()));
    let (rin_chest, rin_neck) = (at(rin_joint(3)), at(rin_joint(4)));
    let (target, neck) = match kind {
        Some(Kind::PullFans) | Some(Kind::Swing { .. }) if pl.state() == 78 => (rin_chest.unwrap_or(target), rin_neck),
        Some(k) if !matches!(k, Kind::Dizzy | Kind::Spin) && play.fight.paired => (chest.unwrap_or(target), neck),
        _ => (target, None),
    };
    // face to face, his eyes keep 0.6 m from her head (the clips' lock-ups nearly touch: fine in third person, inside her
    // hat brim in his eyes)
    st.shift = Vec3::ZERO;
    if let (Some((eye, fwd)), Some(n)) = (st.anchor, neck) {
        let d = Vec2::new(eye.x - n.x, eye.z - n.z);
        if d.length() < 0.75 { let away = d.normalize_or(-Vec2::new(fwd.x, fwd.z)) * (0.75 - d.length()); st.anchor = Some((eye + Vec3::new(away.x, 0., away.y), fwd)); st.shift = Vec3::new(away.x, 0., away.y); }
    }
    if std::env::var_os("MW_VR_LOG").is_some() && kind.is_some() && play.tick % 15 == 0 {
        if let Some((eye, fwd)) = st.anchor { eprintln!("[vr] struggle at tick {}: his eyes {:.2} facing {:.2}; her body {:.2} ({:.2} m away)", play.tick, eye, fwd, target, Vec2::new(target.x - eye.x, target.z - eye.z).length()); }
    }
    st.target = target; st.meter = meter; st.neck = neck;
}

/// Whether the struggle wants the view in the player's own eyes (a push or a swing step against someone).
pub fn first_person(st: &Struggle) -> bool { st.engaged || st.kind.is_some_and(|k| !matches!(k, Kind::Dizzy | Kind::Spin)) }

/// Update, after the pad: the struggle's gesture as the fight's input.
#[allow(clippy::too_many_arguments)]
pub fn drive(mut st: ResMut<Struggle>, mut play: ResMut<super::super::Play>, mut rig: ResMut<Rig>, mut inp: ResMut<VrInput>, set: Res<VrSettings>,
             time: Res<Time<Real>>, pause: Res<super::super::pause::Pause>, mut cue: Query<&mut Visibility, With<Cue>>,
             mut text: Query<(&mut Text, &mut TextColor), With<CueText>>, mut bar: Query<&mut Node, With<CueBar>>) {
    let dt = time.delta_secs().max(1e-4);
    let now = time.elapsed_secs();
    let log = std::env::var_os("MW_VR_LOG").is_some();
    let Some(kind) = st.kind.filter(|_| !pause.open && !super::menu_open()) else {
        st.contact = None; st.gripping = false;
        for mut v in &mut cue { *v = Visibility::Hidden; }
        return;
    };
    let p = &mut *play;
    // facing her once as it begins
    // (once, after the view has settled on his animated eyes in a paired scene)
    let settled = st.anchor.is_none_or(|(eye, _)| Vec2::new(eye.x - rig.head.translation.x, eye.z - rig.head.translation.z).length() < 0.08);
    if !st.faced && settled && !matches!(kind, Kind::Dizzy | Kind::Spin) {
        let to = st.target - rig.head.translation;
        if to.length() > 0.1 {
            let want = to.x.atan2(to.z);
            let d = (want - super::game_yaw(rig.head.rotation) + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            rig.turn(d);
        }
        st.faced = true;
    }
    let chest = chest_of(&rig);
    let word;
    st.gripping = false;
    // toward her: level, tipped a little to her chest's height
    let flat = Vec3::new(st.target.x - chest.x, 0., st.target.z - chest.z);
    let toward = (flat.normalize_or(rig.head.rotation * Vec3::NEG_Z) + Vec3::Y * (st.target.y - chest.y).clamp(-0.6, 0.6) * 0.5).normalize();
    let reach_her = (flat.length() - 0.3).clamp(0.3, 0.62);
    match kind {
        Kind::Punch { step } => {
            word = match step { None => "PUNCH HIM!", Some(true) => "HAMMER DOWN!", Some(false) => "HOOK HIM!" };
            st.contact = None;
            let right = Vec3::new(-toward.z, 0., toward.x);
            for h in 0..2 {
                let v = rig.vel[h];
                let (fwd, down, across) = (v.dot(toward), -v.y, v.dot(right).abs());
                let ext = (rig.grip[h].translation - chest).dot(toward);
                // a blow: the fist fast, out at him, one per hand at a time
                if v.length() > 2.4 && v.length() < 20. && ext > reach_her - 0.2 && now - st.hit_at[h] > 0.18 && (fwd > 1.2 || down > 1.8 || across > 1.8) {
                    st.hit_at[h] = now; st.spot += 1;
                    inp.rumble(h, 0.85, 0.07);
                    st.effort = 1.;
                    let kind_of = if down > fwd.max(across) { "hammer" } else if across > fwd { "hook" } else { "jab" };
                    match step {
                        None => { p.latched.mash = true; }
                        Some(want_down) => {
                            let down_blow = match kind_of { "hammer" => true, "hook" => false, _ => want_down };
                            if down_blow { p.latched.grab = true; } else { p.latched.swing_h = true; }
                        }
                    }
                    if log { eprintln!("[vr] tick {} punch: {kind_of} ({:.1} m/s, hand {h})", p.tick, v.length()); }
                }
            }
            st.effort *= 1. - (dt * 4.).min(1.);
        }
        Kind::PullFans | Kind::Elise => {
            // Elise: her hands off first (below half the meter), then her away; RinRin: her fans all the way
            let pulling = kind == Kind::PullFans || st.meter.is_none_or(|m| m < 0.5);
            word = match (kind, pulling) { (Kind::PullFans, _) => "TEAR HER FANS AWAY!", (_, true) => "PULL HER HANDS OFF!", _ => "SHOVE HER AWAY!" };
            st.contact = Some((chest + toward * reach_her, toward));
            if pulling {
                let mut strength: f32 = 0.;
                for h in 0..2 {
                    let gripping = inp.hands[h].grip > 0.55;
                    let along = (rig.grip[h].translation - chest).dot(toward);
                    match (gripping, st.pull_from[h]) {
                        (true, None) if along > reach_her - 0.18 => { st.pull_from[h] = Some(along); inp.rumble(h, 0.4, 0.05); }
                        (true, Some(from)) => {
                            st.gripping = true;
                            // wrenched back toward you: a stroke of 15 cm counts
                            let back = from - along;
                            strength = strength.max((back / 0.15).clamp(0., 1.));
                            if back > 0.15 && now - st.hit_at[h] > 0.15 {
                                st.hit_at[h] = now; p.latched.mash = true; inp.rumble(h, 0.8, 0.08);
                                if log { eprintln!("[vr] tick {} pull: {} (hand {h})", p.tick, if kind == Kind::PullFans { "fan" } else { "her hand" }); }
                            }
                            // the anchor follows the hand back out (re-reach and pull again)
                            if let Some(f) = st.pull_from[h].as_mut() { if along > *f || back > 0.15 { *f = along; } }
                            inp.rumble(h, 0.2, 0.04);
                        }
                        (true, None) => { if log && now - st.pulse > 0.3 { st.pulse = now; eprintln!("[vr] grip on hand {h} too short of her: {:.2} m out, her {:.2} m", along, reach_her); } }
                        (false, _) => { st.pull_from[h] = None; }
                    }
                }
                st.effort += (strength - st.effort) * (dt * 8.).min(1.);
            } else {
                st.pull_from = [None; 2];
                let mut push: f32 = 0.;
                for h in 0..2 {
                    let into = (rig.grip[h].translation - chest).dot(toward) - reach_her;
                    let vel = rig.vel[h].dot(toward);
                    if into > -0.06 { push += (vel / 1.4).clamp(0., 1.) * 0.8 + (into / 0.12).clamp(0., 1.) * 0.2; inp.rumble(h, 0.2 + 0.5 * push, 0.05); }
                }
                let push = push / 2.;
                st.effort += (push - st.effort) * (dt * 10.).min(1.);
                if push > 0.35 { p.latched.mash = true; if now - st.pulse > 0.25 { st.pulse = now; if log { eprintln!("[vr] tick {} shove ({push:.2})", p.tick); } } }
            }
        }
        Kind::Push => {
            word = "PUSH HER BAT OFF!";
            let flat = Vec3::new(st.target.x - chest.x, 0., st.target.z - chest.z);
            let dir = (flat.normalize_or(rig.head.rotation * Vec3::NEG_Z) + Vec3::Y * (st.target.y - chest.y).clamp(-0.6, 0.6) * 0.5).normalize();
            // where the hands meet her: her surface toward us, kept within his reach
            let reach = (flat.length() - 0.32).clamp(0.3, 0.62);
            st.contact = Some((chest + dir * reach, dir));
            let mut push = 0.;
            let mut touching = 0;
            for h in 0..2 {
                let ext = (rig.grip[h].translation - chest).dot(dir);
                let into = ext - reach;
                let vel = rig.vel[h].dot(dir);
                if into > -0.06 {
                    touching += 1;
                    // a shove: the hand driving into her; resting on her counts a little
                    push += (vel / 1.4).clamp(0., 1.) * 0.8 + (into / 0.12).clamp(0., 1.) * 0.2;
                }
            }
            let push = push / 2.;
            st.effort += (push - st.effort) * (dt * 10.).min(1.);
            // she pushes back: a steady rumble while you hold her, a kick on each shove
            for h in 0..2 { if touching > 0 { inp.rumble(h, 0.18 + 0.5 * push, 0.05); } }
            if push > 0.35 {
                p.latched.mash = true;
                if now - st.pulse > 0.25 { st.pulse = now; if log { st.log.push(format!("shove {push:.2}")); eprintln!("[vr] tick {} shove ({push:.2}, hands on her: {touching})", p.tick); } }
            }
        }
        Kind::Swing { down } => {
            word = if down { "CHOP DOWN!" } else { "SWING ACROSS!" };
            st.contact = None;
            for h in 0..2 {
                let v = rig.vel[h];
                // against the body: down (a chop), across (left / right of the view), a thrust ahead counts as neither
                let fwd = { let f = rig.head.rotation * Vec3::NEG_Z; Vec3::new(f.x, 0., f.z).normalize_or(Vec3::Z) };
                let right = Vec3::new(-fwd.z, 0., fwd.x);
                let (fall, across, ahead) = (-v.y, v.dot(right).abs(), v.dot(fwd).abs());
                if v.length() > 3.0 && now - st.swing_at > 0.45 && (fall > ahead || across > ahead) {
                    let vertical = fall > across;
                    st.swing_at = now;
                    if vertical { p.latched.grab = true; } else { p.latched.swing_h = true; }
                    inp.rumble(h, 0.8, 0.08);
                    if log { eprintln!("[vr] tick {} swing {} at {:.1} m/s (wanted {})", p.tick, if vertical { "down" } else { "across" }, v.length(), if down { "down" } else { "across" }); }
                }
            }
        }
        Kind::Dizzy => {
            word = "SHAKE IT OFF!";
            st.contact = None;
            // the head turning back and forth fast
            let yaw = super::game_yaw(rig.head.rotation);
            if let Some(last) = st.head_yaw {
                let d = (yaw - last + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                let w = d / dt;
                if w.abs() > 2.2 {
                    if w.signum() != st.head_dir && now - st.head_at < 0.5 { p.latched.mash = true; st.effort = 1.; if log { eprintln!("[vr] tick {} head shake", p.tick); } }
                    st.head_dir = w.signum(); st.head_at = now;
                }
            }
            st.head_yaw = Some(yaw);
            st.effort *= 1. - (dt * 3.).min(1.);
        }
        Kind::Spin => {
            word = "SWING HIM ROUND!";
            st.contact = None;
            // whirling: either hand fast and turning (its velocity's direction changing)
            let fast = (0..2).map(|h| rig.vel[h].length()).fold(0., f32::max);
            if fast > 1.8 { p.latched.mash = true; st.effort = (fast / 4.).min(1.); } else { st.effort *= 0.9; }
        }
    }
    let _ = set;
    // the cue on the HUD
    // (the word now hangs in the world in the game's lettering: label below)
    for mut v in &mut cue { *v = Visibility::Hidden; }
    for (mut t, mut c) in &mut text {
        let line = match (kind, st.meter) { (Kind::Push, Some(m)) => format!("{word}   {:.0}%", m * 100.), _ => word.to_string() };
        if t.0 != line { t.0 = line; }
        c.0 = if st.effort > 0.35 { Color::srgb(1., 0.9, 0.85) } else { Color::srgb(0.95, 0.12, 0.08) };
    }
    for mut n in &mut bar { n.width = percent(st.meter.unwrap_or(st.effort).clamp(0., 1.) * 100.); }
}

// ---------------------------------------------------------------- the marks in the world

/// A struggle mark: a ring (the lock-on marker's), the gun's lock sight, or the marker's arrow, standing up (facing the
/// eyes) or lying on the air flat (pointing the way to pull / push).
#[derive(Component)] pub struct Mark(usize);
const MARKS: usize = 7;
const YELLOW: Color = Color::srgb(1., 0.8, 0.);
const RED_HOT: Color = Color::srgb(1., 0.15, 0.08);

pub fn build_marks(app: &mut App) {
    app.add_systems(Startup, spawn_marks).add_systems(Update, marks.in_set(super::VrActive).after(drive).after(super::rig));
}

fn spawn_marks(mut commands: Commands, assets: Res<AssetServer>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let quad = meshes.add(Rectangle::new(1., 1.));
    // 0, 1: rings; 2: the sight; 3..6: arrows
    for i in 0..MARKS {
        let file = match i { 0 | 1 => "native/ui/lock/ring.png", 2 => "native/ui/gun/sight-lock.png", _ => "native/ui/lock/arrow.png" };
        let mat = materials.add(StandardMaterial { base_color: YELLOW, base_color_texture: Some(assets.load(file)), unlit: true, alpha_mode: AlphaMode::Blend,
                                                   cull_mode: None, fog_enabled: false, depth_bias: super::OVERLAY_BIAS, ..default() });
        commands.spawn((Mark(i), Mesh3d(quad.clone()), MeshMaterial3d(mat), Transform::default(), Visibility::Hidden,
                        bevy::camera::visibility::RenderLayers::layer(super::L_EYES), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
    }
}

/// Update: where to strike, grab or push, in the game's own lock-on art.
fn marks(st: Res<Struggle>, rig: Res<Rig>, time: Res<Time<Real>>, mut q: Query<(&Mark, &mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>)>,
         mut materials: ResMut<Assets<StandardMaterial>>) {
    let now = time.elapsed_secs();
    let _head = rig.head.translation;
    let chest = chest_of(&rig);
    let flat = Vec3::new(st.target.x - chest.x, 0., st.target.z - chest.z);
    let toward = flat.normalize_or(rig.head.rotation * Vec3::NEG_Z);
    let right = Vec3::new(-toward.z, 0., toward.x);
    let reach = (flat.length() - 0.3).clamp(0.3, 0.62);
    // (mark, position, size, colour, how it turns: None faces the eyes; Some(dir) lies flat pointing along dir)
    let mut want: Vec<(usize, Vec3, f32, Color, Option<Vec3>)> = vec![];
    let pulse = 1. + 0.12 * (now * 7.).sin();
    let hit = (0..2).map(|h| now - st.hit_at[h]).fold(9., f32::min) < 0.2;
    let hot = if hit || st.effort > 0.5 { RED_HOT } else { YELLOW };
    let slide = |speed: f32| ((now * speed).fract() - 0.5) * 0.24;
    match st.kind {
        Some(Kind::Punch { step }) => {
            // the spot: his face, then his chest, alternating with each blow
            // on his face (over the neck bone), then his chest, a hand's width toward you
            let face = match st.neck { Some(n) if st.spot % 2 == 0 => n + Vec3::Y * 0.14 - toward * 0.12, _ => st.target - toward * 0.15 };
            want.push((0, face, 0.24 * pulse, hot, None));
            want.push((2, face, 0.13, hot, None));
            match step {
                Some(true) => { want.push((3, face + Vec3::Y * 0.3 - Vec3::Y * slide(1.6).abs(), 0.14, YELLOW, None)); }
                Some(false) => { want.push((3, face + right * 0.3, 0.14, YELLOW, Some(-right))); }
                None => {}
            }
        }
        Some(Kind::PullFans) | Some(Kind::Elise) if st.contact.is_some() && !(st.kind == Some(Kind::Elise) && st.meter.is_some_and(|m| m >= 0.5)) => {
            // her fans / her hands: grab them there, then pull back toward you
            for (k, side) in [-1f32, 1.].into_iter().enumerate() {
                let at = chest + toward * reach + right * side * 0.2 + Vec3::Y * 0.05;
                want.push((k, at, 0.17 * if st.gripping { 0.85 } else { pulse }, if st.gripping { RED_HOT } else { YELLOW }, None));
                want.push((3 + k, at - Vec3::Y * 0.16 - toward * (0.12 + slide(1.3)), 0.12, YELLOW, Some(-toward)));
            }
        }
        Some(Kind::Elise) | Some(Kind::Push) => {
            // her chest (Mathilda: the bat): both hands on it, drive it away
            let at = chest + toward * reach + Vec3::Y * 0.1;
            want.push((0, at, 0.3 * pulse, hot, None));
            for (k, side) in [-1f32, 1.].into_iter().enumerate() {
                want.push((3 + k, at + right * side * 0.16 - Vec3::Y * 0.2 + toward * (0.12 + slide(1.3)), 0.12, YELLOW, Some(toward)));
            }
        }
        Some(Kind::Swing { down }) => {
            let at = chest + toward * (reach + 0.1) + Vec3::Y * 0.2;
            if down { want.push((3, at + Vec3::Y * 0.2 - Vec3::Y * slide(1.6).abs(), 0.18, YELLOW, None)); }
            else { want.push((3, at + right * slide(1.6), 0.18, YELLOW, Some(-right))); }
        }
        _ => {}
    }
    let cam = rig.head;
    for (m, mut t, mut v, mat) in &mut q {
        match want.iter().find(|w| w.0 == m.0) {
            Some(&(_, at, size, color, lie)) => {
                *v = Visibility::Visible;
                t.translation = at;
                t.scale = Vec3::splat(size);
                t.rotation = match lie {
                    // flat, the arrow's tip (its -Y in the image) along dir
                    Some(dir) => Quat::from_rotation_arc(Vec3::NEG_Y, dir.normalize_or(Vec3::Z)) * Quat::from_rotation_x(0.) ,
                    None => Transform::from_translation(at).looking_at(at + (at - cam.translation), Vec3::Y).rotation,
                };
                if let Some(dir) = lie {
                    // lay it down: the quad's face up, its tip along dir
                    let d = Vec3::new(dir.x, 0., dir.z).normalize_or(Vec3::Z);
                    t.rotation = Quat::from_mat3(&Mat3::from_cols(d.cross(Vec3::Y).normalize_or(Vec3::X), -d, Vec3::Y));
                    if dir.y.abs() > 0.9 { t.rotation = Transform::from_translation(at).looking_at(at + (at - cam.translation), Vec3::Y).rotation; }
                }
                if let Some(mut mm) = materials.get_mut(&mat.0) { if mm.base_color != color { mm.base_color = color; } }
            }
            None => { if *v != Visibility::Hidden { *v = Visibility::Hidden; } }
        }
    }
}

// ---------------------------------------------------------------- the word in the world

/// The struggle's word over the enemy, in the brush capitals of core_us (the game's red title lettering, a dark brush
/// shadow under it) with its meter ruled in white as the pause menu's AREA DATA box rules: drawn to a texture by the
/// game's own painter and hung in the world over her mark.
#[derive(Resource)] pub struct Label { image: Handle<Image>, cam: Entity, root: Entity, quad: Entity, key: String, fonts: Option<Option<super::super::editor::Fonts>> }
const LW: f32 = 640.;
const LH: f32 = 170.;

pub fn build_label(app: &mut App) {
    app.add_systems(Startup, spawn_label).add_systems(Update, label.in_set(super::VrActive).after(drive).after(super::rig));
}

fn spawn_label(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut img = Image::new_target_texture(LW as u32, LH as u32, bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb, None);
    img.sampler = bevy::image::ImageSampler::linear();
    let image = images.add(img);
    let cam = commands.spawn((Camera2d, Camera { order: -1, is_active: false, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
                              bevy::camera::RenderTarget::from(image.clone()), bevy::camera::visibility::RenderLayers::layer(5), Msaa::Off)).id();
    let root = commands.spawn((Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: px(LW), height: px(LH), ..default() },
                               bevy::ui::UiTargetCamera(cam))).id();
    let mat = materials.add(StandardMaterial { base_color_texture: Some(image.clone()), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, fog_enabled: false,
                                               depth_bias: super::OVERLAY_BIAS, ..default() });
    let quad = commands.spawn((Mesh3d(meshes.add(Rectangle::new(1., LH / LW))), MeshMaterial3d(mat), Transform::default(), Visibility::Hidden,
                               bevy::camera::visibility::RenderLayers::layer(super::L_EYES), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver)).id();
    commands.insert_resource(Label { image, cam, root, quad, key: String::new(), fonts: None });
}

fn word_of(st: &Struggle) -> Option<&'static str> {
    Some(match st.kind? {
        Kind::Punch { step: None } => "PUNCH HIM", Kind::Punch { step: Some(true) } => "HAMMER DOWN", Kind::Punch { step: Some(false) } => "HOOK HIM",
        Kind::PullFans => "TEAR HER FANS AWAY",
        Kind::Elise => if st.meter.is_none_or(|m| m < 0.5) { "PULL HER HANDS OFF" } else { "SHOVE HER AWAY" },
        Kind::Push => "PUSH HER BAT OFF",
        Kind::Swing { down: true } => "CHOP DOWN", Kind::Swing { down: false } => "SWING ACROSS",
        Kind::Dizzy => "SHAKE IT OFF", Kind::Spin => "SWING HIM ROUND",
    })
}

#[allow(clippy::too_many_arguments)]
fn label(mut lb: ResMut<Label>, st: Res<Struggle>, rig: Res<Rig>, assets: Res<AssetServer>, mut commands: Commands, children: Query<&Children>,
         mut cams: Query<&mut Camera>, mut tf: Query<&mut Transform>, mut vis: Query<&mut Visibility>) {
    let word = word_of(&st);
    let shown = word.is_some();
    if let Ok(mut c) = cams.get_mut(lb.cam) { if c.is_active != shown { c.is_active = shown; } }
    if let Ok(mut v) = vis.get_mut(lb.quad) { *v = if shown { Visibility::Visible } else { Visibility::Hidden }; }
    let Some(word) = word else { lb.key.clear(); return; };
    // over her mark (in the dizzy shake and the spin: before the eyes), facing the eyes
    let head = rig.head.translation;
    let fwd = { let f = rig.head.rotation * Vec3::NEG_Z; Vec3::new(f.x, 0., f.z).normalize_or(Vec3::Z) };
    let at = match st.kind { Some(Kind::Dizzy) | Some(Kind::Spin) => head + fwd * 1.3 + Vec3::Y * 0.25, _ => {
        let d = Vec3::new(st.target.x - head.x, 0., st.target.z - head.z);
        let over = st.neck.unwrap_or(st.target) + Vec3::Y * 0.45;
        // (not nearer than 0.9 m, so it reads)
        let d2 = over - head; if d2.length() < 0.9 { head + d2.normalize_or(fwd) * 0.9 } else { over + d.normalize_or(fwd) * 0.0 } } };
    // upright, turned to the eyes about the vertical only
    let face = Vec3::new(at.x - head.x, 0., at.z - head.z).normalize_or(fwd);
    if let Ok(mut t) = tf.get_mut(lb.quad) { *t = Transform::from_translation(at).looking_to(face, Vec3::Y).with_scale(Vec3::splat(0.7)); }
    let meter = st.meter.unwrap_or(st.effort).clamp(0., 1.);
    let key = format!("{word}|{}", (meter * 40.) as i32);
    if lb.key == key { return; }
    lb.key = key;
    let root = lb.root;
    if let Ok(kids) = children.get(root) { for e in kids.iter() { commands.entity(e).despawn(); } }
    if lb.fonts.is_none() { lb.fonts = Some(super::super::editor::load_fonts()); }
    let Some(Some(fonts)) = lb.fonts.take() else { lb.fonts = Some(None); return; };
    {
        let mut p = super::super::editor::Painter { commands: &mut commands, assets: &assets, fonts: &fonts, k: 1., offset: Vec2::ZERO, layer: false, parent: Some(root) };
        let h = 78.;
        let (_, w) = fonts.brush_layout(word, h);
        let x = ((LW - w) * 0.5).max(4.);
        // the brush shadow, then the red
        p.brush(word, x + 4., 10. + 4., h, Color::srgba(0.02, 0.0, 0.02, 0.85));
        p.brush(word, x, 10., h, Color::srgb(0.86, 0.04, 0.04));
        // the meter: white rules, filled white as it grows
        let (mx, my, mw, mh, t) = (LW * 0.18, 112., LW * 0.64, 22., 2.);
        let line = Color::srgb(0.96, 0.95, 0.98);
        p.rect(mx, my, mw, mh, Color::srgba(0.06, 0.05, 0.12, 0.7), 100);
        p.rect(mx, my, mw, t, line, 102); p.rect(mx, my + mh - t, mw, t, line, 102); p.rect(mx, my, t, mh, line, 102); p.rect(mx + mw - t, my, t, mh, line, 102);
        p.rect(mx + 4., my + 4., (mw - 8.) * meter, mh - 8., line, 101);
    }
    lb.fonts = Some(Some(fonts));
    let _ = &lb.image;
}

/// Where the hands push / pull from: his chest standing; lying (the clip has his eyes near the floor), his face.
fn chest_of(rig: &Rig) -> Vec3 {
    let up = (rig.head.translation.y - (rig.root.translation.y - rig.offset - rig.lift)).clamp(0., 2.);
    rig.head.translation - Vec3::Y * (0.38 * ((up - 0.5) / 0.9).clamp(0., 1.))
}
