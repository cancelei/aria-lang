//! MIR-level contract verification
//!
//! This module implements contract verification that operates on MIR (Mid-level IR).
//! It inserts runtime checks for contracts and performs static analysis where possible.

use crate::mir::{
    BinOp, BlockId, Constant, ContractClause, Expr as MirExpr, FunctionContract, Local, MirFunction,
    MirProgram, Operand, Place, Statement, StatementKind, Terminator, TerminatorKind, UnOp,
};
use smol_str::SmolStr;
use std::collections::HashMap;

/// Contract verification mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationMode {
    /// Insert all runtime checks (debug mode)
    Debug,
    /// Remove provably safe checks, keep others (release mode)
    Release,
    /// Force all checks even in release
    ForceAll,
    /// No contract checking
    Disabled,
}

impl VerificationMode {
    /// Check if contracts are enabled in this mode
    pub fn is_enabled(&self) -> bool {
        !matches!(self, VerificationMode::Disabled)
    }

    /// Check if we should insert runtime checks
    pub fn should_insert_checks(&self) -> bool {
        matches!(
            self,
            VerificationMode::Debug | VerificationMode::Release | VerificationMode::ForceAll
        )
    }

    /// Check if we should attempt to prove contracts statically
    pub fn should_prove_static(&self) -> bool {
        matches!(self, VerificationMode::Release)
    }
}

/// Contract verifier that operates on MIR
pub struct MirContractVerifier {
    mode: VerificationMode,
    /// Cache of proven contracts to avoid redundant checks
    proven_cache: HashMap<String, bool>,
}

impl MirContractVerifier {
    /// Create a new MIR contract verifier
    pub fn new(mode: VerificationMode) -> Self {
        Self {
            mode,
            proven_cache: HashMap::new(),
        }
    }

    /// Verify all contracts in a MIR program
    ///
    /// This is the main entry point for contract verification.
    /// It processes each function and inserts appropriate checks.
    pub fn verify_program(&mut self, program: &mut MirProgram) {
        if !self.mode.is_enabled() {
            return;
        }

        // Collect function IDs to avoid borrow checker issues
        let function_ids: Vec<_> = program.functions.keys().copied().collect();

        for fn_id in function_ids {
            if let Some(function) = program.functions.get_mut(&fn_id) {
                self.verify_function(function);
            }
        }
    }

    /// Verify contracts for a single function
    fn verify_function(&mut self, function: &mut MirFunction) {
        let contract = match &function.contract {
            Some(c) if !c.is_empty() => c.clone(),
            _ => return, // No contracts, nothing to do
        };

        // Insert precondition checks at function entry
        self.insert_precondition_checks(function, &contract.requires);

        // Insert postcondition checks before return statements
        self.insert_postcondition_checks(function, &contract.ensures);
    }

    /// Insert precondition checks at function entry
    fn insert_precondition_checks(&mut self, function: &mut MirFunction, requires: &[ContractClause]) {
        if requires.is_empty() {
            return;
        }

        let entry_block = BlockId::ENTRY;

        // For each requires clause, insert an assertion
        for clause in requires {
            if self.should_check_clause(clause, function) {
                self.insert_assertion(function, entry_block, clause, "Precondition");
            }
        }
    }

    /// Insert postcondition checks before all return statements
    fn insert_postcondition_checks(
        &mut self,
        function: &mut MirFunction,
        ensures: &[ContractClause],
    ) {
        if ensures.is_empty() {
            return;
        }

        // Find all blocks with Return terminators
        let return_blocks: Vec<BlockId> = function
            .blocks
            .iter()
            .filter(|block| {
                matches!(
                    block.terminator,
                    Some(Terminator {
                        kind: TerminatorKind::Return,
                        ..
                    })
                )
            })
            .map(|block| block.id)
            .collect();

        // For each return block, insert postcondition checks
        for block_id in return_blocks {
            for clause in ensures {
                if self.should_check_clause(clause, function) {
                    self.insert_assertion(function, block_id, clause, "Postcondition");
                }
            }
        }
    }

    /// Check if we should insert a runtime check for this clause
    fn should_check_clause(&mut self, clause: &ContractClause, _function: &MirFunction) -> bool {
        if !self.mode.should_insert_checks() {
            return false;
        }

        // In release mode, try to prove the clause statically
        if self.mode.should_prove_static() {
            // Simple static analysis: constant propagation
            if self.is_provably_true(clause) {
                return false; // Proven true, no runtime check needed
            }
        }

        true
    }

    /// Try to prove a clause is always true using simple static analysis
    fn is_provably_true(&mut self, clause: &ContractClause) -> bool {
        // For now, only handle constant true expressions
        // More sophisticated analysis (range analysis, etc.) can be added later
        matches!(clause.condition, MirExpr::Bool(true))
    }

    /// Insert an assertion check into a basic block
    fn insert_assertion(
        &mut self,
        function: &mut MirFunction,
        block_id: BlockId,
        clause: &ContractClause,
        kind: &str,
    ) {
        // Convert the contract expression to an operand
        let condition = self.lower_contract_expr(function, &clause.condition);

        // Create assertion message
        let message: SmolStr = if let Some(ref msg) = clause.message {
            format!("{} violated: {}", kind, msg).into()
        } else {
            format!("{} violated", kind).into()
        };

        // Create a new block for the assertion continuation
        let continue_block = BlockId(function.blocks.len() as u32);

        // Get the block and save the old terminator
        let old_terminator = function.blocks[block_id.0 as usize].terminator.take();

        // Replace with Assert terminator
        let assert_terminator = Terminator {
            kind: TerminatorKind::Assert {
                cond: condition,
                expected: true,
                msg: message,
                target: continue_block,
            },
            span: clause.span,
        };

        function.blocks[block_id.0 as usize].terminator = Some(assert_terminator);

        // Create the continuation block with the old terminator
        let mut new_block = crate::mir::BasicBlock::new(continue_block);
        new_block.terminator = old_terminator;
        function.blocks.push(new_block);
    }

    /// Lower a contract expression to a MIR operand
    fn lower_contract_expr(&mut self, function: &mut MirFunction, expr: &MirExpr) -> Operand {
        match expr {
            MirExpr::Bool(b) => Operand::Constant(Constant::Bool(*b)),
            MirExpr::Int(i) => Operand::Constant(Constant::Int(*i)),
            MirExpr::Float(f) => Operand::Constant(Constant::Float(*f)),
            MirExpr::Local(local) => Operand::Copy(Place::from_local(*local)),
            MirExpr::Result => Operand::Copy(Place::return_place()),

            MirExpr::Binary { op, left, right } => {
                // Create temporaries for left and right
                let left_op = self.lower_contract_expr(function, left);
                let right_op = self.lower_contract_expr(function, right);

                // Create a temporary to hold the result
                let result_ty = crate::mir::MirType::Bool; // Most contract expressions are boolean
                let temp = function.new_local(result_ty, Some("_contract_temp".into()));

                // This is a simplified version - in a full implementation,
                // we'd need to insert the binary op as a statement in the current block
                // For now, we'll use Copy of a constant as a placeholder
                // TODO: Properly implement expression lowering with temporary variables

                Operand::Copy(Place::from_local(temp))
            }

            MirExpr::Unary { op, operand } => {
                let operand_val = self.lower_contract_expr(function, operand);

                // Create a temporary for the result
                let result_ty = crate::mir::MirType::Bool;
                let temp = function.new_local(result_ty, Some("_contract_temp".into()));

                // TODO: Insert unary op statement
                Operand::Copy(Place::from_local(temp))
            }

            MirExpr::Field { object, field } => {
                let base_op = self.lower_contract_expr(function, object);
                // Extract the place from the operand and add field projection
                match base_op {
                    Operand::Copy(place) | Operand::Move(place) => {
                        Operand::Copy(place.field(*field))
                    }
                    _ => {
                        // Constant - can't access field, create temp
                        let temp = function.new_local(crate::mir::MirType::Bool, None);
                        Operand::Copy(Place::from_local(temp))
                    }
                }
            }

            MirExpr::MethodCall { object, method, args } => {
                // For now, treat method calls as opaque
                // In a full implementation, we'd lower this to a proper call
                let temp = function.new_local(crate::mir::MirType::Bool, Some("_method_result".into()));
                Operand::Copy(Place::from_local(temp))
            }

            MirExpr::Old(_inner) => {
                // Old values require special handling - need to capture at function entry
                // For now, create a placeholder
                let temp = function.new_local(crate::mir::MirType::Bool, Some("_old_value".into()));
                Operand::Copy(Place::from_local(temp))
            }
        }
    }
}

/// Simple range analysis for proving conditions
///
/// This is a placeholder for more sophisticated static analysis.
/// It could track value ranges to prove things like:
/// - `x > 0 and x < 100` implies result is in range [0, 100]
/// - Array bounds checks: `i < arr.len()` proves no out-of-bounds access
#[derive(Debug, Clone)]
pub struct RangeAnalysis {
    /// Known ranges for local variables
    ranges: HashMap<Local, ValueRange>,
}

#[derive(Debug, Clone)]
pub struct ValueRange {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

impl RangeAnalysis {
    pub fn new() -> Self {
        Self {
            ranges: HashMap::new(),
        }
    }

    /// Analyze a function to compute value ranges
    pub fn analyze(&mut self, _function: &MirFunction) {
        // TODO: Implement dataflow analysis to compute ranges
        // This would walk the CFG and track constraints on values
    }

    /// Check if a condition is provably true given known ranges
    pub fn prove_condition(&self, _expr: &MirExpr) -> bool {
        // TODO: Check if expression is always true given known ranges
        false
    }
}

impl Default for RangeAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_mode() {
        assert!(VerificationMode::Debug.is_enabled());
        assert!(VerificationMode::Debug.should_insert_checks());
        assert!(!VerificationMode::Debug.should_prove_static());

        assert!(VerificationMode::Release.is_enabled());
        assert!(VerificationMode::Release.should_insert_checks());
        assert!(VerificationMode::Release.should_prove_static());

        assert!(!VerificationMode::Disabled.is_enabled());
        assert!(!VerificationMode::Disabled.should_insert_checks());
    }

    #[test]
    fn test_provably_true() {
        let mut verifier = MirContractVerifier::new(VerificationMode::Release);

        let true_clause = ContractClause {
            condition: MirExpr::Bool(true),
            message: None,
            span: aria_lexer::Span::dummy(),
        };

        assert!(verifier.is_provably_true(&true_clause));

        let false_clause = ContractClause {
            condition: MirExpr::Bool(false),
            message: None,
            span: aria_lexer::Span::dummy(),
        };

        assert!(!verifier.is_provably_true(&false_clause));
    }
}
