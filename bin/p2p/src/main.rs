use anyhow::{Context, Result};
use clap::Parser;
use libp2p::{PeerId, Swarm, Multiaddr, identity, floodsub::{Floodsub, FloodsubEvent, Topic}, SwarmBuilder, floodsub};
use futures::StreamExt;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;


#[derive(Parser)]
#[command(name = "p2p")]
#[command(about = "libp2p experiments", long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
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

    // Generate a key pair and derive a PeerId
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local Peer ID: {:?}", local_peer_id);

    // Create a Floodsub instance for pub/sub messaging
    let mut floodsub = floodsub::Behaviour::new(local_peer_id.clone());
    let topic = Topic::new("example-topic");
    floodsub.subscribe(topic.clone());

    // Build the Swarm
    let key = local_key.clone();
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(Default::default(),(libp2p_tls::Config::new, libp2p_noise::Config::new), libp2p_yamux::Config::default,).unwrap()
        .with_behaviour(|_key| floodsub).unwrap()
        .build();
    // Listen on a random local port
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();
    // Event loop
    while let Some(event) = swarm.next().await {
        println!("swarm event: {:?}", event);
    }
    Ok(())
}