use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::Vulnerability;
use crate::error::LockpickError;

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    vulns: Vec<Vulnerability>,
    cached_at: u64,
}

/// Return the default cache directory for OSV results.
/// Falls back to the system temp directory when no user cache dir is available.
fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("lockpick")
        .join("osv")
}

/// Sanitize a string for safe use in filenames: keep alphanumeric, '-', '.', '_' only.
fn sanitize_for_filename(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' { c } else { '_' })
        .collect()
}

/// Build a safe cache filename from package name and version.
fn cache_filename(name: &str, version: &str) -> String {
    let safe_name = sanitize_for_filename(name);
    let safe_version = sanitize_for_filename(version);
    format!("{safe_name}@{safe_version}.json")
}

fn now_secs() -> u64 {
    SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap_or_default()
        .as_secs()
}

// --- Internal functions that accept a custom cache dir (for testing) ---

fn get_from(cache_dir: &Path, name: &str, version: &str, ttl_secs: u64) -> Option<Vec<Vulnerability>> {
    let path = cache_dir.join(cache_filename(name, version));
    let data = fs::read_to_string(&path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&data).ok()?;
    let now = now_secs();
    if entry.cached_at + ttl_secs > now {
        Some(entry.vulns)
    } else {
        None
    }
}

fn set_to(cache_dir: &Path, name: &str, version: &str, vulns: &[Vulnerability]) -> Result<(), LockpickError> {
    fs::create_dir_all(cache_dir)?;
    let entry = CacheEntry {
        vulns: vulns.to_vec(),
        cached_at: now_secs(),
    };
    let path = cache_dir.join(cache_filename(name, version));
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

// --- Public API (uses default cache dir) ---

/// Get cached vulnerabilities for a package. Returns None if missing or expired.
pub fn get(name: &str, version: &str, ttl_secs: u64) -> Option<Vec<Vulnerability>> {
    get_from(&default_cache_dir(), name, version, ttl_secs)
}

/// Store vulnerabilities in cache.
pub fn set(name: &str, version: &str, vulns: &[Vulnerability]) -> Result<(), LockpickError> {
    set_to(&default_cache_dir(), name, version, vulns)
}

/// Clear all cached entries.
pub fn clear() -> Result<(), LockpickError> {
    clear_dir(&default_cache_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Severity;
    use tempfile::TempDir;

    fn sample_vulns() -> Vec<Vulnerability> {
        vec![Vulnerability {
            id: "GHSA-1234".into(),
            summary: "Test vulnerability".into(),
            severity: Severity::High,
            fixed_version: Some("1.2.3".into()),
        }]
    }

    #[test]
    fn test_cache_set_and_get() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let vulns = sample_vulns();
        set_to(dir, "lodash", "4.17.20", &vulns).unwrap();
        let cached = get_from(dir, "lodash", "4.17.20", 3600).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].id, "GHSA-1234");
    }

    #[test]
    fn test_cache_expired() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let entry = CacheEntry {
            vulns: sample_vulns(),
            cached_at: 1000,
        };
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(cache_filename("lodash", "4.17.20"));
        let json = serde_json::to_string(&entry).unwrap();
        fs::write(&path, json).unwrap();
        let result = get_from(dir, "lodash", "4.17.20", 3600);
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_missing() {
        let tmp = TempDir::new().unwrap();
        let result = get_from(tmp.path(), "nonexistent", "0.0.0", 3600);
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("osv_cache");
        set_to(&dir, "lodash", "4.17.20", &sample_vulns()).unwrap();
        assert!(dir.exists());
        // Add a non-json file that should survive clear
        fs::write(dir.join("keep.txt"), "keep me").unwrap();
        clear_dir(&dir).unwrap();
        // Directory still exists, non-json file preserved, json files removed
        assert!(dir.exists());
        assert!(dir.join("keep.txt").exists());
        assert!(!dir.join(cache_filename("lodash", "4.17.20")).exists());
    }

    #[test]
    fn test_scoped_package_name() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        set_to(dir, "@scope/pkg", "1.0.0", &sample_vulns()).unwrap();
        // '@' and '/' are sanitized to '_'
        let expected_file = dir.join("_scope_pkg@1.0.0.json");
        assert!(expected_file.exists());
        let cached = get_from(dir, "@scope/pkg", "1.0.0", 3600).unwrap();
        assert_eq!(cached.len(), 1);
    }
}
