//! I/O operations for the Aria runtime

use std::io::{self, Write};
use std::slice;
use super::string::AriaString;

/// Print an Aria string to stdout with a newline.
///
/// # Safety
/// The string pointer must be valid or null.
#[no_mangle]
pub unsafe extern "C" fn aria_println(s: *mut AriaString) {
    if s.is_null() {
        println!();
        return;
    }

    let string = &*s;

    if string.len == 0 {
        println!();
        return;
    }

    if string.data.is_null() {
        println!();
        return;
    }

    let bytes = slice::from_raw_parts(string.data, string.len);

    // Try to convert to UTF-8 for printing
    match std::str::from_utf8(bytes) {
        Ok(text) => println!("{}", text),
        Err(_) => {
            // If not valid UTF-8, print as bytes
            io::stdout().write_all(bytes).ok();
            println!();
        }
    }
}

/// Print an Aria string to stdout without a newline.
///
/// # Safety
/// The string pointer must be valid or null.
#[no_mangle]
pub unsafe extern "C" fn aria_print(s: *mut AriaString) {
    if s.is_null() {
        return;
    }

    let string = &*s;

    if string.len == 0 || string.data.is_null() {
        return;
    }

    let bytes = slice::from_raw_parts(string.data, string.len);

    // Try to convert to UTF-8 for printing
    match std::str::from_utf8(bytes) {
        Ok(text) => print!("{}", text),
        Err(_) => {
            // If not valid UTF-8, print as bytes
            io::stdout().write_all(bytes).ok();
        }
    }
    io::stdout().flush().ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::string::aria_string_new;

    #[test]
    fn test_println() {
        unsafe {
            let s = aria_string_new(b"Hello, World!".as_ptr(), 13);
            aria_println(s);
        }
    }

    #[test]
    fn test_print() {
        unsafe {
            let s = aria_string_new(b"Test".as_ptr(), 4);
            aria_print(s);
        }
    }

    #[test]
    fn test_println_null() {
        unsafe {
            aria_println(std::ptr::null_mut());
        }
    }
}
