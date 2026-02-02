//! FFI Ownership Annotations
//!
//! This module provides the ownership marker types for FFI boundaries.
//! Based on ARIA-PD-010 Section 3.1 Ownership Annotations.
//!
//! ## Ownership Model
//!
//! Aria uses explicit ownership annotations at FFI boundaries to prevent
//! memory safety issues:
//!
//! - `@owned`: Aria owns the returned memory and must free it
//! - `@borrowed`: Foreign code owns the memory; Aria must not free it
//! - `@transfer`: Ownership transfers to foreign code; Aria must not use after
//! - `@owned(free_with: fn)`: Owned with specific cleanup function
//!
//! These annotations are represented as wrapper types that encode
//! ownership semantics in the type system.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

// ============================================================================
// Ownership Marker Traits
// ============================================================================

/// Marker trait for owned pointers
///
/// Types marked as owned are the responsibility of Aria to free.
pub trait OwnedMarker {}

/// Marker trait for borrowed pointers
///
/// Types marked as borrowed must not be freed by Aria.
pub trait BorrowedMarker {}

/// Marker trait for transferred pointers
///
/// Types marked as transfer have ownership moving to foreign code.
pub trait TransferMarker {}

// ============================================================================
// Owned Wrapper
// ============================================================================

/// Owned pointer wrapper - Aria owns this memory
///
/// When a C function returns an `@owned` pointer, Aria is responsible
/// for freeing the memory. This wrapper encodes that responsibility
/// in the type system.
///
/// ## Example
///
/// ```ignore
/// extern C
///   @owned
///   fn malloc(size: USize) -> CVoidPtr
/// end
///
/// // In Aria, this becomes:
/// // fn malloc(size: USize) -> Owned<CVoidPtr>
/// ```
pub struct Owned<T> {
    ptr: NonNull<T>,
    /// Custom deallocator function, if any
    deallocator: Option<fn(*mut T)>,
}

impl<T> Owned<T> {
    /// Create a new owned pointer
    ///
    /// # Safety
    ///
    /// The pointer must be valid and allocated by compatible allocator.
    pub unsafe fn new(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            deallocator: None,
        })
    }

    /// Create a new owned pointer with a custom deallocator
    ///
    /// This corresponds to `@owned(free_with: fn)` in Aria.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and the deallocator must be appropriate.
    pub unsafe fn with_deallocator(ptr: *mut T, dealloc: fn(*mut T)) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            deallocator: Some(dealloc),
        })
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Get the raw pointer as const
    pub fn as_ptr_const(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Consume and return the raw pointer, transferring ownership out
    ///
    /// After calling this, the caller is responsible for freeing the memory.
    pub fn into_raw(self) -> *mut T {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Check if a custom deallocator is set
    pub fn has_custom_deallocator(&self) -> bool {
        self.deallocator.is_some()
    }
}

impl<T> OwnedMarker for Owned<T> {}

impl<T> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> fmt::Debug for Owned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Owned({:p})", self.ptr)
    }
}

// Note: Drop is intentionally NOT implemented automatically.
// In a real Aria implementation, the drop would call the appropriate
// free function. For now, users must explicitly handle cleanup.

// ============================================================================
// Borrowed Wrapper
// ============================================================================

/// Borrowed pointer wrapper - foreign code owns this memory
///
/// When a C function returns a `@borrowed` pointer, Aria must not
/// free the memory. This wrapper prevents accidental deallocation.
///
/// ## Example
///
/// ```ignore
/// extern C
///   @borrowed
///   fn getenv(name: CString) -> CString
/// end
///
/// // In Aria, this becomes:
/// // fn getenv(name: CString) -> Borrowed<CString>
/// ```
pub struct Borrowed<'a, T> {
    ptr: *const T,
    _lifetime: PhantomData<&'a T>,
}

impl<'a, T> Borrowed<'a, T> {
    /// Create a new borrowed pointer
    ///
    /// # Safety
    ///
    /// The pointer must be valid for the lifetime 'a.
    pub unsafe fn new(ptr: *const T) -> Self {
        Self {
            ptr,
            _lifetime: PhantomData,
        }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    /// Check if the pointer is null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Get a reference to the underlying value
    ///
    /// # Safety
    ///
    /// The pointer must be valid and properly aligned.
    pub unsafe fn as_ref(&self) -> Option<&'a T> {
        if self.ptr.is_null() {
            None
        } else {
            Some(&*self.ptr)
        }
    }
}

impl<'a, T> BorrowedMarker for Borrowed<'a, T> {}

impl<'a, T> Clone for Borrowed<'a, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _lifetime: PhantomData,
        }
    }
}

impl<'a, T> Copy for Borrowed<'a, T> {}

impl<'a, T> fmt::Debug for Borrowed<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Borrowed({:p})", self.ptr)
    }
}

// ============================================================================
// Transfer Wrapper
// ============================================================================

/// Transfer marker - ownership moves to foreign code
///
/// When passing a pointer to C with `@transfer`, ownership moves
/// to the foreign code. Aria must not use the pointer after the call.
///
/// This is a move-only type that prevents use after transfer.
///
/// ## Example
///
/// ```ignore
/// extern C
///   fn free(@transfer ptr: CVoidPtr)
/// end
///
/// // In Aria, after calling free(ptr), ptr cannot be used
/// ```
pub struct Transfer<T> {
    ptr: *mut T,
    /// Flag to track if ownership has been transferred
    transferred: bool,
}

impl<T> Transfer<T> {
    /// Create a new transfer wrapper
    ///
    /// # Safety
    ///
    /// The pointer must be valid.
    pub unsafe fn new(ptr: *mut T) -> Self {
        Self {
            ptr,
            transferred: false,
        }
    }

    /// Get the raw pointer for transfer
    ///
    /// This marks the pointer as transferred. Subsequent calls will panic.
    pub fn transfer(&mut self) -> *mut T {
        if self.transferred {
            panic!("Attempted to use pointer after transfer");
        }
        self.transferred = true;
        self.ptr
    }

    /// Check if the pointer has been transferred
    pub fn is_transferred(&self) -> bool {
        self.transferred
    }

    /// Get the raw pointer without marking as transferred
    ///
    /// Use this carefully - it doesn't prevent use-after-transfer.
    pub fn peek(&self) -> *mut T {
        self.ptr
    }
}

impl<T> TransferMarker for Transfer<T> {}

impl<T> fmt::Debug for Transfer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.transferred {
            write!(f, "Transfer(<transferred>)")
        } else {
            write!(f, "Transfer({:p})", self.ptr)
        }
    }
}

// ============================================================================
// Lifetime Annotations
// ============================================================================

/// Lifetime bound marker for borrowed pointers tied to another object
///
/// This represents `@borrowed(lifetime: obj)` in Aria, where the
/// borrowed pointer's validity is tied to another object's lifetime.
pub struct BorrowedFrom<'owner, T> {
    ptr: *const T,
    _owner: PhantomData<&'owner ()>,
}

impl<'owner, T> BorrowedFrom<'owner, T> {
    /// Create a new borrowed-from pointer
    ///
    /// # Safety
    ///
    /// The pointer must be valid for as long as the owner lives.
    pub unsafe fn new(ptr: *const T) -> Self {
        Self {
            ptr,
            _owner: PhantomData,
        }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    /// Check if null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

impl<'owner, T> BorrowedMarker for BorrowedFrom<'owner, T> {}

impl<'owner, T> Clone for BorrowedFrom<'owner, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _owner: PhantomData,
        }
    }
}

impl<'owner, T> Copy for BorrowedFrom<'owner, T> {}

impl<'owner, T> fmt::Debug for BorrowedFrom<'owner, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BorrowedFrom({:p})", self.ptr)
    }
}

// ============================================================================
// Safety Level Markers
// ============================================================================

/// Safety level for FFI operations
///
/// Based on ARIA-PD-010 Section 2.4 Safety Markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyLevel {
    /// Safe - auto-generated wrappers, bounds checking (default)
    Safe,
    /// Unsafe - direct FFI, no runtime checks
    Unsafe,
    /// Raw - no checks, no ownership tracking (@raw_ffi)
    Raw,
}

impl Default for SafetyLevel {
    fn default() -> Self {
        SafetyLevel::Safe
    }
}

/// Marker for @no_panic annotated functions
///
/// Functions with this marker must not panic across FFI boundaries.
/// They should use `catch_panic` internally.
pub struct NoPanic<F>(pub F);

impl<F> NoPanic<F> {
    /// Create a new no-panic wrapper
    pub fn new(f: F) -> Self {
        NoPanic(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owned_creation() {
        let mut value: i32 = 42;
        let owned = unsafe { Owned::new(&mut value as *mut i32) };
        assert!(owned.is_some());
    }

    #[test]
    fn test_owned_null() {
        let owned: Option<Owned<i32>> = unsafe { Owned::new(std::ptr::null_mut()) };
        assert!(owned.is_none());
    }

    #[test]
    fn test_borrowed_creation() {
        let value: i32 = 42;
        let borrowed = unsafe { Borrowed::new(&value as *const i32) };
        assert!(!borrowed.is_null());
    }

    #[test]
    fn test_borrowed_null() {
        let borrowed: Borrowed<i32> = unsafe { Borrowed::new(std::ptr::null()) };
        assert!(borrowed.is_null());
    }

    #[test]
    fn test_transfer_single_use() {
        let mut value: i32 = 42;
        let mut transfer = unsafe { Transfer::new(&mut value as *mut i32) };
        assert!(!transfer.is_transferred());

        let _ptr = transfer.transfer();
        assert!(transfer.is_transferred());
    }

    #[test]
    #[should_panic(expected = "Attempted to use pointer after transfer")]
    fn test_transfer_double_use_panics() {
        let mut value: i32 = 42;
        let mut transfer = unsafe { Transfer::new(&mut value as *mut i32) };
        let _ptr1 = transfer.transfer();
        let _ptr2 = transfer.transfer(); // Should panic
    }

    #[test]
    fn test_safety_level_default() {
        assert_eq!(SafetyLevel::default(), SafetyLevel::Safe);
    }
}
