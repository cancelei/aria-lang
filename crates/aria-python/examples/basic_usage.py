#!/usr/bin/env python3
"""
Basic usage examples for aria-python bindings.

This demonstrates how to use the Aria programming language from Python.
"""

import aria_python as aria


def example_basic_arithmetic():
    """Example: Basic arithmetic operations."""
    print("=== Basic Arithmetic ===")

    interp = aria.AriaInterpreter()
    result = interp.eval("""
    fn main() -> Int {
        let x = 10;
        let y = 32;
        return x + y;
    }
    """)

    print(f"10 + 32 = {result}")
    print()


def example_string_operations():
    """Example: Working with strings."""
    print("=== String Operations ===")

    interp = aria.AriaInterpreter()
    result = interp.eval("""
    fn main() -> String {
        return "Hello from Aria!";
    }
    """)

    print(f"Aria says: {result}")
    print()


def example_arrays():
    """Example: Working with arrays."""
    print("=== Arrays ===")

    interp = aria.AriaInterpreter()

    # Create an array in Aria
    result = interp.eval("""
    fn main() -> Array {
        return [1, 2, 3, 4, 5];
    }
    """)

    print(f"Array from Aria: {result}")
    print(f"Sum in Python: {sum(result)}")
    print()


def example_function_calls():
    """Example: Calling Aria functions from Python."""
    print("=== Function Calls ===")

    interp = aria.AriaInterpreter()

    # Define functions in Aria
    interp.exec("""
    fn add(a: Int, b: Int) -> Int {
        return a + b;
    }

    fn multiply(a: Int, b: Int) -> Int {
        return a * b;
    }

    fn main() -> Int {
        return 0;
    }
    """)

    # Call them from Python
    sum_result = interp.call_function("add", (10, 20))
    product_result = interp.call_function("multiply", (6, 7))

    print(f"add(10, 20) = {sum_result}")
    print(f"multiply(6, 7) = {product_result}")
    print()


def example_global_variables():
    """Example: Setting and getting global variables."""
    print("=== Global Variables ===")

    interp = aria.AriaInterpreter()

    # Set variables from Python
    interp.set_global("x", 100)
    interp.set_global("y", 200)
    interp.set_global("name", "Python")

    # Use them in Aria
    result = interp.eval("""
    fn main() -> Int {
        return x + y;
    }
    """)

    print(f"x + y = {result}")
    print(f"name = {interp.get_global('name')}")
    print()


def example_fibonacci():
    """Example: Computing Fibonacci numbers."""
    print("=== Fibonacci Sequence ===")

    interp = aria.AriaInterpreter()

    # Define fibonacci function in Aria
    interp.exec("""
    fn fibonacci(n: Int) -> Int {
        if n <= 1 {
            return n;
        }
        return fibonacci(n - 1) + fibonacci(n - 2);
    }

    fn main() -> Int {
        return 0;
    }
    """)

    # Compute fibonacci numbers from Python
    for i in range(11):
        fib = interp.call_function("fibonacci", (i,))
        print(f"fibonacci({i}) = {fib}")
    print()


def example_type_conversions():
    """Example: Various type conversions between Python and Aria."""
    print("=== Type Conversions ===")

    interp = aria.AriaInterpreter()

    # Set various Python types
    interp.set_global("py_int", 42)
    interp.set_global("py_float", 3.14159)
    interp.set_global("py_str", "hello world")
    interp.set_global("py_bool", True)
    interp.set_global("py_list", [1, 2, 3, 4, 5])
    interp.set_global("py_dict", {"a": 10, "b": 20, "c": 30})

    # Get them back
    print(f"int: {interp.get_global('py_int')} (type: {type(interp.get_global('py_int')).__name__})")
    print(f"float: {interp.get_global('py_float')} (type: {type(interp.get_global('py_float')).__name__})")
    print(f"str: {interp.get_global('py_str')} (type: {type(interp.get_global('py_str')).__name__})")
    print(f"bool: {interp.get_global('py_bool')} (type: {type(interp.get_global('py_bool')).__name__})")
    print(f"list: {interp.get_global('py_list')} (type: {type(interp.get_global('py_list')).__name__})")
    print(f"dict: {interp.get_global('py_dict')} (type: {type(interp.get_global('py_dict')).__name__})")
    print()


def example_list_globals():
    """Example: Listing all available functions."""
    print("=== Available Functions ===")

    interp = aria.AriaInterpreter()

    # Define some functions
    interp.exec("""
    fn greet(name: String) -> String {
        return "Hello, " + name + "!";
    }

    fn square(x: Int) -> Int {
        return x * x;
    }

    fn main() -> Int {
        return 0;
    }
    """)

    # List all globals
    globals_list = interp.list_globals()
    print(f"Available functions and variables: {', '.join(sorted(globals_list))}")
    print()


def example_convenience_functions():
    """Example: Using convenience functions."""
    print("=== Convenience Functions ===")

    # Use eval_aria convenience function
    result = aria.eval_aria("""
    fn main() -> Int {
        return 2 * 21;
    }
    """)

    print(f"Using eval_aria: {result}")

    # Use exec_aria convenience function
    aria.exec_aria("""
    fn main() -> Int {
        print("Hello from exec_aria!");
        return 0;
    }
    """)
    print()


def example_control_flow():
    """Example: Control flow structures."""
    print("=== Control Flow ===")

    interp = aria.AriaInterpreter()

    result = interp.eval("""
    fn main() -> String {
        let x = 15;

        if x > 10 {
            return "x is greater than 10";
        } else {
            return "x is not greater than 10";
        }
    }
    """)

    print(f"Result: {result}")
    print()


def example_loops():
    """Example: Loop constructs."""
    print("=== Loops ===")

    interp = aria.AriaInterpreter()

    # Sum of numbers using a loop
    result = interp.eval("""
    fn main() -> Int {
        let sum = 0;
        let i = 1;

        while i <= 10 {
            sum = sum + i;
            i = i + 1;
        }

        return sum;
    }
    """)

    print(f"Sum of 1..10 = {result}")
    print()


def main():
    """Run all examples."""
    print("=" * 60)
    print("Aria-Python Interoperability Examples")
    print("=" * 60)
    print()

    example_basic_arithmetic()
    example_string_operations()
    example_arrays()
    example_function_calls()
    example_global_variables()
    example_fibonacci()
    example_type_conversions()
    example_list_globals()
    example_convenience_functions()
    example_control_flow()
    example_loops()

    print("=" * 60)
    print("All examples completed successfully!")
    print("=" * 60)


if __name__ == "__main__":
    main()
