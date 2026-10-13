#[cfg(feature = "input")]
use crate::input::{Key, MouseButton};

#[derive(Copy, Clone, Debug, Hash, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// A request has been made to close the window, most likely by clicking the 'x' button or by pressing alt+F4.
    /// 
    /// This can be ignored if desired - the window will not actually close until it is dropped.
    CloseRequest,

    /// The window was focused (`true`) or un-focused (`false`).
    Focus(bool),

    /// The window was maximised (`true`) or un-maximised (`false`).
    Maximise(bool),

    /// The window was minimised (`true`) or un-minimised (`false`).
    Minimise(bool),

    /// The window was moved to a new position on the screen. The position relates to the top-left of the window's
    /// inner drawable area, excluding any borders or decorations, and is reported in pixels relative to the top-left
    /// of the user's desktop.
    Move((i16, i16)),

    /// The window was resized. The width and height are reported in pixels.
    Resize((u16, u16)),

    /// The window's visibility has changed.
    Visible(bool),

    /// A [`Key`] was pressed.
    /// 
    /// This event will tell you which physical key the user has pressed. If your aim is to process text input,
    /// consider using the [`Event::Input`] event type instead.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    KeyboardDown(Key),

    /// A [`Key`] was auto-repeated by the system because the user is holding it down.
    /// 
    /// This event will tell you which physical key the user is holding. If your aim is to process text input,
    /// consider using the [`Event::Input`] event type instead.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    KeyboardRepeat(Key),

    /// A [`Key`] was released.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    KeyboardUp(Key),

    /// A [`char`] was typed using the keyboard.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    Input(char),

    /// A [`MouseButton`] was pressed.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    MouseDown(MouseButton),

    /// A [`MouseButton`] was released.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    MouseUp(MouseButton),

    /// The mouse was moved to a new position on the screen. The position is reported in pixels relative to the
    /// top-left of the user's desktop.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    MouseMove((u16, u16)),

    /// The mouse wheel was scrolled up, away from the user.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    ScrollUp,

    /// The mouse wheel was scrolled down, towards the user.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    ScrollDown,

    /// The mouse has entered the client area of the window.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    MouseEnter,

    /// The mouse is no longer in the client area of the window.
    #[cfg_attr(feature = "nightly-rustdoc", doc(cfg(feature = "input")))]
    #[cfg_attr(not(feature = "nightly-rustdoc"), cfg(feature = "input"))]
    MouseLeave,
}
