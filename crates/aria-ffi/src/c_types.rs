//! C Type Mappings for Aria FFI
//!
//! This module provides Aria's representation of C types.
//! Based on ARIA-PD-010 Section 1.3 C Type Mapping.
//!
//! ## Type Categories
//!
//! 1. **Platform-specific types**: `CInt`, `CShort`, `CLong`, `CChar`
//! 2. **Fixed-size types**: Use Rust's native `i32`, `u64`, etc.
//! 3. **Floating point**: `CFloat`, `CDouble` (IEEE 754)
//! 4. **Pointer types**: `CVoidPtr`, `CString`
//! 5. **Function pointers**: `CFn<Args, Ret>`

use std::ffi::{CStr, CString as StdCString};
use std::fmt;
use std::marker::PhantomData;
use std::os::raw::{
    c_char, c_double, c_float, c_int, c_long, c_longlong, c_schar, c_short, c_uchar, c_uint,
    c_ulong, c_ulonglong, c_ushort, c_void,
};

// ============================================================================
// Platform-Specific Integer Types
// ============================================================================

/// C `int` type - platform-specific signed integer
pub type CInt = c_int;

/// C `unsigned int` type - platform-specific unsigned integer
pub type CUInt = c_uint;

/// C `short` type - platform-specific signed short
pub type CShort = c_short;

/// C `unsigned short` type - platform-specific unsigned short
pub type CUShort = c_ushort;

/// C `long` type - platform-specific signed long
pub type CLong = c_long;

/// C `unsigned long` type - platform-specific unsigned long
pub type CULong = c_ulong;

/// C `long long` type - platform-specific signed long long
pub type CLongLong = c_longlong;

/// C `unsigned long long` type - platform-specific unsigned long long
pub type CULongLong = c_ulonglong;

/// C `char` type - platform-specific (may be signed or unsigned)
pub type CChar = c_char;

/// C `signed char` type - explicitly signed char
pub type CSChar = c_schar;

/// C `unsigned char` type - explicitly unsigned char
pub type CUChar = c_uchar;

// ============================================================================
// Floating Point Types
// ============================================================================

/// C `float` type - IEEE 754 single precision
pub type CFloat = c_float;

/// C `double` type - IEEE 754 double precision
pub type CDouble = c_double;

// ============================================================================
// Size Types
// ============================================================================

/// C `size_t` type - unsigned size type
pub type CSize = libc::size_t;

/// C `ssize_t` type - signed size type (POSIX)
pub type CSSize = libc::ssize_t;

/// C `ptrdiff_t` type - pointer difference type
pub type CPtrDiff = libc::ptrdiff_t;

/// C `intptr_t` type - integer type that can hold a pointer
pub type CIntPtr = libc::intptr_t;

/// C `uintptr_t` type - unsigned integer type that can hold a pointer
pub type CUIntPtr = libc::uintptr_t;

// ============================================================================
// Void Type
// ============================================================================

/// C `void` type - used in pointer contexts
pub type CVoid = c_void;

/// C `void*` type - type-erased pointer
///
/// In Aria, this is used for opaque pointers and when interfacing
/// with C functions that accept or return `void*`.
pub type CVoidPtr = *mut c_void;

/// C `const void*` type - type-erased const pointer
pub type CVoidPtrConst = *const c_void;

// ============================================================================
// String Types
// ============================================================================

/// Aria's representation of a C string (NUL-terminated `char*`)
///
/// This type wraps C string handling with safety features.
/// It represents both `char*` and `const char*` from C.
#[derive(Clone)]
pub struct AriaString {
    /// The owned string data (if owned)
    owned: Option<StdCString>,
    /// Raw pointer for borrowed strings
    borrowed_ptr: *const c_char,
}

impl AriaString {
    /// Create a new owned AriaString from a Rust string
    pub fn new(s: &str) -> Result<Self, std::ffi::NulError> {
        let cstring = StdCString::new(s)?;
        Ok(Self {
            owned: Some(cstring),
            borrowed_ptr: std::ptr::null(),
        })
    }

    /// Create an AriaString from a borrowed C string pointer
    ///
    /// # Safety
    ///
    /// The pointer must be valid and point to a NUL-terminated string.
    /// The caller must ensure the string outlives this AriaString.
    pub unsafe fn from_ptr_borrowed(ptr: *const c_char) -> Self {
        Self {
            owned: None,
            borrowed_ptr: ptr,
        }
    }

    /// Get the raw pointer to the C string
    pub fn as_ptr(&self) -> *const c_char {
        if let Some(ref owned) = self.owned {
            owned.as_ptr()
        } else {
            self.borrowed_ptr
        }
    }

    /// Convert to a Rust string slice
    ///
    /// # Safety
    ///
    /// For borrowed strings, the pointer must still be valid.
    pub fn to_str(&self) -> Result<&str, std::str::Utf8Error> {
        unsafe {
            let ptr = self.as_ptr();
            if ptr.is_null() {
                Ok("")
            } else {
                CStr::from_ptr(ptr).to_str()
            }
        }
    }

    /// Check if this is a null string
    pub fn is_null(&self) -> bool {
        self.owned.is_none() && self.borrowed_ptr.is_null()
    }
}

impl Default for AriaString {
    fn default() -> Self {
        Self {
            owned: None,
            borrowed_ptr: std::ptr::null(),
        }
    }
}

impl fmt::Debug for AriaString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_str() {
            Ok(s) => write!(f, "AriaString({:?})", s),
            Err(_) => write!(f, "AriaString(<invalid utf8>)"),
        }
    }
}

// ============================================================================
// Function Pointer Type
// ============================================================================

/// C function pointer type
///
/// Represents `T (*)(Args)` in C.
/// This is a marker type used in the Aria type system to represent
/// C function pointers with their signature.
#[repr(transparent)]
pub struct CFn<Args, Ret> {
    ptr: *const c_void,
    _args: PhantomData<Args>,
    _ret: PhantomData<Ret>,
}

impl<Args, Ret> CFn<Args, Ret> {
    /// Create a CFn from a raw function pointer
    ///
    /// # Safety
    ///
    /// The pointer must be a valid function pointer with the correct signature.
    pub unsafe fn from_ptr(ptr: *const c_void) -> Self {
        Self {
            ptr,
            _args: PhantomData,
            _ret: PhantomData,
        }
    }

    /// Get the raw function pointer
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Check if the function pointer is null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

impl<Args, Ret> Clone for CFn<Args, Ret> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _args: PhantomData,
            _ret: PhantomData,
        }
    }
}

impl<Args, Ret> Copy for CFn<Args, Ret> {}

impl<Args, Ret> fmt::Debug for CFn<Args, Ret> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CFn({:p})", self.ptr)
    }
}

// ============================================================================
// Boolean Type
// ============================================================================

/// C boolean type
///
/// In C99 and later, `_Bool` is the boolean type.
/// We use `c_int` for maximum compatibility.
pub type CBool = c_int;

/// C boolean true value
pub const C_TRUE: CBool = 1;

/// C boolean false value
pub const C_FALSE: CBool = 0;

// ============================================================================
// Null Pointer Constant
// ============================================================================

/// Null pointer constant for CVoidPtr
pub const C_NULL: CVoidPtr = std::ptr::null_mut();

/// Null pointer constant for const CVoidPtr
pub const C_NULL_CONST: CVoidPtrConst = std::ptr::null();

// ============================================================================
// Type Traits
// ============================================================================

/// Marker trait for types that are C-compatible
///
/// Types implementing this trait can be safely passed across FFI boundaries.
/// This is automatically implemented for all C primitive types.
pub trait CCompatible: Sized + Copy {}

// Implement CCompatible for primitive types
// Note: C type aliases (CInt, CUInt, etc.) are already covered by these
// implementations since they are type aliases to the underlying Rust types.
impl CCompatible for i8 {}
impl CCompatible for i16 {}
impl CCompatible for i32 {}
impl CCompatible for i64 {}
impl CCompatible for i128 {}
impl CCompatible for isize {}
impl CCompatible for u8 {}
impl CCompatible for u16 {}
impl CCompatible for u32 {}
impl CCompatible for u64 {}
impl CCompatible for u128 {}
impl CCompatible for usize {}
impl CCompatible for f32 {}
impl CCompatible for f64 {}
impl CCompatible for bool {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aria_string_creation() {
        let s = AriaString::new("hello").unwrap();
        assert!(!s.is_null());
        assert_eq!(s.to_str().unwrap(), "hello");
    }

    #[test]
    fn test_aria_string_with_nul() {
        let result = AriaString::new("hello\0world");
        assert!(result.is_err());
    }

    #[test]
    fn test_c_null() {
        assert!(C_NULL.is_null());
        assert!(C_NULL_CONST.is_null());
    }

    #[test]
    fn test_cfn_null() {
        let cfn: CFn<(), ()> = unsafe { CFn::from_ptr(std::ptr::null()) };
        assert!(cfn.is_null());
    }
}
