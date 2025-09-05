use alloy::primitives::B256;
use anyhow::{Context, Result};
use rocksdb::{DB, Options};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::Block;

// persist blocks + state

pub struct Storage {
    db: DB,
}

impl Storage {
    // Create a new storage instance with the given path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let db = DB::open(&opts, path).context("Failed to open RocksDB")?;

        Ok(Self { db })
    }

    // ========== PRIMARY STORAGE: block_hash -> Block ==========

    // update database, encoded with json for readability
    pub fn put_block_hash_to_block<T: Serialize>(
        &self,
        block_hash: &B256,
        value: &T,
    ) -> Result<()> {
        // Json encoding for readability
        let json_data =
            serde_json::to_vec_pretty(value).context("Failed to serialize block to JSON")?;
        // Handle rocksdb error (remove & reference)
        self.db
            .put(block_hash, json_data)
            .with_context(|| format!("Failed to store data with key: {}", block_hash))?;
        Ok(())
    }

    // retrieve from db and decode with json
    pub fn get_block_from_block_hash<T: for<'de> Deserialize<'de>>(
        &self,
        block_hash: &B256,
    ) -> Result<Option<T>> {
        match self
            .db
            .get(block_hash)
            .with_context(|| format!("Failed to retrieve data with key: {}", block_hash))?
        {
            Some(json_bytes) => {
                let value: T = serde_json::from_slice(&json_bytes).with_context(|| {
                    format!(
                        "Failed to deserialize block with hash: 0x{}",
                        hex::encode(block_hash)
                    )
                })?;
                println!("✅ Block found and deserialized");
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    // ========== SECONDARY INDEX: block_number -> block_hash ==========

    pub fn put_index_to_block_hash(&self, index: &u64, block_hash: &B256) -> Result<()> {
        let index = index.to_le_bytes();
        self.db.put(&index, block_hash).with_context(|| {
            format!(
                "Failed to store block number to hash mapping for block number: {}",
                hex::encode(index)
            )
        })?;
        Ok(())
    }

    // get block hash from block number
    pub fn get_block_hash_from_index(&self, index: &u64) -> Result<Option<B256>> {
        let index = index.to_le_bytes();
        match self.db.get(&index).with_context(|| {
            format!(
                "Failed to retrieve block hash for block number: {}",
                hex::encode(index)
            )
        })? {
            Some(hash_bytes) => {
                if hash_bytes.len() != 32 {
                    return Err(anyhow::anyhow!("Invalid hash length for block number"));
                }
                let mut hash_array = [0u8; 32];
                hash_array.copy_from_slice(&hash_bytes);
                Ok(Some(B256::from(hash_array)))
            }
            None => Ok(None),
        }
    }

    // update last index metadata
    pub fn put_last_index(&self, index: &u64) -> Result<()> {
        let index = index.to_le_bytes();
        self.db
            .put(b"last_index", &index)
            .context("Failed to store last index")?;
        Ok(())
    }

    pub fn get_last_index(&self) -> Result<Option<u64>> {
        match self
            .db
            .get(b"last_index")
            .context("Failed to retrieve last index")?
        {
            Some(index_bytes) => {
                if index_bytes.len() != 8 {
                    return Err(anyhow::anyhow!("Invalid last index length"));
                }
                let mut index_array = [0u8; 8];
                index_array.copy_from_slice(&index_bytes);
                Ok(Some(u64::from_le_bytes(index_array)))
            }
            None => Ok(None),
        }
    }

    // Helper method
    // Store block with all necessary indices
    pub fn store_block(&self, block: &Block) -> Result<()> {
        // Store block data
        self.put_block_hash_to_block(&block.header.hash(), block)?;

        // Store index mapping
        self.put_index_to_block_hash(&block.header.index, &block.header.hash())?;

        // Update last index
        self.put_last_index(&block.header.index)?;

        Ok(())
    }
}
