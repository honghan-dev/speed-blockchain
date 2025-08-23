use rlp_derive::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
// evm compatible fields
use alloy::primitives::{Address, B256, U256, keccak256};
use alloy_signer::Signature;

use crate::crypto::{KeyPair, SignatureError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: Address,  // Sender address
    pub to: Address,    // Receiver address
    pub amount: U256,   // Amount to transfer
    pub fee: U256,      // Transaction fee
    pub timestamp: u64, // When transaction was created
    pub nonce: u64,     // Nonce for transaction uniqueness

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>, // Optional signature for the transaction

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<B256>, // Optional hash for the transaction
}

impl Transaction {
    pub fn new(from: String, to: String, amount: u64, fee: u64) -> Result<Self, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let from = Address::from_str(&from.as_str()).expect("Invalid from address");
        let to = Address::from_str(&to.as_str()).expect("Invalid to address");

        let mut tx = Self {
            from,
            to,
            amount: U256::from(amount),
            fee: U256::from(fee),
            timestamp,
            nonce: 0, // Default nonce
            signature: None,
            hash: None,
        };

        // Create the transaction hash
        tx.hash = Some(tx.create_unsigned_hash());

        Ok(tx)
    }

    /////////////// NOTE ////////////////
    /// This is the server signing part, in real blockchain, this would be done in the wallet(client-side) so that private key is never exposed to the server
    pub fn create_unsigned_hash(&self) -> B256 {
        // vec is heap allocated
        let mut data = Vec::new();

        // @note from = Address = [u8; 20](which is known size and stack allocated)
        // therefore, we need to convert it to slice first, which is heap allocated
        data.extend_from_slice(self.from.as_slice());
        data.extend_from_slice(self.to.as_slice());
        data.extend_from_slice(&self.amount.to_be_bytes::<32>());
        data.extend_from_slice(&self.fee.to_be_bytes::<32>());
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        data.extend_from_slice(&self.nonce.to_be_bytes());

        keccak256(data)
    }

    // Sign the transaction using the provided keypair
    pub async fn sign_hash(&mut self, keypair: &KeyPair) -> Result<Signature, SignatureError> {
        // Sign the transaction hash using the keypair
        let hash = self.create_unsigned_hash();
        let signature = keypair.sign_hash(&hash).await?;

        // Store the signature in the transaction
        self.signature = Some(signature.clone());
        self.hash = Some(hash); // Ensure hash is set

        Ok(signature)
    }
}
