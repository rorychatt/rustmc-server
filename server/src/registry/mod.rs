pub mod error;
mod loader;
pub mod nbt_encoder;

use crate::protocol::configuration::RegistryEntry;
use crate::protocol::packet::Packet;
use crate::protocol::version::PROTOCOL_VERSION;
use std::io;

pub use error::RegistryError;
pub use loader::RegistrySet;

pub const ALL_REGISTRY_IDS: &[&str] = loader::v775::REGISTRY_IDS;

pub fn registry_set_for(protocol_version: i32) -> io::Result<&'static RegistrySet> {
    loader::registry_set_for(protocol_version)
}

pub fn load(registry_id: &str, protocol_version: i32) -> io::Result<Vec<RegistryEntry>> {
    loader::load_registry(registry_id, protocol_version)
}

pub fn load_current(registry_id: &str) -> io::Result<Vec<RegistryEntry>> {
    loader::load_registry(registry_id, PROTOCOL_VERSION)
}

pub fn cached_registry_packets(protocol_version: i32) -> io::Result<&'static [Packet]> {
    loader::cached_registry_packets(protocol_version)
}
