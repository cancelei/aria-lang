/**
 * Aria Runtime Library - Implementation
 *
 * This file implements the runtime support functions for compiled Aria programs.
 * It provides I/O, memory management, string operations, and error handling.
 *
 * Compile with:
 *   gcc -c aria_runtime.c -o aria_runtime.o
 *
 * Link with Aria object file:
 *   gcc aria_runtime.o program.o -o program
 */

#include "aria_runtime.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

/* ========================================================================
 * Print Functions
 * ======================================================================== */

void aria_print_int(int64_t value) {
    printf("%ld", value);
    fflush(stdout);
}

void aria_print_float(double value) {
    printf("%g", value);
    fflush(stdout);
}

void aria_print_string(const char* str) {
    if (str != NULL) {
        printf("%s", str);
        fflush(stdout);
    }
}

void aria_print_bool(int8_t value) {
    printf("%s", value ? "true" : "false");
    fflush(stdout);
}

void aria_print_newline(void) {
    printf("\n");
    fflush(stdout);
}

/* ========================================================================
 * Memory Management Functions
 * ======================================================================== */

void* aria_alloc(int64_t size) {
    if (size <= 0) {
        return NULL;
    }

    void* ptr = malloc((size_t)size);
    if (ptr == NULL) {
        aria_panic("Out of memory: allocation failed");
    }

    return ptr;
}

void aria_dealloc(void* ptr, int64_t size) {
    /* Size parameter reserved for future use (e.g., size classes, debugging) */
    (void)size;

    if (ptr != NULL) {
        free(ptr);
    }
}

/* ========================================================================
 * String Operations
 * ======================================================================== */

char* aria_string_concat(const char* a, const char* b) {
    /* Treat NULL as empty string */
    if (a == NULL) a = "";
    if (b == NULL) b = "";

    size_t len_a = strlen(a);
    size_t len_b = strlen(b);

    /* Allocate space for concatenated string plus null terminator */
    char* result = malloc(len_a + len_b + 1);
    if (result == NULL) {
        aria_panic("Out of memory: string concatenation failed");
    }

    /* Copy first string */
    memcpy(result, a, len_a);

    /* Copy second string and null terminator */
    memcpy(result + len_a, b, len_b + 1);

    return result;
}

int8_t aria_string_eq(const char* a, const char* b) {
    /* Same pointer (including both NULL) */
    if (a == b) {
        return 1;
    }

    /* One is NULL, the other isn't */
    if (a == NULL || b == NULL) {
        return 0;
    }

    /* Compare strings */
    return strcmp(a, b) == 0 ? 1 : 0;
}

int64_t aria_string_len(const char* str) {
    if (str == NULL) {
        return 0;
    }
    return (int64_t)strlen(str);
}

int8_t aria_string_contains(const char* haystack, const char* needle) {
    if (haystack == NULL || needle == NULL) {
        return 0;
    }
    return strstr(haystack, needle) != NULL ? 1 : 0;
}

int8_t aria_string_starts_with(const char* str, const char* prefix) {
    if (str == NULL || prefix == NULL) {
        return 0;
    }
    size_t prefix_len = strlen(prefix);
    return strncmp(str, prefix, prefix_len) == 0 ? 1 : 0;
}

int8_t aria_string_ends_with(const char* str, const char* suffix) {
    if (str == NULL || suffix == NULL) {
        return 0;
    }
    size_t str_len = strlen(str);
    size_t suffix_len = strlen(suffix);
    if (suffix_len > str_len) {
        return 0;
    }
    return strcmp(str + str_len - suffix_len, suffix) == 0 ? 1 : 0;
}

char* aria_string_trim(const char* str) {
    if (str == NULL) {
        return NULL;
    }

    /* Find start (skip whitespace) */
    const char* start = str;
    while (*start && (*start == ' ' || *start == '\t' || *start == '\n' || *start == '\r')) {
        start++;
    }

    /* Find end (skip trailing whitespace) */
    const char* end = str + strlen(str) - 1;
    while (end > start && (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r')) {
        end--;
    }

    /* Calculate length and allocate */
    size_t len = (end >= start) ? (size_t)(end - start + 1) : 0;
    char* result = malloc(len + 1);
    if (result == NULL) {
        aria_panic("Out of memory: string trim failed");
    }

    if (len > 0) {
        memcpy(result, start, len);
    }
    result[len] = '\0';

    return result;
}

char* aria_string_substring(const char* str, int64_t start, int64_t length) {
    if (str == NULL) {
        return NULL;
    }

    size_t str_len = strlen(str);

    /* Handle negative or out-of-bounds start */
    if (start < 0) start = 0;
    if ((size_t)start >= str_len) {
        char* empty = malloc(1);
        if (empty) empty[0] = '\0';
        return empty;
    }

    /* Clamp length */
    if (length < 0) length = 0;
    if ((size_t)(start + length) > str_len) {
        length = (int64_t)(str_len - (size_t)start);
    }

    char* result = malloc((size_t)length + 1);
    if (result == NULL) {
        aria_panic("Out of memory: substring failed");
    }

    memcpy(result, str + start, (size_t)length);
    result[length] = '\0';

    return result;
}

char* aria_string_replace(const char* str, const char* from, const char* to) {
    if (str == NULL || from == NULL || to == NULL) {
        return NULL;
    }

    size_t from_len = strlen(from);
    size_t to_len = strlen(to);

    if (from_len == 0) {
        /* Can't replace empty string, return copy */
        size_t len = strlen(str);
        char* result = malloc(len + 1);
        if (result) strcpy(result, str);
        return result;
    }

    /* Count occurrences */
    int count = 0;
    const char* p = str;
    while ((p = strstr(p, from)) != NULL) {
        count++;
        p += from_len;
    }

    /* Calculate new length */
    size_t old_len = strlen(str);
    size_t new_len = old_len + count * (to_len - from_len);

    char* result = malloc(new_len + 1);
    if (result == NULL) {
        aria_panic("Out of memory: string replace failed");
    }

    /* Build result */
    char* dest = result;
    p = str;
    const char* next;
    while ((next = strstr(p, from)) != NULL) {
        /* Copy part before match */
        size_t prefix_len = (size_t)(next - p);
        memcpy(dest, p, prefix_len);
        dest += prefix_len;

        /* Copy replacement */
        memcpy(dest, to, to_len);
        dest += to_len;

        p = next + from_len;
    }

    /* Copy remainder */
    strcpy(dest, p);

    return result;
}

char* aria_string_to_upper(const char* str) {
    if (str == NULL) {
        return NULL;
    }

    size_t len = strlen(str);
    char* result = malloc(len + 1);
    if (result == NULL) {
        aria_panic("Out of memory: to_upper failed");
    }

    for (size_t i = 0; i <= len; i++) {
        char c = str[i];
        if (c >= 'a' && c <= 'z') {
            result[i] = c - 32;
        } else {
            result[i] = c;
        }
    }

    return result;
}

char* aria_string_to_lower(const char* str) {
    if (str == NULL) {
        return NULL;
    }

    size_t len = strlen(str);
    char* result = malloc(len + 1);
    if (result == NULL) {
        aria_panic("Out of memory: to_lower failed");
    }

    for (size_t i = 0; i <= len; i++) {
        char c = str[i];
        if (c >= 'A' && c <= 'Z') {
            result[i] = c + 32;
        } else {
            result[i] = c;
        }
    }

    return result;
}

int32_t aria_char_at(const char* str, int64_t index) {
    if (str == NULL || index < 0 || (size_t)index >= strlen(str)) {
        return 0; /* Return null character for invalid access */
    }
    return (int32_t)(unsigned char)str[index];
}

/* ========================================================================
 * Type Conversion Functions
 * ======================================================================== */

/**
 * Convert an integer to a string.
 * Returns a newly allocated string that the caller must free.
 */
char* aria_int_to_string(int64_t value) {
    // Calculate required buffer size
    // Max int64_t is 19 digits + sign + null terminator = 21 bytes
    char* buffer = (char*)malloc(32);
    if (buffer == NULL) {
        return NULL;
    }
    snprintf(buffer, 32, "%lld", (long long)value);
    return buffer;
}

/**
 * Convert a float to a string.
 * Returns a newly allocated string that the caller must free.
 */
char* aria_float_to_string(double value) {
    char* buffer = (char*)malloc(32);
    if (buffer == NULL) {
        return NULL;
    }
    // Use %.6f for default precision, remove trailing zeros
    snprintf(buffer, 32, "%.6f", value);

    // Remove trailing zeros after decimal point
    char* dot = strchr(buffer, '.');
    if (dot != NULL) {
        char* end = buffer + strlen(buffer) - 1;
        while (end > dot && *end == '0') {
            *end = '\0';
            end--;
        }
        // If all decimals were zeros, remove the dot too
        if (end == dot) {
            *dot = '\0';
        }
    }

    return buffer;
}

/**
 * Convert a boolean to a string.
 * Returns a newly allocated string that the caller must free.
 */
char* aria_bool_to_string(int8_t value) {
    if (value) {
        char* result = (char*)malloc(5); // "true" + null
        if (result) strcpy(result, "true");
        return result;
    } else {
        char* result = (char*)malloc(6); // "false" + null
        if (result) strcpy(result, "false");
        return result;
    }
}

/**
 * Convert a char (int32) to a string.
 * Returns a newly allocated string that the caller must free.
 */
char* aria_char_to_string(int32_t value) {
    char* buffer = (char*)malloc(2); // char + null
    if (buffer == NULL) {
        return NULL;
    }
    buffer[0] = (char)value;
    buffer[1] = '\0';
    return buffer;
}

/**
 * Convert a string to an integer.
 * Returns 0 if the string is invalid.
 */
int64_t aria_string_to_int(const char* str) {
    if (str == NULL) {
        return 0;
    }
    return (int64_t)atoll(str);
}

/**
 * Convert a float to an integer (truncates).
 */
int64_t aria_float_to_int(double value) {
    return (int64_t)value;
}

/**
 * Convert a string to a float.
 * Returns 0.0 if the string is invalid.
 */
double aria_string_to_float(const char* str) {
    if (str == NULL) {
        return 0.0;
    }
    return atof(str);
}

/**
 * Convert an integer to a float.
 */
double aria_int_to_float(int64_t value) {
    return (double)value;
}

/* ========================================================================
 * Math Functions
 * ======================================================================== */

int64_t aria_abs_int(int64_t x) {
    return x < 0 ? -x : x;
}

double aria_abs_float(double x) {
    return fabs(x);
}

int64_t aria_min_int(int64_t a, int64_t b) {
    return a < b ? a : b;
}

int64_t aria_max_int(int64_t a, int64_t b) {
    return a > b ? a : b;
}

double aria_min_float(double a, double b) {
    return fmin(a, b);
}

double aria_max_float(double a, double b) {
    return fmax(a, b);
}

double aria_sqrt(double x) {
    return sqrt(x);
}

double aria_pow(double base, double exp) {
    return pow(base, exp);
}

double aria_sin(double x) {
    return sin(x);
}

double aria_cos(double x) {
    return cos(x);
}

double aria_tan(double x) {
    return tan(x);
}

int64_t aria_floor(double x) {
    return (int64_t)floor(x);
}

int64_t aria_ceil(double x) {
    return (int64_t)ceil(x);
}

int64_t aria_round(double x) {
    return (int64_t)round(x);
}

/* ========================================================================
 * Array Functions
 * ======================================================================== */

AriaArray* aria_array_new(int64_t capacity, int64_t elem_size) {
    if (capacity < 0) capacity = 0;
    if (elem_size <= 0) elem_size = 8; // Default to 8 bytes

    AriaArray* array = (AriaArray*)malloc(sizeof(AriaArray));
    if (array == NULL) {
        return NULL;
    }

    array->length = 0;
    array->capacity = capacity;
    array->elem_size = elem_size;

    if (capacity > 0) {
        array->data = malloc(capacity * elem_size);
        if (array->data == NULL) {
            free(array);
            return NULL;
        }
    } else {
        array->data = NULL;
    }

    return array;
}

void aria_array_free(AriaArray* array) {
    if (array == NULL) return;
    if (array->data != NULL) {
        free(array->data);
    }
    free(array);
}

int64_t aria_array_length(AriaArray* array) {
    if (array == NULL) return 0;
    return array->length;
}

void* aria_array_get_ptr(AriaArray* array, int64_t index) {
    if (array == NULL || array->data == NULL) return NULL;
    if (index < 0 || index >= array->length) return NULL;

    char* base = (char*)array->data;
    return base + (index * array->elem_size);
}

int64_t aria_array_get_int(AriaArray* array, int64_t index) {
    if (array == NULL) {
        aria_panic("Array access on null array");
    }
    if (index < 0 || index >= array->length) {
        aria_panic("Array index out of bounds");
    }
    void* ptr = aria_array_get_ptr(array, index);
    if (ptr == NULL) return 0;
    return *(int64_t*)ptr;
}

double aria_array_get_float(AriaArray* array, int64_t index) {
    if (array == NULL) {
        aria_panic("Array access on null array");
    }
    if (index < 0 || index >= array->length) {
        aria_panic("Array index out of bounds");
    }
    void* ptr = aria_array_get_ptr(array, index);
    if (ptr == NULL) return 0.0;
    return *(double*)ptr;
}

void aria_array_set_int(AriaArray* array, int64_t index, int64_t value) {
    if (array == NULL || array->data == NULL) return;
    if (index < 0 || index >= array->capacity) return;

    // Set the value directly using capacity bounds (not length)
    char* base = (char*)array->data;
    int64_t* ptr = (int64_t*)(base + (index * array->elem_size));
    *ptr = value;

    // Update length if we're extending the array
    if (index >= array->length) {
        array->length = index + 1;
    }
}

void aria_array_set_float(AriaArray* array, int64_t index, double value) {
    if (array == NULL || array->data == NULL) return;
    if (index < 0 || index >= array->capacity) return;

    // Set the value directly using capacity bounds (not length)
    char* base = (char*)array->data;
    double* ptr = (double*)(base + (index * array->elem_size));
    *ptr = value;

    // Update length if we're extending the array
    if (index >= array->length) {
        array->length = index + 1;
    }
}

int64_t aria_array_first_int(AriaArray* array) {
    if (array == NULL || array->length == 0) {
        aria_panic("first() called on empty array");
    }
    return aria_array_get_int(array, 0);
}

double aria_array_first_float(AriaArray* array) {
    if (array == NULL || array->length == 0) {
        aria_panic("first() called on empty array");
    }
    return aria_array_get_float(array, 0);
}

int64_t aria_array_last_int(AriaArray* array) {
    if (array == NULL || array->length == 0) {
        aria_panic("last() called on empty array");
    }
    return aria_array_get_int(array, array->length - 1);
}

double aria_array_last_float(AriaArray* array) {
    if (array == NULL || array->length == 0) {
        aria_panic("last() called on empty array");
    }
    return aria_array_get_float(array, array->length - 1);
}

AriaArray* aria_array_reverse_int(AriaArray* array) {
    if (array == NULL) {
        return NULL;
    }

    AriaArray* result = aria_array_new(array->length, array->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = array->length;

    for (int64_t i = 0; i < array->length; i++) {
        int64_t val = aria_array_get_int(array, array->length - 1 - i);
        aria_array_set_int(result, i, val);
    }

    return result;
}

AriaArray* aria_array_reverse_float(AriaArray* array) {
    if (array == NULL) {
        return NULL;
    }

    AriaArray* result = aria_array_new(array->length, array->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = array->length;

    for (int64_t i = 0; i < array->length; i++) {
        double val = aria_array_get_float(array, array->length - 1 - i);
        aria_array_set_float(result, i, val);
    }

    return result;
}

void aria_array_push_int(AriaArray* array, int64_t value) {
    if (array == NULL) {
        aria_panic("push() called on null array");
    }

    // Check if we need to grow the array
    if (array->length >= array->capacity) {
        // Grow capacity (double it, or use 4 if starting from 0)
        int64_t new_capacity = array->capacity == 0 ? 4 : array->capacity * 2;
        void* new_data = realloc(array->data, new_capacity * array->elem_size);
        if (new_data == NULL) {
            aria_panic("Out of memory: array push failed");
        }
        array->data = new_data;
        array->capacity = new_capacity;
    }

    // Add the element at the end
    char* base = (char*)array->data;
    int64_t* ptr = (int64_t*)(base + (array->length * array->elem_size));
    *ptr = value;
    array->length++;
}

void aria_array_push_float(AriaArray* array, double value) {
    if (array == NULL) {
        aria_panic("push() called on null array");
    }

    // Check if we need to grow the array
    if (array->length >= array->capacity) {
        // Grow capacity (double it, or use 4 if starting from 0)
        int64_t new_capacity = array->capacity == 0 ? 4 : array->capacity * 2;
        void* new_data = realloc(array->data, new_capacity * array->elem_size);
        if (new_data == NULL) {
            aria_panic("Out of memory: array push failed");
        }
        array->data = new_data;
        array->capacity = new_capacity;
    }

    // Add the element at the end
    char* base = (char*)array->data;
    double* ptr = (double*)(base + (array->length * array->elem_size));
    *ptr = value;
    array->length++;
}

int64_t aria_array_pop_int(AriaArray* array) {
    if (array == NULL || array->length == 0) {
        aria_panic("pop() called on empty array");
    }

    array->length--;
    char* base = (char*)array->data;
    int64_t* ptr = (int64_t*)(base + (array->length * array->elem_size));
    return *ptr;
}

double aria_array_pop_float(AriaArray* array) {
    if (array == NULL || array->length == 0) {
        aria_panic("pop() called on empty array");
    }

    array->length--;
    char* base = (char*)array->data;
    double* ptr = (double*)(base + (array->length * array->elem_size));
    return *ptr;
}

/* ========================================================================
 * Higher-Order Collection Operations
 * ======================================================================== */

AriaArray* aria_array_slice_int(AriaArray* array, int64_t start, int64_t end) {
    if (array == NULL) {
        return NULL;
    }

    /* Handle negative indices and bounds */
    if (start < 0) start = 0;
    if (end < 0) end = 0;
    if (start > array->length) start = array->length;
    if (end > array->length) end = array->length;
    if (start > end) start = end;

    int64_t slice_len = end - start;
    AriaArray* result = aria_array_new(slice_len, array->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = slice_len;

    for (int64_t i = 0; i < slice_len; i++) {
        int64_t val = aria_array_get_int(array, start + i);
        aria_array_set_int(result, i, val);
    }

    return result;
}

AriaArray* aria_array_slice_float(AriaArray* array, int64_t start, int64_t end) {
    if (array == NULL) {
        return NULL;
    }

    /* Handle negative indices and bounds */
    if (start < 0) start = 0;
    if (end < 0) end = 0;
    if (start > array->length) start = array->length;
    if (end > array->length) end = array->length;
    if (start > end) start = end;

    int64_t slice_len = end - start;
    AriaArray* result = aria_array_new(slice_len, array->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = slice_len;

    for (int64_t i = 0; i < slice_len; i++) {
        double val = aria_array_get_float(array, start + i);
        aria_array_set_float(result, i, val);
    }

    return result;
}

AriaArray* aria_array_concat_int(AriaArray* arr1, AriaArray* arr2) {
    if (arr1 == NULL || arr2 == NULL) {
        return NULL;
    }

    int64_t total_len = arr1->length + arr2->length;
    AriaArray* result = aria_array_new(total_len, arr1->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = total_len;

    /* Copy first array */
    for (int64_t i = 0; i < arr1->length; i++) {
        int64_t val = aria_array_get_int(arr1, i);
        aria_array_set_int(result, i, val);
    }

    /* Copy second array */
    for (int64_t i = 0; i < arr2->length; i++) {
        int64_t val = aria_array_get_int(arr2, i);
        aria_array_set_int(result, arr1->length + i, val);
    }

    return result;
}

AriaArray* aria_array_concat_float(AriaArray* arr1, AriaArray* arr2) {
    if (arr1 == NULL || arr2 == NULL) {
        return NULL;
    }

    int64_t total_len = arr1->length + arr2->length;
    AriaArray* result = aria_array_new(total_len, arr1->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = total_len;

    /* Copy first array */
    for (int64_t i = 0; i < arr1->length; i++) {
        double val = aria_array_get_float(arr1, i);
        aria_array_set_float(result, i, val);
    }

    /* Copy second array */
    for (int64_t i = 0; i < arr2->length; i++) {
        double val = aria_array_get_float(arr2, i);
        aria_array_set_float(result, arr1->length + i, val);
    }

    return result;
}

AriaArray* aria_array_map_int(AriaArray* array, AriaIntMapFn func) {
    if (array == NULL || func == NULL) {
        return NULL;
    }

    AriaArray* result = aria_array_new(array->length, array->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = array->length;

    for (int64_t i = 0; i < array->length; i++) {
        int64_t val = aria_array_get_int(array, i);
        int64_t mapped = func(val);
        aria_array_set_int(result, i, mapped);
    }

    return result;
}

AriaArray* aria_array_map_float(AriaArray* array, AriaFloatMapFn func) {
    if (array == NULL || func == NULL) {
        return NULL;
    }

    AriaArray* result = aria_array_new(array->length, array->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = array->length;

    for (int64_t i = 0; i < array->length; i++) {
        double val = aria_array_get_float(array, i);
        double mapped = func(val);
        aria_array_set_float(result, i, mapped);
    }

    return result;
}

AriaArray* aria_array_filter_int(AriaArray* array, AriaIntPredicateFn predicate) {
    if (array == NULL || predicate == NULL) {
        return NULL;
    }

    /* First pass: count matching elements */
    int64_t count = 0;
    for (int64_t i = 0; i < array->length; i++) {
        int64_t val = aria_array_get_int(array, i);
        if (predicate(val)) {
            count++;
        }
    }

    /* Second pass: copy matching elements */
    AriaArray* result = aria_array_new(count, array->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = count;

    int64_t idx = 0;
    for (int64_t i = 0; i < array->length; i++) {
        int64_t val = aria_array_get_int(array, i);
        if (predicate(val)) {
            aria_array_set_int(result, idx++, val);
        }
    }

    return result;
}

AriaArray* aria_array_filter_float(AriaArray* array, AriaFloatPredicateFn predicate) {
    if (array == NULL || predicate == NULL) {
        return NULL;
    }

    /* First pass: count matching elements */
    int64_t count = 0;
    for (int64_t i = 0; i < array->length; i++) {
        double val = aria_array_get_float(array, i);
        if (predicate(val)) {
            count++;
        }
    }

    /* Second pass: copy matching elements */
    AriaArray* result = aria_array_new(count, array->elem_size);
    if (result == NULL) {
        return NULL;
    }
    result->length = count;

    int64_t idx = 0;
    for (int64_t i = 0; i < array->length; i++) {
        double val = aria_array_get_float(array, i);
        if (predicate(val)) {
            aria_array_set_float(result, idx++, val);
        }
    }

    return result;
}

int64_t aria_array_reduce_int(AriaArray* array, AriaIntReduceFn func, int64_t initial) {
    if (array == NULL || func == NULL) {
        return initial;
    }

    int64_t accumulator = initial;
    for (int64_t i = 0; i < array->length; i++) {
        int64_t val = aria_array_get_int(array, i);
        accumulator = func(accumulator, val);
    }

    return accumulator;
}

double aria_array_reduce_float(AriaArray* array, AriaFloatReduceFn func, double initial) {
    if (array == NULL || func == NULL) {
        return initial;
    }

    double accumulator = initial;
    for (int64_t i = 0; i < array->length; i++) {
        double val = aria_array_get_float(array, i);
        accumulator = func(accumulator, val);
    }

    return accumulator;
}

int64_t aria_array_find_int(AriaArray* array, AriaIntPredicateFn predicate) {
    if (array == NULL || predicate == NULL) {
        return -1;
    }

    for (int64_t i = 0; i < array->length; i++) {
        int64_t val = aria_array_get_int(array, i);
        if (predicate(val)) {
            return i;
        }
    }

    return -1;
}

int64_t aria_array_find_float(AriaArray* array, AriaFloatPredicateFn predicate) {
    if (array == NULL || predicate == NULL) {
        return -1;
    }

    for (int64_t i = 0; i < array->length; i++) {
        double val = aria_array_get_float(array, i);
        if (predicate(val)) {
            return i;
        }
    }

    return -1;
}

int8_t aria_array_any_int(AriaArray* array, AriaIntPredicateFn predicate) {
    if (array == NULL || predicate == NULL) {
        return 0;
    }

    for (int64_t i = 0; i < array->length; i++) {
        int64_t val = aria_array_get_int(array, i);
        if (predicate(val)) {
            return 1;
        }
    }

    return 0;
}

int8_t aria_array_any_float(AriaArray* array, AriaFloatPredicateFn predicate) {
    if (array == NULL || predicate == NULL) {
        return 0;
    }

    for (int64_t i = 0; i < array->length; i++) {
        double val = aria_array_get_float(array, i);
        if (predicate(val)) {
            return 1;
        }
    }

    return 0;
}

int8_t aria_array_all_int(AriaArray* array, AriaIntPredicateFn predicate) {
    if (array == NULL || predicate == NULL) {
        return 1; /* Empty array: vacuous truth */
    }

    for (int64_t i = 0; i < array->length; i++) {
        int64_t val = aria_array_get_int(array, i);
        if (!predicate(val)) {
            return 0;
        }
    }

    return 1;
}

int8_t aria_array_all_float(AriaArray* array, AriaFloatPredicateFn predicate) {
    if (array == NULL || predicate == NULL) {
        return 1; /* Empty array: vacuous truth */
    }

    for (int64_t i = 0; i < array->length; i++) {
        double val = aria_array_get_float(array, i);
        if (!predicate(val)) {
            return 0;
        }
    }

    return 1;
}

/* ========================================================================
 * Error Handling
 * ======================================================================== */

void aria_panic(const char* message) {
    fprintf(stderr, "\n");
    fprintf(stderr, "==========================================\n");
    fprintf(stderr, "ARIA RUNTIME PANIC\n");
    fprintf(stderr, "==========================================\n");
    fprintf(stderr, "\n");

    if (message != NULL) {
        fprintf(stderr, "Error: %s\n", message);
    } else {
        fprintf(stderr, "Error: Unknown panic\n");
    }

    fprintf(stderr, "\n");
    fprintf(stderr, "The program has encountered a fatal error\n");
    fprintf(stderr, "and cannot continue execution.\n");
    fprintf(stderr, "==========================================\n");
    fflush(stderr);

    exit(1);
}

/* ========================================================================
 * Program Entry Point
 * ======================================================================== */

/**
 * Main entry point for the program.
 * This wraps the Aria program's main function (aria_main) and provides
 * a standard C entry point.
 */
int main(int argc, char** argv) {
    /* Future enhancement: pass command-line arguments to Aria program */
    (void)argc;
    (void)argv;

    /* Call the Aria program's main function */
    aria_main();

    return 0;
}
