//! One native CPU instance, with immutable options and cooperative terminal stop.
use crate::native_audio::AudioBridge;
use mw_native_platform::{host, native_hardware::host::RenderSink};
use std::{
    ffi::{c_char, c_void, CStr, CString},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};
type Stop = unsafe extern "C" fn();
#[derive(Default)]
struct State {
    stop: Option<Stop>,
    status: String,
}
pub struct Session {
    root: PathBuf,
    state: Mutex<State>,
    worker: Mutex<Option<JoinHandle<()>>>,
    closing: AtomicBool,
}
impl Session {
    pub fn new(root: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            root,
            state: Mutex::new(State {
                status: "Starting MadWorld窶ｦ".into(),
                ..Default::default()
            }),
            worker: Mutex::new(None),
            closing: AtomicBool::new(false),
        })
    }
    pub fn status(&self) -> String {
        self.state.lock().unwrap().status.clone()
    }
    pub fn start(self: &Arc<Self>, render: Box<dyn RenderSink>, audio: AudioBridge) {
        let mut worker = self.worker.lock().unwrap();
        if worker.is_some() || self.closing.load(Ordering::Acquire) {
            return;
        }
        let session = self.clone();
        *worker = Some(std::thread::spawn(move || {
            let done = audio.clone();
            let result = unsafe { session.run(render, audio) };
            host::clear();
            done.finish();
            if let Err(error) = result {
                bevy::log::error!("Original core: {error}");
                session.state.lock().unwrap().status = "Unable to start MadWorld".into();
            }
        }));
    }
    unsafe fn run(&self, render: Box<dyn RenderSink>, audio: AudioBridge) -> Result<(), String> {
        let library = unsafe {
            // MW_CORE_DLL: alternate core build (optimization experiments).
            libloading::Library::new(std::env::var_os("MW_CORE_DLL").map(PathBuf::from).unwrap_or_else(|| {
                self.root
                    .join("tools/upstream-native-core/port/build/madworld_core.dll")
            }))
        }
        .map_err(|e| e.to_string())?;
        let abi = unsafe { library.get::<unsafe extern "C" fn() -> u32>(b"mw_native_core_abi\0") }
            .map_err(|e| e.to_string())?;
        if unsafe { abi() } != 1 {
            return Err("Native core ABI mismatch".into());
        }
        let option = unsafe {
            library.get::<unsafe extern "C" fn(*const c_char, *const c_char) -> i32>(
                b"mw_native_set_option\0",
            )
        }
        .map_err(|e| e.to_string())?;
        let bind = unsafe {
            library.get::<unsafe extern "C" fn(
                    unsafe extern "C" fn(*const c_char) -> *mut c_void,
                ) -> i32>(b"mw_native_bind_platform\0")
        }
        .map_err(|e| e.to_string())?;
        let run = unsafe { library.get::<unsafe extern "C" fn() -> i32>(b"mw_native_run\0") }
            .map_err(|e| e.to_string())?;
        let stop = *unsafe { library.get::<Stop>(b"mw_native_request_stop\0") }
            .map_err(|e| e.to_string())?;
        let error = unsafe {
            library.get::<unsafe extern "C" fn() -> *const c_char>(b"mw_native_last_error\0")
        }
        .map_err(|e| e.to_string())?;
        let partition = std::env::var("MW_OWNED_PARTITION").unwrap_or_else(|_| {
            "D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA".into()
        });
        let user_directory=if cfg!(test){
            if let Ok(tag)=std::env::var("MW_GPU_TEST_USER_TAG"){
                if tag.is_empty()||tag.len()>64||!tag.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'){return Err("Invalid native test user tag".into());}
                let base=self.root.join("evidence/native-users");std::fs::create_dir_all(&base).map_err(|e|e.to_string())?;
                let base=base.canonicalize().map_err(|e|e.to_string())?;let path=base.join(tag);std::fs::create_dir_all(&path).map_err(|e|e.to_string())?;
                let path=path.canonicalize().map_err(|e|e.to_string())?;if !path.starts_with(&base){return Err("Native test user directory escapes evidence".into());}path
            }else{self.root.join("native-core-user")}
        }else{self.root.join("native-core-user")};
        // inspection: MW_NATIVE_ROOM=0x303 boots Elise's room instead (its PlayerTbl Boss point 1)
        let elise_room = std::env::var("MW_NATIVE_ROOM").as_deref() == Ok("0x303");
        let mut options = vec![
            ("MW_PARTITION_DIR", partition),
            (
                "MW_USER_DIR",
                user_directory
                    .to_string_lossy()
                    .into_owned(),
            ),
            ("MW_BOOT_SCENE", if elise_room { "0x303".into() } else { "0x201".into() }),
        ];
        options.push(("MW_BOOT_POINT", if elise_room { "1".into() } else { "5".into() }));
        // (c303 spawns her on entry; its boss switch would leave the room in demo mode)
        if !elise_room { options.push(("MW_BOOT_BOSS", "1".into())); }
        for (key, value) in options {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).map_err(|e| e.to_string())?;
            if unsafe { option(key.as_ptr(), value.as_ptr()) } != 0 {
                return Err("Native configuration rejected".into());
            }
        }
        let pc_ui = std::env::var_os("MW_PC_UI_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| self.root.join("staged/pc-ui/core_us.idd"));
        if pc_ui.is_file() {
            let value = CString::new(pc_ui.to_string_lossy().as_bytes())
                .map_err(|e| e.to_string())?;
            if unsafe { option(c"MW_PC_UI_FILE".as_ptr(), value.as_ptr()) } != 0 {
                return Err("PC prompt resource configuration rejected".into());
            }
        }
        if std::env::var_os("MW_NATIVE_DIAGNOSTICS").is_some() {
            for name in [c"MW_TRACE_WORLD", c"MW_TRACE_OBJECTS"] {
                if unsafe { option(name.as_ptr(), c"1".as_ptr()) } != 0 {
                    return Err("Native diagnostic configuration rejected".into());
                }
            }
        }
        host::install(render, Box::new(audio)).map_err(str::to_owned)?;
        if unsafe { bind(host::resolve_symbol) } != 0 {
            return Err("Native hardware binding rejected".into());
        }
        {
            let mut state = self.state.lock().unwrap();
            state.stop = Some(stop);
            state.status = String::new();
            if self.closing.load(Ordering::Acquire) {
                unsafe { stop() };
            }
        }
        let code = unsafe { run() };
        let message = if code == 0 {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(error()) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        self.state.lock().unwrap().stop = None; // Never retain a pointer into an unloaded DLL.
        if code != 0 {
            return Err(format!("Core exited{code}:{}", message.unwrap_or_default()));
        }
        Ok(())
    }
    pub fn shutdown(&self) {
        self.closing.store(true, Ordering::Release);
        {
            let state = self.state.lock().unwrap();
            if let Some(stop) = state.stop {
                unsafe { stop() };
            }
        }
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mw_native_platform::native_hardware::host::{DrawVertex, EmptyRenderSink, GxAction};
    use std::{
        sync::atomic::AtomicUsize,
        time::{Duration, Instant},
    };
    struct CountRender {
        inner: EmptyRenderSink,
        actions: Arc<AtomicUsize>,
    }
    impl RenderSink for CountRender {
        fn exec(&mut self, action: GxAction) {
            self.actions.fetch_add(1, Ordering::Relaxed);
            self.inner.exec(action);
        }
        fn vertex_scratch(&mut self) -> &mut Vec<DrawVertex> {
            self.inner.vertex_scratch()
        }
    }
    #[test]
    #[ignore = "Requires the owned native core and extracted partition on D:"]
    fn owned_core_uses_static_host_sinks_and_stops() {
        let session = Session::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        let actions = Arc::new(AtomicUsize::new(0));
        let audio = AudioBridge::default();
        session.start(
            Box::new(CountRender {
                inner: EmptyRenderSink::default(),
                actions: actions.clone(),
            }),
            audio,
        );
        let deadline = Instant::now() + Duration::from_secs(15);
        while actions.load(Ordering::Relaxed) == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
            if session.status() == "Unable to start MadWorld" {
                break;
            }
        }
        session.shutdown();
        assert!(
            actions.load(Ordering::Relaxed) > 0,
            "No original GX actions reached the Rust host: {}",
            session.status()
        );
        assert_ne!(session.status(), "Unable to start MadWorld");
        session.shutdown(); // Closing the owner twice is safe.
    }
}
