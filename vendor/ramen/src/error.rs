use std::borrow::Cow;

#[derive(Debug)]
pub enum Error {
    /// The request can't be completed because of a user error such as an invalid parameter or program state
    Invalid,

    /// The program has run out of stack or a heap allocation has failed
    OutOfMemory,

    /// The system did not have the resources to fulfil the request
    SystemResources,

    /// A human-readable error string was returned by the operating system
    Text(Cow<'static, str>),

    /// The reason for failure can't be determined because none was reported by the backend
    Unknown,

    /// The platform does not support the features necessary for this request
    Unsupported,
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}