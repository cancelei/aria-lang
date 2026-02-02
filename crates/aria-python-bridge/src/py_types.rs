//! Python Type Representations
//!
//! This module provides Aria's representation of Python types.
//! Based on ARIA-M10 Python Interop milestone.
//!
//! ## Type Hierarchy
//!
//! - `PyValue`: Enum covering all Python values
//! - `PyObject`: Base object type (stub for real PyObject*)
//! - `PyList`: Python list type
//! - `PyDict`: Python dictionary type
//! - `PyArray`: NumPy array wrapper (see array_bridge module)
//!
//! ## Design Notes
//!
//! This module provides stub infrastructure that mirrors how real pyo3
//! bindings would work. The actual implementation would use pyo3::PyAny,
//! pyo3::types::PyList, etc.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::error::{PyBridgeError, PyBridgeResult};

// ============================================================================
// PyValue Enum - The main Python value representation
// ============================================================================

/// Represents any Python value in Aria.
///
/// This enum maps Python's dynamic type system to a static representation
/// that Aria can work with. It's similar to serde_json::Value but for Python.
#[derive(Debug, Clone)]
pub enum PyValue {
    /// Python `None`
    None,

    /// Python `bool` (True/False)
    Bool(bool),

    /// Python `int` (arbitrary precision in Python, i64 for bridge)
    Int(i64),

    /// Python `float` (IEEE 754 double)
    Float(f64),

    /// Python `str`
    String(SmolStr),

    /// Python `bytes`
    Bytes(Vec<u8>),

    /// Python `list`
    List(PyList),

    /// Python `tuple` (immutable)
    Tuple(Vec<PyValue>),

    /// Python `dict`
    Dict(PyDict),

    /// Python `set`
    Set(Vec<PyValue>),

    /// Generic Python object (opaque handle)
    Object(PyObject),

    /// NumPy array reference
    Array(PyArrayRef),
}

impl PyValue {
    /// Get the Python type name for this value
    pub fn type_name(&self) -> &'static str {
        match self {
            PyValue::None => "NoneType",
            PyValue::Bool(_) => "bool",
            PyValue::Int(_) => "int",
            PyValue::Float(_) => "float",
            PyValue::String(_) => "str",
            PyValue::Bytes(_) => "bytes",
            PyValue::List(_) => "list",
            PyValue::Tuple(_) => "tuple",
            PyValue::Dict(_) => "dict",
            PyValue::Set(_) => "set",
            PyValue::Object(obj) => obj.type_name(),
            PyValue::Array(_) => "numpy.ndarray",
        }
    }

    /// Check if this value is None
    pub fn is_none(&self) -> bool {
        matches!(self, PyValue::None)
    }

    /// Check if this value is truthy (Python bool coercion)
    pub fn is_truthy(&self) -> bool {
        match self {
            PyValue::None => false,
            PyValue::Bool(b) => *b,
            PyValue::Int(n) => *n != 0,
            PyValue::Float(f) => *f != 0.0 && !f.is_nan(),
            PyValue::String(s) => !s.is_empty(),
            PyValue::Bytes(b) => !b.is_empty(),
            PyValue::List(list) => !list.is_empty(),
            PyValue::Tuple(t) => !t.is_empty(),
            PyValue::Dict(dict) => !dict.is_empty(),
            PyValue::Set(s) => !s.is_empty(),
            PyValue::Object(_) => true,
            PyValue::Array(arr) => arr.len > 0,
        }
    }

    /// Try to extract as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PyValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to extract as i64
    pub fn as_int(&self) -> Option<i64> {
        match self {
            PyValue::Int(n) => Some(*n),
            PyValue::Float(f) if f.fract() == 0.0 => Some(*f as i64),
            PyValue::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Try to extract as f64
    pub fn as_float(&self) -> Option<f64> {
        match self {
            PyValue::Float(f) => Some(*f),
            PyValue::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Try to extract as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PyValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Try to extract as list
    pub fn as_list(&self) -> Option<&PyList> {
        match self {
            PyValue::List(list) => Some(list),
            _ => None,
        }
    }

    /// Try to extract as dict
    pub fn as_dict(&self) -> Option<&PyDict> {
        match self {
            PyValue::Dict(dict) => Some(dict),
            _ => None,
        }
    }
}

impl fmt::Display for PyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PyValue::None => write!(f, "None"),
            PyValue::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            PyValue::Int(n) => write!(f, "{}", n),
            PyValue::Float(n) => write!(f, "{}", n),
            PyValue::String(s) => write!(f, "'{}'", s),
            PyValue::Bytes(b) => write!(f, "b'{:?}'", b),
            PyValue::List(list) => write!(f, "{}", list),
            PyValue::Tuple(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                if items.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            PyValue::Dict(dict) => write!(f, "{}", dict),
            PyValue::Set(items) => {
                write!(f, "{{")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "}}")
            }
            PyValue::Object(obj) => write!(f, "<{} object>", obj.type_name()),
            PyValue::Array(arr) => write!(f, "<ndarray shape={:?}>", arr.shape),
        }
    }
}

impl Default for PyValue {
    fn default() -> Self {
        PyValue::None
    }
}

// ============================================================================
// PyObject - Opaque Python object handle
// ============================================================================

/// Opaque handle to a Python object.
///
/// In a real implementation, this would wrap a `*mut pyo3::ffi::PyObject`
/// with proper reference counting. This stub version uses a type name
/// and internal ID for tracking.
#[derive(Debug, Clone)]
pub struct PyObject {
    /// Internal object ID (stub for pointer)
    id: u64,
    /// Python type name
    type_name: SmolStr,
    /// Reference count (stub for real refcount)
    ref_count: Rc<RefCell<usize>>,
    /// Attributes (stub for getattr)
    attributes: Rc<RefCell<HashMap<String, PyValue>>>,
}

impl PyObject {
    /// Create a new PyObject with a given type name
    pub fn new(type_name: impl Into<SmolStr>) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            type_name: type_name.into(),
            ref_count: Rc::new(RefCell::new(1)),
            attributes: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Get the type name of this object
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Get the internal ID (stub for pointer address)
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the current reference count
    pub fn ref_count(&self) -> usize {
        *self.ref_count.borrow()
    }

    /// Increment reference count
    pub fn incref(&self) {
        *self.ref_count.borrow_mut() += 1;
    }

    /// Decrement reference count
    pub fn decref(&self) {
        let mut count = self.ref_count.borrow_mut();
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Check if this object is valid (ref_count > 0)
    pub fn is_valid(&self) -> bool {
        self.ref_count() > 0
    }

    /// Set an attribute on this object
    pub fn set_attr(&self, name: impl Into<String>, value: PyValue) {
        self.attributes.borrow_mut().insert(name.into(), value);
    }

    /// Get an attribute from this object
    pub fn get_attr(&self, name: &str) -> Option<PyValue> {
        self.attributes.borrow().get(name).cloned()
    }

    /// Check if object has an attribute
    pub fn has_attr(&self, name: &str) -> bool {
        self.attributes.borrow().contains_key(name)
    }

    /// Get all attribute names
    pub fn attr_names(&self) -> Vec<String> {
        self.attributes.borrow().keys().cloned().collect()
    }
}

impl PartialEq for PyObject {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

// ============================================================================
// PyList - Python list type
// ============================================================================

/// Python list representation.
///
/// A mutable, ordered collection of PyValue items.
#[derive(Debug, Clone)]
pub struct PyList {
    items: Rc<RefCell<Vec<PyValue>>>,
}

impl PyList {
    /// Create an empty list
    pub fn new() -> Self {
        Self {
            items: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Create a list from a vector
    pub fn from_vec(items: Vec<PyValue>) -> Self {
        Self {
            items: Rc::new(RefCell::new(items)),
        }
    }

    /// Get the length of the list
    pub fn len(&self) -> usize {
        self.items.borrow().len()
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.items.borrow().is_empty()
    }

    /// Get an item by index
    pub fn get(&self, index: usize) -> Option<PyValue> {
        self.items.borrow().get(index).cloned()
    }

    /// Set an item by index
    pub fn set(&self, index: usize, value: PyValue) -> PyBridgeResult<()> {
        let mut items = self.items.borrow_mut();
        if index >= items.len() {
            return Err(PyBridgeError::Custom(format!(
                "list index {} out of range",
                index
            )));
        }
        items[index] = value;
        Ok(())
    }

    /// Append an item to the end
    pub fn append(&self, value: PyValue) {
        self.items.borrow_mut().push(value);
    }

    /// Insert an item at index
    pub fn insert(&self, index: usize, value: PyValue) {
        let mut items = self.items.borrow_mut();
        let idx = index.min(items.len());
        items.insert(idx, value);
    }

    /// Remove and return item at index
    pub fn pop(&self, index: Option<usize>) -> Option<PyValue> {
        let mut items = self.items.borrow_mut();
        if items.is_empty() {
            return None;
        }
        let idx = index.unwrap_or(items.len() - 1);
        if idx >= items.len() {
            return None;
        }
        Some(items.remove(idx))
    }

    /// Clear the list
    pub fn clear(&self) {
        self.items.borrow_mut().clear();
    }

    /// Iterate over items
    pub fn iter(&self) -> impl Iterator<Item = PyValue> {
        self.items.borrow().clone().into_iter()
    }

    /// Convert to a Vec
    pub fn to_vec(&self) -> Vec<PyValue> {
        self.items.borrow().clone()
    }

    /// Extend from an iterator
    pub fn extend(&self, iter: impl IntoIterator<Item = PyValue>) {
        self.items.borrow_mut().extend(iter);
    }
}

impl Default for PyList {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PyList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let items = self.items.borrow();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item)?;
        }
        write!(f, "]")
    }
}

impl FromIterator<PyValue> for PyList {
    fn from_iter<I: IntoIterator<Item = PyValue>>(iter: I) -> Self {
        PyList::from_vec(iter.into_iter().collect())
    }
}

// ============================================================================
// PyDict - Python dictionary type
// ============================================================================

/// Python dictionary representation.
///
/// A mutable mapping from string keys to PyValue values.
/// Note: Real Python dicts support any hashable key, but we simplify to strings.
#[derive(Debug, Clone)]
pub struct PyDict {
    items: Rc<RefCell<IndexMap<SmolStr, PyValue>>>,
}

impl PyDict {
    /// Create an empty dictionary
    pub fn new() -> Self {
        Self {
            items: Rc::new(RefCell::new(IndexMap::new())),
        }
    }

    /// Get the number of key-value pairs
    pub fn len(&self) -> usize {
        self.items.borrow().len()
    }

    /// Check if the dictionary is empty
    pub fn is_empty(&self) -> bool {
        self.items.borrow().is_empty()
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<PyValue> {
        self.items.borrow().get(key).cloned()
    }

    /// Set a key-value pair
    pub fn set(&self, key: impl Into<SmolStr>, value: PyValue) {
        self.items.borrow_mut().insert(key.into(), value);
    }

    /// Remove a key and return its value
    pub fn remove(&self, key: &str) -> Option<PyValue> {
        self.items.borrow_mut().shift_remove(key)
    }

    /// Check if key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.items.borrow().contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<SmolStr> {
        self.items.borrow().keys().cloned().collect()
    }

    /// Get all values
    pub fn values(&self) -> Vec<PyValue> {
        self.items.borrow().values().cloned().collect()
    }

    /// Iterate over key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (SmolStr, PyValue)> {
        self.items.borrow().clone().into_iter()
    }

    /// Clear the dictionary
    pub fn clear(&self) {
        self.items.borrow_mut().clear();
    }

    /// Update from another dict
    pub fn update(&self, other: &PyDict) {
        let other_items = other.items.borrow();
        let mut self_items = self.items.borrow_mut();
        for (k, v) in other_items.iter() {
            self_items.insert(k.clone(), v.clone());
        }
    }

    /// Get or insert with default
    pub fn get_or_insert(&self, key: impl Into<SmolStr>, default: PyValue) -> PyValue {
        let key = key.into();
        {
            let items = self.items.borrow();
            if let Some(v) = items.get(&key) {
                return v.clone();
            }
        }
        self.items.borrow_mut().insert(key.clone(), default.clone());
        default
    }
}

impl Default for PyDict {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PyDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let items = self.items.borrow();
        for (i, (k, v)) in items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "'{}': {}", k, v)?;
        }
        write!(f, "}}")
    }
}

// ============================================================================
// PyArrayRef - NumPy array reference (lightweight metadata)
// ============================================================================

/// Lightweight reference to a NumPy array.
///
/// This contains just the metadata needed to identify and work with
/// an array. The actual data is managed by the ArrayBridge.
#[derive(Debug, Clone)]
pub struct PyArrayRef {
    /// Array ID (for lookup in ArrayBridge)
    pub id: u64,
    /// Array shape
    pub shape: Vec<usize>,
    /// Total number of elements
    pub len: usize,
    /// Data type
    pub dtype: SmolStr,
    /// Whether this is a view (borrowed) or owned
    pub is_view: bool,
}

impl PyArrayRef {
    /// Create a new array reference
    pub fn new(id: u64, shape: Vec<usize>, dtype: impl Into<SmolStr>) -> Self {
        let len = shape.iter().product();
        Self {
            id,
            shape,
            len,
            dtype: dtype.into(),
            is_view: false,
        }
    }

    /// Mark this as a view
    pub fn as_view(mut self) -> Self {
        self.is_view = true;
        self
    }

    /// Get the number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Check if this is a scalar (0-d array)
    pub fn is_scalar(&self) -> bool {
        self.shape.is_empty() || (self.shape.len() == 1 && self.shape[0] == 1)
    }

    /// Get the shape as a tuple
    pub fn shape_tuple(&self) -> Vec<usize> {
        self.shape.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pyvalue_type_names() {
        assert_eq!(PyValue::None.type_name(), "NoneType");
        assert_eq!(PyValue::Bool(true).type_name(), "bool");
        assert_eq!(PyValue::Int(42).type_name(), "int");
        assert_eq!(PyValue::Float(3.14).type_name(), "float");
        assert_eq!(PyValue::String("hello".into()).type_name(), "str");
    }

    #[test]
    fn test_pyvalue_truthiness() {
        assert!(!PyValue::None.is_truthy());
        assert!(!PyValue::Bool(false).is_truthy());
        assert!(PyValue::Bool(true).is_truthy());
        assert!(!PyValue::Int(0).is_truthy());
        assert!(PyValue::Int(1).is_truthy());
        assert!(PyValue::Int(-1).is_truthy());
        assert!(!PyValue::String("".into()).is_truthy());
        assert!(PyValue::String("hello".into()).is_truthy());
    }

    #[test]
    fn test_pyvalue_conversions() {
        let v = PyValue::Int(42);
        assert_eq!(v.as_int(), Some(42));
        assert_eq!(v.as_float(), Some(42.0));
        assert_eq!(v.as_bool(), None);

        let v = PyValue::Bool(true);
        assert_eq!(v.as_bool(), Some(true));
        assert_eq!(v.as_int(), Some(1));
    }

    #[test]
    fn test_pylist_operations() {
        let list = PyList::new();
        assert!(list.is_empty());

        list.append(PyValue::Int(1));
        list.append(PyValue::Int(2));
        list.append(PyValue::Int(3));

        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Some(PyValue::Int(1)));
        assert_eq!(list.get(5), None);

        list.set(1, PyValue::Int(42)).unwrap();
        assert_eq!(list.get(1), Some(PyValue::Int(42)));

        let popped = list.pop(None);
        assert_eq!(popped, Some(PyValue::Int(3)));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_pydict_operations() {
        let dict = PyDict::new();
        assert!(dict.is_empty());

        dict.set("name", PyValue::String("Alice".into()));
        dict.set("age", PyValue::Int(30));

        assert_eq!(dict.len(), 2);
        assert!(dict.contains_key("name"));
        assert_eq!(dict.get("name"), Some(PyValue::String("Alice".into())));
        assert_eq!(dict.get("missing"), None);

        let removed = dict.remove("age");
        assert_eq!(removed, Some(PyValue::Int(30)));
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_pyobject_refcount() {
        let obj = PyObject::new("custom");
        assert_eq!(obj.ref_count(), 1);

        obj.incref();
        assert_eq!(obj.ref_count(), 2);

        obj.decref();
        assert_eq!(obj.ref_count(), 1);

        obj.decref();
        assert_eq!(obj.ref_count(), 0);
        assert!(!obj.is_valid());
    }

    #[test]
    fn test_pyobject_attributes() {
        let obj = PyObject::new("person");

        obj.set_attr("name", PyValue::String("Bob".into()));
        obj.set_attr("age", PyValue::Int(25));

        assert!(obj.has_attr("name"));
        assert!(!obj.has_attr("missing"));

        assert_eq!(obj.get_attr("name"), Some(PyValue::String("Bob".into())));
        assert_eq!(obj.get_attr("age"), Some(PyValue::Int(25)));
    }

    #[test]
    fn test_pyarray_ref() {
        let arr = PyArrayRef::new(1, vec![3, 4], "float64");
        assert_eq!(arr.ndim(), 2);
        assert_eq!(arr.len, 12);
        assert!(!arr.is_scalar());
        assert!(!arr.is_view);

        let view = arr.clone().as_view();
        assert!(view.is_view);
    }

    #[test]
    fn test_pyvalue_display() {
        assert_eq!(format!("{}", PyValue::None), "None");
        assert_eq!(format!("{}", PyValue::Bool(true)), "True");
        assert_eq!(format!("{}", PyValue::Bool(false)), "False");
        assert_eq!(format!("{}", PyValue::Int(42)), "42");
        assert_eq!(format!("{}", PyValue::Float(3.14)), "3.14");
        assert_eq!(format!("{}", PyValue::String("hello".into())), "'hello'");

        let list = PyList::from_vec(vec![PyValue::Int(1), PyValue::Int(2)]);
        assert_eq!(format!("{}", PyValue::List(list)), "[1, 2]");

        let dict = PyDict::new();
        dict.set("x", PyValue::Int(1));
        assert_eq!(format!("{}", PyValue::Dict(dict)), "{'x': 1}");
    }
}
