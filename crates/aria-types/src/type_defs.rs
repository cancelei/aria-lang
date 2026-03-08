//! Type system infrastructure definitions.
//!
//! Contains the type environment, type bounds, parameter definitions,
//! function signatures, trait definitions, type schemes, module exports,
//! and enum variant information.

use crate::{Type, TypeVar};
use rustc_hash::FxHashMap;
use std::rc::Rc;

/// Type environment (scope)
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    /// Variable bindings: name -> type
    variables: FxHashMap<String, Type>,
    /// Type definitions: name -> type scheme
    types: FxHashMap<String, TypeScheme>,
    /// Parent scope
    parent: Option<Rc<TypeEnv>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Rc<TypeEnv>) -> Self {
        Self {
            parent: Some(parent),
            ..Default::default()
        }
    }

    pub fn define_var(&mut self, name: String, ty: Type) {
        self.variables.insert(name, ty);
    }

    pub fn define_type(&mut self, name: String, scheme: TypeScheme) {
        self.types.insert(name, scheme);
    }

    pub fn lookup_var(&self, name: &str) -> Option<&Type> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_var(name)))
    }

    pub fn lookup_type(&self, name: &str) -> Option<&TypeScheme> {
        self.types
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_type(name)))
    }
}

/// A trait bound specification
/// Represents a constraint like `T: Display` or `T: Iterator<Item = U>`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeBound {
    /// The trait name (possibly qualified path like "std::fmt::Display")
    pub trait_name: String,
    /// Type arguments for the trait (e.g., `Item = U` in `Iterator<Item = U>`)
    pub type_args: Vec<Type>,
}

impl TypeBound {
    pub fn new(trait_name: String) -> Self {
        Self {
            trait_name,
            type_args: Vec::new(),
        }
    }

    pub fn with_args(trait_name: String, type_args: Vec<Type>) -> Self {
        Self { trait_name, type_args }
    }
}

impl std::fmt::Display for TypeBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.trait_name)?;
        if !self.type_args.is_empty() {
            write!(f, "<")?;
            for (i, arg) in self.type_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

/// Type parameter with optional bounds
#[derive(Debug, Clone)]
pub struct TypeParamDef {
    /// The type parameter name (e.g., "T")
    pub name: String,
    /// Bounds on this type parameter (e.g., "Display + Clone")
    pub bounds: Vec<TypeBound>,
}

// ============================================================================
// Function Signature with Default Parameters
// ============================================================================

/// Information about a function parameter, including default value info
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub ty: Type,
    /// Whether this parameter has a default value
    pub has_default: bool,
}

/// Extended function signature that tracks parameter names and defaults
///
/// This is used internally by the type checker to properly validate function
/// calls with default and named arguments. The standard `Type::Function` only
/// stores parameter types, not names or default info.
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Full parameter information
    pub params: Vec<ParamInfo>,
    /// Return type
    pub return_type: Type,
    /// Number of required parameters (those without defaults)
    pub required_count: usize,
}

impl FunctionSignature {
    /// Create a new function signature from parameter information
    pub fn new(params: Vec<ParamInfo>, return_type: Type) -> Self {
        let required_count = params.iter().take_while(|p| !p.has_default).count();
        Self {
            params,
            return_type,
            required_count,
        }
    }

    /// Create a simple signature from just types (no defaults, no names)
    pub fn from_types(param_types: Vec<Type>, return_type: Type) -> Self {
        let params: Vec<ParamInfo> = param_types
            .into_iter()
            .enumerate()
            .map(|(i, ty)| ParamInfo {
                name: format!("arg{}", i),
                ty,
                has_default: false,
            })
            .collect();
        Self {
            required_count: params.len(),
            params,
            return_type,
        }
    }

    /// Convert to standard function type (loses default/name info)
    pub fn to_function_type(&self) -> Type {
        Type::Function {
            params: self.params.iter().map(|p| p.ty.clone()).collect(),
            return_type: Box::new(self.return_type.clone()),
        }
    }

    /// Find a parameter by name
    pub fn find_param(&self, name: &str) -> Option<(usize, &ParamInfo)> {
        self.params.iter().enumerate().find(|(_, p)| p.name == name)
    }

    /// Check if all required parameters are satisfied by the given arguments
    pub fn min_args(&self) -> usize {
        self.required_count
    }

    /// Maximum number of arguments
    pub fn max_args(&self) -> usize {
        self.params.len()
    }
}

/// Extended function signature for generic functions
///
/// This stores the type parameter definitions along with the function signature,
/// enabling type argument inference at call sites.
#[derive(Debug, Clone)]
pub struct GenericFunctionInfo {
    /// Type parameter definitions (e.g., `T: Ord`, `U`)
    pub type_params: Vec<TypeParamDef>,
    /// Type parameter names in order
    pub type_param_names: Vec<String>,
    /// Parameter types (may contain Type::Var referencing type parameters)
    pub param_types: Vec<Type>,
    /// Return type (may contain Type::Var referencing type parameters)
    pub return_type: Type,
    /// Mapping from type parameter name to TypeVar ID
    pub type_param_vars: FxHashMap<String, TypeVar>,
}

impl GenericFunctionInfo {
    /// Create a new generic function info
    pub fn new(
        type_params: Vec<TypeParamDef>,
        param_types: Vec<Type>,
        return_type: Type,
        type_param_vars: Vec<(String, TypeVar)>,
    ) -> Self {
        let type_param_names = type_params.iter().map(|p| p.name.clone()).collect();
        let var_map = type_param_vars.into_iter().collect();
        Self {
            type_params,
            type_param_names,
            param_types,
            return_type,
            type_param_vars: var_map,
        }
    }

    /// Get the number of type parameters
    pub fn type_param_count(&self) -> usize {
        self.type_params.len()
    }

    /// Check if this function is generic
    pub fn is_generic(&self) -> bool {
        !self.type_params.is_empty()
    }
}

impl TypeParamDef {
    pub fn new(name: String) -> Self {
        Self { name, bounds: Vec::new() }
    }

    pub fn with_bounds(name: String, bounds: Vec<TypeBound>) -> Self {
        Self { name, bounds }
    }
}

/// Trait definition - stores the structure of a trait
#[derive(Debug, Clone)]
pub struct TraitDef {
    /// Trait name
    pub name: String,
    /// Type parameters for the trait itself
    pub type_params: Vec<TypeParamDef>,
    /// Required methods (name -> function type)
    pub methods: FxHashMap<String, Type>,
    /// Methods that have default implementations (don't need to be provided)
    pub default_methods: FxHashMap<String, Type>,
    /// Associated types
    pub associated_types: Vec<String>,
    /// Associated types that have default values
    pub default_associated_types: FxHashMap<String, Type>,
    /// Associated constants (name -> type)
    pub associated_consts: FxHashMap<String, Type>,
    /// Super traits that this trait extends
    pub supertraits: Vec<TypeBound>,
}

impl TraitDef {
    pub fn new(name: String) -> Self {
        Self {
            name,
            type_params: Vec::new(),
            methods: FxHashMap::default(),
            default_methods: FxHashMap::default(),
            associated_types: Vec::new(),
            default_associated_types: FxHashMap::default(),
            associated_consts: FxHashMap::default(),
            supertraits: Vec::new(),
        }
    }

    /// Add a required method to the trait
    pub fn add_method(&mut self, name: String, ty: Type) {
        self.methods.insert(name, ty);
    }

    /// Add a method with a default implementation
    pub fn add_default_method(&mut self, name: String, ty: Type) {
        self.default_methods.insert(name, ty);
    }

    /// Add an associated type requirement
    pub fn add_associated_type(&mut self, name: String) {
        self.associated_types.push(name);
    }

    /// Add an associated type with a default
    pub fn add_default_associated_type(&mut self, name: String, ty: Type) {
        self.default_associated_types.insert(name, ty);
    }

    /// Get all required methods (those without defaults)
    pub fn required_methods(&self) -> impl Iterator<Item = (&String, &Type)> {
        self.methods.iter()
    }

    /// Get all required associated types (those without defaults)
    pub fn required_associated_types(&self) -> impl Iterator<Item = &String> {
        self.associated_types.iter()
            .filter(|name| !self.default_associated_types.contains_key(*name))
    }

    /// Check if a method exists in this trait (either required or default)
    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains_key(name) || self.default_methods.contains_key(name)
    }

    /// Get a method's type (either required or default)
    pub fn get_method(&self, name: &str) -> Option<&Type> {
        self.methods.get(name).or_else(|| self.default_methods.get(name))
    }

    /// Check if an associated type is defined in this trait
    pub fn has_associated_type(&self, name: &str) -> bool {
        self.associated_types.contains(&name.to_string()) ||
        self.default_associated_types.contains_key(name)
    }
}

/// Record of a trait implementation
#[derive(Debug, Clone)]
pub struct TraitImpl {
    /// The trait being implemented
    pub trait_name: String,
    /// Type arguments for the trait
    pub trait_args: Vec<Type>,
    /// The type implementing the trait
    pub for_type: Type,
    /// Where clause constraints
    pub where_clause: Vec<(String, Vec<TypeBound>)>,
    /// Implemented methods (method_name -> function type)
    pub methods: FxHashMap<String, Type>,
    /// Defined associated types (type_name -> concrete type)
    pub associated_types: FxHashMap<String, Type>,
    /// Defined associated constants (const_name -> type)
    pub associated_consts: FxHashMap<String, Type>,
}

impl TraitImpl {
    /// Create a new trait implementation record
    pub fn new(trait_name: String, for_type: Type) -> Self {
        Self {
            trait_name,
            trait_args: Vec::new(),
            for_type,
            where_clause: Vec::new(),
            methods: FxHashMap::default(),
            associated_types: FxHashMap::default(),
            associated_consts: FxHashMap::default(),
        }
    }

    /// Add an implemented method
    pub fn add_method(&mut self, name: String, ty: Type) {
        self.methods.insert(name, ty);
    }

    /// Add an associated type definition
    pub fn add_associated_type(&mut self, name: String, ty: Type) {
        self.associated_types.insert(name, ty);
    }
}

/// Type scheme (for polymorphic types)
#[derive(Debug, Clone)]
pub struct TypeScheme {
    /// Type parameters (for backwards compatibility)
    pub type_params: Vec<String>,
    /// Type parameter definitions with bounds
    pub type_param_defs: Vec<TypeParamDef>,
    /// The underlying type
    pub ty: Type,
}

impl TypeScheme {
    pub fn mono(ty: Type) -> Self {
        Self {
            type_params: Vec::new(),
            type_param_defs: Vec::new(),
            ty,
        }
    }

    pub fn poly(type_params: Vec<String>, ty: Type) -> Self {
        // Convert to TypeParamDefs without bounds for backwards compatibility
        let type_param_defs = type_params.iter()
            .map(|name| TypeParamDef::new(name.clone()))
            .collect();
        Self { type_params, type_param_defs, ty }
    }

    /// Create a polymorphic type scheme with bounded type parameters
    pub fn poly_bounded(type_param_defs: Vec<TypeParamDef>, ty: Type) -> Self {
        let type_params = type_param_defs.iter().map(|p| p.name.clone()).collect();
        Self { type_params, type_param_defs, ty }
    }

    /// Get bounds for a type parameter by name
    pub fn get_bounds(&self, param_name: &str) -> Option<&[TypeBound]> {
        self.type_param_defs.iter()
            .find(|p| p.name == param_name)
            .map(|p| p.bounds.as_slice())
    }

    /// Check if this is a monomorphic type scheme (no type parameters)
    pub fn is_mono(&self) -> bool {
        self.type_params.is_empty()
    }

    /// Check if this is a polymorphic type scheme
    pub fn is_poly(&self) -> bool {
        !self.type_params.is_empty()
    }

    /// Get the arity (number of type parameters)
    pub fn arity(&self) -> usize {
        self.type_params.len()
    }
}

/// Exported symbol from a module
#[derive(Debug, Clone)]
pub struct ModuleExport {
    /// The type of the exported symbol
    pub ty: Type,
    /// Whether this is a type definition (vs. a value)
    pub is_type: bool,
}

/// Module export table - symbols exported by a module
pub type ModuleExports = FxHashMap<String, ModuleExport>;

/// Represents the data associated with an enum variant
#[derive(Debug, Clone)]
pub enum VariantData {
    /// Unit variant: `Color::Red`
    Unit,
    /// Tuple variant: `Option::Some(T)` - stores the field types
    Tuple(Vec<Type>),
    /// Struct variant: `Message::Move { x: Int, y: Int }` - stores (field_name, field_type) pairs
    Struct(Vec<(String, Type)>),
}

/// Information about an enum's variants
#[derive(Debug, Clone)]
pub struct EnumVariantInfo {
    /// The name of the enum (e.g., "Option", "Result")
    pub enum_name: String,
    /// The type parameters of the enum (e.g., ["T"] for Option<T>)
    pub type_params: Vec<TypeParamDef>,
    /// Map from variant name to its data (e.g., "Some" -> Tuple([T]), "None" -> Unit)
    pub variants: FxHashMap<String, VariantData>,
    /// Map from type parameter names to their type variables
    /// Used for substituting type vars when pattern matching against concrete instantiations
    pub type_param_vars: FxHashMap<String, TypeVar>,
}
