use serde::Deserialize;
use std::collections::HashMap;

use crate::error::LockpickError;
use crate::{DepEdge, DependencyGraph, LockfileType, PackageInfo};

/// Represents a single package entry in the v3 `packages` map.
#[derive(Debug, Deserialize)]
pub struct NpmPackageEntry {
    pub version: Option<String>,
    pub resolved: Option<String>,
    pub integrity: Option<String>,
    #[serde(default)]
    pub dev: bool,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

/// Top-level package-lock.json structure (supports v1/v2/v3).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpmLockfile {
    pub lockfile_version: Option<u32>,
    #[serde(default)]
    pub packages: HashMap<String, NpmPackageEntry>,
    /// v1/v2 legacy format
    #[serde(default)]
    pub dependencies: HashMap<String, NpmLegacyDep>,
}

/// Legacy v1/v2 dependency entry
#[derive(Debug, Deserialize)]
pub struct NpmLegacyDep {
    pub version: Option<String>,
    pub resolved: Option<String>,
    pub integrity: Option<String>,
    #[serde(default)]
    pub dev: bool,
    /// Nested dependencies (v1 style)
    #[serde(default)]
    pub dependencies: HashMap<String, NpmLegacyDep>,
}

/// Parse package-lock.json (v1/v2/v3) content into a DependencyGraph.
pub fn parse(content: &str) -> Result<DependencyGraph, LockpickError> {
    let lockfile: NpmLockfile = serde_json::from_str(content)
        .map_err(|e| LockpickError::Parse(format!("Failed to parse package-lock.json: {e}")))?;

    let mut deps = HashMap::new();
    let mut dev_deps = HashMap::new();
    let mut all_packages: HashMap<String, Vec<String>> = HashMap::new();

    let version = lockfile.lockfile_version.unwrap_or(1);

    if version >= 3 || (version == 2 && !lockfile.packages.is_empty()) {
        // v3 (or v2 with packages field): use `packages` map
        parse_v3_packages(
            &lockfile.packages,
            &mut deps,
            &mut dev_deps,
            &mut all_packages,
        );
    } else {
        // v1 (or v2 without packages): use legacy `dependencies` map
        parse_v1_dependencies(
            &lockfile.dependencies,
            &mut deps,
            &mut dev_deps,
            &mut all_packages,
        );
    }

    Ok(DependencyGraph {
        dependencies: deps,
        dev_dependencies: dev_deps,
        lockfile_type: LockfileType::Npm,
        all_packages,
        dep_edges: HashMap::new(),
    })
}

/// Parse package-lock.json with transitive dependency edges.
pub fn parse_with_edges(content: &str) -> Result<DependencyGraph, LockpickError> {
    let lockfile: NpmLockfile = serde_json::from_str(content)
        .map_err(|e| LockpickError::Parse(format!("Failed to parse package-lock.json: {e}")))?;
    let mut graph = parse(content)?;

    let version = lockfile.lockfile_version.unwrap_or(1);
    if version >= 2 {
        for (key, entry) in &lockfile.packages {
            if key.is_empty() || entry.dependencies.is_empty() {
                continue;
            }
            let name = key
                .rfind("node_modules/")
                .map(|pos| &key[pos + "node_modules/".len()..])
                .unwrap_or(key);
            let Some(ref ver) = entry.version else {
                continue;
            };
            let pkg_key = format!("{name}@{ver}");
            let edges: Vec<DepEdge> = entry
                .dependencies
                .iter()
                .map(|(n, v)| DepEdge {
                    name: n.clone(),
                    version: v.clone(),
                })
                .collect();
            graph.dep_edges.insert(pkg_key, edges);
        }
    }
    Ok(graph)
}

/// Parse v3 (and v2 with packages) format
fn parse_v3_packages(
    packages: &HashMap<String, NpmPackageEntry>,
    deps: &mut HashMap<String, PackageInfo>,
    dev_deps: &mut HashMap<String, PackageInfo>,
    all_packages: &mut HashMap<String, Vec<String>>,
) {
    for (key, entry) in packages {
        if key.is_empty() {
            continue;
        }
        let name = key
            .rfind("node_modules/")
            .map(|pos| &key[pos + "node_modules/".len()..])
            .unwrap_or(key);

        let version = match &entry.version {
            Some(v) => v.clone(),
            None => continue,
        };

        all_packages
            .entry(name.to_string())
            .or_default()
            .push(version.clone());

        let info = PackageInfo {
            name: name.to_string(),
            version,
            integrity: entry.integrity.clone(),
        };

        if entry.dev {
            dev_deps.entry(name.to_string()).or_insert(info);
        } else {
            deps.entry(name.to_string()).or_insert(info);
        }
    }
}

/// Parse v1/v2 legacy `dependencies` format (recursive)
fn parse_v1_dependencies(
    legacy_deps: &HashMap<String, NpmLegacyDep>,
    deps: &mut HashMap<String, PackageInfo>,
    dev_deps: &mut HashMap<String, PackageInfo>,
    all_packages: &mut HashMap<String, Vec<String>>,
) {
    for (name, entry) in legacy_deps {
        let version = match &entry.version {
            Some(v) => v.clone(),
            None => continue,
        };

        all_packages
            .entry(name.clone())
            .or_default()
            .push(version.clone());

        let info = PackageInfo {
            name: name.clone(),
            version,
            integrity: entry.integrity.clone(),
        };

        if entry.dev {
            dev_deps.entry(name.clone()).or_insert(info);
        } else {
            deps.entry(name.clone()).or_insert(info);
        }

        // Recurse into nested dependencies
        if !entry.dependencies.is_empty() {
            parse_v1_dependencies(&entry.dependencies, deps, dev_deps, all_packages);
        }
    }
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

    #[test]
    fn test_parse_v1_basic() {
        let content = r#"{
            "name": "test",
            "version": "1.0.0",
            "lockfileVersion": 1,
            "dependencies": {
                "lodash": {
                    "version": "4.17.21",
                    "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
                    "integrity": "sha512-fake"
                },
                "typescript": {
                    "version": "5.3.3",
                    "dev": true
                }
            }
        }"#;
        let graph = parse(content).unwrap();
        assert_eq!(graph.lockfile_type, LockfileType::Npm);
        assert_eq!(graph.dependencies.len(), 1);
        assert!(graph.dependencies.contains_key("lodash"));
        assert_eq!(graph.dev_dependencies.len(), 1);
        assert!(graph.dev_dependencies.contains_key("typescript"));
    }

    #[test]
    fn test_parse_with_edges_v3() {
        let content = r#"{
            "lockfileVersion": 3,
            "packages": {
                "": { "name": "test", "version": "1.0.0" },
                "node_modules/react": {
                    "version": "18.2.0",
                    "dependencies": { "loose-envify": "^1.4.0" }
                },
                "node_modules/loose-envify": {
                    "version": "1.4.0",
                    "dependencies": { "js-tokens": "^4.0.0" }
                },
                "node_modules/js-tokens": { "version": "4.0.0" }
            }
        }"#;
        let graph = parse_with_edges(content).unwrap();
        assert_eq!(graph.dep_edges["react@18.2.0"].len(), 1);
        assert_eq!(graph.dep_edges["react@18.2.0"][0].name, "loose-envify");
        assert!(!graph.dep_edges.contains_key("js-tokens@4.0.0"));
    }
}
