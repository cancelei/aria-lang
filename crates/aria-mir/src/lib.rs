//! Mid-level Intermediate Representation (MIR) for the Aria programming language.
//!
//! MIR is a CFG-based intermediate representation that sits between the AST
//! and code generation. It provides:
//!
//! - **Explicit control flow**: All control flow is represented as basic blocks
//!   with explicit terminators (goto, switch, call, return)
//! - **Simplified expressions**: Complex expressions are broken down into
//!   simple operations on places and operands
//! - **Ownership tracking**: Move vs copy semantics are made explicit
//! - **Type-resolved representation**: All types are fully resolved
//!
//! # Architecture
//!
//! ```text
//! AST → [lower_program] → MIR → [optimize] → MIR → Codegen
//! ```
//!
//! # Example
//!
//! ```ignore
//! use aria_mir::{lower_program, MirProgram};
//! use aria_parser::parse;
//!
//! let source = "fn main() { let x = 42; print(x); }";
//! let (ast, _) = parse(source);
//! let mir = lower_program(&ast)?;
//! ```

use aria_lexer::Span;
use thiserror::Error;

pub mod mir;
mod lower;
mod lower_expr;
mod lower_stmt;
mod lower_pattern;
pub mod optimize;
mod pretty;
pub mod contract_verifier;

// Re-export main types
pub use mir::{
    // Program structure
    MirProgram,
    MirFunction,
    MirStruct,
    MirEnum,
    MirField,
    MirVariant,
    Linkage,
    BuiltinKind,

    // Identifiers
    FunctionId,
    StructId,
    EnumId,
    Local,
    BlockId,

    // Basic blocks
    BasicBlock,
    Statement,
    StatementKind,
    Terminator,
    TerminatorKind,
    SwitchTargets,

    // Places and values
    Place,
    PlaceElem,
    Rvalue,
    Operand,
    Constant,

    // Operations
    BinOp,
    UnOp,
    AggregateKind,
    CastKind,

    // Types
    MirType,
    LocalDecl,
    TypeVarId,

    // Effect system types
    EffectId,
    OperationId,
    HandlerId,
    ContinuationId,
    EffectType,
    EffectRow,
    EvidenceSlot,
    EffectClassification,
    FfiBarrierStrategy,

    // Effect definitions
    EffectOperation,
    MirEffect,
    MirHandler,
    HandlerBlock,

    // Effect statements and terminators
    EffectStatementKind,
    EffectTerminatorKind,

    // Evidence
    EvidenceLayout,
    EvidenceParam,
};

pub use lower::LoweringContext;
pub use pretty::pretty_print;

/// Errors that can occur during MIR lowering
#[derive(Error, Debug, Clone)]
pub enum MirError {
    #[error("unsupported feature: {feature}")]
    UnsupportedFeature { feature: String, span: Span },

    #[error("cannot find variable `{name}` in this scope")]
    UndefinedVariable { name: String, span: Span },

    #[error("cannot find function `{name}` in this scope")]
    UndefinedFunction { name: String, span: Span },

    #[error("cannot find type `{name}` in this scope")]
    UndefinedType { name: String, span: Span },

    #[error("no field `{field_name}` on type `{struct_name}`")]
    UndefinedField {
        struct_name: String,
        field_name: String,
        span: Span,
    },

    #[error("mismatched types: expected `{expected}`, found `{found}`")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("invalid pattern: cannot use this pattern in a binding context")]
    InvalidPattern { span: Span },

    #[error("internal compiler error: {message}")]
    Internal { message: String, span: Span },
}

impl MirError {
    pub fn span(&self) -> Span {
        match self {
            MirError::UnsupportedFeature { span, .. } => *span,
            MirError::UndefinedVariable { span, .. } => *span,
            MirError::UndefinedFunction { span, .. } => *span,
            MirError::UndefinedType { span, .. } => *span,
            MirError::UndefinedField { span, .. } => *span,
            MirError::TypeMismatch { span, .. } => *span,
            MirError::InvalidPattern { span } => *span,
            MirError::Internal { span, .. } => *span,
        }
    }
}

/// Result type for MIR operations
pub type Result<T> = std::result::Result<T, MirError>;

// Re-export optimization level from optimize module
pub use optimize::OptLevel;

/// Lower an AST program to MIR.
///
/// This is the main entry point for MIR lowering. It takes a type-checked
/// AST and produces a MIR program suitable for code generation.
///
/// # Arguments
///
/// * `program` - The AST program to lower
///
/// # Returns
///
/// A `MirProgram` on success, or a `MirError` if lowering fails.
///
/// # Example
///
/// ```ignore
/// let mir = lower_program(&ast)?;
/// println!("{}", pretty_print(&mir));
/// ```
pub fn lower_program(program: &aria_ast::Program) -> Result<MirProgram> {
    let mut ctx = LoweringContext::new();
    ctx.lower_program(program)
}

/// Apply optimization passes to a MIR program.
///
/// # Arguments
///
/// * `mir` - The MIR program to optimize (modified in place)
/// * `level` - The optimization level
///
/// # Optimization Passes
///
/// - `None`: No optimizations applied
/// - `Basic`: Constant folding, dead code elimination, CFG simplification
/// - `Aggressive`: All basic + copy propagation, multiple iterations
pub fn optimize(mir: &mut MirProgram, level: OptLevel) {
    optimize::optimize_program(mir, level);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mir_program_creation() {
        let program = MirProgram::new();
        assert!(program.functions.is_empty());
        assert!(program.structs.is_empty());
        assert!(program.enums.is_empty());
        assert!(program.entry.is_none());
    }

    #[test]
    fn test_mir_function_creation() {
        let mut func = MirFunction::new(
            "test".into(),
            MirType::Unit,
            Span::dummy(),
        );

        // Local 0 should be the return place
        assert_eq!(func.locals.len(), 1);
        assert_eq!(func.local_decl(Local::RETURN).ty, MirType::Unit);

        // Create a new local
        let local = func.new_local(MirType::Int, Some("x".into()));
        assert_eq!(local.0, 1);
        assert_eq!(func.locals.len(), 2);

        // Create a basic block
        let block = func.new_block();
        assert_eq!(block, BlockId::ENTRY);
        assert_eq!(func.blocks.len(), 1);
    }

    #[test]
    fn test_place_construction() {
        let place = Place::from_local(Local(1))
            .field(0)
            .field(1);

        assert_eq!(place.local, Local(1));
        assert_eq!(place.projection.len(), 2);
    }

    #[test]
    fn test_switch_targets() {
        let targets = SwitchTargets::if_else(BlockId(1), BlockId(2));
        assert_eq!(targets.targets.len(), 1);
        assert_eq!(targets.targets[0], (0, BlockId(2)));
        assert_eq!(targets.otherwise, BlockId(1));
    }

    #[test]
    fn test_operand_helpers() {
        let int_op = Operand::const_int(42);
        let bool_op = Operand::const_bool(true);
        let float_op = Operand::const_float(3.14);
        let unit_op = Operand::const_unit();

        match int_op {
            Operand::Constant(Constant::Int(42)) => {}
            _ => panic!("Expected Int constant"),
        }

        match bool_op {
            Operand::Constant(Constant::Bool(true)) => {}
            _ => panic!("Expected Bool constant"),
        }

        match float_op {
            Operand::Constant(Constant::Float(f)) if (f - 3.14).abs() < 0.001 => {}
            _ => panic!("Expected Float constant"),
        }

        match unit_op {
            Operand::Constant(Constant::Unit) => {}
            _ => panic!("Expected Unit constant"),
        }
    }

    #[test]
    fn test_mir_type_predicates() {
        assert!(MirType::Int.is_primitive());
        assert!(MirType::Bool.is_primitive());
        assert!(!MirType::String.is_primitive());
        assert!(!MirType::Array(Box::new(MirType::Int)).is_primitive());

        assert!(MirType::Int.is_integer());
        assert!(MirType::Int64.is_integer());
        assert!(MirType::UInt32.is_integer());
        assert!(!MirType::Float.is_integer());

        assert!(MirType::Float.is_float());
        assert!(MirType::Float64.is_float());
        assert!(!MirType::Int.is_float());
    }

    // ========================================================================
    // Effect System Tests
    // ========================================================================

    #[test]
    fn test_effect_type_creation() {
        let effect = EffectType::new(EffectId(0), "State".into());
        assert_eq!(effect.id, EffectId(0));
        assert_eq!(effect.name.as_str(), "State");
        assert!(effect.type_params.is_empty());
    }

    #[test]
    fn test_effect_type_with_params() {
        let effect = EffectType::new(EffectId(1), "Reader".into())
            .with_type_params(vec![MirType::Int]);
        assert_eq!(effect.type_params.len(), 1);
        assert_eq!(effect.type_params[0], MirType::Int);
    }

    #[test]
    fn test_effect_row_pure() {
        let row = EffectRow::pure();
        assert!(row.is_pure());
        assert!(row.effects.is_empty());
        assert!(!row.is_open);
    }

    #[test]
    fn test_effect_row_with_effects() {
        let state = EffectType::new(EffectId(0), "State".into());
        let io = EffectType::new(EffectId(1), "IO".into());

        let row = EffectRow::new()
            .with_effect(state.clone())
            .with_effect(io.clone());

        assert!(!row.is_pure());
        assert_eq!(row.effects.len(), 2);
        assert_eq!(row.effects[0].name.as_str(), "State");
        assert_eq!(row.effects[1].name.as_str(), "IO");
    }

    #[test]
    fn test_effect_row_open() {
        let row = EffectRow::new().open();
        assert!(!row.is_pure()); // Open row is not pure
        assert!(row.is_open);
    }

    #[test]
    fn test_evidence_slot_static() {
        let slot = EvidenceSlot::Static(5);
        match slot {
            EvidenceSlot::Static(n) => assert_eq!(n, 5),
            _ => panic!("Expected static slot"),
        }
    }

    #[test]
    fn test_evidence_slot_dynamic() {
        let slot = EvidenceSlot::Dynamic(Local(3));
        match slot {
            EvidenceSlot::Dynamic(local) => assert_eq!(local, Local(3)),
            _ => panic!("Expected dynamic slot"),
        }
    }

    #[test]
    fn test_evidence_layout() {
        let mut layout = EvidenceLayout::new();
        assert_eq!(layout.size, 0);

        let slot1 = layout.add_effect(EffectId(0));
        assert_eq!(slot1, 0);
        assert_eq!(layout.size, 1);

        let slot2 = layout.add_effect(EffectId(1));
        assert_eq!(slot2, 1);
        assert_eq!(layout.size, 2);

        // Adding same effect again should return same slot
        let slot1_again = layout.add_effect(EffectId(0));
        assert_eq!(slot1_again, 0);
        assert_eq!(layout.size, 2);
    }

    #[test]
    fn test_mir_program_effects() {
        let mut program = MirProgram::new();

        // Create an effect
        let effect_id = program.new_effect("State".into(), Span::dummy());
        assert_eq!(effect_id, EffectId(0));

        // Get the effect back
        let effect = program.effect(effect_id).unwrap();
        assert_eq!(effect.name.as_str(), "State");

        // Add operations to the effect
        let effect_mut = program.effect_mut(effect_id).unwrap();
        let op_id = effect_mut.add_operation(EffectOperation {
            id: OperationId(0),
            name: "get".into(),
            params: vec![],
            return_ty: MirType::Int,
        });
        assert_eq!(op_id, OperationId(0));

        let effect = program.effect(effect_id).unwrap();
        assert_eq!(effect.operations.len(), 1);
        assert_eq!(effect.operation(OperationId(0)).unwrap().name.as_str(), "get");
    }

    #[test]
    fn test_mir_program_handlers() {
        let mut program = MirProgram::new();

        let effect_type = EffectType::new(EffectId(0), "State".into());
        let handler_id = program.new_handler(effect_type.clone(), Span::dummy());
        assert_eq!(handler_id, HandlerId(0));

        let handler = program.handler(handler_id).unwrap();
        assert_eq!(handler.effect.name.as_str(), "State");
        assert!(handler.is_tail_resumptive);

        // Modify handler
        let handler_mut = program.handler_mut(handler_id).unwrap();
        handler_mut.is_tail_resumptive = false;
        handler_mut.operation_blocks.push(BlockId(1));

        let handler = program.handler(handler_id).unwrap();
        assert!(!handler.is_tail_resumptive);
        assert_eq!(handler.operation_blocks.len(), 1);
    }

    #[test]
    fn test_mir_function_with_effects() {
        let mut func = MirFunction::new("effectful".into(), MirType::Int, Span::dummy());

        // Check default state
        assert!(func.is_pure());
        assert!(!func.has_evidence());

        // Set effect row
        let state_effect = EffectType::new(EffectId(0), "State".into());
        func.set_effect_row(EffectRow::new().with_effect(state_effect.clone()));
        assert!(!func.is_pure());

        // Add evidence parameter
        let ev_local = func.add_evidence_param(state_effect.clone(), true);
        assert!(func.has_evidence());
        assert_eq!(func.evidence_params.len(), 1);
        assert_eq!(func.evidence_params[0].local, ev_local);
        assert!(func.evidence_params[0].is_static);

        // Check evidence layout
        assert_eq!(func.evidence_layout.size, 1);
        let slot = func.evidence_slot_for(EffectId(0));
        assert!(slot.is_some());
        match slot.unwrap() {
            EvidenceSlot::Static(0) => {}
            _ => panic!("Expected static slot 0"),
        }
    }

    #[test]
    fn test_mir_function_effect_statements() {
        let mut func = MirFunction::new("test".into(), MirType::Unit, Span::dummy());
        let block = func.new_block();

        // Add a regular statement
        func.block_mut(block).push_stmt(Statement {
            kind: StatementKind::Nop,
            span: Span::dummy(),
        });

        // Add an effect statement
        let effect_stmt = EffectStatementKind::PerformEffect {
            effect: EffectType::new(EffectId(0), "State".into()),
            operation: OperationId(0),
            args: vec![],
            evidence_slot: EvidenceSlot::Static(0),
            dest: Place::from_local(Local(1)),
            classification: EffectClassification::TailResumptive,
        };
        func.add_effect_statement(block, 1, effect_stmt);

        // Regular statement
        func.block_mut(block).push_stmt(Statement {
            kind: StatementKind::Nop,
            span: Span::dummy(),
        });

        // Check retrieval
        assert!(func.effect_statement(block, 0).is_none());
        assert!(func.effect_statement(block, 1).is_some());
        assert!(func.effect_statement(block, 2).is_none());
    }

    #[test]
    fn test_mir_function_effect_terminators() {
        let mut func = MirFunction::new("test".into(), MirType::Unit, Span::dummy());
        let block1 = func.new_block();
        let block2 = func.new_block();

        // Set regular terminator on block1
        func.block_mut(block1).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        // Set effect terminator on block2
        let effect_term = EffectTerminatorKind::Resume {
            continuation: Operand::Copy(Place::from_local(Local(1))),
            value: Operand::const_int(42),
            target: block1,
        };
        func.set_effect_terminator(block2, effect_term);

        // Check retrieval
        assert!(func.effect_terminator(block1).is_none());
        assert!(func.effect_terminator(block2).is_some());
    }

    #[test]
    fn test_handler_block_creation() {
        let hb = HandlerBlock {
            block_id: BlockId(5),
            effect: EffectType::new(EffectId(0), "State".into()),
            operation: OperationId(0),
            params: vec![Local(1), Local(2)],
            continuation: Some(Local(3)),
            resume_block: Some(BlockId(6)),
        };

        assert_eq!(hb.block_id, BlockId(5));
        assert_eq!(hb.params.len(), 2);
        assert!(hb.continuation.is_some());
        assert!(hb.resume_block.is_some());
    }

    #[test]
    fn test_effect_classification_default() {
        let classification = EffectClassification::default();
        assert_eq!(classification, EffectClassification::General);
    }

    #[test]
    fn test_effect_statement_kinds() {
        // Test InstallHandler
        let install = EffectStatementKind::InstallHandler {
            handler: HandlerId(0),
            evidence_slot: EvidenceSlot::Static(0),
            effect: EffectType::new(EffectId(0), "State".into()),
            prev_evidence: Some(Local(1)),
        };
        let _ = format!("{}", install); // Should not panic

        // Test PerformEffect
        let perform = EffectStatementKind::PerformEffect {
            effect: EffectType::new(EffectId(0), "State".into()),
            operation: OperationId(0),
            args: vec![Operand::const_int(42)],
            evidence_slot: EvidenceSlot::Static(0),
            dest: Place::from_local(Local(2)),
            classification: EffectClassification::TailResumptive,
        };
        let _ = format!("{}", perform);

        // Test CaptureContunuation
        let capture = EffectStatementKind::CaptureContunuation {
            dest: Place::from_local(Local(3)),
        };
        let _ = format!("{}", capture);

        // Test FfiBarrier
        let barrier = EffectStatementKind::FfiBarrier {
            strategy: FfiBarrierStrategy::CallbackConvert,
            blocked_effects: vec![EffectType::new(EffectId(0), "Async".into())],
        };
        let _ = format!("{}", barrier);
    }

    #[test]
    fn test_effect_terminator_kinds() {
        // Test Yield
        let yield_term = EffectTerminatorKind::Yield {
            effect: EffectType::new(EffectId(0), "Choice".into()),
            operation: OperationId(0),
            args: vec![],
            continuation: Operand::Copy(Place::from_local(Local(1))),
            handler_block: BlockId(2),
        };
        let _ = format!("{}", yield_term);

        // Test Resume
        let resume_term = EffectTerminatorKind::Resume {
            continuation: Operand::Copy(Place::from_local(Local(1))),
            value: Operand::const_bool(true),
            target: BlockId(3),
        };
        let _ = format!("{}", resume_term);

        // Test Handle
        let handle_term = EffectTerminatorKind::Handle {
            body: BlockId(1),
            handler: HandlerId(0),
            normal_return: BlockId(2),
            effect_return: BlockId(3),
        };
        let _ = format!("{}", handle_term);
    }

    #[test]
    fn test_effect_type_display() {
        let simple = EffectType::new(EffectId(0), "IO".into());
        assert_eq!(format!("{}", simple), "IO");

        let with_params = EffectType::new(EffectId(1), "State".into())
            .with_type_params(vec![MirType::Int, MirType::String]);
        assert_eq!(format!("{}", with_params), "State[Int, String]");
    }

    #[test]
    fn test_effect_row_display() {
        let empty = EffectRow::pure();
        assert_eq!(format!("{}", empty), "{}");

        let single = EffectRow::new()
            .with_effect(EffectType::new(EffectId(0), "IO".into()));
        assert_eq!(format!("{}", single), "{IO}");

        let multiple = EffectRow::new()
            .with_effect(EffectType::new(EffectId(0), "IO".into()))
            .with_effect(EffectType::new(EffectId(1), "State".into()));
        assert_eq!(format!("{}", multiple), "{IO, State}");

        let open = EffectRow::new()
            .with_effect(EffectType::new(EffectId(0), "IO".into()))
            .open();
        assert_eq!(format!("{}", open), "{IO, ..}");
    }

    #[test]
    fn test_mir_effect_definition() {
        let mut effect = MirEffect::new(EffectId(0), "State".into(), Span::dummy());
        effect.type_params.push("T".into());

        // Add get operation
        let get_id = effect.add_operation(EffectOperation {
            id: OperationId(0),
            name: "get".into(),
            params: vec![],
            return_ty: MirType::Int,
        });
        assert_eq!(get_id, OperationId(0));

        // Add set operation
        let set_id = effect.add_operation(EffectOperation {
            id: OperationId(1),
            name: "set".into(),
            params: vec![MirType::Int],
            return_ty: MirType::Unit,
        });
        assert_eq!(set_id, OperationId(1));

        // Check operations
        assert_eq!(effect.operations.len(), 2);
        assert_eq!(effect.operation(OperationId(0)).unwrap().name.as_str(), "get");
        assert_eq!(effect.operation(OperationId(1)).unwrap().name.as_str(), "set");
        assert!(effect.operation(OperationId(2)).is_none());
    }

    #[test]
    fn test_mir_handler_definition() {
        let effect_type = EffectType::new(EffectId(0), "State".into());
        let mut handler = MirHandler::new(HandlerId(0), effect_type, Span::dummy());

        // Default is tail-resumptive
        assert!(handler.is_tail_resumptive);

        // Add operation blocks
        handler.operation_blocks.push(BlockId(1));
        handler.operation_blocks.push(BlockId(2));
        handler.return_block = Some(BlockId(3));

        assert_eq!(handler.operation_blocks.len(), 2);
        assert_eq!(handler.return_block, Some(BlockId(3)));
    }

    // ========================================================================
    // Type Inference Tests
    // ========================================================================

    #[test]
    fn test_type_var_creation() {
        let mut ctx = LoweringContext::new();
        let var1 = ctx.fresh_type_var();
        let var2 = ctx.fresh_type_var();

        // Each call should produce a unique type variable
        assert!(matches!(var1, MirType::TypeVar(TypeVarId(0))));
        assert!(matches!(var2, MirType::TypeVar(TypeVarId(1))));
    }

    #[test]
    fn test_type_unification_same_types() {
        let mut ctx = LoweringContext::new();

        // Same types should unify
        assert!(ctx.unify_types(&MirType::Int, &MirType::Int));
        assert!(ctx.unify_types(&MirType::String, &MirType::String));
        assert!(ctx.unify_types(&MirType::Bool, &MirType::Bool));
    }

    #[test]
    fn test_type_unification_different_types() {
        let mut ctx = LoweringContext::new();

        // Different types should not unify
        assert!(!ctx.unify_types(&MirType::Int, &MirType::String));
        assert!(!ctx.unify_types(&MirType::Bool, &MirType::Float));
    }

    #[test]
    fn test_type_var_unification() {
        let mut ctx = LoweringContext::new();
        let var = ctx.fresh_type_var();

        // Type variable should unify with any type
        assert!(ctx.unify_types(&var, &MirType::Int));

        // After unification, resolving should give the concrete type
        let resolved = ctx.resolve_type(&var);
        assert_eq!(resolved, MirType::Int);
    }

    #[test]
    fn test_compound_type_unification() {
        let mut ctx = LoweringContext::new();

        // Array types with same element type should unify
        let arr1 = MirType::Array(Box::new(MirType::Int));
        let arr2 = MirType::Array(Box::new(MirType::Int));
        assert!(ctx.unify_types(&arr1, &arr2));

        // Array types with different element types should not unify
        let arr3 = MirType::Array(Box::new(MirType::String));
        assert!(!ctx.unify_types(&arr1, &arr3));
    }

    #[test]
    fn test_type_var_in_compound_type() {
        let mut ctx = LoweringContext::new();
        let var = ctx.fresh_type_var();

        // Create Array<T> where T is a type variable
        let arr_var = MirType::Array(Box::new(var.clone()));
        let arr_int = MirType::Array(Box::new(MirType::Int));

        // Should unify, binding T to Int
        assert!(ctx.unify_types(&arr_var, &arr_int));

        // Resolving should give Array<Int>
        let resolved = ctx.resolve_type(&arr_var);
        assert_eq!(resolved, MirType::Array(Box::new(MirType::Int)));
    }

    #[test]
    fn test_type_has_type_vars() {
        assert!(!MirType::Int.has_type_vars());
        assert!(!MirType::Array(Box::new(MirType::Int)).has_type_vars());

        assert!(MirType::TypeVar(TypeVarId(0)).has_type_vars());
        assert!(MirType::Array(Box::new(MirType::TypeVar(TypeVarId(0)))).has_type_vars());
        assert!(MirType::TypeParam("T".into()).has_type_vars());
    }

    #[test]
    fn test_type_substitution() {
        let param_type = MirType::TypeParam("T".into());
        let array_of_t = MirType::Array(Box::new(param_type.clone()));

        let substitutions = vec![("T".into(), MirType::Int)];

        let substituted = array_of_t.substitute(&substitutions);
        assert_eq!(substituted, MirType::Array(Box::new(MirType::Int)));
    }

    #[test]
    fn test_type_display() {
        assert_eq!(format!("{}", MirType::TypeVar(TypeVarId(0))), "?T0");
        assert_eq!(format!("{}", MirType::TypeParam("T".into())), "T");
        assert_eq!(
            format!(
                "{}",
                MirType::Generic {
                    name: "List".into(),
                    args: vec![MirType::Int]
                }
            ),
            "List<Int>"
        );
    }

    #[test]
    fn test_mir_type_is_copy_primitives() {
        // All numeric types are Copy
        assert!(MirType::Int.is_copy());
        assert!(MirType::Int8.is_copy());
        assert!(MirType::Int16.is_copy());
        assert!(MirType::Int32.is_copy());
        assert!(MirType::Int64.is_copy());
        assert!(MirType::UInt.is_copy());
        assert!(MirType::UInt8.is_copy());
        assert!(MirType::UInt16.is_copy());
        assert!(MirType::UInt32.is_copy());
        assert!(MirType::UInt64.is_copy());
        assert!(MirType::Float.is_copy());
        assert!(MirType::Float32.is_copy());
        assert!(MirType::Float64.is_copy());
        assert!(MirType::Bool.is_copy());
        assert!(MirType::Char.is_copy());
        assert!(MirType::Unit.is_copy());
        assert!(MirType::Never.is_copy());

        // String is NOT Copy (owns heap data)
        assert!(!MirType::String.is_copy());
    }

    #[test]
    fn test_mir_type_is_copy_compound() {
        // Dynamic array is NOT Copy
        assert!(!MirType::Array(Box::new(MirType::Int)).is_copy());

        // Map is NOT Copy
        assert!(!MirType::Map(Box::new(MirType::String), Box::new(MirType::Int)).is_copy());

        // Tuple of Copy types IS Copy
        assert!(MirType::Tuple(vec![MirType::Int, MirType::Bool]).is_copy());

        // Tuple with non-Copy element is NOT Copy
        assert!(!MirType::Tuple(vec![MirType::Int, MirType::String]).is_copy());

        // Optional of Copy type IS Copy
        assert!(MirType::Optional(Box::new(MirType::Int)).is_copy());

        // Optional of non-Copy type is NOT Copy
        assert!(!MirType::Optional(Box::new(MirType::String)).is_copy());

        // Result of Copy types IS Copy
        assert!(MirType::Result(Box::new(MirType::Int), Box::new(MirType::Bool)).is_copy());

        // Result with non-Copy is NOT Copy
        assert!(!MirType::Result(Box::new(MirType::String), Box::new(MirType::Int)).is_copy());
    }

    #[test]
    fn test_mir_type_is_copy_references() {
        // References are Copy (copying the pointer, not the data)
        assert!(MirType::Ref(Box::new(MirType::Int)).is_copy());
        assert!(MirType::RefMut(Box::new(MirType::Int)).is_copy());
        assert!(MirType::Ref(Box::new(MirType::String)).is_copy());
    }

    #[test]
    fn test_mir_type_is_copy_functions_and_structs() {
        // Functions and closures are NOT Copy
        assert!(!MirType::FnPtr {
            params: vec![MirType::Int],
            ret: Box::new(MirType::Bool)
        }.is_copy());
        assert!(!MirType::Closure {
            params: vec![MirType::Int],
            ret: Box::new(MirType::Bool)
        }.is_copy());

        // Structs and Enums are NOT Copy by default
        assert!(!MirType::Struct(StructId(0)).is_copy());
        assert!(!MirType::Enum(EnumId(0)).is_copy());

        // Type variables are NOT Copy (unknown)
        assert!(!MirType::TypeVar(TypeVarId(0)).is_copy());
        assert!(!MirType::TypeParam("T".into()).is_copy());
    }
}
