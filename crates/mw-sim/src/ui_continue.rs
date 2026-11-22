//! cIdGameOver (layout id 3, vtable 805C9A80, update 8023491C, open(mode) 80235034): the continue window after a
//! death, subscr0ff program 0 (layer 15) over its grain program 3 (layer 13); NO (or no continues left) runs
//! cIdGameOverEnd (layout id 16, update 80235070): subscr0ff program 1, static with "YOU LOSE" and a TV switch-off,
//! done when widget 10's scale ends. All art is core_us sprites (groups 47/48 words, 15 digits, 22/20 the head).
//! States (+28, jump table 805C9A60): 0 wait for the YES entry's opening, 1 select, 2 confirm (flash binding), 3 five
//! flashes then close, 4 wait for the close, 5-6 the game-over end, 7 closed.
//! [inferred: the channel numbers passed to 800CEB9C are read as ui_challenge's bind_rgb / bind_scale]
use crate::ui::UiScene;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Opening, Selecting, Flashing, Closing, Ending, Done }
#[derive(Default, Clone, Copy)]
pub struct Input { pub yes: bool, pub no: bool, pub back: bool, pub confirm: bool }

/// Widgets of program 0: roots 1 (close scale source 2), cursor 9, YES 11, NO 12, flash colour 13, counter root 20,
/// digits 21 (tens) / 22 (ones).
const ROOT: u32 = 1;
const CLOSE: u32 = 2;
const CURSOR: u32 = 9;
const YES: u32 = 11;
const NO: u32 = 12;
const FLASH: u32 = 13;
const COUNTER: u32 = 20;
const TENS: u32 = 21;
const ONES: u32 = 22;
/// Program 1 (the game-over end) is done when widget 10's scale ends.
const END_DONE: u32 = 10;

pub struct ContinueWindow { pub scene: UiScene, pub noise: UiScene, pub end: UiScene, pub phase: Phase, pub yes: bool,
                            pub continues: u32, pub result: bool, age: u32, flashes: u32 }

impl ContinueWindow {
    /// open(0) with `continues` left; with none it opens straight into the game-over end (open(1): state 5).
    pub fn new(scene: UiScene, noise: UiScene, end: UiScene, continues: u32) -> Self {
        let mut w = Self { scene, noise, end, phase: if continues == 0 { Phase::Ending } else { Phase::Opening }, yes: true, continues,
                           result: false, age: 0, flashes: 0 };
        w.scene.hide_tree(CURSOR, true);
        w.show_count();
        w
    }
    /// +42 below 10: widget 22's frame is the count and 21 hidden; else both digits (80222E40(this, 21, 2)).
    fn show_count(&mut self) {
        let n = self.continues.min(99);
        self.scene.hide_tree(TENS, n < 10);
        if n < 10 { self.scene.digit(ONES, n as u16); } else { self.scene.number(TENS, 2, n); }
    }
    /// The window (and its grain) is on screen.
    pub fn window_visible(&self) -> bool { matches!(self.phase, Phase::Opening | Phase::Selecting | Phase::Flashing | Phase::Closing) }
    pub fn step(&mut self, input: Input) {
        if self.phase == Phase::Done { return; }
        self.age += 1;
        if self.phase == Phase::Ending {
            self.end.advance(1.);
            let done = self.end.index(END_DONE).is_some_and(|i| self.end.states[i].stopped & 8 != 0);
            if done || self.age > 240 { self.phase = Phase::Done; self.result = false; }
            return;
        }
        self.scene.advance(1.); self.noise.advance(1.);
        match self.phase {
            Phase::Opening => if self.age >= 24 { self.phase = Phase::Selecting; self.age = 0; self.scene.hide_tree(CURSOR, false); },
            Phase::Selecting => {
                if input.yes { self.yes = true; } if input.no || input.back { self.yes = false; }
                // the cursor copies the selected entry's child position plus its evaluated track offset (as ui_challenge)
                let selected = if self.yes { YES } else { NO };
                if let (Some(i), Some(cursor)) = (self.scene.index(selected), self.scene.index(CURSOR)) {
                    let w = &self.scene.program.widgets[i];
                    let offset = w.position_track(self.scene.bytes()).ok().flatten()
                        .and_then(|t| crate::ui::vector_at(&t, self.scene.states[i].clocks.position).ok()).unwrap_or([0.; 3]);
                    self.scene.states[cursor].position = Some([w.child_position[0] + offset[0], w.child_position[1] + offset[1]]);
                }
                if input.confirm {
                    for id in [CURSOR, selected] { self.scene.bind_rgb(id, FLASH); }
                    if self.yes { self.continues -= 1; self.show_count(); }
                    self.result = self.yes; self.phase = Phase::Flashing; self.age = 0;
                }
            }
            Phase::Flashing => {
                // five flashes, then the root closes (scale from widget 2) and the counter goes
                if self.age % 5 == 0 {
                    self.flashes += 1;
                    if self.flashes >= 5 { self.scene.bind_scale(ROOT, CLOSE); self.scene.hide_tree(COUNTER, true); self.phase = Phase::Closing; self.age = 0; }
                    else { for id in [CURSOR, if self.yes { YES } else { NO }] { if let Some(i) = self.scene.index(id) { self.scene.states[i].clocks.rgb = 0.; } } }
                }
            }
            Phase::Closing => if self.age >= 6 { if self.result { self.phase = Phase::Done; } else { self.phase = Phase::Ending; self.age = 0; } },
            _ => {}
        }
    }
}
