# ARIA-M01-02: Hindley-Milner Extensions Analysis

**Task ID**: ARIA-M01-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study HM extensions (System F, Fw) for Aria's needs

---

## Executive Summary

Hindley-Milner (HM) type inference is the gold standard for functional language type systems, providing principal type inference without annotations. This research analyzes HM extensions and their applicability to Aria's design goals.

---

## 1. Hindley-Milner Fundamentals

### 1.1 Core Properties

- **Principal Types**: Every well-typed expression has a unique most-general type
- **Decidability**: Type inference is decidable (terminates)
- **Completeness**: If a type exists, Algorithm W/J will find it
- **No Annotations**: Types can be inferred without programmer hints

### 1.2 The Type Language

```
Monotypes (t):  t ::= a | C t1...tn | t1 -> t2
Polytypes (s):  s ::= t | forall a. s
```

- **Monotypes**: Concrete types without quantifiers
- **Polytypes (type schemes)**: Types with universal quantification

### 1.3 Key Operations

| Operation | Description | Example |
|-----------|-------------|---------|
| **Generalization** | Close over free type variables | `a -> a` becomes `forall a. a -> a` |
| **Instantiation** | Create fresh type variables | `forall a. a -> a` becomes `t1 -> t1` |
| **Unification** | Find substitution making types equal | `a -> Int` unifies with `Bool -> b` |

---

## 2. Algorithm W vs Algorithm J

### 2.1 Algorithm W (Milner 1978)

```
W(env, expr) -> (substitution, type)

W(env, x) =
  let s = lookup(env, x) in
  (empty_subst, instantiate(s))

W(env, fn x => e) =
  let a = fresh_type_var() in
  let (s, t) = W(env + {x: a}, e) in
  (s, s(a) -> t)

W(env, e1 e2) =
  let (s1, t1) = W(env, e1) in
  let (s2, t2) = W(s1(env), e2) in
  let a = fresh_type_var() in
  let s3 = unify(s2(t1), t2 -> a) in
  (s3 o s2 o s1, s3(a))

W(env, let x = e1 in e2) =
  let (s1, t1) = W(env, e1) in
  let s' = generalize(s1(env), t1) in
  let (s2, t2) = W(s1(env) + {x: s'}, e2) in
  (s2 o s1, t2)
```

**Characteristics**:
- Bottom-up constraint generation
- Eager substitution application
- O(n) typical, O(exp(n)) worst case

### 2.2 Algorithm J (More Efficient Variant)

- Uses **union-find** for type variable equivalence
- Defers substitution application
- Better practical performance
- Same theoretical complexity

### 2.3 Constraint-Based Inference

Modern approach separating concerns:

1. **Constraint Generation**: Traverse AST, generate type equations
2. **Constraint Solving**: Unify constraints to find substitution
3. **Error Localization**: Better error messages with constraint graph

**Advantages**:
- Cleaner implementation
- Better error reporting
- Easier to extend

---

## 3. Let-Polymorphism Patterns

### 3.1 The Value Restriction

**Problem**: Unrestricted generalization is unsound with mutation

```ml
let r = ref None in  (* Would infer: forall a. ref (option a) *)
r := Some 1;         (* r : ref (option int) *)
!r + "hello"         (* r : ref (option string) - UNSOUND! *)
```

**Solution**: Only generalize at let-bindings of syntactic values

### 3.2 Relaxed Value Restriction (OCaml)

- Allows generalization of covariant type variables
- More polymorphism while maintaining soundness

### 3.3 Implications for Aria

- With ownership/move semantics, value restriction less critical
- Consider: when to generalize with effects?

---

## 4. System F and Beyond

### 4.1 System F (Second-Order Lambda Calculus)

- Explicit type abstraction and application
- No principal types (type inference undecidable)
- Example: `id = Λa. λx:a. x`

### 4.2 Rank-N Types

| Rank | Quantifier Positions | Decidable |
|------|---------------------|-----------|
| Rank-1 (HM) | Top-level only | Yes |
| Rank-2 | Argument positions | Yes (with hints) |
| Rank-N | Arbitrary nesting | No |

**Example Rank-2**:
```haskell
runST :: (forall s. ST s a) -> a
```

### 4.3 Bidirectional Type Checking

For higher-rank types without full annotation:

```
check(e, A)  -- Check e has type A
infer(e) -> A  -- Infer type of e

check(λx.e, A -> B) = check(e, B) with x:A
infer(e1 e2) =
  let A -> B = infer(e1) in
  check(e2, A);
  return B
```

---

## 5. Type Class Integration

### 5.1 Qualified Types (Wadler & Blott)

Extend HM with type class constraints:

```
Types:    t ::= a | C t1...tn | t1 -> t2
Schemes:  s ::= t | C a => s | forall a. s

Example: show :: forall a. Show a => a -> String
```

### 5.2 Instance Resolution

- **Coherence**: At most one instance per type
- **Orphan instances**: Instances defined outside type/class module
- **Overlapping instances**: Multiple applicable instances

### 5.3 Type Class Inference

1. Generate constraints including class constraints
2. Solve type constraints (unification)
3. Resolve class constraints (instance search)
4. Report ambiguity if constraints remain

### 5.4 Functional Dependencies

```haskell
class Collection c e | c -> e where
  insert :: e -> c -> c
```

- `c -> e` means: `c` determines `e`
- Improves inference by providing type flow

### 5.5 Associated Types (Type Families)

```haskell
class Collection c where
  type Elem c
  insert :: Elem c -> c -> c
```

- More explicit than fundeps
- Enables type-level computation

---

## 6. Modern Extensions

### 6.1 GADTs (Generalized Algebraic Data Types)

```haskell
data Expr a where
  LitInt  :: Int -> Expr Int
  LitBool :: Bool -> Expr Bool
  Add     :: Expr Int -> Expr Int -> Expr Int
  If      :: Expr Bool -> Expr a -> Expr a -> Expr a

eval :: Expr a -> a
eval (LitInt n) = n  -- Type refinement: a ~ Int
```

**Impact on Inference**:
- Pattern matching refines types
- Requires some annotation on GADT-using functions
- Local type inference typically needed

### 6.2 Type Families

```haskell
type family Element c where
  Element [a] = a
  Element (Set a) = a
  Element ByteString = Word8
```

- Type-level functions
- Can be open (extensible) or closed

### 6.3 Dependent Types (Limited)

- **Const generics** (Rust): Types parameterized by compile-time values
- **Literal types** (TypeScript): `type One = 1`
- **Singletons** (Haskell): Runtime/type correspondence

---

## 7. Recommendations for Aria

### 7.1 Core Type System

**Adopt**: HM-based inference with bidirectional checking
- Annotations at function signatures (like Rust)
- Full inference within function bodies
- Let-polymorphism for local definitions

### 7.2 Polymorphism

**Adopt**: Rank-1 polymorphism as default
- Consider: Limited rank-2 for specific patterns (runST-like)
- Explicit syntax for higher-rank when needed

### 7.3 Ad-Hoc Polymorphism

**Adopt**: Trait system (like Rust) over type classes
- Coherence via orphan rules (crate/module local)
- Associated types for type-level abstraction
- Consider: Functional dependencies for edge cases

### 7.4 Extensions to Evaluate

| Feature | Priority | Rationale |
|---------|----------|-----------|
| Const generics | High | Needed for array sizes, compile-time computation |
| GADTs | Medium | Useful for typed ASTs, contracts |
| Type families | Medium | Complex but powerful for APIs |
| Rank-2 types | Low | Limited use cases |

### 7.5 Inference Strategy

```
Phase 1: Constraint Generation
  - Walk AST
  - Generate type equations
  - Generate trait constraints

Phase 2: Constraint Solving
  - Unification for type equations
  - Trait resolution for constraints

Phase 3: Error Reporting
  - Constraint graph for blame assignment
  - Suggestions based on near-matches
```

---

## 8. Key Papers

1. **Damas & Milner (1982)** - "Principal Type Schemes for Functional Programs"
2. **Wadler & Blott (1989)** - "How to make ad-hoc polymorphism less ad hoc"
3. **Odersky & Laufer (1996)** - "Putting Type Annotations to Work"
4. **Pierce & Turner (2000)** - "Local Type Inference"
5. **Dunfield & Krishnaswami (2013)** - "Complete and Easy Bidirectional Typechecking"
6. **Vytiniotis et al. (2011)** - "OutsideIn(X): Modular Type Inference with Local Assumptions"

---

## 9. Implementation Considerations

### 9.1 Error Messages

- Store source locations with constraints
- Build explanation graph for errors
- Suggest fixes based on:
  - Missing trait implementations
  - Type mismatches with similar types
  - Annotation requirements

### 9.2 Incremental Inference

- Cache per-function type information
- Invalidate only affected functions on change
- Consider: Type-checking as graph computation

### 9.3 IDE Integration

- Provide types on hover (from inference results)
- Suggest type annotations
- Show trait bounds and constraints

---

## Appendix: Algorithm W Implementation Sketch

```python
class TypeChecker:
    def __init__(self):
        self.fresh_counter = 0

    def fresh_var(self):
        self.fresh_counter += 1
        return TypeVar(f"t{self.fresh_counter}")

    def instantiate(self, scheme):
        """Replace quantified vars with fresh vars"""
        subst = {v: self.fresh_var() for v in scheme.vars}
        return apply_subst(subst, scheme.body)

    def generalize(self, env, ty):
        """Quantify free vars not in environment"""
        free = free_vars(ty) - free_vars_env(env)
        return Scheme(free, ty)

    def unify(self, t1, t2):
        """Find substitution making t1 = t2"""
        if isinstance(t1, TypeVar):
            return self.bind(t1, t2)
        if isinstance(t2, TypeVar):
            return self.bind(t2, t1)
        if isinstance(t1, Arrow) and isinstance(t2, Arrow):
            s1 = self.unify(t1.arg, t2.arg)
            s2 = self.unify(apply(s1, t1.ret), apply(s1, t2.ret))
            return compose(s2, s1)
        if t1 == t2:
            return {}
        raise TypeError(f"Cannot unify {t1} with {t2}")

    def infer(self, env, expr):
        """Algorithm W: return (substitution, type)"""
        match expr:
            case Var(name):
                scheme = env[name]
                return ({}, self.instantiate(scheme))

            case Lambda(param, body):
                param_ty = self.fresh_var()
                new_env = env | {param: Scheme([], param_ty)}
                s, body_ty = self.infer(new_env, body)
                return (s, Arrow(apply(s, param_ty), body_ty))

            case App(func, arg):
                s1, func_ty = self.infer(env, func)
                s2, arg_ty = self.infer(apply_env(s1, env), arg)
                ret_ty = self.fresh_var()
                s3 = self.unify(apply(s2, func_ty), Arrow(arg_ty, ret_ty))
                return (compose(s3, compose(s2, s1)), apply(s3, ret_ty))

            case Let(name, value, body):
                s1, val_ty = self.infer(env, value)
                scheme = self.generalize(apply_env(s1, env), val_ty)
                s2, body_ty = self.infer(apply_env(s1, env) | {name: scheme}, body)
                return (compose(s2, s1), body_ty)
```
