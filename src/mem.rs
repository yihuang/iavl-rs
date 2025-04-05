use std::collections::BTreeMap;
use std::ops::RangeBounds;

use super::types::KVStore;

#[derive(Default)]
pub struct MemTree {
    pub tree: BTreeMap<Vec<u8>, Vec<u8>>,
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

    fn range<R>(&self, bounds: R) -> impl DoubleEndedIterator<Item = (&[u8], &[u8])>
    where
        R: RangeBounds<Vec<u8>>,
    {
        self.tree
            .range(bounds)
            .map(|(k, v)| (k.as_slice(), v.as_slice()))
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

        let result = tree.range(..).collect::<Vec<_>>();
        assert_eq!(
            result,
            vec![
                (b"key1".as_ref(), b"value1".as_ref()),
                (b"key2".as_ref(), b"value2".as_ref()),
                (b"key3".as_ref(), b"value3".as_ref())
            ]
        );

        let result = tree.range(b"key2".to_vec()..).collect::<Vec<_>>();
        assert_eq!(
            result,
            vec![
                (b"key2".as_ref(), b"value2".as_ref()),
                (b"key3".as_ref(), b"value3".as_ref())
            ]
        );

        let result = tree.range(b"key2".to_vec()..).rev().collect::<Vec<_>>();
        assert_eq!(
            result,
            vec![
                (b"key3".as_ref(), b"value3".as_ref()),
                (b"key2".as_ref(), b"value2".as_ref()),
            ]
        );
    }
}
