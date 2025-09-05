pub enum StakeError {
    InsufficientStake,
}

#[derive(Debug)]
pub enum ConsensusError {
    NoActiveValidators,
    NotMyTurn,
    StorageError(String),
    SigningFailed(String),
}

pub enum ValidatorError {}
