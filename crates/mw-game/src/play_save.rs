//! Rust-owned save files shared by the original title widgets.
use bevy::prelude::*;
use mw_data::save_store::SaveStore;
pub const RESET:&[u8]=include_bytes!("../../../assets/native/save/reset-record.bin");
#[derive(Resource)]pub struct Saves{pub store:Option<SaveStore>,pub error:Option<String>,pub revision:u64,directory:std::path::PathBuf}
impl Saves{
    pub fn load()->Self{
        let directory=std::env::var_os("MW_RUST_SAVE_DIR").map(std::path::PathBuf::from).unwrap_or_else(||std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../rust-user")).join("MadWorld/Rust/save"));
        let mut saves=Self{store:None,error:None,revision:0,directory};saves.reload();saves
    }
    pub fn reload(&mut self){match SaveStore::open(&self.directory,RESET){Ok(store)=>{self.store=Some(store);self.error=None;self.revision+=1;},Err(error)=>{self.store=None;self.error=Some(format!("Unable to load save data: {error}. Press F5 to retry."));}}}
    pub fn occupied(&self)->[bool;4]{std::array::from_fn(|i|self.store.as_ref().is_some_and(|s|s.bundle().records.slot(i).unwrap().occupied()))}
    pub fn apply(&mut self,operation:mw_sim::ui_save::Operation){
        let result=(||{let store=self.store.as_mut().ok_or("Save data is unavailable")?;let mut next=store.bundle().clone();match operation{mw_sim::ui_save::Operation::Copy{from,to}=>next.copy_slot(from,to)?,mw_sim::ui_save::Operation::Erase{slot}=>next.erase_slot(slot,RESET,1,false)?};store.commit(next)})();
        match result{Ok(())=>{self.error=None;self.revision+=1;eprintln!("[save] committed {operation:?} in {}",self.directory.display());},Err(error)=>{self.error=Some(format!("Unable to save: {error}. Press F5 to reload before retrying."));eprintln!("[save] {}",self.error.as_ref().unwrap());}}
    }
}
