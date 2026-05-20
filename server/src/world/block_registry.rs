use std::collections::HashMap;

#[allow(clippy::type_complexity)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/block_states_generated.rs"));
}

pub use generated::{BlockDef, StateDef, BLOCKS, BLOCK_COUNT, STATES};

pub struct BlockEntry {
    pub default_state_id: u16,
    pub properties: HashMap<String, Vec<String>>,
    pub states: Vec<StateEntry>,
}

pub struct StateEntry {
    pub id: u16,
    pub properties: HashMap<String, String>,
}

pub struct BlockRegistry;

impl BlockRegistry {
    pub fn global() -> &'static BlockRegistry {
        static INSTANCE: BlockRegistry = BlockRegistry;
        &INSTANCE
    }

    pub fn get_state_id(&self, block: &str, properties: &[(&str, &str)]) -> Option<u16> {
        let block_def = BLOCKS.get(block)?;
        block_def.states.iter().find_map(|state| {
            if state.properties.len() != properties.len() {
                return None;
            }
            let matches = properties.iter().all(|(k, v)| {
                state.properties.iter().any(|(sk, sv)| sk == k && sv == v)
            });
            if matches { Some(state.id) } else { None }
        })
    }

    pub fn get_default_state_id(&self, block: &str) -> Option<u16> {
        BLOCKS.get(block).map(|b| b.default_state_id)
    }

    pub fn get_block_from_state(&self, state_id: u16) -> Option<(&str, &HashMap<String, String>)> {
        use std::sync::OnceLock;
        type StateCache = HashMap<u16, (String, HashMap<String, String>)>;
        static CACHE: OnceLock<StateCache> = OnceLock::new();
        let cache = CACHE.get_or_init(|| {
            let mut map = HashMap::new();
            for (id, (name, props)) in &STATES {
                let props_map: HashMap<String, String> = props
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                map.insert(*id, (name.to_string(), props_map));
            }
            map
        });
        cache
            .get(&state_id)
            .map(|(name, props)| (name.as_str(), props))
    }

    pub fn block_count(&self) -> usize {
        BLOCK_COUNT
    }

    pub fn get_block_entry(&self, block: &str) -> Option<BlockEntry> {
        let block_def = BLOCKS.get(block)?;
        let properties: HashMap<String, Vec<String>> = block_def
            .properties
            .iter()
            .map(|(k, vs)| (k.to_string(), vs.iter().map(|v| v.to_string()).collect()))
            .collect();
        let states: Vec<StateEntry> = block_def
            .states
            .iter()
            .map(|s| StateEntry {
                id: s.id,
                properties: s
                    .properties
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            })
            .collect();
        Some(BlockEntry {
            default_state_id: block_def.default_state_id,
            properties,
            states,
        })
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

    #[test]
    fn test_codegen_forward_reverse_roundtrip() {
        let registry = BlockRegistry::global();
        let test_blocks = [
            "minecraft:oak_log",
            "minecraft:stone",
            "minecraft:grass_block",
            "minecraft:redstone_wire",
        ];
        for block_name in &test_blocks {
            if let Some(default_id) = registry.get_default_state_id(block_name) {
                let (name, _) = registry.get_block_from_state(default_id).unwrap();
                assert_eq!(name, *block_name);
            }
        }
    }

    #[test]
    fn test_build_generates_valid_lookup_tables() {
        assert!(BLOCK_COUNT > 1000);
        assert!(BLOCKS.get("minecraft:air").is_some());
        assert!(BLOCKS.get("minecraft:stone").is_some());
        let air = BLOCKS.get("minecraft:air").unwrap();
        assert_eq!(air.name, "minecraft:air");
    }
}
