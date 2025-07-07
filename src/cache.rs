use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde_json::Value as JsonValue;

/// Simple in-memory cache for query results
#[derive(Debug, Clone)]
pub struct QueryCache {
    inner: Arc<RwLock<CacheInner>>,
    ttl: Duration,
    max_size: usize,
}

#[derive(Debug)]
struct CacheInner {
    entries: HashMap<String, CacheEntry>,
    access_order: Vec<String>, // For LRU eviction
    hits: u64,
    misses: u64,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: Vec<HashMap<String, JsonValue>>,
    created_at: Instant,
    access_count: u64,
}

impl QueryCache {
    /// Create a new query cache
    pub fn new(ttl_seconds: u64, max_size: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheInner {
                entries: HashMap::new(),
                access_order: Vec::new(),
                hits: 0,
                misses: 0,
            })),
            ttl: Duration::from_secs(ttl_seconds),
            max_size,
        }
    }

    /// Get cached query result if available and not expired
    pub fn get(&self, query: &str) -> Option<Vec<HashMap<String, JsonValue>>> {
        let Ok(mut cache) = self.inner.write() else { return None; };
        
        // Check if expired first
        if let Some(entry) = cache.entries.get(query) {
            if entry.created_at.elapsed() > self.ttl {
                cache.entries.remove(query);
                cache.access_order.retain(|q| q != query);
                return None;
            }
        }
        
        if cache.entries.contains_key(query) {
            // Update access stats
            if let Some(entry) = cache.entries.get_mut(query) {
                entry.access_count += 1;
                let data = entry.data.clone();
                
                // Move to end of access order (most recently used)
                cache.access_order.retain(|q| q != query);
                cache.access_order.push(query.to_string());
                
                cache.hits += 1;
                return Some(data);
            }
        }
        
        cache.misses += 1;
        None
    }

    /// Store query result in cache
    pub fn put(&self, query: String, result: Vec<HashMap<String, JsonValue>>) {
        let Ok(mut cache) = self.inner.write() else { return; };

        // If cache is full, remove least recently used entry
        if cache.entries.len() >= self.max_size && !cache.entries.contains_key(&query) {
            if let Some(lru_key) = cache.access_order.first().cloned() {
                cache.entries.remove(&lru_key);
                cache.access_order.retain(|q| q != &lru_key);
            }
        }

        let entry = CacheEntry {
            data: result,
            created_at: Instant::now(),
            access_count: 1,
        };

        cache.entries.insert(query.clone(), entry);
        
        // Add to access order if not already there
        if !cache.access_order.contains(&query) {
            cache.access_order.push(query);
        }
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        if let Ok(mut cache) = self.inner.write() {
            cache.entries.clear();
            cache.access_order.clear();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.inner.read().unwrap();
        
        let total_access_count: u64 = cache.entries.values()
            .map(|e| e.access_count)
            .sum();

        CacheStats {
            entries: cache.entries.len(),
            max_size: self.max_size,
            total_accesses: total_access_count,
            ttl_seconds: self.ttl.as_secs(),
        }
    }

    /// Create a cache key from SQL query (normalized)
    pub fn normalize_query(sql: &str) -> String {
        // Simple normalization: trim whitespace and convert to lowercase
        sql.trim().to_lowercase()
    }
    
    pub fn get_stats(&self) -> EnhancedCacheStats {
        let Ok(cache) = self.inner.read() else { 
            return EnhancedCacheStats {
                size: 0,
                hit_rate: 0.0,
                entries: 0,
                max_size: self.max_size,
                total_accesses: 0,
                ttl_seconds: self.ttl.as_secs(),
            }; 
        };
        
        let total_requests = cache.hits + cache.misses;
        let hit_rate = if total_requests > 0 {
            cache.hits as f64 / total_requests as f64
        } else {
            0.0
        };
        
        let total_accesses: u64 = cache.entries.values()
            .map(|entry| entry.access_count)
            .sum();
        
        EnhancedCacheStats {
            size: cache.entries.len(),
            hit_rate,
            entries: cache.entries.len(),
            max_size: self.max_size,
            total_accesses,
            ttl_seconds: self.ttl.as_secs(),
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub entries: usize,
    pub max_size: usize,
    pub total_accesses: u64,
    pub ttl_seconds: u64,
}

/// Enhanced cache statistics for metrics
#[derive(Debug, Clone)]
pub struct EnhancedCacheStats {
    pub size: usize,
    pub hit_rate: f64,
    pub entries: usize,
    pub max_size: usize,
    pub total_accesses: u64,
    pub ttl_seconds: u64,
}

impl Default for QueryCache {
    fn default() -> Self {
        // Default: 5 minute TTL, max 100 entries
        Self::new(300, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_basic_operations() {
        let cache = QueryCache::new(1, 2); // 1 second TTL, max 2 entries
        
        let query = "SELECT * FROM documents".to_string();
        let result = vec![HashMap::new()];
        
        // Initially empty
        assert!(cache.get(&query).is_none());
        
        // Store and retrieve
        cache.put(query.clone(), result.clone());
        assert_eq!(cache.get(&query).unwrap().len(), 1);
        
        // Test expiration
        thread::sleep(Duration::from_millis(1100));
        assert!(cache.get(&query).is_none());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = QueryCache::new(60, 2); // 60 second TTL, max 2 entries
        
        let query1 = "SELECT 1".to_string();
        let query2 = "SELECT 2".to_string();
        let query3 = "SELECT 3".to_string();
        
        let result = vec![HashMap::new()];
        
        // Fill cache
        cache.put(query1.clone(), result.clone());
        cache.put(query2.clone(), result.clone());
        
        // Both should be accessible
        assert!(cache.get(&query1).is_some());
        assert!(cache.get(&query2).is_some());
        
        // Adding third should evict first (LRU)
        cache.put(query3.clone(), result.clone());
        
        assert!(cache.get(&query1).is_none()); // Evicted
        assert!(cache.get(&query2).is_some()); // Still there
        assert!(cache.get(&query3).is_some()); // New entry
    }

    #[test]
    fn test_query_normalization() {
        let query1 = "  SELECT * FROM documents  ";
        let query2 = "SELECT * FROM DOCUMENTS";
        
        assert_eq!(
            QueryCache::normalize_query(query1),
            QueryCache::normalize_query(query2)
        );
    }
}
