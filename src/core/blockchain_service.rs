use crate::{
    Attestation, AttestationVote, Block, BlockProcessResult, Blockchain, BlockchainMessage,
    KeyPair, NetworkMessage, Transaction, ValidatorRole,
};
use alloy::primitives::{Address, B256, keccak256};
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
    received_attestations: HashMap<B256, Vec<Attestation>>,
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
            received_attestations: HashMap::new(),
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

                // Periodical checking whether we should propose block
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
                self.handle_received_block(block, proposer_id, signature)
                    .await?;
            }
            // handle receiving new attestation from other nodes
            NetworkMessage::Attestation {
                block_hash,
                validator_id,
                vote,
                signature,
            } => {
                self.handle_received_attestation(block_hash, validator_id, vote, signature)
                    .await?;
            }
            // handle receiving new transaction from other nodes
            NetworkMessage::NewTransaction {
                transaction,
                from_peer,
            } => {
                self.handle_received_transaction(&transaction, &from_peer)
                    .await?;
            }
        }
        Ok(())
    }

    // receiving a block from network
    async fn handle_received_block(
        &mut self,
        block: Block,
        proposer_id: Address,
        signature: Signature,
    ) -> Result<()> {
        println!(
            "Service: Received block {}, forwarding to blockchain",
            block.header.index
        );

        // early signature verification.
        if !self.verify_block_signature(&block.header.hash(), &proposer_id, &signature)? {
            println!(
                "Service: Invalid block signature from {}, dropping",
                proposer_id
            );
            return Ok(()); // Drop message immediately
        }

        // blockchain layer validation
        let blockchain_result = {
            let blockchain = self.blockchain.lock().await;
            blockchain
                .process_received_block(block, proposer_id, signature)
                .await?
        };

        // React based on blockchain's decision
        match blockchain_result {
            BlockProcessResult::Accepted(block_hash) => {
                if matches!(self.role, ValidatorRole::Attestor) {
                    self.create_and_send_attestation(block_hash, AttestationVote::Accept)
                        .await?;
                }
            }
            BlockProcessResult::Rejected(block_hash, reason) => {
                if matches!(self.role, ValidatorRole::Attestor) {
                    self.create_and_send_attestation(
                        block_hash,
                        AttestationVote::Reject { reason },
                    )
                    .await?;
                }
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

        // verify attestation signature first before calling blockchain layer
        if !self.verify_attestation_signature(&block_hash, &validator_id, &vote, &signature)? {
            println!(
                "Service: Invalid attestation signature from {}, ignoring",
                validator_id
            );
            return Ok(());
        }

        // Store attestation
        let attestation = Attestation {
            validator_id,
            vote: vote.clone(),
            signature,
        };

        // update attestation received
        self.received_attestations
            .entry(block_hash)
            .or_insert_with(Vec::new)
            .push(attestation);

        // process attestation received from other node, as a proposer
        if matches!(self.role, ValidatorRole::Proposer) {
            self.process_attestation_as_proposer(block_hash, vote)
                .await?;
        }

        Ok(())
    }

    // Adding transaction received from other node to mempool
    async fn handle_received_transaction(
        &self,
        transaction: &Transaction,
        from_peer: &Address,
    ) -> Result<()> {
        println!(
            "Service: Received transaction {} from peer {}",
            hex::encode(transaction.hash),
            from_peer
        );

        // @todo No Transaction validation
        let blockchain = self.blockchain.lock().await;
        let result = blockchain.add_transaction_to_mempool(&transaction).await;

        match result {
            Ok(tx_hash) => {
                println!(
                    "Service: Transaction {} added to mempool successfully",
                    hex::encode(tx_hash)
                );
            }
            Err(e) => {
                println!("Service: Failed to add transaction to mempool: {}", e);
            }
        }

        Ok(())
    }

    // Helper method for Blockchain layer
    // Calls blockchain layer to validate block
    async fn validate_block(&self, block: &Block) -> Result<bool> {
        let blockchain = self.blockchain.lock().await;

        // Use your existing blockchain validation
        match blockchain.validate_block(block).await {
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

    // propose new block
    async fn propose_block(&mut self) -> Result<()> {
        let new_block = match {
            let blockchain = self.blockchain.lock().await;
            blockchain.produce_block().await
        } {
            Ok(block) => block,
            Err(_) => {
                // Not our turn or no transactions - normal
                return Ok(());
            }
        };

        let block_msg = BlockchainMessage::NewBlock {
            block: new_block.clone(),
            proposer: self.validator_address,
            signature: new_block
                .header
                .validator_signature
                .ok_or_else(|| anyhow::anyhow!("Block header missing validator signature"))?,
        };

        self.to_network_sender
            .send(block_msg)
            .map_err(|_| anyhow::anyhow!("Failed to send block to network"))?;

        println!("Service: Block broadcasted to network");
        Ok(())
    }

    /// proposer handles attestation received from other nodes
    async fn process_attestation_as_proposer(
        &mut self,
        block_hash: B256,
        vote: AttestationVote,
    ) -> Result<()> {
        // no roll back capability, assuming our block is definitely being accepted
        match vote {
            AttestationVote::Accept => {
                println!(
                    "Service: Received ACCEPT vote for block {}",
                    hex::encode(block_hash)
                );
            }

            AttestationVote::Reject { reason } => {
                println!(
                    "Service: Received REJECT vote for block {}: {}",
                    hex::encode(block_hash),
                    reason
                );
            }
        }
        Ok(())
    }

    // for attestation signature validation before calling blockchain layer
    fn verify_attestation_signature(
        &self,
        block_hash: &B256,
        validator_id: &Address,
        vote: &AttestationVote,
        signature: &Signature,
    ) -> Result<bool> {
        let message = format!("ATTEST:{}:{:?}", hex::encode(block_hash), vote);
        self.verify_signature(&message, validator_id, signature)
    }

    // for block signature verification before calling blockchain layer
    fn verify_block_signature(
        &self,
        block_hash: &B256,
        proposer_id: &Address,
        signature: &Signature,
    ) -> Result<bool> {
        let message = hex::encode(block_hash); // Blocks are signed directly on hash
        self.verify_signature(&message, proposer_id, signature)
    }

    // generic verify signature method
    fn verify_signature(
        &self,
        message: &str,
        expected_signer: &Address,
        signature: &Signature,
    ) -> Result<bool> {
        let message_hash = keccak256(message.as_bytes());

        match signature.recover_address_from_prehash(&message_hash) {
            Ok(recovered_address) => Ok(recovered_address == *expected_signer),
            Err(_) => {
                println!("Service: Failed to recover address from signature");
                Ok(false)
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
        // hash the message -> B256
        let message_hash = keccak256(message.as_bytes());
        // creates signature
        let signature = self.keypair.sign_hash(&message_hash).await?;

        // instantiate attestation msg
        let attestation_msg = BlockchainMessage::Attestation {
            block_hash,
            validator: self.validator_address,
            vote,
            signature: signature,
        };

        // Send attestation via network
        self.to_network_sender
            .send(attestation_msg)
            .map_err(|_| anyhow::anyhow!("Failed to send attestation to network"))?;

        println!("Blockchain: Attestation sent");
        Ok(())
    }
}
