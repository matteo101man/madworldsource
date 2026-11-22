//! The VR menu [added]: the left controller's menu button opens it (the fight holds), a panel in the pause menu's own
//! look (its dark violet AREA DATA box ruled in white, the white title bar, the message font; the brush capitals of
//! core_us for the headings, in the stage select's red) floating before you. Point with the gun hand's laser (or
//! the sticks) and pull the trigger (or press A); B goes back; left / right (or a click on the value) change a setting.
//! Pages: the menu (resume, VR settings, controls, the game's own pause menu, recenter, stage select, quit), the VR
//! settings (saved as they change), the controls, the stage list.
use super::{Btn, Rig, VrInput, VrSettings, OVERLAY_BIAS};
use super::super::editor::{Fonts, Painter};
use bevy::{camera::{visibility::RenderLayers, RenderTarget}, image::ImageSampler, prelude::*, render::render_resource::TextureFormat, ui::UiTargetCamera};

/// The menu texture, in the painter's units (640 x 480) at twice that.
const W: f32 = 640.;
const H: f32 = 480.;
const SCALE: f32 = 2.;
/// The panel's width in the world (m) and its distance.
const PANEL_W: f32 = 1.05;
const PANEL_D: f32 = 1.15;

const LINE: Color = Color::srgb(0.96, 0.95, 0.98);
const TITLE: Color = Color::srgb(0.42, 0.40, 0.52);
const PANEL: Color = Color::srgba(0.06, 0.05, 0.12, 0.985);
const RED: Color = Color::srgb(0.82, 0.04, 0.04);
const DIM: Color = Color::srgb(0.62, 0.6, 0.68);
const HILITE: Color = Color::srgba(0.78, 0.04, 0.04, 0.72);

#[derive(Resource, Default)]
pub struct MenuRequest { pub open: bool, pub key: Option<KeyCode> }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Page { #[default] Main, Settings, Controls, Stages }

const MAIN: [(&str, &str); 7] = [("RESUME", "Back to the fight"), ("VR SETTINGS", "Turning, movement, comfort, hands and the HUD"),
    ("CONTROLS", "What the Touch controllers do"), ("GAME MENU", "The game's own pause menu (sound, controls, save)"),
    ("RECENTER VIEW", "Face forward over Kreely and measure your height again"), ("STAGE SELECT", "Play another stage (the game restarts on it)"),
    ("QUIT GAME", "Close the game")];
const VISIBLE_ROWS: usize = 11;

/// VR switched off (the headset went away) with the menu open: it closes, the fight runs again.
fn shut(menu: Option<ResMut<Menu>>, mut vtime: ResMut<Time<Virtual>>, mut cams: Query<&mut Camera>) {
    let Some(mut menu) = menu else { return; };
    if menu.open { menu.open = false; super::set_menu_open(false); vtime.unpause(); }
    if let Ok(mut c) = cams.get_mut(menu.cam) { if c.is_active { c.is_active = false; } }
}

#[derive(Resource)]
struct Menu {
    open: bool, page: Page, sel: usize, top: usize, key: String,
    image: Handle<Image>, cam: Entity, root: Entity, panel: Entity, laser: Entity, dot: Entity,
    /// The rows' hit rectangles on this page (painter units): (row index, x, y, w, h), and the value arrows (row, left?, rect).
    rows: Vec<(usize, Rect)>, arrows: Vec<(usize, i32, Rect)>,
    /// Where the panel hangs while open.
    at: Transform,
    /// The stick's repeat state.
    repeat: (i8, f64),
    fonts: Option<Option<Fonts>>,
    /// A pointer press is pending release (a click acts once).
    pressed: bool,
    notice: Option<(String, f64)>,
    /// Where the pointer was on the panel last frame (it picks a row only when it moves).
    hover: Option<Vec2>,
}

#[derive(Component)] struct MenuPanel;
#[derive(Component)] struct MirrorMenu;
/// The dark shell round the head while the menu is open (the world dims, as under the native pause).
#[derive(Component)] struct MenuDim;

pub fn build(app: &mut App) {
    app.init_resource::<MenuRequest>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, shut.run_if(|| !super::on()))
        .add_systems(Update, (control, draw, place).chain().in_set(super::VrActive).after(super::panels));
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut img = Image::new_target_texture((W * SCALE) as u32, (H * SCALE) as u32, TextureFormat::Rgba8UnormSrgb, None);
    img.sampler = ImageSampler::linear();
    let image = images.add(img);
    let cam = commands.spawn((Camera2d, Camera { order: -1, is_active: false, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
                              RenderTarget::from(image.clone()), RenderLayers::layer(5), Msaa::Off)).id();
    let root = commands.spawn((Node { position_type: PositionType::Absolute, left: px(0), top: px(0), width: px(W * SCALE), height: px(H * SCALE), ..default() },
                               UiTargetCamera(cam), Visibility::Hidden)).id();
    let layers = RenderLayers::from_layers(&[super::L_EYES, super::L_CINEMA]);
    let mat = materials.add(StandardMaterial { base_color_texture: Some(image.clone()), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, fog_enabled: false, depth_bias: OVERLAY_BIAS, ..default() });
    let panel = commands.spawn((MenuPanel, Mesh3d(meshes.add(Rectangle::new(1., H / W))), MeshMaterial3d(mat), Transform::default(), Visibility::Hidden, layers.clone(),
                                bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver)).id();
    let beam = materials.add(StandardMaterial { base_color: Color::srgba(1., 0.1, 0.08, 0.75), emissive: LinearRgba::rgb(3., 0.2, 0.1), unlit: true, alpha_mode: AlphaMode::Blend, fog_enabled: false, depth_bias: OVERLAY_BIAS, ..default() });
    let laser = commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.004, 0.004, 1.))), MeshMaterial3d(beam.clone()), Transform::default(), Visibility::Hidden, layers.clone(),
                                bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver)).id();
    let dot = commands.spawn((Mesh3d(meshes.add(Sphere::new(0.008))), MeshMaterial3d(beam), Transform::default(), Visibility::Hidden, layers,
                              bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver)).id();
    // the dim: a shell 0.9 m round the head, between the hands and the panel
    let dim = materials.add(StandardMaterial { base_color: Color::srgba(0.01, 0.0, 0.02, 0.62), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, fog_enabled: false, ..default() });
    commands.spawn((MenuDim, Mesh3d(meshes.add(Sphere::new(0.9).mesh().uv(24, 12))), MeshMaterial3d(dim), Transform::default(), Visibility::Hidden,
                    RenderLayers::from_layers(&[super::L_EYES, super::L_CINEMA]), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
    if super::mode() == super::Mode::OpenXr {
        commands.spawn((MirrorMenu, Sprite { image: image.clone(), custom_size: Some(Vec2::new(W, H)), ..default() }, Transform::from_xyz(0., 0., 2.), Visibility::Hidden, RenderLayers::layer(super::L_MIRROR)));
    }
    commands.insert_resource(Menu { open: false, page: Page::Main, sel: 0, top: 0, key: String::new(), image, cam, root, panel, laser, dot, rows: vec![], arrows: vec![],
                                    at: Transform::IDENTITY, repeat: (0, 0.), fonts: None, pressed: false, notice: None, hover: None });
}

fn rows_on(page: Page) -> usize {
    match page { Page::Main => MAIN.len(), Page::Settings => super::settings::ROWS + 1, Page::Controls => 1, Page::Stages => super::super::select::STAGES.len() + 1 }
}

/// Update: opening, the pointer and the sticks, the choices.
#[allow(clippy::too_many_arguments)]
fn control(mut menu: ResMut<Menu>, mut req: ResMut<MenuRequest>, inp: Res<VrInput>, mut rig: ResMut<Rig>, mut set: ResMut<VrSettings>, time: Res<Time<Real>>,
           mut exit: MessageWriter<AppExit>, mut cams: Query<&mut Camera>, keys: Res<ButtonInput<KeyCode>>, mut inp_mut: ResMut<super::interact::Interact>,
           mut tf: Query<&mut Transform>, mut vis: Query<&mut Visibility>, pause: Res<super::super::pause::Pause>, mut vtime: ResMut<Time<Virtual>>) {
    let now = time.elapsed_secs_f64();
    // F10 opens it from the desktop as well
    let toggle = req.open || keys.just_pressed(KeyCode::F10);
    req.open = false;
    if !menu.open && toggle && !pause.open {
        menu.open = true; menu.page = Page::Main; menu.sel = 0; menu.top = 0; menu.key.clear();
        // hang it before the view, level
        let yaw = super::game_yaw(rig.head.rotation);
        let fwd = Vec3::new(yaw.sin(), 0., yaw.cos());
        let pos = rig.head.translation + fwd * PANEL_D - Vec3::Y * 0.08;
        menu.at = Transform::from_translation(pos).looking_at(pos + fwd, Vec3::Y).with_scale(Vec3::splat(PANEL_W));
        super::set_menu_open(true);
        // the world stands still, as under the native pause
        vtime.pause();
        eprintln!("[vr] menu open");
        return;
    }
    if !menu.open { if let Ok(mut c) = cams.get_mut(menu.cam) { if c.is_active { c.is_active = false; } } return; }
    if let Ok(mut c) = cams.get_mut(menu.cam) { if !c.is_active { c.is_active = true; } }
    let close = |menu: &mut Menu, vtime: &mut Time<Virtual>| { menu.open = false; super::set_menu_open(false); vtime.unpause(); eprintln!("[vr] menu closed"); };
    if inp.just(0, Btn::Menu) || keys.just_pressed(KeyCode::Escape) { close(&mut menu, &mut vtime); return; }
    // the pointer: the gun hand's aim ray on the panel
    let g = set.main_hand();
    let ray_o = rig.aim[g].translation;
    let ray_d = (rig.aim[g].rotation * Vec3::NEG_Z).normalize();
    let n = menu.at.rotation * Vec3::Z;
    let denom = ray_d.dot(n);
    let mut hit_uv: Option<Vec2> = None;
    let mut beam_len = 2.;
    if denom.abs() > 1e-4 {
        let t = (menu.at.translation - ray_o).dot(n) / denom;
        if t > 0. && t < 4. {
            let p = ray_o + ray_d * t;
            let local = menu.at.rotation.inverse() * (p - menu.at.translation) / PANEL_W;
            let (u, v) = (local.x + 0.5, 0.5 - local.y / (H / W));
            if (0. ..=1.).contains(&u) && (0. ..=1.).contains(&v) { hit_uv = Some(Vec2::new(u * W, v * H)); beam_len = t; }
        }
    }
    // the laser and its dot
    if let Ok(mut t) = tf.get_mut(menu.laser) { *t = Transform::from_translation(ray_o + ray_d * beam_len * 0.5).looking_to(ray_d, Vec3::Y).with_scale(Vec3::new(1., 1., beam_len)); }
    if let Ok(mut v) = vis.get_mut(menu.laser) { *v = Visibility::Visible; }
    if let Ok(mut t) = tf.get_mut(menu.dot) { t.translation = ray_o + ray_d * beam_len; }
    if let Ok(mut v) = vis.get_mut(menu.dot) { *v = if hit_uv.is_some() { Visibility::Visible } else { Visibility::Hidden }; }
    let count = rows_on(menu.page);
    let last_hover = menu.hover; menu.hover = hit_uv;
    // hovering picks the row
    let mut click_arrow: Option<(usize, i32)> = None;
    if let Some(at) = hit_uv {
        let moved = last_hover.is_none_or(|h| h.distance(at) > 1.5);
        if moved { if let Some(&(r, _)) = menu.rows.iter().find(|(_, rect)| rect.contains(at)) { if menu.sel != r { menu.sel = r; } } }
        click_arrow = menu.arrows.iter().find(|(_, _, rect)| rect.contains(at)).map(|(r, d, _)| (*r, *d));
    }
    // the sticks: up / down with a repeat, left / right on a setting
    let s = inp.hands[0].stick + inp.hands[1].stick;
    let dir: i8 = if s.y > 0.6 { -1 } else if s.y < -0.6 { 1 } else { 0 };
    let fire = dir != 0 && (dir != menu.repeat.0 || now - menu.repeat.1 > 0.22);
    if dir != menu.repeat.0 { menu.repeat = (dir, now + 0.25); } else if fire { menu.repeat.1 = now; }
    if fire { menu.sel = (menu.sel as i32 + dir as i32).rem_euclid(count as i32) as usize; }
    let hstep = |v: f32, last: &mut i8| -> i32 { let d: i8 = if v > 0.7 { 1 } else if v < -0.7 { -1 } else { 0 }; let out = if d != *last { d as i32 } else { 0 }; *last = d; out };
    static LAST_H: std::sync::Mutex<i8> = std::sync::Mutex::new(0);
    let side = { let mut l = LAST_H.lock().unwrap(); hstep(s.x, &mut l) };
    // keep the selection on screen
    if menu.sel < menu.top { menu.top = menu.sel; }
    if menu.sel >= menu.top + VISIBLE_ROWS { menu.top = menu.sel + 1 - VISIBLE_ROWS; }
    let trigger = inp.held(g, Btn::Trigger) || inp.held(1, Btn::Primary) || inp.held(0, Btn::Primary) || keys.pressed(KeyCode::Enter);
    let click = trigger && !menu.pressed;
    menu.pressed = trigger;
    let back = inp.just(1, Btn::Secondary) || inp.just(0, Btn::Secondary) || keys.just_pressed(KeyCode::Backspace);
    if back { if menu.page == Page::Main { close(&mut menu, &mut vtime); } else { menu.sel = match menu.page { Page::Settings => 1, Page::Controls => 2, Page::Stages => 5, Page::Main => 0 }; menu.page = Page::Main; menu.top = 0; } return; }
    let sel = menu.sel;
    match menu.page {
        Page::Main if click => match sel {
            0 => close(&mut menu, &mut vtime),
            1 => { menu.page = Page::Settings; menu.sel = 0; menu.top = 0; }
            2 => { menu.page = Page::Controls; menu.sel = 0; menu.top = 0; }
            3 => { close(&mut menu, &mut vtime); req.key = Some(KeyCode::Escape); }
            4 => { rig.started = false; rig.calib = None; rig.heights.clear(); rig.tracked_for = 0.; menu.notice = Some(("View recentered".into(), now)); close(&mut menu, &mut vtime); }
            5 => { menu.page = Page::Stages; menu.sel = 0; menu.top = 0; }
            6 => { exit.write(AppExit::Success); }
            _ => {}
        },
        Page::Settings => {
            let rows = super::settings::ROWS;
            let d = if let (true, Some((r, d))) = (click, click_arrow) { if r == sel { d } else { 0 } } else if click && sel < rows { 1 } else { side };
            if click && sel == rows { menu.page = Page::Main; menu.sel = 1; menu.top = 0; }
            else if d != 0 && sel < rows {
                let before = set.left_handed;
                set.adjust(sel, d); set.save();
                if set.left_handed != before { inp_mut.gun_drawn = false; }
            }
        }
        Page::Controls if click => { menu.page = Page::Main; menu.sel = 2; menu.top = 0; }
        Page::Stages if click => {
            let stages = &super::super::select::STAGES;
            if sel < stages.len() { super::super::select::relaunch(stages[sel].0); } else { menu.page = Page::Main; menu.sel = 5; menu.top = 0; }
        }
        _ => {}
    }
}

/// Update: the menu's page, redrawn when it changes.
fn draw(mut menu: ResMut<Menu>, set: Res<VrSettings>, assets: Res<AssetServer>, mut commands: Commands, children: Query<&Children>, mut vis: Query<&mut Visibility>) {
    let root = menu.root;
    if let Ok(mut v) = vis.get_mut(root) { *v = if menu.open { Visibility::Inherited } else { Visibility::Hidden }; }
    if !menu.open { return; }
    let key = format!("{:?}|{}|{}|{:?}", menu.page, menu.sel, menu.top, set);
    if menu.key == key { return; }
    menu.key = key;
    if let Ok(kids) = children.get(root) { for e in kids.iter() { commands.entity(e).despawn(); } }
    if menu.fonts.is_none() { menu.fonts = Some(super::super::editor::load_fonts()); }
    let fonts = match menu.fonts.take() { Some(Some(f)) => f, _ => { menu.fonts = Some(None); return; } };
    let (mut rows, mut arrows) = (vec![], vec![]);
    {
        let mut p = Painter { commands: &mut commands, assets: &assets, fonts: &fonts, k: SCALE, offset: Vec2::ZERO, layer: false, parent: Some(root) };
        let box_line = |p: &mut Painter, x: f32, y: f32, w: f32, h: f32, t: f32, z: i32| {
            p.rect(x, y, w, t, LINE, z); p.rect(x, y + h - t, w, t, LINE, z); p.rect(x, y, t, h, LINE, z); p.rect(x + w - t, y, t, h, LINE, z);
        };
        // the panel, its rules and the heading in the brush capitals
        p.rect(0., 0., W, H, PANEL, 100);
        box_line(&mut p, 6., 6., W - 12., H - 12., 1.4, 102);
        let heading = match menu.page { Page::Main => "VR MENU", Page::Settings => "VR SETTINGS", Page::Controls => "CONTROLS", Page::Stages => "STAGE SELECT" };
        p.brush(heading, 26., 18., 46., RED);
        p.text("MADWORLD  /  KREESE KREELY  /  VIRTUAL REALITY", 30., 70., 11., DIM, W - 60.);
        let bar_y = 88.;
        p.rect(20., bar_y, W - 40., 17., LINE, 103);
        let sub = match menu.page { Page::Main => "MENU", Page::Settings => "COMFORT / MOVEMENT / HANDS / HUD", Page::Controls => "TOUCH CONTROLLERS", Page::Stages => "STAGES" };
        let tw = p.width(sub, 13.);
        p.text(sub, (W - tw) * 0.5, bar_y + 1.5, 13., TITLE, W);
        let list_y = bar_y + 26.;
        let row_h = 25.;
        let mut help = String::new();
        match menu.page {
            Page::Main => {
                for (i, (label, about)) in MAIN.iter().enumerate() {
                    let y = list_y + i as f32 * row_h;
                    let r = Rect::new(20., y - 3., W - 20., y + row_h - 5.);
                    if i == menu.sel { p.rect(r.min.x, r.min.y, r.width(), r.height(), HILITE, 104); help = about.to_string(); }
                    p.text(label, 44., y, 17., Color::WHITE, W - 80.);
                    rows.push((i, r));
                }
            }
            Page::Settings => {
                let all = set.rows();
                let n = all.len() + 1;
                for i in menu.top..(menu.top + VISIBLE_ROWS).min(n) {
                    let y = list_y + (i - menu.top) as f32 * row_h;
                    let r = Rect::new(20., y - 3., W - 20., y + row_h - 5.);
                    if i == menu.sel { p.rect(r.min.x, r.min.y, r.width(), r.height(), HILITE, 104); }
                    if i == all.len() {
                        p.text("BACK", 44., y, 16., Color::WHITE, 200.);
                        if i == menu.sel { help = "Back to the menu (the settings are saved)".into(); }
                    } else {
                        let row = &all[i];
                        p.text(&row.label.to_uppercase(), 44., y + 1., 15., Color::WHITE, 300.);
                        // the value between its arrows, in a ruled chip
                        let (vx, vw) = (W - 250., 200.);
                        box_line(&mut p, vx, y - 1., vw, 19., 0.9, 105);
                        let w = p.width(&row.value, 14.);
                        p.text(&row.value, vx + (vw - w) * 0.5, y + 1., 14., Color::WHITE, vw);
                        p.text("<", vx - 18., y + 1., 14., if i == menu.sel { Color::WHITE } else { DIM }, 20.);
                        p.text(">", vx + vw + 6., y + 1., 14., if i == menu.sel { Color::WHITE } else { DIM }, 20.);
                        arrows.push((i, -1, Rect::new(vx - 26., y - 3., vx + 2., y + row_h - 5.)));
                        arrows.push((i, 1, Rect::new(vx + vw - 2., y - 3., vx + vw + 26., y + row_h - 5.)));
                        if i == menu.sel { help = row.help.to_string(); }
                    }
                    rows.push((i, r));
                }
                // the scroll marks
                if menu.top > 0 { p.text("+", W - 36., list_y - 16., 12., DIM, 20.); }
                if menu.top + VISIBLE_ROWS < n { p.text("+", W - 36., list_y + VISIBLE_ROWS as f32 * row_h - 6., 12., DIM, 20.); }
            }
            Page::Controls => {
                let col = |p: &mut Painter, x: f32, title: &str, items: &[(&str, &str)]| {
                    p.rect(x, list_y - 2., 280., 14., LINE, 103);
                    let tw = p.width(title, 10.);
                    p.text(title, x + (280. - tw) * 0.5, list_y - 0.5, 10., TITLE, 280.);
                    for (i, (k, what)) in items.iter().enumerate() {
                        let y = list_y + 20. + i as f32 * 21.;
                        let kw = p.width(k, 11.) + 10.;
                        box_line(p, x + 4., y - 2., kw, 16., 0.8, 105);
                        p.text(k, x + 9., y, 11., Color::WHITE, kw);
                        p.text(what, x + 14. + kw, y, 12., Color::WHITE, 270. - kw);
                    }
                };
                let gun = if set.left_handed { "LEFT" } else { "RIGHT" };
                col(&mut p, 24., "LEFT CONTROLLER", &[("STICK", "Move (push all the way: run)"), ("CLICK", "Run (when set to click)"), ("GRIP", "Grab / climb / hold"),
                    ("TRIGGER", "Point the finger"), ("X", "Reload the pistol"), ("Y", "Taunt"), ("MENU", "This menu")]);
                col(&mut p, 336., "RIGHT CONTROLLER", &[("STICK", "Turn (snap or smooth)"), ("GRIP", "Grab / climb / hold"), ("TRIGGER", "Fire the pistol"),
                    ("A", "Jump (and confirm)"), ("B", "Dodge (and back)"), ("SWING", "Punch (faster hits harder)"), ("SHAKE", "Mash a struggle / clash")]);
                let y = list_y + 20. + 7. * 21. + 6.;
                p.rect(20., y, W - 40., 1., LINE, 103);
                let tips = [format!("PISTOL: grip it at your {} hip, the trigger fires down the barrel; let go of the grip to holster it.", gun.to_lowercase()),
                            "GRAB: close a hand on a grunt or a townsperson to lift him; swing and let go to throw. Objects the same.".to_string(),
                            "CLIMB: grip a wall or a ledge and pull; over a ledge, let go (or push the stick) to vault on.".to_string()];
                for (i, t) in tips.iter().enumerate() { p.text(t, 30., y + 8. + i as f32 * 18., 11., Color::WHITE, W - 60.); }
                let r = Rect::new(20., H - 64., W - 20., H - 42.);
                p.rect(r.min.x, r.min.y, r.width(), r.height(), HILITE, 104);
                p.text("BACK", 44., H - 61., 15., Color::WHITE, 200.);
                rows.push((0, r));
                help = "Pull the trigger or press B to go back".into();
            }
            Page::Stages => {
                let stages = &super::super::select::STAGES;
                for i in 0..=stages.len() {
                    let y = list_y + i as f32 * row_h;
                    let r = Rect::new(20., y - 3., W - 20., y + row_h - 5.);
                    if i == menu.sel { p.rect(r.min.x, r.min.y, r.width(), r.height(), HILITE, 104); }
                    if i < stages.len() {
                        p.text(&stages[i].1.to_uppercase(), 44., y, 16., Color::WHITE, 300.);
                        if i == menu.sel { help = stages[i].2.to_string(); }
                    } else { p.text("BACK", 44., y, 16., Color::WHITE, 200.); if i == menu.sel { help = "Back to the menu".into(); } }
                    rows.push((i, r));
                }
            }
        }
        // the help line, and the buttons' legend
        p.rect(20., H - 36., W - 40., 1., LINE, 103);
        p.text(&help, 28., H - 31., 12., DIM, W - 56.);
        p.text("TRIGGER / A: SELECT     B: BACK     STICK: MOVE / CHANGE     MENU: CLOSE", 28., H - 17., 9., DIM, W - 56.);
    }
    menu.fonts = Some(Some(fonts));
    menu.rows = rows; menu.arrows = arrows;
}

/// Update: the panel where it was opened, the laser only while open; the desktop sees the menu too.
fn place(menu: Res<Menu>, rig: Res<Rig>, mut tf: Query<&mut Transform, Without<MenuDim>>, mut vis: Query<&mut Visibility, (Without<MirrorMenu>, Without<MenuDim>)>, mut mirror: Query<&mut Visibility, With<MirrorMenu>>,
         mut dim: Query<(&mut Transform, &mut Visibility), (With<MenuDim>, Without<MirrorMenu>)>) {
    for (mut t, mut v) in &mut dim { t.translation = rig.head.translation; *v = if menu.open { Visibility::Visible } else { Visibility::Hidden }; }
    if let Ok(mut t) = tf.get_mut(menu.panel) { *t = menu.at; }
    let v = if menu.open { Visibility::Visible } else { Visibility::Hidden };
    if let Ok(mut x) = vis.get_mut(menu.panel) { *x = v; }
    if !menu.open { for e in [menu.laser, menu.dot] { if let Ok(mut x) = vis.get_mut(e) { *x = Visibility::Hidden; } } }
    for mut m in &mut mirror { *m = v; }
    let _ = &menu.image;
}
