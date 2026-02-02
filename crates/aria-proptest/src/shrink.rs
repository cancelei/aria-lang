//! Shrinking strategies for property-based testing.
//!
//! When a test fails, shrinking finds minimal counterexamples.

use crate::AriaValue;

/// Trait for types that can shrink their values
pub trait Shrinker {
    /// Produce an iterator of smaller values
    fn shrink(&self, value: &AriaValue) -> ShrinkIterator;
}

/// An iterator that produces shrunk values
pub struct ShrinkIterator {
    inner: Box<dyn Iterator<Item = AriaValue>>,
}

impl ShrinkIterator {
    /// Create from any iterator
    pub fn new<I: Iterator<Item = AriaValue> + 'static>(iter: I) -> Self {
        Self { inner: Box::new(iter) }
    }

    /// Create an empty shrinker
    pub fn empty() -> Self {
        Self { inner: Box::new(std::iter::empty()) }
    }
}

impl Iterator for ShrinkIterator {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Generic shrinker based on type
pub struct TypedShrinker;

impl TypedShrinker {
    /// Shrink a value based on its type
    pub fn shrink(value: &AriaValue) -> ShrinkIterator {
        match value {
            AriaValue::Int(n) => ShrinkIterator::new(IntShrinkIter::new(*n)),
            AriaValue::Float(f) => ShrinkIterator::new(FloatShrinkIter::new(*f)),
            AriaValue::Bool(b) => ShrinkIterator::new(BoolShrinkIter::new(*b)),
            AriaValue::String(s) => ShrinkIterator::new(StringShrinkIter::new(s.clone())),
            AriaValue::Array(arr) => ShrinkIterator::new(ArrayShrinkIter::new(arr.clone())),
            AriaValue::Tuple(tup) => ShrinkIterator::new(TupleShrinkIter::new(tup.clone())),
            AriaValue::Option(opt) => ShrinkIterator::new(OptionShrinkIter::new(opt.clone())),
            AriaValue::Result(res) => ShrinkIterator::new(ResultShrinkIter::new(res.clone())),
            _ => ShrinkIterator::empty(),
        }
    }
}

// ============================================================================
// Shrink iterators for each type
// ============================================================================

struct IntShrinkIter {
    value: i64,
    step: i64,
    done: bool,
}

impl IntShrinkIter {
    fn new(value: i64) -> Self {
        Self {
            value,
            step: 0,
            done: value == 0,
        }
    }
}

impl Iterator for IntShrinkIter {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Strategy: try 0, then halves towards 0
        if self.step == 0 {
            self.step = 1;
            if self.value != 0 {
                return Some(AriaValue::Int(0));
            }
        }

        // Halve towards 0
        let candidate = self.value / 2_i64.pow(self.step as u32);
        if candidate == self.value || self.step > 20 {
            self.done = true;
            return None;
        }

        self.step += 1;
        Some(AriaValue::Int(candidate))
    }
}

struct FloatShrinkIter {
    value: f64,
    step: u32,
    done: bool,
}

impl FloatShrinkIter {
    fn new(value: f64) -> Self {
        Self {
            value,
            step: 0,
            done: value == 0.0,
        }
    }
}

impl Iterator for FloatShrinkIter {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.step == 0 {
            self.step = 1;
            return Some(AriaValue::Float(0.0));
        }

        let candidate = self.value / 2.0_f64.powi(self.step as i32);
        if candidate.abs() < 1e-10 || self.step > 20 {
            self.done = true;
            return None;
        }

        self.step += 1;
        Some(AriaValue::Float(candidate))
    }
}

struct BoolShrinkIter {
    value: bool,
    done: bool,
}

impl BoolShrinkIter {
    fn new(value: bool) -> Self {
        Self { value, done: !value }
    }
}

impl Iterator for BoolShrinkIter {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        self.done = true;
        Some(AriaValue::Bool(false))
    }
}

struct StringShrinkIter {
    value: String,
    index: usize,
}

impl StringShrinkIter {
    fn new(value: String) -> Self {
        Self { value, index: 0 }
    }
}

impl Iterator for StringShrinkIter {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        // First try empty string
        if self.index == 0 && !self.value.is_empty() {
            self.index = 1;
            return Some(AriaValue::String(String::new()));
        }

        // Then try removing characters one at a time
        if self.index > self.value.len() {
            return None;
        }

        if self.value.is_empty() {
            return None;
        }

        let char_idx = self.index - 1;
        if char_idx >= self.value.len() {
            return None;
        }

        let mut shrunk = self.value.clone();
        shrunk.remove(char_idx);
        self.index += 1;
        Some(AriaValue::String(shrunk))
    }
}

struct ArrayShrinkIter {
    value: Vec<AriaValue>,
    state: ArrayShrinkState,
}

enum ArrayShrinkState {
    TryEmpty,
    RemoveElements(usize),
    Done,
}

impl ArrayShrinkIter {
    fn new(value: Vec<AriaValue>) -> Self {
        Self {
            value,
            state: ArrayShrinkState::TryEmpty,
        }
    }
}

impl Iterator for ArrayShrinkIter {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &self.state {
                ArrayShrinkState::TryEmpty => {
                    self.state = ArrayShrinkState::RemoveElements(0);
                    if !self.value.is_empty() {
                        return Some(AriaValue::Array(Vec::new()));
                    }
                }
                ArrayShrinkState::RemoveElements(idx) => {
                    let idx = *idx;
                    if idx >= self.value.len() {
                        self.state = ArrayShrinkState::Done;
                        return None;
                    }
                    self.state = ArrayShrinkState::RemoveElements(idx + 1);
                    let mut shrunk = self.value.clone();
                    shrunk.remove(idx);
                    return Some(AriaValue::Array(shrunk));
                }
                ArrayShrinkState::Done => return None,
            }
        }
    }
}

struct TupleShrinkIter {
    value: Vec<AriaValue>,
    elem_idx: usize,
    elem_shrinker: Option<ShrinkIterator>,
}

impl TupleShrinkIter {
    fn new(value: Vec<AriaValue>) -> Self {
        Self {
            value,
            elem_idx: 0,
            elem_shrinker: None,
        }
    }
}

impl Iterator for TupleShrinkIter {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut shrinker) = self.elem_shrinker {
                if let Some(shrunk_elem) = shrinker.next() {
                    let mut new_tuple = self.value.clone();
                    new_tuple[self.elem_idx] = shrunk_elem;
                    return Some(AriaValue::Tuple(new_tuple));
                }
            }

            // Move to next element
            self.elem_idx += 1;
            if self.elem_idx > self.value.len() {
                return None;
            }

            if self.elem_idx <= self.value.len() {
                self.elem_shrinker = Some(TypedShrinker::shrink(&self.value[self.elem_idx - 1]));
            }
        }
    }
}

struct OptionShrinkIter {
    value: Option<Box<AriaValue>>,
    state: OptionShrinkState,
    inner_shrinker: Option<ShrinkIterator>,
}

enum OptionShrinkState {
    TryNone,
    ShrinkInner,
    Done,
}

impl OptionShrinkIter {
    fn new(value: Option<Box<AriaValue>>) -> Self {
        Self {
            value,
            state: OptionShrinkState::TryNone,
            inner_shrinker: None,
        }
    }
}

impl Iterator for OptionShrinkIter {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &self.state {
                OptionShrinkState::TryNone => {
                    self.state = OptionShrinkState::ShrinkInner;
                    if self.value.is_some() {
                        return Some(AriaValue::Option(None));
                    }
                }
                OptionShrinkState::ShrinkInner => {
                    if let Some(ref inner) = self.value {
                        if self.inner_shrinker.is_none() {
                            self.inner_shrinker = Some(TypedShrinker::shrink(inner));
                        }
                        if let Some(ref mut shrinker) = self.inner_shrinker {
                            if let Some(shrunk) = shrinker.next() {
                                return Some(AriaValue::Option(Some(Box::new(shrunk))));
                            }
                        }
                    }
                    self.state = OptionShrinkState::Done;
                }
                OptionShrinkState::Done => return None,
            }
        }
    }
}

struct ResultShrinkIter {
    value: Result<Box<AriaValue>, Box<AriaValue>>,
    shrinker: Option<ShrinkIterator>,
    done: bool,
}

impl ResultShrinkIter {
    fn new(value: Result<Box<AriaValue>, Box<AriaValue>>) -> Self {
        Self {
            value,
            shrinker: None,
            done: false,
        }
    }
}

impl Iterator for ResultShrinkIter {
    type Item = AriaValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.shrinker.is_none() {
            match &self.value {
                Ok(v) => self.shrinker = Some(TypedShrinker::shrink(v)),
                Err(e) => self.shrinker = Some(TypedShrinker::shrink(e)),
            }
        }

        if let Some(ref mut shrinker) = self.shrinker {
            if let Some(shrunk) = shrinker.next() {
                return match &self.value {
                    Ok(_) => Some(AriaValue::Result(Ok(Box::new(shrunk)))),
                    Err(_) => Some(AriaValue::Result(Err(Box::new(shrunk)))),
                };
            }
        }

        self.done = true;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_shrink() {
        let shrunk: Vec<_> = TypedShrinker::shrink(&AriaValue::Int(100)).take(5).collect();
        assert!(!shrunk.is_empty());
        // First shrink should be 0
        assert_eq!(shrunk[0], AriaValue::Int(0));
    }

    #[test]
    fn test_string_shrink() {
        let shrunk: Vec<_> = TypedShrinker::shrink(&AriaValue::String("hello".into())).take(3).collect();
        assert!(!shrunk.is_empty());
        // First shrink should be empty
        assert_eq!(shrunk[0], AriaValue::String(String::new()));
    }

    #[test]
    fn test_array_shrink() {
        let arr = AriaValue::Array(vec![AriaValue::Int(1), AriaValue::Int(2), AriaValue::Int(3)]);
        let shrunk: Vec<_> = TypedShrinker::shrink(&arr).take(5).collect();
        assert!(!shrunk.is_empty());
        // First shrink should be empty array
        assert_eq!(shrunk[0], AriaValue::Array(Vec::new()));
    }
}
