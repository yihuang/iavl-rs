use crypto_common::Output;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::sync::LazyLock;

use super::node::Node;
use super::types::KVStore;

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

    pub fn root_hash(&mut self) -> &Output<Sha256> {
        self.root.as_mut().map_or(&EMPTY_HASH, |n| n.update_hash())
    }

    pub fn save_version(&mut self) -> &Output<Sha256> {
        self.version += 1;
        self.root_hash()
    }

    pub fn get_by_index(&self, index: u64) -> Option<(&[u8], &[u8])> {
        self.root.as_ref()?.get_by_index(index)
    }

    pub fn get_with_index(&self, key: &[u8]) -> (Option<&[u8]>, u64) {
        match self.root.as_ref() {
            Some(root) => root.get_with_index(key),
            None => (None, 0),
        }
    }
}

impl KVStore for IAVLTree {
    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.root.as_ref()?.get_with_index(key).0
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        if let Some(root) = self.root.take() {
            let (node, _) = insert_recursive(root, key, value, self.version + 1);
            self.root = Some(node);
        } else {
            self.root = Some(Box::new(Node::leaf(key, value, self.version + 1)));
        }
    }

    fn remove(&mut self, key: &[u8]) {
        if let Some(root) = self.root.take() {
            let (_, root, _) = remove_recursive(root, key, self.version + 1);
            self.root = root;
        }
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

// remove_recursive returns:
// - (false, Some(origNode), None)
//   key not found, nothing changed in subtree
// - (true,  None,           None)
//   leaf is removed, to replace parent with the other leaf or none
// - (true,  Some(new node), None))
//   subtree changed, don't update branch key
// - (true,  Some(new node), Some(newKey))
//   subtree changed, update branch key
fn remove_recursive(
    mut node: Box<Node>,
    key: &[u8],
    version: u64,
) -> (bool, Option<Box<Node>>, Option<Vec<u8>>) {
    if node.is_leaf() {
        if key == node.key {
            (true, None, None)
        } else {
            (false, Some(node), None)
        }
    } else if key < &node.key {
        let (found, new_left, new_key) = remove_recursive(node.left.take().unwrap(), key, version);
        if !found {
            node.left = new_left;
            return (false, Some(node), None);
        }

        if let Some(new_left) = new_left {
            node.mutate(version);
            node.left = Some(new_left);
            node.update_height_size();
            node = balance(node, version);
            (true, Some(node), new_key)
        } else {
            (true, node.right, Some(node.key))
        }
    } else {
        let (found, new_right, new_key) =
            remove_recursive(node.right.take().unwrap(), key, version);
        if !found {
            node.right = new_right;
            return (false, Some(node), None);
        }

        if let Some(new_right) = new_right {
            node.mutate(version);
            node.right = Some(new_right);
            if let Some(new_key) = new_key {
                node.key = new_key;
            }
            node.update_height_size();
            node = balance(node, version);
            (true, Some(node), None)
        } else {
            (true, node.left, None)
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
    use hexhex::hex_literal;

    #[test]
    fn test_basic_operations() {
        let mut tree = IAVLTree::new();
        assert_eq!(tree.root_hash(), &*EMPTY_HASH);

        tree.set(b"key1".to_vec(), b"value1".to_vec());
        assert_eq!(tree.get(b"key1"), Some(b"value1".as_ref()));
        let root1 = tree.save_version().to_vec();

        tree.set(b"key2".to_vec(), b"value2".to_vec());
        assert_eq!(tree.get(b"key2"), Some(b"value2".as_ref()));

        let root2 = tree.save_version().to_vec();
        assert_ne!(root1, root2);

        tree.remove(b"key2");
        assert_eq!(tree.get(b"key2"), None);

        let root3 = tree.save_version().to_vec();
        assert_eq!(root1, root3);
    }

    #[test]
    fn test_update_value() {
        let mut tree = IAVLTree::new();
        tree.set(b"key".to_vec(), b"value1".to_vec());
        let hash1 = tree.save_version().to_vec();

        tree.set(b"key".to_vec(), b"value2".to_vec());
        let hash2 = tree.save_version().to_vec();

        assert_ne!(hash1, hash2);
        assert_eq!(tree.get(b"key"), Some(b"value2".as_ref()));
    }

    #[test]
    fn test_key_index() {
        let mut tree = IAVLTree::new();
        for i in 0u32..10 {
            tree.set(i.to_be_bytes().to_vec(), i.to_be_bytes().to_vec());
        }
        tree.save_version();

        for i in 0u32..10 {
            let (key, value) = tree.get_by_index(i.into()).expect("value exists");
            assert_eq!(key, &i.to_be_bytes());
            assert_eq!(value, &i.to_be_bytes());
            let (value, index) = tree.get_with_index(&i.to_be_bytes());
            assert_eq!(value.expect("value exists"), &i.to_be_bytes());
            assert_eq!(index, i.into());
        }
    }

    struct KVPair {
        delete: bool,
        key: Vec<u8>,
        value: Vec<u8>,
    }

    fn delete(key: &[u8]) -> KVPair {
        KVPair {
            delete: true,
            key: key.to_vec(),
            value: Vec::new(),
        }
    }

    fn insert(key: &[u8], value: &[u8]) -> KVPair {
        KVPair {
            delete: false,
            key: key.to_vec(),
            value: value.to_vec(),
        }
    }

    #[test]
    fn test_hash_vector() {
        let ref_hashes = [
            hex_literal!("6032661ab0d201132db7a8fa1da6a0afe427e6278bd122c301197680ab79ca02"),
            hex_literal!("457d81f933f53e5cfb90d813b84981aa2604d69939e10c94304d18287ded31f7"),
            hex_literal!("c7ab142752add0374992261536e502851ce555d243270d3c3c6b77cf31b7945d"),
            hex_literal!("e54da9407cbca3570d04ad5c3296056a0726467cb06272ffd8ef1b4ae87fb99d"),
            hex_literal!("8b04490800d6b54fa569715a754b5fafe24fd720f677cab819394cf7ccf8cdec"),
            hex_literal!("38abd5268374923e6727b14ac5a9bb6611e591d7e316d0a612904062f244e72f"),
            hex_literal!("d91cf6388eeff3204474bb07b853ab0d7d39163912ac1e610e92f9b178c76922"),
        ];
        let ref_hashes_initial_version = [
            hex_literal!("053bb7cf59993f3c4f3c95f76037bb597cfe2fe662a7c5a49ecb06acb3eaf672"),
            hex_literal!("ac4d11d9d685c38401059dcc097b3780df1d34280a6d291d729d7e98f41f07c6"),
            hex_literal!("49d572c2cf09b4de3167c3d61a38137b3b2f0caf2dfc431ef79ea7ca8e0d701e"),
            hex_literal!("e13b7fdbf9ddc7537c70f00f2f8477e422a1a6e12768cf03edd2b329a310e875"),
            hex_literal!("30f964138a9c8e4b8ee933c57982a58c99ec31948f2dbb3b0eafbfac2d578c13"),
            hex_literal!("c379fbf3f3e83d0a92cfb7c6fc43566a8d1aad25a00de8cae61e7685176cb8bf"),
            hex_literal!("5712608bf5ccb32dd3231bc6e2fc2df427083eca892c7e1766312190fc3ef715"),
        ];

        let mut changesets = vec![
            vec![insert(b"hello", b"world")],
            vec![insert(b"hello", b"world1"), insert(b"hello1", b"world1")],
            vec![insert(b"hello2", b"world1"), insert(b"hello3", b"world1")],
        ];

        let mut changes = vec![];
        for i in 0..1 {
            changes.push(insert(format!("hello{:02}", i).as_bytes(), b"world1"))
        }
        changesets.push(changes);

        changesets.push(vec![delete(b"hello"), delete(b"hello19")]);

        let mut changes = vec![];
        for i in 0..21 {
            changes.push(insert(format!("aello{:02}", i).as_bytes(), b"world1"));
        }
        changesets.push(changes);

        let mut changes = vec![];
        for i in 0..21 {
            changes.push(delete(format!("aello{:02}", i).as_bytes()));
        }
        for i in 0..19 {
            changes.push(delete(format!("hello{:02}", i).as_bytes()));
        }
        changesets.push(changes);

        let mut tree = IAVLTree::new();
        let mut tree_initial_version = IAVLTree::new();
        tree_initial_version.version = 100 - 1;
        for (i, changes) in changesets.iter().enumerate() {
            for change in changes {
                if change.delete {
                    tree.remove(&change.key);
                    tree_initial_version.remove(&change.key);
                } else {
                    tree.set(change.key.clone(), change.value.clone());
                    tree_initial_version.set(change.key.clone(), change.value.clone());
                }
            }
            assert_eq!(tree.save_version().to_vec(), ref_hashes[i]);
            assert_eq!(
                tree_initial_version.save_version().to_vec(),
                ref_hashes_initial_version[i]
            );
        }
    }
}
