use super::gas_config::GasConfig;
use alloy::primitives::U256;

pub struct GasCalculator;

impl GasCalculator {
    // calculate gas cost execution the calldata
    // this is a hardcoded gas amount, because no smart contract opcode calculation yet
    pub fn calculate_instrinsic_gas(config: &GasConfig) -> U256 {
        let mut gas = config.intrinsic_gas;

        gas += config.gas_per_byte * U256::from(40);

        gas
    }

    // validate gas price is valid
    pub fn validate_gas_price(gas_price: U256, config: &GasConfig) -> bool {
        gas_price >= config.min_gas_price
    }

    // validate gas limit is valid
    pub fn validate_gas_limit(gas_limit: U256, config: &GasConfig) -> bool {
        gas_limit >= config.intrinsic_gas && gas_limit <= config.block_gas_limit
    }
}
