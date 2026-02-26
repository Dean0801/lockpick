use comfy_table::{ContentArrangement, Table};
use owo_colors::OwoColorize;

use super::Reporter;
use crate::i18n::I18n;
use crate::{AnalysisResult, DepType, Severity};

pub struct TerminalReporter;

impl Reporter for TerminalReporter {
    fn report(&self, result: &AnalysisResult, i18n: &I18n) -> Result<(), String> {
        print_unused(result, i18n);
        print_vulns(result, i18n);
        print_size(result, i18n);
        println!("\n{}", i18n.t("scan_complete").green());
        Ok(())
    }
}

fn print_unused(result: &AnalysisResult, i18n: &I18n) {
    let Some(ref unused) = result.unused else {
        return;
    };

    if unused.unused.is_empty() {
        println!("\n{}", i18n.t("no_unused").green());
        return;
    }

    let count = unused.unused.len();
    println!(
        "\n{} {} ({} {})",
        "📦".bold(),
        i18n.t("unused_deps").bold(),
        count,
        i18n.t("found")
    );

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![i18n.t("package"), i18n.t("version"), i18n.t("type")]);

    for dep in &unused.unused {
        let dep_type = match dep.dep_type {
            DepType::Prod => i18n.t("prod"),
            DepType::Dev => i18n.t("dev"),
        };
        table.add_row(vec![&dep.name, &dep.version, dep_type]);
    }

    println!("{table}");
}

fn print_vulns(result: &AnalysisResult, i18n: &I18n) {
    let Some(ref vulns) = result.vulns else {
        return;
    };

    if vulns.is_empty() {
        println!("\n{}", i18n.t("no_vulns").green());
        return;
    }

    let total: usize = vulns.iter().map(|v| v.vulns.len()).sum();
    println!(
        "\n{} {} ({} {})",
        "🛡️".bold(),
        i18n.t("vulns").bold(),
        total,
        i18n.t("found")
    );

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        i18n.t("package"),
        i18n.t("version"),
        i18n.t("severity"),
        i18n.t("fix_version"),
    ]);

    for report in vulns {
        for vuln in &report.vulns {
            let sev = match vuln.severity {
                Severity::Critical => i18n.t("critical").red().to_string(),
                Severity::High => i18n.t("high").yellow().to_string(),
                Severity::Medium => i18n.t("medium").cyan().to_string(),
                Severity::Low => i18n.t("low").to_string(),
            };
            let fix = vuln.fixed_version.as_deref().unwrap_or(i18n.t("none"));
            table.add_row(vec![&report.package, &report.version, &sev, fix]);
        }
    }

    println!("{table}");
}

fn print_size(result: &AnalysisResult, i18n: &I18n) {
    let Some(ref size) = result.size else {
        return;
    };
    if size.entries.is_empty() {
        return;
    }

    println!(
        "\n{} {} ({} {})",
        "📊".bold(),
        i18n.t("size_analysis").bold(),
        format_bytes(size.total_bytes),
        i18n.t("total_size")
    );

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![i18n.t("package"), i18n.t("size")]);

    for entry in &size.entries {
        if entry.size_bytes > 0 {
            table.add_row(vec![&entry.name, &format_bytes(entry.size_bytes)]);
        }
    }
    println!("{table}");
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
