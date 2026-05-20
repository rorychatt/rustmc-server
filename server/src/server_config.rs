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
    pub gameplay: GameplaySection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_view_distance")]
    pub view_distance: i32,
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

fn default_bind() -> String {
    "0.0.0.0:25565".to_string()
}

fn default_view_distance() -> i32 {
    8
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
            view_distance: default_view_distance(),
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
    pub fn load() -> Self {
        let path = if let Ok(env_path) = std::env::var("RUSTMC_CONFIG") {
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
        };

        if !path.exists() {
            info!(
                "No config file at {}, using defaults",
                path.display()
            );
            return Self::default();
        }

        let is_yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
            .unwrap_or(false);

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                if is_yaml {
                    match serde_yaml::from_str(&content) {
                        Ok(config) => {
                            info!("Loaded config from {}", path.display());
                            config
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse YAML {}: {}, using defaults", path.display(), e);
                            Self::default()
                        }
                    }
                } else {
                    match toml::from_str(&content) {
                        Ok(config) => {
                            info!("Loaded config from {}", path.display());
                            config
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse TOML {}: {}, using defaults", path.display(), e);
                            Self::default()
                        }
                    }
                }
            }
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
        assert_eq!(config.server.view_distance, 8);
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
        std::env::set_var("RUSTMC_CONFIG", "/nonexistent/path/server.toml");
        let config = ServerConfig::load();
        assert_eq!(config.rate_limit.invalid_packet_threshold, 16);
        assert_eq!(config.rate_limit.invalid_packet_window_secs, 10);
        std::env::remove_var("RUSTMC_CONFIG");
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
}
