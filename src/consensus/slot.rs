use std::alloc::System;
use std::time::{Duration, SystemTime};
use tokio::time;

use super::error::{ConsensusError, ValidatorError};
use super::proposer::ProposerSelection;
use super::validator::ValidatorSet;
use crate::blockchain::{Block, BlockHeader, Transaction};
use alloy::primitives::Address;

pub struct Slot {
    pub duration: Duration,
    pub genesis_time: SystemTime,
    pub validator_set: ValidatorSet,
    pub proposer_selection: ProposerSelection,
    pub node_address: Address,
}

impl Slot {
    // instantiating a new slot
    pub fn new(
        duration: Duration,
        genesis_time: SystemTime,
        validator_set: ValidatorSet,
        node_address: Address,
        randomness_seed: [u8; 32],
    ) -> Self {

        // creates a new proposer selection
        let proposer_selection = ProposerSelection::new(validator_set.clone(), randomness_seed);

        Self {
            duration,
            genesis_time,
            validator_set,
            proposer_selection,
            node_address,
        }
    }

    // get current slot number
    pub fn get_current_slot(&self) -> u64 {
        let elapsed = SystemTime::now()
            .duration_since(self.genesis_time)
            .unwrap_or(Duration::ZERO);

        elapsed.as_secs() / self.duration.as_secs()
    }

    // Check if this node should propose for the given slot
    fn am_i_proposer(&self, expected_proposer: Address) -> bool {
        self.node_address == expected_proposer
    }

    // Calculate when a specific slot starts
    pub fn get_slot_start_time(&self, slot: u64) -> SystemTime {
        // genesis time + how many slots has passed
        self.genesis_time + Duration::from_secs(slot * self.duration.as_secs())
    }

    pub async fn produce_block(&mut self, slot: u64) -> Result<Block, ConsensusError> {
        let proposer = self.proposer_selection.selector_proposer(slot)?;

        // only the selected proposer should create a block
        if !self.am_i_proposer(proposer) {
            return Err(ConsensusError::NotMyTurn);
        }

        let slot_start_time = self.get_slot_start_time(slot);
        if let Ok(duration_until_slot) = slot_start_time.duration_since(SystemTime::now()) {
            time::sleep(duration_until_slot).await;
        }

        // Get pending transaction and previous block hash
        let transaction = self.get_pending_transactions();
        let prev_hash = self.get_latest_block_hash();
        let state_root = self.calculate_state_root();

        let block = BlockHeader::new(
            self.get_next_block_index(),
            slot,
            self.node_address,
            prev_hash,
            tx_root,
            state_root
        )

        // calculate gas used
        let gas_used = self.calculate_gas_used(&transaction);
        header.set_gas_used(gas_used).map_err(|e| ConsensusError::InvalidBlock(e))?;

        // create the complete block
        let block = Block::new(header, transactions);

        println!("✅ Produced block {}: slot={}, hash={}",
            block.header.index,
            block.header.slot,
            block.hash_hex()[..10].to_string()
        );

        Ok(block)
    }

    // main event loop for consensus, selecting validators
    pub async fn run_consensus_loop(&mut self) -> Result<(), ConsensusError> {
        println!("🚀 Starting PoS consensus loop for validator {:?}", self.node_address);

        loop {
            let current_slot = self.get_current_slot();

            match self.produce_block(current_slot).await {
                Ok(block) => {
                    // I'm the proposer - broadcast my block
                    self.broadcast_block(block).await;
                }
                Err(ConsensusError::NotMyTurn) => {
                        // Not our turn, check who should be proposing
                    if let Ok(expected_proposer) = self.proposer_selection.selector_proposer(current_slot) {
                        println!("⏳ Slot {}: Waiting for proposer {:?}", 
                                current_slot, expected_proposer);
                    }
                } 
                Err(e) => {
                    eprintln!("❌ Error producing block for slot {}: {:?}", current_slot, e)
                }
            }

            // calculate time until next slot
            let next_slot = current_slot + 1;
            let next_slot_time = self.get_slot_start_time(next_slot);

            if let Ok(duration_until_next_slot) = next_slot_time.duration_since(SystemTime::now()) {
                time::sleep(duration_until_next_slot).awaitl
            } else {
                time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    // helper method to retrieve pending transaction from the mempool
    fn get_pending_transactions(&self) -> Vec<Transaction> {

    }
}
