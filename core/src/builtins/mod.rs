// Day 6: Standard Library - The Voice
// Builtin functions that run natively (no subprocess needed)

pub mod arrays;
pub mod files;
pub mod json;
pub mod strings;

use crate::eval::Value;
use std::collections::HashMap;

use arrays::*;
use files::*;
use json::*;
use strings::*;

type BuiltinFn = fn(Vec<Value>) -> Result<Value, String>;

pub struct BuiltinRegistry {
    functions: HashMap<String, BuiltinFn>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut functions: HashMap<String, BuiltinFn> = HashMap::new();

        // String functions (9)
        functions.insert("str_len".to_string(), str_len as BuiltinFn);
        functions.insert("str_upper".to_string(), str_upper);
        functions.insert("str_lower".to_string(), str_lower);
        functions.insert("str_trim".to_string(), str_trim);
        functions.insert("str_contains".to_string(), str_contains);
        functions.insert("str_starts_with".to_string(), str_starts_with);
        functions.insert("str_ends_with".to_string(), str_ends_with);
        functions.insert("str_replace".to_string(), str_replace);
        functions.insert("str_concat".to_string(), str_concat);

        // Array functions (6)
        functions.insert("arr_from_split".to_string(), arr_from_split);
        functions.insert("arr_len".to_string(), arr_len);
        functions.insert("arr_get".to_string(), arr_get);
        functions.insert("arr_join".to_string(), arr_join);
        functions.insert("arr_push".to_string(), arr_push);
        functions.insert("arr_pop".to_string(), arr_pop);

        // JSON functions (3)
        functions.insert("json_parse".to_string(), json_parse);
        functions.insert("json_stringify".to_string(), json_stringify);
        functions.insert("json_get".to_string(), json_get);

        // File functions (4)
        functions.insert("file_read".to_string(), file_read);
        functions.insert("file_write".to_string(), file_write);
        functions.insert("file_exists".to_string(), file_exists);
        functions.insert("file_append".to_string(), file_append);

        Self { functions }
    }

    /// Check if a function name is a builtin
    pub fn has(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Call a builtin function by name
    pub fn call(&self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let func = self
            .functions
            .get(name)
            .ok_or(format!("Unknown builtin function: '{}'", name))?;
        func(args)
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}
