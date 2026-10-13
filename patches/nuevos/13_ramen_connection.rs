use crate::{error::Error, platform::imp, window::Builder};
use crate::util::sync::Mutex;
use std::sync::Arc;

#[cfg(unix)]
use crate::util::sync;

/// A connection to a windowing backend, used as a factory object to create [`Window`](crate::window::Window)s and
/// [`Builder`](Builder)s.
/// 
/// This is a wrapper for an atomically reference-counted object, so it can be easily cloned for access from multiple
/// places, or different threads. It can be dropped without needing to drop any windows or builders created from it:
/// the internal connection will stay alive until all clones of this object **and** all such windows and builders
/// have been dropped.
#[derive(Clone)]
pub struct Connection(pub(crate) Arc<Mutex<imp::Connection>>);

impl Connection {
    /// Attempts to connect to the backend for the target platform.
    pub fn new() -> Result<Self, Error> {
        imp::Connection::new().map(|x| Self(Arc::new(Mutex::new(x))))
    }

    /// Creates a new [`Builder`](Builder) for interactively building a [`Window`](crate::window::Window).
    /// 
    /// The Connection object does not need to be kept after creating a Window with it, unless you intend to use it
    /// later to create more windows. If you don't need the Connection anymore after creating a Builder, use
    /// [`into_builder`](Self::into_builder) instead.
    pub fn builder(&self) -> Builder {
        Builder::new(self.clone(), None)
    }

    /// Creates a new [`Builder`](Builder) for interactively building a [`Window`](crate::window::Window).
    /// 
    /// In contrast to [`builder`](Self::builder), this function consumes the Connection, allowing for some minor
    /// optimisations. As such, this function should be preferred if the Connection is no longer needed.
    pub fn into_builder(self) -> Builder {
        Builder::new(self, None)
    }

    #[cfg(unix)]
    pub fn xscreenid(&self) -> u32 {
        let g = sync::mutex_lock(&*self.0);
        g.xscreenid()
    }

    #[cfg(unix)]
    pub fn xdisplay(&self) -> *mut crate::platform::imp::Display {
        let g = sync::mutex_lock(&*self.0);
        g.xdisplay()
    }
}
