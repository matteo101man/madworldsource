//! Compare the original fixed2D THUD binding against native WC widget state.
use mw_data::{ui::UiTimeline,ui_writing::WritingRecord};
use serde_json::{json,Value};
fn main(){
    let args:Vec<_>=std::env::args().collect();let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes=std::fs::read(root.join("assets/native/events/ev21/writing/animation.idt")).unwrap();let bank=UiTimeline::parse(&bytes).unwrap();
    let record=WritingRecord::parse(&std::fs::read(root.join("assets/native/events/ev21/writing/record.wcd")).unwrap()).unwrap();
    let mut scene=mw_sim::ui_writing::fixed_scene(&record,&bank,&bytes).unwrap();
    if args.len()<3{
        for w in &scene.program.widgets{println!("id{} offset{} flags{:x} parent{:?} root{:?} child{:?} size{:?}",w.id,w.record_offset,w.flags,w.parent,w.root_position,w.child_position,w.size);}
        return;
    }
    let indices:std::collections::HashMap<_,_>=scene.program.widgets.iter().enumerate().map(|(i,w)|(w.record_offset,i)).collect();
    let(mut frames,mut observations,mut visible)=(0,0,0);let mut errors=[0f32;4];let mut first=vec![];let mut lifetime=None;
    for line in std::fs::read_to_string(&args[1]).unwrap().lines(){
        let row:Value=serde_json::from_str(line).unwrap();let Some(frame)=row["before"].as_u64()else{continue;};frames+=1;
        scene.seek(frame.saturating_sub(511)as f32);let draws=scene.draw();
        for w in row["writingUi"].as_array().unwrap(){
            let Some(&i)=indices.get(&(w["offset"].as_u64().unwrap()as usize))else{continue;};observations+=1;
            let range=lifetime.get_or_insert((frame,frame));range.1=frame;
            let d=&draws[i];let shown=w["hidden"]==0&&w["rgba"][3].as_f64().unwrap()>0.;
            assert_eq!(d.visible,shown,"visibility frame{frame}/widget{i}");
            if !shown{continue;}visible+=1;assert_eq!(json!(d.uv),w["uv"]);
            let f=|v:&Value|v.as_f64().unwrap()as f32;
            let pos=(d.position[0]-f(&w["matrix"][3])).abs().max((d.position[1]-f(&w["matrix"][7])).abs());
            let size=(d.scale[0]-f(&w["size"][0])).abs().max((d.scale[1]+f(&w["size"][1])).abs());
            let color=(0..4).map(|k|(d.color[k]-f(&w["rgba"][k])).abs()).fold(0.,f32::max);
            let angle=(d.rotation-f(&w["matrix"][4]).atan2(f(&w["matrix"][0]))+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
            for(k,e)in [pos,size,color,angle.abs()].into_iter().enumerate(){errors[k]=errors[k].max(e);}
            if first.len()<8&&[pos,size,color,angle.abs()].iter().any(|&e|e>0.0001){first.push(json!({"frame":frame,"id":d.id,"rustPosition":d.position,"nativeMatrix":w["matrix"],"errors":[pos,size,color,angle]}));}
        }
    }
    let passed=(frames,observations,visible)==(1080,319,80)&&lifetime==Some((511,539))&&errors.iter().all(|&e|e<0.0001);
    let report=json!({"passed":passed,"frames":frames,"observations":observations,"visible":visible,"lifetime":lifetime,"maxErrors":errors,"firstErrors":first,"scope":"Fixed2D WCD111 native pre-update edges. Source time=before-511. Native allocation/removal and all glyph poses compared; draw-time/pixel parity remains separate."});
    std::fs::write(&args[2],serde_json::to_string_pretty(&report).unwrap()).unwrap();println!("{report}");
    assert!(passed,"native writing-character regression");
}
