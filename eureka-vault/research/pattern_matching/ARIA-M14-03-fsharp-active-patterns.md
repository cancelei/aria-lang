# ARIA-M14-03: F# Active Patterns

**Task ID**: ARIA-M14-03
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study F# active patterns for extensible pattern matching

---

## Executive Summary

F# active patterns enable custom pattern matching logic through user-defined pattern recognizers. This research analyzes active pattern design for extending Aria's pattern matching with user-defined extractors and views.

---

## 1. Overview

### 1.1 What are Active Patterns?

Active patterns allow defining custom pattern matching behavior:

```fsharp
// Standard pattern: only works with type structure
match value with
| Some x -> ...
| None -> ...

// Active pattern: custom extraction logic
match emailString with
| Email(user, domain) -> ...  // Custom parser
| _ -> ...
```

### 1.2 Why Active Patterns?

| Benefit | Description |
|---------|-------------|
| Abstraction | Hide implementation details |
| Extensibility | Add patterns to existing types |
| Reusability | Define once, use everywhere |
| Readability | Express intent clearly |

---

## 2. F# Active Pattern Syntax

### 2.1 The "Banana Clips" Syntax

```fsharp
// Define with (| |) syntax - called "banana clips"
let (|Even|Odd|) n =
    if n % 2 = 0 then Even else Odd

// Use in pattern match
match number with
| Even -> "even"
| Odd -> "odd"
```

### 2.2 Pattern Categories

| Category | Syntax | Description |
|----------|--------|-------------|
| Complete | `(|A|B|C|)` | Must cover all cases |
| Partial | `(|A|_|)` | May not match |
| Parameterized | `(|A|_|) param` | Takes parameters |
| Single-case | `(|A|)` | Always matches, extracts |

---

## 3. Complete Active Patterns

### 3.1 Definition

```fsharp
// Must return one of the cases (no Option)
let (|Positive|Negative|Zero|) n =
    if n > 0 then Positive
    elif n < 0 then Negative
    else Zero

// Exhaustive matching
let describe n =
    match n with
    | Positive -> "positive"
    | Negative -> "negative"
    | Zero -> "zero"
```

### 3.2 With Extracted Values

```fsharp
let (|RGB|) (color: Color) =
    RGB(color.R, color.G, color.B)

match myColor with
| RGB(r, g, b) -> sprintf "R:%d G:%d B:%d" r g b
```

---

## 4. Partial Active Patterns

### 4.1 Definition

```fsharp
// Returns Option - may not match
let (|Int|_|) str =
    match System.Int32.TryParse(str) with
    | true, n -> Some n
    | false, _ -> None

// Must handle non-match case
match input with
| Int n -> sprintf "Got integer: %d" n
| _ -> "Not an integer"
```

### 4.2 Chaining Partial Patterns

```fsharp
let (|Email|_|) str =
    let parts = str.Split('@')
    if parts.Length = 2 then Some(parts.[0], parts.[1])
    else None

let (|Phone|_|) str =
    if str.Length = 10 && str |> Seq.forall Char.IsDigit then
        Some str
    else None

// Chain patterns
match contact with
| Email(user, domain) -> sprintf "Email: %s@%s" user domain
| Phone number -> sprintf "Phone: %s" number
| _ -> "Unknown contact format"
```

---

## 5. Parameterized Active Patterns

### 5.1 Definition

```fsharp
// Pattern takes parameters
let (|DivisibleBy|_|) divisor n =
    if n % divisor = 0 then Some() else None

match number with
| DivisibleBy 3 & DivisibleBy 5 -> "FizzBuzz"
| DivisibleBy 3 -> "Fizz"
| DivisibleBy 5 -> "Buzz"
| n -> string n
```

### 5.2 Regex Example

```fsharp
let (|Regex|_|) pattern input =
    let m = System.Text.RegularExpressions.Regex.Match(input, pattern)
    if m.Success then
        Some(List.tail [for g in m.Groups -> g.Value])
    else None

match url with
| Regex @"https?://([^/]+)(.*)" [host; path] ->
    sprintf "Host: %s, Path: %s" host path
| _ -> "Invalid URL"
```

---

## 6. Single-Case Active Patterns

### 6.1 Definition

```fsharp
// Always succeeds, transforms value
let (|Uppercase|) (s: string) = s.ToUpper()

match name with
| Uppercase upper -> printfn "HELLO %s" upper
```

### 6.2 Use Cases

```fsharp
// Normalization
let (|Trimmed|) (s: string) = s.Trim()

// Decomposition
let (|FileInfo|) path =
    let fi = System.IO.FileInfo(path)
    (fi.Name, fi.Extension, fi.Length)

match filePath with
| FileInfo(name, ext, size) ->
    printfn "%s%s is %d bytes" name ext size
```

---

## 7. Combining Patterns

### 7.1 AND Patterns

```fsharp
// Both patterns must match
match value with
| Positive & DivisibleBy 2 -> "positive even"
| _ -> "other"
```

### 7.2 OR Patterns

```fsharp
// Either pattern matches
match number with
| 0 | 1 -> "binary digit"
| _ -> "other"
```

### 7.3 Nested Active Patterns

```fsharp
let (|ParseDate|_|) str = ...
let (|Weekend|Weekday|) (date: DateTime) = ...

match input with
| ParseDate date ->
    match date with
    | Weekend -> "It's the weekend!"
    | Weekday -> "Back to work"
| _ -> "Invalid date"
```

---

## 8. Comparison with Other Languages

### 8.1 Scala Extractors

```scala
// Similar concept, different syntax
object Email {
  def unapply(str: String): Option[(String, String)] = {
    val parts = str.split("@")
    if (parts.length == 2) Some((parts(0), parts(1)))
    else None
  }
}

email match {
  case Email(user, domain) => s"$user at $domain"
  case _ => "not an email"
}
```

### 8.2 Rust Pattern Guards + Bindings

```rust
// Rust doesn't have active patterns
// Must use guards or match guards
match value {
    n if is_even(n) => "even",
    n if is_odd(n) => "odd",
    _ => unreachable!()
}
```

### 8.3 Haskell View Patterns

```haskell
{-# LANGUAGE ViewPatterns #-}

-- View patterns: apply function then match
f (even -> True) = "even"
f (even -> False) = "odd"

-- Pattern synonyms (more similar to active patterns)
pattern Even <- (even -> True)
pattern Odd <- (even -> False)
```

---

## 9. Recommendations for Aria

### 9.1 Pattern Extractor Syntax

```aria
# Define pattern extractor
extractor Email(user: String, domain: String) from String
  match self.split("@")
    [user, domain] => Some((user, domain))
    _ => None
  end
end

# Use in pattern match
match contact
  Email(user, domain) => "#{user}@#{domain}"
  _ => "not an email"
end
```

### 9.2 Complete Extractors

```aria
# Complete extractor (must always produce a value)
extractor Sign(Positive | Negative | Zero) from Int
  if self > 0 then Positive
  elif self < 0 then Negative
  else Zero
  end
end

# Exhaustive matching
match number
  Sign.Positive => "+"
  Sign.Negative => "-"
  Sign.Zero => "0"
end
```

### 9.3 Parameterized Extractors

```aria
# Extractor with parameters
extractor DivisibleBy(n: Int) from Int
  if self % n == 0 then Some(()) else None end
end

match number
  DivisibleBy(3) & DivisibleBy(5) => "FizzBuzz"
  DivisibleBy(3) => "Fizz"
  DivisibleBy(5) => "Buzz"
  n => n.to_string
end
```

### 9.4 Single-Case Extractors

```aria
# Always succeeds, transforms
extractor Normalized(value: String) from String
  self.trim().lowercase()
end

match input
  Normalized(s) => process(s)
end
```

### 9.5 Composable Extractors

```aria
# Regex extractor
extractor Regex(pattern: String, groups: Array[String]) from String
  Regex.new(pattern).match(self).map |m|
    m.groups
  end
end

# URL parsing
match url
  Regex(r"https?://([^/]+)(.*)", [host, path]) =>
    Route(host, path)
  _ => InvalidUrl
end
```

### 9.6 View Patterns

```aria
# Lightweight syntax for simple views
match (value.sign)
  Positive => "+"
  Negative => "-"
  Zero => "0"
end

# Equivalent to:
match value
  x if x.sign == Positive => "+"
  x if x.sign == Negative => "-"
  x if x.sign == Zero => "0"
end
```

### 9.7 Effect-Aware Extractors

```aria
# Extractors can have effects
extractor ParseJson[T](value: T) from String -> {Parse} Option[T]
  JSON.parse[T](self)
end

# Handler provides parsing capability
with Parse.lenient
  match input
    ParseJson(data) => process(data)
    _ => use_default()
  end
end
```

---

## 10. Implementation Considerations

### 10.1 Compilation Strategy

```
Pattern with extractor:
  Email(user, domain)

Compiles to:
  1. Call extractor function: Email.extract(value)
  2. Match on result:
     - Some((user, domain)) → bind variables, continue
     - None → try next pattern
```

### 10.2 Exhaustiveness with Extractors

```aria
# Complete extractors participate in exhaustiveness checking
extractor Sign(Positive | Negative | Zero) from Int
  # ...
end

match number
  Sign.Positive => "+"
  Sign.Negative => "-"
end
# Error: Non-exhaustive, missing Sign.Zero
```

### 10.3 Performance Considerations

```aria
# Extractors should be:
# - Pure (no side effects for partial extractors)
# - Efficient (called during pattern matching)
# - Memoizable (compiler may cache results)

# Compiler optimizations:
# - Inline simple extractors
# - Share extraction results in nested patterns
# - Eliminate redundant extractions
```

---

## 11. Advanced Patterns

### 11.1 Recursive Extractors

```aria
# Parse nested structure
extractor Nested(inner: Expr) from String
  if self.starts_with("(") && self.ends_with(")")
    parse_expr(self[1..-1])
  else
    None
  end
end
```

### 11.2 Type-Based Dispatch

```aria
# Different extractors for different types
extractor AsNumber from Any
  match self
    i: Int => Some(i.to_float)
    f: Float => Some(f)
    s: String => s.parse_float.ok
    _ => None
  end
end
```

---

## 12. Key Resources

1. [F# Active Patterns](https://docs.microsoft.com/en-us/dotnet/fsharp/language-reference/active-patterns)
2. [Scala Extractors](https://docs.scala-lang.org/tour/extractor-objects.html)
3. [Haskell View Patterns](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/view_patterns.html)
4. [Pattern Synonyms in Haskell](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/pattern_synonyms.html)
5. [F# for Fun and Profit - Active Patterns](https://fsharpforfunandprofit.com/posts/convenience-active-patterns/)

---

## 13. Open Questions

1. Should extractors be methods on types or standalone functions?
2. How do we ensure extractors are pure for exhaustiveness checking?
3. What's the syntax for complete vs partial extractors?
4. How do extractors interact with Aria's effect system?
