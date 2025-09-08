use std::fs;

use alloy::primitives::Address;
use anyhow::Result;
use tokio::{signal, sync::mpsc::unbounded_channel};

use crate::{
    Blockchain, DB_PATH, KeyPair, MIN_STAKE, NetworkService, SLOT_DURATION, ValidatorRole,
    core::BlockchainService,
};

// stores the running task for network and blockchain task
pub struct SpeedNode {
    network_task: tokio::task::JoinHandle<Result<()>>,
    blockchain_task: tokio::task::JoinHandle<Result<()>>,
}

// load validators address and stake from json file, for testing purposes
fn load_validators_from_json() -> Result<Vec<(Address, u64)>> {
    let data = fs::read_to_string("validators.json")?;
    let addresses: Vec<(&str, u64)> = serde_json::from_str(&data)?;

    let mut validators = Vec::new();
    for (addr, stake) in addresses {
        let addr = Address::parse_checksummed(addr, Some(1))
            .map_err(|_| anyhow::anyhow!("Invalid address: {}", addr))?;
        validators.push((addr, stake));
    }

    Ok(validators)
}

impl SpeedNode {
    pub async fn new(port: u16, role: ValidatorRole) -> Result<Self> {
        println!("🚀 Starting SpeedNode on port {} as {:?}", port, role);

        // Setup KeyPair for this node
        let keypair = KeyPair::generate("node".to_string());

        // 1. Create channels, network <-> blockchain
        let (network_to_blockchain_tx, network_to_blockchain_rx) = unbounded_channel();
        let (blockchain_to_network_tx, blockchain_to_network_rx) = unbounded_channel();

        let validators: Vec<(Address, u64)> = load_validators_from_json()?;

        // 2. Initialize core blockchain components
        let blockchain = Blockchain::new(
            DB_PATH,
            MIN_STAKE,
            SLOT_DURATION,
            validators,
            Some(keypair.clone()),
        )?;

        println!("🔑 Node validator address: {}", keypair.address);

        // 3. Create network service
        let mut network_service =
            NetworkService::new(network_to_blockchain_tx, blockchain_to_network_rx).await?;

        // 4. Create blockchain service
        let mut blockchain_service = BlockchainService::new(
            network_to_blockchain_rx,
            blockchain_to_network_tx,
            blockchain,
            keypair,
            role,
        );

        // 5. Start network service in separate task
        let network_task = {
            tokio::spawn(async move {
                println!("📡 Starting network service...");
                network_service.start(port).await?;
                network_service.run().await
            })
        };

        // 6. Start blockchain service in separate task
        let blockchain_task = tokio::spawn(async move {
            println!("⛓️  Starting blockchain service...");
            blockchain_service.run().await
        });

        println!("✅ SpeedNode started successfully!");

        Ok(SpeedNode {
            network_task,
            blockchain_task,
        })
    }

    pub async fn run(self) -> Result<()> {
        println!("🏃 SpeedNode running... Press Ctrl+C to shutdown");

        tokio::select! {
            // Wait for either service to complete/error
            network_result = self.network_task => {
                match network_result {
                    Ok(Ok(())) => println!("📡 Network service completed"),
                    Ok(Err(e)) => println!("❌ Network service error: {}", e),
                    Err(e) => println!("❌ Network task panicked: {}", e),
                }
            }

            blockchain_result = self.blockchain_task => {
                match blockchain_result {
                    Ok(Ok(())) => println!("⛓️  Blockchain service completed"),
                    Ok(Err(e)) => println!("❌ Blockchain service error: {}", e),
                    Err(e) => println!("❌ Blockchain task panicked: {}", e),
                }
            }

            // Handle shutdown signal (Ctrl+C)
            _ = signal::ctrl_c() => {
                println!("🛑 Shutdown signal received");
            }
        }

        println!("👋 SpeedNode shutting down...");
        Ok(())
    }
}
