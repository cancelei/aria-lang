# Bool Type Testing Status

## ✅ Working Cases

### 1. Bool with constant comparisons
```aria
let result = 10 < 20
println(result)  // prints: true
```

### 2. Bool with variable comparisons (no prior control flow)
```aria
let x = 10
let y = 20
let res = x < y
println(res)  // prints: true
```

### 3. Bool in if statements
```aria
let x = 10
let y = 20
if x < y
    println(999)  // This prints
else
    println(888)
end
```

## ❌ Known Bug

### Comparisons after if statements on same variables
```aria
let x = 10
let y = 20

if x < y
    println(999)  // Prints correctly
end

let res = x < y
println(res)  // BUG: prints false instead of true!
```

**Root Cause**: SSA/CFG issue where variable states after if statements cause subsequent comparisons to return incorrect values.

**Workaround**: Avoid reusing the same comparison after an if statement, or use fresh variables.

## Fixed Issues

### Lexer case-sensitivity bug
- **Problem**: `result` was being lexed as keyword `Result` instead of identifier
- **Fix**: Removed `#[token("result")]` from lexer, made `result` context-sensitive
- **Status**: ✅ FIXED
