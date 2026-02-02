# ARIA-M04-01: Eiffel's Design by Contract Study

**Task ID**: ARIA-M04-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Deep dive into original DBC implementation

---

## Executive Summary

Design by Contract (DbC) was pioneered by Bertrand Meyer in Eiffel, providing formal specifications for software components. This research analyzes Eiffel's contract semantics, inheritance rules, and runtime/static checking to inform Aria's contract system design.

---

## 1. Design by Contract Fundamentals

### 1.1 The Contract Metaphor

Software contracts mirror legal contracts:
- **Client obligations** (preconditions): What caller must ensure
- **Supplier obligations** (postconditions): What callee guarantees
- **Invariants**: Properties maintained throughout object lifetime

### 1.2 Benefits

| Benefit | Description |
|---------|-------------|
| **Correctness** | Formal specification catches bugs |
| **Documentation** | Contracts are executable docs |
| **Debugging** | Contract violations pinpoint bugs |
| **Testing** | Contracts guide test generation |

---

## 2. Eiffel Contract Syntax

### 2.1 Preconditions (`require`)

```eiffel
put (x: ELEMENT; i: INTEGER)
    -- Insert x at position i
  require
    valid_index: i >= 1 and i <= count + 1
    not_full: count < capacity
  do
    -- implementation
  end
```

**Semantics**:
- Evaluated before routine executes
- Violation = **client bug**
- Routine may assume precondition holds

### 2.2 Postconditions (`ensure`)

```eiffel
put (x: ELEMENT; i: INTEGER)
  require
    valid_index: i >= 1 and i <= count + 1
  do
    -- implementation
  ensure
    count_increased: count = old count + 1
    item_inserted: item(i) = x
  end
```

**Key Feature: `old` expression**
- `old expr` captures value at routine entry
- Only valid in postconditions
- Enables expressing change

### 2.3 Class Invariants (`invariant`)

```eiffel
class BOUNDED_QUEUE [G]
  -- ...
invariant
  count_non_negative: count >= 0
  count_bounded: count <= capacity
  consistent_bounds: capacity > 0
end
```

**Semantics**:
- Must hold after object creation
- Must hold before and after every public routine
- Temporarily violated within routine body

### 2.4 Loop Invariants and Variants

```eiffel
from
  i := 1
invariant
  partial_sum: sum = sum_of_first(i - 1)
variant
  remaining: count - i + 1
until
  i > count
loop
  sum := sum + item(i)
  i := i + 1
end
```

- **Loop invariant**: True on each iteration
- **Loop variant**: Decreasing integer (ensures termination)

---

## 3. Contract Semantics

### 3.1 When Contracts Are Checked

| Contract Type | Check Point |
|---------------|-------------|
| Precondition | Before routine entry |
| Postcondition | After routine exit (normal) |
| Class invariant | After creation, before/after public routines |
| Loop invariant | Before first iteration, after each iteration |
| Check instruction | At that point in code |

### 3.2 Exception Behavior

```eiffel
do
  -- If exception occurs, postcondition NOT checked
  -- Invariant must still hold (rescue clause restores it)
rescue
  -- Restore invariant here
  retry -- optional: try again
end
```

### 3.3 Contract Assertion Levels

Eiffel supports compilation options per class:

| Level | Checks Enabled |
|-------|----------------|
| **no** | None (production optimization) |
| **require** | Preconditions only |
| **ensure** | Pre + postconditions |
| **invariant** | Pre + post + invariants |
| **all** | All assertions including loops |

---

## 4. Inheritance and Contracts

### 4.1 Liskov Substitution Principle

Subclasses must honor parent contracts:
- **Preconditions**: May be **weakened** (accept more)
- **Postconditions**: May be **strengthened** (guarantee more)
- **Invariants**: Must be **preserved**

### 4.2 Eiffel Implementation

```eiffel
class SAVINGS_ACCOUNT inherit ACCOUNT
  redefine withdraw end

feature
  withdraw (amount: REAL)
    require else  -- Weakening: OR with parent precondition
      positive_amount: amount > 0
    do
      -- implementation
    ensure then  -- Strengthening: AND with parent postcondition
      fee_charged: balance <= old balance - amount
    end
end
```

**Keyword Semantics**:
- `require else`: New precondition OR parent precondition
- `ensure then`: New postcondition AND parent postcondition

### 4.3 Invariant Inheritance

```eiffel
class CHILD inherit PARENT

invariant
  child_property: some_condition
  -- Parent invariant automatically included
end
```

Child invariant = Child conditions AND Parent invariant

---

## 5. Runtime vs Static Checking

### 5.1 Eiffel's Approach: Runtime Checking

**Advantages**:
- Works with any expression
- No annotation burden beyond contracts
- Catches violations at runtime

**Disadvantages**:
- Bugs found late (at runtime)
- Performance overhead
- Not all paths tested

### 5.2 Static Verification (AutoProof)

Eiffel has experimental static verification:

```eiffel
note
  status: verified

class VERIFIED_STACK [G]
  -- Verified by AutoProof using Z3/CVC4
end
```

**Challenges**:
- Requires careful annotation
- May not handle all contracts
- Verification can timeout

### 5.3 Hybrid Approach (Recommended for Aria)

| Contract Complexity | Verification |
|--------------------|--------------|
| Simple bounds | Static |
| Type invariants | Static |
| Complex invariants | Runtime |
| Quantified properties | Property testing |

---

## 6. Contract Patterns

### 6.1 Defensive Programming Elimination

```eiffel
-- WITHOUT contracts (defensive):
put (x: ELEMENT; i: INTEGER)
  do
    if i < 1 or i > count + 1 then
      raise Invalid_index_error
    end
    -- actual work
  end

-- WITH contracts:
put (x: ELEMENT; i: INTEGER)
  require
    valid_index: i >= 1 and i <= count + 1
  do
    -- just the actual work
  end
```

### 6.2 Query-Command Separation

```eiffel
-- Query (no side effects)
count: INTEGER
  -- Number of elements

-- Command (may change state)
put (x: ELEMENT)
  ensure
    count_increased: count = old count + 1
```

### 6.3 Design by Contract with Inheritance

```eiffel
deferred class COMPARABLE
feature
  is_less (other: like Current): BOOLEAN
    deferred
    ensure
      asymmetric: Result implies not other.is_less (Current)
    end
end
```

---

## 7. Contract Error Messages

### 7.1 Eiffel's Approach

```
*** PRECONDITION VIOLATION ***
Class: BOUNDED_QUEUE
Feature: put
Tag: valid_index
Expression: i >= 1 and i <= count + 1
Call stack:
  1. MAIN.execute line 42
  2. QUEUE_MANAGER.add line 28
  3. BOUNDED_QUEUE.put line 15
```

### 7.2 Best Practices for Messages

- **Tag your assertions**: `valid_index:` not just `require i >= 1`
- **One condition per tag**: Easier to identify failures
- **Meaningful names**: `count_non_negative` not `cnn`

---

## 8. Recommendations for Aria

### 8.1 Contract Syntax

```aria
fn binary_search(arr, target) -> Int?
  requires
    arr.sorted?                 # Simple, readable
    arr.length > 0
  ensures |result|              # Named result binding
    result.none? or arr[result.unwrap] == target
  do
    # implementation
  end
end
```

### 8.2 Class Invariants

```aria
class BoundedQueue[T]
  @items: Array[T]
  @capacity: Int

  invariant
    @items.length <= @capacity
    @capacity > 0
  end

  fn push(item: T)
    requires @items.length < @capacity
    ensures @items.length == old(@items.length) + 1
  end
end
```

### 8.3 Verification Strategy

| Contract Type | Verification Method |
|---------------|---------------------|
| Type bounds (x > 0) | Static (SMT solver) |
| Null checks | Static (type system) |
| Simple invariants | Static when possible |
| Complex invariants | Runtime checking |
| Quantified (forall) | Property-based testing |
| Performance contracts | Benchmarking |

### 8.4 Inheritance Rules

```aria
class SavingsAccount < Account
  fn withdraw(amount)
    # Weakened precondition (accepts more)
    requires amount > 0  # Less restrictive than parent

    # Strengthened postcondition (guarantees more)
    ensures @balance <= old(@balance) - amount
  end
end
```

### 8.5 Integration with Testing

```aria
fn binary_search(arr, target) -> Int?
  # ... contracts ...

  # Embedded examples (become tests)
  examples
    binary_search([1,2,3,4,5], 3) == Some(2)
    binary_search([1,2,3], 4) == None
  end

  # Property tests generated from contracts
  property
    forall arr: sorted_array, x: Int
      arr.contains?(x) implies binary_search(arr, x).some?
  end
end
```

---

## 9. Key Resources

1. **Meyer** - "Object-Oriented Software Construction" (1997)
2. **Meyer** - "Applying Design by Contract" (IEEE Computer 1992)
3. **Eiffel Documentation** - eiffel.org/doc
4. **AutoProof** - Eiffel static verification tool
5. **Findler & Felleisen** - "Contracts for Higher-Order Functions"

---

## 10. Open Questions

1. How do contracts interact with effect inference?
2. What's the right balance of static vs runtime checking?
3. How do we generate good property tests from contracts?
4. What's the syntax for `old` in concurrent contexts?

---

## Appendix: Full Eiffel Contract Example

```eiffel
class BANK_ACCOUNT

create
  make

feature {NONE} -- Initialization

  make (initial_balance: REAL)
    require
      non_negative: initial_balance >= 0
    do
      balance := initial_balance
    ensure
      balance_set: balance = initial_balance
    end

feature -- Access

  balance: REAL
      -- Current account balance

feature -- Operations

  deposit (amount: REAL)
    require
      positive_amount: amount > 0
    do
      balance := balance + amount
    ensure
      balance_increased: balance = old balance + amount
    end

  withdraw (amount: REAL)
    require
      positive_amount: amount > 0
      sufficient_funds: amount <= balance
    do
      balance := balance - amount
    ensure
      balance_decreased: balance = old balance - amount
    end

  transfer (amount: REAL; target: BANK_ACCOUNT)
    require
      positive_amount: amount > 0
      sufficient_funds: amount <= balance
      different_accounts: target /= Current
    do
      withdraw (amount)
      target.deposit (amount)
    ensure
      balance_decreased: balance = old balance - amount
      target_increased: target.balance = old target.balance + amount
    end

invariant
  non_negative_balance: balance >= 0

end
```
