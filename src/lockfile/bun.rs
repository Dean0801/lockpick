use std::collections::HashMap;

use crate::error::LockpickError;
use crate::utils::strip_jsonc_comments;
use crate::{DependencyGraph, LockfileType, PackageInfo};

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

/// Parse bun.lock (JSONC format) content into a DependencyGraph.
pub fn parse(content: &str) -> Result<DependencyGraph, LockpickError> {
    let stripped = strip_jsonc_comments(content);
    let root: serde_json::Value = serde_json::from_str(&stripped)
        .map_err(|e| LockpickError::Parse(format!("Failed to parse bun.lock: {e}")))?;

    let mut deps = HashMap::new();
    let mut dev_deps = HashMap::new();
    let mut all_packages: HashMap<String, Vec<String>> = HashMap::new();

    // Extract dependencies from workspaces[""]
    if let Some(workspace) = root.get("workspaces").and_then(|w| w.get("")) {
        extract_deps(workspace, "dependencies", &mut deps);
        extract_deps(workspace, "devDependencies", &mut dev_deps);
    }

    // Extract all_packages from "packages"
    if let Some(packages) = root.get("packages").and_then(|p| p.as_object()) {
        for (_key, value) in packages {
            if let Some(arr) = value.as_array() {
                if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                    if let Some((name, version)) = parse_name_version(first) {
                        all_packages
                            .entry(name.to_string())
                            .or_default()
                            .push(version.to_string());
                    }
                }
            }
        }
    }

    Ok(DependencyGraph {
        dependencies: deps,
        dev_dependencies: dev_deps,
        lockfile_type: LockfileType::Bun,
        all_packages,
    })
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
        assert_eq!(graph.dev_dependencies.len(), 1);
        assert!(graph.dev_dependencies.contains_key("typescript"));
        assert_eq!(graph.all_packages["react"], vec!["18.2.0"]);
        assert_eq!(graph.all_packages["@scope/pkg"], vec!["2.0.0"]);
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