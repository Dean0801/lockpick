use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use lockpick::config::load_config;
use lockpick::i18n::I18n;
use lockpick::report::{Reporter, json::JsonReporter, terminal::TerminalReporter};

#[derive(Parser)]
#[command(name = "lockpick")]
#[command(version, about = "Blazing-fast JS/TS dependency analyzer")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Project path (defaults to current directory)
    #[arg(short, long, global = true)]
    path: Option<PathBuf>,

    /// Output format
    #[arg(short, long, global = true, default_value = "terminal")]
    format: OutputFormat,

    /// Language (auto-detect if not specified)
    #[arg(short, long, global = true)]
    lang: Option<LangOption>,

    /// Ignore specific packages (can be used multiple times)
    #[arg(long, global = true)]
    ignore: Vec<String>,

    /// Skip devDependencies
    #[arg(long, global = true)]
    no_dev: bool,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run full analysis (default)
    Scan,
    /// Detect unused dependencies only
    Unused,
    /// Vulnerability scan only
    Audit,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Terminal,
    Json,
}

#[derive(Clone, ValueEnum)]
enum LangOption {
    En,
    Zh,
}

/// Read package.json dependencies and devDependencies
fn read_package_deps(pkg_dir: &std::path::Path) -> Option<(
    std::collections::HashMap<String, lockpick::PackageInfo>,
    std::collections::HashMap<String, lockpick::PackageInfo>,
)> {
    let pkg_path = pkg_dir.join("package.json");
    let content = std::fs::read_to_string(&pkg_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let mut deps = std::collections::HashMap::new();
    let mut dev_deps = std::collections::HashMap::new();

    if let Some(obj) = json.get("dependencies").and_then(|d| d.as_object()) {
        for (name, version) in obj {
            deps.insert(
                name.clone(),
                lockpick::PackageInfo {
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
                lockpick::PackageInfo {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("*").to_string(),
                    integrity: None,
                },
            );
        }
    }

    Some((deps, dev_deps))
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Load .lockpickrc config
    let project_path = cli.path.clone().unwrap_or_else(|| PathBuf::from("."));
    let rc_config = load_config(&project_path);

    // Merge: CLI args override .lockpickrc
    let effective_no_dev = cli.no_dev || rc_config.skip_dev;

    let effective_ignore: Vec<String> = {
        let mut merged = rc_config.ignore.clone();
        merged.extend(cli.ignore.clone());
        merged.sort();
        merged.dedup();
        merged
    };

    // Detect language: CLI > .lockpickrc > env > default
    let lang_str = cli
        .lang
        .as_ref()
        .map(|l| match l {
            LangOption::En => "en",
            LangOption::Zh => "zh",
        })
        .or(rc_config.lang.as_deref());
    let i18n = I18n::detect(lang_str);

    if !project_path.exists() {
        eprintln!("Error: path '{}' does not exist", project_path.display());
        std::process::exit(1);
    }

    // Determine which analyses to run
    let (run_unused, run_audit) = match &cli.command {
        Some(Commands::Unused) => (true, false),
        Some(Commands::Audit) => (false, true),
        Some(Commands::Scan) | None => (true, true),
    };

    if cli.verbose {
        eprintln!("{}", i18n.t("analyzing"));
    }

    // Parse lockfile (auto-detect)
    let graph = match lockpick::lockfile::detect_and_parse(&project_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    // Create reporter early (needed for both monorepo and single-package flows)
    let reporter: Box<dyn Reporter> = match cli.format {
        OutputFormat::Terminal => Box::new(TerminalReporter),
        OutputFormat::Json => Box::new(JsonReporter),
    };

    // Check if monorepo
    if lockpick::workspace::is_monorepo(&project_path) {
        let workspace_packages = match lockpick::workspace::detect_workspaces(&project_path) {
            Ok(pkgs) => pkgs,
            Err(e) => {
                eprintln!("Error detecting workspaces: {e}");
                std::process::exit(1);
            }
        };

        if cli.verbose {
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

            let pkg_graph = lockpick::DependencyGraph {
                dependencies: pkg_deps,
                dev_dependencies: pkg_dev_deps,
                lockfile_type: graph.lockfile_type.clone(),
                all_packages: graph.all_packages.clone(),
            };

            if run_unused {
                let files = match lockpick::scanner::discover_source_files(pkg_dir) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("  Error scanning {pkg_name}: {e}");
                        continue;
                    }
                };

                let mut used = std::collections::HashSet::new();
                for file in &files {
                    if let Ok(source) = std::fs::read_to_string(file) {
                        let imports =
                            lockpick::scanner::imports::extract_imports_from_source(&source, file);
                        used.extend(imports);
                    }
                }

                let config_deps = lockpick::scanner::config::extract_config_deps(pkg_dir);
                used.extend(config_deps);

                let script_deps = lockpick::scanner::scripts::extract_script_deps(pkg_dir);
                used.extend(script_deps);

                let mut report = lockpick::scanner::unused::detect_unused(
                    &pkg_graph,
                    &used,
                    effective_no_dev,
                );

                if !effective_ignore.is_empty() {
                    report
                        .unused
                        .retain(|dep| !effective_ignore.contains(&dep.name));
                }

                if !report.unused.is_empty() {
                    all_has_unused = true;
                }

                let pkg_result = lockpick::AnalysisResult {
                    unused: Some(report),
                    vulns: None,
                    duplicates: None,
                    size: None,
                };

                if let Err(e) = reporter.report(&pkg_result, &i18n) {
                    eprintln!("Report error: {e}");
                }
            }
        }

        // Vulnerability scan at root level (shared lockfile)
        let vulns = if run_audit {
            match lockpick::audit::osv::scan_vulnerabilities(&graph).await {
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("{}: {e}", i18n.t("network_error"));
                    None
                }
            }
        } else {
            None
        };

        let has_vulns = vulns.as_ref().is_some_and(|v| !v.is_empty());

        if let Some(ref v) = vulns {
            let vuln_result = lockpick::AnalysisResult {
                unused: None,
                vulns: Some(v.clone()),
                duplicates: None,
                size: None,
            };
            if let Err(e) = reporter.report(&vuln_result, &i18n) {
                eprintln!("Report error: {e}");
            }
        }

        if all_has_unused || has_vulns {
            std::process::exit(1);
        }
        return;
    }

    // --- Single package flow continues below (existing code) ---

    // Unused detection
    let unused = if run_unused {
        let files = match lockpick::scanner::discover_source_files(&project_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        };

        let mut used = std::collections::HashSet::new();
        for file in &files {
            if let Ok(source) = std::fs::read_to_string(file) {
                let imports =
                    lockpick::scanner::imports::extract_imports_from_source(&source, file);
                used.extend(imports);
            }
        }

        // Scan config files for plugin references
        let config_deps = lockpick::scanner::config::extract_config_deps(&project_path);
        used.extend(config_deps);

        // Scan extra config files from .lockpickrc
        let extra_deps = lockpick::scanner::config::extract_extra_config_deps(
            &project_path,
            &rc_config.extra_configs,
        );
        used.extend(extra_deps);

        // Scan package.json scripts for CLI tool references
        let script_deps = lockpick::scanner::scripts::extract_script_deps(&project_path);
        used.extend(script_deps);

        if cli.verbose {
            eprintln!("{}", i18n.t("scan_config_complete"));
        }

        let mut report = lockpick::scanner::unused::detect_unused(&graph, &used, effective_no_dev);

        // Apply --ignore filter
        if !effective_ignore.is_empty() {
            report.unused.retain(|dep| !effective_ignore.contains(&dep.name));
        }

        Some(report)
    } else {
        None
    };

    // Vulnerability scan
    let vulns = if run_audit {
        match lockpick::audit::osv::scan_vulnerabilities(&graph).await {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("{}: {e}", i18n.t("network_error"));
                None
            }
        }
    } else {
        None
    };

    // Duplicate detection
    let duplicates = Some(lockpick::analyzer::duplicates::detect_duplicates(&graph));

    // Build result and report
    // Size analysis
    let size = Some(lockpick::analyzer::size::analyze_size(
        &project_path,
        &graph,
    ));

    let result = lockpick::AnalysisResult {
        unused,
        vulns,
        duplicates,
        size,
    };

    if let Err(e) = reporter.report(&result, &i18n) {
        eprintln!("Report error: {e}");
        std::process::exit(1);
    }

    // Determine exit code for CI
    let has_unused = result.unused.as_ref().is_some_and(|u| !u.unused.is_empty());
    let has_vulns = result.vulns.as_ref().is_some_and(|v| !v.is_empty());

    if has_unused || has_vulns {
        std::process::exit(1);
    }
}
