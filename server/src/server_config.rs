use serde::Deserialize;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{info, warn};

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub rate_limit: RateLimitSection,
    #[serde(default)]
    pub network: NetworkSection,
    #[serde(default)]
    pub transfer: TransferSection,
    #[serde(default)]
    pub gameplay: GameplaySection,
    #[serde(default)]
    pub cache: CacheSection,
    #[serde(skip)]
    pub last_modified: Option<SystemTime>,
    #[serde(skip)]
    pub config_path: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSection::default(),
            rate_limit: RateLimitSection::default(),
            network: NetworkSection::default(),
            transfer: TransferSection::default(),
            gameplay: GameplaySection::default(),
            cache: CacheSection::default(),
            last_modified: None,
            config_path: Self::default_path(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_plugins_directory")]
    pub plugins_directory: String,
    #[serde(default = "default_view_distance")]
    pub view_distance: i32,
    #[serde(default)]
    pub port_file: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct GameplaySection {
    #[serde(default = "default_motd")]
    pub motd: String,
    #[serde(default = "default_max_players")]
    pub max_players: i32,
    #[serde(default = "default_gamemode")]
    pub gamemode: String,
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
    #[serde(default = "default_pvp")]
    pub pvp: bool,
    #[serde(default = "default_allow_flight")]
    pub allow_flight: bool,
    #[serde(default = "default_hardcore")]
    pub hardcore: bool,
    #[serde(default = "default_simulation_distance")]
    pub simulation_distance: i32,
    #[serde(default = "default_sea_level")]
    pub sea_level: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheSection {
    #[serde(default = "default_uuid_cache_max_entries")]
    pub uuid_cache_max_entries: usize,
}

fn default_uuid_cache_max_entries() -> usize {
    256
}

impl Default for CacheSection {
    fn default() -> Self {
        Self {
            uuid_cache_max_entries: default_uuid_cache_max_entries(),
        }
    }
}
fn default_bind() -> String {
    "0.0.0.0:25565".to_string()
}

fn default_plugins_directory() -> String {
    "plugins".to_string()
}

fn default_view_distance() -> i32 {
    8
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

fn default_motd() -> String {
    "RustMC Server - A Rust-powered Minecraft server".to_string()
}

fn default_max_players() -> i32 {
    20
}

fn default_gamemode() -> String {
    "creative".to_string()
}

fn default_difficulty() -> String {
    "normal".to_string()
}

fn default_pvp() -> bool {
    true
}

fn default_allow_flight() -> bool {
    false
}

fn default_hardcore() -> bool {
    false
}

fn default_simulation_distance() -> i32 {
    8
}

fn default_sea_level() -> i32 {
    63
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            plugins_directory: default_plugins_directory(),
            view_distance: default_view_distance(),
            port_file: None,
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

impl Default for GameplaySection {
    fn default() -> Self {
        Self {
            motd: default_motd(),
            max_players: default_max_players(),
            gamemode: default_gamemode(),
            difficulty: default_difficulty(),
            pvp: default_pvp(),
            allow_flight: default_allow_flight(),
            hardcore: default_hardcore(),
            simulation_distance: default_simulation_distance(),
            sea_level: default_sea_level(),
        }
    }
}

impl GameplaySection {
    pub fn gamemode_id(&self) -> i32 {
        match self.gamemode.to_lowercase().as_str() {
            "survival" => 0,
            "creative" => 1,
            "adventure" => 2,
            "spectator" => 3,
            _ => 1, // Default to creative
        }
    }

    pub fn difficulty_id(&self) -> i32 {
        match self.difficulty.to_lowercase().as_str() {
            "peaceful" => 0,
            "easy" => 1,
            "normal" => 2,
            "hard" => 3,
            _ => 2, // Default to normal
        }
    }
}

impl ServerConfig {
    pub fn port_file(&self) -> Option<String> {
        self.server
            .port_file
            .clone()
            .or_else(|| std::env::var("RUSTMC_PORT_FILE").ok())
    }

    fn default_path() -> PathBuf {
        if let Ok(env_path) = std::env::var("RUSTMC_CONFIG") {
            PathBuf::from(env_path)
        } else {
            let yaml_path = PathBuf::from("server.yaml");
            let yml_path = PathBuf::from("server.yml");
            let toml_path = PathBuf::from("server.toml");

            if yaml_path.exists() {
                yaml_path
            } else if yml_path.exists() {
                yml_path
            } else {
                toml_path
            }
        }
    }

    fn is_yaml_path(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
            .unwrap_or(false)
    }

    pub fn load() -> Self {
        let path = Self::default_path();
        if !path.exists() {
            info!("No config file at {}, using defaults", path.display());
            return Self {
                config_path: path,
                ..Self::default()
            };
        }

        let last_modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        let is_yaml = Self::is_yaml_path(&path);

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let parse_result: Result<ServerConfig, String> = if is_yaml {
                    serde_yaml::from_str(&content).map_err(|e| format!("YAML: {}", e))
                } else {
                    toml::from_str(&content).map_err(|e| format!("TOML: {}", e))
                };
                match parse_result {
                    Ok(mut config) => {
                        config.last_modified = last_modified;
                        config.config_path = path.clone();
                        info!("Loaded config from {}", path.display());
                        config
                    }
                    Err(e) => {
                        warn!("Failed to parse {}: {}, using defaults", path.display(), e);
                        Self {
                            config_path: path,
                            ..Self::default()
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read {}: {}, using defaults", path.display(), e);
                Self {
                    config_path: path,
                    ..Self::default()
                }
            }
        }
    }

    pub fn has_file_changed(&self) -> bool {
        if !self.config_path.exists() {
            return self.last_modified.is_some();
        }
        let current_mtime = std::fs::metadata(&self.config_path)
            .ok()
            .and_then(|m| m.modified().ok());
        current_mtime != self.last_modified
    }

    pub fn reload(&mut self) {
        let last_modified = std::fs::metadata(&self.config_path)
            .ok()
            .and_then(|m| m.modified().ok());

        let is_yaml = Self::is_yaml_path(&self.config_path);

        match std::fs::read_to_string(&self.config_path) {
            Ok(content) => {
                let parse_result: Result<ServerConfig, String> = if is_yaml {
                    serde_yaml::from_str(&content).map_err(|e| format!("YAML: {}", e))
                } else {
                    toml::from_str(&content).map_err(|e| format!("TOML: {}", e))
                };
                match parse_result {
                    Ok(new_config) => {
                        let old_bind = self.server.bind.clone();
                        self.server = new_config.server;
                        self.rate_limit = new_config.rate_limit;
                        self.network = new_config.network;
                        self.transfer = new_config.transfer;
                        self.gameplay = new_config.gameplay;
                        self.cache = new_config.cache;
                        self.last_modified = last_modified;
                        if self.server.bind != old_bind {
                            warn!(
                                "server.bind changed to '{}' — restart required for this to take effect",
                                self.server.bind
                            );
                        }
                        info!("Reloaded server config");
                    }
                    Err(e) => {
                        warn!("Failed to parse config during reload: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read config during reload: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.rate_limit.invalid_packet_threshold, 16);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 10);
        assert_eq!(config.server.bind, "0.0.0.0:25565");
        assert_eq!(config.server.view_distance, 8);
        assert_eq!(config.server.plugins_directory, "plugins");
        assert_eq!(config.network.non_play_timeout_secs, 30);
        assert!(config.transfer.secret.is_none());
        assert!(config.last_modified.is_none());
    }

    #[test]
    fn test_load_from_toml() {
        let toml_str = r#"
[server]
bind = "127.0.0.1:25566"
view_distance = 16

[rate_limit]
invalid_packet_threshold = 32
invalid_packet_window_secs = 20
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.bind, "127.0.0.1:25566");
        assert_eq!(config.server.view_distance, 16);
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
        assert_eq!(config.server.view_distance, 8);
        assert_eq!(config.rate_limit.invalid_packet_threshold, 16);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 10);
    }

    #[test]
    fn test_missing_file_uses_defaults() {
        let config = ServerConfig {
            config_path: PathBuf::from("/nonexistent/path/server.toml"),
            ..ServerConfig::default()
        };
        assert_eq!(config.rate_limit.invalid_packet_threshold, 16);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 10);
    }

    #[test]
    fn test_has_file_changed_false_when_unchanged() {
        let dir = std::env::temp_dir().join("rustmc_config_test_unchanged");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("server.toml");

        let content = r#"
[rate_limit]
invalid_packet_threshold = 8
"#;
        std::fs::write(&config_path, content).unwrap();

        std::env::set_var("RUSTMC_CONFIG", config_path.to_str().unwrap());
        let mut config = ServerConfig::load();
        config.config_path = config_path.clone();
        assert!(!config.has_file_changed());

        std::env::remove_var("RUSTMC_CONFIG");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_has_file_changed_true_after_write() {
        let dir = std::env::temp_dir().join("rustmc_config_test_changed");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("server.toml");

        let content = r#"
[rate_limit]
invalid_packet_threshold = 8
"#;
        std::fs::write(&config_path, content).unwrap();

        std::env::set_var("RUSTMC_CONFIG", config_path.to_str().unwrap());
        let mut config = ServerConfig::load();
        config.config_path = config_path.clone();

        thread::sleep(Duration::from_millis(50));
        std::fs::write(
            &config_path,
            "[rate_limit]\ninvalid_packet_threshold = 99\n",
        )
        .unwrap();

        assert!(config.has_file_changed());

        std::env::remove_var("RUSTMC_CONFIG");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_reload_picks_up_new_values() {
        let dir = std::env::temp_dir().join("rustmc_config_test_reload");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("server.toml");

        let initial = r#"
[rate_limit]
invalid_packet_threshold = 8
invalid_packet_window_secs = 5
"#;
        std::fs::write(&config_path, initial).unwrap();

        let last_modified = std::fs::metadata(&config_path)
            .ok()
            .and_then(|m| m.modified().ok());

        let mut config = ServerConfig {
            rate_limit: toml::from_str::<ServerConfig>(initial).unwrap().rate_limit,
            config_path: config_path.clone(),
            last_modified,
            ..ServerConfig::default()
        };
        assert_eq!(config.rate_limit.invalid_packet_threshold, 8);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 5);

        let updated = r#"
[rate_limit]
invalid_packet_threshold = 64
invalid_packet_window_secs = 30
"#;
        std::fs::write(&config_path, updated).unwrap();
        config.reload();

        assert_eq!(config.rate_limit.invalid_packet_threshold, 64);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 30);

        std::fs::remove_dir_all(&dir).unwrap();
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

    #[test]
    fn test_load_from_yaml() {
        let yaml_str = r#"
server:
  bind: "127.0.0.1:25569"
  view_distance: 12
rate_limit:
  invalid_packet_threshold: 40
  invalid_packet_window_secs: 15
gameplay:
  motd: "Custom MOTD"
  max_players: 50
  gamemode: "survival"
  difficulty: "hard"
  pvp: false
  allow_flight: true
  hardcore: true
  simulation_distance: 6
  sea_level: 60
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml_str).unwrap();
        assert_eq!(config.server.bind, "127.0.0.1:25569");
        assert_eq!(config.server.view_distance, 12);
        assert_eq!(config.rate_limit.invalid_packet_threshold, 40);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 15);
        assert_eq!(config.gameplay.motd, "Custom MOTD");
        assert_eq!(config.gameplay.max_players, 50);
        assert_eq!(config.gameplay.gamemode, "survival");
        assert_eq!(config.gameplay.gamemode_id(), 0);
        assert_eq!(config.gameplay.difficulty, "hard");
        assert_eq!(config.gameplay.difficulty_id(), 3);
        assert!(!config.gameplay.pvp);
        assert!(config.gameplay.allow_flight);
        assert!(config.gameplay.hardcore);
        assert_eq!(config.gameplay.simulation_distance, 6);
        assert_eq!(config.gameplay.sea_level, 60);
    }

    #[test]
    fn test_port_file_returns_none_when_unset() {
        std::env::remove_var("RUSTMC_PORT_FILE");
        let config = ServerConfig::default();
        assert_eq!(config.port_file(), None);
    }

    #[test]
    fn test_port_file_returns_config_value() {
        std::env::remove_var("RUSTMC_PORT_FILE");
        let mut config = ServerConfig::default();
        config.server.port_file = Some("/tmp/port".to_string());
        assert_eq!(config.port_file(), Some("/tmp/port".to_string()));
    }

    #[test]
    fn test_port_file_falls_back_to_env_var() {
        let config = ServerConfig::default();
        std::env::set_var("RUSTMC_PORT_FILE", "/tmp/env_port");
        assert_eq!(config.port_file(), Some("/tmp/env_port".to_string()));
        std::env::remove_var("RUSTMC_PORT_FILE");
    }

    #[test]
    fn test_port_file_config_takes_precedence_over_env() {
        std::env::set_var("RUSTMC_PORT_FILE", "/tmp/env_port");
        let mut config = ServerConfig::default();
        config.server.port_file = Some("/tmp/config_port".to_string());
        assert_eq!(config.port_file(), Some("/tmp/config_port".to_string()));
        std::env::remove_var("RUSTMC_PORT_FILE");
    }

    #[test]
    fn test_port_file_from_toml() {
        let toml_str = r#"
[server]
port_file = "/tmp/test_port"
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.port_file, Some("/tmp/test_port".to_string()));
    }
}
