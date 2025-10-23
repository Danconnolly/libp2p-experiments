mod config;

use ::futures::StreamExt;
use anyhow::{Context, Result};
use clap::Parser;
use libp2p::{PeerId, Swarm, SwarmBuilder, identity, kad};
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "p2p")]
#[command(about = "libp2p experiments", long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Path to config file (YAML format)
    #[arg(short, long, default_value = "config.yml")]
    config: PathBuf,

    /// Listening port
    #[arg(short, long, default_value = "30333")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    // Load configuration
    let cfg = config::Config::from_file(&cli.config)?;
    tracing::info!("Loaded configuration from: {}", cli.config.display());
    tracing::debug!("Bootstrap peers: {:?}", cfg.bootstrap_peers);
    tracing::debug!("Topic: {}", cfg.topic);

    // Generate a key pair and derive a PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local Peer ID: {:?}", local_peer_id);

    // Create Kademlia DHT for peer discovery
    let kademlia = kad::Behaviour::with_config(
        local_peer_id,
        kad::store::MemoryStore::new(local_peer_id),
        Default::default(),
    );

    // Build the Swarm with Kademlia DHT
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            Default::default(),
            (libp2p_tls::Config::new, libp2p_noise::Config::new),
            libp2p_yamux::Config::default,
        )
        .unwrap()
        .with_dns()
        .unwrap()
        .with_behaviour(|_| kademlia)?
        .build();

    // Listen on configured port
    let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", cli.port);
    Swarm::listen_on(&mut swarm, listen_addr.parse().unwrap())
        .context("Failed to listen on configured port")?;
    tracing::info!("Listening on port {}", cli.port);

    // Connect to bootstrap peers and add them to DHT
    for peer in &cfg.bootstrap_peers {
        match swarm.dial(peer.clone()) {
            Ok(()) => tracing::info!("Dialing bootstrap peer: {}", peer),
            Err(e) => tracing::warn!("Failed to dial bootstrap peer {}: {}", peer, e),
        }

        // Extract peer ID from multiaddr and add to DHT
        if let Some(peer_id) = peer.iter().find_map(|proto| {
            if let libp2p::multiaddr::Protocol::P2p(id) = proto {
                Some(id)
            } else {
                None
            }
        }) {
            swarm.behaviour_mut().add_address(&peer_id, peer.clone());
        }
    }

    // Start DHT bootstrap process
    swarm.behaviour_mut().bootstrap()?;

    // Event loop
    println!("P2P node started. Listening for events...");
    while let Some(event) = swarm.next().await {
        match event {
            libp2p::swarm::SwarmEvent::Behaviour(kad::Event::RoutingUpdated { peer, .. }) => {
                tracing::debug!("Routing updated for peer: {}", peer);
            }
            libp2p::swarm::SwarmEvent::Behaviour(kad::Event::InboundRequest { request }) => {
                tracing::debug!("Inbound DHT request: {:?}", request);
            }
            libp2p::swarm::SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
                connection_id,
            } => {
                println!(
                    "Incoming connection on {}: {} (conn_id: {:?})",
                    local_addr, send_back_addr, connection_id
                );
            }
            libp2p::swarm::SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                println!("Connection established with {} at {:?}", peer_id, endpoint);
            }
            libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                println!("Connection closed with {}", peer_id);
            }
            libp2p::swarm::SwarmEvent::Behaviour(event) => {
                tracing::debug!("DHT event: {:?}", event);
            }
            _ => {
                tracing::debug!("Swarm event: {:?}", event);
            }
        }
    }

    Ok(())
}
