#![allow(bad_style, dead_code)]

pub use std::cell::UnsafeCell;

// C Types
pub use core::ffi::c_void;
pub type c_char = i8;
pub type c_schar = i8;
pub type c_uchar = u8;
pub type c_short = i16;
pub type c_ushort = u16;
pub type c_int = i32;
pub type c_uint = u32;
pub type c_long = i32;
pub type c_ulong = u32;
pub type c_longlong = i64;
pub type c_ulonglong = u64;
pub type wchar_t = u16;

// Windows Types
pub type __int64 = i64;
pub type AUDCLNT_BUFFERFLAGS = u32;
pub type AUDCLNT_SHAREMODE = u32;
pub type BOOL = c_int;
pub type BYTE = c_uchar;
pub type COINIT = u32;
pub type COINITBASE = u32;
pub type DWORD = c_ulong;
pub type EDataFlow = u32;
pub type ERole = u32;
pub type HRESULT = u32;
pub type HANDLE = *mut c_void;
pub type IID = GUID;
pub type LONGLONG = __int64;
pub type LPCGUID = *const GUID;
pub type LPCWSTR = *const WCHAR;
pub type LPSECURITY_ATTRIBUTES = *mut SECURITY_ATTRIBUTES;
pub type LPUNKNOWN = *mut IUnknown;
pub type LPVOID = *mut c_void;
pub type LPWSTR = *mut WCHAR;
pub type REFERENCE_TIME = LONGLONG;
pub type REFCLSID = *const IID;
pub type REFIID = *const IID;
pub type REFPROPERTYKEY = *const PROPERTYKEY;
pub type REFPROPVARIANT = *const PROPVARIANT;
pub type UINT = c_uint;
pub type UINT32 = c_uint;
pub type ULONG = c_ulong;
pub type WCHAR = wchar_t;
pub type WORD = c_ushort;

// Windows Constants
pub const AUDCLNT_BUFFERFLAGS_SILENT: AUDCLNT_BUFFERFLAGS = 2;
pub const AUDCLNT_SHAREMODE_SHARED: AUDCLNT_SHAREMODE = 0;
pub const AUDCLNT_STREAMFLAGS_EVENTCALLBACK: DWORD = 0x00040000;
pub const COINIT_MULTITHREADED: COINIT = COINITBASE_MULTITHREADED;
pub const COINITBASE_MULTITHREADED: COINITBASE = 0;
pub const eConsole: ERole = 0;
pub const eMultimedia: ERole = 1;
pub const eCommunications: ERole = 2;
pub const eRender: EDataFlow = 0;
pub const ERROR_NOT_FOUND: HRESULT = 0x80070490;
pub const AUDCLNT_E_DEVICE_INVALIDATED: HRESULT = 0x88890004;
pub const AUDCLNT_E_UNSUPPORTED_FORMAT: HRESULT = 0x88890008;
pub const FALSE: BOOL = 0;
pub const INFINITE: DWORD = 0xFFFFFFFF;
pub const TRUE: BOOL = 0;
pub const WAVE_FORMAT_IEEE_FLOAT: WORD = 0x0003;
pub const WAVE_FORMAT_PCM: WORD = 0x0001;
pub const WAVE_FORMAT_EXTENSIBLE: WORD = 0xFFFE;

pub const CLSID_MMDeviceEnumerator: GUID = GUID {
    data1: 0xBCDE0395,
    data2: 0xE52F,
    data3: 0x467C,
    data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
};
pub const IID_IMMDeviceEnumerator: GUID = GUID {
    data1: 0xA95664D2,
    data2: 0x9614,
    data3: 0x4F35,
    data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
};
pub const IID_IAudioClient: GUID = GUID {
    data1: 0x1CB9AD4C,
    data2: 0xDBFA,
    data3: 0x4c32,
    data4: [0xB1, 0x78, 0xC2, 0xF5, 0x68, 0xA7, 0x03, 0xB2],
};
pub const IID_IAudioRenderClient: GUID = GUID {
    data1: 0xf294acfc,
    data2: 0x3146,
    data3: 0x4483,
    data4: [0xa7, 0xbf, 0xad, 0xdc, 0xa7, 0xc2, 0x60, 0xe2],
};
pub const KSDATAFORMAT_SUBTYPE_PCM: GUID = GUID {
    data1: 1,
    data2: 0,
    data3: 16,
    data4: [128, 0, 0, 170, 0, 56, 155, 113],
};
pub const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: GUID = GUID {
    data1: 3,
    data2: 0,
    data3: 16,
    data4: [128, 0, 0, 170, 0, 56, 155, 113],
};

// Windows Structs & Unions
#[repr(C)]
#[derive(Debug,Eq, PartialEq)]
pub struct GUID {
    pub data1: c_ulong,
    pub data2: c_ushort,
    pub data3: c_ushort,
    pub data4: [c_uchar; 8],
}
#[repr(C)]
pub struct PROPERTYKEY {
    fmtid: GUID,
    pid: DWORD,
}
#[repr(C, packed)]
pub struct WAVEFORMATEX {
    pub wFormatTag: WORD,
    pub nChannels: WORD,
    pub nSamplesPerSec: DWORD,
    pub nAvgBytesPerSec: DWORD,
    pub nBlockAlign: WORD,
    pub wBitsPerSample: WORD,
    pub cbSize: WORD,
}
#[repr(C, packed)]
pub struct WAVEFORMATEXTENSIBLE {
    pub Format: WAVEFORMATEX,
    pub Samples: WAVEFORMATEXTENSIBLE_Samples,
    pub dwChannelMask: DWORD,
    pub SubFormat: GUID,
}
#[repr(C)]
pub union WAVEFORMATEXTENSIBLE_Samples {
    pub wValidBitsPerSample: WORD,
    pub wSamplesPerBlock: WORD,
    pub wReserved: WORD,
}

// Unused structs we don't define
#[repr(C)]
pub struct IMMNotificationClient { _placeholder: *const c_void }
#[repr(C)]
pub struct PROPVARIANT { _placeholder: *const c_void }
#[repr(C)]
pub struct SECURITY_ATTRIBUTES { _placeholder: *const c_void }

// Windows Functions (Static Linking)
#[link(name = "kernel32")]
extern "system" {
    pub fn CreateEventW(
        lpEventAttributes: LPSECURITY_ATTRIBUTES,
        bManualReset: BOOL,
        bInitialState: BOOL,
        lpName: LPCWSTR
    ) -> HANDLE;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn WaitForSingleObjectEx(hHandle: HANDLE, dwMilliseconds: DWORD, bAlertable: BOOL) -> DWORD;

    pub fn GetCurrentThread() -> HANDLE;
    pub fn SetThreadPriority(hThread: HANDLE, nPriority: c_int) -> BOOL;
}
#[link(name = "ole32")]
extern "system" {
    pub fn CoInitializeEx(pvReserved: LPVOID, dwCoInit: DWORD) -> HRESULT;
    pub fn CoUninitialize();
    pub fn CoCreateInstance(
        rclsid: REFCLSID,
        pUnkOuter: LPUNKNOWN,
        dwClsContext: DWORD,
        riid: REFIID,
        ppv: *mut LPVOID,
    ) -> HRESULT;
    pub fn CoTaskMemFree(pv: LPVOID);
}

// Windows COM Interfaces
#[repr(C)]
pub struct IUnknown(*const IUnknownVtable);
#[repr(C)]
pub struct IUnknownVtable {
    pub QueryInterface: unsafe extern "system" fn(this: *mut IUnknown, riid: REFIID, ppvObject: *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut IUnknown) -> ULONG,
    pub Release: unsafe extern "system" fn(this: *mut IUnknown) -> ULONG,
}
macro_rules! com_interface {
    (
        $(
            $(#[$outer:meta])*
            $v:vis interface $name:ident ( $vt_name:ident ) {
                $(
                    $(#[$fn_outer:meta])*
                    fn $fn_name:ident ( $( $arg_name:ident : $arg_ty:ty ),* $(,)? ) -> $ret:ty;
                )*
            }
        )*
    ) => {
        $(
            $(#[$outer])*
            #[repr(C)]
            $v struct $name(pub *const $vt_name);

            #[repr(C)]
            $v struct $vt_name {
                __iunknown_vtable: IUnknownVtable,
                $( $fn_name: unsafe extern "system" fn( *const $name , $($arg_ty),* ) -> $ret ),*
            }

            impl $name {
                $(
                    $(#[$fn_outer])*
                    #[inline]
                    pub unsafe fn $fn_name ( &self , $( $arg_name : $arg_ty ),* ) -> $ret {
                        ((*self.0).$fn_name)( self , $( $arg_name ),* )
                    }
                )*
            }

            impl ::core::ops::Drop for $name {
                fn drop(&mut self) {
                    unsafe {
                        ((*self.0).__iunknown_vtable.Release)(self as *mut Self as *mut IUnknown);
                    }
                }
            }

            unsafe impl Send for $name {}
        )*
    };
}

com_interface! {
    pub interface IMMDeviceEnumerator(IMMDeviceEnumeratorVtable) {
        fn EnumAudioEndpoints(
            dataFlow: EDataFlow,
            dwStateMask: DWORD,
            ppDevices: *mut *mut IMMDeviceCollection,
        ) -> HRESULT;
        fn GetDefaultAudioEndpoint(dataFlow: EDataFlow, role: ERole, ppEndpoint: *mut *mut IMMDevice) -> HRESULT;
        fn GetDevice(pwstrId: LPCWSTR, ppDevices: *mut *mut IMMDevice) -> HRESULT;
        fn RegisterEndpointNotificationCallback(pClient: *mut IMMNotificationClient) -> HRESULT;
        fn UnregisterEndpointNotificationCallback(pClient: *mut IMMNotificationClient) -> HRESULT;
    }

    pub interface IMMDeviceCollection(IMMDeviceCollectionVtable) {
        fn GetCount(pcDevices: *const UINT) -> HRESULT;
        fn Item(nDevice: UINT, ppDevice: *mut *mut IMMDevice) -> HRESULT;
    }

    pub interface IMMDevice(IMMDeviceVtable) {
        fn Activate(
            iid: REFIID,
            dwClsCtx: DWORD,
            pActivationParams: *mut PROPVARIANT,
            ppInterface: *mut LPVOID,
        ) -> HRESULT;
        fn OpenPropertyStore(stgmAccess: DWORD, ppProperties: *mut *mut IPropertyStore) -> HRESULT;
        fn GetId(ppstrId: *mut LPWSTR) -> HRESULT;
        fn GetState(pdwState: *mut DWORD) -> HRESULT;
    }

    pub interface IPropertyStore(IPropertyStoreVtable) {
        fn GetCount(cProps: *mut DWORD) -> HRESULT;
        fn GetAt(iProp: DWORD, pkey: *mut PROPERTYKEY) -> HRESULT;
        fn GetValue(key: REFPROPERTYKEY, pv: *mut PROPVARIANT) -> HRESULT;
        fn SetValue(key: REFPROPERTYKEY, propvar: REFPROPVARIANT) -> HRESULT;
        fn Commit() -> HRESULT;
    }

    pub interface IAudioClient(IAudioClientVtable) {
        fn Initialize(
            ShareMode: AUDCLNT_SHAREMODE,
            StreamFlags: DWORD,
            hnsBufferDuration: REFERENCE_TIME,
            hnsPeriodicity: REFERENCE_TIME,
            pFormat: *const WAVEFORMATEX,
             AudioSessionGuid: LPCGUID,
        ) -> HRESULT;
        fn GetBufferSize(pNumBufferFrames: *mut UINT32) -> HRESULT;
        fn GetStreamLatency(phnsLatency: *mut REFERENCE_TIME) -> HRESULT;
        fn GetCurrentPadding(pNumPaddingFrames: *mut UINT32) -> HRESULT;
        fn IsFormatSupported(
            ShareMode: AUDCLNT_SHAREMODE,
            pFormat: *const WAVEFORMATEX,
            ppClosestMatch: *mut *mut WAVEFORMATEX,
        ) -> HRESULT;
        fn GetMixFormat(ppDeviceFormat: *mut *mut WAVEFORMATEX) -> HRESULT;
        fn GetDevicePeriod(
            phnsDefaultDevicePeriod: *mut REFERENCE_TIME,
            phnsMinimumDevicePeriod: *mut REFERENCE_TIME,
        ) -> HRESULT;
        fn Start() -> HRESULT;
        fn Stop() -> HRESULT;
        fn Reset() -> HRESULT;
        fn SetEventHandle(eventHandle: HANDLE) -> HRESULT;
        fn GetService(riid: REFIID, ppv: *mut LPVOID) -> HRESULT;
    }

    pub interface IAudioRenderClient(IAudioRenderClientVtable) {
        fn GetBuffer(NumFramesRequested: UINT32, ppData: *mut *mut BYTE) -> HRESULT;
        fn ReleaseBuffer(NumFramesWritten: UINT32, dwFlags: DWORD) -> HRESULT;
    }
}

// Non-bindings - binding helpers

pub struct CoTaskMem<T>(pub *mut T);

unsafe impl<T> Send for CoTaskMem<T> {}

impl<T> core::ops::Drop for CoTaskMem<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(self.0 as LPVOID);
        }
    }
}

pub struct IPtr<T> {
    pub ptr: *mut UnsafeCell<T>,
}

unsafe impl<T> Send for IPtr<T> {}

impl<T> IPtr<T> {
    #[inline]
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr: ptr as _ }
    }

    #[inline]
    pub fn null() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
        }
    }

    #[inline]
    pub fn release(&mut self) {
        if !self.ptr.is_null() {
            unsafe { core::ptr::drop_in_place(self.ptr) }
        }
    }
}

impl<T> core::ops::Deref for IPtr<T> {
    type Target = T;

    #[cfg_attr(not(debug_assertions), inline(always))]
    fn deref(&self) -> &Self::Target {
        #[cfg(debug_assertions)]
        if !self.ptr.is_null() {
            unsafe { &*(self.ptr as *mut T) }
        } else {
            panic!("{} deref when null", core::any::type_name::<Self>());
        }
        #[cfg(not(debug_assertions))]
        unsafe { &*(self.ptr as *mut _) }
    }
}

impl<T> core::ops::DerefMut for IPtr<T> {
    #[cfg_attr(not(debug_assertions), inline(always))]
    fn deref_mut(&mut self) -> &mut <Self as core::ops::Deref>::Target {
        #[cfg(debug_assertions)]
        if !self.ptr.is_null() {
            unsafe { &mut *(self.ptr as *mut T) }
        } else {
            panic!("{} deref-mut when null", core::any::type_name::<Self>());
        }
        #[cfg(not(debug_assertions))]
        unsafe { &mut *(self.ptr as *mut _) }
    }
}
