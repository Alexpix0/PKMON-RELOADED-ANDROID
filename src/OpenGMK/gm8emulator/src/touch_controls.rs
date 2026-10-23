//! On-screen touch controls for the Android port.
//!
//! The game only understands keyboard input, so each on-screen control injects the key the game
//! expects. The key set is the one the game binds by default:
//!
//! - arrows: move
//! - `Z` confirm/act, `X` cancel, `C`/`V` extra actions (fishing/bike, attacks 3/4)
//! - `1`..`6`: pick a party PokÃ©mon in battle (and act as the HM shortcuts / berries)
//! - `B` pokÃ©ball, `S` switch, `M` mega, `E` run, `A` walk/assign, `N` use item
//! - `ENTER` menus, `ESC` quit
//!
//! Movement is an analog joystick (bottom-left); the rest are buttons. Positions are normalised to
//! the window so the input hit-test and the renderer share the exact same layout.
//!
//! Controls are grouped by the game's current state (see [current_context]): the title/menu rooms
//! only need the basics, the overworld needs the joystick and interaction keys, and battle needs
//! the combat extras. The emulator reports the current room every frame.

use ramen::input::Key;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

/// How long a state the engine reported keeps being believed after it stops being reported.
///
/// A long conversation is drawn as several boxes one after another, and between two of them the game
/// clears its modal for a frame or two. Reading that literally put the overlay back in the middle of
/// the conversation: the buttons blink in, and they are useless there anyway, because the player
/// cannot move while a dialogue is up.
const GRACE: Duration = Duration::from_millis(600);

/// A yes/no the engine reports every frame, that only drops once the game has stopped saying yes for
/// [GRACE].
struct Sticky {
    flag: AtomicBool,
    seen: Mutex<Option<Instant>>,
}

impl Sticky {
    const fn new() -> Self {
        Self { flag: AtomicBool::new(false), seen: Mutex::new(None) }
    }

    /// Notes this frame's reading. True when the answer actually changed.
    fn report(&self, on: bool) -> bool {
        if on {
            *self.seen.lock().unwrap() = Some(Instant::now());
            return !self.flag.swap(true, Ordering::Relaxed)
        }
        let recent = self.seen.lock().unwrap().map(|at| at.elapsed()).is_some_and(|ago| ago < GRACE);
        if recent {
            return false
        }
        self.flag.swap(false, Ordering::Relaxed)
    }

    fn clear(&self) {
        *self.seen.lock().unwrap() = None;
        self.flag.store(false, Ordering::Relaxed);
    }

    fn get(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}

/// Which game states a control is shown in.
pub const MENU: u8 = 1;
pub const WORLD: u8 = 2;
pub const BATTLE: u8 = 4;
/// A dialogue box is open: only the button that advances it is shown, so nothing covers the text.
pub const DIALOG: u8 = 8;
/// A new game asking you to pick your character. That screen is moved with the left and right keys,
/// so it gets a pair of arrows and nothing else.
pub const CHARACTER: u8 = 16;
/// A modal asking you to pick from a list of options - the preferences the new game asks about right
/// after the character, drawn over that same screen. The list is walked with the movement keys and
/// answered with Z and X, so it gets the joystick and those two buttons, and nothing else.
pub const CHOICE: u8 = 32;
/// The Safari Zone's catching room: everything the map shows, minus the movement stick and the
/// attack diamond - none of your own PokÃ©mon fights there - plus the ball, which is the key that
/// room answers to for throwing. The flee (E), the bag and the rest stay where the map has them.
pub const SAFARI: u8 = 64;
pub const MENU_NAV: u8 = 128;

/// The actual puzzle room: PuzleUNW, 202.
pub const PUZZLE_ROOM: i32 = 202;
pub const ROTATE_NAME: &str = "girar";

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

pub const MASTER_CONTROLS: &[Control] = &[
    Control { name: RADIAL_MENU_NAME, key: Key::ScrollLock, shape: Shape::Circle { cx: 0.500, cy: 0.982, r: 0.016 }, label: Label::Arrow(Dir::Down), contexts: WORLD | BATTLE | MENU },
    Control { name: "alpha1", key: Key::Alpha1, shape: Shape::Circle { cx: 0.300, cy: 0.055, r: 0.020 }, label: Label::Text("1"), contexts: SAFARI },
    Control { name: "alpha2", key: Key::Alpha2, shape: Shape::Circle { cx: 0.372, cy: 0.055, r: 0.020 }, label: Label::Text("2"), contexts: SAFARI },
    Control { name: "alpha3", key: Key::Alpha3, shape: Shape::Circle { cx: 0.444, cy: 0.055, r: 0.020 }, label: Label::Text("3"), contexts: SAFARI },
    Control { name: "alpha4", key: Key::Alpha4, shape: Shape::Circle { cx: 0.516, cy: 0.055, r: 0.020 }, label: Label::Text("4"), contexts: SAFARI },
    Control { name: "alpha5", key: Key::Alpha5, shape: Shape::Circle { cx: 0.588, cy: 0.055, r: 0.020 }, label: Label::Text("5"), contexts: SAFARI },
    Control { name: "alpha6", key: Key::Alpha6, shape: Shape::Circle { cx: 0.660, cy: 0.055, r: 0.020 }, label: Label::Text("6"), contexts: SAFARI },
    Control { name: "alpha7", key: Key::Alpha7, shape: Shape::Circle { cx: 0.732, cy: 0.055, r: 0.020 }, label: Label::Text("7"), contexts: WORLD },
    Control { name: crate::field_controls::NAME, key: Key::Alpha1, shape: Shape::Circle { cx: 0.300, cy: 0.760, r: 0.038 }, label: Label::Text("SEL"), contexts: WORLD },
    Control { name: "space", key: Key::Space, shape: Shape::Circle { cx: 0.300, cy: 0.145, r: 0.022 }, label: Label::Text("PKM"), contexts: WORLD | BATTLE | SAFARI },
    Control { name: "o", key: Key::O, shape: Shape::Circle { cx: 0.372, cy: 0.145, r: 0.022 }, label: Label::Text("O"), contexts: WORLD | BATTLE | SAFARI },
    Control { name: "m", key: Key::M, shape: Shape::Circle { cx: 0.444, cy: 0.145, r: 0.022 }, label: Label::Text("M"), contexts: WORLD | BATTLE | SAFARI },
    Control { name: "e", key: Key::E, shape: Shape::Circle { cx: 0.516, cy: 0.145, r: 0.022 }, label: Label::Text("E"), contexts: WORLD | BATTLE | SAFARI },
    Control { name: "t", key: Key::T, shape: Shape::Circle { cx: 0.588, cy: 0.145, r: 0.022 }, label: Label::Text("T"), contexts: WORLD | SAFARI },
    Control { name: "l", key: Key::L, shape: Shape::Circle { cx: 0.660, cy: 0.145, r: 0.022 }, label: Label::Text("L"), contexts: WORLD | SAFARI },
    Control { name: "p", key: Key::P, shape: Shape::Circle { cx: 0.732, cy: 0.145, r: 0.022 }, label: Label::Text("P"), contexts: WORLD | SAFARI },
    Control { name: "z", key: Key::Z, shape: Shape::Circle { cx: 0.865, cy: 0.851, r: 0.033 }, label: Label::Text("Z"), contexts: WORLD | BATTLE | CHOICE | MENU_NAV | MENU },
    Control { name: "x", key: Key::X, shape: Shape::Circle { cx: 0.933, cy: 0.700, r: 0.033 }, label: Label::Text("X"), contexts: WORLD | BATTLE | CHOICE | MENU_NAV | MENU },
    Control { name: "c", key: Key::C, shape: Shape::Circle { cx: 0.797, cy: 0.700, r: 0.033 }, label: Label::Text("C"), contexts: WORLD | BATTLE | MENU },
    Control { name: "v", key: Key::V, shape: Shape::Circle { cx: 0.865, cy: 0.549, r: 0.033 }, label: Label::Text("V"), contexts: WORLD | BATTLE },
    Control { name: "b", key: Key::B, shape: Shape::Circle { cx: 0.712, cy: 0.840, r: 0.026 }, label: Label::Text("B"), contexts: BATTLE | SAFARI },
    Control { name: "s", key: Key::S, shape: Shape::Circle { cx: 0.712, cy: 0.700, r: 0.026 }, label: Label::Text("S"), contexts: BATTLE },
    Control { name: PARTY_NAME, key: Key::Alpha1, shape: Shape::Circle { cx: 0.300, cy: 0.760, r: 0.038 }, label: Label::Text("SEL"), contexts: BATTLE },
    Control { name: "a", key: Key::A, shape: Shape::Circle { cx: 0.640, cy: 0.900, r: 0.024 }, label: Label::Text("A"), contexts: WORLD | BATTLE | SAFARI | MENU_NAV | MENU },
    Control { name: "n", key: Key::N, shape: Shape::Circle { cx: 0.640, cy: 0.760, r: 0.024 }, label: Label::Text("N"), contexts: WORLD | SAFARI },
    Control { name: "return", key: Key::Return, shape: Shape::Circle { cx: 0.500, cy: 0.945, r: 0.032 }, label: Label::Text("OK"), contexts: WORLD | SAFARI | MENU_NAV | MENU },
    Control { name: "escape", key: Key::Escape, shape: Shape::Circle { cx: 0.400, cy: 0.905, r: 0.026 }, label: Label::Text("ESC"), contexts: WORLD | SAFARI | MENU_NAV | MENU },
    Control { name: "dialogo", key: Key::Z, shape: Shape::Circle { cx: 0.940, cy: 0.880, r: 0.024 }, label: Label::Text("Z"), contexts: DIALOG },
    Control { name: LEFT_ARROW_NAME, key: Key::LeftArrow, shape: Shape::Circle { cx: LEFT_ARROW_CX, cy: ARROW_CY, r: ARROW_R }, label: Label::Arrow(Dir::Left), contexts: CHARACTER },
    Control { name: RIGHT_ARROW_NAME, key: Key::RightArrow, shape: Shape::Circle { cx: RIGHT_ARROW_CX, cy: ARROW_CY, r: ARROW_R }, label: Label::Arrow(Dir::Right), contexts: CHARACTER },
    Control { name: "eleccion_ok", key: Key::Return, shape: Shape::Circle { cx: 0.932, cy: 0.600, r: 0.036 }, label: Label::Text("OK"), contexts: CHARACTER },
    Control { name: "eleccion_atras", key: Key::X, shape: Shape::Circle { cx: 0.972, cy: 0.420, r: 0.028 }, label: Label::Text("X"), contexts: CHARACTER },
    Control { name: COMBO_BUTTON_NAME, key: Key::LeftControl, shape: Shape::Circle { cx: 0.500, cy: 0.060, r: 0.018 }, label: Label::Text("C+P"), contexts: CHARACTER },
    Control { name: "lmb", key: Key::LeftControl, shape: Shape::Circle { cx: 0.850, cy: 0.350, r: 0.028 }, label: Label::Text("LMB"), contexts: 0 },
    Control { name: "rmb", key: Key::LeftAlt, shape: Shape::Circle { cx: 0.920, cy: 0.350, r: 0.028 }, label: Label::Text("RMB"), contexts: 0 },
    Control { name: "qty_plus", key: Key::UpArrow, shape: Shape::Circle { cx: 0.850, cy: 0.260, r: 0.022 }, label: Label::Text("+"), contexts: 0 },
    Control { name: "qty_minus", key: Key::DownArrow, shape: Shape::Circle { cx: 0.920, cy: 0.260, r: 0.022 }, label: Label::Text("-"), contexts: 0 },
    Control { name: "rotate_l", key: Key::Q, shape: Shape::Circle { cx: 0.730, cy: 0.400, r: 0.028 }, label: Label::Text("⟲"), contexts: WORLD | MENU_NAV },
    Control { name: "rotate_r", key: Key::E, shape: Shape::Circle { cx: 0.810, cy: 0.400, r: 0.028 }, label: Label::Text("⟳"), contexts: WORLD | MENU_NAV },
    Control { name: "h", key: Key::H, shape: Shape::Circle { cx: 0.050, cy: 0.145, r: 0.022 }, label: Label::Text("H"), contexts: WORLD },
    Control { name: "f", key: Key::F, shape: Shape::Circle { cx: 0.120, cy: 0.145, r: 0.022 }, label: Label::Text("F"), contexts: WORLD },
    Control { name: "f1", key: Key::F1, shape: Shape::Circle { cx: 0.190, cy: 0.145, r: 0.022 }, label: Label::Text("F1"), contexts: WORLD },
    Control { name: "f5", key: Key::F5, shape: Shape::Circle { cx: 0.260, cy: 0.145, r: 0.022 }, label: Label::Text("F5"), contexts: WORLD },
    Control { name: ROTATE_NAME, key: Key::Z, shape: Shape::Circle { cx: 0.935, cy: 0.150, r: 0.036 }, label: Label::Text("Girar"), contexts: 0 },
];

pub const CLASSIC_CONTROLS: &[Control] = MASTER_CONTROLS;
pub const NEW_CONTROLS: &[Control] = MASTER_CONTROLS;
pub const CONTROLS: &[Control] = MASTER_CONTROLS;

#[inline(always)]
pub fn controls() -> &'static [Control] {
    MASTER_CONTROLS
}

#[inline(always)]
pub fn control_at(index: usize) -> &'static Control {
    &MASTER_CONTROLS[index]
}


/// TEMP: the button that presses the game's trick combination. The game wants both keys held at the
/// same time, so it is not an ordinary one-key control.
pub const COMBO_BUTTON_NAME: &str = "truco_ctrl_p";

/// Whether the picker's one-off trick button has already been used in this new game. It opens the
/// whole character roster, which only needs doing once, so it disappears after the first press and
/// is offered again only when a new game brings the picker back.
static COMBO_USED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_combo_used() {
    COMBO_USED.store(true, Ordering::Relaxed);
}

/// The room the character picker reports. Its own dialogues and its list of questions both live here.
const CHARACTER_ROOM: i32 = 3;

/// The modals the game draws, right after the character, as lists of options to pick from: the
/// preferences it asks about first and the difficulty right after. Both are walked with the movement
/// keys and answered with Z and X. Confirmed on the device: the picker's room 3 reports these ids
/// while each list is up, and nothing that is not one of them.
const PREFERENCES_MODAL: i32 = 7744;
const DIFFICULTY_MODAL: i32 = 7836;
const QUESTION_MODAL: i32 = 9673;
pub const LIST_MODAL: i32 = 7778;

/// The most characters the picker offers at once: one while choosing the player, two while choosing
/// the rival. Anything beyond that is the whole roster drawn at once by the game's own trick.
const PICKER_MAX_CHARACTERS: usize = 3;

/// The character picker's two arrows, and where they sit: one either side of the character.
const LEFT_ARROW_NAME: &str = "flechaizquierda";
const RIGHT_ARROW_NAME: &str = "flechaderecha";
const LEFT_ARROW_CX: f32 = 0.095;
const RIGHT_ARROW_CX: f32 = 0.430;
const ARROW_CY: f32 = 0.340;
const ARROW_R: f32 = 0.030;

/// How far from the character's edge the picker's arrows sit, as a fraction of the window width.
const ARROW_MARGIN: f32 = 0.045;

thread_local! {
    /// One box per character-sized sprite drawn this frame, in the game's own coordinates. The picker's
    /// arrows are placed from this, so they follow the character instead of sitting at a spot guessed
    /// in advance. Each candidate is kept apart: the arrows belong to the one that is selected, not
    /// around all of them.
    static CHARACTER_SPRITES: std::cell::RefCell<Vec<(f64, f64, f64, f64)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Notes one drawn sprite that the renderer judged to be character-sized.
pub fn note_sprite(x1: f64, y1: f64, x2: f64, y2: f64) {
    CHARACTER_SPRITES.with(|cell| {
        let mut seen = cell.borrow_mut();
        // The same sprite is drawn more than once in a frame, and one character can be drawn again in
        // another way (a shadow, an outline). Anything overlapping a box already kept is the same
        // character, and counting it twice would make the arrows step twice over one of them.
        let overlaps = seen.iter().any(|other: &(f64, f64, f64, f64)| {
            x1 < other.2 && other.0 < x2 && y1 < other.3 && other.1 < y2
        });
        if !overlaps {
            seen.push((x1, y1, x2, y2));
        }
    });
}

/// Starts a frame with nothing noted.
pub fn reset_character_sprites() {
    CHARACTER_SPRITES.with(|cell| cell.borrow_mut().clear());
}

/// The candidate boxes of the frame, in game coordinates, left to right.
pub fn character_sprites() -> Vec<(f64, f64, f64, f64)> {
    CHARACTER_SPRITES.with(|cell| {
        let mut boxes = cell.borrow().clone();
        boxes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        boxes
    })
}

/// The candidates' boxes as fractions of the window, left to right, as the engine last worked them
/// out. On the picker's screen there are two of them (the rival's) or one (the player's).
static PICKER_BOXES: Mutex<Vec<(f32, f32, f32, f32)>> = Mutex::new(Vec::new());

pub fn set_picker_boxes(boxes: Vec<(f32, f32, f32, f32)>) {
    // A frame that draws no character at all is almost always a blink of the game's own redrawing
    // rather than the end of the picker. Keeping the last ones means the arrows stay where they are
    // instead of falling back to their guessed spot and out again on the next frame.
    if boxes.is_empty() && !picker_boxes().is_empty() {
        return
    }
    *PICKER_BOXES.lock().unwrap() = boxes;
}

fn picker_boxes() -> Vec<(f32, f32, f32, f32)> {
    PICKER_BOXES.lock().unwrap().clone()
}

/// Whether the game has any of its modals up - the text dialog, a question it asks, a summary. The
/// engine reports it from the game's own modal state, and the picker's controls step aside for all of
/// them: those screens are gone through with the game's own keyboard and buttons.
static MODAL_OPEN: Sticky = Sticky::new();

pub fn set_modal_open(open: bool) {
    MODAL_OPEN.report(open);
}

/// The id of the modal the game is drawing (-1 when there is none), kept only for the log.
///
/// The game gives every one of its screens a modal of its own, and that id is what tells them apart:
/// the text dialog it types a name in is one, the boxes it asks questions with are others, the lists
/// it makes you choose from are others again. Reading that id is the only exact way of saying which
/// screens should have the joystick and the two buttons and which should have nothing at all.
static MODAL_ID: AtomicI32 = AtomicI32::new(-1);

pub fn set_modal_id(id: i32) {
    if MODAL_ID.swap(id, Ordering::Relaxed) != id {
        eprintln!("[overlay] modal {id}");
    }
}

pub fn modal_open() -> bool {
    MODAL_OPEN.get()
}

/// The modal the game is drawing right now, as it reported it this frame (-1 when none).
pub fn modal_id() -> i32 {
    MODAL_ID.load(Ordering::Relaxed)
}

/// Which of the candidates the arrows are around. The buttons step it, so they move from one to the
/// other the way the game's own selection does.
static PICKER_TARGET: AtomicUsize = AtomicUsize::new(0);

pub fn picker_step(delta: isize) {
    let next = PICKER_TARGET.load(Ordering::Relaxed) as isize + delta;
    PICKER_TARGET.store(next.max(0) as usize, Ordering::Relaxed);
}

pub fn picker_reset() {
    PICKER_TARGET.store(0, Ordering::Relaxed);
}

/// The control's shape, with whatever the player moved or resized it to (see [set_placement]) and the
/// overall size of the new overlay on top. The renderer and the hit test share it, so what is drawn is
/// what is touched.
pub fn effective_shape(index: usize) -> Shape {
    let classic = overlay_style() == OverlayStyle::Classic;
    let control = &controls()[index];
    // The classic overlay has its own per-context layout (the positions/tamaÃ±os editor).
    if classic {
        if let Some(entry) = layout_override(control.name) {
            let scale = entry.scale * ui_scale();
            return match control.shape {
                Shape::Circle { r, .. } => Shape::Circle { cx: entry.x, cy: entry.y, r: r * scale },
                Shape::Rect { w, h, .. } => Shape::Rect { cx: entry.x, cy: entry.y, w: w * scale, h: h * scale },
            }
        }
    }
    let mut shape = control.shape;
    // The picker's arrows sit beside the character, placed from where the game drew it: that way they
    // follow the sprite, on the player's screen and on the rival's, where the two characters are a
    // good way apart. Once the player has placed an arrow by hand, their place wins.
    if (control.name == LEFT_ARROW_NAME || control.name == RIGHT_ARROW_NAME)
        && (classic || placement(control.name).is_none())
    {
        let boxes = picker_boxes();
        let target = PICKER_TARGET.load(Ordering::Relaxed).min(boxes.len().saturating_sub(1));
        if let Some(&(x1, y1, x2, y2)) = boxes.get(target) {
            let cx = if control.name == LEFT_ARROW_NAME { x1 - ARROW_MARGIN } else { x2 + ARROW_MARGIN };
            // Kept inside the window, so a character drawn at the edge cannot push its arrow off it.
            let inside = |v: f32| v.clamp(ARROW_R, 1.0 - ARROW_R);
            shape = Shape::Circle { cx: inside(cx), cy: inside((y1 + y2) / 2.0), r: ARROW_R };
        }
    }
    // Our own overlay's saved placement applies only to the new style; the overall size applies to
    // both, so the size slider works on the classic overlay too.
    if !classic {
        if let Some((cx, cy, scale)) = placement(control.name) {
            shape = match shape {
                Shape::Circle { r, .. } => Shape::Circle { cx, cy, r: r * scale },
                Shape::Rect { w, h, .. } => Shape::Rect { cx, cy, w: w * scale, h: h * scale },
            };
        }
    }
    scale_shape(shape, ui_scale())
}

pub fn parse_key(name: &str) -> Option<Key> {
    let upper = name.trim().to_ascii_uppercase();
    if upper.len() == 1 {
        let b = upper.as_bytes()[0];
        if b >= b'A' && b <= b'Z' {
            return match b {
                b'A' => Some(Key::A), b'B' => Some(Key::B), b'C' => Some(Key::C), b'D' => Some(Key::D),
                b'E' => Some(Key::E), b'F' => Some(Key::F), b'G' => Some(Key::G), b'H' => Some(Key::H),
                b'I' => Some(Key::I), b'J' => Some(Key::J), b'K' => Some(Key::K), b'L' => Some(Key::L),
                b'M' => Some(Key::M), b'N' => Some(Key::N), b'O' => Some(Key::O), b'P' => Some(Key::P),
                b'Q' => Some(Key::Q), b'R' => Some(Key::R), b'S' => Some(Key::S), b'T' => Some(Key::T),
                b'U' => Some(Key::U), b'V' => Some(Key::V), b'W' => Some(Key::W), b'X' => Some(Key::X),
                b'Y' => Some(Key::Y), b'Z' => Some(Key::Z), _ => None,
            };
        }
        if b >= b'0' && b <= b'9' {
            return match b {
                b'0' => Some(Key::Alpha0), b'1' => Some(Key::Alpha1), b'2' => Some(Key::Alpha2),
                b'3' => Some(Key::Alpha3), b'4' => Some(Key::Alpha4), b'5' => Some(Key::Alpha5),
                b'6' => Some(Key::Alpha6), b'7' => Some(Key::Alpha7), b'8' => Some(Key::Alpha8),
                b'9' => Some(Key::Alpha9), _ => None,
            };
        }
    }
    match upper.as_str() {
        "RETURN" | "ENTER" | "OK" => Some(Key::Return),
        "SPACE" | "ESPACIO" | "PKM" => Some(Key::Space),
        "ESCAPE" | "ESC" => Some(Key::Escape),
        "TAB" | "TABULADOR" => Some(Key::Tab),
        "BACKSPACE" | "RETROCESO" => Some(Key::Backspace),
        "DEL" | "DELETE" | "SUPR" => Some(Key::Delete),
        "INS" | "INSERT" => Some(Key::Insert),
        "HOME" | "INICIO" => Some(Key::Home),
        "END" | "FIN" => Some(Key::End),
        "PGUP" | "PAGEUP" | "RE_PAG" => Some(Key::PageUp),
        "PGDN" | "PAGEDOWN" | "AV_PAG" => Some(Key::PageDown),
        "UP" | "ARRIBA" => Some(Key::UpArrow),
        "DOWN" | "ABAJO" => Some(Key::DownArrow),
        "LEFT" | "IZQUIERDA" => Some(Key::LeftArrow),
        "RIGHT" | "DERECHA" => Some(Key::RightArrow),
        "SHIFT" | "LSHIFT" | "LEFTSHIFT" => Some(Key::LeftShift),
        "RSHIFT" | "RIGHTSHIFT" => Some(Key::RightShift),
        "CTRL" | "LCTRL" | "LEFTCONTROL" => Some(Key::LeftControl),
        "RCTRL" | "RIGHTCONTROL" => Some(Key::RightControl),
        "ALT" | "LALT" | "LEFTALT" => Some(Key::LeftAlt),
        "RALT" | "RIGHTALT" => Some(Key::RightAlt),
        "F1" => Some(Key::F1), "F2" => Some(Key::F2), "F3" => Some(Key::F3),
        "F4" => Some(Key::F4), "F5" => Some(Key::F5), "F6" => Some(Key::F6),
        "F7" => Some(Key::F7), "F8" => Some(Key::F8), "F9" => Some(Key::F9),
        "F10" => Some(Key::F10), "F11" => Some(Key::F11), "F12" => Some(Key::F12),
        // Gamepad keycodes and tags
        "MANDO A" | "BOTON A" | "96" => Some(Key::Z),
        "MANDO B" | "BOTON B" | "97" => Some(Key::X),
        "MANDO X" | "BOTON X" | "99" => Some(Key::C),
        "MANDO Y" | "BOTON Y" | "100" => Some(Key::V),
        "MANDO L1" | "L1" | "102" => Some(Key::M),
        "MANDO R1" | "R1" | "103" => Some(Key::B),
        "MANDO L2" | "L2" | "104" => Some(Key::S),
        "MANDO R2" | "R2" | "105" => Some(Key::O),
        "MANDO START" | "START" | "108" => Some(Key::Space),
        "MANDO SELECT" | "SELECT" | "109" => Some(Key::Return),
        // Global mouse & quantity controls
        "LMB" | "CLIC IZQUIERDO" | "CLICK IZQUIERDO" => Some(Key::LeftControl),
        "RMB" | "CLIC DERECHO" | "CLICK DERECHO" => Some(Key::LeftAlt),
        "+" | "MAS" | "QTY_PLUS" => Some(Key::UpArrow),
        "-" | "MENOS" | "QTY_MINUS" => Some(Key::DownArrow),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LayoutOverride {
    pub context: u8,
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub visible: bool,
    pub custom_key: Option<Key>,
    pub custom_label: Option<&'static str>,
}

static CLASSIC_LAYOUT: Mutex<Vec<LayoutOverride>> = Mutex::new(Vec::new());

/// The launcher saves each real context independently. Unknown controls and contexts cannot
/// introduce extra buttons. Invalid/missing files leave the original layout intact.
pub fn load_layout(text: &str) {
    let mut entries: Vec<LayoutOverride> = Vec::new();
    if text.len() <= 65536 && text.lines().next() == Some("touch-layout-v1") {
        for line in text.lines().skip(1) {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() < 6 { continue }
            let context = match fields[0] {
                "world" => WORLD, "battle" => BATTLE, "dialog" => DIALOG, "character" => CHARACTER, "choice" => CHOICE, "safari" => SAFARI, "menu_nav" => MENU_NAV, "menu" => MENU, _ => continue,
            };
            let name = if fields[1] == JOYSTICK_NAME && context & (WORLD | BATTLE | CHOICE | MENU_NAV | MENU) != 0 {
                JOYSTICK_NAME
            } else if let Some(c) = CLASSIC_CONTROLS.iter().find(|c| c.name == fields[1]) {
                c.name
            } else { continue };
            let (Ok(x), Ok(y), Ok(scale)) = (fields[2].parse::<f32>(), fields[3].parse::<f32>(), fields[4].parse::<f32>()) else { continue };
            if !x.is_finite() || !y.is_finite() || !scale.is_finite() { continue }
            let visible = match fields[5] { "1" => true, "0" => false, _ => continue };
            let (custom_key, custom_label) = if fields.len() >= 7 && fields[6] != "-" {
                let k = parse_key(fields[6]);
                let l = Box::leak(fields[6].to_string().into_boxed_str()) as &'static str;
                (k, Some(l))
            } else {
                (None, None)
            };
            entries.retain(|e| e.context != context || e.name != name);
            entries.push(LayoutOverride { context, name, x: x.clamp(0.0, 1.0), y: y.clamp(0.0, 1.0), scale: scale.clamp(0.5, 2.0), visible, custom_key, custom_label });
        }
    }
    *CLASSIC_LAYOUT.lock().unwrap() = entries;
}

pub fn control_key(index: usize) -> Key {
    let name = controls()[index].name;
    layout_override(name)
        .and_then(|e| e.custom_key)
        .unwrap_or_else(|| controls()[index].key)
}

pub fn control_custom_label(name: &str) -> Option<&'static str> {
    layout_override(name).and_then(|e| e.custom_label)
}

fn layout_override(name: &str) -> Option<LayoutOverride> {
    let context = current_context();
    CLASSIC_LAYOUT.lock().unwrap().iter().find(|entry| entry.context == context && entry.name == name).copied()
        .or_else(|| profile_default(name, context))
}

fn profile_default(name: &str, context: u8) -> Option<LayoutOverride> {
    let (name, x, y) = match (context, name) {
        (WORLD, "space") => ("space", 0.300, 0.050),
        (WORLD, "o") => ("o", 0.372, 0.050),
        (WORLD, "m") => ("m", 0.444, 0.050),
        (WORLD, "e") => ("e", 0.516, 0.050),
        (WORLD, "t") => ("t", 0.588, 0.050),
        (WORLD, "l") => ("l", 0.660, 0.050),
        (WORLD, "p") => ("p", 0.732, 0.050),
        (WORLD, "n") => ("n", 0.645, 0.720),
        (WORLD, "a") => ("a", 0.645, 0.890),
        (BATTLE, "space") => ("space", 0.042, 0.180),
        (BATTLE, "o") => ("o", 0.096, 0.180),
        (BATTLE, "e") => ("e", 0.148, 0.180),
        // Level with the three on the left, clear of the party's status bars.
        (BATTLE, "t") => ("t", 0.440, 0.180),
        (BATTLE, "l") => ("l", 0.520, 0.180),
        (BATTLE, "p") => ("p", 0.600, 0.180),
        (BATTLE, "m") => ("m", 0.860, 0.330),
        (BATTLE, "a") => ("a", 0.755, 0.448),
        (BATTLE, "s") => ("s", 0.820, 0.880),
        (BATTLE, "b") => ("b", 0.880, 0.880),
        (BATTLE, "z") => ("z", 0.910, 0.720),
        (BATTLE, "x") => ("x", 0.965, 0.580),
        (BATTLE, "c") => ("c", 0.850, 0.580),
        (BATTLE, "v") => ("v", 0.910, 0.440),
        (MENU, "z") => ("z", 0.865, 0.851),
        (MENU, "x") => ("x", 0.933, 0.700),
        (MENU, "c") => ("c", 0.797, 0.700),
        (MENU, "v") => ("v", 0.865, 0.549),
        (MENU, "l") => ("l", 0.730, 0.440),
        (MENU, "p") => ("p", 0.930, 0.440),
        (MENU_NAV, "z") => ("z", 0.865, 0.851),
        (MENU_NAV, "x") => ("x", 0.933, 0.700),
        (MENU_NAV, "a") => ("a", 0.797, 0.851),
        _ => return None,
    };
    Some(LayoutOverride {context, name, x, y, scale: 1.0, visible: true, custom_key: None, custom_label: None})
}


/// Geometry and hit testing share the same saved position and scale.


/// Overall overlay opacity. At 100% the original artwork and colours are preserved.
/// Kept separate from hit testing, so faint controls still accept the same touches.
static OPACITY_PERCENT: AtomicU32 = AtomicU32::new(100);

pub fn set_opacity_percent(percent: u32) {
    OPACITY_PERCENT.store(percent.clamp(10, 100), Ordering::Relaxed);
}

pub fn opacity() -> f32 {
    OPACITY_PERCENT.load(Ordering::Relaxed) as f32 / 100.0
}

/// Overlay vertices contain x, y, r, g, b, alpha, u, v. Fade all batches alike,
/// including artwork, labels, the joystick and the party selector.
pub fn apply_opacity(vertices: &mut [f32]) {
    let alpha = opacity();
    for vertex in vertices.chunks_exact_mut(8) {
        vertex[5] *= alpha;
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
    let classic = overlay_style() == OverlayStyle::Classic;
    let placed = if classic {
        layout_override(controls()[index].name).is_some()
    } else {
        placement(controls()[index].name).is_some()
    };
    // A control the player placed by hand is already in window coordinates; only the built-in ones
    // that ride on the game image (the dialogue button) are moved with it.
    if controls()[index].contexts & DIALOG == 0 || placed {
        return shape
    }
    let (vx, vy, vw, vh) = viewport();
    match shape {
        Shape::Circle { cx, cy, r } => Shape::Circle { cx: vx + cx * vw, cy: vy + cy * vh, r: r * vw },
        Shape::Rect { cx, cy, w, h } => Shape::Rect { cx: vx + cx * vw, cy: vy + cy * vh, w: w * vw, h: h * vh },
    }
}

pub fn window_shape_for(index: usize, ww: f32, wh: f32) -> Shape {
    let shape = window_shape(index);
    if overlay_style() != OverlayStyle::Classic { return shape }
    if ww <= 0.0 || wh <= 0.0 || layout_override(controls()[index].name).is_none() { return shape }
    match shape {
        Shape::Circle { cx, cy, r } => {
            let aspect = control_art_aspect(controls()[index].name).unwrap_or(1.0);
            let rx = r.min(0.49 / aspect).min(0.49 * wh / ww);
            let margin = rx * aspect;
            let ry = rx * ww / wh;
            Shape::Circle { cx: cx.clamp(margin, 1.0 - margin), cy: cy.clamp(ry, 1.0 - ry), r: rx }
        },
        Shape::Rect { cx, cy, w, h } => {
            let (w, h) = (w.min(0.98), h.min(0.98));
            Shape::Rect { cx: cx.clamp(w / 2.0, 1.0 - w / 2.0), cy: cy.clamp(h / 2.0, 1.0 - h / 2.0), w, h }
        },
    }
}

/// The inverse: a point in window coordinates, expressed in a control's own coordinates.
pub fn control_point(index: usize, nx: f32, ny: f32) -> (f32, f32) {
    let classic = overlay_style() == OverlayStyle::Classic;
    let placed = if classic {
        layout_override(controls()[index].name).is_some()
    } else {
        placement(controls()[index].name).is_some()
    };
    if controls()[index].contexts & DIALOG == 0 || placed {
        return (nx, ny)
    }
    let (vx, vy, vw, vh) = viewport();
    ((nx - vx) / vw.max(0.0001), (ny - vy) / vh.max(0.0001))
}

/// Looks a control up by its name.
pub fn index_of(name: &str) -> Option<usize> {
    controls().iter().position(|control| control.name == name)
}

/// Where a control sits when the player has not moved it. Used when the saved layout only carries a
/// size, and by the launcher's editor as its starting point.
pub fn default_center(name: &str) -> Option<(f32, f32)> {
    if name == MOVEMENT_NAME {
        return Some((JOYSTICK_CX, JOYSTICK_CY));
    }
    controls().iter().find(|control| control.name == name).map(|control| match control.shape {
        Shape::Circle { cx, cy, .. } => (cx, cy),
        Shape::Rect { cx, cy, .. } => (cx, cy),
    })
}

/// How big a control is when the player has not resized it.
pub fn default_size(name: &str) -> f32 {
    if name == MOVEMENT_NAME {
        return 1.0;
    }
    let size = controls().iter().find(|control| control.name == name).map(|control| match control.shape {
        Shape::Circle { r, .. } => r,
        Shape::Rect { w, .. } => w,
    });
    size.unwrap_or(0.03)
}

/// The room the emulator reported, or -1 before the first frame.
static ROOM: AtomicI32 = AtomicI32::new(-1);

/// Called by the emulator every frame with the current room id.
pub fn set_room(room: i32) {
    static LAST_ROOM: AtomicI32 = AtomicI32::new(i32::MIN);
    if LAST_ROOM.swap(room, Ordering::Relaxed) != room {
        // A new screen starts on its first character, which is where the game's own selection starts.
        picker_reset();
        // A fresh picker offers its one-off trick button again; using it once was enough last time.
        COMBO_USED.store(false, Ordering::Relaxed);
        #[cfg(target_os = "android")]
        if room == PUZZLE_ROOM {
            crate::android_main::show_puzzle_hint();
        }
    }
    ROOM.store(room, Ordering::Relaxed);
    log_state_change(room);
}

/// Whether the game has one of its dialogue boxes open. Reported every frame by the engine, which
/// reads it from the game's own modal state; the overlay then leaves a single button.
static DIALOGUE: Sticky = Sticky::new();

pub fn set_dialogue(open: bool) {
    if DIALOGUE.report(open) {
        eprintln!(
            "[overlay] dialogo {}",
            if open { "abierto: solo el boton de avanzar" } else { "cerrado" }
        );
    }
}

pub fn dialogue_open() -> bool {
    DIALOGUE.get()
}

pub fn force_dialogue_closed() {
    DIALOGUE.clear();
}

/// Whether the game's own PokÃ©mon box (the PC storage) is open. Its screen is driven entirely with
/// the game's own click boxes, so nothing of the overlay is wanted there; the engine reports it from
/// the game's `ENCAJA` global.
static BOX_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_box_open(open: bool) {
    BOX_OPEN.store(open, Ordering::Relaxed);
}

pub fn box_open() -> bool {
    BOX_OPEN.load(Ordering::Relaxed)
}

/// Whether the game has its own text-entry dialog open (the modal that draws the on-screen keyboard,
/// GML script 7754). The engine reports it every time it draws, and the Android side raises the
/// system keyboard while it is true.
static TEXT_ENTRY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_text_entry(open: bool) {
    let was = TEXT_ENTRY.swap(open, Ordering::Relaxed);
    if was != open {
        eprintln!("[ime] entrada de texto {}", if open { "abierta" } else { "cerrada" });
    }
}

pub fn text_entry_open() -> bool {
    TEXT_ENTRY.load(Ordering::Relaxed)
}

/// Height of the system keyboard, in window pixels, while it is up. The Android side reports it and
/// the renderer slides the game picture up with it, so the text being edited stays above the
/// keyboard instead of hiding under it.
static IME_INSET: AtomicU32 = AtomicU32::new(0);

pub fn set_ime_inset(pixels: u32) {
    IME_INSET.store(pixels, Ordering::Relaxed);
}

pub fn ime_inset() -> u32 {
    IME_INSET.load(Ordering::Relaxed)
}

/// The height the system keyboard is taking, but only while the game's text dialog is open.
///
/// Outside the text dialog the reported height is ignored on purpose. A height left behind by a
/// callback that never came would otherwise keep the game pushed up top and the on-screen controls
/// hidden, and the player would have no way of getting them back: tying it to the dialog means
/// closing the dialog always puts everything back where it was.
pub fn text_entry_inset() -> u32 {
    if text_entry_open() {
        ime_inset()
    } else {
        0
    }
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
    static LAST_ROOM: AtomicI32 = AtomicI32::new(i32::MIN);
    let group = u32::from(current_context());
    let room_changed = LAST_ROOM.swap(room, Ordering::Relaxed) != room;
    if LAST.swap(group, Ordering::Relaxed) == group {
        // The group did not change, but knowing which room we are in is what makes it possible to
        // say "this screen needs no controls" about a specific one.
        if room_changed {
            eprintln!("[overlay] sala {} -> grupo {}", room, group);
        }
        return
    }
    let (window_w, window_h) = ramen::platform::android::window_size();
    let active = (0..controls().len()).filter(|index| is_active(*index)).count();
    eprintln!("[overlay] sala {} -> grupo {} | ventana {}x{} | {} de {} activos", room, group, window_w, window_h, active, controls().len());
    // Every control, one per line, so a report of "buttons missing" can be checked against what the
    // overlay actually decided: which key, where it was put, how big it is, and whether it counted
    // as visible. Normalised coordinates are fractions of the window, so they can be compared
    // between devices of any size.
    for (index, control) in controls().iter().enumerate() {
        // The shape actually in use, not the table's: a control that is moved to follow the game (the
        // picker's arrows) has to be reported where it really is.
        let (at, size) = match effective_shape(index) {
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

pub static KEYBOARD_WAIT_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_keyboard_wait_active(active: bool) {
    KEYBOARD_WAIT_ACTIVE.store(active, Ordering::SeqCst);
}

pub fn is_keyboard_wait_active() -> bool {
    KEYBOARD_WAIT_ACTIVE.load(Ordering::SeqCst)
}


static LAST_CONTEXT: AtomicU8 = AtomicU8::new(0);

pub fn update_context() {
    let new_ctx = current_context();
    let old_ctx = LAST_CONTEXT.swap(new_ctx, Ordering::Relaxed);
    if old_ctx != new_ctx && old_ctx != 0 {
        on_context_change(old_ctx, new_ctx);
    }
}

pub fn on_context_change(_old_ctx: u8, new_ctx: u8) {
    let ctrl_list = controls();
    for (i, ctrl) in ctrl_list.iter().enumerate() {
        if is_pressed(i) {
            // If the control was in old_ctx and is NOT active in new_ctx:
            if (ctrl.contexts & new_ctx) == 0 {
                set_pressed(i, false);
                unregister_autofire(i);
                #[cfg(target_os = "android")]
                ramen::platform::android::push_input(ramen::platform::android::InputEvent::KeyUp {
                    key: control_key(i),
                });
            }
        }
    }
}

/// Which group of controls the current game state needs.
pub fn current_context() -> u8 {
    if is_keyboard_wait_active() {
        return MENU_NAV;
    }
    // A dialogue box takes over the overlay: only the buttons that go with it are shown, so the text
    // 1. If an option list (e.g. PC storage list) or bag is open, MENU_NAV takes precedence immediately
    // over any lingering dialogue grace period, ensuring Z/X/A and joystick are active instantly.
    if modal_id() == LIST_MODAL || crate::touch_navigation::bag_open() {
        return MENU_NAV
    }
    // While a dialogue box is up the screen gets only the single dialogue-advance button, so the text
    // and the portrait stay readable.
    if dialogue_open() {
        return DIALOG
    }
    // The Pokémon box is entirely point-and-click, so nothing is needed next to it: no controls at
    // all. MENU is the context no control belongs to, which is exactly that.
    if box_open() {
        return MENU
    }
    // The system keyboard is up. It owns the bottom of the screen and the controls would sit under
    // it, so the whole overlay steps aside - buttons and joystick - until it is gone. Typing needs
    // no buttons, and the game's own dialogs are advanced with the keyboard's own key.
    if text_entry_inset() > 0 {
        return MENU
    }
    // The game's yes/no questions are answered by touching their Sí/No boxes: the buttons would
    // only cover them. Confirmed on the device: the exit-game and the escape question both report
    // 9673, and 9674 is the bigger variant. Read straight from the current frame (not the sticky
    // state) so the buttons come back as soon as the question closes instead of after a grace.
    if matches!(modal_id(), QUESTION_MODAL | 9674) {
        return MENU
    }
    match ROOM.load(Ordering::Relaxed) {
        // IntroROOM, InicioROOM, CargarPartida, MultijugadorMenu, CreditosInicioScreen:
        // these screens are fully clickable (once the game's own click boxes are fixed, see
        // `crate::patches`), so they need no on-screen controls at all.
        //
        // 528 is the Oak laboratory's introduction and 527 the unknown place that follows it: both
        // are cutscenes the player watches before they can take a step, so nothing of the map's
        // overlay belongs over them. Their dialogues still get their own button, handled above.
        // Confirmed on the device - the two report as rooms 528 and 527.
        22 | 30 | 43 | 461 | 527 | 528 => MENU,
        // OptionsROOM: always return MENU_NAV so controls remain visible and active
        465 | 466 => MENU_NAV,
        // The screen where a new game asks you to pick your character, just after the title menu. It
        // has controls of its own (the two arrows) instead of the map's. Its room cannot be named
        // from the game's data - the names are not readable - so this is the room it reports.
        //
        // A modal of its own - the preferences it asks about next, drawn as a list over the same
        // screen - is walked with the movement keys and answered with Z and X, so it gets those and
        // the joystick. A dialogue here is handled above and stays on the dialogue's two buttons; the
        // name the game types in is handled by the keyboard, so it gets nothing.
        CHARACTER_ROOM => {
            if text_entry_open() {
                MENU
            } else if matches!(modal_id(), PREFERENCES_MODAL | DIFFICULTY_MODAL) {
                CHOICE
            } else {
                CHARACTER
            }
        },
        // BattleROOM, BattleROOMAqua, BattleROOMLava.
        47 | 191 | 709 => BATTLE,
        // The Safari Zone's catching room. Confirmed on the device: it reports room 715 while the
        // ball is being thrown there. It keeps the map's buttons bar the stick and the attacks.
        715 => SAFARI,
        // The puzzle uses only its three buttons; everything else steps aside.
        PUZZLE_ROOM => MENU,
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

static INFO_WINDOW_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_info_window(open: bool) {
    INFO_WINDOW_OPEN.store(open, Ordering::Relaxed);
}

pub fn info_window_open() -> bool {
    INFO_WINDOW_OPEN.load(Ordering::Relaxed)
}


static OPPONENT_SPECIES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn set_opponent_species(species: u32) {
    OPPONENT_SPECIES.store(species, Ordering::Relaxed);
}

pub fn opponent_species() -> u32 {
    OPPONENT_SPECIES.load(Ordering::Relaxed)
}

static LAST_USER_INTERACTION: Mutex<Option<Instant>> = Mutex::new(None);

pub fn report_user_interaction() {
    if let Ok(mut lock) = LAST_USER_INTERACTION.lock() {
        *lock = Some(Instant::now());
    }
}

pub fn is_menu_idle_for_power_saving() -> bool {
    let ctx = current_context();
    if ctx == WORLD || ctx == BATTLE {
        return false;
    }
    if let Ok(lock) = LAST_USER_INTERACTION.lock() {
        if let Some(t) = *lock {
            return t.elapsed().as_secs() > 10;
        }
    }
    false
}

pub fn is_active(index: usize) -> bool {
    if overlay_style() == OverlayStyle::None {
        return false
    }
    if ROOM.load(Ordering::Relaxed) == PUZZLE_ROOM
        && matches!(controls()[index].name, ROTATE_NAME | "f1" | "x")
    {
        return !pad_active() && !info_window_open();
    }
    // In MENU modals / box management, allow hybrid mode with overlay buttons if enabled in layout
    if current_context() == MENU {
        let name = controls()[index].name;
        if let Some(entry) = layout_override(name) {
            return entry.visible;
        }
        return false;
    }
    // In DIALOG, only show the dialogue advancement button
    if current_context() == DIALOG {
        return controls()[index].contexts & DIALOG != 0;
    }
    let classic = overlay_style() == OverlayStyle::Classic;
    if classic {
        if current_context() == WORLD && controls()[index].name.starts_with("alpha") { return false }
        if controls()[index].name == crate::field_controls::NAME && current_context() != WORLD { return false }
        if controls()[index].name == PARTY_NAME && current_context() != BATTLE { return false }
        if layout_override(controls()[index].name).is_some_and(|entry| !entry.visible) { return false }
    }
    if pad_active() || info_window_open() || (controls()[index].contexts & current_context() == 0) {
        return false;
    }
    // The picker's trick button opens the whole roster once, and then has done its job: it is taken
    // away rather than left over the screen for the rest of the game. It comes back with the next new
    // game (see [set_room]).
    if controls()[index].name == COMBO_BUTTON_NAME && COMBO_USED.load(Ordering::Relaxed) {
        return false
    }
    // The picker's own controls belong to the moment the game is showing the characters to choose
    // from. With any of its modals up - the name dialog, the question it asks to check the data, a
    // summary - nothing of the picker belongs on top: those screens are gone through with the game's
    // own keyboard and buttons.
    if current_context() == CHARACTER && controls()[index].contexts & CHARACTER != 0 {
        if modal_open() {
            return false
        }
        // The two arrows travel with the characters, so they only belong where those are being chosen:
        // one (the player) or two (the rival). The game's own trick puts up a screen with the whole
        // roster at once, drawn as a grid; the boxes churn from frame to frame there and the arrows
        // would hop about, so they sit that one out. The rest of the controls stay put: they do not
        // follow anything.
        let candidates = picker_boxes().len();
        if matches!(controls()[index].label, Label::Arrow(_)) && !(1..=PICKER_MAX_CHARACTERS).contains(&candidates) {
            return false
        }
    }
    true
}

/// The movement control (joystick or D-pad) drives the arrow keys, which is how the game's menus (the
/// ones that still show controls) and the overworld are navigated.
pub fn movement_active() -> bool {
    if current_context() == DIALOG { return false; }
    if current_context() == MENU {
        return joystick_visible() && CLASSIC_LAYOUT.lock().unwrap().iter().any(|e| e.context == MENU && e.name == JOYSTICK_NAME && e.visible);
    }
    if overlay_style() == OverlayStyle::Classic { return joystick_active() }
    overlay_style() != OverlayStyle::None
        && !pad_active()
        && !info_window_open()
        && current_context() & (WORLD | BATTLE | CHOICE | MENU_NAV | MENU) != 0
}

/// The joystick drives the arrow keys. Used by the classic overlay.
pub fn joystick_active() -> bool {
    if current_context() == DIALOG { return false; }
    if current_context() == MENU {
        return joystick_visible() && CLASSIC_LAYOUT.lock().unwrap().iter().any(|e| e.context == MENU && e.name == JOYSTICK_NAME && e.visible);
    }
    !pad_active() && !info_window_open() && joystick_visible()
        && overlay_style() != OverlayStyle::None
        && current_context() & (WORLD | BATTLE | CHOICE | MENU_NAV | MENU) != 0
}

/// Returns the index of the control containing the point, if any. The point is normalised, but the
/// test is done in pixels so the touchable area matches what is drawn on a screen that is not square.
pub fn hit(nx: f32, ny: f32, ww: f32, wh: f32) -> Option<usize> {
    controls()
        .iter()
        .enumerate()
        .position(|(index, control)| is_active(index) && control_contains(index, control.name, nx, ny, ww, wh))
}

/// Whether a point is inside a control. A control with artwork is tested against the rectangle its
/// picture was actually drawn into, so a wide label is as touchable as it is painted instead of only
/// inside the little circle its key is defined with. The rest keep their own shape's test.
fn control_contains(index: usize, name: &str, nx: f32, ny: f32, ww: f32, wh: f32) -> bool {
    let shape = window_shape_for(index, ww, wh);
    if let Some(aspect) = control_art_aspect(name) {
        let (cx, cy, half) = match shape {
            Shape::Circle { cx, cy, r } => (cx, cy, r * ww),
            Shape::Rect { cx, cy, w, h } => (cx, cy, (w * ww + h * wh) / 4.0),
        };
        // The renderer sizes every picture by its height and lets its width follow its own aspect.
        return (nx - cx).abs() * ww <= half * aspect && (ny - cy).abs() * wh <= half
    }
    contains(shape, nx, ny, ww, wh)
}

/// The shape each control's artwork was painted in (width over height), as the renderer packed it.
/// Empty until the skin is loaded, and only used to widen the touch area of a picture that is not as
/// tall as it is wide.
static SPRITE_OPACITY: Mutex<f32> = Mutex::new(0.5);

pub fn set_sprite_opacity(value: f32) {
    *SPRITE_OPACITY.lock().unwrap() = value.clamp(0.05, 1.0);
}

pub fn sprite_opacity() -> f32 {
    *SPRITE_OPACITY.lock().unwrap()
}

/// Which overlay the player wants drawn.
#[derive(Clone, Copy, PartialEq)]
pub enum OverlayStyle {
    /// The new, semi-transparent controls drawn as smooth shapes. This is the default.
    New,
    /// The original dynamic controls: the pixel-art skin when it is there, plain shapes otherwise.
    Classic,
    /// No on-screen controls at all; the game is played with a pad or by touching the game itself.
    None,
}

static STYLE: AtomicU8 = AtomicU8::new(1);

pub fn set_overlay_style(style: OverlayStyle) {
    STYLE.store(
        match style {
            OverlayStyle::New => 0,
            OverlayStyle::Classic => 1,
            OverlayStyle::None => 2,
        },
        Ordering::Relaxed,
    );
}

pub fn overlay_style() -> OverlayStyle {
    match STYLE.load(Ordering::Relaxed) {
        1 => OverlayStyle::Classic,
        2 => OverlayStyle::None,
        _ => OverlayStyle::New,
    }
}

/// What drives the arrow keys: the analog joystick or a four-way pad.
#[derive(Clone, Copy, PartialEq)]
pub enum MoveMode {
    Joystick,
    Dpad,
}

static MOVE: AtomicU8 = AtomicU8::new(0);

pub fn set_move_mode(mode: MoveMode) {
    MOVE.store(if mode == MoveMode::Dpad { 1 } else { 0 }, Ordering::Relaxed);
}

pub fn move_mode() -> MoveMode {
    // Moderno always moves with the joystick; the choice only exists for the classic look.
    if crate::touch_appearance::settings().style == crate::touch_appearance::Style::Modern {
        return MoveMode::Joystick
    }
    if MOVE.load(Ordering::Relaxed) == 1 {
        MoveMode::Dpad
    } else {
        MoveMode::Joystick
    }
}

/// The name the movement control (joystick or D-pad) is saved under, both in the layout and in the
/// launcher's editor. It is not one of [controls()]: it is drawn and hit-tested on its own.
pub const MOVEMENT_NAME: &str = "dpad";

/// Where the player moved and resized a control, keyed by its [Control::name]. Only the entries the
/// player actually touched are stored, so the rest keep the places built into the table.
static NEW_LAYOUT: Mutex<Vec<(String, f32, f32, f32)>> = Mutex::new(Vec::new());

pub fn set_placement(name: &str, cx: f32, cy: f32, scale: f32) {
    let mut layout = NEW_LAYOUT.lock().unwrap();
    if let Some(entry) = layout.iter_mut().find(|entry| entry.0 == name) {
        *entry = (name.to_string(), cx, cy, scale);
    } else {
        layout.push((name.to_string(), cx, cy, scale));
    }
}

pub fn clear_layout() {
    NEW_LAYOUT.lock().unwrap().clear();
}

fn placement(name: &str) -> Option<(f32, f32, f32)> {
    NEW_LAYOUT.lock().unwrap().iter().find(|entry| entry.0 == name).map(|entry| (entry.1, entry.2, entry.3))
}

/// A size multiplier applied to every control of the new overlay.
static UI_SCALE: Mutex<f32> = Mutex::new(1.0);

pub fn set_ui_scale(scale: f32) {
    *UI_SCALE.lock().unwrap() = scale.clamp(0.3, 3.0);
}

pub fn ui_scale() -> f32 {
    *UI_SCALE.lock().unwrap()
}

/// Multiplies the size part of a shape, leaving its centre where it is.
fn scale_shape(shape: Shape, scale: f32) -> Shape {
    match shape {
        Shape::Circle { cx, cy, r } => Shape::Circle { cx, cy, r: r * scale },
        Shape::Rect { cx, cy, w, h } => Shape::Rect { cx, cy, w: w * scale, h: h * scale },
    }
}

/// The shape each control's artwork was painted in (width over height), as the renderer packed it.
/// Empty until the skin is loaded, and only used to widen the touch area of a picture that is not as
/// tall as it is wide.
static ART_ASPECTS: Mutex<Vec<(&'static str, f32)>> = Mutex::new(Vec::new());

pub fn set_art_aspect(name: &'static str, aspect: f32) {
    let mut aspects = ART_ASPECTS.lock().unwrap();
    aspects.retain(|(known, _)| *known != name);
    aspects.push((name, aspect));
}

pub fn control_art_aspect(name: &str) -> Option<f32> {
    ART_ASPECTS.lock().unwrap().iter().find(|(known, _)| *known == name).map(|(_, aspect)| *aspect)
}

static PRESSED: AtomicU64 = AtomicU64::new(0);

pub fn set_pressed(index: usize, pressed: bool) {
    if is_pressed(index) != pressed { crate::touch_appearance::set_pressed(index, pressed); }
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
    (0..controls().len()).filter(|&index| is_pressed(index) && !is_active(index)).collect()
}

/// Forcibly releases all held virtual touch controls and stick directions.
/// Called upon room changes, faint transitions, or state switches to prevent sticky inputs.
pub fn flush_held_inputs() {
    let pressed_mask = PRESSED.swap(0, Ordering::Relaxed);
    for index in 0..controls().len() {
        if pressed_mask & (1 << index) != 0 {
            crate::touch_appearance::set_pressed(index, false);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Movement (joystick or D-pad). It presses the arrow keys depending on which way it is pushed.

pub const JOYSTICK_CX: f32 = 0.135;
pub const JOYSTICK_CY: f32 = 0.700;
/// Radius as a fraction of the window *height*, so the base stays round on screen.
pub const JOYSTICK_R: f32 = 0.200;
/// Dead zone, as a fraction of the radius, before a direction registers.
pub const JOYSTICK_DEAD: f32 = 0.35;

/// The movement control's centre, in normalised coordinates.
pub fn movement_center() -> (f32, f32) {
    if overlay_style() == OverlayStyle::Classic { return joystick_center() }
    placement(MOVEMENT_NAME).map(|(cx, cy, _)| (cx, cy)).unwrap_or((JOYSTICK_CX, JOYSTICK_CY))
}

/// Radius as a fraction of the window height.
pub fn movement_radius() -> f32 {
    if overlay_style() == OverlayStyle::Classic { return joystick_radius() }
    let scale = placement(MOVEMENT_NAME).map(|(_, _, scale)| scale).unwrap_or(1.0);
    JOYSTICK_R * scale * ui_scale()
}

/// The joystick's centre, in normalised coordinates.
pub fn joystick_center() -> (f32, f32) {
    layout_override(JOYSTICK_NAME).map_or((JOYSTICK_CX, JOYSTICK_CY), |entry| (entry.x, entry.y))
}

pub fn joystick_center_in_window(ww: f32, wh: f32) -> (f32, f32) {
    let (cx, cy) = joystick_center();
    if ww <= 0.0 || wh <= 0.0 || layout_override(JOYSTICK_NAME).is_none() { return (cx, cy) }
    let ry = (joystick_radius() * 1.05).min(0.49);
    let rx = (ry * wh / ww).min(0.49);
    (cx.clamp(rx, 1.0 - rx), cy.clamp(ry, 1.0 - ry))
}

/// Radius as a fraction of the window height.
pub fn joystick_radius() -> f32 {
    JOYSTICK_R * layout_override(JOYSTICK_NAME).map_or(1.0, |entry| entry.scale)
}

pub fn joystick_visible() -> bool {
    layout_override(JOYSTICK_NAME).map_or(true, |entry| entry.visible)
}

pub fn movement_hit(nx: f32, ny: f32, ww: f32, wh: f32) -> bool {
    if !movement_active() {
        return false
    }
    let (cx, cy) = if overlay_style() == OverlayStyle::Classic { joystick_center_in_window(ww, wh) } else { movement_center() };
    let r = if overlay_style() == OverlayStyle::Classic { joystick_radius() } else { movement_radius() } * wh;
    let dx = (nx - cx) * ww;
    let dy = (ny - cy) * wh;
    dx * dx + dy * dy <= r * r
}

/// `[up, down, left, right]` to hold for the movement control at (nx, ny).
pub fn movement_dirs(nx: f32, ny: f32, ww: f32, wh: f32) -> [bool; 4] {
    let classic = overlay_style() == OverlayStyle::Classic;
    let (cx, cy) = if classic { joystick_center_in_window(ww, wh) } else { movement_center() };
    let r = if classic { joystick_radius() } else { movement_radius() } * wh;
    let dx = (nx - cx) * ww / r;
    let dy = (ny - cy) * wh / r;
    // The D-pad is four discrete zones, not an analog stick: the strongest arm wins and the centre
    // does nothing. That is what makes it feel (and light up) like a real cross.
    if move_mode() == MoveMode::Dpad {
        const DPAD_DEAD: f32 = 0.22;
        const DIAG_THRESHOLD: f32 = 0.4142; // Umbral trigonométrico tan(22.5°) para diagonales de 45°

        let ax = dx.abs();
        let ay = dy.abs();

        if ax.max(ay) < DPAD_DEAD {
            return [false; 4];
        }

        let is_diagonal = (ax.min(ay) / ax.max(ay)) > DIAG_THRESHOLD;

        let up = dy < 0.0 && (ay > ax || is_diagonal);
        let down = dy > 0.0 && (ay > ax || is_diagonal);
        let left = dx < 0.0 && (ax > ay || is_diagonal);
        let right = dx > 0.0 && (ax > ay || is_diagonal);

        return [up, down, left, right];
    }
    stick_dirs(dx, dy)
}

static DYNAMIC_JOYSTICK_DEAD: AtomicU32 = AtomicU32::new(35);

pub fn set_joystick_dead(pct: u32) {
    DYNAMIC_JOYSTICK_DEAD.store(pct.clamp(10, 60), Ordering::Relaxed);
}

pub fn joystick_dead() -> f32 {
    DYNAMIC_JOYSTICK_DEAD.load(Ordering::Relaxed) as f32 / 100.0
}

pub fn joystick_hit(nx: f32, ny: f32, ww: f32, wh: f32) -> bool {
    movement_hit(nx, ny, ww, wh)
}

pub fn joystick_dirs(nx: f32, ny: f32, ww: f32, wh: f32) -> [bool; 4] {
    movement_dirs(nx, ny, ww, wh)
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
static MOVEMENT_OFFSET: Mutex<(f32, f32)> = Mutex::new((0.0, 0.0));

pub fn set_movement_offset(dx: f32, dy: f32) {
    *MOVEMENT_OFFSET.lock().unwrap() = (dx, dy);
}

pub fn movement_offset() -> (f32, f32) {
    *MOVEMENT_OFFSET.lock().unwrap()
}

/// Which directions the movement control is pressing right now, so the D-pad can light up the arm.
static MOVE_DIRS: AtomicU32 = AtomicU32::new(0);

pub fn set_move_dirs(dirs: [bool; 4]) {
    let mut bits = 0u32;
    for (i, on) in dirs.iter().enumerate() {
        if *on {
            bits |= 1 << i;
        }
    }
    MOVE_DIRS.store(bits, Ordering::Relaxed);
}

pub fn move_dirs() -> [bool; 4] {
    let bits = MOVE_DIRS.load(Ordering::Relaxed);
    [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0]
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
// and drops it here, so the overlay can draw each PokÃ©mon's own sprite in its hexagon.

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
    WHEEL.load(Ordering::Relaxed) && index_of(PARTY_NAME).is_some_and(is_active)
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
/// honeycomb of seven hexagons is 2(1+âˆš3) radii wide and 5 radii tall, so its hexagons have a
/// circumradius of 79/2/(1+âˆš3) â‰ˆ 14.5: each side slot sits âˆš3 radii out and each diagonal
/// (âˆš3/2, 1.5) radii out. Horizontal offsets are fractions of the width, vertical of the height,
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

static WHEEL_TOUCH_ORIGIN: Mutex<Option<(f32, f32)>> = Mutex::new(None);

pub fn set_wheel_touch_origin(x: f32, y: f32) {
    *WHEEL_TOUCH_ORIGIN.lock().unwrap() = Some((x, y));
}

pub fn wheel_touch_origin() -> Option<(f32, f32)> {
    *WHEEL_TOUCH_ORIGIN.lock().unwrap()
}

pub fn clear_wheel_touch_origin() {
    *WHEEL_TOUCH_ORIGIN.lock().unwrap() = None;
}

/// Which slot the finger is pointing at, relative to the touch origin of the Gengar button.
pub fn wheel_item_at(nx: f32, ny: f32, ww: f32, wh: f32) -> Option<usize> {
    let (ox, oy) = match wheel_touch_origin() {
        Some((tx, ty)) => (tx * ww, ty * wh),
        None => wheel_center(ww, wh),
    };
    let dx = nx * ww - ox;
    let dy = ny * wh - oy;
    // Deadzone of 16px (16^2 = 256.0)
    if dx * dx + dy * dy < 256.0 {
        return None;
    }
    let mut deg = dy.atan2(dx).to_degrees();
    if deg < 0.0 {
        deg += 360.0;
    }
    let sector = (((deg + 30.0) % 360.0) / 60.0).floor() as usize % 6;
    const SECTOR_TO_SLOT: [usize; 6] = [3, 5, 4, 2, 0, 1];
    Some(SECTOR_TO_SLOT[sector])
}

#[cfg(test)]
pub fn expire_context_grace() {
    *DIALOGUE.seen.lock().unwrap() = None;
    *MODAL_OPEN.seen.lock().unwrap() = None;
}

/// Different visible controls may share a key after adding buttons to another profile.
pub fn control_keys(index: usize) -> [Option<Key>; 2] {
    if controls()[index].name == COMBO_BUTTON_NAME { [Some(Key::LeftControl),Some(Key::P)] }
    else { [Some(control_key(index)),None] }
}
pub fn key_held_by_buttons(key: Key) -> bool {
    (0..controls().len()).any(|i| is_pressed(i) && control_keys(i).contains(&Some(key)))
}



pub const RADIAL_MENU_NAME: &str = "radial_menu";
pub const TURBO_NAME: &str = "turbo";
pub const MUTE_NAME: &str = "mute";
pub const EDIT_NAME: &str = "edit";
pub const GUIDE_NAME: &str = "guide";
pub const LOG_NAME: &str = "log";

static CUSTOM_OFFSETS: Mutex<Option<HashMap<String, (f32, f32)>>> = Mutex::new(None);

pub fn set_control_offset(name: &str, ox: f32, oy: f32) {
    let mut lock = CUSTOM_OFFSETS.lock().unwrap_or_else(|e| e.into_inner());
    let map = lock.get_or_insert_with(HashMap::new);
    map.insert(name.to_string(), (ox, oy));
}

pub fn control_offset(name: &str) -> (f32, f32) {
    let lock = CUSTOM_OFFSETS.lock().unwrap_or_else(|e| e.into_inner());
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
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|set| set.contains(name))
        .unwrap_or(false)
}

pub fn set_button_hidden(name: &str, hidden: bool) {
    let mut lock = HIDDEN_BUTTONS.lock().unwrap_or_else(|e| e.into_inner());
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
        *CUSTOM_OFFSETS.lock().unwrap_or_else(|e| e.into_inner()) = Some(map);
        *HIDDEN_BUTTONS.lock().unwrap_or_else(|e| e.into_inner()) = Some(hidden_set);
    }
}

pub fn save_layout_file(path: &Path) {
    let lock = CUSTOM_OFFSETS.lock().unwrap_or_else(|e| e.into_inner());
    let hidden_lock = HIDDEN_BUTTONS.lock().unwrap_or_else(|e| e.into_inner());
    let mut text = String::from("# Layout personalizado de controles PokÃ©mon Reloaded Android\n");
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
static TURBO_LEVEL: AtomicU8 = AtomicU8::new(0); // 0 = 1x (normal), 1 = 2x (rápido), 2 = 4x (extremo)

pub fn turbo_level() -> u8 {
    TURBO_LEVEL.load(Ordering::Relaxed)
}

pub fn cycle_turbo() -> u8 {
    let next = (turbo_level() + 1) % 3;
    TURBO_LEVEL.store(next, Ordering::Relaxed);
    match next {
        0 => {
            FAST_FORWARD.store(false, Ordering::Relaxed);
            FAST_FORWARD_SPEED.store(10, Ordering::Relaxed);
        },
        1 => {
            FAST_FORWARD.store(true, Ordering::Relaxed);
            FAST_FORWARD_SPEED.store(20, Ordering::Relaxed);
        },
        2 => {
            FAST_FORWARD.store(true, Ordering::Relaxed);
            FAST_FORWARD_SPEED.store(40, Ordering::Relaxed);
        },
        _ => {},
    }
    next
}

pub fn fast_forward_active() -> bool {
    turbo_level() > 0 || FAST_FORWARD.load(Ordering::Relaxed)
}

pub fn toggle_fast_forward() -> bool {
    let active = !fast_forward_active();
    FAST_FORWARD.store(active, Ordering::Relaxed);
    if !active {
        TURBO_LEVEL.store(0, Ordering::Relaxed);
    } else if turbo_level() == 0 {
        TURBO_LEVEL.store(1, Ordering::Relaxed);
    }
    active
}

pub fn fast_forward_multiplier() -> f32 {
    if fast_forward_active() {
        match turbo_level() {
            1 => 2.0,
            2 => 4.0,
            _ => FAST_FORWARD_SPEED.load(Ordering::Relaxed) as f32 / 10.0,
        }
    } else {
        1.0
    }
}

pub fn set_fast_forward_speed(tenths: u32) {
    FAST_FORWARD_SPEED.store(tenths.clamp(10, 100), Ordering::Relaxed);
}

static ACTIVE_USER_PROFILE: AtomicU8 = AtomicU8::new(0); // 0 = P1, 1 = P2, 2 = P3, 3 = P4

pub fn active_user_profile() -> u8 {
    ACTIVE_USER_PROFILE.load(Ordering::Relaxed)
}

pub fn set_active_user_profile(profile: u8) {
    ACTIVE_USER_PROFILE.store(profile % 4, Ordering::Relaxed);
}

pub fn cycle_user_profile() -> u8 {
    let next = (active_user_profile() + 1) % 4;
    set_active_user_profile(next);
    next
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
    let mut lock = AUTOFIRE_HELD.lock().unwrap_or_else(|e| e.into_inner());
    let set = lock.get_or_insert_with(std::collections::HashSet::new);
    set.insert(index);
}

pub fn unregister_autofire(index: usize) {
    let mut lock = AUTOFIRE_HELD.lock().unwrap_or_else(|e| e.into_inner());
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
        let lock = AUTOFIRE_HELD.lock().unwrap_or_else(|e| e.into_inner());
        match lock.as_ref() {
            Some(set) if !set.is_empty() => set.iter().copied().collect::<Vec<_>>(),
            _ => return,
        }
    };

    let tick = AUTOFIRE_TICK.fetch_add(1, Ordering::Relaxed);
    // Cycle every 4 frames (2 frames pressed, 2 frames released) -> 15 pulses/sec at 60 FPS
    let phase = tick % 4;
    for index in buttons {
        if index < controls().len() {
            let key = control_key(index);
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
    *DYNAMIC_JOYSTICK_ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = pos;
}

static EDIT_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LAYOUT_SNAPSHOT: Mutex<Option<(HashMap<String, (f32, f32)>, u32, u32)>> = Mutex::new(None);

pub fn snapshot_layout() {
    let offsets = CUSTOM_OFFSETS.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default();
    let sc = SCALE.load(Ordering::Relaxed);
    let op = OPACITY.load(Ordering::Relaxed);
    *LAYOUT_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = Some((offsets, sc, op));
}

pub fn restore_layout_snapshot() {
    if let Some((offsets, sc, op)) = LAYOUT_SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).take() {
        *CUSTOM_OFFSETS.lock().unwrap_or_else(|e| e.into_inner()) = Some(offsets);
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


// ============================================================================
// MOD EXTENSIONS & PERSISTENT SETTINGS
// ============================================================================


/// The stick's offset from its centre, in units of its radius (for drawing the knob).
static JOYSTICK_OFFSET: Mutex<(f32, f32)> = Mutex::new((0.0, 0.0));

pub fn set_joystick_offset(dx: f32, dy: f32) {
    *JOYSTICK_OFFSET.lock().unwrap_or_else(|e| e.into_inner()) = (dx, dy);
}

pub fn joystick_offset() -> (f32, f32) {
    *JOYSTICK_OFFSET.lock().unwrap_or_else(|e| e.into_inner())
}

static OPACITY: AtomicU32 = AtomicU32::new(70);

pub fn set_opacity(pct: u32) {
    OPACITY.store(pct.clamp(10, 100), Ordering::Relaxed);
    set_opacity_percent(pct);
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

// ============================================================================
// RADIAL MENU, BEZEL, HUD OVERLAYS & GAMEPLAY TOGGLES
// ============================================================================

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
    Turbo,        // Sector 0 (Top / 12h): >> Fast Forward 1x -> 2x -> 4x
    QuickSave,    // Sector 1 (Top-Right / 1:30): Floppy Save State
    QuickLoad,    // Sector 2 (Right / 3h): Return Folder Load State
    TypeChart,    // Sector 3 (Bottom-Right / 4:30): Elemental Trio Weakness Guide
    CycleProfile, // Sector 4 (Bottom / 6h): User Profiles P1 -> P2 -> P3 -> P4
    Mute,         // Sector 5 (Bottom-Left / 7:30): Audio / Mute toggle
    Fps,          // Sector 6 (Left / 9h): Performance / FPS HUD toggle
    Snapshot,     // Sector 7 (Top-Left / 10:30): Camera Screen Capture
    Close,        // Center disc (X): Close Radial Wheel
    Outside,      // Touched outside: Dismiss Wheel
}

pub const RADIAL_SECTORS_COUNT: usize = 8;

pub fn radial_menu_center(ww: f32, wh: f32) -> (f32, f32) {
    (ww * 0.5, wh * 0.5)
}

pub fn radial_menu_inner_radius(ww: f32, wh: f32) -> f32 {
    ww.min(wh) * 0.09
}

pub fn radial_menu_outer_radius(ww: f32, wh: f32) -> f32 {
    ww.min(wh) * 0.44
}

pub fn radial_menu_hit(tx: f32, ty: f32, ww: f32, wh: f32) -> RadialAction {
    let (cx, cy) = radial_menu_center(ww, wh);
    let r_inner = radial_menu_inner_radius(ww, wh);
    let r_outer = radial_menu_outer_radius(ww, wh);
    let dx = tx - cx;
    let dy = ty - cy;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist <= r_inner {
        return RadialAction::Close;
    }
    if dist > r_outer * 1.15 {
        return RadialAction::Outside;
    }

    // Angle in radians (-PI..PI). Top is -PI/2.
    let angle = dy.atan2(dx);
    // Shift so Top (-PI/2) maps to 0
    let rel_angle = (angle + std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU) % std::f32::consts::TAU;
    let sector = (((rel_angle + std::f32::consts::FRAC_PI_8) / std::f32::consts::FRAC_PI_4) as usize) % RADIAL_SECTORS_COUNT;

    match sector {
        0 => RadialAction::Turbo,        // Sector 1: TURBO
        1 => RadialAction::QuickSave,    // Sector 2: GUARDAR
        2 => RadialAction::QuickLoad,    // Sector 3: CARGAR
        3 => RadialAction::TypeChart,    // Sector 4: TIPOS
        4 => RadialAction::CycleProfile, // Sector 5: PERFIL
        5 => RadialAction::Mute,         // Sector 6: SONIDO
        6 => RadialAction::Fps,          // Sector 7: FPS
        7 => RadialAction::Snapshot,     // Sector 8: CAPTURA
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

// ---------------------------------------------------------------------------------------------
// Opciones de Ajustes -> Juego (Comodidad Externa y Opcional)

static BERRY_TRACKER: AtomicBool = AtomicBool::new(false);
static EGG_HATCH_TIMER: AtomicBool = AtomicBool::new(false);
static KEY_ITEMS_RADIAL: AtomicBool = AtomicBool::new(false);
static AUTO_RUN: AtomicBool = AtomicBool::new(false);
static DIALOG_TAP_ADVANCE: AtomicBool = AtomicBool::new(false);
static REPEL_REMINDER: AtomicBool = AtomicBool::new(false);
static SHINY_BEACON: AtomicBool = AtomicBool::new(false);
static PUZZLE_PAUSE: AtomicBool = AtomicBool::new(false);
static FISHING_HAPTIC: AtomicBool = AtomicBool::new(false);
static QUICK_SAVE: AtomicBool = AtomicBool::new(false);
static SAVE_REMINDER: AtomicBool = AtomicBool::new(false);

pub fn berry_tracker_enabled() -> bool { BERRY_TRACKER.load(Ordering::Relaxed) }
pub fn set_berry_tracker(enabled: bool) { BERRY_TRACKER.store(enabled, Ordering::Relaxed); }

pub fn egg_hatch_timer_enabled() -> bool { EGG_HATCH_TIMER.load(Ordering::Relaxed) }
pub fn set_egg_hatch_timer(enabled: bool) { EGG_HATCH_TIMER.store(enabled, Ordering::Relaxed); }

pub fn key_items_radial_enabled() -> bool { KEY_ITEMS_RADIAL.load(Ordering::Relaxed) }
pub fn set_key_items_radial(enabled: bool) { KEY_ITEMS_RADIAL.store(enabled, Ordering::Relaxed); }

pub fn auto_run_enabled() -> bool { AUTO_RUN.load(Ordering::Relaxed) }
pub fn set_auto_run(enabled: bool) { AUTO_RUN.store(enabled, Ordering::Relaxed); }

pub fn dialog_tap_advance_enabled() -> bool { DIALOG_TAP_ADVANCE.load(Ordering::Relaxed) }
pub fn set_dialog_tap_advance(enabled: bool) { DIALOG_TAP_ADVANCE.store(enabled, Ordering::Relaxed); }

pub fn repel_reminder_enabled() -> bool { REPEL_REMINDER.load(Ordering::Relaxed) }
pub fn set_repel_reminder(enabled: bool) { REPEL_REMINDER.store(enabled, Ordering::Relaxed); }

pub fn shiny_beacon_enabled() -> bool { SHINY_BEACON.load(Ordering::Relaxed) }
pub fn set_shiny_beacon(enabled: bool) { SHINY_BEACON.store(enabled, Ordering::Relaxed); }

pub fn puzzle_pause_enabled() -> bool { PUZZLE_PAUSE.load(Ordering::Relaxed) }
pub fn set_puzzle_pause(enabled: bool) { PUZZLE_PAUSE.store(enabled, Ordering::Relaxed); }

pub fn fishing_haptic_enabled() -> bool { FISHING_HAPTIC.load(Ordering::Relaxed) }
pub fn set_fishing_haptic(enabled: bool) { FISHING_HAPTIC.store(enabled, Ordering::Relaxed); }

pub fn quick_save_enabled() -> bool { QUICK_SAVE.load(Ordering::Relaxed) }
pub fn set_quick_save(enabled: bool) { QUICK_SAVE.store(enabled, Ordering::Relaxed); }

pub fn save_reminder_enabled() -> bool { SAVE_REMINDER.load(Ordering::Relaxed) }
pub fn set_save_reminder(enabled: bool) { SAVE_REMINDER.store(enabled, Ordering::Relaxed); }

static LAST_SAVE_REMINDER: Mutex<Option<Instant>> = Mutex::new(None);

pub fn tick_save_reminder() {
    if !save_reminder_enabled() {
        return;
    }
    let mut lock = LAST_SAVE_REMINDER.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    match *lock {
        None => *lock = Some(now),
        Some(last) => {
            // Cada 25 minutos de juego activo sin guardar muestra aviso en el HUD
            if last.elapsed() >= Duration::from_secs(25 * 60) {
                *lock = Some(now);
                show_toast("RECORDATORIO: Â¡No olvides guardar tu partida!");
            }
        }
    }
}

static PUZZLE_PAUSED: AtomicBool = AtomicBool::new(false);

pub fn is_puzzle_paused() -> bool {
    PUZZLE_PAUSED.load(Ordering::Relaxed)
}

pub fn toggle_puzzle_pause() -> bool {
    let was = is_puzzle_paused();
    PUZZLE_PAUSED.store(!was, Ordering::Relaxed);
    if !was {
        show_toast("â¸ï¸ JUEGO PAUSADO (Toca para reanudar)");
    } else {
        show_toast("â–¶ï¸ JUEGO REANUDADO");
    }
    !was
}

pub fn set_puzzle_paused(paused: bool) {
    PUZZLE_PAUSED.store(paused, Ordering::Relaxed);
}

static RENDER_SCALE: AtomicU32 = AtomicU32::new(100);

pub fn get_render_scale() -> f32 {
    let s = RENDER_SCALE.load(Ordering::Relaxed);
    if s == 0 { 1.0 } else { s as f32 / 100.0 }
}

pub fn set_render_scale(scale: f32) {
    let clamped = (scale * 100.0).round().clamp(50.0, 200.0) as u32;
    RENDER_SCALE.store(clamped, Ordering::Relaxed);
}

static REPEL_STEPS: AtomicU32 = AtomicU32::new(0);
static EGG_STEPS: AtomicU32 = AtomicU32::new(0);

pub fn tick_movement_step(is_moving: bool) {
    if !is_moving {
        return;
    }
    if repel_reminder_enabled() && current_context() == WORLD {
        let count = REPEL_STEPS.fetch_add(1, Ordering::Relaxed);
        // ~1500 frames of movement â‰ˆ 250 steps in overworld
        if count >= 1500 {
            REPEL_STEPS.store(0, Ordering::Relaxed);
            show_toast("REPELENTE: Revisa si el efecto se ha agotado");
            #[cfg(target_os = "android")]
            crate::android_main::jni_vibrate(25);
        }
    }
    if egg_hatch_timer_enabled() && current_context() == WORLD {
        let count = EGG_STEPS.fetch_add(1, Ordering::Relaxed);
        // ~3000 movement frames â‰ˆ 500 steps (standard egg hatch cycle)
        if count >= 3000 {
            EGG_STEPS.store(0, Ordering::Relaxed);
            show_toast("HUEVO: Ciclo de eclosión alcanzado (~500 pasos)");
            #[cfg(target_os = "android")]
            crate::android_main::jni_vibrate(35);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Opciones Maestras de v0.9.0 (Opcionales en Ajustes)

static EXTREME_COMBAT: AtomicBool = AtomicBool::new(false);
static MALI_EARLYZ: AtomicBool = AtomicBool::new(false);
static GPU_WAKEUP: AtomicBool = AtomicBool::new(false);
static SMART_PREROLL_SKIP: AtomicBool = AtomicBool::new(false);
static ADAPTIVE_LTPO: AtomicBool = AtomicBool::new(false);
static PIP_MODE: AtomicBool = AtomicBool::new(false);
static LIVE_ACTIVITY: AtomicBool = AtomicBool::new(false);
static ACTIVE_STATIC_PARTITION: AtomicBool = AtomicBool::new(false);
static MEGA_ATLAS: AtomicBool = AtomicBool::new(false);

pub fn extreme_combat_enabled() -> bool { EXTREME_COMBAT.load(Ordering::Relaxed) }
pub fn set_extreme_combat(enabled: bool) { EXTREME_COMBAT.store(enabled, Ordering::Relaxed); }

pub fn mali_earlyz_enabled() -> bool { MALI_EARLYZ.load(Ordering::Relaxed) }
pub fn set_mali_earlyz(enabled: bool) { MALI_EARLYZ.store(enabled, Ordering::Relaxed); }

pub fn gpu_wakeup_enabled() -> bool { GPU_WAKEUP.load(Ordering::Relaxed) }
pub fn set_gpu_wakeup(enabled: bool) { GPU_WAKEUP.store(enabled, Ordering::Relaxed); }

pub fn smart_preroll_skip_enabled() -> bool { SMART_PREROLL_SKIP.load(Ordering::Relaxed) }
pub fn set_smart_preroll_skip(enabled: bool) { SMART_PREROLL_SKIP.store(enabled, Ordering::Relaxed); }

pub fn adaptive_ltpo_enabled() -> bool { ADAPTIVE_LTPO.load(Ordering::Relaxed) }
pub fn set_adaptive_ltpo(enabled: bool) { ADAPTIVE_LTPO.store(enabled, Ordering::Relaxed); }

pub fn pip_mode_enabled() -> bool { PIP_MODE.load(Ordering::Relaxed) }
pub fn set_pip_mode(enabled: bool) { PIP_MODE.store(enabled, Ordering::Relaxed); }

pub fn live_activity_enabled() -> bool { LIVE_ACTIVITY.load(Ordering::Relaxed) }
pub fn set_live_activity(enabled: bool) { LIVE_ACTIVITY.store(enabled, Ordering::Relaxed); }

pub fn active_static_partition_enabled() -> bool { ACTIVE_STATIC_PARTITION.load(Ordering::Relaxed) }
pub fn set_active_static_partition(enabled: bool) { ACTIVE_STATIC_PARTITION.store(enabled, Ordering::Relaxed); }

pub fn mega_atlas_enabled() -> bool { MEGA_ATLAS.load(Ordering::Relaxed) }
pub fn set_mega_atlas(enabled: bool) { MEGA_ATLAS.store(enabled, Ordering::Relaxed); }

// Complementos Avanzados de Rendimiento v9.7 Ultra Engine (activos por defecto para máximo rendimiento)
static TBDR_SURFACE_FUSION: AtomicBool = AtomicBool::new(true);
static ASYNC_RENDER_THREAD: AtomicBool = AtomicBool::new(true);
static TOWN_ZSORT_BAKING: AtomicBool = AtomicBool::new(true);
static SHADER_BINARY_PRECACHE: AtomicBool = AtomicBool::new(true);
static NPC_TICK_CULLING: AtomicBool = AtomicBool::new(true);

pub fn tbdr_surface_fusion_enabled() -> bool { TBDR_SURFACE_FUSION.load(Ordering::Relaxed) }
pub fn set_tbdr_surface_fusion(enabled: bool) { TBDR_SURFACE_FUSION.store(enabled, Ordering::Relaxed); }

pub fn async_render_thread_enabled() -> bool { ASYNC_RENDER_THREAD.load(Ordering::Relaxed) }
pub fn set_async_render_thread(enabled: bool) { ASYNC_RENDER_THREAD.store(enabled, Ordering::Relaxed); }

pub fn town_zsort_baking_enabled() -> bool { TOWN_ZSORT_BAKING.load(Ordering::Relaxed) }
pub fn set_town_zsort_baking(enabled: bool) { TOWN_ZSORT_BAKING.store(enabled, Ordering::Relaxed); }

pub fn shader_binary_precache_enabled() -> bool { SHADER_BINARY_PRECACHE.load(Ordering::Relaxed) }
pub fn set_shader_binary_precache(enabled: bool) { SHADER_BINARY_PRECACHE.store(enabled, Ordering::Relaxed); }

pub fn npc_tick_culling_enabled() -> bool { NPC_TICK_CULLING.load(Ordering::Relaxed) }
pub fn set_npc_tick_culling(enabled: bool) { NPC_TICK_CULLING.store(enabled, Ordering::Relaxed); }



