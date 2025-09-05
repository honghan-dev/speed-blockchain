use super::error::ConsensusError;
use crate::consensus::ValidatorSet;
use alloy::primitives::Address;
use rand_chacha::ChaCha20Rng;
use rand_core::SeedableRng;
use rand_core::TryRngCore;

pub struct ProposerSelection {
    validator_set: ValidatorSet,
    randomness_seed: [u8; 32], // Derived from previous block
}

impl ProposerSelection {
    // instantiate a new proposerselection
    pub fn new(validator_set: ValidatorSet, randomness_seed: [u8; 32]) -> Self {
        Self {
            validator_set,
            randomness_seed,
        }
    }

    pub fn selector_proposer(&self, slot: u64) -> Result<Address, ConsensusError> {
        let active_validators = self.validator_set.get_active_validators();

        if active_validators.is_empty() {
            return Err(ConsensusError::NoActiveValidators);
        }

        // Create deterministic randomness for this slot
        let mut seed = self.randomness_seed;
        seed[0..8].copy_from_slice(&slot.to_le_bytes());

        let mut rng = ChaCha20Rng::from_seed(seed);

        // Generate deterministic random value without gen_range
        let random_bytes = rng.try_next_u64().unwrap();

        // Weighted random selection based on stake
        let total_stake: u64 = active_validators.iter().map(|v| v.staked_amount).sum();
        let random_stake = random_bytes % total_stake;

        let mut cumulative_stake = 0;

        for validator in active_validators {
            cumulative_stake += validator.staked_amount;

            if random_stake < cumulative_stake {
                return Ok(validator.address);
            }
        }

        unreachable!("Should have selected a validator");
    }
}
