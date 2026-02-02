//! Error Types for Python Bridge
//!
//! This module provides error types for Python interop operations.
//! Based on ARIA-M10 Python Interop milestone.
//!
//! ## Error Categories
//!
//! - GIL acquisition failures
//! - Type conversion errors
//! - Python exceptions
//! - Memory/ownership issues
//! - Module/function not found

use std::fmt;
use thiserror::Error;

/// Result type for Python bridge operations
pub type PyBridgeResult<T> = Result<T, PyBridgeError>;

/// Python bridge error types
#[derive(Error, Debug, Clone)]
pub enum PyBridgeError {
    /// Failed to acquire the GIL
    #[error("failed to acquire Python GIL: {reason}")]
    GilAcquisitionFailed {
        /// Reason for failure
        reason: String,
    },

    /// GIL not held when required
    #[error("GIL not held: {context}")]
    GilNotHeld {
        /// Description of the operation that required GIL
        context: String,
    },

    /// Python interpreter not initialized
    #[error("Python interpreter not initialized")]
    InterpreterNotInitialized,

    /// Python exception was raised
    #[error("Python exception: {exception_type}: {message}")]
    PythonException {
        /// Python exception type (e.g., "TypeError", "ValueError")
        exception_type: String,
        /// Exception message
        message: String,
        /// Optional traceback
        traceback: Option<String>,
    },

    /// Type conversion failed
    #[error("type conversion failed: cannot convert {from_type} to {to_type}: {reason}")]
    ConversionFailed {
        /// Source type name
        from_type: String,
        /// Target type name
        to_type: String,
        /// Reason for failure
        reason: String,
    },

    /// Type mismatch during conversion
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected type
        expected: String,
        /// Actual type received
        actual: String,
    },

    /// Module not found
    #[error("Python module not found: {module_name}")]
    ModuleNotFound {
        /// Name of the module
        module_name: String,
    },

    /// Attribute not found on object
    #[error("attribute '{attribute}' not found on {object_type}")]
    AttributeNotFound {
        /// Name of the attribute
        attribute: String,
        /// Type of the object
        object_type: String,
    },

    /// Function/method not found
    #[error("function '{function_name}' not found in module '{module_name}'")]
    FunctionNotFound {
        /// Name of the function
        function_name: String,
        /// Name of the module
        module_name: String,
    },

    /// Wrong number of arguments
    #[error("wrong number of arguments: expected {expected}, got {actual}")]
    WrongArgumentCount {
        /// Expected count
        expected: usize,
        /// Actual count
        actual: usize,
    },

    /// Array layout incompatible for zero-copy
    #[error("array layout incompatible for zero-copy: {reason}")]
    ArrayLayoutIncompatible {
        /// Description of the incompatibility
        reason: String,
    },

    /// Array dtype not supported
    #[error("unsupported array dtype: {dtype}")]
    UnsupportedDtype {
        /// The unsupported dtype string
        dtype: String,
    },

    /// Memory error (allocation, access, etc.)
    #[error("memory error: {message}")]
    MemoryError {
        /// Error message
        message: String,
    },

    /// Null pointer encountered
    #[error("null Python object: {context}")]
    NullObject {
        /// Context where null was encountered
        context: String,
    },

    /// Reference count error
    #[error("reference count error: {message}")]
    RefCountError {
        /// Error message
        message: String,
    },

    /// Ownership violation
    #[error("ownership violation: {message}")]
    OwnershipViolation {
        /// Description of the violation
        message: String,
    },

    /// Object is borrowed and cannot be mutated
    #[error("cannot mutate borrowed object: {context}")]
    BorrowedMutation {
        /// Context description
        context: String,
    },

    /// Numeric overflow during conversion
    #[error("numeric overflow: {value} cannot be represented as {target_type}")]
    NumericOverflow {
        /// String representation of the value
        value: String,
        /// Target type name
        target_type: String,
    },

    /// String encoding error
    #[error("string encoding error: {message}")]
    EncodingError {
        /// Error message
        message: String,
    },

    /// Operation not supported
    #[error("operation not supported: {operation}")]
    NotSupported {
        /// Description of the operation
        operation: String,
    },

    /// Custom error with message
    #[error("{0}")]
    Custom(String),
}

impl PyBridgeError {
    /// Create a GIL acquisition failed error
    pub fn gil_failed(reason: impl Into<String>) -> Self {
        PyBridgeError::GilAcquisitionFailed {
            reason: reason.into(),
        }
    }

    /// Create a GIL not held error
    pub fn gil_not_held(context: impl Into<String>) -> Self {
        PyBridgeError::GilNotHeld {
            context: context.into(),
        }
    }

    /// Create a Python exception error
    pub fn exception(
        exception_type: impl Into<String>,
        message: impl Into<String>,
        traceback: Option<String>,
    ) -> Self {
        PyBridgeError::PythonException {
            exception_type: exception_type.into(),
            message: message.into(),
            traceback,
        }
    }

    /// Create a conversion failed error
    pub fn conversion_failed(
        from_type: impl Into<String>,
        to_type: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        PyBridgeError::ConversionFailed {
            from_type: from_type.into(),
            to_type: to_type.into(),
            reason: reason.into(),
        }
    }

    /// Create a type mismatch error
    pub fn type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        PyBridgeError::TypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a module not found error
    pub fn module_not_found(module_name: impl Into<String>) -> Self {
        PyBridgeError::ModuleNotFound {
            module_name: module_name.into(),
        }
    }

    /// Create an attribute not found error
    pub fn attribute_not_found(attribute: impl Into<String>, object_type: impl Into<String>) -> Self {
        PyBridgeError::AttributeNotFound {
            attribute: attribute.into(),
            object_type: object_type.into(),
        }
    }

    /// Create a function not found error
    pub fn function_not_found(function_name: impl Into<String>, module_name: impl Into<String>) -> Self {
        PyBridgeError::FunctionNotFound {
            function_name: function_name.into(),
            module_name: module_name.into(),
        }
    }

    /// Create an array layout incompatible error
    pub fn array_layout_incompatible(reason: impl Into<String>) -> Self {
        PyBridgeError::ArrayLayoutIncompatible {
            reason: reason.into(),
        }
    }

    /// Create an unsupported dtype error
    pub fn unsupported_dtype(dtype: impl Into<String>) -> Self {
        PyBridgeError::UnsupportedDtype {
            dtype: dtype.into(),
        }
    }

    /// Create a memory error
    pub fn memory_error(message: impl Into<String>) -> Self {
        PyBridgeError::MemoryError {
            message: message.into(),
        }
    }

    /// Create a null object error
    pub fn null_object(context: impl Into<String>) -> Self {
        PyBridgeError::NullObject {
            context: context.into(),
        }
    }

    /// Create an ownership violation error
    pub fn ownership_violation(message: impl Into<String>) -> Self {
        PyBridgeError::OwnershipViolation {
            message: message.into(),
        }
    }

    /// Create a numeric overflow error
    pub fn numeric_overflow(value: impl Into<String>, target_type: impl Into<String>) -> Self {
        PyBridgeError::NumericOverflow {
            value: value.into(),
            target_type: target_type.into(),
        }
    }

    /// Create an encoding error
    pub fn encoding_error(message: impl Into<String>) -> Self {
        PyBridgeError::EncodingError {
            message: message.into(),
        }
    }

    /// Create a not supported error
    pub fn not_supported(operation: impl Into<String>) -> Self {
        PyBridgeError::NotSupported {
            operation: operation.into(),
        }
    }

    /// Create a custom error
    pub fn custom(message: impl Into<String>) -> Self {
        PyBridgeError::Custom(message.into())
    }

    /// Check if this is a type-related error
    pub fn is_type_error(&self) -> bool {
        matches!(
            self,
            PyBridgeError::ConversionFailed { .. }
                | PyBridgeError::TypeMismatch { .. }
                | PyBridgeError::UnsupportedDtype { .. }
        )
    }

    /// Check if this is a GIL-related error
    pub fn is_gil_error(&self) -> bool {
        matches!(
            self,
            PyBridgeError::GilAcquisitionFailed { .. } | PyBridgeError::GilNotHeld { .. }
        )
    }

    /// Check if this is a Python exception
    pub fn is_python_exception(&self) -> bool {
        matches!(self, PyBridgeError::PythonException { .. })
    }
}

/// Source location for Python bridge error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PySourceLocation {
    /// Aria source file
    pub aria_file: Option<String>,
    /// Aria source line
    pub aria_line: Option<u32>,
    /// Python module (if applicable)
    pub python_module: Option<String>,
    /// Python function (if applicable)
    pub python_function: Option<String>,
}

impl PySourceLocation {
    /// Create an empty source location
    pub fn new() -> Self {
        Self {
            aria_file: None,
            aria_line: None,
            python_module: None,
            python_function: None,
        }
    }

    /// Set the Aria source location
    pub fn with_aria(mut self, file: impl Into<String>, line: u32) -> Self {
        self.aria_file = Some(file.into());
        self.aria_line = Some(line);
        self
    }

    /// Set the Python location
    pub fn with_python(mut self, module: impl Into<String>, function: impl Into<String>) -> Self {
        self.python_module = Some(module.into());
        self.python_function = Some(function.into());
        self
    }
}

impl Default for PySourceLocation {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PySourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if let (Some(file), Some(line)) = (&self.aria_file, self.aria_line) {
            parts.push(format!("{}:{}", file, line));
        }

        if let (Some(module), Some(func)) = (&self.python_module, &self.python_function) {
            parts.push(format!("Python: {}.{}", module, func));
        }

        if parts.is_empty() {
            write!(f, "<unknown location>")
        } else {
            write!(f, "{}", parts.join(" -> "))
        }
    }
}

/// Python bridge diagnostic with full context
#[derive(Debug, Clone)]
pub struct PyBridgeDiagnostic {
    /// The error
    pub error: PyBridgeError,
    /// Source location
    pub location: PySourceLocation,
    /// Additional notes
    pub notes: Vec<String>,
    /// Help suggestion
    pub help: Option<String>,
}

impl PyBridgeDiagnostic {
    /// Create a new diagnostic
    pub fn new(error: PyBridgeError) -> Self {
        Self {
            error,
            location: PySourceLocation::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    /// Add location information
    pub fn with_location(mut self, location: PySourceLocation) -> Self {
        self.location = location;
        self
    }

    /// Add a note
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

impl fmt::Display for PyBridgeDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "error: {}", self.error)?;

        if self.location.aria_file.is_some() || self.location.python_module.is_some() {
            writeln!(f, "  --> {}", self.location)?;
        }

        for note in &self.notes {
            writeln!(f, "note: {}", note)?;
        }

        if let Some(ref help) = self.help {
            writeln!(f, "help: {}", help)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_error() {
        let err = PyBridgeError::conversion_failed("int", "String", "incompatible types");
        assert!(err.is_type_error());
        assert!(err.to_string().contains("int"));
        assert!(err.to_string().contains("String"));
    }

    #[test]
    fn test_python_exception() {
        let err = PyBridgeError::exception("ValueError", "invalid value", None);
        assert!(err.is_python_exception());
        assert!(err.to_string().contains("ValueError"));
    }

    #[test]
    fn test_gil_error() {
        let err = PyBridgeError::gil_not_held("array conversion");
        assert!(err.is_gil_error());
    }

    #[test]
    fn test_source_location() {
        let loc = PySourceLocation::new()
            .with_aria("src/main.aria", 42)
            .with_python("numpy", "array");

        let s = loc.to_string();
        assert!(s.contains("src/main.aria:42"));
        assert!(s.contains("numpy.array"));
    }

    #[test]
    fn test_diagnostic_formatting() {
        let diag = PyBridgeDiagnostic::new(PyBridgeError::type_mismatch("int", "str"))
            .with_location(
                PySourceLocation::new()
                    .with_aria("test.aria", 10)
                    .with_python("pandas", "read_csv")
            )
            .with_note("conversion happened here")
            .with_help("use explicit type annotation");

        let output = diag.to_string();
        assert!(output.contains("error:"));
        assert!(output.contains("note:"));
        assert!(output.contains("help:"));
    }
}
