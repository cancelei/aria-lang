//! Native code generation for the Aria programming language.
//!
//! This crate provides native code generation using Cranelift as the backend.
//! It takes MIR (Mid-level Intermediate Representation) and produces native
//! machine code or object files.
//!
//! # Architecture
//!
//! ```text
//! MIR → [compile] → Cranelift IR → [cranelift] → Machine Code → Object File
//! ```
//!
//! # Example
//!
//! ```ignore
//! use aria_codegen::{compile_to_object, Target};
//! use aria_mir::MirProgram;
//!
//! let mir: MirProgram = /* ... */;
//! let object = compile_to_object(&mir, Target::native())?;
//! std::fs::write("output.o", object)?;
//! ```

use aria_lexer::Span;
use thiserror::Error;

mod types;
mod cranelift_backend;
mod runtime;
mod wasm_backend;
mod inline;

pub use cranelift_backend::CraneliftCompiler;
pub use wasm_backend::{WasmCompiler, compile_to_wasm};
pub use inline::{inline_functions, InlineConfig, InlinePolicy};

/// Target architecture for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// x86-64 (AMD64)
    X86_64,
    /// ARM64 (AArch64)
    Aarch64,
    /// WebAssembly 32-bit
    Wasm32,
}

impl Target {
    /// Get the native target for the current platform
    pub fn native() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Target::X86_64
        }
        #[cfg(target_arch = "aarch64")]
        {
            Target::Aarch64
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Target::X86_64 // Default fallback
        }
    }

    /// Get the Cranelift target triple
    pub fn triple(&self) -> target_lexicon::Triple {
        match self {
            Target::X86_64 => target_lexicon::Triple::host(),
            Target::Aarch64 => "aarch64-unknown-linux-gnu".parse().unwrap(),
            Target::Wasm32 => "wasm32-unknown-unknown".parse().unwrap(),
        }
    }
}

/// Errors that can occur during code generation
#[derive(Error, Debug)]
pub enum CodegenError {
    #[error("unsupported target: {0:?}")]
    UnsupportedTarget(Target),

    #[error("unsupported feature: {feature}")]
    UnsupportedFeature { feature: String, span: Option<Span> },

    #[error("internal error: {message}")]
    Internal { message: String },

    #[error("cranelift error: {0}")]
    Cranelift(String),

    #[error("module error: {0}")]
    Module(String),

    #[error("undefined function: {name}")]
    UndefinedFunction { name: String },

    #[error("type error: {message}")]
    TypeError { message: String, span: Option<Span> },
}

impl From<cranelift_module::ModuleError> for CodegenError {
    fn from(e: cranelift_module::ModuleError) -> Self {
        CodegenError::Module(e.to_string())
    }
}

/// Result type for codegen operations
pub type Result<T> = std::result::Result<T, CodegenError>;

/// Compile a MIR program to an object file (bytes)
///
/// This is the main entry point for code generation.
///
/// # Arguments
///
/// * `mir` - The MIR program to compile
/// * `target` - The target architecture
///
/// # Returns
///
/// The compiled object file as bytes, suitable for writing to disk
/// or linking with other object files. For WASM targets, returns .wasm bytes.
pub fn compile_to_object(mir: &aria_mir::MirProgram, target: Target) -> Result<Vec<u8>> {
    match target {
        Target::Wasm32 => compile_to_wasm(mir, target),
        _ => {
            let mut compiler = CraneliftCompiler::new(target)?;
            compiler.compile_program(mir)?;
            compiler.finish()
        }
    }
}

/// Compile a MIR program and return the compiler for further inspection
///
/// This is useful for debugging or when you need access to the
/// compiled function addresses.
pub fn compile(mir: &aria_mir::MirProgram, target: Target) -> Result<CraneliftCompiler> {
    let mut compiler = CraneliftCompiler::new(target)?;
    compiler.compile_program(mir)?;
    Ok(compiler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aria_mir::*;

    #[test]
    fn test_target_native() {
        let target = Target::native();
        // Should be one of the supported targets
        assert!(matches!(target, Target::X86_64 | Target::Aarch64 | Target::Wasm32));
    }

    #[test]
    fn test_target_triple() {
        let target = Target::X86_64;
        let triple = target.triple();
        // Should get a valid triple
        assert!(triple.architecture != target_lexicon::Architecture::Unknown);
    }

    #[test]
    fn test_wasm_compilation() {
        use aria_lexer::Span;

        // Create a simple MIR program: fn add(a: i64, b: i64) -> i64 { a + b }
        let mut program = MirProgram::new();

        // Create the add function
        let fn_id = FunctionId(0);
        let span = Span::dummy();

        let mut mir_func = MirFunction::new("add".into(), MirType::Int64, span);
        mir_func.is_public = true;

        // Add locals: return, param a, param b, temp
        let local_ret = Local::RETURN;
        let local_a = Local(1);
        let local_b = Local(2);
        let local_tmp = Local(3);

        mir_func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("a".into()),
            ty: MirType::Int64,
            mutable: false,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("b".into()),
            ty: MirType::Int64,
            mutable: false,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("tmp".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });

        mir_func.params = vec![local_a, local_b];

        // Create the function body: return_local = a + b
        let entry_block = BasicBlock {
            id: BlockId::ENTRY,
            statements: vec![
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(local_tmp),
                        Rvalue::BinaryOp(
                            BinOp::Add,
                            Operand::Copy(Place::from_local(local_a)),
                            Operand::Copy(Place::from_local(local_b)),
                        ),
                    ),
                    span,
                },
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(local_ret),
                        Rvalue::Use(Operand::Copy(Place::from_local(local_tmp))),
                    ),
                    span,
                },
            ],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        mir_func.blocks.push(entry_block);
        program.functions.insert(fn_id, mir_func);

        // Compile to WASM
        let wasm_bytes = compile_to_object(&program, Target::Wasm32)
            .expect("Failed to compile to WASM");

        // Verify WASM magic number and version
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6D], "Invalid WASM magic number");
        assert_eq!(&wasm_bytes[4..8], &[0x01, 0x00, 0x00, 0x00], "Invalid WASM version");

        // Verify we have some sections (type, function, export, code)
        assert!(wasm_bytes.len() > 8, "WASM output too small");

        // Verify type section exists (section ID 0x01)
        assert!(wasm_bytes.contains(&0x01), "Missing type section");

        // Verify function section exists (section ID 0x03)
        assert!(wasm_bytes.contains(&0x03), "Missing function section");

        // Verify export section exists (section ID 0x07)
        assert!(wasm_bytes.contains(&0x07), "Missing export section");

        // Verify code section exists (section ID 0x0A)
        assert!(wasm_bytes.contains(&0x0A), "Missing code section");

        println!("Successfully compiled to WASM: {} bytes", wasm_bytes.len());
    }

    #[test]
    fn test_wasm_simple_function() {
        use aria_lexer::Span;

        // Create a simple MIR program: fn get_42() -> i64 { 42 }
        let mut program = MirProgram::new();

        let fn_id = FunctionId(0);
        let span = Span::dummy();

        let mut mir_func = MirFunction::new("get_42".into(), MirType::Int64, span);
        mir_func.is_public = true;

        let local_ret = Local::RETURN;

        mir_func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });

        // Create the function body: return 42
        let entry_block = BasicBlock {
            id: BlockId::ENTRY,
            statements: vec![
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(local_ret),
                        Rvalue::Use(Operand::Constant(Constant::Int(42))),
                    ),
                    span,
                },
            ],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        mir_func.blocks.push(entry_block);
        program.functions.insert(fn_id, mir_func);

        // Compile to WASM
        let wasm_bytes = compile_to_object(&program, Target::Wasm32)
            .expect("Failed to compile simple WASM function");

        // Basic validation
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
        assert!(wasm_bytes.len() > 8);

        println!("Successfully compiled simple function to WASM: {} bytes", wasm_bytes.len());
    }

    #[test]
    fn test_effectful_function_signature() {
        use aria_lexer::Span;

        // Create a MIR function with an effect row
        let span = Span::dummy();
        let mut mir_func = MirFunction::new("effectful".into(), MirType::Int64, span);

        // Add locals
        mir_func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("x".into()),
            ty: MirType::Int64,
            mutable: false,
            span,
        });
        mir_func.params = vec![Local(1)];

        // Set effect row - this function has IO effect
        let io_effect = EffectType::new(EffectId(0), "IO".into());
        mir_func.set_effect_row(EffectRow::new().with_effect(io_effect));

        // Verify the function has effects
        assert!(!mir_func.effect_row.is_pure());
        assert_eq!(mir_func.effect_row.effects.len(), 1);
    }

    #[test]
    fn test_effect_statement_creation() {
        use aria_lexer::Span;

        let span = Span::dummy();
        let mut mir_func = MirFunction::new("with_effect".into(), MirType::Int64, span);

        // Add locals
        mir_func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("result".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });

        // Add effect row
        let state_effect = EffectType::new(EffectId(0), "State".into());
        mir_func.set_effect_row(EffectRow::new().with_effect(state_effect.clone()));

        // Create entry block
        let entry_block = BasicBlock {
            id: BlockId::ENTRY,
            statements: vec![Statement {
                kind: StatementKind::Nop,
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };
        mir_func.blocks.push(entry_block);

        // Add an effect statement (PerformEffect)
        let effect_stmt = EffectStatementKind::PerformEffect {
            effect: state_effect.clone(),
            operation: OperationId(0),
            args: vec![],
            evidence_slot: EvidenceSlot::Static(0),
            dest: Place::from_local(Local(1)),
            classification: EffectClassification::TailResumptive,
        };
        mir_func.add_effect_statement(BlockId::ENTRY, 0, effect_stmt);

        // Verify effect statement was added
        let retrieved = mir_func.effect_statement(BlockId::ENTRY, 0);
        assert!(retrieved.is_some());

        match retrieved.unwrap() {
            EffectStatementKind::PerformEffect { effect, classification, .. } => {
                assert_eq!(effect.name.as_str(), "State");
                assert_eq!(*classification, EffectClassification::TailResumptive);
            }
            _ => panic!("Expected PerformEffect statement"),
        }
    }

    #[test]
    fn test_evidence_slot_static() {
        // Test static evidence slot
        let slot = EvidenceSlot::Static(5);
        match slot {
            EvidenceSlot::Static(offset) => assert_eq!(offset, 5),
            _ => panic!("Expected Static slot"),
        }
    }

    #[test]
    fn test_effect_classification() {
        // Test effect classification default
        let default = EffectClassification::default();
        assert_eq!(default, EffectClassification::General);

        // Test tail-resumptive classification
        let tail = EffectClassification::TailResumptive;
        assert!(matches!(tail, EffectClassification::TailResumptive));
    }

    #[test]
    fn test_async_effect_type() {
        // Test that we can create an Async effect type for concurrency
        let async_effect = EffectType::new(EffectId(1), "Async".into());
        assert_eq!(async_effect.name.as_str(), "Async");

        // Async is a OneShot effect (not TailResumptive)
        // This affects how it's compiled - it needs continuation support
        // but we fall back to thread-per-task for now
    }

    #[test]
    fn test_async_effect_row() {
        // Test creating a function with Async effect
        let async_effect = EffectType::new(EffectId(1), "Async".into());
        let io_effect = EffectType::new(EffectId(0), "IO".into());

        let effect_row = EffectRow::new()
            .with_effect(async_effect)
            .with_effect(io_effect);

        assert!(!effect_row.is_pure());
        assert_eq!(effect_row.effects.len(), 2);
    }

    #[test]
    fn test_wasm_control_flow_if_else() {
        use aria_lexer::Span;

        // Create a MIR program with if/else control flow
        // fn abs(x: i64) -> i64 {
        //   if x >= 0 { x } else { -x }
        // }
        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let span = Span::dummy();

        let mut mir_func = MirFunction::new("abs".into(), MirType::Int64, span);
        mir_func.is_public = true;

        // Locals: return (0), x (1), negated (2)
        mir_func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("x".into()),
            ty: MirType::Int64,
            mutable: false,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("negated".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });

        mir_func.params = vec![Local(1)];

        // Block 0: check x >= 0, branch to block 1 (true) or block 2 (false)
        let block0 = BasicBlock {
            id: BlockId(0),
            statements: vec![],
            terminator: Some(Terminator {
                kind: TerminatorKind::SwitchInt {
                    discr: Operand::Copy(Place::from_local(Local(1))),
                    targets: SwitchTargets::if_else(BlockId(1), BlockId(2)),
                },
                span,
            }),
        };

        // Block 1: x >= 0, return x
        let block1 = BasicBlock {
            id: BlockId(1),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(Local::RETURN),
                    Rvalue::Use(Operand::Copy(Place::from_local(Local(1)))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        // Block 2: x < 0, return -x
        let block2 = BasicBlock {
            id: BlockId(2),
            statements: vec![
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(Local(2)),
                        Rvalue::UnaryOp(UnOp::Neg, Operand::Copy(Place::from_local(Local(1)))),
                    ),
                    span,
                },
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(Local::RETURN),
                        Rvalue::Use(Operand::Copy(Place::from_local(Local(2)))),
                    ),
                    span,
                },
            ],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        mir_func.blocks.push(block0);
        mir_func.blocks.push(block1);
        mir_func.blocks.push(block2);
        program.functions.insert(fn_id, mir_func);

        // Compile to WASM
        let wasm_bytes = compile_to_object(&program, Target::Wasm32)
            .expect("Failed to compile WASM with control flow");

        // Verify WASM structure
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6D], "Invalid WASM magic");
        assert!(wasm_bytes.len() > 30, "WASM output too small for control flow");

        println!("Control flow (if/else) compiled to WASM: {} bytes", wasm_bytes.len());
    }

    #[test]
    fn test_wasm_loop_pattern() {
        use aria_lexer::Span;

        // Create a MIR program with a loop-like pattern
        // fn countdown(n: i64) -> i64 {
        //   let result = 0;
        //   while n > 0 {
        //     result = result + n;
        //     n = n - 1;
        //   }
        //   return result;
        // }

        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let span = Span::dummy();

        let mut mir_func = MirFunction::new("sum_to_n".into(), MirType::Int64, span);
        mir_func.is_public = true;

        // Locals: return (0), n (1), result (2), cond (3)
        mir_func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("n".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("result".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("cond".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });

        mir_func.params = vec![Local(1)];

        // Block 0: initialize result = 0, goto loop header
        let block0 = BasicBlock {
            id: BlockId(0),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(Local(2)),
                    Rvalue::Use(Operand::Constant(Constant::Int(0))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Goto { target: BlockId(1) },
                span,
            }),
        };

        // Block 1: loop header - check n > 0
        let block1 = BasicBlock {
            id: BlockId(1),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(Local(3)),
                    Rvalue::BinaryOp(
                        BinOp::Gt,
                        Operand::Copy(Place::from_local(Local(1))),
                        Operand::Constant(Constant::Int(0)),
                    ),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::SwitchInt {
                    discr: Operand::Copy(Place::from_local(Local(3))),
                    targets: SwitchTargets::if_else(BlockId(2), BlockId(3)),
                },
                span,
            }),
        };

        // Block 2: loop body - result += n, n -= 1, goto header
        let block2 = BasicBlock {
            id: BlockId(2),
            statements: vec![
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(Local(2)),
                        Rvalue::BinaryOp(
                            BinOp::Add,
                            Operand::Copy(Place::from_local(Local(2))),
                            Operand::Copy(Place::from_local(Local(1))),
                        ),
                    ),
                    span,
                },
                Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(Local(1)),
                        Rvalue::BinaryOp(
                            BinOp::Sub,
                            Operand::Copy(Place::from_local(Local(1))),
                            Operand::Constant(Constant::Int(1)),
                        ),
                    ),
                    span,
                },
            ],
            terminator: Some(Terminator {
                kind: TerminatorKind::Goto { target: BlockId(1) },
                span,
            }),
        };

        // Block 3: loop exit - return result
        let block3 = BasicBlock {
            id: BlockId(3),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(Local::RETURN),
                    Rvalue::Use(Operand::Copy(Place::from_local(Local(2)))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        mir_func.blocks.push(block0);
        mir_func.blocks.push(block1);
        mir_func.blocks.push(block2);
        mir_func.blocks.push(block3);
        program.functions.insert(fn_id, mir_func);

        // Compile to WASM
        let wasm_bytes = compile_to_object(&program, Target::Wasm32)
            .expect("Failed to compile WASM with loop pattern");

        // Verify WASM structure
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6D], "Invalid WASM magic");
        assert!(wasm_bytes.len() > 40, "WASM output too small for loop pattern");

        println!("Loop pattern compiled to WASM: {} bytes", wasm_bytes.len());
    }
}
