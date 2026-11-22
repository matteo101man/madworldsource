//! Original MESS-only inspection. It does not replace a cutscene or world flow.
//! MW_MESSAGE_EVENT=00 MW_MESSAGE_FRAME=170 MW_MESSAGE_CAPTURE=<png>.
use bevy::{prelude::*,winit::WinitSettings};
#[allow(dead_code)]#[path="../native_ui.rs"]mod native_ui;
#[path="../event_message_ui.rs"]mod event_message_ui;
#[derive(Resource)]struct Preview{program:event_message_ui::Program,frame:u32,enabled:bool,age:u32}
#[derive(Component)]struct Glyph;
fn main(){
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let event=u32::from_str_radix(&std::env::var("MW_MESSAGE_EVENT").unwrap_or_else(|_|"00".into()),16).expect("event hex id");
    let frame=std::env::var("MW_MESSAGE_FRAME").ok().map(|v|v.parse::<u32>().expect("event source frame")).unwrap_or(20);
    let program=event_message_ui::Program::load(&root,event,None);
    let enabled=std::env::var("MW_MESSAGE_ENABLED").as_deref()!=Ok("0");
    App::new().insert_resource(Preview{program,frame,enabled,age:0}).insert_resource(ClearColor(Color::BLACK)).insert_resource(WinitSettings::continuous())
        .add_plugins(DefaultPlugins.set(AssetPlugin{file_path:root.join("assets").to_string_lossy().into_owned(),..default()}).set(WindowPlugin{primary_window:Some(Window{title:format!("MadWorld original event {event:02x} messages"),resolution:(1280,720).into(),visible:std::env::var_os("MW_MESSAGE_CAPTURE").is_none(),..default()}),..default()}))
        .add_systems(Startup,setup).add_systems(Update,present).run();
}
fn setup(mut commands:Commands,assets:Res<AssetServer>,mut p:ResMut<Preview>){commands.spawn(Camera2d);p.program.preload(&assets);}
fn present(mut commands:Commands,assets:Res<AssetServer>,mut p:ResMut<Preview>,keys:Res<ButtonInput<KeyCode>>,windows:Query<&Window>,glyphs:Query<Entity,With<Glyph>>,mut exit:MessageWriter<AppExit>){
    if keys.just_pressed(KeyCode::Escape){p.program.clear();exit.write(AppExit::Success);}
    if !p.program.ready(&assets){return;}
    if keys.just_pressed(KeyCode::KeyM){p.enabled=!p.enabled;}
    if keys.just_pressed(KeyCode::ArrowRight){p.frame+=1;}
    let(frame,enabled)=(p.frame,p.enabled);p.program.advance(frame,enabled);
    for entity in &glyphs{commands.entity(entity).despawn();}
    let quads=p.program.quads(enabled);let window=windows.single().unwrap();
    if let Some(catalog)=&p.program.catalog{for sprite in native_ui::text_sprites(&quads,catalog,&assets,Vec2::new(window.width(),window.height()),1000){commands.spawn((sprite,Glyph));}}
    p.age+=1;
    if p.age==60{
        eprintln!("[message-preview] event={:02x} frame={frame} enabled={enabled} selected={:?} glyphs={}",p.program.bank.event,p.program.selected(enabled),quads.len());
        if let Some(path)=std::env::var_os("MW_MESSAGE_CAPTURE"){commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(std::path::PathBuf::from(path)));}
    }
    if p.age>=100&&std::env::var_os("MW_MESSAGE_CAPTURE").is_some(){exit.write(AppExit::Success);}
}
