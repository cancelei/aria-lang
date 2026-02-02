//! Runtime Performance Benchmarks
//!
//! This module benchmarks the Aria interpreter execution speed
//! with various workloads.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// Include benchmark fixtures
const SMALL_PROGRAM: &str = include_str!("../fixtures/small.aria");
const MEDIUM_PROGRAM: &str = include_str!("../fixtures/medium.aria");

// ============================================================================
// Interpreter Benchmarks
// ============================================================================

fn bench_interpreter_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpreter_execution");

    // Programs that are safe to run (don't have excessive recursion)
    let programs = [
        ("small", SMALL_PROGRAM),
        // Note: medium and large have recursive fibonacci which is too slow for benchmarking
    ];

    for (name, source) in programs.iter() {
        // Pre-parse the program
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();

        if let Ok(program) = program {
            group.bench_with_input(BenchmarkId::from_parameter(name), &program, |b, program| {
                b.iter(|| {
                    let mut interpreter = aria_interpreter::Interpreter::new();
                    let result = interpreter.run(black_box(program));
                    black_box(result)
                })
            });
        }
    }

    group.finish();
}

// ============================================================================
// Specific Algorithm Benchmarks
// ============================================================================

fn bench_interpreter_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpreter_algorithms");

    // Factorial benchmark
    let factorial_program = r#"
fn factorial(n: Int) -> Int
  if n <= 1
    1
  else
    n * factorial(n - 1)
  end
end

fn main()
  let result = factorial(10)
  print(result)
end
"#;

    // Fibonacci with memoization (iterative-style)
    let fib_iterative = r#"
fn fib_iter(n: Int, a: Int, b: Int) -> Int
  if n <= 0
    a
  else
    fib_iter(n - 1, b, a + b)
  end
end

fn main()
  let result = fib_iter(30, 0, 1)
  print(result)
end
"#;

    // Sum to N (tail recursive)
    let sum_to_n = r#"
fn sum_helper(n: Int, acc: Int) -> Int
  if n <= 0
    acc
  else
    sum_helper(n - 1, acc + n)
  end
end

fn main()
  let result = sum_helper(100, 0)
  print(result)
end
"#;

    // GCD benchmark
    let gcd_program = r#"
fn modulo(a: Int, b: Int) -> Int
  a % b
end

fn gcd(a: Int, b: Int) -> Int
  if b == 0
    a
  else
    gcd(b, modulo(a, b))
  end
end

fn main()
  let result = gcd(123456789, 987654321)
  print(result)
end
"#;

    // Loop simulation (using recursion)
    let loop_simulation = r#"
fn loop_helper(count: Int, acc: Int) -> Int
  if count <= 0
    acc
  else
    loop_helper(count - 1, acc + count * 2)
  end
end

fn main()
  let result = loop_helper(1000, 0)
  print(result)
end
"#;

    // Nested function calls
    let nested_calls = r#"
fn add1(x: Int) -> Int
  x + 1
end

fn add2(x: Int) -> Int
  add1(add1(x))
end

fn add4(x: Int) -> Int
  add2(add2(x))
end

fn add8(x: Int) -> Int
  add4(add4(x))
end

fn add16(x: Int) -> Int
  add8(add8(x))
end

fn main()
  let result = add16(0)
  print(result)
end
"#;

    // Arithmetic intensive
    let arithmetic_heavy = r#"
fn compute(x: Int, iterations: Int, acc: Int) -> Int
  if iterations <= 0
    acc
  else
    let new_acc = (acc * 3 + x * 2 - 1) % 1000000
    compute(x, iterations - 1, new_acc)
  end
end

fn main()
  let result = compute(42, 100, 0)
  print(result)
end
"#;

    let algorithms = [
        ("factorial_10", factorial_program),
        ("fib_iterative_30", fib_iterative),
        ("sum_to_100", sum_to_n),
        ("gcd_large", gcd_program),
        ("loop_1000", loop_simulation),
        ("nested_calls", nested_calls),
        ("arithmetic_heavy", arithmetic_heavy),
    ];

    for (name, source) in algorithms.iter() {
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();

        if let Ok(program) = program {
            group.bench_with_input(BenchmarkId::from_parameter(name), &program, |b, program| {
                b.iter(|| {
                    let mut interpreter = aria_interpreter::Interpreter::new();
                    let result = interpreter.run(black_box(program));
                    black_box(result)
                })
            });
        }
    }

    group.finish();
}

// ============================================================================
// Expression Evaluation Benchmarks
// ============================================================================

fn bench_expression_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("expression_evaluation");

    // Simple arithmetic expression
    let simple_arithmetic = r#"
fn main()
  let x = 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10
  print(x)
end
"#;

    // Complex arithmetic expression
    let complex_arithmetic = r#"
fn main()
  let x = ((1 + 2) * 3 - 4) / 2 + ((5 * 6) - (7 + 8)) * 2
  print(x)
end
"#;

    // Boolean expressions
    let boolean_expr = r#"
fn main()
  let a = true and false or true and not false
  let b = 1 < 2 and 3 > 2 or 4 == 4
  let c = 10 >= 5 and 3 <= 5 and 7 != 8
  print(1)
end
"#;

    // Comparison chains
    let comparisons = r#"
fn test_comparisons(x: Int) -> Bool
  x > 0 and x < 100 and x != 50
end

fn main()
  let r1 = test_comparisons(25)
  let r2 = test_comparisons(75)
  let r3 = test_comparisons(50)
  print(1)
end
"#;

    let expressions = [
        ("simple_arithmetic", simple_arithmetic),
        ("complex_arithmetic", complex_arithmetic),
        ("boolean_expr", boolean_expr),
        ("comparisons", comparisons),
    ];

    for (name, source) in expressions.iter() {
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();

        if let Ok(program) = program {
            group.bench_with_input(BenchmarkId::from_parameter(name), &program, |b, program| {
                b.iter(|| {
                    let mut interpreter = aria_interpreter::Interpreter::new();
                    let result = interpreter.run(black_box(program));
                    black_box(result)
                })
            });
        }
    }

    group.finish();
}

// ============================================================================
// Control Flow Benchmarks
// ============================================================================

fn bench_control_flow(c: &mut Criterion) {
    let mut group = c.benchmark_group("control_flow");

    // If-else chain
    let if_else_chain = r#"
fn classify(n: Int) -> Int
  if n < 0
    0 - 1
  elsif n == 0
    0
  elsif n < 10
    1
  elsif n < 100
    2
  else
    3
  end
end

fn test_classify(iterations: Int, acc: Int) -> Int
  if iterations <= 0
    acc
  else
    let r = classify(iterations % 200 - 100)
    test_classify(iterations - 1, acc + r)
  end
end

fn main()
  let result = test_classify(100, 0)
  print(result)
end
"#;

    // Deep recursion (but not too deep)
    let deep_recursion = r#"
fn recurse(depth: Int) -> Int
  if depth <= 0
    0
  else
    1 + recurse(depth - 1)
  end
end

fn main()
  let result = recurse(50)
  print(result)
end
"#;

    // Multiple function calls
    let multi_call = r#"
fn f1(x: Int) -> Int
  x + 1
end

fn f2(x: Int) -> Int
  f1(x) * 2
end

fn f3(x: Int) -> Int
  f2(x) + f1(x)
end

fn f4(x: Int) -> Int
  f3(x) * f2(x)
end

fn main()
  let r1 = f4(1)
  let r2 = f4(2)
  let r3 = f4(3)
  let r4 = f4(4)
  let r5 = f4(5)
  print(r1 + r2 + r3 + r4 + r5)
end
"#;

    let programs = [
        ("if_else_chain", if_else_chain),
        ("deep_recursion_50", deep_recursion),
        ("multi_function_calls", multi_call),
    ];

    for (name, source) in programs.iter() {
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();

        if let Ok(program) = program {
            group.bench_with_input(BenchmarkId::from_parameter(name), &program, |b, program| {
                b.iter(|| {
                    let mut interpreter = aria_interpreter::Interpreter::new();
                    let result = interpreter.run(black_box(program));
                    black_box(result)
                })
            });
        }
    }

    group.finish();
}

// ============================================================================
// Environment/Scope Benchmarks
// ============================================================================

fn bench_environment(c: &mut Criterion) {
    let mut group = c.benchmark_group("environment");

    // Many local variables
    let many_locals = r#"
fn main()
  let a = 1
  let b = 2
  let c = 3
  let d = 4
  let e = 5
  let f = 6
  let g = 7
  let h = 8
  let i = 9
  let j = 10
  let k = 11
  let l = 12
  let m = 13
  let n = 14
  let o = 15
  let p = 16
  let q = 17
  let r = 18
  let s = 19
  let t = 20
  let result = a + b + c + d + e + f + g + h + i + j + k + l + m + n + o + p + q + r + s + t
  print(result)
end
"#;

    // Variable shadowing
    let shadowing = r#"
fn shadow_test() -> Int
  let x = 1
  let x = x + 1
  let x = x + 1
  let x = x + 1
  let x = x + 1
  let x = x + 1
  let x = x + 1
  let x = x + 1
  let x = x + 1
  let x = x + 1
  x
end

fn main()
  let result = shadow_test()
  print(result)
end
"#;

    // Closure-like patterns (functions capturing outer scope)
    let nested_scope = r#"
fn outer(x: Int) -> Int
  fn inner(y: Int) -> Int
    x + y
  end
  inner(10) + inner(20) + inner(30)
end

fn main()
  let r1 = outer(1)
  let r2 = outer(2)
  let r3 = outer(3)
  print(r1 + r2 + r3)
end
"#;

    let programs = [
        ("many_locals", many_locals),
        ("shadowing", shadowing),
        // ("nested_scope", nested_scope), // May not be supported yet
    ];

    for (name, source) in programs.iter() {
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();

        if let Ok(program) = program {
            group.bench_with_input(BenchmarkId::from_parameter(name), &program, |b, program| {
                b.iter(|| {
                    let mut interpreter = aria_interpreter::Interpreter::new();
                    let result = interpreter.run(black_box(program));
                    black_box(result)
                })
            });
        }
    }

    group.finish();
}

// ============================================================================
// Criterion Main
// ============================================================================

criterion_group!(
    benches,
    bench_interpreter_execution,
    bench_interpreter_algorithms,
    bench_expression_evaluation,
    bench_control_flow,
    bench_environment,
);

criterion_main!(benches);
