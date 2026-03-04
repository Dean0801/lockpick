use std::io::{self, Write};

use super::{Reporter, format_bytes};
use crate::AnalysisResult;
use crate::error::LockpickError;
use crate::i18n::I18n;

pub struct InteractiveReporter;

impl Reporter for InteractiveReporter {
    fn report(&self, result: &AnalysisResult, i18n: &I18n) -> Result<(), LockpickError> {
        print_summary(result, i18n);

        let sections = collect_sections(result, i18n);
        if sections.is_empty() {
            println!("\n{}", i18n.t("scan_complete"));
            return Ok(());
        }

        println!("\n{}", i18n.t("interactive_prompt"));
        for (i, (name, _)) in sections.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
        println!("  0. {}", i18n.t("interactive_exit"));

        loop {
            eprint!("\n{} ", i18n.t("interactive_select"));
            io::stderr().flush().ok();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                break;
            }

            let choice = input.trim();
            if choice == "0" || choice.is_empty() {
                break;
            }

            if let Ok(idx) = choice.parse::<usize>()
                && idx > 0
                && idx <= sections.len()
            {
                println!();
                (sections[idx - 1].1)();
            }
        }

        println!("\n{}", i18n.t("scan_complete"));
        Ok(())
    }
}

fn print_summary(result: &AnalysisResult, i18n: &I18n) {
    println!("\n{}", "📊 Summary".bold());

    if let Some(ref u) = result.unused
        && !u.unused.is_empty()
    {
        println!("  📦 {}: {}", i18n.t("unused_deps"), u.unused.len());
    }

    if let Some(ref v) = result.vulns {
        let total: usize = v.iter().map(|r| r.vulns.len()).sum();
        if total > 0 {
            println!("  🛡️ {}: {}", i18n.t("vulns"), total);
        }
    }

    if let Some(ref d) = result.duplicates
        && !d.duplicates.is_empty()
    {
        println!(
            "  🔄 {}: {}",
            i18n.t("duplicate_deps"),
            d.total_duplicate_packages
        );
    }

    if let Some(ref s) = result.size
        && !s.entries.is_empty()
    {
        println!(
            "  📊 {}: {}",
            i18n.t("size_analysis"),
            format_bytes(s.total_bytes)
        );
    }

    if let Some(ref l) = result.license
        && !l.violations.is_empty()
    {
        println!(
            "  ⚠️ {}: {}",
            i18n.t("license_violations"),
            l.violations.len()
        );
    }

    if let Some(ref o) = result.outdated
        && !o.entries.is_empty()
    {
        println!("  📦 {}: {}", i18n.t("outdated_deps"), o.total_outdated);
    }

    if let Some(ref sc) = result.supply_chain
        && !sc.risks.is_empty()
    {
        println!("  🔗 {}: {}", i18n.t("supply_chain_risks"), sc.risks.len());
    }
}

fn collect_sections<'a>(
    result: &'a AnalysisResult,
    i18n: &'a I18n,
) -> Vec<(String, Box<dyn Fn() + 'a>)> {
    let mut sections: Vec<(String, Box<dyn Fn() + 'a>)> = Vec::new();

    if result.unused.as_ref().is_some_and(|u| !u.unused.is_empty()) {
        sections.push((
            i18n.t("unused_deps").to_string(),
            Box::new(|| super::terminal::print_unused(result, i18n)),
        ));
    }

    if result.vulns.as_ref().is_some_and(|v| !v.is_empty()) {
        sections.push((
            i18n.t("vulns").to_string(),
            Box::new(|| super::terminal::print_vulns(result, i18n)),
        ));
    }

    if result
        .duplicates
        .as_ref()
        .is_some_and(|d| !d.duplicates.is_empty())
    {
        sections.push((
            i18n.t("duplicate_deps").to_string(),
            Box::new(|| super::terminal::print_duplicates(result, i18n)),
        ));
    }

    if result.size.as_ref().is_some_and(|s| !s.entries.is_empty()) {
        sections.push((
            i18n.t("size_analysis").to_string(),
            Box::new(|| super::terminal::print_size(result, i18n)),
        ));
    }

    if result
        .license
        .as_ref()
        .is_some_and(|l| !l.violations.is_empty())
    {
        sections.push((
            i18n.t("license_violations").to_string(),
            Box::new(|| super::terminal::print_license(result, i18n)),
        ));
    }

    if result
        .outdated
        .as_ref()
        .is_some_and(|o| !o.entries.is_empty())
    {
        sections.push((
            i18n.t("outdated_deps").to_string(),
            Box::new(|| super::terminal::print_outdated(result, i18n)),
        ));
    }

    if result
        .supply_chain
        .as_ref()
        .is_some_and(|sc| !sc.risks.is_empty())
    {
        sections.push((
            i18n.t("supply_chain_risks").to_string(),
            Box::new(|| super::terminal::print_supply_chain(result, i18n)),
        ));
    }

    sections
}

use owo_colors::OwoColorize;
