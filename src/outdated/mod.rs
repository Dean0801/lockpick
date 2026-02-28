pub mod registry;

use std::collections::HashMap;

use crate::error::LockpickError;
use crate::{
    DepType, DependencyGraph, OutdatedEntry, OutdatedReport, SemverLevel, Severity,
    UpgradePriority, VulnReport,
};

/// Parse "major.minor.patch" from version string (ignores pre-release and build metadata)
fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.split('+').next()?; // strip build metadata
    let v = v.split('-').next()?; // strip pre-release
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Compare current vs latest, return SemverLevel. None if same or unparseable.
fn compare_versions(current: &str, latest: &str) -> Option<SemverLevel> {
    let (cmaj, cmin, cpat) = parse_semver(current)?;
    let (lmaj, lmin, lpat) = parse_semver(latest)?;
    if (cmaj, cmin, cpat) >= (lmaj, lmin, lpat) {
        return None; // not outdated
    }
    Some(if cmaj < lmaj {
        SemverLevel::Major
    } else if cmin < lmin {
        SemverLevel::Minor
    } else {
        SemverLevel::Patch
    })
}

/// Compute upgrade priority based on vuln data
fn compute_priority(name: &str, vulns: Option<&[VulnReport]>) -> (UpgradePriority, Vec<String>) {
    let Some(vulns) = vulns else {
        return (UpgradePriority::Low, vec![]);
    };
    let mut ids = Vec::new();
    let mut max_sev = None::<&Severity>;

    for vr in vulns {
        if vr.package != name {
            continue;
        }
        for v in &vr.vulns {
            ids.push(v.id.clone());
            if max_sev.is_none_or(|s| severity_rank(&v.severity) > severity_rank(s)) {
                max_sev = Some(&v.severity);
            }
        }
    }

    let priority = match max_sev {
        Some(Severity::Critical) => UpgradePriority::Critical,
        Some(Severity::High) => UpgradePriority::High,
        Some(Severity::Medium | Severity::Low) => UpgradePriority::Medium,
        None => UpgradePriority::Low,
    };
    (priority, ids)
}

fn severity_rank(s: &Severity) -> u8 {
    match s {
        Severity::Low => 1,
        Severity::Medium => 2,
        Severity::High => 3,
        Severity::Critical => 4,
    }
}

/// Check all dependencies for available updates.
pub async fn check_outdated(
    graph: &DependencyGraph,
    vulns: Option<&[VulnReport]>,
    skip_dev: bool,
    registry_url: Option<&str>,
    no_cache: bool,
    cache_ttl: Option<u64>,
) -> Result<OutdatedReport, LockpickError> {
    // Collect packages
    let mut packages: Vec<(String, String, DepType)> = graph
        .dependencies
        .iter()
        .map(|(n, i)| (n.clone(), i.version.clone(), DepType::Prod))
        .collect();

    if !skip_dev {
        packages.extend(
            graph
                .dev_dependencies
                .iter()
                .map(|(n, i)| (n.clone(), i.version.clone(), DepType::Dev)),
        );
    }

    let pkg_pairs: Vec<(String, String)> = packages
        .iter()
        .map(|(n, v, _)| (n.clone(), v.clone()))
        .collect();

    let latest_map =
        registry::fetch_latest_versions(&pkg_pairs, registry_url, no_cache, cache_ttl).await?;

    // Build dep_type lookup
    let dep_types: HashMap<&str, &DepType> =
        packages.iter().map(|(n, _, dt)| (n.as_str(), dt)).collect();

    let mut entries = Vec::new();
    for (name, current, _) in &packages {
        let Some(latest) = latest_map.get(name) else {
            continue;
        };
        let Some(level) = compare_versions(current, latest) else {
            continue;
        };
        let (priority, vuln_ids) = compute_priority(name, vulns);
        entries.push(OutdatedEntry {
            name: name.clone(),
            current: current.clone(),
            latest: latest.clone(),
            level,
            dep_type: dep_types[name.as_str()].clone(),
            priority,
            vuln_ids,
        });
    }

    // Sort by priority descending
    entries.sort_by(|a, b| b.priority.cmp(&a.priority));

    let (mut patch_count, mut minor_count, mut major_count) = (0, 0, 0);
    for e in &entries {
        match e.level {
            SemverLevel::Patch => patch_count += 1,
            SemverLevel::Minor => minor_count += 1,
            SemverLevel::Major => major_count += 1,
        }
    }

    Ok(OutdatedReport {
        total_outdated: entries.len(),
        entries,
        patch_count,
        minor_count,
        major_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver_normal() {
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn test_parse_semver_prerelease() {
        assert_eq!(parse_semver("1.0.0-beta.1"), Some((1, 0, 0)));
    }

    #[test]
    fn test_compare_patch() {
        assert_eq!(compare_versions("1.2.3", "1.2.4"), Some(SemverLevel::Patch));
    }

    #[test]
    fn test_compare_minor() {
        assert_eq!(compare_versions("1.2.3", "1.3.0"), Some(SemverLevel::Minor));
    }

    #[test]
    fn test_compare_major() {
        assert_eq!(compare_versions("1.2.3", "2.0.0"), Some(SemverLevel::Major));
    }

    #[test]
    fn test_compare_same_version() {
        assert_eq!(compare_versions("1.2.3", "1.2.3"), None);
    }

    #[test]
    fn test_compute_priority_no_vulns() {
        let (p, ids) = compute_priority("lodash", None);
        assert_eq!(p, UpgradePriority::Low);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_compute_priority_critical() {
        let vulns = vec![VulnReport {
            package: "semver".into(),
            version: "7.5.2".into(),
            vulns: vec![crate::Vulnerability {
                id: "GHSA-1234".into(),
                summary: "test".into(),
                severity: Severity::Critical,
                fixed_version: Some("7.5.4".into()),
            }],
        }];
        let (p, ids) = compute_priority("semver", Some(&vulns));
        assert_eq!(p, UpgradePriority::Critical);
        assert_eq!(ids, vec!["GHSA-1234"]);
    }
}
