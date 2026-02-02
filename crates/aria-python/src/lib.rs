//! Python bindings for the Aria programming language.
//!
//! This crate provides Python interoperability for Aria, allowing you to:
//! - Execute Aria code from Python
//! - Convert between Aria and Python values
//! - Call Aria functions from Python
//! - Access Aria variables and modules

use std::cell::RefCell;
use std::rc::Rc;

use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};

use aria_interpreter::{Interpreter, Value};
use aria_parser::Parser;
use indexmap::IndexMap;
use smol_str::SmolStr;

/// Python wrapper for Aria values.
///
/// This enum represents all possible Aria values that can be passed to/from Python.
/// Note: This type is not Send/Sync because it uses Rc internally.
#[pyclass(name = "AriaValue", unsendable)]
#[derive(Debug, Clone)]
pub struct PyAriaValue {
    inner: Value,
}

#[pymethods]
impl PyAriaValue {
    /// Get the type name of this value.
    #[getter]
    pub fn type_name(&self) -> String {
        self.inner.type_name().to_string()
    }

    /// Convert to Python object.
    pub fn to_python(&self, py: Python) -> PyResult<Py<PyAny>> {
        aria_to_python(py, &self.inner)
    }

    /// Check if value is truthy.
    pub fn is_truthy(&self) -> bool {
        self.inner.is_truthy()
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!("AriaValue({})", self.inner)
    }

    /// String conversion.
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Boolean conversion.
    fn __bool__(&self) -> bool {
        self.inner.is_truthy()
    }

    /// Integer conversion (if applicable).
    fn __int__(&self) -> PyResult<i64> {
        self.inner.as_int().ok_or_else(|| {
            PyTypeError::new_err(format!("Cannot convert {} to int", self.inner.type_name()))
        })
    }

    /// Float conversion (if applicable).
    fn __float__(&self) -> PyResult<f64> {
        self.inner.as_float().ok_or_else(|| {
            PyTypeError::new_err(format!("Cannot convert {} to float", self.inner.type_name()))
        })
    }
}

/// The Aria interpreter accessible from Python.
///
/// This class provides the main interface for executing Aria code from Python.
/// Note: This type is not Send/Sync because the underlying interpreter uses Rc internally.
#[pyclass(name = "AriaInterpreter", unsendable)]
pub struct PyAriaInterpreter {
    interpreter: Interpreter,
}

#[pymethods]
impl PyAriaInterpreter {
    /// Create a new Aria interpreter.
    #[new]
    pub fn new() -> Self {
        PyAriaInterpreter {
            interpreter: Interpreter::new(),
        }
    }

    /// Execute Aria code and return the result.
    ///
    /// # Arguments
    /// * `code` - The Aria source code to execute
    ///
    /// # Returns
    /// The result of executing the code as a Python object
    ///
    /// # Raises
    /// * `RuntimeError` - If execution fails
    pub fn eval(&mut self, py: Python, code: &str) -> PyResult<Py<PyAny>> {
        // Parse the source code into an AST
        let mut parser = Parser::new(code);
        let program = parser
            .parse_program()
            .map_err(|e| PyRuntimeError::new_err(format!("Parser error: {:?}", e)))?;

        // Execute the program
        let result = self
            .interpreter
            .run(&program)
            .map_err(|e| PyRuntimeError::new_err(format!("Runtime error: {}", e)))?;

        // Convert the result to a Python object
        aria_to_python(py, &result)
    }

    /// Execute Aria code without returning the result (for side effects).
    ///
    /// # Arguments
    /// * `code` - The Aria source code to execute
    pub fn exec(&mut self, code: &str) -> PyResult<()> {
        // Parse the source code into an AST
        let mut parser = Parser::new(code);
        let program = parser
            .parse_program()
            .map_err(|e| PyRuntimeError::new_err(format!("Parser error: {:?}", e)))?;

        // Execute the program
        self.interpreter
            .run(&program)
            .map_err(|e| PyRuntimeError::new_err(format!("Runtime error: {}", e)))?;

        Ok(())
    }

    /// Get a global variable from the interpreter.
    ///
    /// # Arguments
    /// * `name` - The name of the variable
    ///
    /// # Returns
    /// The value of the variable as a Python object
    pub fn get_global(&self, py: Python, name: &str) -> PyResult<Py<PyAny>> {
        let globals = self.interpreter.globals.borrow();
        let value = globals
            .get(&SmolStr::new(name))
            .ok_or_else(|| PyValueError::new_err(format!("Variable '{}' not found", name)))?;

        aria_to_python(py, &value)
    }

    /// Set a global variable in the interpreter.
    ///
    /// # Arguments
    /// * `name` - The name of the variable
    /// * `value` - The Python value to set (will be converted to Aria value)
    pub fn set_global(&mut self, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let aria_value = python_to_aria(value)?;
        self.interpreter
            .globals
            .borrow_mut()
            .define(SmolStr::new(name), aria_value);
        Ok(())
    }

    /// Call an Aria function by name.
    ///
    /// # Arguments
    /// * `name` - The name of the function
    /// * `args` - Tuple of arguments to pass to the function
    ///
    /// # Returns
    /// The result of calling the function
    pub fn call_function(
        &mut self,
        py: Python,
        name: &str,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<Py<PyAny>> {
        // Convert Python args to Aria values and set them as globals
        let aria_args: Result<Vec<Value>, _> = args
            .iter()
            .map(|arg| python_to_aria(&arg))
            .collect();
        let aria_args = aria_args?;

        // Build the function call code dynamically
        let mut code = format!("fn main() -> Int {{\n    return {}(", name);
        for (i, _) in aria_args.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            let arg_name = format!("__arg{}", i);
            code.push_str(&arg_name);
            // Set the argument as a global
            self.interpreter
                .globals
                .borrow_mut()
                .define(SmolStr::new(arg_name), aria_args[i].clone());
        }
        code.push_str(");\n}\n");

        // Parse and execute the call
        let mut parser = Parser::new(&code);
        let program = parser
            .parse_program()
            .map_err(|e| PyRuntimeError::new_err(format!("Parser error: {:?}", e)))?;

        let result = self
            .interpreter
            .run(&program)
            .map_err(|e| PyRuntimeError::new_err(format!("Runtime error: {}", e)))?;

        aria_to_python(py, &result)
    }

    /// List all global variables and functions.
    pub fn list_globals(&self) -> Vec<String> {
        self.interpreter
            .globals
            .borrow()
            .all_names()
            .into_iter()
            .map(|k| k.to_string())
            .collect()
    }
}

/// Convert an Aria value to a Python object.
fn aria_to_python(py: Python, value: &Value) -> PyResult<Py<PyAny>> {
    match value {
        Value::Nil => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Int(n) => Ok(n.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Float(f) => Ok(f.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::String(s) => Ok(s.as_str().into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Array(arr) => {
            let arr = arr.borrow();
            let py_list = PyList::empty(py);
            for item in arr.iter() {
                py_list.append(aria_to_python(py, item)?)?;
            }
            Ok(py_list.into_any().unbind())
        }
        Value::Map(map) => {
            let map = map.borrow();
            let py_dict = PyDict::new(py);
            for (k, v) in map.iter() {
                py_dict.set_item(k.as_str(), aria_to_python(py, v)?)?;
            }
            Ok(py_dict.into_any().unbind())
        }
        Value::Tuple(tuple) => {
            let items: Result<Vec<_>, _> = tuple.iter().map(|v| aria_to_python(py, v)).collect();
            Ok(PyTuple::new(py, items?)?.into_any().unbind())
        }
        Value::Range {
            start,
            end,
            inclusive,
        } => {
            // Convert range to Python tuple (start, end)
            if *inclusive {
                Ok((*start, *end + 1).into_pyobject(py)?.into_any().unbind())
            } else {
                Ok((*start, *end).into_pyobject(py)?.into_any().unbind())
            }
        }
        Value::Function(_) => {
            // For functions, return a wrapped AriaValue
            Ok(PyAriaValue {
                inner: value.clone(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind())
        }
        Value::BuiltinFunction(_) => {
            // For builtin functions, return a wrapped AriaValue
            Ok(PyAriaValue {
                inner: value.clone(),
            }
            .into_pyobject(py)?
            .into_any()
            .unbind())
        }
        Value::Struct(s) => {
            // Convert struct to dictionary
            let s = s.borrow();
            let py_dict = PyDict::new(py);
            py_dict.set_item("__struct_name__", s.name.as_str())?;
            for (k, v) in s.fields.iter() {
                py_dict.set_item(k.as_str(), aria_to_python(py, v)?)?;
            }
            Ok(py_dict.into_any().unbind())
        }
        Value::EnumVariant {
            enum_name,
            variant_name,
            fields,
        } => {
            // Convert enum variant to dictionary
            let py_dict = PyDict::new(py);
            py_dict.set_item("__enum_name__", enum_name.as_str())?;
            py_dict.set_item("__variant_name__", variant_name.as_str())?;
            if let Some(fields) = fields {
                let py_fields: Result<Vec<_>, _> =
                    fields.iter().map(|v| aria_to_python(py, v)).collect();
                py_dict.set_item("fields", PyList::new(py, py_fields?)?)?;
            }
            Ok(py_dict.into_any().unbind())
        }
    }
}

/// Convert a Python object to an Aria value.
fn python_to_aria(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    // Try different Python types in order
    if obj.is_none() {
        Ok(Value::Nil)
    } else if let Ok(b) = obj.cast_exact::<PyBool>() {
        Ok(Value::Bool(b.is_true()))
    } else if let Ok(i) = obj.cast_exact::<PyInt>() {
        let n: i64 = i.extract()?;
        Ok(Value::Int(n))
    } else if let Ok(f) = obj.cast_exact::<PyFloat>() {
        let n: f64 = f.extract()?;
        Ok(Value::Float(n))
    } else if let Ok(s) = obj.cast_exact::<PyString>() {
        let s: String = s.extract()?;
        Ok(Value::String(SmolStr::new(s)))
    } else if let Ok(list) = obj.cast_exact::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(python_to_aria(&item)?);
        }
        Ok(Value::Array(Rc::new(RefCell::new(items))))
    } else if let Ok(tuple) = obj.cast_exact::<PyTuple>() {
        let mut items = Vec::new();
        for item in tuple.iter() {
            items.push(python_to_aria(&item)?);
        }
        Ok(Value::Tuple(Rc::new(items)))
    } else if let Ok(dict) = obj.cast_exact::<PyDict>() {
        let mut map = IndexMap::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(SmolStr::new(key), python_to_aria(&v)?);
        }
        Ok(Value::Map(Rc::new(RefCell::new(map))))
    } else if let Ok(aria_val) = obj.extract::<PyAriaValue>() {
        Ok(aria_val.inner)
    } else {
        Err(PyTypeError::new_err(format!(
            "Cannot convert Python type {} to Aria value",
            obj.get_type().name()?
        )))
    }
}

/// Convenience function to evaluate Aria code from Python.
///
/// # Arguments
/// * `code` - The Aria source code to execute
///
/// # Returns
/// The result as a Python object
#[pyfunction]
fn eval_aria(py: Python, code: &str) -> PyResult<Py<PyAny>> {
    let mut interpreter = PyAriaInterpreter::new();
    interpreter.eval(py, code)
}

/// Convenience function to execute Aria code from Python (no return value).
///
/// # Arguments
/// * `code` - The Aria source code to execute
#[pyfunction]
fn exec_aria(code: &str) -> PyResult<()> {
    let mut interpreter = PyAriaInterpreter::new();
    interpreter.exec(code)
}

/// Python module definition.
#[pymodule]
fn aria_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAriaValue>()?;
    m.add_class::<PyAriaInterpreter>()?;
    m.add_function(wrap_pyfunction!(eval_aria, m)?)?;
    m.add_function(wrap_pyfunction!(exec_aria, m)?)?;

    // Module metadata
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "Python bindings for the Aria programming language")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversion_roundtrip() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            // Test integer
            let aria_int = Value::Int(42);
            let py_obj = aria_to_python(py, &aria_int).unwrap();
            let aria_back = python_to_aria(&py_obj.bind(py)).unwrap();
            assert_eq!(aria_int, aria_back);

            // Test string
            let aria_str = Value::String(SmolStr::new("hello"));
            let py_obj = aria_to_python(py, &aria_str).unwrap();
            let aria_back = python_to_aria(&py_obj.bind(py)).unwrap();
            assert_eq!(aria_str, aria_back);

            // Test bool
            let aria_bool = Value::Bool(true);
            let py_obj = aria_to_python(py, &aria_bool).unwrap();
            let aria_back = python_to_aria(&py_obj.bind(py)).unwrap();
            assert_eq!(aria_bool, aria_back);

            // Test nil
            let aria_nil = Value::Nil;
            let py_obj = aria_to_python(py, &aria_nil).unwrap();
            let aria_back = python_to_aria(&py_obj.bind(py)).unwrap();
            assert_eq!(aria_nil, aria_back);
        });
    }

    #[test]
    fn test_interpreter_basic() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut interp = PyAriaInterpreter::new();

            // Test simple arithmetic
            let result = interp.eval(py, "fn main() -> Int { return 2 + 2; }");
            assert!(result.is_ok());
            let result_val: i64 = result.unwrap().extract(py).unwrap();
            assert_eq!(result_val, 4);
        });
    }
}
