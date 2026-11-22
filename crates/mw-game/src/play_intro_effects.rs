//! EV21 numeric2D effects on the event clock. Native dry stereo gain is
//! verified; effect sends and DSP release/resampling are not emulated here.
use bevy::{prelude::*,audio::AudioSinkPlayback};
use mw_sim::event::{EventSeCue,EventSeProgram,EVENT_HZ};
use std::{collections::{HashMap,HashSet},time::Duration};
use super::super::audio;

#[derive(serde::Deserialize)]#[serde(rename_all="camelCase")]
struct Manifest{schema:String,event:u32,event_source_sha256:String,sounds:HashMap<String,Sound>}
#[derive(serde::Deserialize)]#[serde(rename_all="camelCase")]
struct Sound{file:String,rate:u32,samples:u32,program:Vec<u8>,dry_mix:DryMix,#[serde(skip)]handle:Handle<AudioSource>}
#[derive(serde::Deserialize)]struct DryMix{left:u32,right:u32,denominator:u32}
pub struct Program{cues:Vec<EventSeCue>,sounds:HashMap<String,Sound>,cursor:usize,executed:Vec<(u32,u32)>,heard:HashSet<u32>,finished:bool,enabled:bool,
    /// Events without a verified dry-mix manifest (ev33 / ev34): each cue plays its full RedSpark program from the shared
    /// SE bank (assets/native/audio/se/se.json), every layer at its own delay and volume, centred [pitch not applied].
    layered:HashMap<u32,Vec<Layer>>,pending:Vec<(f64,u32,usize)>}
struct Layer{file:String,gain:f32,delay:f64,handle:Handle<AudioSource>}
#[derive(serde::Deserialize)]struct BankSound{volume:u32,plays:Vec<BankPlay>}
#[derive(serde::Deserialize)]struct BankPlay{delay:u64,volume:u32,file:String}
impl Program{
    pub fn load(root:&std::path::Path,dir:&str,event:&mw_data::event::EventData)->Self{
        let manifest:Manifest=serde_json::from_str(&std::fs::read_to_string(root.join("assets/native/events").join(dir).join("audio/effects.json")).expect("staged event effects")).expect("event effects manifest");
        assert_eq!(manifest.schema,"mw-event-effects/1");assert_eq!(manifest.event,event.event);assert_eq!(manifest.event_source_sha256,event.source_sha256);
        let cues=mw_sim::event::event_se_cues(event).expect("native event effects");
        for cue in &cues{
            assert_eq!((cue.attributes,cue.options,cue.slot),(1,8,Some(0)),"Untranslated event SE admission/handle mode");
            let sound=&manifest.sounds[&cue.id.to_string()];let program=EventSeProgram::parse(&sound.program).expect("supported native effect program");
            assert_eq!((cue.volume,program.volume,program.note),(127,127,60),"Untranslated event SE volume/pitch");
            assert_eq!(sound.dry_mix.left,sound.dry_mix.right,"Untranslated asymmetric event mix");assert_eq!(sound.dry_mix.denominator,32768);
            assert!(sound.dry_mix.left<=32767&&sound.rate>0&&sound.samples>0);
            assert!(sound.file.starts_with("native/")&&!sound.file.contains(['\\',':'])&&!sound.file.split('/').any(|p|p=="..")&&root.join("assets").join(&sound.file).is_file());
        }
        Self{cues,sounds:manifest.sounds,cursor:0,executed:vec![],heard:HashSet::new(),finished:false,enabled:std::env::var_os("MW_NO_EVENT_AUDIO").is_none(),layered:HashMap::new(),pending:vec![]}
    }
    pub fn layered(root:&std::path::Path,event:&mw_data::event::EventData)->Self{
        let cues=mw_sim::event::event_se_cues(event).expect("native event effects");
        let bank:HashMap<String,HashMap<String,BankSound>>=serde_json::from_str(&std::fs::read_to_string(root.join("assets/native/audio/se/se.json")).expect("staged SE bank")).expect("SE bank");
        let mut layered=HashMap::new();
        for cue in &cues{
            let Some(sound)=bank.get(&(cue.id/1000).to_string()).and_then(|b|b.get(&(cue.id%1000).to_string()))else{eprintln!("[intro-se] cue {} has no staged program",cue.id);continue;};
            // the low byte of a staged delay is the 60 Hz wait (play.rs queue_sound); 23169/32768 is the centred dry mix
            let layers=sound.plays.iter().map(|p|Layer{file:p.file.clone(),gain:sound.volume as f32/127.*p.volume as f32/127.*cue.volume as f32/127.*0.70709,delay:(p.delay&0xFF)as f64/60.,handle:default()}).collect();
            layered.insert(cue.id,layers);
        }
        Self{cues,sounds:HashMap::new(),cursor:0,executed:vec![],heard:HashSet::new(),finished:false,enabled:std::env::var_os("MW_NO_EVENT_AUDIO").is_none(),layered,pending:vec![]}
    }
    pub fn preload(&mut self,assets:&AssetServer){if self.enabled{
        for sound in self.sounds.values_mut(){sound.handle=assets.load(sound.file.clone());}
        for layer in self.layered.values_mut().flatten(){layer.handle=assets.load(layer.file.clone());}
    }}
    pub fn ready(&self,assets:&AssetServer)->bool{!self.enabled||self.sounds.values().all(|s|assets.is_loaded_with_dependencies(s.handle.id()))&&self.layered.values().flatten().all(|l|assets.is_loaded_with_dependencies(l.handle.id()))}
}
#[derive(Component)]pub struct Effect{pub id:u32}
pub fn play(mut intro:ResMut<super::Intro>,time:Res<Time<Virtual>>,sound:Res<audio::PortAudio>,voices:Query<(Entity,&Effect,Option<&AudioSink>)>,mut commands:Commands){
    let(active,fixed,started,frame,start)=(intro.active,intro.fixed,intro.started,intro.player.as_ref().map(|p|p.clock.frame()).unwrap_or(0),intro.audio.as_ref().map(|a|a.start).unwrap_or(0.));
    let skipped=intro.skipped();
    if std::mem::take(&mut intro.flush_effects){for(entity,_,_)in &voices{commands.entity(entity).despawn();}}
    if fixed||!started{return;}let Some(program)=intro.effects.as_mut()else{return;};if !program.enabled{return;}
    for(entity,effect,sink)in &voices{
        if let Some(sink)=sink{if program.heard.insert(effect.id){eprintln!("[intro-se] sink {} gain={} position={:?}",effect.id,sink.volume().to_linear(),sink.position());}}
        if !active&&!program.finished{commands.entity(entity).despawn();}
    }
    if !active{
        if !program.finished&&std::env::var_os("MW_INTRO_SMOKE").is_some(){
            let expected:Vec<_>=[(180,7522001),(866,7522006)].into_iter().filter(|(f,_)|!skipped||*f<=frame).collect();
            assert_eq!(program.executed,expected);assert_eq!(program.heard.len(),expected.len());
            eprintln!("[intro-se-smoke] {} source cues/waves instantiated; skipped={skipped}",expected.len());
        }program.finished=true;return;
    }
    let now=time.elapsed_secs_f64();
    let due:Vec<_>=program.pending.iter().filter(|p|p.0<=now).copied().collect();program.pending.retain(|p|p.0>now);
    for(_,id,k)in due{
        let layer=&program.layered[&id][k];
        commands.spawn((Effect{id},audio::Gain{bus:audio::Bus::Sfx,base:layer.gain},AudioPlayer::new(layer.handle.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(sound.gain(audio::Bus::Sfx,layer.gain)))));
    }
    while let Some(cue)=program.cues.get(program.cursor).filter(|c|c.frame<=frame).copied(){
        program.cursor+=1;program.executed.push((cue.frame,cue.id));
        if let Some(layers)=program.layered.get(&cue.id){
            let at=start+cue.frame as f64/EVENT_HZ;
            for(k,layer)in layers.iter().enumerate(){program.pending.push((at+layer.delay,cue.id,k));}
            eprintln!("[intro-se] frame={} id={} layers={}",cue.frame,cue.id,layers.len());continue;
        }
        let Some(source)=program.sounds.get(&cue.id.to_string())else{continue;};
        let body=EventSeProgram::parse(&source.program).unwrap();let lag=(time.elapsed_secs_f64()-start-cue.frame as f64/EVENT_HZ).max(0.);
        let duration=body.hold_seconds().min(source.samples as f64/source.rate as f64);
        if lag>=duration{continue;}
        let base=source.dry_mix.left as f32/source.dry_mix.denominator as f32;
        commands.spawn((Effect{id:cue.id},audio::Gain{bus:audio::Bus::Sfx,base},AudioPlayer::new(source.handle.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(sound.gain(audio::Bus::Sfx,base)))
                .with_start_position(Duration::from_secs_f64(lag)).with_duration(Duration::from_secs_f64(duration-lag))));
        eprintln!("[intro-se] frame={} id={} wave={} note={} hold={}s dry_gain={base} seek={lag}",cue.frame,cue.id,body.wave,body.note,body.hold_seconds());
    }
}
