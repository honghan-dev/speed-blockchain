use alloy::primitives::{Address, B256};
use alloy_signer::Signature;
use anyhow::Result;
use libp2p::{
    Swarm, SwarmBuilder,
    futures::StreamExt,
    gossipsub::{self, Behaviour, IdentTopic},
    mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{Block, Transaction};

#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
    pub gossipsub: Behaviour,         // For broadcasting messages
    pub mdns: mdns::tokio::Behaviour, // For discovering local peers
}

// Define message from network -> blockchain
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    NewBlock {
        block: Block,
        proposer_id: Address,
        signature: Signature,
    },
    Attestation {
        block_hash: B256,
        validator_id: Address,
        vote: AttestationVote,
        signature: Signature,
    },
    NewTransaction {
        transaction: Transaction,
        from_peer: Address,
    },
}

// Define blockchain -> network message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockchainMessage {
    NewBlock {
        block: Block,
        proposer: Address,
        signature: Signature,
    },
    Attestation {
        block_hash: B256,
        validator: Address,
        vote: AttestationVote,
        signature: Signature,
    },
    NewTransaction {
        transaction: Transaction,
    },
}

// simple vote type for attestation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttestationVote {
    Accept,                    // Block is valid
    Reject { reason: String }, // Block is invalid with reason
}

// Main function
// 1. Handle swarm events, eg. connecting to peers
// 2. Handle message network -> blockchain layer
// 3. Handle message from blockchain layer -> network -> other nodes
pub struct NetworkService {
    pub swarm: Swarm<BlockchainBehaviour>,
    pub topics: Vec<IdentTopic>,
    // Channels for blockchain communication
    to_blockchain_sender: UnboundedSender<NetworkMessage>,
    from_blockchain_receiver: UnboundedReceiver<BlockchainMessage>,
}

impl NetworkService {
    // starting a new node instance
    pub async fn new(
        to_blockchain: UnboundedSender<NetworkMessage>,
        from_blockchain: UnboundedReceiver<BlockchainMessage>,
    ) -> Result<(Self)> {
        // this creates a new identity in every new run
        let swarm = SwarmBuilder::with_new_identity() // Let libp2p generate identity
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                // Set a custom gossipsub configuration
                // with strict mode, only allows validated message to spread
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .build()?;

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

        Ok(NetworkService {
            swarm,
            topics,
            to_blockchain_sender: to_blockchain,
            from_blockchain_receiver: from_blockchain,
        })
    }

    pub async fn start(&mut self, port: u16) -> Result<()> {
        // Calling swarm to subscribe to all related topics
        for topic in &self.topics {
            // subscribe to each topic, filter out other unrelated topics
            self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
            println!("📡 Subscribed to topic: {}", topic);
        }

        let listen_addr = format!("/ip4/127.0.0.1/tcp/{}", port);
        self.swarm.listen_on(listen_addr.parse()?)?;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await?;
                }

                Some(msg) = self.from_blockchain_receiver.recv() => {
                    self.handle_blockchain_message(&msg).await?;
                }
            }
        }
    }

    // Convert blockchain msg to P2P and broadcast
    async fn handle_blockchain_message(&mut self, msg: &BlockchainMessage) -> Result<()> {
        let serialized = serde_json::to_vec(&msg)?;

        let topic = match &msg {
            BlockchainMessage::NewBlock { .. } => &self.topics[0],
            BlockchainMessage::Attestation { .. } => &self.topics[0],
            BlockchainMessage::NewTransaction { .. } => &self.topics[1],
        };

        // broadcast message to other node, using gossipsub
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), serialized)?;
        println!("📡 Broadcasted message to topic: {}", topic);
        Ok(())
    }

    // 1. convert P2P message received from other node,
    // 2. forward message to blockchain via mpsc channel
    async fn handle_gossipsub_message(&self, data: Vec<u8>) -> Result<()> {
        match serde_json::from_slice::<BlockchainMessage>(&data) {
            Ok(p2p_msg) => {
                // Convert P2P message to NetworkMessage
                let network_msg = match p2p_msg {
                    BlockchainMessage::NewBlock {
                        block,
                        proposer,
                        signature,
                    } => NetworkMessage::NewBlock {
                        block,
                        proposer_id: proposer,
                        signature,
                    },
                    BlockchainMessage::Attestation {
                        block_hash,
                        validator,
                        vote,
                        signature,
                    } => NetworkMessage::Attestation {
                        block_hash,
                        validator_id: validator,
                        vote,
                        signature,
                    },
                    BlockchainMessage::NewTransaction { transaction } => {
                        NetworkMessage::NewTransaction {
                            transaction,
                            from_peer: Address::ZERO, // Simplified for learning
                        }
                    }
                };

                // Forward to blockchain layer
                if let Err(_) = self.to_blockchain_sender.send(network_msg) {
                    println!("❌ Failed to send message to blockchain layer");
                }
            }
            Err(e) => {
                println!("❌ Failed to deserialize P2P message: {}", e);
            }
        }
        Ok(())
    }

    // Pass peer info to message handler
    async fn handle_behaviour_event(&mut self, event: BlockchainBehaviourEvent) -> Result<()> {
        match event {
            BlockchainBehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. }) => {
                self.handle_gossipsub_message(message.data).await?;
            }

            // discover peers
            BlockchainBehaviourEvent::Mdns(mdns::Event::Discovered(peers)) => {
                for (peer_id, addr) in peers {
                    println!("🔍 Discovered peer: {} at {}", peer_id, addr);
                    if let Err(e) = self.swarm.dial(addr) {
                        println!("Failed to dial {}: {}", peer_id, e);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    // handle swarm events
    async fn handle_swarm_event(
        &mut self,
        event: SwarmEvent<BlockchainBehaviourEvent>,
    ) -> Result<()> {
        match event {
            // New connection establish
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                println!("🎧 Listening on: {}, listener id: {}", address, listener_id);
            }
            // Peer connected
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("🤝 Connected to peer: {}", peer_id);
            }
            // Peer disconnected
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                println!("👋 Disconnected from peer: {}", peer_id);
            }
            // Handle protocol-specific events
            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await?;
            }
            // ignore everything else
            _ => {}
        }

        Ok(())
    }
}
