use crate::blockchain::Transaction;
use crate::crypto::{KeyPair, SignatureError};
use alloy::primitives::{Address, B256};
use alloy_signer::Signature;
use anyhow::{Result, anyhow};
use hex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Mempool {
    // Core storage - just the essentials
    // tx_hash, B32 -> Transaction
    transactions: HashMap<B256, Transaction>,
    address_nonces: HashMap<Address, u64>, // Track nonces per address
    // Maximum number of transaction
    max_size: usize,
}

impl Mempool {
    // Create a new mempool with a maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: HashMap::new(),
            address_nonces: HashMap::new(),
            max_size,
        }
    }

    // Add a transaction to the mempool
    pub fn add_transaction(&mut self, transaction: Transaction, keypair: &KeyPair) -> Result<B256> {
        // STEP 1: Get transaction hash, because hash is optional
        let tx_hash = transaction
            .hash
            .ok_or_else(|| anyhow!("Transaction has no hash"))?;

        let signature = transaction
            .signature
            .ok_or_else(|| anyhow!("Transaction has no hash"))?;

        // STEP 2: Check if transaction already exists
        if self.transactions.contains_key(&tx_hash) {
            return Err(anyhow!(
                "Transaction {} already exists in mempool",
                format!("0x{}", hex::encode(&tx_hash.as_slice()[..8]))
            ));
        }

        // STEP 3: Check mempool size limit
        if self.transactions.len() >= self.max_size {
            return Err(anyhow!("Mempool is full ({} transactions)", self.max_size));
        }

        // STEP 4: Verify transaction signature
        self.verify_transaction_signature(&tx_hash, &signature, keypair)?;

        // Basic validation
        self.validate_transaction(&transaction)?;

        // Add to mempool
        // insert consumes the transaction
        self.transactions.insert(tx_hash, transaction); // consumes the value

        println!(
            "✅ Transaction {} added to mempool",
            hex::encode(&tx_hash[..8])
        );
        Ok(tx_hash)
    }

    // helper function to get transaction hash
    pub fn get_transaction_hash(&self, transaction: &Transaction) -> Result<B256> {
        match transaction.hash {
            Some(hash) => Ok(hash),
            None => Err(anyhow!(
                "Transaction has no hash - was it properly created?"
            )),
        }
    }

    // Verify transaction signature
    fn verify_transaction_signature(
        &self,
        hash: &B256,
        signature: &Signature,
        keypair: &KeyPair,
    ) -> Result<()> {
        println!("🔍 Mempool: Verifying transaction signature...");

        // Verify the signature
        keypair
            .verify_signature(&hash, &signature)
            .map_err(|e| match e {
                SignatureError::InvalidSignature => {
                    anyhow!("Transaction has invalid signature format")
                }
                SignatureError::SignatureVerificationFailed => {
                    anyhow!("Transaction signature verification failed - unauthorized signer")
                }
                _ => anyhow!("Signature verification error: {}", e),
            })?;

        // If we reach here, signature is valid
        println!("✅ Mempool - Transaction signature verified successfully");
        Ok(())
    }

    fn validate_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Basic validation only
        if transaction.amount < 0 {
            return Err(anyhow!("Transaction amount cannot be negative"));
        }

        if transaction.fee < 0 {
            return Err(anyhow!("Transaction fee cannot be negative"));
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
