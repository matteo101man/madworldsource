//! Opt-in wall-clock counters (MW_PERF_LOG=1: stderr; MW_PERF_LOG=<file>: append): guest frames presented, core time
//! blocked in the GX sink, and AID PCM produced vs consumed. Observation only.
use mw_native_platform::native_hardware::{
    flipper::gx::{draw::TextureFormat, GraphicsProcessor},
    host::{DrawData, DrawSegment, DrawState, DrawVertex, GxAction, RawVertexStream, RenderSink, RenderStats},
    mmio::RamViewMut,
};
use std::{
    sync::atomic::{AtomicU64, Ordering::Relaxed},
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct Counters {
    pub presents: AtomicU64,
    pub sink_ns: AtomicU64,
    pub blocking_ns: AtomicU64,
    pub produced: AtomicU64,
    pub consumed: AtomicU64,
    pub source_used: AtomicU64,
    pub underruns: AtomicU64,
    pub dropped: AtomicU64,
    pub queued: AtomicU64,
    pub host_frames: AtomicU64,
}
pub static COUNTERS: Counters = Counters {
    presents: AtomicU64::new(0),
    sink_ns: AtomicU64::new(0),
    blocking_ns: AtomicU64::new(0),
    produced: AtomicU64::new(0),
    consumed: AtomicU64::new(0),
    source_used: AtomicU64::new(0),
    underruns: AtomicU64::new(0),
    dropped: AtomicU64::new(0),
    queued: AtomicU64::new(0),
    host_frames: AtomicU64::new(0),
};
/// Wall time between guest presents, for frame-time percentiles.
static PRESENTS: std::sync::Mutex<(Option<Instant>, Vec<f32>)> = std::sync::Mutex::new((None, Vec::new()));
fn record_present() {
    let now = Instant::now();
    let mut presents = PRESENTS.lock().unwrap();
    if let Some(last) = presents.0.replace(now) {
        presents.1.push((now - last).as_secs_f32() * 1000.0);
    }
}
pub fn enabled() -> bool {
    std::env::var_os("MW_PERF_LOG").is_some()
}
/// Writes one JSON line per second.
pub fn spawn_reporter() {
    let target = std::env::var("MW_PERF_LOG").unwrap_or_default();
    let mut file = (target != "1")
        .then(|| std::fs::OpenOptions::new().create(true).append(true).open(&target).ok())
        .flatten();
    use mw_native_platform::native_hardware::host::perf;
    perf::ENABLED.store(true, Relaxed);
    std::thread::spawn(move || {
        let tick_ns = perf::ticks_per_ns();
        let c = &COUNTERS;
        let take = |a: &AtomicU64| a.swap(0, Relaxed);
        let mut last = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let dt = last.elapsed().as_secs_f64();
            last = Instant::now();
            let mut frames = std::mem::take(&mut PRESENTS.lock().unwrap().1);
            frames.sort_by(f32::total_cmp);
            let pct = |p: f64| frames.get(((frames.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0.0);
            let frame_list = frames.iter().map(|f| format!("{f:.2}")).collect::<Vec<_>>().join(",");
            let core = format!(
                "\"frameP50Ms\":{:.2},\"frameP99Ms\":{:.2},\"frameMaxMs\":{:.2},\"gxCommandMsPerSec\":{:.1},\"vertexPrepMsPerSec\":{:.1},\"verticesPerSec\":{:.0},\"dspMsPerSec\":{:.1},\"rawMBPerSec\":{:.1},\"drawsPerSec\":{:.0},\"xfMBPerSec\":{:.1},\"arrayMBPerSec\":{:.1},\"frameMs\":[{frame_list}]",
                pct(0.5),
                pct(0.99),
                frames.last().copied().unwrap_or(0.0),
                take(&perf::GX_COMMAND_TICKS) as f64 / tick_ns / 1e6 / dt,
                take(&perf::VERTEX_TICKS) as f64 / tick_ns / 1e6 / dt,
                take(&perf::VERTICES) as f64 / dt,
                take(&perf::DSP_TICKS) as f64 / tick_ns / 1e6 / dt,
                take(&perf::RAW_BYTES) as f64 / 1e6 / dt,
                take(&perf::DRAWS) as f64 / dt,
                take(&perf::XF_SNAPSHOT_BYTES) as f64 / 1e6 / dt,
                take(&perf::ARRAY_BYTES) as f64 / 1e6 / dt,
            );
            let line = format!(
                "MW_PERF {{\"guestFps\":{:.2},\"hostFps\":{:.2},\"gxSinkMsPerSec\":{:.1},\"gxBlockingMsPerSec\":{:.1},\"aidProducedHz\":{:.0},\"sourceConsumedHz\":{:.0},\"deviceOutputHz\":{:.0},\"underrunFrames\":{},\"droppedFrames\":{},\"queuedFrames\":{},{core}}}",
                take(&c.presents) as f64 / dt,
                take(&c.host_frames) as f64 / dt,
                take(&c.sink_ns) as f64 / 1e6 / dt,
                take(&c.blocking_ns) as f64 / 1e6 / dt,
                take(&c.produced) as f64 / dt,
                take(&c.source_used) as f64 / dt,
                take(&c.consumed) as f64 / dt,
                take(&c.underruns),
                take(&c.dropped),
                c.queued.load(Relaxed),
            );
            match &mut file {
                Some(file) => {
                    use std::io::Write;
                    let _ = writeln!(file, "{line}");
                }
                None => eprintln!("{line}"),
            }
        }
    });
}
pub struct PerfSink(pub Box<dyn RenderSink>);
fn timed<T>(slot: &AtomicU64, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let value = f();
    slot.fetch_add(start.elapsed().as_nanos() as u64, Relaxed);
    value
}
impl RenderSink for PerfSink {
    fn exec(&mut self, action: GxAction) {
        if matches!(action, GxAction::PresentXfb { .. } | GxAction::PresentRawXfb { .. }) {
            COUNTERS.presents.fetch_add(1, Relaxed);
            record_present();
        }
        timed(&COUNTERS.sink_ns, || self.0.exec(action))
    }
    fn peek_efb_depth(&mut self, x: u32, y: u32) -> u32 {
        timed(&COUNTERS.blocking_ns, || self.0.peek_efb_depth(x, y))
    }
    fn capture_efb_depth(&mut self) -> Option<Vec<u32>> {
        timed(&COUNTERS.blocking_ns, || self.0.capture_efb_depth())
    }
    fn exec_draw(&mut self, segment: DrawSegment, state: Option<DrawState>) {
        timed(&COUNTERS.sink_ns, || self.0.exec_draw(segment, state))
    }
    fn exec_draw_batch(&mut self, segments: &[DrawSegment], state: Option<DrawState>) {
        timed(&COUNTERS.sink_ns, || self.0.exec_draw_batch(segments, state))
    }
    fn vertex_scratch(&mut self) -> &mut Vec<DrawVertex> {
        self.0.vertex_scratch()
    }
    fn raw_vertex_stream(&mut self) -> Option<&mut RawVertexStream> {
        self.0.raw_vertex_stream()
    }
    fn inspect_draws(&mut self, gx: &GraphicsProcessor, segments: &[DrawSegment]) {
        self.0.inspect_draws(gx, segments)
    }
    fn has_pending_efb_texture(&self, addr: u32, width: u32, height: u32, fmt: TextureFormat) -> bool {
        self.0.has_pending_efb_texture(addr, width, height, fmt)
    }
    fn flush_efb_copies(&mut self, ram: &mut RamViewMut<'_>) {
        timed(&COUNTERS.blocking_ns, || self.0.flush_efb_copies(ram))
    }
    fn reset_efb(&mut self) {
        self.0.reset_efb()
    }
    fn take_draw_data(&mut self) -> Box<DrawData> {
        self.0.take_draw_data()
    }
    fn render_stats(&self) -> RenderStats {
        self.0.render_stats()
    }
}
