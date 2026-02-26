use crate::{DependencyGraph, DuplicateDep, DuplicateReport};

/// Detect packages that have multiple versions installed
pub fn detect_duplicates(graph: &DependencyGraph) -> DuplicateReport {
    let mut duplicates: Vec<DuplicateDep> = graph
        .all_packages
        .iter()
        .filter(|(_, versions)| versions.len() > 1)
        .map(|(name, versions)| {
            let mut sorted_versions = versions.clone();
            sorted_versions.sort();
            DuplicateDep {
                name: name.clone(),
                versions: sorted_versions,
            }
        })
        .collect();

    // Sort by name for stable output
    duplicates.sort_by(|a, b| a.name.cmp(&b.name));
    let total = duplicates.len();

    DuplicateReport {
        duplicates,
        total_duplicate_packages: total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DependencyGraph, LockfileType};
    use std::collections::HashMap;

    #[test]
    fn test_detect_duplicates_found() {
        let mut all_packages = HashMap::new();
        all_packages.insert(
            "lodash".to_string(),
            vec!["4.17.20".to_string(), "4.17.21".to_string()],
        );
        all_packages.insert("react".to_string(), vec!["18.2.0".to_string()]);

        let graph = DependencyGraph {
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            lockfile_type: LockfileType::Pnpm,
            all_packages,
        };

        let report = detect_duplicates(&graph);
        assert_eq!(report.total_duplicate_packages, 1);
        assert_eq!(report.duplicates.len(), 1);
        assert_eq!(report.duplicates[0].name, "lodash");
        assert_eq!(report.duplicates[0].versions.len(), 2);
    }

    #[test]
    fn test_detect_no_duplicates() {
        let mut all_packages = HashMap::new();
        all_packages.insert("react".to_string(), vec!["18.2.0".to_string()]);
        all_packages.insert("lodash".to_string(), vec!["4.17.21".to_string()]);

        let graph = DependencyGraph {
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            lockfile_type: LockfileType::Pnpm,
            all_packages,
        };

        let report = detect_duplicates(&graph);
        assert_eq!(report.total_duplicate_packages, 0);
        assert!(report.duplicates.is_empty());
    }
}
