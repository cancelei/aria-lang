# Aria Runtime Library - Quick Reference

## Build

```bash
# Debug build
cargo build -p aria-runtime

# Release build
cargo build -p aria-runtime --release

# Run tests
cargo test -p aria-runtime

# Output
# Debug:   target/debug/libariaruntime.a (29MB)
# Release: target/release/libariaruntime.a (22MB)
```

## Link with Compiled Code

```bash
# From C/C++
gcc your_code.o target/debug/libariaruntime.a -lpthread -ldl -lm -o program

# From Rust
# Add to Cargo.toml:
# [dependencies]
# aria-runtime = { path = "../aria-runtime" }
```

## API Quick Reference

### Memory Management
```c
void* aria_alloc(size_t size);              // Allocate memory
void  aria_free(void* ptr);                 // Free memory
void* aria_realloc(void* ptr, size_t size); // Reallocate memory
```

### String Operations
```c
typedef struct {
    uint8_t* data;
    size_t len;
    size_t capacity;
} AriaString;

AriaString* aria_string_new(const uint8_t* data, size_t len);
AriaString* aria_string_concat(AriaString* a, AriaString* b);
AriaString* aria_string_slice(AriaString* s, size_t start, size_t end);
bool aria_string_eq(AriaString* a, AriaString* b);
size_t aria_string_len(AriaString* s);
```

### Array Operations
```c
typedef struct {
    void* data;
    size_t length;
    size_t capacity;
    size_t elem_size;
} AriaArray;

AriaArray* aria_array_new(size_t elem_size, size_t capacity);
void aria_array_push(AriaArray* arr, const void* elem);
const void* aria_array_get(AriaArray* arr, size_t index);
size_t aria_array_len(AriaArray* arr);
```

### HashMap Operations
```c
typedef struct AriaHashMap AriaHashMap;

AriaHashMap* aria_hashmap_new(void);
void aria_hashmap_insert(AriaHashMap* map, AriaString* key, int64_t value);
int64_t aria_hashmap_get(AriaHashMap* map, AriaString* key);
```

### I/O Operations
```c
void aria_println(AriaString* s);  // Print with newline
void aria_print(AriaString* s);    // Print without newline
```

### Panic Handler
```c
void aria_panic(const uint8_t* msg, size_t len) __attribute__((noreturn));
```

## Example Usage

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
extern AriaString* aria_string_concat(AriaString* a, AriaString* b);
extern void aria_println(AriaString* s);

int main() {
    // Create strings
    AriaString* hello = aria_string_new((uint8_t*)"Hello, ", 7);
    AriaString* world = aria_string_new((uint8_t*)"World!", 6);

    // Concatenate
    AriaString* greeting = aria_string_concat(hello, world);

    // Print
    aria_println(greeting);  // Output: Hello, World!

    return 0;
}
```

## Exported Symbols

All runtime functions are exported with C linkage and can be called from LLVM-generated code:

```
aria_alloc
aria_free
aria_realloc
aria_string_new
aria_string_concat
aria_string_slice
aria_string_eq
aria_string_len
aria_array_new
aria_array_push
aria_array_get
aria_array_len
aria_hashmap_new
aria_hashmap_insert
aria_hashmap_get
aria_println
aria_print
aria_panic
```

## Common Patterns

### Creating and Printing a String
```c
const char* text = "Hello, Aria!";
AriaString* s = aria_string_new((uint8_t*)text, strlen(text));
aria_println(s);
```

### Working with Arrays
```c
// Create array of i64
AriaArray* arr = aria_array_new(sizeof(int64_t), 10);

// Push elements
int64_t value = 42;
aria_array_push(arr, &value);

// Get element
const int64_t* elem = (const int64_t*)aria_array_get(arr, 0);
printf("Element: %ld\n", *elem);
```

### Using HashMap
```c
AriaHashMap* map = aria_hashmap_new();

AriaString* key = aria_string_new((uint8_t*)"answer", 6);
aria_hashmap_insert(map, key, 42);

int64_t value = aria_hashmap_get(map, key);
printf("Value: %ld\n", value);  // Output: 42
```

## Notes

- All functions are thread-safe at the Rust level but the library is not designed for concurrent access
- Memory management is manual - callers must free allocated memory
- Null pointers are handled gracefully (most functions check for null)
- Panics terminate the program with an error message

## For More Information

- See `README_FFI.md` for detailed API documentation
- See `RUNTIME_IMPLEMENTATION.md` for implementation details
- See `examples/test_runtime.c` for a complete working example
