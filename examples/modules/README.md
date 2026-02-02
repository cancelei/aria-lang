# Aria Module System Examples

This directory demonstrates the Aria module system with practical examples.

## File Structure

```
examples/modules/
├── math.aria       # Math utilities module
├── utils.aria      # General utilities module
└── main.aria       # Main program importing both modules
```

## Running the Examples

### Check Syntax
```bash
aria parse main.aria
```

### Compile with Module Resolution
```bash
aria build main.aria --lib-path .
```

### Compile Multiple Modules Together
```bash
# The compiler automatically resolves imports
aria build main.aria -L examples/modules
```

## Module Syntax

### Defining a Module (math.aria)

```aria
// Public functions are exported
pub fn add(x: Int, y: Int) -> Int = x + y

pub fn multiply(x: Int, y: Int) -> Int = x * y

// Private function (not exported)
fn internal_helper(x: Int) -> Int = x * 2

// Public constants
pub const PI: Float = 3.14159

// Using other public functions
pub fn square(x: Int) -> Int = multiply(x, x)
```

### Importing from Modules (main.aria)

```aria
// Import specific items
import math::{add, multiply, PI}

// Import from another module
import utils::{max, clamp}

// Use imported items
pub fn main() -> Int {
    let x = add(10, 20)
    let y = multiply(5, 6)
    let z = max(x, y)

    println("Sum: ", x)
    println("Product: ", y)
    println("Max: ", z)

    0
}
```

## Import Styles

### Named Imports
```aria
import math::{add, multiply}
```

### Import All Public Items
```aria
import math::*
```

### Import with Alias
```aria
import math as m
```

### Path Imports
```aria
import std::collections::{HashMap, HashSet}
```

## Visibility

### Public Items
Items marked with `pub` are exported and can be imported by other modules:
```aria
pub fn public_function() -> Int = 42
pub const PUBLIC_CONST: Int = 100
pub struct PublicStruct { x: Int }
```

### Private Items
Items without `pub` are private to the module:
```aria
fn private_helper() -> Int = 42
const INTERNAL_CONSTANT: Int = 100
```

## Module Search Paths

The compiler searches for modules in:
1. Relative to the importing file
2. Paths specified with `-L` flag
3. Current directory

Example:
```bash
aria build main.aria -L ./lib -L ./vendor
```

## Best Practices

1. **One module per file**: Keep modules focused and cohesive
2. **Clear exports**: Only export what's needed for the public API
3. **Avoid circular dependencies**: Structure imports to be acyclic
4. **Use meaningful names**: Module names should reflect their purpose
5. **Group related functionality**: Put related functions in the same module

## Common Patterns

### Utility Module
```aria
// utils.aria
pub fn max(x: Int, y: Int) -> Int = if x > y then x else y
pub fn min(x: Int, y: Int) -> Int = if x < y then x else y
```

### Constants Module
```aria
// constants.aria
pub const MAX_SIZE: Int = 1000
pub const DEFAULT_TIMEOUT: Int = 30
```

### Data Types Module
```aria
// types.aria
pub struct Point {
    pub x: Int,
    pub y: Int
}

pub struct Rectangle {
    pub top_left: Point,
    pub bottom_right: Point
}
```

## Error Handling

### Module Not Found
```
Error: Module not found: nonexistent
```
Solution: Check the module name and search paths

### Circular Dependency
```
Error: Circular dependency detected:
  module A
  -> module B
  -> module A
```
Solution: Restructure your modules to break the cycle

### Private Item Access
```
Error: Item 'helper' is private in module 'utils'
```
Solution: Mark the item as `pub` or don't import it

## Advanced Features

### Re-exports (Future)
```aria
// Re-export items from another module
pub use math::{add, multiply}
```

### Nested Modules (Future)
```aria
module math::advanced {
    pub fn derivative(f: Fn(Float) -> Float) -> Fn(Float) -> Float {
        // ...
    }
}
```

### Conditional Compilation (Future)
```aria
#[cfg(feature = "advanced")]
import math::advanced::*
```
