

use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct HeapItem<K> {
    pub expiry_us: u64, // Expiry time in microseconds
    pub key: K,
}

/// Min-heap for TTL management
pub struct TtlHeap<K> {
    items: Vec<HeapItem<K>>,
}

impl<K> TtlHeap<K> {
    #[must_use]
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Peek at the minimum (earliest expiry) without removing
    #[must_use]
    pub fn peek(&self) -> Option<&HeapItem<K>> {
        self.items.first()
    }

    /// Push a new item onto the heap
    pub fn push(&mut self, item: HeapItem<K>) {
        self.items.push(item);
        self.sift_up(self.items.len() - 1);
    }

    /// Pop the minimum (earliest expiry) item
    pub fn pop(&mut self) -> Option<HeapItem<K>> {
        if self.items.is_empty() {
            return None;
        }

        let len = self.items.len();
        self.items.swap(0, len - 1);
        let item = self.items.pop();
        
        if !self.items.is_empty() {
            self.sift_down(0);
        }

        item
    }

    /// Update an item's expiry time and restore heap property
    pub fn update_by_index(&mut self, idx: usize, new_expiry_us: u64) {
        if idx >= self.items.len() {
            return;
        }

        let old_expiry = self.items[idx].expiry_us;
        self.items[idx].expiry_us = new_expiry_us;

        match new_expiry_us.cmp(&old_expiry) {
            Ordering::Less => self.sift_up(idx),
            Ordering::Greater => self.sift_down(idx),
            Ordering::Equal => {}
        }
    }

    /// Remove an item by index (replace with last and restore heap)
    pub fn remove_by_index(&mut self, idx: usize) -> Option<HeapItem<K>> {
        if idx >= self.items.len() {
            return None;
        }

        let len = self.items.len();
        self.items.swap(idx, len - 1);
        let item = self.items.pop();

        if idx < self.items.len() {
            // Restore heap property
            let parent_idx = if idx > 0 { (idx - 1) / 2 } else { 0 };
            if idx > 0 && self.items[idx].expiry_us < self.items[parent_idx].expiry_us {
                self.sift_up(idx);
            } else {
                self.sift_down(idx);
            }
        }

        item
    }

    fn sift_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent_idx = (idx - 1) / 2;
            if self.items[idx].expiry_us >= self.items[parent_idx].expiry_us {
                break;
            }
            self.items.swap(idx, parent_idx);
            idx = parent_idx;
        }
    }

    fn sift_down(&mut self, mut idx: usize) {
        let len = self.items.len();
        
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;
            let mut smallest = idx;

            if left < len && self.items[left].expiry_us < self.items[smallest].expiry_us {
                smallest = left;
            }

            if right < len && self.items[right].expiry_us < self.items[smallest].expiry_us {
                smallest = right;
            }

            if smallest == idx {
                break;
            }

            self.items.swap(idx, smallest);
            idx = smallest;
        }
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Get items expiring before a given time
    pub fn get_expired(&mut self, now_us: u64) -> Vec<HeapItem<K>> {
        let mut expired = Vec::new();
        
        while let Some(item) = self.peek() {
            if item.expiry_us >= now_us {
                break;
            }
            expired.push(self.pop().unwrap());
        }

        expired
    }
}

impl<K> Default for TtlHeap<K> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_heap_property() {
        let mut heap = TtlHeap::new();
        
        heap.push(HeapItem {
            expiry_us: 100,
            key: "key1",
        });
        heap.push(HeapItem {
            expiry_us: 50,
            key: "key2",
        });
        heap.push(HeapItem {
            expiry_us: 200,
            key: "key3",
        });

        assert_eq!(heap.peek().unwrap().expiry_us, 50);
        assert_eq!(heap.pop().unwrap().expiry_us, 50);
        assert_eq!(heap.pop().unwrap().expiry_us, 100);
        assert_eq!(heap.pop().unwrap().expiry_us, 200);
    }

    #[test]
    fn test_get_expired() {
        let mut heap = TtlHeap::new();
        
        heap.push(HeapItem {
            expiry_us: 100,
            key: 1,
        });
        heap.push(HeapItem {
            expiry_us: 200,
            key: 2,
        });
        heap.push(HeapItem {
            expiry_us: 50,
            key: 3,
        });

        let expired = heap.get_expired(150);
        assert_eq!(expired.len(), 2); // 50 and 100
        assert_eq!(heap.len(), 1); // 200 remains
    }

    #[test]
    fn test_update() {
        let mut heap = TtlHeap::new();
        
        heap.push(HeapItem {
            expiry_us: 100,
            key: "a",
        });
        heap.push(HeapItem {
            expiry_us: 200,
            key: "b",
        });
        heap.push(HeapItem {
            expiry_us: 150,
            key: "c",
        });

        // Update index 0 (100 -> 300)
        heap.update_by_index(0, 300);
        
        assert_eq!(heap.peek().unwrap().expiry_us, 150);
    }

    #[test]
    fn test_remove_by_index() {
        let mut heap = TtlHeap::new();
        
        heap.push(HeapItem {
            expiry_us: 100,
            key: 1,
        });
        heap.push(HeapItem {
            expiry_us: 200,
            key: 2,
        });
        heap.push(HeapItem {
            expiry_us: 150,
            key: 3,
        });

        assert_eq!(heap.len(), 3);
        heap.remove_by_index(1);
        assert_eq!(heap.len(), 2);
    }
}