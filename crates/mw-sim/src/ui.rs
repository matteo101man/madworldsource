//! Original UI vector interpolation: 800CB358, 80249D48 (Hermite), 80249E24 (tangents).
//! This evaluates an explicitly supplied frame; channel restart/loop counters belong to the widget runtime.
use mw_data::ui::UiVectorTrack;

pub fn vector_at(track:&UiVectorTrack,frame:f32)->Result<[f32;3], &'static str> {
    // 800C80C0 calls800CAF3C for bit0x20000. That helper maintains a
    // separate random offset on axes0x4000/0x8000; it does not change the
    // base curve's frame parameter. WC's0x20063 has neither random axis.
    if track.flags&0xc000!=0 {return Err("random UI position requires native jitter state");}
    if !frame.is_finite() {return Err("nonfinite UI frame");}
    let Some(first)=track.keys.first() else{return Ok(track.points.first().copied().unwrap_or([0.;3]));};
    if frame<=first.frame as f32{return Ok(point_at(track,first.value));}
    let Some(segment)=track.keys.windows(2).position(|k|frame<k[1].frame as f32) else {
        return Ok(point_at(track,track.keys.last().unwrap().value));
    };
    let a=track.keys[segment];let b=track.keys[segment+1];
    let t=(frame-a.frame as f32)/(b.frame-a.frame) as f32;
    let value=if track.flags&0x20!=0 {
        let values:Vec<_>=track.keys.iter().map(|k|[k.frame as f32,k.value,0.]).collect();
        let (m0,m1)=tangents(&values,segment);
        hermite(values[segment],m0,values[segment+1],m1,t)[1]
    } else if track.flags&0x10!=0 {a.value+(b.value-a.value)*t} else {a.value};
    Ok(point_at(track,value))
}
fn point_at(track:&UiVectorTrack,value:f32)->[f32;3] {
    if track.points.is_empty() {return [0.;3];}
    let last=track.points.len()-1;
    let value=value.clamp(0.,last as f32);let i=value.floor() as usize;
    if i==last{return track.points[last];}
    let t=value-i as f32;
    if track.flags&0x80!=0 {
        let (a,b)=tangents(&track.points,i);hermite(track.points[i],a,track.points[i+1],b,t)
    } else if track.flags&0x40!=0 {
        [0,1,2].map(|j|track.points[i][j]+(track.points[i+1][j]-track.points[i][j])*t)
    } else {track.points[i]}
}
fn tangents(points:&[[f32;3]],i:usize)->([f32;3],[f32;3]) {
    // The native endpoints use half the adjacent segment, not a duplicated endpoint derivative.
    let a=if i==0 {[0,1,2].map(|j|(points[1][j]-points[0][j])*0.5)}
        else {[0,1,2].map(|j|(points[i+1][j]-points[i-1][j])*0.5)};
    let b=if i+2==points.len() {[0,1,2].map(|j|(points[i+1][j]-points[i][j])*0.5)}
        else {[0,1,2].map(|j|(points[i+2][j]-points[i][j])*0.5)};
    (a,b)
}
fn hermite(a:[f32;3],m0:[f32;3],b:[f32;3],m1:[f32;3],t:f32)->[f32;3] {
    let t2=t*t;let t3=t*t2;
    [0,1,2].map(|i|a[i]*(1.+2.*t3-3.*t2)+m0[i]*(t+t3-2.*t2)+b[i]*(-2.*t3+3.*t2)+m1[i]*(t3-t2))
}
#[cfg(test)]mod tests {
    use super::*;
    use mw_data::ui::{UiTimeline,UiVectorKey};
    #[test]fn source_subscreen_slide_uses_original_two_stage_interpolation() {
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/ui/subscr000_us/animation.idt");
        let bytes=std::fs::read(path).unwrap();let timeline=UiTimeline::parse(&bytes).unwrap();
        let track=timeline.program(0).unwrap().widgets[65].position_track(&bytes).unwrap().unwrap();
        assert_eq!(track.flags,0x57);assert_eq!(track.points,vec![[0.,0.,0.],[0.,300.,0.]]);
        assert_eq!(vector_at(&track,0.).unwrap(),[0.;3]);assert_eq!(vector_at(&track,45.).unwrap(),[0.,150.,0.]);
        assert_eq!(vector_at(&track,90.).unwrap(),[0.,300.,0.]);assert_eq!(vector_at(&track,100.).unwrap(),[0.,300.,0.]);
    }
    #[test]fn random_axis_offsets_are_explicitly_untranslated() {
        let track=UiVectorTrack{flags:0x24053,points:vec![[0.;3],[1.;3]],keys:vec![]};
        assert!(vector_at(&track,1.).is_err());
    }
    #[test]fn hermite_endpoint_tangents_differ_from_linear_and_seek_is_repeatable() {
        let track=UiVectorTrack{flags:0x92,points:vec![[0.;3],[16.,0.,0.]],keys:vec![UiVectorKey{value:0.,frame:0},UiVectorKey{value:1.,frame:16}]};
        assert_eq!(vector_at(&track,4.).unwrap(),[3.25,0.,0.]);assert_eq!(vector_at(&track,12.).unwrap(),[12.75,0.,0.]);
        assert_eq!(vector_at(&track,4.).unwrap(),[3.25,0.,0.]);
    }
}

/// Scalar UI envelopes (800CCA94 / 800CD850); widget clock owns looping and restart state.
pub fn scalar_at(track:&mw_data::ui::UiScalarTrack,frame:f32)->f32 {
    let Some(first)=track.keys.first()else{return 1.;};
    let frame=loop_frame(frame,first.frame as f32,track.keys.last().unwrap().frame as f32,track.flags);
    if frame<=first.frame as f32{return first.value;}
    let Some(i)=track.keys.windows(2).position(|k|frame<k[1].frame as f32)else{return track.keys.last().unwrap().value;};
    let a=track.keys[i];let b=track.keys[i+1];let t=(frame-a.frame as f32)/(b.frame-a.frame) as f32;
    if track.flags&0x20!=0 {
        let values:Vec<_>=track.keys.iter().map(|k|[k.frame as f32,k.value,0.]).collect();
        let (m0,m1)=tangents(&values,i);hermite(values[i],m0,values[i+1],m1,t)[1]
    } else if track.flags&0x10!=0 {a.value+(b.value-a.value)*t} else {a.value}
}
fn loop_frame(frame:f32,first:f32,last:f32,flags:u32)->f32 {
    if flags&4!=0 && last>first && frame>=last {first+(frame-first).rem_euclid(last-first)}else{frame}
}

use mw_data::ui::{UiProgram,UiAppearance};
#[derive(Clone,Debug,Default)]pub struct UiClocks {
    pub position:f32,pub scale:f32,pub rotation:[f32;3],pub rgb:f32,pub alpha:f32,
}
impl UiClocks {
    pub fn at(frame:f32)->Self{Self{position:frame,scale:frame,rotation:[frame;3],rgb:frame,alpha:frame}}
    fn advance(&mut self,frames:f32,stopped:u8){if stopped&4==0{self.position+=frames;}if stopped&8==0{self.scale+=frames;}if stopped&16==0{for frame in &mut self.rotation{*frame+=frames;}}if stopped&32==0{self.rgb+=frames;}if stopped&64==0{self.alpha+=frames;}}
}
#[derive(Clone,Debug)]
pub struct WidgetState {
    pub forced_hidden:bool,pub inverted:bool, pub position_source:usize,pub external_position:Option<mw_data::ui::UiWidget>,pub keep_position_axis:u8,pub scale_speed:f32,pub scale_axes:Option<u32>,pub stopped:u8, pub clocks:UiClocks,
    pub position:Option<[f32;2]>,pub rotation:Option<f32>,
    pub uv_frame:Option<u16>,pub rgb_binding:Option<usize>,pub alpha_binding:Option<usize>, pub color:Option<[f32;4]>,pub gauge:Option<f32>,
}
#[derive(Clone,Debug)]
pub struct UiScene {pub program:UiProgram,pub states:Vec<WidgetState>,pub origin:[f32;2],bytes:Vec<u8>,appearance:Vec<UiAppearance>}
#[derive(Clone,Debug)]
pub struct UiDraw {
    pub index:usize,pub id:u32,pub kind:u32,pub visible:bool,pub inverted:bool,
    pub uv_crop:[f32;4],pub repeat:[f32;2],pub blend_mode:u32,pub tile_kind:u32,
    pub position:[f32;2],pub child_scale:[f32;2],pub scale:[f32;2],pub rotation:f32,pub color:[f32;4],pub uv:[u16;2],
}
impl UiScene {
    pub fn new(program:&UiProgram,bytes:&[u8],origin:[f32;2])->Result<Self,String> {
        let appearance=program.widgets.iter().map(|w|w.appearance(bytes)).collect::<Result<Vec<_>,_>>()?;
        let states=(0..program.widgets.len()).map(|i|WidgetState{forced_hidden:false,inverted:false,position_source:i,external_position:None,keep_position_axis:0,scale_speed:1.,scale_axes:None,stopped:0,clocks:UiClocks::default(),position:None,rotation:None,uv_frame:None,rgb_binding:None,alpha_binding:None,color:None,gauge:None}).collect();
        Ok(Self{program:program.clone(),states,origin,bytes:bytes.to_vec(),appearance})
    }
    pub fn bytes(&self)->&[u8]{&self.bytes}
    pub fn index(&self,id:u32)->Option<usize>{self.program.widgets.iter().position(|w|w.id==id)}
    /// The last keyframe of a widget's own tracks (position, scale, rotation, rgb, alpha): how long its animation plays
    /// (the native "+0xC0 bit 3 still playing" test, e.g. 80240B88 on the clear board's widget 1).
    pub fn track_end(&self,id:u32)->f32{
        let Some(w)=self.index(id).map(|i|&self.program.widgets[i]) else{return 0.};
        let mut end=0u32;
        if let Ok(Some(t))=w.position_track(&self.bytes){for k in &t.keys{end=end.max(k.frame);}}
        if let Ok(a)=w.appearance(&self.bytes){
            for t in a.scale.iter().chain(a.rotation_tracks.iter().flatten()).chain(a.alpha.iter()){for k in &t.keys{end=end.max(k.frame);}}
            if let Some((_,keys))=&a.rgb{for k in keys{end=end.max(k.1.max(0.) as u32);}}
        }
        end as f32
    }
    pub fn hide_tree(&mut self,id:u32,hidden:bool) {
        if let Some(i)=self.index(id){self.hide_index(i,hidden);}
    }
    fn hide_index(&mut self,i:usize,hidden:bool){
        self.states[i].forced_hidden=hidden;
        let mut child=self.program.widgets[i].first_child;
        while let Some(c)=child {self.hide_index(c,hidden);child=self.program.widgets[c].next;}
    }
    /// 800D07E4: recursively select source RGB complement for a confirmed menu entry.
    pub fn select_tree(&mut self,id:u32,selected:bool){if let Some(i)=self.index(id){self.select_index(i,selected);}}
    fn select_index(&mut self,i:usize,selected:bool){
        self.states[i].inverted=selected;let mut child=self.program.widgets[i].first_child;
        while let Some(c)=child{self.select_index(c,selected);child=self.program.widgets[c].next;}
    }
    /// 800CEB9C channel 2: replace the source of both static and animated position.
    pub fn bind_position(&mut self,id:u32,source:u32){
        if let (Some(i),Some(s))=(self.index(id),self.index(source)){self.states[i].position_source=s;self.states[i].external_position=None;self.states[i].keep_position_axis=0;self.states[i].clocks.position=0.;}
    }
    /// 800CEB9C also binds positions across screens sharing one IDT resource.
    pub fn bind_position_from(&mut self,id:u32,other:&UiScene,source:u32){
        assert!(self.bytes==other.bytes,"External position binding needs the same IDT");
        if let(Some(i),Some(s))=(self.index(id),other.index(source)){self.states[i].external_position=Some(other.program.widgets[s].clone());self.states[i].keep_position_axis=0;self.states[i].clocks.position=0.;}
    }
    /// 802249B4 copies a local RGB preset, bypassing the animation envelope.
    pub fn color_preset(&mut self,id:u32,preset:usize){
        if let Some(i)=self.index(id){if let Some(offset)=self.program.widgets[i].channels[5]{
            assert!(preset<4);let at=self.program.widgets[i].record_offset+offset+preset*16;
            let rgb:[f32;3]=[0,1,2].map(|k|f32::from_be_bytes(self.bytes[at+k*4..at+k*4+4].try_into().unwrap()));
            assert!(rgb.iter().all(|v|v.is_finite()));self.states[i].color=Some([rgb[0],rgb[1],rgb[2],self.program.widgets[i].color[3]]);
        }}
    }
    /// Channel-2 context1 preserves target static X; context2 preserves target static Y (800C80C0).
    pub fn bind_position_axis(&mut self,id:u32,source:u32,context:u8){
        self.bind_position(id,source);if let Some(i)=self.index(id){self.states[i].keep_position_axis=context;}
    }
    /// 800CEB9C channels3/5 replace an envelope, then800CF768 restarts it.
    pub fn bind_scale(&mut self,id:u32,source:u32){if let(Some(i),Some(s))=(self.index(id),self.index(source)){self.appearance[i].scale=self.program.widgets[s].appearance(&self.bytes).expect("validated source").scale;self.states[i].scale_axes=Some(self.program.widgets[s].flags);self.restart_scale(id);}}
    pub fn bind_rgb(&mut self,id:u32,source:u32){if let(Some(i),Some(s))=(self.index(id),self.index(source)){let source=self.program.widgets[s].appearance(&self.bytes).expect("validated source");self.appearance[i].rgb=source.rgb;self.appearance[i].rgb_flags=source.rgb_flags;self.states[i].rgb_binding=Some(s);self.states[i].clocks.rgb=0.;self.states[i].stopped&=!32;}}
    pub fn bind_alpha(&mut self,id:u32,source:u32){if let(Some(i),Some(s))=(self.index(id),self.index(source)){let source=self.program.widgets[s].appearance(&self.bytes).expect("validated source");self.appearance[i].alpha=source.alpha;self.appearance[i].alpha_flags=source.alpha_flags;self.states[i].alpha_binding=Some(s);self.states[i].clocks.alpha=0.;self.states[i].stopped&=!64;}}
    /// 800CF1C4/800CF164: follow the auxiliary reference chain, match the
    /// source variant ordinal at+28, bind all channels and restart.
    pub fn bind_variant(&mut self,id:u32,ordinal:Option<u32>)->Result<(),String>{
        let i=self.index(id).ok_or("Variant target")?;let mut source=i;
        if let Some(ordinal)=ordinal{
            let mut candidate=self.program.widgets[i].source_indices[5];let mut found=None;
            for _ in 0..self.program.widgets.len(){
                if candidate<0{break;}let index=candidate as usize;let w=&self.program.widgets[index];let at=w.record_offset+0x28;
                if u32::from_be_bytes(self.bytes[at..at+4].try_into().unwrap())==ordinal{found=Some(index);break;}
                candidate=w.next.map(|n|n as i16).unwrap_or(-1);
            }
            source=found.ok_or("Native variant ordinal")?;
        }
        self.appearance[i]=self.program.widgets[source].appearance(&self.bytes)?;
        let s=&mut self.states[i];s.position_source=source;s.external_position=None;s.keep_position_axis=0;s.scale_axes=Some(self.program.widgets[source].flags);s.rgb_binding=Some(source);s.alpha_binding=Some(source);s.clocks=UiClocks::default();s.stopped=0;s.scale_speed=1.;
        Ok(())
    }
    /// Restart the current bound channels without replacing their sources.
    pub fn restart(&mut self,id:u32){if let Some(i)=self.index(id){self.states[i].clocks=UiClocks::default();self.states[i].stopped=0;self.states[i].scale_speed=1.;}}
    pub fn restart_tree(&mut self,id:u32){if let Some(i)=self.index(id){self.restart_index(i);}}
    fn restart_index(&mut self,i:usize){self.states[i].clocks=UiClocks::default();self.states[i].stopped=0;self.states[i].scale_speed=1.;let mut child=self.program.widgets[i].first_child;while let Some(c)=child{self.restart_index(c);child=self.program.widgets[c].next;}}
    pub fn restart_scale(&mut self,id:u32){if let Some(i)=self.index(id){self.states[i].clocks.scale=0.;self.states[i].scale_speed=1.;self.states[i].stopped&=!8;}}
    pub fn reverse_scale(&mut self,id:u32){if let Some(i)=self.index(id){
        self.states[i].clocks.scale=self.appearance[i].scale.as_ref().and_then(|t|t.keys.last()).map(|k|k.frame as f32).unwrap_or(0.);
        self.states[i].scale_speed=-1.;self.states[i].stopped&=!8;
    }}
    pub fn digit(&mut self,id:u32,value:u16){if let Some(i)=self.index(id){self.states[i].uv_frame=Some(value);}}
    pub fn number(&mut self,id:u32,count:u32,mut value:u32){
        for offset in 0..count {let visible=offset==0||value>0;self.hide_tree(id+offset,!visible);self.digit(id+offset,(value%10) as u16);value/=10;}
    }
    pub fn advance(&mut self,frames:f32){for state in &mut self.states {state.clocks.advance(frames,state.stopped);if state.stopped&8==0{state.clocks.scale=(state.clocks.scale+(state.scale_speed-1.)*frames).max(0.);}}}
    pub fn seek(&mut self,frame:f32){for state in &mut self.states {state.clocks=UiClocks::at(frame);}}
    pub fn draw(&self)->Vec<UiDraw>{
        let mut result:Vec<Option<UiDraw>>=vec![None;self.states.len()];
        for i in 0..self.states.len(){self.evaluate(i,&mut result);}
        result.into_iter().map(Option::unwrap).collect()
    }
    fn evaluate(&self,i:usize,out:&mut [Option<UiDraw>]) {
        if out[i].is_some(){return;}
        let w=&self.program.widgets[i];let state=&self.states[i];let a=&self.appearance[i];
        if let Some(p)=w.parent {self.evaluate(p,out);}
        let parent=w.parent.map(|p|out[p].as_ref().unwrap());
        let source=state.external_position.as_ref().unwrap_or(&self.program.widgets[state.position_source]);
        let mut position=if w.parent.is_some(){source.child_position}else{source.root_position};
        let original=if w.parent.is_some(){w.child_position}else{w.root_position};
        if matches!(state.keep_position_axis,1|3){position[0]=original[0];}
        if matches!(state.keep_position_axis,2|3){position[1]=original[1];}
        if let Some(v)=state.position {position[0]=v[0];position[1]=v[1];}
        let mut animated_position=[0.;2];
        if let Ok(Some(track))=source.position_track(&self.bytes){
            let frame=loop_frame(state.clocks.position,track.keys.first().map(|k|k.frame).unwrap_or(0)as f32,track.keys.last().map(|k|k.frame).unwrap_or(0)as f32,track.flags);
            if let Ok(v)=vector_at(&track,frame){animated_position=[v[0],v[1]];}
        }
        let scalar=a.scale.as_ref().map(|t|scalar_at(t,state.clocks.scale)).unwrap_or(1.);
        let scale_flags=state.scale_axes.unwrap_or(w.flags);
        let factors=[if scale_flags&4!=0{scalar}else{1.},if scale_flags&2!=0{scalar}else{1.}];
        let inherited=parent.map(|p|p.child_scale).unwrap_or([1.;2]);
        // 800C7204: animated envelopes multiply both inherited axes. Without
        // an envelope, source axis flags gate inherited quad dimensions.
        // Descendant scale always propagates the envelope independently.
        let propagated=[factors[0]*inherited[0],factors[1]*inherited[1]];
        let envelope=if a.scale.is_some(){propagated}else{[if scale_flags&4!=0{inherited[0]}else{1.},if scale_flags&2!=0{inherited[1]}else{1.}]};
        let mut scale=[w.size[0]*envelope[0],w.size[1]*envelope[1]];
        let child_scale=if w.flags&1!=0 {scale}else{propagated};
        let mut rotation=state.rotation.unwrap_or(a.rotation[2])+a.rotation_tracks[2].as_ref().map(|t|scalar_at(t,state.clocks.rotation[2])).unwrap_or(0.);
        rotation=rotation.to_radians();
        let mut color=w.color;
        if let Some(s)=state.rgb_binding{color[..3].copy_from_slice(&self.program.widgets[s].color[..3]);}
        if let Some(s)=state.alpha_binding{color[3]=self.program.widgets[s].color[3];}
        if let Some((flags,keys))=&a.rgb {
            let last=keys.last().map(|k|k.1).unwrap_or(0.);//800CD150 restarts the counter at0, including a leading key delay.
            let frame=loop_frame(state.clocks.rgb,0.,last,*flags);
            let rgb=if frame<=keys[0].1 {keys[0].0}else if let Some(k)=keys.windows(2).find(|k|frame<k[1].1){
                let t=(frame-k[0].1)/(k[1].1-k[0].1);[0,1,2].map(|j|k[0].0[j]+(k[1].0[j]-k[0].0[j])*t)
            }else{keys.last().unwrap().0};color[..3].copy_from_slice(&rgb);
        }
        if let Some(track)=&a.alpha {color[3]=scalar_at(track,state.clocks.alpha).clamp(0.,1.);}
        if let Some(value)=state.color{color=value;}
        let position=if let Some(p)=parent {
            let (s,c)=p.rotation.sin_cos();let inherited_position=if w.flags&0x400==0{p.child_scale}else{[1.;2]};
            let x=position[0]*inherited_position[0]+animated_position[0];let y=position[1]*inherited_position[1]+animated_position[1];
            rotation+=p.rotation;
            if a.rgb_flags&0x1000==0 {for j in 0..3{color[j]*=p.color[j];}}
            if a.alpha_flags&0x2000==0 {color[3]*=p.color[3];}
            [p.position[0]+c*x-s*y,p.position[1]+s*x+c*y]
        }else{[self.origin[0]+position[0]+animated_position[0],self.origin[1]+position[1]+animated_position[1]]};
        // 800C9640 / 800C970C: source anchors shift the quad around its native pivot.
        let anchor=anchor(a.position_anchor);
        let mut position=[position[0]-scale[0]*anchor[0],position[1]-scale[1]*anchor[1]];
        let mut uv_crop=[0.,0.,1.,1.];
        if let Some(gauge)=state.gauge {
            // 800C5224 crops the prepared vertices AND UVs around the position anchor.
            let ratio=gauge.clamp(0.,1.);let weights=[0.5+anchor[0],0.5-anchor[1]];let mut shift=[0.;2];
            for axis in 0..2 {
                if (axis==0&&a.gauge_axis==2)||(axis==1&&a.gauge_axis==1){continue;}
                shift[axis]=scale[axis]*(1.-ratio)*(weights[axis]-0.5);
                scale[axis]*=ratio;uv_crop[axis]=(1.-ratio)*weights[axis];uv_crop[axis+2]=uv_crop[axis]+ratio;
            }
            let (sin,cos)=rotation.sin_cos();position[0]+=cos*shift[0]-sin*shift[1];position[1]+=sin*shift[0]+cos*shift[1];
        }
        out[i]=Some(UiDraw{index:i,id:w.id,kind:w.kind,uv_crop,inverted:state.inverted||w.flags&0x200000!=0,repeat:a.repeat,blend_mode:a.blend_mode,tile_kind:a.tile_kind,visible:!state.forced_hidden&&w.flags&0x84001==0&&color[3]>0.,position,child_scale,scale,rotation,color,
            uv:[w.uv_indices[0],state.uv_frame.unwrap_or(w.uv_indices[1])]});
    }
}

fn anchor(kind:u32)->[f32;2] {
    match kind {1=>[0.,-0.5],2=>[-0.5,-0.5],3=>[-0.5,0.],4=>[-0.5,0.5],5=>[0.,0.5],6=>[0.5,0.5],7=>[0.5,0.],8=>[0.5,-0.5],_=>[0.;2]}
}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]pub enum PausePage {#[default]Map,PlayLog,Options,Controls,Sound,Video,TitleConfirm,TitleSplash,SaveSlots,SlotActions,SaveConfirm}
/// Native map entry calls (80247624 / 80248E9C / 80248F78 / 80249030 / 80249054).
pub fn initialize_pause_map(scene:&mut UiScene,hp:f32,continues:u32,score:u32){
    for id in [16,6,30,35,101,120] {scene.hide_tree(id,true);}
    scene.hide_tree(61,continues<10);
    scene.number(if continues<10{62}else{61},if continues<10{1}else{2},continues);
    scene.number(51,9,score);
    if let Some(i)=scene.index(70){scene.states[i].gauge=Some(hp);}
    // Initial highlight binds to the source Play Log entry; two-entry navigation remains in the controller port.
    scene.bind_position(15,10);
}

#[cfg(test)]mod scene_tests {
    use super::*;
    #[test]fn pause_scene_matches_owned_native_observation() {
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes=std::fs::read(root.join("assets/native/ui/subscr000_us/animation.idt")).unwrap();
        let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let mut scene=UiScene::new(timeline.program(0).unwrap(),&bytes,[320.,180.]).unwrap();
        initialize_pause_map(&mut scene,1.,0,0);scene.seek(20.);
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-pause-program0-observed.json")).unwrap()).unwrap();
        let draws=scene.draw();
        assert_eq!(draws[63].repeat,[2.,5.]);assert_eq!(draws[63].tiles().len(),10);
        assert_eq!(draws[67].repeat,[1.,8.]);assert_eq!(draws[67].tiles().len(),8);
        assert_eq!(draws[67].tiles()[0].size,[640.,37.5]);
        let mut fractional=draws[63].clone();fractional.repeat=[1.5,1.];fractional.rotation=0.;
        let pieces=fractional.tiles();assert_eq!(pieces.len(),2);
        assert_eq!(pieces[1].uv_fraction,[0.,0.,0.5,1.]);
        assert!((pieces[0].size[0]+pieces[1].size[0]-640.).abs()<0.0001);
        for binding in observed["bindings"].as_array().unwrap() {
            let i=binding["index"].as_u64().unwrap()as usize;let o=&binding["observed"];let d=&draws[i];
            if o["hidden"]==0 && ![65,66,158].contains(&i) {
                for axis in 0..2 {
                    let position=o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32;
                    assert!((d.position[axis]-position).abs()<0.0001,"widget {i} position axis {axis}");
                    let size=o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.};
                    assert!((d.scale[axis]-size).abs()<0.0001,"widget {i} size axis {axis}");
                }
                for channel in 0..4 {assert!((d.color[channel]-o["rgba"][channel].as_f64().unwrap()as f32).abs()<0.0001,"widget {i} color {channel}");}
                assert!(d.visible,"widget {i} unexpectedly hidden");
            }
        }
    }
}


#[derive(Clone,Debug)]pub struct UiTile {
    pub center:[f32;2],pub size:[f32;2],pub uv_fraction:[f32;4],
}
impl UiDraw {
    /// 800C97DC: repeated atlas subrectangles, including fractional last tiles.
    /// Scroll counters and special distorted tile kinds are separate runtime state.
    pub fn tiles(&self)->Vec<UiTile> {
        let full=self.scale.map(f32::abs);let cell=[full[0]/self.repeat[0],full[1]/self.repeat[1]];
        if full.iter().any(|v|*v<=0.){return vec![];}
        let (sin,cos)=self.rotation.sin_cos();let mut tiles=vec![];
        for y in 0..self.repeat[1].ceil()as usize {for x in 0..self.repeat[0].ceil()as usize {
            let fraction=[(self.repeat[0]-x as f32).min(1.),(self.repeat[1]-y as f32).min(1.)];
            let size=[cell[0]*fraction[0],cell[1]*fraction[1]];
            let local=[(cell[0]*x as f32+size[0]*0.5-full[0]*0.5)*self.scale[0].signum(),
                (cell[1]*y as f32+size[1]*0.5-full[1]*0.5)*self.scale[1].signum()];
            tiles.push(UiTile{center:[self.position[0]+cos*local[0]-sin*local[1],self.position[1]+sin*local[0]+cos*local[1]],
                size,uv_fraction:[self.uv_crop[0],self.uv_crop[1],self.uv_crop[0]+fraction[0]*(self.uv_crop[2]-self.uv_crop[0]),self.uv_crop[1]+fraction[1]*(self.uv_crop[3]-self.uv_crop[1])]});
        }}
        tiles
    }
}

#[cfg(test)]mod observed_channel_tests {
    use super::*;
    #[test]fn independent_native_clocks_match_scanning_strips_and_settled_groups(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes=std::fs::read(root.join("assets/native/ui/subscr000_us/animation.idt")).unwrap();
        let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let mut scene=UiScene::new(timeline.program(0).unwrap(),&bytes,[320.,180.]).unwrap();
        initialize_pause_map(&mut scene,1.,2,0);
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-pause-material-observed.json")).unwrap()).unwrap();
        for b in observed["bindings"].as_array().unwrap(){
            let c=&b["observed"]["channelFrames"];let state=&mut scene.states[b["index"].as_u64().unwrap()as usize];
            state.clocks.position=c["position"].as_f64().unwrap()as f32;state.clocks.scale=c["scale"].as_f64().unwrap()as f32;
            // 800CB358 increments timer200 after evaluation; stopped nonloop endpoints keep their last time.
            if let Some(track)=timeline.program(0).unwrap().widgets[b["index"].as_u64().unwrap()as usize].position_track(&bytes).unwrap(){
                let end=track.keys.last().unwrap().frame as f32;
                if state.clocks.position>0.&&(track.flags&4!=0||state.clocks.position<end){state.clocks.position-=1.;}
            }
            state.clocks.rgb=c["rgb"].as_f64().unwrap()as f32;state.clocks.alpha=c["alpha"].as_f64().unwrap()as f32;
        }
        let draws=scene.draw();
        for b in observed["bindings"].as_array().unwrap(){
            let o=&b["observed"];if o["hidden"]!=0{continue;}
            let i=b["index"].as_u64().unwrap()as usize;let d=&draws[i];
            for axis in 0..2 {
                assert!((d.position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0001,"widget {i} position {axis}");
                assert!((d.scale[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0001,"widget {i} size {axis}");
            }
            for channel in 0..4{assert!((d.color[channel]-o["rgba"][channel].as_f64().unwrap()as f32).abs()<0.0001,"widget {i} color {channel}");}
        }
        scene.seek(45.);scene.bind_position(15,20);let i=scene.index(15).unwrap();
        assert_eq!(scene.states[i].clocks.position,0.);assert_eq!(scene.states[i].clocks.scale,45.);
        let map=mw_data::ui::StageMap::parse(&std::fs::read_to_string(root.join("assets/native/ui/c201-map/map.json")).unwrap()).unwrap();
        assert_eq!(map.layer(0.),0);assert_eq!(map.layer(map.height_thresholds[0]),0);assert_eq!(map.layer(17.55),2);
        let projected=map.project([2.5,17.55,-61.],[-80.,12.],250.);
        assert!((projected[0]+75.53571).abs()<0.0001);assert!((projected[1]+61.214287).abs()<0.0001);
    }
}

/// 802391A0 / 802397E0: pause-context Options screen61 entry and highlight binding.
pub fn initialize_options(scene:&mut UiScene,selection:usize){
    for id in [6,20]{scene.hide_tree(id,true);}
    scene.bind_position_axis(5,[11,12,13,15][selection.min(3)],1);
}

#[cfg(test)]mod options_tests {
    use super::*;
    #[test]fn options_entry_matches_original_selection_and_visibility(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes=std::fs::read(root.join("assets/native/ui/subscr000_us/animation.idt")).unwrap();
        let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();let mut scene=UiScene::new(timeline.program(61).unwrap(),&bytes,[320.,180.]).unwrap();
        initialize_options(&mut scene,0);scene.seek(20.);let draws=scene.draw();
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-options-observed.json")).unwrap()).unwrap();
        for b in observed["bindings"].as_array().unwrap(){let o=&b["observed"];let i=b["index"].as_u64().unwrap()as usize;let d=&draws[i];
            if o["hidden"]!=0{continue;}
            if scene.program.widgets[i].flags&0x4000!=0{assert!(!d.visible);continue;}
            for axis in 0..2{assert!((d.position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0001,"widget {i} position {axis}: {:?}",d.position);}
            assert!(d.visible,"widget {i} hidden");
        }
        scene.bind_position_axis(5,12,1);scene.seek(20.);let first=draws[scene.index(5).unwrap()].position;let second=scene.draw()[scene.index(5).unwrap()].position;
        assert_eq!(first[0],second[0]);assert_eq!(second[1]-first[1],24.);
    }
}

/// Native two-entry map and four-entry pause-context Options navigation.
/// Page controllers: 80248730,80239AEC,8023A900. Other confirmations remain unported.
#[derive(Clone,Debug,Default)]pub struct PauseNavigation {
    pub page:PausePage,pub map_selection:usize,pub options_selection:usize,pub playlog_page:usize,
}
impl PauseNavigation {
    pub fn step(&mut self,direction:i32){
        let (selection,count)=match self.page{PausePage::Map=>(&mut self.map_selection,2),PausePage::Options=>(&mut self.options_selection,4),_=>return};
        *selection=(*selection as i32+direction).rem_euclid(count)as usize;
    }
    pub fn turn_page(&mut self,direction:i32,count:usize){if self.page==PausePage::PlayLog&&count>0{self.playlog_page=(self.playlog_page as i32+direction).rem_euclid(count as i32)as usize;}}
    pub fn confirm(&mut self){match self.page{
        PausePage::Map if self.map_selection==0=>{self.page=PausePage::PlayLog;self.playlog_page=0;},
        PausePage::Map if self.map_selection==1=>{self.page=PausePage::Options;self.options_selection=0;},
        PausePage::Options if self.options_selection==0=>self.page=PausePage::Controls,
        PausePage::Options if self.options_selection==1=>self.page=PausePage::Sound,
        PausePage::Options if self.options_selection==2=>self.page=PausePage::Video,
        PausePage::Options if self.options_selection==3=>self.page=PausePage::TitleConfirm,
        _=>{}
    }}
    /// True only when backing out of the map resumes gameplay.
    pub fn back(&mut self)->bool{match self.page{
        PausePage::Map=>return true,
        PausePage::TitleSplash|PausePage::TitleConfirm|PausePage::SaveSlots|PausePage::SlotActions|PausePage::SaveConfirm=>return false,
        PausePage::Controls|PausePage::Sound|PausePage::Video=>self.page=PausePage::Options,
        _=>self.page=PausePage::Map,
    }false}
}
#[cfg(test)]mod navigation_tests {
    use super::*;
    #[test]fn controls_back_returns_to_options_and_map_before_resuming(){
        let mut menu=PauseNavigation::default();menu.step(-1);assert_eq!(menu.map_selection,1);
        menu.confirm();assert_eq!(menu.page,PausePage::Options);menu.step(-1);assert_eq!(menu.options_selection,3);
        menu.step(1);assert_eq!(menu.options_selection,0);menu.confirm();assert_eq!(menu.page,PausePage::Controls);
        assert!(!menu.back());assert_eq!(menu.page,PausePage::Options);assert_eq!(menu.options_selection,0);
        assert!(!menu.back());assert_eq!(menu.page,PausePage::Map);assert_eq!(menu.map_selection,1);assert!(menu.back());
    }
}

#[cfg(test)]mod controls_frame_tests {
    use super::*;
    #[test]fn controls_uses_the_native_larger_tv_frame(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes=std::fs::read(root.join("assets/native/ui/subscr000_us/animation.idt")).unwrap();let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let mut map=UiScene::new(timeline.program(0).unwrap(),&bytes,[320.,180.]).unwrap();initialize_pause_map(&mut map,1.,2,0);
        map.hide_tree(5,true);map.hide_tree(6,false);map.restart_scale(6);map.advance(20.);
        let draws=map.draw();let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-controls-map-observed.json")).unwrap()).unwrap();
        for i in 81..=92 {
            let o=&observed["bindings"].as_array().unwrap().iter().find(|b|b["index"]==i).unwrap()["observed"];
            for axis in 0..2 {
                assert!((draws[i].position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0001,"widget {i} position {axis}");
                assert!((draws[i].scale[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0001,"widget {i} size {axis}");
            }
        }
        let mut options=UiScene::new(timeline.program(61).unwrap(),&bytes,[320.,180.]).unwrap();options.select_tree(11,true);
        let draws=options.draw();for i in [35,36,37]{assert!(draws[i].inverted);}
        assert!(!draws[33].inverted);options.select_tree(11,false);assert!(!options.draw()[36].inverted);
    }
}

/// Native Sound screen70: music, SFX/game voice, cutscene voice, announcers, subtitles, default.
#[derive(Clone,Debug,PartialEq,Eq,serde::Serialize,serde::Deserialize)]#[serde(default)]
pub struct SoundSettings {pub music_level:u8,pub sfx_level:u8,pub cutscene_level:u8,pub announcer_level:u8,pub subtitles:bool}
impl Default for SoundSettings{fn default()->Self{Self{music_level:10,sfx_level:10,cutscene_level:10,announcer_level:10,subtitles:false}}}
impl SoundSettings{
    pub fn levels(&self)->[u8;4]{[self.music_level,self.sfx_level,self.cutscene_level,self.announcer_level].map(|v|v.min(10))}
    pub fn set_level(&mut self,row:usize,level:u8){match row{0=>self.music_level=level.min(10),1=>self.sfx_level=level.min(10),2=>self.cutscene_level=level.min(10),3=>self.announcer_level=level.min(10),_=>{}}}
}
#[derive(Clone,Debug)]pub struct SoundEdit{pub original:SoundSettings,pub draft:SoundSettings,pub row:usize,pub save_yes:Option<bool>,pub confirmation_age:f32}
#[derive(Clone,Debug)]pub struct SoundOutcome{pub settings:SoundSettings,pub save:bool}
impl SoundEdit{
    pub fn new(settings:SoundSettings)->Self{Self{original:settings.clone(),draft:settings,row:0,save_yes:None,confirmation_age:0.}}
    /// Screen86 buttons finish opening at frame26; hidden choices must never accept input.
    pub fn advance(&mut self,frames:f32){if self.save_yes.is_some(){self.confirmation_age+=frames;}}
    pub fn step(&mut self,direction:i32){if self.save_yes.is_none(){self.row=(self.row as i32+direction).rem_euclid(6)as usize;}}
    pub fn adjust(&mut self,direction:i32){
        if let Some(yes)=&mut self.save_yes{if self.confirmation_age>=26.{*yes=direction<0;}}
        else if self.row<4{let value=(self.draft.levels()[self.row]as i32+direction).clamp(0,10)as u8;self.draft.set_level(self.row,value);}
    }
    pub fn confirm(&mut self)->Option<SoundOutcome>{
        if let Some(yes)=self.save_yes{if self.confirmation_age<26.{return None;}return Some(SoundOutcome{settings:if yes{self.draft.clone()}else{self.original.clone()},save:yes});}
        if self.row==4{self.draft.subtitles=!self.draft.subtitles;}
        if self.row==5{self.draft=SoundSettings::default();}
        None
    }
    pub fn back(&mut self)->Option<SoundOutcome>{
        if let Some(yes)=&mut self.save_yes{if self.confirmation_age>=26.{*yes=false;}return None;}
        if self.original==self.draft{Some(SoundOutcome{settings:self.original.clone(),save:false})}else{self.save_yes=Some(false);self.confirmation_age=0.;None}
    }
}
pub fn initialize_sound(scene:&mut UiScene){for id in [51,61]{scene.hide_tree(id,true);}}
pub fn update_sound(scene:&mut UiScene,edit:&SoundEdit){
    let anchors=[10,20,30,40,60,50];let Some(highlight)=scene.index(5)else{return;};
    if let Some(source)=scene.index(anchors[edit.row]){let p=scene.program.widgets[source].child_position;scene.states[highlight].position=Some([p[0],p[1]]);}
    for (row,level)in edit.draft.levels().into_iter().enumerate(){
        if let Some(i)=scene.index([11,21,31,41][row]){scene.states[i].gauge=Some(level as f32/10.);scene.states[i].color=Some(if row==edit.row{[0.9,0.7,0.,1.]}else{[1.;4]});}
        scene.number([15,25,35,45][row],2,level as u32);
    }
    scene.digit(65,if edit.draft.subtitles{12}else{13});
}

#[cfg(test)]mod sound_tests{
    use super::*;
    #[test]fn sound_edit_previews_clamps_resets_and_requires_apply_choice(){
        let mut edit=SoundEdit::new(SoundSettings::default());assert!(!edit.back().unwrap().save);
        edit.adjust(-20);assert_eq!(edit.draft.music_level,0);edit.adjust(100);assert_eq!(edit.draft.music_level,10);
        edit.row=4;edit.confirm();assert!(edit.draft.subtitles);assert!(edit.back().is_none());assert_eq!(edit.save_yes,Some(false));
        assert!(edit.confirm().is_none());edit.advance(26.);edit.back();let no=edit.confirm().unwrap();assert!(!no.save);assert!(!no.settings.subtitles);
        edit.adjust(-1);let yes=edit.confirm().unwrap();assert!(yes.save);assert!(yes.settings.subtitles);
        let mut custom=SoundSettings::default();custom.sfx_level=3;custom.announcer_level=0;
        let mut edit=SoundEdit::new(custom.clone());edit.step(-1);assert_eq!(edit.row,5);edit.confirm();assert_eq!(edit.draft,SoundSettings::default());
        edit.back();edit.advance(26.);assert_eq!(edit.confirm().unwrap().settings,custom);
    }
    fn sources()->(std::path::PathBuf,Vec<u8>,mw_data::ui::UiTimeline){let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let b=std::fs::read(root.join("assets/native/ui/subscr000_us/animation.idt")).unwrap();let t=mw_data::ui::UiTimeline::parse(&b).unwrap();(root,b,t)}
    #[test]fn partial_hp_fill_matches_native_prepared_vertices(){
        let(root,b,t)=sources();let mut scene=UiScene::new(t.program(0).unwrap(),&b,[320.,180.]).unwrap();initialize_pause_map(&mut scene,0.7,2,0);scene.seek(20.);
        let d=scene.draw()[scene.index(70).unwrap()].clone();
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-sound-map-observed.json")).unwrap()).unwrap();
        let o=&observed["bindings"].as_array().unwrap().iter().find(|v|v["sourceId"]==70).unwrap()["observed"];
        let v=o["preparedVertices"].as_array().unwrap();let x:Vec<f32>=v.chunks(3).map(|p|p[0].as_f64().unwrap()as f32).collect();
        let left=x.iter().copied().fold(f32::INFINITY,f32::min);let right=x.iter().copied().fold(f32::NEG_INFINITY,f32::max);
        assert!((d.position[0]-d.scale[0]*0.5-left).abs()<0.0001);assert!((d.position[0]+d.scale[0]*0.5-right).abs()<0.0001);
        assert!((d.uv_crop[0]-0.3).abs()<0.00001);assert_eq!(d.uv_crop[2],1.);
    }
    #[test]fn sound_page_defaults_match_original_capture(){
        let(root,b,t)=sources();let mut scene=UiScene::new(t.program(70).unwrap(),&b,[320.,180.]).unwrap();initialize_sound(&mut scene);update_sound(&mut scene,&SoundEdit::new(SoundSettings::default()));scene.seek(20.);let draws=scene.draw();
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-sound-observed.json")).unwrap()).unwrap();
        for b in observed["bindings"].as_array().unwrap(){let o=&b["observed"];let i=b["index"].as_u64().unwrap()as usize;
            if o["hidden"]!=0||o["rgba"][3].as_f64().unwrap()==0.||scene.program.widgets[i].flags&0x4000!=0{continue;}
            let d=&draws[i];assert!(d.visible,"widget {i} unexpectedly hidden");
            for axis in 0..2{assert!((d.position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0001,"widget {i} position {axis}: {:?}",d.position);}
        }
    }
}

/// PC native Video screen76 is the full-screen TV calibration/help page.
/// Its original input8023BD18 accepts Return in the initial mode; VI-origin
/// adjustment is only in a separate submode not entered by this PC menu.
pub fn initialize_video(scene:&mut UiScene){for id in [5,6]{scene.hide_tree(id,true);}}
#[cfg(test)]mod video_tests{
    use super::*;
    #[test]fn video_page_matches_native_calibration_capture(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes=std::fs::read(root.join("assets/native/ui/subscr000_us/animation.idt")).unwrap();
        let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let mut scene=UiScene::new(timeline.program(76).unwrap(),&bytes,[320.,180.]).unwrap();initialize_video(&mut scene);scene.seek(120.);
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-video-observed.json")).unwrap()).unwrap();
        let draws=scene.draw();let mut checked=0;
        // The native controller creates two instances of the same screen. Both
        // have identical visible calibration quads; compare the first instance.
        for binding in observed["bindings"].as_array().unwrap().iter().take(60){
            let i=binding["index"].as_u64().unwrap()as usize;let o=&binding["observed"];let d=&draws[i];
            if scene.program.widgets[i].flags&0x4001!=0 {assert!(!d.visible);continue;}
            if o["hidden"]==1{assert!(!d.visible,"native hidden widget {i}");continue;}
            assert!(d.visible,"native visible widget {i}");
            for axis in 0..2{
                assert!((d.position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0001,"widget {i} position {axis}");
                assert!((d.scale[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0001,"widget {i} size {axis}");
            }
            checked+=1;
        }
        assert!(checked>=10);
    }
    #[test]fn video_back_keeps_options_selection_before_map_and_resume(){
        let mut menu=PauseNavigation::default();menu.step(1);menu.confirm();menu.step(2);menu.confirm();
        assert_eq!(menu.page,PausePage::Video);menu.step(1);menu.confirm();assert_eq!(menu.page,PausePage::Video);
        assert!(!menu.back());assert_eq!(menu.page,PausePage::Options);assert_eq!(menu.options_selection,2);
        assert!(!menu.back());assert_eq!(menu.page,PausePage::Map);assert!(menu.back());
    }
}

/// Map controller80247624: keep the original TV frame/legend; replace map with
/// five message rows and source page/Return prompts, with confirmed-entry fill.
pub fn initialize_playlog(scene:&mut UiScene){
    for id in [100,71]{scene.hide_tree(id,true);}for id in [30,35,16]{scene.hide_tree(id,false);}
    scene.select_tree(10,true);
}
pub fn update_playlog(scene:&mut UiScene,page:usize,count:usize){scene.number(36,2,count as u32);scene.number(38,2,(page+1)as u32);}

#[cfg(test)]mod playlog_tests{
    use super::*;
    #[test]fn playlog_frame_and_counter_match_owned_native_page(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/subscr000_us/animation.idt")).unwrap();let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let mut scene=UiScene::new(timeline.program(0).unwrap(),&bytes,[320.,180.]).unwrap();initialize_pause_map(&mut scene,1.,2,0);initialize_playlog(&mut scene);update_playlog(&mut scene,0,10);scene.seek(120.);
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-playlog-map-observed.json")).unwrap()).unwrap();let draws=scene.draw();let mut checked=0;
        for b in observed["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let o=&b["observed"];let d=&draws[i];
            if scene.program.widgets[i].flags&0x4001!=0||[65,66,158].contains(&i){continue;}
            if o["hidden"]==1{assert!(!d.visible,"native hidden widget {i}");continue;}
            assert!(d.visible,"native visible widget {i}");
            for axis in 0..2{assert!((d.position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0002,"widget {i} position {axis}");assert!((d.scale[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0002,"widget {i} size {axis}");}
            checked+=1;
        }assert!(checked>=40);assert!(draws[141].inverted);assert!(draws[142].inverted);
    }
    #[test]fn original_playlog_pages_wrap_and_back_returns_to_map(){
        let mut menu=PauseNavigation::default();menu.confirm();assert_eq!(menu.page,PausePage::PlayLog);
        menu.turn_page(-1,10);assert_eq!(menu.playlog_page,9);menu.turn_page(1,10);assert_eq!(menu.playlog_page,0);
        assert!(!menu.back());assert_eq!(menu.page,PausePage::Map);assert!(menu.back());
    }
}

/// cIdOptTitle:26-frame opening, five4-frame color confirmations,15-frame close.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum TitlePhase{Opening,Selecting,Confirming,Closing,Done}
#[derive(Clone,Debug)]pub struct TitlePrompt{pub yes:bool,pub phase:TitlePhase,pub age:f32}
impl Default for TitlePrompt{fn default()->Self{Self{yes:false,phase:TitlePhase::Opening,age:0.}}}
impl TitlePrompt{
    pub fn advance(&mut self,frames:f32)->Option<bool>{
        self.age+=frames;
        let duration=match self.phase{TitlePhase::Opening=>26.,TitlePhase::Confirming=>20.,TitlePhase::Closing=>15.,_=>return None};
        if self.age<duration{return None;}self.age=0.;match self.phase{
            TitlePhase::Opening=>self.phase=TitlePhase::Selecting,
            TitlePhase::Confirming if self.yes=>{self.phase=TitlePhase::Done;return Some(true);},
            TitlePhase::Confirming=>self.phase=TitlePhase::Closing,
            TitlePhase::Closing=>{self.phase=TitlePhase::Done;return Some(false);},_=>{},
        }None
    }
    pub fn choose(&mut self,yes:bool){if self.phase==TitlePhase::Selecting{self.yes=yes;}}
    pub fn back(&mut self){self.choose(false);}
    pub fn confirm(&mut self){if self.phase==TitlePhase::Selecting{self.phase=TitlePhase::Confirming;self.age=0.;}}
}
pub fn update_title_prompt(scene:&mut UiScene,prompt:&TitlePrompt){
    scene.hide_tree(9,prompt.phase==TitlePhase::Opening);
    let selected=if prompt.yes{11}else{12};
    if scene.states[scene.index(9).unwrap()].position_source!=scene.index(selected).unwrap(){scene.bind_position_axis(9,selected,2);}
    if prompt.phase==TitlePhase::Closing{let i=scene.index(1).unwrap();scene.states[i].clocks.scale=(15.-prompt.age).max(0.);}
    if prompt.phase==TitlePhase::Confirming{
        // Original RGB track13 switches white -> gold(frame2) -> white(frame4).
        let rgb=if prompt.age.rem_euclid(4.)>=2.{[0.9,0.7,0.,1.]}else{[1.;4]};
        for id in [9,selected]{let i=scene.index(id).unwrap();scene.states[i].color=Some(rgb);}
    }
}
#[cfg(test)]mod title_tests{
    use super::*;
    #[test]fn title_default_no_and_opening_gate_preserve_fight_until_yes_confirmation(){
        let mut p=TitlePrompt::default();p.choose(true);p.confirm();assert!(!p.yes);assert_eq!(p.phase,TitlePhase::Opening);
        assert_eq!(p.advance(26.),None);p.choose(true);p.back();assert!(!p.yes);assert_eq!(p.phase,TitlePhase::Selecting);
        p.confirm();p.choose(true);assert!(!p.yes);assert_eq!(p.advance(19.),None);assert_eq!(p.advance(1.),None);assert_eq!(p.phase,TitlePhase::Closing);assert_eq!(p.advance(15.),Some(false));
        let mut p=TitlePrompt::default();p.advance(26.);p.choose(true);p.confirm();assert_eq!(p.advance(20.),Some(true));
        let mut menu=PauseNavigation::default();menu.step(1);menu.confirm();menu.step(3);menu.confirm();assert_eq!(menu.page,PausePage::TitleConfirm);assert!(!menu.back());menu.page=PausePage::TitleSplash;assert!(!menu.back());
    }
}

pub fn initialize_title_splash(prompt:&mut UiScene){for id in [9,10,11,12,15,16,17,20,21,25,26]{prompt.hide_tree(id,true);}}
#[cfg(test)]mod title_art_tests{
    use super::*;
    #[test]fn title_logo_and_press_geometry_match_owned_e00_capture(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/idpac_ce00_us/animation.idt")).unwrap();let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();let mut checked=0;
        for (program,file)in [(0,"research/ui-title-logo-observed.json"),(2,"research/ui-title-splash-observed.json")]{
            let mut scene=UiScene::new(timeline.program(program).unwrap(),&bytes,[320.,180.]).unwrap();if program==2{initialize_title_splash(&mut scene);}scene.seek(180.);
            let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join(file)).unwrap()).unwrap();let draws=scene.draw();
            for b in observed["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let o=&b["observed"];if o["hidden"]!=0||o["rgba"][3].as_f64().unwrap()<=0.||scene.program.widgets[i].flags&0x4001!=0{continue;}
                for axis in 0..2{assert!((draws[i].position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0002,"program {program} widget {i} position {axis}");assert!((draws[i].scale[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0002,"program {program} widget {i} size {axis}");}checked+=1;
            }
        }assert!(checked>=8);
    }
}

#[cfg(test)]mod title_dialog_tests{
    use super::*;
    #[test]fn title_confirmation_outline_and_choices_match_native_geometry(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/subscr000_us/animation.idt")).unwrap();let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();let mut scene=UiScene::new(timeline.program(85).unwrap(),&bytes,[320.,180.]).unwrap();scene.bind_position_axis(9,12,2);
        update_title_prompt(&mut scene,&TitlePrompt{yes:false,phase:TitlePhase::Selecting,age:0.});scene.seek(120.);
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-title-dialog-observed.json")).unwrap()).unwrap();
        // Match the observed scanline phase; native running position clocks increment
        // after evaluation, while completed nonloop tracks stay at their endpoint.
        for b in observed["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let mut frame=b["observed"]["channelFrames"]["position"].as_f64().unwrap()as f32;
            let source=scene.states[i].position_source;if let Some(track)=scene.program.widgets[source].position_track(&bytes).unwrap(){let end=track.keys.last().unwrap().frame as f32;if frame>0.&&(track.flags&4!=0||frame<end){frame-=1.;}}
            scene.states[i].clocks.position=frame;
        }
        let draws=scene.draw();let mut checked=0;
        for b in observed["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let o=&b["observed"];if scene.program.widgets[i].flags&0x4001!=0||o["hidden"]!=0{continue;}
            assert!(draws[i].visible,"widget {i} visible");for axis in 0..2{assert!((draws[i].position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0002,"widget {i} position {axis}");assert!((draws[i].scale[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0002,"widget {i} size {axis}");}checked+=1;
        }assert!(checked>=20);
    }
}

/// Initial empty save slots: cIdOptLoadGame8023C290/CFD4/D994/DF8C.
#[derive(Clone,Debug,Default)]pub struct SaveSelection{pub selected:usize,pub age:f32}
impl SaveSelection{
    pub fn advance(&mut self,frames:f32){self.age+=frames;}
    pub fn step(&mut self,direction:i32){if self.age>=30.{self.selected=(self.selected as i32+direction).clamp(0,3)as usize;}}
}
pub fn initialize_save_frame(scene:&mut UiScene){scene.hide_tree(40,true);scene.hide_tree(30,false);}
pub fn initialize_empty_slot(scene:&mut UiScene,frame:&UiScene,slot:usize){
    assert!(slot<4);scene.bind_position_from(1,frame,5+slot as u32);
    for id in [15,14,13,11,10,20]{scene.hide_tree(id,true);}scene.hide_tree(16,false);scene.number(41,2,(slot+1)as u32);
}
pub fn update_save_selection(frame:&mut UiScene,rows:&mut[UiScene],selected:usize){
    let source=frame.index(selected as u32+5).unwrap();let target=frame.index(30).unwrap();
    if frame.states[target].position_source!=source{frame.bind_position(30,selected as u32+5);}
    for (i,row)in rows.iter_mut().enumerate(){row.color_preset(1,if i==selected{0}else{2});}
}

pub fn initialize_empty_slot_actions(scene:&mut UiScene){
    scene.hide_tree(10,false);for id in [6,12,13]{scene.hide_tree(id,true);}scene.hide_tree(5,false);scene.bind_position_axis(5,11,1);
}

#[cfg(test)]mod save_slot_tests{
    use super::*;
    #[test]fn original_save_frame_and_four_empty_rows_match_owned_native_capture(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/idpac_ce00_us/animation.idt")).unwrap();let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let mut frame=UiScene::new(timeline.program(80).unwrap(),&bytes,[320.,180.]).unwrap();initialize_save_frame(&mut frame);let mut rows=vec![];
        for i in 0..4{let mut row=UiScene::new(timeline.program(81).unwrap(),&bytes,[320.,180.]).unwrap();initialize_empty_slot(&mut row,&frame,i);rows.push(row);}
        update_save_selection(&mut frame,&mut rows,0);let mut scenes=vec![frame];scenes.extend(rows);for scene in &mut scenes{scene.seek(120.);}
        let mut checked=0;
        for (file,is_frame)in [("research/ui-save-slots-frame-observed.json",true),("research/ui-save-slots-rows-observed.json",false)]{
            let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join(file)).unwrap()).unwrap();
            for b in observed["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let o=&b["observed"];let slot=if is_frame{0}else{match o["cluster"].as_u64().unwrap(){1=>1,4=>2,5=>3,6=>4,_=>panic!("native slot cluster")}};let scene=&mut scenes[slot];
                let mut at=o["channelFrames"]["position"].as_f64().unwrap()as f32;let source=scene.states[i].external_position.as_ref().unwrap_or(&scene.program.widgets[scene.states[i].position_source]);
                if let Some(track)=source.position_track(&bytes).unwrap(){let end=track.keys.last().unwrap().frame as f32;if at>0.&&(track.flags&4!=0||at<end){at-=1.;}}scene.states[i].clocks.position=at;
            }
            let draws:Vec<_>=scenes.iter().map(UiScene::draw).collect();
            for b in observed["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let o=&b["observed"];let slot=if is_frame{0}else{match o["cluster"].as_u64().unwrap(){1=>1,4=>2,5=>3,6=>4,_=>unreachable!()}};
                let d=&draws[slot][i];if scenes[slot].program.widgets[i].flags&0x4001!=0{continue;}
                if o["hidden"]==1{assert!(!d.visible,"scene {slot} widget {i} hidden");continue;}assert!(d.visible,"scene {slot} widget {i} visible");
                for axis in 0..2{assert!((d.position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0002,"scene {slot} widget {i} position {axis}");assert!((d.scale[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0002,"scene {slot} widget {i} size {axis}");}
                for channel in 0..4{assert!((d.color[channel]-o["rgba"][channel].as_f64().unwrap()as f32).abs()<0.0002,"scene {slot} widget {i} color {channel}");}checked+=1;
            }
        }assert!(checked>55);
    }
    #[test]fn save_selection_waits_for_opening_and_clamps_instead_of_wrapping(){
        let mut s=SaveSelection::default();s.step(3);assert_eq!(s.selected,0);s.advance(30.);s.step(-1);assert_eq!(s.selected,0);for _ in 0..5{s.step(1);}assert_eq!(s.selected,3);s.step(-1);assert_eq!(s.selected,2);
    }
}

#[cfg(test)]mod slot_action_tests{
    use super::*;
    #[test]fn empty_slot_only_exposes_start_as_in_the_native_capture(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/idpac_ce00_us/animation.idt")).unwrap();let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();let mut scene=UiScene::new(timeline.program(83).unwrap(),&bytes,[320.,180.]).unwrap();initialize_empty_slot_actions(&mut scene);scene.seek(120.);
        let o:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-save-actions-observed.json")).unwrap()).unwrap();let draws=scene.draw();let mut checked=0;
        for b in o["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let n=&b["observed"];let d=&draws[i];if scene.program.widgets[i].flags&0x4001!=0{continue;}
            if n["hidden"]==1{assert!(!d.visible,"widget {i} hidden");continue;}assert!(d.visible,"widget {i} visible");for axis in 0..2{assert!((d.position[axis]-n["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0002,"widget {i} position {axis}");assert!((d.scale[axis]-n["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0002,"widget {i} size {axis}");}checked+=1;
        }assert_eq!(checked,8);
    }
}
