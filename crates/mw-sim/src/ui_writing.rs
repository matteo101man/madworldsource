//! Original fixed2D writing binding. Random/actor-linked variants remain separate.
use crate::ui::UiScene;
use mw_data::{ui::UiTimeline,ui_writing::WritingRecord};

/// 802E2104 starts its deadline at30 and decrements on the creation update.
/// The deadline update frees before drawing: EV21 exposes ages0..28.
pub const CONTROLLER_TIMEOUT:u32=30;

/// 802E275C/802E1DF0/802E1FE4. The current EV21 record has zero random
/// ranges/exclusion and no X/Y rotation. Source character spacing is preserved.
pub fn fixed_scene(record:&WritingRecord,bank:&UiTimeline,bytes:&[u8])->Result<UiScene,String>{
    if record.position_random!=[0.;3]||record.rotation_random!=[0.;3]||record.exclusion!=[0.;3]||record.rotation[..2]!=[0.;2]||record.position[2]!=0.{return Err("Untranslated random/projected WCD placement".into());}
    let mut scene=UiScene::new(bank.program(record.program as u32).ok_or("Missing writing program")?,bytes,[320.,180.])?;
    let root=scene.index(1).ok_or("Missing writing root")?;
    scene.program.widgets[root].root_position=record.position;scene.program.widgets[root].size=record.scale;
    scene.program.widgets[root].color=record.rgba.map(|v|v as f32/255.);
    scene.states[root].rotation=Some(record.rotation[2]);
    let start=if record.flags&2!=0{10-record.count as usize}else if record.flags&4!=0{((11-record.count as i32)>>1)as usize}else{0};
    let group=record.glyph_group()?;let mut shift=-32.*record.count as f32;let mut first=true;
    for slot in 0usize..10{
        let index=scene.index(10+slot as u32).ok_or("Missing writing glyph slot")?;
        let glyph=slot.checked_sub(start).and_then(|i|record.glyphs.get(i)).copied().unwrap_or(-1);
        if glyph<0{scene.states[index].forced_hidden=true;continue;}
        if first{shift-=scene.program.widgets[index].child_position[0];first=false;}
        scene.program.widgets[index].uv_indices[0]=group;scene.program.widgets[index].uv_indices[1]=glyph as u16;
        scene.program.widgets[index].child_position[0]+=shift;
        scene.states[index].forced_hidden=false;
    }
    if record.position_source!=0{
        let owner=UiScene::new(bank.program(100).ok_or("Missing writing owner")?,bytes,[320.,180.])?;
        scene.bind_position_from(1,&owner,record.position_source as u32);
        // Native context3 retains both destination static axes, while the
        // owner's position animation supplies the motion envelope.
        scene.states[root].keep_position_axis=3;
    }
    Ok(scene)
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn ev21_fixed_writing_matches_native_binding_bounce_and_expiry(){
        let event=mw_data::event::EventData::parse(include_str!("../../../assets/native/events/ev21/event.json")).unwrap();
        let cues=crate::event::event_writing_cues(&event).unwrap();assert_eq!(cues,[crate::event::EventWritingCue{frame:510,id:111,position:[0.;3]}]);
        let bytes=include_bytes!("../../../assets/native/events/ev21/writing/animation.idt");let bank=UiTimeline::parse(bytes).unwrap();
        let record=WritingRecord::parse(include_bytes!("../../../assets/native/events/ev21/writing/record.wcd")).unwrap();
        let mut scene=fixed_scene(&record,&bank,bytes).unwrap();
        let indices:std::collections::HashMap<_,_>=scene.program.widgets.iter().enumerate().map(|(i,w)|(w.record_offset,i)).collect();
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let rows=std::fs::read_to_string(root.join("evidence/ev21-native-writing.jsonl")).unwrap();
        let(mut frames,mut samples,mut visible)=(0,0,0);let mut lifetime=None;
        for line in rows.lines(){
            let row:serde_json::Value=serde_json::from_str(line).unwrap();let Some(frame)=row["before"].as_u64().map(|n|n as u32)else{continue;};frames+=1;
            let age=frame.checked_sub(cues[0].frame+1);scene.seek(age.unwrap_or(0)as f32);let draws=scene.draw();let mut allocated=false;
            for native in row["writingUi"].as_array().unwrap(){
                let shown=native["hidden"]==0&&native["rgba"][3].as_f64().unwrap()>0.;
                let Some(&i)=indices.get(&(native["offset"].as_u64().unwrap()as usize))else{assert!(!shown,"unexpected visible writing owner");continue;};
                allocated=true;samples+=1;let range=lifetime.get_or_insert((frame,frame));range.1=frame;
                let d=&draws[i];assert_eq!(d.visible,shown,"frame{frame}/widget{i}");if !shown{continue;}visible+=1;
                assert_eq!(serde_json::json!(d.uv),native["uv"]);let f=|v:&serde_json::Value|v.as_f64().unwrap()as f32;
                for axis in 0..2{
                    assert!((d.position[axis]-f(&native["matrix"][if axis==0{3}else{7}])).abs()<0.0001);
                    assert!((d.scale[axis]-f(&native["size"][axis])*if axis==0{1.}else{-1.}).abs()<0.0001);
                }
                for k in 0..4{assert!((d.color[k]-f(&native["rgba"][k])).abs()<0.0001);}
                let angle=(d.rotation-f(&native["matrix"][4]).atan2(f(&native["matrix"][0]))+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
                assert!(angle.abs()<0.0001);
            }
            assert_eq!(allocated,age.is_some_and(|age|age<CONTROLLER_TIMEOUT-1),"controller lifetime frame{frame}");
        }
        assert_eq!((frames,samples,visible),(1080,319,80));assert_eq!(lifetime,Some((511,539)));
        let mut bad=event;bad.resources.iter_mut().find(|r|r.tag=="ID  ").unwrap().commands.iter_mut().find(|c|c.opcode==16).unwrap().operands[0]=1;
        assert!(crate::event::event_writing_cues(&bad).is_err());
    }
}
