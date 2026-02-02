//! FFI Error Types
//!
//! This module provides error types for FFI operations.
//! Based on ARIA-PD-010 Section 6.3 Diagnostic Messages.
//!
//! ## Error Categories
//!
//! - Ownership violations (use after transfer, double free)
//! - Null pointer dereferences
//! - Type mismatches
//! - ABI incompatibilities
//! - String encoding issues

use std::ffi::NulError;
use std::fmt;
use thiserror::Error;

/// Result type for FFI operations
pub type FfiResult<T> = Result<T, FfiError>;

/// FFI error types
#[derive(Error, Debug, Clone)]
pub enum FfiError {
    /// Null pointer was passed where non-null was expected
    #[error("null pointer error: {context}")]
    NullPointer {
        /// Description of where the null pointer was encountered
        context: String,
    },

    /// Pointer was used after ownership was transferred
    #[error("use after transfer: pointer used after ownership was transferred to foreign code")]
    UseAfterTransfer {
        /// Optional identifier for the pointer
        pointer_id: Option<String>,
    },

    /// Attempted to free memory that was borrowed
    #[error("invalid free: attempted to free borrowed memory")]
    InvalidFree {
        /// Description of the memory
        context: String,
    },

    /// String contains interior NUL bytes
    #[error("string contains interior NUL byte at position {position}")]
    InteriorNul {
        /// Position of the NUL byte
        position: usize,
    },

    /// String is not valid UTF-8
    #[error("string is not valid UTF-8: {message}")]
    InvalidUtf8 {
        /// Error message
        message: String,
    },

    /// Type size mismatch between Aria and C
    #[error("type size mismatch: expected {expected} bytes, got {actual} bytes for type {type_name}")]
    TypeSizeMismatch {
        /// Name of the type
        type_name: String,
        /// Expected size
        expected: usize,
        /// Actual size
        actual: usize,
    },

    /// Type alignment mismatch
    #[error("type alignment mismatch: expected alignment {expected}, got {actual} for type {type_name}")]
    TypeAlignmentMismatch {
        /// Name of the type
        type_name: String,
        /// Expected alignment
        expected: usize,
        /// Actual alignment
        actual: usize,
    },

    /// ABI mismatch (calling convention, etc.)
    #[error("ABI mismatch: {message}")]
    AbiMismatch {
        /// Description of the mismatch
        message: String,
    },

    /// Buffer overflow - operation would exceed buffer bounds
    #[error("buffer overflow: attempted to access {attempted} bytes, but buffer only has {available} bytes")]
    BufferOverflow {
        /// Number of bytes attempted to access
        attempted: usize,
        /// Number of bytes available
        available: usize,
    },

    /// Buffer underflow - not enough data
    #[error("buffer underflow: needed {needed} bytes, but only {available} bytes available")]
    BufferUnderflow {
        /// Number of bytes needed
        needed: usize,
        /// Number of bytes available
        available: usize,
    },

    /// Invalid pointer alignment
    #[error("invalid alignment: pointer {pointer:p} is not aligned to {required} bytes")]
    InvalidAlignment {
        /// The misaligned pointer
        pointer: usize,
        /// Required alignment
        required: usize,
    },

    /// Ownership annotation violation
    #[error("ownership violation: {message}")]
    OwnershipViolation {
        /// Description of the violation
        message: String,
    },

    /// Panic attempted to cross FFI boundary
    #[error("panic at FFI boundary: {message}")]
    PanicAtBoundary {
        /// Panic message
        message: String,
    },

    /// C library returned an error code
    #[error("C error code {code}: {message}")]
    CError {
        /// The error code
        code: i32,
        /// Optional message
        message: String,
    },

    /// Memory allocation failed
    #[error("memory allocation failed: requested {size} bytes")]
    AllocationFailed {
        /// Size requested
        size: usize,
    },

    /// Function not found in dynamic library
    #[error("function not found: {function_name}")]
    FunctionNotFound {
        /// Name of the function
        function_name: String,
    },

    /// Symbol not found in dynamic library
    #[error("symbol not found: {symbol_name}")]
    SymbolNotFound {
        /// Name of the symbol
        symbol_name: String,
    },

    /// Library load failed
    #[error("failed to load library: {library_path}: {reason}")]
    LibraryLoadFailed {
        /// Path to the library
        library_path: String,
        /// Reason for failure
        reason: String,
    },

    /// Custom error with message
    #[error("{0}")]
    Custom(String),
}

impl FfiError {
    /// Create a null pointer error with context
    pub fn null_pointer(context: impl Into<String>) -> Self {
        FfiError::NullPointer {
            context: context.into(),
        }
    }

    /// Create a use-after-transfer error
    pub fn use_after_transfer(pointer_id: Option<String>) -> Self {
        FfiError::UseAfterTransfer { pointer_id }
    }

    /// Create an invalid free error
    pub fn invalid_free(context: impl Into<String>) -> Self {
        FfiError::InvalidFree {
            context: context.into(),
        }
    }

    /// Create a type size mismatch error
    pub fn type_size_mismatch(type_name: impl Into<String>, expected: usize, actual: usize) -> Self {
        FfiError::TypeSizeMismatch {
            type_name: type_name.into(),
            expected,
            actual,
        }
    }

    /// Create a buffer overflow error
    pub fn buffer_overflow(attempted: usize, available: usize) -> Self {
        FfiError::BufferOverflow {
            attempted,
            available,
        }
    }

    /// Create a C error from an error code
    pub fn from_c_error(code: i32, message: impl Into<String>) -> Self {
        FfiError::CError {
            code,
            message: message.into(),
        }
    }

    /// Create an allocation failed error
    pub fn allocation_failed(size: usize) -> Self {
        FfiError::AllocationFailed { size }
    }

    /// Create a custom error
    pub fn custom(message: impl Into<String>) -> Self {
        FfiError::Custom(message.into())
    }
}

impl From<NulError> for FfiError {
    fn from(err: NulError) -> Self {
        FfiError::InteriorNul {
            position: err.nul_position(),
        }
    }
}

impl From<std::str::Utf8Error> for FfiError {
    fn from(err: std::str::Utf8Error) -> Self {
        FfiError::InvalidUtf8 {
            message: err.to_string(),
        }
    }
}

impl From<std::string::FromUtf8Error> for FfiError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        FfiError::InvalidUtf8 {
            message: err.to_string(),
        }
    }
}

// ============================================================================
// Error Code Constants
// ============================================================================

/// Standard FFI error codes for C interop
pub mod error_codes {
    /// Success
    pub const FFI_SUCCESS: i32 = 0;
    /// Generic error
    pub const FFI_ERROR: i32 = -1;
    /// Null pointer
    pub const FFI_NULL_POINTER: i32 = -2;
    /// Invalid argument
    pub const FFI_INVALID_ARGUMENT: i32 = -3;
    /// Buffer overflow
    pub const FFI_BUFFER_OVERFLOW: i32 = -4;
    /// Out of memory
    pub const FFI_OUT_OF_MEMORY: i32 = -5;
    /// Type mismatch
    pub const FFI_TYPE_MISMATCH: i32 = -6;
    /// Ownership violation
    pub const FFI_OWNERSHIP_VIOLATION: i32 = -7;
    /// Panic caught
    pub const FFI_PANIC: i32 = -8;
    /// Not implemented
    pub const FFI_NOT_IMPLEMENTED: i32 = -9;
}

/// Convert an FfiError to an error code for C interop
impl FfiError {
    /// Get the error code for this error
    pub fn to_error_code(&self) -> i32 {
        use error_codes::*;
        match self {
            FfiError::NullPointer { .. } => FFI_NULL_POINTER,
            FfiError::UseAfterTransfer { .. } => FFI_OWNERSHIP_VIOLATION,
            FfiError::InvalidFree { .. } => FFI_OWNERSHIP_VIOLATION,
            FfiError::InteriorNul { .. } => FFI_INVALID_ARGUMENT,
            FfiError::InvalidUtf8 { .. } => FFI_INVALID_ARGUMENT,
            FfiError::TypeSizeMismatch { .. } => FFI_TYPE_MISMATCH,
            FfiError::TypeAlignmentMismatch { .. } => FFI_TYPE_MISMATCH,
            FfiError::AbiMismatch { .. } => FFI_TYPE_MISMATCH,
            FfiError::BufferOverflow { .. } => FFI_BUFFER_OVERFLOW,
            FfiError::BufferUnderflow { .. } => FFI_BUFFER_OVERFLOW,
            FfiError::InvalidAlignment { .. } => FFI_INVALID_ARGUMENT,
            FfiError::OwnershipViolation { .. } => FFI_OWNERSHIP_VIOLATION,
            FfiError::PanicAtBoundary { .. } => FFI_PANIC,
            FfiError::CError { code, .. } => *code,
            FfiError::AllocationFailed { .. } => FFI_OUT_OF_MEMORY,
            FfiError::FunctionNotFound { .. } => FFI_ERROR,
            FfiError::SymbolNotFound { .. } => FFI_ERROR,
            FfiError::LibraryLoadFailed { .. } => FFI_ERROR,
            FfiError::Custom(_) => FFI_ERROR,
        }
    }
}

// ============================================================================
// Source Location for Error Reporting
// ============================================================================

/// Source location for FFI error reporting
///
/// This helps generate better error messages like those in ARIA-PD-010 Section 6.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// File path
    pub file: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// FFI error with source location for better diagnostics
#[derive(Debug, Clone)]
pub struct FfiDiagnostic {
    /// The error
    pub error: FfiError,
    /// Where the error occurred
    pub location: Option<SourceLocation>,
    /// Additional notes (like the "note:" in compiler output)
    pub notes: Vec<String>,
    /// Help suggestions (like the "help:" in compiler output)
    pub help: Option<String>,
}

impl FfiDiagnostic {
    /// Create a new diagnostic
    pub fn new(error: FfiError) -> Self {
        Self {
            error,
            location: None,
            notes: Vec::new(),
            help: None,
        }
    }

    /// Add a source location
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
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

impl fmt::Display for FfiDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Error line
        if let Some(ref loc) = self.location {
            writeln!(f, "error: {}", self.error)?;
            writeln!(f, "  --> {}", loc)?;
        } else {
            writeln!(f, "error: {}", self.error)?;
        }

        // Notes
        for note in &self.notes {
            writeln!(f, "note: {}", note)?;
        }

        // Help
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
    fn test_null_pointer_error() {
        let err = FfiError::null_pointer("function argument 'ptr'");
        assert!(matches!(err, FfiError::NullPointer { .. }));
    }

    #[test]
    fn test_error_code_conversion() {
        let err = FfiError::null_pointer("test");
        assert_eq!(err.to_error_code(), error_codes::FFI_NULL_POINTER);
    }

    #[test]
    fn test_from_nul_error() {
        let result = std::ffi::CString::new("hello\0world");
        assert!(result.is_err());
        let ffi_err: FfiError = result.unwrap_err().into();
        assert!(matches!(ffi_err, FfiError::InteriorNul { position: 5 }));
    }

    #[test]
    fn test_diagnostic_formatting() {
        let diag = FfiDiagnostic::new(FfiError::OwnershipViolation {
            message: "pointer used after transfer".to_string(),
        })
        .with_location(SourceLocation::new("src/lib.aria", 15, 5))
        .with_note("ownership transferred here")
        .with_help("if you need to keep using the pointer, use ptr.clone()");

        let output = diag.to_string();
        assert!(output.contains("error:"));
        assert!(output.contains("src/lib.aria:15:5"));
        assert!(output.contains("note:"));
        assert!(output.contains("help:"));
    }
}
