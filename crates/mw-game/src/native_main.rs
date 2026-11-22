//! Bevy host for the original statically recompiled boss battle.
mod native_audio;
mod native_camera;
mod native_controls;
mod native_core;
mod native_grade;
mod native_perf;
mod native_prompts;
mod native_sixty;
mod pc_config;
mod pc_menu;
use bevy::{
    audio::AddAudioSource,
    camera::CompositingSpace,
    core_pipeline::{
        schedule::{Core2d, Core2dSystems},
        tonemapping::tonemapping,
    },
    prelude::*,
    render::{
        camera::ExtractedCamera,
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        view::ViewTarget,
        RenderApp,
    },
    window::PrimaryWindow,
};
use mw_native_platform::{
    host,
    native_renderer::sink::{Renderer, TargetAspect},
};
use native_audio::{AudioBridge, NativePcm};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
/// C toggles the added color pack (baked from the explorer palette) over source monochrome.
static COLOR_PACK: AtomicBool = AtomicBool::new(true);
/// F3 switches to the legacy v1 pack (explorer palette over ink, no grade) while color is on.
static LEGACY_PACK: AtomicBool = AtomicBool::new(false);
#[derive(Resource)]
struct Owner {
    session: Arc<native_core::Session>,
    audio: AudioBridge,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.session.shutdown();
    }
}
#[derive(Resource, Clone)]
struct Shared {
    session: Arc<native_core::Session>,
    audio: AudioBridge,
}
#[derive(Component)]
struct Status;
#[derive(Component)]
struct ControlsHelp;
#[derive(Component)]
struct SkipSpinner;
fn main() {
    // pc-controls.toml next to the exe; the internal resolution is read by the render
    // world through MW_INTERNAL_RES, which still wins when set (set before any thread).
    let pc = pc_config::PcConfig::load();
    if std::env::var_os("MW_INTERNAL_RES").is_none() && pc.video.internal_res != "auto" {
        std::env::set_var("MW_INTERNAL_RES", &pc.video.internal_res);
    }
    let video = pc.effective_video();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    native_grade::init_from_env();
    // MW_COLOR_PACK=0 starts in source mode (no pack decodes, no grade): A/B runs and goldens.
    if std::env::var("MW_COLOR_PACK").as_deref() == Ok("0") {
        COLOR_PACK.store(false, Ordering::Relaxed);
    }
    if native_perf::enabled() {
        native_perf::spawn_reporter();
    }
    mw_native_platform::native_hardware::paths::set_base(root.join("native-host-data"));
    let session = native_core::Session::new(root);
    let audio = AudioBridge::default();
    let mut app = App::new();
    app.insert_resource(Owner {
        session: session.clone(),
        audio: audio.clone(),
    })
    .add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "MadWorld — RinRin".into(),
            resolution: (1280, 720).into(),
            present_mode: if video.vsync { bevy::window::PresentMode::AutoVsync } else { bevy::window::PresentMode::AutoNoVsync },
            mode: if video.fullscreen {
                bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current)
            } else {
                bevy::window::WindowMode::Windowed
            },
            ..default()
        }),
        ..default()
    }).set(bevy::log::LogPlugin {
        // Thousands of unimplemented-register warnings per second were written
        // synchronously from the emulation thread; RUST_LOG still overrides.
        filter: format!("{},gecko::gx::unimplemented=error", bevy::log::DEFAULT_FILTER),
        ..default()
    }))
    .add_audio_source::<NativePcm>()
    .init_resource::<native_controls::PcControls>()
    .init_resource::<native_controls::CutsceneSkip>()
    .init_resource::<native_camera::FreeCamera>()
    .insert_resource(pc)
    .add_plugins(native_prompts::plugin)
    .init_resource::<pc_config::CameraLook>()
    .add_plugins(pc_menu::PcMenuPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, (input, skip_spinner.after(input), native_camera::free_camera.after(input), status, controls_help, color_toggle, look_key))
    .add_systems(Last, closing);
    app.sub_app_mut(RenderApp)
        .insert_resource(Shared { session, audio })
        .add_systems(
            Core2d,
            blit_native
                .after(tonemapping)
                .in_set(Core2dSystems::PostProcess),
        );
    app.run();
}
fn setup(mut commands: Commands, mut sources: ResMut<Assets<NativePcm>>, owner: Res<Owner>) {
    commands.spawn((Camera2d, CompositingSpace::Srgb));
    commands.spawn((
        Text::new(native_controls::HELP),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0., 0., 0., 0.9)),
        Node {
            position_type: PositionType::Absolute,
            right: px(12),
            top: px(12),
            padding: UiRect::all(px(12)),
            ..default()
        },
        Visibility::Hidden,
        ControlsHelp,
    ));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(32),
                bottom: px(32),
                width: px(64),
                height: px(64),
                border_radius: BorderRadius::MAX,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundGradient::from(spinner_fill(0.)),
            Visibility::Hidden,
            SkipSpinner,
        ))
        .with_child((
            Node {
                width: px(46),
                height: px(46),
                border_radius: BorderRadius::MAX,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.85)),
            children![(
                Text::new("SKIP"),
                TextFont {
                    font_size: bevy::text::FontSize::Px(12.),
                    ..default()
                },
                TextColor(Color::WHITE),
            )],
        ));
    commands.spawn((
        AudioPlayer(sources.add(NativePcm(owner.audio.clone()))),
        PlaybackSettings::DESPAWN,
    ));
    commands.spawn((
        Text::new("Starting MadWorld…"),
        TextFont {
            font_size: bevy::text::FontSize::Px(18.),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: px(16),
            top: px(16),
            ..default()
        },
        Status,
    ));
}
/// EFB/XFB scale. MW_INTERNAL_RES=1..4 picks it; unset or `auto` matches the
/// window height (the GX framebuffer is 528 lines at most, ~480 visible).
fn internal_resolution(window: UVec2) -> u32 {
    let max = mw_native_platform::native_renderer::MAX_EFB_SCALE;
    match std::env::var("MW_INTERNAL_RES").ok().filter(|v| v != "auto") {
        Some(v) => v.parse::<u32>().unwrap_or(1).clamp(1, max),
        None => ((window.y as f32 / 480.0).round() as u32).clamp(1, max),
    }
}
fn blit_native(
    view: ViewQuery<(&ViewTarget, &ExtractedCamera)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    shared: Res<Shared>,
    mut renderer: Local<Option<Renderer>>,
    mut frames: Local<u64>,
    mut grade: Local<Option<native_grade::Grade>>,
    mut ctx: RenderContext,
) {
    let (target, camera) = view.into_inner();
    if renderer.is_none() {
        let (native, sink) = Renderer::new(
            device.wgpu_device().clone(),
            (**queue).clone(),
            target.main_texture_format(),
            TargetAspect::Ratio(16.0 / 9.0),
            internal_resolution(camera.physical_target_size.unwrap_or(UVec2::ONE)),
        );
        let pack = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../native-host-data/color-pack");
        sink.load_texture_pack(&pack, "RZZE8P");
        sink.load_legacy_texture_pack(&pack.with_file_name("color-pack-legacy"), "RZZE8P");
        let sink: Box<dyn mw_native_platform::native_hardware::host::RenderSink> = if native_perf::enabled() {
            Box::new(native_perf::PerfSink(Box::new(sink)))
        } else {
            Box::new(sink)
        };
        let sink: Box<dyn mw_native_platform::native_hardware::host::RenderSink> =
            if native_sixty::enabled() { Box::new(native_sixty::ProbeSink::new(sink)) } else { sink };
        shared.session.start(sink, shared.audio.clone());
        *renderer = Some(native);
    }
    native_perf::COUNTERS.host_frames.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let size = camera.physical_target_size.unwrap_or(UVec2::ONE);
    let renderer = renderer.as_ref().unwrap();
    if let Some(patches) = native_prompts::PENDING.lock().unwrap_or_else(|e| e.into_inner()).take() {
        renderer.set_texture_patches(patches);
    }
    // Opt-in check: MW_COLOR_CAPTURE=<dir> saves color.ppm then source.ppm
    // (pack hidden for frames at+1..at+90, captured at at+60) at host frame MW_COLOR_CAPTURE_FRAME.
    *frames += 1;
    let mut color = COLOR_PACK.load(Ordering::Relaxed);
    if let Some(dir) = std::env::var_os("MW_COLOR_CAPTURE") {
        let at = std::env::var("MW_COLOR_CAPTURE_FRAME").ok().and_then(|v| v.parse().ok()).unwrap_or(2400u64);
        let save = |name: &str| {
            if let Some(f) = renderer.capture_xfb() {
                let mut ppm = format!("P6 {} {} 255\n", f.width, f.height).into_bytes();
                ppm.extend(f.rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]));
                let _ = std::fs::write(PathBuf::from(&dir).join(name), ppm);
            }
        };
        if *frames == at {
            save("color.ppm");
        } else if *frames == at + 60 {
            save("source.ppm");
        }
        color &= !(at + 1..at + 90).contains(&*frames);
    }
    let legacy = LEGACY_PACK.load(Ordering::Relaxed);
    renderer.set_texture_pack_mode(match (color, legacy) {
        (false, _) => 0,
        (true, false) => 1,
        (true, true) => 2,
    });
    renderer.blit_into_encoder(
        ctx.command_encoder(),
        target.main_texture_view(),
        (size.x, size.y),
        bevy::render::render_resource::LoadOp::Clear(bevy::color::LinearRgba::BLACK.into()),
    );
    // Layer 2 color grade (night/day mood); off with the pack so C shows the original frame.
    if color && !legacy {
        let grade = grade.get_or_insert_with(|| native_grade::Grade::new(device.wgpu_device(), target.main_texture_format()));
        let write = target.post_process_write();
        grade.apply(device.wgpu_device(), &queue, ctx.command_encoder(), write.source, write.destination);
    }
}
/// Look reference (MW_LOOK_CAPTURE=<dir>): F12 saves the window (the recomp's final look, grade included) through
/// Bevy's asynchronous screenshot, as does every 5 s; at most 200 files. With MW_GX_LIGHT_DUMP, F12 also toggles the
/// GX ink-lighting dump. (A render-node XFB readback here collided with surface reconfiguration and crashed.)
fn look_key(keys: Res<ButtonInput<KeyCode>>, time: Res<Time<Real>>, mut next: Local<f64>, mut next_view: Local<f64>, mut saved: Local<u32>, mut commands: Commands) {
    let Some(dir) = std::env::var_os("MW_LOOK_CAPTURE") else { return; };
    let manual = keys.just_pressed(KeyCode::F12);
    let now = time.elapsed_secs_f64();
    if manual && std::env::var_os("MW_GX_LIGHT_DUMP").is_some() {
        let dump = &mw_native_platform::native_renderer::GX_LIGHT_DUMP_ON;
        let on = !dump.load(Ordering::Relaxed);
        dump.store(on, Ordering::Relaxed);
        eprintln!("[gx-light] dump {} t={now:.2}", if on { "on" } else { "off" });
    }
    // While the GX light dump is on, the view matrix (g_DrawEnv 80694960, the MTXMultVec source for the light group)
    // once a second, with the same wall-clock stamp as the [gx-light] lines.
    if mw_native_platform::native_renderer::GX_LIGHT_DUMP_ON.load(Ordering::Relaxed) && now >= *next_view {
        *next_view = now + 1.;
        let m: Vec<String> = (0..12).map(|i| mw_native_platform::host::peek_mem1_u32(0x8069_4960 + 4 * i)
            .map_or("null".into(), |w| format!("{}", f32::from_bits(w)))).collect();
        let wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0., |d| d.as_secs_f64());
        eprintln!("[gx-view] t={wall:.3} [{}]", m.join(","));
    }
    if (manual || now >= *next) && *saved < 200 {
        *next = now + 5.;
        *saved += 1;
        let name = format!("{}-{:07.2}{}.png", if manual { "f12" } else { "auto" }, now, if COLOR_PACK.load(Ordering::Relaxed) { "-color" } else { "-source" });
        if manual { eprintln!("[look-capture] {name}"); }
        commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(PathBuf::from(&dir).join(name)));
    }
}
fn input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
    mut controls: ResMut<native_controls::PcControls>,
    mut skip: ResMut<native_controls::CutsceneSkip>,
    mut was_skippable: Local<bool>,
    mut diag_hold: Local<Option<Vec<(f32, f32)>>>,
    pad_input: (
        Query<(&bevy::input::gamepad::Gamepad, Option<&Name>)>,
        Res<pc_config::PcConfig>,
        ResMut<pc_config::CameraLook>,
        ResMut<pc_menu::SettingsMenu>,
        Res<bevy::input::mouse::AccumulatedMouseMotion>,
    ),
    mut pad_locals: (
        Local<pc_config::StickGestures>,
        Local<Option<Vec<(f32, f32, String)>>>,
        Local<([u8; 8], bool)>,
    ),
) {
    let (pads, cfg, mut look, mut menu, mouse_motion) = pad_input;
    let (gestures, diag_pad, last) = &mut pad_locals;
    let focused = windows.single().is_ok_and(|w| w.focused);
    let event = native_controls::EVENT_PLAYER;
    let skippable = matches!(
        (host::peek_mem1_u32(event), host::peek_mem1_u32(event + 4)),
        (Some(head), Some(state)) if native_controls::event_skippable(head, state)
    );
    if skippable != *was_skippable {
        *was_skippable = skippable;
        info!("[pc-skip] skippable original event: {skippable}");
    }
    // Default-off diagnostic: MW_PC_DIAG_HOLD_ENTER=start,seconds holds a virtual Enter;
    // several windows separated by ';' script taps and holds (e.g. accept a prompt, then skip).
    let diag = diag_hold.get_or_insert_with(|| {
        std::env::var("MW_PC_DIAG_HOLD_ENTER").ok().map_or_else(Vec::new, |v| {
            v.split(';')
                .filter_map(|w| {
                    let (a, b) = w.split_once(',')?;
                    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
                })
                .collect()
        })
    });
    let t = time.elapsed_secs();
    let diag_enter = diag.iter().any(|&(start, secs)| t >= start && t < start + secs);
    // First connected gamepad; or, default-off diagnostic MW_PC_DIAG_PAD=start,secs,PadA+PadRB;...
    // a scripted virtual pad (also LStickUp/Down/Left/Right, RStickUp/Down/Left/Right) that
    // goes through the same bindings, so a run without a controller shows pad input arriving.
    let diag_pad = diag_pad.get_or_insert_with(|| {
        std::env::var("MW_PC_DIAG_PAD").ok().map_or_else(Vec::new, |v| {
            v.split(';')
                .filter_map(|w| {
                    let mut it = w.splitn(3, ',');
                    Some((it.next()?.trim().parse().ok()?, it.next()?.trim().parse().ok()?, it.next()?.trim().to_string()))
                })
                .collect()
        })
    });
    let mut pad = pads.iter().next().map(|(g, _)| pc_config::PadView::from_gamepad(g));
    let mut diag_pad_active = false;
    for (start, secs, tokens) in diag_pad.iter() {
        let view = pad.get_or_insert_with(Default::default);
        let held = t >= *start && t < start + secs;
        diag_pad_active |= held;
        for token in tokens.split('+').filter(|_| held) {
            let dir = |s: &str| match s { "Up" => Vec2::Y, "Down" => Vec2::NEG_Y, "Left" => Vec2::NEG_X, "Right" => Vec2::X, _ => Vec2::ZERO };
            if let Some(d) = token.strip_prefix("LStick") {
                view.left += dir(d);
            } else if let Some(d) = token.strip_prefix("RStick") {
                view.right += dir(d);
            } else if let Some(Input::Pad(b)) = pc_config::Input::parse(token) {
                view.pressed.push(b);
                if t - time.delta_secs() < *start { view.just_pressed.push(b); }
            }
        }
    }
    use pc_config::Input;
    if pad.is_some() != last.1 {
        last.1 = pad.is_some();
        let name = pads.iter().next().and_then(|(_, n)| n.map(|n| n.to_string()));
        info!("[pc-pad] gamepad {}: {}", if last.1 { "active" } else { "gone" }, name.unwrap_or_else(|| "virtual (MW_PC_DIAG_PAD)".into()));
    }
    let stick = pad.as_ref().and_then(|p| gestures.update(p.right, cfg.gamepad.gesture_threshold));
    let live = (focused || diag_pad_active) && !menu.open;
    if menu.block_until_release {
        let any_pad = pad.as_ref().is_some_and(|p| !p.pressed.is_empty());
        if keys.get_pressed().next().is_none() && mouse.get_pressed().next().is_none() && !any_pad {
            menu.block_until_release = false;
        }
    }
    let inputs = pc_config::Inputs { keys: &keys, mouse: &mouse, pad: pad.as_ref(), stick };
    let state = cfg.bindings.evaluate(&inputs, cfg.gamepad.deadzone, live && !menu.block_until_release);
    if live && !menu.block_until_release && state.just_pressed(pc_config::Action::Settings) {
        menu.open();
    }
    *look = if state.focused && !menu.open {
        pc_config::CameraLook { stick: state.camera, mouse: mouse_motion.delta, recenter: state.just_pressed(pc_config::Action::LockOn) }
    } else {
        default()
    };
    let confirm = state.pressed(pc_config::Action::Confirm);
    let before = skip.progress;
    skip.update(skippable, diag_enter || confirm, time.delta_secs());
    if before > 0. && skip.progress == 0. && skippable && (diag_enter || confirm) {
        info!("[pc-skip] hold complete: pressing Remote B for the original event skip");
    }
    let mut packet = controls.packet_from(&state);
    if diag_enter {
        packet[1] |= 8; // Remote A, as the real Enter binding.
    }
    if state.focused || diag_enter {
        skip.apply_held(&mut packet, state.pressed(pc_config::Action::Attack));
    }
    if pad.is_some() && packet != last.0 {
        // Packet changes while a pad is active: evidence that pad input reaches the game.
        debug!("[pc-pad] t={t:.2} packet={packet:02x?}");
        if diag_pad_active { info!("[pc-pad] t={t:.2} packet={packet:02x?}"); }
    }
    last.0 = packet;
    host::set_controller(packet);
}
fn spinner_fill(progress: f32) -> ConicGradient {
    let angle = progress.clamp(0., 1.) * std::f32::consts::TAU;
    let (fill, track) = (Color::WHITE, Color::srgba(1., 1., 1., 0.25));
    ConicGradient::new(
        UiPosition::CENTER,
        vec![
            AngularColorStop::new(fill, 0.),
            AngularColorStop::new(fill, angle),
            AngularColorStop::new(track, angle),
            AngularColorStop::new(track, std::f32::consts::TAU),
        ],
    )
}
/// Shown only while Enter is held during a skippable original event; short taps
/// (menu confirm) never flash it.
fn skip_spinner(
    skip: Res<native_controls::CutsceneSkip>,
    mut spinner: Query<(&mut BackgroundGradient, &mut Visibility), With<SkipSpinner>>,
) {
    for (mut gradient, mut visibility) in &mut spinner {
        let shown = skip.progress > 0.08;
        visibility.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
        if shown {
            *gradient = BackgroundGradient::from(spinner_fill(skip.progress));
        }
    }
}
fn color_toggle(keys: Res<ButtonInput<KeyCode>>, menu: Res<pc_menu::SettingsMenu>, mut cfg: ResMut<pc_config::PcConfig>) {
    if menu.open {
        return;
    }
    let mut changed = false;
    if keys.just_pressed(KeyCode::KeyC) {
        COLOR_PACK.fetch_xor(true, Ordering::Relaxed);
        changed = true;
    }
    if keys.just_pressed(KeyCode::F2) {
        native_grade::NIGHT.fetch_xor(true, Ordering::Relaxed);
    }
    if keys.just_pressed(KeyCode::F3) {
        LEGACY_PACK.fetch_xor(true, Ordering::Relaxed);
        changed = true;
    }
    // Hotkeys and the settings menu share one persisted color mode.
    if changed {
        cfg.color = match (COLOR_PACK.load(Ordering::Relaxed), LEGACY_PACK.load(Ordering::Relaxed)) {
            (false, _) => pc_config::ColorMode::Original,
            (true, false) => pc_config::ColorMode::Color,
            (true, true) => pc_config::ColorMode::Legacy,
        };
        cfg.save();
    }
}
fn controls_help(
    keys: Res<ButtonInput<KeyCode>>,
    cfg: Res<pc_config::PcConfig>,
    mut panels: Query<(&mut Visibility, &mut Text), With<ControlsHelp>>,
) {
    if keys.just_pressed(KeyCode::F1) {
        for (mut visibility, mut text) in &mut panels {
            // Built from the current bindings, so rebinding never leaves stale help.
            let help = pc_menu::help_text(&cfg);
            if text.as_str() != help {
                **text = help;
            }
            *visibility = if *visibility == Visibility::Hidden {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
fn status(owner: Res<Owner>, mut text: Query<&mut Text, With<Status>>) {
    let value = owner.session.status();
    for mut text in &mut text {
        if text.as_str() != value {
            **text = value.clone();
        }
    }
}
fn closing(mut exits: MessageReader<AppExit>, owner: Res<Owner>) {
    if exits.read().next().is_some() {
        owner.session.shutdown();
    }
}

#[cfg(test)]
mod native_gpu;
