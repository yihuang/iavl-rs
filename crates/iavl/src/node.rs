use crypto_common::Output;
use integer_encoding::VarInt;
use sha2::{Digest, Sha256};
use std::cmp::{self, Ordering};

#[derive(Debug, Clone)]
pub struct Node {
    pub height: u8,
    pub size: u64,
    pub version: u64,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub left: Option<Box<Node>>,
    pub right: Option<Box<Node>>,
    pub hash: Option<Output<Sha256>>,
}

impl Node {
    // leaf create a leaf node
    pub fn leaf(key: Vec<u8>, value: Vec<u8>, version: u64) -> Self {
        Node {
            height: 0,
            size: 1,
            version,
            key,
            value,
            left: None,
            right: None,
            hash: None,
        }
    }

    // branch_bottom creates a height 1 node with two leafs as children
    pub fn branch_bottom(left: Box<Node>, right: Box<Node>, version: u64) -> Self {
        Node {
            height: 1,
            size: 2,
            version,
            key: right.key.clone(),
            left: Some(left),
            right: Some(right),
            value: Vec::new(),
            hash: None,
        }
    }

    pub fn update_height_size(&mut self) {
        let left = self.left.as_ref().unwrap();
        let right = self.right.as_ref().unwrap();
        self.height = cmp::max(left.height, right.height) + 1;
        self.size = left.size + right.size;
    }

    pub fn is_leaf(&self) -> bool {
        self.height == 0
    }

    pub fn balance_factor(&self) -> i32 {
        let left_height = self.left.as_ref().map(|n| n.height).unwrap_or(0) as i32;
        let right_height = self.right.as_ref().map(|n| n.height).unwrap_or(0) as i32;
        left_height - right_height
    }

    // mutate prepares in-place mutation for the node, it clears the hash and update version.
    pub fn mutate(&mut self, version: u64) {
        self.version = version;
        self.hash = None;
    }

    pub fn update_hash(&mut self) -> &Output<Sha256> {
        if self.hash.is_none() {
            self.hash = Some(hash_node(self));
        };

        // SAFETY: a `None` variant for `self` would have been replaced by a `Some`
        // variant in the code above.
        unsafe { self.hash.as_ref().unwrap_unchecked() }
    }

    // get_with_index returns the value and the index of the key in the tree.
    pub fn get_with_index(&self, key: &[u8]) -> (Option<&[u8]>, u64) {
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
    pub fn get_by_index(&self, index: u64) -> Option<(&[u8], &[u8])> {
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

fn hash_node(node: &mut Node) -> Output<Sha256> {
    let mut buf = [0u8; 8];
    let mut hasher = Sha256::new();

    {
        let n = (node.height as i64).encode_var(&mut buf);
        hasher.update(&buf[..n]);
    }

    {
        let n = (node.size as i64).encode_var(&mut buf);
        hasher.update(&buf[..n]);
    }

    {
        let n = (node.version as i64).encode_var(&mut buf);
        hasher.update(&buf[..n]);
    }

    if node.is_leaf() {
        hash_bytes(&mut hasher, &node.key);
        hash_bytes(&mut hasher, &Sha256::digest(&node.value));
    } else {
        let left_hash = node.left.as_mut().unwrap().update_hash();
        hash_bytes(&mut hasher, left_hash);

        let right_hash = node.right.as_mut().unwrap().update_hash();
        hash_bytes(&mut hasher, right_hash);
    }

    hasher.finalize()
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    let mut buf = [0u8; 8];
    let n = bytes.len().encode_var(&mut buf);
    hasher.update(&buf[..n]);
    hasher.update(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use hexhex::hex_literal;

    #[test]
    fn test_hash() {
        let node1 = Box::new(Node::leaf(b"key1".to_vec(), b"value1".to_vec(), 0));
        let node2 = Box::new(Node::leaf(b"key2".to_vec(), b"value2".to_vec(), 0));
        let mut node3 = Node::branch_bottom(node1.clone(), node2.clone(), 1);
        node3.update_hash();

        assert_eq!(
            node3.left.unwrap().hash.as_deref().expect(""),
            hex_literal!("bffb733c4d36d48583fca5d1d088fcdf2682d2eea77c864d2da00cda56a832ec")
        );
        assert_eq!(
            node3.right.unwrap().hash.as_deref().expect(""),
            hex_literal!("915cdad41f11fc68bc8a9ff3c47c3050c06be086a382d7487cb4a4981dad5ef9")
        );
        assert_eq!(
            node3.hash.as_deref().expect(""),
            hex_literal!("d315e38c4e0093b72123fe70733a733a3fc185dfbce72357595738672ba984f2")
        );
    }
}
