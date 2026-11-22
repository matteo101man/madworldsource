//! Native HUD gauge arithmetic800C5508 and resource initialization800C5120.
//! Full controller admission, delayed bars and HUD rendering are separate.
#[derive(Clone,Copy,Debug)]pub struct Gauge{pub value:f32,pub speed:f32}
impl Gauge{
    pub fn new(value:f32,speed:f32)->Self{assert!(value.is_finite()&&speed.is_finite()&&speed>0.);Self{value,speed}}
    /// The native completion test follows the first increment/decrement. It
    /// snaps within one speed of the target, including a small overshoot.
    pub fn approach(&mut self,target:f32)->bool{
        assert!(target.is_finite());if self.value==target{return true;}
        if self.value<=target{self.value+=self.speed;}else{self.value-=self.speed;}
        let done=(self.value-target).abs()<=self.speed;if done{self.value=target;}done
    }
}
/// Shared HP two-bar progression from80223D28(player)/802242D4(boss).
/// Admission, RGB/envelope restarts and low-health effects remain controller work.
pub struct HealthMeter{pub front:Gauge,pub trail:Gauge,pub committed:f32,pub delay:u8}
impl HealthMeter{
    pub fn from_scene(scene:&crate::ui::UiScene,initial:f32)->Self{
        let rate=|id|scene.program.widgets[scene.index(id).expect("HP widget")].gauge_step(scene.bytes()).expect("native horizontal HP gauge");
        Self{front:Gauge::new(initial,rate(10)),trail:Gauge::new(initial,rate(11)),committed:initial,delay:0}
    }
    pub fn step(&mut self,ratio:f32){
        assert!(ratio.is_finite()&&(0. ..=1.).contains(&ratio));
        if self.committed<=ratio{
            if ratio<=self.committed{self.delay=0;}
            else if self.front.approach(ratio){self.committed=ratio;self.trail.value=ratio;}
        }else{
            self.delay=self.delay.wrapping_add(1);
            if self.delay>30&&self.trail.approach(ratio){self.committed=ratio;}
            self.front.value=ratio;
        }
    }
    pub fn apply(&self,scene:&mut crate::ui::UiScene){
        for(id,value)in [(10,self.front.value),(11,self.trail.value)]{let i=scene.index(id).unwrap();scene.states[i].gauge=Some(value);}
    }
}
/// 80224158 threshold admission and80223D28's variant/effect transitions.
/// Program58's disabled geometry and zero-alpha flash are preserved from the
/// original data. Mode4 reverses the current root alpha towards its start.
pub struct LowHealthEffects{pub scene:crate::ui::UiScene,pub active:bool,pub alpha_age:f32}
impl LowHealthEffects{
    pub fn new(mut scene:crate::ui::UiScene)->Self{assert_eq!(scene.program.id,58);for s in &mut scene.states{s.stopped=0xfc;}Self{scene,active:false,alpha_age:0.}}
    pub fn step(&mut self,ratio:f32,damaged:bool,panel:&mut crate::ui::UiScene){
        self.scene.advance(1.);let low=ratio<0.3;
        if damaged&&low{self.scene.restart_tree(2);}
        if low&&!self.active{panel.bind_variant(1,Some(0)).expect("native HP low variant");self.alpha_age=0.;}
        else if !low&&self.active{panel.bind_variant(1,None).expect("native HP normal variant");}
        else if low{self.alpha_age=(self.alpha_age+1.).min(5.);}
        else{self.alpha_age=(self.alpha_age-1.).max(0.);}
        self.active=low;let i=self.scene.index(1).unwrap();self.scene.states[i].clocks.alpha=self.alpha_age;
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn original_helper_matches_all_owned_native_scratch_calls(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-hud-gauge-observed.json")).unwrap()).unwrap();
        let rows=observed["rows"].as_array().unwrap();assert_eq!(rows.len(),400);
        for r in rows{
            let mut gauge=Gauge::new(r["before"].as_f64().unwrap()as f32,r["speed"].as_f64().unwrap()as f32);
            assert_eq!(gauge.approach(r["target"].as_f64().unwrap()as f32),r["done"].as_bool().unwrap());
            assert_eq!(gauge.value.to_bits(),(r["after"].as_f64().unwrap()as f32).to_bits(),"native case{} step{}",r["case"],r["step"]);
        }
    }
    #[test]fn player_and_boss_rates_come_from_original_widget_widths(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/core_us/animation.idt")).unwrap();let bank=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        for(program,width)in [(31,320.),(39,100.)]{for(id,denominator)in [(10,20.),(11,30.)]{let w=bank.program(program).unwrap().widgets.iter().find(|w|w.id==id).unwrap();assert_eq!(w.gauge_step(&bytes).unwrap().to_bits(),((100_f32/denominator)/width).to_bits());}}
    }
    #[test]fn live_player_and_boss_prepared_geometry_matches_source_gauges(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/core_us/animation.idt")).unwrap();let bank=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-hud-observed.json")).unwrap()).unwrap();let atlas=mw_data::ui::UiLayout::parse(&std::fs::read_to_string(root.join("assets/native/ui/core_us/layout.json")).unwrap()).unwrap();let mut checked=0;
        for capture in observed["captures"].as_array().unwrap(){
            let mut scene=crate::ui::UiScene::new(bank.program(capture["program"].as_u64().unwrap()as u32).unwrap(),&bytes,[320.,180.]).unwrap();
            // Healing binds template12; later damage restores the immutable
            // template10, which must agree with the native settled gold bar.
            scene.bind_rgb(10,12);scene.bind_rgb(10,10);
            for b in capture["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let w=&b["observed"];
                let s=&mut scene.states[i];let clocks=&w["channelFrames"];s.clocks.position=clocks["position"].as_f64().unwrap()as f32;s.clocks.scale=clocks["scale"].as_f64().unwrap()as f32;s.clocks.rgb=clocks["rgb"].as_f64().unwrap()as f32;s.clocks.alpha=clocks["alpha"].as_f64().unwrap()as f32;
                if w["kind"]==8{s.gauge=Some(w["gauge"]["value"].as_f64().unwrap()as f32);}
            }
            let draws=scene.draw();
            for missing in capture["missingIndices"].as_array().unwrap(){let i=missing.as_u64().unwrap()as usize;assert_ne!(scene.program.widgets[i].flags&0x80000,0,"native non-draw reference list");assert!(!draws[i].visible,"reference-only widget rendered");}
            for b in capture["bindings"].as_array().unwrap(){let i=b["index"].as_u64().unwrap()as usize;let w=&b["observed"];let d=&draws[i];
                if w["hidden"]!=0||w["rgba"][3].as_f64().unwrap()<=0.||w["size"][0]==0||w["size"][1]==0{continue;}
                for axis in 0..2{
                    let(center,size)=if w["kind"]==8{let v=w["preparedVertices"].as_array().unwrap();let values:Vec<_>=v.iter().skip(axis).step_by(3).map(|v|v.as_f64().unwrap()as f32).collect();let lo=values.iter().copied().fold(f32::INFINITY,f32::min);let hi=values.iter().copied().fold(f32::NEG_INFINITY,f32::max);((lo+hi)*0.5,hi-lo)}else{(w["matrix1f8"][if axis==0{3}else{7}].as_f64().unwrap()as f32,w["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.})};
                    assert!((d.position[axis]-center).abs()<0.0003,"program{} index{i} position{axis}: {:?} vs {center}",capture["program"],d.position);assert!((d.scale[axis]-size).abs()<0.0003,"program{} index{i} size{axis}: {:?} vs {size}",capture["program"],d.scale);
                }for axis in 0..4{assert!((d.color[axis]-w["rgba"][axis].as_f64().unwrap()as f32).abs()<0.00001,"program{} index{i} color{axis} {:?} vs {}",capture["program"],d.color,w["rgba"]);}
                if w["kind"]==8{
                    let group=atlas.groups.iter().find(|g|g.id==d.uv[0]).unwrap();let(_,sprite)=atlas.sprite(group.first+d.uv[1]as usize).unwrap();let uv=sprite.uv;
                    let cropped=[uv[0]+(uv[2]-uv[0])*d.uv_crop[0],uv[1]+(uv[3]-uv[1])*d.uv_crop[1],uv[0]+(uv[2]-uv[0])*d.uv_crop[2],uv[1]+(uv[3]-uv[1])*d.uv_crop[3]];
                    for axis in 0..4{assert!((cropped[axis]-w["gaugeUvBounds"][axis].as_f64().unwrap()as f32).abs()<0.000001,"program{} index{i} primary UV{axis}",capture["program"]);}
                }
                checked+=1;
            }
        }assert_eq!(checked,17);
    }

    #[test]fn low_health_sequence_matches_original_controller_and_prepared_panel(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let bytes=std::fs::read(root.join("assets/native/ui/core_us/animation.idt")).unwrap();let bank=mw_data::ui::UiTimeline::parse(&bytes).unwrap();
        let observed:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("research/ui-hud-low-observed.json")).unwrap()).unwrap();
        let mut panel=crate::ui::UiScene::new(bank.program(39).unwrap(),&bytes,[320.,180.]).unwrap();for s in &mut panel.states{s.stopped=0xfc;}
        let mut effects=LowHealthEffects::new(crate::ui::UiScene::new(bank.program(58).unwrap(),&bytes,[320.,180.]).unwrap());let mut meter=HealthMeter::from_scene(&panel,1.);meter.apply(&mut panel);let mut previous=1.;let mut published=1.;let rows=observed["rows"].as_array().unwrap();assert_eq!(rows.len(),340);
        for(row,r)in rows.iter().enumerate(){
            if row>0{
                panel.advance(1.);let ratio=published;let damaged=ratio<previous;
                if damaged{panel.bind_rgb(10,10);}if ratio>meter.committed{panel.bind_rgb(10,12);}
                meter.step(ratio);meter.apply(&mut panel);if damaged{panel.restart(1);}
                effects.step(ratio,damaged,&mut panel);previous=ratio;
            }
            assert_eq!(effects.active,r["low"]==1,"row{row} low state");assert_eq!(meter.delay,r["delay"].as_u64().unwrap()as u8,"row{row} delay");assert!((meter.committed-r["committed"].as_f64().unwrap()as f32).abs()<0.00001,"row{row} committed");
            let draw=panel.draw();let effect_draw=effects.scene.draw();assert!(effect_draw.iter().all(|d|!d.visible),"Program58 disabled English geometry became visible");
            for w in r["widgets"].as_array().unwrap(){let i=w["index"].as_u64().unwrap()as usize;let d=if w["program"]==39{&draw[i]}else{&effect_draw[i]};
                if w["program"]==58{
                    if w["id"]==1{assert!((d.color[3]-w["rgba"][3].as_f64().unwrap()as f32).abs()<0.00001,"row{row} low-effect alpha {} vs {}",d.color[3],w["rgba"][3]);}continue;
                }
                if w["hidden"]!=0||w["rgba"][3].as_f64().unwrap()<=0.||w["size"][0]==0||w["size"][1]==0{continue;}
                for axis in 0..4{assert!((d.color[axis]-w["rgba"][axis].as_f64().unwrap()as f32).abs()<0.000001,"row{row} index{i} color{axis} {:?} vs {}",d.color,w["rgba"]);}
                for axis in 0..2{
                    let(center,size)=if matches!(w["id"].as_u64(),Some(10|11)){let vals:Vec<_>=w["vertices"].as_array().unwrap().iter().skip(axis).step_by(3).map(|v|v.as_f64().unwrap()as f32).collect();let lo=vals.iter().copied().fold(f32::INFINITY,f32::min);let hi=vals.iter().copied().fold(f32::NEG_INFINITY,f32::max);((lo+hi)*0.5,hi-lo)}else{(w["matrix"][if axis==0{3}else{7}].as_f64().unwrap()as f32,w["size"][axis].as_f64().unwrap()as f32*if axis==0{1.}else{-1.})};
                    assert!((d.position[axis]-center).abs()<0.0003,"row{row} index{i} position{axis}");assert!((d.scale[axis]-size).abs()<0.0003,"row{row} index{i} size{axis}: {:?} vs {size}",d.scale);
                }
            }
            published=r["target"].as_f64().unwrap()as f32;
        }
    }

}
