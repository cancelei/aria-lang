//! WASM Component Model support for Aria code generation.
//!
//! This module extends the core WASM backend with Component Model features:
//! - Import section generation from Aria effect declarations
//! - Memory section for string/data passing
//! - Component-level type annotations
//!
//! # Architecture
//!
//! ```text
//! MIR Program + Effect Rows
//!     ↓
//! WIT Generation (aria-wit)     →  .wit file (for tooling)
//!     ↓
//! WASM Core Module (existing)   +  Import Section  +  Memory Section
//!     ↓
//! WASM Component Binary (.wasm)
//! ```

use std::collections::HashMap;

use aria_mir::{EffectType, MirProgram, MirType};

use crate::{Result, Target};
use crate::wasm_backend::{WasmCompiler, WasmType};

/// An imported function from the host environment
#[derive(Debug, Clone)]
pub struct WasmImport {
    /// Module name (e.g., "wasi:io/streams")
    pub module: String,
    /// Function name within the module
    pub name: String,
    /// Parameter types
    pub params: Vec<WasmType>,
    /// Return types
    pub returns: Vec<WasmType>,
}

/// Extended WASM compiler that supports the Component Model
pub struct WasmComponentCompiler {
    /// The core WASM compiler
    core: WasmCompiler,
    /// Import declarations derived from effects
    imports: Vec<WasmImport>,
    /// Whether to include a memory section
    include_memory: bool,
    /// Initial memory pages (64KB each)
    initial_memory_pages: u32,
    /// Maximum memory pages
    max_memory_pages: Option<u32>,
}

impl WasmComponentCompiler {
    pub fn new(target: Target) -> Result<Self> {
        Ok(Self {
            core: WasmCompiler::new(target)?,
            imports: Vec::new(),
            include_memory: false,
            initial_memory_pages: 1,
            max_memory_pages: Some(256), // 16MB max by default
        })
    }

    /// Add imports derived from the program's effect declarations
    pub fn add_effect_imports(&mut self, program: &MirProgram) {
        for func in program.functions.values() {
            for effect in &func.effect_row.effects {
                self.add_imports_for_effect(effect);
            }
        }
    }

    /// Map an Aria effect to WASM imports
    fn add_imports_for_effect(&mut self, effect: &EffectType) {
        let imports = effect_to_imports(effect);
        for import in imports {
            // Deduplicate
            let key = format!("{}:{}", import.module, import.name);
            if !self.imports.iter().any(|i| format!("{}:{}", i.module, i.name) == key) {
                self.imports.push(import);
            }
        }
    }

    /// Enable memory section (needed for strings, arrays, etc.)
    pub fn enable_memory(&mut self, initial_pages: u32, max_pages: Option<u32>) {
        self.include_memory = true;
        self.initial_memory_pages = initial_pages;
        self.max_memory_pages = max_pages;
    }

    /// Compile the program
    pub fn compile_program(&mut self, program: &MirProgram) -> Result<()> {
        // Detect if memory is needed
        if program_needs_memory(program) {
            self.include_memory = true;
        }

        // Add imports from effects
        self.add_effect_imports(program);

        // Compile the core module
        self.core.compile_program(program)
    }

    /// Generate the final WASM binary with Component Model extensions
    pub fn finish(self) -> Result<Vec<u8>> {
        let mut output = Vec::new();

        // Magic number
        output.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]);
        // Version
        output.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        // Build the sections in correct order:
        // 1. Type section (includes import function types)
        // 2. Import section
        // 3. Function section
        // 4. Memory section (if needed)
        // 5. Export section
        // 6. Code section

        let num_import_funcs = self.imports.len() as u32;

        // Collect all types: imports first, then local functions
        let mut all_types: Vec<(Vec<WasmType>, Vec<WasmType>)> = Vec::new();
        let mut import_type_indices: Vec<u32> = Vec::new();

        for import in &self.imports {
            let type_sig = (import.params.clone(), import.returns.clone());
            let idx = find_or_add_type(&mut all_types, type_sig);
            import_type_indices.push(idx);
        }

        // Get the core module's types and adjust indices
        let core_types = self.core.get_function_types();
        let mut core_type_remap: HashMap<u32, u32> = HashMap::new();
        for (old_idx, type_sig) in core_types.iter().enumerate() {
            let new_idx = find_or_add_type(&mut all_types, type_sig.clone());
            core_type_remap.insert(old_idx as u32, new_idx);
        }

        // 1. Type section
        emit_type_section(&mut output, &all_types);

        // 2. Import section
        if !self.imports.is_empty() {
            emit_import_section(&mut output, &self.imports, &import_type_indices);
        }

        // 3. Function section (with remapped type indices)
        let core_functions = self.core.get_functions();
        emit_function_section_remapped(&mut output, &core_functions, &core_type_remap);

        // 4. Memory section
        if self.include_memory {
            emit_memory_section(
                &mut output,
                self.initial_memory_pages,
                self.max_memory_pages,
            );
        }

        // 5. Export section (function indices shifted by import count)
        let core_exports = self.core.get_exports();
        emit_export_section_shifted(&mut output, &core_exports, num_import_funcs, self.include_memory);

        // 6. Code section
        let core_code = self.core.get_code_bodies();
        emit_code_section(&mut output, &core_code);

        Ok(output)
    }

    /// Get the number of imports
    pub fn import_count(&self) -> usize {
        self.imports.len()
    }

    /// Get the import list
    pub fn get_imports(&self) -> &[WasmImport] {
        &self.imports
    }
}

/// Map an Aria effect to concrete WASM import functions
fn effect_to_imports(effect: &EffectType) -> Vec<WasmImport> {
    match effect.name.as_str() {
        "IO" => vec![
            WasmImport {
                module: "wasi:io/streams".to_string(),
                name: "read".to_string(),
                params: vec![WasmType::I32, WasmType::I64], // stream handle, len
                returns: vec![WasmType::I32],                // ptr to result
            },
            WasmImport {
                module: "wasi:io/streams".to_string(),
                name: "write".to_string(),
                params: vec![WasmType::I32, WasmType::I32, WasmType::I32], // stream, ptr, len
                returns: vec![WasmType::I64],                               // bytes written
            },
        ],
        "Console" => vec![
            WasmImport {
                module: "wasi:cli/stdout".to_string(),
                name: "get-stdout".to_string(),
                params: vec![],
                returns: vec![WasmType::I32], // stream handle
            },
            WasmImport {
                module: "wasi:cli/stdin".to_string(),
                name: "get-stdin".to_string(),
                params: vec![],
                returns: vec![WasmType::I32], // stream handle
            },
            WasmImport {
                module: "wasi:cli/stdout".to_string(),
                name: "print".to_string(),
                params: vec![WasmType::I32, WasmType::I32], // ptr, len
                returns: vec![],
            },
        ],
        "FileSystem" => vec![
            WasmImport {
                module: "wasi:filesystem/types".to_string(),
                name: "read-via-stream".to_string(),
                params: vec![WasmType::I32, WasmType::I64], // fd, offset
                returns: vec![WasmType::I32],                // stream handle
            },
            WasmImport {
                module: "wasi:filesystem/types".to_string(),
                name: "write-via-stream".to_string(),
                params: vec![WasmType::I32, WasmType::I64], // fd, offset
                returns: vec![WasmType::I32],                // stream handle
            },
            WasmImport {
                module: "wasi:filesystem/types".to_string(),
                name: "stat".to_string(),
                params: vec![WasmType::I32],  // fd
                returns: vec![WasmType::I32], // ptr to stat result
            },
        ],
        "Network" => vec![
            WasmImport {
                module: "wasi:http/outgoing-handler".to_string(),
                name: "handle".to_string(),
                params: vec![WasmType::I32], // request handle
                returns: vec![WasmType::I32], // response handle
            },
        ],
        "Random" => vec![WasmImport {
            module: "wasi:random/random".to_string(),
            name: "get-random-bytes".to_string(),
            params: vec![WasmType::I64],  // len
            returns: vec![WasmType::I32], // ptr to bytes
        }],
        "Clock" => vec![WasmImport {
            module: "wasi:clocks/wall-clock".to_string(),
            name: "now".to_string(),
            params: vec![],
            returns: vec![WasmType::I64], // timestamp
        }],
        _ => vec![], // Unknown effects produce no imports
    }
}

/// Check if a program uses types that need linear memory
fn program_needs_memory(program: &MirProgram) -> bool {
    program.functions.values().any(|f| {
        f.locals.iter().any(|l| type_needs_memory(&l.ty))
            || type_needs_memory(&f.return_ty)
    })
}

fn type_needs_memory(ty: &MirType) -> bool {
    matches!(
        ty,
        MirType::String
            | MirType::Array(_)
            | MirType::Map(_, _)
            | MirType::Tuple(_)
            | MirType::Struct(_)
    )
}

// Section emission helpers

fn find_or_add_type(
    types: &mut Vec<(Vec<WasmType>, Vec<WasmType>)>,
    sig: (Vec<WasmType>, Vec<WasmType>),
) -> u32 {
    for (i, existing) in types.iter().enumerate() {
        if existing.0 == sig.0 && existing.1 == sig.1 {
            return i as u32;
        }
    }
    let idx = types.len() as u32;
    types.push(sig);
    idx
}

fn emit_type_section(output: &mut Vec<u8>, types: &[(Vec<WasmType>, Vec<WasmType>)]) {
    let mut section = Vec::new();
    encode_u32(&mut section, types.len() as u32);

    for (params, returns) in types {
        section.push(0x60); // func type
        encode_u32(&mut section, params.len() as u32);
        for p in params {
            section.push(p.encode());
        }
        encode_u32(&mut section, returns.len() as u32);
        for r in returns {
            section.push(r.encode());
        }
    }

    output.push(0x01); // Type section ID
    encode_u32(output, section.len() as u32);
    output.extend_from_slice(&section);
}

fn emit_import_section(output: &mut Vec<u8>, imports: &[WasmImport], type_indices: &[u32]) {
    let mut section = Vec::new();
    encode_u32(&mut section, imports.len() as u32);

    for (import, &type_idx) in imports.iter().zip(type_indices.iter()) {
        // Module name
        encode_u32(&mut section, import.module.len() as u32);
        section.extend_from_slice(import.module.as_bytes());
        // Field name
        encode_u32(&mut section, import.name.len() as u32);
        section.extend_from_slice(import.name.as_bytes());
        // Import kind: function (0x00)
        section.push(0x00);
        // Type index
        encode_u32(&mut section, type_idx);
    }

    output.push(0x02); // Import section ID
    encode_u32(output, section.len() as u32);
    output.extend_from_slice(&section);
}

fn emit_function_section_remapped(
    output: &mut Vec<u8>,
    functions: &[(u32, Vec<(u32, WasmType)>, Vec<u8>)],
    type_remap: &HashMap<u32, u32>,
) {
    let mut section = Vec::new();
    encode_u32(&mut section, functions.len() as u32);

    for (type_idx, _, _) in functions {
        let remapped = type_remap.get(type_idx).copied().unwrap_or(*type_idx);
        encode_u32(&mut section, remapped);
    }

    output.push(0x03); // Function section ID
    encode_u32(output, section.len() as u32);
    output.extend_from_slice(&section);
}

fn emit_memory_section(output: &mut Vec<u8>, initial: u32, max: Option<u32>) {
    let mut section = Vec::new();
    encode_u32(&mut section, 1); // One memory

    match max {
        Some(max_pages) => {
            section.push(0x01); // Has maximum
            encode_u32(&mut section, initial);
            encode_u32(&mut section, max_pages);
        }
        None => {
            section.push(0x00); // No maximum
            encode_u32(&mut section, initial);
        }
    }

    output.push(0x05); // Memory section ID
    encode_u32(output, section.len() as u32);
    output.extend_from_slice(&section);
}

fn emit_export_section_shifted(
    output: &mut Vec<u8>,
    exports: &[(String, u32)],
    import_offset: u32,
    export_memory: bool,
) {
    let total_exports = exports.len() + if export_memory { 1 } else { 0 };
    if total_exports == 0 {
        return;
    }

    let mut section = Vec::new();
    encode_u32(&mut section, total_exports as u32);

    // Export functions (shifted by import count)
    for (name, func_idx) in exports {
        encode_u32(&mut section, name.len() as u32);
        section.extend_from_slice(name.as_bytes());
        section.push(0x00); // Function export
        encode_u32(&mut section, func_idx + import_offset);
    }

    // Export memory
    if export_memory {
        let name = "memory";
        encode_u32(&mut section, name.len() as u32);
        section.extend_from_slice(name.as_bytes());
        section.push(0x02); // Memory export
        encode_u32(&mut section, 0);  // Memory index 0
    }

    output.push(0x07); // Export section ID
    encode_u32(output, section.len() as u32);
    output.extend_from_slice(&section);
}

fn emit_code_section(output: &mut Vec<u8>, functions: &[(Vec<(u32, WasmType)>, Vec<u8>)]) {
    let mut section = Vec::new();
    encode_u32(&mut section, functions.len() as u32);

    for (locals, code) in functions {
        let mut func_body = Vec::new();
        encode_u32(&mut func_body, locals.len() as u32);
        for (count, ty) in locals {
            encode_u32(&mut func_body, *count);
            func_body.push(ty.encode());
        }
        func_body.extend_from_slice(code);

        encode_u32(&mut section, func_body.len() as u32);
        section.extend_from_slice(&func_body);
    }

    output.push(0x0A); // Code section ID
    encode_u32(output, section.len() as u32);
    output.extend_from_slice(&section);
}

// LEB128 encoding
fn encode_u32(output: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Compile a MIR program to a WASM component binary
pub fn compile_to_wasm_component(mir: &MirProgram, target: Target) -> Result<Vec<u8>> {
    let mut compiler = WasmComponentCompiler::new(target)?;
    compiler.compile_program(mir)?;
    compiler.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aria_mir::*;
    use aria_lexer::Span;
    use smol_str::SmolStr;

    fn make_simple_program() -> MirProgram {
        let mut program = MirProgram::new();
        let span = Span::dummy();
        let fn_id = FunctionId(0);

        let mut func = MirFunction::new("add".into(), MirType::Int64, span);
        func.is_public = true;
        func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        func.locals.push(LocalDecl {
            name: Some("a".into()),
            ty: MirType::Int64,
            mutable: false,
            span,
        });
        func.locals.push(LocalDecl {
            name: Some("b".into()),
            ty: MirType::Int64,
            mutable: false,
            span,
        });
        func.locals.push(LocalDecl {
            name: Some("tmp".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        func.params = vec![Local(1), Local(2)];

        let block = BasicBlock {
            id: BlockId::ENTRY,
            statements: vec![
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(Local(3)),
                        Rvalue::BinaryOp(
                            BinOp::Add,
                            Operand::Copy(Place::from_local(Local(1))),
                            Operand::Copy(Place::from_local(Local(2))),
                        ),
                    ),
                    span,
                },
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(Local::RETURN),
                        Rvalue::Use(Operand::Copy(Place::from_local(Local(3)))),
                    ),
                    span,
                },
            ],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };
        func.blocks.push(block);
        program.functions.insert(fn_id, func);
        program
    }

    fn make_effectful_program(effects: &[&str]) -> MirProgram {
        let mut program = make_simple_program();
        let fn_id = FunctionId(0);

        if let Some(func) = program.functions.get_mut(&fn_id) {
            let mut row = EffectRow::new();
            for (i, name) in effects.iter().enumerate() {
                row = row.with_effect(EffectType::new(EffectId(i as u32), SmolStr::new(name)));
            }
            func.set_effect_row(row);
        }

        program
    }

    #[test]
    fn test_pure_component_no_imports() {
        let program = make_simple_program();
        let bytes = compile_to_wasm_component(&program, Target::Wasm32)
            .expect("Failed to compile pure component");

        // Verify WASM magic
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
        assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);

        // Should NOT have import section (0x02) for pure programs
        // But should have type (0x01), function (0x03), export (0x07), code (0x0A)
        assert!(bytes.len() > 8);
    }

    #[test]
    fn test_io_effect_adds_imports() {
        let program = make_effectful_program(&["IO"]);
        let mut compiler = WasmComponentCompiler::new(Target::Wasm32).unwrap();
        compiler.compile_program(&program).unwrap();

        assert!(compiler.import_count() > 0);
        assert!(compiler.get_imports().iter().any(|i| i.module == "wasi:io/streams"));
    }

    #[test]
    fn test_console_effect_adds_cli_imports() {
        let program = make_effectful_program(&["Console"]);
        let mut compiler = WasmComponentCompiler::new(Target::Wasm32).unwrap();
        compiler.compile_program(&program).unwrap();

        assert!(compiler.get_imports().iter().any(|i| i.module == "wasi:cli/stdout"));
        assert!(compiler.get_imports().iter().any(|i| i.module == "wasi:cli/stdin"));
    }

    #[test]
    fn test_multiple_effects_combine_imports() {
        let program = make_effectful_program(&["IO", "FileSystem", "Network"]);
        let mut compiler = WasmComponentCompiler::new(Target::Wasm32).unwrap();
        compiler.compile_program(&program).unwrap();

        assert!(compiler.get_imports().iter().any(|i| i.module == "wasi:io/streams"));
        assert!(compiler.get_imports().iter().any(|i| i.module == "wasi:filesystem/types"));
        assert!(compiler.get_imports().iter().any(|i| i.module == "wasi:http/outgoing-handler"));
    }

    #[test]
    fn test_component_binary_with_imports() {
        let program = make_effectful_program(&["Console"]);
        let bytes = compile_to_wasm_component(&program, Target::Wasm32)
            .expect("Failed to compile component with imports");

        // Verify WASM structure
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);

        // Should be larger than a pure module due to import section
        assert!(bytes.len() > 50);

        // Check for import section (0x02)
        let has_import_section = bytes.windows(1).enumerate().any(|(i, w)| {
            i > 7 && w[0] == 0x02 && i < 30 // Import section should appear early
        });
        assert!(has_import_section, "Missing import section in component binary");
    }

    #[test]
    fn test_pure_program_no_unnecessary_sections() {
        let program = make_simple_program();
        let bytes = compile_to_wasm_component(&program, Target::Wasm32).unwrap();

        // Parse section IDs from the binary
        let mut pos = 8; // Skip magic + version
        let mut section_ids = Vec::new();
        while pos < bytes.len() {
            let section_id = bytes[pos];
            section_ids.push(section_id);

            // Read section size (LEB128)
            pos += 1;
            let mut size: u32 = 0;
            let mut shift = 0;
            loop {
                if pos >= bytes.len() {
                    break;
                }
                let byte = bytes[pos];
                pos += 1;
                size |= ((byte & 0x7F) as u32) << shift;
                if byte & 0x80 == 0 {
                    break;
                }
                shift += 7;
            }
            pos += size as usize;
        }

        // Should have type (1), function (3), export (7), code (10)
        assert!(section_ids.contains(&0x01), "Missing type section");
        assert!(section_ids.contains(&0x03), "Missing function section");
        assert!(section_ids.contains(&0x07), "Missing export section");
        assert!(section_ids.contains(&0x0A), "Missing code section");

        // Should NOT have import (2) or memory (5) for pure integer programs
        assert!(!section_ids.contains(&0x02), "Unexpected import section");
        assert!(!section_ids.contains(&0x05), "Unexpected memory section");
    }

    #[test]
    fn test_effect_deduplication() {
        // Same effect from multiple functions shouldn't duplicate imports
        let mut program = MirProgram::new();
        let span = Span::dummy();

        for i in 0..3 {
            let fn_id = FunctionId(i);
            let mut func = MirFunction::new(
                SmolStr::new(format!("func_{i}")),
                MirType::Int64,
                span,
            );
            func.is_public = true;
            func.set_effect_row(
                EffectRow::new().with_effect(EffectType::new(EffectId(0), "IO".into())),
            );
            func.locals.push(LocalDecl {
                name: Some("return".into()),
                ty: MirType::Int64,
                mutable: true,
                span,
            });
            func.blocks.push(BasicBlock {
                id: BlockId::ENTRY,
                statements: vec![Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(Local::RETURN),
                        Rvalue::Use(Operand::Constant(Constant::Int(0))),
                    ),
                    span,
                }],
                terminator: Some(Terminator {
                    kind: TerminatorKind::Return,
                    span,
                }),
            });
            program.functions.insert(fn_id, func);
        }

        let mut compiler = WasmComponentCompiler::new(Target::Wasm32).unwrap();
        compiler.compile_program(&program).unwrap();

        // IO maps to 2 imports (read + write), not 6 (2 * 3 functions)
        assert_eq!(compiler.import_count(), 2);
    }
}
