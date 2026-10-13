use crate::{error::Error, source::{ChannelCount, SampleRate, Source}};

macro_rules! backends {
    (
        $(
            $(#[$outer:meta])*
            mod $name:ident => $variant:ident if $cfg:meta
        ),* $(,)?
    ) => {
        const _CALL_BACKENDS_MACRO_ONLY_ONCE: () = ();

        $(
            #[cfg($cfg)]
            $(#[$outer])*
            mod $name;
        )*

        /// Represents a native API to create [`Session`] instances in.
        #[derive(Copy, Clone, Debug, Eq, PartialEq)]
        pub enum Api {
            $(
                $(#[$outer])*
                $variant
            ),*
        }

        /// Handle to an audio input/output device.
        ///
        /// To acquire a [`Device`], request one from a [`Session`].
        pub struct Device(pub(crate) DeviceImpl);

        pub(crate) enum DeviceImpl {
            $(
                #[cfg($cfg)]
                $(#[$outer])*
                $variant ( $name::Device )
            ),*
        }

        backend_wrap_fns! {
            impl Device(DeviceImpl) <- $( $variant if $cfg ),* {
                pub fn channel_count(&self) -> ChannelCount;
                pub fn sample_rate(&self) -> SampleRate;
            }
        }

        /// Handle to an audio output stream.
        pub struct OutputStream(pub(crate) OutputStreamImpl);

        pub(crate) enum OutputStreamImpl {
            $(
                #[cfg($cfg)]
                $(#[$outer])*
                $variant ( $name::OutputStream )
            ),*
        }

        /// Represents an audio session within a native API.
        pub struct Session(pub(crate) SessionImpl);

        pub(crate) enum SessionImpl {
            $(
                #[cfg($cfg)]
                $(#[$outer])*
                $variant ( $name::Session )
            ),*
        }

        impl Session {
            /// Creates an audio `Session` within the given API.
            pub fn new(backend: Api) -> Result<Self, Error> {
                match backend {
                    $(
                        Api::$variant => {
                            #[cfg($cfg)]
                            { $name::Session::new().map(|x| Self(SessionImpl::$variant(x))) }
                            #[cfg(not($cfg))]
                            { Err(Error::ApiNotAvailable) }
                        },
                    )*
                }
            }
        }

        backend_wrap_fns! {
            impl Session(SessionImpl) <- $( $variant if $cfg ),* {
                pub fn default_output_device(&self) -> Result<Device, Error>;

                pub fn open_output_stream(
                    &self,
                    device: Device,
                ) -> Result<OutputStream, Error>;
            }
        }

        backend_wrap_fns! {
            impl OutputStream(OutputStreamImpl) <- $( $variant if $cfg ),* {
                pub fn play(
                    &self,
                    source: impl Source + Send + 'static
                ) -> Result<(), Error>;
            }
        }
    };
}

macro_rules! backend_wrap_fns {
    (
        impl $target:ident ( $enum:ident ) <- $( $variant:ident if $cfg:meta ),* {
            $( $rest:tt )*
        }
    ) => {
        impl $target {
            backend_wrap_fns!(@wrap $enum ( $($variant if $cfg),* ) $($rest)*);
        }
    };
    (
        @wrap $enum:ident ( $( $variant:ident if $cfg:meta ),* )
        $(#[$fn_outer:meta])*
        $v:vis fn $fn_name:ident ( & self $( , $arg_name:ident : $arg_ty:ty )* $(,)? ) $( -> $ret:ty )? ;
        $( $rest:tt )*
    ) => {
        $(#[$fn_outer])*
        $v fn $fn_name ( & self $( , $arg_name : $arg_ty)* ) $( -> $ret )? {
            macro_rules! _invoke_hack {
                ($imp:expr) => { $imp . $fn_name ( $($arg_name),* ) };
            }
            match self.0 {
                $(
                    #[cfg($cfg)]
                    $enum :: $variant (ref imp) => _invoke_hack!(imp)
                ),*
            }
        }
        backend_wrap_fns!(@wrap $enum ( $($variant if $cfg),* ) $($rest)*);
    };
    ( @wrap $enum:ident ( $( $variant:ident if $cfg:meta ),* ) ) => {};
}
macro_rules! session_wrap {
    ($res:expr , $t:ident ( $t_impl:ident ) , $api:ident ) => {
        { $ res }.map(|x| crate :: session :: $t ( crate :: session :: $t_impl :: $api ( x ) ) )
    };
}

backends! {
    /// Dummy API (no sound is played)
    mod dummy => Dummy if not(target_pointer_width = "0"),

    /// andrewrk's libsoundio
    mod sio => SoundIo if all(not(target_os = "android"), not(target_pointer_width = "0")),

    // Windows Audio Session API (WASAPI)
    //mod wasapi => Wasapi if all(target_os = "windows", feature = "wasapi"),

    // ALSA
    //mod alsa => Alsa if all(any(target_os = "dragonfly", target_os = "freebsd", target_os = "linux"), feature = "wasapi"),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SampleFormat {
    // /// Unsigned 16-bit integer PCM
    // U16,

    /// Signed 16-bit integer PCM
    I16,

    /// IEEE 754 32-bit float PCM
    F32,
}
