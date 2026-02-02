# ARIA-PD-014: Error Message System Design

**Decision ID**: ARIA-PD-014
**Status**: Approved
**Date**: 2026-01-15
**Author**: HERALD-II (Product Decision Agent)
**Research Inputs**:
- ARIA-M18-05: Error Message User Experience Research (SCRIBE)

---

## Executive Summary

This document defines Aria's error message system design, synthesizing UX research from SCRIBE's comprehensive analysis of Elm, Rust, and academic research on compiler diagnostics. The goal is to create error messages that embody Aria's philosophy: "The compiler is your pair programming partner."

**Final Decisions**:
- **Format**: Structured multi-line format with error code, location, context, and actionable suggestions
- **Voice**: First-person plural ("We found...") for collaborative tone
- **Color**: Semantic color system with full accessibility compliance (WCAG AA)
- **Cascades**: Smart grouping with root-cause identification; default limit of 10 errors
- **Suggestions**: Four-tier applicability system with confidence-based presentation

---

## 1. Error Message Format Specification

### 1.1 Canonical Error Format

```
error[EXXXX]: <brief description in plain English>
 --> <file>:<line>:<column>
  |
NN | <source code line>
  |   <underline> <inline explanation>
  |
  <secondary spans if present>
  |
  help: <actionable suggestion>
  |
NN |   <suggested fix with highlighting>
  |   <change markers>
  |
  docs: https://aria-lang.org/errors/EXXXX
```

### 1.2 Format Components

| Component | Purpose | Required |
|-----------|---------|----------|
| `error[EXXXX]` | Severity + unique error code | Yes |
| Brief description | One-line summary in plain English | Yes |
| Location arrow | File, line, column pointer | Yes |
| Source context | User's code exactly as written | Yes |
| Primary underline | `^^^` pointing to error location | Yes |
| Inline explanation | What's wrong at this location | Yes |
| Secondary spans | Related locations explaining why | Conditional |
| Help section | Actionable fix suggestion | Recommended |
| Code suggestion | Concrete fix with diff markers | Conditional |
| Docs link | URL to extended documentation | Yes |

### 1.3 Error Severity Levels

| Level | Prefix | Underline | Behavior |
|-------|--------|-----------|----------|
| **Error** | `error` | `^^^` | Blocks compilation |
| **Warning** | `warning` | `~~~` | Compilation continues |
| **Note** | `note` | `---` | Informational, attached to errors |
| **Help** | `help` | `+++` | Suggestion for fixing |

### 1.4 Error Code Registry

Error codes follow the pattern `EXXXX` where:
- First digit indicates category
- Remaining digits are sequential within category

| Range | Category | Examples |
|-------|----------|----------|
| E0XXX | Core/General | E0001 Type mismatch |
| E1XXX | Naming/Scope | E1001 Undefined variable |
| E2XXX | Ownership/Borrowing | E2001 Use after move |
| E3XXX | Syntax/Parsing | E3001 Unexpected token |
| E4XXX | Pattern Matching | E4001 Non-exhaustive patterns |
| E5XXX | Effects | E5001 Unhandled effect |
| E6XXX | Concurrency | E6001 Data race detected |
| E7XXX | FFI/Interop | E7001 Invalid C type |
| E8XXX | Contracts | E8001 Precondition violated |
| E9XXX | Internal/ICE | E9001 Internal compiler error |

---

## 2. Context Inclusion Rules

### 2.1 Primary Context Window

| Scenario | Lines Above | Lines Below | Total Max |
|----------|-------------|-------------|-----------|
| Single-line error | 2 | 1 | 4 |
| Multi-line span (2-5 lines) | 1 | 1 | span + 2 |
| Multi-line span (6+ lines) | 1 | 1 | Fold middle |
| Function boundary | 0 | 0 | Show signature only |

### 2.2 Secondary Span Rules

Secondary spans answer "why" the error occurred. Include when:

| Condition | Include Secondary? | Example |
|-----------|-------------------|---------|
| Type annotation caused mismatch | Yes | "expected due to this annotation" |
| Variable definition relevant | Yes | "move occurs because..." |
| Function signature constrains | Yes | "required by this signature" |
| Import affects scope | Yes | "imported here" |
| Generic instantiation | Yes | "type parameter bound here" |
| Distance > 20 lines | Yes, show both | Always show context |
| Same line | No | Combine into primary |

### 2.3 Span Folding Algorithm

```
IF span_length > 5 lines:
  show: first 2 lines
  fold: middle (show line count)
  show: last 2 lines

IF span_length > 15 lines:
  show: first line
  fold: middle
  show: error line +/- 1
  fold: middle
  show: last line
```

**Folding indicator**: Use `...` with line count

```
   |
 5 |   fn complex_function() {
   | ...
   | (38 lines omitted)
   | ...
45 |       problematic_call()
   |       ^^^^^^^^^^^^^^^^^^ error here
   | ...
89 |   }
   |
```

### 2.4 Multi-File Context

When errors span multiple files:

```
error[E0001]: type mismatch
 --> src/handlers.aria:15:10
  |
15 |   return calculate(data)
  |          ^^^^^^^^^^^^^^^^ expected `Result[User, Error]`, found `User`
  |
 ::: src/types.aria:42:1
  |
42 | fn calculate(input: Data) -> User
  |                               ---- return type defined here
  |
  help: wrap the return value in `Ok`
  |
15 |   return Ok(calculate(data))
  |          +++              +++
```

---

## 3. Fix Suggestion Generation

### 3.1 Suggestion Applicability Tiers

| Tier | Name | Confidence | Presentation | Auto-Apply |
|------|------|------------|--------------|------------|
| 1 | MachineApplicable | 100% | Direct suggestion | `aria fix` |
| 2 | HasPlaceholders | >90% | Template with `<...>` | No |
| 3 | MaybeIncorrect | 60-90% | "Consider..." | No |
| 4 | Educational | <60% | "See docs on..." | No |

### 3.2 Suggestion Presentation Rules

**Tier 1 - MachineApplicable**:
```
  help: convert to string
  |
15 |   let name: String = count.to_string()
  |                            +++++++++++
```

**Tier 2 - HasPlaceholders**:
```
  help: add the missing field
  |
15 |   User { name: "Alice", <age: Int>, email: "..." }
  |                          ^^^^^^^^^^
```

**Tier 3 - MaybeIncorrect**:
```
  help: consider cloning the value if you need to use it again
  |
15 |   consume(data.clone())
  |               ++++++++
```

**Tier 4 - Educational**:
```
  help: see documentation on ownership and borrowing
  docs: https://aria-lang.org/learn/ownership
```

### 3.3 "Did You Mean" Algorithm

**Step 1: Candidate Collection**
- Same scope: variables, functions, types in current scope
- Parent scopes: up to 3 levels
- Imports: available but not imported names
- Stdlib: common functions matching pattern

**Step 2: Distance Calculation**
```
score = base_levenshtein_distance(query, candidate)

// Adjust for common typos
if is_transposition(query, candidate):
    score -= 0.5  // Favor transpositions

if is_adjacent_key_substitution(query, candidate):
    score -= 0.3  // Favor keyboard neighbors

// Adjust for scope proximity
if same_scope:
    score -= 1.0
elif parent_scope:
    score -= 0.5
```

**Step 3: Threshold and Selection**
| Query Length | Max Distance | Show Suggestion |
|--------------|--------------|-----------------|
| 1-4 chars | 1 | Yes |
| 5-8 chars | 2 | Yes |
| 9+ chars | 3 | Yes |
| Any | >3 | No |

**Multiple suggestions**: Show up to 3 if within threshold, ordered by score.

### 3.4 Type-Aware Suggestions

| Mismatch | Confidence | Suggestion |
|----------|------------|------------|
| `Int` -> `String` | High | `.to_string()` |
| `T` -> `Option[T]` | High | `Some(value)` |
| `T` -> `Result[T,E]` | High | `Ok(value)` |
| `T` -> `Vec[T]` | Medium | `vec![value]` |
| `Option[T]` -> `T` | Medium | `.unwrap()` or pattern match |
| `&T` -> `T` | High | `.clone()` or `*ref` |
| `T` -> `&T` | High | `&value` |
| Generic mismatch | Low | Educational link |

### 3.5 Common Error Patterns with Fixes

**Missing Import**:
```
error[E1002]: `HashMap` is not in scope
 --> src/main.aria:5:10
  |
5 |   let map: HashMap[String, Int] = ...
  |            ^^^^^^^ not found in this scope
  |
  help: add this import
  |
1 | use std::collections::HashMap
  | +++++++++++++++++++++++++++++
```

**Missing Method**:
```
error[E1003]: no method `length` on `String`
 --> src/main.aria:10:15
  |
10|   let n = text.length()
  |                ^^^^^^ method not found
  |
  help: did you mean `len`?
  |
10|   let n = text.len()
  |                ~~~
```

**Ownership Move**:
```
error[E2001]: use of moved value
 --> src/main.aria:15:10
  |
12|   let data = vec![1, 2, 3]
  |       ---- move occurs because `data` has type `Vec[Int]`
13|
14|   consume(data)
  |           ---- value moved here
15|   print(data)
  |         ^^^^ value used after move
  |
  help: consider cloning if you need to use the value again
  |
14|   consume(data.clone())
  |               ++++++++
  |
  help: or borrow instead of moving
  |
14|   consume(&data)
  |           +
```

---

## 4. Color and Formatting Guidelines

### 4.1 Semantic Color Palette

**DECISION**: Use semantic color names internally, mapped to ANSI codes per theme.

| Semantic Name | Purpose | Dark Theme | Light Theme | High Contrast |
|---------------|---------|------------|-------------|---------------|
| `DiagError` | Error messages | Bright Red (#FF6B6B) | Red (#CC0000) | Bold Red |
| `DiagWarning` | Warnings | Yellow (#FFE66D) | Orange (#B35900) | Bold Yellow |
| `DiagNote` | Notes/info | Cyan (#4ECDC4) | Blue (#0066CC) | Bold Cyan |
| `DiagHelp` | Suggestions | Green (#95E1D3) | Green (#006600) | Bold Green |
| `DiagEmphasis` | Important text | Bold White | Bold Black | Bold |
| `DiagCode` | Source code | Light Gray (#F8F8F2) | Dark Gray (#333333) | White/Black |
| `DiagLineNum` | Line numbers | Dim Cyan (#6B7280) | Dim Blue (#666666) | Normal |
| `DiagSeparator` | Pipes, arrows | Blue (#4A90D9) | Blue (#0066CC) | Bold Blue |

### 4.2 Colorblind-Safe Alternatives

**DECISION**: Provide a CVD (Color Vision Deficiency) mode activated via `--color-mode=cvd` or `ARIA_COLOR_MODE=cvd`.

| Element | Standard | CVD Mode |
|---------|----------|----------|
| Error | Red | Orange (#D55E00) |
| Warning | Yellow | Yellow (#F0E442) |
| Note | Cyan | Blue (#0072B2) |
| Help | Green | Blue (#0072B2) |
| Success | Green | Blue (#0072B2) |

### 4.3 Color + Symbol Pairing

**DECISION**: NEVER rely on color alone. Always pair with text/symbol.

| Level | Color | Text Prefix | Underline | Symbol |
|-------|-------|-------------|-----------|--------|
| Error | Red | `error:` | `^^^` | None |
| Warning | Yellow | `warning:` | `~~~` | None |
| Note | Cyan | `note:` | `---` | None |
| Help | Green | `help:` | `+++` | None |
| Success | Green | Text | N/A | Checkmark (optional) |

### 4.4 Color Mode Detection

```
Priority order (highest to lowest):
1. --color=never|always|auto flag
2. ARIA_COLOR_MODE environment variable
3. NO_COLOR environment variable (if set, disable color)
4. TERM=dumb (if set, disable color)
5. TTY detection (if not TTY, disable color)
6. Default: auto (color if TTY)
```

### 4.5 Output Modes

| Mode | Trigger | Output |
|------|---------|--------|
| Full Color | TTY + supported terminal | ANSI 256 or 24-bit |
| Basic Color | TTY + basic terminal | ANSI 16 colors |
| No Color | `--color=never`, `NO_COLOR`, pipe | Plain text |
| Machine | `--format=json` | Structured JSON |
| Short | `--format=short` | One line per error |

### 4.6 Line Length and Wrapping

**DECISION**: Default line width of 100 characters, configurable.

| Content | Wrapping Behavior |
|---------|-------------------|
| Error message text | Wrap at word boundary |
| Source code | Never wrap, truncate with `...` if > width |
| Suggestions | Wrap, maintain alignment |
| File paths | Truncate middle with `...` if > 60 chars |

---

## 5. Error Cascade Prevention

### 5.1 Cascade Detection Strategy

**DECISION**: Implement "poison type" approach with cascade grouping.

```
Cascade Detection Algorithm:

1. When error occurs on variable/type X:
   - Mark X as "poisoned" with error ID

2. Subsequent errors involving poisoned X:
   - Tag as "cascade" with parent error ID
   - Still generate error but mark as derivative

3. Display grouping:
   - Show root cause prominently
   - Show derivatives grouped/collapsed
   - Indicate cascade relationship
```

### 5.2 Display Limits

| Mode | Default Limit | Behavior |
|------|---------------|----------|
| Normal | 10 errors | Show first 10, then summary |
| Strict | 1 error | Stop at first error (Elm-style) |
| Verbose | Unlimited | Show all errors |
| Per-file | 5 errors/file | Limit per file |

**Command-line control**:
- `--error-limit=N` - Set limit (0 for unlimited)
- `--strict` - Equivalent to `--error-limit=1`
- `--error-format=grouped` - Group by root cause

### 5.3 Cascade Grouping Display

```
error[E1001]: undefined variable `usre`
 --> src/main.aria:3:5
  |
3 | let usre = get_user()
  |     ^^^^ not found in this scope
  |
  help: did you mean `user`?
  |
3 | let user = get_user()
  |     ~~~~

note: this error likely caused 3 additional errors below
      fix this error first and recompile

  error[E1001]: undefined variable `user` at src/main.aria:5:10
  error[E1001]: undefined variable `user` at src/main.aria:8:15
  error[E0001]: type mismatch at src/main.aria:12:5

Showing 4 errors (1 root cause + 3 cascading).
Run with --error-limit=0 to show all details.
```

### 5.4 Root Cause Heuristics

| Signal | Root Cause Likelihood |
|--------|----------------------|
| First error in file | High |
| Error on definition vs use | Definition is root |
| Parse error before type error | Parse is root |
| Undefined name error | High (often typo) |
| Multiple errors same span | First is root |
| Import/module error | Very high |

### 5.5 Summary Footer

Always end error output with a summary:

```
// Single error
error: aborting due to 1 error
docs: https://aria-lang.org/errors/E1001

// Multiple errors
Showing 6 of 23 errors (17 hidden as cascading errors).
Run `aria check --error-limit=0` to see all errors.

// Warnings only
Compilation succeeded with 3 warnings.
Run `aria check --strict-warnings` to treat warnings as errors.
```

---

## 6. JSON Output Format

### 6.1 Schema Definition

```json
{
  "$schema": "https://aria-lang.org/schemas/diagnostic-v1.json",
  "diagnostics": [
    {
      "code": "E1001",
      "severity": "error",
      "message": "undefined variable `usre`",
      "source": "aria",
      "primarySpan": {
        "file": "src/main.aria",
        "startLine": 3,
        "startColumn": 5,
        "endLine": 3,
        "endColumn": 9
      },
      "secondarySpans": [],
      "labels": [
        {
          "span": { "startLine": 3, "startColumn": 5, "endLine": 3, "endColumn": 9 },
          "message": "not found in this scope",
          "style": "primary"
        }
      ],
      "suggestions": [
        {
          "message": "did you mean `user`?",
          "applicability": "machine-applicable",
          "edits": [
            {
              "file": "src/main.aria",
              "range": { "startLine": 3, "startColumn": 5, "endLine": 3, "endColumn": 9 },
              "newText": "user"
            }
          ]
        }
      ],
      "relatedErrors": ["E1001-2", "E1001-3", "E0001-4"],
      "isRootCause": true,
      "documentation": "https://aria-lang.org/errors/E1001"
    }
  ],
  "summary": {
    "errorCount": 4,
    "warningCount": 0,
    "rootCauseCount": 1,
    "cascadeCount": 3
  }
}
```

### 6.2 LSP Diagnostic Mapping

| Aria Field | LSP Field |
|------------|-----------|
| `code` | `code` |
| `severity` | `severity` (1=Error, 2=Warning, 3=Info, 4=Hint) |
| `message` | `message` |
| `primarySpan` | `range` |
| `secondarySpans` | `relatedInformation` |
| `suggestions` | `data.suggestions` (for Code Actions) |
| `documentation` | `codeDescription.href` |

---

## 7. Error Message Templates

### 7.1 Type Mismatch (E0001)

```
error[E0001]: type mismatch
 --> {file}:{line}:{col}
  |
{line} | {code}
  |   {underline} expected `{expected}`, found `{found}`
  |
{if annotation_span}
 ::: {annotation_file}:{annotation_line}
  |
{annotation_line} | {annotation_code}
  |   {annotation_underline} expected due to this annotation
{endif}
  |
{if suggestion}
  help: {suggestion_message}
  |
{line} |   {suggestion_code}
  |   {suggestion_markers}
{endif}
  |
  docs: https://aria-lang.org/errors/E0001
```

### 7.2 Undefined Variable (E1001)

```
error[E1001]: cannot find `{name}` in this scope
 --> {file}:{line}:{col}
  |
{line} | {code}
  |   {underline} not found in this scope
  |
{if similar_names}
  help: a {kind} with a similar name exists
  |
{line} |   {suggestion_code}
  |   {suggestion_markers}
{endif}
{if needs_import}
  help: consider importing `{import_path}`
  |
1 | use {import_path}
  | ++++++++++++++++++
{endif}
  |
  docs: https://aria-lang.org/errors/E1001
```

### 7.3 Non-Exhaustive Patterns (E4001)

```
error[E4001]: non-exhaustive patterns
 --> {file}:{line}:{col}
  |
{line} | match {scrutinee} {
  |       ^^^^^ patterns not exhaustive
  |
  note: the following patterns are not covered:
{for pattern in missing_patterns}
        - `{pattern}`
{endfor}
  |
  help: add the missing patterns or use a wildcard
  |
{line} |     {missing_pattern} => <handle case>,
  |     +++++++++++++++++++++++++++++++++++++++
  |
  help: or use `_` as a catch-all
  |
{line} |     _ => <default>,
  |     +++++++++++++++++++
  |
  docs: https://aria-lang.org/errors/E4001
```

### 7.4 Use After Move (E2001)

```
error[E2001]: use of moved value `{name}`
 --> {file}:{line}:{col}
  |
{def_line} | {def_code}
  |   {def_underline} move occurs because `{name}` has type `{type}`
  |                   which does not implement `Copy`
  |
{move_line} | {move_code}
  |   {move_underline} value moved here
  |
{use_line} | {use_code}
  |   {use_underline} value used after move
  |
{if can_clone}
  help: consider cloning the value
  |
{move_line} |   {clone_suggestion}
  |   {clone_markers}
{endif}
{if can_borrow}
  help: consider borrowing instead
  |
{move_line} |   {borrow_suggestion}
  |   {borrow_markers}
{endif}
  |
  docs: https://aria-lang.org/errors/E2001
```

---

## 8. Implementation Phases

### 8.1 Phase 1: Foundation (Week 1-2)

| Task | Description | Priority |
|------|-------------|----------|
| DiagnosticRenderer | Core rendering engine with plain text | P0 |
| SpanTracker | Source location tracking | P0 |
| ErrorRegistry | Initial E0001-E1099 codes | P0 |
| Basic Levenshtein | Simple "did you mean" | P1 |

### 8.2 Phase 2: Enhanced UX (Week 3-4)

| Task | Description | Priority |
|------|-------------|----------|
| ANSI Colors | Full color support + NO_COLOR | P0 |
| Secondary Spans | Multi-location errors | P0 |
| Cascade Detection | Poison types, grouping | P1 |
| Suggestion Tiers | Applicability framework | P1 |

### 8.3 Phase 3: Tooling Integration (Week 5-6)

| Task | Description | Priority |
|------|-------------|----------|
| JSON Output | `--format=json` flag | P0 |
| LSP Mapping | Diagnostic to LSP conversion | P0 |
| Code Actions | Suggestion to LSP Code Action | P1 |
| Short Format | One-line per error mode | P2 |

### 8.4 Phase 4: Polish (Week 7-8)

| Task | Description | Priority |
|------|-------------|----------|
| CVD Mode | Colorblind-safe theme | P1 |
| Error Docs | Website with E0001+ pages | P1 |
| Error Catalog | Test suite for all errors | P1 |
| Localization | i18n framework setup | P2 |

---

## 9. Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Readability | Grade 8 reading level | Hemingway Editor audit |
| Typo suggestion accuracy | >85% correct | User study |
| Time to first fix | <30 seconds average | User study |
| Cascade ratio | <2.5 shown per root cause | Automated analysis |
| Accessibility | WCAG AA compliance | Accessibility audit |
| LSP response time | <100ms diagnostic refresh | Performance benchmark |
| User satisfaction | >4.0/5.0 rating | Survey |

---

## 10. Open Questions for Future Iterations

| Question | Current Decision | Revisit When |
|----------|-----------------|--------------|
| Multi-language error messages | English only for v1 | User base grows internationally |
| Verbose/beginner mode | Not in v1 | User feedback indicates need |
| Error telemetry | Opt-in only, not in v1 | Infrastructure ready |
| Custom library errors | Not in v1 | Effect system mature |
| LLM-powered explanations | Not in v1 | Privacy/determinism concerns resolved |

---

## Appendix A: Example Error Output Gallery

### A.1 Simple Type Mismatch

```
error[E0001]: type mismatch
 --> src/main.aria:10:20
   |
10 |   let name: String = 42
   |                      ^^ expected `String`, found `Int`
   |
   help: convert to string
   |
10 |   let name: String = 42.to_string()
   |                        +++++++++++
   |
   docs: https://aria-lang.org/errors/E0001
```

### A.2 Undefined with Suggestion

```
error[E1001]: cannot find `pritn` in this scope
 --> src/main.aria:5:3
  |
5 |   pritn("Hello")
  |   ^^^^^ not found in this scope
  |
  help: did you mean `print`?
  |
5 |   print("Hello")
  |   ~~~~~
  |
  docs: https://aria-lang.org/errors/E1001
```

### A.3 Multi-Span Ownership Error

```
error[E2001]: use of moved value `config`
 --> src/server.aria:45:12
   |
32 |   let config = load_config()
   |       ------ move occurs because `config` has type `Config`
...
40 |   start_server(config)
   |                ------ value moved here
...
45 |   log_config(config)
   |              ^^^^^^ value used after move
   |
   help: consider cloning the value if you need to use it again
   |
40 |   start_server(config.clone())
   |                      ++++++++
   |
   docs: https://aria-lang.org/errors/E2001
```

### A.4 Non-Exhaustive Pattern Match

```
error[E4001]: non-exhaustive patterns
 --> src/parser.aria:78:3
   |
78 |   match token {
   |         ^^^^^ patterns not exhaustive
79 |     Token::Number(n) => Expr::Lit(n),
80 |     Token::Plus => Expr::Op(Op::Add),
81 |   }
   |
   note: the following patterns are not covered:
         - `Token::Minus`
         - `Token::Star`
         - `Token::Slash`
   |
   help: add the missing patterns or use a wildcard
   |
81 |     _ => panic!("unexpected token"),
   |     ++++++++++++++++++++++++++++++++
   |
   docs: https://aria-lang.org/errors/E4001
```

### A.5 Cascade Example

```
error[E1001]: cannot find `usre` in this scope
 --> src/main.aria:3:5
  |
3 |   let usre = get_user()
  |       ^^^^ not found in this scope
  |
  help: did you mean `user`?
  |
3 |   let user = get_user()
  |       ~~~~

note: this error likely caused 3 additional errors below

  error[E1001]: undefined `user` at src/main.aria:5:10
  error[E1001]: undefined `user` at src/main.aria:8:15
  error[E0001]: type mismatch at src/main.aria:12:5

Showing 4 errors (1 root cause + 3 cascading).
Fix the first error and recompile.
```

---

## Appendix B: Configuration Reference

### B.1 Command-Line Flags

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--color` | `always`, `auto`, `never` | `auto` | Color output mode |
| `--color-mode` | `standard`, `cvd`, `high-contrast` | `standard` | Color theme |
| `--error-limit` | `0-1000` | `10` | Max errors to show (0 = unlimited) |
| `--error-format` | `human`, `short`, `json`, `grouped` | `human` | Output format |
| `--strict` | flag | off | Stop at first error |
| `--strict-warnings` | flag | off | Treat warnings as errors |

### B.2 Environment Variables

| Variable | Values | Description |
|----------|--------|-------------|
| `NO_COLOR` | any | Disable color output |
| `ARIA_COLOR_MODE` | `standard`, `cvd`, `high-contrast` | Color theme |
| `ARIA_ERROR_LIMIT` | `0-1000` | Default error limit |
| `TERM` | terminal type | `dumb` disables color |

### B.3 Configuration File (aria.toml)

```toml
[diagnostics]
error_limit = 10
color = "auto"
color_mode = "standard"
show_docs_links = true
cascade_grouping = true
```

---

## References

1. SCRIBE Research: ARIA-M18-05 Error Message User Experience Research
2. Elm - Compiler Errors for Humans
3. Rust Compiler Development Guide - Diagnostics
4. Shape of Errors to Come - Rust Blog
5. annotate-snippets Crate
6. Writing Good Compiler Error Messages (Caleb Meredith)
7. Language Server Protocol Specification
8. NO_COLOR Standard
9. WCAG 2.1 Accessibility Guidelines
10. Bloomberg Terminal Color Accessibility
