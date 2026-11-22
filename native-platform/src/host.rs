//! Bevy/host boundary. Gameplay and shared RAM remain owned by the native core.
use gecko::{host::RenderSink,audio::AudioSink};
use std::{cell::{Cell,RefCell},ffi::{CStr,c_char,c_void},sync::{Mutex,atomic::{AtomicU64,Ordering}}};
type Sinks=(Box<dyn RenderSink>,Box<dyn AudioSink>);
thread_local! {
    static PENDING:RefCell<Option<Sinks>>=const{RefCell::new(None)};
    static HOST_INPUT:Cell<bool>=const{Cell::new(false)};
}
static CONTROLLER:AtomicU64=AtomicU64::new(u64::from_be_bytes([0,0,0,128,128,0,0,0]));
/// Install on the CPU worker before calling the original core. The sinks move
/// into the one System created by mw_dsp_create, never into a second CPU.
pub fn install(render:Box<dyn RenderSink>,audio:Box<dyn AudioSink>)->Result<(), &'static str>{
    PENDING.with(|pending|{
        let mut pending=pending.borrow_mut();
        if pending.is_some(){return Err("Host hardware sinks already installed");}
        *pending=Some((render,audio));HOST_INPUT.set(true);Ok(())
    })
}
pub(crate) fn take()->Option<Sinks>{PENDING.with(|pending|pending.borrow_mut().take())}
pub fn clear(){PENDING.with(|pending|{pending.borrow_mut().take();});HOST_INPUT.set(false);}
/// Explicit game-window input only. All eight bytes are published atomically.
pub fn set_controller(packet:[u8;8]){CONTROLLER.store(u64::from_be_bytes(packet),Ordering::Release);}
pub(crate) fn controller()->Option<[u8;8]>{HOST_INPUT.get().then(||CONTROLLER.load(Ordering::Acquire).to_be_bytes())}
/// PC free camera (MW_PC_FREE_CAMERA=1): orbit radians from mouse / right stick, summed
/// until the core's follow camera (runtime/game/pc_camera.cpp) takes them once per frame.
/// Input older than ORBIT_STALE (no player camera running: events, menus) is dropped,
/// so it cannot land as one jump afterwards.
struct Orbit{yaw:f32,pitch:f32,cycle:i8,since:Option<std::time::Instant>,auto_follow:bool,lock_orbit:bool}
static ORBIT:Mutex<Orbit>=Mutex::new(Orbit{yaw:0.,pitch:0.,cycle:0,since:None,auto_follow:true,lock_orbit:false});
const ORBIT_STALE:std::time::Duration=std::time::Duration::from_millis(250);
const ORBIT_LIMIT:f32=0.5;
pub fn add_camera_orbit(yaw:f32,pitch:f32){
    if !(yaw.is_finite()&&pitch.is_finite())||(yaw==0.&&pitch==0.){return;}
    let mut o=ORBIT.lock().unwrap_or_else(|e|e.into_inner());
    if o.since.is_none(){o.since=Some(std::time::Instant::now());}
    o.yaw=(o.yaw+yaw).clamp(-ORBIT_LIMIT,ORBIT_LIMIT);o.pitch=(o.pitch+pitch).clamp(-ORBIT_LIMIT,ORBIT_LIMIT);
}
/// Lock-on target cycle: +1 next target to the right on screen, -1 to the left. The core acts on it
/// only while the game itself is locked on; one event per take.
pub fn add_camera_cycle(dir:i8){
    if dir==0{return;}
    let mut o=ORBIT.lock().unwrap_or_else(|e|e.into_inner());
    if o.since.is_none(){o.since=Some(std::time::Instant::now());}
    o.cycle=dir.signum();
}
/// Orbit also while the game is locked on (player or game set); off = the original lock camera.
pub fn set_camera_lock_orbit(on:bool){ORBIT.lock().unwrap_or_else(|e|e.into_inner()).lock_orbit=on;}
/// With auto-follow the camera eases back behind a moving Jack after the orbit input stops.
pub fn set_camera_auto_follow(on:bool){ORBIT.lock().unwrap_or_else(|e|e.into_inner()).auto_follow=on;}
/// (yaw, pitch, cycle, auto_follow, lock_orbit) since the previous take.
pub(crate) fn take_camera_orbit()->(f32,f32,i8,bool,bool){
    let mut o=ORBIT.lock().unwrap_or_else(|e|e.into_inner());
    let fresh=o.since.is_some_and(|t|t.elapsed()<=ORBIT_STALE);
    let out=if fresh{(o.yaw,o.pitch,o.cycle)}else{(0.,0.,0)};
    o.yaw=0.;o.pitch=0.;o.cycle=0;o.since=None;(out.0,out.1,out.2,o.auto_follow,o.lock_orbit)
}
/// WPADControlMotor state per channel (bit = channel), set by the core's input HLE;
/// the frontend turns it into gamepad rumble. Presentation only, never read by the game.
static RUMBLE:std::sync::atomic::AtomicU8=std::sync::atomic::AtomicU8::new(0);
pub(crate) fn set_rumble(chan:u32,on:bool){if chan<8{let bit=1u8<<chan;if on{RUMBLE.fetch_or(bit,Ordering::Relaxed);}else{RUMBLE.fetch_and(!bit,Ordering::Relaxed);}}}
pub fn rumble(chan:u32)->bool{chan<8&&RUMBLE.load(Ordering::Relaxed)&(1<<chan)!=0}
/// The game's current remote prompt kind (cockpit child +0x18, 0 = none), as last read
/// by the input HLE. Frontend presentation only (PC prompt glyphs).
static REMOTE_PROMPT:std::sync::atomic::AtomicU8=std::sync::atomic::AtomicU8::new(0);
pub(crate) fn set_remote_prompt(kind:u8){REMOTE_PROMPT.store(kind,Ordering::Relaxed);}
pub fn remote_prompt()->u8{REMOTE_PROMPT.load(Ordering::Relaxed)}
/// Borrowed MEM1 of the live System; cleared before the core unmaps it.
static GUEST_MEM1:Mutex<usize>=Mutex::new(0);
pub(crate) fn attach_guest(mem1:*mut u8){*GUEST_MEM1.lock().unwrap_or_else(|e|e.into_inner())=mem1 as usize;}
pub(crate) fn detach_guest(){*GUEST_MEM1.lock().unwrap_or_else(|e|e.into_inner())=0;}
/// Read-only peek of one big-endian MEM1 word for frontend presentation
/// (e.g. whether an original event is playing). Never writes guest state.
pub fn peek_mem1_u32(address:u32)->Option<u32>{
    let offset=address.checked_sub(0x8000_0000)? as usize;
    if offset+4>0x180_0000{return None;}
    let base=GUEST_MEM1.lock().unwrap_or_else(|e|e.into_inner());
    if *base==0{return None;}
    let mut word=[0u8;4];
    for (i,byte) in word.iter_mut().enumerate(){*byte=unsafe{std::ptr::read_volatile((*base as *const u8).add(offset+i))};}
    Some(u32::from_be_bytes(word))
}
/// Resolve the existing hardware C ABI into this Rust binary. The caller must
/// keep this code loaded until the core has stopped and destroyed its System.
/// # Safety
/// name must point to a live NUL-terminated ABI symbol name.
pub unsafe extern "C" fn resolve_symbol(name:*const c_char)->*mut c_void{
    if name.is_null(){return std::ptr::null_mut();}
    let name=unsafe{CStr::from_ptr(name)}.to_bytes();
    macro_rules! address{($function:path)=>{$function as *const () as *mut c_void};}
    match name {
        b"mw_platform_state_save"=>address!(crate::snapshot::mw_platform_state_save),
        b"mw_platform_state_load"=>address!(crate::snapshot::mw_platform_state_load),
        b"mw_bt_state_save"=>address!(crate::snapshot::mw_bt_state_save),
        b"mw_bt_state_load"=>address!(crate::snapshot::mw_bt_state_load),
        b"mw_state_free"=>address!(crate::snapshot::mw_state_free),
        b"mw_platform_abi"=>address!(crate::mw_platform_abi),
        b"mw_dsp_create"=>address!(crate::mw_dsp_create),
        b"mw_dsp_destroy"=>address!(crate::mw_dsp_destroy),
        b"mw_dsp_read"=>address!(crate::mw_dsp_read),
        b"mw_dsp_write"=>address!(crate::mw_dsp_write),
        b"mw_dsp_tick"=>address!(crate::mw_dsp_tick),
        b"mw_dsp_irq"=>address!(crate::mw_dsp_irq),
        b"mw_dsp_poll_hint"=>address!(crate::mw_dsp_poll_hint),
        b"mw_memory_tracking"=>address!(crate::mw_memory_tracking),
        b"mw_gx_command"=>address!(crate::mw_gx_command),
        b"mw_gx_scene"=>address!(crate::mw_gx_scene),
        b"mw_ai_control"=>address!(crate::mw_ai_control),
        b"mw_audio_flush"=>address!(crate::mw_audio_flush),
        b"mw_vi_write"=>address!(crate::video::mw_vi_write),
        b"mw_bt_create"=>address!(crate::input::mw_bt_create),
        b"mw_bt_destroy"=>address!(crate::input::mw_bt_destroy),
        b"mw_bt_ioctlv"=>address!(crate::input::mw_bt_ioctlv),
        b"mw_bt_tick"=>address!(crate::input::mw_bt_tick),
        b"mw_pad_create"=>address!(crate::pad::mw_pad_create),
        b"mw_pad_destroy"=>address!(crate::pad::mw_pad_destroy),
        b"mw_pad_sample"=>address!(crate::pad::mw_pad_sample),
        b"mw_pad_set_prompt"=>address!(crate::pad::mw_pad_set_prompt),
        b"mw_pad_speaker_control"=>address!(crate::pad::mw_pad_speaker_control),
        b"mw_pad_speaker_on"=>address!(crate::pad::mw_pad_speaker_on),
        b"mw_pad_speaker_stream"=>address!(crate::pad::mw_pad_speaker_stream),
        b"mw_pad_state_save"=>address!(crate::pad::mw_pad_state_save),
        b"mw_pad_state_load"=>address!(crate::pad::mw_pad_state_load),
        b"mw_pad_camera_take"=>address!(crate::pad::mw_pad_camera_take),
        b"mw_pad_motor"=>address!(crate::pad::mw_pad_motor),
        _=>std::ptr::null_mut(),
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn explicit_controller_packets_and_unknown_symbols(){
        assert!(controller().is_none());HOST_INPUT.set(true);
        let packet=[0x10,8,3,230,14,1,0,0];set_controller(packet);
        assert_eq!(controller(),Some(packet));
        assert!(!unsafe{resolve_symbol(c"mw_vi_write".as_ptr())}.is_null());
        assert!(unsafe{resolve_symbol(c"not_a_native_symbol".as_ptr())}.is_null());
        clear();set_controller([0,0,0,128,128,0,0,0]);assert!(controller().is_none());
    }
    #[test]
    fn camera_orbit_sums_until_taken_and_drops_stale_input(){
        assert!(!unsafe{resolve_symbol(c"mw_pad_camera_take".as_ptr())}.is_null());
        take_camera_orbit();
        add_camera_orbit(0.1,-0.02);add_camera_orbit(0.05,0.);add_camera_orbit(f32::NAN,0.);
        let (y,p,cycle,follow,_)=take_camera_orbit();assert!((y-0.15).abs()<1e-6&&(p+0.02).abs()<1e-6&&cycle==0&&follow);
        assert_eq!(take_camera_orbit().0,0.);
        add_camera_orbit(3.,0.);assert_eq!(take_camera_orbit().0,ORBIT_LIMIT);
        add_camera_orbit(0.2,0.);std::thread::sleep(ORBIT_STALE+std::time::Duration::from_millis(30));
        assert_eq!(take_camera_orbit().0,0.);
        set_camera_auto_follow(false);assert!(!take_camera_orbit().3);set_camera_auto_follow(true);
        set_camera_lock_orbit(true);assert!(take_camera_orbit().4);set_camera_lock_orbit(false);assert!(!take_camera_orbit().4);
        add_camera_cycle(-3);assert_eq!(take_camera_orbit().2,-1);assert_eq!(take_camera_orbit().2,0);
    }
    #[test]
    fn host_sinks_receive_native_gx_and_actual_aid_dma(){
        use std::sync::{Arc,Mutex};
        struct Audio(Arc<Mutex<Vec<(i16,i16)>>>);
        impl AudioSink for Audio{fn push_stereo_i16(&mut self,l:i16,r:i16){self.0.lock().unwrap().push((l,r));}}
        let samples=Arc::new(Mutex::new(Vec::new()));
        let (sink,draws)=crate::audit::Sink::new(None);
        install(Box::new(sink),Box::new(Audio(samples.clone()))).unwrap();
        let mut one=vec![0u8;0x1800000];let mut two=vec![0u8;0x4000000];
        let sys=unsafe{crate::mw_dsp_create(one.as_mut_ptr(),two.as_mut_ptr())};assert!(!sys.is_null());
        assert!(take().is_none());
        let send=|packet:&[u8]|assert_eq!(unsafe{crate::mw_gx_command(sys,packet.as_ptr(),packet.len())},0);
        send(&[8,0x50,0,0,2,0]);send(&[8,0x70,0,0,0,9]);
        let mut triangle=vec![0x90,0,3];
        for xyz in [[0f32,0.,0.],[1.,0.,0.],[0.,1.,0.]]{for value in xyz{triangle.extend(value.to_be_bytes());}}
        send(&triangle);assert_eq!(draws.load(Ordering::Relaxed),1);
        for i in 0..8{one[0x1000+i*4..0x1004+i*4].copy_from_slice(&[0,2,0,1]);}
        unsafe{
            assert_eq!(crate::mw_ai_control(sys,64),0);
            assert_eq!(crate::mw_dsp_write(sys,0xcc005030,4,0x1000),0);
            assert_eq!(crate::mw_dsp_write(sys,0xcc005036,2,0x8001),0);
            assert_eq!(crate::mw_dsp_tick(sys,60750),0);
            crate::mw_dsp_destroy(sys);
        }
        assert_eq!(samples.lock().unwrap().as_slice(),vec![(1,2);32]);
        one[0]=7;assert_eq!(one[0],7);clear();
    }
}
