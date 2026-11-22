//! Center the original remote mono stream in the PC stereo output.
//! AID remains the clock; the optional lane uses bounded linear resampling.
use gecko::audio::{AudioSink,AidBlock};
use std::collections::VecDeque;
pub struct SpeakerMixer {inner:Box<dyn AudioSink>,rate:u32,remote:VecDeque<(i16,u32)>,phase:f64,dropped:u64,received:u64,mixed:u64,clipped:u64}
impl SpeakerMixer {
 pub fn new(inner:Box<dyn AudioSink>,rate:u32)->Self {Self{inner,rate,remote:VecDeque::new(),phase:0.,dropped:0,received:0,mixed:0,clipped:0}}
 fn sample(&mut self)->i32 {
  let Some(&(first,rate))=self.remote.front() else {self.phase=0.;return 0};
  self.mixed+=1;
  let second=self.remote.get(1).map_or(first,|p|p.0);
  let value=(first as f64+(second as f64-first as f64)*self.phase).round() as i32;
  self.phase+=rate as f64/self.rate as f64;
  while self.phase>=1. && !self.remote.is_empty(){self.remote.pop_front();self.phase-=1.;}
  if self.remote.is_empty(){self.phase=self.phase.fract();}value
 }
}
impl AudioSink for SpeakerMixer {
 fn set_sample_rate(&mut self,rate:u32){assert!(matches!(rate,32000|48000));self.rate=rate;self.inner.set_sample_rate(rate);}
 fn push_remote_mono(&mut self,samples:&[i16],rate:u32,volume:u8){
  if rate==0{return}
  self.received+=samples.len() as u64;
  let limit=(rate as usize).clamp(1024,192000);
  for &sample in samples {
   if self.remote.len()>=limit {self.remote.pop_front();self.dropped+=1;}
   let sample=(sample as i32*volume.min(127) as i32/127) as i16;
   self.remote.push_back((sample,rate));
  }
 }
 fn push_stereo_i16(&mut self,left:i16,right:i16){let mono=self.sample();let l=left as i32+mono;let r=right as i32+mono;self.clipped+=u64::from(!(-32768..=32767).contains(&l))+u64::from(!(-32768..=32767).contains(&r));self.inner.push_stereo_i16(l.clamp(-32768,32767) as i16,r.clamp(-32768,32767) as i16);}
 fn push_stereo_block(&mut self,frames:&AidBlock){
  if self.remote.is_empty(){self.phase=0.;self.inner.push_stereo_block(frames);}else{for &(l,r) in frames {self.push_stereo_i16(l,r);}}
 }
 fn flush(&mut self){self.inner.flush();}
 fn should_throttle(&self)->bool{self.inner.should_throttle()}
}
#[cfg(test)]mod tests {
 use super::*;use std::sync::{Arc,Mutex};
 struct Recorder(Arc<Mutex<Vec<(i16,i16)>>>);impl AudioSink for Recorder{fn push_stereo_i16(&mut self,l:i16,r:i16){self.0.lock().unwrap().push((l,r));}}
 fn mixer()->(SpeakerMixer,Arc<Mutex<Vec<(i16,i16)>>>) {let frames=Arc::new(Mutex::new(Vec::new()));(SpeakerMixer::new(Box::new(Recorder(frames.clone())),32000),frames)}
 #[test]fn unchanged_aid_without_remote(){let(mut m,f)=mixer();m.push_stereo_block(&[(-32768,32767);8]);assert_eq!(*f.lock().unwrap(),[(-32768,32767);8]);}
 #[test]fn independent_rate_volume_and_centered_linear_mix(){
  let(mut m,f)=mixer();m.push_remote_mono(&[0,3200,6400],8000,127);m.push_stereo_block(&[(100,-100);8]);
  assert_eq!(*f.lock().unwrap(),[(100,-100),(900,700),(1700,1500),(2500,2300),(3300,3100),(4100,3900),(4900,4700),(5700,5500)]);
  let(mut m,f)=mixer();m.push_remote_mono(&[1270],32000,63);m.push_stereo_i16(0,0);assert_eq!(*f.lock().unwrap(),[(630,630)]);
 }
 #[test]fn packet_boundary_rate_change_saturation_and_underrun(){
  let(mut m,f)=mixer();m.push_remote_mono(&[30000],32000,127);m.push_remote_mono(&[-30000],32000,127);m.push_stereo_i16(20000,-20000);m.set_sample_rate(48000);m.push_stereo_i16(-20000,20000);m.push_stereo_i16(-20000,20000);m.push_stereo_i16(5,-5);
  assert_eq!(*f.lock().unwrap(),[(32767,10000),(-32768,-10000),(-32768,-10000),(5,-5)]);
 }
 #[test]fn fractional_phase_survives_adjacent_packets(){
  let(mut m,f)=mixer();m.push_remote_mono(&[0],6000,127);for _ in 0..6{m.push_stereo_i16(0,0);}
  m.push_remote_mono(&[80,160],6000,127);m.push_stereo_i16(0,0);assert_eq!(f.lock().unwrap()[6],(90,90));
 }
 #[test]fn bounded_pending_lane(){let(mut m,_)=mixer();m.push_remote_mono(&vec![1;10000],6000,127);assert_eq!(m.remote.len(),6000);assert_eq!(m.dropped,4000);}
}

impl Drop for SpeakerMixer {
 fn drop(&mut self){if std::env::var("MW_TRACE_REMOTE_SPEAKER").is_ok_and(|v|v=="1"){eprintln!("REMOTE_AUDIO_MIX {{\"receivedMonoSamples\":{},\"mixedStereoFrames\":{},\"droppedMonoSamples\":{},\"queuedMonoSamples\":{},\"clippedChannels\":{}}}",self.received,self.mixed,self.dropped,self.remote.len(),self.clipped);}}
}
