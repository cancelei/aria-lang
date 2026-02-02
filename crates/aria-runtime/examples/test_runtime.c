/**
 * Test program for the Aria runtime library
 *
 * This demonstrates how to use the runtime FFI functions from C code.
 * Compile with:
 *   gcc -c test_runtime.c -o test_runtime.o
 *   gcc test_runtime.o ../../target/debug/libariaruntime.a -lpthread -ldl -lm -o test_runtime
 */

#include <stdint.h>
#include <stdio.h>

// Runtime FFI function declarations

// Memory management
extern void* aria_alloc(size_t size);
extern void aria_free(void* ptr);
extern void* aria_realloc(void* ptr, size_t new_size);

// String operations
typedef struct {
    uint8_t* data;
    size_t len;
    size_t capacity;
} AriaString;

extern AriaString* aria_string_new(const uint8_t* data, size_t len);
extern AriaString* aria_string_concat(AriaString* a, AriaString* b);
extern AriaString* aria_string_slice(AriaString* s, size_t start, size_t end);
extern int aria_string_eq(AriaString* a, AriaString* b);
extern size_t aria_string_len(AriaString* s);

// Array operations
typedef struct {
    void* data;
    size_t length;
    size_t capacity;
    size_t elem_size;
} AriaArray;

extern AriaArray* aria_array_new(size_t elem_size, size_t capacity);
extern void aria_array_push(AriaArray* arr, const void* elem);
extern const void* aria_array_get(AriaArray* arr, size_t index);
extern size_t aria_array_len(AriaArray* arr);

// HashMap operations
typedef struct AriaHashMap AriaHashMap;

extern AriaHashMap* aria_hashmap_new(void);
extern void aria_hashmap_insert(AriaHashMap* map, AriaString* key, int64_t value);
extern int64_t aria_hashmap_get(AriaHashMap* map, AriaString* key);

// I/O operations
extern void aria_println(AriaString* s);
extern void aria_print(AriaString* s);

// Panic
extern void aria_panic(const uint8_t* msg, size_t len) __attribute__((noreturn));

int main(void) {
    printf("=== Aria Runtime Library Test ===\n\n");

    // Test 1: Memory allocation
    printf("Test 1: Memory allocation\n");
    void* ptr = aria_alloc(100);
    if (ptr) {
        printf("  ✓ Allocated 100 bytes\n");
        aria_free(ptr);
        printf("  ✓ Freed memory\n");
    } else {
        printf("  ✗ Allocation failed\n");
    }

    // Test 2: String operations
    printf("\nTest 2: String operations\n");
    AriaString* s1 = aria_string_new((uint8_t*)"Hello", 5);
    AriaString* s2 = aria_string_new((uint8_t*)" World", 6);

    if (s1 && s2) {
        printf("  ✓ Created strings\n");
        printf("  String 1 length: %zu\n", aria_string_len(s1));
        printf("  String 2 length: %zu\n", aria_string_len(s2));

        AriaString* concat = aria_string_concat(s1, s2);
        if (concat) {
            printf("  ✓ Concatenated strings, length: %zu\n", aria_string_len(concat));

            printf("  Output: ");
            aria_println(concat);
        }

        AriaString* slice = aria_string_slice(s1, 0, 4);
        if (slice) {
            printf("  ✓ Created slice, length: %zu\n", aria_string_len(slice));
        }

        AriaString* s3 = aria_string_new((uint8_t*)"Hello", 5);
        int eq = aria_string_eq(s1, s3);
        printf("  ✓ String equality test: %s\n", eq ? "equal" : "not equal");
    }

    // Test 3: Array operations
    printf("\nTest 3: Array operations\n");
    AriaArray* arr = aria_array_new(sizeof(int64_t), 4);
    if (arr) {
        printf("  ✓ Created array\n");

        int64_t values[] = {10, 20, 30, 40, 50};
        for (int i = 0; i < 5; i++) {
            aria_array_push(arr, &values[i]);
        }

        printf("  ✓ Pushed %d elements\n", 5);
        printf("  Array length: %zu\n", aria_array_len(arr));

        for (size_t i = 0; i < aria_array_len(arr); i++) {
            const int64_t* val = (const int64_t*)aria_array_get(arr, i);
            if (val) {
                printf("  arr[%zu] = %ld\n", i, *val);
            }
        }
    }

    // Test 4: HashMap operations
    printf("\nTest 4: HashMap operations\n");
    AriaHashMap* map = aria_hashmap_new();
    if (map) {
        printf("  ✓ Created hash map\n");

        AriaString* key1 = aria_string_new((uint8_t*)"answer", 6);
        AriaString* key2 = aria_string_new((uint8_t*)"count", 5);

        aria_hashmap_insert(map, key1, 42);
        aria_hashmap_insert(map, key2, 100);
        printf("  ✓ Inserted values\n");

        int64_t v1 = aria_hashmap_get(map, key1);
        int64_t v2 = aria_hashmap_get(map, key2);

        printf("  map['answer'] = %ld\n", v1);
        printf("  map['count'] = %ld\n", v2);
    }

    // Test 5: I/O operations
    printf("\nTest 5: I/O operations\n");
    AriaString* msg = aria_string_new((uint8_t*)"Hello from Aria runtime!", 24);
    if (msg) {
        printf("  Printing string: ");
        aria_println(msg);

        printf("  Printing without newline: ");
        aria_print(msg);
        printf(" (done)\n");
    }

    printf("\n=== All tests completed successfully! ===\n");
    return 0;
}
