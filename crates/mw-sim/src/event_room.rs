//! Stage201 boss-entry ordering from80147BBC/80123820, after event playback.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EntryPhase{Event,Ending,Cleanup(u32),Loading,Fade(u32),Ready}
#[derive(Clone,Debug)]
pub struct BossEntry{pub phase:EntryPhase,pub skipped:bool,pub skip_allowed:bool}
impl BossEntry{
    pub fn new(event:u32)->Self{Self{phase:EntryPhase::Event,skipped:false,skip_allowed:![0x18,0x34,0x43,0x51,0x93].contains(&event)}}
    pub fn finish(&mut self,skip:bool)->bool{
        if self.phase!=EntryPhase::Event||(skip&&!self.skip_allowed){return false;}
        self.skipped=skip;self.phase=EntryPhase::Ending;true
    }
    /// One world update. Resource completion is an external condition, not a
    /// baked-in count from a particular native machine's loading latency.
    pub fn advance(&mut self,loaded:bool){
        self.phase=match self.phase{
            EntryPhase::Ending=>EntryPhase::Cleanup(13),
            EntryPhase::Cleanup(0)=>EntryPhase::Loading,
            EntryPhase::Cleanup(n)=>EntryPhase::Cleanup(n-1),
            EntryPhase::Loading if loaded=>EntryPhase::Fade(0),
            EntryPhase::Fade(14)=>EntryPhase::Ready,
            EntryPhase::Fade(n)=>EntryPhase::Fade(n+1),
            other=>other,
        };
    }
    pub fn fight_running(&self)->bool{matches!(self.phase,EntryPhase::Fade(_)|EntryPhase::Ready)}
    pub fn controls_blocked(&self)->bool{self.phase!=EntryPhase::Ready}
    pub fn fade(&self)->Option<[u8;4]>{match self.phase{
        EntryPhase::Event|EntryPhase::Ready=>None,
        EntryPhase::Fade(age)=>Some([0,0,0,((1.-age as f32/15.)*255.)as u8]),
        _=>Some([0,0,0,255]),
    }}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn source_cleanup_wait_loading_and_room_fade_match_native_edges(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for name in ["ev21-native-lifecycle","ev21-native-skip","ev21-skip-late-audio"]{
            let text=std::fs::read_to_string(root.join(format!("evidence/{name}.jsonl"))).unwrap();
            let rows:Vec<serde_json::Value>=text.lines().map(|l|serde_json::from_str::<serde_json::Value>(l).unwrap()).filter(|r|r.get("row").is_some()).collect();
            let end=rows.iter().position(|r|r["state"]==6).unwrap();let cleanup=end+1;
            let load=cleanup+14;let first=rows.iter().position(|r|!r["rinMoved"].is_null()).unwrap();
            assert!(first>load);assert_eq!(rows[cleanup]["state"],0);
            let mut entry=BossEntry::new(0x21);assert!(entry.finish(name.contains("skip")));
            assert_eq!(entry.fade(),Some([0,0,0,255]));assert!(!entry.fight_running());
            for i in cleanup..first+16{
                entry.advance(i>=first);
                if i==cleanup{assert_eq!(entry.phase,EntryPhase::Cleanup(13));}
                if i==load{assert_eq!(entry.phase,EntryPhase::Loading);}
                if i<first{assert!(!entry.fight_running());}
                if (first..first+15).contains(&i){
                    assert!(entry.fight_running()&&entry.controls_blocked());
                    assert_eq!(entry.fade().unwrap()[3]as u64,rows[i]["fade"]["rgba"].as_u64().unwrap());
                }
            }
            assert_eq!(entry.phase,EntryPhase::Ready);assert!(!entry.controls_blocked());
        }
        let mut blocked=BossEntry::new(0x18);assert!(!blocked.finish(true));assert_eq!(blocked.phase,EntryPhase::Event);
    }
}
