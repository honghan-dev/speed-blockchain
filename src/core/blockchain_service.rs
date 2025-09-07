use crate::{AttestationVote, Block, Blockchain, BlockchainMessage, KeyPair, NetworkMessage};
use alloy::primitives::{Address, B256};
use alloy_signer::Signature;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender},
};

// blockchain service layer as an interface between blockchain and network
pub struct BlockchainService {
    // Core blockchain components
    blockchain: Arc<Mutex<Blockchain>>,
    keypair: KeyPair,
    validator_address: Address,
    role: ValidatorRole,

    // Communication channels
    from_network_receiver: UnboundedReceiver<NetworkMessage>,
    to_network_sender: UnboundedSender<BlockchainMessage>,

    // Simple state tracking
    pending_blocks: HashMap<B256, Block>, // Blocks waiting for attestations
}

#[derive(Debug, Clone)]
pub enum ValidatorRole {
    Proposer,
    Attestor,
}

#[derive(Debug, Clone)]
pub struct Attestation {
    pub validator_id: Address,
    pub vote: AttestationVote,
    pub signature: Signature,
}

impl BlockchainService {
    // creating a new instance
    pub fn new(
        from_network: UnboundedReceiver<NetworkMessage>,
        to_network: UnboundedSender<BlockchainMessage>,
        blockchain: Blockchain,
        keypair: KeyPair,
        role: ValidatorRole,
    ) -> Self {
        Self {
            blockchain: Arc::new(Mutex::new(blockchain)),
            validator_address: keypair.address,
            keypair,
            role,
            from_network_receiver: from_network,
            to_network_sender: to_network,
            pending_blocks: HashMap::new(),
        }
    }

    // start blockchain service instance
    pub async fn run(&mut self) -> Result<()> {
        let mut block_timer = tokio::time::interval(tokio::time::Duration::from_secs(10));

        loop {
            tokio::select! {
                // Handle messages from network, message from other nodes
                Some(msg) = self.from_network_receiver.recv() => {
                    self.handle_network_message(msg).await?;
                }

                // Periodic block proposal (for proposers only)
                _ = block_timer.tick() => {
                    if matches!(self.role, ValidatorRole::Proposer) {
                        self.propose_block().await?;
                    }
                }
            }
        }
    }

    // handle message from other notes
    async fn handle_network_message(&mut self, msg: NetworkMessage) -> Result<()> {
        match msg {
            // handle receiving new blocks from other nodes
            NetworkMessage::NewBlock {
                block,
                proposer_id,
                signature,
            } => {
                self.handle_received_block(&block, &proposer_id, &signature)
                    .await?;
            } // handle receiving new attestation from other nodes
              // NetworkMessage::Attestation {
              //     block_hash,
              //     validator_id,
              //     vote,
              //     signature,
              // } => {
              //     self.handle_received_attestation(block_hash, validator_id, vote, signature)
              //         .await?;
              // }
              // // handle receiving new transaction from other nodes
              // NetworkMessage::NewTransaction {
              //     transaction,
              //     from_peer,
              // } => {
              //     self.handle_received_transaction(transaction, from_peer)
              //         .await?;
              // }
        }
        Ok(())
    }

    // receiving a block from network
    async fn handle_received_block(
        &mut self,
        block: &Block,
        proposer_id: &Address,
        signature: &Signature,
    ) -> Result<()> {
        println!(
            "Blockchain: Received block {} from {}",
            block.header.index, proposer_id
        );

        // Simple validation
        let is_valid = self.validate_block(&block).await?;

        if is_valid {
            // Store block as pending
            let block_hash = block.header.hash();
            self.pending_blocks.insert(block_hash, block.clone());

            println!("Blockchain: Block {} validation passed", block.header.index);

            // Create attestation (for attestors)
            if matches!(self.role, ValidatorRole::Attestor) {
                self.create_and_send_attestation(block_hash, AttestationVote::Accept)
                    .await?;
            }
        } else {
            println!("Blockchain: Block validation failed");
            if matches!(self.role, ValidatorRole::Attestor) {
                let block_hash = block.header.hash();
                self.create_and_send_attestation(
                    block_hash,
                    AttestationVote::Reject {
                        reason: "Invalid block".to_string(),
                    },
                )
                .await?;
            }
        }

        Ok(())
    }

    // handle receiving attestations
    async fn handle_received_attestation(
        &mut self,
        block_hash: B256,
        validator_id: Address,
        vote: AttestationVote,
        signature: Signature,
    ) -> Result<()> {
        println!(
            "Blockchain: Received {:?} attestation for block {}",
            vote,
            hex::encode(block_hash)
        );

        // Store attestation
        let attestation = Attestation {
            validator_id,
            vote: vote.clone(),
            signature,
        };
        self.received_attestations
            .entry(block_hash)
            .or_insert_with(Vec::new)
            .push(attestation);

        // Check if we can finalize block (simple: need 1 accept for 2-validator system)
        if matches!(vote, AttestationVote::Accept) && matches!(self.role, ValidatorRole::Proposer) {
            if let Some(block) = self.pending_blocks.remove(&block_hash) {
                self.finalize_block(block).await?;
            }
        }

        Ok(())
    }

    // Helper method for Blockchain layer
    // Calls blockchain layer to validate block
    async fn validate_block(&self, block: &Block) -> Result<bool> {
        let blockchain = self.blockchain.lock().await;

        // Use your existing blockchain validation
        match blockchain.validate_block(block) {
            Ok(is_valid) => {
                println!("Blockchain: Block validation result: {}", is_valid);
                Ok(is_valid)
            }
            Err(e) => {
                println!("Blockchain: Block validation error: {}", e);
                Ok(false) // Treat validation errors as invalid blocks
            }
        }
    }

    // send attestation to network layer
    async fn create_and_send_attestation(
        &self,
        block_hash: B256,
        vote: AttestationVote,
    ) -> Result<()> {
        println!(
            "Blockchain: Creating {:?} attestation for block {}",
            vote,
            hex::encode(block_hash)
        );

        // Create a simple attestation signature
        // In production, you'd sign the block hash + vote
        let message = format!("ATTEST:{}:{:?}", hex::encode(block_hash), vote);
        let signature = self.keypair.sign_message(message.as_bytes()).await?;

        // Send attestation via network
        let attestation_msg = BlockchainMessage::Attestation {
            block_hash,
            validator: self.validator_address,
            vote,
            signature,
        };

        self.to_network_sender
            .send(attestation_msg)
            .map_err(|_| anyhow::anyhow!("Failed to send attestation to network"))?;

        println!("Blockchain: Attestation sent");
        Ok(())
    }
}
