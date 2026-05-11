
use std::cmp::Ordering;

const INVALID_IDX: usize = usize::MAX;

#[derive(Debug, Clone)]
pub struct Node<T> {
    pub data: T,
    pub depth: u32,
    pub cnt: u32,
    pub left: usize,
    pub right: usize,
    pub parent: usize,
}

/// Arena-based AVL tree.
/// All nodes are stored in a Vec, and we use indices instead of pointers.
pub struct AvlTree<T> {
    nodes: Vec<Node<T>>,
    root: usize,
    free_list: Vec<usize>, // Reuse deleted node slots
}

impl<T> AvlTree<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: INVALID_IDX,
            free_list: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root == INVALID_IDX
    }

    #[must_use]
    pub fn size(&self) -> u32 {
        self.cnt(self.root)
    }

    #[must_use]
    pub fn root(&self) -> usize {
        self.root
    }

    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&Node<T>> {
        if idx < self.nodes.len() {
            Some(&self.nodes[idx])
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Node<T>> {
        if idx < self.nodes.len() {
            Some(&mut self.nodes[idx])
        } else {
            None
        }
    }

    fn depth(&self, idx: usize) -> u32 {
        if idx == INVALID_IDX {
            0
        } else {
            self.nodes[idx].depth
        }
    }

    fn cnt(&self, idx: usize) -> u32 {
        if idx == INVALID_IDX {
            0
        } else {
            self.nodes[idx].cnt
        }
    }

    fn update(&mut self, idx: usize) {
        if idx == INVALID_IDX {
            return;
        }
        let left = self.nodes[idx].left;
        let right = self.nodes[idx].right;
        self.nodes[idx].depth = 1 + self.depth(left).max(self.depth(right));
        self.nodes[idx].cnt = 1 + self.cnt(left) + self.cnt(right);
    }

    fn rot_left(&mut self, idx: usize) -> usize {
        let new_root = self.nodes[idx].right;
        let new_left_child = self.nodes[new_root].left;

        // Update parent pointers
        if new_left_child != INVALID_IDX {
            self.nodes[new_left_child].parent = idx;
        }

        self.nodes[idx].right = new_left_child;
        self.nodes[new_root].left = idx;
        self.nodes[new_root].parent = self.nodes[idx].parent;
        self.nodes[idx].parent = new_root;

        self.update(idx);
        self.update(new_root);

        new_root
    }

    fn rot_right(&mut self, idx: usize) -> usize {
        let new_root = self.nodes[idx].left;
        let new_right_child = self.nodes[new_root].right;

        if new_right_child != INVALID_IDX {
            self.nodes[new_right_child].parent = idx;
        }

        self.nodes[idx].left = new_right_child;
        self.nodes[new_root].right = idx;
        self.nodes[new_root].parent = self.nodes[idx].parent;
        self.nodes[idx].parent = new_root;

        self.update(idx);
        self.update(new_root);

        new_root
    }

    fn fix_left(&mut self, idx: usize) -> usize {
        let left = self.nodes[idx].left;
        let left_left = self.nodes[left].left;
        let left_right = self.nodes[left].right;

        if self.depth(left_left) < self.depth(left_right) {
            self.nodes[idx].left = self.rot_left(left);
        }
        self.rot_right(idx)
    }

    fn fix_right(&mut self, idx: usize) -> usize {
        let right = self.nodes[idx].right;
        let right_left = self.nodes[right].left;
        let right_right = self.nodes[right].right;

        if self.depth(right_right) < self.depth(right_left) {
            self.nodes[idx].right = self.rot_right(right);
        }
        self.rot_left(idx)
    }

    pub fn fix(&mut self, mut idx: usize) -> usize {
        loop {
            self.update(idx);

            let l = self.depth(self.nodes[idx].left);
            let r = self.depth(self.nodes[idx].right);

            let parent = self.nodes[idx].parent;
            let is_left_child = if parent != INVALID_IDX {
                self.nodes[parent].left == idx
            } else {
                false
            };

            if l == r + 2 {
                idx = self.fix_left(idx);
            } else if l + 2 == r {
                idx = self.fix_right(idx);
            }

            if parent == INVALID_IDX {
                return idx;
            }

            if is_left_child {
                self.nodes[parent].left = idx;
            } else {
                self.nodes[parent].right = idx;
            }

            idx = parent;
        }
    }

    /// Allocate a new node in the arena
    fn alloc_node(&mut self, data: T) -> usize {
        let idx = if let Some(free_idx) = self.free_list.pop() {
            self.nodes[free_idx] = Node {
                data,
                depth: 1,
                cnt: 1,
                left: INVALID_IDX,
                right: INVALID_IDX,
                parent: INVALID_IDX,
            };
            free_idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Node {
                data,
                depth: 1,
                cnt: 1,
                left: INVALID_IDX,
                right: INVALID_IDX,
                parent: INVALID_IDX,
            });
            idx
        };
        idx
    }

    /// Insert a new element, returns the index of the inserted node
    pub fn insert<F>(&mut self, data: T, cmp: F) -> usize
    where
        F: Fn(&T, &T) -> Ordering,
    {
        let new_idx = self.alloc_node(data);

        if self.root == INVALID_IDX {
            self.root = new_idx;
            return new_idx;
        }

        let mut cur = self.root;
        loop {
            let ordering = cmp(&self.nodes[new_idx].data, &self.nodes[cur].data);
            let next = match ordering {
                Ordering::Less => self.nodes[cur].left,
                _ => self.nodes[cur].right,
            };

            if next == INVALID_IDX {
                // Found insertion point
                match ordering {
                    Ordering::Less => self.nodes[cur].left = new_idx,
                    _ => self.nodes[cur].right = new_idx,
                }
                self.nodes[new_idx].parent = cur;
                break;
            }
            cur = next;
        }

        self.root = self.fix(new_idx);
        new_idx
    }

    /// Delete a node by index
    pub fn delete(&mut self, idx: usize) -> T {
        let data = if self.nodes[idx].right == INVALID_IDX {
            // No right subtree
            let parent = self.nodes[idx].parent;
            let left = self.nodes[idx].left;

            if left != INVALID_IDX {
                self.nodes[left].parent = parent;
            }

            if parent != INVALID_IDX {
                if self.nodes[parent].left == idx {
                    self.nodes[parent].left = left;
                } else {
                    self.nodes[parent].right = left;
                }
                self.root = self.fix(parent);
            } else {
                self.root = left;
            }

            // Extract data before moving the node
            let mut temp_node = Node {
                data: unsafe { std::ptr::read(&self.nodes[0].data) },
                depth: 0,
                cnt: 0,
                left: INVALID_IDX,
                right: INVALID_IDX,
                parent: INVALID_IDX,
            };
            std::mem::swap(&mut self.nodes[idx], &mut temp_node);
            self.free_list.push(idx);
            temp_node.data
        } else {
            // Find in-order successor (leftmost in right subtree)
            let mut victim = self.nodes[idx].right;
            while self.nodes[victim].left != INVALID_IDX {
                victim = self.nodes[victim].left;
            }

            // Delete the victim
            let victim_data = self.delete(victim);

            // Swap victim's data into idx
            let old_data = std::mem::replace(&mut self.nodes[idx].data, victim_data);
            old_data
        };

        data
    }

    /// Find node by offset from a given node
    pub fn offset(&self, mut idx: usize, target_offset: i64) -> Option<usize> {
        let mut pos: i64 = 0;

        while pos != target_offset {
            let right = self.nodes[idx].right;
            let left = self.nodes[idx].left;

            if pos < target_offset && pos + i64::from(self.cnt(right)) >= target_offset {
                idx = right;
                pos += i64::from(self.cnt(self.nodes[idx].left)) + 1;
            } else if pos > target_offset && pos - i64::from(self.cnt(left)) <= target_offset {
                idx = left;
                pos -= i64::from(self.cnt(self.nodes[idx].right)) + 1;
            } else {
                let parent = self.nodes[idx].parent;
                if parent == INVALID_IDX {
                    return None;
                }

                if self.nodes[parent].right == idx {
                    pos -= i64::from(self.cnt(left)) + 1;
                } else {
                    pos += i64::from(self.cnt(right)) + 1;
                }
                idx = parent;
            }
        }

        Some(idx)
    }
}

impl<T> Default for AvlTree<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_size() {
        let mut tree = AvlTree::new();
        assert!(tree.is_empty());

        tree.insert(5, i32::cmp);
        tree.insert(3, i32::cmp);
        tree.insert(7, i32::cmp);

        assert_eq!(tree.size(), 3);
    }

    #[test]
    fn test_balance() {
        let mut tree = AvlTree::new();
        
        // Insert in ascending order (worst case for unbalanced tree)
        for i in 0..10 {
            tree.insert(i, i32::cmp);
        }

        // Check that tree remains balanced (depth should be ~log(n))
        let root_depth = tree.get(tree.root()).unwrap().depth;
        assert!(root_depth <= 5); // log2(10) ≈ 3.3, with balance factor max 4-5
    }

    #[test]
    fn test_delete() {
        let mut tree = AvlTree::new();
        
        let idx1 = tree.insert(5, i32::cmp);
        tree.insert(3, i32::cmp);
        tree.insert(7, i32::cmp);

        assert_eq!(tree.size(), 3);
        
        let val = tree.delete(idx1);
        assert_eq!(val, 5);
        assert_eq!(tree.size(), 2);
    }
}