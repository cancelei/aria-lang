//! Pretty printing for MIR.
//!
//! This module provides human-readable output for MIR structures,
//! useful for debugging and understanding the lowered representation.
//!
//! This includes pretty printing for effect-related constructs such as:
//! - Effect definitions and operations
//! - Handler definitions
//! - Effect statements (InstallHandler, PerformEffect, etc.)
//! - Effect terminators (Yield, Resume, Handle)
//! - Evidence parameters and layouts

use std::fmt::Write;

use crate::mir::*;

/// Pretty print an entire MIR program
pub fn pretty_print(program: &MirProgram) -> String {
    let mut out = String::new();
    let mut printer = PrettyPrinter::new(&mut out, program);
    printer.print_program();
    out
}

/// Pretty print a single MIR function
#[allow(dead_code)]
pub fn pretty_print_function(func: &MirFunction, program: &MirProgram) -> String {
    let mut out = String::new();
    let mut printer = PrettyPrinter::new(&mut out, program);
    printer.print_function(func);
    out
}

#[allow(dead_code)]
struct PrettyPrinter<'a> {
    out: &'a mut String,
    program: &'a MirProgram,
    indent: usize,
}

impl<'a> PrettyPrinter<'a> {
    fn new(out: &'a mut String, program: &'a MirProgram) -> Self {
        Self {
            out,
            program,
            indent: 0,
        }
    }

    fn indent(&mut self) {
        self.indent += 2;
    }

    fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(2);
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push(' ');
        }
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn print_program(&mut self) {
        // Print effects
        for (id, e) in &self.program.effects {
            self.print_effect(*id, e);
            self.out.push('\n');
        }

        // Print handlers
        for (id, h) in &self.program.handlers {
            self.print_handler(*id, h);
            self.out.push('\n');
        }

        // Print structs
        for (id, s) in &self.program.structs {
            self.print_struct(*id, s);
            self.out.push('\n');
        }

        // Print enums
        for (id, e) in &self.program.enums {
            self.print_enum(*id, e);
            self.out.push('\n');
        }

        // Print functions
        for (id, f) in &self.program.functions {
            self.print_function_with_id(*id, f);
            self.out.push('\n');
        }

        // Print entry point
        if let Some(entry) = self.program.entry {
            let _ = writeln!(self.out, "// entry: fn#{}", entry.0);
        }
    }

    fn print_effect(&mut self, id: EffectId, effect: &MirEffect) {
        let type_params = if effect.type_params.is_empty() {
            String::new()
        } else {
            format!("[{}]", effect.type_params.join(", "))
        };
        let _ = writeln!(self.out, "effect {}{} {{ // {}", effect.name, type_params, id);
        self.indent();

        for op in &effect.operations {
            self.print_effect_operation(op);
        }

        self.dedent();
        self.writeln("}");
    }

    fn print_effect_operation(&mut self, op: &EffectOperation) {
        self.write_indent();
        let params: Vec<_> = op.params.iter().map(|t| format!("{}", t)).collect();
        let _ = writeln!(
            self.out,
            "{}: ({}) -> {}, // {}",
            op.name,
            params.join(", "),
            op.return_ty,
            op.id
        );
    }

    fn print_handler(&mut self, id: HandlerId, handler: &MirHandler) {
        let tail_marker = if handler.is_tail_resumptive { " [tail-resumptive]" } else { "" };
        let _ = writeln!(
            self.out,
            "handler for {}{} {{ // {}",
            handler.effect,
            tail_marker,
            id
        );
        self.indent();

        // Print operation blocks
        for (i, block_id) in handler.operation_blocks.iter().enumerate() {
            self.write_indent();
            let _ = writeln!(self.out, "op#{} -> {},", i, block_id);
        }

        // Print return block
        if let Some(ret_block) = handler.return_block {
            self.write_indent();
            let _ = writeln!(self.out, "return -> {},", ret_block);
        }

        self.dedent();
        self.writeln("}");
    }

    fn print_struct(&mut self, id: StructId, s: &MirStruct) {
        let _ = writeln!(self.out, "struct {} {{ // struct#{}", s.name, id.0);
        self.indent();
        for field in &s.fields {
            self.write_indent();
            let _ = writeln!(self.out, "{}: {},", field.name, field.ty);
        }
        self.dedent();
        self.writeln("}");
    }

    fn print_enum(&mut self, id: EnumId, e: &MirEnum) {
        let _ = writeln!(self.out, "enum {} {{ // enum#{}", e.name, id.0);
        self.indent();
        for (i, variant) in e.variants.iter().enumerate() {
            self.write_indent();
            if variant.fields.is_empty() {
                let _ = writeln!(self.out, "{}, // variant {}", variant.name, i);
            } else {
                let fields: Vec<_> = variant.fields.iter().map(|t| format!("{}", t)).collect();
                let _ = writeln!(
                    self.out,
                    "{}({}), // variant {}",
                    variant.name,
                    fields.join(", "),
                    i
                );
            }
        }
        self.dedent();
        self.writeln("}");
    }

    fn print_function(&mut self, f: &MirFunction) {
        self.print_function_with_id(FunctionId(0), f);
    }

    fn print_function_with_id(&mut self, id: FunctionId, f: &MirFunction) {
        // Function signature
        let params: Vec<_> = f
            .params
            .iter()
            .map(|p| format!("{}: {}", p, f.local_decl(*p).ty))
            .collect();

        let visibility = if f.is_public { "pub " } else { "" };

        // Effect row (if not pure)
        let effect_row_str = if f.effect_row.is_pure() {
            String::new()
        } else {
            format!(": {}", f.effect_row)
        };

        let _ = writeln!(
            self.out,
            "{}fn {}({}){} -> {} {{ // fn#{}",
            visibility,
            f.name,
            params.join(", "),
            effect_row_str,
            f.return_ty,
            id.0
        );

        self.indent();

        // Evidence parameters (if any)
        if !f.evidence_params.is_empty() {
            self.writeln("// evidence:");
            for ev in &f.evidence_params {
                self.write_indent();
                let static_marker = if ev.is_static { "static" } else { "dynamic" };
                let _ = writeln!(
                    self.out,
                    "// {}: {} ({})",
                    ev.local,
                    ev.effect,
                    static_marker
                );
            }
            self.out.push('\n');
        }

        // Evidence layout (if any)
        if f.evidence_layout.size > 0 {
            self.writeln("// evidence layout:");
            for (effect_id, slot) in &f.evidence_layout.slots {
                self.write_indent();
                let _ = writeln!(self.out, "// {} -> slot {}", effect_id, slot);
            }
            self.out.push('\n');
        }

        // Local declarations
        self.writeln("// locals:");
        for (i, local) in f.locals.iter().enumerate() {
            self.write_indent();
            let name = local.name.as_ref().map(|s| s.as_str()).unwrap_or("_tmp");
            let mutability = if local.mutable { "mut " } else { "" };
            let _ = writeln!(self.out, "// _{}: {}{} ({})", i, mutability, local.ty, name);
        }
        self.out.push('\n');

        // Handler blocks (if any)
        if !f.handler_blocks.is_empty() {
            self.writeln("// handler blocks:");
            for hb in &f.handler_blocks {
                self.print_handler_block(hb);
            }
            self.out.push('\n');
        }

        // Basic blocks
        for block in &f.blocks {
            self.print_block_with_effects(block, f);
        }

        self.dedent();
        self.writeln("}");
    }

    fn print_handler_block(&mut self, hb: &HandlerBlock) {
        self.write_indent();
        let params: Vec<_> = hb.params.iter().map(|p| format!("{}", p)).collect();
        let cont_str = hb.continuation.map(|c| format!(", cont: {}", c)).unwrap_or_default();
        let resume_str = hb.resume_block.map(|b| format!(" -> resume {}", b)).unwrap_or_default();
        let _ = writeln!(
            self.out,
            "// {}: @{}.{} [{}]{}{},",
            hb.block_id,
            hb.effect,
            hb.operation,
            params.join(", "),
            cont_str,
            resume_str
        );
    }

    fn print_block_with_effects(&mut self, block: &BasicBlock, func: &MirFunction) {
        let _ = writeln!(self.out, "{}:", block.id);
        self.indent();

        // Statements
        for (i, stmt) in block.statements.iter().enumerate() {
            // Check for effect statement
            if let Some(effect_stmt) = func.effect_statement(block.id, i) {
                self.print_effect_statement(effect_stmt);
            } else {
                self.print_statement(stmt);
            }
        }

        // Terminator
        // Check for effect terminator first
        if let Some(effect_term) = func.effect_terminator(block.id) {
            self.print_effect_terminator(effect_term);
        } else if let Some(term) = &block.terminator {
            self.print_terminator(term);
        } else {
            self.writeln("// <no terminator>");
        }

        self.dedent();
        self.out.push('\n');
    }

    fn print_effect_statement(&mut self, stmt: &EffectStatementKind) {
        self.write_indent();
        let _ = writeln!(self.out, "{};", stmt);
    }

    fn print_effect_terminator(&mut self, term: &EffectTerminatorKind) {
        self.write_indent();
        let _ = writeln!(self.out, "{};", term);
    }

    #[allow(dead_code)]
    fn print_block(&mut self, block: &BasicBlock) {
        let _ = writeln!(self.out, "{}:", block.id);
        self.indent();

        // Statements
        for stmt in &block.statements {
            self.print_statement(stmt);
        }

        // Terminator
        if let Some(term) = &block.terminator {
            self.print_terminator(term);
        } else {
            self.writeln("// <no terminator>");
        }

        self.dedent();
        self.out.push('\n');
    }

    fn print_statement(&mut self, stmt: &Statement) {
        self.write_indent();
        match &stmt.kind {
            StatementKind::Assign(place, rvalue) => {
                let _ = writeln!(self.out, "{} = {};", place, self.format_rvalue(rvalue));
            }
            StatementKind::StorageLive(local) => {
                let _ = writeln!(self.out, "StorageLive({});", local);
            }
            StatementKind::StorageDead(local) => {
                let _ = writeln!(self.out, "StorageDead({});", local);
            }
            StatementKind::Nop => {
                let _ = writeln!(self.out, "nop;");
            }
        }
    }

    fn print_terminator(&mut self, term: &Terminator) {
        self.write_indent();
        match &term.kind {
            TerminatorKind::Goto { target } => {
                let _ = writeln!(self.out, "goto -> {};", target);
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                let _ = write!(self.out, "switchInt({}) -> [", discr);
                for (val, target) in &targets.targets {
                    let _ = write!(self.out, "{}: {}, ", val, target);
                }
                let _ = writeln!(self.out, "otherwise: {}];", targets.otherwise);
            }
            TerminatorKind::Call {
                func,
                args,
                dest,
                target,
            } => {
                let args_str: Vec<_> = args.iter().map(|a| format!("{}", a)).collect();
                let target_str = target
                    .map(|t| format!(" -> {}", t))
                    .unwrap_or_else(|| " -> diverge".to_string());
                let _ = writeln!(
                    self.out,
                    "{} = {}({}){};",
                    dest,
                    func,
                    args_str.join(", "),
                    target_str
                );
            }
            TerminatorKind::Return => {
                let _ = writeln!(self.out, "return;");
            }
            TerminatorKind::Unreachable => {
                let _ = writeln!(self.out, "unreachable;");
            }
            TerminatorKind::Drop { place, target } => {
                let _ = writeln!(self.out, "drop({}) -> {};", place, target);
            }
            TerminatorKind::Assert {
                cond,
                expected,
                msg,
                target,
            } => {
                let _ = writeln!(
                    self.out,
                    "assert({}, {}, \"{}\") -> {};",
                    cond, expected, msg, target
                );
            }
        }
    }

    fn format_rvalue(&self, rvalue: &Rvalue) -> String {
        match rvalue {
            Rvalue::Use(op) => format!("{}", op),
            Rvalue::BinaryOp(op, left, right) => {
                format!("{} {} {}", left, op, right)
            }
            Rvalue::UnaryOp(op, operand) => {
                format!("{}{}", op, operand)
            }
            Rvalue::Ref(place) => format!("&{}", place),
            Rvalue::RefMut(place) => format!("&mut {}", place),
            Rvalue::Aggregate(kind, ops) => {
                let ops_str: Vec<_> = ops.iter().map(|o| format!("{}", o)).collect();
                match kind {
                    AggregateKind::Tuple => format!("({})", ops_str.join(", ")),
                    AggregateKind::Array(ty) => format!("[{}; {}]", ops_str.join(", "), ty),
                    AggregateKind::Struct(id) => {
                        format!("struct#{} {{ {} }}", id.0, ops_str.join(", "))
                    }
                    AggregateKind::Enum(id, variant) => {
                        format!("enum#{}::variant#{} {{ {} }}", id.0, variant, ops_str.join(", "))
                    }
                }
            }
            Rvalue::Discriminant(place) => format!("discriminant({})", place),
            Rvalue::Len(place) => format!("len({})", place),
            Rvalue::Cast(kind, op, ty) => format!("{} as {} ({:?})", op, ty, kind),
            Rvalue::Closure(fn_id, captures) => {
                let caps: Vec<_> = captures.iter().map(|c| format!("{}", c)).collect();
                format!("closure(fn#{}, [{}])", fn_id.0, caps.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aria_lexer::Span;

    #[test]
    fn test_pretty_print_empty_program() {
        let program = MirProgram::new();
        let output = pretty_print(&program);
        assert!(output.is_empty() || output.trim().is_empty());
    }

    #[test]
    fn test_pretty_print_effect_definition() {
        let mut program = MirProgram::new();

        // Create State effect with get/set operations
        let effect_id = program.new_effect("State".into(), Span::dummy());
        {
            let effect = program.effect_mut(effect_id).unwrap();
            effect.type_params.push("T".into());
            effect.add_operation(EffectOperation {
                id: OperationId(0),
                name: "get".into(),
                params: vec![],
                return_ty: MirType::Int,
            });
            effect.add_operation(EffectOperation {
                id: OperationId(1),
                name: "set".into(),
                params: vec![MirType::Int],
                return_ty: MirType::Unit,
            });
        }

        let output = pretty_print(&program);
        assert!(output.contains("effect State[T]"));
        assert!(output.contains("get:"));
        assert!(output.contains("set:"));
    }

    #[test]
    fn test_pretty_print_handler() {
        let mut program = MirProgram::new();

        let effect_type = EffectType::new(EffectId(0), "State".into());
        let handler_id = program.new_handler(effect_type, Span::dummy());
        {
            let handler = program.handler_mut(handler_id).unwrap();
            handler.operation_blocks.push(BlockId(1));
            handler.operation_blocks.push(BlockId(2));
            handler.return_block = Some(BlockId(3));
        }

        let output = pretty_print(&program);
        assert!(output.contains("handler for State"));
        assert!(output.contains("[tail-resumptive]"));
        assert!(output.contains("op#0 -> bb1"));
        assert!(output.contains("op#1 -> bb2"));
        assert!(output.contains("return -> bb3"));
    }

    #[test]
    fn test_pretty_print_effectful_function() {
        let mut program = MirProgram::new();

        let mut func = MirFunction::new("counter".into(), MirType::Int, Span::dummy());
        func.is_public = true;

        // Set effect row
        let state_effect = EffectType::new(EffectId(0), "State".into())
            .with_type_params(vec![MirType::Int]);
        func.set_effect_row(EffectRow::new().with_effect(state_effect.clone()));

        // Add evidence parameter
        func.add_evidence_param(state_effect.clone(), true);

        // Add parameter
        let n = func.new_local(MirType::Int, Some("n".into()));
        func.params = vec![n];

        // Create entry block
        let entry = func.new_block();

        // Add effect statement
        let result = func.new_local(MirType::Int, Some("result".into()));
        func.block_mut(entry).push_stmt(Statement {
            kind: StatementKind::Nop, // Placeholder
            span: Span::dummy(),
        });

        let effect_stmt = EffectStatementKind::PerformEffect {
            effect: state_effect.clone(),
            operation: OperationId(0),
            args: vec![],
            evidence_slot: EvidenceSlot::Static(0),
            dest: Place::from_local(result),
            classification: EffectClassification::TailResumptive,
        };
        func.add_effect_statement(entry, 0, effect_stmt);

        func.block_mut(entry).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        let fn_id = FunctionId(0);
        program.functions.insert(fn_id, func);

        let output = pretty_print(&program);
        assert!(output.contains("fn counter"));
        assert!(output.contains("{State[Int]}"));
        assert!(output.contains("// evidence:"));
        assert!(output.contains("effect.perform.tail-resumptive"));
    }

    #[test]
    fn test_pretty_print_effect_terminator() {
        let mut program = MirProgram::new();

        let mut func = MirFunction::new("test_resume".into(), MirType::Int, Span::dummy());

        let block1 = func.new_block();
        let block2 = func.new_block();

        // Add effect terminator on block1
        let effect_term = EffectTerminatorKind::Resume {
            continuation: Operand::Copy(Place::from_local(Local(1))),
            value: Operand::const_int(42),
            target: block2,
        };
        func.set_effect_terminator(block1, effect_term);

        func.block_mut(block2).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        let fn_id = FunctionId(0);
        program.functions.insert(fn_id, func);

        let output = pretty_print(&program);
        assert!(output.contains("effect.resume"));
        assert!(output.contains("42"));
    }

    #[test]
    fn test_pretty_print_handler_block() {
        let mut program = MirProgram::new();

        let mut func = MirFunction::new("with_handler".into(), MirType::Int, Span::dummy());

        let entry = func.new_block();
        func.block_mut(entry).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        // Add handler block
        let hb = HandlerBlock {
            block_id: BlockId(1),
            effect: EffectType::new(EffectId(0), "State".into()),
            operation: OperationId(0),
            params: vec![Local(2)],
            continuation: Some(Local(3)),
            resume_block: Some(BlockId(2)),
        };
        func.add_handler_block(hb);

        let fn_id = FunctionId(0);
        program.functions.insert(fn_id, func);

        let output = pretty_print(&program);
        assert!(output.contains("// handler blocks:"));
        assert!(output.contains("@State.op#0"));
        assert!(output.contains("cont:"));
        assert!(output.contains("resume bb2"));
    }

    #[test]
    fn test_pretty_print_simple_function() {
        let mut program = MirProgram::new();

        let mut func = MirFunction::new("add".into(), MirType::Int, Span::dummy());
        func.is_public = true;

        // Add parameters
        let a = func.new_local(MirType::Int, Some("a".into()));
        let b = func.new_local(MirType::Int, Some("b".into()));
        func.params = vec![a, b];

        // Create entry block
        let entry = func.new_block();

        // Create result temp
        let result = func.new_local(MirType::Int, Some("result".into()));

        // Add statements
        func.block_mut(entry).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::from_local(result),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Copy(Place::from_local(a)),
                    Operand::Copy(Place::from_local(b)),
                ),
            ),
            span: Span::dummy(),
        });

        // Assign to return place
        func.block_mut(entry).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::return_place(),
                Rvalue::Use(Operand::Copy(Place::from_local(result))),
            ),
            span: Span::dummy(),
        });

        // Return
        func.block_mut(entry).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        let fn_id = FunctionId(0);
        program.functions.insert(fn_id, func);
        program.entry = Some(fn_id);

        let output = pretty_print(&program);
        assert!(output.contains("fn add"));
        assert!(output.contains("return"));
    }

    #[test]
    fn test_pretty_print_control_flow() {
        let mut program = MirProgram::new();

        let mut func = MirFunction::new("test_if".into(), MirType::Int, Span::dummy());

        let entry = func.new_block();
        let then_block = func.new_block();
        let else_block = func.new_block();
        let merge = func.new_block();

        let cond = func.new_local(MirType::Bool, Some("cond".into()));

        // Entry: switch on condition
        func.block_mut(entry).set_terminator(Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: Operand::Copy(Place::from_local(cond)),
                targets: SwitchTargets::if_else(then_block, else_block),
            },
            span: Span::dummy(),
        });

        // Then block
        func.block_mut(then_block).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::return_place(),
                Rvalue::Use(Operand::const_int(1)),
            ),
            span: Span::dummy(),
        });
        func.block_mut(then_block).set_terminator(Terminator {
            kind: TerminatorKind::Goto { target: merge },
            span: Span::dummy(),
        });

        // Else block
        func.block_mut(else_block).push_stmt(Statement {
            kind: StatementKind::Assign(
                Place::return_place(),
                Rvalue::Use(Operand::const_int(0)),
            ),
            span: Span::dummy(),
        });
        func.block_mut(else_block).set_terminator(Terminator {
            kind: TerminatorKind::Goto { target: merge },
            span: Span::dummy(),
        });

        // Merge block
        func.block_mut(merge).set_terminator(Terminator {
            kind: TerminatorKind::Return,
            span: Span::dummy(),
        });

        let fn_id = FunctionId(0);
        program.functions.insert(fn_id, func);

        let output = pretty_print(&program);
        assert!(output.contains("switchInt"));
        assert!(output.contains("goto"));
        assert!(output.contains("bb0"));
        assert!(output.contains("bb1"));
        assert!(output.contains("bb2"));
        assert!(output.contains("bb3"));
    }
}
