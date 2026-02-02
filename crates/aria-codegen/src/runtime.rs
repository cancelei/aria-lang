//! Runtime support for compiled Aria programs.
//!
//! This module provides declarations for runtime functions that
//! compiled Aria code needs to call (print, memory allocation, etc.).

use cranelift_codegen::ir::{types, AbiParam, Signature};
use cranelift_codegen::isa::CallConv;
use cranelift_module::{FuncId, Linkage, Module};

use crate::Result;

/// Runtime function declarations
pub struct RuntimeFunctions {
    // I/O functions
    pub print_int: Option<FuncId>,
    pub print_float: Option<FuncId>,
    pub print_bool: Option<FuncId>,
    pub print_string: Option<FuncId>,
    pub print_newline: Option<FuncId>,

    // Memory management
    pub alloc: Option<FuncId>,
    pub dealloc: Option<FuncId>,

    // String operations
    pub string_concat: Option<FuncId>,
    pub string_eq: Option<FuncId>,
    pub string_len: Option<FuncId>,
    pub string_contains: Option<FuncId>,
    pub string_starts_with: Option<FuncId>,
    pub string_ends_with: Option<FuncId>,
    pub string_trim: Option<FuncId>,
    pub string_substring: Option<FuncId>,
    pub string_replace: Option<FuncId>,
    pub string_to_upper: Option<FuncId>,
    pub string_to_lower: Option<FuncId>,
    pub char_at: Option<FuncId>,

    // Type conversion functions
    pub int_to_string: Option<FuncId>,
    pub float_to_string: Option<FuncId>,
    pub bool_to_string: Option<FuncId>,
    pub char_to_string: Option<FuncId>,
    pub string_to_int: Option<FuncId>,
    pub float_to_int: Option<FuncId>,
    pub string_to_float: Option<FuncId>,
    pub int_to_float: Option<FuncId>,

    // Math functions
    pub abs_int: Option<FuncId>,
    pub abs_float: Option<FuncId>,
    pub min_int: Option<FuncId>,
    pub max_int: Option<FuncId>,
    pub min_float: Option<FuncId>,
    pub max_float: Option<FuncId>,
    pub sqrt: Option<FuncId>,
    pub pow: Option<FuncId>,
    pub sin: Option<FuncId>,
    pub cos: Option<FuncId>,
    pub tan: Option<FuncId>,
    pub floor: Option<FuncId>,
    pub ceil: Option<FuncId>,
    pub round: Option<FuncId>,

    // Array functions
    pub array_new: Option<FuncId>,
    pub array_free: Option<FuncId>,
    pub array_length: Option<FuncId>,
    pub array_get_ptr: Option<FuncId>,
    pub array_get_int: Option<FuncId>,
    pub array_get_float: Option<FuncId>,
    pub array_set_int: Option<FuncId>,
    pub array_set_float: Option<FuncId>,
    pub array_first_int: Option<FuncId>,
    pub array_first_float: Option<FuncId>,
    pub array_last_int: Option<FuncId>,
    pub array_last_float: Option<FuncId>,
    pub array_reverse_int: Option<FuncId>,
    pub array_reverse_float: Option<FuncId>,
    pub array_push_int: Option<FuncId>,
    pub array_push_float: Option<FuncId>,
    pub array_pop_int: Option<FuncId>,
    pub array_pop_float: Option<FuncId>,
    pub array_slice_int: Option<FuncId>,
    pub array_slice_float: Option<FuncId>,
    pub array_concat_int: Option<FuncId>,
    pub array_concat_float: Option<FuncId>,

    // Error handling
    pub panic: Option<FuncId>,

    // Effect system runtime functions
    pub effect_evidence_new: Option<FuncId>,
    pub effect_evidence_push: Option<FuncId>,
    pub effect_evidence_pop: Option<FuncId>,
    pub effect_evidence_lookup: Option<FuncId>,
    pub effect_handler_call: Option<FuncId>,

    // Async effect runtime functions
    pub async_spawn: Option<FuncId>,
    pub async_await: Option<FuncId>,
    pub async_yield: Option<FuncId>,

    // State effect runtime functions
    pub state_get: Option<FuncId>,
    pub state_set: Option<FuncId>,
}

impl RuntimeFunctions {
    pub fn new() -> Self {
        Self {
            print_int: None,
            print_float: None,
            print_bool: None,
            print_string: None,
            print_newline: None,
            alloc: None,
            dealloc: None,
            string_concat: None,
            string_eq: None,
            string_len: None,
            string_contains: None,
            string_starts_with: None,
            string_ends_with: None,
            string_trim: None,
            string_substring: None,
            string_replace: None,
            string_to_upper: None,
            string_to_lower: None,
            char_at: None,
            int_to_string: None,
            float_to_string: None,
            bool_to_string: None,
            char_to_string: None,
            string_to_int: None,
            float_to_int: None,
            string_to_float: None,
            int_to_float: None,
            abs_int: None,
            abs_float: None,
            min_int: None,
            max_int: None,
            min_float: None,
            max_float: None,
            sqrt: None,
            pow: None,
            sin: None,
            cos: None,
            tan: None,
            floor: None,
            ceil: None,
            round: None,
            array_new: None,
            array_free: None,
            array_length: None,
            array_get_ptr: None,
            array_get_int: None,
            array_get_float: None,
            array_set_int: None,
            array_set_float: None,
            array_first_int: None,
            array_first_float: None,
            array_last_int: None,
            array_last_float: None,
            array_reverse_int: None,
            array_reverse_float: None,
            array_push_int: None,
            array_push_float: None,
            array_pop_int: None,
            array_pop_float: None,
            array_slice_int: None,
            array_slice_float: None,
            array_concat_int: None,
            array_concat_float: None,
            panic: None,
            // Effect system
            effect_evidence_new: None,
            effect_evidence_push: None,
            effect_evidence_pop: None,
            effect_evidence_lookup: None,
            effect_handler_call: None,
            // Async effects
            async_spawn: None,
            async_await: None,
            async_yield: None,
            // State effects
            state_get: None,
            state_set: None,
        }
    }

    /// Declare all runtime functions in the module
    pub fn declare_all<M: Module>(&mut self, module: &mut M) -> Result<()> {
        let ptr_type = module.target_config().pointer_type();
        let call_conv = module.target_config().default_call_conv;

        // === I/O Functions ===
        self.print_int = Some(declare_function(
            module, "aria_print_int", &[types::I64], None, call_conv,
        )?);
        self.print_float = Some(declare_function(
            module, "aria_print_float", &[types::F64], None, call_conv,
        )?);
        self.print_bool = Some(declare_function(
            module, "aria_print_bool", &[types::I64], None, call_conv,
        )?);
        self.print_string = Some(declare_function(
            module, "aria_print_string", &[ptr_type], None, call_conv,
        )?);
        self.print_newline = Some(declare_function(
            module, "aria_print_newline", &[], None, call_conv,
        )?);

        // === Memory Management ===
        self.alloc = Some(declare_function(
            module, "aria_alloc", &[types::I64], Some(ptr_type), call_conv,
        )?);
        self.dealloc = Some(declare_function(
            module, "aria_dealloc", &[ptr_type, types::I64], None, call_conv,
        )?);

        // === String Operations ===
        self.string_concat = Some(declare_function(
            module, "aria_string_concat", &[ptr_type, ptr_type], Some(ptr_type), call_conv,
        )?);
        self.string_eq = Some(declare_function(
            module, "aria_string_eq", &[ptr_type, ptr_type], Some(types::I64), call_conv,
        )?);
        self.string_len = Some(declare_function(
            module, "aria_string_len", &[ptr_type], Some(types::I64), call_conv,
        )?);
        self.string_contains = Some(declare_function(
            module, "aria_string_contains", &[ptr_type, ptr_type], Some(types::I64), call_conv,
        )?);
        self.string_starts_with = Some(declare_function(
            module, "aria_string_starts_with", &[ptr_type, ptr_type], Some(types::I64), call_conv,
        )?);
        self.string_ends_with = Some(declare_function(
            module, "aria_string_ends_with", &[ptr_type, ptr_type], Some(types::I64), call_conv,
        )?);
        self.string_trim = Some(declare_function(
            module, "aria_string_trim", &[ptr_type], Some(ptr_type), call_conv,
        )?);
        self.string_substring = Some(declare_function(
            module, "aria_string_substring", &[ptr_type, types::I64, types::I64], Some(ptr_type), call_conv,
        )?);
        self.string_replace = Some(declare_function(
            module, "aria_string_replace", &[ptr_type, ptr_type, ptr_type], Some(ptr_type), call_conv,
        )?);
        self.string_to_upper = Some(declare_function(
            module, "aria_string_to_upper", &[ptr_type], Some(ptr_type), call_conv,
        )?);
        self.string_to_lower = Some(declare_function(
            module, "aria_string_to_lower", &[ptr_type], Some(ptr_type), call_conv,
        )?);
        self.char_at = Some(declare_function(
            module, "aria_char_at", &[ptr_type, types::I64], Some(types::I32), call_conv,
        )?);

        // === Type Conversion Functions ===
        self.int_to_string = Some(declare_function(
            module, "aria_int_to_string", &[types::I64], Some(ptr_type), call_conv,
        )?);
        self.float_to_string = Some(declare_function(
            module, "aria_float_to_string", &[types::F64], Some(ptr_type), call_conv,
        )?);
        self.bool_to_string = Some(declare_function(
            module, "aria_bool_to_string", &[types::I8], Some(ptr_type), call_conv,
        )?);
        self.char_to_string = Some(declare_function(
            module, "aria_char_to_string", &[types::I32], Some(ptr_type), call_conv,
        )?);
        self.string_to_int = Some(declare_function(
            module, "aria_string_to_int", &[ptr_type], Some(types::I64), call_conv,
        )?);
        self.float_to_int = Some(declare_function(
            module, "aria_float_to_int", &[types::F64], Some(types::I64), call_conv,
        )?);
        self.string_to_float = Some(declare_function(
            module, "aria_string_to_float", &[ptr_type], Some(types::F64), call_conv,
        )?);
        self.int_to_float = Some(declare_function(
            module, "aria_int_to_float", &[types::I64], Some(types::F64), call_conv,
        )?);

        // === Math Functions ===
        self.abs_int = Some(declare_function(
            module, "aria_abs_int", &[types::I64], Some(types::I64), call_conv,
        )?);
        self.abs_float = Some(declare_function(
            module, "aria_abs_float", &[types::F64], Some(types::F64), call_conv,
        )?);
        self.min_int = Some(declare_function(
            module, "aria_min_int", &[types::I64, types::I64], Some(types::I64), call_conv,
        )?);
        self.max_int = Some(declare_function(
            module, "aria_max_int", &[types::I64, types::I64], Some(types::I64), call_conv,
        )?);
        self.min_float = Some(declare_function(
            module, "aria_min_float", &[types::F64, types::F64], Some(types::F64), call_conv,
        )?);
        self.max_float = Some(declare_function(
            module, "aria_max_float", &[types::F64, types::F64], Some(types::F64), call_conv,
        )?);
        self.sqrt = Some(declare_function(
            module, "aria_sqrt", &[types::F64], Some(types::F64), call_conv,
        )?);
        self.pow = Some(declare_function(
            module, "aria_pow", &[types::F64, types::F64], Some(types::F64), call_conv,
        )?);
        self.sin = Some(declare_function(
            module, "aria_sin", &[types::F64], Some(types::F64), call_conv,
        )?);
        self.cos = Some(declare_function(
            module, "aria_cos", &[types::F64], Some(types::F64), call_conv,
        )?);
        self.tan = Some(declare_function(
            module, "aria_tan", &[types::F64], Some(types::F64), call_conv,
        )?);
        self.floor = Some(declare_function(
            module, "aria_floor", &[types::F64], Some(types::I64), call_conv,
        )?);
        self.ceil = Some(declare_function(
            module, "aria_ceil", &[types::F64], Some(types::I64), call_conv,
        )?);
        self.round = Some(declare_function(
            module, "aria_round", &[types::F64], Some(types::I64), call_conv,
        )?);

        // === Array Functions ===
        self.array_new = Some(declare_function(
            module, "aria_array_new", &[types::I64, types::I64], Some(ptr_type), call_conv,
        )?);
        self.array_free = Some(declare_function(
            module, "aria_array_free", &[ptr_type], None, call_conv,
        )?);
        self.array_length = Some(declare_function(
            module, "aria_array_length", &[ptr_type], Some(types::I64), call_conv,
        )?);
        self.array_get_ptr = Some(declare_function(
            module, "aria_array_get_ptr", &[ptr_type, types::I64], Some(ptr_type), call_conv,
        )?);
        self.array_get_int = Some(declare_function(
            module, "aria_array_get_int", &[ptr_type, types::I64], Some(types::I64), call_conv,
        )?);
        self.array_get_float = Some(declare_function(
            module, "aria_array_get_float", &[ptr_type, types::I64], Some(types::F64), call_conv,
        )?);
        self.array_set_int = Some(declare_function(
            module, "aria_array_set_int", &[ptr_type, types::I64, types::I64], None, call_conv,
        )?);
        self.array_set_float = Some(declare_function(
            module, "aria_array_set_float", &[ptr_type, types::I64, types::F64], None, call_conv,
        )?);
        self.array_first_int = Some(declare_function(
            module, "aria_array_first_int", &[ptr_type], Some(types::I64), call_conv,
        )?);
        self.array_first_float = Some(declare_function(
            module, "aria_array_first_float", &[ptr_type], Some(types::F64), call_conv,
        )?);
        self.array_last_int = Some(declare_function(
            module, "aria_array_last_int", &[ptr_type], Some(types::I64), call_conv,
        )?);
        self.array_last_float = Some(declare_function(
            module, "aria_array_last_float", &[ptr_type], Some(types::F64), call_conv,
        )?);
        self.array_reverse_int = Some(declare_function(
            module, "aria_array_reverse_int", &[ptr_type], Some(ptr_type), call_conv,
        )?);
        self.array_reverse_float = Some(declare_function(
            module, "aria_array_reverse_float", &[ptr_type], Some(ptr_type), call_conv,
        )?);
        self.array_push_int = Some(declare_function(
            module, "aria_array_push_int", &[ptr_type, types::I64], None, call_conv,
        )?);
        self.array_push_float = Some(declare_function(
            module, "aria_array_push_float", &[ptr_type, types::F64], None, call_conv,
        )?);
        self.array_pop_int = Some(declare_function(
            module, "aria_array_pop_int", &[ptr_type], Some(types::I64), call_conv,
        )?);
        self.array_pop_float = Some(declare_function(
            module, "aria_array_pop_float", &[ptr_type], Some(types::F64), call_conv,
        )?);
        self.array_slice_int = Some(declare_function(
            module, "aria_array_slice_int", &[ptr_type, types::I64, types::I64], Some(ptr_type), call_conv,
        )?);
        self.array_slice_float = Some(declare_function(
            module, "aria_array_slice_float", &[ptr_type, types::I64, types::I64], Some(ptr_type), call_conv,
        )?);
        self.array_concat_int = Some(declare_function(
            module, "aria_array_concat_int", &[ptr_type, ptr_type], Some(ptr_type), call_conv,
        )?);
        self.array_concat_float = Some(declare_function(
            module, "aria_array_concat_float", &[ptr_type, ptr_type], Some(ptr_type), call_conv,
        )?);

        // === Error Handling ===
        self.panic = Some(declare_function(
            module, "aria_panic", &[ptr_type], None, call_conv,
        )?);

        // === Effect System Functions ===
        // Evidence vector management
        self.effect_evidence_new = Some(declare_function(
            module, "aria_effect_evidence_new", &[types::I64], Some(ptr_type), call_conv,
        )?);
        self.effect_evidence_push = Some(declare_function(
            module, "aria_effect_evidence_push", &[ptr_type, types::I64, ptr_type], None, call_conv,
        )?);
        self.effect_evidence_pop = Some(declare_function(
            module, "aria_effect_evidence_pop", &[ptr_type, types::I64], Some(ptr_type), call_conv,
        )?);
        self.effect_evidence_lookup = Some(declare_function(
            module, "aria_effect_evidence_lookup", &[ptr_type, types::I64], Some(ptr_type), call_conv,
        )?);
        self.effect_handler_call = Some(declare_function(
            module, "aria_effect_handler_call", &[ptr_type, types::I64, ptr_type], Some(types::I64), call_conv,
        )?);

        // === Async Effect Functions ===
        // Spawn a new task, returns task ID
        self.async_spawn = Some(declare_function(
            module, "aria_async_spawn", &[ptr_type, ptr_type], Some(types::I64), call_conv,
        )?);
        // Await a task by ID, returns result
        self.async_await = Some(declare_function(
            module, "aria_async_await", &[types::I64], Some(types::I64), call_conv,
        )?);
        // Yield to scheduler
        self.async_yield = Some(declare_function(
            module, "aria_async_yield", &[], None, call_conv,
        )?);

        // === State Effect Functions ===
        // Get current state value
        self.state_get = Some(declare_function(
            module, "aria_state_get", &[ptr_type], Some(types::I64), call_conv,
        )?);
        // Set state value
        self.state_set = Some(declare_function(
            module, "aria_state_set", &[ptr_type, types::I64], None, call_conv,
        )?);

        Ok(())
    }
}

impl Default for RuntimeFunctions {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to declare a function with given signature
fn declare_function<M: Module>(
    module: &mut M,
    name: &str,
    params: &[cranelift_codegen::ir::Type],
    ret: Option<cranelift_codegen::ir::Type>,
    call_conv: CallConv,
) -> Result<FuncId> {
    let mut sig = Signature::new(call_conv);

    for &param in params {
        sig.params.push(AbiParam::new(param));
    }

    if let Some(ret_type) = ret {
        sig.returns.push(AbiParam::new(ret_type));
    }

    let func_id = module.declare_function(name, Linkage::Import, &sig)?;
    Ok(func_id)
}

/// Generate C runtime implementation
///
/// This returns C code that can be compiled and linked with
/// the Aria executable to provide runtime support.
#[allow(dead_code)]
pub fn generate_c_runtime() -> &'static str {
    r#"
// Aria Runtime Support Library
// Compile with: gcc -c aria_runtime.c -o aria_runtime.o

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

// Print functions
void aria_print_int(int64_t value) {
    printf("%ld", value);
}

void aria_print_float(double value) {
    printf("%g", value);
}

void aria_print_bool(int8_t value) {
    printf("%s", value ? "true" : "false");
}

void aria_print_string(const char* str) {
    if (str) {
        printf("%s", str);
    }
}

void aria_print_newline(void) {
    printf("\n");
}

// Memory management
void* aria_alloc(int64_t size) {
    return malloc((size_t)size);
}

void aria_dealloc(void* ptr, int64_t size) {
    (void)size;  // Size hint for future use
    free(ptr);
}

// String operations
char* aria_string_concat(const char* a, const char* b) {
    if (!a) a = "";
    if (!b) b = "";

    size_t len_a = strlen(a);
    size_t len_b = strlen(b);
    char* result = malloc(len_a + len_b + 1);

    if (result) {
        memcpy(result, a, len_a);
        memcpy(result + len_a, b, len_b + 1);
    }

    return result;
}

int8_t aria_string_eq(const char* a, const char* b) {
    if (a == b) return 1;
    if (!a || !b) return 0;
    return strcmp(a, b) == 0 ? 1 : 0;
}

// === Effect System Runtime ===

// Evidence vector: array of handler pointers indexed by effect slot
typedef struct {
    void** slots;
    int64_t capacity;
} AriaEvidence;

// Create new evidence vector with given capacity
void* aria_effect_evidence_new(int64_t capacity) {
    AriaEvidence* ev = (AriaEvidence*)malloc(sizeof(AriaEvidence));
    if (ev) {
        ev->slots = (void**)calloc((size_t)capacity, sizeof(void*));
        ev->capacity = capacity;
    }
    return ev;
}

// Push handler to evidence slot, returns previous handler
void aria_effect_evidence_push(void* evidence, int64_t slot, void* handler) {
    AriaEvidence* ev = (AriaEvidence*)evidence;
    if (ev && slot >= 0 && slot < ev->capacity) {
        ev->slots[slot] = handler;
    }
}

// Pop handler from evidence slot, returns previous handler
void* aria_effect_evidence_pop(void* evidence, int64_t slot) {
    AriaEvidence* ev = (AriaEvidence*)evidence;
    if (ev && slot >= 0 && slot < ev->capacity) {
        void* prev = ev->slots[slot];
        ev->slots[slot] = NULL;
        return prev;
    }
    return NULL;
}

// Lookup handler in evidence slot
void* aria_effect_evidence_lookup(void* evidence, int64_t slot) {
    AriaEvidence* ev = (AriaEvidence*)evidence;
    if (ev && slot >= 0 && slot < ev->capacity) {
        return ev->slots[slot];
    }
    return NULL;
}

// Call effect handler operation
int64_t aria_effect_handler_call(void* handler, int64_t operation, void* args) {
    // Handler is a vtable pointer, operation is the method index
    // For now, return 0 as placeholder - full impl needs function pointer dispatch
    (void)handler;
    (void)operation;
    (void)args;
    return 0;
}

// === Async Effect Runtime ===
// Simple thread-based implementation for now

#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#endif

typedef struct {
    int64_t id;
    int64_t result;
    int8_t completed;
    void* func;
    void* captures;
} AriaTask;

static int64_t next_task_id = 1;
static AriaTask* tasks[1024] = {0};  // Simple fixed-size task pool

// Spawn a new async task
int64_t aria_async_spawn(void* func, void* captures) {
    int64_t id = next_task_id++;
    AriaTask* task = (AriaTask*)malloc(sizeof(AriaTask));
    if (task && id < 1024) {
        task->id = id;
        task->result = 0;
        task->completed = 0;
        task->func = func;
        task->captures = captures;
        tasks[id] = task;
        // In full implementation: start thread/fiber here
        // For now, mark as completed with placeholder result
        task->completed = 1;
    }
    return id;
}

// Await task completion
int64_t aria_async_await(int64_t task_id) {
    if (task_id > 0 && task_id < 1024 && tasks[task_id]) {
        AriaTask* task = tasks[task_id];
        // In full implementation: block until completed
        while (!task->completed) {
            // Spin wait (placeholder)
        }
        int64_t result = task->result;
        free(task);
        tasks[task_id] = NULL;
        return result;
    }
    return 0;
}

// Yield to scheduler
void aria_async_yield(void) {
    // In full implementation: yield to work-stealing scheduler
    // For now, no-op
}

// === State Effect Runtime ===

// Thread-local state storage (simple implementation)
static __thread int64_t current_state = 0;

int64_t aria_state_get(void* state_ref) {
    (void)state_ref;  // Could use for multiple state instances
    return current_state;
}

void aria_state_set(void* state_ref, int64_t value) {
    (void)state_ref;
    current_state = value;
}

// Entry point wrapper (calls Aria's main)
extern void aria_main(void);

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;
    aria_main();
    return 0;
}
"#
}
