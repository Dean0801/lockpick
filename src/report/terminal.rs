use comfy_table::{ContentArrangement, Table};
use owo_colors::OwoColorize;

use super::{Reporter, format_bytes};
use crate::error::LockpickError;
use crate::i18n::I18n;
use crate::{AnalysisResult, DepType, Severity, SupplyChainRiskType, ViolationReason};

pub struct TerminalReporter;

impl Reporter for TerminalReporter {
    fn report(&self, result: &AnalysisResult, i18n: &I18n) -> Result<(), LockpickError> {
        print_unused(result, i18n);
        print_vulns(result, i18n);
        print_duplicates(result, i18n);
        print_size(result, i18n);
        print_license(result, i18n);
        print_outdated(result, i18n);
        print_supply_chain(result, i18n);
        println!("\n{}", i18n.t("scan_complete").green());
        Ok(())
    }
}

pub fn print_unused(result: &AnalysisResult, i18n: &I18n) {
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

pub fn print_vulns(result: &AnalysisResult, i18n: &I18n) {
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

pub fn print_duplicates(result: &AnalysisResult, i18n: &I18n) {
    let Some(ref dups) = result.duplicates else {
        return;
    };

    if dups.duplicates.is_empty() {
        println!("\n{}", i18n.t("no_duplicates").green());
        return;
    }

    println!(
        "\n{} {} ({} {})",
        "🔀".bold(),
        i18n.t("duplicate_deps").bold(),
        dups.total_duplicate_packages,
        i18n.t("found")
    );

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![i18n.t("package"), i18n.t("versions")]);

    for dup in &dups.duplicates {
        table.add_row(vec![&dup.name, &dup.versions.join(", ")]);
    }

    println!("{table}");
}

pub fn print_size(result: &AnalysisResult, i18n: &I18n) {
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

pub fn print_license(result: &AnalysisResult, i18n: &I18n) {
    let Some(ref license) = result.license else {
        return;
    };

    if license.entries.is_empty() {
        return;
    }

    println!(
        "\n{} {} ({} {})",
        "📜".bold(),
        i18n.t("license_report").bold(),
        license.entries.len(),
        i18n.t("found")
    );

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        i18n.t("package"),
        i18n.t("version"),
        i18n.t("license"),
        i18n.t("type"),
    ]);

    for entry in &license.entries {
        let dep_type = match entry.dep_type {
            DepType::Prod => i18n.t("prod"),
            DepType::Dev => i18n.t("dev"),
        };
        table.add_row(vec![&entry.name, &entry.version, &entry.license, dep_type]);
    }

    println!("{table}");

    if license.violations.is_empty() {
        println!("{}", i18n.t("no_license_violations").green());
        return;
    }

    println!(
        "\n{} {} ({} {})",
        "⚠️".bold(),
        i18n.t("license_violations").bold(),
        license.violations.len(),
        i18n.t("found")
    );

    let mut vtable = Table::new();
    vtable.set_content_arrangement(ContentArrangement::Dynamic);
    vtable.set_header(vec![
        i18n.t("package"),
        i18n.t("version"),
        i18n.t("license"),
        i18n.t("reason"),
    ]);

    for v in &license.violations {
        let reason = match v.reason {
            ViolationReason::Denied => i18n.t("violation_denied"),
            ViolationReason::NotAllowed => i18n.t("violation_not_allowed"),
            ViolationReason::Unknown => i18n.t("violation_unknown"),
        };
        vtable.add_row(vec![&v.package, &v.version, &v.license, reason]);
    }

    println!("{vtable}");
}

pub fn print_outdated(result: &AnalysisResult, i18n: &I18n) {
    let Some(ref report) = result.outdated else {
        return;
    };
    if report.entries.is_empty() {
        println!("\n{}", i18n.t("no_outdated").green());
        return;
    }

    println!(
        "\n{} {} ({})",
        "📦".bold(),
        i18n.t("outdated_deps").bold(),
        report.total_outdated,
    );

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        i18n.t("package"),
        i18n.t("current"),
        i18n.t("latest"),
        i18n.t("level"),
        i18n.t("priority"),
    ]);

    for e in &report.entries {
        let level = format!("{}", e.level);
        let priority = match e.priority {
            crate::UpgradePriority::Critical => format!("🔴 {}", i18n.t("critical").red()),
            crate::UpgradePriority::High => format!("🟡 {}", i18n.t("high").yellow()),
            crate::UpgradePriority::Medium => format!("🟠 {}", i18n.t("medium").cyan()),
            crate::UpgradePriority::Low => format!("⚪ {}", i18n.t("low")),
        };
        table.add_row(vec![&e.name, &e.current, &e.latest, &level, &priority]);
    }
    println!("{table}");
    println!(
        "  {}: {} patch | {} minor | {} major",
        i18n.t("summary"),
        report.patch_count,
        report.minor_count,
        report.major_count
    );
}

pub fn print_supply_chain(result: &AnalysisResult, i18n: &I18n) {
    let Some(ref report) = result.supply_chain else {
        return;
    };
    if report.risks.is_empty() {
        println!("\n{}", i18n.t("no_supply_chain_risks").green());
        return;
    }

    println!(
        "\n{} {} ({} {})",
        "🔗".bold(),
        i18n.t("supply_chain_analysis").bold(),
        report.risks.len(),
        i18n.t("found"),
    );

    for risk in &report.risks {
        let sev = match risk.severity {
            Severity::Critical | Severity::High => format!("🔴 {}", i18n.t("high").red()),
            Severity::Medium => format!("🟡 {}", i18n.t("medium").yellow()),
            Severity::Low => format!("⚪ {}", i18n.t("low")),
        };
        let desc = match &risk.risk_type {
            SupplyChainRiskType::Typosquat {
                similar_to,
                distance,
            } => format!("similar to \"{}\" (distance: {})", similar_to, distance),
            SupplyChainRiskType::ScopeConfusion { legitimate } => {
                format!("may impersonate \"{}\"", legitimate)
            }
            SupplyChainRiskType::VersionAnomaly { installed_version } => {
                format!("abnormal version ({})", installed_version)
            }
        };
        println!("  ⚠️  {}@{} → {} {}", risk.package, risk.version, desc, sev);
    }
}
