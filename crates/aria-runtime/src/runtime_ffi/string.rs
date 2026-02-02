//! String operations for the Aria runtime

use std::ptr;
use std::slice;
use super::memory::aria_alloc;

/// AriaString represents a heap-allocated string in the Aria runtime.
/// Layout: [length: usize][capacity: usize][data: [u8]]
#[repr(C)]
pub struct AriaString {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

/// Create a new Aria string from raw bytes.
///
/// # Safety
/// The `data` pointer must point to valid memory of at least `len` bytes.
/// Returns a pointer to a heap-allocated AriaString, or null on allocation failure.
#[no_mangle]
pub unsafe extern "C" fn aria_string_new(data: *const u8, len: usize) -> *mut AriaString {
    if data.is_null() && len > 0 {
        return ptr::null_mut();
    }

    // Allocate space for the string structure
    let string_ptr = aria_alloc(std::mem::size_of::<AriaString>()) as *mut AriaString;
    if string_ptr.is_null() {
        return ptr::null_mut();
    }

    // Allocate space for the string data
    let capacity = len;
    let data_ptr = if len > 0 {
        let ptr = aria_alloc(capacity);
        if ptr.is_null() {
            return ptr::null_mut();
        }
        // Copy the data
        if !data.is_null() {
            ptr::copy_nonoverlapping(data, ptr, len);
        }
        ptr
    } else {
        ptr::null_mut()
    };

    // Initialize the structure
    (*string_ptr).data = data_ptr;
    (*string_ptr).len = len;
    (*string_ptr).capacity = capacity;

    string_ptr
}

/// Concatenate two Aria strings.
///
/// # Safety
/// Both pointers must be valid AriaString pointers or null.
/// Returns a new AriaString containing a + b.
#[no_mangle]
pub unsafe extern "C" fn aria_string_concat(
    a: *mut AriaString,
    b: *mut AriaString,
) -> *mut AriaString {
    let a_len = if !a.is_null() { (*a).len } else { 0 };
    let b_len = if !b.is_null() { (*b).len } else { 0 };

    let total_len = a_len + b_len;

    // Allocate new string
    let result_ptr = aria_alloc(std::mem::size_of::<AriaString>()) as *mut AriaString;
    if result_ptr.is_null() {
        return ptr::null_mut();
    }

    let data_ptr = if total_len > 0 {
        let ptr = aria_alloc(total_len);
        if ptr.is_null() {
            return ptr::null_mut();
        }

        // Copy first string
        if !a.is_null() && a_len > 0 && !(*a).data.is_null() {
            ptr::copy_nonoverlapping((*a).data, ptr, a_len);
        }

        // Copy second string
        if !b.is_null() && b_len > 0 && !(*b).data.is_null() {
            ptr::copy_nonoverlapping((*b).data, ptr.add(a_len), b_len);
        }

        ptr
    } else {
        ptr::null_mut()
    };

    (*result_ptr).data = data_ptr;
    (*result_ptr).len = total_len;
    (*result_ptr).capacity = total_len;

    result_ptr
}

/// Extract a slice from an Aria string.
///
/// # Safety
/// The string pointer must be valid. Returns a new string containing s[start..end].
#[no_mangle]
pub unsafe extern "C" fn aria_string_slice(
    s: *mut AriaString,
    start: usize,
    end: usize,
) -> *mut AriaString {
    if s.is_null() {
        return ptr::null_mut();
    }

    let len = (*s).len;
    let start = start.min(len);
    let end = end.min(len).max(start);
    let slice_len = end - start;

    if slice_len == 0 {
        return aria_string_new(ptr::null(), 0);
    }

    let data = (*s).data.add(start);
    aria_string_new(data, slice_len)
}

/// Compare two Aria strings for equality.
///
/// # Safety
/// Both pointers must be valid AriaString pointers or null.
#[no_mangle]
pub unsafe extern "C" fn aria_string_eq(a: *mut AriaString, b: *mut AriaString) -> bool {
    // Handle null cases
    if a.is_null() && b.is_null() {
        return true;
    }
    if a.is_null() || b.is_null() {
        return false;
    }

    // Compare lengths
    if (*a).len != (*b).len {
        return false;
    }

    // Compare data
    if (*a).len == 0 {
        return true;
    }

    if (*a).data.is_null() || (*b).data.is_null() {
        return (*a).data == (*b).data;
    }

    let a_slice = slice::from_raw_parts((*a).data, (*a).len);
    let b_slice = slice::from_raw_parts((*b).data, (*b).len);

    a_slice == b_slice
}

/// Get the length of an Aria string.
///
/// # Safety
/// The pointer must be a valid AriaString or null.
#[no_mangle]
pub unsafe extern "C" fn aria_string_len(s: *mut AriaString) -> usize {
    if s.is_null() {
        return 0;
    }
    (*s).len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_new() {
        unsafe {
            let data = b"hello";
            let s = aria_string_new(data.as_ptr(), data.len());
            assert!(!s.is_null());
            assert_eq!((*s).len, 5);
            assert_eq!(aria_string_len(s), 5);
        }
    }

    #[test]
    fn test_string_concat() {
        unsafe {
            let a = aria_string_new(b"hello".as_ptr(), 5);
            let b = aria_string_new(b" world".as_ptr(), 6);
            let c = aria_string_concat(a, b);
            assert!(!c.is_null());
            assert_eq!((*c).len, 11);
        }
    }

    #[test]
    fn test_string_eq() {
        unsafe {
            let a = aria_string_new(b"test".as_ptr(), 4);
            let b = aria_string_new(b"test".as_ptr(), 4);
            let c = aria_string_new(b"other".as_ptr(), 5);

            assert!(aria_string_eq(a, b));
            assert!(!aria_string_eq(a, c));
        }
    }

    #[test]
    fn test_string_slice() {
        unsafe {
            let s = aria_string_new(b"hello world".as_ptr(), 11);
            let slice = aria_string_slice(s, 0, 5);
            assert!(!slice.is_null());
            assert_eq!((*slice).len, 5);
        }
    }
}
