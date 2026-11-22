//! VI mirror used only for native GX/XFB presentation. CPU reads/IRQs remain C++ owned.
use gecko::{System,WII};
use std::panic::{catch_unwind,AssertUnwindSafe};
/// Same exclusive single-threaded handle contract as all other hardware calls.
/// Returns 1 for a reserved/unimplemented VI register; -2 for malformed access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_vi_write(h:*mut System<WII>,address:u32,size:u32,value:u32)->i32 {
    if !matches!(size,1|2|4)||address&(size-1)!=0{return -2;}
    let address=address&0x3fffffff;
    if !(0x0c002000..0x0c002100).contains(&address){return -2;}
    catch_unwind(AssertUnwindSafe(||{
        i32::from(!gecko::flipper::vi::vi_write(unsafe{&mut *h},address,size,value))
    })).unwrap_or(-1)
}
#[cfg(test)]
mod tests {
    use super::*;
    use gecko::host::{DrawVertex,GxAction,RenderSink};
    use std::sync::{Arc,Mutex};
    struct Recorder(Vec<DrawVertex>,Arc<Mutex<Vec<(u32,u32,Vec<u32>)>>>);
    impl RenderSink for Recorder {
        fn vertex_scratch(&mut self)->&mut Vec<DrawVertex>{&mut self.0}
        fn exec(&mut self,a:GxAction){if let GxAction::PresentRawXfb{width,height,pixels}=a{
            self.1.lock().unwrap().push((width,height,pixels));
        }}
    }
    #[test]
    fn vi_scanout_latches_framebuffer_and_preserves_guest_memory(){
        let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
        let output=Arc::new(Mutex::new(Vec::new()));
        sys.render_sink=Box::new(Recorder(Vec::new(),output.clone()));
        // A tiny progressive synthetic signal: 32x4, one blank half-line at
        // each end. Two CPU-written YUV422 buffers have different luminance.
        for (base,y) in [(0x1000,32u8),(0x2000,200u8)] {
            for offset in (0..32*4*2).step_by(4){sys.mmio.phys_write_u32(base+offset,u32::from_be_bytes([y,128,y,128]));}
        }
        unsafe {
            for (offset,size,value) in [(0,2,4<<4),(2,2,5),(12,4,0x10001),
                (16,4,0x10001),(28,4,0x1000),(72,2,2<<8),(4,4,429)] {
                assert_eq!(mw_vi_write(&mut sys,0xcc002000+offset,size,value),0);
            }
            assert_eq!(mw_vi_write(&mut sys,0xcc002001,2,0),-2);
            assert_eq!(mw_vi_write(&mut sys,0xcc003000,4,0),-2);
            let half=sys.vi.ticks_per_half_line(WII);
            assert_eq!(crate::mw_dsp_tick(&mut sys,half.div_ceil(12)),0);
            assert_eq!(sys.vi.latched_xfb_base,0x1000);
            // A mid-field flip must not change the already latched scanout.
            assert_eq!(mw_vi_write(&mut sys,0xcc00201c,4,0x2000),0);
            assert_eq!(crate::mw_dsp_tick(&mut sys,(half*9).div_ceil(12)),0);
            assert_eq!(output.lock().unwrap().len(),1);
            assert_eq!(crate::mw_dsp_tick(&mut sys,(half*19).div_ceil(12)),0);
        }
        let output=output.lock().unwrap();
        assert_eq!(output.len(),2);
        assert_eq!((output[0].0,output[0].1),(32,4));
        assert_eq!(output[0].2.len(),128);
        assert_ne!(output[0].2,output[1].2);
        assert_eq!(sys.mmio.phys_read_u32(0x1000),0x20802080);
        assert_eq!(sys.mmio.phys_read_u32(0x2000),0xc880c880);
    }
    #[test]
    fn long_native_poll_preserves_all_video_deadlines() {
        fn setup()->(System<WII>,Arc<Mutex<Vec<(u32,u32,Vec<u32>)>>>) {
            let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
            let output=Arc::new(Mutex::new(Vec::new()));
            sys.render_sink=Box::new(Recorder(Vec::new(),output.clone()));
            for offset in (0..32*4*2).step_by(4){sys.mmio.phys_write_u32(0x1000+offset,0x80808080);}
            for (offset,size,value) in [(0,2,4<<4),(2,2,5),(12,4,0x10001),(16,4,0x10001),(28,4,0x1000),(72,2,2<<8),(4,4,429)] {
                assert_eq!(unsafe{mw_vi_write(&mut sys,0xcc002000+offset,size,value)},0);
            }
            (sys,output)
        }
        let (mut late,late_frames)=setup();let (mut timely,timely_frames)=setup();
        let half=late.vi.ticks_per_half_line(WII);
        assert_eq!(unsafe{crate::mw_dsp_tick(&mut late,(half*70000).div_ceil(12))},0);
        for step in 1..=70 {
            assert_eq!(unsafe{crate::mw_dsp_tick(&mut timely,(half*step*1000).div_ceil(12))},0);
        }
        let late_frames=late_frames.lock().unwrap();let timely_frames=timely_frames.lock().unwrap();
        assert_eq!(late_frames.len(),7000);
        assert_eq!(*late_frames,*timely_frames);
        assert_eq!(late.scheduler.cycles,timely.scheduler.cycles);
        assert_eq!(late.scheduler.next_deadline(),timely.scheduler.next_deadline());
    }

}
