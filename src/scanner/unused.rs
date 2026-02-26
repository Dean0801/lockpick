use std::collections::HashSet;

use crate::{DepType, DependencyGraph, UnusedDep, UnusedReport};

/// Detect unused dependencies by comparing declared deps vs actual imports
pub fn detect_unused(
    graph: &DependencyGraph,
    used: &HashSet<String>,
    skip_dev: bool,
) -> UnusedReport {
    let mut unused = Vec::new();

    for (name, info) in &graph.dependencies {
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
            if !used.contains(name.as_str()) {
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
}
