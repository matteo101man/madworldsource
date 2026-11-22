//! Native GX ink shading for the characters (tools/stage_ink.py): MDB render flags pick the captured white/black TEV
//! programs, which run here as generated WGSL with the native integer combiner (port of the explorer's ink.js tevCode /
//! tevMath). Map 2 = the mesh texture, map 3 = its _AB ink texture.
//! Native, read from the original renderer during the Rin fight (MW_GX_LIGHT_DUMP + g_DrawEnv view matrix,
//! evidence/native-look-capture*.log): raster channel 1 color = clamp(ambient 127 + light0 127 x N.L) with light 0 a
//! directional light toward world (0.389, 0.819, 0.422) (ctrl 0x486); channel 1 alpha = the same with light 1, which
//! sits at view-space (0, 0, 0) - the camera - so it measures facing (ctrl 0x48A, light mask 2); signed diffuse, no
//! attenuation. The programs test that alpha against c1 = 128, which inks the silhouette; channel 0 unlit
//! material white; konst k0 = k2 = k3 = 144 and k1 = 0 (alpha 255) - a second state with k1 = 144, k3 = 64 appears at
//! times and is not tied to its trigger yet; registers c1 = 128, c2 = the mesh's +0x5F threshold.
//! GX treats clockwise triangles as front faces and culls the back ones (see `specialize`).
//! Not reproduced: maps 0/1 (the shadow-depth compare is taken as unoccluded), channel 0 vertex colors on meshes whose
//! ctrl is 0x441.
use bevy::{asset::uuid_handle, prelude::*, reflect::TypePath, render::render_resource::{AsBindGroup, ShaderType}, shader::ShaderRef};
use std::collections::HashMap;

const SHADER: Handle<Shader> = uuid_handle!("6b1f0d4e-6a8b-4c0e-9d6e-3f2a1b7c8d90");
/// World direction toward the original's character light [native, see module docs].
const WORLD_LIGHT: Vec3 = Vec3::new(0.389, 0.819, 0.422);

#[derive(serde::Deserialize, Clone)]
struct Inputs { #[serde(rename = "A")] a: String, #[serde(rename = "B")] b: String, #[serde(rename = "C")] c: String, #[serde(rename = "D")] d: String }
#[derive(serde::Deserialize, Clone)]
struct Combine { inputs: Inputs, bias: i32, subtract: bool, clamp: bool, scale: i32, destination: String }
#[derive(serde::Deserialize, Clone)]
struct Stage { color: Combine, alpha: Combine, #[serde(rename = "textureMap")] map: usize, #[serde(rename = "rasterChannel")] raster: i32,
               #[serde(rename = "rasterSwap")] raster_swap: [usize; 4], #[serde(rename = "textureSwap")] texture_swap: [usize; 4],
               #[serde(rename = "konstColor")] konst_color: u32, #[serde(rename = "konstAlpha")] konst_alpha: u32 }
#[derive(serde::Deserialize, Clone)]
struct Program { stages: Vec<Stage> }
#[derive(serde::Deserialize, Clone)]
struct InkMesh { #[serde(default)] name: String, texture: i32, program: Option<String>, threshold: Option<u32> }
#[derive(serde::Deserialize, Clone)]
struct InkModel { ab: Vec<String>, meshes: Vec<InkMesh> }
#[derive(serde::Deserialize, Resource, Clone)]
pub struct InkData { programs: HashMap<String, Program>, models: HashMap<String, InkModel> }

#[derive(Clone, Copy, ShaderType, Debug)]
pub struct InkParams { light: Vec4, camera: Vec4, threshold: f32, program: u32, konst1: i32, konst3: i32 }

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct InkMaterial {
    #[uniform(0)] params: InkParams,
    #[texture(1)] #[sampler(2)] base: Option<Handle<Image>>,
    #[texture(3)] #[sampler(4)] ab: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}
impl Material for InkMaterial {
    fn fragment_shader() -> ShaderRef { ShaderRef::Handle(SHADER) }
    fn alpha_mode(&self) -> AlphaMode { self.alpha_mode }
    /// The Wii's GX treats clockwise triangles as front faces and the characters draw with back faces culled: cloth
    /// is a front mesh plus a coincident back mesh (*_UR_*, ura), so drawing both sides z-fights the pair into a
    /// checkerboard. Clockwise fronts, back faces culled, as GX.
    fn specialize(_: &bevy::pbr::MaterialPipeline, descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
                  _: &bevy::mesh::MeshVertexBufferLayoutRef, _: bevy::pbr::MaterialPipelineKey<Self>) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.front_face = bevy::render::render_resource::FrontFace::Cw;
        descriptor.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);
        Ok(())
    }
}

/// Marks a character root whose meshes take the native ink (`model` keys ink.json).
#[derive(Component)]
pub struct Inked { pub model: &'static str, pub done: bool }

pub struct InkPlugin;
impl Plugin for InkPlugin {
    fn build(&self, app: &mut App) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/ink/ink.json");
        let Some(data) = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str::<InkData>(&s).ok()) else {
            warn!("[ink] no native ink staged: run python -I tools/stage_ink.py"); return;
        };
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        let source = wgsl(&data);
        if let Some(path) = std::env::var_os("MW_INK_DUMP_WGSL") { let _ = std::fs::write(path, &source); }
        let _ = shaders.insert(SHADER.id(), Shader::from_wgsl(source, "mw-game/play_ink.wgsl"));
        app.insert_resource(data).add_plugins(MaterialPlugin::<InkMaterial>::default()).add_systems(Update, (apply, camera_light));
    }
}

/// GX combiner input names -> WGSL (explorer ink.js colorNames / alphaNames).
fn color_in(n: &str) -> String {
    match n {
        "prev.rgb" => "pv.rgb".into(), "reg0.rgb" => "c0.rgb".into(), "reg1.rgb" => "c1.rgb".into(), "reg2.rgb" => "c2.rgb".into(),
        "texture.rgb" => "tx.rgb".into(), "raster.rgb" => "rs.rgb".into(), "konst.rgb" => "kc".into(),
        "one" => "vec3<i32>(255)".into(), "half" => "vec3<i32>(128)".into(), "zero" => "vec3<i32>(0)".into(),
        a if a.ends_with(".a") => format!("vec3<i32>({})", alpha_in(a)),
        other => panic!("untranslated TEV color input {other}"),
    }
}
fn alpha_in(n: &str) -> String {
    match n {
        "prev.a" => "pv.a", "reg0.a" => "c0.a", "reg1.a" => "c1.a", "reg2.a" => "c2.a", "texture.a" => "tx.a", "raster.a" => "rs.a",
        "konst.a" => "ka", "zero" => "0", other => panic!("untranslated TEV alpha input {other}"),
    }.into()
}
fn konst(sel: u32, alpha: bool) -> String {
    const FRAC: [i32; 8] = [255, 223, 191, 159, 128, 96, 64, 32];
    if sel < 8 { return if alpha { FRAC[sel as usize].to_string() } else { format!("vec3<i32>({})", FRAC[sel as usize]) }; }
    if !alpha && (12..16).contains(&sel) { return format!("k{}.rgb", sel - 12); }
    if sel >= 16 { let (c, r) = (['r', 'g', 'b', 'a'][((sel - 16) / 4) as usize], (sel - 16) % 4); return if alpha { format!("k{r}.{c}") } else { format!("vec3<i32>(k{r}.{c})") }; }
    if alpha { "0".into() } else { "vec3<i32>(0)".into() }
}
fn reg(d: &str) -> &'static str { match d { "prev" => "pv", "reg0" => "c0", "reg1" => "c1", "reg2" => "c2", other => panic!("TEV destination {other}") } }
fn swizzle(s: [usize; 4]) -> String { s.iter().map(|&i| ['r', 'g', 'b', 'a'][i]).collect() }

fn program_fn(name: &str, p: &Program) -> String {
    let mut b = format!("fn tev_{name}(s0: vec4<i32>, s1: vec4<i32>, s2: vec4<i32>, s3: vec4<i32>, ra0: vec4<i32>, ra1: vec4<i32>, threshold: i32) -> vec4<i32> {{\n");
    b += "  var pv = vec4<i32>(0); var c0 = vec4<i32>(0); var c1 = vec4<i32>(128); var c2 = vec4<i32>(threshold);\n";
    b += "  let k0 = vec4<i32>(144, 144, 144, 255); let k1 = vec4<i32>(params.konst1, params.konst1, params.konst1, 255); let k2 = vec4<i32>(144, 144, 144, 255); let k3 = vec4<i32>(params.konst3, params.konst3, params.konst3, 255);\n";
    for s in &p.stages {
        let (c, a) = (&s.color, &s.alpha);
        let cargs = [&c.inputs.a, &c.inputs.b, &c.inputs.c, &c.inputs.d].map(|x| color_in(x)).join(", ");
        let aargs = [&a.inputs.a, &a.inputs.b, &a.inputs.c, &a.inputs.d].map(|x| alpha_in(x)).join(", ");
        b += &format!("  {{ let tx = s{}.{}; let rs = ra{}.{}; let kc = {}; let ka = {};\n", s.map, swizzle(s.texture_swap), if s.raster == 1 { 1 } else { 0 },
                      swizzle(s.raster_swap), konst(s.konst_color, false), konst(s.konst_alpha, true));
        b += &format!("    let co = mw_color({cargs}, {}, {}, {}); let al = mw_alpha({aargs}, {}, {}, {});\n", c.bias, c.subtract as i32, c.scale, a.bias, a.subtract as i32, a.scale);
        let (cl, ch) = if c.clamp { (0, 255) } else { (-1024, 1023) };
        let (al_lo, al_hi) = if a.clamp { (0, 255) } else { (-1024, 1023) };
        b += &format!("    {}.r = clamp(co.r, {cl}, {ch}); {}.g = clamp(co.g, {cl}, {ch}); {}.b = clamp(co.b, {cl}, {ch}); {}.a = clamp(al, {al_lo}, {al_hi}); }}\n",
                      reg(&c.destination), reg(&c.destination), reg(&c.destination), reg(&a.destination));
    }
    b + "  return clamp(pv, vec4<i32>(0), vec4<i32>(255));\n}\n"
}

fn wgsl(data: &InkData) -> String {
    let mut s = String::from(r#"
// material bind group 3 (bevy_pbr MATERIAL_BIND_GROUP_INDEX); the #{} shader def is not expanded in a runtime WGSL string
struct InkParams { light: vec4<f32>, camera: vec4<f32>, threshold: f32, program: u32, konst1: i32, konst3: i32 }
@group(3) @binding(0) var<uniform> params: InkParams;
@group(3) @binding(1) var base_tex: texture_2d<f32>;
@group(3) @binding(2) var base_samp: sampler;
@group(3) @binding(3) var ab_tex: texture_2d<f32>;
@group(3) @binding(4) var ab_samp: sampler;

fn mw_mix(a: i32, b: i32, c: i32, d: i32, bias: i32, sub: i32, scale: i32) -> i32 {
  let x = (a << 8u) + (b - a) * (c + (c >> 7u));
  var v = (d << 8u) + select(x, -x, sub == 1) + select(select(0, -32768, bias == 2), 32768, bias == 1);
  if (scale < 3) { v = v << u32(scale); } else { v = v >> 1u; }
  return (v + 128) >> 8u;
}
fn mw_color(a: vec3<i32>, b: vec3<i32>, c: vec3<i32>, d: vec3<i32>, bias: i32, sub: i32, scale: i32) -> vec3<i32> {
  if (bias == 3) {
    if (scale == 3) {
      let m = select(vec3<i32>(greaterThan3(a, b)), vec3<i32>(equal3(a, b)), sub == 1);
      return d + m * c;
    }
    var av = a.r; var bv = b.r;
    if (scale >= 1) { av = av + (a.g << 8u); bv = bv + (b.g << 8u); }
    if (scale == 2) { av = av + (a.b << 16u); bv = bv + (b.b << 16u); }
    let hit = select(av > bv, av == bv, sub == 1);
    return d + select(vec3<i32>(0), c, hit);
  }
  return vec3<i32>(mw_mix(a.r, b.r, c.r, d.r, bias, sub, scale), mw_mix(a.g, b.g, c.g, d.g, bias, sub, scale), mw_mix(a.b, b.b, c.b, d.b, bias, sub, scale));
}
fn greaterThan3(a: vec3<i32>, b: vec3<i32>) -> vec3<bool> { return a > b; }
fn equal3(a: vec3<i32>, b: vec3<i32>) -> vec3<bool> { return a == b; }
fn mw_alpha(a: i32, b: i32, c: i32, d: i32, bias: i32, sub: i32, scale: i32) -> i32 {
  if (bias == 3) { return d + select(0, c, select(a > b, a == b, sub == 1)); }
  return mw_mix(a, b, c, d, bias, sub, scale);
}
fn mw_encode(x: vec3<f32>) -> vec3<f32> { return select(1.055 * pow(max(x, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055, x * 12.92, x <= vec3<f32>(0.0031308)); }
fn mw_decode(x: vec3<f32>) -> vec3<f32> { return select(pow((max(x, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4)), x / 12.92, x <= vec3<f32>(0.04045)); }
"#);
    let mut names: Vec<&String> = data.programs.keys().collect();
    names.sort();
    for n in &names { s += &program_fn(n, &data.programs[*n]); }
    s += r#"
struct In { @builtin(position) position: vec4<f32>, @location(0) world_position: vec4<f32>, @location(1) world_normal: vec3<f32>, @location(2) uv: vec2<f32> }
@fragment
fn fragment(in: In) -> @location(0) vec4<f32> {
  let src = textureSample(base_tex, base_samp, in.uv);
  let shadow = textureSample(ab_tex, ab_samp, in.uv);
  // channel 1: ambient 127 + light 127 x signed N.L, clamped (GX lighting, 8-bit color registers)
  // channel 1: ambient 127 + light 127 x signed N.L, clamped (GX lighting, 8-bit color registers)
  let illum = clamp(127.0 / 255.0 + 127.0 / 255.0 * dot(normalize(in.world_normal), normalize(params.light.xyz)), 0.0, 1.0);
  let s2 = vec4<i32>(vec4<f32>(mw_encode(src.rgb) * 255.0 + 0.5, src.a * 255.0 + 0.5));
  var s3 = vec4<i32>(shadow * 255.0 + 0.5);
  let threshold = i32(params.threshold);
  if (threshold == 0) { s3.a = 0; }
  // alpha: light 1 at the camera (point light, no attenuation): facing term
  let facing = clamp(127.0 / 255.0 + 127.0 / 255.0 * dot(normalize(in.world_normal), normalize(params.camera.xyz - in.world_position.xyz)), 0.0, 1.0);
  let ra1 = vec4<i32>(vec3<i32>(i32(illum * 255.0 + 0.5)), i32(facing * 255.0 + 0.5));
  var o = vec4<i32>(0);
"#;
    for (i, n) in names.iter().enumerate() {
        s += &format!("  if (params.program == {i}u) {{ o = tev_{n}(vec4<i32>(0), vec4<i32>(255), s2, s3, vec4<i32>(255), ra1, threshold); }}\n");
    }
    s += "  if (src.a < 0.5) { discard; }\n";
    // inspection: MW_INK_DEBUG=src|ab|abalpha|illum shows that input instead of the TEV result
    s += match std::env::var("MW_INK_DEBUG").as_deref() {
        Ok("src") => "  return vec4<f32>(src.rgb, 1.0);\n}\n",
        Ok("ab") => "  return vec4<f32>(shadow.rgb, 1.0);\n}\n",
        Ok("abalpha") => "  return vec4<f32>(vec3<f32>(shadow.a), 1.0);\n}\n",
        Ok("illum") => "  return vec4<f32>(vec3<f32>(illum), 1.0);\n}\n",
        _ => "  return vec4<f32>(mw_decode(vec3<f32>(o.rgb) / 255.0), 1.0);\n}\n",
    };
    s
}

fn program_index(data: &InkData, name: &str) -> u32 {
    let mut names: Vec<&String> = data.programs.keys().collect();
    names.sort();
    names.iter().position(|n| n.as_str() == name).unwrap_or(0) as u32
}

/// Swap each inked mesh's material (GLB role_0_mesh_<i> = MDB mesh i) for the native ink once the scene is spawned.
fn apply(mut commands: Commands, data: Res<InkData>, assets: Res<AssetServer>, mut roots: Query<(Entity, &mut Inked)>, children: Query<&Children>,
         names: Query<&Name>, parents: Query<&ChildOf>, std: Query<&MeshMaterial3d<StandardMaterial>>, std_assets: Res<Assets<StandardMaterial>>,
         mut inks: ResMut<Assets<InkMaterial>>) {
    for (root, mut inked) in &mut roots {
        if inked.done { continue; }
        let Some(model) = data.models.get(inked.model) else { inked.done = true; continue; };
        let mut swapped = 0;
        for e in children.iter_descendants(root) {
            let Ok(mat) = std.get(e) else { continue; };
            let Some(index) = parents.get(e).ok().and_then(|p| names.get(p.parent()).ok()).and_then(|n| n.as_str().strip_prefix("role_0_mesh_")).and_then(|i| i.parse::<usize>().ok()) else { continue; };
            let Some(mesh) = model.meshes.get(index) else { continue; };
            // The x-ray skeleton (*_HN_*, drawn by the electrocution effect) and the cut cross-sections (*_DA, drawn when
            // severed) share the body's space; always drawn they z-fight it into a checkerboard. Hidden in normal play.
            if mesh.name.contains("_HN_") || mesh.name.ends_with("_DA") { commands.entity(e).insert(Visibility::Hidden); continue; }
            let (Some(program), Some(ab)) = (mesh.program.as_ref(), usize::try_from(mesh.texture).ok().and_then(|t| model.ab.get(t))) else { continue; };
            let Some(source) = std_assets.get(&mat.0) else { continue; };
            let ink = inks.add(InkMaterial {
                params: InkParams { light: WORLD_LIGHT.normalize().extend(0.), camera: Vec4::ZERO, threshold: mesh.threshold.unwrap_or(128) as f32, program: program_index(&data, program), konst1: 0, konst3: 144 },
                base: source.base_color_texture.clone(), ab: Some(assets.load(ab.clone())), alpha_mode: AlphaMode::Mask(0.5),
            });
            commands.entity(e).remove::<MeshMaterial3d<StandardMaterial>>().insert(MeshMaterial3d(ink));
            swapped += 1;
        }
        if swapped > 0 { inked.done = true; info!("[ink] {} meshes of {} use the native ink", swapped, inked.model); }
    }
}

/// Light 1 of the original's character channel sits at the camera: keep its world position current.
fn camera_light(cam: Query<&GlobalTransform, (With<Camera3d>, Without<bevy_mod_xr::camera::XrCamera>)>, mut inks: ResMut<Assets<InkMaterial>>) {
    let Ok(c) = cam.single() else { return; };
    let at = c.translation().extend(1.);
    for (_, m) in inks.iter_mut() { if m.params.camera != at { m.params.camera = at; } }
}
