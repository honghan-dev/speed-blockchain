use crate::core::Transaction;
use alloy::primitives::B256;
use anyhow::{Result, anyhow};
use hex;
use std::collections::HashMap;

// tx queue, ordering

#[derive(Debug, Clone)]
pub struct Mempool {
    // Core storage - just the essentials
    // tx_hash, B32 -> Transaction
    transactions: HashMap<B256, Transaction>,
    // Maximum number of transaction
    max_size: usize,
}

impl Mempool {
    // Create a new mempool with a maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: HashMap::new(),
            max_size,
        }
    }

    // Add a transaction to the mempool
    pub fn add_transaction(&mut self, transaction: &Transaction) -> Result<B256> {
        let tx_hash = transaction.hash;

        if !transaction.is_signature_valid() {
            return Err(anyhow!(
                "Transaction signature failed for {}",
                hex::encode(&tx_hash[..8])
            ));
        }

        println!(
            "✅ Signature verified for transaction {}",
            hex::encode(&tx_hash[..8])
        );

        let _ = self.validate_transaction(&transaction);

        self.replace_transaction_by_fee(&transaction)?;

        // Add to mempool
        // insert consumes the transaction
        self.transactions.insert(tx_hash, transaction.clone()); // consumes the value

        println!(
            "✅ Transaction {} added to mempool",
            hex::encode(&tx_hash[..8])
        );
        Ok(tx_hash)
    }

    // replace existing transaction by fee
    fn replace_transaction_by_fee(&mut self, transaction: &Transaction) -> Result<()> {
        if let Some(existing) = self
            .transactions
            .values()
            .find(|t| t.from == transaction.from && t.nonce == transaction.nonce)
        {
            if transaction.gas_price > existing.gas_price {
                println!(
                    "⚡ Replacing tx from {} with nonce {} (new fee {} > old fee {})",
                    transaction.from, transaction.nonce, transaction.gas_price, existing.gas_price
                );
                let old_hash = existing.hash;
                self.transactions.remove(&old_hash);
            } else {
                println!(
                    "❌ Duplicate nonce tx rejected (fee {} <= existing fee {})",
                    transaction.gas_price, existing.gas_price
                );
            }
        }
        Ok(())
    }

    fn validate_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Basic validation only
        if transaction.amount < 0 {
            return Err(anyhow!("Transaction amount cannot be negative"));
        }

        if transaction.gas_price < 0 {
            return Err(anyhow!("Transaction gas price cannot be negative"));
        }

        if transaction.from.is_empty() || transaction.to.is_empty() {
            return Err(anyhow!("Transaction addresses cannot be empty"));
        }

        if transaction.from == transaction.to {
            return Err(anyhow!("Cannot send transaction to yourself"));
        }

        Ok(())
    }

    // Get all transactions
    pub fn get_all_transactions(&self) -> Vec<Transaction> {
        self.transactions.values().cloned().collect()
    }

    /// Check if there are transactions to mine
    pub fn has_transactions(&self) -> bool {
        !self.transactions.is_empty()
    }

    // Clear all transactions in the mempool
    pub fn clear_all_transactions(&mut self) {
        self.transactions.clear();
    }
}
