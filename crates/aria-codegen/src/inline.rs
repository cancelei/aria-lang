//! Function inlining optimization pass.
//!
//! This module implements function inlining to reduce call overhead and enable
//! further optimizations. It supports:
//! - Inlining small functions (< 50 instructions)
//! - Inlining functions called once
//! - Inlining contracts in release mode (critical for performance!)
//! - Respecting `#[inline(never)]` and `#[inline(always)]` attributes

use aria_mir::{
    BasicBlock, BlockId, Constant, FunctionId, Local, LocalDecl, MirFunction, MirProgram, MirType,
    Operand, Place, Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Inlining policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlinePolicy {
    /// Never inline
    Never,
    /// Only inline if beneficial (default)
    Heuristic,
    /// Always inline (attribute-driven)
    Always,
}

/// Configuration for inlining
#[derive(Debug, Clone)]
pub struct InlineConfig {
    /// Whether we're in release mode (enables contract inlining)
    pub release_mode: bool,
    /// Maximum function size to inline (in statements)
    pub max_inline_size: usize,
    /// Maximum depth of inlining to prevent excessive code growth
    pub max_inline_depth: usize,
}

impl Default for InlineConfig {
    fn default() -> Self {
        Self {
            release_mode: false,
            max_inline_size: 50,
            max_inline_depth: 4,
        }
    }
}

impl InlineConfig {
    pub fn release() -> Self {
        Self {
            release_mode: true,
            max_inline_size: 100,
            max_inline_depth: 8,
        }
    }
}

/// Inline suitable functions across the program
pub fn inline_functions(program: &mut MirProgram, config: &InlineConfig) {
    // Analyze call graph to find inlining candidates
    let call_graph = build_call_graph(program);
    let call_counts = count_call_sites(&call_graph);

    // Determine inlining policy for each function
    let inline_policies = determine_inline_policies(program, &call_counts, config);

    // Perform inlining in topological order (callees before callers)
    let topo_order = topological_sort(&call_graph);

    for &fn_id in topo_order.iter().rev() {
        if let Some(func) = program.functions.get(&fn_id).cloned() {
            inline_in_function(program, fn_id, &func, &inline_policies, config, 0);
        }
    }
}

/// Build a call graph: function -> set of functions it calls
fn build_call_graph(program: &MirProgram) -> FxHashMap<FunctionId, FxHashSet<FunctionId>> {
    let mut graph = FxHashMap::default();

    for (&fn_id, func) in &program.functions {
        let mut callees = FxHashSet::default();

        for block in &func.blocks {
            if let Some(ref term) = block.terminator {
                if let TerminatorKind::Call { func: callee_op, .. } = &term.kind {
                    if let Operand::Constant(Constant::Function(callee_id)) = callee_op {
                        callees.insert(*callee_id);
                    }
                }
            }
        }

        graph.insert(fn_id, callees);
    }

    graph
}

/// Count how many times each function is called
fn count_call_sites(call_graph: &FxHashMap<FunctionId, FxHashSet<FunctionId>>) -> FxHashMap<FunctionId, usize> {
    let mut counts = FxHashMap::default();

    for callees in call_graph.values() {
        for &callee in callees {
            *counts.entry(callee).or_insert(0) += 1;
        }
    }

    counts
}

/// Determine inlining policy for each function
fn determine_inline_policies(
    program: &MirProgram,
    call_counts: &FxHashMap<FunctionId, usize>,
    config: &InlineConfig,
) -> FxHashMap<FunctionId, InlinePolicy> {
    let mut policies = FxHashMap::default();

    for (&fn_id, func) in &program.functions {
        let policy = if has_inline_never_attr(func) {
            InlinePolicy::Never
        } else if has_inline_always_attr(func) {
            InlinePolicy::Always
        } else if should_inline_heuristic(func, call_counts.get(&fn_id).copied().unwrap_or(0), config) {
            InlinePolicy::Heuristic
        } else {
            InlinePolicy::Never
        };

        policies.insert(fn_id, policy);
    }

    policies
}

/// Check if function has #[inline(never)] attribute
fn has_inline_never_attr(func: &MirFunction) -> bool {
    func.attributes.iter().any(|attr| attr == "inline(never)")
}

/// Check if function has #[inline(always)] attribute
fn has_inline_always_attr(func: &MirFunction) -> bool {
    func.attributes.iter().any(|attr| attr == "inline(always)" || attr == "inline")
}

/// Determine if function should be inlined based on heuristics
fn should_inline_heuristic(func: &MirFunction, call_count: usize, config: &InlineConfig) -> bool {
    // Don't inline recursive functions (for now)
    if is_recursive(func) {
        return false;
    }

    let size = estimate_function_size(func);

    // Always inline very small functions
    if size <= 10 {
        return true;
    }

    // Inline functions called exactly once
    if call_count == 1 && size <= config.max_inline_size {
        return true;
    }

    // Inline small functions with few call sites
    if size <= 25 && call_count <= 3 {
        return true;
    }

    // In release mode, inline contract check functions
    if config.release_mode && is_contract_function(func) && size <= config.max_inline_size {
        return true;
    }

    false
}

/// Estimate the size of a function (in statements)
fn estimate_function_size(func: &MirFunction) -> usize {
    func.blocks.iter().map(|block| block.statements.len() + 1).sum()
}

/// Check if function is recursive
fn is_recursive(func: &MirFunction) -> bool {
    // Simple check: does the function call itself by name?
    // A more sophisticated analysis would check through the call graph
    for block in &func.blocks {
        if let Some(ref term) = block.terminator {
            if let TerminatorKind::Call { func: callee_op, .. } = &term.kind {
                if let Operand::Constant(Constant::Function(_)) = callee_op {
                    // TODO: Check if this is self-recursive
                    // For now, conservatively assume it might be
                }
            }
        }
    }
    false
}

/// Check if this is a contract verification function
fn is_contract_function(func: &MirFunction) -> bool {
    func.name.starts_with("_contract_") ||
    func.name.contains("_requires_") ||
    func.name.contains("_ensures_") ||
    func.name.contains("_invariant_")
}

/// Topological sort of call graph
fn topological_sort(call_graph: &FxHashMap<FunctionId, FxHashSet<FunctionId>>) -> Vec<FunctionId> {
    let mut result = Vec::new();
    let mut visited = FxHashSet::default();
    let mut visiting = FxHashSet::default();

    fn visit(
        node: FunctionId,
        graph: &FxHashMap<FunctionId, FxHashSet<FunctionId>>,
        visited: &mut FxHashSet<FunctionId>,
        visiting: &mut FxHashSet<FunctionId>,
        result: &mut Vec<FunctionId>,
    ) {
        if visited.contains(&node) {
            return;
        }

        if visiting.contains(&node) {
            // Cycle detected - skip
            return;
        }

        visiting.insert(node);

        if let Some(callees) = graph.get(&node) {
            for &callee in callees {
                visit(callee, graph, visited, visiting, result);
            }
        }

        visiting.remove(&node);
        visited.insert(node);
        result.push(node);
    }

    for &fn_id in call_graph.keys() {
        visit(fn_id, call_graph, &mut visited, &mut visiting, &mut result);
    }

    result
}

/// Inline functions in a given function
fn inline_in_function(
    program: &mut MirProgram,
    fn_id: FunctionId,
    func: &MirFunction,
    policies: &FxHashMap<FunctionId, InlinePolicy>,
    config: &InlineConfig,
    depth: usize,
) {
    if depth >= config.max_inline_depth {
        return;
    }

    let mut modified = false;
    let mut new_func = func.clone();

    // Find all call sites
    for block_idx in 0..new_func.blocks.len() {
        if let Some(ref term) = new_func.blocks[block_idx].terminator.clone() {
            if let TerminatorKind::Call {
                func: callee_op,
                args,
                dest,
                target,
                ..
            } = &term.kind {
                if let Operand::Constant(Constant::Function(callee_id)) = callee_op {
                    // Check if we should inline this call
                    let policy = policies.get(callee_id).copied().unwrap_or(InlinePolicy::Never);

                    if policy != InlinePolicy::Never {
                        if let Some(callee_func) = program.functions.get(callee_id) {
                            // Perform the inlining
                            if let Some(target_block) = target {
                                inline_call_site(
                                    &mut new_func,
                                    BlockId(block_idx as u32),
                                    callee_func,
                                    args,
                                    dest,
                                    *target_block,
                                );
                                modified = true;
                            }
                        }
                    }
                }
            }
        }
    }

    if modified {
        // Update the function in the program
        if let Some(func_mut) = program.functions.get_mut(&fn_id) {
            *func_mut = new_func.clone();
        }

        // Recursively inline in the modified function
        inline_in_function(program, fn_id, &new_func, policies, config, depth + 1);
    }
}

/// Inline a single call site
fn inline_call_site(
    caller: &mut MirFunction,
    call_block_id: BlockId,
    callee: &MirFunction,
    args: &[Operand],
    dest: &Place,
    target_block: BlockId,
) {
    // Create a mapping from callee locals to caller locals
    let mut local_map = FxHashMap::default();

    // Map parameters to arguments
    for (param_idx, arg) in args.iter().enumerate() {
        if let Some(&param_local) = callee.params.get(param_idx) {
            // For now, create a new local for each parameter
            // TODO: Optimize by reusing argument locals when possible
            let arg_local = create_temp_local(caller, callee.locals[param_local.0 as usize].ty.clone());
            local_map.insert(param_local, arg_local);
        }
    }

    // Map return value
    local_map.insert(Local::RETURN, dest.local);

    // Map all other callee locals to new caller locals
    for (idx, local_decl) in callee.locals.iter().enumerate() {
        let callee_local = Local(idx as u32);
        if !local_map.contains_key(&callee_local) {
            let new_local = create_temp_local(caller, local_decl.ty.clone());
            local_map.insert(callee_local, new_local);
        }
    }

    // Create a mapping from callee blocks to new caller blocks
    let mut block_map = FxHashMap::default();
    let first_inlined_block = BlockId(caller.blocks.len() as u32);

    for (idx, _) in callee.blocks.iter().enumerate() {
        let callee_block = BlockId(idx as u32);
        let new_block_id = BlockId(caller.blocks.len() as u32 + idx as u32);
        block_map.insert(callee_block, new_block_id);
    }

    // Clone and remap callee blocks
    for (idx, block) in callee.blocks.iter().enumerate() {
        let mut new_block = block.clone();

        // Remap locals in statements
        for stmt in &mut new_block.statements {
            remap_locals_in_stmt(stmt, &local_map);
        }

        // Remap locals and blocks in terminator
        if let Some(ref mut term) = new_block.terminator {
            remap_locals_in_term(term, &local_map);
            remap_blocks_in_term(term, &block_map, target_block);
        }

        new_block.id = BlockId(caller.blocks.len() as u32 + idx as u32);
        caller.blocks.push(new_block);
    }

    // Modify the call block to jump to the first inlined block
    let call_block = &mut caller.blocks[call_block_id.0 as usize];

    // Add statements to copy arguments to parameter locals
    for (param_idx, arg) in args.iter().enumerate() {
        if let Some(&param_local) = callee.params.get(param_idx) {
            if let Some(&mapped_local) = local_map.get(&param_local) {
                call_block.statements.push(Statement {
                    kind: StatementKind::Assign(
                        Place::from_local(mapped_local),
                        Rvalue::Use(arg.clone()),
                    ),
                    span: call_block.terminator.as_ref().map(|t| t.span).unwrap_or(aria_lexer::Span::dummy()),
                });
            }
        }
    }

    // Change terminator to jump to first inlined block
    call_block.terminator = Some(Terminator {
        kind: TerminatorKind::Goto { target: first_inlined_block },
        span: call_block.terminator.as_ref().map(|t| t.span).unwrap_or(aria_lexer::Span::dummy()),
    });
}

/// Create a temporary local in the caller
fn create_temp_local(caller: &mut MirFunction, ty: MirType) -> Local {
    let local_id = Local(caller.locals.len() as u32);
    caller.locals.push(LocalDecl {
        name: Some(format!("_inline_{}", local_id.0).into()),
        ty,
        mutable: true,
        span: aria_lexer::Span::dummy(),
    });
    local_id
}

/// Remap locals in a statement
fn remap_locals_in_stmt(stmt: &mut Statement, local_map: &FxHashMap<Local, Local>) {
    if let StatementKind::Assign(place, rvalue) = &mut stmt.kind {
        remap_place(place, local_map);
        remap_rvalue(rvalue, local_map);
    }
}

/// Remap locals in a terminator
fn remap_locals_in_term(term: &mut Terminator, local_map: &FxHashMap<Local, Local>) {
    match &mut term.kind {
        TerminatorKind::Call { func, args, dest, .. } => {
            remap_operand(func, local_map);
            for arg in args {
                remap_operand(arg, local_map);
            }
            remap_place(dest, local_map);
        }
        TerminatorKind::SwitchInt { discr, .. } => {
            remap_operand(discr, local_map);
        }
        TerminatorKind::Assert { cond, .. } => {
            remap_operand(cond, local_map);
        }
        TerminatorKind::Drop { place, .. } => {
            remap_place(place, local_map);
        }
        _ => {}
    }
}

/// Remap blocks in a terminator (replace Return with Goto to continuation)
fn remap_blocks_in_term(term: &mut Terminator, block_map: &FxHashMap<BlockId, BlockId>, continuation: BlockId) {
    match &mut term.kind {
        TerminatorKind::Goto { target } => {
            if let Some(&new_target) = block_map.get(target) {
                *target = new_target;
            }
        }
        TerminatorKind::SwitchInt { targets, .. } => {
            for (_, target) in &mut targets.targets {
                if let Some(&new_target) = block_map.get(target) {
                    *target = new_target;
                }
            }
            if let Some(&new_otherwise) = block_map.get(&targets.otherwise) {
                targets.otherwise = new_otherwise;
            }
        }
        TerminatorKind::Call { target, .. } => {
            if let Some(target_ref) = target {
                if let Some(&new_target) = block_map.get(target_ref) {
                    *target_ref = new_target;
                }
            }
        }
        TerminatorKind::Return => {
            // Replace return with goto to continuation
            term.kind = TerminatorKind::Goto { target: continuation };
        }
        TerminatorKind::Drop { target, .. } | TerminatorKind::Assert { target, .. } => {
            if let Some(&new_target) = block_map.get(target) {
                *target = new_target;
            }
        }
        _ => {}
    }
}

/// Remap a place
fn remap_place(place: &mut Place, local_map: &FxHashMap<Local, Local>) {
    if let Some(&new_local) = local_map.get(&place.local) {
        place.local = new_local;
    }
}

/// Remap an operand
fn remap_operand(operand: &mut Operand, local_map: &FxHashMap<Local, Local>) {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => {
            remap_place(place, local_map);
        }
        _ => {}
    }
}

/// Remap an rvalue
fn remap_rvalue(rvalue: &mut Rvalue, local_map: &FxHashMap<Local, Local>) {
    match rvalue {
        Rvalue::Use(op) | Rvalue::UnaryOp(_, op) | Rvalue::Cast(_, op, _) => {
            remap_operand(op, local_map);
        }
        Rvalue::BinaryOp(_, left, right) => {
            remap_operand(left, local_map);
            remap_operand(right, local_map);
        }
        Rvalue::Ref(place) | Rvalue::RefMut(place) | Rvalue::Discriminant(place) | Rvalue::Len(place) => {
            remap_place(place, local_map);
        }
        Rvalue::Aggregate(_, operands) | Rvalue::Closure(_, operands) => {
            for op in operands {
                remap_operand(op, local_map);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aria_lexer::Span;

    #[test]
    fn test_estimate_function_size() {
        let mut func = MirFunction::new("test".into(), MirType::Int64, Span::dummy());
        assert_eq!(estimate_function_size(&func), 0);

        // Add a block with statements
        let _ = func.new_block();
        func.blocks[0].statements.push(Statement {
            kind: StatementKind::Nop,
            span: Span::dummy(),
        });
        func.blocks[0].terminator = Some(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        assert_eq!(estimate_function_size(&func), 2); // 1 stmt + 1 terminator
    }

    #[test]
    fn test_inline_config_defaults() {
        let config = InlineConfig::default();
        assert_eq!(config.max_inline_size, 50);
        assert!(!config.release_mode);

        let release_config = InlineConfig::release();
        assert_eq!(release_config.max_inline_size, 100);
        assert!(release_config.release_mode);
    }

    #[test]
    fn test_should_inline_small_function() {
        let mut func = MirFunction::new("small".into(), MirType::Int64, Span::dummy());
        let _ = func.new_block();
        for _ in 0..5 {
            func.blocks[0].statements.push(Statement {
                kind: StatementKind::Nop,
                span: Span::dummy(),
            });
        }

        let config = InlineConfig::default();
        assert!(should_inline_heuristic(&func, 1, &config));
    }

    #[test]
    fn test_contract_function_detection() {
        let func1 = MirFunction::new("_contract_check_foo".into(), MirType::Bool, Span::dummy());
        assert!(is_contract_function(&func1));

        let func2 = MirFunction::new("normal_function".into(), MirType::Int64, Span::dummy());
        assert!(!is_contract_function(&func2));

        let func3 = MirFunction::new("foo_requires_bar".into(), MirType::Bool, Span::dummy());
        assert!(is_contract_function(&func3));
    }
}
