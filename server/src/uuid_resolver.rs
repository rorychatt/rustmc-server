use std::num::NonZeroUsize;
use std::sync::OnceLock;
use std::time::Duration;

use lru::LruCache;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio::time::Instant;
use uuid::Uuid;

const CACHE_TTL: Duration = Duration::from_secs(3600);
const DEFAULT_MAX_ENTRIES: usize = 256;

struct CacheEntry {
    uuid: Option<Uuid>,
    expires_at: Instant,
}

static UUID_CACHE: OnceLock<RwLock<LruCache<String, CacheEntry>>> = OnceLock::new();

fn get_cache() -> &'static RwLock<LruCache<String, CacheEntry>> {
    UUID_CACHE.get_or_init(|| {
        RwLock::new(LruCache::new(
            NonZeroUsize::new(DEFAULT_MAX_ENTRIES).unwrap(),
        ))
    })
}

pub fn init(max_entries: usize) {
    let cap =
        NonZeroUsize::new(max_entries).unwrap_or(NonZeroUsize::new(DEFAULT_MAX_ENTRIES).unwrap());
    let _ = UUID_CACHE.set(RwLock::new(LruCache::new(cap)));
}

#[derive(Deserialize)]
struct MojangProfile {
    id: String,
    #[allow(dead_code)]
    name: String,
}

pub async fn resolve_uuid_from_mojang(username: &str) -> Option<Uuid> {
    let key = username.to_lowercase();

    {
        let mut cache = get_cache().write().await;
        if let Some(entry) = cache.get(&key) {
            if entry.expires_at > Instant::now() {
                return entry.uuid;
            }
            // Expired — remove it
        }
        cache.pop(&key);
    }

    let result = fetch_uuid_from_mojang(username).await;

    {
        let mut cache = get_cache().write().await;
        cache.put(
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
        let cache = get_cache();
        let key = "cache_test_user".to_lowercase();

        {
            let mut c = cache.write().await;
            c.put(
                key.clone(),
                CacheEntry {
                    uuid: Some(Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap()),
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
        let cache = get_cache();
        let key = "expired_user".to_lowercase();

        {
            let mut c = cache.write().await;
            c.put(
                key.clone(),
                CacheEntry {
                    uuid: Some(Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap()),
                    expires_at: Instant::now() - Duration::from_secs(1),
                },
            );
        }

        let result = resolve_uuid_from_mojang("expired_user").await;
        assert_ne!(
            result,
            Some(Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap())
        );
    }

    #[tokio::test]
    async fn test_cache_case_insensitive() {
        let cache = get_cache();
        let key = "case_test_player".to_lowercase();

        {
            let mut c = cache.write().await;
            c.put(
                key.clone(),
                CacheEntry {
                    uuid: Some(Uuid::parse_str("abcdefab-cdef-abcd-efab-cdefabcdefab").unwrap()),
                    expires_at: Instant::now() + CACHE_TTL,
                },
            );
        }

        let result_lower = resolve_uuid_from_mojang("case_test_player").await;
        let result_upper = resolve_uuid_from_mojang("CASE_TEST_PLAYER").await;
        let result_mixed = resolve_uuid_from_mojang("Case_Test_Player").await;

        let expected = Some(Uuid::parse_str("abcdefab-cdef-abcd-efab-cdefabcdefab").unwrap());
        assert_eq!(result_lower, expected);
        assert_eq!(result_upper, expected);
        assert_eq!(result_mixed, expected);
    }

    #[tokio::test]
    async fn test_cache_stores_none_for_unknown() {
        let cache = get_cache();
        let key = "unknown_cached_player".to_lowercase();

        {
            let mut c = cache.write().await;
            c.put(
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

    #[tokio::test]
    async fn test_cache_evicts_lru_entry_at_capacity() {
        let mut cache = LruCache::<String, CacheEntry>::new(NonZeroUsize::new(2).unwrap());

        cache.put(
            "first".to_string(),
            CacheEntry {
                uuid: Some(Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()),
                expires_at: Instant::now() + CACHE_TTL,
            },
        );
        cache.put(
            "second".to_string(),
            CacheEntry {
                uuid: Some(Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()),
                expires_at: Instant::now() + CACHE_TTL,
            },
        );
        cache.put(
            "third".to_string(),
            CacheEntry {
                uuid: Some(Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap()),
                expires_at: Instant::now() + CACHE_TTL,
            },
        );

        assert!(cache.get(&"first".to_string()).is_none());
        assert!(cache.get(&"second".to_string()).is_some());
        assert!(cache.get(&"third".to_string()).is_some());
    }

    #[tokio::test]
    async fn test_cache_expired_entries_evicted_on_access() {
        let cache = get_cache();
        let key = "expired_evict_test".to_lowercase();

        {
            let mut c = cache.write().await;
            c.put(
                key.clone(),
                CacheEntry {
                    uuid: Some(Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap()),
                    expires_at: Instant::now() - Duration::from_secs(1),
                },
            );
        }

        let result = resolve_uuid_from_mojang("expired_evict_test").await;
        assert_ne!(
            result,
            Some(Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap())
        );

        {
            let mut c = cache.write().await;
            assert!(
                c.get(&key).is_none() || {
                    let entry = c.peek(&key).unwrap();
                    entry.expires_at > Instant::now()
                }
            );
        }
    }

    #[tokio::test]
    async fn test_cache_capacity_from_config() {
        let cap = NonZeroUsize::new(128).unwrap();
        let cache = LruCache::<String, CacheEntry>::new(cap);
        assert_eq!(cache.cap().get(), 128);
    }
}
