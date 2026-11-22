//! Inspection: list each program of a staged UI source with the UV (group, index) pairs its widgets draw.
//! `cargo run -p mw-game --example ui_dump -- <ui key> [group filter]`
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let key = args.get(1).expect("ui key");
    let filter: Option<u16> = args.get(2).and_then(|v| v.parse().ok());
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native/ui").join(key);
    let bytes = std::fs::read(folder.join("animation.idt")).expect("idt");
    let timeline = mw_data::ui::UiTimeline::parse(&bytes).expect("timeline");
    for p in &timeline.programs {
        let Ok(scene) = mw_sim::ui::UiScene::new(p, &bytes, [320., 180.]) else { continue; };
        let mut draws: Vec<(u32, [u16; 2], [f32; 2], [f32; 2])> = scene.draw().iter().map(|d| (d.id, d.uv, d.position, d.scale)).collect();
        if let Some(g) = filter { if !draws.iter().any(|d| d.1[0] == g) { continue; } }
        draws.dedup();
        if std::env::var_os("UI_DUMP_FULL").is_some() {
            println!("program {}", p.id);
            for d in scene.draw() { println!("  idx {} id {} kind {} uv {:?} pos {:?} scale {:?} blend {} color {:?} tile {}", d.index, d.id, d.kind, d.uv, d.position, d.scale, d.blend_mode, d.color, d.tile_kind); }
        } else {
            println!("program {} widgets {}: {:?}", p.id, p.widgets.len(), draws.iter().map(|d| (d.0, d.1)).collect::<Vec<_>>());
        }
    }
}
