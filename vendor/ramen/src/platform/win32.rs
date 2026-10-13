mod ffi;
mod imp;

// TODO general notes here about A|W functions and Ex performance penalty

// public re-exports
pub use self::{
    ffi::{HINSTANCE, HWND, WNDPROC},
    imp::{base_hinstance, window_proc},
};

// internals
pub(crate) use imp::{Connection, Window};
