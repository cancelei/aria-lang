//! Aria Language Compiler
//!
//! Command-line interface for the Aria programming language.

use ariadne::{Color, Label, Report, ReportKind, Source};
use aria_parser::{parse, ParseError};
use aria_types::{TypeChecker, TypeError, ModuleExports};
use aria_modules::{ModuleCompiler, FileSystemResolver, CompilationMode, ModuleError};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

/// Get type conversion suggestions based on expected and found types
fn get_type_conversion_suggestion(expected: &str, found: &str) -> Option<String> {
    match (expected, found) {
        ("String", "Int") => Some("to_string(<value>)".to_string()),
        ("String", "Float") => Some("to_string(<value>)".to_string()),
        ("String", "Bool") => Some("to_string(<value>)".to_string()),
        ("Int", "String") => Some("to_int(<value>)".to_string()),
        ("Int", "Float") => Some("to_int(<value>)".to_string()),
        ("Float", "String") => Some("to_float(<value>)".to_string()),
        ("Float", "Int") => Some("to_float(<value>)".to_string()),
        _ => None,
    }
}

/// Find similar names in scope for "did you mean" suggestions
fn find_similar_names<'a>(name: &str, candidates: &'a [&str]) -> Vec<&'a str> {
    let mut matches: Vec<(&str, usize)> = candidates
        .iter()
        .filter_map(|&candidate| {
            let distance = levenshtein_distance(name, candidate);
            // Accept if distance is <= 2 or <= 30% of the name length
            let threshold = std::cmp::max(2, name.len() / 3);
            if distance <= threshold {
                Some((candidate, distance))
            } else {
                None
            }
        })
        .collect();

    matches.sort_by_key(|(_, d)| *d);
    matches.into_iter().map(|(n, _)| n).take(3).collect()
}

/// Simple Levenshtein distance calculation
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 { return n; }
    if n == 0 { return m; }

    let mut matrix = vec![vec![0; n + 1]; m + 1];

    for i in 0..=m { matrix[i][0] = i; }
    for j in 0..=n { matrix[0][j] = j; }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }

    matrix[m][n]
}

/// Known builtin function names for "did you mean" suggestions
const BUILTIN_NAMES: &[&str] = &[
    "print", "println", "len", "first", "last", "reverse", "push", "pop",
    "to_string", "to_int", "to_float", "type_of",
    "contains", "starts_with", "ends_with", "trim", "replace", "substring", "char_at",
    "to_upper", "to_lower", "abs", "min", "max", "sqrt", "pow", "sin", "cos", "tan",
    "floor", "ceil", "round",
];

#[derive(Parser)]
#[command(name = "aria")]
#[command(author = "Aria Team")]
#[command(version = "0.1.0")]
#[command(about = "The Aria programming language compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check source files for errors without compiling
    Check {
        /// Source file to check
        file: PathBuf,
    },
    /// Parse source file and display AST
    Parse {
        /// Source file to parse
        file: PathBuf,
        /// Show full AST details
        #[arg(short, long)]
        verbose: bool,
    },
    /// Lex source file and display tokens
    Lex {
        /// Source file to lex
        file: PathBuf,
    },
    /// Run an Aria program (future)
    Run {
        /// Source file to run
        file: PathBuf,
    },
    /// Compile an Aria program to native executable or object file
    Build {
        /// Source file to compile
        file: PathBuf,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Link with runtime to produce executable (default: object file only)
        #[arg(short, long)]
        link: bool,
        /// Build with optimizations
        #[arg(short, long)]
        release: bool,
        /// Path to aria_runtime.o (auto-detected if not specified)
        #[arg(long)]
        runtime: Option<PathBuf>,
        /// Compile as library (default: binary)
        #[arg(long)]
        lib: bool,
        /// Additional module search paths
        #[arg(short = 'L', long = "lib-path")]
        lib_paths: Vec<PathBuf>,
        /// Target platform (native, wasm32)
        #[arg(long, default_value = "native")]
        target: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file } => check_file(&file),
        Commands::Parse { file, verbose } => parse_file(&file, verbose),
        Commands::Lex { file } => lex_file(&file),
        Commands::Run { file } => run_file(&file),
        Commands::Build { file, output, link, release, runtime, lib, lib_paths, target } => {
            build_file(&file, output.as_deref(), link, release, runtime.as_deref(), lib, &lib_paths, &target)
        }
    }
}

/// Check a source file for parse and type errors
fn check_file(path: &PathBuf) -> ExitCode {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    let filename = path.display().to_string();
    let mut has_errors = false;

    // Parse the source
    let (program, parse_errors) = parse(&source);

    // Report parse errors
    for error in &parse_errors {
        has_errors = true;
        report_parse_error(&filename, &source, error);
    }

    // Type check if parsing succeeded (or partially succeeded)
    if parse_errors.is_empty() {
        let mut checker = TypeChecker::new();
        if let Err(type_error) = checker.check_program(&program) {
            has_errors = true;
            report_type_error(&filename, &source, &type_error);
        }
    }

    if has_errors {
        eprintln!("\nCompilation failed with errors.");
        ExitCode::FAILURE
    } else {
        println!("Check passed: {}", path.display());
        ExitCode::SUCCESS
    }
}

/// Parse a source file and display the AST
fn parse_file(path: &PathBuf, verbose: bool) -> ExitCode {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    let filename = path.display().to_string();
    let (program, parse_errors) = parse(&source);

    // Report any parse errors
    for error in &parse_errors {
        report_parse_error(&filename, &source, error);
    }

    // Display the AST
    if verbose {
        println!("{:#?}", program);
    } else {
        println!("Parsed {} items:", program.items.len());
        for item in &program.items {
            print_item_summary(item);
        }
    }

    if parse_errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Lex a source file and display tokens
fn lex_file(path: &PathBuf) -> ExitCode {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    let lexer = aria_lexer::Lexer::new(&source);
    let (tokens, lex_errors) = lexer.tokenize_filtered();

    println!("Tokens ({}):", tokens.len());
    for token in &tokens {
        println!("  {:?} @ {:?}", token.kind, token.span);
    }

    if !lex_errors.is_empty() {
        println!("\nLexer errors ({}):", lex_errors.len());
        for error in &lex_errors {
            println!("  {:?}", error);
        }
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Run an Aria program using the tree-walking interpreter
fn run_file(path: &PathBuf) -> ExitCode {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    let filename = path.display().to_string();

    // Parse the source
    let (program, parse_errors) = parse(&source);

    // Report parse errors
    if !parse_errors.is_empty() {
        for error in &parse_errors {
            report_parse_error(&filename, &source, error);
        }
        return ExitCode::FAILURE;
    }

    // Run the program
    let mut interpreter = aria_interpreter::Interpreter::new();
    match interpreter.run(&program) {
        Ok(_) => ExitCode::SUCCESS,
        Err(runtime_error) => {
            report_runtime_error(&filename, &source, &runtime_error);
            ExitCode::FAILURE
        }
    }
}

/// Report a runtime error using ariadne
fn report_runtime_error(filename: &str, source: &str, error: &aria_interpreter::RuntimeError) {
    let span = error.span();
    let span_range = span.start..span.end;

    Report::build(ReportKind::Error, filename, span_range.start)
        .with_message("Runtime error")
        .with_label(
            Label::new((filename, span_range))
                .with_message(error.to_string())
                .with_color(Color::Red),
        )
        .finish()
        .print((filename, Source::from(source)))
        .unwrap();
}

/// Compile an Aria program to a native object file, executable, or WASM
fn build_file(
    path: &PathBuf,
    output: Option<&std::path::Path>,
    link: bool,
    release: bool,
    runtime_path: Option<&std::path::Path>,
    is_lib: bool,
    lib_paths: &[PathBuf],
    target: &str,
) -> ExitCode {
    // Parse target
    let codegen_target = match target {
        "native" => aria_codegen::Target::native(),
        "wasm32" | "wasm" => aria_codegen::Target::Wasm32,
        other => {
            eprintln!("Error: Unknown target '{}'. Supported targets: native, wasm32", other);
            return ExitCode::FAILURE;
        }
    };

    let is_wasm = matches!(codegen_target, aria_codegen::Target::Wasm32);
    // Setup module resolver
    let mut resolver = FileSystemResolver::new();
    for lib_path in lib_paths {
        resolver.add_search_path(lib_path);
    }

    // Determine compilation mode
    let mode = if is_lib {
        CompilationMode::Library
    } else {
        CompilationMode::Binary
    };

    // Compile all modules starting from entry point
    let mut compiler = ModuleCompiler::new(Box::new(resolver), mode);
    let modules = match compiler.compile(path) {
        Ok(modules) => modules,
        Err(module_error) => {
            report_module_error(&module_error);
            return ExitCode::FAILURE;
        }
    };

    println!("Compiled {} module(s)", modules.len());
    for module in &modules {
        println!("  - {} ({})", module.name, module.path.display());
    }

    // Type check all modules in dependency order (dependencies first)
    // Build a map of module exports as we type check each module
    let mut all_exports: std::collections::HashMap<String, ModuleExports> = std::collections::HashMap::new();

    for module in &modules {
        let mut checker = TypeChecker::new();

        // Register exports from all previously type-checked modules
        // These are the modules that this one might import from
        for (name, exports) in &all_exports {
            checker.register_module_exports(name.clone(), exports.clone());
        }

        // Type check this module (will process imports internally)
        if let Err(type_error) = checker.check_program(&module.ast) {
            let source = fs::read_to_string(&module.path).unwrap_or_default();
            report_type_error(&module.path.display().to_string(), &source, &type_error);
            return ExitCode::FAILURE;
        }

        // Extract and store this module's exports for use by dependent modules
        let exports = checker.extract_exports(&module.ast);
        all_exports.insert(module.name.to_string(), exports);

        println!("  Type-checked: {} ({} exports)", module.name, all_exports.get(module.name.as_str()).map(|e| e.len()).unwrap_or(0));
    }

    // Use the last module (entry module) for MIR lowering
    let entry_module = modules.last().expect("No entry module");

    // Lower to MIR
    let mut mir = match aria_mir::lower_program(&entry_module.ast) {
        Ok(mir) => mir,
        Err(mir_error) => {
            eprintln!("MIR lowering error: {}", mir_error);
            return ExitCode::FAILURE;
        }
    };

    // Apply optimizations if in release mode
    if release {
        println!("Applying optimizations...");
        aria_mir::optimize(&mut mir, aria_mir::OptLevel::Aggressive);
    }

    // Compile to object file or WASM
    let object_bytes = match aria_codegen::compile_to_object(&mir, codegen_target) {
        Ok(bytes) => bytes,
        Err(codegen_error) => {
            eprintln!("Code generation error: {}", codegen_error);
            return ExitCode::FAILURE;
        }
    };

    // WASM target: output .wasm file directly (no linking)
    if is_wasm {
        let output_path = output.map(|p| p.to_path_buf()).unwrap_or_else(|| {
            let stem = path.file_stem().unwrap_or_default();
            path.with_file_name(format!("{}.wasm", stem.to_string_lossy()))
        });

        if let Err(e) = fs::write(&output_path, &object_bytes) {
            eprintln!("Error writing WASM file '{}': {}", output_path.display(), e);
            return ExitCode::FAILURE;
        }

        println!("Compiled {} -> {}", path.display(), output_path.display());
        println!("\nWASM module ready! Run with:");
        println!("  node --experimental-wasm-modules -e \"import('{}').then(m => console.log(m.aria_main()))\"", output_path.display());
        println!("  # or use a WASM runtime like wasmtime:");
        println!("  wasmtime {}", output_path.display());

        return ExitCode::SUCCESS;
    }

    // Determine output path
    let output_path = output.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        let stem = path.file_stem().unwrap_or_default();
        if link {
            // Executable: no extension on Unix, .exe on Windows
            #[cfg(target_os = "windows")]
            {
                path.with_file_name(format!("{}.exe", stem.to_string_lossy()))
            }
            #[cfg(not(target_os = "windows"))]
            {
                path.with_file_name(stem)
            }
        } else {
            // Object file: .o extension
            path.with_file_name(format!("{}.o", stem.to_string_lossy()))
        }
    });

    if link {
        // Link mode: create temporary object file, then link with runtime
        let temp_obj = path.with_file_name(format!(
            ".{}.o",
            path.file_stem().unwrap_or_default().to_string_lossy()
        ));

        // Write temporary object file
        if let Err(e) = fs::write(&temp_obj, &object_bytes) {
            eprintln!(
                "Error writing temporary object file '{}': {}",
                temp_obj.display(),
                e
            );
            return ExitCode::FAILURE;
        }

        // Find runtime library
        let runtime = match find_runtime_library(runtime_path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error locating runtime library: {}", e);
                eprintln!(
                    "Hint: Build the runtime with 'make' in crates/aria-runtime/c_runtime/"
                );
                let _ = fs::remove_file(&temp_obj);
                return ExitCode::FAILURE;
            }
        };

        // Link with gcc
        let link_result = std::process::Command::new("gcc")
            .arg(&runtime)
            .arg(&temp_obj)
            .arg("-o")
            .arg(&output_path)
            .arg("-lm") // Link math library for sqrt, sin, cos, etc.
            .output();

        // Clean up temporary object file
        let _ = fs::remove_file(&temp_obj);

        match link_result {
            Ok(output) if output.status.success() => {
                println!(
                    "Compiled and linked {} -> {}",
                    path.display(),
                    output_path.display()
                );
                println!("Executable ready to run: ./{}", output_path.display());
                ExitCode::SUCCESS
            }
            Ok(output) => {
                eprintln!("Linker error:");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("Failed to run linker (gcc): {}", e);
                eprintln!("Make sure gcc is installed and in your PATH");
                ExitCode::FAILURE
            }
        }
    } else {
        // Object file mode: write object file directly
        if let Err(e) = fs::write(&output_path, &object_bytes) {
            eprintln!(
                "Error writing object file '{}': {}",
                output_path.display(),
                e
            );
            return ExitCode::FAILURE;
        }

        println!("Compiled {} -> {}", path.display(), output_path.display());
        println!("\nTo create an executable, link with the Aria runtime:");
        println!(
            "  gcc aria_runtime.o {} -o {}",
            output_path.display(),
            output_path.with_extension("").display()
        );
        println!("\nOr compile with --link flag:");
        println!("  aria build {} --link", path.display());

        ExitCode::SUCCESS
    }
}

/// Find the Aria runtime library
fn find_runtime_library(explicit_path: Option<&std::path::Path>) -> Result<PathBuf, String> {
    // If explicit path provided, use it
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path.to_path_buf());
        } else {
            return Err(format!("Runtime library not found at: {}", path.display()));
        }
    }

    // Search in common locations
    let search_paths: Vec<Option<PathBuf>> = vec![
        // Relative to current directory
        Some(PathBuf::from("aria_runtime.o")),
        Some(PathBuf::from("crates/aria-runtime/c_runtime/aria_runtime.o")),
        // Relative to executable
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join("aria_runtime.o"))),
        std::env::current_exe().ok().and_then(|exe| {
            exe.parent()
                .map(|p| p.join("../crates/aria-runtime/c_runtime/aria_runtime.o"))
        }),
        // System locations
        Some(PathBuf::from("/usr/local/lib/aria/aria_runtime.o")),
        Some(PathBuf::from("/usr/lib/aria/aria_runtime.o")),
    ];

    for path in search_paths.into_iter().flatten() {
        if path.exists() {
            return Ok(path);
        }
    }

    Err("Runtime library (aria_runtime.o) not found".to_string())
}

/// Report a parse error using ariadne with enhanced messages
fn report_parse_error(filename: &str, source: &str, error: &ParseError) {
    match error {
        ParseError::UnexpectedToken {
            expected,
            found,
            span,
        } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E3001")
                .with_message("unexpected token")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected {}, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::UnexpectedEof { expected } => {
            let span_range = source.len().saturating_sub(1)..source.len();
            Report::build(ReportKind::Error, filename, span_range.start)
                .with_code("E3002")
                .with_message("unexpected end of file")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected {} here", expected))
                        .with_color(Color::Red),
                )
                .with_help("check for missing closing brackets, parentheses, or 'end' keywords")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::InvalidExpression { span, hint } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E3003")
                .with_message("invalid expression")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(hint)
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::InvalidPattern { span, hint } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E4001")
                .with_message("invalid pattern")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(hint)
                        .with_color(Color::Red),
                )
                .with_help("patterns can be: literals, variables, tuples, or struct patterns")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::InvalidType { span, hint } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E3004")
                .with_message("invalid type")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(hint)
                        .with_color(Color::Red),
                )
                .with_help("valid types: Int, Float, String, Bool, [T], (T1, T2), fn(Args) -> Ret")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::MissingEnd { span, construct, .. } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E3005")
                .with_message(format!("unclosed {}", construct))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("{} starts here but is never closed", construct))
                        .with_color(Color::Red),
                )
                .with_help(format!("add `end` keyword to close this {}", construct))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::InvalidFunction { span, hint } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E3006")
                .with_message("invalid function definition")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(hint)
                        .with_color(Color::Red),
                )
                .with_help("function syntax: fn name(params) -> ReturnType ... end")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::InvalidStructField { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E3007")
                .with_message("invalid struct field")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("expected field definition")
                        .with_color(Color::Red),
                )
                .with_help("struct fields should be: `name: Type`")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::InvalidEnumVariant { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E3008")
                .with_message("invalid enum variant")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("expected variant definition")
                        .with_color(Color::Red),
                )
                .with_help("enum variants can be: `Name` or `Name(Type1, Type2)`")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::InvalidMatchArm { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E4002")
                .with_message("invalid match arm")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("expected pattern => expression")
                        .with_color(Color::Red),
                )
                .with_help("match arm syntax: `pattern => expression`")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        ParseError::DuplicateDefinition { name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E1003")
                .with_message(format!("the name `{}` is defined multiple times", name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("`{}` redefined here", name))
                        .with_color(Color::Red),
                )
                .with_note("names must be unique within the same scope")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // Catch-all for any new parse error types
        _ => {
            eprintln!("Parse error: {}", error);
        }
    }
}

/// Report a type error using ariadne with enhanced messages and suggestions
fn report_type_error(filename: &str, source: &str, error: &TypeError) {
    match error {
        TypeError::Mismatch {
            expected,
            found,
            span,
            expected_source,
        } => {
            let span_range = span.start..span.end;
            let mut report = Report::build(ReportKind::Error, filename, span.start)
                .with_code("E0001")
                .with_message("type mismatch")
                .with_label(
                    Label::new((filename, span_range.clone()))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                );

            // Add conversion suggestion if applicable
            if let Some(suggestion) = get_type_conversion_suggestion(expected, found) {
                report = report.with_help(format!("consider converting: {}", suggestion));
            }

            report
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UndefinedVariable(name, span) => {
            let span_range = span.start..span.end;
            let similar = find_similar_names(name, BUILTIN_NAMES);

            let mut report = Report::build(ReportKind::Error, filename, span.start)
                .with_code("E1001")
                .with_message(format!("cannot find value `{}` in this scope", name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not found in this scope")
                        .with_color(Color::Red),
                );

            if !similar.is_empty() {
                let suggestions = similar.join("`, `");
                report = report.with_help(format!("did you mean `{}`?", suggestions));
            }

            report
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UndefinedType(name, span) => {
            let span_range = span.start..span.end;
            let builtin_types = &["Int", "Float", "String", "Bool", "Char", "Unit", "Array", "Optional", "Result"];
            let similar = find_similar_names(name, builtin_types);

            let mut report = Report::build(ReportKind::Error, filename, span.start)
                .with_code("E1002")
                .with_message(format!("cannot find type `{}` in this scope", name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not found in this scope")
                        .with_color(Color::Red),
                );

            if !similar.is_empty() {
                let suggestions = similar.join("`, `");
                report = report.with_help(format!("did you mean `{}`?", suggestions));
            }

            report
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::CannotInfer(span) => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E0002")
                .with_message("type annotations needed")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("cannot infer type for this expression")
                        .with_color(Color::Red),
                )
                .with_help("consider adding a type annotation: `let x: Type = ...`")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::RecursiveType(span) => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E0003")
                .with_message("recursive type detected")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("this type references itself infinitely")
                        .with_color(Color::Red),
                )
                .with_help("consider using a reference or Box to break the cycle")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::WrongTypeArity {
            expected,
            found,
            span,
        } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E0004")
                .with_message("wrong number of type arguments")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected {} type argument{}, found {}",
                            expected,
                            if *expected == 1 { "" } else { "s" },
                            found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::NonTransferCapture {
            var_name,
            var_type,
            span,
        } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E6001")
                .with_message("cannot spawn task with non-Transfer capture")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("variable `{}` of type `{}` cannot be transferred", var_name, var_type))
                        .with_color(Color::Red),
                )
                .with_note("values captured by spawned tasks must implement the Transfer trait")
                .with_help("consider cloning the value or using a channel to send it")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::NonSharableShare {
            var_name,
            var_type,
            span,
        } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E6002")
                .with_message("cannot share non-Sharable value between tasks")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("variable `{}` of type `{}` is not Sharable", var_name, var_type))
                        .with_color(Color::Red),
                )
                .with_note("values shared between tasks must implement the Sharable trait")
                .with_help("consider using a channel for communication instead")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::InvalidTryOperator { found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E0005")
                .with_message("invalid use of `?` operator")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("`?` cannot be applied to type `{}`", found))
                        .with_color(Color::Red),
                )
                .with_note("the `?` operator can only be used on Result or Optional types")
                .with_help(format!("consider wrapping in Ok() or Some(): `Ok({})` or `Some({})`",
                    found.to_lowercase(), found.to_lowercase()))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::TryInNonResultFunction { function_return, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E0006")
                .with_message("cannot use `?` in function that doesn't return Result or Optional")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("function returns `{}`, not Result or Optional", function_return))
                        .with_color(Color::Red),
                )
                .with_help("change the function's return type to Result<T, E> or Optional<T>")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ModuleNotFound(name, span) => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E2001")
                .with_message(format!("module not found: `{}`", name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("unknown module")
                        .with_color(Color::Red),
                )
                .with_help("check the module path and ensure the module exists")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ImportNotExported { symbol, module, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E2002")
                .with_message(format!("symbol `{}` is not exported from module `{}`", symbol, module))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not exported")
                        .with_color(Color::Red),
                )
                .with_help(format!("add `pub` visibility to `{}` in module `{}`, or use a different symbol", symbol, module))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UnresolvedImport { symbol, module, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E2003")
                .with_message(format!("unresolved import: `{}` from `{}`", symbol, module))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("could not resolve this import")
                        .with_color(Color::Red),
                )
                .with_help("ensure the symbol exists in the imported module and is properly exported")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::NonExhaustivePatterns { missing, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E4003")
                .with_message("non-exhaustive patterns")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("patterns not covered: {}", missing))
                        .with_color(Color::Red),
                )
                .with_help("ensure all possible cases are handled, or add a wildcard pattern `_`")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UnreachablePattern { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Warning, filename, span.start)
                .with_code("W4001")
                .with_message("unreachable pattern")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("this pattern will never be matched")
                        .with_color(Color::Yellow),
                )
                .with_note("previous patterns already cover all possible cases")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UndefinedField { type_name, field_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E2004")
                .with_message(format!("type `{}` has no field `{}`", type_name, field_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("unknown field `{}`", field_name))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::FieldAccessOnNonStruct { type_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E2005")
                .with_message(format!("cannot access field on non-struct type `{}`", type_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("`{}` is not a struct type", type_name))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }
        TypeError::ReturnTypeMismatch { expected, found, span } => {
            let span_range = span.start..span.end;
            let mut report = Report::build(ReportKind::Error, filename, span.start)
                .with_code("E0002")
                .with_message("return type mismatch")
                .with_label(
                    Label::new((filename, span_range.clone()))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                );

            // Add conversion suggestion if applicable
            if let Some(suggestion) = get_type_conversion_suggestion(expected, found) {
                report = report.with_help(format!("consider converting: {}", suggestion));
            }

            report
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // FFI / Extern type errors
        TypeError::InvalidFfiCType { c_type, reason, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E7001")
                .with_message(format!("invalid C type in FFI declaration: `{}`", c_type))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(reason)
                        .with_color(Color::Red),
                )
                .with_note("FFI types must be C-compatible")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::NonFfiSafeParameter { func_name, param_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E7002")
                .with_message(format!("non-FFI-safe parameter in extern function `{}`", func_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("parameter `{}` uses a type not safe for FFI", param_name))
                        .with_color(Color::Red),
                )
                .with_help("use C-compatible types: int, long, float, double, *const T, *mut T, etc.")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::NonFfiSafeReturn { func_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E7003")
                .with_message(format!("non-FFI-safe return type in extern function `{}`", func_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("return type is not safe for FFI")
                        .with_color(Color::Red),
                )
                .with_help("use C-compatible return types: int, long, float, double, void, pointers, etc.")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::NonFfiSafeField { struct_name, field_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E7004")
                .with_message(format!("non-FFI-safe field in extern struct `{}`", struct_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("field `{}` uses a type not safe for FFI", field_name))
                        .with_color(Color::Red),
                )
                .with_help("use C-compatible types for struct fields")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::DuplicateExternDeclaration { name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E7005")
                .with_message(format!("duplicate extern declaration: `{}`", name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("already declared elsewhere")
                        .with_color(Color::Red),
                )
                .with_note("extern symbols must have unique names")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::MissingExternParamType { func_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E7006")
                .with_message(format!("missing type annotation on extern function parameter in `{}`", func_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("parameter type required for extern functions")
                        .with_color(Color::Red),
                )
                .with_help("add explicit type annotation: `param_name: CType`")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::VariadicFfiFunction { func_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E7007")
                .with_message(format!("variadic functions not supported in FFI: `{}`", func_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("variadic functions cannot be called safely")
                        .with_color(Color::Red),
                )
                .with_note("Aria does not support calling C variadic functions directly")
                .with_help("create a wrapper function with fixed parameters")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::MutableCaptureOfImmutable { var_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E8001")
                .with_message(format!("cannot mutably capture immutable variable `{}`", var_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("mutable capture of immutable variable")
                        .with_color(Color::Red),
                )
                .with_help("declare the variable as `let mut` to allow mutable capture")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::MutableCaptureInSpawn { var_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E8002")
                .with_message(format!("cannot mutably capture `{}` in spawn", var_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("spawned closures cannot hold mutable borrows")
                        .with_color(Color::Red),
                )
                .with_note("spawned tasks may outlive the current scope")
                .with_help("use channels or shared state primitives for cross-task communication")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::TupleIndexOutOfBounds { index, length, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E8003")
                .with_message(format!("tuple index {} out of bounds", index))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("tuple has {} element(s), but index {} was requested", length, index))
                        .with_color(Color::Red),
                )
                .with_help(format!("valid indices are 0..{}", length.saturating_sub(1)))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::TupleToArrayHeterogeneousTypes { types, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E8004")
                .with_message("cannot convert tuple to array: heterogeneous types")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("tuple elements have different types: {}", types))
                        .with_color(Color::Red),
                )
                .with_help("arrays require all elements to have the same type")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::TraitNotImplemented { ty, trait_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E9001")
                .with_message(format!("type `{}` does not implement trait `{}`", ty, trait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("`{}` required here", trait_name))
                        .with_color(Color::Red),
                )
                .with_help(format!("consider implementing `{}` for `{}`", trait_name, ty))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::BoundNotSatisfied { type_arg, param, bound, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E9002")
                .with_message(format!("type argument `{}` does not satisfy bound `{}`", type_arg, bound))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("type parameter `{}` requires bound `{}`", param, bound))
                        .with_color(Color::Red),
                )
                .with_help(format!("`{}` must implement `{}`", type_arg, bound))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UndefinedTrait(name, span) => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E9003")
                .with_message(format!("undefined trait: `{}`", name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("trait not found")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::WrongTraitArity { trait_name, expected, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E9004")
                .with_message(format!("wrong number of type arguments for trait `{}`", trait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected {} type argument(s), found {}", expected, found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ConflictingImpl { trait_name, for_type, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E9005")
                .with_message(format!("conflicting implementations of trait `{}`", trait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("duplicate implementation for type `{}`", for_type))
                        .with_color(Color::Red),
                )
                .with_note("only one implementation of a trait is allowed for each type")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::WhereClauseNotSatisfied { constraint, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E9006")
                .with_message("where clause constraint not satisfied")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("constraint `{}` is not satisfied", constraint))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::MissingTraitImpl { ty, trait_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E9007")
                .with_message(format!("type `{}` does not implement trait `{}`", ty, trait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("`{}` required here", trait_name))
                        .with_color(Color::Red),
                )
                .with_help(format!("consider implementing `{}` for `{}`", trait_name, ty))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::AwaitOutsideAsync { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E10001")
                .with_message("`await` can only be used in async context")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("await used outside async function")
                        .with_color(Color::Red),
                )
                .with_help("mark the enclosing function as `async` or use within an async block")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::AwaitNonTask { found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E10002")
                .with_message(format!("`await` expects a Task type, found `{}`", found))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not a Task")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::SendOnNonChannel { found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E10003")
                .with_message(format!("channel send expects a channel, found `{}`", found))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not a channel")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ReceiveOnNonChannel { found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E10004")
                .with_message(format!("channel receive expects a channel, found `{}`", found))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not a channel")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::MissingTraitMethod { method_name, trait_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11001")
                .with_message(format!("missing method `{}` in implementation of trait `{}`", method_name, trait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("required by trait")
                        .with_color(Color::Red),
                )
                .with_help(format!("implement the `{}` method", method_name))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::TraitMethodSignatureMismatch { method_name, expected, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11002")
                .with_message(format!("method `{}` has wrong signature", method_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::MissingAssociatedType { type_name, trait_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11003")
                .with_message(format!("missing associated type `{}` in implementation of trait `{}`", type_name, trait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("required by trait")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::MethodNotInTrait { method_name, trait_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11004")
                .with_message(format!("method `{}` is not a member of trait `{}`", method_name, trait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not in trait")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::AssociatedTypeNotInTrait { type_name, trait_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11005")
                .with_message(format!("associated type `{}` is not defined in trait `{}`", type_name, trait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not in trait")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::SupertraitNotImplemented { trait_name, supertrait_name, span, .. } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11006")
                .with_message(format!("trait `{}` requires implementing supertrait `{}`", trait_name, supertrait_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("supertrait not implemented")
                        .with_color(Color::Red),
                )
                .with_help(format!("implement `{}` first", supertrait_name))
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::SelfTypeMismatch { method_name, expected, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11007")
                .with_message(format!("Self type mismatch in method `{}`", method_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected Self to be `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::DuplicateImplMethod { method_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11008")
                .with_message(format!("duplicate method `{}` in impl block", method_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("already defined")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::DuplicateAssociatedType { type_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E11009")
                .with_message(format!("duplicate associated type `{}` in impl block", type_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("already defined")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // Select expression errors
        TypeError::MultipleDefaultArms { first_span, second_span } => {
            let first_range = first_span.start..first_span.end;
            let second_range = second_span.start..second_span.end;
            Report::build(ReportKind::Error, filename, second_span.start)
                .with_code("E12001")
                .with_message("multiple default arms in select expression")
                .with_label(
                    Label::new((filename, first_range))
                        .with_message("first default arm defined here")
                        .with_color(Color::Yellow),
                )
                .with_label(
                    Label::new((filename, second_range))
                        .with_message("duplicate default arm")
                        .with_color(Color::Red),
                )
                .with_note("only one default arm is allowed in a select expression")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::SelectArmTypeMismatch { expected, found, arm_index, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E12002")
                .with_message("select arm type mismatch")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("arm {} has type `{}`, expected `{}`", arm_index, found, expected))
                        .with_color(Color::Red),
                )
                .with_help("all select arms must have the same result type")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // Const evaluation errors
        TypeError::NotConstant { reason, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E13001")
                .with_message("expression is not a compile-time constant")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(reason)
                        .with_color(Color::Red),
                )
                .with_help("use only literals and const expressions here")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ConstEvalError { reason, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E13002")
                .with_message("const evaluation error")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(reason)
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ConstOverflow { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E13003")
                .with_message("integer overflow in const evaluation")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("overflow occurred")
                        .with_color(Color::Red),
                )
                .with_help("use smaller values or a larger integer type")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ConstDivisionByZero { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E13004")
                .with_message("division by zero in const evaluation")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("cannot divide by zero")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UndefinedConstant(name, span) => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E13005")
                .with_message(format!("undefined constant: `{}`", name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not found")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // Effect system errors
        TypeError::UndeclaredEffect { effect, function_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14001")
                .with_message(format!("effect `{}` not declared in function `{}`", effect, function_name))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("this effect must be declared in the function signature")
                        .with_color(Color::Red),
                )
                .with_help("Add the effect to the function's effect annotation, e.g., `fn foo() !{IO} -> T`")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UnhandledEffect { effect, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14002")
                .with_message(format!("unhandled effect `{}`", effect))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("no handler in scope for this effect")
                        .with_color(Color::Red),
                )
                .with_help("Use a `handle` block to provide a handler for this effect")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::EffectHandlerTypeMismatch { effect, expected, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14003")
                .with_message(format!("effect handler type mismatch for `{}`", effect))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::EffectRowMismatch { expected, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14004")
                .with_message("effect row mismatch")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::EffectfulCallInPureContext { callee_effects, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14005")
                .with_message("cannot call effectful function from pure context")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("this function has effects `{}`", callee_effects))
                        .with_color(Color::Red),
                )
                .with_help("Either declare the effect in the calling function or use a handler")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UndefinedEffect { effect, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14006")
                .with_message(format!("undefined effect: `{}`", effect))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("effect not found")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::DuplicateEffectDeclaration { effect, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14007")
                .with_message(format!("duplicate effect declaration: `{}`", effect))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("effect already declared")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ResumeOutsideHandler { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14008")
                .with_message("`resume` can only be used inside an effect handler")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("not inside a handler")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ResumeTypeMismatch { expected, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E14009")
                .with_message("resume type mismatch")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // Defer errors
        TypeError::DeferNonUnit { found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E15001")
                .with_message("defer block must return unit type")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected `()`, found `{}`", found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ControlFlowInDefer { statement, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E15002")
                .with_message(format!("`{}` cannot be used inside defer", statement))
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("control flow not allowed in defer")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::DeferCaptureInvalid { var_name, defer_span, var_span } => {
            let defer_range = defer_span.start..defer_span.end;
            let var_range = var_span.start..var_span.end;
            Report::build(ReportKind::Error, filename, defer_span.start)
                .with_code("E15003")
                .with_message(format!("variable `{}` may not be valid when defer executes", var_name))
                .with_label(
                    Label::new((filename, defer_range))
                        .with_message("defer block here")
                        .with_color(Color::Red),
                )
                .with_label(
                    Label::new((filename, var_range))
                        .with_message("variable captured here")
                        .with_color(Color::Yellow),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::AwaitInDefer { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E15004")
                .with_message("cannot use `await` inside defer block")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("await not allowed in defer")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // Default parameter errors
        TypeError::DefaultValueTypeMismatch { param_name, param_type, default_type, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E16001")
                .with_message("default value type mismatch")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!(
                            "parameter `{}` has type `{}`, but default value has type `{}`",
                            param_name, param_type, default_type
                        ))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::DefaultAfterRequired { param_name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E16002")
                .with_message("required parameter after default parameter")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("parameter `{}` must have a default value", param_name))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::TooFewArguments { min_required, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E16003")
                .with_message("too few arguments")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!(
                            "expected at least {} arguments, found {}",
                            min_required, found
                        ))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::TooManyArguments { max_allowed, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E16004")
                .with_message("too many arguments")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!(
                            "expected at most {} arguments, found {}",
                            max_allowed, found
                        ))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::UnknownNamedArgument { name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E16005")
                .with_message("unknown named argument")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("no parameter named `{}`", name))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::DuplicateNamedArgument { name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E16006")
                .with_message("duplicate named argument")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("argument `{}` already provided", name))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::MissingRequiredArgument { name, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E16007")
                .with_message("missing required argument")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("argument `{}` is required", name))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::PositionalAfterNamed { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E16008")
                .with_message("positional argument after named argument")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("positional arguments must come before named arguments")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // Spread operator errors
        TypeError::SpreadOnNonArray { found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E17001")
                .with_message("spread on non-array type")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected array type, found `{}`", found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::SpreadElementTypeMismatch { spread_elem_type, param_type, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E17002")
                .with_message("spread element type mismatch")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!(
                            "spread element type `{}` is not compatible with parameter type `{}`",
                            spread_elem_type, param_type
                        ))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::SpreadArrayElementMismatch { spread_elem_type, array_elem_type, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E17003")
                .with_message("spread array element type mismatch")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!(
                            "spread element type `{}` is not compatible with array element type `{}`",
                            spread_elem_type, array_elem_type
                        ))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::SpreadStructTypeMismatch { source_type, target_struct, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E17004")
                .with_message("spread struct type mismatch")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!(
                            "source type `{}` is not compatible with target struct `{}`",
                            source_type, target_struct
                        ))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::SpreadOnNonStruct { found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E17005")
                .with_message("spread on non-struct type")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected struct type, found `{}`", found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        // Loop control flow errors
        TypeError::BreakOutsideLoop { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E18001")
                .with_message("`break` cannot be used outside of a loop")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("break outside loop")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::ContinueOutsideLoop { span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E18002")
                .with_message("`continue` cannot be used outside of a loop")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message("continue outside loop")
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::BreakTypeMismatch { expected, found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E18003")
                .with_message("break value type mismatch")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("expected `{}`, found `{}`", expected, found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }

        TypeError::NotIterable { found, span } => {
            let span_range = span.start..span.end;
            Report::build(ReportKind::Error, filename, span.start)
                .with_code("E18004")
                .with_message("type is not iterable")
                .with_label(
                    Label::new((filename, span_range))
                        .with_message(format!("type `{}` is not iterable", found))
                        .with_color(Color::Red),
                )
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }
    }
}

/// Report a module error
fn report_module_error(error: &ModuleError) {
    match error {
        ModuleError::ModuleNotFound(name) => {
            eprintln!("Error: Module not found: {}", name);
        }
        ModuleError::FileNotFound(path) => {
            eprintln!("Error: File not found: {}", path.display());
        }
        ModuleError::IoError(path, err) => {
            eprintln!("Error reading {}: {}", path.display(), err);
        }
        ModuleError::ParseError { path, errors } => {
            let source = fs::read_to_string(path).unwrap_or_default();
            let filename = path.display().to_string();
            for error in errors {
                report_parse_error(&filename, &source, error);
            }
        }
        ModuleError::CircularDependency(cycle) => {
            eprintln!("Error: Circular dependency detected:");
            for (i, module_id) in cycle.iter().enumerate() {
                if i > 0 {
                    eprintln!("  -> {:?}", module_id);
                } else {
                    eprintln!("  {:?}", module_id);
                }
            }
        }
        ModuleError::CircularDependencyNamed(cycle) => {
            eprintln!("Error: Circular dependency detected:");
            eprintln!("  {}", cycle.join(" -> "));
            eprintln!();
            eprintln!("Hint: Circular imports are not allowed. Consider:");
            eprintln!("  - Breaking the cycle by extracting shared types into a separate module");
            eprintln!("  - Using dependency injection or interfaces to decouple modules");
        }
        ModuleError::ImportResolutionFailed(import) => {
            eprintln!("Error: Failed to resolve import: {}", import);
        }
        ModuleError::ItemNotFound { module, item } => {
            eprintln!("Error: Item '{}' not found in module '{}'", item, module);
        }
        ModuleError::PrivateItem { module, item } => {
            eprintln!("Error: Item '{}' is private in module '{}'", item, module);
        }
        ModuleError::ConflictingImports(name) => {
            eprintln!("Error: Conflicting imports for name '{}'", name);
        }
        ModuleError::NameConflict(name) => {
            eprintln!("Error: Module name conflict: {}", name);
        }
    }
}

/// Print a summary of an AST item
fn print_item_summary(item: &aria_ast::Item) {
    match item {
        aria_ast::Item::Function(f) => {
            let params: Vec<_> = f.params.iter().map(|p| p.name.node.as_str()).collect();
            println!("  fn {}({})", f.name.node, params.join(", "));
        }
        aria_ast::Item::Struct(s) => {
            println!("  struct {} ({} fields)", s.name.node, s.fields.len());
        }
        aria_ast::Item::Enum(e) => {
            println!("  enum {} ({} variants)", e.name.node, e.variants.len());
        }
        aria_ast::Item::Trait(t) => {
            println!("  trait {} ({} members)", t.name.node, t.members.len());
        }
        aria_ast::Item::Impl(i) => {
            let trait_name = i
                .trait_
                .as_ref()
                .map(|t| format!("{:?} for ", t))
                .unwrap_or_default();
            println!("  impl {}{:?}", trait_name, i.for_type);
        }
        aria_ast::Item::Data(d) => {
            println!("  data {} ({} fields)", d.name.node, d.fields.len());
        }
        aria_ast::Item::Module(m) => {
            let path: Vec<_> = m.path.iter().map(|p| p.node.as_str()).collect();
            println!("  module {}", path.join("::"));
        }
        aria_ast::Item::Import(i) => {
            println!("  import {:?}", i.path);
        }
        aria_ast::Item::Export(_) => {
            println!("  export ...");
        }
        aria_ast::Item::TypeAlias(t) => {
            println!("  type {}", t.name.node);
        }
        aria_ast::Item::Const(c) => {
            println!("  const {}", c.name.node);
        }
        aria_ast::Item::Extern(_) => {
            println!("  extern ...");
        }
        aria_ast::Item::Test(t) => {
            println!("  test {:?}", t.name);
        }
        aria_ast::Item::Use(u) => {
            let path: Vec<_> = u.path.iter().map(|p| p.node.as_str()).collect();
            if let Some(alias) = &u.alias {
                println!("  use {} as {}", path.join("::"), alias.node);
            } else {
                println!("  use {}", path.join("::"));
            }
        }
    }
}
