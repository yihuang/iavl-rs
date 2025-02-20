use std::ops::RangeBounds;

pub trait KVStore {
    fn get(&self, key: &[u8]) -> Option<&[u8]>;
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn remove(&mut self, key: &[u8]);
    fn range<R>(&self, bounds: R) -> impl DoubleEndedIterator<Item = (&[u8], &[u8])>
    where
        R: RangeBounds<Vec<u8>> + Clone;
}
