use anyhow::Result;

// use speed_blockchain::server::SpeedBlockchainServer;
use std::net::SocketAddr;

// Database path for RocksDB
const DB_PATH: &str = "blockchain_db";
const SERVER_ADDR: &str = "127.0.0.1:8545";

fn print_banner() {
    println!(
        "
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║               🚀 SPEED BLOCKCHAIN 🚀                      ║
║                                                           ║
║              A Fast & Simple Blockchain                   ║
║                    Built with Rust                        ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
"
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    print_banner();

    let addr: SocketAddr = SERVER_ADDR.parse()?;
    println!("✅ Blockchain initialized\n");

    println!("\n🌐 Starting RPC server...");
    // let server = SpeedBlockchainServer::new(DB_PATH.to_string(), DIFFICULTY, addr)?;

    // This starts the server and runs forever (until Ctrl+C)
    println!("\n✅ Server is running! Press Ctrl+C to stop.");
    // server.run().await?;

    Ok(())
}
