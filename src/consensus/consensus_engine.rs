use alloy::primitives::{B256, keccak256};
use std::time::{Duration, SystemTime};

use super::error::{ConsensusError, ValidatorError};
use super::proposer::ProposerSelection;
use super::validator::ValidatorSet;
use crate::core::{Block, BlockHeader, Transaction};
use crate::{ExecutionResult, KeyPair};
use anyhow::{Result, anyhow};

pub struct ConsensusEngine {
    // Block timing
    slot_duration: Duration,
    genesis_time: SystemTime,
    current_slot: u64,

    // Current consensus state
    current_block_number: u64,
    current_block_hash: B256,

    // proposer selection
    proposer_selection: ProposerSelection,

    // Validator info (for block signing)
    local_keypair: Option<KeyPair>,
}

impl ConsensusEngine {
    /// Create consensus engine using
    pub fn new(
        slot_duration_seconds: u64,
        validator_set: ValidatorSet, // Your ValidatorSet
        randomness_seed: [u8; 32],
        local_keypair: Option<KeyPair>,
    ) -> Self {
        // Use your ProposerSelection
        let proposer_selection = ProposerSelection::new(validator_set, randomness_seed);

        Self {
            slot_duration: Duration::from_secs(slot_duration_seconds),
            genesis_time: SystemTime::now(),
            current_slot: 0,
            current_block_number: 0,
            current_block_hash: B256::ZERO,
            proposer_selection,
            local_keypair,
        }
    }

    /// Validate incoming block
    pub async fn validate_block(&self, block: &Block) -> Result<bool> {
        // Basic validations
        if block.header.index != self.current_block_number + 1 {
            return Ok(false);
        }

        if block.header.parent_hash != self.current_block_hash {
            return Ok(false);
        }

        // CORE: Validate proposer using YOUR ProposerSelection
        let expected_proposer = self
            .proposer_selection
            .selector_proposer(block.header.slot)
            .map_err(|_| anyhow!("Failed to validate proposer"))?;

        if block.header.proposer != expected_proposer {
            println!(
                "Invalid proposer: expected {}, got {}",
                expected_proposer, block.header.proposer
            );
            return Ok(false);
        }

        // Validate timing
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        if block.header.timestamp > now + 30 {
            return Ok(false);
        }

        // Validate hashes
        let calculated_tx_root = self.calculate_transactions_root(&block.transactions);
        if calculated_tx_root != block.header.transactions_root {
            return Ok(false);
        }

        let calculated_hash = self.calculate_block_hash(&block.header);
        if calculated_hash != block.header.hash() {
            return Ok(false);
        }

        println!(
            "Block #{} validated from proposer {}",
            block.header.index, block.header.proposer
        );
        Ok(true)
    }

    pub async fn should_produce_block(&self) -> Result<bool> {
        let current_slot = self.calculate_current_slot()?;

        // Only produce in new slots
        if current_slot <= self.current_slot {
            return Ok(false);
        }

        // Check if we're the selected proposer
        let selected_proposer = self
            .proposer_selection
            .selector_proposer(current_slot)
            .map_err(|e| anyhow!("Proposer selection failed: {:?}", e))?;

        // Can only propose if we're the selected validator
        match &self.local_keypair {
            Some(keypair) => Ok(keypair.address == selected_proposer),
            None => Ok(false),
        }
    }

    /// Create block template
    pub async fn create_block(&self, transactions: Vec<Transaction>) -> Result<Block> {
        let current_slot = self.calculate_current_slot()?;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        // Use your ProposerSelection to get proposer
        let proposer = self
            .proposer_selection
            .selector_proposer(current_slot)
            .map_err(|e| anyhow!("Failed to select proposer: {:?}", e))?;

        let header = BlockHeader {
            index: self.current_block_number + 1,
            parent_hash: self.current_block_hash,
            timestamp,
            slot: current_slot,
            proposer,
            state_root: B256::ZERO,
            transactions_root: self.calculate_transactions_root(&transactions),
            validator_signature: None,
        };

        println!(
            "Created block template for slot {} by proposer {}",
            current_slot, proposer
        );
        Ok(Block {
            header,
            transactions,
        })
    }

    /// Finalize block with signature
    pub async fn finalize_block(
        &self,
        mut block: Block,
        execution_result: ExecutionResult,
    ) -> Result<Block> {
        // Update with execution results
        block.header.state_root = execution_result.state_root;

        // Sign if we're the proposer
        if let Some(keypair) = &self.local_keypair {
            if keypair.address == block.header.proposer {
                let _signature = keypair.sign_hash(&block.header.hash()).await?;
                println!(
                    "Block #{} signed by proposer {}",
                    block.header.index, keypair.address
                );
            }
        }

        Ok(block)
    }

    // update consensus engine value
    pub async fn update_best_block(&mut self, block: &Block) -> Result<()> {
        // Update internal state
        self.current_block_number = block.header.index;
        self.current_block_hash = block.header.hash();
        self.current_slot = block.header.slot;

        println!(
            "Consensus engine updated to block #{}, slot {}",
            block.header.index, block.header.slot
        );
        Ok(())
    }

    // calculate block hash
    fn calculate_block_hash(&self, header: &BlockHeader) -> B256 {
        let mut data = Vec::new();
        data.extend_from_slice(&header.index.to_be_bytes());
        data.extend_from_slice(header.parent_hash.as_slice());
        data.extend_from_slice(&header.timestamp.to_be_bytes());
        data.extend_from_slice(&header.slot.to_be_bytes());
        data.extend_from_slice(header.proposer.as_slice());
        data.extend_from_slice(header.state_root.as_slice());
        data.extend_from_slice(header.transactions_root.as_slice());
        keccak256(data)
    }

    // calculate transaction root hash
    // go through all transactions add them and hash it
    fn calculate_transactions_root(&self, transactions: &[Transaction]) -> B256 {
        if transactions.is_empty() {
            return B256::ZERO;
        }

        let mut data = Vec::new();
        for tx in transactions {
            data.extend_from_slice(tx.hash.as_slice());
        }
        keccak256(data)
    }

    fn calculate_current_slot(&self) -> Result<u64> {
        let elapsed = SystemTime::now().duration_since(self.genesis_time)?;
        Ok(elapsed.as_secs() / self.slot_duration.as_secs())
    }
}
