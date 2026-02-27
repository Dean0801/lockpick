pub mod config;
pub mod imports;
pub mod scripts;
pub mod unused;

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::error::LockpickError;

/// JS/TS file extensions to scan
const SOURCE_EXTENSIONS: &[&str] = &["js", "ts", "jsx", "tsx", "mjs", "cjs", "mts", "cts"];

/// Directories to exclude from scanning
const EXCLUDE_DIRS: &[&str] = &["node_modules", "dist", "build", ".git", ".next", "coverage"];

/// Discover all JS/TS source files in a project directory using walkdir.
/// Skips excluded directories (node_modules, dist, etc.) at entry time for performance.
pub fn discover_source_files(root: &Path) -> Result<Vec<PathBuf>, LockpickError> {
    let mut files = Vec::new();

    let walker = WalkDir::new(root).into_iter().filter_entry(|entry| {
        // Skip excluded directories before descending into them
        if entry.file_type().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                return !EXCLUDE_DIRS.contains(&name);
            }
        }
        true
    });

    for entry in walker {
        let entry = entry.map_err(|e| LockpickError::Io(e.into()))?;
        if entry.file_type().is_file() {
            let path = entry.into_path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if SOURCE_EXTENSIONS.contains(&ext) {
                    files.push(path);
                }
            }
        }
    }

    Ok(files)
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
