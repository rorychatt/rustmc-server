use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct OpsConfig {
    #[serde(default)]
    operators: Vec<OperatorEntry>,
}

#[derive(Debug, Deserialize)]
struct OperatorEntry {
    uuid: String,
    #[allow(dead_code)]
    name: String,
    level: u8,
}

#[derive(Debug, Clone)]
pub struct Operators {
    levels: HashMap<Uuid, u8>,
}

impl Operators {
    pub fn load() -> Self {
        let path = std::env::var("RUSTMC_OPS")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| Path::new("ops.toml").to_path_buf());

        if !path.exists() {
            info!(
                "No ops config found at {}, no operators configured",
                path.display()
            );
            return Self {
                levels: HashMap::new(),
            };
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => Self::parse(&content),
            Err(e) => {
                warn!("Failed to read {}: {}", path.display(), e);
                Self {
                    levels: HashMap::new(),
                }
            }
        }
    }

    pub fn parse(content: &str) -> Self {
        match toml::from_str::<OpsConfig>(content) {
            Ok(config) => {
                let mut levels = HashMap::new();
                for entry in config.operators {
                    match Uuid::parse_str(&entry.uuid) {
                        Ok(uuid) => {
                            levels.insert(uuid, entry.level);
                        }
                        Err(e) => {
                            warn!("Invalid UUID in ops config: {} - {}", entry.uuid, e);
                        }
                    }
                }
                info!("Loaded {} operator(s)", levels.len());
                Self { levels }
            }
            Err(e) => {
                warn!("Failed to parse ops config: {}", e);
                Self {
                    levels: HashMap::new(),
                }
            }
        }
    }

    pub fn empty() -> Self {
        Self {
            levels: HashMap::new(),
        }
    }

    pub fn get_op_level(&self, uuid: &Uuid) -> u8 {
        self.levels.get(uuid).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operators_empty() {
        let ops = Operators {
            levels: HashMap::new(),
        };
        let uuid = Uuid::new_v4();
        assert_eq!(ops.get_op_level(&uuid), 0);
    }

    #[test]
    fn test_operators_with_entry() {
        let uuid = Uuid::new_v4();
        let mut levels = HashMap::new();
        levels.insert(uuid, 4);
        let ops = Operators { levels };
        assert_eq!(ops.get_op_level(&uuid), 4);
    }

    #[test]
    fn test_ops_config_deserialize() {
        let toml_str = r#"
[[operators]]
uuid = "069a79f4-44e9-4726-a5be-fca90e38aaf5"
name = "Notch"
level = 4
"#;
        let config: OpsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.operators.len(), 1);
        assert_eq!(config.operators[0].level, 4);
    }
}
