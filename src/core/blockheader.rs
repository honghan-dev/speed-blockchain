use alloy::primitives::{Address, B256, Signature, keccak256};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{KeyPair, SignatureError};

// Block structure, uses Alloy's B256 for hashes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    // block identity
    pub index: u64,
    pub parent_hash: B256,

    // time & consensus
    pub slot: u64,
    pub timestamp: u64,
    pub proposer: Address,

    // content
    pub transactions_root: B256,
    pub state_root: B256,

    // Ethereum-style signature (65 bytes: r + s + v)
    pub validator_signature: Option<Vec<u8>>,
}

impl BlockHeader {
    // new blockheader
    pub fn new(
        index: u64,
        slot: u64,
        proposer: Address,
        parent_hash: B256,
        transactions_root: B256,
        state_root: B256,
    ) -> Self {
        Self {
            index,
            slot,
            proposer,
            parent_hash,
            transactions_root,
            state_root,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            validator_signature: None,
            // gas_limit: 0,
            // gas_used: 0,
        }
    }

    // create genesis block header
    pub fn genesis() -> Self {
        Self::new(0, 0, Address::ZERO, B256::ZERO, B256::ZERO, B256::ZERO)
    }

    // get the header hash
    // Calculate deterministic hash of the header
    pub fn hash(&self) -> B256 {
        let mut data = Vec::new();

        // Add all consensus-critical fields in deterministic order
        data.extend_from_slice(&self.index.to_be_bytes());
        data.extend_from_slice(self.parent_hash.as_slice());
        data.extend_from_slice(&self.slot.to_be_bytes());
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        data.extend_from_slice(self.proposer.as_slice());
        data.extend_from_slice(self.transactions_root.as_slice());
        data.extend_from_slice(self.state_root.as_slice());

        // NOTE: We don't include validator_signature in hash calculation
        // because the signature is OF the hash, not part of it
        keccak256(&data)
    }

    // Signing message hash
    pub async fn sign(&mut self, keypair: &KeyPair) -> Result<(), String> {
        let block_hash = self.hash();
        let signature = keypair.sign_hash(&block_hash).await.unwrap();

        // store signature as bytes
        self.validator_signature = Some(signature.as_bytes().to_vec());

        Ok(())
    }

    // verify the record signature (when receiving blocks)
    pub fn verify_signature(&self) -> Result<(), SignatureError> {
        let signature_bytes = match &self.validator_signature {
            Some(sig) => sig,
            None => return Err(SignatureError::InvalidSignature),
        };

        // check signature len
        if signature_bytes.len() != 65 {
            return Err(SignatureError::InvalidSignature);
        };

        // extract signature components
        let r_s = &signature_bytes[0..64];
        let v = signature_bytes[64];

        // create signature
        let signature = Signature::from_bytes_and_parity(r_s, v != 0);

        let block_hash = self.hash();

        let recovered_address = signature
            .recover_address_from_prehash(&block_hash)
            .map_err(|_| SignatureError::InvalidSignature)?;

        if recovered_address != self.proposer {
            return Err(SignatureError::SignatureVerificationFailed);
        }

        Ok(())
    }

    // Get hash as hex string for display
    pub fn hash_hex(&self) -> String {
        format!("0x{}", hex::encode(self.hash().as_slice()))
    }
}

// Default implementation
impl Default for BlockHeader {
    fn default() -> Self {
        Self::genesis()
    }
}
