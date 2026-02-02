//! MIR optimization passes.
//!
//! This module implements optimization passes that transform MIR to
//! improve performance of the generated code.
//!
//! # Passes
//!
//! - **Constant folding**: Evaluate constant expressions at compile time
//! - **Dead code elimination**: Remove unreachable blocks and unused assignments
//! - **Copy propagation**: Replace uses of copied values with the original
//! - **Simplify CFG**: Merge trivial blocks and eliminate redundant jumps

use crate::mir::{
    BinOp, BlockId, Constant, Local, MirFunction, MirProgram,
    Operand, Rvalue, StatementKind, Terminator, TerminatorKind,
    UnOp,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Apply all optimization passes to a MIR program.
pub fn optimize_program(program: &mut MirProgram, level: OptLevel) {
    for func in program.functions.values_mut() {
        optimize_function(func, level);
    }
}

/// Apply optimization passes to a single function.
pub fn optimize_function(func: &mut MirFunction, level: OptLevel) {
    match level {
        OptLevel::None => {}
        OptLevel::Basic => {
            constant_fold(func);
            algebraic_simplify(func);
            dead_code_elimination(func);
            simplify_cfg(func);
        }
        OptLevel::Aggressive => {
            // Run passes multiple times for fixed-point
            for _ in 0..3 {
                constant_fold(func);
                algebraic_simplify(func);
                copy_propagation(func);
                dead_code_elimination(func);
                simplify_cfg(func);
            }
        }
    }
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptLevel {
    #[default]
    None,
    Basic,
    Aggressive,
}

// ============================================================================
// Constant Folding
// ============================================================================

/// Fold constant expressions at compile time.
///
/// Examples:
/// - `1 + 2` → `3`
/// - `true && false` → `false`
/// - `-5` → `-5` (already constant)
pub fn constant_fold(func: &mut MirFunction) {
    for block in &mut func.blocks {
        for stmt in &mut block.statements {
            if let StatementKind::Assign(_, ref mut rvalue) = stmt.kind {
                if let Some(folded) = try_fold_rvalue(rvalue) {
                    *rvalue = Rvalue::Use(folded);
                }
            }
        }
    }
}

/// Try to fold an rvalue to a constant.
fn try_fold_rvalue(rvalue: &Rvalue) -> Option<Operand> {
    match rvalue {
        Rvalue::BinaryOp(op, left, right) => {
            let left_const = extract_constant(left)?;
            let right_const = extract_constant(right)?;
            fold_binary_op(*op, &left_const, &right_const)
        }
        Rvalue::UnaryOp(op, operand) => {
            let const_val = extract_constant(operand)?;
            fold_unary_op(*op, &const_val)
        }
        _ => None,
    }
}

/// Extract a constant from an operand.
fn extract_constant(operand: &Operand) -> Option<Constant> {
    match operand {
        Operand::Constant(c) => Some(c.clone()),
        _ => None,
    }
}

/// Fold a binary operation on constants.
fn fold_binary_op(op: BinOp, left: &Constant, right: &Constant) -> Option<Operand> {
    match (op, left, right) {
        // Integer arithmetic
        (BinOp::Add, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Int(a.wrapping_add(*b))))
        }
        (BinOp::Sub, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Int(a.wrapping_sub(*b))))
        }
        (BinOp::Mul, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Int(a.wrapping_mul(*b))))
        }
        (BinOp::Div, Constant::Int(a), Constant::Int(b)) if *b != 0 => {
            Some(Operand::Constant(Constant::Int(a / b)))
        }
        (BinOp::Rem, Constant::Int(a), Constant::Int(b)) if *b != 0 => {
            Some(Operand::Constant(Constant::Int(a % b)))
        }

        // Float arithmetic
        (BinOp::Add, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Float(a + b)))
        }
        (BinOp::Sub, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Float(a - b)))
        }
        (BinOp::Mul, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Float(a * b)))
        }
        (BinOp::Div, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Float(a / b)))
        }

        // Integer comparisons
        (BinOp::Eq, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Bool(a == b)))
        }
        (BinOp::Ne, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Bool(a != b)))
        }
        (BinOp::Lt, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Bool(a < b)))
        }
        (BinOp::Le, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Bool(a <= b)))
        }
        (BinOp::Gt, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Bool(a > b)))
        }
        (BinOp::Ge, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Bool(a >= b)))
        }

        // Boolean operations
        (BinOp::And, Constant::Bool(a), Constant::Bool(b)) => {
            Some(Operand::Constant(Constant::Bool(*a && *b)))
        }
        (BinOp::Or, Constant::Bool(a), Constant::Bool(b)) => {
            Some(Operand::Constant(Constant::Bool(*a || *b)))
        }

        // Bitwise operations
        (BinOp::BitAnd, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Int(a & b)))
        }
        (BinOp::BitOr, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Int(a | b)))
        }
        (BinOp::BitXor, Constant::Int(a), Constant::Int(b)) => {
            Some(Operand::Constant(Constant::Int(a ^ b)))
        }

        // Shift operations
        (BinOp::Shl, Constant::Int(a), Constant::Int(b)) if *b >= 0 && *b < 64 => {
            Some(Operand::Constant(Constant::Int(a.wrapping_shl(*b as u32))))
        }
        (BinOp::Shr, Constant::Int(a), Constant::Int(b)) if *b >= 0 && *b < 64 => {
            Some(Operand::Constant(Constant::Int(a.wrapping_shr(*b as u32))))
        }

        // Power operation (integer)
        (BinOp::Pow, Constant::Int(a), Constant::Int(b)) if *b >= 0 => {
            // Only fold small positive exponents to avoid overflow
            if *b <= 20 {
                Some(Operand::Constant(Constant::Int(a.wrapping_pow(*b as u32))))
            } else {
                None
            }
        }

        // Float comparisons
        (BinOp::Eq, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Bool(a == b)))
        }
        (BinOp::Ne, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Bool(a != b)))
        }
        (BinOp::Lt, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Bool(a < b)))
        }
        (BinOp::Le, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Bool(a <= b)))
        }
        (BinOp::Gt, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Bool(a > b)))
        }
        (BinOp::Ge, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Bool(a >= b)))
        }

        // Power operation (float)
        (BinOp::Pow, Constant::Float(a), Constant::Float(b)) => {
            Some(Operand::Constant(Constant::Float(a.powf(*b))))
        }

        // Bool equality
        (BinOp::Eq, Constant::Bool(a), Constant::Bool(b)) => {
            Some(Operand::Constant(Constant::Bool(a == b)))
        }
        (BinOp::Ne, Constant::Bool(a), Constant::Bool(b)) => {
            Some(Operand::Constant(Constant::Bool(a != b)))
        }

        _ => None,
    }
}

/// Fold a unary operation on a constant.
fn fold_unary_op(op: UnOp, operand: &Constant) -> Option<Operand> {
    match (op, operand) {
        (UnOp::Neg, Constant::Int(n)) => Some(Operand::Constant(Constant::Int(-n))),
        (UnOp::Neg, Constant::Float(f)) => Some(Operand::Constant(Constant::Float(-f))),
        (UnOp::Not, Constant::Bool(b)) => Some(Operand::Constant(Constant::Bool(!b))),
        (UnOp::Not, Constant::Int(n)) => Some(Operand::Constant(Constant::Int(!n))),
        _ => None,
    }
}

// ============================================================================
// Algebraic Simplification
// ============================================================================

/// Simplify expressions using algebraic identities.
///
/// Examples:
/// - `x + 0` → `x`
/// - `x * 1` → `x`
/// - `x * 0` → `0`
/// - `x - 0` → `x`
/// - `x / 1` → `x`
/// - `x && true` → `x`
/// - `x || false` → `x`
/// - `x && false` → `false`
/// - `x || true` → `true`
/// - `x - x` → `0` (when same local)
/// - `x ^ x` → `0` (when same local)
pub fn algebraic_simplify(func: &mut MirFunction) {
    for block in &mut func.blocks {
        for stmt in &mut block.statements {
            if let StatementKind::Assign(_, ref mut rvalue) = stmt.kind {
                if let Some(simplified) = try_algebraic_simplify(rvalue) {
                    *rvalue = simplified;
                }
            }
        }
    }
}

/// Try to simplify an rvalue using algebraic identities.
fn try_algebraic_simplify(rvalue: &Rvalue) -> Option<Rvalue> {
    match rvalue {
        Rvalue::BinaryOp(op, left, right) => {
            simplify_binary_op(*op, left, right)
        }
        Rvalue::UnaryOp(op, operand) => {
            simplify_unary_op(*op, operand)
        }
        _ => None,
    }
}

/// Simplify binary operations using algebraic identities.
fn simplify_binary_op(op: BinOp, left: &Operand, right: &Operand) -> Option<Rvalue> {
    // Check for identity elements on the right
    if let Operand::Constant(c) = right {
        match (op, c) {
            // x + 0 → x
            (BinOp::Add, Constant::Int(0)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x - 0 → x
            (BinOp::Sub, Constant::Int(0)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x * 1 → x
            (BinOp::Mul, Constant::Int(1)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x * 0 → 0
            (BinOp::Mul, Constant::Int(0)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Int(0))));
            }
            // x / 1 → x
            (BinOp::Div, Constant::Int(1)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x // 1 → x (integer division)
            (BinOp::IntDiv, Constant::Int(1)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x ** 0 → 1
            (BinOp::Pow, Constant::Int(0)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Int(1))));
            }
            // x ** 1 → x
            (BinOp::Pow, Constant::Int(1)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x | 0 → x
            (BinOp::BitOr, Constant::Int(0)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x & -1 → x (all bits set)
            (BinOp::BitAnd, Constant::Int(-1)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x ^ 0 → x
            (BinOp::BitXor, Constant::Int(0)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x << 0 → x
            (BinOp::Shl, Constant::Int(0)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x >> 0 → x
            (BinOp::Shr, Constant::Int(0)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x && true → x
            (BinOp::And, Constant::Bool(true)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x && false → false
            (BinOp::And, Constant::Bool(false)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))));
            }
            // x || false → x
            (BinOp::Or, Constant::Bool(false)) => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x || true → true
            (BinOp::Or, Constant::Bool(true)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))));
            }
            // Float identities
            // x + 0.0 → x
            (BinOp::Add, Constant::Float(f)) if *f == 0.0 => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x - 0.0 → x
            (BinOp::Sub, Constant::Float(f)) if *f == 0.0 => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x * 1.0 → x
            (BinOp::Mul, Constant::Float(f)) if *f == 1.0 => {
                return Some(Rvalue::Use(left.clone()));
            }
            // x * 0.0 → 0.0
            (BinOp::Mul, Constant::Float(f)) if *f == 0.0 => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Float(0.0))));
            }
            // x / 1.0 → x
            (BinOp::Div, Constant::Float(f)) if *f == 1.0 => {
                return Some(Rvalue::Use(left.clone()));
            }
            _ => {}
        }
    }

    // Check for identity elements on the left
    if let Operand::Constant(c) = left {
        match (op, c) {
            // 0 + x → x
            (BinOp::Add, Constant::Int(0)) => {
                return Some(Rvalue::Use(right.clone()));
            }
            // 1 * x → x
            (BinOp::Mul, Constant::Int(1)) => {
                return Some(Rvalue::Use(right.clone()));
            }
            // 0 * x → 0
            (BinOp::Mul, Constant::Int(0)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Int(0))));
            }
            // 0 | x → x
            (BinOp::BitOr, Constant::Int(0)) => {
                return Some(Rvalue::Use(right.clone()));
            }
            // -1 & x → x
            (BinOp::BitAnd, Constant::Int(-1)) => {
                return Some(Rvalue::Use(right.clone()));
            }
            // 0 ^ x → x
            (BinOp::BitXor, Constant::Int(0)) => {
                return Some(Rvalue::Use(right.clone()));
            }
            // true && x → x
            (BinOp::And, Constant::Bool(true)) => {
                return Some(Rvalue::Use(right.clone()));
            }
            // false && x → false
            (BinOp::And, Constant::Bool(false)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))));
            }
            // false || x → x
            (BinOp::Or, Constant::Bool(false)) => {
                return Some(Rvalue::Use(right.clone()));
            }
            // true || x → true
            (BinOp::Or, Constant::Bool(true)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))));
            }
            // Float identities
            // 0.0 + x → x
            (BinOp::Add, Constant::Float(f)) if *f == 0.0 => {
                return Some(Rvalue::Use(right.clone()));
            }
            // 1.0 * x → x
            (BinOp::Mul, Constant::Float(f)) if *f == 1.0 => {
                return Some(Rvalue::Use(right.clone()));
            }
            // 0.0 * x → 0.0
            (BinOp::Mul, Constant::Float(f)) if *f == 0.0 => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Float(0.0))));
            }
            _ => {}
        }
    }

    // Check for same operand patterns (x op x)
    if let (Operand::Copy(p1) | Operand::Move(p1), Operand::Copy(p2) | Operand::Move(p2)) = (left, right) {
        // Only simplify if both refer to the same local with no projection
        if p1.local == p2.local && p1.projection.is_empty() && p2.projection.is_empty() {
            match op {
                // x - x → 0
                BinOp::Sub => {
                    return Some(Rvalue::Use(Operand::Constant(Constant::Int(0))));
                }
                // x ^ x → 0
                BinOp::BitXor => {
                    return Some(Rvalue::Use(Operand::Constant(Constant::Int(0))));
                }
                // x == x → true
                BinOp::Eq => {
                    return Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))));
                }
                // x != x → false
                BinOp::Ne => {
                    return Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))));
                }
                // x <= x → true
                BinOp::Le => {
                    return Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))));
                }
                // x >= x → true
                BinOp::Ge => {
                    return Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))));
                }
                // x < x → false
                BinOp::Lt => {
                    return Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))));
                }
                // x > x → false
                BinOp::Gt => {
                    return Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))));
                }
                // x & x → x
                BinOp::BitAnd => {
                    return Some(Rvalue::Use(left.clone()));
                }
                // x | x → x
                BinOp::BitOr => {
                    return Some(Rvalue::Use(left.clone()));
                }
                // x && x → x
                BinOp::And => {
                    return Some(Rvalue::Use(left.clone()));
                }
                // x || x → x
                BinOp::Or => {
                    return Some(Rvalue::Use(left.clone()));
                }
                _ => {}
            }
        }
    }

    None
}

/// Simplify unary operations.
fn simplify_unary_op(op: UnOp, operand: &Operand) -> Option<Rvalue> {
    // Double negation: --x → x
    // This would require looking through the operand to find another UnaryOp
    // For now, we don't have this info, so skip

    // !!x → x for boolean
    // Similar issue

    // !true → false, !false → true (handled by constant folding)

    // Simplify: -0 → 0 (constant folding already handles this)
    if let Operand::Constant(c) = operand {
        match (op, c) {
            // !true → false (redundant with constant folding but included for completeness)
            (UnOp::Not, Constant::Bool(true)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))));
            }
            // !false → true
            (UnOp::Not, Constant::Bool(false)) => {
                return Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))));
            }
            _ => {}
        }
    }

    None
}

// ============================================================================
// Dead Code Elimination
// ============================================================================

/// Remove unreachable blocks and unused assignments.
pub fn dead_code_elimination(func: &mut MirFunction) {
    // Find reachable blocks
    let reachable = find_reachable_blocks(func);

    // Mark unreachable blocks as empty
    for (i, block) in func.blocks.iter_mut().enumerate() {
        if !reachable.contains(&BlockId(i as u32)) {
            block.statements.clear();
            block.terminator = Some(Terminator {
                kind: TerminatorKind::Unreachable,
                span: aria_lexer::Span::dummy(),
            });
        }
    }

    // Find used locals
    let used = find_used_locals(func);

    // Remove assignments to unused locals (except return place)
    for block in &mut func.blocks {
        block.statements.retain(|stmt| {
            if let StatementKind::Assign(place, _) = &stmt.kind {
                // Keep if it's the return place or a used local
                place.local == Local::RETURN || used.contains(&place.local)
            } else {
                true
            }
        });
    }
}

/// Find all reachable blocks via BFS from entry.
fn find_reachable_blocks(func: &MirFunction) -> FxHashSet<BlockId> {
    let mut reachable = FxHashSet::default();
    let mut worklist = vec![BlockId::ENTRY];

    while let Some(block_id) = worklist.pop() {
        if reachable.contains(&block_id) {
            continue;
        }
        reachable.insert(block_id);

        if let Some(block) = func.blocks.get(block_id.0 as usize) {
            if let Some(ref term) = block.terminator {
                for succ in successors(term) {
                    worklist.push(succ);
                }
            }
        }
    }

    reachable
}

/// Get successor blocks of a terminator.
fn successors(term: &Terminator) -> Vec<BlockId> {
    match &term.kind {
        TerminatorKind::Goto { target } => vec![*target],
        TerminatorKind::SwitchInt { targets, .. } => {
            let mut succs: Vec<_> = targets.targets.iter().map(|(_, b)| *b).collect();
            succs.push(targets.otherwise);
            succs
        }
        TerminatorKind::Call { target, .. } => target.into_iter().copied().collect(),
        TerminatorKind::Return | TerminatorKind::Unreachable => vec![],
        TerminatorKind::Drop { target, .. } => vec![*target],
        TerminatorKind::Assert { target, .. } => vec![*target],
    }
}

/// Find all locals that are used (read from).
fn find_used_locals(func: &MirFunction) -> FxHashSet<Local> {
    let mut used = FxHashSet::default();

    for block in &func.blocks {
        for stmt in &block.statements {
            if let StatementKind::Assign(_, rvalue) = &stmt.kind {
                collect_used_in_rvalue(rvalue, &mut used);
            }
        }

        if let Some(ref term) = block.terminator {
            collect_used_in_terminator(term, &mut used);
        }
    }

    used
}

/// Collect locals used in an rvalue.
fn collect_used_in_rvalue(rvalue: &Rvalue, used: &mut FxHashSet<Local>) {
    match rvalue {
        Rvalue::Use(op) | Rvalue::UnaryOp(_, op) | Rvalue::Cast(_, op, _) => {
            collect_used_in_operand(op, used);
        }
        Rvalue::BinaryOp(_, left, right) => {
            collect_used_in_operand(left, used);
            collect_used_in_operand(right, used);
        }
        Rvalue::Ref(place) | Rvalue::RefMut(place) | Rvalue::Discriminant(place) | Rvalue::Len(place) => {
            used.insert(place.local);
        }
        Rvalue::Aggregate(_, operands) | Rvalue::Closure(_, operands) => {
            for op in operands {
                collect_used_in_operand(op, used);
            }
        }
    }
}

/// Collect locals used in an operand.
fn collect_used_in_operand(operand: &Operand, used: &mut FxHashSet<Local>) {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => {
            used.insert(place.local);
        }
        Operand::Constant(_) => {}
    }
}

/// Collect locals used in a terminator.
fn collect_used_in_terminator(term: &Terminator, used: &mut FxHashSet<Local>) {
    match &term.kind {
        TerminatorKind::SwitchInt { discr, .. } => {
            collect_used_in_operand(discr, used);
        }
        TerminatorKind::Call { func, args, .. } => {
            collect_used_in_operand(func, used);
            for arg in args {
                collect_used_in_operand(arg, used);
            }
        }
        TerminatorKind::Assert { cond, .. } => {
            collect_used_in_operand(cond, used);
        }
        TerminatorKind::Drop { place, .. } => {
            used.insert(place.local);
        }
        _ => {}
    }
}

// ============================================================================
// Copy Propagation
// ============================================================================

/// Replace uses of copied values with the original.
///
/// If we have `_2 = Copy(_1)` followed by uses of `_2`, replace
/// those uses with `_1` directly.
pub fn copy_propagation(func: &mut MirFunction) {
    // Build copy map: dest -> source
    let mut copies: FxHashMap<Local, Local> = FxHashMap::default();

    for block in &func.blocks {
        for stmt in &block.statements {
            if let StatementKind::Assign(place, Rvalue::Use(Operand::Copy(src))) = &stmt.kind {
                if place.projection.is_empty() && src.projection.is_empty() {
                    copies.insert(place.local, src.local);
                }
            }
        }
    }

    // Resolve transitive copies
    let resolved = resolve_copies(&copies);

    // Apply substitution
    for block in &mut func.blocks {
        for stmt in &mut block.statements {
            if let StatementKind::Assign(_, rvalue) = &mut stmt.kind {
                substitute_in_rvalue(rvalue, &resolved);
            }
        }

        if let Some(ref mut term) = block.terminator {
            substitute_in_terminator(term, &resolved);
        }
    }
}

/// Resolve transitive copies: if _2 = _1 and _3 = _2, then _3 -> _1
fn resolve_copies(copies: &FxHashMap<Local, Local>) -> FxHashMap<Local, Local> {
    let mut resolved = FxHashMap::default();

    for (&dest, &src) in copies {
        let mut current = src;
        let mut visited = FxHashSet::default();

        while let Some(&next) = copies.get(&current) {
            if visited.contains(&current) {
                break; // Cycle detected
            }
            visited.insert(current);
            current = next;
        }

        resolved.insert(dest, current);
    }

    resolved
}

/// Substitute locals in an rvalue.
fn substitute_in_rvalue(rvalue: &mut Rvalue, subs: &FxHashMap<Local, Local>) {
    match rvalue {
        Rvalue::Use(op) | Rvalue::UnaryOp(_, op) | Rvalue::Cast(_, op, _) => {
            substitute_in_operand(op, subs);
        }
        Rvalue::BinaryOp(_, left, right) => {
            substitute_in_operand(left, subs);
            substitute_in_operand(right, subs);
        }
        Rvalue::Ref(place) | Rvalue::RefMut(place) | Rvalue::Discriminant(place) | Rvalue::Len(place) => {
            if let Some(&new_local) = subs.get(&place.local) {
                place.local = new_local;
            }
        }
        Rvalue::Aggregate(_, operands) | Rvalue::Closure(_, operands) => {
            for op in operands {
                substitute_in_operand(op, subs);
            }
        }
    }
}

/// Substitute locals in an operand.
fn substitute_in_operand(operand: &mut Operand, subs: &FxHashMap<Local, Local>) {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => {
            if let Some(&new_local) = subs.get(&place.local) {
                place.local = new_local;
            }
        }
        Operand::Constant(_) => {}
    }
}

/// Substitute locals in a terminator.
fn substitute_in_terminator(term: &mut Terminator, subs: &FxHashMap<Local, Local>) {
    match &mut term.kind {
        TerminatorKind::SwitchInt { discr, .. } => {
            substitute_in_operand(discr, subs);
        }
        TerminatorKind::Call { func, args, .. } => {
            substitute_in_operand(func, subs);
            for arg in args {
                substitute_in_operand(arg, subs);
            }
        }
        TerminatorKind::Assert { cond, .. } => {
            substitute_in_operand(cond, subs);
        }
        TerminatorKind::Drop { place, .. } => {
            if let Some(&new_local) = subs.get(&place.local) {
                place.local = new_local;
            }
        }
        _ => {}
    }
}

// ============================================================================
// CFG Simplification
// ============================================================================

/// Simplify the control flow graph.
///
/// - Merge trivial blocks (single statement, unconditional jump)
/// - Eliminate redundant jumps
pub fn simplify_cfg(func: &mut MirFunction) {
    // Fold trivial switches on constants
    for block in &mut func.blocks {
        if let Some(ref mut term) = block.terminator {
            if let TerminatorKind::SwitchInt { discr, targets } = &term.kind {
                if let Operand::Constant(Constant::Int(val)) = discr {
                    // Find which target to use
                    let target = targets
                        .targets
                        .iter()
                        .find(|(v, _)| *v == *val as i128)
                        .map(|(_, b)| *b)
                        .unwrap_or(targets.otherwise);

                    term.kind = TerminatorKind::Goto { target };
                } else if let Operand::Constant(Constant::Bool(b)) = discr {
                    let val = if *b { 1 } else { 0 };
                    let target = targets
                        .targets
                        .iter()
                        .find(|(v, _)| *v == val)
                        .map(|(_, b)| *b)
                        .unwrap_or(targets.otherwise);

                    term.kind = TerminatorKind::Goto { target };
                }
            }
        }
    }
}

// ============================================================================
// Loop Optimizations
// ============================================================================

/// Identify and optimize loops in the function.
///
/// Optimizations include:
/// - Loop-invariant code motion (hoisting)
/// - Strength reduction
/// - Loop unrolling for small fixed iterations
pub fn optimize_loops(func: &mut MirFunction) {
    // Find natural loops
    let loops = find_loops(func);

    for loop_info in &loops {
        // Hoist loop-invariant code
        hoist_loop_invariants(func, loop_info);

        // Apply strength reduction
        strength_reduction(func, loop_info);

        // Try to unroll small loops
        try_unroll_loop(func, loop_info);
    }
}

/// Information about a natural loop
#[derive(Debug, Clone)]
struct LoopInfo {
    header: BlockId,
    back_edges: Vec<BlockId>,
    body: FxHashSet<BlockId>,
}

/// Find natural loops in the function using back-edge detection
fn find_loops(func: &MirFunction) -> Vec<LoopInfo> {
    let mut loops = Vec::new();

    // Find dominators (simplified - just use entry block as universal dominator for now)
    // A proper implementation would compute the full dominator tree

    // Find back edges (edges that go to a block that's "earlier" in some sense)
    for (idx, block) in func.blocks.iter().enumerate() {
        let block_id = BlockId(idx as u32);

        if let Some(ref term) = block.terminator {
            for succ in successors(term) {
                // Simple heuristic: if successor has lower ID, it's likely a loop header
                if succ.0 <= block_id.0 {
                    // Found a potential back edge
                    loops.push(LoopInfo {
                        header: succ,
                        back_edges: vec![block_id],
                        body: collect_loop_body(func, succ, block_id),
                    });
                }
            }
        }
    }

    loops
}

/// Collect all blocks in a loop body
fn collect_loop_body(_func: &MirFunction, header: BlockId, back_edge: BlockId) -> FxHashSet<BlockId> {
    let mut body = FxHashSet::default();
    body.insert(header);
    body.insert(back_edge);
    // TODO: Proper loop body collection via dataflow analysis
    body
}

/// Hoist loop-invariant code out of loops
fn hoist_loop_invariants(func: &mut MirFunction, loop_info: &LoopInfo) {
    // Find statements that don't depend on loop variables
    let mut invariant_stmts = Vec::new();

    for &block_id in &loop_info.body {
        if let Some(block) = func.blocks.get(block_id.0 as usize) {
            for (idx, stmt) in block.statements.iter().enumerate() {
                if is_loop_invariant(stmt, loop_info) {
                    invariant_stmts.push((block_id, idx));
                }
            }
        }
    }

    // TODO: Actually move invariant statements to preheader
    // For now, just identify them
    let _ = invariant_stmts;
}

/// Check if a statement is loop-invariant
fn is_loop_invariant(_stmt: &crate::mir::Statement, _loop_info: &LoopInfo) -> bool {
    // Simplified check - a proper implementation would track def-use chains
    false
}

/// Apply strength reduction to loop induction variables
fn strength_reduction(_func: &mut MirFunction, _loop_info: &LoopInfo) {
    // TODO: Replace expensive operations like multiplication with cheaper ones
    // Example: i * 8 in loop → i += 8
}

/// Try to unroll a loop if it's small and has a constant trip count
fn try_unroll_loop(_func: &mut MirFunction, _loop_info: &LoopInfo) {
    // TODO: Detect constant trip count and unroll small loops
}

// ============================================================================
// Bounds Check Elimination
// ============================================================================

/// Track array lengths and eliminate redundant bounds checks.
///
/// This pass uses SSA value analysis to prove that array accesses are
/// within bounds, eliminating unnecessary runtime checks.
pub fn eliminate_bounds_checks(func: &mut MirFunction) {
    // Build a map of array lengths
    let array_lengths = track_array_lengths(func);

    // Find bounds checks that are provably safe
    let safe_checks = find_safe_bounds_checks(func, &array_lengths);

    // Remove safe checks
    for &(block_id, stmt_idx) in &safe_checks {
        if let Some(block) = func.blocks.get_mut(block_id.0 as usize) {
            if stmt_idx < block.statements.len() {
                // Mark as Nop instead of removing to preserve indices
                block.statements[stmt_idx].kind = StatementKind::Nop;
            }
        }
    }
}

/// Track array lengths through the program
fn track_array_lengths(func: &MirFunction) -> FxHashMap<Local, Option<i64>> {
    let mut lengths = FxHashMap::default();

    for block in &func.blocks {
        for stmt in &block.statements {
            if let StatementKind::Assign(place, rvalue) = &stmt.kind {
                // Track array allocations and their lengths
                if let Rvalue::Aggregate(crate::mir::AggregateKind::Array(_), operands) = rvalue {
                    lengths.insert(place.local, Some(operands.len() as i64));
                }
            }
        }
    }

    lengths
}

/// Find bounds checks that can be proven safe
fn find_safe_bounds_checks(
    func: &MirFunction,
    array_lengths: &FxHashMap<Local, Option<i64>>,
) -> Vec<(BlockId, usize)> {
    let mut safe_checks = Vec::new();

    for (block_idx, block) in func.blocks.iter().enumerate() {
        // Look for assert statements that check bounds
        if let Some(ref term) = block.terminator {
            if let TerminatorKind::Assert { cond, .. } = &term.kind {
                // Check if this is a bounds check we can eliminate
                if is_safe_bounds_check(cond, array_lengths) {
                    // Mark the check as safe (would need to track to statement)
                    let _ = safe_checks; // TODO: Properly track and eliminate
                }
            }
        }

        // Also check for explicit bounds check patterns in statements
        for (stmt_idx, stmt) in block.statements.iter().enumerate() {
            if is_bounds_check_stmt(stmt, array_lengths) {
                safe_checks.push((BlockId(block_idx as u32), stmt_idx));
            }
        }
    }

    safe_checks
}

/// Check if a condition is a bounds check that's provably safe
fn is_safe_bounds_check(_cond: &Operand, _array_lengths: &FxHashMap<Local, Option<i64>>) -> bool {
    // TODO: Implement actual bounds check analysis
    // Example: if we know i < arr.len() and we're checking arr[i], it's safe
    false
}

/// Check if a statement is a bounds check
fn is_bounds_check_stmt(_stmt: &crate::mir::Statement, _array_lengths: &FxHashMap<Local, Option<i64>>) -> bool {
    // TODO: Identify bounds check patterns
    false
}

// ============================================================================
// String Optimization
// ============================================================================

/// Optimize string operations.
///
/// - Detect string concatenation chains and use a single allocation
/// - Optimize string slicing to avoid unnecessary copies
pub fn optimize_strings(func: &mut MirFunction) {
    // Find string concatenation chains
    let concat_chains = find_string_concat_chains(func);

    // Optimize each chain
    for chain in &concat_chains {
        optimize_concat_chain(func, chain);
    }
}

/// A chain of string concatenations
#[derive(Debug)]
struct ConcatChain {
    result: Local,
    parts: Vec<Operand>,
    start_block: BlockId,
    start_stmt: usize,
}

/// Find chains of string concatenations
fn find_string_concat_chains(_func: &MirFunction) -> Vec<ConcatChain> {
    let mut chains = Vec::new();

    // TODO: Detect patterns like:
    // s1 = a + b
    // s2 = s1 + c
    // s3 = s2 + d
    // And transform to single allocation: s3 = concat(a, b, c, d)

    chains
}

/// Optimize a concatenation chain to use a single allocation
fn optimize_concat_chain(_func: &mut MirFunction, _chain: &ConcatChain) {
    // TODO: Replace chain with single concat operation
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{MirType, Place, Statement};
    use aria_lexer::Span;

    fn make_test_function() -> MirFunction {
        MirFunction::new("test".into(), MirType::Int, Span::dummy())
    }

    #[test]
    fn test_constant_fold_add() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Add,
            Operand::Constant(Constant::Int(1)),
            Operand::Constant(Constant::Int(2)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Int(3)))
        ));
    }

    #[test]
    fn test_constant_fold_mul() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Mul,
            Operand::Constant(Constant::Int(6)),
            Operand::Constant(Constant::Int(7)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Int(42)))
        ));
    }

    #[test]
    fn test_constant_fold_comparison() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Lt,
            Operand::Constant(Constant::Int(5)),
            Operand::Constant(Constant::Int(10)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Bool(true)))
        ));
    }

    #[test]
    fn test_constant_fold_bool_and() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::And,
            Operand::Constant(Constant::Bool(true)),
            Operand::Constant(Constant::Bool(false)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Bool(false)))
        ));
    }

    #[test]
    fn test_constant_fold_neg() {
        let rvalue = Rvalue::UnaryOp(UnOp::Neg, Operand::Constant(Constant::Int(42)));

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Int(-42)))
        ));
    }

    #[test]
    fn test_constant_fold_not() {
        let rvalue = Rvalue::UnaryOp(UnOp::Not, Operand::Constant(Constant::Bool(true)));

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Bool(false)))
        ));
    }

    #[test]
    fn test_find_reachable_blocks() {
        let mut func = make_test_function();
        let b0 = func.new_block();
        let b1 = func.new_block();
        let _b2 = func.new_block(); // Unreachable

        func.block_mut(b0).set_terminator(Terminator {
            kind: TerminatorKind::Goto { target: b1 },
            span: Span::dummy(),
        });
        func.block_mut(b1).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        let reachable = find_reachable_blocks(&func);
        assert!(reachable.contains(&BlockId(0)));
        assert!(reachable.contains(&BlockId(1)));
        assert!(!reachable.contains(&BlockId(2)));
    }

    #[test]
    fn test_resolve_transitive_copies() {
        let mut copies = FxHashMap::default();
        copies.insert(Local(2), Local(1));
        copies.insert(Local(3), Local(2));
        copies.insert(Local(4), Local(3));

        let resolved = resolve_copies(&copies);

        assert_eq!(resolved.get(&Local(2)), Some(&Local(1)));
        assert_eq!(resolved.get(&Local(3)), Some(&Local(1)));
        assert_eq!(resolved.get(&Local(4)), Some(&Local(1)));
    }

    #[test]
    fn test_copy_propagation() {
        let mut func = make_test_function();
        let _x = func.new_local(MirType::Int, Some("x".into())); // Local(1)
        let y = func.new_local(MirType::Int, Some("y".into())); // Local(2)
        let z = func.new_local(MirType::Int, Some("z".into())); // Local(3)

        let b0 = func.new_block();

        // _2 = Copy(_1)
        func.block_mut(b0).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(y),
                Rvalue::Use(Operand::Copy(Place::from_local(Local(1)))),
            ),
            span: Span::dummy(),
        });

        // _3 = Copy(_2) (should become Copy(_1) after propagation)
        func.block_mut(b0).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(z),
                Rvalue::Use(Operand::Copy(Place::from_local(y))),
            ),
            span: Span::dummy(),
        });

        func.block_mut(b0).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        copy_propagation(&mut func);

        // Check that _3 = Copy(_1) now
        if let StatementKind::Assign(_, Rvalue::Use(Operand::Copy(place))) =
            &func.blocks[0].statements[1].kind
        {
            assert_eq!(place.local, Local(1));
        } else {
            panic!("Expected Copy assignment");
        }
    }

    // ========================================================================
    // Extended Constant Folding Tests
    // ========================================================================

    #[test]
    fn test_constant_fold_shift_left() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Shl,
            Operand::Constant(Constant::Int(1)),
            Operand::Constant(Constant::Int(4)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Int(16)))
        ));
    }

    #[test]
    fn test_constant_fold_shift_right() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Shr,
            Operand::Constant(Constant::Int(32)),
            Operand::Constant(Constant::Int(2)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Int(8)))
        ));
    }

    #[test]
    fn test_constant_fold_power_int() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Pow,
            Operand::Constant(Constant::Int(2)),
            Operand::Constant(Constant::Int(10)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Int(1024)))
        ));
    }

    #[test]
    fn test_constant_fold_power_float() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Pow,
            Operand::Constant(Constant::Float(2.0)),
            Operand::Constant(Constant::Float(3.0)),
        );

        let folded = try_fold_rvalue(&rvalue);
        if let Some(Operand::Constant(Constant::Float(f))) = folded {
            assert!((f - 8.0).abs() < 0.001);
        } else {
            panic!("Expected Float constant");
        }
    }

    #[test]
    fn test_constant_fold_float_comparison() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Lt,
            Operand::Constant(Constant::Float(1.5)),
            Operand::Constant(Constant::Float(2.5)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Bool(true)))
        ));
    }

    #[test]
    fn test_constant_fold_bool_eq() {
        let rvalue = Rvalue::BinaryOp(
            BinOp::Eq,
            Operand::Constant(Constant::Bool(true)),
            Operand::Constant(Constant::Bool(true)),
        );

        let folded = try_fold_rvalue(&rvalue);
        assert!(matches!(
            folded,
            Some(Operand::Constant(Constant::Bool(true)))
        ));
    }

    // ========================================================================
    // Algebraic Simplification Tests
    // ========================================================================

    #[test]
    fn test_algebraic_add_zero_right() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Add,
            x.clone(),
            Operand::Constant(Constant::Int(0)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_add_zero_left() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Add,
            Operand::Constant(Constant::Int(0)),
            x.clone(),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_mul_one() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Mul,
            x.clone(),
            Operand::Constant(Constant::Int(1)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_mul_zero() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Mul,
            x.clone(),
            Operand::Constant(Constant::Int(0)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Int(0))))
        ));
    }

    #[test]
    fn test_algebraic_div_one() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Div,
            x.clone(),
            Operand::Constant(Constant::Int(1)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_pow_zero() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Pow,
            x.clone(),
            Operand::Constant(Constant::Int(0)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Int(1))))
        ));
    }

    #[test]
    fn test_algebraic_pow_one() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Pow,
            x.clone(),
            Operand::Constant(Constant::Int(1)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_and_true() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::And,
            x.clone(),
            Operand::Constant(Constant::Bool(true)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_and_false() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::And,
            x.clone(),
            Operand::Constant(Constant::Bool(false)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))))
        ));
    }

    #[test]
    fn test_algebraic_or_false() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Or,
            x.clone(),
            Operand::Constant(Constant::Bool(false)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_or_true() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Or,
            x.clone(),
            Operand::Constant(Constant::Bool(true)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))))
        ));
    }

    #[test]
    fn test_algebraic_sub_self() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let y = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(BinOp::Sub, x, y);

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Int(0))))
        ));
    }

    #[test]
    fn test_algebraic_xor_self() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let y = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(BinOp::BitXor, x, y);

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Int(0))))
        ));
    }

    #[test]
    fn test_algebraic_eq_self() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let y = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(BinOp::Eq, x, y);

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))))
        ));
    }

    #[test]
    fn test_algebraic_ne_self() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let y = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(BinOp::Ne, x, y);

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))))
        ));
    }

    #[test]
    fn test_algebraic_and_self() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let y = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(BinOp::BitAnd, x.clone(), y);

        let simplified = try_algebraic_simplify(&rvalue);
        if let Some(Rvalue::Use(op)) = simplified {
            if let Operand::Copy(place) = op {
                assert_eq!(place.local, Local(1));
            } else {
                panic!("Expected Copy operand");
            }
        } else {
            panic!("Expected simplified result");
        }
    }

    #[test]
    fn test_algebraic_or_self() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let y = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(BinOp::BitOr, x.clone(), y);

        let simplified = try_algebraic_simplify(&rvalue);
        if let Some(Rvalue::Use(op)) = simplified {
            if let Operand::Copy(place) = op {
                assert_eq!(place.local, Local(1));
            } else {
                panic!("Expected Copy operand");
            }
        } else {
            panic!("Expected simplified result");
        }
    }

    #[test]
    fn test_algebraic_float_add_zero() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Add,
            x.clone(),
            Operand::Constant(Constant::Float(0.0)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_float_mul_one() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Mul,
            x.clone(),
            Operand::Constant(Constant::Float(1.0)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_algebraic_float_mul_zero() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let rvalue = Rvalue::BinaryOp(
            BinOp::Mul,
            x.clone(),
            Operand::Constant(Constant::Float(0.0)),
        );

        let simplified = try_algebraic_simplify(&rvalue);
        if let Some(Rvalue::Use(Operand::Constant(Constant::Float(f)))) = simplified {
            assert!((f - 0.0).abs() < 0.001);
        } else {
            panic!("Expected Float constant 0.0");
        }
    }

    #[test]
    fn test_algebraic_simplify_full_function() {
        let mut func = make_test_function();
        let x = func.new_local(MirType::Int, Some("x".into())); // Local(1)
        let y = func.new_local(MirType::Int, Some("y".into())); // Local(2)

        let b0 = func.new_block();

        // y = x + 0 (should become y = x)
        func.block_mut(b0).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(y),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Copy(Place::from_local(x)),
                    Operand::Constant(Constant::Int(0)),
                ),
            ),
            span: Span::dummy(),
        });

        func.block_mut(b0).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        algebraic_simplify(&mut func);

        // Check that it's now y = x
        if let StatementKind::Assign(_, Rvalue::Use(Operand::Copy(place))) =
            &func.blocks[0].statements[0].kind
        {
            assert_eq!(place.local, x);
        } else {
            panic!("Expected simplified assignment");
        }
    }

    #[test]
    fn test_bitwise_shift_zero() {
        let x = Operand::Copy(Place::from_local(Local(1)));

        // x << 0 should simplify to x
        let rvalue_shl = Rvalue::BinaryOp(
            BinOp::Shl,
            x.clone(),
            Operand::Constant(Constant::Int(0)),
        );
        let simplified = try_algebraic_simplify(&rvalue_shl);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));

        // x >> 0 should simplify to x
        let rvalue_shr = Rvalue::BinaryOp(
            BinOp::Shr,
            x.clone(),
            Operand::Constant(Constant::Int(0)),
        );
        let simplified = try_algebraic_simplify(&rvalue_shr);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_bitwise_xor_zero() {
        let x = Operand::Copy(Place::from_local(Local(1)));

        // x ^ 0 should simplify to x
        let rvalue = Rvalue::BinaryOp(
            BinOp::BitXor,
            x.clone(),
            Operand::Constant(Constant::Int(0)),
        );
        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_bitwise_or_zero() {
        let x = Operand::Copy(Place::from_local(Local(1)));

        // x | 0 should simplify to x
        let rvalue = Rvalue::BinaryOp(
            BinOp::BitOr,
            x.clone(),
            Operand::Constant(Constant::Int(0)),
        );
        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_sub_zero() {
        let x = Operand::Copy(Place::from_local(Local(1)));

        // x - 0 should simplify to x
        let rvalue = Rvalue::BinaryOp(
            BinOp::Sub,
            x.clone(),
            Operand::Constant(Constant::Int(0)),
        );
        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_intdiv_one() {
        let x = Operand::Copy(Place::from_local(Local(1)));

        // x // 1 should simplify to x
        let rvalue = Rvalue::BinaryOp(
            BinOp::IntDiv,
            x.clone(),
            Operand::Constant(Constant::Int(1)),
        );
        let simplified = try_algebraic_simplify(&rvalue);
        assert!(matches!(simplified, Some(Rvalue::Use(_))));
    }

    #[test]
    fn test_comparison_self() {
        let x = Operand::Copy(Place::from_local(Local(1)));
        let y = Operand::Copy(Place::from_local(Local(1)));

        // x <= x should be true
        let rvalue_le = Rvalue::BinaryOp(BinOp::Le, x.clone(), y.clone());
        let simplified = try_algebraic_simplify(&rvalue_le);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))))
        ));

        // x >= x should be true
        let x2 = Operand::Copy(Place::from_local(Local(1)));
        let y2 = Operand::Copy(Place::from_local(Local(1)));
        let rvalue_ge = Rvalue::BinaryOp(BinOp::Ge, x2, y2);
        let simplified = try_algebraic_simplify(&rvalue_ge);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Bool(true))))
        ));

        // x < x should be false
        let x3 = Operand::Copy(Place::from_local(Local(1)));
        let y3 = Operand::Copy(Place::from_local(Local(1)));
        let rvalue_lt = Rvalue::BinaryOp(BinOp::Lt, x3, y3);
        let simplified = try_algebraic_simplify(&rvalue_lt);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))))
        ));

        // x > x should be false
        let x4 = Operand::Copy(Place::from_local(Local(1)));
        let y4 = Operand::Copy(Place::from_local(Local(1)));
        let rvalue_gt = Rvalue::BinaryOp(BinOp::Gt, x4, y4);
        let simplified = try_algebraic_simplify(&rvalue_gt);
        assert!(matches!(
            simplified,
            Some(Rvalue::Use(Operand::Constant(Constant::Bool(false))))
        ));
    }
}
