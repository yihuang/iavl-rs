pub trait KVStore {
    fn get(&self, key: &[u8]) -> Option<&[u8]>;
    fn set(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn remove(&mut self, key: &[u8]);
}
