use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

const DEFAULT_SHARD_COUNT: usize = 256; // Must be power of 2

/// A single shard in the concurrent hash map
struct Shard<K, V> {
    map: RwLock<HashMap<K, V>>,
}

impl<K, V> Shard<K, V> {
    fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            map: RwLock::new(HashMap::with_capacity(capacity)),
        }
    }
}

/// Concurrent hash map with shard-level locking
pub struct ConcurrentHashMap<K, V> {
    shards: Vec<Shard<K, V>>,
    shard_mask: usize,
}

impl<K, V> ConcurrentHashMap<K, V>
where
    K: Eq + Hash,
{
    /// Create a new concurrent hash map with default shard count
    #[must_use]
    pub fn new() -> Self {
        Self::with_shard_count(DEFAULT_SHARD_COUNT)
    }

    /// Create with a specific shard count (must be power of 2)
    #[must_use]
    pub fn with_shard_count(shard_count: usize) -> Self {
        assert!(
            shard_count > 0 && (shard_count & (shard_count - 1)) == 0,
            "shard_count must be a power of 2"
        );

        let shards = (0..shard_count).map(|_| Shard::new()).collect();

        Self {
            shards,
            shard_mask: shard_count - 1,
        }
    }

    /// Create with capacity hint per shard
    #[must_use]
    pub fn with_capacity(shard_count: usize, capacity_per_shard: usize) -> Self {
        assert!(
            shard_count > 0 && (shard_count & (shard_count - 1)) == 0,
            "shard_count must be a power of 2"
        );

        let shards = (0..shard_count)
            .map(|_| Shard::with_capacity(capacity_per_shard))
            .collect();

        Self {
            shards,
            shard_mask: shard_count - 1,
        }
    }

    /// Get the shard index for a given key
    fn shard_idx(&self, key: &K) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish() as usize;
        hash & self.shard_mask
    }

    /// Get a value by key (shared read lock)
    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        V: Clone,
    {
        let idx = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            let hash = hasher.finish() as usize;
            hash & self.shard_mask
        };

        let shard = &self.shards[idx];
        let guard = shard.map.read().unwrap();
        guard.get(key).cloned()
    }

    /// Get a value with a custom function (avoids clone)
    pub fn get_with<Q, F, R>(&self, key: &Q, f: F) -> Option<R>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&V) -> R,
    {
        let idx = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            let hash = hasher.finish() as usize;
            hash & self.shard_mask
        };

        let shard = &self.shards[idx];
        let guard = shard.map.read().unwrap();
        guard.get(key).map(f)
    }

    /// Insert a key-value pair
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let idx = self.shard_idx(&key);
        let shard = &self.shards[idx];
        let mut guard = shard.map.write().unwrap();
        guard.insert(key, value)
    }

    /// Remove a key-value pair
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            let hash = hasher.finish() as usize;
            hash & self.shard_mask
        };

        let shard = &self.shards[idx];
        let mut guard = shard.map.write().unwrap();
        guard.remove(key)
    }

    /// Check if a key exists
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            let hash = hasher.finish() as usize;
            hash & self.shard_mask
        };

        let shard = &self.shards[idx];
        let guard = shard.map.read().unwrap();
        guard.contains_key(key)
    }

    /// Update a value in place
    pub fn update<Q, F>(&self, key: &Q, f: F) -> bool
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(&mut V),
    {
        let idx = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            let hash = hasher.finish() as usize;
            hash & self.shard_mask
        };

        let shard = &self.shards[idx];
        let mut guard = shard.map.write().unwrap();
        if let Some(value) = guard.get_mut(key) {
            f(value);
            true
        } else {
            false
        }
    }

    /// Get approximate total size (sum across all shards)
    pub fn len(&self) -> usize {
        self.shards
            .iter()
            .map(|shard| shard.map.read().unwrap().len())
            .sum()
    }

    /// Check if map is empty
    pub fn is_empty(&self) -> bool {
        self.shards
            .iter()
            .all(|shard| shard.map.read().unwrap().is_empty())
    }

    /// Clear all entries
    pub fn clear(&self) {
        for shard in &self.shards {
            shard.map.write().unwrap().clear();
        }
    }

    /// Iterate over all keys (snapshot at call time)
    pub fn keys(&self) -> Vec<K>
    where
        K: Clone,
    {
        let mut result = Vec::new();
        for shard in &self.shards {
            let guard = shard.map.read().unwrap();
            result.extend(guard.keys().cloned());
        }
        result
    }

    /// Apply a function to each key-value pair
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&K, &V),
    {
        for shard in &self.shards {
            let guard = shard.map.read().unwrap();
            for (k, v) in guard.iter() {
                f(k, v);
            }
        }
    }

    /// Retain only entries that satisfy the predicate
    pub fn retain<F>(&self, mut predicate: F)
    where
        F: FnMut(&K, &V) -> bool,
    {
        for shard in &self.shards {
            let mut guard = shard.map.write().unwrap();
            guard.retain(|k, v| predicate(k, v));
        }
    }
}

impl<K, V> Default for ConcurrentHashMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe reference to the concurrent hash map
pub type SharedHashMap<K, V> = Arc<ConcurrentHashMap<K, V>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_basic_operations() {
        let map = ConcurrentHashMap::new();
        
        assert!(map.is_empty());
        
        map.insert("key1", 100);
        assert_eq!(map.get("key1"), Some(100));
        assert_eq!(map.len(), 1);
        
        map.insert("key2", 200);
        assert_eq!(map.len(), 2);
        
        assert_eq!(map.remove("key1"), Some(100));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key1"), None);
    }

    #[test]
    fn test_concurrent_access() {
        let map = Arc::new(ConcurrentHashMap::new());
        let mut handles = vec![];

        // Spawn multiple threads that insert values
        for i in 0..10 {
            let map_clone = Arc::clone(&map);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let key = format!("key_{}_{}", i, j);
                    map_clone.insert(key, i * 100 + j);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(map.len(), 1000);
    }

    #[test]
    fn test_update() {
        let map = ConcurrentHashMap::new();
        map.insert("counter", 0);

        map.update("counter", |v| *v += 1);
        assert_eq!(map.get("counter"), Some(1));

        map.update("counter", |v| *v += 10);
        assert_eq!(map.get("counter"), Some(11));
    }

    #[test]
    fn test_shard_distribution() {
        let map = ConcurrentHashMap::with_shard_count(16);
        
        // Insert many keys
        for i in 0..1000 {
            map.insert(i, i);
        }

        // Check that keys are distributed across shards (not all in one)
        let mut non_empty_shards = 0;
        for shard in &map.shards {
            if !shard.map.read().unwrap().is_empty() {
                non_empty_shards += 1;
            }
        }
        
        // Should use multiple shards (not perfect distribution expected)
        assert!(non_empty_shards > 1);
    }
}