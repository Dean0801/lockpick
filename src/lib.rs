pub mod audit;
pub mod i18n;
pub mod lockfile;
pub mod report;
pub mod scanner;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}
