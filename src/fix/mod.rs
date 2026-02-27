use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

use crate::error::LockpickError;
use crate::{LockfileType, UnusedDep};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    pub removed: Vec<String>,
    pub failed: Vec<(String, String)>,
}

fn get_pm_command(lockfile_type: &LockfileType) -> (&'static str, &'static str) {
    match lockfile_type {
        LockfileType::Pnpm => ("pnpm", "remove"),
        LockfileType::Npm => ("npm", "uninstall"),
        LockfileType::Yarn => ("yarn", "remove"),
        LockfileType::Bun => ("bun", "remove"),
    }
}

pub fn fix_unused(
    project_path: &Path,
    unused: &[UnusedDep],
    lockfile_type: &LockfileType,
) -> Result<FixResult, LockpickError> {
    if unused.is_empty() {
        return Ok(FixResult {
            removed: vec![],
            failed: vec![],
        });
    }

    let (binary, subcommand) = get_pm_command(lockfile_type);
    let names: Vec<&str> = unused.iter().map(|d| d.name.as_str()).collect();

    // Batch remove: pnpm remove pkg1 pkg2 pkg3
    let output = Command::new(binary)
        .arg(subcommand)
        .args(&names)
        .current_dir(project_path)
        .output()
        .map_err(LockpickError::Io)?;

    if output.status.success() {
        Ok(FixResult {
            removed: names.iter().map(|n| n.to_string()).collect(),
            failed: vec![],
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(FixResult {
            removed: vec![],
            failed: names.iter().map(|n| (n.to_string(), stderr.clone())).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pm_command_pnpm() {
        let (bin, sub) = get_pm_command(&LockfileType::Pnpm);
        assert_eq!(bin, "pnpm");
        assert_eq!(sub, "remove");
    }

    #[test]
    fn test_get_pm_command_npm() {
        let (bin, sub) = get_pm_command(&LockfileType::Npm);
        assert_eq!(bin, "npm");
        assert_eq!(sub, "uninstall");
    }

    #[test]
    fn test_get_pm_command_yarn() {
        let (bin, sub) = get_pm_command(&LockfileType::Yarn);
        assert_eq!(bin, "yarn");
        assert_eq!(sub, "remove");
    }

    #[test]
    fn test_get_pm_command_bun() {
        let (bin, sub) = get_pm_command(&LockfileType::Bun);
        assert_eq!(bin, "bun");
        assert_eq!(sub, "remove");
    }

    #[test]
    fn test_fix_empty_unused() {
        let result = fix_unused(Path::new("."), &[], &LockfileType::Npm).unwrap();
        assert!(result.removed.is_empty());
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_fix_dry_run() {
        // dry_run is now handled in main.rs before calling fix_unused
        // This test verifies fix_unused returns empty for empty input
        let result = fix_unused(Path::new("."), &[], &LockfileType::Pnpm).unwrap();
        assert!(result.removed.is_empty());
    }
}
