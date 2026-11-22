//! Native player slice entry point (research/NATIVE-PORT-GOAL.md phase 1). Pure Rust; no recompiled core.
#[path = "../play.rs"]
mod play;
fn main() { play::main() }
