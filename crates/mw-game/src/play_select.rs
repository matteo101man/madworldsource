//! The stage select [added, PC]: one mw-play for every stage. Without MW_STAGE the game opens this screen; the pick
//! relaunches mw-play with that stage (the stage is fixed when the app is built: rooms, banks and plugins), and F9 in a
//! fight comes back here the same way.
use bevy::prelude::*;

/// (MW_STAGE value, title, what it is)
pub(super) const STAGES: [(&str, &str, &str); 6] = [
    ("rinrin", "RinRin", "Asia Town boss: the c201 arena (ev21)"),
    ("elise", "Elise", "The church c303 (ev33 / ev34)"),
    ("baron", "The Black Baron", "The heliport ring c503 with Mathilda and the oirans (ev50 / ev51)"),
    ("mathilda", "Mathilda", "c503: the added Mathilda boss variant"),
    ("street", "Asia Town streets", "The kung-fu grunts"),
    ("town", "Asia Town free roam (test)", "By day: townsfolk about the streets, substories to step into"),
];

/// Relaunch mw-play on `stage` ("" opens the stage select) and close this process.
pub fn relaunch(stage: &str) { relaunch_with(stage, None); }
/// The same, opening the actor editor on `editor` (an actor id; its stage hosts it, no intro).
pub fn relaunch_with(stage: &str, editor: Option<&str>) {
    let Ok(exe) = std::env::current_exe() else { return; };
    let mut cmd = std::process::Command::new(exe);
    if stage.is_empty() { cmd.env_remove("MW_STAGE"); } else { cmd.env("MW_STAGE", stage); }
    match editor { Some(id) => { cmd.env("MW_EDITOR", id); } None => { cmd.env_remove("MW_EDITOR"); } }
    if let Err(e) = cmd.spawn() { eprintln!("[select] relaunch failed: {e}"); return; }
    std::process::exit(0);
}

#[derive(Resource, Default)]
struct Sel { at: usize }
/// The rows: the stages, then the actor editor's actors (assets/defs/actors with an `editor` page): (stage, editor id).
fn rows() -> Vec<(String, Option<String>, String, String)> {
    let mut v: Vec<(String, Option<String>, String, String)> = STAGES.iter().map(|(s, t, a)| (s.to_string(), None, t.to_string(), a.to_string())).collect();
    for d in super::actors::with_editor() {
        let stage = d.editor.as_ref().map(|e| e.stage.clone()).unwrap_or_default();
        v.push((stage, Some(d.id.clone()), format!("Edit {}", d.id), d.name.clone()));
    }
    v
}
#[derive(Component)]
struct Row(usize);

/// The stage select screen; returns when a stage is picked (by relaunching) or the window closes.
pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { title: "MadWorld native: stage select".into(), ..default() }), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.02, 0.02)))
        .init_resource::<Sel>()
        .add_systems(Startup, setup)
        .add_systems(Update, (input, follow_headset))
        .run();
}

/// A headset connected on the stage select starts the game in VR (its default stage; the VR menu picks others).
fn follow_headset(mut started: Local<bool>) {
    if !*started { *started = true; super::vr::auto::watch_flat(); }
    super::vr::auto::poll_switch();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn(Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center,
                          align_items: AlignItems::Center, row_gap: px(6), ..default() })
        .with_children(|p| {
            p.spawn((Text::new("SELECT STAGE"), TextFont { font_size: bevy::text::FontSize::Px(46.), ..default() }, TextColor(Color::srgb(0.85, 0.05, 0.05))));
            let rows = rows();
            let first_editor = rows.iter().position(|r| r.1.is_some());
            for (i, (_, ed, title, about)) in rows.iter().enumerate() {
                if Some(i) == first_editor {
                    p.spawn((Text::new("ACTOR EDITOR"), TextFont { font_size: bevy::text::FontSize::Px(26.), ..default() }, TextColor(Color::srgb(0.85, 0.05, 0.05)),
                             Node { margin: UiRect::top(px(10)), ..default() }));
                }
                let title = if ed.is_some() { title.clone() } else { format!("{}  {title}", i + 1) };
                let (big, small) = if ed.is_some() { (20., 12.) } else { (30., 15.) };
                p.spawn((Row(i), BackgroundColor(Color::NONE), Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, padding: UiRect::axes(px(24), px(6)), ..default() }))
                    .with_children(|r| {
                        r.spawn((Text::new(title), TextFont { font_size: bevy::text::FontSize::Px(big), ..default() }, TextColor(Color::WHITE)));
                        r.spawn((Text::new(about.clone()), TextFont { font_size: bevy::text::FontSize::Px(small), ..default() }, TextColor(Color::srgb(0.6, 0.6, 0.6))));
                    });
            }
            p.spawn((Text::new("Up / Down (or 1-6) to choose, Enter to play. F9 in a fight comes back here."),
                     TextFont { font_size: bevy::text::FontSize::Px(14.), ..default() }, TextColor(Color::srgb(0.5, 0.5, 0.5))));
        });
}

fn input(keys: Res<ButtonInput<KeyCode>>, mut sel: ResMut<Sel>, mut rows: Query<(&Row, &mut BackgroundColor)>, mut list: Local<Vec<(String, Option<String>, String, String)>>) {
    if list.is_empty() { *list = self::rows(); }
    let n = list.len();
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) { sel.at = (sel.at + 1) % n; }
    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) { sel.at = (sel.at + n - 1) % n; }
    let mut pick = (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space)).then_some(sel.at);
    for (k, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6].into_iter().enumerate() {
        if keys.just_pressed(key) { pick = Some(k); }
    }
    for (row, mut bg) in &mut rows { bg.0 = if row.0 == sel.at { Color::srgba(0.7, 0.04, 0.04, 0.55) } else { Color::NONE }; }
    if let Some(i) = pick { let r = &list[i]; relaunch_with(&r.0, r.1.as_deref()); }
}
