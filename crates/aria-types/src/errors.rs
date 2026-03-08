//! Type error definitions for Aria.
//!
//! Contains the `TypeError` enum covering all type checking errors,
//! and the `TypeResult` type alias.

use crate::TypeSource;
use aria_ast::Span;
use thiserror::Error;

/// Type error
#[derive(Debug, Clone, Error)]
pub enum TypeError {
    #[error("Type mismatch: expected `{expected}`, found `{found}`")]
    Mismatch {
        expected: String,
        found: String,
        span: Span,
        /// Optional context about where the expected type came from
        expected_source: Option<TypeSource>,
    },

    #[error("Undefined type: `{0}`")]
    UndefinedType(String, Span),

    #[error("Undefined variable: `{name}`")]
    UndefinedVariable {
        name: String,
        span: Span,
        /// Optional list of similar names for typo suggestions
        similar_names: Option<Vec<String>>,
    },

    #[error("Cannot infer type")]
    CannotInfer(Span),

    #[error("Recursive type detected")]
    RecursiveType(Span),

    #[error("Wrong number of type arguments: expected {expected}, found {found}")]
    WrongTypeArity {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("Cannot spawn task capturing non-Transfer value `{var_name}` of type `{var_type}`")]
    NonTransferCapture {
        var_name: String,
        var_type: String,
        span: Span,
    },

    #[error("Cannot share non-Sharable value `{var_name}` of type `{var_type}` between tasks")]
    NonSharableShare {
        var_name: String,
        var_type: String,
        span: Span,
    },

    #[error("Cannot mutably capture immutable variable `{var_name}`")]
    MutableCaptureOfImmutable {
        var_name: String,
        span: Span,
    },

    #[error("Cannot mutably capture `{var_name}` in spawn - spawned closures cannot hold mutable borrows")]
    MutableCaptureInSpawn {
        var_name: String,
        span: Span,
    },

    #[error("Type `{ty}` does not implement trait `{trait_name}`")]
    MissingTraitImpl {
        ty: String,
        trait_name: String,
        span: Span,
    },

    #[error("The `?` operator can only be used on Result or Optional types, found `{found}`")]
    InvalidTryOperator {
        found: String,
        span: Span,
    },

    #[error("Cannot use `?` in a function that doesn't return Result or Optional")]
    TryInNonResultFunction {
        function_return: String,
        span: Span,
    },

    #[error("`await` can only be used in async context")]
    AwaitOutsideAsync {
        span: Span,
    },

    #[error("`await` expects a Task type, found `{found}`")]
    AwaitNonTask {
        found: String,
        span: Span,
    },

    #[error("Channel send expects a channel, found `{found}`")]
    SendOnNonChannel {
        found: String,
        span: Span,
    },

    #[error("Channel receive expects a channel, found `{found}`")]
    ReceiveOnNonChannel {
        found: String,
        span: Span,
    },

    #[error("Select expression cannot have multiple default arms")]
    MultipleDefaultArms {
        first_span: Span,
        second_span: Span,
    },

    #[error("Select arm result type mismatch: expected `{expected}`, found `{found}`")]
    SelectArmTypeMismatch {
        expected: String,
        found: String,
        arm_index: usize,
        span: Span,
    },

    // ============================================================================
    // Effect System Errors
    // ============================================================================

    #[error("Effect `{effect}` not declared in function signature")]
    UndeclaredEffect {
        effect: String,
        function_name: String,
        span: Span,
    },

    #[error("Cannot perform effect `{effect}` without a handler in scope")]
    UnhandledEffect {
        effect: String,
        span: Span,
    },

    #[error("Effect handler for `{effect}` has wrong type: expected `{expected}`, found `{found}`")]
    EffectHandlerTypeMismatch {
        effect: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Effect row mismatch: expected `{expected}`, found `{found}`")]
    EffectRowMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Cannot call function with effects `{callee_effects}` from pure context")]
    EffectfulCallInPureContext {
        callee_effects: String,
        span: Span,
    },

    #[error("Effect `{effect}` is not defined")]
    UndefinedEffect {
        effect: String,
        span: Span,
    },

    #[error("Duplicate effect declaration: `{effect}`")]
    DuplicateEffectDeclaration {
        effect: String,
        span: Span,
    },

    #[error("Resume outside of effect handler")]
    ResumeOutsideHandler {
        span: Span,
    },

    #[error("Resume type mismatch: expected `{expected}`, found `{found}`")]
    ResumeTypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Module not found: {0}")]
    ModuleNotFound(String, Span),

    #[error("Symbol `{symbol}` is not exported from module `{module}`")]
    ImportNotExported {
        symbol: String,
        module: String,
        span: Span,
    },

    #[error("Unresolved import: `{symbol}` from module `{module}`")]
    UnresolvedImport {
        symbol: String,
        module: String,
        span: Span,
    },

    #[error("Non-exhaustive patterns: {missing}")]
    NonExhaustivePatterns {
        missing: String,
        span: Span,
    },

    #[error("Unreachable pattern")]
    UnreachablePattern {
        span: Span,
    },

    #[error("Type `{type_name}` has no field `{field_name}`")]
    UndefinedField {
        type_name: String,
        field_name: String,
        span: Span,
    },

    #[error("Cannot access field on non-struct type `{type_name}`")]
    FieldAccessOnNonStruct {
        type_name: String,
        span: Span,
    },

    #[error("Return type mismatch: expected `{expected}`, found `{found}`")]
    ReturnTypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    // ============================================================================
    // FFI Type Errors
    // ============================================================================

    #[error("Invalid C type in FFI declaration: `{c_type}` - {reason}")]
    InvalidFfiCType {
        c_type: String,
        reason: String,
        span: Span,
    },

    #[error("Extern function `{func_name}` parameter `{param_name}` uses non-FFI-safe type")]
    NonFfiSafeParameter {
        func_name: String,
        param_name: String,
        span: Span,
    },

    #[error("Extern function `{func_name}` returns non-FFI-safe type")]
    NonFfiSafeReturn {
        func_name: String,
        span: Span,
    },

    #[error("Extern struct `{struct_name}` field `{field_name}` uses non-FFI-safe type")]
    NonFfiSafeField {
        struct_name: String,
        field_name: String,
        span: Span,
    },

    #[error("Duplicate extern declaration: `{name}` was already declared")]
    DuplicateExternDeclaration {
        name: String,
        span: Span,
    },

    #[error("Missing type annotation on extern function parameter")]
    MissingExternParamType {
        func_name: String,
        span: Span,
    },

    #[error("Variadic functions are not supported in FFI: `{func_name}`")]
    VariadicFfiFunction {
        func_name: String,
        span: Span,
    },

    // ============================================================================
    // Tuple Type Errors
    // ============================================================================

    #[error("Tuple index {index} out of bounds for tuple of length {length}")]
    TupleIndexOutOfBounds {
        index: usize,
        length: usize,
        span: Span,
    },

    #[error("Cannot convert tuple to array: elements have different types ({types})")]
    TupleToArrayHeterogeneousTypes {
        types: String,
        span: Span,
    },

    // ============================================================================
    // Generic Type Bound Errors
    // ============================================================================

    #[error("Type `{ty}` does not implement trait `{trait_name}`")]
    TraitNotImplemented {
        ty: String,
        trait_name: String,
        span: Span,
    },

    #[error("Type argument `{type_arg}` does not satisfy bound `{bound}` for type parameter `{param}`")]
    BoundNotSatisfied {
        type_arg: String,
        param: String,
        bound: String,
        span: Span,
    },

    #[error("Undefined trait: `{0}`")]
    UndefinedTrait(String, Span),

    #[error("Trait `{trait_name}` expects {expected} type argument(s), found {found}")]
    WrongTraitArity {
        trait_name: String,
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("Conflicting implementations of trait `{trait_name}` for type `{for_type}`")]
    ConflictingImpl {
        trait_name: String,
        for_type: String,
        span: Span,
    },

    #[error("Where clause constraint `{constraint}` not satisfied")]
    WhereClauseNotSatisfied {
        constraint: String,
        span: Span,
    },

    // ============================================================================
    // Trait Implementation Errors
    // ============================================================================

    #[error("Missing method `{method_name}` in implementation of trait `{trait_name}`")]
    MissingTraitMethod {
        trait_name: String,
        method_name: String,
        span: Span,
    },

    #[error("Method `{method_name}` has wrong signature: expected `{expected}`, found `{found}`")]
    TraitMethodSignatureMismatch {
        method_name: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Missing associated type `{type_name}` in implementation of trait `{trait_name}`")]
    MissingAssociatedType {
        trait_name: String,
        type_name: String,
        span: Span,
    },

    #[error("Method `{method_name}` is not a member of trait `{trait_name}`")]
    MethodNotInTrait {
        trait_name: String,
        method_name: String,
        span: Span,
    },

    #[error("Associated type `{type_name}` is not defined in trait `{trait_name}`")]
    AssociatedTypeNotInTrait {
        trait_name: String,
        type_name: String,
        span: Span,
    },

    #[error("Trait `{trait_name}` requires implementing supertrait `{supertrait_name}`")]
    SupertraitNotImplemented {
        trait_name: String,
        supertrait_name: String,
        for_type: String,
        span: Span,
    },

    #[error("Self type mismatch in impl block: method `{method_name}` expects Self to be `{expected}`, found `{found}`")]
    SelfTypeMismatch {
        method_name: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Duplicate method `{method_name}` in impl block")]
    DuplicateImplMethod {
        method_name: String,
        span: Span,
    },

    #[error("Duplicate associated type `{type_name}` in impl block")]
    DuplicateAssociatedType {
        type_name: String,
        span: Span,
    },

    // ============================================================================
    // Const Expression Evaluation Errors
    // ============================================================================

    #[error("Expression is not a compile-time constant")]
    NotConstant {
        reason: String,
        span: Span,
    },

    #[error("Const evaluation error: {reason}")]
    ConstEvalError {
        reason: String,
        span: Span,
    },

    #[error("Integer overflow in const evaluation")]
    ConstOverflow {
        span: Span,
    },

    #[error("Division by zero in const evaluation")]
    ConstDivisionByZero {
        span: Span,
    },

    #[error("Undefined constant: {0}")]
    UndefinedConstant(String, Span),

    // ============================================================================
    // Defer Statement Errors
    // ============================================================================

    #[error("Deferred expression should return Unit, found `{found}`")]
    DeferNonUnit {
        found: String,
        span: Span,
    },

    #[error("Control flow statement `{statement}` cannot be used inside defer")]
    ControlFlowInDefer {
        statement: String,
        span: Span,
    },

    #[error("Variable `{var_name}` may not be valid when defer executes")]
    DeferCaptureInvalid {
        var_name: String,
        defer_span: Span,
        var_span: Span,
    },

    #[error("Cannot use `await` inside defer block")]
    AwaitInDefer {
        span: Span,
    },

    // ============================================================================
    // Default Parameter Errors
    // ============================================================================

    #[error("Default value type mismatch: parameter `{param_name}` has type `{param_type}`, but default value has type `{default_type}`")]
    DefaultValueTypeMismatch {
        param_name: String,
        param_type: String,
        default_type: String,
        span: Span,
    },

    #[error("Parameters with default values must come after required parameters: `{param_name}`")]
    DefaultAfterRequired {
        param_name: String,
        span: Span,
    },

    #[error("Too few arguments: expected at least {min_required} arguments, found {found}")]
    TooFewArguments {
        min_required: usize,
        found: usize,
        span: Span,
    },

    #[error("Too many arguments: expected at most {max_allowed} arguments, found {found}")]
    TooManyArguments {
        max_allowed: usize,
        found: usize,
        span: Span,
    },

    #[error("Named argument `{name}` does not match any parameter")]
    UnknownNamedArgument {
        name: String,
        span: Span,
    },

    #[error("Duplicate named argument: `{name}`")]
    DuplicateNamedArgument {
        name: String,
        span: Span,
    },

    #[error("Missing required argument: `{name}`")]
    MissingRequiredArgument {
        name: String,
        span: Span,
    },

    #[error("Positional arguments cannot follow named arguments")]
    PositionalAfterNamed {
        span: Span,
    },

    // ============================================================================
    // Spread Operator Errors
    // ============================================================================

    #[error("Spread operator requires an array type, found `{found}`")]
    SpreadOnNonArray {
        found: String,
        span: Span,
    },

    #[error("Spread argument element type `{spread_elem_type}` is not compatible with parameter type `{param_type}`")]
    SpreadElementTypeMismatch {
        spread_elem_type: String,
        param_type: String,
        span: Span,
    },

    #[error("Spread in array literal: element type `{spread_elem_type}` is not compatible with array element type `{array_elem_type}`")]
    SpreadArrayElementMismatch {
        spread_elem_type: String,
        array_elem_type: String,
        span: Span,
    },

    #[error("Spread in struct: source type `{source_type}` is not compatible with target struct `{target_struct}`")]
    SpreadStructTypeMismatch {
        source_type: String,
        target_struct: String,
        span: Span,
    },

    #[error("Cannot spread non-struct type `{found}` in struct initializer")]
    SpreadOnNonStruct {
        found: String,
        span: Span,
    },

    // ============================================================================
    // Loop Control Flow Errors
    // ============================================================================

    #[error("`break` cannot be used outside of a loop")]
    BreakOutsideLoop {
        span: Span,
    },

    #[error("`continue` cannot be used outside of a loop")]
    ContinueOutsideLoop {
        span: Span,
    },

    #[error("Break value type mismatch: expected `{expected}`, found `{found}`")]
    BreakTypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Type `{found}` is not iterable")]
    NotIterable {
        found: String,
        span: Span,
    },
}

/// Type result
pub type TypeResult<T> = Result<T, TypeError>;
