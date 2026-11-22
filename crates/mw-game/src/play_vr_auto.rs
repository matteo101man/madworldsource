//! VR on its own [added]: with a headset connected the game starts in VR, without one it starts flat. A session that
//! drops (Link ends or hiccups, the headset is unplugged) switches to flat on the spot and back to VR when it runs
//! again, in place: the game keeps running, only the cameras change (play_vr.rs `apply_mode`). A game started flat has
//! no headset renderer (the GPU device is made through OpenXR when a headset is there at launch), so a headset
//! connected to it restarts the game on the same stage instead. `MW_VR=1 / sim / 0` pins a mode and turns this off.
//!
//! How "connected" is told: Meta's runtime (Quest Link / Air Link) must not be asked through a throwaway OpenXR
//! instance — every instance it sees launches Link for the app and every one that closes makes Link leave VR, which
//! ends the headset's Link session ("it closes the Link"). So with Meta's runtime the game starts through OpenXR
//! straight away (its one real instance: a linked headset plays in VR, none leaves it on the monitor in place), and
//! while on the monitor it reads Meta's own service log for the Link session's state (the last
//! `oculus_xrs_session_state_transition`: a `streaming` session `Connected` is a headset in Link or Air Link). Other
//! runtimes (Virtual Desktop, SteamVR, the test runtime) are asked through OpenXR: an instance that finds a
//! head-mounted system.
use bevy::prelude::*;
use bevy_mod_xr::session::{XrCreateSessionMessage, XrState};
use std::sync::atomic::{AtomicBool, Ordering};

fn log_probe(e: &dyn std::fmt::Display) { if std::env::var_os("MW_VR_PROBE_LOG").is_some() { eprintln!("[vr] headset probe: {e}"); } }

/// A process of that image name is running (Windows: tasklist, no console window).
fn running(image: &str) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("tasklist").args(["/FI", &format!("IMAGENAME eq {image}"), "/NH", "/FO", "CSV"])
            .creation_flags(0x0800_0000).output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains(&image.to_ascii_lowercase())).unwrap_or(false)
    }
    #[cfg(not(windows))]
    { let _ = image; false }
}

/// The OpenXR runtime this process will load is Meta's (no XR_RUNTIME_JSON override, the registry's active runtime
/// is the Oculus one).
fn meta_runtime() -> bool {
    if std::env::var_os("XR_RUNTIME_JSON").is_some() { return false; }
    static META: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *META.get_or_init(|| {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("reg").args(["query", r"HKLM\SOFTWARE\Khronos\OpenXR\1", "/v", "ActiveRuntime"])
                .creation_flags(0x0800_0000).output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains("oculus")).unwrap_or(false)
        }
        #[cfg(not(windows))]
        { false }
    })
}

/// Meta's service log (the newest %LOCALAPPDATA%\Oculus\Service_*.txt) says a headset is in Link / Air Link: its
/// last Link session state is a `streaming` session `Connected`. None: no log, or no Link session recorded yet.
fn meta_linked() -> Option<bool> {
    let dir = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Oculus");
    let newest = std::fs::read_dir(&dir).ok()?.filter_map(|e| e.ok())
        .filter(|e| { let n = e.file_name().to_string_lossy().to_string(); n.starts_with("Service_") && n.ends_with(".txt") })
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())?.path();
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(&newest).ok()?;
    let len = f.metadata().ok()?.len();
    // the tail first (the log grows by a few MB an hour), the whole file if the tail has no Link state
    for back in [2u64 << 20, len] {
        let from = len.saturating_sub(back);
        f.seek(SeekFrom::Start(from)).ok()?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).ok()?;
        let text = String::from_utf8_lossy(&buf);
        if let Some(line) = text.lines().rev().find(|l| l.contains("oculus_xrs_session_state_transition")) {
            return Some(line.contains("current_session_type=streaming") && line.contains("current_state=Connected"));
        }
        if from == 0 { break; }
    }
    None
}

/// A headset is there to play on. Meta's runtime: its log says Link is up (no OpenXR call: see above). Any other: a
/// throwaway OpenXR instance finds a head-mounted system.
pub fn headset_present() -> bool {
    if meta_runtime() {
        let linked = meta_linked().unwrap_or(false);
        if !linked { log_probe(&"Meta runtime: no headset in Link (Meta's log shows no Link session up)"); }
        return linked;
    }
    // the loader is linked in on Windows (as the session's own), loaded at run time elsewhere
    #[cfg(windows)] let entry = openxr::Entry::linked();
    #[cfg(not(windows))] let entry = match unsafe { openxr::Entry::load() } { Ok(e) => e, Err(e) => { log_probe(&e); return false; } };
    let app = openxr::ApplicationInfo { application_name: "MadWorld headset probe", application_version: 0, engine_name: "MadWorld", engine_version: 0,
                                        api_version: openxr::Version::new(1, 0, 0) };
    let instance = match entry.create_instance(&app, &openxr::ExtensionSet::default(), &[]) { Ok(i) => i, Err(e) => { log_probe(&e); return false; } };
    match instance.system(openxr::FormFactor::HEAD_MOUNTED_DISPLAY) { Ok(_) => true, Err(e) => { log_probe(&e); false } }
}

/// Whether this run follows the headset (MW_VR unset or "auto", or a run the headset itself started).
pub fn auto() -> bool { matches!(std::env::var("MW_VR").as_deref(), Err(_) | Ok("auto")) || std::env::var_os("MW_VR_AUTO").is_some() }

/// At launch, before anything asks for the mode: VR if a headset is there.
pub fn decide() {
    if !matches!(std::env::var("MW_VR").as_deref(), Err(_) | Ok("auto")) { return; }
    std::env::set_var("MW_VR_AUTO", "1");
    // Meta's runtime: straight through OpenXR (its single real instance answers; no headset: the monitor, in place)
    if std::env::var_os("MW_VR_HOLD_FLAT").is_none() && meta_runtime() && running("OVRServer_x64.exe") {
        eprintln!("[vr] Meta runtime: starting through OpenXR (VR if a headset is in Link)"); std::env::set_var("MW_VR", "1"); return;
    }
    if std::env::var_os("MW_VR_HOLD_FLAT").is_none() && headset_present() { eprintln!("[vr] headset found: starting in VR"); std::env::set_var("MW_VR", "1"); }
    else { eprintln!("[vr] no headset: starting flat (connecting one switches to VR)"); std::env::remove_var("MW_VR"); }
}

/// Restart on the same stage: in VR straight away (the headset was just seen: no second look, which on Meta's runtime
/// would cost the Link session), or flat, held there until the headset has gone and come back.
pub fn restart(vr: bool) -> ! {
    let exe = std::env::current_exe().expect("current exe");
    let mut cmd = std::process::Command::new(exe);
    cmd.env("MW_VR_AUTO", "1");
    if vr { cmd.env("MW_VR", "1").env("MW_VR_RELAUNCHED", "1").env_remove("MW_VR_HOLD_FLAT"); } else { cmd.env_remove("MW_VR").env("MW_VR_HOLD_FLAT", "1"); }
    eprintln!("[vr] restarting {}", if vr { "in VR (headset connected)" } else { "flat (the headset would not start)" });
    match cmd.spawn() { Ok(_) => std::process::exit(0), Err(e) => { eprintln!("[vr] restart failed: {e}"); std::process::exit(1) } }
}

static SWITCH: AtomicBool = AtomicBool::new(false);

/// Started flat (and following the headset): a watcher looks every few seconds; a headset that appears restarts the
/// game in VR (a run held flat waits to see it gone first).
pub fn watch_flat() {
    if !auto() || super::built() { return; }
    watch_headset();
}
fn watch_headset() {
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::Relaxed) { return; }
    let mut seen_gone = std::env::var_os("MW_VR_HOLD_FLAT").is_none();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(4));
        let present = headset_present();
        if !present { seen_gone = true; }
        if present && seen_gone { SWITCH.store(true, Ordering::Relaxed); break; }
    });
}

pub struct AutoPlugin;
impl Plugin for AutoPlugin {
    fn build(&self, app: &mut App) {
        if !auto() { return; }
        match super::mode() {
            super::Mode::OpenXr => { app.add_systems(PreUpdate, watch_vr.before(super::VrInputSet)); }
            super::Mode::Off => { watch_flat(); app.add_systems(Update, switch_flat); }
            super::Mode::Sim => {}
        }
    }
}

/// Started flat: the watcher saw the headset come.
fn switch_flat() { poll_switch(); }
pub fn poll_switch() { if SWITCH.load(Ordering::Relaxed) { restart(true); } }

/// Started in VR. A session that stops running for 2 s (Link drops or hiccups, the headset is unplugged) → flat in
/// place; running again (the runtime brings the session back itself, or a new one starts once the headset answers)
/// → VR. A headset that never runs a session in 25 s also plays flat until it does. OpenXR that could not start at all
/// (no headset after all) → restart flat.
fn watch_vr(state: Option<Res<XrState>>, session: Option<Res<bevy_mod_openxr::session::OxrSession>>,
            instance: Option<Res<bevy_mod_openxr::resources::OxrInstance>>, time: Res<Time<Real>>,
            mut ran: Local<bool>, mut since: Local<Option<f64>>, mut next_try: Local<f64>, mut create: MessageWriter<XrCreateSessionMessage>) {
    let now = time.elapsed_secs_f64();
    let st = state.map(|s| *s);
    if st == Some(XrState::Unavailable) {
        // OpenXR found no headset at launch: play on the monitor here; a headset that comes later restarts into VR
        if super::on() {
            eprintln!("[vr] no headset at launch: on the monitor (connecting one switches to VR)");
            // relaunched for a headset that still would not open: wait for it to go and come back (no relaunch loop)
            if std::env::var_os("MW_VR_RELAUNCHED").is_some() { std::env::set_var("MW_VR_HOLD_FLAT", "1"); }
            super::set_active(false); watch_headset();
        }
        poll_switch();
        return;
    }
    if st == Some(XrState::Running) {
        *ran = true; *since = None;
        if !super::on() { eprintln!("[vr] headset back: VR"); super::set_active(true); }
        return;
    }
    if super::on() {
        let t0 = *since.get_or_insert(now);
        if (*ran && now - t0 > 2.) || (!*ran && now > 25.) {
            eprintln!("[vr] headset {} ({st:?}): flat until it runs again", if *ran { "session stopped" } else { "idle" });
            super::set_active(false); *next_try = now + 1.;
        }
        return;
    }
    // flat with no session at all: ask this instance (no new one) now and then; a new session when it answers
    if st == Some(XrState::Available) && session.is_none() && now >= *next_try {
        *next_try = now + 1.5;
        if instance.is_some_and(|i| i.system(openxr::FormFactor::HEAD_MOUNTED_DISPLAY).is_ok()) {
            eprintln!("[vr] headset connected: starting its session"); create.write_default();
        }
    }
}
