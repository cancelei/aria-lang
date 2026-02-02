//! Type mapping from MIR types to Cranelift types.
//!
//! This module handles the conversion between Aria's type system
//! and Cranelift's low-level type representation.

#![allow(dead_code)]

use aria_mir::MirType;
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::Type as ClifType;
use cranelift_codegen::isa::TargetIsa;

/// Convert a MIR type to a Cranelift type
pub fn mir_type_to_clif(ty: &MirType, isa: &dyn TargetIsa) -> ClifType {
    let ptr_type = isa.pointer_type();

    match ty {
        // Unit type - represented as nothing (or i8 for returns)
        MirType::Unit => types::I8,

        // Boolean - use i64 for compatibility with switch statements
        MirType::Bool => types::I64,

        // Integer types
        MirType::Int => types::I64,      // Default integer is 64-bit
        MirType::Int8 => types::I8,
        MirType::Int16 => types::I16,
        MirType::Int32 => types::I32,
        MirType::Int64 => types::I64,
        MirType::UInt => types::I64,     // Unsigned uses same repr
        MirType::UInt8 => types::I8,
        MirType::UInt16 => types::I16,
        MirType::UInt32 => types::I32,
        MirType::UInt64 => types::I64,

        // Float types
        MirType::Float => types::F64,    // Default float is 64-bit
        MirType::Float32 => types::F32,
        MirType::Float64 => types::F64,

        // Char - 32-bit Unicode codepoint
        MirType::Char => types::I32,

        // String - pointer to string struct
        MirType::String => ptr_type,

        // Compound types - all pointers
        MirType::Array(_) => ptr_type,
        MirType::Tuple(_) => ptr_type,
        MirType::Map(_, _) => ptr_type,
        MirType::Optional(_) => ptr_type,
        MirType::Result(_, _) => ptr_type,

        // References - pointers
        MirType::Ref(_) => ptr_type,
        MirType::RefMut(_) => ptr_type,

        // Named types - pointers to heap objects
        MirType::Struct(_) => ptr_type,
        MirType::Enum(_) => ptr_type,

        // Function types - pointers
        MirType::FnPtr { .. } => ptr_type,
        MirType::Closure { .. } => ptr_type,

        // Never type - unreachable, but use I64 as placeholder
        MirType::Never => types::I64,

        // Type inference constructs - should be resolved before codegen
        // Fall back to pointer type as these represent polymorphic values
        MirType::TypeVar(_) => ptr_type,
        MirType::TypeParam(_) => ptr_type,
        MirType::Generic { .. } => ptr_type,
    }
}

/// Check if a type is passed by value (fits in a register)
pub fn is_value_type(ty: &MirType) -> bool {
    matches!(
        ty,
        MirType::Unit
            | MirType::Bool
            | MirType::Int
            | MirType::Int8
            | MirType::Int16
            | MirType::Int32
            | MirType::Int64
            | MirType::UInt
            | MirType::UInt8
            | MirType::UInt16
            | MirType::UInt32
            | MirType::UInt64
            | MirType::Float
            | MirType::Float32
            | MirType::Float64
            | MirType::Char
    )
}

/// Check if a type needs heap allocation
pub fn needs_heap_allocation(ty: &MirType) -> bool {
    matches!(
        ty,
        MirType::String
            | MirType::Array(_)
            | MirType::Tuple(_)
            | MirType::Map(_, _)
            | MirType::Struct(_)
            | MirType::Enum(_)
            | MirType::Closure { .. }
    )
}

/// Get the size of a type in bytes (for stack allocation)
pub fn type_size(ty: &MirType, ptr_size: u32) -> u32 {
    match ty {
        MirType::Unit => 0,
        MirType::Bool => 8, // Represented as i64 for compatibility
        MirType::Int8 | MirType::UInt8 => 1,
        MirType::Int16 | MirType::UInt16 => 2,
        MirType::Int32 | MirType::UInt32 | MirType::Char => 4,
        MirType::Int | MirType::UInt | MirType::Int64 | MirType::UInt64 => 8,
        MirType::Float32 => 4,
        MirType::Float | MirType::Float64 => 8,
        // Everything else is a pointer
        _ => ptr_size,
    }
}

/// Get the alignment of a type in bytes
pub fn type_align(ty: &MirType, ptr_size: u32) -> u32 {
    match ty {
        MirType::Unit => 1,
        MirType::Bool => 1,
        MirType::Int8 | MirType::UInt8 => 1,
        MirType::Int16 | MirType::UInt16 => 2,
        MirType::Int32 | MirType::UInt32 | MirType::Char | MirType::Float32 => 4,
        MirType::Int | MirType::UInt | MirType::Int64 | MirType::UInt64 |
        MirType::Float | MirType::Float64 => 8,
        // Pointers align to pointer size
        _ => ptr_size,
    }
}

/// Check if a MIR type is a floating point type
pub fn is_float_type(ty: &MirType) -> bool {
    matches!(ty, MirType::Float | MirType::Float32 | MirType::Float64)
}

/// Check if a MIR type is a signed integer
pub fn is_signed_int(ty: &MirType) -> bool {
    matches!(
        ty,
        MirType::Int | MirType::Int8 | MirType::Int16 | MirType::Int32 | MirType::Int64
    )
}

/// Check if a MIR type is an unsigned integer
pub fn is_unsigned_int(ty: &MirType) -> bool {
    matches!(
        ty,
        MirType::UInt | MirType::UInt8 | MirType::UInt16 | MirType::UInt32 | MirType::UInt64
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_value_type() {
        assert!(is_value_type(&MirType::Int));
        assert!(is_value_type(&MirType::Bool));
        assert!(is_value_type(&MirType::Float64));
        assert!(!is_value_type(&MirType::String));
        assert!(!is_value_type(&MirType::Array(Box::new(MirType::Int))));
    }

    #[test]
    fn test_needs_heap_allocation() {
        assert!(needs_heap_allocation(&MirType::String));
        assert!(needs_heap_allocation(&MirType::Array(Box::new(MirType::Int))));
        assert!(!needs_heap_allocation(&MirType::Int));
        assert!(!needs_heap_allocation(&MirType::Bool));
    }

    #[test]
    fn test_type_size() {
        assert_eq!(type_size(&MirType::Bool, 8), 8);  // Bool uses i64 for switch compatibility
        assert_eq!(type_size(&MirType::Int32, 8), 4);
        assert_eq!(type_size(&MirType::Int64, 8), 8);
        assert_eq!(type_size(&MirType::String, 8), 8); // Pointer
    }

    #[test]
    fn test_is_float_type() {
        assert!(is_float_type(&MirType::Float));
        assert!(is_float_type(&MirType::Float32));
        assert!(is_float_type(&MirType::Float64));
        assert!(!is_float_type(&MirType::Int));
    }
}
