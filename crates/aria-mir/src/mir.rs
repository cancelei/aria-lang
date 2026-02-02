//! MIR (Mid-level Intermediate Representation) data structures.
//!
//! MIR is a CFG-based IR designed for optimization and code generation.
//! It flattens control flow into basic blocks with explicit terminators.

use aria_lexer::Span;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::fmt;

// ============================================================================
// Identifiers
// ============================================================================

/// Unique identifier for a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub u32);

/// Unique identifier for a struct type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructId(pub u32);

/// Unique identifier for an enum type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumId(pub u32);

/// Local variable index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Local(pub u32);

impl Local {
    /// The return place (local 0)
    pub const RETURN: Local = Local(0);
}

/// Basic block identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl BlockId {
    /// The entry block (block 0)
    pub const ENTRY: BlockId = BlockId(0);
}

// ============================================================================
// Program Structure
// ============================================================================

/// A complete MIR program
#[derive(Debug, Clone)]
pub struct MirProgram {
    /// All functions in the program
    pub functions: FxHashMap<FunctionId, MirFunction>,
    /// All struct definitions
    pub structs: FxHashMap<StructId, MirStruct>,
    /// All enum definitions
    pub enums: FxHashMap<EnumId, MirEnum>,
    /// All effect definitions
    pub effects: FxHashMap<EffectId, MirEffect>,
    /// All handler definitions
    pub handlers: FxHashMap<HandlerId, MirHandler>,
    /// Entry point function (main)
    pub entry: Option<FunctionId>,
    /// String literals (interned)
    pub strings: Vec<SmolStr>,
    /// Next effect ID for allocation
    next_effect_id: u32,
    /// Next handler ID for allocation
    next_handler_id: u32,
    /// Next function ID for monomorphization
    next_fn_id: u32,
    /// Monomorphization cache: (generic_fn_id, type_args) -> specialized_fn_id
    pub mono_cache: FxHashMap<(FunctionId, Vec<MirType>), FunctionId>,
    /// Map from function name to ID for lookup
    pub fn_name_to_id: FxHashMap<SmolStr, FunctionId>,
}

impl MirProgram {
    pub fn new() -> Self {
        Self {
            functions: FxHashMap::default(),
            structs: FxHashMap::default(),
            enums: FxHashMap::default(),
            effects: FxHashMap::default(),
            handlers: FxHashMap::default(),
            entry: None,
            strings: Vec::new(),
            next_effect_id: 0,
            next_handler_id: 0,
            next_fn_id: 0,
            mono_cache: FxHashMap::default(),
            fn_name_to_id: FxHashMap::default(),
        }
    }

    /// Allocate a new function ID
    pub fn alloc_fn_id(&mut self) -> FunctionId {
        let id = FunctionId(self.next_fn_id);
        self.next_fn_id += 1;
        id
    }

    /// Set the next function ID counter (used to sync with lowering context)
    pub fn set_next_fn_id(&mut self, id: u32) {
        self.next_fn_id = id;
    }

    /// Look up a function by name
    pub fn function_by_name(&self, name: &str) -> Option<(FunctionId, &MirFunction)> {
        let name: SmolStr = name.into();
        self.fn_name_to_id.get(&name).and_then(|id| {
            self.functions.get(id).map(|f| (*id, f))
        })
    }

    /// Get or create a monomorphized version of a generic function
    pub fn get_or_create_mono(
        &mut self,
        generic_fn_id: FunctionId,
        type_args: Vec<MirType>,
    ) -> FunctionId {
        // Check cache first
        let key = (generic_fn_id, type_args.clone());
        if let Some(&mono_id) = self.mono_cache.get(&key) {
            return mono_id;
        }

        // Create new monomorphized function
        let generic_fn = self.functions.get(&generic_fn_id).expect("generic function not found").clone();
        let mono_id = self.alloc_fn_id();

        // Create substitutions
        let substitutions: Vec<(SmolStr, MirType)> = generic_fn.type_params
            .iter()
            .cloned()
            .zip(type_args.iter().cloned())
            .collect();

        // Generate monomorphized name
        let mono_name: SmolStr = format!(
            "{}_mono_{}",
            generic_fn.name,
            type_args.iter()
                .map(|t| format!("{:?}", t).replace(['<', '>', ',', ' '], "_"))
                .collect::<Vec<_>>()
                .join("_")
        ).into();

        // Create the monomorphized function
        let mut mono_fn = generic_fn.clone();
        mono_fn.name = mono_name.clone();
        mono_fn.generic_origin = Some(generic_fn_id);
        mono_fn.type_args = type_args.clone();
        mono_fn.type_params = Vec::new(); // Monomorphized function is no longer generic

        // Substitute types in return type
        mono_fn.return_ty = mono_fn.return_ty.substitute(&substitutions);

        // Substitute types in locals
        for local in &mut mono_fn.locals {
            local.ty = local.ty.substitute(&substitutions);
        }

        // Substitute types in statements and terminators
        for block in &mut mono_fn.blocks {
            for stmt in &mut block.statements {
                substitute_types_in_stmt(stmt, &substitutions);
            }
            substitute_types_in_terminator(&mut block.terminator, &substitutions);
        }

        // Register the monomorphized function
        self.functions.insert(mono_id, mono_fn);
        self.fn_name_to_id.insert(mono_name, mono_id);
        self.mono_cache.insert(key, mono_id);

        mono_id
    }

    /// Intern a string and return its index
    pub fn intern_string(&mut self, s: SmolStr) -> u32 {
        if let Some(idx) = self.strings.iter().position(|x| x == &s) {
            idx as u32
        } else {
            let idx = self.strings.len() as u32;
            self.strings.push(s);
            idx
        }
    }

    /// Create a new effect and return its ID
    pub fn new_effect(&mut self, name: SmolStr, span: Span) -> EffectId {
        let id = EffectId(self.next_effect_id);
        self.next_effect_id += 1;
        let effect = MirEffect::new(id, name, span);
        self.effects.insert(id, effect);
        id
    }

    /// Get an effect by ID
    pub fn effect(&self, id: EffectId) -> Option<&MirEffect> {
        self.effects.get(&id)
    }

    /// Get a mutable effect by ID
    pub fn effect_mut(&mut self, id: EffectId) -> Option<&mut MirEffect> {
        self.effects.get_mut(&id)
    }

    /// Create a new handler and return its ID
    pub fn new_handler(&mut self, effect: EffectType, span: Span) -> HandlerId {
        let id = HandlerId(self.next_handler_id);
        self.next_handler_id += 1;
        let handler = MirHandler::new(id, effect, span);
        self.handlers.insert(id, handler);
        id
    }

    /// Get a handler by ID
    pub fn handler(&self, id: HandlerId) -> Option<&MirHandler> {
        self.handlers.get(&id)
    }

    /// Get a mutable handler by ID
    pub fn handler_mut(&mut self, id: HandlerId) -> Option<&mut MirHandler> {
        self.handlers.get_mut(&id)
    }
}

impl Default for MirProgram {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Types
// ============================================================================

/// Type variable ID for generic type inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVarId(pub u32);

impl TypeVarId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

impl fmt::Display for TypeVarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?T{}", self.0)
    }
}

/// MIR type representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirType {
    // Primitives
    Unit,
    Bool,
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float,
    Float32,
    Float64,
    Char,
    String,

    // Compound types
    Array(Box<MirType>),
    Tuple(Vec<MirType>),
    Map(Box<MirType>, Box<MirType>),
    Optional(Box<MirType>),
    Result(Box<MirType>, Box<MirType>),

    // References
    Ref(Box<MirType>),
    RefMut(Box<MirType>),

    // Named types
    Struct(StructId),
    Enum(EnumId),

    // Function pointer
    FnPtr {
        params: Vec<MirType>,
        ret: Box<MirType>,
    },

    // Closure (captured environment + function)
    Closure {
        params: Vec<MirType>,
        ret: Box<MirType>,
    },

    // Never type (for diverging functions)
    Never,

    // Type variable (for generic type inference)
    TypeVar(TypeVarId),

    // Named type parameter (e.g., T in fn foo<T>)
    TypeParam(SmolStr),

    // Generic type with parameters (e.g., List<T>, Result<T, E>)
    Generic {
        name: SmolStr,
        args: Vec<MirType>,
    },
}

impl MirType {
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            MirType::Unit
                | MirType::Bool
                | MirType::Int
                | MirType::Int8
                | MirType::Int16
                | MirType::Int32
                | MirType::Int64
                | MirType::UInt
                | MirType::UInt8
                | MirType::UInt16
                | MirType::UInt32
                | MirType::UInt64
                | MirType::Float
                | MirType::Float32
                | MirType::Float64
                | MirType::Char
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            MirType::Int
                | MirType::Int8
                | MirType::Int16
                | MirType::Int32
                | MirType::Int64
                | MirType::UInt
                | MirType::UInt8
                | MirType::UInt16
                | MirType::UInt32
                | MirType::UInt64
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, MirType::Float | MirType::Float32 | MirType::Float64)
    }

    /// Check if this type contains any type variables
    pub fn has_type_vars(&self) -> bool {
        match self {
            MirType::TypeVar(_) => true,
            MirType::TypeParam(_) => true,
            MirType::Array(inner) => inner.has_type_vars(),
            MirType::Tuple(elems) => elems.iter().any(|t| t.has_type_vars()),
            MirType::Map(k, v) => k.has_type_vars() || v.has_type_vars(),
            MirType::Optional(inner) => inner.has_type_vars(),
            MirType::Result(ok, err) => ok.has_type_vars() || err.has_type_vars(),
            MirType::Ref(inner) | MirType::RefMut(inner) => inner.has_type_vars(),
            MirType::FnPtr { params, ret } | MirType::Closure { params, ret } => {
                params.iter().any(|t| t.has_type_vars()) || ret.has_type_vars()
            }
            MirType::Generic { args, .. } => args.iter().any(|t| t.has_type_vars()),
            _ => false,
        }
    }

    /// Substitute type parameters with concrete types
    pub fn substitute(&self, substitutions: &[(SmolStr, MirType)]) -> MirType {
        match self {
            MirType::TypeParam(name) => {
                for (param_name, replacement) in substitutions {
                    if param_name == name {
                        return replacement.clone();
                    }
                }
                self.clone()
            }
            MirType::Array(inner) => MirType::Array(Box::new(inner.substitute(substitutions))),
            MirType::Tuple(elems) => {
                MirType::Tuple(elems.iter().map(|t| t.substitute(substitutions)).collect())
            }
            MirType::Map(k, v) => MirType::Map(
                Box::new(k.substitute(substitutions)),
                Box::new(v.substitute(substitutions)),
            ),
            MirType::Optional(inner) => {
                MirType::Optional(Box::new(inner.substitute(substitutions)))
            }
            MirType::Result(ok, err) => MirType::Result(
                Box::new(ok.substitute(substitutions)),
                Box::new(err.substitute(substitutions)),
            ),
            MirType::Ref(inner) => MirType::Ref(Box::new(inner.substitute(substitutions))),
            MirType::RefMut(inner) => MirType::RefMut(Box::new(inner.substitute(substitutions))),
            MirType::FnPtr { params, ret } => MirType::FnPtr {
                params: params.iter().map(|t| t.substitute(substitutions)).collect(),
                ret: Box::new(ret.substitute(substitutions)),
            },
            MirType::Closure { params, ret } => MirType::Closure {
                params: params.iter().map(|t| t.substitute(substitutions)).collect(),
                ret: Box::new(ret.substitute(substitutions)),
            },
            MirType::Generic { name, args } => MirType::Generic {
                name: name.clone(),
                args: args.iter().map(|t| t.substitute(substitutions)).collect(),
            },
            _ => self.clone(),
        }
    }

    /// Collect all type variables in this type
    pub fn collect_type_vars(&self) -> Vec<TypeVarId> {
        let mut vars = Vec::new();
        self.collect_type_vars_into(&mut vars);
        vars
    }

    fn collect_type_vars_into(&self, vars: &mut Vec<TypeVarId>) {
        match self {
            MirType::TypeVar(id) => {
                if !vars.contains(id) {
                    vars.push(*id);
                }
            }
            MirType::Array(inner) => inner.collect_type_vars_into(vars),
            MirType::Tuple(elems) => {
                for elem in elems {
                    elem.collect_type_vars_into(vars);
                }
            }
            MirType::Map(k, v) => {
                k.collect_type_vars_into(vars);
                v.collect_type_vars_into(vars);
            }
            MirType::Optional(inner) => inner.collect_type_vars_into(vars),
            MirType::Result(ok, err) => {
                ok.collect_type_vars_into(vars);
                err.collect_type_vars_into(vars);
            }
            MirType::Ref(inner) | MirType::RefMut(inner) => inner.collect_type_vars_into(vars),
            MirType::FnPtr { params, ret } | MirType::Closure { params, ret } => {
                for param in params {
                    param.collect_type_vars_into(vars);
                }
                ret.collect_type_vars_into(vars);
            }
            MirType::Generic { args, .. } => {
                for arg in args {
                    arg.collect_type_vars_into(vars);
                }
            }
            _ => {}
        }
    }

    /// Check if this type implements the Copy trait (can be implicitly copied).
    ///
    /// Copy types can be duplicated by simple bitwise copying without running
    /// any user-defined code. When a Copy type is used, it is copied rather than moved.
    ///
    /// # Copy types
    ///
    /// - All numeric primitives (Int, Float, etc.)
    /// - Bool, Char, Unit, Never
    /// - References (copying the reference, not what it points to)
    /// - Tuples of Copy types
    /// - Optional of Copy types
    /// - Result of Copy types (both Ok and Err must be Copy)
    ///
    /// # Non-Copy types (owned, require move semantics)
    ///
    /// - String (owns heap-allocated data)
    /// - Array (dynamic arrays own heap memory)
    /// - Map (owns heap-allocated data)
    /// - Functions/closures (may capture owned state)
    /// - Structs/Enums (not Copy by default)
    /// - Type variables (unknown, conservatively non-Copy)
    pub fn is_copy(&self) -> bool {
        match self {
            // All numeric primitives are Copy
            MirType::Int
            | MirType::Int8
            | MirType::Int16
            | MirType::Int32
            | MirType::Int64
            | MirType::UInt
            | MirType::UInt8
            | MirType::UInt16
            | MirType::UInt32
            | MirType::UInt64
            | MirType::Float
            | MirType::Float32
            | MirType::Float64
            | MirType::Bool
            | MirType::Char
            | MirType::Unit
            | MirType::Never => true,

            // String owns heap data - NOT Copy
            MirType::String => false,

            // Dynamic arrays own heap memory - NOT Copy
            MirType::Array(_) => false,

            // Maps own heap memory - NOT Copy
            MirType::Map(_, _) => false,

            // Tuples are Copy if all elements are Copy
            MirType::Tuple(types) => types.iter().all(|t| t.is_copy()),

            // Optional is Copy if inner type is Copy
            MirType::Optional(inner) => inner.is_copy(),

            // Result is Copy if both Ok and Err types are Copy
            MirType::Result(ok, err) => ok.is_copy() && err.is_copy(),

            // References are Copy (we copy the reference itself, not the data)
            MirType::Ref(_) | MirType::RefMut(_) => true,

            // Functions and closures may capture owned state - NOT Copy
            MirType::FnPtr { .. } | MirType::Closure { .. } => false,

            // Structs and Enums are NOT Copy by default
            // TODO: Check against a registry of Copy types (e.g., @copy attribute)
            MirType::Struct(_) | MirType::Enum(_) => false,

            // Generic types need their args checked
            MirType::Generic { .. } => false,

            // Type variables and params are unknown, conservatively non-Copy
            MirType::TypeVar(_) | MirType::TypeParam(_) => false,
        }
    }
}

// ============================================================================
// Functions
// ============================================================================

/// Function linkage type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Linkage {
    /// Normal function defined in this module
    #[default]
    Local,
    /// External function (imported from runtime)
    External,
    /// Builtin function with special handling
    Builtin(BuiltinKind),
}

/// Kind of builtin function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinKind {
    // I/O builtins
    /// print(args...) - print without newline
    Print,
    /// println(args...) - print with newline
    Println,

    // Collection/String builtins
    /// len(collection) - get length
    Len,

    // Type conversion builtins
    /// type_of(value) - get type name
    TypeOf,
    /// to_string(value) - convert to string
    ToString,
    /// to_int(value) - convert to int
    ToInt,
    /// to_float(value) - convert to float
    ToFloat,

    // String builtins
    /// contains(haystack, needle) - check if string contains substring
    StringContains,
    /// starts_with(s, prefix) - check if string starts with prefix
    StringStartsWith,
    /// ends_with(s, suffix) - check if string ends with suffix
    StringEndsWith,
    /// trim(s) - remove leading/trailing whitespace
    StringTrim,
    /// split(s, delimiter) - split string by delimiter (returns array)
    StringSplit,
    /// replace(s, from, to) - replace occurrences
    StringReplace,
    /// substring(s, start, len) - get substring
    Substring,
    /// char_at(s, index) - get character at index
    CharAt,
    /// to_upper(s) - convert to uppercase
    ToUpper,
    /// to_lower(s) - convert to lowercase
    ToLower,

    // Math builtins
    /// abs(number) - absolute value
    Abs,
    /// min(a, b) - minimum
    Min,
    /// max(a, b) - maximum
    Max,
    /// sqrt(x) - square root
    Sqrt,
    /// pow(base, exp) - power
    Pow,
    /// sin(x) - sine
    Sin,
    /// cos(x) - cosine
    Cos,
    /// tan(x) - tangent
    Tan,
    /// floor(x) - floor
    Floor,
    /// ceil(x) - ceiling
    Ceil,
    /// round(x) - round to nearest
    Round,

    // Collection builtins
    /// push(array, value) - add to end
    Push,
    /// pop(array) - remove from end
    Pop,
    /// first(array) - get first element
    First,
    /// last(array) - get last element
    Last,
    /// reverse(array) - reverse in place
    Reverse,

    // Higher-order collection operations
    /// map(array, fn) - transform each element
    Map,
    /// filter(array, fn) - keep elements matching predicate
    Filter,
    /// reduce(array, fn, initial) - fold/accumulate values
    Reduce,
    /// find(array, fn) - find first matching element (returns Option)
    Find,
    /// any(array, fn) - check if any element matches
    Any,
    /// all(array, fn) - check if all elements match
    All,
    /// slice(array, start, end) - extract subarray
    Slice,
    /// concat(array1, array2) - combine arrays
    Concat,

    // Control flow builtins
    /// assert(cond, msg?) - assertion
    Assert,
    /// panic(msg) - panic
    Panic,
}

/// A MIR function
#[derive(Debug, Clone)]
pub struct MirFunction {
    /// Function name
    pub name: SmolStr,
    /// Source span
    pub span: Span,
    /// Parameters (as locals)
    pub params: Vec<Local>,
    /// Return type
    pub return_ty: MirType,
    /// All local variable declarations
    pub locals: Vec<LocalDecl>,
    /// Control flow graph (basic blocks)
    pub blocks: Vec<BasicBlock>,
    /// Whether this function is public
    pub is_public: bool,
    /// Function linkage
    pub linkage: Linkage,
    /// Effect row - effects this function may perform
    pub effect_row: EffectRow,
    /// Evidence parameters for effect handling
    pub evidence_params: Vec<EvidenceParam>,
    /// Evidence layout for this function
    pub evidence_layout: EvidenceLayout,
    /// Handler blocks defined in this function
    pub handler_blocks: Vec<HandlerBlock>,
    /// Effect statements in this function (indexed by block and statement)
    pub effect_statements: FxHashMap<(BlockId, usize), EffectStatementKind>,
    /// Effect terminators in this function (indexed by block)
    pub effect_terminators: FxHashMap<BlockId, EffectTerminatorKind>,
    /// Type parameters for generic functions (empty if not generic)
    pub type_params: Vec<SmolStr>,
    /// If this is a monomorphized instance, the original generic function
    pub generic_origin: Option<FunctionId>,
    /// If this is a monomorphized instance, the type arguments used
    pub type_args: Vec<MirType>,
    /// Function contract (preconditions and postconditions)
    pub contract: Option<FunctionContract>,
    /// Function attributes (e.g., #[inline(always)], #[inline(never)])
    pub attributes: Vec<SmolStr>,
}

impl MirFunction {
    pub fn new(name: SmolStr, return_ty: MirType, span: Span) -> Self {
        // Local 0 is always the return place
        let return_local = LocalDecl {
            name: Some("_return".into()),
            ty: return_ty.clone(),
            mutable: true,
            span: Span::dummy(),
        };

        Self {
            name,
            span,
            params: Vec::new(),
            return_ty,
            locals: vec![return_local],
            blocks: Vec::new(),
            is_public: false,
            linkage: Linkage::Local,
            effect_row: EffectRow::new(),
            evidence_params: Vec::new(),
            evidence_layout: EvidenceLayout::new(),
            handler_blocks: Vec::new(),
            effect_statements: FxHashMap::default(),
            effect_terminators: FxHashMap::default(),
            type_params: Vec::new(),
            generic_origin: None,
            type_args: Vec::new(),
            contract: None,
            attributes: Vec::new(),
        }
    }

    /// Check if this function is generic
    pub fn is_generic(&self) -> bool {
        !self.type_params.is_empty()
    }

    /// Check if this function is a monomorphized instance
    pub fn is_monomorphized(&self) -> bool {
        self.generic_origin.is_some()
    }

    /// Create a new local variable and return its index
    pub fn new_local(&mut self, ty: MirType, name: Option<SmolStr>) -> Local {
        let local = Local(self.locals.len() as u32);
        self.locals.push(LocalDecl {
            name,
            ty,
            mutable: true,
            span: Span::dummy(),
        });
        local
    }

    /// Create a new basic block and return its ID
    pub fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        self.blocks.push(BasicBlock::new(id));
        id
    }

    /// Get a mutable reference to a block
    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id.0 as usize]
    }

    /// Get a reference to a block
    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id.0 as usize]
    }

    /// Get local declaration
    pub fn local_decl(&self, local: Local) -> &LocalDecl {
        &self.locals[local.0 as usize]
    }

    /// Set the effect row for this function
    pub fn set_effect_row(&mut self, row: EffectRow) {
        self.effect_row = row;
    }

    /// Add an evidence parameter for an effect
    pub fn add_evidence_param(&mut self, effect: EffectType, is_static: bool) -> Local {
        let local = self.new_local(
            MirType::UInt64, // Evidence is a pointer
            Some(format!("_ev_{}", effect.name).into()),
        );
        self.evidence_params.push(EvidenceParam {
            local,
            effect: effect.clone(),
            is_static,
        });
        // Also add to evidence layout
        self.evidence_layout.add_effect(effect.id);
        local
    }

    /// Get evidence slot for an effect
    pub fn evidence_slot_for(&self, effect_id: EffectId) -> Option<EvidenceSlot> {
        self.evidence_layout
            .slot_for(effect_id)
            .map(EvidenceSlot::Static)
    }

    /// Add an effect statement to a block
    pub fn add_effect_statement(
        &mut self,
        block: BlockId,
        stmt_index: usize,
        kind: EffectStatementKind,
    ) {
        self.effect_statements.insert((block, stmt_index), kind);
    }

    /// Get effect statement for a block and index
    pub fn effect_statement(&self, block: BlockId, stmt_index: usize) -> Option<&EffectStatementKind> {
        self.effect_statements.get(&(block, stmt_index))
    }

    /// Set effect terminator for a block
    pub fn set_effect_terminator(&mut self, block: BlockId, kind: EffectTerminatorKind) {
        self.effect_terminators.insert(block, kind);
    }

    /// Get effect terminator for a block
    pub fn effect_terminator(&self, block: BlockId) -> Option<&EffectTerminatorKind> {
        self.effect_terminators.get(&block)
    }

    /// Add a handler block to this function
    pub fn add_handler_block(&mut self, handler_block: HandlerBlock) {
        self.handler_blocks.push(handler_block);
    }

    /// Check if this function is pure (no effects)
    pub fn is_pure(&self) -> bool {
        self.effect_row.is_pure()
    }

    /// Check if this function has evidence parameters
    pub fn has_evidence(&self) -> bool {
        !self.evidence_params.is_empty()
    }
}

/// Local variable declaration
#[derive(Debug, Clone)]
pub struct LocalDecl {
    /// Optional debug name
    pub name: Option<SmolStr>,
    /// Type of the local
    pub ty: MirType,
    /// Whether this local is mutable
    pub mutable: bool,
    /// Source span
    pub span: Span,
}

// ============================================================================
// Basic Blocks
// ============================================================================

/// A basic block in the CFG
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Block identifier
    pub id: BlockId,
    /// Statements (no control flow)
    pub statements: Vec<Statement>,
    /// Terminator (control flow)
    pub terminator: Option<Terminator>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            statements: Vec::new(),
            terminator: None,
        }
    }

    /// Add a statement to this block
    pub fn push_stmt(&mut self, stmt: Statement) {
        self.statements.push(stmt);
    }

    /// Set the terminator for this block
    pub fn set_terminator(&mut self, term: Terminator) {
        self.terminator = Some(term);
    }
}

// ============================================================================
// Statements
// ============================================================================

/// A statement (no control flow transfer)
#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

/// Statement kinds
#[derive(Debug, Clone)]
pub enum StatementKind {
    /// Assignment: place = rvalue
    Assign(Place, Rvalue),

    /// Mark storage as live (variable enters scope)
    StorageLive(Local),

    /// Mark storage as dead (variable leaves scope)
    StorageDead(Local),

    /// No-op (placeholder)
    Nop,
}

// ============================================================================
// Terminators
// ============================================================================

/// A terminator (control flow transfer)
#[derive(Debug, Clone)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Span,
}

/// Terminator kinds
#[derive(Debug, Clone)]
pub enum TerminatorKind {
    /// Unconditional jump
    Goto { target: BlockId },

    /// Conditional branch on integer value
    SwitchInt {
        discr: Operand,
        targets: SwitchTargets,
    },

    /// Function call
    Call {
        func: Operand,
        args: Vec<Operand>,
        dest: Place,
        target: Option<BlockId>, // None for diverging calls
    },

    /// Return from function
    Return,

    /// Unreachable code (e.g., after infinite loop)
    Unreachable,

    /// Drop a value (run destructor)
    Drop {
        place: Place,
        target: BlockId,
    },

    /// Assertion (for contracts)
    Assert {
        cond: Operand,
        expected: bool,
        msg: SmolStr,
        target: BlockId,
    },
}

/// Switch targets for SwitchInt
#[derive(Debug, Clone)]
pub struct SwitchTargets {
    /// (value, target) pairs
    pub targets: Vec<(i128, BlockId)>,
    /// Default target (if no value matches)
    pub otherwise: BlockId,
}

impl SwitchTargets {
    pub fn new(targets: Vec<(i128, BlockId)>, otherwise: BlockId) -> Self {
        Self { targets, otherwise }
    }

    /// Create a simple if/else switch (0 = false branch, otherwise = true branch)
    pub fn if_else(true_block: BlockId, false_block: BlockId) -> Self {
        Self {
            targets: vec![(0, false_block)],
            otherwise: true_block,
        }
    }
}

// ============================================================================
// Places (Memory Locations)
// ============================================================================

/// A place in memory (lvalue)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub local: Local,
    pub projection: Vec<PlaceElem>,
}

impl Place {
    /// Create a place from just a local
    pub fn from_local(local: Local) -> Self {
        Self {
            local,
            projection: Vec::new(),
        }
    }

    /// Create the return place
    pub fn return_place() -> Self {
        Self::from_local(Local::RETURN)
    }

    /// Add a field projection
    pub fn field(mut self, field: u32) -> Self {
        self.projection.push(PlaceElem::Field(field));
        self
    }

    /// Add an index projection
    pub fn index(mut self, index: Local) -> Self {
        self.projection.push(PlaceElem::Index(index));
        self
    }

    /// Add a deref projection
    pub fn deref(mut self) -> Self {
        self.projection.push(PlaceElem::Deref);
        self
    }

    /// Add a downcast projection (for enum variant access)
    pub fn downcast(mut self, variant: u32) -> Self {
        self.projection.push(PlaceElem::Downcast(variant));
        self
    }
}

impl From<Local> for Place {
    fn from(local: Local) -> Self {
        Self::from_local(local)
    }
}

/// Place projection element
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlaceElem {
    /// Field access: .field_index
    Field(u32),
    /// Array/tuple index: [index_local]
    Index(Local),
    /// Constant index: [constant]
    ConstantIndex(u64),
    /// Dereference: *place
    Deref,
    /// Downcast to enum variant
    Downcast(u32),
}

// ============================================================================
// Rvalues (Computed Values)
// ============================================================================

/// An rvalue (computed value)
#[derive(Debug, Clone)]
pub enum Rvalue {
    /// Use an operand directly
    Use(Operand),

    /// Binary operation
    BinaryOp(BinOp, Operand, Operand),

    /// Unary operation
    UnaryOp(UnOp, Operand),

    /// Create a reference
    Ref(Place),

    /// Create a mutable reference
    RefMut(Place),

    /// Create an aggregate (struct, tuple, array)
    Aggregate(AggregateKind, Vec<Operand>),

    /// Get discriminant of enum
    Discriminant(Place),

    /// Get length of array/string
    Len(Place),

    /// Cast between types
    Cast(CastKind, Operand, MirType),

    /// Closure creation
    Closure(FunctionId, Vec<Operand>),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Rem,
    Pow,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical (for short-circuit evaluation at MIR level, these become control flow)
    // These are kept for cases where both operands are already evaluated
    And,
    Or,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Arithmetic negation
    Neg,
    /// Logical not
    Not,
    /// Bitwise not
    BitNot,
}

/// Aggregate kinds
#[derive(Debug, Clone)]
pub enum AggregateKind {
    /// Tuple
    Tuple,
    /// Array
    Array(MirType),
    /// Struct
    Struct(StructId),
    /// Enum variant
    Enum(EnumId, u32), // enum id + variant index
}

/// Cast kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    /// Integer to integer (widening/narrowing)
    IntToInt,
    /// Float to float
    FloatToFloat,
    /// Integer to float
    IntToFloat,
    /// Float to integer
    FloatToInt,
    /// Pointer cast
    PtrToPtr,
}

// ============================================================================
// Operands (Values)
// ============================================================================

/// An operand (value that can be used)
#[derive(Debug, Clone)]
pub enum Operand {
    /// Copy from a place
    Copy(Place),
    /// Move from a place
    Move(Place),
    /// Constant value
    Constant(Constant),
}

impl Operand {
    pub fn const_int(value: i64) -> Self {
        Operand::Constant(Constant::Int(value))
    }

    pub fn const_bool(value: bool) -> Self {
        Operand::Constant(Constant::Bool(value))
    }

    pub fn const_float(value: f64) -> Self {
        Operand::Constant(Constant::Float(value))
    }

    pub fn const_float32(value: f32) -> Self {
        Operand::Constant(Constant::Float32(value))
    }

    pub fn const_float64(value: f64) -> Self {
        Operand::Constant(Constant::Float64(value))
    }

    pub fn const_unit() -> Self {
        Operand::Constant(Constant::Unit)
    }
}

/// Constant values
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),      // Default float (f64)
    Float32(f32),    // Explicit f32
    Float64(f64),    // Explicit f64
    Char(char),
    String(u32), // Index into string table
    Function(FunctionId),
}

// ============================================================================
// Contracts
// ============================================================================

/// Function contract (preconditions and postconditions)
#[derive(Debug, Clone)]
pub struct FunctionContract {
    /// Preconditions (requires clauses)
    pub requires: Vec<ContractClause>,
    /// Postconditions (ensures clauses)
    pub ensures: Vec<ContractClause>,
}

impl FunctionContract {
    pub fn new() -> Self {
        Self {
            requires: Vec::new(),
            ensures: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.requires.is_empty() && self.ensures.is_empty()
    }
}

impl Default for FunctionContract {
    fn default() -> Self {
        Self::new()
    }
}

/// A single contract clause (condition + optional message)
#[derive(Debug, Clone)]
pub struct ContractClause {
    /// The condition expression to verify
    pub condition: Expr,
    /// Optional custom error message
    pub message: Option<String>,
    /// Source span for error reporting
    pub span: Span,
}

/// Struct invariants
#[derive(Debug, Clone)]
pub struct StructInvariant {
    /// Invariant conditions that must hold after construction and mutation
    pub conditions: Vec<ContractClause>,
}

impl StructInvariant {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }
}

impl Default for StructInvariant {
    fn default() -> Self {
        Self::new()
    }
}

/// MIR expression for contracts
///
/// This is a simplified expression type used in contract conditions.
/// It's similar to the full MIR expression system but focused on
/// compile-time verifiable conditions.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Constant boolean
    Bool(bool),
    /// Constant integer
    Int(i64),
    /// Constant float
    Float(f64),
    /// Local variable reference
    Local(Local),
    /// Binary operation
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Unary operation
    Unary {
        op: UnOp,
        operand: Box<Expr>,
    },
    /// Field access
    Field {
        object: Box<Expr>,
        field: u32,
    },
    /// Method call (for pure methods like is_valid())
    MethodCall {
        object: Box<Expr>,
        method: SmolStr,
        args: Vec<Expr>,
    },
    /// Old value reference (for postconditions)
    Old(Box<Expr>),
    /// Return value reference (for postconditions)
    Result,
}

// ============================================================================
// Struct and Enum Definitions
// ============================================================================

/// Struct definition
#[derive(Debug, Clone)]
pub struct MirStruct {
    pub name: SmolStr,
    pub fields: Vec<MirField>,
    pub span: Span,
    /// Struct invariants
    pub invariants: Option<StructInvariant>,
}

/// Struct field
#[derive(Debug, Clone)]
pub struct MirField {
    pub name: SmolStr,
    pub ty: MirType,
}

/// Enum definition
#[derive(Debug, Clone)]
pub struct MirEnum {
    pub name: SmolStr,
    pub variants: Vec<MirVariant>,
    pub span: Span,
}

/// Enum variant
#[derive(Debug, Clone)]
pub struct MirVariant {
    pub name: SmolStr,
    pub fields: Vec<MirType>,
}

// ============================================================================
// Display implementations for debugging
// ============================================================================

impl fmt::Display for Local {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_{}", self.0)
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.local)?;
        for elem in &self.projection {
            match elem {
                PlaceElem::Field(idx) => write!(f, ".{}", idx)?,
                PlaceElem::Index(local) => write!(f, "[{}]", local)?,
                PlaceElem::ConstantIndex(idx) => write!(f, "[{}]", idx)?,
                PlaceElem::Deref => write!(f, ".*")?,
                PlaceElem::Downcast(variant) => write!(f, " as variant#{}", variant)?,
            }
        }
        Ok(())
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Copy(place) => write!(f, "copy {}", place),
            Operand::Move(place) => write!(f, "move {}", place),
            Operand::Constant(c) => write!(f, "{}", c),
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Unit => write!(f, "()"),
            Constant::Bool(b) => write!(f, "{}", b),
            Constant::Int(i) => write!(f, "{}", i),
            Constant::Float(fl) => write!(f, "{}", fl),
            Constant::Float32(fl) => write!(f, "{}f32", fl),
            Constant::Float64(fl) => write!(f, "{}f64", fl),
            Constant::Char(c) => write!(f, "'{}'", c),
            Constant::String(idx) => write!(f, "str#{}", idx),
            Constant::Function(id) => write!(f, "fn#{}", id.0),
        }
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::IntDiv => "//",
            BinOp::Rem => "%",
            BinOp::Pow => "**",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnOp::Neg => "-",
            UnOp::Not => "!",
            UnOp::BitNot => "~",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for MirType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirType::Unit => write!(f, "()"),
            MirType::Bool => write!(f, "Bool"),
            MirType::Int => write!(f, "Int"),
            MirType::Int8 => write!(f, "Int8"),
            MirType::Int16 => write!(f, "Int16"),
            MirType::Int32 => write!(f, "Int32"),
            MirType::Int64 => write!(f, "Int64"),
            MirType::UInt => write!(f, "UInt"),
            MirType::UInt8 => write!(f, "UInt8"),
            MirType::UInt16 => write!(f, "UInt16"),
            MirType::UInt32 => write!(f, "UInt32"),
            MirType::UInt64 => write!(f, "UInt64"),
            MirType::Float => write!(f, "Float"),
            MirType::Float32 => write!(f, "Float32"),
            MirType::Float64 => write!(f, "Float64"),
            MirType::Char => write!(f, "Char"),
            MirType::String => write!(f, "String"),
            MirType::Array(t) => write!(f, "[{}]", t),
            MirType::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            MirType::Map(k, v) => write!(f, "{{{}:{}}}", k, v),
            MirType::Optional(t) => write!(f, "{}?", t),
            MirType::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            MirType::Ref(t) => write!(f, "&{}", t),
            MirType::RefMut(t) => write!(f, "&mut {}", t),
            MirType::Struct(id) => write!(f, "struct#{}", id.0),
            MirType::Enum(id) => write!(f, "enum#{}", id.0),
            MirType::FnPtr { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            MirType::Closure { params, ret } => {
                write!(f, "closure(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            MirType::Never => write!(f, "!"),
            MirType::TypeVar(id) => write!(f, "{}", id),
            MirType::TypeParam(name) => write!(f, "{}", name),
            MirType::Generic { name, args } => {
                write!(f, "{}<", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
        }
    }
}

// ============================================================================
// Effect System Types
// ============================================================================

/// Unique identifier for an effect type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(pub u32);

/// Unique identifier for an operation within an effect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(pub u32);

/// Unique identifier for a handler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerId(pub u32);

/// Unique identifier for a continuation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContinuationId(pub u32);

/// Effect type representation in MIR
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectType {
    /// Effect identifier
    pub id: EffectId,
    /// Effect name (for debugging/printing)
    pub name: SmolStr,
    /// Type parameters (if generic)
    pub type_params: Vec<MirType>,
}

impl EffectType {
    pub fn new(id: EffectId, name: SmolStr) -> Self {
        Self {
            id,
            name,
            type_params: Vec::new(),
        }
    }

    pub fn with_type_params(mut self, params: Vec<MirType>) -> Self {
        self.type_params = params;
        self
    }
}

/// Effect row - a set of effects that a function may perform
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectRow {
    /// Effects in this row
    pub effects: Vec<EffectType>,
    /// Whether this row is open (can have additional effects)
    pub is_open: bool,
}

impl EffectRow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pure() -> Self {
        Self {
            effects: Vec::new(),
            is_open: false,
        }
    }

    pub fn with_effect(mut self, effect: EffectType) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn open(mut self) -> Self {
        self.is_open = true;
        self
    }

    pub fn is_pure(&self) -> bool {
        self.effects.is_empty() && !self.is_open
    }
}

/// Evidence slot representation for handler lookup
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceSlot {
    /// Compile-time known offset into evidence vector
    Static(u32),
    /// Runtime lookup required (effect polymorphism)
    Dynamic(Local),
}

/// Effect operation classification (drives codegen strategy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EffectClassification {
    /// Handler in tail-resumptive position - direct call
    TailResumptive,
    /// Handler may capture continuation
    #[default]
    General,
    /// Effect crosses FFI boundary - requires barrier
    FfiBoundary,
}

/// FFI barrier strategy for effect boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiBarrierStrategy {
    /// Compile error if effect would cross barrier
    Prohibit,
    /// Convert continuation to callback
    CallbackConvert,
    /// Save/restore handler state around FFI
    HandlerSaveRestore,
}

// ============================================================================
// Effect Operations in MIR
// ============================================================================

/// Effect operation definition within an effect
#[derive(Debug, Clone)]
pub struct EffectOperation {
    /// Operation identifier
    pub id: OperationId,
    /// Operation name
    pub name: SmolStr,
    /// Parameter types
    pub params: Vec<MirType>,
    /// Return type
    pub return_ty: MirType,
}

/// Effect definition in MIR
#[derive(Debug, Clone)]
pub struct MirEffect {
    /// Effect identifier
    pub id: EffectId,
    /// Effect name
    pub name: SmolStr,
    /// Type parameters
    pub type_params: Vec<SmolStr>,
    /// Operations defined by this effect
    pub operations: Vec<EffectOperation>,
    /// Source span
    pub span: Span,
}

impl MirEffect {
    pub fn new(id: EffectId, name: SmolStr, span: Span) -> Self {
        Self {
            id,
            name,
            type_params: Vec::new(),
            operations: Vec::new(),
            span,
        }
    }

    /// Add an operation to this effect
    pub fn add_operation(&mut self, op: EffectOperation) -> OperationId {
        let id = OperationId(self.operations.len() as u32);
        self.operations.push(op);
        id
    }

    /// Get operation by ID
    pub fn operation(&self, id: OperationId) -> Option<&EffectOperation> {
        self.operations.get(id.0 as usize)
    }
}

/// Effect handler definition in MIR
#[derive(Debug, Clone)]
pub struct MirHandler {
    /// Handler identifier
    pub id: HandlerId,
    /// Effect being handled
    pub effect: EffectType,
    /// Operation implementations (block IDs for each operation)
    pub operation_blocks: Vec<BlockId>,
    /// Return handler block (for values returned from handled computation)
    pub return_block: Option<BlockId>,
    /// Whether this handler is tail-resumptive
    pub is_tail_resumptive: bool,
    /// Source span
    pub span: Span,
}

impl MirHandler {
    pub fn new(id: HandlerId, effect: EffectType, span: Span) -> Self {
        Self {
            id,
            effect,
            operation_blocks: Vec::new(),
            return_block: None,
            is_tail_resumptive: true, // Default to tail-resumptive, analysis may change this
            span,
        }
    }
}

/// Handler block - a special block that handles an effect operation
#[derive(Debug, Clone)]
pub struct HandlerBlock {
    /// Block ID
    pub block_id: BlockId,
    /// Effect being handled
    pub effect: EffectType,
    /// Operation being handled
    pub operation: OperationId,
    /// Parameter locals (received from perform)
    pub params: Vec<Local>,
    /// Continuation local (if non-tail-resumptive)
    pub continuation: Option<Local>,
    /// Resume block ID (where to continue after resume)
    pub resume_block: Option<BlockId>,
}

// ============================================================================
// Effect-Related Statement Kinds
// ============================================================================

/// Extended statement kinds to include effect operations
#[derive(Debug, Clone)]
pub enum EffectStatementKind {
    /// Install a handler for an effect
    /// effect.install %handler_ref, %evidence_slot, @Effect
    InstallHandler {
        /// Handler reference
        handler: HandlerId,
        /// Evidence slot to install into
        evidence_slot: EvidenceSlot,
        /// Effect being handled
        effect: EffectType,
        /// Previous evidence (for restoration)
        prev_evidence: Option<Local>,
    },

    /// Uninstall a handler (restore previous evidence)
    UninstallHandler {
        /// Evidence slot to restore
        evidence_slot: EvidenceSlot,
        /// Previous evidence to restore
        prev_evidence: Local,
    },

    /// Perform an effect operation (tail-resumptive)
    /// effect.perform.tail @Effect.operation, args*, %evidence_slot -> %result
    PerformEffect {
        /// Effect being performed
        effect: EffectType,
        /// Operation within the effect
        operation: OperationId,
        /// Arguments to the operation
        args: Vec<Operand>,
        /// Evidence slot for handler lookup
        evidence_slot: EvidenceSlot,
        /// Result destination
        dest: Place,
        /// Classification (determined by analysis)
        classification: EffectClassification,
    },

    /// Capture current continuation
    /// effect.capture -> %continuation
    CaptureContunuation {
        /// Destination for continuation
        dest: Place,
    },

    /// Clone a continuation (for multi-shot handlers)
    CloneContinuation {
        /// Source continuation
        source: Operand,
        /// Destination for cloned continuation
        dest: Place,
    },

    /// FFI boundary marker
    FfiBarrier {
        /// Strategy for handling effects at this barrier
        strategy: FfiBarrierStrategy,
        /// Effects blocked by this barrier
        blocked_effects: Vec<EffectType>,
    },
}

// ============================================================================
// Effect-Related Terminator Kinds
// ============================================================================

/// Extended terminator kinds to include effect control flow
#[derive(Debug, Clone)]
pub enum EffectTerminatorKind {
    /// Yield to handler with a value (for non-tail-resumptive)
    /// effect.yield @Effect.operation, args*, %continuation
    Yield {
        /// Effect being performed
        effect: EffectType,
        /// Operation within the effect
        operation: OperationId,
        /// Arguments to the operation
        args: Vec<Operand>,
        /// Continuation to pass to handler
        continuation: Operand,
        /// Handler block to jump to
        handler_block: BlockId,
    },

    /// Resume a continuation with a value
    /// effect.resume %continuation, %value
    Resume {
        /// Continuation to resume
        continuation: Operand,
        /// Value to pass to continuation
        value: Operand,
        /// Target block (where continuation resumes)
        target: BlockId,
    },

    /// Handle expression with handler blocks
    Handle {
        /// Body block (computation being handled)
        body: BlockId,
        /// Handler definitions
        handler: HandlerId,
        /// Normal return block (if body returns without effect)
        normal_return: BlockId,
        /// Effect return block (if handler completes)
        effect_return: BlockId,
    },
}

// ============================================================================
// Evidence Vector
// ============================================================================

/// Evidence vector layout for a function
#[derive(Debug, Clone, Default)]
pub struct EvidenceLayout {
    /// Effect -> slot offset mapping
    pub slots: FxHashMap<EffectId, u32>,
    /// Total number of slots
    pub size: u32,
}

impl EvidenceLayout {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an effect to the layout and return its slot
    pub fn add_effect(&mut self, effect_id: EffectId) -> u32 {
        if let Some(&slot) = self.slots.get(&effect_id) {
            slot
        } else {
            let slot = self.size;
            self.slots.insert(effect_id, slot);
            self.size += 1;
            slot
        }
    }

    /// Get the slot for an effect
    pub fn slot_for(&self, effect_id: EffectId) -> Option<u32> {
        self.slots.get(&effect_id).copied()
    }
}

/// Evidence parameter in a function signature
#[derive(Debug, Clone)]
pub struct EvidenceParam {
    /// Local variable holding the evidence pointer
    pub local: Local,
    /// Effect this evidence is for
    pub effect: EffectType,
    /// Whether this is a static or dynamic evidence
    pub is_static: bool,
}

// ============================================================================
// Display implementations for effect types
// ============================================================================

impl fmt::Display for EffectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "effect#{}", self.0)
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "op#{}", self.0)
    }
}

impl fmt::Display for HandlerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "handler#{}", self.0)
    }
}

impl fmt::Display for ContinuationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cont#{}", self.0)
    }
}

impl fmt::Display for EffectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.type_params.is_empty() {
            write!(f, "[")?;
            for (i, ty) in self.type_params.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", ty)?;
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

impl fmt::Display for EffectRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, effect) in self.effects.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", effect)?;
        }
        if self.is_open {
            if !self.effects.is_empty() {
                write!(f, ", ")?;
            }
            write!(f, "..")?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for EvidenceSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceSlot::Static(offset) => write!(f, "ev[{}]", offset),
            EvidenceSlot::Dynamic(local) => write!(f, "ev[{}]", local),
        }
    }
}

impl fmt::Display for EffectClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectClassification::TailResumptive => write!(f, "tail-resumptive"),
            EffectClassification::General => write!(f, "general"),
            EffectClassification::FfiBoundary => write!(f, "ffi-boundary"),
        }
    }
}

impl fmt::Display for FfiBarrierStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FfiBarrierStrategy::Prohibit => write!(f, "prohibit"),
            FfiBarrierStrategy::CallbackConvert => write!(f, "callback-convert"),
            FfiBarrierStrategy::HandlerSaveRestore => write!(f, "save-restore"),
        }
    }
}

impl fmt::Display for EffectStatementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectStatementKind::InstallHandler {
                handler,
                evidence_slot,
                effect,
                prev_evidence,
            } => {
                write!(f, "effect.install {}, {}, @{}", handler, evidence_slot, effect)?;
                if let Some(prev) = prev_evidence {
                    write!(f, " (prev: {})", prev)?;
                }
                Ok(())
            }
            EffectStatementKind::UninstallHandler {
                evidence_slot,
                prev_evidence,
            } => {
                write!(f, "effect.uninstall {}, prev: {}", evidence_slot, prev_evidence)
            }
            EffectStatementKind::PerformEffect {
                effect,
                operation,
                args,
                evidence_slot,
                dest,
                classification,
            } => {
                let args_str: Vec<_> = args.iter().map(|a| format!("{}", a)).collect();
                write!(
                    f,
                    "{} = effect.perform.{} @{}.{}, [{}], {}",
                    dest,
                    classification,
                    effect,
                    operation,
                    args_str.join(", "),
                    evidence_slot
                )
            }
            EffectStatementKind::CaptureContunuation { dest } => {
                write!(f, "{} = effect.capture", dest)
            }
            EffectStatementKind::CloneContinuation { source, dest } => {
                write!(f, "{} = continuation.clone {}", dest, source)
            }
            EffectStatementKind::FfiBarrier {
                strategy,
                blocked_effects,
            } => {
                let effects_str: Vec<_> = blocked_effects.iter().map(|e| format!("{}", e)).collect();
                write!(
                    f,
                    "effect.barrier @ffi, strategy: {}, blocked: [{}]",
                    strategy,
                    effects_str.join(", ")
                )
            }
        }
    }
}

impl fmt::Display for EffectTerminatorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectTerminatorKind::Yield {
                effect,
                operation,
                args,
                continuation,
                handler_block,
            } => {
                let args_str: Vec<_> = args.iter().map(|a| format!("{}", a)).collect();
                write!(
                    f,
                    "effect.yield @{}.{}, [{}], {}, -> {}",
                    effect,
                    operation,
                    args_str.join(", "),
                    continuation,
                    handler_block
                )
            }
            EffectTerminatorKind::Resume {
                continuation,
                value,
                target,
            } => {
                write!(f, "effect.resume {}, {} -> {}", continuation, value, target)
            }
            EffectTerminatorKind::Handle {
                body,
                handler,
                normal_return,
                effect_return,
            } => {
                write!(
                    f,
                    "handle {} with {} -> normal: {}, effect: {}",
                    body, handler, normal_return, effect_return
                )
            }
        }
    }
}

// ============================================================================
// Monomorphization Helpers
// ============================================================================

/// Substitute type parameters in a statement
pub fn substitute_types_in_stmt(stmt: &mut Statement, substitutions: &[(SmolStr, MirType)]) {
    match &mut stmt.kind {
        StatementKind::Assign(_, rvalue) => {
            substitute_types_in_rvalue(rvalue, substitutions);
        }
        StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
    }
}

/// Substitute type parameters in an rvalue
fn substitute_types_in_rvalue(rvalue: &mut Rvalue, substitutions: &[(SmolStr, MirType)]) {
    match rvalue {
        Rvalue::Use(op) => substitute_types_in_operand(op, substitutions),
        Rvalue::BinaryOp(_, op1, op2) => {
            substitute_types_in_operand(op1, substitutions);
            substitute_types_in_operand(op2, substitutions);
        }
        Rvalue::UnaryOp(_, op) => substitute_types_in_operand(op, substitutions),
        Rvalue::Ref(_) | Rvalue::RefMut(_) => {}
        Rvalue::Aggregate(kind, ops) => {
            match kind {
                AggregateKind::Array(ty) => {
                    *ty = ty.substitute(substitutions);
                }
                _ => {}
            }
            for op in ops {
                substitute_types_in_operand(op, substitutions);
            }
        }
        Rvalue::Discriminant(_) | Rvalue::Len(_) => {}
        Rvalue::Cast(_, op, ty) => {
            substitute_types_in_operand(op, substitutions);
            *ty = ty.substitute(substitutions);
        }
        Rvalue::Closure(_, ops) => {
            for op in ops {
                substitute_types_in_operand(op, substitutions);
            }
        }
    }
}

/// Substitute type parameters in an operand
fn substitute_types_in_operand(operand: &mut Operand, substitutions: &[(SmolStr, MirType)]) {
    match operand {
        Operand::Copy(_) | Operand::Move(_) => {}
        Operand::Constant(constant) => {
            substitute_types_in_constant(constant, substitutions);
        }
    }
}

/// Substitute type parameters in a constant
fn substitute_types_in_constant(constant: &mut Constant, _substitutions: &[(SmolStr, MirType)]) {
    // Most constants don't have type information to substitute
    // Function references might need handling in the future for monomorphization
    match constant {
        Constant::Function(_fn_id) => {
            // TODO: If the function reference is to a generic function,
            // we might need to monomorphize it
        }
        _ => {}
    }
}

/// Substitute type parameters in a terminator
pub fn substitute_types_in_terminator(
    terminator: &mut Option<Terminator>,
    substitutions: &[(SmolStr, MirType)],
) {
    if let Some(term) = terminator {
        match &mut term.kind {
            TerminatorKind::Goto { .. } => {}
            TerminatorKind::SwitchInt { discr, .. } => {
                substitute_types_in_operand(discr, substitutions);
            }
            TerminatorKind::Call { func, args, .. } => {
                substitute_types_in_operand(func, substitutions);
                for arg in args {
                    substitute_types_in_operand(arg, substitutions);
                }
            }
            TerminatorKind::Return | TerminatorKind::Unreachable => {}
            TerminatorKind::Drop { .. } => {}
            TerminatorKind::Assert { cond, .. } => {
                substitute_types_in_operand(cond, substitutions);
            }
        }
    }
}
