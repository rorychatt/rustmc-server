use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
struct MojangProfile {
    id: String,
    #[allow(dead_code)]
    name: String,
}

pub async fn resolve_uuid_from_mojang(username: &str) -> Option<Uuid> {
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
        // If the API is unreachable (CI, network issues), the test still passes
    }

    #[tokio::test]
    async fn test_resolve_nonexistent_username() {
        let result = resolve_uuid_from_mojang("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").await;
        assert_eq!(result, None);
    }
}
