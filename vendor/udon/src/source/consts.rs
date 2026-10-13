//! Helper constants for [`ChannelCount`] and [`SampleRate`].
//!
//! Since both of those types are guaranteed to be non-zero,
//! construction can be a bit more tricky than with a regular integer.
//!
//! For common channel counts and sample rates, these constants should simplify that.

use super::{ChannelCount, SampleRate};

macro_rules! nonzero_consts {
    ( $( $t:ty { $( $(#[$outer:meta])* $name:ident = $val:literal ),* $(,)? } ),* $(,)? ) => {
        $(
            $( $(#[$outer])* pub const $name: $t = unsafe { <$t> :: new_unchecked($val) }; )*
        )*
    };
}

nonzero_consts! {
    ChannelCount {
        /// One channel ("mono")
        CH_MONO = 1,

        /// Two channels ("stereo")
        CH_STEREO = 2,
    },

    SampleRate {
        /// 8,000Hz
        SR_8000 = 8000,

        /// 11,025Hz
        SR_11025 = 11025,

        /// 16,000Hz
        SR_16000 = 16000,

        /// 22,050Hz
        SR_22050 = 22050,

        /// 32,000Hz
        SR_32000 = 32000,

        /// 37,800Hz
        SR_37800 = 37800,

        /// 44,056Hz
        SR_44056 = 44056,

        /// 44,100Hz
        SR_44100 = 44100,

        /// 47,250Hz
        SR_47250 = 47250,

        /// 48,000Hz
        SR_48000 = 48000,

        /// 50,000Hz
        SR_50000 = 50000,

        /// 50,400Hz
        SR_50400 = 50400,

        /// 88,200Hz
        SR_88200 = 88200,

        /// 96,000Hz
        SR_96000 = 96000,

        /// 176,400Hz
        SR_176400 = 176400,

        /// 192,000Hz
        SR_192000 = 192000,

        /// 352,800Hz
        SR_352800 = 352800,

        /// 2,822,400Hz
        SR_2822400 = 2822400,

        /// 5,644,800Hz
        SR_5644800 = 5644800,
    },
}
