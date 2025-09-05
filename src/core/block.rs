use super::blockheader::BlockHeader;
use super::transaction::Transaction;
use alloy::primitives::{B256, keccak256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    // Create a new block with transactions
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }

    // creates a genesis block
    pub fn genesis() -> Self {
        Self::new(BlockHeader::genesis(), Vec::new())
    }

    // calculate transaction root, using simple hash, NOT an actual merkle root
    pub fn calculate_transactions_root(transactions: &[Transaction]) -> B256 {
        if transactions.is_empty() {
            return B256::ZERO;
        }

        let mut data = Vec::new();

        // Sort transactions by hash for deterministic ordering
        let mut sorted_transactions = transactions.to_vec();
        sorted_transactions.sort_by_key(|tx| tx.hash);

        for tx in sorted_transactions {
            data.extend_from_slice(tx.hash.as_slice());
        }

        keccak256(&data)
    }
}
