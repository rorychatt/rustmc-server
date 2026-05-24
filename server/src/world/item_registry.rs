mod generated {
    include!(concat!(env!("OUT_DIR"), "/item_registry_generated.rs"));
}

pub use generated::ITEMS;

pub struct ItemRegistry;

impl ItemRegistry {
    pub fn get_id(item_id: &str) -> Option<i32> {
        ITEMS.get(item_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diamond_sword_id() {
        assert_eq!(ItemRegistry::get_id("minecraft:diamond_sword"), Some(837));
        assert_eq!(ItemRegistry::get_id("diamond_sword"), Some(837));
    }

    #[test]
    fn test_air_id() {
        assert_eq!(ItemRegistry::get_id("minecraft:air"), Some(0));
        assert_eq!(ItemRegistry::get_id("air"), Some(0));
    }
}
