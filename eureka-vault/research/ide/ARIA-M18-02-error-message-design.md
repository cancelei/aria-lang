# ARIA-M18-02: Error Message Design Research

**Task ID**: ARIA-M18-02
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Best-in-Class Error Message Design for Aria
**PRD Reference**: REQ-DX-001 - Elm/Rust-quality error messages

---

## Executive Summary

This research analyzes error message design patterns from Elm, Rust, and TypeScript to design Aria's diagnostic system. The goal is to transform `TypeError` enum variants into user-friendly diagnostics with precise source locations, plain English explanations, fix suggestions, and documentation links.

Key findings:
- **Elm's philosophy**: Treat errors as opportunities to educate; use first-person voice ("I see..."); show exact source code
- **Rust's architecture**: Structured `Diagnostic` traits with `Span` metadata; applicability levels for suggestions; JSON output for tooling
- **TypeScript's approach**: Diagnostic categories (Error, Warning, Suggestion, Message); language service plugins for IDE integration

---

## 1. Design Philosophy

### 1.1 Core Principles (Derived from Elm)

1. **Show Exact Source Code**: Never pretty-print or reformat - display exactly what the programmer wrote
2. **Provide Specific Context**: Identify which field, which function, which argument caused the problem
3. **Strategic Use of Color and Layout**:
   - Red highlights problematic code for rapid visual scanning
   - Blue separates distinct error messages
   - General context above, detailed hints below
4. **Minimize Translation Work**: Explain in domain-specific terms, not compiler internals
5. **Educate, Don't Lecture**: Make error messages feel like a helpful dialogue

### 1.2 Voice and Tone

```
# Elm uses first-person: "I see an error"
# Rust uses third-person: "expected `String`, found `Int`"

# Aria recommendation: First-person plural (team)
"We expected a `String` here, but found an `Int`."

# This personifies the compiler as a collaborative team
# looking for bugs in the developer's code.
```

### 1.3 Error Message Structure

```
[Error Code] Error Title
 --> file_path:line:column
  |
N | source_code_line
  |           ^^^^^^ primary annotation
  |
  = help: explanation and suggestions
  = note: additional context
  = docs: https://aria-lang.org/errors/E0001
```

---

## 2. Rust Diagnostic System Analysis

### 2.1 Core Architecture

Rust's diagnostic system (`rustc_errors`) organizes error information into hierarchical components:

| Component | Description |
|-----------|-------------|
| **Severity Level** | error, warning, note, help |
| **Error Code** | Alphanumeric identifier (e.g., E0308) linking to extended explanations |
| **Primary Message** | General description that stands independently |
| **Span Information** | File path, line number, column indicating problem location |
| **Code Context** | Affected source code with surrounding lines |
| **Span Labels** | Primary and secondary annotations with explanatory text |
| **Sub-diagnostics** | Additional related messages for complex scenarios |

### 2.2 Diagnostic Struct Pattern

```rust
// Rust's diagnostic struct approach
#[derive(Diagnostic)]
#[diag(hir_analysis_type_mismatch, code = "E0308")]
struct TypeMismatch<'a> {
    #[primary_span]
    #[label(hir_analysis_expected_found)]
    span: Span,

    expected: &'a Type,
    found: &'a Type,

    #[suggestion(code = "{suggestion}", applicability = "machine-applicable")]
    suggestion_span: Option<Span>,
    suggestion: Option<String>,
}
```

### 2.3 Suggestion Applicability Levels

| Level | Description | Use Case |
|-------|-------------|----------|
| **MachineApplicable** | Can be applied mechanically | Safe for automated tools (cargo fix) |
| **HasPlaceholders** | Contains placeholder text | "Try adding a type: `let x: <type>`" |
| **MaybeIncorrect** | May or may not be correct | Multiple possible fixes |
| **Unspecified** | Unknown applicability | Fallback default |

### 2.4 annotate-snippets Integration

The `annotate-snippets` crate provides unified diagnostic rendering:
- Terminal support (colors, width adaptation)
- Span and label rendering
- Consistent output across Rust ecosystem (rustc, cargo, clippy)

---

## 3. Elm Error Message Patterns

### 3.1 Key Design Decisions

From Evan Czaplicki's "Compiler Errors for Humans":

1. **No Significant Algorithm Changes**: Rich errors required minimal changes to type inference
2. **Extra Constraint Information**: Each type constraint carries metadata for error generation
3. **Conversational Tone**: Errors feel like dialogue, not technical documentation

### 3.2 Error Categories

| Category | Example | Approach |
|----------|---------|----------|
| **Type Mismatch** | Expected `Int`, got `String` | Show both types with location, suggest conversion |
| **Missing Field** | Record missing `name` field | List required fields, suggest additions |
| **Undefined Variable** | `foo` not in scope | Suggest similar names (typo detection) |
| **Syntax Error** | Unexpected token | Point to location, link to syntax guide |

### 3.3 Example: Elm Type Mismatch

```
-- TYPE MISMATCH ------------------------------------------ src/Main.elm

The 1st argument to `div` is not what I expect:

45|       div [ class "container" ] items
                                    ^^^^^
This `items` value is a:

    List String

But `div` needs the 1st argument to be:

    List (Html msg)

Hint: I see you're trying to display a list of strings. You can use
`text` to convert each string to HTML:

    List.map text items
```

### 3.4 JSON Output for IDE Integration

Elm 0.15.1+ supports `--report=json` for editor plugins:
- Structured diagnostic data
- Jump-to-error functionality
- Inline display in IDE

---

## 4. TypeScript Diagnostic System

### 4.1 Diagnostic Categories

```typescript
enum DiagnosticCategory {
    Warning = 0,
    Error = 1,
    Suggestion = 2,  // Proactive refactoring hints
    Message = 3
}
```

### 4.2 Language Service Integration

TypeScript separates:
- **Compiler diagnostics**: Type errors, syntax errors (enforced)
- **Language Service diagnostics**: Suggestions, refactorings (advisory)

This separation allows IDE features without affecting compilation.

### 4.3 Diagnostic Message Structure

TypeScript's `diagnosticMessages.json` contains thousands of messages with:
- Unique numeric codes
- Parameterized message templates
- Category classification

---

## 5. Aria Diagnostic System Design

### 5.1 TypeDiagnostic Structure

```rust
/// Aria diagnostic representing a compiler message
#[derive(Debug, Clone)]
pub struct TypeDiagnostic {
    /// Unique error code (e.g., "E0001")
    pub code: DiagnosticCode,

    /// Severity level
    pub severity: Severity,

    /// Human-readable title
    pub title: String,

    /// Primary message explaining the error
    pub message: String,

    /// Primary source location
    pub primary_span: DiagnosticSpan,

    /// Additional labels (secondary locations)
    pub labels: Vec<SpanLabel>,

    /// Fix suggestions
    pub suggestions: Vec<Suggestion>,

    /// Related notes
    pub notes: Vec<String>,

    /// Help text
    pub help: Option<String>,

    /// Documentation URL
    pub docs_url: Option<String>,
}

/// Severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

/// A labeled span in source code
#[derive(Debug, Clone)]
pub struct DiagnosticSpan {
    pub file_id: FileId,
    pub span: Span,
    pub source_line: Option<String>,
}

/// Label for a span
#[derive(Debug, Clone)]
pub struct SpanLabel {
    pub span: DiagnosticSpan,
    pub message: String,
    pub style: LabelStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum LabelStyle {
    Primary,   // ^^^^^ underline
    Secondary, // ----- dashed
}

/// A suggested fix
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub message: String,
    pub span: Span,
    pub replacement: String,
    pub applicability: Applicability,
}

/// How confidently a suggestion can be applied
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    /// Can be applied mechanically without further review
    MachineApplicable,
    /// Contains placeholders that require user input
    HasPlaceholders,
    /// May or may not be the correct fix
    MaybeIncorrect,
    /// Applicability unknown
    Unspecified,
}

/// Unique diagnostic code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(pub String);

impl DiagnosticCode {
    pub fn new(category: &str, number: u32) -> Self {
        Self(format!("{}{:04}", category, number))
    }
}
```

### 5.2 Error Code Categories

| Prefix | Category | Examples |
|--------|----------|----------|
| **E0xxx** | Type Errors | E0001 (type mismatch), E0002 (undefined variable) |
| **E1xxx** | Ownership Errors | E1001 (use after move), E1002 (borrow conflict) |
| **E2xxx** | Contract Errors | E2001 (precondition failed), E2002 (postcondition failed) |
| **E3xxx** | Effect Errors | E3001 (unhandled effect), E3002 (effect mismatch) |
| **E4xxx** | Pattern Errors | E4001 (non-exhaustive match), E4002 (unreachable pattern) |
| **W0xxx** | Warnings | W0001 (unused variable), W0002 (dead code) |
| **S0xxx** | Suggestions | S0001 (simplify expression), S0002 (extract function) |

### 5.3 TypeError to Diagnostic Transformation

```rust
impl From<TypeError> for TypeDiagnostic {
    fn from(error: TypeError) -> Self {
        match error {
            TypeError::Mismatch { expected, found, span } => {
                TypeDiagnostic {
                    code: DiagnosticCode::new("E", 1),
                    severity: Severity::Error,
                    title: "type mismatch".to_string(),
                    message: format!(
                        "We expected `{}`, but found `{}`.",
                        expected, found
                    ),
                    primary_span: DiagnosticSpan::from_span(span),
                    labels: vec![
                        SpanLabel {
                            span: DiagnosticSpan::from_span(span),
                            message: format!("expected `{}` here", expected),
                            style: LabelStyle::Primary,
                        }
                    ],
                    suggestions: generate_type_suggestions(&expected, &found, span),
                    notes: vec![],
                    help: Some(format!(
                        "The expression has type `{}`, but context expects `{}`.",
                        found, expected
                    )),
                    docs_url: Some("https://aria-lang.org/errors/E0001".to_string()),
                }
            }

            TypeError::UndefinedVariable(name, span) => {
                let similar = find_similar_names(&name);
                TypeDiagnostic {
                    code: DiagnosticCode::new("E", 2),
                    severity: Severity::Error,
                    title: "undefined variable".to_string(),
                    message: format!("We cannot find `{}` in this scope.", name),
                    primary_span: DiagnosticSpan::from_span(span),
                    labels: vec![
                        SpanLabel {
                            span: DiagnosticSpan::from_span(span),
                            message: "not found in this scope".to_string(),
                            style: LabelStyle::Primary,
                        }
                    ],
                    suggestions: similar.iter().map(|s| Suggestion {
                        message: format!("Did you mean `{}`?", s),
                        span,
                        replacement: s.clone(),
                        applicability: Applicability::MaybeIncorrect,
                    }).collect(),
                    notes: vec![],
                    help: if similar.is_empty() {
                        Some("Check that the variable is declared before use.".to_string())
                    } else {
                        None
                    },
                    docs_url: Some("https://aria-lang.org/errors/E0002".to_string()),
                }
            }

            TypeError::UndefinedType(name, span) => {
                TypeDiagnostic {
                    code: DiagnosticCode::new("E", 3),
                    severity: Severity::Error,
                    title: "undefined type".to_string(),
                    message: format!("The type `{}` is not defined.", name),
                    primary_span: DiagnosticSpan::from_span(span),
                    labels: vec![
                        SpanLabel {
                            span: DiagnosticSpan::from_span(span),
                            message: "undefined type".to_string(),
                            style: LabelStyle::Primary,
                        }
                    ],
                    suggestions: vec![],
                    notes: vec![
                        "Built-in types: Int, Float, String, Bool, Array, Map, Option, Result".to_string()
                    ],
                    help: Some("Check your imports or define this type.".to_string()),
                    docs_url: Some("https://aria-lang.org/errors/E0003".to_string()),
                }
            }

            TypeError::CannotInfer(span) => {
                TypeDiagnostic {
                    code: DiagnosticCode::new("E", 4),
                    severity: Severity::Error,
                    title: "cannot infer type".to_string(),
                    message: "We cannot determine the type here.".to_string(),
                    primary_span: DiagnosticSpan::from_span(span),
                    labels: vec![
                        SpanLabel {
                            span: DiagnosticSpan::from_span(span),
                            message: "type annotation needed".to_string(),
                            style: LabelStyle::Primary,
                        }
                    ],
                    suggestions: vec![
                        Suggestion {
                            message: "Consider adding a type annotation".to_string(),
                            span,
                            replacement: ": <type>".to_string(),
                            applicability: Applicability::HasPlaceholders,
                        }
                    ],
                    notes: vec![],
                    help: Some("Add an explicit type annotation to help the compiler.".to_string()),
                    docs_url: Some("https://aria-lang.org/errors/E0004".to_string()),
                }
            }

            TypeError::RecursiveType(span) => {
                TypeDiagnostic {
                    code: DiagnosticCode::new("E", 5),
                    severity: Severity::Error,
                    title: "recursive type".to_string(),
                    message: "This type refers to itself, creating an infinite size.".to_string(),
                    primary_span: DiagnosticSpan::from_span(span),
                    labels: vec![
                        SpanLabel {
                            span: DiagnosticSpan::from_span(span),
                            message: "recursive type detected".to_string(),
                            style: LabelStyle::Primary,
                        }
                    ],
                    suggestions: vec![],
                    notes: vec![
                        "Recursive types must use indirection (Box, Ref) to have finite size.".to_string()
                    ],
                    help: Some("Wrap the recursive field in `Box<T>` or use a reference.".to_string()),
                    docs_url: Some("https://aria-lang.org/errors/E0005".to_string()),
                }
            }

            TypeError::WrongTypeArity { expected, found, span } => {
                TypeDiagnostic {
                    code: DiagnosticCode::new("E", 6),
                    severity: Severity::Error,
                    title: "wrong number of type arguments".to_string(),
                    message: format!(
                        "We expected {} type argument{}, but found {}.",
                        expected,
                        if expected == 1 { "" } else { "s" },
                        found
                    ),
                    primary_span: DiagnosticSpan::from_span(span),
                    labels: vec![
                        SpanLabel {
                            span: DiagnosticSpan::from_span(span),
                            message: format!(
                                "expected {} type argument{}, found {}",
                                expected,
                                if expected == 1 { "" } else { "s" },
                                found
                            ),
                            style: LabelStyle::Primary,
                        }
                    ],
                    suggestions: vec![],
                    notes: vec![],
                    help: Some("Check the generic type definition for required arguments.".to_string()),
                    docs_url: Some("https://aria-lang.org/errors/E0006".to_string()),
                }
            }
        }
    }
}
```

### 5.4 Fix Suggestion Algorithm

```rust
/// Generate type-aware suggestions
fn generate_type_suggestions(expected: &str, found: &str, span: Span) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    // String <-> Int conversion
    if expected == "String" && found == "Int" {
        suggestions.push(Suggestion {
            message: "Convert the integer to a string".to_string(),
            span,
            replacement: ".to_s".to_string(),
            applicability: Applicability::MachineApplicable,
        });
    }

    if expected == "Int" && found == "String" {
        suggestions.push(Suggestion {
            message: "Parse the string as an integer".to_string(),
            span,
            replacement: ".parse_int()?".to_string(),
            applicability: Applicability::MaybeIncorrect,
        });
    }

    // Optional wrapping
    if expected.starts_with("Option") && !found.starts_with("Option") {
        suggestions.push(Suggestion {
            message: "Wrap the value in Some".to_string(),
            span,
            replacement: "Some(...)".to_string(),
            applicability: Applicability::HasPlaceholders,
        });
    }

    // Result wrapping
    if expected.starts_with("Result") && !found.starts_with("Result") {
        suggestions.push(Suggestion {
            message: "Wrap the value in Ok".to_string(),
            span,
            replacement: "Ok(...)".to_string(),
            applicability: Applicability::HasPlaceholders,
        });
    }

    // Numeric coercion
    if is_numeric(expected) && is_numeric(found) {
        suggestions.push(Suggestion {
            message: format!("Convert to `{}`", expected),
            span,
            replacement: format!(".to_{}", expected.to_lowercase()),
            applicability: Applicability::MaybeIncorrect,
        });
    }

    suggestions
}

/// Find similar variable names using edit distance
fn find_similar_names(name: &str) -> Vec<String> {
    // Implementation would use Levenshtein distance
    // or other fuzzy matching algorithm
    vec![]
}
```

### 5.5 Diagnostic Renderer

```rust
/// Render diagnostics to terminal output
pub struct DiagnosticRenderer {
    use_colors: bool,
    terminal_width: usize,
}

impl DiagnosticRenderer {
    pub fn render(&self, diag: &TypeDiagnostic, source: &SourceMap) -> String {
        let mut output = String::new();

        // Error code and title (colored red for errors)
        let severity_color = match diag.severity {
            Severity::Error => Color::Red,
            Severity::Warning => Color::Yellow,
            Severity::Note | Severity::Help => Color::Blue,
        };

        output.push_str(&format!(
            "{}: {}\n",
            self.colorize(&format!("{}[{}]", diag.severity, diag.code.0), severity_color),
            self.colorize(&diag.title, Color::Bold)
        ));

        // Location
        let loc = &diag.primary_span;
        output.push_str(&format!(
            " {} {}:{}:{}\n",
            self.colorize("-->", Color::Blue),
            source.file_name(loc.file_id),
            source.line_number(loc.span.start),
            source.column(loc.span.start)
        ));

        // Source context with labels
        output.push_str(&self.render_source_context(diag, source));

        // Suggestions
        for suggestion in &diag.suggestions {
            output.push_str(&format!(
                "  {} {}\n",
                self.colorize("help:", Color::Cyan),
                suggestion.message
            ));
            if suggestion.applicability == Applicability::MachineApplicable {
                output.push_str(&format!(
                    "   {} {}\n",
                    self.colorize("|", Color::Blue),
                    suggestion.replacement
                ));
            }
        }

        // Notes
        for note in &diag.notes {
            output.push_str(&format!(
                "  {} {}\n",
                self.colorize("note:", Color::Blue),
                note
            ));
        }

        // Help
        if let Some(help) = &diag.help {
            output.push_str(&format!(
                "  {} {}\n",
                self.colorize("help:", Color::Cyan),
                help
            ));
        }

        // Documentation link
        if let Some(docs) = &diag.docs_url {
            output.push_str(&format!(
                "  {} {}\n",
                self.colorize("docs:", Color::Blue),
                docs
            ));
        }

        output
    }
}
```

---

## 6. Example Error Outputs

### 6.1 Type Mismatch

```
error[E0001]: type mismatch
 --> src/main.aria:42:21
  |
42 |   let name: String = get_id()
  |       ----            ^^^^^^^^ expected `String`, found `Int`
  |       |
  |       expected due to this type annotation
  |
  help: Convert the integer to a string
  |
42 |   let name: String = get_id().to_s
  |                              +++++
  |
  docs: https://aria-lang.org/errors/E0001
```

### 6.2 Undefined Variable

```
error[E0002]: undefined variable
 --> src/main.aria:15:10
  |
15 |   print(mesage)
  |         ^^^^^^ not found in this scope
  |
  help: Did you mean `message`?
  |
15 |   print(message)
  |         ~~~~~~~
  |
  docs: https://aria-lang.org/errors/E0002
```

### 6.3 Cannot Infer Type

```
error[E0004]: cannot infer type
 --> src/main.aria:8:9
  |
8 |   let x = []
  |       ^ type annotation needed
  |
  help: Consider adding a type annotation
  |
8 |   let x: Array[Int] = []
  |        +++++++++++
  |
  note: Empty collections require type annotations since we cannot
        infer the element type from context.
  |
  docs: https://aria-lang.org/errors/E0004
```

### 6.4 Contract Violation

```
error[E2001]: precondition failed
 --> src/math.aria:24:3
  |
22 | fn sqrt(x: Float) -> Float
23 |   requires x >= 0.0 : "input must be non-negative"
  |            -------- precondition defined here
24 |   sqrt(-4.0)
  |   ^^^^^^^^^^ precondition `x >= 0.0` not satisfied
  |
  note: The function `sqrt` requires its input to be non-negative,
        but `-4.0` is negative.
  |
  help: Ensure the value is non-negative before calling sqrt:
  |
24 |   sqrt(x.abs)
  |        ~~~~~~
  |
  docs: https://aria-lang.org/errors/E2001
```

### 6.5 Non-Exhaustive Pattern Match

```
error[E4001]: non-exhaustive patterns
 --> src/main.aria:12:3
  |
12 |   match status
13 |     Ok(value) => handle(value)
  |     --------- pattern `Err(_)` not covered
  |
  note: `Result<T, E>` has two variants: `Ok(_)` and `Err(_)`
  |
  help: Ensure all variants are handled:
  |
13 |     Ok(value) => handle(value)
14 |     Err(e) => handle_error(e)
  |     +++++++++++++++++++++++++
  |
  docs: https://aria-lang.org/errors/E4001
```

---

## 7. JSON Output Format

For IDE integration, Aria will support `--output-format=json`:

```json
{
  "diagnostics": [
    {
      "code": "E0001",
      "severity": "error",
      "title": "type mismatch",
      "message": "We expected `String`, but found `Int`.",
      "file": "src/main.aria",
      "line": 42,
      "column": 21,
      "end_line": 42,
      "end_column": 29,
      "labels": [
        {
          "message": "expected `String` here",
          "line": 42,
          "column": 21,
          "style": "primary"
        }
      ],
      "suggestions": [
        {
          "message": "Convert the integer to a string",
          "replacement": ".to_s",
          "applicability": "machine-applicable",
          "line": 42,
          "column": 29
        }
      ],
      "help": "The expression has type `Int`, but context expects `String`.",
      "docs_url": "https://aria-lang.org/errors/E0001"
    }
  ],
  "summary": {
    "errors": 1,
    "warnings": 0
  }
}
```

---

## 8. Implementation Roadmap

### Phase 1: Foundation (MVP)
- [ ] Define `TypeDiagnostic` struct
- [ ] Implement `From<TypeError>` for all variants
- [ ] Basic terminal renderer (no colors)
- [ ] Error codes for type errors

### Phase 2: Enhanced UX
- [ ] Color support with terminal detection
- [ ] Source context rendering with line numbers
- [ ] Suggestion generation algorithms
- [ ] Similar name detection (Levenshtein distance)

### Phase 3: IDE Integration
- [ ] JSON output format
- [ ] LSP diagnostic conversion
- [ ] Documentation URL generation
- [ ] Machine-applicable fixes for LSP code actions

### Phase 4: Advanced Features
- [ ] Multi-file diagnostics (error chains)
- [ ] Contract-aware diagnostics
- [ ] Effect system diagnostics
- [ ] Ownership diagnostics

---

## 9. Key Resources

1. [Elm - Compiler Errors for Humans](https://elm-lang.org/news/compiler-errors-for-humans)
2. [Rust Compiler Development Guide - Diagnostics](https://rustc-dev-guide.rust-lang.org/diagnostics.html)
3. [Rust Diagnostic Structs Guide](https://rustc-dev-guide.rust-lang.org/diagnostics/diagnostic-structs.html)
4. [annotate-snippets Crate](https://crates.io/crates/annotate-snippets)
5. [TypeScript Diagnostic Messages](https://github.com/microsoft/TypeScript/blob/main/src/compiler/diagnosticMessages.json)
6. [Writing Good Compiler Error Messages](https://calebmer.com/2019/07/01/writing-good-compiler-error-messages.html)

---

## 10. Success Criteria

- [ ] All `TypeError` variants have corresponding `TypeDiagnostic` transformations
- [ ] Error messages show exact source location with context
- [ ] Plain English explanations (no compiler jargon)
- [ ] Fix suggestions with applicability levels
- [ ] Documentation links for all error codes
- [ ] JSON output for IDE integration
- [ ] Terminal rendering with color support

---

## 11. Open Questions

1. **Error Code Stability**: Should error codes be stable across versions, or can they change?
2. **Localization**: Should error messages support multiple languages?
3. **Error Catalog**: How do we maintain the error documentation site?
4. **LSP Integration**: Should suggestions be LSP code actions by default?

---

## Appendix A: Complete Error Code Reference

| Code | Category | Title | Machine Fix |
|------|----------|-------|-------------|
| E0001 | Type | Type mismatch | Sometimes |
| E0002 | Type | Undefined variable | Typo correction |
| E0003 | Type | Undefined type | No |
| E0004 | Type | Cannot infer type | Placeholder |
| E0005 | Type | Recursive type | No |
| E0006 | Type | Wrong type arity | No |
| E1001 | Ownership | Use after move | No |
| E1002 | Ownership | Borrow conflict | No |
| E2001 | Contract | Precondition failed | No |
| E2002 | Contract | Postcondition failed | No |
| E3001 | Effect | Unhandled effect | Add handler |
| E4001 | Pattern | Non-exhaustive match | Add arm |
| W0001 | Warning | Unused variable | Prefix with _ |
| W0002 | Warning | Dead code | Remove |
