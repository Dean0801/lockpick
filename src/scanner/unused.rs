use std::collections::HashSet;

use crate::{DepType, DependencyGraph, UnusedDep, UnusedReport};

/// Extract the base package name from a `@types/X` package.
/// e.g. `@types/react` → `Some("react")`, `@types/babel__core` → `Some("@babel/core")`
/// Returns `None` for non-@types packages.
fn get_types_base(name: &str) -> Option<String> {
    let suffix = name.strip_prefix("@types/")?;
    if suffix.contains("__") {
        // Scoped package: @types/babel__core → @babel/core
        let (scope, pkg) = suffix.split_once("__")?;
        Some(format!("@{scope}/{pkg}"))
    } else {
        Some(suffix.to_string())
    }
}

/// Check if a dependency version uses the workspace protocol (monorepo internal package)
fn is_workspace_dep(version: &str) -> bool {
    version.starts_with("workspace:")
}

/// Detect unused dependencies by comparing declared deps vs actual imports
pub fn detect_unused(
    graph: &DependencyGraph,
    used: &HashSet<String>,
    skip_dev: bool,
) -> UnusedReport {
    let mut unused = Vec::new();

    for (name, info) in &graph.dependencies {
        // Skip workspace internal packages — they should never be removed
        if is_workspace_dep(&info.version) {
            continue;
        }
        if !used.contains(name.as_str()) {
            unused.push(UnusedDep {
                name: name.clone(),
                version: info.version.clone(),
                dep_type: DepType::Prod,
            });
        }
    }

    if !skip_dev {
        for (name, info) in &graph.dev_dependencies {
            // Skip workspace internal packages
            if is_workspace_dep(&info.version) {
                continue;
            }
            if !used.contains(name.as_str()) {
                // Smart @types/* association: if the base package is used, skip
                if let Some(base) = get_types_base(name)
                    && used.contains(base.as_str())
                {
                    continue;
                }
                unused.push(UnusedDep {
                    name: name.clone(),
                    version: info.version.clone(),
                    dep_type: DepType::Dev,
                });
            }
        }
    }

    // Sort by name for stable output
    unused.sort_by(|a, b| a.name.cmp(&b.name));

    UnusedReport { unused }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PackageInfo;
    use std::collections::HashMap;

    fn make_graph() -> DependencyGraph {
        let mut deps = HashMap::new();
        deps.insert(
            "react".into(),
            PackageInfo {
                name: "react".into(),
                version: "18.2.0".into(),
                integrity: None,
            },
        );
        deps.insert(
            "lodash".into(),
            PackageInfo {
                name: "lodash".into(),
                version: "4.17.21".into(),
                integrity: None,
            },
        );

        let mut dev = HashMap::new();
        dev.insert(
            "typescript".into(),
            PackageInfo {
                name: "typescript".into(),
                version: "5.3.3".into(),
                integrity: None,
            },
        );

        DependencyGraph {
            dependencies: deps,
            dev_dependencies: dev,
            lockfile_type: crate::LockfileType::Pnpm,
            all_packages: HashMap::new(),
            dep_edges: HashMap::new(),
        }
    }

    #[test]
    fn test_all_used() {
        let graph = make_graph();
        let used: HashSet<String> = ["react", "lodash", "typescript"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let report = detect_unused(&graph, &used, false);
        assert!(report.unused.is_empty());
    }

    #[test]
    fn test_some_unused() {
        let graph = make_graph();
        let used: HashSet<String> = ["react"].iter().map(|s| s.to_string()).collect();
        let report = detect_unused(&graph, &used, false);
        assert_eq!(report.unused.len(), 2);
    }

    #[test]
    fn test_skip_dev() {
        let graph = make_graph();
        let used: HashSet<String> = ["react"].iter().map(|s| s.to_string()).collect();
        let report = detect_unused(&graph, &used, true);
        // Only lodash (prod) should be unused, typescript (dev) skipped
        assert_eq!(report.unused.len(), 1);
        assert_eq!(report.unused[0].name, "lodash");
    }

    #[test]
    fn test_types_smart_association() {
        let mut graph = make_graph();
        graph.dev_dependencies.insert(
            "@types/react".into(),
            PackageInfo {
                name: "@types/react".into(),
                version: "18.2.0".into(),
                integrity: None,
            },
        );

        // react is used, so @types/react should NOT be marked as unused
        let used: HashSet<String> = ["react"].iter().map(|s| s.to_string()).collect();
        let report = detect_unused(&graph, &used, false);

        let unused_names: Vec<&str> = report.unused.iter().map(|d| d.name.as_str()).collect();
        assert!(
            !unused_names.contains(&"@types/react"),
            "@types/react should not be unused when react is used"
        );
        // lodash (prod) and typescript (dev) should still be unused
        assert!(unused_names.contains(&"lodash"));
        assert!(unused_names.contains(&"typescript"));
    }

    #[test]
    fn test_workspace_deps_excluded() {
        let mut graph = make_graph();
        graph.dependencies.insert(
            "@vben/hooks".into(),
            PackageInfo {
                name: "@vben/hooks".into(),
                version: "workspace:*".into(),
                integrity: None,
            },
        );
        graph.dev_dependencies.insert(
            "@vben/utils".into(),
            PackageInfo {
                name: "@vben/utils".into(),
                version: "workspace:^".into(),
                integrity: None,
            },
        );

        let used: HashSet<String> = ["react"].iter().map(|s| s.to_string()).collect();
        let report = detect_unused(&graph, &used, false);
        let unused_names: Vec<&str> = report.unused.iter().map(|d| d.name.as_str()).collect();

        assert!(!unused_names.contains(&"@vben/hooks"), "workspace dep should be excluded");
        assert!(!unused_names.contains(&"@vben/utils"), "workspace dev dep should be excluded");
        assert!(unused_names.contains(&"lodash"));
    }
}
