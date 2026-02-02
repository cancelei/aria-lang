//! Array operations for the Aria runtime

use std::ptr;
use super::memory::aria_alloc;

/// AriaArray represents a dynamically-sized array in the Aria runtime.
/// Layout: [data: *mut u8][length: usize][capacity: usize][elem_size: usize]
#[repr(C)]
pub struct AriaArray {
    pub data: *mut u8,
    pub length: usize,
    pub capacity: usize,
    pub elem_size: usize,
}

/// Create a new Aria array with the given element size and initial capacity.
///
/// # Safety
/// Returns a pointer to a heap-allocated AriaArray, or null on allocation failure.
#[no_mangle]
pub unsafe extern "C" fn aria_array_new(elem_size: usize, capacity: usize) -> *mut AriaArray {
    if elem_size == 0 {
        return ptr::null_mut();
    }

    // Allocate the array structure
    let array_ptr = aria_alloc(std::mem::size_of::<AriaArray>()) as *mut AriaArray;
    if array_ptr.is_null() {
        return ptr::null_mut();
    }

    // Allocate data buffer
    let data_ptr = if capacity > 0 {
        let size = elem_size * capacity;
        let ptr = aria_alloc(size);
        if ptr.is_null() {
            return ptr::null_mut();
        }
        ptr
    } else {
        ptr::null_mut()
    };

    (*array_ptr).data = data_ptr;
    (*array_ptr).length = 0;
    (*array_ptr).capacity = capacity;
    (*array_ptr).elem_size = elem_size;

    array_ptr
}

/// Push an element to the end of the array.
///
/// # Safety
/// The array pointer must be valid. The elem pointer must point to valid memory
/// of at least elem_size bytes. The array will grow if needed.
#[no_mangle]
pub unsafe extern "C" fn aria_array_push(arr: *mut AriaArray, elem: *const u8) {
    if arr.is_null() || elem.is_null() {
        return;
    }

    let array = &mut *arr;

    // Check if we need to grow
    if array.length >= array.capacity {
        let new_capacity = if array.capacity == 0 {
            4
        } else {
            array.capacity * 2
        };

        let new_size = new_capacity * array.elem_size;
        let new_data = aria_alloc(new_size);
        if new_data.is_null() {
            super::panic::aria_panic(
                b"Out of memory: array push failed\0".as_ptr(),
                b"Out of memory: array push failed".len()
            );
        }

        // Copy old data if it exists
        if !array.data.is_null() && array.length > 0 {
            let old_size = array.length * array.elem_size;
            ptr::copy_nonoverlapping(array.data, new_data, old_size);
        }

        array.data = new_data;
        array.capacity = new_capacity;
    }

    // Copy element to the end
    let dest = array.data.add(array.length * array.elem_size);
    ptr::copy_nonoverlapping(elem, dest, array.elem_size);
    array.length += 1;
}

/// Get a pointer to the element at the given index.
///
/// # Safety
/// The array pointer must be valid. Returns null if index is out of bounds.
#[no_mangle]
pub unsafe extern "C" fn aria_array_get(arr: *mut AriaArray, index: usize) -> *const u8 {
    if arr.is_null() {
        return ptr::null();
    }

    let array = &*arr;

    if index >= array.length {
        return ptr::null();
    }

    array.data.add(index * array.elem_size) as *const u8
}

/// Get the length of the array.
///
/// # Safety
/// The array pointer must be valid or null.
#[no_mangle]
pub unsafe extern "C" fn aria_array_len(arr: *mut AriaArray) -> usize {
    if arr.is_null() {
        return 0;
    }
    (*arr).length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_new() {
        unsafe {
            let arr = aria_array_new(8, 10);
            assert!(!arr.is_null());
            assert_eq!((*arr).length, 0);
            assert_eq!((*arr).capacity, 10);
            assert_eq!((*arr).elem_size, 8);
        }
    }

    #[test]
    fn test_array_push_get() {
        unsafe {
            let arr = aria_array_new(8, 2);
            assert!(!arr.is_null());

            let value1: i64 = 42;
            let value2: i64 = 100;

            aria_array_push(arr, &value1 as *const i64 as *const u8);
            aria_array_push(arr, &value2 as *const i64 as *const u8);

            assert_eq!(aria_array_len(arr), 2);

            let elem1 = aria_array_get(arr, 0) as *const i64;
            let elem2 = aria_array_get(arr, 1) as *const i64;

            assert_eq!(*elem1, 42);
            assert_eq!(*elem2, 100);
        }
    }

    #[test]
    fn test_array_grow() {
        unsafe {
            let arr = aria_array_new(8, 2);

            // Push 5 elements to force growth
            for i in 0..5 {
                let value: i64 = i;
                aria_array_push(arr, &value as *const i64 as *const u8);
            }

            assert_eq!(aria_array_len(arr), 5);
            assert!((*arr).capacity >= 5);
        }
    }

    #[test]
    fn test_array_get_bounds() {
        unsafe {
            let arr = aria_array_new(8, 10);
            let ptr = aria_array_get(arr, 100);
            assert!(ptr.is_null());
        }
    }
}
