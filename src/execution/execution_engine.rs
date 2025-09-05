use super::ExecutionError;
use alloy::primitives::{Address, B256, U256};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{GasConfig, Mempool, Receipt, StateManager};
use crate::core::{Block, Transaction};
use crate::{StateTransition, state_manager};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub receipts: Vec<Receipt>,
    pub total_gas_used: U256,
    pub state_root: B256,
}

pub struct ExecutionEngine {
    pub state_manager: Arc<Mutex<StateManager>>,
    mempool: Arc<Mutex<Mempool>>,
    gas_config: GasConfig,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            state_manager: Arc::new(Mutex::new(StateManager::new())),
            mempool: Arc::new(Mutex::new(Mempool::new(1000))),
            gas_config: GasConfig::default(),
        }
    }

    // simulate execute_block, execute transactions without updating states
    pub async fn simulate_execute_block(
        &self,
        transactions: &mut [Transaction],
    ) -> Result<Vec<Transaction>> {
        // let state = self.state_manager.lock().await;
        let mut valid_transactions = Vec::new();
        let mut temp_nonces: HashMap<Address, u64> = HashMap::new();
        let mut temp_balances: HashMap<Address, U256> = HashMap::new();

        let state = self.state_manager.lock().await;

        for tx in transactions {
            // Get current state values (accounting for previous txs in this block)

            let current_nonce = temp_nonces
                .get(&tx.from)
                .copied() // Convert &u64 to u64
                .unwrap_or_else(|| state.get_nonce(&tx.from));

            let current_balance = temp_balances
                .get(&tx.from)
                .copied() // Convert &U256 to U256
                .unwrap_or_else(|| state.get_balance(&tx.from));

            // Simple checks
            let max_cost = tx.amount + (tx.gas_limit * tx.gas_price);

            if tx.nonce == current_nonce
                && tx.gas_limit >= U256::from(21000)
                && current_balance >= max_cost
            {
                valid_transactions.push(tx.clone());

                // Track changes for next transaction from same sender
                temp_nonces.insert(tx.from, current_nonce + 1);
                temp_balances.insert(tx.from, current_balance - max_cost);
            }
        }

        Ok(valid_transactions)
    }

    // execute all the transaction in a block
    pub async fn execute_block_commit(
        &self,
        block: &mut Block,
    ) -> Result<ExecutionResult, ExecutionError> {
        let mut state = self.state_manager.lock().await;
        let mut receipts = Vec::new();
        let mut total_gas_used = U256::ZERO;

        for (idx, tx) in block.transactions.iter_mut().enumerate() {
            match StateTransition::apply_transaction(&mut state, tx, &self.gas_config) {
                Ok(gas_used) => {
                    total_gas_used += gas_used;
                    let receipt = Receipt::success(tx.hash, gas_used);
                    receipts.push(receipt);

                    println!(
                        "✅ Transaction {} executed successfully, gas used: {}",
                        idx + 1,
                        gas_used
                    );
                }
                Err(e) => {
                    let gas_used = tx.gas_limit;

                    total_gas_used += gas_used;

                    let receipt = Receipt::failed(tx.hash, gas_used, e.to_string());
                    receipts.push(receipt);

                    println!(
                        "❌ Transaction {} failed: {}, gas consumed: {}",
                        idx + 1,
                        e,
                        gas_used
                    );
                }
            }
        }

        let final_state_root = state.get_state_root();

        // print messages
        println!("🏁 Block execution complete:");
        println!("   - Total transactions: {}", receipts.len());
        println!(
            "   - Successful: {}",
            receipts.iter().filter(|r| r.success).count()
        );
        println!(
            "   - Failed: {}",
            receipts.iter().filter(|r| !r.success).count()
        );
        println!("   - Total gas used: {}", total_gas_used);
        println!("   - Final state root: 0x{}", hex::encode(final_state_root));

        Ok(ExecutionResult {
            receipts,
            total_gas_used,
            state_root: final_state_root,
        })
    }

    // execution each transaction in a block
    pub async fn execute_transaction(
        &self,
        state: &mut StateManager,
        tx: &mut Transaction,
    ) -> Result<U256> {
        let _ = self.validate_transaction(&state, &tx);

        StateTransition::apply_transaction(state, tx, &self.gas_config)
            .map_err(|e| ExecutionError::TxFailed(e.to_string()))?;

        let gas_used = ExecutionEngine::calculate_gas_used(&tx);

        Ok(gas_used)
    }

    // validate each transaction against current state
    pub async fn validate_transaction(
        &self,
        state: &StateManager,
        tx: &Transaction,
    ) -> Result<(), ExecutionError> {
        if tx.gas_limit < U256::from(21000) {
            return Err(ExecutionError::InvalidTransaction(
                "Gas limit cannot be 0".to_string(),
            ));
        }

        if tx.gas_price < U256::ZERO {
            return Err(ExecutionError::InvalidTransaction(
                "Gas limit cannot be 0".to_string(),
            ));
        }

        // check if sender has enough balance for gas
        let sender_balance = state.get_balance(&tx.from);
        let max_cost = tx.max_transaction_cost();

        if sender_balance < max_cost {
            return Err(ExecutionError::InsufficientGas {
                required: max_cost,
                available: sender_balance,
            });
        };

        Ok(())
    }

    // calculate gas used by transaction
    fn calculate_gas_used(tx: &Transaction) -> U256 {
        let base_cost = U256::from(21000u64);

        if base_cost > tx.gas_limit {
            return tx.gas_limit;
        } else {
            return base_cost;
        }
    }

    // add transaction to mempool (moved from blockchain)
    pub async fn add_transaction(&self, transaction: &Transaction) -> Result<B256> {
        let mut mempool = self.mempool.lock().await;

        mempool.add_transaction(transaction)
    }

    // get all transaction from mempool
    pub async fn get_pending_transactions(&self) -> Vec<Transaction> {
        let mempool = self.mempool.lock().await;

        return mempool.get_all_transactions();
    }
}
