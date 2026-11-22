//! Native boss-entry confirmation, shown before the intro. Yes joins the event flow;
//! No reopens it because the Rust roaming/world return is not ported.
use bevy::prelude::*;
use mw_sim::ui_challenge::{Challenge,Input,Phase};
use super::super::pause::native_ui::{self,UiSource};
pub struct Program{source:UiSource,prompt:Challenge,messages:mw_data::ui_message::MessageCatalog,text:Vec<mw_sim::ui_message::TextQuad>,images:Vec<Handle<Image>>,accumulator:f64,pending:Input,updates:u32,finished:bool}
impl Program{
    pub fn load(root:&std::path::Path)->Option<Self>{
        // Default on (the recomp reaches the fight through this prompt); MW_CHALLENGE=0 starts at the intro.
        if std::env::var("MW_CHALLENGE").as_deref()==Ok("0")&&std::env::var_os("MW_CHALLENGE_SMOKE").is_none(){return None;}
        let source=UiSource::load(root,"core_us");let prompt=Challenge::new(source.scene(42));
        let messages=mw_data::ui_message::MessageCatalog::parse(&std::fs::read_to_string(root.join("assets/native/ui/challenge-c201-us/catalog.json")).expect("staged native challenge message")).unwrap();
        let text=mw_sim::ui_message::centered_stage(&messages,0,[320.,200.],[20.,20.],20.*20./24.).unwrap();
        Some(Self{source,prompt,messages,text,images:vec![],accumulator:0.,pending:Input::default(),updates:0,finished:false})
    }
    pub fn pending(&self)->bool{!self.finished}
    pub fn blocks_entry(&self)->bool{!self.finished||!self.prompt.result}
    pub fn preload(&mut self,assets:&AssetServer){
        for p in &self.source.layout.pages{self.images.push(assets.load(p.image.clone()));}
        for p in &self.messages.pages{self.images.push(assets.load(p.image.clone()));}
    }
    fn ready(&self,assets:&AssetServer)->bool{!self.images.is_empty()&&self.images.iter().all(|h|assets.is_loaded_with_dependencies(h.id()))}
}
#[derive(Component)]pub struct ChallengeQuad;
pub fn advance(mut intro:ResMut<super::Intro>,time:Res<Time<Virtual>>,assets:Res<AssetServer>,keys:Res<ButtonInput<KeyCode>>,mouse:Res<ButtonInput<MouseButton>>,play:Res<super::super::Play>,mut pause:ResMut<super::super::pause::Pause>,mut exit:MessageWriter<AppExit>){
    let Some(p)=intro.challenge.as_mut().filter(|p|p.pending())else{return;};if !p.ready(&assets){return;}
    p.pending.yes|=keys.just_pressed(KeyCode::ArrowLeft)||keys.just_pressed(KeyCode::KeyA);
    p.pending.no|=keys.just_pressed(KeyCode::ArrowRight)||keys.just_pressed(KeyCode::KeyD);
    p.pending.back|=keys.just_pressed(KeyCode::Backspace)||keys.just_pressed(KeyCode::Escape)||mouse.just_pressed(MouseButton::Right);
    p.pending.confirm|=keys.just_pressed(KeyCode::Enter)||mouse.just_pressed(MouseButton::Left);
    let fixed=std::env::var("MW_CHALLENGE_FRAME").ok().map(|s|s.parse::<u32>().expect("MW_CHALLENGE_FRAME"));
    if fixed.is_some_and(|frame|p.updates>=frame){return;}
    let smoke=std::env::var("MW_CHALLENGE_SMOKE").ok();if let Some(mode)=&smoke{assert!(matches!(mode.as_str(),"accept"|"decline"));}
    p.accumulator+=time.delta_secs_f64();
    while p.accumulator>=1./mw_sim::event::EVENT_HZ{
        p.accumulator-=1./mw_sim::event::EVENT_HZ;
        if let Some(mode)=&smoke{assert_eq!(play.tick,0);assert_eq!(play.fight.player.hp,100);assert_eq!(play.fight.player.pos,super::super::START);if p.prompt.phase==Phase::Selecting&&p.prompt.age>=15{p.pending=Input{back:mode=="decline",confirm:true,..default()};}}
        // [added] the challenge is taken for you: once the prompt has shown a moment, Yes is picked and confirmed
        // (MW_CHALLENGE_AUTO=0: choose it yourself)
        if smoke.is_none()&&std::env::var("MW_CHALLENGE_AUTO").as_deref()!=Ok("0")&&p.prompt.phase==Phase::Selecting&&p.prompt.age>=15{p.pending=Input{yes:true,confirm:true,..default()};}
        let before=p.prompt.phase;p.prompt.step(std::mem::take(&mut p.pending));p.updates+=1;
        if before!=p.prompt.phase{eprintln!("[challenge] update={} phase={:?} yes={} result={}",p.updates,p.prompt.phase,p.prompt.yes,p.prompt.result);}
        if p.prompt.phase==Phase::Done{
            p.finished=true;pause.release_inputs=true;
            if let Some(mode)=&smoke{assert_eq!(p.prompt.result,mode=="accept");eprintln!("[challenge-smoke] {mode}: native prompt closed; tick0 HP100; controls held until intro");}
            if !p.prompt.result{
                if smoke.is_some(){exit.write(AppExit::Success);break;}
                // The roaming-world return is unported: No offers the prompt again instead of leaving the stage.
                eprintln!("[challenge] declined: reopening prompt (roaming/world return remains unported)");
                p.prompt=Challenge::new(p.source.scene(42));p.finished=false;p.updates=0;
            }
            break;
        }
        if fixed.is_some_and(|frame|p.updates>=frame){break;}
    }
}
pub fn present(mut commands:Commands,intro:Res<super::Intro>,assets:Res<AssetServer>,windows:Query<&Window>,quads:Query<Entity,With<ChallengeQuad>>){
    for e in &quads{commands.entity(e).despawn();}
    let Some(p)=intro.challenge.as_ref().filter(|p|p.pending()&&p.ready(&assets))else{return;};let Ok(w)=windows.single()else{return;};let viewport=Vec2::new(w.width(),w.height());
    for sprite in native_ui::sprites(&p.prompt.scene,&p.source.layout,&p.source.layout,&assets,viewport,19000,None){commands.spawn((sprite,ChallengeQuad));}
    if p.prompt.message_visible(){for sprite in native_ui::text_sprites(&p.text,&p.messages,&assets,viewport,19500){commands.spawn((sprite,ChallengeQuad));}}
}
