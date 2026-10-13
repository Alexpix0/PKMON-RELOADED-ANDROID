#[derive(Debug)]
#[cfg_attr(feature = "serde-derives", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// The device no longer exists (ie. it has been disabled or unplugged)
    DeviceNotAvailable,

    /// The device doesn't support any of the playback configurations we can use
    DeviceNotUsable,

    /// There is no output device available
    NoOutputDevice,

    /// The requested API is not available on this system
    ApiNotAvailable,

    /// OOM
    OutOfMemory,

    /// An error unknown to this crate has been reported by the host
    Unknown,
}
