use anyhow::Result;
use libp2p::{
    Swarm, SwarmBuilder,
    gossipsub::{self, Behaviour, IdentTopic, Message, MessageId},
    mdns, noise,
    swarm::NetworkBehaviour,
    tcp, yamux,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use tokio::{io, io::AsyncBufReadExt, select};

// This combines all our P2P protocols into one behavior
#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
    pub gossipsub: Behaviour,         // For broadcasting messages
    pub mdns: mdns::tokio::Behaviour, // For discovering local peers
}

// Define what messages our blockchain nodes will exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockchainMessage {
    NewBlock {
        block_hash: String,
        block_number: u64,
        transactions: Vec<String>,
    },
    NewTransaction {
        tx_hash: String,
        from: String,
        to: String,
        amount: u64,
    },
    BlockRequest {
        from_block: u64,
        to_block: u64,
    },
    BlockResponse {
        blocks: Vec<String>,
    },
}

pub struct BlockchainNode {
    pub swarm: Swarm<BlockchainBehaviour>,
    pub topics: Vec<IdentTopic>,
}

impl BlockchainNode {
    pub async fn new() -> Result<Self> {
        let swarm = SwarmBuilder::with_new_identity() // Let libp2p generate identity
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                // Set a custom gossipsub configuration
                let gossipsub_config = gossipsub::ConfigBuilder::default().build()?;

                // build a gossipsub network behaviour
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )?;

                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;

                Ok(BlockchainBehaviour { gossipsub, mdns })
            })?
            .build();

        let topics = vec![
            IdentTopic::new("blockchain-blocks"),
            IdentTopic::new("blockchain-transactions"),
            IdentTopic::new("blockchain-sync"),
        ];

        Ok(BlockchainNode { swarm, topics })
    }
}
