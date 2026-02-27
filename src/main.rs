use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use lockpick::config::load_config;
use lockpick::i18n::I18n;
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

    // Determine which analyses to run
    let (run_unused, run_audit, run_fix) = match &cli.command {
        Some(Commands::Unused) => (true, false, false),
        Some(Commands::Audit) => (false, true, false),
        Some(Commands::Fix) => (true, false, true),
        Some(Commands::Scan) | None => (true, true, false),
    };

    if cli.verbose {
        eprintln!("{}", i18n.t("analyzing"));
    }

    // Parse lockfile
    let graph = match lockpick::lockfile::detect_and_parse(&project_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Create reporter
    let effective_format = cli.format.unwrap_or(match rc_config.format {
        Some(lockpick::config::types::OutputFormatConfig::Json) => OutputFormat::Json,
        _ => OutputFormat::Terminal,
    });
    let reporter: Box<dyn Reporter> = match effective_format {
        OutputFormat::Terminal => Box::new(TerminalReporter),
        OutputFormat::Json => Box::new(JsonReporter),
    };

    let cfg = RunConfig {
        skip_dev: cli.no_dev || rc_config.skip_dev,
        ignore: &effective_ignore,
        extra_configs: &rc_config.extra_configs,
        license_policy: rc_config.license.as_ref(),
        run_unused,
        run_audit,
        run_fix,
        verbose: cli.verbose,
        dry_run: cli.dry_run,
        no_cache: cli.no_cache,
        cache_ttl: rc_config.cache_ttl,
    };

    let has_issues = if lockpick::workspace::is_monorepo(&project_path) {
        lockpick::runner::run_monorepo(&project_path, &graph, &cfg, &i18n, &*reporter).await
    } else {
        lockpick::runner::run_single(&project_path, &graph, &cfg, &i18n, &*reporter).await
    };

    if has_issues {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
