//! Native hardware adapter; original PPC gameplay remains statically recompiled.
#[cfg(feature="native-render")]
pub use backend_wgpu as native_renderer;
pub mod host;
pub mod motion;
pub use gecko as native_hardware;
mod audit;
mod input;
mod pad;
mod audio;
mod speaker;
mod video;
mod snapshot;
mod ax_hle_tests;
use gecko::{System,WII};
use std::panic::{catch_unwind,AssertUnwindSafe};

/// Default-off core-thread time in the DSP bridge (`host::perf`).
struct DspTimer(Option<u64>);
impl DspTimer {
    fn start()->Self{DspTimer(gecko::host::perf::ENABLED.load(std::sync::atomic::Ordering::Relaxed).then(gecko::host::perf::ticks))}
}
impl Drop for DspTimer {
    fn drop(&mut self){if let Some(t)=self.0{gecko::host::perf::DSP_TICKS.fetch_add(gecko::host::perf::ticks()-t,std::sync::atomic::Ordering::Relaxed);}}
}

#[unsafe(no_mangle)]
pub extern "C" fn mw_platform_abi()->u32 {1}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_dsp_create(mem1:*mut u8,mem2:*mut u8)->*mut System<WII> {
    catch_unwind(AssertUnwindSafe(||{
        let mut sys=Box::new(System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty()));
        #[cfg(feature="vertex-jit")]
        {sys.set_execution_mode(gecko::system::ExecutionMode::Interpreter);sys.gx.execution_mode=gecko::system::ExecutionMode::Jit;}
        unsafe{sys.mmio.attach_native_banks(mem1,mem2);}
        // High-level AX mixer replaces the DSP interpreter after the AX handshake (frame-exact
        // against it over the boss intro, research/PERF-AUDIO.md). MW_DSP_INTERPRETER=1 or MW_AX_HLE=0 opts out.
        sys.dsp.hle.enabled=!std::env::var("MW_DSP_INTERPRETER").is_ok_and(|v|v=="1")&&!std::env::var("MW_AX_HLE").is_ok_and(|v|v=="0");
        // MW_AX_HLE_VERIFY=1: interpreter stays authoritative; HLE output is compared per command list.
        sys.dsp.hle.verify=std::env::var("MW_AX_HLE_VERIFY").is_ok_and(|v|v=="1");
        host::attach_guest(mem1);
        let installed=host::take();let host_render=installed.is_some();
        if let Some((render,audio))=installed{sys.render_sink=render;sys.audio_sink=audio;}
        if let Some(path)=std::env::var_os("MW_AUDIO_WAV") {
            let capture=Box::new(audio::WavSink::create(path.into(),sys.ai.control.aid_sample_rate_hz()));
            let previous=std::mem::replace(&mut sys.audio_sink,Box::new(gecko::audio::EmptyAudioSink));
            sys.audio_sink=Box::new(gecko::audio::MultiplexAudioSink::new(vec![previous,capture]));
        }
        let previous=std::mem::replace(&mut sys.audio_sink,Box::new(gecko::audio::EmptyAudioSink));
        sys.audio_sink=Box::new(speaker::SpeakerMixer::new(previous,sys.ai.control.aid_sample_rate_hz()));
        if let Some(path)=std::env::var_os("MW_GX_AUDIT") {
            assert!(!host_render,"MW_GX_AUDIT requires the headless sink; host renderer must remain attached");
            let file=std::fs::File::create(path).expect("GX audit output");
            sys.render_sink=Box::new(audit::Sink::new(Some(file)).0);
        }
        Box::into_raw(sys)
    })).unwrap_or(std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_memory_tracking(h:*mut System<WII>,counter:*mut u64,pages:*mut u64,count:usize)->i32 {
    catch_unwind(AssertUnwindSafe(||{
        unsafe{(&mut *h).mmio.attach_native_dirty(counter,pages,count);}0
    })).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_gx_scene(_h:*mut System<WII>,scene:u32)->i32 {
    audit::mark_scene(scene);0
}

/// Decode an already-framed command against original shared guest memory.
/// Display lists are flattened by the CPU-side parser, except a list it has
/// verified as draw-only and unchanged (MW_GX_DL_PASSTHROUGH): that arrives as
/// one opcode-0x40 call, which the decoder reads from RAM itself.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_gx_command(h:*mut System<WII>,bytes:*const u8,length:usize)->i32 {
    catch_unwind(AssertUnwindSafe(||{
        if length==0||length>16*1024*1024{return -2;}
        let packet=unsafe{std::slice::from_raw_parts(bytes,length)};
        if packet[0]==0x40&&length!=9{return -2;}
        let s=unsafe{&mut *h};
        if (0x80..=0xbf).contains(&packet[0])&&!s.mmio.has_native_dirty_tracking() {
            // Native PPC writes bypass Gecko's tracked stores. Force content
            // rechecks so reusing a guest texture address cannot show stale data.
            s.mmio.mark_memory_dirty_range(0,0x1800000);
            s.mmio.mark_memory_dirty_range(0x10000000,0x4000000);
        }
        s.gx.fifo.extend_from_slice(packet);
        let timed=gecko::host::perf::ENABLED.load(std::sync::atomic::Ordering::Relaxed).then(gecko::host::perf::ticks);
        s.gx.drain_fifo(&mut s.mmio,s.render_sink.as_mut());
        if let Some(t)=timed{gecko::host::perf::GX_COMMAND_TICKS.fetch_add(gecko::host::perf::ticks()-t,std::sync::atomic::Ordering::Relaxed);}
        if !s.gx.fifo.is_empty(){return -3;}
        0
    })).unwrap_or(-1)
}

/// All functions require a live handle with exclusive, single-threaded access.
/// Guest RAM may be accessed by the caller only between these calls.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_dsp_read(h:*mut System<WII>,a:u32,n:u32,out:*mut u32)->i32 {
    let _timer=DspTimer::start();
    catch_unwind(AssertUnwindSafe(||{
        let s=unsafe{&mut *h};s.drain_dsp_synchronous(4096);
        match gecko::flipper::dsp::dsp_read(s,a&0x3fffffff,n){Some(v)=>{unsafe{*out=v;}0},None=>-2}
    })).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_dsp_write(h:*mut System<WII>,a:u32,n:u32,v:u32)->i32 {
    let _timer=DspTimer::start();
    catch_unwind(AssertUnwindSafe(||{
        let s=unsafe{&mut *h};
        if !gecko::flipper::dsp::dsp_write(s,a&0x3fffffff,n,v){return -2;}
        s.drain_dsp_synchronous(4096);0
    })).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_dsp_tick(h:*mut System<WII>,timebase:u64)->i32 {
    let _timer=DspTimer::start();
    catch_unwind(AssertUnwindSafe(||{
        let s=unsafe{&mut *h};
        // Cached: an environment lookup per DSP sync (every event and mailbox access) was measurable.
        static TRACE:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
        if *TRACE.get_or_init(||std::env::var_os("MW_TRACE_DSP").is_some()){
            static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
            if timebase>=NEXT.load(std::sync::atomic::Ordering::Relaxed){
                NEXT.store(timebase+60750000,std::sync::atomic::Ordering::Relaxed);
                eprintln!("[DSP] pc={:04x} csr={:04x} cpu-mail={:04x}{:04x} dsp-mail={:04x}{:04x}",s.dsp.registers.pc,s.dsp.csr.raw(),s.dsp.mailbox_to_dsp_hi.raw(),s.dsp.mailbox_to_dsp_lo.raw(),s.dsp.mailbox_to_cpu_hi.raw(),s.dsp.mailbox_to_cpu_lo.raw());
            }
        }
        let target=timebase.checked_mul(12).expect("DSP time overflow");
        let mut previous_deadline=None;
        let mut stalled_events=0u32;
        while s.scheduler.next_deadline()<=target {
            let deadline=s.scheduler.next_deadline();
            // A host shader compile can span many valid VI/DSP deadlines.
            // Bound only repeated work at one timestamp, never total catch-up.
            if previous_deadline==Some(deadline){stalled_events+=1;}else{stalled_events=0;previous_deadline=Some(deadline);}
            assert!(stalled_events<65536,"native hardware scheduler made no deadline progress");
            // Repeating DMA events schedule from their deadline, not the late
            // host poll; otherwise every late poll silently discards PCM frames.
            s.scheduler.cycles=s.scheduler.cycles.max(s.scheduler.next_deadline());
            let handler=s.scheduler.poll().expect("due native hardware event");
            assert!(matches!(handler,gecko::scheduler::Handler::DspBatch|gecko::scheduler::Handler::AramDma|gecko::scheduler::Handler::AiAudioDmaBlock|gecko::scheduler::Handler::ViHalfLine),"unsupported event in native hardware adapter");
            (handler.resolve::<WII>())(s);
        }
        s.scheduler.cycles=s.scheduler.cycles.max(target);
        s.drain_dsp_synchronous(4096);0
    })).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_audio_flush(h:*mut System<WII>)->i32 {
    catch_unwind(AssertUnwindSafe(||{unsafe{&mut *h}.audio_sink.flush();0})).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_ai_control(h:*mut System<WII>,value:u32)->i32 {
    use gecko::mmio::traits::{MmioAccess,WriteMask};
    catch_unwind(AssertUnwindSafe(||{
        gecko::flipper::ai::regs::AiControl::from_raw(value).write(unsafe{&mut *h},WriteMask::FULL_U32);0
    })).unwrap_or(-1)
}
/// Timebase tick at which the host must next poll (`mw_dsp_tick`) so that every
/// guest-visible interrupt (AI DMA completion, ARAM DMA) is observed at its own
/// deadline. 0 = keep the fixed cadence (the DSP interpreter needs its batches);
/// u64::MAX = nothing pending. Register accesses still synchronise on demand.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_dsp_poll_hint(h:*mut System<WII>)->u64 {
    catch_unwind(AssertUnwindSafe(||{
        let s=unsafe{&*h};
        if !s.dsp.hle.active{return 0;}
        let next=[gecko::flipper::ai::next_interrupt_deadline(s),s.scheduler.deadline_of(gecko::scheduler::Handler::AramDma)]
            .into_iter().flatten().min();
        next.map_or(u64::MAX,|cycles|cycles.div_ceil(12))
    })).unwrap_or(0)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_dsp_irq(h:*mut System<WII>)->u32 {
    u32::from(unsafe{&*h}.dsp.interrupt_active())
}

/// # Safety
/// Handle must come from mw_dsp_create, used on one thread, and destroyed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_dsp_destroy(handle:*mut System<WII>) {
    if !handle.is_null(){host::detach_guest();drop(unsafe{Box::from_raw(handle)});}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_dirty_pages_track_native_and_dsp_writes(){
        let mut one=vec![0;0x1800000];let mut two=vec![0;0x4000000];
        let mut pages=vec![0u64;22528];let mut generation=0u64;
        let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
        unsafe{sys.mmio.attach_native_banks(one.as_mut_ptr(),two.as_mut_ptr());
            assert_eq!(mw_memory_tracking(&mut sys,&mut generation,pages.as_mut_ptr(),pages.len()),0);}
        // Simulate a native write's shared metadata while no FFI borrow is live.
        generation+=1;pages[1]=generation;one[4096]=0x55;
        assert_eq!(sys.mmio.ram_view().memory_page_generations[1],1);
        assert_eq!(sys.mmio.virt_read_u8(0x80001000),0x55);
        sys.mmio.virt_write_u32(0x90003000,0x12345678);
        assert!(generation>1&&pages[6144+3]==generation);
        assert_eq!((pages[6144+2],pages[6144+4],pages[2]),(0,0,0));
        // Actual DSP DMA writes must invalidate only their native destination.
        sys.dsp.dram[0..8].copy_from_slice(&[1,2,3,4,5,6,7,8]);
        sys.dsp.dma_ram_addr_hi=0;sys.dsp.dma_ram_addr_lo=0x2000;
        sys.dsp.dma_dsp_addr=0;sys.dsp.dma_length=8;
        sys.dsp.dma_control=gecko::flipper::dsp::core::regs::DspDmaControl::new()
            .with_direction(gecko::flipper::dsp::core::regs::DspDmaDirection::DspToMain);
        let before=generation;sys.dsp.process_dsp_dma(&mut sys.mmio);
        assert_eq!(&one[0x2000..0x2008],&[1,2,3,4,5,6,7,8]);
        assert!(generation>before&&pages[2]==generation);
        assert_eq!(sys.mmio.ram_view().memory_page_generations[2],generation);
        assert_eq!(pages[3],0);
        drop(sys);one[0]=7;assert_eq!(one[0],7);
    }
    #[test]
    fn aid_dma_preserves_pcm_rate_and_late_poll_blocks() {
        use std::sync::{Arc,Mutex};
        struct Recorder(Arc<Mutex<Vec<(i16,i16)>>>,Arc<Mutex<Vec<u32>>>);
        impl gecko::audio::AudioSink for Recorder {
            fn push_stereo_i16(&mut self,l:i16,r:i16){self.0.lock().unwrap().push((l,r));}
            fn set_sample_rate(&mut self,rate:u32){self.1.lock().unwrap().push(rate);}
        }
        let frames=Arc::new(Mutex::new(Vec::new()));let rates=Arc::new(Mutex::new(Vec::new()));
        let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
        sys.audio_sink=Box::new(Recorder(frames.clone(),rates.clone()));
        let expected:Vec<_>=(0..8).map(|i|(-32768+i,32767-i)).collect();
        // Native AID is big-endian R,L; the host sink receives L,R.
        for (i,&(l,r)) in expected.iter().enumerate() {
            sys.mmio.virt_write_u16(0x80001000+i as u32*4,r as u16);
            sys.mmio.virt_write_u16(0x80001002+i as u32*4,l as u16);
        }
        unsafe {
            assert_eq!(mw_ai_control(&mut sys,64),0);
            assert_eq!(mw_dsp_write(&mut sys,0xcc005030,4,0x1000),0);
            assert_eq!(mw_dsp_write(&mut sys,0xcc005036,2,0x8001),0);
            assert_eq!(mw_dsp_tick(&mut sys,60750),0);
        }
        assert_eq!(frames.lock().unwrap().as_slice(),expected.repeat(4));
        unsafe {
            assert_eq!(mw_dsp_write(&mut sys,0xcc005036,2,0),0);
            assert_eq!(mw_ai_control(&mut sys,0),0);
            assert_eq!(mw_dsp_write(&mut sys,0xcc005036,2,0x8001),0);
            assert_eq!(mw_dsp_tick(&mut sys,121500),0);
            assert_eq!(mw_dsp_write(&mut sys,0xcc005036,2,0),0);
        }
        assert_eq!(&frames.lock().unwrap()[32..],expected.repeat(6));
        assert_eq!(*rates.lock().unwrap(),vec![48000]);
        unsafe{assert_eq!(mw_dsp_tick(&mut sys,182250),0);}
        assert_eq!(frames.lock().unwrap().len(),80);
    }
    #[test]
    fn free_rom_mailbox() {
        let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
        sys.dsp.registers.pc=0x8000;
        sys.dsp.csr.set_reset(false);
        sys.dsp.csr.set_halt(false);
        sys.drain_dsp_synchronous(256*1024);
        assert!(sys.dsp.mailbox_to_cpu_hi.busy());
        assert_eq!(sys.dsp.mailbox_to_cpu_hi.raw(),0x8071);
        assert_eq!(sys.dsp.mailbox_to_cpu_lo.raw(),0xfeed);
    }
    #[test]
    fn native_ram_shared_without_ownership_transfer(){
        let mut one=vec![0;0x1800000];let mut two=vec![0;0x4000000];
        let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
        unsafe{sys.mmio.attach_native_banks(one.as_mut_ptr(),two.as_mut_ptr());}
        one[32..36].copy_from_slice(&0x12345678u32.to_be_bytes());
        assert_eq!(sys.mmio.virt_read_u32(0x80000020),0x12345678);
        sys.mmio.virt_write_u32(0x90000020,0xabcdef01);
        assert_eq!(&two[32..36],&0xabcdef01u32.to_be_bytes());
        assert_eq!(sys.mmio.ram_view().slice(32,4),Some(&one[32..36]));
        let mut control=0;
        assert_eq!(unsafe{mw_dsp_read(&mut sys,0xcc00500a,2,&mut control)},0);
        assert_eq!(control&4,4);
        // Real DSP DMA reads MEM2 and writes MEM1 through the borrowed banks.
        two[0x100..0x108].copy_from_slice(&[1,2,3,4,5,6,7,8]);
        sys.dsp.dma_ram_addr_hi=0x1000;sys.dsp.dma_ram_addr_lo=0x100;
        sys.dsp.dma_dsp_addr=4;sys.dsp.dma_length=8;
        sys.dsp.dma_control=gecko::flipper::dsp::core::regs::DspDmaControl::new();
        sys.dsp.process_dsp_dma(&mut sys.mmio);
        assert_eq!(&sys.dsp.dram[8..16],&[1,2,3,4,5,6,7,8]);
        sys.dsp.dma_ram_addr_hi=0;sys.dsp.dma_ram_addr_lo=0x200;
        sys.dsp.dma_length=8;
        sys.dsp.dma_control.set_direction(gecko::flipper::dsp::core::regs::DspDmaDirection::DspToMain);
        sys.dsp.process_dsp_dma(&mut sys.mmio);
        assert_eq!(&one[0x200..0x208],&[1,2,3,4,5,6,7,8]);
        assert_eq!((one[0x1ff],one[0x208]),(0,0));
        drop(sys);one[32]=5;two[32]=6;assert_eq!(one[32]+two[32],11);
    }
    #[test]
    fn gx_audit_filters_boot_and_caps_selected_scene(){
        let path=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../evidence/synthetic-scene-gx.jsonl");
        let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
        let (sink,draws)=audit::Sink::with_filter(Some(std::fs::File::create(&path).unwrap()),Some(0x201));
        sys.render_sink=Box::new(sink);audit::mark_scene(0);
        let send=|sys:&mut System<WII>,packet:&[u8]|assert_eq!(unsafe{mw_gx_command(sys,packet.as_ptr(),packet.len())},0);
        send(&mut sys,&[8,0x50,0,0,2,0]);send(&mut sys,&[8,0x70,0,0,0,9]);
        let mut triangle=vec![0x90,0,3];
        for xyz in [[0f32,0.,0.],[1.,0.,0.],[0.,1.,0.]]{for x in xyz{triangle.extend(x.to_be_bytes());}}
        send(&mut sys,&triangle);assert!(std::fs::read_to_string(&path).unwrap().is_empty());
        audit::mark_scene(0x201);
        for _ in 0..65 {send(&mut sys,&triangle);}
        let text=std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(),64);
        assert!(text.lines().all(|line|line.contains("\"scene\":513")&&line.contains("\"finitePositions\":true")));
        assert_eq!(draws.load(std::sync::atomic::Ordering::Relaxed),66);
        audit::mark_scene(0);
    }
    #[test]
    fn native_gx_triangle_packet(){
        let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
        let (sink,draws)=audit::Sink::new(None);sys.render_sink=Box::new(sink);
        let mut send=|packet:&[u8]|assert_eq!(unsafe{mw_gx_command(&mut sys,packet.as_ptr(),packet.len())},0);
        send(&[8,0x50,0,0,2,0]);send(&[8,0x70,0,0,0,9]);
        let mut triangle=vec![0x90,0,3];
        for xyz in [[0f32,0.,0.],[1.,0.,0.],[0.,1.,0.]]{for x in xyz{triangle.extend(x.to_be_bytes());}}
        send(&triangle);
        assert_eq!(draws.load(std::sync::atomic::Ordering::Relaxed),1);
    }
}
