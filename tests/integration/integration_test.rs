#[cfg(test)]
mod integration_test {
    use alloy::primitives::{B256, U256};
    use alloy_signer::Signature;
    use anyhow::Result;
    use speed_blockchain::{Blockchain, KeyPair, Transaction};
    use std::str::FromStr;
    use tokio;

    const DB_PATH: &str = "blockchain_db";
    const TO_GWEI: u128 = 1_000_000_000;
    const TO_ETH: u128 = 1_000_000_000_000_000_000;

    #[tokio::test]
    async fn test_complete_block_production_flow() -> Result<()> {
        println!("🧪 Starting complete block production integration test");

        let (blockchain, _) = setup_test_blockchain().await?;

        let (alice, bob) = setup_test_accounts(&blockchain).await?;

        let transactions = create_test_transactions(&alice, &bob).await?;

        // add each transaction into the mempool
        for tx in transactions {
            blockchain.execution_engine.add_transaction(&tx).await?;
        }
        println!("✅ Added transactions to mempool");

        // Step 5: Verify mempool state before block production
        let pending = blockchain.execution_engine.get_pending_transactions().await;
        assert!(!pending.is_empty(), "Mempool should contain transactions");
        println!("Mempool contains {} transactions", pending.len());

        // Step 6: Record initial state
        let initial_alice_balance = {
            let state = blockchain.execution_engine.state_manager.lock().await;
            state.get_balance(&alice.address)
        };
        println!("Alice initial balance: {}", initial_alice_balance);

        let produced_block = blockchain.produce_block().await?;
        println!(
            "Block produced successfully: #{}",
            produced_block.header.index
        );

        Ok(())
    }

    // Setup blockchain
    async fn setup_test_blockchain() -> Result<(Blockchain, KeyPair)> {
        println!("🔧 Setting up test blockchain...");

        // create validator keypair
        let validator_keypair = KeyPair::generate("Validator".into());
        let validator_stake = 10000u64;

        let validators = vec![(validator_keypair.address, validator_stake)];

        let blockchain = Blockchain::new(
            DB_PATH,
            1000, // min_stake
            5,    // slot duration seconds
            validators,
            Some(validator_keypair.clone()),
        )?;

        println!(
            "✅ Blockchain setup complete with validator: {}",
            validator_keypair.address
        );

        Ok((blockchain, validator_keypair))
    }

    // Setup and fund test account
    async fn setup_test_accounts(blockchain: &Blockchain) -> Result<(KeyPair, KeyPair)> {
        println!("💰 Setting up test accounts...");

        let alice = KeyPair::generate("alice".into());
        let bob = KeyPair::generate("bob".into());

        // Fund Alice for transactions
        let mut state_manager = blockchain.execution_engine.state_manager.lock().await;

        // fund alice
        state_manager.fund_account(&alice.address, U256::from(100 * TO_ETH));

        println!("✅ Alice funded: {} | Bob: {}", alice.address, bob.address);
        Ok((alice, bob))
    }

    // creates test transactions and sign it
    async fn create_test_transactions(alice: &KeyPair, bob: &KeyPair) -> Result<Vec<Transaction>> {
        println!("📝 Creating test transactions...");

        let mut transactions = Vec::new();

        let mut transaction = Transaction {
            from: alice.address,
            to: bob.address,
            amount: U256::from(1 * TO_ETH),
            timestamp: current_timestamp(),
            nonce: 0,
            gas_limit: U256::from(21000),
            gas_price: U256::from(TO_GWEI), // 1gwei
            signature: create_dummy_signature(),
            hash: B256::ZERO,
        };

        let tx_hash = transaction.calculate_hash();

        let signature = alice.sign_hash(&tx_hash).await?;

        // Update transaction with signature and hash
        transaction.signature = signature;
        transaction.hash = tx_hash;

        transactions.push(transaction);

        println!("✅ Created test transactions");
        Ok(transactions)
    }

    // helper method

    // create current timestamp
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    // create a dummy signature before replacing it with an actual signature
    fn create_dummy_signature() -> Signature {
        return Signature::from_str(
        "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
        ).unwrap();
    }
}
