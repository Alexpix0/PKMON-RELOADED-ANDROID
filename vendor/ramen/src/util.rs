#![allow(unused_macros)] // macros are unused in win32 at the moment?


use std::{
    cell::UnsafeCell,
    ops::Deref, ptr,
    sync::Once,
};


macro_rules! cstr {
    ($($x:tt)+) => {{ concat!($($x)+, "\0").as_ptr().cast() }};
}

macro_rules! load {
    ($($(#[$outer:meta])* $vis:vis $name:ident($type_name:ident) $($so_name:literal),+ {
        $($(#[$inner:meta])* fn $fn_name:ident($($arg_name:ident:$arg_ty:ty),+$(,)?) $(-> $ret:ty)?;)+
    })+) => {
        pub(self) enum __anyopaque {}
        $(
            $(#[$outer])*
            #[link_section = ".bss"]
            $vis static mut $name: ::std::mem::MaybeUninit<$type_name> = ::std::mem::MaybeUninit::uninit();
            $(#[$outer])*
            #[repr(C)]
            $vis struct $type_name {
                $($(#[$inner])* $fn_name: unsafe extern "system" fn($($arg_ty),+) $(-> $ret)?),+,
            }
            $(#[$outer])*
            impl $type_name {
                $vis unsafe fn load() -> Result<(), crate::error::Error> {
                    static INIT: ::std::sync::Once = ::std::sync::Once::new();
                    static mut LOADED: bool = false;
                    INIT.call_once(|| {
                        let mut handle = ::std::ptr::null_mut();
                        for name in [$(cstr!($so_name)),+] {
                            handle = dlopen(name);
                            if !handle.is_null() { break; }
                        }
                        let _ = dlerror();
                        let mut fp = $name.as_mut_ptr().cast::<*mut c_void>();
                        if handle.is_null() { return; }
                        $($(#[$inner])* {
                            *fp = dlsym(handle, cstr!(stringify!($fn_name)));
                            fp = fp.offset(1);
                        })*
                        _ = fp;
                        LOADED = true;
                    });
                    let err_start = dlerror();
                    if err_start.is_null() {
                        if LOADED {
                            Ok(())
                        } else {
                            Err(crate::error::Error::Unsupported)
                        }
                    } else {
                        let cstr = std::ffi::CStr::from_ptr(err_start);
                        Err(crate::error::Error::Text(String::from_utf8_lossy(cstr.to_bytes())))
                    }
                }
            }
            $($(#[$inner])* #[inline(always)] $vis unsafe fn $fn_name($($arg_name:$arg_ty),+) $(-> $ret)? {
                ((&*$name.as_ptr()).$fn_name)($($arg_name),+)
            })*
        )*
    };
}










// fucking mess below


// lmao


/// Minimal lazily initialized type, similar to the one in `once_cell`.
///
/// Thread safe initialization, immutable-only access.
pub(crate) struct LazyCell<T, F = fn() -> T> {
    // Invariant: Written to at most once on first access.
    init: UnsafeCell<Option<F>>,
    ptr: UnsafeCell<*const T>,

    // Synchronization primitive for initializing `init` and `ptr`.
    once: Once,
}

unsafe impl<T, F> Send for LazyCell<T, F> where T: Send {}
unsafe impl<T, F> Sync for LazyCell<T, F> where T: Sync {}

impl<T, F> LazyCell<T, F> {
    #[allow(dead_code)]
    pub(crate) const fn new(init: F) -> Self {
        Self {
            init: UnsafeCell::new(Some(init)),
            ptr: UnsafeCell::new(ptr::null()),
            once: Once::new(),
        }
    }
}

impl<T, F: FnOnce() -> T> LazyCell<T, F> {
    pub(crate) fn get(&self) -> &T {
        self.once.call_once(|| unsafe {
            if let Some(f) = (&mut *self.init.get()).take() {
                let pointer = Box::into_raw(Box::new(f()));
                ptr::write(self.ptr.get(), pointer);
            }
        });

        // SAFETY: A call to `call_once` initialized the pointer
        unsafe {
            &**self.ptr.get()
        }
    }
}

impl<T, F: FnOnce() -> T> Deref for LazyCell<T, F> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}


use std::collections::TryReserveError;

pub(crate) trait TryPush<T> {
    fn _try_push(&mut self, value: T) -> Result<(), TryReserveError>;
}

impl<T> TryPush<T> for Vec<T> {
    fn _try_push(&mut self, value: T) -> Result<(), TryReserveError> {
        if self.len() == self.capacity() {
            // same allocation as .push()'s impl
            // it usually allocates more, 1 is a hint
            self.try_reserve(1)?;
        }
        self.push(value);
        Ok(())
    }
}

#[allow(dead_code, unused_imports)]
pub(crate) mod sync {
    pub(crate) use self::imp::{cvar_notify_one, cvar_wait, mutex_lock, Condvar, Mutex, MutexGuard};

    #[cfg(not(feature = "parking-lot"))]
    pub(crate) mod imp {
        use std::ptr;
        pub(crate) use std::sync::{Condvar, Mutex, MutexGuard};

        #[inline]
        pub(crate) fn cvar_notify_one(cvar: &Condvar) {
            cvar.notify_one();
        }

        pub(crate) fn cvar_wait<T>(cvar: &Condvar, guard: &mut MutexGuard<T>) {
            // The signature in `std` is quite a pain and CONSUMES the guard
            // We "move it out" for the duration of the wait as a hack
            unsafe {
                let guard_copy = ptr::read(guard);
                let result = cvar.wait(guard_copy).expect("cvar mutex poisoned (this is a bug)");
                ptr::write(guard, result);
            }
        }

        pub(crate) fn mutex_lock<T>(mtx: &Mutex<T>) -> MutexGuard<T> {
            mtx.lock().expect("mutex poisoned (this is a bug)")
        }
    }

    #[cfg(feature = "parking-lot")]
    pub(crate) mod imp {
        pub(crate) use parking_lot::{Condvar, Mutex, MutexGuard};

        #[inline]
        pub(crate) fn cvar_notify_one(cvar: &Condvar) {
            let _ = cvar.notify_one();
        }

        #[inline]
        pub(crate) fn cvar_wait<T>(cvar: &Condvar, guard: &mut MutexGuard<T>) {
            cvar.wait(guard);
        }

        #[inline]
        pub(crate) fn mutex_lock<T>(mtx: &Mutex<T>) -> MutexGuard<T> {
            mtx.lock()
        }
    }
}
