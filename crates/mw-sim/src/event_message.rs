//! 80113198 MESS command18 admission/clear and80098178's subtitle option gate.
//! Rendering/typewriter/autoclear remain separate from this command controller.
use mw_data::event_message::EventMessages;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Action{Show(usize),Clear}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub struct Cue{pub frame:u32,pub action:Action}
pub fn cues(bank:&EventMessages,message_count:usize)->Result<Vec<Cue>,String>{
    let mut result=vec![];
    if let Some(track)=&bank.track{for command in &track.commands{
        if command.opcode!=18{continue;}
        if command.operands.len()!=2{return Err("Short MESS command18".into());}
        let action=match command.operands[0]&3{
            0=>Action::Clear,
            1=>{let id=(command.operands[1]&0xffff)as usize;if id>=message_count{return Err("MESS message index outside catalog".into());}Action::Show(id)},
            _=>continue, // Original ignores modes2/3.
        };result.push(Cue{frame:command.frame,action});
    }}Ok(result)
}
pub struct Controller{cues:Vec<Cue>,next:usize,pub selected:Option<usize>,pub last_frame:Option<u32>}
impl Controller{
    pub fn new(cues:Vec<Cue>)->Self{Self{cues,next:0,selected:None,last_frame:None}}
    /// Call once per applied event frame, before incrementing its source clock.
    /// Off suppresses new show commands; enabling later does not replay them.
    pub fn advance(&mut self,frame:u32,enabled:bool)->Result<(),&'static str>{
        if self.last_frame.is_some_and(|old|frame<old){return Err("Backward MESS playback requires a fresh controller");}
        while let Some(cue)=self.cues.get(self.next).filter(|c|c.frame<=frame){
            match cue.action{Action::Show(id)if enabled=>self.selected=Some(id),Action::Clear=>self.selected=None,_=>{}}self.next+=1;
        }self.last_frame=Some(frame);Ok(())
    }
    pub fn visible(&self,enabled:bool)->Option<usize>{enabled.then_some(self.selected).flatten()}
    pub fn clear(&mut self){self.selected=None;}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn original_ev00_boundaries_and_option_admission(){
        let source=EventMessages::parse(include_str!("../../../assets/native/events/ev00/messages/messages.json")).unwrap();let cues=cues(&source,4).unwrap();
        assert_eq!(cues,vec![Cue{frame:20,action:Action::Show(0)},Cue{frame:170,action:Action::Show(1)},Cue{frame:293,action:Action::Show(2)},Cue{frame:355,action:Action::Show(3)},Cue{frame:395,action:Action::Clear}]);
        let mut p=Controller::new(cues.clone());for(frame,expected)in [(0,None),(19,None),(20,Some(0)),(169,Some(0)),(170,Some(1)),(292,Some(1)),(293,Some(2)),(354,Some(2)),(355,Some(3)),(394,Some(3)),(395,None)]{p.advance(frame,true).unwrap();assert_eq!(p.visible(true),expected);}
        let mut p=Controller::new(cues);p.advance(20,false).unwrap();p.advance(30,true).unwrap();assert_eq!(p.visible(true),None);p.advance(170,true).unwrap();assert_eq!(p.visible(true),Some(1));assert_eq!(p.visible(false),None);p.advance(395,false).unwrap();assert_eq!(p.visible(true),None);assert!(p.advance(0,true).is_err());
        let empty=EventMessages::parse(include_str!("../../../assets/native/events/ev21/messages/messages.json")).unwrap();assert!(super::cues(&empty,0).unwrap().is_empty());
    }
}
