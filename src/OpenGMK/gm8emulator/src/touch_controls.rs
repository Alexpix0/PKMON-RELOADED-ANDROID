//! On-screen touch controls for the Android port.
//!
//! The game only understands keyboard input, so each on-screen control injects the key the game
//! expects. The key set is the one the game binds by default:
//!
//! - arrows: move
//! - `Z` confirm/act, `X` cancel, `C`/`V` extra actions (fishing/bike, attacks 3/4)
//! - `1`..`6`: pick a party Pokémon in battle (and act as the HM shortcuts / berries)
//! - `B` pokéball, `S` switch, `M` mega, `E` run, `A` walk/assign, `N` use item
//! - `ENTER` menus, `ESC` quit
//!
//! Movement is an analog joystick (bottom-left); the rest are buttons. Positions are normalised to
//! the window so the input hit-test and the renderer share the exact same layout.
//!
//! Controls are grouped by the game's current state (see [current_context]): the title/menu rooms
//! only need the basics, the overworld needs the joystick and interaction keys, and battle needs
//! the combat extras. The emulator reports the current room every frame.

use ramen::input::Key;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};

/// Which game states a control is shown in.
pub const MENU: u8 = 1;
pub const WORLD: u8 = 2;
pub const BATTLE: u8 = 4;
/// A dialogue box is open: only the button that advances it is shown, so nothing covers the text.
pub const DIALOG: u8 = 8;

#[derive(Clone, Copy, PartialEq)]
pub enum Shape {
    Circle { cx: f32, cy: f32, r: f32 },
    Rect { cx: f32, cy: f32, w: f32, h: f32 },
}

#[derive(Clone, Copy, PartialEq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Label {
    None,
    Arrow(Dir),
    Text(&'static str),
}

pub struct Control {
    /// Stable name, used as the key when the layout is saved and loaded. Reordering the array of
    /// controls must not move a player's customised button, so the name is what identifies it.
    pub name: &'static str,
    pub key: Key,
    pub shape: Shape,
    pub label: Label,
    /// Bitmask of [MENU] / [WORLD] / [BATTLE] states this control belongs to.
    pub contexts: u8,
}

/// The joystick is not a button but is customised (and saved) the same way.
pub const JOYSTICK_NAME: &str = "joystick";
pub const RADIAL_MENU_NAME: &str = "radial_menu";
pub const TURBO_NAME: &str = "turbo";
pub const MUTE_NAME: &str = "mute";
pub const EDIT_NAME: &str = "edit";
pub const GUIDE_NAME: &str = "guide";
pub const LOG_NAME: &str = "log";

pub const CONTROLS: &[Control] = &[
    // Trigger button for the Drastic-style Radial Quick-Menu (at the bottom center)
    Control { name: RADIAL_MENU_NAME, key: Key::ScrollLock, shape: Shape::Circle { cx: 0.500, cy: 0.958, r: 0.024 }, label: Label::Arrow(Dir::Down), contexts: WORLD | BATTLE | MENU },
    // Number keys (first row along the top). They are for the base game only: in battle those keys
    // pick a party Pokémon, and that is what the SEL wheel below is for, so showing them there would
    // only get in the way.
    Control { name: "alpha1", key: Key::Alpha1, shape: Shape::Circle { cx: 0.300, cy: 0.055, r: 0.020 }, label: Label::Text("1"), contexts: WORLD },
    Control { name: "alpha2", key: Key::Alpha2, shape: Shape::Circle { cx: 0.372, cy: 0.055, r: 0.020 }, label: Label::Text("2"), contexts: WORLD },
    Control { name: "alpha3", key: Key::Alpha3, shape: Shape::Circle { cx: 0.444, cy: 0.055, r: 0.020 }, label: Label::Text("3"), contexts: WORLD },
    // Menu / misc keys (second row along the top).
    Control { name: "space", key: Key::Space, shape: Shape::Circle { cx: 0.300, cy: 0.145, r: 0.022 }, label: Label::Text("PKM"), contexts: WORLD | BATTLE },
    Control { name: "o", key: Key::O, shape: Shape::Circle { cx: 0.372, cy: 0.145, r: 0.022 }, label: Label::Text("O"), contexts: WORLD | BATTLE },
    Control { name: "m", key: Key::M, shape: Shape::Circle { cx: 0.444, cy: 0.145, r: 0.022 }, label: Label::Text("M"), contexts: WORLD | BATTLE },
    Control { name: "e", key: Key::E, shape: Shape::Circle { cx: 0.516, cy: 0.145, r: 0.022 }, label: Label::Text("E"), contexts: WORLD | BATTLE },
    Control { name: "t", key: Key::T, shape: Shape::Circle { cx: 0.588, cy: 0.145, r: 0.022 }, label: Label::Text("T"), contexts: WORLD | BATTLE },
    Control { name: "l", key: Key::L, shape: Shape::Circle { cx: 0.660, cy: 0.145, r: 0.022 }, label: Label::Text("L"), contexts: WORLD | BATTLE },
    Control { name: "p", key: Key::P, shape: Shape::Circle { cx: 0.732, cy: 0.145, r: 0.022 }, label: Label::Text("P"), contexts: WORLD | BATTLE },
    // Main actions, in the diamond a gamepad's face buttons use: the main one at the bottom, cancel
    // to the right, and the other two on the left and on top. The cluster is about the size of the
    // movement joystick and sits at the same height, so the two thumbs fall in the same place.
    Control { name: "z", key: Key::Z, shape: Shape::Circle { cx: 0.865, cy: 0.851, r: 0.033 }, label: Label::Text("Z"), contexts: WORLD | BATTLE },
    Control { name: "x", key: Key::X, shape: Shape::Circle { cx: 0.933, cy: 0.700, r: 0.033 }, label: Label::Text("X"), contexts: WORLD | BATTLE },
    Control { name: "c", key: Key::C, shape: Shape::Circle { cx: 0.797, cy: 0.700, r: 0.033 }, label: Label::Text("C"), contexts: WORLD | BATTLE },
    Control { name: "v", key: Key::V, shape: Shape::Circle { cx: 0.865, cy: 0.549, r: 0.033 }, label: Label::Text("V"), contexts: WORLD | BATTLE },
    // Secondary.
    Control { name: "b", key: Key::B, shape: Shape::Circle { cx: 0.712, cy: 0.840, r: 0.026 }, label: Label::Text("B"), contexts: BATTLE },
    Control { name: "s", key: Key::S, shape: Shape::Circle { cx: 0.712, cy: 0.700, r: 0.026 }, label: Label::Text("S"), contexts: BATTLE },
    // Holding this opens the party wheel (see below): each slot is one Pokémon of the team.
    Control { name: PARTY_NAME, key: Key::Alpha1, shape: Shape::Circle { cx: 0.300, cy: 0.760, r: 0.038 }, label: Label::Text("SEL"), contexts: BATTLE },
    Control { name: "a", key: Key::A, shape: Shape::Circle { cx: 0.640, cy: 0.900, r: 0.024 }, label: Label::Text("A"), contexts: WORLD },
    Control { name: "n", key: Key::N, shape: Shape::Circle { cx: 0.640, cy: 0.760, r: 0.024 }, label: Label::Text("N"), contexts: WORLD },
    // Menu keys (START & SELECT flanking the center quick menu arrow).
    Control { name: "escape", key: Key::Escape, shape: Shape::Circle { cx: 0.405, cy: 0.940, r: 0.026 }, label: Label::Text("ESC"), contexts: WORLD | BATTLE },
    Control { name: "return", key: Key::Return, shape: Shape::Circle { cx: 0.595, cy: 0.940, r: 0.026 }, label: Label::Text("OK"), contexts: WORLD | BATTLE },
    // While a dialogue box is open the overlay steps aside and leaves only this, so the text and the
    // portrait are not buried under buttons. Its position is measured from the game image (not the
    // window), so it sits on the box whatever the screen's shape is. It sends the key the game
    // advances dialogues with.
    Control { name: "dialogo", key: Key::Z, shape: Shape::Circle { cx: 0.940, cy: 0.880, r: 0.024 }, label: Label::Text("OK"), contexts: DIALOG },
];

static CUSTOM_OFFSETS: Mutex<Option<HashMap<String, (f32, f32)>>> = Mutex::new(None);

pub fn set_control_offset(name: &str, ox: f32, oy: f32) {
    let mut lock = CUSTOM_OFFSETS.lock().unwrap();
    let map = lock.get_or_insert_with(HashMap::new);
    map.insert(name.to_string(), (ox, oy));
}

pub fn control_offset(name: &str) -> (f32, f32) {
    let lock = CUSTOM_OFFSETS.lock().unwrap();
    if let Some(map) = lock.as_ref() {
        if let Some(&(ox, oy)) = map.get(name) {
            return (ox, oy);
        }
    }
    (0.0, 0.0)
}

static HIDDEN_BUTTONS: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);

pub fn is_button_hidden(name: &str) -> bool {
    HIDDEN_BUTTONS
        .lock()
        .unwrap()
        .as_ref()
        .map(|set| set.contains(name))
        .unwrap_or(false)
}

pub fn set_button_hidden(name: &str, hidden: bool) {
    let mut lock = HIDDEN_BUTTONS.lock().unwrap();
    let set = lock.get_or_insert_with(std::collections::HashSet::new);
    if hidden {
        set.insert(name.to_string());
    } else {
        set.remove(name);
    }
}

pub fn load_layout_file(path: &Path) {
    if let Ok(text) = std::fs::read_to_string(path) {
        let mut map = HashMap::new();
        let mut hidden_set = std::collections::HashSet::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((name, val)) = line.split_once('=') {
                let name = name.trim();
                let val = val.trim();
                if val.eq_ignore_ascii_case("hidden") || val.eq_ignore_ascii_case("oculto") || val == "false" || val == "0" {
                    hidden_set.insert(name.to_string());
                } else if let Some((xs, ys)) = val.split_once(',') {
                    if let (Ok(x), Ok(y)) = (xs.trim().parse::<f32>(), ys.trim().parse::<f32>()) {
                        map.insert(name.to_string(), (x, y));
                    }
                }
            }
        }
        *CUSTOM_OFFSETS.lock().unwrap() = Some(map);
        *HIDDEN_BUTTONS.lock().unwrap() = Some(hidden_set);
    }
}

pub fn save_layout_file(path: &Path) {
    let lock = CUSTOM_OFFSETS.lock().unwrap();
    let hidden_lock = HIDDEN_BUTTONS.lock().unwrap();
    let mut text = String::from("# Layout personalizado de controles Pokémon Reloaded Android\n");
    if let Some(hidden) = hidden_lock.as_ref() {
        for name in hidden {
            text.push_str(&format!("{}=hidden\n", name));
        }
    }
    if let Some(map) = lock.as_ref() {
        for (name, (x, y)) in map {
            text.push_str(&format!("{}={:.4},{:.4}\n", name, x, y));
        }
    }
    let _ = std::fs::write(path, text);
}

static FAST_FORWARD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static FAST_FORWARD_SPEED: AtomicU32 = AtomicU32::new(15);

pub fn fast_forward_active() -> bool {
    FAST_FORWARD.load(Ordering::Relaxed)
}

pub fn toggle_fast_forward() -> bool {
    let active = !fast_forward_active();
    FAST_FORWARD.store(active, Ordering::Relaxed);
    active
}

pub fn fast_forward_multiplier() -> f32 {
    if fast_forward_active() {
        FAST_FORWARD_SPEED.load(Ordering::Relaxed) as f32 / 10.0
    } else {
        1.0
    }
}

pub fn set_fast_forward_speed(tenths: u32) {
    FAST_FORWARD_SPEED.store(tenths.clamp(10, 100), Ordering::Relaxed);
}

static AUDIO_MUTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn is_audio_muted() -> bool {
    AUDIO_MUTED.load(Ordering::Relaxed)
}

pub fn toggle_audio_mute() -> bool {
    let muted = !is_audio_muted();
    AUDIO_MUTED.store(muted, Ordering::Relaxed);
    muted
}

static BGM_VOLUME: AtomicU32 = AtomicU32::new(100);
static SFX_VOLUME: AtomicU32 = AtomicU32::new(100);

pub fn bgm_volume() -> f32 {
    if is_audio_muted() {
        return 0.0;
    }
    BGM_VOLUME.load(Ordering::Relaxed) as f32 / 100.0
}

pub fn sfx_volume() -> f32 {
    if is_audio_muted() {
        return 0.0;
    }
    SFX_VOLUME.load(Ordering::Relaxed) as f32 / 100.0
}

pub fn set_bgm_volume(pct: u32) {
    BGM_VOLUME.store(pct.clamp(0, 100), Ordering::Relaxed);
}

pub fn set_sfx_volume(pct: u32) {
    SFX_VOLUME.store(pct.clamp(0, 100), Ordering::Relaxed);
}

static GRAPHICS_BACKEND_VULKAN: AtomicBool = AtomicBool::new(false);
static FORCE_60FPS: AtomicBool = AtomicBool::new(false);

pub fn is_vulkan_backend() -> bool {
    GRAPHICS_BACKEND_VULKAN.load(Ordering::Relaxed)
}

pub fn set_vulkan_backend(enabled: bool) {
    GRAPHICS_BACKEND_VULKAN.store(enabled, Ordering::Relaxed);
}

pub fn force_60fps_active() -> bool {
    FORCE_60FPS.load(Ordering::Relaxed)
}

pub fn set_force_60fps(force: bool) {
    FORCE_60FPS.store(force, Ordering::Relaxed);
}

static AUTOFIRE_ENABLED: AtomicBool = AtomicBool::new(true);
static AUTOFIRE_HELD: Mutex<Option<std::collections::HashSet<usize>>> = Mutex::new(None);
static AUTOFIRE_TICK: AtomicU32 = AtomicU32::new(0);

pub fn is_autofire_enabled() -> bool {
    AUTOFIRE_ENABLED.load(Ordering::Relaxed)
}

pub fn set_autofire_enabled(enabled: bool) {
    AUTOFIRE_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn register_autofire(index: usize) {
    let mut lock = AUTOFIRE_HELD.lock().unwrap();
    let set = lock.get_or_insert_with(std::collections::HashSet::new);
    set.insert(index);
}

pub fn unregister_autofire(index: usize) {
    let mut lock = AUTOFIRE_HELD.lock().unwrap();
    if let Some(set) = lock.as_mut() {
        set.remove(&index);
    }
}

#[cfg(target_os = "android")]
pub fn tick_autofire() {
    if !is_autofire_enabled() {
        return;
    }
    let buttons = {
        let lock = AUTOFIRE_HELD.lock().unwrap();
        match lock.as_ref() {
            Some(set) if !set.is_empty() => set.iter().copied().collect::<Vec<_>>(),
            _ => return,
        }
    };

    let tick = AUTOFIRE_TICK.fetch_add(1, Ordering::Relaxed);
    // Cycle every 4 frames (2 frames pressed, 2 frames released) -> 15 pulses/sec at 60 FPS
    let phase = tick % 4;
    for index in buttons {
        if index < CONTROLS.len() {
            let key = CONTROLS[index].key;
            if phase == 0 {
                ramen::platform::android::push_input(ramen::platform::android::InputEvent::KeyDown { key, text: None });
            } else if phase == 2 {
                ramen::platform::android::push_input(ramen::platform::android::InputEvent::KeyUp { key });
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
pub fn tick_autofire() {}

static DYNAMIC_JOYSTICK_ANCHOR: Mutex<Option<(f32, f32)>> = Mutex::new(None);
static DYNAMIC_DPAD_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn is_dynamic_dpad_enabled() -> bool {
    DYNAMIC_DPAD_ENABLED.load(Ordering::Relaxed)
}

pub fn set_dynamic_dpad_enabled(enabled: bool) {
    DYNAMIC_DPAD_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn set_dynamic_anchor(pos: Option<(f32, f32)>) {
    *DYNAMIC_JOYSTICK_ANCHOR.lock().unwrap() = pos;
}

static EDIT_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LAYOUT_SNAPSHOT: Mutex<Option<(HashMap<String, (f32, f32)>, u32, u32)>> = Mutex::new(None);

pub fn snapshot_layout() {
    let offsets = CUSTOM_OFFSETS.lock().unwrap().clone().unwrap_or_default();
    let sc = SCALE.load(Ordering::Relaxed);
    let op = OPACITY.load(Ordering::Relaxed);
    *LAYOUT_SNAPSHOT.lock().unwrap() = Some((offsets, sc, op));
}

pub fn restore_layout_snapshot() {
    if let Some((offsets, sc, op)) = LAYOUT_SNAPSHOT.lock().unwrap().take() {
        *CUSTOM_OFFSETS.lock().unwrap() = Some(offsets);
        SCALE.store(sc, Ordering::Relaxed);
        OPACITY.store(op, Ordering::Relaxed);
    }
}

pub fn is_edit_mode() -> bool {
    EDIT_MODE.load(Ordering::Relaxed)
}

pub fn set_edit_mode(mode: bool) {
    if mode && !is_edit_mode() {
        snapshot_layout();
    }
    EDIT_MODE.store(mode, Ordering::Relaxed);
}

pub fn toggle_edit_mode() -> bool {
    let mode = !is_edit_mode();
    if mode {
        snapshot_layout();
    }
    EDIT_MODE.store(mode, Ordering::Relaxed);
    mode
}

static GUIDE_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn is_guide_open() -> bool {
    GUIDE_OPEN.load(Ordering::Relaxed)
}

pub fn toggle_guide_modal() -> bool {
    let mode = !is_guide_open();
    GUIDE_OPEN.store(mode, Ordering::Relaxed);
    if mode {
        set_log_open(false);
    }
    mode
}

pub fn set_guide_open(open: bool) {
    GUIDE_OPEN.store(open, Ordering::Relaxed);
}

static LOG_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn is_log_open() -> bool {
    LOG_OPEN.load(Ordering::Relaxed)
}

pub fn toggle_log_modal() -> bool {
    let mode = !is_log_open();
    LOG_OPEN.store(mode, Ordering::Relaxed);
    if mode {
        set_guide_open(false);
    }
    mode
}

pub fn set_log_open(open: bool) {
    LOG_OPEN.store(open, Ordering::Relaxed);
}

static RECENT_DIALOGUES: Mutex<Option<Vec<String>>> = Mutex::new(None);

pub fn record_dialogue(text: &str) {
    let clean = text.trim();
    if clean.len() < 6 || !clean.contains(' ') {
        return;
    }
    let mut lock = match RECENT_DIALOGUES.lock() {
        Ok(l) => l,
        Err(_) => return,
    };
    let list = lock.get_or_insert_with(Vec::new);
    if list.last().map(|s| s.as_str()) == Some(clean) {
        return;
    }
    if list.len() >= 12 {
        list.remove(0);
    }
    list.push(clean.to_string());
}

pub fn get_recent_dialogues() -> Vec<String> {
    RECENT_DIALOGUES.lock().ok().and_then(|l| l.clone()).unwrap_or_default()
}

static SHOW_NUM_KEYS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn set_show_num_keys(show: bool) {
    SHOW_NUM_KEYS.store(show, Ordering::Relaxed);
}

pub fn show_num_keys() -> bool {
    SHOW_NUM_KEYS.load(Ordering::Relaxed)
}

pub fn effective_shape(index: usize) -> Shape {
    let c = &CONTROLS[index];
    if c.contexts & DIALOG != 0 || c.name == RADIAL_MENU_NAME {
        return c.shape;
    }
    let s = scale();
    let mx = margin_x();
    let my = margin_y();

    let (mut cx, mut cy) = match c.shape {
        Shape::Circle { cx, cy, .. } => (cx, cy),
        Shape::Rect { cx, cy, .. } => (cx, cy),
    };

    let (ox, oy) = control_offset(c.name);
    cx += ox;
    cy += oy;

    if cx > 0.55 {
        cx -= mx;
    } else if cx < 0.45 {
        cx += mx;
    }

    if cy > 0.50 {
        cy -= my;
    }

    match c.shape {
        Shape::Circle { r, .. } => Shape::Circle { cx, cy, r: r * s },
        Shape::Rect { w, h, .. } => Shape::Rect { cx, cy, w: w * s, h: h * s },
    }
}

/// The rectangle the game image occupies in the window (normalised), as the renderer last presented
/// it. Dialogue controls are placed inside it, so the button lands on the dialogue box whatever the
/// screen's shape is and whichever scaling mode is in use.
static VIEWPORT: Mutex<(f32, f32, f32, f32)> = Mutex::new((0.0, 0.0, 1.0, 1.0));

pub fn set_viewport(x: f32, y: f32, w: f32, h: f32) {
    *VIEWPORT.lock().unwrap() = (x, y, w, h);
}

pub fn viewport() -> (f32, f32, f32, f32) {
    *VIEWPORT.lock().unwrap()
}

/// A control's shape in window coordinates. Most controls are placed relative to the window (they
/// live around its edges, where the thumbs are); the dialogue button is placed relative to the game
/// image, because it has to sit on the dialogue box the game draws.
pub fn window_shape(index: usize) -> Shape {
    let shape = effective_shape(index);
    if CONTROLS[index].contexts & DIALOG == 0 {
        return shape
    }
    let (vx, vy, vw, vh) = viewport();
    match shape {
        Shape::Circle { cx, cy, r } => Shape::Circle { cx: vx + cx * vw, cy: vy + cy * vh, r: r * vw },
        Shape::Rect { cx, cy, w, h } => Shape::Rect { cx: vx + cx * vw, cy: vy + cy * vh, w: w * vw, h: h * vh },
    }
}

/// The inverse: a point in window coordinates, expressed in a control's own coordinates.
pub fn control_point(index: usize, nx: f32, ny: f32) -> (f32, f32) {
    if CONTROLS[index].contexts & DIALOG == 0 {
        return (nx, ny)
    }
    let (vx, vy, vw, vh) = viewport();
    ((nx - vx) / vw.max(0.0001), (ny - vy) / vh.max(0.0001))
}

/// Looks a control up by its name.
pub fn index_of(name: &str) -> Option<usize> {
    CONTROLS.iter().position(|control| control.name == name)
}

/// The room the emulator reported, or -1 before the first frame.
static ROOM: AtomicI32 = AtomicI32::new(-1);

/// Called by the emulator every frame with the current room id.
pub fn set_room(room: i32) {
    let old = ROOM.load(Ordering::Relaxed);
    if old != room {
        // Room changed: immediately clear dialogue state and debounce
        DIALOGUE.store(false, Ordering::Relaxed);
        DIALOGUE_DEBOUNCE_FRAMES.store(0, Ordering::Relaxed);
    }
    ROOM.store(room, Ordering::Relaxed);
    log_state_change(room);
}

/// Whether the game has one of its dialogue boxes open. Reported every frame by the engine, which
/// reads it from the game's own modal state; the overlay then leaves a single button.
static DIALOGUE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DIALOGUE_DEBOUNCE_FRAMES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

const DIALOGUE_DEBOUNCE_HOLD: u32 = 8; // ~130ms at 60 FPS, prevents controls flicker between dialogue pages

pub fn set_dialogue(open: bool) {
    if open {
        DIALOGUE_DEBOUNCE_FRAMES.store(DIALOGUE_DEBOUNCE_HOLD, Ordering::Relaxed);
        let was = DIALOGUE.swap(true, Ordering::Relaxed);
        if !was {
            eprintln!("[overlay] dialogo abierto: solo el boton de avanzar");
        }
    } else {
        let remaining = DIALOGUE_DEBOUNCE_FRAMES.load(Ordering::Relaxed);
        if remaining > 0 {
            DIALOGUE_DEBOUNCE_FRAMES.store(remaining - 1, Ordering::Relaxed);
        } else {
            let was = DIALOGUE.swap(false, Ordering::Relaxed);
            if was {
                eprintln!("[overlay] dialogo cerrado");
            }
        }
    }
}

pub fn dialogue_open() -> bool {
    DIALOGUE.load(Ordering::Relaxed)
}

/// Whether the game's own Pokémon box (the PC storage) is open. Its screen is driven entirely with
/// the game's own click boxes, so nothing of the overlay is wanted there; the engine reports it from
/// the game's `ENCAJA` global.
static BOX_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_box_open(open: bool) {
    BOX_OPEN.store(open, Ordering::Relaxed);
}

pub fn box_open() -> bool {
    BOX_OPEN.load(Ordering::Relaxed)
}

/// Records every change of state, which is what decides which on-screen controls exist.
#[cfg(target_os = "android")]
fn log_state_change(room: i32) {
    static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
    static LAST_ROOM: AtomicI32 = AtomicI32::new(i32::MIN);
    let group = u32::from(current_context());
    let room_changed = LAST_ROOM.swap(room, Ordering::Relaxed) != room;
    if LAST.swap(group, Ordering::Relaxed) == group {
        if room_changed {
            eprintln!("[overlay] sala {} -> grupo {}", room, group);
        }
        return
    }
    let (window_w, window_h) = ramen::platform::android::window_size();
    let active = (0..CONTROLS.len()).filter(|index| is_active(*index)).count();
    eprintln!("[overlay] sala {} -> grupo {} | ventana {}x{} | {} de {} activos", room, group, window_w, window_h, active, CONTROLS.len());
    for (index, control) in CONTROLS.iter().enumerate() {
        let (at, size) = match control.shape {
            Shape::Circle { cx, cy, r } => (format!("{:.3},{:.3}", cx, cy), format!("r={:.3}", r)),
            Shape::Rect { cx, cy, w, h } => (format!("{:.3},{:.3}", cx, cy), format!("{:.3}x{:.3}", w, h)),
        };
        let name = match control.label {
            Label::None => "",
            Label::Arrow(_) => "flecha",
            Label::Text(text) => text,
        };
        eprintln!(
            "[overlay]   {:>2}. {:<6} {:<12} en ({}) {} -> {}",
            index,
            name,
            format!("{:?}", control.key),
            at,
            size,
            if is_active(index) { "visibles" } else { "ocultos" }
        );
    }
}

#[cfg(not(target_os = "android"))]
fn log_state_change(_room: i32) {}

/// Which group of controls the current game state needs.
pub fn current_context() -> u8 {
    // A dialogue box takes over the overlay: only the button that advances it is shown, so the text
    // and the portrait stay readable.
    if dialogue_open() {
        return DIALOG
    }
    // The Pokémon box is entirely point-and-click, so nothing is needed next to it: no controls at
    // all. MENU is the context no control belongs to, which is exactly that.
    if box_open() {
        return MENU
    }
    match ROOM.load(Ordering::Relaxed) {
        // IntroROOM, InicioROOM, CargarPartida, MultijugadorMenu, CreditosInicioScreen, OptionsROOM:
        // these screens are fully clickable (once the game's own click boxes are fixed, see
        // `crate::patches`), so they need no on-screen controls at all.
        22 | 30 | 43 | 461 | 465 | 466 => MENU,
        // BattleROOM, BattleROOMAqua, BattleROOMLava.
        47 | 191 | 709 => BATTLE,
        // Before the first frame we don't know the room yet; show nothing rather than flashing the
        // whole overlay for one frame.
        -1 => MENU,
        _ => WORLD,
    }
}

/// True while the player is on a pad. The on-screen controls then step aside, because somebody
/// holding a controller does not want buttons under their thumbs; the first touch on the screen
/// brings them back.
static PAD_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_pad_active(active: bool) {
    PAD_ACTIVE.store(active, Ordering::Relaxed);
}

pub fn pad_active() -> bool {
    PAD_ACTIVE.load(Ordering::Relaxed)
}

pub fn is_active(index: usize) -> bool {
    if pad_active() || box_open() {
        return false;
    }
    let c = &CONTROLS[index];
    if is_button_hidden(c.name) {
        return false;
    }
    if !show_num_keys() && (c.name.starts_with("alpha") || c.name == "space" || c.name == "o" || c.name == "m" || c.name == "e" || c.name == "t" || c.name == "l" || c.name == "p") {
        return false;
    }
    c.contexts & current_context() != 0
}

/// The joystick drives the arrow keys, which is how the game's menus (the ones that still show
/// controls) and the overworld are navigated.
pub fn joystick_active() -> bool {
    !pad_active() && !box_open() && joystick_visible() && current_context() & (WORLD | BATTLE) != 0
}

/// Returns the index of the control containing the point, if any. The point is normalised, but the
/// test is done in pixels so the touchable area matches what is drawn on a screen that is not square.
pub fn hit(nx: f32, ny: f32, ww: f32, wh: f32) -> Option<usize> {
    CONTROLS
        .iter()
        .enumerate()
        .position(|(index, _)| is_active(index) && contains(window_shape(index), nx, ny, ww, wh))
}

static PRESSED: AtomicU32 = AtomicU32::new(0);

pub fn set_pressed(index: usize, pressed: bool) {
    if pressed {
        PRESSED.fetch_or(1 << index, Ordering::Relaxed);
    } else {
        PRESSED.fetch_and(!(1 << index), Ordering::Relaxed);
    }
}

pub fn is_pressed(index: usize) -> bool {
    PRESSED.load(Ordering::Relaxed) & (1 << index) != 0
}

/// Controls that are pressed but no longer valid, so the caller can release their keys instead of
/// leaving one stuck down when the game changes state.
pub fn stale_pressed() -> Vec<usize> {
    (0..CONTROLS.len()).filter(|&index| is_pressed(index) && !is_active(index)).collect()
}

// ---------------------------------------------------------------------------------------------
// Movement joystick. It presses the arrow keys depending on which way the stick is pushed.

pub const JOYSTICK_CX: f32 = 0.135;
pub const JOYSTICK_CY: f32 = 0.700;
/// Radius as a fraction of the window *height*, so the base stays round on screen.
pub const JOYSTICK_R: f32 = 0.200;
static JOYSTICK_DEAD: AtomicU32 = AtomicU32::new(35);

pub fn set_joystick_dead(pct: u32) {
    JOYSTICK_DEAD.store(pct.clamp(10, 60), Ordering::Relaxed);
}

pub fn joystick_dead() -> f32 {
    JOYSTICK_DEAD.load(Ordering::Relaxed) as f32 / 100.0
}

/// The joystick's centre, in normalised coordinates.
pub fn joystick_center() -> (f32, f32) {
    if let Some((ax, ay)) = *DYNAMIC_JOYSTICK_ANCHOR.lock().unwrap() {
        return (ax, ay);
    }
    let (ox, oy) = control_offset(JOYSTICK_NAME);
    (JOYSTICK_CX + margin_x() + ox, JOYSTICK_CY - margin_y() + oy)
}

/// Radius as a fraction of the window height.
pub fn joystick_radius() -> f32 {
    JOYSTICK_R * scale()
}

pub fn joystick_visible() -> bool {
    true
}

pub fn joystick_hit(nx: f32, ny: f32, ww: f32, wh: f32) -> bool {
    if !joystick_active() {
        return false
    }
    let (cx, cy) = joystick_center();
    let r = joystick_radius() * wh;
    let dx = (nx - cx) * ww;
    let dy = (ny - cy) * wh;
    dx * dx + dy * dy <= r * r
}

/// `[up, down, left, right]` to hold for a stick at (nx, ny).
pub fn joystick_dirs(nx: f32, ny: f32, ww: f32, wh: f32) -> [bool; 4] {
    let (cx, cy) = joystick_center();
    let r = joystick_radius() * wh;
    let dx = (nx - cx) * ww;
    let dy = (ny - cy) * wh;
    stick_dirs(dx / r, dy / r)
}

/// Which directions a stick pushed by (dx, dy) is asking for: up, down, left, right. Both are in
/// the range -1..1, and this is the same dead zone the on-screen joystick uses - a physical pad's
/// stick goes through here too, so both feel identical.
pub fn stick_dirs(dx: f32, dy: f32) -> [bool; 4] {
    let dead = joystick_dead();
    [
        dy < -dead,
        dy > dead,
        dx < -dead,
        dx > dead,
    ]
}

/// The stick's offset from its centre, in units of its radius (for drawing the knob).
static JOYSTICK_OFFSET: Mutex<(f32, f32)> = Mutex::new((0.0, 0.0));

pub fn set_joystick_offset(dx: f32, dy: f32) {
    *JOYSTICK_OFFSET.lock().unwrap() = (dx, dy);
}

pub fn joystick_offset() -> (f32, f32) {
    *JOYSTICK_OFFSET.lock().unwrap()
}

static OPACITY: AtomicU32 = AtomicU32::new(70);

pub fn set_opacity(pct: u32) {
    OPACITY.store(pct.clamp(10, 100), Ordering::Relaxed);
}

pub fn opacity() -> f32 {
    OPACITY.load(Ordering::Relaxed) as f32 / 100.0
}

pub fn opacity_pct() -> u32 {
    OPACITY.load(Ordering::Relaxed)
}

static SCALE: AtomicU32 = AtomicU32::new(100);

pub fn set_scale(pct: u32) {
    SCALE.store(pct.clamp(60, 150), Ordering::Relaxed);
}

pub fn scale() -> f32 {
    SCALE.load(Ordering::Relaxed) as f32 / 100.0
}

pub fn scale_pct() -> u32 {
    SCALE.load(Ordering::Relaxed)
}

pub fn save_controls_config(package_dir: &str) {
    let dir = std::path::PathBuf::from(format!("/data/data/{}/files", package_dir));
    let controls_path = dir.join("controls.conf");
    let mut lines = Vec::new();
    if let Ok(existing) = std::fs::read_to_string(&controls_path) {
        for l in existing.lines() {
            let trim = l.trim();
            if !trim.starts_with("scale=") && !trim.starts_with("opacity=") {
                lines.push(trim.to_string());
            }
        }
    }
    lines.push(format!("scale={}", scale_pct()));
    lines.push(format!("opacity={}", opacity_pct()));
    let _ = std::fs::write(&controls_path, lines.join("\n"));
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EditToolbarAction {
    None,
    Save,
    Cancel,
    ScaleDec,
    ScaleInc,
    ScaleSet(u32),
    OpacityDec,
    OpacityInc,
    OpacitySet(u32),
}

pub struct EditToolbarLayout {
    pub bar_x1: f32,
    pub bar_y1: f32,
    pub bar_x2: f32,
    pub bar_y2: f32,
    pub save_x1: f32,
    pub save_x2: f32,
    pub cancel_x1: f32,
    pub cancel_x2: f32,
    pub scale_btn_minus: (f32, f32),
    pub scale_btn_plus: (f32, f32),
    pub scale_track: (f32, f32),
    pub opac_btn_minus: (f32, f32),
    pub opac_btn_plus: (f32, f32),
    pub opac_track: (f32, f32),
}

pub fn edit_toolbar_layout(ww: f32, wh: f32) -> EditToolbarLayout {
    let bar_x1 = (ww * 0.02).max(12.0);
    let bar_x2 = (ww * 0.98).min(ww - 12.0);
    // Dynamic height scaled with screen resolution: comfortable thumb target on 1080p+ screens
    let bar_h = (wh * 0.11).clamp(64.0, 92.0);
    let bar_y1 = 10.0;
    let bar_y2 = bar_y1 + bar_h;

    // Generous button widths (130px - 220px)
    let btn_w = (ww * 0.16).clamp(130.0, 220.0);
    let save_x1 = bar_x1 + 8.0;
    let save_x2 = save_x1 + btn_w;

    let cancel_x2 = bar_x2 - 8.0;
    let cancel_x1 = cancel_x2 - btn_w;

    let mid_start = save_x2 + 16.0;
    let mid_end = cancel_x1 - 16.0;
    let mid_width = (mid_end - mid_start).max(220.0);
    let half_mid = mid_width * 0.5;
    let slider_width = (half_mid - 70.0).clamp(70.0, 180.0);

    let scale_center = mid_start + half_mid * 0.45;
    let scale_track_x1 = scale_center - slider_width * 0.5;
    let scale_track_x2 = scale_center + slider_width * 0.5;
    let scale_minus = (scale_track_x1 - 28.0, scale_track_x1 - 4.0);
    let scale_plus = (scale_track_x2 + 4.0, scale_track_x2 + 28.0);

    let opac_center = mid_start + half_mid * 1.45;
    let opac_track_x1 = opac_center - slider_width * 0.5;
    let opac_track_x2 = opac_center + slider_width * 0.5;
    let opac_minus = (opac_track_x1 - 28.0, opac_track_x1 - 4.0);
    let opac_plus = (opac_track_x2 + 4.0, opac_track_x2 + 28.0);

    EditToolbarLayout {
        bar_x1, bar_y1, bar_x2, bar_y2,
        save_x1, save_x2,
        cancel_x1, cancel_x2,
        scale_btn_minus: scale_minus,
        scale_btn_plus: scale_plus,
        scale_track: (scale_track_x1, scale_track_x2),
        opac_btn_minus: opac_minus,
        opac_btn_plus: opac_plus,
        opac_track: (opac_track_x1, opac_track_x2),
    }
}

pub fn edit_toolbar_hit(tx: f32, ty: f32, ww: f32, wh: f32) -> EditToolbarAction {
    let l = edit_toolbar_layout(ww, wh);
    // Ultra-forgiving vertical detection: from top of screen (0.0) up to 35px below toolbar
    let max_y = l.bar_y2 + 35.0;
    if ty < 0.0 || ty > max_y {
        return EditToolbarAction::None;
    }

    // ACEPTAR (GUARDAR): Ultra-forgiving hit box on the left edge
    // Any touch anywhere from screen left edge (0.0) up to save_x2 + 30.0
    if tx >= 0.0 && tx <= l.save_x2 + 30.0 {
        return EditToolbarAction::Save;
    }

    // CANCELAR (SALIR): Ultra-forgiving hit box on the right edge
    // Any touch anywhere from cancel_x1 - 30.0 up to screen right edge (ww)
    if tx >= l.cancel_x1 - 30.0 && tx <= ww {
        return EditToolbarAction::Cancel;
    }

    // Middle area: Sliders and +/- increment buttons
    if ty >= l.bar_y1 - 6.0 && ty <= l.bar_y2 + 15.0 {
        if tx >= l.scale_btn_minus.0 - 10.0 && tx <= l.scale_btn_minus.1 + 4.0 {
            return EditToolbarAction::ScaleDec;
        }
        if tx >= l.scale_btn_plus.0 - 4.0 && tx <= l.scale_btn_plus.1 + 10.0 {
            return EditToolbarAction::ScaleInc;
        }
        if tx >= l.scale_track.0 && tx <= l.scale_track.1 {
            let ratio = ((tx - l.scale_track.0) / (l.scale_track.1 - l.scale_track.0)).clamp(0.0, 1.0);
            let pct = (60.0 + ratio * 90.0).round() as u32; // 60%..150%
            return EditToolbarAction::ScaleSet(pct);
        }

        if tx >= l.opac_btn_minus.0 - 10.0 && tx <= l.opac_btn_minus.1 + 4.0 {
            return EditToolbarAction::OpacityDec;
        }
        if tx >= l.opac_btn_plus.0 - 4.0 && tx <= l.opac_btn_plus.1 + 10.0 {
            return EditToolbarAction::OpacityInc;
        }
        if tx >= l.opac_track.0 && tx <= l.opac_track.1 {
            let ratio = ((tx - l.opac_track.0) / (l.opac_track.1 - l.opac_track.0)).clamp(0.0, 1.0);
            let pct = (20.0 + ratio * 80.0).round() as u32; // 20%..100%
            return EditToolbarAction::OpacitySet(pct);
        }
    }

    EditToolbarAction::None
}

static MARGIN_X: AtomicU32 = AtomicU32::new(0);

pub fn set_margin_x(pct: u32) {
    MARGIN_X.store(pct.clamp(0, 20), Ordering::Relaxed);
}

pub fn margin_x() -> f32 {
    MARGIN_X.load(Ordering::Relaxed) as f32 / 100.0
}

static MARGIN_Y: AtomicU32 = AtomicU32::new(0);

pub fn set_margin_y(pct: u32) {
    MARGIN_Y.store(pct.clamp(0, 20), Ordering::Relaxed);
}

pub fn margin_y() -> f32 {
    MARGIN_Y.load(Ordering::Relaxed) as f32 / 100.0
}

/// Point-in-shape, in pixels so it matches what is drawn.
pub fn contains(shape: Shape, nx: f32, ny: f32, ww: f32, wh: f32) -> bool {
    match shape {
        Shape::Circle { cx, cy, r } => {
            let (dx, dy) = ((nx - cx) * ww, (ny - cy) * wh);
            let radius = r * ww;
            dx * dx + dy * dy <= radius * radius
        },
        Shape::Rect { cx, cy, w, h } => (nx - cx).abs() * ww <= w * ww / 2.0 && (ny - cy).abs() * wh <= h * wh / 2.0,
    }
}

// ---------------------------------------------------------------------------------------------
// Party data. The game keeps it in `global.pkmn1[0..5]` (each slot's species); `game.rs` reads it
// and drops it here, so the overlay can draw each Pokémon's own sprite in its hexagon.

/// Name of the button that opens the party wheel. It presses no key of its own. It deliberately has
/// no `pkmn.png` of its own: the honeycomb artwork belongs to the wheel, not to the button.
pub const PARTY_NAME: &str = "pkmn";

pub const PICKER_SLOTS: usize = 6;

/// The key each slot presses: the game's 1..6.
pub const PICKER_KEYS: [Key; PICKER_SLOTS] =
    [Key::Alpha1, Key::Alpha2, Key::Alpha3, Key::Alpha4, Key::Alpha5, Key::Alpha6];

static PARTY_SPECIES: Mutex<Option<[i32; PICKER_SLOTS]>> = Mutex::new(None);
static PARTY_VERSION: AtomicU64 = AtomicU64::new(0);

pub fn set_party(species: [i32; PICKER_SLOTS]) {
    let changed = PARTY_SPECIES.lock().unwrap().as_ref() != Some(&species);
    *PARTY_SPECIES.lock().unwrap() = Some(species);
    if changed {
        // The renderer watches this to know when the GIFs under the slots changed.
        PARTY_VERSION.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn party_species() -> [i32; PICKER_SLOTS] {
    PARTY_SPECIES.lock().unwrap().unwrap_or([0; PICKER_SLOTS])
}

pub fn party_version() -> u64 {
    PARTY_VERSION.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------------------------
// Party wheel. Holding the selector button opens the team around the middle of the screen, like the
// weapon wheel in a certain game: slide towards the one you want and let go. Each step sends the key
// the game uses to pick, so its own screen highlights the same one as you move.

static WHEEL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static WHEEL_SEL: AtomicI32 = AtomicI32::new(-1);

pub fn wheel_open() -> bool {
    WHEEL.load(Ordering::Relaxed)
}

pub fn wheel_selected() -> i32 {
    WHEEL_SEL.load(Ordering::Relaxed)
}

pub fn set_wheel_open(open: bool) {
    WHEEL.store(open, Ordering::Relaxed);
    if !open {
        WHEEL_SEL.store(-1, Ordering::Relaxed);
    }
}

pub fn set_wheel_selected(index: i32) {
    WHEEL_SEL.store(index, Ordering::Relaxed);
}

/// The wheel's centre, in pixels: the middle of the screen, since the honeycomb needs the room.
pub fn wheel_center(ww: f32, wh: f32) -> (f32, f32) {
    (ww * 0.5, wh * 0.5)
}

/// How far out the slots sit, in pixels: the honeycomb is three of these wide.
pub fn wheel_radius(ww: f32, wh: f32) -> f32 {
    ww.min(wh) * 0.21
}

/// Where each of the honeycomb's six slots sits, taken from the shape of the artwork (79x74). A
/// honeycomb of seven hexagons is 2(1+√3) radii wide and 5 radii tall, so its hexagons have a
/// circumradius of 79/2/(1+√3) ≈ 14.5: each side slot sits √3 radii out and each diagonal
/// (√3/2, 1.5) radii out. Horizontal offsets are fractions of the width, vertical of the height,
/// which is how the renderer scales it.
const WHEEL_OFFSETS: [(f32, f32); PICKER_SLOTS] = [
    (-0.159, -0.293), // up-left
    (0.159, -0.293),  // up-right
    (-0.317, 0.0),    // left
    (0.317, 0.0),     // right
    (-0.159, 0.293),  // down-left
    (0.159, 0.293),   // down-right
];

/// How far a slot's centre sits from the selector's, as a fraction of the artwork's half-width. The
/// renderer sizes the artwork with this so its hexagons land exactly on the slots.
pub const SELECTOR_SLOT_DISTANCE: f32 = 0.634;

/// The artwork is very slightly wider than it is tall; the renderer keeps that when scaling it.
pub const SELECTOR_ASPECT: f32 = 74.0 / 79.0;

/// The centre of one of the wheel's slots, in pixels.
pub fn wheel_item_center(index: usize, ww: f32, wh: f32) -> (f32, f32) {
    let (cx, cy) = wheel_center(ww, wh);
    let half = wheel_radius(ww, wh) / SELECTOR_SLOT_DISTANCE;
    let (fx, fy) = WHEEL_OFFSETS[index];
    // The artwork is scaled by its width, so its vertical distances have to follow the same squeeze.
    (cx + fx * 2.0 * half, cy + fy * 2.0 * half * SELECTOR_ASPECT)
}

/// Which slot the finger is pointing at, computed via continuous polar angle (atan2)
/// so that dragging/swiping fluidly preselects all 6 slots in the party honeycomb.
pub fn wheel_item_at(nx: f32, ny: f32, ww: f32, wh: f32) -> Option<usize> {
    let (cx, cy) = wheel_center(ww, wh);
    let radius = wheel_radius(ww, wh);
    let px = nx * ww;
    let py = ny * wh;
    let dx = px - cx;
    let dy = (py - cy) / SELECTOR_ASPECT;
    let dist_sq = dx * dx + dy * dy;

    // Center deadzone around Pokéball: within 25% of radius, no slot is selected.
    let deadzone = radius * 0.25;
    if dist_sq < deadzone * deadzone {
        return None;
    }

    // Continuous angular detection via atan2(dy, dx)
    let mut deg = dy.atan2(dx).to_degrees();
    if deg < 0.0 {
        deg += 360.0;
    }

    // Hexagonal honeycomb slot mapping:
    // Slot 3 (Right):        [-30°, +30°]   -> [330°, 360°) U [0°, 30°)
    // Slot 5 (Down-Right):   [+30°, +90°)
    // Slot 4 (Down-Left):    [+90°, +150°)
    // Slot 2 (Left):         [+150°, +210°)
    // Slot 0 (Up-Left):      [+210°, +270°)
    // Slot 1 (Up-Right):     [+270°, +330°)
    if deg >= 330.0 || deg < 30.0 {
        Some(3)
    } else if deg >= 30.0 && deg < 90.0 {
        Some(5)
    } else if deg >= 90.0 && deg < 150.0 {
        Some(4)
    } else if deg >= 150.0 && deg < 210.0 {
        Some(2)
    } else if deg >= 210.0 && deg < 270.0 {
        Some(0)
    } else {
        Some(1)
    }
}

// ---------------------------------------------------------------------------------------------
// Drastic-style Radial Quick-Menu (Wheel Menu) & In-Game Utilities.

static RADIAL_MENU: AtomicBool = AtomicBool::new(false);
static RADIAL_HOVER_SECTOR: AtomicI32 = AtomicI32::new(-1);

pub fn radial_menu_open() -> bool {
    RADIAL_MENU.load(Ordering::Relaxed)
}

pub fn set_radial_menu_open(open: bool) {
    RADIAL_MENU.store(open, Ordering::Relaxed);
    if !open {
        RADIAL_HOVER_SECTOR.store(-1, Ordering::Relaxed);
    }
}

pub fn toggle_radial_menu() -> bool {
    let was = radial_menu_open();
    set_radial_menu_open(!was);
    !was
}

pub fn radial_hover_sector() -> i32 {
    RADIAL_HOVER_SECTOR.load(Ordering::Relaxed)
}

pub fn set_radial_hover_sector(sector: i32) {
    RADIAL_HOVER_SECTOR.store(sector, Ordering::Relaxed);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RadialAction {
    None,
    Turbo,     // Center disc
    Mute,      // Sector 0 (Top)
    Log,       // Sector 1 (Top-Right)
    Snapshot,  // Sector 2 (Right)
    Fps,       // Sector 3 (Bottom-Right)
    Close,     // Sector 4 (Bottom)
    Bezel,     // Sector 5 (Bottom-Left)
    Guide,     // Sector 6 (Left)
    Edit,      // Sector 7 (Top-Left)
    Outside,   // Touched outside the wheel -> dismiss
}

pub const RADIAL_SECTORS_COUNT: usize = 8;

pub fn radial_menu_center(ww: f32, wh: f32) -> (f32, f32) {
    (ww * 0.5, wh * 0.5)
}

pub fn radial_menu_inner_radius(ww: f32, wh: f32) -> f32 {
    ww.min(wh) * 0.14
}

pub fn radial_menu_outer_radius(ww: f32, wh: f32) -> f32 {
    ww.min(wh) * 0.40
}

pub fn radial_menu_hit(tx: f32, ty: f32, ww: f32, wh: f32) -> RadialAction {
    let (cx, cy) = radial_menu_center(ww, wh);
    let r_inner = radial_menu_inner_radius(ww, wh);
    let r_outer = radial_menu_outer_radius(ww, wh);
    let dx = tx - cx;
    let dy = ty - cy;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist <= r_inner {
        return RadialAction::Turbo;
    }
    if dist > r_outer * 1.08 {
        return RadialAction::Outside;
    }

    // Angle in radians (-PI..PI). Top is -PI/2.
    let angle = dy.atan2(dx);
    // Shift so Top (-PI/2) maps to 0
    let rel_angle = (angle + std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU) % std::f32::consts::TAU;
    let sector = (((rel_angle + std::f32::consts::FRAC_PI_8) / std::f32::consts::FRAC_PI_4) as usize) % RADIAL_SECTORS_COUNT;

    match sector {
        0 => RadialAction::Mute,
        1 => RadialAction::Log,
        2 => RadialAction::Snapshot,
        3 => RadialAction::Fps,
        4 => RadialAction::Close,
        5 => RadialAction::Bezel,
        6 => RadialAction::Guide,
        7 => RadialAction::Edit,
        _ => RadialAction::None,
    }
}

// Dynamic FPS overlay toggle
static SHOW_FPS: AtomicBool = AtomicBool::new(false);

pub fn show_fps() -> bool {
    SHOW_FPS.load(Ordering::Relaxed)
}

pub fn toggle_fps() -> bool {
    let next = !show_fps();
    SHOW_FPS.store(next, Ordering::Relaxed);
    next
}

pub fn set_show_fps(show: bool) {
    SHOW_FPS.store(show, Ordering::Relaxed);
}

// Dynamic Bezel Art cycle
static CURRENT_BEZEL: AtomicU32 = AtomicU32::new(0); // 0 = oled, 1 = retro_slate, 2 = reloaded_blue

pub fn current_bezel() -> &'static str {
    match CURRENT_BEZEL.load(Ordering::Relaxed) % 3 {
        0 => "oled",
        1 => "retro_slate",
        _ => "reloaded_blue",
    }
}

pub fn cycle_bezel() -> &'static str {
    let next = (CURRENT_BEZEL.load(Ordering::Relaxed) + 1) % 3;
    CURRENT_BEZEL.store(next, Ordering::Relaxed);
    current_bezel()
}

// In-Game HUD Toast Banners
static TOAST_MSG: Mutex<Option<(String, Instant)>> = Mutex::new(None);

pub fn show_toast(msg: &str) {
    if let Ok(mut lock) = TOAST_MSG.lock() {
        *lock = Some((msg.to_string(), Instant::now()));
    }
}

pub fn get_active_toast() -> Option<String> {
    if let Ok(lock) = TOAST_MSG.lock() {
        if let Some((msg, time)) = lock.as_ref() {
            if time.elapsed() < Duration::from_millis(2500) {
                return Some(msg.clone());
            }
        }
    }
    None
}
