use std::collections::{HashMap, HashSet};

use crate::error::LockpickError;
use crate::{DepEdge, DependencyGraph, LockfileType, PackageInfo};

/// Detect whether content is yarn Berry (v2+) format
fn is_berry(content: &str) -> bool {
    content.lines().any(|l| l.starts_with("__metadata:"))
}

/// Parse yarn.lock content into DependencyGraph (auto-detects v1 vs Berry)
pub fn parse(content: &str) -> Result<DependencyGraph, LockpickError> {
    parse_with_dev_names(content, &HashSet::new())
}

/// Parse yarn.lock with dev dependency names from package.json
pub fn parse_with_dev_names(
    content: &str,
    dev_names: &HashSet<String>,
) -> Result<DependencyGraph, LockpickError> {
    if is_berry(content) {
        return parse_berry(content, dev_names);
    }
    parse_v1(content, dev_names)
}

/// Parse yarn.lock v1 format
fn parse_v1(content: &str, dev_names: &HashSet<String>) -> Result<DependencyGraph, LockpickError> {
    // Validate it looks like a yarn.lock file
    if !content.contains("yarn lockfile v1") && !content.trim().is_empty() {
        // Try to parse anyway, but if there are no entries, return error
    }

    let mut dependencies = HashMap::new();
    let mut dev_dependencies = HashMap::new();
    let mut all_packages: HashMap<String, Vec<String>> = HashMap::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Skip comments and empty lines
        if line.starts_with('#') || line.trim().is_empty() {
            i += 1;
            continue;
        }

        // Entry header: not indented, ends with ':'
        if !line.starts_with(' ') && line.ends_with(':') {
            let header = line.trim_end_matches(':').trim();
            let name = extract_package_name(header).map_err(LockpickError::Parse)?;

            // Parse indented block
            let mut version = None;
            let mut integrity = None;

            i += 1;
            while i < lines.len() && lines[i].starts_with(' ') {
                let trimmed = lines[i].trim();

                if trimmed.starts_with("version ") {
                    version = Some(
                        extract_quoted_value(trimmed, "version").map_err(LockpickError::Parse)?,
                    );
                } else if trimmed.starts_with("integrity ") {
                    integrity = Some(
                        extract_quoted_value(trimmed, "integrity").map_err(LockpickError::Parse)?,
                    );
                }

                i += 1;
            }

            let ver = version.ok_or_else(|| {
                LockpickError::Parse(format!("Missing version for package '{name}'"))
            })?;

            all_packages
                .entry(name.clone())
                .or_default()
                .push(ver.clone());

            let info = PackageInfo {
                name: name.clone(),
                version: ver,
                integrity,
            };

            if dev_names.contains(&name) {
                dev_dependencies.entry(name).or_insert(info);
            } else {
                dependencies.entry(name).or_insert(info);
            }
        } else {
            i += 1;
        }
    }

    if dependencies.is_empty() && !content.trim().is_empty() {
        // Check if the content had the header but no packages — that's fine
        if !content.contains("yarn lockfile v1") {
            return Err(LockpickError::Parse("Invalid yarn.lock format".into()));
        }
    }

    Ok(DependencyGraph {
        dependencies,
        dev_dependencies,
        lockfile_type: LockfileType::Yarn,
        all_packages,
        dep_edges: HashMap::new(),
    })
}

/// Parse yarn.lock with transitive dependency edges.
pub fn parse_with_edges(
    content: &str,
    dev_names: &HashSet<String>,
) -> Result<DependencyGraph, LockpickError> {
    let mut graph = parse_with_dev_names(content, dev_names)?;
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with('#') || line.trim().is_empty() {
            i += 1;
            continue;
        }
        if !line.starts_with(' ') && line.ends_with(':') {
            let header = line.trim_end_matches(':').trim();
            let name = extract_package_name(header).unwrap_or_default();
            let mut version = String::new();
            let mut edges = Vec::new();
            let mut in_deps = false;

            i += 1;
            while i < lines.len() && lines[i].starts_with(' ') {
                let trimmed = lines[i].trim();
                if trimmed.starts_with("version ") {
                    version = extract_quoted_value(trimmed, "version").unwrap_or_default();
                } else if trimmed == "dependencies:" {
                    in_deps = true;
                } else if in_deps && trimmed.contains(' ') {
                    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        edges.push(DepEdge {
                            name: parts[0].trim_matches('"').to_string(),
                            version: parts[1].trim_matches('"').to_string(),
                        });
                    }
                } else {
                    in_deps = false;
                }
                i += 1;
            }
            if !edges.is_empty() && !version.is_empty() {
                graph.dep_edges.insert(format!("{name}@{version}"), edges);
            }
        } else {
            i += 1;
        }
    }
    Ok(graph)
}

/// Extract package name from a yarn.lock entry header.
/// Handles both regular (`lodash@^4.17.21`) and scoped (`@scope/pkg@^1.0.0`) packages.
fn extract_package_name(header: &str) -> Result<String, String> {
    // Remove surrounding quotes if present
    let header = header.trim_matches('"');

    // For scoped packages like @scope/pkg@^1.0.0, the last '@' is the version separator
    // For regular packages like lodash@^4.17.21, the first '@' is the version separator
    if let Some(stripped) = header.strip_prefix('@') {
        // Scoped package: find the second '@'
        match stripped.find('@') {
            Some(pos) => Ok(header[..pos + 1].to_string()),
            None => Err(format!("Invalid scoped package entry: {header}")),
        }
    } else {
        // Regular package: split at first '@'
        match header.find('@') {
            Some(pos) => Ok(header[..pos].to_string()),
            None => Err(format!(
                "Invalid package entry (no version specifier): {header}"
            )),
        }
    }
}

/// Extract a quoted value from a line like `version "1.2.3"` or `integrity "sha512-..."`.
fn extract_quoted_value(line: &str, key: &str) -> Result<String, String> {
    let after_key = line
        .strip_prefix(key)
        .ok_or_else(|| format!("Line does not start with '{key}': {line}"))?;
    let trimmed = after_key.trim();
    let unquoted = trimmed.trim_matches('"');
    Ok(unquoted.to_string())
}

/// Extract package name from a Berry-format header like `"lodash@npm:^4.17.21":`
/// or `"@scope/pkg@npm:^2.0.0":`.
fn extract_berry_package_name(header: &str) -> Option<String> {
    // Remove surrounding quotes and trailing colon
    let h = header.trim().trim_end_matches(':').trim_matches('"');
    // Strip the `@npm:` or `@workspace:` protocol suffix
    // Find the last `@` that precedes `npm:` or `workspace:` etc.
    if let Some(stripped) = h.strip_prefix('@') {
        // Scoped: @scope/pkg@npm:^ver
        if let Some(pos) = stripped
            .find("@npm:")
            .or_else(|| stripped.find("@workspace:"))
        {
            return Some(h[..pos + 1].to_string());
        }
        // Fallback: try last '@'
        stripped.rfind('@').map(|pos| h[..pos + 1].to_string())
    } else {
        // Unscoped: lodash@npm:^ver
        h.find("@npm:")
            .or_else(|| h.find("@workspace:"))
            .or_else(|| h.find('@'))
            .map(|pos| h[..pos].to_string())
    }
}

/// Parse yarn Berry (v2/v3/v4) format
fn parse_berry(
    content: &str,
    dev_names: &HashSet<String>,
) -> Result<DependencyGraph, LockpickError> {
    let mut dependencies = HashMap::new();
    let mut dev_dependencies = HashMap::new();
    let mut all_packages: HashMap<String, Vec<String>> = HashMap::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Skip __metadata block, comments, empty lines
        if line.starts_with("__metadata:") {
            i += 1;
            while i < lines.len() && lines[i].starts_with(' ') {
                i += 1;
            }
            continue;
        }
        if line.trim().is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }

        // Entry header: starts with `"pkg@npm:...":` (not indented, ends with ':')
        if !line.starts_with(' ') && line.trim_end().ends_with(':') {
            let name = match extract_berry_package_name(line) {
                Some(n) => n,
                None => {
                    i += 1;
                    continue;
                }
            };

            // Parse indented block for version and checksum
            let mut version = None;
            let mut checksum = None;
            i += 1;

            while i < lines.len() && lines[i].starts_with(' ') {
                let trimmed = lines[i].trim();
                if trimmed.starts_with("version:") {
                    version = trimmed
                        .strip_prefix("version:")
                        .map(|v| v.trim().trim_matches('"').to_string());
                } else if trimmed.starts_with("checksum:") {
                    checksum = trimmed
                        .strip_prefix("checksum:")
                        .map(|v| v.trim().trim_matches('"').to_string());
                }
                i += 1;
            }

            let Some(ver) = version else { continue };

            all_packages
                .entry(name.clone())
                .or_default()
                .push(ver.clone());

            let info = PackageInfo {
                name: name.clone(),
                version: ver,
                integrity: checksum,
            };

            if dev_names.contains(&name) {
                dev_dependencies.entry(name).or_insert(info);
            } else {
                dependencies.entry(name).or_insert(info);
            }
        } else {
            i += 1;
        }
    }

    Ok(DependencyGraph {
        dependencies,
        dev_dependencies,
        lockfile_type: LockfileType::Yarn,
        all_packages,
        dep_edges: HashMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v1_basic() {
        let content = include_str!("../../tests/fixtures/yarn.lock");
        let graph = parse(content).unwrap();

        assert_eq!(graph.lockfile_type, LockfileType::Yarn);
        assert_eq!(graph.dependencies.len(), 3);
        assert!(graph.dev_dependencies.is_empty());

        // Verify lodash
        let lodash = &graph.dependencies["lodash"];
        assert_eq!(lodash.name, "lodash");
        assert_eq!(lodash.version, "4.17.21");
        assert_eq!(
            lodash.integrity.as_deref(),
            Some("sha512-fake-lodash-integrity")
        );

        // Verify react
        let react = &graph.dependencies["react"];
        assert_eq!(react.name, "react");
        assert_eq!(react.version, "18.2.0");
        assert_eq!(
            react.integrity.as_deref(),
            Some("sha512-fake-react-integrity")
        );

        // Verify typescript
        let ts = &graph.dependencies["typescript"];
        assert_eq!(ts.name, "typescript");
        assert_eq!(ts.version, "5.3.3");
        assert_eq!(
            ts.integrity.as_deref(),
            Some("sha512-fake-typescript-integrity")
        );

        // Verify all_packages is populated
        assert_eq!(graph.all_packages.len(), 3);
        assert_eq!(graph.all_packages["lodash"], vec!["4.17.21"]);
        assert_eq!(graph.all_packages["react"], vec!["18.2.0"]);
        assert_eq!(graph.all_packages["typescript"], vec!["5.3.3"]);
    }

    #[test]
    fn test_parse_invalid() {
        let result = parse("this is not a valid yarn.lock file at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_berry_basic() {
        let content = include_str!("../../tests/fixtures/yarn-berry.lock");
        let graph = parse(content).unwrap();

        assert_eq!(graph.lockfile_type, LockfileType::Yarn);
        assert!(graph.dependencies.contains_key("lodash"));
        assert!(graph.dependencies.contains_key("react"));
        assert!(graph.dependencies.contains_key("typescript"));
        assert!(graph.dependencies.contains_key("@scope/pkg"));

        assert_eq!(graph.dependencies["lodash"].version, "4.17.21");
        assert_eq!(graph.dependencies["react"].version, "18.2.0");
        assert_eq!(graph.dependencies["typescript"].version, "5.3.3");
        assert_eq!(graph.dependencies["@scope/pkg"].version, "2.0.0");

        // Checksum stored as integrity
        assert!(graph.dependencies["lodash"].integrity.is_some());
    }

    #[test]
    fn test_parse_berry_with_dev() {
        let content = include_str!("../../tests/fixtures/yarn-berry.lock");
        let dev_names: HashSet<String> = ["typescript".to_string()].into();
        let graph = parse_with_dev_names(content, &dev_names).unwrap();

        assert!(graph.dev_dependencies.contains_key("typescript"));
        assert!(!graph.dependencies.contains_key("typescript"));
        assert_eq!(graph.dev_dependencies["typescript"].version, "5.3.3");
    }

    #[test]
    fn test_is_berry_detection() {
        assert!(is_berry("__metadata:\n  version: 8\n"));
        assert!(!is_berry("# yarn lockfile v1\n"));
    }

    #[test]
    fn test_extract_berry_package_name() {
        assert_eq!(
            extract_berry_package_name("\"lodash@npm:^4.17.21\":"),
            Some("lodash".into())
        );
        assert_eq!(
            extract_berry_package_name("\"@scope/pkg@npm:^2.0.0\":"),
            Some("@scope/pkg".into())
        );
    }
}
