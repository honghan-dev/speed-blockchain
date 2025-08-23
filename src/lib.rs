pub mod blockchain;
pub mod crypto;
pub mod mempool;
pub mod rpc;
pub mod server;
pub mod storage;

// Re-export commonly used types for convenience
pub use blockchain::{Block, Blockchain, Transaction};
pub use crypto::{KeyPair, SignatureError};
pub use mempool::Mempool;
pub use rpc::SpeedRpcImpl;
pub use server::SpeedBlockchainServer;
pub use storage::Storage;

// Export anyhow::Result for convenience
pub use anyhow::Result;
