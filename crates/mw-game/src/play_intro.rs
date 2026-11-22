//! Original boss-entry event, cleanup wait, resource gate and room fade.
//! MW_INTRO_PREVIEW=1 retains the fixed/frozen source inspection path.
//! Per stage: c201 plays ev21; c303 (MW_STAGE=elise) plays ev33 (room event 0 fn_80160870, the bats form Elise) and,
//! after the finisher, ev34 (the cBoss fiber fn_80160278 plays it once its 80133AA4 boss wait ends,
//! research/ELISE-BOSS.md §2). A missing effects, UI or writing manifest is an event without that layer staged.
use bevy::prelude::*;
use mw_data::{event::EventData,Clip,Bone};
use mw_sim::event::{EventCameraPose,EventPlayer,EVENT_HZ};
use std::collections::HashSet;
use mw_sim::event_room::{BossEntry,EntryPhase};
#[path="play_intro_audio.rs"]pub mod audio;
#[path="play_intro_effects.rs"]pub mod effects;
#[path="play_intro_ui.rs"]pub mod ui;
#[path="play_challenge.rs"]pub mod challenge;

#[derive(serde::Deserialize)]struct ModelFile{schema:String,actors:Vec<Model>}
#[derive(serde::Deserialize)]struct Model{object:String,glb:String,bones:Vec<Bone>,meshes:Vec<MeshName>}
#[derive(serde::Deserialize)]struct MeshName{node:String,name:String}
#[derive(Component)]pub struct IntroProp{model:usize,instance:u32,joints:Option<Vec<Entity>>}
#[derive(Component)]pub struct IntroMesh{model:usize,instance:u32,name:String,original:Visibility}
/// One event of the stage's chain, loaded and preloaded up front.
struct Stage{event:EventData,player:EventPlayer,audio:audio::Program,effects:Option<effects::Program>,ui:Option<ui::Program>}
fn load_stage(root:&std::path::Path,dir:&str,frame:Option<u32>)->(Stage,Vec<Model>){
    let folder=root.join("assets/native/events").join(dir);
    let event=EventData::parse(&std::fs::read_to_string(folder.join("event.json")).expect("staged event")).expect("valid event");
    let mut player=EventPlayer::new(&event).unwrap();
    if let Some(frame)=frame{assert!(frame<player.clock.total_frames(),"MW_INTRO_FRAME outside event");if frame!=0{assert!(player.clock.seek_global(frame));}}
    let models:ModelFile=serde_json::from_str(&std::fs::read_to_string(folder.join("models.json")).expect("staged intro models")).expect("intro model data");assert_eq!(models.schema,"mw-event-models/1");
    for model in &models.actors{
        assert!(model.glb.starts_with("native/")&&!model.glb.contains('\\')&&!model.glb.contains(':')&&!model.glb.split('/').any(|c|c==".."));
        let mut ids=HashSet::new();for(i,bone)in model.bones.iter().enumerate(){assert!(bone.id>=0&&ids.insert(bone.id)&&bone.parent>=-1&&bone.parent<(i as i16)&&bone.relative.iter().all(|v|v.is_finite()),"intro rig topology");}
    }
    let audio=audio::Program::load(root,dir,&event);
    let effects=Some(if folder.join("audio/effects.json").is_file(){effects::Program::load(root,dir,&event)}else{effects::Program::layered(root,&event)});
    let ui=folder.join("ui/ui.json").is_file().then(||ui::Program::load(root,dir,&event));
    (Stage{event,player,audio,effects,ui},models.actors)
}
#[derive(Component)]pub struct IntroFade;

#[derive(Resource,Default)]pub struct Intro{
    challenge:Option<challenge::Program>,event:Option<EventData>,player:Option<EventPlayer>,active:bool,fixed:bool,accumulator:f64,started:bool,cuts_seen:u64,models:Vec<Model>,bound:HashSet<Entity>,
    audio:Option<audio::Program>,
    effects:Option<effects::Program>,
    ui:Option<ui::Program>,
    entry:Option<BossEntry>,room_bgm:Option<super::BgmTrack>,room_audio:Vec<Handle<AudioSource>>,skip_requested:bool,
    /// PC hold-to-skip (recomp native_controls::CutsceneSkip): armed by an Enter press inside the event, seconds held.
    pub skip_hold:Option<f32>,
    /// The events still to play after this one (none of the shipped stages chains one now).
    queue:Vec<Stage>,
    /// set when a chained event starts: the previous event's effect voices stop
    flush_effects:bool,
    /// an event played after the boss is beaten (the Baron: ev51; Elise: ev34)
    outro:Option<Stage>,
    /// The story event after the room clear (the scene change after the results: Elise's ev84 in scene 0xB14, the
    /// Baron's ev93 then 0xE03), when staged; asked for by play_clear.rs, then its state: playing, done. ev84 can be
    /// skipped (the event player's no-skip list is ev18 / 34 / 43 / 51 / 93).
    ending:Option<Stage>,ending_req:bool,ending_on:bool,ending_finished:bool,ending_skippable:bool,
}
impl Intro{
    pub fn load(root:&std::path::Path)->Self{
        if std::env::var("MW_INTRO_PREVIEW").as_deref()==Ok("0")||std::env::var("MW_DIRECT_FIGHT").as_deref()==Ok("1")||super::editor::on(){return Self::default();}
        let preview=std::env::var("MW_INTRO_PREVIEW").as_deref()==Ok("1")||std::env::var_os("MW_INTRO_FRAME").is_some();
        let frame=std::env::var("MW_INTRO_FRAME").ok().map(|v|v.parse::<u32>().expect("MW_INTRO_FRAME"));
        let elise=super::elise::on();let baron=super::baron::on()&&!super::baron::mathilda_boss();
        // the Mathilda variant has its own intro (play_baron.rs cine), no event
        if super::baron::mathilda_boss(){return Self::default();}
        // inspection: MW_ENDING_TEST=1 plays the room's ending event straight away (no fight, no entry)
        if (elise||baron)&&std::env::var_os("MW_ENDING_TEST").is_some(){
            // (=results: the fight and the results board first, the ending through play_clear.rs as after a real clear)
            if std::env::var("MW_ENDING_TEST").as_deref()==Ok("results"){
                let (stage,models)=load_stage(root,if elise{"ev84"}else{"ev93"},None);
                return Self{ending_skippable:elise,ending:Some(stage),models,..default()};
            }
            let (Stage{event,player,audio,effects,ui},models)=load_stage(root,if elise{"ev84"}else{"ev93"},frame);
            return Self{ending_skippable:elise,ending_on:true,event:Some(event),player:Some(player),audio:Some(audio),effects,ui,active:true,models,..default()};
        }
        let dirs:&[&str]=if elise{&["ev33"]}else if baron{&["ev50"]}else{&["ev21"]};
        let (first,mut models)=load_stage(root,dirs[0],frame);
        // the defeat event after the fight (the cBoss callbacks: the Baron's ev51, then the ending ev93; Elise's ev34)
        let outro=(baron||elise).then(||{let (stage,more)=load_stage(root,if elise{"ev34"}else{"ev51"},None);for m in more{if !models.iter().any(|k|k.object==m.object){models.push(m);}}stage});
        // the ending after the clear (staged by tools/events: ev84 / ev93)
        let ending_dir=if elise{"ev84"}else{"ev93"};
        let ending=((baron||elise)&&root.join("assets/native/events").join(ending_dir).join("event.json").is_file()&&std::env::var("MW_ENDING").as_deref()!=Ok("0"))
            .then(||{let (stage,more)=load_stage(root,ending_dir,None);for m in more{if !models.iter().any(|k|k.object==m.object){models.push(m);}}stage});
        let mut queue=vec![];
        for dir in &dirs[1..]{
            let (stage,more)=load_stage(root,dir,None);
            for m in more{if !models.iter().any(|k|k.object==m.object){models.push(m);}}
            queue.push(stage);
        }
        let Stage{event,player,audio,effects,ui}=first;
        // c303's room music is BS04C (the ev33 SEQ's own track), c201's BS04
        let room_bgm=(!preview).then(||{
            let manifest:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("assets/native/audio/bgm/bgm.json")).expect("staged room music")).unwrap();
            // (Elise's fight loops her intro event's own music, EV33, in place of BS04C [requested]: the whole stream, then again)
            if elise{
                let ev:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("assets/native/events/ev33/audio/audio.json")).expect("ev33 audio")).unwrap();
                let m=&ev["streams"]["EV33_RSD"];
                return serde_json::from_value(serde_json::json!({"intro":m["file"],"loop":m["file"],"rate":m["rate"],"loop_start":m["samples"],"mix":m["mix"]})).expect("ev33 room track");
            }
            serde_json::from_value(manifest[if baron{"bs03"}else{"bs04"}].clone()).expect("native room track")
        });
        // Elise has no challenge prompt in her own room (only RinRin's and the other callers of 8012409C do)
        Self{outro,ending_skippable:elise,ending,challenge:(!preview&&!elise&&!baron).then(||challenge::Program::load(root)).flatten(),entry:(!preview).then(||BossEntry::new(event.event)),room_bgm,event:Some(event),player:Some(player),audio:Some(audio),effects,ui,active:true,fixed:frame.is_some(),models,queue,..default()}
    }
    pub fn challenge_blocked(&self)->bool{self.challenge.as_ref().is_some_and(challenge::Program::blocks_entry)}
    pub fn active(&self)->bool{self.active}
    /// The defeat event is loaded and has not started yet (Elise: ev34 still to come).
    pub fn outro_pending(&self)->bool{self.outro.is_some()}
    /// An ending is staged (play_clear.rs plays it after the results).
    pub fn has_ending(&self)->bool{self.ending.is_some()||self.ending_on}
    pub fn request_ending(&mut self){self.ending_req=true;}
    pub fn ending_active(&self)->bool{self.ending_on&&!self.ending_finished}
    pub fn ending_done(&self)->bool{self.ending_finished}
    pub fn controls_blocked(&self)->bool{self.active||self.entry.as_ref().is_some_and(BossEntry::controls_blocked)}
    pub fn skipped(&self)->bool{self.entry.as_ref().is_some_and(|e|e.skipped)}
    pub fn camera(&self)->Option<EventCameraPose>{(self.active&&!self.challenge_blocked()).then(||self.player.as_ref().unwrap().camera(self.event.as_ref().unwrap()))}
    pub fn actor(&self,name:&str)->Option<(&Clip,f32)>{self.actor_instance(name,0)}
    pub fn actor_instance(&self,name:&str,instance:u32)->Option<(&Clip,f32)>{
        if !self.active||self.challenge_blocked(){return None;}let player=self.player.as_ref().unwrap();
        Some((player.actor_motion(self.event.as_ref().unwrap(),name,instance)?,player.clock.local_frame()as f32))
    }
    /// Move on to the next chained event; false when this was the last.
    fn next_event(&mut self,bgm:&mut super::Bgm)->bool{
        if self.queue.is_empty(){return false;}
        let Stage{event,player,audio,effects,ui}=self.queue.remove(0);
        eprintln!("[intro] chained event ev{:02x} follows",event.event);
        if self.entry.is_some(){self.entry=Some(BossEntry::new(event.event));}
        self.event=Some(event);self.player=Some(player);self.audio=Some(audio);self.effects=effects;self.ui=ui;
        self.started=false;self.flush_effects=true;self.cuts_seen=0;self.accumulator=0.;self.skip_hold=None;self.skip_requested=false;
        // the earlier event's persistent music cue does not carry into the next one
        bgm.track=None;bgm.started=None;bgm.looping=false;
        true
    }
}
pub fn setup(mut commands:Commands,mut intro:ResMut<Intro>,assets:Res<AssetServer>,play:Res<super::Play>){
    if !intro.active{return;}
    if let Some(p)=intro.challenge.as_mut(){p.preload(&assets);}
    intro.audio.as_mut().unwrap().preload(&assets);
    if let Some(p)=intro.effects.as_mut(){p.preload(&assets);}
    if let Some(p)=intro.ui.as_mut(){p.preload(&assets);}
    let it=&mut *intro;
    for stage in it.queue.iter_mut().chain(it.outro.iter_mut()).chain(it.ending.iter_mut()){stage.audio.preload(&assets);if let Some(p)=stage.effects.as_mut(){p.preload(&assets);}if let Some(p)=stage.ui.as_mut(){p.preload(&assets);}}
    if let Some(track)=&intro.room_bgm{let handles=vec![assets.load(track.intro.clone()),assets.load(track.looped.clone())];intro.room_audio=handles;}
    commands.spawn((IntroFade,Node{position_type:PositionType::Absolute,width:percent(100),height:percent(100),..default()},BackgroundColor(Color::BLACK),GlobalZIndex(18000)));
    // one prop per actor instance any chained event uses (ev33's fifteen bats)
    let mut events:Vec<&EventData>=intro.event.iter().collect();events.extend(intro.queue.iter().chain(intro.outro.iter()).chain(intro.ending.iter()).map(|s|&s.event));
    for(index,model)in intro.models.iter().enumerate(){
        if model.object=="pl0030"{continue;}
        let instances=events.iter().map(|e|e.actors.iter().filter(|a|a.object==model.object).map(|a|a.instance+1).max().unwrap_or(0)).max().unwrap_or(1).max(1);
        // the colour look shows Elise and her bats in their staged colour models (same nodes as the event's source GLBs)
        // (generic: an event prop's colour export staged beside it as <stem>-color.glb)
        let color=model.glb.replace(".glb","-color.glb");
        let glb=if play.colored&&model.glb.starts_with("native/events/")&&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(&color).is_file(){color}else{model.glb.clone()};
        for instance in 0..instances{commands.spawn((WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(glb.clone()))),Transform::default(),Visibility::Hidden,IntroProp{model:index,instance,joints:None}));}
    }
}
pub fn fade(intro:Res<Intro>,baron:Res<super::baron::Baron>,mut quads:Query<(&mut BackgroundColor,&mut Visibility),With<IntroFade>>){
    // (the Mathilda variant's own intro uses the same fade quad: baron.cine_fade)
    let rgba=if intro.challenge_blocked(){None}else{intro.entry.as_ref().and_then(BossEntry::fade).or_else(||if intro.active{mw_sim::event::camera_fade(intro.event.as_ref().unwrap(),intro.player.as_ref().unwrap().clock.frame()).expect("native fade command")}else{None}).or_else(||baron.cine_fade())};
    for(mut color,mut visible)in &mut quads{
        if let Some([r,g,b,a])=rgba{*color=BackgroundColor(Color::srgba_u8(r,g,b,a));*visible=Visibility::Visible;}else{*visible=Visibility::Hidden;}
    }
}
fn mesh_nodes(root:Entity,children:&Query<&Children>,names:&Query<(&Name,&Visibility)>,model:&Model)->Vec<(Entity,String,Visibility)>{
    children.iter_descendants(root).filter_map(|e|{let(n,v)=names.get(e).ok()?;let source=model.meshes.iter().find(|m|m.node==n.as_str())?;Some((e,source.name.clone(),*v))}).collect()
}
pub fn bind_models(mut commands:Commands,mut intro:ResMut<Intro>,mut props:Query<(Entity,&mut IntroProp)>,chars:Query<(Entity,&super::Character)>,children:Query<&Children>,names:Query<(&Name,&Visibility)>){
    if !intro.active{return;}
    for(root,mut prop)in &mut props{
        if intro.bound.contains(&root){continue;}
        let model=&intro.models[prop.model];let meshes=mesh_nodes(root,&children,&names,model);if meshes.len()!=model.meshes.len(){continue;}
        let nodes:std::collections::HashMap<_,_>=children.iter_descendants(root).filter_map(|e|names.get(e).ok().map(|(n,_)|(n.as_str().to_string(),e))).collect();
        let joints:Option<Vec<Entity>>=model.bones.iter().map(|b|b.id).chain(std::iter::once(-1)).map(|id|nodes.get(&format!("role_0_bone{id}")).copied()).collect();
        let Some(joints)=joints else{continue;};prop.joints=Some(joints);
        for(e,name,original)in meshes{commands.entity(e).insert(IntroMesh{model:prop.model,instance:prop.instance,name,original});}intro.bound.insert(root);
    }
    let Some(index)=intro.models.iter().position(|m|m.object=="pl0030")else{return;};
    for(root,ch)in &chars{
        if ch.who!=super::Who::Jack||intro.bound.contains(&root){continue;}
        let meshes=mesh_nodes(root,&children,&names,&intro.models[index]);if meshes.len()!=intro.models[index].meshes.len(){continue;}
        for(e,name,original)in meshes{commands.entity(e).insert(IntroMesh{model:index,instance:0,name,original});}intro.bound.insert(root);
    }
}
pub fn pose_props(intro:Res<Intro>,play:Res<super::Play>,mut props:Query<(&IntroProp,&mut Transform,&mut Visibility)>,mut joints:Query<&mut Transform,Without<IntroProp>>){
    for(prop,mut transform,mut visible)in &mut props{
        *visible=Visibility::Hidden;if !intro.active{continue;}
        let model=&intro.models[prop.model];let Some(js)=&prop.joints else{continue;};
        // Jack's chainsaw arm (pl0040 / wp0600) has no place on Kreely
        if play.as_kreely&&matches!(model.object.as_str(),"pl0040"|"wp0600"){continue;}let Some((clip,frame))=intro.actor_instance(&model.object,prop.instance)else{continue;};
        let event=intro.event.as_ref().unwrap();let player=intro.player.as_ref().unwrap();let state=mw_sim::event::actor_state(event,&model.object,prop.instance,player.clock.frame()).expect("native prop commands");
        *transform=Transform::default();
        if let Some(attachment)=state.attachment{
            let target=event.actors.iter().find(|a|format!("{}_{:02}",a.object,a.instance)==attachment.target).expect("attachment target actor");
            let Some((target_clip,target_frame))=intro.actor_instance(&target.object,target.instance)else{continue;};
            let bones=match target.object.as_str(){"pl0030"=>&play.fd.jack.bones[..],"em0440"=>&play.fd.rin.bones[..],other=>intro.model_bones(other).expect("Untranslated attachment target rig")};
            let Some(index)=bones.iter().position(|b|b.id as u32==attachment.bone)else{panic!("Native attachment bone missing");};
            let frames=mw_sim::fight::bone_frames(bones,&mw_sim::character::sample_retargeted(bones,target_clip,target_frame,mw_sim::character::Retarget::NATIVE));let(rotation,position,scale)=frames[index];
            *transform=Transform{translation:Vec3::from_array(position),rotation:Quat::from_array(rotation),scale:Vec3::from_array(scale)};
        }
        let poses=mw_sim::character::sample_retargeted(&model.bones,clip,frame,mw_sim::character::Retarget::NATIVE);super::set_joints(js,&poses,&mut joints);*visible=Visibility::Visible;
    }
}
pub fn mesh_commands(intro:Res<Intro>,mut meshes:Query<(&IntroMesh,&mut Visibility)>){
    let frame=intro.player.as_ref().map(|p|p.clock.frame()).unwrap_or(0);
    // an actor the current event lacks (ev34 has no bats) is simply not commanded
    let state=|model:usize,instance:u32|intro.event.as_ref().and_then(|e|e.actors.iter().any(|a|a.object==intro.models[model].object&&a.instance==instance).then(||mw_sim::event::actor_state(e,&intro.models[model].object,instance,frame).expect("native mesh commands")));
    let mut cache:std::collections::HashMap<(usize,u32),Option<mw_sim::event::EventActorState>>=Default::default();
    for(mesh,mut visible)in &mut meshes{
        *visible=mesh.original;if !intro.active||intro.challenge_blocked(){continue;}
        if cache.entry((mesh.model,mesh.instance)).or_insert_with(||state(mesh.model,mesh.instance)).as_ref().is_some_and(|s|s.mesh_cancelled(&mesh.name)){*visible=Visibility::Hidden;}
    }
}
pub fn verify_restored(intro:Res<Intro>,meshes:Query<(&IntroMesh,&Visibility)>,props:Query<&Visibility,With<IntroProp>>,mut checked:Local<bool>){
    if intro.active||!intro.started||*checked||std::env::var_os("MW_INTRO_SMOKE").is_none(){return;}
    let mut count=0;for(mesh,visible)in &meshes{assert_eq!(*visible,mesh.original);count+=1;}
    assert_eq!(count,80);assert_eq!(props.iter().count(),2);for visible in &props{assert_eq!(*visible,Visibility::Hidden);}
    eprintln!("[intro-smoke] restored80 mesh nodes and hid2 intro prop roots");*checked=true;
}
pub const SKIP_HOLD_SECS:f32=1.2;
impl Intro{pub fn model_bones(&self,object:&str)->Option<&[Bone]>{self.models.iter().find(|m|m.object==object).map(|m|m.bones.as_slice())}
/// The skip hint while the intro cutscene plays: Some(hold progress 0..1).
pub fn skip_hint(&self)->Option<f32>{(self.active&&!self.challenge_blocked()&&!(self.ending_on&&!self.ending_skippable)&&self.entry.as_ref().is_none_or(|e|e.skip_allowed)).then(||self.skip_progress().unwrap_or(0.))}
pub fn skip_progress(&self)->Option<f32>{self.skip_hold.filter(|_|self.active).map(|h|(h/SKIP_HOLD_SECS).min(1.))}}
pub fn fight_running(intro:Res<Intro>,baron:Res<super::baron::Baron>)->bool{
    // the Mathilda variant's own intro holds the fight too
    baron.cine.is_none()&&intro.entry.as_ref().map(BossEntry::fight_running).unwrap_or(!intro.active)
}
pub fn advance(mut intro:ResMut<Intro>,time:Res<Time<Virtual>>,chars:Query<(Entity,&super::Character)>,props:Query<&IntroProp>,play:Res<super::Play>,assets:Res<AssetServer>,keys:Res<ButtonInput<KeyCode>>,mouse:Res<ButtonInput<MouseButton>>,mut pause:ResMut<super::pause::Pause>,mut bgm:ResMut<super::Bgm>){
    if intro.challenge_blocked(){return;}
    // the boss is beaten: its outro event plays on the same player (no room entry after it)
    if play.fight.victory&&!intro.active&&intro.outro.is_some()&&intro.entry.as_ref().is_none_or(|e|e.phase==EntryPhase::Ready){
        let Stage{event,player,audio,effects,ui}=intro.outro.take().unwrap();
        eprintln!("[intro] outro ev{:02x}",event.event);
        intro.event=Some(event);intro.player=Some(player);intro.audio=Some(audio);intro.effects=effects;intro.ui=ui;
        intro.entry=None;intro.active=true;intro.started=false;intro.flush_effects=true;intro.cuts_seen=0;intro.accumulator=0.;
        bgm.track=None;bgm.started=None;bgm.looping=false;
    }
    // the clear's results confirmed: the ending event (no fight after it)
    if intro.ending_req&&!intro.active&&intro.outro.is_none()&&intro.ending.is_some(){
        intro.ending_req=false;
        let Stage{event,player,audio,effects,ui}=intro.ending.take().unwrap();
        eprintln!("[intro] ending ev{:02x}",event.event);
        intro.event=Some(event);intro.player=Some(player);intro.audio=Some(audio);intro.effects=effects;intro.ui=ui;
        intro.entry=None;intro.active=true;intro.started=false;intro.flush_effects=true;intro.cuts_seen=0;intro.accumulator=0.;intro.ending_on=true;
        bgm.track=None;bgm.started=None;bgm.looping=false;
    }
    if intro.ending_on&&!intro.active&&!intro.ending_finished{intro.ending_finished=true;eprintln!("[intro] ending done");}
    if !intro.active&&intro.entry.as_ref().map(|e|e.phase==EntryPhase::Ready).unwrap_or(true){return;}
    if chars.iter().any(|(_,c)|c.joints.is_none())||props.iter().any(|p|p.joints.is_none())||!chars.iter().any(|(e,c)|c.who==super::Who::Jack&&intro.bound.contains(&e)){return;}
    if !intro.ui.as_ref().is_none_or(|u|u.ready(&assets)){return;}
    if !intro.fixed&&(!intro.audio.as_ref().unwrap().ready(&assets)||!intro.effects.as_ref().is_none_or(|e|e.ready(&assets))){return;}
    if !intro.started{
        let clock=&intro.player.as_ref().unwrap().clock;
        eprintln!("[intro-preview] source frame={} cut={} local={} hz={EVENT_HZ} fight frozen; remaining SEQ pending",clock.frame(),clock.cut(),clock.local_frame());intro.started=true;
        intro.audio.as_mut().unwrap().start=time.elapsed_secs_f64();
        return; // Frame0 and its streams share this presentation/time origin.
    }
    if intro.fixed{return;}
    let event_playing=intro.entry.as_ref().map(|e|e.phase==EntryPhase::Event).unwrap_or(intro.active);
    if event_playing&&intro.entry.is_some(){
        let scripted=std::env::var("MW_INTRO_SKIP_AT").ok().map(|v|v.parse::<u32>().expect("MW_INTRO_SKIP_AT")).is_some_and(|frame|intro.player.as_ref().unwrap().clock.frame()>=frame);
        // Hold Enter (SKIP_HOLD_SECS, as the recomp) or tap B/Menu Back (Backspace / right mouse).
        if keys.just_pressed(KeyCode::Enter){intro.skip_hold=Some(0.);}
        if !keys.pressed(KeyCode::Enter){intro.skip_hold=None;}
        let dt=time.delta_secs();let held=intro.skip_hold.as_mut().map(|h|{*h+=dt;*h>=SKIP_HOLD_SECS}).unwrap_or(false);
        if held{intro.skip_hold=None;}
        if keys.just_pressed(KeyCode::Backspace)||mouse.just_pressed(MouseButton::Right)||scripted||held{
            intro.skip_requested=true;pause.release_inputs=true;
        }
    }
    // ev84 skips as the entry events do (hold Enter, or Backspace / right mouse); ev93 does not
    if intro.ending_on&&intro.ending_skippable&&intro.active&&intro.entry.is_none(){
        if keys.just_pressed(KeyCode::Enter){intro.skip_hold=Some(0.);}
        if !keys.pressed(KeyCode::Enter){intro.skip_hold=None;}
        let dt=time.delta_secs();let held=intro.skip_hold.as_mut().map(|h|{*h+=dt;*h>=SKIP_HOLD_SECS}).unwrap_or(false);
        if keys.just_pressed(KeyCode::Backspace)||mouse.just_pressed(MouseButton::Right)||held{
            intro.skip_hold=None;intro.active=false;intro.ending_finished=true;pause.release_inputs=true;eprintln!("[intro] ending skipped (backspace {} rmb {} enter held {})",keys.just_pressed(KeyCode::Backspace),mouse.just_pressed(MouseButton::Right),held);return;
        }
    }
    if event_playing&&std::env::var_os("MW_INTRO_SMOKE").is_some(){assert_eq!(play.tick,0);assert_eq!(play.fight.player.hp,100);assert_eq!(play.fight.player.pos,super::START);}
    intro.cuts_seen|=1<<intro.player.as_ref().unwrap().clock.cut();
    intro.accumulator+=time.delta_secs_f64();
    while intro.accumulator>=1./EVENT_HZ{
        intro.accumulator-=1./EVENT_HZ;
        if let Some(phase)=intro.entry.as_ref().map(|e|e.phase).filter(|p|*p!=EntryPhase::Event){
            let loaded=intro.room_audio.iter().all(|h|assets.is_loaded_with_dependencies(h.id()));
            intro.entry.as_mut().unwrap().advance(loaded);let next=intro.entry.as_ref().unwrap().phase;
            if phase==EntryPhase::Ending{intro.active=false;eprintln!("[intro-room] event cleaned; room wait13 updates");}
            if next==EntryPhase::Fade(0){
                if bgm.started.is_none(){bgm.track=intro.room_bgm.clone().filter(|_|std::env::var_os("MW_NO_BGM").is_none());bgm.stopped=false;eprintln!("[intro-room] starting original room music fallback");}
                eprintln!("[intro-room] combat actor released; starting15-update room fade; tick={}",play.tick);
                if std::env::var_os("MW_INTRO_SMOKE").is_some(){assert_eq!(play.tick,0);}
            }
            if next==EntryPhase::Ready{
                pause.release_inputs=true;eprintln!("[intro-room] control released after room fade; fight tick={}",play.tick);
                if std::env::var_os("MW_INTRO_SMOKE").is_some(){assert!((14..=16).contains(&play.tick));assert_eq!(play.fight.player.hp,100);eprintln!("[intro-room-smoke] fight waited for event/cleanup, AI advanced during fade and controls released");}
                break;
            }
            continue;
        }
        let advanced=intro.player.as_mut().unwrap().clock.advance();
        if intro.skip_requested{
            intro.skip_requested=false;
            if intro.next_event(&mut bgm){break;}
            if intro.entry.as_mut().unwrap().finish(true){eprintln!("[intro-room] B/Menu Back skip ended at frame={}",intro.player.as_ref().unwrap().clock.frame());break;}
        }
        if !advanced{
            if intro.next_event(&mut bgm){break;}
            if std::env::var_os("MW_INTRO_SMOKE").is_some(){assert_eq!(intro.cuts_seen,15);assert_eq!(intro.player.as_ref().unwrap().clock.frame(),1079);eprintln!("[intro-smoke] all four cuts completed; fight tick={} HP={} position={:?}",play.tick,play.fight.player.hp,play.fight.player.pos);}
            if let Some(entry)=intro.entry.as_mut(){entry.finish(false);eprintln!("[intro-room] source event ended; black cleanup transition");}
            else{intro.active=false;eprintln!("[intro-preview] source tracks ended; resuming inspection fight");}break;
        }
        let bit=1u64<<intro.player.as_ref().unwrap().clock.cut();
        if intro.cuts_seen&bit==0{eprintln!("[intro-preview] cut={} frame={}",intro.player.as_ref().unwrap().clock.cut(),intro.player.as_ref().unwrap().clock.frame());intro.cuts_seen|=bit;}
    }
}
