# ARIA-M02-01: Deep Dive into Rust Borrow Checker

**Task ID**: ARIA-M02-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Understanding Rust's borrow checker internals

---

## Executive Summary

Rust's borrow checker is the cornerstone of its memory safety guarantees. This research analyzes the borrow checker internals, including Non-Lexical Lifetimes (NLL) and the next-generation Polonius algorithm, to inform Aria's ownership inference design.

---

## 1. Borrow Checker Fundamentals

### 1.1 Core Rules

1. **Ownership**: Each value has exactly one owner
2. **Borrowing**: References can be shared (&T) or exclusive (&mut T)
3. **Exclusivity**: Either one &mut T OR many &T, never both
4. **Lifetime**: References cannot outlive their referent

### 1.2 Why These Rules Work

```rust
// These rules prevent:
// 1. Use-after-free
let r;
{
    let x = 5;
    r = &x;  // ERROR: x doesn't live long enough
}
println!("{}", r);

// 2. Data races
let mut v = vec![1, 2, 3];
let r = &v[0];
v.push(4);  // ERROR: cannot borrow v as mutable
println!("{}", r);  // r might be invalidated

// 3. Iterator invalidation
for x in &v {
    v.push(*x);  // ERROR: cannot borrow v as mutable
}
```

---

## 2. Non-Lexical Lifetimes (NLL)

### 2.1 The Problem with Lexical Lifetimes

**Old (Lexical) Borrow Checker**:
```rust
fn old_style() {
    let mut data = vec![1, 2, 3];
    let first = &data[0];  // Borrow starts
    println!("{}", first);  // Last use of first
    data.push(4);  // ERROR in old Rust: borrow ends at }
}  // Borrow ends here (lexical scope)
```

**New (NLL) Borrow Checker**:
```rust
fn nll_style() {
    let mut data = vec![1, 2, 3];
    let first = &data[0];  // Borrow starts
    println!("{}", first);  // Last use of first, borrow ends
    data.push(4);  // OK! borrow already ended
}
```

### 2.2 NLL Implementation

NLL operates on **MIR (Mid-level Intermediate Representation)**, not AST.

**Key Components**:

1. **Control Flow Graph (CFG)**: Program represented as basic blocks with edges
2. **Region Variables**: Abstract lifetimes assigned to references
3. **Liveness Analysis**: Determines where each variable might be used
4. **Region Inference**: Solves constraints to find minimum valid lifetimes

```
Region Inference Constraints:
- r1: 'outlives(r2)  // r1 must live at least as long as r2
- r1: 'live_at(point) // r1 must be valid at program point
```

### 2.3 NLL Algorithm Overview

```
1. Build MIR CFG from source
2. Compute liveness for each variable
3. Generate region constraints:
   - From borrows: &'r T creates region 'r
   - From assignments: propagate regions
   - From function calls: match parameter lifetimes
4. Solve constraints using fixed-point iteration
5. Check borrow conflicts using solved regions
```

---

## 3. Polonius: Next-Generation Borrow Checker

### 3.1 What Polonius Adds

Polonius handles cases NLL cannot:

```rust
// NLL rejects this, Polonius accepts
fn get_or_insert(map: &mut HashMap<u32, String>, key: u32) -> &String {
    if let Some(v) = map.get(&key) {
        return v;  // Returns reference
    }
    map.insert(key, String::new());  // NLL thinks map still borrowed
    map.get(&key).unwrap()
}
```

**Key difference**: NLL is location-insensitive for return analysis; Polonius is location-sensitive.

### 3.2 Polonius Architecture

Polonius formulates borrow checking as **Datalog-style** constraint solving:

```datalog
// Facts generated from MIR
loan_issued_at(Loan, Point)
loan_killed_at(Loan, Point)
cfg_edge(Point, Point)
use_of_var_derefs_origin(Variable, Origin)

// Rules (computed)
loan_live_at(Loan, Point) :-
    loan_issued_at(Loan, Point).

loan_live_at(Loan, Q) :-
    loan_live_at(Loan, P),
    cfg_edge(P, Q),
    not loan_killed_at(Loan, P).

// Error if loan live where it shouldn't be
error(Loan, Point) :-
    loan_live_at(Loan, Point),
    loan_invalidated_at(Loan, Point).
```

### 3.3 Polonius Algorithm Variants

| Variant | Description | Performance |
|---------|-------------|-------------|
| **Naive** | Direct Datalog evaluation | Slow |
| **DatafrogOpt** | Optimized Datalog | Moderate |
| **LocationInsensitive** | Quick pre-filter | Fast |
| **Hybrid** | LocationInsensitive + DatafrogOpt | Best practical |

**Hybrid Strategy**:
1. Run fast LocationInsensitive analysis
2. If errors found, re-analyze with precise DatafrogOpt
3. Report precise errors

### 3.4 Key Polonius Concepts

**Origin**: What a reference was derived from
```rust
let x = &y;  // x has origin: borrow of y
let z = x;   // z has same origin as x
```

**Loan**: A specific borrow event
```rust
let r = &mut x;  // Creates loan L1
// L1 is "live" while r or anything derived from r is used
```

**Subset Relations**: Origin containment
```rust
fn identity<'a>(x: &'a i32) -> &'a i32 { x }
// Output origin subset of input origin
```

---

## 4. Cases Requiring Explicit Lifetimes

### 4.1 Functions Returning References

```rust
// Cannot infer: which input does output relate to?
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

// Could be different lifetimes
fn longest2<'a, 'b>(x: &'a str, y: &'b str) -> &'a str {
    x  // Always returns x
}
```

### 4.2 Structs Containing References

```rust
// Must annotate: struct lifetime tied to field lifetime
struct Parser<'a> {
    input: &'a str,
    position: usize,
}
```

### 4.3 Lifetime Bounds in Traits

```rust
trait Processor<'a> {
    fn process(&self, input: &'a str) -> &'a str;
}
```

### 4.4 Self-Referential Structures

```rust
// Impossible in safe Rust without Pin
struct SelfRef {
    data: String,
    ref_to_data: &String,  // Cannot express lifetime
}
```

---

## 5. Borrow Checker Error Categories

### 5.1 Lifetime Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "borrowed value does not live long enough" | Reference outlives referent | Extend lifetime or restructure |
| "lifetime may not live long enough" | Lifetime bounds not satisfied | Add lifetime annotations |
| "cannot return reference to local variable" | Dangling reference | Return owned value or use lifetime parameter |

### 5.2 Borrowing Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "cannot borrow as mutable because also borrowed as immutable" | Aliasing + mutation | Restructure to separate borrows |
| "cannot borrow as mutable more than once" | Multiple mutable borrows | Use interior mutability or restructure |
| "cannot move out of borrowed content" | Moving from reference | Clone or restructure |

---

## 6. Implementation Insights

### 6.1 MIR Representation

```rust
// Source
fn example(x: i32) -> i32 {
    let y = x + 1;
    y * 2
}

// MIR (simplified)
fn example(_1: i32) -> i32 {
    let mut _0: i32;     // return place
    let _2: i32;         // y

    bb0: {
        _2 = Add(_1, const 1_i32);
        _0 = Mul(_2, const 2_i32);
        return;
    }
}
```

### 6.2 Borrow Tracking in MIR

```rust
// Source
fn borrow_example(v: &mut Vec<i32>) {
    let r = &v[0];
    println!("{}", r);
}

// MIR shows explicit borrows
bb0: {
    _2 = &(*_1)[0];      // Borrow created
    // ... use _2 ...
    StorageDead(_2);     // Borrow ends
}
```

### 6.3 Two-Phase Borrows

```rust
// This works due to two-phase borrows
v.push(v.len());

// Desugared:
// 1. First phase: &mut v (reserved, not active)
// 2. Evaluate v.len() with shared borrow
// 3. Second phase: activate mutable borrow
// 4. Call push
```

---

## 7. Recommendations for Aria

### 7.1 What to Adopt

1. **Ownership model**: Move semantics as default
2. **Borrowing**: Shared vs exclusive references
3. **Control-flow-sensitive analysis**: NLL-style, not lexical
4. **Two-phase borrows**: For method call ergonomics

### 7.2 What to Innovate

1. **Lifetime inference**: Infer lifetimes where Rust requires annotations
2. **Simpler mental model**: Hide regions from users when possible
3. **Better errors**: Graph-based explanation of borrow conflicts
4. **Gradual complexity**: Simple cases need no annotation

### 7.3 Inference Opportunities

| Rust Requires Annotation | Aria Could Infer |
|--------------------------|------------------|
| Function return lifetimes | When single input reference |
| Struct lifetime parameters | When struct usage bounded |
| Trait lifetime bounds | When implementation visible |

### 7.4 Potential Simplifications

1. **Default lifetime elision**: More aggressive than Rust
2. **Region variables hidden**: Present as borrow scopes, not 'a
3. **Escape hatches**: Explicit only when inference fails

---

## 8. Key Resources

1. **Rust RFC 2094** - Non-Lexical Lifetimes
2. **Polonius Repository** - github.com/rust-lang/polonius
3. **Niko Matsakis Blog** - "The Borrow Checker Within"
4. **Rustc Dev Guide** - Borrow Checker chapter
5. **"Oxide: The Essence of Rust"** - Formal semantics paper

---

## 9. Open Questions

1. Can we infer function lifetime parameters in most cases?
2. How do we handle self-referential patterns ergonomically?
3. What's the performance cost of more inference?
4. How do we explain borrow errors without exposing regions?

---

## Appendix: Polonius Fact Generation

```rust
fn example<'a>(x: &'a mut i32, y: &'a i32) -> &'a i32 {
    *x = *y + 1;
    y
}
```

**Generated Facts**:
```
// Loans
loan_issued_at(L1, bb0[0])  // &mut x dereference
loan_issued_at(L2, bb0[1])  // &y dereference

// Origins
use_of_var_derefs_origin(x, O_x)
use_of_var_derefs_origin(y, O_y)

// Subset (from signature)
origin_subset(O_return, O_y)

// CFG
cfg_edge(bb0[0], bb0[1])
cfg_edge(bb0[1], bb0[2])
...
```
