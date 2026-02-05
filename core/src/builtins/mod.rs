// Day 6: Built-in Standard Library Functions
// This module provides native Rust implementations of standard library functions

use crate::eval::Value;
use std::collections::HashMap;

mod strings;
mod json;
mod arrays;
mod files;

pub use strings::*;
pub use json::*;
pub use arrays::*;
pub use files::*;

/// Built-in function types
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinFunction {
    // String operations
    StrLen,
    StrConcat,
    StrUpper,
    StrLower,
    StrTrim,
    StrContains,
    StrReplace,
    StrSplit,
    StrStartsWith,
    StrEndsWith,

    // Array operations
    ArrFromSplit,
    ArrLen,
    ArrGet,
    ArrJoin,
    ArrPush,
    ArrPop,

    // JSON operations
    JsonParse,
    JsonStringify,
    JsonGet,

    // File I/O operations
    FileRead,
    FileWrite,
    FileExists,
    FileAppend,
}

/// Registry of built-in functions
pub struct BuiltinRegistry {
    functions: HashMap<String, BuiltinFunction>,
}

impl BuiltinRegistry {
    /// Create a new builtin registry with all standard functions registered
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };

        // Register string functions
        registry.register("str_len", BuiltinFunction::StrLen);
        registry.register("str_concat", BuiltinFunction::StrConcat);
        registry.register("str_upper", BuiltinFunction::StrUpper);
        registry.register("str_lower", BuiltinFunction::StrLower);
        registry.register("str_trim", BuiltinFunction::StrTrim);
        registry.register("str_contains", BuiltinFunction::StrContains);
        registry.register("str_replace", BuiltinFunction::StrReplace);
        registry.register("str_split", BuiltinFunction::StrSplit);
        registry.register("str_starts_with", BuiltinFunction::StrStartsWith);
        registry.register("str_ends_with", BuiltinFunction::StrEndsWith);

        // Register array functions
        registry.register("arr_from_split", BuiltinFunction::ArrFromSplit);
        registry.register("arr_len", BuiltinFunction::ArrLen);
        registry.register("arr_get", BuiltinFunction::ArrGet);
        registry.register("arr_join", BuiltinFunction::ArrJoin);
        registry.register("arr_push", BuiltinFunction::ArrPush);
        registry.register("arr_pop", BuiltinFunction::ArrPop);

        // Register JSON functions
        registry.register("json_parse", BuiltinFunction::JsonParse);
        registry.register("json_stringify", BuiltinFunction::JsonStringify);
        registry.register("json_get", BuiltinFunction::JsonGet);

        // Register file functions
        registry.register("file_read", BuiltinFunction::FileRead);
        registry.register("file_write", BuiltinFunction::FileWrite);
        registry.register("file_exists", BuiltinFunction::FileExists);
        registry.register("file_append", BuiltinFunction::FileAppend);

        registry
    }

    /// Register a builtin function
    fn register(&mut self, name: &str, func: BuiltinFunction) {
        self.functions.insert(name.to_string(), func);
    }

    /// Check if a function is registered
    pub fn has(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Get a builtin function by name
    pub fn get(&self, name: &str) -> Option<&BuiltinFunction> {
        self.functions.get(name)
    }

    /// Call a builtin function with the given arguments
    pub fn call(&self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let func = self.get(name)
            .ok_or(format!("Unknown builtin function: {}", name))?;

        match func {
            // String operations
            BuiltinFunction::StrLen => str_len(args),
            BuiltinFunction::StrConcat => str_concat(args),
            BuiltinFunction::StrUpper => str_upper(args),
            BuiltinFunction::StrLower => str_lower(args),
            BuiltinFunction::StrTrim => str_trim(args),
            BuiltinFunction::StrContains => str_contains(args),
            BuiltinFunction::StrReplace => str_replace(args),
            BuiltinFunction::StrSplit => str_split(args),
            BuiltinFunction::StrStartsWith => str_starts_with(args),
            BuiltinFunction::StrEndsWith => str_ends_with(args),

            // Array operations
            BuiltinFunction::ArrFromSplit => arr_from_split(args),
            BuiltinFunction::ArrLen => arr_len(args),
            BuiltinFunction::ArrGet => arr_get(args),
            BuiltinFunction::ArrJoin => arr_join(args),
            BuiltinFunction::ArrPush => arr_push(args),
            BuiltinFunction::ArrPop => arr_pop(args),

            // JSON operations
            BuiltinFunction::JsonParse => json_parse(args),
            BuiltinFunction::JsonStringify => json_stringify(args),
            BuiltinFunction::JsonGet => json_get(args),

            // File operations
            BuiltinFunction::FileRead => file_read(args),
            BuiltinFunction::FileWrite => file_write(args),
            BuiltinFunction::FileExists => file_exists(args),
            BuiltinFunction::FileAppend => file_append(args),
        }
    }

    /// Get list of all registered function names
    pub fn list_functions(&self) -> Vec<String> {
        let mut names: Vec<String> = self.functions.keys().cloned().collect();
        names.sort();
        names
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}
