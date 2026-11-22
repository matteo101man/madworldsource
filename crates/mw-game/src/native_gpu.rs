//! Owned-core GPU validation. No window, surface, screenshots or desktop input.
use super::{native_audio::AudioBridge, native_core::Session};
use crate::native_controls::PcControls;
use bevy::prelude::{ButtonInput, KeyCode, MouseButton};
use mw_native_platform::{
    host,
    native_renderer::sink::{Renderer, TargetAspect},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[test]
#[ignore = "Requires a DX12 adapter, the owned core DLL and extracted partition"]
fn owned_core_headless_gpu_and_host_blit() {
    mw_native_platform::native_hardware::paths::set_base(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../native-host-data"),
    );
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter =
        bevy::tasks::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("No DX12 adapter");
    let info = adapter.get_info();
    let (device, queue) = bevy::tasks::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("native headless integration"),
        ..Default::default()
    }))
    .expect("GPU device creation");
    let (renderer, sink) = Renderer::new(
        device.clone(),
        queue.clone(),
        wgpu::TextureFormat::Rgba8Unorm,
        TargetAspect::Ratio(16.0 / 9.0),
        1,
    );
    let frames = Arc::new(AtomicUsize::new(0));
    let counter = frames.clone();
    renderer.set_frame_ready_callback(move |_| {
        counter.fetch_add(1, Ordering::Release);
    });
    let session = Session::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let mut prompt_probe = crate::native_prompts::CaptureProbe::from_env(&renderer);
    session.start(Box::new(sink), AudioBridge::default());
    let started = Instant::now();
    let extended = std::env::var("MW_GPU_TEST_SECONDS").ok().map(|x| {
        x.parse::<u64>()
            .expect("MW_GPU_TEST_SECONDS must be integer seconds")
    });
    if let Some(seconds) = extended {
        let maximum=if std::env::var_os("MW_GPU_TEST_PC_SUSTAIN").is_some(){900}else{300};
        assert!((1..=maximum).contains(&seconds));
    }
    let trace = std::env::var_os("MW_GPU_TEST_LOG").map(PathBuf::from);
    let mut input_stopped = false;
    let pc_mode = std::env::var_os("MW_GPU_TEST_PC_SEQUENCE").is_some();
    let mut pc_started = None;
    let mut rin_seen = None;
    let mut script = PcSequence::default();
    let mut death_probe = DeathCapture::from_env(trace.clone());
    // Default-off source UI reference fixture: pause after control opens and retain one native XFB.
    let ui_capture = std::env::var_os("MW_GPU_TEST_UI_PAUSE_CAPTURE").map(PathBuf::from);
    let mut ui_captured = false;
    let mut title_confirm_captured=false;
    let mut ui_last_packet=[0,0,0,128,128,0,0,0];
    let ui_page=std::env::var("MW_GPU_TEST_UI_PAGE").unwrap_or_else(|_|"map".into());
    let save_action=std::env::var("MW_GPU_TEST_SAVE_ACTION").unwrap_or_default();
    assert!(save_action.is_empty()||(ui_page=="title-newgame"&&matches!(save_action.as_str(),"copy"|"erase")),"Save action fixture requires title-newgame");
    assert!(matches!(ui_page.as_str(),"map"|"playlog"|"options"|"controls"|"sound"|"video"|"title"|"title-return"|"title-menu"|"title-newgame"|"title-newgame-confirm"),"UI fixture page");
    while started.elapsed() < Duration::from_secs(extended.unwrap_or(45))
        && (extended.is_some() || frames.load(Ordering::Acquire) < 3)
    {
        // Explicit virtual packets only; no OS keyboard or mouse actions.
        let mut packet = [0, 0, 0, 128, 128, 0, 0, 0];
        if !input_stopped || (pc_mode && pc_started.is_none()) {
            if let Some(path) = &trace {
                if let Ok(text) = std::fs::read_to_string(path) {
                    input_stopped |= text.contains("[event] request=21 caller=");
                    if pc_mode && pc_started.is_none() {
                        if rin_seen.is_none() && text.contains("[rin-native] onMove count=1 ") {
                            rin_seen = Some(Instant::now());
                        }
                        let ready = text
                            .lines()
                            .rev()
                            .find(|line| line.starts_with("[boss-set-update] flags="))
                            .is_some_and(|line| {
                                line.contains("demo-counter=0 ") && line.contains("word0=00000000")
                            });
                        if rin_seen.is_some() && ready {
                            pc_started = Some(Instant::now());
                        }
                    }
                }
            }
        }
        let elapsed = started.elapsed().as_secs_f64();
        if !input_stopped && elapsed >= 3.0 && (elapsed - 3.0) % 1.5 < 0.35 {
            packet[1] = 8;
        }
        if let Some(start) = pc_started {
            packet = script.packet(start.elapsed().as_secs_f64());
        } else if let Some(start) = rin_seen {
            packet = script.confirm(start.elapsed().as_secs_f64());
        }
        if ui_capture.is_some() {
            if let Some(start) = pc_started {
                let seconds = start.elapsed().as_secs_f64();
                packet = [0, 0, 0, 128, 128, 0, 0, 0];
                if (3.0..3.35).contains(&seconds) { packet[1] = 0x10; } // original Minus / PC Pause
                if ui_page!="map" {
                    if ui_page!="playlog"&&(4.5..4.7).contains(&seconds){packet[0]=4;} // remote D-pad Down
                    if (5.5..5.7).contains(&seconds){packet[1]=8;} // A / confirm
                    if matches!(ui_page.as_str(),"controls"|"sound"|"video"|"title"|"title-return"|"title-menu"|"title-newgame"|"title-newgame-confirm") {
                        if matches!(ui_page.as_str(),"sound"|"video"|"title"|"title-return"|"title-menu"|"title-newgame"|"title-newgame-confirm")&&(7.0..7.2).contains(&seconds){packet[0]=4;}
                        if matches!(ui_page.as_str(),"video"|"title"|"title-return"|"title-menu"|"title-newgame"|"title-newgame-confirm")&&(7.7..7.9).contains(&seconds){packet[0]=4;}
                        if matches!(ui_page.as_str(),"title"|"title-return"|"title-menu"|"title-newgame"|"title-newgame-confirm"){
                            if (8.4..8.6).contains(&seconds){packet[0]=4;}
                            if (9.2..9.4).contains(&seconds){packet[1]=8;}
                        }else if (8.5..8.7).contains(&seconds){packet[1]=8;}
                    }
                }
                if matches!(ui_page.as_str(),"title-return"|"title-menu"|"title-newgame"|"title-newgame-confirm"){
                    if seconds>=11.5&&!title_confirm_captured{
                        if let Some(path)=std::env::var_os("MW_GPU_TEST_UI_TITLE_CONFIRM_CAPTURE"){
                            if let Some(frame)=renderer.capture_xfb(){let mut ppm=format!("P6 {} {} 255\n",frame.width,frame.height).into_bytes();ppm.extend(frame.rgba.chunks_exact(4).flat_map(|p|[p[0],p[1],p[2]]));std::fs::write(path,ppm).unwrap();title_confirm_captured=true;}
                        }
                    }
                    if (13.0..13.2).contains(&seconds){packet[0]=1;}
                    if (14.0..14.2).contains(&seconds){packet[1]=8;}
                    if matches!(ui_page.as_str(),"title-menu"|"title-newgame"|"title-newgame-confirm")&&(28.0..28.2).contains(&seconds){packet[1]=8;}
                    if matches!(ui_page.as_str(),"title-newgame"|"title-newgame-confirm")&&(41.0..41.2).contains(&seconds){packet[1]=8;}
                    if ui_page=="title-newgame-confirm"&&(48.0..49.0).contains(&seconds){packet[1]=8;}
                    if !save_action.is_empty(){
                        if (48.0..48.2).contains(&seconds)||(save_action=="erase"&&(49.0..49.2).contains(&seconds)){packet[0]=4;}
                        if (51.0..51.2).contains(&seconds){packet[1]=8;}
                        if save_action=="copy"{if (54.0..54.2).contains(&seconds){packet[0]=4;}if (56.0..56.2).contains(&seconds){packet[1]=8;}}
                    }
                }
                let capture_after=if save_action=="copy"{61.}else if save_action=="erase"||ui_page=="title-newgame-confirm"{55.}else if ui_page=="title-newgame"{45.}else if matches!(ui_page.as_str(),"title-return"|"title-menu"|"title-newgame"|"title-newgame-confirm"){40.}else if ui_page=="title"{11.5}else if ui_page=="map"{6.}else if matches!(ui_page.as_str(),"options"|"playlog"){8.}else{11.};
                if seconds >= capture_after && !ui_captured {
                    if let Some(frame) = renderer.capture_xfb() {
                        let mut ppm = format!("P6 {} {} 255\n", frame.width, frame.height).into_bytes();
                        ppm.extend(frame.rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]));
                        std::fs::write(ui_capture.as_ref().unwrap(), ppm).expect("owned UI reference capture");
                        ui_captured = true;
                        println!("UI_PAUSE_CAPTURE path={} controlSeconds={seconds:.2}", ui_capture.as_ref().unwrap().display());
                    }
                }
            }
        }
        if ui_capture.is_some()&&packet!=ui_last_packet{
            println!("UI_FIXTURE_PACKET controlSeconds={:.3} packet={packet:?}",pc_started.map(|s|s.elapsed().as_secs_f64()).unwrap_or(-1.));
            ui_last_packet=packet;
        }
        host::set_controller(packet);
        if let Some(probe) = &mut prompt_probe {
            probe.tick(&renderer, started.elapsed().as_secs_f64(), script.phase);
        }
        if let Some(probe) = &mut death_probe {
            probe.tick(&renderer, started.elapsed().as_secs_f64());
        }
        if session.status() == "Unable to start MadWorld" {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    host::set_controller([0, 0, 0, 128, 128, 0, 0, 0]);
    session.shutdown();
    let presented = frames.load(Ordering::Acquire);
    if pc_mode {
        assert!(
            pc_started.is_some(),
            "Native player-control phase did not open"
        );
        if ui_capture.is_some() {
            assert!(ui_captured, "Native UI pause fixture did not reach capture");
        } else {
            assert_eq!(script.phase, "released", "PC action sequence did not complete");
        }
    }
    assert_ne!(session.status(), "Unable to start MadWorld");
    assert!(
        presented >= 3,
        "Only {presented} native XFB presentations reached the GPU"
    );
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("host blit validation"),
        size: wgpu::Extent3d {
            width: 1280,
            height: 720,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    renderer.blit_into_encoder(
        &mut encoder,
        &target.create_view(&Default::default()),
        (1280, 720),
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
    );
    queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("GPU completion");
    println!(
        "HEADLESS_GPU {}",
        serde_json::json!({"adapter":info.name,"backend":format!("{:?}",info.backend),"presentations":presented,"hostBlitCompleted":true,"seconds":started.elapsed().as_secs_f64(),"extendedSeconds":extended,"pcSequenceRequested":pc_mode,"pcSequenceStarted":pc_started.is_some(),"startupInputStoppedForEvent21":input_stopped,"uiPauseCaptureRequested":ui_capture.is_some(),"uiPauseCaptured":ui_captured,"scope":"Original native startup presentations and GPU/host blit validation only; no window, image, pixel equivalence or battle fidelity proof"})
    );
}

#[derive(Default)]
struct PcSequence {
    controls: PcControls,
    keys: ButtonInput<KeyCode>,
    mouse: ButtonInput<MouseButton>,
    phase: &'static str,
    last_packet_report:f64,
    steering: Steering,
}
impl PcSequence {
    fn packet(&mut self, seconds: f64) -> [u8; 8] {
        if std::env::var_os("MW_GPU_TEST_PC_MOTION_PROBE").is_some(){
            let (phase,keys)=if seconds<2. {("motion-probe-baseline",vec![KeyCode::KeyQ])}else if seconds<14. {("held-struggle-probe",vec![KeyCode::KeyQ,KeyCode::KeyX])}else if seconds<18. {("motion-probe-release",vec![KeyCode::KeyQ])}else{("released",vec![])};
            return self.emit(phase,seconds,keys,vec![]);
        }
        if std::env::var_os("MW_GPU_TEST_PC_SEEK_COMPETITION").is_some(){
            let (phase,keys)=if seconds<110. || std::env::var_os("MW_GPU_TEST_PC_SUSTAIN").is_some() {("seeking-competition",vec![KeyCode::KeyQ])}else{("released",vec![])};
            return self.emit(phase,seconds,keys,vec![]);
        }
        let mut keys = Vec::new();
        let mut mouse = Vec::new();
        let phase = match seconds {
            t if t < 2. => "baseline",
            t if t < 8. => {
                keys.extend([KeyCode::KeyW, KeyCode::KeyQ]);
                "approach"
            }
            t if t < 18. => {
                keys.push(KeyCode::KeyQ);
                keys.push(KeyCode::KeyW);
                if (t - 8.) % 0.75 < 0.25 {
                    mouse.push(MouseButton::Left);
                }
                "punch"
            }
            t if t < 30. => {
                keys.extend([KeyCode::KeyQ, KeyCode::KeyW]);
                let local = (t - 18.) % 1.2;
                if local < 0.75 {
                    mouse.push(MouseButton::Right);
                }
                if (0.3..0.5).contains(&local) {
                    keys.push(KeyCode::KeyF);
                }
                "chainsaw"
            }
            t if t < 36. => {
                keys.extend([KeyCode::KeyQ, KeyCode::KeyW]);
                let local = (t - 30.) % 1.5;
                if local < 0.25 {
                    keys.push(KeyCode::Space);
                }
                if (0.3..0.55).contains(&local) {
                    mouse.push(MouseButton::Left);
                }
                "jump-attack"
            }
            t if t < 42. => {
                if (t - 36.) % 1.5 < 0.25 {
                    keys.push(KeyCode::ShiftLeft);
                }
                "dodge"
            }
            t if t < 54. => {
                keys.extend([KeyCode::KeyQ, KeyCode::KeyW]);
                let local = (t - 42.) % 2.;
                if local < 0.25 {
                    keys.push(KeyCode::KeyF);
                }
                if (1.0..1.25).contains(&local) {
                    keys.push(KeyCode::KeyR);
                }
                "directional-strikes"
            }
            t if t < 110. => {
                keys.extend([KeyCode::KeyQ, KeyCode::KeyW]);
                let local = (t - 54.) % 8.;
                if local < 4. {
                    if local % 0.75 < 0.3 {
                        mouse.push(MouseButton::Left);
                    }
                } else {
                    let phase = (local - 4.) % 1.5;
                    if phase < 0.9 {
                        mouse.push(MouseButton::Right);
                    }
                    if (0.3..0.6).contains(&phase) {
                        keys.push(KeyCode::KeyF);
                    }
                }
                "sustained-combat"
            }
            _ => "released",
        };
        self.emit(phase, seconds, keys, mouse)
    }
    fn confirm(&mut self, seconds: f64) -> [u8; 8] {
        let pulse = seconds % 1.5 < 0.3;
        let (phase, keys, mouse) = match (seconds / 4.) as u32 % 6 {
            0 => (
                "native-confirm",
                if pulse { vec![KeyCode::Enter] } else { vec![] },
                vec![],
            ),
            1 => ("native-camera", vec![KeyCode::KeyQ], vec![]),
            2 => (
                "native-attack",
                vec![],
                if pulse {
                    vec![MouseButton::Left]
                } else {
                    vec![]
                },
            ),
            3 => (
                "native-chainsaw",
                vec![],
                if pulse {
                    vec![MouseButton::Right]
                } else {
                    vec![]
                },
            ),
            4 => (
                "native-jump",
                if pulse { vec![KeyCode::Space] } else { vec![] },
                vec![],
            ),
            _ => (
                "native-dodge",
                if pulse {
                    vec![KeyCode::ShiftLeft]
                } else {
                    vec![]
                },
                vec![],
            ),
        };
        self.emit(phase, seconds, keys, mouse)
    }
    fn emit(
        &mut self,
        mut phase: &'static str,
        seconds: f64,
        mut keys: Vec<KeyCode>,
        mut mouse: Vec<MouseButton>,
    ) -> [u8; 8] {
        let struggle = std::env::var_os("MW_GPU_TEST_PC_STRUGGLE").is_some();
        if std::env::var_os("MW_GPU_TEST_PC_MOTION_PROBE").is_none() && !phase.starts_with("native-") && (phase != "released" || struggle) {
            self.steering.apply(seconds, &mut keys, phase != "released");
            if std::env::var_os("MW_GPU_TEST_PC_SEEK_COMPETITION").is_some() && phase!="released" && self.steering.near_boss && !self.steering.competition {
                let pulse=std::env::var_os("MW_GPU_TEST_PC_PULSE_CHAINSAW").is_some();
                let local=seconds%1.5;
                if !pulse || local<0.9 {mouse.push(MouseButton::Right);}
                if (self.steering.rin_eligible || self.steering.rin_state==28) && (if pulse {(0.3..0.6).contains(&local)} else {seconds%1.2<0.3}) {keys.push(KeyCode::KeyF);}
            }
            if phase == "released" {
                keys.clear(); mouse.clear();
                if self.steering.competition { phase = "competition-struggle"; }
            }
            if std::env::var_os("MW_GPU_TEST_PC_ATTACK_ONLY_NEAR").is_some() && !self.steering.near_boss {
                mouse.retain(|button| ![MouseButton::Left, MouseButton::Right].contains(button));
                keys.retain(|key| ![KeyCode::KeyF, KeyCode::KeyR].contains(key));
            }
        }
        if !phase.starts_with("native-") && phase != "released" && struggle && self.steering.competition {
            keys.retain(|key| *key==KeyCode::KeyQ);
            mouse.clear();
            keys.push(KeyCode::KeyX);
            phase="competition-struggle";
        }
        // Test-only: once the read-only feedback reports the player at 0 HP, stop
        // battle inputs and pulse the PC confirm binding so the original continue
        // window receives fresh press edges. No game state is written.
        // Test-only: while Rin is in her finish_wait/finish states (27/26), pulse
        // the PC confirm binding so the original boss-finish prompt gets presses.
        if std::env::var_os("MW_GPU_TEST_PC_FINISH_CONFIRM").is_some() && !phase.starts_with("native-") && (self.steering.rin_state==27 || self.steering.rin_state==26) {
            let [_, state, finish_phase, selector] = self.steering.player_state;
            mouse.clear();
            if state==68 {
                keys.clear();
                // Jack's finisher (state 68, selector 3) reads only the remote shake
                // (+0x1088) in phases 12..14 (research/VICTORY-MOTION-INPUT.md): hold the
                // shared PC shake binding X there instead of pulsing Enter.
                // MW_GPU_TEST_PC_FINISH_WIGGLE: the PC shake QTE instead, A/D alternating
                // every 0.1 s, which the input layer turns into a shake under the prompt.
                if selector==3 && (12..15).contains(&finish_phase) {
                    if std::env::var_os("MW_GPU_TEST_PC_FINISH_WIGGLE").is_some() {
                        keys.push(if (seconds/0.1) as u64%2==0 {KeyCode::KeyA} else {KeyCode::KeyD});
                        phase="finish-wiggle";
                    } else {
                        keys.push(KeyCode::KeyX);
                        phase="finish-shake";
                    }
                } else {
                    if seconds%0.8<0.2 { keys.push(KeyCode::Enter); }
                    phase="finish-confirm";
                }
            } else {
                // Rin waits in finish_wait: keep the steering walk toward her and, once
                // close, pulse the chainsaw (right mouse) that starts Jack's finisher, as
                // in the earlier finish-normal run (player 39 -> 68/11/3 at ~1 unit).
                keys.retain(|key| [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD, KeyCode::KeyQ].contains(key));
                if self.steering.near_boss {
                    keys.retain(|key| *key==KeyCode::KeyQ);
                    if seconds%1.2<0.6 { mouse.push(MouseButton::Right); }
                    phase="finish-start";
                } else {
                    phase="finish-approach";
                }
            }
        }
        if std::env::var_os("MW_GPU_TEST_PC_CONTINUE_CONFIRM").is_some() && !phase.starts_with("native-") && self.steering.last_hp==0 {
            keys.clear(); mouse.clear();
            if seconds%1.0<0.2 { keys.push(KeyCode::Enter); }
            phase="dead-confirm";
        }
        self.keys.clear();
        self.mouse.clear();
        let old: Vec<_> = self.keys.get_pressed().copied().collect();
        for key in old {
            if !keys.contains(&key) {
                self.keys.release(key);
            }
        }
        let old: Vec<_> = self.mouse.get_pressed().copied().collect();
        for key in old {
            if !mouse.contains(&key) {
                self.mouse.release(key);
            }
        }
        for key in keys {
            self.keys.press(key);
        }
        for key in mouse {
            self.mouse.press(key);
        }
        let packet = self.controls.packet(&self.keys, &self.mouse, true);
        if phase != self.phase || (std::env::var_os("MW_GPU_TEST_PC_SEEK_COMPETITION").is_some() && seconds-self.last_packet_report>=0.5) {
            self.phase = phase;
            self.last_packet_report=seconds;
            println!(
                "PC_SEQUENCE {}",
                serde_json::json!({"phase":phase,"seconds":seconds,"timeOrigin":if phase.starts_with("native-"){"rin-initialization"}else{"control-ready"},"packet":packet})
            );
        }
        packet
    }
}

// Test-only feedback from bounded read-only snapshots of our own native process.
// The only game-facing output remains PcControls virtual controller packets.
#[derive(Default)]
struct Steering {
    last: Option<([f64; 2], f64, usize)>,
    basis: [[f64; 2]; 2],
    samples: [usize; 2],
    command: usize,
    last_report: f64,
    near_boss: bool,
    competition: bool,
    rin_eligible:bool,
    rin_state:u64,
    player_state:[u64;4],
    last_walk:bool,
    last_hp:i64,
}
impl Steering {
    fn apply(&mut self, seconds: f64, keys: &mut Vec<KeyCode>, movement_enabled: bool) {
        let Some(path) = std::env::var_os("MW_GPU_TEST_PC_FEEDBACK") else { return };
        keys.retain(|key| ![KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD].contains(key));
        if movement_enabled && !keys.contains(&KeyCode::KeyQ) { keys.push(KeyCode::KeyQ); }
        let value = std::fs::read_to_string(path).ok().and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        let Some(value) = value else { return };
        let Some(stamp) = value["sampleSeconds"].as_f64() else { return };
        let coordinate = |role: &str| -> Option<[f64; 2]> {
            Some([value["actors"][role]["position"][0].as_f64()?, value["actors"][role]["position"][2].as_f64()?])
        };
        let (Some(player), Some(rin)) = (coordinate("player"), coordinate("rin")) else { return };
        let fresh = self.last.is_none_or(|(_, time, _)| stamp > time);
        let hp=value["actors"]["player"]["hp"].as_i64().unwrap_or(-1);
        let walk=value["actors"]["player"]["stateBytes11a11d"][1].as_u64()==Some(4)
            && value["playerNativeMotion"]["resourceMatches"].as_array().is_some_and(|rows|rows.iter().any(|r|r["name"].as_str()==Some("pl0030_00ce.mot")));
        if fresh {
            if let Some((old, time, command)) = self.last {
                // Test-only local basis follows the native camera. Learn from
                // stable source walking with unchanged HP, excluding attacks.
                if walk && self.last_walk && hp==self.last_hp && stamp-time>0. && stamp-time<2.5 && command<4 {
                    let delta=[player[0]-old[0],player[1]-old[1]];
                    let length=delta[0].hypot(delta[1]);let duration=stamp-time;
                    if length>0.05 && length/duration<12. {
                        let axis=command%2;let sign=if command<2 {1.}else{-1.};
                        let old_weight=if self.samples[axis]==0 {0.}else{0.5};
                        for component in 0..2 {self.basis[axis][component]=self.basis[axis][component]*old_weight+(1.-old_weight)*sign*delta[component]/duration;}
                        self.samples[axis]+=1;
                    }
                }
            }
        }
        let difference = [rin[0] - player[0], rin[1] - player[1]];
        let distance = difference[0].hypot(difference[1]);
        let vertical = value["actors"]["rin"]["position"][1].as_f64().zip(value["actors"]["player"]["position"][1].as_f64()).map(|(rin, player)| (rin-player).abs()).unwrap_or(f64::INFINITY);
        self.near_boss = distance < 1.6 && vertical < 1.5;
        let flags = value["playerCompetition"]["flags1124"].as_str().and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"),16).ok()).unwrap_or(0);
        self.rin_eligible=value["rinCompetitionEligibility"]["enabledBit0"].as_bool().unwrap_or(false);
        self.rin_state=value["actors"]["rin"]["stateBytes11a11d"][1].as_u64().unwrap_or(u64::MAX);
        for i in 0..4 {self.player_state[i]=value["actors"]["player"]["stateBytes11a11d"][i].as_u64().unwrap_or(u64::MAX);}
        self.competition = value["playerCompetition"]["peerMatchesRin"].as_bool().unwrap_or(false) && (flags & 0x1000000 != 0 || value["playerCompetition"]["stateByte11b"].as_u64()==Some(0x4e));
        self.command = if seconds < 3.5 { 0 } else if seconds < 7. { 1 } else { 4 };
        let close_distance = std::env::var("MW_GPU_TEST_PC_CLOSE_DISTANCE").ok().map(|value| value.parse::<f64>().expect("PC test close distance must be numeric")).unwrap_or(1.8);
        assert!((0.2..=3.).contains(&close_distance));
        if seconds >= 8. && distance > close_distance {
            let mut best = 0.;
            for command in 0..4 {
                let basis = self.basis[command % 2];
                let sign = if command < 2 { 1. } else { -1. };
                let length = basis[0].hypot(basis[1]);
                if length < 0.05 { continue }
                let score = sign * (basis[0] * difference[0] + basis[1] * difference[1]) / length;
                if score > best { best = score; self.command = command; }
            }
        }
        if fresh { self.last = Some((player, stamp, self.command));self.last_walk=walk;self.last_hp=hp; }
        if !movement_enabled { self.command=4; }
        if let Some(key) = [KeyCode::KeyW, KeyCode::KeyD, KeyCode::KeyS, KeyCode::KeyA].get(self.command) { keys.push(*key); }
        if seconds - self.last_report >= 2. {
            self.last_report = seconds;
            println!("PC_STEERING {}", serde_json::json!({"seconds":seconds,"sampleSeconds":stamp,"player":player,"rin":rin,"distance":distance,"vertical":vertical,"nearBoss":self.near_boss,"competition":self.competition,"rinEligible":self.rin_eligible,"rinState":self.rin_state,"basis":self.basis,"samples":self.samples,"command":self.command,"hp":value["actors"]}));
        }
    }
}

/// Headless evidence for the PC death cinematic: `MW_GPU_TEST_DEATH_CAPTURE=<dir>` saves the XFB
/// every `MW_GPU_TEST_DEATH_EVERY` seconds (default 0.5) for `MW_GPU_TEST_DEATH_SPAN` seconds
/// (default 40) from the first death line in the test log (`[pc-death] begin`, or the player's
/// original `onDead` send under MW_TRACE_RPN=1 for a run without the cinematic).
struct DeathCapture {
    dir: PathBuf,
    log: PathBuf,
    read: u64,
    pending: String,
    every: f64,
    span: f64,
    start: Option<f64>,
    next: f64,
    count: usize,
    next_poll: f64,
}
impl DeathCapture {
    fn from_env(log: Option<PathBuf>) -> Option<DeathCapture> {
        let dir = PathBuf::from(std::env::var_os("MW_GPU_TEST_DEATH_CAPTURE")?);
        std::fs::create_dir_all(&dir).ok()?;
        let num = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        Some(DeathCapture { dir, log: log?, read: 0, pending: String::new(), every: num("MW_GPU_TEST_DEATH_EVERY", 0.5), span: num("MW_GPU_TEST_DEATH_SPAN", 40.), start: None, next: 0., count: 0, next_poll: 0. })
    }
    fn tick(&mut self, renderer: &Renderer, t: f64) {
        use std::io::{Read, Seek, SeekFrom};
        if self.start.is_none() && t >= self.next_poll {
            self.next_poll = t + 0.25;
            if let Ok(mut f) = std::fs::File::open(&self.log) {
                if f.seek(SeekFrom::Start(self.read)).is_ok() {
                    let mut buf = Vec::new();
                    if let Ok(n) = f.read_to_end(&mut buf) {
                        self.read += n as u64;
                        self.pending.push_str(&String::from_utf8_lossy(&buf));
                        if self.pending.contains("[pc-death] begin") || self.pending.contains("[rpn-send] name=onDead ") {
                            self.start = Some(t);
                            self.next = t;
                            println!("[death-capture] start t={t:.1}");
                        }
                        if let Some(i) = self.pending.rfind('\n') { self.pending.drain(..=i); }
                    }
                }
            }
        }
        let Some(start) = self.start else { return };
        if t < self.next || t > start + self.span { return; }
        self.next = t + self.every;
        if let Some(f) = renderer.capture_xfb() {
            let name = format!("death-{:03}-t{:05.1}.ppm", self.count, t - start);
            let mut ppm = format!("P6 {} {} 255\n", f.width, f.height).into_bytes();
            ppm.extend(f.rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]));
            if std::fs::write(self.dir.join(&name), ppm).is_ok() {
                self.count += 1;
            }
        }
    }
}
