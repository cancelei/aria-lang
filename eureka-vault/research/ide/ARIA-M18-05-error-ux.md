# ARIA-M18-05: Error Message User Experience Research

**Task ID**: ARIA-M18-05
**Status**: Completed
**Date**: 2026-01-15
**Agent**: SCRIBE (Eureka Iteration 3)
**Focus**: Deep UX Research for Aria Compiler Error Messages
**PRD Reference**: REQ-DX-001 - Elm/Rust-quality error messages

---

## Executive Summary

This research document explores the user experience dimension of compiler error messages, complementing the architectural design in ARIA-M18-02. While that document focuses on data structures and implementation, this document addresses the human factors: cognitive load, visual design, accessibility, error cascades, and the psychology of helpful messaging.

Key findings:
- **Cognitive load matters**: Research shows students often don't read error messages due to overwhelming complexity
- **Color accessibility is critical**: 8% of males have some form of color blindness; never rely on color alone
- **Error cascades frustrate users**: Show one error at a time or group related errors intelligently
- **Suggestions must be actionable**: "Did you mean X?" only helps when confident; otherwise, educate

---

## 1. Error Message Design Principles

### 1.1 The Elm Philosophy: Compiler as Assistant

Elm pioneered the concept of the compiler as a helpful assistant rather than a gatekeeper. Key principles from [Elm's "Compiler Errors for Humans"](https://elm-lang.org/news/compiler-errors-for-humans):

| Principle | Implementation |
|-----------|----------------|
| **Show exact code** | Display the user's code exactly as written, not pretty-printed |
| **Provide specific context** | Identify which field, function, or argument caused the problem |
| **Use strategic color** | Red for problems, blue for separators, minimal palette |
| **Minimize translation work** | Explain in domain terms, not compiler internals |
| **Educate, don't lecture** | Make errors feel like helpful dialogue |

### 1.2 Research-Backed Guidelines

From academic research on [compiler error messages](https://onlinelibrary.wiley.com/doi/10.1155/2010/602570):

1. **Increase readability** through clear structure and natural language
2. **Reduce cognitive load** by limiting information density
3. **Provide context** showing where the error fits in the program
4. **Use positive tone** - research shows this actually matters
5. **Show examples** of similar correct code
6. **Offer hints and solutions** rather than just describing the problem
7. **Allow dynamic interaction** through documentation links
8. **Use logical argumentation** explaining *why* something is wrong

### 1.3 The 80/20 Rule for Error Messages

From [Writing Good Compiler Error Messages](https://calebmer.com/2019/07/01/writing-good-compiler-error-messages.html):

- **80% case**: Developer immediately knows the fix from context or IDE squiggly
- **20% case**: Developer needs deeper explanation for non-intuitive rules

**Design implication**: Keep messages brief for the 80% case. Long messages are a *disservice* to productivity. Provide extended explanations via documentation links for the 20% case.

---

## 2. Suggestion and Fix Generation Patterns

### 2.1 When to Suggest Fixes

| Confidence Level | Action | Example |
|------------------|--------|---------|
| **High (>90%)** | Direct suggestion | Typo: `mesage` -> `message` |
| **Medium (60-90%)** | "Did you mean...?" | Multiple similar names in scope |
| **Low (<60%)** | General guidance | "Check that X is defined before use" |
| **Unknown** | Education link | "See docs on type inference" |

### 2.2 The "Did You Mean" Algorithm

Common algorithms for suggesting corrections, per [research on suggestion systems](https://itenterprise.co.uk/how-does-googles-did-you-mean-algorithm-work/):

#### Levenshtein Distance (Edit Distance)
```
levenshtein("mesage", "message") = 1  // One insertion
levenshtein("recieve", "receive") = 2  // Two swaps
```

**Implementation guidelines**:
- Threshold: suggest if distance <= 2 for short names, <= 3 for longer
- Weight: insertions/deletions cheaper than substitutions
- Context: prefer suggestions in same scope

#### Damerau-Levenshtein (With Transpositions)
Adds transposition as a basic operation (cost 1):
- `teh` -> `the` = 1 (transposition)
- More accurate for common typos

#### Keyboard Distance Weighting
Common typos involve adjacent keys:
- `e` <-> `r`, `i` <-> `o`, `n` <-> `m`
- Weight substitutions by physical keyboard distance

### 2.3 Type-Aware Suggestions

```
// When expected: String, found: Int
Suggestions:
1. ".to_string()" - MachineApplicable
2. "format!(\"{}\", x)" - MaybeIncorrect

// When expected: Option<T>, found: T
Suggestions:
1. "Some(value)" - MachineApplicable

// When expected: Result<T,E>, found: T
Suggestions:
1. "Ok(value)" - MachineApplicable
```

### 2.4 Suggestion Applicability Framework

Following [Rust's approach](https://rustc-dev-guide.rust-lang.org/diagnostics.html):

| Level | When to Use | Tool Behavior |
|-------|-------------|---------------|
| **MachineApplicable** | 100% correct in all contexts | Auto-apply with `aria fix` |
| **HasPlaceholders** | Correct pattern, needs user input | Show with `<placeholder>` |
| **MaybeIncorrect** | Plausible but uncertain | Show as "consider" |
| **Unspecified** | Unknown applicability | Don't auto-apply |

**Critical rule**: Never suggest something that could make the code worse or introduce new errors.

---

## 3. Context and Span Display

### 3.1 Primary vs Secondary Spans

From [Rust's diagnostic design](https://blog.rust-lang.org/2016/08/10/Shape-of-errors-to-come/):

**Primary span** (the `^^^` underline):
- Answers "what" went wrong
- Must be understandable in isolation (for IDE tooltips)
- Points to the immediate source of the error

**Secondary span** (the `---` underline):
- Answers "why" it went wrong
- Shows context that led to the error
- Uses blue text to differentiate

### 3.2 Multi-Span Storytelling

Good error messages tell a story:

```
error[E1001]: use of moved value
  --> src/main.aria:15:10
   |
12 |   let data = vec![1, 2, 3]
   |       ---- move occurs because `data` has type `Vec<Int>`
13 |
14 |   consume(data)
   |           ---- value moved here
15 |   print(data)
   |         ^^^^ value used after move
   |
   help: consider cloning the value if you need to use it again
   |
14 |   consume(data.clone())
   |               ++++++++
```

The spans create a narrative: definition -> move -> invalid use.

### 3.3 Context Window Guidelines

| Scenario | Context Lines |
|----------|---------------|
| Single-line error | Show 2 lines above, 1 below |
| Multi-line error | Show span + 1 line padding |
| Related code | Show just the relevant line |
| Long functions | Fold middle, show start/end |

### 3.4 Span Folding for Long Errors

Following [annotate-snippets](https://docs.rs/annotate-snippets/) conventions:
- Always show first and last line of folded sections
- Use `...` or `|` to indicate folding
- Detect if folding is worth it (minimum 3 lines to fold)

```
   |
 5 |   fn complex_function() {
...  |
45 |       problematic_call()
   |       ^^^^^^^^^^^^^^^^^^ error here
...  |
89 |   }
   |
```

---

## 4. Color and Formatting Guidelines

### 4.1 Terminal Color Accessibility

**The problem**: 8% of males have red-green color blindness ([deuteranomaly](https://davidmathlogic.com/colorblind/)). The traditional red=error, green=success, yellow=warning scheme fails for many users.

**Solutions from [Bloomberg's terminal accessibility work](https://www.bloomberg.com/ux/2021/10/14/designing-the-terminal-for-color-accessibility/)**:
- Use blue/red instead of green/red for positive/negative
- Maintain high contrast ratios (WCAG 4.5:1 minimum)
- Provide CVD (Color Vision Deficiency) alternative schemes

### 4.2 Never Rely on Color Alone

Always pair color with another indicator:

| Element | Color | Secondary Indicator |
|---------|-------|---------------------|
| Error | Red | `error:` prefix, `^^^` underline |
| Warning | Yellow | `warning:` prefix, `~~~` underline |
| Note | Blue | `note:` prefix, `---` underline |
| Help | Cyan | `help:` prefix, `+++` for additions |
| Success | Green | Checkmark icon, "ok" text |

### 4.3 ANSI Color Best Practices

From [Julia Evans' analysis of terminal colors](https://jvns.ca/blog/2024/10/01/terminal-colours/):

**Challenges**:
- No universal standard for ANSI color hex values
- Blue-on-black often unreadable
- Bright yellow-on-white nearly illegible
- Terminal themes vary wildly

**Recommendations for Aria**:

```rust
pub enum DiagnosticColor {
    // Use semantic names, not color names
    Error,      // Maps to red/bright-red
    Warning,    // Maps to yellow/bright-yellow
    Info,       // Maps to blue/cyan
    Hint,       // Maps to cyan/bright-cyan
    Success,    // Maps to green
    Emphasis,   // Maps to bold/bright-white
}

impl DiagnosticColor {
    /// Get ANSI code with fallback for accessibility
    fn ansi_code(&self, theme: &Theme) -> String {
        match (self, theme.high_contrast) {
            (Error, true) => "\x1b[1;31m",     // Bold red
            (Error, false) => "\x1b[91m",      // Bright red
            (Warning, _) => "\x1b[33m",        // Yellow
            (Info, _) => "\x1b[34m",           // Blue
            (Hint, _) => "\x1b[36m",           // Cyan
            (Success, _) => "\x1b[32m",        // Green
            (Emphasis, _) => "\x1b[1m",        // Bold
        }
    }
}
```

### 4.4 Graceful Degradation

Support multiple output modes:

| Mode | When | Output |
|------|------|--------|
| **Full color** | Modern terminals, color enabled | ANSI 256/24-bit |
| **Basic color** | Older terminals | ANSI 16 colors |
| **No color** | `--no-color`, piped output, `NO_COLOR` env | Plain text |
| **Machine** | `--format=json` | Structured JSON |

### 4.5 Respecting User Preferences

```rust
fn should_use_color() -> bool {
    // Check NO_COLOR environment variable (standard)
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check TERM
    if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
        return false;
    }

    // Check if output is a TTY
    if !std::io::stdout().is_terminal() {
        return false;
    }

    // Check our own flag
    // --color=always|auto|never
    true
}
```

---

## 5. Handling Error Cascades

### 5.1 The Cascade Problem

A single typo can trigger dozens of follow-on errors:

```
// One typo...
let usre = get_user()  // should be 'user'

// ...triggers cascade:
error: cannot find value `user` in this scope (line 5)
error: cannot find value `user` in this scope (line 8)
error: cannot find value `user` in this scope (line 12)
error: mismatched types: expected User, found () (line 15)
error: cannot call method `name` on () (line 16)
// ... 16 more errors
```

### 5.2 Recovery Strategies

From [error handling research](https://www.geeksforgeeks.org/error-recovery-strategies-in-compiler-design/):

#### Panic Mode Recovery
- Skip tokens until a synchronizing token (`;`, `}`, `end`)
- Prevents infinite loops
- May skip legitimate code

#### Phrase-Level Recovery
- Insert/delete minimal tokens to continue
- Replace `,` with `;`, insert missing `}`
- More precise but requires language knowledge

#### Error Productions
- Add grammar rules for common errors
- `if x = 5` -> "Did you mean `==`?"
- Proactive error detection

#### Poisoning / Sentinel Values

The Rust approach - introduce a "poison" type:

```rust
// When we encounter an error, use ErrorType as placeholder
enum Type {
    Int,
    String,
    // ...
    Error,  // Poison - matches anything, produces no further errors
}

impl Type {
    fn unify(&self, other: &Type) -> Result<Type, TypeError> {
        match (self, other) {
            // Error absorbs other errors
            (Type::Error, _) | (_, Type::Error) => Ok(Type::Error),
            // Normal unification
            (a, b) if a == b => Ok(a.clone()),
            _ => Err(TypeError::Mismatch { expected: self, found: other })
        }
    }
}
```

### 5.3 Error Limiting Strategies

| Strategy | Description | When to Use |
|----------|-------------|-------------|
| **First N errors** | Stop after N errors | Simple, fast |
| **First N per file** | Limit per-file | Large projects |
| **Category limits** | Max N type errors, M parse errors | Varied error types |
| **Root cause grouping** | Show root, hide derivatives | Sophisticated |
| **Error scoring** | Rank by likely usefulness | Advanced |

### 5.4 Elm's Approach: One Error at a Time

Elm famously shows only one error at a time:

**Pros**:
- Zero cognitive overload
- Clear focus on current problem
- Natural fix-compile-repeat loop

**Cons**:
- Many compile cycles for multiple errors
- Can't see the "big picture"
- Frustrating for experienced users

### 5.5 Rust's Approach: Grouped with Limits

Rust shows multiple errors but with intelligent grouping:

```
error[E0433]: failed to resolve: use of undeclared crate or module `foo`
error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0433`.
```

With `--error-format=short` for quick scanning:
```
src/main.rs:5:5: error[E0433]: failed to resolve
```

### 5.6 Recommended Aria Approach

**Hybrid strategy**:

1. **Default mode**: Show up to 10 errors, grouped by file
2. **Strict mode** (`--strict`): Stop at first error (Elm-style)
3. **Verbose mode** (`--error-limit=0`): Show all errors
4. **Smart grouping**: Identify cascades and show root cause prominently

```
error[E0002]: undefined variable `usre`
 --> src/main.aria:3:5
  |
3 | let usre = get_user()
  |     ^^^^ not found in this scope
  |
  help: did you mean `user`?
  |
3 | let user = get_user()
  |     ~~~~

note: this error may have caused 5 additional errors below
      fix this error first and recompile

error[E0002]: undefined variable `user`
 --> src/main.aria:5:10
  | ...

Showing 6 of 6 errors. Fix the first error and recompile.
```

---

## 6. Error Message Psychology

### 6.1 Why Tone Matters

Research shows that [error message tone affects learning outcomes](https://dl.acm.org/doi/fullHtml/10.1145/3411764.3445696):

| Tone | Example | Effect |
|------|---------|--------|
| **Accusatory** | "You made an error" | Defensive, frustrated |
| **Neutral** | "An error occurred" | Detached, unhelpful |
| **Collaborative** | "We found an issue" | Engaged, receptive |
| **Educational** | "This happens when..." | Learning opportunity |

### 6.2 First-Person Voice Comparison

| Language | Voice | Example |
|----------|-------|---------|
| **Elm** | First person singular | "I see a type mismatch" |
| **Rust** | Third person | "expected `String`, found `Int`" |
| **Recommended Aria** | First person plural | "We expected `String` here" |

The plural "we" positions the compiler as part of the development team, reviewing code together.

### 6.3 Avoiding Jargon

| Jargon | Plain English |
|--------|---------------|
| "identifier" | "name" |
| "token" | "symbol" or specific: "semicolon" |
| "expression" | "value" or "code" |
| "unification failed" | "types don't match" |
| "cannot coerce" | "cannot convert" |
| "arity mismatch" | "wrong number of arguments" |

### 6.4 The Hemingway Test

From [writing good compiler error messages](https://calebmer.com/2019/07/01/writing-good-compiler-error-messages.html):

Run error messages through the [Hemingway Editor](https://hemingwayapp.com/) to check:
- Reading level (aim for Grade 6-8)
- Sentence complexity
- Passive voice usage
- Adverb density

---

## 7. Structured Output for Tooling

### 7.1 JSON Format for IDE Integration

Following [LSP Diagnostic](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/) conventions:

```json
{
  "$schema": "https://aria-lang.org/schemas/diagnostic.json",
  "diagnostics": [
    {
      "code": "E0001",
      "severity": "error",
      "message": "We expected `String`, but found `Int`.",
      "source": "aria",
      "range": {
        "start": { "line": 41, "character": 20 },
        "end": { "line": 41, "character": 28 }
      },
      "relatedInformation": [
        {
          "location": {
            "uri": "file:///src/main.aria",
            "range": {
              "start": { "line": 41, "character": 4 },
              "end": { "line": 41, "character": 8 }
            }
          },
          "message": "expected due to this type annotation"
        }
      ],
      "codeDescription": {
        "href": "https://aria-lang.org/errors/E0001"
      },
      "data": {
        "suggestions": [
          {
            "message": "Convert to string",
            "edit": {
              "range": {
                "start": { "line": 41, "character": 28 },
                "end": { "line": 41, "character": 28 }
              },
              "newText": ".to_string()"
            },
            "applicability": "machine-applicable"
          }
        ]
      }
    }
  ]
}
```

### 7.2 LSP Code Actions from Suggestions

Suggestions with `MachineApplicable` confidence become LSP Code Actions:

```json
{
  "title": "Convert to string",
  "kind": "quickfix",
  "diagnostics": [{ "code": "E0001" }],
  "isPreferred": true,
  "edit": {
    "documentChanges": [
      {
        "textDocument": { "uri": "file:///src/main.aria" },
        "edits": [
          {
            "range": { "start": { "line": 41, "character": 28 }, "end": { "line": 41, "character": 28 } },
            "newText": ".to_string()"
          }
        ]
      }
    ]
  }
}
```

---

## 8. Error Documentation System

### 8.1 Error Index Structure

Each error code should have comprehensive documentation:

```markdown
# E0001: Type Mismatch

## Summary
This error occurs when an expression has a different type than expected.

## Example

```aria
let name: String = 42  // Error: expected String, found Int
```

## Explanation
Aria is a statically-typed language, which means every value has a type
known at compile time. When you declare `let name: String`, you're telling
the compiler that `name` will hold text. The value `42` is a number (Int),
not text (String).

## Common Causes
1. Forgetting to convert between types
2. Function returning the wrong type
3. Incorrect type annotation

## How to Fix

### Convert the value
```aria
let name: String = 42.to_string()  // Converts Int to String
```

### Change the annotation
```aria
let count: Int = 42  // If you actually wanted a number
```

### Use the correct function
```aria
let name: String = get_name()  // Returns String, not get_id() which returns Int
```

## Related Errors
- [E0003: Undefined Type](./E0003.md)
- [E0004: Cannot Infer Type](./E0004.md)

## See Also
- [Type System Guide](/docs/types)
- [Type Conversion](/docs/converting-types)
```

### 8.2 Error Catalog Approach

Following [Elm's error-message-catalog](https://github.com/elm/error-message-catalog):

Create a test suite of programs that trigger each error:
- Systematic coverage of all error paths
- Regression testing for error message quality
- Evidence-based improvement process

---

## 9. Implementation Recommendations for Aria

### 9.1 Phase 1: Foundation

1. Implement `DiagnosticRenderer` with plain text output
2. Add span-based error location tracking
3. Create initial error code registry (E0001-E0100)
4. Implement basic "did you mean" using Levenshtein distance

### 9.2 Phase 2: Enhanced UX

1. Add ANSI color support with `NO_COLOR` respect
2. Implement error cascade detection and grouping
3. Add secondary spans for multi-location errors
4. Create suggestion applicability framework

### 9.3 Phase 3: Tooling Integration

1. JSON output format for IDEs
2. LSP diagnostic conversion
3. Code action generation from suggestions
4. Error documentation website generator

### 9.4 Phase 4: Polish

1. Accessibility audit (colorblind simulation testing)
2. User research on error message effectiveness
3. Error message localization framework
4. A/B testing infrastructure for message variants

---

## 10. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Error readability | Grade 8 reading level | Hemingway Editor |
| Fix suggestion accuracy | >80% for typos | User study |
| Time to first fix | <30 seconds | User study |
| Error cascade ratio | <2 shown per root cause | Automated analysis |
| Accessibility | WCAG AA compliance | Accessibility audit |
| IDE integration | <100ms diagnostic refresh | Performance test |

---

## 11. Key References

### Primary Sources
1. [Elm - Compiler Errors for Humans](https://elm-lang.org/news/compiler-errors-for-humans)
2. [Rust Compiler Development Guide - Diagnostics](https://rustc-dev-guide.rust-lang.org/diagnostics.html)
3. [Shape of Errors to Come - Rust Blog](https://blog.rust-lang.org/2016/08/10/Shape-of-errors-to-come/)
4. [annotate-snippets Crate](https://docs.rs/annotate-snippets/)
5. [Writing Good Compiler Error Messages](https://calebmer.com/2019/07/01/writing-good-compiler-error-messages.html)

### Academic Research
6. [Compiler Error Messages Considered Unhelpful](https://dl.acm.org/doi/10.1145/3344429.3372508)
7. [On Compiler Error Messages: What They Say and What They Mean](https://onlinelibrary.wiley.com/doi/10.1155/2010/602570)
8. [On Designing Programming Error Messages for Novices](https://dl.acm.org/doi/fullHtml/10.1145/3411764.3445696)

### Accessibility Resources
9. [Coloring for Colorblindness](https://davidmathlogic.com/colorblind/)
10. [Terminal Colors are Tricky](https://jvns.ca/blog/2024/10/01/terminal-colours/)
11. [Bloomberg Terminal Color Accessibility](https://www.bloomberg.com/ux/2021/10/14/designing-the-terminal-for-color-accessibility/)

### Tools and Standards
12. [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
13. [Elm Error Message Catalog](https://github.com/elm/error-message-catalog)
14. [NO_COLOR Standard](https://no-color.org/)

---

## 12. Open Questions

1. **Localization**: Should Aria support error messages in multiple languages from day one?
2. **Learning mode**: Should there be a "verbose" mode for beginners with more explanation?
3. **Error telemetry**: Should we collect anonymous error frequency data to prioritize improvements?
4. **Custom error messages**: Should library authors be able to define custom diagnostic messages?
5. **AI assistance**: Should Aria integrate LLM-powered error explanation as an optional feature?

---

## Appendix A: Color Palette Recommendations

### Standard Theme (Dark Background)
| Element | Hex | ANSI | Contrast |
|---------|-----|------|----------|
| Error | #FF6B6B | 9 (bright red) | 4.6:1 |
| Warning | #FFE66D | 11 (bright yellow) | 8.2:1 |
| Info | #4ECDC4 | 14 (bright cyan) | 5.1:1 |
| Hint | #95E1D3 | 6 (cyan) | 5.8:1 |
| Code | #F8F8F2 | 15 (bright white) | 15.8:1 |

### High Contrast Theme
| Element | Hex | ANSI | Contrast |
|---------|-----|------|----------|
| Error | #FF0000 | 1 (red) | 5.3:1 |
| Warning | #FFFF00 | 3 (yellow) | 19.6:1 |
| Info | #00FFFF | 6 (cyan) | 16.7:1 |
| Hint | #FFFFFF | 15 (white) | 21:1 |
| Code | #FFFFFF | 15 (white) | 21:1 |

### Colorblind-Safe Theme (Deuteranopia)
| Element | Hex | Description |
|---------|-----|-------------|
| Error | #D55E00 | Orange (instead of red) |
| Warning | #F0E442 | Yellow |
| Info | #0072B2 | Blue |
| Success | #0072B2 | Blue (instead of green) |

---

## Appendix B: Error Message Templates

### Type Mismatch Template
```
error[E0001]: type mismatch
 --> {file}:{line}:{column}
  |
{line} | {code}
  |     {underline} expected `{expected}`, found `{found}`
  |
  {secondary_spans}
  help: {suggestion}
  docs: https://aria-lang.org/errors/E0001
```

### Undefined Variable Template
```
error[E0002]: cannot find `{name}` in this scope
 --> {file}:{line}:{column}
  |
{line} | {code}
  |     {underline} not found in this scope
  |
  {if similar_names}
  help: a {kind} with a similar name exists: `{similar}`
  {endif}
  docs: https://aria-lang.org/errors/E0002
```

### Pattern Match Template
```
error[E4001]: non-exhaustive patterns
 --> {file}:{line}:{column}
  |
{line} | match {expr}
  |       {patterns}
  |
  note: the following patterns are not covered:
        {missing_patterns}
  |
  help: add remaining patterns or use `_` as a catch-all:
  |
{line} |     {suggestion}
  |     {highlight}
  |
  docs: https://aria-lang.org/errors/E4001
```
