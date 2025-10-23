mod config;

use ::futures::StreamExt;
use anyhow::{Context, Result};
use clap::Parser;
use libp2p::{PeerId, Swarm, SwarmBuilder, identity, kad};
use std::path::{Path, PathBuf};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "p2p")]
#[command(about = "libp2p experiments", long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Data directory for config and identity files
    #[arg(short, long, default_value = ".")]
    data_dir: PathBuf,

    /// Path to config file (relative to data directory)
    #[arg(short, long, default_value = "config.yml")]
    config: PathBuf,

    /// Path to identity file (relative to data directory)
    #[arg(short, long, default_value = "identity")]
    identity_file: PathBuf,

    /// Listening port
    #[arg(short, long, default_value = "30333")]
    port: u16,
}

/// Load or create a keypair from the identity file
fn load_or_create_identity(identity_path: &Path) -> Result<identity::Keypair> {
    if identity_path.exists() {
        let key_bytes = std::fs::read(identity_path).context("Failed to read identity file")?;
        let key_data = String::from_utf8(key_bytes).context("Identity file is not valid UTF-8")?;
        let decoded = hex::decode(&key_data).context("Failed to decode identity from hex")?;
        identity::Keypair::ed25519_from_bytes(decoded)
            .context("Failed to parse identity keypair from bytes")
    } else {
        // Create new keypair
        let keypair = identity::Keypair::generate_ed25519();
        let key_bytes = keypair
            .to_protobuf_encoding()
            .context("Failed to encode identity to protobuf")?;
        let hex_encoded = hex::encode(key_bytes);
        std::fs::write(identity_path, &hex_encoded).context("Failed to write identity file")?;
        println!("Created new identity at: {}", identity_path.display());
        Ok(keypair)
    }
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

    // Construct full paths using data directory
    let config_path = cli.data_dir.join(&cli.config);
    let identity_path = cli.data_dir.join(&cli.identity_file);

    tracing::info!("Data directory: {}", cli.data_dir.display());

    // Load or create identity
    let local_key = load_or_create_identity(&identity_path)?;
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local Peer ID: {}", local_peer_id);

    // Load configuration
    let cfg = config::Config::from_file(&config_path)?;
    tracing::info!("Loaded configuration from: {}", config_path.display());
    tracing::debug!("Bootstrap peers: {:?}", cfg.bootstrap_peers);
    tracing::debug!("Topic: {}", cfg.topic);

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
        .with_quic()
        .with_dns()
        .unwrap()
        .with_behaviour(|_| kademlia)?
        .build();

    // Listen on TCP port
    let tcp_listen_addr = format!("/ip4/0.0.0.0/tcp/{}", cli.port);
    Swarm::listen_on(&mut swarm, tcp_listen_addr.parse().unwrap())
        .context("Failed to listen on TCP port")?;
    tracing::info!("Listening on TCP port {}", cli.port);

    // Listen on QUIC port (UDP, port + 1 for distinction)
    let quic_listen_addr = format!("/ip4/0.0.0.0/udp/{}/quic-v1", cli.port + 1);
    Swarm::listen_on(&mut swarm, quic_listen_addr.parse().unwrap())
        .context("Failed to listen on QUIC port")?;
    tracing::info!("Listening on QUIC port {}", cli.port + 1);

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
                println!("DHT: Routing updated for peer: {}", peer);
            }
            libp2p::swarm::SwarmEvent::Behaviour(kad::Event::InboundRequest { request }) => {
                println!("DHT: Inbound request: {:?}", request);
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
                println!("DHT event: {:?}", event);
            }
            _ => {
                println!("Swarm event: {:?}", event);
            }
        }
    }

    Ok(())
}
