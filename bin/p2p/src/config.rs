use anyhow::{Context, Result};
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Bootstrap peer multiaddresses to connect to
    #[serde(default = "default_bootstrap_peers")]
    pub bootstrap_peers: Vec<Multiaddr>,

    /// Topic to subscribe to
    #[serde(default = "default_topic")]
    pub topic: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bootstrap_peers: default_bootstrap_peers(),
            topic: default_topic(),
        }
    }
}

impl Config {
    /// Load configuration from a YAML file, merging with defaults
    pub fn from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path.display()))?;

        let config: ConfigYaml =
            serde_yaml::from_str(&content).context("Failed to parse YAML config")?;

        Ok(config.into())
    }
}

/// YAML deserializable config (allows partial configuration)
#[derive(Debug, Deserialize, Default)]
struct ConfigYaml {
    #[serde(default)]
    bootstrap_peers: Option<Vec<String>>,

    #[serde(default)]
    topic: Option<String>,
}

impl From<ConfigYaml> for Config {
    fn from(yaml: ConfigYaml) -> Self {
        let bootstrap_peers = yaml
            .bootstrap_peers
            .map(|peers| {
                peers
                    .into_iter()
                    .filter_map(|p| {
                        p.parse::<Multiaddr>()
                            .map_err(|e| {
                                eprintln!("Warning: Failed to parse bootstrap peer '{}': {}", p, e);
                            })
                            .ok()
                    })
                    .collect()
            })
            .unwrap_or_else(default_bootstrap_peers);

        let topic = yaml.topic.unwrap_or_else(default_topic);

        Config {
            bootstrap_peers,
            topic,
        }
    }
}

/// Default bootstrap peers
fn default_bootstrap_peers() -> Vec<Multiaddr> {
    vec![
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN"
            .parse()
            .expect("invalid bootstrap peer address"),
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa"
            .parse()
            .expect("invalid bootstrap peer address"),
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb"
            .parse()
            .expect("invalid bootstrap peer address"),
        "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt"
            .parse()
            .expect("invalid bootstrap peer address"),
        "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ"
            .parse()
            .expect("invalid bootstrap peer address"),
        "/ip4/104.131.131.82/udp/4001/quic/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ"
            .parse()
            .expect("invalid bootstrap peer address"),
    ]
}

/// Default topic to subscribe to
fn default_topic() -> String {
    "example-topic".to_string()
}
