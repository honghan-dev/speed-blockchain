use core::fmt;

use alloy::primitives::U256;

#[derive(Debug, Clone)]
pub enum StateTransitionError {
    InsufficientBalance { has: U256, needs: U256 },
    InvalidNonce { expected: u64, got: u64 },
    GasPriceTooLow,
    BalanceOverflow,
    SameAddress,
    InvalidGasLimit,
    InsufficientGas { provided: U256, required: U256 },
}

impl fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateTransitionError::InsufficientBalance { has, needs } => {
                write!(f, "Insufficient balance: has {}, needs {}", has, needs)
            }
            StateTransitionError::InvalidNonce { expected, got } => {
                write!(f, "Invalid nonce: expected {}, got {}", expected, got)
            }
            StateTransitionError::BalanceOverflow => {
                write!(f, "Balance overflow occurred")
            }
            StateTransitionError::SameAddress => {
                write!(f, "Sender and receiver addresses are the same")
            }
            StateTransitionError::GasPriceTooLow => {
                write!(f, "Gas price is too low")
            }
            StateTransitionError::InvalidGasLimit => {
                write!(f, "Invalid gas limit set")
            }
            StateTransitionError::InsufficientGas { provided, required } => {
                write!(
                    f,
                    "Insufficient gas provided: provided: {}, required {}",
                    provided, required
                )
            }
        }
    }
}
