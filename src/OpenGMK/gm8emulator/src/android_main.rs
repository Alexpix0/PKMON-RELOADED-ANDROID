//! Glue between the Android activity and ramen's Android platform backend.
//!
//! `android-activity` hands us an [`InputEvent`] enum; everything it reports is translated
//! into ramen's `InputEvent` queue so that `Window::poll_events()` can drain it. Only the
//! public wrapper API is used, so this doesn't chase ndk's FFI shape between versions.

use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::sync::Mutex;

use android_activity::{
    input::{Axis, InputEvent, KeyAction, MotionAction},
    AndroidApp, PollEvent,
};

use ramen::{input::Key, platform::android};

/// `AKEYCODE_*`, from <android/keycodes.h>. Letters are contiguous from `A`.
type AKeyCode = i32;
const AKEYCODE_A: AKeyCode = 29;
const AKEYCODE_DEL: AKeyCode = 67;
const AKEYCODE_TAB: AKeyCode = 61;
const AKEYCODE_ENTER: AKeyCode = 66;
const AKEYCODE_SPACE: AKeyCode = 62;
const AKEYCODE_DPAD_UP: AKeyCode = 19;
const AKEYCODE_DPAD_DOWN: AKeyCode = 20;
const AKEYCODE_DPAD_LEFT: AKeyCode = 21;
const AKEYCODE_DPAD_RIGHT: AKeyCode = 22;
const AKEYCODE_ESCAPE: AKeyCode = 111;

/// Gamepad buttons. Android reports the same keycodes for every pad, whatever the brand, so the
/// names below are the physical ones the player sees.
const AKEYCODE_BUTTON_A: AKeyCode = 96;
const AKEYCODE_BUTTON_B: AKeyCode = 97;
const AKEYCODE_BUTTON_X: AKeyCode = 99;
const AKEYCODE_BUTTON_Y: AKeyCode = 100;
const AKEYCODE_BUTTON_L1: AKeyCode = 102;
const AKEYCODE_BUTTON_R1: AKeyCode = 103;
const AKEYCODE_BUTTON_L2: AKeyCode = 104;
const AKEYCODE_BUTTON_R2: AKeyCode = 105;
const AKEYCODE_BUTTON_THUMBL: AKeyCode = 106;
const AKEYCODE_BUTTON_THUMBR: AKeyCode = 107;
const AKEYCODE_BUTTON_START: AKeyCode = 108;
const AKEYCODE_BUTTON_SELECT: AKeyCode = 109;

const LETTERS: [Key; 26] = [
    Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L,
    Key::M, Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X,
    Key::Y, Key::Z,
];

fn map_key(keycode: AKeyCode) -> Option<Key> {
    // A layout saved by the launcher's control screen wins over the built-in one; anything it
    // doesn't mention keeps the default below.
    if let Some(map) = std::sync::OnceLock::get(&MAPPED_BUTTONS) {
        if let Some(mapped) = map.lock().ok().and_then(|map| map.get(&keycode).copied()) {
            return Some(mapped)
        }
    }
    Some(match keycode {
        AKEYCODE_DEL => Key::Backspace,
        AKEYCODE_TAB => Key::Tab,
        AKEYCODE_ENTER => Key::Return,
        AKEYCODE_SPACE => Key::Space,
        AKEYCODE_ESCAPE => Key::Escape,
        AKEYCODE_DPAD_UP => Key::UpArrow,
        AKEYCODE_DPAD_DOWN => Key::DownArrow,
        AKEYCODE_DPAD_LEFT => Key::LeftArrow,
        AKEYCODE_DPAD_RIGHT => Key::RightArrow,

        // Gamepad. The game only ever reads the keyboard, so a controller plays by pressing the
        // same keys the keyboard does. That is the whole trick, and it needs nothing from the game:
        //
        //   D-pad                  -> arrow keys
        //   A (Xbox/Nintendo) / X (PS)      -> Z
        //   B (Xbox/Nintendo) / circle (PS) -> X
        //   X (Xbox/Nintendo) / square (PS) -> C
        //   Y (Xbox/Nintendo) / triangle    -> V
        //   select, share, view, minus      -> Enter
        //   start, options, menu, plus      -> Space
        //   R1 / RB / R                      -> B
        //   L1 / LB / L                      -> M
        //   R2 / RT / ZR                     -> O
        //   L2 / LT / ZL                     -> S
        //   stick press (either one)         -> E
        AKEYCODE_BUTTON_A => Key::Z,
        AKEYCODE_BUTTON_B => Key::X,
        AKEYCODE_BUTTON_X => Key::C,
        AKEYCODE_BUTTON_Y => Key::V,
        AKEYCODE_BUTTON_SELECT => Key::Return,
        AKEYCODE_BUTTON_START => Key::Space,
        AKEYCODE_BUTTON_R1 => Key::B,
        AKEYCODE_BUTTON_L1 => Key::M,
        AKEYCODE_BUTTON_R2 => Key::O,
        AKEYCODE_BUTTON_L2 => Key::S,
        AKEYCODE_BUTTON_THUMBL | AKEYCODE_BUTTON_THUMBR => Key::E,

        _ => return None,
    })
}

fn map_letter(keycode: AKeyCode) -> Option<Key> {
    let index = (keycode - AKEYCODE_A) as usize;
    LETTERS.get(index).copied()
}

fn mapped_key(keycode: i32) -> Option<Key> {
    map_key(keycode).or_else(|| map_letter(keycode))
}

/// The pad layout the launcher's control screen saved, keyed by Android key code. The contents are
/// replaced on every run rather than cached, because the engine and the launcher share one process
/// and the layout can change between one run of the game and the next.
static MAPPED_BUTTONS: std::sync::OnceLock<Mutex<HashMap<AKeyCode, Key>>> = std::sync::OnceLock::new();

/// Whether the physical sticks move the player.
static STICK_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Which arrow keys the physical stick is holding (bit 0 up, 1 down, 2 left, 3 right). Kept apart
/// from the on-screen joystick's state so neither can release a key the other is holding.
static STICK_STATE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Reads the video mode the launcher saved. Only one thing matters here: whether the picture keeps
/// the shape the game was drawn for, or stretches to fill the screen.
fn load_video() {
    let path = PathBuf::from(format!("/data/data/{}/files/video.conf", PACKAGE_DIR));
    if let Ok(text) = fs::read_to_string(&path) {
        for line in text.lines() {
            let Some((name, value)) = line.trim().split_once('=') else { continue };
            if name.trim() != "scaling" {
                continue
            }
            let value = value.trim();
            if value == "aspect" || value == "full" {
                eprintln!("[video] escalado: {}", value);
                // Read by the engine when it builds its renderer.
                std::env::set_var("OPENGMK_SCALING", value);
            }
        }
    }
}

/// Reads the layout the launcher's control screen left behind. Best effort: with no file, the
/// built-in layout is used exactly as before.
fn load_controls() {
    let path = PathBuf::from(format!("/data/data/{}/files/controls.conf", PACKAGE_DIR));
    let mut buttons = HashMap::new();
    if let Ok(text) = fs::read_to_string(&path) {
        eprintln!("[pad] leyendo {}", path.display());
        for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue
                }
                eprintln!("[pad]   {}", line);
            let Some((name, value)) = line.split_once('=') else { continue };
            if name.trim() == "stick" {
                STICK_ENABLED.store(value.trim() != "0", std::sync::atomic::Ordering::Relaxed);
            } else if name.trim() == "pad" {
                // A pad was connected when the game started, so start with the controls out of the
                // way; touching the screen brings them back.
                crate::touch_controls::set_pad_active(value.trim() != "0");
            } else if name.trim() == "opacity" {
                if let Ok(pct) = value.trim().parse::<u32>() {
                    crate::touch_controls::set_opacity(pct);
                }
            } else if name.trim() == "scale" {
                if let Ok(pct) = value.trim().parse::<u32>() {
                    crate::touch_controls::set_scale(pct);
                }
            } else if name.trim() == "margin_x" {
                if let Ok(pct) = value.trim().parse::<u32>() {
                    crate::touch_controls::set_margin_x(pct);
                }
            } else if name.trim() == "margin_y" {
                if let Ok(pct) = value.trim().parse::<u32>() {
                    crate::touch_controls::set_margin_y(pct);
                }
            } else if name.trim() == "haptics" {
                set_haptics_enabled(value.trim() != "0");
            } else if name.trim() == "deadzone" {
                if let Ok(pct) = value.trim().parse::<u32>() {
                    crate::touch_controls::set_joystick_dead(pct);
                }
            } else if name.trim() == "num_keys" {
                crate::touch_controls::set_num_keys_visible(value.trim() != "0");
            } else if let (Ok(code), Some(key)) = (name.trim().parse::<AKeyCode>(), parse_key(value.trim())) {
                buttons.insert(code, key);
            }
        }
    }
    // Replaced rather than cached: the launcher can change the layout while the app is still alive.
    if let Ok(mut map) = MAPPED_BUTTONS.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        *map = buttons;
    }
}

/// The key names the control screen can write into `controls.conf`.
fn parse_key(name: &str) -> Option<Key> {
    let upper = name.trim().to_ascii_uppercase();
    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        if byte.is_ascii_uppercase() {
            return LETTERS.get(usize::from(byte - b'A')).copied()
        }
    }
    Some(match upper.as_str() {
        "RETURN" | "ENTER" => Key::Return,
        "SPACE" => Key::Space,
        "ESCAPE" => Key::Escape,
        "TAB" => Key::Tab,
        "BACKSPACE" => Key::Backspace,
        "UP" => Key::UpArrow,
        "DOWN" => Key::DownArrow,
        "LEFT" => Key::LeftArrow,
        "RIGHT" => Key::RightArrow,
        _ => return None,
    })
}

/// Holds and releases the arrow keys the physical stick asks for, so pushing it feels exactly like
/// the on-screen joystick.
fn stick_update(x: f32, y: f32) {
    let dirs = crate::touch_controls::stick_dirs(x, y);
    let mut state = STICK_STATE.load(std::sync::atomic::Ordering::Relaxed);
    let before = state;
    for (index, held) in dirs.iter().enumerate() {
        let bit = 1u32 << index;
        let was = state & bit != 0;
        if *held && !was {
            jni_vibrate(12);
            android::push_input(ramen::platform::android::InputEvent::KeyDown { key: JOYSTICK_KEYS[index], text: None });
        } else if !*held && was {
            android::push_input(ramen::platform::android::InputEvent::KeyUp { key: JOYSTICK_KEYS[index] });
        }
        if *held {
            state |= bit;
        } else {
            state &= !bit;
        }
    }
    STICK_STATE.store(state, std::sync::atomic::Ordering::Relaxed);
    if state != before {
        // Only when the direction actually changes, so holding a direction does not flood logcat.
        eprintln!("[pad] ejes x={:.2} y={:.2} -> direcciones {}", x, y, state);
    }
}

/// The value of one axis of a motion event. Axis values belong to the event, and the crate reaches
/// them through its first pointer, so there has to be one.
fn axis_value(motion: &android_activity::input::MotionEvent, axis: Axis) -> f32 {
    if motion.pointer_count() == 0 {
        return 0.0
    }
    motion.pointer_at_index(0).axis_value(axis)
}

fn stick_release() {
    let state = STICK_STATE.swap(0, std::sync::atomic::Ordering::Relaxed);
    for (index, key) in JOYSTICK_KEYS.iter().enumerate() {
        if state & (1 << index) != 0 {
            android::push_input(ramen::platform::android::InputEvent::KeyUp { key: *key });
        }
    }
}

/// `Keycode` is a repr(i32) enum, so the raw AKEYCODE_* value is recoverable.
fn key_code_value(keycode: &android_activity::input::Keycode) -> i32 {
    // Keycode's variants are AKEYCODE_*; its Debug form is "A(29)" style, so use its
    // discriminant through `Clone`-free transmute-free access below.
    unsafe { *std::ptr::from_ref(keycode).cast::<i32>().to_owned() }
}

/// Which control each active pointer is currently holding, so a multi-touch release is paired with
/// the right control. Keyed by Android pointer id.
static POINTER_CONTROLS: Mutex<Option<HashMap<i32, usize>>> = Mutex::new(None);

static HAPTICS_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn set_haptics_enabled(enabled: bool) {
    HAPTICS_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

pub fn haptics_enabled() -> bool {
    HAPTICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

fn jni_vibrate(duration_ms: i64) {
    if !haptics_enabled() {
        return;
    }
    let ctx = ndk_context::android_context();
    let vm = match unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) } {
        Ok(vm) => vm,
        Err(_) => return,
    };
    let mut env = match vm.attach_current_thread() {
        Ok(env) => env,
        Err(_) => return,
    };
    let _ = (|| -> jni::errors::Result<()> {
        let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
        let name = env.new_string("vibrator")?;
        let vibrator = env
            .call_method(
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[jni::objects::JValue::Object(&name)],
            )?
            .l()?;
        if vibrator.is_null() {
            return Ok(())
        }
        env.call_method(
            &vibrator,
            "vibrate",
            "(J)V",
            &[jni::objects::JValue::Long(duration_ms)],
        )?;
        Ok(())
    })();
}

fn control_key_down(index: usize) {
    crate::touch_controls::set_pressed(index, true);
    jni_vibrate(18);
    android::push_input(ramen::platform::android::InputEvent::KeyDown {
        key: crate::touch_controls::CONTROLS[index].key,
        text: None,
    });
}

fn control_key_up(index: usize) {
    crate::touch_controls::set_pressed(index, false);
    android::push_input(ramen::platform::android::InputEvent::KeyUp { key: crate::touch_controls::CONTROLS[index].key });
}

/// Pointers whose press was swallowed to bring the on-screen controls back, so that their release
/// is ignored too instead of reaching the game as a stray mouse-up.
static SWALLOWED_INPUTS: Mutex<Option<std::collections::HashSet<i32>>> = Mutex::new(None);

/// A touch on an on-screen control presses the key that control maps to; any other touch is
/// forwarded to the game as a mouse, exactly as before.
fn pointer_down(id: i32, x: f32, y: f32) {
    // The first touch after using a pad only brings the controls back. It must not also land in the
    // game as a click, or reaching for the on-screen buttons would act on whatever is under them.
    if crate::touch_controls::pad_active() {
        crate::touch_controls::set_pad_active(false);
        SWALLOWED_INPUTS.lock().unwrap().get_or_insert_with(std::collections::HashSet::new).insert(id);
        return
    }
    let (w, h) = android::window_size();
    let (wf, hf) = (w as f32, h as f32);
    if wf > 0.0 && hf > 0.0 && crate::touch_controls::joystick_hit(x / wf, y / hf, wf, hf) {
        *JOYSTICK_PTR.lock().unwrap() = Some(id);
        joystick_update(x, y);
        return
    }
    let hit = if w > 0 && h > 0 { crate::touch_controls::hit(x / wf, y / hf) } else { None };
    match hit {
        Some(index) => {
            control_key_down(index);
            POINTER_CONTROLS.lock().unwrap().get_or_insert_with(HashMap::new).insert(id, index);
        },
        None => mouse_down(id, x, y),
    }
}

fn pointer_up(id: i32, x: f32, y: f32) {
    if SWALLOWED_INPUTS.lock().unwrap().get_or_insert_with(std::collections::HashSet::new).remove(&id) {
        return
    }
    if is_joystick_pointer(id) {
        joystick_release();
        return
    }
    let index = POINTER_CONTROLS.lock().unwrap().get_or_insert_with(HashMap::new).remove(&id);
    match index {
        Some(index) => control_key_up(index),
        None => mouse_up(id, x, y),
    }
}

/// A real tap lasts ~100 ms, but a quick one can deliver press and release inside a single poll.
/// The game reads the mouse once per frame, so such a tap is simply missed — that is why clicking
/// the game's menus with the finger felt random. Holding the release briefly makes taps behave
/// like real mouse clicks.
const MIN_TAP: Duration = Duration::from_millis(70);

/// The game decides which menu entry is under the cursor while it *moves*; a click only confirms
/// the current one. A finger sends move and press together, so the press used to land on whatever
/// was selected before. Delivering the move a hair earlier gives the game that frame.
const HOVER_DELAY: Duration = Duration::from_millis(45);

static MOUSE_DOWN_AT: Mutex<Vec<(i32, Instant)>> = Mutex::new(Vec::new());
static DELAYED_UPS: Mutex<Vec<(Instant, f32, f32)>> = Mutex::new(Vec::new());
static DELAYED_DOWNS: Mutex<Vec<(Instant, f32, f32)>> = Mutex::new(Vec::new());

fn mouse_down(id: i32, x: f32, y: f32) {
    let deliver_at = Instant::now() + HOVER_DELAY;
    let mut downs = MOUSE_DOWN_AT.lock().unwrap();
    downs.retain(|(pointer, _)| *pointer != id);
    downs.push((id, deliver_at));
    drop(downs);
    // The move goes out now; the press follows on a later poll so the hover is in place first.
    android::push_input(ramen::platform::android::InputEvent::PointerMoved { x, y });
    DELAYED_DOWNS.lock().unwrap().push((deliver_at, x, y));
}

fn mouse_up(id: i32, x: f32, y: f32) {
    let pressed_at = {
        let mut downs = MOUSE_DOWN_AT.lock().unwrap();
        let at = downs.iter().find(|(pointer, _)| *pointer == id).map(|(_, at)| *at);
        downs.retain(|(pointer, _)| *pointer != id);
        at
    };
    let deadline = pressed_at.map(|at| at + MIN_TAP).unwrap_or_else(Instant::now);
    if deadline > Instant::now() {
        DELAYED_UPS.lock().unwrap().push((deadline, x, y));
    } else {
        android::push_input(ramen::platform::android::InputEvent::PointerUp { x, y, button: 0 });
    }
}

/// Delivers the presses and releases that were held back to make a short tap a proper click.
fn flush_delayed_mouse() {
    let now = Instant::now();
    let mut downs: Vec<(f32, f32)> = Vec::new();
    DELAYED_DOWNS.lock().unwrap().retain(|(at, x, y)| {
        if *at <= now {
            downs.push((*x, *y));
            false
        } else {
            true
        }
    });
    for (x, y) in downs {
        android::push_input(ramen::platform::android::InputEvent::PointerDown { x, y, button: 0 });
    }

    let mut ups: Vec<(f32, f32)> = Vec::new();
    DELAYED_UPS.lock().unwrap().retain(|(at, x, y)| {
        if *at <= now {
            ups.push((*x, *y));
            false
        } else {
            true
        }
    });
    for (x, y) in ups {
        android::push_input(ramen::platform::android::InputEvent::PointerUp { x, y, button: 0 });
    }
}

fn pointer_is_control(id: i32) -> bool {
    POINTER_CONTROLS.lock().unwrap().as_ref().map(|m| m.contains_key(&id)).unwrap_or(false)
}

// --- Movement joystick -------------------------------------------------------------------

/// Pointer id currently driving the joystick, if any.
static JOYSTICK_PTR: Mutex<Option<i32>> = Mutex::new(None);
/// Which arrow keys the joystick is holding (bit 0 up, 1 down, 2 left, 3 right).
static JOYSTICK_STATE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

const JOYSTICK_KEYS: [Key; 4] = [Key::UpArrow, Key::DownArrow, Key::LeftArrow, Key::RightArrow];

fn is_joystick_pointer(id: i32) -> bool {
    *JOYSTICK_PTR.lock().unwrap() == Some(id)
}

/// Presses/releases the arrow keys so they match where the stick is being pushed.
fn joystick_update(x: f32, y: f32) {
    use std::sync::atomic::Ordering;
    let (w, h) = android::window_size();
    let (wf, hf) = (w as f32, h as f32);
    if wf <= 0.0 || hf <= 0.0 {
        return
    }
    let dirs = crate::touch_controls::joystick_dirs(x / wf, y / hf, wf, hf);
    let mut state = JOYSTICK_STATE.load(Ordering::Relaxed);
    for (i, &on) in dirs.iter().enumerate() {
        let bit = 1u32 << i;
        if on != (state & bit != 0) {
            if on {
                android::push_input(ramen::platform::android::InputEvent::KeyDown { key: JOYSTICK_KEYS[i], text: None });
                state |= bit;
            } else {
                android::push_input(ramen::platform::android::InputEvent::KeyUp { key: JOYSTICK_KEYS[i] });
                state &= !bit;
            }
        }
    }
    JOYSTICK_STATE.store(state, Ordering::Relaxed);

    // Clamp the drawn knob to the ring.
    let r = crate::touch_controls::JOYSTICK_R * hf;
    let mut dx = (x - crate::touch_controls::JOYSTICK_CX * wf) / r;
    let mut dy = (y - crate::touch_controls::JOYSTICK_CY * hf) / r;
    let len = (dx * dx + dy * dy).sqrt();
    if len > 1.0 {
        dx /= len;
        dy /= len;
    }
    crate::touch_controls::set_joystick_offset(dx, dy);
}

fn joystick_release() {
    use std::sync::atomic::Ordering;
    let state = JOYSTICK_STATE.swap(0, Ordering::Relaxed);
    for i in 0..4 {
        if state & (1 << i) != 0 {
            android::push_input(ramen::platform::android::InputEvent::KeyUp { key: JOYSTICK_KEYS[i] });
        }
    }
    crate::touch_controls::set_joystick_offset(0.0, 0.0);
    *JOYSTICK_PTR.lock().unwrap() = None;
}

/// The pointer the current action refers to, if it is valid.
fn changed_pointer(motion: &android_activity::input::MotionEvent) -> Option<(i32, f32, f32)> {
    let index = motion.pointer_index();
    if index >= motion.pointer_count() {
        return None
    }
    let p = motion.pointer_at_index(index);
    Some((p.pointer_id(), p.x(), p.y()))
}

/// Android key codes for the hardware volume rocker.
const KEYCODE_VOLUME_UP: i32 = 24;
const KEYCODE_VOLUME_DOWN: i32 = 25;

/// Translates one `android-activity` input event into ramen's queue.
fn translate(event: &InputEvent) {
    match event {
        InputEvent::KeyEvent(key) => match key.action() {
            KeyAction::Down => {
                let code = key_code_value(&key.key_code());
                if code == KEYCODE_VOLUME_UP || code == KEYCODE_VOLUME_DOWN {
                    // Volume keys arrive through the native queue, so Android never handles them.
                    jni_adjust_volume(code == KEYCODE_VOLUME_UP);
                    return
                }
                if let Some(k) = mapped_key(code) {
                    // A real key press means a pad or a keyboard is in the player's hands, so the
                    // on-screen controls get out of the way.
                    crate::touch_controls::set_pad_active(true);
                    eprintln!("[pad] tecla {} -> {:?}", code, k);
                    android::push_input(ramen::platform::android::InputEvent::KeyDown { key: k, text: None });
                }
            },
            KeyAction::Up => {
                if let Some(k) = mapped_key(key_code_value(&key.key_code())) {
                    android::push_input(ramen::platform::android::InputEvent::KeyUp { key: k });
                }
            },
            _ => {},
        },
        InputEvent::MotionEvent(motion) => {
            // A pad's stick arrives as a generic motion event, not as a key, so its axes have to be
            // read from the event itself rather than from a pointer.
            if motion.source().is_joystick_class() {
                crate::touch_controls::set_pad_active(true);
                if STICK_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                    let (mut x, mut y) = (axis_value(motion, Axis::X), axis_value(motion, Axis::Y));
                    // A d-pad is usually reported as a hat axis rather than as keys, and it has to
                    // win over the stick when it is pressed: a resting stick is not always at exactly
                    // zero, and requiring that is what kept the d-pad from doing anything.
                    let (hat_x, hat_y) = (axis_value(motion, Axis::HatX), axis_value(motion, Axis::HatY));
                    if hat_x != 0.0 || hat_y != 0.0 {
                        x = hat_x;
                        y = hat_y;
                    }
                    stick_update(x, y);
                }
                return
            }

            match motion.action() {
                MotionAction::Down | MotionAction::PointerDown => {
                    if let Some((id, x, y)) = changed_pointer(motion) {
                        pointer_down(id, x, y);
                    }
                },
                MotionAction::Up | MotionAction::PointerUp => {
                    if let Some((id, x, y)) = changed_pointer(motion) {
                        pointer_up(id, x, y);
                    }
                },
                MotionAction::Move | MotionAction::HoverMove => {
                    for p in motion.pointers() {
                        if is_joystick_pointer(p.pointer_id()) {
                            joystick_update(p.x(), p.y());
                        } else if !pointer_is_control(p.pointer_id()) {
                            android::push_input(ramen::platform::android::InputEvent::PointerMoved { x: p.x(), y: p.y() });
                        }
                    }
                },
                MotionAction::Cancel => {
                    joystick_release();
                    let ids: Vec<i32> =
                        POINTER_CONTROLS.lock().unwrap().as_ref().map(|m| m.keys().copied().collect()).unwrap_or_default();
                    for id in ids {
                        if let Some(index) = POINTER_CONTROLS.lock().unwrap().get_or_insert_with(HashMap::new).remove(&id) {
                            control_key_up(index);
                        }
                    }
                },
                _ => {},
            }
        },
        _ => {},
    }
}

/// A NativeActivity receives key events through the native queue, so the system never sees the
/// volume keys and the volume can't be changed. Adjust the music stream ourselves instead.
fn jni_adjust_volume(up: bool) {
    let ctx = ndk_context::android_context();
    let vm = match unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) } {
        Ok(vm) => vm,
        Err(_) => return,
    };
    let mut env = match vm.attach_current_thread() {
        Ok(env) => env,
        Err(_) => return,
    };
    let _ = (|| -> jni::errors::Result<()> {
        let activity = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
        let name = env.new_string("audio")?;
        let manager = env
            .call_method(
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[jni::objects::JValue::Object(&name)],
            )?
            .l()?;
        if manager.is_null() {
            return Ok(())
        }
        // STREAM_MUSIC = 3, ADJUST_RAISE = 1 / ADJUST_LOWER = -1, FLAG_SHOW_UI = 1
        let direction = if up { 1 } else { -1 };
        env.call_method(
            &manager,
            "adjustStreamVolume",
            "(III)V",
            &[
                jni::objects::JValue::Int(3),
                jni::objects::JValue::Int(direction),
                jni::objects::JValue::Int(1),
            ],
        )?;
        Ok(())
    })();
}

fn publish_window(app: &AndroidApp) {
    if let Some(window) = app.native_window() {
        let ptr = window.ptr().as_ptr() as *mut std::ffi::c_void;
        // The `NativeWindow` wrapper above owns a reference and drops it when this function
        // returns; without acquiring our own, the window can be freed before the game thread
        // calls eglCreateWindowSurface on it, which crashes with a null function pointer.
        // SAFETY: `ptr` is a valid ANativeWindow handed out by the activity, and this only
        // bumps its reference count.
        unsafe { ANativeWindow_acquire(ptr) };
        let width = window.width() as u32;
        let height = window.height() as u32;
        android::set_native_window(ptr, width, height);
    }
}

extern "C" {
    /// Bumps the reference count of an `ANativeWindow`. From `libandroid`, always linked.
    fn ANativeWindow_acquire(window: *mut std::ffi::c_void) -> i32;
}

/// Runs the emulator. `poll_events` must be driven from the thread `android_main` is called
/// on, because android-activity asserts it, so the pump cannot live on a worker thread.
/// The emulator itself runs on a worker thread instead.
pub fn android_entry(app: AndroidApp) {

    // The pad layout and the video mode are read once, before any input can arrive.
    load_controls();
    load_video();

    publish_window(&app);

    let app = std::sync::Arc::new(app);
    let game_app = app.clone();

    // Set when the emulator returns, i.e. the game ended from its own "Salir del juego" (or failed).
    // Returning from `android_main` makes android-activity finish the Activity, so the launcher
    // comes back instead of leaving a dead black window on screen.
    let game_over = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let game_over_thread = game_over.clone();

    std::thread::spawn(move || {
        let game_exe = match ensure_game_extracted() {
            Ok(exe) => exe,
            Err(e) => {
                logf(&format!("no se pudo preparar el juego: {}", e));
                game_over_thread.store(true, std::sync::atomic::Ordering::SeqCst);
                return
            },
        };

        crate::render::opengl::egl::log_stubbed_once();
        provision_soundfont(game_exe.parent().unwrap_or_else(|| std::path::Path::new("/")));
        logf(&format!("arrancando {}", game_exe.display()));
        crate::run_game(vec![
            "gm8emulator".to_string(),
            game_exe.to_string_lossy().into_owned(),
        ]);
        logf("el juego termino");
        game_over_thread.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    let mut destroying = false;
    loop {
        if game_over.load(std::sync::atomic::Ordering::SeqCst) || destroying {
            break
        }
        // When the game moves to another state, drop the keys of controls that are no longer shown
        // so none of them stays stuck down.
        for index in crate::touch_controls::stale_pressed() {
            control_key_up(index);
        }
        flush_delayed_mouse();
        game_app.poll_events(Some(Duration::from_millis(8)), |event| match event {
            PollEvent::Main(main_event) => match main_event {
                // Init/Resume/Start all mean the surface is (re)available, which is also what
                // happens when the user comes back from the home screen.
                android_activity::MainEvent::InitWindow { .. }
                | android_activity::MainEvent::WindowResized { .. }
                | android_activity::MainEvent::RedrawNeeded { .. }
                | android_activity::MainEvent::Resume { .. }
                | android_activity::MainEvent::GainedFocus
                | android_activity::MainEvent::Start => {
                    ramen::platform::android::set_paused(false);
                    crate::game::audio::set_output_paused(false);
                    publish_window(&game_app);
                    if let Some(window) = game_app.native_window() {
                        // The activity has given us a window: build the GL surface for it now, while
                        // it is alive. Building it later, from the render loop, is what left the
                        // screen black and made EGL complain about a window already connected.
                        unsafe {
                            let ptr = window.ptr().as_ptr() as *mut std::ffi::c_void;
                            crate::render::opengl::egl::attach_surface(ptr);
                        }
                        android::push_resize(window.width() as u16, window.height() as u16);
                    }
                },
                // Going away. The surface is let go on `Pause`/`Stop` as well as on
                // `TerminateWindow`, because Android does not always send the latter - and a dead
                // surface left lying around is worse than none: the renderer believes it can draw,
                // and OpenGL answers with an error that kills the thread.
                android_activity::MainEvent::TerminateWindow { .. }
                | android_activity::MainEvent::Pause { .. }
                | android_activity::MainEvent::Stop => {
                    ramen::platform::android::set_paused(true);
                    crate::game::audio::set_output_paused(true);
                    unsafe { crate::render::opengl::egl::detach_surface() };
                },
                android_activity::MainEvent::InputAvailable => {
                    if let Ok(mut iter) = game_app.input_events_iter() {
                        loop {
                            let handled = iter.next(|event| {
                                translate(&event);
                                android_activity::InputStatus::Handled
                            });
                            if !handled {
                                break
                            }
                        }
                    }
                },
                // The system is tearing the activity down (the app was closed): stop pumping so
                // `android_main` can return and the activity really goes away.
                android_activity::MainEvent::Destroy => destroying = true,
                _ => {},
            },
            PollEvent::Wake => {},
            PollEvent::Timeout => {},
            _ => {},
        });
    }
}

/// android-activity provides `ANativeActivity_onCreate`; this is the plain-Rust-ABI symbol
/// it looks for and calls on a dedicated thread.
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_entry(app);
}

// ---------------------------------------------------------------------------------------
// Asset provisioning
//
// The game ships as a .zip laid out exactly like the game folder it comes from, so
// extracting it reproduces the layout the engine already expects on Linux. It is found
// in a list of conventional places, extracted once into the app's own storage, and then
// the game runs straight off the extracted files.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// Where the app looks for `game.zip`, most specific first. The first two are the app's own
/// storage and need no permissions; the last one is shared storage and may need one.
const ZIP_NAME: &str = "game.zip";
const PACKAGE_DIR: &str = "com.klisee.reloaded";

/// Free bytes on the filesystem containing `path`, or `None` if it can't be queried.
fn free_bytes_on(path: &Path) -> Option<u64> {
    // SAFETY: zeroed statvfs is a valid argument, and the call only reads.
    unsafe {
        let path_c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
        let mut stats: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(path_c.as_ptr(), &mut stats) != 0 {
            return None
        }
        Some(stats.f_bavail * u64::from(stats.f_frsize))
    }
}

/// Enough room for the zip plus its extracted contents.
fn enough_space(zip_len: u64, target: &Path) -> bool {
    match free_bytes_on(target.parent().unwrap_or(target)) {
        Some(free) => free > zip_len * 2,
        None => true, // can't tell, so let the extraction try and fail loudly if it must
    }
}

fn zip_candidates() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from(format!("/sdcard/Android/data/{}/files", PACKAGE_DIR)),
        PathBuf::from(format!("/data/data/{}/files", PACKAGE_DIR)),
        PathBuf::from("/sdcard/Download".to_string()),
        PathBuf::from("/sdcard".to_string()),
        // Writable by `adb push`, which is how the game is usually sideloaded for testing.
        PathBuf::from("/data/local/tmp".to_string()),
    ];
    dirs.retain(|d| d.is_dir());
    dirs.into_iter().map(|d| d.join(ZIP_NAME)).filter(|p| p.is_file()).collect()
}

/// The `.exe` inside the extracted game directory. Its name is not pinned, so a different
/// version of the game, or another GameMaker 8 fangame, works unchanged.
fn find_exe(dir: &Path) -> Option<PathBuf> {
    let mut found = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        })
        .collect::<Vec<_>>();
    found.sort();
    found.into_iter().next()
}

fn logf(msg: &str) {
    // `eprintln` goes to logcat under NativeActivity, which is where this is read from.
    eprintln!("[assets] {}", msg);
    // Keep a small file too, so the launcher can show a log without needing adb/logcat.
    append_log_file(msg);
}

/// Appends one line to `<files>/emulator.log`, rotating once past [LOG_MAX_BYTES].
fn append_log_file(msg: &str) {
    const LOG_MAX_BYTES: u64 = 256 * 1024;
    let path = PathBuf::from(format!("/data/data/{}/files/emulator.log", PACKAGE_DIR));
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = (|| -> std::io::Result<()> {
        use std::io::Write;
        if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > LOG_MAX_BYTES {
            let _ = fs::rename(&path, path.with_extension("log.1"));
        }
        // O_APPEND means concurrent writers interleave whole lines rather than corrupting them.
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(file, "{}", msg)
    })();
}

/// MIDI music needs a General MIDI SoundFont, and Android ships none at the paths the engine looks
/// at. If one was placed next to the game (or pushed to the app's storage), point the engine at it.
fn provision_soundfont(game_dir: &Path) {
    let candidates = [
        game_dir.join("soundfont.sf2"),
        game_dir.join("TimGM6mb.sf2"),
        PathBuf::from(format!("/sdcard/Android/data/{}/files/soundfont.sf2", PACKAGE_DIR)),
        PathBuf::from("/sdcard/Download/soundfont.sf2"),
    ];
    for path in candidates {
        if path.is_file() {
            logf(&format!("soundfont MIDI: {}", path.display()));
            std::env::set_var("OPENGMK_SOUNDFONT", &path);
            return
        }
    }
    logf("aviso: no hay .sf2; la musica MIDI no sonara (copia soundfont.sf2 a la carpeta del juego)");
}

/// Makes the game available on disk and returns the path of its executable.
fn ensure_game_extracted() -> Result<PathBuf, String> {
    let storage = PathBuf::from(format!("/data/data/{}/files", PACKAGE_DIR));
    let game_dir = storage.join("game");
    fs::create_dir_all(&game_dir).map_err(|e| format!("no se pudo crear {}: {}", game_dir.display(), e))?;

    // Already provisioned? Both the exe and the data folder have to be there.
    if let Some(exe) = find_exe(&game_dir) {
        if game_dir.join("data").is_dir() {
            logf(&format!("juego ya extraido en {}", game_dir.display()));
            return Ok(exe)
        }
    }

    let zips = zip_candidates();
    if zips.is_empty() {
        return Err(format!(
            "no encontre {}. Copialo a /sdcard/Android/data/{}/files/",
            ZIP_NAME, PACKAGE_DIR
        ))
    }
    let zip_path = &zips[0];
    let zip_len = fs::metadata(zip_path).map_err(|e| format!("{}: {}", zip_path.display(), e))?.len();
    logf(&format!("encontrado {} ({} bytes)", zip_path.display(), zip_len));

    if !enough_space(zip_len, &game_dir) {
        return Err("espacio insuficiente: hacen falta unas dos veces el tamano del zip".into())
    }

    // Extract into a staging dir and swap it in, so a failed run leaves the previous
    // contents intact rather than a half-extracted game.
    let staging = storage.join("game.staging");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| format!("no se pudo crear {}: {}", staging.display(), e))?;

    {
        let file = fs::File::open(zip_path).map_err(|e| format!("{}: {}", zip_path.display(), e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("no pude leer el zip {}: {}", zip_path.display(), e))?;
        logf(&format!("extrayendo {} entradas", archive.len()));
        archive
            .extract(&staging)
            .map_err(|e| format!("fallo la extraccion de {}: {}", zip_path.display(), e))?;
    }

    let exe = find_exe(&staging).ok_or_else(|| "el zip no contiene ningun .exe en su raiz".to_string())?;
    let _ = fs::remove_dir_all(&game_dir);
    fs::rename(&staging, &game_dir)
        .map_err(|e| format!("no pude mover lo extraido a {}: {}", game_dir.display(), e))?;

    let exe = game_dir.join(exe.file_name().unwrap());
    logf(&format!("listo: {}", exe.display()));
    Ok(exe)
}
