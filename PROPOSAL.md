# aria-lang: Core Syntax and Grammar Proposal

## Vision
**aria-lang** is designed for a future where humans and AI collaborate in a shared execution environment. It prioritizes **readability**, **predictability**, and **resilience**. 

AI models often struggle with:
1. Deeply nested, implicit structures (e.g., Python indentation in long files).
2. Complex punctuation rules (e.g., C++ semicolons vs. Rust expression returns).
3. Ambiguous symbol meanings.

**aria** addresses these by using explicit sigils, keyword-anchored statements, and a minimalist LL(1) grammar.

---

## Core Principles
1. **Predictive (LL(1))**: The parser only needs one token of lookahead. This makes the language deterministic and easy to implement.
2. **AI-Resilient**:
   - **Sigils**: `$variable` for data, `@agent` for execution context. This allows the AI (and the parser) to immediately identify the role of a token.
   - **Keyword-First**: Every statement starts with a keyword (`let`), a sigil (`@`), or a command name.
   - **Punctuation Forgiveness**: Commas and semicolons are treated as optional whitespace, reducing "syntax error" hallucinations.
3. **Minimalist**: A tiny footprint of reserved words.

---

## Grammar (EBNF)

```ebnf
Program      ::= Statement*
Statement    ::= (Binding | AgentBlock | Command) Term
Term         ::= "\n" | ";"

Binding      ::= "let" Variable "=" Expression
AgentBlock   ::= "@" Identifier "{" Program "}"
Command      ::= Identifier (Expression)*

Expression   ::= Value | List | Map | Command
Value        ::= String | Number | Variable
Variable     ::= "$" Identifier
List         ::= "[" (Expression (","? Expression)*)? "]"
Map          ::= "{" (Identifier ":" Expression (","? Identifier ":" Expression)*)? "}"

Identifier   ::= [a-zA-Z_][a-zA-Z0-9_]*
String       ::= "\"" [^"]* "\""
Number       ::= [0-9]+ ("." [0-9]+)?
```

---

## Syntax Features

### 1. Variables and Scoping
Variables are prefixed with `$`. This prevents collisions with commands and makes data flow explicit.
```aria
let $greeting = "Hello"
say $greeting
```

### 2. The Agent Block
Execution context is defined by the `@` sigil. This is the primary way to route commands to specific AI agents or system modules.
```aria
@researcher {
    search "LL(1) grammar"
    let $result = last_output
}
```

### 3. Flexible Lists and Maps
To minimize AI errors, commas in lists and maps are optional.
```aria
let $colors = ["red" "green" "blue"]
let $config = {
    mode: "fast"
    retries: 3
}
```

---

## Sample: Hello World
`hello.aria`
```aria
# Traditional hello world
say "Hello World"

# Using a variable
let $target = "World"
say "Hello" $target
```

## Sample: Agent Call
`agent-call.aria`
```aria
# Calling a specialized researcher agent
@researcher {
    find "Top 3 LL(1) benefits"
    let $top_facts = last_output
    
    # Passing data to a writer agent
    @writer {
        summarize $top_facts
        style "technical"
    }
}
```
