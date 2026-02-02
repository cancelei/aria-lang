//! # Aria Python Bridge
//!
//! Python interoperability bridge for calling Python from Aria.
//!
//! This crate provides the infrastructure for zero-copy data exchange
//! with Python libraries (NumPy, Pandas, ML frameworks).
//!
//! ## Overview
//!
//! Based on ARIA-M10: Python Interop milestone, this crate provides:
//! - Python type representations (`PyObject`, `PyList`, `PyDict`, `PyArray`)
//! - Type conversion traits (`ToPython`, `FromPython`)
//! - NumPy array bridge structures for zero-copy when possible
//! - GIL management abstractions
//! - Python function call infrastructure
//!
//! ## Design Goals
//!
//! 1. **Zero-copy arrays**: Share memory with NumPy without copying
//! 2. **GIL awareness**: Minimize GIL contention for performance
//! 3. **Type safety**: Map Python's dynamic types to Aria's static types
//! 4. **Ownership clarity**: Clear memory ownership across boundaries
//!
//! ## Example (Target Syntax)
//!
//! ```text
//! # In Aria code:
//! extern Python from numpy as np
//!
//! fn analyze(data: Array<Float>) -> Array<Float>
//!     arr = np.array(data)  # Zero-copy conversion
//!     result = np.sqrt(arr)
//!     Array.from(result)    # Zero-copy back
//! end
//! ```
//!
//! ## Module Structure
//!
//! - [`py_types`]: Python value type representations
//! - [`conversion`]: Type conversion traits and implementations
//! - [`array_bridge`]: NumPy array zero-copy bridge
//! - [`gil`]: GIL management abstractions
//! - [`call`]: Python function call infrastructure
//! - [`error`]: Error types for Python interop

pub mod array_bridge;
pub mod call;
pub mod conversion;
pub mod error;
pub mod gil;
pub mod py_types;

// Re-export main types for convenience
pub use array_bridge::{ArrayBridge, ArrayLayout, DType};
pub use call::{PyCallable, PyModule};
pub use conversion::{FromPython, ToPython};
pub use error::{PyBridgeError, PyBridgeResult};
pub use gil::{GilGuard, GilPool, GilState};
pub use py_types::{PyDict, PyList, PyObject, PyValue};
