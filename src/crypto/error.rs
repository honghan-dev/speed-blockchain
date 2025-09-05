#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    #[error("Signing failed")]
    SigningFailed,
    #[error("Invalid private key format")]
    InvalidPrivateKey,
    #[error("Invalid public key format")]
    InvalidPublicKey,
    #[error("Invalid signature format")]
    InvalidSignature,
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
    #[error("ECDSA error: {0}")]
    EcdsaError(String),
    #[error("Invalid message hash")]
    HashMismatch,
}
