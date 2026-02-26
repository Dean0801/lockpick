use std::path::Path;
use crate::{DependencyGraph, SizeEntry, SizeReport};

/// Analyze disk size of each dependency in node_modules
pub fn analyze_size(project_path: &Path, graph: &DependencyGraph) -> SizeReport {
    let node_modules = project_path.join("node_modules");
    let mut entries = Vec::new();

    // Collect all package names from deps + dev_deps
    let all_names: Vec<&String> = graph.dependencies.keys()
        .chain(graph.dev_dependencies.keys())
        .collect();

    for name in all_names {
        let pkg_path = node_modules.join(name);
        let size = dir_size(&pkg_path);
        entries.push(SizeEntry {
            name: name.clone(),
            size_bytes: size,
        });
    }

    // Sort by size descending
    entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    let total = entries.iter().map(|e| e.size_bytes).sum();

    SizeReport {
        entries,
        total_bytes: total,
    }
}

/// Recursively calculate directory size in bytes
fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let meta = entry.metadata();
            if let Ok(meta) = meta {
                if meta.is_file() {
                    total += meta.len();
                } else if meta.is_dir() {
                    total += dir_size(&entry.path());
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use crate::{DependencyGraph, PackageInfo, LockfileType};

    #[test]
    fn test_analyze_size_with_packages() {
        // Create temp node_modules with fake packages
        let tmp = std::env::temp_dir().join("lockpick_size_test");
        let nm = tmp.join("node_modules");
        let react_dir = nm.join("react");
        fs::create_dir_all(&react_dir).unwrap();
        fs::write(react_dir.join("index.js"), "a".repeat(1000)).unwrap();
        fs::write(react_dir.join("package.json"), "b".repeat(500)).unwrap();

        let mut deps = HashMap::new();
        deps.insert("react".to_string(), PackageInfo {
            name: "react".to_string(),
            version: "18.2.0".to_string(),
            integrity: None,
        });

        let graph = DependencyGraph {
            dependencies: deps,
            dev_dependencies: HashMap::new(),
            lockfile_type: LockfileType::Npm,
            all_packages: HashMap::new(),
        };

        let report = analyze_size(&tmp, &graph);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].name, "react");
        assert_eq!(report.entries[0].size_bytes, 1500);
        assert_eq!(report.total_bytes, 1500);

        // Cleanup
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_analyze_size_missing_dir() {
        let tmp = std::env::temp_dir().join("lockpick_size_missing");
        let mut deps = HashMap::new();
        deps.insert("nonexistent".to_string(), PackageInfo {
            name: "nonexistent".to_string(),
            version: "1.0.0".to_string(),
            integrity: None,
        });

        let graph = DependencyGraph {
            dependencies: deps,
            dev_dependencies: HashMap::new(),
            lockfile_type: LockfileType::Npm,
            all_packages: HashMap::new(),
        };

        let report = analyze_size(&tmp, &graph);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].size_bytes, 0);
        assert_eq!(report.total_bytes, 0);
    }

    #[test]
    fn test_analyze_size_scoped_package() {
        let tmp = std::env::temp_dir().join("lockpick_size_scoped");
        let nm = tmp.join("node_modules").join("@types").join("react");
        fs::create_dir_all(&nm).unwrap();
        fs::write(nm.join("index.d.ts"), "c".repeat(200)).unwrap();

        let mut dev_deps = HashMap::new();
        dev_deps.insert("@types/react".to_string(), PackageInfo {
            name: "@types/react".to_string(),
            version: "18.2.48".to_string(),
            integrity: None,
        });

        let graph = DependencyGraph {
            dependencies: HashMap::new(),
            dev_dependencies: dev_deps,
            lockfile_type: LockfileType::Npm,
            all_packages: HashMap::new(),
        };

        let report = analyze_size(&tmp, &graph);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].name, "@types/react");
        assert_eq!(report.entries[0].size_bytes, 200);

        fs::remove_dir_all(&tmp).ok();
    }
}
