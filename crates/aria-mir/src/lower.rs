//! AST to MIR lowering.
//!
//! This module handles the transformation from the high-level AST
//! to the control-flow-graph-based MIR representation.

use aria_ast::{self as ast, Visibility};
use aria_lexer::Span;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;

use crate::mir::*;
use crate::Result;

/// Context for lowering AST to MIR
pub struct LoweringContext {
    /// The MIR program being built
    program: MirProgram,

    /// Next function ID
    next_fn_id: u32,
    /// Next struct ID
    next_struct_id: u32,
    /// Next enum ID
    next_enum_id: u32,

    /// Mapping from function names to IDs
    fn_names: FxHashMap<SmolStr, FunctionId>,
    /// Mapping from struct names to IDs
    struct_names: FxHashMap<SmolStr, StructId>,
    /// Mapping from enum names to IDs
    enum_names: FxHashMap<SmolStr, EnumId>,

    /// Type inference context
    type_ctx: TypeInferenceContext,

    /// Module type registry for cross-module type checking
    module_registry: ModuleTypeRegistry,

    /// Current type parameters in scope (for generic functions/structs)
    /// Maps type parameter names (e.g., "T") to their representation in MIR
    current_type_params: Vec<SmolStr>,

    /// Counter for generating unique lambda names
    next_lambda_id: u32,
}

/// Registry for tracking types exported from modules
#[derive(Default)]
pub struct ModuleTypeRegistry {
    /// Exported function signatures: module_name::fn_name -> (params, return_type)
    pub function_signatures: FxHashMap<SmolStr, FunctionSignature>,
    /// Exported struct definitions: module_name::struct_name -> fields
    pub struct_definitions: FxHashMap<SmolStr, Vec<(SmolStr, MirType)>>,
    /// Exported enum definitions: module_name::enum_name -> variants
    pub enum_definitions: FxHashMap<SmolStr, Vec<(SmolStr, Vec<MirType>)>>,
    /// Module aliases from imports: alias -> full_path
    pub module_aliases: FxHashMap<SmolStr, SmolStr>,
}

/// Function signature for cross-module type checking
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<(SmolStr, MirType)>,
    pub return_type: MirType,
    pub type_params: Vec<SmolStr>,
}

impl ModuleTypeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a function from another module
    pub fn register_function(&mut self, full_name: SmolStr, signature: FunctionSignature) {
        self.function_signatures.insert(full_name, signature);
    }

    /// Register a struct from another module
    pub fn register_struct(&mut self, full_name: SmolStr, fields: Vec<(SmolStr, MirType)>) {
        self.struct_definitions.insert(full_name, fields);
    }

    /// Register an enum from another module
    pub fn register_enum(&mut self, full_name: SmolStr, variants: Vec<(SmolStr, Vec<MirType>)>) {
        self.enum_definitions.insert(full_name, variants);
    }

    /// Add a module alias from an import
    pub fn add_alias(&mut self, alias: SmolStr, full_path: SmolStr) {
        self.module_aliases.insert(alias, full_path);
    }

    /// Look up a function signature by name (handles aliases)
    pub fn lookup_function(&self, name: &str) -> Option<&FunctionSignature> {
        // Try direct lookup first
        if let Some(sig) = self.function_signatures.get(name) {
            return Some(sig);
        }

        // Try resolving through alias
        if let Some((alias, rest)) = name.split_once("::") {
            if let Some(full_module) = self.module_aliases.get(alias) {
                let full_name: SmolStr = format!("{}::{}", full_module, rest).into();
                return self.function_signatures.get(&full_name);
            }
        }

        None
    }

    /// Look up a struct definition by name (handles aliases)
    pub fn lookup_struct(&self, name: &str) -> Option<&Vec<(SmolStr, MirType)>> {
        // Try direct lookup first
        if let Some(def) = self.struct_definitions.get(name) {
            return Some(def);
        }

        // Try resolving through alias
        if let Some((alias, rest)) = name.split_once("::") {
            if let Some(full_module) = self.module_aliases.get(alias) {
                let full_name: SmolStr = format!("{}::{}", full_module, rest).into();
                return self.struct_definitions.get(&full_name);
            }
        }

        None
    }

    /// Look up an enum definition by name (handles aliases)
    pub fn lookup_enum(&self, name: &str) -> Option<&Vec<(SmolStr, Vec<MirType>)>> {
        // Try direct lookup first
        if let Some(def) = self.enum_definitions.get(name) {
            return Some(def);
        }

        // Try resolving through alias
        if let Some((alias, rest)) = name.split_once("::") {
            if let Some(full_module) = self.module_aliases.get(alias) {
                let full_name: SmolStr = format!("{}::{}", full_module, rest).into();
                return self.enum_definitions.get(&full_name);
            }
        }

        None
    }
}

/// Context for type inference with type variables
#[derive(Default)]
pub struct TypeInferenceContext {
    /// Next type variable ID
    next_type_var: u32,
    /// Type variable substitutions (resolved types)
    substitutions: FxHashMap<TypeVarId, MirType>,
}

impl TypeInferenceContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a fresh type variable
    pub fn fresh_type_var(&mut self) -> MirType {
        let id = TypeVarId::new(self.next_type_var);
        self.next_type_var += 1;
        MirType::TypeVar(id)
    }

    /// Resolve a type, following type variable substitutions
    pub fn resolve(&self, ty: &MirType) -> MirType {
        match ty {
            MirType::TypeVar(id) => {
                if let Some(resolved) = self.substitutions.get(id) {
                    self.resolve(resolved)
                } else {
                    ty.clone()
                }
            }
            MirType::Array(inner) => MirType::Array(Box::new(self.resolve(inner))),
            MirType::Tuple(elems) => {
                MirType::Tuple(elems.iter().map(|t| self.resolve(t)).collect())
            }
            MirType::Map(k, v) => {
                MirType::Map(Box::new(self.resolve(k)), Box::new(self.resolve(v)))
            }
            MirType::Optional(inner) => MirType::Optional(Box::new(self.resolve(inner))),
            MirType::Result(ok, err) => {
                MirType::Result(Box::new(self.resolve(ok)), Box::new(self.resolve(err)))
            }
            MirType::Ref(inner) => MirType::Ref(Box::new(self.resolve(inner))),
            MirType::RefMut(inner) => MirType::RefMut(Box::new(self.resolve(inner))),
            MirType::FnPtr { params, ret } => MirType::FnPtr {
                params: params.iter().map(|t| self.resolve(t)).collect(),
                ret: Box::new(self.resolve(ret)),
            },
            MirType::Closure { params, ret } => MirType::Closure {
                params: params.iter().map(|t| self.resolve(t)).collect(),
                ret: Box::new(self.resolve(ret)),
            },
            MirType::Generic { name, args } => MirType::Generic {
                name: name.clone(),
                args: args.iter().map(|t| self.resolve(t)).collect(),
            },
            _ => ty.clone(),
        }
    }

    /// Unify two types, updating substitutions if successful
    /// Returns true if unification succeeded
    pub fn unify(&mut self, a: &MirType, b: &MirType) -> bool {
        let a = self.resolve(a);
        let b = self.resolve(b);

        match (&a, &b) {
            // Same types unify
            _ if a == b => true,

            // Type variable unifies with anything
            (MirType::TypeVar(id), _) => {
                // Occurs check: prevent infinite types
                if b.collect_type_vars().contains(id) {
                    return false;
                }
                self.substitutions.insert(*id, b);
                true
            }
            (_, MirType::TypeVar(id)) => {
                // Occurs check
                if a.collect_type_vars().contains(id) {
                    return false;
                }
                self.substitutions.insert(*id, a);
                true
            }

            // Compound types must have matching structure
            (MirType::Array(a_inner), MirType::Array(b_inner)) => self.unify(a_inner, b_inner),
            (MirType::Tuple(a_elems), MirType::Tuple(b_elems)) if a_elems.len() == b_elems.len() => {
                a_elems
                    .iter()
                    .zip(b_elems.iter())
                    .all(|(a, b)| self.unify(a, b))
            }
            (MirType::Map(ak, av), MirType::Map(bk, bv)) => self.unify(ak, bk) && self.unify(av, bv),
            (MirType::Optional(a_inner), MirType::Optional(b_inner)) => self.unify(a_inner, b_inner),
            (MirType::Result(aok, aerr), MirType::Result(bok, berr)) => {
                self.unify(aok, bok) && self.unify(aerr, berr)
            }
            (MirType::Ref(a_inner), MirType::Ref(b_inner)) => self.unify(a_inner, b_inner),
            (MirType::RefMut(a_inner), MirType::RefMut(b_inner)) => self.unify(a_inner, b_inner),
            (
                MirType::FnPtr {
                    params: ap,
                    ret: ar,
                },
                MirType::FnPtr {
                    params: bp,
                    ret: br,
                },
            ) if ap.len() == bp.len() => {
                ap.iter().zip(bp.iter()).all(|(a, b)| self.unify(a, b)) && self.unify(ar, br)
            }
            (
                MirType::Generic {
                    name: an,
                    args: aa,
                },
                MirType::Generic {
                    name: bn,
                    args: ba,
                },
            ) if an == bn && aa.len() == ba.len() => {
                aa.iter().zip(ba.iter()).all(|(a, b)| self.unify(a, b))
            }

            // Otherwise, types don't unify
            _ => false,
        }
    }

    /// Reset the inference context for a new scope
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.next_type_var = 0;
        self.substitutions.clear();
    }
}

impl LoweringContext {
    pub fn new() -> Self {
        let mut ctx = Self {
            program: MirProgram::new(),
            next_fn_id: 0,
            next_struct_id: 0,
            next_enum_id: 0,
            fn_names: FxHashMap::default(),
            struct_names: FxHashMap::default(),
            enum_names: FxHashMap::default(),
            type_ctx: TypeInferenceContext::new(),
            module_registry: ModuleTypeRegistry::new(),
            current_type_params: Vec::new(),
            next_lambda_id: 0,
        };
        ctx.register_builtins();
        ctx
    }

    /// Get a reference to the module type registry
    pub fn module_registry(&self) -> &ModuleTypeRegistry {
        &self.module_registry
    }

    /// Get a mutable reference to the module type registry
    pub fn module_registry_mut(&mut self) -> &mut ModuleTypeRegistry {
        &mut self.module_registry
    }

    /// Generate a unique lambda name
    pub fn next_lambda_name(&mut self, prefix: &str) -> SmolStr {
        let id = self.next_lambda_id;
        self.next_lambda_id += 1;
        SmolStr::new(format!("{prefix}_{id}"))
    }

    /// Get a fresh type variable for inference
    pub fn fresh_type_var(&mut self) -> MirType {
        self.type_ctx.fresh_type_var()
    }

    /// Resolve a type through type variable substitutions
    pub fn resolve_type(&self, ty: &MirType) -> MirType {
        self.type_ctx.resolve(ty)
    }

    /// Unify two types, returning true if successful
    pub fn unify_types(&mut self, a: &MirType, b: &MirType) -> bool {
        self.type_ctx.unify(a, b)
    }

    /// Register all builtin functions
    fn register_builtins(&mut self) {
        use crate::mir::{BuiltinKind, Linkage, MirType};

        let builtins = [
            // I/O builtins
            ("print", BuiltinKind::Print, MirType::Unit),
            ("println", BuiltinKind::Println, MirType::Unit),

            // Collection/length
            ("len", BuiltinKind::Len, MirType::Int),

            // Type introspection and conversion
            ("type_of", BuiltinKind::TypeOf, MirType::String),
            ("to_string", BuiltinKind::ToString, MirType::String),
            ("to_int", BuiltinKind::ToInt, MirType::Int),
            ("to_float", BuiltinKind::ToFloat, MirType::Float),

            // String builtins
            ("contains", BuiltinKind::StringContains, MirType::Bool),
            ("starts_with", BuiltinKind::StringStartsWith, MirType::Bool),
            ("ends_with", BuiltinKind::StringEndsWith, MirType::Bool),
            ("trim", BuiltinKind::StringTrim, MirType::String),
            ("split", BuiltinKind::StringSplit, MirType::Array(Box::new(MirType::String))),
            ("replace", BuiltinKind::StringReplace, MirType::String),
            ("substring", BuiltinKind::Substring, MirType::String),
            ("char_at", BuiltinKind::CharAt, MirType::Int), // Return Int for ABI compatibility
            ("to_upper", BuiltinKind::ToUpper, MirType::String),
            ("to_lower", BuiltinKind::ToLower, MirType::String),

            // Math builtins
            ("abs", BuiltinKind::Abs, MirType::Int), // Simplified - actually polymorphic
            ("min", BuiltinKind::Min, MirType::Int),
            ("max", BuiltinKind::Max, MirType::Int),
            ("sqrt", BuiltinKind::Sqrt, MirType::Float),
            ("pow", BuiltinKind::Pow, MirType::Float),
            ("sin", BuiltinKind::Sin, MirType::Float),
            ("cos", BuiltinKind::Cos, MirType::Float),
            ("tan", BuiltinKind::Tan, MirType::Float),
            ("floor", BuiltinKind::Floor, MirType::Int),  // Returns int64_t in runtime
            ("ceil", BuiltinKind::Ceil, MirType::Int),    // Returns int64_t in runtime
            ("round", BuiltinKind::Round, MirType::Int),  // Returns int64_t in runtime

            // Collection builtins
            ("push", BuiltinKind::Push, MirType::Unit),
            ("pop", BuiltinKind::Pop, MirType::Int), // Returns element (simplified to Int)
            ("first", BuiltinKind::First, MirType::Int), // Returns element (simplified to Int)
            ("last", BuiltinKind::Last, MirType::Int), // Returns element (simplified to Int)
            ("reverse", BuiltinKind::Reverse, MirType::Array(Box::new(MirType::Int))), // Returns array

            // Higher-order collection operations
            ("map", BuiltinKind::Map, MirType::Array(Box::new(MirType::Int))), // Returns transformed array
            ("filter", BuiltinKind::Filter, MirType::Array(Box::new(MirType::Int))), // Returns filtered array
            ("reduce", BuiltinKind::Reduce, MirType::Int), // Returns accumulated value (simplified to Int)
            ("find", BuiltinKind::Find, MirType::Optional(Box::new(MirType::Int))), // Returns Option
            ("any", BuiltinKind::Any, MirType::Bool), // Returns bool
            ("all", BuiltinKind::All, MirType::Bool), // Returns bool
            ("slice", BuiltinKind::Slice, MirType::Array(Box::new(MirType::Int))), // Returns subarray
            ("concat", BuiltinKind::Concat, MirType::Array(Box::new(MirType::Int))), // Returns combined array

            // Control flow
            ("assert", BuiltinKind::Assert, MirType::Unit),
            ("panic", BuiltinKind::Panic, MirType::Never),
        ];

        for (name, kind, return_ty) in builtins {
            let id = self.alloc_fn_id();
            self.fn_names.insert(SmolStr::new(name), id);

            // Create a placeholder MirFunction with builtin linkage
            let mut func = MirFunction::new(
                SmolStr::new(name),
                return_ty,
                Span::dummy(),
            );
            func.linkage = Linkage::Builtin(kind);
            func.is_public = true;
            self.program.functions.insert(id, func);
        }
    }

    /// Lower an entire AST program to MIR
    pub fn lower_program(&mut self, program: &ast::Program) -> Result<MirProgram> {
        // First pass: collect all type and function declarations
        for item in &program.items {
            self.collect_item(item)?;
        }

        // Sync MirProgram's function ID counter with our counter
        // This ensures monomorphized functions get unique IDs
        self.program.set_next_fn_id(self.next_fn_id);

        // Second pass: lower function bodies
        for item in &program.items {
            self.lower_item(item)?;
        }

        Ok(std::mem::take(&mut self.program))
    }

    /// First pass: collect declarations to build symbol tables
    fn collect_item(&mut self, item: &ast::Item) -> Result<()> {
        match item {
            ast::Item::Function(f) => {
                let id = self.alloc_fn_id();
                self.fn_names.insert(f.name.node.clone(), id);
            }
            ast::Item::Struct(s) => {
                let id = self.alloc_struct_id();
                self.struct_names.insert(s.name.node.clone(), id);
            }
            ast::Item::Data(d) => {
                // Data is like struct in MIR
                let id = self.alloc_struct_id();
                self.struct_names.insert(d.name.node.clone(), id);
            }
            ast::Item::Enum(e) => {
                let id = self.alloc_enum_id();
                self.enum_names.insert(e.name.node.clone(), id);
            }
            ast::Item::Module(m) => {
                // Recursively collect items in module
                for item in &m.items {
                    self.collect_item(item)?;
                }
            }
            // Other items don't need pre-collection
            _ => {}
        }
        Ok(())
    }

    /// Second pass: lower item definitions
    fn lower_item(&mut self, item: &ast::Item) -> Result<()> {
        match item {
            ast::Item::Function(f) => self.lower_function(f),
            ast::Item::Struct(s) => self.lower_struct(s),
            ast::Item::Data(d) => self.lower_data(d),
            ast::Item::Enum(e) => self.lower_enum(e),
            ast::Item::Module(m) => {
                for item in &m.items {
                    self.lower_item(item)?;
                }
                Ok(())
            }
            ast::Item::Const(c) => self.lower_const(c),
            // Items that don't produce MIR directly
            ast::Item::Import(_) | ast::Item::Export(_) | ast::Item::Use(_) => Ok(()),
            ast::Item::TypeAlias(_) => Ok(()), // Type aliases are resolved at type check time
            ast::Item::Trait(_) | ast::Item::Impl(_) => {
                // Traits and impls are monomorphized away
                // TODO: Handle impl blocks by adding methods to types
                Ok(())
            }
            ast::Item::Test(t) => self.lower_test(t),
            ast::Item::Extern(_) => {
                // Extern declarations are handled at codegen time
                Ok(())
            }
        }
    }

    /// Lower a function declaration
    pub fn lower_function(&mut self, f: &ast::FunctionDecl) -> Result<()> {
        let fn_id = self.fn_names[&f.name.node];

        // Handle generic parameters - set current type params before lowering types
        let type_params: Vec<SmolStr> = if let Some(ref generic_params) = f.generic_params {
            generic_params.params.iter()
                .map(|p| p.name.node.clone())
                .collect()
        } else {
            Vec::new()
        };

        // Set current type parameters for type lowering
        self.current_type_params = type_params.clone();

        // Determine return type (now type params are in scope)
        let return_ty = f
            .return_type
            .as_ref()
            .map(|ty| self.lower_type(ty))
            .unwrap_or(MirType::Unit);

        let mut mir_fn = MirFunction::new(f.name.node.clone(), return_ty, f.span);
        mir_fn.is_public = f.visibility == Visibility::Public;
        mir_fn.type_params = type_params;

        // Create function body lowering context
        let mut fn_ctx = FunctionLoweringContext::new(self, &mut mir_fn);

        // Create entry block
        let entry_block = fn_ctx.func.new_block();
        fn_ctx.current_block = entry_block;

        // Add parameters as locals
        for param in &f.params {
            let param_ty = if let Some(ty) = param.ty.as_ref() {
                fn_ctx.ctx.lower_type(ty)
            } else {
                // Parameters without type annotations should be an error in strict mode,
                // but we default to Unit for compatibility with existing code.
                // In a future version, this should require type annotation or use
                // bidirectional type inference from call sites.
                MirType::Unit
            };

            let local = fn_ctx.func.new_local(param_ty, Some(param.name.node.clone()));
            fn_ctx.func.params.push(local);
            fn_ctx.locals.insert(param.name.node.clone(), local);
        }

        // Lower the function body
        match &f.body {
            ast::FunctionBody::Block(block) => {
                let block_value = fn_ctx.lower_block(block)?;
                // If the block has a value, assign it to the return place
                if let Some(value) = block_value {
                    fn_ctx.emit_assign(Place::return_place(), Rvalue::Use(value), block.span);
                }
                // Add implicit return if block doesn't end with return
                fn_ctx.ensure_terminated();
            }
            ast::FunctionBody::Expression(expr) => {
                let result = fn_ctx.lower_expr(expr)?;
                // Assign result to return place
                fn_ctx.emit_assign(Place::return_place(), Rvalue::Use(result), expr.span);
                fn_ctx.emit_terminator(TerminatorKind::Return, expr.span);
            }
        }

        // Register the function
        self.program.functions.insert(fn_id, mir_fn);

        // Lower contracts to MIR (after the function is registered)
        let contract = self.lower_contracts(&f.contracts)?;
        if let Some(contract) = contract {
            // Get the function from the program to set the contract
            if let Some(func) = self.program.functions.get_mut(&fn_id) {
                func.contract = Some(contract);
            }
        }
        self.program.fn_name_to_id.insert(f.name.node.clone(), fn_id);

        // Set entry point if this is main
        if f.name.node == "main" {
            self.program.entry = Some(fn_id);
        }

        // Clear type parameters after function lowering
        self.current_type_params.clear();

        Ok(())
    }

    /// Lower a struct declaration
    fn lower_struct(&mut self, s: &ast::StructDecl) -> Result<()> {
        let struct_id = self.struct_names[&s.name.node];

        let fields = s
            .fields
            .iter()
            .map(|f| MirField {
                name: f.name.node.clone(),
                ty: self.lower_type(&f.ty),
            })
            .collect();

        let mir_struct = MirStruct {
            name: s.name.node.clone(),
            fields,
            span: s.span,
            invariants: None, // TODO: Extract from attributes/contracts
        };

        self.program.structs.insert(struct_id, mir_struct);
        Ok(())
    }

    /// Lower a data declaration (immutable struct)
    fn lower_data(&mut self, d: &ast::DataDecl) -> Result<()> {
        let struct_id = self.struct_names[&d.name.node];

        let fields = d
            .fields
            .iter()
            .map(|f| MirField {
                name: f.name.node.clone(),
                ty: self.lower_type(&f.ty),
            })
            .collect();

        let mir_struct = MirStruct {
            name: d.name.node.clone(),
            fields,
            span: d.span,
            invariants: None, // TODO: Extract from attributes/contracts
        };

        self.program.structs.insert(struct_id, mir_struct);
        Ok(())
    }

    /// Lower an enum declaration
    fn lower_enum(&mut self, e: &ast::EnumDecl) -> Result<()> {
        let enum_id = self.enum_names[&e.name.node];

        let variants = e
            .variants
            .iter()
            .map(|v| {
                let fields = match &v.data {
                    ast::EnumVariantData::Unit => Vec::new(),
                    ast::EnumVariantData::Tuple(types) => {
                        types.iter().map(|ty| self.lower_type(ty)).collect()
                    }
                    ast::EnumVariantData::Struct(fields) => {
                        fields.iter().map(|f| self.lower_type(&f.ty)).collect()
                    }
                    ast::EnumVariantData::Discriminant(_) => Vec::new(),
                };

                MirVariant {
                    name: v.name.node.clone(),
                    fields,
                }
            })
            .collect();

        let mir_enum = MirEnum {
            name: e.name.node.clone(),
            variants,
            span: e.span,
        };

        self.program.enums.insert(enum_id, mir_enum);
        Ok(())
    }

    /// Lower a const declaration
    fn lower_const(&mut self, _c: &ast::ConstDecl) -> Result<()> {
        // Constants are inlined at use sites during lowering
        // TODO: Store constants for reference
        Ok(())
    }

    /// Lower a test declaration
    fn lower_test(&mut self, t: &ast::TestDecl) -> Result<()> {
        // Lower test as a function with a special name
        let fn_id = self.alloc_fn_id();
        let test_name: SmolStr = format!("__test_{}", t.name).into();
        self.fn_names.insert(test_name.clone(), fn_id);

        let mut mir_fn = MirFunction::new(test_name, MirType::Unit, t.span);

        let mut fn_ctx = FunctionLoweringContext::new(self, &mut mir_fn);
        let entry_block = fn_ctx.func.new_block();
        fn_ctx.current_block = entry_block;

        let _ = fn_ctx.lower_block(&t.body)?;
        fn_ctx.ensure_terminated();

        self.program.functions.insert(fn_id, mir_fn);
        Ok(())
    }

    /// Lower an AST type to MIR type
    pub fn lower_type(&self, ty: &ast::TypeExpr) -> MirType {
        match ty {
            ast::TypeExpr::Named(ident) => {
                let name = &ident.node;
                // Check built-in types
                match name.as_str() {
                    "Unit" | "()" => MirType::Unit,
                    "Bool" => MirType::Bool,
                    "Int" => MirType::Int,
                    "Int8" => MirType::Int8,
                    "Int16" => MirType::Int16,
                    "Int32" => MirType::Int32,
                    "Int64" => MirType::Int64,
                    "UInt" => MirType::UInt,
                    "UInt8" => MirType::UInt8,
                    "UInt16" => MirType::UInt16,
                    "UInt32" => MirType::UInt32,
                    "UInt64" => MirType::UInt64,
                    "Float" => MirType::Float,
                    "Float32" => MirType::Float32,
                    "Float64" => MirType::Float64,
                    "Char" => MirType::Char,
                    "String" => MirType::String,
                    _ => {
                        // Check if it's a type parameter (e.g., T in fn foo<T>)
                        if self.current_type_params.iter().any(|p| p.as_str() == name.as_str()) {
                            MirType::TypeParam(name.clone())
                        } else if let Some(&id) = self.struct_names.get(name) {
                            // Check if it's a known struct or enum
                            MirType::Struct(id)
                        } else if let Some(&id) = self.enum_names.get(name) {
                            MirType::Enum(id)
                        } else {
                            // Unknown type - should have been caught by type checker
                            MirType::Unit
                        }
                    }
                }
            }
            ast::TypeExpr::Generic { name, args, .. } => {
                match name.node.as_str() {
                    "Array" | "Vec" if args.len() == 1 => {
                        MirType::Array(Box::new(self.lower_type(&args[0])))
                    }
                    "Map" | "HashMap" if args.len() == 2 => MirType::Map(
                        Box::new(self.lower_type(&args[0])),
                        Box::new(self.lower_type(&args[1])),
                    ),
                    "Option" if args.len() == 1 => {
                        MirType::Optional(Box::new(self.lower_type(&args[0])))
                    }
                    "Result" if args.len() >= 1 => {
                        let ok = self.lower_type(&args[0]);
                        let err = args.get(1).map(|t| self.lower_type(t)).unwrap_or(MirType::Unit);
                        MirType::Result(Box::new(ok), Box::new(err))
                    }
                    _ => {
                        // Custom generic type - lookup by name
                        if let Some(&id) = self.struct_names.get(&name.node) {
                            MirType::Struct(id)
                        } else if let Some(&id) = self.enum_names.get(&name.node) {
                            MirType::Enum(id)
                        } else {
                            MirType::Unit
                        }
                    }
                }
            }
            ast::TypeExpr::Array { element, .. } => {
                MirType::Array(Box::new(self.lower_type(element)))
            }
            ast::TypeExpr::Map { key, value, .. } => MirType::Map(
                Box::new(self.lower_type(key)),
                Box::new(self.lower_type(value)),
            ),
            ast::TypeExpr::Tuple { elements, .. } => {
                MirType::Tuple(elements.iter().map(|t| self.lower_type(t)).collect())
            }
            ast::TypeExpr::Optional { inner, .. } => {
                MirType::Optional(Box::new(self.lower_type(inner)))
            }
            ast::TypeExpr::Result { ok, err, .. } => {
                let ok_ty = self.lower_type(ok);
                let err_ty = err
                    .as_ref()
                    .map(|e| self.lower_type(e))
                    .unwrap_or(MirType::Unit);
                MirType::Result(Box::new(ok_ty), Box::new(err_ty))
            }
            ast::TypeExpr::Reference { mutable, inner, .. } => {
                let inner_ty = Box::new(self.lower_type(inner));
                if *mutable {
                    MirType::RefMut(inner_ty)
                } else {
                    MirType::Ref(inner_ty)
                }
            }
            ast::TypeExpr::Function {
                params,
                return_type,
                ..
            } => {
                let param_tys = params.iter().map(|t| self.lower_type(t)).collect();
                let ret_ty = return_type
                    .as_ref()
                    .map(|t| self.lower_type(t))
                    .unwrap_or(MirType::Unit);
                MirType::FnPtr {
                    params: param_tys,
                    ret: Box::new(ret_ty),
                }
            }
            ast::TypeExpr::Path { segments, .. } => {
                // For now, just use the last segment as the type name
                if let Some(last) = segments.last() {
                    self.lower_type(&ast::TypeExpr::Named(last.clone()))
                } else {
                    MirType::Unit
                }
            }
            ast::TypeExpr::Inferred(_) => {
                // Should have been resolved by type checker
                MirType::Unit
            }
        }
    }

    /// Lower contracts from AST to MIR
    fn lower_contracts(
        &mut self,
        contracts: &[ast::Contract],
    ) -> Result<Option<FunctionContract>> {
        if contracts.is_empty() {
            return Ok(None);
        }

        let mut mir_contract = FunctionContract::new();

        for contract in contracts {
            match contract {
                ast::Contract::Requires(clause) => {
                    let mir_clause = self.lower_contract_clause(clause)?;
                    mir_contract.requires.push(mir_clause);
                }
                ast::Contract::Ensures(clause) => {
                    let mir_clause = self.lower_contract_clause(clause)?;
                    mir_contract.ensures.push(mir_clause);
                }
                ast::Contract::Invariant(_clause) => {
                    // Loop invariants are handled separately during loop lowering
                    // Struct invariants are handled in struct lowering
                }
            }
        }

        if mir_contract.is_empty() {
            Ok(None)
        } else {
            Ok(Some(mir_contract))
        }
    }

    /// Lower a single contract clause
    fn lower_contract_clause(&mut self, clause: &ast::ContractClause) -> Result<ContractClause> {
        let condition = self.lower_contract_expr(&clause.condition)?;
        let message = clause.message.as_ref().map(|s| s.to_string());

        Ok(ContractClause {
            condition,
            message,
            span: clause.span,
        })
    }

    /// Lower a contract expression from AST to MIR contract expression
    fn lower_contract_expr(&mut self, expr: &ast::Expr) -> Result<crate::mir::Expr> {
        use crate::mir::Expr as MirExpr;

        match &expr.kind {
            ast::ExprKind::Bool(b) => Ok(MirExpr::Bool(*b)),
            ast::ExprKind::Integer(s) => {
                let val = s.parse::<i64>().unwrap_or(0);
                Ok(MirExpr::Int(val))
            }
            ast::ExprKind::Float(s) => {
                let val = s.parse::<f64>().unwrap_or(0.0);
                Ok(MirExpr::Float(val))
            }

            ast::ExprKind::Ident(name) => {
                // This references a parameter or local variable
                // For now, we'll create a placeholder - in full implementation,
                // we'd need to track the mapping from names to locals
                Ok(MirExpr::Local(Local(0))) // Placeholder
            }

            ast::ExprKind::SelfLower => {
                // self reference
                Ok(MirExpr::Local(Local(1))) // Typically first param
            }

            ast::ExprKind::Binary { op, left, right } => {
                let left_expr = Box::new(self.lower_contract_expr(left)?);
                let right_expr = Box::new(self.lower_contract_expr(right)?);

                // Convert AST binary op to MIR binary op
                let mir_op = match op {
                    ast::BinaryOp::Add => BinOp::Add,
                    ast::BinaryOp::Sub => BinOp::Sub,
                    ast::BinaryOp::Mul => BinOp::Mul,
                    ast::BinaryOp::Div => BinOp::Div,
                    ast::BinaryOp::Mod => BinOp::Rem,
                    ast::BinaryOp::Pow => BinOp::Pow,
                    ast::BinaryOp::Eq => BinOp::Eq,
                    ast::BinaryOp::NotEq => BinOp::Ne,
                    ast::BinaryOp::Lt => BinOp::Lt,
                    ast::BinaryOp::LtEq => BinOp::Le,
                    ast::BinaryOp::Gt => BinOp::Gt,
                    ast::BinaryOp::GtEq => BinOp::Ge,
                    ast::BinaryOp::And => BinOp::And,
                    ast::BinaryOp::Or => BinOp::Or,
                    ast::BinaryOp::BitAnd => BinOp::BitAnd,
                    ast::BinaryOp::BitOr => BinOp::BitOr,
                    ast::BinaryOp::BitXor => BinOp::BitXor,
                    _ => BinOp::Eq, // Fallback for unsupported ops
                };

                Ok(MirExpr::Binary {
                    op: mir_op,
                    left: left_expr,
                    right: right_expr,
                })
            }

            ast::ExprKind::Unary { op, operand } => {
                let operand_expr = Box::new(self.lower_contract_expr(operand)?);

                let mir_op = match op {
                    ast::UnaryOp::Neg => UnOp::Neg,
                    ast::UnaryOp::Not => UnOp::Not,
                    ast::UnaryOp::BitNot => UnOp::BitNot,
                    _ => UnOp::Not, // Fallback
                };

                Ok(MirExpr::Unary {
                    op: mir_op,
                    operand: operand_expr,
                })
            }

            ast::ExprKind::Field { object, field } => {
                let object_expr = Box::new(self.lower_contract_expr(object)?);
                // Field index would need to be resolved - for now use 0
                Ok(MirExpr::Field {
                    object: object_expr,
                    field: 0, // Placeholder
                })
            }

            ast::ExprKind::MethodCall { object, method, args } => {
                let object_expr = Box::new(self.lower_contract_expr(object)?);
                let arg_exprs: Result<Vec<_>> = args
                    .iter()
                    .map(|arg| self.lower_contract_expr(arg))
                    .collect();

                Ok(MirExpr::MethodCall {
                    object: object_expr,
                    method: method.node.clone(),
                    args: arg_exprs?,
                })
            }

            ast::ExprKind::Old(inner) => {
                let inner_expr = Box::new(self.lower_contract_expr(inner)?);
                Ok(MirExpr::Old(inner_expr))
            }

            ast::ExprKind::Result => Ok(MirExpr::Result),

            _ => {
                // Unsupported expression in contract - default to true
                Ok(MirExpr::Bool(true))
            }
        }
    }

    fn alloc_fn_id(&mut self) -> FunctionId {
        let id = FunctionId(self.next_fn_id);
        self.next_fn_id += 1;
        id
    }

    fn alloc_struct_id(&mut self) -> StructId {
        let id = StructId(self.next_struct_id);
        self.next_struct_id += 1;
        id
    }

    fn alloc_enum_id(&mut self) -> EnumId {
        let id = EnumId(self.next_enum_id);
        self.next_enum_id += 1;
        id
    }

    /// Register an anonymous function (e.g., lambda/closure)
    pub fn register_anonymous_function(&mut self, func: MirFunction) -> FunctionId {
        let fn_id = self.alloc_fn_id();
        self.program.functions.insert(fn_id, func);
        fn_id
    }

    /// Look up a function by name
    pub fn lookup_function(&self, name: &str) -> Option<FunctionId> {
        self.fn_names.get(name).copied()
    }

    /// Get a function by ID
    pub fn get_function(&self, fn_id: &FunctionId) -> Option<&MirFunction> {
        self.program.functions.get(fn_id)
    }

    /// Get or create a monomorphized version of a generic function
    pub fn get_or_create_monomorphized_function(
        &mut self,
        generic_fn_id: FunctionId,
        type_args: Vec<MirType>,
    ) -> FunctionId {
        self.program.get_or_create_mono(generic_fn_id, type_args)
    }

    /// Look up a struct by name
    pub fn lookup_struct(&self, name: &str) -> Option<StructId> {
        self.struct_names.get(name).copied()
    }

    /// Look up an enum by name
    pub fn lookup_enum(&self, name: &str) -> Option<EnumId> {
        self.enum_names.get(name).copied()
    }

    /// Get a struct definition by ID
    pub fn get_struct(&self, id: StructId) -> Option<&MirStruct> {
        self.program.structs.get(&id)
    }

    /// Get an enum definition by ID
    pub fn get_enum(&self, id: EnumId) -> Option<&MirEnum> {
        self.program.enums.get(&id)
    }

    /// Look up a struct field by name, returns (field_index, field_type)
    pub fn lookup_struct_field(&self, struct_id: StructId, field_name: &str) -> Option<(u32, MirType)> {
        let struct_def = self.program.structs.get(&struct_id)?;
        for (idx, field) in struct_def.fields.iter().enumerate() {
            if field.name == field_name {
                return Some((idx as u32, field.ty.clone()));
            }
        }
        None
    }

    /// Look up a struct field by index
    pub fn get_struct_field_type(&self, struct_id: StructId, field_idx: u32) -> Option<MirType> {
        let struct_def = self.program.structs.get(&struct_id)?;
        struct_def.fields.get(field_idx as usize).map(|f| f.ty.clone())
    }

    /// Look up an enum variant by name, returns variant index
    pub fn lookup_enum_variant(&self, enum_id: EnumId, variant_name: &str) -> Option<u32> {
        let enum_def = self.program.enums.get(&enum_id)?;
        for (idx, variant) in enum_def.variants.iter().enumerate() {
            if variant.name == variant_name {
                return Some(idx as u32);
            }
        }
        None
    }

    /// Get an enum variant field type by index
    pub fn get_enum_variant_field_type(&self, enum_id: EnumId, variant_idx: u32, field_idx: u32) -> Option<MirType> {
        let enum_def = self.program.enums.get(&enum_id)?;
        let variant = enum_def.variants.get(variant_idx as usize)?;
        variant.fields.get(field_idx as usize).cloned()
    }

    /// Intern a string in the program's string table
    pub fn intern_string(&mut self, s: SmolStr) -> u32 {
        self.program.intern_string(s)
    }
}

impl Default for LoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for lowering a single function
pub struct FunctionLoweringContext<'a> {
    /// The global lowering context
    pub ctx: &'a mut LoweringContext,
    /// The function being built
    pub func: &'a mut MirFunction,
    /// Current basic block
    pub current_block: BlockId,
    /// Local variable bindings (name -> local)
    pub locals: FxHashMap<SmolStr, Local>,
    /// Loop context stack (break_target, continue_target)
    pub loop_stack: Vec<(BlockId, BlockId)>,
}

impl<'a> FunctionLoweringContext<'a> {
    pub fn new(ctx: &'a mut LoweringContext, func: &'a mut MirFunction) -> Self {
        Self {
            ctx,
            func,
            current_block: BlockId::ENTRY,
            locals: FxHashMap::default(),
            loop_stack: Vec::new(),
        }
    }

    /// Lower a block of statements
    pub fn lower_block(&mut self, block: &ast::Block) -> Result<Option<Operand>> {
        let mut last_expr_value = None;

        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = i == block.stmts.len() - 1;

            // If this is the last statement and it's an expression, capture its value
            if is_last {
                if let ast::StmtKind::Expr(expr) = &stmt.kind {
                    last_expr_value = Some(self.lower_expr(expr)?);
                    continue;
                }
            }

            self.lower_stmt(stmt)?;

            // Stop if we've already terminated the block
            if self.is_terminated() {
                break;
            }
        }
        Ok(last_expr_value)
    }

    /// Check if current block is terminated
    pub fn is_terminated(&self) -> bool {
        self.func.block(self.current_block).terminator.is_some()
    }

    /// Ensure the current block has a terminator
    pub fn ensure_terminated(&mut self) {
        if !self.is_terminated() {
            self.emit_terminator(TerminatorKind::Return, Span::dummy());
        }
    }

    /// Emit a statement to the current block
    pub fn emit_stmt(&mut self, kind: StatementKind, span: Span) {
        let stmt = Statement { kind, span };
        self.func.block_mut(self.current_block).push_stmt(stmt);
    }

    /// Emit an assignment statement
    pub fn emit_assign(&mut self, place: Place, rvalue: Rvalue, span: Span) {
        self.emit_stmt(StatementKind::Assign(place, rvalue), span);
    }

    /// Emit a terminator to the current block
    pub fn emit_terminator(&mut self, kind: TerminatorKind, span: Span) {
        let term = Terminator { kind, span };
        self.func.block_mut(self.current_block).set_terminator(term);
    }

    /// Create a new temporary local
    pub fn new_temp(&mut self, ty: MirType) -> Local {
        self.func.new_local(ty, None)
    }

    /// Create a new named local
    pub fn new_named_local(&mut self, name: SmolStr, ty: MirType) -> Local {
        let local = self.func.new_local(ty, Some(name.clone()));
        self.locals.insert(name, local);
        local
    }

    /// Look up a local by name
    pub fn lookup_local(&self, name: &str) -> Option<Local> {
        self.locals.get(name).copied()
    }

    /// Push a loop context
    pub fn push_loop(&mut self, break_target: BlockId, continue_target: BlockId) {
        self.loop_stack.push((break_target, continue_target));
    }

    /// Pop a loop context
    pub fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    /// Get current loop break target
    pub fn break_target(&self) -> Option<BlockId> {
        self.loop_stack.last().map(|(b, _)| *b)
    }

    /// Get current loop continue target
    pub fn continue_target(&self) -> Option<BlockId> {
        self.loop_stack.last().map(|(_, c)| *c)
    }
}

// Include expression and statement lowering implementations
use crate::lower_expr::*;
use crate::lower_stmt::*;

impl<'a> FunctionLoweringContext<'a> {
    /// Lower an expression and return an operand
    pub fn lower_expr(&mut self, expr: &ast::Expr) -> Result<Operand> {
        lower_expr(self, expr)
    }

    /// Lower an expression to a place (for assignments)
    pub fn lower_expr_to_place(&mut self, expr: &ast::Expr) -> Result<Place> {
        lower_expr_to_place(self, expr)
    }

    /// Lower a statement
    pub fn lower_stmt(&mut self, stmt: &ast::Stmt) -> Result<()> {
        lower_stmt(self, stmt)
    }
}
