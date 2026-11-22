//! VR [added, not native]: Kreese Kreely in first person on a Quest 3 (Link, Air Link or Virtual Desktop) or any other
//! OpenXR headset, and a desktop VR simulator for playing / testing without one.
//!
//! `MW_VR=1` (or `quest`, `openxr`) starts the OpenXR runtime (bevy_mod_openxr, vendored in crates/vendor and ported to
//! Bevy 0.20); without a headset it falls back to the simulator view. `MW_VR=sim` is the simulator: mouse and keys
//! drive the head and both controllers (play_vr_sim.rs; `MW_VR_SCRIPT` replays a scripted session).
//!
//! How the fight is played in VR:
//! - The view is Kreely's head: the tracking space is anchored to the sim player, whose body follows the headset around
//!   the room (sliding along the collision) and moves with the left stick; the right stick snap- or smooth-turns.
//!   His body is hidden from the eyes (the flat camera still sees it); a height offset puts your eyes at his.
//! - Fists hit by their real speed (play_vr_interact.rs): grunts, townsfolk, RinRin and the bosses take the blows
//!   through their own damage paths. The grips grab: objects (the street's props) are carried in the hand and thrown
//!   with the hand's velocity; grunts and townsfolk are lifted by the collar and thrown the same way; RinRin and the
//!   bosses take Kreely's grab. Gripping a wall or a ledge climbs it, and pulling over a ledge vaults onto it.
//! - The pistol sits in a holster at the gun-hand hip: grip it there to draw, the trigger fires along the barrel.
//! - The HUD (gauge, ammo, prompts, the native pause pages) is drawn to a panel floating before you; event scenes, pair
//!   cameras and finishers play on a theater screen (VR settings).
//! - The left menu button opens the VR menu (play_vr_menu.rs) in the pause menu's own look: VR settings, the
//!   controls, the native pause, recentering, the stage select.
use bevy::{
    camera::{visibility::RenderLayers, RenderTarget},
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    image::ImageSampler,
    input::InputSystems,
    prelude::*,
    render::render_resource::TextureFormat,
    ui::IsDefaultUiCamera,
};
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "play_vr_settings.rs"] pub mod settings;
#[path = "play_vr_auto.rs"] pub mod auto;
#[path = "play_vr_sim.rs"] mod sim;
#[path = "play_vr_xr.rs"] mod xr;
#[path = "play_vr_hands.rs"] mod hands;
#[path = "play_vr_body.rs"] mod body;
#[path = "play_vr_struggle.rs"] pub mod struggle;
#[path = "play_vr_interact.rs"] pub mod interact;
#[path = "play_vr_menu.rs"] mod menu;
#[path = "play_vr_gun.rs"] pub mod gun;
pub use settings::VrSettings;

/// Render layers: the world (0); the theater (2: the screen and its dark room); what only the eyes see (3: the HUD
/// panel, the menu, the vignette); Kreely's own body (4: the flat camera only); the desktop compositor (7).
pub const L_WORLD: usize = 0;
pub const L_CINEMA: usize = 2;
pub const L_EYES: usize = 3;
pub const L_BODY: usize = 4;
pub const L_HUD_UI: usize = 6;
pub const L_MIRROR: usize = 7;
/// Kreely as you are him: the IK body with the head cut away (the eyes, and the desktop mirror while it is the head).
pub const L_SELF: usize = 8;
/// The flat (game) camera's and the HUD's render size.
pub const FLAT: UVec2 = UVec2::new(1280, 720);
/// Kreely's eye height above his feet: his eye bones (40 / 41) stand 1.72 m over the root of his 1.85 m bind pose.
pub const EYE_HEIGHT: f32 = 1.72;
/// The depth bias that draws the HUD, the menu and the pointer over the world (reverse-Z: toward the eye).
pub const OVERLAY_BIAS: f32 = 1.0e7;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode { Off, OpenXr, Sim }
pub fn mode() -> Mode {
    static M: std::sync::OnceLock<Mode> = std::sync::OnceLock::new();
    *M.get_or_init(|| match std::env::var("MW_VR").map(|v| v.to_ascii_lowercase()).as_deref() {
        Ok("1" | "on" | "xr" | "openxr" | "quest" | "true" | "yes") => Mode::OpenXr,
        Ok("sim" | "simulator" | "emulate" | "desktop") => Mode::Sim,
        _ => Mode::Off,
    })
}
/// The VR side is built in (MW_VR / a headset at launch): its plugins, cameras and systems exist.
pub fn built() -> bool { mode() != Mode::Off }
static ACTIVE: AtomicBool = AtomicBool::new(true);
/// Playing in VR right now. A headset build plays flat while the headset is away (play_vr_auto.rs switches it in place).
pub fn on() -> bool { built() && ACTIVE.load(Ordering::Relaxed) }
pub fn set_active(v: bool) { ACTIVE.store(v, Ordering::Relaxed); }

/// Every VR system that plays the game (the rig, the hands, the struggles, the menu) runs only while VR is on.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct VrActive;
/// The cameras the switch flips: the HUD's (to the panel image), the desktop compositor's; the game camera's own order.
#[derive(Component)] pub struct HudCam;
#[derive(Component)] pub struct MirrorCam;
#[derive(Component)] pub struct FlatOrder(pub isize);
/// What you hold in VR (the pistol): its meshes go on the self layer as they load (seen by the eyes, not the flat game).
#[derive(Component)] pub struct SelfView;
fn self_layer(roots: Query<Entity, With<SelfView>>, children: Query<&Children>, meshes: Query<(), (With<Mesh3d>, Without<RenderLayers>)>, mut commands: Commands) {
    for root in &roots {
        for e in children.iter_descendants(root) { if meshes.contains(e) { commands.entity(e).try_insert(RenderLayers::layer(L_SELF)); } }
    }
}

static MENU_OPEN: AtomicBool = AtomicBool::new(false);
/// The VR menu holds the fight (pause::running).
pub fn menu_open() -> bool { MENU_OPEN.load(Ordering::Relaxed) }
fn set_menu_open(v: bool) { MENU_OPEN.store(v, Ordering::Relaxed); }

/// One controller (tracking-space poses: the grip, where the hand holds, and the aim, along the pointing ray).
#[derive(Clone, Copy, Debug, Default)]
pub struct Hand {
    pub grip_pose: Transform, pub aim_pose: Transform, pub tracked: bool,
    /// Linear / angular velocity in tracking space (m/s, rad/s).
    pub vel: Vec3, pub ang_vel: Vec3,
    pub grip: f32, pub trigger: f32, pub stick: Vec2,
    pub stick_click: bool, pub primary: bool, pub secondary: bool, pub menu: bool,
    /// Capacitive touches (finger poses): the trigger, the thumb on a button / stick / rest.
    pub trigger_touch: bool, pub thumb_touch: bool,
}

impl Hand {
    /// Kreely's hand on this controller, in tracking space: at the grip (the palm), turned like the controller's
    /// pointing ray (the aim pose: -Z forward, +Y up), with the hand-fit setting applied. (OpenXR's grip pose itself
    /// runs its -Z up the handle, through the fist toward the thumb: a hand turned by it points at the sky.)
    pub fn hand_pose(&self, fit: &Transform) -> Transform {
        let g = self.grip_pose * *fit;
        let a = self.aim_pose * *fit;
        Transform { translation: g.translation, rotation: a.rotation, scale: Vec3::ONE }
    }
}

/// Both controllers and the head this frame, from OpenXR or the simulator.
#[derive(Resource, Default)]
pub struct VrInput {
    pub head: Transform, pub head_tracked: bool,
    pub hands: [Hand; 2],
    /// Last frame's buttons, for edges: (primary, secondary, menu, stick click, trigger > 0.7).
    prev: [[bool; 5]; 2],
    /// Rumble requests (amplitude 0..1, seconds), drained by the backend.
    pub rumble: [Option<(f32, f32)>; 2],
    /// The live headset session is running (false: the simulator drives this).
    pub live: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Btn { Primary, Secondary, Menu, StickClick, Trigger }
impl VrInput {
    fn now(h: &Hand) -> [bool; 5] { [h.primary, h.secondary, h.menu, h.stick_click, h.trigger > 0.7] }
    pub fn held(&self, hand: usize, b: Btn) -> bool { Self::now(&self.hands[hand])[b as usize] }
    pub fn just(&self, hand: usize, b: Btn) -> bool { Self::now(&self.hands[hand])[b as usize] && !self.prev[hand][b as usize] }
    pub fn rumble(&mut self, hand: usize, amp: f32, secs: f32) {
        let r = &mut self.rumble[hand];
        if r.is_none_or(|(a, _)| a < amp) { *r = Some((amp, secs)); }
    }
}

/// The tracking space in the world: its yaw, and its floor origin at the last two fixed steps (interpolated per frame
/// as the player's body is). `root` is this frame's.
#[derive(Resource)]
pub struct Rig {
    pub yaw: f32, pub prev: Vec3, pub cur: Vec3, pub root: Transform,
    /// Height added to the tracking floor (Kreely's eyes over yours); the standing / seated head height it was measured from.
    pub offset: f32, pub calib: Option<f32>, pub started: bool,
    /// Extra height in a paired clash: the eyes ride Kreely's animated eyes (on the floor, kneeling, bent), eased.
    pub lift: f32,
    /// This frame's world poses and velocities.
    pub head: Transform, pub grip: [Transform; 2], pub aim: [Transform; 2], pub vel: [Vec3; 2],
    /// The body position at the start of the fixed step (its own movement is measured against it).
    pub body_before: Vec3,
    /// Vignette strength (0..1), smoothed; the right stick's snap latch; Run::Click's run toggle.
    pub vignette: f32, pub snap_latch: bool, pub running: bool,
    /// Moving or turning this frame (drives the vignette).
    pub motion: f32,
    /// Each hand's fastest tracking-space velocity since the last game frame (a swing between two 30 Hz frames counts).
    pub peak: [Vec3; 2],
    /// Seconds the headset has been tracked (the height estimate waits out the first, while it goes on).
    pub tracked_for: f32,
    /// Head heights sampled 10 times a second over the last 12 s (the standing height is their 70th percentile).
    pub heights: std::collections::VecDeque<f32>, pub sample_t: f32,
}
impl Default for Rig {
    fn default() -> Self {
        Self { yaw: 0., prev: Vec3::ZERO, cur: Vec3::ZERO, root: Transform::IDENTITY, offset: 0., lift: 0., calib: None, started: false,
               head: Transform::IDENTITY, grip: [Transform::IDENTITY; 2], aim: [Transform::IDENTITY; 2], vel: [Vec3::ZERO; 2],
               body_before: Vec3::ZERO, vignette: 0., snap_latch: false, running: false, motion: 0., peak: [Vec3::ZERO; 2], tracked_for: 0., heights: Default::default(), sample_t: 0. }
    }
}
impl Rig {
    pub fn rot(&self) -> Quat { Quat::from_rotation_y(self.yaw) }
    /// A tracking-space pose in the world, with the root at the fixed step's position (`cur`).
    pub fn world_at(&self, root: Vec3, local: &Transform) -> Transform {
        Transform { translation: root + Vec3::Y * (self.offset + self.lift) + self.rot() * local.translation, rotation: self.rot() * local.rotation, scale: Vec3::ONE }
    }
    pub fn to_world(&self, local: &Transform) -> Transform { self.root * *local }
    /// Turn the space by `d` radians around the head.
    pub fn turn(&mut self, d: f32) {
        let pivot = self.head.translation;
        let r = Quat::from_rotation_y(d);
        for p in [&mut self.prev, &mut self.cur] {
            let v = *p - pivot;
            let v = r * Vec3::new(v.x, 0., v.z);
            *p = Vec3::new(pivot.x + v.x, p.y, pivot.z + v.z);
        }
        self.yaw += d;
        self.motion = self.motion.max(1.);
    }
}

/// The game convention's yaw (0 = +Z, the sim's facing) of a rotation's forward.
pub fn game_yaw(q: Quat) -> f32 { let f = q * Vec3::NEG_Z; f.x.atan2(f.z) }

/// What the eyes show: the world from the head, or the theater (an event, a pair camera, a finisher).
#[derive(Resource, Default)]
pub struct View { pub cinema: bool, pub cinema_since: f64 }

/// The images the flat camera and the HUD draw to.
#[derive(Resource, Clone)]
#[allow(dead_code)]
pub struct Targets { pub flat: Handle<Image>, pub hud: Handle<Image> }

#[derive(Component)] pub struct VrEye;
#[derive(Component)] pub struct SimRoot;
#[derive(Component)] struct HudPanel;
#[derive(Component)] struct CinemaScreen;
#[derive(Component)] struct CinemaHud;
#[derive(Component)] struct CinemaRoom;
#[derive(Component)] struct MirrorSprite;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct VrInputSet;

pub struct VrPlugin;
impl Plugin for VrPlugin {
    fn build(&self, app: &mut App) {
        if !built() { return; }
        eprintln!("[vr] {:?} mode (MW_VR); settings {}", mode(), settings::path().display());
        // a deferred command on an entity something else despawned the same frame (the challenge's hand-off to the intro
        // swaps actors) is logged, not fatal
        app.set_error_handler(bevy::ecs::error::warn);
        app.configure_sets(PreUpdate, VrActive.run_if(on)).configure_sets(Update, VrActive.run_if(on)).configure_sets(PostUpdate, VrActive.run_if(on)).configure_sets(FixedUpdate, VrActive.run_if(on));
        app.insert_resource(VrSettings::load()).init_resource::<VrInput>().init_resource::<Rig>().init_resource::<View>()
            .init_resource::<interact::Interact>()
            .add_systems(Startup, setup.after(super::setup))
            .add_systems(PreUpdate, (end_input_frame, sim::read.run_if(|i: Res<VrInput>| !i.live), xr::read_buttons, inject_keys).chain()
                .in_set(VrInputSet).in_set(VrActive).after(InputSystems))
            .add_systems(Update, (xr::read_poses, tag_cameras, tag_body, struggle::detect, view, rig, pad, struggle::drive).chain().in_set(VrActive).after(super::input).before(super::pose))
            .add_systems(Update, (place_views.after(super::camera), hands::present.after(super::pose), panels.after(place_views), mirror_size).in_set(VrActive))
            .add_systems(PostUpdate, xr::rumble.in_set(VrActive))
            .add_systems(FixedUpdate, interact::before_tick.in_set(VrActive).before(super::tick))
            .add_systems(FixedUpdate, interact::step.in_set(VrActive).after(super::hurt::step).run_if(super::pause::running).run_if(super::intro::fight_running).run_if(super::cutin::world_running));
        app.add_systems(PreUpdate, apply_mode.before(VrInputSet).after(InputSystems)).add_systems(Update, (switch_shots, self_layer, cam_debug, growth_log));
        hands::build(app);
        body::build(app);
        struggle::build(app);
        menu::build(app);
        gun::build(app);
        sim::build(app);
        xr::build(app);
    }
}

/// The plugins: OpenXR in place of Bevy's own renderer creation (MW_VR=1), else the given set as is.
pub fn plugins(app: &mut App, base: bevy::app::PluginGroupBuilder) {
    if mode() == Mode::OpenXr {
        use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
        // the eye size the swapchain is made at (the vendored OpenXR plugin reads it): the VR setting, or the variable
        if std::env::var_os("MW_VR_RENDER_SCALE").is_none() { std::env::set_var("MW_VR_RENDER_SCALE", VrSettings::load().render_scale.to_string()); }
        let mut init = bevy_mod_openxr::init::OxrInitPlugin::default();
        init.app_info = bevy_mod_openxr::types::AppInfo { name: "MadWorld VR".into(), version: bevy_mod_openxr::types::Version(0, 1, 0) };
        let group = bevy_mod_openxr::add_xr_plugins(base.disable::<PipelinedRenderingPlugin>()).set(init);
        app.add_plugins(group);
    } else {
        app.add_plugins(base);
    }
}

/// Startup: the flat camera and the HUD draw to images; the eyes (simulator) or the desktop compositor (headset); the
/// panels; the simulator's tracking root.
fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>,
         mut game_cam: Query<(Entity, &mut Camera), (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>) {
    let target = |images: &mut Assets<Image>| {
        let mut i = Image::new_target_texture(FLAT.x, FLAT.y, TextureFormat::Rgba8UnormSrgb, None);
        i.sampler = ImageSampler::linear();
        images.add(i)
    };
    let (flat, hud) = (target(&mut images), target(&mut images));
    commands.insert_resource(Targets { flat: flat.clone(), hud: hud.clone() });
    // the game's own camera (the native follow / event cameras; at the head outside the theater) draws the flat view
    for (e, mut cam) in &mut game_cam {
        commands.entity(e).insert(FlatOrder(cam.order));
        cam.order = -3;
        commands.entity(e).insert((RenderTarget::from(flat.clone()), RenderLayers::from_layers(&[L_WORLD, L_BODY])));
    }
    // every UI node without a camera of its own goes to the HUD image (transparent where nothing is drawn)
    commands.spawn((HudCam, Camera2d, Camera { order: -2, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() }, RenderTarget::from(hud.clone()),
                    IsDefaultUiCamera, RenderLayers::layer(L_HUD_UI), Msaa::Off, Tonemapping::None, DebandDither::Disabled));
    match mode() {
        Mode::Sim => {
            // the simulator's eye: what the headset would show, in the window (mono)
            let root = commands.spawn((SimRoot, Transform::default(), Visibility::default())).id();
            commands.spawn((VrEye, Camera3d::default(), Camera { order: 0, ..default() }, Projection::Perspective(PerspectiveProjection { fov: 80f32.to_radians(), near: 0.05, ..default() }),
                            RenderLayers::from_layers(&[L_WORLD, L_EYES, L_SELF]), Tonemapping::None, DebandDither::Disabled, Transform::default(), ChildOf(root)));
        }
        Mode::OpenXr => {
            // the desktop window: the flat view with the HUD over it
            commands.spawn((MirrorCam, Camera2d, Camera { order: 10, ..default() }, RenderLayers::layer(L_MIRROR), Msaa::Off));
            commands.spawn((MirrorSprite, Sprite { image: flat.clone(), custom_size: Some(FLAT.as_vec2()), ..default() }, Transform::from_xyz(0., 0., 0.), RenderLayers::layer(L_MIRROR)));
            commands.spawn((MirrorSprite, Sprite { image: hud.clone(), custom_size: Some(FLAT.as_vec2()), ..default() }, Transform::from_xyz(0., 0., 1.), RenderLayers::layer(L_MIRROR)));
        }
        Mode::Off => {}
    }
    // the HUD panel (eyes only), the theater screen with its HUD and its room
    let unlit = |materials: &mut Assets<StandardMaterial>, image: Handle<Image>, blend: bool| materials.add(StandardMaterial {
        base_color_texture: Some(image), unlit: true, cull_mode: None, alpha_mode: if blend { AlphaMode::Blend } else { AlphaMode::Opaque },
        fog_enabled: false, depth_bias: if blend { OVERLAY_BIAS } else { 0. }, ..default() });
    let quad = meshes.add(Rectangle::new(1., FLAT.y as f32 / FLAT.x as f32));
    commands.spawn((HudPanel, Mesh3d(quad.clone()), MeshMaterial3d(unlit(&mut materials, hud.clone(), true)), Transform::default(), Visibility::Hidden,
                    RenderLayers::layer(L_EYES), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
    commands.spawn((CinemaScreen, Mesh3d(quad.clone()), MeshMaterial3d(unlit(&mut materials, flat.clone(), false)), Transform::default(), Visibility::Hidden,
                    RenderLayers::layer(L_CINEMA), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
    commands.spawn((CinemaHud, Mesh3d(quad), MeshMaterial3d(unlit(&mut materials, hud, true)), Transform::default(), Visibility::Hidden,
                    RenderLayers::layer(L_CINEMA), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
    // the theater's room: a dark floor disc and a dim dome, so the screen does not hang in nothing
    let dark = materials.add(StandardMaterial { base_color: Color::srgb(0.035, 0.03, 0.04), unlit: true, cull_mode: None, fog_enabled: false, ..default() });
    let floor = materials.add(StandardMaterial { base_color: Color::srgb(0.09, 0.07, 0.07), unlit: true, fog_enabled: false, ..default() });
    commands.spawn((CinemaRoom, Mesh3d(meshes.add(Sphere::new(14.).mesh().uv(24, 12))), MeshMaterial3d(dark), Transform::default(), Visibility::Hidden,
                    RenderLayers::layer(L_CINEMA), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
    commands.spawn((CinemaRoom, Mesh3d(meshes.add(Circle::new(6.))), MeshMaterial3d(floor), Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                    Visibility::Hidden, RenderLayers::layer(L_CINEMA), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
}

/// The headset came or went (play_vr_auto.rs flips `on`): the cameras switch between the headset's layout (the game
/// camera and the HUD drawn to images for the eyes and the desktop mirror) and the flat game's (straight to the window).
fn apply_mode(mut last: Local<Option<bool>>, mut commands: Commands, targets: Option<Res<Targets>>, mut rig: ResMut<Rig>, mut inp: ResMut<VrInput>,
              mut keys: ResMut<ButtonInput<KeyCode>>, mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
              mut game: Query<(Entity, &mut Camera, &FlatOrder), (Without<HudCam>, Without<MirrorCam>)>,
              mut hud: Query<(Entity, &mut Camera), (With<HudCam>, Without<MirrorCam>, Without<FlatOrder>)>,
              mut mirror: Query<&mut Camera, (With<MirrorCam>, Without<HudCam>, Without<FlatOrder>)>) {
    let on = on();
    // (the cameras start in the headset's layout, whatever `on` already says on the first frame)
    let was = last.replace(on).unwrap_or(true);
    if was == on { return; }
    let Some(t) = targets else { return; };
    for (e, mut c, o) in &mut game {
        if on { c.order = -3; commands.entity(e).remove::<IsDefaultUiCamera>().insert(RenderTarget::from(t.flat.clone())); }
        else { c.order = o.0; c.is_active = true; commands.entity(e).insert((RenderTarget::Window(bevy::window::WindowRef::Primary), IsDefaultUiCamera, RenderLayers::from_layers(&[L_WORLD, L_BODY]))); }
    }
    for (e, mut c) in &mut hud { c.is_active = on; if on { commands.entity(e).insert(IsDefaultUiCamera); } else { commands.entity(e).remove::<IsDefaultUiCamera>(); } }
    for mut c in &mut mirror { c.is_active = on; }
    // in the headset the desktop window must not wait for the monitor's refresh (it would hold the headset to it);
    // on the monitor alone it syncs as usual
    for mut w in &mut windows { w.present_mode = if on { bevy::window::PresentMode::AutoNoVsync } else { bevy::window::PresentMode::AutoVsync }; }
    // keys the controllers held down are let go; back in VR the space anchors afresh on Kreely
    keys.release_all();
    let live = inp.live;
    *inp = VrInput { live, ..default() };
    if on { rig.started = false; }
    eprintln!("[vr] switched to {}", if on { "VR" } else { "flat" });
}

/// Testing: `MW_VR_GROWTH_LOG` prints, every 10 s, the frame rate over those 10 s and what the world holds (entities,
/// meshes, materials, images): a count that only climbs is a leak.
fn growth_log(world: &World, time: Res<Time<Real>>, mut last: Local<(f64, u64)>, mut frames: Local<u64>, meshes: Res<Assets<Mesh>>, mats: Res<Assets<StandardMaterial>>, images: Res<Assets<Image>>) {
    if std::env::var_os("MW_VR_GROWTH_LOG").is_none() { return; }
    *frames += 1;
    let now = time.elapsed_secs_f64();
    if now - last.0 < 10. { return; }
    let fps = (*frames - last.1) as f64 / (now - last.0).max(1e-3);
    *last = (now, *frames);
    eprintln!("[growth] t {now:.0}s fps {fps:.0} entities {} meshes {} materials {} images {}", world.entities().len(), meshes.len(), mats.len(), images.len());
}

/// Testing: `MW_VR_CAM_LOG` lists the cameras once, 300 frames in.
fn cam_debug(mut n: Local<u32>, cams: Query<(Entity, &Camera, Option<&RenderTarget>, Has<IsDefaultUiCamera>, Has<Camera3d>, Option<&RenderLayers>)>, nodes: Query<(), With<Node>>) {
    *n += 1;
    if *n != 300 || std::env::var_os("MW_VR_CAM_LOG").is_none() { return; }
    for (e, c, t, ui, three, l) in &cams { eprintln!("[cam] {e:?} order {} active {} target {:?} default_ui {ui} 3d {three} layers {:?}", c.order, c.is_active, t, l); }
    eprintln!("[cam] ui nodes: {}", nodes.iter().count());
}

/// Testing: `MW_VR_SWITCH_SHOTS=<prefix>` saves the window 60 frames after each switch (and 60 frames in).
fn switch_shots(mut last: Local<Option<bool>>, mut wait: Local<u32>, mut n: Local<u32>, mut commands: Commands) {
    let Ok(prefix) = std::env::var("MW_VR_SWITCH_SHOTS") else { return; };
    let on = on();
    if *last != Some(on) { *last = Some(on); *wait = 60; }
    if *wait > 0 { *wait -= 1; if *wait == 0 {
        *n += 1;
        let file = format!("{prefix}{}_{}.png", *n, if on { "vr" } else { "flat" });
        commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(file));
    } }
}

/// PreUpdate, first: last frame's buttons become the edge reference.
fn end_input_frame(mut inp: ResMut<VrInput>) {
    for h in 0..2 { inp.prev[h] = VrInput::now(&inp.hands[h]); }
}

/// The OpenXR eyes (spawned when the session starts) see the world and the eye layer; the theater switches them.
fn tag_cameras(mut commands: Commands, fresh: Query<Entity, Added<bevy_mod_xr::camera::XrCamera>>) {
    for e in &fresh {
        // (2x MSAA per eye: at a headset's eye size the flat game's 4x costs more than it shows)
        commands.entity(e).try_insert((VrEye, RenderLayers::from_layers(&[L_WORLD, L_EYES, L_SELF]), Tonemapping::None, DebandDither::Disabled, Msaa::Sample2));
        eprintln!("[vr] headset eye camera ready");
    }
}

/// Kreely's (and Jack's) own meshes go on the body layer: the eyes do not see him, the flat camera does.
fn tag_body(chars: Query<(Entity, &super::Character)>, children: Query<&Children>, meshes: Query<(), (With<Mesh3d>, Without<RenderLayers>)>, mut commands: Commands) {
    for (root, ch) in &chars {
        if ch.who == super::Who::Rin { continue; }
        for e in children.iter_descendants(root) { if meshes.contains(e) { commands.entity(e).try_insert(RenderLayers::layer(L_BODY)); } }
    }
}

/// Whether the native camera owns the view (the event, pair, finisher, broadcast, defeat and town cut-scene cameras).
fn view(mut v: ResMut<View>, set: Res<VrSettings>, play: Res<super::Play>, intro: Res<super::intro::Intro>, tv: Res<super::broadcast::Broadcast>,
        elise: Res<super::elise::Elise>, baron: Res<super::baron::Baron>, defeat: Res<super::DefeatCam>, execution: Res<super::execution::Execution>,
        real: Res<Time<Real>>, mut eyes: Query<&mut RenderLayers, With<VrEye>>, st: Res<struggle::Struggle>) {
    let native = native_scene(&play, &intro, &tv, &elise, &baron, &defeat, &execution);
    // (a push or a swing step is played in your own eyes, against her)
    let cinema = set.cinema && native && !struggle::first_person(&st);
    if cinema != v.cinema { v.cinema = cinema; v.cinema_since = real.elapsed_secs_f64(); eprintln!("[vr] view: {}", if cinema { "theater" } else { "head" }); }
    let want = if cinema { RenderLayers::from_layers(&[L_CINEMA, L_EYES]) } else { RenderLayers::from_layers(&[L_WORLD, L_EYES, L_SELF]) };
    for mut l in &mut eyes { if *l != want { *l = want.clone(); } }
}

/// Whether the native camera owns the flat view (the event, pair, finisher, broadcast, defeat and town cut-scene cameras).
pub fn native_scene(play: &super::Play, intro: &super::intro::Intro, tv: &super::broadcast::Broadcast, elise: &super::elise::Elise,
                    baron: &super::baron::Baron, defeat: &super::DefeatCam, execution: &super::execution::Execution) -> bool {
    super::town::camera().is_some() || super::rin_pair_camera(play).is_some() || elise.camera(play).is_some() || baron.camera(play).is_some()
        || baron.cine_camera(play).is_some() || tv.camera().is_some() || intro.camera().is_some() || defeat.since.is_some() || execution.hides_hud()
}

/// Update: the tracking space follows the body (interpolated like the character), turns with the right stick, and
/// its height puts the eyes at Kreely's; the world poses of the head and hands for everything after.
fn rig(mut rig: ResMut<Rig>, inp: Res<VrInput>, set: Res<VrSettings>, play: Res<super::Play>, fixed: Res<Time<Fixed>>, time: Res<Time<Real>>,
       view: Res<View>, mut roots: Query<&mut Transform, Or<(With<bevy_mod_xr::session::XrTrackingRoot>, With<SimRoot>)>>, st: Res<struggle::Struggle>) {
    let body = Vec3::from_array(play.fight.player.pos);
    if !rig.started && inp.head_tracked {
        // first sight: face where Kreely faces, the head over his body
        rig.yaw = play.fight.player.loco.facing - game_yaw(inp.head.rotation);
        let h = rig.rot() * inp.head.translation;
        rig.cur = Vec3::new(body.x - h.x, body.y, body.z - h.z); rig.prev = rig.cur;
        rig.started = true;
        eprintln!("[vr] tracking anchored at {:.2} yaw {:.0} deg", rig.cur, rig.yaw.to_degrees());
    }
    // height: calibrate the standing (or seated) head height once it is tracked, then hold Kreely's eye height
    // (not the first reading — which may come while the headset is still going on, or held up — but how you usually
    // stand: the 70th percentile of your head's height over the last 12 s, so a moment held high (under ~3.5 s) or a
    // crouch (under ~8 s) does not move it, and a wrong start settles within seconds)
    if inp.head_tracked && inp.head.translation.y > 0.4 {
        if rig.tracked_for < 1. { rig.tracked_for += time.delta_secs(); }
        else {
            rig.sample_t += time.delta_secs();
            if rig.sample_t >= 0.1 || rig.heights.is_empty() {
                rig.sample_t = 0.;
                rig.heights.push_back(inp.head.translation.y);
                if rig.heights.len() > 120 { rig.heights.pop_front(); }
                let mut v: Vec<f32> = rig.heights.iter().copied().collect();
                v.sort_by(f32::total_cmp);
                rig.calib = Some(v[((v.len() - 1) as f32 * 0.7) as usize]);
                if std::env::var_os("MW_VR_LOG").is_some() && rig.heights.len() % 20 == 0 { eprintln!("[vr] standing height {:.2} (head now {:.2}, {} samples)", rig.calib.unwrap_or(0.), inp.head.translation.y, rig.heights.len()); }
            }
        }
    }
    // (seated, your own height would put Kreely's eyes at his chest: seated play always stands you at his height)
    rig.offset = match (set.height, rig.calib) {
        (settings::Height::Character, Some(h)) => EYE_HEIGHT - h,
        (settings::Height::Real, Some(h)) if set.seated => EYE_HEIGHT - h,
        _ => 0. };
    // turning (right stick x), not in menus
    let dt = time.delta_secs();
    rig.motion = 0.;
    if !menu_open() && !view.cinema {
        let x = inp.hands[1].stick.x;
        match set.turn {
            settings::Turn::Snap => {
                if x.abs() > 0.7 && !rig.snap_latch { rig.snap_latch = true; let d = -x.signum() * set.snap_deg.to_radians(); rig.turn(d); }
                if x.abs() < 0.35 { rig.snap_latch = false; }
            }
            settings::Turn::Smooth => {
                let x = if x.abs() < 0.15 { 0. } else { (x - 0.15 * x.signum()) / 0.85 };
                if x != 0. { rig.turn(-x * set.smooth_deg.to_radians() * dt); rig.motion = rig.motion.max(x.abs()); }
            }
        }
    }
    // a struggle in a paired scene: the view rides his animated eyes (the clips carry him off the sim's origin)
    if let (Some((eye, _)), true) = (st.anchor, st.engaged || st.scene) {
        let head = rig.world_at(rig.cur, &inp.head).translation;
        let k = (time.delta_secs() * 8.).min(1.);
        let d = Vec3::new(eye.x - head.x, 0., eye.z - head.z) * k;
        rig.cur += d; rig.prev += d;
        // and his height: down on the floor, kneeling, bent over
        let want = eye.y - (rig.cur.y + rig.offset + inp.head.translation.y);
        rig.lift += (want - rig.lift) * k;
    } else { rig.lift *= 1. - (time.delta_secs() * 6.).min(1.); }
    let a = fixed.overstep_fraction();
    let root = rig.prev.lerp(rig.cur, a) + Vec3::Y * (rig.offset + rig.lift);
    rig.root = Transform { translation: root, rotation: rig.rot(), scale: Vec3::ONE };
    for mut t in &mut roots { *t = rig.root; }
    rig.head = rig.to_world(&inp.head);
    for h in 0..2 {
        // the hand fit (VR settings) moves and tilts the hand on the controller
        let fit = Transform { translation: Vec3::new(0., set.hand_up, -set.hand_fwd), rotation: Quat::from_rotation_x(set.hand_tilt.to_radians()), scale: Vec3::ONE };
        rig.grip[h] = rig.to_world(&inp.hands[h].hand_pose(&fit));
        rig.aim[h] = rig.to_world(&(inp.hands[h].aim_pose * fit));
        rig.vel[h] = rig.rot() * inp.hands[h].vel;
        if inp.hands[h].vel.length() > rig.peak[h].length() { rig.peak[h] = inp.hands[h].vel; }
    }
    let moving = play.fight.player.loco.state == 1 || play.fight.player.loco.state == 2;
    if moving { rig.motion = rig.motion.max(if play.fight.player.loco.state == 2 { 1. } else { 0.6 }); }
    let want = rig.motion * [0., 0.55, 1.][set.vignette.min(2) as usize];
    let k = (dt * if want > rig.vignette { 8. } else { 3. }).min(1.);
    rig.vignette += (want - rig.vignette) * k;
}

/// Update, after the keyboard input: the controllers as the fight's pad (the stick relative to the head or the left
/// hand, the face buttons, the drawn pistol's aim and trigger, shaking for the mash meters, swings for the clash).
fn pad(mut play: ResMut<super::Play>, inp: Res<VrInput>, mut rig: ResMut<Rig>, set: Res<VrSettings>, pause: Res<super::pause::Pause>,
       intro: Res<super::intro::Intro>, view: Res<View>, mut act: ResMut<interact::Interact>, time: Res<Time<Real>>, st: Res<struggle::Struggle>,
       mut pistol: ResMut<gun::Pistol>) {
    let blocked = intro.controls_blocked() || super::town::locked() || pause.open || pause.release_inputs || menu_open();
    let p = &mut *play;
    if blocked { p.latched = super::Pad::default(); p.vr_aim = None; return; }
    let l = &inp.hands[0];
    // movement: dead zone, then the walk / run by the setting
    let raw = l.stick;
    let mag = raw.length();
    let stick = if mag < 0.18 { Vec2::ZERO } else {
        let dir = raw / mag;
        let m = ((mag - 0.18) / 0.82).min(1.);
        if inp.just(0, Btn::StickClick) { rig.running = !rig.running; }
        let run = match set.run { settings::Run::Push => m > 0.92, settings::Run::Click => rig.running, settings::Run::Never => false };
        dir * if run { 1. } else { (m * set.walk_speed).max(0.3) }
    };
    if stick == Vec2::ZERO { rig.running = false; }
    let reference = match set.move_dir { settings::MoveDir::Head => rig.head.rotation, settings::MoveDir::Hand => rig.aim[0].rotation };
    p.latched.stick = stick;
    p.cam_yaw = game_yaw(reference);
    p.cam_pitch = 0.;
    // the face buttons: A jump, B dodge (right); X reload, Y taunt (left). Holding the pistol (realistic handling) its
    // hand's buttons are the magazine release (A / X) and the slide release (B / Y), and X does not reload
    let manual = set.gun_manual;
    let gun_hand = (act.gun_drawn && !view.cinema).then_some(act.gun_hand);
    let gun_buttons = |h: usize| manual && gun_hand == Some(h);
    if gun_buttons(1) || gun_buttons(0) {
        let gh = act.gun_hand;
        if inp.just(gh, Btn::Primary) { pistol.eject_req = true; }
        if inp.just(gh, Btn::Secondary) { pistol.release_req = true; }
    }
    if !gun_buttons(1) && inp.just(1, Btn::Primary) { p.latched.jump = true; }
    if !gun_buttons(1) && inp.just(1, Btn::Secondary) { p.latched.dodge = true; }
    if !manual && inp.just(0, Btn::Primary) { p.latched.reload = true; }
    if !gun_buttons(0) && inp.just(0, Btn::Secondary) { p.latched.taunt = true; }
    if p.fight.player.state() == 95 && (inp.just(1, Btn::Primary) || inp.just(1, Btn::Trigger)) { p.latched.cont = true; }
    // the pistol: in the gun hand it aims (the sim's aim state) and its trigger fires, along the barrel
    hands::set_gun_pitch(set.gun_pitch);
    let g = act.gun_hand;
    let drawn = act.gun_drawn && !view.cinema;
    p.latched.secondary = drawn;
    if drawn && !act.gun_was_drawn { p.latched.secondary_pressed = true; }
    act.gun_was_drawn = drawn;
    if drawn {
        // (realistic: only a chambered round fires; an empty chamber clicks)
        if inp.just(g, Btn::Trigger) { if !manual || pistol.ready() { p.latched.punch = true; } else { pistol.dry_req = true; } }
        p.latched.punch_held = inp.held(g, Btn::Trigger) && (!manual || pistol.ready());
        // the round leaves along the barrel as it is this moment: zeroed to the sights, lifted by the recoil still in
        // the gun (fast shots climb; a support hand holds it down)
        let root = gun::root_in_hand(&hands::gun_frame(&rig.grip[g], &rig.aim[g]), gun::flip(&pistol, p.ammo.recoil));
        p.vr_aim = Some(gun::barrel(&root));
    } else { p.latched.punch_held = false; p.vr_aim = None; }
    // shaking the controllers (a fast reversal of either hand) mashes the shake meters; in a clash or a struggle the face
    // buttons mash too (the struggles' own gestures: play_vr_struggle.rs)
    let now = time.elapsed_secs();
    if st.kind.is_some_and(|k| !matches!(k, struggle::Kind::Dizzy | struggle::Kind::Spin)) { return; }
    for h in 0..2 {
        let v = rig.vel[h];
        if v.length() > 1.4 && act.shake_dir[h].dot(v.normalize()) < -0.3 && now - act.shake_at[h] < 0.35 { p.latched.mash = true; }
        if v.length() > 1.4 { if act.shake_dir[h].dot(v.normalize()) < 0.5 { act.shake_at[h] = now; } act.shake_dir[h] = v.normalize(); }
    }
    if p.fight.paired && (0..2).any(|h| inp.just(h, Btn::Primary) || inp.just(h, Btn::Secondary)) { p.latched.mash = true; }
    // a big swing in a clash: down (the F swing) or across (the Q swing)
    if p.fight.paired || p.boss.is_some_and(|b| b.0 > 0) && p.fight.carry.is_none() {
        for h in 0..2 {
            let v = rig.vel[h];
            if v.length() > 3.6 && now - act.swing_at[h] > 0.5 {
                act.swing_at[h] = now;
                if p.fight.paired { if -v.y > Vec2::new(v.x, v.z).length() { p.latched.grab = true; } else { p.latched.swing_h = true; } }
            }
        }
    }
}

/// PreUpdate, before the pause menu reads them: in menus, prompts, the intro and the continue window the controllers
/// press the keys the native pages answer (sticks: arrows; A / trigger: Enter; B: Backspace; menu: Escape); in the fight
/// the left menu button opens the VR menu.
fn inject_keys(mut keys: ResMut<ButtonInput<KeyCode>>, inp: Res<VrInput>, pause: Res<super::pause::Pause>, intro: Res<super::intro::Intro>, play: Res<super::Play>,
               time: Res<Time<Real>>, mut held: Local<Vec<KeyCode>>, mut repeat: Local<[(i8, f64); 2]>, mut menu_req: ResMut<menu::MenuRequest>) {
    // (keys held across frames stay pressed: the intro's hold-to-skip times a continuous hold)
    let before = std::mem::take(&mut *held);
    let finish = |keys: &mut ButtonInput<KeyCode>, now: &Vec<KeyCode>| { for k in &before { if !now.contains(k) { keys.release(*k); } } };
    if menu_open() { finish(&mut keys, &held); return; }
    let dead = play.fight.player.state() == 95;
    let ui = pause.open || intro.challenge_blocked() || intro.controls_blocked() || dead || super::town::locked();
    let press = |keys: &mut ButtonInput<KeyCode>, k: KeyCode, held: &mut Vec<KeyCode>| { if !before.contains(&k) { keys.press(k); } held.push(k); };
    let now = time.elapsed_secs_f64();
    if ui {
        // arrows from either stick, with a repeat
        let s = inp.hands[0].stick + inp.hands[1].stick;
        for (axis, (neg, pos)) in [(0usize, (KeyCode::ArrowLeft, KeyCode::ArrowRight)), (1, (KeyCode::ArrowDown, KeyCode::ArrowUp))] {
            let v = if axis == 0 { s.x } else { s.y };
            let dir: i8 = if v > 0.6 { 1 } else if v < -0.6 { -1 } else { 0 };
            let (last, at) = repeat[axis];
            let fire = dir != 0 && (dir != last || now - at > 0.3);
            if dir != last { repeat[axis] = (dir, now + 0.25); } else if fire { repeat[axis].1 = now; }
            if fire { press(&mut keys, if dir > 0 { pos } else { neg }, &mut held); }
        }
        // A / the trigger confirm (held: the intro's hold-to-skip), B backs out
        if inp.held(1, Btn::Primary) || inp.held(0, Btn::Primary) || inp.held(1, Btn::Trigger) || inp.held(0, Btn::Trigger) { press(&mut keys, KeyCode::Enter, &mut held); }
        if inp.just(1, Btn::Secondary) || inp.just(0, Btn::Secondary) { press(&mut keys, KeyCode::Backspace, &mut held); }
        if inp.just(0, Btn::Menu) { if pause.open { press(&mut keys, KeyCode::Escape, &mut held); } else { menu_req.open = true; } }
    } else {
        // the town's ENTER prompts (step into a substory) answer A
        // (held: the bosses' own scenes skip on a held Enter too)
        if inp.held(1, Btn::Primary) { press(&mut keys, KeyCode::Enter, &mut held); }
        if inp.just(0, Btn::Menu) { menu_req.open = true; }
    }
    if let Some(k) = menu_req.key.take() { press(&mut keys, k, &mut held); }
    finish(&mut keys, &held);
}

/// Update, after the native camera: outside the theater the flat camera is the head (a wide view for the desktop).
fn place_views(rig: Res<Rig>, view: Res<View>, inp: Res<VrInput>, mut cam: Query<(&mut Transform, &mut Projection, &mut Camera, &mut RenderLayers), (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>, Without<VrEye>)>,
               mut eyes: Query<&mut Transform, (With<VrEye>, Without<bevy_mod_xr::camera::XrCamera>, Without<Camera2d>)>, set: Res<VrSettings>, mut frame: Local<u32>) {
    // the desktop mirror is a whole extra render: in the headset it draws every third frame (the theater screen, which
    // shows it, every frame); the simulator's window is the eye, the flat view only feeds the theater
    *frame = frame.wrapping_add(1);
    let mirror = view.cinema || !inp.live || (set.mirror && *frame % 3 == 0);
    for (mut t, mut proj, mut c, mut layers) in &mut cam {
        if c.is_active != mirror { c.is_active = mirror; }
        // at the head it sees you as the eyes do (no head in the way); the native cameras see the whole Kreely
        let at_head = !view.cinema && rig.started;
        let want = if at_head { RenderLayers::from_layers(&[L_WORLD, L_SELF]) } else { RenderLayers::from_layers(&[L_WORLD, L_BODY]) };
        if *layers != want { *layers = want; }
        if !at_head { continue; }
        *t = rig.head;
        if let Projection::Perspective(p) = &mut *proj { p.fov = 72f32.to_radians(); p.near = 0.05; }
    }
    // the simulator's eye is the head (a child of its root)
    for mut t in &mut eyes { *t = inp.head; }
}

/// Update: the HUD panel before the eyes (drifting after the view), the theater screen and its room.
fn panels(rig: Res<Rig>, view: Res<View>, set: Res<VrSettings>, time: Res<Time<Real>>, mut anchor: Local<Option<(Vec3, f32)>>,
          mut q: ParamSet<(Query<(&mut Transform, &mut Visibility), With<HudPanel>>, Query<(&mut Transform, &mut Visibility), With<CinemaScreen>>,
                           Query<(&mut Transform, &mut Visibility), With<CinemaHud>>, Query<(&mut Transform, &mut Visibility), With<CinemaRoom>>)>) {
    if !rig.started { return; }
    let head = rig.head.translation;
    let yaw = game_yaw(rig.head.rotation);
    // lazy follow: the panel holds its bearing until the view turns 30 degrees away, then eases after it
    let (mut at_yaw, mut at_y) = anchor.map_or((yaw, head.y), |a| (a.1, a.0.y));
    let off = (yaw - at_yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let k = (time.delta_secs() * 3.).min(1.);
    if !set.hud_follow { } else if off.abs() > 0.5 || anchor.is_none() { at_yaw += off * k; }
    at_y += (head.y - at_y) * k;
    *anchor = Some((Vec3::new(head.x, at_y, head.z), at_yaw));
    let fwd = Vec3::new(at_yaw.sin(), 0., at_yaw.cos());
    let d = set.hud_dist;
    let width = d * 0.95 * set.hud_size;
    let center = head + fwd * d - Vec3::Y * d * 0.12;
    let face = |pos: Vec3, from: Vec3| Transform::from_translation(pos).looking_at(pos + (pos - from), Vec3::Y);
    let hud_vis = if view.cinema || menu_open() { Visibility::Hidden } else { Visibility::Visible };
    for (mut t, mut v) in &mut q.p0() { *t = face(center, head).with_scale(Vec3::splat(width)); *v = hud_vis; }
    // the theater: a 4.4 m screen 4 m ahead, at eye height, fixed where it was when the scene began
    let screen_at = head + fwd * 4. + Vec3::Y * 0.1;
    let cv = if view.cinema { Visibility::Visible } else { Visibility::Hidden };
    for (mut t, mut v) in &mut q.p1() { if view.cinema && *v == Visibility::Hidden || !view.cinema { *t = face(screen_at, head).with_scale(Vec3::splat(4.4)); } *v = cv; }
    let screen = q.p1().iter().next().map(|(t, _)| *t);
    for (mut t, mut v) in &mut q.p2() { if let Some(s) = screen { *t = s; t.translation += (head - s.translation).normalize_or_zero() * 0.01; } *v = cv; }
    for (mut t, mut v) in &mut q.p3() {
        if view.cinema && *v == Visibility::Hidden {
            let is_floor = t.rotation != Quat::IDENTITY;
            t.translation = if is_floor { Vec3::new(head.x, rig.root.translation.y, head.z) } else { head };
        }
        *v = cv;
    }
}

/// The desktop compositor's sprites fill the window.
fn mirror_size(windows: Query<&Window, With<bevy::window::PrimaryWindow>>, mut sprites: Query<&mut Sprite, With<MirrorSprite>>) {
    let Ok(w) = windows.single() else { return; };
    let size = Some(w.size());
    for mut s in &mut sprites { if s.custom_size != size { s.custom_size = size; } }
}
