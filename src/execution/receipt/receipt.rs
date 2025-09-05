use alloy::primitives::{B256, U256};

// receipt to keep track of state change status

#[derive(Debug, Clone)]
pub struct Receipt {
    pub transaction_hash: B256,
    pub gas_used: U256,
    pub success: bool,
    pub error_message: Option<String>,
}

impl Receipt {
    pub fn success(transaction_hash: B256, gas_used: U256) -> Self {
        Self {
            transaction_hash,
            gas_used,
            success: true,
            error_message: None,
        }
    }

    pub fn failed(transaction_hash: B256, gas_used: U256, error: String) -> Self {
        Self {
            transaction_hash,
            gas_used,
            success: false,
            error_message: Some(error),
        }
    }
}
