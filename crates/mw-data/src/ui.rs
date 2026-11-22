//! Original UI atlases, IDT widget records and bounded position tracks. Full widget playback remains separate.
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize)]
pub struct UiPage { pub id: usize, pub width: u32, pub height: u32, pub image: String, pub sha256: String,
    #[serde(default,rename="invertedImage")]pub inverted_image:Option<String>,
    #[serde(default,rename="invertedSha256")]pub inverted_sha256:Option<String> }
#[derive(Clone, Debug, Deserialize)]
pub struct UiGroup { pub id: u16, pub first: usize, pub count: usize }
#[derive(Clone, Debug, Deserialize)]
pub struct UiSprite {
    pub id: usize, pub group: u16,
    #[serde(rename = "texturePage")] pub texture_page: usize,
    pub rect: [u32; 4], pub uv: [f32; 4],
}
#[derive(Clone, Debug, Deserialize)]
pub struct UiSource {
    pub path: String,
    #[serde(rename = "containerSha256")] pub container_sha256: String,
    pub offset: usize,
}
#[derive(Clone, Debug, Deserialize)]
pub struct UiAnimation { pub file: String, pub sha256: String, pub bytes: usize, pub semantics: String }
#[derive(Clone, Debug, Deserialize)]
pub struct UiLayout {
    pub schema: String, pub source: UiSource, pub pages: Vec<UiPage>,
    pub groups: Vec<UiGroup>, pub sprites: Vec<UiSprite>, pub animation: UiAnimation,
}
impl UiLayout {
    pub fn parse(json: &str) -> Result<Self, String> {
        let layout: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if layout.schema != "mw-ui-layout/1" || layout.pages.is_empty() { return Err("UI schema/pages".into()); }
        for (i, page) in layout.pages.iter().enumerate() {
            if page.id != i || page.width == 0 || page.height == 0 || !safe_asset(&page.image) ||page.inverted_image.as_ref().is_some_and(|p|!safe_asset(p)) { return Err("UI texture page".into()); }
        }
        let mut ids = HashSet::new();
        let mut first = 0usize;
        for group in &layout.groups {
            if !ids.insert(group.id) || group.first != first { return Err("UI group table".into()); }
            let end = first.checked_add(group.count).ok_or("UI group overflow")?;
            let sprites = layout.sprites.get(first..end).ok_or("UI group range")?;
            for (i, sprite) in sprites.iter().enumerate() {
                let page = layout.pages.get(sprite.texture_page).ok_or("UI sprite page")?;
                let [x0,y0,x1,y1] = sprite.rect;
                if sprite.id != first+i || sprite.group != group.id || x0>=x1 || y0>=y1 || x1>page.width || y1>page.height {
                    return Err("UI sprite bounds/index".into());
                }
                for ((u, pixel), size) in sprite.uv.iter().zip(sprite.rect).zip([page.width,page.height,page.width,page.height]) {
                    if !u.is_finite() || !(0. ..=1.).contains(u) || (u*size as f32-pixel as f32).abs()>0.001 {
                        return Err("UI UV/pixel mismatch".into());
                    }
                }
            }
            first = end;
        }
        if first != layout.sprites.len() || !safe_asset(&layout.animation.file) || layout.animation.bytes<4 {
            return Err("UI unconsumed sprites/animation".into());
        }
        Ok(layout)
    }
    pub fn sprite(&self, id: usize) -> Option<(&UiPage, &UiSprite)> {
        let sprite = self.sprites.get(id)?;
        Some((self.pages.get(sprite.texture_page)?, sprite))
    }
}
pub(crate) fn safe_asset(s: &str) -> bool {
    (s.starts_with("native/ui/") || (s.starts_with("native/events/") && s.split('/').nth(3) == Some("ui"))) && !s.contains('\\') && !s.contains(':') && !s.split('/').any(|p| p.is_empty() || p==".." || p==".")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn english_ui_sources_cover_hud_pause_and_stage201_challenge() {
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/ui");
        for (key,count) in [("core_us",546),("subscr000_us",168),("subscr001_us",8),("subscr0fe_us",9),
                            ("nl_us",115),("ex000_us",20),("idpac_c201_us",50),("idpac_ce00_us",151)] {
            let text=std::fs::read_to_string(root.join(key).join("layout.json")).unwrap();
            let layout=UiLayout::parse(&text).unwrap();
            assert_eq!(layout.sprites.len(),count);
            assert!(layout.sprite(count-1).is_some());
            assert!(layout.sprite(count).is_none());
            let mut bad:serde_json::Value=serde_json::from_str(&text).unwrap();
            bad["sprites"][0]["texturePage"]=serde_json::json!(999);
            assert!(UiLayout::parse(&bad.to_string()).is_err());
            bad=serde_json::from_str(&text).unwrap();
            bad["pages"][0]["image"]=serde_json::json!("native/ui/../../outside.png");
            assert!(UiLayout::parse(&bad.to_string()).is_err());
        }
    }
}

/// Native IDT directory and serialized widget records (8024BD90 / 8024B81C).
/// Channels retain record-relative offsets until their native evaluators are translated.
#[derive(Clone, Debug)]
pub struct UiTimeline { pub programs: Vec<UiProgram> }
#[derive(Clone, Debug)]
pub struct UiProgram { pub id: u32, pub record_offset: usize, pub widgets: Vec<UiWidget> }
#[derive(Clone, Debug)]
pub struct UiWidget {
    pub record_offset: usize, pub record_bytes: usize,
    pub id: u32, pub kind: u32, pub flags: u32, pub order: u32,
    /// 8024B81C maps A0/A2/A4/A6 to parent/first-child/next/previous.
    /// Native traversal follows source siblings in descending serialized order.
    pub parent: Option<usize>, pub first_child: Option<usize>,
    pub previous: Option<usize>, pub next: Option<usize>,
    pub source_indices: [i16; 6],
    /// Static source fields copied by 80184A1C; UV/group interpretation is not universal across widget kinds.
    pub uv_indices: [u16; 6], pub color: [f32; 4],
    pub root_position: [f32; 3], pub child_position: [f32; 3], pub size: [f32; 3],
    /// +4,+8,+C,+10,+14,+18,+1C,+20, relative to this widget record (zero uses native defaults).
    pub channels: [Option<usize>; 8],
}
impl UiTimeline {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let r=UiReader(bytes);
        if bytes.get(..4)!=Some(b"IDT\0") { return Err("IDT magic".into()); }
        let count=r.u32(4)? as usize;
        if count>1024 { return Err("IDT program count".into()); }
        let directory_end=8+count*8;
        r.range(0,directory_end)?;
        let mut ids=HashSet::new();
        let mut programs=Vec::with_capacity(count);
        let mut last_end=directory_end;
        for i in 0..count {
            let id=r.u32(8+i*8)?;
            let base=r.u32(12+i*8)? as usize;
            let end=if i+1<count {r.u32(20+i*8)? as usize} else {bytes.len()};
            if !ids.insert(id) || base<last_end || base>=end || end>bytes.len() { return Err("IDT program directory".into()); }
            let n=r.u32(base)? as usize;
            if n>256 || base+4+n*4>end { return Err("IDT widget count/table".into()); }
            let mut widgets=Vec::with_capacity(n);
            let mut previous_end=base+4+n*4;
            for j in 0..n {
                let at=base.checked_add(r.u32(base+4+j*4)? as usize).ok_or("IDT widget overflow")?;
                let len=r.u32(at)? as usize;
                let stop=at.checked_add(len).ok_or("IDT record overflow")?;
                if len<0x220 || at<previous_end || stop>end { return Err("IDT widget bounds/overlap".into()); }
                r.range(at,len)?;
                let source_indices: [i16; 6]=read_array(|k|r.u16(at+0xa0+2*k).map(|v|v as i16))?;
                for v in source_indices { if v < -1 || v>=n as i16 { return Err("IDT hierarchy index".into()); } }
                let link=|k:usize| if source_indices[k]<0 {None} else {Some(source_indices[k] as usize)};
                let mut channels=[None;8];
                for (k,channel) in channels.iter_mut().enumerate() {
                    let offset=r.u32(at+4+4*k)? as usize;
                    if offset!=0 {
                        if offset<0x100 || offset.checked_add(4).is_none_or(|v|v>len) { return Err("IDT channel offset".into()); }
                        *channel=Some(offset);
                    }
                }
                widgets.push(UiWidget {record_offset:at,record_bytes:len,id:r.u32(at+0x2c)?,kind:r.u32(at+0x3c)?,flags:r.u32(at+0x34)?,order:r.u32(at+0x30)?,
                    parent:link(0),first_child:link(1),previous:link(3),next:link(2),source_indices,
                    uv_indices:read_array(|k|r.u16(at+0xb0+2*k))?, color:read_array(|k|r.f32(at+0xbc+4*k))?,
                    root_position:read_array(|k|r.f32(at+0xcc+4*k))?,child_position:read_array(|k|r.f32(at+0xd8+4*k))?,
                    size:read_array(|k|r.f32(at+0xe4+4*k))?,channels});
                previous_end=stop;
            }
            for j in 0..n {
                let mut ancestor=widgets[j].parent;
                let mut seen=HashSet::from([j]);
                while let Some(k)=ancestor {
                    if !seen.insert(k) { return Err("IDT parent cycle".into()); }
                    ancestor=widgets[k].parent;
                }
                if let Some(k)=widgets[j].first_child {
                    if widgets[k].parent!=Some(j) { return Err("IDT child/parent mismatch".into()); }
                }
                if let Some(k)=widgets[j].next {
                    if widgets[k].previous!=Some(j) || widgets[k].parent!=widgets[j].parent { return Err("IDT sibling mismatch".into()); }
                }
            }
            programs.push(UiProgram{id,record_offset:base,widgets});
            last_end=end;
        }
        Ok(Self{programs})
    }
    pub fn program(&self,id:u32)->Option<&UiProgram> {self.programs.iter().find(|p|p.id==id)}
}
fn read_array<T: Default+Copy,const N:usize>(mut f:impl FnMut(usize)->Result<T,String>)->Result<[T;N],String> {
    let mut values=[T::default();N];
    for (i,v) in values.iter_mut().enumerate() {*v=f(i)?;}
    Ok(values)
}
struct UiReader<'a>(&'a [u8]);
impl UiReader<'_> {
    fn range(&self,at:usize,len:usize)->Result<&[u8],String> { self.0.get(at..at.checked_add(len).ok_or("IDT bounds overflow")?).ok_or_else(||"IDT truncated".into()) }
    fn u32(&self,at:usize)->Result<u32,String> {Ok(u32::from_be_bytes(self.range(at,4)?.try_into().unwrap()))}
    fn u16(&self,at:usize)->Result<u16,String> {Ok(u16::from_be_bytes(self.range(at,2)?.try_into().unwrap()))}
    fn f32(&self,at:usize)->Result<f32,String> {let v=f32::from_bits(self.u32(at)?);if v.is_finite(){Ok(v)}else{Err("IDT nonfinite scalar".into())}}
}

#[cfg(test)]
mod idt_tests {
    use super::*;
    fn staged(key:&str)->Vec<u8> {std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/ui").join(key).join("animation.idt")).unwrap()}
    #[test]
    fn original_english_screen_hierarchies_are_bounded_and_consistent() {
        for (key,programs,widgets) in [("core_us",72,1396),("subscr000_us",20,897),("subscr001_us",3,63),
            ("subscr0fe_us",6,147),("nl_us",15,910),("ex000_us",5,169),("idpac_c201_us",11,815),("idpac_ce00_us",25,967)] {
            let b=staged(key);let t=UiTimeline::parse(&b).unwrap_or_else(|e|panic!("{key}: {e}"));
            if key=="subscr000_us" {let screen=t.program(0).unwrap();assert_eq!(screen.widgets[0].first_child,Some(149));assert_eq!(screen.widgets[149].next,Some(148));}
            assert_eq!(t.programs.len(),programs);assert_eq!(t.programs.iter().map(|p|p.widgets.len()).sum::<usize>(),widgets);
            for program in &t.programs {for widget in &program.widgets {widget.position_track(&b).unwrap_or_else(|e|panic!("{key} program {} widget {}: {e}",program.id,widget.id));
                widget.appearance(&b).unwrap_or_else(|e|panic!("{key} program {} widget {} appearance: {e}",program.id,widget.id));}}
        }
    }
    #[test]
    fn idt_rejects_truncation_bad_links_cycles_and_overlapping_records() {
        let b=staged("subscr000_us");let t=UiTimeline::parse(&b).unwrap();let p=&t.programs[0];let at=p.widgets[0].record_offset;
        let last=t.programs.last().unwrap().widgets.last().unwrap();
        for len in [0,3,7,100,last.record_offset+last.record_bytes-1] {assert!(UiTimeline::parse(&b[..len]).is_err());}
        let mut bad=b.clone();bad[at+0xa0..at+0xa2].copy_from_slice(&0u16.to_be_bytes());assert!(UiTimeline::parse(&bad).is_err());
        bad=b.clone();bad[at+0xa2..at+0xa4].copy_from_slice(&999u16.to_be_bytes());assert!(UiTimeline::parse(&bad).is_err());
        bad=b.clone();bad[p.record_offset+8..p.record_offset+12].copy_from_slice(&0u32.to_be_bytes());assert!(UiTimeline::parse(&bad).is_err());
        bad=b.clone();bad[at+4..at+8].copy_from_slice(&0x7fffffffu32.to_be_bytes());assert!(UiTimeline::parse(&bad).is_err());
    }
}

#[derive(Clone, Debug)]
pub struct UiVectorTrack { pub flags:u32, pub points:Vec<[f32;3]>, pub keys:Vec<UiVectorKey> }
#[derive(Clone, Copy, Debug)]
pub struct UiVectorKey { pub value:f32, pub frame:u32 }
impl UiWidget {
    /// 800C5120 initializes800C5508's normalized horizontal gauge increment
    /// from signed source numerator/denominator and the original width.
    pub fn gauge_step(&self,bytes:&[u8])->Result<f32,String>{
        if self.kind!=8{return Err("Widget is not a native gauge".into());}
        let r=UiReader(bytes);let axis=r.u16(self.record_offset+0x40)?;
        let numerator=r.u16(self.record_offset+0x42)?as i16;let denominator=r.u16(self.record_offset+0x44)?as i16;
        if axis!=1||numerator<=0||denominator<=0||self.size[0]<=0.{return Err("Untranslated native gauge rate/axis".into());}
        Ok((numerator as f32/denominator as f32)/self.size[0])
    }
    /// Position channel consumed by 800CB358: vectors +40, index/time keys +160/+164.
    /// Returns None for the native default or a channel without the animation-enable bit.
    pub fn position_track(&self,bytes:&[u8])->Result<Option<UiVectorTrack>,String> {
        let Some(offset)=self.channels[2] else {return Ok(None);};
        let r=UiReader(bytes);let base=self.record_offset+offset;
        let flags=r.u32(base+4)?;
        if flags&2==0 {return Ok(None);}
        if self.record_bytes-offset<0x220 {return Err("IDT vector track bounds".into());}
        let count=r.u32(base+8)? as usize;let keys=r.u32(base+12)? as usize;
        if keys==0 {return Ok(None);}
        if count==0 || count>24 || keys>24 {return Err("IDT vector track counts".into());}
        let mut points=Vec::with_capacity(count);
        for i in 0..count {
            // 800C80C0 clears final position Z. Eight owned UI points contain
            // NaN in this unused lane; X/Y must remain finite.
            points.push([r.f32(base+0x40+i*12)?,r.f32(base+0x44+i*12)?,0.]);
        }
        let mut sequence=Vec::with_capacity(keys);
        for i in 0..keys {
            let value=r.f32(base+0x160+i*8)?;let frame=r.u32(base+0x164+i*8)?;
            if value<0. || value>=(count as f32) || sequence.last().is_some_and(|k:&UiVectorKey|k.frame>frame) {
                return Err("IDT vector key index/time".into());
            }
            sequence.push(UiVectorKey{value,frame});
        }
        Ok(Some(UiVectorTrack{flags,points,keys:sequence}))
    }
}

#[derive(Clone, Debug)]
pub struct UiScalarTrack { pub flags:u32, pub keys:Vec<UiVectorKey> }
#[derive(Clone, Debug)]
pub struct UiAppearance {
    pub position_anchor:u32, pub rotation_anchor:u32,
    pub gauge_axis:u16,pub blend_mode:u32, pub tile_kind:u32, pub repeat:[f32;2], pub scroll_rate:u32,pub scroll_step:[i16;2],
    pub rotation:[f32;3], pub scale:Option<UiScalarTrack>,
    pub rotation_tracks:[Option<UiScalarTrack>;3],
    pub rgb:Option<(u32,Vec<([f32;3],f32)>)>, pub alpha:Option<UiScalarTrack>,
    pub rgb_flags:u32, pub alpha_flags:u32,
}
impl UiWidget {
    pub fn appearance(&self,bytes:&[u8])->Result<UiAppearance,String> {
        let r=UiReader(bytes);
        let mut result=UiAppearance{gauge_axis:if self.kind==8{r.u16(self.record_offset+0x40)?}else{1},blend_mode:0,tile_kind:0,repeat:[1.;2],scroll_rate:0,scroll_step:[0;2],position_anchor:self.channels[2].map(|o|r.u32(self.record_offset+o)).transpose()?.unwrap_or(0),rotation_anchor:self.channels[4].map(|o|r.u32(self.record_offset+o)).transpose()?.unwrap_or(0),rotation:read_array(|i|r.f32(self.record_offset+0xf0+i*4))?,scale:None,
            rotation_tracks:[None,None,None],rgb:None,alpha:None,rgb_flags:0,alpha_flags:0};
        if let Some(o)=self.channels[0] {result.blend_mode=r.u32(self.record_offset+o)?;}
        if let Some(o)=self.channels[6] {
            if self.record_bytes-o<0x1c{return Err("IDT texture style bounds".into());}
            let at=self.record_offset+o;
            result.tile_kind=r.u32(at)?;result.repeat=[r.f32(at+12)?,r.f32(at+16)?];
            if result.repeat.iter().any(|v|*v<=0.||*v>64.){return Err("IDT texture repeat bounds".into());}
            result.scroll_rate=r.u32(at+20)?;
            result.scroll_step=[r.u16(at+24)? as i16,r.u16(at+26)? as i16];
        }
        let scalar=|offset:usize,flags:u32,count_at:usize,keys_at:usize|->Result<Option<UiScalarTrack>,String> {
            if flags&2==0 {return Ok(None);}
            let base=self.record_offset+offset;
            let count=r.u32(base+count_at)? as usize;
            if count==0{return Ok(None);}
            if count>24 || keys_at+count*8>self.record_bytes-offset{return Err("IDT scalar track bounds".into());}
            let mut keys=Vec::with_capacity(count);
            for i in 0..count {
                let value=r.f32(base+keys_at+8*i)?;let frame=r.u32(base+keys_at+4+8*i)?;
                if keys.last().is_some_and(|k:&UiVectorKey|k.frame>frame){return Err("IDT scalar key order".into());}
                keys.push(UiVectorKey{value,frame});
            }
            Ok(Some(UiScalarTrack{flags,keys}))
        };
        if let Some(o)=self.channels[3] {result.scale=scalar(o,r.u32(self.record_offset+o)?,4,0x40)?;}
        if let Some(o)=self.channels[4] {
            for axis in 0..3 {result.rotation_tracks[axis]=scalar(o,r.u32(self.record_offset+o+4+axis*4)?,0x10+axis*4,0x20+axis*0xc0)?;}
        }
        if let Some(o)=self.channels[5] {
            if self.record_bytes-o<0x50{return Err("IDT appearance bounds".into());}
            let base=self.record_offset+o;
            result.rgb_flags=r.u32(base+0x40)?;result.alpha_flags=r.u32(base+0x44)?;
            if result.rgb_flags&2!=0 {
                let mut keys=Vec::with_capacity(4);
                for i in 0..4 {
                    keys.push((read_array(|j|r.f32(base+i*16+j*4))?,r.f32(base+i*16+12)?));
                }
                let mut last=0;
                for i in 1..keys.len() {if keys[i].1>keys[last].1 {last=i;}}
                keys.truncate(last+1);
                result.rgb=Some((result.rgb_flags,keys));
            }
            result.alpha=scalar(o,result.alpha_flags,0x48,0x60)?;
        }
        Ok(result)
    }
}


#[derive(Clone,Debug,Deserialize)]#[serde(rename_all="camelCase")]
pub struct StageMap {
    pub schema:String,pub pages:Vec<UiPageMap>,pub height_thresholds:Vec<f32>,
    pub pixels_per_world_unit:f32,pub texture_pixels:f32,pub origin:[f32;2],
}
#[derive(Clone,Debug,Deserialize)]pub struct UiPageMap {pub image:String,pub width:u32,pub height:u32,pub sha256:String}
impl StageMap {
    pub fn parse(json:&str)->Result<Self,String>{
        let map:Self=serde_json::from_str(json).map_err(|e|e.to_string())?;
        if map.schema!="mw-stage-map/1"||map.pages.len()!=map.height_thresholds.len()+1
            ||map.pages.is_empty()||!map.pixels_per_world_unit.is_finite()||map.pixels_per_world_unit<=0.
            ||!map.texture_pixels.is_finite()||map.texture_pixels<=0.
            ||map.origin.iter().any(|v|!v.is_finite())
            ||map.height_thresholds.iter().any(|v|!v.is_finite())
            ||map.height_thresholds.windows(2).any(|w|w[0]>=w[1])
            ||map.pages.iter().any(|p|!safe_asset(&p.image)||p.width==0||p.height==0){return Err("Stage map metadata".into());}
        Ok(map)
    }
    /// 8024803C: a threshold belongs to the lower layer; only strict greater advances.
    pub fn layer(&self,height:f32)->usize{self.height_thresholds.iter().take_while(|&&threshold|height>threshold).count()}
    /// 80247470: map world X/Z into the widget's local logical coordinates.
    pub fn project(&self,world:[f32;3],center:[f32;2],display_size:f32)->[f32;2]{
        let factor=display_size*self.pixels_per_world_unit/self.texture_pixels;
        [center[0]+(world[0]+self.origin[0])*factor,center[1]+(world[2]+self.origin[1])*factor]
    }
}
