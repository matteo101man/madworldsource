//! Headless fight runs for the oracle diff (no Bevy, no collision): the test autopilot plays full fights and the
//! per-tick state is written in mw-play's log layout.
//! `cargo run -p mw-sim --example sim_fight -- <out_dir> <runs>`; variants: every 3rd run starts the player at 1 HP without dodging
//! (death -> continue), every 4th drops the shake input during clashes (clash loss).
use mw_data::MotionBank;
use mw_sim::fight::{Autopilot, CombatData, Fight, FightData};
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = std::path::PathBuf::from(args.get(1).map(String::as_str).unwrap_or("evidence/native-play"));
    let runs: u64 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(6);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
    let read = |p: &str| std::fs::read_to_string(root.join(p)).expect(p);
    let d = FightData::new(MotionBank::parse(&read("jack-player-motion.json")).unwrap(), MotionBank::parse(&read("rinrin-motion.json")).unwrap(),
                           CombatData::parse(&read("combat.json")).unwrap());
    for seed in 1..=runs {
        let mut f = Fight::new(&d, [2.5, 0., -61.], std::f32::consts::PI, [2.3, 0., -68.6], seed);
        if seed % 3 == 0 { f.player.hp = 1; }
        let mut bot = Autopilot::default();
        let mut log = std::fs::File::create(out.join(format!("sim-{seed}.log.jsonl"))).unwrap();
        let (mut deaths, mut lost, mut won) = (0, 0, 0);
        for _ in 0..30000 {
            let mut inp = bot.input(&f, std::f32::consts::PI);
            if seed % 3 == 0 { inp.nunchuk_shake = 0.; }
            if seed % 4 == 0 && f.player.state() == 78 && lost == 0 { inp.shake = 0.; inp.shake_remote = 0.; inp.shake_nunchuk = 0.; }
            f.step_free(&d, &inp);
            let ev: Vec<String> = f.events.iter().map(|e| format!("{e:?}")).collect();
            deaths += ev.iter().filter(|e| *e == "PlayerDead").count();
            lost += ev.iter().filter(|e| *e == "ClashLost").count();
            won += ev.iter().filter(|e| *e == "ClashWon").count();
            let p = &f.player; let r = &f.rin;
            let row = serde_json::json!({"tick": f.tick, "state": p.state(), "clip": d.jack.clips[p.motion.clip].name, "rate": p.motion.rate,
                "frame": p.motion.frame, "hp": p.hp, "events": ev,
                "rin": {"state": r.state, "clip": d.rin.clips[r.motion.clip].name, "hp": r.hp, "frame": r.motion.frame}});
            writeln!(log, "{row}").unwrap();
            if f.victory { break; }
        }
        println!("seed {seed}: victory {} at tick {} deaths {deaths} clashes won {won} lost {lost}", f.victory, f.tick);
    }
}
