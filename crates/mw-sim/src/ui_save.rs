//! cIdOptLoadGame navigation and native record presentation.
use crate::ui::{SaveSelection,TitlePrompt,UiScene,initialize_empty_slot};
use mw_data::save::SaveSlot;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Page{Slots,Actions,Confirm}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Operation{Copy{from:usize,to:usize},Erase{slot:usize}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Intent{Start{slot:usize},Write(Operation)}
#[derive(Clone,Copy,Debug,Default)]pub struct Input{pub vertical:i32,pub horizontal:i32,pub confirm:bool,pub back:bool}
#[derive(Clone,Debug)]pub struct Menu{pub selection:SaveSelection,pub page:Page,pub action:usize,pub copy_from:Option<usize>,pub operation:Option<Operation>,pub prompt:Option<TitlePrompt>}
impl Default for Menu{fn default()->Self{Self{selection:SaveSelection::default(),page:Page::Slots,action:0,copy_from:None,operation:None,prompt:None}}}
impl Menu{
    pub fn advance(&mut self,frames:f32,input:Input,occupied:[bool;4])->Option<Intent>{
        self.selection.advance(frames);
        if self.page==Page::Confirm{
            let prompt=self.prompt.as_mut().unwrap();
            if let Some(outcome)=prompt.advance(frames){
                // This dialog closes after saving too; Title's Yes instead
                // hands directly to a scene change. Keep its selected answer
                // through the shared15-frame close phase.
                if outcome{prompt.phase=crate::ui::TitlePhase::Closing;prompt.age=0.;return None;}
                let yes=prompt.yes;self.page=Page::Slots;self.selection.age=0.;self.prompt=None;self.operation=None;if yes{self.copy_from=None;}return None;
            }
            if input.back{prompt.back();}else{
                if input.horizontal!=0{prompt.choose(input.horizontal<0);}
                if input.confirm&&prompt.phase==crate::ui::TitlePhase::Selecting{let operation=self.operation.unwrap();let yes=prompt.yes;prompt.confirm();if yes{return Some(Intent::Write(operation));}}
            }return None;
        }
        if self.page==Page::Slots{
            if input.back{if let Some(from)=self.copy_from.take(){self.selection.selected=from;self.selection.age=0.;}return None;}
            self.selection.step(input.vertical);
            if input.confirm&&self.selection.age>=30.{
                if let Some(from)=self.copy_from{if from!=self.selection.selected{self.ask(Operation::Copy{from,to:self.selection.selected});}}
                else{self.page=Page::Actions;self.action=0;self.selection.age=0.;}
            }
        }else if self.selection.age>=26.{
            if input.back{self.page=Page::Slots;self.selection.age=0.;return None;}
            let count=if occupied[self.selection.selected]{3}else{1};self.action=(self.action as i32+input.vertical).rem_euclid(count)as usize;
            if input.confirm{match self.action{
                0=>return Some(Intent::Start{slot:self.selection.selected}),
                1=>{self.copy_from=Some(self.selection.selected);self.page=Page::Slots;self.selection.age=0.;},
                2=>self.ask(Operation::Erase{slot:self.selection.selected}),_=>unreachable!(),
            }}
        }None
    }
    fn ask(&mut self,operation:Operation){self.operation=Some(operation);self.prompt=Some(TitlePrompt::default());self.page=Page::Confirm;}
}
pub fn initialize_slot(scene:&mut UiScene,frame:&UiScene,index:usize,record:SaveSlot<'_>){
    initialize_empty_slot(scene,frame,index);if !record.occupied(){return;}
    for id in [14,13,11]{scene.hide_tree(id,false);}scene.hide_tree(16,true);
    scene.number(20,9,record.score().min(999999999));scene.number(35,3,(record.stage()as u32).min(999));
    // Native8023DEE8 doubles the30Hz play-time counter, then displays hours
    // and minutes. Hour digit34 retains its source hidden flag.
    let ticks=(record.play_time()*2.)as u32;let hours=ticks/216000;let minutes=(ticks%216000)/3600;
    for (id,value)in [(30,minutes%10),(31,minutes/10),(32,hours%10),(33,hours/10%10),(34,hours/100%10)]{scene.digit(id,value as u16);}
    scene.digit(11,record.challenge_mark());scene.hide_tree(10,!record.clear_mark());scene.hide_tree(20,record.special_mark().is_none());if let Some(mark)=record.special_mark(){scene.digit(20,mark);}
}
pub fn initialize_actions(scene:&mut UiScene,occupied:bool){crate::ui::initialize_empty_slot_actions(scene);for id in [12,13]{scene.hide_tree(id,!occupied);}}
pub fn update_actions(scene:&mut UiScene,action:usize){scene.bind_position_axis(5,11+action as u32,1);}
pub fn initialize_confirmation(scene:&mut UiScene,operation:Operation,occupied:[bool;4]){
    let (top,bottom)=match operation{Operation::Erase{..}=>(8,2),Operation::Copy{to,..}=>(6,if occupied[to]{12}else{6})};scene.digit(20,top);scene.digit(21,bottom);scene.bind_position_axis(9,12,2);
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn occupied_row_and_three_actions_match_original_widgets(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/idpac_ce00_us/animation.idt")).unwrap();let timeline=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let mut frame=UiScene::new(timeline.program(80).unwrap(),&bytes,[320.,180.]).unwrap();crate::ui::initialize_save_frame(&mut frame);
        let reset=include_bytes!("../../../assets/native/save/reset-record.bin");let mut data=mw_data::save::SaveBundle::empty(reset).unwrap();data.start_new_game(0,reset,1,false).unwrap();let mut checked=0;
        for (program,file)in [(81,"ui-save-occupied-rows-observed.json"),(83,"ui-save-occupied-actions-observed.json"),(82,"ui-save-delete-confirm-observed.json"),(83,"ui-save-delete-actions-observed.json")]{
            let mut scene=UiScene::new(timeline.program(program).unwrap(),&bytes,[320.,180.]).unwrap();if program==81{initialize_slot(&mut scene,&frame,0,data.records.slot(0).unwrap());scene.color_preset(1,0);}else if program==82{initialize_confirmation(&mut scene,Operation::Erase{slot:0},[true,false,false,false]);crate::ui::update_title_prompt(&mut scene,&TitlePrompt{yes:false,phase:crate::ui::TitlePhase::Selecting,age:0.});}else{initialize_actions(&mut scene,true);if file.contains("delete"){scene.bind_position_axis(5,13,1);scene.hide_tree(6,false);scene.bind_position_axis(6,13,1);scene.select_tree(13,true);}}
            scene.seek(120.);let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research").join(file)).unwrap()).unwrap();
            let bindings:Vec<_>=observed["bindings"].as_array().unwrap().iter().filter(|b|program!=81||b["observed"]["cluster"]==1).collect();
            for b in &bindings{let i=b["index"].as_u64().unwrap()as usize;let o=&b["observed"];let mut at=o["channelFrames"]["position"].as_f64().unwrap()as f32;let source=scene.states[i].external_position.as_ref().unwrap_or(&scene.program.widgets[scene.states[i].position_source]);if let Some(track)=source.position_track(&bytes).unwrap(){let end=track.keys.last().unwrap().frame as f32;if at>0.&&(track.flags&4!=0||at<end){at-=1.;}}scene.states[i].clocks.position=at;let rgb_source=scene.states[i].rgb_binding.unwrap_or(i);if let Some((flags,keys))=scene.program.widgets[rgb_source].appearance(&bytes).unwrap().rgb{let mut rgb=o["channelFrames"]["rgb"].as_f64().unwrap()as f32;if rgb>0.&&(flags&4!=0||rgb<keys.last().unwrap().1){rgb-=1.;}scene.states[i].clocks.rgb=rgb;}}
            let draws=scene.draw();for b in bindings{let i=b["index"].as_u64().unwrap()as usize;let o=&b["observed"];let d=&draws[i];if scene.program.widgets[i].flags&0x4001!=0{continue;}
                if o["hidden"]==1{assert!(!d.visible,"program{program} widget{i} hidden");continue;}assert!(d.visible,"program{program} widget{i} visible");
                for axis in 0..2{assert!((d.position[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0002,"program{program} widget{i} position{axis}");assert!((d.scale[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.0002,"program{program} widget{i} size{axis}");}
                for channel in 0..4{assert!((d.color[channel]-o["rgba"][channel].as_f64().unwrap()as f32).abs()<0.0002,"program{program} widget{i} color{channel}");}if let Some(digit)=scene.states[i].uv_frame{assert_eq!(digit as u64,o["uvFrame"].as_u64().unwrap(),"program{program} widget{i} digit");}checked+=1;
            }
        }assert!(checked>=20);
    }
    #[test]fn copy_requires_a_different_destination_and_explicit_yes_and_emits_once(){
        let mut menu=Menu::default();let occupied=[true,false,false,false];
        menu.advance(30.,Input{confirm:true,..Input::default()},occupied);assert_eq!(menu.page,Page::Actions);
        menu.advance(26.,Input{vertical:1,confirm:true,..Input::default()},occupied);assert_eq!(menu.copy_from,Some(0));
        menu.advance(30.,Input{confirm:true,..Input::default()},occupied);assert_eq!(menu.page,Page::Slots);
        menu.advance(0.,Input{vertical:1,confirm:true,..Input::default()},occupied);assert_eq!(menu.page,Page::Confirm);
        assert_eq!(menu.advance(0.,Input{horizontal:-1,confirm:true,..Input::default()},occupied),None);
        assert_eq!(menu.advance(26.,Input{horizontal:-1,confirm:true,..Input::default()},occupied),Some(Intent::Write(Operation::Copy{from:0,to:1})));
        assert_eq!(menu.advance(1.,Input{confirm:true,..Input::default()},occupied),None);menu.advance(19.,Input::default(),occupied);assert_eq!(menu.page,Page::Confirm);menu.advance(15.,Input::default(),occupied);assert_eq!(menu.page,Page::Slots);assert_eq!(menu.copy_from,None);
    }
    #[test]fn empty_slot_has_only_start_back_cancels_copy_and_no_never_writes(){
        let mut menu=Menu::default();menu.advance(30.,Input{confirm:true,..Input::default()},[false;4]);assert_eq!(menu.advance(26.,Input{vertical:1,confirm:true,..Input::default()},[false;4]),Some(Intent::Start{slot:0}));assert_eq!(menu.action,0);
        menu.copy_from=Some(0);menu.page=Page::Slots;menu.selection.selected=3;menu.advance(30.,Input{back:true,..Input::default()},[true;4]);assert_eq!(menu.selection.selected,0);assert_eq!(menu.copy_from,None);
        menu.ask(Operation::Erase{slot:0});menu.advance(26.,Input{horizontal:-1,..Input::default()},[true;4]);menu.advance(0.,Input{back:true,confirm:true,..Input::default()},[true;4]);assert!(!menu.prompt.as_ref().unwrap().yes);assert_eq!(menu.advance(0.,Input{confirm:true,..Input::default()},[true;4]),None);menu.advance(20.,Input::default(),[true;4]);menu.advance(15.,Input::default(),[true;4]);assert_eq!(menu.page,Page::Slots);
    }
}
