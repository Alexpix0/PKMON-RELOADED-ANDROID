#[cfg_attr(feature = "nightly-rustdoc", doc(cfg(target_os = "windows")))]
#[cfg_attr(not(feature = "nightly-rustdoc"), cfg(target_os = "windows"))]
pub mod win32;

#[cfg(target_os = "windows")]
pub(crate) use win32 as imp;

#[cfg_attr(feature = "nightly-rustdoc", doc(cfg(target_os = "linux")))]
#[cfg_attr(not(feature = "nightly-rustdoc"), cfg(target_os = "linux"))]
pub mod linux;

#[cfg(target_os = "linux")]
pub(crate) use linux as imp;

#[cfg_attr(not(feature = "nightly-rustdoc"), cfg(target_os = "android"))]
pub mod android;

#[cfg(target_os = "android")]
pub(crate) use android as imp;
