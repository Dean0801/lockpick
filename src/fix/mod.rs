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
    dry_run: bool,
) -> Result<FixResult, LockpickError> {
    if unused.is_empty() {
        return Ok(FixResult {
            removed: vec![],
            failed: vec![],
        });
    }

    let names: Vec<&str> = unused.iter().map(|d| d.name.as_str()).collect();

    // In dry_run mode, report all packages as removed without executing anything
    if dry_run {
        return Ok(FixResult {
            removed: names.iter().map(|n| n.to_string()).collect(),
            failed: vec![],
        });
    }

    let (binary, subcommand) = get_pm_command(lockfile_type);

    // Batch remove: pnpm remove pkg1 pkg2 pkg3
    let output = Command::new(binary)
        .arg(subcommand)
        .args(&names)
        .current_dir(project_path)
        .output()
        .map_err(LockpickError::Io)?;

    if output.status.success() {
        return Ok(FixResult {
            removed: names.iter().map(|n| n.to_string()).collect(),
            failed: vec![],
        });
    }

    // Batch failed — fall back to removing packages one by one
    // so we can accurately identify which ones actually failed.
    let mut removed = Vec::new();
    let mut failed = Vec::new();

    for name in &names {
        let single = Command::new(binary)
            .arg(subcommand)
            .arg(name)
            .current_dir(project_path)
            .output()
            .map_err(LockpickError::Io)?;

        if single.status.success() {
            removed.push(name.to_string());
        } else {
            let stderr = String::from_utf8_lossy(&single.stderr).to_string();
            failed.push((name.to_string(), stderr));
        }
    }

    Ok(FixResult { removed, failed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DepType;

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
        let result = fix_unused(Path::new("."), &[], &LockfileType::Npm, false).unwrap();
        assert!(result.removed.is_empty());
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_fix_dry_run() {
        // dry_run should return all packages as removed without executing anything
        let deps = vec![
            UnusedDep {
                name: "foo".to_string(),
                version: "1.0.0".to_string(),
                dep_type: DepType::Prod,
            },
            UnusedDep {
                name: "bar".to_string(),
                version: "2.0.0".to_string(),
                dep_type: DepType::Dev,
            },
        ];
        let result = fix_unused(Path::new("."), &deps, &LockfileType::Pnpm, true).unwrap();
        assert_eq!(result.removed, vec!["foo", "bar"]);
        assert!(result.failed.is_empty());
    }
}
