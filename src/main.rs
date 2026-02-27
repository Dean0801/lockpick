use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use lockpick::analyze::{AnalyzeOptions, analyze_package};
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

    /// Output format (defaults to terminal; overridden by .lockpickrc if not specified)
    #[arg(short, long, global = true)]
    format: Option<OutputFormat>,

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
fn read_package_deps(
    pkg_dir: &std::path::Path,
) -> Option<(
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
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Load .lockpickrc config
    let project_path = cli.path.clone().unwrap_or_else(|| PathBuf::from("."));

    if !project_path.exists() {
        eprintln!("Error: path '{}' does not exist", project_path.display());
        return ExitCode::FAILURE;
    }

    let rc_config = match load_config(&project_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Merge: CLI args override .lockpickrc
    let effective_no_dev = cli.no_dev || rc_config.skip_dev;

    let effective_ignore: HashSet<String> = {
        let mut merged: HashSet<String> = rc_config.ignore.iter().cloned().collect();
        merged.extend(cli.ignore.iter().cloned());
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
            return ExitCode::FAILURE;
        }
    };

    // Create reporter: CLI > .lockpickrc > default(terminal)
    let effective_format = cli.format.unwrap_or(match rc_config.format {
        Some(lockpick::config::types::OutputFormatConfig::Json) => OutputFormat::Json,
        _ => OutputFormat::Terminal,
    });
    let reporter: Box<dyn Reporter> = match effective_format {
        OutputFormat::Terminal => Box::new(TerminalReporter),
        OutputFormat::Json => Box::new(JsonReporter),
    };

    // Check if monorepo
    if lockpick::workspace::is_monorepo(&project_path) {
        let workspace_packages = match lockpick::workspace::detect_workspaces(&project_path) {
            Ok(pkgs) => pkgs,
            Err(e) => {
                eprintln!("Error detecting workspaces: {e}");
                return ExitCode::FAILURE;
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

            let opts = AnalyzeOptions {
                project_path: pkg_dir,
                graph: &pkg_graph,
                skip_dev: effective_no_dev,
                ignore: &effective_ignore,
                extra_configs: &rc_config.extra_configs,
                run_unused,
                run_duplicates: run_unused,
                run_size: run_unused,
                verbose: cli.verbose,
                i18n: &i18n,
            };

            match analyze_package(&opts) {
                Ok(pkg_result) => {
                    if pkg_result
                        .unused
                        .as_ref()
                        .is_some_and(|u| !u.unused.is_empty())
                    {
                        all_has_unused = true;
                    }
                    if let Err(e) = reporter.report(&pkg_result, &i18n) {
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
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    // --- Single package flow ---
    let opts = AnalyzeOptions {
        project_path: &project_path,
        graph: &graph,
        skip_dev: effective_no_dev,
        ignore: &effective_ignore,
        extra_configs: &rc_config.extra_configs,
        run_unused,
        run_duplicates: true,
        run_size: true,
        verbose: cli.verbose,
        i18n: &i18n,
    };

    let mut result = match analyze_package(&opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
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

    result.vulns = vulns;

    if let Err(e) = reporter.report(&result, &i18n) {
        eprintln!("Report error: {e}");
        return ExitCode::FAILURE;
    }

    // Determine exit code for CI
    let has_unused = result.unused.as_ref().is_some_and(|u| !u.unused.is_empty());
    let has_vulns = result.vulns.as_ref().is_some_and(|v| !v.is_empty());

    if has_unused || has_vulns {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
