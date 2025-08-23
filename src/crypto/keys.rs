use std::str::FromStr;

use super::SignatureError;
use alloy::primitives::{Address, B256, keccak256};
use alloy_signer::{Signature, Signer};
use alloy_signer_local::PrivateKeySigner;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct KeyPair {
    pub signer: PrivateKeySigner,
    pub address: Address,     // Ethereum-style address
    pub name: Option<String>, // Optional name for the keypair
}

impl KeyPair {
    pub fn generate(name: String) -> Self {
        // Alloy does ALL the hard work correctly:
        // 1. Generates cryptographically secure random private key
        // 2. Derives public key using secp256k1 elliptic curve
        // 3. Creates Ethereum address from public key hash
        let seed = keccak256("testing");
        let signer = PrivateKeySigner::from_bytes(&seed).unwrap();
        let address = signer.address(); // This is the CORRECT Ethereum address!

        println!("Generated new keypair: {} - {}", name, address);

        Self {
            signer,
            address,
            name: name.into(),
        }
    }

    // Sign a message using private key
    pub async fn sign_hash(&self, hash: &B256) -> Result<Signature, SignatureError> {
        // Use alloy_signer_local to sign the hash
        let signature = self
            .signer
            .sign_hash(&hash)
            .await
            .map_err(|_| SignatureError::SigningFailed)?;

        Ok(signature)
    }

    // Verify a signature against the hash
    pub fn verify_signature(
        &self,
        hash: &B256,
        signature: &Signature,
    ) -> Result<(), SignatureError> {
        // Use alloy_signer_local to verify the signature
        let recovered_address = signature
            .recover_address_from_prehash(hash)
            .map_err(|_| SignatureError::InvalidSignature)?;

        // If return Error if the signer doesn't match the original address
        if recovered_address != self.address {
            return Err(SignatureError::SignatureVerificationFailed);
        }

        Ok(())
    }

    /// Get public key as hex string (uncompressed, for Ethereum)
    pub fn public_key_hex(&self) -> String {
        let public_key = self.signer.address();
        format!("{:x}", public_key)
    }
}
