pub mod analyze;
pub mod runner;
pub mod utils;
pub mod analyzer;
pub mod audit;
pub mod cache;
pub mod config;
pub mod error;
pub mod fix;
pub mod i18n;
pub mod lockfile;
pub mod report;
pub mod scanner;
pub mod workspace;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lockfile type identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LockfileType {
    Pnpm,
    Npm,
    Yarn,
    Bun,
}

/// A duplicate dependency entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateDep {
    pub name: String,
    pub versions: Vec<String>,
}

/// Duplicate dependencies report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateReport {
    pub duplicates: Vec<DuplicateDep>,
    pub total_duplicate_packages: usize,
}

/// A single size entry for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeEntry {
    pub name: String,
    pub size_bytes: u64,
}

/// Size analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeReport {
    pub entries: Vec<SizeEntry>,
    pub total_bytes: u64,
}

/// License entry for a single package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseEntry {
    pub name: String,
    pub version: String,
    pub license: String,
    pub dep_type: DepType,
}

/// Reason for license policy violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationReason {
    Denied,
    NotAllowed,
    Unknown,
}

/// A single license violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseViolation {
    pub package: String,
    pub version: String,
    pub license: String,
    pub reason: ViolationReason,
}

/// License analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseReport {
    pub entries: Vec<LicenseEntry>,
    pub violations: Vec<LicenseViolation>,
}

/// Package metadata from lockfile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub integrity: Option<String>,
}

/// Parsed dependency graph from lockfile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub dependencies: HashMap<String, PackageInfo>,
    pub dev_dependencies: HashMap<String, PackageInfo>,
    pub lockfile_type: LockfileType,
    pub all_packages: HashMap<String, Vec<String>>,
}

/// Vulnerability severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// A single vulnerability entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub summary: String,
    pub severity: Severity,
    pub fixed_version: Option<String>,
}

/// Vulnerability report for a single package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnReport {
    pub package: String,
    pub version: String,
    pub vulns: Vec<Vulnerability>,
}

/// Dependency type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DepType {
    Prod,
    Dev,
}

/// A single unused dependency entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedDep {
    pub name: String,
    pub version: String,
    pub dep_type: DepType,
}

/// Unused dependencies report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedReport {
    pub unused: Vec<UnusedDep>,
}

/// Combined analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub unused: Option<UnusedReport>,
    pub vulns: Option<Vec<VulnReport>>,
    pub duplicates: Option<DuplicateReport>,
    pub size: Option<SizeReport>,
    pub license: Option<LicenseReport>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_type_enum_variants() {
        let pnpm = LockfileType::Pnpm;
        let npm = LockfileType::Npm;
        let yarn = LockfileType::Yarn;
        assert_eq!(pnpm, LockfileType::Pnpm);
        assert_eq!(npm, LockfileType::Npm);
        assert_eq!(yarn, LockfileType::Yarn);
        // Should be cloneable
        let cloned = pnpm.clone();
        assert_eq!(cloned, LockfileType::Pnpm);
    }

    #[test]
    fn test_dependency_graph_has_new_fields() {
        let graph = DependencyGraph {
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            lockfile_type: LockfileType::Pnpm,
            all_packages: HashMap::new(),
        };
        assert_eq!(graph.lockfile_type, LockfileType::Pnpm);
        assert!(graph.all_packages.is_empty());
    }

    #[test]
    fn test_analysis_result_has_duplicates_and_size() {
        let result = AnalysisResult {
            unused: None,
            vulns: None,
            duplicates: None,
            size: None,
            license: None,
        };
        assert!(result.duplicates.is_none());
        assert!(result.size.is_none());
    }

    #[test]
    fn test_duplicate_dep_struct() {
        let dup = DuplicateDep {
            name: "lodash".into(),
            versions: vec!["4.17.20".into(), "4.17.21".into()],
        };
        assert_eq!(dup.name, "lodash");
        assert_eq!(dup.versions.len(), 2);
    }

    #[test]
    fn test_duplicate_report_struct() {
        let report = DuplicateReport {
            duplicates: vec![DuplicateDep {
                name: "lodash".into(),
                versions: vec!["4.17.20".into(), "4.17.21".into()],
            }],
            total_duplicate_packages: 1,
        };
        assert_eq!(report.duplicates.len(), 1);
        assert_eq!(report.total_duplicate_packages, 1);
    }

    #[test]
    fn test_size_entry_struct() {
        let entry = SizeEntry {
            name: "react".into(),
            size_bytes: 12345,
        };
        assert_eq!(entry.name, "react");
        assert_eq!(entry.size_bytes, 12345);
    }

    #[test]
    fn test_size_report_struct() {
        let report = SizeReport {
            entries: vec![SizeEntry {
                name: "react".into(),
                size_bytes: 12345,
            }],
            total_bytes: 12345,
        };
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.total_bytes, 12345);
    }

    #[test]
    fn test_lockfile_type_serde_roundtrip() {
        let lt = LockfileType::Npm;
        let json = serde_json::to_string(&lt).unwrap();
        let deserialized: LockfileType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LockfileType::Npm);
    }
}
