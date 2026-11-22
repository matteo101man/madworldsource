//! Original event staging boundary. Commands remain data until their native
//! consumers have been translated; camera component semantics are not assumed.
use serde::Deserialize;
use crate::Clip;

#[derive(Clone,Debug,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct EventData {
    pub schema:String,pub event:u32,pub source_file:String,pub source_sha256:String,pub source_bytes:usize,
    pub cameras:Vec<EventCamera>,pub camera_commands:Vec<EventCommand>,pub actors:Vec<EventActor>,pub resources:Vec<EventResource>,
}
#[derive(Clone,Debug,Deserialize)]
pub struct EventRange {pub offset:usize,pub bytes:usize,pub sha256:String}
#[derive(Clone,Debug,Deserialize)]
pub struct EventCamera {pub frames:u32,pub flags:u16,pub samples:Vec<[f32;8]>,pub source:EventRange}
#[derive(Clone,Debug,Deserialize)]
pub struct EventActor {pub object:String,pub instance:u32,pub motions:Vec<EventMotion>,pub commands:Vec<EventCommand>}
#[derive(Clone,Debug,Deserialize)]
pub struct EventMotion {pub cut:usize,pub clip:Clip,pub source:EventRange}
#[derive(Clone,Debug,Deserialize)]
pub struct EventCommand {pub offset:usize,pub opcode:u32,pub size:usize,pub frame:u32,pub operands:Vec<u32>}
#[derive(Clone,Debug,Deserialize)]
pub struct EventResource {pub tag:String,pub source:EventRange,pub commands:Vec<EventCommand>}

impl EventData {
    pub fn parse(text:&str)->Result<Self,String>{
        let event:Self=serde_json::from_str(text).map_err(|e|e.to_string())?;
        if event.schema!="mw-native-event/1"||event.cameras.is_empty()||event.cameras.len()>64{return Err("Invalid native event header/cut count".into());}
        check_hash(&event.source_sha256)?;
        let mut total=0u32;
        for cut in &event.cameras{
            total=total.checked_add(cut.frames).ok_or("Event length overflow")?;
            if cut.frames==0||cut.samples.len()!=cut.frames as usize||cut.samples.iter().flatten().any(|v|!v.is_finite()) {return Err("Invalid event camera samples".into());}
            if cut.samples.iter().any(|s|s[7]<=0.||s[7]>=std::f32::consts::PI||s[..3]==s[3..6]){return Err("Invalid event camera projection/direction".into());}
            event.check_range(&cut.source)?;
        }
        let mut identities=std::collections::HashSet::new();
        for actor in &event.actors{
            if actor.object.is_empty()||!actor.object.bytes().all(|v|v.is_ascii_alphanumeric()||v==b'_')||!identities.insert((&actor.object,actor.instance)){return Err("Invalid/duplicate event actor identity".into());}
            let mut cuts=std::collections::HashSet::new();
            for motion in &actor.motions{
                let Some(camera)=event.cameras.get(motion.cut)else{return Err("Actor motion outside camera cuts".into());};
                // a clip may be shorter than its cut and holds its last frame (ev50 cut 25: the hand rings em0621 / em0622, 400 of 450)
                if !cuts.insert(motion.cut)||motion.clip.frames>camera.frames as usize||motion.clip.frames==0{return Err("Actor motion cut mismatch".into());}
                for track in &motion.clip.tracks{
                    if track.bone < -1||track.values.len()!=motion.clip.frames||track.values.iter().any(|v|!v.is_finite()){return Err("Invalid event motion samples".into());}
                }
                event.check_range(&motion.source)?;
            }
            event.check_commands(&actor.commands)?;
        }
        event.check_commands(&event.camera_commands)?;
        for resource in &event.resources{event.check_range(&resource.source)?;event.check_commands(&resource.commands)?;}
        Ok(event)
    }
    pub fn cut_lengths(&self)->Vec<u32>{self.cameras.iter().map(|c|c.frames).collect()}
    fn check_range(&self,range:&EventRange)->Result<(),String>{
        if range.bytes==0||range.offset.checked_add(range.bytes).filter(|&n|n<=self.source_bytes).is_none(){return Err("Event source range outside container".into());}
        check_hash(&range.sha256)
    }
    fn check_commands(&self,commands:&[EventCommand])->Result<(),String>{
        for command in commands{
            if command.size<12||command.size%4!=0||command.size.checked_sub(12).map(|n|n/4)!=Some(command.operands.len())||command.offset.checked_add(command.size).filter(|&n|n<=self.source_bytes).is_none(){return Err("Invalid event command framing".into());}
        }Ok(())
    }
}
fn check_hash(hash:&str)->Result<(),String>{if hash.len()==64&&hash.bytes().all(|b|b.is_ascii_hexdigit()){Ok(())}else{Err("Invalid event source hash".into())}}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn original_rin_event_has_four_cuts_and_preserves_missing_actor_cuts(){
        let text=include_str!("../../../assets/native/events/ev21/event.json");let event=EventData::parse(text).unwrap();
        assert_eq!(event.event,0x21);assert_eq!(event.cut_lengths(),[180,375,112,413]);assert_eq!(event.actors.len(),4);
        assert_eq!(event.actors.iter().map(|a|a.motions.len()).sum::<usize>(),12);
        for actor in &event.actors{
            let cuts:Vec<_>=actor.motions.iter().map(|m|m.cut).collect();
            assert_eq!(cuts,if actor.object=="em0440"{vec![1,2,3]}else{vec![0,1,3]});
        }
        assert!(event.actors.iter().any(|a|a.commands.iter().any(|c|c.opcode==14)));
        assert!(event.resources.iter().any(|r|r.tag=="VCE "));
        let value:serde_json::Value=serde_json::from_str(text).unwrap();
        for field in ["frames","cut","fov","direction"]{
            let mut broken=value.clone();match field{
                "frames"=>broken["cameras"][0][field]=0.into(),
                "cut"=>broken["actors"][0]["motions"][0][field]=64.into(),
                "fov"=>broken["cameras"][0]["samples"][0][7]=0.into(),
                _=>for axis in 0..3{broken["cameras"][0]["samples"][0][axis+3]=broken["cameras"][0]["samples"][0][axis].clone();},
            }
            assert!(EventData::parse(&broken.to_string()).is_err());
        }
    }
}
