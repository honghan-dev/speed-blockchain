use super::blockheader::BlockHeader;
use super::transaction::Transaction;
use alloy::primitives::{Address, B256, keccak256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,

    #[serde(skip)]
    pub hash: Option<B256>, // don't serialize cached hash
}

impl Block {
    // Create a new block with transactions
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
            hash: None, // Calculate on demand
        }
    }

    // creates a genesis block
    pub fn genesis() -> Self {
        Self::new(BlockHeader::genesis(), Vec::new())
    }

    // calculate the hash before producing a block
    // @returns B256
    pub fn calculate_hash(index: u64, merkle_root: &B256, prev_hash: &B256) -> B256 {
        let mut data = Vec::new();

        // Add each field as bytes in deterministic order
        data.extend_from_slice(&index.to_be_bytes());
        // Block hash not included in hashing to prevent circular dependencies
        // Add string data as bytes
        data.extend_from_slice(merkle_root.as_slice());
        data.extend_from_slice(prev_hash.as_slice());

        keccak256(&data)
    }
}
