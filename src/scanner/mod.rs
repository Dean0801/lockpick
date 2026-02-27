pub mod config;
pub mod imports;
pub mod scripts;
pub mod unused;

use glob::glob;
use std::path::{Path, PathBuf};

use crate::error::LockpickError;

/// JS/TS file extensions to scan
const SOURCE_EXTENSIONS: &[&str] = &["js", "ts", "jsx", "tsx", "mjs", "cjs", "mts", "cts"];

/// Directories to exclude from scanning
const EXCLUDE_DIRS: &[&str] = &["node_modules", "dist", "build", ".git", ".next", "coverage"];

/// Discover all JS/TS source files in a project directory (single glob pass)
pub fn discover_source_files(root: &Path) -> Result<Vec<PathBuf>, LockpickError> {
    let pattern = format!("{}/**/*", root.display());
    let entries =
        glob(&pattern).map_err(|e| LockpickError::Parse(format!("Invalid glob pattern: {e}")))?;

    let mut files = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| LockpickError::Parse(format!("Glob error: {e}")))?;
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && SOURCE_EXTENSIONS.contains(&ext)
            && !should_exclude(&path)
        {
            files.push(path);
        }
    }

    Ok(files)
}

/// Check if a path should be excluded
fn should_exclude(path: &Path) -> bool {
    path.components()
        .any(|c| EXCLUDE_DIRS.contains(&c.as_os_str().to_str().unwrap_or("")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_discover_source_files() {
        let root = Path::new("tests/fixtures/sample-project");
        let files = discover_source_files(root).unwrap();

        // Should find app.tsx and utils.js
        assert_eq!(files.len(), 2);

        let names: Vec<&str> = files
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        assert!(names.contains(&"app.tsx"));
        assert!(names.contains(&"utils.js"));
    }

    #[test]
    fn test_excludes_node_modules() {
        let root = Path::new("tests/fixtures/sample-project");
        let files = discover_source_files(root).unwrap();

        for f in &files {
            assert!(!f.to_str().unwrap().contains("node_modules"));
        }
    }
}
