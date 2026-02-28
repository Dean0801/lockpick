pub mod typosquat;

use crate::{DependencyGraph, Severity, SupplyChainReport, SupplyChainRisk, SupplyChainRiskType};

/// Official scopes that should not trigger scope confusion alerts
const OFFICIAL_SCOPES: &[&str] = &[
    "@types",
    "@babel",
    "@eslint",
    "@typescript-eslint",
    "@angular",
    "@vue",
    "@nuxt",
    "@nestjs",
    "@react-native",
    "@testing-library",
    "@emotion",
    "@mui",
    "@chakra-ui",
    "@radix-ui",
    "@headlessui",
    "@tanstack",
    "@trpc",
    "@prisma",
    "@aws-sdk",
    "@azure",
    "@google-cloud",
    "@grpc",
    "@opentelemetry",
    "@sentry",
];

/// Check if a scoped package impersonates a popular unscoped package.
fn check_scope_confusion(name: &str) -> Option<String> {
    let rest = name.strip_prefix('@')?;
    let (scope, pkg) = rest.split_once('/')?;
    let full_scope = format!("@{scope}");
    if OFFICIAL_SCOPES.contains(&full_scope.as_str()) {
        return None;
    }
    if typosquat::POPULAR_PACKAGES.contains(&pkg) {
        Some(pkg.to_string())
    } else {
        None
    }
}

/// Check if installed version's major is abnormally high.
fn check_version_anomaly(_name: &str, version: &str) -> bool {
    let major: u64 = version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    major >= 50
}

/// Run all supply chain checks against the dependency graph.
pub fn analyze(graph: &DependencyGraph) -> SupplyChainReport {
    let mut risks = Vec::new();

    let all_deps = graph
        .dependencies
        .iter()
        .chain(graph.dev_dependencies.iter());

    for (name, info) in all_deps {
        // Typosquatting
        if let Some((similar, distance)) = typosquat::check_typosquat(name) {
            risks.push(SupplyChainRisk {
                package: name.clone(),
                version: info.version.clone(),
                risk_type: SupplyChainRiskType::Typosquat {
                    similar_to: similar,
                    distance,
                },
                severity: Severity::High,
            });
        }

        // Scope confusion
        if let Some(legitimate) = check_scope_confusion(name) {
            risks.push(SupplyChainRisk {
                package: name.clone(),
                version: info.version.clone(),
                risk_type: SupplyChainRiskType::ScopeConfusion { legitimate },
                severity: Severity::Medium,
            });
        }

        // Version anomaly
        if check_version_anomaly(name, &info.version) {
            risks.push(SupplyChainRisk {
                package: name.clone(),
                version: info.version.clone(),
                risk_type: SupplyChainRiskType::VersionAnomaly {
                    installed_version: info.version.clone(),
                },
                severity: Severity::High,
            });
        }
    }

    SupplyChainReport { risks }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LockfileType;
    use std::collections::HashMap;

    #[test]
    fn test_scope_confusion_evil_express() {
        let result = check_scope_confusion("@evil/express");
        assert_eq!(result, Some("express".into()));
    }

    #[test]
    fn test_scope_confusion_official_types() {
        assert!(check_scope_confusion("@types/node").is_none());
    }

    #[test]
    fn test_version_anomaly_high_major() {
        assert!(check_version_anomaly("lodash", "99.0.0"));
    }

    #[test]
    fn test_version_anomaly_normal() {
        assert!(!check_version_anomaly("lodash", "4.17.21"));
    }

    #[test]
    fn test_analyze_empty_graph() {
        let graph = DependencyGraph {
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            lockfile_type: LockfileType::Npm,
            all_packages: HashMap::new(),
            dep_edges: HashMap::new(),
        };
        let report = analyze(&graph);
        assert!(report.risks.is_empty());
    }
}
