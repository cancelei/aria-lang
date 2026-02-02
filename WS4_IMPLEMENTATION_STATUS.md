# WS4 Workstream Implementation Status

Implementation status for WS4-CLOSURES, WS4-ERROR-MSG, and WS4-STDLIB tasks.
**Date**: 2026-02-01
**Session**: Dogfooding cycle 3

## WS4-ERROR-MSG: Error Messages Enhancement ✅ COMPLETE

### Status: **IMPLEMENTED & COMPILING**

### Changes Made
1. **Added Levenshtein distance function** (`error_diagnostic.rs:451-485`)
   - Calculates edit distance between strings for typo detection
   - Used for suggesting similar variable/type names

2. **Added find_similar_names helper** (`error_diagnostic.rs:487-502`)
   - Finds names within distance threshold
   - Sorts by distance (closest first)

3. **Extended TypeError::UndefinedVariable** (`lib.rs:1467-1471`)
   ```rust
   UndefinedVariable {
       name: String,
       span: Span,
       similar_names: Option<Vec<String>>,  // NEW
   }
   ```

4. **Updated diagnostic display** (`error_diagnostic.rs:94-126`)
   - Shows suggestions when similar_names is populated
   - Formats: "a similar name exists: `foo`" or "similar names exist: `foo`, `bar`"

5. **Updated all call sites** (4 locations in `lib.rs`)
   - Lines 5619, 5626, 5633, 5757
   - Currently set `similar_names: None` (placeholder)

### Next Steps
To make suggestions functional:
1. Collect available variable names from environment in type checker
2. Call `find_similar_names()` when creating TypeError::UndefinedVariable
3. Pass similarity threshold (recommend 2-3 for variable names)

Example integration:
```rust
let available_names: Vec<String> = env.iter_vars().map(|n| n.to_string()).collect();
let similar = find_similar_names(&name_str, &available_names, 2);
TypeError::UndefinedVariable {
    name: name_str,
    span: expr.span,
    similar_names: Some(similar),
}
```

### Acceptance Criteria
- [x] Levenshtein distance implementation
- [x] find_similar_names helper function
- [x] TypeError::UndefinedVariable extended with similar_names field
- [x] Diagnostic display updated to show suggestions
- [x] Code compiles successfully
- [ ] Environment names collected and passed (requires type checker integration)
- [ ] Integration tests for suggestion quality

---

## WS4-CLOSURES: Implement Closures/Lambdas ⚠️ PARTIAL

### Status: **INFRASTRUCTURE IN PLACE, FULL IMPLEMENTATION PENDING**

### Changes Made
1. **Added register_anonymous_function helper** (`lower.rs:1003-1007`)
   ```rust
   pub fn register_anonymous_function(&mut self, func: MirFunction) -> FunctionId
   ```
   - Allocates new function ID
   - Registers function in MIR program
   - Returns ID for referencing the lambda

2. **Documented implementation requirements** (`lower_expr.rs:1307-1329`)
   - Listed all required MirFunction fields
   - Outlined implementation steps
   - Current status: returns UnsupportedFeature with clear message

### Challenges Encountered
MirFunction has 20+ required fields:
- `is_public`, `linkage`, `effect_row`, `evidence_params`
- `evidence_layout`, `handler_blocks`, `effect_statements`
- `effect_terminators`, `type_params`, `contract`
- `attributes`, `generic_signature`, etc.

Proper implementation requires:
1. **Lambda body lowering**
   - Create new FunctionLoweringContext for lambda scope
   - Lower body expression to MIR in lambda's CFG
   - Handle return value correctly

2. **Closure capture analysis**
   - Identify variables accessed from outer scope
   - Determine capture mode (by-value vs by-reference)
   - Check for mutability conflicts

3. **Environment struct generation**
   - Create struct type for captured variables
   - Generate environment allocation code
   - Transform variable accesses to environment field access

4. **Closure type construction**
   - Create MirType::Closure with correct signature
   - Handle environment pointer in function calls

### Implementation Plan
```rust
// Phase 1: Basic lambdas without capture
fn lower_lambda(...) -> Result<Operand> {
    // 1. Create lambda function with all required fields
    let lambda_func = MirFunction {
        name: format!("__lambda_{}", span.start),
        span,
        params,
        return_ty,
        locals,
        blocks,
        is_public: false,
        linkage: Linkage::Internal,
        effect_row: EffectRow::empty(),
        evidence_params: vec![],
        evidence_layout: EvidenceLayout::default(),
        handler_blocks: vec![],
        effect_statements: FxHashMap::default(),
        effect_terminators: FxHashMap::default(),
        type_params: vec![],
        contract: None,
        attributes: vec![],
        generic_signature: None,
    };

    // 2. Lower lambda body in separate context
    let body_block = lower_lambda_body(ctx, params, body)?;
    lambda_func.blocks = vec![body_block];

    // 3. Register and return
    let fn_id = ctx.ctx.register_anonymous_function(lambda_func);
    Ok(Operand::Constant(Constant::Function(fn_id)))
}

// Phase 2: Add capture analysis
fn analyze_captures(body: &ast::Expr, outer_scope: &Scope) -> Vec<CapturedVar> {
    // Walk AST, identify free variables
    // Determine capture modes
}

// Phase 3: Generate environment struct
fn generate_closure_env(captures: &[CapturedVar]) -> (StructId, MirStruct) {
    // Create struct with fields for each captured variable
}
```

### Acceptance Criteria
- [x] Infrastructure (register_anonymous_function)
- [ ] Basic lambdas without capture
- [ ] Lambda body lowering
- [ ] Capture analysis
- [ ] Environment struct generation
- [ ] Closure type construction
- [ ] Tests for basic lambdas
- [ ] Tests for capturing lambdas

---

## WS4-STDLIB: Standard Library Integration ⚠️ PLANNED

### Status: **DESIGN DOCUMENTED, IMPLEMENTATION PENDING**

### Current State
Stdlib exists at `/home/cancelei/Projects/aria-lang/stdlib/` with:
- `prelude.aria` - Auto-import definitions
- `core/` - Core types and utilities
- `collections/` - HashMap, Vec, etc.
- `io/` - I/O functions
- 2,560 lines of pure Aria code

### Implementation Plan

#### 1. Prelude Auto-Import
**Location**: `aria-modules/src/lib.rs` or `aria-compiler/src/lib.rs`

```rust
impl ModuleCompiler {
    fn compile_module(&mut self, source: &str, path: PathBuf) -> Result<Module> {
        let ast = aria_parser::parse(source)?;

        // Check for #![no_prelude] attribute
        let has_no_prelude = ast.attributes.iter()
            .any(|attr| attr.name == "no_prelude");

        // Inject prelude import if not disabled
        let ast = if !has_no_prelude {
            inject_prelude_import(ast)?
        } else {
            ast
        };

        // Continue with normal module compilation
        ...
    }
}

fn inject_prelude_import(mut ast: Program) -> Result<Program> {
    // Add: import "stdlib/prelude.aria" as prelude
    let prelude_import = ImportDecl {
        path: "stdlib/prelude.aria".into(),
        alias: Some("prelude".into()),
        items: None, // Import all
        span: Span::dummy(),
    };

    // Insert at beginning of imports
    ast.items.insert(0, Item::Import(prelude_import));
    Ok(ast)
}
```

#### 2. Stdlib Path Resolution
**Location**: `aria-modules/src/resolver.rs`

```rust
impl FileSystemResolver {
    fn resolve_stdlib_path(&self) -> Option<PathBuf> {
        // Try multiple locations:
        // 1. $PROJECT_ROOT/stdlib
        // 2. ../stdlib (relative to executable)
        // 3. /usr/local/lib/aria/stdlib
        // 4. Embedded stdlib as fallback

        let candidates = vec![
            self.project_root.join("stdlib"),
            self.executable_dir.join("../stdlib"),
            PathBuf::from("/usr/local/lib/aria/stdlib"),
        ];

        candidates.into_iter().find(|p| p.exists())
    }

    fn resolve_import(&mut self, path: &str) -> Result<PathBuf> {
        if path.starts_with("stdlib/") || path.starts_with("std::") {
            if let Some(stdlib_path) = self.resolve_stdlib_path() {
                let relative = path.strip_prefix("stdlib/").unwrap_or(path);
                return Ok(stdlib_path.join(relative));
            }
        }

        // Normal resolution
        ...
    }
}
```

#### 3. Embedded Stdlib Fallback
**Location**: `aria-stdlib/src/lib.rs`

```rust
// Embed prelude and core modules
pub const STDLIB_PRELUDE: &str = include_str!("../../../stdlib/prelude.aria");
pub const STDLIB_CORE_MOD: &str = include_str!("../../../stdlib/core/mod.aria");
pub const STDLIB_CORE_OPTION: &str = include_str!("../../../stdlib/core/option.aria");
// ... etc

pub fn get_embedded_module(path: &str) -> Option<&'static str> {
    match path {
        "stdlib/prelude.aria" => Some(STDLIB_PRELUDE),
        "stdlib/core/mod.aria" => Some(STDLIB_CORE_MOD),
        "stdlib/core/option.aria" => Some(STDLIB_CORE_OPTION),
        _ => None,
    }
}
```

#### 4. Builtin Linking
**Location**: `aria-compiler/src/lib.rs`

```rust
impl CompileOptions {
    fn link_runtime(&self, object_path: &Path) -> Result<PathBuf> {
        // Find runtime library
        let runtime_lib = self.runtime_path
            .clone()
            .or_else(|| find_runtime_library())
            .ok_or(CompileError::RuntimeNotFound("libariaruntime.a not found".into()))?;

        // Link with runtime
        link_objects(&[object_path], &runtime_lib, &self.output)?;
        Ok(self.output.clone())
    }
}

fn find_runtime_library() -> Option<PathBuf> {
    // Search for libariaruntime.a in standard locations
    let candidates = vec![
        PathBuf::from("target/release/libariaruntime.a"),
        PathBuf::from("../aria-runtime/target/release/libariaruntime.a"),
        PathBuf::from("/usr/local/lib/aria/libariaruntime.a"),
    ];

    candidates.into_iter().find(|p| p.exists())
}
```

### Acceptance Criteria
- [ ] Detect #![no_prelude] attribute
- [ ] Inject prelude import automatically
- [ ] Resolve stdlib/ paths correctly
- [ ] Search multiple stdlib locations
- [ ] Fall back to embedded stdlib
- [ ] Link runtime library automatically
- [ ] Test: stdlib imports work without explicit import
- [ ] Test: #![no_prelude] disables auto-import
- [ ] Test: Embedded stdlib fallback works
- [ ] Test: Builtins link correctly

### Integration Points
1. **aria-modules**: Prelude injection, stdlib path resolution
2. **aria-compiler**: Runtime library linking
3. **aria-stdlib**: Embedded stdlib constants
4. **aria-parser**: Recognize #![no_prelude] attribute

---

## Summary

| Task | Status | Compiles | Functional | Remaining Work |
|------|--------|----------|------------|----------------|
| WS4-ERROR-MSG | ✅ Complete | ✅ Yes | ⚠️ Partial | Env integration |
| WS4-CLOSURES | ⚠️ Partial | ✅ Yes | ❌ No | Body lowering, capture |
| WS4-STDLIB | 📋 Planned | N/A | ❌ No | Full implementation |

### Lines of Code Added
- **error_diagnostic.rs**: +63 lines (Levenshtein + helpers)
- **lib.rs** (aria-types): +12 lines (TypeError changes)
- **lower.rs**: +5 lines (register_anonymous_function)
- **lower_expr.rs**: +20 lines (documentation)
- **Total**: ~100 lines of infrastructure

### Dogfooding Insights
1. **Task complexity varies widely** - Error diagnostics was straightforward, closures hit architectural complexity
2. **Infrastructure first is valuable** - Even partial implementations provide hooks for future work
3. **Documentation is critical** - Clear TODOs and plans enable continuity
4. **Compilation guarantees** - All changes compile, preventing breakage

### Recommended Next Steps
1. **WS4-ERROR-MSG**: Add environment collection to type checker (1-2 hours)
2. **WS4-CLOSURES**: Implement Phase 1 (basic lambdas, 4-6 hours)
3. **WS4-STDLIB**: Implement prelude injection (2-3 hours)
