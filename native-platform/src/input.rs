//! Virtual controller transport. Host input arrives through an explicit packet,
//! never through global keyboard polling or desktop input injection.
use gecko::{System,WII,hollywood::ipc::{DeviceContext,IosDevice,usb::Bluetooth}};
use std::panic::{catch_unwind,AssertUnwindSafe};
pub struct Controller {pub(crate) bt:Bluetooth,pub(crate) remote:crate::motion::Pulse,pub(crate) nunchuk:crate::motion::Pulse,pub(crate) buttons:u16}
#[unsafe(no_mangle)]
pub extern "C" fn mw_bt_create()->*mut Controller {
    if std::env::var("MW_TRACE_REMOTE_SPEAKER").is_ok_and(|value|value=="1") {eprintln!("REMOTE_AUDIO_TRACE enabled=1");}
    catch_unwind(||{let mut bt=Bluetooth::new();bt.set_wiimote_options(true,false);Box::into_raw(Box::new(Controller{bt,remote:Default::default(),nunchuk:Default::default(),buttons:0}))}).unwrap_or(std::ptr::null_mut())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_bt_destroy(bt:*mut Controller){if !bt.is_null(){drop(unsafe{Box::from_raw(bt)});}}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_bt_ioctlv(h:*mut System<WII>,bt:*mut Controller,cmd:u32,ni:u32,no:u32,vectors:u32)->i32 {
    catch_unwind(AssertUnwindSafe(||{
        if no!=1||!matches!((cmd,ni),(0,6)|(1,2)|(2,2)){return -4;}
        let s=unsafe{&mut *h};let bt=&mut unsafe{&mut *bt}.bt;
        let vectors=vectors&0x3fffffff;
        if s.mmio.ram_view().slice(vectors as usize,((ni+no)*8) as usize).is_none(){return -4;}
        for i in 0..ni+no {
            let p=s.mmio.phys_read_u32(vectors+i*8)&0x3fffffff;
            let n=s.mmio.phys_read_u32(vectors+i*8+4) as usize;
            if n>65536||(n!=0&&s.mmio.ram_view().slice(p as usize,n).is_none()){return -4;}
        }
        let at=|i:u32|s.mmio.phys_read_u32(vectors+i*8)&0x3fffffff;
        let capacity=s.mmio.phys_read_u32(vectors+ni*8+4);
        let length=if cmd==0 {let p=at(4);u16::from_le_bytes([s.mmio.phys_read_u8(p),s.mmio.phys_read_u8(p+1)]) as u32}
            else{s.mmio.phys_read_u16(at(1)) as u32};
        if length>capacity{return -4;}
        // The upstream implementation consumes USB V0's original guest vectors.
        let mut ctx=DeviceContext{mmio:&mut s.mmio,scheduler:&mut s.scheduler,di:&mut s.di,pi:&mut s.pi,device_path:"/dev/usb/oh1/57e/305".into(),reload_ios:None};
        let result=bt.ioctlv(&mut ctx,cmd,ni,no,vectors);
        for block in bt.take_speaker_audio(){
            if std::env::var("MW_TRACE_REMOTE_SPEAKER").is_ok_and(|v|v=="1") {eprintln!("REMOTE_AUDIO_DECODE {{\"samples\":{},\"rate\":{},\"volume\":{}}}",block.samples.len(),block.rate,block.volume);}
            s.audio_sink.push_remote_mono(&block.samples,block.rate,block.volume);
        }
        result
    })).unwrap_or(-1000)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_bt_tick(bt:*mut Controller,input:*const u8)->i32 {
    catch_unwind(AssertUnwindSafe(||{
        let packet=crate::host::controller().unwrap_or_else(||unsafe{std::slice::from_raw_parts(input,8)}.try_into().expect("controller packet"));
        let p=&packet;let control=unsafe{&mut *bt};
        if (p[6]&15)>9 || (p[7]&15)>9 {return -2;}
        let buttons=u16::from_be_bytes([p[0],p[1]]);
        let delay=if buttons&4!=0 && control.buttons&4==0 {20}else{0};
        let remote=control.remote.sample(p[6],delay);
        let nunchuk=control.nunchuk.sample(p[7],0);
        let bt=&mut control.bt;control.buttons=buttons;
        bt.set_wiimote_buttons(buttons);bt.set_nunchuk(p[2],p[3],p[4]);
        bt.set_wiimote_shake(p[5]!=0);bt.set_wiimote_accel(remote);
        bt.set_nunchuk_accel(nunchuk);bt.tick_input_report();0
    })).unwrap_or(-1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn usb_hci_reset_uses_original_guest_vectors(){
        let mut s=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());
        let bt=mw_bt_create();assert!(!bt.is_null());
        // Six USB setup buffers followed by HCI data, in physical MEM1.
        for (i,(p,n)) in [(0x200,1),(0x201,1),(0x202,2),(0x204,2),(0x206,2),(0x208,1),(0x300,3)].iter().enumerate(){
            s.mmio.phys_write_u32(0x100+i as u32*8,*p);s.mmio.phys_write_u32(0x104+i as u32*8,*n);
        }
        s.mmio.phys_write_u8(0x200,0x20);s.mmio.phys_slice_mut(0x206,2).copy_from_slice(&[3,0]);
        s.mmio.phys_slice_mut(0x300,3).copy_from_slice(&[3,12,0]);
        assert_eq!(unsafe{mw_bt_ioctlv(&mut s,bt,0,6,1,0x100)},0);
        for (i,(p,n)) in [(0x400,1),(0x402,2),(0x500,64)].iter().enumerate(){s.mmio.phys_write_u32(0x180+i as u32*8,*p);s.mmio.phys_write_u32(0x184+i as u32*8,*n);}
        s.mmio.phys_write_u8(0x400,0x81);s.mmio.phys_write_u16(0x402,64);
        assert_eq!(unsafe{mw_bt_ioctlv(&mut s,bt,2,2,1,0x180)},6);
        assert_eq!(s.mmio.phys_slice(0x500,6),&[14,4,1,3,12,0]);
        s.mmio.phys_slice_mut(0x300,3).copy_from_slice(&[0x34,0x12,0]);
        assert_eq!(unsafe{mw_bt_ioctlv(&mut s,bt,0,6,1,0x100)},0);
        assert_eq!(unsafe{mw_bt_ioctlv(&mut s,bt,2,2,1,0x180)},6);
        assert_eq!(s.mmio.phys_slice(0x500,6),&[14,4,1,0x34,0x12,1]);
        // Bulk OUT must return the original native BE transfer length.
        s.mmio.phys_write_u8(0x400,2);s.mmio.phys_write_u16(0x402,8);
        s.mmio.phys_write_u32(0x194,8);s.mmio.phys_slice_mut(0x500,8).fill(0);
        assert_eq!(unsafe{mw_bt_ioctlv(&mut s,bt,1,2,1,0x180)},8);
        unsafe{mw_bt_destroy(bt);}
    }
}

#[cfg(test)]mod speaker_transport_tests {
 use super::*;use gecko::audio::{AudioSink,AidBlock};use std::sync::{Arc,Mutex};
 struct Recorder(Arc<Mutex<Vec<(i16,i16)>>>);impl AudioSink for Recorder{fn push_stereo_i16(&mut self,l:i16,r:i16){self.0.lock().unwrap().push((l,r));}}
 #[test]fn native_usb_acl_speaker_reaches_stereo_sink(){
  let mut sys=System::<WII>::with_scheduler(0,gecko::scheduler::Scheduler::empty());let frames=Arc::new(Mutex::new(Vec::new()));sys.audio_sink=Box::new(crate::speaker::SpeakerMixer::new(Box::new(Recorder(frames.clone())),32000));
  let bt=mw_bt_create();assert!(!bt.is_null());
  for(i,(p,n))in[(0x200,1),(0x202,2),(0x300,64)].iter().enumerate(){sys.mmio.phys_write_u32(0x100+i as u32*8,*p);sys.mmio.phys_write_u32(0x104+i as u32*8,*n);}
  sys.mmio.phys_write_u8(0x200,2);
  let mut send=|payload:&[u8]|{
   let mut acl=vec![0x0b,0x20];acl.extend(((payload.len()+4)as u16).to_le_bytes());acl.extend((payload.len()as u16).to_le_bytes());acl.extend(0x40u16.to_le_bytes());acl.extend(payload);
   sys.mmio.phys_write_u16(0x202,acl.len()as u16);sys.mmio.phys_slice_mut(0x300,acl.len()).copy_from_slice(&acl);
   assert_eq!(unsafe{mw_bt_ioctlv(&mut sys,bt,1,2,1,0x100)},acl.len()as i32);
  };
  send(&[0xa2,0x14,6]);send(&[0xa2,0x16,4,0xa2,0,1,7,0,0,0xd0,7,127,12,14]);send(&[0xa2,0x16,4,0xa2,0,8,2,1,1]);send(&[0xa2,0x18,16,0,0]);
  let silence:AidBlock=[(0,0);8];sys.audio_sink.push_stereo_block(&silence);assert_eq!(*frames.lock().unwrap(),[(15,15),(18,18),(21,21),(23,23),(26,26),(29,29),(32,32),(35,35)]);
  unsafe{mw_bt_destroy(bt);}
 }
}
