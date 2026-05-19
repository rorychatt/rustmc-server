mod loader;
pub mod nbt_encoder;

use crate::protocol::configuration::RegistryEntry;
use std::io;

pub use loader::RegistrySet;

pub fn registry_set_for(protocol_version: i32) -> io::Result<&'static RegistrySet> {
    loader::registry_set_for(protocol_version)
}

pub fn load(registry_id: &str, protocol_version: i32) -> io::Result<&'static [RegistryEntry]> {
    loader::load_registry(registry_id, protocol_version)
}
