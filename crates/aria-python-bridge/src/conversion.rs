//! Type Conversion Traits
//!
//! This module provides traits for converting between Aria and Python types.
//! Based on ARIA-M10 Python Interop milestone.
//!
//! ## Traits
//!
//! - `ToPython`: Convert Aria types to Python values
//! - `FromPython`: Convert Python values to Aria types
//!
//! ## Design Notes
//!
//! The conversion traits are designed to:
//! 1. Be zero-copy where possible (especially for arrays)
//! 2. Provide clear error messages for type mismatches
//! 3. Support both fallible and infallible conversions

use crate::error::{PyBridgeError, PyBridgeResult};
use crate::py_types::{PyDict, PyList, PyValue};
use smol_str::SmolStr;
use std::collections::HashMap;

// ============================================================================
// ToPython Trait - Convert Aria types to Python
// ============================================================================

/// Trait for converting Aria types to Python values.
///
/// This trait is the primary way to convert Aria values for use in Python.
/// Implementations should be efficient and zero-copy where possible.
///
/// # Example
///
/// ```ignore
/// use aria_python_bridge::ToPython;
///
/// let aria_int: i64 = 42;
/// let py_val = aria_int.to_python();
/// assert!(matches!(py_val, PyValue::Int(42)));
/// ```
pub trait ToPython {
    /// Convert this value to a Python value.
    fn to_python(&self) -> PyValue;
}

/// Trait for fallible conversion to Python values.
///
/// Use this when conversion might fail (e.g., overflow, encoding errors).
pub trait TryToPython {
    /// The error type for conversion failures
    type Error;

    /// Try to convert this value to a Python value.
    fn try_to_python(&self) -> Result<PyValue, Self::Error>;
}

// ============================================================================
// ToPython Implementations for Primitive Types
// ============================================================================

impl ToPython for () {
    fn to_python(&self) -> PyValue {
        PyValue::None
    }
}

impl ToPython for bool {
    fn to_python(&self) -> PyValue {
        PyValue::Bool(*self)
    }
}

impl ToPython for i8 {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self as i64)
    }
}

impl ToPython for i16 {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self as i64)
    }
}

impl ToPython for i32 {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self as i64)
    }
}

impl ToPython for i64 {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self)
    }
}

impl ToPython for u8 {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self as i64)
    }
}

impl ToPython for u16 {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self as i64)
    }
}

impl ToPython for u32 {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self as i64)
    }
}

impl ToPython for u64 {
    fn to_python(&self) -> PyValue {
        // Note: May lose precision for very large u64 values
        PyValue::Int(*self as i64)
    }
}

impl ToPython for isize {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self as i64)
    }
}

impl ToPython for usize {
    fn to_python(&self) -> PyValue {
        PyValue::Int(*self as i64)
    }
}

impl ToPython for f32 {
    fn to_python(&self) -> PyValue {
        PyValue::Float(*self as f64)
    }
}

impl ToPython for f64 {
    fn to_python(&self) -> PyValue {
        PyValue::Float(*self)
    }
}

impl ToPython for str {
    fn to_python(&self) -> PyValue {
        PyValue::String(SmolStr::new(self))
    }
}

impl ToPython for String {
    fn to_python(&self) -> PyValue {
        PyValue::String(SmolStr::new(self))
    }
}

impl ToPython for SmolStr {
    fn to_python(&self) -> PyValue {
        PyValue::String(self.clone())
    }
}

impl<T: ToPython> ToPython for Option<T> {
    fn to_python(&self) -> PyValue {
        match self {
            Some(v) => v.to_python(),
            None => PyValue::None,
        }
    }
}

impl<T: ToPython> ToPython for Vec<T> {
    fn to_python(&self) -> PyValue {
        let items: Vec<PyValue> = self.iter().map(|x| x.to_python()).collect();
        PyValue::List(PyList::from_vec(items))
    }
}

impl<T: ToPython> ToPython for [T] {
    fn to_python(&self) -> PyValue {
        let items: Vec<PyValue> = self.iter().map(|x| x.to_python()).collect();
        PyValue::List(PyList::from_vec(items))
    }
}

impl<K: AsRef<str>, V: ToPython> ToPython for HashMap<K, V> {
    fn to_python(&self) -> PyValue {
        let dict = PyDict::new();
        for (k, v) in self.iter() {
            dict.set(k.as_ref(), v.to_python());
        }
        PyValue::Dict(dict)
    }
}

// Tuple implementations
impl ToPython for (PyValue,) {
    fn to_python(&self) -> PyValue {
        PyValue::Tuple(vec![self.0.clone()])
    }
}

impl ToPython for (PyValue, PyValue) {
    fn to_python(&self) -> PyValue {
        PyValue::Tuple(vec![self.0.clone(), self.1.clone()])
    }
}

impl ToPython for (PyValue, PyValue, PyValue) {
    fn to_python(&self) -> PyValue {
        PyValue::Tuple(vec![self.0.clone(), self.1.clone(), self.2.clone()])
    }
}

// ============================================================================
// FromPython Trait - Convert Python values to Aria types
// ============================================================================

/// Trait for converting Python values to Aria types.
///
/// This trait provides fallible conversion from Python to Aria types,
/// with detailed error messages for type mismatches.
pub trait FromPython: Sized {
    /// Try to convert a Python value to this type.
    fn from_python(value: &PyValue) -> PyBridgeResult<Self>;
}

// ============================================================================
// FromPython Implementations for Primitive Types
// ============================================================================

impl FromPython for () {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::None => Ok(()),
            _ => Err(PyBridgeError::type_mismatch("None", value.type_name())),
        }
    }
}

impl FromPython for bool {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::Bool(b) => Ok(*b),
            PyValue::Int(n) => Ok(*n != 0),
            _ => Err(PyBridgeError::type_mismatch("bool", value.type_name())),
        }
    }
}

impl FromPython for i64 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::Int(n) => Ok(*n),
            PyValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
            PyValue::Float(f) if f.fract() == 0.0 => {
                if *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                    Ok(*f as i64)
                } else {
                    Err(PyBridgeError::numeric_overflow(f.to_string(), "i64"))
                }
            }
            _ => Err(PyBridgeError::type_mismatch("int", value.type_name())),
        }
    }
}

impl FromPython for i32 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= i32::MIN as i64 && n <= i32::MAX as i64 {
            Ok(n as i32)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "i32"))
        }
    }
}

impl FromPython for i16 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= i16::MIN as i64 && n <= i16::MAX as i64 {
            Ok(n as i16)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "i16"))
        }
    }
}

impl FromPython for i8 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= i8::MIN as i64 && n <= i8::MAX as i64 {
            Ok(n as i8)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "i8"))
        }
    }
}

impl FromPython for u64 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= 0 {
            Ok(n as u64)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "u64"))
        }
    }
}

impl FromPython for u32 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= 0 && n <= u32::MAX as i64 {
            Ok(n as u32)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "u32"))
        }
    }
}

impl FromPython for u16 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= 0 && n <= u16::MAX as i64 {
            Ok(n as u16)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "u16"))
        }
    }
}

impl FromPython for u8 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= 0 && n <= u8::MAX as i64 {
            Ok(n as u8)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "u8"))
        }
    }
}

impl FromPython for usize {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= 0 && (n as u64) <= usize::MAX as u64 {
            Ok(n as usize)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "usize"))
        }
    }
}

impl FromPython for isize {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let n = i64::from_python(value)?;
        if n >= isize::MIN as i64 && n <= isize::MAX as i64 {
            Ok(n as isize)
        } else {
            Err(PyBridgeError::numeric_overflow(n.to_string(), "isize"))
        }
    }
}

impl FromPython for f64 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::Float(f) => Ok(*f),
            PyValue::Int(n) => Ok(*n as f64),
            _ => Err(PyBridgeError::type_mismatch("float", value.type_name())),
        }
    }
}

impl FromPython for f32 {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        let f = f64::from_python(value)?;
        if f.is_finite() && f.abs() <= f32::MAX as f64 {
            Ok(f as f32)
        } else if f.is_infinite() || f.is_nan() {
            Ok(f as f32) // Preserve inf/nan
        } else {
            Err(PyBridgeError::numeric_overflow(f.to_string(), "f32"))
        }
    }
}

impl FromPython for String {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::String(s) => Ok(s.to_string()),
            PyValue::Bytes(b) => {
                String::from_utf8(b.clone()).map_err(|e| PyBridgeError::encoding_error(e.to_string()))
            }
            _ => Err(PyBridgeError::type_mismatch("str", value.type_name())),
        }
    }
}

impl FromPython for SmolStr {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        String::from_python(value).map(SmolStr::new)
    }
}

impl<T: FromPython> FromPython for Option<T> {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::None => Ok(None),
            _ => T::from_python(value).map(Some),
        }
    }
}

impl<T: FromPython> FromPython for Vec<T> {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::List(list) => list.iter().map(|item| T::from_python(&item)).collect(),
            PyValue::Tuple(items) => items.iter().map(|item| T::from_python(item)).collect(),
            _ => Err(PyBridgeError::type_mismatch("list", value.type_name())),
        }
    }
}

impl FromPython for PyValue {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        Ok(value.clone())
    }
}

impl FromPython for PyList {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::List(list) => Ok(list.clone()),
            _ => Err(PyBridgeError::type_mismatch("list", value.type_name())),
        }
    }
}

impl FromPython for PyDict {
    fn from_python(value: &PyValue) -> PyBridgeResult<Self> {
        match value {
            PyValue::Dict(dict) => Ok(dict.clone()),
            _ => Err(PyBridgeError::type_mismatch("dict", value.type_name())),
        }
    }
}

// ============================================================================
// Conversion Helpers
// ============================================================================

/// Helper to convert a vector of PyValue to a specific type
pub fn extract_list<T: FromPython>(list: &PyList) -> PyBridgeResult<Vec<T>> {
    list.iter().map(|item| T::from_python(&item)).collect()
}

/// Helper to convert a PyDict to a HashMap
pub fn extract_dict<V: FromPython>(dict: &PyDict) -> PyBridgeResult<HashMap<String, V>> {
    let mut result = HashMap::new();
    for (k, v) in dict.iter() {
        result.insert(k.to_string(), V::from_python(&v)?);
    }
    Ok(result)
}

/// Builder for creating PyDict from key-value pairs
pub struct PyDictBuilder {
    dict: PyDict,
}

impl PyDictBuilder {
    /// Create a new empty dict builder
    pub fn new() -> Self {
        Self {
            dict: PyDict::new(),
        }
    }

    /// Add a key-value pair
    pub fn insert(self, key: impl Into<SmolStr>, value: impl ToPython) -> Self {
        self.dict.set(key, value.to_python());
        self
    }

    /// Build the final PyDict
    pub fn build(self) -> PyDict {
        self.dict
    }
}

impl Default for PyDictBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating PyList
pub struct PyListBuilder {
    items: Vec<PyValue>,
}

impl PyListBuilder {
    /// Create a new empty list builder
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add an item
    pub fn push(mut self, value: impl ToPython) -> Self {
        self.items.push(value.to_python());
        self
    }

    /// Build the final PyList
    pub fn build(self) -> PyList {
        PyList::from_vec(self.items)
    }
}

impl Default for PyListBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_python_primitives() {
        assert!(matches!(42i64.to_python(), PyValue::Int(42)));
        assert!(matches!(3.14f64.to_python(), PyValue::Float(f) if (f - 3.14).abs() < 0.001));
        assert!(matches!(true.to_python(), PyValue::Bool(true)));
        assert!(matches!("hello".to_python(), PyValue::String(s) if s == "hello"));
        assert!(matches!(().to_python(), PyValue::None));
    }

    #[test]
    fn test_to_python_option() {
        let some: Option<i64> = Some(42);
        let none: Option<i64> = None;

        assert!(matches!(some.to_python(), PyValue::Int(42)));
        assert!(matches!(none.to_python(), PyValue::None));
    }

    #[test]
    fn test_to_python_vec() {
        let v = vec![1i64, 2, 3];
        let py = v.to_python();

        if let PyValue::List(list) = py {
            assert_eq!(list.len(), 3);
            assert_eq!(list.get(0), Some(PyValue::Int(1)));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_to_python_hashmap() {
        let mut map = HashMap::new();
        map.insert("a", 1i64);
        map.insert("b", 2i64);

        let py = map.to_python();

        if let PyValue::Dict(dict) = py {
            assert_eq!(dict.len(), 2);
            assert_eq!(dict.get("a"), Some(PyValue::Int(1)));
            assert_eq!(dict.get("b"), Some(PyValue::Int(2)));
        } else {
            panic!("Expected dict");
        }
    }

    #[test]
    fn test_from_python_primitives() {
        assert_eq!(i64::from_python(&PyValue::Int(42)).unwrap(), 42);
        assert_eq!(
            f64::from_python(&PyValue::Float(3.14)).unwrap(),
            3.14
        );
        assert!(bool::from_python(&PyValue::Bool(true)).unwrap());
        assert_eq!(
            String::from_python(&PyValue::String("hello".into())).unwrap(),
            "hello"
        );
        assert_eq!(<()>::from_python(&PyValue::None).unwrap(), ());
    }

    #[test]
    fn test_from_python_numeric_coercion() {
        // int to float
        assert_eq!(f64::from_python(&PyValue::Int(42)).unwrap(), 42.0);

        // bool to int
        assert_eq!(i64::from_python(&PyValue::Bool(true)).unwrap(), 1);
        assert_eq!(i64::from_python(&PyValue::Bool(false)).unwrap(), 0);

        // float with no fraction to int
        assert_eq!(i64::from_python(&PyValue::Float(42.0)).unwrap(), 42);
    }

    #[test]
    fn test_from_python_overflow() {
        // i8 overflow
        let result = i8::from_python(&PyValue::Int(1000));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PyBridgeError::NumericOverflow { .. }
        ));

        // u64 negative
        let result = u64::from_python(&PyValue::Int(-1));
        assert!(result.is_err());
    }

    #[test]
    fn test_from_python_type_mismatch() {
        let result = i64::from_python(&PyValue::String("hello".into()));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PyBridgeError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn test_from_python_option() {
        let some = Option::<i64>::from_python(&PyValue::Int(42)).unwrap();
        assert_eq!(some, Some(42));

        let none = Option::<i64>::from_python(&PyValue::None).unwrap();
        assert_eq!(none, None);
    }

    #[test]
    fn test_from_python_vec() {
        let list = PyList::from_vec(vec![
            PyValue::Int(1),
            PyValue::Int(2),
            PyValue::Int(3),
        ]);

        let v = Vec::<i64>::from_python(&PyValue::List(list)).unwrap();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn test_dict_builder() {
        let dict = PyDictBuilder::new()
            .insert("name", "Alice")
            .insert("age", 30i64)
            .insert("active", true)
            .build();

        assert_eq!(dict.get("name"), Some(PyValue::String("Alice".into())));
        assert_eq!(dict.get("age"), Some(PyValue::Int(30)));
        assert_eq!(dict.get("active"), Some(PyValue::Bool(true)));
    }

    #[test]
    fn test_list_builder() {
        let list = PyListBuilder::new()
            .push(1i64)
            .push(2i64)
            .push(3i64)
            .build();

        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Some(PyValue::Int(1)));
    }

    #[test]
    fn test_extract_list() {
        let list = PyList::from_vec(vec![
            PyValue::Int(10),
            PyValue::Int(20),
            PyValue::Int(30),
        ]);

        let result: Vec<i64> = extract_list(&list).unwrap();
        assert_eq!(result, vec![10, 20, 30]);
    }

    #[test]
    fn test_extract_dict() {
        let dict = PyDict::new();
        dict.set("x", PyValue::Int(1));
        dict.set("y", PyValue::Int(2));

        let result: HashMap<String, i64> = extract_dict(&dict).unwrap();
        assert_eq!(result.get("x"), Some(&1));
        assert_eq!(result.get("y"), Some(&2));
    }
}
