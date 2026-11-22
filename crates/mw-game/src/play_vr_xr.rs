//! The headset's controllers through OpenXR (MW_VR=1): one action set, bound for the Quest Touch controllers (Quest 3's
//! Touch Plus answer the same profile over Link / Air Link / Virtual Desktop) and for Index / Vive wands and the simple
//! profile; the grip and aim poses as tracked spaces; the head from the located views; haptics.
use super::{Hand, VrInput};
use bevy::prelude::*;
use bevy_mod_openxr::{
    action_binding::{OxrSendActionBindings, OxrSuggestActionBinding},
    action_set_attaching::OxrAttachActionSet,
    action_set_syncing::{OxrActionSetSyncSet, OxrSyncActionSet},
    openxr_session_running,
    resources::{OxrInstance, OxrViews},
    session::OxrSession,
};
use bevy_mod_xr::{session::{session_available, XrSessionCreated}, spaces::{XrSpaceLocationFlags, XrVelocity}};

#[derive(Resource)]
pub struct Actions {
    set: openxr::ActionSet, hands: [openxr::Path; 2],
    grip_pose: openxr::Action<openxr::Posef>, aim_pose: openxr::Action<openxr::Posef>,
    grip: openxr::Action<f32>, trigger: openxr::Action<f32>, stick: openxr::Action<openxr::Vector2f>,
    stick_click: openxr::Action<bool>, primary: openxr::Action<bool>, secondary: openxr::Action<bool>, menu: openxr::Action<bool>,
    trigger_touch: openxr::Action<bool>, thumb_touch: openxr::Action<bool>,
    haptic: openxr::Action<openxr::Haptic>,
}

/// A controller's tracked space: hand (0 left, 1 right), aim (true) or grip.
#[derive(Component)]
pub struct HandSpace(usize, bool);

pub fn build(app: &mut App) {
    if super::mode() != super::Mode::OpenXr { return; }
    app.add_systems(Startup, create_actions.run_if(session_available))
        .add_systems(OxrSendActionBindings, suggest)
        .add_systems(XrSessionCreated, (attach, spawn_spaces))
        .add_systems(PreUpdate, sync.before(OxrActionSetSyncSet).run_if(openxr_session_running));
}

fn create_actions(instance: Res<OxrInstance>, mut commands: Commands) {
    let make = || -> openxr::Result<Actions> {
        let set = instance.create_action_set("madworld", "MadWorld", 0)?;
        let hands = [instance.string_to_path("/user/hand/left")?, instance.string_to_path("/user/hand/right")?];
        Ok(Actions {
            grip_pose: set.create_action("grip_pose", "Hand grip pose", &hands)?,
            aim_pose: set.create_action("aim_pose", "Hand aim pose", &hands)?,
            grip: set.create_action("grip", "Grab (grip)", &hands)?,
            trigger: set.create_action("trigger", "Trigger", &hands)?,
            stick: set.create_action("thumbstick", "Thumbstick", &hands)?,
            stick_click: set.create_action("thumbstick_click", "Thumbstick click", &hands)?,
            primary: set.create_action("primary", "A / X", &hands)?,
            secondary: set.create_action("secondary", "B / Y", &hands)?,
            menu: set.create_action("menu", "Menu", &hands)?,
            trigger_touch: set.create_action("trigger_touch", "Trigger touch", &hands)?,
            thumb_touch: set.create_action("thumb_touch", "Thumb touch", &hands)?,
            haptic: set.create_action("haptic", "Rumble", &hands)?,
            set, hands,
        })
    };
    match make() { Ok(a) => { commands.insert_resource(a); eprintln!("[vr] OpenXR actions created"); } Err(e) => eprintln!("[vr] OpenXR actions failed: {e}") }
}

fn suggest(actions: Option<Res<Actions>>, mut out: MessageWriter<OxrSuggestActionBinding>) {
    let Some(a) = actions else { return; };
    let mut bind = |profile: &str, action: openxr::sys::Action, paths: &[&str]| {
        out.write(OxrSuggestActionBinding { action, interaction_profile: profile.to_string().into(), bindings: paths.iter().map(|p| p.to_string().into()).collect() });
    };
    let both = |s: &str| [format!("/user/hand/left/{s}"), format!("/user/hand/right/{s}")];
    let touch = "/interaction_profiles/oculus/touch_controller";
    for (action, path) in [(a.grip_pose.as_raw(), "input/grip/pose"), (a.aim_pose.as_raw(), "input/aim/pose"), (a.grip.as_raw(), "input/squeeze/value"),
                           (a.trigger.as_raw(), "input/trigger/value"), (a.stick.as_raw(), "input/thumbstick"), (a.stick_click.as_raw(), "input/thumbstick/click"),
                           (a.trigger_touch.as_raw(), "input/trigger/touch"), (a.thumb_touch.as_raw(), "input/thumbstick/touch"), (a.haptic.as_raw(), "output/haptic")] {
        let p = both(path);
        bind(touch, action, &[p[0].as_str(), p[1].as_str()]);
    }
    bind(touch, a.primary.as_raw(), &["/user/hand/left/input/x/click", "/user/hand/right/input/a/click"]);
    bind(touch, a.secondary.as_raw(), &["/user/hand/left/input/y/click", "/user/hand/right/input/b/click"]);
    bind(touch, a.menu.as_raw(), &["/user/hand/left/input/menu/click"]);
    // Valve Index
    let index = "/interaction_profiles/valve/index_controller";
    for (action, path) in [(a.grip_pose.as_raw(), "input/grip/pose"), (a.aim_pose.as_raw(), "input/aim/pose"), (a.grip.as_raw(), "input/squeeze/value"),
                           (a.trigger.as_raw(), "input/trigger/value"), (a.stick.as_raw(), "input/thumbstick"), (a.stick_click.as_raw(), "input/thumbstick/click"),
                           (a.trigger_touch.as_raw(), "input/trigger/touch"), (a.thumb_touch.as_raw(), "input/thumbstick/touch"), (a.primary.as_raw(), "input/a/click"),
                           (a.secondary.as_raw(), "input/b/click"), (a.haptic.as_raw(), "output/haptic")] {
        let p = both(path);
        bind(index, action, &[p[0].as_str(), p[1].as_str()]);
    }
    bind(index, a.menu.as_raw(), &["/user/hand/left/input/system/click"]);
    // the simple profile (poses, select, menu)
    let simple = "/interaction_profiles/khr/simple_controller";
    for (action, path) in [(a.grip_pose.as_raw(), "input/grip/pose"), (a.aim_pose.as_raw(), "input/aim/pose"), (a.trigger.as_raw(), "input/select/click"), (a.haptic.as_raw(), "output/haptic")] {
        let p = both(path);
        bind(simple, action, &[p[0].as_str(), p[1].as_str()]);
    }
    bind(simple, a.menu.as_raw(), &["/user/hand/left/input/menu/click"]);
}

fn attach(actions: Option<Res<Actions>>, mut out: MessageWriter<OxrAttachActionSet>) {
    if let Some(a) = actions { out.write(OxrAttachActionSet(a.set.clone())); }
}

fn sync(actions: Option<Res<Actions>>, mut out: MessageWriter<OxrSyncActionSet>) {
    if let Some(a) = actions { out.write(OxrSyncActionSet(a.set.clone())); }
}

fn spawn_spaces(actions: Option<Res<Actions>>, session: Res<OxrSession>, mut commands: Commands) {
    let Some(a) = actions else { return; };
    for h in 0..2 {
        for aim in [false, true] {
            let action = if aim { &a.aim_pose } else { &a.grip_pose };
            match session.create_action_space(action, a.hands[h], Isometry3d::IDENTITY) {
                Ok(space) => { commands.spawn((space, XrVelocity::default(), HandSpace(h, aim))); }
                Err(e) => eprintln!("[vr] hand space {h}: {e}"),
            }
        }
    }
}

/// PreUpdate (after the action sync): buttons and axes.
pub fn read_buttons(actions: Option<Res<Actions>>, session: Option<Res<OxrSession>>, running: Option<Res<bevy_mod_xr::session::XrState>>, mut inp: ResMut<VrInput>) {
    let live = running.is_some_and(|s| *s == bevy_mod_xr::session::XrState::Running);
    inp.live = live;
    let (Some(a), Some(session), true) = (actions, session, live) else { return; };
    let s: &openxr::Session<openxr::AnyGraphics> = &session;
    for h in 0..2 {
        let path = a.hands[h];
        let f = |x: &openxr::Action<f32>| x.state(s, path).map(|v| v.current_state).unwrap_or(0.);
        let b = |x: &openxr::Action<bool>| x.state(s, path).map(|v| v.current_state).unwrap_or(false);
        let hand: &mut Hand = &mut inp.hands[h];
        hand.grip = f(&a.grip); hand.trigger = f(&a.trigger);
        hand.stick = a.stick.state(s, path).map(|v| Vec2::new(v.current_state.x, v.current_state.y)).unwrap_or(Vec2::ZERO);
        hand.stick_click = b(&a.stick_click); hand.primary = b(&a.primary); hand.secondary = b(&a.secondary); hand.menu = b(&a.menu);
        hand.trigger_touch = b(&a.trigger_touch); hand.thumb_touch = b(&a.thumb_touch);
    }
}

/// Update: the head (the views' mean) and the controller spaces' poses.
pub fn read_poses(inp: ResMut<VrInput>, views: Option<Res<OxrViews>>, spaces: Query<(&HandSpace, &Transform, &XrSpaceLocationFlags, Option<&XrVelocity>)>) {
    let mut inp = inp;
    if !inp.live { return; }
    if let Some(v) = views.filter(|v| v.len() >= 2) {
        let p = |i: usize| { let q = v[i].pose.position; Vec3::new(q.x, q.y, q.z) };
        let o = v[0].pose.orientation;
        inp.head = Transform { translation: (p(0) + p(1)) * 0.5, rotation: Quat::from_xyzw(o.x, o.y, o.z, o.w).normalize(), scale: Vec3::ONE };
        inp.head_tracked = true;
    }
    for (hs, t, flags, vel) in &spaces {
        let hand = &mut inp.hands[hs.0];
        if hs.1 { hand.aim_pose = *t; } else {
            hand.grip_pose = *t; hand.tracked = flags.position_tracked;
            if let Some(v) = vel { hand.vel = v.linear; hand.ang_vel = v.angular; }
        }
    }
}

/// PostUpdate: rumble requests to the controllers (the simulator drops them).
pub fn rumble(actions: Option<Res<Actions>>, session: Option<Res<OxrSession>>, mut inp: ResMut<VrInput>, set: Res<super::VrSettings>) {
    let reqs = std::mem::take(&mut inp.rumble);
    let (Some(a), Some(session)) = (actions, session) else { return; };
    if !inp.live { return; }
    let s: &openxr::Session<openxr::AnyGraphics> = &session;
    for (h, r) in reqs.iter().enumerate() {
        let Some((amp, secs)) = r else { continue; };
        let amp = (amp * set.rumble()).clamp(0., 1.);
        if amp <= 0. { continue; }
        let vib = openxr::HapticVibration::new().amplitude(amp).frequency(openxr::FREQUENCY_UNSPECIFIED).duration(openxr::Duration::from_nanos((secs * 1e9) as i64));
        let _ = a.haptic.apply_feedback(s, a.hands[h], &vib);
    }
}
