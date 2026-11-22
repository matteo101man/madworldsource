//! Original optional MESS assets shared by playable entry and message inspection.
use bevy::prelude::*;
use mw_data::{event_message::EventMessages,ui_message::MessageCatalog};
use mw_sim::event_message::{self,Controller};
pub struct Program{pub bank:EventMessages,pub catalog:Option<MessageCatalog>,controller:Controller,images:Vec<Handle<Image>>}
impl Program{
    pub fn load(root:&std::path::Path,event:u32,source_hash:Option<&str>)->Self{
        let path=root.join(format!("assets/native/events/ev{event:02x}/messages/messages.json"));
        let bank=EventMessages::parse(&std::fs::read_to_string(path).expect("staged event message manifest")).expect("valid original MESS manifest");assert_eq!(bank.event,event);
        if let Some(hash)=source_hash{assert_eq!(bank.source_sha256,hash,"Event/MESS source disagreement");}
        let catalog=bank.track.as_ref().map(|t|MessageCatalog::parse(&std::fs::read_to_string(root.join("assets").join(&t.catalog)).expect("staged event glyph catalog")).expect("valid original message glyphs"));
        let controller=Controller::new(event_message::cues(&bank,catalog.as_ref().map_or(0,|c|c.messages.len())).expect("native MESS commands"));
        Self{bank,catalog,controller,images:vec![]}
    }
    pub fn preload(&mut self,assets:&AssetServer){if let Some(catalog)=&self.catalog{self.images=catalog.pages.iter().map(|p|assets.load(p.image.clone())).collect();}}
    pub fn ready(&self,assets:&AssetServer)->bool{self.images.iter().all(|h|assets.is_loaded_with_dependencies(h.id()))}
    pub fn advance(&mut self,frame:u32,enabled:bool){self.controller.advance(frame,enabled).expect("forward native message clock");}
    pub fn selected(&self,enabled:bool)->Option<usize>{self.controller.visible(enabled)}
    pub fn clear(&mut self){self.controller.clear();}
    pub fn quads(&self,enabled:bool)->Vec<mw_sim::ui_message::TextQuad>{
        let Some(id)=self.selected(enabled)else{return vec![];};let catalog=self.catalog.as_ref().expect("selected original MESS catalog");
        let spacing=20.*self.bank.track.as_ref().unwrap().line_height as f32/catalog.base_advance;
        mw_sim::ui_message::event_lines(catalog,id,[320.,330.],[20.,20.],spacing).expect("supported original event text layout")
    }
}
