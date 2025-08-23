use super::transaction::Transaction;
use alloy::primitives::{B256, keccak256};
use hex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
// Block structure, uses Alloy's B256 for hashes
#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub prev_hash: B256,
    pub block_hash: B256,
    pub nonce: u64,
    pub merkle_root: B256,
    pub difficulty: usize,
}

impl Block {
    // Create a new block with transactions
    pub fn new(
        index: u64,
        transactions: Vec<Transaction>,
        prev_hash: B256,
        difficulty: usize,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Calculate the block merkle root
        let merkle_root = Self::calculate_merkle_root(&transactions);
        let mut nonce = 0;
        let mut block_hash;

        println!("Mining block {} with difficulty {}...", index, difficulty);
        let start_time = SystemTime::now();

        loop {
            // @note gets the block hash that meets the difficulty
            block_hash = Self::calculate_hash(index, timestamp, &merkle_root, &prev_hash, nonce);

            // Check if the hash meets the difficulty requirement
            if Self::meets_difficulty(&block_hash, difficulty) {
                // Calculate mining time
                let mining_time = SystemTime::now().duration_since(start_time).unwrap();

                // Log the successful mining
                println!(
                    "✅ Block mined! Hash: {} (took {:?}, nonce: {})",
                    Self::b256_to_hex(&block_hash),
                    mining_time,
                    nonce // ✅ Convert to hex only for display
                );
                break; // Found a valid hash
            }

            // Check if the hash meets the difficulty requirement
            nonce += 1;

            // Progress indicator
            if nonce % 100000 == 0 {
                println!("   Mining... tried {} nonces", nonce);
            }
        }

        Self {
            index,
            timestamp,
            transactions,
            prev_hash,
            block_hash,
            nonce,
            merkle_root,
            difficulty,
        }
    }

    // calculate the hash before producing a block
    // @returns B256
    pub fn calculate_hash(
        index: u64,
        timestamp: u64,
        merkle_root: &B256,
        prev_hash: &B256,
        nonce: u64,
    ) -> B256 {
        let mut data = Vec::new();

        // Add each field as bytes in deterministic order
        data.extend_from_slice(&index.to_be_bytes());
        data.extend_from_slice(&timestamp.to_be_bytes());
        data.extend_from_slice(&nonce.to_be_bytes());

        // Add string data as bytes
        data.extend_from_slice(merkle_root.as_slice());
        data.extend_from_slice(prev_hash.as_slice());

        keccak256(&data)
    }

    // Simple Merkle root calculation (simplified for learning)
    //
    fn calculate_merkle_root(transactions: &[Transaction]) -> B256 {
        if transactions.is_empty() {
            return B256::ZERO; // Empty merkle root
        }

        let mut combined_hashes = Vec::new();

        for tx in transactions {
            match serde_json::to_vec(tx) {
                Ok(tx_bytes) => {
                    let _ = keccak256(&tx_bytes);
                    combined_hashes.extend_from_slice(&tx_bytes);
                }
                Err(e) => {
                    eprintln!("❌ Failed to serialize transaction: {}", e);
                    combined_hashes.extend_from_slice(&[0u8; 32]);
                }
            }
        }
        keccak256(&combined_hashes)
    }

    /// Helper method to check if hash meets difficulty
    fn meets_difficulty(hash: &B256, difficulty: usize) -> bool {
        // Convert to hex string only for difficulty check
        let hash_hex = hex::encode(hash.as_slice());
        hash_hex.starts_with(&"0".repeat(difficulty))
    }

    /// Helper method to convert B256 to hex string (for display, error logging, etc.)
    fn b256_to_hex(hash: &B256) -> String {
        format!("0x{}", hex::encode(hash.as_slice()))
    }
}
