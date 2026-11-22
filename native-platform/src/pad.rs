//! Direct controller and remote-speaker state for the WPAD/KPAD HLE in the
//! native core (runtime/game/input_hle.cpp). No Bluetooth, HID or IOS traffic:
//! the host packet becomes KPAD-unit values here and the C++ side lays out the
//! original KPADStatus. See research/INPUT-HLE.md.
use crate::motion::Pulse;
use gecko::{System,WII,savestate::{StateWriter,StateReader,StateError}};
use std::panic::{catch_unwind,AssertUnwindSafe};
/// Timebase ticks per former 200 Hz HID report; gestures keep that clock.
const TICK:u64=303750;
const MAX_STEPS:u64=64;
/// Former virtual remote calibration (gecko wiimote EEPROM): zero 0x7F, 1 g = 35 LSB.
const REMOTE_ZERO:f32=127.;const REMOTE_LSB:f32=35.;
/// Former Nunchuk calibration: zero 0x80, 1 g = 0x33 LSB at 8 bits, 10-bit samples.
const NUNCHUK_LSB:f32=51.*4.;
/// KPADiRead clamps every acceleration axis to this (sdata 0x80717BDC).
const KPAD_ACC_LIMIT:f32=3.4;
/// Values in KPAD units; acc axes already remapped from remote (X,Y,Z) to KPAD (-X,-Z,Y).
#[repr(C)]#[derive(Clone,Copy,Default,Debug,PartialEq)]
pub struct MwPadSample {pub hold:u32,pub acc:[f32;3],pub fs_acc:[f32;3],pub stick:[f32;2]}
struct Speaker {on:bool,muted:bool,predictor:i32,step:i32}
impl Default for Speaker {fn default()->Self{Self{on:false,muted:false,predictor:0,step:127}}}
impl Speaker {
    fn reset_decoder(&mut self){self.predictor=0;self.step=127;}
    /// Yamaha ADPCM, adapted from Dolphin (GPL-2.0-or-later) commit 257a6d0e,
    /// Speaker.cpp, as previously used by gecko usb/speaker.rs.
    fn nibble(&mut self,n:u8)->i16 {
        const DIFF:[i32;16]=[1,3,5,7,9,11,13,15,-1,-3,-5,-7,-9,-11,-13,-15];
        const SCALE:[i32;16]=[230,230,230,230,307,409,512,614,230,230,230,230,307,409,512,614];
        self.predictor=(self.predictor+self.step*DIFF[n as usize]/8).clamp(-32768,32767);
        self.step=((self.step*SCALE[n as usize])>>8).clamp(127,24576);self.predictor as i16
    }
    /// Data always advances the decoder; only an enabled, unmuted speaker plays it.
    fn stream(&mut self,data:&[u8])->Option<Vec<i16>> {
        let mut samples=Vec::with_capacity(data.len()*2);
        for &v in data {samples.push(self.nibble(v>>4));samples.push(self.nibble(v&15));}
        (self.on&&!self.muted).then_some(samples)
    }
}
/// Remote prompt kinds (cockpit child +0x18) that ask for a shake: setSwingNavigation
/// enum 5 selects 0x11 (research/PLAYER-COMPETITION-API.md); Jack's finisher phase 13
/// shows it (research/VICTORY-MOTION-INPUT.md).
const SHAKE_PROMPTS:[u8;1]=[0x11];
/// Stick wiggle that counts as a shake: this many left/right reversals in the window.
const WIGGLE_REVERSALS:usize=2;
const WIGGLE_WINDOW:u64=TICK*80;// 0.4 s
/// PC shake QTE: while the game shows its own shake prompt, rapid left/right stick
/// reversals (A/D, or a pad stick) drive the remote and Nunchuk shake gesture, so the
/// original accelerometer integrator measures the effort. No game state writes.
#[derive(Default)]
struct Wiggle {prompt:u8,side:i8,reversals:std::collections::VecDeque<u64>}
impl Wiggle {
    fn active(&mut self,stick_x:u8,tb:u64)->bool {
        let side=if stick_x<64 {-1}else if stick_x>192 {1}else{self.side};
        if side!=self.side {if self.side!=0 {self.reversals.push_back(tb);}self.side=side;}
        while self.reversals.front().is_some_and(|&t|tb.saturating_sub(t)>WIGGLE_WINDOW) {self.reversals.pop_front();}
        SHAKE_PROMPTS.contains(&self.prompt)&&self.reversals.len()>=WIGGLE_REVERSALS
    }
}
pub struct Pad {remote:Pulse,nunchuk:Pulse,buttons:u16,last_tb:u64,started:bool,acc:[f32;3],fs_acc:[f32;3],speaker:[Speaker;4],volume:u8,trace:bool,streamed:u64,played:u64,wiggle:Wiggle,trace_pad:bool,traced:(u32,[i8;2],u8,u8)}
const REST:[f32;3]=[0.,0.,1.];
const SHAKE_TOKEN:u8=0x10|crate::motion::Gesture::Shake as u8;
fn quantize(raw:[f32;3],lsb:f32,lo:f32,hi:f32)->[f32;3]{raw.map(|g|((g*lsb).round().clamp(lo,hi))/lsb)}
fn kpad_axes(v:[f32;3])->[f32;3]{[-v[0],-v[2],v[1]].map(|g|(g+0.).clamp(-KPAD_ACC_LIMIT,KPAD_ACC_LIMIT))}
impl Pad {
    pub fn new()->Self {
        let volume=std::env::var("MW_REMOTE_SPEAKER_VOLUME").ok().and_then(|v|v.parse::<u8>().ok()).unwrap_or(88).min(127);
        let trace=std::env::var("MW_TRACE_REMOTE_SPEAKER").is_ok_and(|v|v=="1");
        let trace_pad=std::env::var("MW_TRACE_PAD").is_ok_and(|v|v=="1");
        Self{remote:Pulse::default(),nunchuk:Pulse::default(),buttons:0,last_tb:0,started:false,acc:REST,fs_acc:REST,speaker:Default::default(),volume,trace,streamed:0,played:0,wiggle:Wiggle::default(),trace_pad,traced:(0,[0;2],0,0)}
    }
    /// One KPADReadEx: advance the gesture clock by the whole 200 Hz periods since
    /// the previous read (the newest sample wins, as status[0] did) and convert.
    pub fn sample(&mut self,packet:[u8;8],tb:u64)->Option<MwPadSample> {
        if (packet[6]&15)>9||(packet[7]&15)>9 {return None;}
        let buttons=u16::from_be_bytes([packet[0],packet[1]]);
        let delay=if buttons&4!=0&&self.buttons&4==0 {20}else{0};
        let steps=if !self.started {self.started=true;self.last_tb=tb;1}
            else if tb<self.last_tb||tb-self.last_tb>MAX_STEPS*TICK {self.last_tb=tb;1}
            else {let n=(tb-self.last_tb)/TICK;self.last_tb+=n*TICK;n};
        // The former transport also sent a legacy "shake" byte; PcControls never sets it.
        let mut remote_token=if packet[5]!=0&&packet[6]&15==0 {9}else{packet[6]};
        let mut nunchuk_token=packet[7];
        if self.wiggle.active(packet[3],tb)&&remote_token&15==0 {remote_token=SHAKE_TOKEN;nunchuk_token=SHAKE_TOKEN;}
        for _ in 0..steps {
            let remote=self.remote.sample(remote_token,delay).unwrap_or(REST);
            let nunchuk=self.nunchuk.sample(nunchuk_token,0).unwrap_or(REST);
            self.acc=quantize(remote,REMOTE_LSB,-REMOTE_ZERO,255.-REMOTE_ZERO);
            self.fs_acc=quantize(nunchuk,NUNCHUK_LSB,-512.,511.);
        }
        self.buttons=buttons;
        // Host packet is remote wire order (BE); WPAD hold is that byte-swapped.
        let mut hold=u32::from(buttons.swap_bytes())&0x9f1f;
        if packet[2]&1!=0 {hold|=0x2000;}
        if packet[2]&2!=0 {hold|=0x4000;}
        let stick=|raw:u8|((raw as f32-128.)/96.).clamp(-1.,1.);
        let out=MwPadSample{hold,acc:kpad_axes(self.acc),fs_acc:kpad_axes(self.fs_acc),stick:[stick(packet[3]),stick(packet[4])]};
        // Default-off: MW_TRACE_PAD=1 logs what KPADReadEx hands the game when it changes.
        let seen=(hold,out.stick.map(|v|(v*10.).round() as i8),remote_token,nunchuk_token);
        if self.trace_pad&&seen!=self.traced {self.traced=seen;eprintln!("[pad-hle] tb={tb} hold={hold:04x} stick={:?} remote={remote_token:02x} nunchuk={nunchuk_token:02x}",out.stick);}
        Some(out)
    }
    /// The game's current remote prompt kind, or 0 when none is shown.
    pub fn set_prompt(&mut self,kind:u8){self.wiggle.prompt=kind;}
    /// WPADControlSpeaker: 0 off, 1 on, 2 mute, 3 unmute, 4 play.
    pub fn speaker_control(&mut self,chan:usize,cmd:u32)->bool {
        let Some(s)=self.speaker.get_mut(chan) else {return false};
        match cmd {0=>*s=Speaker::default(),1=>{s.on=true;s.reset_decoder();},2=>s.muted=true,3=>s.muted=false,4=>s.reset_decoder(),_=>{}}
        if self.trace {eprintln!("REMOTE_AUDIO_HLE_CONTROL {{\"chan\":{chan},\"cmd\":{cmd},\"on\":{},\"muted\":{}}}",s.on,s.muted);}
        true
    }
    pub fn speaker_on(&self,chan:usize)->bool {self.speaker.get(chan).is_some_and(|s|s.on)}
    /// WPADSendStreamData payload: 6 kHz Yamaha ADPCM (the format and rate the
    /// original WPAD speaker setup selects) mixed through the existing mono lane.
    pub fn speaker_stream(&mut self,chan:usize,data:&[u8])->Option<Vec<i16>> {
        let s=self.speaker.get_mut(chan)?;self.streamed+=1;
        let out=s.stream(data);self.played+=u64::from(out.is_some());out
    }
    fn save(&self,w:&mut StateWriter){
        self.remote.save(w);self.nunchuk.save(w);w.u16(self.buttons);w.pod(&self.last_tb);w.bool(self.started);
        w.pod(&self.acc);w.pod(&self.fs_acc);
        for s in &self.speaker {w.bool(s.on);w.bool(s.muted);w.pod(&s.predictor);w.pod(&s.step);}
    }
    fn load(&mut self,r:&mut StateReader<'_>)->Result<(),StateError>{
        self.remote.load(r)?;self.nunchuk.load(r)?;self.buttons=r.u16()?;self.last_tb=r.pod()?;self.started=r.bool()?;
        self.acc=r.pod()?;self.fs_acc=r.pod()?;
        for s in &mut self.speaker {
            s.on=r.bool()?;s.muted=r.bool()?;s.predictor=r.pod()?;s.step=r.pod()?;
            if !(-32768..=32767).contains(&s.predictor)||!(127..=24576).contains(&s.step){return Err(StateError::Corrupt("speaker codec state"));}
        }
        Ok(())
    }
}
impl Drop for Pad {
    fn drop(&mut self){if self.trace{eprintln!("REMOTE_AUDIO_HLE {{\"streamPackets\":{},\"playedPackets\":{}}}",self.streamed,self.played);}}
}
#[unsafe(no_mangle)]
pub extern "C" fn mw_pad_create()->*mut Pad {catch_unwind(||Box::into_raw(Box::new(Pad::new()))).unwrap_or(std::ptr::null_mut())}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_destroy(h:*mut Pad){if !h.is_null(){drop(unsafe{Box::from_raw(h)});}}
/// `input` is the core's own 8-byte packet (MW_INPUT_FILE or neutral); the
/// Bevy host packet replaces it whenever a host is installed. 0 ok, -2 bad packet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_sample(h:*mut Pad,input:*const u8,tb:u64,out:*mut MwPadSample)->i32 {
    catch_unwind(AssertUnwindSafe(||{
        let packet=crate::host::controller().unwrap_or_else(||unsafe{std::slice::from_raw_parts(input,8)}.try_into().expect("controller packet"));
        match unsafe{&mut *h}.sample(packet,tb) {Some(s)=>{unsafe{*out=s;}0},None=>-2}
    })).unwrap_or(-1000)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_set_prompt(h:*mut Pad,kind:u32)->i32 {
    catch_unwind(AssertUnwindSafe(||{unsafe{&mut *h}.set_prompt(kind as u8);crate::host::set_remote_prompt(kind as u8);0})).unwrap_or(-1000)
}
/// WPADControlMotor(chan, on): forwarded to the frontend as gamepad rumble (optional ABI;
/// older cores never call it). The handle is unused; motor state is host presentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_motor(_h:*mut Pad,chan:u32,on:u32)->i32 {crate::host::set_rumble(chan,on!=0);0}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_speaker_control(h:*mut Pad,chan:u32,cmd:u32)->i32 {
    catch_unwind(AssertUnwindSafe(||if unsafe{&mut *h}.speaker_control(chan as usize,cmd){0}else{-2})).unwrap_or(-1000)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_speaker_on(h:*mut Pad,chan:u32)->i32 {
    catch_unwind(AssertUnwindSafe(||i32::from(unsafe{&*h}.speaker_on(chan as usize)))).unwrap_or(-1000)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_speaker_stream(sys:*mut System<WII>,h:*mut Pad,chan:u32,data:*const u8,len:u32)->i32 {
    catch_unwind(AssertUnwindSafe(||{
        if len>64 {return -2;}
        let pad=unsafe{&mut *h};let data=unsafe{std::slice::from_raw_parts(data,len as usize)};
        if let Some(samples)=pad.speaker_stream(chan as usize,data) {
            let volume=pad.volume;unsafe{&mut *sys}.audio_sink.push_remote_mono(&samples,6000,volume);
        }
        0
    })).unwrap_or(-1000)
}
/// PC free camera orbit for runtime/game/pc_camera.cpp: radians since the previous call;
/// flags bit0 = auto-follow, bit1 = orbit while locked, bits 8..15 = signed lock-on target cycle. 0 ok.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_camera_take(yaw:*mut f32,pitch:*mut f32,flags:*mut u32)->i32 {
    catch_unwind(AssertUnwindSafe(||{let (y,p,cycle,follow,lock)=crate::host::take_camera_orbit();unsafe{*yaw=y;*pitch=p;*flags=u32::from(follow)|u32::from(lock)<<1|u32::from(cycle as u8)<<8;}0})).unwrap_or(-1000)
}
unsafe fn output(bytes:Vec<u8>,out:*mut *mut u8,len:*mut usize){
    let mut bytes=bytes.into_boxed_slice();
    unsafe{*len=bytes.len();*out=bytes.as_mut_ptr();}std::mem::forget(bytes);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_state_save(h:*mut Pad,out:*mut *mut u8,len:*mut usize)->i32 {
    catch_unwind(AssertUnwindSafe(||{let mut w=StateWriter::new();unsafe{&*h}.save(&mut w);unsafe{output(w.into_inner(),out,len);}0})).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_pad_state_load(h:*mut Pad,data:*const u8,len:usize)->i32 {
    if len>1024*1024{return -2;}
    catch_unwind(AssertUnwindSafe(||{
        let pad=unsafe{&mut *h};let mut r=StateReader::new(unsafe{std::slice::from_raw_parts(data,len)});
        if pad.load(&mut r).is_err()||r.remaining()!=0{return -2;}0
    })).unwrap_or(-1)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::{token,Gesture};
    const NEUTRAL:[u8;8]=[0,0,0,128,128,0,0,0];
    fn near(a:[f32;3],b:[f32;3])->bool{a.iter().zip(b).all(|(x,y)|(x-y).abs()<0.03)}
    #[test]fn neutral_is_kpad_rest_and_buttons_swap_to_wpad(){
        let mut p=Pad::new();let s=p.sample(NEUTRAL,0).unwrap();
        assert_eq!(s.acc,[0.,-1.,0.]);assert_eq!(s.fs_acc,[0.,-1.,0.]);assert_eq!(s.stick,[0.,0.]);assert_eq!(s.hold,0);
        // Wire A 0x0008 -> WPAD A 0x0800; B 0x0004 -> 0x0400; Plus 0x1000 -> 0x0010; Home 0x0080 -> 0x8000.
        for (wire,wpad) in [(8u16,0x800u32),(4,0x400),(0x1000,0x10),(0x10,0x1000),(0x80,0x8000),(0x0100,1),(0x0800,8)] {
            let mut packet=NEUTRAL;packet[..2].copy_from_slice(&wire.to_be_bytes());assert_eq!(p.sample(packet,0).unwrap().hold,wpad);
        }
        let mut packet=NEUTRAL;packet[2]=3;assert_eq!(p.sample(packet,0).unwrap().hold,0x6000);
        packet[3]=255;packet[4]=0;assert_eq!(p.sample(packet,0).unwrap().stick,[1.,-1.]);
    }
    #[test]fn gestures_run_on_the_200hz_clock_not_per_read(){
        let mut p=Pad::new();let mut packet=NEUTRAL;packet[6]=token(0,Gesture::Right);
        p.sample(packet,0);// first read is step 0 of the impulse
        // One 60 Hz frame is 3.33 ticks; after 12 ticks of real time the impulse is at full 2 g.
        let mut tb=0;let mut last=None;for _ in 0..4 {tb+=TICK*3;last=p.sample(packet,tb);}
        assert!(near(last.unwrap().acc,[-2.,-1.,0.]),"{last:?}");
        // No time elapsed: the sample does not advance.
        assert_eq!(p.sample(packet,tb),last);
        // Raw remote +Y maps to KPAD +Z, raw +Z (up) to KPAD -Y.
        let mut q=Pad::new();packet[6]=token(0,Gesture::Forward);q.sample(packet,0);assert!(near(q.sample(packet,TICK*12).unwrap().acc,[0.,-1.,2.]));
    }
    #[test]fn held_shake_keeps_the_finisher_signal(){
        // VICTORY-MOTION-INPUT: phase 13 integrates |‖acc‖-1|; held X (gesture 9) measured mean 1.59.
        let mut p=Pad::new();let mut packet=NEUTRAL;packet[6]=token(0,Gesture::Shake);packet[7]=packet[6];
        let mut sum=0.;let frames=120;let mut tb=0;
        for f in 0..frames {tb=(f as u64*TICK*10)/3;let s=p.sample(packet,tb).unwrap();let n=s.acc.iter().map(|v|v*v).sum::<f32>().sqrt();sum+=(n-1.).abs();
            let m=s.fs_acc.iter().map(|v|v*v).sum::<f32>().sqrt();assert!((m-1.).abs()>0.5);}
        let mean=sum/frames as f32;assert!(mean>1.2&&mean<2.3,"{mean}");
        packet[6]=0;packet[7]=0;assert_eq!(p.sample(packet,tb+TICK).unwrap().acc,[0.,-1.,0.]);
    }
    #[test]fn stick_wiggle_shakes_only_under_the_shake_prompt(){
        let shake=|p:&mut Pad,prompt:u8|{
            p.set_prompt(prompt);let mut packet=NEUTRAL;let mut deviation=0.;let mut tb=0;
            // A/D alternating every 0.1 s (5 reversals/s), sampled at 60 Hz for 2 s.
            for f in 0..120u64 {tb=f*TICK*10/3;packet[3]=if (tb/(TICK*20))%2==0 {1}else{255};
                let s=p.sample(packet,tb).unwrap();deviation+=(s.acc.iter().map(|v|v*v).sum::<f32>().sqrt()-1.).abs();}
            packet[3]=128;(deviation/120.,p.sample(packet,tb+TICK*90).unwrap().acc)
        };
        let (mean,after)=shake(&mut Pad::new(),0x11);assert!(mean>1.0,"{mean}");assert_eq!(after,[0.,-1.,0.]);
        let (mean,_)=shake(&mut Pad::new(),0);assert_eq!(mean,0.);// walking left/right outside the prompt
        let (mean,_)=shake(&mut Pad::new(),0x10);assert_eq!(mean,0.);// two-arm competition prompt
        // A single direction change is steering, not a shake.
        let mut p=Pad::new();p.set_prompt(0x11);let mut packet=NEUTRAL;packet[3]=1;p.sample(packet,0);packet[3]=255;
        assert_eq!(p.sample(packet,TICK*4).unwrap().acc,[0.,-1.,0.]);
    }
    #[test]fn motor_state_reaches_the_host_rumble_flag(){
        assert!(!crate::host::rumble(0));
        unsafe{mw_pad_motor(std::ptr::null_mut(),0,1);}assert!(crate::host::rumble(0)&&!crate::host::rumble(1));
        unsafe{mw_pad_motor(std::ptr::null_mut(),0,0);mw_pad_motor(std::ptr::null_mut(),9,1);}assert!(!crate::host::rumble(0));
    }
    #[test]fn speaker_decodes_while_muted_and_plays_only_when_unmuted(){
        let mut p=Pad::new();
        assert!(p.speaker_stream(0,&[0]).is_none());// off: decoder still advances
        p.speaker_control(0,1);assert_eq!(p.speaker_stream(0,&[0,0]).unwrap(),[15,30,45,60]);
        p.speaker_control(0,2);assert!(p.speaker_stream(0,&[0]).is_none());// muted: 75,90 consumed
        p.speaker_control(0,3);assert_eq!(p.speaker_stream(0,&[0]).unwrap(),[105,120]);
        p.speaker_control(0,4);assert_eq!(p.speaker_stream(0,&[0]).unwrap(),[15,30]);
        p.speaker_control(0,0);assert!(!p.speaker_on(0));assert!(!p.speaker_control(7,1));
    }
    #[test]fn state_round_trip_continues_motion_and_speaker(){
        let mut p=Pad::new();let mut packet=NEUTRAL;packet[6]=token(0,Gesture::Shake);
        p.sample(packet,0);p.sample(packet,TICK*5);p.speaker_control(1,1);p.speaker_stream(1,&[0x10]);
        let mut w=StateWriter::new();p.save(&mut w);let bytes=w.into_inner();
        let expect=(p.sample(packet,TICK*9),p.speaker_stream(1,&[0]));
        let mut q=Pad::new();let mut r=StateReader::new(&bytes);q.load(&mut r).unwrap();assert_eq!(r.remaining(),0);
        assert_eq!((q.sample(packet,TICK*9),q.speaker_stream(1,&[0])),expect);
    }
}
