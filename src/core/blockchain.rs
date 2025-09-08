use alloy::primitives::{Address, B256};
use alloy_signer::Signature;
use anyhow::{Context, Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::block::Block;
use crate::consensus::{ConsensusEngine, ValidatorSet};
use crate::storage::Storage;
use crate::{BlockProcessResult, ExecutionEngine, KeyPair, Transaction};

// chain manager: glue for consensus and execution engines

#[derive(Clone)]
pub struct Blockchain {
    pub execution_engine: Arc<ExecutionEngine>,
    pub consensus_engine: Arc<Mutex<ConsensusEngine>>,
    store: Arc<Mutex<Storage>>, // RocksDB storage
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
        // check if this node has been choosen to propose block
        let consensus = self.consensus_engine.lock().await;
        let should_process = consensus.should_produce_block().await?;

        if !should_process {
            return Err(anyhow!("Not selected as proposer for current slot"));
        }

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

    // process and block received from the service(from other node)
    pub async fn process_received_block(
        &self,
        block: Block,
        proposer_id: Address,
        signature: Signature,
    ) -> Result<BlockProcessResult> {
        println!(
            "Blockchain: Processing received block {} from {}",
            block.header.index, proposer_id
        );

        let block_hash = block.header.hash();

        // Step 1: Verify signature first (quick check)
        if !self.verify_proposer_signature(&block, &proposer_id, &signature)? {
            println!("Blockchain: Invalid proposer signature");
            return Ok(BlockProcessResult::Rejected(
                block_hash,
                "Invalid signature".to_string(),
            ));
        }

        // Step 2: Full block validation
        match self.validate_block(&block).await {
            Ok(true) => {
                // commit the validated block, in consensus and execution state
                self.commit_validated_block(&block).await?;
                println!("Blockchain: Block {} validation passed", block.header.index);
                Ok(BlockProcessResult::Accepted(block_hash))
            }
            Ok(false) => Ok(BlockProcessResult::Rejected(
                block_hash,
                "Block validation failed".to_string(),
            )),
            Err(e) => Ok(BlockProcessResult::Rejected(
                block_hash,
                format!("Validation error: {}", e),
            )),
        }
    }

    // commit validated block by updating consensus values, and execution state
    async fn commit_validated_block(&self, block: &Block) -> Result<()> {
        // Execute transactions and commit state changes
        let mut block_copy = block.clone();
        let _ = self
            .execution_engine
            .execute_block_commit(&mut block_copy)
            .await?;

        // Store the block to disk
        self.store_block(&block).await?;

        // Update consensus engine state
        let mut consensus = self.consensus_engine.lock().await;
        consensus.update_best_block(&block).await?;

        println!("Blockchain: Block {} state committed", block.header.index);
        Ok(())
    }

    // verify block builder's signature
    fn verify_proposer_signature(
        &self,
        block: &Block,
        proposer_id: &Address,
        signature: &Signature,
    ) -> Result<bool> {
        if *proposer_id != block.header.proposer {
            return Ok(false);
        }

        let block_hash = block.header.hash();
        match signature.recover_address_from_prehash(&block_hash) {
            Ok(recovered_address) => Ok(recovered_address == *proposer_id),
            Err(_) => Ok(false),
        }
    }

    // execute by simulating state changes
    async fn validate_execution(&self, block: &Block) -> Result<bool> {
        let mut block_copy = block.clone();

        // Use simulate instead of commit (you already have this method)
        match self
            .execution_engine
            .simulate_execute_block(&mut block_copy.transactions)
            .await
        {
            Ok(valid_txs) => {
                // Check if all transactions are valid
                if valid_txs.len() != block.transactions.len() {
                    println!("Blockchain: Some transactions failed validation");
                    return Ok(false);
                }

                // For a complete check, you'd need a dry-run execution method
                // that returns the state root without committing
                Ok(true) // Simplified for now
            }
            Err(e) => {
                println!("Blockchain: Transaction simulation failed: {}", e);
                Ok(false)
            }
        }
    }

    ///// Validate and add block from network /////

    /// 1. Consensus validation
    /// 2. Execution transactions and validate state transition
    /// Main block validation method (used by both network and internal validation)
    pub async fn validate_block(&self, block: &Block) -> Result<bool> {
        // Consensus validation
        let consensus_valid = {
            let consensus = self.consensus_engine.lock().await;
            consensus.validate_block(block).await?
        };

        if !consensus_valid {
            println!("Blockchain: Consensus validation failed");
            return Ok(false);
        }

        // Execution validation
        if !self.validate_execution(block).await? {
            println!("Blockchain: Execution validation failed");
            return Ok(false);
        }

        Ok(true)
    }

    // Helper method
    // Helper function to all transaction to mempool
    pub async fn add_transaction_to_mempool(&self, transaction: &Transaction) -> Result<B256> {
        return self.execution_engine.add_transaction(transaction).await;
    }

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
