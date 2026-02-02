//! Value generators for property-based testing.
//!
//! Generators produce random values for testing, with support for:
//! - Size-controlled generation
//! - Type-driven inference
//! - Composability

use crate::AriaValue;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

/// Context for generation, controls randomness and size
pub struct GenContext {
    /// Random number generator
    rng: StdRng,
    /// Current size parameter (influences collection sizes, etc.)
    size: usize,
    /// Maximum size parameter
    max_size: usize,
}

impl GenContext {
    /// Create a new generation context with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            size: 10,
            max_size: 100,
        }
    }

    /// Get the current size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Set the size
    pub fn set_size(&mut self, size: usize) {
        self.size = size.min(self.max_size);
    }

    /// Generate a random value in range
    pub fn gen_range<T: rand::distributions::uniform::SampleUniform + PartialOrd>(&mut self, low: T, high: T) -> T {
        self.rng.gen_range(low..high)
    }

    /// Generate a random bool
    pub fn gen_bool(&mut self) -> bool {
        self.rng.gen_bool(0.5)
    }

    /// Generate a random usize up to size
    pub fn gen_usize(&mut self) -> usize {
        self.rng.gen_range(0..self.size)
    }
}

/// A generator that produces values of type T
pub trait Generator {
    /// Generate a value using the context
    fn generate(&self, ctx: &mut GenContext) -> AriaValue;
}

/// Trait for types that can be arbitrarily generated
pub trait Arbitrary {
    /// Create a generator for this type
    fn arbitrary() -> Box<dyn Generator>;

    /// Create a shrinker for a value
    fn shrinker(value: Self) -> Box<dyn Iterator<Item = AriaValue>>;
}

// ============================================================================
// Built-in generators
// ============================================================================

/// Integer generator
pub struct IntGenerator {
    min: i64,
    max: i64,
}

impl IntGenerator {
    pub fn new(min: i64, max: i64) -> Self {
        Self { min, max }
    }
}

impl Generator for IntGenerator {
    fn generate(&self, ctx: &mut GenContext) -> AriaValue {
        let n = ctx.gen_range(self.min, self.max);
        AriaValue::Int(n)
    }
}

impl Arbitrary for i64 {
    fn arbitrary() -> Box<dyn Generator> {
        Box::new(IntGenerator::new(-1000, 1000))
    }

    fn shrinker(value: Self) -> Box<dyn Iterator<Item = AriaValue>> {
        Box::new(IntShrinker::new(value))
    }
}

/// Boolean generator
pub struct BoolGenerator;

impl Generator for BoolGenerator {
    fn generate(&self, ctx: &mut GenContext) -> AriaValue {
        AriaValue::Bool(ctx.gen_bool())
    }
}

impl Arbitrary for bool {
    fn arbitrary() -> Box<dyn Generator> {
        Box::new(BoolGenerator)
    }

    fn shrinker(value: Self) -> Box<dyn Iterator<Item = AriaValue>> {
        Box::new(BoolShrinker::new(value))
    }
}

/// String generator
pub struct StringGenerator {
    max_len: usize,
}

impl StringGenerator {
    pub fn new(max_len: usize) -> Self {
        Self { max_len }
    }
}

impl Generator for StringGenerator {
    fn generate(&self, ctx: &mut GenContext) -> AriaValue {
        let len = ctx.gen_usize().min(self.max_len);
        let s: String = (0..len)
            .map(|_| ctx.gen_range(b'a', b'z' + 1) as char)
            .collect();
        AriaValue::String(s)
    }
}

impl Arbitrary for String {
    fn arbitrary() -> Box<dyn Generator> {
        Box::new(StringGenerator::new(20))
    }

    fn shrinker(value: Self) -> Box<dyn Iterator<Item = AriaValue>> {
        Box::new(StringShrinker::new(value))
    }
}

/// Array generator
pub struct ArrayGenerator<G: Generator> {
    elem_gen: G,
    max_len: usize,
}

impl<G: Generator> ArrayGenerator<G> {
    pub fn new(elem_gen: G, max_len: usize) -> Self {
        Self { elem_gen, max_len }
    }
}

impl<G: Generator> Generator for ArrayGenerator<G> {
    fn generate(&self, ctx: &mut GenContext) -> AriaValue {
        let len = ctx.gen_usize().min(self.max_len);
        let elems: Vec<AriaValue> = (0..len)
            .map(|_| self.elem_gen.generate(ctx))
            .collect();
        AriaValue::Array(elems)
    }
}

impl<T: Arbitrary + 'static> Arbitrary for Vec<T> {
    fn arbitrary() -> Box<dyn Generator> {
        Box::new(VecGenerator::<T>::new())
    }

    fn shrinker(_value: Self) -> Box<dyn Iterator<Item = AriaValue>> {
        Box::new(std::iter::empty())
    }
}

/// Helper for Vec<T> generation
struct VecGenerator<T: Arbitrary> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Arbitrary> VecGenerator<T> {
    fn new() -> Self {
        Self { _marker: std::marker::PhantomData }
    }
}

impl<T: Arbitrary + 'static> Generator for VecGenerator<T> {
    fn generate(&self, ctx: &mut GenContext) -> AriaValue {
        let elem_gen = T::arbitrary();
        let len = ctx.gen_usize().min(10);
        let elems: Vec<AriaValue> = (0..len)
            .map(|_| elem_gen.generate(ctx))
            .collect();
        AriaValue::Array(elems)
    }
}

/// Tuple generator
pub struct TupleGenerator {
    generators: Vec<Box<dyn Generator>>,
}

impl TupleGenerator {
    pub fn new(generators: Vec<Box<dyn Generator>>) -> Self {
        Self { generators }
    }
}

impl Generator for TupleGenerator {
    fn generate(&self, ctx: &mut GenContext) -> AriaValue {
        let values: Vec<AriaValue> = self.generators
            .iter()
            .map(|g| g.generate(ctx))
            .collect();
        AriaValue::Tuple(values)
    }
}

/// Option generator
pub struct OptionGenerator<G: Generator> {
    inner: G,
}

impl<G: Generator> OptionGenerator<G> {
    pub fn new(inner: G) -> Self {
        Self { inner }
    }
}

impl<G: Generator> Generator for OptionGenerator<G> {
    fn generate(&self, ctx: &mut GenContext) -> AriaValue {
        if ctx.gen_bool() {
            AriaValue::Option(Some(Box::new(self.inner.generate(ctx))))
        } else {
            AriaValue::Option(None)
        }
    }
}

// ============================================================================
// Shrinkers
// ============================================================================

/// Integer shrinker - shrinks towards 0
struct IntShrinker {
    current: i64,
    done: bool,
}

impl IntShrinker {
    fn new(value: i64) -> Self {
        Self { current: value, done: value == 0 }
    }
}

impl Iterator for IntShrinker {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Shrink towards 0
        let shrunk = if self.current > 0 {
            self.current / 2
        } else if self.current < 0 {
            self.current / 2
        } else {
            self.done = true;
            return None;
        };

        if shrunk == self.current {
            self.done = true;
            return None;
        }

        self.current = shrunk;
        Some(AriaValue::Int(self.current))
    }
}

/// Boolean shrinker - shrinks true to false
struct BoolShrinker {
    value: bool,
    done: bool,
}

impl BoolShrinker {
    fn new(value: bool) -> Self {
        Self { value, done: !value }
    }
}

impl Iterator for BoolShrinker {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        self.done = true;
        if self.value {
            Some(AriaValue::Bool(false))
        } else {
            None
        }
    }
}

/// String shrinker - shrinks by removing characters
struct StringShrinker {
    current: String,
    index: usize,
}

impl StringShrinker {
    fn new(value: String) -> Self {
        Self { current: value, index: 0 }
    }
}

impl Iterator for StringShrinker {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_empty() || self.index >= self.current.len() {
            return None;
        }

        // Remove character at index
        let mut shrunk = self.current.clone();
        shrunk.remove(self.index);
        self.index += 1;
        Some(AriaValue::String(shrunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_generator() {
        let mut ctx = GenContext::new(42);
        let gen = IntGenerator::new(0, 100);
        let value = gen.generate(&mut ctx);
        if let AriaValue::Int(n) = value {
            assert!(n >= 0 && n < 100);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_string_generator() {
        let mut ctx = GenContext::new(42);
        let gen = StringGenerator::new(10);
        let value = gen.generate(&mut ctx);
        if let AriaValue::String(s) = value {
            assert!(s.len() <= 10);
            assert!(s.chars().all(|c| c.is_ascii_lowercase()));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_int_shrinker_positive() {
        let shrunk: Vec<_> = IntShrinker::new(100).collect();
        assert!(!shrunk.is_empty());
        // Should shrink towards 0
        if let AriaValue::Int(last) = shrunk.last().unwrap() {
            assert!(last.abs() < 100);
        }
    }

    #[test]
    fn test_int_shrinker_zero() {
        let shrunk: Vec<_> = IntShrinker::new(0).collect();
        assert!(shrunk.is_empty());
    }
}
