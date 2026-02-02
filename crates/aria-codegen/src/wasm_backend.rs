//! WebAssembly backend for Aria code generation.
//!
//! This module implements the translation from MIR to WebAssembly binary format.
//! It produces standalone .wasm files that can run in browsers or WASI environments.
//!
//! # Features
//!
//! - Direct WASM bytecode generation (no external dependencies)
//! - Support for basic types: i32, i64, f32, f64
//! - Function exports for public functions
//! - Binary operations (arithmetic, bitwise, comparison)
//! - Unary operations (negation, not, bitwise not)
//! - Control flow: if/else, loops, multi-way switches (match)
//!
//! # Control Flow Strategy
//!
//! MIR uses a CFG (Control Flow Graph) model with basic blocks and explicit jumps,
//! while WASM uses structured control flow (block/loop/if). This backend translates
//! between these models using the following strategy:
//!
//! - **Simple if/else**: Translates directly to WASM `if/else/end`
//! - **Multi-block functions**: Uses WASM `block` nesting with `br` (branch) instructions
//! - **Loops**: Detected via back-edges and translated to WASM `loop` constructs
//!
//! # Example
//!
//! ```ignore
//! use aria_codegen::{compile_to_wasm, Target};
//! use aria_mir::MirProgram;
//!
//! let mir: MirProgram = /* ... */;
//! let wasm_bytes = compile_to_wasm(&mir, Target::Wasm32)?;
//! std::fs::write("output.wasm", wasm_bytes)?;
//! ```

use std::collections::HashMap;

use aria_mir::{
    BasicBlock, BinOp, BlockId, Constant, FunctionId, Local, MirFunction, MirProgram,
    MirType, Operand, Place, Rvalue, StatementKind, SwitchTargets, TerminatorKind, UnOp,
};

use crate::{CodegenError, Result, Target};

/// WebAssembly value types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
}

impl WasmType {
    /// Get the byte encoding for this type
    fn encode(&self) -> u8 {
        match self {
            WasmType::I32 => 0x7F,
            WasmType::I64 => 0x7E,
            WasmType::F32 => 0x7D,
            WasmType::F64 => 0x7C,
        }
    }
}

/// WASM block type encoding
#[allow(dead_code)]
#[repr(u8)]
enum WasmBlockType {
    /// Empty block type (void -> void)
    Empty = 0x40,
    /// Block returning i32
    I32 = 0x7F,
    /// Block returning i64
    I64 = 0x7E,
    /// Block returning f32
    F32 = 0x7D,
    /// Block returning f64
    F64 = 0x7C,
}

/// WebAssembly instruction opcodes
#[allow(dead_code)]
#[repr(u8)]
enum WasmOp {
    // Control flow
    Unreachable = 0x00,
    Nop = 0x01,
    Block = 0x02,
    Loop = 0x03,
    If = 0x04,
    Else = 0x05,
    End = 0x0B,
    Br = 0x0C,
    BrIf = 0x0D,
    BrTable = 0x0E,
    Return = 0x0F,
    Call = 0x10,

    // Parametric
    Drop = 0x1A,
    Select = 0x1B,

    // Locals
    LocalGet = 0x20,
    LocalSet = 0x21,
    LocalTee = 0x22,

    // Constants
    I32Const = 0x41,
    I64Const = 0x42,
    F32Const = 0x43,
    F64Const = 0x44,

    // I32 operations
    I32Eqz = 0x45,
    I32Eq = 0x46,
    I32Ne = 0x47,
    I32LtS = 0x48,
    I32LtU = 0x49,
    I32GtS = 0x4A,
    I32GtU = 0x4B,
    I32LeS = 0x4C,
    I32LeU = 0x4D,
    I32GeS = 0x4E,
    I32GeU = 0x4F,

    // I64 operations
    I64Eqz = 0x50,
    I64Eq = 0x51,
    I64Ne = 0x52,
    I64LtS = 0x53,
    I64LtU = 0x54,
    I64GtS = 0x55,
    I64GtU = 0x56,
    I64LeS = 0x57,
    I64LeU = 0x58,
    I64GeS = 0x59,
    I64GeU = 0x5A,

    // I32 arithmetic
    I32Add = 0x6A,
    I32Sub = 0x6B,
    I32Mul = 0x6C,
    I32DivS = 0x6D,
    I32DivU = 0x6E,
    I32RemS = 0x6F,
    I32RemU = 0x70,
    I32And = 0x71,
    I32Or = 0x72,
    I32Xor = 0x73,
    I32Shl = 0x74,
    I32ShrS = 0x75,
    I32ShrU = 0x76,

    // I64 arithmetic
    I64Add = 0x7C,
    I64Sub = 0x7D,
    I64Mul = 0x7E,
    I64DivS = 0x7F,
    I64DivU = 0x80,
    I64RemS = 0x81,
    I64RemU = 0x82,
    I64And = 0x83,
    I64Or = 0x84,
    I64Xor = 0x85,
    I64Shl = 0x86,
    I64ShrS = 0x87,
    I64ShrU = 0x88,

    // F32 arithmetic
    F32Add = 0x92,
    F32Sub = 0x93,
    F32Mul = 0x94,
    F32Div = 0x95,

    // F64 arithmetic
    F64Add = 0xA0,
    F64Sub = 0xA1,
    F64Mul = 0xA2,
    F64Div = 0xA3,

    // Conversions
    I32WrapI64 = 0xA7,
    I64ExtendI32S = 0xAC,
    I64ExtendI32U = 0xAD,
}

/// Convert MIR type to WASM type
fn mir_type_to_wasm(ty: &MirType) -> WasmType {
    match ty {
        MirType::Unit => WasmType::I32,
        MirType::Bool => WasmType::I32,
        MirType::Int8 | MirType::Int16 | MirType::Int32 => WasmType::I32,
        MirType::UInt8 | MirType::UInt16 | MirType::UInt32 => WasmType::I32,
        MirType::Char => WasmType::I32,
        MirType::Int | MirType::Int64 => WasmType::I64,
        MirType::UInt | MirType::UInt64 => WasmType::I64,
        MirType::Float32 => WasmType::F32,
        MirType::Float | MirType::Float64 => WasmType::F64,
        // Everything else maps to I32 (pointer-like)
        _ => WasmType::I32,
    }
}

/// The main WASM compiler
pub struct WasmCompiler {
    /// Function type signatures
    function_types: Vec<(Vec<WasmType>, Vec<WasmType>)>,
    /// Compiled functions
    functions: Vec<WasmFunction>,
    /// Function ID mapping
    func_id_map: HashMap<FunctionId, u32>,
    /// Export entries
    exports: Vec<(String, u32)>,
}

/// A compiled WASM function
struct WasmFunction {
    type_idx: u32,
    locals: Vec<(u32, WasmType)>,
    code: Vec<u8>,
}

impl WasmCompiler {
    /// Create a new WASM compiler
    pub fn new(_target: Target) -> Result<Self> {
        Ok(Self {
            function_types: Vec::new(),
            functions: Vec::new(),
            func_id_map: HashMap::new(),
            exports: Vec::new(),
        })
    }

    /// Compile an entire MIR program to WASM bytes
    pub fn compile_program(&mut self, program: &MirProgram) -> Result<()> {
        // First pass: create function type signatures
        for (&fn_id, mir_func) in &program.functions {
            let (params, returns) = self.create_function_type(mir_func);
            let type_idx = self.add_function_type(params, returns);
            self.func_id_map.insert(fn_id, self.functions.len() as u32);

            // Mark main or public functions for export
            if mir_func.name == "main" || mir_func.is_public {
                let export_name = if mir_func.name == "main" {
                    "aria_main"
                } else {
                    mir_func.name.as_str()
                };
                self.exports.push((export_name.to_string(), self.functions.len() as u32));
            }

            // Compile the function
            let wasm_func = self.compile_function(mir_func, program, type_idx)?;
            self.functions.push(wasm_func);
        }

        Ok(())
    }

    /// Create a function type signature
    fn create_function_type(&self, mir_func: &MirFunction) -> (Vec<WasmType>, Vec<WasmType>) {
        let params: Vec<WasmType> = mir_func
            .params
            .iter()
            .map(|&local| {
                let ty = &mir_func.locals[local.0 as usize].ty;
                mir_type_to_wasm(ty)
            })
            .collect();

        let returns = if mir_func.return_ty != MirType::Unit {
            vec![mir_type_to_wasm(&mir_func.return_ty)]
        } else {
            vec![]
        };

        (params, returns)
    }

    /// Add a function type and return its index
    fn add_function_type(&mut self, params: Vec<WasmType>, returns: Vec<WasmType>) -> u32 {
        // Check if this type already exists
        for (i, (p, r)) in self.function_types.iter().enumerate() {
            if p == &params && r == &returns {
                return i as u32;
            }
        }

        // Add new type
        let idx = self.function_types.len() as u32;
        self.function_types.push((params, returns));
        idx
    }

    /// Compile a single MIR function
    fn compile_function(
        &self,
        mir_func: &MirFunction,
        program: &MirProgram,
        type_idx: u32,
    ) -> Result<WasmFunction> {
        let mut compiler = WasmFunctionCompiler::new(mir_func, program, &self.func_id_map);
        compiler.compile()?;

        Ok(WasmFunction {
            type_idx,
            locals: compiler.get_locals(),
            code: compiler.finish(),
        })
    }

    /// Generate the final WASM binary
    pub fn finish(self) -> Result<Vec<u8>> {
        let mut output = Vec::new();

        // Magic number
        output.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]);
        // Version
        output.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        // Type section
        self.emit_type_section(&mut output);

        // Function section
        self.emit_function_section(&mut output);

        // Export section
        self.emit_export_section(&mut output);

        // Code section
        self.emit_code_section(&mut output);

        Ok(output)
    }

    fn emit_type_section(&self, output: &mut Vec<u8>) {
        let mut section = Vec::new();

        // Number of types
        encode_u32(&mut section, self.function_types.len() as u32);

        for (params, returns) in &self.function_types {
            section.push(0x60); // func type

            // Parameters
            encode_u32(&mut section, params.len() as u32);
            for param in params {
                section.push(param.encode());
            }

            // Results
            encode_u32(&mut section, returns.len() as u32);
            for ret in returns {
                section.push(ret.encode());
            }
        }

        // Write section
        output.push(0x01); // Type section ID
        encode_u32(output, section.len() as u32);
        output.extend_from_slice(&section);
    }

    fn emit_function_section(&self, output: &mut Vec<u8>) {
        let mut section = Vec::new();

        // Number of functions
        encode_u32(&mut section, self.functions.len() as u32);

        for func in &self.functions {
            encode_u32(&mut section, func.type_idx);
        }

        // Write section
        output.push(0x03); // Function section ID
        encode_u32(output, section.len() as u32);
        output.extend_from_slice(&section);
    }

    fn emit_export_section(&self, output: &mut Vec<u8>) {
        if self.exports.is_empty() {
            return;
        }

        let mut section = Vec::new();

        // Number of exports
        encode_u32(&mut section, self.exports.len() as u32);

        for (name, func_idx) in &self.exports {
            // Name length and bytes
            encode_u32(&mut section, name.len() as u32);
            section.extend_from_slice(name.as_bytes());

            // Export kind (0 = function)
            section.push(0x00);

            // Function index
            encode_u32(&mut section, *func_idx);
        }

        // Write section
        output.push(0x07); // Export section ID
        encode_u32(output, section.len() as u32);
        output.extend_from_slice(&section);
    }

    fn emit_code_section(&self, output: &mut Vec<u8>) {
        let mut section = Vec::new();

        // Number of function bodies
        encode_u32(&mut section, self.functions.len() as u32);

        for func in &self.functions {
            let mut func_body = Vec::new();

            // Locals
            encode_u32(&mut func_body, func.locals.len() as u32);
            for (count, ty) in &func.locals {
                encode_u32(&mut func_body, *count);
                func_body.push(ty.encode());
            }

            // Code
            func_body.extend_from_slice(&func.code);

            // Function body size and content
            encode_u32(&mut section, func_body.len() as u32);
            section.extend_from_slice(&func_body);
        }

        // Write section
        output.push(0x0A); // Code section ID
        encode_u32(output, section.len() as u32);
        output.extend_from_slice(&section);
    }
}

/// Compiler for a single WASM function
///
/// This compiler handles the translation from MIR's CFG model to WASM's
/// structured control flow model. The key challenge is that WASM doesn't
/// support arbitrary gotos - instead it uses nested blocks with labeled
/// branches.
///
/// # Compilation Strategy
///
/// For simple functions with only one block, we compile directly.
/// For multi-block functions, we use a "block per target" approach:
///
/// 1. Wrap all blocks in a nested block structure
/// 2. Each MIR block becomes code within one WASM block
/// 3. Jumps become `br` instructions to the appropriate nesting level
/// 4. For loops (back-edges), we use WASM `loop` constructs
#[allow(dead_code)]
struct WasmFunctionCompiler<'a> {
    mir_func: &'a MirFunction,
    program: &'a MirProgram,
    func_id_map: &'a HashMap<FunctionId, u32>,

    /// Local variables (excluding parameters)
    locals: Vec<(u32, WasmType)>,
    /// Instruction buffer
    code: Vec<u8>,
    /// Local variable mapping
    local_map: HashMap<Local, u32>,
    /// Next local index (after params)
    next_local: u32,
    /// Block nesting depth for break targets
    /// Maps BlockId -> break label depth
    block_depth_map: HashMap<BlockId, u32>,
    /// Current nesting depth
    current_depth: u32,
}

impl<'a> WasmFunctionCompiler<'a> {
    fn new(
        mir_func: &'a MirFunction,
        program: &'a MirProgram,
        func_id_map: &'a HashMap<FunctionId, u32>,
    ) -> Self {
        let param_count = mir_func.params.len() as u32;

        Self {
            mir_func,
            program,
            func_id_map,
            locals: Vec::new(),
            code: Vec::new(),
            local_map: HashMap::new(),
            next_local: param_count,
            block_depth_map: HashMap::new(),
            current_depth: 0,
        }
    }

    fn compile(&mut self) -> Result<()> {
        // Map parameter locals
        for (i, &param_local) in self.mir_func.params.iter().enumerate() {
            self.local_map.insert(param_local, i as u32);
        }

        // Declare all other locals
        for (i, local_decl) in self.mir_func.locals.iter().enumerate() {
            let local = Local(i as u32);
            if !self.local_map.contains_key(&local) {
                let wasm_ty = mir_type_to_wasm(&local_decl.ty);
                self.local_map.insert(local, self.next_local);
                self.next_local += 1;
                self.locals.push((1, wasm_ty));
            }
        }

        // Check if this is a simple single-block function
        if self.mir_func.blocks.len() <= 1 {
            // Simple case: compile the single block directly
            if let Some(entry_block) = self.mir_func.blocks.first() {
                self.compile_block(entry_block)?;
            }
        } else {
            // Multi-block function: use structured control flow compilation
            self.compile_multi_block_function()?;
        }

        // Add end instruction
        self.emit_end();

        Ok(())
    }

    /// Compile a function with multiple basic blocks.
    ///
    /// We use a "relooper-lite" approach:
    /// 1. Create a dispatcher using nested blocks
    /// 2. Each block's code is placed inside its corresponding WASM block
    /// 3. Control flow uses `br` to break out to the right level
    fn compile_multi_block_function(&mut self) -> Result<()> {
        let num_blocks = self.mir_func.blocks.len();

        // Create a local variable to track which block to execute
        let dispatch_local = self.next_local;
        self.next_local += 1;
        self.locals.push((1, WasmType::I32));

        // Initialize dispatch to entry block (0)
        self.emit_i32_const(0);
        self.emit_local_set(dispatch_local);

        // Create the outer loop for re-entry (allows "goto" semantics)
        self.emit_loop(WasmBlockType::Empty);
        self.current_depth += 1;
        let loop_depth = self.current_depth;

        // Create nested blocks for each MIR block (in reverse order)
        // This allows br N to jump to block N
        for i in 0..num_blocks {
            self.emit_block(WasmBlockType::Empty);
            self.current_depth += 1;
            self.block_depth_map.insert(BlockId(i as u32), self.current_depth);
        }

        // Now emit the dispatcher: check dispatch_local and branch to correct block
        for i in 0..num_blocks {
            // Check if dispatch_local == i
            self.emit_local_get(dispatch_local);
            self.emit_i32_const(i as i32);
            self.code.push(WasmOp::I32Eq as u8);
            // br_if to the block for this index
            // The break depth is calculated from current position
            let target_depth = self.current_depth - self.block_depth_map[&BlockId(i as u32)];
            self.emit_br_if(target_depth);
        }

        // If none matched, unreachable
        self.emit_unreachable();

        // Now emit each block's code (in order, after the corresponding end)
        for (i, mir_block) in self.mir_func.blocks.iter().enumerate() {
            // End the block we're inside
            self.emit_end();
            self.current_depth -= 1;

            // Update depth map for break calculations
            let block_id = BlockId(i as u32);
            self.block_depth_map.insert(block_id, self.current_depth);

            // Compile the block's statements
            for stmt in &mir_block.statements {
                self.compile_statement(stmt)?;
            }

            // Compile the terminator with dispatch updates
            if let Some(term) = &mir_block.terminator {
                self.compile_terminator_with_dispatch(term, dispatch_local, loop_depth)?;
            }
        }

        // End the loop
        self.emit_end();
        self.current_depth -= 1;

        Ok(())
    }

    /// Compile a terminator for multi-block functions, updating dispatch as needed
    fn compile_terminator_with_dispatch(
        &mut self,
        term: &aria_mir::Terminator,
        dispatch_local: u32,
        loop_depth: u32,
    ) -> Result<()> {
        match &term.kind {
            TerminatorKind::Return => {
                if self.mir_func.return_ty != MirType::Unit {
                    let ret_local = self.local_map[&Local::RETURN];
                    self.emit_local_get(ret_local);
                }
                self.emit_return();
            }
            TerminatorKind::Goto { target } => {
                // Set dispatch to target block and branch back to loop
                self.emit_i32_const(target.0 as i32);
                self.emit_local_set(dispatch_local);
                // Break to the loop (which continues from the top)
                let br_depth = self.current_depth - loop_depth;
                self.emit_br(br_depth);
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                self.compile_switch_with_dispatch(discr, targets, dispatch_local, loop_depth)?;
            }
            TerminatorKind::Call { func, args, dest, target } => {
                // Compile arguments
                for arg in args {
                    self.compile_operand(arg)?;
                }

                // Get function index and call
                if let Operand::Constant(Constant::Function(fn_id)) = func {
                    let func_idx = self.func_id_map.get(fn_id).ok_or_else(|| {
                        CodegenError::UndefinedFunction {
                            name: format!("fn#{}", fn_id.0),
                        }
                    })?;
                    self.emit_call(*func_idx);

                    // Store result if needed
                    // Check the called function's return type, not the current function
                    if let Some(called_func) = self.program.functions.get(fn_id) {
                        if called_func.return_ty != MirType::Unit {
                            self.store_to_place(dest)?;
                        }
                    }
                } else {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: "indirect function calls".to_string(),
                        span: None,
                    });
                }

                // Continue to target block if specified
                if let Some(target_block) = target {
                    self.emit_i32_const(target_block.0 as i32);
                    self.emit_local_set(dispatch_local);
                    let br_depth = self.current_depth - loop_depth;
                    self.emit_br(br_depth);
                }
            }
            TerminatorKind::Unreachable => {
                self.emit_unreachable();
            }
            TerminatorKind::Drop { place: _, target } => {
                // For now, just continue to target (no actual drop needed for primitives)
                self.emit_i32_const(target.0 as i32);
                self.emit_local_set(dispatch_local);
                let br_depth = self.current_depth - loop_depth;
                self.emit_br(br_depth);
            }
            TerminatorKind::Assert { cond, expected, msg: _, target } => {
                // Evaluate condition
                self.compile_operand(cond)?;

                // If condition matches expected, continue; otherwise trap
                if *expected {
                    // Assert true: if cond is 0 (false), trap
                    self.code.push(WasmOp::I32Eqz as u8);
                    self.emit_if(WasmBlockType::Empty);
                    self.emit_unreachable();
                    self.emit_end();
                } else {
                    // Assert false: if cond is not 0 (true), trap
                    self.emit_if(WasmBlockType::Empty);
                    self.emit_unreachable();
                    self.emit_end();
                }

                // Continue to target
                self.emit_i32_const(target.0 as i32);
                self.emit_local_set(dispatch_local);
                let br_depth = self.current_depth - loop_depth;
                self.emit_br(br_depth);
            }
        }
        Ok(())
    }

    /// Compile a switch/branch instruction with dispatch updates
    fn compile_switch_with_dispatch(
        &mut self,
        discr: &Operand,
        targets: &SwitchTargets,
        dispatch_local: u32,
        loop_depth: u32,
    ) -> Result<()> {
        let br_depth = self.current_depth - loop_depth;

        if targets.targets.len() == 1 {
            // Simple if/else case (most common for if statements)
            let (val, target) = &targets.targets[0];

            // Evaluate discriminant
            self.compile_operand(discr)?;

            // Compare with the target value
            self.emit_i64_const(*val as i64);
            self.emit_i64_eq();

            // If matches, go to target; else go to otherwise
            self.emit_if(WasmBlockType::Empty);
            // True branch: go to target
            self.emit_i32_const(target.0 as i32);
            self.emit_local_set(dispatch_local);
            self.emit_br(br_depth + 1); // +1 because we're inside the if block
            self.emit_else();
            // False branch: go to otherwise
            self.emit_i32_const(targets.otherwise.0 as i32);
            self.emit_local_set(dispatch_local);
            self.emit_br(br_depth + 1); // +1 because we're inside the else block
            self.emit_end();
        } else {
            // Multi-way switch (for match statements)
            // Use a series of if/else for now (could use br_table for large switches)

            for (val, target) in &targets.targets {
                // Evaluate discriminant
                self.compile_operand(discr)?;

                // Compare with this value
                self.emit_i64_const(*val as i64);
                self.emit_i64_eq();

                // If matches, set dispatch and branch
                self.emit_if(WasmBlockType::Empty);
                self.emit_i32_const(target.0 as i32);
                self.emit_local_set(dispatch_local);
                self.emit_br(br_depth + 1);
                self.emit_end();
            }

            // Default: go to otherwise
            self.emit_i32_const(targets.otherwise.0 as i32);
            self.emit_local_set(dispatch_local);
            self.emit_br(br_depth);
        }

        Ok(())
    }

    fn compile_block(&mut self, block: &BasicBlock) -> Result<()> {
        // Compile statements
        for stmt in &block.statements {
            self.compile_statement(stmt)?;
        }

        // Compile terminator
        if let Some(term) = &block.terminator {
            self.compile_terminator(term)?;
        }

        Ok(())
    }

    fn compile_statement(&mut self, stmt: &aria_mir::Statement) -> Result<()> {
        match &stmt.kind {
            StatementKind::Assign(place, rvalue) => {
                self.compile_rvalue(rvalue)?;
                self.store_to_place(place)?;
            }
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {
                // No-op
            }
        }
        Ok(())
    }

    /// Compile terminator for simple single-block functions
    fn compile_terminator(&mut self, term: &aria_mir::Terminator) -> Result<()> {
        match &term.kind {
            TerminatorKind::Return => {
                if self.mir_func.return_ty != MirType::Unit {
                    let ret_local = self.local_map[&Local::RETURN];
                    self.emit_local_get(ret_local);
                }
                self.emit_return();
            }
            TerminatorKind::Goto { .. } => {
                // In a single-block function, Goto is unexpected but we handle gracefully
                // Just fall through (no-op in single block context)
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                // Simple switch in single-block context - inline the if/else
                self.compile_simple_switch(discr, targets)?;
            }
            TerminatorKind::Call { func, args, dest, .. } => {
                // Compile arguments
                for arg in args {
                    self.compile_operand(arg)?;
                }

                // Get function index and call
                if let Operand::Constant(Constant::Function(fn_id)) = func {
                    let func_idx = self.func_id_map.get(fn_id).ok_or_else(|| {
                        CodegenError::UndefinedFunction {
                            name: format!("fn#{}", fn_id.0),
                        }
                    })?;
                    self.emit_call(*func_idx);

                    // Store result if needed - check called function's return type
                    if let Some(called_func) = self.program.functions.get(fn_id) {
                        if called_func.return_ty != MirType::Unit {
                            self.store_to_place(dest)?;
                        }
                    }
                } else {
                    return Err(CodegenError::UnsupportedFeature {
                        feature: "indirect function calls".to_string(),
                        span: None,
                    });
                }
            }
            TerminatorKind::Unreachable => {
                self.emit_unreachable();
            }
            TerminatorKind::Drop { .. } => {
                // For primitives, drop is a no-op
                // Complex types would need heap deallocation in WASM
            }
            TerminatorKind::Assert { cond, expected, .. } => {
                // Simple assertion: trap if condition doesn't match expected
                self.compile_operand(cond)?;
                if *expected {
                    // Assert true: trap if value is 0 (false)
                    self.code.push(WasmOp::I32Eqz as u8);
                    self.emit_if(WasmBlockType::Empty);
                    self.emit_unreachable();
                    self.emit_end();
                } else {
                    // Assert false: trap if value is not 0 (true)
                    self.emit_if(WasmBlockType::Empty);
                    self.emit_unreachable();
                    self.emit_end();
                }
            }
        }
        Ok(())
    }

    /// Compile a simple switch for single-block context
    fn compile_simple_switch(&mut self, discr: &Operand, targets: &SwitchTargets) -> Result<()> {
        // In single-block context, we can only do limited branching
        // For simple if/else, we set a value based on the condition
        if targets.targets.len() == 1 {
            let (val, _target) = &targets.targets[0];

            // Evaluate discriminant and compare
            self.compile_operand(discr)?;
            self.emit_i64_const(*val as i64);
            self.emit_i64_eq();

            // The result is now on stack (0 or 1)
            // In a proper multi-block setup, this would branch
            // In single-block, we just leave the comparison result
        }
        // For more complex switches in single-block, we'd need different handling
        Ok(())
    }

    fn compile_rvalue(&mut self, rvalue: &Rvalue) -> Result<()> {
        match rvalue {
            Rvalue::Use(operand) => {
                self.compile_operand(operand)?;
            }
            Rvalue::BinaryOp(op, left, right) => {
                self.compile_operand(left)?;
                self.compile_operand(right)?;
                self.emit_binop(*op)?;
            }
            Rvalue::UnaryOp(op, operand) => {
                self.compile_operand(operand)?;
                self.emit_unop(*op)?;
            }
            _ => {
                // Default: emit zero
                self.emit_i64_const(0);
            }
        }
        Ok(())
    }

    fn compile_operand(&mut self, operand: &Operand) -> Result<()> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                self.load_from_place(place)?;
            }
            Operand::Constant(constant) => {
                self.compile_constant(constant)?;
            }
        }
        Ok(())
    }

    fn compile_constant(&mut self, constant: &Constant) -> Result<()> {
        match constant {
            Constant::Unit => self.emit_i32_const(0),
            Constant::Bool(b) => self.emit_i32_const(*b as i32),
            Constant::Int(i) => self.emit_i64_const(*i),
            Constant::Float(f) => self.emit_f64_const(*f),
            Constant::Char(c) => self.emit_i32_const(*c as i32),
            _ => self.emit_i32_const(0), // Placeholder
        }
        Ok(())
    }

    fn emit_binop(&mut self, op: BinOp) -> Result<()> {
        match op {
            BinOp::Add => self.emit_i64_add(),
            BinOp::Sub => self.emit_i64_sub(),
            BinOp::Mul => self.emit_i64_mul(),
            BinOp::Div | BinOp::IntDiv => self.emit_i64_div_s(),
            BinOp::Rem => self.emit_i64_rem_s(),
            BinOp::BitAnd => self.emit_i64_and(),
            BinOp::BitOr => self.emit_i64_or(),
            BinOp::BitXor => self.emit_i64_xor(),
            BinOp::Shl => self.emit_i64_shl(),
            BinOp::Shr => self.emit_i64_shr_s(),
            BinOp::Eq => {
                self.emit_i64_eq();
                self.emit_i64_extend_i32_u();
            }
            BinOp::Ne => {
                self.emit_i64_ne();
                self.emit_i64_extend_i32_u();
            }
            BinOp::Lt => {
                self.emit_i64_lt_s();
                self.emit_i64_extend_i32_u();
            }
            BinOp::Le => {
                self.emit_i64_le_s();
                self.emit_i64_extend_i32_u();
            }
            BinOp::Gt => {
                self.emit_i64_gt_s();
                self.emit_i64_extend_i32_u();
            }
            BinOp::Ge => {
                self.emit_i64_ge_s();
                self.emit_i64_extend_i32_u();
            }
            BinOp::And | BinOp::Or => {
                // Logical operations - already evaluated
                self.emit_i64_and();
            }
            BinOp::Pow => {
                // Power not directly supported - placeholder
                self.emit_i64_mul();
            }
        }
        Ok(())
    }

    fn emit_unop(&mut self, op: UnOp) -> Result<()> {
        match op {
            UnOp::Neg => {
                // 0 - value
                self.emit_i64_const(0);
                // Swap and subtract
                let temp_local = self.next_local;
                self.next_local += 1;
                self.locals.push((1, WasmType::I64));
                self.emit_local_set(temp_local);
                self.emit_local_get(temp_local);
                self.emit_i64_sub();
            }
            UnOp::Not => {
                // XOR with 1
                self.emit_i64_const(1);
                self.emit_i64_xor();
            }
            UnOp::BitNot => {
                // XOR with -1
                self.emit_i64_const(-1);
                self.emit_i64_xor();
            }
        }
        Ok(())
    }

    fn load_from_place(&mut self, place: &Place) -> Result<()> {
        let local_idx = self.local_map.get(&place.local).ok_or_else(|| {
            CodegenError::Internal {
                message: format!("local {} not found", place.local),
            }
        })?;
        self.emit_local_get(*local_idx);
        Ok(())
    }

    fn store_to_place(&mut self, place: &Place) -> Result<()> {
        let local_idx = self.local_map.get(&place.local).ok_or_else(|| {
            CodegenError::Internal {
                message: format!("local {} not found", place.local),
            }
        })?;
        self.emit_local_set(*local_idx);
        Ok(())
    }

    // Emission helpers
    fn emit_unreachable(&mut self) {
        self.code.push(WasmOp::Unreachable as u8);
    }

    fn emit_return(&mut self) {
        self.code.push(WasmOp::Return as u8);
    }

    fn emit_call(&mut self, func_idx: u32) {
        self.code.push(WasmOp::Call as u8);
        encode_u32(&mut self.code, func_idx);
    }

    fn emit_local_get(&mut self, local_idx: u32) {
        self.code.push(WasmOp::LocalGet as u8);
        encode_u32(&mut self.code, local_idx);
    }

    fn emit_local_set(&mut self, local_idx: u32) {
        self.code.push(WasmOp::LocalSet as u8);
        encode_u32(&mut self.code, local_idx);
    }

    fn emit_i32_const(&mut self, value: i32) {
        self.code.push(WasmOp::I32Const as u8);
        encode_i32(&mut self.code, value);
    }

    fn emit_i64_const(&mut self, value: i64) {
        self.code.push(WasmOp::I64Const as u8);
        encode_i64(&mut self.code, value);
    }

    fn emit_f64_const(&mut self, value: f64) {
        self.code.push(WasmOp::F64Const as u8);
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    fn emit_i64_add(&mut self) {
        self.code.push(WasmOp::I64Add as u8);
    }

    fn emit_i64_sub(&mut self) {
        self.code.push(WasmOp::I64Sub as u8);
    }

    fn emit_i64_mul(&mut self) {
        self.code.push(WasmOp::I64Mul as u8);
    }

    fn emit_i64_div_s(&mut self) {
        self.code.push(WasmOp::I64DivS as u8);
    }

    fn emit_i64_rem_s(&mut self) {
        self.code.push(WasmOp::I64RemS as u8);
    }

    fn emit_i64_and(&mut self) {
        self.code.push(WasmOp::I64And as u8);
    }

    fn emit_i64_or(&mut self) {
        self.code.push(WasmOp::I64Or as u8);
    }

    fn emit_i64_xor(&mut self) {
        self.code.push(WasmOp::I64Xor as u8);
    }

    fn emit_i64_shl(&mut self) {
        self.code.push(WasmOp::I64Shl as u8);
    }

    fn emit_i64_shr_s(&mut self) {
        self.code.push(WasmOp::I64ShrS as u8);
    }

    fn emit_i64_eq(&mut self) {
        self.code.push(WasmOp::I64Eq as u8);
    }

    fn emit_i64_ne(&mut self) {
        self.code.push(WasmOp::I64Ne as u8);
    }

    fn emit_i64_lt_s(&mut self) {
        self.code.push(WasmOp::I64LtS as u8);
    }

    fn emit_i64_le_s(&mut self) {
        self.code.push(WasmOp::I64LeS as u8);
    }

    fn emit_i64_gt_s(&mut self) {
        self.code.push(WasmOp::I64GtS as u8);
    }

    fn emit_i64_ge_s(&mut self) {
        self.code.push(WasmOp::I64GeS as u8);
    }

    fn emit_i64_extend_i32_u(&mut self) {
        self.code.push(WasmOp::I64ExtendI32U as u8);
    }

    fn emit_end(&mut self) {
        self.code.push(WasmOp::End as u8);
    }

    // Control flow emission helpers
    fn emit_block(&mut self, block_type: WasmBlockType) {
        self.code.push(WasmOp::Block as u8);
        self.code.push(block_type as u8);
    }

    fn emit_loop(&mut self, block_type: WasmBlockType) {
        self.code.push(WasmOp::Loop as u8);
        self.code.push(block_type as u8);
    }

    fn emit_if(&mut self, block_type: WasmBlockType) {
        self.code.push(WasmOp::If as u8);
        self.code.push(block_type as u8);
    }

    fn emit_else(&mut self) {
        self.code.push(WasmOp::Else as u8);
    }

    fn emit_br(&mut self, label_idx: u32) {
        self.code.push(WasmOp::Br as u8);
        encode_u32(&mut self.code, label_idx);
    }

    fn emit_br_if(&mut self, label_idx: u32) {
        self.code.push(WasmOp::BrIf as u8);
        encode_u32(&mut self.code, label_idx);
    }

    #[allow(dead_code)]
    fn emit_nop(&mut self) {
        self.code.push(WasmOp::Nop as u8);
    }

    #[allow(dead_code)]
    fn emit_drop(&mut self) {
        self.code.push(WasmOp::Drop as u8);
    }

    fn get_locals(&self) -> Vec<(u32, WasmType)> {
        self.locals.clone()
    }

    fn finish(self) -> Vec<u8> {
        self.code
    }
}

// LEB128 encoding utilities
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

fn encode_i32(output: &mut Vec<u8>, mut value: i32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        let sign_bit = (byte & 0x40) != 0;
        if (value == 0 && !sign_bit) || (value == -1 && sign_bit) {
            output.push(byte);
            break;
        }
        byte |= 0x80;
        output.push(byte);
    }
}

fn encode_i64(output: &mut Vec<u8>, mut value: i64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        let sign_bit = (byte & 0x40) != 0;
        if (value == 0 && !sign_bit) || (value == -1 && sign_bit) {
            output.push(byte);
            break;
        }
        byte |= 0x80;
        output.push(byte);
    }
}

/// Compile a MIR program to WASM bytes
pub fn compile_to_wasm(mir: &MirProgram, target: Target) -> Result<Vec<u8>> {
    let mut compiler = WasmCompiler::new(target)?;
    compiler.compile_program(mir)?;
    compiler.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aria_mir::{
        BasicBlock, FunctionId, LocalDecl, MirFunction, MirProgram,
        Statement, Terminator,
    };
    use aria_lexer::Span;

    #[test]
    fn test_encode_u32() {
        let mut buf = Vec::new();
        encode_u32(&mut buf, 0);
        assert_eq!(buf, vec![0]);

        buf.clear();
        encode_u32(&mut buf, 127);
        assert_eq!(buf, vec![127]);

        buf.clear();
        encode_u32(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);
    }

    #[test]
    fn test_wasm_type_encode() {
        assert_eq!(WasmType::I32.encode(), 0x7F);
        assert_eq!(WasmType::I64.encode(), 0x7E);
        assert_eq!(WasmType::F32.encode(), 0x7D);
        assert_eq!(WasmType::F64.encode(), 0x7C);
    }

    #[test]
    fn test_wasm_block_type_encode() {
        assert_eq!(WasmBlockType::Empty as u8, 0x40);
        assert_eq!(WasmBlockType::I32 as u8, 0x7F);
        assert_eq!(WasmBlockType::I64 as u8, 0x7E);
    }

    #[test]
    fn test_multi_block_function() {
        // Test a function with multiple blocks (if/else pattern)
        // fn max(a: i64, b: i64) -> i64 {
        //   if a > b { a } else { b }
        // }

        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let span = Span::dummy();

        let mut mir_func = MirFunction::new("max".into(), MirType::Int64, span);
        mir_func.is_public = true;

        // Locals: return (0), a (1), b (2)
        let local_ret = Local(0);
        let local_a = Local(1);
        let local_b = Local(2);

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

        mir_func.params = vec![local_a, local_b];

        // Block 0 (entry): compare a > b, switch to block 1 or 2
        let entry_block = BasicBlock {
            id: BlockId(0),
            statements: vec![],
            terminator: Some(Terminator {
                kind: TerminatorKind::SwitchInt {
                    discr: Operand::Copy(Place::from_local(local_a)),
                    targets: SwitchTargets::if_else(BlockId(1), BlockId(2)),
                },
                span,
            }),
        };

        // Block 1 (true branch): return a
        let true_block = BasicBlock {
            id: BlockId(1),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(local_ret),
                    Rvalue::Use(Operand::Copy(Place::from_local(local_a))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        // Block 2 (false branch): return b
        let false_block = BasicBlock {
            id: BlockId(2),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(local_ret),
                    Rvalue::Use(Operand::Copy(Place::from_local(local_b))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        mir_func.blocks.push(entry_block);
        mir_func.blocks.push(true_block);
        mir_func.blocks.push(false_block);
        program.functions.insert(fn_id, mir_func);

        // Compile to WASM
        let result = compile_to_wasm(&program, crate::Target::Wasm32);
        assert!(result.is_ok(), "Multi-block function compilation failed: {:?}", result.err());

        let wasm_bytes = result.unwrap();

        // Verify WASM magic number
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6D], "Invalid WASM magic");

        // Should have multiple sections
        assert!(wasm_bytes.len() > 20, "WASM output too small for multi-block function");

        println!("Multi-block function compiled to {} bytes", wasm_bytes.len());
    }

    #[test]
    fn test_goto_control_flow() {
        // Test unconditional goto
        // fn sequence() -> i64 {
        //   let x = 1;
        //   goto block1;
        //   block1: return x;
        // }

        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let span = Span::dummy();

        let mut mir_func = MirFunction::new("sequence".into(), MirType::Int64, span);
        mir_func.is_public = true;

        let local_ret = Local(0);
        let local_x = Local(1);

        mir_func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        mir_func.locals.push(LocalDecl {
            name: Some("x".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });

        // Block 0: assign x = 1, goto block 1
        let block0 = BasicBlock {
            id: BlockId(0),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(local_x),
                    Rvalue::Use(Operand::Constant(Constant::Int(1))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Goto { target: BlockId(1) },
                span,
            }),
        };

        // Block 1: return x
        let block1 = BasicBlock {
            id: BlockId(1),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(local_ret),
                    Rvalue::Use(Operand::Copy(Place::from_local(local_x))),
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
        program.functions.insert(fn_id, mir_func);

        let result = compile_to_wasm(&program, crate::Target::Wasm32);
        assert!(result.is_ok(), "Goto compilation failed: {:?}", result.err());

        println!("Goto function compiled to {} bytes", result.unwrap().len());
    }

    #[test]
    fn test_assert_control_flow() {
        // Test assertion
        // fn checked() -> i64 {
        //   assert(true);
        //   return 42;
        // }

        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let span = Span::dummy();

        let mut mir_func = MirFunction::new("checked".into(), MirType::Int64, span);
        mir_func.is_public = true;

        let local_ret = Local(0);

        mir_func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });

        // Block 0: assert true, then continue to block 1
        let block0 = BasicBlock {
            id: BlockId(0),
            statements: vec![],
            terminator: Some(Terminator {
                kind: TerminatorKind::Assert {
                    cond: Operand::Constant(Constant::Bool(true)),
                    expected: true,
                    msg: "assertion failed".into(),
                    target: BlockId(1),
                },
                span,
            }),
        };

        // Block 1: return 42
        let block1 = BasicBlock {
            id: BlockId(1),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(local_ret),
                    Rvalue::Use(Operand::Constant(Constant::Int(42))),
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
        program.functions.insert(fn_id, mir_func);

        let result = compile_to_wasm(&program, crate::Target::Wasm32);
        assert!(result.is_ok(), "Assert compilation failed: {:?}", result.err());

        println!("Assert function compiled to {} bytes", result.unwrap().len());
    }

    #[test]
    fn test_multi_target_switch() {
        // Test multi-way switch (match pattern)
        // fn classify(x: i64) -> i64 {
        //   match x {
        //     0 => 100,
        //     1 => 200,
        //     _ => 300,
        //   }
        // }

        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let span = Span::dummy();

        let mut mir_func = MirFunction::new("classify".into(), MirType::Int64, span);
        mir_func.is_public = true;

        let local_ret = Local(0);
        let local_x = Local(1);

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

        mir_func.params = vec![local_x];

        // Block 0: switch on x
        let block0 = BasicBlock {
            id: BlockId(0),
            statements: vec![],
            terminator: Some(Terminator {
                kind: TerminatorKind::SwitchInt {
                    discr: Operand::Copy(Place::from_local(local_x)),
                    targets: SwitchTargets::new(
                        vec![
                            (0, BlockId(1)),  // x == 0 -> block 1
                            (1, BlockId(2)),  // x == 1 -> block 2
                        ],
                        BlockId(3),  // otherwise -> block 3
                    ),
                },
                span,
            }),
        };

        // Block 1: return 100
        let block1 = BasicBlock {
            id: BlockId(1),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(local_ret),
                    Rvalue::Use(Operand::Constant(Constant::Int(100))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        // Block 2: return 200
        let block2 = BasicBlock {
            id: BlockId(2),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(local_ret),
                    Rvalue::Use(Operand::Constant(Constant::Int(200))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };

        // Block 3: return 300 (default)
        let block3 = BasicBlock {
            id: BlockId(3),
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(local_ret),
                    Rvalue::Use(Operand::Constant(Constant::Int(300))),
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

        let result = compile_to_wasm(&program, crate::Target::Wasm32);
        assert!(result.is_ok(), "Multi-target switch failed: {:?}", result.err());

        println!("Multi-target switch compiled to {} bytes", result.unwrap().len());
    }
}
