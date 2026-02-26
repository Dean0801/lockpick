use std::collections::HashSet;
use std::path::Path;

use lockpick::AnalysisResult;
use lockpick::i18n::I18n;
use lockpick::lockfile::pnpm;
use lockpick::scanner;
use lockpick::scanner::imports::extract_imports_from_source;
use lockpick::scanner::unused::detect_unused;

const FIXTURE_DIR: &str = "tests/fixtures/sample-project";

/// End-to-end: lockfile parse → import scan → unused detection
#[test]
fn test_full_unused_detection_pipeline() {
    let project = Path::new(FIXTURE_DIR);

    // 1. Parse lockfile
    let lockfile_content = std::fs::read_to_string(project.join("pnpm-lock.yaml"))
        .expect("fixture lockfile should exist");
    let graph = pnpm::parse(&lockfile_content).expect("should parse lockfile");

    // 2. Scan source files for imports
    let files = scanner::discover_source_files(project).expect("should discover files");
    let mut used = HashSet::new();
    for file in &files {
        let source = std::fs::read_to_string(file).unwrap();
        let imports = extract_imports_from_source(&source, file);
        used.extend(imports);
    }

    // app.tsx imports react, utils.js imports lodash
    assert!(used.contains("react"), "should detect react import");
    assert!(used.contains("lodash"), "should detect lodash import");

    // 3. Detect unused
    let report = detect_unused(&graph, &used, false);

    // typescript is a dev dep not imported in source
    // @types/react is smart-associated with react (which IS used), so it should NOT be unused
    let unused_names: Vec<&str> = report.unused.iter().map(|d| d.name.as_str()).collect();
    assert!(
        unused_names.contains(&"typescript"),
        "typescript should be unused"
    );
    assert!(
        !unused_names.contains(&"@types/react"),
        "@types/react should NOT be unused when react is used"
    );
    assert!(
        !unused_names.contains(&"react"),
        "react should NOT be unused"
    );
    assert!(
        !unused_names.contains(&"lodash"),
        "lodash should NOT be unused"
    );
}

/// --no-dev flag should skip devDependencies from unused report
#[test]
fn test_unused_skip_dev() {
    let project = Path::new(FIXTURE_DIR);

    let lockfile_content = std::fs::read_to_string(project.join("pnpm-lock.yaml")).unwrap();
    let graph = pnpm::parse(&lockfile_content).unwrap();

    let files = scanner::discover_source_files(project).unwrap();
    let mut used = HashSet::new();
    for file in &files {
        let source = std::fs::read_to_string(file).unwrap();
        used.extend(extract_imports_from_source(&source, file));
    }

    // skip_dev = true
    let report = detect_unused(&graph, &used, true);

    let unused_names: Vec<&str> = report.unused.iter().map(|d| d.name.as_str()).collect();
    // Dev deps should NOT appear
    assert!(!unused_names.contains(&"typescript"));
    assert!(!unused_names.contains(&"@types/react"));
}

/// --ignore flag should filter out specified packages
#[test]
fn test_ignore_filter() {
    let project = Path::new(FIXTURE_DIR);

    let lockfile_content = std::fs::read_to_string(project.join("pnpm-lock.yaml")).unwrap();
    let graph = pnpm::parse(&lockfile_content).unwrap();

    // Pretend nothing is used
    let used = HashSet::new();
    let mut report = detect_unused(&graph, &used, false);

    // Simulate --ignore typescript
    let ignore = vec!["typescript".to_string()];
    report.unused.retain(|dep| !ignore.contains(&dep.name));

    let unused_names: Vec<&str> = report.unused.iter().map(|d| d.name.as_str()).collect();
    assert!(
        !unused_names.contains(&"typescript"),
        "ignored package should be filtered"
    );
    assert!(unused_names.contains(&"react"), "non-ignored should remain");
}

/// i18n detection respects CLI arg
#[test]
fn test_i18n_cli_override() {
    let i18n = I18n::detect(Some("zh"));
    assert_eq!(i18n.t("scan_complete"), "扫描完成");

    let i18n_en = I18n::detect(Some("en"));
    assert_eq!(i18n_en.t("scan_complete"), "Scan complete");
}

/// JSON reporter should produce valid JSON output
#[test]
fn test_json_reporter_output() {
    let result = AnalysisResult {
        unused: Some(lockpick::UnusedReport {
            unused: vec![lockpick::UnusedDep {
                name: "foo".into(),
                version: "1.0.0".into(),
                dep_type: lockpick::DepType::Prod,
            }],
        }),
        vulns: None,
        duplicates: None,
        size: None,
    };

    let json = serde_json::to_string_pretty(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed["unused"]["unused"].is_array());
    assert_eq!(parsed["unused"]["unused"][0]["name"], "foo");
}
