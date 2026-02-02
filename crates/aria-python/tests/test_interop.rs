//! Integration tests for Python-Aria interoperability.

use std::ffi::CString;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Helper to run Python code that uses Aria
fn run_python_test<F>(python_code: &str, assertions: F)
where
    F: FnOnce(&Bound<'_, PyDict>) -> PyResult<()>,
{
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        // Import the aria_python module
        let aria_module = PyModule::import(py, c"aria_python").unwrap();

        // Create a local namespace with aria_python available
        let locals = PyDict::new(py);
        locals.set_item("aria", aria_module).unwrap();

        // Run the Python code
        let code = CString::new(python_code).unwrap();
        py.run(&code, None, Some(&locals)).unwrap();

        // Run assertions
        assertions(&locals).unwrap();
    });
}

#[test]
fn test_python_eval_aria_simple() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
fn main() -> Int {
    return 2 + 2;
}
"#;

        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        let eval_aria = aria_module.getattr("eval_aria").unwrap();

        let result = eval_aria.call1((code,)).unwrap();
        let result_val: i64 = result.extract().unwrap();

        assert_eq!(result_val, 4);
    });
}

#[test]
fn test_python_interpreter_basic() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
result = interp.eval("""
fn main() -> Int {
    let x = 10;
    let y = 32;
    return x + y;
}
""")
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        let result = locals.get_item("result").unwrap().unwrap();
        let result_val: i64 = result.extract().unwrap();
        assert_eq!(result_val, 42);
    });
}

#[test]
fn test_python_aria_string_values() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
result = interp.eval("""
fn main() -> String {
    return "Hello from Aria!";
}
""")
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        let result = locals.get_item("result").unwrap().unwrap();
        let result_val: String = result.extract().unwrap();
        assert_eq!(result_val, "Hello from Aria!");
    });
}

#[test]
fn test_python_aria_boolean_values() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
result_true = interp.eval("""
fn main() -> Bool {
    return true;
}
""")
result_false = interp.eval("""
fn main() -> Bool {
    return false;
}
""")
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        let result_true = locals.get_item("result_true").unwrap().unwrap();
        let result_false = locals.get_item("result_false").unwrap().unwrap();

        assert!(result_true.extract::<bool>().unwrap());
        assert!(!result_false.extract::<bool>().unwrap());
    });
}

#[test]
fn test_python_aria_arrays() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
result = interp.eval("""
fn main() -> Array {
    return [1, 2, 3, 4, 5];
}
""")
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        let result = locals.get_item("result").unwrap().unwrap();
        let result_vec: Vec<i64> = result.extract().unwrap();
        assert_eq!(result_vec, vec![1, 2, 3, 4, 5]);
    });
}

#[test]
fn test_python_set_get_globals() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
# Set a global variable from Python
interp.set_global("x", 100)
interp.set_global("name", "Aria")
# Get them back
x_value = interp.get_global("x")
name_value = interp.get_global("name")
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        let x_value = locals.get_item("x_value").unwrap().unwrap();
        let name_value = locals.get_item("name_value").unwrap().unwrap();

        assert_eq!(x_value.extract::<i64>().unwrap(), 100);
        assert_eq!(name_value.extract::<String>().unwrap(), "Aria");
    });
}

#[test]
fn test_python_call_aria_function() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
# Define a function in Aria
interp.exec("""
fn add(a: Int, b: Int) -> Int {
    return a + b;
}
fn main() -> Int {
    return 0;
}
""")
# Call it from Python
result = interp.call_function("add", (10, 32))
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        let result = locals.get_item("result").unwrap().unwrap();
        assert_eq!(result.extract::<i64>().unwrap(), 42);
    });
}

#[test]
fn test_python_list_globals() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
# Define some functions
interp.exec("""
fn foo() -> Int { return 1; }
fn bar() -> Int { return 2; }
fn main() -> Int { return 0; }
""")
# List all globals
globals_list = interp.list_globals()
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        let globals_list = locals.get_item("globals_list").unwrap().unwrap();
        let globals_vec: Vec<String> = globals_list.extract().unwrap();

        // Should contain at least the functions we defined
        assert!(globals_vec.contains(&"foo".to_string()));
        assert!(globals_vec.contains(&"bar".to_string()));
        assert!(globals_vec.contains(&"main".to_string()));
        // Also contains built-in functions
        assert!(globals_vec.contains(&"print".to_string()));
    });
}

#[test]
fn test_python_aria_complex_computation() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
result = interp.eval("""
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}
fn main() -> Int {
    return fibonacci(10);
}
""")
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        let result = locals.get_item("result").unwrap().unwrap();
        assert_eq!(result.extract::<i64>().unwrap(), 55); // 10th Fibonacci number
    });
}

#[test]
fn test_python_aria_type_conversions() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let code = r#"
interp = aria.AriaInterpreter()
# Test various type conversions
interp.set_global("py_int", 42)
interp.set_global("py_float", 3.14)
interp.set_global("py_str", "hello")
interp.set_global("py_bool", True)
interp.set_global("py_list", [1, 2, 3])
interp.set_global("py_dict", {"a": 1, "b": 2})
# Get them back
int_val = interp.get_global("py_int")
float_val = interp.get_global("py_float")
str_val = interp.get_global("py_str")
bool_val = interp.get_global("py_bool")
list_val = interp.get_global("py_list")
dict_val = interp.get_global("py_dict")
"#;

        let locals = PyDict::new(py);
        let aria_module = PyModule::import(py, c"aria_python").unwrap();
        locals.set_item("aria", aria_module).unwrap();

        let code_cstr = CString::new(code).unwrap();
        py.run(&code_cstr, None, Some(&locals)).unwrap();

        // Verify all conversions worked
        assert_eq!(
            locals
                .get_item("int_val")
                .unwrap()
                .unwrap()
                .extract::<i64>()
                .unwrap(),
            42
        );
        assert_eq!(
            locals
                .get_item("float_val")
                .unwrap()
                .unwrap()
                .extract::<f64>()
                .unwrap(),
            3.14
        );
        assert_eq!(
            locals
                .get_item("str_val")
                .unwrap()
                .unwrap()
                .extract::<String>()
                .unwrap(),
            "hello"
        );
        assert!(locals
            .get_item("bool_val")
            .unwrap()
            .unwrap()
            .extract::<bool>()
            .unwrap());
        assert_eq!(
            locals
                .get_item("list_val")
                .unwrap()
                .unwrap()
                .extract::<Vec<i64>>()
                .unwrap(),
            vec![1, 2, 3]
        );
    });
}
