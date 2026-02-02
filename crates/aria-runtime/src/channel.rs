//! Channel primitives for inter-task communication in Aria
//!
//! This module provides typed channels for communication between concurrent tasks.
//! Channels are the primary synchronization primitive in Aria's concurrency model.
//!
//! # Channel Types
//!
//! - **Unbuffered (rendezvous)**: Send blocks until a receiver is ready
//! - **Buffered**: Send blocks only when the buffer is full
//!
//! # Example
//!
//! ```rust
//! use aria_runtime::channel::{Channel, unbuffered, buffered};
//!
//! // Create an unbuffered channel
//! let (tx, rx) = unbuffered::<i32>();
//!
//! // Send in another thread
//! std::thread::spawn(move || {
//!     tx.send(42).unwrap();
//! });
//!
//! // Receive
//! let value = rx.recv().unwrap();
//! assert_eq!(value, 42);
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

use crate::error::RuntimeError;

/// Error type for channel operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    /// The channel has been closed
    Closed,
    /// The channel is full (for try_send)
    Full,
    /// The channel is empty (for try_recv)
    Empty,
    /// All senders have been dropped
    Disconnected,
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelError::Closed => write!(f, "channel closed"),
            ChannelError::Full => write!(f, "channel full"),
            ChannelError::Empty => write!(f, "channel empty"),
            ChannelError::Disconnected => write!(f, "channel disconnected"),
        }
    }
}

impl std::error::Error for ChannelError {}

impl From<ChannelError> for RuntimeError {
    fn from(e: ChannelError) -> Self {
        RuntimeError::Channel(e.to_string())
    }
}

/// Result type for channel operations
pub type ChannelResult<T> = Result<T, ChannelError>;

/// Internal state of a channel
struct ChannelState<T> {
    /// Buffer for messages (empty for unbuffered channels)
    buffer: VecDeque<T>,
    /// Maximum buffer capacity (0 for unbuffered)
    capacity: usize,
    /// Whether the channel is closed
    closed: bool,
    /// Number of active senders
    sender_count: usize,
    /// Number of active receivers
    receiver_count: usize,
    /// Number of senders waiting to send (for unbuffered channels)
    waiting_senders: usize,
    /// Number of receivers waiting to receive
    waiting_receivers: usize,
}

impl<T> ChannelState<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity.max(1)),
            capacity,
            closed: false,
            sender_count: 1,
            receiver_count: 1,
            waiting_senders: 0,
            waiting_receivers: 0,
        }
    }

    fn is_unbuffered(&self) -> bool {
        self.capacity == 0
    }

    fn is_full(&self) -> bool {
        if self.is_unbuffered() {
            // Unbuffered is "full" if no receiver is waiting
            self.waiting_receivers == 0 && !self.buffer.is_empty()
        } else {
            self.buffer.len() >= self.capacity
        }
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

/// Shared channel inner state with synchronization
struct ChannelInner<T> {
    state: Mutex<ChannelState<T>>,
    /// Condition variable for senders waiting for space
    not_full: Condvar,
    /// Condition variable for receivers waiting for data
    not_empty: Condvar,
}

impl<T> ChannelInner<T> {
    fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(ChannelState::new(capacity)),
            not_full: Condvar::new(),
            not_empty: Condvar::new(),
        }
    }
}

/// Sending half of a channel
///
/// Can be cloned to create multiple senders (MPSC pattern).
pub struct Sender<T> {
    inner: Arc<ChannelInner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        {
            let mut state = self.inner.state.lock().unwrap();
            state.sender_count += 1;
        }
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut state = self.inner.state.lock().unwrap();
        state.sender_count -= 1;
        if state.sender_count == 0 {
            // All senders dropped - notify waiting receivers
            self.inner.not_empty.notify_all();
        }
    }
}

impl<T> Sender<T> {
    /// Send a value on the channel, blocking until space is available.
    ///
    /// For unbuffered channels, this blocks until a receiver is ready.
    /// For buffered channels, this blocks only when the buffer is full.
    ///
    /// Returns `Err(ChannelError::Closed)` if the channel has been closed.
    pub fn send(&self, value: T) -> ChannelResult<()> {
        let mut state = self.inner.state.lock().unwrap();

        // Wait for space or until closed
        while state.is_full() && !state.closed {
            state.waiting_senders += 1;
            state = self.inner.not_full.wait(state).unwrap();
            state.waiting_senders -= 1;
        }

        if state.closed {
            return Err(ChannelError::Closed);
        }

        if state.receiver_count == 0 {
            return Err(ChannelError::Disconnected);
        }

        state.buffer.push_back(value);

        // Notify a waiting receiver
        self.inner.not_empty.notify_one();

        Ok(())
    }

    /// Try to send a value without blocking.
    ///
    /// Returns `Err(ChannelError::Full)` if the channel is full.
    /// Returns `Err(ChannelError::Closed)` if the channel is closed.
    pub fn try_send(&self, value: T) -> ChannelResult<()> {
        let mut state = self.inner.state.lock().unwrap();

        if state.closed {
            return Err(ChannelError::Closed);
        }

        if state.receiver_count == 0 {
            return Err(ChannelError::Disconnected);
        }

        if state.is_full() {
            return Err(ChannelError::Full);
        }

        state.buffer.push_back(value);
        self.inner.not_empty.notify_one();

        Ok(())
    }

    /// Close the sending half of the channel.
    ///
    /// After closing, no more values can be sent, but receivers can still
    /// drain any remaining values from the buffer.
    pub fn close(&self) {
        let mut state = self.inner.state.lock().unwrap();
        state.closed = true;
        self.inner.not_empty.notify_all();
        self.inner.not_full.notify_all();
    }

    /// Check if the channel is closed.
    pub fn is_closed(&self) -> bool {
        self.inner.state.lock().unwrap().closed
    }
}

/// Receiving half of a channel
///
/// Can be cloned to create multiple receivers (for broadcast patterns).
pub struct Receiver<T> {
    inner: Arc<ChannelInner<T>>,
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        {
            let mut state = self.inner.state.lock().unwrap();
            state.receiver_count += 1;
        }
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut state = self.inner.state.lock().unwrap();
        state.receiver_count -= 1;
        if state.receiver_count == 0 {
            // All receivers dropped - notify waiting senders
            self.inner.not_full.notify_all();
        }
    }
}

impl<T> Receiver<T> {
    /// Receive a value from the channel, blocking until one is available.
    ///
    /// Returns `Err(ChannelError::Closed)` if the channel is closed and empty.
    /// Returns `Err(ChannelError::Disconnected)` if all senders have been dropped.
    pub fn recv(&self) -> ChannelResult<T> {
        let mut state = self.inner.state.lock().unwrap();

        // Wait for data or until closed/disconnected
        while state.is_empty() && !state.closed && state.sender_count > 0 {
            state.waiting_receivers += 1;
            state = self.inner.not_empty.wait(state).unwrap();
            state.waiting_receivers -= 1;
        }

        // Try to get a value from the buffer
        if let Some(value) = state.buffer.pop_front() {
            // Notify a waiting sender
            self.inner.not_full.notify_one();
            return Ok(value);
        }

        // Buffer is empty
        if state.closed {
            Err(ChannelError::Closed)
        } else {
            // All senders dropped
            Err(ChannelError::Disconnected)
        }
    }

    /// Try to receive a value without blocking.
    ///
    /// Returns `Err(ChannelError::Empty)` if no value is available.
    /// Returns `Err(ChannelError::Closed)` if the channel is closed and empty.
    pub fn try_recv(&self) -> ChannelResult<T> {
        let mut state = self.inner.state.lock().unwrap();

        if let Some(value) = state.buffer.pop_front() {
            self.inner.not_full.notify_one();
            return Ok(value);
        }

        if state.closed {
            Err(ChannelError::Closed)
        } else if state.sender_count == 0 {
            Err(ChannelError::Disconnected)
        } else {
            Err(ChannelError::Empty)
        }
    }

    /// Check if there are values available to receive.
    pub fn is_empty(&self) -> bool {
        self.inner.state.lock().unwrap().is_empty()
    }

    /// Check if the channel is closed.
    pub fn is_closed(&self) -> bool {
        self.inner.state.lock().unwrap().closed
    }

    /// Get the number of values currently in the channel buffer.
    pub fn len(&self) -> usize {
        self.inner.state.lock().unwrap().buffer.len()
    }
}

/// Create an unbuffered (rendezvous) channel.
///
/// Sends block until a receiver is ready to receive the value.
/// This provides synchronous communication between sender and receiver.
///
/// # Example
///
/// ```rust
/// use aria_runtime::channel::unbuffered;
///
/// let (tx, rx) = unbuffered::<i32>();
///
/// std::thread::spawn(move || {
///     tx.send(42).unwrap();
/// });
///
/// assert_eq!(rx.recv().unwrap(), 42);
/// ```
pub fn unbuffered<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(ChannelInner::new(0));
    (
        Sender {
            inner: Arc::clone(&inner),
        },
        Receiver { inner },
    )
}

/// Create a buffered channel with the specified capacity.
///
/// Sends only block when the buffer is full.
///
/// # Example
///
/// ```rust
/// use aria_runtime::channel::buffered;
///
/// let (tx, rx) = buffered::<i32>(10);
///
/// // Can send up to 10 values without blocking
/// for i in 0..10 {
///     tx.send(i).unwrap();
/// }
///
/// // Receive all values
/// for i in 0..10 {
///     assert_eq!(rx.recv().unwrap(), i);
/// }
/// ```
pub fn buffered<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(ChannelInner::new(capacity.max(1)));
    (
        Sender {
            inner: Arc::clone(&inner),
        },
        Receiver { inner },
    )
}

/// A combined channel handle that can both send and receive.
///
/// This is useful for bidirectional communication patterns.
#[derive(Clone)]
pub struct Channel<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> Channel<T> {
    /// Create a new unbuffered channel.
    pub fn new() -> Self {
        let (sender, receiver) = unbuffered();
        Self { sender, receiver }
    }

    /// Create a new buffered channel with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, receiver) = buffered(capacity);
        Self { sender, receiver }
    }

    /// Send a value on the channel.
    pub fn send(&self, value: T) -> ChannelResult<()> {
        self.sender.send(value)
    }

    /// Try to send without blocking.
    pub fn try_send(&self, value: T) -> ChannelResult<()> {
        self.sender.try_send(value)
    }

    /// Receive a value from the channel.
    pub fn recv(&self) -> ChannelResult<T> {
        self.receiver.recv()
    }

    /// Try to receive without blocking.
    pub fn try_recv(&self) -> ChannelResult<T> {
        self.receiver.try_recv()
    }

    /// Close the channel.
    pub fn close(&self) {
        self.sender.close()
    }

    /// Check if the channel is closed.
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    /// Get the sender half.
    pub fn sender(&self) -> Sender<T> {
        self.sender.clone()
    }

    /// Get the receiver half.
    pub fn receiver(&self) -> Receiver<T> {
        self.receiver.clone()
    }

    /// Split the channel into sender and receiver halves.
    pub fn split(self) -> (Sender<T>, Receiver<T>) {
        (self.sender, self.receiver)
    }
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over values received from a channel.
///
/// The iterator yields values until the channel is closed and empty.
pub struct ChannelIter<T> {
    receiver: Receiver<T>,
}

impl<T> Iterator for ChannelIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.recv().ok()
    }
}

impl<T> IntoIterator for Receiver<T> {
    type Item = T;
    type IntoIter = ChannelIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        ChannelIter { receiver: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_unbuffered_send_recv() {
        let (tx, rx) = unbuffered::<i32>();

        thread::spawn(move || {
            tx.send(42).unwrap();
        });

        assert_eq!(rx.recv().unwrap(), 42);
    }

    #[test]
    fn test_buffered_send_recv() {
        let (tx, rx) = buffered::<i32>(10);

        // Can send multiple values without blocking
        for i in 0..5 {
            tx.send(i).unwrap();
        }

        // Receive all values
        for i in 0..5 {
            assert_eq!(rx.recv().unwrap(), i);
        }
    }

    #[test]
    fn test_try_send_try_recv() {
        let (tx, rx) = buffered::<i32>(2);

        // Send two values (buffer capacity)
        assert!(tx.try_send(1).is_ok());
        assert!(tx.try_send(2).is_ok());

        // Buffer full
        assert_eq!(tx.try_send(3), Err(ChannelError::Full));

        // Receive values
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Err(ChannelError::Empty));
    }

    #[test]
    fn test_channel_close() {
        let (tx, rx) = buffered::<i32>(10);

        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.close();

        // Can still receive buffered values
        assert_eq!(rx.recv().unwrap(), 1);
        assert_eq!(rx.recv().unwrap(), 2);

        // Channel closed and empty
        assert_eq!(rx.recv(), Err(ChannelError::Closed));
    }

    #[test]
    fn test_sender_drop_disconnects() {
        let (tx, rx) = buffered::<i32>(10);

        tx.send(1).unwrap();
        drop(tx);

        // Can receive the buffered value
        assert_eq!(rx.recv().unwrap(), 1);

        // All senders dropped
        assert_eq!(rx.recv(), Err(ChannelError::Disconnected));
    }

    #[test]
    fn test_multiple_senders() {
        let (tx1, rx) = buffered::<i32>(10);
        let tx2 = tx1.clone();
        let tx3 = tx1.clone();

        tx1.send(1).unwrap();
        tx2.send(2).unwrap();
        tx3.send(3).unwrap();

        let mut values: Vec<i32> = (0..3).map(|_| rx.recv().unwrap()).collect();
        values.sort();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_channel_struct() {
        let ch = Channel::<i32>::with_capacity(5);

        ch.send(10).unwrap();
        ch.send(20).unwrap();

        assert_eq!(ch.recv().unwrap(), 10);
        assert_eq!(ch.recv().unwrap(), 20);
    }

    #[test]
    fn test_channel_iterator() {
        let (tx, rx) = buffered::<i32>(10);

        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        tx.close();

        let values: Vec<i32> = rx.into_iter().collect();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_concurrent_send_recv() {
        let (tx, rx) = buffered::<i32>(100);
        let tx2 = tx.clone();

        let producer1 = thread::spawn(move || {
            for i in 0..50 {
                tx.send(i).unwrap();
            }
        });

        let producer2 = thread::spawn(move || {
            for i in 50..100 {
                tx2.send(i).unwrap();
            }
        });

        let consumer = thread::spawn(move || {
            let mut values = Vec::new();
            for _ in 0..100 {
                values.push(rx.recv().unwrap());
            }
            values
        });

        producer1.join().unwrap();
        producer2.join().unwrap();
        let mut values = consumer.join().unwrap();
        values.sort();
        assert_eq!(values, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn test_unbuffered_rendezvous() {
        let (tx, rx) = unbuffered::<i32>();

        // Sender blocks until receiver is ready
        let sender = thread::spawn(move || {
            tx.send(42).unwrap();
            tx.send(43).unwrap();
        });

        // Small delay to ensure sender starts first
        thread::sleep(Duration::from_millis(10));

        // Each recv unblocks one send
        assert_eq!(rx.recv().unwrap(), 42);
        assert_eq!(rx.recv().unwrap(), 43);

        sender.join().unwrap();
    }

    #[test]
    fn test_channel_is_closed() {
        let (tx, rx) = buffered::<i32>(10);

        assert!(!tx.is_closed());
        assert!(!rx.is_closed());

        tx.close();

        assert!(tx.is_closed());
        assert!(rx.is_closed());
    }

    #[test]
    fn test_receiver_len() {
        let (tx, rx) = buffered::<i32>(10);

        assert_eq!(rx.len(), 0);

        tx.send(1).unwrap();
        assert_eq!(rx.len(), 1);

        tx.send(2).unwrap();
        assert_eq!(rx.len(), 2);

        rx.recv().unwrap();
        assert_eq!(rx.len(), 1);
    }
}
