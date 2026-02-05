# Aria-Lang API Reference

**Complete reference for Aria's 24 builtin functions.**

## Quick Reference Table

| Category | Function | Arguments | Returns |
|----------|----------|-----------|---------|
| **Strings** | str_len | (s: string) | number |
| | str_concat | (s1: string, s2: string) | string |
| | str_upper | (s: string) | string |
| | str_lower | (s: string) | string |
| | str_trim | (s: string) | string |
| | str_contains | (haystack: string, needle: string) | number (1/0) |
| | str_replace | (s: string, old: string, new: string) | string |
| | str_split | (s: string, delim: string) | string (JSON array) |
| | str_starts_with | (s: string, prefix: string) | number (1/0) |
| | str_ends_with | (s: string, suffix: string) | number (1/0) |
| **Arrays** | arr_from_split | (s: string, delim: string) | string (JSON array) |
| | arr_len | (arr: string) | number |
| | arr_get | (arr: string, index: number) | string |
| | arr_join | (arr: string, delim: string) | string |
| | arr_push | (arr: string, item: string) | string (new array) |
| | arr_pop | (arr: string) | string (new array) |
| **JSON** | json_parse | (s: string) | string (validated) |
| | json_stringify | (v: any) | string |
| | json_get | (json: string, key: string) | string |
| **Files** | file_read | (path: string) | string |
| | file_write | (path: string, content: string) | null |
| | file_exists | (path: string) | number (1/0) |
| | file_append | (path: string, content: string) | null |

## String Operations (10 functions)

### str_len(s: string) -> number

Get the length of a string.

**Example:**
```aria
let $len = str_len("hello")  // 5
print $len
```

**Edge Cases:**
- Empty string returns 0
- Unicode characters count as single characters

---

### str_concat(s1: string, s2: string) -> string

Concatenate two strings.

**Example:**
```aria
let $msg = str_concat("Hello, ", "World")  // "Hello, World"
```

**Note:** For multiple concatenations, nest calls:
```aria
let $full = str_concat(str_concat($first, " "), $last)
```

---

### str_upper(s: string) -> string

Convert string to uppercase.

**Example:**
```aria
let $loud = str_upper("aria")  // "ARIA"
```

---

### str_lower(s: string) -> string

Convert string to lowercase.

**Example:**
```aria
let $quiet = str_lower("HELLO")  // "hello"
```

---

### str_trim(s: string) -> string

Remove whitespace from both ends.

**Example:**
```aria
let $clean = str_trim("  hello  ")  // "hello"
```

**Removes:** spaces, tabs, newlines from start and end.

---

### str_contains(haystack: string, needle: string) -> number

Check if string contains substring. Returns 1 if found, 0 otherwise.

**Example:**
```aria
let $has = str_contains("hello world", "world")  // 1
let $not = str_contains("hello", "goodbye")      // 0
```

**Use in conditions:**
```aria
let $found = str_contains($text, "error")
// Check if $found is 1 or 0
```

---

### str_replace(s: string, old: string, new: string) -> string

Replace all occurrences of `old` with `new`.

**Example:**
```aria
let $fixed = str_replace("hello world", "world", "aria")
// "hello aria"
```

**Note:** Replaces ALL occurrences, not just the first.

---

### str_split(s: string, delim: string) -> string

Split string by delimiter, returns JSON array string.

**Example:**
```aria
let $parts = str_split("a,b,c", ",")  // ["a","b","c"]
```

**Equivalent to:** arr_from_split (use either)

---

### str_starts_with(s: string, prefix: string) -> number

Check if string starts with prefix. Returns 1 if true, 0 otherwise.

**Example:**
```aria
let $is_greeting = str_starts_with("hello world", "hello")  // 1
```

---

### str_ends_with(s: string, suffix: string) -> number

Check if string ends with suffix. Returns 1 if true, 0 otherwise.

**Example:**
```aria
let $is_log = str_ends_with("debug.log", ".log")  // 1
```

---

## Array Operations (6 functions)

**Important:** Arrays are represented as JSON strings internally (e.g., `["a","b","c"]`). Native array syntax coming in future versions.

### arr_from_split(s: string, delim: string) -> string

Create array by splitting string.

**Example:**
```aria
let $arr = arr_from_split("apple,banana,cherry", ",")
// ["apple","banana","cherry"]
```

---

### arr_len(arr: string) -> number

Get array length.

**Example:**
```aria
let $arr = arr_from_split("a,b,c", ",")
let $count = arr_len($arr)  // 3
```

---

### arr_get(arr: string, index: number) -> string

Get element by index (0-based).

**Example:**
```aria
let $arr = arr_from_split("a,b,c", ",")
let $first = arr_get($arr, 0)   // "a"
let $second = arr_get($arr, 1)  // "b"
```

**Error:** Index out of bounds throws runtime error.

---

### arr_join(arr: string, delim: string) -> string

Join array elements into string.

**Example:**
```aria
let $arr = arr_from_split("a,b,c", ",")
let $str = arr_join($arr, "-")  // "a-b-c"
```

---

### arr_push(arr: string, item: string) -> string

Add element to end (returns new array).

**Example:**
```aria
let $arr = arr_from_split("a,b", ",")
let $new = arr_push($arr, "c")  // ["a","b","c"]
```

**Note:** Arrays are immutable. Returns new array, original unchanged.

---

### arr_pop(arr: string) -> string

Remove last element (returns new array).

**Example:**
```aria
let $arr = arr_from_split("a,b,c", ",")
let $shorter = arr_pop($arr)  // ["a","b"]
```

**Error:** Popping empty array throws runtime error.

---

## JSON Operations (3 functions)

### json_parse(s: string) -> string

Parse and validate JSON string.

**Example:**
```aria
let $json = json_parse('{"name":"aria","version":1}')
// Returns validated JSON string
```

**Error:** Invalid JSON throws parse error with details.

---

### json_stringify(v: any) -> string

Convert value to JSON string.

**Example:**
```aria
let $json = json_stringify("hello")  // "\"hello\""
let $num = json_stringify(42)         // "42.0"
```

---

### json_get(json: string, key: string) -> string

Extract field from JSON object.

**Example:**
```aria
let $json = '{"name":"aria","version":1}'
let $name = json_get($json, "name")  // "\"aria\""
```

**Note:** Returns JSON-encoded value (including quotes for strings).

---

## File Operations (4 functions)

**Security:** File operations execute in sandboxed child processes with timeout enforcement.

### file_read(path: string) -> string

Read file contents.

**Example:**
```aria
let $content = file_read("/tmp/data.txt")
print $content
```

**Timeout:** 30 seconds default
**Error:** File not found, permission denied, or timeout

---

### file_write(path: string, content: string) -> null

Write content to file (overwrites if exists).

**Example:**
```aria
file_write("/tmp/output.txt", "Hello, File!")
```

**Timeout:** 30 seconds default
**Note:** Creates file if doesn't exist

---

### file_exists(path: string) -> number

Check if file exists. Returns 1 if exists, 0 otherwise.

**Example:**
```aria
let $exists = file_exists("/tmp/test.txt")
```

**Timeout:** 5 seconds

---

### file_append(path: string, content: string) -> null

Append content to file.

**Example:**
```aria
file_append("/tmp/log.txt", "New log entry\n")
```

**Note:** Creates file if doesn't exist.

---

## Language Constructs

### Statements

#### let - Variable Assignment

```aria
let $name = "value"
let $number = 42
let $result = function_call()
```

#### print - Output

```aria
print "message"
print $variable
```

#### think - Reasoning Block

```aria
think { "Planning the next step" }
think { "This operation might take time" }
```

Thinking blocks are logged and traced.

#### gate - Human-in-the-Loop

```aria
gate "Approve this action?" {
    // Code executes only if human approves
    dangerous_operation()
}
```

Runtime pauses until signal received.

---

### Tool Definitions

```aria
tool tool_name(param1: string, param2: number) {
    permission: "category.action",
    timeout: 30
}
```

**Parameters:**
- `permission` (optional): Permission category
- `timeout` (optional): Timeout in seconds (default: 30)

**Usage:**
```aria
let $result = tool_name("arg1", 42)
```

---

### Agent Definitions

```aria
agent AgentName {
    allow tool1
    allow tool2
    
    task task_name(param: string) {
        // Task body
        let $result = tool1(param)
        return $result
    }
}
```

**Spawning:**
```aria
let $agent = spawn AgentName
```

**Delegating:**
```aria
delegate agent.task_name("argument")
```

---

## Type System

Aria currently uses dynamic typing. All values are one of:

- **string** - Text values (UTF-8)
- **number** - Floating point numbers (f64)
- **null** - Absent value
- **agent** - Agent instance reference

**Type Inference:** Coming in future versions.

---

## Error Handling

### Common Errors

**Syntax Error:**
```
[Syntax Error] Unexpected token: ...
```
Check your syntax against examples.

**Runtime Error:**
```
[Runtime Error] Tool 'xyz' is not defined
```
Ensure tool is defined before use.

**Permission Denied:**
```
[Permission Denied] Agent '$bot' attempted to call tool 'write_file'
but it is not in the allow list. Allowed tools: ["read_file"]
```
Add tool to agent's `allow` list.

**Timeout:**
```
[Timeout] Tool 'slow_task' exceeded timeout of 5.0s
```
Increase timeout or optimize operation.

---

## Performance Notes

- **Builtins:** Native Rust speed (nanoseconds)
- **Tools:** Process spawn overhead (~1-5ms)
- **Arrays:** JSON serialization overhead (acceptable for v1.0)
- **Files:** I/O bound (depends on system)

---

## Migration Guide (Future)

When native arrays are added in v2.0:

**Current (v1.0):**
```aria
let $arr = arr_from_split("a,b,c", ",")
let $first = arr_get($arr, 0)
```

**Future (v2.0):**
```aria
let $arr = ["a", "b", "c"]
let $first = $arr[0]
```

Your v1.0 code will continue to work.

---

## See Also

- [QUICKSTART.md](QUICKSTART.md) - Get started in 10 minutes
- [TUTORIAL.md](TUTORIAL.md) - Structured learning path
- [examples/](examples/) - Example programs
- [GitHub Issues](https://github.com/cancelei/aria-lang/issues) - Report bugs

---

**Have questions?** Check [TUTORIAL.md](TUTORIAL.md) for detailed explanations or visit the community at [moltbook.com/m/arialang](https://moltbook.com/m/arialang).
