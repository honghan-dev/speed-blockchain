use alloy::primitives::{Address, B256};
use anyhow::{Context, Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::block::Block;
use super::transaction::Transaction;
use crate::mempool::Mempool;
use crate::state::StateTransition;
use crate::storage::Storage;
use crate::{GasConfig, KeyPair, State};

#[derive(Clone)]
pub struct Blockchain {
    store: Arc<Mutex<Storage>>, // RocksDB storage
    mempool: Arc<Mutex<Mempool>>,
    pub state: Arc<Mutex<State>>,
    difficulty: usize,
    gas_config: GasConfig,
}

impl Blockchain {
    // starting a new blockchain
    pub fn new(path: &str, difficulty: usize) -> Result<Self> {
        let store = Storage::new(path)?;
        let state = State::new();
        let gas_config = GasConfig::default();

        Ok(Self {
            store: Arc::new(Mutex::new(store)),
            state: Arc::new(Mutex::new(state)),
            difficulty,
            mempool: Arc::new(Mutex::new(Mempool::new(100))),
            gas_config,
        })
    }

    // Simple helper to create and add transaction
    pub async fn create_transaction(
        &mut self,
        from: String,
        to: String,
        amount: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Result<String> {
        // Clone this because parse_checksummed takes ownership
        let from_addr = from.clone();
        let from_addr = Address::parse_checksummed(from_addr, None).unwrap();

        // @todo //////// Basic signing /////////
        // to remove once we have proper wallet
        // This is a simplified example, in real blockchain, this would be done in the wallet
        // create the transaction
        let mut transaction = Transaction::new(from, to, amount, gas_limit, gas_price)
            .map_err(|e| anyhow!("Failed to create transaction: {}", e))?;

        // Get current nonce for the sender
        {
            let state = self.state.lock().await;
            let current_nonce = state.get_nonce(&from_addr);

            transaction.nonce = current_nonce; // Set correct nonce
        } // dropping lock
        // create a keypair for signing
        let keypair = KeyPair::generate("default".to_string());

        println!(
            "🔑 Blockchain - Generated keypair for transaction: {}",
            keypair.address
        );

        // sign the transaction
        let signature = transaction.sign_hash(&keypair).await?;

        transaction.signature = Some(signature);

        // If valid, update the real state
        let tx_id = self.add_transaction(transaction, &keypair).await?;
        Ok(hex::encode(tx_id))
    }

    // Mine all pending transactions into a block
    pub async fn mine_pending_transactions(&mut self) -> Result<()> {
        let mut mempool = self.mempool.lock().await;
        // Get all pending transactions
        let pending_transactions = mempool.get_all_transactions();

        // execute transaction and update state
        let mut valid_txs = vec![];
        {
            let mut state = self.state.lock().await;

            for tx in pending_transactions.iter() {
                let mut tx_copy = tx.clone();
                match StateTransition::apply_transaction(&mut state, &mut tx_copy, &self.gas_config)
                {
                    Ok(_) => {
                        valid_txs.push(tx.clone());
                    }
                    Err(e) => {
                        println!("❌ Invalid transaction skipped: {}", e);
                        continue;
                    }
                }
            }
        }

        println!(
            "🎯 Mining block with {} transactions...",
            pending_transactions.len()
        );

        // Get previous block info
        let last_index = self.get_last_index().await?;
        let prev_hash = if last_index == 0 {
            B256::ZERO
        } else {
            self.get_block_hash_by_index(&last_index)
                .await?
                .ok_or_else(|| anyhow!("❌ No block found at index: {}", last_index))?
        };

        // Mine the block
        let new_block = Block::new(
            last_index + 1,
            pending_transactions,
            prev_hash,
            self.difficulty,
        );

        let store = self.store.lock().await;
        // Store block hash -> block mapping
        store
            .put_index_to_block_hash(&new_block.index, &new_block.block_hash)
            .with_context(|| format!("Failed to store block at index: {}", new_block.index))?;

        // Store index -> block hash mapping
        store
            .put_block_hash_to_block(&new_block.block_hash, &new_block)
            .with_context(|| format!("Failed to store block at index: {}", new_block.index))?;

        store
            .put_last_index(&new_block.index)
            .context("Failed to update last_index")?;

        // Clear mempool (all transactions are now in the block)
        mempool.clear_all_transactions();

        println!("🎉 Block {} mined! Mempool cleared.", new_block.index);

        Ok(())
    }

    // Add transaction to mempool (no balance checking)
    pub async fn add_transaction(
        &mut self,
        transaction: Transaction,
        keypair: &KeyPair,
    ) -> Result<B256> {
        let mut mempool = self.mempool.lock().await;
        let tx_id = mempool.add_transaction(&transaction, keypair)?;
        Ok(tx_id)
    }

    // get last index
    pub async fn get_last_index(&self) -> Result<u64> {
        let store = self.store.lock().await;
        let last_index: u64 = match store
            .get_last_index()
            .context("Failed to retrieve last block index")?
        {
            Some(index) => index,
            None => 0, // No blocks exist
        };
        Ok(last_index)
    }

    // get block hash by index
    pub async fn get_block_hash_by_index(&self, index: &u64) -> Result<Option<B256>> {
        let store = self.store.lock().await;
        store.get_block_hash_from_index(index)
    }

    // get a block by index
    // 1) Get block hash from index
    // 2) Get block data from block hash
    pub async fn get_block_by_index(&self, index: &u64) -> Result<Block> {
        let store = self.store.lock().await;

        let block_hash = match store.get_block_hash_from_index(&index)? {
            Some(hash) => hash,
            None => {
                return Err(anyhow!("❌ No block found at index: {}", index));
            }
        };

        let block = match store.get_block_from_block_hash::<Block>(&block_hash)? {
            // ✅ Regular match instead of let Some
            Some(block) => block,
            None => {
                return Err(anyhow!(
                    "❌ Block data not found for hash: 0x{}",
                    hex::encode(&block_hash)
                ));
            }
        };

        Ok(block)
    }

    // pub async fn print_chain(&self) -> Result<()> {
    //     let block_count = self
    //         .get_block_count()
    //         .await
    //         .context("Failed to get blockchain length")?;

    //     if block_count == 0 {
    //         println!("Blockchain is empty");
    //         return Ok(());
    //     }

    //     println!("Blockchain contains {} blocks:", block_count);
    //     println!("{}", "=".repeat(50));

    //     for i in 1..=block_count {
    //         match self.get_block_by_index(i).await? {
    //             Some(block) => {
    //                 println!("Block {}: {:?}", i, block);
    //             }
    //             None => {
    //                 return Err(anyhow!("Block {} is missing from the chain", i));
    //             }
    //         }
    //     }

    //     Ok(())
    // }

    ////// Getting functions //////

    // Check if there are transactions ready to mine
    pub async fn has_pending_transactions(&self) -> Result<bool> {
        let mempool = self.mempool.lock().await;
        Ok(mempool.has_transactions())
    }
}
