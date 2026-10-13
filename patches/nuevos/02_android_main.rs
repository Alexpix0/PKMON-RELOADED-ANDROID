//! Glue between the Android activity and ramen's Android platform backend.
//!
//! `android-activity` hands us an [`InputEvent`] enum; everything it reports is translated
//! into ramen's `InputEvent` queue so that `Window::poll_events()` can drain it. Only the
//! public wrapper API is used, so this doesn't chase ndk's FFI shape between versions.

use std::time::Duration;

use android_activity::{
    input::{InputEvent, KeyAction, MotionAction},
    AndroidApp, PollEvent,
};

use ramen::{input::Key, platform::android};

/// The path to the game executable, which the activity extracts from APK assets.
pub fn game_exe_path() -> Option<String> {
    std::env::var("OPENGMK_GAME_EXE").ok()
}

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

const LETTERS: [Key; 26] = [
    Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L,
    Key::M, Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X,
    Key::Y, Key::Z,
];

fn map_key(keycode: AKeyCode) -> Option<Key> {
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

/// `Keycode` is a repr(i32) enum, so the raw AKEYCODE_* value is recoverable.
fn key_code_value(keycode: &android_activity::input::Keycode) -> i32 {
    // Keycode's variants are AKEYCODE_*; its Debug form is "A(29)" style, so use its
    // discriminant through `Clone`-free transmute-free access below.
    unsafe { *std::ptr::from_ref(keycode).cast::<i32>().to_owned() }
}

/// Translates one `android-activity` input event into ramen's queue.
fn translate(event: &InputEvent) {
    match event {
        InputEvent::KeyEvent(key) => match key.action() {
            KeyAction::Down => {
                if let Some(k) = mapped_key(key_code_value(&key.key_code())) {
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
            let (x, y) = motion
                .pointers()
                .next()
                .map(|p| (p.x(), p.y()))
                .unwrap_or((0.0, 0.0));
            match motion.action() {
                MotionAction::Down
                | MotionAction::PointerDown
                | MotionAction::ButtonPress => {
                    android::push_input(ramen::platform::android::InputEvent::PointerDown {
                        x,
                        y,
                        button: 0,
                    });
                },
                MotionAction::Up | MotionAction::PointerUp | MotionAction::ButtonRelease => {
                    android::push_input(ramen::platform::android::InputEvent::PointerUp {
                        x,
                        y,
                        button: 0,
                    });
                },
                MotionAction::Move | MotionAction::HoverMove => {
                    android::push_input(ramen::platform::android::InputEvent::PointerMoved { x, y });
                },
                _ => {},
            }
        },
        _ => {},
    }
}

fn publish_window(app: &AndroidApp) {
    if let Some(window) = app.native_window() {
        let ptr = window.ptr().as_ptr() as *mut std::ffi::c_void;
        let width = window.width() as u32;
        let height = window.height() as u32;
        android::set_native_window(ptr, width, height);
    }
}

/// Runs the pump and then blocks in the emulator's own frame loop, which owns the main thread.
pub fn android_entry(app: AndroidApp) {
    publish_window(&app);

    // The activity is pumped on a worker thread so the emulator keeps the thread it wants;
    // `android_main` is already off the Java main thread, so nothing deadlocks.
    std::thread::spawn(move || loop {
        app.poll_events(Some(Duration::from_millis(8)), |event| match event {
            PollEvent::Main(main_event) => match main_event {
                android_activity::MainEvent::InitWindow { .. }
                | android_activity::MainEvent::WindowResized { .. }
                | android_activity::MainEvent::RedrawNeeded { .. } => {
                    publish_window(&app);
                    if let Some(window) = app.native_window() {
                        android::push_resize(window.width() as u16, window.height() as u16);
                    }
                },
                android_activity::MainEvent::InputAvailable => {
                    if let Ok(mut iter) = app.input_events_iter() {
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
                _ => {},
            },
            PollEvent::Wake => {},
            PollEvent::Timeout => {},
            _ => {},
        });
    });

    crate::run_game(vec![
        "gm8emulator".to_string(),
        game_exe_path().unwrap_or_else(|| "/data/local/tmp/game.exe".to_string()),
    ]);
}

/// android-activity provides `ANativeActivity_onCreate`; this is the plain-Rust-ABI symbol
/// it looks for and calls on a dedicated thread.
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_entry(app);
}
