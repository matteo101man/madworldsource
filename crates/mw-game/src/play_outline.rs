//! Added, not original: a screen-space ink outline for the modern looks (play_look.rs). The Wii only inks the characters'
//! silhouettes (play_ink.rs); this draws comic-style ink on every edge of the frame: where view depth jumps (silhouettes,
//! the stage's overlaps) or the surface normal turns sharply (creases), read from the depth and normal prepasses the
//! modern looks already keep for SSAO. A depth jump between two surfaces facing the same way (a grating wire over the
//! deck below) is not inked, and far surfaces fade out, so the floor does not turn into a black hatch.
//! It runs before the color grade, so the grade tints the ink like the rest of the frame.
//! A FullscreenMaterial cannot read the prepasses, so this is its render pass with two more bindings.
use bevy::{
    asset::uuid_handle,
    core_pipeline::{prepass::ViewPrepassTextures, tonemapping::tonemapping, Core3d, Core3dSystems, FullscreenShader},
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_resource::{
            binding_types::{sampler, texture_2d, texture_depth_2d, uniform_buffer},
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            FragmentState, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
            Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, TextureFormat, TextureSampleType,
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{ExtractedView, ViewTarget},
        RenderApp, RenderStartup,
    },
};

const SHADER: Handle<Shader> = uuid_handle!("4f0c2b7e-93d1-4a6e-8c55-1e7b9d3a6f20");

/// Ink color (rgb, a = strength), then (thickness in target pixels, depth edge ratio, normal edge, fade distance).
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType)]
#[extract_app(RenderApp)]
pub struct Outline { pub ink: Vec4, pub params: Vec4 }
impl Default for Outline {
    fn default() -> Self {
        // inspection: MW_OUTLINE_DEBUG=1 shows the detected edges only
        let strength = if std::env::var_os("MW_OUTLINE_DEBUG").is_some() { 2. } else { 1. };
        Self { ink: Vec4::new(0.01, 0.008, 0.02, strength), params: Vec4::new(4.0, 0.06, 0.45, 30.0) }
    }
}

const WGSL: &str = r#"
struct Outline { ink: vec4f, params: vec4f }
@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var depth: texture_depth_2d;
@group(0) @binding(3) var normals: texture_2d<f32>;
@group(0) @binding(4) var<uniform> o: Outline;
struct In { @builtin(position) pos: vec4f, @location(0) uv: vec2f }

// Bevy's reverse-Z infinite projection: depth = near / view distance (near 0.1)
fn dist(p: vec2i) -> f32 { return 0.1 / max(textureLoad(depth, p, 0), 1e-6); }
fn nrm(p: vec2i) -> vec3f { return normalize(textureLoad(normals, p, 0).xyz * 2.0 - 1.0); }

@fragment fn fragment(v: In) -> @location(0) vec4f {
  let c = textureSample(screen, samp, v.uv);
  let size = vec2i(textureDimensions(depth));
  let p = vec2i(v.pos.xy);
  let t = i32(o.params.x);
  let d0 = dist(p);
  let n0 = nrm(p);
  var edge = 0.0;
  for (var k = 0; k < 4; k++) {
    let off = select(select(vec2i(0, -t), vec2i(0, t), k == 1), select(vec2i(-t, 0), vec2i(t, 0), k == 3), k >= 2);
    let q = clamp(p + off, vec2i(0), size - 1);
    let d1 = dist(q);
    let turn = 1.0 - dot(n0, nrm(q));
    // depth: only the nearer side of a jump draws, so the line sits on the foreground silhouette; a jump between two
    // surfaces facing the same way (a grating wire over the deck below) is not an edge, so the floor stays clean
    edge = max(edge, smoothstep(o.params.y, o.params.y * 2.0, (d1 - d0) / d0) * smoothstep(0.04, 0.15, turn));
    edge = max(edge, smoothstep(o.params.z, o.params.z + 0.25, turn) * step(abs(d1 - d0) / d0, 0.5));
  }
  let fade = 1.0 - smoothstep(o.params.w * 0.5, o.params.w, d0);
  if (o.ink.a > 1.5) { return vec4f(vec3f(edge * fade), 1.0); }   // inspection: edges only
  return vec4f(mix(c.rgb, o.ink.rgb, edge * fade * o.ink.a), c.a);
}
"#;

pub struct OutlinePlugin;
impl Plugin for OutlinePlugin {
    fn build(&self, app: &mut App) {
        let _ = app.world_mut().resource_mut::<Assets<Shader>>().insert(SHADER.id(), Shader::from_wgsl(WGSL, "mw-game/play_outline.wgsl"));
        app.add_plugins((ExtractComponentPlugin::<Outline>::default(), UniformComponentPlugin::<Outline>::default()));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return; };
        render_app.add_systems(RenderStartup, init_pipeline).add_systems(bevy::render::Render, prepare_pipelines.in_set(bevy::render::RenderSystems::Prepare))
            .add_systems(Core3d, outline_pass.in_set(Core3dSystems::PostProcess).before(tonemapping)
                .before(bevy::core_pipeline::fullscreen_material::fullscreen_material_system::<super::grade::Grade>));
    }
}

#[derive(Resource)]
struct OutlinePipeline { layout: BindGroupLayoutDescriptor, sampler: Sampler, shader: Handle<Shader>, vertex: bevy::render::render_resource::VertexState }

#[derive(Component)]
struct OutlinePipelineId(CachedRenderPipelineId);

fn init_pipeline(mut commands: Commands, device: Res<RenderDevice>, fullscreen: Res<FullscreenShader>) {
    let layout = BindGroupLayoutDescriptor::new("mw_outline_layout", &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (
        texture_2d(TextureSampleType::Float { filterable: true }),
        sampler(SamplerBindingType::Filtering),
        texture_depth_2d(),
        texture_2d(TextureSampleType::Float { filterable: false }),
        uniform_buffer::<Outline>(true),
    )));
    commands.insert_resource(OutlinePipeline { layout, sampler: device.create_sampler(&SamplerDescriptor::default()), shader: SHADER, vertex: fullscreen.to_vertex_state() });
}

/// One pipeline per target format, queued for every view that carries an [`Outline`].
fn prepare_pipelines(mut commands: Commands, cache: Res<PipelineCache>, pipeline: Option<Res<OutlinePipeline>>,
                     views: Query<(Entity, &ExtractedView, Option<&Outline>)>, mut ids: Local<std::collections::HashMap<TextureFormat, CachedRenderPipelineId>>) {
    let Some(pipeline) = pipeline else { return; };
    for (e, view, outline) in &views {
        if outline.is_none() { commands.entity(e).remove::<OutlinePipelineId>(); continue; }
        let id = *ids.entry(view.target_format).or_insert_with(|| cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("mw_outline_pipeline".into()),
            layout: vec![pipeline.layout.clone()],
            vertex: pipeline.vertex.clone(),
            fragment: Some(FragmentState { shader: pipeline.shader.clone(),
                targets: vec![Some(ColorTargetState { format: view.target_format, blend: None, write_mask: ColorWrites::ALL })], ..default() }),
            ..default()
        }));
        commands.entity(e).insert(OutlinePipelineId(id));
    }
}

fn outline_pass(view: ViewQuery<(&ViewTarget, &ViewPrepassTextures, &DynamicUniformIndex<Outline>, &OutlinePipelineId)>,
                cache: Res<PipelineCache>, pipeline: Res<OutlinePipeline>, uniforms: Res<ComponentUniforms<Outline>>,
                device: Res<RenderDevice>, mut ctx: RenderContext) {
    let (target, prepass, index, id) = view.into_inner();
    let (Some(render), Some(binding), Some(depth), Some(normal)) =
        (cache.get_render_pipeline(id.0), uniforms.uniforms().binding(), prepass.depth_only_view(), prepass.normal_view()) else { return; };
    let post = target.post_process_write();
    let group = device.create_bind_group("mw_outline_group", &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((post.source, &pipeline.sampler, depth, normal, binding)));
    let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
        label: Some("mw_outline_pass"),
        color_attachments: &[Some(RenderPassColorAttachment { view: post.destination, depth_slice: None, resolve_target: None, ops: Operations::default() })],
        depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
    });
    pass.set_pipeline(render);
    pass.set_bind_group(0, &group, &[index.index()]);
    pass.draw(0..3, 0..1);
}
