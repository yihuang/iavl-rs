use super::types::KVStore;
use std::collections::{btree_map::Iter, BTreeMap};

pub struct MemTree {
    pub tree: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl Default for MemTree {
    fn default() -> Self {
        Self::new()
    }
}

impl MemTree {
    pub fn new() -> Self {
        Self {
            tree: BTreeMap::new(),
        }
    }
}

impl KVStore for MemTree {
    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.tree.get(key).map(|v| v.as_slice())
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.tree.insert(key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        self.tree.remove(key);
    }

    fn iter(&self) -> impl Iterator<Item = (&[u8], &[u8])> {
        MemTreeIterator {
            inner: self.tree.iter(),
        }
    }
}

// 新增：MemTree 的迭代器实现
pub struct MemTreeIterator<'a> {
    inner: Iter<'a, Vec<u8>, Vec<u8>>,
}

impl<'a> Iterator for MemTreeIterator<'a> {
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (k.as_slice(), v.as_slice()))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_mem_tree() {
        use super::KVStore;
        use super::MemTree;

        let mut tree = MemTree::new();
        tree.set(b"key1".to_vec(), b"value1".to_vec());
        tree.set(b"key2".to_vec(), b"value2".to_vec());
        tree.set(b"key3".to_vec(), b"value3".to_vec());

        assert_eq!(tree.get(b"key1"), Some(b"value1".as_ref()));
        assert_eq!(tree.get(b"key2"), Some(b"value2".as_ref()));
        assert_eq!(tree.get(b"key3"), Some(b"value3".as_ref()));

        tree.remove(b"key2");
        assert_eq!(tree.get(b"key2"), None);
    }

    #[test]
    fn test_iterator() {
        use super::KVStore;
        use super::MemTree;

        let mut tree = MemTree::new();
        tree.set(b"key1".to_vec(), b"value1".to_vec());
        tree.set(b"key2".to_vec(), b"value2".to_vec());
        tree.set(b"key3".to_vec(), b"value3".to_vec());

        let mut iter = tree.iter();
        assert_eq!(iter.next(), Some((b"key1".as_ref(), b"value1".as_ref())));
        assert_eq!(iter.next(), Some((b"key2".as_ref(), b"value2".as_ref())));
        assert_eq!(iter.next(), Some((b"key3".as_ref(), b"value3".as_ref())));
        assert_eq!(iter.next(), None);
    }
}
