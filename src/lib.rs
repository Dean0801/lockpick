pub mod analyze;
pub mod analyzer;
pub mod audit;
pub mod cache;
pub mod config;
pub mod diff;
pub mod error;
pub mod fix;
pub mod i18n;
pub mod lockfile;
pub mod outdated;
pub mod report;
pub mod runner;
pub mod scanner;
pub mod supply_chain;
pub mod threshold;
pub mod tree;
pub mod utils;
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

/// Dependency edge: package A depends on package B
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepEdge {
    pub name: String,
    pub version: String,
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
    /// Transitive dependency edges: key = "pkg@version", value = direct deps.
    /// Only populated when needed (e.g. tree command). Empty by default.
    #[serde(default)]
    pub dep_edges: HashMap<String, Vec<DepEdge>>,
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

/// Semver update level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemverLevel {
    Patch,
    Minor,
    Major,
}

impl std::fmt::Display for SemverLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemverLevel::Patch => write!(f, "patch"),
            SemverLevel::Minor => write!(f, "minor"),
            SemverLevel::Major => write!(f, "major"),
        }
    }
}

/// Upgrade priority (combines outdated + vuln info)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum UpgradePriority {
    Low,
    Medium,
    High,
    Critical,
}

/// A single outdated dependency entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutdatedEntry {
    pub name: String,
    pub current: String,
    pub latest: String,
    pub level: SemverLevel,
    pub dep_type: DepType,
    pub priority: UpgradePriority,
    pub vuln_ids: Vec<String>,
}

/// Outdated dependencies report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutdatedReport {
    pub entries: Vec<OutdatedEntry>,
    pub total_outdated: usize,
    pub patch_count: usize,
    pub minor_count: usize,
    pub major_count: usize,
}

/// Type of supply chain risk detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupplyChainRiskType {
    Typosquat { similar_to: String, distance: usize },
    ScopeConfusion { legitimate: String },
    VersionAnomaly { installed_version: String },
}

/// A single supply chain risk finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainRisk {
    pub package: String,
    pub version: String,
    pub risk_type: SupplyChainRiskType,
    pub severity: Severity,
}

/// Supply chain analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainReport {
    pub risks: Vec<SupplyChainRisk>,
}

/// Combined analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub unused: Option<UnusedReport>,
    pub vulns: Option<Vec<VulnReport>>,
    pub duplicates: Option<DuplicateReport>,
    pub size: Option<SizeReport>,
    pub license: Option<LicenseReport>,
    #[serde(default)]
    pub outdated: Option<OutdatedReport>,
    #[serde(default)]
    pub supply_chain: Option<SupplyChainReport>,
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
            dep_edges: HashMap::new(),
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
            outdated: None,
            supply_chain: None,
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

    #[test]
    fn test_dep_edge_struct() {
        let edge = DepEdge {
            name: "lodash".into(),
            version: "4.17.21".into(),
        };
        assert_eq!(edge.name, "lodash");
        assert_eq!(edge.version, "4.17.21");
    }

    #[test]
    fn test_dependency_graph_dep_edges_serde_default() {
        // Old JSON without dep_edges should deserialize with empty dep_edges
        let json =
            r#"{"dependencies":{},"dev_dependencies":{},"lockfile_type":"Pnpm","all_packages":{}}"#;
        let graph: DependencyGraph = serde_json::from_str(json).unwrap();
        assert!(graph.dep_edges.is_empty());
    }

    #[test]
    fn test_outdated_entry_struct() {
        let entry = OutdatedEntry {
            name: "lodash".into(),
            current: "4.17.20".into(),
            latest: "4.17.21".into(),
            level: SemverLevel::Patch,
            dep_type: DepType::Prod,
            priority: UpgradePriority::Low,
            vuln_ids: vec![],
        };
        assert_eq!(entry.name, "lodash");
        assert_eq!(entry.level, SemverLevel::Patch);
        assert_eq!(entry.priority, UpgradePriority::Low);
    }

    #[test]
    fn test_supply_chain_risk_struct() {
        let risk = SupplyChainRisk {
            package: "lod-ash".into(),
            version: "4.17.21".into(),
            risk_type: SupplyChainRiskType::Typosquat {
                similar_to: "lodash".into(),
                distance: 1,
            },
            severity: Severity::High,
        };
        assert_eq!(risk.package, "lod-ash");
        matches!(risk.risk_type, SupplyChainRiskType::Typosquat { .. });
    }

    #[test]
    fn test_analysis_result_serde_default_new_fields() {
        // Old JSON without outdated/supply_chain should deserialize fine
        let json = r#"{"unused":null,"vulns":null,"duplicates":null,"size":null,"license":null}"#;
        let result: AnalysisResult = serde_json::from_str(json).unwrap();
        assert!(result.outdated.is_none());
        assert!(result.supply_chain.is_none());
    }
}
