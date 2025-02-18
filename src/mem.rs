use super::types::KVStore;
use std::collections::BTreeMap;

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
}
