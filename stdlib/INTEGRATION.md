# Stdlib Integration Guide

This document explains how the pure Aria stdlib integrates with the Aria compiler.

## Architecture

The Aria standard library has two components:

1. **Pure Aria Implementation** (`stdlib/` directory)
   - Written entirely in Aria
   - Source of truth for stdlib functionality
   - Can be modified and extended by users
   - Used by BioFlow and other Aria programs

2. **Embedded Stdlib** (`crates/aria-stdlib/src/std/`)
   - Embedded in the compiler for bootstrapping
   - Copied from the pure Aria implementation
   - Provides fallback when filesystem stdlib is unavailable

## Module Resolution

The Aria compiler resolves modules in this order:

1. **Current directory** - Local modules in the project
2. **Stdlib directory** - Pure Aria stdlib from `stdlib/`
3. **Embedded stdlib** - Built-in fallback

### Search Paths

The module resolver automatically adds these stdlib paths:

- `../stdlib` - Relative to executable (installed version)
- `$PROJECT_ROOT/stdlib` - Development builds
- `/usr/local/lib/aria/stdlib` - System-wide installation

### Example Resolution

```aria
import std::collections::HashMap
```

The compiler will search:
1. `./std/collections/HashMap.aria`
2. `$STDLIB_PATH/collections/HashMap.aria`
3. `$STDLIB_PATH/collections/mod.aria` (module index)
4. Embedded `STDLIB_COLLECTIONS` constant

## Auto-Import Prelude

The prelude (`stdlib/prelude.aria`) is automatically imported into every Aria program:

```aria
# Automatically available:
# - Option, Some, None
# - Result, Ok, Err
# - print, println, read_line
# - panic, assert, to_string
# - Core traits: Default, Clone, Eq, Ord
```

### Disabling Auto-Import

To disable the prelude:

```aria
#![no_prelude]

# Must now explicitly import everything
import std::option::Option
import std::io::println
```

## Updating Embedded Stdlib

When the pure Aria stdlib is updated, sync it to the embedded version:

```bash
# Copy stdlib files to embedded location
cp stdlib/core/*.aria crates/aria-stdlib/src/std/
cp stdlib/collections/*.aria crates/aria-stdlib/src/std/
cp stdlib/io/*.aria crates/aria-stdlib/src/std/

# Rebuild compiler
cargo build --release
```

## Using Stdlib in Projects

### Basic Usage

The stdlib is automatically available:

```aria
# main.aria
fn main()
  # Prelude types work automatically
  let x: Option<Int> = Some(42)
  println("Value: #{x.unwrap()}")
end
```

### Explicit Imports

For non-prelude items:

```aria
import std::collections::HashMap
import std::io::File

fn main()
  let mut map = HashMap::new()
  map.insert("key", "value")
end
```

### Relative Imports

Within the stdlib itself, use relative imports:

```aria
# In stdlib/collections/mod.aria
import "./hashmap.aria" as map
export map::HashMap
```

## Native Builtins

Some stdlib functions delegate to native builtins implemented in the runtime:

### String Builtins
- `__builtin_char_at(s: String, i: Int) -> Char`
- `__builtin_string_slice(s: String, start: Int, end: Int) -> String`
- `__builtin_string_concat(a: String, b: String) -> String`
- `__builtin_to_string(value) -> String`

### Array Builtins
- `__builtin_array_len(arr: [T]) -> Int`
- `__builtin_array_push(arr: [T], value: T)`
- `__builtin_array_pop(arr: [T]) -> Option<T>`

### HashMap Builtins
- `__builtin_hash(value) -> Int`

### I/O Builtins
- `__builtin_print(s: String)`
- `__builtin_println(s: String)`
- `__builtin_eprint(s: String)`
- `__builtin_eprintln(s: String)`
- `__builtin_read_line() -> String`
- `__builtin_read_file(path: String) -> Result<String, String>`
- `__builtin_write_file(path: String, content: String) -> Result<(), String>`

### File Handle Builtins
- `__builtin_file_open(path: String, mode: String) -> Result<Int, String>`
- `__builtin_file_read(handle: Int) -> Result<String, String>`
- `__builtin_file_write(handle: Int, content: String) -> Result<(), String>`
- `__builtin_file_close(handle: Int) -> Result<(), String>`

### Control Flow Builtins
- `__builtin_panic(message: String) -> !`

These builtins are implemented in the Aria runtime (`libariaruntime.a`) and linked automatically.

## Testing

Test stdlib functionality:

```bash
# Run stdlib demo
aria run examples/stdlib_demo.aria

# Run specific stdlib tests
aria test stdlib/core/
aria test stdlib/collections/
aria test stdlib/io/
```

## BioFlow Integration

The stdlib is designed for BioFlow bioinformatics workflows:

```aria
import std::collections::HashMap
import std::io::read_file

fn count_kmers(sequence: String, k: Int) -> HashMap<String, Int>
  let mut counts = HashMap::new()
  let mut i = 0

  while i <= sequence.len() - k
    let kmer = sequence.slice(i, i + k)
    match counts.get(kmer)
      Some(count) -> counts.insert(kmer, count + 1)
      None -> counts.insert(kmer, 1)
    end
    i = i + 1
  end

  counts
end

fn main()
  match read_file("sequences.fasta")
    Ok(content) ->
      let kmers = count_kmers(content, 3)
      println("Found #{kmers.len()} unique 3-mers")
    Err(e) ->
      eprintln("Error: #{e.message()}")
  end
end
```

## Performance Considerations

### Zero-Cost Abstractions

Functional methods like `map`, `filter`, and `fold` compile to efficient loops:

```aria
# High-level code
numbers.map(fn(x) -> x * 2).filter(fn(x) -> x > 10)

# Compiles to efficient loop (no intermediate allocations)
```

### Inline Hints

Performance-critical stdlib functions use inline hints:

```aria
#[inline]
fn unwrap<T>(self: Option<T>) -> T
  # ...
end
```

### Memory Management

The stdlib avoids unnecessary allocations:

- String operations reuse buffers when possible
- Array methods use in-place updates where appropriate
- HashMap uses open addressing for cache locality

## Adding New Stdlib Modules

To add a new module:

1. Create the module file:
   ```bash
   touch stdlib/newmodule/mod.aria
   ```

2. Implement functionality in pure Aria:
   ```aria
   # stdlib/newmodule/mod.aria
   fn new_function() -> Result<(), String>
     # Implementation
   end
   ```

3. Export from main stdlib index:
   ```aria
   # stdlib/mod.aria
   export "./newmodule/mod.aria" as newmodule
   ```

4. Update embedded stdlib:
   ```bash
   cp stdlib/newmodule/*.aria crates/aria-stdlib/src/std/
   ```

5. Add to `lib.rs`:
   ```rust
   // crates/aria-stdlib/src/lib.rs
   pub const STDLIB_NEWMODULE: &str = include_str!("std/newmodule/mod.aria");
   ```

## Troubleshooting

### Module Not Found

If modules aren't resolving:

1. Check search paths:
   ```bash
   aria --print-search-paths
   ```

2. Verify stdlib location:
   ```bash
   ls -la $PROJECT_ROOT/stdlib
   ```

3. Enable verbose module resolution:
   ```bash
   aria run --verbose main.aria
   ```

### Builtin Not Found

If native builtins are missing:

1. Check runtime library is linked:
   ```bash
   aria build --verbose main.aria
   # Should show: linking with libariaruntime.a
   ```

2. Verify builtin is exported:
   ```bash
   nm -g libariaruntime.a | grep __builtin_
   ```

3. Rebuild runtime:
   ```bash
   cd crates/aria-runtime
   cargo build --release
   ```

## Future Enhancements

- [ ] Lazy evaluation with Iterator trait
- [ ] Async I/O support
- [ ] SIMD-optimized array operations
- [ ] Parallel iterators for multi-core BioFlow
- [ ] GPU acceleration for bioinformatics kernels
- [ ] JIT compilation for hot loops
- [ ] Incremental compilation for large projects

## Contributing

To contribute to the stdlib:

1. Write pure Aria (minimize native builtins)
2. Add comprehensive documentation
3. Include usage examples
4. Write tests
5. Update this integration guide

See `stdlib/README.md` for more details.
