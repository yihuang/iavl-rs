use sha2::{Digest, Sha256};
use std::cmp::{self, Ordering};

#[derive(Debug, Clone)]
struct Node {
    key: Vec<u8>,
    value: Vec<u8>,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
    height: u32,
    hash: Vec<u8>,
}

impl Node {
    fn new(key: Vec<u8>, value: Vec<u8>) -> Self {
        let mut node = Node {
            key: key.clone(),
            value: value.clone(),
            left: None,
            right: None,
            height: 1,
            hash: Vec::new(),
        };
        node.update_hash();
        node
    }

    fn update_height(&mut self) {
        let left_height = self.left.as_ref().map(|n| n.height).unwrap_or(0);
        let right_height = self.right.as_ref().map(|n| n.height).unwrap_or(0);
        self.height = cmp::max(left_height, right_height) + 1;
    }

    fn balance_factor(&self) -> i32 {
        let left_height = self.left.as_ref().map(|n| n.height).unwrap_or(0) as i32;
        let right_height = self.right.as_ref().map(|n| n.height).unwrap_or(0) as i32;
        left_height - right_height
    }

    fn update_hash(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(self.height.to_be_bytes());
        hasher.update(&self.key);
        hasher.update(&self.value);

        let empty = Vec::new();
        let left_hash = self.left.as_ref().map_or(&empty, |n| &n.hash);
        hasher.update(left_hash);

        let right_hash = self.right.as_ref().map_or(&empty, |n| &n.hash);
        hasher.update(right_hash);

        self.hash = hasher.finalize().to_vec();
    }
}

pub struct IAVLTree {
    root: Option<Box<Node>>,
}

impl Default for IAVLTree {
    fn default() -> Self {
        IAVLTree::new()
    }
}

impl IAVLTree {
    pub fn new() -> Self {
        IAVLTree { root: None }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.root = insert_recursive(self.root.take(), key, value);
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        get_recursive(&self.root, key)
    }

    pub fn root_hash(&self) -> Option<Vec<u8>> {
        self.root.as_ref().map(|n| n.hash.clone())
    }
}

fn insert_recursive(node: Option<Box<Node>>, key: Vec<u8>, value: Vec<u8>) -> Option<Box<Node>> {
    match node {
        None => Some(Box::new(Node::new(key, value))),
        Some(mut n) => {
            match key.cmp(&n.key) {
                Ordering::Less => {
                    n.left = insert_recursive(n.left, key, value);
                }
                Ordering::Greater => {
                    n.right = insert_recursive(n.right, key, value);
                }
                Ordering::Equal => {
                    n.value = value;
                }
            }

            n.update_height();
            let balanced_node = balance(n);
            Some(balanced_node)
        }
    }
}

fn balance(mut node: Box<Node>) -> Box<Node> {
    let balance_factor = node.balance_factor();

    if balance_factor > 1 {
        if node.left.as_ref().unwrap().balance_factor() >= 0 {
            rotate_right(node)
        } else {
            let left = node.left.take().unwrap();
            node.left = Some(rotate_left(left));
            rotate_right(node)
        }
    } else if balance_factor < -1 {
        if node.right.as_ref().unwrap().balance_factor() <= 0 {
            rotate_left(node)
        } else {
            let right = node.right.take().unwrap();
            node.right = Some(rotate_right(right));
            rotate_left(node)
        }
    } else {
        node.update_hash();
        node
    }
}

fn rotate_right(mut a: Box<Node>) -> Box<Node> {
    let mut b = a.left.take().unwrap();
    let t2 = b.right.take();

    a.left = t2;
    a.update_height();
    a.update_hash();

    b.right = Some(a);
    b.update_height();
    b.update_hash();

    b
}

fn rotate_left(mut a: Box<Node>) -> Box<Node> {
    let mut b = a.right.take().unwrap();
    let t2 = b.left.take();

    a.right = t2;
    a.update_height();
    a.update_hash();

    b.left = Some(a);
    b.update_height();
    b.update_hash();

    b
}

fn get_recursive<'a>(node: &'a Option<Box<Node>>, key: &[u8]) -> Option<&'a [u8]> {
    match node {
        None => None,
        Some(n) => match key.cmp(&n.key) {
            cmp::Ordering::Less => get_recursive(&n.left, key),
            cmp::Ordering::Greater => get_recursive(&n.right, key),
            cmp::Ordering::Equal => Some(&n.value),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut tree = IAVLTree::new();
        assert_eq!(tree.root_hash(), None);

        tree.insert(b"key1".to_vec(), b"value1".to_vec());
        assert_eq!(tree.get(b"key1"), Some(b"value1".as_ref()));
        let root1 = tree.root_hash().unwrap();

        tree.insert(b"key2".to_vec(), b"value2".to_vec());
        assert_eq!(tree.get(b"key2"), Some(b"value2".as_ref()));
        let root2 = tree.root_hash().unwrap();

        assert_ne!(root1, root2);
    }

    #[test]
    fn test_update_value() {
        let mut tree = IAVLTree::new();
        tree.insert(b"key".to_vec(), b"value1".to_vec());
        let hash1 = tree.root_hash().unwrap();

        tree.insert(b"key".to_vec(), b"value2".to_vec());
        let hash2 = tree.root_hash().unwrap();

        assert_ne!(hash1, hash2);
        assert_eq!(tree.get(b"key"), Some(b"value2".as_ref()));
    }
}
