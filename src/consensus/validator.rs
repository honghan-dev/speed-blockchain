use super::error::StakeError;
use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Validator {
    pub address: Address,
    pub staked_amount: u64,
    pub is_active: bool,
    pub last_block_proposed: u64,
    pub slash_count: u32,
}

#[derive(Debug, Clone)]
pub struct ValidatorSet {
    validators: HashMap<Address, Validator>,
    total_stake: u64,
    min_stake: u64, // Minimum stake to become validator
}

impl ValidatorSet {
    // Create a new ValidatorSet with minimum stake requirement
    pub fn new(min_stake: u64) -> Self {
        Self {
            validators: HashMap::new(),
            total_stake: 0,
            min_stake,
        }
    }

    // add a new validator
    pub fn add_validator(&mut self, address: Address, stake: u64) -> Result<(), StakeError> {
        if stake < self.min_stake {
            return Err(StakeError::InsufficientStake);
        }

        let validator = Validator {
            address,
            staked_amount: stake,
            is_active: true,
            last_block_proposed: 0,
            slash_count: 0,
        };

        self.validators.insert(address, validator);
        self.total_stake += stake;

        Ok(())
    }

    // get validators that is active and have sufficient staking amount
    pub fn get_active_validators(&self) -> Vec<&Validator> {
        self.validators
            .values()
            .filter(|v| v.is_active && v.staked_amount >= self.min_stake)
            .collect()
    }

    // check if an address is a valid validator
    pub fn is_active_validator(&self, address: &Address) -> bool {
        self.validators
            .get(address)
            .map(|v| v.is_active && v.staked_amount >= self.min_stake)
            .unwrap_or(false)
    }
}
