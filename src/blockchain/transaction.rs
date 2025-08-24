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
    pub timestamp: u64, // When transaction was created
    pub nonce: u64,     // Nonce for transaction uniqueness

    // GAS FIELDS
    pub gas_limit: U256,
    pub gas_price: U256,
    pub gas_used: U256,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>, // Optional signature for the transaction

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<B256>, // Optional hash for the transaction
}

impl Transaction {
    pub fn new(
        from: String,
        to: String,
        amount: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<Self, String> {
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
            gas_limit: U256::from(gas_limit),
            gas_price: U256::from(gas_price),
            gas_used: U256::from(0),
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
        data.extend_from_slice(&self.gas_limit.to_be_bytes::<32>());
        data.extend_from_slice(&self.gas_price.to_be_bytes::<32>());
        data.extend_from_slice(&self.gas_used.to_be_bytes::<32>());
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

    // Helper methods for gas calculations
    pub fn max_transaction_cost(&self) -> U256 {
        self.amount + (self.gas_limit * self.gas_price)
    }

    pub fn actual_gas_fee(&self) -> U256 {
        self.gas_used * self.gas_price
    }

    pub fn gas_refund(&self) -> U256 {
        (self.gas_limit - self.gas_used) * self.gas_price
    }
}
