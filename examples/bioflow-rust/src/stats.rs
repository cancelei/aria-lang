//! Statistical analysis utilities for sequence data.
//!
//! This module provides various statistical functions commonly used
//! in bioinformatics analysis.

use std::collections::HashMap;

/// Summary statistics for a dataset.
#[derive(Debug, Clone)]
pub struct SummaryStats {
    /// Number of values.
    pub count: usize,
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
    /// Sum of values.
    pub sum: f64,
    /// Mean (average).
    pub mean: f64,
    /// Variance.
    pub variance: f64,
    /// Standard deviation.
    pub std_dev: f64,
    /// Median.
    pub median: f64,
    /// First quartile (25th percentile).
    pub q1: f64,
    /// Third quartile (75th percentile).
    pub q3: f64,
}

impl SummaryStats {
    /// Computes summary statistics from a slice of values.
    ///
    /// # Arguments
    ///
    /// * `data` - A slice of f64 values.
    ///
    /// # Returns
    ///
    /// `Some(SummaryStats)` if data is non-empty, `None` otherwise.
    pub fn from_data(data: &[f64]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let count = data.len();
        let sum: f64 = data.iter().sum();
        let mean = sum / count as f64;

        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Variance (population)
        let variance = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        // Sort for percentiles
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let median = percentile(&sorted, 50.0);
        let q1 = percentile(&sorted, 25.0);
        let q3 = percentile(&sorted, 75.0);

        Some(Self {
            count,
            min,
            max,
            sum,
            mean,
            variance,
            std_dev,
            median,
            q1,
            q3,
        })
    }

    /// Returns the interquartile range (Q3 - Q1).
    pub fn iqr(&self) -> f64 {
        self.q3 - self.q1
    }

    /// Returns the coefficient of variation (std_dev / mean).
    pub fn coefficient_of_variation(&self) -> f64 {
        if self.mean.abs() < f64::EPSILON {
            0.0
        } else {
            self.std_dev / self.mean
        }
    }
}

/// Calculates the percentile value from a sorted slice.
///
/// Uses linear interpolation between adjacent values.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let p = p.clamp(0.0, 100.0) / 100.0;
    let n = sorted.len() as f64;
    let index = p * (n - 1.0);
    let lower = index.floor() as usize;
    let upper = (lower + 1).min(sorted.len() - 1);
    let fraction = index - lower as f64;

    sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
}

/// Running statistics calculator for streaming data.
///
/// Uses Welford's online algorithm for numerically stable
/// variance calculation.
#[derive(Debug, Clone)]
pub struct RunningStats {
    count: usize,
    mean: f64,
    m2: f64, // Sum of squared differences from mean
    min: f64,
    max: f64,
}

impl RunningStats {
    /// Creates a new empty RunningStats.
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Adds a value to the running statistics.
    pub fn push(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;

        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    /// Returns the number of values.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the mean.
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Returns the population variance.
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / self.count as f64
        }
    }

    /// Returns the sample variance.
    pub fn sample_variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }

    /// Returns the standard deviation.
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Returns the minimum value.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Returns the maximum value.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Merges another RunningStats into this one.
    pub fn merge(&mut self, other: &RunningStats) {
        if other.count == 0 {
            return;
        }
        if self.count == 0 {
            *self = other.clone();
            return;
        }

        let total = self.count + other.count;
        let delta = other.mean - self.mean;

        self.mean = (self.count as f64 * self.mean + other.count as f64 * other.mean) / total as f64;
        self.m2 = self.m2 + other.m2 + delta * delta * (self.count * other.count) as f64 / total as f64;
        self.count = total;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
}

impl Default for RunningStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Histogram for integer values.
#[derive(Debug, Clone)]
pub struct Histogram {
    bins: HashMap<i64, usize>,
    total: usize,
}

impl Histogram {
    /// Creates a new empty histogram.
    pub fn new() -> Self {
        Self {
            bins: HashMap::new(),
            total: 0,
        }
    }

    /// Adds a value to the histogram.
    pub fn add(&mut self, value: i64) {
        *self.bins.entry(value).or_insert(0) += 1;
        self.total += 1;
    }

    /// Adds a value with a specific count.
    pub fn add_count(&mut self, value: i64, count: usize) {
        *self.bins.entry(value).or_insert(0) += count;
        self.total += count;
    }

    /// Returns the count for a specific value.
    pub fn get(&self, value: i64) -> usize {
        *self.bins.get(&value).unwrap_or(&0)
    }

    /// Returns the total count.
    pub fn total(&self) -> usize {
        self.total
    }

    /// Returns the number of unique values.
    pub fn unique_values(&self) -> usize {
        self.bins.len()
    }

    /// Returns the mode (most frequent value).
    pub fn mode(&self) -> Option<i64> {
        self.bins.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(&value, _)| value)
    }

    /// Returns the mean.
    pub fn mean(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }

        let sum: i64 = self.bins.iter()
            .map(|(&value, &count)| value * count as i64)
            .sum();

        sum as f64 / self.total as f64
    }

    /// Returns the frequency (proportion) of a value.
    pub fn frequency(&self, value: i64) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.get(value) as f64 / self.total as f64
        }
    }

    /// Returns all (value, count) pairs sorted by value.
    pub fn to_vec(&self) -> Vec<(i64, usize)> {
        let mut result: Vec<_> = self.bins.iter()
            .map(|(&v, &c)| (v, c))
            .collect();
        result.sort_by_key(|(v, _)| *v);
        result
    }

    /// Returns the minimum value.
    pub fn min(&self) -> Option<i64> {
        self.bins.keys().min().copied()
    }

    /// Returns the maximum value.
    pub fn max(&self) -> Option<i64> {
        self.bins.keys().max().copied()
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculates Shannon entropy from a probability distribution.
///
/// # Arguments
///
/// * `probabilities` - Slice of probabilities (should sum to 1).
///
/// # Returns
///
/// The Shannon entropy in bits.
pub fn shannon_entropy(probabilities: &[f64]) -> f64 {
    probabilities.iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.log2())
        .sum()
}

/// Calculates the GC content of a sequence string.
pub fn gc_content(sequence: &str) -> f64 {
    if sequence.is_empty() {
        return 0.0;
    }

    let gc_count = sequence.bytes()
        .filter(|&b| b == b'G' || b == b'C' || b == b'g' || b == b'c')
        .count();

    gc_count as f64 / sequence.len() as f64
}

/// Calculates N50 statistic from a collection of lengths.
///
/// N50 is the length such that half of the total sequence length
/// is contained in sequences of this length or longer.
pub fn n50(lengths: &[usize]) -> usize {
    if lengths.is_empty() {
        return 0;
    }

    let mut sorted = lengths.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a)); // Sort descending

    let total: usize = sorted.iter().sum();
    let half = total / 2;

    let mut running_sum = 0;
    for &length in &sorted {
        running_sum += length;
        if running_sum >= half {
            return length;
        }
    }

    sorted[0]
}

/// Calculates L50 statistic (number of sequences in the N50 set).
pub fn l50(lengths: &[usize]) -> usize {
    if lengths.is_empty() {
        return 0;
    }

    let mut sorted = lengths.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a)); // Sort descending

    let total: usize = sorted.iter().sum();
    let half = total / 2;

    let mut running_sum = 0;
    for (i, &length) in sorted.iter().enumerate() {
        running_sum += length;
        if running_sum >= half {
            return i + 1;
        }
    }

    sorted.len()
}

/// Pearson correlation coefficient between two datasets.
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.is_empty() {
        return None;
    }

    let n = x.len() as f64;
    let mean_x: f64 = x.iter().sum::<f64>() / n;
    let mean_y: f64 = y.iter().sum::<f64>() / n;

    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;

    for i in 0..x.len() {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        sum_xy += dx * dy;
        sum_x2 += dx * dx;
        sum_y2 += dy * dy;
    }

    let denominator = (sum_x2 * sum_y2).sqrt();
    if denominator < f64::EPSILON {
        return None;
    }

    Some(sum_xy / denominator)
}

/// Calculates the z-score for a value given mean and standard deviation.
pub fn z_score(value: f64, mean: f64, std_dev: f64) -> f64 {
    if std_dev.abs() < f64::EPSILON {
        0.0
    } else {
        (value - mean) / std_dev
    }
}

/// Moving average with a sliding window.
pub fn moving_average(data: &[f64], window_size: usize) -> Vec<f64> {
    if window_size == 0 || window_size > data.len() {
        return vec![];
    }

    let mut result = Vec::with_capacity(data.len() - window_size + 1);
    let mut sum: f64 = data[..window_size].iter().sum();

    result.push(sum / window_size as f64);

    for i in window_size..data.len() {
        sum -= data[i - window_size];
        sum += data[i];
        result.push(sum / window_size as f64);
    }

    result
}

/// Exponential moving average.
pub fn exponential_moving_average(data: &[f64], alpha: f64) -> Vec<f64> {
    if data.is_empty() {
        return vec![];
    }

    let mut result = Vec::with_capacity(data.len());
    let mut ema = data[0];

    for &value in data {
        ema = alpha * value + (1.0 - alpha) * ema;
        result.push(ema);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_stats() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = SummaryStats::from_data(&data).unwrap();

        assert_eq!(stats.count, 5);
        assert!((stats.mean - 3.0).abs() < f64::EPSILON);
        assert!((stats.min - 1.0).abs() < f64::EPSILON);
        assert!((stats.max - 5.0).abs() < f64::EPSILON);
        assert!((stats.median - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_running_stats() {
        let mut stats = RunningStats::new();
        for v in &[1.0, 2.0, 3.0, 4.0, 5.0] {
            stats.push(*v);
        }

        assert_eq!(stats.count(), 5);
        assert!((stats.mean() - 3.0).abs() < f64::EPSILON);
        assert!((stats.min() - 1.0).abs() < f64::EPSILON);
        assert!((stats.max() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_running_stats_merge() {
        let mut stats1 = RunningStats::new();
        for v in &[1.0, 2.0, 3.0] {
            stats1.push(*v);
        }

        let mut stats2 = RunningStats::new();
        for v in &[4.0, 5.0] {
            stats2.push(*v);
        }

        stats1.merge(&stats2);

        assert_eq!(stats1.count(), 5);
        assert!((stats1.mean() - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_histogram() {
        let mut hist = Histogram::new();
        hist.add(1);
        hist.add(2);
        hist.add(2);
        hist.add(3);
        hist.add(3);
        hist.add(3);

        assert_eq!(hist.total(), 6);
        assert_eq!(hist.get(1), 1);
        assert_eq!(hist.get(2), 2);
        assert_eq!(hist.get(3), 3);
        assert_eq!(hist.mode(), Some(3));
    }

    #[test]
    fn test_shannon_entropy() {
        // Equal probability = maximum entropy
        let probs = vec![0.25, 0.25, 0.25, 0.25];
        let entropy = shannon_entropy(&probs);
        assert!((entropy - 2.0).abs() < f64::EPSILON);

        // Certainty = zero entropy
        let probs = vec![1.0, 0.0, 0.0, 0.0];
        let entropy = shannon_entropy(&probs);
        assert!(entropy.abs() < f64::EPSILON);
    }

    #[test]
    fn test_gc_content() {
        assert!((gc_content("ATGC") - 0.5).abs() < f64::EPSILON);
        assert!((gc_content("GGCC") - 1.0).abs() < f64::EPSILON);
        assert!((gc_content("AATT") - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_n50() {
        let lengths = vec![100, 200, 300, 400, 500];
        // Total = 1500, half = 750
        // 500 >= 750? No (500)
        // 500 + 400 = 900 >= 750? Yes
        assert_eq!(n50(&lengths), 400);
    }

    #[test]
    fn test_l50() {
        let lengths = vec![100, 200, 300, 400, 500];
        // N50 is at position 2 (index 1)
        assert_eq!(l50(&lengths), 2);
    }

    #[test]
    fn test_pearson_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let corr = pearson_correlation(&x, &y).unwrap();
        assert!((corr - 1.0).abs() < f64::EPSILON);

        let y_neg = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let corr_neg = pearson_correlation(&x, &y_neg).unwrap();
        assert!((corr_neg + 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_moving_average() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ma = moving_average(&data, 3);

        assert_eq!(ma.len(), 3);
        assert!((ma[0] - 2.0).abs() < f64::EPSILON); // (1+2+3)/3
        assert!((ma[1] - 3.0).abs() < f64::EPSILON); // (2+3+4)/3
        assert!((ma[2] - 4.0).abs() < f64::EPSILON); // (3+4+5)/3
    }

    #[test]
    fn test_z_score() {
        let z = z_score(10.0, 5.0, 2.0);
        assert!((z - 2.5).abs() < f64::EPSILON);
    }
}
