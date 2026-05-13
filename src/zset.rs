

use crate::avl::{AvlTree, Node};
use std::cmp::Ordering;
use std::collections::HashMap;

/// ZSet element with score and name
#[derive(Debug, Clone)]
pub struct ZSetElement {
    pub score: f64,
    pub name: Vec<u8>,
}

impl ZSetElement {
    #[must_use]
    pub fn new(name: Vec<u8>, score: f64) -> Self {
        Self { name, score }
    }

    /// Compare by (score, name) tuple for total ordering
    fn compare(&self, other: &Self) -> Ordering {
        match self.score.partial_cmp(&other.score) {
            Some(Ordering::Equal) | None => self.name.cmp(&other.name),
            Some(ord) => ord,
        }
    }
}

/// Sorted set data structure
pub struct ZSet {
    tree: AvlTree<ZSetElement>,
    map: HashMap<Vec<u8>, usize>, // name -> tree node index
}

impl ZSet {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tree: AvlTree::new(),
            map: HashMap::new(),
        }
    }

    /// Add or update an element
    pub fn add(&mut self, name: Vec<u8>, score: f64) -> bool {
        // Check if element already exists
        if let Some(&old_idx) = self.map.get(&name) {
            // Update existing element
            let old_elem = self.tree.get(old_idx).unwrap();
            
            // Only update if score changed
            if (old_elem.data.score - score).abs() < f64::EPSILON {
                return false;
            }

            // Remove old node and insert new one with updated score
            let old_elem_data = self.tree.delete(old_idx);
            self.map.remove(&old_elem_data.name);
            
            let elem = ZSetElement::new(name.clone(), score);
            let new_idx = self.tree.insert(elem, |a, b| a.compare(b));
            self.map.insert(name, new_idx);
            
            false // Updated existing
        } else {
            // Insert new element
            let elem = ZSetElement::new(name.clone(), score);
            let idx = self.tree.insert(elem, |a, b| a.compare(b));
            self.map.insert(name, idx);
            true // Added new
        }
    }

    /// Remove an element by name
    pub fn remove(&mut self, name: &[u8]) -> bool {
        if let Some(&idx) = self.map.get(name) {
            self.tree.delete(idx);
            self.map.remove(name);
            true
        } else {
            false
        }
    }

    /// Get score by name
    #[must_use]
    pub fn score(&self, name: &[u8]) -> Option<f64> {
        self.map
            .get(name)
            .and_then(|&idx| self.tree.get(idx))
            .map(|node| node.data.score)
    }

    /// Query elements by score range
    /// Returns elements with score in [min_score, max_score] range, limited by offset and limit
    #[must_use]
    pub fn query(
        &self,
        min_score: f64,
        offset: usize,
        limit: usize,
    ) -> Vec<(Vec<u8>, f64)> {
        let mut result = Vec::new();
        
        if self.tree.is_empty() {
            return result;
        }

        // Find the first element >= min_score
        let start_idx = self.find_first_gte(min_score);
        
        if start_idx.is_none() {
            return result;
        }

        let mut current_idx = start_idx;
        let mut skipped = 0;
        let mut collected = 0;

        while let Some(idx) = current_idx {
            if collected >= limit {
                break;
            }

            if let Some(node) = self.tree.get(idx) {
                if skipped >= offset {
                    result.push((node.data.name.clone(), node.data.score));
                    collected += 1;
                } else {
                    skipped += 1;
                }

                // Move to next element (in-order successor)
                current_idx = self.tree.offset(idx, 1);
            } else {
                break;
            }
        }

        result
    }

    /// Find first element with score >= target
    fn find_first_gte(&self, target_score: f64) -> Option<usize> {
        let mut current = self.tree.root();
        let mut result = None;

        while current != usize::MAX {
            if let Some(node) = self.tree.get(current) {
                if node.data.score >= target_score {
                    result = Some(current);
                    current = node.left;
                } else {
                    current = node.right;
                }
            } else {
                break;
            }
        }

        result
    }

    /// Get rank (0-indexed position) of an element
    #[must_use]
    pub fn rank(&self, name: &[u8]) -> Option<usize> {
        let idx = *self.map.get(name)?;
        
        // Count elements to the left
        let mut rank = 0;
        let mut current = idx;

        // Count left subtree
        if let Some(node) = self.tree.get(current) {
            rank += node.left;
        }

        // Traverse up, adding counts when coming from right
        while let Some(node) = self.tree.get(current) {
            if node.parent == usize::MAX {
                break;
            }

            let parent = self.tree.get(node.parent)?;
            if parent.right == current {
                // Coming from right, add parent + left subtree
                rank += 1;
                rank += parent.left;
            }

            current = node.parent;
        }

        Some(rank)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.tree = AvlTree::new();
        self.map.clear();
    }
}

impl Default for ZSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_score() {
        let mut zset = ZSet::new();
        
        assert!(zset.add(b"alice".to_vec(), 100.0));
        assert_eq!(zset.score(b"alice"), Some(100.0));
        assert_eq!(zset.len(), 1);
    }

    #[test]
    fn test_update_score() {
        let mut zset = ZSet::new();
        
        zset.add(b"alice".to_vec(), 100.0);
        assert!(!zset.add(b"alice".to_vec(), 200.0)); // Update
        assert_eq!(zset.score(b"alice"), Some(200.0));
        assert_eq!(zset.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut zset = ZSet::new();
        
        zset.add(b"alice".to_vec(), 100.0);
        assert!(zset.remove(b"alice"));
        assert_eq!(zset.score(b"alice"), None);
        assert_eq!(zset.len(), 0);
    }

    #[test]
    fn test_query() {
        let mut zset = ZSet::new();
        
        zset.add(b"a".to_vec(), 10.0);
        zset.add(b"b".to_vec(), 20.0);
        zset.add(b"c".to_vec(), 30.0);
        zset.add(b"d".to_vec(), 40.0);

        let results = zset.query(15.0, 0, 10);
        assert_eq!(results.len(), 3); // b, c, d
        assert_eq!(results[0].1, 20.0);
    }

    #[test]
    fn test_query_with_offset() {
        let mut zset = ZSet::new();
        
        for i in 0..10 {
            zset.add(format!("key{}", i).into_bytes(), f64::from(i * 10));
        }

        let results = zset.query(0.0, 2, 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].1, 20.0); // Third element
    }

    #[test]
    fn test_sorted_order() {
        let mut zset = ZSet::new();
        
        // Insert in random order
        zset.add(b"charlie".to_vec(), 30.0);
        zset.add(b"alice".to_vec(), 10.0);
        zset.add(b"bob".to_vec(), 20.0);

        let results = zset.query(0.0, 0, 10);
        assert_eq!(results[0].0, b"alice");
        assert_eq!(results[1].0, b"bob");
        assert_eq!(results[2].0, b"charlie");
    }
}