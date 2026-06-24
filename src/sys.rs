#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::{ffi::c_void, os::raw::c_char};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct os_log_s {
    _unused: [u8; 0],
}

pub type os_log_t = *mut os_log_s;
pub type os_log_type_t = u8;
pub type os_signpost_id_t = u64;
pub type os_signpost_type_t = u8;
pub type os_activity_flag_t = u32;

pub const OS_LOG_TYPE_DEFAULT: os_log_type_t = 0;
pub const OS_LOG_TYPE_INFO: os_log_type_t = 1;
pub const OS_LOG_TYPE_DEBUG: os_log_type_t = 2;
pub const OS_LOG_TYPE_ERROR: os_log_type_t = 16;
pub const OS_LOG_TYPE_FAULT: os_log_type_t = 17;

pub const OS_SIGNPOST_ID_NULL: os_signpost_id_t = 0;
pub const OS_SIGNPOST_ID_INVALID: os_signpost_id_t = !0;
pub const OS_SIGNPOST_ID_EXCLUSIVE: os_signpost_id_t = 0xEEEE_B0B5_B2B2_EEEE;

pub const OS_SIGNPOST_EVENT: os_signpost_type_t = 0;
pub const OS_SIGNPOST_INTERVAL_BEGIN: os_signpost_type_t = 1;
pub const OS_SIGNPOST_INTERVAL_END: os_signpost_type_t = 2;

pub const OS_ACTIVITY_FLAG_DEFAULT: os_activity_flag_t = 0;
pub const OS_ACTIVITY_FLAG_DETACHED: os_activity_flag_t = 0x1;
pub const OS_ACTIVITY_FLAG_IF_NONE_PRESENT: os_activity_flag_t = 0x2;

// Provided by the OS.
extern "C" {
    pub static __dso_handle: c_void;
    pub static _os_log_default: os_log_s;

    pub fn os_log_create(subsystem: *const c_char, category: *const c_char) -> os_log_t;
    pub fn os_release(object: *mut c_void);
    pub fn os_log_type_enabled(log: os_log_t, level: os_log_type_t) -> bool;
    pub fn _os_log_impl(
        dso: *mut c_void,
        log: os_log_t,
        log_type: os_log_type_t,
        format: *const c_char,
        buf: *mut u8,
        size: u32,
    );
    pub fn os_signpost_enabled(log: os_log_t) -> bool;
    pub fn os_signpost_id_generate(log: os_log_t) -> os_signpost_id_t;
    pub fn os_signpost_id_make_with_pointer(log: os_log_t, ptr: *const c_void) -> os_signpost_id_t;
    pub fn _os_signpost_emit_with_name_impl(
        dso: *mut c_void,
        log: os_log_t,
        signpost_type: os_signpost_type_t,
        signpost_id: os_signpost_id_t,
        name: *const c_char,
        format: *const c_char,
        buf: *mut u8,
        size: u32,
    );
    pub fn _os_activity_initiate_f(
        dso: *mut c_void,
        description: *const c_char,
        flags: os_activity_flag_t,
        context: *mut c_void,
        function: extern "C" fn(*mut c_void),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_create_and_release() {
        let subsystem = CString::new("com.example.test").unwrap();
        let category = CString::new("category").unwrap();
        let log = unsafe { os_log_create(subsystem.as_ptr(), category.as_ptr()) };
        assert!(!log.is_null());

        unsafe {
            os_release(log as *mut _);
        }
    }
}
