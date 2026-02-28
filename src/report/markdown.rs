use super::Reporter;
use crate::error::LockpickError;
use crate::i18n::I18n;
use crate::{AnalysisResult, DepType, Severity, ViolationReason};

pub struct MarkdownReporter;

impl Reporter for MarkdownReporter {
    fn report(&self, result: &AnalysisResult, i18n: &I18n) -> Result<(), LockpickError> {
        let content = self.render(result, i18n);
        print!("{content}");
        Ok(())
    }
}

impl MarkdownReporter {
    /// Render analysis result as Markdown string (reusable for --output and diff)
    pub fn render(&self, result: &AnalysisResult, i18n: &I18n) -> String {
        let mut md = String::new();
        md.push_str(&format!("# 🔍 lockpick {}\n\n", i18n.t("scan_complete")));
        Self::render_unused(&mut md, result, i18n);
        Self::render_vulns(&mut md, result, i18n);
        Self::render_duplicates(&mut md, result, i18n);
        Self::render_size(&mut md, result, i18n);
        Self::render_license(&mut md, result, i18n);
        md
    }

    fn render_unused(md: &mut String, result: &AnalysisResult, i18n: &I18n) {
        let Some(ref report) = result.unused else { return };
        if report.unused.is_empty() { return; }

        md.push_str(&format!("## 📦 {} ({})\n\n", i18n.t("unused_deps"), report.unused.len()));
        md.push_str(&format!("| {} | {} | {} |\n", i18n.t("package"), i18n.t("version"), i18n.t("type")));
        md.push_str("|------|------|------|\n");
        for dep in &report.unused {
            let dep_type = match dep.dep_type { DepType::Prod => i18n.t("prod"), DepType::Dev => i18n.t("dev") };
            md.push_str(&format!("| {} | {} | {} |\n", dep.name, dep.version, dep_type));
        }
        md.push('\n');
    }

    fn render_vulns(md: &mut String, result: &AnalysisResult, i18n: &I18n) {
        let Some(ref vulns) = result.vulns else { return };
        if vulns.is_empty() { return; }

        let total: usize = vulns.iter().map(|v| v.vulns.len()).sum();
        md.push_str(&format!("## 🛡️ {} ({})\n\n", i18n.t("vulns"), total));
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            i18n.t("package"), i18n.t("version"), i18n.t("severity"), "ID", i18n.t("fix_version")
        ));
        md.push_str("|------|------|----------|--------|----------|\n");
        for report in vulns {
            for vuln in &report.vulns {
                let sev = match vuln.severity {
                    Severity::Critical => format!("🔴 {}", i18n.t("critical")),
                    Severity::High => format!("🔴 {}", i18n.t("high")),
                    Severity::Medium => format!("🟡 {}", i18n.t("medium")),
                    Severity::Low => format!("🟢 {}", i18n.t("low")),
                };
                let fix = vuln.fixed_version.as_deref().unwrap_or(i18n.t("none"));
                md.push_str(&format!("| {} | {} | {} | {} | {} |\n", report.package, report.version, sev, vuln.id, fix));
            }
        }
        md.push('\n');
    }

    fn render_duplicates(md: &mut String, result: &AnalysisResult, i18n: &I18n) {
        let Some(ref dups) = result.duplicates else { return };
        if dups.duplicates.is_empty() { return; }

        md.push_str(&format!("## 🔄 {} ({})\n\n", i18n.t("duplicate_deps"), dups.total_duplicate_packages));
        md.push_str(&format!("| {} | {} |\n", i18n.t("package"), i18n.t("versions")));
        md.push_str("|------|------|\n");
        for dup in &dups.duplicates {
            md.push_str(&format!("| {} | {} |\n", dup.name, dup.versions.join(", ")));
        }
        md.push('\n');
    }

    fn render_size(md: &mut String, result: &AnalysisResult, i18n: &I18n) {
        let Some(ref size) = result.size else { return };
        if size.entries.is_empty() { return; }

        md.push_str(&format!("## 📊 {} ({} {})\n\n", i18n.t("size_analysis"), format_bytes(size.total_bytes), i18n.t("total_size")));
        md.push_str(&format!("| {} | {} |\n", i18n.t("package"), i18n.t("size")));
        md.push_str("|------|------|\n");
        for entry in &size.entries {
            if entry.size_bytes > 0 {
                md.push_str(&format!("| {} | {} |\n", entry.name, format_bytes(entry.size_bytes)));
            }
        }
        md.push('\n');
    }

    fn render_license(md: &mut String, result: &AnalysisResult, i18n: &I18n) {
        let Some(ref license) = result.license else { return };
        if license.entries.is_empty() { return; }

        md.push_str(&format!("## 📜 {} ({} {})\n\n", i18n.t("license_report"), license.entries.len(), i18n.t("found")));
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            i18n.t("package"), i18n.t("version"), i18n.t("license"), i18n.t("type")
        ));
        md.push_str("|------|------|------|------|\n");
        for entry in &license.entries {
            let dep_type = match entry.dep_type { DepType::Prod => i18n.t("prod"), DepType::Dev => i18n.t("dev") };
            md.push_str(&format!("| {} | {} | {} | {} |\n", entry.name, entry.version, entry.license, dep_type));
        }
        md.push('\n');

        if license.violations.is_empty() { return; }

        md.push_str(&format!("### ⚠️ {} ({})\n\n", i18n.t("license_violations"), license.violations.len()));
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            i18n.t("package"), i18n.t("version"), i18n.t("license"), i18n.t("reason")
        ));
        md.push_str("|------|------|------|------|\n");
        for v in &license.violations {
            let reason = match v.reason {
                ViolationReason::Denied => i18n.t("violation_denied"),
                ViolationReason::NotAllowed => i18n.t("violation_not_allowed"),
                ViolationReason::Unknown => i18n.t("violation_unknown"),
            };
            md.push_str(&format!("| {} | {} | {} | {} |\n", v.package, v.version, v.license, reason));
        }
        md.push('\n');
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{UnusedDep, UnusedReport, VulnReport, Vulnerability};

    fn en() -> I18n {
        I18n::detect(Some("en"))
    }

    #[test]
    fn test_render_unused_section() {
        let result = AnalysisResult {
            unused: Some(UnusedReport {
                unused: vec![UnusedDep {
                    name: "lodash".into(),
                    version: "4.17.21".into(),
                    dep_type: DepType::Prod,
                }],
            }),
            vulns: None,
            duplicates: None,
            size: None,
            license: None,
        };
        let md = MarkdownReporter.render(&result, &en());
        assert!(md.contains("| lodash | 4.17.21 |"));
        assert!(md.contains("## 📦"));
    }

    #[test]
    fn test_empty_sections_skipped() {
        let result = AnalysisResult {
            unused: None,
            vulns: None,
            duplicates: None,
            size: None,
            license: None,
        };
        let md = MarkdownReporter.render(&result, &en());
        assert!(!md.contains("## 📦"));
        assert!(!md.contains("## 🛡"));
        assert!(!md.contains("## 🔄"));
    }

    #[test]
    fn test_render_vulns_severity_emoji() {
        let result = AnalysisResult {
            unused: None,
            vulns: Some(vec![VulnReport {
                package: "semver".into(),
                version: "7.5.2".into(),
                vulns: vec![Vulnerability {
                    id: "GHSA-1234".into(),
                    summary: "test".into(),
                    severity: Severity::Critical,
                    fixed_version: Some("7.5.4".into()),
                }],
            }]),
            duplicates: None,
            size: None,
            license: None,
        };
        let md = MarkdownReporter.render(&result, &en());
        assert!(md.contains("🔴"));
        assert!(md.contains("GHSA-1234"));
        assert!(md.contains("7.5.4"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
    }
}
