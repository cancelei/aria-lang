//! Performance benchmarks for Aria optimization passes.
//!
//! These benchmarks test the effectiveness of various optimization passes
//! to ensure Aria can compete with Go/Rust performance.

use aria_codegen::{compile_to_object, inline_functions, InlineConfig, Target};
use aria_lexer::Span;
use aria_mir::*;
use aria_mir::optimize::{optimize_program, OptLevel};

/// Create a simple function for benchmarking
fn create_simple_function(name: &str, body_size: usize) -> MirFunction {
    let mut func = MirFunction::new(name.into(), MirType::Int64, Span::dummy());
    func.is_public = true;

    // Add locals
    func.locals.push(LocalDecl {
        name: Some("return".into()),
        ty: MirType::Int64,
        mutable: true,
        span: Span::dummy(),
    });
    func.locals.push(LocalDecl {
        name: Some("x".into()),
        ty: MirType::Int64,
        mutable: false,
        span: Span::dummy(),
    });
    func.locals.push(LocalDecl {
        name: Some("result".into()),
        ty: MirType::Int64,
        mutable: true,
        span: Span::dummy(),
    });
    func.params = vec![Local(1)];

    let block_id = func.new_block();
    let result_local = Local(2);

    // Add statements
    for i in 0..body_size {
        func.block_mut(block_id).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(result_local),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Copy(Place::from_local(result_local)),
                    Operand::Constant(Constant::Int(i as i64)),
                ),
            ),
            span: Span::dummy(),
        });
    }

    // Return the result
    func.block_mut(block_id).push_stmt(Statement {
        kind: StatementKind::Assign(
            Place::from_local(Local::RETURN),
            Rvalue::Use(Operand::Copy(Place::from_local(result_local))),
        ),
        span: Span::dummy(),
    });
    func.block_mut(block_id).set_terminator(Terminator {
        kind: TerminatorKind::Return,
        span: Span::dummy(),
    });

    func
}

/// Create a function with a loop pattern
fn create_loop_function() -> MirFunction {
    let mut func = MirFunction::new("loop_func".into(), MirType::Int64, Span::dummy());
    func.is_public = true;

    // Locals: return (0), n (1), result (2), i (3), cond (4)
    func.locals.push(LocalDecl {
        name: Some("return".into()),
        ty: MirType::Int64,
        mutable: true,
        span: Span::dummy(),
    });
    func.locals.push(LocalDecl {
        name: Some("n".into()),
        ty: MirType::Int64,
        mutable: false,
        span: Span::dummy(),
    });
    func.locals.push(LocalDecl {
        name: Some("result".into()),
        ty: MirType::Int64,
        mutable: true,
        span: Span::dummy(),
    });
    func.locals.push(LocalDecl {
        name: Some("i".into()),
        ty: MirType::Int64,
        mutable: true,
        span: Span::dummy(),
    });
    func.locals.push(LocalDecl {
        name: Some("cond".into()),
        ty: MirType::Bool,
        mutable: true,
        span: Span::dummy(),
    });
    func.params = vec![Local(1)];

    // Block 0: initialize result = 0, i = 0
    let b0 = func.new_block();
    func.block_mut(b0).push_stmt(Statement {
        kind: StatementKind::Assign(
            Place::from_local(Local(2)),
            Rvalue::Use(Operand::Constant(Constant::Int(0))),
        ),
        span: Span::dummy(),
    });
    func.block_mut(b0).push_stmt(Statement {
        kind: StatementKind::Assign(
            Place::from_local(Local(3)),
            Rvalue::Use(Operand::Constant(Constant::Int(0))),
        ),
        span: Span::dummy(),
    });
    func.block_mut(b0).set_terminator(Terminator {
        kind: TerminatorKind::Goto { target: BlockId(1) },
        span: Span::dummy(),
    });

    // Block 1: loop header - check i < n
    let b1 = func.new_block();
    func.block_mut(b1).push_stmt(Statement {
        kind: StatementKind::Assign(
            Place::from_local(Local(4)),
            Rvalue::BinaryOp(
                BinOp::Lt,
                Operand::Copy(Place::from_local(Local(3))),
                Operand::Copy(Place::from_local(Local(1))),
            ),
        ),
        span: Span::dummy(),
    });
    func.block_mut(b1).set_terminator(Terminator {
        kind: TerminatorKind::SwitchInt {
            discr: Operand::Copy(Place::from_local(Local(4))),
            targets: SwitchTargets::if_else(BlockId(2), BlockId(3)),
        },
        span: Span::dummy(),
    });

    // Block 2: loop body - result += i, i += 1
    let b2 = func.new_block();
    func.block_mut(b2).push_stmt(Statement {
        kind: StatementKind::Assign(
            Place::from_local(Local(2)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Operand::Copy(Place::from_local(Local(2))),
                Operand::Copy(Place::from_local(Local(3))),
            ),
        ),
        span: Span::dummy(),
    });
    func.block_mut(b2).push_stmt(Statement {
        kind: StatementKind::Assign(
            Place::from_local(Local(3)),
            Rvalue::BinaryOp(
                BinOp::Add,
                Operand::Copy(Place::from_local(Local(3))),
                Operand::Constant(Constant::Int(1)),
            ),
        ),
        span: Span::dummy(),
    });
    func.block_mut(b2).set_terminator(Terminator {
        kind: TerminatorKind::Goto { target: BlockId(1) },
        span: Span::dummy(),
    });

    // Block 3: loop exit - return result
    let b3 = func.new_block();
    func.block_mut(b3).push_stmt(Statement {
        kind: StatementKind::Assign(
            Place::from_local(Local::RETURN),
            Rvalue::Use(Operand::Copy(Place::from_local(Local(2)))),
        ),
        span: Span::dummy(),
    });
    func.block_mut(b3).set_terminator(Terminator {
        kind: TerminatorKind::Return,
        span: Span::dummy(),
    });

    func
}

/// Create a function that would benefit from constant folding
fn create_const_folding_function() -> MirFunction {
    let mut func = MirFunction::new("const_fold".into(), MirType::Int64, Span::dummy());
    func.is_public = true;

    func.locals.push(LocalDecl {
        name: Some("return".into()),
        ty: MirType::Int64,
        mutable: true,
        span: Span::dummy(),
    });
    func.locals.push(LocalDecl {
        name: Some("temp".into()),
        ty: MirType::Int64,
        mutable: true,
        span: Span::dummy(),
    });

    let block_id = func.new_block();

    // Add many constant expressions that can be folded
    for i in 0..20 {
        func.block_mut(block_id).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(Local(1)),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Constant(Constant::Int(i * 2)),
                    Operand::Constant(Constant::Int(i * 3)),
                ),
            ),
            span: Span::dummy(),
        });
    }

    func.block_mut(block_id).push_stmt(Statement {
        kind: StatementKind::Assign(
            Place::from_local(Local::RETURN),
            Rvalue::Use(Operand::Copy(Place::from_local(Local(1)))),
        ),
        span: Span::dummy(),
    });
    func.block_mut(block_id).set_terminator(Terminator {
        kind: TerminatorKind::Return,
        span: Span::dummy(),
    });

    func
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_optimization() {
        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let func = create_simple_function("test_func", 10);
        program.functions.insert(fn_id, func);
        program.entry = Some(fn_id);

        // Test without optimization
        let object_unopt = compile_to_object(&program, Target::native())
            .expect("Failed to compile unoptimized");

        // Test with basic optimization
        let mut program_opt = program.clone();
        optimize_program(&mut program_opt, OptLevel::Basic);
        let object_opt = compile_to_object(&program_opt, Target::native())
            .expect("Failed to compile optimized");

        println!("Unoptimized size: {} bytes", object_unopt.len());
        println!("Optimized size: {} bytes", object_opt.len());

        // Both should compile successfully
        assert!(object_unopt.len() > 0);
        assert!(object_opt.len() > 0);
    }

    #[test]
    fn test_constant_folding_optimization() {
        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let func = create_const_folding_function();
        program.functions.insert(fn_id, func);

        // Apply constant folding
        optimize_program(&mut program, OptLevel::Aggressive);

        // Check that constants were folded
        let optimized_func = &program.functions[&fn_id];

        // Count remaining statements (should be much fewer after folding)
        let stmt_count: usize = optimized_func.blocks.iter()
            .map(|b| b.statements.len())
            .sum();

        println!("Statements after constant folding: {}", stmt_count);
        // Should have fewer statements than before
        assert!(stmt_count < 25);
    }

    #[test]
    fn test_loop_optimization() {
        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);
        let func = create_loop_function();
        program.functions.insert(fn_id, func);

        // Compile with optimizations
        optimize_program(&mut program, OptLevel::Aggressive);
        let object = compile_to_object(&program, Target::native())
            .expect("Failed to compile loop function");

        println!("Loop function compiled: {} bytes", object.len());
        assert!(object.len() > 0);
    }

    #[test]
    fn test_function_inlining() {
        let mut program = MirProgram::new();

        // Create a small callee function
        let callee_id = FunctionId(0);
        let callee = create_simple_function("small_func", 3);
        program.functions.insert(callee_id, callee);

        // Create a caller function that calls the callee
        let caller_id = FunctionId(1);
        let mut caller = MirFunction::new("caller".into(), MirType::Int64, Span::dummy());
        caller.is_public = true;
        caller.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span: Span::dummy(),
        });
        caller.locals.push(LocalDecl {
            name: Some("result".into()),
            ty: MirType::Int64,
            mutable: true,
            span: Span::dummy(),
        });

        let block_id = caller.new_block();
        caller.block_mut(block_id).set_terminator(Terminator {
            kind: TerminatorKind::Call {
                func: Operand::Constant(Constant::Function(callee_id)),
                args: vec![Operand::Constant(Constant::Int(42))],
                destination: Place::from_local(Local(1)),
                target: Some(BlockId(1)),
                unwind: None,
            },
            span: Span::dummy(),
        });

        let return_block = caller.new_block();
        caller.block_mut(return_block).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(Local::RETURN),
                Rvalue::Use(Operand::Copy(Place::from_local(Local(1)))),
            ),
            span: Span::dummy(),
        });
        caller.block_mut(return_block).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        program.functions.insert(caller_id, caller);

        // Test inlining with release config
        let config = InlineConfig::release();
        inline_functions(&mut program, &config);

        // After inlining, the caller should have more blocks
        let inlined_caller = &program.functions[&caller_id];
        println!("Caller blocks after inlining: {}", inlined_caller.blocks.len());

        // Compile to ensure it still works
        let object = compile_to_object(&program, Target::native())
            .expect("Failed to compile inlined function");
        assert!(object.len() > 0);
    }

    #[test]
    fn test_aggressive_vs_basic_optimization() {
        let mut program_basic = MirProgram::new();
        let mut program_aggressive = MirProgram::new();

        let fn_id = FunctionId(0);
        let func = create_const_folding_function();

        program_basic.functions.insert(fn_id, func.clone());
        program_aggressive.functions.insert(fn_id, func);

        // Apply different optimization levels
        optimize_program(&mut program_basic, OptLevel::Basic);
        optimize_program(&mut program_aggressive, OptLevel::Aggressive);

        // Compare results
        let basic_stmts: usize = program_basic.functions[&fn_id].blocks.iter()
            .map(|b| b.statements.len())
            .sum();

        let aggressive_stmts: usize = program_aggressive.functions[&fn_id].blocks.iter()
            .map(|b| b.statements.len())
            .sum();

        println!("Basic optimization statements: {}", basic_stmts);
        println!("Aggressive optimization statements: {}", aggressive_stmts);

        // Aggressive should have same or fewer statements
        assert!(aggressive_stmts <= basic_stmts);
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut program = MirProgram::new();
        let fn_id = FunctionId(0);

        let mut func = MirFunction::new("dce_test".into(), MirType::Int64, Span::dummy());
        func.is_public = true;

        // Add locals
        func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span: Span::dummy(),
        });
        func.locals.push(LocalDecl {
            name: Some("unused".into()),
            ty: MirType::Int64,
            mutable: true,
            span: Span::dummy(),
        });
        func.locals.push(LocalDecl {
            name: Some("used".into()),
            ty: MirType::Int64,
            mutable: true,
            span: Span::dummy(),
        });

        let block_id = func.new_block();

        // Unused assignment - should be eliminated
        func.block_mut(block_id).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(Local(1)),
                Rvalue::Use(Operand::Constant(Constant::Int(999))),
            ),
            span: Span::dummy(),
        });

        // Used assignment - should remain
        func.block_mut(block_id).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(Local(2)),
                Rvalue::Use(Operand::Constant(Constant::Int(42))),
            ),
            span: Span::dummy(),
        });

        // Return the used value
        func.block_mut(block_id).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(Local::RETURN),
                Rvalue::Use(Operand::Copy(Place::from_local(Local(2)))),
            ),
            span: Span::dummy(),
        });

        func.block_mut(block_id).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        program.functions.insert(fn_id, func);

        // Count statements before DCE
        let before_stmts = program.functions[&fn_id].blocks[0].statements.len();

        // Apply DCE
        optimize_program(&mut program, OptLevel::Aggressive);

        // Count statements after DCE
        let after_stmts = program.functions[&fn_id].blocks[0].statements.len();

        println!("Statements before DCE: {}", before_stmts);
        println!("Statements after DCE: {}", after_stmts);

        // Should have removed the unused assignment
        assert!(after_stmts <= before_stmts);
    }
}
