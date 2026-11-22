//! Compare source-driven Jack damage bones with native world positions, without replay drift.
//! Input is a frame recording resolved by tools/pl0000/contact_pose_compare.py.
use mw_data::MotionBank;
use mw_sim::{character::{filter_native_pose, sample_retargeted, strip_root_xz, Retarget}, fight::{bone_frames, to_world}};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
    let bank = MotionBank::parse(&std::fs::read_to_string(root.join("jack-player-motion.json")).unwrap()).unwrap();
    let mut out = std::io::BufWriter::new(std::fs::File::create(&args[2]).unwrap());
    let mut previous = vec![];
    let mut last_name = String::new();
    let mut idle_start = None;
    for line in std::io::BufReader::new(std::fs::File::open(&args[1]).unwrap()).lines() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let p = &row["player"];
        let name = format!("{}.mot", p["clips"][0].as_str().unwrap());
        let Some(clip) = bank.clips.iter().find(|c| c.name == name) else { continue; };
        let f = |v: &Value| v.as_f64().unwrap() as f32;
        let pos = [f(&p["pos"][0]), f(&p["pos"][1]), f(&p["pos"][2])];
        let frame = f(&p["frame"]);
        if name != last_name {
            idle_start = if name == "pl0030_0000.mot" && last_name == "pl0030_1090.mot" {
                Some(row["frame"].as_u64().unwrap())
            } else { None };
        }
        let idle_age = idle_start.map(|start| row["frame"].as_u64().unwrap() - start);
        let remaining = if idle_age.is_some_and(|age| age < 5) { (5. - frame).max(0.) } else { 0. };
        filter_native_pose(&bank.bones, clip, frame, remaining, &mut previous);
        last_name = name.clone();
        let variants: Vec<_> = [0., 1., 2.].into_iter().map(|delay| {
            let at = if delay == 2. { frame } else { (frame - delay * f(&p["step"])).max(0.) };
            let mut pose = if delay == 2. { previous.clone() } else { sample_retargeted(&bank.bones, clip, at, Retarget::NATIVE) };
            strip_root_xz(&bank.bones, &mut pose);
            let frames = bone_frames(&bank.bones, &pose);
            let errors: Vec<_> = row["contacts"]["parts"].as_array().unwrap().iter().map(|part| {
                let i = part["index"].as_u64().unwrap() as usize;
                let world = to_world(pos, f(&p["facing"]), frames[i].1);
                let delta = [0, 1, 2].map(|j| world[j] - f(&part["world"][j]));
                json!({"index": i, "delta": delta, "distance": delta.iter().map(|x| x*x).sum::<f32>().sqrt()})
            }).collect();
            json!({"delay": delay, "sampleFrame": at, "parts": errors})
        }).collect();
        writeln!(out, "{}", json!({"frame":row["frame"], "clip": name, "idleAge":idle_age, "variants":variants})).unwrap();
    }
}
