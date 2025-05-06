use crate::types::StorePrefix;
use alloy_primitives::{Address, U256};
use alloy_rlp::{Decodable, Encodable, RlpDecodable, RlpEncodable};
use iavl::KVStore;

#[derive(Debug, Default, Clone, PartialEq, RlpEncodable, RlpDecodable)]
pub struct AccountValue {
    pub nonce: u64,
    pub balance: U256,
}

impl AccountValue {
    pub fn check_and_incr_nonce(&mut self, exp_nonce: u64) -> Option<()> {
        if self.nonce != exp_nonce {
            return None;
        }
        self.nonce = self.nonce.checked_add(1)?;
        Some(())
    }

    pub fn modify_balance(&mut self, mod_fn: impl FnOnce(U256) -> Option<U256>) -> Option<()> {
        self.balance = mod_fn(self.balance)?;
        Some(())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Account {
    pub address: Address,
    pub inner: AccountValue,
}

impl Account {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_address(address: Address) -> Self {
        Self {
            address,
            inner: AccountValue::default(),
        }
    }
}

pub fn store_key(address: &Address) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(StorePrefix::Auth as u8);
    address.encode(&mut buf);
    buf
}

pub fn save_account(kv: &mut impl KVStore, address: &Address, value: &AccountValue) {
    let mut buf = Vec::new();
    value.encode(&mut buf);
    let key = store_key(address);
    kv.set(key, buf)
}

pub fn load_account(kv: &impl KVStore, address: &Address) -> Option<AccountValue> {
    let key = store_key(address);
    let mut bz = kv.get(&key)?;
    AccountValue::decode(&mut bz).ok()
}

pub fn load_or_default(kv: &impl KVStore, address: Address) -> Account {
    let value = load_account(kv, &address).unwrap_or_default();
    Account {
        address,
        inner: value,
    }
}

pub fn check_and_incr_nonce(
    kv: &mut impl KVStore,
    address: &Address,
    exp_nonce: u64,
) -> Option<()> {
    let mut account = load_account(kv, address).unwrap_or_default();
    if account.nonce != exp_nonce {
        return None;
    }

    account.check_and_incr_nonce(exp_nonce)?;
    save_account(kv, address, &account);
    Some(())
}

pub fn modify_native_balance(
    kv: &mut impl KVStore,
    address: &Address,
    mod_fn: impl FnOnce(U256) -> Option<U256>,
) -> Option<()> {
    let mut account = load_account(kv, address).unwrap_or_default();
    account.modify_balance(mod_fn)?;
    save_account(kv, address, &account);
    Some(())
}

pub fn transfer_native_token(
    kv: &mut impl KVStore,
    from: &Address,
    to: &Address,
    amount: U256,
) -> Option<()> {
    modify_native_balance(kv, from, |balance| balance.checked_sub(amount))?;
    modify_native_balance(kv, to, |balance| balance.checked_add(amount))
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
        let mut account = AccountValue::default();

        // test empty account
        assert_eq!(load_account(&kv, &address).unwrap_or_default(), account);

        account.nonce = 1;
        save_account(&mut kv, &address, &account);
        assert_eq!(load_account(&kv, &address).unwrap_or_default(), account);
    }
}
