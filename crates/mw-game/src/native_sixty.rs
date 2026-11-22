//! Opt-in 60 fps presentation (research/SIXTY-FPS.md option b). MW_60FPS=probe only measures; MW_60FPS=1 also
//! sends the camera estimate to the renderer, which replays each guest frame at the camera midpoint
//! (backend-wgpu interp.rs). MW_60FPS_LOG=<file> appends the per-second probe lines, else stderr.
//! Fingerprints every guest draw (position array base, VCD, primitive, vertex count) and its default position
//! matrix and matches each guest frame against the previous one. Static world geometry keeps its array and
//! model matrix, so the consensus of the matched matrix deltas is the camera motion.
use mw_native_platform::native_hardware::{
    flipper::gx::{
        constants::{ARRAY_BASE_REG, VCD_HI_REG, VCD_LO_REG, XF_MATRIX_INDEX_A, XF_POS_MTX_STRIDE},
        draw::TextureFormat,
        GraphicsProcessor,
    },
    host::{DrawData, DrawSegment, DrawState, DrawVertex, GxAction, RawVertexStream, RenderSink, RenderStats},
    mmio::RamViewMut,
};
use bevy::math::{Affine3A, Mat3A, Quat, Vec3A};
use std::{
    collections::{HashMap, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
    time::Instant,
};
pub fn enabled() -> bool {
    std::env::var("MW_60FPS").is_ok_and(|v| v == "probe" || v == "1")
}
type Projection = [[f32; 4]; 4];
struct Draw {
    sig: u64,
    /// Same key without the position array base: separates moved vertex buffers from re-batched draws.
    shape: u64,
    verts: u32,
    /// Vertices carry their own matrix index (skinned); the default matrix is not the whole transform.
    skinned: bool,
    mtx: [f32; 12],
    /// Perspective projection in effect, if any.
    projection: Option<Projection>,
}
#[derive(Default)]
struct Second {
    frames: u64,
    draws: u64,
    matched: u64,
    in_order: u64,
    verts: u64,
    matched_verts: u64,
    shape_matched: u64,
    efb_copies: u64,
    rigid_moved: u64,
    rigid_matched: u64,
    translation: Vec<f32>,
    /// Camera consensus: frames with an estimate, cuts, inlier vertex share and per-frame delta size.
    cam_frames: u64,
    cam_cuts: u64,
    cam_inlier_share: Vec<f32>,
    cam_rot_deg: Vec<f32>,
    cam_trans: Vec<f32>,
}
/// Row-major 3x4 GX position matrix as an affine transform.
fn affine(m: &[f32; 12]) -> Affine3A {
    Affine3A {
        matrix3: Mat3A::from_cols(Vec3A::new(m[0], m[4], m[8]), Vec3A::new(m[1], m[5], m[9]), Vec3A::new(m[2], m[6], m[10])),
        translation: Vec3A::new(m[3], m[7], m[11]),
    }
}
fn close(a: &Affine3A, b: &Affine3A, scale: f32) -> bool {
    let m = (a.matrix3 - b.matrix3).abs();
    let r = m.x_axis.max(m.y_axis).max(m.z_axis).max_element();
    r < 2e-3 && (a.translation - b.translation).length() < 2e-3 * scale.max(1.0)
}
/// Camera motion between two guest frames, in view space: `delta * view_prev = view_cur`.
pub struct CameraDelta {
    pub delta: Affine3A,
    pub inlier_share: f32,
    /// Index into `pairs` of the winning candidate.
    pub index: usize,
}
/// Static world geometry keeps its vertex array and model matrix, so for those draws
/// `MV_cur * MV_prev^-1` is the same camera delta. Takes the candidate (from the 32
/// largest pairs) that agrees with the most vertices. `pairs` is (prev, cur, vertices).
pub fn camera_consensus(pairs: &[([f32; 12], [f32; 12], u32)]) -> Option<CameraDelta> {
    let deltas: Vec<(Affine3A, u32, usize)> = pairs
        .iter()
        .enumerate()
        .filter_map(|(i, (p, c, v))| {
            let p = affine(p);
            (p.matrix3.determinant().abs() > 1e-8).then(|| (affine(c) * p.inverse(), *v, i))
        })
        .filter(|(d, _, _)| d.translation.is_finite() && d.matrix3.is_finite())
        .collect();
    let total: u64 = deltas.iter().map(|(_, v, _)| *v as u64).sum();
    if total == 0 {
        return None;
    }
    let mut order: Vec<usize> = (0..deltas.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(deltas[i].1));
    let scale = deltas.iter().map(|(d, _, _)| d.translation.length()).fold(0.0, f32::max);
    let mut best = (0u64, 0usize);
    for &i in order.iter().take(32) {
        let w: u64 = deltas.iter().filter(|(d, _, _)| close(d, &deltas[i].0, scale)).map(|(_, v, _)| *v as u64).sum();
        if w > best.0 {
            best = (w, i);
        }
    }
    Some(CameraDelta { delta: deltas[best.1].0, inlier_share: best.0 as f32 / total as f32, index: deltas[best.1].2 })
}
pub struct ProbeSink {
    inner: Box<dyn RenderSink>,
    current: Vec<Draw>,
    previous: Vec<Draw>,
    /// Last perspective projection set.
    projection: Option<Projection>,
    /// Whether the current projection is the perspective one in `projection`.
    perspective: bool,
    send: bool,
    second: Second,
    started: Instant,
    out: Option<std::fs::File>,
}
impl ProbeSink {
    pub fn new(inner: Box<dyn RenderSink>) -> Self {
        let out = std::env::var("MW_60FPS_LOG")
            .ok()
            .and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok());
        Self { inner, current: Vec::new(), previous: Vec::new(), projection: None, perspective: false, send: std::env::var("MW_60FPS").is_ok_and(|v| v == "1"), second: Second::default(), started: Instant::now(), out }
    }
    /// Returns the camera delta and its projection for the renderer, or `None` at a cut.
    fn end_frame(&mut self) -> Option<(Affine3A, Projection)> {
        let mut queues: HashMap<u64, VecDeque<usize>> = HashMap::new();
        let mut shapes: HashMap<u64, u64> = HashMap::new();
        for (i, d) in self.previous.iter().enumerate() {
            queues.entry(d.sig).or_default().push_back(i);
            *shapes.entry(d.shape).or_default() += 1;
        }
        let s = &mut self.second;
        let mut last = None;
        let mut pairs = Vec::new();
        let mut pair_projection = Vec::new();
        let mut out = None;
        for d in &self.current {
            s.draws += 1;
            s.verts += d.verts as u64;
            if let Some(n) = shapes.get_mut(&d.shape).filter(|n| **n > 0) {
                *n -= 1;
                s.shape_matched += 1;
            }
            let Some(j) = queues.get_mut(&d.sig).and_then(VecDeque::pop_front) else { continue };
            s.matched += 1;
            s.matched_verts += d.verts as u64;
            if last.is_none_or(|l| j > l) {
                s.in_order += 1;
            }
            last = Some(j);
            if !d.skinned {
                let p = &self.previous[j];
                if d.projection.is_some() && d.projection == p.projection {
                    pairs.push((p.mtx, d.mtx, d.verts));
                    pair_projection.push(d.projection.unwrap());
                }
                s.rigid_matched += 1;
                if p.mtx != d.mtx {
                    s.rigid_moved += 1;
                }
                let t = |m: &[f32; 12]| [m[3], m[7], m[11]];
                let (a, b) = (t(&p.mtx), t(&d.mtx));
                let dist = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                if dist.is_finite() {
                    s.translation.push(dist);
                }
            }
        }
        if let Some(cam) = camera_consensus(&pairs) {
            s.cam_frames += 1;
            let (_, rot, trans) = cam.delta.to_scale_rotation_translation();
            // The winning pair already shares its projection with the previous frame (zoom or cut
            // otherwise); a weak consensus or a large turn is a cut.
            let cut = cam.inlier_share < 0.3
                || rot.angle_between(Quat::IDENTITY).to_degrees() > 20.0;
            s.cam_cuts += cut as u64;
            if !cut {
                out = Some((cam.delta, pair_projection[cam.index]));
            }
            s.cam_inlier_share.push(cam.inlier_share);
            s.cam_rot_deg.push(rot.angle_between(Quat::IDENTITY).to_degrees());
            s.cam_trans.push(trans.length());
        }
        s.frames += 1;
        self.previous = std::mem::take(&mut self.current);
        if self.started.elapsed().as_secs_f64() >= 1.0 {
            self.report();
        }
        out
    }
    fn report(&mut self) {
        let dt = self.started.elapsed().as_secs_f64();
        self.started = Instant::now();
        let mut s = std::mem::take(&mut self.second);
        for v in [&mut s.translation, &mut s.cam_inlier_share, &mut s.cam_rot_deg, &mut s.cam_trans] {
            v.sort_by(f32::total_cmp);
        }
        let at = |v: &[f32], p: f64| v.get(((v.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0.0);
        let pct = |p: f64| at(&s.translation, p);
        let ratio = |a: u64, b: u64| if b == 0 { 0.0 } else { a as f64 / b as f64 };
        let line = format!(
            "MW_60FPS_PROBE {{\"guestFps\":{:.2},\"drawsPerFrame\":{:.1},\"matchedDraws\":{:.4},\"shapeMatchedDraws\":{:.4},\"inOrderDraws\":{:.4},\"matchedVerts\":{:.4},\"efbCopiesPerFrame\":{:.2},\"rigidMatchedPerFrame\":{:.1},\"rigidMovedShare\":{:.4},\"rigidTranslationP50\":{:.4},\"rigidTranslationP90\":{:.4},\"rigidTranslationMax\":{:.4},\"camFrames\":{},\"camCuts\":{},\"camInlierP10\":{:.3},\"camInlierP50\":{:.3},\"camRotDegP50\":{:.3},\"camRotDegMax\":{:.3},\"camTransP50\":{:.4},\"camTransMax\":{:.4}}}",
            s.frames as f64 / dt,
            ratio(s.draws, s.frames),
            ratio(s.matched, s.draws),
            ratio(s.shape_matched, s.draws),
            ratio(s.in_order, s.draws),
            ratio(s.matched_verts, s.verts),
            ratio(s.efb_copies, s.frames),
            ratio(s.rigid_matched, s.frames),
            ratio(s.rigid_moved, s.rigid_matched),
            pct(0.5),
            pct(0.9),
            s.translation.last().copied().unwrap_or(0.0),
            s.cam_frames,
            s.cam_cuts,
            at(&s.cam_inlier_share, 0.1),
            at(&s.cam_inlier_share, 0.5),
            at(&s.cam_rot_deg, 0.5),
            s.cam_rot_deg.last().copied().unwrap_or(0.0),
            at(&s.cam_trans, 0.5),
            s.cam_trans.last().copied().unwrap_or(0.0),
        );
        match &mut self.out {
            Some(file) => {
                use std::io::Write;
                let _ = writeln!(file, "{line}");
            }
            None => eprintln!("{line}"),
        }
    }
}
impl RenderSink for ProbeSink {
    fn exec(&mut self, action: GxAction) {
        match &action {
            GxAction::PresentXfb { .. } | GxAction::PresentRawXfb { .. } => {
                let camera = self.end_frame();
                if self.send {
                    let (delta, projection) = match camera {
                        Some((d, p)) => (Some(bevy::math::Mat4::from(d).to_cols_array_2d()), p),
                        None => (None, [[0.0; 4]; 4]),
                    };
                    self.inner.exec(GxAction::SetInterpolation { delta, projection });
                }
            }
            GxAction::CopyEfbToTexture { .. } => self.second.efb_copies += 1,
            GxAction::SetProjection { matrix, is_perspective } => {
                self.perspective = *is_perspective;
                if *is_perspective {
                    self.projection = Some(*matrix);
                }
            }
            _ => {}
        }
        self.inner.exec(action)
    }
    fn peek_efb_depth(&mut self, x: u32, y: u32) -> u32 {
        self.inner.peek_efb_depth(x, y)
    }
    fn capture_efb_depth(&mut self) -> Option<Vec<u32>> {
        self.inner.capture_efb_depth()
    }
    fn exec_draw(&mut self, segment: DrawSegment, state: Option<DrawState>) {
        self.inner.exec_draw(segment, state)
    }
    fn exec_draw_batch(&mut self, segments: &[DrawSegment], state: Option<DrawState>) {
        self.inner.exec_draw_batch(segments, state)
    }
    fn vertex_scratch(&mut self) -> &mut Vec<DrawVertex> {
        self.inner.vertex_scratch()
    }
    fn raw_vertex_stream(&mut self) -> Option<&mut RawVertexStream> {
        self.inner.raw_vertex_stream()
    }
    fn inspect_draws(&mut self, gx: &GraphicsProcessor, segments: &[DrawSegment]) {
        let vcd_lo = gx.cp_regs[VCD_LO_REG];
        let mut h = DefaultHasher::new();
        (vcd_lo, gx.cp_regs[VCD_HI_REG]).hash(&mut h);
        let mut verts = 0;
        for s in segments {
            std::mem::discriminant(&s.primitive).hash(&mut h);
            s.vertex_count.hash(&mut h);
            verts += s.vertex_count;
        }
        let shape = h.finish();
        gx.cp_regs[ARRAY_BASE_REG].hash(&mut h);
        let base = (gx.xf_mem[XF_MATRIX_INDEX_A] & 0x3f) as usize * XF_POS_MTX_STRIDE;
        let mtx = std::array::from_fn(|i| f32::from_bits(gx.xf_mem[base + i]));
        let projection = if self.perspective { self.projection } else { None };
        self.current.push(Draw { sig: h.finish(), shape, verts, skinned: vcd_lo & 1 != 0, mtx, projection });
        self.inner.inspect_draws(gx, segments)
    }
    fn has_pending_efb_texture(&self, addr: u32, width: u32, height: u32, fmt: TextureFormat) -> bool {
        self.inner.has_pending_efb_texture(addr, width, height, fmt)
    }
    fn flush_efb_copies(&mut self, ram: &mut RamViewMut<'_>) {
        self.inner.flush_efb_copies(ram)
    }
    fn reset_efb(&mut self) {
        self.inner.reset_efb()
    }
    fn take_draw_data(&mut self) -> Box<DrawData> {
        self.inner.take_draw_data()
    }
    fn render_stats(&self) -> RenderStats {
        self.inner.render_stats()
    }
}
