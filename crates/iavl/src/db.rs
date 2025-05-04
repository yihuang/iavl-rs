use crypto_common::Output;
use sha2::Sha256;
use std::mem;

use serde::{Deserialize, Serialize};
use walcraft::Wal;

use crate::{types::ChangeItem, IAVLTree, KVStore};

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    pub version: u64,
    pub changes: Vec<ChangeItem>,
}

pub struct IAVLDB {
    tree: IAVLTree,
    wal: Wal<Entry>,
    pending_changes: Vec<ChangeItem>,
}

impl IAVLDB {
    pub fn new(path: &str) -> Result<Self, String> {
        let mut tree = IAVLTree::new();
        let wal: Wal<Entry> = Wal::new(path, None);

        for entry in wal.read()? {
            tree.write_batch(entry.changes);
            tree.save_version();
        }

        Ok(Self {
            tree,
            wal,
            pending_changes: Vec::new(),
        })
    }
}

impl KVStore for IAVLDB {
    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.tree.get(key)
    }

    fn set(&mut self, _key: Vec<u8>, _value: Vec<u8>) {
        panic!("PersistedDB does not support set directly");
    }

    fn remove(&mut self, _key: &[u8]) {
        panic!("PersistedDB does not support remove directly");
    }

    fn range<R>(&self, bounds: R) -> impl DoubleEndedIterator<Item = (&[u8], &[u8])>
    where
        R: std::ops::RangeBounds<Vec<u8>> + Clone,
    {
        self.tree.range(bounds)
    }

    fn write_batch(&mut self, batch: impl IntoIterator<Item = ChangeItem>) {
        let changes = batch.into_iter().collect::<Vec<_>>();
        self.pending_changes = changes.clone();
        self.tree.write_batch(changes);
    }
}

impl IAVLDB {
    pub fn save_version(&mut self) -> Output<Sha256> {
        let result = *self.tree.save_version();
        let entry = Entry {
            version: self.tree.version(),
            changes: mem::take(&mut self.pending_changes),
        };
        self.wal.write(entry);
        self.wal.flush();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::Overlay;

    #[test]
    fn test_persisted_db() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = IAVLDB::new(dir.path().to_str().unwrap()).unwrap();

        {
            let mut overlay = Overlay::new(&mut db);

            overlay.set(b"key1".to_vec(), b"value1".to_vec());
            overlay.set(b"key2".to_vec(), b"value2".to_vec());
            assert_eq!(overlay.get(b"key1"), Some(b"value1".as_ref()));
            assert_eq!(overlay.get(b"key2"), Some(b"value2".as_ref()));

            overlay.remove(b"key1");
            assert_eq!(overlay.get(b"key1"), None);

            overlay.flush();
        }

        db.save_version();

        // reload db
        let db = IAVLDB::new(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(db.get(b"key1"), None);
        assert_eq!(db.get(b"key2"), Some(b"value2".as_ref()));
        assert_eq!(db.get(b"removed"), None);
    }
}
