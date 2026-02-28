use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::analyze::{AnalyzeOptions, analyze_package};
use crate::config::types::LicensePolicy;
use crate::i18n::I18n;
use crate::report::Reporter;
use crate::{AnalysisResult, DependencyGraph, PackageInfo, VulnReport};

/// Runtime configuration resolved from CLI args + .lockpickrc
pub struct RunConfig<'a> {
    pub skip_dev: bool,
    pub ignore: &'a HashSet<String>,
    pub extra_configs: &'a [String],
    pub license_policy: Option<&'a LicensePolicy>,
    pub run_unused: bool,
    pub run_audit: bool,
    pub run_fix: bool,
    pub verbose: bool,
    pub dry_run: bool,
    pub no_cache: bool,
    pub cache_ttl: Option<u64>,
}

/// Read package.json dependencies and devDependencies from a directory.
pub fn read_package_deps(
    pkg_dir: &Path,
) -> Option<(HashMap<String, PackageInfo>, HashMap<String, PackageInfo>)> {
    let pkg_path = pkg_dir.join("package.json");
    let content = std::fs::read_to_string(&pkg_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let mut deps = HashMap::new();
    let mut dev_deps = HashMap::new();

    if let Some(obj) = json.get("dependencies").and_then(|d| d.as_object()) {
        for (name, version) in obj {
            deps.insert(
                name.clone(),
                PackageInfo {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("*").to_string(),
                    integrity: None,
                },
            );
        }
    }

    if let Some(obj) = json.get("devDependencies").and_then(|d| d.as_object()) {
        for (name, version) in obj {
            dev_deps.insert(
                name.clone(),
                PackageInfo {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("*").to_string(),
                    integrity: None,
                },
            );
        }
    }

    Some((deps, dev_deps))
}

/// Run vulnerability scan, returning results or None on error.
pub async fn run_audit(
    graph: &DependencyGraph,
    cache_ttl: Option<u64>,
    no_cache: bool,
    i18n: &I18n,
) -> Option<Vec<VulnReport>> {
    match crate::audit::osv::scan_vulnerabilities(graph, cache_ttl, no_cache).await {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("{}: {e}", i18n.t("network_error"));
            None
        }
    }
}

/// Build AnalyzeOptions from RunConfig for a given package path and graph.
fn build_opts<'a>(
    project_path: &'a Path,
    graph: &'a DependencyGraph,
    cfg: &'a RunConfig<'a>,
    i18n: &'a I18n,
) -> AnalyzeOptions<'a> {
    AnalyzeOptions {
        project_path,
        graph,
        skip_dev: cfg.skip_dev,
        ignore: cfg.ignore,
        extra_configs: cfg.extra_configs,
        run_unused: cfg.run_unused,
        run_duplicates: true,
        run_size: true,
        run_license: true,
        license_policy: cfg.license_policy,
        verbose: cfg.verbose,
        i18n,
    }
}

/// Run monorepo analysis: analyze each workspace package, then audit at root level.
/// Returns true if there are issues (unused deps or vulns).
pub async fn run_monorepo(
    project_path: &Path,
    graph: &DependencyGraph,
    cfg: &RunConfig<'_>,
    i18n: &I18n,
    reporter: &dyn Reporter,
) -> bool {
    let workspace_packages = match crate::workspace::detect_workspaces(project_path) {
        Ok(pkgs) => pkgs,
        Err(e) => {
            eprintln!("Error detecting workspaces: {e}");
            return true;
        }
    };

    if cfg.run_fix {
        eprintln!("Warning: fix mode is not yet supported for monorepo workspaces");
    }

    if cfg.verbose {
        eprintln!(
            "{} {} {}",
            i18n.t("monorepo_detected"),
            workspace_packages.len(),
            i18n.t("workspace_packages")
        );
    }

    let mut all_has_unused = false;

    for pkg_dir in &workspace_packages {
        let pkg_name = pkg_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        eprintln!("\n--- {} {} ---", i18n.t("workspace_package"), pkg_name);

        let (pkg_deps, pkg_dev_deps) = match read_package_deps(pkg_dir) {
            Some(d) => d,
            None => {
                eprintln!("  {}", i18n.t("skip_no_deps"));
                continue;
            }
        };

        let pkg_graph = DependencyGraph {
            dependencies: pkg_deps,
            dev_dependencies: pkg_dev_deps,
            lockfile_type: graph.lockfile_type.clone(),
            all_packages: graph.all_packages.clone(),
            dep_edges: HashMap::new(),
        };

        let opts = build_opts(pkg_dir, &pkg_graph, cfg, i18n);

        match analyze_package(&opts) {
            Ok(pkg_result) => {
                if pkg_result.unused.as_ref().is_some_and(|u| !u.unused.is_empty()) {
                    all_has_unused = true;
                }
                if let Err(e) = reporter.report(&pkg_result, i18n) {
                    eprintln!("Report error: {e}");
                }
            }
            Err(e) => {
                eprintln!("  Error analyzing {pkg_name}: {e}");
                continue;
            }
        }
    }

    // Vulnerability scan at root level (shared lockfile)
    let vulns = if cfg.run_audit {
        run_audit(graph, cfg.cache_ttl, cfg.no_cache, i18n).await
    } else {
        None
    };

    let has_vulns = vulns.as_ref().is_some_and(|v| !v.is_empty());

    if let Some(ref v) = vulns {
        let vuln_result = AnalysisResult {
            unused: None,
            vulns: Some(v.clone()),
            duplicates: None,
            size: None,
            license: None,
        };
        if let Err(e) = reporter.report(&vuln_result, i18n) {
            eprintln!("Report error: {e}");
        }
    }

    all_has_unused || has_vulns
}

/// Run single-package analysis. Returns (has_issues, analysis_result).
pub async fn run_single(
    project_path: &Path,
    graph: &DependencyGraph,
    cfg: &RunConfig<'_>,
    i18n: &I18n,
    reporter: &dyn Reporter,
) -> (bool, Option<AnalysisResult>) {
    let opts = build_opts(project_path, graph, cfg, i18n);

    let mut result = match analyze_package(&opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return (true, None);
        }
    };

    // Vulnerability scan
    result.vulns = if cfg.run_audit {
        run_audit(graph, cfg.cache_ttl, cfg.no_cache, i18n).await
    } else {
        None
    };

    // Fix mode
    if cfg.run_fix {
        let has = run_fix_mode(project_path, &result, graph, cfg, i18n);
        return (has, Some(result));
    }

    if let Err(e) = reporter.report(&result, i18n) {
        eprintln!("Report error: {e}");
        return (true, Some(result));
    }

    let has_unused = result.unused.as_ref().is_some_and(|u| !u.unused.is_empty());
    let has_vulns = result.vulns.as_ref().is_some_and(|v| !v.is_empty());
    (has_unused || has_vulns, Some(result))
}

/// Handle fix mode: dry-run or actual removal. Returns true if there are issues.
fn run_fix_mode(
    project_path: &Path,
    result: &AnalysisResult,
    graph: &DependencyGraph,
    cfg: &RunConfig<'_>,
    i18n: &I18n,
) -> bool {
    let unused_report = match &result.unused {
        Some(r) if !r.unused.is_empty() => r,
        _ => {
            eprintln!("{}", i18n.t("fix_nothing"));
            return false;
        }
    };

    if cfg.dry_run {
        eprintln!("{}", i18n.t("fix_dry_run"));
        for dep in &unused_report.unused {
            eprintln!("  - {} ({})", dep.name, dep.version);
        }
        return false;
    }

    match crate::fix::fix_unused(project_path, &unused_report.unused, &graph.lockfile_type, cfg.dry_run) {
        Ok(fix_result) => {
            for name in &fix_result.removed {
                eprintln!("{}: {}", i18n.t("fix_done"), name);
            }
            for (name, err) in &fix_result.failed {
                eprintln!("{}: {} - {}", i18n.t("fix_failed"), name, err);
            }
            eprintln!("{}", i18n.t("fix_done"));
            !fix_result.failed.is_empty()
        }
        Err(e) => {
            eprintln!("Error: {e}");
            true
        }
    }
}
