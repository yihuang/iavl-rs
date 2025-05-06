use alloy_consensus::TxEnvelope;
use alloy_primitives::{Address, U256};
use iavl::{KVStore, Overlay};

use crate::auth;

const CHAIN_ID: u64 = 1;

// execute_tx returns deducted fee, which should be credits to the block miner
pub fn execute_tx(kv: &mut impl KVStore, tx: &TxEnvelope) -> Option<U256> {
    let legacy = tx.as_legacy()?;
    let sender = legacy.recover_signer().ok()?;
    let tx = legacy.tx();

    // check chain-id
    if tx.chain_id? != CHAIN_ID {
        return None;
    }

    let mut account = auth::load_account(kv, &sender).unwrap_or_default();

    // check nonce
    account.check_and_incr_nonce(tx.nonce)?;

    // deduct fee
    let fee = U256::from(tx.gas_price) * U256::from(tx.gas_limit);
    account.modify_balance(|balance| balance.checked_sub(fee))?;

    // execute native transfer
    if tx.value > U256::ZERO {
        let recipient_address = tx.to.to()?;
        let mut recipient = auth::load_account(kv, recipient_address).unwrap_or_default();
        account.modify_balance(|balance| balance.checked_sub(tx.value))?;
        recipient.modify_balance(|balance| balance.checked_add(tx.value))?;

        auth::save_account(kv, recipient_address, &recipient);
    }
    auth::save_account(kv, &sender, &account);

    Some(fee)
}

// execute_block a batch of transactions, credits the collected fee to the block miner.
// each transaction is executed in a atomic way, if fail, the transaction is skipped.
pub fn execute_block(kv: &mut impl KVStore, miner: &Address, txs: &[TxEnvelope]) -> Option<()> {
    let mut reward = U256::ZERO;

    for tx in txs {
        let mut snapshot = Overlay::new(kv);
        if let Some(fee) = execute_tx(&mut snapshot, tx) {
            reward = reward.checked_add(fee)?;
            snapshot.flush();
        }
    }

    // credit fees to the block miner
    auth::modify_native_balance(kv, miner, |balance| balance.checked_add(reward))
}

#[cfg(test)]
mod tests {
    use super::*;
    use iavl::IAVLTree;

    use alloy_consensus::{Signed, TxLegacy};
    use alloy_network::TxSignerSync;
    use alloy_primitives::{TxKind, U160};
    use alloy_signer_local::PrivateKeySigner;

    fn legacy_tx(gas: u64, nonce: u64) -> TxLegacy {
        TxLegacy {
            nonce,
            value: U256::from(100),
            to: TxKind::Call(Address::random()),
            gas_limit: gas,
            gas_price: 20e9 as u128,
            chain_id: Some(CHAIN_ID),
            ..Default::default()
        }
    }

    fn sign(wallet: PrivateKeySigner, mut tx: TxLegacy) -> TxEnvelope {
        let signature = wallet.sign_transaction_sync(&mut tx).unwrap();
        TxEnvelope::Legacy(Signed::<_>::new_unhashed(tx, signature))
    }

    #[test]
    fn test_execute_block() {
        let mut kv = IAVLTree::default();
        let signer = PrivateKeySigner::random();
        let miner = Address::from(U160::from(0x1234));
        let txs = vec![
            sign(signer.clone(), legacy_tx(21000, 0)),
            sign(signer.clone(), legacy_tx(21000, 1)),
            sign(signer.clone(), legacy_tx(21000, 2)),
        ];

        let exp_total_value = U256::from(100 * txs.len());
        let exp_total_fee = U256::from(txs.len() as u128 * 21000 * 20e9 as u128);

        // fund sender account in pre-state
        auth::modify_native_balance(&mut kv, &signer.address(), |balance| {
            // enough for value transfers and fee for 3 transactions
            balance.checked_add(exp_total_value + exp_total_fee)
        });

        assert!(execute_block(&mut kv, &miner, &txs).is_some());

        // check execution side effects

        // assert miner balance
        let miner_account = auth::load_account(&kv, &miner).unwrap_or_default();
        assert_eq!(miner_account.balance, exp_total_fee);

        // assert sender nonce and balance
        let sender_account = auth::load_account(&kv, &signer.address()).unwrap_or_default();
        assert_eq!(sender_account.nonce, 3);
        assert_eq!(sender_account.balance, U256::ZERO);
    }
}
