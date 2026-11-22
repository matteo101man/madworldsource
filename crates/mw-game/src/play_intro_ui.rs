//! Original EV21 comic-lettering and fixed2D writing on the source event clock.
//! Dialogue subtitles have a separate consumer.
use bevy::prelude::*;
use mw_data::ui::{UiLayout,UiTimeline};
use mw_sim::{event::EventUiCue,ui::UiScene};
use super::super::pause::native_ui::{self,UiSource};
#[path="event_message_ui.rs"]mod messages;

#[derive(serde::Deserialize)]#[serde(rename_all="camelCase")]
struct Manifest{schema:String,event:u32,event_source_sha256:String,animation:String,programs:Vec<u32>,
               /// the story events' lettering draws from core_us; an event with its own pages (ev84 / ev93) says
               /// `"atlas":"own"` and names its layout
               #[serde(default)]shared_atlas:Option<String>,#[serde(default)]atlas:Option<String>,#[serde(default)]layout:Option<String>}
#[derive(serde::Deserialize)]#[serde(rename_all="camelCase")]
struct WritingManifest{schema:String,event:u32,event_source_sha256:String,animation:String,record:String,shared_atlas:String,frame:u32,id:u32}
struct Instance{cue:EventUiCue,scene:UiScene,announced:bool,max_quads:usize}
pub struct Writing{cue:mw_sim::event::EventWritingCue,scene:UiScene,announced:bool,max_quads:usize,finished:bool}
pub struct Program{source:UiSource,instances:Vec<Instance>,writing:Option<Writing>,messages:messages::Program,images:Vec<Handle<Image>>,finished:bool}
fn asset_path(path:&str)->bool{path.starts_with("native/")&&!path.contains(['\\',':'])&&!path.split('/').any(|p|p=="..")}
fn load_writing(root:&std::path::Path,dir:&str,event:&mw_data::event::EventData)->Option<Writing>{
    let path=root.join("assets/native/events").join(dir).join("writing/writing.json");if !path.is_file(){return None;}
    let manifest:WritingManifest=serde_json::from_str(&std::fs::read_to_string(path).expect("staged writing manifest")).expect("writing manifest");
    assert_eq!(manifest.schema,"mw-event-writing/1");assert_eq!(manifest.event,event.event);assert_eq!(manifest.event_source_sha256,event.source_sha256);assert_eq!(manifest.shared_atlas,"core_us");
    assert!(asset_path(&manifest.animation)&&asset_path(&manifest.record));
    let cues=mw_sim::event::event_writing_cues(event).expect("native writing cues");assert_eq!(cues.len(),1);let cue=cues[0];assert_eq!((cue.id,cue.frame),(manifest.id,manifest.frame));
    let mut record=mw_data::ui_writing::WritingRecord::parse(&std::fs::read(root.join("assets").join(manifest.record)).expect("original WCD record")).expect("valid WCD");
    assert_eq!(record.id as u32,cue.id);for i in 0..3{record.position[i]+=cue.position[i];}
    let bytes=std::fs::read(root.join("assets").join(manifest.animation)).expect("original WC bank");let bank=UiTimeline::parse(&bytes).expect("WC hierarchy");
    let scene=mw_sim::ui_writing::fixed_scene(&record,&bank,&bytes).expect("supported fixed2D writing");
    Some(Writing{cue,scene,announced:false,max_quads:0,finished:false})
}
impl Program{
    pub fn load(root:&std::path::Path,dir:&str,event:&mw_data::event::EventData)->Self{
        let manifest:Manifest=serde_json::from_str(&std::fs::read_to_string(root.join("assets/native/events").join(dir).join("ui/ui.json")).expect("staged event UI")).expect("event UI manifest");
        assert_eq!(manifest.schema,"mw-event-ui/1");assert_eq!(manifest.event,event.event);assert_eq!(manifest.event_source_sha256,event.source_sha256);
        assert!(manifest.shared_atlas.as_deref()==Some("core_us")||(manifest.atlas.as_deref()==Some("own")&&manifest.layout.as_deref().is_some_and(asset_path)));
        assert!(manifest.animation.starts_with("native/")&&!manifest.animation.contains(['\\',':'])&&!manifest.animation.split('/').any(|p|p==".."));
        let layout_path=match &manifest.layout{Some(l) if manifest.atlas.as_deref()==Some("own")=>root.join("assets").join(l),_=>root.join("assets/native/ui/core_us/layout.json")};
        let layout=UiLayout::parse(&std::fs::read_to_string(layout_path).expect("event atlas")).unwrap();
        let bytes=std::fs::read(root.join("assets").join(manifest.animation)).expect("original event IDT");
        let timeline=UiTimeline::parse(&bytes).expect("event UI hierarchy");
        assert_eq!(timeline.programs.iter().map(|p|p.id).collect::<Vec<_>>(),manifest.programs);
        let source=UiSource{layout,bytes,timeline};
        let instances=mw_sim::event::event_ui_cues(event).expect("native ID commands").into_iter().filter_map(|cue|{
            if !(cue.create&&cue.bank==4&&matches!(cue.layer,8|10)){eprintln!("[intro-ui] skipping untranslated cue program={} bank={} layer={}",cue.program,cue.bank,cue.layer);return None;}
            let scene=match source.try_scene(cue.program as u32){Ok(s)=>s,Err(e)=>{eprintln!("[intro-ui] program {} left out: {e}",cue.program);return None;}};assert!(scene.program.widgets.iter().all(|w|w.order<1000));
            Some(Instance{cue,scene,announced:false,max_quads:0})
        }).collect();
        // Native program100 is owner scaffolding, invisible throughout the
        // capture. Program0 is never requested by EV21. Neither is a text cue.
        let messages=messages::Program::load(root,event.event,Some(&event.source_sha256));
        Self{source,instances,writing:load_writing(root,dir,event),messages,images:vec![],finished:false}
    }
    pub fn preload(&mut self,assets:&AssetServer){
        self.messages.preload(assets);
        for page in &self.source.layout.pages{
            self.images.push(assets.load(page.image.clone()));
            if let Some(path)=&page.inverted_image{self.images.push(assets.load(path.clone()));}
        }
    }
    pub fn ready(&self,assets:&AssetServer)->bool{self.messages.ready(assets)&&!self.images.is_empty()&&self.images.iter().all(|h|assets.is_loaded_with_dependencies(h.id()))}
}
#[derive(Component)]pub struct IntroQuad;
pub fn present(mut commands:Commands,mut intro:ResMut<super::Intro>,sound:Res<super::super::audio::PortAudio>,assets:Res<AssetServer>,windows:Query<&Window>,quads:Query<Entity,With<IntroQuad>>){
    for entity in &quads{commands.entity(entity).despawn();}
    let(active,started,frame)=(intro.active,intro.started,intro.player.as_ref().map(|p|p.clock.frame()).unwrap_or(0));
    let skipped=intro.skipped();
    let Some(program)=intro.ui.as_mut()else{return;};if !started{return;}
    if !active{
        program.messages.clear();
        if !program.finished&&std::env::var_os("MW_INTRO_SMOKE").is_some(){
            if !skipped{
                assert_eq!(program.instances.iter().map(|i|(i.cue.program,i.max_quads)).collect::<Vec<_>>(),[(1,6),(2,5),(3,5)]);
                assert!(quads.is_empty(),"event UI survived its source fade");
                let writing=program.writing.as_ref().unwrap();assert_eq!(writing.max_quads,4);assert!(writing.finished);
                eprintln!("[intro-ui-smoke] all3 comic programs6/5/5 and WCD1114-glyph effect cleared");
            }else{
                for instance in &program.instances{if instance.cue.frame>frame{assert_eq!(instance.max_quads,0);}}
                if let Some(w)=&program.writing{if w.cue.frame>frame{assert_eq!(w.max_quads,0);}}
                eprintln!("[intro-ui-smoke] skipped UI cleaned; no future program was presented");
            }
        }program.finished=true;return;
    }
    program.messages.advance(frame,sound.settings.subtitles);
    if program.messages.bank.event==0x21{assert!(program.messages.selected(sound.settings.subtitles).is_none(),"EV21 has no original MESS task");}
    let Ok(window)=windows.single()else{return;};let viewport=Vec2::new(window.width(),window.height());
    if let Some(catalog)=&program.messages.catalog{for sprite in native_ui::text_sprites(&program.messages.quads(sound.settings.subtitles),catalog,&assets,viewport,16100){commands.spawn((sprite,IntroQuad));}}
    for instance in &mut program.instances{
        let Some(age)=instance.cue.age(frame)else{continue;};instance.scene.seek(age as f32);
        let count=render_scene(&mut commands,&instance.scene,&program.source.layout,&assets,viewport,16000+instance.cue.layer as i32*10);
        instance.max_quads=instance.max_quads.max(count);
        if !instance.announced{eprintln!("[intro-ui] frame={frame} source_cue={} program={} source_age={age}",instance.cue.frame,instance.cue.program);instance.announced=true;}
    }
    let Some(writing)=program.writing.as_mut()else{return;};
    if let Some(age)=frame.checked_sub(writing.cue.frame){
        if !writing.announced{eprintln!("[intro-writing] frame={frame} id={} source_age={age}",writing.cue.id);writing.announced=true;}
        if age>=mw_sim::ui_writing::CONTROLLER_TIMEOUT-1{
            if !writing.finished{eprintln!("[intro-writing] controller removed frame={frame} age={age}");writing.finished=true;}
        }else{
            writing.scene.seek(age as f32);
            writing.max_quads=writing.max_quads.max(render_scene(&mut commands,&writing.scene,&program.source.layout,&assets,viewport,16050));
        }
    }
}
fn render_scene(commands:&mut Commands,scene:&UiScene,layout:&UiLayout,assets:&AssetServer,viewport:Vec2,z:i32)->usize{
    // a draw style or glyph this port has not translated leaves the whole program undrawn rather than wrong
    for draw in scene.draw().iter().filter(|d|d.visible){
        if (draw.kind,draw.blend_mode)!=(0,0)||draw.tile_kind>1{return 0;}
        let Some(group)=layout.groups.iter().find(|g|g.id==draw.uv[0])else{return 0;};if (draw.uv[1]as usize)>=group.count{return 0;}
    }
    let sprites=native_ui::sprites(scene,layout,layout,assets,viewport,z,None);let count=sprites.len();
    for sprite in sprites{commands.spawn((sprite,IntroQuad));}count
}
