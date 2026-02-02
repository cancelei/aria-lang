//! Property-Based Testing Generators
//!
//! Provides the Generator type and Arbitrary trait for generating random
//! test values as specified in ARIA-PD-011. This module includes:
//!
//! - `Generator<T>`: A monadic type for generating values
//! - `Arbitrary`: Trait for types that can be randomly generated
//! - `Gen`: Module with generator combinators
//!
//! # Architecture
//!
//! Generators are deterministic functions from (Seed, Size) -> T, allowing
//! for reproducible test failures by reusing the same seed.


// ============================================================================
// Seed
// ============================================================================

/// A splittable random seed for deterministic generation
///
/// Seeds can be split into independent sub-seeds, allowing for
/// compositional generation of complex values.
#[derive(Debug, Clone, Copy)]
pub struct Seed {
    state: u64,
}

impl Seed {
    /// Create a new seed from a u64 value
    pub fn new(value: u64) -> Self {
        Self { state: value }
    }

    /// Create a random seed from system entropy
    pub fn random() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self::new(nanos as u64)
    }

    /// Get the current state
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Advance to the next seed
    pub fn next(&self) -> Self {
        // Simple LCG-style advancement
        Self {
            state: self.state.wrapping_mul(6364136223846793005).wrapping_add(1),
        }
    }

    /// Split into two independent seeds
    pub fn split(&self) -> (Self, Self) {
        let s1 = self.next();
        let s2 = s1.next();
        (
            Self { state: s1.state },
            Self {
                state: s2.state ^ 0xDEADBEEF,
            },
        )
    }

    /// Split into three independent seeds
    pub fn split3(&self) -> (Self, Self, Self) {
        let (s1, rest) = self.split();
        let (s2, s3) = rest.split();
        (s1, s2, s3)
    }

    /// Generate a random integer in range [min, max]
    pub fn next_int(&self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let range = (max - min + 1) as u64;
        let value = self.state % range;
        min + value as i64
    }

    /// Generate a random usize in range [min, max]
    pub fn next_usize(&self, min: usize, max: usize) -> usize {
        if min >= max {
            return min;
        }
        let range = max - min + 1;
        let value = self.state as usize % range;
        min + value
    }

    /// Generate a random float in range [min, max]
    pub fn next_float(&self, min: f64, max: f64) -> f64 {
        if min >= max {
            return min;
        }
        let normalized = (self.state as f64) / (u64::MAX as f64);
        min + normalized * (max - min)
    }

    /// Generate a random boolean
    pub fn next_bool(&self) -> bool {
        self.state % 2 == 0
    }
}

impl Default for Seed {
    fn default() -> Self {
        Self::random()
    }
}

// ============================================================================
// Size
// ============================================================================

/// Size parameter for controlling generated value complexity
///
/// Generators use this to scale the size of generated values. Larger
/// sizes produce more complex values (longer strings, bigger numbers, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size(pub usize);

impl Size {
    /// Create a new size
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    /// Get the size value
    pub fn value(&self) -> usize {
        self.0
    }

    /// Convert to f64
    pub fn to_float(&self) -> f64 {
        self.0 as f64
    }

    /// Scale the size by a factor
    pub fn scale(&self, factor: f64) -> Self {
        Self((self.0 as f64 * factor) as usize)
    }
}

impl Default for Size {
    fn default() -> Self {
        Self(100)
    }
}

impl From<usize> for Size {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

// ============================================================================
// Generator
// ============================================================================

/// A generator for producing random test values
///
/// Generators are deterministic functions that produce values based on
/// a seed and size parameter. They support monadic composition through
/// `map` and `flat_map`.
///
/// # Example
///
/// ```rust
/// use aria_test::generator::{Generator, Seed, Size};
///
/// let int_gen = Generator::new(|seed, size| {
///     seed.next_int(0, size.value() as i64) as i32
/// });
///
/// let value = int_gen.generate(Seed::new(42), Size::new(100));
/// ```
pub struct Generator<T> {
    /// The generation function
    gen_fn: Box<dyn Fn(Seed, Size) -> T + Send + Sync>,
}

impl<T> Generator<T> {
    /// Create a new generator from a function
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Seed, Size) -> T + Send + Sync + 'static,
    {
        Self { gen_fn: Box::new(f) }
    }

    /// Generate a value using this generator
    pub fn generate(&self, seed: Seed, size: Size) -> T {
        (self.gen_fn)(seed, size)
    }

    /// Transform generated values with a function
    pub fn map<U, F>(self, f: F) -> Generator<U>
    where
        F: Fn(T) -> U + Send + Sync + 'static,
        T: 'static,
    {
        Generator::new(move |seed, size| f(self.generate(seed, size)))
    }

    /// Chain generators (monadic bind)
    pub fn flat_map<U, F>(self, f: F) -> Generator<U>
    where
        F: Fn(T) -> Generator<U> + Send + Sync + 'static,
        T: 'static,
    {
        Generator::new(move |seed, size| {
            let (seed1, seed2) = seed.split();
            let intermediate = self.generate(seed1, size);
            f(intermediate).generate(seed2, size)
        })
    }

    /// Resize the generator to use a fixed size
    pub fn resize(self, new_size: Size) -> Generator<T>
    where
        T: 'static,
    {
        Generator::new(move |seed, _| self.generate(seed, new_size))
    }

    /// Scale the size parameter
    pub fn scale<F>(self, scale_fn: F) -> Generator<T>
    where
        F: Fn(Size) -> Size + Send + Sync + 'static,
        T: 'static,
    {
        Generator::new(move |seed, size| self.generate(seed, scale_fn(size)))
    }
}

impl<T: Clone + Send + Sync + 'static> Generator<T> {
    /// Filter generated values (use sparingly - can be slow)
    pub fn filter<P>(self, predicate: P, max_attempts: usize) -> Generator<T>
    where
        P: Fn(&T) -> bool + Send + Sync + 'static,
    {
        Generator::new(move |seed, size| {
            let mut current_seed = seed;
            for _ in 0..max_attempts {
                let candidate = self.generate(current_seed, size);
                if predicate(&candidate) {
                    return candidate;
                }
                current_seed = current_seed.next();
            }
            // Return last attempt if we couldn't satisfy predicate
            self.generate(current_seed, size)
        })
    }
}

// ============================================================================
// Arbitrary Trait
// ============================================================================

/// Trait for types that can be randomly generated
///
/// Implement this trait to enable property-based testing for your types.
/// Types implementing Arbitrary should also implement shrinking for
/// better counterexample minimization.
pub trait Arbitrary: Sized {
    /// Create a generator for this type
    fn arbitrary() -> Generator<Self>;

    /// Shrink a value to simpler versions
    ///
    /// Returns candidates that are "smaller" or "simpler" than the input.
    /// Used to minimize counterexamples when a property fails.
    fn shrink(value: Self) -> Vec<Self> {
        let _ = value;
        Vec::new()
    }
}

// ============================================================================
// Built-in Arbitrary Implementations
// ============================================================================

impl Arbitrary for bool {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, _| seed.next_bool())
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value {
            vec![false]
        } else {
            vec![]
        }
    }
}

impl Arbitrary for i32 {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| seed.next_int(-(size.0 as i64), size.0 as i64) as i32)
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value == 0 {
            return vec![];
        }

        let mut candidates = vec![0];
        if value > 0 {
            candidates.push(value / 2);
            if value > 1 {
                candidates.push(value - 1);
            }
        } else {
            candidates.push(value / 2);
            if value < -1 {
                candidates.push(value + 1);
            }
        }
        candidates
            .into_iter()
            .filter(|c| c.abs() < value.abs())
            .collect()
    }
}

impl Arbitrary for i64 {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| seed.next_int(-(size.0 as i64), size.0 as i64))
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value == 0 {
            return vec![];
        }

        let mut candidates = vec![0];
        if value > 0 {
            candidates.push(value / 2);
            if value > 1 {
                candidates.push(value - 1);
            }
        } else {
            candidates.push(value / 2);
            if value < -1 {
                candidates.push(value + 1);
            }
        }
        candidates
            .into_iter()
            .filter(|c| c.abs() < value.abs())
            .collect()
    }
}

impl Arbitrary for u32 {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| seed.next_usize(0, size.0) as u32)
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value == 0 {
            return vec![];
        }
        let mut candidates = vec![0];
        candidates.push(value / 2);
        if value > 1 {
            candidates.push(value - 1);
        }
        candidates.into_iter().filter(|c| *c < value).collect()
    }
}

impl Arbitrary for u64 {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| seed.next_usize(0, size.0) as u64)
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value == 0 {
            return vec![];
        }
        let mut candidates = vec![0];
        candidates.push(value / 2);
        if value > 1 {
            candidates.push(value - 1);
        }
        candidates.into_iter().filter(|c| *c < value).collect()
    }
}

impl Arbitrary for usize {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| seed.next_usize(0, size.0))
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value == 0 {
            return vec![];
        }
        let mut candidates = vec![0];
        candidates.push(value / 2);
        if value > 1 {
            candidates.push(value - 1);
        }
        candidates.into_iter().filter(|c| *c < value).collect()
    }
}

impl Arbitrary for f64 {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| seed.next_float(-(size.0 as f64), size.0 as f64))
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value == 0.0 {
            return vec![];
        }
        let mut candidates = vec![0.0, value / 2.0, value.trunc()];
        candidates.retain(|c| c.abs() < value.abs());
        candidates
    }
}

impl Arbitrary for char {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, _| {
            let code = seed.next_int(32, 126) as u32;
            char::from_u32(code).unwrap_or('?')
        })
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value == 'a' {
            return vec![];
        }
        vec!['a']
    }
}

impl Arbitrary for String {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| {
            let len = seed.next_usize(0, size.0.min(100));
            let mut result = String::with_capacity(len);
            let mut current_seed = seed.next();

            for _ in 0..len {
                let code = current_seed.next_int(32, 126) as u32;
                if let Some(c) = char::from_u32(code) {
                    result.push(c);
                }
                current_seed = current_seed.next();
            }
            result
        })
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value.is_empty() {
            return vec![];
        }

        let mut candidates = Vec::new();

        // Remove characters one at a time
        for i in 0..value.len() {
            let mut s = value.clone();
            s.remove(i);
            candidates.push(s);
        }

        // Simplify to shorter prefix
        if value.len() > 1 {
            candidates.push(value[..value.len() / 2].to_string());
        }

        candidates
    }
}

impl<T: Arbitrary + Clone + 'static> Arbitrary for Vec<T> {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| {
            let len = seed.next_usize(0, size.0.min(50));
            let mut result = Vec::with_capacity(len);
            let mut current_seed = seed.next();

            for _ in 0..len {
                let (s1, s2) = current_seed.split();
                result.push(T::arbitrary().generate(s1, size));
                current_seed = s2;
            }
            result
        })
    }

    fn shrink(value: Self) -> Vec<Self> {
        if value.is_empty() {
            return vec![];
        }

        let mut candidates = Vec::new();

        // Remove elements one at a time
        for i in 0..value.len() {
            let mut v = value.clone();
            v.remove(i);
            candidates.push(v);
        }

        // Shrink individual elements
        for (i, elem) in value.iter().enumerate() {
            for smaller in T::shrink(elem.clone()) {
                let mut v = value.clone();
                v[i] = smaller;
                candidates.push(v);
            }
        }

        candidates
    }
}

impl<T: Arbitrary + Clone + 'static> Arbitrary for Option<T> {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| {
            if seed.next_bool() {
                let (_, s2) = seed.split();
                Some(T::arbitrary().generate(s2, size))
            } else {
                None
            }
        })
    }

    fn shrink(value: Self) -> Vec<Self> {
        match value {
            None => vec![],
            Some(v) => {
                let mut candidates = vec![None];
                for smaller in T::shrink(v) {
                    candidates.push(Some(smaller));
                }
                candidates
            }
        }
    }
}

impl<A: Arbitrary + Clone + 'static, B: Arbitrary + Clone + 'static> Arbitrary for (A, B) {
    fn arbitrary() -> Generator<Self> {
        Generator::new(|seed, size| {
            let (s1, s2) = seed.split();
            (
                A::arbitrary().generate(s1, size),
                B::arbitrary().generate(s2, size),
            )
        })
    }

    fn shrink(value: Self) -> Vec<Self> {
        let (a, b) = value;
        let mut candidates = Vec::new();

        for smaller_a in A::shrink(a.clone()) {
            candidates.push((smaller_a, b.clone()));
        }
        for smaller_b in B::shrink(b.clone()) {
            candidates.push((a.clone(), smaller_b));
        }

        candidates
    }
}

// ============================================================================
// Gen Module - Generator Combinators
// ============================================================================

/// Generator combinators module
pub mod gen {
    use super::*;

    /// Create a generator that always produces the same value
    pub fn constant<T: Clone + Send + Sync + 'static>(value: T) -> Generator<T> {
        Generator::new(move |_, _| value.clone())
    }

    /// Choose randomly from a list of values
    pub fn elements<T: Clone + Send + Sync + 'static>(choices: Vec<T>) -> Generator<T> {
        assert!(!choices.is_empty(), "elements requires non-empty choices");
        Generator::new(move |seed, _| {
            let idx = seed.next_usize(0, choices.len() - 1);
            choices[idx].clone()
        })
    }

    /// Choose randomly from a list of generators
    pub fn one_of<T: 'static>(generators: Vec<Generator<T>>) -> Generator<T> {
        assert!(!generators.is_empty(), "one_of requires non-empty generators");
        Generator::new(move |seed, size| {
            let (s1, s2) = seed.split();
            let idx = s1.next_usize(0, generators.len() - 1);
            generators[idx].generate(s2, size)
        })
    }

    /// Choose with weighted probabilities
    pub fn frequency<T: 'static>(weighted: Vec<(usize, Generator<T>)>) -> Generator<T> {
        assert!(!weighted.is_empty(), "frequency requires non-empty list");
        let total: usize = weighted.iter().map(|(w, _)| w).sum();
        assert!(total > 0, "frequency requires positive total weight");

        Generator::new(move |seed, size| {
            let (s1, s2) = seed.split();
            let mut target = s1.next_usize(0, total - 1);

            for (weight, gen) in &weighted {
                if target < *weight {
                    return gen.generate(s2, size);
                }
                target -= weight;
            }
            // Fallback to last generator
            weighted.last().unwrap().1.generate(s2, size)
        })
    }

    /// Generate an integer in a specific range
    pub fn int_range(min: i64, max: i64) -> Generator<i64> {
        Generator::new(move |seed, _| seed.next_int(min, max))
    }

    /// Generate a positive integer
    pub fn positive_int() -> Generator<i64> {
        Generator::new(|seed, size| seed.next_int(1, size.0.max(1) as i64))
    }

    /// Generate a non-negative integer
    pub fn non_negative_int() -> Generator<i64> {
        Generator::new(|seed, size| seed.next_int(0, size.0 as i64))
    }

    /// Generate a float in a specific range
    pub fn float_range(min: f64, max: f64) -> Generator<f64> {
        Generator::new(move |seed, _| seed.next_float(min, max))
    }

    /// Generate a list of values
    pub fn list<T: 'static>(elem_gen: Generator<T>) -> Generator<Vec<T>> {
        Generator::new(move |seed, size| {
            let len = seed.next_usize(0, size.0.min(50));
            let mut result = Vec::with_capacity(len);
            let mut current_seed = seed.next();

            for _ in 0..len {
                let (s1, s2) = current_seed.split();
                result.push(elem_gen.generate(s1, size));
                current_seed = s2;
            }
            result
        })
    }

    /// Generate a list with a specific length range
    pub fn list_of_length<T: 'static>(
        elem_gen: Generator<T>,
        min_len: usize,
        max_len: usize,
    ) -> Generator<Vec<T>> {
        Generator::new(move |seed, size| {
            let len = seed.next_usize(min_len, max_len);
            let mut result = Vec::with_capacity(len);
            let mut current_seed = seed.next();

            for _ in 0..len {
                let (s1, s2) = current_seed.split();
                result.push(elem_gen.generate(s1, size));
                current_seed = s2;
            }
            result
        })
    }

    /// Generate a non-empty list
    pub fn non_empty_list<T: 'static>(elem_gen: Generator<T>) -> Generator<Vec<T>> {
        Generator::new(move |seed, size| {
            let len = seed.next_usize(1, size.0.max(1).min(50));
            let mut result = Vec::with_capacity(len);
            let mut current_seed = seed.next();

            for _ in 0..len {
                let (s1, s2) = current_seed.split();
                result.push(elem_gen.generate(s1, size));
                current_seed = s2;
            }
            result
        })
    }

    /// Generate a vector of fixed length
    pub fn vector<T: 'static>(n: usize, elem_gen: Generator<T>) -> Generator<Vec<T>> {
        list_of_length(elem_gen, n, n)
    }

    /// Generate a string from a character generator
    pub fn string(char_gen: Generator<char>, min_len: usize, max_len: usize) -> Generator<String> {
        Generator::new(move |seed, size| {
            let len = seed.next_usize(min_len, max_len.min(size.0));
            let mut result = String::with_capacity(len);
            let mut current_seed = seed.next();

            for _ in 0..len {
                let (s1, s2) = current_seed.split();
                result.push(char_gen.generate(s1, size));
                current_seed = s2;
            }
            result
        })
    }

    /// Generate an ASCII character
    pub fn ascii_char() -> Generator<char> {
        Generator::new(|seed, _| {
            let code = seed.next_int(32, 126) as u32;
            char::from_u32(code).unwrap_or('?')
        })
    }

    /// Generate an alphabetic character
    pub fn alpha_char() -> Generator<char> {
        elements(
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
                .chars()
                .collect(),
        )
    }

    /// Generate an alphanumeric character
    pub fn alphanumeric_char() -> Generator<char> {
        elements(
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
                .chars()
                .collect(),
        )
    }

    /// Generate an ASCII string
    pub fn ascii_string() -> Generator<String> {
        string(ascii_char(), 0, 100)
    }

    /// Generate an alphanumeric string
    pub fn alphanumeric_string() -> Generator<String> {
        string(alphanumeric_char(), 0, 100)
    }

    /// Generate an optional value
    pub fn option<T: 'static>(gen: Generator<T>) -> Generator<Option<T>> {
        Generator::new(move |seed, size| {
            if seed.next_bool() {
                let (_, s2) = seed.split();
                Some(gen.generate(s2, size))
            } else {
                None
            }
        })
    }

    /// Generate using the size parameter
    pub fn sized<T: 'static, F>(f: F) -> Generator<T>
    where
        F: Fn(Size) -> Generator<T> + Send + Sync + 'static,
    {
        Generator::new(move |seed, size| f(size).generate(seed, size))
    }
}

// ============================================================================
// Shrinking Infrastructure
// ============================================================================

/// Shrink a value to find minimal counterexamples
pub fn shrink_value<T, P>(value: T, predicate: P, max_shrinks: usize) -> T
where
    T: Arbitrary + Clone,
    P: Fn(&T) -> bool,
{
    let mut current = value;
    let mut shrink_count = 0;

    while shrink_count < max_shrinks {
        let candidates = T::shrink(current.clone());
        if candidates.is_empty() {
            break;
        }

        let mut improved = false;
        for candidate in candidates {
            if !predicate(&candidate) {
                current = candidate;
                improved = true;
                break;
            }
        }

        if !improved {
            break;
        }
        shrink_count += 1;
    }

    current
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_determinism() {
        let seed = Seed::new(12345);
        let a = seed.next_int(0, 100);
        let b = seed.next_int(0, 100);
        assert_eq!(a, b); // Same seed produces same value
    }

    #[test]
    fn test_seed_split() {
        let seed = Seed::new(12345);
        let (s1, s2) = seed.split();
        assert_ne!(s1.state(), s2.state());
    }

    #[test]
    fn test_generator_map() {
        let gen = Generator::new(|seed, _| seed.next_int(0, 10) as i32);
        let doubled = gen.map(|x| x * 2);

        let seed = Seed::new(42);
        let value = doubled.generate(seed, Size::new(100));
        assert!(value % 2 == 0);
    }

    #[test]
    fn test_arbitrary_bool() {
        let gen = bool::arbitrary();
        let seed = Seed::new(42);

        // Just verify it generates booleans
        let _value = gen.generate(seed, Size::new(100));
    }

    #[test]
    fn test_arbitrary_int_shrink() {
        let shrunk = i32::shrink(10);
        assert!(shrunk.contains(&0));
        assert!(shrunk.contains(&5)); // 10 / 2
        assert!(shrunk.contains(&9)); // 10 - 1
    }

    #[test]
    fn test_arbitrary_vec_shrink() {
        let vec = vec![1, 2, 3];
        let shrunk = Vec::<i32>::shrink(vec);

        // Should include versions with elements removed
        assert!(shrunk.iter().any(|v| v.len() == 2));
    }

    #[test]
    fn test_gen_elements() {
        let gen = gen::elements(vec![1, 2, 3, 4, 5]);
        let seed = Seed::new(42);
        let value = gen.generate(seed, Size::new(100));
        assert!((1..=5).contains(&value));
    }

    #[test]
    fn test_gen_list() {
        let gen = gen::list(i32::arbitrary());
        let seed = Seed::new(42);
        let value = gen.generate(seed, Size::new(10));
        assert!(value.len() <= 10);
    }

    #[test]
    fn test_shrink_value() {
        // Property: value should be less than 5
        let predicate = |x: &i32| *x < 5;

        // Start with a value that violates the property
        let result = shrink_value(100, predicate, 1000);

        // Should shrink to 5 (minimal counterexample)
        assert_eq!(result, 5);
    }
}
