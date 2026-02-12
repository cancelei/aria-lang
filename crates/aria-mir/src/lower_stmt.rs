//! Statement lowering from AST to MIR.

use aria_ast::{self as ast, BinaryOp};
use aria_lexer::Span;

use crate::lower::FunctionLoweringContext;
use crate::lower_expr::infer_operand_type;
use crate::lower_pattern::lower_pattern_binding;
use crate::mir::*;
use crate::{MirError, Result};

/// Lower a statement
pub fn lower_stmt(ctx: &mut FunctionLoweringContext, stmt: &ast::Stmt) -> Result<()> {
    match &stmt.kind {
        ast::StmtKind::Expr(expr) => {
            // Expression statement - evaluate for side effects
            ctx.lower_expr(expr)?;
            Ok(())
        }

        ast::StmtKind::Let { pattern, ty, value } => {
            lower_let(ctx, pattern, ty.as_ref(), value, stmt.span)
        }

        ast::StmtKind::Var { name, ty, value } => {
            lower_var(ctx, name, ty.as_ref(), value, stmt.span)
        }

        ast::StmtKind::Const { name, ty, value } => {
            lower_const(ctx, name, ty.as_ref(), value, stmt.span)
        }

        ast::StmtKind::Assign { target, op, value } => {
            lower_assign(ctx, target, *op, value, stmt.span)
        }

        ast::StmtKind::For {
            pattern,
            iterable,
            body,
        } => lower_for(ctx, pattern, iterable, body, stmt.span),

        ast::StmtKind::While { condition, body } => {
            lower_while(ctx, condition, body, stmt.span)
        }

        ast::StmtKind::Loop { body } => lower_loop(ctx, body, stmt.span),

        ast::StmtKind::If {
            condition,
            then_branch,
            elsif_branches,
            else_branch,
        } => lower_if_stmt(
            ctx,
            condition,
            then_branch,
            elsif_branches,
            else_branch.as_ref(),
            stmt.span,
        ),

        ast::StmtKind::Unless {
            condition,
            body,
            else_branch,
        } => lower_unless(ctx, condition, body, else_branch.as_ref(), stmt.span),

        ast::StmtKind::Match { scrutinee, arms } => {
            lower_match_stmt(ctx, scrutinee, arms, stmt.span)
        }

        ast::StmtKind::Return(value) => lower_return(ctx, value.as_ref(), stmt.span),

        ast::StmtKind::Break(value) => lower_break(ctx, value.as_ref(), stmt.span),

        ast::StmtKind::Continue => lower_continue(ctx, stmt.span),

        ast::StmtKind::Defer(expr) => lower_defer(ctx, expr, stmt.span),

        ast::StmtKind::Unsafe(block) => lower_unsafe(ctx, block, stmt.span),

        ast::StmtKind::Item(item) => {
            // Nested item declarations are handled at the program level
            // For functions, we need to lower them
            match item.as_ref() {
                ast::Item::Function(f) => ctx.ctx.lower_function(f),
                _ => Ok(()), // Other items are collected in the first pass
            }
        }
    }
}

// ============================================================================
// Variable Bindings
// ============================================================================

fn lower_let(
    ctx: &mut FunctionLoweringContext,
    pattern: &ast::Pattern,
    ty: Option<&ast::TypeExpr>,
    value: &ast::Expr,
    span: Span,
) -> Result<()> {
    // Evaluate the value
    let val = ctx.lower_expr(value)?;

    // Determine the type
    let mir_ty = if let Some(t) = ty {
        ctx.ctx.lower_type(t)
    } else {
        // Infer type from the value using the type inference system
        infer_operand_type(ctx, &val)
    };

    // Lower the pattern binding
    lower_pattern_binding(ctx, pattern, val, mir_ty, span)
}

fn lower_var(
    ctx: &mut FunctionLoweringContext,
    name: &ast::Ident,
    ty: Option<&ast::TypeExpr>,
    value: &ast::Expr,
    span: Span,
) -> Result<()> {
    // Var is like let but mutable
    let val = ctx.lower_expr(value)?;

    let mir_ty = if let Some(t) = ty {
        ctx.ctx.lower_type(t)
    } else {
        // Infer type from the value using the type inference system
        infer_operand_type(ctx, &val)
    };

    let local = ctx.new_named_local(name.node.clone(), mir_ty);
    ctx.emit_stmt(StatementKind::StorageLive(local), span);
    ctx.emit_assign(Place::from_local(local), Rvalue::Use(val), span);
    Ok(())
}

fn lower_const(
    ctx: &mut FunctionLoweringContext,
    name: &ast::Ident,
    ty: Option<&ast::TypeExpr>,
    value: &ast::Expr,
    span: Span,
) -> Result<()> {
    // Const is like let but immutable (same in MIR, different semantics)
    let val = ctx.lower_expr(value)?;

    let mir_ty = if let Some(t) = ty {
        ctx.ctx.lower_type(t)
    } else {
        // Infer type from the value using the type inference system
        infer_operand_type(ctx, &val)
    };

    let local = ctx.new_named_local(name.node.clone(), mir_ty);
    ctx.emit_stmt(StatementKind::StorageLive(local), span);
    ctx.emit_assign(Place::from_local(local), Rvalue::Use(val), span);
    Ok(())
}

// ============================================================================
// Assignment
// ============================================================================

fn lower_assign(
    ctx: &mut FunctionLoweringContext,
    target: &ast::Expr,
    op: Option<BinaryOp>,
    value: &ast::Expr,
    span: Span,
) -> Result<()> {
    let place = ctx.lower_expr_to_place(target)?;

    if let Some(compound_op) = op {
        // Compound assignment: x += 1 -> x = x + 1
        let current = Operand::Copy(place.clone());
        let rhs = ctx.lower_expr(value)?;

        let mir_op = match compound_op {
            BinaryOp::AddAssign | BinaryOp::Add => BinOp::Add,
            BinaryOp::SubAssign | BinaryOp::Sub => BinOp::Sub,
            BinaryOp::MulAssign | BinaryOp::Mul => BinOp::Mul,
            BinaryOp::DivAssign | BinaryOp::Div => BinOp::Div,
            BinaryOp::IntDivAssign | BinaryOp::IntDiv => BinOp::IntDiv,
            BinaryOp::ModAssign | BinaryOp::Mod => BinOp::Rem,
            BinaryOp::BitAndAssign | BinaryOp::BitAnd => BinOp::BitAnd,
            BinaryOp::BitOrAssign | BinaryOp::BitOr => BinOp::BitOr,
            BinaryOp::BitXorAssign | BinaryOp::BitXor => BinOp::BitXor,
            BinaryOp::ShlAssign | BinaryOp::Shl => BinOp::Shl,
            BinaryOp::ShrAssign | BinaryOp::Shr => BinOp::Shr,
            _ => {
                return Err(MirError::UnsupportedFeature {
                    feature: format!("compound assignment {:?}", compound_op),
                    span,
                })
            }
        };

        ctx.emit_assign(place, Rvalue::BinaryOp(mir_op, current, rhs), span);
    } else {
        // Simple assignment
        let val = ctx.lower_expr(value)?;
        ctx.emit_assign(place, Rvalue::Use(val), span);
    }

    Ok(())
}

// ============================================================================
// Loops
// ============================================================================

fn lower_for(
    ctx: &mut FunctionLoweringContext,
    pattern: &ast::Pattern,
    iterable: &ast::Expr,
    body: &ast::Block,
    span: Span,
) -> Result<()> {
    // For loops are lowered to:
    // let iter = iterable.into_iter()
    // loop {
    //     match iter.next() {
    //         Some(pattern) => body,
    //         None => break,
    //     }
    // }

    // For now, use a simplified version that iterates over ranges
    // TODO: Full iterator support

    let loop_header = ctx.func.new_block();
    let loop_body = ctx.func.new_block();
    let loop_exit = ctx.func.new_block();

    // Evaluate iterable
    let iter = ctx.lower_expr(iterable)?;

    // Infer element type from iterable
    let iter_ty = infer_operand_type(ctx, &iter);
    let elem_ty = match &iter_ty {
        MirType::Array(elem) => (**elem).clone(),
        MirType::String => MirType::Char,
        _ => MirType::Int, // Fallback for ranges and unknown iterables
    };

    // Jump to header
    ctx.emit_terminator(TerminatorKind::Goto { target: loop_header }, span);

    // Header: check if we should continue
    ctx.current_block = loop_header;
    // TODO: Proper iteration check
    // For now, just enter body (infinite loop placeholder)
    ctx.emit_terminator(TerminatorKind::Goto { target: loop_body }, span);

    // Body
    ctx.current_block = loop_body;
    ctx.push_loop(loop_exit, loop_header);

    // Bind pattern (simplified - just bind to pattern variable)
    // TODO: Proper pattern matching for destructuring
    if let ast::PatternKind::Ident(name) = &pattern.kind {
        let local = ctx.new_named_local(name.clone(), elem_ty);
        ctx.emit_stmt(StatementKind::StorageLive(local), pattern.span);
    }

    let _ = ctx.lower_block(body)?;
    ctx.pop_loop();

    if !ctx.is_terminated() {
        ctx.emit_terminator(TerminatorKind::Goto { target: loop_header }, span);
    }

    ctx.current_block = loop_exit;
    Ok(())
}

fn lower_while(
    ctx: &mut FunctionLoweringContext,
    condition: &ast::Expr,
    body: &ast::Block,
    span: Span,
) -> Result<()> {
    let cond_block = ctx.func.new_block();
    let body_block = ctx.func.new_block();
    let exit_block = ctx.func.new_block();

    // Jump to condition check
    ctx.emit_terminator(TerminatorKind::Goto { target: cond_block }, span);

    // Condition block
    ctx.current_block = cond_block;
    let cond = ctx.lower_expr(condition)?;
    ctx.emit_terminator(
        TerminatorKind::SwitchInt {
            discr: cond,
            targets: SwitchTargets::if_else(body_block, exit_block),
        },
        span,
    );

    // Body block
    ctx.current_block = body_block;
    ctx.push_loop(exit_block, cond_block);
    let _ = ctx.lower_block(body)?;
    ctx.pop_loop();

    if !ctx.is_terminated() {
        ctx.emit_terminator(TerminatorKind::Goto { target: cond_block }, span);
    }

    // Exit block
    ctx.current_block = exit_block;
    Ok(())
}

fn lower_loop(
    ctx: &mut FunctionLoweringContext,
    body: &ast::Block,
    span: Span,
) -> Result<()> {
    let body_block = ctx.func.new_block();
    let exit_block = ctx.func.new_block();

    // Jump to body
    ctx.emit_terminator(TerminatorKind::Goto { target: body_block }, span);

    // Body
    ctx.current_block = body_block;
    ctx.push_loop(exit_block, body_block);
    let _ = ctx.lower_block(body)?;
    ctx.pop_loop();

    if !ctx.is_terminated() {
        ctx.emit_terminator(TerminatorKind::Goto { target: body_block }, span);
    }

    // Exit (only reachable via break)
    ctx.current_block = exit_block;
    Ok(())
}

// ============================================================================
// Conditionals
// ============================================================================

fn lower_if_stmt(
    ctx: &mut FunctionLoweringContext,
    condition: &ast::Expr,
    then_branch: &ast::Block,
    elsif_branches: &[(ast::Expr, ast::Block)],
    else_branch: Option<&ast::Block>,
    span: Span,
) -> Result<()> {
    let merge_block = ctx.func.new_block();

    // Evaluate condition
    let cond = ctx.lower_expr(condition)?;

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

    // Then branch
    ctx.current_block = then_block;
    let _ = ctx.lower_block(then_branch)?;
    if !ctx.is_terminated() {
        ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
    }

    // Handle elsif branches
    let mut current_else = else_block;
    for (elsif_cond, elsif_body) in elsif_branches {
        if current_else == merge_block {
            break;
        }

        ctx.current_block = current_else;
        let cond = ctx.lower_expr(elsif_cond)?;

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
        let _ = ctx.lower_block(elsif_body)?;
        if !ctx.is_terminated() {
            ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
        }

        current_else = elsif_else;
    }

    // Else branch
    if let Some(else_blk) = else_branch {
        ctx.current_block = current_else;
        let _ = ctx.lower_block(else_blk)?;
        if !ctx.is_terminated() {
            ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
        }
    } else if current_else != merge_block {
        ctx.current_block = current_else;
        ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
    }

    ctx.current_block = merge_block;
    Ok(())
}

fn lower_unless(
    ctx: &mut FunctionLoweringContext,
    condition: &ast::Expr,
    body: &ast::Block,
    else_branch: Option<&ast::Block>,
    span: Span,
) -> Result<()> {
    // Unless is just if with negated condition
    let merge_block = ctx.func.new_block();

    let cond = ctx.lower_expr(condition)?;

    let body_block = ctx.func.new_block();
    let else_block = else_branch
        .map(|_| ctx.func.new_block())
        .unwrap_or(merge_block);

    // Unless = if NOT condition
    // So swap the targets
    ctx.emit_terminator(
        TerminatorKind::SwitchInt {
            discr: cond,
            targets: SwitchTargets::if_else(else_block, body_block), // Swapped!
        },
        span,
    );

    // Body (runs when condition is false)
    ctx.current_block = body_block;
    let _ = ctx.lower_block(body)?;
    if !ctx.is_terminated() {
        ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
    }

    // Else branch (runs when condition is true)
    if let Some(else_blk) = else_branch {
        ctx.current_block = else_block;
        let _ = ctx.lower_block(else_blk)?;
        if !ctx.is_terminated() {
            ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
        }
    }

    ctx.current_block = merge_block;
    Ok(())
}

fn lower_match_stmt(
    ctx: &mut FunctionLoweringContext,
    scrutinee: &ast::Expr,
    arms: &[ast::MatchArm],
    span: Span,
) -> Result<()> {
    let merge_block = ctx.func.new_block();

    let scrutinee_op = ctx.lower_expr(scrutinee)?;
    let scrutinee_ty = infer_operand_type(ctx, &scrutinee_op);
    let scrutinee_local = ctx.new_temp(scrutinee_ty);
    ctx.emit_assign(
        Place::from_local(scrutinee_local),
        Rvalue::Use(scrutinee_op),
        scrutinee.span,
    );

    // Linear chain pattern matching using lower_pattern_match
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
                ctx.lower_expr(expr)?;
            }
            ast::MatchArmBody::Block(block) => {
                let _ = ctx.lower_block(block)?;
            }
        }
        if !ctx.is_terminated() {
            ctx.emit_terminator(TerminatorKind::Goto { target: merge_block }, span);
        }

        check_block = next_arm_block;
    }

    ctx.current_block = merge_block;
    Ok(())
}

// ============================================================================
// Control Flow
// ============================================================================

fn lower_return(
    ctx: &mut FunctionLoweringContext,
    value: Option<&ast::Expr>,
    span: Span,
) -> Result<()> {
    let expected_ty = ctx.func.return_ty.clone();

    if let Some(expr) = value {
        let val = ctx.lower_expr(expr)?;

        // Infer the actual return type from the value
        let actual_ty = infer_return_operand_type(ctx, &val);

        // Validate type compatibility
        if !types_compatible(&expected_ty, &actual_ty) {
            return Err(MirError::TypeMismatch {
                expected: expected_ty.to_string(),
                found: actual_ty.to_string(),
                span,
            });
        }

        ctx.emit_assign(Place::return_place(), Rvalue::Use(val), expr.span);
    } else {
        // No return value - check that function expects Unit
        if !matches!(expected_ty, MirType::Unit) {
            return Err(MirError::TypeMismatch {
                expected: expected_ty.to_string(),
                found: "()".to_string(),
                span,
            });
        }
        ctx.emit_assign(
            Place::return_place(),
            Rvalue::Use(Operand::const_unit()),
            span,
        );
    }
    ctx.emit_terminator(TerminatorKind::Return, span);
    Ok(())
}

/// Check if two types are compatible (for return type validation)
fn types_compatible(expected: &MirType, actual: &MirType) -> bool {
    // Exact match
    if expected == actual {
        return true;
    }

    // Numeric coercion: Int can be returned where Float is expected
    if matches!(expected, MirType::Float | MirType::Float32 | MirType::Float64)
        && matches!(actual, MirType::Int)
    {
        return true;
    }

    // Allow any integer type to match Int
    if matches!(expected, MirType::Int)
        && matches!(
            actual,
            MirType::Int8
                | MirType::Int16
                | MirType::Int32
                | MirType::Int64
                | MirType::UInt
                | MirType::UInt8
                | MirType::UInt16
                | MirType::UInt32
                | MirType::UInt64
        )
    {
        return true;
    }

    // Unit is always compatible with itself
    if matches!(expected, MirType::Unit) && matches!(actual, MirType::Unit) {
        return true;
    }

    false
}

/// Infer the type of an operand for return validation
fn infer_return_operand_type(ctx: &FunctionLoweringContext, operand: &Operand) -> MirType {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => {
            // Get base type from local
            let base_ty = ctx
                .func
                .locals
                .get(place.local.0 as usize)
                .map(|l| l.ty.clone())
                .unwrap_or(MirType::Unit);

            // Apply projections
            let mut ty = base_ty;
            for proj in &place.projection {
                ty = match proj {
                    PlaceElem::Deref => {
                        if let MirType::Ref(inner) = ty {
                            (*inner).clone()
                        } else {
                            MirType::Unit
                        }
                    }
                    PlaceElem::Field(idx) => {
                        match &ty {
                            MirType::Tuple(elem_types) => {
                                elem_types.get(*idx as usize).cloned().unwrap_or(MirType::Unit)
                            }
                            MirType::Struct(sid) => {
                                ctx.ctx.get_struct_field_type(*sid, *idx).unwrap_or(MirType::Unit)
                            }
                            _ => MirType::Unit,
                        }
                    }
                    PlaceElem::Index(_) | PlaceElem::ConstantIndex(_) => {
                        if let MirType::Array(elem) = ty {
                            (*elem).clone()
                        } else {
                            MirType::Unit
                        }
                    }
                    PlaceElem::Downcast(_) => ty.clone(),
                };
            }
            ty
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
            Constant::Function(_) => MirType::Unit,
        },
    }
}

fn lower_break(
    ctx: &mut FunctionLoweringContext,
    value: Option<&ast::Expr>,
    span: Span,
) -> Result<()> {
    if let Some(target) = ctx.break_target() {
        if let Some(expr) = value {
            // Break with value - not directly supported in basic MIR
            // Would need loop result place
            let _val = ctx.lower_expr(expr)?;
        }
        ctx.emit_terminator(TerminatorKind::Goto { target }, span);
        Ok(())
    } else {
        Err(MirError::Internal {
            message: "break outside of loop".to_string(),
            span,
        })
    }
}

fn lower_continue(ctx: &mut FunctionLoweringContext, span: Span) -> Result<()> {
    if let Some(target) = ctx.continue_target() {
        ctx.emit_terminator(TerminatorKind::Goto { target }, span);
        Ok(())
    } else {
        Err(MirError::Internal {
            message: "continue outside of loop".to_string(),
            span,
        })
    }
}

fn lower_defer(
    _ctx: &mut FunctionLoweringContext,
    _expr: &ast::Expr,
    span: Span,
) -> Result<()> {
    // Defer needs special handling - the expression should be evaluated at scope exit
    // For now, just record that we need to handle this
    Err(MirError::UnsupportedFeature {
        feature: "defer statements".to_string(),
        span,
    })
}

fn lower_unsafe(
    ctx: &mut FunctionLoweringContext,
    block: &ast::Block,
    _span: Span,
) -> Result<()> {
    // In MIR, unsafe is mostly a marker for the type system
    // Just lower the block contents
    let _ = ctx.lower_block(block)?;
    Ok(())
}
