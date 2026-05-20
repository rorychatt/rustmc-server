use std::io;

pub const PROTOCOL_VERSION: i32 = 775;
pub const VERSION_NAME: &str = "26.1.2";
pub const DATA_PACK_VERSION: &str = "1.21";
pub const SUPPORTED_VERSIONS: &[i32] = &[775];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolVersionError {
    UnsupportedVersion {
        requested: i32,
        supported: &'static [i32],
    },
}

impl std::fmt::Display for ProtocolVersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion {
                requested,
                supported,
            } => {
                write!(
                    f,
                    "unsupported protocol version {requested} (supported: {supported:?})"
                )
            }
        }
    }
}

impl std::error::Error for ProtocolVersionError {}

impl From<ProtocolVersionError> for io::Error {
    fn from(e: ProtocolVersionError) -> Self {
        io::Error::new(io::ErrorKind::Unsupported, e)
    }
}

pub fn data_pack_version_for(protocol_version: i32) -> io::Result<&'static str> {
    match protocol_version {
        775 => Ok("1.21"),
        _ => Err(ProtocolVersionError::UnsupportedVersion {
            requested: protocol_version,
            supported: SUPPORTED_VERSIONS,
        }
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version_constants() {
        assert_eq!(PROTOCOL_VERSION, 775);
        assert_eq!(VERSION_NAME, "26.1.2");
        assert_eq!(DATA_PACK_VERSION, "1.21");
    }

    #[test]
    fn test_data_pack_version_for_775() {
        assert_eq!(data_pack_version_for(775).unwrap(), "1.21");
    }

    #[test]
    fn test_data_pack_version_for_unknown_returns_error() {
        let result = data_pack_version_for(999);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn test_protocol_version_error_display() {
        let err = ProtocolVersionError::UnsupportedVersion {
            requested: 999,
            supported: SUPPORTED_VERSIONS,
        };
        assert_eq!(
            err.to_string(),
            "unsupported protocol version 999 (supported: [775])"
        );
    }

    #[test]
    fn test_protocol_version_error_downcast() {
        let result = data_pack_version_for(999);
        let err = result.unwrap_err();
        let source = err.get_ref().unwrap();
        let pve = source.downcast_ref::<ProtocolVersionError>().unwrap();
        assert_eq!(
            *pve,
            ProtocolVersionError::UnsupportedVersion {
                requested: 999,
                supported: SUPPORTED_VERSIONS,
            }
        );
    }
}
