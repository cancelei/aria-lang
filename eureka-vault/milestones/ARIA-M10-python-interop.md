# Milestone M10: Python Interop

## Overview

Design Aria's Python interoperability system for zero-copy data exchange with Python libraries (NumPy, Pandas, ML frameworks).

## Research Questions

1. How do we achieve zero-copy array sharing with NumPy?
2. Can we call Python libraries without the GIL overhead?
3. How do we handle Python's dynamic typing in Aria's static system?
4. What's the memory ownership model across the boundary?

## Core Innovation Target

```ruby
# Python library import
extern Python from numpy as np
extern Python from pandas as pd
extern Python from sklearn.linear_model as lm

fn analyze_data(data: Array<Float>) -> Array<Float>
  # Zero-copy conversion to NumPy array
  arr = np.array(data)

  # Use sklearn
  model = lm.LinearRegression()
  model.fit(arr.reshape(-1, 1), targets)

  # Zero-copy back to Aria
  Array.from(model.predict(arr))
end
```

## Competitive Analysis Required

| Language | Python Interop | Study Focus |
|----------|----------------|-------------|
| Rust | PyO3 | Safe bindings |
| Julia | PyCall | Zero-copy arrays |
| Mojo | Native | Python superset |
| C++ | pybind11 | Template magic |
| Nim | nimpy | Pragmatic approach |

## Tasks

### ARIA-M10-01: Deep dive into PyO3
- **Description**: Study Rust's PyO3 Python bindings
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, python, pyo3, rust
- **Deliverables**:
  - GIL handling patterns
  - Memory safety across boundary
  - Performance characteristics

### ARIA-M10-02: Study Julia's Python interop
- **Description**: Analyze Julia's zero-copy approach
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, python, julia, zero-copy
- **Deliverables**:
  - Array protocol usage
  - Type conversion patterns
  - Performance benchmarks

### ARIA-M10-03: Research Python C API
- **Description**: Study Python's C extension API
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, python, c-api
- **Deliverables**:
  - API stability analysis
  - GIL-free patterns
  - Reference counting

### ARIA-M10-04: Study NumPy array protocol
- **Description**: Research buffer protocol and array interface
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, python, numpy, arrays
- **Deliverables**:
  - Buffer protocol details
  - Memory layout compatibility
  - Zero-copy requirements

### ARIA-M10-05: Design Python interop system
- **Description**: Design Aria's Python interop
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M10-01, ARIA-M10-03
- **Tags**: research, python, design
- **Deliverables**:
  - Import syntax
  - Type mapping rules
  - Memory ownership model

### ARIA-M10-06: Design zero-copy array bridge
- **Description**: Design zero-copy NumPy interop
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M10-04
- **Tags**: research, python, numpy, design
- **Deliverables**:
  - Array bridge specification
  - Memory layout guarantees
  - Lifetime management

## Success Criteria

- [ ] Python import system designed
- [ ] Zero-copy arrays achievable
- [ ] GIL handling strategy defined
- [ ] Memory ownership clear

## Key Resources

1. PyO3 documentation and source
2. Python C API documentation
3. NumPy array protocol PEP
4. Julia PyCall source
5. Mojo Python interop docs

## Timeline

Target: Q2-Q3 2026

## Related Milestones

- **Depends on**: M09 (C Interop)
- **Enables**: ML/data science use cases
- **Synergy**: M08 (WASM) - Pyodide integration
