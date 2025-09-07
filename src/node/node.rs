use tokio::mpsc::unbounded_channel;

// stores the running task for network and blockchain task
pub struct SpeedNode {
    network_task: tokio::task::JoinHandle<Result<()>>,
    blockchain_task: tokio::task::JoinHandle<Result<()>>,
}

impl SpeedNode {
    pub async fn new(port: u16, role: ValidatorRole) -> Result<Self> {
        // 1. Create channels, network <-> blockchain
        let (network_to_blockchain_tx, network_to_blockchain_rx) = mpsc::unbounded_channel();
        let (blockchain_to_network_tx, blockchain_to_network_rx) = mpsc::unbounded_channel();
    }
}
