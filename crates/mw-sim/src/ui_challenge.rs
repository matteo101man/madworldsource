//! Stage201 cIdChallengeBoss80243358/802434D0 and base80242834.
//! Explicit world-update edges, including endpoint holds in the native clocks.
use crate::ui::UiScene;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Phase{Opening,Selecting,Confirm,Flashing,Resolve,Closing,Done}
impl Phase{pub fn native(self)->u32{match self{Self::Opening=>0,Self::Selecting=>1,Self::Confirm=>2,Self::Flashing=>3,Self::Resolve=>6,Self::Closing=>7,Self::Done=>8}}}
#[derive(Default,Clone,Copy)]pub struct Input{pub yes:bool,pub no:bool,pub back:bool,pub confirm:bool}
pub struct Challenge{pub scene:UiScene,pub phase:Phase,pub yes:bool,pub result:bool,pub age:u32,pub ranking:u32,pub boss:u32}
impl Challenge{
    pub fn new(mut scene:UiScene)->Self{
        assert_eq!(scene.program.id,42);scene.hide_tree(9,true);scene.digit(21,3);
        Self{scene,phase:Phase::Opening,yes:true,result:false,age:0,ranking:0,boss:3}
    }
    pub fn message_visible(&self)->bool{matches!(self.phase,Phase::Selecting|Phase::Confirm|Phase::Flashing|Phase::Resolve)}
    pub fn step(&mut self,input:Input){
        if self.phase==Phase::Done{return;}self.scene.advance(1.);self.age+=1;
        match self.phase{
            Phase::Opening=>if self.age==24{self.phase=Phase::Selecting;self.age=0;},
            Phase::Selecting=>{
                if input.no{self.yes=false;}if input.yes{self.yes=true;}if input.back{self.yes=false;}
                self.scene.hide_tree(9,false);
                // Controller copies the selected entry's evaluated local position.
                let selected=if self.yes{11}else{12};let i=self.scene.index(selected).unwrap();
                let w=&self.scene.program.widgets[i];let track=w.position_track(self.scene.bytes()).unwrap();
                let offset=track.as_ref().map(|t|crate::ui::vector_at(t,self.scene.states[i].clocks.position).unwrap()).unwrap_or([0.;3]);
                let cursor=self.scene.index(9).unwrap();self.scene.states[cursor].position=Some([w.child_position[0]+offset[0],w.child_position[1]+offset[1]]);
                if input.confirm{self.phase=Phase::Confirm;self.age=0;}
            },
            Phase::Confirm=>{for id in [9,if self.yes{11}else{12}]{self.scene.bind_rgb(id,13);}self.phase=Phase::Flashing;self.age=0;},
            Phase::Flashing=>{
                if self.age==25{assert!(!self.yes||self.boss>=self.ranking,"Unavailable boss ranking path is not translated");self.result=self.yes;self.phase=Phase::Resolve;self.age=0;}
                else if self.age%5==0{for id in [9,if self.yes{11}else{12}]{let i=self.scene.index(id).unwrap();self.scene.states[i].clocks.rgb=0.;}}
            },
            Phase::Resolve=>{self.scene.bind_scale(1,2);self.phase=Phase::Closing;self.age=0;},
            Phase::Closing=>if self.age==4{self.phase=Phase::Done;self.age=0;},
            Phase::Done=>{},
        }
    }
}

#[cfg(test)]mod tests{
    use super::*;
    fn root()->std::path::PathBuf{std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")}
    #[test]fn controller_and_quads_replay_native_accept_and_decline(){
        let bytes=std::fs::read(root().join("assets/native/ui/core_us/animation.idt")).unwrap();let bank=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let fixture:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root().join("research/ui-challenge-observed.json")).unwrap()).unwrap();
        for capture in fixture["captures"].as_array().unwrap(){
            let mut p=Challenge::new(UiScene::new(bank.program(42).unwrap(),&bytes,[320.,180.]).unwrap());let mut down=0;let mut checked=0;
            for r in capture["rows"].as_array().unwrap().iter().skip(1){
                let frame=r["row"].as_u64().unwrap();if frame>1{p.step(Input{back:down&0x20!=0,confirm:down&0x10!=0,..Default::default()});}
                if r["closed"]==true{assert_eq!(p.phase,Phase::Done);continue;}
                assert_eq!(p.phase.native(),r["phase"].as_u64().unwrap()as u32,"frame{frame}");assert_eq!(p.yes,r["yes"]==1,"frame{frame}");assert_eq!(p.result,r["result"]==1,"frame{frame}");
                assert_eq!(p.message_visible(),r["glyphCount"].as_u64().unwrap()>0,"frame{frame} message lifetime");
                let draws=p.scene.draw();
                for w in r["widgets"].as_array().unwrap(){
                    let i=p.scene.program.widgets.iter().position(|q|q.record_offset==w["offset"].as_u64().unwrap()as usize).unwrap();let d=&draws[i];
                    assert_eq!(d.visible,p.scene.program.widgets[i].flags&0x4001==0&&w["hidden"]==0&&w["rgba"][3].as_f64().unwrap()>0.,"frame{frame} index{i} visibility");
                    if p.scene.program.widgets[i].flags&0x4001!=0||w["hidden"]!=0||w["rgba"][3].as_f64().unwrap()<=0.||w["size"][0]==0||w["size"][1]==0{continue;}
                    assert!(d.visible,"frame{frame} id{}",d.id);
                    for axis in 0..2{
                        assert!((d.position[axis]-w["matrix"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0003,"frame{frame} index{i} widget{:?} position{axis} {:?} != {}",p.scene.program.widgets[i],d.position,w["matrix"]);
                        assert!((d.scale[axis]-w["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0003,"frame{frame} index{i} id{} flags{:x} size{axis} {:?} != {}",d.id,p.scene.program.widgets[i].flags,d.scale,w["size"]);
                    }
                    for axis in 0..4{assert!((d.color[axis]-w["rgba"][axis].as_f64().unwrap()as f32).abs()<0.00001,"frame{frame} id{} color{axis} {:?} != {}",d.id,d.color,w["rgba"]);}
                    assert_eq!(d.uv,[w["uv"][0].as_u64().unwrap()as u16,w["uv"][1].as_u64().unwrap()as u16]);checked+=1;
                }down=r["down"].as_u64().unwrap();
            }assert!(checked>500);
        }
    }
    #[test]fn stage_message_matches_all_48_native_glyphs(){
        let c=mw_data::ui_message::MessageCatalog::parse(&std::fs::read_to_string(root().join("assets/native/ui/challenge-c201-us/catalog.json")).unwrap()).unwrap();
        let fixture:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root().join("research/ui-challenge-observed.json")).unwrap()).unwrap();
        let r=fixture["captures"][0]["rows"].as_array().unwrap().iter().find(|r|r["phase"]==1).unwrap();
        let quads=crate::ui_message::centered_stage(&c,0,[320.,200.],[20.,20.],20.*20./24.).unwrap();assert_eq!(quads.len(),48);
        for(q,g)in quads.iter().zip(r["glyphs"].as_array().unwrap()){
            for axis in 0..2{assert!((q.center[axis]-g["matrix"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0003,"glyph{} center {:?} != {}",q.glyph,q.center,g["matrix"]);assert!((q.size[axis]-g["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.00001);}
            let glyph=c.glyph(q.glyph).unwrap();let page=&c.pages[glyph.page];let bounds=[glyph.rect[0]as f32/page.width as f32,glyph.rect[1]as f32/page.height as f32,glyph.rect[2]as f32/page.width as f32,glyph.rect[3]as f32/page.height as f32];
            let uv=g["uv"].as_array().unwrap();for axis in 0..2{let v:Vec<_>=uv.iter().skip(axis).step_by(2).map(|v|v.as_f64().unwrap()as f32).collect();assert!((v.iter().copied().fold(f32::INFINITY,f32::min)-bounds[axis]).abs()<0.00001);assert!((v.iter().copied().fold(f32::NEG_INFINITY,f32::max)-bounds[axis+2]).abs()<0.00001);}
        }
    }
}
