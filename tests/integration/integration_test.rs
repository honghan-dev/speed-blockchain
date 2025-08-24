use alloy::primitives::U256;
use anyhow::Result;
use speed_blockchain::{Blockchain, KeyPair};
use tempfile::TempDir;

// Helper function to create a test blockchain with temporary storage
fn create_test_blockchain() -> Result<(Blockchain, TempDir)> {
    let temp_dir = TempDir::new()?;
    let blockchain = Blockchain::new(temp_dir.path().to_str().unwrap(), 1)?; // Low difficulty for fast tests
    Ok((blockchain, temp_dir))
}

fn create_keypair(name: String) -> KeyPair {
    KeyPair::generate(name)
}

// Helper functions for realistic amounts
fn ether_to_wei(ether: u64) -> U256 {
    U256::from(ether) * U256::from(10_u64.pow(18)) // 1 ETH = 10^18 wei
}

fn gwei_to_wei(gwei: u64) -> U256 {
    U256::from(gwei) * U256::from(10_u64.pow(9)) // 1 gwei = 10^9 wei
}

#[tokio::test]
async fn test_blockchain_integration() -> Result<()> {
    // create a keypair for signing
    eprintln!("🧪 Testing complete transaction flow...");

    let (blockchain, _temp_dir) = create_test_blockchain()?;

    // Create wallets
    let alice = create_keypair("alice".to_string());
    let bob = create_keypair("bob".to_string());

    {
        // acquire state lock
        let mut state = blockchain.state.lock().await;

        eprintln!("💰 Funding accounts...");
        state.fund_account(&alice.address, ether_to_wei(100));

        state.fund_account(&bob.address, ether_to_wei(100));

        let alice_initial = state.get_balance(&alice.address);
        let bob_initial = state.get_balance(&bob.address);

        // Check initial balances
        assert_eq!(alice_initial, ether_to_wei(100));
        assert_eq!(bob_initial, ether_to_wei(100));
    }

    eprintln!("📤 Creating and processing transaction from Alice to Bob...");
    let chain = &mut blockchain.clone();

    // Transaction 1: Send 2 ETH (realistic personal transfer)
    let tx1_amount = 2_000_000_000_000_000_000; // 2 ETH
    let tx1_gas_limit = 21_000; // Standard ETH transfer
    let tx1_gas_price = 20_000_000_000; // 20 gwei - typical gas price

    let tx_hash = chain
        .create_transaction(
            alice.address.to_string(),
            bob.address.to_string(),
            tx1_amount,
            tx1_gas_limit,
            tx1_gas_price,
        )
        .await?;
    eprintln!("✅ Transaction created with hash: {}", tx_hash);

    // Transaction 1: Send 2 ETH (realistic personal transfer)
    let tx2_amount = 1_500_000_000_000_000_000; // 2 ETH
    let tx2_gas_limit = 21_000; // Standard ETH transfer
    let tx2_gas_price = 20_000_000_000; // 20 gwei - typical gas price

    let tx_hash = chain
        .create_transaction(
            alice.address.to_string(),
            bob.address.to_string(),
            tx2_amount,
            tx2_gas_limit,
            tx2_gas_price,
        )
        .await?;

    eprintln!("✅ Transaction created with hash: {}", tx_hash);
    chain.mine_pending_transactions().await?;

    {
        // acquire state lock
        let state = blockchain.state.lock().await;

        let alice_final = state.get_balance(&alice.address);
        let bob_final = state.get_balance(&bob.address);
        eprintln!("Alice final balance: {}", alice_final);
        eprintln!("Bob final balance: {}", bob_final);
    }
    // Check final balances
    // assert_eq!(alice_final, U256::from(500_000));
    // assert_eq!(bob_final, U256::from(1_500_000));
    Ok(())
}
