//! Optional original MESS bank boundary; absence is meaningful (EV21 has none).
use serde::Deserialize;
use crate::event::{EventCommand,EventRange};
#[derive(Clone,Debug,Deserialize)]#[serde(rename_all="camelCase")]
pub struct EventMessages{pub schema:String,pub event:u32,pub source_file:String,pub source_sha256:String,pub source_bytes:usize,pub track:Option<MessageTrack>}
#[derive(Clone,Debug,Deserialize)]#[serde(rename_all="camelCase")]
pub struct MessageTrack{pub catalog:String,pub source:EventRange,pub commands:Vec<EventCommand>,pub line_height:u32}
fn safe_relative(path:&str)->bool{!path.contains(['\\',':'])&&!path.split('/').any(|p|p.is_empty()||p=="."||p=="..")}
impl EventMessages{
    pub fn parse(text:&str)->Result<Self,String>{
        let bank:Self=serde_json::from_str(text).map_err(|e|e.to_string())?;
        let hash=|s:&str|s.len()==64&&s.bytes().all(|b|b.is_ascii_hexdigit());
        if bank.schema!="mw-event-messages/1"||!hash(&bank.source_sha256)||bank.source_bytes<8||!bank.source_file.starts_with("event/")||!safe_relative(&bank.source_file){return Err("Event message source".into());}
        if let Some(track)=&bank.track{
            let start=track.source.offset;let end=start.checked_add(track.source.bytes).filter(|&end|end<=bank.source_bytes).ok_or("Message track bounds")?;
            if track.source.bytes<8||!hash(&track.source.sha256)||!track.catalog.starts_with("native/events/")||!safe_relative(&track.catalog)||track.line_height==0||track.line_height>256{return Err("Message track metadata".into());}
            let mut previous=0;
            for command in &track.commands{
                if command.size<12||command.size%4!=0||(command.size-12)/4!=command.operands.len()||command.offset<start||command.offset.checked_add(command.size).filter(|&n|n<=end).is_none()||command.frame<previous{return Err("Message command framing/order".into());}previous=command.frame;
            }
        }Ok(bank)
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn original_absence_and_positive_message_tracks_are_distinct(){
        let absent=EventMessages::parse(include_str!("../../../assets/native/events/ev21/messages/messages.json")).unwrap();assert_eq!(absent.event,0x21);assert!(absent.track.is_none());
        let text=include_str!("../../../assets/native/events/ev00/messages/messages.json");let present=EventMessages::parse(text).unwrap();let track=present.track.unwrap();assert_eq!(track.commands.len(),5);
        let catalog=crate::ui_message::MessageCatalog::parse(include_str!("../../../assets/native/events/ev00/messages/catalog.json")).unwrap();assert_eq!((catalog.glyphs.len(),catalog.messages.len()),(35,4));
        let mut value:serde_json::Value=serde_json::from_str(text).unwrap();value["track"]["catalog"]="../catalog.json".into();assert!(EventMessages::parse(&value.to_string()).is_err());
        value=serde_json::from_str(text).unwrap();value["track"]["commands"][0]["offset"]=0.into();assert!(EventMessages::parse(&value.to_string()).is_err());
    }
}
