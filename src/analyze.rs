use std::collections::HashSet;
use std::path::Path;

use crate::error::LockpickError;
use crate::i18n::I18n;
use crate::{AnalysisResult, DependencyGraph, UnusedReport};
use crate::config::types::LicensePolicy;

/// Options for analyzing a single package
pub struct AnalyzeOptions<'a> {
    pub project_path: &'a Path,
    pub graph: &'a DependencyGraph,
    pub skip_dev: bool,
    pub ignore: &'a HashSet<String>,
    pub extra_configs: &'a [String],
    pub run_unused: bool,
    pub run_duplicates: bool,
    pub run_size: bool,
    pub run_license: bool,
    pub license_policy: Option<&'a LicensePolicy>,
    pub verbose: bool,
    pub i18n: &'a I18n,
}

/// Analyze a single package: unused detection, duplicates, size
pub fn analyze_package(opts: &AnalyzeOptions<'_>) -> Result<AnalysisResult, LockpickError> {
    let unused = if opts.run_unused {
        Some(run_unused_detection(opts)?)
    } else {
        None
    };

    let duplicates = if opts.run_duplicates {
        Some(crate::analyzer::duplicates::detect_duplicates(opts.graph))
    } else {
        None
    };

    let size = if opts.run_size {
        Some(crate::analyzer::size::analyze_size(
            opts.project_path,
            opts.graph,
        ))
    } else {
        None
    };

    let license = if opts.run_license {
        let mut report = crate::analyzer::license::extract_licenses(
            opts.project_path,
            opts.graph,
            opts.skip_dev,
        );
        if let Some(policy) = opts.license_policy {
            report.violations = crate::analyzer::license::check_policy(&report, policy);
        }
        Some(report)
    } else {
        None
    };

    Ok(AnalysisResult {
        unused,
        vulns: None,
        duplicates,
        size,
        license,
    })
}

/// Run the full unused dependency detection pipeline
fn run_unused_detection(opts: &AnalyzeOptions<'_>) -> Result<UnusedReport, LockpickError> {
    // 1. Discover source files
    let files = crate::scanner::discover_source_files(opts.project_path)?;

    // 2. Extract imports from all source files
    let mut used = HashSet::new();
    for file in &files {
        if let Ok(source) = std::fs::read_to_string(file) {
            let imports =
                crate::scanner::imports::extract_imports_from_source(&source, file);
            used.extend(imports);
        }
    }

    // 3. Scan config files for plugin references
    let config_deps = crate::scanner::config::extract_config_deps(opts.project_path);
    used.extend(config_deps);

    // 4. Scan extra config files from .lockpickrc
    let extra_deps = crate::scanner::config::extract_extra_config_deps(
        opts.project_path,
        opts.extra_configs,
    );
    used.extend(extra_deps);

    // 5. Scan package.json scripts for CLI tool references
    let script_deps = crate::scanner::scripts::extract_script_deps(opts.project_path);
    used.extend(script_deps);

    if opts.verbose {
        eprintln!("{}", opts.i18n.t("scan_config_complete"));
    }

    // 6. Detect unused
    let mut report =
        crate::scanner::unused::detect_unused(opts.graph, &used, opts.skip_dev);

    // 7. Apply ignore filter
    if !opts.ignore.is_empty() {
        report.unused.retain(|dep| !opts.ignore.contains(&dep.name));
    }

    Ok(report)
}
