//! Aria Language Effect System
//!
//! This module implements Aria's row-polymorphic algebraic effect system as specified
//! in ARIA-PD-005. The effect system tracks computational effects (IO, async, mutation,
//! exceptions) in the type system while maintaining Aria's core philosophy:
//! "Write code like Ruby/Python, get safety like Rust."
//!
//! ## Architecture
//!
//! The effect system consists of three layers:
//!
//! 1. **Effect Types (Static Analysis)**
//!    - Effects tracked in function signatures
//!    - Row polymorphism for flexible effect composition
//!    - Principal type inference (no annotation required)
//!
//! 2. **Effect Handlers (Runtime Semantics)**
//!    - Define custom effect interpretations
//!    - Scoped to handler blocks
//!    - First-class, composable
//!
//! 3. **Compilation Strategy (Performance)**
//!    - Evidence-passing for tail-resumptive effects (zero overhead)
//!    - Selective CPS for resumption-requiring effects
//!    - Direct style preserved where possible
//!
//! ## Effect Categories
//!
//! | Category | Effects | Compile Strategy | Runtime Cost |
//! |----------|---------|------------------|--------------|
//! | Tail-Resumptive | IO, State, Reader, Console | Evidence-passing | Zero |
//! | One-Shot | Async, Exception, Yield | Fiber/CPS hybrid | Low |
//! | Multi-Shot | Amb, Backtrack, Logic | Full CPS | Moderate |
//! | Pure | None | Direct compilation | Zero |
//!
//! ## Example
//!
//! ```aria
//! # Effect declaration
//! effect Console
//!   fn print(message: String) -> Unit
//!   fn read_line() -> String
//! end
//!
//! # Function with inferred effects
//! fn greet(name: String) -> Unit
//!   Console.print("Hello, #{name}!")  # Inferred: !Console
//! end
//!
//! # Effect polymorphism
//! fn map_with_effect[T, U, E](items: Array[T], f: Fn(T) -> U !E) -> Array[U] !E
//!   items.map(f)
//! end
//! ```

use aria_ast::Span;
use indexmap::IndexSet;
use rustc_hash::FxHashMap;
use std::fmt;
use thiserror::Error;

// ============================================================================
// Effect Variables
// ============================================================================

/// Effect row variable ID for inference
///
/// Effect variables are used during type inference to represent unknown
/// effect sets that will be resolved later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectVar(pub u32);

impl fmt::Display for EffectVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use lowercase letters for effect variables (e, e1, e2, etc.)
        if self.0 < 26 {
            write!(f, "{}", (b'e' + (self.0 % 26) as u8) as char)
        } else {
            write!(f, "e{}", self.0)
        }
    }
}

// ============================================================================
// Effect Kinds
// ============================================================================

/// Classification of effects for compilation strategy selection
///
/// The effect kind determines how the effect is compiled:
/// - TailResumptive: Evidence-passing (zero overhead)
/// - OneShot: Fiber-based or selective CPS (low overhead)
/// - MultiShot: Full CPS transformation (moderate overhead)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectKind {
    /// Tail-resumptive effects: resume is always called in tail position
    ///
    /// Examples: IO, State, Reader, Console
    /// Compilation: Evidence-passing (zero runtime overhead)
    TailResumptive,

    /// One-shot effects: resume is called at most once
    ///
    /// Examples: Async, Exception, Yield
    /// Compilation: Fiber suspension or selective CPS
    OneShot,

    /// Multi-shot effects: resume may be called multiple times
    ///
    /// Examples: Amb, Backtrack, Logic programming effects
    /// Compilation: Full CPS transformation
    MultiShot,
}

impl EffectKind {
    /// Returns the compilation strategy for this effect kind
    pub fn compilation_strategy(&self) -> CompilationStrategy {
        match self {
            EffectKind::TailResumptive => CompilationStrategy::EvidencePassing,
            EffectKind::OneShot => CompilationStrategy::FiberCpsHybrid,
            EffectKind::MultiShot => CompilationStrategy::FullCps,
        }
    }

    /// Returns true if this effect kind has zero runtime overhead
    pub fn is_zero_cost(&self) -> bool {
        matches!(self, EffectKind::TailResumptive)
    }
}

impl Default for EffectKind {
    fn default() -> Self {
        // Default to tail-resumptive (most common case)
        EffectKind::TailResumptive
    }
}

/// Compilation strategy for effects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationStrategy {
    /// Evidence-passing: pass effect implementation as hidden parameter
    EvidencePassing,
    /// Fiber/CPS hybrid: use fibers for suspension, CPS where needed
    FiberCpsHybrid,
    /// Full CPS transformation: convert to continuation-passing style
    FullCps,
}

// ============================================================================
// Effect Types
// ============================================================================

/// An individual effect in the effect system
///
/// Effects are named types that can optionally be parameterized by type arguments.
/// Examples: `IO`, `Console`, `State[Int]`, `Exception[HttpError]`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Effect {
    /// Effect name (e.g., "IO", "Console", "State")
    pub name: String,
    /// Type arguments for parameterized effects (e.g., `State[Int]` has `[Int]`)
    pub type_args: Vec<EffectTypeArg>,
    /// Effect kind classification
    pub kind: EffectKind,
}

impl Effect {
    /// Create a new simple effect with no type arguments
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_args: Vec::new(),
            kind: EffectKind::default(),
        }
    }

    /// Create a new simple effect with a specified kind
    pub fn with_kind(name: impl Into<String>, kind: EffectKind) -> Self {
        Self {
            name: name.into(),
            type_args: Vec::new(),
            kind,
        }
    }

    /// Create a parameterized effect
    pub fn parameterized(name: impl Into<String>, type_args: Vec<EffectTypeArg>) -> Self {
        Self {
            name: name.into(),
            type_args,
            kind: EffectKind::default(),
        }
    }

    /// Create a parameterized effect with a specified kind
    pub fn parameterized_with_kind(
        name: impl Into<String>,
        type_args: Vec<EffectTypeArg>,
        kind: EffectKind,
    ) -> Self {
        Self {
            name: name.into(),
            type_args,
            kind,
        }
    }

    /// Check if this effect matches another (ignoring kind)
    pub fn matches(&self, other: &Effect) -> bool {
        self.name == other.name && self.type_args == other.type_args
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.type_args.is_empty() {
            write!(f, "[")?;
            for (i, arg) in self.type_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

/// Type argument for parameterized effects
///
/// Effect type arguments can be concrete type names or type variables.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectTypeArg {
    /// Concrete type (e.g., `Int`, `String`, `HttpError`)
    Concrete(String),
    /// Type variable (e.g., `T`, `S`)
    Variable(String),
}

impl fmt::Display for EffectTypeArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectTypeArg::Concrete(name) => write!(f, "{}", name),
            EffectTypeArg::Variable(name) => write!(f, "{}", name),
        }
    }
}

// ============================================================================
// Effect Rows
// ============================================================================

/// A set of effects that a computation may perform
///
/// Effect sets are used to track all effects that may occur during execution.
/// They form the basis for effect rows in the type system.
#[derive(Debug, Clone, Default)]
pub struct EffectSet {
    /// The effects in this set
    effects: IndexSet<Effect>,
}

impl EffectSet {
    /// Create an empty effect set (pure computation)
    pub fn empty() -> Self {
        Self {
            effects: IndexSet::new(),
        }
    }

    /// Create an effect set with a single effect
    pub fn singleton(effect: Effect) -> Self {
        let mut effects = IndexSet::new();
        effects.insert(effect);
        Self { effects }
    }

    /// Create an effect set from multiple effects
    pub fn from_effects(effects: impl IntoIterator<Item = Effect>) -> Self {
        Self {
            effects: effects.into_iter().collect(),
        }
    }

    /// Add an effect to this set
    pub fn insert(&mut self, effect: Effect) {
        self.effects.insert(effect);
    }

    /// Remove an effect from this set
    pub fn remove(&mut self, effect: &Effect) -> bool {
        self.effects.shift_remove(effect)
    }

    /// Remove an effect by name (ignoring kind and type arguments)
    ///
    /// This is used by handlers which match effects by name only.
    pub fn remove_by_name(&mut self, name: &str) -> bool {
        let to_remove: Vec<_> = self.effects.iter()
            .filter(|e| e.name == name)
            .cloned()
            .collect();
        let removed = !to_remove.is_empty();
        for effect in to_remove {
            self.effects.shift_remove(&effect);
        }
        removed
    }

    /// Check if this set contains an effect
    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    /// Check if this set contains an effect by name (ignoring kind and type arguments)
    pub fn contains_named(&self, name: &str) -> bool {
        self.effects.iter().any(|e| e.name == name)
    }

    /// Check if this set is empty (pure)
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Get the number of effects in this set
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Iterate over effects in this set
    pub fn iter(&self) -> impl Iterator<Item = &Effect> {
        self.effects.iter()
    }

    /// Union of two effect sets
    pub fn union(&self, other: &EffectSet) -> EffectSet {
        let mut result = self.clone();
        for effect in &other.effects {
            result.effects.insert(effect.clone());
        }
        result
    }

    /// Intersection of two effect sets
    pub fn intersection(&self, other: &EffectSet) -> EffectSet {
        EffectSet {
            effects: self
                .effects
                .intersection(&other.effects)
                .cloned()
                .collect(),
        }
    }

    /// Difference of two effect sets (self - other)
    pub fn difference(&self, other: &EffectSet) -> EffectSet {
        EffectSet {
            effects: self.effects.difference(&other.effects).cloned().collect(),
        }
    }

    /// Check if this set is a subset of another
    pub fn is_subset(&self, other: &EffectSet) -> bool {
        self.effects.is_subset(&other.effects)
    }

    /// Check if this set is a superset of another
    pub fn is_superset(&self, other: &EffectSet) -> bool {
        self.effects.is_superset(&other.effects)
    }

    /// Get the most restrictive effect kind in this set
    ///
    /// Returns MultiShot if any effect is multi-shot,
    /// OneShot if any effect is one-shot (and none are multi-shot),
    /// TailResumptive otherwise.
    pub fn dominant_kind(&self) -> EffectKind {
        let mut has_one_shot = false;
        for effect in &self.effects {
            match effect.kind {
                EffectKind::MultiShot => return EffectKind::MultiShot,
                EffectKind::OneShot => has_one_shot = true,
                EffectKind::TailResumptive => {}
            }
        }
        if has_one_shot {
            EffectKind::OneShot
        } else {
            EffectKind::TailResumptive
        }
    }
}

impl fmt::Display for EffectSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            write!(f, "{{}}")
        } else {
            write!(f, "{{")?;
            for (i, effect) in self.effects.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", effect)?;
            }
            write!(f, "}}")
        }
    }
}

impl PartialEq for EffectSet {
    fn eq(&self, other: &Self) -> bool {
        self.effects == other.effects
    }
}

impl Eq for EffectSet {}

/// Row-polymorphic effect type
///
/// Effect rows represent the effect signature of a computation:
/// - `Closed(effects)`: Exactly these effects, no more (e.g., `!{IO, Console}`)
/// - `Open(effects, var)`: These effects plus unknown others (e.g., `!{IO | e}`)
/// - `Var(var)`: Unknown effect row (e.g., `!e`)
///
/// Row polymorphism allows functions to be generic over effects while maintaining
/// type safety. For example:
///
/// ```aria
/// fn map_with_effect[T, U, E](items: Array[T], f: Fn(T) -> U !E) -> Array[U] !E
///   items.map(f)
/// end
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectRow {
    /// Closed effect row: exactly these effects
    ///
    /// `!{IO, Console}` - has IO and Console effects, nothing else
    Closed(EffectSet),

    /// Open effect row: these effects plus a row variable
    ///
    /// `!{IO | e}` - has IO effect plus whatever effects `e` represents
    Open {
        effects: EffectSet,
        tail: EffectVar,
    },

    /// Effect row variable: unknown effect row
    ///
    /// `!e` - represents an unknown set of effects
    Var(EffectVar),
}

impl EffectRow {
    /// Create an empty (pure) effect row
    pub fn pure() -> Self {
        EffectRow::Closed(EffectSet::empty())
    }

    /// Create a closed effect row with a single effect
    pub fn single(effect: Effect) -> Self {
        EffectRow::Closed(EffectSet::singleton(effect))
    }

    /// Create a closed effect row from multiple effects
    pub fn closed(effects: impl IntoIterator<Item = Effect>) -> Self {
        EffectRow::Closed(EffectSet::from_effects(effects))
    }

    /// Create an open effect row
    pub fn open(effects: impl IntoIterator<Item = Effect>, tail: EffectVar) -> Self {
        EffectRow::Open {
            effects: EffectSet::from_effects(effects),
            tail,
        }
    }

    /// Create an effect row variable
    pub fn var(var: EffectVar) -> Self {
        EffectRow::Var(var)
    }

    /// Check if this is a pure (no effects) row
    pub fn is_pure(&self) -> bool {
        match self {
            EffectRow::Closed(effects) => effects.is_empty(),
            _ => false,
        }
    }

    /// Check if this row contains a specific effect
    pub fn contains(&self, effect: &Effect) -> bool {
        match self {
            EffectRow::Closed(effects) => effects.contains(effect),
            EffectRow::Open { effects, .. } => effects.contains(effect),
            EffectRow::Var(_) => false,
        }
    }

    /// Get the known effects in this row
    pub fn known_effects(&self) -> &EffectSet {
        static EMPTY: std::sync::LazyLock<EffectSet> = std::sync::LazyLock::new(EffectSet::empty);
        match self {
            EffectRow::Closed(effects) => effects,
            EffectRow::Open { effects, .. } => effects,
            EffectRow::Var(_) => &EMPTY,
        }
    }

    /// Get the row variable if this is an open row or row variable
    pub fn row_variable(&self) -> Option<EffectVar> {
        match self {
            EffectRow::Closed(_) => None,
            EffectRow::Open { tail, .. } => Some(*tail),
            EffectRow::Var(var) => Some(*var),
        }
    }

    /// Add an effect to this row
    pub fn with_effect(self, effect: Effect) -> Self {
        match self {
            EffectRow::Closed(mut effects) => {
                effects.insert(effect);
                EffectRow::Closed(effects)
            }
            EffectRow::Open { mut effects, tail } => {
                effects.insert(effect);
                EffectRow::Open { effects, tail }
            }
            EffectRow::Var(var) => EffectRow::Open {
                effects: EffectSet::singleton(effect),
                tail: var,
            },
        }
    }

    /// Remove an effect from this row (for handler elimination)
    pub fn without_effect(self, effect: &Effect) -> Self {
        match self {
            EffectRow::Closed(mut effects) => {
                effects.remove(effect);
                EffectRow::Closed(effects)
            }
            EffectRow::Open { mut effects, tail } => {
                effects.remove(effect);
                EffectRow::Open { effects, tail }
            }
            EffectRow::Var(var) => EffectRow::Var(var),
        }
    }

    /// Remove an effect by name from this row (for handler elimination)
    ///
    /// This matches effects by name only, ignoring kind and type arguments.
    /// Used by handlers which handle effects regardless of their compilation strategy.
    pub fn without_named_effect(self, name: &str) -> Self {
        match self {
            EffectRow::Closed(mut effects) => {
                effects.remove_by_name(name);
                EffectRow::Closed(effects)
            }
            EffectRow::Open { mut effects, tail } => {
                effects.remove_by_name(name);
                EffectRow::Open { effects, tail }
            }
            EffectRow::Var(var) => EffectRow::Var(var),
        }
    }

    /// Get all free effect variables in this row
    pub fn free_vars(&self) -> Vec<EffectVar> {
        match self {
            EffectRow::Closed(_) => Vec::new(),
            EffectRow::Open { tail, .. } => vec![*tail],
            EffectRow::Var(var) => vec![*var],
        }
    }
}

impl Default for EffectRow {
    fn default() -> Self {
        EffectRow::pure()
    }
}

impl fmt::Display for EffectRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectRow::Closed(effects) => {
                if effects.is_empty() {
                    write!(f, "!{{}}")
                } else {
                    write!(f, "!{}", effects)
                }
            }
            EffectRow::Open { effects, tail } => {
                if effects.is_empty() {
                    write!(f, "!{}", tail)
                } else {
                    write!(f, "!{{{} | {}}}", effects, tail)
                }
            }
            EffectRow::Var(var) => write!(f, "!{}", var),
        }
    }
}

// ============================================================================
// Effect Declarations
// ============================================================================

/// An effect operation declaration
///
/// Effect operations are the interface methods that an effect exposes.
/// For example, the `Console` effect might have operations `print` and `read_line`.
#[derive(Debug, Clone)]
pub struct EffectOperation {
    /// Operation name
    pub name: String,
    /// Type parameters for generic operations
    pub type_params: Vec<String>,
    /// Parameter types
    pub params: Vec<EffectOperationParam>,
    /// Return type (as string, resolved during type checking)
    pub return_type: String,
    /// Source span for error reporting
    pub span: Span,
}

/// Parameter for an effect operation
#[derive(Debug, Clone)]
pub struct EffectOperationParam {
    /// Parameter name
    pub name: String,
    /// Parameter type (as string, resolved during type checking)
    pub ty: String,
}

/// An effect declaration
///
/// Effect declarations define the interface for an algebraic effect.
/// They specify the operations that the effect supports and their types.
///
/// ```aria
/// effect Console
///   fn print(message: String) -> Unit
///   fn read_line() -> String
/// end
/// ```
#[derive(Debug, Clone)]
pub struct EffectDecl {
    /// Effect name
    pub name: String,
    /// Type parameters for generic effects (e.g., `State[S]`)
    pub type_params: Vec<String>,
    /// Effect operations
    pub operations: Vec<EffectOperation>,
    /// Effect kind classification
    pub kind: EffectKind,
    /// Source span for error reporting
    pub span: Span,
}

impl EffectDecl {
    /// Create a new effect declaration
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            type_params: Vec::new(),
            operations: Vec::new(),
            kind: EffectKind::default(),
            span,
        }
    }

    /// Add a type parameter to this effect
    pub fn with_type_param(mut self, param: impl Into<String>) -> Self {
        self.type_params.push(param.into());
        self
    }

    /// Add an operation to this effect
    pub fn with_operation(mut self, op: EffectOperation) -> Self {
        self.operations.push(op);
        self
    }

    /// Set the effect kind
    pub fn with_kind(mut self, kind: EffectKind) -> Self {
        self.kind = kind;
        self
    }
}

// ============================================================================
// Effect Environment
// ============================================================================

/// Environment for effect declarations and bindings
///
/// Tracks available effects and their definitions during type checking.
#[derive(Debug, Clone, Default)]
pub struct EffectEnv {
    /// Effect declarations: name -> declaration
    effects: FxHashMap<String, EffectDecl>,
}

impl EffectEnv {
    /// Create a new empty effect environment
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an environment with standard library effects
    pub fn with_stdlib() -> Self {
        let mut env = Self::new();
        env.register_stdlib_effects();
        env
    }

    /// Register standard library effects
    fn register_stdlib_effects(&mut self) {
        // IO effect (tail-resumptive)
        self.define(EffectDecl {
            name: "IO".to_string(),
            type_params: Vec::new(),
            operations: vec![
                EffectOperation {
                    name: "read".to_string(),
                    type_params: Vec::new(),
                    params: vec![EffectOperationParam {
                        name: "path".to_string(),
                        ty: "String".to_string(),
                    }],
                    return_type: "Bytes".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "write".to_string(),
                    type_params: Vec::new(),
                    params: vec![
                        EffectOperationParam {
                            name: "path".to_string(),
                            ty: "String".to_string(),
                        },
                        EffectOperationParam {
                            name: "data".to_string(),
                            ty: "Bytes".to_string(),
                        },
                    ],
                    return_type: "Unit".to_string(),
                    span: Span::default(),
                },
            ],
            kind: EffectKind::TailResumptive,
            span: Span::default(),
        });

        // Console effect (tail-resumptive)
        self.define(EffectDecl {
            name: "Console".to_string(),
            type_params: Vec::new(),
            operations: vec![
                EffectOperation {
                    name: "print".to_string(),
                    type_params: Vec::new(),
                    params: vec![EffectOperationParam {
                        name: "message".to_string(),
                        ty: "String".to_string(),
                    }],
                    return_type: "Unit".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "read_line".to_string(),
                    type_params: Vec::new(),
                    params: Vec::new(),
                    return_type: "String".to_string(),
                    span: Span::default(),
                },
            ],
            kind: EffectKind::TailResumptive,
            span: Span::default(),
        });

        // Exception effect (one-shot)
        self.define(EffectDecl {
            name: "Exception".to_string(),
            type_params: vec!["E".to_string()],
            operations: vec![EffectOperation {
                name: "raise".to_string(),
                type_params: Vec::new(),
                params: vec![EffectOperationParam {
                    name: "error".to_string(),
                    ty: "E".to_string(),
                }],
                return_type: "Never".to_string(),
                span: Span::default(),
            }],
            kind: EffectKind::OneShot,
            span: Span::default(),
        });

        // Async effect (one-shot)
        self.define(EffectDecl {
            name: "Async".to_string(),
            type_params: Vec::new(),
            operations: vec![
                EffectOperation {
                    name: "await".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![EffectOperationParam {
                        name: "future".to_string(),
                        ty: "Future[T]".to_string(),
                    }],
                    return_type: "T".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "spawn".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![EffectOperationParam {
                        name: "f".to_string(),
                        ty: "Fn() -> T".to_string(),
                    }],
                    return_type: "Future[T]".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "yield".to_string(),
                    type_params: Vec::new(),
                    params: Vec::new(),
                    return_type: "Unit".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "scope".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![EffectOperationParam {
                        name: "f".to_string(),
                        ty: "Fn(Scope) -> T".to_string(),
                    }],
                    return_type: "T".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "supervisor".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![EffectOperationParam {
                        name: "f".to_string(),
                        ty: "Fn(Scope) -> T".to_string(),
                    }],
                    return_type: "T".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "timeout".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![
                        EffectOperationParam {
                            name: "duration".to_string(),
                            ty: "Duration".to_string(),
                        },
                        EffectOperationParam {
                            name: "f".to_string(),
                            ty: "Fn(Scope) -> T".to_string(),
                        },
                    ],
                    return_type: "Result[T, TimeoutError]".to_string(),
                    span: Span::default(),
                },
            ],
            kind: EffectKind::OneShot,
            span: Span::default(),
        });

        // Cancel effect for cooperative cancellation
        self.define(EffectDecl {
            name: "Cancel".to_string(),
            type_params: Vec::new(),
            operations: vec![
                EffectOperation {
                    name: "check".to_string(),
                    type_params: Vec::new(),
                    params: Vec::new(),
                    return_type: "Unit".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "token".to_string(),
                    type_params: Vec::new(),
                    params: Vec::new(),
                    return_type: "CancelToken".to_string(),
                    span: Span::default(),
                },
            ],
            kind: EffectKind::TailResumptive,
            span: Span::default(),
        });

        // State effect (tail-resumptive)
        self.define(EffectDecl {
            name: "State".to_string(),
            type_params: vec!["S".to_string()],
            operations: vec![
                EffectOperation {
                    name: "get".to_string(),
                    type_params: Vec::new(),
                    params: Vec::new(),
                    return_type: "S".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "put".to_string(),
                    type_params: Vec::new(),
                    params: vec![EffectOperationParam {
                        name: "value".to_string(),
                        ty: "S".to_string(),
                    }],
                    return_type: "Unit".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "modify".to_string(),
                    type_params: Vec::new(),
                    params: vec![EffectOperationParam {
                        name: "f".to_string(),
                        ty: "Fn(S) -> S".to_string(),
                    }],
                    return_type: "Unit".to_string(),
                    span: Span::default(),
                },
            ],
            kind: EffectKind::TailResumptive,
            span: Span::default(),
        });

        // Reader effect (tail-resumptive)
        self.define(EffectDecl {
            name: "Reader".to_string(),
            type_params: vec!["R".to_string()],
            operations: vec![EffectOperation {
                name: "ask".to_string(),
                type_params: Vec::new(),
                params: Vec::new(),
                return_type: "R".to_string(),
                span: Span::default(),
            }],
            kind: EffectKind::TailResumptive,
            span: Span::default(),
        });

        // Choice effect (multi-shot)
        self.define(EffectDecl {
            name: "Choice".to_string(),
            type_params: Vec::new(),
            operations: vec![
                EffectOperation {
                    name: "choose".to_string(),
                    type_params: vec!["T".to_string()],
                    params: vec![EffectOperationParam {
                        name: "options".to_string(),
                        ty: "Array[T]".to_string(),
                    }],
                    return_type: "T".to_string(),
                    span: Span::default(),
                },
                EffectOperation {
                    name: "fail".to_string(),
                    type_params: Vec::new(),
                    params: Vec::new(),
                    return_type: "Never".to_string(),
                    span: Span::default(),
                },
            ],
            kind: EffectKind::MultiShot,
            span: Span::default(),
        });

        // Channel effect (one-shot) - inter-task communication
        self.define(EffectDecl {
            name: "Channel".to_string(),
            type_params: vec!["T".to_string()],
            operations: vec![
                // Create a new channel
                EffectOperation {
                    name: "new".to_string(),
                    type_params: Vec::new(),
                    params: vec![EffectOperationParam {
                        name: "capacity".to_string(),
                        ty: "Int?".to_string(), // Optional capacity, None = unbuffered
                    }],
                    return_type: "Channel[T]".to_string(),
                    span: Span::default(),
                },
                // Send a value (blocking)
                EffectOperation {
                    name: "send".to_string(),
                    type_params: Vec::new(),
                    params: vec![
                        EffectOperationParam {
                            name: "channel".to_string(),
                            ty: "Channel[T]".to_string(),
                        },
                        EffectOperationParam {
                            name: "value".to_string(),
                            ty: "T".to_string(),
                        },
                    ],
                    return_type: "Unit".to_string(),
                    span: Span::default(),
                },
                // Receive a value (blocking)
                EffectOperation {
                    name: "recv".to_string(),
                    type_params: Vec::new(),
                    params: vec![EffectOperationParam {
                        name: "channel".to_string(),
                        ty: "Channel[T]".to_string(),
                    }],
                    return_type: "T".to_string(),
                    span: Span::default(),
                },
                // Try to send without blocking
                EffectOperation {
                    name: "try_send".to_string(),
                    type_params: Vec::new(),
                    params: vec![
                        EffectOperationParam {
                            name: "channel".to_string(),
                            ty: "Channel[T]".to_string(),
                        },
                        EffectOperationParam {
                            name: "value".to_string(),
                            ty: "T".to_string(),
                        },
                    ],
                    return_type: "Result[Unit, ChannelError]".to_string(),
                    span: Span::default(),
                },
                // Try to receive without blocking
                EffectOperation {
                    name: "try_recv".to_string(),
                    type_params: Vec::new(),
                    params: vec![EffectOperationParam {
                        name: "channel".to_string(),
                        ty: "Channel[T]".to_string(),
                    }],
                    return_type: "T?".to_string(), // Optional - None if empty
                    span: Span::default(),
                },
                // Close a channel
                EffectOperation {
                    name: "close".to_string(),
                    type_params: Vec::new(),
                    params: vec![EffectOperationParam {
                        name: "channel".to_string(),
                        ty: "Channel[T]".to_string(),
                    }],
                    return_type: "Unit".to_string(),
                    span: Span::default(),
                },
            ],
            kind: EffectKind::OneShot,
            span: Span::default(),
        });
    }

    /// Define a new effect
    pub fn define(&mut self, decl: EffectDecl) {
        self.effects.insert(decl.name.clone(), decl);
    }

    /// Look up an effect by name
    pub fn lookup(&self, name: &str) -> Option<&EffectDecl> {
        self.effects.get(name)
    }

    /// Check if an effect exists
    pub fn contains(&self, name: &str) -> bool {
        self.effects.contains_key(name)
    }

    /// Get all defined effects
    pub fn all_effects(&self) -> impl Iterator<Item = &EffectDecl> {
        self.effects.values()
    }
}

// ============================================================================
// Effect Errors
// ============================================================================

/// Errors that can occur during effect type checking
#[derive(Debug, Clone, Error)]
pub enum EffectError {
    #[error("Undefined effect: {0}")]
    UndefinedEffect(String, Span),

    #[error("Effect operation not found: {effect}.{operation}")]
    UndefinedOperation {
        effect: String,
        operation: String,
        span: Span,
    },

    #[error("Unhandled effect: {effect}")]
    UnhandledEffect { effect: String, span: Span },

    #[error("Effect mismatch: expected {expected}, found {found}")]
    EffectMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Cannot unify effect rows: {left} and {right}")]
    UnificationError {
        left: String,
        right: String,
        span: Span,
    },

    #[error("Recursive effect row detected")]
    RecursiveEffect(Span),

    #[error("Effect annotation required but not found")]
    MissingAnnotation(Span),

    #[error("Effect purity violation: expected pure but found {effects}")]
    PurityViolation { effects: String, span: Span },
}

/// Result type for effect operations
pub type EffectResult<T> = Result<T, EffectError>;

// ============================================================================
// Effect Inference
// ============================================================================

/// Effect inference engine
///
/// Handles effect type inference using row polymorphism and unification.
/// Integrates with the main type inference system from aria-types.
#[derive(Debug)]
pub struct EffectInference {
    /// Next effect variable ID
    next_var: u32,
    /// Substitution map: EffectVar -> EffectRow
    substitution: FxHashMap<EffectVar, EffectRow>,
    /// Collected errors
    errors: Vec<EffectError>,
}

impl EffectInference {
    /// Create a new effect inference engine
    pub fn new() -> Self {
        Self {
            next_var: 0,
            substitution: FxHashMap::default(),
            errors: Vec::new(),
        }
    }

    /// Create a fresh effect variable
    pub fn fresh_var(&mut self) -> EffectVar {
        let var = EffectVar(self.next_var);
        self.next_var += 1;
        var
    }

    /// Create a fresh effect row variable
    pub fn fresh_row(&mut self) -> EffectRow {
        EffectRow::Var(self.fresh_var())
    }

    /// Unify two effect rows
    ///
    /// Row unification follows these rules:
    /// 1. Var(v) ~ r: Substitute v -> r (with occurs check)
    /// 2. Closed(e1) ~ Closed(e2): e1 must equal e2
    /// 3. Open(e1, v1) ~ Open(e2, v2): Unify effects, unify tails
    /// 4. Open(e, v) ~ Closed(e'): e must be subset of e', v -> remaining
    pub fn unify(&mut self, r1: &EffectRow, r2: &EffectRow, span: Span) -> EffectResult<()> {
        let r1 = self.apply(r1);
        let r2 = self.apply(r2);

        match (&r1, &r2) {
            // Same row
            _ if r1 == r2 => Ok(()),

            // Variable on left
            (EffectRow::Var(var), _) => {
                if self.occurs_check(*var, &r2) {
                    Err(EffectError::RecursiveEffect(span))
                } else {
                    self.substitution.insert(*var, r2.clone());
                    Ok(())
                }
            }

            // Variable on right
            (_, EffectRow::Var(var)) => {
                if self.occurs_check(*var, &r1) {
                    Err(EffectError::RecursiveEffect(span))
                } else {
                    self.substitution.insert(*var, r1.clone());
                    Ok(())
                }
            }

            // Both closed: must be equal
            (EffectRow::Closed(e1), EffectRow::Closed(e2)) => {
                if e1 == e2 {
                    Ok(())
                } else {
                    Err(EffectError::UnificationError {
                        left: format!("{}", r1),
                        right: format!("{}", r2),
                        span,
                    })
                }
            }

            // Open on left, closed on right
            (EffectRow::Open { effects, tail }, EffectRow::Closed(closed_effects)) => {
                // Check that all effects in the open row are in the closed row
                if effects.is_subset(closed_effects) {
                    // The tail variable represents the remaining effects
                    let remaining = closed_effects.difference(effects);
                    self.substitution
                        .insert(*tail, EffectRow::Closed(remaining));
                    Ok(())
                } else {
                    Err(EffectError::UnificationError {
                        left: format!("{}", r1),
                        right: format!("{}", r2),
                        span,
                    })
                }
            }

            // Closed on left, open on right
            (EffectRow::Closed(closed_effects), EffectRow::Open { effects, tail }) => {
                // Symmetric case
                if effects.is_subset(closed_effects) {
                    let remaining = closed_effects.difference(effects);
                    self.substitution
                        .insert(*tail, EffectRow::Closed(remaining));
                    Ok(())
                } else {
                    Err(EffectError::UnificationError {
                        left: format!("{}", r1),
                        right: format!("{}", r2),
                        span,
                    })
                }
            }

            // Both open
            (
                EffectRow::Open {
                    effects: e1,
                    tail: t1,
                },
                EffectRow::Open {
                    effects: e2,
                    tail: t2,
                },
            ) => {
                // Create a fresh tail variable for the combined row
                let fresh_tail = self.fresh_var();

                // Effects unique to each side
                let only_in_r1 = e1.difference(e2);
                let only_in_r2 = e2.difference(e1);

                // t1 = {only_in_r2 | fresh}
                // t2 = {only_in_r1 | fresh}
                if only_in_r2.is_empty() {
                    self.substitution
                        .insert(*t1, EffectRow::Var(fresh_tail));
                } else {
                    self.substitution.insert(
                        *t1,
                        EffectRow::Open {
                            effects: only_in_r2,
                            tail: fresh_tail,
                        },
                    );
                }

                if only_in_r1.is_empty() {
                    self.substitution
                        .insert(*t2, EffectRow::Var(fresh_tail));
                } else {
                    self.substitution.insert(
                        *t2,
                        EffectRow::Open {
                            effects: only_in_r1,
                            tail: fresh_tail,
                        },
                    );
                }

                Ok(())
            }
        }
    }

    /// Apply substitution to an effect row
    pub fn apply(&self, row: &EffectRow) -> EffectRow {
        match row {
            EffectRow::Var(var) => self
                .substitution
                .get(var)
                .map(|r| self.apply(r))
                .unwrap_or_else(|| row.clone()),

            EffectRow::Open { effects, tail } => {
                match self.substitution.get(tail) {
                    Some(tail_row) => {
                        // Apply the tail substitution
                        let applied_tail = self.apply(tail_row);
                        match applied_tail {
                            EffectRow::Closed(tail_effects) => {
                                // Combine effects with tail
                                EffectRow::Closed(effects.union(&tail_effects))
                            }
                            EffectRow::Open {
                                effects: tail_effects,
                                tail: new_tail,
                            } => EffectRow::Open {
                                effects: effects.union(&tail_effects),
                                tail: new_tail,
                            },
                            EffectRow::Var(new_var) => EffectRow::Open {
                                effects: effects.clone(),
                                tail: new_var,
                            },
                        }
                    }
                    None => row.clone(),
                }
            }

            EffectRow::Closed(_) => row.clone(),
        }
    }

    /// Check if an effect variable occurs in an effect row
    fn occurs_check(&self, var: EffectVar, row: &EffectRow) -> bool {
        match row {
            EffectRow::Var(v) => {
                if *v == var {
                    true
                } else if let Some(r) = self.substitution.get(v) {
                    self.occurs_check(var, r)
                } else {
                    false
                }
            }
            EffectRow::Open { tail, .. } => {
                if *tail == var {
                    true
                } else if let Some(r) = self.substitution.get(tail) {
                    self.occurs_check(var, r)
                } else {
                    false
                }
            }
            EffectRow::Closed(_) => false,
        }
    }

    /// Generalize free effect variables in a row
    ///
    /// Returns the generalized row and the list of quantified variables.
    pub fn generalize(&self, row: &EffectRow) -> (EffectRow, Vec<EffectVar>) {
        let row = self.apply(row);
        let free_vars = row.free_vars();
        (row, free_vars)
    }

    /// Instantiate a generalized effect row with fresh variables
    pub fn instantiate(&mut self, row: &EffectRow, bound_vars: &[EffectVar]) -> EffectRow {
        if bound_vars.is_empty() {
            return row.clone();
        }

        let fresh_mapping: FxHashMap<EffectVar, EffectVar> = bound_vars
            .iter()
            .map(|v| (*v, self.fresh_var()))
            .collect();

        self.substitute_vars(row, &fresh_mapping)
    }

    /// Substitute effect variables in a row
    fn substitute_vars(
        &self,
        row: &EffectRow,
        mapping: &FxHashMap<EffectVar, EffectVar>,
    ) -> EffectRow {
        match row {
            EffectRow::Var(var) => {
                if let Some(new_var) = mapping.get(var) {
                    EffectRow::Var(*new_var)
                } else {
                    row.clone()
                }
            }
            EffectRow::Open { effects, tail } => {
                let new_tail = mapping.get(tail).copied().unwrap_or(*tail);
                EffectRow::Open {
                    effects: effects.clone(),
                    tail: new_tail,
                }
            }
            EffectRow::Closed(_) => row.clone(),
        }
    }

    /// Check if a row is closed (no free variables after substitution)
    pub fn is_closed(&self, row: &EffectRow) -> bool {
        matches!(self.apply(row), EffectRow::Closed(_))
    }

    /// Get the final, fully-resolved effect row
    pub fn resolve(&self, row: &EffectRow) -> EffectRow {
        self.apply(row)
    }

    /// Get collected errors
    pub fn errors(&self) -> &[EffectError] {
        &self.errors
    }

    /// Add an error
    pub fn add_error(&mut self, error: EffectError) {
        self.errors.push(error);
    }

    /// Take all errors, leaving an empty list
    pub fn take_errors(&mut self) -> Vec<EffectError> {
        std::mem::take(&mut self.errors)
    }
}

impl Default for EffectInference {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Integration with Type System
// ============================================================================

/// Effectful type: a type paired with its effect row
///
/// This represents the complete type of an effectful computation, combining
/// the value type with the effects it may perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectfulType {
    /// The value type (from aria-types)
    pub value_type: String, // We use String here to avoid circular dependency
    /// The effect row
    pub effects: EffectRow,
}

impl EffectfulType {
    /// Create a pure effectful type (no effects)
    pub fn pure(value_type: impl Into<String>) -> Self {
        Self {
            value_type: value_type.into(),
            effects: EffectRow::pure(),
        }
    }

    /// Create an effectful type with specific effects
    pub fn with_effects(value_type: impl Into<String>, effects: EffectRow) -> Self {
        Self {
            value_type: value_type.into(),
            effects,
        }
    }

    /// Check if this type is pure (no effects)
    pub fn is_pure(&self) -> bool {
        self.effects.is_pure()
    }
}

impl fmt::Display for EffectfulType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_pure() {
            write!(f, "{}", self.value_type)
        } else {
            write!(f, "{} {}", self.value_type, self.effects)
        }
    }
}

// ============================================================================
// Effect Handlers
// ============================================================================

/// Handler clause for an effect operation
#[derive(Debug, Clone)]
pub struct HandlerClause {
    /// Effect name
    pub effect: String,
    /// Operation name
    pub operation: String,
    /// Parameter patterns
    pub params: Vec<String>,
    /// Handler body (as AST, would be ast::Expr in full implementation)
    pub body: String, // Placeholder for now
}

/// Return clause for a handler
#[derive(Debug, Clone)]
pub struct ReturnClause {
    /// Parameter pattern for the return value
    pub param: String,
    /// Handler body for the return value
    pub body: String, // Placeholder for now
}

/// Effect handler
///
/// Handlers define interpretations for algebraic effects. They transform
/// computations that perform effects into computations with fewer (or no) effects.
#[derive(Debug, Clone)]
pub struct Handler {
    /// Handler clauses for effect operations
    pub clauses: Vec<HandlerClause>,
    /// Return clause
    pub return_clause: Option<ReturnClause>,
    /// Effects handled by this handler
    pub handled_effects: EffectSet,
}

impl Handler {
    /// Create a new empty handler
    pub fn new() -> Self {
        Self {
            clauses: Vec::new(),
            return_clause: None,
            handled_effects: EffectSet::empty(),
        }
    }

    /// Add a handler clause
    pub fn with_clause(mut self, clause: HandlerClause) -> Self {
        // Track the effect being handled
        self.handled_effects
            .insert(Effect::simple(clause.effect.clone()));
        self.clauses.push(clause);
        self
    }

    /// Set the return clause
    pub fn with_return(mut self, return_clause: ReturnClause) -> Self {
        self.return_clause = Some(return_clause);
        self
    }

    /// Compute the resulting effect row after handling
    ///
    /// Given an input effect row, returns the effect row after this handler
    /// has eliminated its handled effects. Uses name-based matching so effects
    /// are handled regardless of their compilation strategy (kind).
    pub fn resulting_effects(&self, input: &EffectRow) -> EffectRow {
        let mut result = input.clone();
        for effect in self.handled_effects.iter() {
            result = result.without_named_effect(&effect.name);
        }
        result
    }
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Effect Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_effect_creation() {
        let io = Effect::simple("IO");
        assert_eq!(io.name, "IO");
        assert!(io.type_args.is_empty());
        assert_eq!(io.kind, EffectKind::TailResumptive);

        let exception = Effect::parameterized_with_kind(
            "Exception",
            vec![EffectTypeArg::Concrete("HttpError".to_string())],
            EffectKind::OneShot,
        );
        assert_eq!(exception.name, "Exception");
        assert_eq!(exception.type_args.len(), 1);
        assert_eq!(exception.kind, EffectKind::OneShot);
    }

    #[test]
    fn test_effect_display() {
        let io = Effect::simple("IO");
        assert_eq!(format!("{}", io), "IO");

        let exception = Effect::parameterized(
            "Exception",
            vec![EffectTypeArg::Concrete("Error".to_string())],
        );
        assert_eq!(format!("{}", exception), "Exception[Error]");

        let state = Effect::parameterized(
            "State",
            vec![
                EffectTypeArg::Concrete("Int".to_string()),
                EffectTypeArg::Variable("T".to_string()),
            ],
        );
        assert_eq!(format!("{}", state), "State[Int, T]");
    }

    // ------------------------------------------------------------------------
    // EffectSet Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_effect_set_operations() {
        let mut set1 = EffectSet::empty();
        assert!(set1.is_empty());

        set1.insert(Effect::simple("IO"));
        set1.insert(Effect::simple("Console"));
        assert_eq!(set1.len(), 2);
        assert!(set1.contains(&Effect::simple("IO")));

        let set2 = EffectSet::from_effects(vec![
            Effect::simple("IO"),
            Effect::simple("Async"),
        ]);

        let union = set1.union(&set2);
        assert_eq!(union.len(), 3);

        let intersection = set1.intersection(&set2);
        assert_eq!(intersection.len(), 1);
        assert!(intersection.contains(&Effect::simple("IO")));

        let diff = set1.difference(&set2);
        assert_eq!(diff.len(), 1);
        assert!(diff.contains(&Effect::simple("Console")));
    }

    #[test]
    fn test_effect_set_dominant_kind() {
        let pure = EffectSet::empty();
        assert_eq!(pure.dominant_kind(), EffectKind::TailResumptive);

        let tail_resumptive = EffectSet::from_effects(vec![
            Effect::with_kind("IO", EffectKind::TailResumptive),
            Effect::with_kind("Console", EffectKind::TailResumptive),
        ]);
        assert_eq!(tail_resumptive.dominant_kind(), EffectKind::TailResumptive);

        let one_shot = EffectSet::from_effects(vec![
            Effect::with_kind("IO", EffectKind::TailResumptive),
            Effect::with_kind("Async", EffectKind::OneShot),
        ]);
        assert_eq!(one_shot.dominant_kind(), EffectKind::OneShot);

        let multi_shot = EffectSet::from_effects(vec![
            Effect::with_kind("IO", EffectKind::TailResumptive),
            Effect::with_kind("Choice", EffectKind::MultiShot),
        ]);
        assert_eq!(multi_shot.dominant_kind(), EffectKind::MultiShot);
    }

    // ------------------------------------------------------------------------
    // EffectRow Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_effect_row_pure() {
        let pure = EffectRow::pure();
        assert!(pure.is_pure());
        assert_eq!(format!("{}", pure), "!{}");
    }

    #[test]
    fn test_effect_row_closed() {
        let row = EffectRow::closed(vec![
            Effect::simple("IO"),
            Effect::simple("Console"),
        ]);
        assert!(!row.is_pure());
        assert!(row.contains(&Effect::simple("IO")));
        assert!(!row.contains(&Effect::simple("Async")));
    }

    #[test]
    fn test_effect_row_open() {
        let var = EffectVar(0);
        let row = EffectRow::open(vec![Effect::simple("IO")], var);

        assert!(!row.is_pure());
        assert!(row.contains(&Effect::simple("IO")));
        assert_eq!(row.row_variable(), Some(var));
    }

    #[test]
    fn test_effect_row_with_effect() {
        let row = EffectRow::pure();
        let row = row.with_effect(Effect::simple("IO"));
        assert!(row.contains(&Effect::simple("IO")));
    }

    #[test]
    fn test_effect_row_without_effect() {
        let row = EffectRow::closed(vec![
            Effect::simple("IO"),
            Effect::simple("Console"),
        ]);
        let row = row.without_effect(&Effect::simple("IO"));

        assert!(!row.contains(&Effect::simple("IO")));
        assert!(row.contains(&Effect::simple("Console")));
    }

    // ------------------------------------------------------------------------
    // Effect Unification Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_unify_same_closed() {
        let mut inference = EffectInference::new();
        let row1 = EffectRow::closed(vec![Effect::simple("IO")]);
        let row2 = EffectRow::closed(vec![Effect::simple("IO")]);

        assert!(inference.unify(&row1, &row2, Span::default()).is_ok());
    }

    #[test]
    fn test_unify_different_closed_fails() {
        let mut inference = EffectInference::new();
        let row1 = EffectRow::closed(vec![Effect::simple("IO")]);
        let row2 = EffectRow::closed(vec![Effect::simple("Console")]);

        assert!(inference.unify(&row1, &row2, Span::default()).is_err());
    }

    #[test]
    fn test_unify_var_with_closed() {
        let mut inference = EffectInference::new();
        let var = inference.fresh_var();
        let row1 = EffectRow::Var(var);
        let row2 = EffectRow::closed(vec![Effect::simple("IO")]);

        assert!(inference.unify(&row1, &row2, Span::default()).is_ok());

        let resolved = inference.apply(&row1);
        assert_eq!(resolved, row2);
    }

    #[test]
    fn test_unify_open_with_closed() {
        let mut inference = EffectInference::new();
        let var = inference.fresh_var();

        // {IO | e} ~ {IO, Console}
        let row1 = EffectRow::open(vec![Effect::simple("IO")], var);
        let row2 = EffectRow::closed(vec![
            Effect::simple("IO"),
            Effect::simple("Console"),
        ]);

        assert!(inference.unify(&row1, &row2, Span::default()).is_ok());

        // e should be {Console}
        let resolved = inference.apply(&EffectRow::Var(var));
        match resolved {
            EffectRow::Closed(effects) => {
                assert!(effects.contains(&Effect::simple("Console")));
                assert!(!effects.contains(&Effect::simple("IO")));
            }
            _ => panic!("Expected closed row"),
        }
    }

    #[test]
    fn test_unify_open_with_open() {
        let mut inference = EffectInference::new();
        let var1 = inference.fresh_var();
        let var2 = inference.fresh_var();

        // {IO | e1} ~ {Console | e2}
        let row1 = EffectRow::open(vec![Effect::simple("IO")], var1);
        let row2 = EffectRow::open(vec![Effect::simple("Console")], var2);

        assert!(inference.unify(&row1, &row2, Span::default()).is_ok());
    }

    #[test]
    fn test_unify_occurs_check() {
        let mut inference = EffectInference::new();
        let var = inference.fresh_var();

        // e ~ {IO | e} should fail (occurs check)
        let row1 = EffectRow::Var(var);
        let row2 = EffectRow::open(vec![Effect::simple("IO")], var);

        assert!(inference.unify(&row1, &row2, Span::default()).is_err());
    }

    // ------------------------------------------------------------------------
    // Effect Environment Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_effect_env_stdlib() {
        let env = EffectEnv::with_stdlib();

        assert!(env.contains("IO"));
        assert!(env.contains("Console"));
        assert!(env.contains("Exception"));
        assert!(env.contains("Async"));
        assert!(env.contains("State"));
        assert!(env.contains("Reader"));
        assert!(env.contains("Choice"));

        let io = env.lookup("IO").unwrap();
        assert_eq!(io.kind, EffectKind::TailResumptive);

        let exception = env.lookup("Exception").unwrap();
        assert_eq!(exception.kind, EffectKind::OneShot);
        assert_eq!(exception.type_params, vec!["E"]);

        let choice = env.lookup("Choice").unwrap();
        assert_eq!(choice.kind, EffectKind::MultiShot);
    }

    #[test]
    fn test_effect_env_custom() {
        let mut env = EffectEnv::new();

        let decl = EffectDecl::new("MyEffect", Span::default())
            .with_type_param("T")
            .with_kind(EffectKind::OneShot);

        env.define(decl);

        let lookup = env.lookup("MyEffect").unwrap();
        assert_eq!(lookup.type_params, vec!["T"]);
        assert_eq!(lookup.kind, EffectKind::OneShot);
    }

    // ------------------------------------------------------------------------
    // Handler Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_handler_effect_elimination() {
        let handler = Handler::new()
            .with_clause(HandlerClause {
                effect: "Console".to_string(),
                operation: "print".to_string(),
                params: vec!["msg".to_string()],
                body: "resume(Unit)".to_string(),
            });

        let input = EffectRow::closed(vec![
            Effect::simple("IO"),
            Effect::simple("Console"),
        ]);

        let output = handler.resulting_effects(&input);

        assert!(output.contains(&Effect::simple("IO")));
        assert!(!output.contains(&Effect::simple("Console")));
    }

    // ------------------------------------------------------------------------
    // EffectfulType Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_effectful_type_pure() {
        let ty = EffectfulType::pure("Int");
        assert!(ty.is_pure());
        assert_eq!(format!("{}", ty), "Int");
    }

    #[test]
    fn test_effectful_type_with_effects() {
        let ty = EffectfulType::with_effects(
            "String",
            EffectRow::closed(vec![Effect::simple("IO")]),
        );
        assert!(!ty.is_pure());
        assert_eq!(format!("{}", ty), "String !{IO}");
    }

    // ------------------------------------------------------------------------
    // Generalization and Instantiation Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_generalize_closed() {
        let inference = EffectInference::new();
        let row = EffectRow::closed(vec![Effect::simple("IO")]);

        let (generalized, bound) = inference.generalize(&row);

        assert!(bound.is_empty());
        assert_eq!(generalized, row);
    }

    #[test]
    fn test_generalize_open() {
        let mut inference = EffectInference::new();
        let var = inference.fresh_var();
        let row = EffectRow::open(vec![Effect::simple("IO")], var);

        let (generalized, bound) = inference.generalize(&row);

        assert_eq!(bound.len(), 1);
        assert_eq!(bound[0], var);
        assert_eq!(generalized, row);
    }

    #[test]
    fn test_instantiate() {
        let mut inference = EffectInference::new();
        // Use fresh_var to get the first variable (0)
        let var = inference.fresh_var();
        let row = EffectRow::open(vec![Effect::simple("IO")], var);
        let bound = vec![var];

        let instantiated = inference.instantiate(&row, &bound);

        // Should have a fresh variable (different from original)
        match instantiated {
            EffectRow::Open { effects, tail } => {
                assert!(effects.contains(&Effect::simple("IO")));
                // The instantiated variable should be different from the original
                assert_ne!(tail, var, "Instantiation should create fresh variables");
            }
            _ => panic!("Expected open row"),
        }
    }

    // ------------------------------------------------------------------------
    // Integration Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_effect_inference_scenario() {
        // Simulate inferring effects for a function that calls IO and Console
        let mut inference = EffectInference::new();
        let _env = EffectEnv::with_stdlib();

        // Start with a fresh row variable
        let func_effects = inference.fresh_row();

        // After calling IO.read, we know the function has IO effect
        let io_effect = EffectRow::single(Effect::simple("IO"));
        inference.unify(&func_effects, &io_effect.clone().with_effect(
            Effect::simple("__temp__")  // Placeholder for the rest
        ).without_effect(&Effect::simple("__temp__")), Span::default()).ok();

        // This demonstrates the flow of effect inference
        let resolved = inference.resolve(&func_effects);
        assert!(resolved.contains(&Effect::simple("IO")));
    }

    // ------------------------------------------------------------------------
    // Async Effect and Concurrency Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_async_effect_in_stdlib() {
        let env = EffectEnv::with_stdlib();

        // Async effect should be defined in stdlib
        assert!(env.contains("Async"));

        let async_effect = env.lookup("Async").unwrap();
        assert_eq!(async_effect.name, "Async");

        // Async is a OneShot effect (can suspend but resume at most once)
        assert_eq!(async_effect.kind, EffectKind::OneShot);

        // Should have spawn, await, yield, scope, supervisor, and timeout operations
        assert_eq!(async_effect.operations.len(), 6);

        let op_names: Vec<_> = async_effect.operations.iter().map(|op| op.name.as_str()).collect();
        assert!(op_names.contains(&"await"));
        assert!(op_names.contains(&"spawn"));
        assert!(op_names.contains(&"yield"));
        assert!(op_names.contains(&"scope"));
        assert!(op_names.contains(&"supervisor"));
    }

    #[test]
    fn test_async_effect_row() {
        // Test creating a function with Async effect
        let async_effect = Effect::with_kind("Async", EffectKind::OneShot);
        let row = EffectRow::single(async_effect.clone());

        assert!(!row.is_pure());
        assert!(row.contains(&async_effect));

        // Async functions can also have IO
        let io_effect = Effect::simple("IO");
        let combined = row.with_effect(io_effect.clone());

        assert!(combined.contains(&async_effect));
        assert!(combined.contains(&io_effect));
    }

    #[test]
    fn test_async_effect_dominant_kind() {
        // When combining effects, the dominant kind determines compilation strategy
        let async_effect = Effect::with_kind("Async", EffectKind::OneShot);
        let io_effect = Effect::with_kind("IO", EffectKind::TailResumptive);

        let effects = EffectSet::from_effects(vec![async_effect, io_effect]);

        // OneShot dominates TailResumptive
        assert_eq!(effects.dominant_kind(), EffectKind::OneShot);
    }

    #[test]
    fn test_async_with_exception_effect() {
        // Common pattern: Async functions that may throw exceptions
        let async_effect = Effect::with_kind("Async", EffectKind::OneShot);
        let exception_effect = Effect::parameterized_with_kind(
            "Exception",
            vec![EffectTypeArg::Concrete("Error".to_string())],
            EffectKind::OneShot,
        );

        let row = EffectRow::closed(vec![async_effect.clone(), exception_effect.clone()]);

        assert!(row.contains(&async_effect));
        assert!(row.contains(&exception_effect));

        // Both are OneShot
        assert_eq!(row.known_effects().dominant_kind(), EffectKind::OneShot);
    }

    #[test]
    fn test_async_handler_elimination() {
        // When we handle the Async effect, it should be removed from the row
        let async_effect = Effect::with_kind("Async", EffectKind::OneShot);
        let io_effect = Effect::simple("IO");

        let row = EffectRow::closed(vec![async_effect.clone(), io_effect.clone()]);

        let handler = Handler::new()
            .with_clause(HandlerClause {
                effect: "Async".to_string(),
                operation: "spawn".to_string(),
                params: vec!["f".to_string()],
                body: "/* spawn implementation */".to_string(),
            });

        // After handling Async, only IO should remain
        let resulting = handler.resulting_effects(&row);
        assert!(!resulting.contains(&async_effect));
        assert!(resulting.contains(&io_effect));
    }

    #[test]
    fn test_async_effect_inference() {
        let mut inference = EffectInference::new();

        // Infer effects for a function that spawns a task
        let func_row = inference.fresh_row();

        // After calling spawn, we know the function has Async effect
        let async_row = EffectRow::single(Effect::with_kind("Async", EffectKind::OneShot));
        assert!(inference.unify(&func_row, &async_row, Span::default()).is_ok());

        let resolved = inference.resolve(&func_row);
        assert!(resolved.contains(&Effect::with_kind("Async", EffectKind::OneShot)));
    }

    #[test]
    fn test_effect_polymorphic_spawn() {
        // Test effect polymorphism: spawn should preserve other effects
        //
        // fn spawn[E](f: () -> T !{Async | E}) -> Task[T] !{Async | E}
        //
        // The spawned function's effects (other than Async) are preserved
        let mut inference = EffectInference::new();

        let spawned_effects = inference.fresh_var();
        let io_effect = Effect::simple("IO");

        // spawned function has {Async, IO | e}
        let spawned_row = EffectRow::open(
            vec![
                Effect::with_kind("Async", EffectKind::OneShot),
                io_effect.clone(),
            ],
            spawned_effects,
        );

        // Resulting effect row preserves the IO effect
        assert!(spawned_row.contains(&Effect::with_kind("Async", EffectKind::OneShot)));
        assert!(spawned_row.contains(&io_effect));
    }

    #[test]
    fn test_pure_function_cannot_spawn() {
        // A pure function cannot use Async.spawn
        let pure_row = EffectRow::pure();

        let async_effect = Effect::with_kind("Async", EffectKind::OneShot);

        // Pure row should not contain Async
        assert!(!pure_row.contains(&async_effect));
        assert!(pure_row.is_pure());
    }

    #[test]
    fn test_effectful_type_with_async() {
        // Test EffectfulType representing an async function
        let async_row = EffectRow::single(Effect::with_kind("Async", EffectKind::OneShot));
        let ty = EffectfulType::with_effects("Task[Int]", async_row);

        assert!(!ty.is_pure());
        assert_eq!(ty.value_type, "Task[Int]");

        let display = format!("{}", ty);
        assert!(display.contains("Task[Int]"));
        assert!(display.contains("Async"));
    }

    // =========================================================================
    // Channel Effect Tests
    // =========================================================================

    #[test]
    fn test_channel_effect_in_stdlib() {
        let env = EffectEnv::with_stdlib();

        // Channel effect should be defined
        assert!(env.contains("Channel"));

        let channel_decl = env.lookup("Channel").unwrap();
        assert_eq!(channel_decl.name, "Channel");
        assert_eq!(channel_decl.type_params, vec!["T".to_string()]);
        assert_eq!(channel_decl.kind, EffectKind::OneShot);
    }

    #[test]
    fn test_channel_effect_operations() {
        let env = EffectEnv::with_stdlib();
        let channel_decl = env.lookup("Channel").unwrap();

        // Check operations exist
        let op_names: Vec<&str> = channel_decl.operations.iter().map(|op| op.name.as_str()).collect();
        assert!(op_names.contains(&"new"));
        assert!(op_names.contains(&"send"));
        assert!(op_names.contains(&"recv"));
        assert!(op_names.contains(&"try_send"));
        assert!(op_names.contains(&"try_recv"));
        assert!(op_names.contains(&"close"));
    }

    #[test]
    fn test_channel_effect_row() {
        // Create a channel effect with type argument
        let channel_effect = Effect::parameterized_with_kind(
            "Channel",
            vec![EffectTypeArg::Concrete("Int".to_string())],
            EffectKind::OneShot,
        );

        let row = EffectRow::single(channel_effect.clone());

        assert!(!row.is_pure());
        assert!(row.contains(&channel_effect));
    }

    #[test]
    fn test_channel_with_async_effect() {
        // Common pattern: using channels within async code
        let channel_effect = Effect::parameterized_with_kind(
            "Channel",
            vec![EffectTypeArg::Concrete("String".to_string())],
            EffectKind::OneShot,
        );
        let async_effect = Effect::with_kind("Async", EffectKind::OneShot);

        let row = EffectRow::closed(vec![channel_effect.clone(), async_effect.clone()]);

        assert!(row.contains(&channel_effect));
        assert!(row.contains(&async_effect));

        // Both are OneShot
        assert_eq!(row.known_effects().dominant_kind(), EffectKind::OneShot);
    }

    #[test]
    fn test_channel_handler_elimination() {
        // When we handle the Channel effect, it should be removed from the row
        let channel_effect = Effect::with_kind("Channel", EffectKind::OneShot);
        let io_effect = Effect::simple("IO");

        let row = EffectRow::closed(vec![channel_effect.clone(), io_effect.clone()]);

        let handler = Handler::new()
            .with_clause(HandlerClause {
                effect: "Channel".to_string(),
                operation: "send".to_string(),
                params: vec!["ch".to_string(), "value".to_string()],
                body: "/* send implementation */".to_string(),
            });

        // After handling Channel, only IO should remain
        let resulting = handler.resulting_effects(&row);
        assert!(!resulting.contains(&channel_effect));
        assert!(resulting.contains(&io_effect));
    }

    #[test]
    fn test_channel_effect_display() {
        // Test that Channel effect displays correctly
        let channel_effect = Effect::parameterized(
            "Channel",
            vec![EffectTypeArg::Concrete("Int".to_string())],
        );

        let display = format!("{}", channel_effect);
        assert!(display.contains("Channel"));
        assert!(display.contains("Int"));
    }

    #[test]
    fn test_effectful_type_with_channel() {
        // Test EffectfulType representing a function that uses channels
        let channel_row = EffectRow::single(Effect::parameterized_with_kind(
            "Channel",
            vec![EffectTypeArg::Concrete("Message".to_string())],
            EffectKind::OneShot,
        ));
        let ty = EffectfulType::with_effects("Unit", channel_row);

        assert!(!ty.is_pure());
        assert_eq!(ty.value_type, "Unit");
    }

    #[test]
    fn test_channel_producer_consumer_pattern() {
        // Model a producer-consumer pattern with channels
        let channel_int = Effect::parameterized_with_kind(
            "Channel",
            vec![EffectTypeArg::Concrete("Int".to_string())],
            EffectKind::OneShot,
        );
        let async_effect = Effect::with_kind("Async", EffectKind::OneShot);

        // Producer function: sends values on channel
        let producer_effects = EffectRow::closed(vec![channel_int.clone()]);
        assert!(!producer_effects.is_pure());

        // Consumer function: receives values from channel
        let consumer_effects = EffectRow::closed(vec![channel_int.clone()]);
        assert!(!consumer_effects.is_pure());

        // Main function spawns both, so has Async + Channel
        let main_effects = EffectRow::closed(vec![async_effect.clone(), channel_int.clone()]);
        assert!(main_effects.contains(&async_effect));
        assert!(main_effects.contains(&channel_int));
    }

    #[test]
    fn test_cancel_effect_in_stdlib() {
        let env = EffectEnv::with_stdlib();

        // Cancel effect should be defined
        assert!(env.contains("Cancel"));

        let cancel_decl = env.lookup("Cancel").unwrap();
        assert_eq!(cancel_decl.name, "Cancel");

        // Cancel is TailResumptive (checking cancellation is lightweight)
        assert_eq!(cancel_decl.kind, EffectKind::TailResumptive);

        // Should have check and token operations
        assert_eq!(cancel_decl.operations.len(), 2);

        let op_names: Vec<&str> = cancel_decl.operations.iter().map(|op| op.name.as_str()).collect();
        assert!(op_names.contains(&"check"));
        assert!(op_names.contains(&"token"));
    }

    #[test]
    fn test_async_scope_operations() {
        let env = EffectEnv::with_stdlib();
        let async_decl = env.lookup("Async").unwrap();

        let op_names: Vec<&str> = async_decl.operations.iter().map(|op| op.name.as_str()).collect();

        // scope operation for structured concurrency
        assert!(op_names.contains(&"scope"));

        // supervisor operation for fault-tolerant scopes
        assert!(op_names.contains(&"supervisor"));

        // timeout operation for time-bounded scopes
        assert!(op_names.contains(&"timeout"));
    }
}
