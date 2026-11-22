//! Inspect the original EV21 UI bank and compare source-time geometry with
//! clock-edge native widget observations. Reports errors; does not hide them.
use mw_data::ui::UiTimeline;
use mw_sim::ui::UiScene;
use serde_json::{json,Value};
use std::collections::HashMap;

fn main(){
    let args:Vec<_>=std::env::args().collect();let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes=std::fs::read(root.join("assets/native/events/ev21/ui/animation.idt")).unwrap();let bank=UiTimeline::parse(&bytes).unwrap();
    let mut widgets=HashMap::new();let mut summary=vec![];
    for p in &bank.programs{
        let mut ends=vec![];
        for(i,w)in p.widgets.iter().enumerate(){
            widgets.insert(w.record_offset,(p.id,i));let a=w.appearance(&bytes).unwrap();
            ends.push(json!({"index":i,"offset":w.record_offset,"flags":format!("{:x}",w.flags),"uv":&w.uv_indices[..2],
                "positionEnd":w.position_track(&bytes).unwrap().and_then(|t|t.keys.last().map(|k|k.frame)),
                "scaleEnd":a.scale.and_then(|t|t.keys.last().map(|k|k.frame)),"alphaEnd":a.alpha.and_then(|t|t.keys.last().map(|k|k.frame))}));
        }summary.push(json!({"program":p.id,"widgets":ends}));
    }
    if args.len()<3{println!("{}",serde_json::to_string_pretty(&summary).unwrap());return;}
    let mut errors:HashMap<u32,(usize,f32,f32,f32,f32)>=HashMap::new();let mut lifetimes:HashMap<u32,(u64,u64)>=HashMap::new();let mut first=vec![];
    for line in std::fs::read_to_string(&args[1]).unwrap().lines(){
        let row:Value=serde_json::from_str(line).unwrap();let Some(frame)=row["before"].as_u64()else{continue;};
        for native in row["eventUi"].as_array().unwrap(){
            let offset=native["offset"].as_u64().unwrap()as usize;let &(id,index)=widgets.get(&offset).expect("original UI widget offset");
            let range=lifetimes.entry(id).or_insert((frame,frame));range.1=frame;
            let start=match id{1=>260,2=>350,3=>392,100=>0,_=>continue};
            let mut scene=UiScene::new(bank.program(id).unwrap(),&bytes,[320.,180.]).unwrap();
            scene.seek(frame.saturating_sub(start+1)as f32);let draw=&scene.draw()[index];
            if id!=100 {assert_eq!(draw.visible,native["hidden"]==0&&native["rgba"][3].as_f64().unwrap()>0.,"visibility frame{frame}/program{id}/widget{index}");assert_eq!(json!(draw.uv),native["uv"]);}
            if native["hidden"]!=0||native["rgba"][3].as_f64().unwrap()<=0.||!draw.visible{continue;}
            let f=|v:&Value|v.as_f64().unwrap()as f32;
            let pos=(draw.position[0]-f(&native["matrix"][3])).abs().max((draw.position[1]-f(&native["matrix"][7])).abs());
            let size=(draw.scale[0]-f(&native["size"][0])).abs().max((draw.scale[1]+f(&native["size"][1])).abs());
            let rgba=(0..4).map(|i|(draw.color[i]-f(&native["rgba"][i])).abs()).fold(0.,f32::max);
            let rotation=(draw.rotation-f(&native["matrix"][4]).atan2(f(&native["matrix"][0]))+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
            let e=errors.entry(id).or_default();e.0+=1;e.1=e.1.max(pos);e.2=e.2.max(size);e.3=e.3.max(rgba);e.4=e.4.max(rotation.abs());
            if first.len()<12&&(pos>0.001||size>0.001||rgba>0.001||rotation.abs()>0.001){first.push(json!({"frame":frame,"program":id,"index":index,"clocks":native["clocks"],"sourceFrame":frame.saturating_sub(start+1),"positionError":pos,"sizeError":size,"colorError":rgba,"rotationError":rotation}));}
        }
    }
    let passed=first.is_empty()&&errors.values().map(|e|e.0).sum::<usize>()==400;
    let report=json!({"source":summary,"lifetimes":lifetimes,"errors":errors,"firstErrors":first,"passed":passed,"scope":"1080 pre-update native clock edges; all requested program UV/visibility and400 visible glyph transform/color observations. Edge N observes UI from update N-1; source programs advance once per event frame. Program100 is invisible owner scaffolding. First draw-time alignment and pixel/compositing parity remain unverified."});
    std::fs::write(&args[2],serde_json::to_string_pretty(&report).unwrap()).unwrap();println!("{}",json!({"lifetimes":lifetimes,"errors":errors,"firstErrors":first}));
    assert!(passed,"event UI geometry regression");
}
