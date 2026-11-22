//! Original mixed AID PCM to Bevy's audio device; no replacement mix or voices.
//!
//! The device sees one constant-rate stereo stream ([`OUTPUT_RATE`]). AID frames
//! (32 or 48 kHz, tagged per frame) are resampled here, so rodio never has to
//! follow per-sample rate changes. A small jitter buffer is primed before
//! playback, and a bounded (±0.5%) step trim keeps the queue near its target
//! despite device-clock vs host-clock drift; that trim is far below audible
//! pitch error and never changes guest timing.
use bevy::{
    audio::{ChannelCount, Decodable, SampleRate, Source},
    prelude::*,
};
use mw_native_platform::native_hardware::audio::{AidBlock, AudioSink};
use std::{
    collections::VecDeque,
    sync::atomic::Ordering::Relaxed,
    sync::{Arc, Mutex},
    time::Duration,
};
pub const OUTPUT_RATE: u32 = 48000;
/// Jitter buffer target and hard latency bound, in seconds of source audio.
const TARGET_SECONDS: f64 = 0.06;
const LIMIT_SECONDS: f64 = 0.25;
/// Output frames rendered per queue lock.
const CHUNK: usize = 256;
const MAX_TRIM: f64 = 0.005;
fn perf() -> &'static crate::native_perf::Counters {
    &crate::native_perf::COUNTERS
}
#[derive(Default)]
struct Queue {
    frames: VecDeque<(i16, i16, u32)>,
    rate: u32,
    stopped: bool,
    dropped: u64,
    underruns: u64,
}
#[derive(Clone, Default)]
pub struct AudioBridge(Arc<Mutex<Queue>>);
impl AudioBridge {
    pub fn finish(&self) {
        self.0.lock().unwrap().stopped = true;
    }
    fn push(&self, frames: &[(i16, i16)]) {
        let mut q = self.0.lock().unwrap();
        let rate = q.rate.max(32000);
        let limit = (rate as f64 * LIMIT_SECONDS) as usize;
        for &(l, r) in frames {
            while q.frames.len() >= limit {
                q.frames.pop_front();
                q.dropped += 1;
                perf().dropped.fetch_add(1, Relaxed);
            }
            q.frames.push_back((l, r, rate));
        }
        perf().produced.fetch_add(frames.len() as u64, Relaxed);
        perf().queued.store(q.frames.len() as u64, Relaxed);
    }
}
impl AudioSink for AudioBridge {
    fn set_sample_rate(&mut self, rate: u32) {
        assert!(matches!(rate, 32000 | 48000));
        self.0.lock().unwrap().rate = rate;
    }
    fn push_stereo_i16(&mut self, l: i16, r: i16) {
        self.push(&[(l, r)]);
    }
    fn push_stereo_block(&mut self, frames: &AidBlock) {
        self.push(frames);
    }
}
#[derive(Asset, TypePath, Clone)]
pub struct NativePcm(pub AudioBridge);
pub struct Decoder {
    audio: AudioBridge,
    /// Interpolation endpoints: `a` at phase 0, `b` at phase 1, with `b`'s rate.
    a: (f32, f32),
    b: (f32, f32),
    rate: u32,
    phase: f64,
    primed: bool,
    /// Smoothed queue fill, in source frames.
    fill: f64,
    out: Vec<f32>,
    cursor: usize,
    ended: bool,
}
impl Decodable for NativePcm {
    type Decoder = Decoder;
    fn decoder(&self) -> Decoder {
        Decoder {
            audio: self.0.clone(),
            a: (0., 0.),
            b: (0., 0.),
            rate: 32000,
            phase: 0.,
            primed: false,
            fill: 0.,
            out: Vec::with_capacity(CHUNK * 2),
            cursor: 0,
            ended: false,
        }
    }
}
fn sample(x: i16) -> f32 {
    x as f32 / 32768.
}
impl Decoder {
    /// Render the next chunk of interleaved output; false at end of stream.
    fn render(&mut self) -> bool {
        self.out.clear();
        self.cursor = 0;
        let mut q = self.audio.0.lock().unwrap();
        let target = self.rate as f64 * TARGET_SECONDS;
        if !self.primed {
            if q.stopped && q.frames.is_empty() {
                return false;
            }
            if (q.frames.len() as f64) < target && !q.stopped {
                // Still buffering after start or an underrun: emit silence.
                self.out.resize(CHUNK * 2, 0.);
                perf().underruns.fetch_add(CHUNK as u64, Relaxed);
                perf().consumed.fetch_add(CHUNK as u64, Relaxed);
                return true;
            }
            self.primed = true;
            self.fill = q.frames.len() as f64;
        }
        self.fill += (q.frames.len() as f64 - self.fill) * 0.02;
        let trim = ((self.fill - target) / target * 0.01).clamp(-MAX_TRIM, MAX_TRIM);
        let mut used = 0u64;
        for _ in 0..CHUNK {
            let step = self.rate as f64 / OUTPUT_RATE as f64 * (1. + trim);
            self.phase += step;
            while self.phase >= 1. {
                let Some((l, r, rate)) = q.frames.pop_front() else {
                    break;
                };
                used += 1;
                self.phase -= 1.;
                self.a = self.b;
                self.b = (sample(l), sample(r));
                self.rate = rate;
            }
            if self.phase >= 1. {
                // Source ran dry: hold position and re-prime before resuming.
                self.phase = 1.;
                self.primed = false;
                q.underruns += 1;
                if q.stopped && self.out.is_empty() {
                    return false;
                }
                let missing = CHUNK - self.out.len() / 2;
                self.out.resize(CHUNK * 2, 0.);
                perf().underruns.fetch_add(missing as u64, Relaxed);
                break;
            }
            let t = self.phase as f32;
            self.out.push(self.a.0 + (self.b.0 - self.a.0) * t);
            self.out.push(self.a.1 + (self.b.1 - self.a.1) * t);
        }
        perf().consumed.fetch_add((self.out.len() / 2) as u64, Relaxed);
        perf().source_used.fetch_add(used, Relaxed);
        perf().queued.store(q.frames.len() as u64, Relaxed);
        true
    }
}
impl Iterator for Decoder {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.cursor == self.out.len() {
            if self.ended || !self.render() {
                self.ended = true;
                return None;
            }
        }
        let value = self.out[self.cursor];
        self.cursor += 1;
        Some(value)
    }
}
impl Source for Decoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(OUTPUT_RATE).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn drain(stream: &mut Decoder) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        while let (Some(l), Some(r)) = (stream.next(), stream.next()) {
            out.push((l, r));
        }
        out
    }
    #[test]
    fn resamples_aid_to_fixed_output_rate_at_original_pitch() {
        let mut bridge = AudioBridge::default();
        bridge.set_sample_rate(32000);
        let tone = |i: usize| ((i as f64 * 1000. / 32000. * std::f64::consts::TAU).sin() * 16000.) as i16;
        // Producer and consumer interleave as on the device; queue stays near target.
        let mut stream = NativePcm(bridge.clone()).decoder();
        let mut out = Vec::new();
        for block in 0..4000 {
            let frames: Vec<_> = (block * 8..block * 8 + 8).map(|i| (tone(i), -tone(i))).collect();
            bridge.push(&frames);
            for _ in 0..12 {
                out.push((stream.next().unwrap(), stream.next().unwrap()));
            }
        }
        assert_eq!(stream.sample_rate().get(), OUTPUT_RATE);
        assert_eq!(stream.channels().get(), 2);
        assert_eq!(stream.current_span_len(), None);
        let audible: Vec<_> = out.iter().skip_while(|f| f.0 == 0.).copied().collect();
        // Rising zero crossings of the left channel per output second = tone pitch.
        let rising = audible.windows(2).filter(|w| w[0].0 < 0. && w[1].0 >= 0.).count();
        let hz = rising as f64 / (audible.len() as f64 / OUTPUT_RATE as f64);
        assert!((hz - 1000.).abs() < 5., "pitch {hz}");
        assert!(audible.iter().all(|f| (f.0 + f.1).abs() < 1e-3), "stereo channels kept separate");
        let peak = audible.iter().map(|f| f.0.abs()).fold(0., f32::max);
        assert!((peak - 16000. / 32768.).abs() < 0.01, "amplitude {peak}");
    }
    #[test]
    fn rate_switch_and_end_of_stream() {
        let mut bridge = AudioBridge::default();
        bridge.set_sample_rate(48000);
        bridge.push(&vec![(16384, -16384); 4800]);
        bridge.finish();
        let out = drain(&mut NativePcm(bridge).decoder());
        // 48 kHz source maps 1:1 (priming is skipped once the producer stopped).
        assert!((out.len() as i64 - 4800).unsigned_abs() <= CHUNK as u64, "{}", out.len());
        assert!(out[10..4700].iter().all(|&f| f == (0.5, -0.5)));
    }
    #[test]
    fn primes_then_silence_on_underrun_and_bounded_latency() {
        let mut bridge = AudioBridge::default();
        bridge.set_sample_rate(32000);
        let mut stream = NativePcm(bridge.clone()).decoder();
        assert_eq!(stream.next(), Some(0.));
        bridge.push(&[(1000, 1000); 100]); // below the jitter target
        assert!((0..CHUNK * 4).all(|_| stream.next() == Some(0.)));
        bridge.push(&vec![(1000, 1000); 48000]);
        let q = bridge.0.lock().unwrap();
        assert_eq!(q.frames.len(), (32000. * LIMIT_SECONDS) as usize);
        assert_eq!(q.dropped, 48100 - q.frames.len() as u64);
    }
    #[test]
    #[ignore = "Requires the owned native AID capture and a default audio output device"]
    fn owned_pcm_plays_through_headless_bevy_audio_device() {
        use bevy::audio::{AddAudioSource, AudioPlugin, AudioSinkPlayback, Volume};
        use std::{path::PathBuf, time::Instant};
        let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path=root.join("evidence/native-boss-intro-aid.wav");
        let bytes=std::fs::read(&path).expect("Owned original AID PCM capture");
        assert_eq!(&bytes[..4],b"RIFF");assert_eq!(&bytes[8..16],b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()),1);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()),2);
        assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()),16);
        assert_eq!(&bytes[36..40],b"data");
        let rate=u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        assert!(matches!(rate,32000|48000));
        let frames:Vec<_>=bytes[44..].chunks_exact(4).map(|b|(i16::from_le_bytes([b[0],b[1]]),i16::from_le_bytes([b[2],b[3]]))).collect();
        let start=frames.iter().position(|&(l,r)|(l as i32).abs()>256||(r as i32).abs()>256).expect("Nonzero original PCM");
        let segment=&frames[start..(start+rate as usize).min(frames.len())];
        assert!(segment.len()>=rate as usize/2);
        let mut bridge=AudioBridge::default();bridge.set_sample_rate(rate);bridge.push(segment);bridge.finish();
        let initial=bridge.0.lock().unwrap().frames.len();
        let mut app=App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin{file_path:root.join("assets").to_string_lossy().into_owned(),..default()})
            .add_plugins(AudioPlugin::default())
            .add_audio_source::<NativePcm>();
        app.finish();app.cleanup();
        let handle=app.world_mut().resource_mut::<Assets<NativePcm>>().add(NativePcm(bridge.clone()));
        app.world_mut().spawn((AudioPlayer(handle),PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.03))));
        let started=Instant::now();let mut sink_seen=false;let mut position=0f64;
        while started.elapsed()<Duration::from_millis(750) {
            app.update();
            let mut query=app.world_mut().query::<&bevy::audio::AudioSink>();
            for sink in query.iter(app.world()) {sink_seen=true;position=position.max(sink.position().as_secs_f64());}
            std::thread::sleep(Duration::from_millis(5));
        }
        let remaining=bridge.0.lock().unwrap().frames.len();
        let mut query=app.world_mut().query::<&bevy::audio::AudioSink>();
        for sink in query.iter(app.world()) {sink.stop();}
        drop(app);
        assert!(sink_seen,"No default audio-device sink was created");
        assert!(position>0.05,"Audio device playback position did not advance");
        assert!(initial.saturating_sub(remaining)>rate as usize/10,"Native PCM source was not consumed");
        println!("HEADLESS_AUDIO_DEVICE {}",serde_json::json!({"source":path,"sampleRate":rate,"sourceFrameOffset":start,"initialFrames":initial,"remainingFrames":remaining,"playbackPositionSeconds":position,"sinkCreated":sink_seen,"testVolume":0.03,"scope":"Headless Bevy AudioPlugin/default-device sink consumes original captured AID PCM through NativePcm. No window, desktop capture, human audibility, original mix/timing or integrated battle audio equivalence claim."}));
    }
    #[test]
    fn lowering_rate_keeps_queue_bounded() {
        let mut bridge = AudioBridge::default();
        bridge.set_sample_rate(48000);
        bridge.push(&vec![(1, 2); 96000]);
        bridge.set_sample_rate(32000);
        bridge.push_stereo_i16(3, 4);
        let q = bridge.0.lock().unwrap();
        assert_eq!(q.frames.len(), 8000);
        assert_eq!(q.frames.back(), Some(&(3, 4, 32000)));
    }
}
