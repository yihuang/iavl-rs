use std::collections::BTreeMap;
use std::ops::RangeBounds;

use super::{KVStore, MergeIter};

pub struct Overlay<S> {
    pub parent: Box<S>,

    // use `Option` as value to represent deletion(tomestone).
    pub tree: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

impl<S: KVStore> Overlay<S> {
    pub fn new(parent: Box<S>) -> Self {
        Self {
            parent,
            tree: BTreeMap::new(),
        }
    }

    // flush flushes all the changes to the parent store.
    pub fn flush(&mut self) {
        for (key, value) in std::mem::take(&mut self.tree).into_iter() {
            match value {
                Some(value) => self.parent.set(key, value),
                None => self.parent.remove(&key),
            }
        }
    }
}

impl<S: KVStore> KVStore for Overlay<S> {
    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        match self.tree.get(key) {
            Some(value) => value.as_deref(),
            None => self.parent.get(key),
        }
    }

    fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.tree.insert(key, Some(value));
    }

    fn remove(&mut self, key: &[u8]) {
        self.tree.insert(key.to_vec(), None);
    }

    fn range<R>(&self, bounds: R) -> impl DoubleEndedIterator<Item = (&[u8], &[u8])>
    where
        R: RangeBounds<Vec<u8>> + Clone,
    {
        MergeIter::new(
            self.tree
                .range(bounds.clone())
                .map(|(k, v)| (k.as_slice(), v.as_deref())),
            self.parent.range(bounds),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemTree;

    #[test]
    fn test_overlay() {
        let mut parent = Box::new(MemTree::new());
        parent.set(b"removed".to_vec(), b"removed".to_vec());

        let mut overlay = Overlay::new(parent);
        assert_eq!(overlay.get(b"removed"), Some(b"removed" as &[u8]));

        overlay.set(b"key1".to_vec(), b"value1".to_vec());
        overlay.remove(b"removed");

        assert_eq!(overlay.get(b"key1"), Some(b"value1" as &[u8]));
        assert_eq!(overlay.get(b"removed"), None);

        overlay.flush();
        assert_eq!(overlay.parent.get(b"key1"), Some(b"value1" as &[u8]));
        assert_eq!(overlay.parent.get(b"removed"), None);
    }

    #[test]
    fn test_overlay_range() {
        let mut parent = Box::new(MemTree::new());
        parent.set(b"key1".to_vec(), b"value1".to_vec());
        parent.set(b"key2".to_vec(), b"value2".to_vec());
        parent.set(b"key3".to_vec(), b"value3".to_vec());
        parent.set(b"key4".to_vec(), b"value4".to_vec());

        let mut overlay = Overlay::new(parent);
        overlay.set(b"key2".to_vec(), b"new_value2".to_vec());
        overlay.remove(b"key3");

        assert_eq!(
            overlay.range(..).collect::<Vec<_>>(),
            vec![
                (b"key1" as &[u8], b"value1" as &[u8]),
                (b"key2" as &[u8], b"new_value2" as &[u8]),
                (b"key4" as &[u8], b"value4" as &[u8]),
            ]
        );

        assert_eq!(
            overlay.range(b"key2".to_vec()..).collect::<Vec<_>>(),
            vec![
                (b"key2" as &[u8], b"new_value2" as &[u8]),
                (b"key4" as &[u8], b"value4" as &[u8]),
            ]
        );

        assert_eq!(
            overlay.range(b"key2".to_vec()..).rev().collect::<Vec<_>>(),
            vec![
                (b"key4" as &[u8], b"value4" as &[u8]),
                (b"key2" as &[u8], b"new_value2" as &[u8]),
            ]
        );
    }
}
