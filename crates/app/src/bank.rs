use crate::types::StorePrefix;
use alloy_primitives::{Address, U256};
use alloy_rlp::{Decodable, Encodable};
use iavl::KVStore;

pub fn store_key(address: &Address, denom: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(StorePrefix::Bank as u8);
    address.encode(&mut buf);
    buf.extend_from_slice(denom.as_bytes());
    buf
}

pub fn get_balance(kv: &impl KVStore, address: &Address, denom: &str) -> U256 {
    let key = store_key(address, denom);
    if let Some(mut bz) = kv.get(&key) {
        U256::decode(&mut bz).unwrap()
    } else {
        U256::from(0)
    }
}

pub fn set_balance(kv: &mut impl KVStore, address: &Address, denom: &str, amount: U256) {
    let mut buf = Vec::new();
    amount.encode(&mut buf);
    let key = store_key(address, denom);
    kv.set(key, buf)
}

pub fn transfer(
    kv: &mut impl KVStore,
    from: &Address,
    to: &Address,
    denom: &str,
    amount: U256,
) -> Option<()> {
    let from_balance = get_balance(kv, from, denom);
    if from_balance < amount {
        return None;
    }
    set_balance(kv, from, denom, from_balance - amount);
    let to_balance = get_balance(kv, to, denom);
    set_balance(kv, to, denom, to_balance + amount);
    Some(())
}

#[cfg(test)]
mod test {
    use super::*;
    use alloy_primitives::Address;
    use alloy_primitives::U160;
    use iavl::IAVLTree;

    #[test]
    fn test_bank() {
        let mut kv = IAVLTree::default();
        let address = Address::from(U160::from(0x1234));
        let denom = "atom";
        let amount = U256::from(100);

        set_balance(&mut kv, &address, denom, amount);
        assert_eq!(get_balance(&kv, &address, denom), amount);
    }
}
