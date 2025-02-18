use crypto_common::Output;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::sync::LazyLock;

use super::node::Node;

static EMPTY_HASH: LazyLock<Output<Sha256>> = LazyLock::new(|| Sha256::digest(b""));

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
        if let Some(root) = self.root.take() {
            let (node, _) = insert_recursive(root, key, value, self.version);
            self.root = Some(node);
        } else {
            self.root = Some(Box::new(Node::leaf(key, value, self.version)));
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.root.as_ref()?.get_with_index(key).0
    }

    pub fn root_hash(&mut self) -> &Output<Sha256> {
        self.root.as_mut().map_or(&EMPTY_HASH, |n| n.update_hash())
    }

    pub fn save_version(&mut self) -> &Output<Sha256> {
        self.version += 1;
        self.root_hash()
    }
}

// it returns if it's an update or insertion, if update, the tree height and balance is not changed.
fn insert_recursive(
    mut node: Box<Node>,
    key: Vec<u8>,
    value: Vec<u8>,
    version: u64,
) -> (Box<Node>, bool) {
    if node.is_leaf() {
        match key.cmp(&node.key) {
            Ordering::Less => (
                Box::new(Node::branch_bottom(
                    Box::new(Node::leaf(key, value, version)),
                    node,
                    version,
                )),
                false,
            ),
            Ordering::Greater => (
                Box::new(Node::branch_bottom(
                    node,
                    Box::new(Node::leaf(key, value, version)),
                    version,
                )),
                false,
            ),
            Ordering::Equal => {
                node.mutate(version);
                node.value = value;
                (node, true)
            }
        }
    } else {
        node.mutate(version);
        let updated = if key.cmp(&node.key) == Ordering::Less {
            let (n1, updated) = insert_recursive(node.left.unwrap(), key, value, version);
            node.left = Some(n1);
            updated
        } else {
            let (n1, updated) = insert_recursive(node.right.unwrap(), key, value, version);
            node.right = Some(n1);
            updated
        };

        if !updated {
            node.update_height_size();
            node = balance(node, version);
        }

        (node, updated)
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

    #[test]
    fn test_key_index() {
        let mut tree = IAVLTree::new();
        for i in 0u32..10 {
            tree.insert(i.to_be_bytes().to_vec(), i.to_be_bytes().to_vec());
        }
        tree.save_version();

        let root = tree.root.expect("root non empty");
        for i in 0u32..10 {
            let (key, value) = root.get_by_index(i.into()).expect("value exists");
            assert_eq!(key, &i.to_be_bytes());
            assert_eq!(value, &i.to_be_bytes());
            let (value, index) = root.get_with_index(&i.to_be_bytes());
            assert_eq!(value.expect("value exists"), &i.to_be_bytes());
            assert_eq!(index, i.into());
        }
    }
}
