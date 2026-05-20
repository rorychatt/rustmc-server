use serde::Deserialize;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub rate_limit: RateLimitSection,
    #[serde(default)]
    pub network: NetworkSection,
    #[serde(default)]
    pub transfer: TransferSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_plugins_directory")]
    pub plugins_directory: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkSection {
    #[serde(default = "default_non_play_timeout_secs")]
    pub non_play_timeout_secs: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TransferSection {
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitSection {
    #[serde(default = "default_invalid_packet_threshold")]
    pub invalid_packet_threshold: u32,
    #[serde(default = "default_invalid_packet_window_secs")]
    pub invalid_packet_window_secs: u64,
}

fn default_bind() -> String {
    "0.0.0.0:25565".to_string()
}

fn default_plugins_directory() -> String {
    "plugins".to_string()
}

fn default_non_play_timeout_secs() -> u64 {
    30
}

fn default_invalid_packet_threshold() -> u32 {
    16
}

fn default_invalid_packet_window_secs() -> u64 {
    10
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            plugins_directory: default_plugins_directory(),
        }
    }
}

impl Default for NetworkSection {
    fn default() -> Self {
        Self {
            non_play_timeout_secs: default_non_play_timeout_secs(),
        }
    }
}

impl Default for RateLimitSection {
    fn default() -> Self {
        Self {
            invalid_packet_threshold: default_invalid_packet_threshold(),
            invalid_packet_window_secs: default_invalid_packet_window_secs(),
        }
    }
}

impl ServerConfig {
    pub fn load() -> Self {
        let path = std::env::var("RUSTMC_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("server.toml"));

        if !path.exists() {
            info!(
                "No config file at {}, using defaults",
                path.display()
            );
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => {
                    info!("Loaded config from {}", path.display());
                    config
                }
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}, using defaults", path.display(), e);
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read {}: {}, using defaults", path.display(), e);
                Self::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.rate_limit.invalid_packet_threshold, 16);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 10);
        assert_eq!(config.server.bind, "0.0.0.0:25565");
        assert_eq!(config.server.plugins_directory, "plugins");
        assert_eq!(config.network.non_play_timeout_secs, 30);
        assert!(config.transfer.secret.is_none());
    }

    #[test]
    fn test_load_from_toml() {
        let toml_str = r#"
[server]
bind = "127.0.0.1:25566"

[rate_limit]
invalid_packet_threshold = 32
invalid_packet_window_secs = 20
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.bind, "127.0.0.1:25566");
        assert_eq!(config.rate_limit.invalid_packet_threshold, 32);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 20);
    }

    #[test]
    fn test_partial_toml() {
        let toml_str = r#"
[server]
bind = "0.0.0.0:25567"
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.bind, "0.0.0.0:25567");
        assert_eq!(config.rate_limit.invalid_packet_threshold, 16);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 10);
    }

    #[test]
    fn test_missing_file_uses_defaults() {
        std::env::set_var("RUSTMC_CONFIG", "/nonexistent/path/server.toml");
        let config = ServerConfig::load();
        assert_eq!(config.rate_limit.invalid_packet_threshold, 16);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 10);
        std::env::remove_var("RUSTMC_CONFIG");
    }

    #[test]
    fn test_network_section_defaults() {
        let toml_str = "";
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.network.non_play_timeout_secs, 30);
    }

    #[test]
    fn test_transfer_section_defaults() {
        let toml_str = "";
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert!(config.transfer.secret.is_none());
    }

    #[test]
    fn test_plugins_directory_default() {
        let toml_str = "";
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.plugins_directory, "plugins");
    }

    #[test]
    fn test_full_config_with_new_sections() {
        let toml_str = r#"
[server]
bind = "127.0.0.1:25566"
plugins_directory = "my_plugins"

[network]
non_play_timeout_secs = 60

[transfer]
secret = "my-secret-key"

[rate_limit]
invalid_packet_threshold = 32
invalid_packet_window_secs = 20
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.bind, "127.0.0.1:25566");
        assert_eq!(config.server.plugins_directory, "my_plugins");
        assert_eq!(config.network.non_play_timeout_secs, 60);
        assert_eq!(config.transfer.secret, Some("my-secret-key".to_string()));
        assert_eq!(config.rate_limit.invalid_packet_threshold, 32);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 20);
    }
}
