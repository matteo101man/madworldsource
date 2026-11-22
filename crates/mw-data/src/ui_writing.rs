//! Native100-byte WCD records consumed by802E2394/802E1DF0.
#[derive(Clone,Debug)]
pub struct WritingRecord{
    pub id:u16,pub program:u16,pub style:u16,pub position_source:u16,pub flags:u32,
    pub position:[f32;3],pub position_random:[f32;3],pub exclusion:[f32;3],
    pub scale:[f32;3],pub rotation:[f32;3],pub rotation_random:[f32;3],
    pub rgba:[u8;4],pub glyphs:[i8;11],pub count:u8,
}
impl WritingRecord{
    pub fn parse(bytes:&[u8])->Result<Self,&'static str>{
        if bytes.len()!=100{return Err("WCD record length");}
        let h=|o:usize|u16::from_be_bytes(bytes[o..o+2].try_into().unwrap());
        let v=|o:usize|[0usize,1,2].map(|i|f32::from_be_bytes(bytes[o+i*4..o+i*4+4].try_into().unwrap()));
        let record=Self{id:h(0),program:h(2),style:h(4),position_source:h(6),flags:u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
            position:v(12),position_random:v(24),exclusion:v(36),scale:v(48),rotation:v(60),rotation_random:v(72),
            rgba:bytes[84..88].try_into().unwrap(),glyphs:std::array::from_fn(|i|bytes[88+i]as i8),count:bytes[99]};
        if record.count>10||[record.position,record.position_random,record.exclusion,record.scale,record.rotation,record.rotation_random].iter().flatten().any(|v|!v.is_finite()){
            return Err("Invalid WCD dimensions/count");
        }Ok(record)
    }
    pub fn glyph_group(&self)->Result<u16,&'static str>{match self.style{0=>Ok(50),1=>Ok(60),2=>Ok(61),3=>Ok(68),_=>Err("Untranslated WCD glyph style")}}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn original_thud_record_preserves_native_fields(){
        let bytes=include_bytes!("../../../assets/native/events/ev21/writing/record.wcd");let record=WritingRecord::parse(bytes).unwrap();
        assert_eq!((record.id,record.program,record.position_source,record.flags,record.count),(111,0,3,0,4));
        assert_eq!(record.glyph_group(),Ok(61));assert_eq!(record.position,[200.,40.,0.]);assert_eq!(record.rotation,[0.,0.,-20.]);
        assert_eq!(&record.glyphs[..4],&[19,7,20,3]);assert_eq!(record.rgba,[242,241,212,240]);
        assert!(WritingRecord::parse(&bytes[..99]).is_err());let mut bad=*bytes;bad[99]=11;assert!(WritingRecord::parse(&bad).is_err());
    }
}
