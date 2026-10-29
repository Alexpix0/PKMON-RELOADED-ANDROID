//! Low-overhead ATrace telemetry for profiling engine hotpaths.

#[cfg(target_os = "android")]
#[link(name = "android")]
extern "C" {
    fn ATrace_beginSection(sectionName: *const std::os::raw::c_char);
    fn ATrace_endSection();
}

pub struct ProfileScope;

impl ProfileScope {
    #[inline(always)]
    pub fn new(name: &'static [u8]) -> Self {
        #[cfg(target_os = "android")]
        unsafe {
            ATrace_beginSection(name.as_ptr() as *const _);
        }
        let _ = name;
        ProfileScope
    }
}

impl Drop for ProfileScope {
    #[inline(always)]
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        unsafe {
            ATrace_endSection();
        }
    }
}
