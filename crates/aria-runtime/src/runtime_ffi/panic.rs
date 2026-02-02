//! Panic handling for the Aria runtime

use std::slice;
use std::process;

/// Panic with a message and terminate the program.
///
/// # Safety
/// This function never returns. The msg pointer must point to valid UTF-8 data
/// of at least `len` bytes, or can be null (in which case a generic panic message is used).
#[no_mangle]
pub unsafe extern "C" fn aria_panic(msg: *const u8, len: usize) -> ! {
    eprintln!();
    eprintln!("==========================================");
    eprintln!("ARIA RUNTIME PANIC");
    eprintln!("==========================================");
    eprintln!();

    if msg.is_null() || len == 0 {
        eprintln!("Error: Unknown panic");
    } else {
        let bytes = slice::from_raw_parts(msg, len);
        match std::str::from_utf8(bytes) {
            Ok(text) => eprintln!("Error: {}", text),
            Err(_) => eprintln!("Error: <invalid UTF-8 message>"),
        }
    }

    eprintln!();
    eprintln!("The program has encountered a fatal error");
    eprintln!("and cannot continue execution.");
    eprintln!("==========================================");

    process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: We can't really test panic since it exits the process.
    // In a real test suite, we'd use a separate process or mock the exit.

    #[test]
    #[ignore] // Ignore by default since it would exit
    fn test_panic_with_message() {
        unsafe {
            let msg = b"Test panic message";
            aria_panic(msg.as_ptr(), msg.len());
        }
    }
}
