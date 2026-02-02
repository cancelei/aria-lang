//! Environment for variable bindings in the Aria interpreter.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use smol_str::SmolStr;

use crate::Value;

/// An environment holds variable bindings and optionally a parent scope.
#[derive(Debug, Default)]
pub struct Environment {
    /// Variable bindings in this scope
    values: HashMap<SmolStr, Value>,

    /// Parent scope (for lexical scoping)
    parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    /// Create a new empty global environment.
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
            parent: None,
        }
    }

    /// Create a new child environment with the given parent.
    pub fn with_parent(parent: Rc<RefCell<Environment>>) -> Self {
        Environment {
            values: HashMap::new(),
            parent: Some(parent),
        }
    }

    /// Define a new variable in the current scope.
    pub fn define(&mut self, name: SmolStr, value: Value) {
        self.values.insert(name, value);
    }

    /// Get the value of a variable, searching up through parent scopes.
    pub fn get(&self, name: &SmolStr) -> Option<Value> {
        if let Some(value) = self.values.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.borrow().get(name)
        } else {
            None
        }
    }

    /// Check if a variable exists in any scope.
    pub fn contains(&self, name: &SmolStr) -> bool {
        if self.values.contains_key(name) {
            true
        } else if let Some(parent) = &self.parent {
            parent.borrow().contains(name)
        } else {
            false
        }
    }

    /// Assign to an existing variable, searching up through parent scopes.
    /// Returns true if the variable was found and assigned, false otherwise.
    pub fn assign(&mut self, name: &SmolStr, value: Value) -> bool {
        if self.values.contains_key(name) {
            self.values.insert(name.clone(), value);
            true
        } else if let Some(parent) = &self.parent {
            parent.borrow_mut().assign(name, value)
        } else {
            false
        }
    }

    /// Get the parent environment if it exists.
    pub fn parent(&self) -> Option<Rc<RefCell<Environment>>> {
        self.parent.clone()
    }

    /// Get all variable names defined in this scope (not including parents).
    pub fn local_names(&self) -> Vec<SmolStr> {
        self.values.keys().cloned().collect()
    }

    /// Get all variables including from parent scopes.
    pub fn all_names(&self) -> Vec<SmolStr> {
        let mut names: Vec<_> = self.values.keys().cloned().collect();
        if let Some(parent) = &self.parent {
            names.extend(parent.borrow().all_names());
        }
        names
    }
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Environment {
            values: self.values.clone(),
            parent: self.parent.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_and_get() {
        let mut env = Environment::new();
        env.define("x".into(), Value::Int(42));
        assert_eq!(env.get(&"x".into()), Some(Value::Int(42)));
    }

    #[test]
    fn test_undefined_variable() {
        let env = Environment::new();
        assert_eq!(env.get(&"x".into()), None);
    }

    #[test]
    fn test_scoping() {
        let global = Rc::new(RefCell::new(Environment::new()));
        global.borrow_mut().define("x".into(), Value::Int(10));

        let local = Environment::with_parent(global.clone());
        // Local can see global
        assert_eq!(local.get(&"x".into()), Some(Value::Int(10)));
    }

    #[test]
    fn test_shadowing() {
        let global = Rc::new(RefCell::new(Environment::new()));
        global.borrow_mut().define("x".into(), Value::Int(10));

        let mut local = Environment::with_parent(global.clone());
        local.define("x".into(), Value::Int(20));

        // Local shadows global
        assert_eq!(local.get(&"x".into()), Some(Value::Int(20)));
        // Global unchanged
        assert_eq!(global.borrow().get(&"x".into()), Some(Value::Int(10)));
    }

    #[test]
    fn test_assign() {
        let global = Rc::new(RefCell::new(Environment::new()));
        global.borrow_mut().define("x".into(), Value::Int(10));

        let mut local = Environment::with_parent(global.clone());

        // Assign to global from local
        assert!(local.assign(&"x".into(), Value::Int(20)));
        assert_eq!(global.borrow().get(&"x".into()), Some(Value::Int(20)));
    }

    #[test]
    fn test_assign_nonexistent() {
        let mut env = Environment::new();
        assert!(!env.assign(&"x".into(), Value::Int(42)));
    }
}
