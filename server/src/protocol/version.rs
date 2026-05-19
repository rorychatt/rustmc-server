use std::io;

pub const PROTOCOL_VERSION: i32 = 775;
pub const VERSION_NAME: &str = "26.1.2";
pub const DATA_PACK_VERSION: &str = "1.21";

pub fn data_pack_version_for(protocol_version: i32) -> io::Result<&'static str> {
    match protocol_version {
        775 => Ok("1.21"),
        _ => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("unsupported protocol version: {protocol_version}"),
        )),
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
    fn test_data_pack_version_for_unknown() {
        assert!(data_pack_version_for(999).is_err());
    }
}
