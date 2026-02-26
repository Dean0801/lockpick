use serde::Deserialize;
use std::collections::HashMap;

use crate::{DependencyGraph, PackageInfo};

/// Top-level pnpm-lock.yaml v9 structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmLockfile {
    pub lockfile_version: String,
    #[serde(default)]
    pub importers: HashMap<String, PnpmImporter>,
    #[serde(default)]
    pub packages: HashMap<String, PnpmPackage>,
}

/// An importer entry (workspace project)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmImporter {
    #[serde(default)]
    pub dependencies: HashMap<String, PnpmDepEntry>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, PnpmDepEntry>,
}

/// A dependency entry in importers
#[derive(Debug, Deserialize)]
pub struct PnpmDepEntry {
    pub specifier: String,
    pub version: String,
}

/// A package entry in packages section
#[derive(Debug, Deserialize)]
pub struct PnpmPackage {
    #[serde(default)]
    pub resolution: PnpmResolution,
}

/// Package resolution info
#[derive(Debug, Default, Deserialize)]
pub struct PnpmResolution {
    pub integrity: Option<String>,
}

/// Parse pnpm-lock.yaml content into DependencyGraph
pub fn parse(content: &str) -> Result<DependencyGraph, String> {
    let lockfile: PnpmLockfile = serde_yaml::from_str(content)
        .map_err(|e| format!("Failed to parse pnpm-lock.yaml: {e}"))?;

    let importer = lockfile
        .importers
        .get(".")
        .ok_or("No root importer found in pnpm-lock.yaml")?;

    let mut deps = HashMap::new();
    let mut dev_deps = HashMap::new();

    for (name, entry) in &importer.dependencies {
        let pkg_key = format!("{name}@{}", entry.version);
        let integrity = lockfile
            .packages
            .get(&pkg_key)
            .and_then(|p| p.resolution.integrity.clone());

        deps.insert(
            name.clone(),
            PackageInfo {
                name: name.clone(),
                version: entry.version.clone(),
                integrity,
            },
        );
    }

    for (name, entry) in &importer.dev_dependencies {
        let pkg_key = format!("{name}@{}", entry.version);
        let integrity = lockfile
            .packages
            .get(&pkg_key)
            .and_then(|p| p.resolution.integrity.clone());

        dev_deps.insert(
            name.clone(),
            PackageInfo {
                name: name.clone(),
                version: entry.version.clone(),
                integrity,
            },
        );
    }

    Ok(DependencyGraph {
        dependencies: deps,
        dev_dependencies: dev_deps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v9_basic() {
        let content = include_str!("../../tests/fixtures/pnpm-lock.yaml");
        let graph = parse(content).unwrap();

        assert_eq!(graph.dependencies.len(), 2);
        assert_eq!(graph.dev_dependencies.len(), 2);

        let react = &graph.dependencies["react"];
        assert_eq!(react.version, "18.2.0");
        assert!(react.integrity.as_ref().unwrap().contains("fake-react"));

        let lodash = &graph.dependencies["lodash"];
        assert_eq!(lodash.version, "4.17.21");

        let ts = &graph.dev_dependencies["typescript"];
        assert_eq!(ts.version, "5.3.3");

        let types_react = &graph.dev_dependencies["@types/react"];
        assert_eq!(types_react.version, "18.2.48");
    }

    #[test]
    fn test_parse_empty_importers() {
        let content = "lockfileVersion: '9.0'\nimporters: {}\n";
        let result = parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let result = parse("not: [valid: yaml: {{");
        assert!(result.is_err());
    }
}
