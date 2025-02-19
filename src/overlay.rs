use std::collections::BTreeMap;

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

    fn iter(&self) -> impl Iterator<Item = (&[u8], &[u8])> {
        MergeIter::new(
            self.tree.iter().map(|(k, v)| (k.as_slice(), v.as_deref())),
            self.parent.iter(),
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
    fn test_overlay_iter() {
        let mut parent = Box::new(MemTree::new());
        parent.set(b"key1".to_vec(), b"value1".to_vec());
        parent.set(b"key2".to_vec(), b"value2".to_vec());
        parent.set(b"key3".to_vec(), b"value3".to_vec());

        let mut overlay = Overlay::new(parent);
        overlay.set(b"key2".to_vec(), b"new_value2".to_vec());
        overlay.remove(b"key3");

        let mut iter = overlay.iter();
        assert_eq!(iter.next(), Some((b"key1" as &[u8], b"value1" as &[u8])));
        assert_eq!(
            iter.next(),
            Some((b"key2" as &[u8], b"new_value2" as &[u8]))
        );
        assert_eq!(iter.next(), None);
    }
}
