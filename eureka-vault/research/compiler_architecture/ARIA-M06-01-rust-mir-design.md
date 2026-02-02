# ARIA-M06-01: Rust MIR Design Study

**Task ID**: ARIA-M06-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Deep dive into Rust's Mid-level IR

---

## Executive Summary

MIR (Mid-level Intermediate Representation) is Rust's key IR for borrow checking and optimization. This research analyzes its structure, role in borrow checking, and optimization capabilities.

---

## 1. MIR Overview

### 1.1 What is MIR?

MIR is Rust's **Mid-level Intermediate Representation**, introduced in RFC 1211:
- Constructed from HIR (High-level IR)
- Used for flow-sensitive safety checks
- Foundation for borrow checker
- Target for optimization passes

### 1.2 Compilation Pipeline

```
Source → AST → HIR → MIR → LLVM IR → Machine Code
                      ↑
            Borrow checking happens here
            Type checking complete
            Monomorphization done
```

---

## 2. MIR Design Principles

### 2.1 Control-Flow Graph Based

MIR is based on a CFG, not an AST:

```rust
// Source code
fn example(x: bool) -> i32 {
    if x { 1 } else { 2 }
}

// MIR representation (simplified)
bb0: {
    switchInt(_1) -> [0: bb2, otherwise: bb1]
}
bb1: {
    _0 = const 1_i32
    goto -> bb3
}
bb2: {
    _0 = const 2_i32
    goto -> bb3
}
bb3: {
    return
}
```

### 2.2 Key Characteristics

| Feature | Description |
|---------|-------------|
| No nested expressions | Every expression is a statement |
| Explicit types | All types fully resolved |
| Basic blocks | Linear sequences of statements |
| Terminators | End each block (goto, switch, return) |
| Places | Memory locations (variables, fields) |
| Rvalues | Computed values |

### 2.3 MIR Node Types

```rust
// Statement kinds
StorageLive(Local)      // Start of variable lifetime
StorageDead(Local)      // End of variable lifetime
Assign(Place, Rvalue)   // Assignment

// Terminator kinds
Goto { target }
SwitchInt { discr, targets }
Call { func, args, destination, target }
Return
Drop { place, target }
Assert { cond, expected, target }
```

---

## 3. MIR and Borrow Checking

### 3.1 Why MIR for Borrow Checking?

The borrow checker operates on MIR, not source code:
- CFG enables precise control-flow analysis
- Explicit lifetimes and types
- Simplified reasoning about borrows

### 3.2 Non-Lexical Lifetimes (NLL)

MIR enabled NLL by allowing fine-grained lifetime analysis:

```rust
fn example() {
    let mut x = 5;
    let y = &x;        // Borrow starts
    println!("{}", y); // Last use of y
    x = 6;             // This now works! Borrow ended at last use
}
```

### 3.3 Borrow Checking Algorithm

```
1. Build MIR from HIR
2. Compute liveness for each variable
3. For each borrow:
   a. Determine borrow's scope (NLL regions)
   b. Check no conflicting accesses in scope
4. Report errors with source locations
```

### 3.4 Polonius (Next-Gen Borrow Checker)

Polonius uses Datalog-style analysis on MIR:
- More precise lifetime analysis
- Better error messages
- Handles more complex patterns

---

## 4. MIR Optimization

### 4.1 Optimization Passes

| Pass | Description |
|------|-------------|
| Constant propagation | Evaluate constants at compile time |
| Dead code elimination | Remove unreachable code |
| Inline | Inline small functions |
| InstCombine | Combine instructions |
| SimplifyLocals | Remove unused locals |
| CopyPropagation | Eliminate redundant copies |

### 4.2 Optimization Pipeline

```
MIR Building
    ↓
Validation
    ↓
Const Qualification
    ↓
Borrow Checking
    ↓
Optimization Passes
    ↓
Monomorphization
    ↓
LLVM IR Generation
```

---

## 5. MIR Code Locations

### 5.1 Key Crates

```
compiler/rustc_middle/src/mir/    - MIR data structures
compiler/rustc_mir_build/         - HIR → MIR lowering
compiler/rustc_mir_transform/     - Optimization passes
compiler/rustc_mir_dataflow/      - Dataflow analysis
compiler/rustc_borrowck/          - Borrow checking
```

---

## 6. Miri: MIR Interpreter

### 6.1 What is Miri?

Miri interprets MIR to detect undefined behavior:
- Finds bugs in unsafe code
- Memory model validation
- Used for const evaluation

### 6.2 Memory Models

| Model | Description |
|-------|-------------|
| Stacked Borrows | Stack-based permission tracking (default) |
| Tree Borrows | Tree-based for complex aliasing |

---

## 7. Recommendations for Aria

### 7.1 MIR-Style IR for Aria

```
Aria HIR → Aria MIR → Target (LLVM/Cranelift/WASM)

Aria MIR should:
- Be CFG-based like Rust MIR
- Represent ownership explicitly
- Track effects in the IR
- Support borrow checking
```

### 7.2 Key Design Decisions

| Decision | Recommendation |
|----------|----------------|
| IR Level | CFG-based mid-level (like MIR) |
| Ownership | Explicit in IR nodes |
| Effects | Annotate blocks/functions |
| Lifetimes | Region-based tracking |

### 7.3 Proposed Aria MIR Structure

```aria
# Conceptual Aria MIR
AriaMIR {
  functions: Map[FunctionId, MIRFunction]
}

MIRFunction {
  locals: Array[LocalDecl]     # Variable declarations
  blocks: Array[BasicBlock]    # CFG blocks
  effects: Set[Effect]         # Function effects
}

BasicBlock {
  statements: Array[Statement]
  terminator: Terminator
}

Statement =
  | Assign(place, rvalue)
  | StorageLive(local)
  | StorageDead(local)
  | EffectOp(effect, args)    # Effect operations

Terminator =
  | Goto(target)
  | SwitchInt(discr, targets)
  | Call(fn, args, dest, effects)
  | Return
  | HandleEffect(effect, handler, body)  # Effect handlers
```

### 7.4 Ownership in Aria MIR

```aria
# Track ownership state per place
OwnershipState = Owned | Borrowed(region) | Moved

# Statements update ownership
Assign(place, Move(source))  # source → Moved
Assign(place, Borrow(source)) # source → Borrowed(r)
Drop(place)                   # place → Moved
```

---

## 8. Key Resources

1. [The MIR - Rust Compiler Development Guide](https://rustc-dev-guide.rust-lang.org/mir/index.html)
2. [RFC 1211: MIR](https://rust-lang.github.io/rfcs/1211-mir.html)
3. [Introducing MIR - Rust Blog](https://blog.rust-lang.org/2016/04/19/MIR/)
4. [Miri Interpreter](https://github.com/rust-lang/miri)
5. [Inside the Borrow Checker](https://medium.com/@bugsybits/inside-the-borrow-checker-how-rust-validates-lifetimes-in-mir-721dce48a8ab)

---

## 9. Open Questions

1. How do we represent effect handlers in MIR?
2. Should Aria MIR have explicit lifetime annotations?
3. What optimization passes are essential for Aria?
4. How do we handle LLM-suggested optimizations at MIR level?
