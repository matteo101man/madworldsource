//! 80184490 glyph placement,80185BF8 command subset,80186DAC row sizing.
use mw_data::ui_message::{Atom,MessageCatalog};
#[derive(Clone,Debug)]pub struct TextQuad{pub glyph:u16,pub center:[f32;2],pub size:[f32;2]}
pub fn layout(catalog:&MessageCatalog,message:usize,anchor:[f32;2],dimensions:[f32;3],statistics:&[u32])->Result<Vec<TextQuad>,String>{
    let mut out=vec![];let mut cursor=anchor[0];let mut scale=1.;
    for atom in catalog.atoms(message)?{match atom{
        Atom::Scale(value)=>{scale=value;cursor=anchor[0];},
        Atom::Space=>cursor+=0.5*dimensions[0]*scale,
        Atom::Newline=>return Err("Multiline message requires centered row layout".into()),
        Atom::ResetScale=>scale=1.,
        Atom::Glyph(id)=>glyph(catalog,id,&mut cursor,anchor[1],dimensions,scale,&mut out)?,
        Atom::Statistic(index)=>{
            let value=statistics.get(index).ok_or("Missing native statistic")?.min(&999999).to_string();
            cursor-=(value.len()-1)as f32*0.5*dimensions[0]*scale;
            for digit in value.bytes(){glyph(catalog,(digit-b'0')as u16,&mut cursor,anchor[1],dimensions,scale,&mut out)?;}
        },
    }}Ok(out)
}
fn glyph(catalog:&MessageCatalog,id:u16,cursor:&mut f32,y:f32,dimensions:[f32;3],scale:f32,out:&mut Vec<TextQuad>)->Result<(),String>{
    let g=catalog.glyph(id).ok_or("Missing original glyph")?;let factor=dimensions.map(|v|v/catalog.base_advance*scale);
    *cursor+=0.5*g.size[0]*factor[0];
    out.push(TextQuad{glyph:id,center:[*cursor,y-0.5*dimensions[2]*scale-0.25*scale*(catalog.base_advance-g.size[1])],size:[g.size[0]*factor[0],g.size[1]*factor[1]]});
    *cursor+=2.+0.3*g.size[0]*factor[0];Ok(())
}

/// 80186DAC(type5 default metrics),80184A1C/801875F8 vertical centering,
/// 801870F4 row widths and80185BF8 line/scale commands. Stage family1 uses
/// 0.2 trailing width; common family0 (Play Log) uses0.3.
pub fn centered_stage(catalog:&MessageCatalog,message:usize,anchor:[f32;2],dimensions:[f32;2],line_spacing:f32)->Result<Vec<TextQuad>,String>{
    centered_rows(catalog,message,anchor,dimensions,line_spacing,true,0.2)
}
/// Event MESS family3 fixes its first baseline; challenge family1 centers all rows.
pub fn event_lines(catalog:&MessageCatalog,message:usize,anchor:[f32;2],dimensions:[f32;2],line_spacing:f32)->Result<Vec<TextQuad>,String>{
    centered_rows(catalog,message,anchor,dimensions,line_spacing,false,0.3)
}
fn centered_rows(catalog:&MessageCatalog,message:usize,anchor:[f32;2],dimensions:[f32;2],line_spacing:f32,vertical_center:bool,trailing_width:f32)->Result<Vec<TextQuad>,String>{
    let atoms=catalog.atoms(message)?;
    if atoms.iter().any(|a|matches!(a,Atom::Glyph(id)if *id<0x1000)){return Err("Centered common-family glyph spacing untranslated".into());}
    let mut scale=atoms.iter().find_map(|a|if let Atom::Scale(s)=a{Some(*s)}else{None}).unwrap_or(1.);
    let rows=atoms.iter().filter(|a|matches!(a,Atom::Newline)).count()+1;
    let mut y=anchor[1]-if vertical_center{rows as f32*0.5*line_spacing*scale}else{0.};
    let mut out=vec![];
    for row in atoms.split(|a|matches!(a,Atom::Newline)){
        let start_scale=scale;let mut width=0.;
        for a in row{match *a{
            Atom::Scale(v)=>scale=v,Atom::ResetScale=>scale=1.,Atom::Space=>width+=0.5*dimensions[0]*scale,
            Atom::Glyph(id)=>{let g=catalog.glyph(id).ok_or("Missing original glyph")?;width+=2.+(0.5+trailing_width)*g.size[0]*dimensions[0]/catalog.base_advance*scale;},
            Atom::Statistic(_)=>return Err("Centered statistic message untranslated".into()),Atom::Newline=>unreachable!(),
        }}
        scale=start_scale;let mut x=anchor[0]-0.5*width;
        for a in row{match *a{
            Atom::Scale(v)=>scale=v,Atom::ResetScale=>scale=1.,Atom::Space=>x+=0.5*dimensions[0]*scale,
            Atom::Glyph(id)=>{let g=catalog.glyph(id).unwrap();let size=[0,1].map(|i|g.size[i]*dimensions[i]/catalog.base_advance*scale);
                x+=0.5*size[0];out.push(TextQuad{glyph:id,center:[x,y-0.25*scale*(catalog.base_advance-g.size[1])],size});x+=2.+trailing_width*size[0];},
            _=>unreachable!(),
        }}
        y+=line_spacing*scale;
    }
    Ok(out)
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn event_messages_match_all_original_component_glyphs(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalog=MessageCatalog::parse(&std::fs::read_to_string(root.join("assets/native/events/ev00/messages/catalog.json")).unwrap()).unwrap();
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/event-message-observed.json")).unwrap()).unwrap();
        let mut total=0;
        for capture in observed["captures"].as_array().unwrap(){
            let id=capture["message"].as_u64().unwrap()as usize;
            let quads=event_lines(&catalog,id,[320.,330.],[20.,20.],capture["dimensions"][2].as_f64().unwrap()as f32).unwrap();
            let glyphs=capture["glyphs"].as_array().unwrap();assert_eq!(quads.len(),glyphs.len());
            for(index,(q,g))in quads.iter().zip(glyphs).enumerate(){
                for axis in 0..2{
                    assert!((q.center[axis]-g["matrix"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0003,"message{id} glyph{index} center{axis}: {:?} vs {}",q.center,g["matrix"]);
                    assert!((q.size[axis]-g["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.00001,"message{id} glyph{index} size{axis}");
                }
                let glyph=catalog.glyph(q.glyph).unwrap();let page=&catalog.pages[glyph.page];let bounds=[glyph.rect[0]as f32/page.width as f32,glyph.rect[1]as f32/page.height as f32,glyph.rect[2]as f32/page.width as f32,glyph.rect[3]as f32/page.height as f32];
                for axis in 0..2{let values:Vec<_>=g["uv"].as_array().unwrap().iter().skip(axis).step_by(2).map(|v|v.as_f64().unwrap()as f32).collect();assert!((values.iter().copied().fold(f32::INFINITY,f32::min)-bounds[axis]).abs()<0.000001);assert!((values.iter().copied().fold(f32::NEG_INFINITY,f32::max)-bounds[axis+2]).abs()<0.000001);}
                assert!(g["rgba"].as_array().unwrap().iter().all(|v|v.as_f64()==Some(1.)));total+=1;
            }
        }assert_eq!(total,209);
    }
    #[test]fn all_first_page_glyphs_match_owned_native_pool(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let c=MessageCatalog::parse(&std::fs::read_to_string(root.join("assets/native/ui/playlog-us/catalog.json")).unwrap()).unwrap();
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-playlog-glyphs-observed.json")).unwrap()).unwrap();
        let mut quads=vec![];
        for row in 0..5{for col in 0..2{quads.extend(layout(&c,row*2+col,[if col==0{110.}else{370.},100.+row as f32*30.],[18.,18.,16.],&[0;46]).unwrap());}}
        assert_eq!(quads.len(),75);let glyphs=observed["glyphs"].as_array().unwrap();assert_eq!(glyphs.len(),quads.len());
        for (i,(q,o))in quads.iter().zip(glyphs).enumerate(){
            for axis in 0..2{
                assert!((q.center[axis]-o["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32).abs()<0.0002,"glyph {i} center {axis}");
                assert!((q.size[axis]-o["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.}).abs()<0.00001,"glyph {i} size {axis}");
            }
            let g=c.glyph(q.glyph).unwrap();let p=&c.pages[g.page];let bounds=[g.rect[0]as f32/p.width as f32,g.rect[1]as f32/p.height as f32,g.rect[2]as f32/p.width as f32,g.rect[3]as f32/p.height as f32];
            let uv:Vec<_>=o["uvCorners"].as_array().unwrap().iter().map(|v|v.as_f64().unwrap()as f32).collect();
            for axis in 0..2{let values:Vec<_>=uv.iter().skip(axis).step_by(2).copied().collect();assert!((values.iter().copied().fold(f32::INFINITY,f32::min)-bounds[axis]).abs()<0.00001,"glyph {i} UV min");assert!((values.iter().copied().fold(f32::NEG_INFINITY,f32::max)-bounds[axis+2]).abs()<0.00001,"glyph {i} UV max");}
        }
    }
    #[test]fn every_page_lays_out_and_numbers_use_original_six_digit_cap(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let c=MessageCatalog::parse(&std::fs::read_to_string(root.join("assets/native/ui/playlog-us/catalog.json")).unwrap()).unwrap();
        for id in 0..c.messages.len(){let q=layout(&c,id,[110.,100.],[18.,18.,16.],&[12345678;46]).unwrap();assert!(!q.is_empty());assert!(q.iter().all(|q|q.center.iter().chain(&q.size).all(|v|v.is_finite())));if id%2==1{assert_eq!(q.len(),6);assert!(q.iter().all(|q|q.glyph==9));}}
    }
}
