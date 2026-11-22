//! Original IDT layout preview. This exercises the shared data boundary before game-flow integration.
//! MW_UI_RESOURCE=subscr000_us MW_UI_PROGRAM=0; left/right selects a native screen.
//! Animation, special widget kinds and native clipping are deliberately reported as pending.
use bevy::{prelude::*, winit::WinitSettings};
use std::{path::PathBuf};
#[path="../native_ui.rs"]mod native_ui;

#[derive(Resource)]
struct Preview { source:native_ui::UiSource, common:native_ui::UiSource, images:Vec<Handle<Image>>, frame:f32, index:usize, changed:bool, drawn:usize, skipped:usize }
#[derive(Component)]struct PreviewRoot;
#[derive(Component)]struct Label;
fn main() {
    let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let key=std::env::var("MW_UI_RESOURCE").unwrap_or_else(|_|"subscr000_us".into());
    assert!(key.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'),"resource key");
    let source=native_ui::UiSource::load(&root,&key);
    let common=native_ui::UiSource::load(&root,"core_us");
    let frame=std::env::var("MW_UI_FRAME").ok().and_then(|v|v.parse().ok()).unwrap_or(0.);
    let id=std::env::var("MW_UI_PROGRAM").ok().and_then(|v|v.parse().ok()).unwrap_or(0);
    let index=source.timeline.programs.iter().position(|p|p.id==id).expect("native screen ID");
    App::new().insert_resource(Preview{source,common,images:vec![],frame,index,changed:true,drawn:0,skipped:0})
        .insert_resource(WinitSettings::continuous()).insert_resource(ClearColor(Color::srgb(0.08,0.08,0.08)))
        .add_plugins(DefaultPlugins.set(AssetPlugin{file_path:root.join("assets").to_string_lossy().into_owned(),..default()})
            .set(WindowPlugin{primary_window:Some(Window{title:format!("MadWorld original UI: {key}"),resolution:(960,540).into(),visible:std::env::var_os("MW_UI_CAPTURE").is_none(),..default()}),..default()}))
        .add_systems(Startup,setup).add_systems(Update,(input,draw,capture).chain()).run();
}
fn setup(mut c:Commands,assets:Res<AssetServer>,mut p:ResMut<Preview>) {
    p.images=p.source.layout.pages.iter().chain(&p.common.layout.pages).map(|page|assets.load(page.image.clone())).collect();
    c.spawn(Camera2d);
    c.spawn((Text::new(""),TextFont{font_size:bevy::text::FontSize::Px(14.),..default()},
        Node{position_type:PositionType::Absolute,left:px(8),bottom:px(8),..default()},GlobalZIndex(10000),Label));
}
fn input(keys:Res<ButtonInput<KeyCode>>,mut p:ResMut<Preview>,mut exit:MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape){exit.write(AppExit::Success);}
    let d=if keys.just_pressed(KeyCode::ArrowRight){1}else if keys.just_pressed(KeyCode::ArrowLeft){-1}else{0};
    if keys.just_pressed(KeyCode::ArrowUp){p.frame+=1.;p.changed=true;}
    if keys.just_pressed(KeyCode::ArrowDown){p.frame=(p.frame-1.).max(0.);p.changed=true;}
    if d!=0 {p.index=(p.index as isize+d).rem_euclid(p.source.timeline.programs.len() as isize) as usize;p.changed=true;}
}
fn draw(mut c:Commands,assets:Res<AssetServer>,mut p:ResMut<Preview>,roots:Query<Entity,With<PreviewRoot>>,mut label:Query<&mut Text,With<Label>>) {
    if !p.changed{return;}
    for e in &roots{c.entity(e).despawn();}
    p.changed=false;
    let program=&p.source.timeline.programs[p.index];
    let mut scene=p.source.scene(program.id);
    if program.id==0 && std::env::var("MW_UI_RESOURCE").unwrap_or_else(|_|"subscr000_us".into())=="subscr000_us" {mw_sim::ui::initialize_pause_map(&mut scene,1.,0,0);}
    scene.seek(p.frame);
    let draws=scene.draw();
    let skipped=draws.iter().filter(|d|d.visible&&(!matches!(d.kind,0|8)||d.blend_mode!=0||d.tile_kind>1)).count();
    let sprites=native_ui::sprites(&scene,&p.source.layout,&p.common.layout,&assets,Vec2::new(960.,540.),0,None);
    let drawn=sprites.len();for sprite in sprites{c.spawn((sprite,PreviewRoot));}
    let text=format!("Original screen {} | quads {drawn} | unsupported special widgets {skipped}\nLeft/Right screen   Up/Down frame   Esc close | full animation/transforms/clipping parity pending | frame {}",program.id,p.frame);
    for mut label in &mut label{**label=text.clone();}
    p.drawn=drawn;p.skipped=skipped;
    eprintln!("[ui-preview] {text}");
}
fn capture(mut frame:Local<u32>,mut c:Commands,p:Res<Preview>,server:Res<AssetServer>,mut exit:MessageWriter<AppExit>) {
    *frame+=1;
    let Ok(path)=std::env::var("MW_UI_CAPTURE")else{return;};
    let loaded=p.images.iter().all(|image|server.is_loaded_with_dependencies(image.id()));
    if *frame==90 && !loaded {*frame=89;return;}
    if *frame==90 {
        eprintln!("[ui-preview] capture program={} drawn={} skipped={} assetsLoaded={loaded}",p.source.timeline.programs[p.index].id,p.drawn,p.skipped);
        c.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(bevy::render::view::screenshot::save_to_disk(path));
    }
    if *frame>=120{exit.write(AppExit::Success);}
}

