//! Safe Rust access to Apple's unified logging system.
//!
//! ```no_run
//! let log = oslog::OsLog::global();
//! let code = 500i32;
//! oslog::error!(&log, "request failed with code %{public}d", code);
//! ```
//!
//! Format/argument type mismatches are rejected during type checking:
//!
//! ```compile_fail
//! let log = oslog::OsLog::global();
//! oslog::debug!(&log, "%i", "hi");
//! ```
//!
//! Unsupported format specifiers are rejected during macro expansion:
//!
//! ```compile_fail
//! let log = oslog::OsLog::global();
//! oslog::debug!(&log, "%@", "hi");
//! ```
//!
//! Argument count mismatches are rejected during macro expansion:
//!
//! ```compile_fail
//! let log = oslog::OsLog::global();
//! oslog::debug!(&log, "%i %i", 1i32);
//! ```

mod sys;

use crate::sys::*;
use std::ffi::{c_char, c_void, CString};
use std::ptr;

extern crate self as oslog;

pub use oslog_macros::{debug, default, error, fault, info, log};

#[inline]
fn to_cstr(message: &str) -> CString {
    CString::new(message).unwrap_or_else(|_| CString::new(message.replace('\0', "(null)")).unwrap())
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Level {
    Debug = OS_LOG_TYPE_DEBUG,
    Info = OS_LOG_TYPE_INFO,
    Default = OS_LOG_TYPE_DEFAULT,
    Error = OS_LOG_TYPE_ERROR,
    Fault = OS_LOG_TYPE_FAULT,
}

pub struct OsLog {
    inner: os_log_t,
    /// These need to remain allocated or system logging code can use
    /// them after they are freed.
    #[allow(dead_code)]
    subsystem: Option<CString>,
    #[allow(dead_code)]
    category: Option<CString>,
}

unsafe impl Send for OsLog {}
unsafe impl Sync for OsLog {}

impl Drop for OsLog {
    fn drop(&mut self) {
        if !ptr::eq(self.inner, default_log()) {
            unsafe {
                os_release(self.inner as *mut c_void);
            }
        }
    }
}

impl OsLog {
    #[inline]
    pub fn new(subsystem: &str, category: &str) -> Self {
        let subsystem = to_cstr(subsystem);
        let category = to_cstr(category);

        let inner = unsafe { os_log_create(subsystem.as_ptr(), category.as_ptr()) };

        assert!(!inner.is_null(), "Unexpected null value from os_log_create");

        Self {
            inner,
            subsystem: Some(subsystem),
            category: Some(category),
        }
    }

    #[inline]
    pub fn global() -> Self {
        let inner = default_log();

        assert!(!inner.is_null(), "Unexpected null value for OS_LOG_DEFAULT");

        Self {
            inner,
            subsystem: None,
            category: None,
        }
    }

    #[inline]
    pub fn level_is_enabled(&self, level: Level) -> bool {
        unsafe { os_log_type_enabled(self.inner, level as u8) }
    }

    #[doc(hidden)]
    #[inline]
    pub fn as_raw(&self) -> os_log_t {
        self.inner
    }
}

#[inline]
fn default_log() -> os_log_t {
    unsafe { (&_os_log_default as *const os_log_s).cast_mut() }
}

#[doc(hidden)]
pub trait Argument {
    #[doc(hidden)]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, storage: &mut Vec<CString>);
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct FormatSpec {
    kind: ArgumentKind,
    privacy: u8,
    size: u8,
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgumentKind {
    Scalar,
    String,
}

impl FormatSpec {
    #[doc(hidden)]
    #[inline]
    pub const fn new(kind: ArgumentKind, privacy: u8, size: u8) -> Self {
        Self {
            kind,
            privacy,
            size,
        }
    }

    fn header(self) -> u8 {
        let kind = match self.kind {
            ArgumentKind::Scalar => 0,
            ArgumentKind::String => 2,
        };
        (kind << 4) | self.privacy
    }
}

trait RawScalar {
    const SIZE: u8;

    fn append_raw(&self, buffer: &mut Vec<u8>);
}

macro_rules! impl_scalar {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RawScalar for $type {
                const SIZE: u8 = std::mem::size_of::<Self>() as u8;

                #[inline]
                fn append_raw(&self, buffer: &mut Vec<u8>) {
                    buffer.extend_from_slice(&self.to_ne_bytes());
                }
            }

            impl Argument for $type {
                #[inline]
                fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, _: &mut Vec<CString>) {
                    assert_eq!(spec.kind, ArgumentKind::Scalar, "OSLog format specifier expects a non-scalar argument");
                    assert_eq!(spec.size, Self::SIZE, "OSLog scalar argument size does not match its format length modifier");
                    buffer.push(spec.header());
                    buffer.push(spec.size);
                    self.append_raw(buffer);
                }
            }
        )+
    };
}

impl_scalar!(u8, i8, u16, i16, u32, i32, u64, i64, usize, isize, f64);

impl Argument for f32 {
    #[inline]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, storage: &mut Vec<CString>) {
        f64::from(*self).encode(spec, buffer, storage);
    }
}

impl Argument for bool {
    #[inline]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, storage: &mut Vec<CString>) {
        let value = i32::from(*self);
        value.encode(spec, buffer, storage);
    }
}

impl Argument for str {
    #[inline]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, storage: &mut Vec<CString>) {
        assert_eq!(
            spec.kind,
            ArgumentKind::String,
            "OSLog format specifier expects a string argument"
        );

        let value = to_cstr(self);
        let ptr = value.as_ptr() as usize;

        assert_eq!(
            spec.size,
            std::mem::size_of::<usize>() as u8,
            "OSLog string arguments should be pointer-sized"
        );
        storage.push(value);
        buffer.push(spec.header());
        buffer.push(spec.size);
        buffer.extend_from_slice(&ptr.to_ne_bytes());
    }
}

impl Argument for &str {
    #[inline]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, storage: &mut Vec<CString>) {
        (*self).encode(spec, buffer, storage);
    }
}

impl Argument for String {
    #[inline]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, storage: &mut Vec<CString>) {
        self.as_str().encode(spec, buffer, storage);
    }
}

impl Argument for &String {
    #[inline]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, storage: &mut Vec<CString>) {
        self.as_str().encode(spec, buffer, storage);
    }
}

impl<T> Argument for *const T {
    #[inline]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, _: &mut Vec<CString>) {
        assert!(
            spec.kind == ArgumentKind::Scalar,
            "OSLog format specifier expects a pointer argument"
        );

        let ptr = *self as usize;
        assert_eq!(
            spec.size,
            std::mem::size_of::<usize>() as u8,
            "OSLog pointer arguments should be pointer-sized"
        );
        buffer.push(spec.header());
        buffer.push(spec.size);
        buffer.extend_from_slice(&ptr.to_ne_bytes());
    }
}

impl<T> Argument for *mut T {
    #[inline]
    fn encode(&self, spec: FormatSpec, buffer: &mut Vec<u8>, storage: &mut Vec<CString>) {
        (*self as *const T).encode(spec, buffer, storage);
    }
}

macro_rules! marker_trait {
    ($name:ident: $($type:ty),+ $(,)?) => {
        #[doc(hidden)]
        pub trait $name: Argument {}

        $(
            impl $name for $type {}
        )+
    };
}

marker_trait!(Signed1Argument: i8);
marker_trait!(Signed2Argument: i16);
marker_trait!(Signed4Argument: i32);
marker_trait!(Signed8Argument: i64);
marker_trait!(SignedPtrArgument: isize);
marker_trait!(Unsigned1Argument: u8);
marker_trait!(Unsigned2Argument: u16);
marker_trait!(Unsigned4Argument: u32);
marker_trait!(Unsigned8Argument: u64);
marker_trait!(UnsignedPtrArgument: usize);
marker_trait!(FloatArgument: f32, f64);
marker_trait!(CharArgument: i32, u32);
marker_trait!(StringArgument: str, &str, String, &String);

#[doc(hidden)]
pub trait PointerArgument: Argument {}

impl<T> PointerArgument for *const T {}
impl<T> PointerArgument for *mut T {}

macro_rules! checker {
    ($name:ident, $trait:ident) => {
        #[doc(hidden)]
        #[inline]
        pub fn $name<T: $trait>(value: &T) -> &dyn Argument {
            value
        }
    };
}

checker!(__private_arg_signed_1, Signed1Argument);
checker!(__private_arg_signed_2, Signed2Argument);
checker!(__private_arg_signed_4, Signed4Argument);
checker!(__private_arg_signed_8, Signed8Argument);
checker!(__private_arg_signed_ptr, SignedPtrArgument);
checker!(__private_arg_unsigned_1, Unsigned1Argument);
checker!(__private_arg_unsigned_2, Unsigned2Argument);
checker!(__private_arg_unsigned_4, Unsigned4Argument);
checker!(__private_arg_unsigned_8, Unsigned8Argument);
checker!(__private_arg_unsigned_ptr, UnsignedPtrArgument);
checker!(__private_arg_float, FloatArgument);
checker!(__private_arg_char, CharArgument);
checker!(__private_arg_string, StringArgument);
checker!(__private_arg_pointer, PointerArgument);

#[doc(hidden)]
pub fn emit_with_specs(
    log: &OsLog,
    level: Level,
    format: &'static [u8],
    specs: &[FormatSpec],
    args: &[&dyn Argument],
) {
    if !log.level_is_enabled(level) {
        return;
    }

    assert!(
        format.ends_with(b"\0"),
        "OSLog format string must be null-terminated"
    );

    assert_eq!(
        specs.len(),
        args.len(),
        "OSLog format string expects {} arguments, but {} were supplied",
        specs.len(),
        args.len()
    );
    assert!(
        args.len() <= u8::MAX as usize,
        "OSLog supports at most {} arguments",
        u8::MAX
    );

    let (mut buffer, _storage) = build_buffer(specs, args);

    unsafe {
        _os_log_impl(
            (&__dso_handle as *const c_void).cast_mut(),
            log.as_raw(),
            level as u8,
            format.as_ptr().cast::<c_char>(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        );
    }
}

fn build_buffer(specs: &[FormatSpec], args: &[&dyn Argument]) -> (Vec<u8>, Vec<CString>) {
    let mut storage = Vec::new();
    let mut buffer = Vec::with_capacity(2 + args.len() * 10);
    let preamble_position = buffer.len();
    buffer.push(0);
    buffer.push(args.len() as u8);

    for (arg, spec) in args.iter().zip(specs.iter().copied()) {
        arg.encode(spec, &mut buffer, &mut storage);
        if spec.privacy == 1 {
            buffer[preamble_position] |= 0x1;
        }
        if spec.kind != ArgumentKind::Scalar {
            buffer[preamble_position] |= 0x2;
        }
    }

    (buffer, storage)
}

#[doc(hidden)]
pub const fn __private_str_to_array<const N: usize>(value: &str) -> [u8; N] {
    let bytes = value.as_bytes();
    let mut out = [0; N];
    let mut index = 0;

    while index < N {
        out[index] = bytes[index];
        index += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsystem_and_category_interior_nulls_are_sanitized() {
        let log = OsLog::new("com.example.oslog\0test", "category");
        crate::debug!(&log, "Hi");
        let log = OsLog::new("com.example.oslog", "category\0test");
        crate::debug!(&log, "Hi");
    }

    #[test]
    fn macros_emit_all_levels() {
        let log = OsLog::global();
        crate::log!(&log, Level::Debug, "Debug");
        crate::log!(&log, Level::Info, "Info");
        crate::log!(&log, Level::Default, "Default");
        crate::log!(&log, Level::Error, "Error");
        crate::log!(&log, Level::Fault, "Fault");
        crate::debug!(&log, "Debug");
        crate::info!(&log, "Info");
        crate::default!(&log, "Default");
        crate::error!(&log, "Error");
        crate::fault!(&log, "Fault");
    }

    #[test]
    fn macros_accept_supported_argument_types() {
        let log = OsLog::new("com.example.oslog", "testing");
        let string = String::from("String");

        crate::debug!(&log, "%{public}hhd", 1i8);
        crate::debug!(&log, "%{public}hd", 2i16);
        crate::debug!(&log, "%{public}d", 3i32);
        crate::debug!(&log, "%{public}lld", 4i64);
        crate::debug!(&log, "%{public}zd", 5isize);
        crate::debug!(&log, "%{public}hhu", 1u8);
        crate::debug!(&log, "%{public}hu", 2u16);
        crate::debug!(&log, "%{public}u", 3u32);
        crate::debug!(&log, "%{public}llu", 4u64);
        crate::debug!(&log, "%{public}zu", 5usize);
        crate::debug!(&log, "%{public}f", 1.5f64);
        crate::debug!(&log, "%{public}f", 1.5f32);
        crate::debug!(&log, "%{public}c", 65i32);
        crate::debug!(&log, "%{public}s", "str");
        crate::debug!(&log, "%{public}s", string);
        crate::debug!(&log, "%{public}p", &log as *const OsLog);
    }

    #[test]
    fn encodes_public_scalar_buffer() {
        let specs = [FormatSpec::new(ArgumentKind::Scalar, 2, 4)];
        let value = 42i32;
        let (buffer, _storage) = build_buffer(&specs, &[&value]);

        assert_eq!(&buffer[..2], &[0, 1]);
        assert_eq!(&buffer[2..4], &[0x02, 4]);
        assert_eq!(&buffer[4..8], &42i32.to_ne_bytes());
    }

    #[test]
    fn encodes_private_string_buffer() {
        let specs = [FormatSpec::new(
            ArgumentKind::String,
            1,
            std::mem::size_of::<usize>() as u8,
        )];
        let value = "secret";
        let (buffer, storage) = build_buffer(&specs, &[&value]);

        assert_eq!(&buffer[..2], &[0x3, 1]);
        assert_eq!(&buffer[2..4], &[0x21, std::mem::size_of::<usize>() as u8]);
        assert_eq!(storage.len(), 1);
        assert_eq!(storage[0].to_str().unwrap(), "secret");
    }

    #[test]
    fn encodes_public_pointer_buffer() {
        let specs = [FormatSpec::new(
            ArgumentKind::Scalar,
            2,
            std::mem::size_of::<usize>() as u8,
        )];
        let value = 1234u32;
        let pointer = &value as *const u32;
        let (buffer, _storage) = build_buffer(&specs, &[&pointer]);

        assert_eq!(&buffer[..2], &[0, 1]);
        assert_eq!(&buffer[2..4], &[0x02, std::mem::size_of::<usize>() as u8]);
    }
}
