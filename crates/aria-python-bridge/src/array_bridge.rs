//! NumPy Array Bridge for Zero-Copy Interop
//!
//! This module provides the infrastructure for zero-copy data exchange
//! with NumPy arrays. Based on ARIA-M10 Python Interop milestone.
//!
//! ## Goals
//!
//! 1. **Zero-copy when possible**: Share memory between Aria and NumPy
//! 2. **Fallback to copy**: When layouts are incompatible
//! 3. **Clear ownership**: Who owns the memory at any given time
//! 4. **Type safety**: Map NumPy dtypes to Aria types
//!
//! ## Array Protocol
//!
//! NumPy's array interface defines:
//! - `__array_interface__`: Python dict with array metadata
//! - Buffer protocol: C-level memory sharing
//!
//! ## Memory Layout
//!
//! For zero-copy, arrays must be:
//! - Contiguous (C or Fortran order)
//! - Properly aligned for the dtype
//! - Have compatible element types

use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use smol_str::SmolStr;

use crate::error::{PyBridgeError, PyBridgeResult};
use crate::py_types::PyArrayRef;

// ============================================================================
// DType - NumPy Data Type Representation
// ============================================================================

/// NumPy data type representation.
///
/// This enum covers the common NumPy dtypes that Aria supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DType {
    /// Boolean (numpy.bool_)
    Bool,

    /// Signed 8-bit integer (numpy.int8)
    Int8,
    /// Signed 16-bit integer (numpy.int16)
    Int16,
    /// Signed 32-bit integer (numpy.int32)
    Int32,
    /// Signed 64-bit integer (numpy.int64)
    Int64,

    /// Unsigned 8-bit integer (numpy.uint8)
    UInt8,
    /// Unsigned 16-bit integer (numpy.uint16)
    UInt16,
    /// Unsigned 32-bit integer (numpy.uint32)
    UInt32,
    /// Unsigned 64-bit integer (numpy.uint64)
    UInt64,

    /// 32-bit floating point (numpy.float32)
    Float32,
    /// 64-bit floating point (numpy.float64)
    Float64,

    /// 64-bit complex (numpy.complex64 = 2x float32)
    Complex64,
    /// 128-bit complex (numpy.complex128 = 2x float64)
    Complex128,
}

impl DType {
    /// Get the size in bytes for this dtype
    pub fn size(&self) -> usize {
        match self {
            DType::Bool => 1,
            DType::Int8 | DType::UInt8 => 1,
            DType::Int16 | DType::UInt16 => 2,
            DType::Int32 | DType::UInt32 | DType::Float32 => 4,
            DType::Int64 | DType::UInt64 | DType::Float64 | DType::Complex64 => 8,
            DType::Complex128 => 16,
        }
    }

    /// Get the alignment requirement for this dtype
    pub fn alignment(&self) -> usize {
        self.size().min(8) // Max alignment is 8 bytes
    }

    /// Get the NumPy type character
    pub fn type_char(&self) -> char {
        match self {
            DType::Bool => '?',
            DType::Int8 => 'b',
            DType::Int16 => 'h',
            DType::Int32 => 'i',
            DType::Int64 => 'l',
            DType::UInt8 => 'B',
            DType::UInt16 => 'H',
            DType::UInt32 => 'I',
            DType::UInt64 => 'L',
            DType::Float32 => 'f',
            DType::Float64 => 'd',
            DType::Complex64 => 'F',
            DType::Complex128 => 'D',
        }
    }

    /// Get the NumPy dtype string (e.g., "float64")
    pub fn name(&self) -> &'static str {
        match self {
            DType::Bool => "bool",
            DType::Int8 => "int8",
            DType::Int16 => "int16",
            DType::Int32 => "int32",
            DType::Int64 => "int64",
            DType::UInt8 => "uint8",
            DType::UInt16 => "uint16",
            DType::UInt32 => "uint32",
            DType::UInt64 => "uint64",
            DType::Float32 => "float32",
            DType::Float64 => "float64",
            DType::Complex64 => "complex64",
            DType::Complex128 => "complex128",
        }
    }

    /// Parse dtype from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "bool" | "bool_" | "?" => Some(DType::Bool),
            "int8" | "i1" | "b" => Some(DType::Int8),
            "int16" | "i2" | "h" => Some(DType::Int16),
            "int32" | "i4" | "i" => Some(DType::Int32),
            "int64" | "i8" | "l" => Some(DType::Int64),
            "uint8" | "u1" | "B" => Some(DType::UInt8),
            "uint16" | "u2" | "H" => Some(DType::UInt16),
            "uint32" | "u4" | "I" => Some(DType::UInt32),
            "uint64" | "u8" | "L" => Some(DType::UInt64),
            "float32" | "f4" | "f" => Some(DType::Float32),
            "float64" | "f8" | "d" | "float" => Some(DType::Float64),
            "complex64" | "c8" | "F" => Some(DType::Complex64),
            "complex128" | "c16" | "D" | "complex" => Some(DType::Complex128),
            _ => None,
        }
    }

    /// Check if this is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            DType::Int8
                | DType::Int16
                | DType::Int32
                | DType::Int64
                | DType::UInt8
                | DType::UInt16
                | DType::UInt32
                | DType::UInt64
        )
    }

    /// Check if this is a floating point type
    pub fn is_float(&self) -> bool {
        matches!(self, DType::Float32 | DType::Float64)
    }

    /// Check if this is a complex type
    pub fn is_complex(&self) -> bool {
        matches!(self, DType::Complex64 | DType::Complex128)
    }

    /// Check if this is a signed type
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            DType::Int8
                | DType::Int16
                | DType::Int32
                | DType::Int64
                | DType::Float32
                | DType::Float64
                | DType::Complex64
                | DType::Complex128
        )
    }
}

impl fmt::Display for DType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// ArrayLayout - Memory Layout Information
// ============================================================================

/// Memory layout order for multi-dimensional arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayOrder {
    /// C-contiguous (row-major): last dimension varies fastest
    C,
    /// Fortran-contiguous (column-major): first dimension varies fastest
    Fortran,
    /// Neither C nor Fortran contiguous
    Neither,
}

/// Complete memory layout description for an array.
#[derive(Debug, Clone)]
pub struct ArrayLayout {
    /// Shape of the array (dimensions)
    pub shape: Vec<usize>,
    /// Strides in bytes for each dimension
    pub strides: Vec<isize>,
    /// Data type
    pub dtype: DType,
    /// Memory order
    pub order: ArrayOrder,
    /// Starting offset from base pointer
    pub offset: usize,
}

impl ArrayLayout {
    /// Create a new C-contiguous layout for the given shape and dtype
    pub fn c_contiguous(shape: Vec<usize>, dtype: DType) -> Self {
        let mut strides = Vec::with_capacity(shape.len());
        let mut stride = dtype.size() as isize;

        // Calculate strides from last to first dimension
        for &dim in shape.iter().rev() {
            strides.push(stride);
            stride *= dim as isize;
        }
        strides.reverse();

        Self {
            shape,
            strides,
            dtype,
            order: ArrayOrder::C,
            offset: 0,
        }
    }

    /// Create a new Fortran-contiguous layout for the given shape and dtype
    pub fn fortran_contiguous(shape: Vec<usize>, dtype: DType) -> Self {
        let mut strides = Vec::with_capacity(shape.len());
        let mut stride = dtype.size() as isize;

        // Calculate strides from first to last dimension
        for &dim in shape.iter() {
            strides.push(stride);
            stride *= dim as isize;
        }

        Self {
            shape,
            strides,
            dtype,
            order: ArrayOrder::Fortran,
            offset: 0,
        }
    }

    /// Get the number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Get the total number of elements
    pub fn size(&self) -> usize {
        self.shape.iter().product()
    }

    /// Get the total size in bytes (element count * element size)
    pub fn nbytes(&self) -> usize {
        self.size() * self.dtype.size()
    }

    /// Check if the layout is contiguous
    pub fn is_contiguous(&self) -> bool {
        self.order != ArrayOrder::Neither
    }

    /// Check if layout is C-contiguous
    pub fn is_c_contiguous(&self) -> bool {
        self.order == ArrayOrder::C
    }

    /// Check if layout is Fortran-contiguous
    pub fn is_fortran_contiguous(&self) -> bool {
        self.order == ArrayOrder::Fortran
    }

    /// Check if this layout supports zero-copy sharing
    pub fn supports_zero_copy(&self) -> bool {
        // Must be contiguous and properly aligned
        self.is_contiguous() && (self.offset % self.dtype.alignment() == 0)
    }

    /// Calculate flat index from multi-dimensional indices
    pub fn flat_index(&self, indices: &[usize]) -> Option<usize> {
        if indices.len() != self.shape.len() {
            return None;
        }

        for (idx, dim) in indices.iter().zip(self.shape.iter()) {
            if *idx >= *dim {
                return None;
            }
        }

        let byte_offset: isize = indices
            .iter()
            .zip(self.strides.iter())
            .map(|(&idx, &stride)| (idx as isize) * stride)
            .sum();

        Some(((self.offset as isize) + byte_offset) as usize / self.dtype.size())
    }

    /// Create a view with a slice
    pub fn slice_view(&self, start: &[usize], end: &[usize]) -> PyBridgeResult<Self> {
        if start.len() != self.ndim() || end.len() != self.ndim() {
            return Err(PyBridgeError::array_layout_incompatible(
                "slice dimensions don't match array dimensions",
            ));
        }

        let mut new_shape = Vec::with_capacity(self.ndim());
        let mut new_offset = self.offset;

        for (((&s, &e), &stride), &dim) in start
            .iter()
            .zip(end.iter())
            .zip(self.strides.iter())
            .zip(self.shape.iter())
        {
            if s > e || e > dim {
                return Err(PyBridgeError::array_layout_incompatible(format!(
                    "invalid slice {}:{} for dimension of size {}",
                    s, e, dim
                )));
            }
            new_shape.push(e - s);
            new_offset = ((new_offset as isize) + (s as isize) * stride) as usize;
        }

        Ok(Self {
            shape: new_shape,
            strides: self.strides.clone(),
            dtype: self.dtype,
            order: ArrayOrder::Neither, // Slices may not be contiguous
            offset: new_offset,
        })
    }

    /// Reshape to new shape (must have same total size)
    pub fn reshape(&self, new_shape: Vec<usize>) -> PyBridgeResult<Self> {
        let new_size: usize = new_shape.iter().product();
        if new_size != self.size() {
            return Err(PyBridgeError::array_layout_incompatible(format!(
                "cannot reshape array of size {} to shape {:?}",
                self.size(),
                new_shape
            )));
        }

        if !self.is_contiguous() {
            return Err(PyBridgeError::array_layout_incompatible(
                "cannot reshape non-contiguous array",
            ));
        }

        // Create new layout with same order
        match self.order {
            ArrayOrder::C => Ok(Self::c_contiguous(new_shape, self.dtype)),
            ArrayOrder::Fortran => Ok(Self::fortran_contiguous(new_shape, self.dtype)),
            ArrayOrder::Neither => Err(PyBridgeError::array_layout_incompatible(
                "cannot reshape non-contiguous array",
            )),
        }
    }

    /// Transpose the array (reverse dimensions and strides)
    pub fn transpose(&self) -> Self {
        let mut new_shape = self.shape.clone();
        let mut new_strides = self.strides.clone();
        new_shape.reverse();
        new_strides.reverse();

        let new_order = match self.order {
            ArrayOrder::C => ArrayOrder::Fortran,
            ArrayOrder::Fortran => ArrayOrder::C,
            ArrayOrder::Neither => ArrayOrder::Neither,
        };

        Self {
            shape: new_shape,
            strides: new_strides,
            dtype: self.dtype,
            order: new_order,
            offset: self.offset,
        }
    }
}

// ============================================================================
// ArrayBridge - Main Array Interop Structure
// ============================================================================

/// Global array registry for tracking live arrays.
static NEXT_ARRAY_ID: AtomicU64 = AtomicU64::new(1);

/// Ownership state for array data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayOwnership {
    /// Aria owns the data (allocated by Aria)
    AriaOwned,
    /// Python owns the data (allocated by NumPy)
    PythonOwned,
    /// Shared ownership (reference counted)
    Shared,
    /// View into another array (borrowed)
    View,
}

/// Array data storage (stub for actual buffer)
#[derive(Debug)]
pub struct ArrayData {
    /// The raw data bytes (stub - in real impl this would be a pointer)
    data: Vec<u8>,
    /// Whether the data can be modified
    readonly: bool,
}

impl ArrayData {
    /// Create new owned data
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
            readonly: false,
        }
    }

    /// Create from existing bytes
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data,
            readonly: false,
        }
    }

    /// Get a slice of the data
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Get a mutable slice of the data
    pub fn as_mut_slice(&mut self) -> PyBridgeResult<&mut [u8]> {
        if self.readonly {
            return Err(PyBridgeError::BorrowedMutation {
                context: "array data is read-only".to_string(),
            });
        }
        Ok(&mut self.data)
    }

    /// Get the size in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Set readonly flag
    pub fn set_readonly(&mut self, readonly: bool) {
        self.readonly = readonly;
    }

    /// Check if readonly
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }
}

/// Bridge for zero-copy NumPy array interop.
///
/// This structure manages array data that can be shared between Aria and Python.
#[derive(Debug)]
pub struct ArrayBridge {
    /// Unique identifier for this array
    id: u64,
    /// Memory layout
    layout: ArrayLayout,
    /// The actual data
    data: Arc<RwLock<ArrayData>>,
    /// Ownership state
    ownership: ArrayOwnership,
    /// Base array ID (if this is a view)
    base_id: Option<u64>,
}

impl ArrayBridge {
    /// Create a new zero-initialized array
    pub fn zeros(shape: Vec<usize>, dtype: DType) -> Self {
        let layout = ArrayLayout::c_contiguous(shape, dtype);
        let data = ArrayData::new(layout.nbytes());

        Self {
            id: NEXT_ARRAY_ID.fetch_add(1, Ordering::Relaxed),
            layout,
            data: Arc::new(RwLock::new(data)),
            ownership: ArrayOwnership::AriaOwned,
            base_id: None,
        }
    }

    /// Create from existing data
    pub fn from_data(data: Vec<u8>, shape: Vec<usize>, dtype: DType) -> PyBridgeResult<Self> {
        let layout = ArrayLayout::c_contiguous(shape, dtype);

        if data.len() != layout.nbytes() {
            return Err(PyBridgeError::array_layout_incompatible(format!(
                "data size {} doesn't match expected size {}",
                data.len(),
                layout.nbytes()
            )));
        }

        Ok(Self {
            id: NEXT_ARRAY_ID.fetch_add(1, Ordering::Relaxed),
            layout,
            data: Arc::new(RwLock::new(ArrayData::from_bytes(data))),
            ownership: ArrayOwnership::AriaOwned,
            base_id: None,
        })
    }

    /// Get the array ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the layout
    pub fn layout(&self) -> &ArrayLayout {
        &self.layout
    }

    /// Get the shape
    pub fn shape(&self) -> &[usize] {
        &self.layout.shape
    }

    /// Get the dtype
    pub fn dtype(&self) -> DType {
        self.layout.dtype
    }

    /// Get the number of dimensions
    pub fn ndim(&self) -> usize {
        self.layout.ndim()
    }

    /// Get the total number of elements
    pub fn size(&self) -> usize {
        self.layout.size()
    }

    /// Get the total size in bytes
    pub fn nbytes(&self) -> usize {
        self.layout.nbytes()
    }

    /// Check if this is a view
    pub fn is_view(&self) -> bool {
        self.base_id.is_some()
    }

    /// Check if this supports zero-copy to Python
    pub fn supports_zero_copy(&self) -> bool {
        self.layout.supports_zero_copy()
    }

    /// Get ownership state
    pub fn ownership(&self) -> ArrayOwnership {
        self.ownership
    }

    /// Create a PyArrayRef for use in Python
    pub fn to_py_ref(&self) -> PyArrayRef {
        PyArrayRef::new(self.id, self.layout.shape.clone(), self.layout.dtype.name())
    }

    /// Create a view with a slice
    pub fn slice(&self, start: &[usize], end: &[usize]) -> PyBridgeResult<Self> {
        let new_layout = self.layout.slice_view(start, end)?;

        Ok(Self {
            id: NEXT_ARRAY_ID.fetch_add(1, Ordering::Relaxed),
            layout: new_layout,
            data: Arc::clone(&self.data),
            ownership: ArrayOwnership::View,
            base_id: Some(self.id),
        })
    }

    /// Reshape the array (creates a view if possible)
    pub fn reshape(&self, new_shape: Vec<usize>) -> PyBridgeResult<Self> {
        let new_layout = self.layout.reshape(new_shape)?;

        Ok(Self {
            id: NEXT_ARRAY_ID.fetch_add(1, Ordering::Relaxed),
            layout: new_layout,
            data: Arc::clone(&self.data),
            ownership: ArrayOwnership::View,
            base_id: Some(self.id),
        })
    }

    /// Transpose the array (creates a view)
    pub fn transpose(&self) -> Self {
        Self {
            id: NEXT_ARRAY_ID.fetch_add(1, Ordering::Relaxed),
            layout: self.layout.transpose(),
            data: Arc::clone(&self.data),
            ownership: ArrayOwnership::View,
            base_id: Some(self.id),
        }
    }

    /// Copy the array to ensure owned data
    pub fn copy(&self) -> PyBridgeResult<Self> {
        let data_guard = self
            .data
            .read()
            .map_err(|_| PyBridgeError::memory_error("failed to acquire read lock"))?;

        Ok(Self {
            id: NEXT_ARRAY_ID.fetch_add(1, Ordering::Relaxed),
            layout: ArrayLayout::c_contiguous(self.layout.shape.clone(), self.layout.dtype),
            data: Arc::new(RwLock::new(ArrayData::from_bytes(
                data_guard.as_slice().to_vec(),
            ))),
            ownership: ArrayOwnership::AriaOwned,
            base_id: None,
        })
    }

    /// Get read access to the raw data
    pub fn read_data<F, T>(&self, f: F) -> PyBridgeResult<T>
    where
        F: FnOnce(&[u8]) -> T,
    {
        let guard = self
            .data
            .read()
            .map_err(|_| PyBridgeError::memory_error("failed to acquire read lock"))?;
        Ok(f(guard.as_slice()))
    }

    /// Get write access to the raw data
    pub fn write_data<F, T>(&self, f: F) -> PyBridgeResult<T>
    where
        F: FnOnce(&mut [u8]) -> PyBridgeResult<T>,
    {
        let mut guard = self
            .data
            .write()
            .map_err(|_| PyBridgeError::memory_error("failed to acquire write lock"))?;
        f(guard.as_mut_slice()?)
    }
}

impl Clone for ArrayBridge {
    fn clone(&self) -> Self {
        // Clone creates a view, not a copy
        Self {
            id: NEXT_ARRAY_ID.fetch_add(1, Ordering::Relaxed),
            layout: self.layout.clone(),
            data: Arc::clone(&self.data),
            ownership: ArrayOwnership::View,
            base_id: Some(self.id),
        }
    }
}

// ============================================================================
// Typed Array Wrapper
// ============================================================================

/// Type-safe wrapper for ArrayBridge.
///
/// This provides compile-time type safety for array element access.
pub struct TypedArray<T> {
    bridge: ArrayBridge,
    _marker: PhantomData<T>,
}

impl<T: ArrayElement> TypedArray<T> {
    /// Create a new typed array
    pub fn new(shape: Vec<usize>) -> Self {
        Self {
            bridge: ArrayBridge::zeros(shape, T::dtype()),
            _marker: PhantomData,
        }
    }

    /// Get the underlying bridge
    pub fn bridge(&self) -> &ArrayBridge {
        &self.bridge
    }

    /// Get element at flat index
    pub fn get_flat(&self, index: usize) -> PyBridgeResult<T> {
        if index >= self.bridge.size() {
            return Err(PyBridgeError::Custom(format!(
                "index {} out of bounds for array of size {}",
                index,
                self.bridge.size()
            )));
        }

        self.bridge.read_data(|data| {
            let offset = index * std::mem::size_of::<T>();
            T::from_bytes(&data[offset..offset + std::mem::size_of::<T>()])
        })
    }

    /// Set element at flat index
    pub fn set_flat(&mut self, index: usize, value: T) -> PyBridgeResult<()> {
        if index >= self.bridge.size() {
            return Err(PyBridgeError::Custom(format!(
                "index {} out of bounds for array of size {}",
                index,
                self.bridge.size()
            )));
        }

        self.bridge.write_data(|data| {
            let offset = index * std::mem::size_of::<T>();
            let bytes = value.to_bytes();
            data[offset..offset + bytes.len()].copy_from_slice(&bytes);
            Ok(())
        })
    }
}

/// Trait for types that can be array elements
pub trait ArrayElement: Sized + Copy {
    /// Get the numpy dtype for this type
    fn dtype() -> DType;

    /// Convert from bytes
    fn from_bytes(bytes: &[u8]) -> Self;

    /// Convert to bytes
    fn to_bytes(&self) -> Vec<u8>;
}

impl ArrayElement for f64 {
    fn dtype() -> DType {
        DType::Float64
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let arr: [u8; 8] = bytes[..8].try_into().unwrap();
        f64::from_ne_bytes(arr)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_ne_bytes().to_vec()
    }
}

impl ArrayElement for f32 {
    fn dtype() -> DType {
        DType::Float32
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let arr: [u8; 4] = bytes[..4].try_into().unwrap();
        f32::from_ne_bytes(arr)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_ne_bytes().to_vec()
    }
}

impl ArrayElement for i64 {
    fn dtype() -> DType {
        DType::Int64
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let arr: [u8; 8] = bytes[..8].try_into().unwrap();
        i64::from_ne_bytes(arr)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_ne_bytes().to_vec()
    }
}

impl ArrayElement for i32 {
    fn dtype() -> DType {
        DType::Int32
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let arr: [u8; 4] = bytes[..4].try_into().unwrap();
        i32::from_ne_bytes(arr)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_ne_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtype_properties() {
        assert_eq!(DType::Float64.size(), 8);
        assert_eq!(DType::Float32.size(), 4);
        assert_eq!(DType::Int32.size(), 4);
        assert_eq!(DType::Int8.size(), 1);

        assert!(DType::Float64.is_float());
        assert!(DType::Int32.is_integer());
        assert!(DType::Complex128.is_complex());
    }

    #[test]
    fn test_dtype_parsing() {
        assert_eq!(DType::from_str("float64"), Some(DType::Float64));
        assert_eq!(DType::from_str("int32"), Some(DType::Int32));
        assert_eq!(DType::from_str("d"), Some(DType::Float64));
        assert_eq!(DType::from_str("unknown"), None);
    }

    #[test]
    fn test_array_layout_c_contiguous() {
        let layout = ArrayLayout::c_contiguous(vec![3, 4, 5], DType::Float64);

        assert_eq!(layout.ndim(), 3);
        assert_eq!(layout.size(), 60);
        assert_eq!(layout.nbytes(), 480);
        assert!(layout.is_c_contiguous());

        // C-contiguous: last dimension varies fastest
        assert_eq!(layout.strides, vec![160, 40, 8]); // 20*8, 5*8, 1*8
    }

    #[test]
    fn test_array_layout_fortran_contiguous() {
        let layout = ArrayLayout::fortran_contiguous(vec![3, 4, 5], DType::Float64);

        assert!(layout.is_fortran_contiguous());

        // Fortran-contiguous: first dimension varies fastest
        assert_eq!(layout.strides, vec![8, 24, 96]); // 1*8, 3*8, 12*8
    }

    #[test]
    fn test_array_layout_flat_index() {
        let layout = ArrayLayout::c_contiguous(vec![3, 4], DType::Float64);

        assert_eq!(layout.flat_index(&[0, 0]), Some(0));
        assert_eq!(layout.flat_index(&[0, 1]), Some(1));
        assert_eq!(layout.flat_index(&[1, 0]), Some(4));
        assert_eq!(layout.flat_index(&[2, 3]), Some(11));

        // Out of bounds
        assert_eq!(layout.flat_index(&[3, 0]), None);
        assert_eq!(layout.flat_index(&[0, 4]), None);
    }

    #[test]
    fn test_array_layout_transpose() {
        let layout = ArrayLayout::c_contiguous(vec![3, 4], DType::Float64);
        let transposed = layout.transpose();

        assert_eq!(transposed.shape, vec![4, 3]);
        assert!(transposed.is_fortran_contiguous());
    }

    #[test]
    fn test_array_bridge_creation() {
        let arr = ArrayBridge::zeros(vec![3, 4], DType::Float64);

        assert_eq!(arr.shape(), &[3, 4]);
        assert_eq!(arr.dtype(), DType::Float64);
        assert_eq!(arr.ndim(), 2);
        assert_eq!(arr.size(), 12);
        assert_eq!(arr.nbytes(), 96);
        assert!(!arr.is_view());
        assert!(arr.supports_zero_copy());
    }

    #[test]
    fn test_array_bridge_from_data() {
        let data = vec![0u8; 24]; // 3 elements * 8 bytes
        let arr = ArrayBridge::from_data(data, vec![3], DType::Float64).unwrap();

        assert_eq!(arr.shape(), &[3]);
        assert_eq!(arr.nbytes(), 24);
    }

    #[test]
    fn test_array_bridge_reshape() {
        let arr = ArrayBridge::zeros(vec![12], DType::Float64);
        let reshaped = arr.reshape(vec![3, 4]).unwrap();

        assert_eq!(reshaped.shape(), &[3, 4]);
        assert!(reshaped.is_view());
    }

    #[test]
    fn test_array_bridge_slice() {
        let arr = ArrayBridge::zeros(vec![10, 10], DType::Float64);
        let sliced = arr.slice(&[2, 3], &[5, 7]).unwrap();

        assert_eq!(sliced.shape(), &[3, 4]);
        assert!(sliced.is_view());
    }

    #[test]
    fn test_typed_array() {
        let mut arr: TypedArray<f64> = TypedArray::new(vec![5]);

        arr.set_flat(0, 1.0).unwrap();
        arr.set_flat(1, 2.0).unwrap();
        arr.set_flat(2, 3.0).unwrap();

        assert_eq!(arr.get_flat(0).unwrap(), 1.0);
        assert_eq!(arr.get_flat(1).unwrap(), 2.0);
        assert_eq!(arr.get_flat(2).unwrap(), 3.0);
    }

    #[test]
    fn test_array_ownership() {
        let owned = ArrayBridge::zeros(vec![10], DType::Float64);
        assert_eq!(owned.ownership(), ArrayOwnership::AriaOwned);

        let view = owned.reshape(vec![2, 5]).unwrap();
        assert_eq!(view.ownership(), ArrayOwnership::View);

        let copied = view.copy().unwrap();
        assert_eq!(copied.ownership(), ArrayOwnership::AriaOwned);
    }
}
