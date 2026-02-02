# Aria Standard Library

Pure Aria implementation of the standard library for the Aria programming language.

## Overview

The Aria standard library provides essential data structures, I/O operations, and utility functions for Aria programs, with special focus on supporting BioFlow bioinformatics workflows.

## Structure

```
stdlib/
├── core/               # Core types and functionality
│   ├── mod.aria       # Core module index
│   ├── string.aria    # String operations
│   ├── array.aria     # Array functional methods
│   ├── option.aria    # Option<T> type
│   └── result.aria    # Result<T, E> type
├── collections/       # Data structures
│   ├── mod.aria      # Collections module index
│   └── hashmap.aria  # HashMap<K, V> implementation
├── io/               # Input/output operations
│   └── mod.aria     # I/O functions and file handling
├── prelude.aria     # Auto-imported items
└── mod.aria        # Main stdlib index
```

## Modules

### Core (`stdlib/core/`)

Core functionality and fundamental types:

- **Option<T>** - Optional values that may or may not exist
- **Result<T, E>** - Error handling for operations that may fail
- **String** - String manipulation and operations
- **Array** - Functional array methods (map, filter, fold, etc.)
- **Traits** - Default, Clone, PartialEq, Ord, Display, Debug

### Collections (`stdlib/collections/`)

Data structures for organizing data:

- **HashMap<K, V>** - Hash map with key-value pairs
- **HashSet<T>** - Hash set for unique values
- **LinkedList<T>** - Singly-linked list
- **TreeNode<T>** - Binary tree nodes

### I/O (`stdlib/io/`)

Input/output operations:

- **print/println** - Console output
- **read_line** - Console input
- **read_file/write_file** - File I/O
- **File** - File handles for buffered I/O
- **BufReader/BufWriter** - Buffered I/O for efficiency

## Usage

### Prelude

The prelude is automatically imported into every Aria program. It includes commonly used types and functions:

```aria
# These are automatically available
fn main()
  let x: Option<Int> = Some(42)
  let result: Result<String, String> = Ok("success")

  println("Hello, Aria!")
end
```

### Explicit Imports

For other stdlib modules, use explicit imports:

```aria
import std::collections::HashMap
import std::io::File

fn main()
  let mut map = HashMap::new()
  map.insert("key", "value")

  match File::open("data.txt")
    Ok(file) ->
      match file.read_to_string()
        Ok(content) -> println(content)
        Err(e) -> eprintln("Error: #{e.message()}")
      end
    Err(e) -> eprintln("Failed to open file")
  end
end
```

## Examples

### String Operations

```aria
import std::string::*

fn process_sequence(seq: String) -> String
  seq.to_uppercase()
     .trim()
     .replace("T", "U")
end

fn main()
  let dna = "  atcg  "
  let rna = process_sequence(dna)
  println(rna)  # "AUCG"
end
```

### Array Operations

```aria
import std::array::*

fn main()
  let numbers = [1, 2, 3, 4, 5]

  # Map: transform each element
  let doubled = numbers.map(fn(x) -> x * 2)

  # Filter: keep only matching elements
  let evens = numbers.filter(fn(x) -> x % 2 == 0)

  # Fold: reduce to single value
  let sum = numbers.fold(0, fn(acc, x) -> acc + x)

  println("Sum: #{sum}")
end
```

### HashMap

```aria
import std::collections::HashMap

fn main()
  let mut counts = HashMap::new()

  counts.insert("A", 10)
  counts.insert("C", 15)
  counts.insert("G", 12)
  counts.insert("T", 13)

  match counts.get("A")
    Some(count) -> println("A count: #{count}")
    None -> println("A not found")
  end
end
```

### File I/O

```aria
import std::io::*

fn read_sequences(path: String) -> Result<[String], IoError>
  let content = read_file(path)?
  let lines = content.split("\n")
  Ok(lines.filter(fn(line) -> !line.is_empty()))
end

fn main()
  match read_sequences("sequences.txt")
    Ok(seqs) ->
      println("Read #{seqs.len()} sequences")
      for seq in seqs
        println(seq)
      end
    Err(e) ->
      eprintln("Error: #{e.message()}")
  end
end
```

### Error Handling

```aria
fn parse_quality_score(s: String) -> Result<Int, String>
  match s.parse_int()
    Ok(n) ->
      if n >= 0 && n <= 40
        Ok(n)
      else
        Err("Quality score out of range")
      end
    Err(_) -> Err("Invalid integer")
  end
end

fn main()
  match parse_quality_score("30")
    Ok(score) -> println("Score: #{score}")
    Err(msg) -> eprintln("Error: #{msg}")
  end
end
```

## BioFlow Integration

The stdlib is designed to work seamlessly with BioFlow bioinformatics programs:

```aria
import std::collections::HashMap
import std::io::read_file

fn count_kmers(sequence: String, k: Int) -> HashMap<String, Int>
  let mut counts = HashMap::new()
  let mut i = 0

  while i <= sequence.len() - k
    let kmer = sequence.slice(i, i + k)
    match counts.get(kmer)
      Some(count) -> counts.insert(kmer, count + 1)
      None -> counts.insert(kmer, 1)
    end
    i = i + 1
  end

  counts
end
```

## Implementation Notes

- **Pure Aria**: All implementations are in pure Aria, with minimal native builtins
- **Zero-cost abstractions**: Functional methods compile to efficient code
- **Generic types**: Full support for type parameters (e.g., `Option<T>`, `HashMap<K, V>`)
- **Memory safe**: No manual memory management required
- **Iterator-friendly**: Designed to work with Aria's for-loop syntax

## Auto-Import Configuration

The prelude is automatically imported by the Aria compiler. To disable auto-import:

```aria
#![no_prelude]

# Now you must explicitly import everything
import std::option::Option
import std::io::println
```

## Future Enhancements

- Iterator trait and lazy evaluation
- More efficient hash function implementations
- Async I/O support
- Regular expressions
- Date/time types
- JSON parsing
- Network I/O
- Parallel iterators for BioFlow performance

## Contributing

Contributions to the stdlib are welcome! Please ensure:

1. Implementations are in pure Aria (minimize native builtins)
2. Code is well-documented with examples
3. Generic types are used where appropriate
4. Error handling uses Result<T, E>
5. Tests are included

## License

Part of the Aria programming language project.
