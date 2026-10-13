#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Key {
    // common
    LeftAlt,
    RightAlt,
    Applications, // menu looking thing on right of spacebar usually
    Backspace,
    CapsLock,
    LeftControl,
    RightControl,
    Delete,
    End,
    Escape,
    Home,
    Insert,
    NumLock,
    PageDown,
    PageUp,
    Pause,
    PrintScreen,
    Return,
    ScrollLock,
    Sleep,
    Space,
    LeftShift,
    RightShift,
    LeftSuper,
    RightSuper,
    Tab,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,

    // alphanumeric
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Alpha0, Alpha1, Alpha2, Alpha3, Alpha4, Alpha5, Alpha6, Alpha7, Alpha8, Alpha9,

    // OEM keys which may not be present on all keyboards
    Minus, Plus, Equals, Period, Comma, Hash, BracketLeft, BracketRight, BraceLeft, BraceRight,
    Grave, Apostrophe, Semicolon, Colon, Underscore, Pipe, Slash, Backslash, QuestionMark,
    LessThan, GreaterThan, Quote,

    // numpad
    KeypadAdd, KeypadSubtract, KeypadMultiply, KeypadDivide,
    KeypadDecimal, KeypadSeparator, // dude i love locales
    Keypad0, Keypad1, Keypad2, Keypad3, Keypad4, Keypad5, Keypad6, Keypad7, Keypad8, Keypad9,

    // arrow
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,

    // media
    MediaPreviousTrack,
    MediaNextTrack,
    MediaPlayPause,
    MediaStop,
    MediaVolumeDown,
    MediaVolumeUp,
    MediaVolumeMute,

    // ibm related nonsense, useful if you have a programmable keyboard
    Attn,
    Clear,
    CrSel,
    EraseEof,
    Execute,
    ExSel,
    OemReset,
    OemJump,
    Oem102,
    OemPa1,
    OemPa2, // TODO map these keys on windows (see headers)
    OemPa3,
    OemWsCtrl,
    OemClear,
    OemCuSel,
    OemAttn,
    OemFinish,
    OemCopy,
    OemAuto,
    OemEnlw,
    OemBackTab,
    Pa1,
    Print,
    Select,
    Zoom,

    // ime input stuff
    ImeAccept,
    ImeConvert,
    ImeNonConvert,
    ImeFinal,
    ImeModeChangeRequest,
    ImeProcess,
    ImeOn,
    ImeOff,
    ImeKanaOrHangul,
    ImeHanjaOrKanji,
    ImeJunja,

    // microsoft nonsense
    BrowserBack,
    BrowserFavourites,
    BrowserForward,
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    BrowserStop,
    Help,
    LaunchApplication1,
    LaunchApplication2,
    LaunchMail, // what the fuck?
    LaunchMediaSelect,
    Play,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}
