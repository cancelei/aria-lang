//! WIT (WebAssembly Interface Types) generation from Aria effect declarations.
//!
//! This crate implements the core breakthrough of Aria-Lang: mapping the
//! algebraic effect system to WASM Component Model capabilities.
//!
//! # Effects-as-Capabilities
//!
//! Aria's effect system (`!{IO, Network, FileSystem}`) maps directly to
//! the WASM Component Model's capability system (WIT imports):
//!
//! ```text
//! Aria Effect          ->  WIT Interface          ->  WASI Interface
//! !IO                  ->  import wasi:io/streams  ->  wasi:io/streams
//! !Console             ->  import wasi:cli/stdout  ->  wasi:cli/stdout
//! !FileSystem          ->  import wasi:filesystem  ->  wasi:filesystem/types
//! !Network             ->  import wasi:http        ->  wasi:http/types
//! !{} (pure)           ->  No imports              ->  None needed
//! ```
//!
//! An agent declared with `!{Network, FileSystem}` compiles to a WASM
//! component that **physically cannot** access anything else.
//!
//! # Two-Layer Guarantee
//!
//! 1. **Compile-time**: The Aria compiler verifies agents only use declared effects
//! 2. **Runtime**: WASM sandbox physically prevents access to undeclared capabilities

use std::fmt;

use aria_mir::{MirFunction, MirProgram, MirType};
use rustc_hash::FxHashMap;

/// A WASI capability that an Aria effect maps to
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WasiCapability {
    /// The WASI package (e.g., "wasi:io", "wasi:filesystem")
    pub package: String,
    /// The interface within the package (e.g., "streams", "types")
    pub interface: String,
    /// Specific functions needed from this interface
    pub functions: Vec<WitFunction>,
}

/// A function signature in a WIT interface
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WitFunction {
    pub name: String,
    pub params: Vec<(String, WitType)>,
    pub result: Option<WitType>,
}

/// WIT value types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WitType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
    F32,
    F64,
    Char,
    String,
    List(Box<WitType>),
    Option(Box<WitType>),
    Result {
        ok: Option<Box<WitType>>,
        err: Option<Box<WitType>>,
    },
    Tuple(Vec<WitType>),
    Record(Vec<(String, WitType)>),
    Handle(String),
}

impl fmt::Display for WitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WitType::Bool => write!(f, "bool"),
            WitType::U8 => write!(f, "u8"),
            WitType::U16 => write!(f, "u16"),
            WitType::U32 => write!(f, "u32"),
            WitType::U64 => write!(f, "u64"),
            WitType::S8 => write!(f, "s8"),
            WitType::S16 => write!(f, "s16"),
            WitType::S32 => write!(f, "s32"),
            WitType::S64 => write!(f, "s64"),
            WitType::F32 => write!(f, "f32"),
            WitType::F64 => write!(f, "f64"),
            WitType::Char => write!(f, "char"),
            WitType::String => write!(f, "string"),
            WitType::List(inner) => write!(f, "list<{inner}>"),
            WitType::Option(inner) => write!(f, "option<{inner}>"),
            WitType::Result { ok, err } => {
                write!(f, "result")?;
                match (ok, err) {
                    (Some(ok), Some(err)) => write!(f, "<{ok}, {err}>"),
                    (Some(ok), None) => write!(f, "<{ok}>"),
                    (None, Some(err)) => write!(f, "<_, {err}>"),
                    (None, None) => Ok(()),
                }
            }
            WitType::Tuple(types) => {
                write!(f, "tuple<")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{ty}")?;
                }
                write!(f, ">")
            }
            WitType::Record(fields) => {
                write!(f, "record {{ ")?;
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name}: {ty}")?;
                }
                write!(f, " }}")
            }
            WitType::Handle(name) => write!(f, "{name}"),
        }
    }
}

/// Maps Aria effect names to WASI capability requirements
#[derive(Debug, Clone)]
pub struct EffectCapabilityMap {
    mappings: FxHashMap<String, Vec<WasiCapability>>,
}

impl Default for EffectCapabilityMap {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectCapabilityMap {
    /// Create the standard mapping from Aria effects to WASI capabilities
    pub fn new() -> Self {
        let mut mappings = FxHashMap::default();

        // !IO -> wasi:io/streams
        mappings.insert(
            "IO".to_string(),
            vec![
                WasiCapability {
                    package: "wasi:io".to_string(),
                    interface: "streams".to_string(),
                    functions: vec![
                        WitFunction {
                            name: "read".to_string(),
                            params: vec![
                                ("stream".to_string(), WitType::Handle("input-stream".to_string())),
                                ("len".to_string(), WitType::U64),
                            ],
                            result: Some(WitType::Result {
                                ok: Some(Box::new(WitType::List(Box::new(WitType::U8)))),
                                err: Some(Box::new(WitType::Handle("stream-error".to_string()))),
                            }),
                        },
                        WitFunction {
                            name: "write".to_string(),
                            params: vec![
                                ("stream".to_string(), WitType::Handle("output-stream".to_string())),
                                ("contents".to_string(), WitType::List(Box::new(WitType::U8))),
                            ],
                            result: Some(WitType::Result {
                                ok: Some(Box::new(WitType::U64)),
                                err: Some(Box::new(WitType::Handle("stream-error".to_string()))),
                            }),
                        },
                    ],
                },
            ],
        );

        // !Console -> wasi:cli/stdout + wasi:cli/stdin
        mappings.insert(
            "Console".to_string(),
            vec![
                WasiCapability {
                    package: "wasi:cli".to_string(),
                    interface: "stdout".to_string(),
                    functions: vec![WitFunction {
                        name: "get-stdout".to_string(),
                        params: vec![],
                        result: Some(WitType::Handle("output-stream".to_string())),
                    }],
                },
                WasiCapability {
                    package: "wasi:cli".to_string(),
                    interface: "stdin".to_string(),
                    functions: vec![WitFunction {
                        name: "get-stdin".to_string(),
                        params: vec![],
                        result: Some(WitType::Handle("input-stream".to_string())),
                    }],
                },
            ],
        );

        // !FileSystem -> wasi:filesystem/types
        mappings.insert(
            "FileSystem".to_string(),
            vec![WasiCapability {
                package: "wasi:filesystem".to_string(),
                interface: "types".to_string(),
                functions: vec![
                    WitFunction {
                        name: "read-via-stream".to_string(),
                        params: vec![
                            ("fd".to_string(), WitType::Handle("descriptor".to_string())),
                            ("offset".to_string(), WitType::U64),
                        ],
                        result: Some(WitType::Result {
                            ok: Some(Box::new(WitType::Handle("input-stream".to_string()))),
                            err: Some(Box::new(WitType::Handle("error-code".to_string()))),
                        }),
                    },
                    WitFunction {
                        name: "write-via-stream".to_string(),
                        params: vec![
                            ("fd".to_string(), WitType::Handle("descriptor".to_string())),
                            ("offset".to_string(), WitType::U64),
                        ],
                        result: Some(WitType::Result {
                            ok: Some(Box::new(WitType::Handle("output-stream".to_string()))),
                            err: Some(Box::new(WitType::Handle("error-code".to_string()))),
                        }),
                    },
                    WitFunction {
                        name: "stat".to_string(),
                        params: vec![
                            ("fd".to_string(), WitType::Handle("descriptor".to_string())),
                        ],
                        result: Some(WitType::Result {
                            ok: Some(Box::new(WitType::Record(vec![
                                ("size".to_string(), WitType::U64),
                                ("type".to_string(), WitType::U8),
                            ]))),
                            err: Some(Box::new(WitType::Handle("error-code".to_string()))),
                        }),
                    },
                ],
            }],
        );

        // !Network -> wasi:http/types + wasi:http/outgoing-handler
        mappings.insert(
            "Network".to_string(),
            vec![
                WasiCapability {
                    package: "wasi:http".to_string(),
                    interface: "types".to_string(),
                    functions: vec![
                        WitFunction {
                            name: "new-outgoing-request".to_string(),
                            params: vec![
                                ("method".to_string(), WitType::String),
                                ("url".to_string(), WitType::String),
                            ],
                            result: Some(WitType::Handle("outgoing-request".to_string())),
                        },
                    ],
                },
                WasiCapability {
                    package: "wasi:http".to_string(),
                    interface: "outgoing-handler".to_string(),
                    functions: vec![WitFunction {
                        name: "handle".to_string(),
                        params: vec![
                            ("request".to_string(), WitType::Handle("outgoing-request".to_string())),
                        ],
                        result: Some(WitType::Result {
                            ok: Some(Box::new(WitType::Handle("incoming-response".to_string()))),
                            err: Some(Box::new(WitType::Handle("error-code".to_string()))),
                        }),
                    }],
                },
            ],
        );

        // !Random -> wasi:random/random
        mappings.insert(
            "Random".to_string(),
            vec![WasiCapability {
                package: "wasi:random".to_string(),
                interface: "random".to_string(),
                functions: vec![WitFunction {
                    name: "get-random-bytes".to_string(),
                    params: vec![("len".to_string(), WitType::U64)],
                    result: Some(WitType::List(Box::new(WitType::U8))),
                }],
            }],
        );

        // !Clock -> wasi:clocks/wall-clock
        mappings.insert(
            "Clock".to_string(),
            vec![WasiCapability {
                package: "wasi:clocks".to_string(),
                interface: "wall-clock".to_string(),
                functions: vec![WitFunction {
                    name: "now".to_string(),
                    params: vec![],
                    result: Some(WitType::Record(vec![
                        ("seconds".to_string(), WitType::U64),
                        ("nanoseconds".to_string(), WitType::U32),
                    ])),
                }],
            }],
        );

        Self { mappings }
    }

    /// Look up the WASI capabilities for an Aria effect name
    pub fn capabilities_for(&self, effect_name: &str) -> Option<&[WasiCapability]> {
        self.mappings.get(effect_name).map(|v| v.as_slice())
    }

    /// Register a custom effect-to-capability mapping
    pub fn register(&mut self, effect_name: String, capabilities: Vec<WasiCapability>) {
        self.mappings.insert(effect_name, capabilities);
    }
}

/// A complete WIT world generated from an Aria program
#[derive(Debug, Clone)]
pub struct WitWorld {
    /// World name (derived from the agent/program name)
    pub name: String,
    /// Imported interfaces (capabilities the agent requires)
    pub imports: Vec<WitImport>,
    /// Exported interfaces (what the agent provides)
    pub exports: Vec<WitExport>,
}

/// An imported WIT interface
#[derive(Debug, Clone)]
pub struct WitImport {
    /// Import path (e.g., "wasi:io/streams")
    pub path: String,
    /// Functions available through this import
    pub functions: Vec<WitFunction>,
}

/// An exported WIT interface
#[derive(Debug, Clone)]
pub struct WitExport {
    /// Export name
    pub name: String,
    /// Exported functions
    pub functions: Vec<WitFunction>,
}

/// Generate WIT from a MIR program
pub fn generate_wit(program: &MirProgram) -> WitWorld {
    generate_wit_with_map(program, &EffectCapabilityMap::new())
}

/// Generate WIT from a MIR program with a custom capability map
pub fn generate_wit_with_map(program: &MirProgram, cap_map: &EffectCapabilityMap) -> WitWorld {
    let mut imports: FxHashMap<String, WitImport> = FxHashMap::default();
    let mut exports = Vec::new();

    // Collect all effects from all functions
    for func in program.functions.values() {
        for effect in &func.effect_row.effects {
            if let Some(capabilities) = cap_map.capabilities_for(effect.name.as_str()) {
                for cap in capabilities {
                    let path = format!("{}/{}", cap.package, cap.interface);
                    imports.entry(path.clone()).or_insert_with(|| WitImport {
                        path,
                        functions: cap.functions.clone(),
                    });
                }
            }
        }

        // Public functions become exports
        if func.is_public {
            exports.push(WitExport {
                name: func.name.to_string(),
                functions: vec![mir_func_to_wit_func(func)],
            });
        }
    }

    // Derive world name from entry function or first public function
    let world_name = program
        .functions
        .values()
        .find(|f| f.name == "main")
        .or_else(|| program.functions.values().find(|f| f.is_public))
        .map(|f| f.name.to_string())
        .unwrap_or_else(|| "aria-agent".to_string());

    WitWorld {
        name: world_name,
        imports: imports.into_values().collect(),
        exports,
    }
}

/// Convert a MIR function signature to a WIT function
fn mir_func_to_wit_func(func: &MirFunction) -> WitFunction {
    let params = func
        .params
        .iter()
        .map(|&local| {
            let decl = &func.locals[local.0 as usize];
            let name = decl
                .name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("p{}", local.0));
            (name, mir_type_to_wit(&decl.ty))
        })
        .collect();

    let result = if func.return_ty != MirType::Unit {
        Some(mir_type_to_wit(&func.return_ty))
    } else {
        None
    };

    WitFunction {
        name: func.name.to_string(),
        params,
        result,
    }
}

/// Convert a MIR type to a WIT type
pub fn mir_type_to_wit(ty: &MirType) -> WitType {
    match ty {
        MirType::Unit => WitType::Tuple(vec![]),
        MirType::Bool => WitType::Bool,
        MirType::Int8 => WitType::S8,
        MirType::Int16 => WitType::S16,
        MirType::Int32 => WitType::S32,
        MirType::Int | MirType::Int64 => WitType::S64,
        MirType::UInt8 => WitType::U8,
        MirType::UInt16 => WitType::U16,
        MirType::UInt32 => WitType::U32,
        MirType::UInt | MirType::UInt64 => WitType::U64,
        MirType::Float32 => WitType::F32,
        MirType::Float | MirType::Float64 => WitType::F64,
        MirType::Char => WitType::Char,
        MirType::String => WitType::String,
        MirType::Array(inner) => WitType::List(Box::new(mir_type_to_wit(inner))),
        MirType::Optional(inner) => WitType::Option(Box::new(mir_type_to_wit(inner))),
        MirType::Result(ok, err) => WitType::Result {
            ok: Some(Box::new(mir_type_to_wit(ok))),
            err: Some(Box::new(mir_type_to_wit(err))),
        },
        MirType::Tuple(types) => WitType::Tuple(types.iter().map(mir_type_to_wit).collect()),
        _ => WitType::S64, // Fallback for unsupported types
    }
}

/// Render a WitWorld as a .wit text file
pub fn render_wit(world: &WitWorld) -> String {
    let mut out = String::new();

    out.push_str(&format!("/// Auto-generated by Aria-Lang compiler\n"));
    out.push_str(&format!("/// This WIT world defines the capabilities required by the agent.\n"));
    out.push_str(&format!("/// Effects declared in Aria source map to WASI imports below.\n\n"));

    out.push_str(&format!("world {} {{\n", sanitize_wit_name(&world.name)));

    // Imports (capabilities required)
    for import in &world.imports {
        out.push_str(&format!("    import {};\n", import.path));
    }

    if !world.imports.is_empty() && !world.exports.is_empty() {
        out.push_str("\n");
    }

    // Exports (functions provided)
    for export in &world.exports {
        for func in &export.functions {
            out.push_str(&format!("    export {}: func(", sanitize_wit_name(&func.name)));
            for (i, (name, ty)) in func.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("{}: {}", sanitize_wit_name(name), ty));
            }
            out.push_str(")");
            if let Some(result) = &func.result {
                out.push_str(&format!(" -> {result}"));
            }
            out.push_str(";\n");
        }
    }

    out.push_str("}\n");
    out
}

/// Sanitize a name for WIT (kebab-case, no underscores)
fn sanitize_wit_name(name: &str) -> String {
    name.replace('_', "-")
}

/// Collect all unique WASI capabilities required by a program's effect rows
pub fn collect_capabilities(program: &MirProgram) -> Vec<WasiCapability> {
    let cap_map = EffectCapabilityMap::new();
    let mut seen = FxHashMap::default();
    let mut result = Vec::new();

    for func in program.functions.values() {
        for effect in &func.effect_row.effects {
            if let Some(caps) = cap_map.capabilities_for(effect.name.as_str()) {
                for cap in caps {
                    let key = format!("{}/{}", cap.package, cap.interface);
                    if !seen.contains_key(&key) {
                        seen.insert(key, true);
                        result.push(cap.clone());
                    }
                }
            }
        }
    }

    result
}

/// Check if a program is pure (requires no capabilities)
pub fn is_pure_program(program: &MirProgram) -> bool {
    program.functions.values().all(|f| f.effect_row.is_pure())
}

/// Errors from WIT generation
#[derive(Debug, thiserror::Error)]
pub enum WitError {
    #[error("unknown effect: {name}")]
    UnknownEffect { name: String },

    #[error("effect {effect} conflicts with {other}: both require exclusive access to {resource}")]
    ConflictingEffects {
        effect: String,
        other: String,
        resource: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use aria_mir::*;
    use smol_str::SmolStr;

    fn make_program_with_effects(effects: &[&str]) -> MirProgram {
        let mut program = MirProgram::new();
        let span = aria_lexer::Span::dummy();

        let fn_id = FunctionId(0);
        let mut func = MirFunction::new("agent_main".into(), MirType::Int64, span);
        func.is_public = true;

        let mut effect_row = EffectRow::new();
        for (i, name) in effects.iter().enumerate() {
            let et = EffectType::new(EffectId(i as u32), SmolStr::new(name));
            effect_row = effect_row.with_effect(et);
        }
        func.set_effect_row(effect_row);

        // Add a simple return block
        func.locals.push(LocalDecl {
            name: Some("return".into()),
            ty: MirType::Int64,
            mutable: true,
            span,
        });
        let block = BasicBlock {
            id: BlockId::ENTRY,
            statements: vec![Statement {
                kind: StatementKind::Assign(
                    Place::from_local(Local::RETURN),
                    Rvalue::Use(Operand::Constant(Constant::Int(0))),
                ),
                span,
            }],
            terminator: Some(Terminator {
                kind: TerminatorKind::Return,
                span,
            }),
        };
        func.blocks.push(block);
        program.functions.insert(fn_id, func);
        program
    }

    #[test]
    fn test_pure_program_has_no_imports() {
        let program = make_program_with_effects(&[]);
        let world = generate_wit(&program);

        assert!(world.imports.is_empty());
        assert!(is_pure_program(&program));
    }

    #[test]
    fn test_io_effect_maps_to_wasi_io() {
        let program = make_program_with_effects(&["IO"]);
        let world = generate_wit(&program);

        assert!(!world.imports.is_empty());
        assert!(world.imports.iter().any(|i| i.path == "wasi:io/streams"));
    }

    #[test]
    fn test_console_effect_maps_to_wasi_cli() {
        let program = make_program_with_effects(&["Console"]);
        let world = generate_wit(&program);

        assert!(world.imports.iter().any(|i| i.path == "wasi:cli/stdout"));
        assert!(world.imports.iter().any(|i| i.path == "wasi:cli/stdin"));
    }

    #[test]
    fn test_filesystem_effect_maps_to_wasi_filesystem() {
        let program = make_program_with_effects(&["FileSystem"]);
        let world = generate_wit(&program);

        assert!(world
            .imports
            .iter()
            .any(|i| i.path == "wasi:filesystem/types"));
    }

    #[test]
    fn test_network_effect_maps_to_wasi_http() {
        let program = make_program_with_effects(&["Network"]);
        let world = generate_wit(&program);

        assert!(world.imports.iter().any(|i| i.path == "wasi:http/types"));
        assert!(world
            .imports
            .iter()
            .any(|i| i.path == "wasi:http/outgoing-handler"));
    }

    #[test]
    fn test_multiple_effects_combine() {
        let program = make_program_with_effects(&["IO", "FileSystem", "Network"]);
        let world = generate_wit(&program);

        assert!(world.imports.iter().any(|i| i.path == "wasi:io/streams"));
        assert!(world
            .imports
            .iter()
            .any(|i| i.path == "wasi:filesystem/types"));
        assert!(world.imports.iter().any(|i| i.path == "wasi:http/types"));
    }

    #[test]
    fn test_exports_include_public_functions() {
        let program = make_program_with_effects(&["IO"]);
        let world = generate_wit(&program);

        assert!(!world.exports.is_empty());
        assert!(world.exports.iter().any(|e| e.name == "agent_main"));
    }

    #[test]
    fn test_wit_rendering() {
        let program = make_program_with_effects(&["Console"]);
        let world = generate_wit(&program);
        let wit_text = render_wit(&world);

        assert!(wit_text.contains("world agent-main"));
        assert!(wit_text.contains("import wasi:cli/stdout"));
        assert!(wit_text.contains("export agent-main"));
    }

    #[test]
    fn test_mir_type_to_wit_mapping() {
        assert_eq!(mir_type_to_wit(&MirType::Bool), WitType::Bool);
        assert_eq!(mir_type_to_wit(&MirType::Int), WitType::S64);
        assert_eq!(mir_type_to_wit(&MirType::Int32), WitType::S32);
        assert_eq!(mir_type_to_wit(&MirType::Float64), WitType::F64);
        assert_eq!(mir_type_to_wit(&MirType::String), WitType::String);
        assert_eq!(mir_type_to_wit(&MirType::Char), WitType::Char);
    }

    #[test]
    fn test_collect_capabilities() {
        let program = make_program_with_effects(&["IO", "FileSystem"]);
        let caps = collect_capabilities(&program);

        assert!(caps.iter().any(|c| c.package == "wasi:io"));
        assert!(caps.iter().any(|c| c.package == "wasi:filesystem"));
    }

    #[test]
    fn test_pure_program_detection() {
        let pure = make_program_with_effects(&[]);
        let impure = make_program_with_effects(&["IO"]);

        assert!(is_pure_program(&pure));
        assert!(!is_pure_program(&impure));
    }

    #[test]
    fn test_wit_type_display() {
        assert_eq!(format!("{}", WitType::String), "string");
        assert_eq!(
            format!("{}", WitType::List(Box::new(WitType::U8))),
            "list<u8>"
        );
        assert_eq!(
            format!(
                "{}",
                WitType::Result {
                    ok: Some(Box::new(WitType::String)),
                    err: Some(Box::new(WitType::U32)),
                }
            ),
            "result<string, u32>"
        );
        assert_eq!(
            format!(
                "{}",
                WitType::Option(Box::new(WitType::S64))
            ),
            "option<s64>"
        );
    }

    #[test]
    fn test_custom_effect_mapping() {
        let mut cap_map = EffectCapabilityMap::new();
        cap_map.register(
            "ML".to_string(),
            vec![WasiCapability {
                package: "wasi:nn".to_string(),
                interface: "graph".to_string(),
                functions: vec![WitFunction {
                    name: "load".to_string(),
                    params: vec![("name".to_string(), WitType::String)],
                    result: Some(WitType::Handle("graph".to_string())),
                }],
            }],
        );

        let program = make_program_with_effects(&["ML"]);
        let world = generate_wit_with_map(&program, &cap_map);

        assert!(world.imports.iter().any(|i| i.path == "wasi:nn/graph"));
    }

    #[test]
    fn test_sanitize_wit_name() {
        assert_eq!(sanitize_wit_name("my_agent"), "my-agent");
        assert_eq!(sanitize_wit_name("agent_main"), "agent-main");
        assert_eq!(sanitize_wit_name("simple"), "simple");
    }

    #[test]
    fn test_full_wit_output() {
        let program = make_program_with_effects(&["IO", "Console"]);
        let world = generate_wit(&program);
        let wit_text = render_wit(&world);

        // Verify it's valid WIT structure
        assert!(wit_text.starts_with("///"));
        assert!(wit_text.contains("world "));
        assert!(wit_text.contains("import "));
        assert!(wit_text.contains("export "));
        assert!(wit_text.ends_with("}\n"));

        println!("Generated WIT:\n{wit_text}");
    }
}
