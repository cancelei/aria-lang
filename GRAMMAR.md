# Aria Language Grammar Specification

**Version:** 0.1.0
**Status:** Draft
**Notation:** Extended Backus-Naur Form (EBNF)

---

## Table of Contents

1. [Lexical Structure](#1-lexical-structure)
2. [Top-Level Declarations](#2-top-level-declarations)
3. [Type System](#3-type-system)
4. [Functions](#4-functions)
5. [Contracts](#5-contracts)
6. [Expressions](#6-expressions)
7. [Statements](#7-statements)
8. [Pattern Matching](#8-pattern-matching)
9. [Control Flow](#9-control-flow)
10. [Data Structures](#10-data-structures)
11. [Testing Constructs](#11-testing-constructs)
12. [FFI & Interop](#12-ffi--interop)
13. [Concurrency](#13-concurrency)
14. [Modules & Imports](#14-modules--imports)

---

## 1. Lexical Structure

### 1.1 Character Set

```ebnf
letter          = 'a'..'z' | 'A'..'Z' ;
digit           = '0'..'9' ;
hex_digit       = digit | 'a'..'f' | 'A'..'F' ;
octal_digit     = '0'..'7' ;
binary_digit    = '0' | '1' ;

newline         = '\n' | '\r\n' ;
whitespace      = ' ' | '\t' ;
```

### 1.2 Comments

```ebnf
comment         = line_comment | block_comment | doc_comment ;
line_comment    = '#' { any_char - newline } newline ;
block_comment   = '###' { any_char | newline } '###' ;
doc_comment     = '##' { any_char - newline } newline ;
```

### 1.3 Identifiers

```ebnf
identifier      = ( letter | '_' ) { letter | digit | '_' } [ '?' | '!' ] ;
constant_id     = upper_letter { letter | digit | '_' } ;
type_id         = upper_letter { letter | digit } ;

upper_letter    = 'A'..'Z' ;
lower_letter    = 'a'..'z' ;
```

**Examples:**
```ruby
name            # regular identifier
empty?          # predicate (returns boolean)
save!           # mutating operation
MAX_SIZE        # constant
UserProfile     # type name
_private        # private by convention
```

### 1.4 Keywords

```ebnf
keyword         = 'fn' | 'let' | 'mut' | 'if' | 'else' | 'elsif' | 'match'
                | 'for' | 'while' | 'loop' | 'break' | 'continue' | 'return'
                | 'struct' | 'data' | 'enum' | 'trait' | 'impl'
                | 'module' | 'import' | 'export' | 'from' | 'as'
                | 'extern' | 'unsafe' | 'defer'
                | 'spawn' | 'await' | 'select'
                | 'requires' | 'ensures' | 'invariant'
                | 'examples' | 'property' | 'forall' | 'exists'
                | 'true' | 'false' | 'nil'
                | 'self' | 'Self' | 'super'
                | 'and' | 'or' | 'not' | 'in' | 'is'
                | 'ref' | 'move' | 'copy'
                | 'pub' | 'priv'
                | 'end' ;
```

### 1.5 Operators

```ebnf
operator        = arithmetic_op | comparison_op | logical_op
                | bitwise_op | assignment_op | special_op ;

arithmetic_op   = '+' | '-' | '*' | '/' | '//' | '%' | '**' ;
comparison_op   = '==' | '!=' | '<' | '>' | '<=' | '>=' | '<=>' ;
logical_op      = 'and' | 'or' | 'not' | '&&' | '||' | '!' ;
bitwise_op      = '&' | '|' | '^' | '~' | '<<' | '>>' ;
assignment_op   = '=' | '+=' | '-=' | '*=' | '/=' | '//=' | '%='
                | '&=' | '|=' | '^=' | '<<=' | '>>=' ;
special_op      = '->' | '=>' | '|>' | '..' | '..<' | '..=' | '?'
                | '::' | '.' | '&' | '@' ;
```

### 1.6 Literals

```ebnf
literal         = integer_lit | float_lit | string_lit | char_lit
                | bool_lit | nil_lit | array_lit | map_lit | tuple_lit ;

(* Integers *)
integer_lit     = decimal_lit | hex_lit | octal_lit | binary_lit ;
decimal_lit     = digit { digit | '_' } [ integer_suffix ] ;
hex_lit         = '0x' hex_digit { hex_digit | '_' } [ integer_suffix ] ;
octal_lit       = '0o' octal_digit { octal_digit | '_' } [ integer_suffix ] ;
binary_lit      = '0b' binary_digit { binary_digit | '_' } [ integer_suffix ] ;
integer_suffix  = 'i8' | 'i16' | 'i32' | 'i64' | 'i128'
                | 'u8' | 'u16' | 'u32' | 'u64' | 'u128' | 'isize' | 'usize' ;

(* Floats *)
float_lit       = decimal_lit '.' decimal_lit [ exponent ] [ float_suffix ]
                | decimal_lit exponent [ float_suffix ] ;
exponent        = ( 'e' | 'E' ) [ '+' | '-' ] decimal_lit ;
float_suffix    = 'f32' | 'f64' ;

(* Strings *)
string_lit      = simple_string | interpolated_string | raw_string | heredoc ;
simple_string   = "'" { string_char | escape_seq } "'" ;
interpolated_string = '"' { string_char | escape_seq | interpolation } '"' ;
raw_string      = 'r"' { any_char - '"' } '"' ;
heredoc         = '<<' identifier newline { any_char } newline identifier ;

interpolation   = '#{' expression '}' ;
escape_seq      = '\\' ( 'n' | 'r' | 't' | '\\' | '"' | "'" | '0'
                       | 'x' hex_digit hex_digit
                       | 'u{' hex_digit { hex_digit } '}' ) ;

(* Characters *)
char_lit        = '`' ( string_char | escape_seq ) '`' ;

(* Boolean and Nil *)
bool_lit        = 'true' | 'false' ;
nil_lit         = 'nil' ;

(* Collections *)
array_lit       = '[' [ expression_list ] ']' ;
map_lit         = '{' [ map_entry_list ] '}' ;
tuple_lit       = '(' expression ',' [ expression_list ] ')' ;

map_entry_list  = map_entry { ',' map_entry } [ ',' ] ;
map_entry       = ( identifier ':' expression )
                | ( expression '=>' expression ) ;
```

**Examples:**
```ruby
# Integers
42
1_000_000
0xFF_AA_BB
0b1010_1010
42u64

# Floats
3.14159
1.0e-10
2.5f32

# Strings
'simple string'
"Hello, #{name}!"
r"raw\nstring"
<<EOF
  Multi-line
  heredoc
EOF

# Collections
[1, 2, 3]
{name: "Alice", age: 30}
{"key" => value}
(1, "two", 3.0)
```

---

## 2. Top-Level Declarations

```ebnf
program         = { top_level_decl } ;

top_level_decl  = module_decl
                | import_decl
                | export_decl
                | extern_decl
                | struct_decl
                | data_decl
                | enum_decl
                | trait_decl
                | impl_decl
                | function_decl
                | constant_decl
                | type_alias ;

visibility      = [ 'pub' | 'priv' ] ;
```

---

## 3. Type System

### 3.1 Type Expressions

```ebnf
type_expr       = simple_type | compound_type | function_type | generic_type ;

simple_type     = 'Int' | 'Int8' | 'Int16' | 'Int32' | 'Int64' | 'Int128'
                | 'UInt' | 'UInt8' | 'UInt16' | 'UInt32' | 'UInt64' | 'UInt128'
                | 'Float' | 'Float32' | 'Float64'
                | 'Bool' | 'Char' | 'String' | 'Bytes'
                | 'Never' | 'Unit' | 'Any'
                | type_id ;

compound_type   = array_type | map_type | tuple_type | optional_type
                | result_type | reference_type ;

array_type      = 'Array' '<' type_expr '>'
                | '[' type_expr ']'
                | '[' type_expr ';' integer_lit ']' ;   (* fixed-size *)

map_type        = 'Map' '<' type_expr ',' type_expr '>'
                | '{' type_expr ':' type_expr '}' ;

tuple_type      = '(' type_expr ',' type_list ')' ;
type_list       = type_expr { ',' type_expr } ;

optional_type   = type_expr '?' ;
result_type     = 'Result' '<' type_expr [ ',' type_expr ] '>' ;

reference_type  = '&' [ 'mut' ] type_expr ;

function_type   = 'Fn' '(' [ type_list ] ')' [ '->' type_expr ] ;

generic_type    = type_id '<' type_list '>' ;
```

### 3.2 Type Constraints

```ebnf
type_constraint = type_id ':' trait_bound { '+' trait_bound } ;
trait_bound     = type_id [ '<' type_list '>' ] ;

where_clause    = 'where' type_constraint { ',' type_constraint } ;
```

**Examples:**
```ruby
Int                         # Simple type
Array<String>               # Generic type
[Int]                       # Array shorthand
[Int; 10]                   # Fixed-size array
{String: Int}               # Map shorthand
(Int, String, Bool)         # Tuple
String?                     # Optional (Maybe)
Result<User, Error>         # Result type
&String                     # Immutable reference
&mut Array<Int>             # Mutable reference
Fn(Int, Int) -> Int         # Function type
```

---

## 4. Functions

### 4.1 Function Declaration

```ebnf
function_decl   = visibility 'fn' identifier [ generic_params ]
                  '(' [ param_list ] ')' [ '->' type_expr ]
                  [ where_clause ]
                  [ contract_block ]
                  ( function_body | '=' expression )
                  [ test_block ] ;

generic_params  = '<' generic_param { ',' generic_param } '>' ;
generic_param   = type_id [ ':' trait_bound { '+' trait_bound } ] ;

param_list      = param { ',' param } [ ',' ] ;
param           = [ 'mut' ] identifier [ ':' type_expr ] [ '=' expression ] ;

function_body   = newline { statement } 'end' ;
```

### 4.2 Lambda / Block Expressions

```ebnf
lambda_expr     = block_lambda | arrow_lambda ;

block_lambda    = '{' [ '|' param_list '|' ] { statement } '}' ;
arrow_lambda    = '(' [ param_list ] ')' '=>' expression
                | identifier '=>' expression ;

block_short     = '&:' identifier ;   (* shorthand for { |x| x.method } *)
```

**Examples:**
```ruby
# Full function
fn calculate_total(items: Array<Item>, tax_rate: Float = 0.1) -> Float
  requires items.length > 0
  ensures result >= 0.0

  subtotal = items.map(&:price).sum
  subtotal * (1 + tax_rate)
end

# Short function
fn double(x) = x * 2

# Generic function
fn first<T>(items: Array<T>) -> T?
  items[0]
end

# Lambdas
numbers.map { |n| n * 2 }
numbers.map(&:to_s)
numbers.filter { |n| n > 0 }
add = (a, b) => a + b
```

---

## 5. Contracts

### 5.1 Contract Block

```ebnf
contract_block  = { contract_clause } ;

contract_clause = requires_clause
                | ensures_clause
                | invariant_clause ;

requires_clause = 'requires' expression [ ':' string_lit ] ;
ensures_clause  = 'ensures' expression [ ':' string_lit ] ;
invariant_clause = 'invariant' expression [ ':' string_lit ] ;
```

### 5.2 Contract Keywords

```ebnf
contract_expr   = 'old' '(' expression ')'      (* previous value *)
                | 'result'                       (* return value *)
                | 'forall' quantifier_expr
                | 'exists' quantifier_expr ;

quantifier_expr = identifier ':' type_expr
                  [ 'where' expression ]
                  ',' expression ;
```

**Examples:**
```ruby
fn binary_search<T: Ord>(arr: Array<T>, target: T) -> Int?
  requires arr.sorted?                              : "array must be sorted"
  requires arr.length > 0                           : "array cannot be empty"
  ensures result.nil? or arr[result.unwrap] == target
  ensures forall i: Int where 0 <= i < arr.length,
          arr[i] == target implies result == Some(i)

  # implementation...
end

fn withdraw(account: &mut Account, amount: Float) -> Result<Float>
  requires amount > 0                               : "amount must be positive"
  requires account.balance >= amount                : "insufficient funds"
  ensures account.balance == old(account.balance) - amount
  ensures result.ok? implies result.unwrap == amount

  account.balance -= amount
  Ok(amount)
end
```

---

## 6. Expressions

### 6.1 Expression Hierarchy

```ebnf
expression      = assignment_expr ;

assignment_expr = conditional_expr [ assignment_op assignment_expr ] ;

conditional_expr = or_expr [ '?' expression ':' expression ]
                 | 'if' expression 'then' expression 'else' expression ;

or_expr         = and_expr { ( 'or' | '||' ) and_expr } ;
and_expr        = not_expr { ( 'and' | '&&' ) not_expr } ;
not_expr        = [ 'not' | '!' ] comparison_expr ;

comparison_expr = range_expr { comparison_op range_expr } ;
range_expr      = bitwise_expr [ ( '..' | '..<' | '..=' ) bitwise_expr ] ;

bitwise_expr    = shift_expr { ( '&' | '|' | '^' ) shift_expr } ;
shift_expr      = additive_expr { ( '<<' | '>>' ) additive_expr } ;

additive_expr   = multiplicative_expr { ( '+' | '-' ) multiplicative_expr } ;
multiplicative_expr = unary_expr { ( '*' | '/' | '//' | '%' ) unary_expr } ;

unary_expr      = [ '-' | '+' | '~' | '&' | '*' ] power_expr ;
power_expr      = postfix_expr [ '**' unary_expr ] ;

postfix_expr    = primary_expr { postfix_op } ;
postfix_op      = call_expr | index_expr | field_access | method_call
                | unwrap_op | pipe_expr ;

call_expr       = '(' [ arg_list ] ')' ;
index_expr      = '[' expression [ ':' expression ] ']' ;
field_access    = '.' identifier ;
method_call     = '.' identifier '(' [ arg_list ] ')' ;
unwrap_op       = '?' | '!' ;
pipe_expr       = '|>' identifier [ '(' [ arg_list ] ')' ] ;

arg_list        = arg { ',' arg } [ ',' ] ;
arg             = [ identifier ':' ] expression
                | '...' expression ;    (* spread *)
```

### 6.2 Primary Expressions

```ebnf
primary_expr    = literal
                | identifier
                | grouped_expr
                | array_comprehension
                | map_comprehension
                | if_expr
                | match_expr
                | try_expr
                | lambda_expr
                | struct_init
                | tuple_expr
                | 'self'
                | 'Self' ;

grouped_expr    = '(' expression ')' ;
tuple_expr      = '(' expression ',' [ expression_list ] ')' ;

array_comprehension = '[' expression 'for' pattern 'in' expression
                      [ 'if' expression ] ']' ;
map_comprehension   = '{' expression '=>' expression
                      'for' pattern 'in' expression
                      [ 'if' expression ] '}' ;
```

**Examples:**
```ruby
# Arithmetic
a + b * c ** 2

# Comparison chaining
0 <= x < 100

# Ranges
1..10           # inclusive: 1, 2, ..., 10
1..<10          # exclusive: 1, 2, ..., 9

# Pipe operator
data
  |> parse
  |> validate
  |> transform
  |> save

# Conditional
result = if valid? then process(data) else default_value

# Ternary
status = age >= 18 ? "adult" : "minor"

# Safe navigation
user?.profile?.avatar?.url

# Comprehensions
squares = [x ** 2 for x in 1..10]
evens = [x for x in numbers if x.even?]
lookup = {k => v.upcase for (k, v) in pairs}

# Struct initialization
user = User(name: "Alice", age: 30)
point = Point(x: 1.0, y: 2.0)
```

---

## 7. Statements

```ebnf
statement       = expression_stmt
                | declaration_stmt
                | assignment_stmt
                | control_stmt
                | defer_stmt
                | unsafe_block ;

expression_stmt = expression newline ;

declaration_stmt = let_decl | var_decl | const_decl ;

let_decl        = 'let' pattern [ ':' type_expr ] '=' expression ;
var_decl        = identifier [ ':' type_expr ] '=' expression ;
const_decl      = constant_id [ ':' type_expr ] '=' expression ;

assignment_stmt = lvalue assignment_op expression ;
lvalue          = identifier | field_access | index_expr ;

defer_stmt      = 'defer' ( expression | block ) ;

unsafe_block    = 'unsafe' newline { statement } 'end' ;
```

**Examples:**
```ruby
# Declarations
let name = "Alice"              # immutable
age = 30                        # mutable
const MAX_SIZE = 1024           # constant

# Destructuring
let (x, y, z) = get_coordinates()
let {name:, age:} = user
let [first, second, ...rest] = items

# Defer (runs at end of scope)
fn read_file(path)
  file = File.open(path)
  defer file.close()            # guaranteed cleanup

  file.read_all()
end
```

---

## 8. Pattern Matching

### 8.1 Pattern Syntax

```ebnf
pattern         = literal_pattern
                | identifier_pattern
                | wildcard_pattern
                | tuple_pattern
                | array_pattern
                | struct_pattern
                | enum_pattern
                | range_pattern
                | or_pattern
                | guard_pattern
                | binding_pattern
                | type_pattern ;

literal_pattern   = literal ;
identifier_pattern = identifier ;
wildcard_pattern  = '_' ;

tuple_pattern     = '(' pattern { ',' pattern } ')' ;
array_pattern     = '[' [ pattern { ',' pattern } [ ',' '...' [ identifier ] ] ] ']' ;
struct_pattern    = type_id '(' [ field_pattern { ',' field_pattern } ] ')'
                  | '{' [ field_pattern { ',' field_pattern } ] '}' ;
field_pattern     = identifier [ ':' pattern ] ;

enum_pattern      = type_id '::' identifier [ '(' [ pattern_list ] ')' ] ;

range_pattern     = literal '..' literal
                  | literal '..<' literal ;

or_pattern        = pattern ( '|' pattern )+ ;
guard_pattern     = pattern 'if' expression ;
binding_pattern   = identifier '@' pattern ;
type_pattern      = pattern ':' type_expr ;
```

### 8.2 Match Expression

```ebnf
match_expr      = 'match' expression newline
                  { match_arm }
                  'end' ;

match_arm       = pattern [ 'if' expression ] '=>'
                  ( expression | newline { statement } ) ;
```

**Examples:**
```ruby
# Basic matching
match value
  0           => "zero"
  1 | 2 | 3   => "small"
  4..10       => "medium"
  n if n < 0  => "negative: #{n}"
  _           => "other"
end

# Destructuring
match point
  Point(x: 0, y: 0)     => "origin"
  Point(x: 0, y:)       => "on y-axis at #{y}"
  Point(x:, y: 0)       => "on x-axis at #{x}"
  Point(x:, y:)         => "at (#{x}, #{y})"
end

# Enum matching
match result
  Ok(value)             => process(value)
  Err(NotFound)         => nil
  Err(e @ NetworkError) => retry(e)
  Err(e)                => raise e
end

# Array patterns
match items
  []                    => "empty"
  [single]              => "one: #{single}"
  [first, second]       => "two: #{first}, #{second}"
  [head, ...tail]       => "head: #{head}, rest: #{tail.length}"
end
```

---

## 9. Control Flow

### 9.1 Conditional

```ebnf
if_stmt         = 'if' expression newline
                  { statement }
                  { elsif_clause }
                  [ else_clause ]
                  'end' ;

elsif_clause    = 'elsif' expression newline { statement } ;
else_clause     = 'else' newline { statement } ;

if_expr         = 'if' expression 'then' expression 'else' expression ;

unless_stmt     = 'unless' expression newline
                  { statement }
                  [ else_clause ]
                  'end' ;
```

### 9.2 Loops

```ebnf
loop_stmt       = for_loop | while_loop | loop_infinite ;

for_loop        = 'for' pattern 'in' expression newline
                  { statement }
                  'end' ;

while_loop      = 'while' expression newline
                  { statement }
                  'end' ;

loop_infinite   = 'loop' newline
                  { statement }
                  'end' ;

break_stmt      = 'break' [ expression ] ;
continue_stmt   = 'continue' ;
return_stmt     = 'return' [ expression ] ;
```

**Examples:**
```ruby
# If statement
if condition
  do_something()
elsif other_condition
  do_other()
else
  do_default()
end

# Unless (negated if)
unless valid?
  raise InvalidError
end

# For loop
for item in items
  process(item)
end

for (index, value) in items.enumerate()
  print("#{index}: #{value}")
end

# While loop
while running?
  tick()
end

# Infinite loop with break
loop
  event = poll()
  break if event.quit?
  handle(event)
end

# Loop with value (like Rust)
result = loop
  if ready?
    break compute_result()
  end
  wait()
end
```

---

## 10. Data Structures

### 10.1 Struct

```ebnf
struct_decl     = visibility 'struct' type_id [ generic_params ]
                  newline
                  { struct_field }
                  [ struct_derive ]
                  'end' ;

struct_field    = visibility identifier ':' type_expr
                  [ '=' expression ]      (* default value *)
                  newline ;

struct_derive   = 'derive' '(' derive_list ')' ;
derive_list     = type_id { ',' type_id } ;

struct_init     = type_id '(' [ field_init_list ] ')'
                | type_id '{' [ field_init_list ] '}' ;
field_init_list = field_init { ',' field_init } [ ',' ] ;
field_init      = identifier ':' expression
                | identifier                  (* shorthand: name: name *)
                | '...' expression ;          (* spread *)
```

### 10.2 Data (Record)

```ebnf
data_decl       = visibility 'data' type_id [ generic_params ]
                  '(' data_fields ')'
                  [ struct_derive ] ;

data_fields     = data_field { ',' data_field } ;
data_field      = identifier ':' type_expr [ '=' expression ] ;
```

### 10.3 Enum

```ebnf
enum_decl       = visibility 'enum' type_id [ generic_params ]
                  newline
                  { enum_variant }
                  [ struct_derive ]
                  'end' ;

enum_variant    = identifier [ enum_variant_data ] newline ;
enum_variant_data = '(' type_list ')'
                  | '(' data_fields ')'
                  | '=' expression ;
```

### 10.4 Trait

```ebnf
trait_decl      = visibility 'trait' type_id [ generic_params ]
                  [ ':' trait_bound { '+' trait_bound } ]
                  newline
                  { trait_member }
                  'end' ;

trait_member    = trait_method | trait_const | trait_type ;

trait_method    = 'fn' identifier [ generic_params ]
                  '(' [ param_list ] ')' [ '->' type_expr ]
                  [ '=' expression | function_body ] ;

trait_const     = 'const' identifier ':' type_expr [ '=' expression ] ;
trait_type      = 'type' type_id [ ':' trait_bound ] [ '=' type_expr ] ;
```

### 10.5 Implementation

```ebnf
impl_decl       = 'impl' [ generic_params ]
                  [ trait_type 'for' ] type_expr
                  [ where_clause ]
                  newline
                  { impl_member }
                  'end' ;

impl_member     = function_decl | const_decl | type_alias ;
```

**Examples:**
```ruby
# Struct with defaults
struct User
  pub name: String
  pub email: String
  age: Int = 0
  active: Bool = true

  derive(Eq, Hash, Clone, Debug)
end

# Data record (immutable, auto-derives)
data Point(x: Float, y: Float)
data Color(r: UInt8, g: UInt8, b: UInt8, a: UInt8 = 255)

# Enum with variants
enum Result<T, E>
  Ok(T)
  Err(E)
end

enum Option<T>
  Some(T)
  None
end

enum Message
  Quit
  Move(x: Int, y: Int)
  Write(String)
  Color(r: Int, g: Int, b: Int)
end

# Trait definition
trait Display
  fn to_string(self) -> String
end

trait Numeric: Add + Sub + Mul + Div
  const ZERO: Self
  const ONE: Self

  fn abs(self) -> Self
  fn pow(self, exp: Int) -> Self
end

# Implementation
impl User
  fn new(name: String, email: String) -> Self
    Self(name:, email:)
  end

  fn adult?(self) -> Bool
    self.age >= 18
  end
end

impl Display for User
  fn to_string(self) -> String
    "User(#{self.name}, #{self.email})"
  end
end

impl<T: Display> Display for Array<T>
  fn to_string(self) -> String
    "[" + self.map(&:to_string).join(", ") + "]"
  end
end
```

---

## 11. Testing Constructs

### 11.1 Examples Block

```ebnf
examples_block  = 'examples' newline
                  { example_assertion }
                  'end' ;

example_assertion = expression ( '==' | '!=' | '<' | '>' | '<=' | '>=' ) expression
                  | expression                  (* truthy assertion *)
                  | 'raises' type_id newline expression ;
```

### 11.2 Property Block

```ebnf
property_block  = 'property' string_lit newline
                  { property_body }
                  'end' ;

property_body   = quantified_assertion | expression ;

quantified_assertion = 'forall' identifier ':' type_expr
                       [ 'where' expression ]
                       newline expression ;
```

### 11.3 Test Declaration

```ebnf
test_decl       = 'test' string_lit newline
                  { statement }
                  'end' ;
```

**Examples:**
```ruby
fn factorial(n: Int) -> Int
  requires n >= 0
  ensures result >= 1

  match n
    0, 1 => 1
    _    => n * factorial(n - 1)
  end

  examples
    factorial(0) == 1
    factorial(1) == 1
    factorial(5) == 120
    factorial(10) == 3628800
  end

  property "result is always positive"
    forall n: Int where n >= 0
      factorial(n) > 0
    end
  end

  property "factorial grows monotonically"
    forall n: Int where n > 0
      factorial(n) > factorial(n - 1)
    end
  end
end

# Standalone test
test "user creation with valid data"
  user = User.new("Alice", "alice@example.com")

  assert user.name == "Alice"
  assert user.email == "alice@example.com"
  assert user.active?
end

test "division by zero raises error"
  raises DivisionByZero
    divide(10, 0)
  end
end
```

---

## 12. FFI & Interop

### 12.1 Extern Declarations

```ebnf
extern_decl     = extern_c | extern_python | extern_wasm ;

extern_c        = 'extern' 'C' 'from' string_lit [ 'as' identifier ]
                  [ newline extern_c_body 'end' ] ;

extern_c_body   = { extern_c_item } ;
extern_c_item   = extern_fn | extern_struct | extern_const | extern_type ;

extern_fn       = 'fn' identifier '(' [ c_param_list ] ')' [ '->' c_type ] ;
extern_struct   = 'struct' type_id newline { c_field } 'end' ;
extern_const    = 'const' identifier ':' c_type ;
extern_type     = 'type' type_id ;

c_type          = 'c_int' | 'c_uint' | 'c_long' | 'c_ulong' | 'c_longlong'
                | 'c_float' | 'c_double' | 'c_char' | 'c_void'
                | 'c_size_t' | 'c_ssize_t'
                | '*' [ 'const' ] c_type         (* pointer *)
                | type_id ;

extern_python   = 'extern' 'Python' 'from' identifier [ 'as' identifier ] ;

extern_wasm     = 'extern' 'WASM' ( 'import' | 'export' ) identifier
                  [ newline wasm_body 'end' ] ;
```

### 12.2 Unsafe Blocks

```ebnf
unsafe_block    = 'unsafe' newline
                  { statement }
                  'end' ;

unsafe_fn       = 'unsafe' 'fn' identifier ... ;
```

**Examples:**
```ruby
# C interop - direct header import (Zig-style)
extern C from "sqlite3.h"
extern C from "openssl/ssl.h" as ssl

# C interop - explicit declarations
extern C from "mylib.h"
  fn my_function(x: c_int, y: c_int) -> c_int
  fn process_data(data: *c_void, len: c_size_t) -> c_int

  struct MyStruct
    field1: c_int
    field2: *c_char
  end

  const MY_CONSTANT: c_int
end

# Python interop
extern Python from numpy as np
extern Python from pandas as pd
extern Python from sklearn.linear_model as lm

fn analyze_data(data: Array<Float>) -> Array<Float>
  arr = np.array(data)
  model = lm.LinearRegression()
  model.fit(arr)
  Array.from(model.predict(arr))
end

# WASM exports
extern WASM export MyModule
  fn add(a: Int, b: Int) -> Int
  fn process(data: Bytes) -> Bytes
end

# Unsafe operations
fn read_raw_memory(ptr: *UInt8, len: Int) -> Bytes
  unsafe
    bytes = Bytes.with_capacity(len)
    for i in 0..<len
      bytes.push(ptr.offset(i).read())
    end
    bytes
  end
end
```

---

## 13. Concurrency

### 13.1 Spawn & Async

```ebnf
spawn_expr      = 'spawn' ( expression | block ) ;
await_expr      = expression '.await'
                | 'await' expression
                | expression_list '.await_all' ;

select_expr     = 'select' newline
                  { select_arm }
                  'end' ;

select_arm      = pattern '=' '<-' expression '=>' expression
                | 'default' '=>' expression ;
```

### 13.2 Channels

```ebnf
channel_type    = 'Channel' '<' type_expr '>' ;
channel_ops     = channel_send | channel_recv ;
channel_send    = expression '.' 'send' '(' expression ')' ;
channel_recv    = '<-' expression
                | expression '.' 'recv' '(' ')' ;
```

**Examples:**
```ruby
# Spawn concurrent task
handle = spawn {
  expensive_computation()
}

result = handle.await

# Parallel map
results = items.par.map { |item| process(item) }

# Channels
fn producer_consumer
  ch = Channel.new(Int, capacity: 10)

  # Producer
  spawn {
    for i in 1..100
      ch.send(i)
    end
    ch.close()
  }

  # Consumer
  for value in ch
    process(value)
  end
end

# Select (like Go)
fn multiplexer(ch1: Channel<Int>, ch2: Channel<String>)
  loop
    select
      n = <-ch1 => print("Got int: #{n}")
      s = <-ch2 => print("Got string: #{s}")
      default   => sleep(10.ms)
    end
  end
end

# Async inference (no async keyword needed)
fn fetch_all(urls: Array<String>) -> Array<Response>
  # Compiler infers this is async and parallelizable
  urls.map { |url| http.get(url) }.await_all
end
```

---

## 14. Modules & Imports

### 14.1 Module Declaration

```ebnf
module_decl     = 'module' module_path newline
                  { top_level_decl }
                  'end' ;

module_path     = identifier { '::' identifier } ;
```

### 14.2 Import Declaration

```ebnf
import_decl     = 'import' import_path [ import_alias ] [ import_selection ] ;

import_path     = module_path | string_lit ;
import_alias    = 'as' identifier ;
import_selection = '::' ( '*' | '{' import_list '}' ) ;
import_list     = import_item { ',' import_item } ;
import_item     = identifier [ 'as' identifier ] ;
```

### 14.3 Export Declaration

```ebnf
export_decl     = 'export' ( '*' | '{' export_list '}' ) ;
export_list     = identifier { ',' identifier } ;
```

**Examples:**
```ruby
# Module declaration
module MyApp::Models
  struct User
    # ...
  end

  struct Post
    # ...
  end
end

# Imports
import std::collections::{Array, Map, Set}
import std::io::File
import std::net::http as http

import MyApp::Models::User
import MyApp::Models::*                    # Import all public items

import "vendor/custom_lib" as custom       # Path-based import

# Re-exports
module MyApp
  import MyApp::Models::User
  import MyApp::Models::Post

  export {User, Post}                      # Re-export for consumers
end

# File structure convention
# src/
#   main.aria                              # module Main
#   models/
#     mod.aria                             # module Models (index)
#     user.aria                            # module Models::User
#     post.aria                            # module Models::Post
```

---

## 15. Compiler Directives & Attributes

```ebnf
attribute       = '@' attribute_name [ '(' attr_args ')' ] ;
attribute_name  = identifier ;
attr_args       = attr_arg { ',' attr_arg } ;
attr_arg        = identifier [ ':' expression ]
                | expression ;

inner_attribute = '@!' attribute_name [ '(' attr_args ')' ] ;
```

**Examples:**
```ruby
# Function attributes
@inline
fn small_helper(x: Int) = x + 1

@optimize(level: :aggressive, verify: :formal)
fn hot_path(data: Array<Float>) -> Float
  # LLM optimizer enabled for this function
end

@deprecated("Use new_function instead")
fn old_function()
  # ...
end

@target(:wasm)
fn browser_only()
  # Only compiled for WASM target
end

# Struct attributes
@derive(Eq, Hash, Clone, Serialize, Deserialize)
struct Config
  # ...
end

@repr(C)                    # C-compatible memory layout
struct FFIStruct
  # ...
end

# Test attributes
@test
@timeout(5.seconds)
fn slow_test()
  # ...
end

# Conditional compilation
@cfg(target: :linux)
fn linux_specific()
  # ...
end

@cfg(debug)
fn debug_only()
  # ...
end
```

---

## 16. Full Example Program

```ruby
#!/usr/bin/env aria
@!version("0.1.0")
@!authors(["Your Name <you@example.com>"])

module Example

import std::io::{print, File}
import std::net::http
import std::json::JSON

## A simple web scraper example demonstrating Aria's features.

# Data types
data Article(title: String, url: String, content: String?)
data ScrapedResult(articles: Array<Article>, errors: Array<String>)

# Configuration
struct Config
  pub base_url: String
  pub max_pages: Int = 10
  pub timeout: Duration = 30.seconds
  pub retry_count: Int = 3

  derive(Clone, Debug)
end

# Traits
trait Scrapable
  fn scrape(self, url: String) -> Result<String>
  fn parse(self, html: String) -> Array<Article>
end

# Implementation
impl Scrapable for Config
  fn scrape(self, url: String) -> Result<String>
    requires url.starts_with?("http")
    ensures result.ok? implies result.unwrap.length > 0

    response = http.get(url, timeout: self.timeout)?

    if response.status != 200
      return Err(HttpError("Status: #{response.status}"))
    end

    Ok(response.body)
  end

  fn parse(self, html: String) -> Array<Article>
    # Simplified parsing logic
    html.scan(/<article.*?>(.*?)<\/article>/m)
        .map { |match| extract_article(match) }
        .compact
  end

  examples
    config = Config(base_url: "https://example.com")
    config.max_pages == 10
    config.timeout == 30.seconds
  end
end

# Main scraping function
fn scrape_site(config: Config) -> ScrapedResult
  requires config.max_pages > 0
  ensures result.articles.length <= config.max_pages

  articles = []
  errors = []

  for page in 1..=config.max_pages
    url = "#{config.base_url}/page/#{page}"

    match config.scrape(url)
      Ok(html) =>
        articles.append(config.parse(html))
      Err(e) =>
        errors.push("Page #{page}: #{e}")
    end
  end

  ScrapedResult(articles: articles.flatten, errors:)
end

# Parallel version
fn scrape_site_parallel(config: Config) -> ScrapedResult
  urls = (1..=config.max_pages).map { |p| "#{config.base_url}/page/#{p}" }

  results = urls.par.map { |url|
    match config.scrape(url)
      Ok(html) => Ok(config.parse(html))
      Err(e)   => Err(e.to_string)
    end
  }

  articles = results.filter_map(&:ok?).flatten
  errors = results.filter_map(&:err?)

  ScrapedResult(articles:, errors:)
end

# Entry point
fn main
  config = Config(
    base_url: "https://news.example.com",
    max_pages: 5
  )

  print("Starting scrape of #{config.base_url}...")

  result = scrape_site_parallel(config)

  print("Found #{result.articles.length} articles")

  if result.errors.any?
    print("Errors encountered:")
    for error in result.errors
      print("  - #{error}")
    end
  end

  # Save results
  File.write("articles.json", JSON.encode(result.articles))
end

# Tests
test "config has sensible defaults"
  config = Config(base_url: "https://test.com")

  assert config.max_pages == 10
  assert config.timeout == 30.seconds
  assert config.retry_count == 3
end

test "scraping requires valid URL"
  config = Config(base_url: "https://test.com")

  raises ContractViolation
    config.scrape("invalid-url")
  end
end

property "parallel scraping returns same results as sequential"
  forall config: Config where config.max_pages <= 3
    seq_result = scrape_site(config)
    par_result = scrape_site_parallel(config)

    seq_result.articles.to_set == par_result.articles.to_set
  end
end

end # module Example
```

---

## Appendix A: Operator Precedence (Highest to Lowest)

| Level | Operators | Associativity |
|-------|-----------|---------------|
| 1 | `()` `[]` `.` `?.` `::` | Left |
| 2 | `?` `!` (postfix) | Left |
| 3 | `-` `+` `~` `&` `*` `not` `!` (prefix) | Right |
| 4 | `**` | Right |
| 5 | `*` `/` `//` `%` | Left |
| 6 | `+` `-` | Left |
| 7 | `<<` `>>` | Left |
| 8 | `&` | Left |
| 9 | `^` | Left |
| 10 | `\|` | Left |
| 11 | `..` `..<` `..=` | None |
| 12 | `==` `!=` `<` `>` `<=` `>=` `<=>` `in` `is` | Left |
| 13 | `and` `&&` | Left |
| 14 | `or` `\|\|` | Left |
| 15 | `? :` (ternary) | Right |
| 16 | `\|>` | Left |
| 17 | `=` `+=` `-=` etc. | Right |

---

## Appendix B: Reserved for Future Use

```
async       actor       macro       quote       unquote
yield       generator   lazy        effect      handle
resume      signal      atomic      volatile    packed
union       bitfield    asm         link        align
```

---

## Appendix C: Standard Library Prelude

The following are automatically imported into every Aria program:

```ruby
# Types
Int, Int8, Int16, Int32, Int64, Int128
UInt, UInt8, UInt16, UInt32, UInt64, UInt128
Float, Float32, Float64
Bool, Char, String, Bytes
Array, Map, Set, Tuple
Option, Result, Never, Unit

# Traits
Eq, Ord, Hash, Clone, Copy, Default, Debug, Display
Add, Sub, Mul, Div, Rem, Neg
Iterator, IntoIterator, FromIterator
Into, From, TryInto, TryFrom

# Functions
print, println, debug, panic
assert, assert_eq, assert_ne

# Macros (if added)
todo!, unimplemented!, unreachable!
```

---

## 15. MCP Integration (M21)

### 15.1 MCP Tool Definitions

```ebnf
mcp_tool_def    = 'tool' identifier 'from' 'mcp' '(' string_literal ')' '{' mcp_tool_body '}' ;
mcp_tool_body   = { mcp_tool_field } ;
mcp_tool_field  = 'permission' ':' string_literal [ ',' ]
                | 'capabilities' ':' '[' identifier_list ']' [ ',' ]
                | 'timeout' ':' number_literal [ ',' ] ;
identifier_list = identifier { ',' identifier } ;
```

**Example:**
```aria
tool code_search from mcp("github-server") {
    permission: "mcp.connect",
    capabilities: [search_code, search_issues],
    timeout: 15
}
```

---

## 16. Multi-Agent Orchestration (M22)

### 16.1 Pipeline (Sequential)

```ebnf
pipeline_def    = 'pipeline' identifier '{' { stage_def } '}' ;
stage_def       = 'stage' identifier '->' expression ;
```

**Example:**
```aria
pipeline ReviewPipeline {
    stage Analyst -> analyze($input)
    stage Reviewer -> review($prev)
    stage Summarizer -> summarize($prev)
}
```

### 16.2 Concurrent (Fan-Out/Merge)

```ebnf
concurrent_def  = 'concurrent' identifier '{' { concurrent_branch } [ merge_clause ] '}' ;
concurrent_branch = 'agent' identifier '->' expression ;
merge_clause    = 'merge' expression ;
```

**Example:**
```aria
concurrent ResearchTask {
    agent WebSearcher -> search_web($query)
    agent CodeSearcher -> search_codebase($query)
    merge combine_results($results)
}
```

### 16.3 Handoff (Routing)

```ebnf
handoff_def     = 'handoff' identifier '{' agent_classifier { route_def } '}' ;
agent_classifier = 'agent' identifier '->' expression ;
route_def       = 'route' route_pattern '=>' identifier ;
route_pattern   = string_literal | '_' ;
```

**Example:**
```aria
handoff SupportFlow {
    agent Triage -> classify($input)
    route "billing" => BillingAgent
    route "technical" => TechAgent
    route _ => HumanEscalation
}
```

---

## 17. A2A Protocol (M23)

### 17.1 Agent Card Definitions

```ebnf
a2a_def         = 'a2a' identifier '{' { a2a_field } '}' ;
a2a_field       = 'discovery' ':' string_literal [ ',' ]
                | 'skills' ':' '[' identifier_list ']' [ ',' ]
                | 'endpoint' ':' string_literal [ ',' ] ;
```

**Example:**
```aria
a2a ResearchCard {
    discovery: "/.well-known/agent.json"
    skills: [search, analyze, summarize]
    endpoint: "https://agents.aria.dev/research"
}
```

---

## 18. Workflow State Machines (M24)

### 18.1 Workflow Definitions

```ebnf
workflow_def    = 'workflow' identifier '{' { state_def } '}' ;
state_def       = 'state' identifier '{' { state_body_item } '}' ;
state_body_item = transition_def | requires_clause | ensures_clause | statement ;
transition_def  = 'on' identifier '->' identifier ;
requires_clause = 'requires' expression ;
ensures_clause  = 'ensures' expression ;
```

**Example:**
```aria
workflow OrderProcessing {
    state pending {
        on receive_order -> validating
    }
    state validating {
        requires $order_valid
        on valid -> processing
        on invalid -> rejected
    }
    state processing {
        on complete -> shipped
    }
    state shipped {
        ensures $tracking_exists
    }
    state rejected {
    }
}
```

---

## 19. Model Declarations (M25)

### 19.1 Model Definitions

```ebnf
model_def       = 'model' identifier '{' { model_field } '}' ;
model_field     = 'capability' ':' string_literal [ ',' ]
                | 'provider' ':' string_literal [ ',' ]
                | 'supports' ':' '[' identifier_list ']' [ ',' ] ;
```

**Example:**
```aria
model assistant {
    capability: "chat_completion"
    provider: "openai"
    supports: [tool_calling, structured_output, vision]
}
```

---

## 20. Agent Memory (M26)

### 20.1 Memory Definitions

```ebnf
memory_def      = 'memory' identifier '{' { memory_field } '}' ;
memory_field    = 'store' ':' string_literal [ ',' ]
                | 'embedding' ':' string_literal [ ',' ]
                | 'operations' ':' '[' identifier_list ']' [ ',' ] ;
```

**Example:**
```aria
memory ProjectKnowledge {
    store: "chromadb://localhost:8000/project"
    embedding: "text_embedder"
    operations: [remember, recall, forget]
}
```

---

**End of Grammar Specification**
