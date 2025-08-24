use alloy::primitives::U256;

#[derive(Clone)]
pub struct GasConfig {
    pub intrinsic_gas: U256,   // Base cost for any transaction
    pub gas_per_byte: U256,    // Cost per byte of data
    pub min_gas_price: U256,   // Minimum gas price
    pub block_gas_limit: U256, // Maximum gas per block
}

impl Default for GasConfig {
    fn default() -> Self {
        Self {
            intrinsic_gas: U256::from(21_000),        // Like Ethereum
            gas_per_byte: U256::from(4),              // Cost for transaction data
            min_gas_price: U256::from(1_000_000_000), // 1 gwei
            block_gas_limit: U256::from(1_000_000),   // 1M gas per block
        }
    }
}
