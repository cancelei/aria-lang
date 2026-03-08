//! Core type definitions for Aria.
//!
//! Contains the primary `Type` enum, effect types, constant values,
//! and the static primitive type lookup table.

use rustc_hash::FxHashMap;
use std::sync::OnceLock;

/// Represents a compile-time constant value.
///
/// Const values are computed at compile time and can be used in contexts
/// that require compile-time evaluation, such as array sizes, const declarations,
/// and constant folding optimizations.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    /// Integer constant (stored as i128 to handle all integer types)
    Int(i128),
    /// Unsigned integer constant (stored as u128 to handle all unsigned types)
    UInt(u128),
    /// Floating-point constant (stored as f64 for maximum precision)
    Float(f64),
    /// Boolean constant
    Bool(bool),
    /// Character constant
    Char(char),
    /// String constant
    String(String),
    /// Unit value
    Unit,
    /// Array of constant values
    Array(Vec<ConstValue>),
    /// Tuple of constant values
    Tuple(Vec<ConstValue>),
}

impl ConstValue {
    /// Get the type of this constant value
    pub fn ty(&self) -> Type {
        match self {
            ConstValue::Int(_) => Type::Int,
            ConstValue::UInt(_) => Type::UInt,
            ConstValue::Float(_) => Type::Float,
            ConstValue::Bool(_) => Type::Bool,
            ConstValue::Char(_) => Type::Char,
            ConstValue::String(_) => Type::String,
            ConstValue::Unit => Type::Unit,
            ConstValue::Array(elems) => {
                if elems.is_empty() {
                    Type::Array(Box::new(Type::Any))
                } else {
                    Type::Array(Box::new(elems[0].ty()))
                }
            }
            ConstValue::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|e| e.ty()).collect())
            }
        }
    }

    /// Try to convert this value to an i128
    pub fn as_int(&self) -> Option<i128> {
        match self {
            ConstValue::Int(n) => Some(*n),
            ConstValue::UInt(n) => i128::try_from(*n).ok(),
            _ => None,
        }
    }

    /// Try to convert this value to a u128
    pub fn as_uint(&self) -> Option<u128> {
        match self {
            ConstValue::UInt(n) => Some(*n),
            ConstValue::Int(n) if *n >= 0 => Some(*n as u128),
            _ => None,
        }
    }

    /// Try to convert this value to an f64
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConstValue::Float(f) => Some(*f),
            ConstValue::Int(n) => Some(*n as f64),
            ConstValue::UInt(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Try to convert this value to a bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConstValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to convert this value to a usize (for array sizes)
    pub fn as_usize(&self) -> Option<usize> {
        match self {
            ConstValue::Int(n) if *n >= 0 => usize::try_from(*n).ok(),
            ConstValue::UInt(n) => usize::try_from(*n).ok(),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConstValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstValue::Int(n) => write!(f, "{}", n),
            ConstValue::UInt(n) => write!(f, "{}", n),
            ConstValue::Float(n) => write!(f, "{}", n),
            ConstValue::Bool(b) => write!(f, "{}", b),
            ConstValue::Char(c) => write!(f, "'{}'", c),
            ConstValue::String(s) => write!(f, "\"{}\"", s),
            ConstValue::Unit => write!(f, "()"),
            ConstValue::Array(elems) => {
                write!(f, "[")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            ConstValue::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
        }
    }
}

// ============================================================================
// Static Primitive Type Lookup (Performance Optimization)
// ============================================================================

/// Get the static lookup table for primitive type names.
/// Using OnceLock for lazy initialization ensures this is only computed once.
pub(crate) fn primitive_type_lookup() -> &'static FxHashMap<&'static str, Type> {
    static PRIMITIVES: OnceLock<FxHashMap<&'static str, Type>> = OnceLock::new();
    PRIMITIVES.get_or_init(|| {
        let mut map = FxHashMap::default();
        map.insert("Int", Type::Int);
        map.insert("Int8", Type::Int8);
        map.insert("Int16", Type::Int16);
        map.insert("Int32", Type::Int32);
        map.insert("Int64", Type::Int64);
        map.insert("Int128", Type::Int128);
        map.insert("UInt", Type::UInt);
        map.insert("UInt8", Type::UInt8);
        map.insert("UInt16", Type::UInt16);
        map.insert("UInt32", Type::UInt32);
        map.insert("UInt64", Type::UInt64);
        map.insert("UInt128", Type::UInt128);
        map.insert("Float", Type::Float);
        map.insert("Float32", Type::Float32);
        map.insert("Float64", Type::Float64);
        map.insert("Bool", Type::Bool);
        map.insert("Char", Type::Char);
        map.insert("String", Type::String);
        map.insert("Bytes", Type::Bytes);
        map.insert("Unit", Type::Unit);
        map.insert("Never", Type::Never);
        map
    })
}

/// Type variable ID for inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(pub u32);

/// Effect row variable ID for effect inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectRowVar(pub u32);
// ============================================================================
// Effect System Types
// ============================================================================

/// An effect represents a computational side effect that a function may perform.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// IO effect - file system, network operations
    IO,
    /// Console effect - print/read console operations
    Console,
    /// Async effect - async/await operations
    Async,
    /// Mutation effect - mutable state operations
    Mutation,
    /// Exception effect with error type
    Exception(Box<Type>),
    /// State effect with state type
    State(Box<Type>),
    /// Reader effect with environment type
    Reader(Box<Type>),
    /// Writer effect with output type
    Writer(Box<Type>),
    /// Custom user-defined effect
    Custom {
        name: String,
        type_args: Vec<Type>,
    },
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effect::IO => write!(f, "IO"),
            Effect::Console => write!(f, "Console"),
            Effect::Async => write!(f, "Async"),
            Effect::Mutation => write!(f, "Mutation"),
            Effect::Exception(ty) => write!(f, "Exception[{}]", ty),
            Effect::State(ty) => write!(f, "State[{}]", ty),
            Effect::Reader(ty) => write!(f, "Reader[{}]", ty),
            Effect::Writer(ty) => write!(f, "Writer[{}]", ty),
            Effect::Custom { name, type_args } => {
                write!(f, "{}", name)?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
        }
    }
}

/// An effect row represents a set of effects, optionally open (polymorphic).
#[derive(Debug, Clone, PartialEq)]
pub struct EffectRow {
    /// The concrete effects in this row
    pub effects: Vec<Effect>,
    /// Optional row variable for polymorphism (open row)
    pub row_var: Option<EffectRowVar>,
}

impl EffectRow {
    /// Create an empty (pure) effect row
    pub fn pure() -> Self {
        Self {
            effects: Vec::new(),
            row_var: None,
        }
    }

    /// Create a closed effect row with the given effects
    pub fn closed(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            row_var: None,
        }
    }

    /// Create an open effect row with a row variable
    pub fn open(effects: Vec<Effect>, row_var: EffectRowVar) -> Self {
        Self {
            effects,
            row_var: Some(row_var),
        }
    }

    /// Create an effect row with just a row variable (fully polymorphic)
    pub fn var(row_var: EffectRowVar) -> Self {
        Self {
            effects: Vec::new(),
            row_var: Some(row_var),
        }
    }

    /// Check if this is a pure (empty, closed) effect row
    pub fn is_pure(&self) -> bool {
        self.effects.is_empty() && self.row_var.is_none()
    }

    /// Check if this row is closed (no row variable)
    pub fn is_closed(&self) -> bool {
        self.row_var.is_none()
    }

    /// Check if this row contains a specific effect
    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    /// Add an effect to this row (returns new row, preserves openness)
    pub fn with_effect(mut self, effect: Effect) -> Self {
        if !self.effects.contains(&effect) {
            self.effects.push(effect);
        }
        self
    }

    /// Merge two effect rows (union of effects)
    pub fn merge(&self, other: &EffectRow) -> EffectRow {
        let mut effects = self.effects.clone();
        for effect in &other.effects {
            if !effects.contains(effect) {
                effects.push(effect.clone());
            }
        }
        let row_var = self.row_var.or(other.row_var);
        EffectRow { effects, row_var }
    }

    /// Remove an effect from this row (for handler elimination)
    pub fn without_effect(&self, effect: &Effect) -> EffectRow {
        EffectRow {
            effects: self.effects.iter()
                .filter(|e| *e != effect)
                .cloned()
                .collect(),
            row_var: self.row_var,
        }
    }

    /// Check if this row is a subset of another (subtyping)
    pub fn is_subset_of(&self, other: &EffectRow) -> bool {
        if other.row_var.is_some() {
            return self.effects.iter().all(|e| other.effects.contains(e) || true);
        }
        self.effects.iter().all(|e| other.effects.contains(e))
    }
}

impl Default for EffectRow {
    fn default() -> Self {
        Self::pure()
    }
}

impl std::fmt::Display for EffectRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "!{{")?;
        for (i, effect) in self.effects.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", effect)?;
        }
        if let Some(var) = self.row_var {
            if !self.effects.is_empty() {
                write!(f, " | ")?;
            }
            write!(f, "e{}", var.0)?;
        }
        write!(f, "}}")
    }
}


/// Aria types
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Primitive types
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    UInt,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    Float,
    Float32,
    Float64,
    Bool,
    Char,
    String,
    Bytes,
    Unit,
    Never,

    // Compound types
    Array(Box<Type>),
    FixedArray(Box<Type>, usize),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Optional(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Reference { mutable: bool, inner: Box<Type> },
    Function { params: Vec<Type>, return_type: Box<Type> },

    /// Channel type for inter-task communication.
    ///
    /// Channels are typed, bounded message queues used for communication
    /// between concurrent tasks. A Channel[T] can send and receive values
    /// of type T.
    ///
    /// Properties:
    /// - Transfer: if T is Transfer (can send channel to another task)
    /// - Sharable: always (channels are designed for sharing between tasks)
    Channel(Box<Type>),

    /// Task type for concurrent computation.
    ///
    /// A Task[T] represents a spawned concurrent computation that will
    /// eventually produce a value of type T. Tasks can be awaited to
    /// retrieve their result.
    ///
    /// Properties:
    /// - Transfer: if T is Transfer (task handles can be sent between tasks)
    /// - Sharable: always (task handles are designed for sharing)
    ///
    /// Created by: spawn expressions
    /// Consumed by: await expressions
    Task(Box<Type>),

    /// Function type with effects.
    ///
    /// A function that may perform effects declared in its effect row.
    /// The syntax is: `fn(params) !{effects} -> return_type`
    ///
    /// Examples:
    /// - `fn(Int) !{IO} -> String` - function that performs IO
    /// - `fn() !{} -> Int` - pure function
    /// - `fn(T) !{e} -> T` - effect-polymorphic function
    EffectfulFunction {
        params: Vec<Type>,
        effects: EffectRow,
        return_type: Box<Type>,
    },

    // Named types
    Named {
        name: String,
        type_args: Vec<Type>,
    },

    /// Effect row variable for effect inference
    EffectVar(EffectRowVar),

    // Type variable (for inference)
    Var(TypeVar),

    // Error type (for error recovery)
    Error,

    // Any type (for polymorphic builtins)
    // Matches any other type during type checking
    Any,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Primitive types
            Type::Int => write!(f, "Int"),
            Type::Int8 => write!(f, "Int8"),
            Type::Int16 => write!(f, "Int16"),
            Type::Int32 => write!(f, "Int32"),
            Type::Int64 => write!(f, "Int64"),
            Type::Int128 => write!(f, "Int128"),
            Type::UInt => write!(f, "UInt"),
            Type::UInt8 => write!(f, "UInt8"),
            Type::UInt16 => write!(f, "UInt16"),
            Type::UInt32 => write!(f, "UInt32"),
            Type::UInt64 => write!(f, "UInt64"),
            Type::UInt128 => write!(f, "UInt128"),
            Type::Float => write!(f, "Float"),
            Type::Float32 => write!(f, "Float32"),
            Type::Float64 => write!(f, "Float64"),
            Type::Bool => write!(f, "Bool"),
            Type::Char => write!(f, "Char"),
            Type::String => write!(f, "String"),
            Type::Bytes => write!(f, "Bytes"),
            Type::Unit => write!(f, "()"),
            Type::Never => write!(f, "!"),

            // Compound types
            Type::Array(elem) => write!(f, "[{}]", elem),
            Type::FixedArray(elem, size) => write!(f, "[{}; {}]", elem, size),
            Type::Map(key, val) => write!(f, "Map<{}, {}>", key, val),
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            Type::Optional(inner) => write!(f, "{}?", inner),
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Reference { mutable, inner } => {
                if *mutable {
                    write!(f, "&mut {}", inner)
                } else {
                    write!(f, "&{}", inner)
                }
            }
            Type::Function { params, return_type } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::Channel(elem) => write!(f, "Channel[{}]", elem),
            Type::Task(result) => write!(f, "Task[{}]", result),

            Type::EffectfulFunction { params, effects, return_type } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") ")?;
                if !effects.is_pure() {
                    write!(f, "{} ", effects)?;
                }
                write!(f, "-> {}", return_type)
            }

            // Named types
            Type::Named { name, type_args } => {
                write!(f, "{}", name)?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }

            // Effect row variable
            Type::EffectVar(var) => write!(f, "!e{}", var.0),

            // Type variable
            Type::Var(var) => write!(f, "?{}", var.0),

            // Error type
            Type::Error => write!(f, "<error>"),

            // Any type
            Type::Any => write!(f, "Any"),
        }
    }
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Int128
                | Type::UInt
                | Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::UInt128
                | Type::Float
                | Type::Float32
                | Type::Float64
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Int128
                | Type::UInt
                | Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::UInt128
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float | Type::Float32 | Type::Float64)
    }

    /// Check if this type implements the Transfer trait (can be safely moved between tasks).
    ///
    /// Transfer is Aria's equivalent of Rust's Send trait. Types that are Transfer can be
    /// safely transferred to another task/thread without data races.
    ///
    /// # Transfer types
    ///
    /// - All primitive types (Int, Float, Bool, Char, String, etc.)
    /// - Arrays and tuples of Transfer types
    /// - Optional and Result of Transfer types
    /// - Functions (closure captures must be checked separately)
    /// - Named types (assumed Transfer unless marked otherwise)
    ///
    /// # Non-Transfer types
    ///
    /// - Mutable references (could cause data races)
    /// - Type variables (unknown, conservatively non-Transfer)
    /// - Error types
    pub fn is_transfer(&self) -> bool {
        match self {
            // All primitives are Transfer
            Type::Int
            | Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::Int128
            | Type::UInt
            | Type::UInt8
            | Type::UInt16
            | Type::UInt32
            | Type::UInt64
            | Type::UInt128
            | Type::Float
            | Type::Float32
            | Type::Float64
            | Type::Bool
            | Type::Char
            | Type::String
            | Type::Bytes
            | Type::Unit
            | Type::Never => true,

            // Compound types are Transfer if their components are Transfer
            Type::Array(inner) => inner.is_transfer(),
            Type::FixedArray(inner, _) => inner.is_transfer(),
            Type::Map(key, value) => key.is_transfer() && value.is_transfer(),
            Type::Tuple(types) => types.iter().all(|t| t.is_transfer()),
            Type::Optional(inner) => inner.is_transfer(),
            Type::Result(ok, err) => ok.is_transfer() && err.is_transfer(),

            // Channels are Transfer if their element type is Transfer
            // (a channel handle can be sent to another task)
            Type::Channel(elem) => elem.is_transfer(),

            // Task handles are Transfer if their result type is Transfer
            Type::Task(result) => result.is_transfer(),

            // Mutable references are NOT Transfer (could cause data races)
            Type::Reference { mutable, inner } => {
                if *mutable {
                    false
                } else {
                    // Immutable references to Sharable types are Transfer
                    inner.is_sharable()
                }
            }

            // Functions are Transfer (but captures need separate checking)
            Type::Function { .. } => true,

            // Effectful functions are Transfer
            Type::EffectfulFunction { .. } => true,

            // Named types are assumed Transfer by default
            // TODO: Check against a registry of non-Transfer types
            Type::Named { .. } => true,

            // Effect row variables are Transfer (they represent effect rows, not values)
            Type::EffectVar(_) => true,

            // Type variables are unknown, conservatively non-Transfer
            Type::Var(_) => false,

            // Error types are not Transfer
            Type::Error => false,

            // Any type is considered Transfer (used for polymorphic builtins)
            Type::Any => true,
        }
    }

    /// Check if this type implements the Sharable trait (can be safely shared between tasks).
    ///
    /// Sharable is Aria's equivalent of Rust's Sync trait. Types that are Sharable can be
    /// safely accessed from multiple tasks through immutable references.
    ///
    /// # Sharable types
    ///
    /// - All primitive types (Int, Float, Bool, Char, String, etc.)
    /// - Arrays and tuples of Sharable types (with immutable access)
    /// - Immutable references to Sharable types
    ///
    /// # Non-Sharable types
    ///
    /// - Mutable references (concurrent mutation could cause data races)
    /// - Type variables (unknown, conservatively non-Sharable)
    /// - Error types
    pub fn is_sharable(&self) -> bool {
        match self {
            // All primitives are Sharable
            Type::Int
            | Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::Int128
            | Type::UInt
            | Type::UInt8
            | Type::UInt16
            | Type::UInt32
            | Type::UInt64
            | Type::UInt128
            | Type::Float
            | Type::Float32
            | Type::Float64
            | Type::Bool
            | Type::Char
            | Type::String
            | Type::Bytes
            | Type::Unit
            | Type::Never => true,

            // Compound types are Sharable if their components are Sharable
            Type::Array(inner) => inner.is_sharable(),
            Type::FixedArray(inner, _) => inner.is_sharable(),
            Type::Map(key, value) => key.is_sharable() && value.is_sharable(),
            Type::Tuple(types) => types.iter().all(|t| t.is_sharable()),
            Type::Optional(inner) => inner.is_sharable(),
            Type::Result(ok, err) => ok.is_sharable() && err.is_sharable(),

            // Channels are always Sharable - they are designed for sharing between tasks
            // (multiple tasks can hold references to the same channel)
            Type::Channel(_) => true,

            // Task handles are always Sharable - they can be awaited from any task
            Type::Task(_) => true,

            // Mutable references are NOT Sharable
            // Immutable references to Sharable types are Sharable
            Type::Reference { mutable, inner } => !mutable && inner.is_sharable(),

            // Functions are Sharable (captures need separate checking)
            Type::Function { .. } => true,

            // Effectful functions are Sharable
            Type::EffectfulFunction { .. } => true,

            // Named types are assumed Sharable by default
            // TODO: Check against a registry of non-Sharable types
            Type::Named { .. } => true,

            // Effect row variables are Sharable
            Type::EffectVar(_) => true,

            // Type variables are unknown, conservatively non-Sharable
            Type::Var(_) => false,

            // Error types are not Sharable
            Type::Error => false,

            // Any type is considered Sharable (used for polymorphic builtins)
            Type::Any => true,
        }
    }

    /// Check if this type can be safely captured by a spawned task.
    ///
    /// For a type to be capturable by spawn, it must be Transfer.
    /// This is the primary check used during spawn type checking.
    pub fn is_spawn_safe(&self) -> bool {
        self.is_transfer()
    }

    /// Check if this type implements the Copy trait (can be implicitly copied).
    ///
    /// Copy types can be duplicated by simple bitwise copying without running
    /// any user-defined code. When a Copy type is used, it is copied rather than moved.
    ///
    /// # Copy types
    ///
    /// - All numeric primitives (Int, Float, etc.)
    /// - Bool, Char, Unit, Never
    /// - References (copying the reference, not what it points to)
    /// - Tuples of Copy types
    /// - Fixed-size arrays of Copy types
    /// - Optional of Copy types
    /// - Result of Copy types (both Ok and Err must be Copy)
    ///
    /// # Non-Copy types (owned, require move semantics)
    ///
    /// - String (owns heap-allocated data)
    /// - Bytes (owns heap-allocated data)
    /// - Array (dynamic arrays own heap memory)
    /// - Map (owns heap-allocated data)
    /// - Channel (unique handle to communication endpoint)
    /// - Functions/closures (may capture owned state)
    /// - Named types (not Copy by default, require explicit @copy attribute)
    /// - Type variables (unknown, conservatively non-Copy)
    pub fn is_copy(&self) -> bool {
        match self {
            // All numeric primitives are Copy
            Type::Int
            | Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::Int128
            | Type::UInt
            | Type::UInt8
            | Type::UInt16
            | Type::UInt32
            | Type::UInt64
            | Type::UInt128
            | Type::Float
            | Type::Float32
            | Type::Float64
            | Type::Bool
            | Type::Char
            | Type::Unit
            | Type::Never => true,

            // String and Bytes own heap data - NOT Copy
            Type::String | Type::Bytes => false,

            // Dynamic arrays own heap memory - NOT Copy
            Type::Array(_) => false,

            // Fixed-size arrays are Copy if element type is Copy
            Type::FixedArray(inner, _) => inner.is_copy(),

            // Maps own heap memory - NOT Copy
            Type::Map(_, _) => false,

            // Tuples are Copy if all elements are Copy
            Type::Tuple(types) => types.iter().all(|t| t.is_copy()),

            // Optional is Copy if inner type is Copy
            Type::Optional(inner) => inner.is_copy(),

            // Result is Copy if both Ok and Err types are Copy
            Type::Result(ok, err) => ok.is_copy() && err.is_copy(),

            // References are Copy (we copy the reference itself, not the data)
            // This matches Rust's behavior where &T and &mut T are Copy
            Type::Reference { .. } => true,

            // Functions/closures may capture owned state - NOT Copy
            Type::Function { .. } => false,

            // Channels are unique handles - NOT Copy
            Type::Channel(_) => false,

            // Task handles are NOT Copy (they represent ownership of a computation)
            Type::Task(_) => false,

            // Effectful functions may capture owned state - NOT Copy
            Type::EffectfulFunction { .. } => false,

            // Named types are NOT Copy by default
            // Types can opt-in to Copy with @copy attribute
            // TODO: Check against a registry of Copy types
            Type::Named { .. } => false,

            // Effect row variables are not Copy (they're type-level constructs)
            Type::EffectVar(_) => false,

            // Type variables are unknown, conservatively non-Copy
            Type::Var(_) => false,

            // Error types are not Copy
            Type::Error => false,

            // Any type is NOT Copy (be conservative for polymorphic contexts)
            Type::Any => false,
        }
    }
}
