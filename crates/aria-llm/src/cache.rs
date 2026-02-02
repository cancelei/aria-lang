//! Caching system for verified optimizations.
//!
//! Provides deterministic, reproducible builds by caching verified LLM optimizations.

use crate::{LlmError, Result};
use crate::provider::{LlmResponse, OptimizationSuggestion};
use crate::verifier::VerificationResult;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, Duration};

/// Key for cache lookups
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Hash of the original code
    pub code_hash: String,
    /// Hash of the optimization request parameters
    pub params_hash: String,
    /// Model identifier
    pub model: String,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(code: &str, params: &str, model: &str) -> Self {
        Self {
            code_hash: hash_string(code),
            params_hash: hash_string(params),
            model: model.to_string(),
        }
    }

    /// Create a combined hash for storage
    pub fn combined_hash(&self) -> String {
        hash_string(&format!("{}:{}:{}", self.code_hash, self.params_hash, self.model))
    }
}

/// Entry in the optimization cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cache key
    pub key: CacheKey,

    /// Cached LLM response
    pub response: LlmResponse,

    /// Verification result
    pub verification: VerificationResult,

    /// When this entry was created
    pub created_at: u64,

    /// When this entry expires (0 = never)
    pub expires_at: u64,

    /// Number of times this entry was used
    pub hit_count: u64,

    /// Version of the cache format
    pub version: u32,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(key: CacheKey, response: LlmResponse, verification: VerificationResult) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            key,
            response,
            verification,
            created_at: now,
            expires_at: 0, // Never expires by default
            hit_count: 0,
            version: 1,
        }
    }

    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now > self.expires_at
    }

    /// Set expiration time
    pub fn with_expiry(mut self, duration: Duration) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.expires_at = now + duration.as_secs();
        self
    }
}

/// Optimization cache
pub struct OptimizationCache {
    /// In-memory cache
    entries: HashMap<String, CacheEntry>,

    /// Disk cache directory (optional)
    cache_dir: Option<PathBuf>,

    /// Maximum entries in memory
    max_memory_entries: usize,

    /// Maximum disk size in bytes
    max_disk_size: u64,

    /// Statistics
    stats: CacheStats,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub disk_reads: u64,
    pub disk_writes: u64,
}

impl OptimizationCache {
    /// Create a new in-memory cache
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            cache_dir: None,
            max_memory_entries: 1000,
            max_disk_size: 100 * 1024 * 1024, // 100MB
            stats: CacheStats::default(),
        }
    }

    /// Create a cache with disk persistence
    pub fn with_disk(cache_dir: PathBuf) -> Result<Self> {
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)
                .map_err(|e| LlmError::CacheError(format!("Failed to create cache directory: {}", e)))?;
        }

        let mut cache = Self::new();
        cache.cache_dir = Some(cache_dir);
        Ok(cache)
    }

    /// Set maximum memory entries
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_memory_entries = max;
        self
    }

    /// Get a cached entry
    pub fn get(&mut self, key: &CacheKey) -> Option<CacheEntry> {
        let hash = key.combined_hash();

        // Try memory first
        if let Some(entry) = self.entries.get_mut(&hash) {
            if !entry.is_expired() {
                entry.hit_count += 1;
                self.stats.hits += 1;
                return Some(entry.clone());
            } else {
                // Entry expired, remove it
                let _ = self.entries.remove(&hash);
                self.stats.misses += 1;
                return None;
            }
        }

        // Try disk
        if let Some(mut entry) = self.load_from_disk(&hash) {
            if !entry.is_expired() {
                self.stats.hits += 1;
                self.stats.disk_reads += 1;
                entry.hit_count += 1;
                let result = entry.clone();
                self.entries.insert(hash, entry);
                return Some(result);
            }
        }

        self.stats.misses += 1;
        None
    }

    /// Store an entry in the cache
    pub fn put(&mut self, entry: CacheEntry) {
        let hash = entry.key.combined_hash();

        // Evict if necessary
        if self.entries.len() >= self.max_memory_entries {
            self.evict_lru();
        }

        // Store to disk if enabled
        if let Some(_) = &self.cache_dir {
            let _ = self.save_to_disk(&hash, &entry);
        }

        self.entries.insert(hash, entry);
    }

    /// Check if an entry exists
    pub fn contains(&self, key: &CacheKey) -> bool {
        let hash = key.combined_hash();
        if self.entries.contains_key(&hash) {
            return true;
        }

        if let Some(dir) = &self.cache_dir {
            let path = dir.join(&hash);
            return path.exists();
        }

        false
    }

    /// Remove an entry
    pub fn remove(&mut self, key: &CacheKey) -> Option<CacheEntry> {
        let hash = key.combined_hash();
        let entry = self.entries.remove(&hash);

        if let Some(dir) = &self.cache_dir {
            let path = dir.join(&hash);
            let _ = std::fs::remove_file(path);
        }

        entry
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();

        if let Some(dir) = &self.cache_dir {
            let _ = std::fs::remove_dir_all(dir);
            let _ = std::fs::create_dir_all(dir);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn evict_lru(&mut self) {
        // Find entry with lowest hit count
        if let Some((key, _)) = self.entries
            .iter()
            .min_by_key(|(_, e)| e.hit_count)
            .map(|(k, e)| (k.clone(), e.clone()))
        {
            self.entries.remove(&key);
            self.stats.evictions += 1;
        }
    }

    fn load_from_disk(&self, hash: &str) -> Option<CacheEntry> {
        let dir = self.cache_dir.as_ref()?;
        let path = dir.join(hash);

        if !path.exists() {
            return None;
        }

        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_to_disk(&self, hash: &str, entry: &CacheEntry) -> Result<()> {
        let dir = self.cache_dir.as_ref()
            .ok_or_else(|| LlmError::CacheError("No cache directory configured".to_string()))?;

        let path = dir.join(hash);
        let data = serde_json::to_string(entry)
            .map_err(|e| LlmError::CacheError(format!("Serialization error: {}", e)))?;

        std::fs::write(&path, data)
            .map_err(|e| LlmError::CacheError(format!("Write error: {}", e)))?;

        Ok(())
    }
}

impl Default for OptimizationCache {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_string(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifier::VerificationMethod;

    fn mock_response() -> LlmResponse {
        LlmResponse {
            request_id: "test".to_string(),
            suggestions: vec![],
            model: "test".to_string(),
            input_tokens: 0,
            output_tokens: 0,
            cached: false,
        }
    }

    fn mock_verification() -> VerificationResult {
        VerificationResult::success(VerificationMethod::Syntactic, Duration::from_secs(0))
    }

    #[test]
    fn test_cache_key() {
        let key1 = CacheKey::new("code", "params", "model");
        let key2 = CacheKey::new("code", "params", "model");
        let key3 = CacheKey::new("different", "params", "model");

        assert_eq!(key1.combined_hash(), key2.combined_hash());
        assert_ne!(key1.combined_hash(), key3.combined_hash());
    }

    #[test]
    fn test_cache_put_get() {
        let mut cache = OptimizationCache::new();
        let key = CacheKey::new("code", "params", "model");
        let entry = CacheEntry::new(key.clone(), mock_response(), mock_verification());

        cache.put(entry);
        assert!(cache.contains(&key));

        let retrieved = cache.get(&key);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = OptimizationCache::new();
        let key = CacheKey::new("nonexistent", "params", "model");

        assert!(!cache.contains(&key));
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = OptimizationCache::new().with_max_entries(2);

        for i in 0..3 {
            let key = CacheKey::new(&format!("code{}", i), "params", "model");
            let entry = CacheEntry::new(key, mock_response(), mock_verification());
            cache.put(entry);
        }

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().evictions, 1);
    }
}
