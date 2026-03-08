//! Type inference engine.
//!
//! Contains the `TypeInference` struct which manages type variable
//! generation, unification, and substitution application.

use crate::{Type, TypeVar, TypeError, TypeResult};
use aria_ast::Span;
use rustc_hash::FxHashMap;

/// Type inference state
#[derive(Debug)]
pub struct TypeInference {
    /// Next type variable ID
    pub(crate) next_var: u32,
    /// Substitution map: TypeVar -> Type
    pub(crate) substitution: FxHashMap<TypeVar, Type>,
    /// Collected errors
    pub(crate) errors: Vec<TypeError>,
}

impl TypeInference {
    pub fn new() -> Self {
        Self {
            next_var: 0,
            substitution: FxHashMap::default(),
            errors: Vec::new(),
        }
    }

    /// Create a fresh type variable
    pub fn fresh_var(&mut self) -> Type {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        Type::Var(var)
    }

    /// Check if a type contains any type variables that need substitution.
    /// This allows us to skip expensive apply() calls when unnecessary.
    #[inline]
    fn needs_apply(&self, ty: &Type) -> bool {
        match ty {
            Type::Var(var) => self.substitution.contains_key(var),
            Type::Array(elem)
            | Type::FixedArray(elem, _)
            | Type::Optional(elem)
            | Type::Channel(elem)
            | Type::Task(elem) => self.needs_apply(elem),
            Type::Map(k, v) | Type::Result(k, v) => self.needs_apply(k) || self.needs_apply(v),
            Type::Tuple(ts) => ts.iter().any(|t| self.needs_apply(t)),
            Type::Reference { inner, .. } => self.needs_apply(inner),
            Type::Function { params, return_type } => {
                params.iter().any(|t| self.needs_apply(t)) || self.needs_apply(return_type)
            }
            Type::Named { type_args, .. } => type_args.iter().any(|t| self.needs_apply(t)),
            // Primitives and other types don't need substitution
            _ => false,
        }
    }

    /// Unify two types
    pub fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> TypeResult<()> {
        let t1 = self.apply(t1);
        let t2 = self.apply(t2);

        match (&t1, &t2) {
            // Same type
            _ if t1 == t2 => Ok(()),

            // Type variable on left
            (Type::Var(var), _) => {
                if self.occurs_check(*var, &t2) {
                    Err(TypeError::RecursiveType(span))
                } else {
                    self.substitution.insert(*var, t2.clone());
                    Ok(())
                }
            }

            // Type variable on right
            (_, Type::Var(var)) => {
                if self.occurs_check(*var, &t1) {
                    Err(TypeError::RecursiveType(span))
                } else {
                    self.substitution.insert(*var, t1.clone());
                    Ok(())
                }
            }

            // Array types
            (Type::Array(e1), Type::Array(e2)) => self.unify(e1, e2, span),

            // Optional types
            (Type::Optional(i1), Type::Optional(i2)) => self.unify(i1, i2, span),

            // Tuple types
            (Type::Tuple(ts1), Type::Tuple(ts2)) if ts1.len() == ts2.len() => {
                for (t1, t2) in ts1.iter().zip(ts2.iter()) {
                    self.unify(t1, t2, span)?;
                }
                Ok(())
            }

            // Function types
            (
                Type::Function {
                    params: p1,
                    return_type: r1,
                },
                Type::Function {
                    params: p2,
                    return_type: r2,
                },
            ) if p1.len() == p2.len() => {
                for (t1, t2) in p1.iter().zip(p2.iter()) {
                    self.unify(t1, t2, span)?;
                }
                self.unify(r1, r2, span)
            }

            // Map types
            (Type::Map(k1, v1), Type::Map(k2, v2)) => {
                self.unify(k1, k2, span)?;
                self.unify(v1, v2, span)
            }

            // Result types
            (Type::Result(ok1, err1), Type::Result(ok2, err2)) => {
                self.unify(ok1, ok2, span)?;
                self.unify(err1, err2, span)
            }

            // Channel types
            (Type::Channel(e1), Type::Channel(e2)) => self.unify(e1, e2, span),

            // Task types
            (Type::Task(r1), Type::Task(r2)) => self.unify(r1, r2, span),

            // Named types (same name, unify args)
            (
                Type::Named {
                    name: n1,
                    type_args: a1,
                },
                Type::Named {
                    name: n2,
                    type_args: a2,
                },
            ) if n1 == n2 && a1.len() == a2.len() => {
                for (t1, t2) in a1.iter().zip(a2.iter()) {
                    self.unify(t1, t2, span)?;
                }
                Ok(())
            }

            // Error types unify with anything
            (Type::Error, _) | (_, Type::Error) => Ok(()),

            // Any type unifies with anything (for polymorphic builtins)
            (Type::Any, _) | (_, Type::Any) => Ok(()),

            // Never type (bottom type) unifies with anything
            // Never is a subtype of all types - it represents computations that don't return
            (Type::Never, _) | (_, Type::Never) => Ok(()),

            // No match
            _ => Err(TypeError::Mismatch {
                expected: format!("{}", t1),
                found: format!("{}", t2),
                span,
                expected_source: None,
            }),
        }
    }

    /// Apply substitution to a type.
    ///
    /// Performance optimization: If the type contains no type variables that need
    /// substitution, we return a clone directly without recursing.
    pub fn apply(&self, ty: &Type) -> Type {
        // Fast path: if no substitution needed, just clone
        if !self.needs_apply(ty) {
            return ty.clone();
        }

        self.apply_inner(ty)
    }

    /// Internal apply implementation that does the actual substitution.
    /// Called only when needs_apply returns true.
    #[inline]
    fn apply_inner(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(var) => self
                .substitution
                .get(var)
                .map(|t| self.apply(t))
                .unwrap_or_else(|| ty.clone()),
            Type::Array(elem) => Type::Array(Box::new(self.apply(elem))),
            Type::FixedArray(elem, size) => Type::FixedArray(Box::new(self.apply(elem)), *size),
            Type::Map(k, v) => Type::Map(Box::new(self.apply(k)), Box::new(self.apply(v))),
            Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| self.apply(t)).collect()),
            Type::Optional(inner) => Type::Optional(Box::new(self.apply(inner))),
            Type::Result(ok, err) => {
                Type::Result(Box::new(self.apply(ok)), Box::new(self.apply(err)))
            }
            Type::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.apply(inner)),
            },
            Type::Function {
                params,
                return_type,
            } => Type::Function {
                params: params.iter().map(|t| self.apply(t)).collect(),
                return_type: Box::new(self.apply(return_type)),
            },
            Type::Named { name, type_args } => Type::Named {
                name: name.clone(),
                type_args: type_args.iter().map(|t| self.apply(t)).collect(),
            },
            Type::Channel(elem) => Type::Channel(Box::new(self.apply(elem))),
            Type::Task(result) => Type::Task(Box::new(self.apply(result))),
            _ => ty.clone(),
        }
    }

    /// Check if type variable occurs in type (for recursive type detection)
    fn occurs_check(&self, var: TypeVar, ty: &Type) -> bool {
        match ty {
            Type::Var(v) => {
                if *v == var {
                    true
                } else if let Some(t) = self.substitution.get(v) {
                    self.occurs_check(var, t)
                } else {
                    false
                }
            }
            Type::Array(elem) => self.occurs_check(var, elem),
            Type::FixedArray(elem, _) => self.occurs_check(var, elem),
            Type::Map(k, v) => self.occurs_check(var, k) || self.occurs_check(var, v),
            Type::Tuple(ts) => ts.iter().any(|t| self.occurs_check(var, t)),
            Type::Optional(inner) => self.occurs_check(var, inner),
            Type::Result(ok, err) => self.occurs_check(var, ok) || self.occurs_check(var, err),
            Type::Reference { inner, .. } => self.occurs_check(var, inner),
            Type::Function {
                params,
                return_type,
            } => {
                params.iter().any(|t| self.occurs_check(var, t))
                    || self.occurs_check(var, return_type)
            }
            Type::Named { type_args, .. } => type_args.iter().any(|t| self.occurs_check(var, t)),
            Type::Channel(elem) => self.occurs_check(var, elem),
            Type::Task(result) => self.occurs_check(var, result),
            _ => false,
        }
    }

    /// Get collected errors
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// Add an error
    pub fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }
}
