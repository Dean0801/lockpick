use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use lockpick::config::load_config;
use lockpick::i18n::I18n;
use lockpick::report::markdown::MarkdownReporter;
use lockpick::report::{Reporter, json::JsonReporter, terminal::TerminalReporter};
use lockpick::runner::RunConfig;

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

    /// Dry run mode (for fix command)
    #[arg(long, global = true)]
    dry_run: bool,

    /// Disable OSV cache
    #[arg(long, global = true)]
    no_cache: bool,

    /// Write output to file instead of stdout
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,

    /// Exit code strategy: critical | high | any
    #[arg(long, global = true)]
    fail_on: Option<FailLevel>,

    /// Custom npm registry URL
    #[arg(long, global = true)]
    registry: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run full analysis (default)
    Scan,
    /// Detect unused dependencies only
    Unused,
    /// Vulnerability scan only
    Audit,
    /// Auto-remove unused dependencies
    Fix,
    /// Visualize dependency tree
    Tree {
        /// Output format: terminal | dot | json | mermaid
        #[arg(short, long, default_value = "terminal")]
        format: CliTreeFormat,
        /// Focus on a specific package
        #[arg(long)]
        focus: Option<String>,
        /// Limit tree depth
        #[arg(long)]
        depth: Option<usize>,
    },
    /// Compare against a baseline JSON file
    Diff {
        /// Path to baseline JSON file
        baseline: PathBuf,
        /// Output format: terminal | markdown
        #[arg(short, long, default_value = "terminal")]
        format: DiffFormat,
    },
    /// Check for outdated dependencies
    Outdated {
        /// Skip vulnerability correlation
        #[arg(long)]
        no_audit: bool,
        /// Filter by semver level: patch | minor | major
        #[arg(long)]
        level: Option<SemverLevelFilter>,
    },
    /// Supply chain security analysis
    SupplyChain,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Terminal,
    Json,
    Markdown,
}

#[derive(Clone, ValueEnum)]
enum CliTreeFormat {
    Terminal,
    Dot,
    Json,
    Mermaid,
}

#[derive(Clone, ValueEnum)]
enum DiffFormat {
    Terminal,
    Markdown,
}

#[derive(Clone, ValueEnum)]
enum FailLevel {
    Critical,
    High,
    Any,
}

#[derive(Clone, ValueEnum)]
enum LangOption {
    En,
    Zh,
}

#[derive(Clone, ValueEnum)]
enum SemverLevelFilter {
    Patch,
    Minor,
    Major,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
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

    if cli.verbose {
        eprintln!("{}", i18n.t("analyzing"));
    }

    // Handle tree subcommand (separate pipeline with dep_edges)
    if let Some(Commands::Tree {
        format,
        focus,
        depth,
    }) = &cli.command
    {
        let graph = match lockpick::lockfile::detect_and_parse_with_edges(&project_path) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        };
        let tree_fmt = match format {
            CliTreeFormat::Terminal => lockpick::tree::render::TreeFormat::Terminal,
            CliTreeFormat::Dot => lockpick::tree::render::TreeFormat::Dot,
            CliTreeFormat::Json => lockpick::tree::render::TreeFormat::Json,
            CliTreeFormat::Mermaid => lockpick::tree::render::TreeFormat::Mermaid,
        };
        let dep_tree = lockpick::tree::DepTree::from_graph_with_depth(&graph, *depth);
        let tree = match focus {
            Some(pkg) => dep_tree.focus(pkg),
            None => dep_tree,
        };
        let out = lockpick::tree::render::render(&tree, &tree_fmt, &i18n);
        return write_output(&cli.output, &out);
    }

    // Parse lockfile (standard pipeline)
    let graph = match lockpick::lockfile::detect_and_parse(&project_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Handle diff subcommand (separate pipeline)
    if let Some(Commands::Diff { baseline, format }) = &cli.command {
        let opts = lockpick::analyze::AnalyzeOptions {
            project_path: &project_path,
            graph: &graph,
            skip_dev: cli.no_dev || rc_config.skip_dev,
            ignore: &effective_ignore,
            extra_configs: &rc_config.extra_configs,
            run_unused: true,
            run_duplicates: true,
            run_size: false,
            run_license: rc_config.license.is_some(),
            license_policy: rc_config.license.as_ref(),
            verbose: cli.verbose,
            i18n: &i18n,
        };
        let current = match lockpick::analyze::analyze_package(&opts) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error: {e}");
                return ExitCode::from(2);
            }
        };
        let report = match lockpick::diff::compute_diff(baseline, &current) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error loading baseline: {e}");
                return ExitCode::from(2);
            }
        };
        let out = match format {
            DiffFormat::Terminal => lockpick::diff::render_terminal(&report, &i18n),
            DiffFormat::Markdown => lockpick::diff::render_markdown(&report, &i18n),
        };
        return write_output(&cli.output, &out);
    }

    // Handle outdated subcommand
    if let Some(Commands::Outdated { no_audit, level }) = &cli.command {
        let vulns = if *no_audit {
            None
        } else {
            lockpick::runner::run_audit(&graph, rc_config.cache_ttl, cli.no_cache, &i18n).await
        };
        let registry_url = cli.registry.as_deref().or(rc_config.registry.as_deref());
        let report = match lockpick::outdated::check_outdated(
            &graph,
            vulns.as_deref(),
            cli.no_dev || rc_config.skip_dev,
            registry_url,
            cli.no_cache,
            rc_config.cache_ttl,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        };
        let result = lockpick::AnalysisResult {
            unused: None,
            vulns: None,
            duplicates: None,
            size: None,
            license: None,
            outdated: Some(filter_outdated(report, level)),
            supply_chain: None,
        };
        let effective_format = cli.format.unwrap_or(match rc_config.format {
            Some(lockpick::config::types::OutputFormatConfig::Json) => OutputFormat::Json,
            Some(lockpick::config::types::OutputFormatConfig::Markdown) => OutputFormat::Markdown,
            _ => OutputFormat::Terminal,
        });
        let reporter: Box<dyn Reporter> = match effective_format {
            OutputFormat::Terminal => Box::new(TerminalReporter),
            OutputFormat::Json => Box::new(JsonReporter),
            OutputFormat::Markdown => Box::new(MarkdownReporter),
        };
        if let Err(e) = reporter.report(&result, &i18n) {
            eprintln!("Report error: {e}");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    // Handle supply-chain subcommand
    if let Some(Commands::SupplyChain) = &cli.command {
        let report = lockpick::supply_chain::analyze(&graph);
        let result = lockpick::AnalysisResult {
            unused: None,
            vulns: None,
            duplicates: None,
            size: None,
            license: None,
            outdated: None,
            supply_chain: Some(report),
        };
        let effective_format = cli.format.unwrap_or(match rc_config.format {
            Some(lockpick::config::types::OutputFormatConfig::Json) => OutputFormat::Json,
            Some(lockpick::config::types::OutputFormatConfig::Markdown) => OutputFormat::Markdown,
            _ => OutputFormat::Terminal,
        });
        let reporter: Box<dyn Reporter> = match effective_format {
            OutputFormat::Terminal => Box::new(TerminalReporter),
            OutputFormat::Json => Box::new(JsonReporter),
            OutputFormat::Markdown => Box::new(MarkdownReporter),
        };
        if let Err(e) = reporter.report(&result, &i18n) {
            eprintln!("Report error: {e}");
            return ExitCode::FAILURE;
        }
        let has_high = result.supply_chain.as_ref().is_some_and(|sc| {
            sc.risks.iter().any(|r| {
                matches!(
                    r.severity,
                    lockpick::Severity::High | lockpick::Severity::Critical
                )
            })
        });
        return if has_high {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        };
    }

    // Standard analysis pipeline
    let (run_unused, run_audit, run_fix, run_supply_chain) = match &cli.command {
        Some(Commands::Unused) => (true, false, false, false),
        Some(Commands::Audit) => (false, true, false, false),
        Some(Commands::Fix) => (true, false, true, false),
        _ => (true, true, false, true), // scan: all enabled
    };

    let effective_format = cli.format.unwrap_or(match rc_config.format {
        Some(lockpick::config::types::OutputFormatConfig::Json) => OutputFormat::Json,
        Some(lockpick::config::types::OutputFormatConfig::Markdown) => OutputFormat::Markdown,
        _ => OutputFormat::Terminal,
    });
    let reporter: Box<dyn Reporter> = match effective_format {
        OutputFormat::Terminal => Box::new(TerminalReporter),
        OutputFormat::Json => Box::new(JsonReporter),
        OutputFormat::Markdown => Box::new(MarkdownReporter),
    };

    let cfg = RunConfig {
        skip_dev: cli.no_dev || rc_config.skip_dev,
        ignore: &effective_ignore,
        extra_configs: &rc_config.extra_configs,
        license_policy: rc_config.license.as_ref(),
        run_unused,
        run_audit,
        run_fix,
        run_supply_chain,
        verbose: cli.verbose,
        dry_run: cli.dry_run,
        no_cache: cli.no_cache,
        cache_ttl: rc_config.cache_ttl,
    };

    // When --output is specified, use a no-op reporter (we'll write to file after)
    let noop_reporter = lockpick::report::NoopReporter;
    let active_reporter: &dyn Reporter = if cli.output.is_some() {
        &noop_reporter
    } else {
        &*reporter
    };

    let (has_issues, analysis_result) = if lockpick::workspace::is_monorepo(&project_path) {
        (
            lockpick::runner::run_monorepo(&project_path, &graph, &cfg, &i18n, active_reporter)
                .await,
            None,
        )
    } else {
        lockpick::runner::run_single(&project_path, &graph, &cfg, &i18n, active_reporter).await
    };

    // Write to file when --output is specified
    if let (Some(output_path), Some(result)) = (&cli.output, &analysis_result) {
        let content = match effective_format {
            OutputFormat::Json => serde_json::to_string_pretty(result).unwrap_or_default(),
            OutputFormat::Markdown => MarkdownReporter.render(result, &i18n),
            OutputFormat::Terminal => MarkdownReporter.render(result, &i18n),
        };
        if let Err(e) = std::fs::write(output_path, content) {
            eprintln!("Error writing to {}: {e}", output_path.display());
            return ExitCode::from(2);
        }
    }

    // Threshold evaluation: --fail-on CLI or .lockpickrc thresholds
    let threshold_exceeded = if let Some(result) = &analysis_result {
        let thresholds = cli
            .fail_on
            .as_ref()
            .map(|level| {
                let s = match level {
                    FailLevel::Critical => "critical",
                    FailLevel::High => "high",
                    FailLevel::Any => "any",
                };
                lockpick::threshold::from_fail_on(s)
            })
            .or(rc_config.thresholds.clone());
        thresholds.is_some_and(|t| lockpick::threshold::evaluate(result, &t))
    } else {
        false
    };

    if has_issues || threshold_exceeded {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn write_output(path: &Option<PathBuf>, content: &str) -> ExitCode {
    if let Some(p) = path {
        if let Err(e) = std::fs::write(p, content) {
            eprintln!("Error writing to {}: {e}", p.display());
            return ExitCode::from(2);
        }
    } else {
        print!("{content}");
    }
    ExitCode::SUCCESS
}

fn filter_outdated(
    mut report: lockpick::OutdatedReport,
    level: &Option<SemverLevelFilter>,
) -> lockpick::OutdatedReport {
    if let Some(filter) = level {
        let target = match filter {
            SemverLevelFilter::Patch => lockpick::SemverLevel::Patch,
            SemverLevelFilter::Minor => lockpick::SemverLevel::Minor,
            SemverLevelFilter::Major => lockpick::SemverLevel::Major,
        };
        report.entries.retain(|e| e.level == target);
        report.total_outdated = report.entries.len();
        report.patch_count = report
            .entries
            .iter()
            .filter(|e| e.level == lockpick::SemverLevel::Patch)
            .count();
        report.minor_count = report
            .entries
            .iter()
            .filter(|e| e.level == lockpick::SemverLevel::Minor)
            .count();
        report.major_count = report
            .entries
            .iter()
            .filter(|e| e.level == lockpick::SemverLevel::Major)
            .count();
    }
    report
}
