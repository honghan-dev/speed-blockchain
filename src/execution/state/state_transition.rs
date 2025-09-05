use crate::error::StateTransitionError;
use crate::{GasCalculator, GasConfig, StateManager, Transaction};
use alloy::primitives::U256;
use anyhow::Result;

pub struct StateTransition;

// execution layer

impl StateTransition {
    pub fn apply_transaction(
        state: &mut StateManager,
        tx: &mut Transaction,
        config: &GasConfig,
    ) -> Result<U256, StateTransitionError> {
        println!(
            "🔄 Processing: {} → {}, amount: {}, gas_limit: {}, gas_price: {}",
            tx.from, tx.to, tx.amount, tx.gas_limit, tx.gas_price
        );

        // Gas price config validation
        if !GasCalculator::validate_gas_price(tx.gas_price, config) {
            return Err(StateTransitionError::GasPriceTooLow);
        }

        // Gas limit config validation
        if !GasCalculator::validate_gas_limit(tx.gas_limit, config) {
            return Err(StateTransitionError::InvalidGasLimit);
        }

        let intrinsic_gas = GasCalculator::calculate_instrinsic_gas(config);
        if tx.gas_limit < intrinsic_gas {
            return Err(StateTransitionError::InsufficientGas {
                provided: tx.gas_limit,
                required: intrinsic_gas,
            });
        }

        // STEP 1: Basic validation
        if tx.from == tx.to {
            return Err(StateTransitionError::SameAddress);
        }

        let mut sender = state.get_account(&tx.from);
        let mut recipient = state.get_account(&tx.to);

        println!(
            "📖 Sender: balance={}, nonce={}",
            sender.balance, sender.nonce
        );
        println!("📖 Recipient: balance={}", recipient.balance);

        // Check sender can afford maximum possible cost
        let max_cost = tx.max_transaction_cost();
        if sender.balance < max_cost {
            println!(
                "❌ Insufficient balance! Has {}, needs {}",
                sender.balance, max_cost
            );
            return Err(StateTransitionError::InsufficientBalance {
                has: sender.balance,
                needs: max_cost,
            });
        }

        // 3b. Prevent replay attacks
        if tx.nonce != sender.nonce {
            println!(
                "❌ Replay attack attempt! Expected nonce {}, got {}",
                sender.nonce, tx.nonce
            );
            return Err(StateTransitionError::InvalidNonce {
                expected: sender.nonce,
                got: tx.nonce,
            });
        }

        // 3c. Prevent integer overflow
        if recipient.balance.checked_add(tx.amount).is_none() {
            println!("❌ Overflow attack attempt!");
            return Err(StateTransitionError::BalanceOverflow);
        }

        let gas_used = intrinsic_gas;
        let gas_cost = gas_used * tx.gas_price;
        let total_cost = tx.amount + gas_cost;

        // STEP 4: Apply state changes
        sender.nonce += 1;
        // deduct total cost from sender
        sender.balance = sender.balance.checked_sub(total_cost).unwrap();
        // add amount to recipient
        recipient.balance = recipient.balance.checked_add(tx.amount).unwrap();

        println!(
            "✅ New balances - Sender: {}, Recipient: {}",
            sender.balance, recipient.balance
        );

        state.set_account(tx.from, sender);
        state.set_account(tx.to, recipient);

        println!(
            "🌳 New state root: 0x{}",
            hex::encode(state.get_state_root())
        );

        Ok(gas_used)
    }
}
