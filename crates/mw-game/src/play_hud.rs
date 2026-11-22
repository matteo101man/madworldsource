//! Original common39 player HP/common31 boss HP. Remaining cockpit consumers
//! are tracked in research/HUD-API.md.
use bevy::prelude::*;
use mw_sim::{ui::UiScene,ui_hud::{HealthMeter,LowHealthEffects}};
use super::pause::native_ui::{self,UiSource};
struct Bar{scene:UiScene,meter:HealthMeter,last:f32,hold:i32,low:Option<LowHealthEffects>}
impl Bar{
    fn new(source:&UiSource,program:u32,ratio:f32)->Self{
        let mut scene=source.scene(program);let meter=HealthMeter::from_scene(&scene,ratio);meter.apply(&mut scene);
        let low=if program==39{for state in &mut scene.states{state.stopped=0xfc;}Some(LowHealthEffects::new(source.scene(58)))}else{None};
        Self{scene,meter,last:ratio,hold:-1,low}
    }
    fn step(&mut self,ratio:f32,boss:bool){
        self.scene.advance(1.);
        if ratio<self.last{self.scene.bind_rgb(10,10);}
        if ratio>self.meter.committed{self.scene.bind_rgb(10,12);}
        if boss{
            if ratio!=self.last{
                if self.hold==-1{self.scene.bind_rgb(1,1);self.scene.bind_alpha(1,1);}
                self.hold=100;
            }
            if self.hold>0{self.hold-=1;}else if self.hold==0{self.hold=-1;self.scene.bind_rgb(1,2);self.scene.bind_alpha(1,2);}
        }else if ratio<self.last{
            self.scene.restart(1);
        }
        self.meter.step(ratio);self.meter.apply(&mut self.scene);
        if let Some(low)=&mut self.low{let was=low.active;low.step(ratio,ratio<self.last,&mut self.scene);if was!=low.active&&std::env::var_os("MW_HUD_SMOKE").is_some(){eprintln!("[hud-smoke] low-health active={} HP={ratio}",low.active);}}
        self.last=ratio;
    }
}
#[derive(Resource)]pub struct Hud{source:UiSource,images:Vec<Handle<Image>>,bars:Option<[Bar;2]>,last_tick:u64}
impl Hud{pub fn source(&self)->&UiSource{&self.source}}
#[derive(Component)]pub struct HudQuad;
pub fn setup(mut commands:Commands,assets:Res<AssetServer>){
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let source=UiSource::load(&root,"core_us");let images=source.layout.pages.iter().map(|p|assets.load(p.image.clone())).collect();
    commands.insert_resource(Hud{source,images,bars:None,last_tick:0});
}
pub fn advance(mut hud:ResMut<Hud>,play:Res<super::Play>,intro:Res<super::intro::Intro>,assets:Res<AssetServer>){
    if intro.controls_blocked()||hud.images.iter().any(|h|!assets.is_loaded_with_dependencies(h.id())){return;}
    // the boss gauge: Elise (play.boss) or RinRin (1600 [observed])
    let boss=play.boss.map(|(hp,max,_)|hp as f32/max.max(1) as f32).unwrap_or(play.fight.rin.hp as f32/1600.);
    let hp=[(play.fight.player.hp as f32/100.).clamp(0.,1.),boss.clamp(0.,1.)];
    if hud.bars.is_none(){hud.bars=Some([Bar::new(&hud.source,39,hp[0]),Bar::new(&hud.source,31,hp[1])]);hud.last_tick=play.tick;eprintln!("[hud] original player39/boss31 admitted; tick={}",play.tick);return;}
    if hud.last_tick==play.tick{return;}hud.last_tick=play.tick;
    for(i,bar)in hud.bars.as_mut().unwrap().iter_mut().enumerate(){let changed=bar.last!=hp[i];bar.step(hp[i],i==1);if changed&&std::env::var_os("MW_HUD_SMOKE").is_some(){eprintln!("[hud-smoke] tick={} program={} HP={} front={} trail={} delay={} hold={}",play.tick,bar.scene.program.id,hp[i],bar.meter.front.value,bar.meter.trail.value,bar.meter.delay,bar.hold);}}
}
pub fn present(mut commands:Commands,hud:Res<Hud>,play:Res<super::Play>,intro:Res<super::intro::Intro>,baron:Res<super::baron::Baron>,execution:Res<super::execution::Execution>,pause:Res<super::pause::Pause>,assets:Res<AssetServer>,windows:Query<&Window>,quads:Query<Entity,With<HudQuad>>,mut frozen:Local<Option<Vec<u32>>>,mut faded:Local<bool>){
    for e in &quads{commands.entity(e).despawn();}
    if std::env::var_os("MW_HUD_SMOKE").is_some(){
        let signature:Vec<u32>=hud.bars.as_ref().into_iter().flatten().flat_map(|b|{let mut values=vec![b.meter.front.value.to_bits(),b.meter.trail.value.to_bits(),b.hold as u32];for state in &b.scene.states{values.extend([state.clocks.position.to_bits(),state.clocks.rgb.to_bits(),state.clocks.alpha.to_bits()]);}if let Some(low)=&b.low{values.extend([low.active as u32,low.alpha_age.to_bits()]);for state in &low.scene.states{values.extend([state.clocks.position.to_bits(),state.clocks.rgb.to_bits(),state.clocks.alpha.to_bits()]);}}values}).collect();
        if pause.open{if let Some(old)=frozen.as_ref(){assert_eq!(*old,signature,"HUD progressed while paused");}else{*frozen=Some(signature);eprintln!("[hud-smoke] HUD frozen/hidden with pause");}}
        else if frozen.take().is_some(){eprintln!("[hud-smoke] HUD resumed with fight");}
    }
    if intro.controls_blocked()||baron.cine.is_some()||execution.hides_hud()||pause.open{return;}let Some(bars)=&hud.bars else{return;};let Ok(window)=windows.single()else{return;};let viewport=Vec2::new(window.width(),window.height());
    for(i,bar)in bars.iter().enumerate(){
        if i==1&&(play.fight.victory||(play.fight.solo&&play.boss.is_none())){continue;}
        if let Some(low)=&bar.low{let effects=native_ui::sprites(&low.scene,&hud.source.layout,&hud.source.layout,&assets,viewport,500,None);if std::env::var_os("MW_HUD_SMOKE").is_some(){assert!(effects.is_empty(),"Disabled original program58 geometry became visible");}for sprite in effects{commands.spawn((sprite,HudQuad));}}
        let sprites=native_ui::sprites(&bar.scene,&hud.source.layout,&hud.source.layout,&assets,viewport,1000,None);
        if i==1&&bar.last<1.&&bar.hold==-1&&bar.scene.states[bar.scene.index(1).unwrap()].clocks.alpha>=20.&&std::env::var_os("MW_HUD_SMOKE").is_some(){assert!(sprites.is_empty(),"Boss bar survived source hold/fade");if !*faded{eprintln!("[hud-smoke] boss bar cleared after source100-update hold/fade");*faded=true;}}
        for sprite in sprites{commands.spawn((sprite,HudQuad));}
    }
}
