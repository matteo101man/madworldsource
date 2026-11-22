//! Original event BGM/VCE commands; audio assets and all names resolve before
//! the inspection clock starts. Scene overlap, event SE and subtitles are separate.
use bevy::{prelude::*,audio::AudioSinkPlayback};
use mw_sim::event::{StreamAction,StreamBus,StreamCue,EVENT_HZ};
use std::{collections::{HashMap,HashSet},time::Duration};
use super::super::{audio,Bgm,BgmTrack,BgmVoice,StreamMix};

#[derive(serde::Deserialize)]#[serde(rename_all="camelCase")]
struct Manifest{schema:String,event:u32,event_source_sha256:String,language:String,streams:HashMap<String,Track>}
#[derive(Clone,serde::Deserialize)]#[serde(rename_all="camelCase")]
struct Track{
    id:u32,bus:String,mix:StreamMix,rate:u32,samples:Option<u32>,file:Option<String>,intro:Option<String>,
    #[serde(rename="loop")]looped:Option<String>,loop_start:Option<u32>,loop_end:Option<u32>,
    #[serde(skip)]handle:Handle<AudioSource>,#[serde(skip)]loop_handle:Handle<AudioSource>,
}
pub struct Program{
    tracks:HashMap<String,Track>,cues:Vec<StreamCue>,cursor:usize,pub start:f64,finished:bool,enabled:bool,
    executed:Vec<(u32,String)>,heard:HashSet<String>,
}
impl Program{
    pub fn load(root:&std::path::Path,dir:&str,event:&mw_data::event::EventData)->Self{
        let manifest:Manifest=serde_json::from_str(&std::fs::read_to_string(root.join("assets/native/events").join(dir).join("audio/audio.json")).expect("staged event audio")).expect("event audio manifest");
        assert_eq!(manifest.schema,"mw-event-audio/1");assert_eq!(manifest.event,event.event);assert_eq!(manifest.event_source_sha256,event.source_sha256);assert_eq!(manifest.language,"ENG");
        let cues=mw_sim::event::stream_cues(event).expect("native event stream commands");
        for cue in &cues{if let StreamAction::Play{name,..}|StreamAction::Preload{name}=&cue.action{
            let track=manifest.streams.get(name).expect("resolved native stream");assert!(track.rate>0);let _=track.mix.gain();
            assert_eq!(track.bus,if cue.bus==StreamBus::Music{"BGM"}else{"VCE"});
            for file in [track.file.as_ref(),track.intro.as_ref(),track.looped.as_ref()].into_iter().flatten(){
                assert!(file.starts_with("native/")&&!file.contains(['\\',':'])&&!file.split('/').any(|s|s=="..")&&root.join("assets").join(file).is_file(),"native stream path");
            }
            assert!(track.file.is_some()&&track.samples.is_some_and(|n|n>0)||track.intro.is_some()&&track.looped.is_some()&&track.loop_start.zip(track.loop_end).is_some_and(|(start,end)|start>0&&start<end),"native stream payload");
        }}
        Self{tracks:manifest.streams,cues,cursor:0,start:0.,finished:false,enabled:std::env::var_os("MW_NO_EVENT_AUDIO").is_none(),executed:vec![],heard:HashSet::new()}
    }
    pub fn preload(&mut self,assets:&AssetServer){
        if !self.enabled{return;}
        for track in self.tracks.values_mut(){
            track.handle=assets.load(track.file.as_ref().or(track.intro.as_ref()).unwrap().clone());
            if let Some(file)=&track.looped{track.loop_handle=assets.load(file.clone());}
        }
    }
    pub fn ready(&self,assets:&AssetServer)->bool{
        !self.enabled||self.tracks.values().all(|t|assets.is_loaded_with_dependencies(t.handle.id())&&(t.looped.is_none()||assets.is_loaded_with_dependencies(t.loop_handle.id())))
    }
}
#[derive(Component)]pub struct Stream{bus:StreamBus,name:String,base:f32,start:f64,fade_in:f64,fade_out:Option<(f64,f64)>,persist:bool}
// 804F8478 converts 60Hz fade units to333 callbacks/sec; each callback is96 samples at32kHz.
fn fade_seconds(frames:u32)->f64{((frames as u64*333)/60)as f64*96./32000.}

pub fn play(mut intro:ResMut<super::Intro>,time:Res<Time<Virtual>>,sound:Res<audio::PortAudio>,mut bgm:ResMut<Bgm>,
    mut voices:Query<(Entity,&mut Stream,&mut audio::Gain,Option<&AudioSink>)>,mut commands:Commands){
    let(active,fixed,started,frame)=(intro.active,intro.fixed,intro.started,intro.player.as_ref().map(|p|p.clock.frame()).unwrap_or(0));
    let skipped=intro.skipped();
    if fixed||!started{return;}let Some(program)=intro.audio.as_mut()else{return;};if !program.enabled{return;}
    let now=time.elapsed_secs_f64();
    for(entity,mut stream,mut gain,sink)in &mut voices{
        if let Some(sink)=sink{if program.heard.insert(stream.name.clone()){eprintln!("[intro-audio] sink {} gain={} paused={}",stream.name,sink.volume().to_linear(),sink.is_paused());}}
        let mut envelope=if stream.fade_in>0.{((now-stream.start)/stream.fade_in).clamp(0.,1.)as f32}else{1.};
        if let Some((start,duration))=stream.fade_out{
            if now-start>=duration{commands.entity(entity).despawn();continue;}
            envelope*= (1.-(now-start)/duration).clamp(0.,1.)as f32;
        }
        gain.base=stream.base*envelope;
        if !active&&!program.finished{
            if stream.persist&&stream.bus==StreamBus::Music{
                eprintln!("[intro-audio] retained {} position={:?} event_elapsed={}",stream.name,sink.map(AudioSink::position),now-stream.start);
                commands.entity(entity).remove::<Stream>();
            }else{stream.fade_out=Some((now,0.));commands.entity(entity).despawn();}
        }
    }
    if !active{
        if !program.finished{
            if std::env::var_os("MW_INTRO_SMOKE").is_some(){
                let expected:Vec<_>=[(0,"EV21_RSD".into()),(0,"JKSNN_EVVOSTR_EV21".into()),(748,"BS04C_RSD".into())].into_iter().filter(|(f,_)|!skipped||*f<=frame).collect();
                assert_eq!(program.executed,expected);assert_eq!(program.heard.len(),expected.len(),"event stream sinks missing");assert!(!bgm.looping);
                if frame>=748{assert!((bgm.started.unwrap()-(program.start+748./EVENT_HZ)).abs()<0.00001);}
                eprintln!("[intro-audio-smoke] {} source stream cues/sinks; skipped={skipped}",expected.len());
            }program.finished=true;
        }return;
    }
    while let Some(cue)=program.cues.get(program.cursor).filter(|c|c.frame<=frame).cloned(){
        program.cursor+=1;
        let(start,name,volume,fade_out,fade_in,persist)=match cue.action{
            StreamAction::Preload{..}=>continue,
            StreamAction::Stop{fade}=>{
                for(_,mut stream,_,_)in &mut voices{if stream.bus==cue.bus{stream.fade_out=Some((now,fade_seconds(fade)));}}continue;
            },
            StreamAction::Play{name,volume,fade_out,fade_in,persist}=>(program.start+cue.frame as f64/EVENT_HZ,name,volume,fade_out,fade_in,persist),
        };
        program.executed.push((cue.frame,name.clone()));
        if cue.bus==StreamBus::Music&&std::env::var_os("MW_NO_BGM").is_some(){continue;}
        // (Elise's fight music is her intro's EV33 [requested]: ev33's closing BS04C is not started, so the room starts
        // its EV33 loop, play_intro.rs room_bgm)
        if super::super::elise::on()&&name=="BS04C_RSD"{continue;}
        let track=&program.tracks[&name];let mut mix=track.mix;if let Some(volume)=volume{mix.volume=volume;}
        let base=mix.gain();let lag=(now-start).max(0.);
        // Do not restart a completed one-shot when a slow presentation crosses its cue.
        if track.samples.is_some_and(|n|lag>=n as f64/track.rate as f64){continue;}
        for(_,mut stream,_,_)in &mut voices{if stream.bus==cue.bus{stream.fade_out=Some((now,fade_seconds(fade_out)));}}
        let bus=if cue.bus==StreamBus::Music{audio::Bus::Music}else{audio::Bus::Cutscene};
        let envelope=if fade_in==0{1.}else{(lag/fade_seconds(fade_in)).clamp(0.,1.)as f32};
        let settings=PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(sound.gain(bus,base*envelope))).with_start_position(Duration::from_secs_f64(lag));
        let mut entity=commands.spawn((Stream{bus:cue.bus,name:name.clone(),base,start,fade_in:fade_seconds(fade_in),fade_out:None,persist},audio::Gain{bus,base:base*envelope},AudioPlayer::new(track.handle.clone()),settings));
        if cue.bus==StreamBus::Music{
            entity.insert(BgmVoice);
            // a persistent one-shot (ev34's JINGLE05) has no loop to hand the room music; it just plays out
            if persist&&track.looped.is_some(){
                bgm.track=Some(BgmTrack{intro:track.intro.clone().expect("persistent boss intro"),looped:track.looped.clone().expect("persistent boss loop"),rate:track.rate,loop_start:track.loop_start.unwrap(),mix:Some(mix)});
                bgm.started=Some(start);bgm.looping=false;bgm.stopped=false;
            }
        }
        eprintln!("[intro-audio] frame={} id={:#x} {name} source_gain={base} seek={lag}",cue.frame,track.id);
    }
}
