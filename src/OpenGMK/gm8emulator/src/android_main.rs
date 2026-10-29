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

pub static SHUTDOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static WORKER_HANDLE: std::sync::Mutex<Option<std::thread::JoinHandle<()>>> = std::sync::Mutex::new(None);

/// `AKEYCODE_*`, from <android/keycodes.h>. Letters are contiguous from `A`.
type AKeyCode = i32;
const AKEYCODE_A: AKeyCode = 29;
const AKEYCODE_Z: AKeyCode = 54;
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

        // Numbers 0..9 on physical PC keyboard
        7 => Key::Alpha0,
        8 => Key::Alpha1,
        9 => Key::Alpha2,
        10 => Key::Alpha3,
        11 => Key::Alpha4,
        12 => Key::Alpha5,
        13 => Key::Alpha6,
        14 => Key::Alpha7,
        15 => Key::Alpha8,
        16 => Key::Alpha9,

        // Modifiers on physical PC keyboard
        57 => Key::LeftAlt,
        58 => Key::RightAlt,
        59 => Key::LeftShift,
        60 => Key::RightShift,
        113 => Key::LeftControl,
        114 => Key::RightControl,

        // Function keys F1..F12
        131 => Key::F1,
        132 => Key::F2,
        133 => Key::F3,
        134 => Key::F4,
        135 => Key::F5,
        136 => Key::F6,
        137 => Key::F7,
        138 => Key::F8,
        139 => Key::F9,
        140 => Key::F10,
        141 => Key::F11,
        142 => Key::F12,

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
        AKEYCODE_BUTTON_THUMBL => Key::Q,
        AKEYCODE_BUTTON_THUMBR => Key::E,

        _ => return None,
    })
}

fn map_letter(keycode: AKeyCode) -> Option<Key> {
    if (AKEYCODE_A..=AKEYCODE_Z).contains(&keycode) {
        let index = (keycode - AKEYCODE_A) as usize;
        LETTERS.get(index).copied()
    } else {
        None
    }
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

/// Analog trigger states with hysteresis
static L2_TRIGGER_HELD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static R2_TRIGGER_HELD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static PAD_SELECT_HELD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Reads the video mode the launcher saved. Only one thing matters here: whether the picture keeps
/// the shape the game was drawn for, or stretches to fill the screen.
fn load_video() {
    let path = PathBuf::from(format!("/data/data/{}/files/video.conf", PACKAGE_DIR));
    if let Ok(text) = fs::read_to_string(&path) {
        for line in text.lines() {
            let Some((name, value)) = line.trim().split_once('=') else { continue };
            let name = name.trim();
            let value = value.trim();
            if name == "scaling" {
                if value == "aspect" || value == "full" || value == "pixel_perfect" || value == "integer" {
                    eprintln!("[video] escalado: {}", value);
                    // Read by the engine when it builds its renderer.
                    std::env::set_var("OPENGMK_SCALING", value);
                }
            } else if name == "render" {
                // Named modes plus the numeric caps the Settings screen writes (60/120/...).
                let numeric = value.parse::<u32>().map(|hz| hz > 0).unwrap_or(false);
                if matches!(value, "native" | "auto" | "uncapped") || numeric {
                    eprintln!("[video] render: {}", value);
                    std::env::set_var("OPENGMK_RENDER", value);
                }
            } else if name == "detected_hz" {
                if let Ok(hz) = value.parse::<u32>() {
                    if hz > 0 {
                        eprintln!("[video] pantalla: {} Hz", hz);
                        std::env::set_var("OPENGMK_RENDER_HZ", hz.to_string());
                    }
                }
            } else if name == "show_fps" {
                let show = value == "1" || value.eq_ignore_ascii_case("true");
                eprintln!("[video] show_fps: {}", show);
                std::env::set_var("OPENGMK_SHOW_FPS", if show { "1" } else { "0" });
                crate::touch_controls::set_show_fps(show);
            } else if name == "bezel" {
                eprintln!("[video] marco bezel: {}", value);
                std::env::set_var("OPENGMK_BEZEL", value);
            } else if name == "turbo_speed" {
                if let Ok(spd) = value.parse::<f32>() {
                    eprintln!("[video] turbo_speed: {}", spd);
                    crate::touch_controls::set_fast_forward_speed((spd * 10.0).round() as u32);
                }
            } else if name == "frame_skip" {
                eprintln!("[video] frame_skip: {}", value);
                std::env::set_var("OPENGMK_FRAME_SKIP", value);
            } else if name == "render_scale" {
                eprintln!("[video] render_scale: {}", value);
                if let Ok(scale) = value.parse::<f32>() {
                    crate::touch_controls::set_render_scale(scale);
                }
                std::env::set_var("OPENGMK_RENDER_SCALE", value);
            } else if name == "filter" {
                eprintln!("[video] filter: {}", value);
                std::env::set_var("OPENGMK_FILTER", value);
            } else if name == "weather_opt" {
                eprintln!("[video] weather_opt: {}", value);
                std::env::set_var("OPENGMK_WEATHER_OPT", value);
            } else if name == "backend" {
                eprintln!("[video] backend: {}", value);
                let vulkan = value.eq_ignore_ascii_case("vulkan");
                crate::touch_controls::set_vulkan_backend(vulkan);
                std::env::set_var("OPENGMK_BACKEND", value);
            } else if name == "bgm_volume" {
                if let Ok(vol) = value.parse::<u32>() {
                    eprintln!("[video] bgm_volume: {}", vol);
                    crate::touch_controls::set_bgm_volume(vol);
                }
            } else if name == "sfx_volume" {
                if let Ok(vol) = value.parse::<u32>() {
                    eprintln!("[video] sfx_volume: {}", vol);
                    crate::touch_controls::set_sfx_volume(vol);
                }
            } else if name == "autofire" {
                let af = value == "1" || value.eq_ignore_ascii_case("true");
                eprintln!("[video] autofire: {}", af);
                crate::touch_controls::set_autofire_enabled(af);
            } else if name == "dynamic_dpad" {
                let dd = value == "1" || value.eq_ignore_ascii_case("true");
                eprintln!("[video] dynamic_dpad: {}", dd);
                crate::touch_controls::set_dynamic_dpad_enabled(dd);
            } else if name == "berry_tracker" {
                crate::touch_controls::set_berry_tracker(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "egg_hatch_timer" {
                crate::touch_controls::set_egg_hatch_timer(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "key_items_radial" {
                crate::touch_controls::set_key_items_radial(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "auto_run" {
                crate::touch_controls::set_auto_run(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "dialog_tap_advance" {
                crate::touch_controls::set_dialog_tap_advance(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "repel_reminder" {
                crate::touch_controls::set_repel_reminder(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "shiny_beacon" {
                crate::touch_controls::set_shiny_beacon(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "puzzle_pause" {
                crate::touch_controls::set_puzzle_pause(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "fishing_haptic" {
                crate::touch_controls::set_fishing_haptic(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "quick_save" {
                crate::touch_controls::set_quick_save(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "save_reminder" {
                crate::touch_controls::set_save_reminder(value == "1" || value.eq_ignore_ascii_case("true"));
            } else if name == "extreme_combat" {
                let ec = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_extreme_combat(ec);
                if ec {
                    crate::render::fsr::set_drs_enabled(false);
                    crate::render::fsr::set_fsr_mode(0);
                }
            } else if name == "mali_earlyz" {
                let me = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_mali_earlyz(me);
            } else if name == "gpu_wakeup" {
                let gw = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_gpu_wakeup(gw);
            } else if name == "smart_preroll_skip" {
                let sp = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_smart_preroll_skip(sp);
            } else if name == "adaptive_ltpo" {
                let al = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_adaptive_ltpo(al);
            } else if name == "pip_mode" {
                let pm = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_pip_mode(pm);
            } else if name == "live_activity" {
                let la = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_live_activity(la);
            } else if name == "active_static_partition" {
                let asp = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_active_static_partition(asp);
            } else if name == "mega_atlas" {
                let ma = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_mega_atlas(ma);
            } else if name == "fsr_mode" {
                if let Ok(m) = value.parse::<u32>() {
                    crate::render::fsr::set_fsr_mode(m);
                    if m > 0 {
                        crate::touch_controls::set_extreme_combat(false);
                        crate::render::fsr::set_drs_enabled(false);
                    }
                }
            } else if name == "drs_enabled" {
                let drs = value == "1" || value.eq_ignore_ascii_case("true");
                crate::render::fsr::set_drs_enabled(drs);
                if drs {
                    crate::touch_controls::set_extreme_combat(false);
                    crate::render::fsr::set_fsr_mode(0);
                }
            } else if name == "xbrz_enabled" {
                let xbrz = value == "1" || value.eq_ignore_ascii_case("true");
                crate::render::fsr::set_xbrz_enabled(xbrz);
            } else if name == "lighting2d" {
                let l2d = value == "1" || value.eq_ignore_ascii_case("true");
                crate::render::lighting2d::set_lighting2d_enabled(l2d);
            } else if name == "audio_3d" {
                let a3d = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::audio_3d::set_audio_3d_enabled(a3d);
            } else if name == "reactive_stems" {
                let rs = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::audio_3d::set_reactive_stems_enabled(rs);
            } else if name == "wsola" {
                let ws = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::audio_3d::set_wsola_enabled(ws);
            } else if name == "colorblind_mode" {
                if let Ok(cbm) = value.parse::<u32>() {
                    crate::render::colorblind::set_colorblind_mode(cbm);
                }
            } else if name == "radial_menu" {
                let rm = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::radial_menu::set_radial_menu_enabled(rm);
            } else if name == "gestures" {
                let gst = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::gestures::set_gestures_enabled(gst);
            } else if name == "floating_dpad" {
                let fd = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::gestures::set_floating_dpad_enabled(fd);
            } else if name == "one_handed" {
                let oh = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::gestures::set_one_handed_layout_enabled(oh);
            } else if name == "replay_blackbox" {
                let rb = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::replay_blackbox::set_replay_blackbox_enabled(rb);
            } else if name == "achievements" {
                let ach = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::achievements::set_achievements_enabled(ach);
            } else if name == "big_little_affinity" {
                let bla = value == "1" || value.eq_ignore_ascii_case("true");
                crate::game::big_little::set_big_little_affinity_enabled(bla);
            } else if name == "async_render_thread" {
                let val = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_async_render_thread(val);
            } else if name == "shader_binary_precache" {
                let val = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_shader_binary_precache(val);
            } else if name == "npc_tick_culling" {
                let val = value == "1" || value.eq_ignore_ascii_case("true");
                crate::touch_controls::set_npc_tick_culling(val);
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
            } else if name.trim() == "show_num_keys" {
                crate::touch_controls::set_show_num_keys(value.trim() != "0");
            } else if name.trim() == "autofire" {
                crate::touch_controls::set_autofire_enabled(value.trim() != "0");
            } else if name.trim() == "dynamic_dpad" {
                crate::touch_controls::set_dynamic_dpad_enabled(value.trim() != "0");
            } else if let (Ok(code), Some(key)) = (name.trim().parse::<AKeyCode>(), parse_key(value.trim())) {
                buttons.insert(code, key);
            }
        }
    }
    // Replaced rather than cached: the launcher can change the layout while the app is still alive.
    if let Ok(mut map) = MAPPED_BUTTONS.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        *map = buttons;
    }
    let layout_path = PathBuf::from(format!("/data/data/{}/files/layout.conf", PACKAGE_DIR));
    crate::touch_controls::load_layout_file(&layout_path);
    let touch_layout_path = PathBuf::from(format!("/data/data/{}/files/touch-layout.conf", PACKAGE_DIR));
    if let Ok(text) = std::fs::read_to_string(&touch_layout_path) {
        crate::touch_controls::load_layout(&text);
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
        if byte.is_ascii_digit() {
            return match byte {
                b'0' => Some(Key::Alpha0),
                b'1' => Some(Key::Alpha1),
                b'2' => Some(Key::Alpha2),
                b'3' => Some(Key::Alpha3),
                b'4' => Some(Key::Alpha4),
                b'5' => Some(Key::Alpha5),
                b'6' => Some(Key::Alpha6),
                b'7' => Some(Key::Alpha7),
                b'8' => Some(Key::Alpha8),
                b'9' => Some(Key::Alpha9),
                _ => None,
            };
        }
    }
    Some(match upper.as_str() {
        "RETURN" | "ENTER" => Key::Return,
        "SPACE" => Key::Space,
        "ESCAPE" | "ESC" => Key::Escape,
        "TAB" => Key::Tab,
        "BACKSPACE" => Key::Backspace,
        "UP" => Key::UpArrow,
        "DOWN" => Key::DownArrow,
        "LEFT" => Key::LeftArrow,
        "RIGHT" => Key::RightArrow,
        "SHIFT" | "LSHIFT" | "LEFTSHIFT" => Key::LeftShift,
        "RSHIFT" | "RIGHTSHIFT" => Key::RightShift,
        "CTRL" | "LCTRL" | "LEFTCONTROL" => Key::LeftControl,
        "RCTRL" | "RIGHTCONTROL" => Key::RightControl,
        "ALT" | "LALT" | "LEFTALT" => Key::LeftAlt,
        "RALT" | "RIGHTALT" => Key::RightAlt,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
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
    let is_moving = state != 0;
    if is_moving {
        crate::touch_controls::tick_movement_step(true);
    }
    if crate::touch_controls::auto_run_enabled() {
        static STICK_AUTO_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if is_moving && !STICK_AUTO_RUN.swap(true, std::sync::atomic::Ordering::Relaxed) {
            android::push_input(ramen::platform::android::InputEvent::KeyDown { key: Key::E, text: None });
        } else if !is_moving && STICK_AUTO_RUN.swap(false, std::sync::atomic::Ordering::Relaxed) {
            android::push_input(ramen::platform::android::InputEvent::KeyUp { key: Key::E });
        }
    }
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
    if crate::touch_controls::auto_run_enabled() {
        android::push_input(ramen::platform::android::InputEvent::KeyUp { key: Key::E });
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

/// The pointer the party wheel is following, if any.
static WHEEL_PTR: Mutex<Option<i32>> = Mutex::new(None);

#[derive(Clone, Copy, PartialEq, Debug)]
enum SliderTarget {
    Scale,
    Opacity,
}
static EDIT_SLIDER_DRAG: Mutex<Option<(SliderTarget, i32)>> = Mutex::new(None);

/// Sends a key as a quick tap. The engine defers the release to the end of the poll, so the game
/// still sees it as pressed.
fn tap_key(key: ramen::input::Key) {
    android::push_input(ramen::platform::android::InputEvent::KeyDown { key, text: None });
    android::push_input(ramen::platform::android::InputEvent::KeyUp { key });
}

pub fn jni_vibrate(duration_ms: i64) {
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

static CURSOR_POS: Mutex<(f32, f32)> = Mutex::new((400.0, 300.0));

fn current_cursor_pos() -> (f32, f32) {
    let (w, h) = android::window_size();
    let mut pos = CURSOR_POS.lock().unwrap_or_else(|e| e.into_inner());
    if pos.0 <= 0.0 || pos.1 <= 0.0 {
        *pos = (w as f32 * 0.5, h as f32 * 0.5);
    }
    *pos
}

fn update_cursor_delta(dx: f32, dy: f32) {
    let (w, h) = android::window_size();
    let (wf, hf) = (w as f32, h as f32);
    if wf <= 0.0 || hf <= 0.0 { return; }
    let mut pos = CURSOR_POS.lock().unwrap_or_else(|e| e.into_inner());
    pos.0 = (pos.0 + dx).clamp(0.0, wf);
    pos.1 = (pos.1 + dy).clamp(0.0, hf);
    android::push_input(ramen::platform::android::InputEvent::PointerMoved { x: pos.0, y: pos.1 });
}

fn control_key_down(index: usize) {
    crate::touch_controls::set_pressed(index, true);
    let name = crate::touch_controls::control_at(index).name;
    // Mouse button emulation (LMB = button 0, RMB = button 2)
    if name == "lmb" {
        let (x, y) = current_cursor_pos();
        android::push_input(ramen::platform::android::InputEvent::PointerDown { x, y, button: 0 });
        jni_vibrate(18);
        return;
    } else if name == "rmb" {
        let (x, y) = current_cursor_pos();
        android::push_input(ramen::platform::android::InputEvent::PointerDown { x, y, button: 2 });
        jni_vibrate(18);
        return;
    }

    // The trick combination needs both keys held at once, so it is not one key like the rest.
    if crate::touch_controls::control_at(index).name == crate::touch_controls::COMBO_BUTTON_NAME {
        for key in [Key::LeftControl, Key::P] {
            android::push_input(ramen::platform::android::InputEvent::KeyDown { key, text: None });
        }
        // It has done its job: it takes itself away until the next new game.
        crate::touch_controls::set_combo_used();
        return
    }
    if crate::touch_controls::is_autofire_enabled() && (name == "z" || name == "x" || name == "c" || name == "v" || name == "dialogo") {
        crate::touch_controls::register_autofire(index);
    }
    let key = crate::touch_controls::control_key(index);
    // On the character picker the overlay's arrows travel with the game's own selection, so the
    // buttons stay around the character the player is on rather than around the first one.
    match key {
        Key::LeftArrow => crate::touch_controls::picker_step(-1),
        Key::RightArrow => crate::touch_controls::picker_step(1),
        _ => {},
    }
    if crate::touch_controls::fishing_haptic_enabled() && (name == "v" || name == "c" || name == "z") {
        jni_vibrate(28);
    } else {
        jni_vibrate(18);
    }
    android::push_input(ramen::platform::android::InputEvent::KeyDown {
        key,
        text: None,
    });
}

fn control_key_up(index: usize) {
    crate::touch_controls::set_pressed(index, false);
    crate::touch_controls::unregister_autofire(index);
    let name = crate::touch_controls::control_at(index).name;
    if name == "lmb" {
        let (x, y) = current_cursor_pos();
        android::push_input(ramen::platform::android::InputEvent::PointerUp { x, y, button: 0 });
        return;
    } else if name == "rmb" {
        let (x, y) = current_cursor_pos();
        android::push_input(ramen::platform::android::InputEvent::PointerUp { x, y, button: 2 });
        return;
    }
    if crate::touch_controls::control_at(index).name == crate::touch_controls::COMBO_BUTTON_NAME {
        for key in [Key::LeftControl, Key::P] {
            android::push_input(ramen::platform::android::InputEvent::KeyUp { key });
        }
        return
    }
    android::push_input(ramen::platform::android::InputEvent::KeyUp { key: crate::touch_controls::control_key(index) });
}

/// Pointers whose press was swallowed to bring the on-screen controls back, so that their release
/// is ignored too instead of reaching the game as a stray mouse-up.
static SWALLOWED_INPUTS: Mutex<Option<std::collections::HashSet<i32>>> = Mutex::new(None);
static DRAGGED_ITEM: Mutex<Option<(String, f32, f32)>> = Mutex::new(None);

fn perform_quick_snapshot() {
    let internal_saves = std::path::Path::new("/data/data/com.klisee.reloaded/files/game/data/partidas");
    let relative_saves = std::path::Path::new("data/partidas");
    let src_dir = if internal_saves.is_dir() {
        internal_saves
    } else if relative_saves.is_dir() {
        relative_saves
    } else {
        crate::touch_controls::show_toast("ERROR: NO SE HALLARON PARTIDAS");
        return;
    };

    let snapshot_dir = std::path::PathBuf::from("/data/data/com.klisee.reloaded/files/snapshots");
    let _ = std::fs::create_dir_all(&snapshot_dir);

    let mut copied = 0;
    if let Ok(entries) = std::fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("sav") || ext.eq_ignore_ascii_case("nfo") {
                    if let Some(file_name) = path.file_name() {
                        let dest = snapshot_dir.join(format!("quick_return_{}", file_name.to_string_lossy()));
                        if std::fs::copy(&path, &dest).is_ok() {
                            copied += 1;
                        }
                    }
                }
            }
        }
    }
    if copied > 0 {
        crate::touch_controls::show_toast("CAPTURA LIMPIA GUARDADA");
    } else {
        crate::touch_controls::show_toast("CAPTURA COMPLETADA");
    }
}

fn perform_quick_save() {
    // Instantaneous in-memory RAM snapshot (<1.5 ms)
    crate::game::savestate::request_ram_save();

    let internal_saves = std::path::Path::new("/data/data/com.klisee.reloaded/files/game/data/partidas");
    let relative_saves = std::path::Path::new("data/partidas");
    let src_dir = if internal_saves.is_dir() {
        internal_saves
    } else if relative_saves.is_dir() {
        relative_saves
    } else {
        crate::touch_controls::show_toast("ERROR: NO SE HALLARON PARTIDAS");
        return;
    };

    let quicksave_dir = std::path::PathBuf::from("/data/data/com.klisee.reloaded/files/quicksaves");
    let _ = std::fs::create_dir_all(&quicksave_dir);

    let mut copied = 0;
    if let Ok(entries) = std::fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("sav") || ext.eq_ignore_ascii_case("nfo") {
                    if let Some(file_name) = path.file_name() {
                        let dest = quicksave_dir.join(file_name);
                        if std::fs::copy(&path, &dest).is_ok() {
                            copied += 1;
                        }
                    }
                }
            }
        }
    }
    // Also dispatch F5 (standard GM quicksave key)
    android::push_input(ramen::platform::android::InputEvent::KeyDown { key: Key::F5, text: None });
    android::push_input(ramen::platform::android::InputEvent::KeyUp { key: Key::F5 });

    if copied > 0 {
        crate::touch_controls::show_toast("QUICK SAVE: GUARDADO (1-TOQUE)");
    } else {
        crate::touch_controls::show_toast("GUARDANDO EN MEMORIA...");
    }
}

fn perform_quick_load() {
    // Instantaneous in-memory RAM snapshot restore (<2.0 ms)
    crate::game::savestate::request_ram_load();

    let internal_saves = std::path::Path::new("/data/data/com.klisee.reloaded/files/game/data/partidas");
    let relative_saves = std::path::Path::new("data/partidas");
    let dst_dir = if internal_saves.is_dir() {
        internal_saves
    } else if relative_saves.is_dir() {
        relative_saves
    } else {
        crate::touch_controls::show_toast("ERROR: NO SE HALLO CARPETA PARTIDAS");
        return;
    };

    let quicksave_dir = std::path::PathBuf::from("/data/data/com.klisee.reloaded/files/quicksaves");
    let mut restored = 0;
    if let Ok(entries) = std::fs::read_dir(&quicksave_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("sav") || ext.eq_ignore_ascii_case("nfo") {
                    if let Some(file_name) = path.file_name() {
                        let dest = dst_dir.join(file_name);
                        if std::fs::copy(&path, &dest).is_ok() {
                            restored += 1;
                        }
                    }
                }
            }
        }
    }
    // Also dispatch F6 (standard GM quickload key)
    android::push_input(ramen::platform::android::InputEvent::KeyDown { key: Key::F6, text: None });
    android::push_input(ramen::platform::android::InputEvent::KeyUp { key: Key::F6 });

    if restored > 0 {
        crate::touch_controls::show_toast("QUICK LOAD: PARTIDA RESTAURADA");
    } else {
        crate::touch_controls::show_toast("NO HAY QUICK SAVE PREVIO");
    }
}

pub fn apply_user_profile(p: u8) {
    crate::touch_controls::set_active_user_profile(p);
    let p_num = p + 1;
    let specific_path = std::path::PathBuf::from(format!("/data/data/{}/files/touch-layout-p{}.conf", PACKAGE_DIR, p_num));
    let base_path = std::path::PathBuf::from(format!("/data/data/{}/files/touch-layout.conf", PACKAGE_DIR));
    let path = if specific_path.exists() { specific_path } else { base_path };
    if let Ok(text) = std::fs::read_to_string(&path) {
        crate::touch_controls::load_layout(&text);
    }
}

/// A touch on an on-screen control presses the key that control maps to; any other touch is
/// forwarded to the game as a mouse, exactly as before.
fn pointer_down(id: i32, x: f32, y: f32) {
    crate::touch_controls::report_user_interaction();
    // The first touch after using a pad only brings the controls back. It must not also land in the
    // game as a click, or reaching for the on-screen buttons would act on whatever is under them.
    if crate::touch_controls::pad_active() {
        crate::touch_controls::set_pad_active(false);
        SWALLOWED_INPUTS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(std::collections::HashSet::new).insert(id);
        return
    }
    let (w, h) = android::window_size();
    let (wf, hf) = (w as f32, h as f32);

    // If the in-game type chart guide or dialogue log is open, any screen tap dismisses it
    if crate::touch_controls::is_guide_open() || crate::touch_controls::is_log_open() {
        crate::touch_controls::set_guide_open(false);
        crate::touch_controls::set_log_open(false);
        jni_vibrate(12);
        return;
    }

    // If Drastic-style Radial Quick-Menu is open, intercept touch for menu wheel actions
    if crate::touch_controls::radial_menu_open() {
        let action = crate::touch_controls::radial_menu_hit(x, y, wf, hf);
        match action {
            crate::touch_controls::RadialAction::Turbo => {
                let lvl = crate::touch_controls::cycle_turbo();
                let txt = match lvl {
                    0 => "TURBO: 1X (NORMAL)",
                    1 => "TURBO: 2X (RAPIDO)",
                    2 => "TURBO: 4X (EXTREMO)",
                    _ => "TURBO: 1X",
                };
                crate::touch_controls::show_toast(txt);
                jni_vibrate(25);
            },
            crate::touch_controls::RadialAction::QuickSave => {
                crate::touch_controls::set_radial_menu_open(false);
                perform_quick_save();
                jni_vibrate(35);
            },
            crate::touch_controls::RadialAction::QuickLoad => {
                crate::touch_controls::set_radial_menu_open(false);
                perform_quick_load();
                jni_vibrate(35);
            },
            crate::touch_controls::RadialAction::TypeChart => {
                crate::touch_controls::set_radial_menu_open(false);
                crate::touch_controls::toggle_guide_modal();
                jni_vibrate(25);
            },
            crate::touch_controls::RadialAction::CycleProfile => {
                let next = crate::touch_controls::cycle_user_profile();
                apply_user_profile(next);
                crate::touch_controls::show_toast(&format!("PERFIL CONTROLES: PERFIL {}", next + 1));
                jni_vibrate(25);
            },
            crate::touch_controls::RadialAction::Mute => {
                let muted = crate::touch_controls::toggle_audio_mute();
                crate::touch_controls::show_toast(if muted { "AUDIO: SILENCIADO" } else { "AUDIO: ACTIVADO" });
                jni_vibrate(if muted { 25 } else { 12 });
            },
            crate::touch_controls::RadialAction::Fps => {
                let on = crate::touch_controls::toggle_fps();
                crate::touch_controls::show_toast(if on { "FPS: MOSTRAR" } else { "FPS: OCULTAR" });
                jni_vibrate(20);
            },
            crate::touch_controls::RadialAction::Snapshot => {
                crate::touch_controls::set_radial_menu_open(false);
                perform_quick_snapshot();
                jni_vibrate(35);
            },
            crate::touch_controls::RadialAction::Close | crate::touch_controls::RadialAction::Outside => {
                crate::touch_controls::set_radial_menu_open(false);
                jni_vibrate(15);
            },
            crate::touch_controls::RadialAction::None => {},
        }
        return;
    }

    // If edit mode is active, handle toolbar actions (Save, Cancel, Sliders) or drag buttons
    if crate::touch_controls::is_edit_mode() {
        if wf > 0.0 && hf > 0.0 {
            let tb_action = crate::touch_controls::edit_toolbar_hit(x, y, wf, hf);
            match tb_action {
                crate::touch_controls::EditToolbarAction::Save => {
                    let layout_path = PathBuf::from(format!("/data/data/{}/files/layout.conf", PACKAGE_DIR));
                    crate::touch_controls::save_layout_file(&layout_path);
                    crate::touch_controls::save_controls_config(PACKAGE_DIR);
                    *DRAGGED_ITEM.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    *EDIT_SLIDER_DRAG.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    SWALLOWED_INPUTS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(std::collections::HashSet::new).insert(id);
                    crate::touch_controls::set_edit_mode(false);
                    for i in 0..crate::touch_controls::CONTROLS.len() {
                        control_key_up(i);
                    }
                    joystick_release();
                    crate::touch_controls::show_toast("¡CONTROLES GUARDADOS!");
                    jni_vibrate(28);
                    return;
                },
                crate::touch_controls::EditToolbarAction::Cancel => {
                    crate::touch_controls::restore_layout_snapshot();
                    *DRAGGED_ITEM.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    *EDIT_SLIDER_DRAG.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    SWALLOWED_INPUTS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(std::collections::HashSet::new).insert(id);
                    crate::touch_controls::set_edit_mode(false);
                    for i in 0..crate::touch_controls::CONTROLS.len() {
                        control_key_up(i);
                    }
                    joystick_release();
                    crate::touch_controls::show_toast("EDICION CANCELADA");
                    jni_vibrate(18);
                    return;
                },
                crate::touch_controls::EditToolbarAction::ScaleDec => {
                    let cur = crate::touch_controls::scale_pct();
                    crate::touch_controls::set_scale(cur.saturating_sub(5));
                    jni_vibrate(10);
                    return;
                },
                crate::touch_controls::EditToolbarAction::ScaleInc => {
                    let cur = crate::touch_controls::scale_pct();
                    crate::touch_controls::set_scale(cur + 5);
                    jni_vibrate(10);
                    return;
                },
                crate::touch_controls::EditToolbarAction::ScaleSet(pct) => {
                    crate::touch_controls::set_scale(pct);
                    *EDIT_SLIDER_DRAG.lock().unwrap_or_else(|e| e.into_inner()) = Some((SliderTarget::Scale, id));
                    jni_vibrate(8);
                    return;
                },
                crate::touch_controls::EditToolbarAction::OpacityDec => {
                    let cur = crate::touch_controls::opacity_pct();
                    crate::touch_controls::set_opacity(cur.saturating_sub(5));
                    jni_vibrate(10);
                    return;
                },
                crate::touch_controls::EditToolbarAction::OpacityInc => {
                    let cur = crate::touch_controls::opacity_pct();
                    crate::touch_controls::set_opacity(cur + 5);
                    jni_vibrate(10);
                    return;
                },
                crate::touch_controls::EditToolbarAction::OpacitySet(pct) => {
                    crate::touch_controls::set_opacity(pct);
                    *EDIT_SLIDER_DRAG.lock().unwrap_or_else(|e| e.into_inner()) = Some((SliderTarget::Opacity, id));
                    jni_vibrate(8);
                    return;
                },
                crate::touch_controls::EditToolbarAction::None => {},
            }

            let hit = crate::touch_controls::hit(x / wf, y / hf, wf, hf);
            if let Some(index) = hit {
                let name = crate::touch_controls::control_at(index).name;
                *DRAGGED_ITEM.lock().unwrap_or_else(|e| e.into_inner()) = Some((name.to_string(), x / wf, y / hf));
                jni_vibrate(12);
                return;
            }
            if crate::touch_controls::joystick_hit(x / wf, y / hf, wf, hf) {
                *DRAGGED_ITEM.lock().unwrap_or_else(|e| e.into_inner()) = Some((crate::touch_controls::JOYSTICK_NAME.to_string(), x / wf, y / hf));
                jni_vibrate(12);
                return;
            }
        }
        return;
    }

    if crate::touch_controls::is_puzzle_paused() {
        crate::touch_controls::set_puzzle_paused(false);
        crate::touch_controls::show_toast("▶️ JUEGO REANUDADO");
        jni_vibrate(15);
        return;
    }

    let hit = if w > 0 && h > 0 { crate::touch_controls::hit(x / wf, y / hf, wf, hf) } else { None };
    if hit.is_none() && wf > 0.0 && hf > 0.0 && crate::touch_controls::joystick_active() {
        if crate::touch_controls::is_dynamic_dpad_enabled() {
            let nx = x / wf;
            let ny = y / hf;
            if nx < 0.45 && ny > 0.20 && ny < 0.95 {
                crate::touch_controls::set_dynamic_anchor(Some((nx, ny)));
                *JOYSTICK_PTR.lock().unwrap_or_else(|e| e.into_inner()) = Some(id);
                joystick_update(x, y);
                jni_vibrate(12);
                return;
            }
        } else if crate::touch_controls::joystick_hit(x / wf, y / hf, wf, hf) {
            *JOYSTICK_PTR.lock().unwrap_or_else(|e| e.into_inner()) = Some(id);
            joystick_update(x, y);
            return;
        }
    }
    match hit {
        Some(index) => {
            let name = crate::touch_controls::control_at(index).name;
            if name == crate::touch_controls::PARTY_NAME {
                // Holding it opens the party wheel instead of pressing a key of its own.
                crate::touch_controls::set_wheel_open(true);
                crate::touch_controls::set_wheel_touch_origin(x / wf, y / hf);
                *WHEEL_PTR.lock().unwrap_or_else(|e| e.into_inner()) = Some(id);
                jni_vibrate(18);
            } else if name == crate::touch_controls::RADIAL_MENU_NAME {
                let open = crate::touch_controls::toggle_radial_menu();
                jni_vibrate(if open { 25 } else { 15 });
            } else if name == crate::touch_controls::TURBO_NAME {
                let on = crate::touch_controls::toggle_fast_forward();
                let mult = crate::touch_controls::fast_forward_multiplier();
                if on {
                    crate::touch_controls::show_toast(&format!("TURBO: {:.1}x ACTIVADO", mult));
                } else {
                    crate::touch_controls::show_toast("TURBO: DESACTIVADO");
                }
                jni_vibrate(if on { 25 } else { 12 });
            } else if name == crate::touch_controls::MUTE_NAME {
                let muted = crate::touch_controls::toggle_audio_mute();
                jni_vibrate(if muted { 25 } else { 12 });
            } else if name == crate::touch_controls::EDIT_NAME {
                let edit = crate::touch_controls::toggle_edit_mode();
                if edit {
                    for i in 0..crate::touch_controls::controls().len() {
                        control_key_up(i);
                    }
                    joystick_release();
                    *DRAGGED_ITEM.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    *EDIT_SLIDER_DRAG.lock().unwrap_or_else(|e| e.into_inner()) = None;
                }
                jni_vibrate(if edit { 30 } else { 15 });
            } else if name == crate::touch_controls::GUIDE_NAME {
                let open = crate::touch_controls::toggle_guide_modal();
                jni_vibrate(if open { 25 } else { 12 });
            } else if name == crate::touch_controls::LOG_NAME {
                let open = crate::touch_controls::toggle_log_modal();
                jni_vibrate(if open { 25 } else { 12 });
            } else if name == crate::touch_controls::ROTATE_NAME {
                queue_rotate_at_last_game_touch();
                SWALLOWED_INPUTS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(std::collections::HashSet::new).insert(id);
                jni_vibrate(15);
                return;
            } else {
                control_key_down(index);
                POINTER_CONTROLS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(id, index);
            }
        },
        None => {
            if crate::touch_controls::is_keyboard_wait_active() {
                // Escape key fallback on tap to avoid getting trapped in keyboard_wait
                jni_vibrate(20);
                tap_key(Key::Escape);
                return;
            }
            if crate::touch_controls::text_entry_open() {
                ime_set(true);
            }
            if crate::touch_controls::dialog_tap_advance_enabled()
                && (crate::touch_controls::current_context() & crate::touch_controls::DIALOG != 0
                    || crate::touch_controls::dialogue_open())
            {
                jni_vibrate(12);
                tap_key(Key::Z);
                return;
            }
            mouse_down(id, x, y)
        },
    }
}

fn pointer_up(id: i32, x: f32, y: f32) {
    if crate::touch_controls::is_edit_mode() {
        if let Some((_, drag_id)) = *EDIT_SLIDER_DRAG.lock().unwrap_or_else(|e| e.into_inner()) {
            if drag_id == id {
                *EDIT_SLIDER_DRAG.lock().unwrap_or_else(|e| e.into_inner()) = None;
            }
        }
        if DRAGGED_ITEM.lock().unwrap_or_else(|e| e.into_inner()).take().is_some() {
            let layout_path = PathBuf::from(format!("/data/data/{}/files/layout.conf", PACKAGE_DIR));
            crate::touch_controls::save_layout_file(&layout_path);
            jni_vibrate(15);
        }
        return;
    }
    if WHEEL_PTR.lock().unwrap_or_else(|e| e.into_inner()).take() == Some(id) {
        // The key only goes out now, on the release, and only if a slot was picked. Letting go over
        // the middle of the wheel, or letting go without moving at all, sends nothing.
        let selected = crate::touch_controls::wheel_selected();
        if selected >= 0 {
            tap_key(crate::touch_controls::PICKER_KEYS[selected as usize]);
            jni_vibrate(22);
        }
        crate::touch_controls::clear_wheel_touch_origin();
        crate::touch_controls::set_wheel_open(false);
        return
    }
    if SWALLOWED_INPUTS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(std::collections::HashSet::new).remove(&id) {
        return
    }
    if is_joystick_pointer(id) {
        joystick_release();
        return
    }
    let index = POINTER_CONTROLS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).remove(&id);
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

/// Last place the finger touched the game itself, not a button.
static LAST_GAME_TOUCH: Mutex<Option<(f32, f32)>> = Mutex::new(None);
const ROTATE_DELAY: Duration = Duration::from_millis(350);
static PENDING_ROTATE: Mutex<Option<(Instant, f32, f32)>> = Mutex::new(None);

fn right_click_at(x: f32, y: f32) {
    use ramen::platform::android::InputEvent;
    android::push_input(InputEvent::PointerMoved { x, y });
    android::push_input(InputEvent::PointerDown { x, y, button: 2 });
    android::push_input(InputEvent::PointerUp { x, y, button: 2 });
}

fn queue_rotate_at_last_game_touch() {
    if let Some((x, y)) = *LAST_GAME_TOUCH.lock().unwrap_or_else(|e| e.into_inner()) {
        *PENDING_ROTATE.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now() + ROTATE_DELAY, x, y));
        eprintln!("[puzzle] girar encolado en {},{}", x, y);
    }
}

fn flush_pending_rotate() {
    let mut pending = PENDING_ROTATE.lock().unwrap_or_else(|e| e.into_inner());
    let Some((at, x, y)) = *pending else { return };
    if Instant::now() < at {
        return;
    }
    *pending = None;
    eprintln!("[puzzle] girar enviado en {},{}", x, y);
    right_click_at(x, y);
}

pub fn show_puzzle_hint() {
    with_activity(|env, class| {
        env.call_static_method(class, "showTouchHint", "()V", &[])?;
        Ok(())
    });
}

fn mouse_down(id: i32, x: f32, y: f32) {
    *LAST_GAME_TOUCH.lock().unwrap_or_else(|e| e.into_inner()) = Some((x, y));
    let deliver_at = Instant::now() + HOVER_DELAY;
    let mut downs = MOUSE_DOWN_AT.lock().unwrap_or_else(|e| e.into_inner());
    downs.retain(|(pointer, _)| *pointer != id);
    downs.push((id, deliver_at));
    drop(downs);
    // The move goes out now; the press follows on a later poll so the hover is in place first.
    android::push_input(ramen::platform::android::InputEvent::PointerMoved { x, y });
    DELAYED_DOWNS.lock().unwrap_or_else(|e| e.into_inner()).push((deliver_at, x, y));
}

fn mouse_up(id: i32, x: f32, y: f32) {
    let pressed_at = {
        let mut downs = MOUSE_DOWN_AT.lock().unwrap_or_else(|e| e.into_inner());
        let at = downs.iter().find(|(pointer, _)| *pointer == id).map(|(_, at)| *at);
        downs.retain(|(pointer, _)| *pointer != id);
        at
    };
    let deadline = pressed_at.map(|at| at + MIN_TAP).unwrap_or_else(Instant::now);
    if deadline > Instant::now() {
        DELAYED_UPS.lock().unwrap_or_else(|e| e.into_inner()).push((deadline, x, y));
    } else {
        android::push_input(ramen::platform::android::InputEvent::PointerUp { x, y, button: 0 });
    }
}

/// Delivers the presses and releases that were held back to make a short tap a proper click.
fn flush_delayed_mouse() {
    let now = Instant::now();
    let mut downs: Vec<(f32, f32)> = Vec::new();
    DELAYED_DOWNS.lock().unwrap_or_else(|e| e.into_inner()).retain(|(at, x, y)| {
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
    DELAYED_UPS.lock().unwrap_or_else(|e| e.into_inner()).retain(|(at, x, y)| {
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
    POINTER_CONTROLS.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|m| m.contains_key(&id)).unwrap_or(false)
}

// --- Movement joystick -------------------------------------------------------------------

/// Pointer id currently driving the joystick, if any.
static JOYSTICK_PTR: Mutex<Option<i32>> = Mutex::new(None);
/// Which arrow keys the joystick is holding (bit 0 up, 1 down, 2 left, 3 right).
static JOYSTICK_STATE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

const JOYSTICK_KEYS: [Key; 4] = [Key::UpArrow, Key::DownArrow, Key::LeftArrow, Key::RightArrow];

fn is_joystick_pointer(id: i32) -> bool {
    *JOYSTICK_PTR.lock().unwrap_or_else(|e| e.into_inner()) == Some(id)
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
    if crate::touch_controls::auto_run_enabled() {
        let is_moving = state != 0;
        static JOY_AUTO_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if is_moving && !JOY_AUTO_RUN.swap(true, Ordering::Relaxed) {
            android::push_input(ramen::platform::android::InputEvent::KeyDown { key: Key::E, text: None });
        } else if !is_moving && JOY_AUTO_RUN.swap(false, Ordering::Relaxed) {
            android::push_input(ramen::platform::android::InputEvent::KeyUp { key: Key::E });
        }
    }

    // Clamp the drawn knob to the ring.
    let (jcx, jcy) = crate::touch_controls::joystick_center();
    let r = crate::touch_controls::joystick_radius() * hf;
    let mut dx = (x - jcx * wf) / r;
    let mut dy = (y - jcy * hf) / r;
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
    if crate::touch_controls::auto_run_enabled() {
        android::push_input(ramen::platform::android::InputEvent::KeyUp { key: Key::E });
    }
    crate::touch_controls::set_joystick_offset(0.0, 0.0);
    crate::touch_controls::set_dynamic_anchor(None);
    *JOYSTICK_PTR.lock().unwrap_or_else(|e| e.into_inner()) = None;
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
                if code == AKEYCODE_BUTTON_SELECT {
                    PAD_SELECT_HELD.store(true, std::sync::atomic::Ordering::Relaxed);
                } else if PAD_SELECT_HELD.load(std::sync::atomic::Ordering::Relaxed) {
                    if code == AKEYCODE_BUTTON_R1 {
                        crate::touch_controls::set_pad_active(true);
                        perform_quick_save();
                        return;
                    } else if code == AKEYCODE_BUTTON_L1 {
                        crate::touch_controls::set_pad_active(true);
                        perform_quick_load();
                        return;
                    }
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
                let code = key_code_value(&key.key_code());
                if code == AKEYCODE_BUTTON_SELECT {
                    PAD_SELECT_HELD.store(false, std::sync::atomic::Ordering::Relaxed);
                }
                if let Some(k) = mapped_key(code) {
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

                // --- Analog Triggers with Hysteresis (L2 / R2) ---
                let l2_raw = axis_value(motion, Axis::Ltrigger).max(axis_value(motion, Axis::Brake));
                let r2_raw = axis_value(motion, Axis::Rtrigger).max(axis_value(motion, Axis::Gas));

                let l2_was = L2_TRIGGER_HELD.load(std::sync::atomic::Ordering::Relaxed);
                if !l2_was && l2_raw > 0.55 {
                    L2_TRIGGER_HELD.store(true, std::sync::atomic::Ordering::Relaxed);
                    if let Some(k) = mapped_key(AKEYCODE_BUTTON_L2) {
                        android::push_input(ramen::platform::android::InputEvent::KeyDown { key: k, text: None });
                    }
                } else if l2_was && l2_raw < 0.40 {
                    L2_TRIGGER_HELD.store(false, std::sync::atomic::Ordering::Relaxed);
                    if let Some(k) = mapped_key(AKEYCODE_BUTTON_L2) {
                        android::push_input(ramen::platform::android::InputEvent::KeyUp { key: k });
                    }
                }

                let r2_was = R2_TRIGGER_HELD.load(std::sync::atomic::Ordering::Relaxed);
                if !r2_was && r2_raw > 0.55 {
                    R2_TRIGGER_HELD.store(true, std::sync::atomic::Ordering::Relaxed);
                    if let Some(k) = mapped_key(AKEYCODE_BUTTON_R2) {
                        android::push_input(ramen::platform::android::InputEvent::KeyDown { key: k, text: None });
                    }
                } else if r2_was && r2_raw < 0.40 {
                    R2_TRIGGER_HELD.store(false, std::sync::atomic::Ordering::Relaxed);
                    if let Some(k) = mapped_key(AKEYCODE_BUTTON_R2) {
                        android::push_input(ramen::platform::android::InputEvent::KeyUp { key: k });
                    }
                }

                if STICK_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                    let (mut x, mut y) = (axis_value(motion, Axis::X), axis_value(motion, Axis::Y));
                    let (z, rz) = (axis_value(motion, Axis::Z), axis_value(motion, Axis::Rz));

                    // Virtual Mouse in PC / Box mode: stick directly drives (x,y) pointer coordinates
                    if crate::touch_controls::box_open() || crate::touch_controls::current_context() == crate::touch_controls::MENU {
                        const CURSOR_SPEED: f32 = 14.0;
                        let dead = 0.15;
                        let mx = if x.abs() > dead { x } else if z.abs() > dead { z } else { 0.0 };
                        let my = if y.abs() > dead { y } else if rz.abs() > dead { rz } else { 0.0 };
                        if mx != 0.0 || my != 0.0 {
                            update_cursor_delta(mx * CURSOR_SPEED, my * CURSOR_SPEED);
                        }
                    }

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
                    if crate::touch_controls::is_edit_mode() {
                        let (w, h) = android::window_size();
                        let (wf, hf) = (w as f32, h as f32);
                        if wf > 0.0 && hf > 0.0 {
                            let slider_drag = *EDIT_SLIDER_DRAG.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some((target, drag_id)) = slider_drag {
                                for p in motion.pointers() {
                                    if p.pointer_id() == drag_id {
                                        let layout = crate::touch_controls::edit_toolbar_layout(wf, hf);
                                        match target {
                                            SliderTarget::Scale => {
                                                let (st1, st2) = layout.scale_track;
                                                let ratio = ((p.x() - st1) / (st2 - st1)).clamp(0.0, 1.0);
                                                let pct = (60.0 + ratio * 90.0).round() as u32;
                                                crate::touch_controls::set_scale(pct);
                                            },
                                            SliderTarget::Opacity => {
                                                let (ot1, ot2) = layout.opac_track;
                                                let ratio = ((p.x() - ot1) / (ot2 - ot1)).clamp(0.0, 1.0);
                                                let pct = (20.0 + ratio * 80.0).round() as u32;
                                                crate::touch_controls::set_opacity(pct);
                                            },
                                        }
                                        break;
                                    }
                                }
                                return;
                            }

                            let mut item_lock = DRAGGED_ITEM.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some((name, last_x, last_y)) = item_lock.clone() {
                                for p in motion.pointers() {
                                    let cur_x = p.x() / wf;
                                    let cur_y = p.y() / hf;
                                    let dx = cur_x - last_x;
                                    let dy = cur_y - last_y;
                                    let (ox, oy) = crate::touch_controls::control_offset(&name);
                                    crate::touch_controls::set_control_offset(&name, ox + dx, oy + dy);
                                    *item_lock = Some((name, cur_x, cur_y));
                                    break;
                                }
                            }
                        }
                        return;
                    }
                    let wheel_ptr = *WHEEL_PTR.lock().unwrap_or_else(|e| e.into_inner());
                    for p in motion.pointers() {
                        if wheel_ptr == Some(p.pointer_id()) {
                            let (w, h) = android::window_size();
                            let (wf, hf) = (w as f32, h as f32);
                            if wf > 0.0 && hf > 0.0 {
                                // Only highlight while the finger moves. The key waits for the
                                // release: in battle it sends the Pokémon out, so it must never go
                                // out on a touch that was not meant as a pick.
                                let picked = crate::touch_controls::wheel_item_at(p.x() / wf, p.y() / hf, wf, hf);
                                let prev = crate::touch_controls::wheel_selected();
                                let next = picked.map(|i| i as i32).unwrap_or(-1);
                                if prev != next && next >= 0 {
                                    jni_vibrate(12);
                                }
                                crate::touch_controls::set_wheel_selected(next);
                            }
                        } else if is_joystick_pointer(p.pointer_id()) {
                            joystick_update(p.x(), p.y());
                        } else if !pointer_is_control(p.pointer_id()) {
                            android::push_input(ramen::platform::android::InputEvent::PointerMoved { x: p.x(), y: p.y() });
                        }
                    }
                },
                MotionAction::Cancel => {
                    if WHEEL_PTR.lock().unwrap_or_else(|e| e.into_inner()).take().is_some() {
                        crate::touch_controls::clear_wheel_touch_origin();
                        crate::touch_controls::set_wheel_open(false);
                    }
                    joystick_release();
                    let ids: Vec<i32> =
                        POINTER_CONTROLS.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|m| m.keys().copied().collect()).unwrap_or_default();
                    for id in ids {
                        if let Some(index) = POINTER_CONTROLS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).remove(&id) {
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

static PUBLISHED_WINDOW: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

fn publish_window(app: &AndroidApp) {
    let Some(window) = app.native_window() else { return };
    let ptr = window.ptr().as_ptr() as *mut std::ffi::c_void;
    let width = window.width() as u32;
    let height = window.height() as u32;
    let previous = PUBLISHED_WINDOW.swap(ptr, std::sync::atomic::Ordering::SeqCst);
    // Acquire first, then tell the renderer, and only then drop the old one: at no point is the
    // window the renderer has been told about already freed.
    if previous != ptr {
        unsafe { ANativeWindow_acquire(ptr) };
    }
    android::set_native_window(ptr, width, height);
    if !previous.is_null() && previous != ptr {
        unsafe { ANativeWindow_release(previous) };
    }
}

extern "C" {
    /// Bumps the reference count of an `ANativeWindow`. From `libandroid`, always linked.
    fn ANativeWindow_acquire(window: *mut std::ffi::c_void) -> i32;
    /// Drops a reference taken with [ANativeWindow_acquire].
    fn ANativeWindow_release(window: *mut std::ffi::c_void);
}

/// Android can recreate the Activity inside the same process. Carrying the previous session's
/// input state over would leave keys held down (the player walking on their own, a wheel stuck...).
fn reset_input_state() {
    if let Ok(mut value) = POINTER_CONTROLS.lock() { *value = None; }
    if let Ok(mut value) = WHEEL_PTR.lock() { *value = None; }
    if let Ok(mut value) = SWALLOWED_INPUTS.lock() { *value = None; }
    if let Ok(mut value) = MOUSE_DOWN_AT.lock() { value.clear(); }
    if let Ok(mut value) = DELAYED_UPS.lock() { value.clear(); }
    if let Ok(mut value) = DELAYED_DOWNS.lock() { value.clear(); }
    if let Ok(mut value) = JOYSTICK_PTR.lock() { *value = None; }
    JOYSTICK_STATE.store(0, std::sync::atomic::Ordering::Relaxed);
    L2_TRIGGER_HELD.store(false, std::sync::atomic::Ordering::Relaxed);
    R2_TRIGGER_HELD.store(false, std::sync::atomic::Ordering::Relaxed);
    IME_SHOWN.store(false, std::sync::atomic::Ordering::Relaxed);
    crate::touch_controls::clear_layout();
    crate::touch_controls::clear_wheel_touch_origin();
    crate::touch_controls::set_wheel_open(false);
    crate::touch_controls::set_guide_open(false);
    crate::touch_controls::set_log_open(false);
}

// ---------------------------------------------------------------------------------------------
// System (soft) keyboard.
//
// The game has no Android view of its own to type into, so the activity's `InputConnection` sends
// what is typed here through the JNI exports below. `ime_pump` then feeds it to ramen the same way
// a hardware keyboard would, right before each poll, so the game's dialog sees a normal key tap.

/// What the system keyboard committed, waiting for the next poll.
enum ImeEvent {
    Text(String),
    Backspace,
    Enter,
}

fn ime_queue() -> &'static Mutex<std::collections::VecDeque<ImeEvent>> {
    static QUEUE: std::sync::OnceLock<Mutex<std::collections::VecDeque<ImeEvent>>> = std::sync::OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(std::collections::VecDeque::new()))
}

/// Feeds what the system keyboard typed into ramen's queue. Each character goes in as a quick key
/// tap that carries its text (`KeyDown{text}` + `KeyUp`): the press sets `keyboard_lastkey` and
/// `vk_anykey`, the text fills `keyboard_string`, and the deferred release keeps the key down for
/// the poll it arrived in - exactly what `tap_key` does for the on-screen buttons. Backspace and
/// enter are plain taps of the keys the game already reads for those.
pub fn ime_pump() {
    let events: Vec<ImeEvent> = match ime_queue().lock() {
        Ok(mut queue) => queue.drain(..).collect(),
        Err(_) => return,
    };
    for event in events {
        match event {
            ImeEvent::Text(text) => {
                for c in text.chars() {
                    let key = ime_char_to_key(c);
                    android::push_input(ramen::platform::android::InputEvent::KeyDown { key, text: Some(c) });
                    android::push_input(ramen::platform::android::InputEvent::KeyUp { key });
                }
            },
            ImeEvent::Backspace => tap_key(Key::Backspace),
            ImeEvent::Enter => tap_key(Key::Return),
        }
    }
}

/// The key a committed character is reported as. It only decides `keyboard_lastkey`; the character
/// itself travels in the event's text. Letters and digits become the same keys a hardware keyboard
/// would send; anything without a Game Maker key (accents, most signs) falls back to space, whose
/// code no dialog key uses.
fn ime_char_to_key(c: char) -> Key {
    if c.is_ascii_alphabetic() {
        return LETTERS[usize::from(c.to_ascii_uppercase() as u8 - b'A')]
    }
    if c.is_ascii_digit() {
        const DIGITS: [Key; 10] = [
            Key::Alpha0, Key::Alpha1, Key::Alpha2, Key::Alpha3, Key::Alpha4, Key::Alpha5, Key::Alpha6,
            Key::Alpha7, Key::Alpha8, Key::Alpha9,
        ];
        return DIGITS[usize::from(c as u8 - b'0')]
    }
    match c {
        ' ' => Key::Space,
        '.' => Key::Period,
        ',' => Key::Comma,
        '-' => Key::Minus,
        '+' => Key::Plus,
        '=' => Key::Equals,
        '/' => Key::Slash,
        '\\' => Key::Backslash,
        ';' => Key::Semicolon,
        ':' => Key::Colon,
        '\'' => Key::Apostrophe,
        '"' => Key::Quote,
        '?' => Key::QuestionMark,
        '_' => Key::Underscore,
        _ => Key::Space,
    }
}

/// Low-latency atomic input bus for @CriticalNative JNI bridge.
pub static INPUT_BUS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[inline(always)]
pub fn get_input_bus_mask() -> u32 {
    INPUT_BUS.load(std::sync::atomic::Ordering::Acquire)
}

#[no_mangle]
pub extern "C" fn Java_com_klisee_reloaded_NativeBridge_nativeSetInputMask(mask: i32) {
    INPUT_BUS.store(mask as u32, std::sync::atomic::Ordering::Release);
}

/// Called from the activity's `InputConnection` for every committed string.
#[no_mangle]
pub extern "system" fn Java_com_klisee_reloaded_GameActivity_nativeCommitText<'local>(
    env: jni::JNIEnv<'local>,
    _this: jni::objects::JObject<'local>,
    text: jni::objects::JString<'local>,
) {
    if env.push_local_frame(16).is_err() {
        return;
    }
    let value: String = env.get_string(&text).map(Into::into).unwrap_or_default();
    let _ = unsafe { env.pop_local_frame(&jni::objects::JObject::null()) };
    if value.is_empty() {
        return
    }
    if let Ok(mut queue) = ime_queue().lock() {
        queue.push_back(ImeEvent::Text(value));
    }
}

/// Called from the activity's `InputConnection` for the delete key.
#[no_mangle]
pub extern "system" fn Java_com_klisee_reloaded_GameActivity_nativeBackspace<'local>(
    _env: jni::JNIEnv<'local>,
    _this: jni::objects::JObject<'local>,
) {
    if let Ok(mut queue) = ime_queue().lock() {
        queue.push_back(ImeEvent::Backspace);
    }
}

/// Called from the activity's `InputConnection` for the enter/done key.
#[no_mangle]
pub extern "system" fn Java_com_klisee_reloaded_GameActivity_nativeEnter<'local>(
    _env: jni::JNIEnv<'local>,
    _this: jni::objects::JObject<'local>,
) {
    if let Ok(mut queue) = ime_queue().lock() {
        queue.push_back(ImeEvent::Enter);
    }
}

/// Called from the activity whenever the system keyboard's height changes, so the renderer can keep
/// the game above it.
#[no_mangle]
pub extern "system" fn Java_com_klisee_reloaded_GameActivity_nativeImeInset<'local>(
    _env: jni::JNIEnv<'local>,
    _this: jni::objects::JObject<'local>,
    height: jni::sys::jint,
) {
    // A keyboard taller than the window is not a keyboard: a value like that would push the game off
    // the top and hide the controls behind nothing, so it is dropped rather than believed.
    let (_, window_height) = android::window_size();
    let height = height.max(0) as u32;
    let height = if window_height > 0 && height >= window_height { 0 } else { height };
    crate::touch_controls::set_ime_inset(height);
    // The keyboard can be dropped by the system as well as by us - its own close button, or the back
    // gesture - so whether it is up is taken from its height. Without this, the next touch on the
    // game's text would be read as "already up" and would not raise it again.
    IME_SHOWN.store(height > 0, std::sync::atomic::Ordering::Relaxed);
}

/// Called from the audio focus listener: true pauses the game's output.
#[no_mangle]
pub extern "system" fn Java_com_klisee_reloaded_GameActivity_nativeAudioFocus<'local>(
    _env: jni::JNIEnv<'local>,
    _this: jni::objects::JObject<'local>,
    lost: jni::sys::jboolean,
) {
    crate::game::audio::set_output_paused(lost != 0);
}

/// Called when the user updates controls/layouts in the launcher or touch editor,
/// reloading all layouts in hot memory without restarting the activity.
#[no_mangle]
pub extern "system" fn Java_com_klisee_reloaded_GameActivity_nativeReloadLayout<'local>(
    _env: jni::JNIEnv<'local>,
    _this: jni::objects::JObject<'local>,
    _profile: jni::sys::jint,
) {
    load_controls();
}

/// Returns backend status and fallback reason in JSON.
#[no_mangle]
pub unsafe extern "system" fn Java_com_klisee_reloaded_GameActivity_nativeBackendInfo<'local>(
    mut env: jni::JNIEnv<'local>,
    _this: jni::objects::JObject<'local>,
) -> jni::sys::jstring {
    let json = crate::render::get_backend_info_json();
    let s = env.new_string(json).unwrap_or_else(|_| env.new_string("").unwrap());
    s.into_raw()
}

/// Whether the system keyboard is up, so it is only raised or dropped on a change.
static IME_SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Raises or drops the system keyboard, but only when that is a change.
fn ime_set(visible: bool) {
    if IME_SHOWN.swap(visible, std::sync::atomic::Ordering::Relaxed) != visible {
        jni_set_ime(visible);
    }
}

/// Raises or drops the system keyboard by calling the activity's static `GameActivity.setImeVisible`.
/// The activity shows it from the UI thread, which Kotlin takes care of; here we only need to reach
/// the class. `FindClass` would look on the system class path (the context the activity publishes is
/// the Application), so the class is loaded through its own loader instead.
fn jni_set_ime(visible: bool) {
    with_activity(|env, class| {
        env.call_static_method(class, "setImeVisible", "(Z)V", &[jni::objects::JValue::Bool(visible as u8)])?;
        Ok(())
    });
}

/// Asks the activity to read the keyboard's height again.
///
/// The height normally arrives on its own, from the inset callback. This is the belt to that braces:
/// while the engine believes the keyboard is up it asks again once in a while, so an inset callback
/// that never came cannot leave the game pushed up top with the controls hidden.
fn jni_refresh_ime_height() {
    with_activity(|env, class| {
        env.call_static_method(class, "refreshImeHeight", "()V", &[])?;
        Ok(())
    });
}

/// The activity's class, held for the life of the process.
static ACTIVITY_CLASS: std::sync::OnceLock<jni::objects::GlobalRef> = std::sync::OnceLock::new();

/// Attaches this thread and runs `f` with the activity's class.
fn with_activity(f: impl FnOnce(&mut jni::JNIEnv, &jni::objects::JClass) -> jni::errors::Result<()>) {
    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }) else { return };
    // Permanent: the thread stays attached, and attaching on every call would only add work.
    let Ok(mut env) = vm.attach_current_thread_permanently() else { return };
    if ACTIVITY_CLASS.get().is_none() {
        let loaded = (|| -> jni::errors::Result<jni::objects::GlobalRef> {
            let context = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
            let loader = env.call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?.l()?;
            let name = env.new_string("com.klisee.reloaded.GameActivity")?;
            let class = env
                .call_method(
                    &loader,
                    "loadClass",
                    "(Ljava/lang/String;)Ljava/lang/Class;",
                    &[jni::objects::JValue::Object(&name)],
                )?
                .l()?;
            env.new_global_ref(class)
        })();
        match loaded {
            Ok(global) => {
                let _ = ACTIVITY_CLASS.set(global);
            },
            Err(_) => return,
        }
    }
    let Some(global) = ACTIVITY_CLASS.get() else { return };
    let class = unsafe { jni::objects::JClass::from_raw(global.as_obj().as_raw()) };
    let _ = f(&mut env, &class);
}

/// Copies everything the engine prints to logcat.
fn pipe_std_to_logcat() {
    // The native entry can run again if the Activity is recreated in the same process; a second
    // pipe thread would keep the first one blocked on a dead pipe forever.
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(install_std_log_pipe);
}

fn install_std_log_pipe() {
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: `fds` is a valid array of two ints, which is what pipe() fills in.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return
    }
    // SAFETY: both ends are valid fds. Duplicating the write end onto stdout and stderr is what puts
    // the engine's prints in the pipe, and the spare write end is not needed after that.
    unsafe {
        libc::dup2(fds[1], libc::STDOUT_FILENO);
        libc::dup2(fds[1], libc::STDERR_FILENO);
        libc::close(fds[1]);
    }
    let read_fd = fds[0];
    std::thread::spawn(move || {
        use std::io::Read;
        use std::os::unix::io::FromRawFd;
        // SAFETY: read_fd is the pipe's read end and nothing else owns it.
        let mut pipe = unsafe { std::fs::File::from_raw_fd(read_fd) };
        let mut chunk = [0u8; 2048];
        let mut pending: Vec<u8> = Vec::new();
        let mut last = String::new();
        let mut repeats = 0u32;
        loop {
            let Ok(read) = pipe.read(&mut chunk) else { break };
            if read == 0 {
                break
            }
            pending.extend_from_slice(&chunk[..read]);
            while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
                let line: Vec<u8> = pending.drain(..=end).collect();
                let text = String::from_utf8_lossy(&line).trim_end().to_string();
                if text.is_empty() {
                    continue
                }
                if text == last {
                    repeats += 1;
                    continue
                }
                if repeats > 0 {
                    android_log_line(&format!("  (repetido {} veces)", repeats));
                    repeats = 0;
                }
                last = text.clone();
                android_log_line(&text);
            }
        }
    });
}

/// One line to logcat, under the tag the launcher's log screen shows.
fn android_log_line(text: &str) {
    const ANDROID_LOG_INFO: i32 = 4;
    let text = format!("{}\0", text);
    // SAFETY: both arguments are NUL-terminated C strings that outlive the call.
    unsafe {
        __android_log_write(
            ANDROID_LOG_INFO,
            b"gm8emulator\0".as_ptr() as *const libc::c_char,
            text.as_ptr() as *const libc::c_char,
        );
    }
}

extern "C" {
    /// From `liblog`, which this library already depends on.
    fn __android_log_write(priority: i32, tag: *const libc::c_char, text: *const libc::c_char) -> i32;
}

/// Runs the emulator. `poll_events` must be driven from the thread `android_main` is called
/// on, because android-activity asserts it, so the pump cannot live on a worker thread.
/// The emulator itself runs on a worker thread instead.
pub fn android_entry(app: AndroidApp) {
    // First thing, so the opening lines of the session are captured as well.
    pipe_std_to_logcat();
    std::env::set_var("RUST_BACKTRACE", "1");
    // A recreated Activity must not inherit the previous session's held keys.
    reset_input_state();

    // Install robust crash dump telemetry hook
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Error desconocido"
        };
        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "ubicacion desconocida".to_string());
        let current_room = crate::touch_controls::current_context();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| format!("{} seg (UNIX)", d.as_secs()))
            .unwrap_or_else(|_| "desconocida".to_string());
        let dump = format!(
            "=== POKEMON RELOADED ANDROID CRASH DUMP ===\n\
             Version: 9.6\n\
             Fecha/Hora (UTC): {}\n\
             Panico: {}\n\
             Ubicacion: {}\n\
             Estado de Sala: {}\n\
             ===========================================\n",
            timestamp, payload, location, current_room
        );
        eprintln!("{}", dump);
        let _ = std::fs::write("/data/data/com.klisee.reloaded/files/crash_dump.txt", &dump);
        let _ = std::fs::write("crash_dump.txt", &dump);
    }));

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

    SHUTDOWN.store(false, std::sync::atomic::Ordering::SeqCst);
    let handle = std::thread::spawn(move || {
        crate::game::big_little::get_cpu_topology().pin_current_thread_to_big();
        #[cfg(target_os = "android")]
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            let num_cpus = libc::sysconf(libc::_SC_NPROCESSORS_ONLN);
            if num_cpus >= 8 {
                for cpu in 4..8 {
                    let idx = cpu as usize / (8 * std::mem::size_of::<libc::c_ulong>());
                    let bit = cpu as usize % (8 * std::mem::size_of::<libc::c_ulong>());
                    let ptr = &mut set as *mut _ as *mut libc::c_ulong;
                    *ptr.add(idx) |= 1 << bit;
                }
                libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
            }
        }

        // The activity asks for landscape in the manifest, so the window is normally already the right
        // way round; the rotate still takes a moment on some phones. The engine lays its screens out
        // from the size of the window it is given, and whatever it gets here is what the whole session
        // keeps, so wait for the landscape window instead of racing it. Bounded, so a phone that never
        // reports one cannot hang the launch.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
        while std::time::Instant::now() < deadline {
            let (w, h) = android::window_size();
            if w > h {
                break
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let game_exe = match ensure_game_extracted() {
            Ok(exe) => exe,
            Err(e) => {
                logf(&format!("no se pudo preparar el juego: {}", e));
                game_over_thread.store(true, std::sync::atomic::Ordering::SeqCst);
                return
            },
        };

        crate::render::opengl::egl::log_stubbed_once();
        let game_parent = game_exe.parent().unwrap_or_else(|| std::path::Path::new("/")).to_path_buf();
        provision_soundfont(&game_parent);
        let battle_dir = game_parent.join("data").join("pkmnbattle");
        crate::asset::sprite_cache::start_background_asset_streamer(&battle_dir);

        logf(&format!("arrancando {}", game_exe.display()));
        let outcome = std::panic::catch_unwind(|| {
            crate::run_game(vec![
                "gm8emulator".to_string(),
                game_exe.to_string_lossy().into_owned(),
            ])
        });
        match outcome {
            Ok(code) => logf(&format!("el emulador termino (codigo {}); se cierra la actividad", code)),
            Err(_) => logf("el emulador se cayo (panic); se cierra la actividad"),
        }
        game_over_thread.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    if let Ok(mut worker_slot) = WORKER_HANDLE.lock() {
        *worker_slot = Some(handle);
    }

    let mut destroying = false;
    let mut ime_check = Instant::now();
    let mut text_entry_was_open = false;
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
        flush_pending_rotate();
        let pending_haptic = crate::touch_controls::PENDING_HAPTIC.swap(0, std::sync::atomic::Ordering::Relaxed);
        if pending_haptic > 0 {
            jni_vibrate(pending_haptic as i64);
        }

        // The keyboard follows the game's text dialog: it comes up by itself the moment the game asks
        // for a name, and goes away when the dialog closes. Only the change is acted on, so closing
        // it by hand while the dialog is open is respected instead of being undone on the next tick;
        // touching the text brings it back (see `pointer_down`).
        let text_entry = crate::touch_controls::text_entry_open();
        if text_entry != text_entry_was_open {
            text_entry_was_open = text_entry;
            ime_set(text_entry);
        }
        // Belt and braces: while the keyboard is believed to be up, ask the activity for its height
        // again about once a second. A callback that never arrived would otherwise leave the game
        // pushed up top and the controls hidden, with no way for the player to get them back.
        if crate::touch_controls::ime_inset() > 0 && ime_check.elapsed() >= Duration::from_secs(1) {
            ime_check = Instant::now();
            jni_refresh_ime_height();
        }

        // While paused the engine is not drawing, so the pump does not need to spin: a long wait
        // still wakes immediately for Resume, input or Destroy.
        let poll_wait = if ramen::platform::android::paused() {
            Duration::from_secs(1)
        } else {
            Duration::from_millis(8)
        };
        game_app.poll_events(Some(poll_wait), |event| match event {
            PollEvent::Main(main_event) => match main_event {
                // Init/Resume/Start all mean the surface is (re)available, which is also what
                // happens when the user comes back from the home screen.
                android_activity::MainEvent::InitWindow { .. }
                | android_activity::MainEvent::WindowResized { .. }
                | android_activity::MainEvent::RedrawNeeded { .. }
                | android_activity::MainEvent::Resume { .. }
                | android_activity::MainEvent::GainedFocus
                | android_activity::MainEvent::Start => {
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
                        // Only wake the engine once it has somewhere to draw. Resuming before the
                        // surface exists made the render loop spin without presenting a frame.
                        if crate::render::opengl::egl::has_surface() {
                            ramen::platform::android::set_paused(false);
                            crate::game::audio::set_output_paused(false);
                            crate::game::big_little::get_cpu_topology().pin_current_thread_to_big();
        #[cfg(target_os = "android")]
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            let num_cpus = libc::sysconf(libc::_SC_NPROCESSORS_ONLN);
            if num_cpus >= 8 {
                for cpu in 4..8 {
                    let idx = cpu as usize / (8 * std::mem::size_of::<libc::c_ulong>());
                    let bit = cpu as usize % (8 * std::mem::size_of::<libc::c_ulong>());
                    let ptr = &mut set as *mut _ as *mut libc::c_ulong;
                    *ptr.add(idx) |= 1 << bit;
                }
                libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
            }
        }
                        }
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

    if !game_over.load(std::sync::atomic::Ordering::SeqCst) {
        SHUTDOWN.store(true, std::sync::atomic::Ordering::SeqCst);
        android::push_input(ramen::platform::android::InputEvent::CloseRequest);

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
        while !game_over.load(std::sync::atomic::Ordering::SeqCst) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if !game_over.load(std::sync::atomic::Ordering::SeqCst) {
            eprintln!("[LIFECYCLE] Worker thread no respondió a SHUTDOWN tras 2s. Terminando proceso de forma segura.");
            std::process::exit(0);
        }
    }

    let last = PUBLISHED_WINDOW.swap(std::ptr::null_mut(), std::sync::atomic::Ordering::Relaxed);
    if !last.is_null() {
        unsafe { ANativeWindow_release(last) };
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
        Some(stats.f_bavail as u64 * stats.f_frsize as u64)
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

    let exe_name = exe.file_name().ok_or_else(|| "nombre de archivo .exe invalido".to_string())?;
    let exe = game_dir.join(exe_name);
    logf(&format!("listo: {}", exe.display()));
    Ok(exe)
}
