use anyhow::{Context, Result};
use chrono::Utc;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use std::net::SocketAddr;
use tokio::signal;
use tokio::time::{Duration, interval};

use crate::blockchain::Blockchain;
use crate::rpc::SpeedRpcImpl;
use crate::rpc::rpc::SpeedBlockchainRpcServer;

#[derive(Clone)]
pub struct SpeedBlockchainServer {
    blockchain: Blockchain,
    addr: SocketAddr,
}

impl SpeedBlockchainServer {
    // Create a new Speed Blockchain server
    pub fn new(blockchain_path: String, difficulty: usize, addr: SocketAddr) -> Result<Self> {
        let blockchain = Blockchain::new(&blockchain_path, difficulty)?;
        Ok(Self { blockchain, addr })
    }

    // Start the server and listen for RPC calls
    pub async fn start(&self) -> Result<ServerHandle> {
        println!("🔧 Initializing Speed Blockchain Server...");

        // Create RPC implementation
        let rpc_impl = SpeedRpcImpl::new(self.blockchain.clone());

        let server = ServerBuilder::default().build(self.addr).await?;

        println!("🚀 Speed Blockchain RPC server starting on {}", self.addr);
        println!("✅ Server is running!");
        println!("📡 You can send RPC calls to: http://{}", self.addr);
        // self.print_available_methods();

        // Start the server
        let handle = server.start(rpc_impl.into_rpc());

        Ok(handle)
    }

    pub async fn run(&self) -> Result<()> {
        let handle = self.start().await?;

        // Start mining blocks periodically
        self.start_block_mining().await;

        // Wait for shutdown signal
        self.wait_for_shutdown().await;

        println!("🛑 Shutting down server...");
        handle.stop()?;
        println!("✅ Server stopped gracefully");

        Ok(())
    }

    // Mine new blocks periodically
    pub async fn start_block_mining(&self) {
        println!("⛏️  Starting node mining every 2 seconds");

        let server = self.clone();

        let _ = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(5));

            // Wait for the first tick
            ticker.tick().await;

            loop {
                // create timer for 2 seconds
                ticker.tick().await;

                match server.process_mining().await {
                    Ok(Some(_stat)) => {
                        println!("🎉 [] Block mined in {:?}", Utc::now().format("%H:%M:%S"),);
                    }
                    Ok(None) => {
                        println!(
                            "💤 [{}] No transactions to mine",
                            Utc::now().format("%H:%M:%S")
                        );
                    }
                    Err(e) => {
                        eprintln!("❌ Mining cycle failed: {}", e);
                        continue;
                    }
                }
            }
        });
    }

    async fn process_mining(&self) -> Result<Option<bool>> {
        // check if there are transactions to mine
        if self.blockchain.has_pending_transactions().await? {
            println!("🔍 Starting mining transaction...");
        } else {
            println!("🔍 No transactions to mine, skipping...");
            return Ok(None);
        }

        // Get the latest block
        // let latest_block = self.blockchain.get_latest_block().await?;

        // Start current time for mining
        // let mining_start = Instant::now();

        let mut blockchain = self.blockchain.clone();

        // Mine pending transactions
        let result = blockchain
            .mine_pending_transactions()
            .await
            .context("Failed to mine pending transactions");

        match result {
            Ok(_) => {
                // Successfully mined a block
                Ok(Some(true))
            }
            Err(e) => {
                // No transactions to mine or other error
                if e.to_string().contains("No transactions to mine") {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn wait_for_shutdown(&self) {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        tokio::select! {
            _ = ctrl_c => {
                println!("\n📡 Received Ctrl+C signal");
            },
            _ = terminate => {
                println!("\n📡 Received terminate signal");
            },
        }
    }
}
