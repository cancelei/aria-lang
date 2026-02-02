//! Expression evaluation and statement execution for the Aria interpreter.

use std::cell::RefCell;
use std::rc::Rc;

use aria_ast::{
    BinaryOp, Expr, ExprKind, Item, Program, Stmt, StmtKind, UnaryOp,
};
use aria_lexer::Span;
use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::builtins;
use crate::environment::Environment;
use crate::value::{AriaFunction, Value};
use crate::{Result, RuntimeError};

/// The Aria interpreter - evaluates AST nodes.
pub struct Interpreter {
    /// Global environment
    pub globals: Rc<RefCell<Environment>>,

    /// Current environment (changes with scope)
    pub environment: Rc<RefCell<Environment>>,

    /// Whether we're inside a loop (for break/continue validation)
    loop_depth: usize,

    /// Whether we're inside a function (for return validation)
    function_depth: usize,
}

impl Interpreter {
    /// Create a new interpreter with built-in functions.
    pub fn new() -> Self {
        let globals = Rc::new(RefCell::new(Environment::new()));

        // Register built-in functions
        builtins::register(&mut globals.borrow_mut());

        Interpreter {
            globals: globals.clone(),
            environment: globals,
            loop_depth: 0,
            function_depth: 0,
        }
    }

    /// Execute a program.
    pub fn run(&mut self, program: &Program) -> Result<Value> {
        // First pass: collect all function and struct definitions
        for item in &program.items {
            self.define_item(item)?;
        }

        // Second pass: if there's a main function, call it
        let main_fn = self.globals.borrow().get(&SmolStr::new("main"));
        if let Some(main_fn) = main_fn {
            self.call_value(main_fn, vec![], aria_lexer::Span::default())
        } else {
            // No main function - return success (useful for REPL/module loading)
            Ok(Value::Nil)
        }
    }

    /// Define a top-level item (function, struct, etc.)
    fn define_item(&mut self, item: &Item) -> Result<()> {
        match item {
            Item::Function(func) => {
                let name = func.name.node.clone();
                let params: Vec<SmolStr> = func
                    .params
                    .iter()
                    .map(|p| p.name.node.clone())
                    .collect();

                let aria_fn = AriaFunction {
                    name: name.clone(),
                    params,
                    body: func.body.clone(),
                    closure: self.environment.clone(),
                };

                self.globals
                    .borrow_mut()
                    .define(name, Value::Function(Rc::new(aria_fn)));
                Ok(())
            }
            // TODO: Implement other items (structs, enums, etc.)
            _ => Ok(()),
        }
    }

    /// Evaluate an expression to a value.
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match &expr.kind {
            // Literals
            ExprKind::Integer(s) => {
                let n = s.parse::<i64>().map_err(|_| RuntimeError::General {
                    message: format!("invalid integer: {}", s),
                    span: expr.span,
                })?;
                Ok(Value::Int(n))
            }
            ExprKind::Float(s) => {
                let n = s.parse::<f64>().map_err(|_| RuntimeError::General {
                    message: format!("invalid float: {}", s),
                    span: expr.span,
                })?;
                Ok(Value::Float(n))
            }
            ExprKind::String(s) => Ok(Value::String(s.clone())),
            ExprKind::Char(s) => Ok(Value::String(s.clone())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Nil => Ok(Value::Nil),

            ExprKind::Ident(name) => {
                self.environment
                    .borrow()
                    .get(name)
                    .ok_or_else(|| RuntimeError::UndefinedVariable {
                        name: name.clone(),
                        span: expr.span,
                    })
            }

            ExprKind::Binary { op, left, right } => {
                self.eval_binary(*op, left, right, expr.span)
            }

            ExprKind::Unary { op, operand } => {
                self.eval_unary(*op, operand, expr.span)
            }

            ExprKind::Call { func, args } => {
                self.eval_call(func, args, expr.span)
            }

            ExprKind::Array(elements) => {
                let values: Result<Vec<_>> =
                    elements.iter().map(|e| self.eval_expr(e)).collect();
                Ok(Value::Array(Rc::new(RefCell::new(values?))))
            }

            ExprKind::Tuple(elements) => {
                let values: Result<Vec<_>> =
                    elements.iter().map(|e| self.eval_expr(e)).collect();
                Ok(Value::Tuple(Rc::new(values?)))
            }

            ExprKind::Map(entries) => {
                let mut map = IndexMap::new();
                for (key, value) in entries {
                    let key_val = self.eval_expr(key)?;
                    let key_str = match key_val {
                        Value::String(s) => s,
                        _ => {
                            return Err(RuntimeError::TypeError {
                                message: format!(
                                    "map key must be a string, got {}",
                                    key_val.type_name()
                                ),
                                span: key.span,
                            });
                        }
                    };
                    let val = self.eval_expr(value)?;
                    map.insert(key_str, val);
                }
                Ok(Value::Map(Rc::new(RefCell::new(map))))
            }

            ExprKind::Index { object, index } => {
                let obj_val = self.eval_expr(object)?;
                let idx_val = self.eval_expr(index)?;
                self.eval_index(obj_val, idx_val, expr.span)
            }

            ExprKind::Field { object, field } => {
                let obj_val = self.eval_expr(object)?;
                self.eval_field(obj_val, &field.node, expr.span)
            }

            ExprKind::MethodCall { object, method, args } => {
                let obj_val = self.eval_expr(object)?;
                self.eval_method_call(obj_val, &method.node, args, expr.span)
            }

            ExprKind::If { condition, then_branch, elsif_branches, else_branch } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    self.eval_block(then_branch)
                } else {
                    // Try elsif branches
                    for (elsif_cond, elsif_body) in elsif_branches {
                        let cond = self.eval_expr(elsif_cond)?;
                        if cond.is_truthy() {
                            return self.eval_block(elsif_body);
                        }
                    }
                    // Fall through to else
                    if let Some(else_branch) = else_branch {
                        self.eval_block(else_branch)
                    } else {
                        Ok(Value::Nil)
                    }
                }
            }

            ExprKind::Ternary { condition, then_expr, else_expr } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    self.eval_expr(then_expr)
                } else {
                    self.eval_expr(else_expr)
                }
            }

            ExprKind::Lambda { params, body, .. } => {
                let param_names: Vec<SmolStr> =
                    params.iter().map(|p| p.name.node.clone()).collect();

                let aria_fn = AriaFunction {
                    name: "<lambda>".into(),
                    params: param_names,
                    body: aria_ast::FunctionBody::Expression(body.clone()),
                    closure: self.environment.clone(),
                };

                Ok(Value::Function(Rc::new(aria_fn)))
            }

            ExprKind::BlockLambda { params, body, .. } => {
                let param_names: Vec<SmolStr> =
                    params.iter().map(|p| p.name.node.clone()).collect();

                let aria_fn = AriaFunction {
                    name: "<lambda>".into(),
                    params: param_names,
                    body: aria_ast::FunctionBody::Block(body.clone()),
                    closure: self.environment.clone(),
                };

                Ok(Value::Function(Rc::new(aria_fn)))
            }

            ExprKind::Range { start, end, inclusive } => {
                let start_val = match start {
                    Some(e) => self.eval_expr(e)?.as_int().ok_or_else(|| {
                        RuntimeError::TypeError {
                            message: "range start must be an integer".into(),
                            span: e.span,
                        }
                    })?,
                    None => 0,
                };

                let end_val = match end {
                    Some(e) => self.eval_expr(e)?.as_int().ok_or_else(|| {
                        RuntimeError::TypeError {
                            message: "range end must be an integer".into(),
                            span: e.span,
                        }
                    })?,
                    None => i64::MAX,
                };

                Ok(Value::Range {
                    start: start_val,
                    end: end_val,
                    inclusive: *inclusive,
                })
            }

            ExprKind::Pipe { left, right } => {
                // Evaluate left, then pass result as first argument to right
                let left_val = self.eval_expr(left)?;

                // Right should be a call - inject left_val as first arg
                match &right.kind {
                    ExprKind::Call { func, args } => {
                        let mut new_args = vec![left_val];
                        for arg in args {
                            new_args.push(self.eval_expr(&arg.value)?);
                        }
                        let func_val = self.eval_expr(func)?;
                        self.call_value(func_val, new_args, expr.span)
                    }
                    ExprKind::Ident(name) => {
                        // Treat as a function call with left_val as the only arg
                        let func_val = self.environment
                            .borrow()
                            .get(name)
                            .ok_or_else(|| RuntimeError::UndefinedVariable {
                                name: name.clone(),
                                span: right.span,
                            })?;
                        self.call_value(func_val, vec![left_val], expr.span)
                    }
                    _ => Err(RuntimeError::TypeError {
                        message: "pipe right-hand side must be a function call".into(),
                        span: right.span,
                    }),
                }
            }

            ExprKind::Try(inner) => {
                // For now, Try just evaluates the inner expression
                // Full error handling would need Result<T, E> types
                self.eval_expr(inner)
            }

            ExprKind::Paren(inner) => {
                // Parenthesized expression - just evaluate the inner expression
                self.eval_expr(inner)
            }

            // TODO: Implement remaining expression kinds
            _ => Err(RuntimeError::General {
                message: format!("unimplemented expression kind: {:?}", expr.kind),
                span: expr.span,
            }),
        }
    }

    /// Evaluate a binary operation.
    fn eval_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: Span,
    ) -> Result<Value> {
        let lhs = self.eval_expr(left)?;

        // Short-circuit for logical operators
        match op {
            BinaryOp::And => {
                if !lhs.is_truthy() {
                    return Ok(Value::Bool(false));
                }
                let rhs = self.eval_expr(right)?;
                return Ok(Value::Bool(rhs.is_truthy()));
            }
            BinaryOp::Or => {
                if lhs.is_truthy() {
                    return Ok(Value::Bool(true));
                }
                let rhs = self.eval_expr(right)?;
                return Ok(Value::Bool(rhs.is_truthy()));
            }
            _ => {}
        }

        let rhs = self.eval_expr(right)?;

        match op {
            // Arithmetic
            BinaryOp::Add => self.eval_add(lhs, rhs, span),
            BinaryOp::Sub => self.eval_arithmetic(lhs, rhs, span, |a, b| a - b, |a, b| a - b),
            BinaryOp::Mul => self.eval_arithmetic(lhs, rhs, span, |a, b| a * b, |a, b| a * b),
            BinaryOp::Div => {
                // Check for division by zero
                let is_zero = match &rhs {
                    Value::Int(0) => true,
                    Value::Float(f) if *f == 0.0 => true,
                    _ => false,
                };
                if is_zero {
                    return Err(RuntimeError::DivisionByZero { span });
                }
                self.eval_arithmetic(lhs, rhs, span, |a, b| a / b, |a, b| a / b)
            }
            BinaryOp::IntDiv => {
                if let Value::Int(0) = rhs {
                    return Err(RuntimeError::DivisionByZero { span });
                }
                match (lhs, rhs) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
                    _ => Err(RuntimeError::TypeError {
                        message: "integer division requires integers".into(),
                        span,
                    }),
                }
            }
            BinaryOp::Mod => self.eval_arithmetic(lhs, rhs, span, |a, b| a % b, |a, b| a % b),
            BinaryOp::Pow => self.eval_power(lhs, rhs, span),

            // Comparison
            BinaryOp::Eq => Ok(Value::Bool(lhs == rhs)),
            BinaryOp::NotEq => Ok(Value::Bool(lhs != rhs)),
            BinaryOp::Lt => self.eval_comparison(lhs, rhs, span, |a, b| a < b, |a, b| a < b),
            BinaryOp::LtEq => self.eval_comparison(lhs, rhs, span, |a, b| a <= b, |a, b| a <= b),
            BinaryOp::Gt => self.eval_comparison(lhs, rhs, span, |a, b| a > b, |a, b| a > b),
            BinaryOp::GtEq => self.eval_comparison(lhs, rhs, span, |a, b| a >= b, |a, b| a >= b),

            // Bitwise
            BinaryOp::BitAnd => self.eval_bitwise(lhs, rhs, span, |a, b| a & b),
            BinaryOp::BitOr => self.eval_bitwise(lhs, rhs, span, |a, b| a | b),
            BinaryOp::BitXor => self.eval_bitwise(lhs, rhs, span, |a, b| a ^ b),
            BinaryOp::Shl => self.eval_bitwise(lhs, rhs, span, |a, b| a << b),
            BinaryOp::Shr => self.eval_bitwise(lhs, rhs, span, |a, b| a >> b),

            // Already handled above
            BinaryOp::And | BinaryOp::Or => unreachable!(),

            // Range (inclusive ..=)
            BinaryOp::Range => {
                let start = lhs.as_int().ok_or_else(|| RuntimeError::TypeError {
                    message: "range start must be an integer".into(),
                    span,
                })?;
                let end = rhs.as_int().ok_or_else(|| RuntimeError::TypeError {
                    message: "range end must be an integer".into(),
                    span,
                })?;
                Ok(Value::Range { start, end, inclusive: true })
            }
            // Range exclusive (..)
            BinaryOp::RangeExclusive => {
                let start = lhs.as_int().ok_or_else(|| RuntimeError::TypeError {
                    message: "range start must be an integer".into(),
                    span,
                })?;
                let end = rhs.as_int().ok_or_else(|| RuntimeError::TypeError {
                    message: "range end must be an integer".into(),
                    span,
                })?;
                Ok(Value::Range { start, end, inclusive: false })
            }

            _ => Err(RuntimeError::General {
                message: format!("unimplemented binary operator: {:?}", op),
                span,
            }),
        }
    }

    fn eval_add(&self, lhs: Value, rhs: Value, span: Span) -> Result<Value> {
        match (lhs, rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
            (Value::String(a), Value::String(b)) => {
                Ok(Value::String(format!("{}{}", a, b).into()))
            }
            (Value::String(a), b) => {
                Ok(Value::String(format!("{}{}", a, b).into()))
            }
            (a, Value::String(b)) => {
                Ok(Value::String(format!("{}{}", a, b).into()))
            }
            (a, b) => Err(RuntimeError::TypeError {
                message: format!("cannot add {} and {}", a.type_name(), b.type_name()),
                span,
            }),
        }
    }

    fn eval_arithmetic<F, G>(
        &self,
        lhs: Value,
        rhs: Value,
        span: Span,
        int_op: F,
        float_op: G,
    ) -> Result<Value>
    where
        F: Fn(i64, i64) -> i64,
        G: Fn(f64, f64) -> f64,
    {
        match (lhs, rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(a, b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(a as f64, b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(a, b as f64))),
            (a, b) => Err(RuntimeError::TypeError {
                message: format!(
                    "cannot perform arithmetic on {} and {}",
                    a.type_name(),
                    b.type_name()
                ),
                span,
            }),
        }
    }

    fn eval_power(&self, lhs: Value, rhs: Value, span: Span) -> Result<Value> {
        match (lhs, rhs) {
            (Value::Int(base), Value::Int(exp)) if exp >= 0 => {
                Ok(Value::Int(base.pow(exp as u32)))
            }
            (Value::Int(base), Value::Int(exp)) => {
                Ok(Value::Float((base as f64).powi(exp as i32)))
            }
            (Value::Float(base), Value::Int(exp)) => {
                Ok(Value::Float(base.powi(exp as i32)))
            }
            (Value::Float(base), Value::Float(exp)) => {
                Ok(Value::Float(base.powf(exp)))
            }
            (Value::Int(base), Value::Float(exp)) => {
                Ok(Value::Float((base as f64).powf(exp)))
            }
            (a, b) => Err(RuntimeError::TypeError {
                message: format!("cannot raise {} to power of {}", a.type_name(), b.type_name()),
                span,
            }),
        }
    }

    fn eval_comparison<F, G>(
        &self,
        lhs: Value,
        rhs: Value,
        span: Span,
        int_op: F,
        float_op: G,
    ) -> Result<Value>
    where
        F: Fn(i64, i64) -> bool,
        G: Fn(f64, f64) -> bool,
    {
        match (lhs, rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(int_op(a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(float_op(a, b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Bool(float_op(a as f64, b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(float_op(a, b as f64))),
            (Value::String(a), Value::String(b)) => Ok(Value::Bool(int_op(
                a.cmp(&b) as i64,
                0,
            ))),
            (a, b) => Err(RuntimeError::TypeError {
                message: format!("cannot compare {} and {}", a.type_name(), b.type_name()),
                span,
            }),
        }
    }

    fn eval_bitwise<F>(&self, lhs: Value, rhs: Value, span: Span, op: F) -> Result<Value>
    where
        F: Fn(i64, i64) -> i64,
    {
        match (lhs, rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(op(a, b))),
            (a, b) => Err(RuntimeError::TypeError {
                message: format!(
                    "bitwise operations require integers, got {} and {}",
                    a.type_name(),
                    b.type_name()
                ),
                span,
            }),
        }
    }

    /// Evaluate a unary operation.
    fn eval_unary(&mut self, op: UnaryOp, operand: &Expr, span: Span) -> Result<Value> {
        let val = self.eval_expr(operand)?;

        match op {
            UnaryOp::Neg => match val {
                Value::Int(n) => Ok(Value::Int(-n)),
                Value::Float(n) => Ok(Value::Float(-n)),
                _ => Err(RuntimeError::TypeError {
                    message: format!("cannot negate {}", val.type_name()),
                    span,
                }),
            },
            UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
            UnaryOp::BitNot => match val {
                Value::Int(n) => Ok(Value::Int(!n)),
                _ => Err(RuntimeError::TypeError {
                    message: format!("bitwise not requires integer, got {}", val.type_name()),
                    span,
                }),
            },
            UnaryOp::Ref | UnaryOp::Deref => {
                // References are mostly for type checking; in the interpreter
                // we can just return the value
                Ok(val)
            }
        }
    }

    /// Evaluate a function call.
    fn eval_call(&mut self, func: &Expr, args: &[aria_ast::CallArg], span: Span) -> Result<Value> {
        let func_val = self.eval_expr(func)?;
        let arg_vals: Result<Vec<_>> = args.iter().map(|a| self.eval_expr(&a.value)).collect();
        self.call_value(func_val, arg_vals?, span)
    }

    /// Call a value as a function.
    fn call_value(&mut self, func: Value, args: Vec<Value>, span: Span) -> Result<Value> {
        match func {
            Value::Function(f) => {
                if f.params.len() != args.len() {
                    return Err(RuntimeError::ArityMismatch {
                        expected: f.params.len(),
                        got: args.len(),
                        span,
                    });
                }

                // Create new environment with closure as parent
                let env = Rc::new(RefCell::new(Environment::with_parent(f.closure.clone())));

                // Bind parameters
                for (param, arg) in f.params.iter().zip(args) {
                    env.borrow_mut().define(param.clone(), arg);
                }

                // Save current environment and enter function
                let previous = self.environment.clone();
                self.environment = env;
                self.function_depth += 1;

                // Execute function body
                let result = match &f.body {
                    aria_ast::FunctionBody::Block(block) => self.eval_block(block),
                    aria_ast::FunctionBody::Expression(expr) => self.eval_expr(expr),
                };

                // Restore environment
                self.environment = previous;
                self.function_depth -= 1;

                // Handle return values and control flow
                match result {
                    Ok(val) => Ok(val),
                    Err(RuntimeError::General { message, .. })
                        if message.starts_with("RETURN:") =>
                    {
                        // This is a hack - proper implementation would use ControlFlow enum
                        Ok(Value::Nil)
                    }
                    Err(e) => Err(e),
                }
            }
            Value::BuiltinFunction(f) => {
                if let Some(arity) = f.arity {
                    if args.len() != arity {
                        return Err(RuntimeError::ArityMismatch {
                            expected: arity,
                            got: args.len(),
                            span,
                        });
                    }
                }
                (f.func)(args)
            }
            _ => Err(RuntimeError::NotCallable {
                value_type: func.type_name().into(),
                span,
            }),
        }
    }

    /// Evaluate array/map indexing.
    fn eval_index(&self, obj: Value, index: Value, span: Span) -> Result<Value> {
        match obj {
            Value::Array(arr) => {
                let idx = index.as_int().ok_or_else(|| RuntimeError::TypeError {
                    message: "array index must be an integer".into(),
                    span,
                })?;

                let arr = arr.borrow();
                let len = arr.len();

                // Support negative indexing
                let actual_idx = if idx < 0 {
                    (len as i64 + idx) as usize
                } else {
                    idx as usize
                };

                arr.get(actual_idx).cloned().ok_or_else(|| {
                    RuntimeError::IndexOutOfBounds {
                        index: idx,
                        length: len,
                        span,
                    }
                })
            }
            Value::Map(map) => {
                let key = match index {
                    Value::String(s) => s,
                    _ => {
                        return Err(RuntimeError::TypeError {
                            message: "map key must be a string".into(),
                            span,
                        });
                    }
                };

                map.borrow().get(&key).cloned().ok_or_else(|| {
                    RuntimeError::InvalidKey {
                        key: key.to_string(),
                        span,
                    }
                })
            }
            Value::Tuple(t) => {
                let idx = index.as_int().ok_or_else(|| RuntimeError::TypeError {
                    message: "tuple index must be an integer".into(),
                    span,
                })?;

                t.get(idx as usize).cloned().ok_or_else(|| {
                    RuntimeError::IndexOutOfBounds {
                        index: idx,
                        length: t.len(),
                        span,
                    }
                })
            }
            Value::String(s) => {
                let idx = index.as_int().ok_or_else(|| RuntimeError::TypeError {
                    message: "string index must be an integer".into(),
                    span,
                })?;

                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();

                let actual_idx = if idx < 0 {
                    (len as i64 + idx) as usize
                } else {
                    idx as usize
                };

                chars.get(actual_idx).map(|c| Value::String(c.to_string().into())).ok_or_else(|| {
                    RuntimeError::IndexOutOfBounds {
                        index: idx,
                        length: len,
                        span,
                    }
                })
            }
            _ => Err(RuntimeError::TypeError {
                message: format!("{} is not indexable", obj.type_name()),
                span,
            }),
        }
    }

    /// Evaluate field access.
    fn eval_field(&self, obj: Value, field: &SmolStr, span: Span) -> Result<Value> {
        match obj {
            Value::Struct(s) => {
                let s = s.borrow();
                s.fields.get(field).cloned().ok_or_else(|| {
                    RuntimeError::UndefinedField {
                        struct_name: s.name.clone(),
                        field: field.clone(),
                        span,
                    }
                })
            }
            Value::Tuple(t) => {
                // Allow tuple.0, tuple.1, etc.
                let idx: usize = field.parse().map_err(|_| RuntimeError::TypeError {
                    message: format!("invalid tuple field: {}", field),
                    span,
                })?;

                t.get(idx).cloned().ok_or_else(|| RuntimeError::IndexOutOfBounds {
                    index: idx as i64,
                    length: t.len(),
                    span,
                })
            }
            _ => Err(RuntimeError::TypeError {
                message: format!("{} has no field {}", obj.type_name(), field),
                span,
            }),
        }
    }

    /// Evaluate a method call.
    fn eval_method_call(
        &mut self,
        obj: Value,
        method: &SmolStr,
        args: &[Expr],
        span: Span,
    ) -> Result<Value> {
        let arg_vals: Result<Vec<_>> = args.iter().map(|a| self.eval_expr(a)).collect();
        let arg_vals = arg_vals?;

        // Built-in methods on collection types
        match &obj {
            Value::Array(arr) => match method.as_str() {
                "len" => return Ok(Value::Int(arr.borrow().len() as i64)),
                "push" => {
                    if arg_vals.len() != 1 {
                        return Err(RuntimeError::ArityMismatch {
                            expected: 1,
                            got: arg_vals.len(),
                            span,
                        });
                    }
                    arr.borrow_mut().push(arg_vals.into_iter().next().unwrap());
                    return Ok(Value::Nil);
                }
                "pop" => {
                    return arr.borrow_mut().pop().ok_or_else(|| RuntimeError::General {
                        message: "pop from empty array".into(),
                        span,
                    });
                }
                _ => {}
            },
            Value::String(s) => match method.as_str() {
                "len" => return Ok(Value::Int(s.len() as i64)),
                "to_uppercase" => return Ok(Value::String(s.to_uppercase().into())),
                "to_lowercase" => return Ok(Value::String(s.to_lowercase().into())),
                "trim" => return Ok(Value::String(s.trim().into())),
                "split" => {
                    if arg_vals.len() != 1 {
                        return Err(RuntimeError::ArityMismatch {
                            expected: 1,
                            got: arg_vals.len(),
                            span,
                        });
                    }
                    let sep = match &arg_vals[0] {
                        Value::String(sep) => sep.as_str(),
                        _ => {
                            return Err(RuntimeError::TypeError {
                                message: "split separator must be a string".into(),
                                span,
                            });
                        }
                    };
                    let parts: Vec<Value> = s
                        .split(sep)
                        .map(|p| Value::String(p.into()))
                        .collect();
                    return Ok(Value::Array(Rc::new(RefCell::new(parts))));
                }
                _ => {}
            },
            Value::Map(map) => match method.as_str() {
                "len" => return Ok(Value::Int(map.borrow().len() as i64)),
                "keys" => {
                    let keys: Vec<Value> = map
                        .borrow()
                        .keys()
                        .map(|k| Value::String(k.clone()))
                        .collect();
                    return Ok(Value::Array(Rc::new(RefCell::new(keys))));
                }
                "values" => {
                    let values: Vec<Value> = map.borrow().values().cloned().collect();
                    return Ok(Value::Array(Rc::new(RefCell::new(values))));
                }
                "contains" => {
                    if arg_vals.len() != 1 {
                        return Err(RuntimeError::ArityMismatch {
                            expected: 1,
                            got: arg_vals.len(),
                            span,
                        });
                    }
                    let key = match &arg_vals[0] {
                        Value::String(k) => k.clone(),
                        _ => {
                            return Err(RuntimeError::TypeError {
                                message: "map key must be a string".into(),
                                span,
                            });
                        }
                    };
                    return Ok(Value::Bool(map.borrow().contains_key(&key)));
                }
                _ => {}
            },
            _ => {}
        }

        Err(RuntimeError::TypeError {
            message: format!("{} has no method {}", obj.type_name(), method),
            span,
        })
    }

    /// Evaluate a block and return the last expression value.
    pub fn eval_block(&mut self, block: &aria_ast::Block) -> Result<Value> {
        self.eval_stmts(&block.stmts)
    }

    /// Bind a pattern to a value, defining variables in the current environment.
    fn bind_pattern(&mut self, pattern: &aria_ast::Pattern, value: Value, span: Span) -> Result<()> {
        match &pattern.kind {
            aria_ast::PatternKind::Ident(name) => {
                self.environment.borrow_mut().define(name.clone(), value);
                Ok(())
            }
            aria_ast::PatternKind::Wildcard => {
                // Ignore the value
                Ok(())
            }
            aria_ast::PatternKind::Tuple(patterns) => {
                if let Value::Tuple(values) = value {
                    if patterns.len() != values.len() {
                        return Err(RuntimeError::TypeError {
                            message: format!(
                                "tuple pattern has {} elements, but value has {}",
                                patterns.len(),
                                values.len()
                            ),
                            span,
                        });
                    }
                    for (p, v) in patterns.iter().zip(values.iter()) {
                        self.bind_pattern(p, v.clone(), span)?;
                    }
                    Ok(())
                } else {
                    Err(RuntimeError::TypeError {
                        message: format!("expected tuple, got {}", value.type_name()),
                        span,
                    })
                }
            }
            aria_ast::PatternKind::Typed { pattern, .. } => {
                // Ignore type annotation at runtime, just bind the inner pattern
                self.bind_pattern(pattern, value, span)
            }
            _ => Err(RuntimeError::General {
                message: format!("unsupported pattern in let binding: {:?}", pattern.kind),
                span,
            }),
        }
    }

    /// Evaluate a slice of statements and return the last expression value.
    pub fn eval_stmts(&mut self, stmts: &[Stmt]) -> Result<Value> {
        let mut last_value = Value::Nil;

        for stmt in stmts {
            last_value = self.exec_stmt(stmt)?;
        }

        Ok(last_value)
    }

    /// Execute a statement.
    pub fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Value> {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.eval_expr(expr),

            StmtKind::Let { pattern, value, .. } => {
                let val = self.eval_expr(value)?;
                self.bind_pattern(pattern, val, stmt.span)?;
                Ok(Value::Nil)
            }

            StmtKind::Assign { target, value, .. } => {
                let val = self.eval_expr(value)?;

                match &target.kind {
                    ExprKind::Ident(name) => {
                        if !self.environment.borrow_mut().assign(name, val) {
                            return Err(RuntimeError::UndefinedVariable {
                                name: name.clone(),
                                span: target.span,
                            });
                        }
                    }
                    ExprKind::Index { object, index } => {
                        let obj_val = self.eval_expr(object)?;
                        let idx_val = self.eval_expr(index)?;

                        match obj_val {
                            Value::Array(arr) => {
                                let idx = idx_val.as_int().ok_or_else(|| {
                                    RuntimeError::TypeError {
                                        message: "array index must be integer".into(),
                                        span: index.span,
                                    }
                                })?;
                                let mut arr = arr.borrow_mut();
                                let len = arr.len();
                                let actual_idx = if idx < 0 {
                                    (len as i64 + idx) as usize
                                } else {
                                    idx as usize
                                };
                                if actual_idx >= len {
                                    return Err(RuntimeError::IndexOutOfBounds {
                                        index: idx,
                                        length: len,
                                        span: target.span,
                                    });
                                }
                                arr[actual_idx] = val;
                            }
                            Value::Map(map) => {
                                let key = match idx_val {
                                    Value::String(s) => s,
                                    _ => {
                                        return Err(RuntimeError::TypeError {
                                            message: "map key must be string".into(),
                                            span: index.span,
                                        });
                                    }
                                };
                                map.borrow_mut().insert(key, val);
                            }
                            _ => {
                                return Err(RuntimeError::TypeError {
                                    message: format!(
                                        "{} does not support indexed assignment",
                                        obj_val.type_name()
                                    ),
                                    span: target.span,
                                });
                            }
                        }
                    }
                    ExprKind::Field { object, field } => {
                        let obj_val = self.eval_expr(object)?;

                        match obj_val {
                            Value::Struct(s) => {
                                let mut s = s.borrow_mut();
                                if !s.fields.contains_key(&field.node) {
                                    return Err(RuntimeError::UndefinedField {
                                        struct_name: s.name.clone(),
                                        field: field.node.clone(),
                                        span: target.span,
                                    });
                                }
                                s.fields.insert(field.node.clone(), val);
                            }
                            _ => {
                                return Err(RuntimeError::TypeError {
                                    message: format!(
                                        "{} does not support field assignment",
                                        obj_val.type_name()
                                    ),
                                    span: target.span,
                                });
                            }
                        }
                    }
                    _ => {
                        return Err(RuntimeError::TypeError {
                            message: "invalid assignment target".into(),
                            span: target.span,
                        });
                    }
                }
                Ok(Value::Nil)
            }

            StmtKind::If { condition, then_branch, elsif_branches, else_branch } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    self.eval_block(then_branch)
                } else {
                    // Try elsif branches
                    for (elsif_cond, elsif_body) in elsif_branches {
                        let cond = self.eval_expr(elsif_cond)?;
                        if cond.is_truthy() {
                            return self.eval_block(elsif_body);
                        }
                    }
                    // Fall through to else
                    if let Some(else_branch) = else_branch {
                        self.eval_block(else_branch)
                    } else {
                        Ok(Value::Nil)
                    }
                }
            }

            StmtKind::While { condition, body } => {
                self.loop_depth += 1;
                let mut result = Value::Nil;

                loop {
                    let cond = self.eval_expr(condition)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    match self.eval_block(body) {
                        Ok(val) => result = val,
                        Err(RuntimeError::General { message, .. })
                            if message == "BREAK" =>
                        {
                            break;
                        }
                        Err(RuntimeError::General { message, .. })
                            if message == "CONTINUE" =>
                        {
                            continue;
                        }
                        Err(e) => {
                            self.loop_depth -= 1;
                            return Err(e);
                        }
                    }
                }

                self.loop_depth -= 1;
                Ok(result)
            }

            StmtKind::For { pattern, iterable, body } => {
                let iter_val = self.eval_expr(iterable)?;
                self.loop_depth += 1;

                // Get the variable name from the pattern
                let var_name = match &pattern.kind {
                    aria_ast::PatternKind::Ident(name) => name.clone(),
                    _ => {
                        return Err(RuntimeError::General {
                            message: "only simple identifier patterns supported in for loops".into(),
                            span: stmt.span,
                        });
                    }
                };

                let mut result = Value::Nil;

                match iter_val {
                    Value::Range { start, end, inclusive } => {
                        let end_val = if inclusive { end + 1 } else { end };
                        for i in start..end_val {
                            // Create new scope for loop body
                            let scope = Rc::new(RefCell::new(Environment::with_parent(
                                self.environment.clone(),
                            )));
                            scope.borrow_mut().define(var_name.clone(), Value::Int(i));

                            let prev = self.environment.clone();
                            self.environment = scope;

                            match self.eval_block(body) {
                                Ok(val) => result = val,
                                Err(RuntimeError::General { message, .. })
                                    if message == "BREAK" =>
                                {
                                    self.environment = prev;
                                    break;
                                }
                                Err(RuntimeError::General { message, .. })
                                    if message == "CONTINUE" =>
                                {
                                    self.environment = prev;
                                    continue;
                                }
                                Err(e) => {
                                    self.environment = prev;
                                    self.loop_depth -= 1;
                                    return Err(e);
                                }
                            }

                            self.environment = prev;
                        }
                    }
                    Value::Array(arr) => {
                        for item in arr.borrow().iter() {
                            let scope = Rc::new(RefCell::new(Environment::with_parent(
                                self.environment.clone(),
                            )));
                            scope.borrow_mut().define(var_name.clone(), item.clone());

                            let prev = self.environment.clone();
                            self.environment = scope;

                            match self.eval_block(body) {
                                Ok(val) => result = val,
                                Err(RuntimeError::General { message, .. })
                                    if message == "BREAK" =>
                                {
                                    self.environment = prev;
                                    break;
                                }
                                Err(RuntimeError::General { message, .. })
                                    if message == "CONTINUE" =>
                                {
                                    self.environment = prev;
                                    continue;
                                }
                                Err(e) => {
                                    self.environment = prev;
                                    self.loop_depth -= 1;
                                    return Err(e);
                                }
                            }

                            self.environment = prev;
                        }
                    }
                    Value::String(s) => {
                        for c in s.chars() {
                            let scope = Rc::new(RefCell::new(Environment::with_parent(
                                self.environment.clone(),
                            )));
                            scope.borrow_mut().define(
                                var_name.clone(),
                                Value::String(c.to_string().into()),
                            );

                            let prev = self.environment.clone();
                            self.environment = scope;

                            match self.eval_block(body) {
                                Ok(val) => result = val,
                                Err(RuntimeError::General { message, .. })
                                    if message == "BREAK" =>
                                {
                                    self.environment = prev;
                                    break;
                                }
                                Err(RuntimeError::General { message, .. })
                                    if message == "CONTINUE" =>
                                {
                                    self.environment = prev;
                                    continue;
                                }
                                Err(e) => {
                                    self.environment = prev;
                                    self.loop_depth -= 1;
                                    return Err(e);
                                }
                            }

                            self.environment = prev;
                        }
                    }
                    _ => {
                        self.loop_depth -= 1;
                        return Err(RuntimeError::TypeError {
                            message: format!("{} is not iterable", iter_val.type_name()),
                            span: iterable.span,
                        });
                    }
                }

                self.loop_depth -= 1;
                Ok(result)
            }

            StmtKind::Loop { body } => {
                self.loop_depth += 1;
                let mut result = Value::Nil;

                loop {
                    match self.eval_block(body) {
                        Ok(val) => result = val,
                        Err(RuntimeError::General { message, .. })
                            if message == "BREAK" =>
                        {
                            break;
                        }
                        Err(RuntimeError::General { message, .. })
                            if message == "CONTINUE" =>
                        {
                            continue;
                        }
                        Err(e) => {
                            self.loop_depth -= 1;
                            return Err(e);
                        }
                    }
                }

                self.loop_depth -= 1;
                Ok(result)
            }

            StmtKind::Break(_) => {
                if self.loop_depth == 0 {
                    return Err(RuntimeError::BreakOutsideLoop { span: stmt.span });
                }
                Err(RuntimeError::General {
                    message: "BREAK".into(),
                    span: stmt.span,
                })
            }

            StmtKind::Continue => {
                if self.loop_depth == 0 {
                    return Err(RuntimeError::ContinueOutsideLoop { span: stmt.span });
                }
                Err(RuntimeError::General {
                    message: "CONTINUE".into(),
                    span: stmt.span,
                })
            }

            StmtKind::Return(expr) => {
                if self.function_depth == 0 {
                    return Err(RuntimeError::ReturnOutsideFunction { span: stmt.span });
                }
                let val = if let Some(expr) = expr {
                    self.eval_expr(expr)?
                } else {
                    Value::Nil
                };
                Err(RuntimeError::General {
                    message: format!("RETURN:{:?}", val),
                    span: stmt.span,
                })
            }

            _ => Err(RuntimeError::General {
                message: format!("unimplemented statement kind: {:?}", stmt.kind),
                span: stmt.span,
            }),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod eval_tests {
    use super::*;
    use aria_parser::parse;

    fn eval(source: &str) -> Result<Value> {
        let (program, errors) = parse(source);
        if !errors.is_empty() {
            panic!("Parse errors: {:?}", errors);
        }

        let mut interp = Interpreter::new();
        interp.run(&program)
    }

    #[test]
    fn test_eval_literals() {
        // These would require expression-level evaluation
        // For now, the interpreter handles full programs
    }
}
