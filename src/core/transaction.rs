use serde::{Deserialize, Serialize};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
// evm compatible fields
use alloy::primitives::{Address, B256, U256, keccak256};
use alloy_signer::Signature;

use crate::crypto::SignatureError;

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

    // Signature
    pub signature: Signature,
    // Transaction hash
    pub hash: B256,
}

impl Transaction {
    pub fn new(
        from: String,
        to: String,
        amount: u64,
        gas_limit: u64,
        gas_price: u64,
        signature: Signature,
        hash: B256,
    ) -> Result<Self, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let from = Address::from_str(&from.as_str()).expect("Invalid from address");
        let to = Address::from_str(&to.as_str()).expect("Invalid to address");

        let tx = Self {
            from,
            to,
            amount: U256::from(amount),
            gas_limit: U256::from(gas_limit),
            gas_price: U256::from(gas_price),
            timestamp,
            nonce: 0, // Default nonce
            signature,
            hash,
        };

        Ok(tx)
    }

    // verify signature
    pub fn verify_signature(&self) -> Result<Address, SignatureError> {
        let calculated_hash = self.calculate_hash();

        if calculated_hash != self.hash {
            return Err(SignatureError::HashMismatch);
        }

        let recovered_address = self
            .signature
            .recover_address_from_prehash(&calculated_hash)
            .unwrap();

        Ok(recovered_address)
    }

    /// Check if signature is valid
    pub fn is_signature_valid(&self) -> bool {
        match self.verify_signature() {
            Ok(recovered_address) => recovered_address == self.from,
            Err(_) => false,
        }
    }

    // calculate transaction hash, excluding Signature
    pub fn calculate_hash(&self) -> B256 {
        let mut data = Vec::new();

        data.extend_from_slice(self.from.as_slice());
        data.extend_from_slice(self.to.as_slice());
        data.extend_from_slice(&self.amount.to_be_bytes::<32>());
        data.extend_from_slice(&self.gas_limit.to_be_bytes::<32>());
        data.extend_from_slice(&self.gas_price.to_be_bytes::<32>());
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        data.extend_from_slice(&self.nonce.to_be_bytes());

        // we don't include signature here because of circular dependency
        keccak256(data)
    }

    // Helper methods for gas calculations
    pub fn max_transaction_cost(&self) -> U256 {
        self.amount + (self.gas_limit * self.gas_price)
    }
}
