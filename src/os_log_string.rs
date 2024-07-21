use std::ffi::{CStr, CString};

pub trait OsLogString {
    fn with_cstr<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&CStr) -> R;
}

impl OsLogString for CString {
    fn with_cstr<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&CStr) -> R,
    {
        f(self.as_c_str())
    }
}

impl OsLogString for CStr {
    fn with_cstr<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&CStr) -> R,
    {
        f(self)
    }
}

impl OsLogString for String {
    fn with_cstr<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&CStr) -> R,
    {
        let s = CString::new(self.as_bytes()).unwrap_or_default();

        f(s.as_c_str())
    }
}

impl OsLogString for str {
    fn with_cstr<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&CStr) -> R,
    {
        let s = CString::new(self).unwrap_or_default();

        f(s.as_c_str())
    }
}
