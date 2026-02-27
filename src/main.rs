use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Detect language
    let lang_str = cli.lang.as_ref().map(|l| match l {
        LangOption::En => "en",
        LangOption::Zh => "zh",
    });
    let i18n = I18n::detect(lang_str);

    // Resolve project path
    let project_path = cli.path.unwrap_or_else(|| PathBuf::from("."));
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

        let mut report = lockpick::scanner::unused::detect_unused(&graph, &used, cli.no_dev);

        // Apply --ignore filter
        if !cli.ignore.is_empty() {
            report.unused.retain(|dep| !cli.ignore.contains(&dep.name));
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

    let reporter: Box<dyn Reporter> = match cli.format {
        OutputFormat::Terminal => Box::new(TerminalReporter),
        OutputFormat::Json => Box::new(JsonReporter),
    };

    if let Err(e) = reporter.report(&result, &i18n) {
        eprintln!("Report error: {e}");
        std::process::exit(1);
    }

    // Determine exit code for CI
    let has_unused = result
        .unused
        .as_ref()
        .is_some_and(|u| !u.unused.is_empty());
    let has_vulns = result
        .vulns
        .as_ref()
        .is_some_and(|v| !v.is_empty());

    if has_unused || has_vulns {
        std::process::exit(1);
    }
}
