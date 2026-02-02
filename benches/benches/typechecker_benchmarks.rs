//! Type Checker Performance Benchmarks
//!
//! This module benchmarks the Aria type checker:
//! - Type inference throughput
//! - Unification performance
//! - Type resolution speed

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// Include all benchmark fixtures as compile-time strings for reproducibility
const SMALL_PROGRAM: &str = include_str!("../fixtures/small.aria");
const MEDIUM_PROGRAM: &str = include_str!("../fixtures/medium.aria");
const LARGE_PROGRAM: &str = include_str!("../fixtures/large.aria");

// ============================================================================
// Type Checker Benchmarks
// ============================================================================

fn bench_typechecker_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("typechecker_throughput");

    let programs = [
        ("small", SMALL_PROGRAM),
        ("medium", MEDIUM_PROGRAM),
        ("large", LARGE_PROGRAM),
    ];

    for (name, source) in programs.iter() {
        // Pre-parse the AST (we want to measure type checking, not parsing)
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();

        if let Ok(program) = program {
            let bytes = source.len() as u64;
            group.throughput(Throughput::Bytes(bytes));

            group.bench_with_input(
                BenchmarkId::new("check_program", name),
                &program,
                |b, program| {
                    b.iter(|| {
                        let mut checker = aria_types::TypeChecker::new();
                        let result = checker.check_program(black_box(program));
                        black_box(result)
                    })
                },
            );
        }
    }

    group.finish();
}

// ============================================================================
// Type Inference Stress Tests
// ============================================================================

/// Generate a program with many type annotations (tests resolve_type)
fn generate_many_type_annotations(count: usize) -> String {
    let mut source = String::new();
    for i in 0..count {
        source.push_str(&format!(
            r#"fn func_{i}(x: Int, y: Float, z: String) -> Bool
  true
end

"#
        ));
    }
    source.push_str(
        r#"fn main()
  print("done")
end
"#,
    );
    source
}

/// Generate a program with complex nested types
fn generate_nested_types(depth: usize) -> String {
    // Create nested array types: [[[...Int...]]]
    let mut type_str = "Int".to_string();
    for _ in 0..depth {
        type_str = format!("[{}]", type_str);
    }

    format!(
        r#"fn process_nested(data: {type_str}) -> Int
  0
end

fn main()
  print("done")
end
"#
    )
}

/// Generate a program with many local variables (tests environment lookups)
fn generate_many_locals(count: usize) -> String {
    let mut source = String::from("fn main()\n");
    for i in 0..count {
        source.push_str(&format!("  let var_{i}: Int = {i}\n"));
    }
    // Reference all variables to ensure type checking
    source.push_str("  let sum = var_0");
    for i in 1..count.min(10) {
        source.push_str(&format!(" + var_{i}"));
    }
    source.push_str("\n  print(sum)\nend\n");
    source
}

/// Generate a program with many function calls (tests inference)
fn generate_many_calls(count: usize) -> String {
    let mut source = String::new();

    // Define helper functions
    source.push_str(
        r#"fn add(a: Int, b: Int) -> Int
  a + b
end

fn multiply(a: Int, b: Int) -> Int
  a * b
end

fn main()
"#,
    );

    source.push_str("  let result = 0\n");
    for i in 0..count {
        if i % 2 == 0 {
            source.push_str(&format!("  let tmp_{i} = add({i}, {i})\n"));
        } else {
            source.push_str(&format!("  let tmp_{i} = multiply({i}, {i})\n"));
        }
    }
    source.push_str("  print(result)\nend\n");
    source
}

fn bench_typechecker_stress(c: &mut Criterion) {
    let mut group = c.benchmark_group("typechecker_stress");

    // Many type annotations
    let annotation_counts = [10, 50, 100];
    for &count in &annotation_counts {
        let source = generate_many_type_annotations(count);
        let mut parser = aria_parser::Parser::new(&source);

        if let Ok(program) = parser.parse_program() {
            let bytes = source.len() as u64;
            group.throughput(Throughput::Bytes(bytes));

            group.bench_with_input(
                BenchmarkId::new("type_annotations", count),
                &program,
                |b, program| {
                    b.iter(|| {
                        let mut checker = aria_types::TypeChecker::new();
                        let result = checker.check_program(black_box(program));
                        black_box(result)
                    })
                },
            );
        }
    }

    // Nested types
    let nesting_depths = [5, 10, 15];
    for &depth in &nesting_depths {
        let source = generate_nested_types(depth);
        let mut parser = aria_parser::Parser::new(&source);

        if let Ok(program) = parser.parse_program() {
            group.bench_with_input(
                BenchmarkId::new("nested_types", depth),
                &program,
                |b, program| {
                    b.iter(|| {
                        let mut checker = aria_types::TypeChecker::new();
                        let result = checker.check_program(black_box(program));
                        black_box(result)
                    })
                },
            );
        }
    }

    // Many local variables
    let local_counts = [10, 50, 100];
    for &count in &local_counts {
        let source = generate_many_locals(count);
        let mut parser = aria_parser::Parser::new(&source);

        if let Ok(program) = parser.parse_program() {
            let bytes = source.len() as u64;
            group.throughput(Throughput::Bytes(bytes));

            group.bench_with_input(
                BenchmarkId::new("local_variables", count),
                &program,
                |b, program| {
                    b.iter(|| {
                        let mut checker = aria_types::TypeChecker::new();
                        let result = checker.check_program(black_box(program));
                        black_box(result)
                    })
                },
            );
        }
    }

    // Many function calls
    let call_counts = [10, 50, 100];
    for &count in &call_counts {
        let source = generate_many_calls(count);
        let mut parser = aria_parser::Parser::new(&source);

        if let Ok(program) = parser.parse_program() {
            let bytes = source.len() as u64;
            group.throughput(Throughput::Bytes(bytes));

            group.bench_with_input(
                BenchmarkId::new("function_calls", count),
                &program,
                |b, program| {
                    b.iter(|| {
                        let mut checker = aria_types::TypeChecker::new();
                        let result = checker.check_program(black_box(program));
                        black_box(result)
                    })
                },
            );
        }
    }

    group.finish();
}

// ============================================================================
// Criterion Main
// ============================================================================

criterion_group!(
    benches,
    bench_typechecker_throughput,
    bench_typechecker_stress,
);

criterion_main!(benches);
