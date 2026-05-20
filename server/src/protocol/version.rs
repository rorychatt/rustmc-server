use tracing::warn;

pub const PROTOCOL_VERSION: i32 = 775;
pub const VERSION_NAME: &str = "26.1.2";
pub const DATA_PACK_VERSION: &str = "1.21";
pub const SUPPORTED_VERSIONS: &[i32] = &[775];

pub fn data_pack_version_for(protocol_version: i32) -> &'static str {
    match protocol_version {
        775 => "1.21",
        _ => {
            warn!(
                "Unsupported protocol version {protocol_version}, falling back to latest known data pack version {DATA_PACK_VERSION}"
            );
            DATA_PACK_VERSION
        }
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
        assert_eq!(data_pack_version_for(775), "1.21");
    }

    #[test]
    fn test_data_pack_version_for_unknown_falls_back() {
        assert_eq!(data_pack_version_for(999), DATA_PACK_VERSION);
    }
}
