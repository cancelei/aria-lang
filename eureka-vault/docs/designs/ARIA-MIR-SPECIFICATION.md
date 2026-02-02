# Aria MIR Specification

**Version**: 1.0
**Date**: 2026-01-26
**Status**: Implementation Complete

## Overview

MIR (Mid-level Intermediate Representation) is Aria's CFG-based intermediate representation that sits between the AST and code generation. It provides a lower-level view of the program suitable for optimization and multi-target compilation.

```
Source → AST → [type check] → [lower] → MIR → [optimize] → Codegen
                                              ↓
                                        Cranelift IR → Native
                                        WASM bytecode → WASM
```

## Design Principles

1. **Explicit Control Flow**: All control flow is represented as basic blocks with explicit terminators
2. **Simple Operations**: Complex expressions are broken down into primitive operations
3. **Ownership Tracking**: Move vs copy semantics are explicit in the IR
4. **Effect Awareness**: Effect rows and evidence are first-class in function signatures
5. **Type-Resolved**: All types are fully resolved (no inference variables in output)

## Program Structure

### MirProgram

The top-level container for a compiled program.

```
MirProgram {
    functions: Map<FunctionId, MirFunction>
    structs: Map<StructId, MirStruct>
    enums: Map<EnumId, MirEnum>
    effects: Map<EffectId, MirEffect>
    handlers: Map<HandlerId, MirHandler>
    entry: Option<FunctionId>
    strings: Vec<String>  // interned string literals
}
```

### MirFunction

A function definition with CFG, locals, and effect information.

```
MirFunction {
    name: String
    params: Vec<Local>
    return_ty: MirType
    locals: Vec<LocalDecl>
    blocks: Vec<BasicBlock>
    effect_row: EffectRow
    evidence_params: Vec<EvidenceParam>
    evidence_layout: EvidenceLayout
    linkage: Local | External | Builtin(kind)
}
```

**Invariants**:
- `Local(0)` is always the return place
- `blocks[0]` is always the entry block
- Every block must have a terminator (except unreachable blocks)

## Type System

### MirType

```
MirType =
    // Primitives
    | Unit | Bool | Char
    | Int | Int8 | Int16 | Int32 | Int64
    | UInt | UInt8 | UInt16 | UInt32 | UInt64
    | Float | Float32 | Float64
    | String

    // Compound
    | Array(MirType)
    | Tuple(Vec<MirType>)
    | Map(MirType, MirType)
    | Optional(MirType)
    | Result(MirType, MirType)

    // References
    | Ref(MirType)       // immutable borrow
    | RefMut(MirType)    // mutable borrow

    // Named
    | Struct(StructId)
    | Enum(EnumId)

    // Function types
    | FnPtr { params: Vec<MirType>, ret: MirType }
    | Closure { params: Vec<MirType>, ret: MirType }

    // Type parameters (for generics)
    | TypeVar(TypeVarId)    // inference variable
    | TypeParam(String)     // named parameter (T, U, etc.)
    | Generic { name: String, args: Vec<MirType> }

    // Control flow
    | Never  // diverging (panic, infinite loop)
```

## Control Flow Graph

### BasicBlock

```
BasicBlock {
    id: BlockId
    statements: Vec<Statement>
    terminator: Terminator
}
```

### Statement

Statements perform operations without transferring control flow.

```
Statement =
    | Assign(Place, Rvalue)      // place = rvalue
    | StorageLive(Local)         // variable enters scope
    | StorageDead(Local)         // variable leaves scope
    | Nop                        // no operation
```

### Terminator

Terminators transfer control flow between blocks.

```
Terminator =
    | Goto { target: BlockId }
    | SwitchInt { discr: Operand, targets: SwitchTargets }
    | Call { func: Operand, args: Vec<Operand>, dest: Place, target: Option<BlockId> }
    | Return
    | Unreachable
    | Drop { place: Place, target: BlockId }
    | Assert { cond: Operand, expected: bool, msg: String, target: BlockId }
```

**SwitchTargets** maps integer values to blocks with a default:
```
SwitchTargets {
    targets: Vec<(i128, BlockId)>  // value → block
    otherwise: BlockId              // default block
}
```

## Memory Model

### Place

A place represents a memory location that can be read from or written to.

```
Place {
    local: Local                  // base local variable
    projection: Vec<PlaceElem>    // field/index/deref chain
}
```

### PlaceElem

```
PlaceElem =
    | Field(u32)           // struct field access
    | Index(Local)         // array/tuple index (runtime)
    | ConstIndex(u32)      // array/tuple index (compile-time)
    | Deref                // pointer dereference
    | Downcast(u32)        // enum variant cast
```

### Operand

An operand is a value that can be used in an operation.

```
Operand =
    | Copy(Place)     // copy value from place
    | Move(Place)     // move value from place (invalidates source)
    | Constant(Constant)
```

### Rvalue

An rvalue produces a value that can be assigned to a place.

```
Rvalue =
    | Use(Operand)                              // simple use
    | BinaryOp(BinOp, Operand, Operand)        // binary operation
    | UnaryOp(UnOp, Operand)                   // unary operation
    | Ref(Place)                               // create immutable borrow
    | RefMut(Place)                            // create mutable borrow
    | Aggregate(AggregateKind, Vec<Operand>)   // construct composite
    | Discriminant(Place)                      // get enum discriminant
    | Len(Place)                               // get collection length
    | Cast(CastKind, Operand, MirType)         // type cast
```

### Binary Operations

```
BinOp =
    // Arithmetic
    | Add | Sub | Mul | Div | Rem
    // Comparison
    | Eq | Ne | Lt | Le | Gt | Ge
    // Logical
    | And | Or
    // Bitwise
    | BitAnd | BitOr | BitXor | Shl | Shr
    // String
    | Concat
```

### Unary Operations

```
UnOp =
    | Neg      // arithmetic negation
    | Not      // logical/bitwise not
```

## Effect System in MIR

### EffectRow

Represents the set of effects a function may perform.

```
EffectRow {
    effects: Vec<EffectType>
    is_open: bool  // true if row can contain additional effects
}
```

### EffectType

```
EffectType {
    id: EffectId
    name: String
    type_params: Vec<MirType>
}
```

### EvidenceSlot

Evidence slots link effect operations to their handlers.

```
EvidenceSlot =
    | Static(usize)   // compile-time known slot
    | Dynamic(Local)  // runtime slot in local variable
```

### Effect Statements

```
EffectStatementKind =
    | InstallHandler { handler: HandlerId, evidence_slot: EvidenceSlot, effect: EffectType }
    | PerformEffect { effect: EffectType, operation: OperationId, args: Vec<Operand>, dest: Place }
    | CaptureContunuation { dest: Place }
    | FfiBarrier { strategy: FfiBarrierStrategy, blocked_effects: Vec<EffectType> }
```

### Effect Terminators

```
EffectTerminatorKind =
    | Yield { effect: EffectType, operation: OperationId, args: Vec<Operand>, continuation: Operand }
    | Resume { continuation: Operand, value: Operand, target: BlockId }
    | Handle { body: BlockId, handler: HandlerId, normal_return: BlockId, effect_return: BlockId }
```

### EffectClassification

Optimizes effect handling based on usage patterns.

```
EffectClassification =
    | General            // may capture continuation
    | TailResumptive     // always resumes immediately, tail position
    | Stateless          // no handler state
    | Abortive           // never resumes (like exceptions)
```

## Struct and Enum Definitions

### MirStruct

```
MirStruct {
    id: StructId
    name: String
    fields: Vec<MirField>
    type_params: Vec<String>
}

MirField {
    name: String
    ty: MirType
    is_public: bool
}
```

### MirEnum

```
MirEnum {
    id: EnumId
    name: String
    variants: Vec<MirVariant>
    type_params: Vec<String>
}

MirVariant {
    name: String
    fields: Vec<MirType>  // tuple-style variant
}
```

## Built-in Functions

Functions with `Linkage::Builtin(kind)` are handled specially by codegen:

| Kind | Signature | Description |
|------|-----------|-------------|
| Print | `(args...) -> Unit` | Print without newline |
| Println | `(args...) -> Unit` | Print with newline |
| Len | `(T) -> Int` | Collection length |
| Assert | `(Bool, String?) -> Unit` | Assertion |
| Panic | `(String) -> Never` | Panic and abort |
| Abs | `(Num) -> Num` | Absolute value |
| Min/Max | `(T, T) -> T` | Minimum/Maximum |
| Sqrt/Sin/Cos/... | `(Float) -> Float` | Math functions |
| Push/Pop | `(Array<T>, T?) -> ...` | Array operations |

## Lowering from AST

The `LoweringContext` transforms AST to MIR:

1. **Functions**: Each AST function becomes a `MirFunction`
2. **Expressions**: Decomposed into temporaries and simple operations
3. **Control Flow**: `if`/`match`/`loop` become basic blocks with terminators
4. **Effects**: Effect annotations become evidence parameters

### Example Lowering

```aria
fn add(x: Int, y: Int) -> Int
    x + y
end
```

Becomes:

```
fn add(_1: Int, _2: Int) -> Int {
    bb0:
        _0 = BinaryOp(Add, Copy(_1), Copy(_2))
        return
}
```

## Code Generation Targets

MIR supports multiple backends:

| Target | Backend | Output |
|--------|---------|--------|
| x86_64 | Cranelift | Native object file |
| aarch64 | Cranelift | Native object file |
| wasm32 | Custom WASM | .wasm binary |

## Implementation Files

| File | Purpose |
|------|---------|
| `crates/aria-mir/src/mir.rs` | Core MIR data structures |
| `crates/aria-mir/src/lower.rs` | AST to MIR lowering |
| `crates/aria-mir/src/lower_expr.rs` | Expression lowering |
| `crates/aria-mir/src/lower_stmt.rs` | Statement lowering |
| `crates/aria-mir/src/lower_pattern.rs` | Pattern lowering |
| `crates/aria-mir/src/pretty.rs` | MIR pretty printing |
| `crates/aria-codegen/src/cranelift_backend.rs` | Native codegen |
| `crates/aria-codegen/src/wasm_backend.rs` | WASM codegen |

## Future Enhancements

1. **Optimization Passes**: Constant folding, dead code elimination, inlining
2. **LLM Integration Points**: Hook for AI-suggested optimizations
3. **HIR Layer**: Higher-level IR for trait resolution
4. **LIR Layer**: Lower-level IR closer to machine code
