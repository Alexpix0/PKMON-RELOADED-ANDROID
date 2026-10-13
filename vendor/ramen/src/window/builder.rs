use super::{Cursor, Controls, Style};
use crate::{connection::Connection, error::Error, platform::imp};

use std::borrow::Cow;

/// Builder for instantiating a [`Window`](super::Window).
///
/// To create a builder, use [`Connection::builder`](Connection::builder).
/// 
/// To finish building and open a window, use [`build`](Self::build). This will consume the Builder.
/// Builders can be cloned if you'd like to re-use one to build multiple windows.
#[derive(Clone)]
pub struct Builder {
    pub(crate) cursor: Cursor,
    pub(crate) connection: Connection,
    pub(crate) class_name: Cow<'static, str>,
    pub(crate) maximised: bool,
    pub(crate) position: Option<(i16, i16)>,
    pub(crate) size: (u16, u16),
    pub(crate) style: Style,
    pub(crate) title: Cow<'static, str>,
    #[cfg(unix)]
    pub(crate) depth: Option<u8>,
    #[cfg(unix)]
    pub(crate) visual: Option<u32>,
}

impl Builder {
    pub(crate) const fn new(connection: Connection, style: Option<Style>) -> Self {
        Builder {
            cursor: Cursor::Arrow,
            connection,
            class_name: Cow::Borrowed("ramen_window"),
            maximised: false,
            position: None,
            size: (800, 600),
            style: match style {
                // Why is `Option::unwrap_or` not const fn?!
                Some(style) => style,
                None => Style::new(),
            },
            title: Cow::Borrowed("a nice window"),
            #[cfg(unix)]
            depth: None,
            #[cfg(unix)]
            visual: None,
        }
    }

    /// Attempt to build a Window, consuming this Builder object.
    pub fn build(self) -> Result<super::Window, Error> {
        imp::Window::new(self).map(super::Window)
    }

    /// Sets whether the window should be borderless.
    /// 
    /// Defaults to `false`.
    pub fn borderless(mut self, borderless: bool) -> Self {
        self.style.borderless = borderless;
        self
    }

    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = cursor;
        self
    }

    /// Hints at what control buttons this window should have. Window Managers may override any of these settings.
    /// 
    /// Even if `maximise` is `true`, ramen will not show the `maximise` button while the window is not resizable.
    /// 
    /// A value of `None` indicates that there should be no menu at all.
    /// 
    /// By default, all controls are set to `true`.
    pub fn controls(mut self, controls: Option<Controls>) -> Self {
        self.style.controls = controls;
        self
    }

    /// Sets the platform-specific window class name.
    ///
    /// Defaults to `"ramen_window"`.
    pub fn class_name<T>(mut self, class_name: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.class_name = class_name.into();
        self
    }

    /// Sets whether the window can be initially interactively resized by the user.
    ///
    /// Note that this being `false` does not completely prevent a window from changing size; it only prevents the user
    /// from resizing it via the normal methods.
    ///
    /// Defaults to `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.style.resizable = resizable;
        self
    }

    /// Sets the initial window title.
    ///
    /// Defaults to `"a nice window"`.
    pub fn title<T>(mut self, title: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.title = title.into();
        self
    }

    /// Sets whether the window should begin maximised on the user's monitor.
    /// 
    /// If the user has multiple monitors, the primary monitor will usually be selected.
    /// 
    /// This setting will have no effect if `visible` is set to `false`.
    /// 
    /// Defaults to `false`.
    pub fn maximised(mut self, maximised: bool) -> Self {
        self.maximised = maximised;
        self
    }

    /// Sets the screen position of the top-left of the window's inner drawable area, in pixels, relative to the
    /// top-left of the user's desktop. If `None`, the operating system will decide where to place the window.
    /// 
    /// While it is necessary to pass a position to the X11 backend, it is unlikely to be used, since almost every
    /// X11-based Window Manager will ignore this and use its own positioning logic. If the Window Manager places
    /// the window in a different position to this one, you will receive a `Move` event.
    /// 
    /// Defaults to `None`.
    pub fn position(mut self, position: Option<(i16, i16)>) -> Self {
        self.position = position;
        self
    }

    /// Sets the size of the window.
    /// 
    /// Defaults to (800, 600).
    pub fn size(mut self, size: (u16, u16)) -> Self {
        self.size = size;
        self
    }

    /// Sets whether the window is initially visible to the user.
    ///
    /// Defaults to `true`.
    pub fn visible(mut self, visible: bool) -> Self {
        self.style.visible = visible;
        self
    }

    #[cfg(unix)]
    pub fn depth(mut self, depth: u8) -> Self {
        self.depth = Some(depth);
        self
    }

    #[cfg(unix)]
    pub fn visual(mut self, visual: u32) -> Self {
        self.visual = Some(visual);
        self
    }
}
