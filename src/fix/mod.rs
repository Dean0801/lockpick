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

    if dry_run {
        return Ok(FixResult {
            removed: unused.iter().map(|d| d.name.clone()).collect(),
            failed: vec![],
        });
    }

    let (binary, subcommand) = get_pm_command(lockfile_type);
    let mut removed = Vec::new();
    let mut failed = Vec::new();

    for dep in unused {
        let output = Command::new(binary)
            .arg(subcommand)
            .arg(&dep.name)
            .current_dir(project_path)
            .output()
            .map_err(LockpickError::Io)?;

        if output.status.success() {
            removed.push(dep.name.clone());
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            failed.push((dep.name.clone(), stderr));
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
        let unused = vec![
            UnusedDep { name: "lodash".into(), version: "4.17.21".into(), dep_type: DepType::Prod },
            UnusedDep { name: "chalk".into(), version: "5.0.0".into(), dep_type: DepType::Dev },
        ];
        let result = fix_unused(Path::new("."), &unused, &LockfileType::Pnpm, true).unwrap();
        assert_eq!(result.removed, vec!["lodash", "chalk"]);
        assert!(result.failed.is_empty());
    }
}
