//! Native save.dat is four big-endian 0xC450-byte records. Keep undecoded bytes
//! intact: this codec does not synthesize progression or companion option data.
pub const RECORD_BYTES: usize = 0xC450;
pub const SLOT_COUNT: usize = 4;
pub const SAVE_BYTES: usize = RECORD_BYTES * SLOT_COUNT;
pub const OPTION_BYTES: usize = 8 * SLOT_COUNT;
const FLAGS: usize = 0x44E;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveFile { bytes: Vec<u8> }

#[derive(Clone, Copy, Debug)]
pub struct SaveSlot<'a> { bytes: &'a [u8] }

/// Native option.dat: four eight-byte profiles, including undecoded flags.
#[derive(Clone,Debug,PartialEq,Eq)]pub struct OptionFile{bytes:[u8;OPTION_BYTES]}
impl OptionFile{
    pub fn parse(bytes:&[u8])->Result<Self,String>{Ok(Self{bytes:bytes.try_into().map_err(|_|"option.dat must contain32 bytes")?})}
    /// Initial empty profiles observed independently in newgame-fs-fixed-v1.
    /// Language/initialized bits are assigned by8017FE88 when a slot is used.
    pub fn empty()->Self{let mut bytes=[0;OPTION_BYTES];for profile in bytes.chunks_exact_mut(8){profile.copy_from_slice(&[8,0,0xaa,0xaa,0,0x28,0,0]);}Self{bytes}}
    pub fn as_bytes(&self)->&[u8]{&self.bytes}
    pub fn profile(&self,index:usize)->Option<&[u8]>{let start=index.checked_mul(8)?;self.bytes.get(start..start.checked_add(8)?) }
    /// 8023DD84 copies byte0 and words2/4/6; destination padding byte1 survives.
    pub fn copy_slot(&mut self,from:usize,to:usize)->Result<(),String>{check_index(from)?;check_index(to)?;let source:[u8;8]=self.profile(from).unwrap().try_into().unwrap();let target=&mut self.bytes[to*8..to*8+8];target[0]=source[0];target[2..].copy_from_slice(&source[2..]);Ok(())}
    /// 8017FE88, with the caller's resolved language and default subtitle flag.
    pub fn initialize_profile(&mut self,index:usize,language:u8,subtitles:bool)->Result<(),String>{
        check_index(index)?;if language>7{return Err("Native profile language outside3 bits".into());}let language=if language==0{1}else{language};let p=&mut self.bytes[index*8..index*8+8];
        p[0]=((p[0]&0x8f)|language<<4)&!4|u8::from(subtitles)<<2|0x88;p[2..].copy_from_slice(&[0xaa,0xaa,0,0x28,0,0]);Ok(())
    }
}
#[derive(Clone,Debug,PartialEq,Eq)]pub struct SaveBundle{pub records:SaveFile,pub options:OptionFile}
impl SaveBundle{
    pub fn empty(reset:&[u8])->Result<Self,String>{Ok(Self{records:SaveFile::empty(reset)?,options:OptionFile::empty()})}
    pub fn copy_slot(&mut self,from:usize,to:usize)->Result<(),String>{
        if !self.records.slot(from).ok_or("Save source index")?.occupied()||from==to{return Err("Copy requires an occupied source and a different destination".into());}
        let mut next=self.clone();next.records.copy_slot(from,to)?;next.options.copy_slot(from,to)?;*self=next;Ok(())
    }
    pub fn erase_slot(&mut self,index:usize,reset:&[u8],language:u8,subtitles:bool)->Result<(),String>{let mut next=self.clone();next.records.reset_slot(index,reset)?;next.options.initialize_profile(index,language,subtitles)?;*self=next;Ok(())}
    pub fn start_new_game(&mut self,index:usize,reset:&[u8],language:u8,subtitles:bool)->Result<(),String>{let mut next=self.clone();next.records.start_new_game(index,reset)?;next.options.initialize_profile(index,language,subtitles)?;*self=next;Ok(())}
}

impl SaveFile {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != SAVE_BYTES { return Err(format!("save.dat must contain {SAVE_BYTES} bytes, got {}",bytes.len())); }
        Ok(Self { bytes: bytes.to_vec() })
    }
    pub fn empty(reset: &[u8]) -> Result<Self, String> {
        check_reset(reset)?;
        Ok(Self { bytes: reset.repeat(SLOT_COUNT) })
    }
    pub fn as_bytes(&self) -> &[u8] { &self.bytes }
    pub fn slot(&self, index: usize) -> Option<SaveSlot<'_>> {
        let start=index.checked_mul(RECORD_BYTES)?;
        Some(SaveSlot { bytes:self.bytes.get(start..start.checked_add(RECORD_BYTES)?)? })
    }
    /// Copy the complete record, including all unknown fields. Confirmation and
    /// whether the destination may be overwritten belong to the menu controller.
    pub fn copy_slot(&mut self, from: usize, to: usize) -> Result<(), String> {
        check_index(from)?; check_index(to)?;
        self.bytes.copy_within(from*RECORD_BYTES..(from+1)*RECORD_BYTES,to*RECORD_BYTES);
        Ok(())
    }
    pub fn reset_slot(&mut self, index: usize, reset: &[u8]) -> Result<(), String> {
        check_index(index)?; check_reset(reset)?;
        self.bytes[index*RECORD_BYTES..(index+1)*RECORD_BYTES].copy_from_slice(reset);
        Ok(())
    }
    /// 8023D3F4: initialize an empty slot and mark it occupied with new-game
    /// setup pending. Saving and the subsequent scene request are separate.
    pub fn start_new_game(&mut self, index: usize, reset: &[u8]) -> Result<(), String> {
        check_index(index)?;
        if self.slot(index).unwrap().occupied() { return Err("Selected save slot is occupied".into()); }
        self.reset_slot(index,reset)?;
        self.bytes[index*RECORD_BYTES+FLAGS]|=3;
        Ok(())
    }
}

impl SaveSlot<'_> {
    pub fn as_bytes(&self) -> &[u8] { self.bytes }
    /// 8017FCA0(record, 0).
    pub fn occupied(&self) -> bool { self.bytes[FLAGS]&1 != 0 }
    pub fn new_game_pending(&self) -> bool { self.bytes[FLAGS]&2 != 0 }
    pub fn score(&self) -> u32 { u32::from_be_bytes(self.bytes[0..4].try_into().unwrap()) }
    pub fn stage(&self) -> u16 { u16::from_be_bytes(self.bytes[0xC..0xE].try_into().unwrap()) }
    pub fn play_time(&self) -> f32 { f32::from_be_bytes(self.bytes[0x10..0x14].try_into().unwrap()) }
    pub fn clear_mark(&self)->bool{self.bytes[0x44c]!=0}
    pub fn special_mark(&self)->Option<u16>{(self.bytes[0x44d]&0x40!=0).then_some(16+(self.bytes[0x44d]>>7)as u16)}
    /// 8023D3A0's first unfilled challenge record; the all-filled case returns0.
    pub fn challenge_mark(&self)->u16{(0..15).find(|i|self.bytes[0x3d4+i*8..0x3d8+i*8]==[0;4]).unwrap_or(0)as u16}
}

fn check_index(index:usize)->Result<(),String>{if index<SLOT_COUNT{Ok(())}else{Err("Save slot index outside 0..4".into())}}
fn check_reset(reset:&[u8])->Result<(),String>{
    if reset.len()!=RECORD_BYTES || reset[FLAGS]&1!=0 {Err("Expected one unoccupied native reset record".into())}else{Ok(())}
}

#[cfg(test)]
mod tests {
    use super::*;
    const RESET:&[u8]=include_bytes!("../../../assets/native/save/reset-record.bin");
    #[test]fn native_profile_initialization_copy_padding_and_erase_match_source(){
        let mut data=SaveBundle::empty(RESET).unwrap();data.start_new_game(0,RESET,1,false).unwrap();
        // Independent original32-byte option.dat from newgame-fs-fixed-v1.
        assert_eq!(data.options.as_bytes(),&[0x98,0,0xaa,0xaa,0,0x28,0,0,8,0,0xaa,0xaa,0,0x28,0,0,8,0,0xaa,0xaa,0,0x28,0,0,8,0,0xaa,0xaa,0,0x28,0,0]);
        let mut options=data.options.as_bytes().to_vec();options[0]=0xbd;options[1]=0xee;options[2..8].copy_from_slice(&[1,2,3,4,5,6]);options[9]=0x7b;data.options=OptionFile::parse(&options).unwrap();
        let mut records=data.records.as_bytes().to_vec();records[0xbeef]=0xa7;data.records=SaveFile::parse(&records).unwrap();data.copy_slot(0,1).unwrap();
        assert_eq!(data.records.slot(1).unwrap().as_bytes(),data.records.slot(0).unwrap().as_bytes());assert_eq!(data.options.profile(1).unwrap(),&[0xbd,0x7b,1,2,3,4,5,6]);
        data.erase_slot(1,RESET,1,false).unwrap();assert_eq!(data.records.slot(1).unwrap().as_bytes(),RESET);assert_eq!(data.options.profile(1).unwrap(),&[0x99,0x7b,0xaa,0xaa,0,0x28,0,0]);
        assert_eq!(data.records.slot(0).unwrap().as_bytes()[0xbeef],0xa7);assert_eq!(data.options.profile(0).unwrap(),&[0xbd,0xee,1,2,3,4,5,6]);
        let before=data.clone();assert!(data.start_new_game(0,RESET,1,false).is_err());assert!(data.erase_slot(0,RESET,8,false).is_err());assert_eq!(data,before);assert!(OptionFile::parse(&[0;31]).is_err());
    }

    #[test]
    fn reset_matches_independent_native_empty_slot_observation() {
        // Nonzero bytes observed in each of the three empty native records.
        // Keeping this sparse observation independent catches reset table/layout errors.
        let expected:[(usize,u8);29]=[(7,3),(12,1),(977,1),(985,24),(986,106),(993,26),(994,179),(995,240),
            (1009,30),(1010,132),(1011,128),(1017,34),(1018,85),(1019,16),(1033,38),(1034,37),(1035,160),
            (1041,40),(1042,111),(1043,144),(1057,42),(1058,185),(1059,128),(1065,44),(1066,64),(1067,32),
            (1081,38),(1082,37),(1083,160)];
        let nonzero:Vec<_>=RESET.iter().copied().enumerate().filter(|(_,b)|*b!=0).collect();
        assert_eq!(nonzero,expected);
        let save=SaveFile::empty(RESET).unwrap();
        for i in 0..4{let s=save.slot(i).unwrap();assert!(!s.occupied());assert_eq!((s.score(),s.stage(),s.play_time()),(0,0x100,0.));}
    }

    #[test]
    fn mutations_preserve_unknown_fields_and_other_slots() {
        let mut bytes=RESET.repeat(4);
        bytes[FLAGS]=0x81;bytes[0..4].copy_from_slice(&123456u32.to_be_bytes());
        bytes[0xC..0xE].copy_from_slice(&0x201u16.to_be_bytes());bytes[0x10..0x14].copy_from_slice(&91.5f32.to_be_bytes());
        bytes[0xBEEF]=0xAA;
        let mut save=SaveFile::parse(&bytes).unwrap();assert_eq!(save.as_bytes(),bytes);
        let slot=save.slot(0).unwrap();assert!(slot.occupied());assert_eq!((slot.score(),slot.stage(),slot.play_time()),(123456,0x201,91.5));
        save.copy_slot(0,2).unwrap();assert_eq!(save.slot(0).unwrap().as_bytes(),save.slot(2).unwrap().as_bytes());
        save.reset_slot(0,RESET).unwrap();assert_eq!(save.slot(0).unwrap().as_bytes(),RESET);
        assert_eq!(save.slot(2).unwrap().as_bytes()[0xBEEF],0xAA);
        assert_eq!(save.slot(1).unwrap().as_bytes(),RESET);assert_eq!(save.slot(3).unwrap().as_bytes(),RESET);
        let before=save.clone();assert!(save.copy_slot(0,4).is_err());assert!(save.reset_slot(usize::MAX,RESET).is_err());assert_eq!(save,before);
        assert!(save.slot(usize::MAX).is_none());
        assert!(SaveFile::parse(&bytes[..SAVE_BYTES-1]).is_err());assert!(SaveFile::parse(&[bytes,vec![0]].concat()).is_err());
    }

    #[test]
    fn new_game_record_matches_native_start_observation_and_protects_occupied_slots() {
        let mut save=SaveFile::empty(RESET).unwrap();save.start_new_game(0,RESET).unwrap();
        let slot=save.slot(0).unwrap();assert!(slot.occupied()&&slot.new_game_pending());
        assert_eq!((slot.score(),slot.stage(),slot.play_time()),(0,256,0.));
        // ui-save-fs-trace-oracle-final-state: native record SHA256
        // 4528f6695021ead5cdaa59d78d4f74766d0f2b9d0faf711871ddb860eecd0894.
        // The full reset hash and this flag delta were observed independently.
        let changes:Vec<_>=slot.as_bytes().iter().zip(RESET).enumerate().filter_map(|(i,(a,b))|(a!=b).then_some((i,*a))).collect();
        assert_eq!(changes,vec![(0x44E,3)]);
        let before=save.clone();assert!(save.start_new_game(0,RESET).is_err());assert_eq!(save,before);
        for i in 1..4{assert_eq!(save.slot(i).unwrap().as_bytes(),RESET);}
    }
}
