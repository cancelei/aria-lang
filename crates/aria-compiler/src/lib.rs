//! Aria Language Compiler Library
//!
//! This module exposes the compilation functionality as a library
//! for use by aria-pkg and other tools.

use aria_modules::{CompilationMode, FileSystemResolver, ModuleCompiler, ModuleError};
use aria_types::{TypeChecker, TypeError};
use aria_mir::contract_verifier::{MirContractVerifier, VerificationMode};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Errors that can occur during compilation
#[derive(Debug)]
pub enum CompileError {
    /// I/O error reading/writing files
    Io(std::io::Error),
    /// Module resolution or loading error
    Module(ModuleError),
    /// Type checking error
    Type(TypeError),
    /// MIR lowering error
    Mir(aria_mir::MirError),
    /// Code generation error
    Codegen(aria_codegen::CodegenError),
    /// Linker error
    Linker(String),
    /// Runtime library not found
    RuntimeNotFound(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Io(e) => write!(f, "I/O error: {}", e),
            CompileError::Module(e) => write!(f, "Module error: {:?}", e),
            CompileError::Type(e) => write!(f, "Type error: {:?}", e),
            CompileError::Mir(e) => write!(f, "MIR lowering error: {}", e),
            CompileError::Codegen(e) => write!(f, "Code generation error: {}", e),
            CompileError::Linker(e) => write!(f, "Linker error: {}", e),
            CompileError::RuntimeNotFound(e) => write!(f, "Runtime not found: {}", e),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e)
    }
}

impl From<ModuleError> for CompileError {
    fn from(e: ModuleError) -> Self {
        CompileError::Module(e)
    }
}

impl From<TypeError> for CompileError {
    fn from(e: TypeError) -> Self {
        CompileError::Type(e)
    }
}

impl From<aria_mir::MirError> for CompileError {
    fn from(e: aria_mir::MirError) -> Self {
        CompileError::Mir(e)
    }
}

impl From<aria_codegen::CodegenError> for CompileError {
    fn from(e: aria_codegen::CodegenError) -> Self {
        CompileError::Codegen(e)
    }
}

/// Result type for compilation operations
pub type CompileResult<T> = Result<T, CompileError>;

/// Options for compilation
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Output path for the compiled file
    pub output: Option<PathBuf>,
    /// Whether to link into an executable
    pub link: bool,
    /// Explicit path to runtime library
    pub runtime_path: Option<PathBuf>,
    /// Compile as library instead of binary
    pub is_library: bool,
    /// Additional library search paths
    pub lib_paths: Vec<PathBuf>,
    /// Build in release mode (with optimizations)
    pub release: bool,
    /// Contract verification mode
    pub verify_contracts: VerificationMode,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            output: None,
            link: false,
            runtime_path: None,
            is_library: false,
            lib_paths: Vec::new(),
            release: false,
            verify_contracts: VerificationMode::Debug,
        }
    }
}

impl CompileOptions {
    /// Create options for a linked executable
    pub fn executable() -> Self {
        Self {
            link: true,
            ..Default::default()
        }
    }

    /// Create options for an object file only
    pub fn object_file() -> Self {
        Self::default()
    }

    /// Set the output path
    pub fn with_output(mut self, path: impl Into<PathBuf>) -> Self {
        self.output = Some(path.into());
        self
    }

    /// Add a library search path
    pub fn with_lib_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.lib_paths.push(path.into());
        self
    }

    /// Set release mode
    pub fn with_release(mut self, release: bool) -> Self {
        self.release = release;
        self
    }
}

/// Compilation output
#[derive(Debug)]
pub struct CompileOutput {
    /// Path to the output file
    pub output_path: PathBuf,
    /// Modules that were compiled
    pub modules: Vec<String>,
    /// Whether the output is an executable
    pub is_executable: bool,
}

/// Compile an Aria source file
///
/// This is the main entry point for compilation. It handles:
/// - Module resolution and loading
/// - Type checking
/// - MIR lowering
/// - Code generation
/// - Optional linking
///
/// # Example
///
/// ```ignore
/// use aria_compiler::{compile_file, CompileOptions};
///
/// let result = compile_file("src/main.aria", CompileOptions::executable())?;
/// println!("Compiled to: {}", result.output_path.display());
/// ```
pub fn compile_file(path: impl AsRef<Path>, options: CompileOptions) -> CompileResult<CompileOutput> {
    let path = path.as_ref();

    // Setup module resolver
    let mut resolver = FileSystemResolver::new();
    for lib_path in &options.lib_paths {
        resolver.add_search_path(lib_path);
    }

    // Determine compilation mode
    let mode = if options.is_library {
        CompilationMode::Library
    } else {
        CompilationMode::Binary
    };

    // Compile all modules starting from entry point
    let mut compiler = ModuleCompiler::new(Box::new(resolver), mode);
    let modules = compiler.compile(&path.to_path_buf())?;

    let module_names: Vec<String> = modules.iter().map(|m| m.name.to_string()).collect();

    // Type check the entry module
    // TODO: Type check all modules with proper import resolution
    let entry_module = modules.last().expect("No entry module");
    let mut checker = TypeChecker::new();
    checker.check_program(&entry_module.ast)?;

    // Lower to MIR
    let mut mir = aria_mir::lower_program(&entry_module.ast)?;

    // Verify contracts
    let mut contract_verifier = MirContractVerifier::new(
        if options.release {
            VerificationMode::Release
        } else {
            options.verify_contracts
        }
    );
    contract_verifier.verify_program(&mut mir);

    // Compile to object file
    let object_bytes = aria_codegen::compile_to_object(&mir, Target::native())?;

    // Determine output path
    let output_path = options.output.clone().unwrap_or_else(|| {
        let stem = path.file_stem().unwrap_or_default();
        if options.link {
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

    if options.link {
        // Link mode: create executable
        link_executable(path, &object_bytes, &output_path, options.runtime_path.as_deref())?;
    } else {
        // Object file mode: write object file directly
        fs::write(&output_path, &object_bytes)?;
    }

    Ok(CompileOutput {
        output_path,
        modules: module_names,
        is_executable: options.link,
    })
}

/// Link object bytes into an executable
fn link_executable(
    source_path: &Path,
    object_bytes: &[u8],
    output_path: &Path,
    runtime_path: Option<&Path>,
) -> CompileResult<()> {
    // Create temporary object file
    let temp_obj = source_path.with_file_name(format!(
        ".{}.o",
        source_path.file_stem().unwrap_or_default().to_string_lossy()
    ));

    // Write temporary object file
    fs::write(&temp_obj, object_bytes)?;

    // Find runtime library
    let runtime = find_runtime_library(runtime_path)?;

    // Link with gcc
    let link_result = Command::new("gcc")
        .arg(&runtime)
        .arg(&temp_obj)
        .arg("-o")
        .arg(output_path)
        .arg("-lm") // Link math library for sqrt, sin, cos, etc.
        .output();

    // Clean up temporary object file
    let _ = fs::remove_file(&temp_obj);

    match link_result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            Err(CompileError::Linker(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
        Err(e) => Err(CompileError::Linker(format!(
            "Failed to run linker (gcc): {}. Make sure gcc is installed.",
            e
        ))),
    }
}

/// Find the Aria runtime library
///
/// Searches in common locations for the pre-compiled runtime library.
/// You can also specify an explicit path.
pub fn find_runtime_library(explicit_path: Option<&Path>) -> CompileResult<PathBuf> {
    // If explicit path provided, use it
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path.to_path_buf());
        } else {
            return Err(CompileError::RuntimeNotFound(format!(
                "Runtime library not found at: {}",
                path.display()
            )));
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
        // Relative to library crate (for development)
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../aria-runtime/c_runtime/aria_runtime.o")),
        // System locations
        Some(PathBuf::from("/usr/local/lib/aria/aria_runtime.o")),
        Some(PathBuf::from("/usr/lib/aria/aria_runtime.o")),
        // Home directory
        dirs::home_dir().map(|h| h.join(".aria/lib/aria_runtime.o")),
    ];

    for path in search_paths.into_iter().flatten() {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(CompileError::RuntimeNotFound(
        "Runtime library (aria_runtime.o) not found. \
         Build it with 'make' in crates/aria-runtime/c_runtime/ \
         or specify --runtime path."
            .to_string(),
    ))
}

/// Check a source file for errors without compiling
pub fn check_file(path: impl AsRef<Path>) -> CompileResult<Vec<String>> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;

    // Parse the source
    let (program, parse_errors) = aria_parser::parse(&source);

    if !parse_errors.is_empty() {
        // Return first parse error
        return Err(CompileError::Module(ModuleError::ParseError {
            path: path.to_path_buf(),
            errors: parse_errors,
        }));
    }

    // Type check
    let mut checker = TypeChecker::new();
    checker.check_program(&program)?;

    // Collect item names
    let items: Vec<String> = program
        .items
        .iter()
        .filter_map(|item| match item {
            aria_ast::Item::Function(f) => Some(format!("fn {}", f.name.node)),
            aria_ast::Item::Struct(s) => Some(format!("struct {}", s.name.node)),
            aria_ast::Item::Enum(e) => Some(format!("enum {}", e.name.node)),
            aria_ast::Item::Trait(t) => Some(format!("trait {}", t.name.node)),
            _ => None,
        })
        .collect();

    Ok(items)
}

// Re-export key types for convenience
pub use aria_codegen::Target;
pub use aria_modules::ModuleId;
