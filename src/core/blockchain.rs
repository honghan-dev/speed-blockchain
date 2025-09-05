use alloy::primitives::{Address, B256};
use anyhow::{Context, Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::block::Block;
use crate::consensus::{ConsensusEngine, ValidatorSet};
use crate::storage::Storage;
use crate::{ExecutionEngine, GasConfig, KeyPair};

// chain manager: glue for consensus and execution engines

#[derive(Clone)]
pub struct Blockchain {
    pub execution_engine: Arc<ExecutionEngine>,
    pub consensus_engine: Arc<Mutex<ConsensusEngine>>,
    store: Arc<Mutex<Storage>>, // RocksDB storage
                                // gas_config: GasConfig,
}

impl Blockchain {
    /// Create blockchain
    pub fn new(
        storage_path: &str,
        min_stake: u64,
        slot_duration_seconds: u64,
        validators: Vec<(Address, u64)>, // (address, stake) pairs
        local_keypair: Option<KeyPair>,
    ) -> Result<Self> {
        let store = Arc::new(tokio::sync::Mutex::new(Storage::new(storage_path)?));
        let execution_engine = Arc::new(ExecutionEngine::new());

        // Create validator set using your ValidatorSet
        let mut validator_set = ValidatorSet::new(min_stake);
        for (address, stake) in validators {
            let _ = validator_set.add_validator(address, stake);
        }

        // Simple randomness seed (in production, use block hashes)
        let randomness_seed = [1u8; 32]; // Placeholder

        // Create consensus engine with your components
        let consensus_engine = Arc::new(Mutex::new(ConsensusEngine::new(
            slot_duration_seconds,
            validator_set,
            randomness_seed,
            local_keypair,
        )));

        // let gas_config = GasConfig::default();

        Ok(Self {
            execution_engine,
            consensus_engine,
            store,
            // gas_config,
        })
    }

    /// Produce new block if choosen as proposer
    pub async fn produce_block(&self) -> Result<Block> {
        // 2. Get pending transactions
        let mut pending_txs = self.execution_engine.get_pending_transactions().await;
        if pending_txs.is_empty() {
            return Err(anyhow!("No transactions to mine"));
        }

        // 4. Simulate transaction execution
        let valid_transactions = self
            .execution_engine
            .simulate_execute_block(&mut pending_txs)
            .await?;

        // if no valid transactions
        if valid_transactions.is_empty() {
            return Err(anyhow!("No valid transactions"));
        }

        let mut consensus = self.consensus_engine.lock().await;

        // 3. Create block template
        let mut block = consensus.create_block(pending_txs).await?;

        // 7. Update engines
        let execution_result = self
            .execution_engine
            .execute_block_commit(&mut block)
            .await?;

        // get finalized block
        let finalized_block = match consensus.finalize_block(block, execution_result).await {
            Ok(block) => block,
            Err(e) => {
                println!("Finalized failed: {}", e);
                return Err(e.into());
            }
        };

        let _ = self.store_block(&finalized_block).await;

        // update consensus engine state
        consensus.update_best_block(&finalized_block).await?;

        Ok(finalized_block)
    }

    /// Validate and add block from network
    // pub async fn validate_block(&self, block: Block) -> Result<bool> {
    //     // 1. Consensus validation
    //     let consensus_valid = self.consensus_engine.validate_block(&block).await?;
    //     if !consensus_valid {
    //         return Ok(false);
    //     }

    //     // 2. Execution validation
    //     let mut block_copy = block.clone();
    //     let execution_result = self.execution_engine.execute_block(&mut block_copy).await?;

    //     // 4. Store and update
    //     self.store_block(&block).await?;
    //     self.execution_engine.finalize_block(&block).await?;
    //     self.consensus_engine.update_best_block(&block).await?;

    //     Ok(true)
    // }

    // call storage layer to store block
    async fn store_block(&self, block: &Block) -> Result<()> {
        let storage = self.store.lock().await;
        storage
            .store_block(block)
            .context("Failed to store block")?;

        println!("📦 Block #{} stored successfully", block.header.index);
        Ok(())
    }

    // get last index from storage
    pub async fn get_last_index(&self) -> Result<u64> {
        let store = self.store.lock().await;
        let last_index: u64 = match store
            .get_last_index()
            .context("Failed to retrieve last block index")?
        {
            Some(index) => index,
            None => 0, // No blocks exist
        };
        Ok(last_index)
    }

    // get block hash by index
    pub async fn get_block_hash_by_index(&self, index: &u64) -> Result<Option<B256>> {
        let store = self.store.lock().await;
        store.get_block_hash_from_index(index)
    }

    // get a block by index
    // 1) Get block hash from index
    // 2) Get block data from block hash
    pub async fn get_block_by_index(&self, index: &u64) -> Result<Block> {
        let store = self.store.lock().await;

        let block_hash = match store.get_block_hash_from_index(&index)? {
            Some(hash) => hash,
            None => {
                return Err(anyhow!("❌ No block found at index: {}", index));
            }
        };

        let block = match store.get_block_from_block_hash::<Block>(&block_hash)? {
            // ✅ Regular match instead of let Some
            Some(block) => block,
            None => {
                return Err(anyhow!(
                    "❌ Block data not found for hash: 0x{}",
                    hex::encode(&block_hash)
                ));
            }
        };

        Ok(block)
    }
}
