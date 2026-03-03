use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::error::LockpickError;
use crate::i18n::I18n;
use crate::{AnalysisResult, UnusedDep};

/// Generic diff section: added vs removed items
#[derive(Debug, Clone, Serialize)]
pub struct DiffSection<T> {
    pub added: Vec<T>,
    pub removed: Vec<T>,
}

/// Diff summary statistics
#[derive(Debug, Clone, Serialize)]
pub struct DiffSummary {
    pub baseline_issues: usize,
    pub current_issues: usize,
    pub new_issues: usize,
    pub resolved_issues: usize,
}

/// Complete diff report
#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    pub unused: DiffSection<UnusedDep>,
    pub vulns: DiffSection<String>,
    pub duplicates: DiffSection<String>,
    pub license_violations: DiffSection<String>,
    pub outdated: DiffSection<String>,
    pub supply_chain: DiffSection<String>,
    pub summary: DiffSummary,
}

/// Load baseline JSON and compute diff against current result.
pub fn compute_diff(
    baseline_path: &Path,
    current: &AnalysisResult,
) -> Result<DiffReport, LockpickError> {
    let content = std::fs::read_to_string(baseline_path).map_err(LockpickError::Io)?;
    let baseline: AnalysisResult = serde_json::from_str(&content)
        .map_err(|e| LockpickError::Parse(format!("Failed to parse baseline JSON: {e}")))?;

    let unused = diff_unused(&baseline, current);
    let vulns = diff_vulns(&baseline, current);
    let duplicates = diff_duplicates(&baseline, current);
    let license_violations = diff_licenses(&baseline, current);
    let outdated = diff_outdated(&baseline, current);
    let supply_chain = diff_supply_chain(&baseline, current);

    let new_issues = unused.added.len()
        + vulns.added.len()
        + duplicates.added.len()
        + license_violations.added.len()
        + outdated.added.len()
        + supply_chain.added.len();
    let resolved_issues = unused.removed.len()
        + vulns.removed.len()
        + duplicates.removed.len()
        + license_violations.removed.len()
        + outdated.removed.len()
        + supply_chain.removed.len();

    Ok(DiffReport {
        summary: DiffSummary {
            baseline_issues: count_issues(&baseline),
            current_issues: count_issues(current),
            new_issues,
            resolved_issues,
        },
        unused,
        vulns,
        duplicates,
        license_violations,
        outdated,
        supply_chain,
    })
}

/// Render diff report to terminal.
pub fn render_terminal(report: &DiffReport, i18n: &I18n) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\n{}: {} -> {} ({} {}, {} {})\n",
        i18n.t("diff_summary"),
        report.summary.baseline_issues,
        report.summary.current_issues,
        report.summary.new_issues,
        i18n.t("diff_new"),
        report.summary.resolved_issues,
        i18n.t("diff_resolved"),
    ));
    render_diff_section_terminal(
        &mut out,
        &report.unused.added,
        &report.unused.removed,
        i18n.t("unused_deps"),
    );
    render_diff_string_section(&mut out, &report.vulns, i18n.t("vulns"));
    render_diff_string_section(&mut out, &report.duplicates, i18n.t("duplicates"));
    render_diff_string_section(
        &mut out,
        &report.license_violations,
        i18n.t("license_violations"),
    );
    render_diff_string_section(&mut out, &report.outdated, i18n.t("outdated_deps"));
    render_diff_string_section(&mut out, &report.supply_chain, i18n.t("supply_chain_risks"));
    out
}

fn render_diff_section_terminal(
    out: &mut String,
    added: &[UnusedDep],
    removed: &[UnusedDep],
    label: &str,
) {
    if !added.is_empty() {
        out.push_str(&format!("\n+ {label} ({}):\n", added.len()));
        for dep in added {
            out.push_str(&format!("  + {} {}\n", dep.name, dep.version));
        }
    }
    if !removed.is_empty() {
        out.push_str(&format!("\n- {label} ({}):\n", removed.len()));
        for dep in removed {
            out.push_str(&format!("  - {} {}\n", dep.name, dep.version));
        }
    }
}

fn render_diff_string_section(out: &mut String, section: &DiffSection<String>, label: &str) {
    if !section.added.is_empty() {
        out.push_str(&format!("\n+ {label} ({}):\n", section.added.len()));
        for s in &section.added {
            out.push_str(&format!("  + {s}\n"));
        }
    }
    if !section.removed.is_empty() {
        out.push_str(&format!("\n- {label} ({}):\n", section.removed.len()));
        for s in &section.removed {
            out.push_str(&format!("  - {s}\n"));
        }
    }
}

/// Render diff report as Markdown.
pub fn render_markdown(report: &DiffReport, i18n: &I18n) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "# {} lockpick diff {}\n\n",
        "🔍",
        i18n.t("scan_complete")
    ));
    md.push_str("| | baseline | current | delta |\n");
    md.push_str("|------|------|------|------|\n");
    md.push_str(&format!(
        "| total | {} | {} | {} |\n\n",
        report.summary.baseline_issues,
        report.summary.current_issues,
        format_delta(report.summary.new_issues, report.summary.resolved_issues),
    ));
    if report.summary.new_issues > 0 {
        md.push_str(&format!(
            "## {} ({})\n\n",
            i18n.t("diff_new"),
            report.summary.new_issues
        ));
        for dep in &report.unused.added {
            md.push_str(&format!("- {} {}\n", dep.name, dep.version));
        }
        for s in &report.vulns.added {
            md.push_str(&format!("- {s}\n"));
        }
        for s in &report.duplicates.added {
            md.push_str(&format!("- {s}\n"));
        }
        for s in &report.license_violations.added {
            md.push_str(&format!("- {s}\n"));
        }
        for s in &report.outdated.added {
            md.push_str(&format!("- {s}\n"));
        }
        for s in &report.supply_chain.added {
            md.push_str(&format!("- {s}\n"));
        }
    }
    if report.summary.resolved_issues > 0 {
        md.push_str(&format!(
            "\n## {} ({})\n\n",
            i18n.t("diff_resolved"),
            report.summary.resolved_issues
        ));
        for dep in &report.unused.removed {
            md.push_str(&format!("- {} {}\n", dep.name, dep.version));
        }
        for s in &report.vulns.removed {
            md.push_str(&format!("- {s}\n"));
        }
        for s in &report.duplicates.removed {
            md.push_str(&format!("- {s}\n"));
        }
        for s in &report.license_violations.removed {
            md.push_str(&format!("- {s}\n"));
        }
        for s in &report.outdated.removed {
            md.push_str(&format!("- {s}\n"));
        }
        for s in &report.supply_chain.removed {
            md.push_str(&format!("- {s}\n"));
        }
    }
    md
}

fn diff_unused(baseline: &AnalysisResult, current: &AnalysisResult) -> DiffSection<UnusedDep> {
    let base_names: HashSet<String> = baseline
        .unused
        .as_ref()
        .map(|u| u.unused.iter().map(|d| d.name.clone()).collect())
        .unwrap_or_default();
    let curr_names: HashSet<String> = current
        .unused
        .as_ref()
        .map(|u| u.unused.iter().map(|d| d.name.clone()).collect())
        .unwrap_or_default();
    let added = current
        .unused
        .as_ref()
        .map(|u| {
            u.unused
                .iter()
                .filter(|d| !base_names.contains(&d.name))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let removed = baseline
        .unused
        .as_ref()
        .map(|u| {
            u.unused
                .iter()
                .filter(|d| !curr_names.contains(&d.name))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    DiffSection { added, removed }
}

fn diff_vulns(baseline: &AnalysisResult, current: &AnalysisResult) -> DiffSection<String> {
    let to_keys = |r: &AnalysisResult| -> HashSet<String> {
        r.vulns
            .as_ref()
            .map(|vs| {
                vs.iter()
                    .flat_map(|vr| {
                        vr.vulns
                            .iter()
                            .map(move |v| format!("{}@{}:{}", vr.package, vr.version, v.id))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let base = to_keys(baseline);
    let curr = to_keys(current);
    DiffSection {
        added: curr.difference(&base).cloned().collect(),
        removed: base.difference(&curr).cloned().collect(),
    }
}

fn diff_duplicates(baseline: &AnalysisResult, current: &AnalysisResult) -> DiffSection<String> {
    let to_keys = |r: &AnalysisResult| -> HashSet<String> {
        r.duplicates
            .as_ref()
            .map(|d| d.duplicates.iter().map(|e| e.name.clone()).collect())
            .unwrap_or_default()
    };
    let base = to_keys(baseline);
    let curr = to_keys(current);
    DiffSection {
        added: curr.difference(&base).cloned().collect(),
        removed: base.difference(&curr).cloned().collect(),
    }
}

fn diff_licenses(baseline: &AnalysisResult, current: &AnalysisResult) -> DiffSection<String> {
    let to_keys = |r: &AnalysisResult| -> HashSet<String> {
        r.license
            .as_ref()
            .map(|l| {
                l.violations
                    .iter()
                    .map(|v| format!("{}:{}", v.package, v.license))
                    .collect()
            })
            .unwrap_or_default()
    };
    let base = to_keys(baseline);
    let curr = to_keys(current);
    DiffSection {
        added: curr.difference(&base).cloned().collect(),
        removed: base.difference(&curr).cloned().collect(),
    }
}

fn diff_outdated(baseline: &AnalysisResult, current: &AnalysisResult) -> DiffSection<String> {
    let to_keys = |r: &AnalysisResult| -> HashSet<String> {
        r.outdated
            .as_ref()
            .map(|o| {
                o.entries
                    .iter()
                    .map(|e| format!("{}@{}", e.name, e.current))
                    .collect()
            })
            .unwrap_or_default()
    };
    let base = to_keys(baseline);
    let curr = to_keys(current);
    DiffSection {
        added: curr.difference(&base).cloned().collect(),
        removed: base.difference(&curr).cloned().collect(),
    }
}

fn diff_supply_chain(baseline: &AnalysisResult, current: &AnalysisResult) -> DiffSection<String> {
    let to_keys = |r: &AnalysisResult| -> HashSet<String> {
        r.supply_chain
            .as_ref()
            .map(|sc| {
                sc.risks
                    .iter()
                    .map(|r| format!("{}@{}", r.package, r.version))
                    .collect()
            })
            .unwrap_or_default()
    };
    let base = to_keys(baseline);
    let curr = to_keys(current);
    DiffSection {
        added: curr.difference(&base).cloned().collect(),
        removed: base.difference(&curr).cloned().collect(),
    }
}

fn count_issues(r: &AnalysisResult) -> usize {
    let unused = r.unused.as_ref().map(|u| u.unused.len()).unwrap_or(0);
    let vulns: usize = r
        .vulns
        .as_ref()
        .map(|v| v.iter().map(|vr| vr.vulns.len()).sum())
        .unwrap_or(0);
    let dups = r
        .duplicates
        .as_ref()
        .map(|d| d.duplicates.len())
        .unwrap_or(0);
    let lics = r.license.as_ref().map(|l| l.violations.len()).unwrap_or(0);
    let outdated = r.outdated.as_ref().map(|o| o.total_outdated).unwrap_or(0);
    let supply_chain = r
        .supply_chain
        .as_ref()
        .map(|sc| sc.risks.len())
        .unwrap_or(0);
    unused + vulns + dups + lics + outdated + supply_chain
}

fn format_delta(new: usize, resolved: usize) -> String {
    match (new > 0, resolved > 0) {
        (true, true) => format!("+{new}, -{resolved}"),
        (true, false) => format!("+{new}"),
        (false, true) => format!("-{resolved}"),
        _ => "—".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DepType, UnusedReport};

    fn make_baseline() -> AnalysisResult {
        AnalysisResult {
            unused: Some(UnusedReport {
                unused: vec![
                    UnusedDep {
                        name: "lodash".into(),
                        version: "4.17.21".into(),
                        dep_type: DepType::Prod,
                    },
                    UnusedDep {
                        name: "moment".into(),
                        version: "2.30.1".into(),
                        dep_type: DepType::Prod,
                    },
                ],
            }),
            vulns: None,
            duplicates: None,
            size: None,
            license: None,
            outdated: None,
            supply_chain: None,
        }
    }

    #[test]
    fn test_diff_detects_new_and_resolved() {
        let baseline = make_baseline();
        let current = AnalysisResult {
            unused: Some(UnusedReport {
                unused: vec![
                    UnusedDep {
                        name: "moment".into(),
                        version: "2.30.1".into(),
                        dep_type: DepType::Prod,
                    },
                    UnusedDep {
                        name: "axios".into(),
                        version: "1.6.0".into(),
                        dep_type: DepType::Prod,
                    },
                ],
            }),
            vulns: None,
            duplicates: None,
            size: None,
            license: None,
            outdated: None,
            supply_chain: None,
        };

        let dir = tempfile::tempdir().unwrap();
        let baseline_path = dir.path().join("baseline.json");
        std::fs::write(&baseline_path, serde_json::to_string(&baseline).unwrap()).unwrap();

        let report = compute_diff(&baseline_path, &current).unwrap();
        assert_eq!(report.unused.added.len(), 1);
        assert_eq!(report.unused.added[0].name, "axios");
        assert_eq!(report.unused.removed.len(), 1);
        assert_eq!(report.unused.removed[0].name, "lodash");
        assert_eq!(report.summary.new_issues, 1);
        assert_eq!(report.summary.resolved_issues, 1);
    }
}
