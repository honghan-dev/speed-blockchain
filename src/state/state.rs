use crate::account::Account;
use alloy::primitives::{Address, B256, U256, keccak256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub accounts: HashMap<Address, Account>,
    pub state_root: B256,
}

impl State {
    // Initial state with empty accounts and zero state root
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            state_root: B256::ZERO,
        }
    }

    // Get account by address, return a new account if not found
    pub fn get_account(&self, address: &Address) -> Account {
        self.accounts
            .get(address)
            .cloned()
            .unwrap_or_else(|| Account::new(*address))
    }

    // Set account in the state and recalculate state root
    pub fn set_account(&mut self, address: Address, account: Account) {
        if account.balance == U256::ZERO && account.nonce == 0 {
            self.accounts.remove(&address);
        } else {
            self.accounts.insert(address, account);
        }

        self.calculate_state_root();
    }

    fn calculate_state_root(&mut self) {
        // Simple state root calculation by hashing concatenated account data
        let mut data = Vec::new();

        let mut addresses: Vec<&Address> = self.accounts.keys().collect();
        addresses.sort(); // Ensure consistent order

        for address in addresses {
            let account = &self.accounts[address];
            data.extend_from_slice(address.as_slice());
            data.extend_from_slice(&account.balance.to_be_bytes::<32>());
            data.extend_from_slice(&account.nonce.to_be_bytes());
        }

        self.state_root = if data.is_empty() {
            B256::ZERO
        } else {
            keccak256(&data)
        };
    }

    /// Get state root
    pub fn get_state_root(&self) -> B256 {
        self.state_root
    }

    /// Get balance of an address
    pub fn get_balance(&self, address: &Address) -> U256 {
        self.get_account(address).balance
    }

    // Get nonce of an address
    pub fn get_nonce(&self, address: &Address) -> u64 {
        self.get_account(address).nonce
    }

    /// Get total number of accounts
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Fund account (for testing)
    pub fn fund_account(&mut self, address: &Address, amount: U256) {
        let mut account = self.get_account(&address);
        account.balance += amount;
        self.set_account(address.clone(), account);
        println!("💰 State - Funded {} with {} tokens", address, amount);
    }
}
