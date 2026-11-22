//! Rust-owned native save.dat/option.dat persistence. A write-ahead journal
//! completes the pair after interruption; native file bytes stay unchanged.
use crate::save::{OptionFile,SaveBundle,SaveFile,SAVE_BYTES,OPTION_BYTES};
use std::{fs::{self,File,OpenOptions},io::{self,Write},path::{Path,PathBuf},sync::atomic::{AtomicU64,Ordering}};
const JOURNAL:&str="pending-save.bin";
static NEXT:AtomicU64=AtomicU64::new(0);
pub struct SaveStore{directory:PathBuf,bundle:SaveBundle,loaded:Option<(Vec<u8>,Vec<u8>)>}
impl SaveStore{
    pub fn open(directory:impl Into<PathBuf>,reset:&[u8])->Result<Self,String>{
        let directory=directory.into();
        let _guard=if directory.try_exists().map_err(error)?{Some(lock(&directory)?)}else{None};
        if directory.join(JOURNAL).try_exists().map_err(error)?{
            if let Some(bytes)=read_optional(&directory.join(JOURNAL))?{let bundle=decode_journal(&bytes)?;finish(&directory,&bundle)?;}
        }
        let loaded=read_pair(&directory)?;
        let bundle=if let Some((records,options))=&loaded{SaveBundle{records:SaveFile::parse(records)?,options:OptionFile::parse(options)?}}else{SaveBundle::empty(reset)?};
        Ok(Self{directory,bundle,loaded})
    }
    pub fn bundle(&self)->&SaveBundle{&self.bundle}
    pub fn directory(&self)->&Path{&self.directory}
    /// Commit a complete native pair. UI confirmation precedes calling this.
    /// Memory changes only after both native files have been published.
    pub fn commit(&mut self,next:SaveBundle)->Result<(),String>{
        fs::create_dir_all(&self.directory).map_err(error)?;let _guard=lock(&self.directory)?;
        if self.directory.join(JOURNAL).try_exists().map_err(error)?{return Err("An unfinished save must be reloaded before another operation".into());}
        if read_pair(&self.directory)?!=self.loaded{return Err("Save data changed since it was loaded".into());}
        replace(&self.directory.join(JOURNAL),&encode_journal(&next))?;
        finish(&self.directory,&next)?;
        self.loaded=Some((next.records.as_bytes().to_vec(),next.options.as_bytes().to_vec()));self.bundle=next;Ok(())
    }
}
fn error(e:io::Error)->String{e.to_string()}
fn read_optional(path:&Path)->Result<Option<Vec<u8>>,String>{match fs::read(path){Ok(data)=>Ok(Some(data)),Err(e)if e.kind()==io::ErrorKind::NotFound=>Ok(None),Err(e)=>Err(error(e))}}
fn read_pair(directory:&Path)->Result<Option<(Vec<u8>,Vec<u8>)>,String>{match(read_optional(&directory.join("save.dat"))?,read_optional(&directory.join("option.dat"))?){(None,None)=>Ok(None),(Some(a),Some(b))=>Ok(Some((a,b))),_=>Err("Native save.dat and option.dat must both be present".into())}}
fn lock(directory:&Path)->Result<File,String>{let file=OpenOptions::new().create(true).truncate(false).write(true).open(directory.join("save.lock")).map_err(error)?;file.try_lock().map_err(|e|format!("Save data is busy: {e}"))?;Ok(file)}
fn replace(path:&Path,bytes:&[u8])->Result<(),String>{
    let parent=path.parent().ok_or("Save file has no parent")?;let name=path.file_name().ok_or("Save filename")?.to_string_lossy();
    let(mut file,temp)=loop{let temp=parent.join(format!(".{name}.{}.{}.tmp",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));match OpenOptions::new().write(true).create_new(true).open(&temp){Ok(file)=>break(file,temp),Err(e)if e.kind()==io::ErrorKind::AlreadyExists=>continue,Err(e)=>return Err(error(e))}};
    let result=(||{file.write_all(bytes).map_err(error)?;file.sync_all().map_err(error)?;drop(file);fs::rename(&temp,path).map_err(error)})();
    if result.is_err(){let _=fs::remove_file(&temp);}result
}
fn finish(directory:&Path,bundle:&SaveBundle)->Result<(),String>{replace(&directory.join("option.dat"),bundle.options.as_bytes())?;replace(&directory.join("save.dat"),bundle.records.as_bytes())?;fs::remove_file(directory.join(JOURNAL)).map_err(error)}
fn checksum(bytes:&[u8])->u64{bytes.iter().fold(0xcbf29ce484222325,|hash,byte|(hash^*byte as u64).wrapping_mul(0x100000001b3))}
fn encode_journal(bundle:&SaveBundle)->Vec<u8>{let mut bytes=b"MWRSAVE1".to_vec();bytes.extend_from_slice(bundle.records.as_bytes());bytes.extend_from_slice(bundle.options.as_bytes());let sum=checksum(&bytes);bytes.extend_from_slice(&sum.to_be_bytes());bytes}
fn decode_journal(bytes:&[u8])->Result<SaveBundle,String>{
    let size=8+SAVE_BYTES+OPTION_BYTES;
    if bytes.len()!=size+8||&bytes[..8]!=b"MWRSAVE1"||checksum(&bytes[..size])!=u64::from_be_bytes(bytes[size..].try_into().unwrap()){return Err("Invalid pending save transaction".into());}
    Ok(SaveBundle{records:SaveFile::parse(&bytes[8..8+SAVE_BYTES])?,options:OptionFile::parse(&bytes[8+SAVE_BYTES..size])?})
}
#[cfg(test)]mod tests{
    use super::*;
    const RESET:&[u8]=include_bytes!("../../../assets/native/save/reset-record.bin");
    struct Temp(PathBuf);impl Temp{fn new()->Self{let dir=std::env::temp_dir().join(format!("mw-rust-save-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));fs::create_dir(&dir).unwrap();Self(dir)}}
    impl Drop for Temp{fn drop(&mut self){for entry in fs::read_dir(&self.0).unwrap(){let path=entry.unwrap().path();if path.is_file(){let _=fs::remove_file(path);}}let _=fs::remove_dir(&self.0);}}
    #[test]fn native_pair_roundtrips_and_preserves_record_and_profile_bytes(){
        let dir=Temp::new();let mut store=SaveStore::open(&dir.0,RESET).unwrap();assert!(!dir.0.join("save.dat").exists());let mut data=store.bundle().clone();data.start_new_game(0,RESET,1,false).unwrap();store.commit(data.clone()).unwrap();
        assert_eq!(fs::read(dir.0.join("save.dat")).unwrap(),data.records.as_bytes());assert_eq!(fs::read(dir.0.join("option.dat")).unwrap(),data.options.as_bytes());assert!(!dir.0.join(JOURNAL).exists());
        let mut reopen=SaveStore::open(&dir.0,RESET).unwrap();assert_eq!(reopen.bundle(),&data);data.copy_slot(0,3).unwrap();reopen.commit(data.clone()).unwrap();assert_eq!(SaveStore::open(&dir.0,RESET).unwrap().bundle(),&data);
    }
    #[test]fn pending_pair_is_completed_after_an_interrupted_second_write(){
        let dir=Temp::new();let mut store=SaveStore::open(&dir.0,RESET).unwrap();store.commit(store.bundle().clone()).unwrap();let mut next=store.bundle().clone();next.start_new_game(2,RESET,1,false).unwrap();
        replace(&dir.0.join(JOURNAL),&encode_journal(&next)).unwrap();replace(&dir.0.join("option.dat"),next.options.as_bytes()).unwrap();
        let reopen=SaveStore::open(&dir.0,RESET).unwrap();assert_eq!(reopen.bundle(),&next);assert_eq!(fs::read(dir.0.join("save.dat")).unwrap(),next.records.as_bytes());assert!(!dir.0.join(JOURNAL).exists());
    }
    #[test]fn malformed_or_stale_files_are_preserved_instead_of_replaced(){
        let dir=Temp::new();fs::write(dir.0.join("save.dat"),b"bad").unwrap();assert!(SaveStore::open(&dir.0,RESET).is_err());assert_eq!(fs::read(dir.0.join("save.dat")).unwrap(),b"bad");fs::remove_file(dir.0.join("save.dat")).unwrap();
        let mut first=SaveStore::open(&dir.0,RESET).unwrap();let mut second=SaveStore::open(&dir.0,RESET).unwrap();let mut next=first.bundle().clone();next.start_new_game(1,RESET,1,false).unwrap();first.commit(next.clone()).unwrap();assert!(second.commit(second.bundle().clone()).is_err());assert_eq!(SaveStore::open(&dir.0,RESET).unwrap().bundle(),&next);
        fs::write(dir.0.join(JOURNAL),b"broken").unwrap();assert!(SaveStore::open(&dir.0,RESET).is_err());assert_eq!(fs::read(dir.0.join("save.dat")).unwrap(),next.records.as_bytes());
    }
}
