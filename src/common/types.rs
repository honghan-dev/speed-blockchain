use alloy::primitives::{Address, B256};
use alloy_signer::Signature;
use serde::{Deserialize, Serialize};

use crate::{Block, Transaction};

// For result of block processing, valid or not
#[derive(Debug, Clone)]
pub enum BlockProcessResult {
    Accepted(B256),
    Rejected(B256, String),
}

// Validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
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

// simple vote type for attestation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttestationVote {
    Accept,                    // Block is valid
    Reject { reason: String }, // Block is invalid with reason
}

// Define message from network -> blockchain
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    NewBlock {
        block: Block,
        proposer_id: Address,
        signature: Signature,
    },
    Attestation {
        block_hash: B256,
        validator_id: Address,
        vote: AttestationVote,
        signature: Signature,
    },
    NewTransaction {
        transaction: Transaction,
        from_peer: Address,
    },
}

// Define blockchain -> network message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockchainMessage {
    NewBlock {
        block: Block,
        proposer: Address,
        signature: Signature,
    },
    Attestation {
        block_hash: B256,
        validator: Address,
        vote: AttestationVote,
        signature: Signature,
    },
    NewTransaction {
        transaction: Transaction,
    },
}
