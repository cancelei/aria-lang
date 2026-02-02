# ARIA-M09-02: libclang for C Parsing Study

**Task ID**: ARIA-M09-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Evaluate libclang for header parsing

---

## Executive Summary

libclang is the stable C API for the Clang compiler, providing AST traversal and type information extraction. This research evaluates its capabilities for Aria's C header import system.

---

## 1. libclang Overview

### 1.1 What is libclang?

libclang is Clang's **stable C interface** for:
- Parsing C/C++/Objective-C code
- Traversing the Abstract Syntax Tree (AST)
- Extracting type information
- Source location mapping

### 1.2 Design Philosophy

> "This C interface to Clang will never provide all of the information representation stored in Clang's C++ AST, nor should it: the intent is to maintain an API that is relatively stable from one release to the next."

**Trade-off**: Stability over completeness.

---

## 2. Core Concepts

### 2.1 Translation Units

```c
CXIndex index = clang_createIndex(0, 0);
CXTranslationUnit tu = clang_parseTranslationUnit(
    index,
    "header.h",
    args, num_args,  // Compiler flags
    NULL, 0,         // Unsaved files
    CXTranslationUnit_None
);
```

A translation unit represents a parsed source file with all includes.

### 2.2 Cursors

Cursors are pointers to AST nodes:

```c
CXCursor cursor = clang_getTranslationUnitCursor(tu);
// cursor now points to root of AST
```

### 2.3 Cursor Kinds

| Kind | Description |
|------|-------------|
| `CXCursor_StructDecl` | Struct definition |
| `CXCursor_FieldDecl` | Struct field |
| `CXCursor_FunctionDecl` | Function declaration |
| `CXCursor_ParmDecl` | Function parameter |
| `CXCursor_TypedefDecl` | Typedef |
| `CXCursor_EnumDecl` | Enum definition |
| `CXCursor_EnumConstantDecl` | Enum value |
| `CXCursor_MacroDefinition` | Macro |

---

## 3. AST Traversal

### 3.1 Visitor Pattern

```c
CXChildVisitResult visitor(
    CXCursor cursor,
    CXCursor parent,
    CXClientData client_data
) {
    CXCursorKind kind = clang_getCursorKind(cursor);
    CXString name = clang_getCursorSpelling(cursor);

    printf("Found: %s (%s)\n",
           clang_getCString(name),
           clang_getCString(clang_getCursorKindSpelling(kind)));

    clang_disposeString(name);

    return CXChildVisit_Recurse;  // Continue traversal
}

// Start traversal
clang_visitChildren(root_cursor, visitor, NULL);
```

### 3.2 Visit Return Values

| Value | Meaning |
|-------|---------|
| `CXChildVisit_Break` | Stop traversal |
| `CXChildVisit_Continue` | Skip children, continue siblings |
| `CXChildVisit_Recurse` | Visit children recursively |

### 3.3 Example: Extracting Function Declarations

```c
CXChildVisitResult find_functions(CXCursor cursor, CXCursor parent, void* data) {
    if (clang_getCursorKind(cursor) == CXCursor_FunctionDecl) {
        CXString name = clang_getCursorSpelling(cursor);
        CXType return_type = clang_getCursorResultType(cursor);
        int num_args = clang_Cursor_getNumArguments(cursor);

        printf("Function: %s, returns: %s, args: %d\n",
               clang_getCString(name),
               clang_getCString(clang_getTypeSpelling(return_type)),
               num_args);

        clang_disposeString(name);
    }
    return CXChildVisit_Recurse;
}
```

---

## 4. Type Information Extraction

### 4.1 CXType

```c
CXType type = clang_getCursorType(cursor);
CXTypeKind kind = type.kind;
CXString type_name = clang_getTypeSpelling(type);
```

### 4.2 Type Kinds

| Kind | C Type | Example |
|------|--------|---------|
| `CXType_Int` | int | `int x` |
| `CXType_Pointer` | T* | `int *p` |
| `CXType_ConstantArray` | T[N] | `int arr[10]` |
| `CXType_Record` | struct/union | `struct Point` |
| `CXType_Enum` | enum | `enum Color` |
| `CXType_Typedef` | typedef | `typedef int MyInt` |
| `CXType_FunctionProto` | fn type | `int (*)(int)` |

### 4.3 Type Properties

```c
// Get pointed-to type
CXType pointee = clang_getPointeeType(pointer_type);

// Get array element type
CXType element = clang_getArrayElementType(array_type);

// Get array size
long long size = clang_getArraySize(array_type);

// Get struct fields
int num_fields = clang_Type_getNumFields(struct_type);

// Check qualifiers
unsigned is_const = clang_isConstQualifiedType(type);
```

---

## 5. Performance Characteristics

### 5.1 Parsing Speed

| File Size | Parse Time | Memory |
|-----------|------------|--------|
| Small header (1KB) | ~10ms | ~5MB |
| Medium header (100KB) | ~100ms | ~50MB |
| Large header (1MB) | ~1s | ~200MB |
| System headers (all) | ~5s | ~500MB |

### 5.2 Caching Strategies

```c
// Precompiled headers for faster reparsing
CXTranslationUnit tu = clang_parseTranslationUnit(
    index, "header.h", args, num_args,
    NULL, 0,
    CXTranslationUnit_PrecompiledPreamble
);

// Reparse with cached preamble
clang_reparseTranslationUnit(tu, 0, NULL, 0);
```

### 5.3 Memory Management

```c
// Always dispose resources
clang_disposeString(string);
clang_disposeTranslationUnit(tu);
clang_disposeIndex(index);
```

---

## 6. Integration Patterns

### 6.1 Python Bindings (libclang)

```python
import clang.cindex as ci

index = ci.Index.create()
tu = index.parse("header.h")

for cursor in tu.cursor.walk_preorder():
    if cursor.kind == ci.CursorKind.FUNCTION_DECL:
        print(f"Function: {cursor.spelling}")
        print(f"  Return: {cursor.result_type.spelling}")
        for arg in cursor.get_arguments():
            print(f"  Arg: {arg.spelling}: {arg.type.spelling}")
```

### 6.2 Rust Bindings (clang-sys)

```rust
use clang::*;

let clang = Clang::new().unwrap();
let index = Index::new(&clang, false, false);
let tu = index.parser("header.h").parse().unwrap();

for entity in tu.get_entity().get_children() {
    if entity.get_kind() == EntityKind::FunctionDecl {
        println!("Function: {}", entity.get_name().unwrap());
    }
}
```

---

## 7. Limitations

### 7.1 What libclang Doesn't Provide

| Feature | Status |
|---------|--------|
| Template instantiation details | Limited |
| Some C++ features | Incomplete |
| Preprocessor state | Partial |
| Macro expansion details | Limited |
| Cross-TU analysis | Not supported |

### 7.2 Macro Handling Limitations

```c
// libclang sees:
#define MAX(a, b) ((a) > (b) ? (a) : (b))

// Can get: name, definition text
// Cannot get: fully parsed macro body as AST
```

### 7.3 Alternative: Clang C++ API

For more control, use Clang's C++ API directly:
- Full AST access
- Template handling
- But: unstable between versions

---

## 8. Recommendations for Aria

### 8.1 Integration Architecture

```
┌─────────────────────────────────────────────────┐
│               Aria Compiler                      │
├─────────────────────────────────────────────────┤
│  extern C from "header.h"                       │
│       │                                          │
│       ▼                                          │
│  ┌─────────────────────────────────────────┐    │
│  │         C Import Module                  │    │
│  │  ┌─────────────┐   ┌─────────────────┐  │    │
│  │  │  libclang   │──►│ Type Translator │  │    │
│  │  │  Parser     │   └────────┬────────┘  │    │
│  │  └─────────────┘            │           │    │
│  │                             ▼           │    │
│  │                   ┌─────────────────┐   │    │
│  │                   │ Aria Type Decls │   │    │
│  │                   └─────────────────┘   │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### 8.2 Type Mapping

```aria
# C → Aria type mapping
CTypeMapping = {
  "int"           => CInt,
  "long"          => CLong,
  "char"          => CChar,
  "float"         => CFloat,
  "double"        => CDouble,
  "void"          => CVoid,
  "T*"            => CPtr[T],
  "const T*"      => CPtr[T].const,
  "struct S"      => extern_struct S,
  "enum E"        => extern_enum E,
  "T[N]"          => CArray[T, N],
  "T (*)(Args)"   => CFn[Args, T],
}
```

### 8.3 Safe Wrapper Generation

```aria
# From C:
# int sqlite3_open(const char *filename, sqlite3 **db);

# Generated Aria wrapper:
extern fn _sqlite3_open(filename: CString, db: CPtr[CPtr[sqlite3]]) -> CInt

fn sqlite3_open(filename: String) -> Result[SqliteDb, SqliteError]
  db_ptr: CPtr[sqlite3] = null
  result = _sqlite3_open(filename.to_c_string, &db_ptr)

  if result == SQLITE_OK
    Ok(SqliteDb.new(handle: db_ptr))
  else
    Err(SqliteError.from_code(result))
  end
end
```

### 8.4 Caching Strategy

```aria
# Cache parsed headers
module CHeaderCache
  @cache: Map[String, ParsedHeader] = {}

  fn get_or_parse(path: String) -> ParsedHeader
    cache_key = "#{path}:#{File.mtime(path)}"

    @cache.get(cache_key) or begin
      result = parse_with_libclang(path)
      @cache[cache_key] = result
      result
    end
  end
end
```

---

## 9. Key Resources

1. [libclang Documentation](https://clang.llvm.org/docs/LibClang.html)
2. [libclang C API Reference](https://clang.llvm.org/doxygen/group__CINDEX.html)
3. [Baby Steps with libclang](https://bastian.rieck.me/blog/2015/baby_steps_libclang_ast/)
4. [Using libclang to Parse C++](https://shaharmike.com/cpp/libclang/)

---

## 10. Open Questions

1. Should Aria bundle libclang or require system installation?
2. How do we handle platform-specific headers?
3. What's the strategy for complex macros?
4. How do we expose C functions with safe Aria semantics?
