//! # Aria Channel
//!
//! Channel-based communication primitives for the Aria programming language.
//!
//! This crate provides type-safe channels with move semantics by default,
//! implementing the design from ARIA-PD-007.
//!
//! ## Core Types
//!
//! - [`Chan`] - Bidirectional channel (wraps both send and receive ends)
//! - [`SendChan`] - Send-only channel endpoint (cloneable for MPSC/MPMC)
//! - [`RecvChan`] - Receive-only channel endpoint (cloneable for SPMC/MPMC)
//!
//! ## Channel Creation
//!
//! - [`bounded`] - Create a bounded channel with specified capacity (recommended)
//! - [`unbounded`] - Create an unbounded channel (use with caution)
//! - [`rendezvous`] - Create a rendezvous (zero-capacity) channel for synchronization
//!
//! ## Example
//!
//! ```rust
//! use aria_channel::{bounded, SendChan, RecvChan};
//!
//! let (tx, rx) = bounded::<i32>(10);
//! tx.send(42).unwrap();
//! assert_eq!(rx.recv().unwrap(), 42);
//! ```

pub mod select;

use std::fmt;
use std::time::Duration;
use thiserror::Error;

// Re-export the select macro
pub use select::*;

// ============================================================================
// Error Types
// ============================================================================

/// Error returned when sending to a disconnected channel.
/// Contains the value that could not be sent, allowing recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendError<T>(pub T);

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "channel disconnected")
    }
}

impl<T: fmt::Debug> std::error::Error for SendError<T> {}

/// Error returned by `try_send` operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrySendError<T> {
    /// The channel is full but not disconnected.
    Full(T),
    /// The channel is disconnected.
    Disconnected(T),
}

impl<T> fmt::Display for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrySendError::Full(_) => write!(f, "channel is full"),
            TrySendError::Disconnected(_) => write!(f, "channel disconnected"),
        }
    }
}

impl<T: fmt::Debug> std::error::Error for TrySendError<T> {}

impl<T> TrySendError<T> {
    /// Extracts the value that could not be sent.
    pub fn into_inner(self) -> T {
        match self {
            TrySendError::Full(v) | TrySendError::Disconnected(v) => v,
        }
    }

    /// Returns true if this error is due to the channel being full.
    pub fn is_full(&self) -> bool {
        matches!(self, TrySendError::Full(_))
    }

    /// Returns true if this error is due to the channel being disconnected.
    pub fn is_disconnected(&self) -> bool {
        matches!(self, TrySendError::Disconnected(_))
    }
}

/// Error returned by `send_timeout` operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendTimeoutError<T> {
    /// The timeout elapsed before the value could be sent.
    Timeout(T),
    /// The channel is disconnected.
    Disconnected(T),
}

impl<T> fmt::Display for SendTimeoutError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SendTimeoutError::Timeout(_) => write!(f, "send operation timed out"),
            SendTimeoutError::Disconnected(_) => write!(f, "channel disconnected"),
        }
    }
}

impl<T: fmt::Debug> std::error::Error for SendTimeoutError<T> {}

impl<T> SendTimeoutError<T> {
    /// Extracts the value that could not be sent.
    pub fn into_inner(self) -> T {
        match self {
            SendTimeoutError::Timeout(v) | SendTimeoutError::Disconnected(v) => v,
        }
    }

    /// Returns true if this error is due to timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(self, SendTimeoutError::Timeout(_))
    }

    /// Returns true if this error is due to the channel being disconnected.
    pub fn is_disconnected(&self) -> bool {
        matches!(self, SendTimeoutError::Disconnected(_))
    }
}

/// Error returned when receiving from a disconnected channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("channel disconnected")]
pub struct RecvError;

/// Error returned by `try_recv` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TryRecvError {
    /// No value is available in the channel (non-blocking).
    #[error("channel is empty")]
    Empty,
    /// The channel is disconnected.
    #[error("channel disconnected")]
    Disconnected,
}

impl TryRecvError {
    /// Returns true if this error is due to the channel being empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, TryRecvError::Empty)
    }

    /// Returns true if this error is due to the channel being disconnected.
    pub fn is_disconnected(&self) -> bool {
        matches!(self, TryRecvError::Disconnected)
    }
}

/// Error returned by `recv_timeout` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RecvTimeoutError {
    /// The timeout elapsed before a value was available.
    #[error("receive operation timed out")]
    Timeout,
    /// The channel is disconnected.
    #[error("channel disconnected")]
    Disconnected,
}

impl RecvTimeoutError {
    /// Returns true if this error is due to timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(self, RecvTimeoutError::Timeout)
    }

    /// Returns true if this error is due to the channel being disconnected.
    pub fn is_disconnected(&self) -> bool {
        matches!(self, RecvTimeoutError::Disconnected)
    }
}

// ============================================================================
// Channel Types
// ============================================================================

/// A send-only channel endpoint.
///
/// `SendChan<T>` can be cloned to create multiple senders for MPSC or MPMC patterns.
/// Values sent through this channel are moved (ownership transferred).
///
/// # Examples
///
/// ```rust
/// use aria_channel::bounded;
///
/// let (tx, rx) = bounded::<String>(10);
///
/// // Clone the sender for multiple producers
/// let tx2 = tx.clone();
///
/// tx.send("hello".to_string()).unwrap();
/// tx2.send("world".to_string()).unwrap();
/// ```
#[derive(Debug)]
pub struct SendChan<T> {
    inner: crossbeam_channel::Sender<T>,
}

impl<T> Clone for SendChan<T> {
    fn clone(&self) -> Self {
        SendChan {
            inner: self.inner.clone(),
        }
    }
}

impl<T> SendChan<T> {
    /// Creates a new `SendChan` wrapping a crossbeam sender.
    fn new(sender: crossbeam_channel::Sender<T>) -> Self {
        SendChan { inner: sender }
    }

    /// Sends a value through the channel, blocking until space is available.
    ///
    /// This transfers ownership of `value` to the receiver.
    ///
    /// # Errors
    ///
    /// Returns `SendError` containing the value if the channel is disconnected.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.inner.send(value).map_err(|e| SendError(e.0))
    }

    /// Attempts to send a value without blocking.
    ///
    /// # Errors
    ///
    /// - `TrySendError::Full` if the channel is full (for bounded channels)
    /// - `TrySendError::Disconnected` if the channel is disconnected
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        self.inner.try_send(value).map_err(|e| match e {
            crossbeam_channel::TrySendError::Full(v) => TrySendError::Full(v),
            crossbeam_channel::TrySendError::Disconnected(v) => TrySendError::Disconnected(v),
        })
    }

    /// Sends a value with a timeout.
    ///
    /// # Errors
    ///
    /// - `SendTimeoutError::Timeout` if the timeout elapsed
    /// - `SendTimeoutError::Disconnected` if the channel is disconnected
    pub fn send_timeout(&self, value: T, timeout: Duration) -> Result<(), SendTimeoutError<T>> {
        self.inner.send_timeout(value, timeout).map_err(|e| match e {
            crossbeam_channel::SendTimeoutError::Timeout(v) => SendTimeoutError::Timeout(v),
            crossbeam_channel::SendTimeoutError::Disconnected(v) => {
                SendTimeoutError::Disconnected(v)
            }
        })
    }

    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the channel is full.
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Returns the number of messages in the channel.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the channel capacity, or `None` for unbounded channels.
    pub fn capacity(&self) -> Option<usize> {
        self.inner.capacity()
    }

    /// Returns a reference to the underlying crossbeam sender.
    ///
    /// This is useful for integration with crossbeam's `select!` macro.
    pub fn as_crossbeam(&self) -> &crossbeam_channel::Sender<T> {
        &self.inner
    }
}

/// A receive-only channel endpoint.
///
/// `RecvChan<T>` can be cloned to create multiple receivers for SPMC or MPMC patterns.
/// Values received from this channel are moved (ownership transferred to the receiver).
///
/// # Examples
///
/// ```rust
/// use aria_channel::bounded;
///
/// let (tx, rx) = bounded::<i32>(10);
/// tx.send(42).unwrap();
///
/// assert_eq!(rx.recv().unwrap(), 42);
/// ```
#[derive(Debug)]
pub struct RecvChan<T> {
    inner: crossbeam_channel::Receiver<T>,
}

impl<T> Clone for RecvChan<T> {
    fn clone(&self) -> Self {
        RecvChan {
            inner: self.inner.clone(),
        }
    }
}

impl<T> RecvChan<T> {
    /// Creates a new `RecvChan` wrapping a crossbeam receiver.
    fn new(receiver: crossbeam_channel::Receiver<T>) -> Self {
        RecvChan { inner: receiver }
    }

    /// Receives a value from the channel, blocking until one is available.
    ///
    /// This transfers ownership of the value from the sender to the caller.
    ///
    /// # Errors
    ///
    /// Returns `RecvError` if the channel is disconnected and empty.
    pub fn recv(&self) -> Result<T, RecvError> {
        self.inner.recv().map_err(|_| RecvError)
    }

    /// Attempts to receive a value without blocking.
    ///
    /// # Errors
    ///
    /// - `TryRecvError::Empty` if no value is available
    /// - `TryRecvError::Disconnected` if the channel is disconnected
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.inner.try_recv().map_err(|e| match e {
            crossbeam_channel::TryRecvError::Empty => TryRecvError::Empty,
            crossbeam_channel::TryRecvError::Disconnected => TryRecvError::Disconnected,
        })
    }

    /// Receives a value with a timeout.
    ///
    /// # Errors
    ///
    /// - `RecvTimeoutError::Timeout` if the timeout elapsed
    /// - `RecvTimeoutError::Disconnected` if the channel is disconnected
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        self.inner.recv_timeout(timeout).map_err(|e| match e {
            crossbeam_channel::RecvTimeoutError::Timeout => RecvTimeoutError::Timeout,
            crossbeam_channel::RecvTimeoutError::Disconnected => RecvTimeoutError::Disconnected,
        })
    }

    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns `true` if the channel is full.
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Returns the number of messages in the channel.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the channel capacity, or `None` for unbounded channels.
    pub fn capacity(&self) -> Option<usize> {
        self.inner.capacity()
    }

    /// Returns an iterator that receives messages until the channel is disconnected.
    pub fn iter(&self) -> ChannelIter<'_, T> {
        ChannelIter { receiver: self }
    }

    /// Returns a reference to the underlying crossbeam receiver.
    ///
    /// This is useful for integration with crossbeam's `select!` macro.
    pub fn as_crossbeam(&self) -> &crossbeam_channel::Receiver<T> {
        &self.inner
    }
}

impl<T> IntoIterator for RecvChan<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { receiver: self }
    }
}

impl<'a, T> IntoIterator for &'a RecvChan<T> {
    type Item = T;
    type IntoIter = ChannelIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over received channel messages.
pub struct ChannelIter<'a, T> {
    receiver: &'a RecvChan<T>,
}

impl<T> Iterator for ChannelIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.recv().ok()
    }
}

/// An owning iterator over received channel messages.
pub struct IntoIter<T> {
    receiver: RecvChan<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.recv().ok()
    }
}

/// A bidirectional channel handle containing both send and receive endpoints.
///
/// `Chan<T>` provides a convenient way to hold both ends of a channel together.
/// It is not cloneable - use `split()` to get separate `SendChan` and `RecvChan`
/// which can be cloned independently.
///
/// # Examples
///
/// ```rust
/// use aria_channel::Chan;
///
/// let chan = Chan::<i32>::bounded(10);
/// chan.send(42).unwrap();
/// assert_eq!(chan.recv().unwrap(), 42);
///
/// // Split into separate endpoints
/// let (tx, rx) = chan.split();
/// ```
#[derive(Debug)]
pub struct Chan<T> {
    sender: SendChan<T>,
    receiver: RecvChan<T>,
}

impl<T> Chan<T> {
    /// Creates a new bounded bidirectional channel with the specified capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is 0. Use `rendezvous()` for zero-capacity channels.
    pub fn bounded(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be > 0; use rendezvous() for zero-capacity channels");
        let (tx, rx) = bounded(capacity);
        Chan {
            sender: tx,
            receiver: rx,
        }
    }

    /// Creates a new unbounded bidirectional channel.
    ///
    /// # Warning
    ///
    /// Unbounded channels can grow without limit and exhaust memory.
    /// Prefer `bounded()` for production code.
    pub fn unbounded() -> Self {
        let (tx, rx) = unbounded();
        Chan {
            sender: tx,
            receiver: rx,
        }
    }

    /// Creates a new rendezvous (zero-capacity) bidirectional channel.
    ///
    /// Send operations block until a receiver is ready, and vice versa.
    pub fn rendezvous() -> Self {
        let (tx, rx) = rendezvous();
        Chan {
            sender: tx,
            receiver: rx,
        }
    }

    /// Splits this channel into separate send and receive endpoints.
    pub fn split(self) -> (SendChan<T>, RecvChan<T>) {
        (self.sender, self.receiver)
    }

    /// Sends a value through the channel.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.sender.send(value)
    }

    /// Attempts to send a value without blocking.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        self.sender.try_send(value)
    }

    /// Sends a value with a timeout.
    pub fn send_timeout(&self, value: T, timeout: Duration) -> Result<(), SendTimeoutError<T>> {
        self.sender.send_timeout(value, timeout)
    }

    /// Receives a value from the channel.
    pub fn recv(&self) -> Result<T, RecvError> {
        self.receiver.recv()
    }

    /// Attempts to receive a value without blocking.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receives a value with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    /// Returns a reference to the send endpoint.
    pub fn sender(&self) -> &SendChan<T> {
        &self.sender
    }

    /// Returns a reference to the receive endpoint.
    pub fn receiver(&self) -> &RecvChan<T> {
        &self.receiver
    }

    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }

    /// Returns `true` if the channel is full.
    pub fn is_full(&self) -> bool {
        self.sender.is_full()
    }

    /// Returns the number of messages in the channel.
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Returns the channel capacity, or `None` for unbounded channels.
    pub fn capacity(&self) -> Option<usize> {
        self.sender.capacity()
    }
}

// ============================================================================
// Channel Constructors
// ============================================================================

/// Creates a bounded channel with the specified capacity.
///
/// Bounded channels provide backpressure - senders will block when the
/// channel is full until space becomes available.
///
/// # Panics
///
/// Panics if `capacity` is 0. Use [`rendezvous`] for zero-capacity channels.
///
/// # Examples
///
/// ```rust
/// use aria_channel::bounded;
///
/// let (tx, rx) = bounded::<i32>(100);
///
/// // Multiple senders
/// let tx2 = tx.clone();
/// std::thread::spawn(move || {
///     tx2.send(1).unwrap();
/// });
///
/// tx.send(2).unwrap();
/// ```
pub fn bounded<T>(capacity: usize) -> (SendChan<T>, RecvChan<T>) {
    assert!(
        capacity > 0,
        "capacity must be > 0; use rendezvous() for zero-capacity channels"
    );
    let (tx, rx) = crossbeam_channel::bounded(capacity);
    (SendChan::new(tx), RecvChan::new(rx))
}

/// Creates an unbounded channel with unlimited capacity.
///
/// # Warning
///
/// Unbounded channels can grow without limit and potentially exhaust memory
/// if producers outpace consumers. Prefer [`bounded`] channels for production
/// code.
///
/// # Examples
///
/// ```rust
/// use aria_channel::unbounded;
///
/// let (tx, rx) = unbounded::<i32>();
///
/// // Never blocks on send
/// for i in 0..1000 {
///     tx.send(i).unwrap();
/// }
/// ```
pub fn unbounded<T>() -> (SendChan<T>, RecvChan<T>) {
    let (tx, rx) = crossbeam_channel::unbounded();
    (SendChan::new(tx), RecvChan::new(rx))
}

/// Creates a rendezvous (zero-capacity) channel for synchronous handoff.
///
/// Both sender and receiver must be ready simultaneously for a message
/// to be transferred. This is useful for synchronization points.
///
/// # Examples
///
/// ```rust
/// use aria_channel::rendezvous;
/// use std::thread;
///
/// let (tx, rx) = rendezvous::<i32>();
///
/// thread::spawn(move || {
///     // Blocks until receiver is ready
///     tx.send(42).unwrap();
/// });
///
/// // Blocks until sender is ready
/// assert_eq!(rx.recv().unwrap(), 42);
/// ```
pub fn rendezvous<T>() -> (SendChan<T>, RecvChan<T>) {
    let (tx, rx) = crossbeam_channel::bounded(0);
    (SendChan::new(tx), RecvChan::new(rx))
}

// ============================================================================
// Convenience constructors matching Aria design patterns
// ============================================================================

/// Creates an MPSC (Multiple Producer, Single Consumer) channel.
///
/// This is the most common channel pattern. The sender can be cloned
/// for multiple producers, but the receiver cannot be cloned.
///
/// Defaults to capacity of 32 if not specified.
///
/// # Examples
///
/// ```rust
/// use aria_channel::mpsc;
///
/// let (tx, rx) = mpsc::<i32>(Some(100));
///
/// let tx2 = tx.clone();
/// std::thread::spawn(move || {
///     tx2.send(1).unwrap();
/// });
///
/// tx.send(2).unwrap();
/// ```
pub fn mpsc<T>(capacity: Option<usize>) -> (SendChan<T>, RecvChan<T>) {
    bounded(capacity.unwrap_or(32))
}

/// Creates an MPMC (Multiple Producer, Multiple Consumer) channel.
///
/// Both sender and receiver can be cloned for work-stealing patterns.
///
/// Defaults to capacity of 32 if not specified.
///
/// # Examples
///
/// ```rust
/// use aria_channel::mpmc;
///
/// let (tx, rx) = mpmc::<i32>(Some(100));
///
/// // Multiple producers
/// let tx2 = tx.clone();
/// // Multiple consumers
/// let rx2 = rx.clone();
/// ```
pub fn mpmc<T>(capacity: Option<usize>) -> (SendChan<T>, RecvChan<T>) {
    bounded(capacity.unwrap_or(32))
}

/// Creates an SPSC (Single Producer, Single Consumer) optimized channel.
///
/// Note: In this implementation, SPSC uses the same underlying mechanism
/// as MPSC/MPMC. True SPSC optimization would require additional implementation.
/// The cloneability is the same, but semantically this is intended for
/// single-producer, single-consumer patterns.
///
/// Defaults to capacity of 64 if not specified.
pub fn spsc<T>(capacity: Option<usize>) -> (SendChan<T>, RecvChan<T>) {
    bounded(capacity.unwrap_or(64))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_bounded_send_recv() {
        let (tx, rx) = bounded::<i32>(10);
        tx.send(42).unwrap();
        assert_eq!(rx.recv().unwrap(), 42);
    }

    #[test]
    fn test_unbounded_send_recv() {
        let (tx, rx) = unbounded::<String>();
        tx.send("hello".to_string()).unwrap();
        tx.send("world".to_string()).unwrap();
        assert_eq!(rx.recv().unwrap(), "hello");
        assert_eq!(rx.recv().unwrap(), "world");
    }

    #[test]
    fn test_rendezvous_sync() {
        let (tx, rx) = rendezvous::<i32>();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            tx.send(42).unwrap();
        });

        // This blocks until the sender is ready
        assert_eq!(rx.recv().unwrap(), 42);
        handle.join().unwrap();
    }

    #[test]
    fn test_try_send_full() {
        let (tx, rx) = bounded::<i32>(1);
        tx.send(1).unwrap();

        match tx.try_send(2) {
            Err(TrySendError::Full(v)) => assert_eq!(v, 2),
            _ => panic!("expected TrySendError::Full"),
        }

        // Drain to allow more sends
        rx.recv().unwrap();
    }

    #[test]
    fn test_try_recv_empty() {
        let (_tx, rx) = bounded::<i32>(10);
        match rx.try_recv() {
            Err(TryRecvError::Empty) => {}
            _ => panic!("expected TryRecvError::Empty"),
        }
    }

    #[test]
    fn test_disconnect_send() {
        let (tx, rx) = bounded::<i32>(10);
        drop(rx);

        match tx.send(42) {
            Err(SendError(v)) => assert_eq!(v, 42),
            _ => panic!("expected SendError"),
        }
    }

    #[test]
    fn test_disconnect_recv() {
        let (tx, rx) = bounded::<i32>(10);
        drop(tx);

        match rx.recv() {
            Err(RecvError) => {}
            _ => panic!("expected RecvError"),
        }
    }

    #[test]
    fn test_clone_sender() {
        let (tx, rx) = bounded::<i32>(10);
        let tx2 = tx.clone();

        tx.send(1).unwrap();
        tx2.send(2).unwrap();

        let mut values: Vec<i32> = vec![rx.recv().unwrap(), rx.recv().unwrap()];
        values.sort();
        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn test_clone_receiver() {
        let (tx, rx) = bounded::<i32>(10);
        let rx2 = rx.clone();

        tx.send(1).unwrap();
        tx.send(2).unwrap();

        // Either receiver can get the values
        let v1 = rx.recv().unwrap();
        let v2 = rx2.recv().unwrap();
        let mut values = vec![v1, v2];
        values.sort();
        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn test_chan_bidirectional() {
        let chan = Chan::<i32>::bounded(10);
        chan.send(42).unwrap();
        assert_eq!(chan.recv().unwrap(), 42);
    }

    #[test]
    fn test_chan_split() {
        let chan = Chan::<String>::bounded(10);
        let (tx, rx) = chan.split();

        tx.send("hello".to_string()).unwrap();
        assert_eq!(rx.recv().unwrap(), "hello");
    }

    #[test]
    fn test_iterator() {
        let (tx, rx) = bounded::<i32>(10);
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        drop(tx);

        let values: Vec<i32> = rx.into_iter().collect();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_capacity_and_len() {
        let (tx, rx) = bounded::<i32>(10);
        assert_eq!(tx.capacity(), Some(10));
        assert_eq!(rx.capacity(), Some(10));
        assert_eq!(tx.len(), 0);
        assert!(tx.is_empty());
        assert!(!tx.is_full());

        tx.send(1).unwrap();
        assert_eq!(tx.len(), 1);
        assert!(!tx.is_empty());
    }

    #[test]
    fn test_send_timeout() {
        let (tx, _rx) = bounded::<i32>(1);
        tx.send(1).unwrap();

        match tx.send_timeout(2, Duration::from_millis(10)) {
            Err(SendTimeoutError::Timeout(v)) => assert_eq!(v, 2),
            _ => panic!("expected timeout"),
        }
    }

    #[test]
    fn test_recv_timeout() {
        let (_tx, rx) = bounded::<i32>(10);

        match rx.recv_timeout(Duration::from_millis(10)) {
            Err(RecvTimeoutError::Timeout) => {}
            _ => panic!("expected timeout"),
        }
    }

    #[test]
    fn test_mpsc_pattern() {
        let (tx, rx) = mpsc::<i32>(Some(100));

        let handles: Vec<_> = (0..4)
            .map(|i| {
                let tx = tx.clone();
                thread::spawn(move || {
                    tx.send(i).unwrap();
                })
            })
            .collect();

        drop(tx);

        for handle in handles {
            handle.join().unwrap();
        }

        let mut values: Vec<i32> = rx.into_iter().collect();
        values.sort();
        assert_eq!(values, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_mpmc_work_stealing() {
        let (tx, rx) = mpmc::<i32>(Some(100));

        // Send values
        for i in 0..10 {
            tx.send(i).unwrap();
        }
        drop(tx);

        // Multiple consumers
        let rx2 = rx.clone();
        let handle = thread::spawn(move || rx2.into_iter().collect::<Vec<_>>());

        let values1: Vec<i32> = rx.into_iter().collect();
        let values2: Vec<i32> = handle.join().unwrap();

        let mut all_values: Vec<i32> = values1.into_iter().chain(values2).collect();
        all_values.sort();
        assert_eq!(all_values, (0..10).collect::<Vec<_>>());
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn test_bounded_zero_capacity_panics() {
        let _ = bounded::<i32>(0);
    }

    #[test]
    fn test_error_types() {
        // Test TrySendError methods
        let err: TrySendError<i32> = TrySendError::Full(42);
        assert!(err.is_full());
        assert!(!err.is_disconnected());
        assert_eq!(err.into_inner(), 42);

        let err: TrySendError<i32> = TrySendError::Disconnected(42);
        assert!(!err.is_full());
        assert!(err.is_disconnected());

        // Test TryRecvError methods
        assert!(TryRecvError::Empty.is_empty());
        assert!(!TryRecvError::Empty.is_disconnected());
        assert!(!TryRecvError::Disconnected.is_empty());
        assert!(TryRecvError::Disconnected.is_disconnected());

        // Test SendTimeoutError methods
        let err: SendTimeoutError<i32> = SendTimeoutError::Timeout(42);
        assert!(err.is_timeout());
        assert!(!err.is_disconnected());

        // Test RecvTimeoutError methods
        assert!(RecvTimeoutError::Timeout.is_timeout());
        assert!(!RecvTimeoutError::Timeout.is_disconnected());
    }
}
