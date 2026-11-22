//! Headless GX evidence sink. Retains source shader inputs, not rendered pixels.
use gecko::host::{DrawState,DrawVertex,GxAction,RenderSink};
use std::{cell::Cell,fs::File,io::Write,sync::{Arc,atomic::{AtomicU64,Ordering}}};
thread_local! {static SCENE:Cell<u32>=const{Cell::new(0)};}
pub fn mark_scene(scene:u32){SCENE.set(scene);}
pub struct Sink {
    scene_filter:Option<u32>,
    min_tev:u8,
    perspective_only:bool,
    is_perspective:bool,
    captured:u64,
    presents:u64,
    scratch:Vec<DrawVertex>,
    file:Option<File>,
    draws:Arc<AtomicU64>,
    state:Option<DrawState>,
    projection:[u32;16],
}
impl Sink {
    pub fn new(file:Option<File>)->(Self,Arc<AtomicU64>){
        let scene_filter=std::env::var("MW_GX_AUDIT_SCENE").ok().map(|value|{
            if let Some(hex)=value.strip_prefix("0x"){u32::from_str_radix(hex,16).expect("GX audit scene")}
            else{value.parse().expect("GX audit scene")}
        });
        Self::with_filter(file,scene_filter)
    }
    pub fn with_filter(file:Option<File>,scene_filter:Option<u32>)->(Self,Arc<AtomicU64>){
        let draws=Arc::new(AtomicU64::new(0));
        (Self{scene_filter,min_tev:std::env::var("MW_GX_AUDIT_MIN_TEV").map(|v|v.parse().expect("GX audit minimum TEV")).unwrap_or(0),perspective_only:std::env::var_os("MW_GX_AUDIT_PERSPECTIVE").is_some(),is_perspective:false,captured:0,presents:0,scratch:Vec::new(),file,draws:draws.clone(),state:None,projection:[0;16]},draws)
    }
}
impl Sink {
    fn present(&mut self,width:u32,height:u32,source:&str,parts:usize){
        if self.presents>=8||self.scene_filter.is_some_and(|filter|filter!=SCENE.get()){return;}
        self.presents+=1;
        if let Some(file)=self.file.as_mut(){
            writeln!(file,"{{\"kind\":\"native-vi-present\",\"scene\":{},\"width\":{width},\"height\":{height},\"source\":\"{source}\",\"parts\":{parts}}}",SCENE.get()).expect("VI audit write");
            file.flush().expect("VI audit flush");
        }
    }
}
impl RenderSink for Sink {
    fn vertex_scratch(&mut self)->&mut Vec<DrawVertex>{&mut self.scratch}
    fn exec(&mut self,action:GxAction){
        match action {
            GxAction::SetProjection{matrix,is_perspective}=>{
                self.is_perspective=is_perspective;
                self.projection=std::array::from_fn(|i|matrix[i/4][i%4].to_bits());
            }
            GxAction::Draw(draw)=>{
                if let Some(state)=draw.state {self.state=Some(state);}
                let n=self.draws.fetch_add(1,Ordering::Relaxed)+1;
                let scene=SCENE.get();
                let selected=self.scene_filter.map_or(n<=64,|filter|filter==scene&&self.captured<64);
                let selected=selected&&(!self.perspective_only||self.is_perspective)&&self.state.as_ref().is_some_and(|state|state.num_tev_stages>=self.min_tev);
                if selected {
                    self.captured+=1;
                    let vertices:u32=draw.segments.iter().map(|s|s.vertex_count).sum();
                    let all_finite=self.scratch.iter().all(|v|v.color0.iter().chain(v.color1.iter()).chain(v.normal.iter()).chain(v.texcoords.iter().flatten()).all(|x|x.is_finite()));
                    let finite=self.scratch.iter().all(|v|v.position.iter().chain(v.pos_view.iter()).all(|x|x.is_finite()));
                    if let Some(file)=self.file.as_mut(){
                        writeln!(file,"{{\"kind\":\"native-gx-draw\",\"draw\":{n},\"scene\":{scene},\"segments\":{},\"vertices\":{vertices},\"decodedVertices\":{},\"finitePositions\":{finite},\"finiteVertexAttributes\":{all_finite},\"isPerspective\":{},\"tevColorRegBits\":{:?},\"tevKonstBits\":{:?},\"channelColorCtrl\":{:?},\"channelAlphaCtrl\":{:?},\"lightBits\":{:?},\"projectionBits\":{:?},\"tevColorEnv\":{:?},\"tevAlphaEnv\":{:?},\"tevOrders\":{:?},\"tevKsel\":{:?},\"numTevStages\":{},\"materialBits\":{:?},\"ambientBits\":{:?},\"lightPositionBits\":{:?}}}",
                            draw.segments.len(),self.scratch.len(),self.is_perspective,
                            self.state.as_ref().map(|s|s.tev_color_regs.map(|c|c.map(f32::to_bits))).unwrap_or([[0;4];4]),
                            self.state.as_ref().map(|s|s.tev_konst_colors.map(|c|c.map(f32::to_bits))).unwrap_or([[0;4];16]),
                            self.state.as_ref().map(|s|s.color_ctrl.map(|c|c.raw())).unwrap_or([0;2]),
                            self.state.as_ref().map(|s|s.alpha_ctrl.map(|c|c.raw())).unwrap_or([0;2]),
                            self.state.as_ref().map(|s|s.lights.map(|l|[l.color.map(f32::to_bits),l.cosatt.map(f32::to_bits),l.distatt.map(f32::to_bits),l.position.map(f32::to_bits),l.direction.map(f32::to_bits)])).unwrap_or([[[0;4];5];8]),
                            self.projection,
                            self.state.as_ref().map(|s|s.tev_color_env).unwrap_or([0;16]),self.state.as_ref().map(|s|s.tev_alpha_env).unwrap_or([0;16]),self.state.as_ref().map(|s|s.tev_orders).unwrap_or([0;16]),self.state.as_ref().map(|s|s.tev_ksel).unwrap_or([0;8]),self.state.as_ref().map(|s|s.num_tev_stages).unwrap_or(0),
                            self.state.as_ref().map(|s|s.material_color.map(|c|c.map(f32::to_bits))).unwrap_or([[0;4];2]),self.state.as_ref().map(|s|s.ambient_color.map(|c|c.map(f32::to_bits))).unwrap_or([[0;4];2]),self.state.as_ref().map(|s|s.lights.map(|l|l.position.map(f32::to_bits))).unwrap_or([[0;4];8])
                        ).expect("GX audit write");
                        file.flush().expect("GX audit flush");
                    }
                }
            }
            GxAction::PresentXfb{width,height,parts}=>{
                self.present(width,height,"gx-copies",parts.len());
            }
            GxAction::PresentRawXfb{width,height,pixels:_}=>{
                self.present(width,height,"cpu-xfb",0);
            }
            _=>{}
        }
        self.scratch.clear();
    }
}
