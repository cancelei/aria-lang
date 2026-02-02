//! # Select Macro for Channel Multiplexing
//!
//! This module provides the `select!` macro for multiplexing over multiple
//! channel operations. It wraps crossbeam-channel's select functionality
//! with an API matching the Aria design from ARIA-PD-007.
//!
//! ## Basic Usage
//!
//! For selecting over two channels, use the convenience functions:
//!
//! ```rust
//! use aria_channel::{bounded, select::{select2_recv, Select2Result}};
//!
//! let (tx1, rx1) = bounded::<i32>(10);
//! let (_tx2, rx2) = bounded::<String>(10);
//!
//! tx1.send(42).unwrap();
//!
//! match select2_recv(&rx1, &rx2) {
//!     Select2Result::First(n) => println!("Got number: {}", n),
//!     Select2Result::Second(s) => println!("Got string: {}", s),
//!     _ => println!("Channel disconnected"),
//! }
//! ```
//!
//! For more complex scenarios, use the `aria_select!` macro with `.as_crossbeam()`:
//!
//! ```rust
//! use aria_channel::{bounded, aria_select};
//!
//! let (tx1, rx1) = bounded::<i32>(10);
//! let (_tx2, rx2) = bounded::<String>(10);
//!
//! tx1.send(42).unwrap();
//!
//! aria_select! {
//!     recv(rx1.as_crossbeam()) -> msg => {
//!         println!("Got number: {:?}", msg);
//!     }
//!     recv(rx2.as_crossbeam()) -> msg => {
//!         println!("Got string: {:?}", msg);
//!     }
//!     default => {
//!         println!("Nothing ready");
//!     }
//! }
//! ```

use crate::{RecvChan, SendChan};
use std::time::Duration;

// Re-export crossbeam_channel's select macro for advanced use cases
pub use crossbeam_channel::select as crossbeam_select;

/// A wrapper around crossbeam's `Select` for building dynamic select operations.
///
/// This is useful when you need to select over a dynamic number of channels.
pub struct Select<'a> {
    inner: crossbeam_channel::Select<'a>,
}

impl<'a> Select<'a> {
    /// Creates a new empty `Select`.
    pub fn new() -> Self {
        Select {
            inner: crossbeam_channel::Select::new(),
        }
    }

    /// Adds a receive operation to the select set.
    ///
    /// Returns the index assigned to this operation.
    pub fn recv<T>(&mut self, receiver: &'a RecvChan<T>) -> usize {
        self.inner.recv(receiver.as_crossbeam())
    }

    /// Adds a send operation to the select set.
    ///
    /// Returns the index assigned to this operation.
    pub fn send<T>(&mut self, sender: &'a SendChan<T>) -> usize {
        self.inner.send(sender.as_crossbeam())
    }

    /// Removes a previously added operation.
    pub fn remove(&mut self, index: usize) {
        self.inner.remove(index);
    }

    /// Blocks until one of the operations becomes ready, then returns its index.
    pub fn ready(&mut self) -> usize {
        self.inner.ready()
    }

    /// Attempts to find a ready operation without blocking.
    ///
    /// Returns `Some(index)` if an operation is ready, `None` otherwise.
    pub fn try_ready(&mut self) -> Option<usize> {
        self.inner.try_ready().ok()
    }

    /// Blocks until one of the operations becomes ready or the timeout elapses.
    ///
    /// Returns `Some(index)` if an operation is ready, `None` on timeout.
    pub fn ready_timeout(&mut self, timeout: Duration) -> Option<usize> {
        self.inner.ready_timeout(timeout).ok()
    }
}

impl Default for Select<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// The result of a select operation.
#[derive(Debug, Clone)]
pub enum SelectResult<T> {
    /// A receive operation completed successfully.
    Recv(T),
    /// A send operation completed successfully.
    Send,
    /// The default branch was taken (no operations ready).
    Default,
    /// The timeout elapsed.
    Timeout,
    /// The operation was disconnected.
    Disconnected,
}

impl<T> SelectResult<T> {
    /// Returns `true` if this is a successful receive.
    pub fn is_recv(&self) -> bool {
        matches!(self, SelectResult::Recv(_))
    }

    /// Returns `true` if this is a successful send.
    pub fn is_send(&self) -> bool {
        matches!(self, SelectResult::Send)
    }

    /// Returns `true` if the default branch was taken.
    pub fn is_default(&self) -> bool {
        matches!(self, SelectResult::Default)
    }

    /// Returns `true` if the operation timed out.
    pub fn is_timeout(&self) -> bool {
        matches!(self, SelectResult::Timeout)
    }

    /// Unwraps the received value, panicking if not a receive result.
    pub fn unwrap_recv(self) -> T {
        match self {
            SelectResult::Recv(v) => v,
            _ => panic!("called unwrap_recv on non-Recv result"),
        }
    }
}

/// Aria's select macro for multiplexing channel operations.
///
/// This macro provides a convenient way to wait on multiple channel operations
/// simultaneously, matching the Aria syntax from ARIA-PD-007.
///
/// # Syntax
///
/// ```text
/// aria_select! {
///     recv(receiver) -> pattern => { body }
///     send(sender, value) -> result => { body }
///     default => { body }
///     default(timeout) => { body }
/// }
/// ```
///
/// # Examples
///
/// ## Blocking Select
///
/// ```rust
/// use aria_channel::{bounded, aria_select};
///
/// let (tx1, rx1) = bounded::<i32>(10);
/// let (_tx2, rx2) = bounded::<&str>(10);
///
/// tx1.send(42).unwrap();
///
/// aria_select! {
///     recv(rx1.as_crossbeam()) -> msg => {
///         match msg {
///             Ok(n) => println!("Got number: {}", n),
///             Err(_) => println!("Channel 1 disconnected"),
///         }
///     }
///     recv(rx2.as_crossbeam()) -> msg => {
///         match msg {
///             Ok(s) => println!("Got string: {}", s),
///             Err(_) => println!("Channel 2 disconnected"),
///         }
///     }
/// }
/// ```
///
/// ## Non-blocking Select with Default
///
/// ```rust
/// use aria_channel::{bounded, aria_select};
///
/// let (_tx, rx) = bounded::<i32>(10);
///
/// aria_select! {
///     recv(rx.as_crossbeam()) -> msg => {
///         println!("Got: {:?}", msg);
///     }
///     default => {
///         println!("Nothing ready");
///     }
/// }
/// ```
///
/// ## Select with Timeout
///
/// ```rust
/// use aria_channel::{bounded, aria_select};
/// use std::time::Duration;
///
/// let (_tx, rx) = bounded::<i32>(10);
///
/// aria_select! {
///     recv(rx.as_crossbeam()) -> msg => {
///         println!("Got: {:?}", msg);
///     }
///     default(Duration::from_millis(100)) => {
///         println!("Timed out");
///     }
/// }
/// ```
///
/// ## Select with Send
///
/// ```rust
/// use aria_channel::{bounded, aria_select};
///
/// let (tx, _rx) = bounded::<i32>(10);
///
/// aria_select! {
///     send(tx.as_crossbeam(), 42) -> result => {
///         match result {
///             Ok(()) => println!("Sent successfully"),
///             Err(_) => println!("Send failed"),
///         }
///     }
///     default => {
///         println!("Channel full or disconnected");
///     }
/// }
/// ```
#[macro_export]
macro_rules! aria_select {
    // Entry point - delegates to crossbeam's select
    ($($tokens:tt)*) => {
        $crate::select::crossbeam_select! {
            $($tokens)*
        }
    };
}

/// Helper function to perform a select over two receive channels.
///
/// This is a convenience function for the common case of selecting
/// between two channels.
pub fn select2_recv<T1, T2>(
    rx1: &RecvChan<T1>,
    rx2: &RecvChan<T2>,
) -> Select2Result<T1, T2> {
    crossbeam_channel::select! {
        recv(rx1.as_crossbeam()) -> msg => {
            match msg {
                Ok(v) => Select2Result::First(v),
                Err(_) => Select2Result::FirstDisconnected,
            }
        }
        recv(rx2.as_crossbeam()) -> msg => {
            match msg {
                Ok(v) => Select2Result::Second(v),
                Err(_) => Select2Result::SecondDisconnected,
            }
        }
    }
}

/// Result of selecting over two channels.
#[derive(Debug, Clone)]
pub enum Select2Result<T1, T2> {
    /// First channel received a value.
    First(T1),
    /// Second channel received a value.
    Second(T2),
    /// First channel is disconnected.
    FirstDisconnected,
    /// Second channel is disconnected.
    SecondDisconnected,
}

impl<T1, T2> Select2Result<T1, T2> {
    /// Returns `true` if the first channel received.
    pub fn is_first(&self) -> bool {
        matches!(self, Select2Result::First(_))
    }

    /// Returns `true` if the second channel received.
    pub fn is_second(&self) -> bool {
        matches!(self, Select2Result::Second(_))
    }
}

/// Helper function to perform a non-blocking select over two receive channels.
pub fn try_select2_recv<T1, T2>(
    rx1: &RecvChan<T1>,
    rx2: &RecvChan<T2>,
) -> Option<Select2Result<T1, T2>> {
    crossbeam_channel::select! {
        recv(rx1.as_crossbeam()) -> msg => {
            Some(match msg {
                Ok(v) => Select2Result::First(v),
                Err(_) => Select2Result::FirstDisconnected,
            })
        }
        recv(rx2.as_crossbeam()) -> msg => {
            Some(match msg {
                Ok(v) => Select2Result::Second(v),
                Err(_) => Select2Result::SecondDisconnected,
            })
        }
        default => None
    }
}

/// Helper function to perform a select with timeout over two receive channels.
pub fn select2_recv_timeout<T1, T2>(
    rx1: &RecvChan<T1>,
    rx2: &RecvChan<T2>,
    timeout: Duration,
) -> Option<Select2Result<T1, T2>> {
    crossbeam_channel::select! {
        recv(rx1.as_crossbeam()) -> msg => {
            Some(match msg {
                Ok(v) => Select2Result::First(v),
                Err(_) => Select2Result::FirstDisconnected,
            })
        }
        recv(rx2.as_crossbeam()) -> msg => {
            Some(match msg {
                Ok(v) => Select2Result::Second(v),
                Err(_) => Select2Result::SecondDisconnected,
            })
        }
        default(timeout) => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bounded, unbounded};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_select_recv() {
        let (tx1, rx1) = bounded::<i32>(10);
        let (_tx2, rx2) = bounded::<String>(10);

        tx1.send(42).unwrap();

        aria_select! {
            recv(rx1.as_crossbeam()) -> msg => {
                assert_eq!(msg.unwrap(), 42);
            }
            recv(rx2.as_crossbeam()) -> _msg => {
                panic!("Should not receive from rx2");
            }
        }
    }

    #[test]
    fn test_select_default() {
        let (_tx, rx) = bounded::<i32>(10);

        let took_default;

        aria_select! {
            recv(rx.as_crossbeam()) -> _msg => {
                panic!("Should not receive");
            }
            default => {
                took_default = true;
            }
        }

        assert!(took_default);
    }

    #[test]
    fn test_select_timeout() {
        let (_tx, rx) = bounded::<i32>(10);

        let start = std::time::Instant::now();

        aria_select! {
            recv(rx.as_crossbeam()) -> _msg => {
                panic!("Should not receive");
            }
            default(Duration::from_millis(50)) => {
                // Timed out as expected
            }
        }

        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[test]
    fn test_select_send() {
        let (tx, rx) = bounded::<i32>(1);

        // Fill the channel
        tx.send(1).unwrap();

        let mut sent;

        aria_select! {
            send(tx.as_crossbeam(), 2) -> _result => {
                panic!("Should not be able to send");
            }
            default => {
                sent = false;
            }
        }

        assert!(!sent);

        // Drain and try again
        rx.recv().unwrap();

        aria_select! {
            send(tx.as_crossbeam(), 2) -> result => {
                assert!(result.is_ok());
                sent = true;
            }
            default => {
                panic!("Should be able to send");
            }
        }

        assert!(sent);
    }

    #[test]
    fn test_select2_recv() {
        let (tx1, rx1) = bounded::<i32>(10);
        let (tx2, rx2) = bounded::<String>(10);

        tx1.send(42).unwrap();

        match select2_recv(&rx1, &rx2) {
            Select2Result::First(v) => assert_eq!(v, 42),
            _ => panic!("Expected First"),
        }

        tx2.send("hello".to_string()).unwrap();

        match select2_recv(&rx1, &rx2) {
            Select2Result::Second(v) => assert_eq!(v, "hello"),
            _ => panic!("Expected Second"),
        }
    }

    #[test]
    fn test_try_select2_recv() {
        let (_tx1, rx1) = bounded::<i32>(10);
        let (_tx2, rx2) = bounded::<String>(10);

        // Both empty
        assert!(try_select2_recv(&rx1, &rx2).is_none());
    }

    #[test]
    fn test_select2_recv_timeout() {
        let (_tx1, rx1) = bounded::<i32>(10);
        let (_tx2, rx2) = bounded::<String>(10);

        let start = std::time::Instant::now();
        let result = select2_recv_timeout(&rx1, &rx2, Duration::from_millis(50));

        assert!(result.is_none());
        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[test]
    fn test_select_with_multiple_ready() {
        let (tx1, rx1) = bounded::<i32>(10);
        let (tx2, rx2) = bounded::<i32>(10);

        tx1.send(1).unwrap();
        tx2.send(2).unwrap();

        // Both are ready, either could be selected
        let result = select2_recv(&rx1, &rx2);
        assert!(result.is_first() || result.is_second());
    }

    #[test]
    fn test_select_builder() {
        let (tx1, rx1) = bounded::<i32>(10);
        let (_tx2, rx2) = bounded::<i32>(10);

        tx1.send(42).unwrap();

        let mut sel = Select::new();
        let idx1 = sel.recv(&rx1);
        let _idx2 = sel.recv(&rx2);

        let ready_idx = sel.ready();
        assert_eq!(ready_idx, idx1);

        // Actually receive the value
        assert_eq!(rx1.recv().unwrap(), 42);
    }

    #[test]
    fn test_select_builder_try_ready() {
        let (tx, rx) = bounded::<i32>(10);

        let mut sel = Select::new();
        sel.recv(&rx);

        // Nothing ready
        assert!(sel.try_ready().is_none());

        tx.send(42).unwrap();

        // Now something is ready
        assert!(sel.try_ready().is_some());
    }

    #[test]
    fn test_select_concurrent() {
        let (tx, rx) = unbounded::<i32>();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            tx.send(42).unwrap();
        });

        aria_select! {
            recv(rx.as_crossbeam()) -> msg => {
                assert_eq!(msg.unwrap(), 42);
            }
        }

        handle.join().unwrap();
    }

    #[test]
    fn test_select_result_methods() {
        let recv_result: SelectResult<i32> = SelectResult::Recv(42);
        assert!(recv_result.is_recv());
        assert!(!recv_result.is_send());
        assert!(!recv_result.is_default());
        assert!(!recv_result.is_timeout());
        assert_eq!(recv_result.unwrap_recv(), 42);

        let send_result: SelectResult<i32> = SelectResult::Send;
        assert!(send_result.is_send());

        let default_result: SelectResult<i32> = SelectResult::Default;
        assert!(default_result.is_default());

        let timeout_result: SelectResult<i32> = SelectResult::Timeout;
        assert!(timeout_result.is_timeout());
    }

    #[test]
    fn test_select_disconnected() {
        let (tx, rx) = bounded::<i32>(10);
        drop(tx);

        match select2_recv(&rx, &rx) {
            Select2Result::FirstDisconnected | Select2Result::SecondDisconnected => {}
            _ => panic!("Expected disconnected"),
        }
    }
}
