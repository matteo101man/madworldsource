//! Original MTD glyphs and the bounded message commands used by Play Log and boss-entry confirmation.
use serde::Deserialize;
use std::collections::HashSet;
#[derive(Clone,Debug,Deserialize)]pub struct Glyph{pub id:u16,pub page:usize,pub rect:[u32;4],pub size:[f32;2]}
#[derive(Clone,Debug,Deserialize)]pub struct Message{pub id:usize,pub tokens:Vec<u16>}
#[derive(Clone,Debug,Deserialize)]pub struct MessageCatalog{
    pub schema:String,pub pages:Vec<crate::ui::UiPageMap>,pub glyphs:Vec<Glyph>,pub messages:Vec<Message>,
    #[serde(rename="baseAdvance")]pub base_advance:f32,
}
#[derive(Clone,Debug,PartialEq)]pub enum Atom{Glyph(u16),Space,Newline,Scale(f32),ResetScale,Statistic(usize)}
impl MessageCatalog{
    pub fn parse(text:&str)->Result<Self,String>{
        let catalog:Self=serde_json::from_str(text).map_err(|e|e.to_string())?;
        if catalog.schema!="mw-native-message-catalog/2"||catalog.pages.is_empty()||catalog.messages.is_empty()||!catalog.base_advance.is_finite()||catalog.base_advance<=0.{return Err("Message catalog header".into());}
        if catalog.pages.iter().any(|p|!safe_message_asset(&p.image)||p.width==0||p.height==0){return Err("Message atlas page".into());}
        let mut ids=HashSet::new();
        for g in &catalog.glyphs{
            let p=catalog.pages.get(g.page).ok_or("Message glyph page")?;
            if !ids.insert(g.id)||g.rect[0]>=g.rect[2]||g.rect[1]>=g.rect[3]||g.rect[2]>p.width||g.rect[3]>p.height||g.size.iter().any(|v|!v.is_finite()||*v<=0.){return Err("Message glyph geometry".into());}
        }
        for (id,m)in catalog.messages.iter().enumerate(){if m.id!=id{return Err("Message IDs".into());}catalog.atoms(id)?;}
        Ok(catalog)
    }
    pub fn glyph(&self,id:u16)->Option<&Glyph>{self.glyphs.iter().find(|g|g.id==id)}
    pub fn page_count(&self)->usize{(self.messages.len()/2)/5+1}
    pub fn atoms(&self,id:usize)->Result<Vec<Atom>,String>{
        let tokens=&self.messages.get(id).ok_or("Message ID")?.tokens;let mut i=0;let mut atoms=vec![];
        // (an unused slot: ev84 / ev93 leave most of theirs empty)
        if tokens.is_empty(){return Ok(atoms);}
        while let Some(&token)=tokens.get(i){i+=1;match token{
            0x8002=>{if i!=tokens.len(){return Err("Trailing message tokens".into());}return Ok(atoms);},
            0x8001=>atoms.push(Atom::Space),
            0x8000=>atoms.push(Atom::Newline),
            0x8011=>atoms.push(Atom::ResetScale),
            0x8010|0x8073|0x8093=>{
                let value=*tokens.get(i).ok_or("Missing message operand")?;i+=1;
                match token{0x8010 if value>0&&value<=400=>atoms.push(Atom::Scale(value as f32/100.)),
                    0x8073 if value==0=>{},0x8093 if (value as usize)<self.messages.len()/2=>atoms.push(Atom::Statistic(value as usize)),
                    _=>return Err(format!("Unsupported message operand {token:x}:{value}")),}
            },
            id if self.glyph(id).is_some()=>atoms.push(Atom::Glyph(id)),
            // (ev84's numerals, 0x0-0x9: drawn from a table this catalog does not carry; left out [inferred])
            0..=9=>{},
            _=>return Err(format!("Unsupported message token {token:x}")),
        }}Err("Unterminated message".into())
    }
}
fn safe_message_asset(path:&str)->bool{
    (path.starts_with("native/ui/")||path.starts_with("native/events/"))&&!path.contains(['\\',':'])&&!path.split('/').any(|p|p.is_empty()||p=="."||p=="..")
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn original_catalog_validates_and_rejects_unsafe_or_untranslated_data(){
        let p=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/ui/playlog-us/catalog.json");let text=std::fs::read_to_string(p).unwrap();
        let c=MessageCatalog::parse(&text).unwrap();assert_eq!(c.glyphs.len(),37);assert_eq!(c.page_count(),10);
        assert_eq!(c.atoms(1).unwrap(),vec![Atom::Scale(0.8),Atom::Statistic(0)]);
        assert!(MessageCatalog::parse(&text.replacen("native/ui/playlog-us/font-0.png","../font.png",1)).is_err());
        let mut json:serde_json::Value=serde_json::from_str(&text).unwrap();json["messages"][0]["tokens"][0]=serde_json::json!(0x8fff);assert!(MessageCatalog::parse(&json.to_string()).is_err());
        json=serde_json::from_str(&text).unwrap();json["glyphs"][0]["rect"][2]=serde_json::json!(99999);assert!(MessageCatalog::parse(&json.to_string()).is_err());
    }
}
