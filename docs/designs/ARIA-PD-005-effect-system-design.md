# ARIA-PD-005: Effect System Design

**Decision ID**: ARIA-PD-005
**Status**: Approved
**Date**: 2026-01-15
**Author**: ORACLE (Product Decision Agent)
**Research Inputs**:
- ARIA-M03-01: Algebraic Effects Survey (KIRA)
- ARIA-M03-02: Koka Effect System Analysis (KIRA)
- ARIA-M03-03: Effect Inference in Practice (KIRA)
- ARIA-M03-04: Algebraic Effects Deep Dive (KIRA)
- ARIA-M03-05: Effect Compilation Strategies (PRISM)

---

## Executive Summary

This document defines Aria's effect system design, synthesizing research from KIRA (theoretical foundations) and PRISM (compilation strategies). The goal is to track computational effects (IO, async, mutation, exceptions) in the type system while maintaining Aria's core philosophy: "Write code like Ruby/Python, get safety like Rust."

**Final Decision**: Aria will implement a **Row-Polymorphic Effect System** with:
- **Syntax**: Koka-inspired effect annotations using `!` suffix
- **Semantics**: One-shot algebraic effects with tail-resumptive optimization
- **Compilation**: Evidence-passing for tail-resumptive effects, selective CPS for general effects
- **Inference**: Full effect inference with principal types; annotations optional but available

---

## 1. Effect System Architecture

### 1.1 Design Philosophy

```
ARIA EFFECT SYSTEM ARCHITECTURE

Goal: Track effects without "function coloring" problem
      while enabling automatic async optimization

Layer 1: Effect Types (Static Analysis)
  - Effects tracked in function signatures
  - Row polymorphism for flexible effect composition
  - Principal type inference (no annotation required)

Layer 2: Effect Handlers (Runtime Semantics)
  - Define custom effect interpretations
  - Scoped to handler blocks
  - First-class, composable

Layer 3: Compilation Strategy (Performance)
  - Evidence-passing for 90% of effects (zero overhead)
  - Selective CPS for resumption-requiring effects
  - Direct style preserved where possible
```

### 1.2 Effect Categories

| Category | Effects | Compile Strategy | Runtime Cost |
|----------|---------|------------------|--------------|
| **Tail-Resumptive** | IO, State, Reader, Console | Evidence-passing | Zero |
| **Multi-Shot** | Amb, Backtrack, Logic | Full CPS | Moderate |
| **One-Shot** | Async, Exception, Yield | Fiber/CPS hybrid | Low |
| **Pure** | None | Direct compilation | Zero |

### 1.3 Comparison with Research Recommendations

| Research Finding | ORACLE Decision | Rationale |
|------------------|-----------------|-----------|
| KIRA: Row-polymorphic effects (Koka-style) | **Adopted** | Best balance of expressiveness and inference |
| KIRA: One-shot fibers (OCaml 5-style) | **Adopted** | Efficient for async, matches Aria's concurrency model |
| PRISM: Evidence-passing for tail-resumptive | **Adopted** | Zero overhead for common cases |
| PRISM: Selective CPS for general effects | **Adopted** | Preserves direct style, CPS only when needed |

---

## 2. Effect Syntax Design

### 2.1 Effect Declaration Syntax

```aria
# Effect declaration using 'effect' keyword
effect Console
  fn print(message: String) -> Unit
  fn read_line() -> String
end

effect State[S]
  fn get() -> S
  fn put(value: S) -> Unit
  fn modify(f: Fn(S) -> S) -> Unit
end

effect Async
  fn await[T](promise: Promise[T]) -> T
  fn yield() -> Unit
end

effect Exception[E]
  fn raise(error: E) -> Never
end
```

### 2.2 Function Effect Annotation Syntax

Functions declare effects using the `!` suffix on return types:

```aria
# Explicit effect annotation
fn greet(name: String) -> Unit !Console
  Console.print("Hello, #{name}!")
end

# Multiple effects (order doesn't matter)
fn fetch_and_log(url: String) -> String !IO, Console, Exception[HttpError]
  response = http.get(url)?
  Console.print("Fetched: #{response.status}")
  response.body
end

# Effect polymorphism with row variables
fn map_with_effect[T, U, E](items: Array[T], f: Fn(T) -> U !E) -> Array[U] !E
  items.map(f)
end

# No annotation = inferred (preferred style)
fn process(data: String) -> Int
  # Compiler infers: !IO, Exception[ParseError]
  parsed = parse(data)?
  File.write("log.txt", parsed.to_string)
  parsed.value
end
```

### 2.3 Effect Handler Syntax

```aria
# Basic handler syntax
handle expr with
  effect_name.operation(args...) => body
  ...
  return(x) => final_value
end

# Example: Handling exceptions
fn safe_divide(a: Int, b: Int) -> Option[Int]
  handle
    if b == 0
      Exception.raise(DivisionByZero)
    end
    Some(a / b)
  with
    Exception.raise(_) => None
    return(x) => x
  end
end

# Example: Handling state
fn run_stateful[S, T](initial: S, computation: Fn() -> T !State[S]) -> (T, S)
  mut state = initial

  result = handle computation() with
    State.get() => resume(state)
    State.put(v) =>
      state = v
      resume(Unit)
    State.modify(f) =>
      state = f(state)
      resume(Unit)
    return(x) => x
  end

  (result, state)
end
```

### 2.4 Effect Type Syntax in Type Annotations

```aria
# Effect row types
type EffectfulFn[T, U, E] = Fn(T) -> U !E

# Open effect rows (polymorphic)
type Handler[E, R] = Fn() -> R !E  # E is a row variable

# Closed effect rows
type PureComputation[T] = Fn() -> T !{}  # Empty effect row = pure

# Effect constraints in where clauses
fn transform[T, E](
  items: Array[T],
  f: Fn(T) -> T !E
) -> Array[T] !E
where E: subset_of(IO, Console)  # Effect bounds
  items.map(f)
end
```

---

## 3. Type System Integration

### 3.1 Effect Row Types

Aria uses row polymorphism for effect types, following Koka's design:

```
Effect Row Grammar:

  EffectRow ::= '{' EffectList '}'
              | '{' EffectList '|' RowVar '}'
              | RowVar

  EffectList ::= Effect (',' Effect)*
               | empty

  Effect ::= EffectName
           | EffectName '[' TypeArgs ']'

  RowVar ::= lowercase_identifier
```

**Examples**:
```aria
# Closed row (exactly these effects)
!{IO, Console}

# Open row (these effects plus unknown others)
!{IO, Console | e}

# Row variable only (any effects)
!e

# Empty row (pure)
!{}
```

### 3.2 Subtyping Rules

Effect rows support width subtyping:

```
EFFECT SUBTYPING RULES

1. Empty is subtype of any row:
   {} <: e  for any effect row e

2. Subset inclusion:
   {E1, E2} <: {E1, E2, E3}

3. Row variable extension:
   {E | e} <: {E, F | e}  (adding effects is safe)

4. Handler elimination:
   If handler H handles effect E, then:
   handle (expr : T !{E | e}) with H : T !{e}
```

### 3.3 Integration with Existing Type System

The effect system integrates with ARIA-PD-001 (Type System) and ARIA-PD-002 (Ownership):

| Feature | Integration Point |
|---------|-------------------|
| **Bidirectional typing** | Effect annotations propagate bidirectionally |
| **Flow-sensitive narrowing** | Effect sets narrow after handler blocks |
| **Ownership** | Effects interact with borrow scopes |
| **Generics** | Effect rows as type parameters |

**Example of integrated type checking**:
```aria
fn process_file[E](path: String) -> Result[Data, Error] !{IO | E}
  # Ownership: path is borrowed for IO operation
  # Bidirectional: Result type propagates to match arms
  # Flow-sensitive: After Ok branch, we know no exception occurred

  content = File.read(path)?    # IO effect

  match parse(content)
    Ok(data) =>
      # data: Data (narrowed from Result)
      Ok(data)
    Err(e) =>
      # e: ParseError (narrowed)
      Err(Error.from(e))
  end
end
```

### 3.4 Effect and Ownership Interaction

Effects must respect Aria's ownership rules from ARIA-PD-002:

```aria
# Effects on borrowed values are limited
fn read_from[life L](buffer: ref[L] Buffer) -> String !IO
  # OK: IO effect on borrowed reference
  buffer.read_string()
end

# Mutable effects require mutable borrows
fn modify_state(state: mut ref State) -> Unit !State[Int]
  # State.put requires mutable access
  State.put(state.value + 1)
end

# Effect handlers can't outlive borrowed data
fn example()
  let data = create_data()

  handle
    process(borrow data)  # data borrowed here
  with
    SomeEffect.op() =>
      # resume captures borrow, must complete before data drops
      resume(...)
  end
end  # data drops here
```

---

## 4. Effect Inference Algorithm

### 4.1 Algorithm Overview

Aria uses **bidirectional effect inference** with principal types:

```
EFFECT INFERENCE ALGORITHM

Phase 1: Bottom-Up Effect Collection
  - Traverse AST, collecting effect operations
  - Build effect constraints for each subexpression
  - Generate fresh row variables for unknown effects

Phase 2: Top-Down Propagation
  - Propagate expected effect types from context
  - Unify row variables with concrete effects
  - Check effect bounds in where clauses

Phase 3: Handler Resolution
  - Match handlers to effect operations
  - Eliminate handled effects from row
  - Check handler return type matches

Phase 4: Generalization
  - Generalize remaining row variables
  - Create polymorphic effect signature
  - Validate no escaping effects
```

### 4.2 Inference Rules

```
EFFECT TYPE RULES

[E-VAR]
  G |- x : T !{}
  where G(x) = T

[E-APP]
  G |- f : (T1, ..., Tn) -> U !E1
  G |- e1 : T1 !E2  ...  G |- en : Tn !En+1
  ------------------------------------------------
  G |- f(e1, ..., en) : U !{E1 | E2 | ... | En+1}

[E-EFFECT-OP]
  effect F has op : (A1, ..., An) -> R
  G |- e1 : A1 !E1  ...  G |- en : An !En
  ----------------------------------------
  G |- F.op(e1, ..., en) : R !{F | E1 | ... | En}

[E-HANDLE]
  G |- e : T !{F | E}
  G |- H handles F with return type T -> U
  ----------------------------------------
  G |- handle e with H : U !E

[E-LAMBDA]
  G, x : T |- e : U !E
  -----------------------
  G |- |x| e : (T) -> U !E

[E-GENERALIZE]
  G |- e : T !{E | r}
  r not free in G
  ----------------------
  G |- e : forall r. T !{E | r}
```

### 4.3 Inference Examples

```aria
# Example 1: Simple inference
fn greet(name: String)
  Console.print("Hello, #{name}!")
end
# Inferred: fn greet(name: String) -> Unit !Console

# Example 2: Effect polymorphism inference
fn twice(f: Fn() -> Unit)
  f()
  f()
end
# Inferred: fn twice[E](f: Fn() -> Unit !E) -> Unit !E

# Example 3: Handler eliminates effect
fn safe_log(message: String) -> Unit
  handle
    Console.print(message)  # !Console effect
  with
    Console.print(m) =>
      # Swallow output in production
      resume(Unit)
  end
end
# Inferred: fn safe_log(message: String) -> Unit !{}  (pure!)

# Example 4: Partial handling
fn log_to_file(path: String, message: String) -> Unit !IO
  handle
    Console.print(message)  # !Console
  with
    Console.print(m) =>
      File.append(path, m)  # !IO remains
      resume(Unit)
  end
end
# Inferred: fn log_to_file(...) -> Unit !IO
```

### 4.4 Error Messages for Effect Mismatches

Following ARIA-PD-001's error message philosophy:

```
EFFECT MISMATCH at line 42

  40 |  fn pure_computation() -> Int !{}
  41 |    let x = calculate()
  42 |    Console.print("result: #{x}")
               ^^^^^^^^^^^^^^^^^^^^^^^

I found effect `Console` but expected no effects (`!{}`).

The function `pure_computation` is declared pure (`!{}`), but
the call to `Console.print` at line 42 requires the `Console` effect.

To fix this, either:
  1. Add the effect to the function signature:
     fn pure_computation() -> Int !Console

  2. Handle the effect locally:
     handle Console.print("result: #{x}") with
       Console.print(_) => resume(Unit)  # Suppress output
     end

Learn more: https://aria-lang.org/docs/effects/purity
```

---

## 5. Built-in Effects

### 5.1 Standard Library Effects

```aria
# Core effects (always available)
effect IO
  fn read(path: String) -> Bytes
  fn write(path: String, data: Bytes) -> Unit
  fn network_request(request: Request) -> Response
end

effect Console
  fn print(message: String) -> Unit
  fn print_error(message: String) -> Unit
  fn read_line() -> String
end

effect Exception[E]
  fn raise(error: E) -> Never
end

effect Async
  fn await[T](future: Future[T]) -> T
  fn spawn[T](f: Fn() -> T) -> Future[T]
  fn yield() -> Unit
end

# State effects (parameterized)
effect State[S]
  fn get() -> S
  fn put(value: S) -> Unit
  fn modify(f: Fn(S) -> S) -> Unit
end

effect Reader[R]
  fn ask() -> R
end

effect Writer[W: Monoid]
  fn tell(value: W) -> Unit
end

# Non-determinism (advanced)
effect Choice
  fn choose[T](options: Array[T]) -> T
  fn fail() -> Never
end
```

### 5.2 Effect Aliases for Common Patterns

```aria
# Convenience aliases
type Pure[T] = Fn() -> T !{}
type Effectful[T] = Fn() -> T !IO
type Failable[T, E] = Fn() -> T !Exception[E]
type Stateful[S, T] = Fn() -> T !State[S]

# Combined effect patterns
type WebHandler[T] = Fn(Request) -> T !{IO, Async, Exception[HttpError]}
type CliApp[T] = Fn(Args) -> T !{IO, Console, Exception[AppError]}
```

---

## 6. Compilation Strategy

### 6.1 Evidence-Passing Compilation (Default)

For tail-resumptive effects (90% of cases), Aria uses evidence-passing:

```aria
# Source code
fn log_twice(message: String) !Console
  Console.print(message)
  Console.print(message)
end

# Compiled (conceptual) - evidence passed as hidden parameter
fn log_twice_compiled(message: String, __console_ev: ConsoleEvidence) -> Unit
  __console_ev.print(message)
  __console_ev.print(message)
end
```

**Benefits**:
- Zero runtime allocation
- Inlines naturally
- No stack manipulation

### 6.2 Selective CPS Transformation

For effects requiring resumption control:

```aria
# Source: Non-deterministic choice
fn choose_path() !Choice
  x = Choice.choose([1, 2, 3])
  y = Choice.choose(["a", "b"])
  (x, y)
end

# Compiled: CPS transformation (only for choose_path)
fn choose_path_cps[R](
  __choice_handler: ChoiceHandler,
  __cont: Fn((Int, String)) -> R
) -> R
  __choice_handler.choose(
    [1, 2, 3],
    |x| __choice_handler.choose(
      ["a", "b"],
      |y| __cont((x, y))
    )
  )
end
```

### 6.3 Fiber-Based Async

For async effects, Aria uses one-shot fibers (OCaml 5 style):

```aria
# Source: Async operation
fn fetch_all(urls: Array[String]) -> Array[Response] !Async
  urls.map(|url| Async.await(http.get(url)))
end

# Runtime: Fiber-based execution
# - Each await suspends the fiber
# - Scheduler resumes when IO completes
# - No CPS transformation needed
```

### 6.4 Compilation Decision Table

| Effect Pattern | Detection | Compilation Strategy |
|----------------|-----------|---------------------|
| No effects | Pure function | Direct compilation |
| Tail-resumptive only | All resume in tail position | Evidence-passing |
| Non-tail resume | resume not in tail position | Local CPS |
| Multi-shot | resume called multiple times | Full CPS |
| Async | Contains Async effect | Fiber suspension |
| Mixed | Combination | Selective per-operation |

---

## 7. Migration Path

### 7.1 From Existing Aria Code

Since Aria is new, migration is from the current GRAMMAR.md style:

**Current style (no effects)**:
```aria
fn fetch_user(id: Int) -> Result[User, Error]
  response = http.get("/users/#{id}")?
  parse_user(response.body)
end
```

**With effects (fully compatible)**:
```aria
# Option 1: Let inference handle it (recommended)
fn fetch_user(id: Int) -> Result[User, Error]
  response = http.get("/users/#{id}")?  # Inferred: !IO, Exception[HttpError]
  parse_user(response.body)
end

# Option 2: Explicit annotation (documentation)
fn fetch_user(id: Int) -> Result[User, Error] !{IO, Exception[HttpError]}
  response = http.get("/users/#{id}")?
  parse_user(response.body)
end
```

### 7.2 Migration Strategy

| Phase | Timeline | Changes |
|-------|----------|---------|
| Phase 1 | Immediate | Effects inferred, no code changes needed |
| Phase 2 | 3 months | Effect annotations available for documentation |
| Phase 3 | 6 months | Custom effect definitions enabled |
| Phase 4 | 12 months | Full handler syntax available |

### 7.3 Compatibility Guarantees

1. **Inference-first**: All existing code works without modification
2. **Opt-in annotations**: Explicit effects are always optional
3. **No breaking changes**: Effect inference never rejects valid code
4. **Gradual adoption**: Mix inferred and annotated code freely

---

## 8. Example Aria Code with Effects

### 8.1 Web Server Handler

```aria
import std::http::{Request, Response, StatusCode}
import std::json::JSON

# Effect declarations (typically in stdlib)
effect Database
  fn query[T: FromRow](sql: String, params: Array[Any]) -> Array[T]
  fn execute(sql: String, params: Array[Any]) -> Int
end

effect Auth
  fn get_current_user() -> User?
  fn require_role(role: Role) -> Unit
end

# Handler with inferred effects
fn get_users_handler(req: Request) -> Response
  # Effects inferred: !{Database, Auth, IO, Exception[AppError]}

  Auth.require_role(Role.Admin)?

  page = req.query.get("page")?.parse_int() ?? 1
  limit = req.query.get("limit")?.parse_int() ?? 10

  users = Database.query[User](
    "SELECT * FROM users LIMIT ? OFFSET ?",
    [limit, (page - 1) * limit]
  )

  Response.ok()
    .json(JSON.encode(users))
end

# Wiring effects to implementation
fn run_server(config: ServerConfig)
  # Create effect implementations
  db_pool = DatabasePool.connect(config.database_url)
  auth_service = AuthService.new(config.jwt_secret)

  server = HttpServer.new(config.port)

  server.get("/users", |req|
    # Handle effects with concrete implementations
    handle get_users_handler(req) with
      Database.query(sql, params) =>
        result = db_pool.execute_query(sql, params)
        resume(result)

      Database.execute(sql, params) =>
        count = db_pool.execute_update(sql, params)
        resume(count)

      Auth.get_current_user() =>
        user = auth_service.decode_token(req.header("Authorization"))
        resume(user)

      Auth.require_role(role) =>
        user = auth_service.decode_token(req.header("Authorization"))?
        if user.role < role
          Exception.raise(Forbidden("Requires #{role} role"))
        end
        resume(Unit)

      Exception.raise(e) =>
        Response.error(e.status_code, e.message)

      return(response) => response
    end
  )

  server.listen()
end
```

### 8.2 Async Concurrent Operations

```aria
# Fetch multiple resources concurrently
fn fetch_dashboard_data(user_id: Int) -> DashboardData !Async
  # Spawn concurrent fetches
  profile_future = Async.spawn(|| fetch_profile(user_id))
  posts_future = Async.spawn(|| fetch_recent_posts(user_id))
  notifications_future = Async.spawn(|| fetch_notifications(user_id))

  # Await all results
  profile = Async.await(profile_future)
  posts = Async.await(posts_future)
  notifications = Async.await(notifications_future)

  DashboardData(profile:, posts:, notifications:)
end

# With error handling
fn fetch_dashboard_safe(user_id: Int) -> Result[DashboardData, Error]
  handle
    Ok(fetch_dashboard_data(user_id))
  with
    Exception.raise(e: NetworkError) =>
      Err(Error.network(e.message))
    Exception.raise(e: AuthError) =>
      Err(Error.auth(e.message))
    return(result) => result
  end
end
```

### 8.3 Stateful Computation

```aria
# Parser with state effect
effect Parser[S]
  fn peek() -> Char?
  fn advance() -> Char?
  fn position() -> Int
end

fn parse_identifier() -> String !Parser[String]
  result = StringBuilder.new()

  while let Some(c) = Parser.peek()
    if c.is_alphanumeric() or c == '_'
      result.push(Parser.advance().unwrap())
    else
      break
    end
  end

  result.to_string()
end

# Run parser with string input
fn parse_string[T](input: String, parser: Fn() -> T !Parser[String]) -> Result[T, ParseError]
  mut pos = 0

  handle parser() with
    Parser.peek() =>
      if pos < input.length
        resume(Some(input[pos]))
      else
        resume(None)
      end

    Parser.advance() =>
      if pos < input.length
        char = input[pos]
        pos += 1
        resume(Some(char))
      else
        resume(None)
      end

    Parser.position() =>
      resume(pos)

    return(result) => Ok(result)
  end
end
```

### 8.4 Testing with Effect Mocking

```aria
# Production code
fn process_order(order: Order) -> Receipt !{Database, Payment, Email}
  # Validate inventory
  items = Database.query[InventoryItem](
    "SELECT * FROM inventory WHERE product_id IN ?",
    [order.product_ids]
  )

  if items.any?(|i| i.quantity < order.quantity)
    Exception.raise(OutOfStock)
  end

  # Process payment
  transaction = Payment.charge(order.customer_id, order.total)

  # Update database
  Database.execute(
    "INSERT INTO orders (customer_id, total, transaction_id) VALUES (?, ?, ?)",
    [order.customer_id, order.total, transaction.id]
  )

  # Send confirmation
  Email.send(order.customer_email, "Order Confirmed", receipt_template(order))

  Receipt(order_id: order.id, transaction_id: transaction.id)
end

# Test with mocked effects
test "process_order handles payment failure"
  order = Order(
    customer_id: 1,
    product_ids: [1, 2],
    quantity: 1,
    total: 100.00,
    customer_email: "test@example.com"
  )

  # Mock implementations
  mock_db = MockDatabase.new()
  mock_db.add_result([
    InventoryItem(product_id: 1, quantity: 10),
    InventoryItem(product_id: 2, quantity: 10)
  ])

  result = handle process_order(order) with
    Database.query(sql, params) =>
      resume(mock_db.query(sql, params))

    Database.execute(sql, params) =>
      resume(mock_db.execute(sql, params))

    Payment.charge(customer, amount) =>
      # Simulate payment failure
      Exception.raise(PaymentDeclined("Insufficient funds"))

    Email.send(_, _, _) =>
      # Should not reach here
      panic!("Email should not be sent on payment failure")

    Exception.raise(e) => Err(e)
    return(receipt) => Ok(receipt)
  end

  assert result.is_err?
  assert result.unwrap_err() is PaymentDeclined
  assert mock_db.execute_count == 0  # No database writes
end
```

---

## 9. Interaction with Other Systems

### 9.1 Effect System and Contracts (ARIA-PD-001)

```aria
# Contracts can reference effect state
fn withdraw(amount: Float) -> Float !State[Account]
  requires amount > 0
  requires State.get().balance >= amount  # Effect in contract!
  ensures State.get().balance == old(State.get().balance) - amount

  account = State.get()
  State.put(Account(balance: account.balance - amount))
  amount
end
```

### 9.2 Effect System and Ownership (ARIA-PD-002)

```aria
# Effects must respect borrow lifetimes
fn process_buffer[life L](buffer: ref[L] Buffer) -> Data !IO
  # IO operations on borrowed buffer are safe
  # Handler for IO cannot outlive 'L
  buffer.read_to_end()
end

# Effect handlers capture ownership
fn with_logger[T](f: Fn() -> T !Console) -> T
  let log_buffer = Vec.new()  # Owned by handler

  handle f() with
    Console.print(msg) =>
      log_buffer.push(msg)  # Borrows log_buffer
      resume(Unit)
    return(x) =>
      File.write("log.txt", log_buffer.join("\n"))
      x
  end
end  # log_buffer drops here
```

### 9.3 Effect System and Concurrency

```aria
# Spawn preserves effect requirements
fn parallel_map[T, U, E](items: Array[T], f: Fn(T) -> U !E) -> Array[U] !{Async | E}
  futures = items.map(|item| Async.spawn(|| f(item)))
  futures.map(|fut| Async.await(fut))
end

# Effect handlers can be async-aware
fn with_timeout[T](duration: Duration, f: Fn() -> T !Async) -> Result[T, TimeoutError]
  # Implementation uses async effect internally
  race(f, || { sleep(duration); Exception.raise(TimeoutError) })
end
```

---

## 10. Trade-offs and Decisions

### 10.1 Accepted Trade-offs

| Trade-off | Decision | Rationale |
|-----------|----------|-----------|
| **Inference complexity** | Full inference with optional annotations | Matches Aria philosophy of minimal syntax |
| **Runtime overhead** | Zero for 90% of effects | Evidence-passing eliminates allocation |
| **Handler syntax verbosity** | Required for custom effects | Clear semantics worth verbosity |
| **Multi-shot effects** | Supported but discouraged | Needed for logic programming, search |

### 10.2 Rejected Alternatives

| Alternative | Reason for Rejection |
|-------------|---------------------|
| **Monad transformers** | Poor type inference, complex composition |
| **Checked exceptions (Java)** | Forces annotation, poor ergonomics |
| **No effect tracking** | Loses async optimization opportunity |
| **Full dependent types for effects** | Too complex for Aria's goals |
| **Effect subtyping (Scala 3 style)** | Less predictable inference |

### 10.3 Open Questions (Deferred)

| Question | Options | Decision Deadline |
|----------|---------|-------------------|
| Effect variance | Invariant vs covariant rows | Before beta |
| Effect aliases | Type aliases vs first-class | Before beta |
| Effect reflection | Runtime effect inspection | Post-1.0 |
| Effect optimization hints | @inline, @noinline for effects | Post-1.0 |

---

## 11. Success Metrics

### 11.1 Quantitative Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Effect annotation rate | <5% of functions | Static analysis of stdlib + apps |
| Inference success rate | >99% | Test suite coverage |
| Runtime overhead (evidence-passing) | 0% | Benchmark suite |
| Runtime overhead (CPS effects) | <10% | Benchmark suite |
| Compile time impact | <5% | Benchmark suite |

### 11.2 Qualitative Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Developer comprehension | "Effects are transparent" | User survey |
| Error message clarity | "Understood without docs" | User study |
| Testing experience | "Mocking feels natural" | Library author feedback |

---

## 12. Implementation Roadmap

### Phase 1: Core Effect Types (8 weeks)
1. Effect declaration parsing and AST
2. Effect row type representation
3. Basic effect inference (bottom-up)
4. Built-in effects (IO, Console, Exception)

### Phase 2: Effect Inference (6 weeks)
1. Full bidirectional effect inference
2. Row unification and generalization
3. Effect error messages
4. Integration with existing type checker

### Phase 3: Handler Syntax (6 weeks)
1. Handler parsing and AST
2. Handler type checking
3. Evidence-passing compilation
4. Selective CPS transformation

### Phase 4: Async Integration (4 weeks)
1. Async effect implementation
2. Fiber-based runtime
3. Spawn/await compilation
4. Performance optimization

### Phase 5: Polish (4 weeks)
1. Effect documentation
2. IDE support (effect hover, completions)
3. Performance benchmarks
4. User testing and iteration

---

## Appendix A: Effect Syntax Summary

```ebnf
effect_decl     = 'effect' type_id [ generic_params ]
                  newline
                  { effect_operation }
                  'end' ;

effect_operation = 'fn' identifier [ generic_params ]
                   '(' [ param_list ] ')' '->' type_expr ;

effect_annotation = '!' effect_row ;

effect_row      = '{' [ effect_list ] [ '|' row_var ] '}'
                | row_var
                | '{' '}' ;

effect_list     = effect { ',' effect } ;

effect          = type_id [ '[' type_list ']' ] ;

row_var         = lower_identifier ;

handler_expr    = 'handle' expression 'with'
                  { handler_clause }
                  'end' ;

handler_clause  = effect '.' identifier '(' pattern_list ')' '=>' expression
                | 'return' '(' pattern ')' '=>' expression ;
```

---

## Appendix B: Comparison with Other Languages

| Feature | Aria | Koka | OCaml 5 | Scala 3 | Eff |
|---------|------|------|---------|---------|-----|
| Effect inference | Yes | Yes | Limited | No | No |
| Row polymorphism | Yes | Yes | No | No | Yes |
| Handler syntax | Yes | Yes | Yes | No | Yes |
| One-shot optimization | Yes | Yes | Yes | N/A | No |
| Evidence-passing | Yes | Yes | No | N/A | No |
| Integration with ownership | Yes | N/A | No | N/A | No |

---

## Appendix C: Research Attribution

This decision synthesizes research from:

1. **ARIA-M03-01** (KIRA): Survey of algebraic effects implementations
2. **ARIA-M03-02** (KIRA): Deep dive into Koka's effect system
3. **ARIA-M03-03** (KIRA): Effect inference approaches
4. **ARIA-M03-04** (KIRA): Algebraic effects deep dive
5. **ARIA-M03-05** (PRISM): Effect compilation strategies

Key academic references:
- Leijen (2017): "Type Directed Compilation of Row-Typed Algebraic Effects"
- Sivaramakrishnan et al. (2021): "Retrofitting Effect Handlers onto OCaml"
- Pretnar (2015): "An Introduction to Algebraic Effects and Handlers"
- Lindley et al. (2017): "Do Be Do Be Do"

---

## Appendix D: Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-15 | Row-polymorphic effects | Best inference, proven in Koka |
| 2026-01-15 | Evidence-passing default | Zero overhead for common cases |
| 2026-01-15 | `!` suffix syntax | Consistent with Result `?` operator |
| 2026-01-15 | One-shot fibers for async | Matches OCaml 5, efficient |
| 2026-01-15 | Optional annotations | Matches Aria's inference-first philosophy |
| 2026-01-15 | Built-in standard effects | IO, Console, Exception, Async, State |

---

**Document Status**: Approved for implementation
**Next Steps**: ARIA-M03-05 - Prototype effect inference
**Owner**: Compiler Team
**Reviewers**: KIRA (Research), PRISM (Compilation), ORACLE (Product)
