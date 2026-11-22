//! Pause ownership for the Rust fight. Original map widget data is shared with the UI oracle viewer.
use bevy::{prelude::*, input::InputSystems, audio::AudioSinkPlayback};
use std::collections::HashSet;
use mw_sim::ui::{PausePage,PauseNavigation,UiScene,SoundEdit,TitlePrompt,SaveSelection};
#[path="play_save.rs"]mod save;
#[path="native_ui.rs"]pub(super) mod native_ui;
#[derive(Resource,Default)]
pub struct Pause {
    pub open:bool, pub release_inputs:bool,pub menu:PauseNavigation,
    pub frame:u32, pub smoke:bool,pub sound:Option<SoundEdit>,pub title:Option<TitlePrompt>,pub title_age:f32,pub slots:Option<SaveSelection>,
    paused_audio:HashSet<Entity>,
    save_menu:mw_sim::ui_save::Menu,
}
impl Pause{
    // Smoke inputs are packets on a60Hz menu timeline, independent of hidden
    // window presentation speed. Production menu clocks consume real time.
    fn menu_step(&self,time:&Time<Real>)->f32{if self.smoke{1.}else{time.delta_secs()*60.}}
}
#[derive(Component)]struct PauseQuad;
#[derive(Resource)]struct PauseUi {
    source:native_ui::UiSource,common:native_ui::UiSource,title_source:native_ui::UiSource,title_scenes:Option<[UiScene;2]>,save_scenes:Option<Vec<UiScene>>,scenes:Option<[mw_sim::ui::UiScene;3]>,
    messages:mw_data::ui_message::MessageCatalog,map:mw_data::ui::StageMap,map_images:Vec<Handle<Image>>,selection:usize,
    page:PausePage,options:Option<UiScene>,background:Option<UiScene>,content:Option<UiScene>,dialog:Option<UiScene>,dialog_yes:bool,option_selection:usize,save_revision:u64,
}
pub struct PausePlugin;
impl Plugin for PausePlugin {
    fn build(&self,app:&mut App){
        app.insert_resource(Pause{smoke:std::env::var_os("MW_PAUSE_SMOKE").is_some(),..default()})
            .insert_resource(save::Saves::load())
            .add_systems(Startup,setup)
            .add_systems(PreUpdate,control.after(InputSystems).after(super::vr::VrInputSet))
            .add_systems(Update,(draw,port_settings))
            .add_systems(PostUpdate,audio);
    }
}
pub fn running(pause:Res<Pause>)->bool{!pause.open&&!super::vr::menu_open()}

/// The port's own settings beside the native pause pages [added]: Music (the room music) and Commentary (the
/// commentators; the bosses still speak), M / C while the menu is open, saved with the audio settings. Until the player
/// sets it, commentary follows who is playing: off as Kreely, on as Jack.
#[derive(Component)]struct PortSettingsText;
fn port_settings(mut commands:Commands,pause:Res<Pause>,keys:Res<ButtonInput<KeyCode>>,mut sound:ResMut<super::audio::PortAudio>,play:Res<super::Play>,
                 mut text:Query<(&mut Text,&mut Visibility),With<PortSettingsText>>){
    if !sound.commentary_set{let want=!play.as_kreely;if sound.commentary_on!=want{sound.commentary_on=want;}}
    let menu=pause.open&&!matches!(pause.menu.page,PausePage::TitleSplash|PausePage::SaveSlots|PausePage::SlotActions|PausePage::SaveConfirm|PausePage::TitleConfirm);
    if menu{
        let mut changed=false;
        if keys.just_pressed(KeyCode::KeyM){sound.music_on=!sound.music_on;changed=true;}
        if keys.just_pressed(KeyCode::KeyC){sound.commentary_on=!sound.commentary_on;sound.commentary_set=true;changed=true;}
        if changed{sound.save();}
    }
    let label=format!("SETTINGS    [M] MUSIC {}    [C] COMMENTARY {}",if sound.music_on{"ON"}else{"OFF"},if sound.commentary_on{"ON"}else{"OFF"});
    match text.single_mut(){
        Ok((mut t,mut v))=>{if t.0!=label{t.0=label;}let want=if menu{Visibility::Visible}else{Visibility::Hidden};if *v!=want{*v=want;}}
        Err(_)=>{commands.spawn((PortSettingsText,Text::new(label),TextFont{font_size:bevy::text::FontSize::Px(17.),..default()},TextColor(Color::srgb(0.95,0.92,0.85)),
            Node{position_type:PositionType::Absolute,left:px(150),bottom:px(10),padding:UiRect::axes(px(10),px(4)),..default()},BackgroundColor(Color::srgba(0.,0.,0.,0.6)),
            GlobalZIndex(30000),if menu{Visibility::Visible}else{Visibility::Hidden}));}
    }
}
fn control(keys:Res<ButtonInput<KeyCode>>,mut pause:ResMut<Pause>,mut time:ResMut<Time<Virtual>>,mut sound:ResMut<super::audio::PortAudio>,real:Res<Time<Real>>,ui:Res<PauseUi>,intro:Res<super::intro::Intro>,play:Res<super::Play>,mut saves:ResMut<save::Saves>){
    if intro.controls_blocked(){return;}
    // Delay only the optional smoke script, allowing natural combat to reach
    // a requested HP/animation boundary before its ordinary pause sequence.
    if pause.smoke&&pause.frame==0&&play.tick<std::env::var("MW_PAUSE_SMOKE_WAIT_TICKS").ok().and_then(|v|v.parse::<u64>().ok()).unwrap_or(0){return;}
    pause.frame+=1;
    let menu_step=pause.menu_step(&real);
    let mode=std::env::var("MW_PAUSE_SMOKE_PAGE").unwrap_or_default();
    let scripted=pause.smoke&&matches!(mode.as_str(),"options"|"controls"|"sound"|"video"|"playlog"|"title-cancel"|"title-return"|"title-slots"|"title-save");let title_script=scripted&&mode.starts_with("title-");let sound_script=scripted&&mode=="sound";
    let frame=pause.frame;
    let back=keys.just_pressed(KeyCode::Escape)||keys.just_pressed(KeyCode::Backspace)
        ||(scripted&&frame==170&&!sound_script&&!title_script)||(scripted&&matches!(mode.as_str(),"controls"|"video")&&frame==160)
        ||(sound_script&&matches!(frame,160|220))||(title_script&&mode=="title-cancel"&&frame==220);
    let enter=keys.just_pressed(KeyCode::Enter)||(scripted&&frame==110)||(scripted&&!matches!(mode.as_str(),"options"|"playlog")&&frame==120)||(sound_script&&frame==210)||(title_script&&frame==170);
    let down=keys.just_pressed(KeyCode::ArrowDown)||(scripted&&mode!="playlog"&&frame==100)||(scripted&&matches!(mode.as_str(),"sound"|"video")&&frame==115)||(scripted&&mode=="video"&&frame==117)||(title_script&&matches!(frame,115|117|119));
    let up=keys.just_pressed(KeyCode::ArrowUp);
    if pause.open&&pause.menu.page==PausePage::TitleSplash{
        pause.title_age+=menu_step;
        if (enter||(matches!(mode.as_str(),"title-slots"|"title-save")&&frame==280))&&pause.title_age>=90.{saves.reload();pause.menu.page=PausePage::SaveSlots;pause.save_menu=Default::default();pause.slots=Some(SaveSelection::default());}
        return;
    }
    if pause.open&&matches!(pause.menu.page,PausePage::SaveSlots|PausePage::SlotActions|PausePage::SaveConfirm){
        if keys.just_pressed(KeyCode::F5){saves.reload();pause.save_menu=Default::default();}
        if pause.smoke&&mode=="title-save"&&frame==590{saves.reload();assert!(saves.store.is_some()&&saves.error.is_none());}
        let down=down||(mode=="title-slots"&&matches!(frame,320|330|390|400|415|425|435));
        let up=up||(mode=="title-slots"&&matches!(frame,315|340|445));
        if saves.store.is_some(){
            let mut input=mw_sim::ui_save::Input{vertical:down as i32-up as i32,horizontal:keys.just_pressed(KeyCode::ArrowRight)as i32-keys.just_pressed(KeyCode::ArrowLeft)as i32,confirm:enter||(mode=="title-slots"&&frame==350),back:back||(mode=="title-slots"&&frame==380)};
            if pause.smoke&&mode=="title-save"{input.confirm|=matches!(frame,350|390|430|460|530|560|630|670|700);if matches!(frame,380|425|660|665){input.vertical=1;}if matches!(frame,560|700){input.horizontal=-1;}}
            match pause.save_menu.advance(menu_step,input,saves.occupied()){
                Some(mw_sim::ui_save::Intent::Write(operation))=>saves.apply(operation),
                Some(mw_sim::ui_save::Intent::Start{..})=>saves.error=Some("Starting a saved game is not available yet.".into()),None=>{},
            }
        }
        if pause.smoke&&mode=="title-save"&&matches!(frame,500|580|740){let data=saves.store.as_ref().expect("smoke save loaded").bundle();assert!(saves.error.is_none(),"{:?}",saves.error);if frame==500{assert_eq!(data.records.slot(1).unwrap().as_bytes()[0xbeef],0xbb,"No changed destination");}else if frame==580{assert_eq!(data.records.slot(0).unwrap().as_bytes(),data.records.slot(1).unwrap().as_bytes());assert_eq!(data.options.profile(1).unwrap()[1],0x7b,"Copy changed padding");}else{assert_eq!(data.records.slot(1).unwrap().as_bytes(),save::RESET);}eprintln!("[save-smoke] frame{frame}: cancel/copy/erase byte checks passed");}
        pause.slots=Some(pause.save_menu.selection.clone());
        pause.menu.page=match pause.save_menu.page{mw_sim::ui_save::Page::Slots=>PausePage::SaveSlots,mw_sim::ui_save::Page::Actions=>PausePage::SlotActions,mw_sim::ui_save::Page::Confirm=>PausePage::SaveConfirm};
        if mode=="title-slots"&&pause.smoke{
            if frame==365{assert_eq!(pause.menu.page,PausePage::SlotActions);}
            if frame==395{assert_eq!(pause.menu.page,PausePage::SaveSlots);assert_eq!(pause.slots.as_ref().unwrap().selected,1,"Opening gate accepted early selection");}
            if frame==440{assert_eq!(pause.slots.as_ref().unwrap().selected,3,"Slot selection did not clamp at4");}
        }
        return;
    }
    if pause.open&&pause.menu.page==PausePage::TitleConfirm{
        let prompt=pause.title.as_mut().expect("Title prompt");let outcome=prompt.advance(menu_step);
        if back{prompt.back();}else{
            if keys.just_pressed(KeyCode::ArrowLeft)||(title_script&&mode!="title-cancel"&&frame==160){prompt.choose(true);}
            if keys.just_pressed(KeyCode::ArrowRight){prompt.choose(false);}
            if enter{prompt.confirm();}
        }
        if let Some(yes)=outcome{pause.menu.page=if yes{PausePage::TitleSplash}else{PausePage::Options};pause.title_age=0.;pause.title=None;}
    }else if pause.open&&pause.menu.page==PausePage::Sound {
        let edit=pause.sound.as_mut().expect("Sound edit transaction");edit.advance(menu_step);
        let outcome=if back{edit.back()}else{
            if down{edit.step(1);}if up{edit.step(-1);}
            if keys.just_pressed(KeyCode::ArrowLeft)||(sound_script&&matches!(frame,130|205)){edit.adjust(-1);}
            if keys.just_pressed(KeyCode::ArrowRight){edit.adjust(1);}
            if enter{edit.confirm()}else{None}
        };
        sound.settings=edit.draft.clone();
        if let Some(outcome)=outcome{
            sound.settings=outcome.settings;if outcome.save{sound.save();}
            pause.menu.page=PausePage::Options;pause.sound=None;
        }
    }else{
        let toggle=(pause.smoke&&(frame==90||frame==if sound_script||title_script{240}else{180}))||back;
        if toggle {
            if !pause.open {
                if keys.just_pressed(KeyCode::Escape)||(pause.smoke&&frame==90){pause.open=true;pause.menu=PauseNavigation::default();pause.release_inputs=true;time.pause();}
            }else if pause.menu.back(){pause.open=false;pause.release_inputs=true;time.unpause();}
            return;
        }
        if pause.open{
            if down{pause.menu.step(1);}if up{pause.menu.step(-1);}
            if keys.just_pressed(KeyCode::ArrowLeft)||(scripted&&mode=="playlog"&&matches!(frame,140|155)){pause.menu.turn_page(-1,ui.messages.page_count());}
            if keys.just_pressed(KeyCode::ArrowRight)||(scripted&&mode=="playlog"&&frame==130){pause.menu.turn_page(1,ui.messages.page_count());}
            if scripted&&mode=="playlog"&&matches!(frame,131|141|156){assert_eq!(pause.menu.playlog_page,match frame{131=>1,141=>0,_=>9});eprintln!("[pause-smoke] Play Log page {}",pause.menu.playlog_page+1);}
            if enter{pause.menu.confirm();if pause.menu.page==PausePage::Sound{pause.sound=Some(SoundEdit::new(sound.settings.clone()));}if pause.menu.page==PausePage::TitleConfirm{pause.title=Some(TitlePrompt::default());}}
        }
    }
    if scripted&&frame==150{
        assert_eq!(pause.menu.page,match mode.as_str(){"controls"=>PausePage::Controls,"sound"=>PausePage::Sound,"video"=>PausePage::Video,"playlog"=>PausePage::PlayLog,"title-cancel"|"title-return"|"title-slots"|"title-save"=>PausePage::TitleConfirm,_=>PausePage::Options});
        eprintln!("[pause-smoke] page {:?}",pause.menu.page);
    }
    if scripted&&!matches!(mode.as_str(),"title-return"|"title-slots"|"title-save")&&frame==if sound_script||title_script{230}else{178}{assert_eq!(pause.menu.page,PausePage::Map);}
}

fn audio(mut pause:ResMut<Pause>,sinks:Query<(Entity,&AudioSink)>,spatial:Query<(Entity,&SpatialAudioSink)>){
    // Keep ownership of only those sinks this menu paused, including clips that finished loading later.
    // (the VR menu holds the world's sound as the pause does)
    let held=pause.open||super::vr::menu_open();
    for (entity,sink) in &sinks {
        if held {if !sink.is_paused(){sink.pause();pause.paused_audio.insert(entity);}}
        else if pause.paused_audio.remove(&entity){sink.play();}
    }
    for (entity,sink) in &spatial {
        if held {if !sink.is_paused(){sink.pause();pause.paused_audio.insert(entity);}}
        else if pause.paused_audio.remove(&entity){sink.play();}
    }
    pause.paused_audio.retain(|e|sinks.get(*e).is_ok()||spatial.get(*e).is_ok());
}

fn setup(mut commands:Commands,assets:Res<AssetServer>){
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source=native_ui::UiSource::load(&root,"subscr000_us");let common=native_ui::UiSource::load(&root,"core_us");
    let title_source=native_ui::UiSource::load(&root,"idpac_ce00_us");
    let map=mw_data::ui::StageMap::parse(&std::fs::read_to_string(root.join("assets/native/ui/c201-map/map.json")).unwrap()).unwrap();
    let messages=mw_data::ui_message::MessageCatalog::parse(&std::fs::read_to_string(root.join("assets/native/ui/playlog-us/catalog.json")).unwrap()).unwrap();
    let map_images=map.pages.iter().map(|p|assets.load(p.image.clone())).collect();
    commands.insert_resource(PauseUi{source,common,title_source,title_scenes:None,save_scenes:None,scenes:None,messages,map,map_images,selection:0,page:PausePage::Map,options:None,background:None,content:None,dialog:None,dialog_yes:false,option_selection:0,save_revision:0});
}
fn draw(mut commands:Commands,pause:Res<Pause>,play:Res<super::Play>,time:Res<Time<Real>>,assets:Res<AssetServer>,
    windows:Query<&Window>,mut ui:ResMut<PauseUi>,quads:Query<Entity,With<PauseQuad>>,saves:Res<save::Saves>){
    for e in &quads {commands.entity(e).despawn();}
    if !pause.open {ui.scenes=None;ui.options=None;ui.background=None;ui.content=None;ui.dialog=None;ui.title_scenes=None;ui.save_scenes=None;ui.page=PausePage::Map;return;}
    if ui.scenes.is_none(){
        let mut map=ui.source.scene(0);mw_sim::ui::initialize_pause_map(&mut map,play.fight.player.hp as f32/100.,play.continues,0);
        ui.scenes=Some([map,ui.source.scene(2),ui.common.scene(18)]);ui.selection=0;
    }
    if ui.selection!=pause.menu.map_selection {
        ui.scenes.as_mut().unwrap()[0].bind_position(15,if pause.menu.map_selection==0{10}else{20});
        ui.selection=pause.menu.map_selection;
    }
    if ui.save_revision!=saves.revision{ui.save_revision=saves.revision;ui.save_scenes=None;}
    if matches!(pause.menu.page,PausePage::SaveSlots|PausePage::SlotActions|PausePage::SaveConfirm)&&ui.save_scenes.is_none(){
        let mut frame=ui.title_source.scene(80);mw_sim::ui::initialize_save_frame(&mut frame);let mut rows=vec![];
        for slot in 0..4{let mut row=ui.title_source.scene(81);if let Some(store)=&saves.store{mw_sim::ui_save::initialize_slot(&mut row,&frame,slot,store.bundle().records.slot(slot).unwrap());}else{mw_sim::ui::initialize_empty_slot(&mut row,&frame,slot);}rows.push(row);}
        let mut scenes=vec![frame];scenes.extend(rows);ui.save_scenes=Some(scenes);
    }
    if ui.page!=pause.menu.page || (matches!(pause.menu.page,PausePage::SaveSlots|PausePage::SlotActions|PausePage::SaveConfirm)&&ui.save_scenes.is_none()) {
        match pause.menu.page {
            PausePage::Map=>{
                let map=&mut ui.scenes.as_mut().unwrap()[0];
                for id in [100,71,2,5]{map.hide_tree(id,false);}
                map.hide_tree(6,true);
                map.hide_tree(16,true);map.hide_tree(30,true);map.hide_tree(35,true);map.select_tree(10,false);
                ui.options=None;ui.background=None;ui.content=None;
            },
            PausePage::PlayLog=>{
                mw_sim::ui::initialize_playlog(&mut ui.scenes.as_mut().unwrap()[0]);
                ui.options=None;ui.background=None;ui.content=None;
            },
            PausePage::Options=>{
                let map=&mut ui.scenes.as_mut().unwrap()[0];for id in [100,71,2,6]{map.hide_tree(id,true);}
                map.hide_tree(5,false);
                if ui.options.is_none(){let mut options=ui.source.scene(61);mw_sim::ui::initialize_options(&mut options,pause.menu.options_selection);ui.options=Some(options);}
                if let Some(options)=ui.options.as_mut(){options.hide_tree(10,false);options.hide_tree(5,false);options.hide_tree(6,true);options.hide_tree(20,true);for id in [11,12,13,15]{options.select_tree(id,false);}}
                if ui.background.is_none(){ui.background=Some(ui.source.scene(7));}
                ui.content=None;ui.dialog=None;ui.option_selection=pause.menu.options_selection;
            },
            PausePage::Controls=>{
                let map=&mut ui.scenes.as_mut().unwrap()[0];map.hide_tree(5,true);map.hide_tree(6,false);map.restart_scale(6);
                if let Some(options)=ui.options.as_mut(){
                    options.hide_tree(10,true);options.hide_tree(11,false);
                    options.hide_tree(6,false);options.bind_position_axis(6,11,1);options.select_tree(11,true);
                }
                let mut controls=ui.source.scene(65);controls.hide_tree(15,false);ui.content=Some(controls);
            },
            PausePage::Sound=>{
                let map=&mut ui.scenes.as_mut().unwrap()[0];map.hide_tree(5,true);map.hide_tree(6,false);map.restart_scale(6);
                if let Some(options)=ui.options.as_mut(){options.hide_tree(10,false);options.hide_tree(6,false);options.bind_position_axis(6,12,1);options.select_tree(12,true);}
                let mut scene=ui.source.scene(70);mw_sim::ui::initialize_sound(&mut scene);ui.content=Some(scene);
            },
            PausePage::TitleConfirm=>{
                if let Some(options)=ui.options.as_mut(){options.hide_tree(6,false);options.bind_position_axis(6,15,1);options.select_tree(15,true);}
                let mut scene=ui.source.scene(85);scene.bind_position_axis(9,12,2);ui.dialog=Some(scene);ui.dialog_yes=false;
            },
            PausePage::TitleSplash=>{
                ui.dialog=None;ui.options=None;ui.background=None;ui.content=None;
                let logo=ui.title_source.scene(0);let mut prompt=ui.title_source.scene(2);
                mw_sim::ui::initialize_title_splash(&mut prompt);
                ui.title_scenes=Some([logo,prompt]);
            },
            PausePage::SaveSlots=>{
                ui.title_scenes=None;ui.content=None;ui.dialog=None;
                if ui.save_scenes.is_none(){let mut frame=ui.title_source.scene(80);mw_sim::ui::initialize_save_frame(&mut frame);let mut rows=vec![];
                    for slot in 0..4{let mut row=ui.title_source.scene(81);if let Some(store)=&saves.store{mw_sim::ui_save::initialize_slot(&mut row,&frame,slot,store.bundle().records.slot(slot).unwrap());}else{mw_sim::ui::initialize_empty_slot(&mut row,&frame,slot);}rows.push(row);}
                    let mut scenes=vec![frame];scenes.extend(rows);ui.save_scenes=Some(scenes);
                }
            },
            PausePage::SlotActions=>{
                let mut actions=ui.title_source.scene(83);mw_sim::ui_save::initialize_actions(&mut actions,saves.occupied()[pause.save_menu.selection.selected]);ui.content=Some(actions);
            },
            PausePage::SaveConfirm=>{
                let operation=pause.save_menu.operation.unwrap();
                if let mw_sim::ui_save::Operation::Erase{..}=operation{
                    let mut actions=ui.title_source.scene(83);mw_sim::ui_save::initialize_actions(&mut actions,true);actions.bind_position_axis(5,13,1);actions.hide_tree(6,false);actions.bind_position_axis(6,13,1);actions.select_tree(13,true);ui.content=Some(actions);
                }else{ui.content=None;}
                let mut dialog=ui.title_source.scene(82);mw_sim::ui_save::initialize_confirmation(&mut dialog,operation,saves.occupied());ui.dialog=Some(dialog);
            },
            PausePage::Video=>{
                let mut scene=ui.source.scene(76);mw_sim::ui::initialize_video(&mut scene);ui.content=Some(scene);
            },
        }
        ui.page=pause.menu.page;
    }
    if ui.option_selection!=pause.menu.options_selection {
        if let Some(options)=ui.options.as_mut(){options.bind_position_axis(5,[11,12,13,15][pause.menu.options_selection],1);}
        ui.option_selection=pause.menu.options_selection;
    }
    if let Some(prompt)=&pause.save_menu.prompt{if let Some(scene)=ui.dialog.as_mut(){mw_sim::ui::update_title_prompt(scene,prompt);}}
    if ui.page==PausePage::SlotActions{if let Some(scene)=ui.content.as_mut(){mw_sim::ui_save::update_actions(scene,pause.save_menu.action);}}
    if let Some(prompt)=&pause.title{
        if let Some(scene)=ui.dialog.as_mut(){mw_sim::ui::update_title_prompt(scene,prompt);}
    }
    if let Some(edit)=&pause.sound{
        if let Some(scene)=ui.content.as_mut(){mw_sim::ui::update_sound(scene,edit);}
        if let Some(yes)=edit.save_yes{
            if ui.dialog.is_none(){let mut dialog=ui.source.scene(86);dialog.digit(20,4);dialog.bind_position_axis(9,12,2);ui.dialog=Some(dialog);ui.dialog_yes=false;}
            if ui.dialog_yes!=yes{ui.dialog.as_mut().unwrap().bind_position_axis(9,if yes{11}else{12},2);ui.dialog_yes=yes;}
        }else{ui.dialog=None;}
    }
    if pause.menu.page==PausePage::PlayLog{
        let count=ui.messages.page_count();mw_sim::ui::update_playlog(&mut ui.scenes.as_mut().unwrap()[0],pause.menu.playlog_page,count);
    }
    let player=play.fight.player.pos;let projected=ui.map.project(player,[-80.,12.],250.);
    let layer=ui.map.layer(player[1]);let map_image=ui.map_images[layer].clone();
    let scenes=ui.scenes.as_mut().unwrap();
    for scene in scenes.iter_mut(){scene.advance(pause.menu_step(&time));}
    scenes[2].states[0].position=Some(projected);
    scenes[2].states[0].rotation=Some(180.-play.fight.player.loco.facing.to_degrees());
    let Ok(window)=windows.single()else{return;};let size=Vec2::new(window.width(),window.height());
    commands.spawn((Node{position_type:PositionType::Absolute,width:percent(100),height:percent(100),..default()},BackgroundColor(Color::BLACK),GlobalZIndex(19000),PauseQuad));
    if let Some(scene)=&mut ui.background{scene.advance(pause.menu_step(&time));}
    if let Some(scene)=&mut ui.options{scene.advance(pause.menu_step(&time));}
    if let Some(scene)=&mut ui.content{scene.advance(pause.menu_step(&time));}
    if let Some(scene)=&mut ui.dialog{scene.advance(pause.menu_step(&time));}
    let scenes=ui.scenes.as_ref().unwrap();
    for (i,scene)in scenes.iter().enumerate(){
        if matches!(ui.page,PausePage::Video|PausePage::TitleSplash|PausePage::SaveSlots|PausePage::SlotActions|PausePage::SaveConfirm)||(ui.page!=PausePage::Map&&!(ui.page==PausePage::PlayLog&&i==1)&&i>0){continue;}
        let layout=if i==2{&ui.common.layout}else{&ui.source.layout};
        for sprite in native_ui::sprites(scene,layout,&ui.common.layout,&assets,size,20000+i as i32*1000,if i==0{Some((100,map_image.clone()))}else{None}){
            commands.spawn((sprite,PauseQuad));
        }
    }
    if ui.page==PausePage::TitleSplash{
        if let Some(scenes)=ui.title_scenes.as_mut(){for scene in scenes{scene.advance(pause.menu_step(&time));}}
        for (i,scene)in ui.title_scenes.as_ref().unwrap().iter().enumerate(){
            for sprite in native_ui::sprites(scene,&ui.title_source.layout,&ui.common.layout,&assets,size,20000+i as i32*2000,None){commands.spawn((sprite,PauseQuad));}
        }return;
    }
    if matches!(ui.page,PausePage::SaveSlots|PausePage::SlotActions|PausePage::SaveConfirm){
        let selected=pause.slots.as_ref().unwrap().selected;
        if let Some(scenes)=ui.save_scenes.as_mut(){let(frame,rows)=scenes.split_first_mut().unwrap();mw_sim::ui::update_save_selection(frame,rows,selected);frame.hide_tree(40,pause.save_menu.copy_from.is_none());if let Some(from)=pause.save_menu.copy_from{rows[from].color_preset(1,1);}if let Some(mw_sim::ui_save::Operation::Erase{slot})=pause.save_menu.operation{rows[slot].color_preset(1,1);}for scene in scenes{scene.advance(pause.menu_step(&time));}}
        for (i,scene)in ui.save_scenes.as_ref().unwrap().iter().enumerate(){
            for sprite in native_ui::sprites(scene,&ui.title_source.layout,&ui.common.layout,&assets,size,if i==0{20000}else{19800},None){commands.spawn((sprite,PauseQuad));}
        }
        if let Some(actions)=&ui.content{for sprite in native_ui::sprites(actions,&ui.title_source.layout,&ui.common.layout,&assets,size,20200,None){commands.spawn((sprite,PauseQuad));}}
        if let Some(dialog)=&ui.dialog{for sprite in native_ui::sprites(dialog,&ui.title_source.layout,&ui.common.layout,&assets,size,21000,None){commands.spawn((sprite,PauseQuad));}}
        if let Some(error)=&saves.error{commands.spawn((Text::new(error),TextFont{font_size:18.0.into(),..default()},TextColor(Color::WHITE),Node{position_type:PositionType::Absolute,left:px(30),bottom:px(15),..default()},GlobalZIndex(32000),PauseQuad));}
        return;
    }
    if ui.page==PausePage::PlayLog{
        let map=&scenes[0];
        for row in 0..5{for column in 0..2{
            let message=pause.menu.playlog_page*10+row*2+column;if message>=ui.messages.messages.len(){continue;}
            let id=[91,96,106,111,116][row]+column as u32;let w=&map.program.widgets[map.index(id).unwrap()];
            let at=w.record_offset+0x40;let bytes=&ui.source.bytes;
            let dimensions=[0,1,2].map(|axis|i16::from_be_bytes(bytes[at+axis*2..at+axis*2+2].try_into().unwrap())as f32);
            let anchor=[w.root_position[0]+320.,w.root_position[1]+180.];
            // Stage object/scoring statistics are not ported yet; native initial counters are zero.
            let text=mw_sim::ui_message::layout(&ui.messages,message,anchor,dimensions,&[0;46]).unwrap();
            for sprite in native_ui::text_sprites(&text,&ui.messages,&assets,size,29999){commands.spawn((sprite,PauseQuad));}
        }}
    }
    for (layer,scene)in [(-400,ui.background.as_ref()),(11000,ui.options.as_ref()),(-200,ui.content.as_ref()),(16000,ui.dialog.as_ref())]{
        if ui.page==PausePage::Video&&matches!(layer,-400|11000){continue;}
        if let Some(scene)=scene{for sprite in native_ui::sprites(scene,&ui.source.layout,&ui.common.layout,&assets,size,20000+layer,None){commands.spawn((sprite,PauseQuad));}}
    }

}
