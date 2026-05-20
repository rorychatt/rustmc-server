use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

const BLOCK_STATES_JSON: &str = include_str!("../../data/block_states.json");

static REGISTRY: OnceLock<BlockRegistry> = OnceLock::new();

#[derive(Deserialize)]
struct RawData {
    blocks: HashMap<String, RawBlock>,
}

#[derive(Deserialize)]
struct RawBlock {
    default_state_id: u16,
    properties: HashMap<String, Vec<String>>,
    states: Vec<RawState>,
}

#[derive(Deserialize)]
struct RawState {
    id: u16,
    properties: HashMap<String, String>,
}

pub struct BlockRegistry {
    blocks: HashMap<String, BlockEntry>,
    state_to_block: HashMap<u16, (String, HashMap<String, String>)>,
}

pub struct BlockEntry {
    pub default_state_id: u16,
    pub properties: HashMap<String, Vec<String>>,
    pub states: Vec<StateEntry>,
}

pub struct StateEntry {
    pub id: u16,
    pub properties: HashMap<String, String>,
}

impl BlockRegistry {
    pub fn global() -> &'static BlockRegistry {
        REGISTRY.get_or_init(Self::load)
    }

    fn load() -> Self {
        let raw: RawData = serde_json::from_str(BLOCK_STATES_JSON)
            .expect("block_states.json should be valid");

        let mut blocks = HashMap::with_capacity(raw.blocks.len());
        let mut state_to_block = HashMap::new();

        for (name, raw_block) in raw.blocks {
            for raw_state in &raw_block.states {
                state_to_block.insert(
                    raw_state.id,
                    (name.clone(), raw_state.properties.clone()),
                );
            }

            let states = raw_block
                .states
                .into_iter()
                .map(|s| StateEntry {
                    id: s.id,
                    properties: s.properties,
                })
                .collect();

            blocks.insert(
                name,
                BlockEntry {
                    default_state_id: raw_block.default_state_id,
                    properties: raw_block.properties,
                    states,
                },
            );
        }

        Self {
            blocks,
            state_to_block,
        }
    }

    pub fn get_state_id(&self, block: &str, properties: &[(&str, &str)]) -> Option<u16> {
        let entry = self.blocks.get(block)?;
        entry.states.iter().find_map(|state| {
            let matches = properties.iter().all(|(k, v)| {
                state.properties.get(*k).is_some_and(|sv| sv == v)
            });
            if matches && state.properties.len() == properties.len() {
                Some(state.id)
            } else {
                None
            }
        })
    }

    pub fn get_default_state_id(&self, block: &str) -> Option<u16> {
        self.blocks.get(block).map(|e| e.default_state_id)
    }

    pub fn get_block_from_state(&self, state_id: u16) -> Option<(&str, &HashMap<String, String>)> {
        self.state_to_block
            .get(&state_id)
            .map(|(name, props)| (name.as_str(), props))
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn get_block_entry(&self, block: &str) -> Option<&BlockEntry> {
        self.blocks.get(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_loads() {
        let registry = BlockRegistry::global();
        assert!(registry.block_count() > 0);
    }

    #[test]
    fn test_lookup_default_state() {
        let registry = BlockRegistry::global();
        let id = registry
            .get_state_id("minecraft:oak_log", &[("axis", "y")])
            .unwrap();
        let expected = registry.get_default_state_id("minecraft:oak_log").unwrap();
        assert_eq!(id, expected);
    }

    #[test]
    fn test_lookup_non_default() {
        let registry = BlockRegistry::global();
        let default_id = registry.get_default_state_id("minecraft:oak_log").unwrap();
        let x_id = registry
            .get_state_id("minecraft:oak_log", &[("axis", "x")])
            .unwrap();
        assert_ne!(x_id, default_id);
    }

    #[test]
    fn test_reverse_lookup() {
        let registry = BlockRegistry::global();
        let default_id = registry.get_default_state_id("minecraft:oak_log").unwrap();
        let (name, props) = registry.get_block_from_state(default_id).unwrap();
        assert_eq!(name, "minecraft:oak_log");
        assert_eq!(props.get("axis").unwrap(), "y");
    }

    #[test]
    fn test_block_without_properties() {
        let registry = BlockRegistry::global();
        let entry = registry.get_block_entry("minecraft:stone").unwrap();
        assert!(entry.properties.is_empty());
        assert_eq!(entry.states.len(), 1);
        assert_eq!(entry.states[0].id, entry.default_state_id);
    }
}
