use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub balance: U256,
    pub nonce: u64,
    pub address: Address,
}

impl Account {
    // Create a new account with zero balance and nonce
    pub fn new(address: Address) -> Self {
        Self {
            balance: U256::ZERO,
            nonce: 0,
            address,
        }
    }
}
