use crate::types::StorePrefix;
use alloy_primitives::{Address, U256};
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};
use iavl::KVStore;

#[derive(Debug, Default, Clone, PartialEq, RlpEncodable, RlpDecodable)]
pub struct Account {
    pub address: Address,
    pub nonce: u64,
    pub balance: U256,
}

impl Account {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_address(address: Address) -> Self {
        Self {
            address,
            nonce: 0,
            balance: U256::from(0),
        }
    }
}

pub fn store_key(address: &Address) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(StorePrefix::Auth as u8);
    address.encode(&mut buf);
    buf
}

pub fn save(kv: &mut impl KVStore, account: &Account) {
    let mut buf = Vec::new();
    account.encode(&mut buf);
    let key = store_key(&account.address);
    kv.set(key, buf)
}

pub fn load(kv: &impl KVStore, address: Address) -> Account {
    let key = store_key(&address);
    if let Some(mut bz) = kv.get(&key) {
        Account::decode(&mut bz).unwrap()
    } else {
        Account::with_address(address)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloy_primitives::U160;
    use iavl::IAVLTree;

    #[test]
    fn test_auth() {
        let mut kv = IAVLTree::default();
        let address = Address::from(U160::from(0x1234));
        let mut account = Account::with_address(address);

        // test empty account
        assert_eq!(load(&kv, address), account);

        account.nonce = 1;
        save(&mut kv, &account);
        assert_eq!(load(&kv, address), account);
    }
}
