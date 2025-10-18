use lru::LruCache;
use serde_json::Value;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use super::ToolResult;

type CacheKey = (String, String); // (tool_name, params_hash)

/// Thread-safe LRU cache for tool results
pub struct ToolCache {
    cache: Arc<Mutex<LruCache<CacheKey, ToolResult>>>,
    enabled: bool,
    hits: Arc<Mutex<u64>>,
    misses: Arc<Mutex<u64>>,
}

impl ToolCache {
    /// Create a new ToolCache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(
                LruCache::new(NonZeroUsize::new(capacity).unwrap())
            )),
            enabled: true,
            hits: Arc::new(Mutex::new(0)),
            misses: Arc::new(Mutex::new(0)),
        }
    }

    /// Get a cached result if available
    pub fn get(&self, tool_name: &str, params: &Value) -> Option<ToolResult> {
        if !self.enabled {
            return None;
        }

        let key = self.make_key(tool_name, params);
        let mut cache = self.cache.lock().unwrap();

        match cache.get(&key) {
            Some(result) => {
                *self.hits.lock().unwrap() += 1;
                tracing::debug!("Cache hit for tool '{}': params={}", tool_name, params);
                Some(result.clone())
            }
            None => {
                *self.misses.lock().unwrap() += 1;
                tracing::debug!("Cache miss for tool '{}': params={}", tool_name, params);
                None
            }
        }
    }

    /// Store a result in the cache
    pub fn put(&self, tool_name: &str, params: &Value, result: ToolResult) {
        if !self.enabled {
            return;
        }

        let key = self.make_key(tool_name, params);
        let mut cache = self.cache.lock().unwrap();
        cache.put(key, result);
        tracing::debug!("Cached result for tool '{}': params={}", tool_name, params);
    }

    /// Invalidate all cached results for a specific tool
    pub fn invalidate_tool(&self, tool_name: &str) {
        let mut cache = self.cache.lock().unwrap();

        // Collect keys to remove (can't modify while iterating)
        let keys_to_remove: Vec<CacheKey> = cache
            .iter()
            .filter(|(k, _)| k.0 == tool_name)
            .map(|(k, _)| k.clone())
            .collect();

        // Remove collected keys
        for key in keys_to_remove {
            cache.pop(&key);
        }

        tracing::debug!("Invalidated all cache entries for tool '{}'", tool_name);
    }

    /// Invalidate all cached results
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        tracing::debug!("Cleared entire cache");
    }

    /// Create a cache key from tool name and parameters
    /// Uses JSON serialization for simplicity (no need for md5 dependency)
    fn make_key(&self, tool_name: &str, params: &Value) -> CacheKey {
        let params_str = serde_json::to_string(params).unwrap_or_default();
        (tool_name.to_string(), params_str)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        let hits = *self.hits.lock().unwrap();
        let misses = *self.misses.lock().unwrap();
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
            hits,
            misses,
            hit_rate,
        }
    }

    /// Enable or disable the cache
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        tracing::info!("Cache {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache: {}/{} entries, {} hits, {} misses, {:.1}% hit rate",
            self.size, self.capacity, self.hits, self.misses, self.hit_rate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_get_put() {
        let cache = ToolCache::new(10);
        let params = serde_json::json!({"file_path": "/tmp/test.txt"});
        let result = ToolResult {
            content: "test content".to_string(),
            is_error: false,
        };

        // Should be a miss initially
        assert!(cache.get("read", &params).is_none());

        // Put and retrieve
        cache.put("read", &params, result.clone());
        let cached = cache.get("read", &params).unwrap();
        assert_eq!(cached.content, "test content");
        assert!(!cached.is_error);
    }

    #[test]
    fn test_cache_different_params() {
        let cache = ToolCache::new(10);
        let params1 = serde_json::json!({"file_path": "/tmp/test1.txt"});
        let params2 = serde_json::json!({"file_path": "/tmp/test2.txt"});

        let result1 = ToolResult {
            content: "content1".to_string(),
            is_error: false,
        };
        let result2 = ToolResult {
            content: "content2".to_string(),
            is_error: false,
        };

        cache.put("read", &params1, result1);
        cache.put("read", &params2, result2);

        // Should retrieve correct results
        assert_eq!(cache.get("read", &params1).unwrap().content, "content1");
        assert_eq!(cache.get("read", &params2).unwrap().content, "content2");
    }

    #[test]
    fn test_cache_invalidate_tool() {
        let cache = ToolCache::new(10);
        let params = serde_json::json!({"file_path": "/tmp/test.txt"});
        let result = ToolResult {
            content: "test".to_string(),
            is_error: false,
        };

        cache.put("read", &params, result.clone());
        cache.put("grep", &serde_json::json!({"pattern": "test"}), result);

        // Invalidate read tool
        cache.invalidate_tool("read");

        // read should be gone, grep should remain
        assert!(cache.get("read", &params).is_none());
        assert!(cache.get("grep", &serde_json::json!({"pattern": "test"})).is_some());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = ToolCache::new(2); // Small cache
        let params1 = serde_json::json!({"key": "1"});
        let params2 = serde_json::json!({"key": "2"});
        let params3 = serde_json::json!({"key": "3"});

        let result = ToolResult {
            content: "test".to_string(),
            is_error: false,
        };

        // Fill cache
        cache.put("tool", &params1, result.clone());
        cache.put("tool", &params2, result.clone());

        // Access params1 to make it most recently used
        cache.get("tool", &params1);

        // Add params3, should evict params2 (least recently used)
        cache.put("tool", &params3, result);

        assert!(cache.get("tool", &params1).is_some());
        assert!(cache.get("tool", &params2).is_none()); // Evicted
        assert!(cache.get("tool", &params3).is_some());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ToolCache::new(10);
        let params = serde_json::json!({"file_path": "/tmp/test.txt"});
        let result = ToolResult {
            content: "test".to_string(),
            is_error: false,
        };

        // Initial stats
        let stats = cache.stats();
        assert_eq!(stats.size, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);

        // Miss
        cache.get("read", &params);
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.0);

        // Put and hit
        cache.put("read", &params, result);
        cache.get("read", &params);
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 1);
        assert_eq!(stats.hit_rate, 50.0);
    }

    #[test]
    fn test_cache_disabled() {
        let mut cache = ToolCache::new(10);
        cache.set_enabled(false);

        let params = serde_json::json!({"file_path": "/tmp/test.txt"});
        let result = ToolResult {
            content: "test".to_string(),
            is_error: false,
        };

        // Put should do nothing when disabled
        cache.put("read", &params, result);
        assert!(cache.get("read", &params).is_none());

        // Re-enable
        cache.set_enabled(true);
        assert!(cache.is_enabled());
    }

    #[test]
    fn test_cache_clear() {
        let cache = ToolCache::new(10);
        let params = serde_json::json!({"file_path": "/tmp/test.txt"});
        let result = ToolResult {
            content: "test".to_string(),
            is_error: false,
        };

        cache.put("read", &params, result);
        assert!(cache.get("read", &params).is_some());

        cache.clear();
        assert!(cache.get("read", &params).is_none());
        assert_eq!(cache.stats().size, 0);
    }
}
