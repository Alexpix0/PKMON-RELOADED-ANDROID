//! Android platform backend for ramen.
//!
//! The window and all input come from `android-activity` (`ANativeActivity` + the
//! NDK looper). The GL surface is created by the caller with EGL on the
//! `ANativeWindow` handed out by [`Window::native_window`].

use std::{
    collections::VecDeque,
    ffi::c_void,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex, OnceLock,
    },
};

use crate::{
    error::Error,
    event::Event,
    input::{Key, MouseButton},
    window::Builder,
};

/// Stand-in for X11's `Display`, which Android does not have.
#[repr(C)]
pub struct Display {
    _opaque: [u8; 0],
}

/// A raw input event, as translated by the activity pump.
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    PointerMoved { x: f32, y: f32 },
    PointerDown { x: f32, y: f32, button: u8 },
    PointerUp { x: f32, y: f32, button: u8 },
    KeyDown { key: Key, text: Option<char> },
    KeyUp { key: Key },
    Resize { w: u16, h: u16 },
    CloseRequest,
    Focus(bool),
}

pub struct AppState {
    window: *mut c_void,
    width: u32,
    height: u32,
    queue: VecDeque<InputEvent>,
}

unsafe impl Send for AppState {}

static APP_STATE: OnceLock<Mutex<AppState>> = OnceLock::new();

pub fn state() -> &'static Mutex<AppState> {
    APP_STATE.get_or_init(|| {
        Mutex::new(AppState { window: std::ptr::null_mut(), width: 0, height: 0, queue: VecDeque::new() })
    })
}

pub fn set_native_window(window: *mut c_void, width: u32, height: u32) {
    let mut s = state().lock().unwrap();
    s.window = window;
    s.width = width;
    s.height = height;
}

pub fn push_input(ev: InputEvent) {
    state().lock().unwrap().queue.push_back(ev);
}

/// True while the activity is in the background. The engine stops working then: there is nothing to
/// draw to, and a game running behind a home screen only wastes battery.
static PAUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Relaxed);
}

pub fn paused() -> bool {
    PAUSED.load(Ordering::Relaxed)
}

pub fn push_resize(w: u16, h: u16) {
    let mut s = state().lock().unwrap();
    s.width = u32::from(w);
    s.height = u32::from(h);
    s.queue.push_back(InputEvent::Resize { w, h });
}

pub struct Connection {
    native_window: *mut c_void,
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

impl Connection {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self { native_window: std::ptr::null_mut() })
    }

    pub(crate) fn xscreenid(&self) -> u32 {
        0
    }

    pub(crate) fn xdisplay(&self) -> *mut Display {
        std::ptr::null_mut()
    }
}

pub struct Window {
    native_window: *mut c_void,
    buffer: Vec<Event>,
}

impl Window {
    pub(crate) fn new(builder: Builder) -> Result<Self, Error> {
        let _ = builder;
        let native_window = state().lock().unwrap().window;
        if native_window.is_null() {
            return Err(Error::Unsupported)
        }
        Ok(Self { native_window, buffer: Vec::new() })
    }

    pub(crate) fn events(&self) -> &[Event] {
        &self.buffer
    }


    pub(crate) fn poll_events(&mut self) {
        self.buffer.clear();

        let mut queue: Vec<InputEvent> = {
            let mut s = state().lock().unwrap();
            s.queue.drain(..).collect()
        };

        for ev in queue.drain(..) {
            match ev {
                InputEvent::PointerMoved { x, y } => {
                    self.buffer.push(Event::MouseMove((x as u16, y as u16)));
                },
                InputEvent::PointerDown { x, y, button } => {
                    self.buffer.push(Event::MouseMove((x as u16, y as u16)));
                    self.buffer.push(Event::MouseDown(translate_button(button)));
                },
                InputEvent::PointerUp { x, y, button } => {
                    self.buffer.push(Event::MouseMove((x as u16, y as u16)));
                    self.buffer.push(Event::MouseUp(translate_button(button)));
                },
                InputEvent::KeyDown { key, text } => {
                    self.buffer.push(Event::KeyboardDown(key));
                    if let Some(ch) = text {
                        self.buffer.push(Event::Input(ch));
                    }
                },
                InputEvent::KeyUp { key } => {
                    self.buffer.push(Event::KeyboardUp(key));
                },
                InputEvent::Resize { w, h } => {
                    self.buffer.push(Event::Resize((w, h)));
                },
                InputEvent::CloseRequest => self.buffer.push(Event::CloseRequest),
                InputEvent::Focus(f) => self.buffer.push(Event::Focus(f)),
            }
        }
    }

    pub(crate) fn set_maximised(&self, _maximised: bool) {}
    pub(crate) fn set_position(&self, _position: (i16, i16)) {}
    pub(crate) fn set_borderless(&self, _borderless: bool) {}
    pub(crate) fn set_resizable(&self, _resizable: bool) {}
    pub(crate) fn set_size(&self, _size: (u16, u16)) {}
    pub(crate) fn set_title(&self, _title: &str) {}
    pub(crate) fn set_visible(&self, _visible: bool) {}
    pub(crate) fn xid(&self) -> *mut c_void {
        self.native_window
    }
    pub(crate) fn native_window(&self) -> *mut c_void {
        self.native_window
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        let s = state().lock().unwrap();
        (s.width, s.height)
    }
}

impl Drop for Window {
    fn drop(&mut self) {}
}

fn translate_button(id: u8) -> MouseButton {
    match id {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        _ => MouseButton::Left,
    }
}

/// Maps an Android keycode (`AKEYCODE_*`) to a ramen key.
pub(crate) fn android_vk_to_key(vk: i32) -> Option<Key> {
    Some(match vk {
        4 => Key::Backspace, // AKEYCODE_BACK
        66 => Key::Return,   // AKEYCODE_ENTER
        111 => Key::Escape,  // AKEYCODE_ESCAPE
        19 => Key::UpArrow,
        20 => Key::DownArrow,
        21 => Key::LeftArrow,
        22 => Key::RightArrow,
        _ => return None,
    })
}

pub(crate) fn next_event() -> Option<InputEvent> {
    state().lock().unwrap().queue.pop_front()
}

/// Helper for the activity pump: reports the current surface size in pixels.
pub fn window_size() -> (u32, u32) {
    let s = state().lock().unwrap();
    (s.width, s.height)
}

/// Drops an already-consumed key from the queue (no-op, kept for API symmetry).
pub(crate) fn ack_event() {}

static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);

pub(crate) fn bump_frame() -> u32 {
    FRAME_COUNT.fetch_add(1, Ordering::Relaxed)
}

/// Drains queued input without touching the event buffer, so a thread other than the
/// one calling `poll_events` can consume input while the pump waits on the activity.
pub fn drain_pending_input() -> Vec<InputEvent> {
    let mut s = state().lock().unwrap();
    s.queue.drain(..).collect()
}
