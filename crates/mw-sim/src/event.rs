//! Native event clock field effects, independent of rendering and wall time.
//! Original 0x800a6f60 (advance), 0x800a7034 (seek cut) and 0x800a6fc0
//! (seek global), plus ev21 camera application and its observed NTSC cadence.
use mw_data::{event::EventData,Clip};

/// Native NTSC VI is60000/1001 fields/s; ev21 uses two fields per update.
/// Confirmed by the complete MW_EVENT_RECORD capture and native vi.cpp.
pub const EVENT_HZ:f64=30000./1001.;

#[derive(Clone,Copy,Debug)]
pub struct EventCameraPose {pub eye:[f32;3],pub target:[f32;3],pub up:[f32;3],pub fov_radians:f32}
impl EventCameraPose{
    /// 800838F4/80083988/80082D58/8008229C. Roll rotates world-up in the
    /// camera's horizontal-back yaw frame (native ROT_ORDER2), before look-at.
    pub fn from_sample(sample:[f32;8])->Self{
        let yaw=(sample[0]-sample[3]).atan2(sample[2]-sample[5]);
        let (s,c)=sample[6].sin_cos();let(sy,cy)=yaw.sin_cos();
        Self{eye:[sample[0],sample[1],sample[2]],target:[sample[3],sample[4],sample[5]],up:[-cy*s,c,sy*s],fov_radians:sample[7]}
    }
}

/// Source cut/local clock and camera/actor selection. SEQ effects, physical
/// attachments and scene lifecycle remain separate consumers.
#[derive(Clone,Debug)]
pub struct EventPlayer {pub clock:EventClock}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct EventAttachment {pub target:String,pub bone:u32}
#[derive(Clone,Debug)]
pub struct EventMeshRule {pub pattern:String,pub cancelled:bool,pub exact:bool}
#[derive(Clone,Debug,Default)]
pub struct EventActorState {pub attachment:Option<EventAttachment>,pub mesh_rules:Vec<EventMeshRule>}
impl EventActorState{
    pub fn mesh_cancelled(&self,name:&str)->bool{
        self.mesh_rules.iter().rev().find(|r|if r.exact{name==r.pattern}else{name.contains(&r.pattern)}).is_some_and(|r|r.cancelled)
    }
}

/// 8011E120: command14 attaches/detaches by event actor name and native bone
/// ID. Command15 cancels substring-matched meshes; command10's Jack mode uses
/// exact-name cancellation (80099558/802072E4). Other command10 effects are
/// render-state consumers, not actor visibility.
pub fn actor_state(event:&EventData,object:&str,instance:u32,frame:u32)->Result<EventActorState,&'static str>{
    let actor=event.actors.iter().find(|a|a.object==object&&a.instance==instance).ok_or("Missing event actor")?;
    let mut state=EventActorState::default();
    for command in actor.commands.iter().filter(|c|c.frame<=frame){
        let args=&command.operands;
        match command.opcode{
            14=>{
                let flags=*args.first().ok_or("Short attachment command")?;
                if flags&1==0{state.attachment=None;}else{
                    if args.len()<5{return Err("Short attachment command");}
                    let target=command_string(&args[1..4])?;
                    if !target.is_empty(){state.attachment=Some(EventAttachment{target,bone:args[4]});}
                }
            },
            15=>{
                let mode=*args.first().ok_or("Short mesh command")?;
                if mode>1{continue;}
                let pattern=command_string(&args[1..])?;
                if !pattern.is_empty(){state.mesh_rules.push(EventMeshRule{pattern,cancelled:mode==1,exact:false});}
            },
            10 if object=="pl0030"=>{
                let mode=(*args.first().ok_or("Short actor mode command")?>>1)&3;
                if mode==1||mode==2{
                    for(name,alternative)in [("goggles_KB",false),("goggles_belt_SB",false),("goggles_belt_R_SB",false),("goggles2_KB",true),("goggles2_belt_SB",true),("goggles2_belt_R_SB",true)]{
                        state.mesh_rules.push(EventMeshRule{pattern:name.into(),cancelled:if alternative{mode==1}else{mode==2},exact:true});
                    }
                }
            },_=>{},
        }
    }Ok(state)
}
fn command_string(words:&[u32])->Result<String,&'static str>{
    let bytes:Vec<u8>=words.iter().flat_map(|w|w.to_be_bytes()).collect();
    let end=bytes.iter().position(|&b|b==0).ok_or("Unterminated event command string")?;
    std::str::from_utf8(&bytes[..end]).map(str::to_owned).map_err(|_|"Invalid event command string")
}

/// Camera SEQ1, 800A7930/800A73A4: hold when bit0 is set, otherwise
/// interpolate normalized RGBA floats to the next fade key, then truncate.
pub fn camera_fade(event:&EventData,frame:u32)->Result<Option<[u8;4]>,&'static str>{
    let Some(key)=event.camera_commands.iter().filter(|c|c.opcode==1&&c.frame<=frame).max_by_key(|c|c.frame)else{return Ok(None);};
    let flags=*key.operands.first().ok_or("Short event fade command")?;
    let from=key.operands.get(1).ok_or("Short event fade command")?.to_be_bytes();
    if frame==key.frame||flags&1!=0{return Ok(Some(from));}
    let Some(next)=event.camera_commands.iter().filter(|c|c.opcode==1&&c.frame>key.frame).min_by_key(|c|c.frame)else{return Ok(Some(from));};
    let to=next.operands.get(1).ok_or("Short event fade command")?.to_be_bytes();
    let fraction=(frame-key.frame)as f32/(next.frame-key.frame)as f32;
    Ok(Some([0,1,2,3].map(|i|{
        let a=from[i]as f32/255.;let b=to[i]as f32/255.;
        ((a+(b-a)*fraction)*255.)as u8
    })))
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum StreamBus{Music,Voice}
#[derive(Clone,Debug,PartialEq,Eq)]pub enum StreamAction{
    Play{name:String,volume:Option<u8>,fade_out:u32,fade_in:u32,persist:bool},
    Stop{fade:u32},Preload{name:String},
}
#[derive(Clone,Debug,PartialEq,Eq)]pub struct StreamCue{pub frame:u32,pub bus:StreamBus,pub action:StreamAction}
/// Event BGM/VCE task commands (80073DA8/80189F04). A transition uses
/// the music master; its command volume is not a playback volume.
pub fn stream_cues(event:&EventData)->Result<Vec<StreamCue>,&'static str>{
    let mut cues=vec![];
    for resource in &event.resources{
        let(bus,opcode)=match resource.tag.as_str(){"BGM "=>(StreamBus::Music,8),"VCE "=>(StreamBus::Voice,9),_=>continue};
        for command in resource.commands.iter().filter(|c|c.opcode==opcode){
            let a=&command.operands;let flags=*a.first().ok_or("Short event stream command")?;let mode=flags&3;
            let action=if mode==0{
                StreamAction::Stop{fade:*a.get(if bus==StreamBus::Music{15}else{13}).ok_or("Short event stream stop")?}
            }else{
                if a.len()<13{return Err("Short event stream name");}let name=command_string(&a[1..13])?;
                if bus==StreamBus::Music&&mode==3||bus==StreamBus::Voice&&mode==2{StreamAction::Preload{name}}
                else if bus==StreamBus::Music||mode==1{
                    let(volume,fade_out,fade_in)=if bus==StreamBus::Music{
                        if a.len()<16{return Err("Short event music play");}
                        let volume=if mode==2{None}else{let v=if a[13]==u32::MAX{100}else{a[13]};if v>127{return Err("Invalid event music volume");}Some(v as u8)};
                        (volume,if mode==2{a[15]}else{0},if mode==2{a[14]}else{0})
                    }else{(None,0,0)};
                    StreamAction::Play{name,volume,fade_out,fade_in,persist:bus==StreamBus::Music&&flags&0x10!=0}
                }else{continue}
            };
            cues.push(StreamCue{frame:command.frame,bus,action});
        }
    }cues.sort_by_key(|c|c.frame);Ok(cues)
}
/// Stream mixer 80501E64/80502320: native master, RedSpark stream header
/// volume and per-channel volume, with its integer truncation before AX gain.
pub fn stream_gain(volume:u8,header:u8,channel:u8)->Result<f32,&'static str>{
    if [volume,header,channel].iter().any(|&v|v>127){return Err("Invalid native stream mix");}
    let master=if volume==0{0}else{(volume as u32+1)*256-1};
    let channel=((0x80800*(channel as u32+1))>>7)>>12;
    let scalar=(((header as u32+1)*master)>>7)*channel>>7;
    Ok(scalar as f32/32768.)
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct EventUiCue{pub frame:u32,pub program:u16,pub layer:u32,pub bank:u8,pub create:bool}
impl EventUiCue{
    /// ID animations advance once per event update, starting at source time0.
    pub fn age(&self,frame:u32)->Option<u32>{frame.checked_sub(self.frame)}
}
/// 800DA154 ->8023235C: ID command5 creates or restarts a program instance.
/// Command16 belongs to the separate writing-character consumer.
pub fn event_ui_cues(event:&EventData)->Result<Vec<EventUiCue>,&'static str>{
    let mut cues=vec![];
    for c in event.resources.iter().filter(|r|r.tag=="ID  ").flat_map(|r|&r.commands).filter(|c|c.opcode==5){
        let a=&c.operands;if a.len()<3{return Err("Short event UI command");}
        cues.push(EventUiCue{frame:c.frame,program:a[1]as u16,layer:a[2],bank:(a[0]&15)as u8,create:a[0]&16!=0});
    }cues.sort_by_key(|c|c.frame);Ok(cues)
}

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct EventWritingCue{pub frame:u32,pub id:u32,pub position:[f32;3]}
/// 800DA154 command16's mode0 passes operands6..8 to802E2BBC Print2D.
pub fn event_writing_cues(event:&EventData)->Result<Vec<EventWritingCue>,&'static str>{
    let mut cues=vec![];
    for c in event.resources.iter().filter(|r|r.tag=="ID  ").flat_map(|r|&r.commands).filter(|c|c.opcode==16){
        let a=&c.operands;if a.len()<9{return Err("Short event writing command");}
        if a[0]&3!=0{return Err("Untranslated projected/actor-linked writing mode");}
        let position=[0,1,2].map(|i|f32::from_bits(a[6+i]));if position.iter().any(|v|!v.is_finite()){return Err("Invalid writing position");}
        cues.push(EventWritingCue{frame:c.frame,id:a[1],position});
    }cues.sort_by_key(|c|c.frame);Ok(cues)
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct EventSeCue{pub frame:u32,pub id:u32,pub attributes:u32,pub options:u8,pub volume:u8,pub slot:Option<u8>}
/// 80189530's numeric, non-spatial SE13 path through80268DBC. Other SE13
/// modes are explicit missing consumers; actor-name fields are unused in mode1.
pub fn event_se_cues(event:&EventData)->Result<Vec<EventSeCue>,&'static str>{
    let mut cues=vec![];
    for c in event.resources.iter().filter(|r|r.tag=="SE  ").flat_map(|r|&r.commands).filter(|c|c.opcode==13){
        let a=&c.operands;if a.len()<24{return Err("Short event SE command");}
        // mode 0 looks its sound up by name (ev34's eSeVoBooing): untranslated, left silent
        if a[0]&7==0{continue;}
        if a[0]&7!=5{return Err("Untranslated event SE mode/name lookup");}
        let volume=f32::from_bits(a[21]);if !volume.is_finite()||!(0. ..=127.).contains(&volume){return Err("Invalid event SE volume");}
        cues.push(EventSeCue{frame:c.frame,id:a[1],attributes:a[16],options:a[17]as u8,volume:volume as u8,slot:if a[23]<64{Some(a[23]as u8)}else{None}});
    }cues.sort_by_key(|c|c.frame);Ok(cues)
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct EventSeProgram{pub wave:u8,pub volume:u8,pub note:u8,pub hold_ticks:u16,pub effect_sends:bool}
impl EventSeProgram{
    /// Exact linear program used by both EV21 effects, with native handler
    /// addresses in EVENT-API.md. Reject other programs instead of skipping bytes.
    pub fn parse(code:&[u8])->Result<Self,&'static str>{
        if code.len()!=11||code[0]!=0x20||code[2]!=0x22||code[4]!=0x60||code[5]!=0x12||code[7]!=0x15||code[10]!=0{return Err("Untranslated event SE program");}
        if code[3]>127||code[6]>127{return Err("Invalid event SE volume/note");}
        Ok(Self{wave:code[1],volume:code[3],note:code[6],hold_ticks:u16::from_be_bytes([code[8],code[9]]),effect_sends:true})
    }
    /// 804FDA28: phase +=416, -=120 per96-sample/32kHz callback. This is
    /// mean sequencer time; callback/key-off/envelope quantization is separate.
    pub fn hold_seconds(&self)->f64{self.hold_ticks as f64*416./120.*96./32000.}
}
impl EventPlayer{
    pub fn new(event:&EventData)->Result<Self,&'static str>{Ok(Self{clock:EventClock::new(&event.cut_lengths())?})}
    pub fn camera(&self,event:&EventData)->EventCameraPose{
        EventCameraPose::from_sample(event.cameras[self.clock.cut()].samples[self.clock.local_frame()as usize])
    }
    /// 8011DB08: an absent actor cut is hidden, rather than holding its last clip.
    pub fn actor_motion<'a>(&self,event:&'a EventData,object:&str,instance:u32)->Option<&'a Clip>{
        event.actors.iter().find(|a|a.object==object&&a.instance==instance)?.motions.iter().find(|m|m.cut==self.clock.cut()).map(|m|&m.clip)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct EventClock {
    lengths: Vec<u32>,
    total: u32,
    cut: usize,
    local: u32,
    global: u32,
}

impl EventClock {
    /// Validated staging boundary; the native length table has 64 slots.
    pub fn new(lengths: &[u32]) -> Result<Self, &'static str> {
        if lengths.is_empty() || lengths.len()>64 || lengths.contains(&0) { return Err("invalid native event cut lengths"); }
        let total=lengths.iter().try_fold(0u32,|n,&v|n.checked_add(v)).ok_or("event length overflow")?;
        Ok(Self {lengths:lengths.to_vec(),total,cut:0,local:0,global:0})
    }
    pub fn frame(&self) -> u32 { self.global }
    pub fn local_frame(&self) -> u32 { self.local }
    pub fn cut(&self) -> usize { self.cut }
    pub fn total_frames(&self) -> u32 { self.total }

    /// Returns false on the final frame and leaves it unchanged, as the native
    /// caller uses this result to finish its event. There is no automatic loop.
    pub fn advance(&mut self) -> bool {
        if self.global>=self.total-1 { return false; }
        self.global+=1; self.local+=1;
        if self.local>=self.lengths[self.cut] { self.local=0; self.cut+=1; }
        true
    }

    pub fn seek_cut(&mut self, cut: usize, local: u32) -> bool {
        let Some(&length)=self.lengths.get(cut) else { return false; };
        if local>=length { return false; }
        self.cut=cut; self.local=local;
        self.global=self.lengths[..cut].iter().sum::<u32>()+local;
        true
    }

    /// Native global seek reports false for an unchanged or out-of-range frame.
    pub fn seek_global(&mut self, frame: u32) -> bool {
        if frame>=self.total || frame==self.global { return false; }
        let mut local=frame;
        for (cut,&length) in self.lengths.iter().enumerate() {
            if local<length { self.cut=cut; self.local=local; self.global=frame; return true; }
            local-=length;
        }
        unreachable!("validated total and positive lengths")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn original_event_ui_commands_select_the_three_comic_programs(){
        let event=EventData::parse(include_str!("../../../assets/native/events/ev21/event.json")).unwrap();
        let cues=event_ui_cues(&event).unwrap();
        assert_eq!(cues,[(260,1),(350,2),(392,3)].map(|(frame,program)|EventUiCue{frame,program,layer:10,bank:4,create:true}));
        assert_eq!(cues[0].age(259),None);assert_eq!(cues[0].age(260),Some(0));assert_eq!(cues[0].age(275),Some(15));
        let mut short=event.clone();short.resources.iter_mut().find(|r|r.tag=="ID  ").unwrap().commands.iter_mut().find(|c|c.opcode==5).unwrap().operands.clear();
        assert!(event_ui_cues(&short).is_err());
    }
    #[test]fn original_event_ui_matches_native_widget_geometry_visibility_and_lifetime(){
        use crate::ui::UiScene;
        let event=EventData::parse(include_str!("../../../assets/native/events/ev21/event.json")).unwrap();
        let cues=event_ui_cues(&event).unwrap();let bytes=include_bytes!("../../../assets/native/events/ev21/ui/animation.idt");
        let bank=mw_data::ui::UiTimeline::parse(bytes).unwrap();
        let widgets:std::collections::HashMap<_,_>=bank.programs.iter().flat_map(|p|p.widgets.iter().enumerate().map(move |(i,w)|(w.record_offset,(p.id,i)))).collect();
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let rows=std::fs::read_to_string(root.join("evidence/ev21-native-ui.jsonl")).unwrap();
        let(mut count,mut observed,mut visible)=(0,0,0);
        let mut ranges:std::collections::HashMap<u32,(u32,u32)>=Default::default();
        for line in rows.lines(){
            let row:serde_json::Value=serde_json::from_str(line).unwrap();let Some(frame)=row["before"].as_u64().map(|n|n as u32)else{continue;};count+=1;
            // 800A6F60 is a pre-update clock edge. Its UI is the pose
            // applied by update N-1; rendering update N uses age(N).
            let scenes:std::collections::HashMap<_,_>=cues.iter().filter_map(|cue|{
                let age=cue.age(frame.checked_sub(1)?)?;
                let mut scene=UiScene::new(bank.program(cue.program as u32).unwrap(),bytes,[320.,180.]).unwrap();scene.seek(age as f32);
                Some((cue.program as u32,scene.draw()))
            }).collect();
            for native in row["eventUi"].as_array().unwrap(){
                let &(id,index)=widgets.get(&(native["offset"].as_u64().unwrap()as usize)).unwrap();
                let range=ranges.entry(id).or_insert((frame,frame));range.1=frame;
                let shown=native["hidden"]==0&&native["rgba"][3].as_f64().unwrap()>0.;
                if id==100{assert!(!shown,"owner scaffolding became visible");continue;}
                let draw=&scenes[&id][index];observed+=1;
                assert_eq!(draw.visible,shown,"frame{frame}/program{id}/widget{index}");assert_eq!(serde_json::json!(draw.uv),native["uv"]);
                if !shown{continue;}visible+=1;
                let f=|v:&serde_json::Value|v.as_f64().unwrap()as f32;
                for axis in 0..2{
                    assert!((draw.position[axis]-f(&native["matrix"][if axis==0{3}else{7}])).abs()<0.0001);
                    assert!((draw.scale[axis]-f(&native["size"][axis])*if axis==0{1.}else{-1.}).abs()<0.0001);
                }
                for channel in 0..4{assert!((draw.color[channel]-f(&native["rgba"][channel])).abs()<0.0001);}
                let angle=(draw.rotation-f(&native["matrix"][4]).atan2(f(&native["matrix"][0]))+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
                assert!(angle.abs()<0.0001);
                assert_eq!((draw.kind,draw.blend_mode),(0,0));assert!(draw.tile_kind<=1);
            }
        }
        assert_eq!((count,observed,visible),(1080,14229,400));
        for cue in cues{assert_eq!(ranges[&(cue.program as u32)],(cue.frame+1,1079));}
        assert_eq!(ranges[&100],(0,1079));
    }
    #[test]fn event_se_numeric_mode_and_linear_program_are_source_derived(){
        let event=EventData::parse(include_str!("../../../assets/native/events/ev21/event.json")).unwrap();let cues=event_se_cues(&event).unwrap();
        assert_eq!(cues,vec![EventSeCue{frame:180,id:7522001,attributes:1,options:8,volume:127,slot:Some(0)},EventSeCue{frame:866,id:7522006,attributes:1,options:8,volume:127,slot:Some(0)}]);
        let code=[0x20,1,0x22,127,0x60,0x12,60,0x15,0x05,0xdc,0];let p=EventSeProgram::parse(&code).unwrap();
        assert_eq!((p.wave,p.note,p.volume,p.hold_ticks,p.effect_sends),(1,60,127,1500,true));assert!((p.hold_seconds()-15.6).abs()<1e-9);
        let mut bad=code;bad[5]=0x23;assert!(EventSeProgram::parse(&bad).is_err());assert!(EventSeProgram::parse(&code[..10]).is_err());
    }
    #[test]fn original_event_stream_commands_and_integer_mixer(){
        let event=EventData::parse(include_str!("../../../assets/native/events/ev21/event.json")).unwrap();
        let cues=stream_cues(&event).unwrap();assert_eq!(cues.len(),3);
        assert!(cues.iter().any(|c|c.frame==0&&c.bus==StreamBus::Music&&c.action==StreamAction::Play{name:"EV21_RSD".into(),volume:None,fade_out:60,fade_in:0,persist:false}));
        assert!(cues.iter().any(|c|c.frame==0&&c.bus==StreamBus::Voice&&matches!(&c.action,StreamAction::Play{name,..}if name=="JKSNN_EVVOSTR_EV21")));
        assert!(cues.iter().any(|c|c.frame==748&&c.bus==StreamBus::Music&&c.action==StreamAction::Play{name:"BS04C_RSD".into(),volume:None,fade_out:60,fade_in:0,persist:true}));
        assert_eq!(stream_gain(0,127,127).unwrap(),0.);
        assert_eq!(stream_gain(127,127,127).unwrap(),32767./32768.);
        assert_eq!(stream_gain(100,70,127).unwrap(),14341./32768.);
        assert!(stream_gain(128,127,127).is_err());
    }
    #[test]
    fn original_fade_track_matches_all_native_audio_capture_rows(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let event=EventData::parse(include_str!("../../../assets/native/events/ev21/event.json")).unwrap();
        let rows=std::fs::read_to_string(root.join("evidence/ev21-native-audio.jsonl")).unwrap();let mut count=0;
        for row in rows.lines(){
            let row:serde_json::Value=serde_json::from_str(row).unwrap();let Some(frame)=row["before"].as_u64()else{continue;};
            let native=u32::from_str_radix(row["fade"]["rgba"].as_str().unwrap(),16).unwrap().to_be_bytes();
            assert_eq!(row["fade"]["active"],1);assert_eq!(camera_fade(&event,frame as u32).unwrap(),Some(native),"event fade frame{frame}");count+=1;
        }assert_eq!(count,1080);
        // Integer interpolation/rounding gives170 here; native float/truncate gives169.
        assert_eq!(camera_fade(&event,5).unwrap(),Some([0,0,0,169]));
        assert_eq!(camera_fade(&event,748).unwrap(),Some([0;4]));
        assert_eq!(camera_fade(&event,1079).unwrap(),Some([0,0,0,255]));
    }
    #[test]
    fn rinrin_source_camera_cut_boundaries_and_final_frame() {
        // Original ev21 CAM MOT sample counts, source hash in events manifest.
        let mut clock=EventClock::new(&[180,375,112,413]).unwrap();
        assert_eq!(clock.total_frames(),1080);
        for (frame,cut,local) in [(179,0,179),(180,1,0),(554,1,374),(555,2,0),(666,2,111),(667,3,0),(1079,3,412)] {
            assert!(clock.seek_global(frame));
            assert_eq!((clock.cut(),clock.local_frame()),(cut,local));
            assert!(!clock.seek_global(frame));
        }
        let last=clock.clone(); assert!(!clock.advance()); assert_eq!(clock,last);
        assert!(!clock.seek_global(1080)); assert_eq!(clock,last);
        assert!(!clock.seek_cut(2,112)); assert_eq!(clock,last);
        assert!(clock.seek_cut(1,374)); assert_eq!(clock.frame(),554);
        assert!(clock.advance()); assert_eq!((clock.frame(),clock.cut(),clock.local_frame()),(555,2,0));
        assert!(clock.seek_cut(0,0));
        let mut steps=0; while clock.advance() { steps+=1; }
        assert_eq!(steps,1079);
        assert!(EventClock::new(&[]).is_err());
        assert!(EventClock::new(&[0]).is_err());
        assert!(EventClock::new(&[u32::MAX,1]).is_err());
        assert!(EventClock::new(&[1;65]).is_err());
        assert!(!EventClock::new(&[1]).unwrap().advance());
    }

    #[test]
    fn complete_native_camera_capture_matches_source_pose_application(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let event=EventData::parse(&std::fs::read_to_string(root.join("assets/native/events/ev21/event.json")).unwrap()).unwrap();
        let mut player=EventPlayer::new(&event).unwrap();let mut checked=0;
        let rows=std::fs::read_to_string(root.join("evidence/ev21-native-camera.jsonl")).unwrap();
        for row in rows.lines(){
            let row:serde_json::Value=serde_json::from_str(row).unwrap();let Some(before)=row["before"].as_u64()else{continue;};
            assert_eq!(row["framesPerUpdate"],2);
            // Clock callback runs after the event step; the camera's applied
            // packet still belongs to the preceding update. Row0 is the prior
            // stage camera. Compare the applied view, not its newly moved cursor.
            if before==0{continue;}
            player.clock.seek_global(before as u32-1);let pose=player.camera(&event);
            for (name,expected)in [("eye",pose.eye),("target",pose.target),("up",pose.up)]{
                for axis in 0..3{let actual=row["applied"][name][axis].as_f64().unwrap()as f32;assert!((actual-expected[axis]).abs()<0.00002,"event frame {} {name}/{axis}: {actual} != {}",before-1,expected[axis]);}
            }
            assert!((row["applied"]["fovDegrees"].as_f64().unwrap()as f32-pose.fov_radians.to_degrees()).abs()<0.00002);checked+=1;
        }
        assert_eq!(checked,1079);
    }

    #[test]
    fn actor_cuts_follow_source_visibility_and_local_frame(){
        let event=EventData::parse(include_str!("../../../assets/native/events/ev21/event.json")).unwrap();let mut player=EventPlayer::new(&event).unwrap();
        assert!(player.actor_motion(&event,"em0440",0).is_none());assert_eq!(player.actor_motion(&event,"pl0030",0).unwrap().frames,180);
        player.clock.seek_global(180);assert_eq!(player.actor_motion(&event,"em0440",0).unwrap().frames,375);
        player.clock.seek_global(555);assert!(player.actor_motion(&event,"pl0030",0).is_none());assert_eq!(player.actor_motion(&event,"em0440",0).unwrap().frames,112);
        player.clock.seek_global(667);assert_eq!(player.actor_motion(&event,"pl0030",0).unwrap().frames,413);
        assert!(player.actor_motion(&event,"pl0030",1).is_none());
    }
    #[test]
    fn original_intro_attachment_ids_and_mesh_cancellation(){
        let event=EventData::parse(include_str!("../../../assets/native/events/ev21/event.json")).unwrap();
        for(name,bone)in [("pl0040",14),("wp0600",8)]{assert_eq!(actor_state(&event,name,0,0).unwrap().attachment,Some(EventAttachment{target:"pl0030_00".into(),bone}));}
        let jack=actor_state(&event,"pl0030",0,0).unwrap();
        for name in ["fist_SB","L_hand_SB","kobushi02_SB","kobushi01_SB","goggles2_belt_UR_SB","goggles2_KB","goggles2_belt_SB"]{assert!(jack.mesh_cancelled(name),"{name}");}
        for name in ["body_SB","goggles_KB","goggles_belt_SB","goggles_belt_UR_SB"]{assert!(!jack.mesh_cancelled(name),"{name}");}
        let saw=actor_state(&event,"wp0600",0,0).unwrap();assert!(saw.mesh_cancelled("blade_SB")&&saw.mesh_cancelled("blade_UR_SB"));assert!(!saw.mesh_cancelled("engineA_SB"));
        // Native15 is substring matching; its pattern is not a glob/exact name.
        assert!(saw.mesh_cancelled("blade_SB_extra"));
    }
}
