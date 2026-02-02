//! Pointer Types for Aria FFI
//!
//! This module provides the pointer types used in FFI:
//! - `CPtr<T>`: Nullable pointer to T (`T*` in C)
//! - `CArray<T, N>`: Fixed-size array (`T[N]` in C)
//!
//! Based on ARIA-PD-010 Section 1.3 C Type Mapping.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::c_types::CCompatible;

// ============================================================================
// CPtr - C Pointer Type
// ============================================================================

/// C pointer type - nullable pointer to T
///
/// Represents `T*` in C. This is nullable, unlike Rust references.
///
/// ## Qualifiers
///
/// - `CPtr<T>`: Mutable pointer (`T*`)
/// - Use `CPtr<T>::as_const_ptr()` for const pointer (`const T*`)
/// - Use `CPtr<T>::restrict()` for restrict-qualified pointer (`T* restrict`)
///
/// ## Example
///
/// ```ignore
/// // C: int* ptr;
/// // Aria: ptr: CPtr[Int]
/// ```
#[repr(transparent)]
pub struct CPtr<T> {
    ptr: *mut T,
}

impl<T> CPtr<T> {
    /// Create a new CPtr from a raw mutable pointer
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }

    /// Create a null CPtr
    pub fn null() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }

    /// Check if the pointer is null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Get the raw mutable pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    /// Get the raw const pointer
    pub fn as_const_ptr(&self) -> *const T {
        self.ptr
    }

    /// Get a reference to the pointed value
    ///
    /// # Safety
    ///
    /// The pointer must be valid, non-null, and properly aligned.
    pub unsafe fn as_ref(&self) -> Option<&T> {
        self.ptr.as_ref()
    }

    /// Get a mutable reference to the pointed value
    ///
    /// # Safety
    ///
    /// The pointer must be valid, non-null, and properly aligned.
    /// No other references to the same memory may exist.
    pub unsafe fn as_mut(&mut self) -> Option<&mut T> {
        self.ptr.as_mut()
    }

    /// Offset the pointer by a number of elements
    ///
    /// # Safety
    ///
    /// The resulting pointer must be within bounds of the same allocation.
    pub unsafe fn offset(&self, count: isize) -> Self {
        Self {
            ptr: self.ptr.offset(count),
        }
    }

    /// Add to the pointer (positive offset)
    ///
    /// # Safety
    ///
    /// The resulting pointer must be within bounds of the same allocation.
    pub unsafe fn add(&self, count: usize) -> Self {
        Self {
            ptr: self.ptr.add(count),
        }
    }

    /// Subtract from the pointer (negative offset)
    ///
    /// # Safety
    ///
    /// The resulting pointer must be within bounds of the same allocation.
    pub unsafe fn sub(&self, count: usize) -> Self {
        Self {
            ptr: self.ptr.sub(count),
        }
    }

    /// Read the value at the pointer location
    ///
    /// # Safety
    ///
    /// The pointer must be valid and properly aligned.
    pub unsafe fn read(&self) -> T
    where
        T: Copy,
    {
        self.ptr.read()
    }

    /// Write a value to the pointer location
    ///
    /// # Safety
    ///
    /// The pointer must be valid and properly aligned.
    pub unsafe fn write(&self, val: T) {
        self.ptr.write(val);
    }

    /// Cast to a different pointer type
    ///
    /// # Safety
    ///
    /// The pointer must be valid for the target type.
    pub fn cast<U>(&self) -> CPtr<U> {
        CPtr {
            ptr: self.ptr as *mut U,
        }
    }
}

impl<T> Clone for CPtr<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Copy for CPtr<T> {}

impl<T> Default for CPtr<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T> PartialEq for CPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T> Eq for CPtr<T> {}

impl<T> fmt::Debug for CPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CPtr({:p})", self.ptr)
    }
}

impl<T> fmt::Pointer for CPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

// ============================================================================
// CPtrConst - Const C Pointer Type
// ============================================================================

/// Const C pointer type - `const T*` in C
///
/// This explicitly models const-qualified pointers from C.
#[repr(transparent)]
pub struct CPtrConst<T> {
    ptr: *const T,
}

impl<T> CPtrConst<T> {
    /// Create a new CPtrConst from a raw const pointer
    pub fn new(ptr: *const T) -> Self {
        Self { ptr }
    }

    /// Create a null CPtrConst
    pub fn null() -> Self {
        Self {
            ptr: std::ptr::null(),
        }
    }

    /// Check if the pointer is null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Get the raw const pointer
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    /// Get a reference to the pointed value
    ///
    /// # Safety
    ///
    /// The pointer must be valid, non-null, and properly aligned.
    pub unsafe fn as_ref(&self) -> Option<&T> {
        self.ptr.as_ref()
    }

    /// Read the value at the pointer location
    ///
    /// # Safety
    ///
    /// The pointer must be valid and properly aligned.
    pub unsafe fn read(&self) -> T
    where
        T: Copy,
    {
        self.ptr.read()
    }

    /// Cast to a different pointer type
    pub fn cast<U>(&self) -> CPtrConst<U> {
        CPtrConst {
            ptr: self.ptr as *const U,
        }
    }
}

impl<T> Clone for CPtrConst<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Copy for CPtrConst<T> {}

impl<T> Default for CPtrConst<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T> From<CPtr<T>> for CPtrConst<T> {
    fn from(ptr: CPtr<T>) -> Self {
        Self { ptr: ptr.ptr }
    }
}

impl<T> fmt::Debug for CPtrConst<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CPtrConst({:p})", self.ptr)
    }
}

// ============================================================================
// CPtrRestrict - Restrict-Qualified Pointer
// ============================================================================

/// Restrict-qualified C pointer - `T* restrict` in C
///
/// This marker type indicates that the pointer is the only way to
/// access the pointed-to memory, enabling compiler optimizations.
#[repr(transparent)]
pub struct CPtrRestrict<T> {
    ptr: *mut T,
    _marker: PhantomData<T>,
}

impl<T> CPtrRestrict<T> {
    /// Create a restrict pointer
    ///
    /// # Safety
    ///
    /// The caller guarantees this is the only pointer to this memory.
    pub unsafe fn new(ptr: *mut T) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    /// Check if null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

impl<T> fmt::Debug for CPtrRestrict<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CPtrRestrict({:p})", self.ptr)
    }
}

// ============================================================================
// CArray - Fixed-Size C Array
// ============================================================================

/// Fixed-size C array - `T[N]` in C
///
/// This type provides a C-compatible fixed-size array.
///
/// ## Example
///
/// ```ignore
/// // C: char buffer[256];
/// // Aria: buffer: CArray[CChar, 256]
/// ```
#[repr(C)]
pub struct CArray<T, const N: usize> {
    data: [T; N],
}

impl<T, const N: usize> CArray<T, N> {
    /// Create a new CArray initialized with zeros
    pub fn zeroed() -> Self
    where
        T: Default + Copy,
    {
        Self {
            data: [T::default(); N],
        }
    }

    /// Create a new CArray from an existing array
    pub fn new(data: [T; N]) -> Self {
        Self { data }
    }

    /// Get the length of the array
    pub const fn len(&self) -> usize {
        N
    }

    /// Check if the array is empty
    pub const fn is_empty(&self) -> bool {
        N == 0
    }

    /// Get a pointer to the first element
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Get a mutable pointer to the first element
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr()
    }

    /// Get the underlying array as a slice
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Get the underlying array as a mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Get a CPtr to the start of the array
    pub fn ptr(&self) -> CPtr<T> {
        CPtr::new(self.data.as_ptr() as *mut T)
    }

    /// Get an element by index
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    /// Get a mutable element by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    /// Fill the array with a value
    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        self.data.fill(value);
    }

    /// Copy from a slice, returning number of elements copied
    pub fn copy_from_slice(&mut self, src: &[T]) -> usize
    where
        T: Copy,
    {
        let count = src.len().min(N);
        self.data[..count].copy_from_slice(&src[..count]);
        count
    }
}

impl<T: Copy + Default, const N: usize> Default for CArray<T, N> {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl<T, const N: usize> Deref for CArray<T, N> {
    type Target = [T; N];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, const N: usize> DerefMut for CArray<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T, const N: usize> Index<usize> for CArray<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T, const N: usize> IndexMut<usize> for CArray<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for CArray<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CArray<{}>", N)?;
        fmt::Debug::fmt(&self.data, f)
    }
}

impl<T: Clone, const N: usize> Clone for CArray<T, N> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl<T: Copy, const N: usize> Copy for CArray<T, N> {}

impl<T: PartialEq, const N: usize> PartialEq for CArray<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T: Eq, const N: usize> Eq for CArray<T, N> {}

// ============================================================================
// CArrayView - View into a C array (for borrowed arrays)
// ============================================================================

/// View into a C array - for borrowed array data from FFI
///
/// This type provides safe access to C array data without ownership.
pub struct CArrayView<'a, T> {
    ptr: *const T,
    len: usize,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> CArrayView<'a, T> {
    /// Create a new array view
    ///
    /// # Safety
    ///
    /// The pointer must be valid for `len` elements and remain valid
    /// for the lifetime 'a.
    pub unsafe fn new(ptr: *const T, len: usize) -> Self {
        Self {
            ptr,
            len,
            _marker: PhantomData,
        }
    }

    /// Get the length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    /// Convert to a slice
    ///
    /// # Safety
    ///
    /// The underlying data must still be valid.
    pub unsafe fn as_slice(&self) -> &'a [T] {
        if self.ptr.is_null() || self.len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(self.ptr, self.len)
        }
    }
}

impl<'a, T> fmt::Debug for CArrayView<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CArrayView({:p}, len={})", self.ptr, self.len)
    }
}

// ============================================================================
// CArrayMutView - Mutable view into a C array
// ============================================================================

/// Mutable view into a C array
pub struct CArrayMutView<'a, T> {
    ptr: *mut T,
    len: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> CArrayMutView<'a, T> {
    /// Create a new mutable array view
    ///
    /// # Safety
    ///
    /// The pointer must be valid for `len` elements and remain valid
    /// for the lifetime 'a. No other mutable references may exist.
    pub unsafe fn new(ptr: *mut T, len: usize) -> Self {
        Self {
            ptr,
            len,
            _marker: PhantomData,
        }
    }

    /// Get the length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    /// Convert to a mutable slice
    ///
    /// # Safety
    ///
    /// The underlying data must still be valid.
    pub unsafe fn as_mut_slice(&mut self) -> &'a mut [T] {
        if self.ptr.is_null() || self.len == 0 {
            &mut []
        } else {
            std::slice::from_raw_parts_mut(self.ptr, self.len)
        }
    }
}

impl<'a, T> fmt::Debug for CArrayMutView<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CArrayMutView({:p}, len={})", self.ptr, self.len)
    }
}

// ============================================================================
// Implement CCompatible for pointer types
// ============================================================================

impl<T> CCompatible for CPtr<T> where T: CCompatible {}
impl<T> CCompatible for CPtrConst<T> where T: CCompatible {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cptr_null() {
        let ptr: CPtr<i32> = CPtr::null();
        assert!(ptr.is_null());
    }

    #[test]
    fn test_cptr_not_null() {
        let mut value: i32 = 42;
        let ptr = CPtr::new(&mut value as *mut i32);
        assert!(!ptr.is_null());
    }

    #[test]
    fn test_cptr_read_write() {
        let mut value: i32 = 42;
        let ptr = CPtr::new(&mut value as *mut i32);

        unsafe {
            assert_eq!(ptr.read(), 42);
            ptr.write(100);
            assert_eq!(ptr.read(), 100);
        }
    }

    #[test]
    fn test_carray_creation() {
        let arr: CArray<i32, 4> = CArray::zeroed();
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0], 0);
    }

    #[test]
    fn test_carray_new() {
        let arr = CArray::new([1, 2, 3, 4, 5]);
        assert_eq!(arr.len(), 5);
        assert_eq!(arr[2], 3);
    }

    #[test]
    fn test_carray_fill() {
        let mut arr: CArray<i32, 4> = CArray::zeroed();
        arr.fill(42);
        assert!(arr.as_slice().iter().all(|&x| x == 42));
    }

    #[test]
    fn test_carray_copy_from_slice() {
        let mut arr: CArray<i32, 4> = CArray::zeroed();
        let src = [1, 2, 3];
        let copied = arr.copy_from_slice(&src);
        assert_eq!(copied, 3);
        assert_eq!(arr[0], 1);
        assert_eq!(arr[1], 2);
        assert_eq!(arr[2], 3);
        assert_eq!(arr[3], 0);
    }

    #[test]
    fn test_carray_ptr() {
        let arr: CArray<i32, 4> = CArray::new([1, 2, 3, 4]);
        let ptr = arr.ptr();
        assert!(!ptr.is_null());
    }

    #[test]
    fn test_cptr_const_from_cptr() {
        let mut value: i32 = 42;
        let ptr = CPtr::new(&mut value as *mut i32);
        let const_ptr: CPtrConst<i32> = ptr.into();
        assert!(!const_ptr.is_null());
    }

    #[test]
    fn test_carray_view() {
        let data = [1, 2, 3, 4, 5];
        let view = unsafe { CArrayView::new(data.as_ptr(), data.len()) };
        assert_eq!(view.len(), 5);
        unsafe {
            assert_eq!(view.as_slice(), &[1, 2, 3, 4, 5]);
        }
    }
}
