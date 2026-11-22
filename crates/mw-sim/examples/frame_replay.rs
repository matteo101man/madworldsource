//! Same-input replay of a recompiled-core frame recording (`MW_FRAME_RECORD`, research/FRAME-COMPARE.md) into
//! `mw_sim::fight::Fight`. Each recorded frame's input, as the original player saw it after its own pad copy, becomes
//! one `FightInput`. The sim starts from the recorded positions, facings and HP and from the shared cRand word
//! (0x80609B90) read when the recording armed. It writes one JSON row per frame; `tools/pl0000/frame_compare.py`
//! compares those rows with the recording.
//!
//! cargo run -p mw-sim --example frame_replay -- <record.jsonl> <native.jsonl> [--start N --player-clip C --rin-clip C]
//! `--start N` begins at recorded frame N: state/phase (+0x11B/+0x11C), motion clip (resolved by frame_compare.py)
//! and frame, position, facing and HP of both actors come from that row, and the seed is that row's cRand word.
//! Internals the recording does not hold (Rin's attack cooldowns, scaffold timers, input buffers) keep their
//! `Fight::new` values.
use mw_data::MotionBank;
use mw_sim::fight::{CombatData, Fight, FightData, FightInput};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

fn f(v: &Value) -> f32 { v.as_f64().unwrap_or(0.) as f32 }
fn hex(v: &Value) -> u32 { u32::from_str_radix(v.as_str().unwrap_or("0"), 16).unwrap_or(0) }
fn vec3(v: &Value) -> [f32; 3] { [f(&v[0]), f(&v[1]), f(&v[2])] }

/// Game pad bits after remap (PL0000-COMBAT.md "Input buffers"): A 0x10, B 0x20, jump 0x1000. Triggers `+0xEE4`,
/// holds `+0xEE0`; remote accel `+0xEF0`/`+0xEF4` (> 2 is a swing, |y| > 2 vertical); stick `+0xF1C/+0xF20` and its
/// world yaw `+0x113C`; shake metrics `+0x1088` / `+0x1090` / `+0x109C`; nunchuk shake `+0xF64`.
fn input(row: &Value, camera_yaw: &mut f32) -> FightInput {
    let i = &row["input"];
    let (hold, trig) = (hex(&i["hold"]), hex(&i["trig"]));
    let stick = [f(&i["stick"][0]), f(&i["stick"][1])];
    // 802C7144 turns toward +0x113C only past 0.2; below that the yaw is stale, so keep the last derived camera.
    if stick[0] * stick[0] + stick[1] * stick[1] > 0.04 { *camera_yaw = f(&i["stickYaw"]) - (-stick[0]).atan2(stick[1]); }
    let (ax, ay) = (f(&i["accX"]), f(&i["accY"]));
    FightInput {
        demo_active: row["demo"].as_i64().unwrap_or(0) < 0,
        stick, camera_yaw: *camera_yaw,
        a_trig: trig & 0x10 != 0, a_held: hold & 0x10 != 0, b_trig: trig & 0x20 != 0, b_held: hold & 0x20 != 0,
        swing: if ay.abs() > 2. { 1 } else if ax.abs() > 2. { 3 } else { 0 },
        shake: f(&i["m1088"]), shake_remote: f(&i["m1090"]), shake_nunchuk: f(&i["m109c"]),
        cont: false, jump: trig & 0x1000 != 0, nunchuk_shake: f(&i["f64"]),
        heavy: false, grab: false, grab_grunt: false, whip: false,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (record, out) = (args.get(1).expect("record.jsonl"), args.get(2).expect("native.jsonl"));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap_or_else(|e| panic!("{p}: {e}"));
    let d = FightData::new(MotionBank::parse(&read("jack-player-motion.json")).unwrap(), MotionBank::parse(&read("rinrin-motion.json")).unwrap(),
                           CombatData::parse(&read("combat.json")).unwrap());
    let mut d = d;
    if args.iter().any(|a| a == "--collision") {
        d.world = Some(mw_data::collision::CollisionWorld::parse(&read("collision/world.json")).expect("collision"));
    }
    let mut head = None;
    let mut rows = vec![];
    for line in std::io::BufReader::new(std::fs::File::open(record).expect("record")).lines() {
        let v: Value = serde_json::from_str(&line.unwrap()).expect("json row");
        if v.get("recording").is_some() { head = Some(v); } else if v.get("frame").is_some() { rows.push(v); }
    }
    let head = head.expect("recording header");
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let start: u64 = opt("--start").map(|s| s.parse().expect("--start")).unwrap_or(0);
    let rows: Vec<Value> = rows.into_iter().filter(|r| r["frame"].as_u64().unwrap_or(0) >= start).collect();
    let first = rows.first().expect("frames");
    let seed = if start == 0 { hex(&head["seedAtStart"]) } else { hex(&first["seed"]) };
    let (pp, rp) = (vec3(&first["player"]["pos"]), vec3(&first["rin"]["pos"]));
    // Row `start` is the state after that frame; the sim's first step then reproduces frame start + 1.
    let mut fight = Fight::new(&d, pp, f(&first["player"]["facing"]), rp, seed as u64);
    fight.player.loco.facing = f(&first["player"]["facing"]);
    fight.player.hp = first["player"]["hp"].as_i64().unwrap_or(100) as i16;
    fight.rin.facing = f(&first["rin"]["facing"]);
    fight.rin.hp = first["rin"]["hp"].as_i64().unwrap_or(1600) as i16;
    if start > 0 {
        let byte = |a: &Value, i: usize| a["state"][i].as_u64().unwrap_or(0) as u8;
        fight.player.loco.state = byte(&first["player"], 1); fight.player.loco.phase = byte(&first["player"], 2);
        fight.rin.state = byte(&first["rin"], 1); fight.rin.phase = byte(&first["rin"], 2);
        fight.rin.on_scaffold = rp[1] > pp[1] + 3.;
        if let Some(c) = opt("--player-clip") {
            fight.player.motion = mw_sim::fight::Motion::new(d.jack_clip(&format!("{c}.mot")), opt("--player-looping").map(|v| v == "1").unwrap_or(fight.player.motion.looping));
            fight.player.motion.frame = f(&first["player"]["frame"]);
        }
        if let Some(c) = opt("--rin-clip") {
            fight.rin.motion = mw_sim::fight::Motion::new(d.rin_clip(&format!("{c}.mot")), opt("--rin-looping").map(|v| v == "1").unwrap_or(fight.rin.motion.looping));
            fight.rin.motion.frame = f(&first["rin"]["frame"]);
        }
    }
    let mut camera_yaw = f(&first["player"]["facing"]);
    let mut w = std::io::BufWriter::new(std::fs::File::create(out).expect("out"));
    let snap = |fight: &Fight, frame: u64, events: Vec<String>| {
        let (pl, rin) = (&fight.player, &fight.rin);
        let mut row = json!({"frame": frame, "rng": format!("{:08x}", fight_rng(fight)),
               "player": {"state": pl.state(), "phase": pl.loco.phase, "clip": d.jack.clips[pl.motion.clip].name, "frame": pl.motion.frame, "hp": pl.hp, "pos": pl.pos, "facing": pl.loco.facing},
               "rin": {"state": rin.state, "phase": rin.phase, "clip": d.rin.clips[rin.motion.clip].name, "frame": rin.motion.frame, "hp": rin.hp, "pos": rin.pos, "facing": rin.facing},
               "events": events});
        if args.iter().any(|a| a == "--contacts") {
            let poses = mw_sim::fight::bone_frames(&d.jack.bones, &d.jack_poses(&pl.motion));
            let hands = mw_sim::fight::bone_frames(&d.rin.bones, &d.rin_poses(&rin.motion));
            row["contacts"] = json!({"parts": mw_sim::fight::PLAYER_DAMAGE_PARTS.iter().map(|&(i, _)| json!({"index":i,"world":mw_sim::fight::to_world(pl.pos,pl.loco.facing,poses[i].1)})).collect::<Vec<_>>(),
                "fans":rin.fans.iter().map(|f|{
                    let i=d.rin.bones.iter().position(|b|b.id==f.hand).expect("fan hand");let(q,p,s)=hands[i];
                    // The same quaternion-vector product as bone_frames, with
                    // scale and actor yaw, expressed as the recorder's 3x4.
                    let norm=q.iter().map(|v|v*v).sum::<f32>();
                    let axis=|j:usize|{let u=[q[0],q[1],q[2]];let mut v=[0.;3];v[j]=s[j];let dot=u[j]*s[j];let cross=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];mw_sim::fight::to_world([0.;3],rin.facing,[0,1,2].map(|k|2.*dot*u[k]+(2.*q[3]*q[3]-norm)*v[k]+2.*q[3]*cross[k]))};
                    let columns=[axis(0),axis(1),axis(2)];let origin=mw_sim::fight::to_world(rin.pos,rin.facing,p);
                    let matrix:[f32;12]=std::array::from_fn(|k|if k%4==3{origin[k/4]}else{columns[k%4][k/4]});
                    json!({"phase":f.phase,"t":f.t,"dt":f.step_size(),"world":f.pos,"local":f.local_position(),"target":f.world_target(),"handMatrix":matrix})
                }).collect::<Vec<_>>()});
        }
        row
    };
    writeln!(w, "{}", snap(&fight, first["frame"].as_u64().unwrap_or(0), vec![])).unwrap();
    let reseed = args.iter().any(|a| a == "--reseed");
    let mut prev_seed = seed;
    let mut prev_demo = first["demo"].as_i64().unwrap_or(0);
    for row in &rows[1..] {
        // --reseed: start each frame from the original's cRand word at the end of the previous frame (the original
        // draws ~20 times per frame across all systems; this aligns the starting state, not the draw order)
        if reseed { fight.rng = mw_sim::fight::Rng::new(prev_seed as u64); }
        prev_seed = hex(&row["seed"]);
        let mut inp = input(row, &mut camera_yaw);
        inp.demo_active = prev_demo < 0;
        prev_demo = row["demo"].as_i64().unwrap_or(0);
        fight.step_world(&d, &inp);
        let events = fight.events.iter().map(|e| format!("{e:?}")).collect();
        writeln!(w, "{}", snap(&fight, row["frame"].as_u64().unwrap_or(0), events)).unwrap();
    }
    eprintln!("replayed {} frames, seed {seed:08x} -> {out}", rows.len() - 1);
}

/// The sim's cRand word, for comparing draw counts with the recorded 0x80609B90.
fn fight_rng(f: &Fight) -> u32 { let s = format!("{:?}", f.rng); s.trim_start_matches("Rng(").trim_end_matches(')').parse().unwrap_or(0) }
