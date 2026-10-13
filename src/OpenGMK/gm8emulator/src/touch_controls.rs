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
use std::sync::{
    atomic::{AtomicI32, AtomicU32, Ordering},
    Mutex,
};

/// Which game states a control is shown in.
pub const MENU: u8 = 1;
pub const WORLD: u8 = 2;
pub const BATTLE: u8 = 4;

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
    pub key: Key,
    pub shape: Shape,
    pub label: Label,
    /// Bitmask of [MENU] / [WORLD] / [BATTLE] states this control belongs to.
    pub contexts: u8,
}

pub const CONTROLS: &[Control] = &[
    // Party Pokémon 1-6 (battle: pick a Pokémon).
    Control { key: Key::Alpha1, shape: Shape::Circle { cx: 0.300, cy: 0.055, r: 0.020 }, label: Label::Text("1"), contexts: BATTLE },
    Control { key: Key::Alpha2, shape: Shape::Circle { cx: 0.372, cy: 0.055, r: 0.020 }, label: Label::Text("2"), contexts: BATTLE },
    Control { key: Key::Alpha3, shape: Shape::Circle { cx: 0.444, cy: 0.055, r: 0.020 }, label: Label::Text("3"), contexts: BATTLE },
    Control { key: Key::Alpha4, shape: Shape::Circle { cx: 0.516, cy: 0.055, r: 0.020 }, label: Label::Text("4"), contexts: BATTLE },
    Control { key: Key::Alpha5, shape: Shape::Circle { cx: 0.588, cy: 0.055, r: 0.020 }, label: Label::Text("5"), contexts: BATTLE },
    Control { key: Key::Alpha6, shape: Shape::Circle { cx: 0.660, cy: 0.055, r: 0.020 }, label: Label::Text("6"), contexts: BATTLE },
    // Menu / misc keys (second row along the top).
    Control { key: Key::Space, shape: Shape::Circle { cx: 0.300, cy: 0.145, r: 0.022 }, label: Label::Text("PKM"), contexts: WORLD | BATTLE },
    Control { key: Key::O, shape: Shape::Circle { cx: 0.372, cy: 0.145, r: 0.022 }, label: Label::Text("O"), contexts: WORLD | BATTLE },
    Control { key: Key::M, shape: Shape::Circle { cx: 0.444, cy: 0.145, r: 0.022 }, label: Label::Text("M"), contexts: WORLD | BATTLE },
    Control { key: Key::E, shape: Shape::Circle { cx: 0.516, cy: 0.145, r: 0.022 }, label: Label::Text("E"), contexts: WORLD | BATTLE },
    Control { key: Key::T, shape: Shape::Circle { cx: 0.588, cy: 0.145, r: 0.022 }, label: Label::Text("T"), contexts: WORLD | BATTLE },
    Control { key: Key::L, shape: Shape::Circle { cx: 0.660, cy: 0.145, r: 0.022 }, label: Label::Text("L"), contexts: WORLD | BATTLE },
    Control { key: Key::P, shape: Shape::Circle { cx: 0.732, cy: 0.145, r: 0.022 }, label: Label::Text("P"), contexts: WORLD | BATTLE },
    // Main actions.
    Control { key: Key::Z, shape: Shape::Circle { cx: 0.930, cy: 0.840, r: 0.034 }, label: Label::Text("Z"), contexts: WORLD | BATTLE },
    Control { key: Key::X, shape: Shape::Circle { cx: 0.800, cy: 0.868, r: 0.031 }, label: Label::Text("X"), contexts: WORLD | BATTLE },
    Control { key: Key::C, shape: Shape::Circle { cx: 0.858, cy: 0.635, r: 0.030 }, label: Label::Text("C"), contexts: WORLD | BATTLE },
    Control { key: Key::V, shape: Shape::Circle { cx: 0.950, cy: 0.665, r: 0.028 }, label: Label::Text("V"), contexts: WORLD | BATTLE },
    // Secondary.
    Control { key: Key::B, shape: Shape::Circle { cx: 0.712, cy: 0.840, r: 0.026 }, label: Label::Text("B"), contexts: BATTLE },
    Control { key: Key::S, shape: Shape::Circle { cx: 0.712, cy: 0.700, r: 0.026 }, label: Label::Text("S"), contexts: BATTLE },
    Control { key: Key::A, shape: Shape::Circle { cx: 0.640, cy: 0.900, r: 0.024 }, label: Label::Text("A"), contexts: WORLD },
    Control { key: Key::N, shape: Shape::Circle { cx: 0.640, cy: 0.760, r: 0.024 }, label: Label::Text("N"), contexts: WORLD },
    // Menu keys.
    Control { key: Key::Return, shape: Shape::Circle { cx: 0.500, cy: 0.945, r: 0.032 }, label: Label::Text("OK"), contexts: WORLD | BATTLE },
    Control { key: Key::Escape, shape: Shape::Circle { cx: 0.400, cy: 0.905, r: 0.026 }, label: Label::Text("ESC"), contexts: WORLD | BATTLE },
];

/// The room the emulator reported, or -1 before the first frame.
static ROOM: AtomicI32 = AtomicI32::new(-1);

/// Called by the emulator every frame with the current room id.
pub fn set_room(room: i32) {
    ROOM.store(room, Ordering::Relaxed);
    log_state_change(room);
}

/// Records every change of state, which is what decides which on-screen controls exist.
///
/// A report of buttons missing, or in the wrong place, cannot be chased without knowing what the
/// overlay believed at the time: which room, which group of controls it picked, and how many of
/// them it counted as active. It only writes when something changes, so it stays quiet while
/// playing.
#[cfg(target_os = "android")]
fn log_state_change(room: i32) {
    static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
    let group = u32::from(current_context());
    if LAST.swap(group, Ordering::Relaxed) == group {
        return
    }
    let (window_w, window_h) = ramen::platform::android::window_size();
    let active = (0..CONTROLS.len()).filter(|index| is_active(*index)).count();
    eprintln!("[overlay] sala {} -> grupo {} | ventana {}x{} | {} de {} activos", room, group, window_w, window_h, active, CONTROLS.len());
    // Every control, one per line, so a report of "buttons missing" can be checked against what the
    // overlay actually decided: which key, where it was put, how big it is, and whether it counted
    // as visible. Normalised coordinates are fractions of the window, so they can be compared
    // between devices of any size.
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

/// Nothing to report where there is no overlay.
#[cfg(not(target_os = "android"))]
fn log_state_change(_room: i32) {}

/// Which group of controls the current game state needs.
pub fn current_context() -> u8 {
    match ROOM.load(Ordering::Relaxed) {
        // IntroROOM, InicioROOM, MultijugadorMenu, CreditosInicioScreen, OptionsROOM: these screens
        // are fully clickable (once the game's own click boxes are fixed, see `crate::patches`), so
        // they need no on-screen controls at all.
        22 | 30 | 461 | 465 | 466 => MENU,
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
    !pad_active() && CONTROLS[index].contexts & current_context() != 0
}

/// The joystick drives the arrow keys, which is how the game's menus (the ones that still show
/// controls) and the overworld are navigated.
pub fn joystick_active() -> bool {
    !pad_active() && current_context() & (WORLD | BATTLE) != 0
}

/// Calculates the effective shape and position of a control accounting for scale and margins.
pub fn effective_shape(index: usize) -> Option<Shape> {
    if !is_active(index) {
        return None;
    }
    let c = &CONTROLS[index];
    let s = scale();
    let mx = margin_x();
    let my = margin_y();

    let (mut cx, mut cy) = match c.shape {
        Shape::Circle { cx, cy, .. } => (cx, cy),
        Shape::Rect { cx, cy, .. } => (cx, cy),
    };

    // Lateral margins: controls on the right move inward (left), controls on the left move inward (right).
    if cx > 0.55 {
        cx -= mx;
    } else if cx < 0.45 {
        cx += mx;
    }

    // Vertical margins: controls in the lower half move upward.
    if cy > 0.50 {
        cy -= my;
    }

    match c.shape {
        Shape::Circle { r, .. } => Some(Shape::Circle { cx, cy, r: r * s }),
        Shape::Rect { w, h, .. } => Some(Shape::Rect { cx, cy, w: w * s, h: h * s }),
    }
}

/// Returns the index of the control containing the normalised point, if any.
pub fn hit(nx: f32, ny: f32) -> Option<usize> {
    CONTROLS.iter().enumerate().position(|(index, _)| {
        let Some(shape) = effective_shape(index) else {
            return false;
        };
        match shape {
            Shape::Circle { cx, cy, r } => {
                let (dx, dy) = (nx - cx, ny - cy);
                dx * dx + dy * dy <= r * r
            },
            Shape::Rect { cx, cy, w, h } => (nx - cx).abs() <= w / 2.0 && (ny - cy).abs() <= h / 2.0,
        }
    })
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
/// Dead zone, as a fraction of the radius, before a direction registers.
pub const JOYSTICK_DEAD: f32 = 0.35;

pub fn joystick_center() -> (f32, f32) {
    (JOYSTICK_CX + margin_x(), JOYSTICK_CY - margin_y())
}

pub fn joystick_radius() -> f32 {
    JOYSTICK_R * scale()
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
    [
        dy < -JOYSTICK_DEAD,
        dy > JOYSTICK_DEAD,
        dx < -JOYSTICK_DEAD,
        dx > JOYSTICK_DEAD,
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

static SCALE: AtomicU32 = AtomicU32::new(100);

pub fn set_scale(pct: u32) {
    SCALE.store(pct.clamp(60, 150), Ordering::Relaxed);
}

pub fn scale() -> f32 {
    SCALE.load(Ordering::Relaxed) as f32 / 100.0
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

