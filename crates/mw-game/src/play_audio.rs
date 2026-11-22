//! Rust fight audio buses, persisted in the same [audio] table as the PC frontend.
use bevy::{prelude::*,audio::AudioSinkPlayback};
use mw_sim::ui::SoundSettings;
use std::path::PathBuf;
/// Commentary: the commentators' lines (on the announcer level, and the commentary switch).
#[derive(Clone,Copy,Debug)]pub enum Bus{Music,Sfx,Cutscene,Announcers,Commentary}
#[derive(Component,Clone,Copy)]pub struct Gain{pub bus:Bus,pub base:f32}
/// `music_on` / `commentary_on`: the port's switches in the pause menu (Esc: M / C), saved as [audio] music_on /
/// commentary_on; commentary unset means the player's default (off as Kreely, on as Jack) [added].
#[derive(Resource)]pub struct PortAudio{pub settings:SoundSettings,pub master:f32,path:Option<PathBuf>,pub music_on:bool,pub commentary_on:bool,pub commentary_set:bool}
impl PortAudio{
    pub fn load()->Self{
        let path=std::env::var_os("MW_PC_CONTROLS").map(PathBuf::from).or_else(||std::env::current_exe().ok().and_then(|p|p.parent().map(|p|p.join("pc-controls.toml"))));
        Self::load_path(path,std::env::var("MW_MASTER_VOLUME").ok().and_then(|s|s.parse::<f32>().ok()))
    }
    fn load_path(path:Option<PathBuf>,override_master:Option<f32>)->Self{
        let table=path.as_ref().and_then(|p|std::fs::read_to_string(p).ok()).and_then(|s|s.parse::<toml::Table>().ok());
        let audio=table.as_ref().and_then(|t|t.get("audio")).and_then(toml::Value::as_table);
        let mut settings:SoundSettings=audio.cloned().and_then(|t|toml::Value::Table(t).try_into().ok()).unwrap_or_default();
        for (row,level)in settings.levels().into_iter().enumerate(){settings.set_level(row,level);}
        let configured=audio.and_then(|t|t.get("master_volume")).and_then(|v|v.as_float().or_else(||v.as_integer().map(|n|n as f64))).unwrap_or(1.)as f32;
        let master=override_master.unwrap_or(configured);
        let flag=|k:&str|audio.and_then(|t|t.get(k)).and_then(toml::Value::as_bool);
        Self{settings,master:if master.is_finite(){master.clamp(0.,1.)}else{1.},path,music_on:flag("music_on").unwrap_or(true),
             commentary_on:flag("commentary_on").unwrap_or(true),commentary_set:flag("commentary_on").is_some()}
    }
    pub fn gain(&self,bus:Bus,base:f32)->f32{
        let levels=self.settings.levels();
        let on=match bus{Bus::Music=>self.music_on,Bus::Commentary=>self.commentary_on,_=>true};
        if !on{return 0.;}
        self.master*base*levels[match bus{Bus::Music=>0,Bus::Sfx=>1,Bus::Cutscene=>2,Bus::Announcers|Bus::Commentary=>3}]as f32/10.
    }
    pub fn save(&self){
        let Some(path)=&self.path else{return;};
        let mut table=match std::fs::read_to_string(path){
            Ok(text)=>match text.parse::<toml::Table>(){Ok(t)=>t,Err(e)=>{warn!("[sound-settings] not overwriting invalid config: {e}");return;}},
            Err(e)if e.kind()==std::io::ErrorKind::NotFound=>toml::Table::new(),
            Err(e)=>{warn!("[sound-settings] read failed: {e}");return;}
        };
        let values=toml::Value::try_from(&self.settings).unwrap();
        let Some(audio)=table.entry("audio").or_insert_with(||toml::Value::Table(toml::Table::new())).as_table_mut()else{warn!("[sound-settings] invalid audio table");return;};
        for (key,value)in values.as_table().unwrap(){audio.insert(key.clone(),value.clone());}
        audio.insert("music_on".into(),toml::Value::Boolean(self.music_on));
        if self.commentary_set{audio.insert("commentary_on".into(),toml::Value::Boolean(self.commentary_on));}
        let text=toml::to_string_pretty(&table).unwrap();let temporary=path.with_extension("toml.tmp");
        let result=std::fs::write(&temporary,text).and_then(|_|std::fs::rename(&temporary,path));
        if let Err(e)=result{warn!("[sound-settings] save failed: {e}");}else{eprintln!("[sound-settings] saved {}",path.display());}
    }
}
pub struct PortAudioPlugin;
impl Plugin for PortAudioPlugin{fn build(&self,app:&mut App){app.insert_resource(PortAudio::load()).add_systems(PostUpdate,apply);}}
fn apply(settings:Res<PortAudio>,mut sinks:Query<(&Gain,&mut AudioSink)>,mut spatial:Query<(&Gain,&mut SpatialAudioSink)>){
    for (gain,mut sink)in &mut sinks{sink.set_volume(bevy::audio::Volume::Linear(settings.gain(gain.bus,gain.base)));}
    for (gain,mut sink)in &mut spatial{sink.set_volume(bevy::audio::Volume::Linear(settings.gain(gain.bus,gain.base)));}
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn independent_buses_multiply_source_gain_and_master(){
        let mut audio=PortAudio{settings:SoundSettings::default(),master:0.7,path:None,music_on:true,commentary_on:true,commentary_set:false};
        audio.settings.music_level=9;audio.settings.sfx_level=3;audio.settings.announcer_level=0;audio.settings.cutscene_level=2;
        assert!((audio.gain(Bus::Music,0.6)-0.378).abs()<0.000001);
        assert!((audio.gain(Bus::Sfx,0.4)-0.084).abs()<0.000001);assert_eq!(audio.gain(Bus::Announcers,1.),0.);
        assert!((audio.gain(Bus::Cutscene,1.)-0.14).abs()<0.000001);
    }
    #[test]fn save_reload_preserves_other_pc_settings_and_rejects_invalid_files(){
        let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("mw-rust-sound-{}-{nonce}",std::process::id()));std::fs::create_dir(&dir).unwrap();let path=dir.join("pc-controls.toml");
        std::fs::write(&path,"version=1\n[audio]\nmaster_volume=0.4\n[keyboard]\nattack=['MouseLeft','KeyE']\n[video]\nfullscreen=true\n[custom]\nkeep=12\n").unwrap();
        let mut audio=PortAudio::load_path(Some(path.clone()),None);assert_eq!(audio.master,0.4);audio.settings.music_level=9;audio.settings.subtitles=true;audio.save();
        let loaded=PortAudio::load_path(Some(path.clone()),None);assert_eq!(loaded.settings,audio.settings);
        let table=std::fs::read_to_string(&path).unwrap().parse::<toml::Table>().unwrap();assert_eq!(table["keyboard"]["attack"].as_array().unwrap().len(),2);assert_eq!(table["video"]["fullscreen"].as_bool(),Some(true));assert_eq!(table["custom"]["keep"].as_integer(),Some(12));
        std::fs::write(&path,"invalid ! toml").unwrap();audio.save();assert_eq!(std::fs::read_to_string(&path).unwrap(),"invalid ! toml");
        std::fs::remove_file(&path).unwrap();std::fs::remove_dir(&dir).unwrap();
    }
}
