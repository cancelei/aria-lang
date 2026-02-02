# Aria Runtime Library (FFI)

A minimal runtime library in Rust that provides core functionality for compiled Aria programs. This library is statically linked with Aria programs and provides C-compatible FFI functions.

## Overview

The runtime library provides the following functionality:

1. **Memory Management** - Heap allocation, deallocation, and reallocation
2. **String Operations** - String creation, concatenation, slicing, and comparison
3. **Array Operations** - Dynamic arrays with push, get, and length operations
4. **HashMap Operations** - Hash map with string keys and i64 values
5. **I/O Operations** - Print functions for output
6. **Panic Handler** - Fatal error handling

## Building

The library is configured to build as a static library (`staticlib`) and also as a Rust library (`rlib`):

```bash
# Build debug version
cargo build -p aria-runtime

# Build release version
cargo build -p aria-runtime --release
```

The output static library will be located at:
- Debug: `target/debug/libariaruntime.a`
- Release: `target/release/libariaruntime.a`

## API Reference

### Memory Management

```rust
// Allocate memory (returns null on failure or if size is 0)
pub extern "C" fn aria_alloc(size: usize) -> *mut u8

// Free memory (safe to call with null pointer)
pub extern "C" fn aria_free(ptr: *mut u8)

// Reallocate memory to a new size
pub extern "C" fn aria_realloc(ptr: *mut u8, new_size: usize) -> *mut u8
```

### String Operations

```rust
// String structure
#[repr(C)]
pub struct AriaString {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

// Create a new string from raw bytes
pub extern "C" fn aria_string_new(data: *const u8, len: usize) -> *mut AriaString

// Concatenate two strings
pub extern "C" fn aria_string_concat(
    a: *mut AriaString,
    b: *mut AriaString
) -> *mut AriaString

// Extract a slice from a string
pub extern "C" fn aria_string_slice(
    s: *mut AriaString,
    start: usize,
    end: usize
) -> *mut AriaString

// Compare two strings for equality
pub extern "C" fn aria_string_eq(
    a: *mut AriaString,
    b: *mut AriaString
) -> bool

// Get string length
pub extern "C" fn aria_string_len(s: *mut AriaString) -> usize
```

### Array Operations

```rust
// Array structure
#[repr(C)]
pub struct AriaArray {
    pub data: *mut u8,
    pub length: usize,
    pub capacity: usize,
    pub elem_size: usize,
}

// Create a new array with given element size and capacity
pub extern "C" fn aria_array_new(
    elem_size: usize,
    capacity: usize
) -> *mut AriaArray

// Push an element to the array (grows if needed)
pub extern "C" fn aria_array_push(arr: *mut AriaArray, elem: *const u8)

// Get a pointer to an element at index (returns null if out of bounds)
pub extern "C" fn aria_array_get(
    arr: *mut AriaArray,
    index: usize
) -> *const u8

// Get the length of the array
pub extern "C" fn aria_array_len(arr: *mut AriaArray) -> usize
```

### HashMap Operations

```rust
// HashMap structure (opaque)
#[repr(C)]
pub struct AriaHashMap { /* private */ }

// Create a new empty hash map
pub extern "C" fn aria_hashmap_new() -> *mut AriaHashMap

// Insert a key-value pair
pub extern "C" fn aria_hashmap_insert(
    map: *mut AriaHashMap,
    key: *mut AriaString,
    value: i64
)

// Get a value by key (returns 0 if not found)
pub extern "C" fn aria_hashmap_get(
    map: *mut AriaHashMap,
    key: *mut AriaString
) -> i64
```

### I/O Operations

```rust
// Print a string with newline
pub extern "C" fn aria_println(s: *mut AriaString)

// Print a string without newline
pub extern "C" fn aria_print(s: *mut AriaString)
```

### Panic Handler

```rust
// Panic with a message and exit the program
pub extern "C" fn aria_panic(msg: *const u8, len: usize) -> !
```

## Usage Example

Here's how to link with the runtime library from C or LLVM:

```c
#include <stdint.h>

// Declare runtime functions
extern void* aria_alloc(size_t size);
extern void aria_free(void* ptr);

typedef struct {
    uint8_t* data;
    size_t len;
    size_t capacity;
} AriaString;

extern AriaString* aria_string_new(const uint8_t* data, size_t len);
extern void aria_println(AriaString* s);

int main() {
    // Create a string
    const char* hello = "Hello, Aria!";
    AriaString* s = aria_string_new((uint8_t*)hello, 12);

    // Print it
    aria_println(s);

    return 0;
}
```

Compile and link:

```bash
# Compile your C code
gcc -c your_code.c -o your_code.o

# Link with the runtime
gcc your_code.o -L target/debug -lariaruntime -o program

# Or link directly with the .a file
gcc your_code.o target/debug/libariaruntime.a -o program
```

## Notes

### Memory Management

The current implementation uses Rust's global allocator. The `aria_free` and `aria_realloc` functions have some limitations:

- `aria_free` doesn't properly deallocate memory because we don't track allocation sizes
- `aria_realloc` allocates new memory but doesn't copy old data properly

For production use, consider:
- Using a custom allocator like jemalloc
- Storing allocation metadata (size) alongside allocations
- Implementing proper reference counting or garbage collection

### Safety

All FFI functions are `unsafe` and assume:
- Pointers passed are valid or null
- Data pointed to is valid for the specified length
- Callers manage memory properly

### Thread Safety

The current implementation is **not thread-safe**. All runtime functions should be called from a single thread, or external synchronization should be used.

## Testing

Run the tests with:

```bash
cargo test -p aria-runtime
```

## Integration with Aria Compiler

The Aria compiler's codegen (in `crates/aria-codegen`) should:

1. Link with `libariaruntime.a` during compilation
2. Generate calls to these FFI functions for runtime operations
3. Ensure proper memory management and cleanup

For example, string concatenation in Aria:

```aria
let result = "Hello, " + "World!";
```

Would generate LLVM IR similar to:

```llvm
%str1 = call ptr @aria_string_new(ptr @.str1_data, i64 7)
%str2 = call ptr @aria_string_new(ptr @.str2_data, i64 6)
%result = call ptr @aria_string_concat(ptr %str1, ptr %str2)
call void @aria_println(ptr %result)
```

## Future Enhancements

Planned improvements:

1. **Proper memory management** - Track allocation sizes, implement realloc correctly
2. **Reference counting** - Automatic memory management for strings and arrays
3. **Generic arrays** - Support for different element types
4. **HashMap improvements** - Support for different value types, better hash functions
5. **Error handling** - Return error codes instead of panicking
6. **Thread safety** - Add synchronization primitives
7. **Performance** - Optimize hot paths, use SIMD for string operations
