mod config;

use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use libp2p::{PeerId, Swarm, SwarmBuilder, floodsub, floodsub::Topic, identity};
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

    // Create a Floodsub instance for pub/sub messaging
    let mut floodsub = floodsub::Behaviour::new(local_peer_id);
    let topic = Topic::new(cfg.topic.clone());
    floodsub.subscribe(topic.clone());

    // Build the Swarm with DNS and WebSocket support
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
        .with_behaviour(|_| floodsub)?
        .build();

    // Listen on a random local port
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

    // Connect to bootstrap peers
    for peer in &cfg.bootstrap_peers {
        match swarm.dial(peer.clone()) {
            Ok(()) => tracing::info!("Dialing bootstrap peer: {}", peer),
            Err(e) => tracing::warn!("Failed to dial bootstrap peer {}: {}", peer, e),
        }
    }

    // Event loop
    println!("P2P node started. Listening for events...");
    while let Some(event) = swarm.next().await {
        match event {
            libp2p::swarm::SwarmEvent::Behaviour(floodsub::Event::Message(msg)) => {
                let message_text = String::from_utf8_lossy(&msg.data);
                println!("Message from {}: {}", msg.source, message_text);
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
            _ => {
                println!("Swarm event: {:?}", event);
            }
        }
    }

    Ok(())
}
