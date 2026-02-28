use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::LockpickError;

const DEFAULT_REGISTRY_TTL: u64 = 3600; // 1 hour

#[derive(Serialize, Deserialize)]
struct RegistryCacheEntry {
    latest: String,
    cached_at: u64,
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("lockpick")
        .join("registry")
}

fn sanitize_for_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn now_secs() -> u64 {
    SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap_or_default()
        .as_secs()
}

fn cache_filename(name: &str) -> String {
    format!("{}.json", sanitize_for_filename(name))
}

// --- Internal functions for testing ---

fn get_from(cache_dir: &Path, name: &str, ttl_secs: u64) -> Option<String> {
    let path = cache_dir.join(cache_filename(name));
    let data = fs::read_to_string(&path).ok()?;
    let entry: RegistryCacheEntry = serde_json::from_str(&data).ok()?;
    if entry.cached_at + ttl_secs > now_secs() {
        Some(entry.latest)
    } else {
        None
    }
}

fn set_to(cache_dir: &Path, name: &str, latest: &str) -> Result<(), LockpickError> {
    fs::create_dir_all(cache_dir)?;
    let entry = RegistryCacheEntry {
        latest: latest.to_string(),
        cached_at: now_secs(),
    };
    let path = cache_dir.join(cache_filename(name));
    let json = serde_json::to_string(&entry)
        .map_err(|e| LockpickError::Parse(format!("cache serialize error: {e}")))?;
    fs::write(&path, json)?;
    Ok(())
}

fn clear_dir(cache_dir: &Path) -> Result<(), LockpickError> {
    if cache_dir.exists() {
        for entry in fs::read_dir(cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                fs::remove_file(&path)?;
            }
        }
    }
    Ok(())
}

// --- Public API ---

/// Get cached latest version for a package. Returns None if missing or expired.
pub fn get(name: &str, ttl_secs: u64) -> Option<String> {
    get_from(&default_cache_dir(), name, ttl_secs)
}

/// Store latest version in cache.
pub fn set(name: &str, latest: &str) -> Result<(), LockpickError> {
    set_to(&default_cache_dir(), name, latest)
}

/// Clear all registry cache entries.
pub fn clear() -> Result<(), LockpickError> {
    clear_dir(&default_cache_dir())
}

/// Default TTL for registry cache (1 hour).
pub fn default_ttl() -> u64 {
    DEFAULT_REGISTRY_TTL
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_registry_cache_set_and_get() {
        let tmp = TempDir::new().unwrap();
        set_to(tmp.path(), "lodash", "4.17.21").unwrap();
        let cached = get_from(tmp.path(), "lodash", 3600).unwrap();
        assert_eq!(cached, "4.17.21");
    }

    #[test]
    fn test_registry_cache_expired() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path()).unwrap();
        let entry = RegistryCacheEntry {
            latest: "4.17.21".into(),
            cached_at: 1000,
        };
        let path = tmp.path().join(cache_filename("lodash"));
        fs::write(&path, serde_json::to_string(&entry).unwrap()).unwrap();
        assert!(get_from(tmp.path(), "lodash", 3600).is_none());
    }

    #[test]
    fn test_registry_cache_scoped_package() {
        let tmp = TempDir::new().unwrap();
        set_to(tmp.path(), "@scope/pkg", "2.0.0").unwrap();
        let expected = tmp.path().join("_scope_pkg.json");
        assert!(expected.exists());
        let cached = get_from(tmp.path(), "@scope/pkg", 3600).unwrap();
        assert_eq!(cached, "2.0.0");
    }
}
