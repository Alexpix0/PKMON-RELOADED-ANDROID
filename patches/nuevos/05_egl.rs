//! EGL context creation for Android.
//!
//! Mirrors `glx.rs`, but against `libEGL` rather than GLX: the surface is created from the
//! `ANativeWindow` handed out by ramen, and functions are loaded with `eglGetProcAddress`.

use std::{
    ffi::{c_void, CString},
    os::raw::{c_char, c_int, c_uint},
    ptr,
};

use ramen::{connection::Connection, window::Window};

pub type EGLDisplay = *mut c_void;
pub type EGLConfig = *mut c_void;
pub type EGLContext = *mut c_void;
pub type EGLSurface = *mut c_void;
pub type EGLNativeWindow = *mut c_void;
pub type EGLClientBuffer = *mut c_void;
pub type EGLImage = *mut c_void;
pub type EGLSync = *mut c_void;
pub type __eglMustCastToProperFunctionPointerType = *const c_void;

pub const EGL_NO_DISPLAY: EGLDisplay = ptr::null_mut();
pub const EGL_NO_CONTEXT: EGLContext = ptr::null_mut();
pub const EGL_NO_SURFACE: EGLSurface = ptr::null_mut();

pub const EGL_BUFFER_SIZE: c_int = 0x3020;
pub const EGL_RED_SIZE: c_int = 0x3024;
pub const EGL_GREEN_SIZE: c_int = 0x3023;
pub const EGL_BLUE_SIZE: c_int = 0x3022;
pub const EGL_ALPHA_SIZE: c_int = 0x3021;
pub const EGL_DEPTH_SIZE: c_int = 0x3025;
pub const EGL_STENCIL_SIZE: c_int = 0x3026;
pub const EGL_RENDERABLE_TYPE: c_int = 0x3040;
pub const EGL_OPENGL_ES2_BIT: c_int = 0x0004;
pub const EGL_OPENGL_ES3_BIT: c_int = 0x0040;
pub const EGL_CONTEXT_CLIENT_VERSION: c_int = 0x3098;
pub const EGL_SURFACE_TYPE: c_int = 0x3033;
pub const EGL_WINDOW_BIT: c_int = 0x0004;
pub const EGL_NONE: c_int = 0x3038;

extern "C" {
    fn eglGetDisplay(display_id: *mut c_void) -> EGLDisplay;
    fn eglInitialize(display: EGLDisplay, major: *mut c_int, minor: *mut c_int) -> c_uint;
    fn eglChooseConfig(
        display: EGLDisplay,
        attrib_list: *const c_int,
        configs: *mut EGLConfig,
        config_size: c_int,
        num_config: *mut c_int,
    ) -> c_uint;
    fn eglCreateContext(display: EGLDisplay, config: EGLConfig, share: EGLContext, attrib_list: *const c_int)
        -> EGLContext;
    fn eglCreateWindowSurface(
        display: EGLDisplay,
        config: EGLConfig,
        window: EGLNativeWindow,
        attrib_list: *const c_int,
    ) -> EGLSurface;
    fn eglMakeCurrent(display: EGLDisplay, draw: EGLSurface, read: EGLSurface, context: EGLContext) -> c_uint;
    fn eglSwapBuffers(display: EGLDisplay, surface: EGLSurface) -> c_uint;
    fn eglSwapInterval(display: EGLDisplay, interval: c_int) -> c_uint;
    fn eglGetProcAddress(procname: *const c_char) -> __eglMustCastToProperFunctionPointerType;
    fn eglDestroySurface(display: EGLDisplay, surface: EGLSurface) -> c_uint;
    fn eglDestroyContext(display: EGLDisplay, context: EGLContext) -> c_uint;
    fn eglTerminate(display: EGLDisplay) -> c_uint;
}

pub struct PlatformImpl {
    display: EGLDisplay,
    surface: EGLSurface,
    context: EGLContext,
}

impl PlatformImpl {
    /// Requests a GLES 3 context, falling back to GLES 2 if unavailable.
    pub unsafe fn new(connection: &Connection, window: &Window) -> Result<Self, String> {
        let _ = connection;
        let native_window = window.xid();
        if native_window.is_null() {
            return Err("Android no ha entregado una ventana nativa".into())
        }

        let display = eglGetDisplay(EGL_NO_DISPLAY);
        if display == EGL_NO_DISPLAY {
            return Err("eglGetDisplay fallo".into())
        }
        let (mut major, mut minor) = (0, 0);
        if eglInitialize(display, &mut major, &mut minor) == 0 {
            return Err("eglInitialize fallo".into())
        }

        let config_attribs = [
            EGL_RENDERABLE_TYPE,
            EGL_OPENGL_ES3_BIT,
            EGL_SURFACE_TYPE,
            EGL_WINDOW_BIT,
            EGL_RED_SIZE,
            8,
            EGL_GREEN_SIZE,
            8,
            EGL_BLUE_SIZE,
            8,
            EGL_ALPHA_SIZE,
            8,
            EGL_DEPTH_SIZE,
            24,
            EGL_STENCIL_SIZE,
            8,
            EGL_NONE,
        ];
        let mut config: EGLConfig = ptr::null_mut();
        let mut num_configs = 0;
        if eglChooseConfig(display, config_attribs.as_ptr(), &mut config, 1, &mut num_configs) == 0
            || num_configs == 0
        {
            // Fall back to GLES 2 sizing, which is more widely available.
            let config_attribs = [
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES2_BIT,
                EGL_SURFACE_TYPE,
                EGL_WINDOW_BIT,
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_DEPTH_SIZE,
                24,
                EGL_STENCIL_SIZE,
                8,
                EGL_NONE,
            ];
            if eglChooseConfig(display, config_attribs.as_ptr(), &mut config, 1, &mut num_configs) == 0
                || num_configs == 0
            {
                return Err("eglChooseConfig fallo".into())
            }
        }

        let version_attribs = [EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE];
        let context = eglCreateContext(display, config, ptr::null_mut(), version_attribs.as_ptr());
        let context = if context == EGL_NO_CONTEXT {
            let version_attribs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
            eglCreateContext(display, config, ptr::null_mut(), version_attribs.as_ptr())
        } else {
            context
        };
        if context == EGL_NO_CONTEXT {
            return Err("eglCreateContext fallo".into())
        }

        let surface = eglCreateWindowSurface(display, config, native_window, ptr::null());
        if surface == EGL_NO_SURFACE {
            return Err("eglCreateWindowSurface fallo".into())
        }
        if eglMakeCurrent(display, surface, surface, context) == 0 {
            eglDestroySurface(display, surface);
            eglDestroyContext(display, context);
            return Err("eglMakeCurrent fallo".into())
        }

        Ok(Self { display, surface, context })
    }

    pub unsafe fn get_function_loader() -> Result<Box<dyn FnMut(&'static str) -> *const c_void>, String> {
        Ok(Box::new(|name: &'static str| {
            let cname = CString::new(name).unwrap();
            let ptr = eglGetProcAddress(cname.as_ptr());
            if ptr.is_null() {
                ptr::null()
            } else {
                ptr
            }
        }))
    }

    pub unsafe fn clean_function_loader() {}

    pub unsafe fn swap_buffers(&self) {
        eglSwapBuffers(self.display, self.surface);
    }

    pub unsafe fn set_swap_interval(&self, n: u32) -> bool {
        eglSwapInterval(self.display, n as c_int) != 0
    }

    pub unsafe fn get_swap_interval(&self) -> u32 {
        // EGL has no portable query for the swap interval.
        1
    }

    pub unsafe fn wait_vsync(&self) {
        // No portable vsync wait on Android.
    }
}

impl Drop for PlatformImpl {
    fn drop(&mut self) {
        unsafe {
            eglMakeCurrent(self.display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
            eglDestroySurface(self.display, self.surface);
            eglDestroyContext(self.display, self.context);
            eglTerminate(self.display);
        }
    }
}
