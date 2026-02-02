//! Compiler Performance Benchmarks
//!
//! This module benchmarks the Aria compiler pipeline:
//! - Lexer throughput (tokens/second)
//! - Parser throughput (AST nodes/second)
//! - MIR lowering speed
//! - Codegen compilation speed

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// Include all benchmark fixtures as compile-time strings for reproducibility
const SMALL_PROGRAM: &str = include_str!("../fixtures/small.aria");
const MEDIUM_PROGRAM: &str = include_str!("../fixtures/medium.aria");
const LARGE_PROGRAM: &str = include_str!("../fixtures/large.aria");

// ============================================================================
// Lexer Benchmarks
// ============================================================================

fn bench_lexer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer_throughput");

    let programs = [
        ("small", SMALL_PROGRAM),
        ("medium", MEDIUM_PROGRAM),
        ("large", LARGE_PROGRAM),
    ];

    for (name, source) in programs.iter() {
        let bytes = source.len() as u64;
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(
            BenchmarkId::new("tokenize", name),
            source,
            |b, source| {
                b.iter(|| {
                    let lexer = aria_lexer::Lexer::new(black_box(source));
                    let (tokens, _errors) = lexer.tokenize();
                    black_box(tokens)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("tokenize_filtered", name),
            source,
            |b, source| {
                b.iter(|| {
                    let lexer = aria_lexer::Lexer::new(black_box(source));
                    let (tokens, _errors) = lexer.tokenize_filtered();
                    black_box(tokens)
                })
            },
        );
    }

    group.finish();
}

fn bench_lexer_token_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer_token_count");

    let programs = [
        ("small", SMALL_PROGRAM),
        ("medium", MEDIUM_PROGRAM),
        ("large", LARGE_PROGRAM),
    ];

    for (name, source) in programs.iter() {
        // Count tokens first for throughput measurement
        let lexer = aria_lexer::Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let token_count = tokens.len() as u64;

        group.throughput(Throughput::Elements(token_count));

        group.bench_with_input(BenchmarkId::from_parameter(name), source, |b, source| {
            b.iter(|| {
                let lexer = aria_lexer::Lexer::new(black_box(source));
                let (tokens, _errors) = lexer.tokenize();
                black_box(tokens)
            })
        });
    }

    group.finish();
}

// ============================================================================
// Parser Benchmarks
// ============================================================================

fn bench_parser_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_throughput");

    let programs = [
        ("small", SMALL_PROGRAM),
        ("medium", MEDIUM_PROGRAM),
        ("large", LARGE_PROGRAM),
    ];

    for (name, source) in programs.iter() {
        let bytes = source.len() as u64;
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(
            BenchmarkId::new("parse_program", name),
            source,
            |b, source| {
                b.iter(|| {
                    let mut parser = aria_parser::Parser::new(black_box(source));
                    let program = parser.parse_program();
                    black_box(program)
                })
            },
        );
    }

    group.finish();
}

fn bench_parser_ast_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_ast_nodes");

    let programs = [
        ("small", SMALL_PROGRAM),
        ("medium", MEDIUM_PROGRAM),
        ("large", LARGE_PROGRAM),
    ];

    for (name, source) in programs.iter() {
        // Parse once to count AST items
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();
        let item_count = program.map(|p| p.items.len() as u64).unwrap_or(0);

        group.throughput(Throughput::Elements(item_count));

        group.bench_with_input(BenchmarkId::from_parameter(name), source, |b, source| {
            b.iter(|| {
                let mut parser = aria_parser::Parser::new(black_box(source));
                let program = parser.parse_program();
                black_box(program)
            })
        });
    }

    group.finish();
}

// ============================================================================
// MIR Lowering Benchmarks
// ============================================================================

fn bench_mir_lowering(c: &mut Criterion) {
    let mut group = c.benchmark_group("mir_lowering");

    let programs = [
        ("small", SMALL_PROGRAM),
        ("medium", MEDIUM_PROGRAM),
        ("large", LARGE_PROGRAM),
    ];

    for (name, source) in programs.iter() {
        // Pre-parse the AST
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();

        if let Ok(program) = program {
            let bytes = source.len() as u64;
            group.throughput(Throughput::Bytes(bytes));

            group.bench_with_input(
                BenchmarkId::from_parameter(name),
                &program,
                |b, program| {
                    b.iter(|| {
                        let mir = aria_mir::lower_program(black_box(program));
                        black_box(mir)
                    })
                },
            );
        }
    }

    group.finish();
}

// ============================================================================
// Codegen Benchmarks
// ============================================================================

fn bench_codegen_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("codegen_compilation");
    // Codegen is slow, reduce sample count
    group.sample_size(20);

    let programs = [
        ("small", SMALL_PROGRAM),
        ("medium", MEDIUM_PROGRAM),
        // Skip large for codegen - too slow for benchmarking
    ];

    for (name, source) in programs.iter() {
        // Pre-parse and lower to MIR
        let mut parser = aria_parser::Parser::new(source);
        let program = parser.parse_program();

        if let Ok(program) = program {
            if let Ok(mir) = aria_mir::lower_program(&program) {
                let bytes = source.len() as u64;
                group.throughput(Throughput::Bytes(bytes));

                group.bench_with_input(BenchmarkId::from_parameter(name), &mir, |b, mir| {
                    b.iter(|| {
                        let result = aria_codegen::compile_to_object(
                            black_box(mir),
                            aria_codegen::Target::native(),
                        );
                        black_box(result)
                    })
                });
            }
        }
    }

    group.finish();
}

// ============================================================================
// Full Pipeline Benchmarks
// ============================================================================

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    let programs = [
        ("small", SMALL_PROGRAM),
        ("medium", MEDIUM_PROGRAM),
        ("large", LARGE_PROGRAM),
    ];

    for (name, source) in programs.iter() {
        let bytes = source.len() as u64;
        group.throughput(Throughput::Bytes(bytes));

        // Lex + Parse
        group.bench_with_input(
            BenchmarkId::new("lex_and_parse", name),
            source,
            |b, source| {
                b.iter(|| {
                    let mut parser = aria_parser::Parser::new(black_box(source));
                    let program = parser.parse_program();
                    black_box(program)
                })
            },
        );

        // Lex + Parse + MIR
        group.bench_with_input(
            BenchmarkId::new("lex_parse_mir", name),
            source,
            |b, source| {
                b.iter(|| {
                    let mut parser = aria_parser::Parser::new(black_box(source));
                    if let Ok(program) = parser.parse_program() {
                        let mir = aria_mir::lower_program(&program);
                        black_box(mir);
                    }
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Stress Tests
// ============================================================================

/// Generate a synthetic program with many functions
fn generate_many_functions(count: usize) -> String {
    let mut source = String::new();
    for i in 0..count {
        source.push_str(&format!(
            r#"fn func_{i}(x: Int) -> Int
  x + {i}
end

"#
        ));
    }
    source.push_str(
        r#"fn main()
  let result = 0
  print(result)
end
"#,
    );
    source
}

/// Generate a synthetic program with deeply nested expressions
fn generate_deep_nesting(depth: usize) -> String {
    let mut expr = String::from("1");
    for _ in 0..depth {
        expr = format!("({expr} + 1)");
    }
    format!(
        r#"fn main()
  let x = {expr}
  print(x)
end
"#
    )
}

/// Generate a synthetic program with many let bindings
fn generate_many_bindings(count: usize) -> String {
    let mut source = String::from("fn main()\n");
    for i in 0..count {
        source.push_str(&format!("  let var_{i} = {i}\n"));
    }
    source.push_str(&format!(
        "  let result = var_0\n  print(result)\nend\n"
    ));
    source
}

fn bench_stress_tests(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_tests");

    // Many functions
    let function_counts = [10, 50, 100, 200];
    for &count in &function_counts {
        let source = generate_many_functions(count);
        let bytes = source.len() as u64;

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::new("many_functions", count),
            &source,
            |b, source| {
                b.iter(|| {
                    let mut parser = aria_parser::Parser::new(black_box(source));
                    let program = parser.parse_program();
                    black_box(program)
                })
            },
        );
    }

    // Deep nesting
    let nesting_depths = [10, 25, 50, 100];
    for &depth in &nesting_depths {
        let source = generate_deep_nesting(depth);
        let bytes = source.len() as u64;

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::new("deep_nesting", depth),
            &source,
            |b, source| {
                b.iter(|| {
                    let mut parser = aria_parser::Parser::new(black_box(source));
                    let program = parser.parse_program();
                    black_box(program)
                })
            },
        );
    }

    // Many bindings
    let binding_counts = [10, 50, 100, 200];
    for &count in &binding_counts {
        let source = generate_many_bindings(count);
        let bytes = source.len() as u64;

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::new("many_bindings", count),
            &source,
            |b, source| {
                b.iter(|| {
                    let mut parser = aria_parser::Parser::new(black_box(source));
                    let program = parser.parse_program();
                    black_box(program)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Criterion Main
// ============================================================================

criterion_group!(
    benches,
    bench_lexer_throughput,
    bench_lexer_token_count,
    bench_parser_throughput,
    bench_parser_ast_nodes,
    bench_mir_lowering,
    bench_codegen_compilation,
    bench_full_pipeline,
    bench_stress_tests,
);

criterion_main!(benches);
