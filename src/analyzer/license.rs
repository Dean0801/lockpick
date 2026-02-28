use crate::config::types::LicensePolicy;
use crate::{DepType, DependencyGraph, LicenseEntry, LicenseReport, LicenseViolation, ViolationReason};
use serde_json::Value;
use std::path::Path;

/// Normalize common license name aliases to their canonical SPDX form.
fn normalize_license(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed {
        "Apache 2.0" | "Apache-2" => "Apache-2.0".to_string(),
        "BSD" => "BSD-2-Clause".to_string(),
        _ => trimmed.to_string(),
    }
}

/// Read the license field from a package.json value, handling multiple formats.
fn extract_license_from_json(pkg_json: &Value) -> String {
    // Format 1: "license": "MIT"
    if let Some(license_str) = pkg_json.get("license").and_then(|v| v.as_str()) {
        return normalize_license(license_str);
    }

    // Format 2: "license": { "type": "MIT" }
    if let Some(license_obj) = pkg_json.get("license").and_then(|v| v.as_object())
        && let Some(t) = license_obj.get("type").and_then(|v| v.as_str())
    {
        return normalize_license(t);
    }

    // Format 3: "licenses": [{ "type": "MIT" }]
    if let Some(licenses_arr) = pkg_json.get("licenses").and_then(|v| v.as_array())
        && let Some(first) = licenses_arr.first()
        && let Some(t) = first.get("type").and_then(|v| v.as_str())
    {
        return normalize_license(t);
    }

    "UNKNOWN".to_string()
}

/// Extract license information for all dependencies in the graph.
///
/// Reads each package's `package.json` from `node_modules` to determine its license.
/// If `skip_dev` is true, dev dependencies are excluded.
pub fn extract_licenses(
    project_path: &Path,
    graph: &DependencyGraph,
    skip_dev: bool,
) -> LicenseReport {
    let node_modules = project_path.join("node_modules");
    let mut entries = Vec::new();

    for (name, info) in &graph.dependencies {
        let license = read_package_license(&node_modules, name);
        entries.push(LicenseEntry {
            name: name.clone(),
            version: info.version.clone(),
            license,
            dep_type: DepType::Prod,
        });
    }

    if !skip_dev {
        for (name, info) in &graph.dev_dependencies {
            let license = read_package_license(&node_modules, name);
            entries.push(LicenseEntry {
                name: name.clone(),
                version: info.version.clone(),
                license,
                dep_type: DepType::Dev,
            });
        }
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));

    LicenseReport {
        entries,
        violations: vec![],
    }
}

/// Check license entries against a policy and return any violations.
///
/// - If `policy.allow` is non-empty: any license NOT in the allow list is `NotAllowed`.
/// - If `policy.deny` is non-empty (and allow is empty): any license IN the deny list is `Denied`.
/// - `"UNKNOWN"` always produces an `Unknown` violation unless explicitly in the allow list.
pub fn check_policy(
    report: &LicenseReport,
    policy: &LicensePolicy,
) -> Vec<LicenseViolation> {
    let mut violations = Vec::new();

    for entry in &report.entries {
        if !policy.allow.is_empty() {
            // Allowlist mode
            if !policy.allow.contains(&entry.license) {
                let reason = if entry.license == "UNKNOWN" {
                    ViolationReason::Unknown
                } else {
                    ViolationReason::NotAllowed
                };
                violations.push(LicenseViolation {
                    package: entry.name.clone(),
                    version: entry.version.clone(),
                    license: entry.license.clone(),
                    reason,
                });
            }
        } else if !policy.deny.is_empty() {
            // Denylist mode
            if policy.deny.contains(&entry.license) {
                violations.push(LicenseViolation {
                    package: entry.name.clone(),
                    version: entry.version.clone(),
                    license: entry.license.clone(),
                    reason: ViolationReason::Denied,
                });
            }
            // UNKNOWN always violates in denylist mode too
            if entry.license == "UNKNOWN" && !policy.deny.contains(&"UNKNOWN".to_string()) {
                violations.push(LicenseViolation {
                    package: entry.name.clone(),
                    version: entry.version.clone(),
                    license: entry.license.clone(),
                    reason: ViolationReason::Unknown,
                });
            }
        } else {
            // No policy — UNKNOWN still violates
            if entry.license == "UNKNOWN" {
                violations.push(LicenseViolation {
                    package: entry.name.clone(),
                    version: entry.version.clone(),
                    license: entry.license.clone(),
                    reason: ViolationReason::Unknown,
                });
            }
        }
    }

    violations
}

/// Read the license from a package's package.json in node_modules.
fn read_package_license(node_modules: &Path, name: &str) -> String {
    let pkg_json_path = node_modules.join(name).join("package.json");
    match std::fs::read_to_string(&pkg_json_path) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(json) => extract_license_from_json(&json),
            Err(_) => "UNKNOWN".to_string(),
        },
        Err(_) => "UNKNOWN".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DependencyGraph, LockfileType, PackageInfo};
    use std::collections::HashMap;
    use std::fs;

    fn make_graph(
        deps: Vec<(&str, &str)>,
        dev_deps: Vec<(&str, &str)>,
    ) -> DependencyGraph {
        let mut dependencies = HashMap::new();
        for (name, version) in deps {
            dependencies.insert(
                name.to_string(),
                PackageInfo {
                    name: name.to_string(),
                    version: version.to_string(),
                    integrity: None,
                },
            );
        }
        let mut dev_dependencies = HashMap::new();
        for (name, version) in dev_deps {
            dev_dependencies.insert(
                name.to_string(),
                PackageInfo {
                    name: name.to_string(),
                    version: version.to_string(),
                    integrity: None,
                },
            );
        }
        DependencyGraph {
            dependencies,
            dev_dependencies,
            lockfile_type: LockfileType::Npm,
            all_packages: HashMap::new(),
            dep_edges: HashMap::new(),
        }
    }

    fn write_pkg_json(root: &Path, name: &str, content: &str) {
        let pkg_dir = root.join("node_modules").join(name);
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("package.json"), content).unwrap();
    }

    #[test]
    fn test_string_license_extraction() {
        let tmp = tempfile::tempdir().unwrap();
        write_pkg_json(tmp.path(), "react", r#"{"name":"react","license":"MIT"}"#);
        let graph = make_graph(vec![("react", "18.2.0")], vec![]);
        let report = extract_licenses(tmp.path(), &graph, false);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].license, "MIT");
        assert_eq!(report.entries[0].dep_type, DepType::Prod);
    }

    #[test]
    fn test_object_license_format() {
        let tmp = tempfile::tempdir().unwrap();
        write_pkg_json(tmp.path(), "old-pkg", r#"{"name":"old-pkg","license":{"type":"ISC"}}"#);
        let graph = make_graph(vec![("old-pkg", "1.0.0")], vec![]);
        let report = extract_licenses(tmp.path(), &graph, false);
        assert_eq!(report.entries[0].license, "ISC");
    }

    #[test]
    fn test_legacy_array_license_format() {
        let tmp = tempfile::tempdir().unwrap();
        write_pkg_json(tmp.path(), "ancient-pkg", r#"{"name":"ancient-pkg","licenses":[{"type":"MIT"},{"type":"GPL-2.0"}]}"#);
        let graph = make_graph(vec![("ancient-pkg", "0.1.0")], vec![]);
        let report = extract_licenses(tmp.path(), &graph, false);
        assert_eq!(report.entries[0].license, "MIT");
    }

    #[test]
    fn test_unknown_when_no_license_field() {
        let tmp = tempfile::tempdir().unwrap();
        write_pkg_json(tmp.path(), "no-license", r#"{"name":"no-license"}"#);
        let graph = make_graph(vec![("no-license", "1.0.0")], vec![]);
        let report = extract_licenses(tmp.path(), &graph, false);
        assert_eq!(report.entries[0].license, "UNKNOWN");
    }

    #[test]
    fn test_normalization_apache() {
        let tmp = tempfile::tempdir().unwrap();
        write_pkg_json(tmp.path(), "pkg-a", r#"{"name":"pkg-a","license":"Apache 2.0"}"#);
        write_pkg_json(tmp.path(), "pkg-b", r#"{"name":"pkg-b","license":"Apache-2"}"#);
        let graph = make_graph(vec![("pkg-a", "1.0.0"), ("pkg-b", "2.0.0")], vec![]);
        let report = extract_licenses(tmp.path(), &graph, false);
        for entry in &report.entries {
            assert_eq!(entry.license, "Apache-2.0", "failed for {}", entry.name);
        }
    }

    #[test]
    fn test_skip_dev_dependencies() {
        let tmp = tempfile::tempdir().unwrap();
        write_pkg_json(tmp.path(), "react", r#"{"name":"react","license":"MIT"}"#);
        write_pkg_json(tmp.path(), "jest", r#"{"name":"jest","license":"MIT"}"#);
        let graph = make_graph(vec![("react", "18.2.0")], vec![("jest", "29.0.0")]);
        let report_with_dev = extract_licenses(tmp.path(), &graph, false);
        assert_eq!(report_with_dev.entries.len(), 2);
        let report_skip_dev = extract_licenses(tmp.path(), &graph, true);
        assert_eq!(report_skip_dev.entries.len(), 1);
        assert_eq!(report_skip_dev.entries[0].name, "react");
    }

    #[test]
    fn test_check_policy_allow_list() {
        let report = LicenseReport {
            entries: vec![
                LicenseEntry { name: "a".into(), version: "1.0.0".into(), license: "MIT".into(), dep_type: DepType::Prod },
                LicenseEntry { name: "b".into(), version: "1.0.0".into(), license: "GPL-3.0".into(), dep_type: DepType::Prod },
            ],
            violations: vec![],
        };
        let policy = LicensePolicy { allow: vec!["MIT".into(), "ISC".into()], deny: vec![] };
        let violations = check_policy(&report, &policy);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].package, "b");
    }

    #[test]
    fn test_check_policy_deny_list() {
        let report = LicenseReport {
            entries: vec![
                LicenseEntry { name: "a".into(), version: "1.0.0".into(), license: "MIT".into(), dep_type: DepType::Prod },
                LicenseEntry { name: "b".into(), version: "1.0.0".into(), license: "GPL-3.0".into(), dep_type: DepType::Prod },
            ],
            violations: vec![],
        };
        let policy = LicensePolicy { allow: vec![], deny: vec!["GPL-3.0".into()] };
        let violations = check_policy(&report, &policy);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].package, "b");
    }

    #[test]
    fn test_unknown_always_violates_unless_allowed() {
        let report = LicenseReport {
            entries: vec![LicenseEntry {
                name: "mystery".into(), version: "1.0.0".into(),
                license: "UNKNOWN".into(), dep_type: DepType::Prod,
            }],
            violations: vec![],
        };
        let policy_empty = LicensePolicy { allow: vec![], deny: vec![] };
        let v = check_policy(&report, &policy_empty);
        assert_eq!(v.len(), 1);

        let policy_allow_unknown = LicensePolicy { allow: vec!["UNKNOWN".into(), "MIT".into()], deny: vec![] };
        let v = check_policy(&report, &policy_allow_unknown);
        assert!(v.is_empty());
    }
}
