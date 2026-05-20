use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::RwLock;
use tokio::time::Instant;
use uuid::Uuid;

const CACHE_TTL: Duration = Duration::from_secs(3600);

struct CacheEntry {
    uuid: Option<Uuid>,
    expires_at: Instant,
}

static UUID_CACHE: LazyLock<RwLock<HashMap<String, CacheEntry>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Deserialize)]
struct MojangProfile {
    id: String,
    #[allow(dead_code)]
    name: String,
}

pub async fn resolve_uuid_from_mojang(username: &str) -> Option<Uuid> {
    let key = username.to_lowercase();

    {
        let cache = UUID_CACHE.read().await;
        if let Some(entry) = cache.get(&key) {
            if entry.expires_at > Instant::now() {
                return entry.uuid;
            }
        }
    }

    let result = fetch_uuid_from_mojang(username).await;

    {
        let mut cache = UUID_CACHE.write().await;
        cache.insert(
            key,
            CacheEntry {
                uuid: result,
                expires_at: Instant::now() + CACHE_TTL,
            },
        );
    }

    result
}

async fn fetch_uuid_from_mojang(username: &str) -> Option<Uuid> {
    let url = format!(
        "https://api.mojang.com/users/profiles/minecraft/{}",
        username
    );
    let resp = reqwest::get(&url).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let profile: MojangProfile = resp.json().await.ok()?;
    Uuid::parse_str(&profile.id).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_known_username() {
        let result = resolve_uuid_from_mojang("Notch").await;
        if let Some(uuid) = result {
            assert_eq!(uuid.to_string(), "069a79f4-44e9-4726-a5be-fca90e38aaf5");
        }
    }

    #[tokio::test]
    async fn test_resolve_nonexistent_username() {
        let result = resolve_uuid_from_mojang("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_returns_stored_value() {
        let key = "cache_test_user".to_lowercase();

        {
            let mut cache = UUID_CACHE.write().await;
            cache.insert(
                key.clone(),
                CacheEntry {
                    uuid: Some(
                        Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap(),
                    ),
                    expires_at: Instant::now() + CACHE_TTL,
                },
            );
        }

        let result = resolve_uuid_from_mojang("Cache_Test_User").await;
        assert_eq!(
            result,
            Some(Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap())
        );
    }

    #[tokio::test]
    async fn test_cache_expires_after_ttl() {
        let key = "expired_user".to_lowercase();

        {
            let mut cache = UUID_CACHE.write().await;
            cache.insert(
                key.clone(),
                CacheEntry {
                    uuid: Some(
                        Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap(),
                    ),
                    expires_at: Instant::now() - Duration::from_secs(1),
                },
            );
        }

        let result = resolve_uuid_from_mojang("expired_user").await;
        // Expired entry should not be returned; the API call will determine the result
        // (likely None for a non-existent username)
        assert_ne!(
            result,
            Some(Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap())
        );
    }

    #[tokio::test]
    async fn test_cache_case_insensitive() {
        let key = "case_test_player".to_lowercase();

        {
            let mut cache = UUID_CACHE.write().await;
            cache.insert(
                key.clone(),
                CacheEntry {
                    uuid: Some(
                        Uuid::parse_str("abcdefab-cdef-abcd-efab-cdefabcdefab").unwrap(),
                    ),
                    expires_at: Instant::now() + CACHE_TTL,
                },
            );
        }

        let result_lower = resolve_uuid_from_mojang("case_test_player").await;
        let result_upper = resolve_uuid_from_mojang("CASE_TEST_PLAYER").await;
        let result_mixed = resolve_uuid_from_mojang("Case_Test_Player").await;

        let expected =
            Some(Uuid::parse_str("abcdefab-cdef-abcd-efab-cdefabcdefab").unwrap());
        assert_eq!(result_lower, expected);
        assert_eq!(result_upper, expected);
        assert_eq!(result_mixed, expected);
    }

    #[tokio::test]
    async fn test_cache_stores_none_for_unknown() {
        let key = "unknown_cached_player".to_lowercase();

        {
            let mut cache = UUID_CACHE.write().await;
            cache.insert(
                key.clone(),
                CacheEntry {
                    uuid: None,
                    expires_at: Instant::now() + CACHE_TTL,
                },
            );
        }

        let result = resolve_uuid_from_mojang("unknown_cached_player").await;
        assert_eq!(result, None);
    }
}
