//! Shared original IDT/TUV presenter. Coordinates are native logical 640x360.
use bevy::prelude::*;
use mw_data::ui::{UiLayout,UiTimeline};
use mw_sim::ui::UiScene;
use std::path::Path;
pub type UiSprite=(Node,ImageNode,UiTransform,GlobalZIndex);
pub struct UiSource {pub layout:UiLayout,pub bytes:Vec<u8>,pub timeline:UiTimeline}
impl UiSource {
    pub fn load(root:&Path,key:&str)->Self{
        let folder=root.join("assets/native/ui").join(key);
        let layout=UiLayout::parse(&std::fs::read_to_string(folder.join("layout.json")).expect("native UI layout")).unwrap();
        let bytes=std::fs::read(folder.join("animation.idt")).expect("native UI IDT");
        let timeline=UiTimeline::parse(&bytes).expect("native UI hierarchy");Self{layout,bytes,timeline}
    }
    /// An IDT-only resource (tools/ui/stage_native_ui.py IDT_ONLY): its programs draw with `pages_from`'s pages/TUV.
    #[allow(dead_code)]
    pub fn load_idt(root:&Path,key:&str,pages_from:&str)->Self{
        let layout=UiLayout::parse(&std::fs::read_to_string(root.join("assets/native/ui").join(pages_from).join("layout.json")).expect("native UI layout")).unwrap();
        let bytes=std::fs::read(root.join("assets/native/ui").join(key).join("animation.idt")).expect("native UI IDT");
        let timeline=UiTimeline::parse(&bytes).expect("native UI hierarchy");Self{layout,bytes,timeline}
    }
    pub fn scene(&self,program:u32)->UiScene{UiScene::new(self.timeline.program(program).expect("native UI program"),&self.bytes,[320.,180.]).unwrap()}
    /// The scene, or why this port cannot build it (an event's program it does not read yet).
    pub fn try_scene(&self,program:u32)->Result<UiScene,String>{UiScene::new(self.timeline.program(program).ok_or("no such program")?,&self.bytes,[320.,180.]).map_err(|e|e.to_string())}
}
pub fn sprites(scene:&UiScene,layout:&UiLayout,common:&UiLayout,assets:&AssetServer,viewport:Vec2,z:i32,
    override_image:Option<(u32,Handle<Image>)>)->Vec<UiSprite>{sprites_ex(scene,layout,common,assets,viewport,z,override_image,false)}

/// As `sprites`; with `clip` the kind-7 widgets act as windows: an unrotated quad under one is cut to its rect (the clash
/// cut-in's portrait panels, program 11 w2 / w13) [inferred: how the original draws kind 7 is unresolved].
pub fn sprites_ex(scene:&UiScene,layout:&UiLayout,common:&UiLayout,assets:&AssetServer,viewport:Vec2,z:i32,
    override_image:Option<(u32,Handle<Image>)>,clip:bool)->Vec<UiSprite>{
    let factor=(viewport.x/640.).min(viewport.y/360.);let offset=(viewport-Vec2::new(640.,360.)*factor)*0.5;
    let mut sprites=vec![];
    let draws=scene.draw();
    // the windows: each visible kind-7 widget's rect (640 x 360 units), by widget index
    let windows:std::collections::HashMap<usize,[f32;4]>=if clip{draws.iter().filter(|d|d.kind==7&&d.visible).filter_map(|d|{
        let t=d.tiles();let t=t.first()?;Some((d.index,[t.center[0]-t.size[0]*0.5,t.center[1]-t.size[1]*0.5,t.center[0]+t.size[0]*0.5,t.center[1]+t.size[1]*0.5]))}).collect()}else{Default::default()};
    let window_of=|mut i:usize|->Option<[f32;4]>{while let Some(p)=scene.program.widgets[i].parent{if let Some(r)=windows.get(&p){return Some(*r);}i=p;}None};
    for d in draws{
        if !d.visible||!matches!(d.kind,0|8)||d.blend_mode!=0||d.tile_kind>1{continue;}
        let mut image=if let Some((id,handle))=&override_image {if d.id==*id{Some(ImageNode::new(handle.clone()))}else{None}}else{None};
        if image.is_none(){
            if d.uv[0]==255 {image=Some(ImageNode::solid_color(Color::WHITE));}
            else {
                let resource=if layout.groups.iter().any(|g|g.id==d.uv[0]){layout}else{common};
                let Some(group)=resource.groups.iter().find(|g|g.id==d.uv[0])else{continue;};
                if d.uv[1]as usize>=group.count{continue;}
                let (page,s)=resource.sprite(group.first+d.uv[1]as usize).unwrap();
                let image_path=if d.inverted{page.inverted_image.as_ref().unwrap_or(&page.image)}else{&page.image};
                let mut node=ImageNode::new(assets.load(image_path.clone()));
                node.rect=Some(Rect::new(s.rect[0]as f32,s.rect[1]as f32,s.rect[2]as f32,s.rect[3]as f32));image=Some(node);
            }
        }
        let mut image=image.unwrap();image.color=Color::srgba(d.color[0],d.color[1],d.color[2],d.color[3]);
        image.flip_x=d.scale[0]<0.;image.flip_y=d.scale[1]<0.;
        // Auto imposes intrinsic image sizing; every native quad explicitly specifies its dimensions.
        image.image_mode=NodeImageMode::Stretch;
        let window=if clip&&d.rotation.abs()<1e-4{window_of(d.index)}else{None};
        for tile in d.tiles(){
            let mut piece=image.clone();
            let mut uvf=tile.uv_fraction;let (mut c,mut sz)=(tile.center,tile.size);
            if let Some(w)=window{
                // cut the quad to the window, its UVs with it (mirrored axes run the other way)
                let (x0,y0,x1,y1)=(c[0]-sz[0]*0.5,c[1]-sz[1]*0.5,c[0]+sz[0]*0.5,c[1]+sz[1]*0.5);
                let (nx0,ny0,nx1,ny1)=(x0.max(w[0]),y0.max(w[1]),x1.min(w[2]),y1.min(w[3]));
                if nx1<=nx0||ny1<=ny0{continue;}
                let fx=|v:f32|(v-x0)/(x1-x0);let fy=|v:f32|(v-y0)/(y1-y0);
                let (ua,ub)=(uvf[0],uvf[2]);let (va,vb)=(uvf[1],uvf[3]);
                let (u0,u1)=if d.scale[0]<0.{(ub-(ub-ua)*fx(nx1),ub-(ub-ua)*fx(nx0))}else{(ua+(ub-ua)*fx(nx0),ua+(ub-ua)*fx(nx1))};
                let (v0,v1)=if d.scale[1]<0.{(vb-(vb-va)*fy(ny1),vb-(vb-va)*fy(ny0))}else{(va+(vb-va)*fy(ny0),va+(vb-va)*fy(ny1))};
                uvf=[u0,v0,u1,v1];c=[(nx0+nx1)*0.5,(ny0+ny1)*0.5];sz=[nx1-nx0,ny1-ny0];
            }
            if let Some(rect)=piece.rect{let size=rect.size();piece.rect=Some(Rect::new(rect.min.x+size.x*uvf[0],rect.min.y+size.y*uvf[1],rect.min.x+size.x*uvf[2],rect.min.y+size.y*uvf[3]));}
            let size=Vec2::from_array(sz)*factor;let min=offset+Vec2::from_array(c)*factor-size*0.5;
            sprites.push((Node{position_type:PositionType::Absolute,left:px(min.x),top:px(min.y),width:px(size.x),height:px(size.y),..default()},
                piece,UiTransform::from_rotation(Rot2::radians(d.rotation)),GlobalZIndex(z+scene.program.widgets[d.index].order as i32)));
        }
    }
    sprites
}

/// Original message glyphs use MTD metric dimensions independently of UV crops.
#[allow(dead_code)] // The standalone quad preview does not present messages yet.
pub fn text_sprites(quads:&[mw_sim::ui_message::TextQuad],catalog:&mw_data::ui_message::MessageCatalog,assets:&AssetServer,viewport:Vec2,z:i32)->Vec<UiSprite>{
    let factor=(viewport.x/640.).min(viewport.y/360.);let offset=(viewport-Vec2::new(640.,360.)*factor)*0.5;
    quads.iter().map(|q|{
        let g=catalog.glyph(q.glyph).expect("validated native glyph");let page=&catalog.pages[g.page];
        let size=Vec2::from_array(q.size)*factor;let min=offset+Vec2::from_array(q.center)*factor-size*0.5;
        let mut image=ImageNode::new(assets.load(page.image.clone()));image.image_mode=NodeImageMode::Stretch;
        image.rect=Some(Rect::new(g.rect[0]as f32,g.rect[1]as f32,g.rect[2]as f32,g.rect[3]as f32));
        (Node{position_type:PositionType::Absolute,left:px(min.x),top:px(min.y),width:px(size.x),height:px(size.y),..default()},image,UiTransform::default(),GlobalZIndex(z))
    }).collect()
}

/// A line of the core_us small alphabet (UV group 38: A-Z = 0..25, '!' 26, '?' 27; "YOU LOSE" in subscr0ff program
/// 1 draws it 12x16 at a 10 px advance), left edge at `at` on the 640x360 layout, `scale` x that size. Spaces advance.
#[allow(dead_code)]
pub fn small_text(layout:&UiLayout,assets:&AssetServer,text:&str,at:Vec2,scale:f32,color:Color,viewport:Vec2,z:i32)->Vec<UiSprite>{
    let factor=(viewport.x/640.).min(viewport.y/360.);let offset=(viewport-Vec2::new(640.,360.)*factor)*0.5;
    let Some(group)=layout.groups.iter().find(|g|g.id==38)else{return vec![];};
    let mut x=at.x;let mut out=vec![];
    for c in text.chars(){
        let index=match c{'A'..='Z'=>Some(c as usize-'A' as usize),'!'=>Some(26),'?'=>Some(27),_=>None};
        if let Some(i)=index.filter(|i|*i<group.count){
            let (page,s)=layout.sprite(group.first+i).unwrap();
            let mut image=ImageNode::new(assets.load(page.image.clone()));image.image_mode=NodeImageMode::Stretch;image.color=color;
            image.rect=Some(Rect::new(s.rect[0]as f32,s.rect[1]as f32,s.rect[2]as f32,s.rect[3]as f32));
            let size=Vec2::new(12.,16.)*scale*factor;let min=offset+Vec2::new(x,at.y)*factor-Vec2::new(0.,size.y*0.5);
            out.push((Node{position_type:PositionType::Absolute,left:px(min.x),top:px(min.y),width:px(size.x),height:px(size.y),..default()},image,UiTransform::default(),GlobalZIndex(z)));
        }
        x+=10.*scale;
    }
    out
}
