//! Cranelift backend for Aria code generation.
//!
//! This module implements the translation from MIR to Cranelift IR
//! and produces native machine code.

use std::collections::HashMap;

use aria_mir::{
    AggregateKind, BasicBlock, BinOp, BlockId, BuiltinKind, Constant, FunctionId,
    Linkage as MirLinkage, Local, MirFunction, MirProgram, MirType, Operand, Place, PlaceElem,
    Rvalue, StatementKind, SwitchTargets, TerminatorKind, UnOp,
    // Effect system types
    EffectClassification, EffectStatementKind, EffectType,
    EvidenceSlot, HandlerId,
};
use cranelift_codegen::ir::{
    condcodes::{FloatCC, IntCC},
    types, AbiParam, Block, FuncRef, Function, GlobalValue, InstBuilder, MemFlags, Signature,
    Type as ClifType, Value,
};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use rustc_hash::FxHashMap;

use crate::runtime::RuntimeFunctions;
use crate::types::mir_type_to_clif;
use crate::{CodegenError, Result, Target};

/// Runtime function references declared in a specific function
#[derive(Default)]
#[allow(dead_code)]
struct RuntimeFuncRefs {
    // I/O
    print_int: Option<FuncRef>,
    print_float: Option<FuncRef>,
    print_bool: Option<FuncRef>,
    print_string: Option<FuncRef>,
    print_newline: Option<FuncRef>,

    // String operations
    string_len: Option<FuncRef>,
    string_contains: Option<FuncRef>,
    string_starts_with: Option<FuncRef>,
    string_ends_with: Option<FuncRef>,
    string_trim: Option<FuncRef>,
    string_substring: Option<FuncRef>,
    string_replace: Option<FuncRef>,
    string_to_upper: Option<FuncRef>,
    string_to_lower: Option<FuncRef>,
    string_concat: Option<FuncRef>,
    char_at: Option<FuncRef>,

    // Type conversion
    int_to_string: Option<FuncRef>,
    float_to_string: Option<FuncRef>,
    bool_to_string: Option<FuncRef>,
    char_to_string: Option<FuncRef>,
    string_to_int: Option<FuncRef>,
    float_to_int: Option<FuncRef>,
    string_to_float: Option<FuncRef>,
    int_to_float: Option<FuncRef>,

    // Math
    abs_int: Option<FuncRef>,
    abs_float: Option<FuncRef>,
    min_int: Option<FuncRef>,
    max_int: Option<FuncRef>,
    min_float: Option<FuncRef>,
    max_float: Option<FuncRef>,
    sqrt: Option<FuncRef>,
    pow: Option<FuncRef>,
    sin: Option<FuncRef>,
    cos: Option<FuncRef>,
    tan: Option<FuncRef>,
    floor: Option<FuncRef>,
    ceil: Option<FuncRef>,
    round: Option<FuncRef>,

    // Array operations
    array_new: Option<FuncRef>,
    array_free: Option<FuncRef>,
    array_length: Option<FuncRef>,
    array_get_ptr: Option<FuncRef>,
    array_get_int: Option<FuncRef>,
    array_get_float: Option<FuncRef>,
    array_set_int: Option<FuncRef>,
    array_set_float: Option<FuncRef>,
    array_first_int: Option<FuncRef>,
    array_first_float: Option<FuncRef>,
    array_last_int: Option<FuncRef>,
    array_last_float: Option<FuncRef>,
    array_reverse_int: Option<FuncRef>,
    array_reverse_float: Option<FuncRef>,
    array_push_int: Option<FuncRef>,
    array_push_float: Option<FuncRef>,
    array_pop_int: Option<FuncRef>,
    array_pop_float: Option<FuncRef>,
    array_slice_int: Option<FuncRef>,
    array_slice_float: Option<FuncRef>,
    array_concat_int: Option<FuncRef>,
    array_concat_float: Option<FuncRef>,

    // Error handling
    panic: Option<FuncRef>,

    // Memory allocation
    alloc: Option<FuncRef>,
    free: Option<FuncRef>,

    // Effect system runtime functions
    effect_evidence_new: Option<FuncRef>,
    effect_evidence_push: Option<FuncRef>,
    effect_evidence_pop: Option<FuncRef>,
    effect_evidence_lookup: Option<FuncRef>,
    effect_handler_call: Option<FuncRef>,

    // Async effect runtime functions
    // These provide the bridge to aria-runtime for concurrency operations
    async_spawn: Option<FuncRef>,
    async_await: Option<FuncRef>,
    async_yield: Option<FuncRef>,
}

/// The main Cranelift compiler
pub struct CraneliftCompiler {
    /// The target ISA
    isa: std::sync::Arc<dyn TargetIsa>,
    /// The object module being built
    module: ObjectModule,
    /// Cranelift context for compiling functions
    ctx: Context,
    /// Function builder context (reusable)
    func_ctx: FunctionBuilderContext,
    /// Mapping from MIR function IDs to Cranelift function IDs
    func_ids: HashMap<FunctionId, FuncId>,
    /// Runtime function declarations
    runtime: RuntimeFunctions,
    /// String literal data
    string_data: HashMap<u32, cranelift_module::DataId>,
}

impl CraneliftCompiler {
    /// Create a new compiler for the given target
    pub fn new(target: Target) -> Result<Self> {
        // Set up Cranelift flags
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
        flag_builder.set("is_pic", "true").unwrap();

        let flags = settings::Flags::new(flag_builder);
        let _triple = target.triple();

        // Create the target ISA
        let isa = match cranelift_native::builder() {
            Ok(builder) => builder.finish(flags).map_err(|e| CodegenError::Internal {
                message: format!("failed to create ISA: {}", e),
            })?,
            Err(e) => {
                return Err(CodegenError::Internal {
                    message: format!("native ISA not available: {}", e),
                })
            }
        };

        // Create the object module
        let builder = ObjectBuilder::new(
            isa.clone(),
            "aria_module",
            cranelift_module::default_libcall_names(),
        )
        .map_err(|e| CodegenError::Internal {
            message: format!("failed to create object builder: {}", e),
        })?;

        let module = ObjectModule::new(builder);

        Ok(Self {
            isa,
            module,
            ctx: Context::new(),
            func_ctx: FunctionBuilderContext::new(),
            func_ids: HashMap::new(),
            runtime: RuntimeFunctions::new(),
            string_data: HashMap::new(),
        })
    }

    /// Compile an entire MIR program
    pub fn compile_program(&mut self, program: &MirProgram) -> Result<()> {
        // Declare runtime functions
        self.runtime.declare_all(&mut self.module)?;

        // First pass: declare all functions
        for (&fn_id, mir_func) in &program.functions {
            // Skip builtin functions - they use runtime calls instead
            if matches!(mir_func.linkage, MirLinkage::Builtin(_)) {
                continue;
            }
            // Skip generic functions - only compile their monomorphized versions
            if !mir_func.type_params.is_empty() {
                continue;
            }
            let clif_id = self.declare_function(mir_func)?;
            self.func_ids.insert(fn_id, clif_id);
        }

        // Create string data for all strings
        for (idx, string) in program.strings.iter().enumerate() {
            let data_id = self.create_string_data(idx as u32, string)?;
            self.string_data.insert(idx as u32, data_id);
        }

        // Second pass: compile all non-builtin functions
        for (&fn_id, mir_func) in &program.functions {
            // Skip builtin functions
            if matches!(mir_func.linkage, MirLinkage::Builtin(_)) {
                continue;
            }
            // Skip generic functions - only compile their monomorphized versions
            if !mir_func.type_params.is_empty() {
                continue;
            }
            self.compile_function(fn_id, mir_func, program)?;
        }

        Ok(())
    }

    /// Declare a function in the module
    fn declare_function(&mut self, mir_func: &MirFunction) -> Result<FuncId> {
        let sig = self.create_signature(mir_func);

        let linkage = if mir_func.name == "main" {
            Linkage::Export
        } else if mir_func.is_public {
            Linkage::Export
        } else {
            Linkage::Local
        };

        // Use "aria_main" for the main function to avoid conflict with C main
        let name = if mir_func.name == "main" {
            "aria_main"
        } else {
            mir_func.name.as_str()
        };

        let func_id = self.module.declare_function(name, linkage, &sig)?;
        Ok(func_id)
    }

    /// Create a Cranelift signature from a MIR function
    fn create_signature(&self, mir_func: &MirFunction) -> Signature {
        let call_conv = self.module.target_config().default_call_conv;
        let mut sig = Signature::new(call_conv);

        // Add parameters
        for &param_local in &mir_func.params {
            let ty = &mir_func.locals[param_local.0 as usize].ty;
            let clif_ty = mir_type_to_clif(ty, self.isa.as_ref());
            sig.params.push(AbiParam::new(clif_ty));
        }

        // Add evidence vector parameter for effectful functions
        // The evidence vector is a pointer to the current handler chain
        if !mir_func.effect_row.is_pure() {
            let ptr_type = self.isa.pointer_type();
            sig.params.push(AbiParam::new(ptr_type));
        }

        // Add return type
        if mir_func.return_ty != MirType::Unit {
            let ret_ty = mir_type_to_clif(&mir_func.return_ty, self.isa.as_ref());
            sig.returns.push(AbiParam::new(ret_ty));
        }

        sig
    }

    /// Check if a function has effects (non-pure effect row)
    #[allow(dead_code)]
    fn function_has_effects(&self, mir_func: &MirFunction) -> bool {
        !mir_func.effect_row.is_pure()
    }

    /// Create string data in the module
    fn create_string_data(&mut self, idx: u32, string: &str) -> Result<cranelift_module::DataId> {
        let name = format!("__str_{}", idx);

        // Create null-terminated string
        let mut bytes = string.as_bytes().to_vec();
        bytes.push(0); // Null terminator

        let data_id = self
            .module
            .declare_data(&name, Linkage::Local, false, false)?;

        let mut desc = DataDescription::new();
        desc.define(bytes.into_boxed_slice());

        self.module.define_data(data_id, &desc)?;

        Ok(data_id)
    }

    /// Compile a single MIR function
    fn compile_function(
        &mut self,
        fn_id: FunctionId,
        mir_func: &MirFunction,
        program: &MirProgram,
    ) -> Result<()> {
        let clif_id = self.func_ids[&fn_id];
        let sig = self.create_signature(mir_func);
        let ptr_type = self.isa.pointer_type();

        self.ctx.func = Function::with_name_signature(
            cranelift_codegen::ir::UserFuncName::user(0, fn_id.0),
            sig,
        );

        // Pre-declare all function references and data references in the function
        // This avoids borrow conflicts later
        let mut declared_funcs: FxHashMap<FunctionId, FuncRef> = FxHashMap::default();
        let mut declared_data: FxHashMap<u32, GlobalValue> = FxHashMap::default();

        // Scan MIR for all function and data references
        for block in &mir_func.blocks {
            for stmt in &block.statements {
                if let StatementKind::Assign(_, rvalue) = &stmt.kind {
                    self.collect_refs_from_rvalue(rvalue, &mut declared_funcs, &mut declared_data);
                }
            }
            if let Some(term) = &block.terminator {
                self.collect_refs_from_terminator(&term.kind, &mut declared_funcs, &mut declared_data);
            }
        }

        // Now declare all collected references
        for (&mir_fn_id, func_ref) in &mut declared_funcs {
            if let Some(&clif_fn_id) = self.func_ids.get(&mir_fn_id) {
                *func_ref = self.module.declare_func_in_func(clif_fn_id, &mut self.ctx.func);
            }
        }
        for (&string_idx, global_val) in &mut declared_data {
            if let Some(&data_id) = self.string_data.get(&string_idx) {
                *global_val = self.module.declare_data_in_func(data_id, &mut self.ctx.func);
            }
        }

        // Declare runtime functions in this function
        let mut runtime_funcs: RuntimeFuncRefs = RuntimeFuncRefs::default();

        // Helper macro to reduce boilerplate
        macro_rules! declare_rt {
            ($field:ident) => {
                if let Some(func_id) = self.runtime.$field {
                    runtime_funcs.$field = Some(self.module.declare_func_in_func(func_id, &mut self.ctx.func));
                }
            };
        }

        // I/O functions
        declare_rt!(print_int);
        declare_rt!(print_float);
        declare_rt!(print_bool);
        declare_rt!(print_string);
        declare_rt!(print_newline);

        // Memory management
        declare_rt!(alloc);

        // String operations
        declare_rt!(string_concat);
        declare_rt!(string_len);
        declare_rt!(string_contains);
        declare_rt!(string_starts_with);
        declare_rt!(string_ends_with);
        declare_rt!(string_trim);
        declare_rt!(string_substring);
        declare_rt!(string_replace);
        declare_rt!(string_to_upper);
        declare_rt!(string_to_lower);
        declare_rt!(char_at);

        // Type conversion functions
        declare_rt!(int_to_string);
        declare_rt!(float_to_string);
        declare_rt!(bool_to_string);
        declare_rt!(char_to_string);
        declare_rt!(string_to_int);
        declare_rt!(float_to_int);
        declare_rt!(string_to_float);
        declare_rt!(int_to_float);

        // Math functions
        declare_rt!(abs_int);
        declare_rt!(abs_float);
        declare_rt!(min_int);
        declare_rt!(max_int);
        declare_rt!(min_float);
        declare_rt!(max_float);
        declare_rt!(sqrt);
        declare_rt!(pow);
        declare_rt!(sin);
        declare_rt!(cos);
        declare_rt!(tan);
        declare_rt!(floor);
        declare_rt!(ceil);
        declare_rt!(round);

        // Array operations
        declare_rt!(array_new);
        declare_rt!(array_free);
        declare_rt!(array_length);
        declare_rt!(array_get_ptr);
        declare_rt!(array_get_int);
        declare_rt!(array_get_float);
        declare_rt!(array_set_int);
        declare_rt!(array_set_float);
        declare_rt!(array_first_int);
        declare_rt!(array_first_float);
        declare_rt!(array_last_int);
        declare_rt!(array_last_float);
        declare_rt!(array_reverse_int);
        declare_rt!(array_reverse_float);
        declare_rt!(array_push_int);
        declare_rt!(array_push_float);
        declare_rt!(array_pop_int);
        declare_rt!(array_pop_float);
        declare_rt!(array_slice_int);
        declare_rt!(array_slice_float);
        declare_rt!(array_concat_int);
        declare_rt!(array_concat_float);

        // Error handling
        declare_rt!(panic);

        // Build the function
        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.func_ctx);
            let func_compiler = FunctionCompiler::new(
                &self.isa,
                &self.func_ids,
                &declared_funcs,
                &declared_data,
                &runtime_funcs,
                &mut builder,
                mir_func,
                program,
                ptr_type,
            );
            func_compiler.compile()?;
            builder.finalize();
        }

        // Compile and define
        self.module
            .define_function(clif_id, &mut self.ctx)
            .map_err(|e| CodegenError::Module(e.to_string()))?;

        self.ctx.clear();
        Ok(())
    }

    /// Collect function and data references from an rvalue
    fn collect_refs_from_rvalue(
        &self,
        rvalue: &Rvalue,
        funcs: &mut FxHashMap<FunctionId, FuncRef>,
        data: &mut FxHashMap<u32, GlobalValue>,
    ) {
        match rvalue {
            Rvalue::Use(op) => self.collect_refs_from_operand(op, funcs, data),
            Rvalue::BinaryOp(_, l, r) => {
                self.collect_refs_from_operand(l, funcs, data);
                self.collect_refs_from_operand(r, funcs, data);
            }
            Rvalue::UnaryOp(_, op) => self.collect_refs_from_operand(op, funcs, data),
            Rvalue::Aggregate(_, ops) => {
                for op in ops {
                    self.collect_refs_from_operand(op, funcs, data);
                }
            }
            Rvalue::Closure(fn_id, _) => {
                funcs.entry(*fn_id).or_insert(FuncRef::from_u32(0));
            }
            Rvalue::Cast(_, op, _) => self.collect_refs_from_operand(op, funcs, data),
            _ => {}
        }
    }

    /// Collect function and data references from a terminator
    fn collect_refs_from_terminator(
        &self,
        term: &TerminatorKind,
        funcs: &mut FxHashMap<FunctionId, FuncRef>,
        data: &mut FxHashMap<u32, GlobalValue>,
    ) {
        match term {
            TerminatorKind::Call { func, args, .. } => {
                self.collect_refs_from_operand(func, funcs, data);
                for arg in args {
                    self.collect_refs_from_operand(arg, funcs, data);
                }
            }
            TerminatorKind::SwitchInt { discr, .. } => {
                self.collect_refs_from_operand(discr, funcs, data);
            }
            TerminatorKind::Assert { cond, .. } => {
                self.collect_refs_from_operand(cond, funcs, data);
            }
            _ => {}
        }
    }

    /// Collect function and data references from an operand
    fn collect_refs_from_operand(
        &self,
        operand: &Operand,
        funcs: &mut FxHashMap<FunctionId, FuncRef>,
        data: &mut FxHashMap<u32, GlobalValue>,
    ) {
        if let Operand::Constant(c) = operand {
            match c {
                Constant::Function(fn_id) => {
                    funcs.entry(*fn_id).or_insert(FuncRef::from_u32(0));
                }
                Constant::String(idx) => {
                    data.entry(*idx).or_insert(GlobalValue::from_u32(0));
                }
                _ => {}
            }
        }
    }

    /// Finish compilation and return the object file bytes
    pub fn finish(self) -> Result<Vec<u8>> {
        let product = self.module.finish();
        Ok(product.emit().map_err(|e| CodegenError::Internal {
            message: format!("failed to emit object: {}", e),
        })?)
    }
}

/// Compiler for a single function
#[allow(dead_code)]
struct FunctionCompiler<'a, 'b> {
    isa: &'a std::sync::Arc<dyn TargetIsa>,
    func_ids: &'a HashMap<FunctionId, FuncId>,
    declared_funcs: &'a FxHashMap<FunctionId, FuncRef>,
    declared_data: &'a FxHashMap<u32, GlobalValue>,
    runtime_funcs: &'a RuntimeFuncRefs,
    builder: &'a mut FunctionBuilder<'b>,
    mir_func: &'a MirFunction,
    program: &'a MirProgram,
    ptr_type: ClifType,

    /// Mapping from MIR locals to Cranelift variables
    locals: FxHashMap<Local, Variable>,
    /// Mapping from MIR blocks to Cranelift blocks
    blocks: FxHashMap<BlockId, Block>,
    /// Next variable index
    next_var: usize,

    /// Evidence vector variable (for effectful functions)
    evidence_var: Option<Variable>,
    /// Whether this function has effects
    has_effects: bool,
}

impl<'a, 'b> FunctionCompiler<'a, 'b> {
    fn new(
        isa: &'a std::sync::Arc<dyn TargetIsa>,
        func_ids: &'a HashMap<FunctionId, FuncId>,
        declared_funcs: &'a FxHashMap<FunctionId, FuncRef>,
        declared_data: &'a FxHashMap<u32, GlobalValue>,
        runtime_funcs: &'a RuntimeFuncRefs,
        builder: &'a mut FunctionBuilder<'b>,
        mir_func: &'a MirFunction,
        program: &'a MirProgram,
        ptr_type: ClifType,
    ) -> Self {
        let has_effects = !mir_func.effect_row.is_pure();
        Self {
            isa,
            func_ids,
            declared_funcs,
            declared_data,
            runtime_funcs,
            builder,
            mir_func,
            program,
            ptr_type,
            locals: FxHashMap::default(),
            blocks: FxHashMap::default(),
            next_var: 0,
            evidence_var: None,
            has_effects,
        }
    }

    fn compile(mut self) -> Result<()> {
        // Create blocks for all MIR blocks
        for block in &self.mir_func.blocks {
            let clif_block = self.builder.create_block();
            self.blocks.insert(block.id, clif_block);
        }

        // Declare variables for all locals
        for (i, local_decl) in self.mir_func.locals.iter().enumerate() {
            let local = Local(i as u32);
            let ty = mir_type_to_clif(&local_decl.ty, self.isa.as_ref());
            let var = Variable::from_u32(self.next_var as u32);
            self.next_var += 1;
            self.builder.declare_var(var, ty);
            self.locals.insert(local, var);
        }

        // Set entry block and add parameters
        let entry_block = self.blocks[&BlockId::ENTRY];
        self.builder.append_block_params_for_function_params(entry_block);
        self.builder.switch_to_block(entry_block);
        self.builder.seal_block(entry_block);

        // Initialize parameter variables
        for (i, &param_local) in self.mir_func.params.iter().enumerate() {
            let param_value = self.builder.block_params(entry_block)[i];
            let var = self.locals[&param_local];
            self.builder.def_var(var, param_value);
        }

        // Initialize evidence variable for effectful functions
        if self.has_effects {
            let param_count = self.mir_func.params.len();
            let evidence_value = self.builder.block_params(entry_block)[param_count];
            let ev_var = Variable::from_u32(self.next_var as u32);
            self.next_var += 1;
            self.builder.declare_var(ev_var, self.ptr_type);
            self.builder.def_var(ev_var, evidence_value);
            self.evidence_var = Some(ev_var);
        }

        // Initialize return variable to default
        if self.mir_func.return_ty != MirType::Unit {
            let ret_ty = mir_type_to_clif(&self.mir_func.return_ty, self.isa.as_ref());
            let zero = if ret_ty.is_float() {
                // Use f64const or f32const for float types
                if ret_ty == types::F32 {
                    self.builder.ins().f32const(0.0)
                } else {
                    self.builder.ins().f64const(0.0)
                }
            } else {
                self.builder.ins().iconst(ret_ty, 0)
            };
            let ret_var = self.locals[&Local::RETURN];
            self.builder.def_var(ret_var, zero);
        }

        // Compile all blocks
        for mir_block in &self.mir_func.blocks {
            self.compile_block(mir_block)?;
        }

        // Seal all blocks after compilation so SSA construction has all predecessors
        for mir_block in &self.mir_func.blocks {
            if mir_block.id != BlockId::ENTRY {
                let clif_block = self.blocks[&mir_block.id];
                self.builder.seal_block(clif_block);
            }
        }

        Ok(())
    }

    fn compile_block(&mut self, mir_block: &BasicBlock) -> Result<()> {
        let clif_block = self.blocks[&mir_block.id];

        // Skip entry block as we've already switched to it
        if mir_block.id != BlockId::ENTRY {
            self.builder.switch_to_block(clif_block);
        }

        // Compile statements
        for (stmt_idx, stmt) in mir_block.statements.iter().enumerate() {
            self.compile_statement(stmt)?;

            // Check for effect statement at this position
            if let Some(effect_stmt) = self.mir_func.effect_statement(mir_block.id, stmt_idx) {
                self.compile_effect_statement(effect_stmt)?;
            }
        }

        // Compile terminator
        if let Some(term) = &mir_block.terminator {
            self.compile_terminator(term)?;
        } else {
            // No terminator - add unreachable
            self.builder.ins().trap(cranelift_codegen::ir::TrapCode::user(0).unwrap());
        }

        // Note: Blocks are sealed after all blocks are compiled, not here

        Ok(())
    }

    fn compile_statement(&mut self, stmt: &aria_mir::Statement) -> Result<()> {
        match &stmt.kind {
            StatementKind::Assign(place, rvalue) => {
                let value = self.compile_rvalue(rvalue)?;
                self.store_to_place(place, value)?;
            }
            StatementKind::StorageLive(_local) => {
                // No-op for now - could track for debug info
            }
            StatementKind::StorageDead(_local) => {
                // No-op for now
            }
            StatementKind::Nop => {
                // No-op
            }
        }
        Ok(())
    }

    /// Compile an effect statement
    fn compile_effect_statement(&mut self, effect_stmt: &EffectStatementKind) -> Result<()> {
        match effect_stmt {
            EffectStatementKind::InstallHandler {
                handler,
                evidence_slot,
                effect,
                ..
            } => {
                self.compile_install_handler(*handler, evidence_slot, effect)?;
            }

            EffectStatementKind::UninstallHandler {
                evidence_slot,
                prev_evidence,
            } => {
                self.compile_uninstall_handler(evidence_slot, *prev_evidence)?;
            }

            EffectStatementKind::PerformEffect {
                effect,
                operation,
                args,
                evidence_slot,
                dest,
                classification,
            } => {
                self.compile_perform_effect(
                    effect,
                    *operation,
                    args,
                    evidence_slot,
                    dest,
                    classification,
                )?;
            }

            EffectStatementKind::CaptureContunuation { dest } => {
                // Continuation capture requires fiber runtime
                // For now, store a null pointer as placeholder
                let null = self.builder.ins().iconst(self.ptr_type, 0);
                self.store_to_place(dest, null)?;
            }

            EffectStatementKind::CloneContinuation { source, dest } => {
                // Clone continuation - copy the pointer for now
                let cont = self.compile_operand(source)?;
                self.store_to_place(dest, cont)?;
            }

            EffectStatementKind::FfiBarrier {
                strategy: _,
                blocked_effects: _,
            } => {
                // FFI barrier is a compile-time annotation
                // No runtime code needed for basic implementation
            }
        }
        Ok(())
    }

    /// Compile handler installation
    fn compile_install_handler(
        &mut self,
        _handler: HandlerId,
        _evidence_slot: &EvidenceSlot,
        _effect: &EffectType,
    ) -> Result<()> {
        // For tail-resumptive effects, handler installation is a no-op
        // at runtime since we use evidence-passing style.
        // The handler reference is passed via the evidence parameter.
        //
        // For general effects with continuation capture, this would:
        // 1. Push handler onto the handler chain
        // 2. Update evidence vector slot
        //
        // TODO: Implement general handler installation when fiber runtime is ready
        Ok(())
    }

    /// Compile handler uninstallation
    fn compile_uninstall_handler(
        &mut self,
        _evidence_slot: &EvidenceSlot,
        _prev_evidence: Local,
    ) -> Result<()> {
        // Symmetric to install - for tail-resumptive, this is a no-op
        // For general effects, would restore previous evidence from prev_evidence local
        Ok(())
    }

    /// Compile effect perform operation
    fn compile_perform_effect(
        &mut self,
        effect: &EffectType,
        operation: aria_mir::OperationId,
        args: &[Operand],
        evidence_slot: &EvidenceSlot,
        dest: &Place,
        classification: &EffectClassification,
    ) -> Result<()> {
        // Check for built-in effects with special handling
        match effect.name.as_str() {
            "Async" => return self.compile_async_effect(operation, args, dest),
            "Console" => return self.compile_console_effect(operation, args, dest),
            "IO" => return self.compile_io_effect(operation, args, dest),
            _ => {}
        }

        match classification {
            EffectClassification::TailResumptive => {
                // Tail-resumptive optimization: direct handler call
                // This compiles to a simple indirect function call through the vtable
                self.compile_tail_resumptive_perform(operation, args, evidence_slot, dest)?;
            }

            EffectClassification::General => {
                // General effect requires continuation capture
                // For now, fall back to tail-resumptive style
                // TODO: Implement proper CPS transformation when fiber runtime is ready
                self.compile_tail_resumptive_perform(operation, args, evidence_slot, dest)?;
            }

            EffectClassification::FfiBoundary => {
                // FFI boundary effects need special handling
                // For now, treat as tail-resumptive
                self.compile_tail_resumptive_perform(operation, args, evidence_slot, dest)?;
            }
        }
        Ok(())
    }

    /// Compile Async effect operations (spawn, await, yield)
    ///
    /// The Async effect is special-cased because it bridges to the aria-runtime
    /// concurrency primitives rather than using the general effect handler mechanism.
    ///
    /// Operations:
    /// - Operation 0 (await): Wait for a task to complete via aria_async_await
    /// - Operation 1 (spawn): Create a new concurrent task via aria_async_spawn
    /// - Operation 2 (yield): Yield control to the scheduler via aria_async_yield
    ///
    /// # Implementation Status
    ///
    /// The FFI bridge functions are implemented in aria-runtime/src/ffi.rs:
    /// - `aria_async_spawn(func, captures)` -> task_id
    /// - `aria_async_await(task_id)` -> result
    /// - `aria_async_yield()` -> void
    ///
    /// To fully connect these:
    /// 1. Build aria-runtime as a staticlib or link dynamically
    /// 2. Declare the FFI functions in the Cranelift module
    /// 3. Generate calls to them here
    ///
    /// Current implementation uses placeholders until the linking infrastructure
    /// is complete.
    fn compile_async_effect(
        &mut self,
        operation: aria_mir::OperationId,
        args: &[Operand],
        dest: &Place,
    ) -> Result<()> {
        match operation.0 {
            // await operation: Wait for task completion
            // Full impl: result = aria_async_await(task_id from args[0])
            0 => {
                // When linked to runtime, this would call aria_async_await
                // For now, if there's a task_id argument, pretend we awaited it
                if !args.is_empty() {
                    // In full implementation:
                    // let task_id = self.compile_operand(&args[0])?;
                    // let call = self.builder.ins().call(async_await_func, &[task_id]);
                    // let result = self.builder.inst_results(call)[0];
                    // self.store_to_place(dest, result)?;

                    // Placeholder: just return the task_id as the "result"
                    let result = self.compile_operand(&args[0])?;
                    self.store_to_place(dest, result)?;
                } else {
                    let zero = self.builder.ins().iconst(types::I64, 0);
                    self.store_to_place(dest, zero)?;
                }
            }

            // spawn operation: Create new task
            // Full impl: task_id = aria_async_spawn(func_ptr, captures_ptr)
            1 => {
                // When linked to runtime, this would call aria_async_spawn
                // The closure/function to spawn would be in args[0]
                //
                // Full implementation would:
                // 1. Get function pointer from closure operand
                // 2. Get/allocate captures struct
                // 3. Call: task_id = aria_async_spawn(func_ptr, captures_ptr)
                // 4. Store task_id to dest
                //
                // Placeholder: return incrementing task ID
                let task_id = self.builder.ins().iconst(types::I64, 1);
                self.store_to_place(dest, task_id)?;
            }

            // yield operation: Cooperative yield
            // Full impl: aria_async_yield() (void return)
            2 => {
                // When linked to runtime, this would call aria_async_yield
                // which internally calls std::thread::yield_now()
                //
                // Placeholder: no-op, return unit (0)
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.store_to_place(dest, zero)?;
            }

            // scope operation: Create structured concurrency scope
            3 => {
                // Structured concurrency scope would integrate with Scope
                // Placeholder for now
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.store_to_place(dest, zero)?;
            }

            // supervisor operation: Create supervised scope
            4 => {
                // Supervised scope for error isolation
                // Placeholder for now
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.store_to_place(dest, zero)?;
            }

            // timeout operation: Create timeout scope
            5 => {
                // Timeout scope with cancellation
                // Placeholder for now
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.store_to_place(dest, zero)?;
            }

            _ => {
                // Unknown Async operation
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("Async operation {}", operation.0),
                    span: None,
                });
            }
        }
        Ok(())
    }

    /// Compile Console effect operations (print, read_line)
    ///
    /// The Console effect provides standard I/O operations and bridges
    /// directly to the C runtime functions.
    ///
    /// Operations:
    /// - Operation 0 (print): Print a value to stdout
    /// - Operation 1 (read_line): Read a line from stdin
    fn compile_console_effect(
        &mut self,
        operation: aria_mir::OperationId,
        args: &[Operand],
        dest: &Place,
    ) -> Result<()> {
        match operation.0 {
            // print operation: Output to stdout
            0 => {
                if !args.is_empty() {
                    let arg_val = self.compile_operand(&args[0])?;
                    let arg_ty = self.get_operand_type(&args[0]);

                    match arg_ty {
                        MirType::Int | MirType::Int64 | MirType::Int32 | MirType::Int16 | MirType::Int8
                        | MirType::UInt | MirType::UInt64 | MirType::UInt32 | MirType::UInt16 | MirType::UInt8 => {
                            if let Some(f) = self.runtime_funcs.print_int {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                        MirType::Float | MirType::Float64 => {
                            if let Some(f) = self.runtime_funcs.print_float {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                        MirType::Float32 => {
                            if let Some(f) = self.runtime_funcs.print_float {
                                let arg_f64 = self.builder.ins().fpromote(types::F64, arg_val);
                                self.builder.ins().call(f, &[arg_f64]);
                            }
                        }
                        MirType::Bool => {
                            if let Some(f) = self.runtime_funcs.print_bool {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                        MirType::String => {
                            if let Some(f) = self.runtime_funcs.print_string {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                        MirType::Char => {
                            // Convert char to string then print
                            if let Some(to_str) = self.runtime_funcs.char_to_string {
                                let call = self.builder.ins().call(to_str, &[arg_val]);
                                let str_val = self.builder.inst_results(call)[0];
                                if let Some(f) = self.runtime_funcs.print_string {
                                    self.builder.ins().call(f, &[str_val]);
                                }
                            }
                        }
                        _ => {
                            // Default: print as int
                            if let Some(f) = self.runtime_funcs.print_int {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                    }
                }
                // Print newline after value
                if let Some(f) = self.runtime_funcs.print_newline {
                    self.builder.ins().call(f, &[]);
                }
                // Return unit
                let unit = self.builder.ins().iconst(types::I64, 0);
                self.store_to_place(dest, unit)?;
            }

            // read_line operation: Read line from stdin
            1 => {
                // TODO: Implement read_line when stdin reading is added to C runtime
                // For now, return empty string (null pointer)
                let empty = self.builder.ins().iconst(types::I64, 0);
                self.store_to_place(dest, empty)?;
            }

            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("Console operation {}", operation.0),
                    span: None,
                });
            }
        }
        Ok(())
    }

    /// Compile IO effect operations (read, write)
    ///
    /// The IO effect provides file I/O operations.
    ///
    /// Operations:
    /// - Operation 0 (read): Read from a file handle
    /// - Operation 1 (write): Write to a file handle
    fn compile_io_effect(
        &mut self,
        operation: aria_mir::OperationId,
        args: &[Operand],
        dest: &Place,
    ) -> Result<()> {
        match operation.0 {
            // read operation
            0 => {
                // TODO: Implement file read when file I/O is added to C runtime
                // For now, return 0 (no bytes read)
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.store_to_place(dest, zero)?;
            }

            // write operation
            1 => {
                // For write(handle, data), if handle is stdout (0), use print
                if args.len() >= 2 {
                    let _handle = self.compile_operand(&args[0])?;
                    let data = self.compile_operand(&args[1])?;
                    let data_ty = self.get_operand_type(&args[1]);

                    // Check if writing to stdout (handle == 0 or 1)
                    // For now, always treat as stdout write
                    match data_ty {
                        MirType::String => {
                            if let Some(f) = self.runtime_funcs.print_string {
                                self.builder.ins().call(f, &[data]);
                            }
                        }
                        _ => {
                            if let Some(f) = self.runtime_funcs.print_int {
                                self.builder.ins().call(f, &[data]);
                            }
                        }
                    }
                }
                // Return bytes written (placeholder)
                let written = self.builder.ins().iconst(types::I64, 1);
                self.store_to_place(dest, written)?;
            }

            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("IO operation {}", operation.0),
                    span: None,
                });
            }
        }
        Ok(())
    }

    /// Compile a tail-resumptive effect perform as a direct call
    ///
    /// Generated code pattern:
    /// ```text
    /// ; Load handler from evidence slot
    /// handler_ptr = load evidence[slot_offset]
    /// ; Load vtable from handler
    /// vtable = load handler_ptr[0]
    /// ; Load operation function from vtable
    /// op_fn = load vtable[operation_index * 8]
    /// ; Call the operation
    /// result = call_indirect op_fn(handler_ptr, args...)
    /// ```
    fn compile_tail_resumptive_perform(
        &mut self,
        operation: aria_mir::OperationId,
        args: &[Operand],
        evidence_slot: &EvidenceSlot,
        dest: &Place,
    ) -> Result<()> {
        // Get evidence vector pointer
        let evidence_ptr = match &self.evidence_var {
            Some(var) => self.builder.use_var(*var),
            None => {
                // No evidence - this shouldn't happen for effectful code
                // Return a zero value as fallback
                let zero = self.builder.ins().iconst(types::I64, 0);
                self.store_to_place(dest, zero)?;
                return Ok(());
            }
        };

        // Calculate slot offset
        let slot_offset = match evidence_slot {
            EvidenceSlot::Static(offset) => *offset as i32 * 8, // Each slot is 8 bytes (pointer)
            EvidenceSlot::Dynamic(_) => {
                // Dynamic lookup would need runtime call
                // For now, use offset 0
                0
            }
        };

        // Load handler pointer from evidence vector
        let handler_ptr = self.builder.ins().load(
            self.ptr_type,
            MemFlags::trusted(),
            evidence_ptr,
            slot_offset,
        );

        // Load vtable pointer from handler (first field)
        let vtable_ptr = self.builder.ins().load(
            self.ptr_type,
            MemFlags::trusted(),
            handler_ptr,
            0,
        );

        // Load operation function pointer from vtable
        let op_offset = (operation.0 as i32) * 8;
        let op_fn_ptr = self.builder.ins().load(
            self.ptr_type,
            MemFlags::trusted(),
            vtable_ptr,
            op_offset,
        );

        // Compile arguments
        let mut call_args = vec![handler_ptr]; // Handler is first argument (self)
        for arg in args {
            call_args.push(self.compile_operand(arg)?);
        }

        // Create signature for indirect call
        let call_conv = self.builder.func.signature.call_conv;
        let mut sig = Signature::new(call_conv);

        // Handler pointer parameter
        sig.params.push(AbiParam::new(self.ptr_type));

        // Add argument types
        for arg in args {
            let ty = self.get_operand_type(arg);
            let clif_ty = mir_type_to_clif(&ty, self.isa.as_ref());
            sig.params.push(AbiParam::new(clif_ty));
        }

        // Return type (assume i64 for now)
        sig.returns.push(AbiParam::new(types::I64));

        let sig_ref = self.builder.import_signature(sig);

        // Make indirect call
        let call = self.builder.ins().call_indirect(sig_ref, op_fn_ptr, &call_args);
        let results = self.builder.inst_results(call);

        // Store result
        if !results.is_empty() {
            self.store_to_place(dest, results[0])?;
        }

        Ok(())
    }

    fn compile_terminator(&mut self, term: &aria_mir::Terminator) -> Result<()> {
        match &term.kind {
            TerminatorKind::Goto { target } => {
                let target_block = self.blocks[target];
                self.builder.ins().jump(target_block, &[]);
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                self.compile_switch(discr, targets)?;
            }
            TerminatorKind::Call {
                func,
                args,
                dest,
                target,
            } => {
                self.compile_call(func, args, dest, *target)?;
            }
            TerminatorKind::Return => {
                if self.mir_func.return_ty != MirType::Unit {
                    let ret_var = self.locals[&Local::RETURN];
                    let ret_val = self.builder.use_var(ret_var);
                    self.builder.ins().return_(&[ret_val]);
                } else {
                    self.builder.ins().return_(&[]);
                }
            }
            TerminatorKind::Unreachable => {
                self.builder.ins().trap(cranelift_codegen::ir::TrapCode::user(0).unwrap());
            }
            TerminatorKind::Drop { place: _, target } => {
                // For now, just jump to target (no actual drop)
                let target_block = self.blocks[target];
                self.builder.ins().jump(target_block, &[]);
            }
            TerminatorKind::Assert {
                cond,
                expected,
                msg: _,
                target,
            } => {
                let cond_val = self.compile_operand(cond)?;
                let trap_block = self.builder.create_block();
                let target_block = self.blocks[target];

                if *expected {
                    // Assert true: branch to target if true, trap if false
                    self.builder.ins().brif(cond_val, target_block, &[], trap_block, &[]);
                } else {
                    // Assert false: branch to target if false, trap if true
                    self.builder.ins().brif(cond_val, trap_block, &[], target_block, &[]);
                }

                self.builder.switch_to_block(trap_block);
                self.builder.seal_block(trap_block);
                self.builder.ins().trap(cranelift_codegen::ir::TrapCode::user(1).unwrap());
            }
        }
        Ok(())
    }

    fn compile_switch(&mut self, discr: &Operand, targets: &SwitchTargets) -> Result<()> {
        let discr_val = self.compile_operand(discr)?;
        let otherwise_block = self.blocks[&targets.otherwise];

        if targets.targets.len() == 1 {
            // Simple if/else
            let (val, target) = &targets.targets[0];
            let target_block = self.blocks[target];

            let const_val = self.builder.ins().iconst(types::I64, *val as i64);
            let cmp = self.builder.ins().icmp(IntCC::Equal, discr_val, const_val);

            self.builder.ins().brif(cmp, target_block, &[], otherwise_block, &[]);
        } else {
            // Multi-way switch - use jump table or linear search
            // For simplicity, use linear search for now
            let mut _current_block = self.builder.current_block().unwrap();

            for (val, target) in &targets.targets {
                let target_block = self.blocks[target];
                let next_block = self.builder.create_block();

                let const_val = self.builder.ins().iconst(types::I64, *val as i64);
                let cmp = self.builder.ins().icmp(IntCC::Equal, discr_val, const_val);

                self.builder.ins().brif(cmp, target_block, &[], next_block, &[]);

                self.builder.switch_to_block(next_block);
                self.builder.seal_block(next_block);
                _current_block = next_block;
            }

            // Fall through to otherwise
            self.builder.ins().jump(otherwise_block, &[]);
        }

        Ok(())
    }

    fn compile_call(
        &mut self,
        func: &Operand,
        args: &[Operand],
        dest: &Place,
        target: Option<BlockId>,
    ) -> Result<()> {
        // Check if this is a builtin function call
        if let Operand::Constant(Constant::Function(fn_id)) = func {
            if let Some(mir_func) = self.program.functions.get(fn_id) {
                if let MirLinkage::Builtin(builtin_kind) = mir_func.linkage {
                    return self.compile_builtin_call(builtin_kind, args, dest, target);
                }
            }
        }

        // Compile arguments
        let arg_vals: Vec<Value> = args
            .iter()
            .map(|arg| self.compile_operand(arg))
            .collect::<Result<_>>()?;

        // Get the function reference
        let func_ref = match func {
            Operand::Constant(Constant::Function(fn_id)) => {
                *self.declared_funcs.get(fn_id).ok_or_else(|| {
                    CodegenError::UndefinedFunction {
                        name: format!("fn#{}", fn_id.0),
                    }
                })?
            }
            _ => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: "indirect function calls".to_string(),
                    span: None,
                })
            }
        };

        // Make the call
        let call = self.builder.ins().call(func_ref, &arg_vals);
        let results = self.builder.inst_results(call);

        // Store result if any
        if !results.is_empty() {
            self.store_to_place(dest, results[0])?;
        }

        // Jump to target block
        if let Some(target) = target {
            let target_block = self.blocks[&target];
            self.builder.ins().jump(target_block, &[]);
        }

        Ok(())
    }

    fn compile_builtin_call(
        &mut self,
        builtin_kind: BuiltinKind,
        args: &[Operand],
        dest: &Place,
        target: Option<BlockId>,
    ) -> Result<()> {
        match builtin_kind {
            // === I/O Builtins ===
            BuiltinKind::Print | BuiltinKind::Println => {
                if !args.is_empty() {
                    let arg = &args[0];
                    let arg_val = self.compile_operand(arg)?;
                    let arg_ty = self.get_operand_type(arg);

                    match arg_ty {
                        MirType::Int => {
                            if let Some(f) = self.runtime_funcs.print_int {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                        MirType::Float | MirType::Float64 => {
                            if let Some(f) = self.runtime_funcs.print_float {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                        MirType::Float32 => {
                            // Convert f32 to f64 for printing
                            if let Some(f) = self.runtime_funcs.print_float {
                                let arg_f64 = self.builder.ins().fpromote(types::F64, arg_val);
                                self.builder.ins().call(f, &[arg_f64]);
                            }
                        }
                        MirType::Bool => {
                            if let Some(f) = self.runtime_funcs.print_bool {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                        MirType::String => {
                            if let Some(f) = self.runtime_funcs.print_string {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                        _ => {
                            if let Some(f) = self.runtime_funcs.print_int {
                                self.builder.ins().call(f, &[arg_val]);
                            }
                        }
                    }
                }

                if builtin_kind == BuiltinKind::Println {
                    if let Some(f) = self.runtime_funcs.print_newline {
                        self.builder.ins().call(f, &[]);
                    }
                }
            }

            // === String/Array Builtins ===
            BuiltinKind::Len => {
                if !args.is_empty() {
                    let arg_val = self.compile_operand(&args[0])?;
                    let arg_ty = self.get_operand_type(&args[0]);

                    let result = match arg_ty {
                        MirType::Array(_) => {
                            // Array length
                            if let Some(f) = self.runtime_funcs.array_length {
                                let call = self.builder.ins().call(f, &[arg_val]);
                                self.builder.inst_results(call)[0]
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_length not declared".into(),
                                });
                            }
                        }
                        _ => {
                            // String length (default)
                            if let Some(f) = self.runtime_funcs.string_len {
                                let call = self.builder.ins().call(f, &[arg_val]);
                                self.builder.inst_results(call)[0]
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "string_len not declared".into(),
                                });
                            }
                        }
                    };

                    self.store_to_place(dest, result)?;
                }
            }

            BuiltinKind::StringContains => {
                if args.len() >= 2 {
                    let haystack = self.compile_operand(&args[0])?;
                    let needle = self.compile_operand(&args[1])?;
                    if let Some(f) = self.runtime_funcs.string_contains {
                        let call = self.builder.ins().call(f, &[haystack, needle]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::StringStartsWith => {
                if args.len() >= 2 {
                    let s = self.compile_operand(&args[0])?;
                    let prefix = self.compile_operand(&args[1])?;
                    if let Some(f) = self.runtime_funcs.string_starts_with {
                        let call = self.builder.ins().call(f, &[s, prefix]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::StringEndsWith => {
                if args.len() >= 2 {
                    let s = self.compile_operand(&args[0])?;
                    let suffix = self.compile_operand(&args[1])?;
                    if let Some(f) = self.runtime_funcs.string_ends_with {
                        let call = self.builder.ins().call(f, &[s, suffix]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::StringTrim => {
                if !args.is_empty() {
                    let s = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.string_trim {
                        let call = self.builder.ins().call(f, &[s]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::Substring => {
                if args.len() >= 3 {
                    let s = self.compile_operand(&args[0])?;
                    let start = self.compile_operand(&args[1])?;
                    let len = self.compile_operand(&args[2])?;
                    if let Some(f) = self.runtime_funcs.string_substring {
                        let call = self.builder.ins().call(f, &[s, start, len]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::StringReplace => {
                if args.len() >= 3 {
                    let s = self.compile_operand(&args[0])?;
                    let from = self.compile_operand(&args[1])?;
                    let to = self.compile_operand(&args[2])?;
                    if let Some(f) = self.runtime_funcs.string_replace {
                        let call = self.builder.ins().call(f, &[s, from, to]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::ToUpper => {
                if !args.is_empty() {
                    let s = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.string_to_upper {
                        let call = self.builder.ins().call(f, &[s]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::ToLower => {
                if !args.is_empty() {
                    let s = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.string_to_lower {
                        let call = self.builder.ins().call(f, &[s]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::CharAt => {
                if args.len() >= 2 {
                    let s = self.compile_operand(&args[0])?;
                    let idx = self.compile_operand(&args[1])?;
                    if let Some(f) = self.runtime_funcs.char_at {
                        let call = self.builder.ins().call(f, &[s, idx]);
                        let result_i32 = self.builder.inst_results(call)[0];
                        // Extend i32 to i64 for compatibility with MirType::Int
                        let result = self.builder.ins().uextend(types::I64, result_i32);
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            // === Math Builtins ===
            BuiltinKind::Abs => {
                if !args.is_empty() {
                    let arg = &args[0];
                    let arg_val = self.compile_operand(arg)?;
                    let arg_ty = self.get_operand_type(arg);

                    let result = match arg_ty {
                        MirType::Float => {
                            if let Some(f) = self.runtime_funcs.abs_float {
                                let call = self.builder.ins().call(f, &[arg_val]);
                                Some(self.builder.inst_results(call)[0])
                            } else {
                                None
                            }
                        }
                        _ => {
                            if let Some(f) = self.runtime_funcs.abs_int {
                                let call = self.builder.ins().call(f, &[arg_val]);
                                Some(self.builder.inst_results(call)[0])
                            } else {
                                None
                            }
                        }
                    };

                    if let Some(r) = result {
                        self.store_to_place(dest, r)?;
                    }
                }
            }

            BuiltinKind::Min => {
                if args.len() >= 2 {
                    let a = self.compile_operand(&args[0])?;
                    let b = self.compile_operand(&args[1])?;
                    let arg_ty = self.get_operand_type(&args[0]);

                    let result = match arg_ty {
                        MirType::Float => {
                            if let Some(f) = self.runtime_funcs.min_float {
                                let call = self.builder.ins().call(f, &[a, b]);
                                Some(self.builder.inst_results(call)[0])
                            } else {
                                None
                            }
                        }
                        _ => {
                            if let Some(f) = self.runtime_funcs.min_int {
                                let call = self.builder.ins().call(f, &[a, b]);
                                Some(self.builder.inst_results(call)[0])
                            } else {
                                None
                            }
                        }
                    };

                    if let Some(r) = result {
                        self.store_to_place(dest, r)?;
                    }
                }
            }

            BuiltinKind::Max => {
                if args.len() >= 2 {
                    let a = self.compile_operand(&args[0])?;
                    let b = self.compile_operand(&args[1])?;
                    let arg_ty = self.get_operand_type(&args[0]);

                    let result = match arg_ty {
                        MirType::Float => {
                            if let Some(f) = self.runtime_funcs.max_float {
                                let call = self.builder.ins().call(f, &[a, b]);
                                Some(self.builder.inst_results(call)[0])
                            } else {
                                None
                            }
                        }
                        _ => {
                            if let Some(f) = self.runtime_funcs.max_int {
                                let call = self.builder.ins().call(f, &[a, b]);
                                Some(self.builder.inst_results(call)[0])
                            } else {
                                None
                            }
                        }
                    };

                    if let Some(r) = result {
                        self.store_to_place(dest, r)?;
                    }
                }
            }

            BuiltinKind::Sqrt => {
                if !args.is_empty() {
                    let x = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.sqrt {
                        let call = self.builder.ins().call(f, &[x]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::Pow => {
                if args.len() >= 2 {
                    let base = self.compile_operand(&args[0])?;
                    let exp = self.compile_operand(&args[1])?;
                    if let Some(f) = self.runtime_funcs.pow {
                        let call = self.builder.ins().call(f, &[base, exp]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::Sin => {
                if !args.is_empty() {
                    let x = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.sin {
                        let call = self.builder.ins().call(f, &[x]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::Cos => {
                if !args.is_empty() {
                    let x = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.cos {
                        let call = self.builder.ins().call(f, &[x]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::Tan => {
                if !args.is_empty() {
                    let x = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.tan {
                        let call = self.builder.ins().call(f, &[x]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::Floor => {
                if !args.is_empty() {
                    let x = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.floor {
                        let call = self.builder.ins().call(f, &[x]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::Ceil => {
                if !args.is_empty() {
                    let x = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.ceil {
                        let call = self.builder.ins().call(f, &[x]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::Round => {
                if !args.is_empty() {
                    let x = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.round {
                        let call = self.builder.ins().call(f, &[x]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            // === Control Flow Builtins ===
            BuiltinKind::Panic => {
                if !args.is_empty() {
                    let msg = self.compile_operand(&args[0])?;
                    if let Some(f) = self.runtime_funcs.panic {
                        self.builder.ins().call(f, &[msg]);
                    }
                }
                self.builder.ins().trap(cranelift_codegen::ir::TrapCode::user(1).unwrap());
            }

            // === Type Conversion Builtins ===
            BuiltinKind::ToString => {
                if !args.is_empty() {
                    let arg = &args[0];
                    let arg_val = self.compile_operand(arg)?;
                    let arg_ty = self.get_operand_type(arg);

                    match arg_ty {
                        MirType::String => {
                            // Already a string, just store it
                            self.store_to_place(dest, arg_val)?;
                        }
                        MirType::Bool => {
                            // Bool is i64 in Cranelift, but runtime expects i8
                            if let Some(f) = self.runtime_funcs.bool_to_string {
                                let arg_i8 = self.builder.ins().ireduce(types::I8, arg_val);
                                let call = self.builder.ins().call(f, &[arg_i8]);
                                let result = self.builder.inst_results(call)[0];
                                self.store_to_place(dest, result)?;
                            }
                        }
                        MirType::Int => {
                            if let Some(f) = self.runtime_funcs.int_to_string {
                                let call = self.builder.ins().call(f, &[arg_val]);
                                let result = self.builder.inst_results(call)[0];
                                self.store_to_place(dest, result)?;
                            }
                        }
                        MirType::Float => {
                            if let Some(f) = self.runtime_funcs.float_to_string {
                                let call = self.builder.ins().call(f, &[arg_val]);
                                let result = self.builder.inst_results(call)[0];
                                self.store_to_place(dest, result)?;
                            }
                        }
                        MirType::Char => {
                            if let Some(f) = self.runtime_funcs.char_to_string {
                                let call = self.builder.ins().call(f, &[arg_val]);
                                let result = self.builder.inst_results(call)[0];
                                self.store_to_place(dest, result)?;
                            }
                        }
                        _ => {}
                    }
                }
            }

            BuiltinKind::ToInt => {
                if !args.is_empty() {
                    let arg = &args[0];
                    let arg_val = self.compile_operand(arg)?;
                    let arg_ty = self.get_operand_type(arg);

                    let func_ref = match arg_ty {
                        MirType::Int => {
                            // Already an int, just store it
                            self.store_to_place(dest, arg_val)?;
                            return Ok(());
                        }
                        MirType::Float => self.runtime_funcs.float_to_int,
                        MirType::String => self.runtime_funcs.string_to_int,
                        _ => None,
                    };

                    if let Some(f) = func_ref {
                        let call = self.builder.ins().call(f, &[arg_val]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::ToFloat => {
                if !args.is_empty() {
                    let arg = &args[0];
                    let arg_val = self.compile_operand(arg)?;
                    let arg_ty = self.get_operand_type(arg);

                    let func_ref = match arg_ty {
                        MirType::Float => {
                            // Already a float, just store it
                            self.store_to_place(dest, arg_val)?;
                            return Ok(());
                        }
                        MirType::Int => self.runtime_funcs.int_to_float,
                        MirType::String => self.runtime_funcs.string_to_float,
                        _ => None,
                    };

                    if let Some(f) = func_ref {
                        let call = self.builder.ins().call(f, &[arg_val]);
                        let result = self.builder.inst_results(call)[0];
                        self.store_to_place(dest, result)?;
                    }
                }
            }

            BuiltinKind::First => {
                if args.is_empty() {
                    return Err(CodegenError::Internal {
                        message: "first() requires an array argument".into(),
                    });
                }

                let array_val = self.compile_operand(&args[0])?;

                // Determine element type from array operand
                let elem_ty = self.get_array_elem_type(&args[0])?;

                let result = match elem_ty {
                    MirType::Float | MirType::Float32 | MirType::Float64 => {
                        if let Some(f) = self.runtime_funcs.array_first_float {
                            let call = self.builder.ins().call(f, &[array_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_first_float not declared".into(),
                            });
                        }
                    }
                    _ => {
                        // Default to int for Int, Bool, Char, String, and other types
                        if let Some(f) = self.runtime_funcs.array_first_int {
                            let call = self.builder.ins().call(f, &[array_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_first_int not declared".into(),
                            });
                        }
                    }
                };

                self.store_to_place(dest, result)?;
            }

            BuiltinKind::Last => {
                if args.is_empty() {
                    return Err(CodegenError::Internal {
                        message: "last() requires an array argument".into(),
                    });
                }

                let array_val = self.compile_operand(&args[0])?;

                // Determine element type from array operand
                let elem_ty = self.get_array_elem_type(&args[0])?;

                let result = match elem_ty {
                    MirType::Float | MirType::Float32 | MirType::Float64 => {
                        if let Some(f) = self.runtime_funcs.array_last_float {
                            let call = self.builder.ins().call(f, &[array_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_last_float not declared".into(),
                            });
                        }
                    }
                    _ => {
                        // Default to int for Int, Bool, Char, String, and other types
                        if let Some(f) = self.runtime_funcs.array_last_int {
                            let call = self.builder.ins().call(f, &[array_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_last_int not declared".into(),
                            });
                        }
                    }
                };

                self.store_to_place(dest, result)?;
            }

            BuiltinKind::Reverse => {
                if args.is_empty() {
                    return Err(CodegenError::Internal {
                        message: "reverse() requires an array argument".into(),
                    });
                }

                let array_val = self.compile_operand(&args[0])?;

                // Determine element type from array operand
                let elem_ty = self.get_array_elem_type(&args[0])?;

                let result = match elem_ty {
                    MirType::Float | MirType::Float32 | MirType::Float64 => {
                        if let Some(f) = self.runtime_funcs.array_reverse_float {
                            let call = self.builder.ins().call(f, &[array_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_reverse_float not declared".into(),
                            });
                        }
                    }
                    _ => {
                        // Default to int for Int, Bool, Char, String, and other types
                        if let Some(f) = self.runtime_funcs.array_reverse_int {
                            let call = self.builder.ins().call(f, &[array_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_reverse_int not declared".into(),
                            });
                        }
                    }
                };

                self.store_to_place(dest, result)?;
            }

            BuiltinKind::Push => {
                if args.len() < 2 {
                    return Err(CodegenError::Internal {
                        message: "push() requires array and value arguments".into(),
                    });
                }

                let array_val = self.compile_operand(&args[0])?;
                let value_val = self.compile_operand(&args[1])?;

                // Determine element type from array operand
                let elem_ty = self.get_array_elem_type(&args[0])?;

                match elem_ty {
                    MirType::Float | MirType::Float32 | MirType::Float64 => {
                        if let Some(f) = self.runtime_funcs.array_push_float {
                            self.builder.ins().call(f, &[array_val, value_val]);
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_push_float not declared".into(),
                            });
                        }
                    }
                    _ => {
                        // Default to int for Int, Bool, Char, String, and other types
                        if let Some(f) = self.runtime_funcs.array_push_int {
                            self.builder.ins().call(f, &[array_val, value_val]);
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_push_int not declared".into(),
                            });
                        }
                    }
                }

                // Push doesn't return a value, but we need to store something
                // Store a unit value
                let unit_val = self.builder.ins().iconst(types::I8, 0);
                self.store_to_place(dest, unit_val)?;
            }

            BuiltinKind::Pop => {
                if args.is_empty() {
                    return Err(CodegenError::Internal {
                        message: "pop() requires an array argument".into(),
                    });
                }

                let array_val = self.compile_operand(&args[0])?;

                // Determine element type from array operand
                let elem_ty = self.get_array_elem_type(&args[0])?;

                let result = match elem_ty {
                    MirType::Float | MirType::Float32 | MirType::Float64 => {
                        if let Some(f) = self.runtime_funcs.array_pop_float {
                            let call = self.builder.ins().call(f, &[array_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_pop_float not declared".into(),
                            });
                        }
                    }
                    _ => {
                        // Default to int for Int, Bool, Char, String, and other types
                        if let Some(f) = self.runtime_funcs.array_pop_int {
                            let call = self.builder.ins().call(f, &[array_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_pop_int not declared".into(),
                            });
                        }
                    }
                };

                self.store_to_place(dest, result)?;
            }

            // === Higher-Order Collection Operations ===
            // Note: These require function pointers which are not yet fully supported
            // For now, we'll return errors for these operations
            BuiltinKind::Map
            | BuiltinKind::Filter
            | BuiltinKind::Reduce
            | BuiltinKind::Find
            | BuiltinKind::Any
            | BuiltinKind::All => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("builtin function {:?} (requires function pointer support)", builtin_kind),
                    span: None,
                });
            }

            BuiltinKind::Slice => {
                if args.len() < 3 {
                    return Err(CodegenError::Internal {
                        message: "slice() requires array, start, and end arguments".into(),
                    });
                }

                let array_val = self.compile_operand(&args[0])?;
                let start_val = self.compile_operand(&args[1])?;
                let end_val = self.compile_operand(&args[2])?;

                // Determine element type from array operand
                let elem_ty = self.get_array_elem_type(&args[0])?;

                let result = match elem_ty {
                    MirType::Float | MirType::Float32 | MirType::Float64 => {
                        if let Some(f) = self.runtime_funcs.array_slice_float {
                            let call = self.builder.ins().call(f, &[array_val, start_val, end_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_slice_float not declared".into(),
                            });
                        }
                    }
                    _ => {
                        // Default to int for Int, Bool, Char, String, and other types
                        if let Some(f) = self.runtime_funcs.array_slice_int {
                            let call = self.builder.ins().call(f, &[array_val, start_val, end_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_slice_int not declared".into(),
                            });
                        }
                    }
                };

                self.store_to_place(dest, result)?;
            }

            BuiltinKind::Concat => {
                if args.len() < 2 {
                    return Err(CodegenError::Internal {
                        message: "concat() requires two array arguments".into(),
                    });
                }

                let array1_val = self.compile_operand(&args[0])?;
                let array2_val = self.compile_operand(&args[1])?;

                // Determine element type from first array operand
                let elem_ty = self.get_array_elem_type(&args[0])?;

                let result = match elem_ty {
                    MirType::Float | MirType::Float32 | MirType::Float64 => {
                        if let Some(f) = self.runtime_funcs.array_concat_float {
                            let call = self.builder.ins().call(f, &[array1_val, array2_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_concat_float not declared".into(),
                            });
                        }
                    }
                    _ => {
                        // Default to int for Int, Bool, Char, String, and other types
                        if let Some(f) = self.runtime_funcs.array_concat_int {
                            let call = self.builder.ins().call(f, &[array1_val, array2_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_concat_int not declared".into(),
                            });
                        }
                    }
                };

                self.store_to_place(dest, result)?;
            }

            // Builtins not yet implemented
            BuiltinKind::TypeOf
            | BuiltinKind::StringSplit
            | BuiltinKind::Assert => {
                return Err(CodegenError::UnsupportedFeature {
                    feature: format!("builtin function {:?} (not yet implemented)", builtin_kind),
                    span: None,
                });
            }
        }

        // Jump to target block
        if let Some(target) = target {
            let target_block = self.blocks[&target];
            self.builder.ins().jump(target_block, &[]);
        }

        Ok(())
    }

    /// Get the MIR type of an operand
    fn get_operand_type(&self, operand: &Operand) -> MirType {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                self.mir_func.locals.get(place.local.0 as usize)
                    .map(|l| l.ty.clone())
                    .unwrap_or(MirType::Unit)
            }
            Operand::Constant(constant) => match constant {
                Constant::Unit => MirType::Unit,
                Constant::Bool(_) => MirType::Bool,
                Constant::Int(_) => MirType::Int,
                Constant::Float(_) => MirType::Float,
                Constant::Float32(_) => MirType::Float32,
                Constant::Float64(_) => MirType::Float64,
                Constant::Char(_) => MirType::Char,
                Constant::String(_) => MirType::String,
                Constant::Function(_) => MirType::Unit, // Function pointers
            },
        }
    }

    fn compile_rvalue(&mut self, rvalue: &Rvalue) -> Result<Value> {
        match rvalue {
            Rvalue::Use(operand) => self.compile_operand(operand),

            Rvalue::BinaryOp(op, left, right) => {
                // Check if this is string concatenation (Add on string operands)
                if *op == BinOp::Add {
                    let left_ty = self.get_operand_type(left);
                    if matches!(left_ty, MirType::String) {
                        // String concatenation
                        let left_val = self.compile_operand(left)?;
                        let right_val = self.compile_operand(right)?;

                        if let Some(f) = self.runtime_funcs.string_concat {
                            let call = self.builder.ins().call(f, &[left_val, right_val]);
                            return Ok(self.builder.inst_results(call)[0]);
                        } else {
                            return Err(CodegenError::Internal {
                                message: "string_concat runtime function not declared".into(),
                            });
                        }
                    }
                }

                let left_val = self.compile_operand(left)?;
                let right_val = self.compile_operand(right)?;
                self.compile_binop(*op, left_val, right_val)
            }

            Rvalue::UnaryOp(op, operand) => {
                let val = self.compile_operand(operand)?;
                self.compile_unop(*op, val)
            }

            Rvalue::Ref(place) => {
                // For now, just return the value (proper refs need stack slots)
                self.load_from_place(place)
            }

            Rvalue::RefMut(place) => {
                self.load_from_place(place)
            }

            Rvalue::Aggregate(kind, operands) => {
                match kind {
                    AggregateKind::Array(elem_ty) => {
                        // Allocate array with C runtime
                        let capacity = operands.len() as i64;
                        let elem_size = self.get_type_size(elem_ty);

                        let capacity_val = self.builder.ins().iconst(types::I64, capacity);
                        let elem_size_val = self.builder.ins().iconst(types::I64, elem_size);

                        // Call aria_array_new(capacity, elem_size) -> ptr
                        let array_ptr = if let Some(f) = self.runtime_funcs.array_new {
                            let call = self.builder.ins().call(f, &[capacity_val, elem_size_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "array_new runtime function not declared".into(),
                            });
                        };

                        // Set each element
                        for (i, operand) in operands.iter().enumerate() {
                            let val = self.compile_operand(operand)?;
                            let index = self.builder.ins().iconst(types::I64, i as i64);

                            // Choose appropriate setter based on element type
                            match elem_ty {
                                MirType::Int | MirType::Bool | MirType::Char => {
                                    if let Some(f) = self.runtime_funcs.array_set_int {
                                        self.builder.ins().call(f, &[array_ptr, index, val]);
                                    }
                                }
                                MirType::Float | MirType::Float32 | MirType::Float64 => {
                                    if let Some(f) = self.runtime_funcs.array_set_float {
                                        self.builder.ins().call(f, &[array_ptr, index, val]);
                                    }
                                }
                                MirType::String => {
                                    // Strings are pointers, use set_int
                                    if let Some(f) = self.runtime_funcs.array_set_int {
                                        self.builder.ins().call(f, &[array_ptr, index, val]);
                                    }
                                }
                                _ => {
                                    // For other types, use set_int (ptr-sized)
                                    if let Some(f) = self.runtime_funcs.array_set_int {
                                        self.builder.ins().call(f, &[array_ptr, index, val]);
                                    }
                                }
                            }
                        }

                        Ok(array_ptr)
                    }
                    AggregateKind::Tuple => {
                        // Allocate tuple on heap as array of pointer-sized values
                        let num_fields = operands.len() as i64;
                        let field_size = 8i64; // Pointer size
                        let total_size = num_fields * field_size;

                        let size_val = self.builder.ins().iconst(types::I64, total_size);

                        // Call aria_alloc to allocate memory
                        let tuple_ptr = if let Some(f) = self.runtime_funcs.alloc {
                            let call = self.builder.ins().call(f, &[size_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "alloc runtime function not declared".into(),
                            });
                        };

                        // Store each field at offset
                        for (i, operand) in operands.iter().enumerate() {
                            let val = self.compile_operand(operand)?;
                            let offset = (i as i32) * 8;
                            self.builder.ins().store(MemFlags::trusted(), val, tuple_ptr, offset);
                        }

                        Ok(tuple_ptr)
                    }
                    AggregateKind::Struct(_struct_id) => {
                        // Allocate struct on heap
                        // Each field is 8 bytes (pointer-sized for simplicity)
                        let num_fields = operands.len() as i64;
                        let field_size = 8i64;
                        let total_size = num_fields * field_size;

                        let size_val = self.builder.ins().iconst(types::I64, total_size);

                        // Call aria_alloc to allocate memory
                        let struct_ptr = if let Some(f) = self.runtime_funcs.alloc {
                            let call = self.builder.ins().call(f, &[size_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "alloc runtime function not declared".into(),
                            });
                        };

                        // Store each field at offset (field i at offset i*8)
                        for (i, operand) in operands.iter().enumerate() {
                            let val = self.compile_operand(operand)?;
                            let offset = (i as i32) * 8;
                            self.builder.ins().store(MemFlags::trusted(), val, struct_ptr, offset);
                        }

                        Ok(struct_ptr)
                    }
                    AggregateKind::Enum(_enum_id, variant_idx) => {
                        // Enum: discriminant (i64) + variant data
                        // Layout: [discriminant: i64][field0: i64][field1: i64]...
                        let num_fields = operands.len() as i64;
                        let total_size = 8 + (num_fields * 8); // discriminant + fields

                        let size_val = self.builder.ins().iconst(types::I64, total_size);

                        // Allocate memory
                        let enum_ptr = if let Some(f) = self.runtime_funcs.alloc {
                            let call = self.builder.ins().call(f, &[size_val]);
                            self.builder.inst_results(call)[0]
                        } else {
                            return Err(CodegenError::Internal {
                                message: "alloc runtime function not declared".into(),
                            });
                        };

                        // Store discriminant at offset 0
                        let discriminant = self.builder.ins().iconst(types::I64, *variant_idx as i64);
                        self.builder.ins().store(MemFlags::trusted(), discriminant, enum_ptr, 0);

                        // Store each field at offset 8, 16, 24, ...
                        for (i, operand) in operands.iter().enumerate() {
                            let val = self.compile_operand(operand)?;
                            let offset = ((i + 1) as i32) * 8;
                            self.builder.ins().store(MemFlags::trusted(), val, enum_ptr, offset);
                        }

                        Ok(enum_ptr)
                    }
                }
            }

            Rvalue::Discriminant(place) => {
                // Load the discriminant field (first field for enums)
                let val = self.load_from_place(place)?;
                Ok(val)
            }

            Rvalue::Len(place) => {
                // Get array pointer
                let array_ptr = self.load_from_place(place)?;

                // Call aria_array_length(array_ptr) -> i64
                if let Some(f) = self.runtime_funcs.array_length {
                    let call = self.builder.ins().call(f, &[array_ptr]);
                    Ok(self.builder.inst_results(call)[0])
                } else {
                    Err(CodegenError::Internal {
                        message: "array_length runtime function not declared".into(),
                    })
                }
            }

            Rvalue::Cast(_kind, operand, _to_ty) => {
                let val = self.compile_operand(operand)?;
                // For now, just pass through the value
                Ok(val)
            }

            Rvalue::Closure(fn_id, _captures) => {
                // Return function pointer
                let func_ref = *self.declared_funcs.get(fn_id).ok_or_else(|| {
                    CodegenError::UndefinedFunction {
                        name: format!("fn#{}", fn_id.0),
                    }
                })?;
                let ptr = self.builder.ins().func_addr(self.ptr_type, func_ref);
                Ok(ptr)
            }
        }
    }

    fn compile_operand(&mut self, operand: &Operand) -> Result<Value> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.load_from_place(place),
            Operand::Constant(constant) => self.compile_constant(constant),
        }
    }

    fn compile_constant(&mut self, constant: &Constant) -> Result<Value> {
        match constant {
            Constant::Unit => Ok(self.builder.ins().iconst(types::I8, 0)),
            Constant::Bool(b) => Ok(self.builder.ins().iconst(types::I64, *b as i64)),
            Constant::Int(i) => Ok(self.builder.ins().iconst(types::I64, *i)),
            Constant::Float(f) => Ok(self.builder.ins().f64const(*f)),
            Constant::Float32(f) => Ok(self.builder.ins().f32const(*f)),
            Constant::Float64(f) => Ok(self.builder.ins().f64const(*f)),
            Constant::Char(c) => Ok(self.builder.ins().iconst(types::I32, *c as i64)),
            Constant::String(idx) => {
                // Get pointer to string data
                let gv = *self.declared_data.get(idx).ok_or_else(|| {
                    CodegenError::Internal {
                        message: format!("string {} not found", idx),
                    }
                })?;
                let ptr = self.builder.ins().global_value(self.ptr_type, gv);
                Ok(ptr)
            }
            Constant::Function(fn_id) => {
                let func_ref = *self.declared_funcs.get(fn_id).ok_or_else(|| {
                    CodegenError::UndefinedFunction {
                        name: format!("fn#{}", fn_id.0),
                    }
                })?;
                let ptr = self.builder.ins().func_addr(self.ptr_type, func_ref);
                Ok(ptr)
            }
        }
    }

    fn compile_binop(&mut self, op: BinOp, left: Value, right: Value) -> Result<Value> {
        // Check if we're dealing with floats
        let left_type = self.builder.func.dfg.value_type(left);
        let is_float = matches!(left_type, types::F32 | types::F64);

        let val = match op {
            // Arithmetic
            BinOp::Add => {
                if is_float {
                    self.builder.ins().fadd(left, right)
                } else {
                    self.builder.ins().iadd(left, right)
                }
            }
            BinOp::Sub => {
                if is_float {
                    self.builder.ins().fsub(left, right)
                } else {
                    self.builder.ins().isub(left, right)
                }
            }
            BinOp::Mul => {
                if is_float {
                    self.builder.ins().fmul(left, right)
                } else {
                    self.builder.ins().imul(left, right)
                }
            }
            BinOp::Div => {
                if is_float {
                    self.builder.ins().fdiv(left, right)
                } else {
                    self.builder.ins().sdiv(left, right)
                }
            }
            BinOp::IntDiv => self.builder.ins().sdiv(left, right),
            BinOp::Rem => {
                if is_float {
                    // Float remainder - would need runtime call
                    self.builder.ins().fdiv(left, right) // Placeholder
                } else {
                    self.builder.ins().srem(left, right)
                }
            }
            BinOp::Pow => {
                // Use runtime pow function for floating point
                if is_float {
                    if let Some(f) = self.runtime_funcs.pow {
                        let call = self.builder.ins().call(f, &[left, right]);
                        self.builder.inst_results(call)[0]
                    } else {
                        self.builder.ins().fmul(left, right) // Fallback
                    }
                } else {
                    // Integer power - for now use multiplication as placeholder
                    // A proper implementation would need a runtime function
                    self.builder.ins().imul(left, right)
                }
            }

            // Bitwise
            BinOp::BitAnd => self.builder.ins().band(left, right),
            BinOp::BitOr => self.builder.ins().bor(left, right),
            BinOp::BitXor => self.builder.ins().bxor(left, right),
            BinOp::Shl => self.builder.ins().ishl(left, right),
            BinOp::Shr => self.builder.ins().sshr(left, right),

            // Comparison - results are Bool (represented as i64)
            BinOp::Eq => {
                if is_float {
                    let cmp = self.builder.ins().fcmp(FloatCC::Equal, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                } else {
                    let cmp = self.builder.ins().icmp(IntCC::Equal, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                }
            }
            BinOp::Ne => {
                if is_float {
                    let cmp = self.builder.ins().fcmp(FloatCC::NotEqual, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                } else {
                    let cmp = self.builder.ins().icmp(IntCC::NotEqual, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                }
            }
            BinOp::Lt => {
                if is_float {
                    let cmp = self.builder.ins().fcmp(FloatCC::LessThan, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                } else {
                    let cmp = self.builder.ins().icmp(IntCC::SignedLessThan, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                }
            }
            BinOp::Le => {
                if is_float {
                    let cmp = self.builder.ins().fcmp(FloatCC::LessThanOrEqual, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                } else {
                    let cmp = self.builder.ins().icmp(IntCC::SignedLessThanOrEqual, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                }
            }
            BinOp::Gt => {
                if is_float {
                    let cmp = self.builder.ins().fcmp(FloatCC::GreaterThan, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                } else {
                    let cmp = self.builder.ins().icmp(IntCC::SignedGreaterThan, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                }
            }
            BinOp::Ge => {
                if is_float {
                    let cmp = self.builder.ins().fcmp(FloatCC::GreaterThanOrEqual, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                } else {
                    let cmp = self.builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, left, right);
                    self.builder.ins().uextend(types::I64, cmp)
                }
            }

            // Logical (already evaluated, just compute)
            BinOp::And => self.builder.ins().band(left, right),
            BinOp::Or => self.builder.ins().bor(left, right),
        };

        Ok(val)
    }

    fn compile_unop(&mut self, op: UnOp, val: Value) -> Result<Value> {
        let result = match op {
            UnOp::Neg => self.builder.ins().ineg(val),
            UnOp::Not => {
                // Boolean not: xor with 1, using correct type
                let val_type = self.builder.func.dfg.value_type(val);
                let one = self.builder.ins().iconst(val_type, 1);
                self.builder.ins().bxor(val, one)
            }
            UnOp::BitNot => self.builder.ins().bnot(val),
        };

        Ok(result)
    }

    /// Get the size in bytes of a MIR type
    fn get_type_size(&self, ty: &MirType) -> i64 {
        match ty {
            MirType::Unit => 0,
            MirType::Bool => 1,
            MirType::Int => 8,
            MirType::Float => 8,
            MirType::Float32 => 4,
            MirType::Float64 => 8,
            MirType::Char => 4,
            MirType::String => 8,  // Pointer size
            MirType::Array(_) => 8,  // Pointer size
            MirType::Tuple(_) => 8,  // Pointer size
            MirType::Struct(_) => 8,  // Pointer size
            MirType::Enum(_) => 8,  // Pointer size
            MirType::FnPtr { .. } => 8,  // Pointer size
            MirType::Ref(_) => 8,  // Pointer size
            MirType::RefMut(_) => 8,  // Pointer size
            _ => 8,  // Default to pointer size for other types
        }
    }

    /// Get the element type of an array operand
    fn get_array_elem_type(&self, operand: &Operand) -> Result<MirType> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if place.projection.is_empty() {
                    let local_ty = &self.mir_func.locals[place.local.0 as usize].ty;
                    if let MirType::Array(elem_ty) = local_ty {
                        Ok(elem_ty.as_ref().clone())
                    } else {
                        Err(CodegenError::Internal {
                            message: format!("expected array type, got {:?}", local_ty),
                        })
                    }
                } else {
                    // Array with projections - default to Int
                    Ok(MirType::Int)
                }
            }
            Operand::Constant(_) => {
                // Constant arrays - default to Int
                Ok(MirType::Int)
            }
        }
    }

    fn load_from_place(&mut self, place: &Place) -> Result<Value> {
        let var = self.locals.get(&place.local).ok_or_else(|| CodegenError::Internal {
            message: format!("local {} not found", place.local),
        })?;

        let mut val = self.builder.use_var(*var);

        // Handle projections (field access, index, deref)
        for proj in &place.projection {
            match proj {
                PlaceElem::Field(idx) => {
                    // Field access: load from struct/tuple pointer at offset
                    // Layout: each field is 8 bytes (pointer-sized)
                    let struct_ptr = val;
                    let offset = (*idx as i32) * 8;
                    val = self.builder.ins().load(
                        types::I64,
                        MemFlags::trusted(),
                        struct_ptr,
                        offset,
                    );
                }
                PlaceElem::Index(idx_local) => {
                    // Array indexing with dynamic index
                    let array_ptr = val;

                    // Get index value
                    let idx_var = self.locals.get(idx_local).ok_or_else(|| CodegenError::Internal {
                        message: format!("index local {} not found", idx_local),
                    })?;
                    let index = self.builder.use_var(*idx_var);

                    // Get element type from local's type
                    let local_ty = &self.mir_func.locals[place.local.0 as usize].ty;
                    let elem_ty = if let MirType::Array(elem) = local_ty {
                        elem.as_ref()
                    } else {
                        return Err(CodegenError::Internal {
                            message: format!("expected array type, got {:?}", local_ty),
                        });
                    };

                    // Call appropriate getter based on element type
                    val = match elem_ty {
                        MirType::Int | MirType::Bool | MirType::Char | MirType::String => {
                            if let Some(f) = self.runtime_funcs.array_get_int {
                                let call = self.builder.ins().call(f, &[array_ptr, index]);
                                self.builder.inst_results(call)[0]
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_get_int not declared".into(),
                                });
                            }
                        }
                        MirType::Float | MirType::Float32 | MirType::Float64 => {
                            if let Some(f) = self.runtime_funcs.array_get_float {
                                let call = self.builder.ins().call(f, &[array_ptr, index]);
                                self.builder.inst_results(call)[0]
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_get_float not declared".into(),
                                });
                            }
                        }
                        _ => {
                            // For other types (nested arrays, structs, etc.), use get_int (ptr-sized)
                            if let Some(f) = self.runtime_funcs.array_get_int {
                                let call = self.builder.ins().call(f, &[array_ptr, index]);
                                self.builder.inst_results(call)[0]
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_get_int not declared".into(),
                                });
                            }
                        }
                    };
                }
                PlaceElem::ConstantIndex(idx) => {
                    // Array indexing with constant index
                    let array_ptr = val;
                    let index = self.builder.ins().iconst(types::I64, *idx as i64);

                    // Get element type from local's type
                    let local_ty = &self.mir_func.locals[place.local.0 as usize].ty;
                    let elem_ty = if let MirType::Array(elem) = local_ty {
                        elem.as_ref()
                    } else {
                        return Err(CodegenError::Internal {
                            message: format!("expected array type, got {:?}", local_ty),
                        });
                    };

                    // Call appropriate getter based on element type
                    val = match elem_ty {
                        MirType::Int | MirType::Bool | MirType::Char | MirType::String => {
                            if let Some(f) = self.runtime_funcs.array_get_int {
                                let call = self.builder.ins().call(f, &[array_ptr, index]);
                                self.builder.inst_results(call)[0]
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_get_int not declared".into(),
                                });
                            }
                        }
                        MirType::Float | MirType::Float32 | MirType::Float64 => {
                            if let Some(f) = self.runtime_funcs.array_get_float {
                                let call = self.builder.ins().call(f, &[array_ptr, index]);
                                self.builder.inst_results(call)[0]
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_get_float not declared".into(),
                                });
                            }
                        }
                        _ => {
                            // For other types, use get_int (ptr-sized)
                            if let Some(f) = self.runtime_funcs.array_get_int {
                                let call = self.builder.ins().call(f, &[array_ptr, index]);
                                self.builder.inst_results(call)[0]
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_get_int not declared".into(),
                                });
                            }
                        }
                    };
                }
                PlaceElem::Deref => {
                    // Load through pointer
                    val = self.builder.ins().load(
                        types::I64,
                        MemFlags::trusted(),
                        val,
                        0,
                    );
                }
                PlaceElem::Downcast(_variant) => {
                    // Enum downcast - no-op for value
                }
            }
        }

        Ok(val)
    }

    fn store_to_place(&mut self, place: &Place, value: Value) -> Result<()> {
        if place.projection.is_empty() {
            // Simple local assignment
            let var = self.locals.get(&place.local).ok_or_else(|| CodegenError::Internal {
                message: format!("local {} not found", place.local),
            })?;
            self.builder.def_var(*var, value);
        } else {
            // Handle projections for array indexing
            let last_proj = place.projection.last();

            match last_proj {
                Some(PlaceElem::Index(_)) | Some(PlaceElem::ConstantIndex(_)) => {
                    // Get array pointer
                    let var = self.locals.get(&place.local).ok_or_else(|| CodegenError::Internal {
                        message: format!("local {} not found", place.local),
                    })?;
                    let array_ptr = self.builder.use_var(*var);

                    // Get index
                    let index = match last_proj {
                        Some(PlaceElem::Index(idx_local)) => {
                            let idx_var = self.locals.get(idx_local).ok_or_else(|| {
                                CodegenError::Internal {
                                    message: format!("index local {} not found", idx_local),
                                }
                            })?;
                            self.builder.use_var(*idx_var)
                        }
                        Some(PlaceElem::ConstantIndex(idx)) => {
                            self.builder.ins().iconst(types::I64, *idx as i64)
                        }
                        _ => unreachable!(),
                    };

                    // Get element type from local's type
                    let local_ty = &self.mir_func.locals[place.local.0 as usize].ty;
                    let elem_ty = if let MirType::Array(elem) = local_ty {
                        elem.as_ref()
                    } else {
                        return Err(CodegenError::Internal {
                            message: format!("expected array type for store, got {:?}", local_ty),
                        });
                    };

                    // Call appropriate setter based on element type
                    match elem_ty {
                        MirType::Int | MirType::Bool | MirType::Char | MirType::String => {
                            if let Some(f) = self.runtime_funcs.array_set_int {
                                self.builder.ins().call(f, &[array_ptr, index, value]);
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_set_int not declared".into(),
                                });
                            }
                        }
                        MirType::Float | MirType::Float32 | MirType::Float64 => {
                            if let Some(f) = self.runtime_funcs.array_set_float {
                                self.builder.ins().call(f, &[array_ptr, index, value]);
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_set_float not declared".into(),
                                });
                            }
                        }
                        _ => {
                            // For other types, use set_int (ptr-sized)
                            if let Some(f) = self.runtime_funcs.array_set_int {
                                self.builder.ins().call(f, &[array_ptr, index, value]);
                            } else {
                                return Err(CodegenError::Internal {
                                    message: "array_set_int not declared".into(),
                                });
                            }
                        }
                    }
                }
                _ => {
                    // For other projections, just store to the base
                    let var = self.locals.get(&place.local).ok_or_else(|| CodegenError::Internal {
                        message: format!("local {} not found", place.local),
                    })?;
                    self.builder.def_var(*var, value);
                }
            }
        }

        Ok(())
    }
}
