
use crate::concurrent_map::ConcurrentHashMap;
use crate::heap::{HeapItem, TtlHeap};
use crate::protocol::{hash_bytes, ErrorCode, ResponseBuilder};
use crate::zset::ZSet;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Entry type discriminator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    String,
    ZSet,
}

/// Database entry
pub struct Entry {
    pub entry_type: EntryType,
    pub string_value: Option<Vec<u8>>,
    pub zset_value: Option<ZSet>,
    pub ttl_heap_idx: Option<usize>, // Index in TTL heap if TTL is set
}

impl Entry {
    #[must_use]
    fn new_string(value: Vec<u8>) -> Self {
        Self {
            entry_type: EntryType::String,
            string_value: Some(value),
            zset_value: None,
            ttl_heap_idx: None,
        }
    }

    #[must_use]
    fn new_zset() -> Self {
        Self {
            entry_type: EntryType::ZSet,
            string_value: None,
            zset_value: Some(ZSet::new()),
            ttl_heap_idx: None,
        }
    }
}

/// Main storage engine
pub struct StorageEngine {
    db: Arc<ConcurrentHashMap<Vec<u8>, Arc<RwLock<Entry>>>>,
    ttl_heap: Arc<RwLock<TtlHeap<Vec<u8>>>>,
}

impl StorageEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            db: Arc::new(ConcurrentHashMap::with_capacity(256, 100)),
            ttl_heap: Arc::new(RwLock::new(TtlHeap::with_capacity(1000))),
        }
    }

    /// Get current time in microseconds
    #[must_use]
    fn now_us() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }

    /// GET command
    pub fn get(&self, key: &[u8], response: &mut ResponseBuilder) {
        match self.db.get(key) {
            Some(entry_arc) => {
                let entry = entry_arc.read().unwrap();
                match entry.entry_type {
                    EntryType::String => {
                        if let Some(ref val) = entry.string_value {
                            response.str(val);
                        } else {
                            response.nil();
                        }
                    }
                    EntryType::ZSet => {
                        response.error(ErrorCode::TypeError, "expect string type");
                    }
                }
            }
            None => response.nil(),
        }
    }

    /// SET command
    pub fn set(&self, key: Vec<u8>, value: Vec<u8>, response: &mut ResponseBuilder) {
        match self.db.get(&key) {
            Some(entry_arc) => {
                let mut entry = entry_arc.write().unwrap();
                if entry.entry_type != EntryType::String {
                    response.error(ErrorCode::TypeError, "expect string type");
                    return;
                }
                entry.string_value = Some(value);
            }
            None => {
                let entry = Arc::new(RwLock::new(Entry::new_string(value)));
                self.db.insert(key, entry);
            }
        }
        response.nil();
    }

    /// DEL command
    pub fn del(&self, key: &[u8], response: &mut ResponseBuilder) -> bool {
        if let Some(entry_arc) = self.db.remove(key) {
            // Remove from TTL heap if present
            let entry = entry_arc.read().unwrap();
            if let Some(heap_idx) = entry.ttl_heap_idx {
                let mut heap = self.ttl_heap.write().unwrap();
                heap.remove_by_index(heap_idx);
            }
            response.int(1);
            true
        } else {
            response.int(0);
            false
        }
    }

    /// ZADD command
    pub fn zadd(&self, key: Vec<u8>, score: f64, member: Vec<u8>, response: &mut ResponseBuilder) {
        match self.db.get(&key) {
            Some(entry_arc) => {
                let mut entry = entry_arc.write().unwrap();
                if entry.entry_type != EntryType::ZSet {
                    response.error(ErrorCode::TypeError, "expect zset type");
                    return;
                }
                if let Some(ref mut zset) = entry.zset_value {
                    let added = zset.add(member, score);
                    response.int(i64::from(added));
                } else {
                    response.error(ErrorCode::Unknown, "zset value is None");
                }
            }
            None => {
                let mut zset = ZSet::new();
                zset.add(member, score);
                let entry = Arc::new(RwLock::new(Entry::new_zset()));
                entry.write().unwrap().zset_value = Some(zset);
                self.db.insert(key, entry);
                response.int(1);
            }
        }
    }

    /// ZREM command
    pub fn zrem(&self, key: &[u8], member: &[u8], response: &mut ResponseBuilder) {
        match self.db.get(key) {
            Some(entry_arc) => {
                let mut entry = entry_arc.write().unwrap();
                if entry.entry_type != EntryType::ZSet {
                    response.error(ErrorCode::TypeError, "expect zset type");
                    return;
                }
                if let Some(ref mut zset) = entry.zset_value {
                    let removed = zset.remove(member);
                    response.int(i64::from(removed));
                } else {
                    response.int(0);
                }
            }
            None => response.int(0),
        }
    }

    /// ZSCORE command
    pub fn zscore(&self, key: &[u8], member: &[u8], response: &mut ResponseBuilder) {
        match self.db.get(key) {
            Some(entry_arc) => {
                let entry = entry_arc.read().unwrap();
                if entry.entry_type != EntryType::ZSet {
                    response.error(ErrorCode::TypeError, "expect zset type");
                    return;
                }
                if let Some(ref zset) = entry.zset_value {
                    match zset.score(member) {
                        Some(score) => response.double(score),
                        None => response.nil(),
                    }
                } else {
                    response.nil();
                }
            }
            None => response.nil(),
        }
    }

    /// ZQUERY command
    pub fn zquery(
        &self,
        key: &[u8],
        score: f64,
        offset: i64,
        limit: i64,
        response: &mut ResponseBuilder,
    ) {
        if offset < 0 || limit < 0 {
            response.error(ErrorCode::ArgError, "offset and limit must be non-negative");
            return;
        }

        match self.db.get(key) {
            Some(entry_arc) => {
                let entry = entry_arc.read().unwrap();
                if entry.entry_type != EntryType::ZSet {
                    response.error(ErrorCode::TypeError, "expect zset type");
                    return;
                }
                if let Some(ref zset) = entry.zset_value {
                    let results = zset.query(score, offset as usize, limit as usize);
                    let mut arr = response.array_start();
                    for (name, score) in results {
                        arr.item_str(&name);
                        arr.item_double(score);
                    }
                    arr.finish();
                } else {
                    response.array_start().finish();
                }
            }
            None => {
                response.array_start().finish();
            }
        }
    }

    /// PEXPIRE command (set TTL in milliseconds)
    pub fn pexpire(&self, key: &[u8], ttl_ms: i64, response: &mut ResponseBuilder) {
        if ttl_ms < 0 {
            // Remove TTL
            if let Some(entry_arc) = self.db.get(key) {
                let mut entry = entry_arc.write().unwrap();
                if let Some(heap_idx) = entry.ttl_heap_idx {
                    let mut heap = self.ttl_heap.write().unwrap();
                    heap.remove_by_index(heap_idx);
                    entry.ttl_heap_idx = None;
                }
            }
            response.int(1);
            return;
        }

        match self.db.get(key) {
            Some(entry_arc) => {
                let expiry_us = Self::now_us() + (ttl_ms as u64 * 1000);
                let mut entry = entry_arc.write().unwrap();
                let mut heap = self.ttl_heap.write().unwrap();

                if let Some(heap_idx) = entry.ttl_heap_idx {
                    // Update existing TTL
                    heap.update_by_index(heap_idx, expiry_us);
                } else {
                    // Add new TTL
                    let item = HeapItem {
                        expiry_us,
                        key: key.to_vec(),
                    };
                    heap.push(item);
                    entry.ttl_heap_idx = Some(heap.len() - 1);
                }
                response.int(1);
            }
            None => response.int(0),
        }
    }

    /// Process expired keys
    pub fn process_expiry(&self, max_work: usize) -> usize {
        let now_us = Self::now_us();
        let mut heap = self.ttl_heap.write().unwrap();
        let mut expired_count = 0;

        while expired_count < max_work {
            if let Some(item) = heap.peek() {
                if item.expiry_us >= now_us {
                    break;
                }
            } else {
                break;
            }

            if let Some(item) = heap.pop() {
                self.db.remove(&item.key);
                expired_count += 1;
            }
        }

        expired_count
    }

    /// Get next expiry time in microseconds (for timer management)
    #[must_use]
    pub fn next_expiry_us(&self) -> Option<u64> {
        let heap = self.ttl_heap.read().unwrap();
        heap.peek().map(|item| item.expiry_us)
    }

    /// Get database size
    #[must_use]
    pub fn size(&self) -> usize {
        self.db.len()
    }
}

impl Default for StorageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for StorageEngine {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            ttl_heap: Arc::clone(&self.ttl_heap),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_operations() {
        let engine = StorageEngine::new();
        let mut resp = ResponseBuilder::new();

        engine.set(b"key1".to_vec(), b"value1".to_vec(), &mut resp);
        
        let mut resp = ResponseBuilder::new();
        engine.get(b"key1", &mut resp);
        // Should have string response

        let mut resp = ResponseBuilder::new();
        engine.del(b"key1", &mut resp);
        
        let mut resp = ResponseBuilder::new();
        engine.get(b"key1", &mut resp);
        // Should be nil
    }

    #[test]
    fn test_zset_operations() {
        let engine = StorageEngine::new();
        let mut resp = ResponseBuilder::new();

        engine.zadd(b"myzset".to_vec(), 100.0, b"alice".to_vec(), &mut resp);
        
        let mut resp = ResponseBuilder::new();
        engine.zscore(b"myzset", b"alice", &mut resp);
        // Should have double response

        let mut resp = ResponseBuilder::new();
        engine.zrem(b"myzset", b"alice", &mut resp);
    }
}