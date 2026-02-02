# ARIA-M01-04: Bidirectional Type Checking Enhancement

**Task ID**: ARIA-M01-04
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Enhancing Aria's type system with bidirectional checking for better error messages
**Researcher**: ATLAS (Research Agent)

---

## Executive Summary

This research document analyzes bidirectional type checking algorithms and their integration with Aria's existing Hindley-Milner-based unification system. Bidirectional typing combines two complementary modes: **type synthesis** (inferring types from expressions) and **type checking** (verifying expressions against expected types). This hybrid approach provides superior error messages while maintaining the expressiveness of full type inference.

**Key Finding**: Aria's current `TypeChecker` already implements elements of bidirectional typing through its `infer_expr` method. The enhancement proposed here formalizes this approach with an explicit `check_expr` mode that propagates expected types downward, dramatically improving error localization and message quality.

---

## 1. Bidirectional Type Checking Fundamentals

### 1.1 The Core Principle

Bidirectional type checking flows type information in two directions:

| Direction | Mode | Operation | Example |
|-----------|------|-----------|---------|
| **Bottom-up** | Synthesis | `infer(e) => A` | Literal `42` synthesizes `Int` |
| **Top-down** | Checking | `check(e, A)` | Lambda checked against `Int -> String` |

This contrasts with pure Hindley-Milner inference, which is exclusively bottom-up with deferred unification.

### 1.2 Pierce & Turner's Local Type Inference (2000)

The foundational work by Pierce and Turner introduced two key innovations:

1. **Local Type Argument Synthesis**: Inferring type arguments for polymorphic functions using only information from adjacent AST nodes
2. **Downward Propagation**: Passing expected types from function applications to arguments

```
// Traditional HM: type flows bottom-up only
let f = |x| x + 1    // Must infer x: Int from usage

// Bidirectional: type flows both ways
let f: Int -> Int = |x| x + 1  // x: Int known immediately
```

### 1.3 Dunfield & Krishnaswami's Complete and Easy (2013)

Extended bidirectional typing to handle higher-rank polymorphism with these key rules:

```
Synthesis Rules (=>):
  Var:    G |- x => A           (if x:A in G)
  App:    G |- e1 => A -> B
          G |- e2 <= A
          ----------------
          G |- e1 e2 => B

Checking Rules (<=):
  Sub:    G |- e => A    A <: B
          ---------------------
          G |- e <= B

  Lam:    G, x:A |- e <= B
          ------------------
          G |- \x.e <= A -> B
```

**Key Insight**: The checking mode (`<=`) allows lambdas to receive their parameter types from context, eliminating the need for annotations.

---

## 2. Industry Language Implementations

### 2.1 TypeScript: Contextual Typing

TypeScript uses bidirectional checking extensively for contextual typing:

```typescript
// Type flows from variable declaration to lambda
const handler: (event: MouseEvent) => void = (e) => {
    console.log(e.clientX);  // e typed as MouseEvent
};

// Type flows from array literal context
const nums: number[] = [1, 2, 3];  // Elements checked against number

// Generic inference with contextual return type
function map<T, U>(arr: T[], f: (x: T) => U): U[];
const result = map([1, 2, 3], x => x.toString());  // x: number inferred
```

**Error Message Quality**: TypeScript's bidirectional approach enables precise error locations:

```typescript
// Error points to specific argument, not the whole expression
const f: (x: number) => number = (x) => x + "!";
//                                      ^^^^^^^
// Error: Type 'string' is not assignable to type 'number'
```

### 2.2 Kotlin: Flow-Sensitive Smart Casts

Kotlin combines bidirectional inference with flow analysis:

```kotlin
// Bidirectional parameter inference in lambdas
val processor: (String) -> Int = { s -> s.length }

// Smart casts refine types bidirectionally
fun process(value: Any) {
    if (value is String) {
        println(value.length)  // value: String (smart cast)
    }
}

// SAM conversions use expected type
fun interface Runnable { fun run() }
val r: Runnable = { println("Running") }  // Lambda checked against SAM
```

**Specification Quote**: "Type inference in Kotlin is bidirectional; meaning the types of expressions may be derived not only from their arguments, but from their usage as well."

### 2.3 Swift: Constraint-Based Bidirectional

Swift uses a constraint solver with bidirectional information flow:

```swift
// Type propagates to closure parameters
let doubled = [1, 2, 3].map { x in x * 2 }  // x: Int from context

// Protocol witness matching
protocol Describable {
    func describe() -> String
}

struct Point: Describable {
    let x: Int
    func describe() -> String { "(\(x), \(y))" }  // Return type known
}

// Result builder DSL inference
@ViewBuilder
var body: some View {
    Text("Hello")  // Type checked against View
    Image("icon")  // Type checked against View
}
```

---

## 3. Integration Strategy for Aria

### 3.1 Current State Analysis

Aria's `TypeChecker` in `aria-types/src/lib.rs` currently implements:

| Feature | Implementation | Status |
|---------|---------------|--------|
| Type Variable Generation | `fresh_var()` | Complete |
| Unification | `unify(t1, t2, span)` | Complete |
| Bottom-up Inference | `infer_expr()` | Complete |
| Top-down Checking | Implicit in some cases | Partial |
| Error Localization | Span tracking | Basic |

### 3.2 Proposed Enhancement: Explicit Checking Mode

Add a `check_expr` method complementing the existing `infer_expr`:

```rust
impl TypeChecker {
    /// Check expression against an expected type (top-down)
    pub fn check_expr(
        &mut self,
        expr: &ast::Expr,
        expected: &Type,
        env: &TypeEnv
    ) -> TypeResult<()> {
        match (&expr.kind, expected) {
            // Lambda checking: propagate parameter types
            (ast::ExprKind::Lambda { params, body },
             Type::Function { params: param_types, return_type }) => {
                let mut lambda_env = TypeEnv::with_parent(Rc::new(env.clone()));

                // Bind parameters from expected type
                for (param, expected_ty) in params.iter().zip(param_types.iter()) {
                    let param_ty = if let Some(annotation) = &param.ty {
                        let annotated = self.resolve_type(annotation)?;
                        self.inference.unify(&annotated, expected_ty, param.name.span)?;
                        annotated
                    } else {
                        expected_ty.clone()  // Use expected type directly!
                    };
                    lambda_env.define_var(param.name.node.to_string(), param_ty);
                }

                // Check body against return type
                self.check_expr(body, return_type, &lambda_env)
            }

            // Array literal: check elements against expected element type
            (ast::ExprKind::Array(elements), Type::Array(elem_type)) => {
                for elem in elements {
                    self.check_expr(elem, elem_type, env)?;
                }
                Ok(())
            }

            // If expression: check both branches against expected type
            (ast::ExprKind::If { condition, then_branch, else_branch, .. }, _) => {
                let cond_type = self.infer_expr(condition, env)?;
                self.inference.unify(&cond_type, &Type::Bool, condition.span)?;

                self.check_block(then_branch, expected, env)?;
                if let Some(else_body) = else_branch {
                    self.check_block(else_body, expected, env)?;
                }
                Ok(())
            }

            // Fallback: synthesize then unify
            _ => {
                let inferred = self.infer_expr(expr, env)?;
                self.inference.unify(&inferred, expected, expr.span)
            }
        }
    }

    /// Check block against expected type
    pub fn check_block(
        &mut self,
        block: &ast::Block,
        expected: &Type,
        env: &TypeEnv
    ) -> TypeResult<()> {
        let mut block_env = TypeEnv::with_parent(Rc::new(env.clone()));

        for (i, stmt) in block.stmts.iter().enumerate() {
            if i == block.stmts.len() - 1 {
                // Last statement: check against expected
                if let ast::StmtKind::Expr(expr) = &stmt.kind {
                    return self.check_expr(expr, expected, &block_env);
                }
            }
            self.check_stmt(stmt, &mut block_env)?;
        }

        // Block has no tail expression
        self.inference.unify(&Type::Unit, expected, block.span)
    }
}
```

### 3.3 Modified Expression Inference

Update `infer_expr` to use checking mode when possible:

```rust
impl TypeChecker {
    pub fn infer_expr(&mut self, expr: &ast::Expr, env: &TypeEnv) -> TypeResult<Type> {
        match &expr.kind {
            // Function call: check arguments against parameter types
            ast::ExprKind::Call { func, args } => {
                let func_type = self.infer_expr(func, env)?;
                match self.inference.apply(&func_type) {
                    Type::Function { params, return_type } => {
                        if args.len() != params.len() {
                            return Err(TypeError::WrongTypeArity {
                                expected: params.len(),
                                found: args.len(),
                                span: expr.span,
                            });
                        }

                        // CHECK mode: propagate expected types to arguments
                        for (arg, param_type) in args.iter().zip(params.iter()) {
                            self.check_expr(&arg.value, param_type, env)?;
                        }

                        Ok(*return_type)
                    }
                    // ... handle type variable case
                    _ => /* existing logic */
                }
            }

            // Let binding with type annotation: check value
            ast::ExprKind::Let { name, ty: Some(type_ann), value, .. } => {
                let expected = self.resolve_type(type_ann)?;
                self.check_expr(value, &expected, env)?;
                // ... bind name
                Ok(Type::Unit)
            }

            // ... other cases remain synthesis-based
        }
    }
}
```

### 3.4 Hybrid Unification + Bidirectional Architecture

The recommended architecture combines both approaches:

```
┌─────────────────────────────────────────────────────────────┐
│                    Bidirectional Layer                       │
│  ┌─────────────┐              ┌─────────────┐               │
│  │   Synthesis  │ <==========> │   Checking  │               │
│  │  infer_expr  │              │  check_expr │               │
│  └──────┬──────┘              └──────┬──────┘               │
│         │                            │                       │
│         └──────────┬─────────────────┘                       │
│                    │                                          │
│              ┌─────▼─────┐                                   │
│              │ Unification │                                  │
│              │   Engine    │                                  │
│              └─────────────┘                                  │
│         (TypeVar substitution, occurs check)                 │
└─────────────────────────────────────────────────────────────┘
```

**Key Principle**: Bidirectional typing handles the *flow* of type information, while unification handles *equality constraints* between types.

---

## 4. Error Message Quality Improvements

### 4.1 Current Error Messages (Pure Synthesis)

```
// Current Aria error for mismatched lambda return
fn process(callback: (Int) -> String) { ... }
process(|x| x + 1)

// Error: Type mismatch: expected String, found Int
//        at line 2, column 15
```

Problem: Error points to the entire lambda, not the specific mismatch location.

### 4.2 Enhanced Error Messages (With Bidirectional)

```
// Enhanced error with bidirectional checking
fn process(callback: (Int) -> String) { ... }
process(|x| x + 1)
//          ^^^^^
// Error: Type mismatch in return expression
//   Expected: String (from callback parameter type)
//   Found: Int (inferred from expression `x + 1`)
//
//   Help: Consider converting to String:
//         process(|x| (x + 1).to_string())
```

### 4.3 Error Localization Strategy

Bidirectional checking enables precise blame assignment:

| Scenario | Synthesis-Only Blame | Bidirectional Blame |
|----------|---------------------|---------------------|
| Lambda parameter | Entire lambda | Missing annotation |
| Lambda body | Lambda or call site | Specific expression |
| Array element | Entire array | Mismatched element |
| Function arg | Function call | Specific argument |
| If branches | If expression | Divergent branch |

### 4.4 Implementation: Enhanced TypeError

```rust
#[derive(Debug, Clone, Error)]
pub enum TypeError {
    #[error("Type mismatch")]
    Mismatch {
        expected: String,
        found: String,
        span: Span,
        expected_source: Option<TypeSource>,  // NEW: Where expected type came from
        found_source: Option<TypeSource>,      // NEW: Where found type came from
    },

    // ... other variants
}

#[derive(Debug, Clone)]
pub enum TypeSource {
    Annotation { span: Span },
    ParameterType { func_name: String, param_index: usize },
    ReturnType { func_name: String },
    ArrayElement { array_span: Span },
    Inference { from_expr: String },
}

impl TypeError {
    pub fn format_with_context(&self, source: &str) -> String {
        match self {
            TypeError::Mismatch { expected, found, span, expected_source, found_source } => {
                let mut msg = format!("Type mismatch at {}:\n", span);

                if let Some(src) = expected_source {
                    msg.push_str(&format!("  Expected: {} ({})\n", expected, src.description()));
                }
                if let Some(src) = found_source {
                    msg.push_str(&format!("  Found: {} ({})\n", found, src.description()));
                }

                msg
            }
            // ... other cases
        }
    }
}
```

---

## 5. Aria-Specific Design Considerations

### 5.1 Ownership and Bidirectional Typing

Aria's ownership system interacts with bidirectional checking:

```aria
fn transform(processor: (own String) -> own String) {
    processor("hello")
}

// Bidirectional checking propagates ownership requirements
transform(|s| {
    s.uppercase()  // s has ownership context from parameter type
})
```

### 5.2 Effect Inference Integration

Effect annotations can flow bidirectionally:

```aria
fn with_io<T>(action: () -> T throws IO) -> T throws IO {
    action()
}

// Effect type flows to lambda
with_io(|| {
    print("Hello")  // IO effect checked, not inferred
})
```

### 5.3 Contract System Interaction

Contracts benefit from bidirectional type flow:

```aria
fn sqrt(x: Float) -> Float
    requires x >= 0.0
    ensures result >= 0.0
{
    // 'result' type known from return signature
    // Enables contract expression type checking
}
```

---

## 6. Implementation Roadmap

### Phase 1: Core Bidirectional Infrastructure (Week 1-2)

1. Add `check_expr` method to `TypeChecker`
2. Implement checking mode for:
   - Lambda expressions
   - Array/Map literals
   - If/match expressions
3. Update call expression inference to use checking

### Phase 2: Error Message Enhancement (Week 3-4)

1. Extend `TypeError` with source tracking
2. Implement blame assignment for checking failures
3. Add contextual help suggestions
4. Create error formatting with source context

### Phase 3: Integration Testing (Week 5)

1. Add test suite for bidirectional scenarios
2. Benchmark error message quality
3. Validate against PRD-v2 requirements

### Phase 4: Advanced Features (Week 6+)

1. Flow-sensitive smart casts
2. Generic type argument synthesis
3. Effect type propagation

---

## 7. Code Examples: Before and After

### Example 1: Lambda Parameter Inference

```aria
// BEFORE: Requires annotation or poor error
fn map<T, U>(arr: [T], f: (T) -> U) -> [U] { ... }

let nums = [1, 2, 3]
let strs = map(nums, |x| x.to_string())  // x type unclear in error

// AFTER: Type flows from map's signature
let strs = map(nums, |x| x.to_string())
//                    ^ x: Int known from (T) -> U where T = Int
```

### Example 2: Conditional Expression Typing

```aria
// BEFORE: Branches must be unified
fn get_value() -> String {
    if condition {
        "hello"
    } else {
        42  // Error: somewhere in if expression
    }
}

// AFTER: Both branches checked against String
fn get_value() -> String {
    if condition {
        "hello"
    } else {
        42  // Error: expected String in else branch, found Int
//      ^^
    }
}
```

### Example 3: Nested Callback Types

```aria
// Complex callback scenario
fn with_transaction<T>(action: (Transaction) -> T) -> Result<T, DbError>

// AFTER: Deep type propagation
with_transaction(|tx| {
    tx.query("SELECT ...")  // tx: Transaction from context
      .map(|row| row.get("name"))  // row type inferred from query return
})
```

---

## 8. References

### Academic Papers

1. Pierce, B. C. & Turner, D. N. (2000). "Local Type Inference." ACM TOPLAS 22(1):1-44.
   - https://www.cis.upenn.edu/~bcpierce/papers/lti-toplas.pdf

2. Dunfield, J. & Krishnaswami, N. R. (2013). "Complete and Easy Bidirectional Typechecking for Higher-Rank Polymorphism." ICFP 2013.
   - https://arxiv.org/abs/1306.6032

3. Dunfield, J. & Krishnaswami, N. R. (2021). "Bidirectional Typing." ACM Computing Surveys 54(5).
   - https://dl.acm.org/doi/10.1145/3450952

4. Zhao, J. et al. (2022). "Elementary Type Inference." ECOOP 2022.
   - https://drops.dagstuhl.de/storage/00lipics/lipics-vol222-ecoop2022/LIPIcs.ECOOP.2022.2/LIPIcs.ECOOP.2022.2.pdf

### Language Implementations

5. TypeScript Type Inference Documentation
   - https://www.typescriptlang.org/docs/handbook/type-inference.html

6. Kotlin Language Specification: Type Inference
   - https://kotlinlang.org/spec/type-inference.html

7. Swift Compiler: New Diagnostic Architecture
   - https://www.swift.org/blog/new-diagnostic-arch-overview/

### Error Localization

8. "Total Type Error Localization and Recovery with Holes." POPL 2024.
   - https://dl.acm.org/doi/10.1145/3632910

9. "Getting into the Flow: Towards Better Type Error Messages." OOPSLA 2023.
   - https://dl.acm.org/doi/10.1145/3622812

---

## 9. Conclusion

Bidirectional type checking is the recommended enhancement for Aria's type system. By adding an explicit checking mode that propagates expected types downward, Aria can achieve:

1. **Better Error Messages**: Precise blame assignment with contextual information
2. **Reduced Annotation Burden**: Lambda parameters inferred from context
3. **Improved IDE Support**: Richer type information for completions
4. **Future Extensibility**: Foundation for effects, contracts, and ownership propagation

The hybrid approach preserves Aria's existing HM-style inference while adding the complementary checking mode, giving developers the best of both worlds.

---

## Appendix A: Bidirectional Typing Judgment Summary

```
Synthesis (G |- e => A):
  G |- x => G(x)                                    [Var]
  G |- n => Int                                     [IntLit]
  G |- "s" => String                                [StrLit]
  G |- e1 => A -> B    G |- e2 <= A
  ─────────────────────────────────                 [App]
  G |- e1 e2 => B

Checking (G |- e <= A):
  G, x:A |- e <= B
  ────────────────                                  [LamCheck]
  G |- \x.e <= A -> B

  G |- e1 <= Bool    G |- e2 <= A    G |- e3 <= A
  ───────────────────────────────────────────────   [IfCheck]
  G |- if e1 then e2 else e3 <= A

  G |- e => A    A <: B
  ─────────────────────                             [Sub]
  G |- e <= B
```

---

## Appendix B: TypeChecker Method Reference

| Method | Mode | Purpose |
|--------|------|---------|
| `infer_expr` | Synthesis | Determine type from expression |
| `check_expr` | Checking | Verify expression has expected type |
| `infer_block` | Synthesis | Infer block's result type |
| `check_block` | Checking | Verify block produces expected type |
| `unify` | Constraint | Equate two types via substitution |
| `apply` | Resolution | Apply substitution to type |
