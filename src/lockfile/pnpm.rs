use serde::Deserialize;
use std::collections::HashMap;

use crate::error::LockpickError;
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

/// Build all_packages map from the packages section keys (e.g. "react@18.2.0")
fn build_all_packages(packages: &HashMap<String, PnpmPackage>) -> HashMap<String, Vec<String>> {
    let mut all_packages: HashMap<String, Vec<String>> = HashMap::new();
    for key in packages.keys() {
        if let Some(pos) = key.rfind('@')
            && pos > 0
        {
            let name = &key[..pos];
            let version = &key[pos + 1..];
            all_packages
                .entry(name.to_string())
                .or_default()
                .push(version.to_string());
        }
    }
    all_packages
}

/// Parse pnpm-lock.yaml content into DependencyGraph (merges all importers)
pub fn parse(content: &str) -> Result<DependencyGraph, LockpickError> {
    let lockfile: PnpmLockfile = serde_yml::from_str(content)
        .map_err(|e| LockpickError::Parse(format!("Failed to parse pnpm-lock.yaml: {e}")))?;

    if lockfile.importers.is_empty() {
        return Err(LockpickError::Parse(
            "No importers found in pnpm-lock.yaml".into(),
        ));
    }

    let mut deps = HashMap::new();
    let mut dev_deps = HashMap::new();

    // Merge all importers for root-level analysis
    for importer in lockfile.importers.values() {
        collect_importer_deps(importer, &lockfile.packages, &mut deps, &mut dev_deps);
    }

    // Build all_packages from the packages section
    let all_packages = build_all_packages(&lockfile.packages);

    Ok(DependencyGraph {
        dependencies: deps,
        dev_dependencies: dev_deps,
        lockfile_type: crate::LockfileType::Pnpm,
        all_packages,
    })
}

/// Collect deps from a single importer into the provided maps
fn collect_importer_deps(
    importer: &PnpmImporter,
    packages: &HashMap<String, PnpmPackage>,
    deps: &mut HashMap<String, PackageInfo>,
    dev_deps: &mut HashMap<String, PackageInfo>,
) {
    for (name, entry) in &importer.dependencies {
        let pkg_key = format!("{name}@{}", entry.version);
        let integrity = packages
            .get(&pkg_key)
            .and_then(|p| p.resolution.integrity.clone());
        deps.entry(name.clone()).or_insert(PackageInfo {
            name: name.clone(),
            version: entry.version.clone(),
            integrity,
        });
    }
    for (name, entry) in &importer.dev_dependencies {
        let pkg_key = format!("{name}@{}", entry.version);
        let integrity = packages
            .get(&pkg_key)
            .and_then(|p| p.resolution.integrity.clone());
        dev_deps.entry(name.clone()).or_insert(PackageInfo {
            name: name.clone(),
            version: entry.version.clone(),
            integrity,
        });
    }
}

/// Parse pnpm-lock.yaml for a specific workspace importer key
pub fn parse_for_workspace(
    content: &str,
    importer_key: &str,
) -> Result<DependencyGraph, LockpickError> {
    let lockfile: PnpmLockfile = serde_yml::from_str(content)
        .map_err(|e| LockpickError::Parse(format!("Failed to parse pnpm-lock.yaml: {e}")))?;

    let importer = lockfile.importers.get(importer_key).ok_or_else(|| {
        LockpickError::Parse(format!("Importer '{importer_key}' not found in pnpm-lock.yaml"))
    })?;

    let mut deps = HashMap::new();
    let mut dev_deps = HashMap::new();
    collect_importer_deps(importer, &lockfile.packages, &mut deps, &mut dev_deps);

    let all_packages = build_all_packages(&lockfile.packages);

    Ok(DependencyGraph {
        dependencies: deps,
        dev_dependencies: dev_deps,
        lockfile_type: crate::LockfileType::Pnpm,
        all_packages,
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
