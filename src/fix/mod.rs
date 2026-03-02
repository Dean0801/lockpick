use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

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

/// Lockfile filenames in detection order
const LOCKFILE_NAMES: &[&str] = &[
    "pnpm-lock.yaml",
    "bun.lock",
    "package-lock.json",
    "yarn.lock",
];

/// Backup package.json and lockfile before fix. Returns backup dir path.
pub fn backup_before_fix(project_path: &Path) -> Result<PathBuf, LockpickError> {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_dir = project_path.join(format!(".lockpick-backup/{ts}"));
    std::fs::create_dir_all(&backup_dir).map_err(LockpickError::Io)?;

    // Backup package.json
    let pkg = project_path.join("package.json");
    if pkg.exists() {
        std::fs::copy(&pkg, backup_dir.join("package.json")).map_err(LockpickError::Io)?;
    }

    // Backup lockfile
    for name in LOCKFILE_NAMES {
        let lf = project_path.join(name);
        if lf.exists() {
            std::fs::copy(&lf, backup_dir.join(name)).map_err(LockpickError::Io)?;
            break;
        }
    }

    Ok(backup_dir)
}

/// Restore from the most recent backup in .lockpick-backup/
pub fn restore_backup(project_path: &Path) -> Result<PathBuf, LockpickError> {
    let backup_root = project_path.join(".lockpick-backup");
    if !backup_root.exists() {
        return Err(LockpickError::Config("No backup found".into()));
    }

    // Find latest backup (highest timestamp directory name)
    let mut entries: Vec<_> = std::fs::read_dir(&backup_root)
        .map_err(LockpickError::Io)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    let latest = entries
        .last()
        .ok_or_else(|| LockpickError::Config("No backup found".into()))?;
    let backup_dir = latest.path();

    // Restore files
    for entry in std::fs::read_dir(&backup_dir).map_err(LockpickError::Io)? {
        let entry = entry.map_err(LockpickError::Io)?;
        let name = entry.file_name();
        std::fs::copy(entry.path(), project_path.join(&name)).map_err(LockpickError::Io)?;
    }

    Ok(backup_dir)
}

/// Prompt user for confirmation. Returns true if user confirms.
pub fn confirm_fix(unused: &[UnusedDep]) -> bool {
    eprintln!("\n  即将删除以下 {} 个未使用依赖：\n", unused.len());
    for dep in unused {
        let tag = match dep.dep_type {
            crate::DepType::Prod => "prod",
            crate::DepType::Dev => "dev",
        };
        eprintln!("    [{tag}] {}@{}", dep.name, dep.version);
    }
    eprint!("\n  确认删除？(y/N) ");
    io::stderr().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim(), "y" | "Y" | "yes" | "YES")
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

    #[test]
    fn test_backup_before_fix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"test"}"#).unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'").unwrap();

        let backup_dir = backup_before_fix(dir.path()).unwrap();
        assert!(backup_dir.join("package.json").exists());
        assert!(backup_dir.join("pnpm-lock.yaml").exists());
    }

    #[test]
    fn test_restore_backup() {
        let dir = tempfile::tempdir().unwrap();
        let original = r#"{"name":"original"}"#;
        std::fs::write(dir.path().join("package.json"), original).unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "original-lock").unwrap();

        // Create backup
        backup_before_fix(dir.path()).unwrap();

        // Overwrite files
        std::fs::write(dir.path().join("package.json"), "modified").unwrap();

        // Restore
        restore_backup(dir.path()).unwrap();
        let restored = std::fs::read_to_string(dir.path().join("package.json")).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_restore_no_backup() {
        let dir = tempfile::tempdir().unwrap();
        assert!(restore_backup(dir.path()).is_err());
    }
}
