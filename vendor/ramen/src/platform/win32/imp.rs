//! TODO explain rationale behind creating a thread for this

#![allow(clippy::mutex_atomic)]

use super::ffi::*;

use crate::{
    connection,
    error::Error,
    event::Event,
    util::{sync::{self, Condvar, Mutex}, LazyCell},
    window::{self, Cursor, Style},
};

#[cfg(feature = "input")]
use crate::input::{Key, MouseButton};

use std::{cell::UnsafeCell, mem, ptr};

/// TODO: yeah
/// 
/// 
// Global immutable struct containing dynamically acquired API state
static WIN32: LazyCell<Win32State> = LazyCell::new(Win32State::new);

const BASE_DPI: UINT = 96;
/// Custom window message
const RAMEN_WM_CREATE: UINT = WM_USER + 0;
const RAMEN_WM_DROP: UINT = WM_USER + 1;
const RAMEN_WM_SETCURSOR: UINT = WM_USER + 2;

/// Checks the current Windows version (see usage in `Win32State`)
unsafe fn is_windows_ver_or_greater(dl: &Win32DL, major: WORD, minor: WORD, sp_major: WORD) -> bool {
    let mut osvi: OSVERSIONINFOEXW = mem::zeroed();
    osvi.dwOSVersionInfoSize = mem::size_of_val(&osvi) as DWORD;
    osvi.dwMajorVersion = major.into();
    osvi.dwMinorVersion = minor.into();
    osvi.wServicePackMajor = sp_major;

    let mask = VER_MAJORVERSION | VER_MINORVERSION | VER_SERVICEPACKMAJOR;
    let mut cond = VerSetConditionMask(0, VER_MAJORVERSION, VER_GREATER_EQUAL);
    cond = VerSetConditionMask(cond, VER_MINORVERSION, VER_GREATER_EQUAL);
    cond = VerSetConditionMask(cond, VER_SERVICEPACKMAJOR, VER_GREATER_EQUAL);

    dl.RtlVerifyVersionInfo(&mut osvi, mask, cond) == Some(0)
}

/// Checks a specific Windows 10 update level (see usage in `Win32State`)
unsafe fn is_win10_ver_or_greater(dl: &Win32DL, build: WORD) -> bool {
    let mut osvi: OSVERSIONINFOEXW = mem::zeroed();
    osvi.dwOSVersionInfoSize = mem::size_of_val(&osvi) as DWORD;
    osvi.dwMajorVersion = 10;
    osvi.dwMinorVersion = 0;
    osvi.dwBuildNumber = build.into();

    let mask = VER_MAJORVERSION | VER_MINORVERSION | VER_BUILDNUMBER;
    let mut cond = VerSetConditionMask(0, VER_MAJORVERSION, VER_GREATER_EQUAL);
    cond = VerSetConditionMask(cond, VER_MINORVERSION, VER_GREATER_EQUAL);
    cond = VerSetConditionMask(cond, VER_BUILDNUMBER, VER_GREATER_EQUAL);

    dl.RtlVerifyVersionInfo(&mut osvi, mask, cond) == Some(0)
}

struct Win32State {
    /// Whether the system is at least on Windows 10 1607 (build 14393 - "Anniversary Update").
    at_least_anniversary_update: bool,

    /// The DPI mode that's enabled process-wide. The newest available is selected.
    /// MSDN recommends setting this with the manifest but that's rather unpleasant.
    /// Instead, it's set dynamically at runtime when this struct is instanced.
    dpi_mode: Win32DpiMode,

    /// Dynamically linked Win32 functions that might not be available on all systems.
    dl: Win32DL,
}

#[derive(PartialEq)]
enum Win32DpiMode {
    Unsupported,
    System,
    PerMonitorV1,
    PerMonitorV2,
}

impl Win32State {
    fn new() -> Self {
        const VISTA_MAJ: WORD = (_WIN32_WINNT_VISTA >> 8) & 0xFF;
        const VISTA_MIN: WORD = _WIN32_WINNT_VISTA & 0xFF;
        const W81_MAJ: WORD = (_WIN32_WINNT_WINBLUE >> 8) & 0xFF;
        const W81_MIN: WORD = _WIN32_WINNT_WINBLUE & 0xFF;

        unsafe {
            let dl = Win32DL::link();

            let at_least_vista = is_windows_ver_or_greater(&dl, VISTA_MAJ, VISTA_MIN, 0);
            let at_least_8_point_1 = is_windows_ver_or_greater(&dl, W81_MAJ, W81_MIN, 0);
            let at_least_anniversary_update = is_win10_ver_or_greater(&dl, 14393);
            let at_least_creators_update = is_win10_ver_or_greater(&dl, 15063);

            let dpi_mode = if at_least_creators_update {
                let _ = dl.SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                Win32DpiMode::PerMonitorV2
            } else if at_least_8_point_1 {
                let _ = dl.SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
                Win32DpiMode::PerMonitorV1
            } else if at_least_vista {
                let _ = dl.SetProcessDPIAware();
                Win32DpiMode::System
            } else {
                Win32DpiMode::Unsupported
            };

            Self {
                at_least_anniversary_update,
                dpi_mode,
                dl,
            }
        }
    }
}

/// Win32 functions need the full outer size for creation. This function calculates that size from an inner size.
///
/// Since for legacy reasons things like drop shadow are part of the bounds, don't use this for reporting outer size.
unsafe fn adjust_window_for_dpi(
    win32: &Win32State,
    (width, height): (u16, u16),
    style: DWORD,
    style_ex: DWORD,
    dpi: UINT,
) -> ((LONG, LONG), RECT) {
    let mut window = RECT { left: 0, top: 0, right: width as LONG, bottom: height as LONG };
    if match win32.dpi_mode {
        // Non-client area DPI scaling is enabled in PMv1 Win10 1607+ and PMv2 (any).
        // For PMv1, this is done with EnableNonClientDpiScaling at WM_NCCREATE.
        Win32DpiMode::PerMonitorV1 if win32.at_least_anniversary_update => true,
        Win32DpiMode::PerMonitorV2 => true,
        _ => false,
    } {
        let _ = win32.dl.AdjustWindowRectExForDpi(&mut window, style, FALSE, style_ex, dpi);
    } else {
        // TODO: This *is* correct for old PMv1, right? How does broken NC scaling work?
        let _ = AdjustWindowRectEx(&mut window, style, FALSE, style_ex);
    }
    (rect_to_size2d(&window), window)
}

#[inline]
fn rect_to_size2d(rect: &RECT) -> (LONG, LONG) {
    (rect.right - rect.left, rect.bottom - rect.top)
}

pub(crate) struct Connection {
    id: DWORD,
    handle: HANDLE,
}

unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}

impl Connection {
    pub(crate) fn new() -> Result<Self, Error> {
        unsafe {
            let event = CreateEventW(ptr::null_mut(), 0, 0, ptr::null());
            if event.is_null() {
                return Err(Error::SystemResources);
            }
            let mut id: DWORD = 0;
            let handle = CreateThread(ptr::null_mut(), 0, connection_proc, event as _, 0, &mut id);
            if handle.is_null() {
                return Err(Error::SystemResources);
            }
            assert!(WaitForSingleObject(event, INFINITE) == 0);
            let _ = CloseHandle(event);
            Ok(Self { id, handle })
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.id, WM_QUIT, 0, 0);
            let _ = WaitForSingleObject(self.handle, INFINITE);
        }
    }
}

unsafe extern "system" fn connection_proc(fparam: *mut c_void) -> DWORD {
    let mut msg = mem::MaybeUninit::zeroed();

    // force creating message queue, signal ready (won't consume message)
    let _ = PeekMessageW(msg.as_mut_ptr(), ptr::null_mut(), 0, 0, PM_NOREMOVE);
    let _ = SetEvent(fparam as HANDLE);

    'message_loop: loop {
        let success = GetMessageW(msg.as_mut_ptr(), ptr::null_mut(), 0, 0);
        if success > 0 && success != -1 {
            let message = &*msg.as_ptr();
            if message.hwnd.is_null() {
                // connection message
                if message.message == RAMEN_WM_CREATE {
                    let (cvar, mutex) = &*(message.wParam as *const (Condvar, Mutex<Option<Result<HWND, Error>>>));
                    let csw = &*(message.lParam as *const CREATESTRUCTW);
                    let mut reply = sync::mutex_lock(&mutex);

                    static CLASS_REGISTRY_GUARD: Mutex<()> = Mutex::new(());
                    let mut class = mem::MaybeUninit::<WNDCLASSEXW>::uninit();
                    let class_ptr = class.as_mut_ptr();
                    (*class_ptr).cbSize = mem::size_of_val(&class) as UINT;
                    let class_guard = sync::mutex_lock(&CLASS_REGISTRY_GUARD);
                    let class_created_here = GetClassInfoExW(base_hinstance(), csw.lpszClass, class_ptr) == 0;
                    if class_created_here {
                        // Failure sets a global error code, but we don't care, we know the error
                        SetLastError(ERROR_SUCCESS);

                        // Fill in the default state and register the class.
                        let class = &mut *class_ptr;
                        class.style = CS_OWNDC;
                        class.lpfnWndProc = window_proc;
                        class.hIcon = ptr::null_mut();
                        class.hCursor = ptr::null_mut();
                        class.hbrBackground = ptr::null_mut();
                        class.lpszMenuName = ptr::null_mut();
                        // TODO Filter reserved class names
                        class.lpszClassName = csw.lpszClass;
                        class.hIconSm = ptr::null_mut();

                        class.cbClsExtra = mem::size_of::<usize>() as c_int;
                        class.cbWndExtra = 0;

                        // A handle to the executable base is given so the OS knows how to free it.
                        class.hInstance = base_hinstance();

                        if RegisterClassExW(class) == 0 {
                            *reply = Some(Err(Error::SystemResources));
                            sync::cvar_notify_one(&cvar);
                            continue 'message_loop;
                        }
                    }
                    mem::drop(class_guard);

                    let hwnd = CreateWindowExW(
                        csw.dwExStyle,
                        csw.lpszClass,
                        csw.lpszName,
                        csw.style as _,
                        csw.x,
                        csw.y,
                        csw.cx,
                        csw.cy,
                        csw.hwndParent,
                        csw.hMenu,
                        csw.hInstance,
                        csw.lpCreateParams,
                    );
                    if hwnd.is_null() {
                        // there's probably many reasons, but...
                        *reply = Some(Err(Error::SystemResources));
                        sync::cvar_notify_one(&cvar);
                        continue 'message_loop;
                    }
                    *reply = Some(Ok(hwnd));
                    sync::cvar_notify_one(&cvar);
                }
            } else {
                // window message
                let _ = TranslateMessage(msg.as_ptr());
                let _ = DispatchMessageW(msg.as_ptr());
            }
        } else if success == 0 {
            break 'message_loop;
        } else {
            // TODO: poison connection
            panic!("oh no");
        }
    }
    0
}

/// Retrieves the base module [`HINSTANCE`].
#[inline]
pub fn base_hinstance() -> HINSTANCE {
    extern "system" {
        // Microsoft's linkers provide a static `HINSTANCE` to not have to query it at runtime.
        // Doing it this way is also much more predictable for dynamic and static linked libraries.
        // See: https://devblogs.microsoft.com/oldnewthing/20041025-00/?p=37483
        static __ImageBase: IMAGE_DOS_HEADER;
    }
    (unsafe { &__ImageBase }) as *const IMAGE_DOS_HEADER as HINSTANCE
}

/// Converts a &str to a Windows wide-string (for `LPCWSTR` parameters).
///
/// If the length is 0 then no allocation is made, you get a pointer to static memory.
fn str_to_wstr(src: &str, buffer: &mut Vec<WCHAR>) -> Option<*const WCHAR> {
    // MultiByteToWideChar can't actually handle zero-length because 0 means error.
    // The boundaries of `c_int` are checked here too since it's cast below a few times.
    if src.is_empty() || src.len() >= c_int::max_value() as usize {
        return Some([0x00].as_ptr())
    }
    let src_len = src.len() as c_int;

    unsafe {
        // calculate buffer size
        // +1 for null terminator (that we add ourselves)
        let req_buffer_size = MultiByteToWideChar(CP_UTF8 as _, 0, src.as_ptr().cast(), src_len, ptr::null_mut(), 0) + 1;

        // ensure buffer capacity
        buffer.clear();
        buffer.try_reserve(req_buffer_size as usize).ok()?;

        // write to destination buffer
        let chars_written =
            MultiByteToWideChar(CP_UTF8 as _, 0, src.as_ptr().cast(), src_len, buffer.as_mut_ptr(), req_buffer_size)
                as usize;

        // append null terminator
        *buffer.as_mut_ptr().add(chars_written) = 0x00;

        // mark buffer as initialised
        buffer.set_len(req_buffer_size as usize);
        Some(buffer.as_ptr())
    }
}

/// Turns a `Style` into a dwStyle and dwExStyle.
/// This does not include the close button, see `set_close_button`.
fn style_to_bits(style: &window::Style) -> (DWORD, DWORD) {
    let window::Style {
        borderless,
        controls,
        resizable,
        visible,
    } = *style;

    let (mut style, style_ex) = (0, 0);

    if borderless {
        style |= WS_POPUP;
    } else {
        style |= WS_OVERLAPPED | WS_BORDER | WS_CAPTION;
    }
    if let Some(controls) = controls {
        if controls.minimise {
            style |= WS_MINIMIZEBOX;
        }
        if controls.maximise && resizable {
            style |= WS_MAXIMIZEBOX;
        }
        style |= WS_SYSMENU;
    }
    if resizable {
        style |= WS_THICKFRAME;
    }
    if visible {
        style |= WS_VISIBLE;
    }

    (style, style_ex)
}

/// Due to legacy reasons, the close button is a system menu item and not a window style.
unsafe fn set_close_button(hwnd: HWND, enabled: bool) {
    let menu: HMENU = GetSystemMenu(hwnd, FALSE);
    let flag = if enabled {
        MF_BYCOMMAND | MF_ENABLED
    } else {
        MF_BYCOMMAND | MF_DISABLED | MF_GRAYED
    };
    let _ = EnableMenuItem(menu, SC_CLOSE as UINT, flag);
}

pub(crate) struct Window {
    _connection: connection::Connection,
    hwnd: HWND,
    state: Box<UnsafeCell<WindowState>>,
}
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

/// Parameters sent to `WM_NCCREATE` and `WM_CREATE`.
struct WindowCreateParams {
    state: *mut WindowState,
    wh: (u16, u16),
}

/// Volatile state which the `Window` and its thread both have a pointer to.
struct WindowState {
    event_backbuf: Vec<Event>,
    event_frontbuf: Vec<Event>,
    event_sync: Mutex<()>,
    mouse_tracked: bool,
    cursor: HCURSOR,
    dpi: UINT,
    is_max: bool,
    is_min: bool,
    style: Style,
    wh: (u16, u16),
}

fn cursor_to_int_resource(cursor: Cursor) -> *const WCHAR {
    match cursor {
        Cursor::Arrow => IDC_ARROW,
        Cursor::Blank => ptr::null(),
        Cursor::Cross => IDC_CROSS,
        Cursor::Hand => IDC_HAND,
        Cursor::Help => IDC_HELP,
        Cursor::IBeam => IDC_IBEAM,
        Cursor::Progress => IDC_APPSTARTING,
        Cursor::ResizeNESW => IDC_SIZENESW,
        Cursor::ResizeNS => IDC_SIZENS,
        Cursor::ResizeNWSE => IDC_SIZENWSE,
        Cursor::ResizeWE => IDC_SIZEWE,
        Cursor::ResizeAll => IDC_SIZEALL,
        Cursor::Unavailable => IDC_NO,
        Cursor::Wait => IDC_WAIT,
    }
}

unsafe fn make_window(builder: window::Builder) -> Result<Window, Error> {
    let mut class_name_wstr = Vec::<WCHAR>::new();
    let class_name = str_to_wstr(&*builder.class_name, class_name_wstr.as_mut())
        .ok_or(Error::OutOfMemory)?;

    let mut title_wstr = Vec::new();
    let title_name = str_to_wstr(&*builder.title, &mut title_wstr).ok_or(Error::OutOfMemory)?;

    let style = builder.style;
    let (dw_style, dw_style_ex) = style_to_bits(&style);
    let dpi = BASE_DPI;
    let ((width, height), wrect) = adjust_window_for_dpi(WIN32.get(), builder.size, dw_style, dw_style_ex, dpi);
    let (pos_x, pos_y) = builder.position.map(|(x, y)| (x as LONG + wrect.left, y as LONG + wrect.top)).unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));
    let window_state = Box::new(UnsafeCell::new(WindowState {
        event_backbuf: Vec::new(),
        event_frontbuf: Vec::new(),
        event_sync: Mutex::new(()),
        mouse_tracked: false,
        cursor: {
            let rsrc = cursor_to_int_resource(builder.cursor);
            if !rsrc.is_null() {
                LoadImageW(ptr::null_mut(), rsrc, IMAGE_CURSOR, 0, 0, LR_DEFAULTSIZE | LR_SHARED).cast()
            } else {
                ptr::null_mut()
            }
        },
        dpi,
        is_max: false,
        is_min: false,
        style: builder.style,
        wh: builder.size,
    }));

    let _ = (&*window_state.get()).mouse_tracked;

    let create_params = WindowCreateParams {
        state: window_state.get(),
        wh: builder.size,
    };

    let hwnd = {
        let csw = CREATESTRUCTW {
            lpCreateParams: (&create_params) as *const WindowCreateParams as *mut c_void,
            hInstance: base_hinstance(),
            hMenu: ptr::null_mut(),
            hwndParent: ptr::null_mut(),
            x: pos_x,
            y: pos_y,
            cx: width,
            cy: height,
            style: dw_style as _,
            lpszName: title_name,
            lpszClass: class_name,
            dwExStyle: dw_style_ex,
        };
        let response = (Condvar::new(), Mutex::<Option<Result<HWND, Error>>>::new(None));
        let conn = sync::mutex_lock(&builder.connection.0);
        let _ = PostThreadMessageW(conn.id, RAMEN_WM_CREATE, &response as *const _ as _, &csw as *const _ as _);
        let (cvar, mutex) = &response;
        let mut lock = sync::mutex_lock(mutex);
        'reply: loop {
            if let Some(result) = (&mut *lock).take() {
                break 'reply result;
            } else {
                sync::cvar_wait(cvar, &mut lock);
            }
        }
    }?;

    set_close_button(hwnd, style.controls.as_ref().map(|x| x.close).unwrap_or(false));
    if builder.maximised {
        let _ = ShowWindow(hwnd, 3);
    }

    Ok(Window {
        _connection: builder.connection,
        hwnd,
        state: window_state,
    })
}

impl Window {
    pub(crate) fn new(builder: window::Builder) -> Result<Self, Error> {
        unsafe { make_window(builder) }
    }

    pub(crate) fn events(&self) -> &[Event] {
        unsafe {
            // safety: `poll_events`'s signature invalidates this reference
            (&*self.state.get()).event_frontbuf.as_slice()
        }
    }

    pub(crate) fn poll_events(&mut self) {
        unsafe {
            let state = &mut *self.state.get();
            let guard = sync::mutex_lock(&state.event_sync);
            state.event_frontbuf.clear();
            mem::swap(&mut state.event_frontbuf, &mut state.event_backbuf);
            mem::drop(guard);
        }
    }

    pub(crate) fn set_borderless(&self, borderless: bool) {
        unsafe {
            let state = &mut *user_state(self.hwnd);
            let _g = sync::mutex_lock(&state.event_sync);
            state.style.borderless = borderless;
            let (dw_style, dw_style_ex) = style_to_bits(&state.style);
            std::mem::drop(_g);
            let _ = set_instance_storage(self.hwnd, GWL_STYLE, dw_style as _);
            let _ = set_instance_storage(self.hwnd, GWL_EXSTYLE, dw_style_ex as _);
            ping_window_frame(self.hwnd);
        }
    }

    pub(crate) fn set_cursor(&self, cursor: Cursor) {
        unsafe {
            _ = SendMessageW(self.hwnd, RAMEN_WM_SETCURSOR, cursor as u32 as WPARAM, 0);
        }
    }

    pub(crate) fn set_resizable(&self, resizable: bool) {
        unsafe {
            let state = &mut *user_state(self.hwnd);
            let _g = sync::mutex_lock(&state.event_sync);
            state.style.resizable = resizable;
            let (dw_style, dw_style_ex) = style_to_bits(&state.style);
            std::mem::drop(_g);
            let _ = set_instance_storage(self.hwnd, GWL_STYLE, dw_style as _);
            let _ = set_instance_storage(self.hwnd, GWL_EXSTYLE, dw_style_ex as _);
            ping_window_frame(self.hwnd);
        }
    }

    pub(crate) fn set_maximised(&self, maximised: bool) {
        unsafe {
            let state = &*self.state.get();
            if state.is_max != maximised {
                let _ = ShowWindow(self.hwnd, if maximised {3} else {1});
            }
        }
    }

    pub(crate) fn set_title(&self, title: &str) {
        unsafe {
            let mut name_wstr = Vec::<WCHAR>::new();
            let name = str_to_wstr(title, name_wstr.as_mut()).unwrap();
            let _ = SetWindowTextW(self.hwnd, name);
        }
    }

    pub(crate) fn set_position(&self, (x, y): (i16, i16)) {
        unsafe {
            let state = &*self.state.get();
            let _g = sync::mutex_lock(&state.event_sync);
            let (dw_style, dw_style_ex) = style_to_bits(&state.style);
            std::mem::drop(_g);
            let (_, RECT { top, left, .. }) = adjust_window_for_dpi(WIN32.get(), (0, 0), dw_style, dw_style_ex, state.dpi);
            let xx = x + left as i16;
            let yy = y + top as i16;
            let _ = SetWindowPos(self.hwnd, ptr::null_mut(), xx as _, yy as _, 0, 0, SWP_NOSIZE);
        }
    }

    pub(crate) fn set_size(&self, (w, h): (u16, u16)) {
        unsafe {
            let state = &*self.state.get();
            let _g = sync::mutex_lock(&state.event_sync);
            let (dw_style, dw_style_ex) = style_to_bits(&state.style);
            let ((width, height), _) = adjust_window_for_dpi(WIN32.get(), (w, h), dw_style, dw_style_ex, state.dpi);
            std::mem::drop(_g);
            let _ = SetWindowPos(self.hwnd, ptr::null_mut(), 0, 0, width as _, height as _, SWP_NOMOVE);
        }
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        unsafe {
            let _ = ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    pub(crate) fn hwnd(&self) -> HWND {
        self.hwnd
    }
}

impl WindowState {
    fn dispatch_event(&mut self, event: Event) {
        let guard = sync::mutex_lock(&self.event_sync);
        self.event_backbuf.push(event);
        mem::drop(guard);
    }
}

/// Returns a pointer to the `WindowState` for a ramen window.
#[inline]
unsafe fn user_state(hwnd: HWND) -> *mut WindowState {
    instance_storage(hwnd, GWL_USERDATA) as *mut WindowState
}

#[cfg(feature = "input")]
fn extend_key(key: Key, lparam: LPARAM) -> Key {
    let scancode = (lparam & 0x00FF0000) >> 16u8;
    let extended_bit = (lparam & (1 << 24)) != 0;

    match key {
        Key::LeftShift if scancode == 54 => Key::RightShift,
        Key::LeftControl if extended_bit => Key::RightControl,
        Key::LeftAlt if extended_bit => Key::RightAlt,
        x => x,
    }
}

#[cfg(feature = "input")]
fn map_tr_state(key: Key, lparam: LPARAM) -> Event {
    if (lparam & (1 << 31)) == 0 {
        if (lparam & (1 << 30)) != 0 {
            Event::KeyboardRepeat(key)
        } else {
            Event::KeyboardDown(key)
        }
    } else {
        Event::KeyboardUp(key)
    }
}

#[cfg(feature = "input")]
fn sys_key_event(wparam: WPARAM, lparam: LPARAM) -> Option<Event> {
    let alt_bit = (lparam & (1 << 29)) != 0;
    let transition_state = (lparam & (1 << 31)) != 0;

    // If it's not an F10 press, and the alt bit is not set,
    // and it's not a key release, it is not being sent by a key.
    if wparam & 0xFF != VK_F10 as WPARAM && !alt_bit && !transition_state {
        return None
    }

    let virtual_key = translate_vk(wparam)?;
    let key = extend_key(virtual_key, lparam);

    Some(map_tr_state(key, lparam))
}

#[cfg(feature = "input")]
fn translate_vk(wparam: WPARAM) -> Option<Key> {
    match (wparam & 0xFF) as u8 {
        // Undocumented.
        0x00 => None,

        // These are not keys.
        VK_LBUTTON | VK_RBUTTON | VK_CANCEL | VK_MBUTTON | VK_XBUTTON1 | VK_XBUTTON2 => None,

        // Undefined.
        0x07 => None,

        VK_BACK => Some(Key::Backspace),
        VK_TAB => Some(Key::Tab),

        // Reserved.
        0x0A..=0x0B => None,

        VK_CLEAR => Some(Key::Clear),
        VK_RETURN => Some(Key::Return),

        // Undefined.
        0x0E..=0x0F => None,

        VK_SHIFT => Some(Key::LeftShift),
        VK_CONTROL => Some(Key::LeftControl),
        VK_MENU => Some(Key::LeftAlt),
        VK_PAUSE => Some(Key::Pause),
        VK_CAPITAL => Some(Key::CapsLock),
        VK_KANA => Some(Key::ImeKanaOrHangul),
        VK_IME_ON => Some(Key::ImeOn),
        VK_JUNJA => Some(Key::ImeJunja),
        VK_FINAL => Some(Key::ImeFinal),
        VK_KANJI => Some(Key::ImeHanjaOrKanji),
        VK_IME_OFF => Some(Key::ImeOff),
        VK_ESCAPE => Some(Key::Escape),
        VK_CONVERT => Some(Key::ImeConvert),
        VK_NONCONVERT => Some(Key::ImeNonConvert),
        VK_ACCEPT => Some(Key::ImeAccept),
        VK_MODECHANGE => Some(Key::ImeModeChangeRequest),
        VK_SPACE => Some(Key::Space),
        VK_PRIOR => Some(Key::PageUp),
        VK_NEXT => Some(Key::PageDown),
        VK_END => Some(Key::End),
        VK_HOME => Some(Key::Home),
        VK_LEFT => Some(Key::LeftArrow),
        VK_UP => Some(Key::UpArrow),
        VK_RIGHT => Some(Key::RightArrow),
        VK_DOWN => Some(Key::DownArrow),
        VK_SELECT => Some(Key::Select),
        VK_PRINT => Some(Key::Print),
        VK_EXECUTE => Some(Key::Execute),
        VK_SNAPSHOT => Some(Key::PrintScreen), // this one's going in my cringe compilation
        VK_INSERT => Some(Key::Insert),
        VK_DELETE => Some(Key::Delete),
        VK_HELP => Some(Key::Help),

        0x30 => Some(Key::Alpha0),
        0x31 => Some(Key::Alpha1),
        0x32 => Some(Key::Alpha2),
        0x33 => Some(Key::Alpha3),
        0x34 => Some(Key::Alpha4),
        0x35 => Some(Key::Alpha5),
        0x36 => Some(Key::Alpha6),
        0x37 => Some(Key::Alpha7),
        0x38 => Some(Key::Alpha8),
        0x39 => Some(Key::Alpha9),

        // Undefined.
        0x3A..=0x40 => None,

        0x41 => Some(Key::A),
        0x42 => Some(Key::B),
        0x43 => Some(Key::C),
        0x44 => Some(Key::D),
        0x45 => Some(Key::E),
        0x46 => Some(Key::F),
        0x47 => Some(Key::G),
        0x48 => Some(Key::H),
        0x49 => Some(Key::I),
        0x4A => Some(Key::J),
        0x4B => Some(Key::K),
        0x4C => Some(Key::L),
        0x4D => Some(Key::M),
        0x4E => Some(Key::N),
        0x4F => Some(Key::O),
        0x50 => Some(Key::P),
        0x51 => Some(Key::Q),
        0x52 => Some(Key::R),
        0x53 => Some(Key::S),
        0x54 => Some(Key::T),
        0x55 => Some(Key::U),
        0x56 => Some(Key::V),
        0x57 => Some(Key::W),
        0x58 => Some(Key::X),
        0x59 => Some(Key::Y),
        0x5A => Some(Key::Z),

        VK_LWIN => Some(Key::LeftSuper),
        VK_RWIN => Some(Key::RightSuper),
        VK_APPS => Some(Key::Applications),

        // Reserved.
        0x5E => None,

        VK_SLEEP => Some(Key::Sleep),

        VK_NUMPAD0 => Some(Key::Keypad0),
        VK_NUMPAD1 => Some(Key::Keypad1),
        VK_NUMPAD2 => Some(Key::Keypad2),
        VK_NUMPAD3 => Some(Key::Keypad3),
        VK_NUMPAD4 => Some(Key::Keypad4),
        VK_NUMPAD5 => Some(Key::Keypad5),
        VK_NUMPAD6 => Some(Key::Keypad6),
        VK_NUMPAD7 => Some(Key::Keypad7),
        VK_NUMPAD8 => Some(Key::Keypad8),
        VK_NUMPAD9 => Some(Key::Keypad9),

        VK_MULTIPLY => Some(Key::KeypadMultiply),
        VK_ADD => Some(Key::KeypadAdd),
        VK_SEPARATOR => Some(Key::KeypadSeparator), // TODO: document this nightmare
        VK_SUBTRACT => Some(Key::KeypadSubtract),
        VK_DECIMAL => Some(Key::KeypadDecimal),
        VK_DIVIDE => Some(Key::KeypadDivide),

        VK_F1 => Some(Key::F1),
        VK_F2 => Some(Key::F2),
        VK_F3 => Some(Key::F3),
        VK_F4 => Some(Key::F4),
        VK_F5 => Some(Key::F5),
        VK_F6 => Some(Key::F6),
        VK_F7 => Some(Key::F7),
        VK_F8 => Some(Key::F8),
        VK_F9 => Some(Key::F9),
        VK_F10 => Some(Key::F10),
        VK_F11 => Some(Key::F11),
        VK_F12 => Some(Key::F12),
        VK_F13 => Some(Key::F13),
        VK_F14 => Some(Key::F14),
        VK_F15 => Some(Key::F15),
        VK_F16 => Some(Key::F16),
        VK_F17 => Some(Key::F17),
        VK_F18 => Some(Key::F18),
        VK_F19 => Some(Key::F19),
        VK_F20 => Some(Key::F20),
        VK_F21 => Some(Key::F21),
        VK_F22 => Some(Key::F22),
        VK_F23 => Some(Key::F23),
        VK_F24 => Some(Key::F24),

        // Unassigned.
        0x88..=0x8F => None,

        VK_NUMLOCK => Some(Key::NumLock),
        VK_SCROLL => Some(Key::ScrollLock),

        // OEM Specific. TODO, perhaps.
        0x92..=0x96 => None,

        // Unassigned.
        0x97..=0x9F => None,

        // These values are only recognized by GetAsyncKeyState and related,
        // but I'll add them for completion regardless.
        VK_LSHIFT => Some(Key::LeftShift),
        VK_RSHIFT => Some(Key::RightShift),
        VK_LCONTROL => Some(Key::LeftControl),
        VK_RCONTROL => Some(Key::RightControl),
        VK_LMENU => Some(Key::LeftAlt),
        VK_RMENU => Some(Key::RightAlt),

        VK_BROWSER_BACK => Some(Key::BrowserBack),
        VK_BROWSER_FORWARD => Some(Key::BrowserForward),
        VK_BROWSER_REFRESH => Some(Key::BrowserRefresh),
        VK_BROWSER_STOP => Some(Key::BrowserStop),
        VK_BROWSER_SEARCH => Some(Key::BrowserSearch),
        VK_BROWSER_FAVORITES => Some(Key::BrowserFavourites),
        VK_BROWSER_HOME => Some(Key::BrowserHome),
        VK_VOLUME_MUTE => Some(Key::MediaVolumeMute),
        VK_VOLUME_DOWN => Some(Key::MediaVolumeDown),
        VK_VOLUME_UP => Some(Key::MediaVolumeUp),
        VK_MEDIA_NEXT_TRACK => Some(Key::MediaNextTrack),
        VK_MEDIA_PREV_TRACK => Some(Key::MediaPreviousTrack),
        VK_MEDIA_STOP => Some(Key::MediaStop),
        VK_MEDIA_PLAY_PAUSE => Some(Key::MediaPlayPause),
        VK_LAUNCH_MAIL => Some(Key::LaunchMail),
        VK_LAUNCH_MEDIA_SELECT => Some(Key::LaunchMediaSelect),
        VK_LAUNCH_APP1 => Some(Key::LaunchApplication1),
        VK_LAUNCH_APP2 => Some(Key::LaunchApplication2),

        // Reserved.
        0xB8..=0xB9 => None,

        // Reserved (VK_GAMEPAD_xxx).
        0xC1..=0xDA => None,

        // Reserved.
        0xE0 => None,

        // TODO: "OEM Specific"
        0xE1 => None,

        VK_OEM_102 => Some(Key::Oem102),

        // TODO: "OEM Specific"
        0xE3..=0xE4 => None,

        VK_PROCESSKEY => Some(Key::ImeProcess),

        // TODO: "OEM Specific"
        0xE6 => None,

        VK_PACKET => None, // TODO

        // Unassigned.
        0xE8 => None,

        // TODO: "OEM Specific"
        0xE9..=0xF5 => None,

        VK_ATTN => Some(Key::Attn),
        VK_CRSEL => Some(Key::CrSel),
        VK_EXSEL => Some(Key::ExSel),
        VK_EREOF => Some(Key::EraseEof),
        VK_PLAY => Some(Key::Play),
        VK_ZOOM => Some(Key::Zoom),

        // Reserved.
        VK_NONAME => None,

        VK_PA1 => Some(Key::Pa1),
        VK_OEM_CLEAR => Some(Key::OemClear),

        // Undocumented.
        0xFF => None,

        // The keys not listed above are OEM. We handle these by mapping to an ASCII char, then mapping that to `Key`.
        k => {
            match unsafe { MapVirtualKeyW(k.into(), 2) } { // 2 is MAPVK_VK_TO_CHAR
                34 => Some(Key::Quote),
                35 => Some(Key::Hash),
                39 => Some(Key::Apostrophe),
                43 => Some(Key::Plus),
                44 => Some(Key::Comma),
                45 => Some(Key::Minus),
                46 => Some(Key::Period),
                47 => Some(Key::Slash),
                58 => Some(Key::Colon),
                59 => Some(Key::Semicolon),
                60 => Some(Key::LessThan),
                61 => Some(Key::Equals),
                62 => Some(Key::GreaterThan),
                63 => Some(Key::QuestionMark),
                91 => Some(Key::BracketLeft),
                92 => Some(Key::Backslash),
                93 => Some(Key::BracketRight),
                95 => Some(Key::Underscore),
                96 => Some(Key::Grave),
                123 => Some(Key::BraceLeft),
                124 => Some(Key::Pipe),
                125 => Some(Key::BraceRight),
                _ => None,
            }
        },
    }
}

pub unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
   // println!("WindowProc ({:p}, {:#X}, {:#X} {:#X})", hwnd, msg, wparam, lparam);
    // Fantastic resource for a comprehensive list of window messages:
    // https://wiki.winehq.org/List_Of_Windows_Messages
    match msg {
        // No-op event, usable for pinging the event loop, stubbing, etc. Returns 0.
        WM_NULL => 0,

        // Received after the client area has been created, but before the window is made visible.
        // This event comes after `WM_NCCREATE`, the event sent after the non-client area is created.
        // wParam: Unused, ignore.
        // lParam: `CREATESTRUCTW *` (in)
        // Return 0 to succeed `CreateWindowExW`, or -1 to destroy the window and return NULL.
        // See also: `WM_NCCREATE`
        WM_CREATE => {
            let param = &**(lparam as *const *const WindowCreateParams);
            let state = &mut *param.state;
            if WIN32.dpi_mode == Win32DpiMode::PerMonitorV1 || WIN32.dpi_mode == Win32DpiMode::PerMonitorV2 {
                let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut dx: UINT = 0;
                let mut dy: UINT = 0;
                if WIN32.dl.GetDpiForMonitor(monitor, 0, &mut dx, &mut dy) == Some(0) && dx != state.dpi {
                    state.dpi = dx;
                    let (dw_style, dw_style_ex) = style_to_bits(&state.style);
                    let ((width, height), _) = adjust_window_for_dpi(WIN32.get(), param.wh, dw_style, dw_style_ex, dx);
                    let _ = SetWindowPos(hwnd, ptr::null_mut(), 0, 0, width as _, height as _, SWP_NOMOVE);
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        // Received as the client area is being destroyed.
        // This event is received, then `WM_NCDESTROY`, and the window is gone after that.
        // Nothing can actually be done once this message is received, and you always return 0.
        WM_DESTROY => 0,

        // TODO
        WM_MOVE => {
            let x = lparam & 0xFFFF;
            let y = (lparam >> 16) & 0xFFFF;
            let state = &mut *user_state(hwnd);
            if !(x as i16 == -32000 || y as i16 == -32000) {
                state.dispatch_event(Event::Move((x as _, y as _)));
            }
            0
        },

        // << Event 0x0004 is not known to exist. >>

        // TODO
        WM_SIZE => {
            let w = lparam & 0xFFFF;
            let h = (lparam >> 16) & 0xFFFF;
            let state = &mut *user_state(hwnd);

            fn set_max_min(user_data: &mut WindowState, max: bool, min: bool) {
                if user_data.is_max != max {
                    user_data.is_max = max;
                    let _ = user_data.dispatch_event(Event::Maximise(max));
                }
                if user_data.is_min != min {
                    user_data.is_min = min;
                    let _ = user_data.dispatch_event(Event::Minimise(min));
                }
            }

            // Minimize events give us a confusing new client size of (0, 0) so we ignore that
            if wparam != SIZE_MINIMIZED {
                state.wh = (w as _, h as _);
                state.dispatch_event(Event::Resize((w as _, h as _)));
            }

            match wparam {
                SIZE_RESTORED => set_max_min(state, false, false),
                SIZE_MINIMIZED => set_max_min(state,  false, true),
                SIZE_MAXIMIZED => set_max_min(state,  true, false),
                _ => (), // rest are for pop-up (`WS_POPUP`) windows
            }
            0
        },

        // Received when the window is activated or deactivated (focus gain/loss). Return 0.
        // wParam: HIWORD = non-zero if minimized, LOWORD = WA_ACTIVE | WA_CLICKACTIVE | WA_INACTIVE
        // lParam: HWND to window being deactivated (if ACTIVE|CLICKATIVE) otherwise the activated one
        // See also: `WM_ACTIVATEAPP` and `WM_SETFOCUS` & `WM_KILLFOCUS`
        WM_ACTIVATE => {
            // Quoting MSDN:
            // "The high-order word specifies the minimized state of the window being activated
            // or deactivated. A nonzero value indicates the window is minimized."
            //
            // This doesn't work entirely correctly in all situations, as with most of Win32,
            // so if we don't do some logic here we get two events on unfocusing
            // by clicking on the taskbar icon for example, among other things:
            // 1) WM_INACTIVE (HIWORD == 0)
            // 2) WM_ACTIVATE (HIWORD != 0)
            // Note that #2 translates to active(focused) & minimized simultaneously.
            // This would mean the window would be told it's focused after being minimized. Fantastic.

            // These problems could be avoided like this:
            // match (loword, hiword) {
            //     (true, true) => return 0,
            //     (x, _) => {
            //         if state.is_focused != x {
            //             state.is_focused = x;
            //             state.push_event(Event::Focus(x));
            //         }
            //     },
            // }
            // However, that's a waste of time when you can just process `WM_SETFOCUS` and `WM_KILLFOCUS`.

            0
        },

        // Received when a window receives keyboard focus. Return 0.
        // This is mainly intended for textbox controls but works perfectly fine for actual windows.
        // See also: `WM_ACTIVATE` (to know why this is used for focus events)
        WM_SETFOCUS => {
            (*user_state(hwnd)).dispatch_event(Event::Focus(true));
            0
        },

        // Received when a window loses keyboard focus. Return 0.
        // See also: `WM_SETFOCUS` and `WM_ACTIVATE`
        WM_KILLFOCUS => {
            (*user_state(hwnd)).dispatch_event(Event::Focus(false));
            0
        },

        WM_SHOWWINDOW => {
            if lparam == 0 {
                (*user_state(hwnd)).dispatch_event(Event::Visible(wparam != 0));
            }
            0
        },

        WM_PAINT => {
            // windows will not stop spamming this event until you process it like this
            // we don't actually draw anything, of course
            let mut paintstruct = mem::MaybeUninit::<PAINTSTRUCT>::uninit();
            let _ = BeginPaint(hwnd, paintstruct.as_mut_ptr());
            let _ = EndPaint(hwnd, paintstruct.as_mut_ptr());
            0
        },

        // Received when a window is requested to close. Return 0.
        WM_CLOSE => {
            let state = &mut *user_state(hwnd);
            state.dispatch_event(Event::CloseRequest);
            0
        },

        WM_NCCREATE => {
            // `lpCreateParams` is the first field, so `CREATESTRUCTW *` is `WindowCreateParams **`
            let params = &**(lparam as *const *const WindowCreateParams);
            let _ = set_instance_storage(hwnd, GWL_USERDATA, params.state as usize);
            if WIN32.dpi_mode == Win32DpiMode::PerMonitorV1 {
                _ = WIN32.dl.EnableNonClientDpiScaling(hwnd);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        },

        // Received when a key is pressed or released.
        // wParam: Virtual key code.
        // lParam: Giant bitfield. Please just read MSDN (hint: bit 31 means MSB, 0 is LSB).
        // Return 0.
        WM_KEYDOWN | WM_KEYUP => {
            #[cfg(feature = "input")]
            if let Some(key) = translate_vk(wparam) {
                (*user_state(hwnd)).dispatch_event(map_tr_state(extend_key(key, lparam), lparam));
            }
            0
        },

        // Same as `WM_KEYDOWN` & `WM_KEYUP` but with a few horrific bitfield quirks.
        WM_SYSKEYDOWN | WM_SYSKEYUP => {
            // As a side-effect of handling "system keys", we actually override Alt+F4.
            // It's re-implemented here, because it's usually expected that Alt+F4 does something.
            if wparam & 0xFF == VK_F4 as WPARAM && lparam & (1 << 29) != 0 {
                let _ = SendMessageW(hwnd, WM_CLOSE, 0, 0);
            }

            // This is one of the worst parts of the Win32 input event system.
            // Countless games have bugs and exploits relating to the Alt & F10 keys.

            // To sum it up, the Alt and F10 keys are very special due to historical reasons.
            // They have their own event if in combination with other keys.
            // Making it worse, "it also occurs when no window currently has the keyboard focus",
            // although this isn't actually observable on new OSes so it's just a legacy feature.

            // Bit 29 in lParam is set if the Alt key is down while emitting this message,
            // which sounds like a reasonable fix to tell apart the two reasons for this message.
            // Except if it's the Alt key being released, it won't be set! So you must trust wParam.
            // F10 doesn't even set any bit because there's no F10 bit, so you trust that one too.
            #[cfg(feature = "input")]
            if let Some(event) = sys_key_event(wparam, lparam) {
                let state = &mut *user_state(hwnd);
                state.dispatch_event(event);
            }

            0
        },

        WM_CHAR => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                let data = wparam as u32;
                let b1 = data & 0xFF;
                let cp = if matches!(b1, 0xD800..=0xDFFF) {
                    // supplementary plane
                    let b2 = (data >> 16) & 0xFF;
                    let u = ((b1 - 0xD800) << 10) | (b2 - 0xDC00);
                    u + 0x10000
                } else {
                    // basic multilingual plane
                    b1 as u32
                };
                state.dispatch_event(Event::Input(char::from_u32_unchecked(cp)));
            }
            0
        },

        WM_UNICHAR => {
            let brony_detected = wparam != 0xFFFF;
            #[cfg(feature = "input")]
            if brony_detected {
                let state = &mut *user_state(hwnd);
                state.dispatch_event(Event::Input(char::from_u32_unchecked(wparam as _)));
            }
            brony_detected.into()
        },

        WM_MOUSEWHEEL => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                let delta = (wparam >> 16) as u16 as i16;
                if delta > 0 {
                    state.dispatch_event(Event::ScrollUp);
                } else if delta < 0 {
                    state.dispatch_event(Event::ScrollDown);
                }
            }
            0
        },

        WM_MOUSEMOVE => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                let x = lparam & 0xFFFF;
                let y = (lparam >> 16) & 0xFFFF;
                if !state.mouse_tracked {
                    state.mouse_tracked = true;
                    state.dispatch_event(Event::MouseEnter);
                    let mut tme = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as _,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    let _ = TrackMouseEvent(&mut tme);
                }
                state.dispatch_event(Event::MouseMove((x as _, y as _)));
            }
            0
        },

        WM_DPICHANGED => {
            let dx = (wparam & 0xffff) as UINT;
            let mut state = &mut *user_state(hwnd);
            if WIN32.dpi_mode == Win32DpiMode::PerMonitorV1 || WIN32.dpi_mode == Win32DpiMode::PerMonitorV2 {
                state.dpi = dx;
                let (dw_style, dw_style_ex) = style_to_bits(&state.style);
                let ((width, height), _) = adjust_window_for_dpi(WIN32.get(), state.wh, dw_style, dw_style_ex, dx);
                let _ = SetWindowPos(hwnd, ptr::null_mut(), 0, 0, width as _, height as _, SWP_NOMOVE);
            }
            0
        },

        WM_MOUSELEAVE => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                state.mouse_tracked = false;
                state.dispatch_event(Event::MouseLeave);
            }
            0
        },

        WM_LBUTTONDOWN => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                state.dispatch_event(Event::MouseDown(MouseButton::Left));
            }
            0
        },
        WM_RBUTTONDOWN => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                state.dispatch_event(Event::MouseDown(MouseButton::Right));
            }
            0
        },
        WM_MBUTTONDOWN => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                state.dispatch_event(Event::MouseDown(MouseButton::Middle));
            }
            0
        },
        WM_LBUTTONUP => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                state.dispatch_event(Event::MouseUp(MouseButton::Left));
            }
            0
        },
        WM_RBUTTONUP => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                state.dispatch_event(Event::MouseUp(MouseButton::Right));
            }
            0
        },
        WM_MBUTTONUP => {
            #[cfg(feature = "input")]
            {
                let state = &mut *user_state(hwnd);
                state.dispatch_event(Event::MouseUp(MouseButton::Middle));
            }
            0
        },

        WM_SETCURSOR => {
            if (hwnd == wparam as HWND) && ((lparam & 0xFFFF) as WORD == HTCLIENT as WORD) {
                _ = SetCursor((*user_state(hwnd)).cursor);
                TRUE as LRESULT
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        },

        // Custom message: The "real" destroy signal that won't be rejected.
        // TODO: document the rejection emchanism somewhere
        // Return 0.
        RAMEN_WM_DROP => {
            let _ = DestroyWindow(hwnd);
            0
        },

        RAMEN_WM_SETCURSOR => {
            let state = &mut *user_state(hwnd);
            let cursor = mem::transmute::<_, Cursor>(wparam as u32);
            let rsrc = cursor_to_int_resource(cursor);

            // `LoadImageW` is not only superseding `LoadCursorW` but it's ~20µs faster. Wow, use this!
            state.cursor = if !rsrc.is_null() {
                LoadImageW(ptr::null_mut(), rsrc, IMAGE_CURSOR, 0, 0, LR_DEFAULTSIZE | LR_SHARED).cast()
            } else {
                ptr::null_mut()
            };

            // Immediately update the cursor icon if it's within the client area.
            let mut mouse_pos: POINT = mem::zeroed();
            if GetCursorPos(&mut mouse_pos) != 0 && WindowFromPoint(POINT { ..mouse_pos }) == hwnd {
                _ = SetCursor(state.cursor);
            }
            0
        },

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = PostMessageW(self.hwnd, RAMEN_WM_DROP, 0, 0);
        }
    }
}

#[inline]
unsafe fn ping_window_frame(hwnd: HWND) {
    const MASK: UINT = SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER | SWP_NOZORDER | SWP_FRAMECHANGED;
    let _ = SetWindowPos(hwnd, ptr::null_mut(), 0, 0, 0, 0, MASK);
}
