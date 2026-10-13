/// Represents the state of GUI controls on a [`Window`](super::Window).
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Controls {
    pub(crate) close: bool,
    pub(crate) maximise: bool,
    pub(crate) minimise: bool,
}

impl Controls {
    pub const fn new() -> Self {
        Controls {
            close: true,
            maximise: true,
            minimise: true,
        }
    }

    pub const fn close(self, close: bool) -> Self {
        Self { close, ..self }
    }

    pub const fn maximise(self, maximise: bool) -> Self {
        Self { maximise, ..self }
    }

    pub const fn minimise(self, minimise: bool) -> Self {
        Self { minimise, ..self }
    }
}

impl Default for Controls {
    /// Default trait implementation, same as [`Controls::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the state of visual properties of a [`Window`](super::Window).
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Style {
    pub(crate) borderless: bool,
    pub(crate) controls: Option<Controls>,
    pub(crate) resizable: bool,
    pub(crate) visible: bool,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            borderless: false,
            controls: Some(Controls::new()),
            resizable: true,
            visible: true,
        }
    }

    pub const fn borderless(self, borderless: bool) -> Self {
        Self { borderless, ..self }
    }

    pub const fn controls(self, controls: Option<Controls>) -> Self {
        Self { controls, ..self }
    }

    /// Whether the window can be interactively resized by the user.
    ///
    /// Note that this being `false` does not prevent it being done via API calls.
    ///
    /// Defaults to `true`.
    pub const fn resizable(self, resizable: bool) -> Self {
        // TODO spelling
        Self { resizable, ..self }
    }

    /// Whether the window is visible to the user.
    ///
    /// Defaults to `true`.
    pub const fn visible(self, visible: bool) -> Self {
        Self { visible, ..self }
    }
}

impl Default for Style {
    /// Default trait implementation, same as [`Style::new`].
    fn default() -> Self {
        Self::new()
    }
}
