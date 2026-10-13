#![allow(bad_style)]
#![allow(clippy::too_many_arguments)]

pub(super) use libc::{c_char, c_int, c_uint, c_void, free, getpid};
#[cfg(feature = "input")]
pub(super) use input::*;

use libc::{dlsym, dlerror};
unsafe fn dlopen(name: *const c_char) -> *mut c_void {
    libc::dlopen(name, libc::RTLD_LOCAL | libc::RTLD_LAZY)
}

load! {
    pub(super) xlib(libX11) "libX11.so.6", "libX11.so" {
        fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
        fn XDefaultScreen(display: *mut Display) -> c_int;
        fn XCloseDisplay(display: *mut Display) -> c_int;
        #[cfg(feature = "input")]
        fn XLookupKeysym(event_struct: *mut XKeyEvent, index: c_int) -> KeySym;
        #[cfg(feature = "input")]
        fn XLookupString(event_struct: *mut XKeyEvent, buffer_return: *mut c_char, bytes_buffer: c_int, keysym_return: *mut KeySym, status_in_out: *mut c_void) -> c_int;
    }
    pub(super) xlib_xcb(libX11_xcb) "libX11-xcb.so.1", "libX11-xcb.so" {
        fn XGetXCBConnection(dpy: *mut Display) -> *mut xcb_connection_t;
        fn XSetEventQueueOwner(dpy: *mut Display, owner: EventQueueOwner);
    }
    pub(super) xcb(libxcb) "libxcb.so.1", "libxcb.so" {
        //fn xcb_connect(displayname: *const c_char, screenp: *mut c_int) -> *mut xcb_connection_t;
        fn xcb_connection_has_error(c: *mut xcb_connection_t) -> c_int;
        //fn xcb_disconnect(c: *mut xcb_connection_t);
        fn xcb_discard_reply(c: *mut xcb_connection_t, sequence: c_uint);
        fn xcb_get_setup(c: *mut xcb_connection_t) -> *const xcb_setup_t;
        fn xcb_setup_roots_iterator(R: *const xcb_setup_t) -> xcb_screen_iterator_t;
        fn xcb_screen_next(i: *mut xcb_screen_iterator_t);

        fn xcb_flush(c: *mut xcb_connection_t) -> c_int;
        fn xcb_generate_id(c: *mut xcb_connection_t) -> u32;
        fn xcb_request_check(c: *mut xcb_connection_t, sequence: c_uint) -> *mut xcb_generic_error_t;
        fn xcb_create_colormap(c: *mut xcb_connection_t, alloc: u8, mid: xcb_colormap_t, window: xcb_window_t, visual: xcb_visualid_t) -> c_uint;
        fn xcb_create_window_checked(
            c: *mut xcb_connection_t,
            depth: u8,
            wid: xcb_window_t,
            parent: xcb_window_t,
            x: i16,
            y: i16,
            width: u16,
            height: u16,
            border_width: u16,
            class: u16,
            visual: xcb_visualid_t,
            value_mask: u32,
            value_list: *const u32,
        ) -> c_uint;
        fn xcb_configure_window(
            c: *mut xcb_connection_t,
            window: xcb_window_t,
            value_mask: u16,
            value_list: *const c_void,
        ) -> c_uint;
        fn xcb_change_property(
            c: *mut xcb_connection_t,
            mode: u8,
            window: xcb_window_t,
            property: xcb_atom_t,
            r#type: xcb_atom_t,
            format: u8,
            data_len: u32,
            data: *const c_void,
        ) -> c_uint;
        fn xcb_translate_coordinates(
            c: *mut xcb_connection_t,
            src: xcb_window_t,
            dest: xcb_window_t,
            srcx: i16,
            srcy: i16,
        ) -> c_uint;
        fn xcb_translate_coordinates_reply(
            c: *mut xcb_connection_t,
            cookie: c_uint,
            err: *mut *mut xcb_generic_error_t,
        ) -> *mut xcb_translate_coordinates_reply_t;
        fn xcb_map_window(c: *mut xcb_connection_t, window: xcb_window_t) -> c_uint;
        fn xcb_unmap_window(c: *mut xcb_connection_t, window: xcb_window_t) -> c_uint;
        fn xcb_intern_atom(
            c: *mut xcb_connection_t,
            only_if_exists: u8,
            name_len: u16,
            name: *const c_char,
        ) -> c_uint;
        fn xcb_intern_atom_reply(
            c: *mut xcb_connection_t,
            sequence: c_uint,
            e: *mut *mut xcb_generic_error_t,
        ) -> *mut xcb_intern_atom_reply_t;
        fn xcb_poll_for_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
        fn xcb_poll_for_queued_event(c: *mut xcb_connection_t) -> *mut xcb_generic_event_t;
        fn xcb_send_event_checked(c: *mut xcb_connection_t, propagate: u8, destination: xcb_window_t, event_mask: u32, event: *const c_char) -> c_uint;
        fn xcb_destroy_window(c: *mut xcb_connection_t, xid: xcb_window_t) -> c_uint;
        fn xcb_get_property(c: *mut xcb_connection_t, delete: u8, window: xcb_window_t, property: xcb_atom_t, r#type: xcb_atom_t, long_offset: u32, long_length: u32) -> c_uint;
        fn xcb_get_property_reply(c: *mut xcb_connection_t, sequence: c_uint, e: *mut *mut xcb_generic_error_t) -> *mut xcb_get_property_reply_t;
        fn xcb_get_property_value(reply: *const xcb_get_property_reply_t) -> *mut c_void;
        fn xcb_get_property_value_length(reply: *const xcb_get_property_reply_t) -> c_int;
        #[cfg(feature = "input")]
        fn xcb_query_extension(c: *mut xcb_connection_t, name_len: u16, name: *const c_char) -> c_uint;
        #[cfg(feature = "input")]
        fn xcb_query_extension_reply(c: *mut xcb_connection_t, sequence: c_uint, e: *mut *mut xcb_generic_error_t) -> *mut xcb_query_extension_reply_t;
    }
    #[cfg(feature = "input")]
    pub(super) xinput(libxcb_xinput) "libxcb-xinput.so.0", "libxcb-xinput.so" {
        #[cfg(feature = "input")]
        fn xcb_input_xi_select_events_checked(c: *mut xcb_connection_t, window: xcb_window_t, num_mask: u16, masks: *mut xcb_input_event_mask_t) -> c_uint;
    }
    #[cfg(feature = "input")]
    pub(super) xkbcommon(libxkbcommon) "libxkbcommon.so.0", "libxkbcommon.so" {
        #[cfg(feature = "input")]
        fn xkb_keysym_to_utf32(keysym: u32) -> u32;
    }
}

pub enum Display {}
pub(super) enum xcb_setup_t {}
pub(super) enum xcb_connection_t {}

#[repr(C)]
#[allow(dead_code)]
pub(super) enum EventQueueOwner { XlibOwnsEventQueue = 0, XCBOwnsEventQueue }
pub(super) type xcb_atom_t = u32;
pub(super) type xcb_colormap_t = u32;
pub(super) type xcb_visualid_t = u32;
pub type xcb_window_t = u32;

pub(super) const XCB_WINDOW_CLASS_INPUT_OUTPUT: u16 = 1;
pub(super) const XCB_COPY_FROM_PARENT: u8 = 0;
// pub(super) const XCB_KEY_PRESS: u8 = 2;
// pub(super) const XCB_KEY_RELEASE: u8 = 3;
// pub(super) const XCB_BUTTON_PRESS: u8 = 4;
// pub(super) const XCB_BUTTON_RELEASE: u8 = 5;
pub(super) const XCB_FOCUS_IN: u8 = 9;
pub(super) const XCB_FOCUS_OUT: u8 = 10;
pub(super) const XCB_UNMAP_NOTIFY: u8 = 18;
pub(super) const XCB_MAP_NOTIFY: u8 = 19;
pub(super) const XCB_REPARENT_NOTIFY: u8 = 21;
pub(super) const XCB_CONFIGURE_NOTIFY: u8 = 22;
pub(super) const XCB_PROPERTY_NOTIFY: u8 = 28;
pub(super) const XCB_CLIENT_MESSAGE: u8 = 33;
#[cfg(feature = "input")]
pub(super) const XCB_GE_GENERIC: u8 = 35;

pub(super) const XCB_PROP_MODE_REPLACE: u8 = 0;
//pub(super) const XCB_PROP_MODE_APPEND: u8 = 1;
//pub(super) const XCB_PROP_MODE_PREPEND: u8 = 2;

//pub(super) const XCB_ATOM_NONE: xcb_atom_t = 0;
pub(super) const XCB_ATOM_ATOM: xcb_atom_t = 4;
pub(super) const XCB_ATOM_CARDINAL: xcb_atom_t = 6;
pub(super) const XCB_ATOM_STRING: xcb_atom_t = 31;
pub(super) const XCB_ATOM_WM_NAME: xcb_atom_t = 39;
pub(super) const XCB_ATOM_WM_CLASS: xcb_atom_t = 67;

pub(super) const XCB_CW_COLORMAP: u32 = 8192;
pub(super) const XCB_CW_EVENT_MASK: u32 = 2048;
#[cfg(feature = "input")]
pub(super) const XCB_EVENT_MASK_BUTTON_PRESS: u32 = 4;
pub(super) const XCB_EVENT_MASK_STRUCTURE_NOTIFY: u32 = 131072;
pub(super) const XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY: u32 = 524288;
pub(super) const XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT: u32 = 1048576;
pub(super) const XCB_EVENT_MASK_PROPERTY_CHANGE: u32 = 4194304;
#[cfg(not(feature = "input"))]
pub(super) const XCB_EVENT_MASK_FOCUS_CHANGE: u32 = 2097152;

pub(super) const XCB_NONE: c_int = 0;
pub(super) const XCB_ALLOC: c_int = 11;
pub(super) const XCB_CONN_CLOSED_EXT_NOTSUPPORTED: c_int = 2;
pub(super) const XCB_CONN_CLOSED_MEM_INSUFFICIENT: c_int = 3;

pub(super) const ICCCM_SIZE_HINT_P_MIN_SIZE: u32 = 1 << 4;
pub(super) const ICCCM_SIZE_HINT_P_MAX_SIZE: u32 = 1 << 5;
pub(super) const ICCCM_SIZE_HINT_BASE_SIZE: u32 = 1 << 8;

#[repr(C)]
pub(super) struct xcb_generic_error_t {
    pub(super) response_type: u8,
    pub(super) error_code: u8,
    pub(super) sequence: u16,
    pub(super) resource_id: u32,
    pub(super) minor_code: u16,
    pub(super) major_code: u8,
    pub(super) _pad0: u8,
    pub(super) _pad: [u32; 5],
    pub(super) full_sequence: u32,
}

#[repr(C)]
pub(super) struct xcb_screen_iterator_t {
    pub(super) data: *mut xcb_screen_t,
    pub(super) rem: c_int,
    pub(super) index: c_int,
}

#[repr(C)]
pub(super) struct xcb_size_hints_t {
    pub(super) flags: u32,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) min_width: i32,
    pub(super) min_height: i32,
    pub(super) max_width: i32,
    pub(super) max_height: i32,
    pub(super) width_inc: i32,
    pub(super) height_inc: i32,
    pub(super) min_aspect_num: i32,
    pub(super) min_aspect_den: i32,
    pub(super) max_aspect_num: i32,
    pub(super) max_aspect_den: i32,
    pub(super) base_width: i32,
    pub(super) base_height: i32,
    pub(super) win_gravity: u32,
}

#[repr(C)]
pub(super) struct xcb_screen_t {
    pub(super) root: xcb_window_t,
    pub(super) default_colourmap: xcb_colormap_t,
    pub(super) white_pixel: u32,
    pub(super) black_pixel: u32,
    pub(super) current_input_masks: u32,
    pub(super) width_in_pixels: u16,
    pub(super) height_in_pixels: u16,
    pub(super) width_in_millimeters: u16,
    pub(super) height_in_millimeters: u16,
    pub(super) min_installed_maps: u16,
    pub(super) max_installed_maps: u16,
    pub(super) root_visual: xcb_visualid_t,
    pub(super) backing_stores: u8,
    pub(super) save_unders: u8,
    pub(super) root_depth: u8,
    pub(super) allowed_depths_len: u8,
}

#[repr(C)]
pub(super) struct xcb_translate_coordinates_reply_t {
    pub(super) response_type: u8,
    pub(super) same_screen: u8,
    pub(super) sequence: u16,
    pub(super) length: u32,
    pub(super) child: xcb_window_t,
    pub(super) dst_x: i16,
    pub(super) dst_y: i16,
}

#[repr(C)]
pub(super) struct xcb_intern_atom_reply_t {
    pub(super) response_type: u8,
    pub(super) pad0: u8,
    pub(super) sequence: u16,
    pub(super) length: u32,
    pub(super) atom: xcb_atom_t,
}

#[repr(C)]
pub(super) struct xcb_get_property_reply_t {
    pub(super) response_type: u8,
    pub(super) format: u8,
    pub(super) sequence: u16,
    pub(super) length: u32,
    pub(super) r#type: xcb_atom_t,
    pub(super) bytes_after: u32,
    pub(super) value_len: u32,
    pub(super) _pad0: [u8; 12],
}

#[cfg(feature = "input")]
#[repr(C)]
pub(super) struct xcb_query_extension_reply_t {
    pub(super) response_type: u8,
    pub(super) _pad0: u8,
    pub(super) sequence: u16,
    pub(super) length: u32,
    pub(super) present: u8,
    pub(super) major_opcode: u8,
    pub(super) first_event: u8,
    pub(super) first_error: u8,
}

#[repr(C)]
pub(super) struct xcb_generic_event_t {
    pub(super) response_type: u8,
    pub(super) _pad0: u8,
    pub(super) sequence: u16,
    pub(super) _pad: [u32; 7],
    pub(super) full_sequence: u32,
}

#[repr(C)]
pub(super) struct xcb_client_message_event_t {
    pub(super) response_type: u8,
    pub(super) format: u8,
    pub(super) sequence: u16,
    pub(super) window: xcb_window_t,
    pub(super) r#type: xcb_atom_t,
    pub(super) client_data: ClientData,
}

#[repr(C)]
pub(super) struct xcb_focus_in_event_t {
    pub(super) response_type: u8,
    pub(super) detail: u8,
    pub(super) sequence: u16,
    pub(super) event: xcb_window_t,
    pub(super) mode: u8,
    pub(super) _pad0: [u8; 3],
}

#[repr(C)]
pub(super) struct xcb_configure_notify_event_t {
    pub(super) response_type: u8,
    pub(super) _pad0: u8,
    pub(super) sequence: u16,
    pub(super) event: xcb_window_t,
    pub(super) window: xcb_window_t,
    pub(super) above_sibling: xcb_window_t,
    pub(super) x: i16,
    pub(super) y: i16,
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) border_width: u16,
    pub(super) override_redirect: u8,
    pub(super) _pad1: u8,
}

#[repr(C)]
pub(super) struct xcb_reparent_notify_event_t {
    pub(super) response_type: u8,
    pub(super) _pad0: u8,
    pub(super) sequence: u16,
    pub(super) event: xcb_window_t,
    pub(super) window: xcb_window_t,
    pub(super) parent: xcb_window_t,
    pub(super) x: i16,
    pub(super) y: i16,
    pub(super) override_redirect: u8,
    pub(super) _pad1: [u8; 3],
}

#[repr(C)]
pub(super) struct xcb_map_notify_event_t {
    pub(super) response_type: u8,
    pub(super) _pad0: u8,
    pub(super) sequence: u16,
    pub(super) event: xcb_window_t,
    pub(super) window: xcb_window_t,
    pub(super) override_redirect: u8,
    pub(super) _pad1: [u8; 3],
}

#[repr(C)]
pub(super) struct xcb_unmap_notify_event_t {
    pub(super) response_type: u8,
    pub(super) _pad0: u8,
    pub(super) sequence: u16,
    pub(super) event: xcb_window_t,
    pub(super) window: xcb_window_t,
    pub(super) from_configure: u8,
    pub(super) _pad1: [u8; 3],
}

#[repr(C)]
pub(super) struct xcb_property_notify_event_t {
    pub(super) response_type: u8,
    pub(super) _pad0: u8,
    pub(super) sequence: u16,
    pub(super) window: xcb_window_t,
    pub(super) atom: xcb_atom_t,
    pub(super) time: xcb_timestamp_t,
    pub(super) state: u8,
    pub(super) _pad1: [u8; 3],
}
pub(super) const XCB_PROPERTY_NEW_VALUE: u8 = 0;
pub(super) const XCB_PROPERTY_DELETE: u8 = 1;
pub(super) const XCB_ATOM_WM_NORMAL_HINTS: u32 = 40;
pub(super) const XCB_ATOM_WM_SIZE_HINTS: u32 = 41;

#[repr(C)]
pub(crate) union ClientData {
    pub(crate) data8: [u8; 20],
    pub(crate) data16: [u16; 10],
    pub(crate) data32: [u32; 5],
}

#[cfg(feature = "input")]
#[repr(C)]
pub(super) struct xcb_ge_generic_event_t {
    pub(super) response_type: u8,
    pub(super) extension: u8,
    pub(super) sequence: u16,
    pub(super) length: u32,
    pub(super) event_type: u16,
    pub(super) _pad0: [u8; 22],
    pub(super) full_sequence: u32,
}

pub(super) type xcb_timestamp_t = u32;

#[cfg(feature = "input")]
mod input {
    use super::*;
    pub(in super::super) use libc::c_ulong;

    pub(in super::super) type xcb_input_device_id_t = u16;

    //pub(in super::super) type xcb_input_xi_event_mask_t = u32;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_DEVICE_CHANGED: u32 = 2;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_KEY_PRESS: u32 = 4;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_KEY_RELEASE: u32 = 8;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_BUTTON_PRESS: u32 = 16;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_BUTTON_RELEASE: u32 = 32;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_MOTION: u32 = 64;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_ENTER: u32 = 128;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_LEAVE: u32 = 256;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_FOCUS_IN: u32 = 512;
    pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_FOCUS_OUT: u32 = 1024;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_HIERARCHY: u32 = 2048;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_PROPERTY: u32 = 4096;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_RAW_KEY_PRESS: u32 = 8192;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_RAW_KEY_RELEASE: u32 = 16384;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_RAW_BUTTON_PRESS: u32 = 32768;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_RAW_BUTTON_RELEASE: u32 = 65536;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_RAW_MOTION: u32 = 131072;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_TOUCH_BEGIN: u32 = 262144;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_TOUCH_UPDATE: u32 = 524288;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_TOUCH_END: u32 = 1048576;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_TOUCH_OWNERSHIP: u32 = 2097152;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_RAW_TOUCH_BEGIN: u32 = 4194304;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_RAW_TOUCH_UPDATE: u32 = 8388608;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_RAW_TOUCH_END: u32 = 16777216;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_BARRIER_HIT: u32 = 33554432;
    //pub(in super::super) const XCB_INPUT_XI_EVENT_MASK_BARRIER_LEAVE: u32 = 67108864;

    //pub(in super::super) const XCB_INPUT_DEVICE_ALL: u16 = 0;
    pub(in super::super) const XCB_INPUT_DEVICE_ALL_MASTER: u16 = 1;

    pub(in super::super) const XCB_INPUT_KEY_PRESS: u16 = 2;
    pub(in super::super) const XCB_INPUT_KEY_RELEASE: u16 = 3;
    pub(in super::super) const XCB_INPUT_BUTTON_PRESS: u16 = 4;
    pub(in super::super) const XCB_INPUT_BUTTON_RELEASE: u16 = 5;
    pub(in super::super) const XCB_INPUT_MOTION: u16 = 6;
    pub(in super::super) const XCB_INPUT_ENTER: u16 = 7;
    pub(in super::super) const XCB_INPUT_LEAVE: u16 = 8;
    pub(in super::super) const XCB_INPUT_FOCUS_IN: u16 = 9;
    pub(in super::super) const XCB_INPUT_FOCUS_OUT: u16 = 10;

    pub(in super::super) const XCB_INPUT_KEY_EVENT_FLAGS_KEY_REPEAT: u32 = 65536;

    pub(in super::super) type xcb_input_fp1616_t = i32;

    #[repr(C)]
    pub(in super::super) struct xcb_input_event_mask_t {
        pub(in super::super) deviceid: xcb_input_device_id_t,
        pub(in super::super) mask_len: u16,
    }

    #[repr(C)]
    pub(in super::super) struct xcb_input_enter_event_t {
        pub(in super::super) response_type: u8,
        pub(in super::super) extension: u8,
        pub(in super::super) sequence: u16,
        pub(in super::super) length: u32,
        pub(in super::super) event_type: u16,
        pub(in super::super) deviceid: xcb_input_device_id_t,
        pub(in super::super) time: xcb_timestamp_t,
        pub(in super::super) sourceid: xcb_input_device_id_t,
        pub(in super::super) mode: u8,
        pub(in super::super) detail: u8,
        pub(in super::super) root: xcb_window_t,
        pub(in super::super) event: xcb_window_t,
        pub(in super::super) child: xcb_window_t,
        pub(in super::super) full_sequence: u32,
        pub(in super::super) root_x: xcb_input_fp1616_t,
        pub(in super::super) root_y: xcb_input_fp1616_t,
        pub(in super::super) event_x: xcb_input_fp1616_t,
        pub(in super::super) event_y: xcb_input_fp1616_t,
        pub(in super::super) same_screen: u8,
        pub(in super::super) focus: u8,
        pub(in super::super) buttons_len: u16,
        pub(in super::super) mods: xcb_input_modifier_info_t,
        pub(in super::super) group: xcb_input_group_info_t,
    }
    pub(in super::super) type xcb_input_leave_event_t = xcb_input_enter_event_t;
    //pub(in super::super) type xcb_input_focus_in_event_t = xcb_input_enter_event_t;
    //pub(in super::super) type xcb_input_focus_out_event_t = xcb_input_enter_event_t;

    #[repr(C)]
    pub(in super::super) struct xcb_input_key_press_event_t {
        pub(in super::super) response_type: u8,
        pub(in super::super) extension: u8,
        pub(in super::super) sequence: u16,
        pub(in super::super) length: u32,
        pub(in super::super) event_type: u16,
        pub(in super::super) deviceid: xcb_input_device_id_t,
        pub(in super::super) time: xcb_timestamp_t,
        pub(in super::super) detail: u32,
        pub(in super::super) root: xcb_window_t,
        pub(in super::super) event: xcb_window_t,
        pub(in super::super) child: xcb_window_t,
        pub(in super::super) full_sequence: u32,
        pub(in super::super) root_x: xcb_input_fp1616_t,
        pub(in super::super) root_y: xcb_input_fp1616_t,
        pub(in super::super) event_x: xcb_input_fp1616_t,
        pub(in super::super) event_y: xcb_input_fp1616_t,
        pub(in super::super) buttons_len: u16,
        pub(in super::super) valuators_len: u16,
        pub(in super::super) sourceid: xcb_input_device_id_t,
        pub(in super::super) _pad0: [u8; 2],
        pub(in super::super) flags: u32,
        pub(in super::super) mods: xcb_input_modifier_info_t,
        pub(in super::super) group: xcb_input_group_info_t,
    }
    //pub(in super::super) type xcb_input_key_release_event_t = xcb_input_key_press_event_t;
    pub(in super::super) type xcb_input_button_press_event_t = xcb_input_key_press_event_t;
    //pub(in super::super) type xcb_input_button_release_event_t = xcb_input_key_press_event_t;
    pub(in super::super) type xcb_input_motion_event_t = xcb_input_key_press_event_t;

    #[repr(C)]
    pub(in super::super) struct xcb_input_modifier_info_t {
        pub(in super::super) base: u32,
        pub(in super::super) latched: u32,
        pub(in super::super) locked: u32,
        pub(in super::super) effective: u32,
    }

    #[repr(C)]
    pub(in super::super) struct xcb_input_group_info_t {
        pub(in super::super) base: u8,
        pub(in super::super) latched: u8,
        pub(in super::super) locked: u8,
        pub(in super::super) effective: u8,
    }

    pub(in super::super) type KeySym = c_ulong;
    #[repr(C)]
    pub(in super::super) struct XKeyEvent {
        pub(in super::super) r#type: c_int,
        pub(in super::super) serial: c_ulong,
        pub(in super::super) send_event: u8,
        pub(in super::super) display: *mut Display,
        pub(in super::super) window: c_ulong,
        pub(in super::super) root: c_ulong,
        pub(in super::super) subwindow: c_ulong,
        pub(in super::super) time: c_ulong,
        pub(in super::super) x: c_int,
        pub(in super::super) y: c_int,
        pub(in super::super) x_root: c_int,
        pub(in super::super) y_root: c_int,
        pub(in super::super) state: c_uint,
        pub(in super::super) keycode: c_uint,
        pub(in super::super) same_screen: u8,
    }
}


// Borrows from Xm/MwmUtil.h (see VendorShell, XmNmwmDecorations)

#[repr(C)]
pub(crate) struct MwmHints {
    pub(crate) flags: u32,
    pub(crate) functions: u32,
    pub(crate) decorations: u32,
    pub(crate) input_mode: u32,
    pub(crate) status: u32,
}

pub(crate) const MWM_HINTS_FUNCTIONS: u32 = 1 << 0;
pub(crate) const MWM_HINTS_DECORATIONS: u32 = 1 << 1;

// pub(crate) const MWM_DECOR_ALL: u32 = 1 << 0;
pub(crate) const MWM_DECOR_BORDER: u32 = 1 << 1;
pub(crate) const MWM_DECOR_RESIZEH: u32 = 1 << 2;
pub(crate) const MWM_DECOR_TITLE: u32 = 1 << 3;
pub(crate) const MWM_DECOR_MENU: u32 = 1 << 4;
// pub(crate) const MWM_DECOR_MINIMIZE: u32 = 1 << 5;
// pub(crate) const MWM_DECOR_MAXIMIZE: u32 = 1 << 6;

// pub(crate) const MWM_FUNC_ALL: u32 = 1 << 0;
pub(crate) const MWM_FUNC_RESIZE: u32 = 1 << 1;
pub(crate) const MWM_FUNC_MOVE: u32 = 1 << 2;
pub(crate) const MWM_FUNC_MINIMIZE: u32 = 1 << 3;
pub(crate) const MWM_FUNC_MAXIMIZE: u32 = 1 << 4;
pub(crate) const MWM_FUNC_CLOSE: u32 = 1 << 5;
