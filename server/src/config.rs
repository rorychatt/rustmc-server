use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
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
    name: String,
    level: u8,
}

#[derive(Debug, Clone)]
pub struct OperatorData {
    pub name: String,
    pub level: u8,
}

#[derive(Debug, Clone)]
pub struct Operators {
    entries: HashMap<Uuid, OperatorData>,
    last_modified: Option<SystemTime>,
}

impl Operators {
    pub fn ops_path() -> PathBuf {
        std::env::var("RUSTMC_OPS")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Path::new("ops.toml").to_path_buf())
    }

    pub fn load() -> Self {
        let path = Self::ops_path();

        if !path.exists() {
            info!(
                "No ops config found at {}, no operators configured",
                path.display()
            );
            return Self {
                entries: HashMap::new(),
                last_modified: None,
            };
        }

        let last_modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let mut ops = Self::parse(&content);
                ops.last_modified = last_modified;
                ops
            }
            Err(e) => {
                warn!("Failed to read {}: {}", path.display(), e);
                Self {
                    entries: HashMap::new(),
                    last_modified: None,
                }
            }
        }
    }

    pub fn parse(content: &str) -> Self {
        match toml::from_str::<OpsConfig>(content) {
            Ok(config) => {
                let mut entries = HashMap::new();
                for entry in config.operators {
                    match Uuid::parse_str(&entry.uuid) {
                        Ok(uuid) => {
                            entries.insert(
                                uuid,
                                OperatorData {
                                    name: entry.name,
                                    level: entry.level,
                                },
                            );
                        }
                        Err(e) => {
                            warn!("Invalid UUID in ops config: {} - {}", entry.uuid, e);
                        }
                    }
                }
                info!("Loaded {} operator(s)", entries.len());
                Self {
                    entries,
                    last_modified: None,
                }
            }
            Err(e) => {
                warn!("Failed to parse ops config: {}", e);
                Self {
                    entries: HashMap::new(),
                    last_modified: None,
                }
            }
        }
    }

    pub fn empty() -> Self {
        Self {
            entries: HashMap::new(),
            last_modified: None,
        }
    }

    pub fn find_uuid_by_name(&self, name: &str) -> Option<Uuid> {
        self.entries
            .iter()
            .find(|(_, data)| data.name.eq_ignore_ascii_case(name))
            .map(|(uuid, _)| *uuid)
    }

    pub fn get_op_level(&self, uuid: &Uuid) -> u8 {
        self.entries.get(uuid).map(|d| d.level).unwrap_or(0)
    }

    pub fn set_op_level(&mut self, uuid: Uuid, name: String, level: u8) {
        self.entries.insert(uuid, OperatorData { name, level });
    }

    pub fn remove_op(&mut self, uuid: &Uuid) {
        self.entries.remove(uuid);
    }

    pub fn serialize_to_toml(&self) -> String {
        let mut output = String::new();
        for (uuid, data) in &self.entries {
            output.push_str("[[operators]]\n");
            output.push_str(&format!("uuid = \"{}\"\n", uuid));
            output.push_str(&format!("name = \"{}\"\n", data.name));
            output.push_str(&format!("level = {}\n\n", data.level));
        }
        output
    }

    pub fn save(&self) {
        let path = Self::ops_path();
        let content = self.serialize_to_toml();
        if let Err(e) = std::fs::write(&path, &content) {
            warn!("Failed to write ops config to {}: {}", path.display(), e);
        }
    }

    pub fn has_file_changed(&self) -> bool {
        let path = Self::ops_path();
        if !path.exists() {
            return self.last_modified.is_some();
        }
        let current_mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        current_mtime != self.last_modified
    }

    pub fn reload(&mut self) {
        let path = Self::ops_path();
        let last_modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let reloaded = Self::parse(&content);
                self.entries = reloaded.entries;
                self.last_modified = last_modified;
                info!("Reloaded ops config: {} operator(s)", self.entries.len());
            }
            Err(e) => {
                warn!("Failed to reload ops config: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_operators_empty() {
        let ops = Operators::empty();
        let uuid = Uuid::new_v4();
        assert_eq!(ops.get_op_level(&uuid), 0);
    }

    #[test]
    fn test_operators_with_entry() {
        let uuid = Uuid::new_v4();
        let mut ops = Operators::empty();
        ops.set_op_level(uuid, "TestPlayer".to_string(), 4);
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

    #[test]
    fn test_set_and_remove_op() {
        let mut ops = Operators::empty();
        let uuid = Uuid::new_v4();
        ops.set_op_level(uuid, "Player1".to_string(), 3);
        assert_eq!(ops.get_op_level(&uuid), 3);
        ops.remove_op(&uuid);
        assert_eq!(ops.get_op_level(&uuid), 0);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let mut ops = Operators::empty();
        let uuid = Uuid::parse_str("069a79f4-44e9-4726-a5be-fca90e38aaf5").unwrap();
        ops.set_op_level(uuid, "Notch".to_string(), 4);
        let toml_output = ops.serialize_to_toml();
        let parsed = Operators::parse(&toml_output);
        assert_eq!(parsed.get_op_level(&uuid), 4);
    }

    #[test]
    fn test_find_uuid_by_name() {
        let mut ops = Operators::empty();
        let uuid = Uuid::parse_str("069a79f4-44e9-4726-a5be-fca90e38aaf5").unwrap();
        ops.set_op_level(uuid, "Notch".to_string(), 4);

        assert_eq!(ops.find_uuid_by_name("Notch"), Some(uuid));
        assert_eq!(ops.find_uuid_by_name("notch"), Some(uuid));
        assert_eq!(ops.find_uuid_by_name("NOTCH"), Some(uuid));
        assert_eq!(ops.find_uuid_by_name("Unknown"), None);
    }

    #[test]
    #[serial]
    fn test_reload_picks_up_changes() {
        let dir = std::env::temp_dir().join("rustmc_ops_test_reload");
        std::fs::create_dir_all(&dir).unwrap();
        let ops_path = dir.join("ops.toml");

        let initial = r#"[[operators]]
uuid = "069a79f4-44e9-4726-a5be-fca90e38aaf5"
name = "Notch"
level = 4
"#;
        std::fs::write(&ops_path, initial).unwrap();

        std::env::set_var("RUSTMC_OPS", ops_path.to_str().unwrap());
        let mut ops = Operators::load();
        let uuid = Uuid::parse_str("069a79f4-44e9-4726-a5be-fca90e38aaf5").unwrap();
        assert_eq!(ops.get_op_level(&uuid), 4);

        let updated = r#"[[operators]]
uuid = "069a79f4-44e9-4726-a5be-fca90e38aaf5"
name = "Notch"
level = 2
"#;
        std::fs::write(&ops_path, updated).unwrap();
        ops.reload();
        assert_eq!(ops.get_op_level(&uuid), 2);

        std::env::remove_var("RUSTMC_OPS");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
