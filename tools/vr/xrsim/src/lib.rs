//! mw_xrsim: a headless OpenXR runtime that stands in for a Quest 3 over Link, for testing MadWorld's VR path on a
//! machine with no headset. Point the OpenXR loader at it for one process only:
//!
//!   XR_RUNTIME_JSON=D:/.../tools/vr/xrsim/xrsim.json
//!
//! It offers XR_KHR_vulkan_enable2 (the game's renderer), a stereo HMD (two 960 x 1056 views, 90 degree fov, 72 Hz)
//! in a STAGE space, and two Oculus Touch controllers whose poses and buttons follow a script (`MW_XRSIM_SCRIPT`,
//! JSON, on seconds since the session began). Swapchains are real Vulkan images on the game's device; `MW_XRSIM_DUMP`
//! ("frame,frame,...:dir") writes those frames' two eye images side by side as PNG. Haptics and session events are
//! logged to stderr ("[xrsim] ...").
#![allow(clippy::missing_safety_doc)]
use ash::vk;
use ash::vk::Handle as _;
use openxr_sys as xr;
use openxr_sys::Handle;
use std::collections::{HashMap, VecDeque};
use std::ffi::{c_char, CStr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Per-eye size: 960 x 1056, or MW_XRSIM_VIEW=WxH (Quest 3 over Link renders about 1832 x 1920 at 1.0x).
fn view_size() -> (u32, u32) {
    std::env::var("MW_XRSIM_VIEW").ok().and_then(|v| { let (w, h) = v.split_once('x')?; Some((w.parse().ok()?, h.parse().ok()?)) }).unwrap_or((960, 1056))
}
const PERIOD_NS: i64 = 13_888_889; // 72 Hz
const IPD: f32 = 0.064;

// ---------------------------------------------------------------- the script

#[derive(serde::Deserialize, Clone, Default)]
struct Step {
    from: f32, to: f32,
    #[serde(default)] buttons: Vec<String>,
    head: Option<[f32; 5]>, head_to: Option<[f32; 5]>,
    l: Option<[f32; 3]>, r: Option<[f32; 3]>, l_to: Option<[f32; 3]>, r_to: Option<[f32; 3]>,
    l_rot: Option<[f32; 3]>, r_rot: Option<[f32; 3]>,
    lstick: Option<[f32; 2]>, rstick: Option<[f32; 2]>,
}

#[derive(Clone, Copy, Default)]
struct Pose { p: [f32; 3], q: [f32; 4] }
#[derive(Clone, Copy, Default)]
struct HandState { grip: Pose, aim: Pose, squeeze: f32, trigger: f32, stick: [f32; 2], stick_click: bool, a: bool, b: bool, menu: bool }
struct Input { head: Pose, hands: [HandState; 2] }

fn quat_ypr(yaw: f32, pitch: f32, roll: f32) -> [f32; 4] {
    // yaw about Y, then pitch about X, then roll about Z (Y-X-Z), as [x, y, z, w]
    let (sy, cy) = (yaw * 0.5).sin_cos(); let (sp, cp) = (pitch * 0.5).sin_cos(); let (sr, cr) = (roll * 0.5).sin_cos();
    let qy = [0., sy, 0., cy]; let qx = [sp, 0., 0., cp]; let qz = [0., 0., sr, cr];
    qmul(qmul(qy, qx), qz)
}
fn qmul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
     a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
     a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
     a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2]]
}
fn qrot(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let p = qmul(qmul(q, [v[0], v[1], v[2], 0.]), [-q[0], -q[1], -q[2], q[3]]);
    [p[0], p[1], p[2]]
}
fn qinv(q: [f32; 4]) -> [f32; 4] { [-q[0], -q[1], -q[2], q[3]] }
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn lerp(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] { [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k] }

fn input_at(script: &[Step], t: f32) -> Input {
    let mut head = Pose { p: [0., 1.72, 0.], q: [0., 0., 0., 1.] };
    // the hands' rest: chest high, forward (-Z), pointing ahead
    let mut pos = [[-0.2, 1.42, -0.36], [0.2, 1.42, -0.36]];
    let mut rot = [[0f32; 3]; 2];
    let mut hands = [HandState::default(); 2];
    for st in script.iter().filter(|s| s.from <= t && t < s.to) {
        let k = ((t - st.from) / (st.to - st.from).max(1e-3)).clamp(0., 1.);
        let k = k * k * (3. - 2. * k);
        if let Some(a) = st.head {
            let h = st.head_to.map_or(a, |b| std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k));
            head = Pose { p: [h[0], h[1], h[2]], q: quat_ypr(h[3].to_radians(), h[4].to_radians(), 0.) };
            // (the hands come along with a walking head)
            let d = sub(head.p, [0., 1.72, 0.]);
            for q in pos.iter_mut() { *q = add(*q, [d[0], 0., d[2]]); }
        }
        for (h, (a, b, r)) in [(st.l, st.l_to, st.l_rot), (st.r, st.r_to, st.r_rot)].into_iter().enumerate() {
            if let Some(a) = a { pos[h] = b.map_or(a, |b| lerp(a, b, k)); }
            if let Some(r) = r { rot[h] = r; }
        }
        if let Some(s) = st.lstick { hands[0].stick = s; }
        if let Some(s) = st.rstick { hands[1].stick = s; }
        for b in &st.buttons {
            match b.as_str() {
                "A" => hands[1].a = true, "B" => hands[1].b = true, "X" => hands[0].a = true, "Y" => hands[0].b = true, "Menu" => hands[0].menu = true,
                "LGrip" => hands[0].squeeze = 1., "RGrip" => hands[1].squeeze = 1., "LTrigger" => hands[0].trigger = 1., "RTrigger" => hands[1].trigger = 1.,
                "LClick" => hands[0].stick_click = true, "RClick" => hands[1].stick_click = true, _ => {}
            }
        }
    }
    for h in 0..2 {
        let q = quat_ypr(rot[h][1].to_radians(), rot[h][0].to_radians(), rot[h][2].to_radians());
        // as the OpenXR spec defines Touch's poses: the script's rotation is the controller's pointing ray (the aim:
        // -Z forward, +Y up), leaving a little ahead of the palm; the grip pose's -Z runs up the handle through the
        // fist toward the thumb, 55 degrees above the ray
        hands[h].grip = Pose { p: pos[h], q: qmul(q, quat_ypr(0., 0.96, 0.)) };
        hands[h].aim = Pose { p: add(pos[h], qrot(q, [0., 0.02, -0.05])), q };
    }
    Input { head, hands }
}

// ---------------------------------------------------------------- state

enum SpaceKind { Reference(xr::ReferenceSpaceType), Action { action: u64, sub: u64, offset: Pose } }
struct Swap { images: Vec<vk::Image>, memory: Vec<vk::DeviceMemory>, w: u32, h: u32, layers: u32, format: vk::Format, next: u32, last: u32 }
struct ActionInfo { name: String, ty: xr::ActionType }
struct Vk { entry: ash::Entry, instance: Option<ash::Instance>, phys: vk::PhysicalDevice, device: Option<ash::Device>, family: u32, queue_index: u32 }

struct State {
    paths: Vec<String>, next: u64,
    vk: Option<Vk>,
    session: u64, session_state: xr::SessionState, events: VecDeque<(u64, xr::SessionState)>,
    spaces: HashMap<u64, SpaceKind>, swaps: HashMap<u64, Swap>,
    sets: HashMap<u64, String>, actions: HashMap<u64, ActionInfo>, bindings: HashMap<u64, Vec<String>>,
    start: Instant, began: Option<Instant>, frame: u64, last_wait: Option<Instant>,
    /// a Link-style drop is under way (MW_XRSIM_DROP_FILE): the session was stopped by the runtime
    dropped: bool,
    script: Vec<Step>, dump: (Vec<u64>, String),
}
static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap();
    if g.is_none() {
        let script = std::env::var("MW_XRSIM_SCRIPT").ok().map(|p| {
            let t = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("[xrsim] MW_XRSIM_SCRIPT {p}: {e}"));
            serde_json::from_str::<Vec<Step>>(&t).unwrap_or_else(|e| panic!("[xrsim] script: {e}"))
        }).unwrap_or_default();
        let dump = std::env::var("MW_XRSIM_DUMP").ok().and_then(|v| {
            let (frames, dir) = v.split_once(':')?;
            Some((frames.split(',').filter_map(|f| f.trim().parse().ok()).collect(), dir.replace('|', ":")))
        }).unwrap_or_default();
        *g = Some(State { paths: vec![], next: 100, vk: None, session: 0, session_state: xr::SessionState::UNKNOWN, events: VecDeque::new(),
                          spaces: HashMap::new(), swaps: HashMap::new(), sets: HashMap::new(), actions: HashMap::new(), bindings: HashMap::new(),
                          start: Instant::now(), began: None, frame: 0, last_wait: None, script, dump, dropped: false });
    }
    f(g.as_mut().unwrap())
}
impl State {
    fn handle(&mut self) -> u64 { self.next += 1; self.next }
    fn path(&mut self, s: &str) -> u64 {
        if let Some(i) = self.paths.iter().position(|p| p == s) { return i as u64 + 1; }
        self.paths.push(s.to_string()); self.paths.len() as u64
    }
    fn path_str(&self, p: u64) -> &str { self.paths.get((p as usize).wrapping_sub(1)).map(|s| s.as_str()).unwrap_or("") }
    fn now_ns(&self) -> i64 { self.start.elapsed().as_nanos() as i64 + 1_000_000 }
    fn t_of(&self, time: i64) -> f32 {
        let base = self.began.map_or(0, |b| (b - self.start).as_nanos() as i64 + 1_000_000);
        ((time - base) as f64 / 1e9).max(0.) as f32
    }
    fn input(&self, time: i64) -> Input { input_at(&self.script, self.t_of(time)) }
    fn set_state(&mut self, st: xr::SessionState) {
        self.session_state = st;
        self.events.push_back((self.session, st));
        eprintln!("[xrsim] session -> {st:?}");
    }
    /// The input source an action reads for a hand (from the suggested Touch bindings), e.g. "input/trigger/value".
    fn source(&self, action: u64, sub: u64) -> Option<(usize, String)> {
        let sub = self.path_str(sub).to_string();
        let b = self.bindings.get(&action)?;
        b.iter().filter(|p| sub.is_empty() || p.starts_with(&sub)).find_map(|p| {
            let hand = if p.starts_with("/user/hand/left") { 0 } else if p.starts_with("/user/hand/right") { 1 } else { return None };
            Some((hand, p.splitn(5, '/').nth(4).unwrap_or("").to_string()))
        })
    }
    fn space_pose(&self, id: u64, time: i64) -> Option<Pose> {
        let inp = self.input(time);
        match self.spaces.get(&id)? {
            SpaceKind::Reference(t) => Some(match *t {
                xr::ReferenceSpaceType::VIEW => inp.head,
                xr::ReferenceSpaceType::LOCAL => Pose { p: [0., 1.72, 0.], q: [0., 0., 0., 1.] },
                _ => Pose { p: [0.; 3], q: [0., 0., 0., 1.] },
            }),
            SpaceKind::Action { action, sub, offset } => {
                let (hand, src) = self.source(*action, *sub)?;
                let base = if src.contains("aim") { inp.hands[hand].aim } else { inp.hands[hand].grip };
                Some(Pose { p: add(base.p, qrot(base.q, offset.p)), q: qmul(base.q, offset.q) })
            }
        }
    }
}

fn pose_from(p: &xr::Posef) -> Pose { Pose { p: [p.position.x, p.position.y, p.position.z], q: [p.orientation.x, p.orientation.y, p.orientation.z, p.orientation.w] } }
fn to_posef(p: Pose) -> xr::Posef {
    xr::Posef { orientation: xr::Quaternionf { x: p.q[0], y: p.q[1], z: p.q[2], w: p.q[3] }, position: xr::Vector3f { x: p.p[0], y: p.p[1], z: p.p[2] } }
}
fn relative(base: Pose, p: Pose) -> Pose { let bi = qinv(base.q); Pose { p: qrot(bi, sub(p.p, base.p)), q: qmul(bi, p.q) } }

unsafe fn write_str(dst: *mut c_char, cap: usize, s: &str) {
    let b = s.as_bytes(); let n = b.len().min(cap - 1);
    std::ptr::copy_nonoverlapping(b.as_ptr() as *const c_char, dst, n); *dst.add(n) = 0;
}
unsafe fn cstr<'a>(p: *const c_char) -> &'a str { if p.is_null() { "" } else { CStr::from_ptr(p).to_str().unwrap_or("") } }

// ---------------------------------------------------------------- the loader

#[no_mangle]
pub unsafe extern "system" fn xrNegotiateLoaderRuntimeInterface(_info: *const xr::loader::XrNegotiateLoaderInfo, req: *mut xr::loader::XrNegotiateRuntimeRequest) -> xr::Result {
    let r = &mut *req;
    r.runtime_interface_version = 1;
    r.runtime_api_version = xr::CURRENT_API_VERSION;
    r.get_instance_proc_addr = Some(get_instance_proc_addr);
    eprintln!("[xrsim] loader negotiated");
    xr::Result::SUCCESS
}

unsafe extern "system" fn unsupported() -> xr::Result { xr::Result::ERROR_FUNCTION_UNSUPPORTED }

macro_rules! table { ($name:expr, $( $s:literal => $f:expr ),* $(,)?) => {
    match $name { $( $s => Some(std::mem::transmute::<*const (), xr::pfn::VoidFunction>($f as *const ())), )* _ => None }
} }

unsafe extern "system" fn get_instance_proc_addr(_instance: xr::Instance, name: *const c_char, out: *mut Option<xr::pfn::VoidFunction>) -> xr::Result {
    let n = cstr(name);
    let f: Option<xr::pfn::VoidFunction> = table!(n,
        "xrGetInstanceProcAddr" => get_instance_proc_addr as unsafe extern "system" fn(_, _, _) -> _,
        "xrEnumerateInstanceExtensionProperties" => enumerate_extensions as unsafe extern "system" fn(_, _, _, _) -> _,
        "xrEnumerateApiLayerProperties" => enumerate_layers as unsafe extern "system" fn(_, _, _) -> _,
        "xrCreateInstance" => create_instance as unsafe extern "system" fn(_, _) -> _,
        "xrDestroyInstance" => ok1 as unsafe extern "system" fn(_) -> _,
        "xrGetInstanceProperties" => instance_properties as unsafe extern "system" fn(_, _) -> _,
        "xrPollEvent" => poll_event as unsafe extern "system" fn(_, _) -> _,
        "xrResultToString" => result_to_string as unsafe extern "system" fn(_, _, _) -> _,
        "xrStructureTypeToString" => structure_to_string as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetSystem" => get_system as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetSystemProperties" => system_properties as unsafe extern "system" fn(_, _, _) -> _,
        "xrEnumerateEnvironmentBlendModes" => blend_modes as unsafe extern "system" fn(_, _, _, _, _, _) -> _,
        "xrCreateSession" => create_session as unsafe extern "system" fn(_, _, _) -> _,
        "xrDestroySession" => ok1 as unsafe extern "system" fn(_) -> _,
        "xrEnumerateReferenceSpaces" => reference_spaces as unsafe extern "system" fn(_, _, _, _) -> _,
        "xrCreateReferenceSpace" => create_reference_space as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetReferenceSpaceBoundsRect" => bounds_rect as unsafe extern "system" fn(_, _, _) -> _,
        "xrCreateActionSpace" => create_action_space as unsafe extern "system" fn(_, _, _) -> _,
        "xrLocateSpace" => locate_space as unsafe extern "system" fn(_, _, _, _) -> _,
        "xrDestroySpace" => destroy_space as unsafe extern "system" fn(_) -> _,
        "xrEnumerateViewConfigurations" => view_configurations as unsafe extern "system" fn(_, _, _, _, _) -> _,
        "xrGetViewConfigurationProperties" => view_configuration_properties as unsafe extern "system" fn(_, _, _, _) -> _,
        "xrEnumerateViewConfigurationViews" => view_configuration_views as unsafe extern "system" fn(_, _, _, _, _, _) -> _,
        "xrEnumerateSwapchainFormats" => swapchain_formats as unsafe extern "system" fn(_, _, _, _) -> _,
        "xrCreateSwapchain" => create_swapchain as unsafe extern "system" fn(_, _, _) -> _,
        "xrDestroySwapchain" => destroy_swapchain as unsafe extern "system" fn(_) -> _,
        "xrEnumerateSwapchainImages" => swapchain_images as unsafe extern "system" fn(_, _, _, _) -> _,
        "xrAcquireSwapchainImage" => acquire_image as unsafe extern "system" fn(_, _, _) -> _,
        "xrWaitSwapchainImage" => ok2 as unsafe extern "system" fn(_, _) -> _,
        "xrReleaseSwapchainImage" => release_image as unsafe extern "system" fn(_, _) -> _,
        "xrBeginSession" => begin_session as unsafe extern "system" fn(_, _) -> _,
        "xrEndSession" => end_session as unsafe extern "system" fn(_) -> _,
        "xrRequestExitSession" => request_exit as unsafe extern "system" fn(_) -> _,
        "xrWaitFrame" => wait_frame as unsafe extern "system" fn(_, _, _) -> _,
        "xrBeginFrame" => ok2 as unsafe extern "system" fn(_, _) -> _,
        "xrEndFrame" => end_frame as unsafe extern "system" fn(_, _) -> _,
        "xrLocateViews" => locate_views as unsafe extern "system" fn(_, _, _, _, _, _) -> _,
        "xrStringToPath" => string_to_path as unsafe extern "system" fn(_, _, _) -> _,
        "xrPathToString" => path_to_string as unsafe extern "system" fn(_, _, _, _, _) -> _,
        "xrCreateActionSet" => create_action_set as unsafe extern "system" fn(_, _, _) -> _,
        "xrDestroyActionSet" => ok1 as unsafe extern "system" fn(_) -> _,
        "xrCreateAction" => create_action as unsafe extern "system" fn(_, _, _) -> _,
        "xrDestroyAction" => ok1 as unsafe extern "system" fn(_) -> _,
        "xrSuggestInteractionProfileBindings" => suggest_bindings as unsafe extern "system" fn(_, _) -> _,
        "xrAttachSessionActionSets" => attach_sets as unsafe extern "system" fn(_, _) -> _,
        "xrGetCurrentInteractionProfile" => current_profile as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetActionStateBoolean" => state_bool as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetActionStateFloat" => state_float as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetActionStateVector2f" => state_vec2 as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetActionStatePose" => state_pose as unsafe extern "system" fn(_, _, _) -> _,
        "xrSyncActions" => ok2 as unsafe extern "system" fn(_, _) -> _,
        "xrEnumerateBoundSourcesForAction" => bound_sources as unsafe extern "system" fn(_, _, _, _, _) -> _,
        "xrGetInputSourceLocalizedName" => localized_name as unsafe extern "system" fn(_, _, _, _, _) -> _,
        "xrApplyHapticFeedback" => haptic as unsafe extern "system" fn(_, _, _) -> _,
        "xrStopHapticFeedback" => ok2 as unsafe extern "system" fn(_, _) -> _,
        "xrCreateVulkanInstanceKHR" => create_vulkan_instance as unsafe extern "system" fn(_, _, _, _) -> _,
        "xrCreateVulkanDeviceKHR" => create_vulkan_device as unsafe extern "system" fn(_, _, _, _) -> _,
        "xrGetVulkanGraphicsDevice2KHR" => vulkan_graphics_device as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetVulkanGraphicsRequirements2KHR" => vulkan_requirements as unsafe extern "system" fn(_, _, _) -> _,
        "xrGetVulkanGraphicsRequirementsKHR" => vulkan_requirements as unsafe extern "system" fn(_, _, _) -> _,
    );
    *out = Some(f.unwrap_or(std::mem::transmute::<*const (), xr::pfn::VoidFunction>(unsupported as *const ())));
    xr::Result::SUCCESS
}

unsafe extern "system" fn ok1(_a: u64) -> xr::Result { xr::Result::SUCCESS }
unsafe extern "system" fn ok2(_a: u64, _b: *const u8) -> xr::Result { xr::Result::SUCCESS }

// ---------------------------------------------------------------- instance and system

unsafe extern "system" fn enumerate_extensions(_layer: *const c_char, cap: u32, count: *mut u32, out: *mut xr::ExtensionProperties) -> xr::Result {
    let exts: [&str; 2] = ["XR_KHR_vulkan_enable2", "XR_KHR_vulkan_enable"];
    *count = exts.len() as u32;
    if cap == 0 { return xr::Result::SUCCESS; }
    if (cap as usize) < exts.len() { return xr::Result::ERROR_SIZE_INSUFFICIENT; }
    for (i, e) in exts.iter().enumerate() {
        let o = &mut *out.add(i);
        write_str(o.extension_name.as_mut_ptr(), o.extension_name.len(), e);
        o.extension_version = 2;
    }
    xr::Result::SUCCESS
}
unsafe extern "system" fn enumerate_layers(_cap: u32, count: *mut u32, _out: *mut u8) -> xr::Result { *count = 0; xr::Result::SUCCESS }
unsafe extern "system" fn create_instance(_info: *const xr::InstanceCreateInfo, out: *mut xr::Instance) -> xr::Result {
    let h = with(|s| s.handle());
    *out = xr::Instance::from_raw(h);
    eprintln!("[xrsim] instance created");
    xr::Result::SUCCESS
}
unsafe extern "system" fn instance_properties(_i: xr::Instance, out: *mut xr::InstanceProperties) -> xr::Result {
    let o = &mut *out;
    o.runtime_version = xr::Version::new(0, 1, 0);
    write_str(o.runtime_name.as_mut_ptr(), o.runtime_name.len(), "MadWorld XR Simulator");
    xr::Result::SUCCESS
}
unsafe extern "system" fn poll_event(_i: xr::Instance, buf: *mut xr::EventDataBuffer) -> xr::Result {
    // a Link-style drop (MW_XRSIM_DROP_FILE exists): the runtime stops the running session itself, refuses to end it
    // (ERROR_SESSION_NOT_RUNNING, as Meta's does after a USB drop) and brings it back READY once the file is gone
    let drop_file = std::env::var_os("MW_XRSIM_DROP_FILE").is_some_and(|f| std::path::Path::new(&f).exists());
    with(|s| {
        if s.session != 0 && drop_file && !s.dropped && matches!(s.session_state, xr::SessionState::FOCUSED | xr::SessionState::VISIBLE | xr::SessionState::SYNCHRONIZED) {
            eprintln!("[xrsim] link drop"); s.dropped = true;
            s.set_state(xr::SessionState::SYNCHRONIZED); s.set_state(xr::SessionState::STOPPING);
        } else if s.session != 0 && !drop_file && s.dropped && s.session_state == xr::SessionState::IDLE {
            eprintln!("[xrsim] link back"); s.dropped = false; s.set_state(xr::SessionState::READY);
        }
    });
    // unplugged mid-session: the session is lost
    with(|s| if s.session != 0 && !plugged() && !matches!(s.session_state, xr::SessionState::LOSS_PENDING | xr::SessionState::EXITING) {
        eprintln!("[xrsim] headset unplugged"); s.set_state(xr::SessionState::LOSS_PENDING);
    });
    let ev = with(|s| s.events.pop_front());
    let Some((session, state)) = ev else { return xr::Result::EVENT_UNAVAILABLE; };
    let time = with(|s| s.now_ns());
    let e = buf as *mut xr::EventDataSessionStateChanged;
    *e = xr::EventDataSessionStateChanged { ty: xr::StructureType::EVENT_DATA_SESSION_STATE_CHANGED, next: std::ptr::null(), session: xr::Session::from_raw(session), state, time: xr::Time::from_nanos(time) };
    xr::Result::SUCCESS
}
unsafe extern "system" fn result_to_string(_i: xr::Instance, v: xr::Result, out: *mut c_char) -> xr::Result { write_str(out, 64, &format!("{v:?}")); xr::Result::SUCCESS }
unsafe extern "system" fn structure_to_string(_i: xr::Instance, v: xr::StructureType, out: *mut c_char) -> xr::Result { write_str(out, 64, &format!("{v:?}")); xr::Result::SUCCESS }
/// The headset is plugged in: always, or while the file MW_XRSIM_PRESENT_FILE exists (deleting it pulls the cable).
fn plugged() -> bool { std::env::var_os("MW_XRSIM_PRESENT_FILE").is_none_or(|f| std::path::Path::new(&f).exists()) }

unsafe extern "system" fn get_system(_i: xr::Instance, info: *const xr::SystemGetInfo, out: *mut xr::SystemId) -> xr::Result {
    if (*info).form_factor != xr::FormFactor::HEAD_MOUNTED_DISPLAY { return xr::Result::ERROR_FORM_FACTOR_UNSUPPORTED; }
    if !plugged() { return xr::Result::ERROR_FORM_FACTOR_UNAVAILABLE; }
    *out = xr::SystemId::from_raw(1);
    xr::Result::SUCCESS
}
unsafe extern "system" fn system_properties(_i: xr::Instance, _id: xr::SystemId, out: *mut xr::SystemProperties) -> xr::Result {
    let o = &mut *out;
    o.system_id = xr::SystemId::from_raw(1);
    o.vendor_id = 0x2833;
    write_str(o.system_name.as_mut_ptr(), o.system_name.len(), "MadWorld XR Simulator (Quest 3 stand-in)");
    o.graphics_properties = xr::SystemGraphicsProperties { max_swapchain_image_height: 4096, max_swapchain_image_width: 4096, max_layer_count: 16 };
    o.tracking_properties = xr::SystemTrackingProperties { orientation_tracking: xr::TRUE, position_tracking: xr::TRUE };
    xr::Result::SUCCESS
}
unsafe extern "system" fn blend_modes(_i: xr::Instance, _id: xr::SystemId, _v: xr::ViewConfigurationType, cap: u32, count: *mut u32, out: *mut xr::EnvironmentBlendMode) -> xr::Result {
    *count = 1; if cap >= 1 { *out = xr::EnvironmentBlendMode::OPAQUE; } xr::Result::SUCCESS
}
unsafe extern "system" fn view_configurations(_i: xr::Instance, _id: xr::SystemId, cap: u32, count: *mut u32, out: *mut xr::ViewConfigurationType) -> xr::Result {
    *count = 1; if cap >= 1 { *out = xr::ViewConfigurationType::PRIMARY_STEREO; } xr::Result::SUCCESS
}
unsafe extern "system" fn view_configuration_properties(_i: xr::Instance, _id: xr::SystemId, v: xr::ViewConfigurationType, out: *mut xr::ViewConfigurationProperties) -> xr::Result {
    (*out).view_configuration_type = v; (*out).fov_mutable = xr::FALSE; xr::Result::SUCCESS
}
unsafe extern "system" fn view_configuration_views(_i: xr::Instance, _id: xr::SystemId, _v: xr::ViewConfigurationType, cap: u32, count: *mut u32, out: *mut xr::ViewConfigurationView) -> xr::Result {
    *count = 2;
    if cap == 0 { return xr::Result::SUCCESS; }
    for i in 0..2 {
        let o = &mut *out.add(i);
        o.recommended_image_rect_width = view_size().0; o.max_image_rect_width = 4096;
        o.recommended_image_rect_height = view_size().1; o.max_image_rect_height = 4096;
        o.recommended_swapchain_sample_count = 1; o.max_swapchain_sample_count = 4;
    }
    xr::Result::SUCCESS
}

// ---------------------------------------------------------------- Vulkan

unsafe extern "system" fn vulkan_requirements(_i: xr::Instance, _id: xr::SystemId, out: *mut xr::GraphicsRequirementsVulkanKHR) -> xr::Result {
    (*out).min_api_version_supported = xr::Version::new(1, 0, 0);
    (*out).max_api_version_supported = xr::Version::new(1, 3, 0);
    xr::Result::SUCCESS
}
unsafe extern "system" fn create_vulkan_instance(_i: xr::Instance, info: *const xr::VulkanInstanceCreateInfoKHR, out: *mut xr::platform::VkInstance, vk_result: *mut xr::platform::VkResult) -> xr::Result {
    let info = &*info;
    let gipa: vk::PFN_vkGetInstanceProcAddr = std::mem::transmute(info.pfn_get_instance_proc_addr.unwrap());
    let entry = ash::Entry::from_static_fn(ash::StaticFn { get_instance_proc_addr: gipa });
    let ci = &*(info.vulkan_create_info as *const vk::InstanceCreateInfo);
    match entry.create_instance(ci, None) {
        Ok(inst) => {
            *out = inst.handle().as_raw() as _;
            *vk_result = 0;
            with(|s| s.vk = Some(Vk { entry, instance: Some(inst), phys: vk::PhysicalDevice::null(), device: None, family: 0, queue_index: 0 }));
            eprintln!("[xrsim] Vulkan instance created for the app");
            xr::Result::SUCCESS
        }
        Err(e) => { *vk_result = e.as_raw(); xr::Result::SUCCESS }
    }
}
unsafe extern "system" fn vulkan_graphics_device(_i: xr::Instance, _info: *const xr::VulkanGraphicsDeviceGetInfoKHR, out: *mut xr::platform::VkPhysicalDevice) -> xr::Result {
    with(|s| {
        let Some(v) = s.vk.as_mut() else { return xr::Result::ERROR_VALIDATION_FAILURE; };
        let inst = v.instance.as_ref().unwrap();
        let devs = inst.enumerate_physical_devices().unwrap_or_default();
        let pick = devs.iter().copied().find(|d| inst.get_physical_device_properties(*d).device_type == vk::PhysicalDeviceType::DISCRETE_GPU).or(devs.first().copied());
        let Some(d) = pick else { return xr::Result::ERROR_RUNTIME_FAILURE; };
        v.phys = d;
        let name = inst.get_physical_device_properties(d).device_name_as_c_str().map(|c| c.to_string_lossy().into_owned()).unwrap_or_default();
        eprintln!("[xrsim] GPU: {name}");
        *out = d.as_raw() as _;
        xr::Result::SUCCESS
    })
}
unsafe extern "system" fn create_vulkan_device(_i: xr::Instance, info: *const xr::VulkanDeviceCreateInfoKHR, out: *mut xr::platform::VkDevice, vk_result: *mut xr::platform::VkResult) -> xr::Result {
    let info = &*info;
    with(|s| {
        let Some(v) = s.vk.as_mut() else { return xr::Result::ERROR_VALIDATION_FAILURE; };
        let inst = v.instance.as_ref().unwrap();
        let phys = vk::PhysicalDevice::from_raw(info.vulkan_physical_device as u64);
        let ci = &*(info.vulkan_create_info as *const vk::DeviceCreateInfo);
        match inst.create_device(phys, ci, None) {
            Ok(d) => { *out = d.handle().as_raw() as _; *vk_result = 0; v.phys = phys; v.device = Some(d); eprintln!("[xrsim] Vulkan device created for the app"); }
            Err(e) => { *vk_result = e.as_raw(); }
        }
        xr::Result::SUCCESS
    })
}

// ---------------------------------------------------------------- session

unsafe extern "system" fn create_session(_i: xr::Instance, info: *const xr::SessionCreateInfo, out: *mut xr::Session) -> xr::Result {
    if !plugged() { return xr::Result::ERROR_SYSTEM_INVALID; }
    // the Vulkan binding names the queue the app renders on
    let mut next = (*info).next as *const xr::BaseInStructure;
    let mut binding: Option<xr::GraphicsBindingVulkanKHR> = None;
    while !next.is_null() {
        if (*next).ty == xr::StructureType::GRAPHICS_BINDING_VULKAN_KHR { binding = Some(*(next as *const xr::GraphicsBindingVulkanKHR)); }
        next = (*next).next;
    }
    with(|s| {
        if let (Some(b), Some(v)) = (binding, s.vk.as_mut()) { v.family = b.queue_family_index; v.queue_index = b.queue_index; }
        let h = s.handle();
        s.session = h;
        *out = xr::Session::from_raw(h);
        eprintln!("[xrsim] session created (queue family {})", s.vk.as_ref().map_or(0, |v| v.family));
        s.set_state(xr::SessionState::IDLE);
        s.set_state(xr::SessionState::READY);
    });
    xr::Result::SUCCESS
}
unsafe extern "system" fn begin_session(_s: xr::Session, _info: *const xr::SessionBeginInfo) -> xr::Result {
    with(|s| { s.began = Some(Instant::now()); s.set_state(xr::SessionState::SYNCHRONIZED); s.set_state(xr::SessionState::VISIBLE); s.set_state(xr::SessionState::FOCUSED); });
    xr::Result::SUCCESS
}
unsafe extern "system" fn request_exit(_s: xr::Session) -> xr::Result {
    with(|s| { s.set_state(xr::SessionState::VISIBLE); s.set_state(xr::SessionState::SYNCHRONIZED); s.set_state(xr::SessionState::STOPPING); });
    xr::Result::SUCCESS
}
unsafe extern "system" fn end_session(_s: xr::Session) -> xr::Result {
    if with(|s| s.dropped) { with(|s| s.set_state(xr::SessionState::IDLE)); return xr::Result::ERROR_SESSION_NOT_RUNNING; }
    with(|s| { s.set_state(xr::SessionState::IDLE); s.set_state(xr::SessionState::EXITING); });
    xr::Result::SUCCESS
}
unsafe extern "system" fn reference_spaces(_s: xr::Session, cap: u32, count: *mut u32, out: *mut xr::ReferenceSpaceType) -> xr::Result {
    let all = [xr::ReferenceSpaceType::VIEW, xr::ReferenceSpaceType::LOCAL, xr::ReferenceSpaceType::STAGE];
    *count = 3;
    if cap >= 3 { for (i, t) in all.iter().enumerate() { *out.add(i) = *t; } }
    xr::Result::SUCCESS
}
unsafe extern "system" fn create_reference_space(_s: xr::Session, info: *const xr::ReferenceSpaceCreateInfo, out: *mut xr::Space) -> xr::Result {
    let t = (*info).reference_space_type;
    with(|s| { let h = s.handle(); s.spaces.insert(h, SpaceKind::Reference(t)); *out = xr::Space::from_raw(h); });
    xr::Result::SUCCESS
}
unsafe extern "system" fn bounds_rect(_s: xr::Session, _t: xr::ReferenceSpaceType, out: *mut xr::Extent2Df) -> xr::Result {
    *out = xr::Extent2Df { width: 2.5, height: 2.5 }; xr::Result::SUCCESS
}
unsafe extern "system" fn create_action_space(_s: xr::Session, info: *const xr::ActionSpaceCreateInfo, out: *mut xr::Space) -> xr::Result {
    let i = &*info;
    with(|s| {
        let h = s.handle();
        s.spaces.insert(h, SpaceKind::Action { action: i.action.into_raw(), sub: i.subaction_path.into_raw(), offset: pose_from(&i.pose_in_action_space) });
        *out = xr::Space::from_raw(h);
    });
    xr::Result::SUCCESS
}
unsafe extern "system" fn destroy_space(sp: xr::Space) -> xr::Result { with(|s| s.spaces.remove(&sp.into_raw())); xr::Result::SUCCESS }
unsafe extern "system" fn locate_space(space: xr::Space, base: xr::Space, time: xr::Time, out: *mut xr::SpaceLocation) -> xr::Result {
    let t = time.as_nanos();
    let (p, b, v) = with(|s| {
        let p = s.space_pose(space.into_raw(), t); let b = s.space_pose(base.into_raw(), t);
        // velocity by a central difference over 5 ms
        let v = match (s.space_pose(space.into_raw(), t + 5_000_000), s.space_pose(space.into_raw(), t - 5_000_000)) {
            (Some(a), Some(c)) => Some(sub(a.p, c.p).map(|x| x / 0.01)), _ => None };
        (p, b, v)
    });
    let o = &mut *out;
    match (p, b) {
        (Some(p), Some(b)) => {
            o.location_flags = xr::SpaceLocationFlags::POSITION_VALID | xr::SpaceLocationFlags::ORIENTATION_VALID | xr::SpaceLocationFlags::POSITION_TRACKED | xr::SpaceLocationFlags::ORIENTATION_TRACKED;
            o.pose = to_posef(relative(b, p));
        }
        _ => { o.location_flags = xr::SpaceLocationFlags::EMPTY; }
    }
    // a velocity chained after the location
    let mut next = o.next as *mut xr::BaseOutStructure;
    while !next.is_null() {
        if (*next).ty == xr::StructureType::SPACE_VELOCITY {
            let sv = &mut *(next as *mut xr::SpaceVelocity);
            if let (Some(v), Some(b)) = (v, b) {
                let lv = qrot(qinv(b.q), v);
                sv.velocity_flags = xr::SpaceVelocityFlags::LINEAR_VALID;
                sv.linear_velocity = xr::Vector3f { x: lv[0], y: lv[1], z: lv[2] };
                sv.angular_velocity = xr::Vector3f { x: 0., y: 0., z: 0. };
            } else { sv.velocity_flags = xr::SpaceVelocityFlags::EMPTY; }
        }
        next = (*next).next;
    }
    xr::Result::SUCCESS
}
unsafe extern "system" fn locate_views(_s: xr::Session, info: *const xr::ViewLocateInfo, state: *mut xr::ViewState, cap: u32, count: *mut u32, out: *mut xr::View) -> xr::Result {
    *count = 2;
    if cap == 0 { return xr::Result::SUCCESS; }
    let i = &*info;
    let t = i.display_time.as_nanos();
    let (head, base) = with(|s| (s.input(t).head, s.space_pose(i.space.into_raw(), t).unwrap_or(Pose { p: [0.; 3], q: [0., 0., 0., 1.] })));
    (*state).view_state_flags = xr::ViewStateFlags::POSITION_VALID | xr::ViewStateFlags::ORIENTATION_VALID | xr::ViewStateFlags::POSITION_TRACKED | xr::ViewStateFlags::ORIENTATION_TRACKED;
    for e in 0..2usize {
        let side = if e == 0 { -0.5 } else { 0.5 };
        let eye = Pose { p: add(head.p, qrot(head.q, [side * IPD, 0., 0.])), q: head.q };
        let v = &mut *out.add(e);
        v.pose = to_posef(relative(base, eye));
        let a = 45f32.to_radians();
        v.fov = xr::Fovf { angle_left: -a, angle_right: a, angle_up: a, angle_down: -a };
    }
    xr::Result::SUCCESS
}

// ---------------------------------------------------------------- swapchains

unsafe extern "system" fn swapchain_formats(_s: xr::Session, cap: u32, count: *mut u32, out: *mut i64) -> xr::Result {
    let f = [vk::Format::R8G8B8A8_SRGB.as_raw() as i64, vk::Format::B8G8R8A8_SRGB.as_raw() as i64, vk::Format::R8G8B8A8_UNORM.as_raw() as i64];
    *count = f.len() as u32;
    if cap as usize >= f.len() { for (i, x) in f.iter().enumerate() { *out.add(i) = *x; } }
    xr::Result::SUCCESS
}
unsafe extern "system" fn create_swapchain(_s: xr::Session, info: *const xr::SwapchainCreateInfo, out: *mut xr::Swapchain) -> xr::Result {
    let i = &*info;
    with(|s| {
        let Some(v) = s.vk.as_ref() else { return xr::Result::ERROR_GRAPHICS_DEVICE_INVALID; };
        let (Some(dev), Some(inst)) = (v.device.as_ref(), v.instance.as_ref()) else { return xr::Result::ERROR_GRAPHICS_DEVICE_INVALID; };
        let format = vk::Format::from_raw(i.format as i32);
        let mem_props = inst.get_physical_device_memory_properties(v.phys);
        let mut images = vec![]; let mut memory = vec![];
        for _ in 0..3 {
            let ci = vk::ImageCreateInfo::default().image_type(vk::ImageType::TYPE_2D).format(format)
                .extent(vk::Extent3D { width: i.width, height: i.height, depth: 1 }).mip_levels(i.mip_count.max(1)).array_layers(i.array_size.max(1))
                .samples(vk::SampleCountFlags::TYPE_1).tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST)
                .initial_layout(vk::ImageLayout::UNDEFINED);
            let Ok(img) = dev.create_image(&ci, None) else { return xr::Result::ERROR_RUNTIME_FAILURE; };
            let req = dev.get_image_memory_requirements(img);
            let ty = (0..mem_props.memory_type_count).find(|t| req.memory_type_bits & (1 << t) != 0
                && mem_props.memory_types[*t as usize].property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)).unwrap_or(0);
            let Ok(mem) = dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(req.size).memory_type_index(ty), None) else { return xr::Result::ERROR_RUNTIME_FAILURE; };
            let _ = dev.bind_image_memory(img, mem, 0);
            images.push(img); memory.push(mem);
        }
        let h = s.handle();
        eprintln!("[xrsim] swapchain {h}: {}x{} x{} layers, {:?}", i.width, i.height, i.array_size, format);
        s.swaps.insert(h, Swap { images, memory, w: i.width, h: i.height, layers: i.array_size.max(1), format, next: 0, last: 0 });
        *out = xr::Swapchain::from_raw(h);
        xr::Result::SUCCESS
    })
}
unsafe extern "system" fn destroy_swapchain(sc: xr::Swapchain) -> xr::Result {
    with(|s| {
        if let (Some(sw), Some(dev)) = (s.swaps.remove(&sc.into_raw()), s.vk.as_ref().and_then(|v| v.device.as_ref())) {
            let _ = dev.device_wait_idle();
            for (i, m) in sw.images.iter().zip(&sw.memory) { dev.destroy_image(*i, None); dev.free_memory(*m, None); }
        }
    });
    xr::Result::SUCCESS
}
unsafe extern "system" fn swapchain_images(sc: xr::Swapchain, cap: u32, count: *mut u32, out: *mut xr::SwapchainImageBaseHeader) -> xr::Result {
    with(|s| {
        let Some(sw) = s.swaps.get(&sc.into_raw()) else { return xr::Result::ERROR_HANDLE_INVALID; };
        *count = sw.images.len() as u32;
        if cap == 0 { return xr::Result::SUCCESS; }
        let out = out as *mut xr::SwapchainImageVulkanKHR;
        for (i, img) in sw.images.iter().enumerate() { (*out.add(i)).image = img.as_raw() as _; }
        xr::Result::SUCCESS
    })
}
unsafe extern "system" fn acquire_image(sc: xr::Swapchain, _info: *const u8, index: *mut u32) -> xr::Result {
    with(|s| { let Some(sw) = s.swaps.get_mut(&sc.into_raw()) else { return xr::Result::ERROR_HANDLE_INVALID; };
               *index = sw.next; sw.next = (sw.next + 1) % sw.images.len() as u32; xr::Result::SUCCESS })
}
unsafe extern "system" fn release_image(sc: xr::Swapchain, _info: *const u8) -> xr::Result {
    with(|s| { if let Some(sw) = s.swaps.get_mut(&sc.into_raw()) { sw.last = (sw.next + sw.images.len() as u32 - 1) % sw.images.len() as u32; } });
    xr::Result::SUCCESS
}

// ---------------------------------------------------------------- frames

unsafe extern "system" fn wait_frame(_s: xr::Session, _info: *const u8, out: *mut xr::FrameState) -> xr::Result {
    // pace to 72 Hz
    let sleep = with(|s| {
        let now = Instant::now();
        let due = s.last_wait.map(|l| l + Duration::from_nanos(PERIOD_NS as u64));
        s.last_wait = Some(due.filter(|d| *d > now).unwrap_or(now));
        due.and_then(|d| d.checked_duration_since(now))
    });
    // (MW_XRSIM_NOPACE: no 72 Hz pacing, frames as fast as the app makes them: measures its real frame cost)
    if let Some(d) = sleep.filter(|_| std::env::var_os("MW_XRSIM_NOPACE").is_none()) { std::thread::sleep(d); }
    let t = with(|s| s.now_ns());
    let o = &mut *out;
    o.predicted_display_time = xr::Time::from_nanos(t + PERIOD_NS * 2);
    o.predicted_display_period = xr::Duration::from_nanos(PERIOD_NS);
    o.should_render = xr::TRUE;
    xr::Result::SUCCESS
}
unsafe extern "system" fn end_frame(_s: xr::Session, info: *const xr::FrameEndInfo) -> xr::Result {
    let i = &*info;
    let frame = with(|s| { s.frame += 1; s.frame });
    if frame % 72 == 0 {
        let fps = with(|s| { let e = s.began.map_or(1., |b| b.elapsed().as_secs_f64()); frame as f64 / e.max(1e-3) });
        eprintln!("[xrsim] frame {frame} ({} layers, {fps:.1} fps since the session began)", i.layer_count);
    }
    let want = with(|s| s.dump.0.contains(&frame));
    if want && i.layer_count > 0 {
        let layer = &**i.layers;
        if layer.ty == xr::StructureType::COMPOSITION_LAYER_PROJECTION {
            let proj = &*(*i.layers as *const xr::CompositionLayerProjection);
            let views = std::slice::from_raw_parts(proj.views, proj.view_count as usize);
            dump(frame, views);
        }
    }
    xr::Result::SUCCESS
}

/// Reads the projection layer's two views back and writes them side by side as a PNG.
unsafe fn dump(frame: u64, views: &[xr::CompositionLayerProjectionView]) {
    with(|s| {
        let dir = s.dump.1.clone();
        let Some(v) = s.vk.as_ref() else { return; };
        let (Some(dev), Some(inst)) = (v.device.as_ref(), v.instance.as_ref()) else { return; };
        let queue = dev.get_device_queue(v.family, v.queue_index);
        let mem_props = inst.get_physical_device_memory_properties(v.phys);
        let mut eyes: Vec<(u32, u32, Vec<u8>)> = vec![];
        for view in views {
            let Some(sw) = s.swaps.get(&view.sub_image.swapchain.into_raw()) else { continue; };
            let img = sw.images[sw.last as usize];
            let (w, h) = (view.sub_image.image_rect.extent.width as u32, view.sub_image.image_rect.extent.height as u32);
            let size = (w * h * 4) as u64;
            let buf = dev.create_buffer(&vk::BufferCreateInfo::default().size(size).usage(vk::BufferUsageFlags::TRANSFER_DST), None).unwrap();
            let req = dev.get_buffer_memory_requirements(buf);
            let ty = (0..mem_props.memory_type_count).find(|t| req.memory_type_bits & (1 << t) != 0
                && mem_props.memory_types[*t as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)).unwrap();
            let mem = dev.allocate_memory(&vk::MemoryAllocateInfo::default().allocation_size(req.size).memory_type_index(ty), None).unwrap();
            dev.bind_buffer_memory(buf, mem, 0).unwrap();
            let pool = dev.create_command_pool(&vk::CommandPoolCreateInfo::default().queue_family_index(v.family), None).unwrap();
            let cb = dev.allocate_command_buffers(&vk::CommandBufferAllocateInfo::default().command_pool(pool).command_buffer_count(1)).unwrap()[0];
            dev.begin_command_buffer(cb, &vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)).unwrap();
            let layer = view.sub_image.image_array_index;
            let range = vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: layer, layer_count: 1 };
            let to_src = vk::ImageMemoryBarrier::default().image(img).subresource_range(range).old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL).new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE).dst_access_mask(vk::AccessFlags::TRANSFER_READ);
            dev.cmd_pipeline_barrier(cb, vk::PipelineStageFlags::ALL_COMMANDS, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[to_src]);
            let region = vk::BufferImageCopy::default().image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: layer, layer_count: 1 })
                .image_offset(vk::Offset3D { x: view.sub_image.image_rect.offset.x, y: view.sub_image.image_rect.offset.y, z: 0 })
                .image_extent(vk::Extent3D { width: w, height: h, depth: 1 });
            dev.cmd_copy_image_to_buffer(cb, img, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, buf, &[region]);
            let back = vk::ImageMemoryBarrier::default().image(img).subresource_range(range).old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL).new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .src_access_mask(vk::AccessFlags::TRANSFER_READ).dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);
            dev.cmd_pipeline_barrier(cb, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::ALL_COMMANDS, vk::DependencyFlags::empty(), &[], &[], &[back]);
            dev.end_command_buffer(cb).unwrap();
            let _ = dev.queue_wait_idle(queue);
            dev.queue_submit(queue, &[vk::SubmitInfo::default().command_buffers(&[cb])], vk::Fence::null()).unwrap();
            let _ = dev.queue_wait_idle(queue);
            let ptr = dev.map_memory(mem, 0, size, vk::MemoryMapFlags::empty()).unwrap() as *const u8;
            let mut px = std::slice::from_raw_parts(ptr, size as usize).to_vec();
            dev.unmap_memory(mem);
            if sw.format == vk::Format::B8G8R8A8_SRGB { for c in px.chunks_mut(4) { c.swap(0, 2); } }
            dev.destroy_command_pool(pool, None); dev.destroy_buffer(buf, None); dev.free_memory(mem, None);
            eyes.push((w, h, px));
        }
        if eyes.len() == 2 && eyes[0].0 == eyes[1].0 && eyes[0].1 == eyes[1].1 {
            let (w, h) = (eyes[0].0, eyes[0].1);
            let mut rgb = Vec::with_capacity((w * 2 * h * 3) as usize);
            for y in 0..h { for e in &eyes { for x in 0..w { let i = ((y * w + x) * 4) as usize; rgb.extend_from_slice(&e.2[i..i + 3]); } } }
            let path = format!("{dir}/xr_frame_{frame:05}.png");
            match std::fs::write(&path, png(w * 2, h, &rgb)) { Ok(_) => eprintln!("[xrsim] wrote {path}"), Err(e) => eprintln!("[xrsim] {path}: {e}") }
        }
    });
}

/// A minimal PNG (RGB8, stored deflate blocks).
fn png(w: u32, h: u32, rgb: &[u8]) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xffff_ffffu32;
        for b in data { c ^= *b as u32; for _ in 0..8 { c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 }; } }
        !c
    }
    let mut raw = Vec::with_capacity((w * 3 + 1) as usize * h as usize);
    for y in 0..h as usize { raw.push(0); raw.extend_from_slice(&rgb[y * w as usize * 3..(y + 1) * w as usize * 3]); }
    let mut z = vec![0x78, 0x01];
    for (i, chunk) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(last as u8); z.extend_from_slice(&(chunk.len() as u16).to_le_bytes()); z.extend_from_slice(&(!(chunk.len() as u16)).to_le_bytes()); z.extend_from_slice(chunk);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for x in &raw { a = (a + *x as u32) % 65521; b = (b + a) % 65521; }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |ty: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut c = ty.to_vec(); c.extend_from_slice(data);
        out.extend_from_slice(&c); out.extend_from_slice(&crc(&c).to_be_bytes());
    };
    let mut ihdr = vec![]; ihdr.extend_from_slice(&w.to_be_bytes()); ihdr.extend_from_slice(&h.to_be_bytes()); ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr); chunk(b"IDAT", &z); chunk(b"IEND", &[]);
    out
}

// ---------------------------------------------------------------- paths and actions

unsafe extern "system" fn string_to_path(_i: xr::Instance, s: *const c_char, out: *mut xr::Path) -> xr::Result {
    let p = with(|st| st.path(cstr(s))); *out = xr::Path::from_raw(p); xr::Result::SUCCESS
}
unsafe extern "system" fn path_to_string(_i: xr::Instance, p: xr::Path, cap: u32, count: *mut u32, out: *mut c_char) -> xr::Result {
    let s = with(|st| st.path_str(p.into_raw()).to_string());
    *count = s.len() as u32 + 1;
    if cap as usize > s.len() { write_str(out, cap as usize, &s); }
    xr::Result::SUCCESS
}
unsafe extern "system" fn create_action_set(_i: xr::Instance, info: *const xr::ActionSetCreateInfo, out: *mut xr::ActionSet) -> xr::Result {
    let name = cstr((*info).action_set_name.as_ptr()).to_string();
    with(|s| { let h = s.handle(); s.sets.insert(h, name); *out = xr::ActionSet::from_raw(h); });
    xr::Result::SUCCESS
}
unsafe extern "system" fn create_action(_set: xr::ActionSet, info: *const xr::ActionCreateInfo, out: *mut xr::Action) -> xr::Result {
    let i = &*info;
    let name = cstr(i.action_name.as_ptr()).to_string();
    with(|s| { let h = s.handle(); s.actions.insert(h, ActionInfo { name, ty: i.action_type }); *out = xr::Action::from_raw(h); });
    xr::Result::SUCCESS
}
unsafe extern "system" fn suggest_bindings(_i: xr::Instance, info: *const xr::InteractionProfileSuggestedBinding) -> xr::Result {
    let i = &*info;
    with(|s| {
        let profile = s.path_str(i.interaction_profile.into_raw()).to_string();
        if profile != "/interaction_profiles/oculus/touch_controller" { return; }
        let list = std::slice::from_raw_parts(i.suggested_bindings, i.count_suggested_bindings as usize);
        for b in list {
            let p = s.path_str(b.binding.into_raw()).to_string();
            s.bindings.entry(b.action.into_raw()).or_default().push(p);
        }
        eprintln!("[xrsim] {} Touch bindings suggested", list.len());
    });
    xr::Result::SUCCESS
}
unsafe extern "system" fn attach_sets(_s: xr::Session, _info: *const u8) -> xr::Result {
    with(|s| { let n: Vec<String> = s.actions.values().map(|a| format!("{}:{:?}", a.name, a.ty)).collect(); eprintln!("[xrsim] action sets attached ({} actions: {})", n.len(), n.join(", ")); });
    xr::Result::SUCCESS
}
unsafe extern "system" fn current_profile(_s: xr::Session, _user: xr::Path, out: *mut xr::InteractionProfileState) -> xr::Result {
    let p = with(|s| s.path("/interaction_profiles/oculus/touch_controller"));
    (*out).interaction_profile = xr::Path::from_raw(p);
    xr::Result::SUCCESS
}
/// An action's value now: (active, float, bool, vec2).
fn value(s: &State, info: &xr::ActionStateGetInfo) -> (bool, f32, bool, [f32; 2]) {
    let Some((hand, src)) = s.source(info.action.into_raw(), info.subaction_path.into_raw()) else { return (false, 0., false, [0.; 2]); };
    let t = s.now_ns();
    let h = s.input(t).hands[hand];
    let f = match src.as_str() {
        "input/squeeze/value" => h.squeeze, "input/trigger/value" => h.trigger,
        "input/trigger/touch" => (h.trigger > 0.) as u8 as f32, "input/thumbstick/click" => h.stick_click as u8 as f32,
        "input/thumbstick/touch" => (h.stick != [0.; 2] || h.a || h.b) as u8 as f32,
        "input/a/click" | "input/x/click" => h.a as u8 as f32, "input/b/click" | "input/y/click" => h.b as u8 as f32, "input/menu/click" => h.menu as u8 as f32,
        _ => 0.,
    };
    (true, f, f > 0.5, if src == "input/thumbstick" { h.stick } else { [0.; 2] })
}
unsafe extern "system" fn state_bool(_s: xr::Session, info: *const xr::ActionStateGetInfo, out: *mut xr::ActionStateBoolean) -> xr::Result {
    let (a, _, b, _) = with(|s| value(s, &*info));
    let o = &mut *out; o.current_state = b.into(); o.is_active = a.into(); o.changed_since_last_sync = xr::FALSE; o.last_change_time = xr::Time::from_nanos(0);
    xr::Result::SUCCESS
}
unsafe extern "system" fn state_float(_s: xr::Session, info: *const xr::ActionStateGetInfo, out: *mut xr::ActionStateFloat) -> xr::Result {
    let (a, f, _, _) = with(|s| value(s, &*info));
    let o = &mut *out; o.current_state = f; o.is_active = a.into(); o.changed_since_last_sync = xr::FALSE; o.last_change_time = xr::Time::from_nanos(0);
    xr::Result::SUCCESS
}
unsafe extern "system" fn state_vec2(_s: xr::Session, info: *const xr::ActionStateGetInfo, out: *mut xr::ActionStateVector2f) -> xr::Result {
    let (a, _, _, v) = with(|s| value(s, &*info));
    let o = &mut *out; o.current_state = xr::Vector2f { x: v[0], y: v[1] }; o.is_active = a.into(); o.changed_since_last_sync = xr::FALSE; o.last_change_time = xr::Time::from_nanos(0);
    xr::Result::SUCCESS
}
unsafe extern "system" fn state_pose(_s: xr::Session, info: *const xr::ActionStateGetInfo, out: *mut xr::ActionStatePose) -> xr::Result {
    let a = with(|s| s.source((*info).action.into_raw(), (*info).subaction_path.into_raw()).is_some());
    (*out).is_active = a.into();
    xr::Result::SUCCESS
}
unsafe extern "system" fn bound_sources(_s: xr::Session, _info: *const u8, _cap: u32, count: *mut u32, _out: *mut xr::Path) -> xr::Result { *count = 0; xr::Result::SUCCESS }
unsafe extern "system" fn localized_name(_s: xr::Session, _info: *const u8, cap: u32, count: *mut u32, out: *mut c_char) -> xr::Result {
    *count = 1; if cap >= 1 { *out = 0; } xr::Result::SUCCESS
}
unsafe extern "system" fn haptic(_s: xr::Session, info: *const xr::HapticActionInfo, event: *const xr::HapticBaseHeader) -> xr::Result {
    let hand = with(|s| s.path_str((*info).subaction_path.into_raw()).to_string());
    if (*event).ty == xr::StructureType::HAPTIC_VIBRATION {
        let v = &*(event as *const xr::HapticVibration);
        eprintln!("[xrsim] haptic {hand} amplitude {:.2} for {:.0} ms", v.amplitude, v.duration.as_nanos() as f64 / 1e6);
    }
    xr::Result::SUCCESS
}
