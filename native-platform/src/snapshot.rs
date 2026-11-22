//! Native bridge snapshots. No PPC interpreter is run by these entry points.
use gecko::{System,WII,savestate::{StateWriter,StateReader},hollywood::ipc::IosDevice};
use std::panic::{catch_unwind,AssertUnwindSafe};
use crate::input::Controller;
unsafe fn output(bytes:Vec<u8>,out:*mut *mut u8,len:*mut usize){
    let mut bytes=bytes.into_boxed_slice();
    unsafe{*len=bytes.len();*out=bytes.as_mut_ptr();}std::mem::forget(bytes);
}
const HLE_TAG:&[u8;4]=b"AXHL";
fn split_hle(bytes:&[u8])->(&[u8],Option<&[u8]>){
    let n=bytes.len();
    if n<8||&bytes[n-4..]!=HLE_TAG{return (bytes,None);}
    let len=u32::from_le_bytes(bytes[n-8..n-4].try_into().unwrap()) as usize;
    if len>n-8{return (bytes,None);}
    (&bytes[..n-8-len],Some(&bytes[n-8-len..n-8]))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_state_free(data:*mut u8,len:usize){
    if !data.is_null(){drop(unsafe{Box::from_raw(std::ptr::slice_from_raw_parts_mut(data,len))});}
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_platform_state_save(h:*mut System<WII>,out:*mut *mut u8,len:*mut usize)->i32{
    catch_unwind(AssertUnwindSafe(||{
        let sys=unsafe{&mut *h};
        match sys.save_state(){Ok(mut bytes)=>{
            // AX HLE trailer: [state][u32 length][b"AXHL"]; older snapshots simply lack it.
            let mut w=StateWriter::new();sys.dsp.hle.save(&mut w);let hle=w.into_inner();
            bytes.extend_from_slice(&hle);bytes.extend_from_slice(&(hle.len() as u32).to_le_bytes());bytes.extend_from_slice(HLE_TAG);
            unsafe{output(bytes,out,len);}0},Err(e)=>{eprintln!("Native hardware save: {e}");-2}}
    })).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_platform_state_load(h:*mut System<WII>,data:*const u8,len:usize)->i32{
    if len>256*1024*1024{return -2;}
    catch_unwind(AssertUnwindSafe(||{
        let sys=unsafe{&mut *h};let bytes=unsafe{std::slice::from_raw_parts(data,len)};
        let (state,hle)=split_hle(bytes);
        if let Err(e)=sys.load_state(state){eprintln!("Native hardware load: {e}");return -2;}
        if let Some(hle)=hle{let mut r=StateReader::new(hle);if let Err(e)=sys.dsp.hle.load(&mut r){eprintln!("Native AX HLE load: {e}");return -2;}}
        0
    })).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_bt_state_save(h:*mut Controller,out:*mut *mut u8,len:*mut usize)->i32{
    catch_unwind(AssertUnwindSafe(||{
        let c=unsafe{&mut *h};let mut w=StateWriter::new();
        IosDevice::save_state(&mut c.bt,&mut w);c.remote.save(&mut w);c.nunchuk.save(&mut w);w.u16(c.buttons);
        unsafe{output(w.into_inner(),out,len);}0
    })).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mw_bt_state_load(h:*mut Controller,data:*const u8,len:usize)->i32{
    if len>16*1024*1024{return -2;}
    catch_unwind(AssertUnwindSafe(||{
        let c=unsafe{&mut *h};let mut r=StateReader::new(unsafe{std::slice::from_raw_parts(data,len)});
        let result=(||{IosDevice::load_state(&mut c.bt,&mut r)?;c.remote.load(&mut r)?;c.nunchuk.load(&mut r)?;c.buttons=r.u16()?;Ok::<_,gecko::savestate::StateError>(())})();
        if result.is_err()||r.remaining()!=0{return -2;}0
    })).unwrap_or(-1)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn borrowed_banks_and_hardware_restore(){
  let mut one=vec![0u8;24<<20];let mut two=vec![0u8;64<<20];
  let h=unsafe{crate::mw_dsp_create(one.as_mut_ptr(),two.as_mut_ptr())};assert!(!h.is_null());
  one[0x1234]=0x57;two[0x4321]=0x93;unsafe{(*h).scheduler.cycles=123456;(*h).dsp.dram[17]=0xab;}
  let mut data=std::ptr::null_mut();let mut len=0;
  unsafe{
   assert_eq!(mw_platform_state_save(h,&mut data,&mut len),0);one[0x1234]=0;two[0x4321]=0;(*h).scheduler.cycles=9;(*h).dsp.dram[17]=0;
   assert_eq!(mw_platform_state_load(h,data,len),0);assert_eq!((one[0x1234],two[0x4321]),(0x57,0x93));assert_eq!((*h).scheduler.cycles,123456);assert_eq!((*h).dsp.dram[17],0xab);
   mw_state_free(data,len);crate::mw_dsp_destroy(h);
  }
 }
 #[test]fn controller_pending_reports_and_motion_restore(){
  let h=crate::input::mw_bt_create();assert!(!h.is_null());let mut data=std::ptr::null_mut();let mut len=0;
  unsafe{
   (*h).buttons=0x1234;(*h).remote.sample(0x19,0);(*h).remote.sample(0x19,0);
   assert_eq!(mw_bt_state_save(h,&mut data,&mut len),0);let expected=(*h).remote.sample(0x19,0);(*h).buttons=0;
   assert_eq!(mw_bt_state_load(h,data,len),0);assert_eq!((*h).buttons,0x1234);assert_eq!((*h).remote.sample(0x19,0),expected);
   mw_state_free(data,len);crate::input::mw_bt_destroy(h);
  }
 }
}
