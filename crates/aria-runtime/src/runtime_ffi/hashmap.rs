//! HashMap operations for the Aria runtime

use std::collections::HashMap;
use std::ptr;
use std::slice;
use super::string::AriaString;

/// AriaHashMap is a simple hash map that maps strings to i64 values.
/// In production, this would be a more sophisticated data structure,
/// but for the minimal runtime, we use Rust's built-in HashMap.
#[repr(C)]
pub struct AriaHashMap {
    // We hide the actual HashMap behind a pointer to avoid exposing the
    // generic type in the C ABI
    inner: *mut HashMap<Vec<u8>, i64>,
}

/// Create a new empty hash map.
///
/// # Safety
/// Returns a pointer to a heap-allocated AriaHashMap, or null on allocation failure.
#[no_mangle]
pub unsafe extern "C" fn aria_hashmap_new() -> *mut AriaHashMap {
    let map = Box::new(HashMap::new());
    let inner = Box::into_raw(map);

    let hashmap_ptr = super::memory::aria_alloc(std::mem::size_of::<AriaHashMap>()) as *mut AriaHashMap;
    if hashmap_ptr.is_null() {
        // Clean up the HashMap we created
        drop(Box::from_raw(inner));
        return ptr::null_mut();
    }

    (*hashmap_ptr).inner = inner;
    hashmap_ptr
}

/// Insert a key-value pair into the hash map.
///
/// # Safety
/// The map and key pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn aria_hashmap_insert(
    map: *mut AriaHashMap,
    key: *mut AriaString,
    value: i64,
) {
    if map.is_null() || key.is_null() {
        return;
    }

    let hashmap = &mut *(*map).inner;
    let key_str = &*key;

    if key_str.data.is_null() && key_str.len > 0 {
        return;
    }

    let key_bytes = if key_str.len > 0 {
        slice::from_raw_parts(key_str.data, key_str.len).to_vec()
    } else {
        Vec::new()
    };

    hashmap.insert(key_bytes, value);
}

/// Get a value from the hash map by key.
///
/// # Safety
/// The map and key pointers must be valid.
/// Returns the value if found, or 0 if not found.
#[no_mangle]
pub unsafe extern "C" fn aria_hashmap_get(
    map: *mut AriaHashMap,
    key: *mut AriaString,
) -> i64 {
    if map.is_null() || key.is_null() {
        return 0;
    }

    let hashmap = &*(*map).inner;
    let key_str = &*key;

    if key_str.data.is_null() && key_str.len > 0 {
        return 0;
    }

    let key_bytes = if key_str.len > 0 {
        slice::from_raw_parts(key_str.data, key_str.len)
    } else {
        &[]
    };

    hashmap.get(key_bytes).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::string::aria_string_new;

    #[test]
    fn test_hashmap_new() {
        unsafe {
            let map = aria_hashmap_new();
            assert!(!map.is_null());
        }
    }

    #[test]
    fn test_hashmap_insert_get() {
        unsafe {
            let map = aria_hashmap_new();
            let key = aria_string_new(b"test_key".as_ptr(), 8);

            aria_hashmap_insert(map, key, 42);
            let value = aria_hashmap_get(map, key);

            assert_eq!(value, 42);
        }
    }

    #[test]
    fn test_hashmap_get_missing() {
        unsafe {
            let map = aria_hashmap_new();
            let key = aria_string_new(b"missing".as_ptr(), 7);

            let value = aria_hashmap_get(map, key);
            assert_eq!(value, 0);
        }
    }
}
