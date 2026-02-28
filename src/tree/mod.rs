pub mod render;

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::{DepEdge, DepType, DependencyGraph};

/// A node in the dependency tree
#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    pub name: String,
    pub version: String,
    pub dep_type: DepType,
    pub children: Vec<TreeNode>,
    pub circular: bool,
}

/// The complete dependency tree
#[derive(Debug, Clone, Serialize)]
pub struct DepTree {
    pub roots: Vec<TreeNode>,
}

impl DepTree {
    /// Build dependency tree from DependencyGraph with optional depth limit.
    pub fn from_graph_with_depth(graph: &DependencyGraph, max_depth: Option<usize>) -> Self {
        let mut roots = Vec::new();

        for (name, info) in &graph.dependencies {
            let mut visited = HashSet::new();
            roots.push(build_node(
                name,
                &info.version,
                DepType::Prod,
                &graph.dep_edges,
                &mut visited,
                max_depth,
                0,
            ));
        }

        for (name, info) in &graph.dev_dependencies {
            let mut visited = HashSet::new();
            roots.push(build_node(
                name,
                &info.version,
                DepType::Dev,
                &graph.dep_edges,
                &mut visited,
                max_depth,
                0,
            ));
        }

        roots.sort_by(|a, b| a.name.cmp(&b.name));
        DepTree { roots }
    }

    /// Build dependency tree from DependencyGraph (no depth limit).
    pub fn from_graph(graph: &DependencyGraph) -> Self {
        Self::from_graph_with_depth(graph, None)
    }

    /// Focus mode: keep only paths containing the specified package.
    pub fn focus(&self, package: &str) -> Self {
        let roots = self.roots.iter().filter_map(|r| filter_node(r, package)).collect();
        DepTree { roots }
    }
}

fn build_node(
    name: &str,
    version: &str,
    dep_type: DepType,
    edges: &HashMap<String, Vec<DepEdge>>,
    visited: &mut HashSet<String>,
    max_depth: Option<usize>,
    depth: usize,
) -> TreeNode {
    let pkg_key = format!("{name}@{version}");

    if visited.contains(&pkg_key) {
        return TreeNode {
            name: name.to_string(),
            version: version.to_string(),
            dep_type,
            children: vec![],
            circular: true,
        };
    }

    if max_depth.is_some_and(|d| depth >= d) {
        return TreeNode {
            name: name.to_string(),
            version: version.to_string(),
            dep_type,
            children: vec![],
            circular: false,
        };
    }

    visited.insert(pkg_key.clone());

    let children = edges
        .get(&pkg_key)
        .map(|deps| {
            deps.iter()
                .map(|e| build_node(&e.name, &e.version, DepType::Prod, edges, visited, max_depth, depth + 1))
                .collect()
        })
        .unwrap_or_default();

    visited.remove(&pkg_key);

    TreeNode {
        name: name.to_string(),
        version: version.to_string(),
        dep_type,
        children,
        circular: false,
    }
}

fn filter_node(node: &TreeNode, target: &str) -> Option<TreeNode> {
    if node.name == target {
        return Some(node.clone());
    }
    let filtered: Vec<TreeNode> = node.children.iter().filter_map(|c| filter_node(c, target)).collect();
    if filtered.is_empty() {
        return None;
    }
    Some(TreeNode {
        name: node.name.clone(),
        version: node.version.clone(),
        dep_type: node.dep_type.clone(),
        children: filtered,
        circular: false,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{DependencyGraph, LockfileType, PackageInfo};
    use std::collections::HashMap;

    pub(crate) fn make_test_graph() -> DependencyGraph {
        let mut deps = HashMap::new();
        deps.insert(
            "react".into(),
            PackageInfo {
                name: "react".into(),
                version: "18.2.0".into(),
                integrity: None,
            },
        );

        let mut dep_edges = HashMap::new();
        dep_edges.insert(
            "react@18.2.0".into(),
            vec![DepEdge {
                name: "loose-envify".into(),
                version: "1.4.0".into(),
            }],
        );
        dep_edges.insert(
            "loose-envify@1.4.0".into(),
            vec![DepEdge {
                name: "js-tokens".into(),
                version: "4.0.0".into(),
            }],
        );

        DependencyGraph {
            dependencies: deps,
            dev_dependencies: HashMap::new(),
            lockfile_type: LockfileType::Pnpm,
            all_packages: HashMap::new(),
            dep_edges,
        }
    }

    #[test]
    fn test_from_graph_builds_tree() {
        let graph = make_test_graph();
        let tree = DepTree::from_graph(&graph);
        assert_eq!(tree.roots.len(), 1);
        assert_eq!(tree.roots[0].name, "react");
        assert_eq!(tree.roots[0].children.len(), 1);
        assert_eq!(tree.roots[0].children[0].name, "loose-envify");
        let leaf = &tree.roots[0].children[0].children[0];
        assert_eq!(leaf.name, "js-tokens");
        assert!(leaf.children.is_empty());
    }

    #[test]
    fn test_focus_filters_tree() {
        let graph = make_test_graph();
        let tree = DepTree::from_graph(&graph);
        let focused = tree.focus("js-tokens");
        assert_eq!(focused.roots.len(), 1);
        assert_eq!(focused.roots[0].name, "react");
        let leaf = &focused.roots[0].children[0].children[0];
        assert_eq!(leaf.name, "js-tokens");
    }

    #[test]
    fn test_focus_nonexistent_returns_empty() {
        let graph = make_test_graph();
        let tree = DepTree::from_graph(&graph);
        let focused = tree.focus("nonexistent-pkg");
        assert!(focused.roots.is_empty());
    }

    #[test]
    fn test_circular_dependency_detected() {
        let mut graph = make_test_graph();
        graph.dep_edges.insert(
            "js-tokens@4.0.0".into(),
            vec![DepEdge {
                name: "react".into(),
                version: "18.2.0".into(),
            }],
        );
        let tree = DepTree::from_graph(&graph);
        let circular_node = &tree.roots[0].children[0].children[0].children[0];
        assert!(circular_node.circular);
        assert_eq!(circular_node.name, "react");
    }
}
