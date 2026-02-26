use serde::Deserialize;
use std::collections::HashMap;

use crate::{DependencyGraph, LockfileType, PackageInfo};

/// Represents a single package entry in the v3 `packages` map.
#[derive(Debug, Deserialize)]
pub struct NpmPackageEntry {
    pub version: Option<String>,
    pub resolved: Option<String>,
    pub integrity: Option<String>,
    #[serde(default)]
    pub dev: bool,
}

/// Top-level package-lock.json v3 structure.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpmLockfile {
    pub lockfile_version: u32,
    #[serde(default)]
    pub packages: HashMap<String, NpmPackageEntry>,
}

/// Parse package-lock.json (v3) content into a DependencyGraph.
pub fn parse(content: &str) -> Result<DependencyGraph, String> {
    let lockfile: NpmLockfile =
        serde_json::from_str(content).map_err(|e| format!("Failed to parse package-lock.json: {e}"))?;

    let mut deps = HashMap::new();
    let mut dev_deps = HashMap::new();
    let mut all_packages: HashMap<String, Vec<String>> = HashMap::new();

    for (key, entry) in &lockfile.packages {
        // Skip the root entry (empty string key)
        if key.is_empty() {
            continue;
        }

        // Extract package name: strip "node_modules/" prefix, handle scoped packages
        let name = key
            .rfind("node_modules/")
            .map(|pos| &key[pos + "node_modules/".len()..])
            .unwrap_or(key);

        let version = match &entry.version {
            Some(v) => v.clone(),
            None => continue,
        };

        // Track in all_packages
        all_packages
            .entry(name.to_string())
            .or_default()
            .push(version.clone());

        let info = PackageInfo {
            name: name.to_string(),
            version: version.clone(),
            integrity: entry.integrity.clone(),
        };

        if entry.dev {
            dev_deps.insert(name.to_string(), info);
        } else {
            deps.insert(name.to_string(), info);
        }
    }

    Ok(DependencyGraph {
        dependencies: deps,
        dev_dependencies: dev_deps,
        lockfile_type: LockfileType::Npm,
        all_packages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v3_basic() {
        let content = include_str!("../../tests/fixtures/package-lock.json");
        let graph = parse(content).unwrap();

        // Verify lockfile type
        assert_eq!(graph.lockfile_type, LockfileType::Npm);

        // 2 prod deps: react, lodash
        assert_eq!(graph.dependencies.len(), 2);
        assert!(graph.dependencies.contains_key("react"));
        assert!(graph.dependencies.contains_key("lodash"));

        // 1 dev dep: typescript
        assert_eq!(graph.dev_dependencies.len(), 1);
        assert!(graph.dev_dependencies.contains_key("typescript"));

        // Verify versions
        assert_eq!(graph.dependencies["react"].version, "18.2.0");
        assert_eq!(graph.dependencies["lodash"].version, "4.17.21");
        assert_eq!(graph.dev_dependencies["typescript"].version, "5.3.3");

        // Verify all_packages is populated
        assert_eq!(graph.all_packages.len(), 3);
        assert_eq!(graph.all_packages["react"], vec!["18.2.0"]);
        assert_eq!(graph.all_packages["lodash"], vec!["4.17.21"]);
        assert_eq!(graph.all_packages["typescript"], vec!["5.3.3"]);
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse("this is not valid json {{{");
        assert!(result.is_err());
    }
}
