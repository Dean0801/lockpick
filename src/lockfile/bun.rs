use std::collections::HashMap;

use crate::error::LockpickError;
use crate::utils::strip_jsonc_comments;
use crate::{DepEdge, DependencyGraph, LockfileType, PackageInfo};

/// Resolved package info: (version, optional integrity hash)
type ResolvedMap = HashMap<String, (String, Option<String>)>;

/// Parse the `name@version` format used in bun.lock package entries.
/// Handles scoped packages like `@scope/pkg@1.0.0`.
fn parse_name_version(s: &str) -> Option<(&str, &str)> {
    let search_start = if s.starts_with('@') { 1 } else { 0 };
    let at_pos = s[search_start..].rfind('@').map(|p| p + search_start)?;
    if at_pos == 0 {
        return None;
    }
    let name = &s[..at_pos];
    let version = &s[at_pos + 1..];
    if version.is_empty() {
        return None;
    }
    Some((name, version))
}

/// Build a mapping from package name to (resolved_version, integrity) from the
/// "packages" section of bun.lock. The integrity hash is extracted from the
/// third element of each package array, if present.
fn build_resolved_map(
    root: &serde_json::Value,
) -> (HashMap<String, Vec<String>>, ResolvedMap) {
    let mut all_packages: HashMap<String, Vec<String>> = HashMap::new();
    let mut resolved: ResolvedMap = HashMap::new();

    if let Some(packages) = root.get("packages").and_then(|p| p.as_object()) {
        for (_key, value) in packages {
            if let Some(arr) = value.as_array()
                && let Some(first) = arr.first().and_then(|v| v.as_str())
                && let Some((name, version)) = parse_name_version(first)
            {
                let integrity = arr.get(2).and_then(|v| v.as_str()).map(String::from);
                all_packages
                    .entry(name.to_string())
                    .or_default()
                    .push(version.to_string());
                resolved.insert(name.to_string(), (version.to_string(), integrity));
            }
        }
    }

    (all_packages, resolved)
}

/// Backfill resolved versions from the packages map into dependency entries.
/// Replaces specifiers (e.g. "^18.2.0") with actual resolved versions (e.g. "18.2.0").
fn backfill_versions(
    target: &mut HashMap<String, PackageInfo>,
    resolved: &ResolvedMap,
) {
    for (name, info) in target.iter_mut() {
        if let Some((version, integrity)) = resolved.get(name.as_str()) {
            info.version = version.clone();
            if info.integrity.is_none() {
                info.integrity = integrity.clone();
            }
        }
    }
}

/// Parse bun.lock (JSONC format) content into a DependencyGraph.
/// Supports monorepo workspaces by merging all workspace entries.
pub fn parse(content: &str) -> Result<DependencyGraph, LockpickError> {
    let stripped = strip_jsonc_comments(content);
    let root: serde_json::Value = serde_json::from_str(&stripped)
        .map_err(|e| LockpickError::Parse(format!("Failed to parse bun.lock: {e}")))?;

    let mut deps = HashMap::new();
    let mut dev_deps = HashMap::new();

    // Extract dependencies from all workspaces (root "" and sub-workspaces)
    if let Some(workspaces) = root.get("workspaces").and_then(|w| w.as_object()) {
        for (key, workspace) in workspaces {
            if key.is_empty() {
                // Root workspace: deps go into main deps/dev_deps
                extract_deps(workspace, "dependencies", &mut deps);
                extract_deps(workspace, "devDependencies", &mut dev_deps);
            } else {
                // Sub-workspace: merge into main deps
                extract_deps(workspace, "dependencies", &mut deps);
                extract_deps(workspace, "devDependencies", &mut dev_deps);
            }
        }
    }

    // Build resolved version map and backfill specifiers with actual versions
    let (all_packages, resolved) = build_resolved_map(&root);
    backfill_versions(&mut deps, &resolved);
    backfill_versions(&mut dev_deps, &resolved);

    Ok(DependencyGraph {
        dependencies: deps,
        dev_dependencies: dev_deps,
        lockfile_type: LockfileType::Bun,
        all_packages,
        dep_edges: HashMap::new(),
    })
}

/// Parse bun.lock with transitive dependency edges.
pub fn parse_with_edges(content: &str) -> Result<DependencyGraph, LockpickError> {
    let stripped = strip_jsonc_comments(content);
    let root: serde_json::Value = serde_json::from_str(&stripped)
        .map_err(|e| LockpickError::Parse(format!("Failed to parse bun.lock: {e}")))?;
    let mut graph = parse(content)?;

    if let Some(packages) = root.get("packages").and_then(|p| p.as_object()) {
        for (_key, value) in packages {
            let Some(arr) = value.as_array() else { continue };
            let Some(first) = arr.first().and_then(|v| v.as_str()) else { continue };
            let Some((name, ver)) = parse_name_version(first) else { continue };
            let Some(deps_obj) = arr.get(3).and_then(|v| v.as_object()) else { continue };
            let edges: Vec<DepEdge> = deps_obj
                .iter()
                .map(|(n, v)| DepEdge {
                    name: n.clone(),
                    version: v.as_str().unwrap_or("*").to_string(),
                })
                .collect();
            if !edges.is_empty() {
                graph.dep_edges.insert(format!("{name}@{ver}"), edges);
            }
        }
    }
    Ok(graph)
}

/// Extract dependency entries from a workspace object field into the target map.
fn extract_deps(
    workspace: &serde_json::Value,
    field: &str,
    target: &mut HashMap<String, PackageInfo>,
) {
    if let Some(obj) = workspace.get(field).and_then(|d| d.as_object()) {
        for (name, version_val) in obj {
            let version = match version_val.as_str() {
                Some(v) => v.to_string(),
                None => continue,
            };
            target.insert(
                name.clone(),
                PackageInfo {
                    name: name.clone(),
                    version,
                    integrity: None,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_jsonc_line_comments() {
        let input = "{\n  \"key\": \"value\" // this is a comment\n}";
        let result = strip_jsonc_comments(input);
        assert!(!result.contains("//"));
        assert!(result.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_strip_jsonc_block_comments() {
        let input = "{\n  /* block comment */\n  \"key\": \"value\"\n}";
        let result = strip_jsonc_comments(input);
        assert!(!result.contains("/*"));
        assert!(result.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_strip_jsonc_preserves_strings() {
        let input = r#"{"url": "https://example.com/path", "comment": "has // slashes and /* stars */"}"#;
        let result = strip_jsonc_comments(input);
        assert!(result.contains("https://example.com/path"));
        assert!(result.contains("has // slashes and /* stars */"));
    }

    #[test]
    fn test_parse_basic_bun_lock() {
        let content = r#"{
  "lockfileVersion": 0,
  "workspaces": {
    "": {
      "name": "my-app",
      "dependencies": {
        "react": "^18.2.0",
        "lodash": "^4.17.21"
      },
      "devDependencies": {
        "typescript": "^5.3.0"
      }
    }
  },
  "packages": {
    "react": ["react@18.2.0"],
    "lodash": ["lodash@4.17.21"],
    "typescript": ["typescript@5.3.3"],
    "@scope/pkg": ["@scope/pkg@2.0.0"]
  }
}"#;
        let graph = parse(content).unwrap();
        assert_eq!(graph.lockfile_type, LockfileType::Bun);
        assert_eq!(graph.dependencies.len(), 2);
        assert!(graph.dependencies.contains_key("react"));
        // Verify resolved versions (not specifiers)
        assert_eq!(graph.dependencies["react"].version, "18.2.0");
        assert_eq!(graph.dependencies["lodash"].version, "4.17.21");
        assert_eq!(graph.dev_dependencies["typescript"].version, "5.3.3");
        assert_eq!(graph.dev_dependencies.len(), 1);
        assert_eq!(graph.all_packages["react"], vec!["18.2.0"]);
        assert_eq!(graph.all_packages["@scope/pkg"], vec!["2.0.0"]);
    }

    #[test]
    fn test_parse_bun_lock_monorepo() {
        let content = r#"{
  "lockfileVersion": 0,
  "workspaces": {
    "": {
      "name": "monorepo-root",
      "dependencies": {
        "react": "^18.2.0"
      }
    },
    "packages/app-a": {
      "name": "app-a",
      "dependencies": {
        "lodash": "^4.17.21"
      },
      "devDependencies": {
        "vitest": "^1.0.0"
      }
    }
  },
  "packages": {
    "react": ["react@18.2.0"],
    "lodash": ["lodash@4.17.21"],
    "vitest": ["vitest@1.2.0"]
  }
}"#;
        let graph = parse(content).unwrap();
        // Root + sub-workspace deps merged
        assert!(graph.dependencies.contains_key("react"));
        assert!(graph.dependencies.contains_key("lodash"));
        assert!(graph.dev_dependencies.contains_key("vitest"));
        // Resolved versions, not specifiers
        assert_eq!(graph.dependencies["react"].version, "18.2.0");
        assert_eq!(graph.dependencies["lodash"].version, "4.17.21");
        assert_eq!(graph.dev_dependencies["vitest"].version, "1.2.0");
    }

    #[test]
    fn test_parse_bun_lock_with_comments() {
        let content = r#"{
  // bun lockfile
  "lockfileVersion": 0,
  "workspaces": {
    "": {
      "name": "my-app",
      /* project dependencies */
      "dependencies": {
        "react": "^18.2.0"
      }
    }
  },
  "packages": {
    "react": ["react@18.2.0"]
  }
}"#;
        let graph = parse(content).unwrap();
        assert_eq!(graph.lockfile_type, LockfileType::Bun);
        assert_eq!(graph.dependencies.len(), 1);
        assert!(graph.dependencies.contains_key("react"));
    }
}