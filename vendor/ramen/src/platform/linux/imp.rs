// TODO: I suppose we'll need some method of deciding at runtime whether to use x11 or wayland? This is just x11
use crate::{error::Error, event::Event, util::sync::{mutex_lock, Mutex}, connection, window};
use super::ffi::*;

use std::{collections::HashMap, sync::Arc};

/// The initial capacity for any Vec<Event>
/// Event is around 8 bytes in size, so it's fairly costless for this to be a large starting capacity.
const QUEUE_SIZE: usize = 256;

pub(crate) struct Connection {
    details: ConnectionDetails,
    event_buffer: HashMap<xcb_window_t, Vec<*mut xcb_generic_event_t>>,
    hostname: Option<Vec<c_char>>,
}

// Proxy struct for passing Connection details around without the allocated parts
#[derive(Clone, Copy)]
struct ConnectionDetails {
    display: *mut Display,
    connection: *mut xcb_connection_t,
    screen: *mut xcb_screen_t,
    screenid: u32,
    atoms: Atoms,
    extensions: Extensions,
}

#[derive(Clone, Copy)]
struct Atoms {
    wm_protocols: xcb_atom_t,
    wm_delete_window: xcb_atom_t,
    _net_wm_name: xcb_atom_t,
    utf8_string: xcb_atom_t,
    _net_wm_pid: xcb_atom_t,
    wm_client_machine: xcb_atom_t,
    _net_wm_ping: xcb_atom_t,
    _net_wm_state: xcb_atom_t,
    _net_wm_state_maximized_horz: xcb_atom_t,
    _net_wm_state_maximized_vert: xcb_atom_t,
    _net_wm_state_hidden: xcb_atom_t,
    _motif_wm_hints: xcb_atom_t,
}

#[derive(Clone, Copy)]
struct Extensions {
    #[cfg(feature = "input")]
    xinput: u8,
}

impl Connection {
    pub(crate) fn new() -> Result<Self, Error> {
        unsafe {
            libX11::load()?;
            libX11_xcb::load()?;
            libxcb::load()?;

            let display = XOpenDisplay(std::ptr::null_mut());
            if display.is_null() {
                // TODO: Unclear why this could fail when passing nullptr to it. Maybe the system has no screens?
                // Maybe the underlying connection has failed, but how would we check?
                return Err(Error::Unknown)
            }
            let screen_num = XDefaultScreen(display);
            let connection = XGetXCBConnection(display);
            XSetEventQueueOwner(display, EventQueueOwner::XCBOwnsEventQueue);
            let mut iter = xcb_setup_roots_iterator(xcb_get_setup(connection));
            for _ in 0..screen_num {
                xcb_screen_next(&mut iter);
            }
            let screen = iter.data;
            let atoms = Atoms::new(connection)?;

            // Make sure xinput is available
            #[cfg(feature = "input")]
            let xi_opcode;
            #[cfg(feature = "input")]
            {
                // xcb_query_extension cannot generate errors, so we don't check
                let xi_name = "XInputExtension";
                let xi = xcb_query_extension_reply(
                    connection,
                    xcb_query_extension(connection, xi_name.bytes().len() as _, xi_name.as_ptr().cast()),
                    std::ptr::null_mut(),
                );
                if xi.is_null() {
                    return Err(Error::SystemResources)
                }
                if (*xi).present == 0 {
                    return Err(Error::Unsupported)
                }
                xi_opcode = (*xi).major_opcode;
                free(xi.cast());

                libxcb_xinput::load()?;
                libxkbcommon::load()?;
            }

            // Try to get machine's hostname
            let mut len = 16;
            let mut hostname: Vec<c_char> = Vec::new();
            let hostname = loop {
                hostname.resize_with(len, Default::default); // Make sure vec is full of null-terminators
                let err = libc::gethostname((&mut hostname).as_mut_ptr(), len);
                if err == 0 {
                    // We got the hostname, now let's make sure the i8 vec is exactly the right size with no extra nulls
                    if let Some(pos) = hostname.iter().position(|x| *x == 0) {
                        hostname.set_len(pos + 1);
                    } else {
                        // There are no null-terminators, this means the vec was exactly the size of the hostname
                        // So we need to push a null-terminator onto it ourselves
                        hostname.push(0);
                    }
                    //hostname.shrink_to_fit(); // useful?
                    break Some(hostname);
                } else {
                    // Either ENAMETOOLONG or EINVAL would both indicate that the hostname is longer than the buffer
                    match len.checked_mul(2) {
                        Some(l) if l <= (1 << 16) => len = l,
                        _ => break None, // Give up if some sanity limit is reached or we overflowed usize..
                    }
                }
            };

            Ok(Connection {
                details: ConnectionDetails {
                    display,
                    connection,
                    screen,
                    screenid: screen_num as _,
                    atoms,
                    extensions: Extensions {
                        #[cfg(feature = "input")]
                        xinput: xi_opcode,
                    },
                },
                event_buffer: HashMap::new(),
                hostname,
            })
        }
    }

    pub(crate) fn xscreenid(&self) -> u32 {
        self.details.screenid
    }

    pub(crate) fn xdisplay(&self) -> *mut Display {
        self.details.display
    }

    // Helper wrapper for `xcb_connection_has_error` for use with `?`. Assumes pointer is valid.
    unsafe fn check(c: *mut xcb_connection_t) -> Result<(), Error> {
        let err = xcb_connection_has_error(c);
        match err {
            XCB_NONE => Ok(()),
            XCB_CONN_CLOSED_EXT_NOTSUPPORTED => Err(Error::Unsupported),
            XCB_CONN_CLOSED_MEM_INSUFFICIENT => Err(Error::SystemResources),
            _ => Err(Error::Invalid),
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.details.extensions;
        unsafe {
            let _ = xcb_flush(self.details.connection);
            let _ = XCloseDisplay(self.details.display);
        }
    }
}

unsafe impl Send for Connection {}

impl Atoms {
    unsafe fn new(connection: *mut xcb_connection_t) -> Result<Self, Error> {
        const N_ATOMS: usize = 12;
        let mut atom_replies = [0 as c_uint; N_ATOMS];
        let mut atoms = [0 as xcb_atom_t; N_ATOMS];
        macro_rules! atom {
            ($n:literal, $name:literal) => {{
                atom_replies[$n] = xcb_intern_atom(connection, 0, $name.len() as u16, $name.as_ptr().cast());
            }};
        }
        atom!(0, "WM_PROTOCOLS");
        atom!(1, "WM_DELETE_WINDOW");
        atom!(2, "_NET_WM_NAME");
        atom!(3, "UTF8_STRING");
        atom!(4, "_NET_WM_PID");
        atom!(5, "WM_CLIENT_MACHINE");
        atom!(6, "_NET_WM_PING");
        atom!(7, "_NET_WM_STATE");
        atom!(8, "_NET_WM_STATE_MAXIMIZED_HORZ");
        atom!(9, "_NET_WM_STATE_MAXIMIZED_VERT");
        atom!(10, "_NET_WM_STATE_HIDDEN");
        atom!(11, "_MOTIF_WM_HINTS");
        for (r, seq) in atoms.iter_mut().zip(atom_replies.into_iter()) {
            let mut err: *mut xcb_generic_error_t = std::ptr::null_mut();
            let reply = xcb_intern_atom_reply(connection, seq, &mut err);
            if !reply.is_null() {
                *r = (*reply).atom;
                free(reply.cast());
            } else {
                free(err.cast());
                // xcb_intern_atom can only fail due to alloc error or value error,
                // and this can't be a value error because we always pass a valid value (0) for only_if_exists
                return Err(Error::SystemResources);
            }
        }
        Ok(Self {
            wm_protocols: atoms[0],
            wm_delete_window: atoms[1],
            _net_wm_name: atoms[2],
            utf8_string: atoms[3],
            _net_wm_pid: atoms[4],
            wm_client_machine: atoms[5],
            _net_wm_ping: atoms[6],
            _net_wm_state: atoms[7],
            _net_wm_state_maximized_horz: atoms[8],
            _net_wm_state_maximized_vert: atoms[9],
            _net_wm_state_hidden: atoms[10],
            _motif_wm_hints: atoms[11],
        })
    }
}

pub(crate) struct Window {
    connection: connection::Connection,
    details: WindowDetails,
}

// Proxy struct that pretty much only exists to get around the fact that we're using Rust
pub(crate) struct WindowDetails {
    handle: xcb_window_t,
    style: Arc<Mutex<crate::window::Style>>,
    event_buffer: Vec<Event>,
    parent: xcb_window_t,
    position: (i16, i16),
    size: (u16, u16),
    state_maximised: (bool, bool), // horz vert
    state_minimised: bool,
}

impl Window {
    pub(crate) fn new(builder: window::Builder) -> Result<Self, Error> {
        unsafe {
            let mut connection_mtx = mutex_lock(&builder.connection.0);
            let connection: &mut Connection = &mut *connection_mtx;
            let c = connection.details.connection;
            let hostname = connection.hostname.as_ref();

            // TODO: copy these from the builder when they're in there
            let (x, y) = builder.position.unwrap_or((0, 0));
            let (width, height) = builder.size;

            // Generate an ID for our new window
            let xid = xcb_generate_id(c);
            if xid == !0u32 {
                // xcb_generate_id returns -1 on any type of failure, most likely because it has run out of
                // resources to fulfil requests for new IDs. It could also mean the connection has been closed.
                return Err(Error::SystemResources);
            }

            // Clear the event queue, in case any events remain in it intended for a previous object with this xid we just claimed
            let event = xcb_poll_for_event(c);
            if !event.is_null() {
                if let Some(window) = get_event_window(event, &connection.details) {
                    if let Some(queue) = connection.event_buffer.get_mut(&window) {
                        queue.push(event);
                    }
                }
            }
            loop {
                let event = xcb_poll_for_queued_event(c);
                if event.is_null() { break }
                if let Some(window) = get_event_window(event, &connection.details) {
                    if let Some(queue) = connection.event_buffer.get_mut(&window) {
                        queue.push(event);
                    }
                }
            }

            // Create the new X window
            const REGULAR_MASK: u32 = XCB_EVENT_MASK_STRUCTURE_NOTIFY | XCB_EVENT_MASK_PROPERTY_CHANGE;
            // ButtonPress is exclusive, so we request it in CreateWindow to make sure we get it first
            #[cfg(feature = "input")]
            const EVENT_MASK: u32 = XCB_EVENT_MASK_BUTTON_PRESS | REGULAR_MASK;
            #[cfg(not(feature = "input"))]
            const EVENT_MASK: u32 = XCB_EVENT_MASK_FOCUS_CHANGE | REGULAR_MASK;
            const VALUE_MASK: u32 = XCB_CW_EVENT_MASK;

            let cmap = builder.visual.map(|vi| {
                let cmap = xcb_generate_id(c);
                _ = xcb_create_colormap(c, 0, cmap, (*connection.details.screen).root, vi);
                _ = xcb_flush(c);
                cmap
            });
            let value_mask = if cmap.is_some() { VALUE_MASK|XCB_CW_COLORMAP } else { VALUE_MASK };
            let value_list = if let Some(c) = cmap { (&[EVENT_MASK, c]).to_vec() } else { (&[EVENT_MASK]).to_vec() };

            let create_error = xcb_request_check(c, xcb_create_window_checked(
                c,
                builder.depth.unwrap_or(XCB_COPY_FROM_PARENT as _),
                xid,
                (*connection.details.screen).root,
                x,
                y,
                width,
                height,
                0,
                XCB_WINDOW_CLASS_INPUT_OUTPUT,
                builder.visual.unwrap_or(XCB_COPY_FROM_PARENT.into()),
                value_mask,
                &value_list[0],
            ));
            if !create_error.is_null() {
                // Reasons CreateWindow may fail are:
                // Alloc - maps to Error::SystemResources
                // Colormap - we don't currently pass a colormap
                // Cursor - we do not pass a Cursor
                // IDChoice - we got our ID straight from xcb_generate_id and didn't use it for anything else
                // Match - bad configuration of user params, so maps to Error::Invalid
                // Pixmap - we don't currently pass a pixmap
                // Value - bad value for a user param, so maps to Error::Invalid
                // Window - we just created that XID so that's not possible
                let errno = (*create_error).error_code;
                free(create_error.cast());
                if errno as c_int == XCB_ALLOC {
                    return Err(Error::SystemResources);
                } else {
                    return Err(Error::Invalid);
                }
            }

            // Select xinput events
            #[cfg(feature = "input")]
            {
                // xcb_input_xi_select_events cannot generate errors so we use _checked and discard it
                #[repr(C)]
                struct XiMask {
                    head: xcb_input_event_mask_t,
                    body: u32,
                }
                let mut mask = XiMask {
                    head: xcb_input_event_mask_t {
                        deviceid: XCB_INPUT_DEVICE_ALL_MASTER,
                        mask_len: 1,
                    },
                    body: XCB_INPUT_XI_EVENT_MASK_KEY_PRESS | XCB_INPUT_XI_EVENT_MASK_KEY_RELEASE
                        | XCB_INPUT_XI_EVENT_MASK_BUTTON_PRESS | XCB_INPUT_XI_EVENT_MASK_BUTTON_RELEASE
                        | XCB_INPUT_XI_EVENT_MASK_MOTION | XCB_INPUT_XI_EVENT_MASK_ENTER | XCB_INPUT_XI_EVENT_MASK_LEAVE
                        | XCB_INPUT_XI_EVENT_MASK_FOCUS_IN | XCB_INPUT_XI_EVENT_MASK_FOCUS_OUT,
                };
                xcb_discard_reply(c, xcb_input_xi_select_events_checked(c, xid, 1, (&mut mask.head) as _));
            }

            // Setup WM_PROTOCOLS
            let window_atoms = [
                connection.details.atoms.wm_delete_window,
                connection.details.atoms._net_wm_ping,
            ];
            let _ = xcb_change_property(
                c,
                XCB_PROP_MODE_REPLACE,
                xid,
                connection.details.atoms.wm_protocols,
                XCB_ATOM_ATOM,
                32,
                window_atoms.len() as _,
                (&window_atoms) as *const xcb_window_t as _,
            );

            // Try to write the requested window title to the WM_NAME and _NET_WM_NAME properties
            // Note: multibyte characters won't render correctly in WM_NAME, but any modern and worthwhile WM will
            // prioritise using _NET_WM_NAME which is UTF-8 as standard, that's why it's better to write both.
            let title = builder.title.as_ref();
            internal_set_title(c, xid, &connection.details.atoms, title);

            // If hostname is known, get PID of current process and write that to _NET_WM_PID
            // But don't write either of these properties if hostname is not known, because:
            // "If _NET_WM_PID is set, the ICCCM-specified property WM_CLIENT_MACHINE MUST also be set." - EWMH spec
            if let Some(hostname) = hostname {
                let pid = getpid();
                let _ = xcb_change_property(
                    c,
                    XCB_PROP_MODE_REPLACE,
                    xid,
                    connection.details.atoms._net_wm_pid,
                    XCB_ATOM_CARDINAL,
                    32,
                    1,
                    (&pid) as *const i32 as _,
                );

                let _ = xcb_change_property(
                    c,
                    XCB_PROP_MODE_REPLACE,
                    xid,
                    connection.details.atoms.wm_client_machine,
                    XCB_ATOM_STRING,
                    8,
                    hostname.len() as _,
                    hostname.as_ptr().cast(),
                );
            }

            // Set class name
            let mut instance = "unknown".to_string();
            if let Some(name) = std::env::var_os("RESOURCE_NAME") {
                instance = name.as_os_str().to_string_lossy().into_owned();
            } else {
                if let Some(argv0) = std::env::args_os().next() {
                    let path = std::path::Path::new(argv0.as_os_str());
                    if let Some(basename) = path.file_name() {
                        instance = basename.to_string_lossy().into_owned();
                    }
                }
            }
            let wm_class = format!("{}\0{}\0", instance, builder.class_name.as_ref());
            _ = xcb_change_property(
                c,
                XCB_PROP_MODE_REPLACE,
                xid,
                XCB_ATOM_WM_CLASS,
                XCB_ATOM_STRING,
                8,
                wm_class.len() as _,
                wm_class.as_ptr().cast(),
            );

            // Map window to screen
            if builder.style.visible {
                let _ = xcb_map_window(c, xid);

                // Set maximised (this needs to be done after map)
                if builder.maximised {
                    internal_set_maximised(c, xid, &connection.details, true);
                }
            }

            // Now we'll insert an entry into the EVENT_QUEUE hashmap for this window we've created.
            // We do this even if the queue probably won't be used, as it's the soundest way to ensure
            // memory gets cleaned up.
            let _ = connection.event_buffer.insert(xid, Vec::with_capacity(QUEUE_SIZE));

            // TODO: This "returns <= 0 on error", how is that value significant? Is it -EINVAL type thing?
            if xcb_flush(c) <= 0 {
                Connection::check(c)?;
                return Err(Error::Unknown)
            }

            let root = (*connection.details.screen).root;
            std::mem::drop(connection_mtx);

            let window = Window {
                connection: builder.connection,
                details: WindowDetails {
                    handle: xid,
                    event_buffer: Vec::with_capacity(QUEUE_SIZE),
                    parent: root,
                    position: (x, y),
                    style: Arc::new(Mutex::new(builder.style)),
                    size: (width, height),
                    state_maximised: (false, false),
                    state_minimised: false,
                },
            };

            window.set_borderless(builder.style.borderless);
            set_wm_normal_hints(c, &window.details, window.details.size);

            Ok(window)
        }
    }

    pub(crate) fn events(&self) -> &[Event] {
        &self.details.event_buffer
    }

    pub(crate) fn poll_events(&mut self) {
        unsafe {
            let window_details = &mut self.details;
            // First: lock the global event queue, which is used as backup storage for events
            // which have been pulled but are not immediately relevant
            let mut connection_ = mutex_lock(&self.connection.0);
            let Connection {
                details: connection_details,
                event_buffer: map,
                ..
            } = &mut *connection_;
            let c = connection_details.connection;

            // Clear our event buffer of the previous set of events
            window_details.event_buffer.clear();

            // Fill our event buffer with any events which may have been stored in the global event queue,
            // also clearing them from the global queue
            // Note: this queue SHOULD always exist, but it's possible some bad or malicious user code might get a
            // `None` result, so it's better to check and take no action if there's no queue to copy from...
            if let Some(queue) = map.get_mut(&window_details.handle) {
                for event in queue.iter().copied() {
                    process_event(event, window_details, connection_details);
                }
                queue.clear();
            }

            // Deliver stuff (polling won't flush out)
            let _ = xcb_flush(c);

            // Call `poll_event` once, which populates XCB's internal linked list from the connection
            let event = xcb_poll_for_event(c);
            if !event.is_null() {
                if let Some(window) = get_event_window(event, connection_details) {
                    if window == window_details.handle {
                        process_event(event, window_details, connection_details);
                    } else if let Some(queue) = map.get_mut(&window) {
                        queue.push(event);
                    }
                }
            }
            // Now repeatedly call `poll_for_queued_event` to drain xcb's queue without any new events arriving in it
            loop {
                let event = xcb_poll_for_queued_event(c);
                if event.is_null() { break }
                if let Some(window) = get_event_window(event, connection_details) {
                    if window == window_details.handle {
                        process_event(event, window_details, connection_details);
                    } else if let Some(queue) = map.get_mut(&window) {
                        queue.push(event);
                    }
                }
            }
        }
    }

    pub(crate) fn set_maximised(&self, maximised: bool) {
        let mut connection_ = mutex_lock(&self.connection.0);
        let connection = &mut connection_;
        unsafe {
            internal_set_maximised(connection.details.connection, self.details.handle, &connection.details, maximised);
        }
    }

    pub(crate) fn set_position(&self, (x, y): (i16, i16)) {
        let mut connection_ = mutex_lock(&self.connection.0);
        let connection = &mut connection_;
        // TODO how does negative stuff interact here with xcb? how is it MEANT TO?
        let xy = [x as u32, y as u32];
        unsafe {
            _ = xcb_configure_window(connection.details.connection, self.details.handle, 1|2, xy.as_ptr().cast());
        }
    }

    pub(crate) fn set_borderless(&self, borderless: bool) {
        let mut connection_ = mutex_lock(&self.connection.0);
        let connection = &mut connection_;
        let mut g = mutex_lock(&self.details.style);
        g.borderless = borderless;
        std::mem::drop(g);
        unsafe { set_mwm_hints(connection.details.connection, &connection.details, &self.details) };
    }

    pub(crate) fn set_resizable(&self, resizable: bool) {
        let mut connection_ = mutex_lock(&self.connection.0);
        let connection = &mut connection_;
        let mut g = mutex_lock(&self.details.style);
        g.resizable = resizable;
        std::mem::drop(g);
        unsafe {
            set_mwm_hints(connection.details.connection, &connection.details, &self.details);
            set_wm_normal_hints(connection.details.connection, &self.details, self.details.size);
        }
    }

    pub(crate) fn set_size(&self, (width, height): (u16, u16)) {
        let mut connection_ = mutex_lock(&self.connection.0);
        let connection = &mut connection_;
        let wh = [width as u32, height as u32];
        unsafe {
            _ = xcb_configure_window(connection.details.connection, self.details.handle, 4|8, wh.as_ptr().cast());
            set_wm_normal_hints(connection.details.connection, &self.details, (width, height));
        }
    }

    pub(crate) fn set_title(&self, title: &str) {
        let mut connection_ = mutex_lock(&self.connection.0);
        let connection = &mut connection_;
        unsafe {
            internal_set_title(connection.details.connection, self.details.handle, &connection.details.atoms, title);
        }
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        let mut connection_ = mutex_lock(&self.connection.0);
        let connection = &mut connection_;
        unsafe {
            if visible {
                _ = xcb_map_window(connection.details.connection, self.details.handle);
            } else {
                _ = xcb_unmap_window(connection.details.connection, self.details.handle);
            }
        }
    }

    pub(crate) fn xid(&self) -> xcb_window_t {
        self.details.handle
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let mut connection_ = mutex_lock(&self.connection.0);
        let connection = &mut connection_;
        unsafe {
            let _ = xcb_destroy_window(connection.details.connection, self.details.handle);
            let _ = xcb_flush(connection.details.connection);
        }
    }
}

unsafe fn set_mwm_hints(
    c: *mut xcb_connection_t,
    cdetails: &ConnectionDetails,
    wdetails: &WindowDetails,
) {
    let g = mutex_lock(&wdetails.style);
    let style = *g;
    std::mem::drop(g);
    let mut hints = MwmHints {
        flags: MWM_HINTS_FUNCTIONS | MWM_HINTS_DECORATIONS,
        functions: 0,
        decorations: 0,
        input_mode: 0,
        status: 0,
    };
    if !style.borderless {
        hints.decorations |= MWM_DECOR_BORDER;
        hints.decorations |= MWM_DECOR_TITLE;
        hints.functions |= MWM_FUNC_MOVE;
        if style.resizable {
            hints.decorations |= MWM_DECOR_RESIZEH;
            hints.functions |= MWM_FUNC_RESIZE;
        }
        if let Some(controls) = style.controls {
            hints.decorations |= MWM_DECOR_MENU;
            if controls.minimise { hints.functions |= MWM_FUNC_MINIMIZE; }
            if controls.maximise && style.resizable { hints.functions |= MWM_FUNC_MAXIMIZE; }
            if controls.close { hints.functions |= MWM_FUNC_CLOSE; }
        }
    }
    _ = xcb_change_property(
        c,
        XCB_PROP_MODE_REPLACE,
        wdetails.handle,
        cdetails.atoms._motif_wm_hints,
        cdetails.atoms._motif_wm_hints,
        32,
        std::mem::size_of_val(&hints) as u32 / 4,
        (&hints) as *const _ as _,
    );
}

unsafe fn set_wm_normal_hints(c: *mut xcb_connection_t, details: &WindowDetails, size: (u16, u16)) {
    let mut hints = std::mem::MaybeUninit::<xcb_size_hints_t>::zeroed().assume_init();
    let g = mutex_lock(&details.style);
    let resizable = g.resizable;
    std::mem::drop(g);
    if !resizable {
        hints.flags |= ICCCM_SIZE_HINT_P_MIN_SIZE;
        hints.flags |= ICCCM_SIZE_HINT_P_MAX_SIZE;
        hints.min_width = size.0 as _;
        hints.min_height = size.1 as _;
        hints.max_width = size.0 as _;
        hints.max_height = size.1 as _;
    } else {
        hints.flags |= ICCCM_SIZE_HINT_P_MIN_SIZE;
        hints.min_width = 1;
        hints.min_height = 1;
    }
    hints.flags |= ICCCM_SIZE_HINT_BASE_SIZE;
    hints.base_width = size.0 as _;
    hints.base_height = size.1 as _;
    _ = xcb_change_property(
        c,
        XCB_PROP_MODE_REPLACE,
        details.handle,
        XCB_ATOM_WM_NORMAL_HINTS,
        XCB_ATOM_WM_SIZE_HINTS,
        32,
        std::mem::size_of_val(&hints) as u32 / 4,
        (&hints) as *const _ as _,
    );
}

// Gets the window an event is destined for, if any. `None` results should be discarded.
unsafe fn get_event_window(ev: *mut xcb_generic_event_t, details: &ConnectionDetails) -> Option<xcb_window_t> {
    #[cfg(not(feature = "input"))]
    { _ = details }
    match (*ev).response_type & !(1 << 7) {
        XCB_CLIENT_MESSAGE => Some((*(ev as *mut xcb_client_message_event_t)).window),
        XCB_FOCUS_IN | XCB_FOCUS_OUT => Some((*(ev as *mut xcb_focus_in_event_t)).event),
        XCB_MAP_NOTIFY => Some((*(ev as *mut xcb_map_notify_event_t)).window),
        XCB_UNMAP_NOTIFY => Some((*(ev as *mut xcb_unmap_notify_event_t)).window),
        XCB_REPARENT_NOTIFY => Some((*(ev as *mut xcb_reparent_notify_event_t)).window),
        XCB_CONFIGURE_NOTIFY => Some((*(ev as *mut xcb_configure_notify_event_t)).window),
        XCB_PROPERTY_NOTIFY => Some((*(ev as *mut xcb_property_notify_event_t)).window),
        #[cfg(feature = "input")]
        XCB_GE_GENERIC => {
            let event = &*(ev as *mut xcb_ge_generic_event_t);
            if event.extension == details.extensions.xinput {
                match event.event_type & !(1 << 7) {
                    XCB_INPUT_KEY_PRESS | XCB_INPUT_KEY_RELEASE | XCB_INPUT_BUTTON_PRESS | XCB_INPUT_BUTTON_RELEASE | XCB_INPUT_MOTION
                        => Some((*(ev as *mut xcb_input_button_press_event_t)).event),
                    XCB_INPUT_ENTER | XCB_INPUT_LEAVE | XCB_INPUT_FOCUS_IN | XCB_INPUT_FOCUS_OUT
                        => Some((*(ev as *mut xcb_input_enter_event_t)).event),
                    _ => None,
                }
            } else {
                None
            }
        },
        _ => None,
    }
}

// This function assumes the given event is destined for the given Window - check first with get_event_window
unsafe fn process_event(ev: *mut xcb_generic_event_t, window: &mut WindowDetails, details: &ConnectionDetails) {
    let is_send_event = ((*ev).response_type >> 7) != 0;
    match (*ev).response_type & !(1 << 7) {
        XCB_CLIENT_MESSAGE => {
            let event = &mut *(ev as *mut xcb_client_message_event_t);
            if event.r#type == details.atoms.wm_protocols && event.format == 32 {
                if event.client_data.data32[0] == details.atoms.wm_delete_window {
                    window.event_buffer.push(Event::CloseRequest)
                } else if event.client_data.data32[0] == details.atoms._net_wm_ping {
                    // data32[2] contains the window xid, that might be useful for something?
                    event.window = (*details.screen).root;
                    xcb_discard_reply(details.connection, xcb_send_event_checked(
                        details.connection,
                        false.into(),
                        event.window,
                        XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY | XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT,
                        (event as *const _) as *const i8,
                    ));
                    let _ = xcb_flush(details.connection); // Makes sure the event is processed before we free it
                }
            }
        },
        e @ XCB_FOCUS_IN | e @ XCB_FOCUS_OUT => {
            let state = e == XCB_FOCUS_IN;
            window.event_buffer.push(Event::Focus(state));
        },
        XCB_REPARENT_NOTIFY => {
            let event = &*(ev as *mut xcb_reparent_notify_event_t);
            window.parent = event.parent;
        },
        XCB_CONFIGURE_NOTIFY => {
            let event = &*(ev as *mut xcb_configure_notify_event_t);
            let wh = (event.width, event.height);
            if window.size != wh {
                window.size = wh;
                window.event_buffer.push(Event::Resize(wh));
            }

            let (mut x, mut y) = (event.x, event.y);
            if !is_send_event && window.parent != (*details.screen).root {
                let mut err: *mut xcb_generic_error_t = std::ptr::null_mut();
                let cookie = xcb_translate_coordinates(details.connection, event.window, (*details.screen).root, 0, 0);
                let reply = xcb_translate_coordinates_reply(details.connection, cookie, &mut err);
                if !reply.is_null() {
                    let r = &*reply;
                    x = r.dst_x;
                    y = r.dst_y;
                    free(reply.cast());
                } else {
                    // Errors: Window
                    if !err.is_null() {
                        free(err.cast());
                    }
                    free(ev.cast());
                    return;
                }
            }
            let xy = (x, y);
            if window.position != xy {
                window.position = xy;
                window.event_buffer.push(Event::Move(xy));
            }
        },
        XCB_PROPERTY_NOTIFY => {
            let event = &*(ev as *mut xcb_property_notify_event_t);
            if event.atom == details.atoms._net_wm_state {
                match event.state {
                    XCB_PROPERTY_NEW_VALUE => {
                        let prop = xcb_get_property_reply(details.connection, xcb_get_property(
                            details.connection,
                            0,
                            window.handle,
                            details.atoms._net_wm_state,
                            XCB_ATOM_ATOM,
                            0,
                            !0,
                        ), std::ptr::null_mut());
                        if prop.is_null() {
                            return;
                        }
                        if (*prop).r#type != XCB_ATOM_ATOM || (*prop).format != 32 {
                            free(prop.cast());
                            return;
                        }
                        let len = xcb_get_property_value_length(prop);
                        let data = xcb_get_property_value(prop) as *const xcb_atom_t;
                        let len = match usize::try_from(len / 4) {
                            Ok(l) => l,
                            _ => {
                                free(prop.cast());
                                return
                            },
                        };
                        let data_slice = std::slice::from_raw_parts(data, len);
                        let minimised = data_slice.iter().any(|x| *x == details.atoms._net_wm_state_hidden);
                        let maximised = (
                            data_slice.iter().any(|x| *x == details.atoms._net_wm_state_maximized_horz),
                            data_slice.iter().any(|x| *x == details.atoms._net_wm_state_maximized_vert),
                        );
                        free(prop.cast());

                        if minimised && !window.state_minimised {
                            if window.state_maximised == (true, true) {
                                window.event_buffer.push(Event::Maximise(false));
                            }
                            window.event_buffer.push(Event::Minimise(true));
                        } else if !minimised && window.state_minimised {
                            window.event_buffer.push(Event::Minimise(false));
                            if maximised == (true, true) {
                                window.event_buffer.push(Event::Maximise(true));
                            }
                        } else if !minimised {
                            if maximised == (true, true) && window.state_maximised != (true, true) {
                                window.event_buffer.push(Event::Maximise(true));
                            } else if maximised != (true, true) && window.state_maximised == (true, true) {
                                window.event_buffer.push(Event::Maximise(false));
                            }
                        }

                        window.state_maximised = maximised;
                        window.state_minimised = minimised;
                    },
                    XCB_PROPERTY_DELETE => {
                        // The whole state property got deleted for some reason?
                        if window.state_minimised {
                            window.event_buffer.push(Event::Minimise(false));
                        } else if window.state_maximised == (true, true) {
                            window.event_buffer.push(Event::Maximise(false));
                        }
                        window.state_minimised = false;
                        window.state_maximised = (false, false);
                    },
                    _ => (),
                }
            }
        },
        XCB_MAP_NOTIFY => window.event_buffer.push(Event::Visible(true)),
        XCB_UNMAP_NOTIFY => window.event_buffer.push(Event::Visible(false)),
        #[cfg(feature = "input")]
        XCB_GE_GENERIC => {
            let event = &*(ev as *mut xcb_ge_generic_event_t);
            if event.extension == details.extensions.xinput {
                match event.event_type & !(1 << 7) {
                    e @ XCB_INPUT_KEY_PRESS | e @ XCB_INPUT_KEY_RELEASE => {
                        let is_press = e == XCB_INPUT_KEY_PRESS;
                        let event = &*(ev as *mut xcb_input_key_press_event_t);
                        let mut xevent = XKeyEvent {
                            r#type: 2,
                            serial: 0,
                            send_event: 0,
                            display: details.display,
                            window: 0,
                            root: 0,
                            subwindow: 0,
                            time: 0,
                            x: 0,
                            y: 0,
                            x_root: 0,
                            y_root: 0,
                            state: event.mods.effective,
                            keycode: event.detail,
                            same_screen: 0,
                        };
                        let repeat = (event.flags & XCB_INPUT_KEY_EVENT_FLAGS_KEY_REPEAT) != 0;
                        let f = if is_press {
                            if repeat { Event::KeyboardRepeat } else { Event::KeyboardDown }
                        } else {
                            Event::KeyboardUp
                        };
                        let unmodified_keysym = XLookupKeysym(&mut xevent, 0);
                        let mut modified_keysym: KeySym = 0;
                        let _ = XLookupString(
                            &mut xevent,
                            std::ptr::null_mut(),
                            0,
                            &mut modified_keysym,
                            std::ptr::null_mut(),
                        );

                        if let Some(k) = keysym_to_key(unmodified_keysym, modified_keysym) {
                            window.event_buffer.push(f(k));
                        }

                        if is_press {
                            if let Ok(utf32) = u32::try_from(modified_keysym) {
                                if let Some(ch) = char::from_u32(xkb_keysym_to_utf32(utf32)) {
                                    if ch != '\0' {
                                        window.event_buffer.push(Event::Input(ch));
                                    }
                                }
                            }
                        }
                    },
                    e @ XCB_INPUT_BUTTON_PRESS | e @ XCB_INPUT_BUTTON_RELEASE => {
                        use crate::input::MouseButton;
                        let event = &*(ev as *mut xcb_input_button_press_event_t);
                        let f = if e == XCB_INPUT_BUTTON_PRESS { Event::MouseDown } else { Event::MouseUp };
                        match event.detail {
                            1 => window.event_buffer.push(f(MouseButton::Left)),
                            2 => window.event_buffer.push(f(MouseButton::Middle)),
                            3 => window.event_buffer.push(f(MouseButton::Right)),
                            4 if e == XCB_INPUT_BUTTON_PRESS => window.event_buffer.push(Event::ScrollUp),
                            5 if e == XCB_INPUT_BUTTON_PRESS => window.event_buffer.push(Event::ScrollDown),
                            _ => (),
                        }
                    },
                    XCB_INPUT_MOTION => {
                        let event = &*(ev as *mut xcb_input_motion_event_t);
                        window.event_buffer.push(Event::MouseMove(((event.event_x >> 16) as _, (event.event_y >> 16) as _)))
                    },
                    XCB_INPUT_ENTER => {
                        let _event = &*(ev as *mut xcb_input_enter_event_t);
                        window.event_buffer.push(Event::MouseEnter);
                    },
                    XCB_INPUT_LEAVE => {
                        let _event = &*(ev as *mut xcb_input_leave_event_t);
                        window.event_buffer.push(Event::MouseLeave);
                    },
                    e @ XCB_INPUT_FOCUS_IN | e @ XCB_INPUT_FOCUS_OUT => {
                        let state = e == XCB_INPUT_FOCUS_IN;
                        window.event_buffer.push(Event::Focus(state))
                    },
                    _ => (),
                }
            }
        },
        _ => (),
    };
    free(ev.cast());
}

// assumes we hold connection lock
unsafe fn internal_set_maximised(c: *mut xcb_connection_t, xid: xcb_window_t, details: &ConnectionDetails, maximised: bool) {
    let action = if maximised { 1 } else { 0 };
    let client_message = xcb_client_message_event_t {
        response_type: XCB_CLIENT_MESSAGE,
        format: 32,
        sequence: 0,
        window: xid,
        r#type: details.atoms._net_wm_state,
        client_data: ClientData { data32: [
            action,
            details.atoms._net_wm_state_maximized_horz,
            details.atoms._net_wm_state_maximized_vert,
            1,
            0,
        ] },
    };
    xcb_discard_reply(c, xcb_send_event_checked(
        c,
        0,
        (*details.screen).root,
        XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY | XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT,
        (&client_message as *const _) as *const i8,
    ));
}

// assumes we hold connection lock
unsafe fn internal_set_title(c: *mut xcb_connection_t, xid: xcb_window_t, atoms: &Atoms, title: &str) {
    let _ = xcb_change_property(
        c,
        XCB_PROP_MODE_REPLACE,
        xid,
        atoms._net_wm_name,
        atoms.utf8_string,
        8,
        title.bytes().len() as _,
        title.as_ptr().cast(),
    );
    let _ = xcb_change_property(
        c,
        XCB_PROP_MODE_REPLACE,
        xid,
        XCB_ATOM_WM_NAME,
        XCB_ATOM_STRING,
        8,
        title.bytes().len() as _,
        title.as_ptr().cast(),
    );
}

#[cfg(feature = "input")]
use crate::input::Key;
#[cfg(feature = "input")]
fn keysym_to_key(keysym: KeySym, keysym2: KeySym) -> Option<Key> {
    // This function converts a keysym, as returned by XLookupKeysym, to a ramen key.
    // X does have multiple keysyms per key (for example, XK_A vs XK_a depending if shift is held),
    // however, XLookupKeysym ignores all modifiers, so this function should only receive "base" keysym values.
    // To avoid some annoying situations we also request keysym2 which is the modified keysym.
    // Values mostly copied from <X11/keysymdef.h>
    match keysym {
        0x20 => Some(Key::Space),
        0x22 => Some(Key::Quote),
        0x23 => Some(Key::Hash),
        0x27 => Some(Key::Apostrophe),
        0x2B => Some(Key::Plus),
        0x2C => Some(Key::Comma),
        0x2D => Some(Key::Minus),
        0x2E => Some(Key::Period),
        0x2F => Some(Key::Slash),
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
        0x3A => Some(Key::Colon),
        0x3B => Some(Key::Semicolon),
        0x3C => Some(Key::LessThan),
        0x3D => Some(Key::Equals),
        0x3E => Some(Key::GreaterThan),
        0x3F => Some(Key::QuestionMark),
        0x5B => Some(Key::BracketLeft),
        0x5C => Some(Key::Backslash),
        0x5D => Some(Key::BracketRight),
        0x5F => Some(Key::Underscore),
        0x60 => Some(Key::Grave),
        0x61 => Some(Key::A),
        0x62 => Some(Key::B),
        0x63 => Some(Key::C),
        0x64 => Some(Key::D),
        0x65 => Some(Key::E),
        0x66 => Some(Key::F),
        0x67 => Some(Key::G),
        0x68 => Some(Key::H),
        0x69 => Some(Key::I),
        0x6A => Some(Key::J),
        0x6B => Some(Key::K),
        0x6C => Some(Key::L),
        0x6D => Some(Key::M),
        0x6E => Some(Key::N),
        0x6F => Some(Key::O),
        0x70 => Some(Key::P),
        0x71 => Some(Key::Q),
        0x72 => Some(Key::R),
        0x73 => Some(Key::S),
        0x74 => Some(Key::T),
        0x75 => Some(Key::U),
        0x76 => Some(Key::V),
        0x77 => Some(Key::W),
        0x78 => Some(Key::X),
        0x79 => Some(Key::Y),
        0x7A => Some(Key::Z),
        0x7B => Some(Key::BraceLeft),
        0x7C => Some(Key::Pipe),
        0x7D => Some(Key::BraceRight),
        0xFF08 => Some(Key::Backspace),
        0xFF09 => Some(Key::Tab),
        0xFF0D => Some(Key::Return),
        0xFF13 => Some(Key::Pause),
        0xFF14 => Some(Key::ScrollLock),
        0xFF1B => Some(Key::Escape),
        0xFF50 => Some(Key::Home),
        0xFF51 => Some(Key::LeftArrow),
        0xFF52 => Some(Key::UpArrow),
        0xFF53 => Some(Key::RightArrow),
        0xFF54 => Some(Key::DownArrow),
        0xFF55 => Some(Key::PageUp),
        0xFF56 => Some(Key::PageDown),
        0xFF57 => Some(Key::End),
        0xFF58 => Some(Key::Home), // From numpad keysym names I'm pretty confident Begin and Home mean the same thing
        0xFF63 => Some(Key::Insert),
        0xFF7F => Some(Key::NumLock),
        0xFFBE => Some(Key::F1),
        0xFFBF => Some(Key::F2),
        0xFFC0 => Some(Key::F3),
        0xFFC1 => Some(Key::F4),
        0xFFC2 => Some(Key::F5),
        0xFFC3 => Some(Key::F6),
        0xFFC4 => Some(Key::F7),
        0xFFC5 => Some(Key::F8),
        0xFFC6 => Some(Key::F9),
        0xFFC7 => Some(Key::F10),
        0xFFC8 => Some(Key::F11),
        0xFFC9 => Some(Key::F12),
        0xFFCA => Some(Key::F13),
        0xFFCB => Some(Key::F14),
        0xFFCC => Some(Key::F15),
        0xFFCD => Some(Key::F16),
        0xFFCE => Some(Key::F17),
        0xFFCF => Some(Key::F18),
        0xFFD0 => Some(Key::F19),
        0xFFD1 => Some(Key::F20),
        0xFFD2 => Some(Key::F21),
        0xFFD3 => Some(Key::F22),
        0xFFD4 => Some(Key::F23),
        0xFFD5 => Some(Key::F24),
        0xFFE1 => Some(Key::LeftShift),
        0xFFE2 => Some(Key::RightShift),
        0xFFE3 => Some(Key::LeftControl),
        0xFFE4 => Some(Key::RightControl),
        0xFFE5 => Some(Key::CapsLock),
        0xFFE9 => Some(Key::LeftAlt),
        0xFFEA => Some(Key::RightAlt),
        0xFFEB => Some(Key::LeftSuper),
        0xFFEC => Some(Key::RightSuper),
        0xFFFF => Some(Key::Delete),
        
        (0xFF80..=0xFFB9) => match keysym2 {
            // We use the modified keysym for numpad keys because modifiers actually change our mapping rules
            // eg: numpad "0" maps to either `Keypad0` or `Insert` depending on the states of shift and numlock
            0xFF80 => Some(Key::Space),
            0xFF89 => Some(Key::Tab),
            0xFF8D => Some(Key::Return),
            0xFF91 => Some(Key::F1),
            0xFF92 => Some(Key::F2),
            0xFF93 => Some(Key::F3),
            0xFF94 => Some(Key::F4),
            0xFF95 => Some(Key::Home),
            0xFF96 => Some(Key::LeftArrow),
            0xFF97 => Some(Key::UpArrow),
            0xFF98 => Some(Key::RightArrow),
            0xFF99 => Some(Key::DownArrow),
            0xFF9A => Some(Key::PageUp),
            0xFF9B => Some(Key::PageDown),
            0xFF9C => Some(Key::End),
            0xFF9D => Some(Key::Home),
            0xFF9E => Some(Key::Insert),
            0xFF9F => Some(Key::Delete),
            0xFFAA => Some(Key::KeypadMultiply),
            0xFFAB => Some(Key::KeypadAdd),
            0xFFAC => Some(Key::KeypadSeparator),
            0xFFAD => Some(Key::KeypadSubtract),
            0xFFAE => Some(Key::KeypadDecimal),
            0xFFAF => Some(Key::KeypadDivide),
            0xFFB0 => Some(Key::Keypad0),
            0xFFB1 => Some(Key::Keypad1),
            0xFFB2 => Some(Key::Keypad2),
            0xFFB3 => Some(Key::Keypad3),
            0xFFB4 => Some(Key::Keypad4),
            0xFFB5 => Some(Key::Keypad5),
            0xFFB6 => Some(Key::Keypad6),
            0xFFB7 => Some(Key::Keypad7),
            0xFFB8 => Some(Key::Keypad8),
            0xFFB9 => Some(Key::Keypad9),
            _ => None,
        },
        _ => None,
    }
}
