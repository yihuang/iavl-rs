use sha2::{Digest, Sha256};
use std::cmp::{self, Ordering};
use std::sync::LazyLock;

static EMPTY_HASH: LazyLock<Vec<u8>> = LazyLock::new(|| Sha256::digest(b"").to_vec());

#[derive(Debug, Clone)]
struct Node {
    height: u8,
    size: u64,
    version: u64,
    key: Vec<u8>,
    value: Vec<u8>,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
    hash: Vec<u8>,
}

impl Node {
    // leaf create a leaf node
    fn leaf(key: Vec<u8>, value: Vec<u8>, version: u64) -> Self {
        Node {
            height: 0,
            size: 1,
            version,
            key,
            value,
            left: None,
            right: None,
            hash: Vec::new(),
        }
    }

    // branch_bottom creates a height 1 node with two leafs as children
    fn branch_bottom(left: Box<Node>, right: Box<Node>, version: u64) -> Self {
        Node {
            height: 1,
            size: 2,
            version,
            key: right.key.clone(),
            left: Some(left),
            right: Some(right),
            value: Vec::new(),
            hash: Vec::new(),
        }
    }

    fn update_height_size(&mut self) {
        let left = self.left.as_ref().unwrap();
        let right = self.right.as_ref().unwrap();
        self.height = cmp::max(left.height, right.height) + 1;
        self.size = left.size + right.size;
    }

    fn is_leaf(&self) -> bool {
        self.height == 0
    }

    fn balance_factor(&self) -> i32 {
        let left_height = self.left.as_ref().map(|n| n.height).unwrap_or(0) as i32;
        let right_height = self.right.as_ref().map(|n| n.height).unwrap_or(0) as i32;
        left_height - right_height
    }

    // mutate prepares in-place mutation for the node, it clears the hash and update version.
    fn mutate(&mut self, version: u64) {
        self.version = version;
        self.hash.clear();
    }

    fn update_hash(&mut self) -> &[u8] {
        if !self.hash.is_empty() {
            return &self.hash;
        }

        let mut hasher = Sha256::new();
        hasher.update(self.height.to_be_bytes());
        hasher.update(self.size.to_be_bytes());
        hasher.update(self.version.to_be_bytes());
        hasher.update(&self.key);
        hasher.update(&self.value);

        let empty = Vec::new();
        let left_hash = self.left.as_ref().map_or(&empty, |n| &n.hash);
        hasher.update(left_hash);

        let right_hash = self.right.as_ref().map_or(&empty, |n| &n.hash);
        hasher.update(right_hash);

        self.hash = hasher.finalize().to_vec();

        &self.hash
    }

    // get_with_index returns the value and the index of the key in the tree.
    fn get_with_index(&self, key: &[u8]) -> (Option<&[u8]>, u64) {
        if self.is_leaf() {
            match self.key.as_slice().cmp(key) {
                Ordering::Less => (None, 1),
                Ordering::Greater => (None, 0),
                Ordering::Equal => (Some(&self.value), 0),
            }
        } else if key.cmp(&self.key) == Ordering::Less {
            self.left.as_ref().unwrap().get_with_index(key)
        } else {
            let right = self.right.as_ref().unwrap();
            let (value, index) = right.get_with_index(key);
            (value, index + self.size - right.size)
        }
    }

    // get_by_index returns the key and value at the given index.
    fn get_by_index(&self, index: u64) -> Option<(&[u8], &[u8])> {
        if self.is_leaf() {
            if index == 0 {
                return Some((&self.key, &self.value));
            }
            return None;
        }

        let left = self.left.as_ref().unwrap();
        if index < left.size {
            return left.get_by_index(index);
        }

        self.right.as_ref().unwrap().get_by_index(index - left.size)
    }
}

pub struct IAVLTree {
    root: Option<Box<Node>>,
    version: u64,
}

impl Default for IAVLTree {
    fn default() -> Self {
        IAVLTree::new()
    }
}

impl IAVLTree {
    pub fn new() -> Self {
        IAVLTree {
            root: None,
            version: 0,
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let (root, _) = insert_recursive(self.root.take(), key, value, self.version);
        self.root = Some(root);
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.root.as_ref()?.get_with_index(key).0
    }

    pub fn root_hash(&mut self) -> &[u8] {
        self.root.as_mut().map_or(&EMPTY_HASH, |n| n.update_hash())
    }

    pub fn save_version(&mut self) -> &[u8] {
        self.version += 1;
        self.root_hash()
    }
}

// it returns if it's an update or insertion, if update, the tree height and balance is not changed.
fn insert_recursive(
    node: Option<Box<Node>>,
    key: Vec<u8>,
    value: Vec<u8>,
    version: u64,
) -> (Box<Node>, bool) {
    match node {
        None => (Box::new(Node::leaf(key, value, version)), true),
        Some(mut n) if n.is_leaf() => match key.cmp(&n.key) {
            Ordering::Less => (
                Box::new(Node::branch_bottom(
                    Box::new(Node::leaf(key, value, version)),
                    n,
                    version,
                )),
                false,
            ),
            Ordering::Greater => (
                Box::new(Node::branch_bottom(
                    n,
                    Box::new(Node::leaf(key, value, version)),
                    version,
                )),
                false,
            ),
            Ordering::Equal => {
                n.mutate(version);
                n.value = value;
                (n, true)
            }
        },
        Some(mut n) => {
            n.mutate(version);
            let updated = if key.cmp(&n.key) == Ordering::Less {
                let (n1, updated) = insert_recursive(n.left, key, value, version);
                n.left = Some(n1);
                updated
            } else {
                let (n1, updated) = insert_recursive(n.right, key, value, version);
                n.right = Some(n1);
                updated
            };

            if !updated {
                n.update_height_size();
                n = balance(n, version);
            }

            (n, updated)
        }
    }
}

fn balance(mut node: Box<Node>, version: u64) -> Box<Node> {
    let balance_factor = node.balance_factor();

    if balance_factor > 1 {
        node.mutate(version);
        if node.left.as_ref().unwrap().balance_factor() >= 0 {
            rotate_right(node, version)
        } else {
            node.left = node.left.map(|mut n| {
                n.mutate(version);
                rotate_left(n, version)
            });
            rotate_right(node, version)
        }
    } else if balance_factor < -1 {
        node.mutate(version);
        if node.right.as_ref().unwrap().balance_factor() <= 0 {
            rotate_left(node, version)
        } else {
            let right = node.right.take().unwrap();
            node.right = Some(rotate_right(right, version));
            rotate_left(node, version)
        }
    } else {
        node
    }
}

fn rotate_right(mut a: Box<Node>, version: u64) -> Box<Node> {
    let mut b = a.left.take().unwrap();
    let t2 = b.right.take();

    a.left = t2;
    a.update_height_size();

    b.mutate(version);
    b.right = Some(a);
    b.update_height_size();

    b
}

fn rotate_left(mut a: Box<Node>, version: u64) -> Box<Node> {
    let mut b = a.right.take().unwrap();
    let t2 = b.left.take();

    a.right = t2;
    a.update_height_size();

    b.mutate(version);
    b.left = Some(a);
    b.update_height_size();

    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut tree = IAVLTree::new();
        assert_eq!(tree.root_hash(), &*EMPTY_HASH);

        tree.insert(b"key1".to_vec(), b"value1".to_vec());
        assert_eq!(tree.get(b"key1"), Some(b"value1".as_ref()));
        let root1 = tree.save_version().to_vec();

        tree.insert(b"key2".to_vec(), b"value2".to_vec());
        assert_eq!(tree.get(b"key2"), Some(b"value2".as_ref()));
        let root2 = tree.save_version().to_vec();

        assert_ne!(root1, root2);
    }

    #[test]
    fn test_update_value() {
        let mut tree = IAVLTree::new();
        tree.insert(b"key".to_vec(), b"value1".to_vec());
        let hash1 = tree.save_version().to_vec();

        tree.insert(b"key".to_vec(), b"value2".to_vec());
        let hash2 = tree.save_version().to_vec();

        assert_ne!(hash1, hash2);
        assert_eq!(tree.get(b"key"), Some(b"value2".as_ref()));
    }
}
