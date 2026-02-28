use crate::config::types::Thresholds;
use crate::{AnalysisResult, Severity};

/// Returns true if any threshold is exceeded.
pub fn evaluate(result: &AnalysisResult, thresholds: &Thresholds) -> bool {
    check_vulns(result, thresholds)
        || check_unused(result, thresholds)
        || check_duplicates(result, thresholds)
        || check_license(result, thresholds)
        || check_supply_chain(result, thresholds)
}

/// Create Thresholds from --fail-on preset level
pub fn from_fail_on(level: &str) -> Thresholds {
    match level {
        "critical" => Thresholds {
            max_critical: 0,
            ..Default::default()
        },
        "high" => Thresholds {
            max_critical: 0,
            max_high: 0,
            ..Default::default()
        },
        "any" => Thresholds {
            max_critical: 0,
            max_high: 0,
            max_unused: 0,
            max_duplicates: 0,
            fail_on_license: true,
            max_supply_chain_high: 0,
        },
        _ => Thresholds::default(),
    }
}

fn exceeds(count: usize, limit: i32) -> bool {
    limit >= 0 && count as i32 > limit
}

fn check_vulns(result: &AnalysisResult, t: &Thresholds) -> bool {
    let Some(ref vulns) = result.vulns else {
        return false;
    };
    let (mut critical, mut high) = (0usize, 0usize);
    for vr in vulns {
        for v in &vr.vulns {
            match v.severity {
                Severity::Critical => critical += 1,
                Severity::High => high += 1,
                _ => {}
            }
        }
    }
    exceeds(critical, t.max_critical) || exceeds(high, t.max_high)
}

fn check_unused(result: &AnalysisResult, t: &Thresholds) -> bool {
    let Some(ref unused) = result.unused else {
        return false;
    };
    exceeds(unused.unused.len(), t.max_unused)
}

fn check_duplicates(result: &AnalysisResult, t: &Thresholds) -> bool {
    let Some(ref dups) = result.duplicates else {
        return false;
    };
    exceeds(dups.total_duplicate_packages, t.max_duplicates)
}

fn check_license(result: &AnalysisResult, t: &Thresholds) -> bool {
    if !t.fail_on_license {
        return false;
    }
    let Some(ref license) = result.license else {
        return false;
    };
    !license.violations.is_empty()
}

fn check_supply_chain(result: &AnalysisResult, t: &Thresholds) -> bool {
    let Some(ref sc) = result.supply_chain else {
        return false;
    };
    let high_count = sc
        .risks
        .iter()
        .filter(|r| matches!(r.severity, Severity::High | Severity::Critical))
        .count();
    exceeds(high_count, t.max_supply_chain_high)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnalysisResult, DepType, UnusedDep, UnusedReport, VulnReport, Vulnerability};

    fn empty_result() -> AnalysisResult {
        AnalysisResult {
            unused: None,
            vulns: None,
            duplicates: None,
            size: None,
            license: None,
            outdated: None,
            supply_chain: None,
        }
    }

    #[test]
    fn test_default_thresholds_never_fail() {
        let mut r = empty_result();
        r.vulns = Some(vec![VulnReport {
            package: "x".into(),
            version: "1.0.0".into(),
            vulns: vec![Vulnerability {
                id: "V1".into(),
                summary: "t".into(),
                severity: Severity::Critical,
                fixed_version: None,
            }],
        }]);
        assert!(!evaluate(&r, &Thresholds::default()));
    }

    #[test]
    fn test_critical_threshold_exceeded() {
        let mut r = empty_result();
        r.vulns = Some(vec![VulnReport {
            package: "x".into(),
            version: "1.0.0".into(),
            vulns: vec![Vulnerability {
                id: "V1".into(),
                summary: "t".into(),
                severity: Severity::Critical,
                fixed_version: None,
            }],
        }]);
        let t = Thresholds {
            max_critical: 0,
            ..Default::default()
        };
        assert!(evaluate(&r, &t));
    }

    #[test]
    fn test_unused_threshold() {
        let mut r = empty_result();
        r.unused = Some(UnusedReport {
            unused: vec![UnusedDep {
                name: "a".into(),
                version: "1.0.0".into(),
                dep_type: DepType::Prod,
            }],
        });
        let t = Thresholds {
            max_unused: 0,
            ..Default::default()
        };
        assert!(evaluate(&r, &t));
    }

    #[test]
    fn test_from_fail_on_presets() {
        let t = from_fail_on("critical");
        assert_eq!(t.max_critical, 0);
        assert_eq!(t.max_high, -1);

        let t = from_fail_on("high");
        assert_eq!(t.max_critical, 0);
        assert_eq!(t.max_high, 0);

        let t = from_fail_on("any");
        assert_eq!(t.max_unused, 0);
        assert!(t.fail_on_license);
    }

    #[test]
    fn test_empty_result_never_fails() {
        let r = empty_result();
        let t = from_fail_on("any");
        assert!(!evaluate(&r, &t));
    }
}
