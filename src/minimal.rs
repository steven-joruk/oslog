use crate::level::Level;
use crate::os_log_string::OsLogString;
use crate::sys::{
    os_log_create, os_log_t, os_log_type_enabled, os_release, wrapped_get_default_log,
    wrapped_os_log_with_type,
};
use std::ffi::c_void;

#[derive(Eq, PartialEq, Hash)]
pub struct OsLog {
    pub(crate) inner: os_log_t,
}

unsafe impl Send for OsLog {}
unsafe impl Sync for OsLog {}

impl Drop for OsLog {
    fn drop(&mut self) {
        unsafe {
            if self.inner != wrapped_get_default_log() {
                os_release(self.inner as *mut c_void);
            }
        }
    }
}

impl Default for OsLog {
    fn default() -> Self {
        OsLog::global()
    }
}

impl OsLog {
    #[inline]
    pub fn new<S, C>(subsystem: &S, category: &C) -> Self
    where
        S: OsLogString + ?Sized,
        C: OsLogString + ?Sized,
    {
        let inner = subsystem.with_cstr(|s| {
            category.with_cstr(|c| unsafe { os_log_create(s.as_ptr(), c.as_ptr()) })
        });

        assert!(!inner.is_null(), "Unexpected null value from os_log_create");

        Self { inner }
    }

    #[inline]
    pub fn global() -> Self {
        let inner = unsafe { wrapped_get_default_log() };

        assert!(!inner.is_null(), "Unexpected null value for OS_DEFAULT_LOG");

        Self { inner }
    }

    #[inline]
    pub fn with_level<M: OsLogString + ?Sized>(&self, level: Level, message: &M) {
        message
            .with_cstr(|m| unsafe { wrapped_os_log_with_type(self.inner, level as u8, m.as_ptr()) })
    }

    #[inline]
    pub fn debug<M: OsLogString + ?Sized>(&self, message: &M) {
        self.with_level(Level::Debug, message);
    }

    #[inline]
    pub fn info<M: OsLogString + ?Sized>(&self, message: &M) {
        self.with_level(Level::Info, message);
    }

    #[inline]
    pub fn default<M: OsLogString + ?Sized>(&self, message: &M) {
        self.with_level(Level::Default, message);
    }

    #[inline]
    pub fn error<M: OsLogString + ?Sized>(&self, message: &M) {
        self.with_level(Level::Error, message);
    }

    #[inline]
    pub fn fault<M: OsLogString + ?Sized>(&self, message: &M) {
        self.with_level(Level::Fault, message);
    }

    #[inline]
    pub fn level_is_enabled(&self, level: Level) -> bool {
        unsafe { os_log_type_enabled(self.inner, level as u8) }
    }
}
