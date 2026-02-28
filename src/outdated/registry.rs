use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::cache;
use crate::error::LockpickError;

const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";
const MAX_CONCURRENT: usize = 16;

/// Encode package name for npm registry URL.
/// Scoped packages like `@scope/pkg` become `@scope%2fpkg`.
fn encode_package_name(name: &str) -> String {
    if let Some(rest) = name.strip_prefix('@') {
        format!("@{}", rest.replace('/', "%2f"))
    } else {
        name.to_string()
    }
}

/// Fetch latest version for multiple packages concurrently.
/// Failed packages are silently skipped (warning to stderr).
pub async fn fetch_latest_versions(
    packages: &[(String, String)],
    registry_url: Option<&str>,
    no_cache: bool,
    cache_ttl: Option<u64>,
) -> Result<HashMap<String, String>, LockpickError> {
    let base = registry_url
        .unwrap_or(DEFAULT_REGISTRY)
        .trim_end_matches('/');
    let ttl = cache_ttl.unwrap_or_else(cache::registry::default_ttl);
    let sem = Arc::new(Semaphore::new(MAX_CONCURRENT));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| LockpickError::Network(format!("HTTP client error: {e}")))?;

    let mut handles = Vec::new();
    let mut cached_results: Vec<(String, String)> = Vec::new();

    for (name, _version) in packages {
        // Check cache first
        if !no_cache && let Some(latest) = cache::registry::get(name, ttl) {
            cached_results.push((name.clone(), latest));
            continue;
        }

        let sem = Arc::clone(&sem);
        let client = client.clone();
        let name = name.clone();
        let url = format!("{}/{}/latest", base, encode_package_name(&name));
        let use_cache = !no_cache;

        handles.push(tokio::spawn(async move {
            let Ok(_permit) = sem.acquire().await else {
                return None;
            };
            let resp = match client.get(&url).send().await {
                Ok(r) if r.status().is_success() => r,
                Ok(r) => {
                    eprintln!("warning: registry returned {} for {}", r.status(), name);
                    return None;
                }
                Err(e) => {
                    eprintln!("warning: failed to fetch {}: {e}", name);
                    return None;
                }
            };

            let json: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("warning: failed to parse response for {}: {e}", name);
                    return None;
                }
            };

            let latest = json.get("version")?.as_str()?.to_string();

            if use_cache && let Err(e) = cache::registry::set(&name, &latest) {
                eprintln!("warning: failed to cache {}: {e}", name);
            }

            Some((name, latest))
        }));
    }

    let mut result: HashMap<String, String> = cached_results.into_iter().collect();
    for handle in handles {
        if let Ok(Some((name, version))) = handle.await {
            result.insert(name, version);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_scoped_package() {
        assert_eq!(encode_package_name("@scope/pkg"), "@scope%2fpkg");
    }

    #[test]
    fn test_encode_normal_package() {
        assert_eq!(encode_package_name("lodash"), "lodash");
    }
}
