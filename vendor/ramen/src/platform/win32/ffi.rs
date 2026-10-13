#![allow(clippy::upper_case_acronyms, nonstandard_style, overflowing_literals)]
// TODO
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
#![allow(dead_code)] // <------------
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// THIS NEEDS TO BE REMOVED BEFORE RELEASE
// Don't you dare forget...

// C Typedefs
pub(crate) use std::os::raw::{
    c_void,
    c_uchar, c_char,
    c_ushort, c_short,
    c_uint, c_int,
    c_ulong, c_long,
    c_ulonglong,
    //c_longlong,
};
pub(crate) type wchar_t = u16;

// Windows Typedefs
macro_rules! opaque {
    ($( $(#[$attr:meta])* $v:vis $name:ident = $void:ident ),+ $(,)?) => {
        $(
            $(#[$attr])* $v enum $void {}
            $(#[$attr])* $v type $name = *mut $void;
        )+
    };
}
opaque! {
    pub(crate) FARPROC = __some_function,
    pub(crate) HBRUSH = HBRUSH__,
    pub(crate) HDC = HDC__,
    pub(crate) HHOOK = HHOOK__,
    pub(crate) HICON = HICON__,
    pub(crate) HMENU = HMENU__,
    pub(crate) HMODULE = HMODULE__,
    pub(crate) HMONITOR = HMONITOR__,

    /// Opaque handle to a module in memory.
    pub HINSTANCE = HINSTANCE__,

    /// Opaque handle to a window.
    pub HWND = HWND__,
}
pub(crate) type ATOM = WORD;
pub(crate) type BOOL = c_int;
pub(crate) type BYTE = c_uchar;
pub(crate) type CHAR = c_char;
pub(crate) enum DPI_AWARENESS_CONTEXT__ {}
pub(crate) type DPI_AWARENESS_CONTEXT = *mut DPI_AWARENESS_CONTEXT__;
pub(crate) type DWORD = c_ulong;
pub(crate) type HANDLE = *mut c_void;
pub(crate) type HCURSOR = HICON;
pub(crate) type HLOCAL = HANDLE;
pub(crate) type HOOKPROC = unsafe extern "system" fn(c_int, WPARAM, LPARAM) -> LRESULT;
pub(crate) type HRESULT = c_long;
pub(crate) type INT = c_int;
pub(crate) type LANGID = USHORT;
pub(crate) type LONG = c_long;
pub(crate) type LONG_PTR = isize;
pub(crate) type LPARAM = LONG_PTR;
pub(crate) type LRESULT = LONG_PTR;
pub(crate) type NTSTATUS = LONG;
pub(crate) type PROCESS_DPI_AWARENESS = u32;
pub(crate) type SHORT = c_short;
pub(crate) type UINT = c_uint;
pub(crate) type UINT_PTR = usize;
pub(crate) type ULONG_PTR = usize;
pub(crate) type USHORT = c_ushort;
pub(crate) type WCHAR = wchar_t;
pub(crate) type WORD = c_ushort;
pub(crate) type WPARAM = UINT_PTR;
/// A user-defined application window callback procedure.
pub type WNDPROC = unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT;

// Constants
pub(crate) const _WIN32_WINNT_VISTA: WORD = 0x0600;
pub(crate) const _WIN32_WINNT_WINBLUE: WORD = 0x0603;
pub(crate) const CCHILDREN_TITLEBAR: usize = 5;
pub(crate) const CP_UTF8: DWORD = 65001;
pub(crate) const CS_OWNDC: UINT = 0x0020;
pub(crate) const CW_USEDEFAULT: c_int = 0x80000000;
pub(crate) const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: DPI_AWARENESS_CONTEXT = -4isize as _;
pub(crate) const ERROR_SUCCESS: DWORD = 0; // lol
pub(crate) const FALSE: BOOL = 0;
pub(crate) const GCL_CBCLSEXTRA: c_int = -20;
pub(crate) const GWL_EXSTYLE: c_int = -20;
pub(crate) const GWL_STYLE: c_int = -16;
pub(crate) const GWL_USERDATA: c_int = -21;
pub(crate) const HCBT_DESTROYWND: c_int = 4;
pub(crate) const HTCAPTION: LRESULT = 2;
pub(crate) const IDC_APPSTARTING: *const WCHAR = 32650 as _;
pub(crate) const IDC_ARROW: *const WCHAR = 32512 as _;
pub(crate) const IDC_CROSS: *const WCHAR = 32515 as _;
pub(crate) const IDC_HAND: *const WCHAR = 32649 as _;
pub(crate) const IDC_HELP: *const WCHAR = 32651 as _;
pub(crate) const IDC_IBEAM: *const WCHAR = 32513 as _;
pub(crate) const IDC_ICON: *const WCHAR = 32641 as _;
pub(crate) const IDC_NO: *const WCHAR = 32648 as _;
pub(crate) const IDC_SIZE: *const WCHAR = 32640 as _;
pub(crate) const IDC_SIZEALL: *const WCHAR = 32646 as _;
pub(crate) const IDC_SIZENESW: *const WCHAR = 32643 as _;
pub(crate) const IDC_SIZENS: *const WCHAR = 32645 as _;
pub(crate) const IDC_SIZENWSE: *const WCHAR = 32642 as _;
pub(crate) const IDC_SIZEWE: *const WCHAR = 32644 as _;
pub(crate) const IDC_UPARROW: *const WCHAR = 32516 as _;
pub(crate) const IDC_WAIT: *const WCHAR = 32514 as _;
pub(crate) const IMAGE_CURSOR: UINT = 2;
pub(crate) const INFINITE: DWORD = 0xFFFFFFFF;
pub(crate) const HTCLIENT: LRESULT = 1;
pub(crate) const LR_DEFAULTSIZE: UINT = 0x00000040;
pub(crate) const LR_SHARED: UINT = 0x00008000;
pub(crate) const MF_BYCOMMAND: UINT = 0x00000000;
pub(crate) const MF_DISABLED: UINT = 0x00000002;
pub(crate) const MF_ENABLED: UINT = 0x00000000;
pub(crate) const MF_GRAYED: UINT = 0x00000001;
pub(crate) const PM_NOREMOVE: UINT = 0;
pub(crate) const PROCESS_PER_MONITOR_DPI_AWARE: PROCESS_DPI_AWARENESS = 2;
pub(crate) const PROCESS_SYSTEM_DPI_AWARE: PROCESS_DPI_AWARENESS = 1;
pub(crate) const SC_CLOSE: WPARAM = 0xF060;
pub(crate) const SC_MAXIMIZE: WPARAM = 0xF030;
pub(crate) const SC_RESTORE: WPARAM = 0xF120;
pub(crate) const SIZE_RESTORED: WPARAM = 0;
pub(crate) const SIZE_MINIMIZED: WPARAM = 1;
pub(crate) const SIZE_MAXIMIZED: WPARAM = 2;
pub(crate) const SIZE_MAXSHOW: WPARAM = 3;
pub(crate) const SIZE_MAXHIDE: WPARAM = 4;
pub(crate) const SW_HIDE: c_int = 0;
pub(crate) const SW_SHOW: c_int = 5;
pub(crate) const SWP_ASYNCWINDOWPOS: UINT = 0x4000;
pub(crate) const SWP_DEFERERASE: UINT = 0x2000;
pub(crate) const SWP_DRAWFRAME: UINT = SWP_FRAMECHANGED;
pub(crate) const SWP_FRAMECHANGED: UINT = 0x0020;
pub(crate) const SWP_HIDEWINDOW: UINT = 0x0080;
pub(crate) const SWP_NOACTIVATE: UINT = 0x0010;
pub(crate) const SWP_NOCOPYBITS: UINT = 0x0100;
pub(crate) const SWP_NOMOVE: UINT = 0x0002;
pub(crate) const SWP_NOOWNERZORDER: UINT = 0x0200;
pub(crate) const MONITOR_DEFAULTTONEAREST: DWORD = 0x00000002;
pub(crate) const SWP_NOREDRAW: UINT = 0x0008;
pub(crate) const SWP_NOREPOSITION: UINT = SWP_NOOWNERZORDER;
pub(crate) const SWP_NOSENDCHANGING: UINT = 0x0400;
pub(crate) const SWP_NOSIZE: UINT = 0x0001;
pub(crate) const SWP_NOZORDER: UINT = 0x0004;
pub(crate) const SWP_SHOWWINDOW: UINT = 0x0040;
pub(crate) const TRUE: BOOL = 1;
pub(crate) const VER_BUILDNUMBER: DWORD = 0x0000004;
pub(crate) const VER_GREATER_EQUAL: BYTE = 3;
pub(crate) const VER_MAJORVERSION: DWORD = 0x0000002;
pub(crate) const VER_MINORVERSION: DWORD = 0x0000001;
pub(crate) const VER_SERVICEPACKMAJOR: DWORD = 0x0000020;
pub(crate) const VER_SERVICEPACKMINOR: DWORD = 0x0000010;
// WINAPI defines these as `int` but that's annoying and stupid for several reasons.
// We redefine them as u8's.
pub(crate) const VK_LBUTTON: u8 = 0x01;
pub(crate) const VK_RBUTTON: u8 = 0x02;
pub(crate) const VK_CANCEL: u8 = 0x03;
pub(crate) const VK_MBUTTON: u8 = 0x04;
pub(crate) const VK_XBUTTON1: u8 = 0x05;
pub(crate) const VK_XBUTTON2: u8 = 0x06;
pub(crate) const VK_BACK: u8 = 0x08;
pub(crate) const VK_TAB: u8 = 0x09;
pub(crate) const VK_CLEAR: u8 = 0x0C;
pub(crate) const VK_RETURN: u8 = 0x0D;
pub(crate) const VK_SHIFT: u8 = 0x10;
pub(crate) const VK_CONTROL: u8 = 0x11;
pub(crate) const VK_MENU: u8 = 0x12;
pub(crate) const VK_PAUSE: u8 = 0x13;
pub(crate) const VK_CAPITAL: u8 = 0x14;
pub(crate) const VK_KANA: u8 = 0x15;
pub(crate) const VK_IME_ON: u8 = 0x16;
pub(crate) const VK_JUNJA: u8 = 0x17;
pub(crate) const VK_FINAL: u8 = 0x18;
pub(crate) const VK_HANJA: u8 = 0x19;
pub(crate) const VK_KANJI: u8 = 0x19;
pub(crate) const VK_IME_OFF: u8 = 0x1A;
pub(crate) const VK_ESCAPE: u8 = 0x1B;
pub(crate) const VK_CONVERT: u8 = 0x1C;
pub(crate) const VK_NONCONVERT: u8 = 0x1D;
pub(crate) const VK_ACCEPT: u8 = 0x1E;
pub(crate) const VK_MODECHANGE: u8 = 0x1F;
pub(crate) const VK_SPACE: u8 = 0x20;
pub(crate) const VK_PRIOR: u8 = 0x21;
pub(crate) const VK_NEXT: u8 = 0x22;
pub(crate) const VK_END: u8 = 0x23;
pub(crate) const VK_HOME: u8 = 0x24;
pub(crate) const VK_LEFT: u8 = 0x25;
pub(crate) const VK_UP: u8 = 0x26;
pub(crate) const VK_RIGHT: u8 = 0x27;
pub(crate) const VK_DOWN: u8 = 0x28;
pub(crate) const VK_SELECT: u8 = 0x29;
pub(crate) const VK_PRINT: u8 = 0x2A;
pub(crate) const VK_EXECUTE: u8 = 0x2B;
pub(crate) const VK_SNAPSHOT: u8 = 0x2C;
pub(crate) const VK_INSERT: u8 = 0x2D;
pub(crate) const VK_DELETE: u8 = 0x2E;
pub(crate) const VK_HELP: u8 = 0x2F;
pub(crate) const VK_LWIN: u8 = 0x5B;
pub(crate) const VK_RWIN: u8 = 0x5C;
pub(crate) const VK_APPS: u8 = 0x5D;
pub(crate) const VK_SLEEP: u8 = 0x5F;
pub(crate) const VK_NUMPAD0: u8 = 0x60;
pub(crate) const VK_NUMPAD1: u8 = 0x61;
pub(crate) const VK_NUMPAD2: u8 = 0x62;
pub(crate) const VK_NUMPAD3: u8 = 0x63;
pub(crate) const VK_NUMPAD4: u8 = 0x64;
pub(crate) const VK_NUMPAD5: u8 = 0x65;
pub(crate) const VK_NUMPAD6: u8 = 0x66;
pub(crate) const VK_NUMPAD7: u8 = 0x67;
pub(crate) const VK_NUMPAD8: u8 = 0x68;
pub(crate) const VK_NUMPAD9: u8 = 0x69;
pub(crate) const VK_MULTIPLY: u8 = 0x6A;
pub(crate) const VK_ADD: u8 = 0x6B;
pub(crate) const VK_SEPARATOR: u8 = 0x6C;
pub(crate) const VK_SUBTRACT: u8 = 0x6D;
pub(crate) const VK_DECIMAL: u8 = 0x6E;
pub(crate) const VK_DIVIDE: u8 = 0x6F;
pub(crate) const VK_F1: u8 = 0x70;
pub(crate) const VK_F2: u8 = 0x71;
pub(crate) const VK_F3: u8 = 0x72;
pub(crate) const VK_F4: u8 = 0x73;
pub(crate) const VK_F5: u8 = 0x74;
pub(crate) const VK_F6: u8 = 0x75;
pub(crate) const VK_F7: u8 = 0x76;
pub(crate) const VK_F8: u8 = 0x77;
pub(crate) const VK_F9: u8 = 0x78;
pub(crate) const VK_F10: u8 = 0x79;
pub(crate) const VK_F11: u8 = 0x7A;
pub(crate) const VK_F12: u8 = 0x7B;
pub(crate) const VK_F13: u8 = 0x7C;
pub(crate) const VK_F14: u8 = 0x7D;
pub(crate) const VK_F15: u8 = 0x7E;
pub(crate) const VK_F16: u8 = 0x7F;
pub(crate) const VK_F17: u8 = 0x80;
pub(crate) const VK_F18: u8 = 0x81;
pub(crate) const VK_F19: u8 = 0x82;
pub(crate) const VK_F20: u8 = 0x83;
pub(crate) const VK_F21: u8 = 0x84;
pub(crate) const VK_F22: u8 = 0x85;
pub(crate) const VK_F23: u8 = 0x86;
pub(crate) const VK_F24: u8 = 0x87;
pub(crate) const VK_NAVIGATION_VIEW: u8 = 0x88;
pub(crate) const VK_NAVIGATION_MENU: u8 = 0x89;
pub(crate) const VK_NAVIGATION_UP: u8 = 0x8A;
pub(crate) const VK_NAVIGATION_DOWN: u8 = 0x8B;
pub(crate) const VK_NAVIGATION_LEFT: u8 = 0x8C;
pub(crate) const VK_NAVIGATION_RIGHT: u8 = 0x8D;
pub(crate) const VK_NAVIGATION_ACCEPT: u8 = 0x8E;
pub(crate) const VK_NAVIGATION_CANCEL: u8 = 0x8F;
pub(crate) const VK_NUMLOCK: u8 = 0x90;
pub(crate) const VK_SCROLL: u8 = 0x91;
pub(crate) const VK_OEM_NEC_EQUAL: u8 = 0x92;
pub(crate) const VK_OEM_FJ_JISHO: u8 = 0x92;
pub(crate) const VK_OEM_FJ_MASSHOU: u8 = 0x93;
pub(crate) const VK_OEM_FJ_TOUROKU: u8 = 0x94;
pub(crate) const VK_OEM_FJ_LOYA: u8 = 0x95;
pub(crate) const VK_OEM_FJ_ROYA: u8 = 0x96;
pub(crate) const VK_LSHIFT: u8 = 0xA0;
pub(crate) const VK_RSHIFT: u8 = 0xA1;
pub(crate) const VK_LCONTROL: u8 = 0xA2;
pub(crate) const VK_RCONTROL: u8 = 0xA3;
pub(crate) const VK_LMENU: u8 = 0xA4;
pub(crate) const VK_RMENU: u8 = 0xA5;
pub(crate) const VK_BROWSER_BACK: u8 = 0xA6;
pub(crate) const VK_BROWSER_FORWARD: u8 = 0xA7;
pub(crate) const VK_BROWSER_REFRESH: u8 = 0xA8;
pub(crate) const VK_BROWSER_STOP: u8 = 0xA9;
pub(crate) const VK_BROWSER_SEARCH: u8 = 0xAA;
pub(crate) const VK_BROWSER_FAVORITES: u8 = 0xAB;
pub(crate) const VK_BROWSER_HOME: u8 = 0xAC;
pub(crate) const VK_VOLUME_MUTE: u8 = 0xAD;
pub(crate) const VK_VOLUME_DOWN: u8 = 0xAE;
pub(crate) const VK_VOLUME_UP: u8 = 0xAF;
pub(crate) const VK_MEDIA_NEXT_TRACK: u8 = 0xB0;
pub(crate) const VK_MEDIA_PREV_TRACK: u8 = 0xB1;
pub(crate) const VK_MEDIA_STOP: u8 = 0xB2;
pub(crate) const VK_MEDIA_PLAY_PAUSE: u8 = 0xB3;
pub(crate) const VK_LAUNCH_MAIL: u8 = 0xB4;
pub(crate) const VK_LAUNCH_MEDIA_SELECT: u8 = 0xB5;
pub(crate) const VK_LAUNCH_APP1: u8 = 0xB6;
pub(crate) const VK_LAUNCH_APP2: u8 = 0xB7;
pub(crate) const VK_OEM_1: u8 = 0xBA;
pub(crate) const VK_OEM_PLUS: u8 = 0xBB;
pub(crate) const VK_OEM_COMMA: u8 = 0xBC;
pub(crate) const VK_OEM_MINUS: u8 = 0xBD;
pub(crate) const VK_OEM_PERIOD: u8 = 0xBE;
pub(crate) const VK_OEM_2: u8 = 0xBF;
pub(crate) const VK_OEM_3: u8 = 0xC0;
pub(crate) const VK_OEM_4: u8 = 0xDB;
pub(crate) const VK_OEM_5: u8 = 0xDC;
pub(crate) const VK_OEM_6: u8 = 0xDD;
pub(crate) const VK_OEM_7: u8 = 0xDE;
pub(crate) const VK_OEM_8: u8 = 0xDF;
pub(crate) const VK_OEM_AX: u8 = 0xE1;
pub(crate) const VK_OEM_102: u8 = 0xE2;
pub(crate) const VK_ICO_HELP: u8 = 0xE3;
pub(crate) const VK_ICO_00: u8 = 0xE4;
pub(crate) const VK_PROCESSKEY: u8 = 0xE5;
pub(crate) const VK_ICO_CLEAR: u8 = 0xE6;
pub(crate) const VK_PACKET: u8 = 0xE7;
pub(crate) const VK_OEM_RESET: u8 = 0xE9;
pub(crate) const VK_OEM_JUMP: u8 = 0xEA;
pub(crate) const VK_OEM_PA1: u8 = 0xEB;
pub(crate) const VK_OEM_PA2: u8 = 0xEC;
pub(crate) const VK_OEM_PA3: u8 = 0xED;
pub(crate) const VK_OEM_WSCTRL: u8 = 0xEE;
pub(crate) const VK_OEM_CUSEL: u8 = 0xEF;
pub(crate) const VK_OEM_ATTN: u8 = 0xF0;
pub(crate) const VK_OEM_FINISH: u8 = 0xF1;
pub(crate) const VK_OEM_COPY: u8 = 0xF2;
pub(crate) const VK_OEM_AUTO: u8 = 0xF3;
pub(crate) const VK_OEM_ENLW: u8 = 0xF4;
pub(crate) const VK_OEM_BACKTAB: u8 = 0xF5;
pub(crate) const VK_ATTN: u8 = 0xF6;
pub(crate) const VK_CRSEL: u8 = 0xF7;
pub(crate) const VK_EXSEL: u8 = 0xF8;
pub(crate) const VK_EREOF: u8 = 0xF9;
pub(crate) const VK_PLAY: u8 = 0xFA;
pub(crate) const VK_ZOOM: u8 = 0xFB;
pub(crate) const VK_NONAME: u8 = 0xFC;
pub(crate) const VK_PA1: u8 = 0xFD;
pub(crate) const VK_OEM_CLEAR: u8 = 0xFE;
pub(crate) const WH_CBT: c_int = 5;
pub(crate) const WM_NULL: UINT = 0x0000;
pub(crate) const WM_CREATE: UINT = 0x0001;
pub(crate) const WM_DESTROY: UINT = 0x0002;
pub(crate) const WM_MOVE: UINT = 0x0003;
pub(crate) const WM_SIZE: UINT = 0x0005;
pub(crate) const WM_ACTIVATE: UINT = 0x0006;
pub(crate) const WM_SETFOCUS: UINT = 0x0007;
pub(crate) const WM_KILLFOCUS: UINT = 0x0008;
pub(crate) const WM_QUIT: UINT = 0x0012;
pub(crate) const WM_ENABLE: UINT = 0x000A;
pub(crate) const WM_CHAR: UINT = 0x0102;
pub(crate) const WM_UNICHAR: UINT = 0x0109;
pub(crate) const WM_DPICHANGED: UINT = 0x02E0;
pub(crate) const WM_SETREDRAW: UINT = 0x000B;
pub(crate) const WM_SETTEXT: UINT = 0x000C;
pub(crate) const WM_PAINT: UINT = 0x000F;
pub(crate) const WM_CLOSE: UINT = 0x0010;
pub(crate) const WM_ERASEBKGND: UINT = 0x0014;
pub(crate) const WM_SHOWWINDOW: UINT = 0x0018;
pub(crate) const WM_ACTIVATEAPP: UINT = 0x001C;
pub(crate) const WM_SETCURSOR: UINT = 0x0020;
pub(crate) const WM_NCCREATE: UINT = 0x0081;
pub(crate) const WM_NCDESTROY: UINT = 0x0082;
pub(crate) const WM_NCLBUTTONDOWN: UINT = 0x00A1;
pub(crate) const WM_KEYDOWN: UINT = 0x0100;
pub(crate) const WM_KEYUP: UINT = 0x0101;
pub(crate) const WM_SYSKEYDOWN: UINT = 0x0104;
pub(crate) const WM_SYSKEYUP: UINT = 0x0105;
pub(crate) const WM_SYSCOMMAND: UINT = 0x0112;
pub(crate) const WM_MOUSEMOVE: UINT = 0x0200;
pub(crate) const WM_MOUSELEAVE: UINT = 0x02A3;
pub(crate) const WM_LBUTTONDOWN: UINT = 0x0201;
pub(crate) const WM_LBUTTONUP: UINT = 0x0202;
pub(crate) const WM_RBUTTONDOWN: UINT = 0x0204;
pub(crate) const WM_RBUTTONUP: UINT = 0x0205;
pub(crate) const WM_MBUTTONDOWN: UINT = 0x0207;
pub(crate) const WM_MBUTTONUP: UINT = 0x0208;
pub(crate) const WM_MOUSEWHEEL: UINT = 0x020A;
pub(crate) const WM_XBUTTONDOWN: UINT = 0x020B;
pub(crate) const WM_XBUTTONUP: UINT = 0x020C;
pub(crate) const WM_MOVING: UINT = 0x0216;
pub(crate) const WM_EXITSIZEMOVE: UINT = 0x0232;
pub(crate) const WM_USER: UINT = 0x0400;
pub(crate) const WS_BORDER: DWORD = 0x00800000;
pub(crate) const WS_CAPTION: DWORD = 0x00C00000;
pub(crate) const WS_CHILD: DWORD = 0x40000000;
pub(crate) const WS_CLIPCHILDREN: DWORD = 0x02000000;
pub(crate) const WS_CLIPSIBLINGS: DWORD = 0x04000000;
pub(crate) const WS_DISABLED: DWORD = 0x08000000;
pub(crate) const WS_DLGFRAME: DWORD = 0x00400000;
pub(crate) const WS_EX_LAYOUTRTL: DWORD = 0x00400000;
pub(crate) const WS_EX_TOOLWINDOW: DWORD = 0x00000080;
pub(crate) const WS_GROUP: DWORD = 0x00020000;
pub(crate) const WS_HSCROLL: DWORD = 0x00100000;
pub(crate) const WS_ICONIC: DWORD = WS_MINIMIZE;
pub(crate) const WS_MAXIMIZE: DWORD = 0x01000000;
pub(crate) const WS_MAXIMIZEBOX: DWORD = 0x00010000;
pub(crate) const WS_MINIMIZE: DWORD = 0x20000000;
pub(crate) const WS_MINIMIZEBOX: DWORD = 0x00020000;
pub(crate) const WS_OVERLAPPED: DWORD = 0x00000000;
pub(crate) const WS_OVERLAPPEDWINDOW: DWORD =
    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;
pub(crate) const WS_POPUP: DWORD = 0x80000000;
pub(crate) const WS_SIZEBOX: DWORD = WS_THICKFRAME;
pub(crate) const WS_SYSMENU: DWORD = 0x00080000;
pub(crate) const WS_TABSTOP: DWORD = 0x00010000;
pub(crate) const WS_THICKFRAME: DWORD = 0x00040000;
pub(crate) const WS_TILED: DWORD = WS_OVERLAPPED;
pub(crate) const WS_TILEDWINDOW: DWORD = WS_OVERLAPPEDWINDOW;
pub(crate) const WS_VISIBLE: DWORD = 0x10000000;
pub(crate) const WS_VSCROLL: DWORD = 0x00200000;
pub(crate) const XBUTTON1: WORD = 0x0001;
pub(crate) const XBUTTON2: WORD = 0x0002;
pub(crate) const TME_LEAVE: DWORD = 2;

// Structs
#[repr(C)]
pub(crate) struct CREATESTRUCTW {
    pub(crate) lpCreateParams: *mut c_void,
    pub(crate) hInstance: HINSTANCE,
    pub(crate) hMenu: HMENU,
    pub(crate) hwndParent: HWND,
    pub(crate) cy: c_int,
    pub(crate) cx: c_int,
    pub(crate) y: c_int,
    pub(crate) x: c_int,
    pub(crate) style: LONG,
    pub(crate) lpszName: *const WCHAR,
    pub(crate) lpszClass: *const WCHAR,
    pub(crate) dwExStyle: DWORD,
}

#[repr(C)]
pub(crate) struct TRACKMOUSEEVENT {
    pub(crate) cbSize: DWORD,
    pub(crate) dwFlags: DWORD,
    pub(crate) hwndTrack: HWND,
    pub(crate) dwHoverTime: DWORD,
}
    
#[repr(C)]
pub(crate) struct IMAGE_DOS_HEADER {
    pub(crate) e_magic: WORD,
    pub(crate) e_cblp: WORD,
    pub(crate) e_cp: WORD,
    pub(crate) e_crlc: WORD,
    pub(crate) e_cparhdr: WORD,
    pub(crate) e_minalloc: WORD,
    pub(crate) e_maxalloc: WORD,
    pub(crate) e_ss: WORD,
    pub(crate) e_sp: WORD,
    pub(crate) e_csum: WORD,
    pub(crate) e_ip: WORD,
    pub(crate) e_cs: WORD,
    pub(crate) e_lfarlc: WORD,
    pub(crate) e_ovno: WORD,
    pub(crate) e_res: [WORD; 4],
    pub(crate) e_oemid: WORD,
    pub(crate) e_oeminfo: WORD,
    pub(crate) e_res2: [WORD; 10],
    pub(crate) e_lfanew: LONG,
}
#[repr(C)]
pub(crate) struct MSG {
    pub(crate) hwnd: HWND,
    pub(crate) message: UINT,
    pub(crate) wParam: WPARAM,
    pub(crate) lParam: LPARAM,
    pub(crate) time: DWORD,
    pub(crate) pt: POINT,
}
#[repr(C)]
pub(crate) struct PAINTSTRUCT {
    hdc: HDC,
    fErase: BOOL,
    rcPaint: RECT,
    fRestore: BOOL,
    fIncUpdate: BOOL,
    rgbReserved: [BYTE; 32],
}
#[repr(C)]
pub(crate) struct POINT {
    pub(crate) x: LONG,
    pub(crate) y: LONG,
}
#[repr(C)]
pub(crate) struct RECT {
    pub(crate) left: LONG,
    pub(crate) top: LONG,
    pub(crate) right: LONG,
    pub(crate) bottom: LONG,
}
#[repr(C)]
pub(crate) struct TITLEBARINFO {
    pub(crate) cbSize: DWORD,
    pub(crate) rcTitleBar: RECT,
    pub(crate) rgstate: [DWORD; CCHILDREN_TITLEBAR + 1],
}
#[repr(C)]
pub(crate) struct WNDCLASSEXW {
    pub(crate) cbSize: UINT,
    pub(crate) style: UINT,
    pub(crate) lpfnWndProc: WNDPROC,
    pub(crate) cbClsExtra: c_int,
    pub(crate) cbWndExtra: c_int,
    pub(crate) hInstance: HINSTANCE,
    pub(crate) hIcon: HICON,
    pub(crate) hCursor: HCURSOR,
    pub(crate) hbrBackground: HBRUSH,
    pub(crate) lpszMenuName: *const WCHAR,
    pub(crate) lpszClassName: *const WCHAR,
    pub(crate) hIconSm: HICON,
}

// Static-linked Functions
#[link(name = "kernel32")]
extern "system" {
    // Global state error code API mess
    pub(crate) fn GetLastError() -> DWORD;
    pub(crate) fn SetLastError(dwErrCode: DWORD);
    pub(crate) fn ExitProcess(uExitCode: UINT);

    pub(crate) fn LoadLibraryExA(lpLibFileName: *const CHAR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE;
    // Threading
    pub(crate) fn GetCurrentThreadId() -> DWORD;

    // String conversion
    pub(crate) fn MultiByteToWideChar(
        CodePage: UINT,
        dwFlags: DWORD,
        lpMultiByteStr: *const CHAR,
        cbMultiByte: c_int,
        lpWideCharStr: *mut WCHAR,
        cchWideChar: c_int,
    ) -> c_int;

    // Threading (because Rust still can't give you thread IDs from a thread handle)
    pub(crate) fn CreateThread(
        lpThreadAttributes: *mut c_void,
        dwStackSize: usize,
        lpStartAddress: unsafe extern "system" fn(param: *mut c_void) -> DWORD,
        lpParameter: *mut c_void,
        dwCreationFlags: DWORD,
        lpThreadId: *mut DWORD,
    ) -> HANDLE;
    pub(crate) fn CreateEventW(
        lpEventAttributes: *mut c_void,
        bManualReset: BOOL,
        bInitialState: BOOL,
        lpName: *const WCHAR,
    ) -> HANDLE;
    pub(crate) fn CloseHandle(handle: HANDLE) -> BOOL;
    pub(crate) fn SetEvent(hEvent: HANDLE) -> BOOL;
    pub(crate) fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;

    // Dynamic linking
    pub(crate) fn GetProcAddress(hModule: HMODULE, lpProcName: *const CHAR) -> FARPROC;
    pub(crate) fn LoadLibraryExW(lpLibFileName: *const WCHAR, hFile: HANDLE, dwFlags: DWORD) -> HMODULE;

    // Note: The kernel treats these `LARGE_INTEGER`s as unsigned
    pub(crate) fn QueryPerformanceCounter(lpPerformanceCount: *mut u64) -> BOOL;
    pub(crate) fn QueryPerformanceFrequency(lpPerformanceCount: *mut u64) -> BOOL;

    // Operating system version
    pub(crate) fn VerSetConditionMask(ConditionMask: c_ulonglong, TypeMask: DWORD, Condition: BYTE) -> c_ulonglong;
}
#[link(name = "user32")]
extern "system" {
    // Window class management
    pub(crate) fn GetClassInfoExW(hinst: HINSTANCE, lpszClass: *const WCHAR, lpwcx: *mut WNDCLASSEXW) -> BOOL;
    pub(crate) fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> ATOM;
    pub(crate) fn SetWindowTextW(hWnd: HWND, lpString: *const WCHAR) -> BOOL;
    pub(crate) fn MonitorFromWindow(hwnd: HWND, dwFlags: DWORD) -> HMONITOR;
    // Window management
    pub(crate) fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: *const WCHAR,
        lpWindowName: *const WCHAR,
        dwStyle: DWORD,
        x: c_int,
        y: c_int,
        nWidth: c_int,
        nHeight: c_int,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *mut c_void,
    ) -> HWND;
    pub(crate) fn AdjustWindowRectEx(lpRect: *mut RECT, dwStyle: DWORD, bMenu: BOOL, dwExStyle: DWORD) -> BOOL;
    pub(crate) fn ClientToScreen(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    pub(crate) fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub(crate) fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub(crate) fn GetTitleBarInfo(hwnd: HWND, pti: *mut TITLEBARINFO) -> BOOL;
    pub(crate) fn SetWindowPos(
        hWnd: HWND,
        hWndInsertAfter: HWND,
        X: c_int,
        Y: c_int,
        cx: c_int,
        cy: c_int,
        uFlags: UINT,
    ) -> BOOL;
    pub(crate) fn WindowFromPoint(Point: POINT) -> HWND;
    pub(crate) fn DestroyWindow(hWnd: HWND) -> BOOL;

    pub(crate) fn TrackMouseEvent(x: *mut TRACKMOUSEEVENT) -> BOOL;

    // Hooking API
    pub(crate) fn CallNextHookEx(hhk: HHOOK, nCode: c_int, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub(crate) fn SetWindowsHookExW(idHook: c_int, lpfn: HOOKPROC, hmod: HINSTANCE, dwThreadId: DWORD) -> HHOOK;
    pub(crate) fn UnhookWindowsHookEx(hhk: HHOOK) -> BOOL;

    // Message loop
    pub(crate) fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub(crate) fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> BOOL;
    pub(crate) fn PeekMessageW(
        lpMsg: *mut MSG, 
        hWnd: HWND, 
        wMsgFilterMin: UINT, 
        wMsgFilterMax: UINT, 
        wRemoveMsg: UINT,
    ) -> BOOL;
    pub(crate) fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub(crate) fn PostThreadMessageW(idThread: DWORD, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub(crate) fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub(crate) fn DispatchMessageW(lpmsg: *const MSG) -> LRESULT;
    pub(crate) fn TranslateMessage(lpmsg: *const MSG) -> LRESULT;
    pub(crate) fn PostQuitMessage(nExitCode: c_int);

    // Message loop utility
    pub(crate) fn ShowWindow(hWnd: HWND, nCmdShow: c_int) -> BOOL;
    pub(crate) fn ShowWindowAsync(hWnd: HWND, nCmdShow: c_int) -> BOOL;

    // Keyboard & mouse related
    pub(crate) fn ClipCursor(lpRect: *const RECT) -> BOOL;
    pub(crate) fn SetCursorPos(X: c_int, Y: c_int) -> BOOL;
    pub(crate) fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub(crate) fn SetCapture(hWnd: HWND) -> HWND;
    pub(crate) fn GetCapture() -> HWND;
    pub(crate) fn ReleaseCapture() -> BOOL;
    pub(crate) fn GetAsyncKeyState(vKey: c_int) -> SHORT;
    pub(crate) fn GetSystemMetrics(nIndex: c_int) -> c_int;
    pub(crate) fn SetCursor(hCursor: HCURSOR) -> HCURSOR;
    pub(crate) fn MapVirtualKeyW(hCode: UINT, uMapType: UINT) -> UINT;

    // Misc legacy garbage
    pub(crate) fn EnableMenuItem(hMenu: HMENU, uIDEnableItem: UINT, uEnable: UINT) -> BOOL;
    pub(crate) fn GetSystemMenu(hWnd: HWND, bRevert: BOOL) -> HMENU;

    // Yeah, whatever
    pub(crate) fn LoadImageW(
        hInst: HINSTANCE,
        name: *const WCHAR,
        type_: UINT,
        cx: c_int,
        cy: c_int,
        fuLoad: UINT,
    ) -> HANDLE;
    pub(crate) fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    pub(crate) fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;

    // Class/instance storage manipulation
    #[cfg(target_pointer_width = "32")]
    pub(crate) fn GetClassLongW(hWnd: HWND, nIndex: c_int) -> DWORD;
    #[cfg(target_pointer_width = "32")]
    pub(crate) fn SetClassLongW(hWnd: HWND, nIndex: c_int, dwNewLong: LONG) -> DWORD;
    #[cfg(target_pointer_width = "32")]
    pub(crate) fn GetWindowLongW(hWnd: HWND, nIndex: c_int) -> LONG;
    #[cfg(target_pointer_width = "32")]
    pub(crate) fn SetWindowLongW(hWnd: HWND, nIndex: c_int, dwNewLong: LONG) -> LONG;
    #[cfg(target_pointer_width = "64")]
    pub(crate) fn GetClassLongPtrW(hWnd: HWND, nIndex: c_int) -> ULONG_PTR;
    #[cfg(target_pointer_width = "64")]
    pub(crate) fn SetClassLongPtrW(hWnd: HWND, nIndex: c_int, dwNewLong: LONG_PTR) -> ULONG_PTR;
    #[cfg(target_pointer_width = "64")]
    pub(crate) fn GetWindowLongPtrW(hWnd: HWND, nIndex: c_int) -> LONG_PTR;
    #[cfg(target_pointer_width = "64")]
    pub(crate) fn SetWindowLongPtrW(hWnd: HWND, nIndex: c_int, dwNewLong: LONG_PTR) -> LONG_PTR;
}

// Wrapper for the API that manipulates window class and instance storage.
//
// History lesson: `SetClassLongW` and friends all took `LONG`, a 32-bit type.
// Only, when Micro$oft upgraded from 32 to 64 bit, they realised pointers needed to fit into them.
// They added a new set of functions, `LongPtr` taking `LONG_PTR`, but *only available for 64-bit*.
// Their solution for using those functions on 32-bit was to define C preprocessor macros, like so:
// #define SetClassLongPtrW SetClassLongW
// The problem is that the signatures are incompatible in a language with a good integer type system.
// These functions wrap both function sets to `usize`, which lines up with the intended sizes.
#[cfg(target_pointer_width = "32")]
#[inline]
pub(crate) unsafe fn class_storage(hwnd: HWND, offset: c_int) -> usize {
    GetClassLongW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "64")]
#[inline]
pub(crate) unsafe fn class_storage(hwnd: HWND, offset: c_int) -> usize {
    GetClassLongPtrW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "32")]
#[inline]
pub(crate) unsafe fn set_class_storage(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetClassLongW(hwnd, offset, data as LONG) as usize
}
#[cfg(target_pointer_width = "64")]
#[inline]
pub(crate) unsafe fn set_class_storage(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetClassLongPtrW(hwnd, offset, data as LONG_PTR) as usize
}
#[cfg(target_pointer_width = "32")]
#[inline]
pub(crate) unsafe fn instance_storage(hwnd: HWND, offset: c_int) -> usize {
    GetWindowLongW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "64")]
#[inline]
pub(crate) unsafe fn instance_storage(hwnd: HWND, offset: c_int) -> usize {
    GetWindowLongPtrW(hwnd, offset) as usize
}
#[cfg(target_pointer_width = "32")]
#[inline]
pub(crate) unsafe fn set_instance_storage(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetWindowLongW(hwnd, offset, data as LONG) as usize
}
#[cfg(target_pointer_width = "64")]
#[inline]
pub(crate) unsafe fn set_instance_storage(hwnd: HWND, offset: c_int, data: usize) -> usize {
    SetWindowLongPtrW(hwnd, offset, data as LONG_PTR) as usize
}


#[repr(C)]
pub(crate) struct OSVERSIONINFOEXW {
    pub(crate) dwOSVersionInfoSize: DWORD,
    pub(crate) dwMajorVersion: DWORD,
    pub(crate) dwMinorVersion: DWORD,
    pub(crate) dwBuildNumber: DWORD,
    pub(crate) dwPlatformId: DWORD,
    pub(crate) szCSDVersion: [WCHAR; 128],
    pub(crate) wServicePackMajor: WORD,
    pub(crate) wServicePackMinor: WORD,
    pub(crate) wSuiteMask: WORD,
    pub(crate) wProductType: BYTE,
    pub(crate) wReserved: BYTE,
}


/// Generates simple dynamic linkings.
macro_rules! dyn_link {
    (
        $(#[$outer:meta])*
        $s_vis:vis struct $s_ident:ident($dlopen:expr => $dlopen_ty:ty | $dlsym:expr) {
            $($($module_name:literal)|+ {
                $(
                    $(#[$fn_outer:meta])*
                    fn $sym_fn:ident($($name:ident : $ty:ty),*$(,)?) -> $ret:ty;
                )*
            }),* $(,)?
        }
    ) => {
        $(#[$outer])*
        pub(crate) struct $s_ident {
            $($(
                $(#[$fn_outer])*
                $sym_fn : ::std::option::Option<
                    unsafe extern "system" fn($($name : $ty ,)*) -> $ret
                > ,
            )*)*
        }

        impl $s_ident {
            #[allow(unused_doc_comments)]
            unsafe fn _link() -> Self {
                let mut inst = ::std::mem::MaybeUninit::<Self>::uninit();
                let mut inst_ref = &mut *(inst.as_mut_ptr());
                $(
                    let mut handle = 0 as $dlopen_ty;
                    for name in &[$(cstr!($module_name) ,)*] {
                        handle = $dlopen(*name);
                        if handle != (0 as $dlopen_ty) {
                            break
                        }
                    }
                    if handle != (0 as $dlopen_ty) {
                        $(
                            $(#[$fn_outer])*
                            {
                                inst_ref.$sym_fn =
                                    ::std::mem::transmute::<_,
                                        Option<unsafe extern "system" fn($($name : $ty ,)*) -> $ret>>
                                    ($dlsym(handle, cstr!(stringify!($sym_fn))));
                            }
                        )*
                    } else {
                        $(
                            $(#[$fn_outer])*
                            {
                                inst_ref.$sym_fn = None;
                            }
                        )*
                    }
                )*
                inst.assume_init()
            }
            $($(
                $(#[$fn_outer])*
                $s_vis unsafe fn $sym_fn(&self, $($name : $ty ,)*) -> ::std::option::Option<$ret> {
                    self.$sym_fn.map(|f| f($($name ,)*))
                }
            )*)*
        }
    };
}

#[inline]
unsafe fn dlopen(name: *const CHAR) -> HMODULE {
    // Patch loading mechanism here, if you wish
    LoadLibraryExA(name, 0 as HANDLE, 0)
}

dyn_link! {
    pub(crate) struct Win32DL(dlopen => HMODULE | GetProcAddress) {
        "Dwmapi.dll" {
            /// (Windows Vista+)
            /// Advanced querying of window attributes via the desktop window manager.
            fn DwmGetWindowAttribute(
                hWnd: HWND,
                dwAttribute: DWORD,
                pvAttribute: *mut c_void,
                cbAttribute: DWORD,
            ) -> HRESULT;

            /// (Windows Vista+)
            /// Advanced setting of window attributes via the desktop window manager.
            fn DwmSetWindowAttribute(
                hWnd: HWND,
                dwAttribute: DWORD,
                pvAttribute: *const c_void,
                cbAttribute: DWORD,
            ) -> HRESULT;
        },

        "Ntdll.dll" {
            /// (Win2000+)
            /// This is used in place of VerifyVersionInfoW, as it's not manifest dependent, and doesn't lie.
            fn RtlVerifyVersionInfo(
                VersionInfo: *mut OSVERSIONINFOEXW,
                TypeMask: DWORD,
                ConditionMask: c_ulonglong,
            ) -> NTSTATUS;
        },

        "Shcore.dll" {
            /// (Win8.1+)
            /// The intended way to query a monitor's DPI values since PMv1 and above.
            fn GetDpiForMonitor(
                hmonitor: HMONITOR,
                dpiType: u32,
                dpiX: *mut UINT,
                dpiY: *mut UINT,
            ) -> HRESULT;
        },

        "User32.dll" {
            // (Win10 1607+)
            // It's a version of AdjustWindowRectEx with DPI, but they added it 7 years late.
            // The DPI parameter accounts for scaled non-client areas, not to scale client areas.
            fn AdjustWindowRectExForDpi(
                lpRect: *mut RECT,
                dwStyle: DWORD,
                bMenu: BOOL,
                dwExStyle: DWORD,
                dpi: UINT,
            ) -> BOOL;

            /// (Win10 1603+)
            /// Enables automatic scaling of the non-client area as a hack for PMv1 DPI mode.
            fn EnableNonClientDpiScaling(hwnd: HWND) -> BOOL;

            /// (Vista+)
            /// First introduction of DPI awareness, this function enables System-Aware DPI.
            fn SetProcessDPIAware() -> BOOL;

            /// (Win8.1+)
            /// Allows you to set either System-Aware DPI mode, or Per-Monitor-Aware (v1).
            fn SetProcessDpiAwareness(value: PROCESS_DPI_AWARENESS) -> HRESULT;

            /// (Win10 1703+)
            /// Allows you to set either System-Aware DPI mode, or Per-Monitor-Aware (v1 *or* v2).
            fn SetProcessDpiAwarenessContext(value: DPI_AWARENESS_CONTEXT) -> BOOL;
        },
    }
}

impl Win32DL {
    pub(crate) unsafe fn link() -> Self {
        // Trying to load a nonexistent dynamic library or symbol sets the thread-global error.
        // Since this is intended and acceptable for missing functions, we restore the error state.

        let prev_error = GetLastError();
        let instance = Self::_link();
        SetLastError(prev_error);
        instance
    }
}
