use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use glob::glob;
use serde_json::Value;

use crate::error::LockpickError;

/// Checks if the given project root is a monorepo with workspace configuration.
///
/// Returns true if pnpm-workspace.yaml exists OR package.json has a "workspaces" field.
pub fn is_monorepo(project_root: &Path) -> bool {
    if project_root.join("pnpm-workspace.yaml").exists() {
        return true;
    }

    let pkg_path = project_root.join("package.json");
    if let Ok(content) = fs::read_to_string(&pkg_path)
        && let Ok(json) = serde_json::from_str::<Value>(&content)
    {
        return json.get("workspaces").is_some();
    }

    false
}

/// Extracts workspace glob patterns from pnpm-workspace.yaml or package.json.
fn get_workspace_globs(project_root: &Path) -> Result<Vec<String>, LockpickError> {
    // Try pnpm-workspace.yaml first
    let pnpm_path = project_root.join("pnpm-workspace.yaml");
    if let Ok(content) = fs::read_to_string(&pnpm_path) {
        let yaml: Value = serde_yaml::from_str(&content)
            .map_err(|e| LockpickError::Parse(format!("Failed to parse pnpm-workspace.yaml: {e}")))?;
        if let Some(packages) = yaml.get("packages").and_then(|v| v.as_array()) {
            let globs = packages
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            return Ok(globs);
        }
    }

    // Try package.json workspaces
    let pkg_path = project_root.join("package.json");
    if let Ok(content) = fs::read_to_string(&pkg_path) {
        let json: Value = serde_json::from_str(&content)
            .map_err(|e| LockpickError::Parse(format!("Failed to parse package.json: {e}")))?;

        if let Some(workspaces) = json.get("workspaces") {
            // Array format: "workspaces": ["packages/*"]
            if let Some(arr) = workspaces.as_array() {
                let globs = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                return Ok(globs);
            }
            // Object format: "workspaces": { "packages": ["packages/*"] }
            if let Some(arr) = workspaces.get("packages").and_then(|v| v.as_array()) {
                let globs = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                return Ok(globs);
            }
        }
    }

    Err(LockpickError::Config("No workspace configuration found".into()))
}

/// Expands workspace glob patterns to find all workspace package directories.
///
/// For each glob pattern, appends `/package.json` and uses `glob::glob()` to find matches.
/// Returns the parent directories of found package.json files, sorted and deduplicated.
pub fn detect_workspaces(project_root: &Path) -> Result<Vec<PathBuf>, LockpickError> {
    let globs = get_workspace_globs(project_root)?;
    let mut dirs = BTreeSet::new();

    for pattern in &globs {
        let full_pattern = project_root.join(pattern).join("package.json");
        let pattern_str = full_pattern.to_string_lossy().to_string();

        let entries =
            glob(&pattern_str).map_err(|e| LockpickError::Parse(format!("Invalid glob pattern '{pattern}': {e}")))?;

        for entry in entries.flatten() {
            if let Some(parent) = entry.parent() {
                dirs.insert(parent.to_path_buf());
            }
        }
    }

    Ok(dirs.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_pnpm_workspace(dir: &Path) {
        fs::create_dir_all(dir.join("packages/app-a")).unwrap();
        fs::create_dir_all(dir.join("packages/app-b")).unwrap();
        fs::create_dir_all(dir.join("libs/shared")).unwrap();

        fs::write(
            dir.join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n  - 'libs/*'\n",
        )
        .unwrap();

        for sub in &["packages/app-a", "packages/app-b", "libs/shared"] {
            fs::write(
                dir.join(sub).join("package.json"),
                format!(r#"{{"name": "{}"}}"#, sub.replace('/', "-")),
            )
            .unwrap();
        }
    }

    #[test]
    fn test_is_monorepo_pnpm() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        )
        .unwrap();
        assert!(is_monorepo(tmp.path()));
    }

    #[test]
    fn test_is_monorepo_npm_workspaces() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "root", "workspaces": ["packages/*"]}"#,
        )
        .unwrap();
        assert!(is_monorepo(tmp.path()));
    }

    #[test]
    fn test_is_monorepo_false() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "solo-project"}"#,
        )
        .unwrap();
        assert!(!is_monorepo(tmp.path()));
    }

    #[test]
    fn test_detect_workspaces_pnpm() {
        let tmp = TempDir::new().unwrap();
        setup_pnpm_workspace(tmp.path());

        let result = detect_workspaces(tmp.path()).unwrap();
        assert_eq!(result.len(), 3);

        let names: Vec<String> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"app-a".to_string()));
        assert!(names.contains(&"app-b".to_string()));
        assert!(names.contains(&"shared".to_string()));
    }

    #[test]
    fn test_detect_workspaces_npm_array() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("packages/foo")).unwrap();
        fs::create_dir_all(tmp.path().join("packages/bar")).unwrap();

        fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "root", "workspaces": ["packages/*"]}"#,
        )
        .unwrap();

        for sub in &["packages/foo", "packages/bar"] {
            fs::write(
                tmp.path().join(sub).join("package.json"),
                format!(r#"{{"name": "{}"}}"#, sub.replace('/', "-")),
            )
            .unwrap();
        }

        let result = detect_workspaces(tmp.path()).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_detect_workspaces_npm_object() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("modules/core")).unwrap();

        fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "root", "workspaces": {"packages": ["modules/*"]}}"#,
        )
        .unwrap();

        fs::write(
            tmp.path().join("modules/core/package.json"),
            r#"{"name": "core"}"#,
        )
        .unwrap();

        let result = detect_workspaces(tmp.path()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_detect_workspaces_no_config() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"name": "solo"}"#).unwrap();

        let result = detect_workspaces(tmp.path());
        assert!(result.is_err());
    }
}
