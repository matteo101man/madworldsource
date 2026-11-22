//! Compare source actor/cut poses, attachment roots and cancelled meshes with
//! read-only MW_EVENT_ACTORS observations. Camera comparison is separate.
use mw_data::{event::EventData,Bone,MotionBank};
use mw_sim::{event::{EventPlayer,actor_state},character::{sample_retargeted,Retarget},fight::bone_frames};
use serde_json::{Value,json};
use std::{collections::HashMap,path::Path};

fn matrix(q:[f32;4],p:[f32;3],s:[f32;3])->[f32;12]{
    let[x,y,z,w]=q;
    [(1.-2.*y*y-2.*z*z)*s[0],2.*(x*y-w*z)*s[1],2.*(x*z+w*y)*s[2],p[0],
     2.*(x*y+w*z)*s[0],(1.-2.*x*x-2.*z*z)*s[1],2.*(y*z-w*x)*s[2],p[1],
     2.*(x*z-w*y)*s[0],2.*(y*z+w*x)*s[1],(1.-2.*x*x-2.*y*y)*s[2],p[2]]
}
fn compose(a:[f32;12],b:[f32;12])->[f32;12]{
    std::array::from_fn(|i|{let(r,c)=(i/4,i%4);(0..3).map(|k|a[r*4+k]*b[k*4+c]).sum::<f32>()+if c==3{a[r*4+3]}else{0.}})
}
fn error(actual:&Value,expected:[f32;12])->f32{actual.as_array().unwrap().iter().zip(expected).map(|(a,b)|(a.as_f64().unwrap()as f32-b).abs()).fold(0.,f32::max)}
fn main(){
    let args:Vec<String>=std::env::args().collect();let record=args.get(1).expect("native event actors.jsonl");let output=args.get(2).expect("comparison.json");
    let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let read=|p:&str|std::fs::read_to_string(root.join(p)).unwrap();
    let event=EventData::parse(&read("assets/native/events/ev21/event.json")).unwrap();let mut player=EventPlayer::new(&event).unwrap();
    let models:Value=serde_json::from_str(&read("assets/native/events/ev21/models.json")).unwrap();let mut rigs:HashMap<String,Vec<Bone>>=HashMap::new();
    for model in models["actors"].as_array().unwrap(){rigs.insert(model["object"].as_str().unwrap().into(),serde_json::from_value(model["bones"].clone()).unwrap());}
    rigs.insert("em0440".into(),MotionBank::parse(&read("assets/native/rinrin-motion.json")).unwrap().bones);
    let mut rows=0;let(mut visibility,mut mesh_errors,mut meshes,mut parts,mut roots,mut attachment_checks,mut attachment_errors,mut initial_bindings)=(0,0,0,0,0,0,0,0);let(mut root_error,mut part_error)=(0f32,0f32);let mut first_bad=vec![];
    for line in std::fs::read_to_string(record).unwrap().lines(){
        let row:Value=serde_json::from_str(line).unwrap();let Some(frame)=row["before"].as_u64()else{continue;};assert_eq!(frame,rows);rows+=1;player.clock.seek_global(frame as u32);
        let mut evaluated=HashMap::new();
        for actor in &event.actors{if let Some(clip)=player.actor_motion(&event,&actor.object,actor.instance){let bones=&rigs[&actor.object];let poses=sample_retargeted(bones,clip,player.clock.local_frame()as f32,Retarget::NATIVE);let root=poses[bones.len()];evaluated.insert(actor.object.clone(),(matrix(root.rotation,root.translation,root.scale),bone_frames(bones,&poses)));}}
        for actor in row["actors"].as_array().expect("MW_EVENT_ACTORS capture"){
            let name=actor["name"].as_str().unwrap();let object=name.strip_suffix("_00").expect("source actor instance0");let expected=evaluated.get(object);let state=actor_state(&event,object,0,frame as u32).unwrap();
            if actor["visible"].as_bool().unwrap()!=expected.is_some(){visibility+=1;}
            attachment_checks+=1;
            if let Some(attachment)=&state.attachment{
                let parent=row["actors"].as_array().unwrap().iter().find(|a|a["name"]==attachment.target).unwrap();let part=parent["parts"].as_array().unwrap().iter().find(|p|p["bone"]==attachment.bone).unwrap();
                if actor["attachment"]!=part["pointer"]{attachment_errors+=1;}
            }else if actor["attachment"]!="00000000"{attachment_errors+=1;}
            for mesh in actor["meshes"].as_array().unwrap(){meshes+=1;if mesh["cancelled"].as_bool().unwrap()!=state.mesh_cancelled(mesh["name"].as_str().unwrap()){mesh_errors+=1;}}
            let Some((local_root,frames))=expected else{continue;};
            let attachment=state.attachment.as_ref().map(|a|{let object=a.target.strip_suffix("_00").unwrap();let target=&evaluated[object];let i=rigs[object].iter().position(|b|b.id as u32==a.bone).unwrap();let(q,p,s)=target.1[i];matrix(q,p,s)});
            // At creation the native command binds AFTER the first parts
            // update; this clock-edge observation still contains an unbound
            // identity root. Its draw-time first-frame behavior is unverified.
            if frame==0&&attachment.is_some(){initial_bindings+=1;continue;}
            let expected_root=attachment.map(|a|compose(a,*local_root)).unwrap_or(*local_root);let e=error(&actor["world"],expected_root);root_error=root_error.max(e);roots+=1;
            if e>0.001&&first_bad.len()<12{first_bad.push(json!({"frame":frame,"actor":object,"kind":"root","maxError":e,"actual":actor["world"],"expected":expected_root}));}
            for part in actor["parts"].as_array().unwrap(){let id=part["bone"].as_u64().unwrap();let i=rigs[object].iter().position(|b|b.id as u64==id).unwrap();let(q,p,s)=frames[i];let local=matrix(q,p,s);let world=attachment.map(|a|compose(a,local)).unwrap_or(local);let e=error(&part["world"],world);part_error=part_error.max(e);parts+=1;
                if e>0.001&&first_bad.len()<12{first_bad.push(json!({"frame":frame,"actor":object,"kind":"part","bone":id,"maxError":e,"actual":part["world"],"expected":world}));}
            }
        }
    }
    let report=json!({"schema":"mw-event-actor-comparison/1","record":record,"frames":rows,"visibilityMismatches":visibility,"attachmentChecks":attachment_checks,"attachmentMismatches":attachment_errors,"meshChecks":meshes,"meshMismatches":mesh_errors,"rootChecks":roots,"partChecks":parts,"creationAttachmentRowsExcludedFromTransformComparison":initial_bindings,"maxRootError":root_error,"maxPartError":part_error,"firstBad":first_bad,"scope":"Source pose/attachment/mesh-state comparison. At creation command14 binds after the initial parts update; two unbound root observations are excluded only from transform checks. Their first draw-time placement remains unverified. Selected bone matrices, not all bones/pixels or complete lifecycle."});
    std::fs::write(output,serde_json::to_string_pretty(&report).unwrap()+"\n").unwrap();println!("frames={rows} visibility={visibility} mesh={mesh_errors}/{meshes} root={root_error} part={part_error}");
    assert_eq!(rows,1080);assert_eq!(visibility,0);assert_eq!(mesh_errors,0);assert_eq!(attachment_errors,0);assert_eq!(initial_bindings,2);assert!(root_error<0.001&&part_error<0.001,"native actor transforms still differ; inspect comparison");
}
