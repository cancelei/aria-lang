# ARIA-M17-02: PubGrub Version Resolution

**Task ID**: ARIA-M17-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Research PubGrub version resolution algorithm

---

## Executive Summary

PubGrub is a version resolution algorithm using Conflict-Driven Clause Learning (CDCL) from SAT solvers. Created for Dart's pub package manager, it's now used by Cargo, uv, and others. This research analyzes its algorithm for Aria's dependency resolver.

---

## 1. Overview

### 1.1 The Dependency Resolution Problem

Given:
- Root package with dependencies
- Each dependency specifies version constraints
- Each version may have its own dependencies

Find:
- A set of package versions satisfying all constraints
- Or prove no solution exists (with explanation)

### 1.2 Why It's Hard

- **NP-complete**: Can encode SAT problems
- **Combinatorial explosion**: Exponential search space
- **Error messages**: Users need understandable failures

### 1.3 PubGrub's Innovation

Based on CDCL (Conflict-Driven Clause Learning):
- Learn from conflicts to prune search space
- Provide clear error explanations
- Fast in practice despite NP-completeness

---

## 2. Traditional Approaches

### 2.1 Backtracking (pip, early Cargo)

```
resolve(packages):
  for each undecided package:
    for each compatible version:
      try selecting this version
      if resolve(remaining) succeeds:
        return solution
      else:
        backtrack
  return failure
```

**Problem**: Re-explores same conflicts repeatedly

### 2.2 SAT Solvers (Composer, libsolv)

Convert to Boolean SAT formula:
- Variables: (package, version) pairs
- Clauses: Constraints

**Problem**: Opaque error messages ("UNSAT")

### 2.3 Performance Comparison

| Approach | Speed | Error Messages |
|----------|-------|----------------|
| Backtracking | Slow (exponential) | Poor |
| SAT Solver | Fast | Very poor |
| PubGrub | Fast | Excellent |

---

## 3. PubGrub Algorithm

### 3.1 Core Concepts

**Incompatibility**: A set of terms that cannot all be true simultaneously

```
# Example incompatibilities
{foo >=1.0, bar <2.0}  # Can't have both
{foo >=2.0, <no solution>}  # foo >=2.0 impossible
```

**Partial Solution**: Current version selections + derivations

**Term**: Package name + version range (positive or negative)

### 3.2 Algorithm Phases

```
PubGrub:
  loop:
    1. Unit Propagation
       - Apply known incompatibilities
       - Derive new facts
       - Check for contradictions

    2. Decision Making
       - If all packages decided: return solution
       - Pick undecided package
       - Select a version

    3. Conflict Resolution (if contradiction)
       - Analyze conflict
       - Derive new incompatibility (root cause)
       - Backjump to relevant decision
```

### 3.3 Unit Propagation

```
# If we have incompatibility {foo >=1.0, bar >=2.0}
# And solution contains foo >=1.0
# Then we must NOT have bar >=2.0
# Add "bar <2.0" to solution
```

### 3.4 Conflict Resolution

```
# When contradiction found:
1. Find incompatibility causing it
2. Find the decision that led here
3. Resolve: combine incompatibilities
4. Learn: record new incompatibility
5. Backjump to earlier decision

# Key insight: learned incompatibility prevents
# revisiting same failed path
```

---

## 4. Error Messages

### 4.1 Traditional Resolver

```
Error: Could not resolve dependencies
```

### 4.2 PubGrub's Derivation Chain

```
Because foo >=1.0.0 depends on bar >=2.0.0
and bar >=2.0.0 depends on baz >=3.0.0
and baz >=3.0.0 requires Python >=3.8
and your Python version is 3.7
foo >=1.0.0 is not compatible with your environment.

Possible solutions:
1. Upgrade Python to >=3.8
2. Use foo <1.0.0
```

### 4.3 How It Works

- Every derived fact has a cause (incompatibilities)
- Conflict resolution builds derivation tree
- Tree can be printed as human-readable explanation

---

## 5. Implementations

### 5.1 Rust (pubgrub-rs)

```rust
use pubgrub::{
    resolve, Range, OfflineDependencyProvider,
    PackageResolutionStatistics,
};

// Define dependency provider
let mut provider = OfflineDependencyProvider::new();
provider.add_dependencies("root", "1.0.0", vec![
    ("foo", Range::from(1..2)),
]);
provider.add_dependencies("foo", "1.5.0", vec![
    ("bar", Range::from(2..3)),
]);

// Resolve
match resolve(&provider, "root", "1.0.0") {
    Ok(solution) => println!("{:?}", solution),
    Err(error) => println!("Error: {}", error),
}
```

### 5.2 Python (uv)

uv uses PubGrub for fast Python package resolution:
- 10-100x faster than pip
- Clear error messages
- Handles complex constraint scenarios

### 5.3 Go (pubgrub-go)

2025 implementation with:
- Full CDCL solver
- Learned clause optimization
- Derivation tree generation

---

## 6. Performance Characteristics

### 6.1 Best Case

- Linear in number of packages
- Most real-world scenarios

### 6.2 Worst Case

- Exponential (inherent to NP-completeness)
- Rare in practice

### 6.3 Optimizations

| Optimization | Effect |
|--------------|--------|
| Prioritize recent versions | Fewer backtracks |
| Check 1/61 of global queue | Go scheduler trick |
| Learned clauses | Avoid repeated conflicts |
| Early termination | Stop when solution found |

---

## 7. Comparison with Other Resolvers

### 7.1 Cargo (Pre-PubGrub)

- Used backtracking
- Could take minutes for complex graphs
- Poor error messages

### 7.2 Cargo (Post-PubGrub)

- Adopted PubGrub in 2020s
- Sub-second resolution typically
- Clear conflict explanations

### 7.3 npm/pnpm

- Different strategy: allow duplicates
- Multiple versions of same package OK
- Avoids resolution complexity but increases size

---

## 8. Recommendations for Aria

### 8.1 Use PubGrub-Based Resolution

```aria
# Aria resolver (conceptual)
Resolver {
  fn resolve(root: Package, deps: Map[String, VersionReq]) -> Result[Solution, ResolutionError]

  # Returns human-readable error chain
  fn explain_conflict(error: ResolutionError) -> String
}
```

### 8.2 Incompatibility Representation

```aria
# Core data structures
struct Term {
  package: String
  range: VersionRange
  positive: Bool  # true = require, false = forbid
}

struct Incompatibility {
  terms: Set[Term]
  cause: IncompatibilityCause
}

enum IncompatibilityCause {
  Root                    # Root package requirement
  Dependency(Package)     # Package's dependencies
  NoVersions              # No matching versions exist
  Conflict(Incompatibility, Incompatibility)  # Derived
}
```

### 8.3 Error Message Format

```aria
# Aria resolution error example
Error: Cannot resolve dependencies

Because my-project depends on http >=2.0.0
and http >=2.0.0 depends on tls >=1.2.0
and tls >=1.2.0 is not compatible with platform "wasm32"
http >=2.0.0 cannot be used with target "wasm32".

Suggestions:
  1. Use http >=1.0.0, <2.0.0 which doesn't require tls
  2. Change target from "wasm32" to "native"
```

### 8.4 Lock File Integration

```aria
# Resolution flow
fn resolve_with_lock(manifest: Manifest, lock: Option[LockFile]) -> Resolution
  if lock.is_some and lock_is_valid(manifest, lock.unwrap())
    # Fast path: use locked versions
    return lock.unwrap().to_resolution()
  else
    # Full resolution
    resolution = pubgrub_resolve(manifest)
    write_lock_file(resolution)
    return resolution
  end
end
```

### 8.5 Effect Constraints

```aria
# Aria-specific: effects as constraints
struct EffectConstraint {
  package: String
  required_effects: Set[Effect]
}

# Resolution must ensure effect compatibility
fn check_effect_compatibility(solution: Solution) -> Result[(), EffectError]
  for (pkg, version) in solution
    for dep in pkg.dependencies
      if not dep.effects.subset_of(pkg.declared_effects)
        return Err(EffectError.undeclared(dep, pkg))
      end
    end
  end
  Ok(())
end
```

---

## 9. Key Resources

1. [PubGrub: Next-Generation Version Solving](https://nex3.medium.com/pubgrub-2fb6470504f)
2. [Dart's Solver Documentation](https://github.com/dart-lang/pub/blob/master/doc/solver.md)
3. [pubgrub-rs](https://github.com/pubgrub-rs/pubgrub)
4. [uv Resolver Internals](https://docs.astral.sh/uv/reference/internals/resolver/)
5. [CDCL Algorithm Overview](https://en.wikipedia.org/wiki/Conflict-driven_clause_learning)

---

## 10. Open Questions

1. How do we handle effect constraints in resolution?
2. Should we support "allows" (npm-style duplicate versions)?
3. What heuristics for version prioritization?
4. How do we handle platform-specific dependencies?
