//! Bounded native PC stereo capture: AID plus the optional decoded remote lane.
use gecko::audio::{AidBlock,AudioSink};
use std::{fs::File,io::{Write,Seek,SeekFrom},path::PathBuf};

pub struct WavSink {
    file:File, path:PathBuf, rate:u32, frames:u32, nonzero:u64,
    peak:u32, pending:Vec<u8>, dropped:u64,
}
impl WavSink {
    pub fn create(path:PathBuf,rate:u32)->Self {
        let file=File::create(&path).expect("native PCM capture output");
        let mut sink=Self{file,path,rate,frames:0,nonzero:0,peak:0,pending:Vec::with_capacity(16384),dropped:0};
        sink.flush();sink
    }
    fn frame(&mut self,left:i16,right:i16) {
        if self.frames>=48_000*300 {self.dropped+=1;return;}
        self.frames+=1;
        for value in [left,right] {
            self.nonzero+=u64::from(value!=0);
            self.peak=self.peak.max(i32::from(value).unsigned_abs());
            self.pending.extend_from_slice(&value.to_le_bytes());
        }
        if self.pending.len()>=16384 {self.flush();}
    }
}
impl AudioSink for WavSink {
    fn set_sample_rate(&mut self,rate:u32) {
        assert!(matches!(rate,32000|48000),"unsupported native AID rate");
        assert!(self.frames==0||self.rate==rate,"native AID rate changed after PCM; capture requires a new segment");
        self.rate=rate;
    }
    fn push_stereo_i16(&mut self,left:i16,right:i16) {self.frame(left,right);}
    fn push_stereo_block(&mut self,frames:&AidBlock) {for &(left,right) in frames {self.frame(left,right);}}
    fn flush(&mut self) {
        self.file.seek(SeekFrom::End(0)).expect("PCM seek");
        self.file.write_all(&self.pending).expect("PCM data");self.pending.clear();
        let data=self.frames*4;
        let mut header=Vec::with_capacity(44);
        header.extend_from_slice(b"RIFF");header.extend_from_slice(&(36+data).to_le_bytes());
        header.extend_from_slice(b"WAVEfmt ");header.extend_from_slice(&16u32.to_le_bytes());
        header.extend_from_slice(&1u16.to_le_bytes());header.extend_from_slice(&2u16.to_le_bytes());
        header.extend_from_slice(&self.rate.to_le_bytes());header.extend_from_slice(&(self.rate*4).to_le_bytes());
        header.extend_from_slice(&4u16.to_le_bytes());header.extend_from_slice(&16u16.to_le_bytes());
        header.extend_from_slice(b"data");header.extend_from_slice(&data.to_le_bytes());
        self.file.seek(SeekFrom::Start(0)).expect("PCM header seek");
        self.file.write_all(&header).expect("PCM header");self.file.flush().expect("PCM flush");
        std::fs::write(self.path.with_extension("audio.json"),format!(
            "{{\"sampleRate\":{},\"stereoFrames\":{},\"nonzeroSamples\":{},\"absolutePeak\":{},\"framesDroppedAtCaptureLimit\":{},\"authority\":\"Native PC stereo: original AID plus optional decoded remote mono; routing/timing equivalence not established\"}}\n",
            self.rate,self.frames,self.nonzero,self.peak,self.dropped)).expect("PCM capture stats");
    }
}
impl Drop for WavSink {fn drop(&mut self){self.flush();}}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wav_preserves_stereo_signed_pcm_and_header() {
        let path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../evidence/synthetic-aid.wav");
        let mut sink=WavSink::create(path.clone(),32000);
        sink.set_sample_rate(48000);
        sink.push_stereo_block(&[(-32768,32767);8]);sink.flush();
        let bytes=std::fs::read(path).unwrap();
        assert_eq!(bytes.len(),44+32);assert_eq!(&bytes[..4],b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()),48000);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()),32);
        assert_eq!(&bytes[44..48],&[0,128,255,127]);
        assert_eq!((sink.frames,sink.nonzero,sink.peak),(8,16,32768));
    }
}
