//! Added color grade over the native frame (layer 2 of research/LOST-PARADISE-LOOK.md).
//! Lost Paradise-style lighting mood: no pure black, lifted shadows, warm or moonlit
//! light, neon/lantern glow, haze, vignette. Night suits Asian Town (the boss-intro HUD
//! shows a moon); F2 switches to day sun. Saturated reds (blood, red UI) pass through.
//! Bypassed with the color pack (C), so C still shows the original game exactly.
use std::sync::atomic::{AtomicBool, Ordering};

/// true = night (default; MW_TIME=day starts in day).
pub static NIGHT: AtomicBool = AtomicBool::new(true);

pub fn init_from_env() {
    if std::env::var("MW_TIME").is_ok_and(|v| v.eq_ignore_ascii_case("day")) {
        NIGHT.store(false, Ordering::Relaxed);
    }
}

fn rgb(hex: u32) -> [f32; 3] {
    [(hex >> 16 & 255) as f32 / 255., (hex >> 8 & 255) as f32 / 255., (hex & 255) as f32 / 255.]
}

/// [shadow+gain, light, haze+strength, saturation/glow/vignette/linear, sky top, sky horizon]
pub(crate) fn preset(night: bool, linear: bool) -> [f32; 24] {
    let (shadow, light, haze, strength, sat, glow, vignette, gain, sky_top, sky_low) = if night {
        // moonlit indigo shade, warm lantern light, violet neon haze, deep night sky
        (rgb(0x121426), rgb(0xf0e4d4), rgb(0x2a1838), 0.3, 1.4, 0.6, 0.4, 1.05, rgb(0x0b0d22), rgb(0x2a1638))
    } else {
        // daylight: neutral warm white light, soft brown-grey shade, blue sky with pale horizon
        (rgb(0x24201c), rgb(0xfff6e6), rgb(0xbcd3e6), 0.12, 1.35, 0.25, 0.2, 1.15, rgb(0x4f86c6), rgb(0xbcd3e6))
    };
    let mut out = [0.; 24];
    out[..3].copy_from_slice(&shadow);
    out[3] = gain;
    out[4..7].copy_from_slice(&light);
    out[8..11].copy_from_slice(&haze);
    out[11] = strength;
    out[12..16].copy_from_slice(&[sat, glow, vignette, if linear { 1. } else { 0. }]);
    out[16..19].copy_from_slice(&sky_top);
    // sky_top.a: paint the black upper frame as sky (the native frame has no sky; mw-play draws a skybox and clears it)
    out[19] = 1.;
    out[20..23].copy_from_slice(&sky_low);
    out
}

pub(crate) const SHADER: &str = r#"
struct Grade { shadow: vec4f, light: vec4f, haze: vec4f, p: vec4f, sky_top: vec4f, sky_low: vec4f }
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> g: Grade;
struct V { @builtin(position) pos: vec4f, @location(0) uv: vec2f }
@vertex fn vs(@builtin(vertex_index) i: u32) -> V {
    let uv = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    return V(vec4f(uv * vec2f(2., -2.) + vec2f(-1., 1.), 0., 1.), uv);
}
fn to_srgb(c: vec3f) -> vec3f { return select(1.055 * pow(c, vec3f(1. / 2.4)) - .055, c * 12.92, c <= vec3f(.0031308)); }
fn to_linear(c: vec3f) -> vec3f { return select(pow((c + .055) / 1.055, vec3f(2.4)), c / 12.92, c <= vec3f(.04045)); }
fn luma(c: vec3f) -> f32 { return dot(c, vec3f(.299, .587, .114)); }
fn fetch(uv: vec2f) -> vec3f {
    let c = textureSampleLevel(src, samp, uv, 0.).rgb;
    // MadWorld's output carries a fixed cream/sepia cast (neutrals measure blue at 0.886 of red);
    // white-balance it out so the presets alone set the mood.
    return clamp(select(c, to_srgb(c), g.p.w > .5) * vec3f(1., 1.006, 1.128), vec3f(0.), vec3f(1.));
}
@fragment fn fs(v: V) -> @location(0) vec4f {
    let c = fetch(v.uv);
    let l = luma(c);
    // Shadows/lights remapped to the mood colors; the frame's own chroma rides on top.
    // Per-channel lift/gain keeps each texture's own hue (no single-ramp sepia), then saturation.
    var o = (g.shadow.rgb + pow(c, vec3f(.9)) * (g.light.rgb - g.shadow.rgb)) * g.shadow.a;
    o = mix(vec3f(luma(o)), o, g.p.x);
    // Pure black in the upper frame is background, not ink (the bake lifts all ink): paint the sky.
    // Averaged over a few rows so the game's TV scanlines don't stripe the sky.
    let px = 1. / vec2f(textureDimensions(src));
    var dark = 0.;
    for (var k = -3; k <= 3; k++) { dark += luma(fetch(v.uv + vec2f(0., f32(k) * 2.) * px)); }
    let sky = select(0., (1. - smoothstep(.04, .09, dark / 7.)) * (1. - smoothstep(.35, .7, v.uv.y)), g.sky_top.a > .5);
    o = mix(o, mix(g.sky_top.rgb, g.sky_low.rgb, smoothstep(0., .6, v.uv.y)), sky);
    // Cheap glow: bright texels around this one bleed light (lanterns, neon, sun glare).
    var bloom = vec3f(0.);
    for (var k = 0; k < 12; k++) {
        let a = f32(k) * 0.5236;
        for (var r = 1; r <= 2; r++) {
            let s = fetch(v.uv + vec2f(cos(a), sin(a)) * px * f32(r) * 7.);
            bloom += s * max(luma(s) - .62, 0.);
        }
    }
    o += bloom / 24. * g.p.y * 2.;
    o += g.haze.rgb * g.haze.a * pow(1. - v.uv.y, 2.);
    let d = (v.uv - .5) * vec2f(1.3, 1.);
    o *= 1. - g.p.z * dot(d, d);
    // Blood and red UI keep their exact red.
    let red = smoothstep(.18, .4, c.r - max(c.g, c.b));
    o = clamp(mix(o, c, red), vec3f(0.), vec3f(1.));
    return vec4f(select(o, to_linear(o), g.p.w > .5), 1.);
}
"#;

pub struct Grade {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniforms: wgpu::Buffer,
    linear: bool,
}

impl Grade {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mw color grade"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mw color grade"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mw color grade"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mw color grade"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mw color grade"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mw color grade"),
            size: 96,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // sRGB and float targets sample as linear; grade in display (sRGB) space either way.
        let linear = format.is_srgb() || matches!(format, wgpu::TextureFormat::Rgba16Float);
        Self { pipeline, layout, sampler, uniforms, linear }
    }

    /// Draw `source` graded into `destination` (a ViewTarget post-process write).
    pub fn apply(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        destination: &wgpu::TextureView,
    ) {
        let data = preset(NIGHT.load(Ordering::Relaxed), self.linear);
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        queue.write_buffer(&self.uniforms, 0, &bytes);
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mw color grade"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(source) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: self.uniforms.as_entire_binding() },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mw color grade"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: destination,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.draw(0..3, 0..1);
    }
}
