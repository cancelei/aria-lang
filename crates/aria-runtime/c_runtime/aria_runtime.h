/**
 * Aria Runtime Library - Header File
 *
 * This header declares all runtime functions that compiled Aria programs
 * can call. These functions provide core functionality like I/O, memory
 * management, and string operations.
 */

#ifndef ARIA_RUNTIME_H
#define ARIA_RUNTIME_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ========================================================================
 * Print Functions
 * ======================================================================== */

/**
 * Print an integer value to stdout.
 * @param value The integer to print
 */
void aria_print_int(int64_t value);

/**
 * Print a floating-point value to stdout.
 * @param value The float to print
 */
void aria_print_float(double value);

/**
 * Print a string to stdout.
 * @param str Pointer to null-terminated string
 */
void aria_print_string(const char* str);

/**
 * Print a boolean value to stdout (as "true" or "false").
 * @param value The boolean value (0 = false, non-zero = true)
 */
void aria_print_bool(int8_t value);

/**
 * Print a newline character to stdout.
 */
void aria_print_newline(void);

/* ========================================================================
 * Memory Management Functions
 * ======================================================================== */

/**
 * Allocate memory on the heap.
 * @param size Number of bytes to allocate
 * @return Pointer to allocated memory, or NULL on failure
 */
void* aria_alloc(int64_t size);

/**
 * Deallocate previously allocated memory.
 * @param ptr Pointer to memory to free
 * @param size Size hint (currently unused, reserved for future use)
 */
void aria_dealloc(void* ptr, int64_t size);

/* ========================================================================
 * String Operations
 * ======================================================================== */

/**
 * Concatenate two strings.
 * @param a First string (can be NULL, treated as empty string)
 * @param b Second string (can be NULL, treated as empty string)
 * @return Newly allocated string containing a + b, or NULL on allocation failure
 * @note The caller is responsible for freeing the returned string
 */
char* aria_string_concat(const char* a, const char* b);

/**
 * Compare two strings for equality.
 * @param a First string
 * @param b Second string
 * @return 1 if strings are equal, 0 otherwise
 */
int8_t aria_string_eq(const char* a, const char* b);

/**
 * Get the length of a string.
 * @param str The string
 * @return Length in bytes (0 if NULL)
 */
int64_t aria_string_len(const char* str);

/**
 * Check if a string contains a substring.
 * @param haystack The string to search in
 * @param needle The substring to search for
 * @return 1 if found, 0 otherwise
 */
int8_t aria_string_contains(const char* haystack, const char* needle);

/**
 * Check if a string starts with a prefix.
 * @param str The string
 * @param prefix The prefix to check
 * @return 1 if starts with prefix, 0 otherwise
 */
int8_t aria_string_starts_with(const char* str, const char* prefix);

/**
 * Check if a string ends with a suffix.
 * @param str The string
 * @param suffix The suffix to check
 * @return 1 if ends with suffix, 0 otherwise
 */
int8_t aria_string_ends_with(const char* str, const char* suffix);

/**
 * Remove leading and trailing whitespace from a string.
 * @param str The string to trim
 * @return Newly allocated trimmed string
 */
char* aria_string_trim(const char* str);

/**
 * Get a substring.
 * @param str The source string
 * @param start Starting index
 * @param length Number of characters
 * @return Newly allocated substring
 */
char* aria_string_substring(const char* str, int64_t start, int64_t length);

/**
 * Replace all occurrences of a substring.
 * @param str The source string
 * @param from Substring to replace
 * @param to Replacement string
 * @return Newly allocated string with replacements
 */
char* aria_string_replace(const char* str, const char* from, const char* to);

/**
 * Convert a string to uppercase.
 * @param str The string
 * @return Newly allocated uppercase string
 */
char* aria_string_to_upper(const char* str);

/**
 * Convert a string to lowercase.
 * @param str The string
 * @return Newly allocated lowercase string
 */
char* aria_string_to_lower(const char* str);

/**
 * Get character at index.
 * @param str The string
 * @param index The index
 * @return Character code (0 for invalid index)
 */
int32_t aria_char_at(const char* str, int64_t index);

/* ========================================================================
 * Type Conversion Functions
 * ======================================================================== */

/** Convert integer to string (caller must free) */
char* aria_int_to_string(int64_t value);

/** Convert float to string (caller must free) */
char* aria_float_to_string(double value);

/** Convert boolean to string (caller must free) */
char* aria_bool_to_string(int8_t value);

/** Convert char to string (caller must free) */
char* aria_char_to_string(int32_t value);

/** Convert string to integer (returns 0 on error) */
int64_t aria_string_to_int(const char* str);

/** Convert float to integer (truncates) */
int64_t aria_float_to_int(double value);

/** Convert string to float (returns 0.0 on error) */
double aria_string_to_float(const char* str);

/** Convert integer to float */
double aria_int_to_float(int64_t value);

/* ========================================================================
 * Math Functions
 * ======================================================================== */

/** Absolute value of integer */
int64_t aria_abs_int(int64_t x);

/** Absolute value of float */
double aria_abs_float(double x);

/** Minimum of two integers */
int64_t aria_min_int(int64_t a, int64_t b);

/** Maximum of two integers */
int64_t aria_max_int(int64_t a, int64_t b);

/** Minimum of two floats */
double aria_min_float(double a, double b);

/** Maximum of two floats */
double aria_max_float(double a, double b);

/** Square root */
double aria_sqrt(double x);

/** Power (base^exp) */
double aria_pow(double base, double exp);

/** Sine */
double aria_sin(double x);

/** Cosine */
double aria_cos(double x);

/** Tangent */
double aria_tan(double x);

/** Floor (round down to integer) */
int64_t aria_floor(double x);

/** Ceiling (round up to integer) */
int64_t aria_ceil(double x);

/** Round to nearest integer */
int64_t aria_round(double x);

/* ========================================================================
 * Array Functions
 * ======================================================================== */

/** Array structure for heap-allocated arrays */
typedef struct {
    void* data;         // Pointer to array elements
    int64_t length;     // Current number of elements
    int64_t capacity;   // Allocated capacity
    int64_t elem_size;  // Size of each element in bytes
} AriaArray;

/** Create a new array with given capacity and element size */
AriaArray* aria_array_new(int64_t capacity, int64_t elem_size);

/** Free an array and its data */
void aria_array_free(AriaArray* array);

/** Get the length of an array */
int64_t aria_array_length(AriaArray* array);

/** Get pointer to element at index (no bounds checking) */
void* aria_array_get_ptr(AriaArray* array, int64_t index);

/** Get integer element at index */
int64_t aria_array_get_int(AriaArray* array, int64_t index);

/** Get float element at index */
double aria_array_get_float(AriaArray* array, int64_t index);

/** Set integer element at index */
void aria_array_set_int(AriaArray* array, int64_t index, int64_t value);

/** Set float element at index */
void aria_array_set_float(AriaArray* array, int64_t index, double value);

/** Get first element (int) - panics if empty */
int64_t aria_array_first_int(AriaArray* array);

/** Get first element (float) - panics if empty */
double aria_array_first_float(AriaArray* array);

/** Get last element (int) - panics if empty */
int64_t aria_array_last_int(AriaArray* array);

/** Get last element (float) - panics if empty */
double aria_array_last_float(AriaArray* array);

/** Reverse an int array - returns new array */
AriaArray* aria_array_reverse_int(AriaArray* array);

/** Reverse a float array - returns new array */
AriaArray* aria_array_reverse_float(AriaArray* array);

/** Push an int element to the end of array - grows capacity if needed */
void aria_array_push_int(AriaArray* array, int64_t value);

/** Push a float element to the end of array - grows capacity if needed */
void aria_array_push_float(AriaArray* array, double value);

/** Pop an int element from the end of array - panics if empty */
int64_t aria_array_pop_int(AriaArray* array);

/** Pop a float element from the end of array - panics if empty */
double aria_array_pop_float(AriaArray* array);

/** Slice an int array - returns new array with elements from start to end (exclusive) */
AriaArray* aria_array_slice_int(AriaArray* array, int64_t start, int64_t end);

/** Slice a float array - returns new array with elements from start to end (exclusive) */
AriaArray* aria_array_slice_float(AriaArray* array, int64_t start, int64_t end);

/** Concatenate two int arrays - returns new array */
AriaArray* aria_array_concat_int(AriaArray* arr1, AriaArray* arr2);

/** Concatenate two float arrays - returns new array */
AriaArray* aria_array_concat_float(AriaArray* arr1, AriaArray* arr2);

/* Function pointer types for higher-order operations */
typedef int64_t (*AriaIntMapFn)(int64_t);
typedef double (*AriaFloatMapFn)(double);
typedef int8_t (*AriaIntPredicateFn)(int64_t);
typedef int8_t (*AriaFloatPredicateFn)(double);
typedef int64_t (*AriaIntReduceFn)(int64_t, int64_t);
typedef double (*AriaFloatReduceFn)(double, double);

/** Map over int array - returns new array with function applied to each element */
AriaArray* aria_array_map_int(AriaArray* array, AriaIntMapFn func);

/** Map over float array - returns new array with function applied to each element */
AriaArray* aria_array_map_float(AriaArray* array, AriaFloatMapFn func);

/** Filter int array - returns new array with elements that match predicate */
AriaArray* aria_array_filter_int(AriaArray* array, AriaIntPredicateFn predicate);

/** Filter float array - returns new array with elements that match predicate */
AriaArray* aria_array_filter_float(AriaArray* array, AriaFloatPredicateFn predicate);

/** Reduce int array - fold elements from left using accumulator function */
int64_t aria_array_reduce_int(AriaArray* array, AriaIntReduceFn func, int64_t initial);

/** Reduce float array - fold elements from left using accumulator function */
double aria_array_reduce_float(AriaArray* array, AriaFloatReduceFn func, double initial);

/** Find first int element matching predicate - returns index or -1 if not found */
int64_t aria_array_find_int(AriaArray* array, AriaIntPredicateFn predicate);

/** Find first float element matching predicate - returns index or -1 if not found */
int64_t aria_array_find_float(AriaArray* array, AriaFloatPredicateFn predicate);

/** Check if any int element matches predicate */
int8_t aria_array_any_int(AriaArray* array, AriaIntPredicateFn predicate);

/** Check if any float element matches predicate */
int8_t aria_array_any_float(AriaArray* array, AriaFloatPredicateFn predicate);

/** Check if all int elements match predicate */
int8_t aria_array_all_int(AriaArray* array, AriaIntPredicateFn predicate);

/** Check if all float elements match predicate */
int8_t aria_array_all_float(AriaArray* array, AriaFloatPredicateFn predicate);

/* ========================================================================
 * Error Handling
 * ======================================================================== */

/**
 * Panic with a runtime error message.
 * This function prints the error message and terminates the program.
 * @param message Error message to display
 */
void aria_panic(const char* message) __attribute__((noreturn));

/* ========================================================================
 * Program Entry Point
 * ======================================================================== */

/**
 * Aria program's main function.
 * This is implemented by the compiled Aria code and called by the C runtime.
 */
extern void aria_main(void);

#ifdef __cplusplus
}
#endif

#endif /* ARIA_RUNTIME_H */
