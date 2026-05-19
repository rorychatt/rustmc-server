use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const TOKEN_VALIDITY_SECS: u64 = 30;

pub struct TransferToken {
    pub origin: String,
    pub player_uuid: Uuid,
    pub player_name: String,
    pub timestamp: u64,
}

pub fn generate_token(secret: &[u8], token: &TransferToken) -> Vec<u8> {
    let message = format!(
        "{}|{}|{}|{}",
        token.origin, token.player_uuid, token.player_name, token.timestamp
    );
    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let signature = mac.finalize().into_bytes();

    let mut payload = message.into_bytes();
    payload.push(b'|');
    payload.extend_from_slice(&signature);
    payload
}

pub fn validate_token(secret: &[u8], payload: &[u8]) -> Option<TransferToken> {
    if payload.len() < 33 {
        return None;
    }

    let sig_separator = payload.len() - 32 - 1;
    if payload[sig_separator] != b'|' {
        return None;
    }

    let message = &payload[..sig_separator];
    let signature = &payload[sig_separator + 1..];

    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(message);
    if mac.verify_slice(signature).is_err() {
        return None;
    }

    let message_str = std::str::from_utf8(message).ok()?;
    let parts: Vec<&str> = message_str.splitn(4, '|').collect();
    if parts.len() != 4 {
        return None;
    }

    let origin = parts[0].to_string();
    let player_uuid = Uuid::parse_str(parts[1]).ok()?;
    let player_name = parts[2].to_string();
    let timestamp: u64 = parts[3].parse().ok()?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now.saturating_sub(timestamp) > TOKEN_VALIDITY_SECS {
        return None;
    }

    Some(TransferToken {
        origin,
        player_uuid,
        player_name,
        timestamp,
    })
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_validate_roundtrip() {
        let secret = b"test-secret-key";
        let token = TransferToken {
            origin: "127.0.0.1:25565".to_string(),
            player_uuid: Uuid::new_v4(),
            player_name: "TestPlayer".to_string(),
            timestamp: current_timestamp(),
        };

        let payload = generate_token(secret, &token);
        let result = validate_token(secret, &payload).expect("Token should be valid");

        assert_eq!(result.origin, token.origin);
        assert_eq!(result.player_uuid, token.player_uuid);
        assert_eq!(result.player_name, token.player_name);
        assert_eq!(result.timestamp, token.timestamp);
    }

    #[test]
    fn test_expired_token_rejected() {
        let secret = b"test-secret-key";
        let token = TransferToken {
            origin: "127.0.0.1:25565".to_string(),
            player_uuid: Uuid::new_v4(),
            player_name: "TestPlayer".to_string(),
            timestamp: current_timestamp() - 60,
        };

        let payload = generate_token(secret, &token);
        assert!(
            validate_token(secret, &payload).is_none(),
            "Expired token should be rejected"
        );
    }

    #[test]
    fn test_tampered_payload_rejected() {
        let secret = b"test-secret-key";
        let token = TransferToken {
            origin: "127.0.0.1:25565".to_string(),
            player_uuid: Uuid::new_v4(),
            player_name: "TestPlayer".to_string(),
            timestamp: current_timestamp(),
        };

        let mut payload = generate_token(secret, &token);
        // Tamper with the message portion
        if payload.len() > 5 {
            payload[5] ^= 0xFF;
        }
        assert!(
            validate_token(secret, &payload).is_none(),
            "Tampered token should be rejected"
        );
    }

    #[test]
    fn test_wrong_secret_rejected() {
        let secret = b"correct-secret";
        let wrong_secret = b"wrong-secret";
        let token = TransferToken {
            origin: "127.0.0.1:25565".to_string(),
            player_uuid: Uuid::new_v4(),
            player_name: "TestPlayer".to_string(),
            timestamp: current_timestamp(),
        };

        let payload = generate_token(secret, &token);
        assert!(
            validate_token(wrong_secret, &payload).is_none(),
            "Token with wrong secret should be rejected"
        );
    }

    #[test]
    fn test_empty_payload_rejected() {
        let secret = b"test-secret-key";
        assert!(validate_token(secret, &[]).is_none());
    }
}
