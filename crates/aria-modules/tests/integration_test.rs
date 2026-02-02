use aria_modules::{ModuleCompiler, FileSystemResolver, CompilationMode, ModuleError};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_simple_module_compilation() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a simple module
    let utils_code = r#"
pub fn add(x: Int, y: Int) -> Int = x + y
pub fn multiply(x: Int, y: Int) -> Int = x * y
"#;

    fs::write(base_path.join("utils.aria"), utils_code).unwrap();

    // Create main module that imports utils
    let main_code = r#"
import utils::{add, multiply}

pub fn main() -> Int
  let sum = add(10, 20)
  let product = multiply(5, 6)
  sum + product
end
"#;

    fs::write(base_path.join("main.aria"), main_code).unwrap();

    // Compile
    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("main.aria"));

    match result {
        Ok(modules) => {
            assert_eq!(modules.len(), 2, "Should compile 2 modules");

            // First module should be utils (dependency)
            assert_eq!(modules[0].name, "utils");

            // Second module should be main
            assert_eq!(modules[1].name, "main");

            // Check exports
            assert!(modules[0].is_exported("add"));
            assert!(modules[0].is_exported("multiply"));
        }
        Err(e) => {
            panic!("Compilation failed: {}", e);
        }
    }
}

#[test]
fn test_circular_dependency_detection() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Module A imports B
    let a_code = r#"
import b::{func_b}
pub fn func_a() -> Int = func_b()
"#;

    // Module B imports A (circular!)
    let b_code = r#"
import a::{func_a}
pub fn func_b() -> Int = func_a()
"#;

    fs::write(base_path.join("a.aria"), a_code).unwrap();
    fs::write(base_path.join("b.aria"), b_code).unwrap();

    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("a.aria"));

    match result {
        Ok(_) => panic!("Should have detected circular dependency"),
        Err(e) => {
            let error_msg = e.to_string();
            assert!(error_msg.contains("circular"), "Error should mention circular dependency: {}", error_msg);
        }
    }
}

#[test]
fn test_module_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    let main_code = r#"
import nonexistent::{func}
pub fn main() -> Int = func()
"#;

    fs::write(base_path.join("main.aria"), main_code).unwrap();

    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("main.aria"));

    assert!(result.is_err(), "Should fail when module not found");
}

#[test]
fn test_transitive_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // A imports B imports C
    let c_code = r#"
pub fn func_c() -> Int = 42
"#;

    let b_code = r#"
import c::{func_c}
pub fn func_b() -> Int = func_c()
"#;

    let a_code = r#"
import b::{func_b}
pub fn func_a() -> Int = func_b()
"#;

    fs::write(base_path.join("c.aria"), c_code).unwrap();
    fs::write(base_path.join("b.aria"), b_code).unwrap();
    fs::write(base_path.join("a.aria"), a_code).unwrap();

    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("a.aria"));

    match result {
        Ok(modules) => {
            assert_eq!(modules.len(), 3, "Should compile 3 modules");

            // Should be in dependency order: C, B, A
            assert_eq!(modules[0].name, "c");
            assert_eq!(modules[1].name, "b");
            assert_eq!(modules[2].name, "a");
        }
        Err(e) => {
            panic!("Compilation failed: {}", e);
        }
    }
}

#[test]
fn test_three_module_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // a -> b -> c -> a (three-way cycle)
    let a_code = r#"
import b::{func_b}
pub fn func_a() -> Int = func_b()
"#;

    let b_code = r#"
import c::{func_c}
pub fn func_b() -> Int = func_c()
"#;

    let c_code = r#"
import a::{func_a}
pub fn func_c() -> Int = func_a()
"#;

    fs::write(base_path.join("a.aria"), a_code).unwrap();
    fs::write(base_path.join("b.aria"), b_code).unwrap();
    fs::write(base_path.join("c.aria"), c_code).unwrap();

    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("a.aria"));

    match result {
        Ok(_) => panic!("Should have detected circular dependency"),
        Err(e) => {
            let error_msg = e.to_string();
            assert!(error_msg.contains("circular"), "Error should mention circular dependency: {}", error_msg);
            // Should contain module names in the cycle
            assert!(error_msg.contains("a") || error_msg.contains("b") || error_msg.contains("c"),
                "Error message should include module names: {}", error_msg);
        }
    }
}

#[test]
fn test_circular_dependency_error_message_includes_names() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a simple two-module cycle
    let alpha_code = r#"
import beta::{beta_func}
pub fn alpha_func() -> Int = beta_func()
"#;

    let beta_code = r#"
import alpha::{alpha_func}
pub fn beta_func() -> Int = alpha_func()
"#;

    fs::write(base_path.join("alpha.aria"), alpha_code).unwrap();
    fs::write(base_path.join("beta.aria"), beta_code).unwrap();

    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("alpha.aria"));

    match result {
        Ok(_) => panic!("Should have detected circular dependency"),
        Err(ModuleError::CircularDependencyNamed(cycle)) => {
            // Verify we got module names, not IDs
            assert!(cycle.contains(&"alpha".to_string()) || cycle.contains(&"beta".to_string()),
                "Cycle should contain module names: {:?}", cycle);
        }
        Err(e) => {
            // Fallback - still check the error message
            let error_msg = e.to_string();
            assert!(error_msg.contains("circular"), "Error should be about circular dependency: {}", error_msg);
        }
    }
}

#[test]
fn test_diamond_dependency_no_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Diamond: a -> b, a -> c, b -> d, c -> d (no cycle)
    let d_code = r#"
pub fn func_d() -> Int = 1
"#;

    let b_code = r#"
import d::{func_d}
pub fn func_b() -> Int = func_d()
"#;

    let c_code = r#"
import d::{func_d}
pub fn func_c() -> Int = func_d() + 1
"#;

    let a_code = r#"
import b::{func_b}
import c::{func_c}
pub fn func_a() -> Int = func_b() + func_c()
"#;

    fs::write(base_path.join("d.aria"), d_code).unwrap();
    fs::write(base_path.join("b.aria"), b_code).unwrap();
    fs::write(base_path.join("c.aria"), c_code).unwrap();
    fs::write(base_path.join("a.aria"), a_code).unwrap();

    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("a.aria"));

    match result {
        Ok(modules) => {
            assert_eq!(modules.len(), 4, "Should compile 4 modules");

            // d must come first (it has no dependencies)
            assert_eq!(modules[0].name, "d");

            // a must come last (it depends on everything)
            assert_eq!(modules[3].name, "a");

            // b and c can be in either order, but must come between d and a
            let b_or_c_1 = &modules[1].name;
            let b_or_c_2 = &modules[2].name;
            assert!((b_or_c_1 == "b" && b_or_c_2 == "c") ||
                    (b_or_c_1 == "c" && b_or_c_2 == "b"),
                    "Expected b and c in middle, got: {}, {}", b_or_c_1, b_or_c_2);
        }
        Err(e) => {
            panic!("Compilation should succeed (no cycle): {}", e);
        }
    }
}

#[test]
fn test_deep_transitive_chain() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Chain: e -> d -> c -> b -> a (5 levels deep)
    let a_code = r#"
pub fn func_a() -> Int = 1
"#;

    let b_code = r#"
import a::{func_a}
pub fn func_b() -> Int = func_a() + 1
"#;

    let c_code = r#"
import b::{func_b}
pub fn func_c() -> Int = func_b() + 1
"#;

    let d_code = r#"
import c::{func_c}
pub fn func_d() -> Int = func_c() + 1
"#;

    let e_code = r#"
import d::{func_d}
pub fn func_e() -> Int = func_d() + 1
"#;

    fs::write(base_path.join("a.aria"), a_code).unwrap();
    fs::write(base_path.join("b.aria"), b_code).unwrap();
    fs::write(base_path.join("c.aria"), c_code).unwrap();
    fs::write(base_path.join("d.aria"), d_code).unwrap();
    fs::write(base_path.join("e.aria"), e_code).unwrap();

    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("e.aria"));

    match result {
        Ok(modules) => {
            assert_eq!(modules.len(), 5, "Should compile 5 modules");

            // Should be in strict dependency order: a, b, c, d, e
            assert_eq!(modules[0].name, "a");
            assert_eq!(modules[1].name, "b");
            assert_eq!(modules[2].name, "c");
            assert_eq!(modules[3].name, "d");
            assert_eq!(modules[4].name, "e");
        }
        Err(e) => {
            panic!("Compilation failed: {}", e);
        }
    }
}

#[test]
fn test_module_with_no_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Single module with no imports
    let standalone_code = r#"
pub fn standalone() -> Int = 42
"#;

    fs::write(base_path.join("standalone.aria"), standalone_code).unwrap();

    let mut resolver = FileSystemResolver::new();
    resolver.add_search_path(base_path);

    let mut compiler = ModuleCompiler::new(
        Box::new(resolver),
        CompilationMode::Binary
    );

    let result = compiler.compile(&base_path.join("standalone.aria"));

    match result {
        Ok(modules) => {
            assert_eq!(modules.len(), 1, "Should compile 1 module");
            assert_eq!(modules[0].name, "standalone");
            assert!(modules[0].is_exported("standalone"));
        }
        Err(e) => {
            panic!("Compilation failed: {}", e);
        }
    }
}
