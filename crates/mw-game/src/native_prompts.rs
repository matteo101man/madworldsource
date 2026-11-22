//! PC button glyphs over the original Wii remote/Nunchuk prompts (etc/core_us.idd).
//!
//! Glyphs come from tools/gen_pc_prompt_glyphs.py (native-host-data/pc-prompts). Each
//! prompt sprite slot shows the current binding of the action that Wii button performs,
//! from the keyboard/mouse or gamepad set of the last device used. The remote/Nunchuk
//! pictures follow the game's live remote prompt kind (shake vs. one swing direction).
//! The renderer draws the glyph rectangles over whatever the page shows (guest decode or
//! color pack); nothing is written to guest memory.
//!
//! On (auto) by default. `MW_PC_PROMPTS=auto|kbm|pad|off` sets the start mode; F4 cycles
//! off -> auto -> keyboard -> gamepad.
use crate::pc_config::{Action, Input, PcConfig};
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use mw_native_platform::native_renderer::{TexturePatch, TexturePatches};
use std::{collections::BTreeMap, path::Path, sync::{Arc, Mutex}};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode { #[default] Off, Auto, Keyboard, Gamepad }
impl Mode {
    fn parse(s: &str) -> Option<Mode> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "off" | "0" => Mode::Off,
            "auto" | "1" | "on" => Mode::Auto,
            "kbm" | "keyboard" => Mode::Keyboard,
            "pad" | "gamepad" => Mode::Gamepad,
            _ => return None,
        })
    }
    fn next(self) -> Mode {
        match self { Mode::Off => Mode::Auto, Mode::Auto => Mode::Keyboard, Mode::Keyboard => Mode::Gamepad, Mode::Gamepad => Mode::Off }
    }
}

/// Patch set waiting for the render world (`blit_native` hands it to the renderer).
pub static PENDING: Mutex<Option<TexturePatches>> = Mutex::new(None);

struct Glyph { width: u32, height: u32, rgba: Arc<[u8]> }

#[derive(Clone, Copy)]
struct Slot { page: u32, rect: [u32; 4], role: &'static str }

/// Remote prompt kinds (cockpit child +0x18). setSwingNavigation stores 0x0C + its enum:
/// 0 Y+ (down) .. 3 X- (left), 4 shake, 5 keep shaking (pad.rs SHAKE_PROMPTS = 0x11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Gesture { Shake, Down, Up, Right, Left }
fn gesture(kind: u8) -> Gesture {
    match kind { 0x0C => Gesture::Down, 0x0D => Gesture::Up, 0x0E => Gesture::Right, 0x0F => Gesture::Left, _ => Gesture::Shake }
}

#[derive(Resource)]
pub struct Prompts {
    pub mode: Mode,
    pub gamepad: bool,
    glyphs: BTreeMap<String, BTreeMap<u32, Glyph>>,
    pages: Vec<(u32, Vec<String>)>,
    slots: Vec<Slot>,
    shown: Option<(Mode, bool, Gesture, u64)>,
}

impl Prompts {
    fn load(root: &Path) -> Result<Prompts, String> {
        let read = |name: &str| std::fs::read(root.join(name)).map_err(|e| format!("{name}: {e}"));
        let index: serde_json::Value = serde_json::from_slice(&read("glyphs.json")?).map_err(|e| e.to_string())?;
        let blob = read("glyphs.rgba")?;
        let mut glyphs = BTreeMap::new();
        for (name, sizes) in index["glyphs"].as_object().ok_or("glyphs.json: no glyphs")? {
            for (size, g) in sizes.as_object().ok_or("glyphs.json: bad entry")? {
                let (o, w, h) = (g["offset"].as_u64(), g["width"].as_u64(), g["height"].as_u64());
                let (Some(o), Some(w), Some(h), Ok(size)) = (o, w, h, size.parse::<u32>()) else { return Err(format!("glyph {name}")) };
                let bytes = blob.get(o as usize..(o + w * h * 4) as usize).ok_or(format!("glyph {name} out of range"))?;
                glyphs.entry(name.clone()).or_insert_with(BTreeMap::new)
                    .insert(size, Glyph { width: w as u32, height: h as u32, rgba: bytes.into() });
            }
        }
        let pages_json: serde_json::Value = serde_json::from_slice(&read("pages.json")?).map_err(|e| e.to_string())?;
        let mut pages = Vec::new();
        for (page, list) in pages_json["pages"].as_object().ok_or("pages.json: no pages")? {
            let names = list.as_array().into_iter().flatten().filter_map(|e| e["name"].as_str().map(String::from)).collect();
            pages.push((page.parse().map_err(|_| "pages.json: page id")?, names));
        }
        let mut slots = Vec::new();
        for s in pages_json["slots"].as_array().ok_or("pages.json: no slots")? {
            let rect: Vec<u32> = s["rect"].as_array().into_iter().flatten().filter_map(|v| v.as_u64().map(|v| v as u32)).collect();
            let role = match s["role"].as_str() {
                Some(r) => ROLES.iter().copied().find(|x| *x == r).ok_or(format!("unknown slot role {r}"))?,
                None => return Err("slot role".into()),
            };
            let page = s["page"].as_u64().ok_or("slot page")? as u32;
            slots.push(Slot { page, rect: rect.try_into().map_err(|_| "slot rect")?, role });
        }
        Ok(Prompts { mode: Mode::Off, gamepad: false, glyphs, pages, slots, shown: None })
    }

    /// Largest generated glyph of `name` that fits `max` px.
    fn glyph(&self, name: &str, max: u32) -> Option<&Glyph> {
        self.glyphs.get(name)?.range(..=max).next_back().map(|(_, g)| g)
    }

    /// Input names of the first binding of `actions` for the device set, single inputs first.
    fn binding(cfg: &PcConfig, actions: &[Action], pad: bool) -> Option<Vec<String>> {
        let mut chords: Vec<_> = actions.iter().flat_map(|a| cfg.bindings.get(*a)).filter(|c| c.0.iter().all(|i| i.is_gamepad() == pad)).collect();
        chords.sort_by_key(|c| c.0.len());
        chords.first().map(|c| c.0.iter().map(|i| Input::name(*i)).collect())
    }

    /// Glyph images (top to bottom) for one slot.
    fn compose(&self, cfg: &PcConfig, role: &str, pad: bool, g: Gesture, w: u32, h: u32) -> Vec<&Glyph> {
        let one = |actions: &[Action]| -> Vec<&Glyph> {
            let Some(names) = Self::binding(cfg, actions, pad) else { return vec![] };
            // Square slots show the last input of a chord (e.g. the stick of RB+stick).
            names.last().and_then(|n| self.glyph(n, w.min(h))).into_iter().collect()
        };
        let tall = |actions: &[Action], word: bool| -> Vec<&Glyph> {
            let Some(names) = Self::binding(cfg, actions, pad) else { return vec![] };
            let mut out = Vec::new();
            for (i, n) in names.iter().enumerate() {
                if i > 0 {
                    out.extend(self.glyph("WordPlus", w / 2));
                }
                if let Some(x) = self.glyph(n, w) { out.push(x); }
            }
            if word {
                out.extend(self.glyph("WordShake", w));
            }
            out
        };
        match role {
            "attack" => one(&[Action::Attack, Action::Confirm]),
            "chainsaw" => one(&[Action::Chainsaw, Action::Back]),
            "lock_on" => one(&[Action::LockOn]),
            "jump" => one(&[Action::Jump]),
            "plus" => one(&[Action::Plus]),
            "pause" => one(&[Action::Pause]),
            "pickup" => one(&[Action::Pickup]),
            "move" => {
                let wasd = ["KeyW", "KeyA", "KeyS", "KeyD"].map(String::from).to_vec();
                let keys: Vec<_> = [Action::MoveUp, Action::MoveLeft, Action::MoveDown, Action::MoveRight]
                    .iter().filter_map(|a| Self::binding(cfg, &[*a], false).and_then(|v| v.into_iter().next())).collect();
                match (pad, keys == wasd) {
                    (true, _) => self.glyph("PadLStick", w.min(h)).into_iter().collect(),
                    (false, true) => self.glyph("MoveKeys", w.min(h)).into_iter().collect(),
                    (false, false) => one(&[Action::MoveUp]),
                }
            }
            "remote" => {
                let arm = match g {
                    Gesture::Shake => None,
                    Gesture::Down => Some(Action::RightArmDown),
                    Gesture::Up => Some(Action::RightArmUp),
                    Gesture::Right => Some(Action::RightArmRight),
                    Gesture::Left => Some(Action::RightArmLeft),
                };
                match arm { Some(a) => tall(&[a], false), None => tall(&[Action::Shake], true) }
            }
            _ => vec![],
        }
    }

    /// Build page overlays for the current state.
    fn patches(&self, cfg: &PcConfig, pad: bool, g: Gesture) -> TexturePatches {
        let mut out = TexturePatches::default();
        for (page, names) in &self.pages {
            let (pw, ph) = (1024u32, 1024u32);
            let mut rgba = vec![0u8; (pw * ph * 4) as usize];
            let mut rects = Vec::new();
            for slot in self.slots.iter().filter(|s| s.page == *page) {
                let [x0, y0, x1, y1] = slot.rect;
                let (w, h) = (x1 - x0, y1 - y0);
                // The Nunchuk pictures are removed outright (left transparent); the remote's
                // Shake binding already shakes both arms on PC.
                if slot.role == "nunchuk" {
                    rects.push(slot.rect);
                    continue;
                }
                let stack = self.compose(cfg, slot.role, pad, g, w, h);
                if stack.is_empty() {
                    continue; // unbound: the original Wii glyph stays
                }
                let gap = 2;
                let total: u32 = stack.iter().map(|g| g.height).sum::<u32>() + gap * (stack.len() as u32 - 1);
                let mut y = y0 + h.saturating_sub(total) / 2;
                for glyph in stack {
                    let x = x0 + (w - glyph.width.min(w)) / 2;
                    for row in 0..glyph.height.min(y1.saturating_sub(y)) {
                        let src = &glyph.rgba[(row * glyph.width * 4) as usize..][..(glyph.width.min(w) * 4) as usize];
                        let at = (((y + row) * pw + x) * 4) as usize;
                        rgba[at..at + src.len()].copy_from_slice(src);
                    }
                    y += glyph.height + gap;
                }
                rects.push(slot.rect);
            }
            if rects.is_empty() {
                continue;
            }
            let patch = TexturePatch { width: pw, height: ph, rgba: rgba.into(), rects: rects.into() };
            for name in names {
                out.insert(name.clone(), patch.clone());
            }
        }
        out
    }
}

const ROLES: [&str; 11] = ["attack", "chainsaw", "lock_on", "jump", "plus", "pause", "move", "pickup", "remote", "nunchuk", "none"];

pub fn plugin(app: &mut App) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../native-host-data/pc-prompts");
    let mut prompts = match Prompts::load(&root) {
        Ok(p) => p,
        Err(e) => {
            warn!("[pc-prompts] glyphs unavailable ({e}); run tools/gen_pc_prompt_glyphs.py");
            return;
        }
    };
    prompts.mode = std::env::var("MW_PC_PROMPTS").ok().and_then(|v| Mode::parse(&v)).unwrap_or(Mode::Auto);
    app.insert_resource(prompts).add_systems(Update, update);
}

fn bindings_hash(cfg: &PcConfig) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (a, chords) in &cfg.bindings.map {
        a.name().hash(&mut h);
        for c in chords { c.name().hash(&mut h); }
    }
    h.finish()
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut prompts: ResMut<Prompts>,
    cfg: Res<PcConfig>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    pads: Query<&Gamepad>,
) {
    if keys.just_pressed(KeyCode::F4) {
        prompts.mode = prompts.mode.next();
        info!("[pc-prompts] mode {:?}", prompts.mode);
    }
    // Last device used: any key, mouse button or wheel vs. any pad button or a deflected stick.
    let kbm = keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() || wheel.read().next().is_some();
    let pad = pads.iter().any(|p| {
        p.get_just_pressed().next().is_some() || p.left_stick().length() > 0.5 || p.right_stick().length() > 0.5
            || [GamepadButton::LeftTrigger2, GamepadButton::RightTrigger2].iter().any(|b| p.get(*b).is_some_and(|v| v > 0.5))
    });
    if pad && !prompts.gamepad {
        prompts.gamepad = true;
    } else if kbm && !pad && prompts.gamepad {
        prompts.gamepad = false;
    }
    let mode = prompts.mode;
    let device_pad = match mode { Mode::Keyboard => false, Mode::Gamepad => true, _ => prompts.gamepad };
    let kind = mw_native_platform::host::remote_prompt();
    let g = gesture(kind);
    let state = (mode, device_pad, g, bindings_hash(&cfg));
    if prompts.shown == Some(state) {
        return;
    }
    if prompts.shown.is_none_or(|s| s.2 != g) && kind != 0 {
        info!("[pc-prompts] remote prompt kind {kind:#04x} -> {g:?}");
    }
    let patches = if mode == Mode::Off { TexturePatches::default() } else { prompts.patches(&cfg, device_pad, g) };
    prompts.shown = Some(state);
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(patches);
}

/// Headless evidence (native_gpu.rs): `MW_GPU_TEST_PROMPT_CAPTURE=<dir>` saves the XFB with
/// keyboard, gamepad and original (Wii) glyphs each time the game raises a new remote
/// prompt kind, when the PC test enters a shake phase, and every `MW_GPU_TEST_PROMPT_EVERY`
/// seconds (default 20).
#[cfg(test)]
pub(crate) struct CaptureProbe {
    prompts: Prompts,
    cfg: PcConfig,
    dir: std::path::PathBuf,
    every: f64,
    next: f64,
    last_kind: u8,
    last_phase: String,
    // (set at, step) of a running keyboard -> gamepad -> original sequence.
    run: Option<(f64, usize, u8)>,
    pub(crate) saved: Vec<String>,
}
#[cfg(test)]
impl CaptureProbe {
    /// Arms the renderer's texture naming; call before the session starts.
    pub(crate) fn from_env(renderer: &mw_native_platform::native_renderer::sink::Renderer) -> Option<CaptureProbe> {
        let dir = std::path::PathBuf::from(std::env::var_os("MW_GPU_TEST_PROMPT_CAPTURE")?);
        std::fs::create_dir_all(&dir).ok()?;
        let prompts = Prompts::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../native-host-data/pc-prompts")).expect("pc-prompts glyphs");
        renderer.set_texture_patches(TexturePatches::default());
        let every = std::env::var("MW_GPU_TEST_PROMPT_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(20.);
        Some(CaptureProbe { prompts, cfg: PcConfig::default(), dir, every, next: every, last_kind: 0, last_phase: String::new(), run: None, saved: vec![] })
    }
    pub(crate) fn tick(&mut self, renderer: &mw_native_platform::native_renderer::sink::Renderer, t: f64, phase: &str) {
        const SETTLE: f64 = 0.5;
        let kind = mw_native_platform::host::remote_prompt();
        let entered = phase != self.last_phase && phase.contains("shake");
        self.last_phase = phase.to_string();
        if self.run.is_none() && ((kind != 0 && kind != self.last_kind) || entered || t >= self.next) {
            self.next = t + self.every;
            self.run = Some((t, 0, kind));
            renderer.set_texture_patches(self.prompts.patches(&self.cfg, false, gesture(kind)));
        }
        self.last_kind = kind;
        let Some((at, step, k)) = self.run else { return };
        if t < at + SETTLE {
            return;
        }
        let label = ["kbm", "pad", "wii"][step];
        if let Some(f) = renderer.capture_xfb() {
            let name = format!("t{:06.1}-kind{k:02x}-{label}.ppm", t);
            let mut ppm = format!("P6 {} {} 255\n", f.width, f.height).into_bytes();
            ppm.extend(f.rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]));
            if std::fs::write(self.dir.join(&name), ppm).is_ok() {
                println!("[pc-prompts] capture {name} live-kind={kind:#04x}");
                self.saved.push(name);
            }
        }
        match step {
            0 => renderer.set_texture_patches(self.prompts.patches(&self.cfg, true, gesture(k))),
            1 => renderer.set_texture_patches(TexturePatches::default()),
            _ => {}
        }
        self.run = (step < 2).then_some((t, step + 1, k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn prompts() -> Option<Prompts> {
        Prompts::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../native-host-data/pc-prompts")).ok()
    }
    #[test]
    fn default_bindings_pick_expected_glyphs() {
        let Some(p) = prompts() else { return };
        let cfg = PcConfig::default();
        let names = |actions: &[Action], pad| Prompts::binding(&cfg, actions, pad).unwrap();
        assert_eq!(names(&[Action::Attack, Action::Confirm], false), ["MouseLeft"]);
        assert_eq!(names(&[Action::Attack, Action::Confirm], true), ["PadA"]);
        assert_eq!(names(&[Action::Shake], true), ["PadL3"]);
        assert_eq!(names(&[Action::RightArmUp], true), ["PadRB", "PadRStickUp"]);
        // Every slot role resolves to at least one glyph in both device sets.
        for pad in [false, true] {
            for g in [Gesture::Shake, Gesture::Down, Gesture::Up, Gesture::Right, Gesture::Left] {
                for role in ["attack", "chainsaw", "lock_on", "jump", "plus", "pause", "move", "pickup", "remote"] {
                    assert!(!p.compose(&cfg, role, pad, g, 46, 154).is_empty(), "{role} pad={pad} {g:?}");
                }
            }
        }
        let patches = p.patches(&cfg, true, Gesture::Up);
        assert_eq!(patches.len(), 6, "three pages, disc and staged names");
        // Nunchuk slots are patched with nothing but transparency.
        for slot in p.slots.iter().filter(|s| s.role == "nunchuk") {
            let page = p.pages.iter().find(|(id, _)| *id == slot.page).unwrap();
            let patch = &patches[&page.1[0]];
            assert!(patch.rects.contains(&slot.rect));
            let [x0, y0, x1, y1] = slot.rect;
            for y in y0..y1 {
                for x in x0..x1 {
                    assert_eq!(patch.rgba[((y * 1024 + x) * 4 + 3) as usize], 0);
                }
            }
        }
    }
    #[test]
    fn modes_cycle_and_parse() {
        assert_eq!(Mode::parse("pad"), Some(Mode::Gamepad));
        assert_eq!(Mode::Off.next().next().next().next(), Mode::Off);
        assert_eq!(gesture(0x11), Gesture::Shake);
        assert_eq!(gesture(0x0E), Gesture::Right);
    }
}
