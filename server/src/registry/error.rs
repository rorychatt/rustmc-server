use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    UnknownRegistry {
        registry_id: String,
        protocol_version: i32,
    },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRegistry {
                registry_id,
                protocol_version,
            } => {
                write!(
                    f,
                    "unknown registry '{registry_id}' for protocol version {protocol_version}"
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<RegistryError> for io::Error {
    fn from(e: RegistryError) -> Self {
        io::Error::new(io::ErrorKind::NotFound, e)
    }
}
