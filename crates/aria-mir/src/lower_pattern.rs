//! Pattern lowering from AST to MIR.
//!
//! Patterns in Aria are used in let bindings, match arms, and function parameters.
//! This module handles lowering patterns to MIR assignments and control flow.

use aria_ast::{self as ast, PatternKind};
use aria_lexer::Span;

use crate::lower::FunctionLoweringContext;
use crate::mir::*;
use crate::{MirError, Result};

/// Create an operand for a place based on whether its type is Copy.
///
/// This uses the ownership inference system to decide whether to copy or move.
fn operand_for_place(place: Place, ty: &MirType) -> Operand {
    if ty.is_copy() {
        Operand::Copy(place)
    } else {
        Operand::Move(place)
    }
}

/// Lower a pattern binding, assigning the value to pattern variables
pub fn lower_pattern_binding(
    ctx: &mut FunctionLoweringContext,
    pattern: &ast::Pattern,
    value: Operand,
    ty: MirType,
    span: Span,
) -> Result<()> {
    match &pattern.kind {
        PatternKind::Wildcard => {
            // Wildcard doesn't bind anything, just evaluate for side effects
            Ok(())
        }

        PatternKind::Ident(name) => {
            // Simple identifier binding
            let local = ctx.new_named_local(name.clone(), ty);
            ctx.emit_stmt(StatementKind::StorageLive(local), span);
            ctx.emit_assign(Place::from_local(local), Rvalue::Use(value), span);
            Ok(())
        }

        PatternKind::Literal(_lit) => {
            // Literal patterns don't bind, they match
            // In a binding context, this is an error
            Err(MirError::InvalidPattern { span: pattern.span })
        }

        PatternKind::Tuple(patterns) => {
            // Tuple destructuring
            // Store value in temp, then extract each element
            let temp = ctx.new_temp(ty.clone());
            ctx.emit_assign(Place::from_local(temp), Rvalue::Use(value), span);

            for (i, pat) in patterns.iter().enumerate() {
                let elem_ty = if let MirType::Tuple(tys) = &ty {
                    tys.get(i).cloned().unwrap_or(MirType::Unit)
                } else {
                    MirType::Unit
                };

                let elem_place = Place::from_local(temp).field(i as u32);
                // Use Copy or Move based on whether element type is Copy
                let elem_value = operand_for_place(elem_place, &elem_ty);
                lower_pattern_binding(ctx, pat, elem_value, elem_ty, pat.span)?;
            }
            Ok(())
        }

        PatternKind::Array { elements, rest } => {
            // Array destructuring
            let temp = ctx.new_temp(ty.clone());
            ctx.emit_assign(Place::from_local(temp), Rvalue::Use(value), span);

            let elem_ty = if let MirType::Array(inner) = &ty {
                inner.as_ref().clone()
            } else {
                MirType::Unit
            };

            for (i, pat) in elements.iter().enumerate() {
                // Create index local
                let idx_local = ctx.new_temp(MirType::Int);
                ctx.emit_assign(
                    Place::from_local(idx_local),
                    Rvalue::Use(Operand::const_int(i as i64)),
                    pat.span,
                );

                let elem_place = Place::from_local(temp).index(idx_local);
                // Use Copy or Move based on whether element type is Copy
                let elem_value = operand_for_place(elem_place, &elem_ty);
                lower_pattern_binding(ctx, pat, elem_value, elem_ty.clone(), pat.span)?;
            }

            // Handle rest pattern if present
            if let Some(rest_pat) = rest {
                // Rest captures remaining elements as a slice
                // For now, unsupported
                return Err(MirError::UnsupportedFeature {
                    feature: "rest patterns in arrays".to_string(),
                    span: rest_pat.span,
                });
            }

            Ok(())
        }

        PatternKind::Struct { name, fields } => {
            // Struct destructuring
            // Try to get struct ID from pattern type annotation or from passed type
            let struct_id = if let Some(type_name) = name {
                ctx.ctx.lookup_struct(&type_name.node)
            } else if let MirType::Struct(id) = &ty {
                Some(*id)
            } else {
                None
            };

            let temp = ctx.new_temp(ty.clone());
            ctx.emit_assign(Place::from_local(temp), Rvalue::Use(value), span);

            for field_pat in fields.iter() {
                // Look up field index and type
                let (field_idx, field_ty) = if let Some(sid) = struct_id {
                    ctx.ctx.lookup_struct_field(sid, &field_pat.name.node)
                        .unwrap_or((0, MirType::Unit))
                } else {
                    (0, MirType::Unit)
                };

                let field_place = Place::from_local(temp).field(field_idx);
                // Use Copy or Move based on whether field type is Copy
                let field_value = operand_for_place(field_place, &field_ty);

                if let Some(inner_pat) = &field_pat.pattern {
                    lower_pattern_binding(ctx, inner_pat, field_value, field_ty, field_pat.span)?;
                } else {
                    // Shorthand: `{x}` means `{x: x}`
                    let local = ctx.new_named_local(field_pat.name.node.clone(), field_ty);
                    ctx.emit_stmt(StatementKind::StorageLive(local), field_pat.span);
                    ctx.emit_assign(Place::from_local(local), Rvalue::Use(field_value), field_pat.span);
                }
            }

            Ok(())
        }

        PatternKind::Variant { path, variant, fields } => {
            // Enum variant destructuring
            // First, store the value and check discriminant
            let temp = ctx.new_temp(ty.clone());
            ctx.emit_assign(Place::from_local(temp), Rvalue::Use(value), span);

            // Get enum ID from the type or path
            let enum_id = if let MirType::Enum(id) = &ty {
                Some(*id)
            } else if let Some(first) = path.first() {
                ctx.ctx.lookup_enum(&first.node)
            } else {
                None
            };

            // Look up variant index
            let variant_idx = enum_id
                .and_then(|eid| ctx.ctx.lookup_enum_variant(eid, &variant.node))
                .unwrap_or(0);

            // Downcast and extract fields
            if let Some(field_pats) = fields {
                for (i, field_pat) in field_pats.iter().enumerate() {
                    // Look up field type from enum variant definition
                    let field_ty = enum_id
                        .and_then(|eid| ctx.ctx.get_enum_variant_field_type(eid, variant_idx, i as u32))
                        .unwrap_or(MirType::Unit);

                    let field_place = Place::from_local(temp)
                        .field(variant_idx) // Downcast projection
                        .field(i as u32);
                    // Use Copy or Move based on whether field type is Copy
                    let field_value = operand_for_place(field_place, &field_ty);
                    lower_pattern_binding(ctx, field_pat, field_value, field_ty, field_pat.span)?;
                }
            }

            Ok(())
        }

        PatternKind::Range { .. } => {
            // Range patterns don't bind, they match
            Err(MirError::InvalidPattern { span: pattern.span })
        }

        PatternKind::Or(patterns) => {
            // Or patterns in binding context: all branches must bind same names
            // For now, just use the first pattern
            if let Some(first) = patterns.first() {
                lower_pattern_binding(ctx, first, value, ty, span)
            } else {
                Ok(())
            }
        }

        PatternKind::Guard { pattern, condition } => {
            // Guard patterns need runtime check
            // Lower the inner pattern, then check the condition
            lower_pattern_binding(ctx, pattern, value, ty, span)?;

            // Evaluate guard condition
            let cond = ctx.lower_expr(condition)?;

            // Assert the condition holds
            let next_block = ctx.func.new_block();
            ctx.emit_terminator(
                TerminatorKind::Assert {
                    cond,
                    expected: true,
                    msg: "pattern guard failed".into(),
                    target: next_block,
                },
                condition.span,
            );
            ctx.current_block = next_block;

            Ok(())
        }

        PatternKind::Binding { name, pattern: inner } => {
            // `name @ pattern` binds name to the whole value and destructures with pattern
            // First, bind the name
            let local = ctx.new_named_local(name.node.clone(), ty.clone());
            ctx.emit_stmt(StatementKind::StorageLive(local), name.span);
            ctx.emit_assign(Place::from_local(local), Rvalue::Use(value.clone()), span);

            // Then, lower the inner pattern
            lower_pattern_binding(ctx, inner, value, ty, span)
        }

        PatternKind::Typed { pattern: inner, ty: type_annot } => {
            // Type-annotated pattern - use the annotation for type
            let annotated_ty = ctx.ctx.lower_type(type_annot);
            lower_pattern_binding(ctx, inner, value, annotated_ty, span)
        }

        PatternKind::Rest(_name) => {
            // Rest pattern in isolation is invalid
            Err(MirError::InvalidPattern { span: pattern.span })
        }
    }
}

/// Lower a pattern for matching (returns whether it matched)
/// Used in match expressions
#[allow(dead_code)]
pub fn lower_pattern_match(
    ctx: &mut FunctionLoweringContext,
    pattern: &ast::Pattern,
    scrutinee: Place,
    match_block: BlockId,
    no_match_block: BlockId,
    span: Span,
) -> Result<()> {
    match &pattern.kind {
        PatternKind::Wildcard => {
            // Wildcard always matches
            ctx.emit_terminator(TerminatorKind::Goto { target: match_block }, span);
            Ok(())
        }

        PatternKind::Ident(name) => {
            // Identifier always matches and binds
            let ty = ctx.func.local_decl(scrutinee.local).ty.clone();
            let local = ctx.new_named_local(name.clone(), ty.clone());
            ctx.emit_stmt(StatementKind::StorageLive(local), span);
            // Use Copy or Move based on whether scrutinee type is Copy
            let scrutinee_value = operand_for_place(scrutinee, &ty);
            ctx.emit_assign(
                Place::from_local(local),
                Rvalue::Use(scrutinee_value),
                span,
            );
            ctx.emit_terminator(TerminatorKind::Goto { target: match_block }, span);
            Ok(())
        }

        PatternKind::Literal(lit_expr) => {
            // Compare scrutinee with literal
            let lit_val = ctx.lower_expr(lit_expr)?;
            let ty = ctx.func.local_decl(scrutinee.local).ty.clone();
            // Use Copy or Move based on whether scrutinee type is Copy
            let scrutinee_val = operand_for_place(scrutinee, &ty);

            // Create comparison
            let cmp_result = ctx.new_temp(MirType::Bool);
            ctx.emit_assign(
                Place::from_local(cmp_result),
                Rvalue::BinaryOp(BinOp::Eq, scrutinee_val, lit_val),
                span,
            );

            ctx.emit_terminator(
                TerminatorKind::SwitchInt {
                    // Bool is always Copy
                    discr: Operand::Copy(Place::from_local(cmp_result)),
                    targets: SwitchTargets::if_else(match_block, no_match_block),
                },
                span,
            );
            Ok(())
        }

        PatternKind::Tuple(patterns) => {
            // Check each element with nested matching
            if patterns.is_empty() {
                // Empty tuple always matches
                ctx.emit_terminator(TerminatorKind::Goto { target: match_block }, span);
                return Ok(());
            }

            // Create a chain of blocks: each pattern check leads to the next
            // Pattern 0 -> Pattern 1 -> ... -> Pattern N -> match_block
            // Any failure -> no_match_block

            let n = patterns.len();
            let mut check_blocks: Vec<BlockId> = Vec::with_capacity(n);

            // Create blocks for each pattern check
            for _ in 0..n {
                check_blocks.push(ctx.func.new_block());
            }

            // First, go to the first check block
            ctx.emit_terminator(TerminatorKind::Goto { target: check_blocks[0] }, span);

            // For each pattern, set up the check
            for (i, pat) in patterns.iter().enumerate() {
                ctx.current_block = check_blocks[i];
                let elem_place = scrutinee.clone().field(i as u32);

                // If this is the last pattern, success goes to match_block
                // Otherwise, success goes to the next check
                let success_target = if i + 1 < n {
                    check_blocks[i + 1]
                } else {
                    match_block
                };

                lower_pattern_match(ctx, pat, elem_place, success_target, no_match_block, pat.span)?;
            }

            Ok(())
        }

        PatternKind::Variant { path, variant, fields } => {
            // Check discriminant
            let discr = ctx.new_temp(MirType::Int);
            ctx.emit_assign(
                Place::from_local(discr),
                Rvalue::Discriminant(scrutinee.clone()),
                span,
            );

            // Infer enum ID from scrutinee type or path
            let scrutinee_ty = ctx.func.local_decl(scrutinee.local).ty.clone();
            let enum_id = if let MirType::Enum(id) = scrutinee_ty {
                Some(id)
            } else if let Some(first) = path.first() {
                ctx.ctx.lookup_enum(&first.node)
            } else {
                None
            };

            // Look up variant index
            let variant_idx_u32 = enum_id
                .and_then(|eid| ctx.ctx.lookup_enum_variant(eid, &variant.node))
                .unwrap_or(0);
            let variant_idx: i128 = variant_idx_u32 as i128;

            // Create blocks for match and no-match
            let variant_match_block = ctx.func.new_block();

            ctx.emit_terminator(
                TerminatorKind::SwitchInt {
                    discr: Operand::Copy(Place::from_local(discr)),
                    targets: SwitchTargets::new(
                        vec![(variant_idx, variant_match_block)],
                        no_match_block,
                    ),
                },
                span,
            );

            // In variant match block, bind fields if any
            ctx.current_block = variant_match_block;

            if let Some(field_patterns) = fields {
                // Bind each field pattern
                for (i, field_pat) in field_patterns.iter().enumerate() {
                    // Get field type
                    let field_ty = enum_id
                        .and_then(|eid| ctx.ctx.get_enum_variant_field_type(eid, variant_idx_u32, i as u32))
                        .unwrap_or(MirType::Unit);

                    // Create place for the field
                    let field_place = scrutinee.clone()
                        .downcast(variant_idx_u32)
                        .field(i as u32);

                    // Bind the field to a local
                    if let PatternKind::Ident(name) = &field_pat.kind {
                        let local = ctx.new_named_local(name.clone(), field_ty.clone());
                        ctx.emit_stmt(StatementKind::StorageLive(local), field_pat.span);
                        // Use Copy or Move based on whether field type is Copy
                        let field_value = operand_for_place(field_place, &field_ty);
                        ctx.emit_assign(
                            Place::from_local(local),
                            Rvalue::Use(field_value),
                            field_pat.span,
                        );
                    }
                    // For non-identifier patterns, we would need nested matching
                    // which is more complex - for now just bind identifiers
                }
            }

            ctx.emit_terminator(TerminatorKind::Goto { target: match_block }, span);

            Ok(())
        }

        PatternKind::Range { start, end, inclusive } => {
            // Check if scrutinee is in range
            let ty = ctx.func.local_decl(scrutinee.local).ty.clone();
            // Use Copy or Move based on whether scrutinee type is Copy
            let scrutinee_val = operand_for_place(scrutinee, &ty);
            let start_val = ctx.lower_expr(start)?;
            let end_val = ctx.lower_expr(end)?;

            // Check start <= scrutinee
            let ge_start = ctx.new_temp(MirType::Bool);
            ctx.emit_assign(
                Place::from_local(ge_start),
                Rvalue::BinaryOp(BinOp::Ge, scrutinee_val.clone(), start_val),
                span,
            );

            // Check scrutinee <= end (or < if exclusive)
            let le_end = ctx.new_temp(MirType::Bool);
            let end_op = if *inclusive { BinOp::Le } else { BinOp::Lt };
            ctx.emit_assign(
                Place::from_local(le_end),
                Rvalue::BinaryOp(end_op, scrutinee_val, end_val),
                span,
            );

            // Combine: in_range = ge_start && le_end
            let in_range = ctx.new_temp(MirType::Bool);
            ctx.emit_assign(
                Place::from_local(in_range),
                Rvalue::BinaryOp(
                    BinOp::And,
                    Operand::Copy(Place::from_local(ge_start)),
                    Operand::Copy(Place::from_local(le_end)),
                ),
                span,
            );

            ctx.emit_terminator(
                TerminatorKind::SwitchInt {
                    discr: Operand::Copy(Place::from_local(in_range)),
                    targets: SwitchTargets::if_else(match_block, no_match_block),
                },
                span,
            );

            Ok(())
        }

        PatternKind::Or(patterns) => {
            // Try each pattern in sequence
            if patterns.is_empty() {
                ctx.emit_terminator(TerminatorKind::Goto { target: no_match_block }, span);
                return Ok(());
            }

            let try_blocks: Vec<BlockId> = patterns.iter().map(|_| ctx.func.new_block()).collect();

            // Start with first pattern
            ctx.emit_terminator(TerminatorKind::Goto { target: try_blocks[0] }, span);

            for (i, pat) in patterns.iter().enumerate() {
                ctx.current_block = try_blocks[i];
                let next_try = if i + 1 < patterns.len() {
                    try_blocks[i + 1]
                } else {
                    no_match_block
                };
                lower_pattern_match(ctx, pat, scrutinee.clone(), match_block, next_try, pat.span)?;
            }

            Ok(())
        }

        PatternKind::Guard { pattern: inner, condition } => {
            // First check the pattern, then the guard
            let guard_check_block = ctx.func.new_block();
            lower_pattern_match(ctx, inner, scrutinee, guard_check_block, no_match_block, span)?;

            // Guard check
            ctx.current_block = guard_check_block;
            let cond = ctx.lower_expr(condition)?;
            ctx.emit_terminator(
                TerminatorKind::SwitchInt {
                    discr: cond,
                    targets: SwitchTargets::if_else(match_block, no_match_block),
                },
                condition.span,
            );

            Ok(())
        }

        _ => {
            // Default: just match
            ctx.emit_terminator(TerminatorKind::Goto { target: match_block }, span);
            Ok(())
        }
    }
}
