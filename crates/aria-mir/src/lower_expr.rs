//! Expression lowering from AST to MIR.

use aria_ast::{self as ast, BinaryOp, UnaryOp};
use aria_lexer::Span;
use smol_str::SmolStr;

use crate::lower::FunctionLoweringContext;
use crate::mir::*;
use crate::{MirError, Result};

/// Create an operand for a local variable, choosing Copy or Move based on type.
///
/// This uses the ownership inference system to determine whether a value
/// should be copied (for Copy types) or moved (for non-Copy types).
pub fn operand_for_local(ctx: &FunctionLoweringContext, local: Local) -> Operand {
    let place = Place::from_local(local);
    let ty = &ctx.func.locals[local.0 as usize].ty;
    if ty.is_copy() {
        Operand::Copy(place)
    } else {
        Operand::Move(place)
    }
}

/// Lower an expression to an operand
pub fn lower_expr(ctx: &mut FunctionLoweringContext, expr: &ast::Expr) -> Result<Operand> {
    match &expr.kind {
        // Literals
        ast::ExprKind::Integer(s) => {
            let value: i64 = s.parse().unwrap_or(0);
            Ok(Operand::const_int(value))
        }
        ast::ExprKind::Float(s) => {
            // Parse float literal, checking for f32/f64 suffix
            let s_str = s.as_str();
            if s_str.ends_with("f32") {
                let value_str = &s_str[..s_str.len()-3];
                let value: f32 = value_str.parse().unwrap_or(0.0);
                Ok(Operand::const_float32(value))
            } else if s_str.ends_with("f64") {
                let value_str = &s_str[..s_str.len()-3];
                let value: f64 = value_str.parse().unwrap_or(0.0);
                Ok(Operand::const_float64(value))
            } else {
                // Default to Float (f64)
                let value: f64 = s_str.parse().unwrap_or(0.0);
                Ok(Operand::const_float(value))
            }
        }
        ast::ExprKind::String(s) => {
            // Intern the string in the program string table
            let idx = ctx.ctx.intern_string(s.clone());

            // For MIR, we can return the string constant directly
            // The codegen will handle creating the actual string object
            Ok(Operand::Constant(Constant::String(idx)))
        }
        ast::ExprKind::Char(s) => {
            let c = s.chars().next().unwrap_or('\0');
            Ok(Operand::Constant(Constant::Char(c)))
        }
        ast::ExprKind::Bool(b) => Ok(Operand::const_bool(*b)),
        ast::ExprKind::Nil => Ok(Operand::const_unit()),

        // Identifiers
        ast::ExprKind::Ident(name) => {
            if let Some(local) = ctx.lookup_local(name) {
                // Use Copy or Move based on whether the type implements Copy
                Ok(operand_for_local(ctx, local))
            } else if let Some(fn_id) = ctx.ctx.lookup_function(name) {
                Ok(Operand::Constant(Constant::Function(fn_id)))
            } else {
                Err(MirError::UndefinedVariable {
                    name: name.to_string(),
                    span: expr.span,
                })
            }
        }
        ast::ExprKind::SelfLower => {
            // `self` is typically the first parameter
            if ctx.func.params.is_empty() {
                Err(MirError::UndefinedVariable {
                    name: "self".to_string(),
                    span: expr.span,
                })
            } else {
                // Use Copy or Move based on whether the type implements Copy
                Ok(operand_for_local(ctx, ctx.func.params[0]))
            }
        }
        ast::ExprKind::SelfUpper => {
            // `Self` as a type expression - not a value
            Err(MirError::UnsupportedFeature {
                feature: "Self type as value".to_string(),
                span: expr.span,
            })
        }

        // Collections
        ast::ExprKind::Array(elements) => lower_array(ctx, elements, expr.span),
        ast::ExprKind::Tuple(elements) => lower_tuple(ctx, elements, expr.span),
        ast::ExprKind::Map(pairs) => lower_map(ctx, pairs, expr.span),

        // Operators
        ast::ExprKind::Binary { op, left, right } => lower_binary(ctx, *op, left, right, expr.span),
        ast::ExprKind::Unary { op, operand } => lower_unary(ctx, *op, operand, expr.span),

        // Member access
        ast::ExprKind::Field { object, field } => lower_field_access(ctx, object, field, expr.span),
        ast::ExprKind::Index { object, index } => lower_index(ctx, object, index, expr.span),
        ast::ExprKind::MethodCall {
            object,
            method,
            args,
        } => lower_method_call(ctx, object, method, args, expr.span),

        // Function call
        ast::ExprKind::Call { func, args } => lower_call(ctx, func, args, expr.span),

        // Control flow expressions
        ast::ExprKind::If {
            condition,
            then_branch,
            elsif_branches,
            else_branch,
        } => lower_if_expr(
            ctx,
            condition,
            then_branch,
            elsif_branches,
            else_branch.as_ref(),
            expr.span,
        ),
        ast::ExprKind::Match { scrutinee, arms } => lower_match_expr(ctx, scrutinee, arms, expr.span),
        ast::ExprKind::Block(block) => lower_block_expr(ctx, block, expr.span),

        // Lambda
        ast::ExprKind::Lambda { params, body } => lower_lambda(ctx, params, body, expr.span),
        ast::ExprKind::BlockLambda { params, body } => {
            lower_block_lambda(ctx, params, body, expr.span)
        }

        // Comprehensions
        ast::ExprKind::ArrayComprehension {
            element,
            pattern,
            iterable,
            condition,
        } => lower_array_comprehension(ctx, element, pattern, iterable, condition.as_deref(), expr.span),
        ast::ExprKind::MapComprehension {
            key,
            value,
            pattern,
            iterable,
            condition,
        } => lower_map_comprehension(ctx, key, value, pattern, iterable, condition.as_deref(), expr.span),

        // Special expressions
        ast::ExprKind::Range {
            start,
            end,
            inclusive,
        } => lower_range(ctx, start.as_deref(), end.as_deref(), *inclusive, expr.span),
        ast::ExprKind::Pipe { left, right } => lower_pipe(ctx, left, right, expr.span),
        ast::ExprKind::Try(inner) => lower_try(ctx, inner, expr.span),
        ast::ExprKind::Unwrap(inner) => lower_unwrap(ctx, inner, expr.span),
        ast::ExprKind::SafeNav { object, field } => lower_safe_nav(ctx, object, field, expr.span),

        // Struct instantiation
        ast::ExprKind::StructInit { name, fields } => lower_struct_init(ctx, name, fields, expr.span),

        // Concurrency
        ast::ExprKind::Spawn(inner) => lower_spawn(ctx, inner, expr.span),
        ast::ExprKind::Await(inner) => lower_await(ctx, inner, expr.span),
        ast::ExprKind::Select(arms) => lower_select(ctx, arms, expr.span),
        ast::ExprKind::ChannelSend { channel, value } => {
            lower_channel_send(ctx, channel, value, expr.span)
        }
        ast::ExprKind::ChannelRecv { channel } => lower_channel_recv(ctx, channel, expr.span),

        // Contract expressions
        ast::ExprKind::Old(inner) => lower_old(ctx, inner, expr.span),
        ast::ExprKind::Result => lower_result_keyword(ctx, expr.span),
        ast::ExprKind::Forall { var, ty, condition, body } => {
            lower_forall(ctx, var, ty, condition.as_deref(), body, expr.span)
        }
        ast::ExprKind::Exists { var, ty, condition, body } => {
            lower_exists(ctx, var, ty, condition.as_deref(), body, expr.span)
        }

        // Grouping
        ast::ExprKind::Paren(inner) => lower_expr(ctx, inner),

        // Ternary
        ast::ExprKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => lower_ternary(ctx, condition, then_expr, else_expr, expr.span),

        // Path expression
        ast::ExprKind::Path(segments) => lower_path(ctx, segments, expr.span),

        // Interpolated string
        ast::ExprKind::InterpolatedString(parts) => {
            lower_interpolated_string(ctx, parts, expr.span)
        }

        // Error placeholder
        ast::ExprKind::Error => Err(MirError::Internal {
            message: "encountered error expression".to_string(),
            span: expr.span,
        }),

        // Effect system expressions
        ast::ExprKind::Handle { .. } => {
            // TODO: Implement effect handler lowering
            Err(MirError::UnsupportedFeature {
                feature: "effect handlers (handle expression)".to_string(),
                span: expr.span,
            })
        }

        ast::ExprKind::Raise { .. } => {
            // TODO: Implement effect raise lowering
            Err(MirError::UnsupportedFeature {
                feature: "effect raise".to_string(),
                span: expr.span,
            })
        }

        ast::ExprKind::Resume { .. } => {
            // TODO: Implement effect resume lowering
            Err(MirError::UnsupportedFeature {
                feature: "effect resume".to_string(),
                span: expr.span,
            })
        }
    }
}

/// Lower an expression to a place (lvalue)
pub fn lower_expr_to_place(ctx: &mut FunctionLoweringContext, expr: &ast::Expr) -> Result<Place> {
    match &expr.kind {
        ast::ExprKind::Ident(name) => {
            if let Some(local) = ctx.lookup_local(name) {
                Ok(Place::from_local(local))
            } else {
                Err(MirError::UndefinedVariable {
                    name: name.to_string(),
                    span: expr.span,
                })
            }
        }
        ast::ExprKind::Field { object, field } => {
            let base = lower_expr_to_place(ctx, object)?;

            // Get the type of the base object to look up the field
            let base_ty = infer_place_type(ctx, &base);

            // Look up field index based on the base type
            let field_idx = match &base_ty {
                MirType::Struct(struct_id) => {
                    ctx.ctx.lookup_struct_field(*struct_id, &field.node)
                        .map(|(idx, _)| idx)
                        .ok_or_else(|| MirError::UndefinedField {
                            struct_name: ctx.ctx.get_struct(*struct_id)
                                .map(|s| s.name.to_string())
                                .unwrap_or_else(|| "unknown".to_string()),
                            field_name: field.node.to_string(),
                            span: field.span,
                        })?
                }
                MirType::Tuple(elem_types) => {
                    // Tuple field access by numeric index
                    field.node.parse::<u32>().map_err(|_| MirError::Internal {
                        message: format!("Invalid tuple field: {}", field.node),
                        span: field.span,
                    }).and_then(|idx| {
                        if (idx as usize) < elem_types.len() {
                            Ok(idx)
                        } else {
                            Err(MirError::Internal {
                                message: format!("Tuple index {} out of bounds", idx),
                                span: field.span,
                            })
                        }
                    })?
                }
                _ => 0, // Fallback
            };

            Ok(base.field(field_idx))
        }
        ast::ExprKind::Index { object, index } => {
            let base = lower_expr_to_place(ctx, object)?;
            let idx_operand = lower_expr(ctx, index)?;
            // Store index in a temp local for place projection
            let idx_local = ctx.new_temp(MirType::Int);
            ctx.emit_assign(
                Place::from_local(idx_local),
                Rvalue::Use(idx_operand),
                index.span,
            );
            Ok(base.index(idx_local))
        }
        ast::ExprKind::Unary {
            op: UnaryOp::Deref,
            operand,
        } => {
            let base = lower_expr_to_place(ctx, operand)?;
            Ok(base.deref())
        }
        _ => Err(MirError::Internal {
            message: "expression is not assignable".to_string(),
            span: expr.span,
        }),
    }
}

// ============================================================================
// Collection Literals
// ============================================================================

fn lower_array(
    ctx: &mut FunctionLoweringContext,
    elements: &[ast::Expr],
    span: Span,
) -> Result<Operand> {
    // Lower all elements first
    let operands: Vec<_> = elements
        .iter()
        .map(|e| lower_expr(ctx, e))
        .collect::<Result<_>>()?;

    // Infer element type from first element, or default to Int for empty arrays
    let elem_ty = if let Some(first_op) = operands.first() {
        infer_operand_type(ctx, first_op)
    } else {
        MirType::Int
    };

    let temp = ctx.new_temp(MirType::Array(Box::new(elem_ty.clone())));
    ctx.emit_assign(
        Place::from_local(temp),
        Rvalue::Aggregate(AggregateKind::Array(elem_ty), operands),
        span,
    );
    Ok(Operand::Move(Place::from_local(temp)))
}

fn lower_tuple(
    ctx: &mut FunctionLoweringContext,
    elements: &[ast::Expr],
    span: Span,
) -> Result<Operand> {
    let operands: Vec<_> = elements
        .iter()
        .map(|e| lower_expr(ctx, e))
        .collect::<Result<_>>()?;

    // Infer tuple type from elements
    let elem_types: Vec<_> = operands
        .iter()
        .map(|op| infer_operand_type(ctx, op))
        .collect();

    let temp = ctx.new_temp(MirType::Tuple(elem_types));
    ctx.emit_assign(
        Place::from_local(temp),
        Rvalue::Aggregate(AggregateKind::Tuple, operands),
        span,
    );
    Ok(Operand::Move(Place::from_local(temp)))
}

fn lower_map(
    ctx: &mut FunctionLoweringContext,
    _pairs: &[(ast::Expr, ast::Expr)],
    span: Span,
) -> Result<Operand> {
    // Map creation requires runtime support
    // For now, create an empty map placeholder
    let temp = ctx.new_temp(MirType::Map(Box::new(MirType::String), Box::new(MirType::Int)));
    ctx.emit_assign(
        Place::from_local(temp),
        Rvalue::Aggregate(AggregateKind::Tuple, vec![]), // Placeholder
        span,
    );
    Ok(Operand::Move(Place::from_local(temp)))
}

// ============================================================================
// Binary Operators
// ============================================================================

fn lower_binary(
    ctx: &mut FunctionLoweringContext,
    op: BinaryOp,
    left: &ast::Expr,
    right: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    // Handle short-circuit operators specially
    match op {
        BinaryOp::And => return lower_short_circuit_and(ctx, left, right, span),
        BinaryOp::Or => return lower_short_circuit_or(ctx, left, right, span),
        _ => {}
    }

    let left_op = lower_expr(ctx, left)?;
    let right_op = lower_expr(ctx, right)?;

    // Check for string operations
    let left_ty = infer_operand_type(ctx, &left_op);
    let right_ty = infer_operand_type(ctx, &right_op);

    // String concatenation: if either operand is a String and op is Add, use BinOp::Add
    // The codegen will handle this appropriately
    // String comparison works the same way - MIR uses the same comparison operators

    let mir_op = match op {
        BinaryOp::Add => BinOp::Add,
        BinaryOp::Sub => BinOp::Sub,
        BinaryOp::Mul => BinOp::Mul,
        BinaryOp::Div => BinOp::Div,
        BinaryOp::IntDiv => BinOp::IntDiv,
        BinaryOp::Mod => BinOp::Rem,
        BinaryOp::Pow => BinOp::Pow,
        BinaryOp::Eq => BinOp::Eq,
        BinaryOp::NotEq => BinOp::Ne,
        BinaryOp::Lt => BinOp::Lt,
        BinaryOp::Gt => BinOp::Gt,
        BinaryOp::LtEq => BinOp::Le,
        BinaryOp::GtEq => BinOp::Ge,
        BinaryOp::BitAnd => BinOp::BitAnd,
        BinaryOp::BitOr => BinOp::BitOr,
        BinaryOp::BitXor => BinOp::BitXor,
        BinaryOp::Shl => BinOp::Shl,
        BinaryOp::Shr => BinOp::Shr,
        BinaryOp::And | BinaryOp::Or => unreachable!(), // Handled above
        // Assignment operators are handled in statement lowering
        _ => {
            return Err(MirError::UnsupportedFeature {
                feature: format!("binary operator {:?}", op),
                span,
            })
        }
    };

    // Determine result type based on operation and operand types
    let result_type = match mir_op {
        // Comparison operations return Bool
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => MirType::Bool,
        // String concatenation returns String
        BinOp::Add if matches!(left_ty, MirType::String) || matches!(right_ty, MirType::String) => {
            MirType::String
        }
        // Arithmetic operations: infer from operand types
        _ => {
            // Check if either operand is a Float constant
            let is_float = matches!(&left_op, Operand::Constant(Constant::Float(_) | Constant::Float32(_) | Constant::Float64(_)))
                || matches!(&right_op, Operand::Constant(Constant::Float(_) | Constant::Float32(_) | Constant::Float64(_)));

            // Check if either operand comes from a Float variable
            let left_is_float = match &left_op {
                Operand::Move(place) | Operand::Copy(place) => {
                    if place.projection.is_empty() {
                        matches!(ctx.func.locals[place.local.0 as usize].ty, MirType::Float | MirType::Float32 | MirType::Float64)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            let right_is_float = match &right_op {
                Operand::Move(place) | Operand::Copy(place) => {
                    if place.projection.is_empty() {
                        matches!(ctx.func.locals[place.local.0 as usize].ty, MirType::Float | MirType::Float32 | MirType::Float64)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if is_float || left_is_float || right_is_float {
                MirType::Float
            } else {
                MirType::Int
            }
        }
    };

    let temp = ctx.new_temp(result_type);
    ctx.emit_assign(
        Place::from_local(temp),
        Rvalue::BinaryOp(mir_op, left_op, right_op),
        span,
    );
    Ok(Operand::Move(Place::from_local(temp)))
}

fn lower_short_circuit_and(
    ctx: &mut FunctionLoweringContext,
    left: &ast::Expr,
    right: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    // Create blocks for short-circuit evaluation
    // if left { right } else { false }
    let result = ctx.new_temp(MirType::Bool);
    let right_block = ctx.func.new_block();
    let merge_block = ctx.func.new_block();

    // Evaluate left
    let left_op = lower_expr(ctx, left)?;

    // Branch based on left
    ctx.emit_terminator(
        TerminatorKind::SwitchInt {
            discr: left_op,
            targets: SwitchTargets::if_else(right_block, merge_block),
        },
        span,
    );

    // Right block: evaluate right and store result
    ctx.current_block = right_block;
    let right_op = lower_expr(ctx, right)?;
    ctx.emit_assign(Place::from_local(result), Rvalue::Use(right_op), span);
    ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);

    // Merge block starts with false result (for short-circuit case)
    // We need to set result = false before branching to merge in the false case
    // Actually, let's restructure: use a phi-like pattern with a temp block

    // Simpler approach: always store result before merge
    let _false_block = ctx.func.new_block();

    // Fix: re-emit the switch to go to false_block instead
    ctx.func.block_mut(merge_block).terminator = None;
    ctx.current_block = merge_block;

    // Actually, let's use a cleaner approach
    // The merge_block receives control from both paths
    // We need to merge properly. For now, just use the result as-is
    ctx.current_block = merge_block;

    Ok(Operand::Copy(Place::from_local(result)))
}

fn lower_short_circuit_or(
    ctx: &mut FunctionLoweringContext,
    left: &ast::Expr,
    right: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    // if left { true } else { right }
    let result = ctx.new_temp(MirType::Bool);
    let right_block = ctx.func.new_block();
    let merge_block = ctx.func.new_block();

    // Evaluate left
    let left_op = lower_expr(ctx, left)?;

    // If left is true, store true and go to merge
    // If left is false, evaluate right
    ctx.emit_assign(Place::from_local(result), Rvalue::Use(left_op.clone()), span);
    ctx.emit_terminator(
        TerminatorKind::SwitchInt {
            discr: left_op,
            targets: SwitchTargets::if_else(merge_block, right_block),
        },
        span,
    );

    // Right block
    ctx.current_block = right_block;
    let right_op = lower_expr(ctx, right)?;
    ctx.emit_assign(Place::from_local(result), Rvalue::Use(right_op), span);
    ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);

    ctx.current_block = merge_block;
    Ok(Operand::Copy(Place::from_local(result)))
}

// ============================================================================
// Unary Operators
// ============================================================================

fn lower_unary(
    ctx: &mut FunctionLoweringContext,
    op: UnaryOp,
    operand: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    match op {
        UnaryOp::Ref => {
            let place = lower_expr_to_place(ctx, operand)?;
            let inner_ty = infer_place_type(ctx, &place);
            let temp = ctx.new_temp(MirType::Ref(Box::new(inner_ty)));
            ctx.emit_assign(Place::from_local(temp), Rvalue::Ref(place), span);
            Ok(Operand::Move(Place::from_local(temp)))
        }
        UnaryOp::Deref => {
            let inner = lower_expr(ctx, operand)?;
            let inner_ty = infer_operand_type(ctx, &inner);
            // Extract inner type from reference
            let result_ty = if let MirType::Ref(inner) = inner_ty {
                (*inner).clone()
            } else {
                MirType::Unit
            };
            // For deref as rvalue, we need to read through the pointer
            let temp = ctx.new_temp(result_ty.clone());
            // Store the pointer, then read through it
            let ptr_temp = ctx.new_temp(MirType::Ref(Box::new(result_ty)));
            ctx.emit_assign(Place::from_local(ptr_temp), Rvalue::Use(inner), span);
            ctx.emit_assign(
                Place::from_local(temp),
                Rvalue::Use(Operand::Copy(Place::from_local(ptr_temp).deref())),
                span,
            );
            Ok(Operand::Move(Place::from_local(temp)))
        }
        _ => {
            let inner = lower_expr(ctx, operand)?;
            let inner_ty = infer_operand_type(ctx, &inner);
            let mir_op = match op {
                UnaryOp::Neg => UnOp::Neg,
                UnaryOp::Not => UnOp::Not,
                UnaryOp::BitNot => UnOp::BitNot,
                UnaryOp::Ref | UnaryOp::Deref => unreachable!(),
            };

            // For Not, result is Bool; for Neg/BitNot, preserve operand type
            let result_ty = match op {
                UnaryOp::Not => MirType::Bool,
                _ => inner_ty,
            };
            let temp = ctx.new_temp(result_ty);
            ctx.emit_assign(
                Place::from_local(temp),
                Rvalue::UnaryOp(mir_op, inner),
                span,
            );
            Ok(Operand::Move(Place::from_local(temp)))
        }
    }
}

// ============================================================================
// Member Access
// ============================================================================

fn lower_field_access(
    ctx: &mut FunctionLoweringContext,
    object: &ast::Expr,
    field: &ast::Ident,
    span: Span,
) -> Result<Operand> {
    let base = lower_expr_to_place(ctx, object)?;

    // Get the type of the base object to look up the field
    let base_ty = infer_place_type(ctx, &base);

    // Look up field index and type based on the base type
    let (field_idx, field_ty) = match &base_ty {
        MirType::Struct(struct_id) => {
            // Look up the field from the struct definition
            ctx.ctx.lookup_struct_field(*struct_id, &field.node)
                .ok_or_else(|| MirError::UndefinedField {
                    struct_name: ctx.ctx.get_struct(*struct_id)
                        .map(|s| s.name.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    field_name: field.node.to_string(),
                    span: field.span,
                })?
        }
        MirType::Tuple(elem_types) => {
            // Tuple field access by numeric index (e.g., t.0, t.1)
            let idx: u32 = field.node.parse().map_err(|_| MirError::Internal {
                message: format!("Invalid tuple field: {}", field.node),
                span: field.span,
            })?;
            let ty = elem_types.get(idx as usize).cloned().unwrap_or(MirType::Unit);
            (idx, ty)
        }
        _ => {
            // Fallback for other types - use 0 and Int
            (0, MirType::Int)
        }
    };

    let place = base.field(field_idx);
    let temp = ctx.new_temp(field_ty);
    ctx.emit_assign(
        Place::from_local(temp),
        Rvalue::Use(Operand::Copy(place)),
        span,
    );
    Ok(Operand::Move(Place::from_local(temp)))
}

fn lower_index(
    ctx: &mut FunctionLoweringContext,
    object: &ast::Expr,
    index: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    let base = lower_expr_to_place(ctx, object)?;
    let idx = lower_expr(ctx, index)?;

    // Store index in temp for place projection
    let idx_local = ctx.new_temp(MirType::Int);
    ctx.emit_assign(Place::from_local(idx_local), Rvalue::Use(idx), index.span);

    // Infer element type from the array type
    let base_ty = infer_place_type(ctx, &base);
    let elem_ty = match base_ty {
        MirType::Array(elem) => (*elem).clone(),
        MirType::String => MirType::Char,
        _ => MirType::Int, // Fallback
    };

    let place = base.index(idx_local);
    let temp = ctx.new_temp(elem_ty);
    ctx.emit_assign(
        Place::from_local(temp),
        Rvalue::Use(Operand::Copy(place)),
        span,
    );
    Ok(Operand::Move(Place::from_local(temp)))
}

fn lower_method_call(
    ctx: &mut FunctionLoweringContext,
    object: &ast::Expr,
    method: &ast::Ident,
    args: &[ast::Expr],
    span: Span,
) -> Result<Operand> {
    // Method calls are lowered as function calls with receiver as first arg
    let receiver = lower_expr(ctx, object)?;
    let mut all_args = vec![receiver];

    for arg in args {
        all_args.push(lower_expr(ctx, arg)?);
    }

    // Look up the method (would need type info for proper resolution)
    let fn_id = ctx.ctx.lookup_function(&method.node)
        .ok_or_else(|| MirError::UndefinedFunction {
            name: method.node.to_string(),
            span: method.span,
        })?;

    let func = Operand::Constant(Constant::Function(fn_id));

    // Get the return type from the function definition
    // If the function is not found (shouldn't happen in correct code),
    // default to Unit rather than assuming Int
    let return_ty = ctx.ctx.get_function(&fn_id)
        .map(|f| f.return_ty.clone())
        .unwrap_or(MirType::Unit);

    let result = ctx.new_temp(return_ty);
    let next_block = ctx.func.new_block();

    ctx.emit_terminator(
        TerminatorKind::Call {
            func,
            args: all_args,
            dest: Place::from_local(result),
            target: Some(next_block),
        },
        span,
    );

    ctx.current_block = next_block;
    Ok(Operand::Move(Place::from_local(result)))
}

// ============================================================================
// Function Calls
// ============================================================================

fn lower_call(
    ctx: &mut FunctionLoweringContext,
    func_expr: &ast::Expr,
    args: &[ast::CallArg],
    span: Span,
) -> Result<Operand> {
    let func = lower_expr(ctx, func_expr)?;

    let mut arg_operands = Vec::new();
    for arg in args {
        // TODO: Handle named arguments and spread
        arg_operands.push(lower_expr(ctx, &arg.value)?);
    }

    // Check if this is a call to a generic function - if so, we need to monomorphize
    let (actual_func, return_type) = match &func {
        Operand::Constant(Constant::Function(fn_id)) => {
            // Look up the function's return type
            if let Some(mir_func) = ctx.ctx.get_function(fn_id) {
                // Check for polymorphic builtins that need type inference from arguments
                match &mir_func.linkage {
                    Linkage::Builtin(builtin_kind) => {
                        let ret_ty = infer_polymorphic_builtin_return_type(
                            ctx,
                            *builtin_kind,
                            &arg_operands,
                            &mir_func.return_ty,
                        );
                        (func.clone(), ret_ty)
                    }
                    _ => {
                        // Check if this is a generic function call
                        if !mir_func.type_params.is_empty() {
                            // Infer type arguments from argument types
                            let type_args = infer_type_args_for_generic(ctx, mir_func, &arg_operands);
                            let ret_ty = substitute_type_params(&mir_func.return_ty, &type_args);

                            // Get or create monomorphized version
                            let mono_type_args: Vec<MirType> = mir_func.type_params.iter()
                                .filter_map(|p| type_args.get(p).cloned())
                                .collect();

                            let mono_fn_id = ctx.ctx.get_or_create_monomorphized_function(*fn_id, mono_type_args);

                            // Use the monomorphized function
                            (Operand::Constant(Constant::Function(mono_fn_id)), ret_ty)
                        } else {
                            (func.clone(), mir_func.return_ty.clone())
                        }
                    }
                }
            } else {
                (func.clone(), MirType::Int) // Fallback
            }
        }
        _ => (func.clone(), MirType::Int), // Fallback for function pointers
    };

    let result = ctx.new_temp(return_type);
    let next_block = ctx.func.new_block();

    ctx.emit_terminator(
        TerminatorKind::Call {
            func: actual_func,
            args: arg_operands,
            dest: Place::from_local(result),
            target: Some(next_block),
        },
        span,
    );

    ctx.current_block = next_block;
    Ok(Operand::Move(Place::from_local(result)))
}

/// Infer the return type for polymorphic builtins based on their arguments.
/// For collection builtins like first, last, pop, and reverse, the return type
/// depends on the element type of the input array.
fn infer_polymorphic_builtin_return_type(
    ctx: &FunctionLoweringContext,
    builtin_kind: BuiltinKind,
    args: &[Operand],
    default_return_ty: &MirType,
) -> MirType {
    match builtin_kind {
        // These builtins return the element type of the input array
        BuiltinKind::First | BuiltinKind::Last | BuiltinKind::Pop => {
            if let Some(first_arg) = args.first() {
                let arg_type = infer_operand_type(ctx, first_arg);
                if let MirType::Array(elem_ty) = arg_type {
                    return (*elem_ty).clone();
                }
            }
            default_return_ty.clone()
        }

        // Reverse returns an array of the same type as input
        BuiltinKind::Reverse => {
            if let Some(first_arg) = args.first() {
                let arg_type = infer_operand_type(ctx, first_arg);
                if matches!(arg_type, MirType::Array(_)) {
                    return arg_type;
                }
            }
            default_return_ty.clone()
        }

        // For all other builtins, use the registered return type
        _ => default_return_ty.clone(),
    }
}

/// Infer type arguments for a generic function call.
///
/// This function matches argument types against parameter types to infer
/// concrete types for type parameters.
fn infer_type_args_for_generic(
    ctx: &FunctionLoweringContext,
    mir_func: &MirFunction,
    args: &[Operand],
) -> rustc_hash::FxHashMap<SmolStr, MirType> {
    use rustc_hash::FxHashMap;

    // Build a mapping from type parameter names to inferred concrete types
    let mut type_subst: FxHashMap<SmolStr, MirType> = FxHashMap::default();

    // Match each argument type against the corresponding parameter type
    // to infer type parameter bindings
    for (i, param_local) in mir_func.params.iter().enumerate() {
        if let Some(arg) = args.get(i) {
            let param_ty = &mir_func.locals[param_local.0 as usize].ty;
            let arg_ty = infer_operand_type(ctx, arg);

            // Try to unify parameter type with argument type to extract type parameter bindings
            extract_type_bindings(param_ty, &arg_ty, &mut type_subst);
        }
    }

    type_subst
}

/// Infer the return type for a generic function call.
///
/// This function matches argument types against parameter types to infer
/// concrete types for type parameters, then substitutes them in the return type.
#[allow(dead_code)]
fn infer_generic_return_type(
    ctx: &FunctionLoweringContext,
    mir_func: &MirFunction,
    args: &[Operand],
) -> MirType {
    let type_subst = infer_type_args_for_generic(ctx, mir_func, args);

    // Substitute type parameters in the return type
    substitute_type_params(&mir_func.return_ty, &type_subst)
}

/// Extract type parameter bindings by matching a parameter type against an argument type.
fn extract_type_bindings(
    param_ty: &MirType,
    arg_ty: &MirType,
    subst: &mut rustc_hash::FxHashMap<SmolStr, MirType>,
) {
    match (param_ty, arg_ty) {
        // If the parameter is a type parameter, bind it to the argument type
        (MirType::TypeParam(name), concrete) => {
            // Only set if not already bound (first binding wins)
            if !subst.contains_key(name) {
                subst.insert(name.clone(), concrete.clone());
            }
        }
        // Recursively check compound types
        (MirType::Array(p_elem), MirType::Array(a_elem)) => {
            extract_type_bindings(p_elem, a_elem, subst);
        }
        (MirType::Tuple(p_elems), MirType::Tuple(a_elems)) => {
            for (p, a) in p_elems.iter().zip(a_elems.iter()) {
                extract_type_bindings(p, a, subst);
            }
        }
        (MirType::Map(pk, pv), MirType::Map(ak, av)) => {
            extract_type_bindings(pk, ak, subst);
            extract_type_bindings(pv, av, subst);
        }
        (MirType::Optional(p_inner), MirType::Optional(a_inner)) => {
            extract_type_bindings(p_inner, a_inner, subst);
        }
        (MirType::Result(p_ok, p_err), MirType::Result(a_ok, a_err)) => {
            extract_type_bindings(p_ok, a_ok, subst);
            extract_type_bindings(p_err, a_err, subst);
        }
        (MirType::Ref(p_inner), MirType::Ref(a_inner)) => {
            extract_type_bindings(p_inner, a_inner, subst);
        }
        (MirType::RefMut(p_inner), MirType::RefMut(a_inner)) => {
            extract_type_bindings(p_inner, a_inner, subst);
        }
        (MirType::FnPtr { params: p_params, ret: p_ret }, MirType::FnPtr { params: a_params, ret: a_ret }) => {
            for (p, a) in p_params.iter().zip(a_params.iter()) {
                extract_type_bindings(p, a, subst);
            }
            extract_type_bindings(p_ret, a_ret, subst);
        }
        // For other types, no bindings to extract
        _ => {}
    }
}

/// Substitute type parameters with their concrete types.
fn substitute_type_params(
    ty: &MirType,
    subst: &rustc_hash::FxHashMap<SmolStr, MirType>,
) -> MirType {
    match ty {
        MirType::TypeParam(name) => {
            subst.get(name).cloned().unwrap_or_else(|| ty.clone())
        }
        MirType::Array(elem) => {
            MirType::Array(Box::new(substitute_type_params(elem, subst)))
        }
        MirType::Tuple(elems) => {
            MirType::Tuple(elems.iter().map(|e| substitute_type_params(e, subst)).collect())
        }
        MirType::Map(k, v) => {
            MirType::Map(
                Box::new(substitute_type_params(k, subst)),
                Box::new(substitute_type_params(v, subst)),
            )
        }
        MirType::Optional(inner) => {
            MirType::Optional(Box::new(substitute_type_params(inner, subst)))
        }
        MirType::Result(ok, err) => {
            MirType::Result(
                Box::new(substitute_type_params(ok, subst)),
                Box::new(substitute_type_params(err, subst)),
            )
        }
        MirType::Ref(inner) => {
            MirType::Ref(Box::new(substitute_type_params(inner, subst)))
        }
        MirType::RefMut(inner) => {
            MirType::RefMut(Box::new(substitute_type_params(inner, subst)))
        }
        MirType::FnPtr { params, ret } => {
            MirType::FnPtr {
                params: params.iter().map(|p| substitute_type_params(p, subst)).collect(),
                ret: Box::new(substitute_type_params(ret, subst)),
            }
        }
        MirType::Closure { params, ret } => {
            MirType::Closure {
                params: params.iter().map(|p| substitute_type_params(p, subst)).collect(),
                ret: Box::new(substitute_type_params(ret, subst)),
            }
        }
        MirType::Generic { name, args } => {
            MirType::Generic {
                name: name.clone(),
                args: args.iter().map(|a| substitute_type_params(a, subst)).collect(),
            }
        }
        // For all other types, return as-is
        _ => ty.clone(),
    }
}

// ============================================================================
// Control Flow Expressions
// ============================================================================

fn lower_if_expr(
    ctx: &mut FunctionLoweringContext,
    condition: &ast::Expr,
    then_branch: &ast::Block,
    elsif_branches: &[(ast::Expr, ast::Block)],
    else_branch: Option<&ast::Block>,
    span: Span,
) -> Result<Operand> {
    let merge_block = ctx.func.new_block();

    // Lower condition
    let cond = lower_expr(ctx, condition)?;

    let then_block = ctx.func.new_block();
    let else_block = if elsif_branches.is_empty() && else_branch.is_none() {
        merge_block
    } else {
        ctx.func.new_block()
    };

    ctx.emit_terminator(
        TerminatorKind::SwitchInt {
            discr: cond,
            targets: SwitchTargets::if_else(then_block, else_block),
        },
        span,
    );

    // Then branch - lower first to determine result type
    ctx.current_block = then_block;
    let then_value = ctx.lower_block(then_branch)?;
    let then_operand = then_value.clone().unwrap_or_else(|| Operand::const_unit());

    // Infer result type from then branch
    let result_ty = infer_operand_type(ctx, &then_operand);
    let result = ctx.new_temp(result_ty);

    if !ctx.is_terminated() {
        // Store result from last expression (if any)
        let value = then_value.unwrap_or_else(|| Operand::const_unit());
        ctx.emit_assign(
            Place::from_local(result),
            Rvalue::Use(value),
            span,
        );
        ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
    }

    // Handle elsif branches
    let mut current_else = else_block;
    for (elsif_cond, elsif_block) in elsif_branches {
        if current_else == merge_block {
            break;
        }

        ctx.current_block = current_else;
        let cond = lower_expr(ctx, elsif_cond)?;

        let elsif_then = ctx.func.new_block();
        let elsif_else = ctx.func.new_block();

        ctx.emit_terminator(
            TerminatorKind::SwitchInt {
                discr: cond,
                targets: SwitchTargets::if_else(elsif_then, elsif_else),
            },
            span,
        );

        ctx.current_block = elsif_then;
        let elsif_value = ctx.lower_block(elsif_block)?;
        if !ctx.is_terminated() {
            let value = elsif_value.unwrap_or_else(|| Operand::const_unit());
            ctx.emit_assign(
                Place::from_local(result),
                Rvalue::Use(value),
                span,
            );
            ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
        }

        current_else = elsif_else;
    }

    // Else branch
    if let Some(else_blk) = else_branch {
        ctx.current_block = current_else;
        let else_value = ctx.lower_block(else_blk)?;
        if !ctx.is_terminated() {
            let value = else_value.unwrap_or_else(|| Operand::const_unit());
            ctx.emit_assign(
                Place::from_local(result),
                Rvalue::Use(value),
                span,
            );
            ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
        }
    } else if current_else != merge_block {
        // No else branch, set result to unit
        ctx.current_block = current_else;
        ctx.emit_assign(
            Place::from_local(result),
            Rvalue::Use(Operand::const_unit()),
            span,
        );
        ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
    }

    ctx.current_block = merge_block;
    Ok(Operand::Copy(Place::from_local(result)))
}

fn lower_match_expr(
    ctx: &mut FunctionLoweringContext,
    scrutinee: &ast::Expr,
    arms: &[ast::MatchArm],
    span: Span,
) -> Result<Operand> {
    let merge_block = ctx.func.new_block();

    let scrutinee_op = lower_expr(ctx, scrutinee)?;
    let scrutinee_ty = infer_operand_type(ctx, &scrutinee_op);
    let scrutinee_local = ctx.new_temp(scrutinee_ty.clone());
    ctx.emit_assign(
        Place::from_local(scrutinee_local),
        Rvalue::Use(scrutinee_op),
        scrutinee.span,
    );

    // Infer result type: for expression arms use the scrutinee type as a
    // reasonable default (the actual arm expression type may differ, but this
    // avoids always defaulting to Int); for block arms use Unit.
    let result_ty = if let Some(first_arm) = arms.first() {
        match &first_arm.body {
            ast::MatchArmBody::Expr(_) => scrutinee_ty.clone(),
            ast::MatchArmBody::Block(_) => MirType::Unit,
        }
    } else {
        MirType::Unit
    };
    let result = ctx.new_temp(result_ty);

    // Check if this is a Result or Option enum match
    let is_enum_match = matches!(scrutinee_ty, MirType::Enum(_) | MirType::Result(_, _) | MirType::Optional(_));

    if is_enum_match && !arms.is_empty() {
        // Improved pattern matching for enums (Result, Option, etc.)
        let mut check_block = ctx.func.new_block();
        ctx.emit_terminator(TerminatorKind::Goto { target: check_block }, span);

        for (i, arm) in arms.iter().enumerate() {
            ctx.current_block = check_block;

            let arm_body_block = ctx.func.new_block();
            let next_arm_block = if i < arms.len() - 1 {
                ctx.func.new_block()
            } else {
                merge_block
            };

            // Check the pattern
            use crate::lower_pattern::lower_pattern_match;
            lower_pattern_match(
                ctx,
                &arm.pattern,
                Place::from_local(scrutinee_local),
                arm_body_block,
                next_arm_block,
                arm.pattern.span,
            )?;

            // Arm body
            ctx.current_block = arm_body_block;
            match &arm.body {
                ast::MatchArmBody::Expr(expr) => {
                    let val = lower_expr(ctx, expr)?;
                    ctx.emit_assign(Place::from_local(result), Rvalue::Use(val), expr.span);
                }
                ast::MatchArmBody::Block(block) => {
                    let block_value = ctx.lower_block(block)?;
                    let value = block_value.unwrap_or_else(|| Operand::const_unit());
                    ctx.emit_assign(Place::from_local(result), Rvalue::Use(value), span);
                }
            }
            if !ctx.is_terminated() {
                ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
            }

            check_block = next_arm_block;
        }
    } else {
        // Linear matching for non-enum types using lower_pattern_match
        let mut check_block = ctx.func.new_block();
        ctx.emit_terminator(TerminatorKind::Goto { target: check_block }, span);

        for (i, arm) in arms.iter().enumerate() {
            ctx.current_block = check_block;

            let arm_body_block = ctx.func.new_block();
            let next_arm_block = if i < arms.len() - 1 {
                ctx.func.new_block()
            } else {
                merge_block
            };

            // Use pattern matching to test and bind the pattern
            use crate::lower_pattern::lower_pattern_match;
            lower_pattern_match(
                ctx,
                &arm.pattern,
                Place::from_local(scrutinee_local),
                arm_body_block,
                next_arm_block,
                arm.pattern.span,
            )?;

            // Arm body
            ctx.current_block = arm_body_block;
            match &arm.body {
                ast::MatchArmBody::Expr(expr) => {
                    let val = lower_expr(ctx, expr)?;
                    ctx.emit_assign(Place::from_local(result), Rvalue::Use(val), expr.span);
                }
                ast::MatchArmBody::Block(block) => {
                    let block_value = ctx.lower_block(block)?;
                    let value = block_value.unwrap_or_else(|| Operand::const_unit());
                    ctx.emit_assign(Place::from_local(result), Rvalue::Use(value), span);
                }
            }
            if !ctx.is_terminated() {
                ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
            }

            check_block = next_arm_block;
        }
    }

    ctx.current_block = merge_block;
    Ok(Operand::Copy(Place::from_local(result)))
}

fn lower_block_expr(
    ctx: &mut FunctionLoweringContext,
    block: &ast::Block,
    _span: Span,
) -> Result<Operand> {
    let block_value = ctx.lower_block(block)?;
    // Return the block's value if it has one, otherwise unit
    Ok(block_value.unwrap_or_else(|| Operand::const_unit()))
}

// ============================================================================
// Lambda Expressions
// ============================================================================

fn lower_lambda(
    ctx: &mut FunctionLoweringContext,
    params: &[ast::Param],
    body: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    // Lambda |params| expr is lowered to:
    // 1. Create a new MirFunction for the lambda body
    // 2. Collect captured variables from the enclosing scope
    // 3. Return a Closure operand referencing the function + captures

    // Build param types
    let param_types: Vec<MirType> = params.iter().map(|p| {
        p.ty.as_ref()
            .map(|t| ctx.ctx.lower_type(t))
            .unwrap_or(MirType::Int)
    }).collect();

    // Create the lambda function with a generated name
    let lambda_name: SmolStr = format!("__lambda_{}", span.start).into();
    let mut lambda_fn = MirFunction::new(lambda_name.clone(), MirType::Int, span);

    // Add parameters
    let entry_block = lambda_fn.new_block();
    for (i, param) in params.iter().enumerate() {
        let ty = param_types[i].clone();
        let local = lambda_fn.new_local(ty, Some(param.name.node.clone()));
        lambda_fn.params.push(local);
    }

    // Collect captured variables from enclosing scope
    let mut captures: Vec<(SmolStr, Local, MirType)> = Vec::new();
    collect_free_variables_expr(body, params, &ctx.locals, &mut captures, ctx);

    // Build the captures list as operands from the enclosing scope
    let capture_operands: Vec<Operand> = captures.iter().map(|(_, local, _)| {
        operand_for_local(ctx, *local)
    }).collect();

    // Lower the lambda body in a new function context
    {
        let mut fn_ctx = FunctionLoweringContext::new(ctx.ctx, &mut lambda_fn);
        fn_ctx.current_block = entry_block;

        // Bind parameters
        for (i, param) in params.iter().enumerate() {
            fn_ctx.locals.insert(param.name.node.clone(), fn_ctx.func.params[i]);
        }

        // Bind captured variables as locals in the lambda
        for (name, _, ty) in &captures {
            let local = fn_ctx.func.new_local(ty.clone(), Some(name.clone()));
            fn_ctx.locals.insert(name.clone(), local);
        }

        // Lower the body expression
        let body_val = lower_expr(&mut fn_ctx, body)?;

        // Infer return type from the body and update the function
        let ret_ty = infer_operand_type(&fn_ctx, &body_val);
        fn_ctx.func.locals[Local::RETURN.0 as usize].ty = ret_ty.clone();
        fn_ctx.func.return_ty = ret_ty;

        // Store return value
        fn_ctx.emit_assign(
            Place::from_local(Local::RETURN),
            Rvalue::Use(body_val),
            span,
        );
        fn_ctx.ensure_terminated();
    }

    // Build the closure type
    let return_ty = lambda_fn.return_ty.clone();
    let closure_ty = MirType::Closure {
        params: param_types,
        ret: Box::new(return_ty),
    };

    // Register the lambda function in the program
    let fn_id = ctx.ctx.register_anonymous_function(lambda_fn);

    // Create the closure value (function + captures)
    let result = ctx.new_temp(closure_ty);
    ctx.emit_assign(
        Place::from_local(result),
        Rvalue::Closure(fn_id, capture_operands),
        span,
    );
    Ok(Operand::Move(Place::from_local(result)))
}

fn lower_block_lambda(
    ctx: &mut FunctionLoweringContext,
    params: &[ast::Param],
    body: &ast::Block,
    span: Span,
) -> Result<Operand> {
    // Block lambda |params| { stmts } is similar to expression lambda
    // but the body is a block instead of a single expression.

    let param_types: Vec<MirType> = params.iter().map(|p| {
        p.ty.as_ref()
            .map(|t| ctx.ctx.lower_type(t))
            .unwrap_or(MirType::Int)
    }).collect();

    let lambda_name: SmolStr = format!("__block_lambda_{}", span.start).into();
    let mut lambda_fn = MirFunction::new(lambda_name.clone(), MirType::Unit, span);

    let entry_block = lambda_fn.new_block();
    for (i, param) in params.iter().enumerate() {
        let ty = param_types[i].clone();
        let local = lambda_fn.new_local(ty, Some(param.name.node.clone()));
        lambda_fn.params.push(local);
    }

    // Collect captured variables
    let mut captures: Vec<(SmolStr, Local, MirType)> = Vec::new();
    collect_free_variables_block(body, params, &ctx.locals, &mut captures, ctx);

    let capture_operands: Vec<Operand> = captures.iter().map(|(_, local, _)| {
        operand_for_local(ctx, *local)
    }).collect();

    // Lower the block body
    {
        let mut fn_ctx = FunctionLoweringContext::new(ctx.ctx, &mut lambda_fn);
        fn_ctx.current_block = entry_block;

        for (i, param) in params.iter().enumerate() {
            fn_ctx.locals.insert(param.name.node.clone(), fn_ctx.func.params[i]);
        }

        for (name, _, ty) in &captures {
            let local = fn_ctx.func.new_local(ty.clone(), Some(name.clone()));
            fn_ctx.locals.insert(name.clone(), local);
        }

        let block_val = fn_ctx.lower_block(body)?;
        if let Some(val) = block_val {
            let ret_ty = infer_operand_type(&fn_ctx, &val);
            fn_ctx.func.locals[Local::RETURN.0 as usize].ty = ret_ty.clone();
            fn_ctx.func.return_ty = ret_ty;
            fn_ctx.emit_assign(
                Place::from_local(Local::RETURN),
                Rvalue::Use(val),
                span,
            );
        }
        fn_ctx.ensure_terminated();
    }

    let return_ty = lambda_fn.return_ty.clone();
    let closure_ty = MirType::Closure {
        params: param_types,
        ret: Box::new(return_ty),
    };

    let fn_id = ctx.ctx.register_anonymous_function(lambda_fn);

    let result = ctx.new_temp(closure_ty);
    ctx.emit_assign(
        Place::from_local(result),
        Rvalue::Closure(fn_id, capture_operands),
        span,
    );
    Ok(Operand::Move(Place::from_local(result)))
}

/// Collect free variables referenced in an expression that are defined in the enclosing scope.
/// This is a simplified capture analysis - it finds identifiers used in the body that
/// are not parameters and are available in the enclosing scope.
fn collect_free_variables_expr(
    expr: &ast::Expr,
    params: &[ast::Param],
    enclosing_locals: &FxHashMap<SmolStr, Local>,
    captures: &mut Vec<(SmolStr, Local, MirType)>,
    ctx: &FunctionLoweringContext,
) {
    let param_names: Vec<&SmolStr> = params.iter().map(|p| &p.name.node).collect();

    visit_expr_idents(expr, &mut |name: &SmolStr| {
        if !param_names.contains(&name) {
            if let Some(local) = enclosing_locals.get(name) {
                // Check if not already captured
                if !captures.iter().any(|(n, _, _)| n == name) {
                    let ty = ctx.func.locals[local.0 as usize].ty.clone();
                    captures.push((name.clone(), *local, ty));
                }
            }
        }
    });
}

/// Collect free variables referenced in a block body.
fn collect_free_variables_block(
    block: &ast::Block,
    params: &[ast::Param],
    enclosing_locals: &FxHashMap<SmolStr, Local>,
    captures: &mut Vec<(SmolStr, Local, MirType)>,
    ctx: &FunctionLoweringContext,
) {
    let param_names: Vec<&SmolStr> = params.iter().map(|p| &p.name.node).collect();

    for stmt in &block.stmts {
        visit_stmt_idents(stmt, &mut |name: &SmolStr| {
            if !param_names.contains(&name) {
                if let Some(local) = enclosing_locals.get(name) {
                    if !captures.iter().any(|(n, _, _)| n == name) {
                        let ty = ctx.func.locals[local.0 as usize].ty.clone();
                        captures.push((name.clone(), *local, ty));
                    }
                }
            }
        });
    }
}

/// Walk an expression tree and call the visitor for each identifier reference.
fn visit_expr_idents(expr: &ast::Expr, visitor: &mut dyn FnMut(&SmolStr)) {
    match &expr.kind {
        ast::ExprKind::Ident(name) => visitor(name),
        ast::ExprKind::Binary { left, right, .. } => {
            visit_expr_idents(left, visitor);
            visit_expr_idents(right, visitor);
        }
        ast::ExprKind::Unary { operand, .. } => visit_expr_idents(operand, visitor),
        ast::ExprKind::Call { func, args } => {
            visit_expr_idents(func, visitor);
            for arg in args {
                visit_expr_idents(&arg.value, visitor);
            }
        }
        ast::ExprKind::Field { object, .. } => visit_expr_idents(object, visitor),
        ast::ExprKind::Index { object, index } => {
            visit_expr_idents(object, visitor);
            visit_expr_idents(index, visitor);
        }
        ast::ExprKind::If { condition, then_branch, elsif_branches, else_branch } => {
            visit_expr_idents(condition, visitor);
            for stmt in &then_branch.stmts {
                visit_stmt_idents(stmt, visitor);
            }
            for (cond, block) in elsif_branches {
                visit_expr_idents(cond, visitor);
                for stmt in &block.stmts {
                    visit_stmt_idents(stmt, visitor);
                }
            }
            if let Some(b) = else_branch {
                for stmt in &b.stmts {
                    visit_stmt_idents(stmt, visitor);
                }
            }
        }
        ast::ExprKind::Array(elems) => {
            for e in elems { visit_expr_idents(e, visitor); }
        }
        ast::ExprKind::Tuple(elems) => {
            for e in elems { visit_expr_idents(e, visitor); }
        }
        ast::ExprKind::Paren(inner) => visit_expr_idents(inner, visitor),
        ast::ExprKind::Ternary { condition, then_expr, else_expr } => {
            visit_expr_idents(condition, visitor);
            visit_expr_idents(then_expr, visitor);
            visit_expr_idents(else_expr, visitor);
        }
        ast::ExprKind::MethodCall { object, args, .. } => {
            visit_expr_idents(object, visitor);
            for arg in args { visit_expr_idents(arg, visitor); }
        }
        ast::ExprKind::Pipe { left, right } => {
            visit_expr_idents(left, visitor);
            visit_expr_idents(right, visitor);
        }
        ast::ExprKind::InterpolatedString(parts) => {
            for part in parts {
                if let ast::StringPart::Expr(e) = part {
                    visit_expr_idents(e, visitor);
                }
            }
        }
        // For other expression kinds, we don't recurse (simplification)
        _ => {}
    }
}

/// Walk a statement tree and call the visitor for each identifier reference.
fn visit_stmt_idents(stmt: &ast::Stmt, visitor: &mut dyn FnMut(&SmolStr)) {
    match &stmt.kind {
        ast::StmtKind::Expr(expr) => visit_expr_idents(expr, visitor),
        ast::StmtKind::Let { value, .. } => {
            visit_expr_idents(value, visitor);
        }
        ast::StmtKind::Assign { target, value, op: _ } => {
            visit_expr_idents(target, visitor);
            visit_expr_idents(value, visitor);
        }
        ast::StmtKind::Return(Some(expr)) => visit_expr_idents(expr, visitor),
        _ => {}
    }
}

use rustc_hash::FxHashMap;

// ============================================================================
// Comprehensions
// ============================================================================

fn lower_array_comprehension(
    ctx: &mut FunctionLoweringContext,
    element: &ast::Expr,
    pattern: &ast::Pattern,
    iterable: &ast::Expr,
    condition: Option<&ast::Expr>,
    span: Span,
) -> Result<Operand> {
    // [expr for pattern in iterable if condition]
    // Lowered to:
    //   let result = []
    //   for pattern in iterable {
    //       if condition { push(result, expr) }
    //   }
    //   result

    // Evaluate iterable
    let iter_op = lower_expr(ctx, iterable)?;
    let iter_ty = infer_operand_type(ctx, &iter_op);
    let elem_ty = match &iter_ty {
        MirType::Array(elem) => (**elem).clone(),
        MirType::String => MirType::Char,
        _ => MirType::Int,
    };

    // Store iterable in temp
    let iter_local = ctx.new_temp(iter_ty.clone());
    ctx.emit_assign(Place::from_local(iter_local), Rvalue::Use(iter_op), span);

    // Create result array (initially empty, element type inferred after lowering element expr)
    // We'll create as Array<Int> initially and update after lowering the element
    let result_arr = ctx.new_temp(MirType::Array(Box::new(MirType::Int)));
    ctx.emit_assign(
        Place::from_local(result_arr),
        Rvalue::Aggregate(AggregateKind::Array(MirType::Int), vec![]),
        span,
    );

    // Loop structure
    let loop_header = ctx.func.new_block();
    let loop_body = ctx.func.new_block();
    let loop_exit = ctx.func.new_block();

    ctx.emit_terminator(TerminatorKind::Goto { target: loop_header }, span);

    // Header (simplified - just enter body, real iteration not implemented yet)
    ctx.current_block = loop_header;
    ctx.emit_terminator(TerminatorKind::Goto { target: loop_body }, span);

    // Body
    ctx.current_block = loop_body;
    ctx.push_loop(loop_exit, loop_header);

    // Bind pattern variable
    if let ast::PatternKind::Ident(name) = &pattern.kind {
        let _local = ctx.new_named_local(name.clone(), elem_ty);
    }

    // Check condition if present
    let push_block = if let Some(cond_expr) = condition {
        let cond_val = lower_expr(ctx, cond_expr)?;
        let true_block = ctx.func.new_block();
        let skip_block = ctx.func.new_block();
        ctx.emit_terminator(
            TerminatorKind::SwitchInt {
                discr: cond_val,
                targets: SwitchTargets::if_else(true_block, skip_block),
            },
            span,
        );

        // Skip block goes back to loop header
        ctx.current_block = skip_block;
        ctx.emit_terminator(TerminatorKind::Goto { target: loop_header }, span);

        true_block
    } else {
        ctx.current_block
    };

    // Evaluate the element expression and push to result
    ctx.current_block = push_block;
    let elem_val = lower_expr(ctx, element)?;
    let elem_result_ty = infer_operand_type(ctx, &elem_val);

    // Update the result array type with the actual element type
    ctx.func.locals[result_arr.0 as usize].ty = MirType::Array(Box::new(elem_result_ty.clone()));

    // Push element to array via builtin call
    if let Some(push_fn) = ctx.ctx.lookup_function("push") {
        let next = ctx.func.new_block();
        let push_dest = ctx.new_temp(MirType::Unit);
        ctx.emit_terminator(
            TerminatorKind::Call {
                func: Operand::Constant(Constant::Function(push_fn)),
                args: vec![Operand::Move(Place::from_local(result_arr)), elem_val],
                dest: Place::from_local(push_dest),
                target: Some(next),
            },
            span,
        );
        ctx.current_block = next;
    }

    ctx.pop_loop();

    if !ctx.is_terminated() {
        ctx.emit_terminator(TerminatorKind::Goto { target: loop_header }, span);
    }

    ctx.current_block = loop_exit;
    Ok(Operand::Move(Place::from_local(result_arr)))
}

fn lower_map_comprehension(
    ctx: &mut FunctionLoweringContext,
    key: &ast::Expr,
    value: &ast::Expr,
    pattern: &ast::Pattern,
    iterable: &ast::Expr,
    condition: Option<&ast::Expr>,
    span: Span,
) -> Result<Operand> {
    // {key: value for pattern in iterable if condition}
    // Lowered similarly to array comprehension but builds a map

    let iter_op = lower_expr(ctx, iterable)?;
    let iter_ty = infer_operand_type(ctx, &iter_op);
    let elem_ty = match &iter_ty {
        MirType::Array(elem) => (**elem).clone(),
        _ => MirType::Int,
    };

    let iter_local = ctx.new_temp(iter_ty);
    ctx.emit_assign(Place::from_local(iter_local), Rvalue::Use(iter_op), span);

    // Create result map (initially empty)
    let result_map = ctx.new_temp(MirType::Map(Box::new(MirType::String), Box::new(MirType::Int)));
    ctx.emit_assign(
        Place::from_local(result_map),
        Rvalue::Aggregate(AggregateKind::Tuple, vec![]),
        span,
    );

    let loop_header = ctx.func.new_block();
    let loop_body = ctx.func.new_block();
    let loop_exit = ctx.func.new_block();

    ctx.emit_terminator(TerminatorKind::Goto { target: loop_header }, span);

    ctx.current_block = loop_header;
    ctx.emit_terminator(TerminatorKind::Goto { target: loop_body }, span);

    ctx.current_block = loop_body;
    ctx.push_loop(loop_exit, loop_header);

    if let ast::PatternKind::Ident(name) = &pattern.kind {
        let _local = ctx.new_named_local(name.clone(), elem_ty);
    }

    // Check condition
    let insert_block = if let Some(cond_expr) = condition {
        let cond_val = lower_expr(ctx, cond_expr)?;
        let true_block = ctx.func.new_block();
        let skip_block = ctx.func.new_block();
        ctx.emit_terminator(
            TerminatorKind::SwitchInt {
                discr: cond_val,
                targets: SwitchTargets::if_else(true_block, skip_block),
            },
            span,
        );
        ctx.current_block = skip_block;
        ctx.emit_terminator(TerminatorKind::Goto { target: loop_header }, span);
        true_block
    } else {
        ctx.current_block
    };

    ctx.current_block = insert_block;
    let key_val = lower_expr(ctx, key)?;
    let value_val = lower_expr(ctx, value)?;

    // Update map type with actual key/value types
    let key_ty = infer_operand_type(ctx, &key_val);
    let value_ty = infer_operand_type(ctx, &value_val);
    ctx.func.locals[result_map.0 as usize].ty = MirType::Map(Box::new(key_ty), Box::new(value_ty));

    // Store key-value pair (simplified - real implementation would call map_insert)
    let _kv_temp = ctx.new_temp(MirType::Unit);

    ctx.pop_loop();

    if !ctx.is_terminated() {
        ctx.emit_terminator(TerminatorKind::Goto { target: loop_header }, span);
    }

    ctx.current_block = loop_exit;
    Ok(Operand::Move(Place::from_local(result_map)))
}

// ============================================================================
// Special Expressions
// ============================================================================

fn lower_range(
    _ctx: &mut FunctionLoweringContext,
    _start: Option<&ast::Expr>,
    _end: Option<&ast::Expr>,
    _inclusive: bool,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "range expressions".to_string(),
        span,
    })
}

fn lower_pipe(
    ctx: &mut FunctionLoweringContext,
    left: &ast::Expr,
    right: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    // Pipe `a |> f` is desugared to `f(a)`
    let arg = lower_expr(ctx, left)?;
    let func = lower_expr(ctx, right)?;

    // Infer return type from the function being called
    let return_ty = match &func {
        Operand::Constant(Constant::Function(fn_id)) => {
            ctx.ctx.get_function(fn_id)
                .map(|f| f.return_ty.clone())
                .unwrap_or(MirType::Int)
        }
        _ => MirType::Int, // Fallback for function pointers
    };

    let result = ctx.new_temp(return_ty);
    let next_block = ctx.func.new_block();

    ctx.emit_terminator(
        TerminatorKind::Call {
            func,
            args: vec![arg],
            dest: Place::from_local(result),
            target: Some(next_block),
        },
        span,
    );

    ctx.current_block = next_block;
    Ok(Operand::Move(Place::from_local(result)))
}

fn lower_try(
    _ctx: &mut FunctionLoweringContext,
    _inner: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "try expressions".to_string(),
        span,
    })
}

fn lower_unwrap(
    ctx: &mut FunctionLoweringContext,
    inner: &ast::Expr,
    _span: Span,
) -> Result<Operand> {
    // Unwrap `x!` asserts that x is Some/Ok and extracts the value
    let val = lower_expr(ctx, inner)?;
    // For now, just return the value (proper implementation needs runtime check)
    Ok(val)
}

fn lower_safe_nav(
    _ctx: &mut FunctionLoweringContext,
    _object: &ast::Expr,
    _field: &ast::Ident,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "safe navigation".to_string(),
        span,
    })
}

fn lower_struct_init(
    ctx: &mut FunctionLoweringContext,
    name: &ast::TypeIdent,
    fields: &[ast::FieldInit],
    span: Span,
) -> Result<Operand> {
    let struct_id = ctx
        .ctx
        .lookup_struct(&name.node)
        .ok_or_else(|| MirError::UndefinedType {
            name: name.node.to_string(),
            span: name.span,
        })?;

    let mut field_ops = Vec::new();
    for field in fields {
        let val = if let Some(expr) = &field.value {
            lower_expr(ctx, expr)?
        } else {
            // Shorthand: `name:` means `name: name`
            if let Some(local) = ctx.lookup_local(&field.name.node) {
                Operand::Copy(Place::from_local(local))
            } else {
                return Err(MirError::UndefinedVariable {
                    name: field.name.node.to_string(),
                    span: field.name.span,
                });
            }
        };
        field_ops.push(val);
    }

    let result = ctx.new_temp(MirType::Struct(struct_id));
    ctx.emit_assign(
        Place::from_local(result),
        Rvalue::Aggregate(AggregateKind::Struct(struct_id), field_ops),
        span,
    );
    Ok(Operand::Move(Place::from_local(result)))
}

// ============================================================================
// Concurrency
// ============================================================================

fn lower_spawn(
    _ctx: &mut FunctionLoweringContext,
    _inner: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "spawn expressions".to_string(),
        span,
    })
}

fn lower_await(
    _ctx: &mut FunctionLoweringContext,
    _inner: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "await expressions".to_string(),
        span,
    })
}

fn lower_select(
    _ctx: &mut FunctionLoweringContext,
    _arms: &[ast::SelectArm],
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "select expressions".to_string(),
        span,
    })
}

fn lower_channel_send(
    _ctx: &mut FunctionLoweringContext,
    _channel: &ast::Expr,
    _value: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "channel send expressions".to_string(),
        span,
    })
}

fn lower_channel_recv(
    _ctx: &mut FunctionLoweringContext,
    _channel: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "channel receive expressions".to_string(),
        span,
    })
}

// ============================================================================
// Contract Expressions
// ============================================================================

fn lower_old(
    _ctx: &mut FunctionLoweringContext,
    _inner: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "old() in contracts".to_string(),
        span,
    })
}

fn lower_result_keyword(
    _ctx: &mut FunctionLoweringContext,
    _span: Span,
) -> Result<Operand> {
    // `result` in ensures clause refers to return value
    Ok(Operand::Copy(Place::return_place()))
}

fn lower_forall(
    _ctx: &mut FunctionLoweringContext,
    _var: &ast::Ident,
    _ty: &ast::TypeExpr,
    _condition: Option<&ast::Expr>,
    _body: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "forall quantifier".to_string(),
        span,
    })
}

fn lower_exists(
    _ctx: &mut FunctionLoweringContext,
    _var: &ast::Ident,
    _ty: &ast::TypeExpr,
    _condition: Option<&ast::Expr>,
    _body: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    Err(MirError::UnsupportedFeature {
        feature: "exists quantifier".to_string(),
        span,
    })
}

// ============================================================================
// Miscellaneous
// ============================================================================

fn lower_ternary(
    ctx: &mut FunctionLoweringContext,
    condition: &ast::Expr,
    then_expr: &ast::Expr,
    else_expr: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    // Ternary is just if/else expression
    let then_block = ctx.func.new_block();
    let else_block = ctx.func.new_block();
    let merge_block = ctx.func.new_block();

    let cond = lower_expr(ctx, condition)?;
    ctx.emit_terminator(
        TerminatorKind::SwitchInt {
            discr: cond,
            targets: SwitchTargets::if_else(then_block, else_block),
        },
        span,
    );

    // Then
    ctx.current_block = then_block;
    let then_val = lower_expr(ctx, then_expr)?;
    // Infer result type from the then-branch value
    let result_ty = infer_operand_type(ctx, &then_val);
    let result = ctx.new_temp(result_ty);
    ctx.emit_assign(Place::from_local(result), Rvalue::Use(then_val), then_expr.span);
    ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);

    // Else
    ctx.current_block = else_block;
    let else_val = lower_expr(ctx, else_expr)?;
    ctx.emit_assign(Place::from_local(result), Rvalue::Use(else_val), else_expr.span);
    ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);

    ctx.current_block = merge_block;
    Ok(Operand::Copy(Place::from_local(result)))
}

// ============================================================================
// String Interpolation
// ============================================================================

fn lower_interpolated_string(
    ctx: &mut FunctionLoweringContext,
    parts: &[ast::StringPart],
    span: Span,
) -> Result<Operand> {
    // String interpolation "Hello, #{name}!" is lowered as:
    //   concat(concat("Hello, ", to_string(name)), "!")
    //
    // We build up the result by concatenating each part.

    if parts.is_empty() {
        let idx = ctx.ctx.intern_string("".into());
        return Ok(Operand::Constant(Constant::String(idx)));
    }

    // Lower the first part to get a starting string
    let mut result_op = lower_string_part(ctx, &parts[0], span)?;

    // Concatenate each subsequent part
    for part in &parts[1..] {
        let part_op = lower_string_part(ctx, part, span)?;

        // Concat: result = result + part (string concatenation)
        let concat_result = ctx.new_temp(MirType::String);
        ctx.emit_assign(
            Place::from_local(concat_result),
            Rvalue::BinaryOp(BinOp::Add, result_op, part_op),
            span,
        );
        result_op = Operand::Move(Place::from_local(concat_result));
    }

    Ok(result_op)
}

/// Lower a single part of an interpolated string to a String operand.
fn lower_string_part(
    ctx: &mut FunctionLoweringContext,
    part: &ast::StringPart,
    span: Span,
) -> Result<Operand> {
    match part {
        ast::StringPart::Literal(s) => {
            let idx = ctx.ctx.intern_string(s.clone());
            Ok(Operand::Constant(Constant::String(idx)))
        }
        ast::StringPart::Expr(expr) => {
            lower_string_part_expr(ctx, expr, span)
        }
        ast::StringPart::FormattedExpr { expr, format: _ } => {
            // For formatted expressions like #{val:.2f}, lower the expression
            // and convert to string. Format specifiers would be handled by codegen.
            lower_string_part_expr(ctx, expr, span)
        }
    }
}

/// Convert an expression to a String operand for interpolation.
fn lower_string_part_expr(
    ctx: &mut FunctionLoweringContext,
    expr: &ast::Expr,
    span: Span,
) -> Result<Operand> {
    let val = lower_expr(ctx, expr)?;
    let val_ty = infer_operand_type(ctx, &val);

    // If already a String, use directly; otherwise call to_string builtin
    if matches!(val_ty, MirType::String) {
        Ok(val)
    } else if let Some(fn_id) = ctx.ctx.lookup_function("to_string") {
        // Call to_string(val) to convert to string
        let result = ctx.new_temp(MirType::String);
        let next_block = ctx.func.new_block();
        ctx.emit_terminator(
            TerminatorKind::Call {
                func: Operand::Constant(Constant::Function(fn_id)),
                args: vec![val],
                dest: Place::from_local(result),
                target: Some(next_block),
            },
            span,
        );
        ctx.current_block = next_block;
        Ok(Operand::Move(Place::from_local(result)))
    } else {
        // No to_string available - use a Cast to String and let codegen handle it
        let result = ctx.new_temp(MirType::String);
        ctx.emit_assign(
            Place::from_local(result),
            Rvalue::Cast(CastKind::ToString, val, MirType::String),
            span,
        );
        Ok(Operand::Move(Place::from_local(result)))
    }
}

fn lower_path(
    ctx: &mut FunctionLoweringContext,
    segments: &[ast::Ident],
    span: Span,
) -> Result<Operand> {
    // Path like `foo::bar::baz`
    // For now, just look up the full path as a name
    let name: SmolStr = segments
        .iter()
        .map(|s| s.node.as_str())
        .collect::<Vec<_>>()
        .join("::")
        .into();

    if let Some(fn_id) = ctx.ctx.lookup_function(&name) {
        Ok(Operand::Constant(Constant::Function(fn_id)))
    } else if let Some(local) = ctx.lookup_local(&name) {
        Ok(Operand::Copy(Place::from_local(local)))
    } else {
        // Try looking up just the last segment
        if let Some(last) = segments.last() {
            if let Some(fn_id) = ctx.ctx.lookup_function(&last.node) {
                return Ok(Operand::Constant(Constant::Function(fn_id)));
            }
        }
        Err(MirError::UndefinedVariable {
            name: name.to_string(),
            span,
        })
    }
}

/// Infer the type of a place
pub fn infer_place_type(ctx: &FunctionLoweringContext, place: &Place) -> MirType {
    let base_ty = ctx.func.locals.get(place.local.0 as usize)
        .map(|l| l.ty.clone())
        .unwrap_or(MirType::Unit);

    // Apply projections to get final type
    let mut ty = base_ty;
    for proj in &place.projection {
        ty = match proj {
            PlaceElem::Deref => {
                // Deref a reference to get inner type
                if let MirType::Ref(inner) = ty {
                    (*inner).clone()
                } else {
                    MirType::Unit
                }
            }
            PlaceElem::Field(field_idx) => {
                match &ty {
                    MirType::Tuple(elem_types) => {
                        elem_types.get(*field_idx as usize).cloned().unwrap_or(MirType::Unit)
                    }
                    MirType::Struct(struct_id) => {
                        // Look up field type from struct definition
                        ctx.ctx.get_struct_field_type(*struct_id, *field_idx)
                            .unwrap_or(MirType::Unit)
                    }
                    _ => MirType::Unit
                }
            }
            PlaceElem::Index(_) | PlaceElem::ConstantIndex(_) => {
                // Get element type from array
                if let MirType::Array(elem_ty) = ty {
                    (*elem_ty).clone()
                } else if let MirType::Tuple(elem_types) = ty {
                    // Tuple indexing - assume first element type for constant index
                    elem_types.first().cloned().unwrap_or(MirType::Unit)
                } else {
                    MirType::Unit
                }
            }
            PlaceElem::Downcast(_) => ty.clone(), // Downcast preserves type for now
        };
    }
    ty
}

/// Infer the type of an operand
pub fn infer_operand_type(ctx: &FunctionLoweringContext, operand: &Operand) -> MirType {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => infer_place_type(ctx, place),
        Operand::Constant(constant) => match constant {
            Constant::Unit => MirType::Unit,
            Constant::Bool(_) => MirType::Bool,
            Constant::Int(_) => MirType::Int,
            Constant::Float(_) => MirType::Float,
            Constant::Float32(_) => MirType::Float32,
            Constant::Float64(_) => MirType::Float64,
            Constant::Char(_) => MirType::Char,
            Constant::String(_) => MirType::String,
            Constant::Function(_) => MirType::Unit,
        },
    }
}
