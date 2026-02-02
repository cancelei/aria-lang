//! Runtime values for the Aria interpreter.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use aria_ast::FunctionBody;
use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::Environment;

/// Runtime values in the Aria interpreter.
#[derive(Debug, Clone)]
pub enum Value {
    /// Nil value (absence of value)
    Nil,

    /// Boolean value
    Bool(bool),

    /// 64-bit signed integer
    Int(i64),

    /// 64-bit floating point number
    Float(f64),

    /// String value
    String(SmolStr),

    /// Array/list of values
    Array(Rc<RefCell<Vec<Value>>>),

    /// Map/dictionary with string keys
    Map(Rc<RefCell<IndexMap<SmolStr, Value>>>),

    /// Tuple (immutable fixed-size collection)
    Tuple(Rc<Vec<Value>>),

    /// Range value (for iteration)
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },

    /// User-defined function
    Function(Rc<AriaFunction>),

    /// Built-in function
    BuiltinFunction(BuiltinFn),

    /// Struct instance
    Struct(Rc<RefCell<StructInstance>>),

    /// Enum variant
    EnumVariant {
        enum_name: SmolStr,
        variant_name: SmolStr,
        fields: Option<Vec<Value>>,
    },
}

/// A user-defined Aria function.
#[derive(Debug, Clone)]
pub struct AriaFunction {
    pub name: SmolStr,
    pub params: Vec<SmolStr>,
    pub body: FunctionBody,
    pub closure: Rc<RefCell<Environment>>,
}

/// A built-in function implemented in Rust.
#[derive(Clone)]
pub struct BuiltinFn {
    pub name: SmolStr,
    pub arity: Option<usize>, // None means variadic
    pub func: fn(Vec<Value>) -> crate::Result<Value>,
}

impl fmt::Debug for BuiltinFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BuiltinFn({})", self.name)
    }
}

/// A struct instance with its fields.
#[derive(Debug, Clone)]
pub struct StructInstance {
    pub name: SmolStr,
    pub fields: IndexMap<SmolStr, Value>,
}

impl Value {
    /// Get the type name of this value.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "Nil",
            Value::Bool(_) => "Bool",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::Array(_) => "Array",
            Value::Map(_) => "Map",
            Value::Tuple(_) => "Tuple",
            Value::Range { .. } => "Range",
            Value::Function(_) => "Function",
            Value::BuiltinFunction(_) => "Function",
            Value::Struct(_) => "Struct",
            Value::EnumVariant { .. } => "EnumVariant",
        }
    }

    /// Check if this value is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.borrow().is_empty(),
            Value::Map(map) => !map.borrow().is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Range { start, end, .. } => start != end,
            Value::Function(_) | Value::BuiltinFunction(_) => true,
            Value::Struct(_) => true,
            Value::EnumVariant { .. } => true,
        }
    }

    /// Try to convert to bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to convert to i64
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to convert to f64
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(n) => Some(*n),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Try to convert to string
    pub fn as_string(&self) -> Option<&SmolStr> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Array(arr) => {
                let arr = arr.borrow();
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Map(map) => {
                let map = map.borrow();
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Tuple(t) => {
                write!(f, "(")?;
                for (i, v) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                if t.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            Value::Range {
                start,
                end,
                inclusive,
            } => {
                if *inclusive {
                    write!(f, "{}..={}", start, end)
                } else {
                    write!(f, "{}..{}", start, end)
                }
            }
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::BuiltinFunction(func) => write!(f, "<builtin {}>", func.name),
            Value::Struct(s) => {
                let s = s.borrow();
                write!(f, "{} {{ ", s.name)?;
                for (i, (k, v)) in s.fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            Value::EnumVariant {
                enum_name,
                variant_name,
                fields,
            } => {
                write!(f, "{}::{}", enum_name, variant_name)?;
                if let Some(fields) = fields {
                    write!(f, "(")?;
                    for (i, v) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => *a.borrow() == *b.borrow(),
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Range { start: s1, end: e1, inclusive: i1 },
             Value::Range { start: s2, end: e2, inclusive: i2 }) => {
                s1 == s2 && e1 == e2 && i1 == i2
            }
            (Value::EnumVariant { enum_name: e1, variant_name: v1, fields: f1 },
             Value::EnumVariant { enum_name: e2, variant_name: v2, fields: f2 }) => {
                e1 == e2 && v1 == v2 && f1 == f2
            }
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Default for Value {
    fn default() -> Self {
        Value::Nil
    }
}
